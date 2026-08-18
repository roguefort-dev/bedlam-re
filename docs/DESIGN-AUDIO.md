# DESIGN-AUDIO - bedlam-audio crate design note (P3; elaborates D16/D17, adds D25)

Status: DESIGN PINNED by this note; the crate skeleton implementing secs 3-10
lands in the same unit. Mirrors the DESIGN-RENDER flow (short note first, code
second). The DEVICE half of audio is explicitly P4 (sec 4).

## 1. The contract

`Mixer` is a pure hermetic integer signal processor: event script + wave data +
gain knobs go IN, an interleaved S16 stereo PCM stream comes OUT. It holds no
clock, performs no I/O, spawns no threads, contains no floats and no unsafe
code, and is callable from any host frame rate and from tests unchanged. The
platform device (cpal or similar) is out of scope until P4 (open question Q1).

## 2. RE basis (what the original does, with anchors)

| # | Fact | Anchor | Tag |
|---|------|--------|-----|
| 1 | Native rate 11025 Hz in BOTH builds: EXW SubVoiceStart@0044c4a8 SetFrequency = (ratio * 0x2b11) >> 16; DSBUFFERDESC WAVEFORMATEX = PCM 11025 Hz 8-bit mono; RAW SFX = 11025 Hz 8-bit mono; B2 = IRQ0-shared 11025 Hz PCM driver (PIT reprogram on arm, driver struct 0x1276dc) | RE-EXW-MUSIC sec 1/3/6, census sec 7 | verified |
| 2 | Sub-voice structure: mrw_load@0044c30d primes 4 sub-voices per instrument (FUN_0044c828); SubVoiceFind@0044c3a4 probes slots 0..3 first-free and DROPS the note when all busy; a volume-0xFF note-off releases the BASE buffer only - slots 1..3 ring out un-stopped | RE-EXW-MUSIC sec 6 | verified |
| 3 | Volume: SetVolume@+0x3c = ((master * volume) / 0x30 - 0x7f) * 0x7d0 >> 7 (signed hundredths of dB, DirectSound attenuation domain); master g_music_master_vol@004ee9b4 scale 0..127, written only by FUN_0044c630 (UI 0..100 -> >>1 -> 0..50); note volume bytes 9..42 observed | RE-EXW-MUSIC sec 6 | verified |
| 4 | Pan: SetPan@+0x40 (arg 0 = center) - the shipped EXW never pans off-center | RE-EXW-MUSIC sec 6 | verified |
| 5 | Pitch: ratio = RATIO_TABLE[byte] @00454174, 16.16 fixed (1.0 @ 0x54, +18 st ceiling 0x2d410, 0 floor below 0x18) | RE-EXW-MUSIC sec 2b/3b | verified |
| 6 | Music events: .MRS streams are u16 delta + event bytes at 10 ms ticks; MusicPump@00402bac (song slot 3) dispatches MrsTriggerNote(inst, ratio, volume, tag) when deltas expire, quantized to the 100 Hz service tick | RE-EXW-MUSIC sec 2b/6 | verified |
| 7 | B2 mixer: 20-channel PCM voice walker PcmMixerService@0x136e0 (spawn/free sub-voices) gated by triple flag 0x11ef50/0x11ef74/0x11f0e0 | census sec 7 | verified |
| 8 | Timing fabric: fixed-rate service clock + present-paced frames (D16/D22/D23); audio triggering rides the service clock, the device consumes at its own rate | DECISIONS D16/D22/D23 | verified |

## 3. Mix-graph topology

    waves (per instrument, 8-bit unsigned 11025 Hz mono, MRW records)
      -> VOICES: flat pool of 20 (MAX_VOICES, B2 fact 7), each tagged
         (instrument, sub 0..3, EXW fact 2), one-shot
      -> per-voice resampler: 16.16 phase accumulator, step = ratio (fact 5)
      -> per-voice gains: Q8 volume (sec 6) x Q8 pan L/R (fact 4)
      -> MASTER BUS: i32 stereo accumulator, one master volume knob (fact 3)
      -> symmetric clamp to i16 (sec 8) = the crate boundary
      -> DEVICE half (P4): cpal-or-similar stream + rate conversion.

