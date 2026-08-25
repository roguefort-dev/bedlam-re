# Bedlam (1996) — Mission File Format Dossier

**Scope:** `game-data/BEDLAM/EDITOR/ZONE{A..G}/MISSIONn.*` — 37 missions × 17 extensions,
plus the zone-level `MISSION{A..G}.*` files where relevant.
**Method:** statistics over all files (sizes, divisibility, header probes), cross-file
arithmetic checks, hex inspection of representative files, spatial-coherence tests for
map layouts. All scratch scripts were run from `/tmp`; `game-data` was only read.
**Byte order:** little-endian everywhere (dimensions, counts and offsets all decode
sanely as LE; no big-endian field was ever needed).
**Confidence tags:** VERIFIED (multiple files agree + arithmetic checks out) /
LIKELY (strong pattern, one interpretive leap) / HYPOTHESIS (needs executable RE).

---

## 0. Corpus inventory (VERIFIED)

| Zone | Missions | Map dims (from MAP header) | Zone-level extras |
|------|----------|---------------------------|-------------------|
| A | MISSION1 | 25 × 75 | MISSIONA.{BIN,BLD,CGR,CTG,LNG,LNK,MIN} |
| B | MISSION1–7 | 100 × 100 | MISSIONB.{BIN,BLD,CGR,CTG,LNG,LNK,MIN,PAL} + MISSION6.BIN |
| C | MISSION1–7 | 100 × 100 | MISSIONC.{BIN,BLD,CGR,CTG,LNG,LNK,MIN,PAL} |
| D | MISSION1–7 | 100 × 100 | MISSIOND.{BIN,CGR,CTG,LNG,LNK,MIN} + MISSION5.BIN — no zone-level BLD/PAL, but mission-level MISSION1-7.BLD DO ship (37 mission BLDs total incl. zone D; §17) |
| E | MISSION1–7 | 100 × 100 | MISSIONE.{BIN,BLD,CGR,CTG,LNG,LNK,MIN} + MISSION6.BIN |
| F | MISSION1–7 | 100 × 100 | MISSIONF.{BIN,BLD,CGR,CTG,LNG,LNK,MIN} (no PAL) |
| G | MISSION1 | 100 × 25 | MISSIONG.{BIN,BLD,CGR,CTG,LNG,LNK,MIN,PAL} |

37 missions total (1+7·5+1). Total tile counts: ZONEA 1 875, ZONEG 2 500, others
10 000 each; global total 354 375 tiles.

Recurring engine constants seen across formats: **128** (CGR sprite count),
**8192** (LNK/CTG/LNG table length), **2000** (POS slots), **999** (PAD slots),
**12** (MRK slots).

### 0.1 The retired ".MOFO" tag (2026-08-22, VERIFIED absent)

A suspected fifth mission extension `.MOFO` (from a DGROUP string at
0x457a4c adjacent to the loader tags) **does not exist**:
- `0x457a4c` = `"MOFO\0"`, the dead tail of the fatal-message string
  `"Buggered direction in MOFO"` @0x457a3c — zero code references;
  the message's sole consumer is FUN_00415490, the mode-9 critter
  seek-acquisition dispatcher (RE-EXW-SIM §7j.29).
- The loader-tag family in that block is exactly `.NME` @0x457a57 /
  `.TRT` @0x457a5c / `.POS` @0x457a64 / `.BDG` @0x457a69 — one
  reference each, all four loaders CLOSED (RE §7j.18/§7j.15/§7j.25).
- The byte sequence `.MOFO` appears in neither BEDLAM.EXW nor
  BEDLAM.EXD, and no `*.MOFO` file exists anywhere in the corpus.
The mission extension set therefore stands at the 17 shipped
extensions of §0 with no unresolved member at that dispatcher.

### 0.2 Runtime-loaded vs editor-only extensions (2026-08-22, VERIFIED — EXW §7j.33)

Full dot-extension string census of EXW DGROUP + the two loader
sites: the RUNTIME loads exactly `.TOT .DAT .CGR .BIN .MIN
.LNG/.LNK (language gate [0x4eba1c]==1 → .LNG) .PAD` (family
loader FUN_0041dc5a, tag table @0x4587d9..0x4587fc) plus
`.NME .TRT .POS .BDG .MRK` (FUN_0041a4f8 etc.) from
`EDITOR\ZONE{A..G}\MISSION{n}` paths (builder FUN_0044670c).
Six shipped EDITOR-tree extensions have ZERO references in any
executable (case-insensitive byte census over EXW/EXD/EXE/
DIRECTX ×3): **.BLD, .CTG, .COL, .MAP, .PTH, .TXT** — they are
editor-only data (the game's .LNK/.LNG gate means .CTG is never
read at runtime; .BLD's content reaches the game only through
its compiled sibling .BDG, §16/§17). `"SAVED.BDL"` @0x4597d6 is
the SAVEGAME file, unrelated to the ZONE* .BLD libraries.

---

## 1. MAP — tile map, 8 planes of u16

- **Sizes:** 30004 (ZONEA), 40004 (ZONEG), 160004 (×35). VERIFIED
- **Formula:** `u16 width + u16 height + width*height*16` — exact for all 37 files
  (e.g. ZONEA/MISSION1.MAP @0x0000: `19 00 4B 00` = 25, 75; 4+25·75·16 = 30004).
- **Layout:** the 16 bytes per tile are **8 plane-major layers**, each `w*h` u16
  (plane p occupies payload bytes `[p·2·w·h, (p+1)·2·w·h)`). VERIFIED
  - Evidence vs. the alternative (16-byte per-tile records): horizontal-adjacency
    coherence is 0.93–0.97 on planes 2–5 (ZONEA/M1) vs ~0.81 for every
    tile-major attribute slot (0.81 is the "mostly-zero" baseline); planes 6–7
    of ZONEA/M1 are entirely zero, which is trivially plane-shaped, not tile-shaped.
  - Rendering plane 0 of ZONEA/M1 as ASCII produces recognisable rooms, walls and
    corridors (runs of identical values tens of tiles long).
- **Value ranges (global, all 37 missions):** per-plane maxima ≈ 1465–1865;
  nonzero cells per plane fall off monotonically: 166 657 / 108 508 / 54 562 /
  34 889 / 16 386 / 12 005 / 4 771 / 1 155 (of 354 375 total). Plane 0 is the
  densest "terrain/tile-ID"-like layer; higher planes are sparse overlays.
- **What RE must confirm:** the semantic of each plane (height? collision?
  occupancy? damage?). Candidates: plane 0 = base terrain/tile index; planes
  that are near-empty in MAP but fuller in TOT = dynamic links (see §2).

## 2. TOT — same layout as MAP, a superset of it

- **Sizes:** identical distribution to MAP (30004/40004/160004). VERIFIED
- **Layout:** same `u16 w + u16 h + 8 × w·h u16 planes`. VERIFIED
- **EXW loader ANCHORED (2026-08-21, RE-EXW-SIM §7j.16):** FUN_0041dc5a
  (MissionShell mission-load call @0x447b3a) loads ".TOT" into the heap
  volume behind [0x4ede20] (word/voxel, arena 0x27104 → max 100×100×8),
  reads the `u16 w, u16 h` header into the map dims ([0x4eddec]/[0x4eddf0],
  plane pitch w·h → [0x4eddf4]) and skips 4 bytes. FUN_00440a2d
  (sole caller FUN_00440dc2 = the BRIEF objective-minimap
  snapshotter, EXW §7j.49 — BRIEF screen only, never the in-game
  mission render) is the incremental runtime consumer: it walks 7×7 tiles × 8
  levels and copies every nonzero TOT word whose DAT byte is 0 into the
  TOT MIRROR (0x4796bc word@[row·0x1E]+2z) + seen flag 0x4796cc — i.e.
  **the .TOT file volume is the persistent word state; the mirror is the
  per-frame runtime view** (where the TRT structure animation frames
  1..0x1E live). FUN_0044661b re-loads .TOT/.BIN/.DAT on the
  save/EDITOR\ZONE restore path.
