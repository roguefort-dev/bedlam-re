# RE-EXW-MUSIC - .MRS loader chain + CONFIG.BDL (BEDLAM.EXW)

Provenance: [EXW] = decompiled from BEDLAM.EXW (Ghidra BedlamWatcom project,
dump ghidra-project/exw-music.txt, script tools/ghidra-scripts/ExwMusicLoader.java).
[DATA] = verified byte-exact against shipped files in game-data/BEDLAM/SOUND/MIDI/.
Confidence tags inline. 2026-08-17.

## 1. Loader call chain [EXW, high confidence]

Five screens load a song by basename (strings "SOUND\MIDI\<NAME>"):

| screen fn | basename |
|---|---|
| FUN_0043a5fc | OPTIONS (main menu) |
| FUN_0043d00b | BRIEF   (also the 50Hz-gate reader, see RE-EXW-GAMETHREAD) |
| FUN_0043e7d4 | SELECT  |
| FUN_00440e45 | SHOP    (also zone/level manager) |
| FUN_0044425c | DEBRIEF |

Chain (SFX path shown for completeness):
- `FUN_0043a39c(name, size)` = SFX loader: `LoadFile(name, arenaTop)` then
  `FUN_0044c64c(buf, loadedSize, 0x2b11, 8, 1)` creates a DirectSound voice
  (0x2b11 = 11025 Hz little-endian rate word, 8-bit, mono) and primes 4 sub-voices
  via FUN_0044c828. RAW SFX = 11025 Hz 8-bit mono.
- `FUN_0044c2cc(base, song)` = **mrw_load** (load_midi tail, decompiled in
  exw-music-followup.txt): if DirectSound active (`_DAT_004ee9b0 != -1`) calls
  `FUN_004038c6(base,...)` = load_mrw, then for each instrument i < MRW word0:
  assigns DS voice slot `*(u16*)(i*2 + song*0x40 + 0x4ef4e0) = _DAT_004ef4d8`
  (global next-voice counter) and `FUN_0044c64c(mrw+2+rec.off, rec.size, 0x2b11, 8, 1)`
  creates the voice. `FUN_0044c64c` = DirectSound CreateSoundBuffer wrapper:
  DSBUFFERDESC dwSize 0x14 / flags 0xe2, WAVEFORMATEX PCM 11025 Hz 8-bit mono,
  then memcpy (FUN_0044f326) + SetCurrentPosition-family vtable call. Voices are
  released via FUN_0044c480 (Release).
- `FUN_004033d4(song)` = music start/reset: zeroes chunk position counters
  (0045ca60), resets per-chunk state via FUN_00402e74, sets play flag 0045b010[song]=1.
- `FUN_00402bac` = **sequencer pump** (called from the 100 Hz tick, channel 3 =
  music only; the "gated pump, chan 3, 20x38B records" of RE-EXW-TICK): per chunk,
  per 0x26-stride state - bit 0x80 of the state word = idle gate; state word 0xff =
  unconditional chunk jump `FUN_004032a5(song, target)`; 0xfe = conditional jump
  (only if song flag 0045cdbe == 1, i.e. loop mode); FUN_00402e74 = advance/read
  next event into state; FUN_00402e46 = trigger note (4 params from state fields);
  FUN_00402db9 = allocate one of the 4 sub-voices. FUN_004032a5 = chunk event
  interpreter entry (= 8street sub_4032A5). DAT_0046ae78 = music-active flag.
  [EXW, high confidence on structure; opcode semantics partial]
- `FUN_00402975` = 16-bit-pair RNG over 004ede48/004ede4a (carry-mixed adds
  0x62e9 / 0x3619, compare 0x9d16) - the consumer of seed dword 004ede48
  (=123456, cf. RE-EXW-GAMETHREAD); .MRK loader uses it for per-robot spawn
  direction (`rand() & 3`). Now named **RandA** in the Ghidra project;
  concurrent run also found twin **RandB@004029b6** consuming 004ede4c/004ede4e
  (= 234567 halves) - CLOSES the second-seed question. 00402965 renamed
  MemZero (rep-stos zero-fill), fixing the earlier Rand16 misname. [names:
  script commit ee6089e, verified vs decompils]
