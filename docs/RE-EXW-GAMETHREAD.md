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
  if (first run / iVar4 == 0) {                   // intro path
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
          ret = (ret == 1) ? 1 : FUN_0043d00b(...);   // THE gameplay advance;
                                                     // reads 50Hz gate 004ede10 (0043d5e6)
          if (ret == 2) DAT_0046ae74 = 1;         // 2 = abort -> quit
        }
      }
      _DAT_004eb934 = _DAT_004dd40c;              // checkpoint level context
      _DAT_004eb938 = DAT_0046ae70;
      DAT_0046ae74 = FUN_00440e45(DAT_0046ae70, ...);  // [inferred: zone/level manager]
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
      FUN_0043c87c(&DAT_0046bc4c, ..., 0x96, 0x82);   // [hyp: text draws, coords 150/180/210]
      FUN_0043c87c(&DAT_0046bc7c, ..., 0xb4, 0x82);
      FUN_0043c87c(&DAT_0046af5c + (zone+0x51)*0x30, ..., 0xd2, 0x82);
      if (zone == 6) FUN_0043c87c(&DAT_0046bfdc, ..., 0x104, 0x82);
      FUN_00425a03();
      ... copy 24 dwords + tail bytes from pcStack_24+2 into buf+0x2a2 ...  // font row blit
      FUN_0041cbf0(_DAT_004edbf8, 10);            // FadeSetup(pal,10): arm 10-step 50Hz fade (004ede10 = steps left)
      episode++;                                  // iStack_20
      _DAT_004edd8c++;                            // zone++
    }
  }
  goto LAB_0041c454;                              // next episode
}
```

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

### New xref leads (second-hop candidates, NOT decompiled this run)

- **FUN_0043d00b** reads 004ede10 (0043d5e6) - prime candidate for the
  per-frame sim/render step [inferred]. NOTE (tick2): 004ede10 = fade
  countdown, so that read is fade-status, not a rate gate; the sim/render
  rate mechanism inside FUN_0043d00b is the open question (D15).
- **FUN_00440e45** - zone/level manager returning quit status [inferred].
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
2. Bodies of FUN_0043d00b (gate-consuming gameplay advance) and FUN_00440e45
   (level manager) - the real per-frame sim/render, and whether the 50Hz gate
   is further subdivided (25Hz? 20Hz?) there. This is the last place the
   8street claim could still hold partially.
3. RNG function/constants consuming 004ede48/004ede4c; reader of exit code
   004ef676.
4. DONE 2026-08-17 (tick2 run): FUN_0041e19d decompiled - divider 004edbc8
   re-zeroed when the boot sequence releases the timer (prevents a mid-phase
   first fade/palette-cycle tick); see above.
5. FUN_00448ef1 - the four-read divider consumer.