- **Load-time mirror staging CORRECTED (EXW §7h.4, 2026-08-22):**
  the FULL load-time build is init_tiles@00407e11 (0x407fb0..0x407ff8),
  and it copies **every nonzero TOT plane word** into the mirror —
  the DAT byte gates ONLY the seen flag (`DAT[z]==0 → seen=1`). The
  DAT==0 word gate belongs to FUN_00440a2d's incremental restamp path
  alone. Consequence: words at DAT≠0 cells (the pickup substrate —
  DAT type 3 + word in the terrain-set pickup ranges, §7h.4) DO
  mirror-stage at load with seen=0.
- **Mirror-record grammar + pre-stamped footprints (EXW §7j.32,
  2026-08-22; tail semantics completed §7j.34):** the TOT MIRROR
  is one **0x1E-B record per tile** @0x4796bc+0x1E·tile: `+2·z`
  the 8 plane words, `+0x10+z` the 8 SEEN bytes, `+0x18` scorch,
  `+0x19` the door/scenery TARGET-TAG byte and `+0x1A` {bit7 door
  phase, low7 frame counter} — the sliding-door animation machine
  (FUN_00423081 writes DAT door-frame bytes 0x40..0x5E and shifts
  the z-stack on completion), `+0x1B/+0x1C` the OBJECT-HEIGHT
  pair (z0, z0+D) stamped/cleared by the objective-building
  family, `+0x1D` unused (zero traffic confirmed). And
  the shipped .TOT/.DAT are **pre-stamped with the destructible
  buildings**: every .POS footprint cell carries its BDG
  CURRENT-state bank word/byte in the shipped files (434/435
  ZONEA/M1 cells; the one miss = a footprint overlap,
  last-.POS-slot-wins) — buildings never get stamped at runtime;
  destroy re-instates the BDG UNDER-terrain pair (§16) into the
  runtime mirror/seen/DAT only.
- **Relationship to MAP — VERIFIED, with a caveat:**
  - MAP and TOT are **never** byte-identical (0/37).
  - Across all 37 missions and all 8 planes there are exactly **0** cells where
    MAP is nonzero and TOT is zero — TOT's support is a superset of MAP's.
  - 85 758 cells are nonzero in TOT but zero in MAP (additions).
  - 4 292 cells are nonzero in *both* but differ (value rewrites) — so TOT is
    **not** a trivial overlay; e.g. ZONEA/M1 plane 0 tile 409: MAP=347, TOT=789.
  - Plane-equality pattern varies per mission (plane 0 equal in 10/37, plane 7
    equal in 9/37, etc.).
- **Planes 6/7 — CLOSED (RE-EXW-SIM §7j.47/D119, 2026-08-23):** almost empty
  in MAP (4 771 + 1 155 nonzero cells globally), fuller in TOT (8 016 +
  2 882; 36/37 missions carry them — only ZONEG/M1 is zero; 6 504 cells are
  overlays on planes 0..5, 2 792 standalone). **Semantics: planes 6/7 are
  ordinary z-levels 6/7 of the same word stack** — the tops of tall
  structures, carrying per-level sprite ids (ZONEA/M1's single cell, tile
  642 = (17,25), is one tower column: TOT words [454,1354,1355,1356] at
  z=4..7 — the "1355/1356 adjacent integers" are just the z-6/z-7 sprite
  ids). The value domain is IDENTICAL to planes 1..5 (35..1868 vs 33..1868
  global nonzero). **The "~2000-entry target-table" hypothesis is REFUTED**:
  the "≤1868, just under the 2000-slot POS count" nearness is a property of
  the tile-word grammar (planes 1..5 reach 1868 too); resolving every
  plane-6/7 value as a POS slot gives 9 217 live / 1 681 empty records in
  their own missions (coincidence, not linkage — and ZONEA's 1355/1356 hit
  EMPTY slots); and the renderer draws plane-6/7 words exactly like planes
  0..5 (no z≥6 gate anywhere: the FUN_00403938 restamp z-stack loop runs
  z 0..7 with a word-only restart gate; init_tiles stages all 8 planes).
- **What RE must confirm:** what "TOT" stands for and when the engine reads it
  (working copy? editor "totals"? merged runtime map?). The plane-6/7 target
  table item is CLOSED (§7j.47).

## 3. COL — per-tile attribute codes, 8 planes of u16

- **Sizes:** identical distribution to MAP. VERIFIED formula
  `u16 w + u16 h + w·h·16`.
- **Values:** every plane's values are ≤ **102** (global max per plane = 102).
  Dominant values: 1 (empty/default) and **37**; also 29–32, 65, 97, 99, 101
  in some zones. ZONEA/M1 @0x0004 starts `25 00 25 00 …` = (37, 37, …).
- **Not a palette:** despite the name, the VGA palette is PAL (§14); COL is a
  per-tile code grid. LIKELY: per-tile colour/zone/attribute class codes used
  with PAL/CGR for rendering, or collision classes.
- **What RE must confirm:** meaning of the codes (37 recurs in CTG and DAT too —
  plausibly one shared "solid/wall" class id, but that is HYPOTHESIS).

## 4. DAT — 8 planes of u8 over the same grid

- **Sizes:** 15004 (ZONEA), 20004 (ZONEG), 80004 (×35). VERIFIED
- **Formula:** `u16 w + u16 h + w·h·8` — exactly half of MAP per tile, i.e.
  **8 plane-major u8 layers** (`w*h` bytes each). VERIFIED
  (e.g. ZONEA/M1.DAT @0x0004 reads as u8 planes; u16-view gives the tell-tale
  doubled bytes 0x0101, 0x2525 — byte planes, not word planes).
- **Values:** each plane ≤ 98; plane 0 is almost all 1 with 37-anomalies
  (ZONEA/M1: 1×1500, 37×355, 98×11); deeper planes mostly 0/1/2 with max ~10.
- **Semantics — CONFIRMED by EXW RE (2026-08-21, docs/RE-EXW-SIM.md §7c):**
  the DAT byte at `[z][y][x]` is the **tile TYPE consumed by walkability**
  (get_from_dat_file@0041eb28 via the y-line/z-base tables the loader
  builds): type 0/0x2A = empty (get_z_pos searches z, z+1, z−2), 0xFF reads
  back as type 1, type 3 latches the trigger triple, everything else indexes
  the CGR height sprites (slot = type−1). The loader sweeps planes 0..6
  clearing bytes ≥0x80, then overwrites `DAT[kind][y][x] = 0xFF` for every
  PAD record — the PAD "effect" is stored IN the DAT type grid.
