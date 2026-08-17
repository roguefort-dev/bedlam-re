# RE: BEDLAM.EXW - the 100Hz timer tick and its satellites

Provenance: Ghidra headless `-process BEDLAM.EXW -noanalysis` + postScript
`tools/ghidra-scripts/ExwTickFollowup.java` against the single BedlamWatcom
import (x86:LE:32:default + openwatcomcpp cspec; run exit 0, Save succeeded,
2026-08-17 04:2x). Raw dump: `ghidra-project/exw-tick.txt`, log:
`ghidra-project/process-exw-tick.log` (both gitignored; NOTE dumps live in
ghidra-project/ root, not ghidra-project/analysis/). Names applied and persisted
in the Ghidra project this run: TickWorker@0041bfb6, MousePosHandler@0044b4fc,
ThreadSpawnThunk@00450242, FKeyHandler@0044ceb0, AppActivate@0044b1c0
(TimerCallback@0044de58, GameThreadStart@0044d9c0 from the previous run).

Tags: [verified] read in decompile/listing; [inferred] strong deduction;
[hypothesis] plausible, needs confirmation. Addresses are EXW VAs (base 00400000).

Follow-up pass 2026-08-17 (tick2 run): scripts ExwTickFollowup2.java +
ExwTickNames.java, dumps ghidra-project/exw-tick2.txt + exw-tick2-names.txt
(logs process-exw-tick2*.log). It resolved open items 2-5 below and CORRECTED
the 50Hz-gate reading of 004ede10 (see D15). Names applied + persisted:
FadeStep@00425901, CursorToGame@0044b428, SetPaletteRGB@0044aed4,
FadeSetup@0041cbf0, DDCreate@0044a5f0, DDInitSurfaces@0044a660,
DDShutdown@0044ab54, ThreadSpawnImpl@0045204b; labels g_fade_ticks_left
(004ede10), g_fade_state_16_16 (004edc38), g_fade_palette_6bit (004edc3c),
g_dd_obj (004ee9b8), g_dd_palette (004ee9d0), g_dd_clipper (004ee9d4),
g_thread_spawn_slot (00457874).

## Headline: the tick is a SERVICE routine, not the game loop

The 100Hz `timeSetEvent` callback (TimerCallback@0044de58) does per-tick
housekeeping only: input-side updates, counters, palette cycling. The
sim/render loop does NOT live here. It lives on the worker thread spawned by
GameThreadStart@0044d9c0 with start address **0x0044dea0** [verified via
listing; region 0044dea0..0044dfec is not yet a function in the project].
The 8street "20fps sim/render in the tick" claim therefore cannot be confirmed
from the tick body itself; pacing evidence must come from the 0044dea0 thread
body and from the counter gates below [inferred].

EXW threading model [verified/inferred]:
```
main thread : WinMain -> InitInstance -> GameThreadStart (spawn worker)
                                                  -> TimerInit (spin until go)
                                                  -> MsgPump (GUI only)
worker thread (start 0044dea0, stack 0x1000): real game loop; signals go flag
winmm worker thread: TimerCallback every 10ms (this doc)
```

## TimerCallback @0044de58 [verified]

```c
void TimerCallback(void)
{
  TickWorker();                                  // every tick
  if ((word@004ee9d6 hi == 1) && (word@004eee5a hi == 0)) {
    word@004ef708 = 0;
    GetCursorPos(&pt);                           // import CS:[0x4f0228]
    MousePosHandler(pt.x, pt.y);                 // Watcom: args in EAX/EDX
  }
}
```
- Mouse is POLLED at up to 100Hz whenever the state gates allow (gate meaning
  [inferred]: a mouse-driven screen is active and nothing blocks it).
- No render, no sim step here.

## TickWorker @0041bfb6 [verified]

```c
if (dword@004edbe0 != 0) FUN_00402bac();   // gated subsystem pump [hyp: sound/event queue]
FUN_00402b0c();                            // counters + 50Hz sub-gate
FUN_00425ab9();                            // input-driven scroll update
if (((0x8f < dword@004edb7c) && (dword@004edb7c < 0x98))
    && ((dword@004edbc8 & 7) == 0)) {
  dword@004edb7c++;                         // 8-frame palette cycle, advance
  if (dword@004edb7c == 0x98) dword@004edb7c = 0x90;   // every 8th tick
  FUN_0041d714(dword@004edb7c);             // SetPaletteIndex
}
```

