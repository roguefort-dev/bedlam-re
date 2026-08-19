//! The mix graph (DESIGN-AUDIO secs 3-8): a flat pool of voices with
//! per-voice Q16 resamplers and Q8 gains, summed on an i32 master bus and
//! symmetric-clamped into interleaved S16 stereo output.
//!
//! Everything here is integer math. The host asks for N frames; nothing
//! inside converts time (dt never enters mix math, D17 bucket b), and the
//! script dispatch grid is Q16-exact so any host buffer chunking yields the
//! identical byte stream.
//!
//! One queued BYTE-STREAM channel exists alongside the voice pool (D31):
//! native-format PCM (11025 Hz 8-bit unsigned mono, the format the .SMK
//! audio tracks decode to and the .RAW SFX files store, RE-EXW-MUSIC sec
//! 3) appended in order and mixed at the cursor. It exists for movie
//! playback and host-timed SFX - the one audio source the original drives
//! as a continuous buffer rather than pitched sub-voices.

use std::collections::VecDeque;

use crate::script::{tick_pos_q16, MusicCommand, MusicScript};
use crate::AudioError;

/// Native mix rate, both builds (EXW SetFrequency 0x2b11 base + B2
/// IRQ0-shared PCM driver; DESIGN-AUDIO sec 2 fact 1).
pub const SAMPLE_RATE: u32 = 11025;
/// Flat voice pool size (B2 PcmMixerService 20-channel walker, fact 7).
pub const MAX_VOICES: usize = 20;
/// Concurrent sub-voices per instrument (EXW mrw_load primes 4, fact 2).
pub const SUB_VOICES_PER_INST: usize = 4;

/// EXW master volume scale (g_music_master_vol 0..=127, fact 3).
const MASTER_MAX: u32 = 127;
/// EXW volume divisor 0x30 (fact 3): the gain domain is (master * vol) / 48.
const VOL_DIVISOR: u32 = 48;
/// Unity gain in Q8. Nothing amplifies: the DirectSound domain the original
/// drives is attenuation-only, so unity is the ceiling everywhere.
const GAIN_UNITY_Q8: u32 = 256;

/// Pan domain bound; 64 is the center base of the linear balance law.
const PAN_MAX: i32 = 63;
const PAN_CENTER_BASE: i32 = 64;
/// Q8 gain step per pan unit (256 / 64).
const PAN_GAIN_STEP: i32 = 4;

/// Q16 one output sample.
const Q16_ONE: u64 = 1 << 16;

/// Upper bound on queued stream (movie) samples (~5.9 s at 11025 Hz):
/// a host that stops pulling must not grow the queue without limit.
/// Dropping the tail is the deterministic overflow policy (D31).
pub const PCM_STREAM_CAP: usize = 65_536;

/// Identifies a sounding sub-voice the way the EXW voice table does:
/// instrument id plus sub-voice index, where sub 0 is the BASE buffer a
/// note-off releases (RE-EXW-MUSIC sec 6 quirk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceRef {
    pub instrument: u16,
    pub sub: usize,
}

/// One voice slot. One-shot like DirectSound Play(0, 0, 0): the voice
/// deactivates itself when the playhead passes the wave end.
#[derive(Debug, Clone)]
struct Voice {
    instrument: u16,
    sub: usize,
    active: bool,
    /// 16.16 playhead position in source samples.
    phase_q16: u64,
    /// 16.16 per-output-sample advance = the RATIO_TABLE value verbatim
    /// (unity 0x10000 replays the wave at 11025 Hz, which is the same fact
    /// as EXW SetFrequency (ratio * 0x2b11) >> 16).
    step_q16: u32,
    /// Combined Q8 gains (volume x pan) snapshotted at spawn: the original
    /// reads g_music_master_vol per SubVoiceStart only, so mid-note master
    /// changes never touch sounding DS buffers either.
    left_q8: u32,
    right_q8: u32,
}

