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
| D | MISSION1–7 | 100 × 100 | MISSIOND.{BIN,CGR,CTG,LNG,LNK,MIN} (no BLD/PAL) + MISSION5.BIN |
| E | MISSION1–7 | 100 × 100 | MISSIONE.{BIN,BLD,CGR,CTG,LNG,LNK,MIN} + MISSION6.BIN |
| F | MISSION1–7 | 100 × 100 | MISSIONF.{BIN,BLD,CGR,CTG,LNG,LNK,MIN} (no PAL) |
| G | MISSION1 | 100 × 25 | MISSIONG.{BIN,BLD,CGR,CTG,LNG,LNK,MIN,PAL} |

37 missions total (1+7·5+1). Total tile counts: ZONEA 1 875, ZONEG 2 500, others
10 000 each; global total 354 375 tiles.

Recurring engine constants seen across formats: **128** (CGR sprite count),
**8192** (LNK/CTG/LNG table length), **2000** (POS slots), **999** (PAD slots),
**12** (MRK slots).

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
- **Relationship to MAP — VERIFIED, with a caveat:**
  - MAP and TOT are **never** byte-identical (0/37).
  - Across all 37 missions and all 8 planes there are exactly **0** cells where
    MAP is nonzero and TOT is zero — TOT's support is a superset of MAP's.
  - 85 758 cells are nonzero in TOT but zero in MAP (additions).
  - 4 292 cells are nonzero in *both* but differ (value rewrites) — so TOT is
    **not** a trivial overlay; e.g. ZONEA/M1 plane 0 tile 409: MAP=347, TOT=789.
  - Plane-equality pattern varies per mission (plane 0 equal in 10/37, plane 7
    equal in 9/37, etc.).
- **Planes 6/7:** almost empty in MAP (4 771 + 1 155 nonzero cells globally),
  fuller in TOT (8 016 + 2 882). TOT plane-6/7 values are ≤ 1868 — just under
  the 2000 POS slot count. ZONEA/M1 has exactly one such cell: tile 642
  (x=17, y=25) with plane6=1355, plane7=1356 (adjacent integers).
  HYPOTHESIS: planes 6/7 store indices into the 2000-slot POS table (or another
  ~2000-entry table). **Negative result:** POS[1355] and POS[1356] in
  ZONEA/M1 are empty (0xFFFFFFFF), so the naive "plane value = POS slot" reading
  is *not* confirmed; the linkage, if any, is indirect.
- **What RE must confirm:** what "TOT" stands for and when the engine reads it
  (working copy? editor "totals"? merged runtime map?), and the plane-6/7
  target table.

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

## 9. NME — enemy/„NME" placement script (partially decoded)

- **Sizes:** vary 16–1492 B (28 distinct). 10 files are **16 bytes of zeros**
  (ZONE{B,C,D,E,F}/MISSION{6,7} — no enemies). VERIFIED
- **No strings anywhere** — all u16 values; records contain in-bounds tile
  coordinates. VERIFIED
- **Confirmed sub-structure:**
  1. **Header:** `u16 n1, u16 n2`. VERIFIED existence.
  2. **Leading record run (VERIFIED for the 9 files with n1=0, n2>0):** exactly
     `n2` records of **10 bytes** `(1, 4, flag, x, y)` (u16 each), with x ≤ w,
     y ≤ h. E.g. ZONEB/MISSION1.NME (n2=24): 24×`(1,4,0,7,64)`, `(1,4,0,38,89)`, …
     consuming bytes 4…243 exactly.
  3. **Sections:** `(u16 count, u16 type)` followed by `count` records, mostly
     **8 bytes** = 4 u16, frequently shaped `(type, x, y, flag)` or
     `(1, n, x, y)` with in-bounds coords.
  4. **Worked exact decode — ZONEA/MISSION1.NME (120 B, 60 u16), VERIFIED:**
     ```
     header (0, 0)
     section (6, 1): (1,1,18,9) (1,1,18,8) (1,1,18,7) (1,1,7,8) (1,1,7,7) (1,1,7,6)
     section (6, 5): (1,13,9,1) (1,22,8,1) (1,22,6,1) (2,3,6,1) (2,2,7,0) (0,0,0,1)
     section (1, 0): (18, 0, 66, 0)          <- x=18≤25, y=66≤75
     ``` — consumes all 60 u16 exactly with uniform 8-byte section records.
  5. Files end with zero runs; `(0,0)` appears to terminate section lists.
- **Honest negative:** no single global grammar was found. A pure
  "(count,type) + count×8B" model parses ZONEA/M1 and the empty files exactly
  but misaligns in the other 25 non-empty files (usually where 10-byte
  `(1,4,…)`-style records recur mid-file); a "header + n2×10B + sections" model
  gets through the leading run everywhere but breaks later. The true grammar
  mixes record widths keyed by the type word (4 → 10 B, else 8 B is the best
  current guess) — HYPOTHESIS, needs disassembly of the loader.
