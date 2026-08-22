# RE-EXD-MAP — BEDLAM.EXD import + the EXW→EXD address map (P4.2/W1)

Status: COMPLETE for the bounded scope (2026-08-22, worker d06341cf,
claim 1): EXD imported, frame-tail/S0 trigger pinned, T0/T1 rows mapped
with dual anchors + the static-after-load table aliases. W5-FOLLOWUP
(2026-08-22, worker ef11271c, claim 2) CLOSED four of the six explicit
gaps: difficulty, order-target triple, keystore, command ring + count
(§5c). Remaining gaps (unmapped, schema-visible): SFX master gate,
blink-cursor, no-extract latch, selection cursor/squad cells — each
carries its anchor method for the follow-up unit. T2-T4 aliasing
stays later per the W1 ticket. Purpose: the DESIGN-DIFFHARNESS.md W1
deliverable — the `exd_addr` fills for the harness watch rows.

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
  twin), FUN_0001d9cd = the spawn initializer (EXW 0040cca0 twin),
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
| mode | 0x4edb88 | **0x1075d8** | SP/MP branch `DAT_001075d8 == 0` in the spawn twin + mission-loop `== 2` MP gates + new-game `MOV [0x1075d8],1` | [verified] |
| linear mission m | 0x46ae8c | **0x119610** (SAME cell as mission) | the stagger consumer reads 0x119610 — EXD uses ONE scalar where EXW has two (see divergence seed #3) | [verified] |
| SFX master gate | 0x4ede58 | TODO (gap) | gate lives inside the SFX dispatch twin (DEADMAN1 str ref → FUN_0004c121 is the bank loader); pin when the W2 registry needs it | |

## 5. T1 — the P4 slice (EXW → EXD)

| watch | EXW addr | EXD addr | anchors used | tag |
|---|---|---|---|---|
| robot bank | 0x4c69e4, count 0x46ccbc | **base 0xf6d34, count 0x11958c** (stride 0xA8 same; cap cell 0x11950c) | armer alive loop `[0x11958c]×0xA8` over [0xf6db0+i]=presence@+0x7C; state w@+0xC via [0xf6d40+i·0xA8]:=3; hp@+0x78 via `[EAX+0xf6dac]=0x1388` (5000 respawn base); **pod timer w@+0x2C** via the stagger store `[0xf6d60+i·0xA8] = 1+k·(2000−m·1000/27)` (formula EXACT) | [verified] |
| selection triple | 0x46cbd4/dc/d8 | **selected idx 0x11954c** (cursor/squad cells TODO) | mission-loop auto-switch: `i != DAT_0011954c` skip-current check + `DAT_0011954c := i` on switch (state write 0x119498 := 3) | [verified] |
| blink-cursor selector | 0x4dc5d0 | TODO (gap) | 7j.7 producer twin not located this unit; anchor via the effect-row family when W2 needs it | |
| per-player selected anchor | 0x4c71c4 | **0x971a4** | spawn-tail seed loop `do {[0x971a4+i]=x>>8; [0x971a8+i]=y>>8; [0x971ac+i]=z} ×4 (0x30/0xC)` — EXW 4×0xC {x>>8,y>>8,z} EXACT | [verified] |
| order target xyz | 0x4dd484/88/8c | **0x10e0a4/0x10e0a8/0x10e0ac** | consumer twin FUN_00019ee9 bit1-ORDER branch writes all three @0x1a0af/0x1a0bc/0x1a0d8 (words@+7/+9/+0xB of the record, EXW EXACT) + the click-order twin FUN_00021112 writes the triple from the pick (FUN_0002a271, EXW FUN_00419943): ground branch iso combine, rect branch reads rec@0x9df30-base, the `&0x2000` structure flag EXACT; loop position = EXW MissionShell trio EXACT (FUN_00021112 → FUN_0005b066(1) builder → FUN_00019ee9 consumer = EXW FUN_00410644 → FUN_00449c94 → FUN_00409138) | [verified] |
| per-robot move-target words | 0x46cc30/0x46cc60 | **0xf75ec / 0xf761c** | spawn −1-init stores at both + the 0x30 gap twin (EXW 0x46cc60−0x46cc30 = 0x30 = EXD 0xf761c−0xf75ec) + all writers in the order monolith FUN_000448e7 (47 refs). EXTENT PINNED (W7-followup): per-robot u32 ×2 indexed by ABSOLUTE robot id over the CAP cell 0x11950c (tick loop `+= 4` per record; 0x11950c := 0x11958c SP / 0x119588 MP, ≤ 12); the fixed 0x60-B span at 0xf75ec covers x[12]+y[12] deterministically — the dbx-plan row can now be filled | [verified] |
| extraction beacon family | 0x4eabb0/b2/b4/b6/b8 | **0x119628/0x11962a/0x11962c/0x11962e/0x119630** | armer FUN_0003570e full decode (guard/timer 0x197/tile trio) + mission-loop countdown `(short)DAT_0011962a −−` with the digit draws and the all-state-3 → FUN_00030899 completion sweep | [verified] |
| spread claims | 0x4eabba | **0x119632** | picker FUN_0003581b full decode: first-free u16 scan `[0x119632+i]`, bound = cap cell 0x11950c, marks 1, the 12-offset switch around beacon x/y — EXW FUN_004248c8 EXACT | [verified] |
| no-extract latch | 0x46aed4 | TODO (gap) | animator twin not decoded this unit (FUN_0001f8c1 turned out to be the debrief/payout fn); anchor via the pod-ring animator when W2 needs it | |
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
| command count 0x46cbe0 | **0x119588** | consumer loop bound `if ([0x119588] <= rec) → cooldown-tail/return` + the ring-modulo read `(_DAT_00107688 + 1) % [0x119588]` + `_DAT_00107658 += [0x119588]` (builder family FUN_0005b066 ×8 reads) + 6 WRITE sites in FUN_0004c80c (the net/input pump — also the keystore[ESC] reader; mission-start resets @0x4ccd8/0x4cd6a) | [verified] |
| order-active flag 0x4dc6bc | **0x10e140** | consumer bit1 branch `:= 1` (EXW `_DAT_004dc6bc := 1`); also cleared 0 at MissionShell boot (probe7 line 81) | [verified] |
| order spot staging (EXW: none separate) | **0x10e15c** | consumer stores record word@+3 → 0x10e15c; builder reads it back as the spot short — EXD keeps a staging cell EXW inlines | [verified] |
| command flags staging (EXW: none separate) | **0x11a51a** | builder reads flags byte from it; MissionShell ORs 4 into it on the MP path (DAT_001075d8 != 0, probe7 line 480) | [verified] |
| projectile bank 0x4c71f4 (400×0x36) | **0x980d4** (×0x36) | consumer artillery case writes the record: type w@+0 (0x980d4), owner d@+2 (0x980d6), ttl d@+0xA (0x980de), xyz d@+0x12/+0x16/+0x1A (0x980e6/ea/ee), vxyz d@+0x1E/+0x22/+0x26 (0x980f2/f6/fa), +0x2A := 4, ttl2 `0x900 − (RandA&0x2ff)` @+0x2E — EXW §7j.17 field map EXACT (T2 registry fill lands with the T2 unit) | [verified] |

Order-dispatcher reader twins over the triple (EXW FUN_0040af98 ×3,
FUN_0040a56f/0xa7a1/0xace8/0xb615/0xa9ff ×2 each → EXD FUN_0001b369,
FUN_0001b598, FUN_0001bae0, FUN_0001bd8f, FUN_0001c3fb, FUN_0001b7f7;
the 0xf75ec/0xf761c move-target writers stay in the FUN_000448e7 UI
monolith as mapped in §5).

Divergence seeds found this unit → §7 items 6-7 (attack-break
randomness source; EXD-only staging cells).

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
guessed):** target_present/target_x/target_y ONLY — 3 of the 34
canonical leaf fields. Their SOURCE is pinned (§5 move-target arrays
0xf75ec/0xf761c: per-robot u32 x/y Q5 by ABSOLUTE robot index, −1 =
none, writers = spawn-init −1 fill + order consumer + the beacon
auto-order `:= tile<<5`, and the tick's arrive-clear; indexing loop
`local_48 += 4` bounded by the CAP cell 0x11950c — the extent formula
is therefore `2 × cap × 4 B ≤ 0x60`, and the fixed 0x60-B span at
0xf75ec covers x[12]+y[12] exactly since 0xf761c−0xf75ec = 0x30), but
the O1 PLAN row is still deferred (dbx-plan must emit the row; the O1
normalizer then parses the span). Named follow-up: fill the
move-target-words plan row + splice target_present/tx/ty into the O1
robot-bank fields (coverage 3 → 0 per robot).

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