impl Voice {
    fn idle() -> Self {
        Voice {
            instrument: 0,
            sub: 0,
            active: false,
            phase_q16: 0,
            step_q16: 0,
            left_q8: 0,
            right_q8: 0,
        }
    }
}

/// Q8 linear volume gain from the EXW volume product (D25): monotone in
/// (master * vol) / 48, clamped to unity. The DS hundredths-of-dB curve
/// stays an API-level documented fact; P4 parity is a correlation band.
fn volume_gain_q8(master: u8, volume: u8) -> u32 {
    let g = (master as u32 * volume as u32 * GAIN_UNITY_Q8) / (MASTER_MAX * VOL_DIVISOR);
    g.min(GAIN_UNITY_Q8)
}

/// Q8 pan gains (linear balance, DESIGN-AUDIO sec 6): left =
/// (64 - pan) * 4, right = (64 + pan) * 4, both clamped to unity. The
/// shipped EXW always passes 0 (SetPan(0) in SubVoiceStart).
fn pan_gains_q8(pan: i8) -> (u32, u32) {
    let p = (pan as i32).clamp(-PAN_MAX, PAN_MAX);
    let l = ((PAN_CENTER_BASE - p) * PAN_GAIN_STEP) as u32;
    let r = ((PAN_CENTER_BASE + p) * PAN_GAIN_STEP) as u32;
    (l.min(GAIN_UNITY_Q8), r.min(GAIN_UNITY_Q8))
}

