# RE-EXW-PACER - EXW sim/render rate mechanism (present-paced, no software frame clock)

Run 2026-08-17 (GameMain second hop + pacer passes). Evidence dumps:
ghidra-project/exw-gamemainhop.txt (mission loop + first-hop decompiles),
ghidra-project/exw-pacer.txt (loop-body callees + timing-import census),
ghidra-project/exw-pacer-names.txt (0044ac5c + 0043e7d4 bodies + names applied).
Scripts: tools/ghidra-scripts/ExwGameMainHop.java, ExwPacerFollowup.java,
ExwPacerNames.java (3 x -process passes on BedlamWatcom, no import).
Closes the D15 open item (sim/render rate UNKNOWN); verdict = DECISIONS D16.

## 1. The mission loop [verified - exw-gamemainhop.txt FUN_0043d00b lines 333-462]

    while (DAT_0046cbe4 == 0 || _g_fade_ticks_left != 0) {   // until exit chosen + fade done
        FUN_00425ab9();                    // poll: input/cursor/music, non-blocking
        if (cinematics) { ...SmackDoFrame/NextFrame; exit-handling; }
        ...UI hit tests (exit buttons, 24 hotspots @ 0x4e9628)...
        MemCopy(0x4b000);                  // FUN_00402aaa arg = 307200 = 640*480
        if (in mission play) {
            AnimSprites();                 // FUN_0043f5b1: 24 entries x 0xe stride
            if (blit queued) FUN_00402a56();   // row blit into staging
            DrawOverlays();                // FUN_0043fb80: 15+15 text overlays
            AnimEntities();                // FUN_0043f68d: 300 entries x 0xc stride
        }
        PresentCopy(framebuf);             // FUN_00425a1e
        g_frame_count++;                   // DAT_0046ae68 - ONE per loop pass
        PresentEnd();                      // FUN_00425a03
        while (cinematics) _SmackWait();   // Smaker paces ONLY this path
    }

No Sleep / WaitForSingleObject / counter gate anywhere in the body. The loop
rate IS the frame rate (g_frame_count++ exactly once per pass), so whatever
bounds the loop bounds the frames.

## 2. The present chain [verified - exw-pacer.txt]

- MemCopy@00402aaa: 14-byte rep movsb/movsd (Watcom inline). NOT a pacer;
  PresentCopy calls it 480x (one per 640-byte row) + 1x with 0x4b000.
- SurfaceLock@00425a8b:  while (_g_surface_locked == 0)
      _g_surface_locked = FUN_0044ac5c();     // spin until success
- FUN_0044ac5c (unnamed; LockStaging) [verified - exw-pacer-names.txt]:
      if (ddraw active && fullscreen) {
          r = surf_A->vt[0x60]();             // IsLost
          if (r == DDERR_SURFACELOST 0x887601C2) surf_A->vt[0x6c]();  // Restore
          r = surf_B->vt[0x64]();             // Lock (staging)
          if (r == DD_OK) { cache ptr/pitch/ptr-0x280 in 004ee9e8..f0;
                             return lpSurface; }   // non-zero -> spin exits
      }
      return 0;
- PresentCopy@00425a1e = SurfaceLock + 480 row MemCopies + SurfaceUnlock
  (FUN_00425aa0 -> DDSurfaceUnlock@0044acf4 = staging->vt[0x80] Unlock).
- PresentEnd@00425a03 = SurfaceUnlock + DDFlipOrBlt.
- DDFlipOrBlt@0044ad18 [verified]: fullscreen -> surf->vt[0x2c] Flip;
  windowed -> surf->vt[0x14] Blt; hw-cursor handshake (spin on word
  004eedf8 lo==1, set 004eedfa) + GetCursorPos/MousePosHandler re-anchor;
  SetPalette vt[0x7c] when pending; guarded by g_presenting (004eee5c).

## 3. Rate verdict [verified census + inference -> D16]

