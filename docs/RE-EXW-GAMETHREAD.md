# RE: BEDLAM.EXW - the game worker thread (0044dea0) and GameMain (0041c050)

Provenance: Ghidra headless `-process BEDLAM.EXW -noanalysis` + postScript
`tools/ghidra-scripts/ExwGameThread.java` against the single BedlamWatcom
import (x86:LE:32:default + openwatcomcpp cspec; run exit 0, Save succeeded,
2026-08-17 04:4x). Raw dump: `ghidra-project/exw-gamethread.txt`, log:
`ghidra-project/process-exw-gamethread.log` (both gitignored; dumps live in
ghidra-project/ root). Names applied and persisted this run:
**GameThread@0044dea0** (region 0044dea0..0044dedb, 59 bytes - was not a
function before), **GoFlagSet@0044d9b4** (was FUN_0044d9b4); naming pass added
**GameMain@0041c050** (was FUN_0041c050).

Tags: [verified] read in decompile/listing; [inferred] strong deduction;
[hypothesis] plausible, needs confirmation. Addresses are EXW VAs (base 00400000).

## Headline: GameThread is a 59-byte trampoline; the game loop is GameMain@0041c050

The worker thread body is NOT itself the sim/render loop. It calls one big
function - FUN_0041c050, now **GameMain** - which is the real game shell:
global init, RNG seed planting, language select, menus, intro movies, the
zone/level progression state machine, and the per-level advance loop. When
GameMain returns, the thread stores an exit code, clears its thread id and
asks the main window to destroy itself [all verified, see below].

## GameThread @0044dea0 [verified - full listing, 59 bytes]

```c
void GameThread(void)                    // 0044dea0: PUSH EBX/ECX/EDX (Watcom)
{
  short r = GameMain();                  // 0044dea3: CALL 0041c050
  if (r != 0) {                          // 0044dea8: MOVSX EDX,AX; TEST
    // 0044deb2: ADD AH,0x40 ('@'); 0044deb5: MOV [0x004ef676],AX
    word@004ef676 = (((r >> 8) + 0x40) << 8) | (r & 0xff);
  }
  dword@004ef694 = 0xffffffff;           // 0044deca: MOV EDX,-1; ..: MOV [004ef694],EDX
  SendMessageA(dword@004ef68c, 2, 0, 0); // 0044dedb-ish: CALL CS:[0x4f025c]
}                                        // POP EDX/ECX/EBX; RET
```

- **Correction to RE-EXW-TICK.md**: the instruction at 0044deca is
  `MOV EDX,0xffffffff` feeding `MOV [004ef694],EDX` - it does **not** write
  go flag 004ef674 [verified listing]. The earlier "0044deca writes 004ef674"
  claim was a misread of 004ef694 as 004ef674.
- 004ef676 = encoded game exit code (`(hi+0x40)<<8 | lo`, classic
  letter+digit style) [verified write; reader not yet found - open].
- 004ef68c = main window HWND; msg 2 = WM_DESTROY. After the game finishes,
  the worker thread tells the GUI thread to tear down, which ends the message
  pump [verified call, inferred effect].