Everything above the clamp is this crate and pure; everything below it is
platform-side and deferred.

## 4. Determinism boundary (the D17 audio split)

- MIX GRAPH (this crate): hermetic integer math ONLY. Byte-deterministic: the
  same script + waves + knob sequence produces a byte-identical output stream
  under ANY host buffer chunking, because the event grid is internal Q16 math
  (sec 5), not host-frame-quantized. dt NEVER enters: the host asks for N
  frames; nothing inside converts time to samples.
- DEVICE half: the real-time stream, hardware rate conversion and buffering -
  deferred to P4 (open question Q1). Like the render golden path, goldens and
  parity checks never touch it.

## 5. Sample-rate policy

Mix natively at 11025 Hz (fact 1: same rate in EXW and B2). The mix graph
never sees any other rate; conversion to the device rate is DEVICE-half work.
The .MRS 10 ms pump tick = 441/4 samples exactly = 0x6C0000 Q16 (11025/100),
so the tick grid is representable without error; script events fire when the
mix cursor reaches their exact Q16 position - the same 100 Hz quantization the
EXW MusicPump applies (fact 6), never coarser.

## 6. Pitch / volume / pan semantics

- PITCH: voice phase is 16.16; advance per output sample = the RATIO_TABLE
  value verbatim (unity 0x10000 = one source sample per output sample, which
  is the same fact as SetFrequency (0x10000 * 11025) >> 16 = 11025). ratio
  == 0 (table floor below 0x18) mutes the voice (open question Q3).
- VOLUME (D25): the EXW domain is the integer product (master * vol) / 48
  (fact 3), delivered to DirectSound as hundredths of dB. The mix graph
  linearizes that product to a Q8 gain = min(256, (master * vol * 256) /
  (127 * 48)) - monotone in the same product, never above unity (DS domain
  is attenuation-only). The dB curve itself stays an API-level documented
  fact; PLAN P4 pins audio parity to a correlation band on the downsampled
  mix, which does not require reproducing the dB curve.
- PAN: linear balance, Q8 left = (64 - pan) * 4, right = (64 + pan) * 4,
  pan an i8 clamped to -63..=63; the original always passes 0 (fact 4).
- NOTE-OFF QUIRK (kept, faithful): note_off(instrument) stops the sub == 0
  voice of that instrument only; subs 1..3 ring out until their wave ends
  (fact 2). Sub index = spawn order among currently-active voices of the
  instrument; note_on drops the note when the instrument already has 4
  active sub-voices or the pool is exhausted.

## 7. MRS-event-driven triggering

`bedlam-assets` music.rs `Mrs::walk` already yields the full decoded event
stream (pure, budget-guarded). The mapping to this crate is mechanical and
lands in bedlam-game (the MusicPump analog):

    MrsEvent::Note { volume != 0xFF }  ->  (tick, NoteOn  { instrument, ratio, volume })
    MrsEvent::Note { volume == 0xFF }  ->  (tick, NoteOff { instrument })
    MrsEvent::Rest                     ->  advance only
    MrsEvent::SongEnd / Restart        ->  stop / re-init the walk (host side)

bedlam-audio deliberately takes NO dependency on bedlam-assets: data crosses
as plain structs (the same coupling rule that keeps bedlam-platform off
bedlam-core, DESIGN-RENDER sec 5). The script stores absolute tick positions
(the walk already resolved deltas); SFX bypasses the script and calls note_on
directly (EXW: FUN_0044c8c4 -> SubVoiceStart) - host-event-timed by nature,
so SFX timing is per-frame bucket, not golden material.

## 8. Saturation and clipping policy

Symmetric clamp to [-32768, 32767] at the master bus, per sample per channel,
after the i32 accumulate. The DirectSound / B2 driver-side saturation shape is
unknown (open question Q4); a symmetric clamp is the standard PCM behavior
and the P4 correlation band does not pin it. No soft knee, no dither.

## 9. Hash policy (pinned per D17 b)

