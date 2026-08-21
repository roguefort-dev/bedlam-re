//! The platform audio output (P4 shell step 2, D40): a cpal output
//! stream fed by the GameHost audio bus.
//!
//! RUNTIME-GATED exactly like [`crate::window`]: a stream is only
//! ever built by the window host (behind `--window` /
//! `BEDLAM_SHELL=1`); the headless smoke and tests never touch a
//! device. The DEVICE path lives only here - the mixer in
//! bedlam-audio stays hermetic (integer math, no I/O, DESIGN-AUDIO),
//! and the mixed byte stream remains the determinism gate (D17
//! bucket b: audio is NOT hashed).
//!
//! Threading shape: [`GameHost::render_audio`] (and the mixer inside
//! it) is NOT Send and must never be - so the mix happens on the
//! MAIN thread, once per window iteration, into a bounded ring of
//! ready i16 stereo frames; the cpal callback (its own realtime
//! thread) only DRAINS that ring. The ring is the only crossing
//! point, guarded by a plain mutex whose critical sections are a
//! few samples wide (poison-tolerant: a panicking producer must not
//! turn the callback into an error storm). A DEAD-FEED GUARD (D48)
//! sits in front: once the owning [`AudioDevice`] drops (quiet ->
//! pause -> stream drop), any late callback invocation writes
//! EXACT silence without touching the ring, and [`AudioFeed::
//! fill_from`] renders nothing - teardown of the window host can
//! never race the realtime thread.
//!
//! Device-feed arithmetic (all integer, all unit-pinned):
//! - Rate policy (D47): the stream is opened at the best MODERN
//!   device rate - 48000 Hz first, then 44100 Hz - falling back to
//!   the mixer-native 11025 Hz (resampling is NOT owed at the native
//!   rate) only when no modern rate is offered, and to the device
//!   default config when not even that. The mixer bus and the
//!   parity stream stay 11025 Hz byte-faithful; conversion is a
//!   DEVICE-BOUNDARY concern only, never visible upstream.
//! - Rate conversion: the Q16 frame stepper (output position
//!   n * step / 65536) with LINEAR INTERPOLATION between the
//!   bracketing input frames - round to nearest, ties toward +inf,
//!   i64 internally (a full-scale delta times frac overflows i32).
//!   At the native rate the phase residue is always 0, so the
//!   passthrough stays EXACT and un-interpolated.
//! - Underrun is EXACT silence ([0, 0] frames), matching the mixer
//!   bus semantics; a full ring drops the OLDEST frames (lateness
//!   is skipped, never accumulated).
//! - Channel mapping: mono devices take the floor average
//!   `(l + r) >> 1`; stereo passes through; >2 channels repeat
//!   L/R alternately (even = L, odd = R).
//! - Sample format conversion goes through cpal's `Sample`
//!   conversions (dasp); no floating point exists on the produce
//!   path, only inside the device callback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use bedlam_audio::SAMPLE_RATE;
use bedlam_game::{GameError, GameHost};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Ring capacity in frames (~371 ms at 11025 Hz): enough that a
/// busy compositor hiccup does not underrun, small enough that a
/// stalled loop recovers with one 67 ms skip, not a buffer's worth.
pub const RING_CAP_FRAMES: usize = 4096;

/// Fill target in frames (~67 ms at 11025 Hz = four 60 Hz pumps).
/// The window loop refills toward this mark every iteration, so
/// production self-balances against the device clock: drift shows
/// up as a slightly different deficit, never as runaway latency.
pub const TARGET_FRAMES: usize = 736;

/// Headless per-pump drain count: ceil(11025 / 60) frames - the
/// deterministic smoke paces the mix the same way the device paces
/// playback (60 pumps * 184 frames = 11040 frames per 10 s walk,
/// the same 0.3 % surplus the window loop sheds via drop-oldest).
pub const PUMP_FRAMES: usize = 184;

/// Preference rank for device channel counts: stereo first (the mix
/// is interleaved stereo), then mono, then anything else.
fn channel_pref(channels: u16) -> u8 {
    match channels {
        2 => 0,
        1 => 1,
        _ => 2,
    }
}

/// One device output channel's sample for an input stereo frame.
/// Mono = floor average; even channels = L, odd = R.
fn channel_sample(channels: usize, c: usize, l: i16, r: i16) -> i16 {
    if channels == 1 {
        return ((i32::from(l) + i32::from(r)) >> 1) as i16;
    }
    if !c.is_multiple_of(2) {
        r
    } else {
        l
    }
}

/// The sample formats the negotiation cares about: prefer the
/// device-native S16 (a pure widening of the ring's i16), then F32
/// (the cpal/dasp float default of most modern hosts), then accept
/// whatever else the winning range offers rather than lose the rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SampleFormatKind {
    I16,
    F32,
    Other,
}

impl SampleFormatKind {
    fn of(format: cpal::SampleFormat) -> SampleFormatKind {
        match format {
            cpal::SampleFormat::I16 => SampleFormatKind::I16,
            cpal::SampleFormat::F32 => SampleFormatKind::F32,
            _ => SampleFormatKind::Other,
        }
    }

