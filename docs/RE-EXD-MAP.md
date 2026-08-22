# RE-EXD-MAP — BEDLAM.EXD import + the EXW→EXD address map (P4.2/W1)

Status: COMPLETE for the bounded scope (2026-08-22, worker d06341cf,
claim 1): EXD imported, frame-tail/S0 trigger pinned, T0/T1 rows mapped
with dual anchors + the static-after-load table aliases. Explicit gaps
(unmapped, schema-visible): difficulty, SFX master gate, blink-cursor,
order-target triple, no-extract latch, selection cursor/squad cells —
each carries its anchor method for the follow-up unit. T2-T4 aliasing
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
- **robots()/tick monolith = FUN_0001c7dc** (14,644 B; contains the
  trap-pair resolver reads + calls FUN_000448e7, the 28,451-B draw/UI
  monolith whose 0xf75ec move-target writes are the EXW
  order-dispatcher family; 87× FUN_000332f8 + 38× FUN_00033d94 draws).

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
| difficulty | 0x46cbf8 | TODO (gap) | not in the GameInit boot plants; cycled at the name-entry/save path — anchor via the (d+1)%3 site or the critter table reads (7j.17 DAT_00454edc twin) in a later unit | |
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
| order target xyz | 0x4dd484/88/8c | TODO (gap) | click-order path lives in the FUN_000448e7 UI monolith; anchor via the command-record consumer or FUN_00410644 twin's store when W2 needs it | |
| per-robot move-target words | 0x46cc30/0x46cc60 | **0xf75ec / 0xf761c** | spawn −1-init stores at both + the 0x30 gap twin (EXW 0x46cc60−0x46cc30 = 0x30 = EXD 0xf761c−0xf75ec) + all writers in the order monolith FUN_000448e7 (47 refs) | [verified] |
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

**W5 input-twin note (2026-08-22, probe ghidra-project/
exd-input-probe.txt via EXDInputProbe.java):** the EXD KEYSTORE
alias (EXW g_keystore 0x4edc44, 256 B scan-indexed) is STILL a
gap. The candidate suggested by the MissionShell pause spin —
FUN_0002ec12 — is NOT a keystore reader: disassembly shows only
`MOV [0x1075b4],0; CMP EAX,[0x1075b4]; JG` = a WAIT-for-latch
spin on the P-pause latch 0x1075b4 (it never reads a 256-byte
array). The keystore twin needs a proper reader census (start:
the any-key scan family twin, EXW FUN_0041f9d1 scanning codes
1..0xFE, and the InputReset memset-256 twin EXW 0x4207b5). Until
then KEYSTATE/ORDER/PAD/COMMAND/BOOT injection steps cannot
compile to O1 addresses (dbx-plan errors, naming the gap) — the
engine side consumes the same steps directly (W6).

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