- Sleep: exactly ONE caller in the binary - wrapper FUN_0044e1ca (11 bytes);
  used by shutdown paths (e.g. DDShutdown Sleep(500)), never in the loop.
- WaitForSingleObject: exactly ONE caller - FUN_00451b62 = Watcom CRT
  recursive mutex (CreateMutexA + owner-tid + recursion count + self-recursion
  via global 004ef8cc). Runtime locking, not pacing.
- 004ede10 = fade countdown only (D15). 004edbcc waits (FUN_0043a5fc, 5x)
  = attract-mode INPUT waits: spin on FUN_00425ab9 until input flag
  004edb50/004eddcc or 2000-tick (20 s @100Hz) timeout. Not a frame pacer.
- 00448ef1 reads of divider 004edbc8 (4x) = change-detection snapshots on a
  menu/high-score screen (HEREIAM string), not a rate gate.
=> The ONLY blocking edges per frame are the DirectDraw present calls:
SurfaceLock spins until Lock succeeds and DDFlipOrBlt issues Flip/Blt with
wait semantics. Stock DirectDraw completes a flip at the vertical blank, and
Lock on a busy flipped surface returns DDERR_WASSTILLDRAWING until the flip
retires. Therefore: **one sim/render frame per display flip = vsync-locked,
no software frame clock.** Frame rate = monitor refresh of the era (60Hz
class; the game imposes nothing else). Cinematics are additionally paced by
_SmackWait (Smacker internal frame timing).

## 4. Surface vtable layout CORRECTION (supersedes the tick2 +8 note)

Uniform **+4 = ONE extra slot at 0x0c** for the whole IDirectDrawSurface
vtable, verified on 9 anchors: Blt@0x14 (stock 0x10), Flip@0x2c (0x28),
IsLost@0x60 (0x5c), Lock@0x64 (0x60), Restore@0x6c (0x68), SetClipper@0x70
(0x6c), SetPalette@0x7c (0x78), Unlock@0x80 (0x7c), UpdateOverlay-era
DDSurfaceUnlock@0x80 mapping consistent. The tick2 claim "stock up to
GetCaps then +8 / 2 extra slots" came from a stock list already shifted one
slot (its "GetCaps@0x30" is stock GetBltStatus; its "Blt@0x14 stock" is
already +4). DirectDraw OBJECT / palette / clipper vtables are STOCK,
confirmed on 8 anchors (SetDisplayMode 0x54, GetDisplayMode 0x30,
CreateSurface 0x18, CreatePalette 0x14, CreateClipper 0x10,
FlipToGDISurface 0x28, RestoreDisplayMode 0x4c, palette SetEntries 0x18).
Irrelevant for reimplementation (real ddraw semantics); matters for RE
reading.

## 5. Names persisted this run (exw-pacer-names.txt NAMES_APPLIED)

MemCopy, SurfaceLock, SurfaceUnlock, PresentCopy, PresentEnd, DDFlipOrBlt,
DDSurfaceUnlock, AnimSprites, AnimEntities, DrawOverlays, PlayClockTick
(00402b48: hh:mm:ss divider from 100Hz ticks), GameGoRelease (0041e19d),
g_frame_count (0046ae68), g_surface_locked (004edb3c), g_input_seen
(004edb50), g_presenting (004eee5c). FUN_0044ac5c left unnamed by design
("name only if listing shows vtable call" - it does; cosmetic follow-up).

## 6. Residuals / next

- FUN_0043e7d4 (g_frame_count consumer, 1000+ lines) dumped but not yet
  analyzed - likely FPS/debug overlay or demo timing [hyp].
- FUN_00425ab9 (poll) body not yet decomposed (input/cursor pump hypothesis
  stands; spec-doc input/control map will need it).
- FUN_00440e45 zone/level manager decompiled in exw-gamemainhop.txt
  (lines 500-1498) but not yet written up - fold into the mission/progress
  spec doc when P4 starts.