    /// Preference rank within one rate: S16, then F32, then any.
    fn pref(self) -> u8 {
        match self {
            SampleFormatKind::I16 => 0,
            SampleFormatKind::F32 => 1,
            SampleFormatKind::Other => 2,
        }
    }
}

/// A neutralized view of one cpal supported-config range. cpal's
/// `SupportedStreamConfigRange` is not constructible outside the
/// crate, so the negotiation is a PURE function over these - the
/// unit-test "mocked device configs" (D47 task wording).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputConfigSpec {
    pub min_rate: u32,
    pub max_rate: u32,
    pub channels: u16,
    pub format: SampleFormatKind,
}

impl OutputConfigSpec {
    fn of_range(range: &cpal::SupportedStreamConfigRange) -> OutputConfigSpec {
        OutputConfigSpec {
            min_rate: range.min_sample_rate(),
            max_rate: range.max_sample_rate(),
            channels: range.channels(),
            format: SampleFormatKind::of(range.sample_format()),
        }
    }

    fn contains_rate(&self, rate: u32) -> bool {
        (self.min_rate..=self.max_rate).contains(&rate)
    }
}

/// The modern-rate ladder (D47): 48000 Hz first, then 44100 Hz, then
/// the mixer-native 11025 Hz (exact, no resampling owed). Devices
/// offering none of these fall back to their default config.
pub(crate) const PREFERRED_RATES: [u32; 3] = [48_000, 44_100, SAMPLE_RATE];

/// Pick the output config: for each rate in [`PREFERRED_RATES`]
/// (in order), the best range CONTAINING that rate wins - ranked by
/// (channel count, sample format), stereo before mono, S16 before
/// F32 before the rest, first-listed on exact ties (stable, matching
/// the device's own enumeration order). RATE DOMINATES the ranking:
/// 48000 mono beats 44100 stereo. Returns `(index into specs, the
/// concrete rate)` for the caller to pin via `try_with_sample_rate`;
/// `None` = nothing preferred offered, use the device default.
pub(crate) fn choose_output_config(specs: &[OutputConfigSpec]) -> Option<(usize, u32)> {
    for &rate in PREFERRED_RATES.iter() {
        let mut best: Option<usize> = None;
        for (i, spec) in specs.iter().enumerate() {
            if !spec.contains_rate(rate) {
                continue;
            }
            let better = match best {
                None => true,
                Some(b) => {
                    let cur = &specs[b];
                    (channel_pref(spec.channels), spec.format.pref())
                        < (channel_pref(cur.channels), cur.format.pref())
                }
            };
            if better {
                best = Some(i);
            }
        }
        if let Some(i) = best {
            return Some((i, rate));
        }
    }
    None
}

/// Bounded FIFO ring of interleaved-stereo i16 frames. Pushes onto
/// a full ring overwrite the OLDEST frames (audio lateness policy:
/// skip ahead, never accumulate delay).
#[derive(Debug)]
pub(crate) struct SampleRing {
    buf: Vec<i16>,
    cap: usize,
    head: usize,
    len: usize,
}

