# RE-EXW-CAMERA — where the EXW camera/scroll state lives in the frame path

Provenance: synthesis unit for the P6 `p6-high-refresh-interpolation`
present-quality work (PLAN §6 "time-based simulation ... the frame is
composed from latest state + camera/scroll interpolation"). Every fact
below is RE-ANCHORED to EXW/EXD addresses and CROSS-REFERENCED to the
committed section that owns the verification; nothing here is new
disassembly — this file collects the camera/scroll traffic in one place
and records the one NEW claim (§4, the no-interpolation negative) that
the modernization policy rests on. Confidence: [verified] where cited;
§4's negative is scoped to the decoded frame path, not the whole binary.

## 1. The camera state cells

The mission camera is the Q5 fixed-point pixel pair:

| Cell | Meaning | Verification owner |
|---|---|---|
| `_DAT_004edde4` | camX, Q5 (1 unit = 1/32 px) | RE-EXW-MISSIONVIEW §3 "Camera: `_DAT_004edde4/8` are Q5 pixel cams; camTileX/Y = `>>5` stored to `_DAT_004ddb24/28`" [verified] |
| `_DAT_004edde8` | camY, Q5 | same |

The camera's INPUT (the scroll source) is the cursor pair
`g_cursor_x/y` @`0x4eddc4`/`0x4eddc8`:

- EXW: `ScrollUpdate@0x425ab9` runs per 100 Hz tick (TickWorker
  @`0041bfb6`, RE-EXW-TICK "TickWorker" listing), maps the absolute
  window cursor through `CursorToGame@0x44b428` and clamps into
  **[9,631]×[9,463]** of the 640×480 game space
  (`+9` margin, `<9→9`, `>0x277→0x277`, `>0x1cf→0x1cf`
  @`0x425b2e..0x425b84`) [verified; RE-EXW-TICK §FUN_00425ab9 +
  bedlam-core `frame.rs` CURSOR_* docs, S0-17/D160].
- EXD twin: the INT 33h AX=000B mickeys poll handler
  @`0x12615..0x12659` integrates then clamps the SAME box, storing
  `[0x1074b0]`(X)/`[0x1074ac]`(Y) [verified; RE-EXD-MAP §5h via
  D160].

## 2. Writers — every camera write lands on a tick boundary

1. **Camera recenter (the scroll integrator)**: the `robots()` tick
   block @`0x40b875..0x40b8c5` integrates the Q5 pair toward the
   cursor: gate `[0x4edbd8]` (ACTIONPAN registry flag, default 1)
   @`0x40b875`, chase-override gate `[0x4de654]≠0` @`0x40b885`, then
   `new camera target += (cursor−240)·v/480` per axis
   @`0x40b89e`/`0x40b8c5` — `v` = the zoom cell `[0x4ede54]`
   (240..480 backbuffer rows): full-speed at zoom-out, half-speed at
   max zoom-in [verified; RE-EXW-SIM §7j.56 items 2 + the §7j.56 zoom
   census row].
2. **Chase-camera override**: `FUN_004245c9(x,y,z)` stages
   `{x,y,z}` → `0x4de648/4c/50` + countdown `0xF` → `0x4de654`; while
   the countdown runs, the camera-point ring slot
   (`0x4c71c4/c8/cc`, 4-slot ring `[0x46ccdc]`) loads the staged
   triple instead of the selected robot's position, decremented per
   FRAME by the renderer head @`0x4039b0..0x403a42` [verified;
   RE-EXW-SIM §7j.54 item 6 + ledger row]. Four callers, all
   "look-at-me" cuts (door/section stepper `0x422427`, delayed-trigger
   expiry `0x422e55`, artillery spotter `0x41173a`, bombardment
   record-0 impact `0x423ed5`).
3. **Scene activation**: the camera pair points at the first robot's
   Q5 spawn on mission staging [verified; RE-EXW-SIM §7j.20 anchor
   bank + DESIGN-GAME sec 11 LIFECYCLE note].

## 3. Readers — the frame path consumes the CURRENT camera value

- **Per-frame viewport renderer** `FUN_00403938` [verified core loop,
  RE-EXW-MISSIONVIEW §3]: `camTileX/Y = cam>>5` → `_DAT_004ddb24/28`;
  every terrain/sprite placement subtracts the camera (terrain via
  the 36×36 viewport cache `dtile` deltas; sprites via the shared
  `dx = (x d@+0x12 >>8) − [0x4edde4]` iso math, RE-EXW-SIM §7j shared
  math item 8); shake adds `shake*0x280`/`shake` on top.
- **Present** `FUN_00401107` [verified, RE-EXW-MISSIONVIEW §7]: the
  480×480 source window sits at backbuffer `+0xa040` plus the
  fine-camera offset `colAdj = ((camX&0x1f)−(camY&0x1f)+0x20)&0x3f`,
  `rowAdj = ((camX&0x1f)+(camY&0x1f))>>1`; the zoom/iris path
  (`FUN_004012f7`/`FUN_004013e8`) is a display-side Q16 magnifier of
  the already-rendered backbuffer, never a camera mover
  (§7j.56/§7j.58).
- **Non-render consumers** (same cells, no frame-path role): the
  click unproject (`world_x = cam + sx + sy`, RE-EXW-SIM map-click
  arm) and the SFX pan/vol listener (`FUN_0043a3e0`/`FUN_0044a447`
  vs `0x4edde4/0x4edde8`).

## 4. The negative fact: NO sub-tick camera interpolation exists

[verified by absence in the decoded frame path — scope: the loop/
render/present/tick functions decoded across RE-EXW-PACER,
RE-EXW-TICK, RE-EXW-MISSIONVIEW, RE-EXW-SIM]

Every writer above fires on a tick boundary (TickWorker 100 Hz
service; `robots()` per sim tick; staging on scene entry), and every
frame-path reader reads the CURRENT cell value at composition time.
Under the original frame-locked pacing (RE-EXW-PACER §3 [verified]:
one sim/render frame per display flip — the `FUN_0043d00b` loop pass
and its PresentEnd are ONE event, `g_frame_count++` exactly once per
flip) there is no moment at which a frame is composed between two
camera states, and no code path computes one:

- the Q5 fixed point is sub-PIXEL precision WITHIN integer tick
  updates — not sub-TICK interpolation;
- the `FUN_00401107` zoom path's Q16 magnifier scales an
  already-rendered backbuffer (a display scaler), it never blends two
  camera positions;
- the only "camera cut" mechanism (§2 item 2) is a hard override with
  a 15-frame countdown, not a blend.

So the original's camera is exactly "the latest tick's value,
presented once per flip". Any between-ticks camera position is by
construction a modernization manufacture.

## 5. Consequence for the reimplementation (the P6 policy)

PLAN §6 "time-based simulation" scopes the modern decoupled present's
composition policy exactly against §4:

- **Interpolate the CAMERA/SCROLL ONLY**, between the state at the
  last executed logic tick and the present, parameterized by the
  accumulator fraction of the pending tick (the shell
  `FixedStepClock` bank). Implemented as the one-tick-behind blend
  `lerp(prev_tick_camera, cur_camera, alpha)` in bedlam-render
  `compose.rs::camera_for` (D12 contract, integer-grid-quantized,
  scroll-bounds-clamped) — this MANUFACTURES camera positions the
  original never showed (deliberate, budgeted T2/T3 divergence) and
  is presentation-bucket state (D17 b): it can never reach the sim,
  the state hash or the scene hash.
- **NEVER interpolate sprite positions**: the 1996 sprites are
  grid-quantized and had no sub-pixel positions (§4); interpolating
  them manufactures motion the original never showed. Sub-pixel
  blitting stays a default-off presentation option (PLAN §6,
  DESIGN-RENDER sec 9).
- **Classic arm unchanged**: the frame-locked pacing (D203) presents
  only after a tick executes, so the presented image is exactly the
  tick-state camera — the §4 shape, nothing to interpolate.

## 6. Open items

None for this unit. (The `robots()` recenter integration step size
per tick and the Q5 chase-cam ring details are §7j.54/§7j.56 material
and stay owned there; this file only needed the frame-path traffic.)