### FUN_00402b0c: tick counters + fade fire condition [verified]
Increments five free-running counters each tick: @004edb84, @004edbc8,
@004edbcc, @004edba4, @004edba8. Calls FUN_00402b48 every tick. When
`(ctr@004edbc8 & 1) && dword@004ede10` -> calls FadeStep@00425901
[verified]. **CORRECTED (tick2 run, D15)**: 004ede10 is NOT a frame-rate
gate - it is the **palette-fade step countdown** (nonzero only while a fade
is running). FadeStep runs at 50Hz (bit0 of the 100Hz counter) *while
fading*, decrements 004ede10, and stops at 0. Counter @004edbc8: bit0 ->
50Hz fade phase, bits0..2 -> palette cycle phase (12.5Hz) [verified/impl].
See "Palette fade engine" below.

### FUN_00425ab9: scroll/camera update [verified structure]
- snapshot: dword@004eddcc = dword@004dc6e4 (live input direction flags
  [inferred]); bit0/bit1 drive x/y handling.
- FUN_0044b428(&x,&y) = CursorToGame@0044b428 [verified, tick2]: GetCursorPos
  then (640*(cx-win_x0))/win_w, (480*(cy-win_y0))/win_h using the WM_MOVE/
  WM_SIZE window-rect cache (004ef6a4/a8/a6/aa) and the canonical screen
  dims word@00456ec6=640 / word@00456ec8=480 (same dims the BMP screenshot
  writer uses); clamps to [0,win_w-1]x[0,win_h-1]. So the scroll source is
  the **cursor mapped into the 640x480 game space** (edge/drag camera
  control), NOT accumulated key deltas.
- clamp x to [9, 0x277=631], y to [9, 0x1cf=463] (640x480 minus margins),
  store to dword@004eddc4 (x) / @004eddc8 (y); copies to @004eddf8/fc when
  bit0 and @004ede00/04 when bit1.
- if dword@004edb80: FUN_0041d714(x >= 0x1e0 ? 0x5d : 0) - region palette
  select when scrolled past half screen [inferred].

### FUN_0041d714: SetPaletteIndex [verified structure]
- guards: idx != dword@004dc9f4 (last applied) and non-reentrant (word@004ededa hi).
- FUN_0044bbac(0x18) / FUN_0044bb84 / FUN_0044bc90 = DirectDraw palette
  prepare chain [hyp: IDirectDrawPalette-related].
- copies 24 rows x 0x18 bytes (read stride 0x20) from table base
  dword@004edd7c + idx*4 + 2 into the target structure; commits via
  FUN_0044bcf4; records idx into @004dc9f4 and @004edb7c.

## MousePosHandler @0044b4fc [verified]

- Clamps cursor x/y into the window rect cache from WM_MOVE/WM_SIZE
  (x0 @004ef6a4, w @004ef6a8; y0 @004ef6a6, h @004ef6aa; max minus 1).
- Gated by dword@004ee9bc (DirectDraw surface object ptr != 0) and a busy
  word @004eedf8 (skips if lo==1 or hi==1; sets lo=1 during work, clears at
  end).
- DirectDraw surface vtable calls [offsets verified, semantics verified via
  DDERR_SURFACELOST 0x887601C2 handling]: +0x60 IsLost, +0x6C Restore,
  +0x14 Blt - on objects at *004ee9bc, *004ee9c8, *004ee9cc; helper
  FUN_0044b7b0 repeats IsLost/Restore and sets word@00457396 hi = 0xFFFF
  (cursor validity window [hyp]). NOTE (tick2): these surface offsets are
  **+8 vs the stock IDirectDrawSurface layout for everything past GetCaps**
  (Lock@+0x64, SetPalette@+0x7c, Unlock@+0x80, SetClipper@+0x70 - the
  last confirmed by the windowed-mode clipper attach in DDInitSurfaces),
  while Blt@+0x14 and GetCaps@+0x30 are stock; i.e. the ddraw.h this game
  compiled against carries 2 extra slots in the GetClipper..Initialize
  region. DD/palette/clipper objects (004ee9b8/d0/d4) are fully stock -
  see the DDRAW init section below.
- Stores the clamped cursor: short@00457398 = x, short@004eedf6 = y.
- Net effect [inferred]: hardware-cursor-style tracking + surface recovery at
  100Hz, decoupled from the (slower) sim/render on the worker thread.

## GameThreadStart @0044d9c0 and ThreadSpawnThunk @00450242 [verified listing]