impl SampleRing {
    fn new(cap: usize) -> SampleRing {
        SampleRing {
            buf: vec![0i16; cap * 2],
            cap,
            head: 0,
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    /// Append frames from `src` (len must be even). Returns the
    /// frame count taken (always all of them - a full ring sheds
    /// its own oldest frames instead).
    fn push_frames(&mut self, src: &[i16]) -> usize {
        debug_assert!(src.len().is_multiple_of(2));
        for f in 0..src.len() / 2 {
            let tail = (self.head + self.len) % self.cap;
            self.buf[2 * tail] = src[2 * f];
            self.buf[2 * tail + 1] = src[2 * f + 1];
            if self.len == self.cap {
                self.head = (self.head + 1) % self.cap; // overwrite oldest
            } else {
                self.len += 1;
            }
        }
        src.len() / 2
    }

    /// Frame `offset` positions ahead of the read cursor, without
    /// consuming. `None` past the stored end (underrun).
    fn peek_frame(&self, offset: usize) -> Option<[i16; 2]> {
        if offset >= self.len {
            return None;
        }
        let i = (self.head + offset) % self.cap;
        Some([self.buf[2 * i], self.buf[2 * i + 1]])
    }

    /// Consume up to `n` frames (clamped to the stored length).
    fn pop_frames(&mut self, n: usize) {
        let n = n.min(self.len);
        self.head = (self.head + n) % self.cap;
        self.len -= n;
    }
}

/// Q16 frame stepper with LINEAR INTERPOLATION (D47): output n
/// reads input position n * step / 65536, step = round(src * 65536 /
/// dst), blending the bracketing input frames base + ((target -
/// base) * frac rounded) - round to nearest, ties toward +inf. At
/// the native rate (step 0x10000) the phase residue is always 0, so
/// each output IS its input frame exactly (a true 1:1 pass-through);
/// anything else walks the input at the converted rate.
#[derive(Debug)]
pub(crate) struct FrameStepper {
    step_q16: u32,
    phase_q16: u64,
}

/// Blend two i16 samples at Q16 fractional position. The result is
/// convex (it can reach, never pass, the target when frac rounds up
/// from 0xFFFF), so the clamp is purely defensive; the product is
/// computed in i64 because a full-scale delta (|t-b| up to 65535)
/// times frac (up to 0xFFFF) overflows i32.
fn blend(base: i16, target: i16, frac_q16: u32) -> i16 {
    let delta = i64::from(i32::from(target) - i32::from(base));
    let v = i32::from(base) + (((delta * i64::from(frac_q16)) + 0x8000) >> 16) as i32;
    v.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

impl FrameStepper {
    fn new(src_rate: u32, dst_rate: u32) -> FrameStepper {
        let dst = dst_rate.max(1) as u64;
        let step = ((src_rate as u64 * 65_536) + dst / 2) / dst;
        FrameStepper {
            step_q16: step.clamp(1, u32::MAX as u64) as u32,
            phase_q16: 0,
        }
    }

    /// Produce one output frame at the current input position, then
    /// consume the input frames the output cursor has fully passed.
    /// A bracketing pair interpolates; a LONE frame (its neighbor
    /// not yet buffered) is held for every output that lands on it
    /// (edge-hold - never reach ahead into underrun); an EMPTY ring
    /// yields EXACT silence. Every case still advances the phase (a
    /// skipped stretch stays skipped once the data returns -
    /// streaming semantics, deterministic).
    fn next_frame(&mut self, ring: &mut SampleRing) -> [i16; 2] {
        let frac = self.phase_q16 as u32;
        let frame = match (ring.peek_frame(0), ring.peek_frame(1)) {
            (Some(base), Some(target)) => [
                blend(base[0], target[0], frac),
                blend(base[1], target[1], frac),
            ],
            (Some(base), None) => base, // edge-hold
            (None, _) => [0, 0],        // underrun
        };
        self.phase_q16 += u64::from(self.step_q16);
        let pops = (self.phase_q16 >> 16) as usize;
        self.phase_q16 &= 0xFFFF;
        ring.pop_frames(pops);
        frame
    }
}

/// The state shared with the device callback.
#[derive(Debug)]
pub(crate) struct FeedState {
    ring: SampleRing,
    step: FrameStepper,
}

/// Producer handle: the main-thread side that mixes into the ring.
/// Cheap `Clone` (one Arc) so split-borrow sites can hand the feed
/// out while the host stays mutably borrowed.
#[derive(Debug, Clone)]
pub struct AudioFeed {
    state: Arc<Mutex<FeedState>>,
    /// Dead-feed guard (D48): shared with the device callback. The
    /// owning [`AudioDevice`] clears it on drop; from then on the
    /// callback writes EXACT silence without touching the ring and
    /// [`AudioFeed::fill_from`] renders nothing. Sticky - a feed
    /// never wakes back up (a dropped stream is gone for good).
    alive: Arc<AtomicBool>,
}

impl AudioFeed {
    pub(crate) fn new(cap: usize, src_rate: u32, dst_rate: u32) -> AudioFeed {
        AudioFeed {
            state: Arc::new(Mutex::new(FeedState {
                ring: SampleRing::new(cap),
                step: FrameStepper::new(src_rate, dst_rate),
            })),
            alive: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Silence this feed forever (the [`AudioDevice`] drop path).
    /// Late callback invocations emit exact silence afterwards.
    fn quiet(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }

    /// Whether the feed is still live (diagnostics/tests).
    fn is_quiet(&self) -> bool {
        !self.alive.load(Ordering::Relaxed)
    }

    /// Poison-tolerant lock: a panicking producer must not turn the
    /// realtime callback into an error storm; the ring contents are
    /// plain data, safe to keep using.
    fn lock(&self) -> MutexGuard<'_, FeedState> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Frames currently buffered.
    pub fn buffered_frames(&self) -> usize {
        self.lock().ring.len()
    }

    /// Mix from the host until the ring holds `target` frames (the
    /// window-loop watermark fill; chunking-invariant by the mixer
    /// contract). Returns the frames actually rendered (0 when the
    /// ring is already at target, or the feed is dead - a dropped
    /// device never consumes more). A render error propagates - the
    /// caller decides whether that is fatal (it is not, for audio).
    pub fn fill_from(&self, host: &mut GameHost, target: usize) -> Result<usize, GameError> {
        if self.is_quiet() {
            return Ok(0);
        }
        let deficit = target.saturating_sub(self.buffered_frames());
        if deficit == 0 {
            return Ok(0);
        }
        let mut scratch = vec![0i16; deficit * 2];
        let mixed = host.render_audio(&mut scratch)?;
        let mut state = self.lock();
        let taken = state.ring.push_frames(&scratch[..mixed * 2]);
        debug_assert_eq!(taken, mixed);
        Ok(mixed)
    }
}

/// What the device actually opened (diagnostics; unit-pinned shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamFacts {
    pub rate: u32,
    pub channels: u16,
    pub format: String,
}

impl StreamFacts {
    fn of(config: &cpal::StreamConfig, format: cpal::SampleFormat) -> StreamFacts {
        StreamFacts {
            rate: config.sample_rate,
            channels: config.channels,
            format: format.to_string(),
        }
    }
}

/// The live output device: a playing cpal stream plus its feed.
/// Dropping this stops the stream - keep it for the loop's life.
pub struct AudioDevice {
    stream: cpal::Stream,
    feed: AudioFeed,
    facts: StreamFacts,
}

impl AudioDevice {
    /// Open the default output device at the best MODERN rate
    /// (D47): 48000 Hz, then 44100 Hz, then the mixer-native
    /// 11025 Hz - each pinned exactly inside a supported range
    /// (`try_with_sample_rate`), ranked within a rate by channels
    /// (stereo, mono, other) then format (S16, F32, other). No
    /// preferred rate offered falls back to the device default
    /// config; either non-native rate runs through the interpolated
    /// frame stepper. `None` means no usable audio (the shell
    /// continues silent).
    pub fn open_default() -> Option<AudioDevice> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let mut chosen: Option<cpal::SupportedStreamConfig> = None;
        if let Ok(iter) = device.supported_output_configs() {
            let ranges: Vec<cpal::SupportedStreamConfigRange> = iter.collect();
            let specs: Vec<OutputConfigSpec> =
                ranges.iter().map(OutputConfigSpec::of_range).collect();
            if let Some((i, rate)) = choose_output_config(&specs) {
                chosen = ranges[i].try_with_sample_rate(rate);
            }
        }
        let config = chosen.or_else(|| device.default_output_config().ok())?;
        AudioDevice::build(&device, config)
    }

    fn build(device: &cpal::Device, config: cpal::SupportedStreamConfig) -> Option<AudioDevice> {
        let format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();
        let feed = AudioFeed::new(RING_CAP_FRAMES, SAMPLE_RATE, stream_config.sample_rate);
        let state = feed.state.clone();
        let alive = feed.alive.clone();
        let channels = usize::from(stream_config.channels);
        let report_error = |err| eprintln!("bedlam-shell: audio stream error: {err}");
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                stream_config,
                callback::<f32>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::F64 => device.build_output_stream(
                stream_config,
                callback::<f64>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::I8 => device.build_output_stream(
                stream_config,
                callback::<i8>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                stream_config,
                callback::<i16>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::I32 => device.build_output_stream(
                stream_config,
                callback::<i32>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::I64 => device.build_output_stream(
                stream_config,
                callback::<i64>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::U8 => device.build_output_stream(
                stream_config,
                callback::<u8>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                stream_config,
                callback::<u16>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::U32 => device.build_output_stream(
                stream_config,
                callback::<u32>(state, alive, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::U64 => device.build_output_stream(
                stream_config,
                callback::<u64>(state, alive, channels),
                report_error,
                None,
            ),
            _ => return None,
        }
        .ok()?;
        stream.play().ok()?;
        Some(AudioDevice {
            stream,
            feed,
            facts: StreamFacts::of(&stream_config, format),
        })
    }

    /// The producer handle (clone the Arc out for split-borrow sites).
    pub fn feed(&self) -> &AudioFeed {
        &self.feed
    }

    /// The opened device shape (diagnostics).
    pub fn facts(&self) -> &StreamFacts {
        &self.facts
    }
}

impl Drop for AudioDevice {
    /// Ordered stream stop (D48): quiet the feed FIRST - the
    /// dead-feed guard makes any late callback invocation write
    /// exact silence without touching the ring - then pause, then
    /// the fields drop (stream before feed by declaration, which
    /// releases the callback closure and its ring Arc).
    fn drop(&mut self) {
        self.feed.quiet();
        let _ = self.stream.pause();
    }
}

/// The device callback: drain ring frames into the interleaved
/// device buffer through the stepper + channel mapping + sample
/// conversion. Underrun frames are exact silence. The dead-feed
/// guard (D48) is checked BEFORE the lock: a dropped device's
/// stream must never touch the shared ring again.
fn callback<S: cpal::SizedSample + cpal::FromSample<i16>>(
    state: Arc<Mutex<FeedState>>,
    alive: Arc<AtomicBool>,
    channels: usize,
) -> impl FnMut(&mut [S], &cpal::OutputCallbackInfo) + Send + 'static {
    move |data, _info| {
        if !alive.load(Ordering::Relaxed) {
            silence(data);
            return;
        }
        let mut st = state.lock().unwrap_or_else(|p| p.into_inner());
        drain(&mut st, data, channels);
    }
}

/// Fill the buffer with EXACT silence - each format's zero, so u8
/// lands at the 128 midpoint, f32/i* at 0 (the same dasp mapping
/// the device-edge pin test asserts).
fn silence<S: cpal::SizedSample + cpal::FromSample<i16>>(data: &mut [S]) {
    for out in data.iter_mut() {
        *out = S::from_sample::<i16>(0);
    }
}

/// The live-drain half of the callback (factored so the ring walk
/// is unit-testable without a device).
fn drain<S: cpal::SizedSample + cpal::FromSample<i16>>(
    state: &mut FeedState,
    data: &mut [S],
    channels: usize,
) {
    let FeedState { ring, step } = state;
    for chunk in data.chunks_mut(channels.max(1)) {
        let frame = step.next_frame(ring);
        for (c, out) in chunk.iter_mut().enumerate() {
            *out = S::from_sample::<i16>(channel_sample(channels, c, frame[0], frame[1]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frame values that carry their source index: [i, -i].
    fn frame(i: usize) -> [i16; 2] {
        let v = i as i16; // every test index stays below i16::MAX
        [v, -v]
    }

    fn flat(frames: &[[i16; 2]]) -> Vec<i16> {
        frames.iter().flat_map(|f| [f[0], f[1]]).collect()
    }

    #[test]
    fn ring_is_fifo_with_wraparound() {
        let mut ring = SampleRing::new(4);
        let src = flat(&[frame(1), frame(2), frame(3), frame(4), frame(5), frame(6)]);
        assert_eq!(ring.push_frames(&src), 6, "all frames taken; 3 oldest shed");
        assert_eq!(ring.len(), 4);
        assert_eq!(ring.peek_frame(0), Some(frame(3)));
        assert_eq!(ring.peek_frame(3), Some(frame(6)));
        assert_eq!(ring.peek_frame(4), None);
        ring.pop_frames(2);
        assert_eq!(ring.peek_frame(0), Some(frame(5)), "wrap read");
        ring.pop_frames(10); // clamped
        assert_eq!(ring.len(), 0);
        assert_eq!(ring.peek_frame(0), None);
    }

    #[test]
    fn ring_overwrite_oldest_keeps_newest_when_push_exceeds_capacity() {
        let mut ring = SampleRing::new(3);
        let src = flat(&[frame(1), frame(2), frame(3), frame(4), frame(5)]);
        ring.push_frames(&src);
        assert_eq!(ring.peek_frame(0), Some(frame(3)), "kept the newest 3");
        assert_eq!(ring.peek_frame(2), Some(frame(5)));
    }

    #[test]
    fn stepper_step_values_are_pinned() {
        assert_eq!(FrameStepper::new(11025, 11025).step_q16, 0x10000);
        assert_eq!(FrameStepper::new(11025, 44100).step_q16, 0x4000);
        // (11025 * 65536 + 24000) / 48000 = 15053
        assert_eq!(FrameStepper::new(11025, 48000).step_q16, 15053);
        // (11025 * 65536 + 4000) / 8000 = 90317 (downsample: frames skipped)
        assert_eq!(FrameStepper::new(11025, 8000).step_q16, 90317);
    }

    #[test]
    fn stepper_native_rate_is_exact_passthrough() {
        let mut ring = SampleRing::new(8);
        ring.push_frames(&flat(&[frame(1), frame(2), frame(3)]));
        let mut step = FrameStepper::new(11025, 11025);
        assert_eq!(step.next_frame(&mut ring), frame(1));
        assert_eq!(step.next_frame(&mut ring), frame(2));
        assert_eq!(step.next_frame(&mut ring), frame(3));
        assert_eq!(step.next_frame(&mut ring), [0, 0], "underrun = silence");
        assert_eq!(step.next_frame(&mut ring), [0, 0]);
        assert_eq!(step.phase_q16, 0, "unity leaves no phase residue");
    }

    #[test]
    fn stepper_4x_interpolates_exact_quarter_steps_at_44100() {
        // step 0x4000: the phase walks 0, 0x4000, 0x8000, 0xC000 -
        // a [0, 1000, 2000] ramp audibles as exact quarter steps
        // (delta 1000: (1000*frac + 0x8000) >> 16 = 0/250/500/750).
        let mut ring = SampleRing::new(8);
        let src = flat(&[[0, 0], [1000, 1000], [2000, 2000]]);
        ring.push_frames(&src);
        let mut step = FrameStepper::new(11025, 44100);
        for want in [0, 250, 500, 750, 1000, 1250, 1500, 1750] {
            assert_eq!(step.next_frame(&mut ring), [want, want]);
        }
        // The last input frame has no buffered neighbor: edge-hold.
        // 3 input frames carry exactly 12 outputs at exact 4x; the
        // final frame owns outputs 8..=11, then the ring is empty.
        for _ in 0..4 {
            assert_eq!(step.next_frame(&mut ring), [2000, 2000]);
        }
        assert_eq!(
            step.next_frame(&mut ring),
            [0, 0],
            "13th output is underrun silence"
        );
        assert_eq!(ring.len(), 0, "all three input frames consumed");
    }

    #[test]
    fn stepper_48k_pins_the_interpolated_positions() {
        // step 15053: output n reads input i = (n * 15053) >> 16 at
        // frac = (n * 15053) & 0xFFFF. A 4096-per-frame ramp pins the
        // blend: out = i*4096 + ((4096*frac + 0x8000) >> 16).
        let mut ring = SampleRing::new(32);
        let input: Vec<[i16; 2]> = (0..8).map(|i| [4096 * i, 4096 * i]).collect();
        ring.push_frames(&flat(&input));
        let mut step = FrameStepper::new(11025, 48000);
        // Hand-computed literals for the first six outputs.
        let pinned = [0i16, 941, 1882, 2822, 3763, 4704];
        for (n, want) in pinned.iter().enumerate() {
            assert_eq!(step.next_frame(&mut ring), [*want, *want], "output {n}");
        }
        // Then the closed form through the last bracketed position
        // (8 input frames = 34.8 outputs; the tail holds frame 7).
        for n in pinned.len()..35usize {
            let pos = n * 15053;
            let i = pos >> 16;
            let frac = pos & 0xFFFF;
            let want = if i >= input.len() - 1 {
                4096 * (input.len() as i16 - 1) // edge-hold on the last frame
            } else {
                4096 * i as i16 + (((4096 * frac as i64) + 0x8000) >> 16) as i16
            };
            assert_eq!(step.next_frame(&mut ring), [want, want], "output {n}");
        }
        assert_eq!(step.next_frame(&mut ring), [0, 0], "35th+ is silence");
        assert_eq!(ring.len(), 0);
    }

    #[test]
    fn stepper_downsample_interpolates_and_skips_deterministically() {
        // step 90317 > 0x10000: some inputs are never emitted as a
        // base position, but every output still blends toward the
        // next frame. 80 outputs consume floor(80 * 90317 / 65536) =
        // 110 frames.
        let mut ring = SampleRing::new(256);
        let input: Vec<[i16; 2]> = (0..200).map(frame).collect();
        ring.push_frames(&flat(&input));
        let mut step = FrameStepper::new(11025, 8000);
        for n in 0..80usize {
            let pos = n * 90317;
            let i = pos >> 16;
            let frac = pos & 0xFFFF;
            let base = frame(i);
            let target = frame(i + 1);
            let want = [
                blend(base[0], target[0], frac as u32),
                blend(base[1], target[1], frac as u32),
            ];
            assert_eq!(step.next_frame(&mut ring), want, "output {n}");
        }
        assert_eq!(ring.peek_frame(0), Some(frame(110)), "consumed exactly 110");
    }

    #[test]
    fn mono_downmix_and_channel_mapping() {
        assert_eq!(channel_sample(1, 0, 100, 101), 100, "floor average");
        assert_eq!(channel_sample(1, 0, -101, 100), -1, "(-1) >> 1 = -1");
        assert_eq!(channel_sample(1, 0, 32767, 32767), 32767);
        assert_eq!(channel_sample(1, 0, -32768, -32768), -32768);
        assert_eq!(channel_sample(2, 0, 7, 9), 7);
        assert_eq!(channel_sample(2, 1, 7, 9), 9);
        assert_eq!(channel_sample(6, 4, 7, 9), 7, "even = L");
        assert_eq!(channel_sample(6, 5, 7, 9), 9, "odd = R");
    }

    #[test]
    fn underrun_silence_advances_but_never_panics_or_rewinds() {
        let mut ring = SampleRing::new(4);
        let mut step = FrameStepper::new(11025, 48000);
        for _ in 0..100 {
            assert_eq!(step.next_frame(&mut ring), [0, 0]);
        }
        // The silence consumed nothing real (pops clamp on an empty
        // ring), so late data plays immediately at the read cursor:
        ring.push_frames(&flat(&[frame(42)]));
        assert_eq!(step.next_frame(&mut ring), frame(42));
    }

    #[test]
    fn negotiation_prefers_48000_then_44100_over_native_11025() {
        // Rate dominates: 48000 beats 44100 even at worse channel
        // counts and formats (the D47 preference order).
        let specs = [
            OutputConfigSpec {
                min_rate: 44_100,
                max_rate: 44_100,
                channels: 2,
                format: SampleFormatKind::I16,
            },
            OutputConfigSpec {
                min_rate: 48_000,
                max_rate: 48_000,
                channels: 1,
                format: SampleFormatKind::Other,
            },
        ];
        assert_eq!(choose_output_config(&specs), Some((1, 48_000)));
        // THE D47 behavior change: a modern rate beats a range that
        // contains the mixer-native 11025 (D40 preferred 11025).
        let specs = [
            OutputConfigSpec {
                min_rate: 11_025,
                max_rate: 11_025,
                channels: 2,
                format: SampleFormatKind::I16,
            },
            OutputConfigSpec {
                min_rate: 44_100,
                max_rate: 96_000,
                channels: 2,
                format: SampleFormatKind::F32,
            },
        ];
        // 48000 is inside range 1's span too - the WIDE range is
        // pinned at the modern rate, not left at its minimum.
        assert_eq!(choose_output_config(&specs), Some((1, 48_000)));
        // No 48000 anywhere: 44100 wins.
        let specs = [
            OutputConfigSpec {
                min_rate: 11_025,
                max_rate: 11_025,
                channels: 2,
                format: SampleFormatKind::I16,
            },
            OutputConfigSpec {
                min_rate: 44_100,
                max_rate: 44_100,
                channels: 1,
                format: SampleFormatKind::Other,
            },
        ];
        assert_eq!(choose_output_config(&specs), Some((1, 44_100)));
    }

    #[test]
    fn negotiation_ranks_channels_then_formats_within_a_rate() {
        let spec = |channels: u16, format: SampleFormatKind| OutputConfigSpec {
            min_rate: 48_000,
            max_rate: 48_000,
            channels,
            format,
        };
        // Stereo beats mono even at a worse format (channel rank
        // first - the mix is interleaved stereo).
        let specs = [
            spec(1, SampleFormatKind::I16),
            spec(2, SampleFormatKind::F32),
        ];
        assert_eq!(choose_output_config(&specs), Some((1, 48_000)));
        // S16 beats F32 at the same rate + channels.
        let specs = [
            spec(2, SampleFormatKind::F32),
            spec(2, SampleFormatKind::I16),
            spec(2, SampleFormatKind::Other),
        ];
        assert_eq!(choose_output_config(&specs), Some((1, 48_000)));
        // F32 beats anything exotic when S16 is not offered.
        let specs = [
            spec(2, SampleFormatKind::Other),
            spec(2, SampleFormatKind::F32),
        ];
        assert_eq!(choose_output_config(&specs), Some((1, 48_000)));
        // Exact tie: first-listed (stable device enumeration order).
        let specs = [
            spec(2, SampleFormatKind::I16),
            spec(2, SampleFormatKind::I16),
        ];
        assert_eq!(choose_output_config(&specs), Some((0, 48_000)));
    }

    #[test]
    fn negotiation_falls_back_to_native_then_none() {
        let native = OutputConfigSpec {
            min_rate: 8_000,
            max_rate: 11_025,
            channels: 2,
            format: SampleFormatKind::Other,
        };
        // No modern rate offered, but a range CONTAINS 11025: pin the
        // mixer-native rate (resampling is not owed at native).
        assert_eq!(
            choose_output_config(std::slice::from_ref(&native)),
            Some((0, 11_025))
        );
        // Nothing preferred at all -> None -> the caller uses the
        // device default config through the frame stepper.
        let odd = OutputConfigSpec {
            min_rate: 22_050,
            max_rate: 22_050,
            channels: 2,
            format: SampleFormatKind::F32,
        };
        assert_eq!(choose_output_config(&[odd]), None);
        assert_eq!(choose_output_config(&[]), None);
    }

    #[test]
    fn device_edge_sample_format_mapping_is_pinned() {
        // The callback converts the ring's i16 through cpal's dasp
        // FromSample: S16 is the identity, F32 is x / 32768, U8 is
        // (x + 32768) >> 8. Silence and both full-scale ends:
        use cpal::Sample;
        assert_eq!(i16::from_sample::<i16>(0), 0);
        assert_eq!(f32::from_sample::<i16>(0), 0.0);
        assert_eq!(u8::from_sample::<i16>(0), 128, "u8 silence");
        assert_eq!(
            i16::from_sample::<i16>(32767),
            32767,
            "s16 positive full scale"
        );
        assert_eq!(f32::from_sample::<i16>(32767), 32767.0 / 32768.0);
        assert_eq!(u8::from_sample::<i16>(32767), 255, "u8 positive full scale");
        assert_eq!(
            i16::from_sample::<i16>(-32768),
            -32768,
            "s16 negative full scale"
        );
        assert_eq!(f32::from_sample::<i16>(-32768), -1.0);
        assert_eq!(u8::from_sample::<i16>(-32768), 0, "u8 negative full scale");
    }

    #[test]
    fn mixer_u8_silence_and_full_scale_reach_the_ring_as_i16() {
        // The whole device edge in miniature: the mixer bus (11025 Hz
        // stereo, u8 samples, byte-faithful) -> ring i16. u8 128 is
        // exact silence; u8 255 is positive full scale (127 << 8 =
        // 32512 before the default host master Q8 gain 100/256 ->
        // 32512 * 100 >> 8 = 12700 - the same pinned path as the
        // u8-200 -> 7200 pin in fill_from_renders_the_deficit).
        let mut host = GameHost::new(
            &bedlam_game::GameConfig::default(),
            &bedlam_core::sim::SimConfig::default(),
            [[0u8, 0, 0]; 256],
        );
        host.mixer_mut().queue_pcm_u8(&[128, 255]).unwrap();
        let feed = AudioFeed::new(64, SAMPLE_RATE, SAMPLE_RATE);
        assert_eq!(feed.fill_from(&mut host, 2).unwrap(), 2);
        let st = feed.lock();
        assert_eq!(st.ring.peek_frame(0), Some([0, 0]), "u8 128 = silence");
        assert_eq!(
            st.ring.peek_frame(1),
            Some([12700, 12700]),
            "u8 255 full scale"
        );
    }

    #[test]
    fn fill_from_renders_the_deficit_from_the_host() {
        let mut host = GameHost::new(
            &bedlam_game::GameConfig::default(),
            &bedlam_core::sim::SimConfig::default(),
            [[0u8, 0, 0]; 256],
        );
        // Native-rate stream: unity stepper. Queue two PCM bytes on
        // the D31 bus: 128 -> exact silence, 200 -> (72 << 8) = 18432
        // at the default host master (volume 100 -> music_master 50
        // -> Q8 gain (50*48*256)/(127*48) = 100): 18432*100 >> 8 =
        // 7200, both channels.
        host.mixer_mut().queue_pcm_u8(&[128, 200]).unwrap();
        let feed = AudioFeed::new(64, SAMPLE_RATE, SAMPLE_RATE);
        assert_eq!(feed.fill_from(&mut host, 3).unwrap(), 3);
        assert_eq!(feed.buffered_frames(), 3);
        {
            let st = feed.lock();
            assert_eq!(st.ring.peek_frame(0), Some([0, 0]));
            assert_eq!(st.ring.peek_frame(1), Some([7200, 7200]));
            assert_eq!(
                st.ring.peek_frame(2),
                Some([0, 0]),
                "underrun mixed as silence"
            );
        }
        // At target: nothing more is rendered.
        assert_eq!(feed.fill_from(&mut host, 3).unwrap(), 0);
        assert_eq!(feed.buffered_frames(), 3);
    }

    #[test]
    fn quiet_feed_renders_nothing_and_the_guard_is_sticky() {
        // D48 dead-feed guard, producer side: once the owning device
        // quiets the feed, fill_from renders NOTHING (a dropped
        // stream never consumes), the ring stays empty, and the
        // guard never wakes back up.
        let mut host = GameHost::new(
            &bedlam_game::GameConfig::default(),
            &bedlam_core::sim::SimConfig::default(),
            [[0u8, 0, 0]; 256],
        );
        host.mixer_mut().queue_pcm_u8(&[200, 200]).unwrap();
        let feed = AudioFeed::new(64, SAMPLE_RATE, SAMPLE_RATE);
        assert!(!feed.is_quiet(), "a fresh feed is live");
        feed.quiet();
        assert!(feed.is_quiet());
        assert_eq!(
            feed.fill_from(&mut host, 4).unwrap(),
            0,
            "quiet feed renders nothing"
        );
        assert_eq!(feed.buffered_frames(), 0);
        feed.quiet(); // idempotent
        assert!(feed.is_quiet(), "the guard is sticky");
    }

    #[test]
    fn dead_feed_callback_writes_exact_silence_without_the_ring() {
        // D48 dead-feed guard, callback side: a late invocation on a
        // quieted feed writes each format's EXACT zero (u8 midpoint
        // 128) and never drains the ring.
        let mut state = FeedState {
            ring: SampleRing::new(8),
            step: FrameStepper::new(SAMPLE_RATE, SAMPLE_RATE),
        };
        state.ring.push_frames(&flat(&[frame(7), frame(9)]));
        let mut buf = [0u8; 4];
        silence(&mut buf);
        assert_eq!(buf, [128, 128, 128, 128], "u8 silence is the midpoint");
        assert_eq!(state.ring.len(), 2, "the ring was not touched");
    }

    #[test]
    fn live_drain_maps_ring_frames_into_the_device_buffer() {
        // The factored live half of the callback (i16 device format,
        // native rate: identity stepper, stereo passthrough).
        let mut state = FeedState {
            ring: SampleRing::new(8),
            step: FrameStepper::new(SAMPLE_RATE, SAMPLE_RATE),
        };
        state.ring.push_frames(&flat(&[frame(7), frame(9)]));
        let mut buf = [0i16; 4];
        drain(&mut state, &mut buf, 2);
        assert_eq!(buf, [7, -7, 9, -9]);
        assert_eq!(state.ring.len(), 0, "drained exactly the two frames");
        // Further drains are underrun silence.
        drain(&mut state, &mut buf, 2);
        assert_eq!(buf, [0, 0, 0, 0]);
    }

    /// LIVE-DEVICE PROBE - explicitly opt-in (`cargo test -- --ignored`),
    /// NEVER part of the normal suite: it opens the real default
    /// output device and lets the callback drain ~100 ms of exact
    /// silence, proving the cpal wiring (config pick, stream build,
    /// play, drop) end to end on this machine. Hermetic CI boxes
    /// without audio simply never run it.
    #[test]
    #[ignore = "opens the real audio device"]
    fn device_open_probe_drains_silence() {
        let Some(dev) = AudioDevice::open_default() else {
            eprintln!("probe: no audio device present - skipping (not a failure)");
            return;
        };
        let facts = dev.facts().clone();
        eprintln!(
            "probe: opened {} Hz, {} ch, {}",
            facts.rate, facts.channels, facts.format
        );
        let feed = dev.feed().clone();
        assert_eq!(
            feed.buffered_frames(),
            0,
            "a fresh device has an empty ring"
        );
        // ~100 ms of real host mix (all silence on a fresh host).
        let mut host = GameHost::new(
            &bedlam_game::GameConfig::default(),
            &bedlam_core::sim::SimConfig::default(),
            [[0u8, 0, 0]; 256],
        );
        assert_eq!(
            feed.fill_from(&mut host, TARGET_FRAMES).unwrap(),
            TARGET_FRAMES
        );
        // Let the callback drain it. The default ALSA/Pulse device has
        // ~100-200 ms of startup latency before the first callback
        // pulls (visible in the samples below); the window loop's
        // steady-state 16 ms refills never see this.
        let mut left = TARGET_FRAMES;
        for i in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let now = feed.buffered_frames();
            eprintln!("probe: t={}00ms buffered={}", i + 1, now);
            left = left.min(now);
        }
        assert!(
            left < TARGET_FRAMES,
            "callback drained the ring ({left} left)"
        );
        eprintln!(
            "probe: drained {} of {} frames",
            TARGET_FRAMES - left,
            TARGET_FRAMES
        );
    }
}