- **Loader side ANCHORED (2026-08-21, RE-EXW-SIM §7j.16):** FUN_0041dc5a
  loads ".DAT" into the volume behind [0x4edd58] (byte/voxel, skips the
  same `u16 w, u16 h` 4-byte header; arena 0x13884), runs the ≥0x80→0
  sweep, then the ".PAD" parse stamps `0xFF`; the ".TRT" loader
  FUN_004170a6 additionally stamps tile **0x66** (the terrain-structure/
  turret tile, sibling of the 0x62 trap tile) at each turret's (x,y,z).
- **Type-3 cells = the pickup/trigger substrate (EXW §7h.4,
  2026-08-22):** a DAT==3 cell whose sibling TOT plane word lies in the
  terrain-set pickup ranges ([A,A+0x10) ∪ [B,B+0xC), tables
  0x454a58/0x454a74, set [0x4edd8c] = zone_index+1) is a PICKUP — any
  move_is_possible probe touching it consumes it (DAT byte := 0, TOT
  mirror word := floor word 0x454a90+4·set, seen := 1) and dispatches
  FUN_0040eba0. Corpus census (read-only): ZONEA/M1 has 80 type-3
  cells but ZERO in set-1 pickup range; ZONEB (set 2) 601 pickup
  cells, ZONEF (set 6) 149, zones C/D/E/G none — the type-3 bytes
  with out-of-range words (ZONEA's 0x81..0x84/0x230-family) are
  other trigger scenery, inert to the pickup walk.

## 5. LNK — u16[8192] rotation/permutation link table

- **Size:** exactly 16384 = 8192 × u16, all 44 files. VERIFIED
- **Content:** VERIFIED — the table is `identity[i] = i` except for a small
  number of cells forming **cyclic permutations**. ZONEA/MISSION1.LNK begins
  `00 00 01 00 02 00 03 00 …` (0,1,2,3…) and deviates from identity in only
  138 cells, in cycles such as 33→34→35→33, 36→37→38→36, 55→56→…→64→55
  (length 10), 88→89→90→91→88.
  - 27 cycles in ZONEA/M1; other variants have 78–286 deviating cells.
- **Interpretation (LIKELY):** `LNK[i]` = "next orientation/variant of object i"
  — i.e. rotate-object links over the same ~8192-object index space as CTG/LNG.
  Cycle lengths 3–10 fit 4/8-direction rotations plus multi-part sequences.
- **First verified runtime consumer (2026-08-25, EXW §7j.62/D149):** the
  ZONE-level LNK image (loaded to 0x45cdda behind the language gate) is the
  **type→mask-index lookup** of the map-overlay territory stamps:
  cw = LNK_word[TOT word], mask entry = the .MIN bank's 16 B at cw·16 —
  the permutation cycles rotate/variant-link adjacent 4×4 mask entries.
- **Only 7 distinct contents among 44 files** (per-zone variants; ZONEB/M6
  shares ZONEA/M1's exact table).
- **What RE must confirm:** what the index space enumerates (sprites? BLD
  records?) and what "following the link" does.

## 6. CTG — u16[8192] sparse category table (parallel to LNK)

- **Size:** exactly 16384, all 44 files. VERIFIED
- **Content:** sparse; nonzero values are small — dominantly **1** and **37**
  (ZONEA/M1: 523 nonzero u16; ranges 39–42, 55–77, 79–82, 103–104, 117–132,
  141–150, 157–162). 8 distinct contents of 44 files.
- **Relationship to LNK (LIKELY):** LNK's permutation cycles live inside CTG's
  nonzero ranges (ZONEA: cycles 33–38 ⊂ CTG range 32–42; cycle 55–64 ⊂ 55–77)
  — the two tables are parallel arrays over one object index space. Not a perfect
  overlap (e.g. LNK cycle 88–95 lies outside CTG range 79–82), so CTG ≠ "is
  rotatable" exactly. HYPOTHESIS: CTG[i] = category/class of object i
  (37 = one shared class, 1 = another).
- **What RE must confirm:** the class vocabulary and consumers of the table.

## 7. LNG (zone-level only) — third 8192-entry permutation

7 files, exactly 16384 B each. ZONEA/MISSIONA.LNG is another near-identity
u16[8192] (8170 distinct values, 0…8191, starts 0,1,2,…). Same index space as
LNK/CTG. LIKELY another link table ("language"? "lineage"? "long-link"?).
What RE must confirm: everything beyond the layout.

## 8. MRK — 12 marker records on the tile grid

- **Size:** exactly 192 = 12 × 16 bytes, all 37 files. VERIFIED
- **Record (4 × u32), VERIFIED:** `(flag, x, y, type)`
  - ZONEA/MISSION1.MRK @0x0000: `01 00 00 00 15 00 00 00 49 00 00 00 01 00 00 00`
    = (1, 21, 73, 1).
  - Across all 444 records (37 × 12): `x ≤ width` in **444/444**, `y ≤ height`
    in **444/444** — coordinates are tile-grid positions. VERIFIED
  - flag ∈ {0 (×55), 1 (×389)} — likely "active/visible".
  - type ∈ 0…7 (1×204, 2×75, 3×51, 4×39, 5×10, 6×12, 7×1, 0×52).
  - All 37 files have distinct contents (each mission places its own markers).
- **Interpretation (CONFIRMED for robot spawns, EXW RE 2026-08-21 —
  docs/RE-EXW-SIM.md §7c.7):** load_markers@0040cca0 spawns robot i from
  record i verbatim: `pos = (x<<13)+0xF00, (y<<13)+0xF00`,
  `z = word3·0x20 − 1` — so **word 3 is the spawn Z LEVEL (1 = ground),
  not a type**, and the flag word is DROPPED (all 12 records are staged;
  only the first `robots_per_player` become robots: zone<3||7→1, 3→2,
  else 3). Remaining open: what consumes word-3=0 records (z −1 seeds)
  and the flag word elsewhere.

## 9. NME — critter + personnel placement script (FULLY DECODED)

- **Sizes:** vary 16–1492 B (28 distinct). 10 files are **16 bytes of zeros**
  (ZONE{B,C,D,E,F}/MISSION{6,7} — no enemies). VERIFIED
- **No strings anywhere** — all u16 values; records contain in-bounds tile
  coordinates. VERIFIED
- **Grammar (LOADER-ANCHORED, VERIFIED 2026-08-21 — EXW RE
  docs/RE-EXW-SIM.md §7j.18):** the file is **8 sequential sections in a
  FIXED order**, each `u16 count + count × rec` where the record width is
  fixed per section position — exactly the read schedule of the mission-load
  dispatcher `FUN_00416458` after it stages ".NME" (@0x457a57) into the
  0x4dca0c reader:

  | # | width | feeds | critter state (7j.17 controller) | spawn multiplier | record fields (u16) |
  |---|-------|-------|----------------------------------|------------------|---------------------|
  | 1 | 10 B | critter bank 0x4cff98 | 2 (sine-walk shooter, 0x65) | `w1 + difficulty` | w0=1 marker, w1 = spawn base (≤8, typ. 4), w2 = mirror flag (negates the variant param), w3 = x tile, w4 = y tile |
  | 2 | 10 B | 〃 | 1 (wander) | `difficulty + 3` | w3 = x tile, w4 = y tile (DAT tile search z=6→down for a 1..3 floor with empty above) |
  | 3 | 8 B | 〃 | 5 (mixed-AI) | `difficulty` (min 1, d=1 → rand 1..2) | w1 = probe level (0..7), w2 = x, w3 = y |
  | 4 | 8 B | 〃 | 4 (mixed-AI, seek steppers) | `(difficulty>>1) + 2` | w1 = probe level, w2 = x, w3 = y |
  | 5 | 10 B | 〃 | 3 (chase, 0x67; stores home x/y) | 1 | w1 = timer `<<6`, w2 = probe level, w3 = x, w4 = y |
  | 6 | 8 B | 〃 | 6 (mixed-AI, ballistic) | 1 | w1 = probe level, w2 = x, w3 = y |
  | 7 | 6 B | 〃 | 7 (close combat, 0x69) | `difficulty` (min 1) | w1 = x, w2 = y; z fixed 0xDF |
  | 8 | 8 B | POI/personnel bank 0x4dabdc | — (spawns in state 5 = ESCAPE) | **4 POIs per record** (jitter ±31 sub-tiles) | w1 = probe level, w2 = x, w3 = y |

  The old "header (n1,n2)" was just the first two section counts (the 9 files
  with n1=0 have an empty section 1); the old "(count,type)" pairs were a
  section count followed by the first word of its first record.
- **Exact-consumption check (VERIFIED):** the 8-section schedule consumes
  every one of the 37 shipped files **exactly** — 36/37 byte-exact to EOF;
  the one exception is ZONEA/MISSION1.NME which leaves a 16-B orphan tail
  (words `1,0,18,0,66,0,1,0`) the game loader never reads (editor dregs;
  the dispatcher stops after section 8 and calls only a reader-close +
  an empty stub FUN_004180b9).
- **Field stats (VERIFIED, all 37 files):** w0 is always the marker 1 in
  every non-empty section; section-1 w1 ∈ 0..8; 8-B w1 (probe level) ∈ 0..7;
  all x/y words ≤ 99 and in-bounds for their zone (matches §2 MAP dims).
- **Runtime meaning (EXW RE §7j.18):** section 1 places each critter at
  Q13 `(w3 + scatter(5) − 2)·0x2000` (jitter), z fixed 0xC000, variant
  param `scatter(4)+3` negated when w2 ≠ 0; sections 3/5/6 at
  `tile·0x2000 + 0xF00`, section 4 at `tile·0x20 + 0xF`, section 2 at
  `tile·0x20 + 0x10` with z from a DAT tile search (z=6 downward, floor
  value 1..3 with an empty cell above), z elsewhere from the floor probe
  FUN_0041e411 seeded by w1; hp always
  `base + (base·difficulty)/27` with base 0xAF/0xC8/0x96/0x5DC/0x9C4 by
  section (175/200/150/1500/2500). Section 8 seeds each POI
  {+0 active=1, +2 0x32, +4 5 (ESCAPE), +6 1, +8 heading RandA&7} —
  personnel spawn already fleeing toward the 5 exit slots.
- **What RE must confirm:** nothing structural. Optional: the exact
  distribution of FUN_0041ec1c scatter returns (jitter ranges), and the
  semantic of the marker word w0 (always 1; loader never reads it).

## 10. PAD — 999 pad (elevator/teleporter) record slots

- **Size:** exactly 5994 = 6 × 999 for all 37 files. VERIFIED
- **Layout (VERIFIED):** 999 × 6-byte slots (3 × u16). Record = `(x, y, type)`.
  The loader stops at the first record whose `x == 0xFFFF`; bytes after that
  terminator are not semantically consumed and need not be uniform fill.
  Shipped `ZONEB/MISSION3.PAD` contains an ignored orphan record after its
  terminator.
  - ZONEA/MISSION1.PAD: 114 records; first bytes
    `05 00 3D 00 00 00 05 00 35 00 01 00 …` = (5,61,0), (5,53,1), (10,46,1)…
  - Record counts across missions: 2 … 114. type tally: 0×310, 1×173, 2×51,
    3×50, 4×62, 5×47, 6×8 (7 pad types).
- **Meaning (CONFIRMED storage, EXW RE 2026-08-21 — docs/RE-EXW-SIM.md
  §7c.5):** after loading, the engine writes `DAT[plane=type][y·w+x] = 0xFF`
  for every loaded record (shipped type values are 0..6).
  get_from_dat_file reads 0xFF back as tile type 1 — a CGR slot-0
  0x1F-height deck block at level `kind`. So **`type` is the z LEVEL the
  pad materialises its tile at**, matching the TXT "lowers section two
  levels" phrasing (a level change re-marks the DAT cell). Loader side
  ANCHORED (RE-EXW-SIM §7j.16): FUN_0041dc5a parses the ".PAD" section
  into the 0x4e44f8 runtime slots — 8 B each (word@+0 = active, written
  1 after load; x@+2, y@+4, z@+6 = the 3×u16 file record), x==0xFFFF
  terminates, then `DAT[z][y][x] = 0xFF`. [Re-verified 2026-08-25,
  S0-07: the whole 999×8 bank is memset-0 BEFORE parsing
  (FUN_00402965 @0x41de62); the terminator's own x IS staged (slot t =
  {0, 0xFFFF, 0, 0} — y/z never read, active never set); all slots
  after t stay all-zero and their file bytes are never read; EXD twin
  0x2e7a0..0x2e85d is the identical algorithm.] The 0x4e44f8 slots are drawn as
  scanner icon 0xC (FUN_0041ee20). Open: the
  interactive side (when a pad fires) lives in TOT/NME consumers.
- **Honest negative (kept):** the TXT coordinates (e.g. `0/006/005`) still
  do not match PAD records under simple transforms; the TXT `L/x/y` frame
  uses a section-local coordinate space we haven't mapped.

## 11. PAL — one shared 256-colour 6-bit VGA palette

- **Size:** exactly 770 = 2 + 256×3 for all 40 files. VERIFIED
- **Layout (LIKELY):** 2 bytes (u16 @0 = 0 — purpose unknown; padding/version),
  then 256 × RGB triples with components 0–63 (6-bit VGA). First entries:
  (0,0,0), (62,58,57), (63,54,54), (0,63,63)…
- **All 40 files are byte-identical** — a single global palette. VERIFIED
- **What RE must confirm:** whether the engine ever applies gamma/first-2-bytes
  meaning.

## 12. POS — 2000 scenery/object placement slots × 16 B

- **Size:** exactly 32000 = 2000 × 16 for all 37 files. VERIFIED
- **Layout (VERIFIED):** 2000 records of 4 × u32 `(x, y, kind, index)`;
  unused slots are **entirely 0xFFFFFFFF** (all four words).
  - ZONEA/MISSION1.POS @0x0000: `(11,11,1,25) (13,11,1,25) (21,18,1,18)
    (3,69,1,0) (6,67,1,0) …`
  - Used slots per mission: 48 (ZONEF/M7) … 1954 (ZONEE/M7); ZONEA/M1: 213.
  - `x ≤ w`, `y ≤ h` for every non-sentinel record checked (in-bounds test
    over all missions flags only 0xFFFFFFFF slots).
  - **word 2 = the BASE Z LEVEL 0..5, not a "kind" (EXW §7j.32, 2026-08-22):
    the destroy restore runs z ∈ [word2, word2+D) and the BDG CURRENT-state
    banks match the shipped TOT/DAT at planes word2+z' for 434/435 ZONEA/M1
    cells — word 2 is consumed as the footprint's z origin (values 0..5 fit
    the 8-plane stack with max D 3).**
  - index ∈ 0…273 with 0xFFFFFFFF also occurring in field 3.