```
GameThreadStart:
  word@004ef674 = 0                 ; reset go flag
  EAX=0 EDX=0x1000 EBX=0x0044dea0 ECX=0   ; Watcom register args
  push 0x004ef694  (lpThreadId)  push 0  (flags)
  CALL 00450242
  dword@004ef698 = EAX             ; thread handle
  ret (ZF set from 004ef694 == -1) ; error path -> "Error starting thread"
ThreadSpawnThunk (23 bytes):
  pushes the 2 stack args, CALL dword ptr [0x00457874], RET 0x8
```
- CreateThread(0, 0x1000, 0x0044dea0, 0, 0, &004ef694) semantics through a
  .data function slot @00457874 (NOT the IAT at 0x4f0xxx; unresolved -
  possibly Watcom CRT indirection) [verified args, slot target unresolved].
- **Worker thread body = 0x0044dea0** - decompiled 2026-08-17 (follow-up run):
  see **docs/RE-EXW-GAMETHREAD.md**. It is a 59-byte trampoline around
  GameMain@0041c050 (the real game shell/loop). CORRECTION: the instruction at
  0044deca is `MOV EDX,-1` feeding a write to thread id 004ef694 - it does NOT
  write go flag 004ef674 (earlier reading conflated the two globals). Go-flag
  writers are exactly GameThreadStart (reset 0) and GoFlagSet@0044d9b4
  (word 1). 20fps pacing claim refuted at this depth: no Sleep on the game
  thread; pacing = 100Hz tick -> 50Hz gate 004ede10 (see that doc).

## FKeyHandler @0044ceb0 = SCREENSHOT TO NUMBERED BMP [verified]

DISPROVES the earlier hypothesis that F toggles fullscreen (see
RE-EXW-MAINLOOP.md WndProc table, corrected).

Flow:
1. Copies 768 bytes (256 RGB triplets, dword stride) from dword@004ee9f4 =
   current palette into a local BGR buffer.
2. FUN_0044d1f2 builds a filename from counter dword@004ef66c
   (format/template strings at 0x459cec/0x459cfa [hyp: numbered BEDLAMx.BMP
   names]); loop: open via FUN_0044e729(name, mode); if it opened an EXISTING
   file, close (FUN_0044ea0b) and retry with counter+1 - finds a free slot.
3. Writes a bare BMP: "BM", BITMAPINFOHEADER {biSize=0x28, biWidth=640
   (word@00456ec6), biHeight=480 (word@00456ec8), biPlanes=1, biBitCount=8},
   the 768-byte palette, then the locked surface (FUN_0044ac5c = Lock,
   returns base ptr; pitch = dword@004ee9e8) written bottom-up row by row,
   dword-padded per scanline.
4. Patches biSizeImage/file size via FUN_0044e30b (ftell) + FUN_0044e217
   (fseek 2), closes, FUN_0044acf4 = Unlock.

File I/O layer identified (usage-verified, exact libc mapping [inferred]):
FUN_0044e729=fopen(path,mode), FUN_0044e815=fwrite(buf,1,n,f),
FUN_0044f34b=buffered putc (flush thresholds 0x400/0x600),
FUN_0044e30b=ftell, FUN_0044e217=fseek, FUN_0044ea0b=fclose.
Also confirmed here: FUN_0044ac5c=Lock primary surface (sets
dword@004ee9e8=pitch, @004ee9f0=pitch/4, @004ee9ec=pitch-0x280) and
FUN_0044acf4=Unlock (vtable +0x80) [verified].

## AppActivate @0044b1c0 [verified]

- Windowed mode only (returns if fullscreen flag word@004ef69e hi == 1).
- GetDC(0); GetDeviceCaps(hdc, RASTERCAPS 0x26); if RC_PALETTE (0x100):
  SetSystemPaletteUse(hdc, ...) - classic 256-color activation handling
  (activate -> SYSPAL_NOSTATIC when currently static, deactivate ->
  SYSPAL_STATIC) [enum values standard Win32].
- On activate (arg==1) with DD object present [verified, listing]: surface
  004ee9bc +0x6C (Restore), then **004ee9d0 +0x18 = IDirectDrawPalette::
  SetEntries** - the listing pushes exactly (this, 0, 0, 0xFE, 0x4ee9f8) =
  SetEntries(flags=0, first=0, count=254, &entries[1]) - then surface
  +0x7C (SetPalette). Classic palette-app focus-regain sequence. RESOLVED
  (tick2): +0x18 on the PALETTE object is stock SetEntries; it had been
  misfiled as a surface call.
- NOT a pause function. WM_DESTROY calls AppActivate(0) to hand the system
  palette back on exit.

## Globals added to the map this pass

