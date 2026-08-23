# RE-EXD-MAP — BEDLAM.EXD import + the EXW→EXD address map (P4.2/W1)

Status: COMPLETE for the bounded scope (2026-08-22, worker d06341cf,
claim 1): EXD imported, frame-tail/S0 trigger pinned, T0/T1 rows mapped
with dual anchors + the static-after-load table aliases. W5-FOLLOWUP
(2026-08-22, worker ef11271c, claim 2) CLOSED four of the six explicit
gaps: difficulty, order-target triple, keystore, command ring + count
(§5c). D132 (2026-08-23) CLOSED the blink-cursor gap (twin [0x10e108],
7-site census, §5) and, as a by-product, the selection-triple gap (the
label-swap correction + squad-base cell 0x11955c, §5). D133 (2026-08-23)
CLOSED the no-extract-latch gap (twin [0xf929c], §5/§5f). D134
(2026-08-23) CLOSED the LAST gap — the SFX master gate (twin
[0x10743c], 18-site census, §5g): the W1 registry gap set is now EMPTY.
T2-T4 aliasing stays later per the W1 ticket.
Purpose: the DESIGN-DIFFHARNESS.md W1 deliverable — the `exd_addr` fills
for the harness watch rows.

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

## 2. EXD present/frame-tail site (the S0 dump trigger) — PINNED

- **Present/flip = FUN_00010670** [verified, 3 anchors]: MOV EAX,0x4f07 ×2
  (set-display-start flip op) + INT 0x10 ×2 + MOV ECX,0x96 ×2 (the
  0x96-dword cursor-block copy tail — B2 PresentFlip@0x1066b's exact
  339-byte shape and tail). Reads banked-video flag 0x1075a0, page state
  0x107484/0x1074b4, flip lock 0x80088 (B2 twin 0x8008e).
- **MissionShell = FUN_000596ed@0x596ed** [verified, full decompile
  ghidra-project/exd-probe7.txt]: boot flips → mission load chain
  (map FUN_0002e5c3 → spawn/.MRK FUN_0001d9cd → .NME FUN_00026dc1 →
  … .POS/.BDG FUN_0002adb4 — the EXW 0x447b3a→0x447b76 order) → the
  mission loop: epilogue tick chain (FUN_00023967 9,382 B + 12 more
  ticks) → **robots() ×6 phases `FUN_0001c7dc(i, i+1)`** → **enemy ×4
  (`FUN_000212f2(i)` + `FUN_00022a52(i)` + `FUN_0002a0f7` on odd i)** —
  EXW §1 structure exact → draw/UI → flip → counter++ → loop. The P-pause
  spin (key scan 0x19 via FUN_0002ec12, latch 0x107594) matches EXW
  "pause still calls PresentEnd".
- **Frame counter = [0x001195f0]** [verified]: increments in every
  screen loop (INC form) AND in the mission loop tail in REGISTER form
  (`MOV ECX,[0x1195f0]; INC ECX; MOV [0x1195f0],ECX` @0x5a6f0-0x5a6fd —
  why the INC-only census first missed it).
- **THE S0 DUMP POINT = instruction 0x0005a6eb** (`CALL 0x00010670`,
  the mission-loop flip): after the last state writer, before the flip;
  the counter increments immediately after the flip — the EXW
  PresentEnd→g_frame_count++ order exact. DOSBox-X linear breakpoint
  target for W4; the dump reads are side-effect-free.
- Supporting twins: WaitVRetrace FUN_0001085b (0x3da poll); VESA family
  FUN_00012298 (mode init, 4f02) / FUN_00012516 (set window, 4f05) /
  FUN_00012aca + FUN_00012b46 (page mappers); PIT tick install
  FUN_0002eb0d (divisor 0x2e9b = 100.01 Hz, same as B2); INT8 handler
  region ~0x12780-0x127a5 (six counter INCs: 0x801a0, 0x1075e8,
  0x1075fc, 0x1075b4, 0x1075c8, 0x1075e0 — the B2 seven-counter twin).