- **Cross-file (LIKELY):** the `index` field never exceeds the mission's BLD
  record count (e.g. ZONEA: max 196 vs ≈285 BLD records; ZONEB/M1: max 230 vs
  ≈344; ZONEF/M5: max 273 vs ≈473) — consistent with *index = BLD record
  (scenery object type)*. Not yet proven.
- **EXW ANCHOR (2026-08-21, RE-EXW-SIM §7j.25):** FUN_0041a4f8 (mission load
  @0x447b76) opens ".POS" (string 0x457a64) and reads EXACTLY 2000 × 0x10 B
  via FUN_0041cccb into the 0x46cbf4 object-instance array (stride 0x14 =
  x,y,kind?,index→id dword), then scans id ≠ −1 for the count 0x46cbe8 and
  re-stamps footprints via FUN_0041a7f0. So `.POS` = the DESTRUCTIBLE-OBJECT
  INSTANCE list (4 × u32 per record); the semantics of words 2/3 refine the
  kind/index gloss — word 3 lands in the record id dword consumed as the
  .BDG/type-table row index (the 7j.12/7j.13 family).
- **TOT planes 6/7** — CLOSED (§2 + RE-EXW-SIM §7j.47): ordinary z-levels
  6/7 of the word stack (tall-structure sprite tops); the POS-slot linkage
  is REFUTED.
