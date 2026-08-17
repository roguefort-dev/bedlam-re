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
- `FUN_00402bac` = **MusicPump** (called from the 100 Hz tick; iterates song
  slot 3 ONLY - `for (song = 3; song < 4; ...)` - exactly one music song is ever
  sequenced; the "gated pump, chan 3" of RE-EXW-TICK is this song-3 loop).
  Per tick, per chunk (0x26-stride state): while delta word 0045b038 == 0,
  dispatch the pending event (0045b03a): note-on/off via FUN_00402e46 (params
  from state fields), rest (bit7, <0xfe) = skip, 0xfe/0xff = PATTERN RESTART
  `FUN_004032a5(song, chanbyte)` (NOT a chunk jump: 004032a5 re-inits ALL chunks
  for channel chanbyte from the header tables; 0xfe only when loop flag
  word@0045cdc0[song] == 1, else falls through; 0xff always; both set
  music-active flag DAT_0046ae78 = 1). Then FUN_00402e74 = MrsNextEvent reads the
  next event; after the while, delta decrements once per tick. FUN_00402db9 =
  VoiceAlloc: variant-1 chunks round-robin 4 sub-voice slots (0x45b020..2c),
  variant-0 always slot 0. A word@004543d4+song*2 != 0xffff pending-restart flag
  also triggers 004032a5 + play flag. [EXW + DATA, high confidence - full event
  grammar VERIFIED against all 5 shipped files, see section 2b]
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

Header table roles RESOLVED (VERIFIED via load_midi pointer math + the
MrsChunkStart/MrsNextEvent consumers; the earlier A/B/C labels were only
guessed):

    +0x04+2*W0   W0 x u16  per-chunk VARIANT word (copied to 0045cce8+song*0x28+i*2):
                        0 = variant-0 chunks (event byte = instrument id),
                        else variant-1 (instrument = value+7, byte = note selector)
    +0x04+4*W0   W0*W1 x u16  START-OFFSET table (ptr 0045cd88): byte offset of
                        the chunk first event; 0xffff = chunk DISABLED
    +0x04+4*W0+2*W0*W1  W0*W1 x u16  INITIAL-TICK-DELAY table (ptr 0045cd98):
                        overrides the first event delta (chunk 1 = song length!)
    +0x04+4*W0+4*W0*W1  W0*W1 x u16  table C (ptr 0045cda8): set by load_midi,
                        read by NOTHING in the decompiled chain [open]

## 2b. .MRS event-stream grammar [EXW decompile + DATA, VERIFIED byte-exact, all 5 files]

Per chunk, a flat event stream (chunk data = contiguous arena blocks, no bounds
checks in the original). One event = u16 LE delta (tick countdown; 1 tick =
10 ms at the 100 Hz pump), then an event byte b:

| b | meaning | extra bytes |
|---|---|---|
| 0x00..0x7E | NOTE-ON. variant 0: instrument = b, param = 0x10000 (ratio 1.0). variant 1: instrument = variant+7, resample ratio = dword table @00454174[b] (16.16 fixed point; 1.0 at b=0x54, +18 semitone ceiling at 0x66 = 0x2d410, clamped above, 0 below b=0x18), note tag (0045b044) = b-0x54 | +1 byte: volume (observed 9..42); 0xFF = NOTE-OFF: releases the sub-voice whose note tag matches |
| 0x7F | SONG END: discards 1 byte, copies every chunk (pos,ptr) to shadow song slot song+4, resets shadow state, play flag 0045b010[song]=0, end flag 0045b018[song]=1. UNUSED by shipped data | +1 discarded |
| 0x80..0xFD | REST / idle gate (state keeps bit7 so the pump skips). UNUSED by shipped data (no occurrence in any stream) | +1 consumed |
| 0xFE | pattern restart on channel c, ONLY in loop mode (word@0045cdc0[song]==1; MusicStart zeroes it) else fall-through rest. UNUSED by shipped data | +1: channel c |
| 0xFF | unconditional pattern restart on channel c (MrsChunkStart(song,c) re-inits all chunks from the header tables; sets music-active 0046ae78=1) | +1: channel c |

Special MrsNextEvent cases: delta word SIGNED > 30000 (30001..32767) = backward
stream reposition pos -= delta*4 - 0x1d4be, then re-read delta (loop-back
encoding; UNUSED by shipped data). Signed < 0 (0xFFxx words) = freeze (pump
never decrements; the chunk stalls forever = natural stop). A note with
instrument 0x0E while word@0045b03c in {1,2} = alternate stop path (dead code
for the song-3 pump, which always has 0045b03c = song&3 = 3).

Structure VERIFIED against all 5 shipped files (script logic reimplements
MrsChunkStart/MrsNextEvent exactly; see commit):

- chunk 0 = DISABLED in every file (start-offset 0xffff): the 2-byte setup
  chunk is dead padding. chunk 1 (6 bytes) = the LOOP/TIMING chunk: one event
  (delta, 0xFF, channel 0) whose initial-tick-delay == the stream first delta ==
  the SONG LENGTH in 10 ms ticks: BRIEF 331 (3.31 s), SHOP 400, SELECT 1476,
  DEBRIEF 1600, OPTIONS 3388 - EXACT equality in all 5.
- every melody chunk stream is consumed to EXACTLY its last byte by whole
  events (28/28 streams, zero trailing bytes), and each chunk delta sum <=
  the song length (e.g. OPTIONS c2 = 3388 = song length): the 0xFF restart
  fires before any stream can run past its end. Songs loop forever via chunk 1.
- instruments: variant-1 chunks use instrument variant+7 (8..13 observed), all
  < the matching .MRW n_inst (11 or 14); variant-0 chunks use raw bytes 0..6.
- variant-1 note bytes observed in 0x4F..0x5E (i.e. -5..+10 table steps);
  bytes < 0x54 give ratio 0 (silent/rest notes: SELECT c2 = 4, OPTIONS c2 = 36,
  OPTIONS c6 = 2 events).
- NOTE-OFF (volume byte 0xFF) only ever appears in variant-1 chunks (e.g.
  OPTIONS c2: 180 of 360 events) - variant-0 notes play to natural sample end.
- No 0x7F / 0x80..0xFD / 0xFE events and no >30000 wrap deltas exist anywhere
  in the shipped corpus (all interpreter-supported but data-unused paths).

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

- .MRS event opcodes: ANSWERED 2026-08-17 (section 2b; byte-validated all 5
  files). Remaining small tails: (a) FUN_0044c4a8 (called by FUN_0044c3a4 when a
  free sub-voice is found) - presumed SetFrequency/ratio+volume applier, not yet
  decompiled; (b) header table C (0045cda8) written but never read by the
  decompiled chain - dead data or consumed elsewhere; (c) writer of the loop
  flag 0045cdc0[song]=1 (MusicStart zeroes it; without a writer all music plays
  as 0xFF-restart loops anyway) and of the pending-restart word 004543d4;
  (d) W1>1 multi-channel layout untested (all shipped files W1=1).
- Table C + W1 semantics (all shipped files W1=1, so multi-channel paths
  are untested by data).
- RNG 004ede4c: ANSWERED - RandB@004029b6 consumes it (see section 1).
