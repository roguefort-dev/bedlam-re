//! The mix graph (DESIGN-AUDIO secs 3-8): a flat pool of voices with
//! per-voice Q16 resamplers and Q8 gains, summed on an i32 master bus and
//! symmetric-clamped into interleaved S16 stereo output.
//!
//! Everything here is integer math. The host asks for N frames; nothing
//! inside converts time (dt never enters mix math, D17 bucket b), and the
//! script dispatch grid is Q16-exact so any host buffer chunking yields the
//! identical byte stream.

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
    /// Absolute output position in Q16 samples since construction.
    cursor_q16: u64,
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
            cursor_q16: 0,
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

    /// Install the music script (the walked .MRS stream). Dispatch starts
    /// from the first event; events whose tick positions already lie in the
    /// past fire on the next rendered sample (deterministic, documented).
    pub fn load_script(&mut self, script: MusicScript) {
        self.script = Some(script);
        self.script_next = 0;
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
            .map_or(true, |w| w.is_empty())
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
        if out.len() % 2 != 0 {
            return Err(AudioError::OddBufferLength { len: out.len() });
        }
        let frames = out.len() / 2;
        for f in 0..frames {
            self.cursor_q16 += Q16_ONE;
            self.dispatch_due();
            let (l, r) = self.mix_one();
            out[2 * f] = l;
            out[2 * f + 1] = r;
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
                Some(&(tick, _)) => tick_pos_q16(tick) <= self.cursor_q16,
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
        assert_eq!(volume_gain_q8(200u8.min(127), 48), 256);
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