- **What RE must confirm:** field semantics, the kind vocabulary, and whether
  index really references BLD records (or CGR sprites for some kinds).

## 13. PTH — path data, always empty in shipped missions

- **Size:** exactly 2 bytes for all 37 files; every file is `00 00`. VERIFIED
- **Interpretation (LIKELY):** `u16 count = 0` — waypoint/path records would
  follow the count; the shipped campaign simply has none (or paths are stored
  in TOT/NME). **What RE must confirm:** record layout for count > 0 (probably
  recoverable only via the editor binary).

## 14. TRT — count + 12-byte trigger/turret records

- **Sizes:** 2 … 1310 B, 19 distinct; **every size ≡ 2 (mod 12)** —
  `(size − 2) / 12` is an exact integer for all 37 files. VERIFIED
- **Layout (VERIFIED):** `u16 count + count × 12 B`, each record = 3 × u32
  `(x, y, z-level)` (third field re-anchored from "type" — see
  Interpretation).
  - ZONEA/MISSION1.TRT @0x0000: `03 00` then (14,15,1), (11,15,1), (10,33,1).
  - ZONEB/MISSION1.TRT: count=19; records (13,45,2), (15,45,2), (1,73,2), …
  - Across all missions: x ≤ 97, y ≤ 97, always within that mission's map
    bounds (no out-of-bounds record found); type ∈ 0…6 (1×265, 2×212, 3×64,
    4×24, 5×5, 6×6, 0×1).
  - 11 files have count = 0 (2-byte file).
- **Interpretation (ANCHORED 2026-08-21 to RE-EXW-SIM §7j.15,
  consumers CLOSED §7j.16):** the
  third u32 is the **z LEVEL** (values 0..6 = map levels; per-zone bands:
  ZONEA records all level 1, ZONEB all 2), not a type enum. The records are
  destructible **terrain-structure placements — SHOOTING SENTRY TURRETS**
  (consumer hop §7j.16): runtime record (active@0x4cccf8 frame, stride 0x20)
  = {active, state, anim_frame, fire_ctr, hp, x, y, z}; the MissionShell
  animator FUN_00417264 runs an 8-state machine (idle→alert→aim S/N/W/E by
  octant toward the nearest robot) that animates the TOT mirror word
  (frame+1, muzzle frames up to 0x1E) and, via FUN_00417698, fires
  **projectile type 0x66** (damage (d+1)·300) at robots within a 40px
  directional lane and ≤2 z-levels. Structures never move; the 250-rec
  capacity bank is staged by the ".TRT" loader FUN_004170a6 and damaged by
  the resolver FUN_0041bc1c (hp = 250+(250·linear mission)/27). The
  7j.15 "turrets? retired" note is itself retired — turrets is the right
  primary reading. The +0x08 scratch dword = the animation frame, runtime
  producer FUN_00417264 (loader zeroes it; no file producer exists).

## 15. TXT — ASCII designer/mission notes (two known documents)

- **Sizes:** 409 B ×33 (all byte-identical) and 1649 B ×4 (all byte-identical). VERIFIED
- CRLF line endings, plain ASCII. VERIFIED
- 409 B doc: *"Full Score Codes for all buildings"* — a score-value table
  (code 0 = 0 points … code 28 = 250 000). Implies buildings carry a 0–28+
  score code somewhere (BDG/BLD are the candidates).
- 1649 B doc (ZONEC/M7, ZONED/M6, ZONED/M7, ZONEF/M6): *"All the pads and the
  location of their effects for reference by me or paul"* — the PAD file's
  design note (see §10).
- **What RE must confirm:** nothing structural — but these notes are the best
  semantic Rosetta stones in the corpus.

## 16. BDG — the destructible-object spec library (loader-anchored 2026-08-21)

- **Sizes:** 26 distinct values, 17100–43644 (37 mission files). All consumed
  exactly by the grammar below.
- **GRAMMAR VERIFIED (EXW §7j.25; census 37/37 files, byte-exact
  consumption, exactly 282 records per file):** records start at offset 0 —
  **NO header** (the old "12-B header mirroring BLD rec0" read was a
  mis-frame; the `(1,1,1,1,150,0,1,15)` opening u16s ARE record 0). Record =
  - `u16 control` — ≠1: the record is just these 2 bytes (empty library row;
    the value is **0 on all 2527 corpus empty rows** — §7j.61/2);
  - ==1: `u16 W, u16 H, u16 D` (footprint; **113 distinct tuples, W ≤ 10,
    H ≤ 10, D ≤ 8, max (10,10,5) = 500 cells at ZONEF/M1 #184 — the
    pre-2026-08-25 "max (3,3,3)" claim was WRONG (§7j.61/D)**; (1,1,1)
    alone covers 3581 records), `i32 hp` (e.g. 150; domain −1..18900 —
    negative hp exists on disk), `u16 chain` (chain-detonation gate,
    EXW 7j.13; domain {0,1}), `i32 type` (objective/score code; corpus 15/5/30/11/120/90/
    20/40/10/60/180/270…; 0xb = score-10), `5 × 8 B` effect entries
    (`u16 selector, u16 x_off, u16 y_off, u16 z_off` — tile offsets staged
    relative to the instance; selector 1..9 → the destroy-tail debris/effect
    cases, EXW §7j.25; corpus uses ONLY 1..9: ×11098/1490/1385/402/330/304/
    316/178/56), then **four template banks of `2·W·H·D` bytes each**
    (u16 cells, linear `(z·H+i)·W+j` — H = y-extent, W = x-extent).
- **STAGING SEMANTICS (§7j.61, re-verified instruction-by-instruction
  2026-08-25):** the whole 282×0x4E = 0x55EC-B in-memory table at 0x4dedf2
  AND 0x9C40 B of the bank arena are memset-0 before every load; the
  loader stages the raw control word at row+0 BEFORE the ==1 test; empty
  rows leave +2..+0x4E memset-0 (head, effects, count, and all four bank
  pointer slots NULL); the count word@+0x12 = the number of NONZERO
  selectors, computed at load on ACTIVE rows only (census 0..5; 554
  active rows carry count 0 — not a presence flag) and has ZERO runtime
  readers (write-only); the four banks are read into CONSECUTIVE arena
  slots in DISK ORDER (cursor 0x46ad5c, += 2·W·H·D per bank) — the
  current/under interleave lives only in the row's pointer slots.
