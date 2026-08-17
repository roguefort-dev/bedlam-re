# RE: BEDLAM.EXW — startup, window, message pump, main loop

Provenance: all facts below from Ghidra headless analysis of the single imported
program `BedlamWatcom:/BEDLAM.EXW` (x86:LE:32:default + `openwatcomcpp` cspec,
import verified 2026-08-17 03:33). Scripts: `tools/ghidra-scripts/ExportExwMainLoop.java`,
`tools/ghidra-scripts/ExportExwFollowup.java`. Raw dumps (gitignored):
`ghidra-project/analysis/exw-functions.txt` (672 functions),
`exw-mainloop.txt`, `exw-followup.txt`.

Tags: [verified] = read in decompile + listing; [inferred] = strong deduction
from verified facts; [hypothesis] = plausible, needs confirmation.
Addresses are EXW VAs (imageBase 00400000).

## Boot chain [verified]

```
entry 004502ee      : _DAT_004ef8fc = &LAB_0044d6e8; jmp/call FUN_004520ed
  FUN_004520ed      : Watcom C++ startup (heap init, GetCommandLineA parse,
                      GetModuleHandleA, then call (*_DAT_004ef8fc)() @004521cc)
    LAB_0044d6e8    : "BedlamShutdown" (renamed in project) — this is main().
                      (Created as function by follow-up script; also reached
                      directly at 0044d6e8..)
```

`main()` flow @0044d6e8 [verified]:

```
FUN_0044bdd0(); FUN_0044ef50();            // pre-init (unknown detail)
_DAT_004ef678 = 1; _DAT_004ef6b4 = 1;      // state flags
r = FUN_0044d320(hInst, nCmdShow, cmdline);// window + DirectDraw + Smacker init
if (r == 0) {
  r2 = FUN_0044d9c0();                     // thread/etc; nonzero -> err 4
  if (r2 == 0) {
    r3 = FUN_0044da64();                   // wait-for-go, arm 100Hz timer
    if (r3 == 0) {
      ret = FUN_0044d93c();                // BLOCKING message pump until WM_QUIT
      timeKillEvent(_DAT_004ef69c);        // @0044d756
      timeEndPeriod(10);                   // @0044d75e
    }
    FUN_0044da1c();                        // post-run teardown
  } else err = 4 ("Error starting thread")
  FUN_0044c20c(); FUN_0044ab54();          // cleanup (DDraw/etc release)
  if windowed-mode-latched: mciSendCommandA(x3)  // CD-audio close
}
FUN_0044de10(<error text>) on nonzero r    // error display (its own ShowWindow)
```

Error strings resolved in listing: "Class error", "DDInit error",
"SetVideoMode error", "Error starting thread", "Please install DirectX",
"DirectX is already in use by another application", "DirectDraw can not
initialise a colour", "DirectDraw initialization failed", "Unknown error".
[verified]

## Init: FUN_0044d320 (WinMain-equivalent) [verified]

- Command line scanned for `-f` (toggles windowed flag, high word of dword
  @004ef69e) and `-v` (sets byte @004ef6a2). [verified]
- If fullscreen intended: `GetDeviceCaps(hdc, RASTERCAPS)` without RC_PALETTE
  (0x100) -> MessageBoxA "256 colour mode needed" then force windowed. [verified]
- WNDCLASSEX: cbSize 0x30, style 3 (CS_VREDRAW|CS_HREDRAW) or 0xb (+CS_DBLCLKS)
  when dword@004edece high word == 1; lpfnWndProc = 0044dacc. [verified]
- Window: class "Bedlam", title "Bedlam for Windows 95", default size from
  word@00456ec6 = 0x0280 (640) x word@00456ec8 = 0x01e0 (480). Fullscreen:
  WS_VISIBLE|WS_POPUP (0x90000000) at 640x480. Windowed: 0xca0000
  (caption/sysmenu/minimizebox), client+2 / +0x15 borders, centered. HWND stored
  @004ef68c. [verified]
- DirectDraw: FUN_0044a5f0() primary path; on failure FUN_0044a660(640,480)
  fallback + FUN_0044ab54(); success path also FUN_0044a9ac() +
  _SmackSoundUseDirectSound() when flag high-word @004ef676 set. [verified,
  flag semantics partially unclear — see open questions]

## Message pump: FUN_0044d93c [verified]

```c
while ((word @004ef692) == 0) {
    if (PeekMessageA(&msg,0,0,0,0)) {
        if (GetMessageA(&msg,0,0,0) == 0)   // WM_QUIT
            word @004ef692 = 1;              // -> loop exit, returns msg.wParam
        else if (!(msg.message==WM_ACTIVATEAPP(0x1c) && msg.wParam==0
                  && windowed))
            DispatchMessageA(&msg);
    }
}
```
Calls via import pointer table .idata (@004f024c Peek, @004f0230 GetMessage,
@004f0224 Dispatch). Quirk: in windowed mode, deactivation WM_ACTIVATEAPP is
not dispatched [verified].