| VA | meaning | tag |
|---|---|---|
| 004dc6e4 | live input direction flags (snapshot source for FUN_00425ab9) | inferred |
| 004dc9f4 | last committed palette index | verified |
| 00457394/6/8 | cursor triplet; 00457396 hi = validity window, 00457398 = clamped x | verified/hyp |
| 00457874 | .data thread-spawn function slot (CreateThread-like) | verified use |
| 004edb7c | palette cycle index, operating range 0x90..0x97 | verified |
| 004edb80 | enables region palette select (0x5d vs 0) | inferred |
| 004edb84/edba4/edba8/edbcc/edbc8 | five free-running 100Hz tick counters; 004edbc8 = divider (50Hz + palette phase) | verified |
| 004edbe0 | gates MusicPump (FUN_00402bac, song 3 only - see RE-EXW-MUSIC.md 2b); set =1 at config load (FUN_004252c0) | verified |
| 004ede10 | g_fade_ticks_left: palette-fade step countdown (NOT a frame gate; D15) | verified |
| 004eddc4/c8 | scroll x/y (clamped 9..631 / 9..463) | verified |
| 004eddf8..004ede04 | direction-filtered scroll copies | verified |
| 004edd7c | palette table base pointer | verified use |
| 004ee9b8 | g_dd_obj: IDirectDraw object (stock vtable) | verified |
| 004ee9bc/c0/c8/cc | g_dd surfaces 1..4 (roles not yet distinguished; +8-shifted vtable past GetCaps) | verified slots / inferred roles |
| 004ee9d0 | g_dd_palette: IDirectDrawPalette (stock vtable; +0x18 SetEntries) | verified |
| 004ee9d4 | g_dd_clipper: IDirectDrawClipper (stock vtable; +0x20 SetHWnd) | verified |
| 004edc38 | g_fade_state_16_16: 768 x (cur,step) 16.16 fade accumulators | verified |
| 004edc3c | g_fade_palette_6bit: 768-byte 6-bit RGB destination of FadeStep | verified |
| 004ee9f4/004ee9f8 | PALETTEENTRY array (r,g,b,flags stride 4; flags=1) / entries[1] ptr used by AppActivate SetEntries | verified |
| 004ee9d6 | state gate: hi word == 1 enables mouse poll | inferred |
| 004ee9e8/ec/f0 | surface pitch / pitch-0x280 / pitch>>2 | verified |
| 004ee9f4 | current palette RGB source (screenshot) | inferred |
| 004eedf6/df8 | clamped cursor y / busy-flag word | verified |
| 004eee5a | state gate: hi word == 0 allows mouse poll | inferred |
| 004ef66c | screenshot filename counter | inferred |
| 004ef708 | word zeroed on each mouse-poll tick | verified use |

## tick2 findings (2026-08-17, dumps exw-tick2.txt / exw-tick2-names.txt)

### Palette fade engine [all verified]

```
FadeSetup@0041cbf0(target_pal_ptr, steps):
    reads 768 target bytes (from ptr+2) + current palette (FUN_0044b040);
    per channel: state[i].cur = current << 8 (16.16 fixed point),
                 state[i].step = signed (target-current)*256 / steps;
    g_fade_ticks_left (004ede10) = steps   [also clears it first]
FadeStep@00425901:                        // fired by FUN_00402b0c at 50Hz
    for i in 0..0x2FF: state.cur += state.step; out[i] = cur >> 8
    SetPaletteRGB(out, 0, 0x100)          // upload all 256 entries
    g_fade_ticks_left--
SetPaletteRGB@0044aed4(bytes, start, count):
    entries[j] = {r<<2, g<<2, b<<2, flags} into 004ee9f4 array;
    surface IsLost/Restore(+0x6c) then SetPalette(+0x7c) if lost;
    windowed: Unlock(+0x80) on backbuffer-ish 004ee9c0;
    palette SetEntries(+0x18 on 004ee9d0), retry once if it failed;
    re-check IsLost, restore again if lost.
```
- Call sites: GameMain FadeSetup(pal@004edbf8, 10) after zone/level
  transitions (200 ms fades); FUN_0041e19d arms it from an EDX arg when
  releasing the timer at boot; FUN_00420100 cancels (=0) on screen change.
- The 768-byte palette buffer 004edbf8 (0x302 bytes total: header + rgb) is
  also the file image of .PAL files (header 2 bytes + 0x300 rgb) [inferred
  from FadeSetup reading target at +2].
- FUN_0044b040 = get-current-palette helper [inferred from use].

### DDRAW init/shutdown chain [verified]

- DDCreate@0044a5f0: DirectDrawCreate -> 004ee9b8 (g_dd_obj); error codes
  0x3e9/0x3f2; then SetCooperativeLevel check via +0x50 (stock layout).