- **TEMPLATE-BANK SEMANTICS CLOSED (EXW §7j.32, 2026-08-22):** the four
  banks are a 2×2 of {CURRENT state, UNDER-terrain} × {TOT words, DAT
  volume}, and the **on-disk order is interleaved vs the in-memory slot
  order** (loader FUN_0041a4f8 @0x41a71d..0x41a782): disk bank **1 → slot
  +0x3E** (CURRENT TOT words), **2 → +0x46** (UNDER TOT words), **3 →
  +0x42** (CURRENT DAT words), **4 → +0x4A** (UNDER DAT words). Corpus
  proof (ZONEA/M1, 435 footprint cells): bank1 ≡ the shipped .TOT plane
  word and bank3 ≡ the shipped .DAT byte at every .POS footprint (434/435
  each — the one miss is a genuine footprint overlap, last-.POS-slot-wins);
  bank2/bank4 (the UNDER pair, what the destroy restore writes back into
  the runtime mirror/seen/DAT) differ from the shipped files almost
  everywhere. **+0x3E/+0x42 are loaded into the arena and read by NO code**
  (triple census: slot addresses, displacement forms, arena walk) — the
  editor's stamp payload, already baked into the shipped .TOT/.DAT; there
  is NO runtime spawn-stamp pass. Value domains: banks 1/2 = tile words
  ≤1868; bank 3 ≤ 102 (DAT domain); bank 4 ≤ 512 (word 0 → seen=1, low
  byte → DAT volume on restore).
- **Cross-file:** mission-level BDG size vs BLD size Pearson r = 0.985 —
  relationship CLOSED (§17/§7j.33): .BLD is the EDITOR-SOURCE format
  that compiles to this runtime spec (record j ≡ BDG non-empty record
  j — same head scalars, same four template banks, names dropped,
  banks compacted). .POS word 3 (index) selects the BDG row (§12).
- **What RE must confirm:** nothing structural; the BLD-side record
  walk is CLOSED (§17, §7j.33 — BLD is editor-only, zero runtime
  readers).

## 17. BLD — the EDITOR-SOURCE object library (grammar verified 2026-08-22; EDITOR-ONLY, zero runtime readers)

- **Runtime status (VERIFIED, EXW §7j.33):** the game NEVER
  opens .BLD — the byte sequence "BLD" occurs in NO shipped
  executable (EXW/EXD/EXE/DIRECTX ×3, case-insensitive). .BLD
  is the editor's SOURCE format; the runtime consumes only its
  compiled sibling .BDG (§16). Same for .CTG, .COL, .MAP,
  .PTH, .TXT (§0.2).
- **Sizes:** mission-level 29964–96430 B (37 files) + 6
  zone-level (MISSION{A,B,C,E,F,G}.BLD — zone D DOES ship
  mission-level BLDs; the §0 row above is corrected) — 43
  files. Zone-level sharing: MISSIONA.BLD ≡ MISSIONF.BLD,
  MISSIONB.BLD ≡ MISSIONG.BLD (byte-identical, md5).
- **Header (VERIFIED constancy, 43/43):** 12 bytes; u16 view
  `(13365, 1, 1, 0, {1|3|5}, 0)`; 13365 = 0x3435 = ASCII "54".
  The 5th u16 is 1 (zones A/B/C/D + the F zone file), 3 (zone
  E), 5 (zone F mission files) — asset-set id [open].
- **GRAMMAR (VERIFIED corpus-anchored, §7j.33):** records start
  at +0xC, one per .BDG NON-EMPTY record in the same order
  (ZONEA/M1: 197 = 282 − 85 tail-EMPTY rows; EMPTY rows have no
  BLD counterpart). **Record length = 137 + 64·W·H + tail_extra**
  (W/H from the same-index BDG record) — this subsumes the old
  "201 B + k×64-B extension blocks" name-delta observation
  (201 = 137+64·1). The "extension blocks" are actually:
  - **+0x00 head u32s:** [+0] = H (y-extent), [+4] = hp,
    [+8] = chain, [+0xC] = type — identical to the BDG values;
    [+0x10..0x2F] flag/count words [open]. **W and D are NOT
    stored** in the record (no offset matches them).
  - **+0x60 name**, NUL-padded (~33 B field; data resumes
    +0x81). Names: `4barrels`, `square crate`, `gate1`,
    `FENCE 1..6`, `Building #1..10`, `hiddenwall4..10`,
    `bio pod`, `streetlight`, `seans hangar 1`, `sub tunnel
    wall`, `small plane 2`, `EXIT POINT`, … (96–282 records
    per file).
  - **+0x81 FOUR template-bank slots, 16·W·H bytes each:**
    slot+0 u16 = bank[cell 0], slot+2.. u16 array =
    bank[1 : 1+min(n−1, 16)] (n = W·H·D; arrays cap at 16
    values), rest zero pad. The slot values ARE the four BDG
    template banks (+0x3E/+0x42/+0x46/+0x4A, §16) — verified
    equal at every walked record (ZONEA/C/D/E + ZONEF
    M2/M4/M7 = 7 286/7 907 records byte-validated).
  - **Variable tail (≥8 B):** standard two u32(1)s;
    "sub tunnel wall" +12 B (1,5,4,0x1194), "small plane 2"
    +16 B (1,1,1,0xFFFF…), ZONEA's last "EXIT POINT" +320 B
    (zero) — tunnel/animated/exit annotations [open].
- **File end:** zero fill after the last record (≥12 B). There
  is **no record terminator and no count field — .BLD is not
  self-delimiting**; a parser needs the sibling .BDG's (W,H)
  per record (or a name-scan heuristic). This closes the old
  "what RE must confirm" item (the 64-B blocks = bank slots;
  counts live in the BDG; terminator = none).
- **Desync classes [open, bounded]:** ZONEB/G + ZONEF/M6 walks
  desync at a few records (BLD longer than the formula = the
  variable tails; ZONEB/M1 has exactly two); ZONEC/M2+M3 BDG
  non-empty count exceeds the name-scan count by 1 (one
  empty/short name). Details §7j.33.

## 18. CGR — sprite bank: u16 count + self-relative u32 offset directory

- **Size:** exactly 132354 B for **all 44 files**. VERIFIED
- **Directory layout — VERIFIED 44/44 (100 % fit):** LE `u16 count = 128`
  occupies bytes 0..1 (`80 00`). Directory entry `s` begins at `2 + 4·s`;
  its LE u32 offset is relative to that entry, not to the start of the file.
  The first stored offset, 512, therefore places record 0 at absolute byte 514,
  immediately after the directory. The last stored offset, 130814, is in the
  entry at byte 510, placing the final record at byte 131324; its 1030 bytes
  end exactly at EOF 132354.
- **Runtime-selected record layout (VERIFIED, seven shipped
  `ZONE?/MISSION?.CGR` files):** every record is exactly 1030 bytes: a 6-byte
  header `u16(0), u16 width=32, u16 height=32`
  (`00 00 20 00 20 00`), then 1024 raw row-major bytes, with no tail padding.
  Successive stored offsets differ by 1026 because each successive directory
  entry's base advances by 4 bytes while absolute record starts advance by
  1030. This record-layout claim does not generalize to numbered/editor CGRs.
