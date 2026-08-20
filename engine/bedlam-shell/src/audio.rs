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
//! turn the callback into an error storm).
//!
//! Device-feed arithmetic (all integer, all unit-pinned):
//! - The stream is opened at the mixer-native 11025 Hz whenever any
//!   supported config range contains it (resampling is NOT owed at
//!   the native rate); otherwise the default config runs through a
//!   Q16 nearest-neighbor frame stepper (the classic sample-hold:
//!   output n reads input floor(n * step / 65536)).
//! - Underrun is EXACT silence ([0, 0] frames), matching the mixer
//!   bus semantics; a full ring drops the OLDEST frames (lateness
//!   is skipped, never accumulated).
//! - Channel mapping: mono devices take the floor average
//!   `(l + r) >> 1`; stereo passes through; >2 channels repeat
//!   L/R alternately (even = L, odd = R).
//! - Sample format conversion goes through cpal's `Sample`
//!   conversions (dasp); no floating point exists on the produce
//!   path, only inside the device callback.

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

/// Q16 nearest-neighbor frame stepper: output n reads input frame
/// floor(n * step / 65536), step = round(src * 65536 / dst). At the
/// native rate (step 0x10000) this is an exact 1:1 pass-through
/// with zero phase residue; at 4x (44100) each frame repeats
/// exactly 4 times; anything else deterministically sample-holds.
#[derive(Debug)]
pub(crate) struct FrameStepper {
    step_q16: u32,
    phase_q16: u64,
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

    /// Produce one output frame from the ring head, then consume the
    /// input frames the output has fully passed. An empty ring
    /// yields EXACT silence and still advances the phase (a skipped
    /// stretch stays skipped once the data returns - streaming
    /// semantics, deterministic).
    fn next_frame(&mut self, ring: &mut SampleRing) -> [i16; 2] {
        let frame = ring.peek_frame(0).unwrap_or([0, 0]);
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
#[derive(Debug)]
pub struct AudioFeed {
    state: Arc<Mutex<FeedState>>,
}

impl AudioFeed {
    pub(crate) fn new(cap: usize, src_rate: u32, dst_rate: u32) -> AudioFeed {
        AudioFeed {
            state: Arc::new(Mutex::new(FeedState {
                ring: SampleRing::new(cap),
                step: FrameStepper::new(src_rate, dst_rate),
            })),
        }
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
    /// ring is already at target). A render error propagates - the
    /// caller decides whether that is fatal (it is not, for audio).
    pub fn fill_from(&self, host: &mut GameHost, target: usize) -> Result<usize, GameError> {
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
    /// Open the default output device, preferring a config at the
    /// mixer-native 11025 Hz (stereo, then mono, then any channel
    /// count whose supported range contains the rate). With no such
    /// range the device default runs through the frame stepper.
    /// `None` means no usable audio (the shell continues silent).
    pub fn open_default() -> Option<AudioDevice> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let mut chosen: Option<cpal::SupportedStreamConfig> = None;
        if let Ok(ranges) = device.supported_output_configs() {
            let mut best: Option<cpal::SupportedStreamConfigRange> = None;
            for range in ranges {
                if !range.contains_rate(SAMPLE_RATE) {
                    continue;
                }
                let better = match &best {
                    None => true,
                    Some(b) => channel_pref(range.channels()) < channel_pref(b.channels()),
                };
                if better {
                    best = Some(range);
                }
            }
            if let Some(range) = best {
                chosen = range.try_with_sample_rate(SAMPLE_RATE);
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
        let channels = usize::from(stream_config.channels);
        let report_error = |err| eprintln!("bedlam-shell: audio stream error: {err}");
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                stream_config,
                callback::<f32>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::F64 => device.build_output_stream(
                stream_config,
                callback::<f64>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::I8 => device.build_output_stream(
                stream_config,
                callback::<i8>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                stream_config,
                callback::<i16>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::I32 => device.build_output_stream(
                stream_config,
                callback::<i32>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::I64 => device.build_output_stream(
                stream_config,
                callback::<i64>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::U8 => device.build_output_stream(
                stream_config,
                callback::<u8>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                stream_config,
                callback::<u16>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::U32 => device.build_output_stream(
                stream_config,
                callback::<u32>(state, channels),
                report_error,
                None,
            ),
            cpal::SampleFormat::U64 => device.build_output_stream(
                stream_config,
                callback::<u64>(state, channels),
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
    /// Stop the stream deterministically (the field is also what
    /// keeps cpal pumping - this just makes the lifetime contract
    /// explicit at the drop site).
    fn drop(&mut self) {
        let _ = self.stream.pause();
    }
}

/// The device callback: drain ring frames into the interleaved
/// device buffer through the stepper + channel mapping + sample
/// conversion. Underrun frames are exact silence.
fn callback<S: cpal::SizedSample + cpal::FromSample<i16>>(
    state: Arc<Mutex<FeedState>>,
    channels: usize,
) -> impl FnMut(&mut [S], &cpal::OutputCallbackInfo) + Send + 'static {
    move |data, _info| {
        let mut st = state.lock().unwrap_or_else(|p| p.into_inner());
        let FeedState { ring, step } = &mut *st;
        for chunk in data.chunks_mut(channels.max(1)) {
            let frame = step.next_frame(ring);
            for (c, out) in chunk.iter_mut().enumerate() {
                *out = S::from_sample::<i16>(channel_sample(channels, c, frame[0], frame[1]));
            }
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
    fn stepper_4x_repeats_each_frame_exactly_four_times() {
        let mut ring = SampleRing::new(8);
        ring.push_frames(&flat(&[frame(1), frame(2)]));
        let mut step = FrameStepper::new(11025, 44100);
        for expected in [frame(1), frame(1), frame(1), frame(1)] {
            assert_eq!(step.next_frame(&mut ring), expected);
        }
        for expected in [frame(2), frame(2), frame(2), frame(2)] {
            assert_eq!(step.next_frame(&mut ring), expected);
        }
        assert_eq!(ring.len(), 0, "both input frames consumed");
    }

    #[test]
    fn stepper_48k_pins_the_sample_hold_positions() {
        // Output n reads input floor(n * 15053 / 65536): frames 0 and
        // 1 each hold 5 outputs, then 4s. 10 input frames carry
        // exactly 44 outputs (n = 0..=43); output 44 hits underrun.
        let mut ring = SampleRing::new(32);
        let input: Vec<[i16; 2]> = (0..10).map(frame).collect();
        ring.push_frames(&flat(&input));
        let mut step = FrameStepper::new(11025, 48000);
        let mut audibled = 0;
        for n in 0..44usize {
            let want = (n * 15053) / 65536;
            assert_eq!(step.next_frame(&mut ring), input[want], "output {n}");
            audibled += 1;
        }
        assert_eq!(audibled, 44);
        assert_eq!(step.next_frame(&mut ring), [0, 0], "44th+ is silence");
        assert_eq!(ring.len(), 0);
    }

    #[test]
    fn stepper_downsample_skips_frames_deterministically() {
        // step 90317 > 0x10000: some inputs are never emitted.
        // 80 outputs consume floor(80 * 90317 / 65536) = 110 frames.
        let mut ring = SampleRing::new(256);
        let input: Vec<[i16; 2]> = (0..200).map(frame).collect();
        ring.push_frames(&flat(&input));
        let mut step = FrameStepper::new(11025, 8000);
        for n in 0..80usize {
            let want = (n * 90317) / 65536;
            assert_eq!(step.next_frame(&mut ring), input[want], "output {n}");
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
}
