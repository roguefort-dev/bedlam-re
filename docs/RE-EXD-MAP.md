# RE-EXD-MAP — BEDLAM.EXD import + the EXW→EXD address map (P4.2/W1)

Status: IN PROGRESS (2026-08-22, worker d06341cf, claim 1). Purpose: the
DESIGN-DIFFHARNESS.md W1 deliverable — import the B1 DOS build
(BEDLAM.EXD, DOS4GW LE) into the BedlamWatcom Ghidra project, pin the EXD
present/frame-tail site (the S0 dump trigger), and fill `exd_addr` aliases
for the harness watch rows. Scope bounded to T0 + T1 rows; T2-T4 aliasing
stays a later unit.

Tag convention per PLAN §9: [verified] = read from the imported EXD
program this unit; [derived] = arithmetic consequence of verified rows;
[hypothesis] = plausible but unanchored; every mapped row carries DUAL
anchors (two independent evidence pieces) per the W1 ticket.

## 1. Import record (2026-08-22)

- Guards honored: no `analyzeHeadless` java process running (pgrep filtered
  against the agent's own cmdline), no prior `Import succeeded` for EXD in
  any ghidra-project log; MANIFEST.sha256 verified OK immediately before.
- Loader: yetmorecode ghidra-lx-loader `LeLoader` (installed at
  ~/.config/ghidra/ghidra_12.1.2_DEV/Extensions/ghidra-lx-loader since the
  B2 import), prefs unchanged from B2SetLxPrefs (fixup+page labels,
  map-extra, fixup stats ON).
- Command (mirrors the proven B2 form from RESEARCH-BEDLAM2-CENSUS §5):

  ```
  ~/ghidra-12.1.2-watcom/support/analyzeHeadless ghidra-project BedlamWatcom \
    -import game-data/BEDLAM/BEDLAM.EXD -loader LeLoader \
    -processor x86:LE:32:default -cspec openwatcomcpp
  ```

- Log: ghidra-project/import-exd.log. Loader results [verified]:
  - `.object1` CODE mapped 0x00010000-0x00072800 (403,456 B; 23225 fixups
    applied: sel16:1 + off32:23224), selector 000.
  - `.object2` DATA mapped 0x00080000-0x0012583e (678,206 B; 114 off32
    fixups), selector 001. The end 0x12583e = census esp (obj2 top) — the
    zero-fill tail present, exactly the census §1 gate.
  - Entrypoint 0x0005fbb0 = base 0x10000 + eip 0x4fbb0 (object-relative,
    census-pinned).
- Post-import program census: see §1b below (function count, strings).
- MANIFEST.sha256 re-verified OK after the import (game-data read-only).

### 1b. Program census

- TODO (filled after analysis completes): function count, entry-chain
  names, strings scale, comparison vs EXW 675 fns / B2 671 fns.

## 2. EXD present/frame-tail site (the S0 dump trigger)

EXW canon (DESIGN-DIFFHARNESS §2): one harness frame = one MissionShell
loop pass; dump point = the epilogue/present tail after the last state
writer, before the flip. EXW anchors: PresentEnd@0x425a03 (DDRAW flip) +
`g_frame_count++`@0x46ae68 in the loop tail.

EXD expectations [derived]: the DOS build presents via the VESA banked
flip family (B2 prior art: PresentFlip = VesaSetWindow 4f05 + 4f07
display-start + WaitVRetrace double-poll of 0x3da bit 3), NOT DDRAW. The
frame-tail site therefore = the MissionShell-analog loop tail containing
(a) the present/flip call and (b) the frame-counter increment, in that
order or equivalents.

- TODO (filled in §4/§5): EXD MissionShell analog address, the flip
  helper, the frame counter address + increment site, and the exact
  dump-point instruction.

## 3. Mapping method (how EXW rows get EXD aliases)

The two B1 builds are the same game from (presumably) one source tree —
EXW = Win32/DDRAW/DirectSound port, EXD = DOS4GW/VESA/HMI original. Game
logic functions should decompile near-identically; platform layers
differ. Anchoring hierarchy (each mapped row needs TWO of):

1. **Code constants**: RNG additives 0x3619/0x62E9 + seeds 123456/234567;
   damage table (2→20, 3→30 … 0x29→250); platform build 300/199; beacon
   timer 0x197; pod stagger `1+k·(2000−m·1000/27)`; order-validity window
   0x197; hp 250+250·m/27; counts 400×0x36 / 50×0x22 / 250×0x20 / 45×0x24
   / 80×0x1E / 2000×0x14 / 100×0x1E / 12×0x1C; magic words 0x7d2/0x7d3/
   0x7d4; Q13 tile 0x2000.
2. **String refs**: ".TOT"/".DAT"/".CGR"/".BIN"/".MIN"/".NME"/".TRT"/
   ".POS"/".BDG"/".PAD"/"EDITOR\\ZONE"; asset names (WEAPONS.BIN,
   DROPSHIP.BIN, SMOKER.BIN, DEBRIS.BIN, TABLE.BIN, SCANNER.BIN, ROBNUMS,
   MISSION@.BIN); SFX .RAW names (DEADMAN1/2 …).
3. **Call shape**: MissionShell epilogue call order (robots ×6, enemy ×4,
   debris/epilogue/splash/effects ticks, draw chain, present, frame++).

Procedure per row: locate the EXD twin of the EXW anchor function by
(1)+(2), confirm by (3), then read the twin's operand addresses — those
are the EXD aliases. Any EXW↔EXD semantic mismatch found on the way goes
to docs/DIVERGENCES.md as a seed.

## 4. T0 — frame & session (EXW → EXD)

| watch | EXW addr | EXD addr | anchors used | tag |
|---|---|---|---|---|
| frame counter | 0x46ae68 | TODO | loop-tail increment near present; PACER | |
| RNG state A | 0x4ede48 | TODO | seeds 123456/234567; stepper additives | |
| RNG state B | 0x4ede4c | TODO | ditto | |
| score | 0x4dd40c | TODO | chain-detonation `score += type` site | |
| money | 0x46ae70 | TODO | fresh-campaign 4000 plant | |
| difficulty | 0x46cbf8 | TODO | (d+1)%3 cycle; critter table reads | |
| zone | 0x4edd8c | TODO | elevator stager reads (values 1..7) | |
| mission | 0x4edd88 | TODO | elevator stager reads | |
| mode | 0x4edb88 | TODO | elevator stager reads | |
| linear mission m | 0x46ae8c | TODO | pod-stagger formula consumer | |
| SFX master gate | 0x4ede58 | TODO | impact-SFX trio gate | |

## 5. T1 — the P4 slice (EXW → EXD)

| watch | EXW addr | EXD addr | anchors used | tag |
|---|---|---|---|---|
| robot bank | 0x4c69e4, count 0x46ccbc | TODO | robots() manager; stride 0xA8; hit applier | |
| selection triple | 0x46cbd4/dc/d8 | TODO | scanner-overlay reads | |
| blink-cursor selector | 0x4dc5d0 | TODO | 7j.7 producer | |
| per-player selected anchor | 0x4c71c4 | TODO | renderer writes; pre-0x4c71f4 bank | |
| order target xyz | 0x4dd484/88/8c | TODO | FUN_00410644 writer | |
| move-target words | 0x46cc30/0x46cc60 | TODO | command-record bit0 arm | |
| extraction beacon family | 0x4eabb0/b2/b4/b6/b8 | TODO | armer 0x197 + alive==1 gate | |
| spread claims | 0x4eabba | TODO | picker 12×u16 first-free | |
| no-extract latch | 0x46aed4 | TODO | animator gate; boot-clear | |
| tile word grid | 0x460dfa+2·tile | TODO | impact resolver; 0x7d2/3/4 words | |
| platform strength bank | 0x465daa+2·tile | TODO | build 300/199; weaken/destroy | |
| type-DB mirror rows | 0x4796bc+30·tile | TODO | fast z-writer; TOT materializer | |
| type-DB +0x18 fade byte | 0x4796d4+0x1E·tile | TODO | FUN_00424051 head fade walk | |
| variant/flag bytes | 0x4796d5/0x4796d6 | TODO | stamper variant<<4 / 0x80 | |
| object instances | 0x46cbf4, count 0x46cbe8 | TODO | .POS loader; stride 0x14 | |
| TRT array | 0x4cccf8, count 0x46ccd4 | TODO | .TRT loader; 250×0x20; hp 250+250m/27 | |

## 6. Divergence seeds found while mapping

(none yet — every EXW↔EXD mismatch lands here as a DIVERGENCES.md seed)