- **Pixel codec — RESOLVED by EXW RE (2026-08-21, docs/RE-EXW-SIM.md §7c.6):
  there is NO codec.** get_z_pos@0041e231 reads the height byte directly:
  `CGR[2 + 4·(type−1) + dir[type−1] + 6 + (sy<<5) + sx]` — a 6-byte sprite
  header then the RAW 1024-byte row-major 32×32 **height map** (the walkability
  floor field; slot 0 = type 1 is 0x1F everywhere, slot 36 = type 37 reads
  0x01 at row starts). The six-byte `{0,32,32}` header is also consumed by
  the render side (P4 render slice input).
- **Contents:** 36 of 44 files are byte-identical; the 7 zone-level CGRs plus
  ZONEE/MISSION4.CGR differ **only in pixel data** — the 128-entry directory is
  identical in every file. VERIFIED
- **Cross-ref (2026-08-21, 7j.26):** the GAMEGFX `*.BIN` sprite banks share
  this exact container grammar — u16 count word0 + u32-offset directory at
  +2, offsets relative to their own slot — decoded from EXW FUN_00401e39
  and corpus-verified 24/24 DEBRIS / 160/160 DANTE (docs/RE-EXW-MISSIONVIEW
  §5f). **VERIFIED 2026-08-22 (7j.36): the MISSION{A..G}[.BIN] zone sprite
  banks follow the SAME self-relative grammar** (u16[0] = count 989..1872;
  entry = bank+2+4·id, sprite = entry + u32[entry]; 11/11 shipped banks
  incl. MISSION6.BIN/MISSION5.BIN monotone and in-file; last record runs
  to EOF). Record grammar: u16 fmt/dy/dx/gate/rows + stream (FUN_00401471
  0x401477..0x4014c8: fmt ≥ 4 u8-RLE, 1..3 u16-RLE, 0 raw; gate==0 or
  rows==0 → draws nothing); ALL real terrain sprites are fmt 7; each zone
  bank carries EXACTLY 9 fmt-0 stub records (6-B head {0,64,64} + 4096-B
  image, span 0x1006) = the VESTIGIAL radar-stamp scratch family
  (u32[0x454b00+4·set]-indexed; written every present by FUN_00401010,
  never drawn — gate/rows 0 forever, LNK identity; TOT refs in zones A–D
  render nothing). [EXW §7j.36]
 - **Bank census add-on (2026-08-22, 7j.28):** the boot loader's
   arena→file pairs pinned from the string block 0x45884e..0x4588c3 +
   corpus headers: WEAPONS 70 imgs (0x4F86 B → [0x4eddbc], alloc
   0x5208), SHRIKE 64 ([0x46af30], 0x1F40), REAPER 64 ([0x46af2c],
   0x1770), SMOKE 4 ([0x46af34], 0x7D0), TELEPORT 10 ([0x46af38],
   0x6D60), NUMBERS ([0x46af3c], 0xFA0), FLAGS ([0x46af40]), GENERAL
   153 ([0x4edd7c], 0x1F7E8) — SHRIKE/REAPER = exactly the 64
   direction frames of the rocket/homing mid-flight draws, SMOKE =
   the 4 exhaust-puff frames (docs/RE-EXW-SIM §7j.28); all
   exact-consumption arenas like DEBRIS/DANTE.

---

## 19. Cross-file relationship map

| Link | Status | Evidence |
|------|--------|----------|
| MAP ⊂ TOT (support superset) | VERIFIED | 0 counterexamples in 37×8 planes; 85 758 added + 4 292 rewritten cells |
| TOT plane 6/7 values < 2000 = POS slots | REFUTED → planes 6/7 are ordinary z-levels (tall-structure sprite ids; domain ≡ planes 1..5, max 1868 there too; 9 217 live/1 681 empty .POS resolutions = coincidence; renderer draws them ungated — RE-EXW-SIM §7j.47) | §7j.47 |
| POS.index → BLD record | SUPERSEDED → indexes the BDG row (7j.25); BLD row count ≡ BDG non-empty count, so the index is bounded by both (ZONEA max 196 < 197) | §7j.25, §7j.33 |
| BDG ↔ BLD same object list | VERIFIED = COMPILED PAIR (EXW §7j.33: BLD record j ≡ BDG non-empty record j — same hp/chain/type heads + the SAME four template banks; BLD = the editor source, BDG = the compiled runtime spec; .POS word 3 indexes the BDG row; BLD never loaded at runtime) | 7 286/7 907 records walked byte-exact incl. all four bank-slot heads; "SAVED.BDL" is the savegame, unrelated |
| LNK ↔ CTG same index space | LIKELY; NOTE .CTG is NEVER loaded at runtime (§0.2 — editor-only like .BLD) | LNK cycles ⊆ CTG nonzero ranges (partial overlap only) |
| LNK, CTG, LNG: three 8192-entry tables | VERIFIED layout / HYPOTHESIS semantics | all exactly 16384 B, near-identity or sparse |
| PAD ↔ TXT pad notes | LIKELY | TXT explicitly describes "pads" with effects; coordinate transform unresolved |
| TRT/MRK/PAD type enums (0–6 / 0–7 / 0–6) | HYPOTHESIS | similar small vocabularies, may be one family |
| CGR sprites ↔ MAP/COL rendering | HYPOTHESIS | 128 sprites, 32×32, shared palette |
| NME n2 = leading 10 B-record count | VERIFIED for all 9 n1=0 non-empty files | exact byte consumption |
| NME = 8 fixed-order sections, widths 10/10/8/8/10/8/6/8 | VERIFIED (loader FUN_00416458, 36/37 byte-exact; ZONEA/M1 has a 16-B unread orphan tail) | §9 + EXW §7j.18 |
| MAP dims bound all coordinate files (MRK, TRT, NME, PAD, POS) | VERIFIED | 100 % in-bounds across every check |

Notable **negative** results (things that did NOT fit):
- MAP/TOT/COL are **not** always 30004 B — the "25×75" anchor is ZONEA only;
  35 missions are 100×100 and ZONEG is 100×25.
- MAP payload is **not** 16-byte-per-tile records in tile-major order (coherence
  test rejects it); it is 8 plane-major u16 layers.
- NME has **no global heuristic stride rule** — the pre-RE heuristic walkers
  failed; the true grammar is 8 fixed-order sections with per-position widths
  (now VERIFIED from the loader, see §9).
- DAT is **not** u16 planes (u16 view shows doubled bytes); it is u8 planes.
- PAD↔TXT coordinates do not correspond under any simple transform.

---

## 20. Summary table