- `FUN_00403642(base, song)` = **load_midi** [VERIFIED by structure]: stops current
  music (`FUN_004034ef(song)` = per-chunk release of 4 sub-voices each;
  `FUN_0043a48d` free voices; `FUN_004035f5` = voice-table wipe - confirms table
  shape 8 songs x 20 chunks x 4 sub-voices, strides 0x2f8 per song / 0x26 per
  chunk, voice ids at 0045b020, init -1), then
  `FUN_00403827(base, &DAT_0045cdd0)` = **load_mrs**: builds "<base>.MRS" (literal
  ".MRS" @00457a1c), arena-allocs (`FUN_0041db89` bump allocator) and calls
  `FUN_0041cc7f(name, dest)` = **LoadFile** - the game-universal file loader (also
  used for all GAMEGFX\*.BIN/*.PAL). `FUN_004038c6` = **load_mrw** ("<base>.MRW",
  literal ".MRW" @00457a21), same shape; its caller is downstream of load_midi
  (tail call `FUN_0044c2cc(base, song)`, not yet decompiled - next unit).

Globals set by load_midi (matches 8street names, now EXW-anchored):
- DAT_0045cdd0 = raw .MRS file base (arena ptr)
- 0045cce0+2*song = midi_arr[song]    = file word0 = W0 (chunk count)
- 0045cdb8+2*song = midi_arr_pl1[song]= file word1 = W1 (channel count)
- 0045cd88/98/a8 +4*song = per-song pointers to three W0*W1 u16 tables (A/B/C)
- 0045c7e0 + song*0x50 + i*4 = runtime per-chunk data pointers (accumulated from
  the size array); 0045ca60 same stride = per-chunk position counters (reset 0)
- 0045cce8 + song*0x28 + i*2 = per-chunk init state (copied from 2nd word array)

## 2. .MRS container layout [DATA, VERIFIED byte-exact, all 5 files]

    +0x00  u16 W0   chunk count
    +0x02  u16 W1   channel count (all 5 shipped files: W1 = 1)
    +0x04  W0 x u16 chunk data sizes (bytes)
    +0x04+2*W0  W0 x u16 per-chunk init state words
    +0x04+4*W0  table A  (W0*W1 u16)
    ... +2*W0*W1 bytes: table B (W0*W1 u16)
    ... +2*W0*W1 bytes: table C (W0*W1 u16)
    data_off = 4 + 4*W0 + 6*W1*W0 ; chunk event streams follow (W0 blocks,
    back-to-back, sizes from the size array; chunk i ptr = file + data_off + sum(sizes[0..i-1]))

Validation: file_size == data_off + sum(sizes) holds EXACTLY for all five files:

| file | W0 | W1 | data_off | sum(sizes) | file size |
|---|---|---|---|---|---|
| OPTIONS.MRS | 10 | 1 | 104 | 2944 | 3048 |
| BRIEF.MRS    |  3 | 1 |  34 |  134 |  168 |
| SELECT.MRS   |  9 | 1 |  94 |  838 |  932 |
| SHOP.MRS     |  3 | 1 |  34 |  178 |  212 |
| DEBRIEF.MRS  |  8 | 1 |  84 |  684 |  768 |

(e.g. OPTIONS sizes = 2,6,1442,34,98,194,98,194,866,10; every file starts 2,6 -
chunks 0/1 look like fixed setup chunks.) [DATA]

Confidence: header + size-array layout VERIFIED; table A/B/C semantics INFERRED
(consumer = sequencer pump, below) - event-word opcodes still open.

## 3. .MRW container layout [DATA, VERIFIED byte-exact, all 5 files]

    +0x00 u16 n_inst (instrument count)
    +0x02 n_inst x { u32 rel_offset (relative to file start + 2), u32 size }
    +0x02+8*n_inst: waveform data, 11025 Hz 8-bit mono
    (records may share offsets - waveforms are deduplicated)

Validation: max(off+size) == file_size exactly, all records in range:

| file | n_inst | distinct waves | file size |
|---|---|---|---|
| BRIEF    | 9  | 2 | 5032 |
| OPTIONS         | 14 | 9 | 153103 |
| SELECT          | 11 | 8 | 199159 |
| SHOP            | 9  | 3 | 13422 |
| DEBRIEF         | 11 | 6 | 162667 |

(BRIEF chain check: rec0 {0x4a, 0x84d}, rec1 {0x897, 0xb11}, 0x897 = 0x4a+0x84d,
max end 0x897+0xb11 = 5032 = size.) Voice slots land in the 0x40-stride-per-song
table at 0x4ef4e0; global voice counter 004ef4d8.

## 4. CONFIG.BDL (root, 61 B) [DATA + EXW negative evidence, high confidence]

Layout (no EXW code reads it - see below, so decoded from bytes only):

    +0x00 u8  0 (flag)
    +0x01 char[] "SOUNDBLASTER COMPATIBLE" (23 chars, NUL) zero-padded to +0x2C
    +0x2D.. tail = 01 00 00 00 05 00 00 00 00 e0 00 00 20 02 00 00
            (u16 LE walk: 1, 0, 5, 0, 0xe000, 0, 0x0220, 0)

0x0220 = SoundBlaster base port, 5 = IRQ 5: classic SB setup record.

EXW never opens it: memory string census (exw-music.txt STRINGS) finds no
"CONFIG" string except "CONFIG.SYS file, or" @0045860f (error message text, no
code refs as a filename); the ".BDL" hit @004597d6 is the TAIL of the literal
"SAVED.BDL" @004597d1 (overlapping suffix, not an independent literal); the only
.BDL-family literals in EXW are SAVED.BDL + HISCORES (SAVES\ persistence,
FUN_00446938 / FUN_00446ebc / FUN_00447550). Conclusion: root CONFIG.BDL is an
installer / sound-setup artifact (likely written by the DOS setup program),
NOT game state. RESEARCH-8STREET open question 7 answered (also explains why the
8street reconstruction never reads it).

## 5. .MRK (adjacent finding)

FUN_0040cca0 opens "<something>.MRK" (literal @00457a34, name built from
DAT_004dca0c) and reads 12 records of 3 dwords into 004e6430/34/38, then builds
the 0xa8-stride per-robot state at 0x4c69e4 (weapon table 0x4de664 stride 0x62,
0x4deafc stride 0x1c). Robot/mission marker data, not music. [EXW, medium -
record semantics not decoded]

## 6. Open (next unit)

- .MRS event-word opcodes: IN FLIGHT from the concurrent run (dumps
  ghidra-project/exw-music-events*.txt; names MrsChunkStart=004032a5,
  MrsNextEvent=00402e74, MrsTriggerNote=00402e46, VoiceAlloc=00402db9,
  DSCreateVoice=0044c64c, DSPrimeSubVoice=0044c828, DSReleaseVoice=0044c480,
  VoiceTableWipe=004035f5, VoicesFree=0043a48d already applied to the Ghidra
  project, script commit dce31e9). Decoded so far [EXW, medium confidence,
  from the MrsNextEvent decompile]: per event = u16 delta (loop-wrap when the
  running state word > 30000: pos -= n*4 - 0x1d4be, re-read) then event byte:
  0x80 = idle gate; 0x7f = SONG END (all chunk positions reset to chunk
  starts, play flag 0045b010=0, end flag 0045b018=1); otherwise NOTE-ON with
  per-chunk encoding variant selected by the MRS header init-state word
  (0045cce6): variant 0 = byte is the instrument id (state 0045b03a);
  variant 1 = note = byte-0x54, instrument = init+7, param dword from table
  00454174[byte]; then ONE more byte -> 0045b042 (volume?). REMAINING:
  MrsNextEvent tail (special case 0xe sub 1/2 + _DAT_004edb4c),
  MrsTriggerNote 4-param decode, byte-validate against BRIEF.MRS.
  Original plan text kept: consumers - FUN_0044c2cc (load_midi
  tail: MRW load + sequencer start), the 100Hz-driven sequencer pump
  (FUN_00402bac "gated pump, chan 3" per RE-EXW-TICK open list, 20x38B records),
  FUN_004034ef/004035f5/004033d4 (stop/reset/start).
- Table A/B/C + W1 semantics (all shipped files W1=1, so multi-channel paths
  are untested by data).
- RNG 004ede4c: ANSWERED - RandB@004029b6 consumes it (see section 1).