/// Symmetric saturation at the master bus (DESIGN-AUDIO sec 8). The
/// original driver-side shape is unknown (open question Q4); a symmetric
/// clamp is the standard PCM behavior and the correlation band tolerates it.
fn clamp_i16(x: i32) -> i16 {
    x.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

/// The hermetic mixer. Construct, load waves, optionally load a music
/// script, then call render once per host frame with the device-side
/// buffer. Same inputs plus same call sequence give the identical byte
/// stream regardless of how the host chunks its render calls.
#[derive(Debug, Clone)]
pub struct Mixer {
    /// Instrument waves, centered i16 ((b - 128) << 8), indexed by
    /// instrument id. A missing or empty entry means the instrument has no
    /// buffer: note_on drops it, mirroring the EXW ptr-non-null guard.
    waves: Vec<Vec<i16>>,
    /// Fixed pool; exactly MAX_VOICES entries for the life of the mixer.
    voices: Vec<Voice>,
    /// Master volume 0..=127 (g_music_master_vol analog; the EXW UI writes
    /// 0..50 into it). Read at note spawn only (see Voice field docs).
    master: u8,
    script: Option<MusicScript>,
    script_next: usize,
    /// Cursor anchor captured at load_script time: the script tick 0
    /// aligns with the frame where the script ATTACHED, so a script
    /// swapped in mid-stream (every scene change, DESIGN-GAME sec 5)
    /// plays from its top instead of dumping every past event at once.
    script_base_q16: u64,
    /// Absolute output position in Q16 samples since construction.
    cursor_q16: u64,
    /// Queued byte-stream PCM in native format (u8), consumed at one
    /// byte per output frame. Host-paced (D17 bucket b): bytes enter
    /// via queue_pcm_u8 in decode order and mix at the cursor, so the
    /// mix depends only on the queue-vs-render SEQUENCE, not on host
    /// chunking.
    pcm: Vec<u8>,
    /// Read cursor into `pcm`.
    pcm_pos: usize,
}

impl Default for Mixer {
    fn default() -> Self {
        Self::new()
    }
}

impl Mixer {
    pub fn new() -> Self {
        Mixer {
            waves: Vec::new(),
            voices: (0..MAX_VOICES).map(|_| Voice::idle()).collect(),
            master: MASTER_MAX as u8,
            script: None,
            script_next: 0,
            script_base_q16: 0,
            cursor_q16: 0,
            pcm: Vec::new(),
            pcm_pos: 0,
        }
    }

    /// Load (or replace) the wave of one instrument. pcm_u8 is raw 8-bit
    /// unsigned mono PCM as stored in an .MRW record / RAW SFX file
    /// (RE-EXW-MUSIC sec 3: 11025 Hz 8-bit mono, VERIFIED).
    pub fn load_wave(&mut self, instrument: u16, pcm_u8: &[u8]) -> Result<(), AudioError> {
        if pcm_u8.is_empty() {
            return Err(AudioError::EmptyWave { instrument });
        }
        let idx = instrument as usize;
        if idx >= self.waves.len() {
            self.waves.resize(idx + 1, Vec::new());
        }
        self.waves[idx] = pcm_u8.iter().map(|&b| ((b as i16) - 128) * 256).collect();
        Ok(())
    }

    /// Set the master volume knob, 0..=127 (values above clamp, matching
    /// the 0..127 scale of g_music_master_vol). Affects future note spawns
    /// only - the EXW semantic, see Voice field docs.
    pub fn set_master_volume(&mut self, master: u8) {
        self.master = master.min(MASTER_MAX as u8);
    }

    /// Queue decoded stream PCM on the movie bus: raw 8-bit unsigned
    /// mono at the native 11025 Hz rate (the Smacker DPCM/Raw track
    /// decode output, already byte-decoded by bedlam-assets). Samples
    /// play FIFO at unity gain on both channels underneath the voices
    /// and are consumed one per stereo output frame. Returns the number
    /// of accepted samples; a full queue (PCM_STREAM_CAP) drops the
    /// tail - the deterministic overflow policy for a host that stops
    /// pulling (D31).
    pub fn queue_pcm_mono8(&mut self, pcm: &[u8]) -> usize {
        let room = PCM_STREAM_CAP.saturating_sub(self.pcm_stream.len());
        let take = room.min(pcm.len());
        self.pcm_stream
            .extend(pcm[..take].iter().map(|&b| ((b as i16) - 128) * 256));
        take
    }

    /// Samples currently queued on the movie bus.
    pub fn pcm_stream_len(&self) -> usize {
        self.pcm_stream.len()
    }

    /// Drop everything queued on the movie bus (movie detach).
    pub fn clear_pcm_stream(&mut self) {
        self.pcm_stream.clear();
    }

    /// Append bytes to the queued PCM stream (movie audio / host-timed
    /// SFX, D31). Native format only: 11025 Hz 8-bit unsigned mono - the
    /// decoded output of TITLE.SMK track 0 is exactly this (D30 gate).
    /// Order is the only contract: bytes mix strictly in queue order,
    /// one byte per output frame, at the master-volume gain of the
    /// sample being mixed (a stream has no spawn point to snapshot).
    pub fn queue_pcm_u8(&mut self, pcm: &[u8]) {
        self.pcm.extend_from_slice(pcm);
    }

    /// Bytes queued but not yet mixed.
    pub fn pcm_pending(&self) -> usize {
        self.pcm.len() - self.pcm_pos
    }

    /// Drop the queued stream (movie stop / scene change). Sounding
    /// voices are untouched; only the byte stream silences.
    pub fn clear_pcm_stream(&mut self) {
        self.pcm.clear();
        self.pcm_pos = 0;
    }

    /// Install the music script (the walked .MRS stream). Dispatch is
    /// ANCHORED at the current cursor: tick 0 = the frame where the script
    /// attaches, so a script swapped in mid-stream (every scene change,
    /// DESIGN-GAME sec 5) plays from its top; absolute-cursor dispatch
    /// would fire its whole past at once. Chunking-invariant via the Q16
    /// grid (the anchor is always a whole frame).
    pub fn load_script(&mut self, script: MusicScript) {
        self.script = Some(script);
        self.script_next = 0;
        self.script_base_q16 = self.cursor_q16;
    }

    /// Trigger one note on the lowest free sub-voice of the instrument:
    /// the EXW SubVoiceFind probe order (base..base+3, first free wins,
    /// note dropped when all four are busy). Pan is center, as in every
    /// SubVoiceStart call the shipped EXW makes.
    pub fn note_on(&mut self, instrument: u16, ratio: u32, volume: u8) -> Option<VoiceRef> {
        self.note_on_pan(instrument, ratio, volume, 0)
    }

    /// General trigger with a pan argument (future/host use; the original
    /// never pans off-center - DESIGN-AUDIO sec 6).
    pub fn note_on_pan(
        &mut self,
        instrument: u16,
        ratio: u32,
        volume: u8,
        pan: i8,
    ) -> Option<VoiceRef> {
        if self
            .waves
            .get(instrument as usize)
            .is_none_or(|w| w.is_empty())
        {
            return None; // no buffer: the EXW ptr-non-null guard analog
        }
        let mut sub = None;
        for s in 0..SUB_VOICES_PER_INST {
            let busy = self
                .voices
                .iter()
                .any(|v| v.active && v.instrument == instrument && v.sub == s);
            if !busy {
                sub = Some(s);
                break;
            }
        }
        let sub = sub?; // all four sub-voices busy: drop the note (EXW drops)
        let slot = self.voices.iter().position(|v| !v.active)?; // pool full
        let gain = volume_gain_q8(self.master, volume);
        let (pl, pr) = pan_gains_q8(pan);
        // Q3 policy: a zero ratio (RATIO_TABLE floor) mutes the voice but
        // still occupies the slot - see DESIGN-AUDIO sec 6 / open Q3.
        let mute = ratio == 0;
        let v = &mut self.voices[slot];
        v.instrument = instrument;
        v.sub = sub;
        v.active = true;
        v.phase_q16 = 0;
        v.step_q16 = ratio;
        v.left_q8 = if mute { 0 } else { (gain * pl) >> 8 };
        v.right_q8 = if mute { 0 } else { (gain * pr) >> 8 };
        Some(VoiceRef { instrument, sub })
    }

    /// Release the BASE (sub == 0) sub-voice of an instrument - the
    /// faithful EXW quirk: a 0xFF-volume note-off calls DSReleaseVoice on
    /// the instrument base buffer, whichever sub-slot is sounding; subs
    /// 1..3 ring out until their waves end. Returns true if a base voice
    /// was sounding.
    pub fn note_off(&mut self, instrument: u16) -> bool {
        let mut stopped = false;
        for v in self.voices.iter_mut() {
            if v.active && v.instrument == instrument && v.sub == 0 {
                v.active = false;
                stopped = true;
            }
        }
        stopped
    }

    /// Sub-voice sounding status (the SubVoiceProbe GetStatus analog).
    pub fn voice_playing(&self, instrument: u16, sub: usize) -> bool {
        self.voices
            .iter()
            .any(|v| v.active && v.instrument == instrument && v.sub == sub)
    }

    /// Mix floor-free stereo frames into out (interleaved L, R), advancing
    /// the cursor, dispatching due script events at their exact Q16 sample
    /// positions (quantized down onto a sample boundary - chunking-
    /// invariant), and returning the number of frames mixed.
    pub fn render(&mut self, out: &mut [i16]) -> Result<usize, AudioError> {
        if !out.len().is_multiple_of(2) {
            return Err(AudioError::OddBufferLength { len: out.len() });
        }
        let frames = out.len() / 2;
        for f in 0..frames {
            // Dispatch with the cursor AT the position of the frame about
            // to mix: an event on an exact sample boundary s contributes to
            // frame s, a fractional position at the first frame past it
            // (DESIGN-AUDIO sec 5 - the cursor reaches the event). The
            // cursor advances after mixing, never before.
            self.dispatch_due();
            let (l, r) = self.mix_one();
            out[2 * f] = l;
            out[2 * f + 1] = r;
            self.cursor_q16 += Q16_ONE;
        }
        Ok(frames)
    }

    /// Fire every not-yet-dispatched script event whose Q16 position is at
    /// or before the cursor. Borrow-safe: the command is Copy.
    fn dispatch_due(&mut self) {
        loop {
            let fire = match self
                .script
                .as_ref()
                .and_then(|s| s.events().get(self.script_next))
            {
                Some(&(tick, _)) => self.script_base_q16 + tick_pos_q16(tick) <= self.cursor_q16,
                None => false,
            };
            if !fire {
                return;
            }
            let cmd = self.script.as_ref().unwrap().events()[self.script_next].1;
            self.script_next += 1;
            match cmd {
                MusicCommand::NoteOn {
                    instrument,
                    ratio,
                    volume,
                } => {
                    self.note_on(instrument, ratio, volume);
                }
                MusicCommand::NoteOff { instrument } => {
                    self.note_off(instrument);
                }
            }
        }
    }

    /// Accumulate one output sample over all active voices (one-shot: a
    /// voice past its wave end deactivates itself) and clamp.
    fn mix_one(&mut self) -> (i16, i16) {
        let mut l: i32 = 0;
        let mut r: i32 = 0;
        // Queued stream first: one native byte per output frame, gain
        // read at mix time (no spawn point exists to snapshot, D31).
        // Master-gain domain: volume 48 = unity on the 127 master scale.
        if self.pcm_pos < self.pcm.len() {
            let s = (i32::from(self.pcm[self.pcm_pos]) - 128) << 8;
            let g = volume_gain_q8(self.master, 48) as i32;
            l += (s * g) >> 8;
            r += (s * g) >> 8;
            self.pcm_pos += 1;
            if self.pcm_pos == self.pcm.len() {
                // Compacted on drain so a finished movie does not pin
                // its bytes; an immediately re-queued packet starts at 0.
                self.pcm.clear();
                self.pcm_pos = 0;
            }
        }
        for v in self.voices.iter_mut() {
            if !v.active {
                continue;
            }
            let wave = match self.waves.get(v.instrument as usize) {
                Some(w) if !w.is_empty() => w,
                _ => {
                    v.active = false; // unloaded mid-flight: stop cleanly
                    continue;
                }
            };
            let idx = (v.phase_q16 >> 16) as usize;
            if idx >= wave.len() {
                v.active = false; // played out (non-looping Play(0,0,0))
                continue;
            }
            let s = wave[idx] as i32;
            l += (s * v.left_q8 as i32) >> 8;
            r += (s * v.right_q8 as i32) >> 8;
            v.phase_q16 += v.step_q16 as u64;
            if (v.phase_q16 >> 16) as usize >= wave.len() {
                v.active = false; // played out: free right after the last sample
            }
        }
        if let Some(s) = self.pcm_stream.pop_front() {
            l += s as i32;
            r += s as i32;
        }
        (clamp_i16(l), clamp_i16(r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_gain_edges() {
        assert_eq!(volume_gain_q8(0, 42), 0, "master 0 is silence");
        assert_eq!(volume_gain_q8(127, 42), 224, "42 * 256 / 48");
        assert_eq!(volume_gain_q8(50, 42), 88);
        assert_eq!(volume_gain_q8(127, 48), 256, "unity point");
        assert_eq!(volume_gain_q8(127, 255), 256, "clamped at unity");
    }

    #[test]
    fn pan_law() {
        assert_eq!(pan_gains_q8(0), (256, 256));
        assert_eq!(pan_gains_q8(63), (4, 256), "right clamps at unity");
        assert_eq!(pan_gains_q8(-63), (256, 4));
        assert_eq!(pan_gains_q8(127), pan_gains_q8(63), "input clamped");
        assert_eq!(pan_gains_q8(-127), pan_gains_q8(-63));
    }

    #[test]
    fn clamp_is_symmetric() {
        assert_eq!(clamp_i16(40000), 32767);
        assert_eq!(clamp_i16(-40000), -32768);
        assert_eq!(clamp_i16(1234), 1234);
    }

    #[test]
    fn master_volume_clamps_into_domain() {
        let mut m = Mixer::new();
        m.set_master_volume(200);
        assert_eq!(volume_gain_q8(127, 48), 256); // set_master_volume(200) clamps here
        let mut m2 = Mixer::new();
        m2.load_wave(0, &[128, 200, 128, 56]).unwrap();
        m2.set_master_volume(0);
        let r = m2.note_on(0, 0x10000, 48);
        assert!(r.is_some(), "spawn still succeeds when muted");
    }

    #[test]
    fn empty_wave_is_an_error() {
        let mut m = Mixer::new();
        assert!(matches!(
            m.load_wave(3, &[]),
            Err(AudioError::EmptyWave { instrument: 3 })
        ));
    }

    #[test]
    fn odd_buffer_is_rejected() {
        let mut m = Mixer::new();
        let mut out = [0i16; 7];
        assert!(matches!(
            m.render(&mut out),
            Err(AudioError::OddBufferLength { len: 7 })
        ));
    }


    #[test]
    fn pcm_stream_mixes_native_bytes_at_unity_when_master_max() {
        let mut m = Mixer::new(); // master 127 => volume 48 = unity
        m.queue_pcm_u8(&[128, 255, 0, 128]);
        let mut out = [1i16; 8];
        m.render(&mut out).unwrap();
        // (b - 128) << 8 exactly, on both channels, one byte per frame.
        assert_eq!(
            out,
            [0, 0, 32512, 32512, -32768, -32768, 0, 0]
        );
        assert_eq!(m.pcm_pending(), 0, "drained");
    }

    #[test]
    fn pcm_stream_gain_follows_master_at_mix_time() {
        let mut quiet = Mixer::new();
        quiet.set_master_volume(0);
        quiet.queue_pcm_u8(&[255, 255, 255, 255]);
        let mut out = [1i16; 4];
        quiet.render(&mut out).unwrap();
        assert!(out.iter().all(|&s| s == 0), "muted master silences stream");
    }

    #[test]
    fn pcm_stream_is_chunking_invariant() {
        let bytes: Vec<u8> = (0..100u32).map(|i| (i * 7 + 13) as u8).collect();
        let run = |frames_per_call: usize| -> Vec<i16> {
            let mut m = Mixer::new();
            m.queue_pcm_u8(&bytes);
            let mut acc = Vec::new();
            while m.pcm_pending() > 0 {
                let n = m.pcm_pending().min(frames_per_call);
                let mut buf = vec![0i16; n * 2];
                m.render(&mut buf).unwrap();
                acc.extend_from_slice(&buf);
            }
            acc
        };
        let a = run(1);
        let b = run(7);
        let c = run(64);
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a.len(), 200);
    }

    #[test]
    fn pcm_stream_mixes_under_a_sounding_voice() {
        let mut m = Mixer::new();
        m.load_wave(0, &[128, 128, 128, 128]).unwrap(); // silent wave
        m.note_on(0, 0x10000, 48).unwrap();
        m.queue_pcm_u8(&[255]);
        let mut out = [0i16; 2];
        m.render(&mut out).unwrap();
        assert_eq!(out, [32512, 32512], "voice contributes zeros, stream passes");
    }

    #[test]
    fn clear_pcm_stream_silences_and_frees() {
        let mut m = Mixer::new();
        m.queue_pcm_u8(&[200, 200, 200, 200]);
        m.clear_pcm_stream();
        assert_eq!(m.pcm_pending(), 0);
        let mut out = [1i16; 8];
        m.render(&mut out).unwrap();
        assert!(out.iter().all(|&s| s == 0));
        // Re-queue after clear starts from the top.
        m.queue_pcm_u8(&[128, 255]);
        let mut out = [0i16; 4];
        m.render(&mut out).unwrap();
        assert_eq!(out[2], 32512, "second byte of the new queue");
    }

    #[test]
    fn pcm_stream_requeue_after_drain_appends_fresh() {
        let mut m = Mixer::new();
        m.queue_pcm_u8(&[10, 20]);
        let mut out = [0i16; 4];
        m.render(&mut out).unwrap();
        assert!(m.pcm_pending() == 0);
        // Drain compaction must not lose a subsequently queued packet.
        m.queue_pcm_u8(&[30]);
        let mut out = [0i16; 2];
        m.render(&mut out).unwrap();
        let want = ((30i32 - 128) << 8) as i16;
        assert_eq!(out, [want, want]);
    }

    #[test]
    fn stream_bus_plays_fifo_then_underruns() {
        let mut m = Mixer::new();
        let mut out = [0i16; 8];
        // underrun is exact silence
        m.render(&mut out).unwrap();
        assert!(out.iter().all(|&s| s == 0));
        // 200, 128, 100 centered: 18432, 0, -7168; then underrun zeros
        assert_eq!(m.queue_pcm_mono8(&[200, 128, 100]), 3);
        m.render(&mut out).unwrap();
        assert_eq!(
            &out[..8],
            &[18432, 18432, 0, 0, -7168, -7168, 0, 0]
        );
        assert_eq!(m.pcm_stream_len(), 0);
    }

    #[test]
    fn stream_bus_is_chunking_invariant() {
        let run = |chunk: usize| -> Vec<i16> {
            let mut m = Mixer::new();
            m.queue_pcm_mono8(&[148, 168, 98, 68, 135, 121]);
            let mut out = Vec::new();
            while m.pcm_stream_len() > 0 || out.len() < 12 {
                let mut buf = vec![7i16; chunk * 2]; // poison: render must overwrite
                m.render(&mut buf).unwrap();
                out.extend_from_slice(&buf);
            }
            out
        };
        let want: Vec<i16> = vec![
            5120, 5120, 10240, 10240, -7680, -7680, -15360, -15360, 1792, 1792, -1792, -1792,
        ];
        for chunk in [1usize, 7, 64] {
            assert_eq!(&run(chunk)[..12], want.as_slice(), "chunk {chunk}");
        }
    }

    #[test]
    fn stream_bus_cap_drops_tail_deterministically() {
        let mut m = Mixer::new();
        let big: Vec<u8> = (0..PCM_STREAM_CAP + 10).map(|i| (i % 251) as u8).collect();
        assert_eq!(m.queue_pcm_mono8(&big), PCM_STREAM_CAP);
        assert_eq!(m.pcm_stream_len(), PCM_STREAM_CAP);
        assert_eq!(m.queue_pcm_mono8(&[5]), 0, "full queue accepts nothing");
        m.clear_pcm_stream();
        assert_eq!(m.pcm_stream_len(), 0);
        // cleared bus works again, in order
        assert_eq!(m.queue_pcm_mono8(&[200]), 1);
        let mut out = [0i16; 2];
        m.render(&mut out).unwrap();
        assert_eq!(out, [18432, 18432]);
    }

    #[test]
    fn stream_bus_mixes_under_voices_and_clamps() {
        let mut m = Mixer::new();
        m.load_wave(0, &[128, 255]).unwrap();
        m.note_on(0, 0x10000, 48); // unity gain at master 127
        m.queue_pcm_mono8(&[255]); // both sources at +32512
        let mut out = [0i16; 2];
        m.render(&mut out).unwrap();
        assert_eq!(out, [i16::MAX, i16::MAX], "sum saturates");
    }

    #[test]
    fn pool_starts_idle_and_silent() {
        let m = Mixer::new();
        assert_eq!(m.voices.len(), MAX_VOICES);
        assert!(m.voices.iter().all(|v| !v.active));
        let mut m2 = Mixer::new();
        let mut out = [7i16; 8];
        m2.render(&mut out).unwrap();
        assert!(out.iter().all(|&s| s == 0), "silence is exact zeros");
    }
}