- **What RE must confirm:** the exact record-width rule, the meaning of
  n1/n2, record field semantics (enemy type? patrol group? count of spawned
  units?), and whether sections nest.

## 10. PAD — up to 999 pad (elevator/teleporter) records + 0xFF fill

- **Size:** exactly 5994 = 6 × 999 for all 37 files. VERIFIED
- **Layout (VERIFIED):** `N × 6-byte records (3 × u16)`, then 0xFF fill to the
  end; the first 0xFF marks the end and the rest is all 0xFF (checked: fill is
  pure). Record = `(x, y, type)`.
  - ZONEA/MISSION1.PAD: 114 records; first bytes
    `05 00 3D 00 00 00 05 00 35 00 01 00 …` = (5,61,0), (5,53,1), (10,46,1)…
  - Record counts across missions: 2 … 114. type tally: 0×310, 1×173, 2×51,
    3×50, 4×62, 5×47, 6×8 (7 pad types).
- **Meaning (CONFIRMED storage, EXW RE 2026-08-21 — docs/RE-EXW-SIM.md
  §7c.5):** after loading, the engine writes `DAT[plane=type][y·w+x] = 0xFF`
  for every record (the EXW write is unchecked; shipped type values are
  0..6 and the arena covers them).
  get_from_dat_file reads 0xFF back as tile type 1 — a CGR slot-0
  0x1F-height deck block at level `kind`. So **`type` is the z LEVEL the
  pad materialises its tile at**, matching the TXT "lowers section two
  levels" phrasing (a level change re-marks the DAT cell). Open: the
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
  - kind ∈ 0…5; index ∈ 0…273 with 0xFFFFFFFF also occurring in field 3.
- **Cross-file (LIKELY):** the `index` field never exceeds the mission's BLD
  record count (e.g. ZONEA: max 196 vs ≈285 BLD records; ZONEB/M1: max 230 vs
  ≈344; ZONEF/M5: max 273 vs ≈473) — consistent with *index = BLD record
  (scenery object type)*. Not yet proven.
- **TOT planes 6/7** have values ≤ 1868 < 2000 (see §2) — a POS-slot linkage is
  plausible but unconfirmed.
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
  `(x, y, type)`.
  - ZONEA/MISSION1.TRT @0x0000: `03 00` then (14,15,1), (11,15,1), (10,33,1).
  - ZONEB/MISSION1.TRT: count=19; records (13,45,2), (15,45,2), (1,73,2), …
  - Across all missions: x ≤ 97, y ≤ 97, always within that mission's map
    bounds (no out-of-bounds record found); type ∈ 0…6 (1×265, 2×212, 3×64,
    4×24, 5×5, 6×6, 0×1).
  - 11 files have count = 0 (2-byte file).
- **Interpretation (LIKELY):** per-tile placed entities with a 7-value type
  vocabulary — "TRT" suggests **turrets**; could equally be triggers/traps.
- **What RE must confirm:** type vocabulary and behaviour (compare with MRK
  types 0–7 and PAD types 0–6, which may be one shared enum family).

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

## 16. BDG — binary companion of BLD (per-building data)

- **Sizes:** 26 distinct values, 17100–43644, **all divisible by 4** (u32-array
  friendly). VERIFIED
- **Content:** sparse — mostly zero u32s with small values; long zero tail
  (ZONEA/M1: last nonzero u32 at offset 21808 of 21988). ZONEA/MISSION1.BDG
  @0x0000: `01 00 01 00 01 00 01 00 96 00 00 00 01 00 0F 00 …` — as u16:
  (1,1,1,1,**150**,0,1,**15**).
- **Cross-file (VERIFIED correlation / LIKELY linkage):**
  - Mission-level BDG size vs BLD size: **Pearson r = 0.985** (n=37).
  - BDG's opening u16s (…150, 0, 1, 15) exactly mirror BLD record 0's u32
    fields (1, **150**, 1, **15**) — ZONEB/M1 shows the same with (…50, 0, 0, 5)
    vs BLD rec0 (1, **50**, 0, **5**). The two files describe the same
    object list.
- **What RE must confirm:** whether BDG is indexed by BLD record (parallel
  array) and what its fields encode (graphics/flags/score codes per building?).
  Name guess: "BuiLDinG" data or "badges".

## 17. BLD — scenery/building object library with names

- **Sizes:** mission-level 29964–96430 B (37 files) + 6 zone-level
  (MISSION{A,B,C,E,F,G}.BLD — zone D has none) — 43 files total.