| Ext | Size rule (n=37 unless noted) | Content hypothesis | Confidence | RE must confirm |
|-----|-------------------------------|--------------------|------------|-----------------|
| MAP | 4 + w·h·16; dims 25×75 / 100×25 / 100×100 | 8 u16 planes/tile; plane 0 terrain-IDs, rest overlays | layout VERIFIED; semantics HYPOTHESIS | per-plane meaning |
| TOT | same as MAP | eight u16 runtime tile-word planes plus mirror; meanings beyond anchored consumers unresolved; earlier target-table hypothesis REFUTED | layout VERIFIED; loader/mirror VERIFIED (EXW §7j.16/§7h.4); broader semantics unresolved | what TOT stands for; remaining plane meanings |
| COL | same as MAP | per-tile class codes (≤102; 1 and 37 dominant) | layout VERIFIED; content LIKELY | code vocabulary |
| DAT | 4 + w·h·8 | walkability TYPE grid (plane=z level); PAD writes 0xFF marks | layout VERIFIED; semantics VERIFIED (EXW 7c) | per-type behaviours |
| LNK | 16384 = 8192×u16 (44 files) | orientation-link cycles over object space | layout VERIFIED; cycles VERIFIED; meaning LIKELY | index space + usage |
| CTG | 16384 (44 files) | sparse category table, parallel to LNK | layout VERIFIED; meaning HYPOTHESIS | class vocabulary |
| LNG | 16384 (7 zone files) | third permutation table, same space | layout VERIFIED; meaning HYPOTHESIS | everything |
| MRK | 192 = 12×16 B | spawn markers: (flag, x, y, z-level) — record i spawns robot i | layout VERIFIED; spawn VERIFIED (EXW 7c) | word-3=0 / flag consumers |
| NME | 16–1492 B | critter/personnel placements: 8 fixed sections (10/10/8/8/10/8/6/8 B), §9 | VERIFIED (loader-anchored, EXW 7j.18) | scatter jitter ranges; w0 marker |
| PAD | 5994 = 999×6 B slots; x=0xFFFF terminates | pads: (x, y, z-level) — loader writes DAT[type][y][x]=0xFF | layout + write VERIFIED (EXW 7c) | interactive trigger path |
| PAL | 770 = 2 + 256×3 (40 files, all identical) | 6-bit VGA palette | VERIFIED | leading 2 bytes |
| POS | 32000 = 2000×16 B | object placements (x, y, kind 0–5, BDG-type index); empty = all-FF | layout VERIFIED; index → BDG row VERIFIED (EXW 7j.25/7j.33) | kind semantics tail |
| PTH | 2 (`00 00`) everywhere | u16 count=0 path list | content VERIFIED; layout LIKELY | record format |
| TRT | 2 + count×12 | placed entities (x, y, type 0–6); turrets? | layout VERIFIED; meaning LIKELY | type vocabulary |
| TXT | 409×33, 1649×4 (CRLF ASCII) | designer notes: score codes; pad reference | VERIFIED; EDITOR-ONLY (§0.2) | — |
| BDG | 17100–43644, 282 recs/file | the destructible-object spec library (loader-parsed) | GRAMMAR VERIFIED (EXW 7j.25/7j.32) | — |
| BLD | 29964–96430 (+6 zone files, A≡F, B≡G) | EDITOR-SOURCE object library (names + template banks); NEVER loaded at runtime | GRAMMAR VERIFIED (§17, EXW 7j.33) — editor-only | head flags; variable tails; zone-level (no BDG sibling) |
| MIN | 15824–29952 (7 zone files, A≡D) | raw 4×4 territory-mask entries, 16 B/cw, indexed cw = LNK[TOT word] | VERIFIED (loader-anchored, EXW §7j.62) | — |
| CGR | 132354 (44 files) | self-relative u32 directory; runtime-selected 7: 6 B hdr + raw 1024 B 32×32 maps | directory VERIFIED; codec RESOLVED (raw, EXW 7c) | render-side header use |

---

## 21. Suggested RE attack order

1. **CGR pixel codec** — RESOLVED 2026-08-21 (raw height maps, §18).
2. **BLD/BDG pair** — CLOSED 2026-08-22 (§16/§17, EXW §7j.33):
   BLD = the editor source, BDG = the compiled runtime spec;
   both grammars verified.
3. **NME loader** — CLOSED 2026-08-21: the game loader FUN_00416458 is
   decoded (§9, EXW §7j.18); no editor disassembly needed.
4. **TOT writer** — find the code path that produces TOT from MAP (the pad
   "lowers section" mechanics in the TXT notes must be implemented there).
5. **LNK/CTG/LNG consumers** — one routine likely walks all three; identifying
   the 8192-object index space unlocks three files at once. NOTE (§0.2): only
   .LNK/.LNG are ever loaded (the language gate); .CTG is editor-only.

## 22. LANGUAGE.* — the INI-style localisation pack (boot-table grammar decoded 2026-08-23; EXW §7j.53)

- **Files:** LANGUAGE.{DCH,ENG,FRE,GER,ITL,SPA} — six locales, ~69–79 KB each,
  pure text, CRLF. The active file is picked by the language gate
  ([0x4eba1c] per §7j.30; index 1 = ENG family). VERIFIED
- **Grammar:** `[SECTION]\r\n\r\n[\r\n<record>\r\n<record>\r\n...\r\n]` —
  bracketed sections whose records are LINE-delimited (one line = one
  record; multi-line free text only in the hint/overview sections). VERIFIED
- **Boot tables loaded from it (GameMain):**
  - `[MENU_ITEMS]` (name @0x457abe) → 0x46af5c, 0x30-B records; the boot
    walk bounds at 0x1200 = 64 records while the ENG section carries 96 —
    the overflow is not consumed by this loader. [observed]
  - `[WARNINGS]` (name @0x457ac9) → **0x46c18c, 53 × 0x30-B records** =
    the radio-warning text table indexed by the FUN_004239ef message id
    (all 53 lines + the id map: EXW §7j.53). All six locales carry
    exactly 53 records in the same order. VERIFIED
- **What RE must confirm:** the section-name → file-position resolution
  inside FUN_004424679 (the open-by-section reader) is not yet decoded;
  the [BOOT_CAMP_*] hint sections (§3 of EXW-SIM / D117) use the same
  container.

## 23. MIN — the map-overlay territory-mask bank (loader-anchored 2026-08-25, EXW §7j.62/D149)

- **Files:** exactly **7, ZONE-scoped** — `MISSION{A..G}.MIN` (the family
  loader builds `EDITOR\ZONE{X}\MISSION{X}.MIN` via the zone-stem path
  buffer; every mission load of a zone re-reads the same file). VERIFIED.
- **Sizes:** A/D 23200, B/G 29952, C 27888, E 23280, F 15824 — all 16-B
  entry multiples, all under the 0x7530 (30000) arena bank. **ZONEA and
  ZONED files are byte-identical.** VERIFIED.
- **Content:** **raw 4×4 mask-entry bytes, 16 B per entry, no header, no
  codec** — the LoadFile read is verbatim (§7j.62 B). Entry `cw` is the
  stamp source: byte[r·4+c] == 0 → transparent, else the pixel colour is
  `MAPTRAN_ramp[variant][byte]` (XLAT through the territory-variant ramp
  selected by the robot-proximity ring byte). Entries are indexed by
  **cw = LNK_word[TOT word]** (the zone-level .LNK image, or .LNG under
  the language gate) — the first verified runtime consumer of the LNK
  permutation (§5): its cycles rotate/variant-link adjacent mask entries.
  VERIFIED (EXW FUN_00402ab8 + caller 0x408a8e..0x408ae3; EXD twins
  0x12df3 / 0x197da..0x19841).
- **Reachable surface:** per zone only a subset of entries is reachable
  (union over the zone's missions' TOT words under both LNK and LNG:
  A 349, B 1180, C 1055, D 1008, E 954, F 632, G 271 nonzero cw); every
  reachable entry lies inside the file prefix (max cw·16+16 ≤ size in all
  zones) — the arena tail beyond the file bytes is never read. 9–12
  reachable entries are all-zero per zone (2 in G). Mask byte domain ≤ 254
  (ramp index space). VERIFIED (whole-corpus census).
- **Editor note:** like §0.2's runtime set, .MIN IS runtime-loaded; it is
  absent from the editor-only list. It has no BDG/BLD sibling — the mask
  vocabulary is standalone.
