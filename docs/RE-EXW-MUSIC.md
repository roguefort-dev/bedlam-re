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
- `FUN_00403642(base, song)` = **load_midi** [VERIFIED by structure]: stops current
  music (`FUN_004034ef(3)`, frees voices `FUN_0043a48d`, `FUN_004035f5`), then
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

## 3. CONFIG.BDL (root, 61 B) [DATA + EXW negative evidence, high confidence]

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

## 4. .MRK (adjacent finding)

FUN_0040cca0 opens "<something>.MRK" (literal @00457a34, name built from
DAT_004dca0c) and reads 12 records of 3 dwords into 004e6430/34/38, then builds
the 0xa8-stride per-robot state at 0x4c69e4 (weapon table 0x4de664 stride 0x62,
0x4deafc stride 0x1c). Robot/mission marker data, not music. [EXW, medium -
record semantics not decoded]

## 5. Open (next unit)

- .MRS event-word opcodes: decompile the consumers - FUN_0044c2cc (load_midi
  tail: MRW load + sequencer start), the 100Hz-driven sequencer pump
  (FUN_00402bac "gated pump, chan 3" per RE-EXW-TICK open list, 20x38B records),
  FUN_004034ef/004035f5/004033d4 (stop/reset/start).
- .MRW internal layout: BRIEF.MRW header u16 9 (matches BRIEF W0=3? no),
  then dwords whose sum exceeds file size - entries are not plain sizes;
  likely per-instrument {size, rate} or dedup/shared waveforms. Needs the
  load_mrw consumer.
- Table A/B/C + W1 semantics (all shipped files W1=1, so multi-channel paths
  are untested by data).