- **Header (VERIFIED constancy):** 12 bytes; u16 view: `(13365, 1, 1, 0, N, 0)`
  where 13365 = 0x3435 = ASCII "54" (magic/version?) is identical in every
  file checked; the 5th u16 varies (1 or 5).
- **Records (LIKELY):** variable-length, **base 201 bytes + k×64-byte extension
  blocks**. Evidence: string-offset deltas cluster at 201 (ZONEA/M1: ×123) and
  at 265/329/393/521/649/713 = 201+64k (×11/×7/×20/×4/×4/×8 across files).
  - Object **name at record offset +96** (file offset 108 for record 0),
    NUL-padded, e.g. `4barrels\0…`.
  - Record 0's leading u32s: `(1, 150, 1, 15, 1, …)` (ZONEA) / `(1, 50, 0, 5, 1)`
    (ZONEB) — 150/50 looks like a count of something per library.
- **Names found:** `4barrels`, `square crate`, `16 barrels`, `gatepost`,
  `bio pod`, `streetlight`, `round container`, `FENCE 1`, `fence straight top
  left`, `secret wall #1`, `guardpost`, `yellow crane part 1`, `telescreen`,
  `exit point`, `sub obj 2` — scenery pieces, fences, secret walls, mission
  objects. (177–271 strings per file.)
- **What RE must confirm:** the 64-byte extension blocks (probably per-facing
  or per-frame graphics references), the count fields, and the exact record
  terminator.

## 18. CGR — sprite bank: u16 count + u32 offset directory (hypothesis CONFIRMED)

- **Size:** exactly 132354 B for **all 44 files**. VERIFIED
- **Directory layout — VERIFIED 44/44 (100 % fit):**
  - `u16 count = 128` (@0x0000: `80 00`),
  - followed by 128 × u32 offsets @0x0002…0x0201; first offset = **512**
    (= 2 + 128×4 — data begins exactly after the directory),
  - offsets are monotonically increasing, last offset = 130814, and
    130814 ≤ 132354 — the final sprite runs to EOF.
- **Sprites:** sizes 1026–1540 B. Sprite 0 (@512) begins
  `01 00 00 00 20 00 20 00 …` = u32(1), u16(32), u16(32) → **32×32** tiles.
- **Pixel codec — RESOLVED by EXW RE (2026-08-21, docs/RE-EXW-SIM.md §7c.6):
  there is NO codec.** get_z_pos@0041e231 reads the height byte directly:
  `CGR[2 + 4·(type−1) + dir[type−1] + 6 + (sy<<5) + sx]` — a 6-byte sprite
  header then the RAW 1024-byte 32×32 **height map** (the walkability
  floor field; slot 0 = type 1 is 0x1F everywhere, slot 36 = type 37 reads
  0x01 at row starts). The 1026-B sprite size = 6 + 1024 exactly; larger
  sizes pad (the single 1540-B tail sprite per file). The `01 00 00 00
  20 00 20 00` header is the u32(1) + 32×32 dims the render side also
  consumes (P4 render slice input).
- **Contents:** 36 of 44 files are byte-identical; the 7 zone-level CGRs plus
  ZONEE/MISSION4.CGR differ **only in pixel data** — the 128-entry directory is
  identical in every file. VERIFIED

---

## 19. Cross-file relationship map

| Link | Status | Evidence |
|------|--------|----------|
| MAP ⊂ TOT (support superset) | VERIFIED | 0 counterexamples in 37×8 planes; 85 758 added + 4 292 rewritten cells |
| TOT plane 6/7 values < 2000 = POS slots | LIKELY | max 1868; but ZONEA/M1 tile 642→1355/1356 while POS[1355] is empty ⇒ indirect |
| POS.index → BLD record | LIKELY | index max per mission < BLD record estimate in all sampled missions |
| BDG ↔ BLD same object list | LIKELY (strong) | size correlation r=0.985; BDG header mirrors BLD record 0 fields exactly |
| LNK ↔ CTG same index space | LIKELY | LNK cycles ⊆ CTG nonzero ranges (partial overlap only) |
| LNK, CTG, LNG: three 8192-entry tables | VERIFIED layout / HYPOTHESIS semantics | all exactly 16384 B, near-identity or sparse |
| PAD ↔ TXT pad notes | LIKELY | TXT explicitly describes "pads" with effects; coordinate transform unresolved |
| TRT/MRK/PAD type enums (0–6 / 0–7 / 0–6) | HYPOTHESIS | similar small vocabularies, may be one family |
| CGR sprites ↔ MAP/COL rendering | HYPOTHESIS | 128 sprites, 32×32, shared palette |
| NME n2 = leading 10 B-record count | VERIFIED for all 9 n1=0 non-empty files | exact byte consumption |
| MAP dims bound all coordinate files (MRK, TRT, NME, PAD, POS) | VERIFIED | 100 % in-bounds across every check |