Audio state is NEVER hashed. It lives entirely in the D17 bucket (b) -
per-frame, host-paced - and bedlam-core hashes exclude it by construction.
This crate exposes no hash function at all; the audio determinism gate is the
byte-identity of the mix stream (sec 4), which is strictly stronger than a
hash and checkable in ordinary CI.

## 10. Type sketch (API as implemented by the skeleton)

    pub const SAMPLE_RATE: u32 = 11025;     // fact 1, both builds
    pub const MAX_VOICES: usize = 20;       // B2 walker (fact 7)
    pub const SUB_VOICES_PER_INST: usize = 4; // EXW (fact 2)
    pub const TICKS_PER_SECOND: u32 = 100;  // .MRS 10 ms grid (fact 6)

    pub struct Mixer { /* voices, waves, master, script, cursor */ }
    impl Mixer {
        pub fn new() -> Mixer;
        pub fn load_wave(&mut self, instrument: u16, pcm_u8: &[u8]) -> Result<(), AudioError>;
        pub fn set_master_volume(&mut self, master: u8);          // 0..=127, fact 3
        pub fn load_script(&mut self, script: MusicScript) -> Result<(), AudioError>;
        pub fn note_on(&mut self, instrument: u16, ratio: u32, volume: u8) -> Option<VoiceRef>;
        pub fn note_off(&mut self, instrument: u16) -> bool;      // base-only quirk
        pub fn voice_playing(&self, instrument: u16, sub: usize) -> bool; // GetStatus analog
        pub fn render(&mut self, out: &mut [i16]) -> Result<Frames, AudioError>;
    }

    pub struct MusicScript { /* sorted (tick, MusicCommand) list */ }
    pub enum MusicCommand { NoteOn { instrument: u16, ratio: u32, volume: u8 },
                            NoteOff { instrument: u16 } }

    #[derive(Debug, Error)] pub enum AudioError { /* thiserror only */ }

Notes: `render` mixes floor(len/2) stereo frames and dispatches script events
whose Q16 positions fall inside the buffer at their exact sample offsets
(chunking-invariant); `VoiceRef` is (instrument, sub). Errors: thiserror only;
no new dependencies; `#![forbid(unsafe_code)]`.

## 11. Testing and goldens

- Unit: gain formula edges (master 0 = silent; unity clamp), pan law corners,
  tick-to-Q16 exactness (tick 4 == sample 441), ratio-0 mute policy.
- Integration `tests/determinism.rs`: same event script => byte-identical mix
  buffer; chunking invariance (1/7/64/512-frame chunks re-concatenate to the
  same bytes); note-off-releases-base-only; drop-when-full; one-shot voice
  recycling; saturation clamp; pan/volume audibility; odd-length rejection.
- Miri over the crate (charter hygiene; no unsafe anywhere).
- P4 goldens: correlation band on the downsampled mix, never exact bytes
  (PLAN P4) - original-side capture needs the harness.

## 12. Open questions (each names its answer source)

- Q1: device backend (cpal vs others) + hardware rate conversion + latency
  budget -> P4 dependency spike (PLAN sec 6 P4 item 1), same slot as wgpu.
- Q2: W1 > 1 multi-channel .MRS layout untested (all shipped files W1 = 1)
  -> RE-EXW-MUSIC open item; affects only the bedlam-game pump, not this
  crate.
- Q3: ratio == 0 notes (byte < 0x18 in a stream): does any shipped stream
  emit them, and what does SetFrequency(0) really do -> corpus scan + one
  Ghidra probe; until then the mixer mutes them (sec 6).
- Q4: driver-side saturation shape (DS mixer / B2 adder) -> P4 harness
  correlation; symmetric clamp until evidence.

## Provenance

Written 2026-08-18 by the item-1 worker (claim lock-v1 1787020711) from
RE-EXW-MUSIC sec 1/2b/3/6, RESEARCH-BEDLAM2-CENSUS sec 7, DECISIONS
D16/D17/D22/D23 and PLAN sec 6 P3/P4. RE facts above carry their anchors;
everything marked [design] or D25 is a reimplementation choice, not an RE
claim. Confidence: high on facts 1-8 (all verified in prior runs), high on
the API shape (mirrors DESIGN-RENDER acceptance flow).