- 004ef694 = game thread id; -1 marks "no longer running" (CreateThread wrote
  the real id there; GameThreadStart's error path tests it) [verified].

### Go flag 004ef674 - complete writer/reader set [verified xrefs]

| site | function | action |
|---|---|---|
| 0044d9cc | GameThreadStart | WRITE (= 0, reset before spawn) |
| 0044d9b4 | GoFlagSet | WRITE (word 1 - `MOV word ptr [0x004ef674],0x1`) |
| 0044da66 | TimerInit | READ (spin until nonzero) |

Nothing inside 0044dea0..0044dfec writes it; neither does GameMain,
TimerCallback, or TickWorker (checked in their decompiles) [verified].
**RESOLVED 2026-08-17 (tick2 run, ghidra-project/exw-tick2.txt)**: GoFlagSet
is called by FUN_0041e19d [verified xref + decompile], which GameMain calls
right after LoadFile(LANGUAGE.*) at boot. FUN_0041e19d = the release-the-timer
routine: zeroes divider 004edbc8 + counters 004edb54/58/5c, applies base
palette (SetPaletteIndex 0x5d), resets scroll copies 004eddf8..004ede04 = -1,
sets 004dc6c8=1 / 004dc6e4=0, arms 004ede10/004ede14 from its EDX arg (fade),
then GoFlagSet(). The TimerInit spin therefore ends during GameMain init, as
inferred.

## GoFlagSet @0044d9b4 [verified - full listing, 10 bytes]

```asm
MOV word ptr [0x004ef674],0x1
RET
```

Note it is a WORD write of 1 (GameThreadStart's reset at 0044d9cc matches the
width). No local caller found yet.

## Pacing verdict: NO Sleep / timeGetTime anywhere on the game thread - 20fps claim REFUTED at this depth

Evidence:

1. GameThread's listing (59 bytes, above) contains no wait of any kind
   [verified].
2. GameMain@0041c050 and its 44 direct callees (PSEUDOCALLS list in the dump)
   contain no Sleep, no timeGetTime, no WaitForSingleObject, no delay loop
   built from those [verified - full decompile of GameMain + callee name
   scan; only wait-like import seen on this thread is SendMessageA, which is
   a send, not a sleep].
3. The only rate mechanism visible from the game thread first hop is the
   timer-service chain (RE-EXW-TICK.md): 100Hz timeSetEvent -> TickWorker ->
   counter@004edbc8, `(ctr & 1) && dword@004ede10` -> FUN_00425901.
   **CORRECTED 2026-08-17 (tick2 run)**: FUN_00425901 = FadeStep and
   004ede10 = **palette-fade step countdown**, NOT a frame gate (see D15 and
   the tick doc). The 50Hz figure is the palette-fade rate only.
4. The in-game advance loop inside GameMain (below) calls FUN_0043d00b, which
   READS 004ede10 (xref at 0043d5e6) [verified xref]. CORRECTED: 004ede10 is
   the fade countdown, so this read is a fade-status check [inferred]; it is
   NOT evidence that the gameplay step runs at 50Hz.

**Verdict (as of the gamethread run, PARTIALLY SUPERSEDED by D15)**: the
8street "20fps sim/render" claim was refuted at this level - no 50ms sleep and
no /5 divider exists on the game thread. The further claim "effective
sim/render rate = 50Hz max via gate 004ede10" is **WRONG**: the tick2 run
showed 004ede10 is the palette-fade countdown and FUN_00425901 is FadeStep
(see D15 + RE-EXW-TICK.md). Verified rates today: 100Hz service tick, 50Hz
palette fade while fading, 12.5Hz palette cycle. **The sim/render pacing
mechanism is UNKNOWN** - reopen via FUN_0043d00b / FUN_00440e45 bodies (second
hop) and the divider consumers (004edbc8 etc., e.g. FUN_00448ef1). Parity
budget: no committed logic rate until then (D13 demoted by D15).

## GameMain @0041c050 - annotated structure [verified unless tagged]

(One of the largest functions in EXW; decompile cleaned, key addresses kept.
Watcom register-arg artifacts (unaff_EDX/extraout_EDX) elided.)

```c
void GameMain(void)                       // returns 16-bit code in AX
{
  // ---- global init block ----
  _DAT_004eddc4 = 0x140; _DAT_004eddc8 = 0xf0;   // 0041c19x: scroll x/y = (320,240) = screen center
  _DAT_004ede10 = 0;                              // 0041c193: clear fade countdown at boot (CORRECTED: not a frame gate)
  _DAT_004ede48 = 0x1e240;                        // = 123456  RNG seed A
  _DAT_004ede4c = 0x39447;                        // = 234567  RNG seed B
  ... (~30 more global zero/one inits: 004edeb2=1, 004edec0=1, 004ede4c, ...)
  uVar10 = FUN_0044b3f8();                        // first big init (ret stored 8-byte)
  FUN_004249ca(...); FUN_00402965();              // FUN_00402965 recurs everywhere [hyp: yield/commit]
  ...
  _DAT_004edb40 = _DAT_004ee9e8;                  // copy DD surface PITCH into game-side var
  FUN_0041f9b5();
  FUN_0041d4e9();
  // language select on _DAT_004eba1c: 1 GER, 2 SPA, 3 FRE, 4 ITL, 5 DCH, else ENG
  //   (strings s_LANGUAGE_*_00457a70..00457ab1) -> FUN_0041cc7f(lang, DAT_0046cbb4)
  pcStack_34 = FUN_0041db89(0x4b000);             // alloc 307200 = 640*480 [inferred: framebuffer]
  _DAT_004edbf8[5..7] = 0x3f;                     // 0x302-byte scratch buf (palette/DAC commit)
  FUN_004258d0(_DAT_004edbf8);
  if (_DAT_004eb93c) FUN_0041cf29();
  // string tables: FUN_00424679("MENU ITEMS"), FUN_004245e6, FUN_0042463d loop;
  //                 FUN_00424679("WARNINGS"),  same pattern (loads, counts entries)
  DAT_0046cca4 = 1;  FUN_0043a144();              // suppress flag forced during load, restored after
  if (first run / iVar4 == 0) {                   // intro path (boot attract;
                                                   // RE-verified 2026-08-20 - see the
                                                   // "Boot attract arm RE" section:
                                                   // GTLOG then LOGO, full screen,
                                                   // unskippable, one pass each)

    FUN_0042582a(0x400);
    FUN_0044567c(DAT_0046ae64 ? "GAMEGFX\\GTLOG_US.SMK" : "GAMEGFX\\GTLOG_UK.SMK", 0);
    FUN_0044567c(DAT_0046ae64 ? "GAMEGFX\\LOGO_US.SMK"   : "GAMEGFX\\LOGO_UK.SMK", 0);
    FUN_00425851();
  }
  DAT_0046ae78 = 1;                               // "in movies/menu" flag
LAB_0041c3d6:                                     // outer restart point
  FUN_0043a48d(); FUN_0044745e();
  _DAT_004eae54 = 0;  _DAT_004edd8c? stays; _DAT_004edd88 = 1;
  FUN_0043a5fc();
  DAT_0046ae74 = 0;                               // clear quit flag
LAB_0041c454:                                     // ===== EPISODE LOOP =====
  while ((episode < 7) && (DAT_0046ae74 == 0)) {  // iStack_20 = episode counter
    // ---- zone-complete scan: 17 entries x 0xC bytes at 004decb2 ----
    // entry { u32 state; ..; u32 done_flag }; count = (state==1||state==7) ? 4 : 0
    // count entries where state == _DAT_004edd8c && done_flag != 0
    // if count == 5 -> zone complete -> LAB_0041c69e   [inferred: 5 levels/zone]
    // ---- level wait/advance loop ----
    do {
      while (ret == 0) {
        if (_DAT_004edd8c == 7) { _DAT_004edd88 = 1; ret = 1; }
        else {
          DAT_0046ae80 = _DAT_004edbd4;           // save flag, force 1 during call
          _DAT_004edbd4 = 1;
          FUN_0043e7d4(old);                      // [hyp: menu/modal runner]
          _DAT_004edbd4 = DAT_0046ae80;           // restore
          if (DAT_0046ae74) break;                // quit
          ret = (ret == 1) ? 1 : FUN_0043d00b(...);   // CORRECTED D37: the BRIEFING screen
                                                     // (drop intro + zone backdrop + mission select -
                                                     // see the D37 section); its 004ede10 read (0043d5e6)
                                                     // is the exit while-loop fade-done condition
          if (ret == 2) DAT_0046ae74 = 1;         // 2 = abort -> quit
        }
      }
      _DAT_004eb934 = _DAT_004dd40c;              // checkpoint level context
      _DAT_004eb938 = DAT_0046ae70;
      DAT_0046ae74 = FUN_00440e45(DAT_0046ae70, ...);  // [RESOLVED 2026-08-23 §7j.45: the SHOP screen; ret 1 = abort/quit]
      if (DAT_0046ae74) goto zone_done;
      DAT_0046ae8c = clamp((_DAT_004edd8c - 2)*5 + _DAT_004edd88 - 1, 1, 26);
                                                   // linear mission number, 1..26
      _DAT_0046af04 = 1;
      if (_DAT_004edd8c == 7) { iVar4 = 2; DAT_0046cbf8 = 2; }  // endgame: force variant
      uVar10 = FUN_0044771c(iVar4, _DAT_004ddb2c);   // [hyp: music/ambience select]
      if (hi(uVar10) != _DAT_004ddb2c) FUN_0042540c(uVar10);
      if (_DAT_004edbe8 && _DAT_004edbec) FUN_0044dfec();  // region/mode-gated helper
      _DAT_004edb80 = 0;
      FUN_0041d714(0x5d);                          // SetPaletteIndex(0x5d) - see tick doc
      switch (lo(uVar10) - 1) {                    // end-of-level outcome
      case 0:  // advance: next level
        if (_DAT_004edd8c != 7) {
          FUN_0041ca2e(); FUN_004474ef(_DAT_004edd8c, _DAT_004edd88);
          _DAT_004eae54 = 1;
        }
        break;
      case 1:  // restart level: restore checkpoint, reset flags
        _DAT_004dd40c = _DAT_004eb934; DAT_0046ae70 = _DAT_004eb938;
        FUN_0041ca06(); DAT_0046ae74 = 0; _DAT_004edb50 = 0;
        goto outer_restart;
      case 2:  // quit game
        FUN_0041ca2e(); FUN_00447550(...); DAT_0046ae74 = 1;
        goto outer_restart;
      case 3:  FUN_0041ca2e(); break;             // [hyp: retry variant]
      default: goto loop_bottom;
      }
      FUN_0044425c();
    } while (...);
  }
LAB_0041c69e:                                     // ===== ZONE COMPLETE =====
  if (DAT_0046ae74 == 0) {
    DAT_0046af0c = DAT_0046af20;
    FUN_0041db89(86000); FUN_0041db89(0x302); FUN_0041db89(0x5000a);  // allocs
    FUN_0041cc7f("GAMEGFX\\FULLFONT.BIN", ...);
    FUN_0041cc7f("GAMEGFX\\FULLPAL.PAL", ...);
    if (_DAT_004edd8c == 7) {                     // ENDGAME
      FUN_0042582a(0x400); FUN_0044567c("GAMEGFX\\END.SMK", 0); FUN_00425851();
      ... refill _DAT_004edbf8 (0x3f tail) ...; FUN_004258d0(_DAT_004edbf8);
      FUN_0041c9f0(...);                          // [hyp: credits/roll]
    } else {                                      // zone transition
      FUN_0044567c("GAMEGFX\\ZONEDONE.SMK", 0);
      FUN_0041db89(310000); FUN_0041cc7f("GAMEGFX\\BETWEEN.BIN", ...);
      FUN_00401e39(0,1,0,0);
      FUN_0042597c(...);                          // [hyp: present/commit]
      FUN_0041db89(300000);
      FUN_0041cc7f("GAMEGFX\\LOAD_UK.BIN"/"LOAD_US.BIN" per DAT_0046ae64);
      FUN_0041cc7f("GAMEGFX\\LOADPAL.PAL"/"LOADPALU.PAL", _DAT_004edbf8);
      ... palette buffer fill 0x2a2..0x301 = 0x3f; FUN_004258d0(buf);
      // loading text, four draws [verified D35 - see the font-drawer
      // note below]: DAT_0046bc4c/0046bc7c/0046bfdc = table entries
      // 0x45/0x46/0x58 of the LANGUAGE string table at 0046af5c
      // (base + idx*0x30); the third is entry zone+0x51. EBX args
      // 0x96/0xb4/0xd2/0x104 = draw ROWS 150/180/210/260; 0x82 = the
      // glyph entry base (ECX arg), NOT a y coordinate (D34 recorded
      // the pair swapped - corrected D35).
      FUN_0043c87c(&DAT_0046bc4c, bank, 0x96, 0x82);
      FUN_0043c87c(&DAT_0046bc7c, bank, 0xb4, 0x82);
      FUN_0043c87c(&DAT_0046af5c + (zone+0x51)*0x30, bank, 0xd2, 0x82);
      if (zone == 6) FUN_0043c87c(&DAT_0046bfdc, bank, 0x104, 0x82);
      FUN_00425a03();
      // [verified D35] font ramp: 0x60 bytes (24 dwords + 0 tail)
      // copied from the FULLPAL.PAL load buffer +2 into DAC commit
      // buffer +0x2a2 = palette entries 224..=255 (the same range the
      // pre-text fill had forced to 0x3f), THEN FadeSetup arms.
      FUN_0041cbf0(_DAT_004edbf8, 10);            // FadeSetup(pal,10): arm 10-step 50Hz fade (004ede10 = steps left)
      episode++;                                  // iStack_20
      _DAT_004edd8c++;                            // zone++
    }
  }
  goto LAB_0041c454;                              // next episode
}
```

### Font drawer FUN_0043c87c (D35, 2026-08-20)

Fully decompiled + cross-checked against the corpus (ghidra-project/
exw-font-drawer.txt + exw-font-strings.txt + exw-menu-parse.txt;
reimplemented in engine/bedlam-game/src/font.rs, corpus pinned in
engine/bedlam-assets/tests/font_gate.rs):

- Signature (register args): EAX = string ptr, EDX = the FULLFONT.BIN
  bank, EBX = the draw ROW, ECX = the glyph entry base [verified].
- Two passes over the NUL-terminated string: measure then draw;
  x0 = 0x140 - total/2 (each line centers on screen x 320) [verified:
  MOV EAX,0x140 / SAR EBP,1 / SUB EAX,EBP].
- Per byte c: c >= 0x80 remaps through FUN_00410493 to (base char,
  accent id) first; k = char - 0x21; k < 0 (space/control) advances
  the pen 9 px; else glyph entry = ECX + k blits transparent
  (FUN_00401ca2 EDX=1 path: skip runs advance without writing) and
  the pen advances FUN_00402a12(entry) + 2 = slot width + 2 [verified].
- Hotspot (flags bit 1, all corpus glyphs flags 0x0003): u16@+2 adds
  to the dest ROW, u16@+4 to the dest COLUMN (FUN_00401ca2) - dy
  anchors the baseline (x-height letters dy=5, mid punctuation dy=10,
  low punctuation dy=15) [verified + corpus-pinned].
- Accent id 1..=4 (set by the FUN_00410493 stub) additionally blits
  the overlay glyph at ECX + 0x6b + id = entries 238..=241
  (diaeresis / acute / grave / circumflex) at the same pen position
  [verified].
- FUN_00410493 quirks kept verbatim: e-diaeresis and o-diaeresis
  stubs leave the prologue default base 0x2d (dash) under the
  diaeresis; k > 0x78 falls to dash + diaeresis [verified stub
  bodies, objdump 0x4104c0..0x410650].
- The strings come from the LANGUAGE.* [MENU_ITEMS] table (96
  entries x 0x30 at 0046af5c, filled by the boot language arm);
  the loading row uses entries 0x45 / 0x46 / zone+0x51 / 0x58 [verified].
- FULLFONT.BIN = 390-entry bank, 333 decodable RLE16|hotspot glyphs +
  57 empty slots; drawer glyphs = entries 130..=241 (chars 0x21..=0x81
  + overlays); ASCII glyph pixels are exactly {0} U {233..=244},
  inside the FULLPAL ramp entries 224..=255 [corpus-pinned].

### What this settles

- **Main state machine strides** [verified structure]:
  - language switch on 004eba1c, stride 1 (6 cases via string table);
  - zone/level decode `(zone@004edd8c - 2)*5 + level@004edd88 - 1` -> linear
    mission 1..26 (clamped) - **5 levels per zone, 7 zones** (zone 7 = endgame
    END.SMK path), consistent with the 17x12 completion table at 004decb2
    (count of 5 done-flags = zone complete);
  - end-of-level outcome switch (result of FUN_0044771c): cases 0..3 =
    advance / restart-from-checkpoint / quit / retry-variant [inferred
    semantics].
- **RNG chain entry** [verified writes, inferred role]: GameMain plants
  004ede48 = 123456 (0x1E240) and 004ede4c = 234567 (0x39447) at boot -
  classic seed pair; the LCG itself (multiplier constants) is not in GameMain,
  its function is still to be found [open].
- **Present/flip path**: no explicit flip/Blt call in GameMain's first hop;
  commit helpers are FUN_004258d0 (palette/DAC buffer apply), FUN_0042597c /
  FUN_00425a03 (present/commit [hyp]). Surface pitch 004ee9e8 is copied into
  game-side 004edb40 at boot. The per-frame blit most likely lives in
  second-hop FUN_0043d00b (gate consumer) [hypothesis].
- **Region variants** [verified]: DAT_0046ae64 selects US vs UK assets
  (GTLOG/LOGO/LOAD, LOADPAL vs LOADPALU) - explains the paired files in
  game-data/GAMEGFX.

## Globals added to the map this pass

| VA | meaning | tag |
|---|---|---|
| 004ef676 | encoded game exit code (hi+0x40, lo) written by GameThread | verified |
| 004ef68c | main window HWND (WM_DESTROY target) | verified use |
| 004ef694 | game thread id; -1 = dead/never-started | verified |
| 004ef674 | go flag: reset by GameThreadStart, set by GoFlagSet (word 1), TimerInit spins on it; NOT written at 0044deca | verified |
| 004ede48 / 004ede4c | RNG seed pair 123456 / 234567 planted in GameMain | verified writes / inferred role |
| 004edd8c | zone/stage index 1..7 (7 = endgame); ++ on zone completion | verified ops / inferred |
| 004edd88 | level-in-zone 1..5; =1 entering endgame | verified ops / inferred |
| 0046ae8c | linear mission number clamp((zone-2)*5 + level - 1, 1, 26) | verified arithmetic |
| 0046ae74 | quit/abort flag (FUN_0043d00b ret 2, FUN_00440e45 ret, outcome case 2) | verified use |
| 0046ae64 | region flag: 0 = UK, nonzero = US asset set | verified via string selects |
| 004dd40c / 0046ae70 | level-context pair, checkpointed to 004eb934/004eb938, restored on restart | verified ops / inferred |
| 004eba1c | language id (1 GER..5 DCH, 0/other ENG) | verified switch |
| 004decb2 | 17-entry x 12-byte completion table {u32 state; ..; u32 done_flag} | verified structure |
| 004edbf8 | 0x302-byte palette/DAC commit buffer (0x3f fills, FUN_004258d0 target) | verified sizes / hyp role |
| 004edbd4 | modal/menu flag, forced 1 around FUN_0043e7d4 | verified ops / hyp |
| 004edb40 | game-side copy of DD surface pitch (004ee9e8) | verified |
| 0046ae78 | "in movies/menu" flag (1 during intro, cleared at outer restart) | verified ops / hyp |
| 0046cca4 | load-suppress flag (forced 1 around loads/screenshots) | verified ops / hyp |
| 004eae54 | end-of-level outcome latched flag (0 loop top, 1 after advance) | verified ops / hyp |
| 004ddb2c | current music/ambience id, compared against FUN_0044771c result | verified use / hyp |
| 004edbe8 / 004edbec | mode gates: pair conditions FUN_0044e06c and FUN_0044dfec | verified use |
| 0046af5c | LANGUAGE [MENU_ITEMS] string table base: 96 entries x 0x30, filled at boot from LANGUAGE.<lang> | verified D35 |
| 0046bc4c / 0046bc7c / 0046bfdc | table slots 0x45 / 0x46 / 0x58 (= base + idx*0x30) - loading-text draws 1/2/4 | verified D35 |
| 0046ccd0 | accent id set by FUN_00410493 (default 1 = diaeresis); drawer blits overlay entry base+0x6b+id | verified D35 |

### New xref leads (second-hop candidates, NOT decompiled this run)

- **FUN_0043d00b** reads 004ede10 (0043d5e6). RESOLVED D37 (2026-08-20):
  FUN_0043d00b is the BRIEFING screen (the BRF_DROP play site - see the
  D37 section), not the sim/render step; that read is its exit-loop
  fade-done condition. The sim/render body question (D15) moves wholly
  to FUN_00440e45 (gameplay, runs after the briefing returns 1).
- **FUN_00440e45** - [RESOLVED 2026-08-23, RE-EXW-SIM §7j.45: THE SHOP
  screen (buy/sell/auto-loadout + the MP loadout sync), NOT the gameplay
  loop; returns 0 = continue / 1 = abort-quit].
- **FUN_00448ef1** reads divider 004edbc8 four times (0044936b/004493eb/
  004495ae/00449797) - another rate consumer, candidate render/anim pacer
  [hypothesis].
- 004ede10 (= fade countdown) writer set: GameMain (=0 at boot),
  FadeSetup@0041cbf0 (arm: =ticks, 2 writes incl. clear),
  FUN_0041e19d (arm at 0041e1d8; also resets divider 004edbc8 at 0041e1a2),
  FUN_00420100 (cancel: =0 at 00420135 when 004dc6c8), FadeStep itself
  (decrement); readers: FUN_00402b0c (fire condition), FUN_0041fa3f,
  FUN_0043d00b [verified xrefs - CORRECTED semantics: fade engine, not
  screen gates].

## Settled / still open

**Settled this run:**
1. Worker-thread body 0044dea0 = trampoline; real loop = GameMain@0041c050
   [verified].
2. No Sleep/timeGetTime pacing on the game thread. **8street 20fps sim/render
   claim: REFUTED at this depth** (no /5 divider or 50ms wait exists here).
   CORRECTED 2026-08-17 (tick2/D15): 004ede10 is NOT a 50Hz frame gate but
   the palette-fade countdown; sim/render rate mechanism unknown until the
   FUN_0043d00b/FUN_00440e45 bodies are read.
3. Go flag 004ef674 writer set is exactly {GameThreadStart=0, GoFlagSet=1};
   0044deca misread corrected [verified].
4. RNG seeds planted here: 123456 / 234567 at 004ede48/004ede4c [verified].
5. Structure: 7 zones x 5 levels, mission number 1..26, completion table
   004decb2 [verified structure].
6. US/UK asset fork via DAT_0046ae64 [verified].

**Still open:**
1. DONE 2026-08-17 (tick2 run): GoFlagSet caller = FUN_0041e19d (see above).
2. PARTIALLY RESOLVED D37 (2026-08-20): FUN_0043d00b fully decompiled =
   the briefing screen (the BRF_DROP play site, see the D37 section) - NOT
   the gameplay advance. FUN_00440e45 (level manager = the real gameplay
   loop) is still open, as is any rate-gate subdivision there. This is the
   last place the 8street claim could still hold partially.
3. RNG function/constants consuming 004ede48/004ede4c; reader of exit code
   004ef676.
4. DONE 2026-08-17 (tick2 run): FUN_0041e19d decompiled - divider 004edbc8
   re-zeroed when the boot sequence releases the timer (prevents a mid-phase
   first fade/palette-cycle tick); see above.
5. FUN_00448ef1 - the four-read divider consumer.

## Boot attract arm RE (2026-08-20 run; closes the item-1 RE prerequisite)

Provenance: Ghidra headless `-process BEDLAM.EXW -noanalysis` + postScripts
`tools/ghidra-scripts/ExwBootAttract.java` / `ExwBootAttract2.java` on the
single BedlamWatcom import. Raw dumps: `ghidra-project/exw-bootattract.txt`
(string xrefs + GameMain decompiles + skip-gate/flag ref census) and
`ghidra-project/exw-bootattract2.txt` (runner body + callee closure + caller
census). Same tag discipline as the rest of this doc.

### The movie runner FUN_0044567c(name_EAX, arg2_EDX) [verified - decompile]

```c
undefined4 FUN_0044567c(char *name, int arg2)
{
  if (DAT_0046cca4 == 0) return 0;          // movies-enabled gate [verified ref census]
  FUN_0042597c();                            // CLEAR: SurfaceLock, zero 480 rows x 640,
                                             // PresentEnd - TWICE [verified body]
  ...                                        // staging-lock priming spin (0044bc08/0044bc6c)
  handle = FUN_0041ce69(name);               // path prefix (DAT_004de544) + _SmackOpen,
                                             // drive-letter retry via 0046ccc8 [verified]
  if (handle == 0) { FUN_00420100(); ... }   // open-failed cleanup (MCI + global frees)
  dst_h = 0x1e0 - 2*arg2;                    // = 480 - 2*arg2  [verified]
  for (f = 1; f < *(uint*)(handle+0xc); f++) {
    if ((_DAT_004edbc4 != 0) &&
        (ScrollUpdate(), (DAT_004edc45 != 0) || (_g_scroll_flags != 0))) {
      _SmackClose(); FUN_00425851(); return 1;   // SKIP: close + subsystem shutdown
    }
    FUN_0044bc08();                          // lock staging
    _SmackToBuffer(..., dst_h, ...);         // decode into the staging buffer
    if (*(int*)(handle+0x68) != 0) {         // frame set a new palette [verified]
      // if palette entry 0 non-black / entry 255 not 0x3f3f3f:
      //   build _SmackColorTrans() nearest-color tables [verified]
      SetPaletteRGB(handle+0x6c, 0, 0x100);  // FULL 256-entry movie palette
    }
    _SmackDoFrame();                         // scale/blit to the surface
    FUN_0044bc6c();                          // unlock
    FUN_0044b340();                          // Blt/Flip primary (present)
    _SmackNextFrame(..., 0x1e0, 0, handle);
    do { _SmackWait(); } while (ret != 0);   // pace: Smacker frame timing
  }
  return 0;
}
```

Facts pinned [verified from the decompile unless tagged]:

1. ONE-PASS BOUND: the frame loop runs `framecount - 1` iterations (loop
   var 1..count-1, first iteration renders the frame _SmackOpen left
   current, index 0). RING movies therefore play EXACTLY ONE bounded
   pass through the file and the runner returns - the ring flag never
   matters at a play site. Last frame RENDERED is index count-2; the
   final _SmackNextFrame lands on count-1 which is never rendered or
   audibly played. [field at handle+0xc = frame count: inferred - the
   only header field the bound reads, and the only sane external stop
   for the corpus ring movies; loop renders count-1 frames: verified]
2. Y-INSET ARG: destination height = 480 - 2*arg2 (symmetric top/bottom
   bars). Boot GTLOG/LOGO: arg2 = 0 -> full 640x480 1:1, no letterbox.
   TITLE replay: arg2 = 0x50 = 80 -> 480-160 = 320 rows = the 640x320
   title raster letterboxed at y=80. This VERIFIES the D31 "[design]
   exact centering" placement note with the actual EXW arithmetic.
3. PALETTE: applied per frame from the Smack struct (+0x6c, 768 bytes =
   256 RGB), ALL 256 entries, only when the frame changed it (flag
   +0x68); nearest-color translate tables are additionally built when
   palette entry 0 is non-black or entry 255 is not (0x3f,0x3f,0x3f).
   The D31 per-frame palette_dirty compositing matches this shape.
4. SKIP GATE: abort requires _DAT_004edbc4 != 0 AND (after ScrollUpdate)
   DAT_004edc45 / _g_scroll_flags nonzero -> _SmackClose + FUN_00425851
   + return 1. Writers of 004edbc4 [verified xref census]: GameMain
   0041c06b writes 0 at entry; NameEntryScreen (0043a5fc) 0043a843 /
   0043a84e write ESI/EDI - straddling its FUN_004459f7 title-replay
   call at 0043a849. CONSEQUENCE: during the BOOT attract (GTLOG+LOGO,
   which runs BEFORE NameEntryScreen) the gate is 0 -> the check is
   short-circuited OFF -> the boot attract movies are UNSKIPPABLE in
   EXW and always play their full one pass. The skippable movie is the
   TITLE replay inside NameEntryScreen (gate armed around it).
5. SCREEN BETWEEN MOVIES: every FUN_0044567c call starts with
   FUN_0042597c = lock + zero 480x640 + present, done TWICE - the plane
   between two movies (and before TITLE.SMK) is CLEARED TO BLACK by the
   runner itself, then the movie takes it over.
6. MOVIES-ENABLED GATE: DAT_0046cca4 != 0 to play anything; GameMain
   saves/restores it around menu-string parsing and forces 1 around
   FUN_0043a144 [verified ref census, 26 sites].

### The boot arm call order [verified - GameMain decompile]

```c
if (iVar4 == 0) {                            // first-run intro gate (local_2c == 0)
  FUN_0042582a(0x400);                       // Smacker subsystem init
  FUN_0044567c(DAT_0046ae64 ? "GAMEGFX\GTLOG_US.SMK" : "GAMEGFX\GTLOG_UK.SMK", 0);
  FUN_0044567c(DAT_0046ae64 ? "GAMEGFX\LOGO_US.SMK" : "GAMEGFX\LOGO_UK.SMK", 0);
  FUN_00425851();                            // Smacker subsystem shutdown
}
```

GTLOG first, then LOGO; region via DAT_0046ae64 (0 = UK, nonzero = US);
both at full screen (arg2 = 0). The pair is bracketed by
FUN_0042582a/FUN_00425851 (init/shutdown; arg = allocation hint:
0x400 boot, 0x4b0 title, 0x800 gameover [inferred from call sites]).
FUN_0044567c callers [verified census]: GameMain x4 (boot pair, END,
ZONEDONE), FUN_004459f7 (TITLE replay), FUN_0044764c (GAMEOVER).

### The title replay FUN_004459f7 [verified - decompile]

Called from NameEntryScreen@0043a5fc at 0043a849 (its only caller):
FUN_0042582a(0x4b0); if movies enabled: FUN_0044567c("GAMEGFX\TITLE.SMK",
0x50); FUN_00425851(); then either a present-sequence or FUN_00445aab.

### FUN_0041ce69 path builder [verified - decompile]

Builds `<prefix from DAT_004de544>\<name>` (char-pair copy loops), calls
_SmackOpen; on failure retries with a drive-letter substitute
(DAT_0046ccc8, not C) - a CD-drive fallback.

## Briefing screen + BRF_DROP play site (D37, 2026-08-20; queue item 1)

Provenance: Ghidra headless `-process BEDLAM.EXW -noanalysis` + postScript
`tools/ghidra-scripts/ExwBrfDrop.java` (string-block xrefs + decompile +
caller census). Raw dump: `ghidra-project/exw-brfdrop.txt`, log
`ghidra-project/process-exw-brfdrop.log`. String VAs computed from
strings/xxd file offsets via the DGROUP map (file off + 0x401a00) and
confirmed by the Ghidra reference targets; instruction anchors
cross-checked with objdump. Same tag discipline as the rest of this doc.

### Function identity: FUN_0043d00b IS the briefing screen [verified]

The full decompile (dump lines 47-527) shows a self-contained modal
screen, not the gameplay loop: it loads the briefing asset set
(FULLFONT.BIN, BRIEF.BIN, TXPAL2.PAL, DARKPAL.PAL, six BEEP/TEXTBOX SFX
via SfxLoad), builds the zone-movie name, plays the movies (below), runs
a mission-map UI (24 hotspots at 0x4e9628, stride 0xe, cursor hit tests;
GO/exit buttons at rows 0x16b..0x18e), then exits INTO the region loading
screen (LOAD_{UK,US}.BIN + LOADPAL(U) + SetPaletteIndex(0x90)) and
returns 1 when GO was clicked (DAT_0046cbe4 == 1), 0 otherwise. GameMain
calls it in the level wait/advance loop; the real gameplay runs in
FUN_00440e45 AFTER it returns 1. This corrects the earlier "[inferred]
THE gameplay advance / prime candidate for the per-frame sim/render step"
gloss. The briefing entry also starts the music:
`load_midi("SOUND\MIDI\BRIEF") + MusicStart(3)` [verified decompile].

### The BRF_DROP play site [verified - asm at 0043d447..0043d490]

```asm
0043d447  MOV EAX,0x400
0043d44c  CALL 0042582a         ; Smacker subsystem init (0x400)
0043d451  MOV EAX,0x4591f7      ; "GAMEGFX\BRF_DROP.SMK" (literal!)
0043d456  CALL 0041ce69         ; path prefix + _SmackOpen (+ CD retry)
0043d45b  MOV EBP,EAX           ; handle held in EBP across the screen
0043d45d  TEST EAX,EAX / JNE 0043d47d
0043d461  CALL 00420100         ; fade cancel
0043d466  PUSH 0x45920c         ; "ERROR: COULD NOT OPEN BRF_DROP SMACK"
0043d46b  CALL 0044eba0         ; print (FUN_00450bc7, CRT printf family)
0043d473  MOV EAX,1
0043d478  CALL 0044d2da         ; FATAL EXIT (see below)
0043d47d  PUSH 0 / PUSH buf / PUSH 0x1e0 / ...  ; _SmackToBuffer(...)
                                                ; dst height 0x1e0 = 480 rows
```

- **Position**: the HEAD of EVERY briefing (movies enabled): BRF_DROP
  opens first, plays its pass, and hands off to the zone backdrop. A
  dedicated literal + a dedicated error string exist for it - unlike the
  zone backdrops, whose names are constructed at runtime.
- **Full screen**: _SmackToBuffer dst height 0x1e0 = 480 rows = the
  640x480 raster 1:1, no letterbox [verified asm].
- **Gate**: the only condition is DAT_0046cca4 != 0 (movies enabled -
  the same gate FUN_0044567c checks). No skip gate is consulted; the
  drop movie is unskippable (the GO button activates only AFTER the
  handoff, see below) [verified decompile].
- **FATAL on open failure**: FUN_0044d2da/0044d2f2 = fn-pointer teardown
  chain (`call *0x4575a4`, `call *0x4575a8`) then `JMP 00450202` into
  the CRT exit - no return path [verified asm]. The same fatal tail
  guards the zone-backdrop open at the handoff (generic
  "ERROR: COULD NOT OPEN %s SMACK" 0x459245 + FUN_0044eb7b strupr of
  the name) [verified decompile + xref at 0043d695].
- **8street re-anchor** [navigation ref only]: their reconstruction
  plays the briefing pair the same way and keys
  `cinematics_is_enable()` off the EXISTENCE of GAMEGFX/BRF_DROP.SMK
  (options.cpp:217-224, a Win95-vs-DOS data-set sniff) - the EXW analog
  is the DAT_0046cca4 gate this site checks; EXW anchors above are
  primary.

### The handoff to the zone backdrop [verified - decompile]

The screen main loop (`while (DAT_0046cbe4 == 0 || _g_fade_ticks_left !=
0)` - the 004ede10 read at 0043d5e6 is this exit-loop fade-done
condition) per iteration: ScrollUpdate -> if movies: palette sync
(Smack +0x68 new-palette flag -> 0x300-byte copy -> FUN_004258d0 DAC
apply) -> _SmackDoFrame -> _SmackNextFrame -> **handoff check
`framecount(+0xc) - 1 == frame_index(+0x370)`**: _SmackClose +
FUN_00425851 shutdown + FUN_0042582a(0x400) re-init + open the pre-built
name buffer DAT_004dca0c + _SmackToBuffer (failure = the generic fatal
error above). The zone backdrop then RINGS forever (no close bound; the
loop exits only via the UI/quit paths, then _SmackClose +
FUN_00425851). Pacing per frame: PresentEnd + `while (_SmackWait() !=
0)` [verified decompile].

- **One-pass bound**: the drop closes exactly when the frame index
  reaches count-1, i.e. frames 0..count-2 rendered = count-1 frames -
  the SAME render count as the FUN_0044567c modal runner (mechanism
  differs: frame-index equality vs loop counter) [verified decompile;
  field identities: +0xc = frame count (the runner bound field),
  +0x370 = current frame index - inferred from use].
- **The GO button arms only after the handoff**: the cursor handlers
  are gated on `(DAT_0046cca4 != 0 && local_20 == 1)` where local_20 =
  "handoff fired" - the player cannot leave the screen while the drop
  movie plays [verified decompile].
- **Corpus fit** (D32 gate): BRF_DROP.SMK = 640x480, 30 frames,
  33_330 us/frame (~1.0 s), NON-ring, no audio track - a one-shot by
  construction; the handoff is therefore mandatory (a non-ring Smack
  simply stops). 29 of its 30 frames render. The BRF_{B..F}{1..5}
  backdrops are 512-frame silent rings.

### The name builder (resolves the D33 open note) [verified]

The zone-movie name is assembled BEFORE the movies play (0043d1b7..
0043d335, Watcom inline strcpy/strcat/itoa expansions) into the buffer
at DAT_004dca0c:

```
"GAMEGFX\BRF_" (0x4591c2) + char(zone@004edd8c + 0x40) +
itoa(level@004edd88, 10) (FUN_0044d291) + ".SMK" (0x4591cf)
```

when movies are enabled; when DAT_0046cca4 == 0 the SAME layout uses the
second prefix copy (0x4591d4) + ".BIN" (0x4591e1) and the screen instead
loads the static backdrop: ArenaAlloc(290000) + LoadFile(.BIN) +
LoadFile("GAMEGFX\BRFPAL.PAL" 0x459232 -> 004edbf8) + FUN_00401e39(0,1,
0,0) + FUN_004258d0 [verified decompile; corpus: BRIEF.BIN + BRFPAL.PAL
exist - the branch is corpus-backed, merely shadowed by the default-on
movies gate].

**Letter map [verified]**: letter = zone + 0x40, so zones 2..=6 = B..=F
directly - the D33 open note about the zone-number-to-letter map (the
letter was taken verbatim) is resolved. The theoretical letters A
(zone 1) / G (zone 7) have no corpus files; zone 7 never briefs
(GameMain `if (zone@004edd8c == 7) ret = 1` skips the call [verified])
and the boot camp briefs through its own pre-episode path (D33
reconciliation) [inferred] - the episode-loop briefing domain is zones
2..=6 = exactly the 25-file BRF_{B..F}{1..5} corpus.

### Rust wiring (D37)

engine/bedlam-game `src/brief.rs` BriefIntro (Staged -> Drop ->
Backdrop on the D31 movie lifecycle: the drop plays its one pass capped
at frames-1 decoded frames, then the backdrop ring takes the plane for
the rest of the scene), GameHost::load_briefing(drop, backdrop) staging
it inert-until-Brief; corpus gate `tests/brief_gate.rs` (drop 29/30
frames, exact switch pump, silent, ring continues, two runs
byte-identical). See DECISIONS.md D37.

Landed as bba01fe (2026-08-20). The gate run also exposed + fixed a
latent D31 MoviePlayer bug: the seam reports Last at the closing
slot of EVERY ring pass (ring total = frames + 1, wrap jumps to
frame 1); advance_limited used to latch finished on ring-Last,
freezing any ring stream at its first cycle end (no prior consumer
had driven a ring that far). Ring-Last now continues (DECISIONS D37
item 4); the gate pins the 512 -> 1 wrap of the corpus backdrop.