## Timer thread + game tick [verified]

FUN_0044da64 (called from main before the pump):

```c
while (_DAT_004ef674 == 0) ;               // wait for go flag
timeBeginPeriod(word @00456ec4);            // = 0x000a = 10 ms
_DAT_004ef69c = timeSetEvent(word @00456ec4, 0, LAB_0044de58, 0, TIME_PERIODIC);
```

- Period word @00456ec4 = 10 => **100 Hz tick, anchored [EXW .data @00456ec4]**.
  This independently confirms the 8street claim of a 100Hz timer (8street
  citation non-normative; now anchored).
- **Game main loop = periodic multimedia-timer callback LAB_0044de58** (runs on
  a winmm worker thread, not the GUI thread). Pump keeps the window alive on
  the main thread. NOT YET DECOMPILED — top follow-up.
- WM_DESTROY handler suspends thread handle @004ef698 (id @004ef694) before
  quit [verified].

## WndProc @0044dacc ("BedlamWndProc", created+decompiled by follow-up) [verified]

| msg | handling |
|---|---|
| WM_DESTROY (2) | SuspendThread(@004ef698) if != -1; FUN_0044b1c0(0); word@004ef692=exit; PostQuitMessage(0) |
| WM_MOVE (3) | windowed only: store x/y @004ef6a4/6a6; InvalidateRect |
| WM_SIZE (5) | windowed only: store w/h @004ef6a8/6aa (+@004ef682/684); InvalidateRect |
| WM_ACTIVATEAPP (0x1c) | windowed && deactivate -> swallow (ret 1); else @004ef670=fActive, FUN_0044b1c0(fActive) |
| WM_SETCURSOR (0x20) | hide cursor in fullscreen (hit-test 1), else arrow; blocks when iconic |
| WM_KEYDOWN (0x100) | vk 0x46 `F` -> FUN_0044ceb0 [hypothesis: fullscreen toggle]; else FUN_0041be05(vk, 0) |
| WM_KEYUP (0x101) | FUN_0041be05(vk, 1) |
| WM_SYSCOMMAND (0x112) | wParam 0xf000-0xf100 -> FUN_0041be05(0x44,0); SC_SCREENSAVE(0xf140) -> blocked (ret 1) |
| WM_LBUTTONDOWN/UP/DBLCLK (0x201/2/3) | FUN_0041bf35(0, 0/1/2) |
| WM_RBUTTONDOWN/UP/DBLCLK (0x204/5/6) | FUN_0041bf35(1, 0/1/2) |

Input entry points (for the future input subsystem spec):
- **FUN_0041be05(keyCode, isRelease)** — keyboard events. [verified]
- **FUN_0041bf35(button{0=L,1=R}, event{0=down,1=up,2=dblclk})** — mouse
  buttons. No WM_MOUSEMOVE handling: mouse position must be polled
  (GetCursorPos is imported). [verified/verified-inferred]

## Globals map (this pass) [verified unless noted]

| VA | meaning |
|---|---|
| 00456ec4 | word: timer period ms (=10) |
| 00456ec6/8 | word pair: default window client 640x480 |
| 004ef670 | fActive (WM_ACTIVATEAPP) |
| 004ef674 | go flag the timer-arming waits on |
| 004ef676 | dword; high word gates DirectDraw/Smacker path |
| 004ef682/4 | window x/y (WM_SIZE path) |
| 004ef68c | HWND |
| 004ef690 | dword; high word @004ef692 = quit flag |
| 004ef694/98 | worker thread id / handle |
| 004ef69c | timeSetEvent id |
| 004ef69e | dword; high word @004ef6a0 = windowed-mode flag |
| 004ef6a0..b2 | window geometry cache |
| 004ef8fc | function pointer to main() (set by entry, called by Watcom startup) |

## Open questions / next steps

1. Decompile LAB_0044de58 (100Hz tick): expected to contain the 20fps
   sim/render pacing (8street claim, unanchored), input polling, RNG chain,
   and state strides. This is the next RE target.
2. FUN_0044d9c0 — what thread it starts ("Error starting thread" path);
   relation to FUN_0044da64 (is 0044da64 that thread body? then who sets
   004ef674?). [open]
3. FUN_0044ceb0 — confirm F-key = fullscreen toggle. [hypothesis]
4. FUN_0041be05 / FUN_0041bf35 internals -> input map spec (backlog task).
