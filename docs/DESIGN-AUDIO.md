# DESIGN-AUDIO - bedlam-audio crate design note (P3; elaborates D17b)

Status: DESIGN 2026-08-18, implementation lands as the crate skeleton in the
same unit. Every RE claim below carries an address anchor + confidence tag
per PLAN sec 9; design choices are tagged [design]. Where this note proposes
something the RE has not settled, it says so in sec 9 instead of guessing.

## 1. The contract

bedlam-audio is a PURE hermetic mix graph + music sequencer. Topology:

    MRS events (bedlam-assets music.rs walk) -> Sequencer (100 Hz pump analog)
        -> voices (fixed table: instruments x 4 sub-voices)
            -> master bus (integer sum + saturating clamp + master gain folded per voice)
                -> S16 stereo interleaved output slice (the device boundary)

- Canonical sample rate: 11025 Hz (0x2b11), EXW- and B2-confirmed (sec 2).
- Output = i16 stereo interleaved; the caller hands the slice in and owns it.
- The DEVICE half (cpal-or-similar platform sink, resampling to host rate,
  callback thread) is bedlam-platform P4 territory and OUT of this crate
  (open question 1). Nothing in bedlam-audio performs I/O, owns a clock,
  spawns threads, reads floats, or uses unsafe.
- The crate is driven per frame by the host (D17b bucket): the host decides
  HOW MANY samples to request per call (from host dt, OUTSIDE the crate);
  dt itself never enters any mix or sequencer math. Inside, everything is
  integer sample counting on an exact Q16 grid (sec 6).

## 2. RE basis (anchors)

| # | Fact | Anchor | Tag |
|---|------|--------|-----|
| 1 | Native rate 11025 Hz = 0x2b11: every EXW voice is created via FUN_0044c64c WAVEFORMATEX PCM 11025 8-bit mono; SFX loader FUN_0043a39c passes 0x2b11; .MRW waveforms + .RAW SFX are 11025 Hz 8-bit unsigned mono | RE-EXW-MUSIC sec 1/3 | verified |
| 2 | Pitch = SetFrequency((ratio * 0x2b11) >> 16) on 16.16 ratio dwords; ratio table @00454174 (1.0 at byte 0x54, ceiling 0x2d410 = +18 st) | SubVoiceStart@0044c4a8, RE-EXW-MUSIC sec 6 | verified |
| 3 | Volume = SetVolume(((master * vol) / 48 - 127) * 2000 >> 7) hundredths of dB; master = g_music_master_vol@004ee9b4 (setter FUN_0044c630; UI paths pass vol>>1 of a 0..100 setting, i.e. master 0..50; scale documented 0..127); SubVoiceStart guards master != 0 (no start when muted) | RE-EXW-MUSIC sec 6 + RE-EXW-INPUT | verified |
| 4 | Pan = SetPan(0) = center, ALWAYS, in the entire music path; SFX path passes 0 too via SubVoiceFind | SubVoiceStart@0044c4a8 | verified |
| 5 | Sub-voices: 4 slots per instrument; variant-1 chunks round-robin 4 slots, variant-0 always slot 0; SubVoiceFind probes for the FIRST FREE slot and starts it; MusicPump runs at the 100 Hz service tick, 10 ms event deltas (MRS grammar) | VoiceAlloc/MusicPump@00402bac, RE-EXW-MUSIC sec 1/2b | verified |
| 6 | NOTE-OFF releases the BASE buffer (slot 0 of the instrument), whichever sub-slot is sounding; slots 1..3 ring out to natural sample end | RE-EXW-MUSIC sec 6 quirk | verified |
| 7 | B2 (DOS): IRQ0-shared 11025 Hz PCM driver, 20-channel voice walker PcmMixerService@0x136e0, driver struct 0x1276dc - same native rate, hardware-mixer shape | RESEARCH-BEDLAM2-CENSUS sec 7, D23 | verified |
| 8 | DirectSound does the mixing + saturation in hardware for EXW; no software clamp exists in the game code | RE-EXW-MUSIC | verified (absence) |

