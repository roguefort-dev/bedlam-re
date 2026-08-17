# BEDLAM.EXW — Startup Chain, Message Pump, and Game-Loop Architecture

Verified 2026-08-17 by Ghidra 12.1.2 headless analysis (project `BedlamWatcom`,
language `x86:LE:32:default`, compiler spec `openwatcomcpp` = GhiOWat watcall).
Provenance: every address/claim below was read from decompiler output exported by
`tools/ghidra-scripts/{ExportExwMainLoop,ExportExwFollowup,ExwLoopFollowup,ExwNameAndExport}.java`
(artifacts in gitignored `ghidra-project/*.txt`; raw dumps local-only, facts re-stated here).
Image base 00400000; all addresses are EXW virtual addresses. Function inventory:
`docs/exw-functions.txt` (675 functions after this pass).

Confidence: [high] = read directly in decompilation/listing; [med] = interpreted
(meaning inferred from usage); [hyp] = hypothesis for next pass.

## 1. Startup chain [high]

```
PE entry 004502ee  : MOV [0x004ef8fc], offset 0044d6e8 ; call 004520ed
WatcomCrtStartup 004520ed : Watcom CLIB init (argc/argv from GetCommandLineA,
                            GetModuleHandleA) then  CALL [0x004ef8fc]  -> WinMain
WinMain 0044d6e8   : pre-init -> InitInstance -> GameThreadStart -> TimerInit
                     -> MsgPump -> teardown
```

The pointer-at-global indirection (`004ef8fc` <- WinMain) is how this Watcom build
links the CRT startup to the app entry — there is no exported `WinMain` symbol.

## 2. WinMain (0044d6e8) [high; was misnamed "BedlamShutdown" — corrected]

Call order: `FUN_0044bdd0`, `FUN_0044ef50` (pre-init), then:
1. `InitInstance` (0044d320) — returns 16-bit error code
2. `GameThreadStart` (0044d9c0) — nonzero => "Error starting thread"
3. `TimerInit` (0044da64)
4. `MsgPump` (0044d93c) — returns wParam (exit code)
5. Teardown: `timeKillEvent(_DAT_004ef69c)`, `timeEndPeriod(period)`,
   `FUN_0044da1c`, `FUN_0044c20c`, `FUN_0044ab54`; if windowed mode was active,
   3x `mciSendCommandA(wDev, 0x808/0x804, 2, 0)` (CDDA close/pause).
Error dispatch maps codes to strings at 00459d5b..00459e6f:
1="Class error" 2="DDInit error" 3="SetVideoMode error" 4="Error starting thread"
1000..1003 = "Please install DirectX" / "DirectX is already in use by another program"
0x3eb..0x3ed = DirectDraw init failures; 0x3f2 silent.

## 3. InitInstance (0044d320) [high]

- Command-line flags parsed manually: `-f` toggles bit 0x10000 of mode flags
  `004ef69e` [med: force windowed/fullscreen], `-v` sets `004ef6a2`=1 [med].
- 256-colour capability check: `GetDeviceCaps(hdc, RASTERCAPS)` bit 0x100 else
  MessageBox "256 colour mode needed. Click OK." -> abort path.
- `RegisterClassExA` with WNDPROC = `BedlamWndProc` (0044dacc), icons 0x7d0/0x7d1,
  arrow cursor 0x7f00, hbrBackground = GetStockObject(4).
- Window: class "Bedlam", title "Bedlam for Windows 95" (strings 00456e00/08);
  size from `DAT_00456ec6` (w,h pair [med: 640x480 game mode]); windowed
  style 0xca0000 vs popup 0x90000000; centered on GetSystemMetrics(0/1).
- DirectDraw bring-up: `FUN_0044a5f0` (probe), `FUN_0044a660` (mode set),
  `FUN_0044ab54`, `FUN_0044a9ac` (surface canary 0x12345678 + full clear loop —
  verifies lockable primary/back buffers, clears both).
- `_SmackSoundUseDirectSound` hooked before returning (Smacker audio routing).

## 4. MsgPump (0044d93c) [high] — hybrid Peek/Get pump

```
while (hiword(_DAT_004ef690) == 0) {
    if (PeekMessageA(&msg,0,0,0,0)) {
        if (GetMessageA(&msg,0,0,0) == 0) hiword(_DAT_004ef690) = 1;   // WM_QUIT
        else if (!(msg.message==WM_ACTIVATEAPP(0x1C) && msg.wParam==0
                  && hiword(_DAT_004ef69e)==1))
            DispatchMessageA(&msg);                                     // filtered
    }
}
return msg.wParam;
```
The filter swallows deactivation WM_ACTIVATEAPP while in windowed mode
[med: prevents pause-on-deactivate confusion]. Quit flag: hiword of 004ef690;
also set by InitInstance failure path (0044d640).

## 5. Game thread + periodic timer = the actual "main loop" [high]

The pump thread is NOT the game loop. WinMain starts a second thread and a
multimedia timer; rendering/sim cadence hangs off them:

- `GameThreadStart` (0044d9c0): `_DAT_004ef674 = 0; handle = FUN_00450242()`
  -> `FUN_00450242` is a Watcom `_beginthread`-style wrapper [med]; thread handle
  stored `004ef698`, id `004ef694` (WndProc WM_DESTROY suspends this handle).
