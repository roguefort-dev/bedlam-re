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

### FUN_00402b0c: tick counters and the 50Hz gate [verified structure]
Increments five free-running counters each tick: @004edb84, @004edbc8,
@004edbcc, @004edba4, @004edba8. Calls FUN_00402b48 every tick. When
`(ctr@004edbc8 & 1) && dword@004ede10` -> calls FUN_00425901: a
**50Hz-gated update** (every other 100Hz tick) [inferred rate; body of
FUN_00425901 not yet decompiled]. Counter @004edbc8 doubles as divider:
bit0 -> 50Hz gate, bits0..2 -> palette cycle phase (12.5Hz) [inferred].

### FUN_00425ab9: scroll/camera update [verified structure]
- snapshot: dword@004eddcc = dword@004dc6e4 (live input direction flags
  [inferred]); bit0/bit1 drive x/y handling.
- FUN_0044b428(&x,&y) [hyp: accumulated scroll deltas from key/mouse input].
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
- DirectDraw surface vtable calls with standard IDirectDrawSurface offsets
  [mapping inferred, offsets verified]: +0x60 IsLost, +0x6C Restore when
  result == 0x887601C2 (DDERR_SURFACELOST), +0x14 Blt - on objects at
  *004ee9bc, *004ee9c8, *004ee9cc; helper FUN_0044b7b0 repeats IsLost/Restore
  and sets word@00457396 hi = 0xFFFF (cursor validity window [hyp]).
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
- **Worker thread body = 0x0044dea0** - instruction at 0044deca (inside that
  region, currently outside any function) writes 004ef674, the exact flag
  TimerInit spins on; FUN_0044d9b4 (10 bytes) also writes 004ef674
  [verified xrefs]. Region 0044dea0..0044dfec = next RE target (the actual
  sim/render loop, 20fps pacing claim to be settled there).

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
- On activate (arg==1) with DD object present: surface vtable +0x6C
  (Restore), +0x18, +0x7C (SetPalette) [verified calls; +0x18 mapping open].
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
| 004edbe0 | gates FUN_00402bac subsystem pump | verified use |
| 004ede10 | gates the 50Hz FUN_00425901 update | verified use |
| 004eddc4/c8 | scroll x/y (clamped 9..631 / 9..463) | verified |
| 004eddf8..004ede04 | direction-filtered scroll copies | verified |
| 004edd7c | palette table base pointer | verified use |
| 004ee9b0..004ee9d0 | DirectDraw surface/palette object pointer array (vtables used at +0x14/+0x18/+0x60/+0x64/+0x6c/+0x7c/+0x80) | inferred mapping |
| 004ee9d6 | state gate: hi word == 1 enables mouse poll | inferred |
| 004ee9e8/ec/f0 | surface pitch / pitch-0x280 / pitch>>2 | verified |
| 004ee9f4 | current palette RGB source (screenshot) | inferred |
| 004eedf6/df8 | clamped cursor y / busy-flag word | verified |
| 004eee5a | state gate: hi word == 0 allows mouse poll | inferred |
| 004ef66c | screenshot filename counter | inferred |
| 004ef708 | word zeroed on each mouse-poll tick | verified use |

## Open questions / next steps

1. **TOP: decompile 0044dea0..0044dfec** (worker thread body): create the
   function, dump decompile + callees. Expect: sim/render pacing (20fps claim),
   main state machine strides, RNG chain entry, who calls FUN_0044d9b4 / sets
   004ef674 = 1.
2. FUN_00402bac: gated pump over 20 slots x 38-byte records (base ~0x45b020),
   channel 3 only, FUN_0044c480(id) fires entries; DAT_0046ae78 flag.
   [hyp: sound/event scheduler]
3. FUN_00425901 (50Hz update) + FUN_0044b428 (scroll delta source).
4. Resolve the .data slot 00457874 (CreateThread vs Watcom _beginthread).
5. IDirectDrawSurface vtable +0x18 in AppActivate.