## 3. Type sketch (API) [design]

    // bedlam-audio
    pub const MIX_RATE: u32 = 11025;          // sec 2 fact 1
    pub const SUB_VOICES: usize = 4;          // sec 2 fact 5
    pub const MAX_INSTRUMENTS: usize = 32;    // shipped banks max n_inst = 14; headroom
    pub const VOICE_COUNT: usize = MAX_INSTRUMENTS * SUB_VOICES;

    pub struct Mixer { /* voices, bank, master vol - all integer */ }
    impl Mixer {
        pub fn load_bank(&mut self, waves: &[&[u8]]) -> Result<(), AudioError>;
        pub fn set_master(&mut self, master: u16);            // 0..=127, 0 = mute-guard
        pub fn master(&self) -> u16;
        pub fn note_on(&mut self, inst: u16, ratio: u32, vol: u8, pan: i16, tag: i16) -> bool;
        pub fn note_off(&mut self, inst: u16);                // releases slot 0 (fact 6)
        pub fn mix_frame(&mut self) -> (i16, i16);            // one stereo frame
        pub fn active_voices(&self) -> usize;                 // test/telemetry
    }

    pub struct Sequencer { /* per-chunk cursors + deltas from Mrs walk */ }
    impl Sequencer {
        pub fn new(mrs: &Mrs) -> Sequencer;                   // walks every chunk once
        pub fn pump(&mut self, mixer: &mut Mixer);            // ONE 10 ms tick
    }

    pub struct Audio { sequencer, mixer, Q16 phase grid }
    impl Audio {
        pub fn new(mixer: Mixer) -> Audio;
        pub fn with_song(mixer: Mixer, mrs: &Mrs) -> Audio;
        pub fn mixer(&mut self) -> &mut Mixer;
        pub fn render(&mut self, out: &mut [i16]);            // stereo interleaved
    }

## 4. Gain, pitch, pan, saturation policy

- Gain: the EXW dB ladder is realized as a 128-entry Q8 integer table
  GAIN_Q8[k] = round(256 * 10^(-15k/2000)), k = 0..=127, with k =
  clamp(127 - (master * vol) / 48, 0, 127) computed at note start (the
  SetVolume analog; master is folded per voice exactly like fact 3 - a
  master change affects NEW notes only, matching the once-at-start
  SetVolume call). k > 127 cannot occur after clamping; k clamped UP from
  negative values reproduces the DS behavior where a positive SetVolume
  argument fails and the buffer stays at full volume. [design: table-as-data
  follows the RATIO_TABLE precedent; runtime math stays pure integer]
- Pitch: per-voice 16.16 phase accumulator steps by the RAW ratio per output
  sample (source advances ratio/65536 samples per output sample). This is
  the SetFrequency formula of fact 2 without the integer truncation of
  (ratio * 11025) >> 16, which is a DirectSound API artifact (sub-Hz
  difference). [design, T2]
- Interpolation: linear, integer (s16 = base + ((next - base) * frac) >> 16,
  final sample held); voice frees when the position reaches the wave end
  (natural ring-out, fact 6). [design; nearest would also be legal]
- Pan: linear balance law, pan argument clamped to -100..=100 percent,
  0 = center = the only value the original ever uses (fact 4). Equal-power /
  DS-exact balance curve deferred (open question 3). [design]
- Saturation: master bus accumulates i32 per channel, then saturates to
  i16. No limiter, no soft clip - the hardware-mixer analog of fact 8.
  [design]

## 5. Determinism boundary and hash policy (D17b, pinned)

- Audio state is NEVER hashed into bedlam-core state hashes. Audio is a
  per-frame (D17b) system: the host calls render() as often as it presents,
  and the sample count per call is host-derived. This note pins that policy.
- Determinism of audio is BY CONSTRUCTION instead: integer-only math, fixed
  voice table order, event dispatch on an exact sample grid (sec 6). The
  test contract: the same Mrs + the same render call sequence yields
  byte-identical output; stronger, output is invariant to render CHUNKING
  (render(1024) == 4 x render(256) == 1024 x render(1)).
- forbid(unsafe_code); no floats anywhere in mix state; no allocation on
  the per-frame path (allocation happens at bank load / song load only);
  no unordered iteration influencing output.

