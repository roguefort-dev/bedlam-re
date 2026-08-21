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

### 1b. Program census (probe 1, ghidra-project/exd-probe.txt)

- 758 functions (EXW 675, B2 671 — same scale). Largest: FUN_000448e7
  (28,451 B), FUN_0003d253 (27,493), FUN_00036b6f (26,211, INT 0x60 +
  PIT 0x43/0x40 = the HMI sound driver), FUN_0001476d (14,644).
- Blocks: .object1 0x10000-0x727ff R, .object2 0x80000-0x12583d R(Ghidra
  marks R; loader semantics RW), .image overlay 0x0-0x9ff1c.
- Entry 0x5fbb0 decompiles as the Watcom DOS4GW CRT stub (INT 21h AH=30h,
  DX 0x4458 marker, PSP cmdline @0x81, ENV scan) → CRT init chain
  FUN_00064320 / FUN_000642d1 / FUN_0006436b → INT 21h 4C exit. The
  game-init/main dispatch hangs off FUN_000642d1 (B2: CrtInitChain@0x6b1bc
  twin shape).
- Strings corpus (418 defined): C:\MIRAGE\BEDLAM\OPTIONS.BDL @0x850a3,
  SOUND\SPEECH\SPCH*.RAW (B1-only speech family), SOUND\MIDI\* (B1 has
  MIDI scores), GAMEGFX banks GENERAL/SINTABLE/MONOFONT/TINYFONT/SMLFONT/
  DROPSHIP/SPIDER/TERRA/CACO/HUMANS/SENTRYG/SENTRY/BIOMEX3G/BIOMEX3/
  WEAPONS/DEBRIS/ROBNUMS/SMOKER/SCANNER, DEADMAN1/2.RAW @0x869ea/0x86a01,
  "\MISSION" @0x86f9c (path builder), hmidrv/hmidet.386, HMI error
  strings. Note: mission-section strings (.TOT/.NME/.TRT/.POS/.BDG/.PAD)
  are NOT in the defined-data pass — raw scan queued for probe 2.

## 1c. Probe-1 anchor hits (all [verified] = read from the EXD listing)

| EXW anchor | EXW evidence | EXD hit | EXD evidence |
|---|---|---|---|
| PresentFlip family | EXW PresentEnd@0x425a03 (DDRAW) / B2 PresentFlip@0x1066b | **FUN_00010670** | MOV EAX,0x4f07 ×2 @0x106cd/0x10769 + INT 0x10 ×2 + MOV ECX,0x96 ×2 (the B2 cursor-block copy tail) |
| WaitVRetrace | B2 @0x10856 | **FUN_0001085b** | MOV EDX,0x3da @0x10865 |
| VESA mode init | B2 VesaModeInit@0x12290 | **FUN_00012298** | 4f02 @0x12441 + 4f07 + 4f05 + INT 0x10 |
| VESA set-window | B2 VesaSetWindow@0x12ac8 | **FUN_00012516** | 4f05 ×2 @0x12548/0x12560 |
| page mappers | B2 0x128df/0x12960 | **FUN_00012aca / FUN_00012b46** | 4f05 pairs @0x12b1c/31, 0x12b6d/93 |
| RngStepA/B | B2 @0x1220e/0x1224f; additives 0x3619/0x62e9 | **FUN_00012216 / FUN_00012257** | ADC AX,0x3619 + ADD BX,0x62e9 in both |
| RNG-A seed plant | EXW 0x4ede48 ← 123456 | **[0x00107470] ← 0x1e240** | MOV dword ptr [0x107470],0x1e240 @0x596f9 (FUN_000596ed); second site MOV EBX,0x1e240 + MOV EDX,0x39447 @0x2c7bf/ba (FUN_0002c6e3 — mission reseed twin) |
| PIT tick install | B2 TickInstall@0x32546, divisor 0x2e9b | **FUN_0002eb0d** | MOV EAX,0x2e9b @0x2eb91 (100.01 Hz — SAME divisor as B2) |
| tile word grid | EXW 0x460dfa+2·tile (0x7d2/0x7d3/0x7d4 words) | **0x000fe37c + 2·tile** | MOV word ptr [EAX*0x2 + 0xfe37c],0x7d4 @0x33985 (FUN_000337f4 = platform stamper); MOV word ptr [EBX + 0xfe37c],0x7d2/0x7d3 @0x33ea8/0x33ed4 (FUN_00033e44 = the EXW FUN_00422f18 twin) |
| weapon impact resolver | EXW FUN_0041a894 (0/0x7d2/0x7d3 pass, 0x7d4 platform) | **FUN_0002b150** | CMP EDX,0x7d2/0x7d3/0x7d4 triple @0x2b1a1/a9/b1 |
| second resolver site | EXW trap pair FUN_0040fe93/FUN_0040ff92 | FUN_0001c7dc @0x1ca6b/8b | CMP EAX,0x7d3/0x7d2 pair |
| beacon/order armer | EXW FUN_004247b5 (0x197 timer + validity window) | **FUN_0003570e** | MOV ECX,0x197 @0x3572c (the only 0x197 immediate in the EXD code) |
| pod payout 5000 | EXW escape-craft animator FUN_0041fbb1 (+5000) | **FUN_0001f8c1** + score cand | ADD [0x10da28],0x1388 @0x1feca; ADD [0x10da28],0x2710 @0x1fed6 — score home candidate 0x10da28 [hypothesis, needs the chain-detonation site for the dual anchor] |

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