Notable **negative** results (things that did NOT fit):
- MAP/TOT/COL are **not** always 30004 B — the "25×75" anchor is ZONEA only;
  35 missions are 100×100 and ZONEG is 100×25.
- MAP payload is **not** 16-byte-per-tile records in tile-major order (coherence
  test rejects it); it is 8 plane-major u16 layers.
- NME has **no fixed record stride** — several global grammar attempts failed;
  only the partial grammar above survives.
- DAT is **not** u16 planes (u16 view shows doubled bytes); it is u8 planes.
- PAD↔TXT coordinates do not correspond under any simple transform.

---

## 20. Summary table

| Ext | Size rule (n=37 unless noted) | Content hypothesis | Confidence | RE must confirm |
|-----|-------------------------------|--------------------|------------|-----------------|
| MAP | 4 + w·h·16; dims 25×75 / 100×25 / 100×100 | 8 u16 planes/tile; plane 0 terrain-IDs, rest overlays | layout VERIFIED; semantics HYPOTHESIS | per-plane meaning |
| TOT | same as MAP | MAP superset + dynamic data; planes 6/7 sparse indices | layout VERIFIED; superset VERIFIED; meaning LIKELY | what TOT is for; plane 6/7 target table |
| COL | same as MAP | per-tile class codes (≤102; 1 and 37 dominant) | layout VERIFIED; content LIKELY | code vocabulary |
| DAT | 4 + w·h·8 | walkability TYPE grid (plane=z level); PAD writes 0xFF marks | layout VERIFIED; semantics VERIFIED (EXW 7c) | per-type behaviours |
| LNK | 16384 = 8192×u16 (44 files) | orientation-link cycles over object space | layout VERIFIED; cycles VERIFIED; meaning LIKELY | index space + usage |
| CTG | 16384 (44 files) | sparse category table, parallel to LNK | layout VERIFIED; meaning HYPOTHESIS | class vocabulary |
| LNG | 16384 (7 zone files) | third permutation table, same space | layout VERIFIED; meaning HYPOTHESIS | everything |
| MRK | 192 = 12×16 B | spawn markers: (flag, x, y, z-level) — record i spawns robot i | layout VERIFIED; spawn VERIFIED (EXW 7c) | word-3=0 / flag consumers |
| NME | 16–1492 B | enemy placements: header (n1,n2), 10 B `(1,4,f,x,y)` run, then (count,type)+8 B sections | partial VERIFIED / grammar HYPOTHESIS | full grammar + field semantics |
| PAD | 5994 = N×6 B + 0xFF fill (N≤999) | pads: (x, y, z-level) — loader writes DAT[type][y][x]=0xFF | layout + write VERIFIED (EXW 7c) | interactive trigger path |
| PAL | 770 = 2 + 256×3 (40 files, all identical) | 6-bit VGA palette | VERIFIED | leading 2 bytes |
| POS | 32000 = 2000×16 B | object placements (x, y, kind 0–5, BLD-index); empty = all-FF | layout VERIFIED; index link LIKELY | index/kind semantics |
| PTH | 2 (`00 00`) everywhere | u16 count=0 path list | content VERIFIED; layout LIKELY | record format |
| TRT | 2 + count×12 | placed entities (x, y, type 0–6); turrets? | layout VERIFIED; meaning LIKELY | type vocabulary |
| TXT | 409×33, 1649×4 (CRLF ASCII) | designer notes: score codes; pad reference | VERIFIED | — |
| BDG | 17100–43644, ≡0 mod 4 | per-building binary data, parallel to BLD | layout partial; BLD link LIKELY (r=.985) | field semantics |
| BLD | 29964–96430 (+zone files) | scenery library: 12 B header + 201+64k B records, name@+96 | structure LIKELY | extension blocks; counts |
| CGR | 132354 (44 files) | height-map bank: u16 128 + 128×u32 dir + 6 B hdr + raw 1024 B 32×32 maps | directory VERIFIED; codec RESOLVED (raw, EXW 7c) | render-side header use |

---

## 21. Suggested RE attack order

1. **CGR pixel codec** — directory is solved; decode sprite 0 with candidate
   RLE schemes against the shared palette to get a ground-truth image.
2. **BLD/BDG pair** — parse records from the verified name anchors; BDG is
   likely a fixed-stride parallel array once BLD record boundaries are known.
3. **NME loader** — the only multi-format grammar still open; a disassembly of
   the editor's NME reader would settle the 8-vs-10-byte record rule.
4. **TOT writer** — find the code path that produces TOT from MAP (the pad
   "lowers section" mechanics in the TXT notes must be implemented there).
5. **LNK/CTG/LNG consumers** — one routine likely walks all three; identifying
   the 8192-object index space unlocks three files at once.