- `TimerInit` (0044da64): `while (_DAT_004ef674 == 0);` — spin-waits until the
  game thread signals readiness, then `timeBeginPeriod(period@00456ec4)`
  and `timeSetEvent(period, 0, 0044de58, 0, TIME_PERIODIC)` -> id `004ef69c`.
- `TimerCallback` (0044de58), runs every 10ms (100Hz) [high]:
  1. `FUN_0041bfb6()` (named TickWorker) — called EVERY tick; it is a SERVICE
     step (counters + 50Hz gate, scroll clamp, palette cycle), NOT the
     sim/render loop [high — see docs/RE-EXW-TICK.md]
  2. if active (`004ee9d6` hiword==1) && not paused (`004eee5a` hiword==0):
     `_DAT_004ef708 = 0`; `GetCursorPos` -> `FUN_0044b4fc(x, y)` mouse sample.

**Determinism (P3/P5 impact):** timing entropy slots on the EXW side are exactly
( a ) the `timeSetEvent` period (u16 at 00456ec4) driving tick cadence,
( b ) `GetCursorPos` sampled inside the tick, ( c ) keyboard/mouse WndProc bridge
below. The tick advances counters/scroll/palette only; the sim/render frame
lives on the worker thread spawned at 0044dea0 (docs/RE-EXW-TICK.md).

## 6. BedlamWndProc (0044dacc) — input bridge [high]

- WM_DESTROY(2): `SuspendThread(game thread)` (if id != -1), `FUN_0044b1c0(0)`,
  `004ef692`=1, `PostQuitMessage(0)`.
- WM_MOVE(3)/WM_SIZE(5): update window bounds globals + InvalidateRect (windowed
  mode only, guarded by hiword(004ef69e)).
- WM_ACTIVATEAPP(0x1C): gate `004ef670` = wParam, `FUN_0044b1c0(wParam)`; blocked
  in windowed mode unless wParam==1.
- WM_SETCURSOR(0x20): hide cursor when hit-test==1 and DD-active; else arrow.
- WM_KEYDOWN(0x100)/WM_SYSKEYDOWN(0x101): vkey := hiword(lParam)&0xFF ->
  `FUN_0041be05(vkey, down)`; 'F'(0x46) special-cased to `FUN_0044ceb0`
  (FKeyHandler) = screenshot to numbered BMP [high — see RE-EXW-TICK.md];
  WM_SYSCOMMAND 0xF100 filtered, 0xF140 eaten.
- Mouse: WM_LBUTTON*(0x201..0x206) -> `FUN_0041bf35(button, state)` with
  (button 0/1, state 0/1/2 = down/up/double [med]) and keyState bits pushed.

`FUN_0041be05` (keyboard) / `FUN_0041bf35` (mouse) are the game-side input
sinks — the input/control map task should start there.

## 7. Globals (EXW .data) referenced above [high unless noted]

| Addr | Role |
|---|---|
| 004ef674 | game-thread ready flag (spin-wait) |
| 004ef670 | active flag (WM_ACTIVATEAPP wParam) |
| 004ef676 | DD/Smacker state flags (hiword: DD active) |
| 004ef682/84 | last window pos; 004ef6a4/a6 current pos |
| 004ef688 | hInstance (InitInstance) |
| 004ef68c | HWND main |
| 004ef690 | quit flag (hiword) |
| 004ef692 | quit reason (set 1 on WM_DESTROY / pump WM_QUIT) |
| 004ef694/98 | game-thread id / handle |
| 004ef69c | timeSetEvent id |
| 004ef69e | mode flags (hiword bit0: windowed) |
| 004ef6a2 | `-v` flag [med] |
| 004ef6a8/aa | client size; 004ef6b0/b2 screen metrics |
| 004ef708 | per-tick counter reset in TimerCallback [med: skip/idle counter] |
| 004ef8fc | WinMain function pointer (CRT->app link) |
| 00456ec4 | timer period u16 (10ms = 100Hz [high]); mode dims at 00456ec6 |

## 8. Watcall notes for future passes [high]

- Compiler spec `openwatcomcpp` (GhiOWat): args in EAX,EDX,EBX,ECX then stack;
  decompiler shows them as `unaff_EAX/EDX/ECX` / `in_stack_*`; callee cleans
  (`RET n`; WndProc `RET 0x10` matches its 4 stack params).
- Win32 imports are cdecl/stdcall and reached through IAT thunks
  (`CALL CS:[0x004f0xxx]`); the global watcall default proto does NOT model
  their params — per-function overrides still needed when decompiling
  import-heavy wrappers.
- CRT pieces identified: `FUN_00451f22` (init), `FUN_004501bc`/`FUN_0045283c`
  (stack/argv setup), `FUN_00451edd` (argv parse), `FUN_00450242`
  (_beginthread wrapper [med]), `FUN_0044d2da` (exit/return-to-CRT).

## 9. Open / next

1. DONE — TickWorker@0041bfb6 cataloged in docs/RE-EXW-TICK.md (service tick:
   5 counters, 50Hz sub-gate, scroll clamp, 8-frame palette cycle @12.5Hz).
2. ANSWERED — GameThreadStart passes start address 0x0044dea0 (stack 0x1000,
   id -> 004ef694, handle -> 004ef698); region 0044dea0..0044dfec is not yet
   a function = TOP next target (docs/RE-EXW-TICK.md).
3. `FUN_0044b4fc`, `FUN_0041be05`, `FUN_0041bf35` input sinks -> control map.
4. mciSendCommandA trio in teardown -> CDDA control path (audio cross-check).