## 6. Timing integration: the exact Q16 event grid [design]

The original streams DS buffers continuously and fires MRS events on the
100 Hz service tick; events land on 10 ms walls and DS starts voices at the
next hardware block. A pull-model mixer needs one precise rule; ours:

- Internal sample clock in Q16 units (1 sample = 65536). Each sequencer
  tick advances the next-tick boundary by SAMPLES_PER_TICK_Q16 =
  11025 * 65536 / 100 = 7225344 EXACTLY (11025 = 441 * 25, so the quarter
  sample per tick is exactly representable; four ticks = 441 samples
  exactly).
- render() fires every tick whose boundary is <= the current sample
  position BEFORE mixing that frame (a while loop, so a large first chunk
  fires all due ticks), then mixes one frame, then advances the position by
  65536. Events therefore take effect at floor(boundary sample) - sample
  accurate, chunking invariant, and reproducible on every OS and toolchain.
- Sequencer tick = the MusicPump shape of fact 5: per chunk, while delta ==
  0 dispatch the pending event and load the next delta, then one decrement
  per tick; initial delta = the table-B tick delay (chunk 1 = song length);
  freeze-terminal chunks halt; a Restart event re-inits every enabled chunk
  from the header tables (the song loop); SongEnd halts all chunks.
- This grid is OUR rule, not an original mechanism (DS had none at this
  granularity); it is pinned here so goldens and the audio correlation band
  (PLAN P4) have a fixed target.

## 7. Skeleton scope

Implemented now: Mixer (bank load, master, note_on/note_off, S16 stereo
mix, gains/pitch/pan/saturation per sec 4), Sequencer (full MRS event
dispatch over bedlam-assets Mrs walk), Audio facade with the sec 6 grid,
AudioError (thiserror). Not implemented: SFX one-shot convenience wrappers
(note_on already covers them), per-song voice tables (one Mixer serves one
song at a time, like MusicPump song slot 3), the shadow song slot + loop
flag + pending-restart dead mechanisms of RE-EXW-MUSIC sec 6c (dead in the
shipped binary - not modeled), any device/backend code (P4).

## 8. Sample-rate policy

Native 11025 Hz is the ONLY rate inside the crate; host-rate conversion is
a device-side (P4) concern. The 8street 44.1 kHz mixer is a known deviant
(PLAN sec 0 canon table) and is NOT followed. B2 needs no separate rate
(fact 7). Resampling policy at the device edge (integer-ratio first,
windowed-sinc later if measurable) is open question 2.

## 9. Open questions (each names its answer source)

1. Device half choice (cpal vs alternatives) + callback/thread model - the
   P4 dependency spike (PLAN sec 6 P4 item 1), with D19-style pinning.
2. Device-edge resampling policy (11025 -> host rate) - P4, measured against
   the audio correlation band of the golden pipeline.
3. Pan law fidelity (DS balance curve vs linear) - irrelevant until any
   caller passes nonzero pan (none known in RE; revisit if SFX panning is
   found in the P2f audio pass leftovers).
4. ratio == 0 notes (variant-1 bytes below 0x54): the original calls
   SetFrequency(0) (invalid for DS; behavior at runtime unknown). We treat
   them as rests (no voice start). A DOSBox/Wine runtime probe can settle
   it; only affects silent notes.
5. Sub-voice steal policy when all 4 slots sound: RE shows probe-first-free
   and implies drop-when-none-free; verify against runtime behavior if the
   music ever audibly clips notes (T2 judgment).
6. Whether SFX needs its own voice budget separate from the music bank
   (EXW creates separate DS buffers) - decided when the SFX loader is
   reimplemented in P3 bedlam-game / P4.

## Provenance

RE facts restate results recorded in docs/RE-EXW-MUSIC.md (secs 1, 2b, 3,
6) and docs/RESEARCH-BEDLAM2-CENSUS.md sec 7, produced by the 2026-08-17/18
RE runs; no new RE was performed for this note. Design sections are
proposals for the implementing unit to follow or amend with a DECISIONS
entry if deviating.