- **robots()/tick family (sizes CORRECTED W7-followup)**: FUN_0001c7dc
  = the per-PHASE tick (2,712 B; arg = phase 0..5, contains the
  trap-pair resolver reads + the decay/booster/armor/pod-gate pre-pass
  + the move/arrive loop; calls FUN_000448e7, the 28,451-B draw/UI
  monolith whose 0xf75ec move-target writes are the EXW
  order-dispatcher family; 87× FUN_000332f8 + 38× FUN_00033d94 draws).
  FUN_0001476d = the 14,644-B phase monolith (the W1 "(14,644 B)"
  size belonged here — the earlier line mis-attributed it to
  FUN_0001c7dc). FUN_0001ef61 = the damage applier (EXW 0040e230
  twin), FUN_0001d9cd = the spawn initializer (EXW 0040cca0 twin;
  count/marker staging one-hop-confirmed both sides — §5d),
  FUN_0001d274 = robot_move, FUN_0001e440 = the probe writer.

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
| frame counter | 0x46ae68 | **0x1195f0** | mission-loop tail `MOV ECX,[0x1195f0]; INC; MOV back` @0x5a6f0-fd right after the flip 0x5a6eb + INC form in 13 other screen-loop tails | [verified] |
| RNG state A | 0x4ede48 | **0x107470** | plant 0x1e240 @0x596f9 + @0x2c7db; stepper FUN_00012216 read/write | [verified] |
| RNG state B | 0x4ede4c | **0x107474** | plant 0x39447 @0x2c7ba→0x2c7d5; stepper FUN_00012257 read/write | [verified] |
| score | 0x4dd40c | **0x10da28** | resolver FUN_0002b150 `+= ESI` (type) @0x2bff6 + `+= 0xa` (type-0xb→10) @0x2bfed = the EXW chain-detonation rule; debrief payouts += 0x3e8/0x7d0/0x1388/0x2710 (FUN_0001f8c1) | [verified] |
| money | 0x46ae70 | **0x119600** | two 0xfa0 (4000) plants: campaign start `MOV EDX,0xfa0; SUB EDX,EAX; MOV [0x119600],EDX` @0x4ccd3-0x4cce3 + new-game case `MOV EDI,0xfa0; MOV [0x119600],EDI` @0x4e2ab-0x4e2ba | [verified] |
| difficulty | 0x46cbf8 | **0x119558** | the 7j.17 range formula EXACT in the epilogue tick FUN_00023967: `iVar14 < (2 − [0x119558])·(−0x40) + 300` → d=0→172/d=1→236/d=2→300 (decompile line 416, the CMP-d/0/1/2 dispatch @0x24035/4d/65) + the respawn-delay table twin `MOV EAX,[0x119558]; CMP/MOV EDX,[EAX*4+0x81050]` @0x24181-9/0x241f5-0x24200 (EXW DAT_00454edc[d] → EXD table **0x81050**, 3 dwords); 44 refs program-wide, all READ except 3 WRITE sites in FUN_0002c6e3 (the mission-reseed/save-load twin @0x2c831/0x2ceae/0x2cec4) | [verified] |
| zone | 0x4edd8c | **0x107500** | spawn twin per-zone rule `<3∨==7→1, 3→2, else 3` over [0x107500] (EXW FUN_0040cca0 rule exact) + zone-param table reads 0x80bcc/0x80c58 by [0x107500] | [verified] |
| mission | 0x4edd88 | **0x119610** | TRT hp formula `(m·250)/27+250` EXACT (EXW 250+250·m/27) + pod-stagger `2000−m·1000/27` EXACT (both in EXD read [0x119610]) | [verified] |
| mode | 0x4edb88 | **0x1075d8** | SP/MP branch `DAT_001075d8 == 0` in the spawn twin + mission-loop `== 2` MP gates + new-game `MOV [0x1075d8],1`. EXW one-hop PINNED (W8-prep, exw-spawncount-asm.txt): the spawn override gate is `CMP [0x4edb88],0` @0x40cd8d, and the title-menu SP handler @0x43aaa3 sets `0x4edb88=0 ∧ 0x46cbe0=1` (RE-EXW-TITLEMENU §4 [verified]; MP lobby: 1 = Coop, 2 = Head2Head) — the gate is never taken in SP | [verified] |
| linear mission m | 0x46ae8c | **0x119610** (SAME cell as mission) | the stagger consumer reads 0x119610 — EXD uses ONE scalar where EXW has two (see divergence seed #3) | [verified] |
| SFX master gate | 0x4ede58 | **0x10743c** | D134 twin census (§5g): the BOOM-trio twin FUN_00032de9 gates `cmp [0x10743c],0` @0x32df1 ⟷ FUN_00421e60 @0x421e68, shape-identical (prologue, RandB idiv-3, cell trio, play tail `call 0x4c584` ⟷ FUN_0043a48e); 18-site EXD census mirrors the 19-site EXW census (§5g); writers = the sound init FUN_0004be7d (CONFIG.BDL file parse) ⟷ FUN_0043a144 (registry SOUND + probe) | [verified] |

## 5. T1 — the P4 slice (EXW → EXD)

| watch | EXW addr | EXD addr | anchors used | tag |
|---|---|---|---|---|
| robot bank | 0x4c69e4, count 0x46ccbc | **base 0xf6d34, count 0x11958c** (stride 0xA8 same; cap cell 0x11950c) | armer alive loop `[0x11958c]×0xA8` over [0xf6db0+i]=presence@+0x7C; state w@+0xC via [0xf6d40+i·0xA8]:=3; hp@+0x78 via `[EAX+0xf6dac]=0x1388` (5000 respawn base); **pod timer w@+0x2C** via the stagger store `[0xf6d60+i·0xA8] = 1+k·(2000−m·1000/27)` (formula EXACT). COUNT-MAPPING CORRECTED (W8-prep 2026-08-22, ghidra-project/exw-spawncount-asm.txt): EXW 0x46ccbc is the TOTAL banked count (the spawn staging-loop bound @0x40ce74) = the EXD **cap 0x11950c** twin; EXD **0x11958c is PER-PLAYER** — its true EXW twin is **0x46cbd8** (the zone-rule target @0x40cd5c..0x40cd80; MP := 1 @0x40cdad). In SP both cells equal the zone rule, so this row's SP evidence held; they diverge only in MP (0x46ccbc := [0x46cbe0], 0x46cbd8 := 1). SP S0/S1 plan rows: count = cap = 1, no impact — future MP scenarios must bound the bank dump by the CAP cell | [verified] |
| selection triple | 0x46cbd4 / 0x46cbdc / 0x46cbd8 | **selected-slot 0x11954c (≡ EXW 0x46cbdc), squad-base 0x11955c (≡ EXW 0x46cbd4), squad size 0x11958c** | LABEL-SWAP CORRECTED (D132, 2026-08-23 — history: this row first pinned "selected idx 0x11954c" against EXW 0x46cbd4): the blink-cursor census proved the pairing is swapped — the EXW twin of 0x11954c is **0x46cbdc** (the SELECTED-SLOT/cursor cell), and the former "cursor gap" cell is **0x11955c ≡ 0x46cbd4** (the SQUAD-BASE cell, player·per-player-count). Evidence: (1) the MissionShell auto-switch — the §5 original evidence for 0x11954c — reads/writes **0x46cbdc** on the EXW side (cmp/mov @0x448109/0x448111) and 0x11954c on EXD (@0x5a117/0x5a124); (2) the arm strips + chase gate pair the cells 1:1 — EXW k=0 strip compares idx vs [0x46cbd4] where EXD compares vs [0x11955c] (0x40c1b2 ↔ 0x1cecc), and the chase-camera gate reads [0x46cbdc] then [0x46cbd4] where EXD reads [0x11954c] then [0x11955c] (0x423e8c/0x423e9c ↔ 0x34e20/0x34e30); (3) the global-index computation `eax := [0x11955c] + [0x11954c]; ·0xA8` @0x5a871 (EXW 0x4480c1 imul [0x46cbd4],0xa8); (4) the EXW ref-count shape matches: 0x46cbd4 is the ~100-site heavy sidebar/UI cell ↔ EXD 0x11955c (~100 sites, portrait family 0x180a7..0x191xx); 0x46cbdc is the ~40-site sparse cell (key handlers 0x40731a/0x407886 cmp, cmd-builder 0x44a11f word) ↔ EXD 0x11954c (0x180fc/0x18617 cmp, 0x5b5b5 word). Squad size: EXW 0x46cbd8 = EXD 0x11958c per the W8-prep count-mapping correction (robot-bank row). ALL THREE cells now mapped — the selection-triple W1 gap is CLOSED | [verified] |
| blink-cursor selector | 0x4dc5d0 | **0x10e108** | THE EXD TWIN CENSUS (D132, 2026-08-23, objdump from tools/exd-relod.py → ghidra-project/exd-text-objdump.txt, the §7j.59/D131 anchor template): EXACTLY 7 .text sites, one-for-one with the EXW 7-site census. WRITERS: arm strips k=0/1/2 `mov [0x10e108],ecx(1)` @0x1cef1 / `mov [0x10e108],0x2` @0x1cf2c / `mov [0x10e108],ecx(3)` @0x1cf72 (⟷ EXW 0x40c1d7/0x40c217/0x40c254; k=0: idx==[0x11955c], posts (0xC,0)+(0xF,·,1); k=1: idx==[0x11955c]+1 ∧ [0x11958c]>1, (0xD,1)+(0xF,1); k=2: +2 ∧ >2, (0xE,2)+(0xF,2) — all via the warning-post twin FUN_00034972 ≡ EXW FUN_004239ef); impact-completion tail `mov [0x10e108],ebx(0)` @0x34f7f (⟷ 0x423fef, in the shell-resolver FUN_00034d89 ≡ EXW FUN_00423e1c, after the 3×3 nine-blast patch FUN_00035406, + record-valid word := 0); MissionShell reset `mov [0x10e108],ecx(0)` @0x59842 (⟷ 0x447871, in the zero-cascade between map-overlay [0x1075bc] ≡ EXW [0x4edba0] and salvo latch [0x1081fc] ≡ EXW [0x4de658]). READERS: portrait-pass blink gate @0x186dc (⟷ 0x407428, inside FUN_000180a1 ≡ EXW FUN_004072bf): `(frame [0x1195f0] & 3) + 0x51`, literal 1/2/3 x-dispatch → x=0x1F0/0x222/0x254, y=0xD, bank ptr [0x1074fc] ≡ EXW [0x4edd7c] (GENERAL.BIN), draw FUN_000111fa ≡ EXW FUN_00401ca2, 0 AND >3 draw nothing; chase-camera record-0 impact gate @0x34e25 (⟷ 0x423e91): SP ∧ rec-0 ∧ [0x11954c]+1 ≠ [0x10e108] → record ([0x11955c]+[0x10e108]−1)·0xA8 kind@+0x2A == player-type [0x1075c0] → cut FUN_0003552e ≡ EXW FUN_004245c9. VALUE GRAMMAR {0,1,2,3} = endangered slot+1, :=0 at reset/first impact — EXACT. Gates identical: SP arm needs state==0 ∧ local ∧ idx==[0x11954c]+[0x11955c] (⟷ EXW [0x46cbd4]+[0x46cbdc] @0x40c119); MP every idle robot; idle threshold = [0x8105c][difficulty] = {400,300,200,5000} BYTE-IDENTICAL to EXW 0x454ee8; zone∉{1,7}, latch==0, mode≠2. Shared arm tail: idle +0x70 := 0, [0x1081fc] := 0x80, 8-shell scatter into bank 0x8f0b4 (stride 0xA ×8, x/y words @+0/+2 record-relative, RandA()&0x7f−0x3f / −0x80, map-bounds check; ≡ EXW 0x4ea238 — grammar in §5e) | [verified] |
| per-player selected anchor | 0x4c71c4 | **0x971a4** | spawn-tail seed loop `do {[0x971a4+i]=x>>8; [0x971a8+i]=y>>8; [0x971ac+i]=z} ×4 (0x30/0xC)` — EXW 4×0xC {x>>8,y>>8,z} EXACT | [verified] |
| order target xyz | 0x4dd484/88/8c | **0x10e0a4/0x10e0a8/0x10e0ac** | consumer twin FUN_00019ee9 bit1-ORDER branch writes all three @0x1a0af/0x1a0bc/0x1a0d8 (words@+7/+9/+0xB of the record, EXW EXACT) + the click-order twin FUN_00021112 writes the triple from the pick (FUN_0002a271, EXW FUN_00419943): ground branch iso combine, rect branch reads rec@0x9df30-base, the `&0x2000` structure flag EXACT; loop position = EXW MissionShell trio EXACT (FUN_00021112 → FUN_0005b066(1) builder → FUN_00019ee9 consumer = EXW FUN_00410644 → FUN_00449c94 → FUN_00409138) | [verified] |
| per-robot move-target words | 0x46cc30/0x46cc60 | **0xf75ec / 0xf761c** | spawn −1-init stores at both + the 0x30 gap twin (EXW 0x46cc60−0x46cc30 = 0x30 = EXD 0xf761c−0xf75ec) + all writers in the order monolith FUN_000448e7 (47 refs). EXTENT PINNED (W7-followup): per-robot u32 ×2 indexed by ABSOLUTE robot id over the CAP cell 0x11950c (tick loop `+= 4` per record; 0x11950c := 0x11958c SP / 0x119588 MP, ≤ 12); the fixed 0x60-B span at 0xf75ec covers x[12]+y[12] deterministically — FILLED (W7-followup2, D90): the dbx-plan row emits the 0x60 span and the differ splices the trio into the robot-bank fields | [verified] |
| extraction beacon family | 0x4eabb0/b2/b4/b6/b8 | **0x119628/0x11962a/0x11962c/0x11962e/0x119630** | armer FUN_0003570e full decode (guard/timer 0x197/tile trio) + mission-loop countdown `(short)DAT_0011962a −−` with the digit draws and the all-state-3 → FUN_00030899 completion sweep | [verified] |
| spread claims | 0x4eabba | **0x119632** | picker FUN_0003581b full decode: first-free u16 scan `[0x119632+i]`, bound = cap cell 0x11950c, marks 1, the 12-offset switch around beacon x/y — EXW FUN_004248c8 EXACT | [verified] |
| no-extract latch | 0x46aed4+i·4 (12 u32, boot memset 0x30) | **0xf929c+i·4** | THE EXD TWIN CENSUS (D133, 2026-08-23, same substrate as D132: tools/exd-relod.py → ghidra-project/exd-text-objdump.txt): 12 .text sites, 8 of them ONE-FOR-ONE with the EXW 8-reader census. READERS ⟷ pairs: 0x19c71 ⟷ 0x408ef7 (FUN_00408e99 death-anim walk: ≠0 → image 0x65, ==0 → tables [0x82e5a]/[0x82e8a] via [idx·0xC+0x8b618]≠0 selector ⟷ EXW 0x456ce8/0x456d18 via 0x4ebaac); 0x1f4cf ⟷ 0x40e7a1 (death core: MP cell [0x1075d8]≠0 ∧ latch==0 → respawn re-init staging 0x107768/0x107764/0x107770 ⟷ EXW 0x4edb88/0x4ea8ec/f0/f8, else the SP death tail); 0x30c87 ⟷ 0x4200db (escape-pod animator: pod record [0x8d314+0x1C·idx] active ∧ latch==0 → phases 2/3 ⟷ EXW 0x4e64c0 bank); 0x5b1cc/0x5b34a/0x5b51c ⟷ 0x449dc8/0x449ee8/0x44a08c (MP cycler trio: cursor [0x107688] ≠ current [0x1075c0] ∧ latch==0 → switch FUN_0006209c(…, rec 0x9255c+[0x1075c0]·0x80) ⟷ EXW FUN_00449b60 + 0x4dd4a0); 0x5b7ea (`cmp edi(0),[latch]` — codegen swap) ⟷ 0x44a322 (cycler 4); 0x5b89c ⟷ 0x44a3d2 (MP endgame census: marks [0x8b744]+0x30 clear, count [0x10760c]++ ⟷ EXW 0x4eba30/0x4edb8c). BOOT-CLEAR ⟷ pair: GameMain memset(0xf929c, 0x30=12 dwords) @0x2cd41 ⟷ EXW (0x46aed4, 0x30) @0x41c412 — the 12-slot bank extent both sides, NOT per-mission. **WRITER ASYMMETRY (the D133 headline):** EXD has exactly ONE setter — FUN_0005bb71 @0x5bba0 `mov [edx·4+0xf929c],esi(1)` (the MP LOBBY ROBOT-PICK: [0x1195dc]:=idx, [0x1195bc]:=0x32, call FUN_000347a3(idx), alive@+0x7C:=0 @0x5bbb3 = [idx·0xA8+0xf6db0], memset(0x9255c+idx·0x80, 0x80), then the same latch==0 census cmp 2 + message 0x8720b; callers 0x5ba27 the lobby tally + 0x1b2bd) + the EXD-only lobby type-tally walk @0x5ba83 (16-dword bank 0x8b5d4, staging-rec type byte @+6, <0x10 gate). EXW has NO setter at all — census-complete: all 9 literal sites are the 8 readers + the boot memset, no memset/rep-movs span overlaps the array (corroborated + data-side loophole closed by the 07cf72bf re-verification, same date: a raw-dword scan of the whole BEDLAM.EXW file image finds EXACTLY 9 occurrences of 0x0046aed4, every one mapping via BEGTEXT raw 0x400/va 0x1000 (VA = file+0x400C00) to a known site's disp32/imm32 operand — 0x82f9→0x408ef9, 0xdba4→0x40e7a4, 0x1b80e→0x41c40e, 0x1f4dd→0x4200dd, 0x491cb→0x449dcb, 0x492eb→0x449eeb, 0x4948f→0x44a08f, 0x49725→0x44a325, 0x497d5→0x44a3d5; DGROUP/.idata/.reloc hold ZERO — no initialized pointer cell or table can alias the array, and a .bss pointer would need a code literal initializer, already excluded by the sweep), and the four functions the §7j.19/§7j.27 rows named as "writers" (FUN_0040e230/00449c94/0044a38a/00408e99) are READERS (SIM rows corrected this unit). SEMANTICS CORRECTED: the latch is the per-robot CLAIMED/CONSUMED flag (lobby pick claims; claimed robots get no pods, no MP re-drop, no cycler switch) — NOT a death-core-written "no more pods" flag; on EXW every gate takes the ==0 path at runtime (writer set empty). By-products: the per-TYPE sibling 0x46ae94+type·4 (writers 0x40d01b/0x40d028/0x40ea61/0x40ea6a {1,reg,1,2}, readers ==1/==2, clear 0x447aa6 len 0x30) is a DIFFERENT array — do not confuse; §5f carries the ten D133 cascade aliases | [verified] |
| tile word grid | 0x460dfa+2·tile | **0xfe37c+2·tile** | 0x7d4 store `[EAX*2+0xfe37c]` @0x33985 + 0x7d2/0x7d3 stores `[EBX+0xfe37c]` @0x33ea8/0x33ed4 + resolver CMPs + platform-ring empty-check reads | [verified] |
| platform strength bank | 0x465daa+2·tile | **0xf93cc+2·tile** | platform ring FUN_000337f4 (EXW FUN_00422832 twin): empty-check `[0xf93cc+tile]==0` + strength store `:= unaff_CX` beside the 0x7d4 stamp + scorch+4 tail | [verified] |
| type-DB mirror rows | 0x4796bc+30·tile | **0xac1e4+0x1E·tile** (z-word row+2z) | platform ring: `[0xac1e4 + tile·0x1E + z·2] == 0` z-word check with the EXW 0x1E-stride row shape + plane-B==1 DAT check via z-line table 0x107714 | [verified] |
| type-DB +0x18 fade byte | 0x4796d4+0x1E·tile | **0xac1fc+0x1E·tile** | [derived: row base 0xac1e4 + 0x18; the fade walk twin anchors on the row base] | [derived] |
| variant/flag bytes | 0x4796d5/0x4796d6 | **0xac1fd / 0xac1fe** (+0x1E·tile) | [derived: row base +0x19/+0x1A] | [derived] |
| object instances | 0x46cbf4, count 0x46cbe8 | ***(0x119584) bank, count 0x119554** (indirect: EXD stores the bank in a pointer cell) | .POS/.BDG loader FUN_0002adb4: 2000×0x10 reads (cap CMP 0x7d0 @0x2ae45/86), id≠−1 → count, stride 0x14 (piVar1+5); footprint stamper FUN_0002b0af tail | [verified] |
| TRT array | 0x4cccf8, count 0x46ccd4 | **0x95264 (static), count 0x11949c** | .TRT loader FUN_000279e3: u16 count read, stride 0x20 (i·8 dwords), hp `(m·0xfa)/0x1b+0xfa` EXACT, active=1/state=1 stamps, tile-0x66 DAT byte + TOT word 1 | [verified] |

**W5 input-twin note — CLOSED 2026-08-22 (worker ef11271c, claim 2,
probes ghidra-project/exd-input-twin{,2,3,4}.txt via
tools/ghidra-scripts/EXDInputTwin{,2,3,4}.java, `-process BEDLAM.EXD
-noanalysis`):** see §5c. The FUN_0002ec12 dead end stands (P-latch
spin only). KEYSTATE/ORDER/COMMAND injection steps now compile for O1
(dbx-plan reads §5c's registry rows); the PAD step still awaits the
capgen runtime pad-slot op (its die names both halves).

### 5c. Input & command family (EXW → EXD) — the W5-followup census

Provenance: 4 Ghidra probe passes over the imported EXD (listing-text
immediate censuses, ref censuses, decompiles, targeted disasm). All
rows [verified] = read from the EXD program this unit; EXW column from
RE-EXW-INPUT / RE-EXW-SIM §7j.16-17 ledger rows.

| EXW | EXD | anchors | tag |
|---|---|---|---|
| g_keystore 0x4edc44 (256 B, scan-indexed, 1=held) | **0x894d4** (256 B) | AnyKeyWait twin FUN_00030792 @0x30792-0x307c0: `EAX:=1; loop: [EAX+0x894d4]!=0 ∧ EAX∉{0x2a,0x36} → consume(byte:=0) return code; EAX++; EAX<0xff` — the EXW FUN_0041f9d1 scan 1..0xFE skip-both-shifts shape EXACT + the DOS KeySink = an INT 9 hook installed by FUN_0003064d (vector set via FUN_0005fe87(9,…,&LAB_000303f5,…)): make/break handler @0x30446-0x30495 (`AL&=0x7f; keystore[AL]:=1/0` with the held-keys counter 0x107534 INC/DEC) + the ARROW-REMAP twin `OR BL/AL,0x80` @0x304a5/0x304d3 (E0-prefixed extended scancodes → bytes 0xc8/0xcb/0xcd/0xd0) + the installer's memset `MOV EDI,0x894d4` @0x30665 (the EXW InputReset 0x4207b5 memset-256 twin; FUN_0003064d also zeroes the aux cell 0x1194c4 + counter 0x107534) | [verified] |
| remapped arrow bytes 0x4edd0c/0f/11/14 | **0x8959c / 0x8959f / 0x895a1 / 0x895a4** | [derived: keystore base 0x894d4 + 0xc8/0xcb/0xcd/0xd0; the ISR OR-0x80 path stores them] | [derived] |
| keystore[ESC] byte 0x4edc45 | **0x894d5** | direct-byte readers: FUN_0004c80c/0004f1d1/00052fd7/00057775 ×2/0005b066 ×3/0005b853 (B3 census) — the EXW ESC-family readers | [verified] |
| held-keys counter (EXW has none) | **0x107534** | ISR INC/DEC @0x30456/0x30488 (EXD-specific bookkeeping; aux state cell 0x1194c4 set 2/cleared in the ISR) | [verified] |
| ScanToChar FUN_0041fa02 | **0x307c1-0x307e8** | `keystore[0x2a] (0x894fe) ∨ keystore[0x36] (0x8950a)` → table word>>16: shifted **0x8097a** / unshifted **0x8077a** | [verified] |
| command records 0x4dd4a0 (stride 0x80) | **0x9255c** (stride 0x80) | builder twin FUN_0005b066 (EXW FUN_00449c94; called FUN_0005b066(1) in the MissionShell loop = EXW's call): append cursor = `&0x9255d + player·0x40` (short-scaled = byte +1 of record player·0x80), marker byte@+0 := DAT_001075c0 (player type), id short@+1 := [0x11954c] (selected idx), spot short@+3 := [0x10e15c] (order word), flags byte@+5 := [0x11a51a], payload = rand&0xf / MP weapon-mask / flags&1 move-words 0x119484/0x119488 / flags&2 the order-target triple words + consumer twin FUN_00019ee9 (EXW FUN_00409138): record walk with id = marker·robot_count([0x11958c]) + slot, flags bit0 SELECT → move-target writes 0xf75ec/0xf761c + auto-arm (state ∉ 2..5 → state:=1, target@+0x74 := 1000000 = 0xf4240 @0xf6da8 base), bit1 ORDER → triple 0x10e0a4/a8/ac := words@+7/+9/+0xB + order-active 0x10e140 := 1 + five clears 0x1076b4/70/90/a4/7c (EXW clears 0x4eb940..50), weapon dispatch on the robot's 7 slots w@+0x36+8k: w 2/3/4 → FUN_0001c3fb (EXW FUN_0040b615) orders 3/2/1, w 6/7/8 → FUN_0001bd8f (EXW FUN_0040af98) 0/1/2, w 9/0xA/0xB artillery → FUN_00023295 free slot + 0x36-stride record into the projectile bank (see below), 39-case bound CMP 0x26 | [verified] |
| command count 0x46cbe0 | **0x119588** | consumer loop bound `if ([0x119588] <= rec) → cooldown-tail/return` + the ring-modulo read `(_DAT_00107688 + 1) % [0x119588]` + `_DAT_00107658 += [0x119588]` (builder family FUN_0005b066 ×8 reads) + 6 WRITE sites in FUN_0004c80c (the net/input pump — also the keystore[ESC] reader; mission-start resets @0x4ccd8/0x4cd6a). SP staging PINNED (W8-prep, §5d): title-menu "New Single Player Game" @0x43aaa3 sets 0x46cbe0 := 1 (the host marker only); the spawn override that would read it is gated on `[0x4edb88] != 0` @0x40cd8d and is never taken in SP | [verified] |
| order-active flag 0x4dc6bc | **0x10e140** | consumer bit1 branch `:= 1` (EXW `_DAT_004dc6bc := 1`); also cleared 0 at MissionShell boot (probe7 line 81) | [verified] |
| order spot staging (EXW: none separate) | **0x10e15c** | consumer stores record word@+3 → 0x10e15c; builder reads it back as the spot short — EXD keeps a staging cell EXW inlines | [verified] |
| command flags staging (EXW: none separate) | **0x11a51a** | builder reads flags byte from it; MissionShell ORs 4 into it on the MP path (DAT_001075d8 != 0, probe7 line 480) | [verified] |
| projectile bank 0x4c71f4 (400×0x36) | **0x980d4** (×0x36) | consumer artillery case writes the record: type w@+0 (0x980d4), owner d@+2 (0x980d6), ttl d@+0xA (0x980de), xyz d@+0x12/+0x16/+0x1A (0x980e6/ea/ee), vxyz d@+0x1E/+0x22/+0x26 (0x980f2/f6/fa), +0x2A := 4, ttl2 `0x900 − (RandA&0x2ff)` @+0x2E — EXW §7j.17 field map EXACT (T2 registry fill lands with the T2 unit). SLOT COUNT RE-CONFIRMED (W12-S3 hop, ghidra-project/exd-projbank.txt): the free-slot twin FUN_00023295 walks stride 0x36 from 0x980d4 with the bound `iVar1 < 0x5460` = 400·0x36 EXACT; the weapon-anim tick twin = FUN_000212f2 (MissionShell enemy ×4 family; its 0x17 case performs the 3-CLONE SPLIT at 0x980d4 base — `(&DAT_000980d4)[slot·0x1b] := 0x17` (0x1b words = 0x36 bytes), parent xyz copy from +0x12/+0x16/+0x1A, damped v −= v>>1 at +0x1E/+0x22 — the §7j.37 clone split EXACT) | [verified] |
| projectile bank 0x4cc654 (50×0x22) | **0x10e174** (×0x22) | [verified, W12-S3 hop, ghidra-project/exd-projbank.txt — the enemy ×4 tick twins]: the projectile tick = FUN_00022a52 — 50-slot walk (`local_2c` 0..0x31, stride 0x22) over the type word @0x10e174 (switch values 0x65..0x68 = the §7j.28 draw-dispatch family; the lane reads use the (base−2)+2 word trick `*(int*)(&DAT_0010e170 + i·0x22 + 2) >> 0x10` = the type), xyz d@+2/+6/+0xA (0x10e176/0x10e17a/0x10e17e); FUN_0002a0f7 (odd-i robot-hit lane) reads the same record (|dx Q8|<0x10, |dz|<0x20 lanes, FUN_0001ef61 damage + FUN_0002a428(type) disburser). TAIL WORDS beyond the 7 EXW-modeled fields (type+xyz+v = 0x1A B of the 0x22 stride): +0x1A (0x10e18e) a clamp-0..7 per-tick incrementing counter (type-0x65 branch), +0x1E (0x10e192) a −1 countdown whose zero CLEARS the type — E's `EnemyProjectile` models neither (the S3 T2 row's documented coverage gap; O1-only fields, never fabricated) | [verified] |

Order-dispatcher reader twins over the triple (EXW FUN_0040af98 ×3,
FUN_0040a56f/0xa7a1/0xace8/0xb615/0xa9ff ×2 each → EXD FUN_0001b369,
FUN_0001b598, FUN_0001bae0, FUN_0001bd8f, FUN_0001c3fb, FUN_0001b7f7;
the 0xf75ec/0xf761c move-target writers stay in the FUN_000448e7 UI
monolith as mapped in §5).

Divergence seeds found this unit → §7 items 6-7 (attack-break
randomness source; EXD-only staging cells).

### 5d. The W8 robot-count override pin (2026-08-22) — ANSWERED

**Question (DESIGN §10-W8 / D85): does the original SP path fill the
network-marker override at 0x46cbe0?** No. Pinned both sides:

- **EXW** (FUN_0040cca0 @0x40cd4c..0x40ce23, raw disasm
  ghidra-project/exw-spawncount-asm.txt, no new Ghidra run — extracted
  from the 7j.27 exw-text-objdump): `per_player [0x46cbd8] := zone
  rule (zone [0x4edd8c]: <3∨==7→1, ==3→2, else 3)`; `total [0x46ccbc]
  := per_player`; **the override branch is gated on `[0x4edb88] != 0`
  @0x40cd8d** — taken only for network sessions, where `total :=
  [0x46cbe0]` (command count = players), `per_player := 1`, and
  markers `record[i]+0x2A := i` for i < cmd count.
- **EXD** (FUN_0001d9cd, exd-robot-backhalf2.txt lines 466-495): the
  instruction-for-instruction twin — `if (mode [0x1075d8] == 0)` SP
  keeps the zone rule (count 0x11958c, cap 0x11950c := count); MP sets
  `cap := 0x119588`, `count := 1`, markers `record[i]+0x2A := i`.
- **SP staging source** (RE-EXW-TITLEMENU §4 [verified]): "New Single
  Player Game" @0x43aaa3 sets `0x4edb88 := 0` **and** `0x46cbe0 := 1`
  (the host's own marker only). The MP lobby sets `0x4edb88 := 1`
  (Coop) / `2` (Head2Head). So in every local session the gate is
  closed and `[0x46cbe0]` holds just the host default.

**Consequences:** (1) SP ZONEA banks **one** robot in EXW, EXD, and E
(D85 host-default staging) — robot-count parity holds across all three
channels; robot-count diffs in SP scenarios are a genuine finding
class, not a staging artifact. No E-side staging seam changes (the
conditional "(if the original fills it)" deliverable is moot). (2) The
count-cell mapping correction in the §5 robot-bank row. (3) Faithful
quirk recorded: the SP marker write targets `record[12]+0x2A` — one
past the 12-record bank — because the index register still holds the
finished 12-iteration MRK-copy counter; both twins do it identically
(EXW lands at 0x4c71ee inside the 0x4c71c4 anchor bank, which this
function's own tail @0x40d175..0x40d18d re-stamps immediately after;
EXD lands in the dead gap 0xf7514..0xf75ec). No diff surface: the
anchor-bank watch reads post-stamp state.

### 5e. Cascade/asset aliases pinned by the D132 blink-cursor census (2026-08-23)

All [verified] this unit from the relocated objdump (tools/exd-relod.py →
ghidra-project/exd-text-objdump.txt; every row carries the EXW twin site):

| EXW | EXD | anchor (what the decode showed) | tag |
|---|---|---|---|
| salvo-cooldown latch 0x4de658 | **0x1081fc** | arm tail `mov [0x1081fc],esi(0x80)` @0x1cf87 (⟷ EXW [0x4de658]:=0x80 in the shared 0x40c25e tail) + reset-cascade zero @0x59848 (⟷ 0x447877) + arm-gate `!= 0 → skip` @0x1cea5 (⟷ 0x40c18b) | [verified] |
| 8-shell bombardment bank 0x4ea238 | **0x8f0b4** (8×0xA records) | record grammar IDENTICAL both sides, base-relative: {x w@+0, y w@+2, fall w@+4 (seed 0xFF), start-delay w@+6, valid w@+8} — EXW record k = 0x4ea238+k·0xA, EXD record k = 0x8f0b4+k·0xA (the 0x4ea236/0x8f0b2 dword-read sites = the Watcom read-word-at[base−2]≫16 idiom for x; rebased from the first draft's 0x8f0b2/+2/+4 phrasing this unit); scatter writes the five words @0x1d00e/0x1d01a/0x1d026/0x1d02d/0x1d038 x/y/fall/valid(1)/start-delay (⟷ EXW 0x40c323..0x40c348, same five stores with the last two in swapped codegen order; map-bounds gate `[0x1074b8]` ⟷ `[0x4eddec]`); resolver walk `add ecx,0xA`, bound `cmp esi,8` @0x34fa0/0x34fab; fall `sub 0x20` @0x34f91 (⟷ 0x424001); record-valid w@+8 := 0 @0x34f85 (⟷ [rec+8]:=0 @0x423ff5); third consumer = the restamp z-stack restart gate 0x1751c..0x17579 (⟷ EXW 0x4066f4..0x406749, §7j.47) | [verified] |
| map-overlay flag 0x4edba0 | **0x1075bc** | reset-cascade zero @0x5983c (⟷ EXW 0x44786b, immediately before the cursor store in both) | [verified] |
| viewport zoom 0x4ede54 | **0x107448** | reset cascade `mov [0x107448],edx` (edx=0x1e0) @0x59854 (⟷ EXW 0x447883, same edx=0x1e0 staged at cascade head 0x5981b/0x44784a) — the §7j.56/D128 zoom cell's EXD alias | [verified] |
| idle-threshold table 0x454ee8 | **0x8105c** (4 dwords) | arm gate `mov ecx,[0x8105c+difficulty·4]` @0x1ce81 (⟷ EXW 0x40c167 `[0x454ee8+diff·4]`); table bytes {400,300,200,5000} BYTE-IDENTICAL both sides; sibling respawn table 0x81050 {1500,900,600} (§4 difficulty row) | [verified] |
| GENERAL.BIN bank ptr 0x4edd7c | **0x1074fc** | portrait blink draw `mov esi,[0x1074fc]` + call FUN_000111fa @0x1872b (⟷ EXW [0x4edd7c] + FUN_00401ca2 @0x407948) | [verified] |
| warning poster FUN_004239ef | **FUN_00034972** | arm strips call it with ids (0xC/0xD/0xE, 0xF) + slot args (⟷ EXW §7j.53/D125 ids) @0x1cedb..0x1cf6d | [verified] |
| shell-fall resolver FUN_00423e1c | **FUN_00034d89** | contains the cursor reader 0x34e25 + impact tail 0x34f7f + the 3×3 nine-blast patch (FUN_00035406) + per-record fall counters (⟷ §7j.54) | [verified] |
| chase-camera cut FUN_004245c9 | **FUN_0003552e** | call site 0x34e69 with the shell x/z args (⟷ §7j.54/§7j.59.C.3) | [verified] |
| portrait pass FUN_004072bf | **FUN_000180a1** | the §6c.6d twin containing the blink gate 0x186dc (function entry = nearest call target 0x180a1, already §8's hit_flash portrait pass) | [verified] |

### 5f. Cascade/asset aliases pinned by the D133 no-extract-latch census (2026-08-23)

All [verified] this unit from the relocated objdump (same substrate as
§5e; every row carries the EXW twin site). The robot-bank base pin
0xf6d34 is re-confirmed INDEPENDENTLY of the §5 robot-bank row's
original anchor (the respawn staging reads [ebp+0xf6d34]/+8/+0xC at
0x1f4f1..0x1f518 ⟷ EXW [ebp+0x4c69e4]/+8/+0xC at 0x40e7c3..0x40e7ea):

| EXW | EXD | anchor (what the decode showed) | tag |
|---|---|---|---|
| MP-mode cell 0x4edb88 | **0x1075d8** | death-core gate `cmp [0x1075d8],0; je SP-tail` @0x1f4bf (⟷ 0x40e791 `cmp ds:0x4edb88,0; je 0x40ea77`) | [verified] |
| current-robot cell 0x4edb90 | **0x1075c0** | the cycler trio's "current" operand + `0x9255c+[0x1075c0]·0x80` record pick (⟷ `0x4dd4a0+[0x4edb90]·0x80`); REFINES the D132 gloss "player-type [0x1075c0]" — it is the current-robot id, used as the player-type index in the chase gate | [verified] |
| MP staging records 0x4dd4a0 (12×0x80) | **0x9255c** | record base of every cycler call + the lobby tally walk `edx := 0x9255c+idx·0x80` @0x5ba8c; EXW memset family 0x600 @0x449cb1/0x448f07/0x43d3e8/0x447a4c; type byte @rec+6 | [verified] |
| MP marks array 0x4eba30 (0x30) | **0x8b744** | census head memset 0x30 + `[eax·4+0x8b744]:=1` for current @0x5b859..0x5b891 (⟷ 0x44a392..0x44a3bf) | [verified] |
| MP endgame count cells 0x4edb8c / 0x4eba28 | **0x10760c / 0x107660** | the census `inc [0x10760c]` on latch==0 + the `count−1 vs [0x107660]` game-over compare @0x5b8a5/0x5b8c7 (⟷ 0x44a3dc/0x44a3e4) | [verified] |
| cycler cursor 0x4eba00 | **0x107688** | the trio's wrap-around cursor (`idiv`-family walk) @0x5b1bf..0x5b1cc (⟷ 0x449db9..0x449dc8) | [verified] |
| cycler word 0x4eba08 / result 0x4dc6e0 | **0x11a9a6 / 0x10e0c0** | the switch-call args + `mov ds:0x10e0c0,eax` result store @0x5b1fe (⟷ 0x449e14) | [verified] |
| cycler msg gate 0x4edc45 | **0x894d5** | `cmp BYTE [0x894d5],0` before the message trio @0x5b19d (⟷ 0x449d97) | [verified] |
| robot switch fn FUN_00449b60 | **FUN_0006209c** | the cycler callee (5-arg push shape identical) @0x5b1f6 (⟷ 0x449de6) | [verified] |
| msg post pair 0x44d2ac/0x44d2da | **0x5ef05/0x5ef33** | string-then-count message posts @0x5b1b0/0x5b1ba (⟷ 0x449daa/0x449db4) | [verified] |
| death-anim selector family 0x4ebaa0 (0xC stride: flag 0x4ebaac, idx 0x4ebab0) | **0x8b60c (flag 0x8b618, idx 0x8b61c)** | the FUN_00408e99 walk `cmp [idx·0xC+0x8b618],0 → [idx·4+0x82e8a] else [idx·4+0x82e5a]` @0x19c88..0x19ca6 (⟷ 0x408f0e..0x408f2c) | [verified] |
| death-anim image tables 0x456ce8 / 0x456d18 | **0x82e5a / 0x82e8a** | same walk, the two `[idx·4+base]` lookups flanking the 0x65 literal | [verified] |
| escape-pod bank 0x4e64c0 (0x1C stride) | **0x8d314** | animator gate `cmp [ebp+0x8d314],0` then latch then phase [ebp+0x8d318]∈{2,3} @0x30c77..0x30ca4 (⟷ 0x4200cf..0x4200f8) | [verified] |
| respawn staging quad 0x4ea8ec/f0/f4/f8 | **0x107768 / 0x107764 / 0x10776c / 0x107770** | x>>8, y>>8, raw z, 0x20 stores at 0x1f4fa/0x1f50d/0x1f51e/0x1f518 (⟷ 0x40e7cc/0x40e7da/0x40e7ea-region/0x40e7ef; EXD emits z AFTER the 0x20 — codegen order swap only) | [verified] |
| memset fn FUN_00402965 | **0x12206** | every (edi,ecx) clear pair incl. both latch boot-clears | [verified] |

**EXW-side by-product census (the per-TYPE sibling 0x46ae94, stride 4,
cleared 0x30 @0x447aa6 per-mission):** writers 0x40d01b :=1, 0x40d028
:=ebp, 0x40ea61 :=edi(1), 0x40ea6a :=2 (the loadout jump-table cases,
indexed by the TYPE word ≫16 of [rec+0x4c6a0c]); readers 0x41ee30/0x41f111
(==1), 0x41f80a (==2). NOT the latch — recorded so the two per-robot-family
arrays in 0x46ae94..0x46af04 are never conflated.

### 5g. The SFX-master-gate twin census (D134, 2026-08-23) — the last W1 schema gap closed

**THE TWIN = [0x10743c]**, pinned by the arrival-SFX family anchor the
queue named: the EXD BOOM-trio twin FUN_00032de9 (gate `cmp [0x10743c],0;
je 0x32f9a` @0x32df1) is shape-identical to EXW FUN_00421e60 (gate
@0x421e68) — same push-order prologue (ebx/ecx/esi/edi/ebp/eax), same
`mov ebp,edx`, same RandB call (EXD 0x12257 ⟷ EXW 0x4029b6) with the
signed idiv-3 dispatch, same per-arm `push 0x2; mov ebx,[esp+4]; mov
eax,<cell>; mov ecx,ebp` bodies over the BOOM cell trio, same shared
play tail (EXD `call 0x4c584` @0x32f95 ⟷ EXW `call 0x43a48e` — THE
PLAY TWIN: FUN_0004c584 ≡ FUN_0043a48e). Whole-objdump greps: EXW 19
literal sites ('4ede58'), EXD 18 ('10743c') — both sides census-complete,
no displacement-form or address-load strays (the one EXW address-load is
the registry loader below).

**EXW census (19 sites).** WRITERS (2, both in the sound-system init
FUN_0043a144, entry 0x43a144 `push ebx; push edx; push esi; push edi`,
SOLE caller GameMain 0x41c33f, raw-dword scan zero hits): 0x43a198
:=ebx(1) SET branch / 0x43a1b1 :=edi(0) CLEAR branch. Init head:
`eax=[0x4ddb2c] sar 1; edx=0x3e8; call 0x44c630` (driver init/volume),
then the 16-entry voice-table fill loop `[0x4eada8+eax]:=edx` for
eax=0x10..0xa0 step 0x10 (value 0x3e8), then the branch: gate==0 →
`[0x4ee9b0]:=-1` (disabled forces "no hardware"); `[0x4ee9b0]==-1` →
CLEAR (also `[0x4ede5c]:=0`, `[0x4eb93c]:=0` SPEECH, `[0x46ae84]:=0`)
else SET (also `[0x4ede5c]:=1`, `[0x46ae84]:=0xfe000`). VALUE SOURCE =
the Win32 REGISTRY, not the init: boot loader FUN_004252c0 @0x42530a
`mov edx,0x4ede58; mov eax,0x458cb7 ("SOUND"); call 0x44ede4` (the D128
bounded loader, HKCU\Software\Mirage\Bedlam\1.00; the volume cell
[0x4ddb2c] is loaded two instructions earlier @0x4252f0); saver
FUN_0042540c @0x4253f3 `mov ebp,[0x4ede58]` → FUN_00444ed98
RegSetValueExA("SOUND" @0x458d07) at the name-entry exit. READERS (13
cmp + the saver read): the arrival/impact family FIVE — 0x421df9
FUN_00421dec (RICOCHT1-4, priority 1), 0x421e68 FUN_00421e60 (BOOM1-3,
p2), 0x421ede FUN_00421ed6 (GRUNT1-3, p2), 0x421f54 FUN_00421f4c
(DEATH1-3, §7j.24), 0x421fca FUN_00421fc2 (HURT1-3, §7j.23); the
music-sequencer trio 0x4033df FUN_004033d4 / 0x4034fa FUN_004034ef /
0x40364d FUN_00403642 (each gates ∧ [0x4edbe0] ∧ [0x4ede5c]; the first
also ∧ [0x4ee9b0]≠-1); the radio-warning queue consumer 0x423af7 (inside
FUN_00423a85, §7j.53: arg≠0 ∧ [0x4ede5c] ∧ [0x4ede58] ∧ id∉{0xF,0x29});
the driver-sync wait 0x425bfe in FUN_00425bf5 ([0x4ede5c] ∧ [0x4ede58]
→ spin `call 0x44c600`); the play dispatcher's own master gate 0x43a79e
inside FUN_0043a48e (pair [0x4ede5c]/[0x4ede58], fail →
`[0x46ae78]:=1` drop-flag); the init's own 0x43a16c; the MissionShell
volume-key pair 0x447e72 (up) / 0x447efd (down) — gate
([0x4ede58]≠0 ∨ [0x4edbe8]≠0) ∧ key-latch ∧ repeat-timer<0x12 → volume
±5 clamp [0,0x64], the ×0x147≫7 scale, `call 0x43a48d`/`0x44c630`.

**EXD census (18 sites).** WRITERS (2, both in the sound init
FUN_0004be7d, entry 0x4be7d; callers 0x2cc70 (boot) AND 0x5b03f (the
title path — EXW has only GameMain; the init's already-on guard
`cmp [0x107444],0; jne skip` @0x4bed3 makes re-calls idempotent)):
0x4c0c8 :=ecx(1) SET / 0x4bf85 :=edi(0) CLEAR. SET also
`[0x107444]:=1`, `[0x119620]:=0xfe000`, and the voice-table fill loop
`[0x8b938+eax]:=ebx(0x3e8)` for eax=0x10..0xa0 step 0x10 —
INSTRUCTION-EXACT twin of the EXW 0x4eada8 loop; CLEAR also
`[0x107444]:=0`, `[0x10766c]:=0` (SPEECH), `[0x119620]:=0`.
**THE CONFIG DIVERGENCE (the "who sets it" answer):** EXD parses the
FILE **CONFIG.BDL** — the init copies the runtime install-dir buffer
[0x9237c] to the stack, appends "CONFIG.BDL" (0x867ea; companions
"r+b" 0x867f5, "NO SOUND FX" 0x867f9, "Sound initialisation failed"
0x86805), probes via FUN_0005f23f (result → [0x1076ac]) +
FUN_0005f471; parse failure → CLEAR. EXW instead pre-loads the
registry SOUND scalar (0x42530a) and forces CLEAR through the
[0x4ee9b0]:=-1 path — the DOS file-config vs Win32 registry port seam
(the EXW TITLEMENU "CONFIG.BDL" gloss was already retired by D128;
this is the EXD side of that same port). READERS (15 cmp + 1 read):
the arrival family FIVE one-for-one — 0x32d88 (RICOCHT quad twin:
cells 0x11a918/0x11a910/0x11a914/0x11a920, priority 1), 0x32df1
FUN_00032de9 (BOOM trio: 0x11a944/0x11a940/0x11a93c), 0x32e68 (GRUNT
trio: 0x11a8b8/0x11a8b4/0x11a8b0 — cell order REVERSED vs EXW
0x4ee000/04/08), 0x32eda (DEATH trio: 0x11a948/0x11a8d8/0x11a8dc),
0x32f49 (HURT trio: 0x11a938/0x11a930/0x11a934); the play twin's own
gates 0x4c593 (entry master gate, pair [0x10743c]/[0x107444] — arg
order swapped vs EXW 0x43a79e, codegen only) and 0x4c9a9 (mid-body
drop-flag gate → `[0x1195f4]:=1`); the music-sequencer trio
0x13e0f/0x13f26/0x1406a (each gates ∧ [0x107578]); the frame-tick
music hook 0x12767 (FUN_00012762: [0x10743c] ∧ [0x107444] ∧
[0x107578] → call 0x135ef, then `inc [0x801a0]`); the radio-warning
consumer twin 0x34a8e (gates [0x10766c] SPEECH ≠0 ∧ [0x107444] ∧
[0x10743c] ∧ esi∉{0xF,…} — the EXW twin's first gate reads its edi
ARG, independently confirming [0x10766c]≡[0x4eb93c] and
[0x107444]≡[0x4ede5c]); the driver-sync twin 0x3696f (gates
[0x107444] ∧ [0x10743c] → `call 0x60472(eax=[0x11a898])` ⟷ EXW
FUN_00425bf5/0x44c600); the MissionShell volume-key pair 0x59eae (up)
/ 0x59f29 (down) with the EXACT scale `imul ebx,[0x1081f0],0x147; sar
ebx,7` and the [0x107570] OR-leg. EXW-only sites (no EXD twin): the
registry loader address-take + the saver read + the init's own
0x43a16c — all three absorbed by the file-config divergence (the EXD
init never reads its own gate). EXD-only: 0x12767 + the second init
caller 0x5b03f.

**Bank-name walk (FUN_0004c121, called from the MissionShell reset
cascade @0x5982a — the §4 lead):** loads via FUN_0004c3dd with name
pointers past the shared "SOUND\SFX\" prefix (0x86837..), stores
handles: MIDIGUN→0x11a954 (+0x11a958 dup), BOOM1/2/3→
0x11a944/0x11a940/0x11a93c, SQUISH2/3→0x11a950/0x11a94c, HURT1/2/3→
0x11a938/0x11a930/0x11a934, DEATH1/2/3→0x11a948/0x11a8d8/0x11a8dc,
PLASMA→0x11a8e4, RICOCHT1..4→0x11a918/0x11a910/0x11a914/0x11a920,
MISSILE1→0x11a8e0, POWERUP→0x11a924, ELEV1/2→0x11a8e8/0x11a91c,
DEADMAN1/2→0x11a8d4/0x11a8d0, BEEP5→0x11a92c (+0x11a8ec dup),
TEXTBOX1→0x11a8f8 ("COULD NOT FIND SAMPLE:%s" 0x86a39 = the miss
message). The GRUNT trio is NOT in this walk — it rides the
MissionShell head walk @0x59b79..0x59c09 via FUN_0004c384 (a
mission-scoped loader flavor) with BEAMIN/THROW/PEXPLODE/BIOFIRE/
CACODETH/SQUAWK companions (the §7j.30/D120 mission-bank family).

**Cascade aliases pinned as by-products (all [verified]):**

| EXW | EXD | anchor (what the decode showed) | tag |
|---|---|---|---|
| sister gate 0x4ede5c | **0x107444** | init SET/CLEAR tandem writes; the play-gate pair 0x4c593/0x4c9a0 ⟷ 0x43a79e/0x43a795; the radio triple | [verified] |
| SPEECH cell 0x4eb93c | **0x10766c** | init CLEAR zero + the radio consumer first gate (EXW arg edi ≡ EXD cell load @0x34a6e) | [verified] |
| mixer arena cell 0x46ae84 | **0x119620** | :=0xfe000 / :=0 in both init branches | [verified] |
| 16-entry voice table 0x4eada8 | **0x8b938** | the 0x10-step/0xa0-bound/0x3e8 fill loop instruction-exact (0x4c0a6..0x4c0bb ⟷ 0x43a15a..0x43a16a) | [verified] |
| music gate 0x4edbe0 | **0x107578** | sequencer trio second gate (0x13e1c ⟷ 0x4033ec) + the frame hook | [verified] |
| music OR-leg 0x4edbe8 | **0x107570** | volume-key pair OR-leg (0x59eb7 ⟷ 0x447e7b) | [verified] |
| master VOLUME 0x4ddb2c | **0x1081f0** | volume-key ±5 clamp [0,0x64] + `imul ·0x147; sar 7` scale (0x59f0e ⟷ 0x447ed5); boot load @0x4252f0 ⟷ the EXD config parse | [verified] |
| play drop-flag 0x46ae78 | **0x1195f4** | :=1 on the play master-gate fail (0x4c9b2 ⟷ 0x43a7a7) | [verified] |
| volume-scale byte 0x4edec5 | **0x80333** | the `mov al,[byte]` operand inside the same scale call (0x59f1d ⟷ 0x447ee4) | [verified] |
| BOOM1-3 cells 0x4edf64/68/6c | **0x11a944/0x11a940/0x11a93c** | FUN_00032de9 dispatch ⟷ FUN_00421e60 | [verified] |
| HURT1-3 cells 0x4edf7c/80/84 | **0x11a938/0x11a930/0x11a934** | FUN_00032f41 gate fn ⟷ FUN_00421fc2 | [verified] |
| DEATH1-3 cells 0x4edf88/8c/90 | **0x11a948/0x11a8d8/0x11a8dc** | FUN_00032ed2 ⟷ FUN_00421f4c | [verified] |
| RICOCHT1-4 cells 0x4edf98/9c/a0/a4 | **0x11a918/0x11a910/0x11a914/0x11a920** | the 0x32d88-gate quad ⟷ FUN_00421dec | [verified] |
| GRUNT1-3 cells 0x4ee000/04/08 | **0x11a8b8/0x11a8b4/0x11a8b0** (REVERSED) | the 0x32e68-gate trio ⟷ FUN_00421ed6; MissionShell-head load 0x59bdf/0x59bf3/0x59c09 | [verified] |
| DEADMAN1/2 cells 0x4edfb8/bc | **0x11a8d4/0x11a8d0** | the FUN_0004c121 walk (0x4c270/0x4c27f) | [verified] |
| PLASMA cell 0x4edf94 | **0x11a8e4** | the FUN_0004c121 walk (0x4c1e9) | [verified] |
| play twin FUN_0043a48e | **FUN_0004c584** | the five-family shared tail `call 0x4c584` @0x32f95 ⟷ `call 0x43a48e` | [verified] |
| driver-sync FUN_00425bf5 | **FUN_00036966** | gate pair + spin call (0x36966..0x3697d ⟷ 0x425bf5..0x425c0e) | [verified] |
| sound init FUN_0043a144 | **FUN_0004be7d** | the branch twin + voice-table loop + arena cell (callers differ: GameMain-only ⟷ boot 0x2cc70 + title 0x5b03f) | [verified] |

**Engine/differ consequence:** NONE — the T0 cell is a
session-constant config scalar (set once by the init from the
machine's sound config; 0 only when sound is disabled/absent). The
watch row now emits 4 B/frame on O1; the E side keeps its W6 row list
(sfx-master-gate stays a documented E-gap exactly like
no-extract-latch after D133 — a future E config model can emit it as
constant 1). A live O1 capture machine with sound DISABLED would dump
0 where E-side chains assume the row exists — recorded as the S0
fingerprint-step companion to the D128 ACTIONPAN registry note (one
dbgprobe read of [0x10743c] at the anchor stop settles it).

### 5b. Static-after-load table aliases (DESIGN §4 one-shot dump)

| EXW | EXD | anchor | tag |
|---|---|---|---|
| type table 0x4dedf2 | **0x108428** (static) | .BDG parse: stride 0x4E, 282 recs (loop bound 0x55ec), 5×8B entries @+0x16.., template bank ptrs @+0x3E/0x42/0x46/0x4A (0x108466/6a/6e/72) — EXW layout byte-exact; arena cursor 0x1195f8 (EXW 0x46ad5c) | [verified] |
| TOT volume ptr 0x4ede20 | ***0x107454** (pointer cell) | map loader ".TOT" → FUN_0002d57c(buf, DAT_00107454) | [verified] |
| DAT volume ptr 0x4edd58 | ***0x107518** (+4 = u8 planes) | ".DAT" load + the ≥0x7F→0 sanitize sweep + every volume read (TRT stamp, platform plane-B) | [verified] |
| CGR ptr 0x4edd60 | ***0x107540** | ".CGR" load | [verified] |
| BIN terrain bank ptr 0x4ede1c | ***0x107434**; header word → **0x11a4a8** (EXW 0x46cdb8) | ".BIN" load + `_DAT_0011a4a8 = *DAT_00107434` | [verified] |
| MIN bank ptr 0x4edd9c | ***0x107538** | ".MIN" load | [verified] |
| LNK map 0x45cdda | **0x10336c** | mode-indexed ".MAP/.LNK" strings 0x862c2/0x862c7 load (mode cell 0x10768c) | [verified] |
| PAD slots 0x4e44f8 | **0xf63c** (999×8, size imm 0x1f38) | ".PAD" load `FUN_0002e55a(…, &DAT_000862cc, …, 0x1f38)` | [verified] |
| map w/h 0x4eddec/0x4eddf0 | **w 0x1074b8 / h 0x10748c** (w·h → 0x1074e4) | TOT header words → cells; every bounds check (platform ring, resolver) | [verified] |
| tile-claim bank 0x46af58 | ***0x119564** (pointer cell) | platform ring claim check `… + DAT_00119564 == 0` | [verified] |
| order table 0x4de664 (0x62 stride) | **0x91ee4** | spawn weapon/equipment copy `type·0x62` + the 0x2a/0x2b/0x2c extras switch @0x9240c (row 28) | [verified] |
| player TYPE word 0x4edb90 | **0x1075c0** | spawn SP path `robot[0].type := DAT_001075c0` + mission-loop auto-switch `type == DAT_001075c0` | [verified] |
| y-line/z-base 0x4ea900/0x4eaacc | y-multiplier table **0x8b78c**, z-line **0x107718**, z-idx **0x107714** | map-loader build loops + volume reads | [verified] |
| dither noise bank 0x4e6ed8 (cursor 0x4ddb30) | **0x8ded4**, cursor **0x108424** | mission-loop churn: 15 B/frame, `RandB()&3==0 → 0xFF` else 0, ring wrap 0x800 — EXW 7i EXACT | [verified] |
| cursor clamp (INPUT) | x max **0x1074ac**=0xf0, y max **0x1074b0**=0x140 | GameInit boot plants (320×240 logical space, B2 twin) | [verified] |

EXD↔EXW layout note (divergence seed #1): the beacon armer writes the
teleported x/y at robot+0x00/+0x04 (dwords, Q13) while EXW robots carry
x@+4/y@+8 — the EXD record front is shifted 4 bytes vs EXW (state stays
@+0x0C in both; presence +0x7C and hp +0x78 verified in both). The
harness normalizes to canonical fields (DESIGN §6), so this affects the
field map only.

## 6. Mission-loader string block (EXD, [verified] raw scan)

.MRK 0x85064 · .NME 0x85087 · .TRT 0x8508c · .POS 0x85094 · .BDG 0x85099
· .TOT 0x862a9 · .DAT 0x862ae · .CGR 0x862b3 · .BIN 0x862b8 · .MIN
0x862bd · .PAD 0x862cc · second .TOT/.DAT pair 0x86f85/0x86f8a (the
EDITOR\ZONE restore-reload twin) · "EDITOR\" 0x86f8f · "ZONE" 0x86f97 ·
"\MISSION" 0x86f9c. Bank strings: DANTE 0x862d1, SCANNER 0x862e3,
BLOWUP(G) 0x862f7/0x8630b, WEAPONS 0x8631e, SHRIKE 0x86332, REAPER
0x86345, SMOKE 0x86358, TELEPORT 0x8636a, NUMBERS 0x8637f, FLAGS
0x86393, VICERA 0x863a5, DEBRIS 0x863b8, SHIELD 0x863cb, ROBNUMS
0x863de, TABLE 0x86405, DIGITS 0x8642b, SMOKER 0x86452, IDIOTGFX
0x86465 — the EXW 7j.26/7j.28 bank family confirmed name-for-name.

## 7. Divergence seeds found while mapping

All are field-map/normalization concerns (canonical differ unaffected);
none change game rules. To be recorded in docs/DIVERGENCES.md when that
file is next touched.

1. **Robot record front**: EXD {x@0, y@4, state@0xC} vs EXW {x@4, y@8,
   state@0xC} (beacon-armer decode; presence +0x7C, hp +0x78, pod timer
   +0x2C identical in both).
2. **EXD merges the EXW function split**: one session driver
   FUN_000596ed (MissionShell), one tick monolith FUN_0001c7dc
   (robots()+resolver), one draw/UI monolith FUN_000448e7 (EXW's
   renderer FUN_00403938 + sidebar_control + order dispatchers).
   Address-map consequence: EXW watch anchors on separate functions
   resolve into these three EXD containers.
3. **One mission scalar**: EXD 0x119610 serves both the EXW mission
   0x4edd88 (TRT hp formula) and the EXW linear-m 0x46ae8c (pod
   stagger). Both EXW rows alias to 0x119610.
4. **Indirect banks**: EXD keeps several banks behind pointer cells
   (object instances *(0x119584), TOT/DAT/CGR/BIN/MIN volumes, claim
   bank *(0x119564)) where EXW uses static bases. Watch rows must
   read-through the pointer; the registry schema carries an `indirect`
   note.
5. **EXD-only /KARMA cmdline switch** (GameInit FUN_0002c6e3 → cell
   0x11960c) — no EXW counterpart; non-scenario surface.
6. **Critter attack-break randomness source** (§5c, W5-followup): EXD
   gates on FRAME-COUNTER+TIMER masks (`TEST AL,0x1f/0xf/0x7` per
   d=0/1/2, FUN_00023967 @0x24035-0x2407d) where EXW gates RandA
   draws (1/8, 1/16, never per 7j.17 @0x41353e/56/6e) — different
   randomness SOURCE and an inverted mapping (EXD d=2 breaks most
   often, EXW d=2 never). A live T2/T3 diff class.
7. **EXD staging cells the EXW build inlines** (§5c): order word
   0x10e15c, command flags 0x11a51a, held-keys counter 0x107534 —
   watch-artifact class, not gameplay divergence.

## 8. The W7 normalizer field map (robot record + row forms) — PINNED 2026-08-22, back half added 2026-08-22 (D88)

The W7 differ (DESIGN-DIFFHARNESS §6/§6a) converts O1 raw guest bytes
into the §6a canonical grammar per registry row. The map below is the
per-field EXD evidence table (robot record, base 0xf6d34, stride 0xA8).
Rows 1-4 + back anchors from the W7 unit; the remaining 23 leaf fields
+ the drop_countdown CORRECTION pinned by the W7-followup back-half
probe (probes ghidra-project/exd-robot-backhalf{,2}.txt via
tools/ghidra-scripts/EXDRobotBackhalf{,2}.java, `-process BEDLAM.EXD
-noanalysis`; census = every instruction whose immediate lands in
0xf6d34..0xf6ddc, plus decompiles of the writer family):

| canonical field | EXD off | type | provenance |
|---|---|---|---|
| pos_x | +0x00 | i32 Q13 | [verified] beacon armer teleported x/y (seed #1) + the per-player anchor writer `d@(0xf6d34+i)>>8` → 0x971a4 |
| pos_y | +0x04 | i32 Q13 | [verified] same pair (`d@(0xf6d38+i)>>8`) |
| z | +0x08 | i32 | [verified] the per-player anchor writer's third word: `d@(0xf6d3c+i) + 0x20` (exd-probe5.txt — the 4×0xC {x>>8, y>>8, z+0x20} triple reads +0/+4/+8 of the record) |
| state | +0x0C | u16 | [verified] armer `[0xf6d40+i·0xA8] := 3` (§4) |
| dir_byte | +0x0E | u16→i32 | [verified-NEW] move twin FUN_0001d274 @~0x1d2f5: `w@(0xf6d42+i·0xA8) := angle_byte` at every move start; the diagonal branch reads `d@+0x0E>>0x10` (= the facing word) — EXW §3 "+0x0E last dir byte" EXACT |
| facing | +0x10 | u16→i32 | [verified-NEW] spawn-init FUN_0001d9cd `w@(0xf6d44+i·0xA8) := 0xFFFF` (spawn-none); move twin writes the four cardinals 0x00/0x40/0x80/0xC0 + 0xFFFF fallback — EXW §3 row EXACT |
| anim | +0x12 | u16→i32 | [verified-NEW] move twin `w@(0xf6d46+…) := ((angle+4)&0xFF)>>3` — the EXW formula EXACT |
| variant | +0x18 | u16→i32 | [verified-NEW] spawn-init `w@(0xf6d4c+…) := RandA()&3` (FUN_00012216 = RandA) — EXW §3 row EXACT |
| probe_z[8] | +0x1A..+0x29 | u16×8→i32 | [verified-NEW] spawn-init 8-word seed loop `w@(0xf6d4e+2k+…) := z` (bound 0x10 B); probe writer FUN_0001e440 touches each of 0xf6d4e..0xf6d5a; `d@+0x28>>0x10` = the kind read (the 7b.2 climb-compare form) |
| kind | +0x2A | u16→i32 | [verified-NEW] spawn-init SP `w@(0xf6d5e+…) := [0x1075c0]` (player type) / MP `:= i`; gates the alarm trip + booster SFX (always via `d@+0x28>>0x10`) — EXW §3 row EXACT |
| hit_flash | +0x2E | u16→i32 | [verified-NEW] damage applier FUN_0001ef61 `w@(0xf6d62+…) += 1` FIRST on every unshielded hit; portrait pass FUN_000180a1 clamps 5 + decrements while alive ∧ hp≥1 — EXW 7g.1/7g.8 EXACT |
| armor | +0x30 | i16 | [verified-NEW] phase-1 pad pass (FUN_0001c7dc): off-pad `w@(0xf6d64+…) −= 10` clamp 0 behind the type-DB fade byte; pad-charge FUN_00020dea `+= charge` clamp 3000, bar-full chime at 2500 — EXW 7g.3/7f EXACT (read form `d@+0x2E>>0x10`) |
| alarm | +0x34 | u16→i32 | [verified-NEW] damage applier trip `w@(0xf6d68+…) := 100`; phase-0 decay `−= 1` (FUN_0001c7dc) — EXW 7g.1/7g.2 EXACT |
| stop_dist | +0x74 | i32 | [verified] consumer auto-arm `target@+0x74 := 1000000 (0xf4240)` @0xf6da8 (§5c) + the beacon-proximity armer's second `:= &DAT_000f4240` store + the arrive gate `stop < dist ∨ dist < 0x1400` |
| hp | +0x78 | i32 | [verified] respawn base `[EAX+0xf6dac] = 0x1388` (§4) + damage `d@(0xf6dac+…) −= dmg`, ceiling `battery·100+5000` |
| alive | +0x7C | i32→u8 (≠0) | [verified] armer alive loop over `[0xf6db0+i]` presence (§4) |
| drop_countdown | +0x80 | i32 | [verified-CORRECTED, D88] the phase-4/5 gate `phase < 4 ∨ phase·32 < d@(0xf6db4+…)` + per-tick decrement (FUN_0001c7dc; `local_2c = phase<<5`) + death clear (FUN_0001ef61) — the ENGINE field's semantics EXACT. The W7 row bound this canonical field to +0x2C (the pod timer) — WRONG: +0x2C is the mission-start pod-DESCENT timer (spawn stagger `w@(0xf6d60+…)`, freeze gate `w@+0x2C ≠ 0` skips the whole tick), which the engine does NOT model as a canonical field (pod-stagger modeling stays a backlog note). Both EXW +0x2C and +0x80 carry the same split (SIM §3) |
| shield | +0x88 | i32 | [verified-NEW] damage absorb `d@(0xf6dbc+…) −= dmg` clamp 0; phase-0 decay 2; conversions := 0x20 (state-3); booster forces 10000, expiry 150 — EXW 7g.1/7g.2 EXACT |
| shield_charges | +0x8C | i32 | [verified-NEW] spawn stat-0x2A copy `d@(0xf6dc0+…) := stat>>0x10` (FUN_0001d9cd switch); damage gate `charges==0 ∨ shield≠0` → spend one charge for a 0x20 shield — EXW 7g.1 EXACT |
| battery | +0x94 | i32 | [verified-NEW] spawn stat-0x2B copy `d@(0xf6dc8+…) := stat>>0x10`; hp ceiling `battery·100+5000` in the damage clamp — EXW 7f.8 EXACT |
| armor_pool | +0x98 | i32 | [verified-NEW] spawn stat-0x2C copy `d@(0xf6dcc+…) := (stat>>0x10)·200`; pad-charge gate `pool==0 → direct armor charge` — EXW 7g.4 EXACT |
| death_flag | +0x9C | u16→i32 | [verified-NEW] `d@(0xf6dd0+…) := 1` on BOTH death subsets (FUN_0001ef61 SP @~0x1f0e3 + MP); READER pinned: FUN_0005961c = the SP all-dead mission-fail sweep (`any +0x9C==0 → skip`) — closes the EXW §7g.6 "readers not census'd" note |
| shield_boost | +0xA0 | i32 | [verified-NEW] countdown `d@(0xf6dd4+…) −= 1` forces `shield := 10000` per frame; expiry `< 1` → 0 + `shield := 150 (0x96)`; countdown==200/0xC4 SFX pair — EXW 7g.2/7h.2 (arms 200) EXACT |
| alarm_ctr | +0xA4 | i32 | [verified-NEW] damage `d@(0xf6dd8+…) += 3` while alarm==0; `> 100 ∧ kind==player-type` → alarm := 100, ctr := 0 — EXW 7g.1 EXACT. NOTE: EXD ALSO decrements it 1/phase-0-pass when nonzero (FUN_0001c7dc @~0x1c886) — an EXW-side evidence gap (7g.1 lists no decay); flagged as divergence-seed candidate until a live S1 diff or an EXW re-read |

**Coverage gaps AFTER this unit (canonical fields with NO pinned EXD
offset — the normalizer leaves them OUT of coverage, they are reported
as STRUCTURAL coverage findings, never zero-filled-then-compared, never
guessed):** NONE — target_present/target_x/target_y were the last 3 of
the 34 canonical leaf fields, and they are RECORD-EXTERNAL but now
SOURCED (W7-followup2, D90): the O1/O2 plan dumps the §5 move-target
span as one fixed 0x60-B row at 0xf75ec, and the differ SPLICES the
per-robot trio into the robot-bank row (x[i] u32 @+4i, y[i] u32
@+0x30+4i, present = x ≠ −1, robots bounded by the same frame's
robot-bank count; absent canonicalizes to present 0 + tx/ty 0, matching
the E §6a row; the E row itself stays an E-only row — the O1 side
carries no standalone move-target row after the splice). UNIT CHECK
(Q5): both sides are Q5 — the EXD writers are `tile<<5` (spawn −1
fill, order consumer, beacon auto-order, arrive-clear; indexing loop
`+= 4` per record bounded by the CAP cell 0x11950c ≤ 12) and the
engine's `Robot::target` is `dest_tile·Q5_PER_TILE(0x20)` (bedlam-core
mission.rs order consumption) — raw i32 comparison, no shift.

Non-canonical record cells decoded in passing (context rows, no
canonical field — the engine does not model them): +0x14 viewport
frame-base word; +0x16 deploy countdown (`:= 0xFFFF` spent, phase-0
decrement, gates the +0x40 overlay); +0x32 EXD-side phase-0 countdown
word (0xf6d66, no EXW §3 row — evidence gap, not gameplay-relevant);
+0x36..+0x6C the 7×8-B order-stat groups + order-gate cooldown drains
(FUN_00020fd5); +0x6E order bits; +0x70 deploy delay (vs table
0x8105c[difficulty]); +0x84 pathing scratch (108 refs, heavy);
+0x90 dying countdown (states 5/6, expiry → alive:=0 / state:=0).

**Seed #1 EXW-front discrepancy [OPEN, W11]:** RE-EXW-SIM §3 documents
EXW pos_x@+0x00/pos_y@+0x04 (evidence column: 0x4c69ec z, 0x4c69f0
state) while seed #1 above says EXW carries x@+4/y@+8. The EXD side
(x@0/y@4/z@8/state@0xC) is independently verified and unaffected. The
O2/EXW normalizer uses the SIM §3 table (the per-field evidence one);
the first live EXW capture (W11) arbitrates the conflict — a one-field
map flip either way.

**Non-robot row forms the O1 normalizer consumes** (all from the
dbx-plan-emitted capture rows; identity unless noted):
- T0 scalars (score/money/difficulty/zone/mission/mode/linear-mission-m):
  4-B u32 identity; rng-state-a/b: u32 → canonical u64 zero-extend
  (channel-native state word, §6a — T3 class, never bit-compared);
  frame-counter: u32 identity (T2 class: the O1 counter never resets —
  14 INC sites incl. menus — so a live O1 value ≠ E's 0..N by
  construction; the ALIGNMENT key is the record `frame_no`, not this
  row).
- selection-triple: the 4-B selected-idx alias (D83 form) — identity.
- order-target: 12-B i32×3 span — identity.
- beacon-family: 10-B span of five u16 cells {flag, timer, tile×3} →
  canonical {flag u32, timer u32, tile i32×3} (zero-extend each; the
  EXW cells are u16-spaced identically). Cell-order note: the tile
  trio maps in registry-listed order; if the original writes {z,x,y}
  a live diff flags it as a finding (layout differences are findings,
  not false negatives).
- spread-claims: 24-B u16×12 — identity.
- per-player-selected: 0x30 = 4×0xC {x>>8, y>>8, z} — identity.
- typedb-fade-byte / armor-pad-reads: raw w·h grid → canonical
  u32 len + bytes with the §6a equivalence "len 0 ≡ all-zero w·h"
  applied (an all-zero grid canonicalizes to len 0 — the ZONEA corpus
  shape until a death materializes the bank).
- static-map-wh: 48-B span → canonical {w u32, h u32} with
  w = u32@span+0x2C (cell 0x1074b8), h = u32@span+0x00 (0x10748c).
- every other TS/T2/T3/T4/TI row: no E-side emitter (§6a E-gaps) and/
  or no EXD alias — comparison is impossible; the differ reports
  coverage asymmetry, never silence.