- DDInitSurfaces@0044a660(w,h): fullscreen vs windowed (_004ef6a0 flag):
  SetDisplayMode(+0x54) / GetDisplayMode(+0x30); releases old surfaces;
  CreateSurface(+0x18) -> 004ee9bc/c0/c8/cc; windowed: CreatePalette(+0x14)
  -> 004ee9d0, clipper CreateClipper(+0x10) -> 004ee9d4, SetHWnd(+0x20 on
  clipper), SetClipper(+0x70 on surface 004ee9bc); builds the initial
  256-entry palette at 004ee9f4 (black/flags=1 fullscreen, white entry 0 +
  flags=1 windowed); RC_PALETTE handling mirrors AppActivate.
- DDShutdown@0044ab54: RestoreDisplayMode(+0x4c), FlipToGDISurface(+0x28),
  Sleep(500), Release(+8) on all seven object slots, clear 004ef676 hi.
- Object slots: 004ee9b8=dd obj, 004ee9bc/c0/c8/cc=surfaces (roles of the
  four not yet distinguished), 004ee9d0=palette, 004ee9d4=clipper.
- Vtable layouts: dd obj / palette / clipper = stock COM; surfaces = stock
  up to GetCaps(+0x30) then +8 shifted (2 extra slots in the game ddraw.h,
  see MousePosHandler note above). Irrelevant for reimplementation (we use
  real ddraw semantics); matters only for RE reading.

### Thread spawn slot resolved [verified]

00457874 (g_thread_spawn_slot) initial value = **0x0045204b =
ThreadSpawnImpl** - the statically linked **Watcom CRT thread-start helper**
(a _beginthread-style wrapper, not an IAT import): allocates a 16-byte
thread info block (FUN_0044f237(0x10)), stores args + GetCurrentThread
handle + page-rounded stack ((stack+0xfff)&~0xfff), then calls the REAL
KERNEL32 CreateThread (via IAT thunk 00452f36) with start routine
**0x00451fbc** (the Watcom CRT per-thread init trampoline that eventually
reaches GameThread@0044dea0). Chain: GameThreadStart -> ThreadSpawnThunk
00450242 -> [00457874] -> ThreadSpawnImpl 0045204b -> CreateThread(00451fbc).
Note: LAB_00451fbc is not yet a function in the project (future run could
create + decompile it to close the trampoline).

### Bonus: GoFlagSet caller found [verified]

FUN_0041e19d (called by GameMain right after LoadFile(LANGUAGE.*) at boot)
is the release-the-timer routine and ends with GoFlagSet() - this closes
gamethread doc open item 1. It also zeroes divider 004edbc8 there.

## Open questions / next steps

1. DONE 2026-08-17 (gamethread run, see docs/RE-EXW-GAMETHREAD.md): 0044dea0
   decompiled + named GameThread - a trampoline; the loop is GameMain@0041c050
   (also decompiled + named). Settled: no Sleep pacing (20fps refuted at this
   depth), zone/level strides (7x5, mission 1..26 via (zone-2)*5+level-1),
   RNG seeds 004ede48=123456/004ede4c=234567, go-flag writer set
   {GameThreadStart, GoFlagSet}. GoFlagSet CALLER found in the tick2 run:
   FUN_0041e19d (see above). Open remainder moved to that doc:
   FUN_0043d00b/FUN_00440e45 bodies (per-frame sim/render; NOTE the pacing
   question REOPENED by D15 - 004ede10 is a fade countdown, so the actual
   sim/render rate mechanism is unknown).
2. DONE 2026-08-17 (music run, see docs/RE-EXW-MUSIC.md 2b): FUN_00402bac =
   MusicPump, song slot 3 only.
3. DONE 2026-08-17 (tick2 run): FUN_00425901 = FadeStep (palette fade
   stepper, 50Hz while fading); FUN_0044b428 = CursorToGame (cursor ->
   640x480 game coords; scroll source is the mapped cursor).
4. DONE 2026-08-17 (tick2 run): 00457874 -> ThreadSpawnImpl@0045204b =
   Watcom CRT _beginthread-style helper wrapping the real CreateThread
   (CRT trampoline 00451fbc).
5. DONE 2026-08-17 (tick2 run): +0x18 in AppActivate is on the PALETTE
   object (004ee9d0) = stock IDirectDrawPalette::SetEntries (5-arg call:
   0, 0, 0xFE, &entries[1]). Residual: the +8 surface-vtable shift past
   GetCaps is documented empirically above (2 extra slots in the game
   ddraw.h); naming them is cosmetic - not blocking anything.
