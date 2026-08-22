# RE: BEDLAM.EXW — mission-sim tail (P2d slice: one squad-member move)

Scope: the EXW gameplay loop's order-input path, movement grid/pathing, and
per-tick mover state — the sim tail the P4 vertical slice (ZONEA/MISSION1
render + one squad move) needs.

Provenance: Ghidra headless `-process BEDLAM.EXW -noanalysis` + postScript
`tools/ghidra-scripts/ExwSimTail.java` (raw dump `ghidra-project/exw-simtail.txt`,
log `ghidra-project/process-exw-simtail.log`, both gitignored). Function
targets were LOCATED with the 8street disasm (navigation reference ONLY,
per AGENTS.md; label VAs like `loc_40C594` are EXW VAs because that listing
is an IDA export of this same binary) and every fact below was then read
out of the EXW decompile/listing itself — the 8street gloss appears only
as cross-check notes. objdump anchors are from game-data/BEDLAM/BEDLAM.EXW
directly. Tags: [verified] = read in EXW decompile/listing/objdump;
[inferred] = strong deduction; [hypothesis] = plausible, unconfirmed.

## 1. The gameplay loop is MissionShell@0044771c (8street "game_level")

GameMain calls it in the episode loop; GameMain's outcome switch keys on
its return (1 = mission complete via DAT_0046ccc4 countdown; 2 = quit-ish
path; the function was already named MissionShell in the Ghidra project).
This CORRECTS the RE-EXW-GAMETHREAD gloss "FUN_0044771c = music/ambience
select" — it is the per-frame gameplay shell [verified: full decompile].
FUN_00440e45 (10661 B, also called by GameMain) remains unidentified —
NOT the gameplay loop [verified negative: 8street's game_level call lands
at 0x44771c via label fit].

Structure [verified decompile, order as written]:

```c
MissionShell(EAX, EDX) {
  seed: _DAT_004ede48 = 0x1e240 (123456)            // RNG A reseeded per mission
  loading-screen assets (LOAD_{UK,US}.BIN + LOADPAL(U) + FULLFONT),
  FadeSetup(10), two loading-text draws + PresentEnd
  MusicStop(3); FUN_0043a1d3(); FUN_0041d954()      // arena alloc pass (below)
  // ~40 state resets: DAT_004ddb20=0 (input-delta bits), selection
  // DAT_0046cbd4/0046cbdc, click flags 004eabb0-family, freeze
  // _DAT_004eaac0=0, cursor angle 004dc678=0, latches cleared
  FUN_0040cca0();                                    // load_markers_mrk_file (spawn)
  ...
  for (;;) {                                         // main loop, PresentEnd-paced
    volume keys; input_seen handling
    FUN_00402965() /*tick service*/; ...anim/pacer helpers...
    FUN_0041fbb1();                                  // dropship/anim update
    FUN_0040b835();                                  // mouse_l_click  (input)
    FUN_00410644(); FUN_00449c94(1); FUN_00409138();
    for (phase = 0; phase < 6; phase++)
      FUN_0040b9f6(phase);                           // robots() — SIX phases/frame
    for (i = 0; i < 4; i++) { FUN_00410823(i); FUN_00412010(); if (i&1) FUN_004197d4(); }
                                                     // enemy pass ×4
    ...draw/UI chain... FUN_00425010() ...
    // order validity timer: if (_DAT_004eabb0 != 0) { if (_DAT_004eabb2 != 0)
    //   _DAT_004eabb2--; if timer==0 or all robots state-3/dead -> FUN_0041faf0() }
    if (!(_g_latch_P && !_DAT_004edb88)) PresentEnd();  // present/flip = vblank pace
    else { pause: draw PAUSED, PresentEnd, spin on _g_latch_P; }
    g_frame_count++;
  }
}
```

Pacing: NO Sleep/divider on this loop — the frame is PresentEnd-paced
(DDRAW flip), the same present-paced mission-loop architecture as B2 (D16)
[verified absence + verified PresentEnd tail]. **The sim sub-tick count is
6 per frame** (robots() × 6 phases), phase-gated per robot (see §5) — this
is the concrete number for the parity budget's logic-rate question at this
depth [verified call loop; interpretation of the gate below].

FUN_0041d954 [verified] = the arena pass that allocates the mission
buffers: `DAT_004edd60` = 0x20788 (CGR), `_DAT_004edd58` = 0x13884 (DAT),
`_DAT_004ede20` = 0x27104 (TOT), `_DAT_004ede24` = 0x3cc0 (viewport tile
cache), `_DAT_004ede18` = 0x64000 (tile render buffer). The file-load +
table-build pass (y-line table 0x4ea900, z-base table 0x4eaacc, map sizes
004eddec/004eddf0) is a separate function not yet decoded — open item,
input for the P4 render slice.

## 2. Coordinate systems [verified]

- **World position: Q13** — 0x2000 (8192) units per tile. `tile = pos >> 13`
  (used at robots() 0x40bd-style reads and the arrive snap).
- **Sub-tile Q5** — 32 units per tile; `q5 = pos >> 8`. All
  move_is_possible/get_z_pos input is Q5; `tile = q5 >> 5`.
- **World z: Q5** — `level*0x20 + sprite_height_byte` (0..8 levels).
- **Robot spawn from MRK** [verified 0x40cfe9 region]: `pos_x = mrk.x*0x2000
  + 0xF00`, `pos_y = mrk.y*0x2000 + 0xF00` (0xF00 = 15/32-tile center
  offset), `pos_z = mrk.z*0x20 - 1`; the 8-word probe-z cache (+0x1A..) is
  seeded with pos_z; then one `move_is_possible(pos>>8, ...)` settles the
  floor. (Anchors the 8street hint exactly.)

## 3. The robot record: base 0x4c69e4, stride 0xA8, count DAT_0046ccbc [verified]

Fields pinned this pass (offsets from 0x4c69e4 + idx*0xA8):

| off | type | meaning | evidence |
|---|---|---|---|
| +0x00 | i32 | pos_x (Q13) | robot_move/load_markers |
| +0x04 | i32 | pos_y (Q13) | robot_move/load_markers |
| +0x08 | i32 | pos_z / floor z (Q5, clamped 0..0xFF, mutated by move_is_possible) | 0x4c69ec |
| +0x0C | u16 | state (0 idle?, 2 selected?, 3 ordered/waiting, 4 moving, 5/6 dying; 3/5 skip robot_move) | 0x4c69f0 |
| +0x0E | u16 | last dir byte used (copied from facing each move) | 0x4c69f2 |
| +0x10 | u16 | facing/direction: 0x00 N, 0x40 E, 0x80 S, 0xC0 W, 0xFFFF none | 0x4c69f4 |
| +0x12 | u16 | anim phase = ((angle_byte+4)&0xFF)>>3 (0..31 walk sectors) | 0x4c69f6 |
| +0x14 | u16 | frame-base word for the viewport overlay sprites (DANTE frames `base+0x20` and `base*3+…+0x40`); zero-filled at spawn [MISSIONVIEW §5d] | 0x4c69f8 |
| +0x16 | u16 | deploy countdown (0xFFFF when spent; decrements phase 0; gates the +0x40 overlay) — this row was previously mislabeled +0x14 [corrected 2026-08-21, MISSIONVIEW §5d] | 0x4c69fa |
| +0x18 | u16 | random at spawn: RandA()&3 (variant) | 0x4c69fc |
| +0x1A..+0x29 | 8×u16 | per-probe floor z cache (written by move_is_possible; +0x1A doubles as the climb-compare z: `dword@+0x18 >> 16`) | 0x4c69fe |
| +0x2A | u16 | robot TYPE (indexes the 0x62-stride ORDER/stats table at 0x4de664; all player robots take the global word@0x4edb90) | via 0x4c6a0c>>16, 0x40cdf3 |
| +0x2C | u16 | DROP-POD descent timer: ≠0 freezes the whole robot brain per sub-tick (FUN_0040b9f6); 0-hit → pod anim FUN_0041fb4b + msgs 9/10/0xB. Writers: spawn stagger `1+k·(2000−m·1000/27)` (FUN_0040cca0 @0x40d132), MP respawn 0x28 (FUN_0040e230 @0x40e89d) | 0x4c6a10, §7j.20 |
| +0x36/+0x38/+0x3A | u16×3 | per-order stats-group copy i (8-byte groups, i=0..6): word0 = group availability (spawn default probe), word1 = the sidebar order gate (copied twice) [§6c.6] | 0x4c6a1a/1c/1e, spawn 0x40cf05..0x40cf42 |
| +0x6E | u16 | ORDER BITS (bit i = order i active; bits 0..6 toggled by keys 1..7 / the 7 sidebar order rows; spawn default = 1 << first available) | 0x4c6a52, §6c |
| +0x70 | i32 | deploy-delay counter (vs table DAT_00454ee8[DAT_0046cbf8]) | 0x4c6a54 |
| +0x74 | i32 | stop distance for the active order (1000000 = go all the way) | 0x4c6a58 |
| +0x78 | i32 | (label corrected 2026-08-21, §6c.7: this row had drifted +4 — alive is +0x7C) — | — |
| +0x7C | i32 | alive flag (0 = slot free; sidebar select gate + armer's one-alive count) | 0x4c6a60 |
| +0x80 | i32 | countdown (decrements when ≠0; gates phases 4/5: serviced iff > phase*32) | 0x4c6a64 |
| +0x90 | i32 | dying countdown (states 5/6 → despawn/revive) | 0x4c6a74 |

Related globals [verified]: move-target arrays `DAT_0046cc30` (x) /
`DAT_0046cc60` (y), Q5, -1 = none; robots available `DAT_0046cbd8`
(zone<3 or 7 → 1, zone 3 → 2, else 3); selected = `DAT_0046cbd4` (group
base = player_id*robots_per_player) + `DAT_0046cbdc` (0..2 slot).

## 4. Movement grid & walkability [verified]

**move_is_possible@0041e897** (EAX = candidate x Q5, EDX = candidate y Q5,
EBX = robot idx; returns nonzero = pass, 0 = blocked):

- Walks **8 footprint probes**: static dword tables X[8]@0x4543e4,
  Y[8]@0x454404 [verified from the PE bytes, file off 0x529e4]:
  X = {-11,-11,+12,+12, 0, 0,-11,+12}, Y = {-11,+12,-11,+12,-11,+12, 0, 0}
  — the 4 corners + 4 edge midpoints of a 23×23 Q5 box around the robot.
- Per probe: candidate+off must satisfy `0 <= x`, `x>>5 < map_w
  (DAT_004eddec)`, `0 <= y`, `y>>5 < map_h (DAT_004eddf0)` [bounds];
  `floor_z = get_z_pos(candidate)`; climb check
  `|floor_z - robot_z| <= 4` else blocked. **The climb limit IS the wall
  test** — no separate solidity bit exists on this path.
- On pass: `robot+0x08 = get_z_pos(center, clamp(robot+0x08,0xFF))` and
  the 8 probe z's are cached to +0x1A.

**get_z_pos@0041e231** (EAX x Q5, EDX y Q5, EBX z Q5; clamps z 0..0xFF):

- `type = get_from_dat_file(x>>5, y>>5, z>>5)`; search order: z, then z+1
  (if z<7), then z-2 (i.e. z-1 after the +1 step), while type ∈ {0, 0x2A};
  if still empty → return 0.
- Type 3 latches the trigger triple `_DAT_004dc688/8c/90 = {z, x, y}`
  (consumed by robots() as a tile-effect site).
- Height: CGR bank `DAT_004edd60`, directory slot `(type-1)`, sprite byte
  at `(x&0x1F) + (y&0x1F)*0x20 + slot_offset + 6` — **the 32×32 CGR sprite
  bytes are the per-sub-tile height map**. Return `z_level*0x20 + byte`.
- Slope continuity at tile tops: byte == 0x1F and z<7 → probe z+1's byte;
  nonzero → `(z+1)*0x20 + byte`.

**get_from_dat_file@0041eb28** [verified asm]: `byte =
*(DAT_z_base_tbl[z]@0x4eaacc + DAT_y_line_tbl[y]@0x4ea900 + x)`; 0xFF
reads back as type 1. (Anchors the DAT addressing: plane-major u8 z-planes
+ row-offset table, exactly the FORMATS-MISSION §4 layout.)

Per-type runtime DB: 0x1E-stride tables at 0x4796bc (current variant word)
/ 0x4796cc (seen flag), mirrored from the TOT data by init_tiles@00407e11
[verified]; robots() consults `byte@(0x4796d4 + 0x1E*LINEAR_TILE_INDEX)`
for the armor pad check [CORRECTED §7g.3: the DB is one 0x1E record per
TILE — MISSIONVIEW §2 — and the index is the linear tile index, not the
type; the byte is the record +0x18 static-frame byte]. init_tiles also
builds the 36×36 ISO viewport tile cache
at DAT_004ede24 (12 B entries: screen offset + tile deltas).

## 5. robots@0040b9f6 — the per-tick unit manager (phase arg 0..5) [verified]

Per phase call (6×/frame from MissionShell), for each robot record:

- Timers decrement (fields +0x32/+0x34 (0x4c6a16/18), +0x9C (0x4c6a88),
  +0x88 (0x4c6a6c -= 2), +0xA0 (0x4c6a84 dying/flash with FadeSetup
  side effects — phase 0 block)).
- Body gate: `state's +0x2C countdown == 0` AND
  `(phase < 4) || (phase*32 < field_7C)` — i.e. phases 0..3 always run;
  phases 4/5 only while field_7C > 128/160 [verified expression;
  interpretation: field_7C is a drop/animation countdown that buys the
  extra sub-ticks — hypothesis].
- TOT tile-type specials: type 0x7d3 gates phase skips, 0x7d2 (phase 0)
  triggers FUN_0040e230(robot, 0xF, -1) [verified reads via 0x4ea900 +
  TOT mirror DAT_00460df8].
- Reinforcement ready: deploy-delay counter +0x70 vs
  DAT_00454ee8[DAT_0046cbf8] → slot SFX (0xC/0xD/0xE) + scatter of 8
  jittered markers into 0x4ea238 (10-byte records).
- **Order consumption** [verified, the click→move bridge]:
  `word@0x4eabb0 != 0` (order armed) and state ∉ {3,4,5} and
  `dist_octagonal(pos_q5, order_tile*0x20+0x10) < 0xC0` (6-tile radius)
  → `FUN_004248c8(&tx, &ty)` picks a free spread tile →
  `state = 4`, `field_0x74 = 1000000`, `target[idx] = {tx<<5, ty<<5}`.
- **Move toward target** (state ∈ {1,4}, target ≠ -1):
  ```
  Δx = target_x - (pos_x>>8); Δy likewise                 (Q5)
  dist = FUN_0041ebf8(Δx<<8, Δy<<8)                       (octagonal, Q13)
  dist_hi = max(dist>>8, 1); dist clamped [1, 0xFFFF]
  angle = fold64(atan_idx(|Δx|*0x80/dist_hi))             (0..0xFF byte)
  if (dist > field_0x74 || dist < 0x1400)  -> ARRIVE:
      state 4 -> 3; pos_x &= 0xFFFFE000; pos_y &= ...    (snap to tile grid)
      state 1 -> 0, field_0x74 = 0, targets = -1
  else robot_move(idx, ((Δx<<16)/dist)<<2, ((Δy<<16)/dist)<<2, angle)
  ```
  [verified decompile + asm 0x40bd90..0x40bf0b]. Pure-axis stride =
  0x400 Q13 = **1/8 tile per sub-tick**; diagonal ≈ 0x2AA per axis
  (octagonal normalization, idiv truncation toward zero).
- Helpers [verified objdump]: `FUN_0041ebf8` = `max(|dx|,|dy|) +
  min(|dx|,|dy|)/2` (octagonal distance); `FUN_0041eb7d` = 64-entry
  ascending-word threshold table at `*(0x46cbd0)+4` over ratio
  `|Δx|·0x80/dist_hi` → sector 0..0x3F; `FUN_0041ebc1` = quadrant fold
  (dx≥0,dy>0 → 0x7F-i; dx<0,dy≥0 → i+0x80; dx<0,dy≤0 → 0x100-i; else i)
  → 0..0x100 angle byte; cardinals N/E/S/W = 0x00/0x40/0x80/0xC0.
- Selected-robot cursor angle: `_DAT_004dc678 = (angle-0x1C)>>3` from
  cursor offset ((cursor-0xF0)*0x100, (cursor-0xE0)*0x80).

**robot_move@0040c536** (EAX robot, EDX dx Q13, EBX dy Q13, ECX angle
byte) [verified decompile + asm]:

- Skip states 3/5; freeze `_DAT_004eaac0 != 0` → state(+0x0C) = 0, return.
- Store angle byte at +0x0E; try the diagonal:
  `move_is_possible((pos_x+dx)>>8, (pos_y+dy)>>8, idx)` → on pass:
  pos += delta; facing = 0xFFFF; anim = ((angle+4)&0xFF)>>3; return.
- On block: if facing == 0xFFFF → pick a cardinal from the deltas by
  probing `move_is_possible2` (non-mutating: restores +0x08) with
  single-axis ±0x400 steps, preferring the sign of the larger requested
  axis, else the free perpendicular (writes the new facing 0x00/0x40/
  0x80/0xC0, or 0xFFFF if none).
- Then slide on the chosen facing: N/S facing → try (0,∓0x400), blocked →
  `move_x_who`; E/W facing → try (±0x400,0), blocked → `move_y_who`.
  **move_x_who@0040cac2 / move_y_who@0040cb4f**: single ±0x400 axis step
  via move_is_possible; on pass pos moves and facing is set to the axis
  cardinal (0xC0/0x40 resp. 0x00/0x80) [verified].
- Success tail: anim = ((angle+4)&0xFF)>>3; +0x0E = facing; facing =
  0xFFFF (consumed).

## 6. The order input path (squad select + destination) [verified]

1. **Input layer** latches the click at `_DAT_004eddf8/_DAT_004eddfc`
   (x/y, -1 when consumed) — the pair RE-EXW-GAMETHREAD saw reset to -1.
2. **mouse_l_click@0040b835** (called once per frame from MissionShell):
   x ≥ 0x1E0 → `sidebar_control@0040d197` (sidebar UI — full decode
   §6c: robot-select strips
   x∈[0x1E7,0x217)/[0x219,0x249)/[0x24B,0x27B) y∈[5,0x35], F1/F2/F3 +
   keys 1..7 order toggles, order rows x∈[0x1E9,0x275] y∈[0x57,0xB8],
   map toggle strip x∈[0x213,0x24D) y∈[0x1B4,0x1D0); writes selection
   DAT_0046cbdc, redraw flag DAT_0046ccec=2). Else (map viewport): **isometric unproject**
   ```
   sx = ((click_x - 0xF0) * viewport_h) / 0x1E0; sx >>= 1
   sy = ((click_y - 0xF0) * viewport_h) / 0x1E0 - 8 + _DAT_004edd54
   world_x = _DAT_004edde4 + sx + sy          (scroll x)
   world_y = _DAT_004edde8 + sy - sx          (scroll y)
   clamp [0, map_w*0x20] / [0, map_h*0x20]; DAT_004ddb20 |= 1
   ```
   then consumes the latch (-1). Keyboard latches also route to
   sidebar_control.
3. **DAT_004ddb20** = input-delta bitfield (bit0 click-pos, bit1 ?, bit2
   selection): serialized by FUN_00449c94 into the replay/network packet
   buffer (`DAT_004eba04` stream with a RandA()&0xF sequence marker) and
   cleared — the multiplayer input channel [verified 0x44a1df region].
4. **Beacon arm** [CORRECTED 2026-08-21 §7j.20 — provenance was wrong]:
   the arm is NOT a click path: **FUN_00433980 @0x433cfb (the zone
   pad-trigger script dispatcher, §7j.19 item 4)** calls
   **FUN_004247b5(pos_x>>13, pos_y>>13, z, robot_idx)** — i.e. the
   extraction beacon is armed AT THE TRIGGERING ROBOT'S TILE when it
   steps on one of the ~25 scripted (zone, .PAD slot) extraction pads
   (the old "~0x433cbc robot hit-test family" gloss mis-identified the
   enclosing function):
   ```
   if (word@0x4eabb0 != 0) return          // one beacon at a time
   word@0x4eabb2 = 0x197                    // dropship countdown (407)
   word@0x4eabb0 = 1                        // ARM
   if (alive-count of player-0 group == 1)  // last robot standing
       word@0x4eabb2 = 0                    // deploy NOW
   word@0x4eabb4/6/8 = tile trio            // x, y, z (z = dead store)
   robot[idx].state = 3                     // halt (sweepable)
   FUN_004248c8(&tx,&ty); robot.pos = (tx<<13, ty<<13)   // spread-assign
   SFX 0x2A
   ```
   Full decode + the trigger-chain closure: §7j.20.
   **FUN_004248c8** = spread/claim search: finds a free slot in the 12×u16
   claim array 0x4eabba, then a 12-case jumptable (table@0x424898) offsets
   the order tile by {0, ±1 on x, ±1 on y — the 8 neighbors + center
   variants} → each consumer gets a distinct destination tile near the
   click [verified asm 0x4248ca..0x424985].
5. **Consumption** (robots() §5): every robot within 6 tiles of the
   beacon tile (state ∉ {3,4,5}) gets state 4 + its own spread-tile
   target + stop distance 1e6 → walks per §5. The beacon expires via
   the MissionShell timer (or early once all robots are state-3/dead)
   → **FUN_0041faf0** clears 0x4eabb0/2 and stages the dropship
   animation target (0x4e6610 = the EXTRACTION dropship, §7j.19)
   [verified].
   Net effect [CORRECTED §7j.20]: stepping on a scripted extraction
   pad = the squad rallies at the pad (spread claims) until the
   countdown expires or the squad is halted, then the extraction
   dropship lands and sweeps them (states 3/4 → 5) — no click is
   involved anywhere in this family.

## 6c. sidebar_control@0040d197 — full decode (2026-08-21, worker 6ebe5cff)

Decompiled + asm-verified 0x40d197..0x40d712 (objdump), with xref
provenance from the new `tools/ghidra-scripts/XRefList.java`. This
corrects §6.2's imprecise gloss: the map-toggle strip writes
`_DAT_004eb8dc`/`_DAT_004edba0`, NOT the selection `DAT_0046cbdc`;
the selection write is the robot-select strip family below.

Entry: called from mouse_l_click@0040b835 when the click latch x ≥
0x1E0, and from the keyboard-latch path (the 12 latches,
RE-EXW-INPUT; keyboard wiring is deferred to the P2e button-map
slice on the engine side). The click latch `_DAT_004eddf8` is
consumed unconditionally at the tail (`= -1`, 0x40d70d).
`FUN_00424a6e` (called at every fire site) is an EMPTY STUB
[decompile 0x424a6e: bare ret] — a no-op.

1. **Map-toggle strip** [asm 0x40d19d..0x40d21a]: fires iff
   `_DAT_004edd8c ∉ {1,7}` AND `_DAT_004eb8dc == 0` AND (click
   x∈[0x213,0x24D], y∈[0x1B5,0x1CF] OR `_g_latch_MSpace@0x4edc08`).
   Action: `_DAT_004eb8dc = 5`; `_DAT_004edba0 = (old == 0)` (0↔1
   toggle); clear the MSpace latch. `_DAT_004edba0` is the map-overlay
   draw-mode bit: the terrain-loop tail runs `FUN_004089b1` when it is
   nonzero (0x4071d5) and the present window `FUN_00401107` reads it
   (0x40110c) — the strategic-map overlay family, NOT modeled in the
   engine slice. `_DAT_004edd8c` = the screen/mode global GameMain
   writes (values 1/7 gate the strip off).
2. **Robot-select strips** [asm 0x40d220..0x40d3b0]: y∈[5,0x35] all
   three; x∈[0x1E7,0x217] slot 0 (F1 latch@0x4edc0c, needs
   `DAT_0046cbd8 ≥ 1`), x∈[0x219,0x249] slot 1 (F2@0x4edc10, ≥ 2),
   x∈[0x24B,0x27B] slot 2 (F3@0x4edc14, ≥ 3). Gate: target slot's
   ALIVE dword `0x4c6a60 + 0xA8*(DAT_0046cbd4 + slot)` ≠ 0. Action:
   `DAT_0046cbdc = slot` (the squad slot), `DAT_0046ccec = 2`,
   `_DAT_004ede34 = 0`, `_DAT_004ea8f8 = 0`; the F-latch clears
   regardless of the alive gate. `_DAT_004ede34/_DAT_004ea8f8` are
   map-overlay/present aux globals (readers FUN_00401107,
   FUN_00403938, FUN_0044764c) — cleared on select, consumers not
   decoded this pass.
3. **Order keys 1..7** [asm 0x40d3b0..0x40d659]: latches
   0x4edc18+4*(k-1). Selected robot idx = `DAT_0046cbd4 +
   DAT_0046cbdc`. Gate: word@`0x4c6a1c + 0xA8*idx + 8*(k-1)` ≠ 0
   (record +0x38+8(k-1) — a per-type ORDER-AVAILABILITY word, see 6).
   Action: word@`0x4c6a52 + 0xA8*idx` (record +0x6E, the ORDER-BITS
   word) `^= 1 << (k-1)`; `DAT_0046ccec = 2`; latch clears always.
4. **Order-row click** [asm 0x40d659..0x40d712]: needs the click
   latch `_g_scroll_btn_latch@0x4ede14` AND x∈[0x1E9,0x275] AND
   y∈[0x57,0xB8]. `row = (y - 0x57) / 14` (idiv trunc), clamped to ≤ 6
   — 7 rows of 14 px starting at y=0x57 (0xB9-0x57 = 98 = 7*14
   exactly). Same gate/toggle as key row+1; the click latch clears
   ONLY when the availability gate passes; `DAT_0046ccec = 2`.
5. **Redraw flag semantics** [asm 0x4071ed..0x407217, in the
   FUN_00403938 draw tail]: `DAT_0046ccec` is a per-frame COUNTDOWN:
   when nonzero, decrement and call the sidebar redraw pass
   `FUN_00408403`. Producers: sidebar_control sets 2; the robot-death
   path FUN_00409138 sets 3 (0x40a483 region, alongside clearing the
   dead record's 7 order-gate words at 0x4c6a1c+8k — full sidebar
   refresh); MissionShell entry ZEROES both countdowns with the other
   mission-state globals (0x4478bf/0x4478c5). Sibling countdowns in
   the same tail: `0x46ccf0` → FUN_004085ce, `0x46ccf8` →
   FUN_00401ca2 (rect (0x12,1,0x1EE,0xC3)).
6. **Spawn-side order init** [asm 0x40ceb2 + 0x40cef1..0x40cf70, in
   load_markers' record init]: order bits word (+0x6E) starts 0, then
   the stats-copy loop runs 7 iterations (i = 0..6, stats byte offset
   0x0E*i, record word offset 8*i):
   ```
   type   = word@(+0x2A)                       // dword@+0x28 >> 16
   stats  = 0x4de664 + type*0x62               // the per-type stats table
   word@(record+0x36+8i) = word@(stats+0x0E*i)      // group word0
   word@(record+0x38+8i) = word@(stats+0x0E*i+2)    // group word1
   word@(record+0x3A+8i) = word@(stats+0x0E*i+2)    // word1 again
   if (!found && word@(record+0x36+8i) != 0) bits |= 1<<i; found=1
   ```
   i.e. **the 0x62-stride table at 0x4de664 is the 7×0x0E per-type
   ORDER table** (open item 5's stride now structurally explained:
   7 groups of 14 B); the default order bit is `1 << first i whose
   group word0 ≠ 0`. The select gate (2) uses the ALIVE word; the
   order gates (3/4) use group word1 (+0x38+8i). The table is .bss
   LIVE SESSION STATE — NOT file-loaded (amendment 7d REFUTES the
   TABLE.BIN hypothesis: TABLE.BIN is the map-overlay backdrop bank;
   the loadout is written only by shop/save/MP). Every player
   robot's TYPE comes from the one global word@0x4edb90, written 0
   once by GameMain@0x41c34c (SP; the MP lobby writes otherwise) —
   also read by the multiplayer lobby FUN_00448ef1 and the shell
   screens.
7. **Field-table offset correction**: rows +0x78/+0x7C drifted — the
   ALIVE flag is at 0x4c6a60 = **+0x7C** and the drop countdown at
   0x4c6a64 = **+0x80** (address column was right, offset column
   wrong). Now double-anchored: sidebar select gate (0x40d269/
   0x40d2ef/0x40d37b) and the armer's exactly-one-alive loop
   (0x424810, `cmpl $0, 0x4c6a60(%eax)` stepping 0xA8). The engine
   struct labels follow below.

Engine seam (this slice): mouse-only sidebar dispatch in
MissionScene::tick — select strips + order rows + the redraw
countdown (decrement in present) live on the PRESENTATION half
(D17 split: none of it enters the sim state hash). Per-robot order
availability defaults to all-7 [design: the type-table file source
is open]; per-robot order bits default `1 << first available`
[verified 6]. Squad = player-0 group (base 0), size
robots_per_player(zone). Keyboard latches + the map-toggle strip
stay unwired (button map P2e; overlay machinery open).

8. **The sidebar redraw pass FUN_00408403 + the sidebar art family**
   (2026-08-21, worker 49294e3c; decompile + full objdump
   0x408403..0x4085c6, banks verified against the shipped bytes).

   a. **FUN_00408403 = the 7 order-row draw** [asm 0x408403..0x4085c6]:
   loops i = 0..6 over the SELECTED robot's record (base
   `(DAT_0046cbd4 + DAT_0046cbdc) * 0xA8`, group cursor `+0x36+8i`,
   bit mask `1<<i`):
   - row gate: group word0 (name index, +0x36+8i) ≠ 0 — rows with
     no weapon draw nothing;
   - count = group word1 (+0x38+8i), clamped ≤ 9999 (0x270F, the
     4-digit cap of the "%04i" template);
   - ARMED row (order-bits word +0x6E bit i set): sprite **0x47**
     @ (0x1EB, 0x59+14i) + sprite **0x4A** @ (0x25A, 0x59+14i),
     count template @0x457A28 [asm 0x4084bc..0x4084ef];
   - UNARMED row: sprites **0x49** / **0x4C** at the same positions,
     template @0x457A2E [asm 0x408553..0x408586];
   - name text: `FUN_00420260(name idx)` → SMLFONT string draw
     `FUN_00408913` at (0x1ED, 0x5B+14i), color 0x24 [asm
     0x40850c..0x40851f];
   - count text: `BmpNameBuild@0x44d1f2(buf, template, count)` —
     both templates are the literal `"%04i"` (0x457A28/0x457A2E,
     objdump -s) — then FUN_00408913 at (0x25C, 0x5B+14i) [asm
     0x408524..0x408549];
   - row body y = 0x59..0xAD step 14 (text +2), sitting inside the
     click rows 0x57..0xB8 [4]. GENERAL.BIN sprite geometry (real
     bank bytes): 0x47/0x49 = 108×11 (x 0x1EB..0x257, the row
     body), 0x4A/0x4C = 27×11 (x 0x25A..0x275, the count well).

   b. **Semantic correction — the "orders" are weapons.**
   `FUN_00420260@0x420260` is a compiled-in equipment-name switch:
   2..4 NEEDLER CANNON (1..3), 6..8 PLASMA CANNON X1..X3, 9..0xB
   HADES BOMB (1..3), 0xE FLAME BOMB, 0x10..0x12 PROXIMITY MINE
   X2/X4/X6, 0x14..0x16 PRESSURE MINE X2/X4/X6, 0x18..0x19 FRAG
   GRENADE (1..2), 0x1B/0x1C BOUNCY GRENADE X4/X6, 0x1D/0x1E STICKY
   GRENADE X4/X6, 0x20..0x23 ROCKET PACK X1/X3/X6/X9, 0x25..0x28
   REAPER PACK X1/X2/X4/X6, 0x2A AUTO SHIELDING, 0x2B BATTERY
   PACK, 0x2C THERMAL DAMPER, 0x2D/0x2E SCANNER LEVEL 2/3, default
   "ERROR" [strings 0x4589DD..0x458C0F]. So group word0 = the
   weapon NAME index, group word1 = its AMMO count, and the
   "+0x6E order-bits word" = per-weapon ARMED bits. Confirmation:
   FUN_0040eba0 case 8 (0x40ec86 region) is the ammo-refill pickup
   — per group it caps the current count at the group word1 max
   (mode-2 games read `>>0x11`, i.e. half the stored max) and sets
   `DAT_0046ccec = 2`; case 4 (0x40f0xx) is the money/score pickup
   (player type `== word@0x4edb90` only): +1000/+2000/+5000/+
   10000 to score `_DAT_004dd40c` or +10/+50/+100/+250 to money
   `DAT_0046ae70`, each setting sibling countdown `0x46ccf0 = 2`.

   c. **The banks** [FUN_0041d4e9 game-init / FUN_0041df10
   mission-init loads; ESI anchors in each consumer's asm]: the row
   sprites, portraits and bars come from **GAMEGFX\GENERAL.BIN**
   (153 sprites, 128826 B → `_DAT_004edd7c`, ESI at 0x4084CB/
   0x4080CE/0x40819F); the name/count text from
   **GAMEGFX\SMLFONT.BIN** (63 glyphs 5×7, chars 0x21..0x5E →
   `_DAT_004ede7c`, ESI at 0x408511/0x408540); the score/money
   strip from **GAMEGFX\NUMBERS.BIN** (12 sprites: digits 0..9
   9×11, 0xA 100×11, 0xB 74×11 → `DAT_0046af3c`, ESI at
   0x4085E7); the deploy-panel backdrop from
   **GAMEGFX\SCANNER.BIN** (→ `_DAT_004edd80`, ESI at 0x407233).
   All use the one .BIN layout [RESEARCH-8STREET .BIN row]:
   directory entry `2 + 4*id`, record `entry + u32[entry]`, record
   `{u16 flags; if flags&2: u16 yhot(+2), u16 xhot(+4); u16 w;
   u16 h; data}`. The sprite blit `FUN_00401ca2(id, transp, x, y)`
   [0x401ca2]: flags bit0 = RLE (control u16 — bit15 skip
   (w&0xFFF), bit14 end-of-line, else literal run of (w&0xFFF)
   bytes); bit0 clear = raw rows, transp=1 keeping only nonzero
   bytes; flags bit1 adds the hotspot to (x,y). The glyph blit
   `FUN_00402884(ch-0x21, color, x, y)` [0x402884] fills the
   glyph mask with a solid color (width at record+6, i.e.
   SMLFONT glyphs carry hotspots);
   `FUN_00402a12` is the width lookup; `FUN_00408913` is the text
   draw: char < 0x21 advances 6 px, else glyph `ch-0x21`, advance
   `w + 1`; chars ≥ 0x7F remap through FUN_00410493 (the codepage
   family — unused by these ASCII names).

   d. **The sibling per-frame passes** in FUN_00403938's tail [asm
   0x4071d5..0x40724e, verified]: with the map overlay off
   (`_DAT_004edba0 == 0`): 0x4071E3 `FUN_004072bf` EVERY FRAME —
   per-slot select portraits from GENERAL.BIN at (0x1E7/0x219/
   0x24B, y=5): slot 0 sprite 0x12 (selected) / 0x15, slot 1
   0x13/0x16, slot 2 0x14/0x17 (48×48 each — filling strip
   y 5..0x35 exactly); gates per slot: squad size
   `DAT_0046cbd8 > slot`, ALIVE word +0x7C ≠ 0, HP +0x78 ≥ 1; the
   pass also ticks armor (+0x2E −1/frame, clamp 5 — a draw-pass
   state mutation the engine does not model), draws the dither HP
   fill `FUN_00401ae6` under each portrait, and the active-robot
   blink cursor sprite `(g_frame_count & 3) + 0x51` @ (0x1F0/
   0x222/0x254, 0xD) when the sprite-list field `0x4dc5d0` ∈
   {1,2,3} (its producer is open). 0x4071E8 `FUN_0040807f` EVERY
   FRAME — per-slot HP bar sprite `0x46 - min(hp*46/5000, ...)`
   (46 steps, hp = dword@+0x78 clamped ≤ 5000; hp ≤ 0 → 0x46) @
   (slot_x, 0x3C) and armor bar `0x8E - (armor*46)/2500` clamped
   ≤ 0x8D (armor = word@+0x2E clamped ≤ 2500; the +0x30 word == 0
   → 0x8E) @ (slot_x, 0x49), GENERAL.BIN, slot_x = 0x1E8/0x21A/
   0x24C. Countdown `0x46ccf0` → `FUN_004085ce` (the score/money
   strip, NUMBERS.BIN): icon 0xA @ (0x1FE, 0x18E) + nine digit
   sprites (score `_DAT_004dd40c`, x 0x202..0x256 step), icon 0xB
   @ (0x20B, 0x1A4) + six digits (money `DAT_0046ae70`, x
   0x211..0x245). Countdown `0x46ccf8` → sprite 0x12 from
   SCANNER.BIN @ (0x1EE, 0xC3) — the deploy-panel backdrop (the
   FUN_0041ec81 deploy-strip region y 0xC3..0x147).

   e. **The initial draw**: MissionShell zeroes both countdowns
   with the mission-state globals at entry (0x4478BE/0x4478C4),
   then AFTER the mission-load calls sets BOTH `0x46ccec` and
   `0x46ccf0` to 2 (0x447C5D `mov edi,2` → 0x447C74/0x447C7A) —
   the rows + score strip draw on the first frames. Other
   `0x46ccec` producers: sidebar_control = 2 [2-4], robot death
   FUN_00409138 = 3 [5], ammo pickup = 2 [b], the MissionShell
   auto-reselect (0x448111..0x448117: when it changes
   `DAT_0046cbdc` it writes 0x46ccec = ebx(2), clearing
   `_DAT_004ede34`/`_DAT_004ea8f8`).

   Engine seam (this unit): GENERAL.BIN + SMLFONT.BIN stage with
   the mission (the GAMEGFX tail grows to 12 files); present draws
   the row chrome exactly like (a) on the countdown — sprites
   0x47/0x4A (armed) / 0x49/0x4C (unarmed) at (0x1EB,0x59+14i) /
   (0x25A,0x59+14i), rows gated by the availability mask bit (the
   name-index gate analog) — plus the select portraits (d) at
   (0x1E7+0x32*slot, 5) gated by squad size + alive (0x12+slot
   selected / 0x15+slot not); the initial countdown is 2 on
   activate [e]. NOT wired (each needs unmodeled state, never
   invented pixels): name/count text (needs the type table's name
   indices + ammo counts), HP/armor bars (needs +0x78/+0x2E sim
   fields), the score strip (needs `_DAT_004dd40c`/`DAT_0046ae70`
   sim state + NUMBERS.BIN), the deploy panel + blink cursor
   (overlay family / 0x4dc5d0 producer open).

## 7. Per-tick mover state the sim hash must cover (P4 slice)

From the verified write sets of {robots, robot_move, move_x/y_who,
move_is_possible, move_is_possible2} on the robot record + globals:

- pos_x, pos_y (Q13), pos_z/floor (Q5), state, facing, dir byte, anim
  phase, the 8-word probe-z cache (+0x1A..), stop distance (+0x74),
  move-target pair (DAT_0046cc30/60[idx]), extraction-beacon globals
  0x4eabb0/2/4/6/8
  + the 12-slot claim array 0x4eabba, RNG (RandA — spawn variant, scatter
  jitter), selection (DAT_0046cbd4/cbdc/cbd8), and g_frame_count (drives
  the 6-phase sub-tick position in-frame). Presentation-side (cursor,
  sidebar redraw flags, SFX queues) stays outside the hashed core per the
  D17 hybrid split.

## 7b. Amendment 2026-08-20 (worker 778d091a, objdump re-read driving the
mission.rs seam green; corrects/refines §§3-4 and the spawn-settle gloss)

1. **FUN_0041ebf8 abs's BOTH arguments** [verified asm 0x41ebfc..0x41ec19]:
   `cdq/xor/sub` runs on dx, then on dy, THEN max+min/2. The result is
   therefore ALWAYS non-negative — `dist(-10,-4)` is +12, never -12.
   (Sec 8's ledger line was right; recorded here because a stray test
   expectation assumed signed output.)
2. **move_is_possible per-probe climb reference is the probe's OWN cached
   word** [verified asm 0x41e8ce + esi stride]: the loop keeps
   `esi = idx*0xA8 + 2*i` and loads `edi = dword@(0x4c69fc + esi) >> 16`
   = `word@(+0x1A + 2i)` = probe cache slot i, sign-extended (sar). That
   same zref is BOTH the get_z_pos z input AND the climb-compare
   reference: blocked iff `|floor_z - zref_i| > 4` (signed abs). Sec 3's
   "+0x1A doubles as the climb-compare z" was probe 0's instance of the
   general per-probe rule.
3. **move_is_possible pass-side writes** [verified 0x41e928..0x41e996]:
   on pass, `robot+0x08 = get_z_pos(center, min(robot_z, 0xFF))` (upper
   clamp only at the call; get_z_pos clamps 0..0xFF internally), and the
   8 probe floors are cached as u16 words at +0x1A..+0x28. On ANY probe
   failure the loop aborts with NO writes — probe cache and +0x08 keep
   their prior values.
4. **Spawn settle is best-effort** [consequence of 2+3, verified seed
   loop 0x40d031..0x40d098]: pos_z = mrk.z*0x20 - 1 seeds all 8 cache
   words with its LOW word (level-0 marker → 0xFFFF → zref −1). The
   settle move_is_possible then passes only if every probe floor is
   within 4 of `L*0x20 - 1` — e.g. a ground tile with height byte ≤ 3 at
   the marker level. A tall floor (height byte 8 at level 0: |8−(−1)|=9)
   leaves the robot at z −1 with 0xFFFF probes, i.e. permanently
   un-walkable until something else updates the cache — faithful EXW
   behavior, not an error case.
5. **Armer snap has NO center offset** [verified 0x42486a..0x424882]:
   `robot.pos_x = tx << 13`, `pos_y = ty << 13` exactly (tile origin),
   unlike the MRK spawn (+0xF00). Sec 6's pseudocode was already right.
6. **FUN_004248c8 spread table slots 0..8** [verified jumptable bodies
   0x4248c8..0x424990]: (0,0),(+1,0),(−1,0),(0,−1),(0,+1),(−1,−1),
   (+1,−1),(−1,+1),(+1,+1) — matches the seam's SPREAD_OFFSETS order.
   The free-slot scan runs over the first `DAT_0046ccbc` (robot count)
   slots; slot > 0xB → no assignment (caller skips the pos write).
   [Completed §7j.20: slots 9/10/11 = (−2,0)/(0,−2)/(+2,0), and the
   ≥12 case actually leaves the caller's out-params UNINITIALIZED —
   both callers store them, so a 13th+ consumer gets a garbage tile.]

## 7c. Amendment 2026-08-21 (worker d8c46c88, objdump re-read of the
mission file-load + table-build pass — closes open item 1)

**load_mission@0041dc5a** [verified full disasm 0x41dc5a..0x41df0b], called
from MissionShell@0x447b3a immediately after the input-echo-array init
(0x4e6ed8) and BEFORE the GAMEGFX staging family (0x447b3f onward) and
load_markers@0x447b58. (Corrects §1's gloss that FUN_0041d954's arena pass
was where the tables filled: 0x41d954 only allocates.)

1. **Paths** [verified 0x41dc63 → build_mission_paths@0044670c, string VAs
   resolved from the PE]: two path prefixes are built from the zone index
   `0x4edd8c` (0..6) and mission number `0x4edd88`:
   - path1 @0x4dca0c = `EDITOR\ZONE{chr(0x41+zone)}\MISSION{mission}`
     (mission number +5 when `0x4edb88 == 2` [multiplayer/demo variant,
     untested]) — used for `.TOT`, `.DAT`, `.PAD` (all appended by the
     concat helper @0x41dbed, which returns the START of the full path);
   - path2 @0x4dca8c = `EDITOR\ZONE{chr(0x41+zone)}\MISSION{chr(0x41+zone)}`
     (the zone-level file) — used for `.CGR`, `.BIN`, `.MIN`, and
     `.LNG` when `0x4eba1c == 1` else `.LNK` [verified 0x41dcf4].
2. **Loads** [verified call order]: whole-file reads via read_file@0041cc7f
   (open/seek-end/tell/rewind/read ≤0x80000-chunks/close) into the arena:
   TOT→`DAT_004ede20`, DAT→`_DAT_004edd58`, CGR→`DAT_004edd60`,
   BIN→`DAT_004ede1c`, MIN→`DAT_004edd9c`, LNK/LNG→`0x45cdda` (0x8000).
3. **Table build** [verified 0x41dd1d..0x41dde3]:
   - `0x302` bytes copied from `0x4edbf8` → `0x4ddb34`, and
     `word[BIN_buf]` → `DAT_0046cdb8` [staged data, consumer unidentified];
   - `w = s16[TOT]` → `DAT_004eddec`, `h = s16[TOT+2]` → `DAT_004eddf0`
     (the TOT cursor advances 4; `DAT_004eddf4 = w*h`);
   - **DAT cursor +4** (skips the on-disk w/h header — the payload IS the
     plane-major u8 plane set);
   - `y_line[y] = y*w` for y in 0..=h at `0x4ea900` (h+1 dwords; the
     get_from_dat_file table);
   - `z_base[z] = z*w*h` for z in 0..=7 at `0x4eaacc` (8 dwords).
4. **Runtime sweep** [verified 0x41dde4..0x41de43]: every DAT byte ≥ 0x80
   in planes 0..6 is set to 0 (plane 7 untouched). The shipped corpus has
   0 such bytes in ZONEA (the 0xFF seen in-plane there is PAD-written
   post-sweep), so this only matters for editor/padded data.
5. **PAD staging** [verified 0x41de44..0x41df03]: PAD is read into
   `0x4e44f8` (0x1f38 = 999 records + slack) as 8-byte staged records
   `(flag, x, y, kind)` (disk 6-byte `(x, y, kind)` unpacked 2 bytes at a
   time), then for i in 0..999: if `x != -1`: flag word set 1, and
   **`DAT[kind·w·h + y·w + x] = 0xFF`** with NO bounds check on kind/x/y
   [verified absence — shipped kind values are 0..6 and the 0x13884
   arena covers the largest map, so real writes stay in the allocation].
   get_from_dat_file reads 0xFF back as type 1 → **a PAD marks its tile
   as a type-1 (CGR slot 0, 0x1F-height deck block) cell at level `kind`**
   — the concrete "pad effect" storage FORMATS-MISSION §10 was looking
   for [inferred reading; the write itself is verified].
6. **CGR height addressing** [verified 0x41e328..0x41e353, corrects
   FORMATS-MISSION §18's "light compression/RLE" hypothesis]:
   `height = CGR[2 + 4·(type−1) + dir[type−1] + 6 + (sy<<5) + sx]` —
   i.e. the u32 directory entry plus a 6-byte per-sprite header, then the
   RAW 1024-byte 32×32 height map. The 1026/1028/… sprite sizes are just
   header (6 B) + 1024 B map (+ pad); there is no codec. CGR slot 0
   (type 1) is 0x1F everywhere; slot 36 (type 37) reads 0x01 at its row
   start [corpus bytes] — so ZONEA walls block not by being tall but by
   being a LOWER floor than the type-1 deck the robots stand on (climb 4).
7. **Markers → robots** [verified 0x40cca0..0x40d098]: load_markers stages
   the 12×16 B MRK records into 12×12 B at `0x4e6430` (flag dropped),
   clears 12×0xA8 robot records + targets, then robot i (i <
   DAT_0046ccbc = the TOTAL banked count; robots_per_player is the
   separate cell DAT_0046cbd8: zone<3||zone==7 → 1, zone 3 → 2,
   else 3 — both cells get the zone rule in SP, `total := per_player`;
   in a network session `[0x4edb88] != 0` the override @0x40cd8d sets
   `total [0x46ccbc] := [0x46cbe0]` (player count) and
   `per_player [0x46cbd8] := 1`, stamping markers
   `record[i]+0x2A := i` — CORRECTED W8-prep 2026-08-22, raw disasm
   ghidra-project/exw-spawncount-asm.txt; the SP branch instead stamps
   player type `[0x4edb90]` at `record[12]+0x2A` (stale MRK-copy
   counter — one past the bank; harmless, the spawn tail re-stamps the
   0x4c71c4 anchor bank after; the EXD twin does it identically). SP
   never takes the override: the title menu sets 0x4edb88=0 ∧
   0x46cbe0=1 — RE-EXD-MAP §5d, the W8 pin) takes MRK record i VERBATIM:
   `pos = (x<<13)+0xF00, (y<<13)+0xF00`, `z = word3<<5 − 1` — so **MRK
   word 3 is the spawn Z LEVEL** (1 = ground), not a "type"; a word-3=0
   marker seeds z −1 and only settles on a height-≤3 ground tile
   (amendment 7b.4). The 0x62-stride stats copy, variant RandA()&3, the
   probe seeding, and the one settle probe match §§3/7b exactly.
8. **Beacon armer callers** [verified 0x433cfb; RE-IDENTIFIED §7j.20]:
   the only call site of FUN_004247b5 is inside **FUN_00433980** (the
   zone pad-trigger script dispatcher, §7j.19 item 4) — this §7c.8
   note originally mis-attributed the enclosing function to a
   "robot-sprite click family ~0x433cbc" (0x433cbc lies inside
   FUN_00433980's 3185-byte body); the arm is a scripted extraction
   pad, and "robot state 2 = selected" is wrong — the armer writes
   state 3 (halt). FUN_00424a6f (the nearby call
   with small immediates 0..0xA) is a message/popup builder, NOT a move
   producer [verified negative]. The verified move producer remains the
   §5/§6 order-consumption path; on real shipped maps no two MRK spawn
   markers sit within the 6-tile order radius (ZONEB's four are adjacent
   but zone 1 spawns one robot), so a second walker on the real map must
   be staged externally (host/test seam) — exactly what multiplayer's
   0x46cbe0 override does in the original.

## 7d. Amendment 2026-08-21 (worker 4b75846d, the weapon-table provenance
pass — REFUTES the TABLE.BIN hypothesis and closes open item 5)

New dump `ghidra-project/exw-typetable.txt` (script
`tools/ghidra-scripts/ExwTypeTable.java`, -process -noanalysis) + XRefList
over the whole program.

1. **TABLE.BIN is NOT the 0x4de664 table's file source — the §6c.6
   hypothesis is REFUTED.** The TABLE.BIN load
   `LoadFile("GAMEGFX\TABLE.BIN", [0x0046cbbc])` (FUN_0041df10 @0x41e01e)
   targets an `ArenaAlloc(160000)` bump-arena buffer (FUN_0041d954
   @0x41dad6) — a runtime heap address, never .bss 0x4de664. The pointer
   variable 0x0046cbbc has exactly 3 xrefs program-wide [XRefList,
   verified]: alloc @0x41dad6, load @0x41e01e, and ONE reader:
   `FUN_004089b1 @0x4089d5`. FUN_004089b1 is the STRATEGIC-MAP overlay
   pass: it clears the 0x4b000 map buffer (0x4ede18), then its FIRST
   action is `ESI = [0x0046cbbc]; FUN_00401e39(EDI=map_buf, EAX=0,
   EDX=1, ESI=TABLE.BIN)` [asm 0x4089d5..0x4089e1] — i.e. **TABLE.BIN is
   a draw_IMG-family sprite bank whose image 0 is the map backdrop**
   (ESI is immediately after clobbered by GENERAL.BIN 0x4edd7c for the
   robot markers 0x55/0x56 and the PAD/order-target markers 0x57/0x58;
   per-tile map coloring runs through the word table at 0x45cdd8+2*type,
   kin to the GAMEGFX\PALTRAN\*.TRN + GAMEGFX\MAPTRAN\*.TRN strings at
   0x458c15..0x458c40). TABLE.BIN = map-overlay art (the sec-6c.1
   backlog family), nothing to do with weapons.
2. **0x4de664 is LIVE SESSION STATE, not file data** [verified writer
   census over the dumped shells + the zero-init]: .bss-zeroed at boot,
   mutated only by (a) the SHOP FUN_00440e45 buy/sell/auto-buy paths
   (0x4413xx..0x4425xx — e.g. buy writes the full 7-word group
   `name_idx, ammo, price, category, item_idx, 0, owned=1` at
   `0x4de664 + type*0x62 + group*0xE` [decomp 0x44168b region]; the
   sibling chassis table 0x4deafc is type-stride 0x1C = 2×0xE groups),
   (b) the SAVE-LOAD restore (FUN_0044745e case 2 copies the saved row
   word-for-word, 7×7 words, into `0x4de664 + type*0x62` [decomp
   0x43c1xx region]), (c) the MULTIPLAYER lobby exchange FUN_00448ef1
   (5 writer sites 0x4491xx..0x449axx staging rows via the 0x4dd4a0
   0x80-stride per-player buffer). No loader ever bulk-copies the table.
3. **The player TYPE word@0x4edb90 = 0 for the whole single-player
   campaign** [verified]: GameMain writes `_DAT_004edb90 = 0` once at
   boot (0x41c34c, right after FUN_0043a144 — the bootattract decompile;
   XRefList shows the only SP writer), so every player robot's stats row
   is row 0 = 0x4de664 itself; all other writers are the MP lobby
   FUN_00448ef1 (network-chosen chassis). Every other 0x4edb90 xref is a
   READ (NameEntryScreen, shop, MissionShell 0x4480d0, load_markers).
4. **Campaign flow + fresh-campaign loadout** [verified GameMain
   decompile, bootattract dump]: the episode loop per mission is
   map-room FUN_0043e7d4 → briefing FUN_0043d00b → **SHOP FUN_00440e45
   (before EVERY mission, incl. mission 1)** → MissionShell → debrief.
   A fresh campaign enters the shop with money 4000 (single-player;
   difficulty −500/step via the title start `4000 − 500*diff`, mode-2
   variant 0x5DC) and an ALL-ZERO loadout [verified: money init sites +
   no table initializer exists] — weapons exist only after purchase.
   Consequence for the mission slice: a faithful fresh-campaign mission
   has NO weapon rows (all group word0 = 0 → no rows draw, spawn stats
   copy yields zero groups, default armed-bits word stays 0 — the
   §6c.6 `1 << first i with word0 != 0` finds no i).
5. **FUN_00420260 name switch pinned exactly** (for the row text): the
   compiled-in table 0x4589DD..0x458C11 maps name index → string
   [verified decompile + PE bytes]: 2/3/4 NEEDLER CANNON #1/#2/#3,
   6/7/8 PLASMA CANNON X1/X2/X3, 9/10/0xB HADES BOMB #1/#2/#3, 0xE
   FLAME BOMB, 0x10/11/12 PROXIMITY MINE X2/X4/X6, 0x14/15/16
   PRESSURE MINE X2/X4/X6, 0x18/19 FRAG GRENADE #1/#2, 0x1B/0x1C
   BOUNCY GRENADE X4/X6, 0x1D/0x1E STICKY GRENADE X4/X6, 0x20..0x23
   ROCKET PACK X1/X3/X6/X9, 0x25..0x28 REAPER PACK X1/X2/X4/X6, 0x2A
   AUTO SHIELDING, 0x2B BATTERY PACK, 0x2C THERMAL DAMPER, 0x2D/0x2E
   SCANNER LEVEL 2/3, default (incl. 0/1/5/0xD/0xF/0x17/0x1A/0x24/0x29)
   "ERROR" (0x458C0F). String 0x4589D2 "CLASSIFIED" sits just before
   the table (map overlay use, not a weapon).

Engine consequence (this unit): the type table is modeled as HOST-STAGED
per-robot loadout data — 7 groups of (name_idx, ammo) — with the
faithful default EMPTY (fresh campaign, D51); the all-7 availability
default + set_order_availability seam are removed. See DECISIONS D51.

## 7e. Amendment 2026-08-21 (worker f4982e53, the map-overlay family —
FUN_004089b1 fully decoded + the toggle family)

New dumps `ghidra-project/exw-mapoverlay.txt` + `exw-mapoverlay2.txt`
(scripts `tools/ghidra-scripts/ExwMapOverlay.java` + `ExwMapOverlay2.java`,
-process -noanalysis) + objdump re-read of the whole EXE. Tags below
[verified] = decompile+asm+file bytes agree.

1. **FUN_004089b1@0x4089b1 = the strategic-map overlay draw**
   [0x4089b1..0x408dcd, verified]:
   a. Clear 0x4b000 bytes (640×480) of the backbuffer `[0x4ede18]`
      via the rep-stos helper FUN_00402965 (ECX=0x4b000).
   b. `FUN_00401e39(id=0, transp=1, x=0, y=0)` with ESI = the
      TABLE.BIN arena `[0x46cbbc]`, EDI = backbuffer — image 0 is a
      **480×480 RLE sprite** (TABLE.BIN bytes: dir count 1, entry+0
      record flags 3 = RLE+hotspot, yhot/xhot 0/0, w=h=0x1E0). The
      draw is the one .BIN blit with EXPLICIT dest+bank (FUN_00401e39
      = the FUN_00401ca2 codec with EDI dest + ESI bank params).
   c. Per-tile territory stamps: for row 0..H (`[0x4eddf0]`), col
      0..W (`[0x4eddec]`), tile = `dword@(0x4ea900+4*row) + col`
      (y-line table = plain row-major `r*W`, built by load_mission
      0x41ddb1); record = `0x4796bc + tile*0x1E`; for z-word offset
      0..0xE step 2:
      - `w = word@(rec+zoff)` (the TOT type-DB mirror word);
      - `cw = word@(0x45cdda + 2*w)` — **the LNK file image** (the
        load_mission `.LNK` load lands AT 0x45cdda in .bss,
        FUN_0041dbed concat + FUN_0041cc7f(EDX=0x45cdda) at
        0x41dd13; the "0x45cdd8 table" of 7d.1 is the same image —
        the dword read at 0x45cdd8+2w >>16 = word at 0x45cdda+2w);
      - `word@(rec+zoff) = cw` (destructive chain-advance, the SAME
        LNK step the terrain renderer does — idempotent on ZONEA's
        identity LNK);
      - if cw != 0: `FUN_00402ab8(cw, row'=(0x80+row+col)-zoff,
        ramp, col'=(0xf0-2*row+2*col))` where
        `variant = byte@(0x4c420c+tile)`, `ramp = u32@(0x4dd464 +
        4*variant)`.
   d. Robot markers: slot 0..`[0x46ccbc]`, gate ALIVE(+0x7C);
      tile from Q13 ((v>>8)+0x10)>>5; sprite **0x55** when
      (mode `[0x4edb88]`==0: slot == selected `[0x46cbdc]`; else
      slot == player type `[0x4edb90]`), else **0x56**; GENERAL.BIN
      (ESI `[0x4edd7c]`) FUN_00401e39 transp=1 at
      x = 2*(tx−ty)+0xf0−0xc, y = tx+ty+0x80−0x1e−(z_dword>>4).
   e. Mode != 2 only: per-player (0..5) PAD/order markers
      0x57/0x58 (player 0 → 0x57) from the staging at
      0x4eaaee+0x20*p — either the single staged marker (word@
      0x4eab0c+0x20p indexes the 8-byte records {?,x,y,z} at
      0x4e44f8+8*i) or the count@0x4eaaee>>16 walk of the
      0x14-stride records at `[0x46cbf4]` + 0x4dedf2 type geometry
      (0x408c94..0x408dc4, the order-target loop) — then the
      active-order sprite **0x59** at 2x−2y+0xf0−0xc,
      x+y+0x80−0x1e−2z from `[0x4eb8d0/d4/d8]` (writer
      FUN_00425261).
   f. The tail is `JMP 0x4072b8` (the FUN_00403938 epilogue) —
      **the overlay draw NEVER RETURNS**: on an overlay frame the
      sidebar passes, the countdown consumers and the button chrome
      below 0x4071e3 are all skipped. This CORRECTS the 6c.8d
      reading (the passes are not "run with the overlay off" by an
      else-branch; they are skipped by the non-return).
2. **FUN_00402ab8@0x402ab8 = the 4×4 territory stamp** [verified]:
   dest = backbuffer + row'*0x280 + col'; mask =
   `byte[16]@([0x4edd9c] + cw*0x10)`; for r,c in 4×4: if
   mask[r*4+c] != 0: `dest[r*640+c] = ramp[mask[r*4+c]]` (XLAT
   through EBX = the ramp base). The mask bank `[0x4edd9c]` =
   ArenaAlloc(0x7530=30000 B) in FUN_0041d954@0x41dac7, filled by
   load_mission with the mission's **`.MIN` file** (FUN_0041dbed
   ".MIN" concat @0x41dcd8 + FUN_0041cc7f post-process) — MIN = the
   mini-map mask bank, 16 B per LNK-resolved word.
3. **The territory variant bytes 0x4c420c** [verified]: zeroed
   0x27d8 bytes by MissionShell@0x4479e8; then FUN_00408dcc
   (0x408dcc..0x408e98, called from robots()@0x40bc52 per robot in
   state 2) max-stamps an 11×11-tile square around each robot's
   tile: `variant[tile] = max(variant[tile], ring121[k])` with
   ring121 = the 121 dwords at 0x454cf8 (Chebyshev-diamond rings:
   7 center → 1 corners, PE bytes verified). So the variant = the
   robot-proximity ring 0..7 = the MAPTRAN ramp selector.
4. **MAPTRAN/PALTRAN loaders** [verified]:
   - FUN_00422171: for i in 0..7 `LoadFile("GAMEGFX\MAPTRAN"+i+
     ".TRN", u32@(0x4dd464+4i))` — 8 × 256-byte ramps
     (MAPTRAN0..7.TRN each exactly 256 B, shipped).
   - FUN_0042209b: same for "GAMEGFX\PALTRAN"+i+".TRN" into
     u32@(0x4dd444+4i), and after the i=7 load `_DAT_004dd444 = 0`
     (slot 0 NULLed). **This closes the MISSIONVIEW §8.2 producer
     question for u32[0x4dd444]: the PALTRAN ramp pointers.**
   - Both slot arrays allocated 8 × 0x100 in FUN_0041d954
     (0x41db2d/0x41db45 loops).
5. **The toggle family** [verified]:
   - Strip (6c.1) writes `[0x4eb8dc] = 5` + toggles `[0x4edba0]`;
     MissionShell decrements `[0x4eb8dc]` per frame while nonzero
     (0x44871d..0x44872a) — a pure 5-frame re-fire lockout, no
     other consumer.
   - MissionShell entry zeroes `[0x4edba0]` (0x44786b, with
     `[0x4ede34]` zoom and siblings).
   - Present FUN_00401107 map mode (0x40110c): presents the
     backbuffer top-left **480×480** (stride 640) directly — no
     camera window, no zoom.
   - mouse_l_click@0x40b868: overlay on + click x < 0x1e0 → the
     game-area dispatch is SKIPPED (clicks swallowed); x ≥ 0x1e0
     still reaches sidebar_control (so the toggle strip works while
     the map is open).
   - The button chrome in the FUN_00403938 tail (0x40724e..0x4072b2,
     only reached when the overlay did NOT draw): mode ∈ {1,7} →
     GENERAL.BIN sprite **0x8f**; else overlay on → **0x5f**; else
     **0x5e**; at (0x213, 0x1b5) via FUN_00401ca2 — the exact strip
     rect [0x213,0x24D]×[0x1B5,0x1CF].
   - Sibling (open): FUN_0044874b = the camera-fly-to helper reading
     the same 0x4e44f8 staging (map-click follow), gated by
     `[0x4eb9f4] != -1`; FUN_00408dc6-family map click routing not
     needed for the toggle slice.

Engine seam (this unit): the overlay draw lives on the PRESENTATION
half (D17): the toggle strip + the 5-frame lockout + the overlay bit
in MissionScene; the draw = backdrop (TABLE.BIN image 0) + territory
stamps (MIN masks × LNK words × MAPTRAN ramps × the variant rings
around live robots) + markers (GENERAL 0x55..0x59); the map present
presents the composed backbuffer; sim state untouched (never in the
hash). Order-target markers 0x57..0x59 need the order staging (not
modeled) — deliberately unwired (D50 never-invent).

## 7f. Amendment 2026-08-21 (worker 36c9e956, the sidebar HP/armor
bars + the score strip — the vitals producers)

New dump `ghidra-project/exw-sidebarbars.txt` (script
`tools/ghidra-scripts/ExwSidebarBars.java`, -process -noanalysis) +
full-EXE objdump displacement censuses. Tags [verified] = decompile +
asm agree; the four draw functions are fully pinned.

1. **FUN_0040807f = the HP + armor bars** [verified, 0x40807f..0x408402]:
   per slot k (gates `DAT_0046cbd8 > k`), robot idx `DAT_0046cbd4 + k`,
   GENERAL.BIN (`[0x4edd7c]`), `FUN_00401ca2(id, transp=1, x, y)`:
   - HP bar @ (0x1E8 + 0x32*k, 0x3C): `hp = min(dword@+0x78, 5000)`
     (SIGNED); `hp < 1 → sprite 0x46` else
     `sprite = 0x46 - (hp * 0x2E) / 5000` (idiv, trunc toward 0);
     i.e. ids 0x18 (full 5000) .. 0x46 (empty ≤ 0), 47 sprites.
   - Armor bar @ (slot_x, 0x49): gate `word@+0x30 == 0 → sprite 0x8E`;
     else `armor = SAR16(dword@+0x2E)` — **the armor VALUE is the word
     at +0x30** (the NEXT.md "+0x2E armor" gloss read the dword AT
     +0x2E; its HIGH word +0x30 is the value) — clamp ≤ 0x9C4 (2500),
     `sprite = 0x8E - (armor * 0x2E) / 0x9C4`, then `> 0x8D → 0x8D`;
     ids 0x60 (full 2500) .. 0x8E (empty), 47 sprites. Armor 0 (no
     armor) still DRAWS the empty 0x8E bar every frame.
2. **FUN_004085ce = the score/money strip** [verified, 0x4085ce..0x4085cd?
   size 837]: NUMBERS.BIN (`DAT_0046af3c`, ESI at every call), transp=1:
   icon 0xA @ (0x1FE, 0x18E); nine score digits of `_DAT_004dd40c`
   (UNSIGNED div/mod 10) at x = 0x202, 0x20C, 0x216, 0x222, 0x22C,
   0x236, 0x242, 0x24C, 0x256 (10^8..10^0 — irregular pitch, thousands
   groups), y 0x18E; icon 0xB @ (0x20B, 0x1A4); six money digits of
   `DAT_0046ae70` (SIGNED idiv — SAR/IDIV asm 0x4088b2..0x40890e) at
   x = 0x211, 0x21B, 0x225, 0x231, 0x23B, 0x245 (10^5..10^0), y
   0x1A4. Consumed by countdown `0x46ccf0` (decrement-then-draw).
3. **The FUN_00403938 tail order CORRECTED** [verified,
   exw-missionrender2 0x4071d5..]: overlay (non-return) →
   FUN_004072bf portraits → FUN_0040807f bars → `0x46ccf0` strip →
   `0x46ccec` rows → `0x46ccf8` SCANNER 0x12 → button chrome. The
   strip pass runs BEFORE the row pass (the 6c.8d listing implied the
   reverse).
4. **FUN_004072bf portrait pass exact gates** [verified]: slot k draws
   iff `ALIVE(+0x7C) != 0 AND hp(+0x78) >= 1`; sprite 0x12+k selected /
   0x15+k not @ (0x1E7+0x32*k, 5). When dead/hp<1, or while
   `word@+0x2E != 0`, a PATTERN blit `FUN_00401ae6(5, ..)` (mask bank
   0x4e6ed8, nonzero bytes only [objdump 0x401b0e..0x401b53]) shades
   the strip — **word@+0x2E is the HIT-FLASH timer, not armor**: damage
   bumps it (`word@+0x2E += 1`, FUN_0040e230 0x40e6xx), this pass
   clamps it to 5 then decrements it per frame while nonzero
   (0x4073a0 region). The D50 gloss "+0x2E ticks armor" is corrected.
5. **FUN_0040e230 = damage application** [verified, 0x40e230..0x40eb3c,
   already dumped in exw-missionrender]: per robot idx (EAX), damage
   EDX, killer EBX. Gates `DAT_0046cd0c == 0`, `type(+0x2A) != 2`,
   ALIVE != 0. Type 3 (ordered): shield pool +0x88 = 0x20. Else if
   `shield_charges(+0x8C) == 0 OR shield(+0x88) != 0` → the damage
   path: `if shield(+0x88) == 0` → `hit_flash(+0x2E) += 1`,
   `hp(+0x78) -= dmg` (with low-HP SFX thresholds vs
   `5000 + battery(+0x94)*100`, half, eighth; the +0xA4 counter += 3,
   > 100 → +0x34 = 100, shield = 0); else `shield = max(0, shield -
   dmg)`. Else (auto-shield idle): `charges -= 1; shield = 0x20`.
   `hp < 1` → DEATH: SP subset clears ALIVE, hp, armor(+0x30),
   drop(+0x80)=1, the 7 order words, `DAT_0046ccec = 3`, spawns 5
   debris via FUN_00420608 (2× RandA each — the shared stream!),
   FUN_0042382c; MP mode-2 does kill bookkeeping + FULL respawn from
   MRK + the equipment switch. NOT landed engine-side this unit (the
   death/debris/robot-death-pass FUN_00409138 interplay + the RNG
   interleaving need their own slice).
6. **FUN_0040eba0 = the robot-state event consumer** [verified,
   0x40eba0..0x40f273]: dispatch on a per-zone tile-type word (range
   tables 0x454a58/0x454a74 indexed by `_DAT_004edd8c`) → cases:
   1 = reinforcement staging (`drop(+0x80) = 1000`); 2 = shield
   pickup (`shield(+0x88) = 1000`); 3 = HEALTH pickup
   (`hp(+0x78) += 0x9C4` clamp 5000); **4 = the score/money pickup**
   (player-type gate `type(+0x2A) == [0x4edb90]`): `RandA()&1 == 0` →
   score += 1000/2000/5000/10000 by `RandA()&3`, else money
   `DAT_0046ae70` += 10/50/100/250; EACH award sets
   `0x46ccf0 = 2` (2 RandA draws per pickup — shared stream); 7 =
   `+0xA0 = 200`; 8 = the ammo refill (6c.8b, also
   `DAT_0046ccec = 2`); 9 = episode staging word `0x46cd30... = 2`.
   All cases also stage sprite-list effects at the 0x4dc5d0 family.
7. **The armor producers** [verified, objdump census — all writers of
   `0x4c6a14`]: `FUN_004100b7(idx, amount)` (0x4100b7, called from
   robots() 0x40bc72 with amount 0x14 when the robot's tile type-DB
   byte@+0x18 (0x4796d4+type*0x1E) != 0 — an ARMOR PAD): armor += 20,
   clamp 3000 (0xBB8); armor ≥ 2500 → FUN_004102b6 (full SFX), <
   2500 → the charge SFX family. When the +0x18 byte == 0: armor
   -= 10/frame, clamp ≥ 0 (robots() 0x40bc7d). Death/respawn zero it
   (FUN_0040e230). So armor = a transient pad-charged shield that
   BLEEDS 10/frame off-pad; the bar denominates 2500.
8. **HP init = the dropship landing** [verified, exw-simtail 0x41fc..
   family]: load_markers zeroes the records (hp 0, ALIVE 0); the
   dropship-anim state 2 (landing) sets `ALIVE = 1` and
   `hp = 5000 + battery(+0x94)*100` (0x4c6a5c write @ the landing
   block); battery comes from the equipment switch (0x2B BATTERY
   PACK word1, 0 fresh campaign). The engine's spawn collapses the
   landing (D50 design) — the faithful spawn hp = 5000 + 100*battery.
9. **Score/money + NUMBERS.BIN census** [verified xrefs]: score
   `_DAT_004dd40c` writers: GameMain boot 0 (0x41c44e) + campaign
   restart (0x41c5e2); pickups case 4; the FUN_0041a894 tally tail
   (0x41b758, + 0x46ccf0 = 2); save-load 6-slot family (0x4188xx);
   debrief readers. Money `DAT_0046ae70` writers: GameMain campaign
   init (0x41c5ec — the 4000−500*diff fresh value), the SHOP
   FUN_00440e45 everywhere, pickups, save-load, NameEntryScreen.
   NUMBERS.BIN: `ArenaAlloc` 0x41da5e + LoadFile mission-init
   (FUN_0041df10 @0x41dfae), SOLE consumer FUN_004085ce — stages with
   the mission exactly like GENERAL/SMLFONT (13-file tail).

Engine seam (this unit, D52): hp/armor stay HOST-STAGED presentation
state (Sidebar `vitals`: hp i32, armor i16) — the damage path does NOT
genuinely land this unit (death/debris/RNG interplay = its own slice),
so the sim hash + pins stay frozen; the bars read the staged vitals
with the faithful fresh defaults (hp 5000 — battery 0; armor 0 → the
0x8E empty bar draws every frame exactly like the original);
`set_weapon_loadout` derives battery from the 0x2B group and sets hp =
5000 + 100*battery (the landing formula). score/money land as
MissionScene session state (score 0, money 4000 fresh campaign), the
case-4 producer as a host-seam method (+ 0x46ccf0 = 2), the strip on
its own countdown (init 2 at activate, MissionShell 0x447c7a) in the
corrected tail order. Portrait gate gains the hp ≥ 1 arm (default
identical). The dither pattern blit + hit-flash stay unwired (never
invent pixels; FUN_00401ae6/0x4e6ed8 bank decode queued).

## 7g. Amendment 2026-08-21 (worker d115c2ea, the damage unit pre-decode)

Objdump censuses over `game-data/BEDLAM/BEDLAM.EXW` (regions
0x40bb60..0x40bd20, 0x40e670..0x40e7c0, 0x4100b7..0x4102c0, plus
FUN_00409138 displacement census) against the existing dumps
(exw-simtail FUN_0040b9f6, exw-missionrender FUN_0040e230,
exw-sidebarbars FUN_004072bf). Three 7f glosses are CORRECTED here.

1. **FUN_0040e230 field-map corrections** [verified asm
   0x40e260..0x40e2b5]: the damage gates read the STATE word
   (+0x0C, dword@+0x0A >> 16), NOT the type: `MP_mode == 0`,
   `state != 2`, `alive(+0x7C) != 0`; `state == 3` → shield
   (+0x88) = 0x20 AND RETURN (ordered robots convert damage into
   a shield tick). The 7f.5 "type(+0x2A)" gloss misread the
   register. The alarm trip is `if word@+0x34 == 0 → ctr(+0xA4)
   += 3; if ctr > 100 && TYPE(+0x2A, dword@+0x28 >> 16) ==
   player_type([0x4edb90]) → word@+0x34 = 100, ctr = 0` — the
   7f.5 "shield = 0" tail is CORRECTED to "+0xA4 = 0" (no shield
   write; the SFX slot alerts 0x10/0x11/0x12 are presentation).
   Damage order [verified]: hit_flash(+0x2E) += 1 FIRST, then
   hp(+0x78) -= dmg; the low-HP SFX thresholds compare OLD hp
   vs `5000 + 100*battery(+0x94)` full/half/eighth crossings
   (presentation only, no state).
2. **The robots() phase-0 pre-walk** [verified asm 0x40ba..0x40bb
   block, decompile exw-simtail 10930..10981]: inside the
   phase-0 invocation ONLY, per robot over ALL records with NO
   alive gate: word@+0x32 decay-if-nonzero (producer unknown —
   always 0, no-op), word@+0x34 (alarm) decay, dword@+0xA4 decay,
   shield(+0x88) −2 clamp ≥ 0, and the +0xA0 booster family:
   while +0xA0 != 0 → shield = 10000, +0xA0 −= 1, expiry
   (+0xA0 < 1) → +0xA0 = 0 AND shield = 150 (0x96). The
   player-type FadeSetup/FUN_004258d0 palette flashes inside are
   presentation. So the shield pool decays 2/frame; the pickup
   case 7 (7f.6) arms the 10000-while-boosting override.
3. **The armor pass CORRECTED — it is PHASE 1, not robot state 1**
   [verified asm 0x40bbab..0x40bc9f]: in the per-robot walk,
   after the alive gate (and the outer `word@+0x2C == 0` gate —
   producers decoded §7j.20: the SP spawn stagger writes
   1+k·(2000−m·1000/27), so it is NOT always 0; MP respawn 0x28),
   `if (phase == 1)`: tile = `tile_x + y_line[tile_y]`
   (`pos>>13` + dword@0x4ea900+4*ty); pad byte =
   `byte@(0x4796d4 + 0x1E*tile)` = the PER-TILE 0x1E record's
   +0x18 byte — the MISSIONVIEW §2 "static frame byte" (the DB
   at 0x4796bc is one 0x1E record per TILE, correcting the 7f.7
   "type-DB +0x18, 0x4796d4+type*0x1E" gloss; FUN_0040fe93
   preserves ECX push/pop, so the index really is the LINEAR
   TILE INDEX, verified). Producers of +0x18 are still
   MISSIONVIEW §8.1-open and the zero-fill leaves them 0 on
   ZONEA → on the shipped A-zone corpus armor ALWAYS bleeds.
   byte != 0 → `FUN_004100b7(idx, 20)`; byte == 0 → armor
   word(+0x30) −= 10 (i16 wrapping) then `SAR(dword@+0x2E) < 0`
   (the word re-read as signed) → 0.
4. **FUN_004100b7 decoded** [verified 0x4100b7..0x4102b6, sole
   caller 0x40bc72]: `amount == 0` → return. If the +0x98 pool
   != 0: pool −= amount; pool > 0 → return; pool ≤ 0 → pool = 0
   + the slot SFX 0x2e/0x2f/0x30 ("pool empty") + return. Only
   when pool == 0: armor word += amount (i16 wrapping), clamp
   new > 0xBB8 → 0xBB8; SFX families keyed on OLD (pre-write)
   vs NEW armor: old ≥ 0x9C4 → FUN_004102b6; old < 0x9C4 ∧ new
   ≥ 0x9C4 → crossing chirps 0x3/0x4/0x5; old < 0x753 → charge
   ticks. The +0x98 pool producer = the equipment stats-copy
   switch (spawn path 0x40d013, MP respawn path 0x40ea59):
   stat case 0x2C → +0x98 = word × 200 (same switch: 0x2A →
   shield_charges +0x8C, 0x2B → battery +0x94). Fresh campaign:
   all-zero stats → pool 0 → pads charge armor immediately.
   [mechanics verified; the drain-before-charge design intent
   stays tagged unclear]
5. **The 0x7d2/0x7d3 tile words** [verified 0x40bbef..0x40bc38]:
   the per-tile word at `0x460dfa + 2*tile` (read as the dword
   at 0x460df8+2*tile >> 16) — word == 0x7d2 ∧ phase 0 →
   `FUN_0040e230(idx, 15, -1)` (a hazard tile); word == 0x7d3 →
   the phase clamp (drop==0 → skip phases > 2, drop!=0 → skip
   phases > 4). **PRODUCER CLOSED 2026-08-21 (§7j.12 item 6)**:
   FUN_00422f18 stamps both words at mission load from the
   type-DB z-words vs the per-zone 4-word ranges at
   0x454a20 (0x7d2) / 0x454a3c (0x7d3); the bank is a
   runtime-mutable object grid, not a TOT mirror. The engine
   leaves the words unwired (never-invent; the stamper runs at
   load — a future engine seam for the mission-load path).
6. **The death tail pinned** [verified asm 0x40e6b9..0x40e791]:
   FUN_0042382c(idx) (presentation) → `DAT_0046ccec = 3` → the
   SEVEN order words zeroed (words +0x38..+0x68 stride 8 — NOT
   armor; the +0x30 armor word is zeroed later, only in the SP
   branch) → FIVE debris staged via FUN_00420608, per debris:
   RandA#1 → y = (rand&0x1f) + (pos_y>>8) − 0x10, RandA#2 → x =
   (rand&0x1f) + (pos_x>>8) − 0x10, z arg = robot.z + 8k, kind
   5, param5 = 2k, param6 = −1 (k = 0..4; FUN_00420608 = the
   128-slot 0x30-stride effect stager with map-bounds + z-clamp
   0x20..0xFF inside — presentation; the 10 draws on the SHARED
   stream are the sim side). Then the SP/MP gate
   (`mp_mode == 0 || respawn_ok[idx] != 0`): SP subset = alive =
   0, drop(+0x80) = 0, hp = 0, +0x9C = 1 (readers not yet
   census'd), armor = 0, SFX 0x19/0x1a/0x1b + the selected-robot
   `_DAT_004ede34 = 1` flag. MP branch = the full respawn
   (zeroes hit_flash/state/facing/armor/order-bits, variant =
   RandA&3, +0x2C = 0x28, re-spawn from MRK + probe re-seed +
   the stats re-copy incl. shield_charges/battery/+0x98).
7. **FUN_00409138 interplay CLOSED** [verified displacement
   census 0x409138..0x40a573]: it is NOT a death pass — it never
   writes hp/armor/shield; its two 0x4c6a60 refs are reads. It
   iterates robots (state-2 skip; player-type present →
   `DAT_0046ccec = 2`), writes order words (+0x36/+0x38/+0x3C),
   order bits (+0x6E), positions/state — the robot AI/aim pass
   (calls FUN_0040b615 → FUN_0040d197; neither reaches
   FUN_0040e230). ALL death state work flows through
   FUN_0040e230, which has exactly 4 callers: the robots()
   0x7d2 hazard (0x40bc38), FUN_0040db9e (0x40dbc2),
   FUN_004190bc (0x419067), FUN_004197d4 (0x4198b2) — the
   projectile/effect family.
8. **hit_flash decay pinned** [verified asm 0x40736d..0x4073b3]:
   in the portrait pass, per SIDEBAR SLOT robot, ONLY while
   `alive && hp ≥ 1 && word@+0x2E != 0`: clamp `> 5 → 5`, then
   −= 1, then the dither blit FUN_00401ae6(5, ...). Dead/hp<1
   robots' word freezes (the portrait path never reads it again);
   SP death does not clear it (the MP respawn does).

## 7h. Amendment 2026-08-21 (worker 66831068, the pickup consumer unit)

Objdump decode of the FUN_0040eba0 dispatch head + case bodies
(0x40eba0..0x40f273, intel syntax, fresh `--start-address` dumps —
the exw-full.asm linear sweep is mis-synced across 0x40eb6b..0x40eba0
because it swallowed the jump table), the robots() CALLER block
(0x40bf18..0x40bff8), and the DGROUP tables at 0x454a58/0x454a74/
0x454a90. The 7f.6 case GLOSSES are all confirmed; the dispatch
MECHANISM is new.

1. **The tile-word dispatch** [verified asm 0x40ebaa..0x40ecef]:
   entry eax = the per-tile type word (signed), edx = the robot
   index (kept in ebp). `idx = _DAT_004edd8c` (the TERRAIN-SET
   index) selects one dword from EACH of two 7-entry DGROUP tables
   (values [verified PE bytes 0x454a58..0x454a8f]):
   - A = dword[0x454a58 + idx*4] = `[0x4e, 0x75, 0x75, 0x358, 0x75,
     0xa3, 0xa3]`
   - B = dword[0x454a74 + idx*4] = `[0x75, 0x535, 0x70b, 0x656,
     0x535, 0x4fe, 0x31e]`
   Each table base then splits into FOUR closed 4-word groups
   (signed compares, `w >= base+k*4 && w <= base+k*4+3`):
   - A+0..3 → case 1 (reinforcement), A+4..7 → case 3 (health),
     A+8..11 → case 2 (shield), A+12..15 → case 4 (score/money)
   - B+0..3 → case 9 (episode staging), B+4..7 → case 7 (shield
     booster), B+8..11 → case 8 (ammo refill)
   Then `ebx--; if (ebx > 8 unsigned) → 0x40dcbc` and
   `jmp [0x40eb7c + (case-1)*4]` — jump table [verified bytes]:
   1→0x40eed9, 2→0x40ef72, 3→0x40f1ea, 4→0x40eefe, 5→0x40dcbc,
   6→0x40dcbc (5/6 share the common-exit stub with the
   out-of-range default), 7→0x40edcc, 8→0x40ecf0, 9→0x40ee40.
   So 7 pickup words per set live in table A's block (16 words,
   `[A, A+0x10)`) and 6 in B's (`[B, B+0xc)`) — 28 pickup words
   per terrain set, four per case.
2. **The case bodies this unit lands** [verified asm]: all four
   open with the SFX queue call `0x43a48e(bank 0x4edfa8, -1, 0,
   -1, stack 3)` (the mission SFX tier is a backlog slice — not
   modeled), address the record as `robot_idx*0xA8` off the
   0x4c69e4 fields, then:
   - case 1 (0x40eed9): `drop(+0x80) = 0x3E8` (1000) — the
     reinforcement staging; effect id 1.
   - case 2 (0x40ef72): `shield(+0x88) = 0x3E8` (1000) — the
     shield pickup refills the pool; effect id 6.
   - case 3 (0x40f1ea): `hp(+0x78) += 0x9C4` (2500) then
     `if hp > 0x1388 (5000) → hp = 5000`; effect id 7.
   - case 7 (0x40edcc): `shield_boost(+0xA0) = 0xC8` (200) — arms
     the 10000-while-boosting override the phase-0 pre-walk
     already implements (7g.2); effect id 0xE.
   The shared tail stages one 16-B sprite-effect row via the
   0x422038 slot allocator at `0x4dc5d0+slot*0x10`: +4 = pos_x>>8,
   +8 = pos_y>>8, +0xC = z(+0x08)+0x20, +0x10 = the per-case
   effect id above; then `if [0x4edb88] == 2` (MP present) an
   extra FUN_00425647 staging on the pickup tile latch. All
   presentation (effect rows + SFX) — not modeled this unit.
3. **The CALLER consume block** [verified asm 0x40bf18..0x40bff8,
   in the robots() walk]: reads the per-tile word as
   `dword[0x4796ba + (yline([0x4dc690]) + [0x4dc68c])*0x1E +
   [0x4dc688]*2] >> 16` — i.e. the type-DB mirror word
   (0x4796bc rows of 0x1E B, one row per tile in yline+x order)
   of the LAST get_z_pos probe cell (the 0x4dc688/8c/90 latch =
   (z_level, tile_x, tile_y), sec 5 Terrain.last_trigger). If the
   word is in `[A, A+0x10)` or `[B, B+0xc)` the pickup FIRES:
   (a) the DAT z-plane byte is zeroed — `DAT[z_base[0x4eaacc +
   z*4] + tile_x + yline(tile_y)] = 0` (consumes the pickup from
   the collision plane); (b) the mirror word is REPLACED by
   `word[0x454a90 + idx*4]` (the bare-floor word; table C =
   `[0x70b, 0x48f, 0x24c, 0x368, 0x48f, 0x39, 0x39]`) so the
   drawer stops drawing the pickup sprite; (c) mirror byte at
   row+0x10+z (0x4796cc family) = 1; (d) the tile latch
   x/y/z is staged at 0x4dc6ac/b0/b4; then
   `FUN_0040eba0(tile_word, robot_idx)` runs the dispatch above
   on the ORIGINAL word.
4. **_DAT_004edd8c producers** [verified xrefs]: GameMain boots it
   to 1 (0x41c42a, alongside 0x4edd88=1); the mission-select
   family 0x43edb0..0x43ee3d maps mission NUMBER → set
   (1..2→2, 3..4→3, 5..6→4, 7..8→5, 9..10→6, continuing to 7) —
   i.e. a campaign-episode → terrain-set index, 7 sets (A..G),
   with the title/first state using set 1. Engine-side the zone
   letter already keys MISSION{A..G}.BIN; the set per zone is
   [hypothesis: set = zone+1, boot-consistent] — untested until
   the tile-word producer lands.

Engine seam (this unit): the tile-word producer (type-DB mirror +
probe latch walk) is the entangled piece — the engine Terrain has
no mirror rows — so the DISPATCH lands as a pure decode function
(`pickup_case(word, set)` over the verified tables) and the case
BODIES land as a sim seam `MissionSim::apply_pickup(idx, case)`
on the already-real fields (drop/shield/hp/shield_boost — cases
1/2/3/7; case 4 stays the D52 host seam, score/money are session
state). Presentation (SFX 0x43a48e, the 0x4dc5d0 effect rows, the
MP FUN_00425647 tail) stays unwired; `PickupOutcome` exposes the
per-case effect id for that future slice. Nothing on the default
corpus path invokes the seam — the sim pins stay frozen.

## 7i. Amendment 2026-08-21 (worker efc8b1e0, the dead/hit dither
overlay — FUN_00401ae6 + the 0x4e6ed8 noise bank)

Fresh objdump of 0x401ae6..0x401bbb (the whole blit), 0x447ab0..0x447b60
+ 0x448090..0x4481b0 (the bank producers inside MissionShell
FUN_0044771c), cross-checked against the exw-sidebarbars FUN_004072bf
decompile/asm. The 7f.4 gloss is confirmed and COMPLETED: the "mask
bank" is NOT EXE content and NOT 512 B — it is a RUNTIME noise ring in
.bss. All items [verified] (decompile + asm agree).

1. **FUN_00401ae6 = the static/dither blit** [verified, Watcon reg
   args EAX,EDX,EBX,ECX,ESI,EDI; call sites set the registers
   explicitly at 0x4073bb..0x4073d3 etc.]: signature
   `(y, height, x, width, src_off, mode)`:
   - dest = `[0x4edb3c] + y*[0x4edb40] + x` (framebuffer base +
     pitch), rows advance by pitch; bracketed by FUN_00425a8b /
     FUN_00425a8a (the acquire/release pair — presentation only).
   - `mode == 0` (EDI=0, the DEAD/UNOCCUPIED path): per row a plain
     `rep movsb` of `width` bytes — zeros included, the box content
     is REPLACED by the pattern.
   - `mode != 0` (EDI=1, the HIT-FLASH path): per byte
     `if bank[b] != 0 → dest = bank[b]` (0x401b3b..0x401b46) — a
     sparse overlay, the portrait pixels under zero bytes survive.
     Both modes advance src AND dst identically; only the write is
     conditional.
   - wrap: before EACH row, `if (src_off + 2*width − 0x800) ≥ 0` →
     RESEED (not a sequential wrap): `src_off = RandB() & 0x1ff`
     (a fresh random 0..511 into the bank head, 0x401b22..0x401b39).
     A full 48×48 blit reads 2304 B > the 2048-B bank, so every
     full blit reseeds at least once.
2. **The noise bank** [verified]: 2048 B at 0x4e6ed8 (IN .bss
   0x45b000..0x4efa00 — runtime state, no EXE bytes; the queue-item
   "512-B mask bank" gloss is corrected: 512 is the reseed MASK
   `& 0x1ff`, the bank is 0x800 B). Persistent cursor dword at
   0x4ddb30. Content is strictly binary {0x00, 0xFF}:
   - boot fill [0x447b13..0x447b3a, the MissionShell staging block
     right before the LoadFile family 0x447b3f]: all 2048 bytes,
     each `RandB()&3 == 0 ? 0xFF : 0x00` — 25% white noise.
   - churn [0x448147..0x448195, the MissionShell per-frame epilogue
     after the render call 0x448094 and the 0x419f62/0x41ec81
     pair]: 15 bytes/frame — `cursor = (cursor+1) mod 2048` (asm:
     inc, store, `≥ 0x800 signed → 0`), then the byte AT the
     advanced cursor is re-randomized 25%/75%. The whole bank
     refreshes every ceil(2048/15) ≈ 137 frames — slow-crawl TV
     static. Unconditional (runs on overlay frames too — the
     sidebar passes are skipped but the epilogue is not).
3. **The portrait-pass consume** [verified 0x4072bf; extends 7f.4]:
   per slot k of the squad (DAT_0046cbd8), in order k=0,1,2:
   - alive ∧ hp ≥ 1 → portrait FUN_00401ca2(0x12+k sel / 0x15+k
     unselected, 1, 0x1E7+0x32k, 5); THEN if hit_flash(+0x2E) != 0
     the clamp>5→5 + decrement (the 7g.8 decay lives HERE) and the
     dither blit mode 1 (sparse flicker over the live portrait).
   - dead ∨ hp < 1 (inside the squad) → NO portrait, blit mode 0
     (full static replaces the box).
   - k ≥ squad size → blit mode 0 EVERY frame (the strip always
     shows 3 boxes; the unoccupied ones are pure static).
   - seed per blit: `FUN_0041ec59(0x7f6, 0x30)` [verified decompile
     exw-missionrender3 1067..]: `(RandB() & 0x7fff) / 15` clamped
     ≤ 0x7f5 (divisor = 0x8000/0x7f6 − 1 = 15; the 0x30 arg is a
     pass-through returned in EDX — it is the WIDTH the caller then
     moves into ECX). One seed draw per blit, i.e. up to 3/frame.
4. **RandB consumers joined** [verified call census]: the bank
   producers + the seeds + the intra-blit reseeds + the terrain
   edge variants (MISSIONVIEW sec 7) all draw the ONE shared RandB
   stream (0x4029b6). Per-frame order: terrain edges (inside
   FUN_00403938's terrain loop) → portrait seeds/reseeds (the
   sidebar tail of the same call) → the 15 churn bytes (the
   epilogue). The boot fill's 2048 draws precede the first frame.

Engine seam (this unit, D55): presentation only — the sim decay
(7g.8) already runs per frame in `MissionSim` (the sim hash covers
hit_flash since D53); the portrait pass READS it for the mode-1
gate and never decrements again. The noise bank + cursor + the
RandB stand-in draws (fill/churn/seeds/reseeds) are MissionScene
presentation state modeled on the shared stand-in stream (charter
T3 — the EXW interleaves them with the terrain edge variants on
one stream; the engine consumes its stand-in in the same per-frame
order: terrain edges → dither draws → churn, boot fill at
activate). The blit lands in the sidebar portrait pass after the
portrait sprite, before the bars (7f.3 tail order unchanged).

## 7j. Amendment 2026-08-21 (worker 6ab53863, the 0x4dc5d0 effect-row
family + the FUN_00420608 debris stager)

Fresh objdump census (`--start-address` slices of BEDLAM.EXW, intel
syntax, plus DGROUP `-s` dumps of the kind tables at 0x4544xx and the
mission-chain strings at 0x4588xx). Decodes the whole blink/effect-row
producer family the 7f.4 sidebar switch consumed with "producer open",
plus the 128-slot debris stager the 7g.6 death tail feeds. All items
[verified] against the asm.

1. **The 10 effect rows** [verified writers 0x40ed5e..0x40f26c, the
   shared per-case tails of FUN_0040eba0]: a 0xa0-byte array of 10
   rows × 16 B at `0x4dc5d4 + r*0x10`, layout `{ i32 x; i32 y; i32 z;
   i32 id }` (id at `0x4dc5e0 + r*0x10`). Boot-cleared 0xa0 B at
   0x4dc5d4 by the MissionShell staging block (memset-family
   FUN_00402965: ecx=0xa0, edi=0x4dc5d4 @0x447a1a, alongside 0xa00 B
   @0x4cec38 and 0x960 B @0x4cf638 — two OTHER effect arrays, census
   only). Rows are staged by EVERY pickup case tail identically:
   `r = FUN_00422038(); row[r] = { pos_x>>8, pos_y>>8, z(+0x08)+0x20,
   id }` with the per-case ids [verified]: case 1 reinforcement → 1,
   case 2 shield → 6, case 3 health → 7, case 4 score/money → 1,
   case 7 booster → 0xE, case 8 ammo → 0xC, case 9 episode → 0xD
   (7h.2's "1/6/7/0xE" set completed; case 4 reuses id 1).
2. **FUN_00422038 = the row slot allocator** [verified whole, 36 B]:
   scans k = 0..9 (`eax = k*0x10 < 0xa0`) for
   `dword[0x4dc5e0 + k*0x10] == 0` (a free row — the id word), returns
   the FIRST free k, else 9 when full (reuse the last row). Note the
   0xa0 array spans ids at 0x4dc5e0..0x4dc67c while the SCALAR
   `_DAT_004dc5d0` (4 B below row 0's x) is a SEPARATE variable —
   the blink-cursor selector, see 6.
3. **FUN_0042205c = the row tick** [verified whole; sole caller the
   MissionShell per-frame epilogue @0x448080, i.e. BEFORE the draw
   call 0x448094]: per row with id != 0: `z <= 0x190 → z += 6` else
   `id = 0` — the rows RISE 6 z-units/frame to z 400 then vanish
   (the floating pickup-icon effect).
4. **The row draw pass** [verified 0x40632d..0x4063de, inside
   FUN_00403938's tail]: per active row, iso projection
   `sx = (x−camx + y−camy)/2 + 0x124 + fine_x`,
   `sy = (x−camx)−(y−camy) + 0x118 + fine_y − z` (camera
   0x4edde4/0x4edde8, the same 2:1 as robots), viewport bounds
   0..0x23f / 0..0x266, then `FUN_0040798e(sx, sy, bank 0x46af40,
   …, stack {0x12c, z>>5, id−1, y})` — sprite-list enqueue, layer
   0x12c (the TXPAL1 composite mode). Bank 0x46af40 = **FLAGS.BIN**
   [verified load site 0x41da6d + name string 0x4588c3
   "GAMEGFX\FLAGS.BIN", staged by the mission LoadFile family
   FUN_0041df10 alongside NUMBERS 0x46af3c] — so the effect rows
   draw FLAGS.BIN sprite `id−1` (ids 1..0xE → sprites 0..0xD).
5. **FUN_00420608 = the 128-slot debris/effect stager** [verified
   head + kind table + kinds 5/16; Watcon args eax,edx,ebx,ecx then
   stack]: head = sign/map bounds on the x/y args (`>>5` vs map
   w/h at 0x4eddec/0x4eddf0), the z arg clamped 0x20..0xFF, then a
   128-slot scan (`k*0x30 < 0x1800`) over records at `0x476fbc`:
   first slot with `+0 == 0`, else the slot with the SMALLEST `+0x18`
   (LRU eviction by sequence age). Record layout (0x30 B, from the
   kind bodies): `+0x00 active(1); +0x04 x; +0x08 y; +0x0C z
   (clamped); +0x10 init 0x40; +0x14 init 0x40; +0x18 seq ctr (0 at
   stage); +0x1C kind; +0x20 physics flag / param; +0x24 start
   delay (arg9); +0x28 param (arg10); +0x2C ptr to an i16 SEQUENCE
   table (−1-terminated sprite-id walks in DGROUP)`. Kind dispatch =
   a 20-entry jump table at 0x4205b8 (kinds 1..20; 7h.2/7g.6
   confirmed kind 5 = death debris). Kind 5 [0x421327]: writes the
   record, then SIX FUN_00422287 ring calls at (x±0x20, y±0x20)
   with values 1/2/4 — the scorch-mark writer, see 8. The death
   tail's five calls (7g.6) therefore stage five debris + 30 scorch
   ring writes around the corpse.
6. **`_DAT_004dc5d0` = the sidebar BLINK-CURSOR selector** [verified
   all xrefs]: value = the SELECTED robot's SLOT + 1 (1..3).
   Producers: the robots() per-robot walk 0x40c1ae..0x40c25e — when
   `idx == [0x46cbd4] + k` (k = 0..2, squad-window base 0x46cbd4)
   and squad size `0x46cbd8 > k`: select SFX `FUN_004239ef(0xC+k, k)`
   + `FUN_004239ef(0xF, k, 1)` and `[0x4dc5d0] = k+1`; MissionShell
   entry zeroes it (0x447871); FUN_00423e1c (the selection chaser,
   0x423e8c..) re-points the selection when cursor ≠ selected+1, and
   its exit path 0x423fef clears both the cursor and word 0x4ea240.
   Consumer = the 7f.4 sidebar switch [0x407420..0x407989, verified]:
   `edx = [0x4dc5d0]`; edx ∈ {1,2,3} → blink-cursor sprite
   `FUN_00401ca2((g_frame_count & 3) + 0x51, 1, x, 0xD)` from
   GENERAL.BIN (0x4edd7c) at x = 0x1F0 / 0x222 / 0x254 (slot k =
   edx−1); any other value → nothing (0x4072b8 skip).
7. **FUN_00420549 = the debris tick** [verified whole; MissionShell
   epilogue @0x448076]: per active record: if `+0x24 (delay) != 0` →
   decrement, skip; else `+0x18 (seq) += 1`, read
   `(i16)table[+0x2C][seq]`: `== −1` → `+0 = 0` (done, slot freed);
   else if `+0x20 (physics flag) != 0` → `FUN_0040de9c(idx)`.
   FUN_0040de9c [head verified] = the per-frame debris PHYSICS +
   collision pass (walks ALIVE robots, compares positions in Q13 —
   the moving-chunk damage family, callers 0x40df45/0x41a515
   context). The kind-5 table at 0x454424 [bytes verified]:
   `{5,6,7,8,9,0xA,0xB,0xC,0xD,0xE,0xF,0x10,−1}` — 13-frame debris
   tumble, then free. The whole array is cleared 0x1800 B at
   0x476fbc by FUN_0041a4f8 (the full-reset family) alongside
   0x55ec B @0x4dedf2.
8. **FUN_00422287 = the per-tile type-DB +0x18 byte writer** [whole
   verified]: `(world_x>>5, world_y>>5)` → tile, bounds vs map w/h,
   `byte[0x4796d4 + tile*0x1E] = value` clamped < 8. This CLOSES the
   MISSIONVIEW §8.1 "+0x18 producer OPEN" question: the byte has a
   runtime writer — the debris scorch ring (kind 5 passes 1/2/4 at
   the ring tiles). CAVEAT [RESOLVED by §7j.9 below — the reader is
   raw, no mask; the ring is NINE 3×3 writes, not six]:
   7g.3's robots() reader treats byte != 0 as an ARMOR PAD
   (FUN_004100b7(idx,20)); the reader at 0x40bbab tests the raw
   byte != 0, so a death DOES arm 3×3 armor-pad tiles around each
   of the five debris — verified original semantics, now wired.
9. **The debris draw pass** [verified 0x4063e3..0x4064f4, inside
   FUN_00403938]: per active, non-delayed record: the same iso
   projection with +0x110 offsets, bounds 0x23f/0x23e, sprite =
   `(i16)table[+0x2C][+0x18]`, `== −1` skip; kind (+0x1C) ∈
   {3, 7, 0xA} → enqueue layer 0x12c else layer 0x12e (DARKPAL),
   BOTH from bank 0x4edd6c = **BLOWUP.BIN** (region variant
   BLOWUPG.BIN when `[0x4eba1c] == 1`, name strings 0x45883b/0x458827
   [verified]). The earlier "effects loop 0x4cf638 / FUN_00401e39
   draw_IMG" backlog item is a DIFFERENT family (the 0xa00 @0x4cec38
   + 0x960 @0x4cf638 arrays, boot-cleared alongside the rows —
   census only this unit).

Engine seam (this unit): presentation-side effect rows + debris
stager modeled in the mission scene over the already-landed sim
outcomes (7g.6 DamageOutcome debris, 7h.2 PickupOutcome ids) — no sim
hash inputs, pins stay frozen. The blink cursor reads the existing
selection state (the sidebar already models the select strips +
selected index). FLAGS.BIN/BLOWUP(B/G).BIN join the mission fetch
chain; the two new draw passes land in the enqueue/flush order of
FUN_00403938's tail. NOT modeled: the 0x4cec38/0x4cf638 effect-array
families, the debris physics/collision FUN_0040de9c (no corpus-path
producer), SFX. The scorch-byte write WAS the pending 7j.8 caveat —
resolved + landed by 7j.9 below.

## 7j.9 Amendment 2026-08-21 (worker 11384359, the 7j.8 scorch/
armor-pad re-verify)

Byte-precise re-dump of the reader + the writer + a FULL caller
census (`objdump | grep "call.*0x422287"`). All [verified] asm.

1. **The armor reader is RAW — no mask** [re-verified
   0x40bc57..0x40bc9f]: `imul ecx,ecx,0x1e; cmp BYTE PTR
   [ecx+0x4796d4],0; je bleed` — the phase-1 pass tests the RAW
   per-tile record +0x18 byte against zero, nothing else. Scorch
   values and pad values SHARE the byte; a death genuinely arms
   armor-pad tiles around the corpse (a survivor standing on a
   scorched tile charges +20/frame instead of bleeding −10).
2. **The writer hits the SAME byte** [re-verified
   0x422287..0x4222cd]: FUN_00422287 computes the tile as
   `line[y>>5] + (x>>5)` (bounds 0 ≤ t < map w/h, both coords
   `sar 5`), then `tile*0x1E` via `shl 4/sub/add` (= ×30, the same
   scale the reader's `imul 0x1e` applies) and writes
   `byte[0x4796d4 + tile*0x1E] = bl`; the clamp reads the ZERO-
   extended byte back as u32 (`mov dl,bl; cmp edx,8`) — `≥ 8 →
   write 7`, so stored values are always 0..7. Reader and writer
   address the identical array byte. No value family, no bit
   separation.
3. **The kind-5 ring is NINE 3×3 writes, not six** [verified
   0x421465..0x4215d8 + shared tail 0x421285..0x421291]: each ring
   call re-loads the record x/y (+0x04/+0x08) and passes world
   coords ± 0x20 (one tile in Q5); since `(x±0x20)>>5` is always
   `tile±1`, the ring is the full 3×3 TILE neighborhood of the
   debris tile, written in this exact order (call sites): TL
   0x421476 (x−0x20,y−0x20)=1 · L 0x4214a3 (x−0x20,y)=2 · BL
   0x4214d3 (x−0x20,y+0x20)=1 · T 0x421500 (x,y−0x20)=2 · C
   0x42152a (x,y)=4 · B 0x421557 (x,y+0x20)=2 · TR 0x421587
   (x+0x20,y−0x20)=1 · R 0x4215b4 (x+0x20,y)=2 · BR 0x421291
   (x+0x20,y+0x20)=1 (entered via `jmp 0x421285` with ebx=1 from
   0x4215d8). Pattern = corners 1, edges 2, center 4. A death
   stages FIVE debris (7g.6) → 45 ring writes; adjacent rings
   overlap (the ±0x10 jitter keeps debris within ±1 tile of the
   corpse tile), last-write-wins in staging order k=0..4.
4. **Caller census — kind 5 is NOT the only producer**: SEVEN
   in-family producers inside FUN_00420608, all writing the
   IDENTICAL 3×3 ring (same order, corners 1 / edges 2 / center
   4; corner values verified per kind: k3 edi=1 @0x421bc0, k4
   edi=1 @0x42192a, k5 const 1, k6/12 ebx=1 @0x420d38, k9 ecx=1
   @0x421098, k11 esi=1 @0x420eb9, k20 edi=1 @0x4209ef):
   kind 3 [0x421c50..0x421db9], kind 4 [0x4219bf..0x421aeb],
   kind 5 [0x421476..0x421291], kinds 6+12 [shared body 0x420cbf,
   ring 0x420d89..0x421291], kind 9 [0x42112c..0x421291], kind 11
   [0x420f26..0x420fd2 + tail], kind 20 [0x420a2e..0x421291].
   Kinds 1/13/14/15 (shared body 0x42129b), 2, 7, 8, 10, 16..19
   stage records with NO ring. [CORRECTED by §7j.11 item 3 —
   kinds 1/13/14/15 DO ring via the k20 tail (jmp 0x4209e9);
   kinds 2/8 write ONE center tile (values 3/4); only 7/10/
   16..19 are ring-free.] Jump table re-verified at 0x4205b8:
   k1=0x42129b k2=0x4215dd k3=0x421b11 k4=0x42186f k5=0x421327
   k6=k12=0x420cbf k7=0x420af2 k8=0x421726 k9=0x420fde
   k10=0x420c13 k11=0x420e4a k13=k14=k15=k1 k16=0x4206b1
   k17=0x420764 k18=0x420812 k19=0x4208ba k20=0x420962.
5. **ONE external producer — FUN_00424051** [census-only,
   0x424209..0x424269]: guarded by `word@[0x4e9780] != 0`; reads
   tile words at 0x4e9776/0x4e9778 (high words), scales `<<5` to
   world, then FIVE calls: center value `(RandA&3)+3` (3..6), and
   four "neighbors" via `inc/dec` of the WORLD coord by ONE unit —
   which after the writer's `>>5` floor is the SAME tile as the
   center, so the four later calls simply re-roll the one tile
   (final stored value 1..4 from the y−1 call). Either an original
   bug (intended ±0x20) or deliberate re-roll jitter; function
   purpose unidentified (calls FUN_0042394a first — SFX family).
   NOT the death path; stays unwired (host-seamed if ever needed).
   [FULLY DECODED by §7j.10 below: FUN_00424051 = the per-frame
   mission-epilogue tick — the five writes are the water-splash
   event's per-tick scorch, and the function ALSO runs the global
   +0x18 fade every frame, making every ring byte transient.]

Engine seam (this unit): `MissionSim::scorch_write` models
FUN_00422287 (world>>5, map bounds, value ≥ 8 → 7) over the
existing `armor_pads` type-DB mirror (growing it zero-padded on
first write); the death tail stages the NINE ring writes per
debris row in the EXW order. No hash input moves: `armor_pads`
is hashed only through its armor effect, the corpus gates stage
no deaths before their pins, and the default corpus pads stay
all-zero until a death. The scene's DebrisFx is untouched — the
scorch is sim state, not presentation.

## 7j.10 Amendment 2026-08-21 (worker 89d34b53, the FUN_00424051
decode — queued from 7j.9 item 5)

Full decode of 0x424051..0x424355 (772 B) + its sibling stager
FUN_00424355@0x424355 + the FUN_0042394a/FUN_0041eb28/FUN_0041bd78
helpers, from `objdump` + the full-disasm dump. All [verified] asm.
**FUN_00424051 is NOT a scorch variant — it is the per-frame
MISSION-EPILOGUE TICK** (called unconditionally at 0x447ff0 in the
MissionShell epilogue chain, immediately after the debris tick
FUN_00420549@0x447feb, before 0x423a85), and it does TWO unrelated
things:

1. **The GLOBAL +0x18 FADE** [0x42405a..0x42409e, verified]: for
   EVERY map tile (outer x over `w`@0x4eddec, inner y over `h`
   rows via the 0x4ea900 line table — w*h records), if the
   type-DB +0x18 byte (`0x4796d4 + (line[y]+x)*0x1E`) is nonzero,
   decrement it by 1. NO gate — runs EVERY frame. **This corrects
   the 7j.9 reading of the ring as persistent sim state: every
   +0x18 byte (death ring AND armor-pad charge) is TRANSIENT,
   fading to 0 in ≤ value frames (max 7). A corpse arms its 3×3
   pads for only ~1-4 frames; permanent map pads CANNOT exist
   (consistent with §MISSIONVIEW 8.1: no static +0x18 producer,
   the mirror zero-fills).** The 7j.9 "last-write-wins overlap"
   question is thereby moot — overlapping ring writes self-heal
   within 7 frames.
2. **The WATER-SPLASH EVENT TICK** — a 250-record array at
   0x4e9778, stride 0xA (10 B), spanning 0x4e9778..0x4ea13c
   (`ebp` loop 0..0x9C4 step 0xA). Record layout [verified via
   the Watcom `dword@base−2; sar 16` word loads]:
   `+0x00 x (tile i16) · +0x02 y (tile i16) · +0x04 z (level
   0..7) · +0x06 delay u16 · +0x08 age u16`. The loop
   [0x424300..0x424324]: age==0 → free slot, skip; delay != 0 →
   delay--, skip (like the debris delay); else run body+tail.
   - Body [0x4240a5..0x4241c5, gated by `g_frame_count@0x46ae68
     & 1` — ODD FRAMES ONLY (0x46ae68 = the pacer frame counter,
     PACER doc) and z != 0]: read the DAT VOLUME below via
     FUN_0041eb28(x, y, z−1, DAT@0x4edd58) == 0 (empty below):
     - z-word below ∈ water range (`0x454aac[zone@0x4edd8c]` ..
       +0x1E, the same per-zone water sprite family the renderer
       remaps, MISSIONVIEW §2c) → **ABSORB**: clear own level
       (FUN_0042394a(x,y,z,0,0)), age = 0 — record dies, nothing
       stamped (a splash into existing water).
     - else → **FALL one level**: remember own z-word S, clear
       own level, z−−, FUN_0042394a(x,y,z−1,S,0) re-stamps S one
       level down (S is 0 until the age-1 stamp below — the first
       falls are bare drops).
   - Tail [0x4241ca..0x4242fa, EVERY processed frame, age != 0]:
     the FIVE same-tile scorch writes 7j.9 item 5 pinned
     (center `(RandA&3)+3` = 3..6, four ±1-unit re-rolls 1..4 —
     after `>>5` floor all five hit the SAME tile; final value
     1..4) at `(x<<5, y<<5)`; then `age == 1` →
     FUN_0042394a(x,y,z, water_base[zone], 0) **stamps the zone
     water sprite base at z**; age++; `age == 0x28` (40) →
     stamps `water_base+0x16` (later water frame); `age ≥ 0x2F`
     (47) → FUN_0042394a(...,0,0) clears the level, age = 0 —
     **the splash dries up** (~47-frame life, stamps at ticks 1
     and 40).
   So the record semantics = a weapon-impact WATER SPLASH: soak
   the hit tile, drain downward through empty z-levels (odd
   frames), merge into water below, evaporate at 47 ticks, all
   while scorching the tile each frame.

Supporting family [all verified]:
- **FUN_0042394a@0x42394a (33 registers, `ret 4`)** — the
  per-tile z-STRUCTURE writer: tile = `line[y]+x`, record =
  `0x4796bc + tile*0x1E`; writes the TOT-mirror z-word at
  `record + z*2` (arg ecx; 0 → clear), the SEEN byte at
  `record+0x10+z` (`sete` of the byte arg when ecx != 0), and
  the **DAT volume byte** `DAT[0x4edd58] + tile + zoff[z]`
  (zoff = 0x4eaacc table, z*w*h) = the byte arg. The map-edit
  primitive the splash family (and future terrain-destroy
  features) share.
- **FUN_0041eb28@0x41eb28** — read the DAT volume byte at
  (x,y,z): `byte[DAT + tile + zoff[z]]`, `0xFF → 1`, else raw.
  NOT a visibility test (corrects the 7j.9 "guard" guess).
- **FUN_0041bd78@0x41bd78** — find the first FREE z-level at
  (x,y): clamp z ≤ 7, x/y in-bounds, then scan z upward while
  volume(z) != 0 OR seen(z) != 0; returns z (7 if exhausted).
  The stager's z source.
- **FUN_00424355@0x424355 — the STAGER** (12 callers): args
  (x, y, z, delay). Gates: in-bounds x/y, z clamp ≤ 7, DAT
  volume(z) == 0 (FUN_0041eb28), z-word(z) == 0, claim byte
  `byte[0x46af58-bank + tile] == 0` (0x46af58 = a 10000-B arena
  bank alloc'd at mission load 0x41d9d7; written 1 by the ORDER
  marker family 0x425556, read by the platform stager 0x423858 —
  the per-tile order/platform CLAIM bank). Allocation: first
  age==0 slot, else the max-age slot (evict; the evicted record
  is cancelled with FUN_0042394a(old,0,0) first). Writes
  {x, y, z, delay, age=1}.
- **Producer census (11 call sites, census-only)**: 0x413212
  (inside FUN_00412f34, 9546 B), 0x417f04 (FUN_00417e2f), and
  NINE inside FUN_0041a894 (5000 B, 0x41ad84/0x41adb1/0x41af8b/
  0x41b0eb/0x41b3a7/0x41b4c8/0x41b5f1/0x41b725) + 0x41bd47
  (FUN_0041bc1c). All in the WEAPON-FIRE family (projectiles/
  impacts): 0x41bd47 stages debris (FUN_00420608) AND the splash
  at the same tile — a weapon hitting water splashes + debris.
  Delay args vary (0x41bd47 passes 0; 0x41af8b passes base +
  RandA&3). Deeper weapon decode = backlog.

Engine seam (this unit, D58): the FADE lands sim-side
(`advance_frame` tail — epilogue position after the phases:
corpus-safe, `armor_pads` has no corpus producers and
`set_armor_pads` is test-only, so pins are unmoved; the landed
D57 ring becomes transient exactly like the original). The
250-record splash system stays UNWIRED + undocumented-in-code —
no corpus-path producer exists (weapons never fire in the
gates); re-open when the weapon family decodes.

## 7j.11 Amendment 2026-08-21 (worker 804e8c9d, the
FUN_00420608 remaining-kind census + the 0x4203a5 question)

Fresh objdump of 0x42034c..0x421dd0 + a program-wide caller
census (`grep "call 0x420608"` = 47 sites, kind = the ecx
immediate at each site, function ownership via
exw-functions.txt). Answers the 7j.10 tail note and completes
the 20-kind table. All [verified] asm unless tagged.

1. **The 0x4203a5 FUN_0042394a call is NOT in the debris
   stager** — it is inside **FUN_0042034c** (0x42034c..0x4204ea,
   414 B, the function Ghidra lists between the string stubs and
   the debris tick; its only caller is the MissionShell epilogue
   **@0x448076**, i.e. right before the effect-row tick
   FUN_0042205c@0x448080). FUN_0042034c is the **delayed-arrival
   scheduler**: it walks up to 45 records (esi 0..0x654 step
   0x24) at 0x4dcdb8, layout `+0x00 active · +0x04/+0x08/+0x0C
   marker tile x/y/z [CORRECTED §7j.21 — three words, not two
   x/y pairs] · +0x10/+0x14/+0x18
   spawn x/y/z (0x4dcdc8/0x4dcdcc/0x4dcdd0) · +0x1C countdown
   (0x4dcdd4) · +0x20 target robot slot (0x4dcdd8, −1 when
   fired/skip)`. Per record [WALK SEMANTICS CORRECTED §7j.21]:
   active==0 → the walk STOPS (shared epilogue 0x41e176) — live
   records must be contiguous from record 0; the −1 store
   happens only on the fire path; countdown==0 → skip silently
   (dormant); countdown==0xa → SFX FUN_0043a48e(bank 0x4edfe0,
   0, marker.x<<5, marker.y<<5, push 2); decrement; on reaching
   0: gate `word[0x465daa + 2*tile] != 0` (a per-tile word bank)
   → clear BOTH gate words at the tile (0x465daa and 0x460dfa
   banks) → scan z 0..7 for the FIRST level whose type-DB
   z-word lies in the per-zone water range
   `[0x454ae4 + 4*zone, +0xe)` → **FUN_0042394a(x, y, level,
   0, 0)** = CLEAR that water z-structure (arg order re-pinned
   from the 0x42394a body: eax=x, edx=y, ebx=z, ecx=z-word
   value, stack=volume byte; ecx==0 branch clears word+seen) →
   stage the robot `[0x4dcdd8]` at `(x<<13, y<<13, z<<5−1)` →
   get_z_pos re-settle (FUN_0041e231, Q13>>8 args) → fill the
   8 words robot+0x1A..+0x28 with the settled z word, clear the
   robot+0x0C word → +0x20 = −1. Producers of the 45-record
   array: the 0x425xxx family (0x425daf/0x426079/0x42688c
   blocks; register-addressed countdown writes, not decoded —
   [census-only]) [CLOSED 2026-08-21 §7j.21: FUN_00425da4 =
   fixed-address stager, countdown NEVER producer-written;
   runtime arming = FUN_00433980 ride cases, countdown := 10]. The gate banks: 0x460dfa written by weapon
   fire (0x41a84f) + the 0x4227xx platform family (0x7d2/0x7d3/
   0x7d4 tile words), read by the armor pass (0x40fef4/0x410018)
   + the splash tick (0x41b8xx); 0x465daa written/read by the
   platform family + read at 0x41f0fd. [SEMANTICS PINNED
   2026-08-21 §7j.12: 0x460dfa = the tile object-word grid
   (0x7d4 while a platform stands), 0x465daa = the platform
   strength word; the arrival "clear both" burns the platform.]
   The records ALSO have a
   draw pass in the FUN_00403938 tail (0x4065f8..0x4066a3
   reads +0/+0x1C/+0x04/+0x0C) [census-only] [DECODED §7j.21:
   sprite 0x12E marker flash, width clamp(11−countdown,0,9),
   drawn only while the ride countdown runs]. **Verdict for the
   queue note: NO debris kind edits terrain — the stager body
   (0x420608..0x421dd0) contains ZERO references to the
   0x4796xx type-DB and ZERO FUN_0042394a calls; the only
   terrain writes are the FUN_00422287 ring calls.**
2. **The 20-kind table** [verified per body; args eax=x Q5,
   edx=y Q5, ebx=z (clamped 0x20..0xFF at the head), ecx=kind
   1..20, [esp+0x1C/0x20/0x24] = stack args feeding +0x24
   (delay) and +0x28 (param)]:

   | kind | body | seq table | +0x20 phys | +0x10/+0x14 | terrain writes | arrival SFX |
   |---|---|---|---|---|---|---|
   | 1 (=13/14/15) | 0x42129b | 0x454424 | 6 | 0x40 | NINE ring (via k20 tail!) | FUN_00421e60 |
   | 2 | 0x4215dd | 0x4544c2 | 0 | 0x20 | ONE center write, value 3 | FUN_00421dec |
   | 3 | 0x421b11 | 0x4544e0 | 1 | 0x20 | NINE ring | none |
   | 4 | 0x42186f | 0x4544ce | 2 | 0x20 | NINE ring | FUN_00421e60 |
   | 5 | 0x421327 | 0x454424 | 0 | 0x40 | NINE ring | FUN_00421e60 |
   | 6+12 | 0x420cbf | 0x454424 | 6 | 0x40 | NINE ring | FUN_00421e60 |
   | 7 | 0x420af2 | 0x4544f0 | 0 | 0x40 | none | none |
   | 8 | 0x421726 | 0x4544c2 | 2 | 0x20 | ONE center write, value 4 | FUN_00421dec |
   | 9 | 0x420fde | 0x454424 | 3 | 0x40 | NINE ring | FUN_00421e60 |
   | 10 | 0x420c13 | 0x4544fe | 0 | 0x40 | none | none |
   | 11 | 0x420e4a | 0x454424 | 0 | 0x40 | NINE ring | gated (see 4) |
   | 16 | 0x4206b1 | 0x454472 | 6 | 0x40 | none | FUN_00421e60 |
   | 17 | 0x420764 | 0x45448c | 6 | 0x40 | none | FUN_00421e60 |
   | 18 | 0x420812 | 0x454458 | 6 | 0x40 | none | FUN_00421e60 |
   | 19 | 0x4208ba | 0x45443e | 6 | 0x40 | none | FUN_00421e60 |
   | 20 | 0x420962 | 0x4544a6 | 6 | 0x40 | NINE ring | FUN_00421e60 |

   Field precision (corrects 7j.5's loose numbering):
   `+0x1C` stores the kind ARGUMENT verbatim (1..20 — the draw
   pass layer choice reads it); `+0x20` is a per-kind PHYSICS
   constant (0 = no physics; 1/2/3/6 = run FUN_0040de9c each
   tick — likely a physics-class index into the dword table at
   0x454510+ [census-only]); `+0x24` ← [esp+0x20] (start
   delay); `+0x28` ← [esp+0x24] (param); `+0x10/+0x14` init
   0x40 or 0x20; `+0x18` seq = 0; `+0x2C` = the seq-table ptr.
3. **CORRECTION to 7j.9 item 4**: kinds 1/13/14/15 (shared
   body 0x42129b) DO write the nine-write ring — the body ends
   `jmp 0x4209e9`, landing INSIDE the kind-20 tail whose
   straight-line flow performs the nine FUN_00422287 calls at
   0x420a2e..0x421291 (slot/param writes then the ring). The
   ring-producer census is therefore EIGHT kind-bodies / NINE
   kind numbers (1 [+13/14/15], 3, 4, 5, 6+12, 9, 11, 20), plus
   TWO single-write kinds (2 → value 3 center, 8 → value 4
   center; both jump straight onto the shared call site
   0x421291 with unmodified x/y), plus SIX no-ring kinds
   (7/10/16..19). The engine's kind-5 death model (D57) is
   unaffected — kinds 1/2/8 etc. have no engine producers yet.
4. **FUN_00421e60 = the 3-way arrival-SFX pick** [verified]:
   gated on dword 0x4ede58 != 0; picks RandA()%3 → plays
   FUN_0043a48e(bank 0x4edf64/0x4edf68/0x4edf6c, 0, x, y,
   push 2). **FUN_00421dec = the 4-way variant** [verified]:
   RandA()&3 (jump table 0x421ddc) → banks 0x4edf98/0x4edf9c/
   0x4edfa0/0x4edfa4, push 1; sole callers kinds 2+8.
   **FUN_00402975 = a 16-bit LCG** over the 32-bit state
   word@0x4ede48 (add tail constants 0x62E9/0x3619), returns
   the new high word — kind 11 gates its SFX on `al & 1`
   (call 0x420e82, i.e. a ~50% chance), and the selection
   chaser uses `& 0x3f` jitter at 0x423f50 [verified shape].
5. **The seq tables** [bytes verified, DGROUP dump]: eleven
   distinct −1-terminated i16 walks spanning 0x454424..0x454510:
   0x454424 {5..16} 13 entries (k1/k5/k6+12/k9/k11) ·
   0x45443e {44..55} 12 (k19) · 0x454458 {56..67} 12 (k18) ·
   0x454472 {68..79} 12 (k16) · 0x45448c {80..91} 12 (k17) ·
   0x4544a6 {92..104} 13 (k20) · 0x4544c2 {0..4} 6 (k2/k8) ·
   0x4544ce {29..36} 9 (k4) · 0x4544e0 {37..43} 7 (k3) ·
   0x4544f0 {17..23} 7 (k7) · 0x4544fe {24..28} 5 (k10). The
   BLOWUP sprite ids therefore partition cleanly: 0..4 =
   kinds 2/8, 5..16 = the shared tumble family, 17..28 =
   kinds 7/10/4, 29..43 = kinds 3, 44..104 = the long physics
   kinds 16..20. The dword block at 0x454510+ ({0,1,0,0,0,1,
   0x20,0x10,0,0x10,0x20, 0x1D..0x14 descending}) is a
   DIFFERENT table [census-only — likely FUN_0040de9c physics
   params indexed by the +0x20 class].
6. **The complete 47-site caller census** [verified; kind from
   the ecx immediate at each site]:
   k1 → 0x417ec3 (FUN_00417e2f), 0x4182b9 (FUN_00418250),
   0x41887d (FUN_00418835) · k2 → 0x412568 (FUN_004124a4),
   0x41273a (FUN_004126dc) · k3 → 0x410c19 (FUN_00410823),
   0x4125a9 (FUN_004124a4) · k4 → 0x4127a8/0x4127df
   (FUN_004126dc) · k5 → 0x40e771 (FUN_0040e230 — the damage
   death tail, 7g.6; the ONLY k5 site) · k6 → 0x4125e6
   (FUN_004124a4), 0x413177 (FUN_00412f34), 0x418b60/
   0x418bcb/0x418c36 (FUN_00418aa6 ×3), 0x423f6d
   (FUN_00423e1c — the selection chaser!), 0x424536
   (FUN_004244a1) · k7 → 0x418a20 (FUN_0041896c), 0x418af5
   (FUN_00418aa6), 0x418d1c/0x418d94 (FUN_00418ca4),
   0x418eac/0x418f3c (FUN_00418e26), 0x4227b9 (FUN_00422693 —
   the platform/destructible family) · k8 → 0x412816
   (FUN_004126dc) · k9 → 0x412625 (FUN_004124a4) · k10 →
   0x40dcb7 (FUN_0040dc1b), 0x41a142 (FUN_0041a028),
   0x41b185/0x41b26e (FUN_0041a894) · k11 → 0x4116dc
   (FUN_00410823) · k12 → 0x40ff7e (FUN_0040fe93), 0x4100a8
   (FUN_0040ff92), 0x412674 (FUN_004124a4) · k13 → 0x418919
   (FUN_004188d0), 0x418a4f (FUN_0041896c) · k14 → 0x41ace7/
   0x41b002/0x41b0dc (FUN_0041a894 ×3) · k15 → 0x41bd2b
   (FUN_0041bc1c) · k16..19 → 0x41b554/0x41b42b/0x41b302/
   0x41b67a (FUN_0041a894 ×4) · k20 → 0x412153 (FUN_00412010),
   0x412773 (FUN_004126dc), 0x41aebf (FUN_0041a894).
   **Corpus-path verdict**: every kind except k5 lives in the
   weapon-fire/impact families (FUN_00410823 cluster +
   FUN_00412f34 + the 0x417e2f..0x418f3c per-weapon handlers +
   FUN_0041a894/FUN_0041bc1c), the platform/destructible family
   (FUN_00422693), the selection chaser (FUN_00423e1c) and
   FUN_004244a1 — all outside the current corpus path (weapons
   never fire in the gates; the platform family is unstaged).
   k5 via apply_damage is the only corpus-reachable producer
   today, exactly what the engine models.

Engine seam: NONE this unit (census-only, D59) — the kind
table feeds the LATER debris-stager widening beyond kind 5
(backlog). Pins untouched by construction (no code change).

## 7j.12 Amendment 2026-08-21 (worker 5aa2d164, the
FUN_00422693 platform/destructible family decode)

Fresh intel objdump of 0x42223c..0x4222ce, 0x41a8c0..0x41a9b0,
0x41bd54..0x41bd77, 0x41eb4c..0x41eb64, 0x422600..0x423100 (+ the
DGROUP words 0x454a20..0x454a9c and the jump tables 0x4225d0/
0x4225e4). Answers the queue item: the two gate banks, the two
FUN_0042394a calls, the k7 staging, the 0x7d2/0x7d3 producer, and
the type-DB tail producers. All [verified] asm unless tagged.

1. **The gate banks are an OBJECT-PRESENCE WORD GRID, not a TOT
   mirror** [verified 0x41a8c0..0x41a906]: the weapon-fire ray step
   (FUN_0041a894) reads `word[0x460dfa + 2*tile]` each step and
   dispatches: **0** → projectile passes; **0x7d2/0x7d3** → ray
   stops, no effect (the hazard/phase-clamp tiles); **0x7d4** →
   `FUN_00422693(x, y, damage=esi)` (call 0x41a8ff, ebx=esi = the
   weapon damage/charge); **any other word n** → destructible
   OBJECT record n−1 at the 0x46cbf4 array: hp@+0x10 −= damage;
   ≤0 → hp=0 and flags byte@+0xD |= 0x40 (triggered/destroyed);
   −1 hp = immune. The 0x46cbf4 array (ptr@0x46cbf4, count@
   0x46cbe8, stride 0x14) is therefore the map's DESTRUCTIBLE
   OBJECT/TRIGGER LIST: `+0/+4/+8` spawn x/y/z, `+0xC` id byte,
   `+0xD` flags (0x40 = fired), `+0x10` hp (−1 = never dies).
   Consumers of the grid besides the ray: robots() 0x7d2 hazard +
   0x7d3 clamp (§7g.5), the armor pass 0x40fef4/0x410018, the
   splash tick 0x41b8xx, the arrival scheduler (§7j.11), and
   weapon fire's own object-stamp loop (0x41a84f writes word=si
   over a tile run). The §7c "TOT mirror DAT_00460df8" gloss is
   superseded: 0x460dfa is a runtime-mutable word bank, zeroed at
   load, written by FUN_00422f18/0x4228ce/0x41a84f.
2. **FUN_00422693 = the PLATFORM DAMAGE entry** [verified
   0x422693..0x422832] (single caller 0x41a8ff, args eax=x, edx=y,
   ebx=damage): bounds-check; scan z 0..7 for the FIRST level
   whose type-DB z-word (dword[0x4796ba+30·tile+2z]>>16) lies in
   the zone water range `[0x454ae4+4·zone, +0xe)` — none → exit
   (only real platforms take damage); `cx = word[0x465daa+2·tile]`
   (the STRENGTH bank), diff = (i16)cx − damage:
   - **diff ≤ 0 → DESTROY**: `FUN_0042394a(x, y, z, 0, 0)` (call
     0x422750 — CLEARS the platform's water z-structure: word 0,
     seen cleared, volume 0); both banks zeroed; then FIVE kind-7
     debris via FUN_00420608 (call 0x4227b9, ebp = 0,2,4,6,8):
     x = (x<<5)+(RandA&0xf)+8, y = (y<<5)+(RandA&0xf)+8, z<<5,
     delay = k·2, param = −1 (the queue's k7 staging).
   - **diff > 0 → WEAKEN**: word[0x465daa+2·tile] = cx−damage
     (writer 0x4227d5); `FUN_0042223c(x, y, 4)` (scorch +4, see
     5); if strength ≥ 100 and (diff < 200 or new < 100) →
     `FUN_00422832(x, y, z, new_strength)` (ring spread, see 3) —
     i.e. a big hit or a drop below half (100 of 199/300) makes
     the platform spawn neighbors; both paths store the site
     `0x4dc5c8/0x4dc5cc = x/y` (the creep seed, see 4).
   So 0x465daa = per-tile PLATFORM STRENGTH (0 = none) and
   0x460dfa = the tile OBJECT WORD (0x7d4 while a platform
   stands). The arrival scheduler's "gate ≠ 0 → clear both banks"
   (§7j.11) = arrivals burn the platform they land on.
3. **FUN_00422832/FUN_004228ce = the platform SPREAD ring**
   [verified 0x422832..0x422a84]: FUN_00422832(x,y,z,strength)
   calls FUN_004228ce for the EIGHT 3×3-minus-center neighbors
   (same geometry as the debris scorch ring). FUN_004228ce
   builds one new platform tile if ALL hold: in bounds; BOTH bank
   words 0; tile-claim byte @0x46af58-arena 0; no live robot
   standing in the tile's SE 2×2 sub-block (pos>>13 == tile or
   the three SE neighbors); z ≥ 1; type-DB z-word@2z == 0 (empty
   stack level); z-plane-A byte ([z·4+0x4eaacc] rowstarts +
   [0x4edd58] base) == 0; z-plane-B byte ([z·4+0x4eaac8] +
   base + tile) == 1. Then it writes the platform:
   `FUN_0042394a(x, y, z, [0x454ae4+4·zone], volume 2)` (call
   0x422a54 — creates a WATER z-structure at the empty level:
   word = zone water base, seen=1); `word[0x460dfa+2·tile] =
   0x7d4` (writer 0x422a61); `word[0x465daa+2·tile] = strength`
   (writer 0x422a73); `FUN_0042223c(x, y, 4)`. Platforms are
   walkable water: build = write a water z-word + gate words;
   destroy = clear the water z-word + banks.
4. **FUN_00422a9c = the platform CREEP tick** [verified
   0x422a9c..0x422c78] (epilogue call 0x44808a, right after
   FUN_00422cc2@0x448085 and FUN_0042205c@0x448080): 1/32 gate
   (RandA&0x1f == 0), else exit; start from the last damage site
   0x4dc5c8/0x4dc5cc + RandA&7 jitter −3; require
   word[0x465daa+2·tile] ≠ 0 (gate read 0x422b0d — the queue's
   third "writer" site is this reader); find the FIRST water
   level (same
   z-scan); RandA&3 → 4-way direction {up, right, down, left};
   walk the ray while each next tile's z-word is in the water
   range; one step back onto the last water tile; if in bounds →
   `FUN_00422832(x, y, z, 199)` (build ring, strength 0xC7) and
   update 0x4dc5c8/cc to the new tip. Bridges GROW across water
   one hop per lucky frame from the last touched platform tile.
5. **FUN_0042223c = the type-DB scorch INCREMENT writer**
   [verified 0x42223c..0x422287]: `byte[0x4796d4+30·tile] +=
   value; if ≥ 8 → 7` — the +0x18 byte's SECOND producer beside
   the absolute writer FUN_00422287 (7j.8/7j.9); platform damage
   and platform build both add 4 (clamp 7). The byte decays via
   the §7j.10 fade, so platform hits leave ≤7-frame scorch.
6. **FUN_00422f18 = the 0x7d2/0x7d3 STAMPER — the §7g.5 word
   producer, CLOSED** [verified 0x422f18..0x422fd1] (mission-load
   call 0x447b8f): for EVERY tile, for z 0..7: z-word in
   `[0x454a20+4·zone, +4]` → `word[0x460dfa+2·tile] = 0x7d2`;
   z-word in `[0x454a3c+4·zone, +4]` → `= 0x7d3` (writers
   0x422f9a/0x422fc6; later z can overwrite earlier). Zone bases
   [bytes verified]: 0x7d2 {0x20,0x49,0x49,0x34e,0x49,0x77,0x77},
   0x7d3 {0x49,0x77,0x77,0x49,0x4e,0x4e,0x349} — the tables sit
   directly before the 7h pickup tables 0x454a58/74/90.
7. **FUN_00422fd1 = the type-DB TAIL stamper (rect list)**
   [verified 0x422fd1..0x423081] (mission-load call 0x447ba3):
   walks up to 45 records @0x4dcae8 stride 0x10 (STOPS at the
   first word@+0 == 0) `{+0 active, +2 x0, +4 y0, +6 w, +8 h,
   +0xA variant byte, +0xC countdown, +0xE flag}` [field map
   verified; the array is immediately followed by the §7j.11
   arrival array at 0x4dcdb8]. Records with word@+2 (type) ≥ 3
   stamp every tile of their rectangle:
   `byte[0x4796d5+30·tile] = variant<<4` and
   `byte[0x4796d6+30·tile] = (type==3 ? 0 : 0x80)` (writers
   0x423061/0x423070/0x423078). These are type-DB +0x19/+0x1a
   (MISSIONVIEW record base 0x4796bc) — the MISSIONVIEW §8.1
   "+0x1a height-bias" producer plus the unlisted +0x19 variant
   nibble; +0x1b/+0x1c (0x4796d7/d8) remain open.
8. **FUN_00422cc2 = the delayed-TRIGGER timer tick** [verified
   0x422cc2..0x422e0a] (epilogue 0x448085): 32 records @0x4ea828
   stride 0x18 `{dword payload, word@+4 countdown}`; producers
   FUN_00422c9b (find-free + set countdown 8) and FUN_00422e0a
   (payload = FUN_00439c20() result, then rec-id match →
   FUN_004245c9(x<<5, y<<5, z<<5) — census). On countdown 0 the
   payload's LOW and HIGH bytes each select 0x46cbf4 records by
   id: flags |= 0x40; their z-plane-A occupancy byte is CLEARED;
   `FUN_0041bd54(x, y, z, word[0x454a90+4·zone])` — writes the
   BARE-FLOOR z-word (the 7h floor-word table!) + seen=1 into
   the type-DB. SFX FUN_004239ef(0x22, 3) fires once per expiry.
   FUN_0041bd54 [verified 0x41bd54..0x41bd77] = the fast
   z-structure writer: `word[0x4796bc+30·tile+2z] = cx`,
   `byte[0x4796cc+30·tile+z] = 1` — FUN_0042394a minus the DAT
   volume write. This is a SECOND floor-word context beside the
   7h pickup consume (§7h item 3): trigger expiry converts a
   tile to bare floor at level z with the SAME 0x454a90 table —
   the 7h.3 PICKUP word producer (staging pickup words into the
   mirror rows) remains open.
9. **FUN_00422e5e = the void-tile marker check** [census,
   0x422e5e..0x422f09]: args (Q5 x,y,z); tile byte via
   FUN_0041eb4c(x,y,z,[0x4edd58]) == 0xFF (void) → scan 999
   stride-8 records @0x4e44f8 `{word active, word x, word y,
   word z}` for a match; if the hit is the active index
   [0x4eb9fc] → set it −2, bump counter [0x4eb9f4]; miss → −1.
   Six callers in the 0x433xxx–0x435xxx script family. The
   0x422600 trigger dispatcher + FUN_00423081 zone-script tick
   are census-only (consumers of the same 0x46cbf4/0x4dcae8
   arrays; 0x422600: per-zone trigger-code jump tables
   0x4225e4/0x4225d0 → codes {5, 0x84, 0x6f, 0x7e, 0x80, 0x79,
   0x88, 0x2f, 0x2710} matching a record id → FUN_00422832(rec
   x,y,z, 300) — destroying the right object BUILDS a bridge
   ring).

Corpus-path verdict: the family's callers are the weapon ray
(FUN_0041a894), the MissionShell load/epilogue (0x447b8f/
0x447ba3/0x448085/0x44808a) and the 0x433xxx+ script family —
weapons never fire and platforms never stage in the gates, so the
engine seam stays NONE this unit (D60): banks/timers remain
unwired (never-invent). What CLOSED: the 0x7d2/0x7d3 producer
(§7g.5 open), the type-DB +0x19/+0x1a producers (MISSIONVIEW
§8.1), the trigger-expiry floor-word WRITE (second 0x454a90
context, §7h item 8 above — the 7h.3 pickup producer stays
open), and the "non-splash non-arrival z-writer" pair (both are
platform water-word writes at 0x422750 clear / 0x422a54 create).

## 7j.13 Amendment 2026-08-21 (worker b7f866b6, the
FUN_0041a894 weapon-impact ray head - first hop)

Fresh objdump (PE32, `objdump -d -M intel --section=BEGTEXT
game-data/BEDLAM/BEDLAM.EXW`) of 0x41a4f8..0x41bc60 + all 17
call sites of FUN_0041a894 + FUN_00419aff's head. NOTE: objdump's
linear sweep desyncs just before 0x41a894 (`00 56 57` eaten as
one instruction); the true prologue bytes are `56 57 55 81 ec 18
01 00 00` (push esi/edi/ebp; sub esp,0x118) - Ghidra's
FUN_0041a894 boundary is right. Answers the queue item: the head
dispatch, the damage source, the callers, and the 0x41a84f stamp
loop. All [verified] asm unless tagged.

1. **FUN_0041a894 = the per-tile WEAPON-IMPACT OBJECT RESOLVER -
   it does NOT walk.** Calling convention [verified 0x41a894 +
   all sites]: `eax` = x Q13, `edx` = y Q13, `ecx` = chain/step
   counter (homed to [esp+0x10], incremented per chained
   detonation), `ebx` = DAMAGE, `[stack]` = score-award flag
   (read at [esp+0x128]; 0x41b73f `cmp [esp+0x128],0` gates the
   destroy-tail score award). The stack word is the SAME push
   that FUN_00419aff consumes at the fire sites (Watcom cdecl
   leaves it on the stack), so the fire sites' `push 0x1`
   simultaneously selects the weapon-stat field and arms the
   score. Return value: **0 = pass-through (keep flying), 1 =
   object destroyed (0x41a8c7 `xor eax,eax` vs 0x41bc0b
   `mov eax,1`)**. The ray STEP lives in the callers (see 5).
2. **Head dispatch [verified 0x41a8a3..0x41a9c3]** (extends
   7j.12 item 1): x<0 ∨ x>>13 ≥ map_w ∨ y<0 ∨ y>>13 ≥ map_h →
   return 0; tile = (y>>13)·w + (x>>13); word =
   grid[0x460dfa+2·tile] (read via the dword-at-0x460df8>>16
   idiom): **0 / 0x7d2 / 0x7d3 → return 0** (pass-through: empty,
   hazard, phase-clamp); **0x7d4 → FUN_00422693(x_tile, y_tile,
   damage)** (call 0x41a8ff, ebx=esi=entry ebx) → return 0;
   **n → destructible object rec n−1 @0x46cbf4**: flags byte
   (id dword high byte) & 0x40 → already destroyed → return 0;
   hp == −1 → immune → return 0; hp −= damage: >0 → store,
   return 0; ≤0 → destroy (hp=0, flags |= 0x40) → destroy tail →
   return 1. So the platform word and objects both STOP nothing -
   only a DESTROYED object returns 1 (the callers that check it
   stop their walk; FUN_00412010 does not).
3. **Destroy tail [verified 0x41a95d..0x41bc06]**: notify
   [0x46cce4]=2; zone [0x4edd8c] ≠ 1 → FUN_00448b80(rec idx);
   FUN_00422e0a(id dword) (the 7j.12 delayed-trigger payload
   producer) and FUN_00422600(id dword) (the per-zone trigger
   dispatcher - destroying the right object builds a bridge);
   id-table dword@+0xE == 0xb AND language latch [0x4eba1c] == 1
   (GER) gate; the 7j.11 debris kinds (k10 0x41a142-family sites
   live here: k14 0x41ace7/0x41b002/0x41b0dc, k16..19
   0x41b554/0x41b42b/0x41b302/0x41b67a, k20 0x41aebf) with the
   counter+2 as delay base ([esp+0x68]); a 4-iteration loop
   (0x41b699..0x41b73a) staging the water-splash records
   (FUN_0041bd78 = the 7j.10 stager) + FUN_00424355 per corner
   with RandA jitter (the 7j.10 "one co-staging debris" is this
   loop); score award gated by the stack flag: type 0xb →
   [0x4dd40c] += 10 else += type value, [0x46ccf0] = 2; then the
   **four PERIMETER CHAIN WALKS** (0x41b771..0x41bc06): the N row
   (y−1, x from −1 to id-table W), and the W/S/E edges (walks at
   0x41b8d5, 0x41b9da, 0x41ba1e): each tile reads the grid word;
   a neighbor object (word−1 > 0) that is alive (id dword <
   0x4000) and CHAINABLE (id-table word@+0xC = [0x4dedfe+78·id]
   ≠ 0) is recursively detonated: RandA&3 == 0 → counter++, then
   `FUN_0041a894(edge pos Q13, counter, damage 0x3E8 = 1000,
   forwarded flag)` (self-calls 0x41b895/0x41b9d0/0x41badf/
   0x41bc01 - the 4 "self" sites of the 17). Destroying an
   object chain-detonates its chainable neighbors at damage
   1000 regardless of the RandA roll (the roll only bumps the
   delay counter).
4. **The 0x41a84f stamp loop is FUN_0041a7f0, a separate
   function (125 B)** [verified 0x41a7f0..0x41a86c, single
   caller 0x41a7df]: args (eax = rec+0 spawn x, edx = rec+4
   spawn y, ecx = rec_index+1, ebx = id dword); stamps
   `word[0x460dfa + 2·(yline[0x4ea900+4·(y+i)] + x + j)] = si`
   for j in 0..W−1, i in 0..H−1 (W/H = id-table words@+2/+4) -
   the object's FOOTPRINT into the object grid. Its caller block
   (0x41a7ad..0x41a7e6, the tail of FUN_0041a4f8, mission-load
   call 0x447b76) re-stamps every 0x46cbf4 record whose id dword
   ≠ −1 and sets `hp = id-table dword@+8` first. **The OBJECT
   TYPE TABLE [verified 0x41a5d6..0x41a7a8]**: base 0x4dedf2,
   stride 0x4E (78), 0x11A (282) records, parsed from the mission
   file by FUN_0041a4f8 (word reads via FUN_0041cccb; record
   parsed only when control word@+0 == 1): W word@+2, H word@+4,
   D word@+6, hp dword@+8, CHAIN word@+0xC, type dword@+0xE
   (0x4dee00+78·id; 0xb = score-10 type), jitter words@+0x16/
   +0x18/+0x1A/+0x1C, count word@+0x12, FOUR scratch bank ptrs
   @+0x30/+0x34/+0x38/+0x3C each W·H·D words (arena pointer
   0x46ad5c). This is the mission-file destructible-object DB
   that 7j.12's 0x46cbf4 array instantiates.
5. **The ray stepping - caller census of all 17 sites**
   [verified]:
   - **FUN_00412010 = the PROJECTILE TICK** (the actual ray
     walk): 50 records @0x4cc654 stride 0x22 {word@+0 active,
     x@+2, y@+6, z@+0xA, vx@+0xE, vy@+0x12, vz@+0x16} -
     per-frame x+=vx, y+=vy, z+=vz (z Q13, 0x2000 = 1 level),
     deactivate on bounds exit; terrain probe FUN_0041eaa1(z);
     impact branches: type-1 (0x41222d) → FUN_004126dc(idx) +
     FUN_0041a894(damage = FUN_00419aff(0x65)); type-2
     (0x41245d) → FUN_004126dc + FUN_0041a894(FUN_00419aff(0x66))
     + FUN_0041bc1c(same) + deactivate; type-3 (0x41241f) →
     FUN_004126dc + deactivate. eax ignored (projectile dies on
     terrain, not on objects).
   - **FUN_00410823 = the robot FIRE controller** (6102 B,
     per-weapon anim state machine; weapon id = word@ rec+0xE,
     0x4c71f4-family records): 8 sites, all `push 1` (score
     armed), all damage = FUN_00419aff(weapon_id, 1):
     0x410ae0 (anim state 2, weapon from rec, muzzle-offset
     target), 0x410c8d (weapon 5, + FUN_0041bc1c +
     FUN_004124a4), 0x410de1/0x410e08/0x410e27/0x410e42 (weapon
     0x1a = 26: FOUR quadrant hits at ±0x1000 Q13 half-tile
     offsets - a blast), 0x4118ad (weapon 0x24 = 36, +
     FUN_0041bc1c), 0x411f3f (weapon 0x29 = 41, +
     FUN_0041bc1c + FUN_004124a4).
   - **FUN_0040fe93 / FUN_0040ff92 = the tile-0x62 TRAP
     resolvers**: FUN_0040fe93(idx) checks the CURRENT tile of a
     0x4c69e4-array record (x/y/z @+0/+4/+8), FUN_0040ff92
     checks a FUN_004128ec-probed position; both require type-DB
     tile byte == 0x62 (FUN_0041eb4c) AND grid word ≠ 0 →
     FUN_0041a894(damage 100, flag 0); destroyed → 5× kind-12
     debris (RandA&0xF/&0x1F jitter, delays 0/2/4/6/8 - the
     7j.11 k12 sites 0x40ff7e/0x4100a8). OPEN: FUN_0040fe93
     indexes 0x4c69e4 with a 160-byte stride (20·i << 3) while
     the canonical robot stride is 0xA8 (`imul 0xa8` sites) -
     either a second array aliasing the base or an original-code
     quirk; unpinned [census].
   - **FUN_004244a1 = the script/explosion entry** (site
     0x424503): tile-coordinate args, FUN_00424355 first, then
     FUN_0041bc1c(x, y, 5000) + FUN_0041a894(x, y, 0, damage
     0x1388 = 5000, flag 1) - a kill-anything scripted blast
     (the 7j.11 k6 site 0x424536's function).
   - **FUN_0041bc1c** (312 B, 10 callers) = the SIBLING resolver
     (terrain/robot damage) always paired at the fire/impact
     sites - decode deferred (backlog: the family's second hop).
     [Decoded 7j.14: terrain-STRUCTURE resolver, see §7j.14.]
   - **FUN_00419aff(weapon_id, field)** (381 B, 28 callers) =
     the per-weapon STAT lookup (id-switched, 0x46cbf8 base
     field) feeding every damage argument [census].
6. **Erratum to 7j.12 item 1**: "weapon fire's own object-stamp
   loop (0x41a84f)" - the stamp loop is FUN_0041a7f0 invoked from
   the mission-load restamp pass (FUN_0041a4f8 tail), not from
   FUN_0041a894. The §7c "TOT mirror DAT_00460df8" supersession
   stands.

Corpus-path verdict: weapons never fire in the gates (no corpus
producer reaches any of the 17 sites - the fire controller,
projectile tick, trap checks and script blasts are all player/
script-driven), so the engine seam stays NONE this unit (D61,
docs-only). What CLOSED: the queue item's three questions (walk
= the callers' projectile tick; damage = ebx from
FUN_00419aff/100/1000/5000; callers = the 17-site census), the
object TYPE TABLE (0x4dedf2/0x4E/282 from the mission file), and
the 7j.10/7j.11 producer geography (the debris/splash co-staging
lives in the destroy tail). Re-opens cleanly at: FUN_0041bc1c
(the terrain/robot resolver), the FUN_00410823 weapon-anim
machine, and the type-table's remaining words.

### 7j.13 Erratum 2026-08-21 (worker 22c1c14b - independent
cross-check; the object-type field map re-anchored)

An independent decode of the same region (FUN_0041a4f8/
FUN_0041a7f0/FUN_0041a894 + all 17 sites, objdump) confirms all
of 7j.13's findings EXCEPT the item-4 field offsets, which mix
two bases. The 7j.13 W@+2/H@+4/D@+6 and ptrs@+0x30..+0x3C are
inconsistent with the runtime consumers (the +0x30 ptrs would
collide with the effect entries). Corrected map [verified, every
field double-anchored on a runtime read], base B = 0x4dedf2 +
78·id — CORRECTED AGAIN by the 7j.16 verification pass: the
draft's W/H/D anchors below are dword reads SAR'd 16, so they
consume word@+2/+4/+6, NOT +0/+2/+4 (see 7j.16 item 7 for the
instruction proofs; the ORIGINAL 7j.13 W/H/D offsets were right):

| field | offset | anchor |
|---|---|---|
| gap (unconsumed) | word@B+0x00 | no runtime consumer found [open] |
| W = X-extent | word@B+0x02 | stamper column-run bound (0x41a857 `[0x4dedf2+78id]` dword>>16) + scanner x-center (7j.16) |
| H = Y-extent | word@B+0x04 | restore row count (0x41aa02 dword>>16) + scanner y-center (7j.16) |
| D = Z depth (levels) | word@B+0x06 | restore level addend spawn_z+D cap 8 (0x41aaf9 `[0x4dedf6]` dword>>16); loader bank size 2·W·H·D (0x41a6fc..0x41a723) |
| default hp | dword@B+0x08 | hp init read 0x41a7d3 `[0x4dedfa+78id]` |
| chainable flag | word@B+0x0C | `[0x4dedfe+78id]` reads in the perimeter walks |
| type/score value | dword@B+0x0E | `[0x4dee00+78id]` reads (0xB = score-10 class) |
| effect-entry count | word@B+0x12 | loader's nonzero-selector count `[0x4dee04+78n]` |
| gap | word@B+0x14 | no consumer found |
| FIVE 8-B destroy-effect entries | B+0x16..B+0x3E | selectors at B+0x16+8k (k=0..4): destroy tail 0x41ac58 reads word@`[0x4dee08+78id]`, steps +8, exits at +0x28; selector 1..9 → jump table @0x41a870 (cases 0x41ac77/0x41b298/0x41b3c1/0x41b4ea/0x41b613/0x41b1fe/0x41b11c/0x41af96/0x41ae53); entry payload words = the per-case staging args |
| four W·H·D-word template bank ptrs | dwords@B+0x3E/+0x42/+0x46/+0x4A | loader writes `[0x4dee30/34/38/3c]+78n`; the restore consumes t2@B+0x46 (per-level z words) + t3@B+0x4A (seen/empty markers) |

Proof of fit: 0x16 + 5·8 = 0x3E, and 0x3E + 4·4 = 0x4E = the
stride — the record closes exactly with no overlap. The 7j.13
"jitter words@+0x16/+0x18/+0x1A/+0x1C" label mis-strides the
selector words (they sit at +0x16/+0x1E/+0x26/+0x2E/+0x36).
Why the original erred: the loader's parse gate reads a dword at
`[78n+0x4dedf0]` (2 below B) == 1 — a parse-cursor artifact that
suggests a 0x4dedf0-anchored record; but hp/chain/type/effects/
banks all anchor at B under BOTH maps, and only the B anchor
closes the record at 0x4E.

Corroborations from the same pass [verified]: the FUN_0041a4f8
memsets confirm the counts exactly — type array 0x55EC = 282·78,
object array 0x9C40 = 2000·0x14 at [0x46cbf4], grid 0x460dfa and
strength 0x465daa 0x4FB0 each, robot bank 0x476fbc 0x1800; and
the load pass forces a record DEAD (id dword = −1) when its
spawn x or y dword == −1, with [0x46cbe8] = last-live-index+1.

## 7j.14 Amendment 2026-08-21 (worker d37fb3a2, the weapon-fire
family SECOND HOP: FUN_0041bc1c + FUN_0041eaa1 + the two co-stager
heads)

Method: `ExwWeaponFire2.java` (-process BEDLAM.EXW -noanalysis),
dump = `ghidra-project/exw-weaponfire2.txt` (full decompile +
listing + depth-1 callees + 0x30 pre-call arg windows for all 4
roots). All facts below [verified] against that dump unless tagged.

1. **FUN_0041bc1c = the TERRAIN-STRUCTURE damage resolver** (312 B,
   10 callers) — the sibling of FUN_0041a894 paired at every
   fire/impact site. NOT a robot-armor path, NOT the object grid
   (0x46cbf4), NOT the platform bank (no FUN_00422693 call):
   - Register args: EAX x Q13, EDX y Q13, EBX damage. Tile =
     x>>13, y>>13, bounds-gated by DAT_004eddec/DAT_004eddf0.
   - Scans the **TERRAIN-STRUCTURE array**: records @0x4cccf8 +
     i·0x20, i < DAT_0046ccd4 — {+0x00 active dword, +0x10 hp
     dword, +0x14 x tile, +0x18 y tile, +0x1C z level}. NEW bank
     (distinct from every array pinned so far).
   - Match (active≠0 ∧ x ∧ y): hp −= damage; hp > 0 → return
     (survivor takes NO floor/z/scorch write — pure hp subtract).
   - Destroyed (hp ≤ 0): active = 0, then the tile is converted
     back to floor IN ALL THREE RENDER/VOLUME BANKS:
     (a) TOT mirror word @0x4796bc + 30·tile + 2z ←
         word[0x454a04 + zone·4] (the zone FLOOR word,
         _DAT_004edd8c = terrain-set idx, cf. 7j.12 zone bases);
     (b) seen byte @0x4796cc + 30·tile + z = 1;
     (c) DAT volume byte @DAT[0x4edd58] + y·W + x + z·
         _DAT_004eddf4 = 0.
     Then FUN_00420608(x<<5, y<<5, z<<5, kind 0xF, 0, −1) debris
     + FUN_0041bd78(x,y,z)→FUN_00424355(x,y,z′,0) splash. The
     same triple FUN_0042394a writes (7j.10), inlined here with a
     floor-word source instead.
   - **ID convention pinned**: external refs are 1-BASED — the
     fire controller reads dword[0x4cccd8 + id·0x20] (site
     0x411e41, id = animrec field &0x1FFF) = record id−1's active
     field; dword@0x4cccd8 itself is the id-0 guard slot. Same
     id+1 convention as the tile-word grid (7j.12/7j.13).
   - 10 call sites, uniform args (damage = FUN_00419aff(weapon)):
     8 in FUN_00410823 (0x410ca2 weapon 5, 0x410e59/0x410e72/
     0x410e87/0x410e9e = weapon 0x1a ×4 quadrants, 0x4118c2
     weapon 0x24, 0x411f24 weapon 0x29, 0x410af9 rec-weapon —
     weapon id = dword[0x4c71f2]>>16 = anim rec 0's kind word),
     1 in FUN_004244a1 script blast (damage 0x1388), 1 in
     FUN_00412010 projectile impact (weapon 0x66).
2. **FUN_0041eaa1 = the projectile TERRAIN-HEIGHT probe** (135 B,
   3 callers, all in FUN_00412010):
   - Args EAX x Q5, EDX y Q5, EBX z. h = FUN_0041eb28(x>>5, y>>5,
     z>>5, DAT) — the DAT volume byte at the containing tile; 0
     → return 0 (air, no terrain).
   - h ≠ 0 → per-PIXEL height: bank ptr = [0x4edd60] (a per-level
     bank-pointer array); entry = (h−1)·4 + 2 → dword, +6 header;
     height byte = bank[(y&31)·32 + (x&31)]. Return 1 iff
     z ≤ (z>>5)·0x20 + height (terrain top at that sub-tile pixel
     reaches z). NEW: the 32×32 byte height-map banks behind
     [0x4edd60] — same family as the effects/draw_IMG bank
     arrays (MISSIONVIEW §5d backlog).
   - Sites: 0x4120dc main post-move test; 0x4121fd/0x4122e1
     per-axis variants (bounds-gate deactivates word[rec+0]=0
     when x/y leave [0, W/H<<5)). Hit → FUN_004126dc(idx) +
     the impact family. [census open] site-1's z arg is
     ([rec+0xA] − rec[+0x1A clamped ≤7]) << 5 — the projectile
     record's z encoding is NOT plain Q13 (per-axis sites pass
     [rec+0xA]>>8); left unpinned, head-bounded unit.
3. **FUN_004124a4 = the WEAPON-ANIM debris disburser** (568 B, 9
   callers, ALL in FUN_00410823): arg EAX = anim record idx; rec
   = 0x4c71f4 + idx·0x36; kind = word[rec+0] (= the weapon/anim
   id space). Coords x@+0x12, y@+0x16, z@+0x1A (all >>8), param
   dword@+2 → FUN_00420608(x, y, z−10, K, 0, param). Kind map:
   - weapons 2..4 → debris K2 with x/y jitter (RandA&7)−3;
     weapon 5 → K3; weapon 0x24 → K6; weapon 0x29 → K9;
     weapons {0xE, 0xF, 0x13, 0x17, 0x1A, 0x1F} → K0xC;
     weapons 9..0xB → NO debris (word still cleared);
     weapons 0xC/0xD → no-op (word kept); everything else →
     no-op. Every debris branch clears word[rec+0].
4. **FUN_004126dc = the PROJECTILE debris disburser** (364 B, 6
   callers): arg EAX = projectile idx; rec = 0x4cc654 + idx·0x22;
   type = word[rec+0] — REFINES 7j.13's "active": +0 is the TYPE
   word, 0 = free. Types pinned: 1 → debris K2, 0x65 → K0x14,
   0x66 → K8, 0x67 → K4, 0x68 → K4; coords (x>>8, y>>8, z>>8)
   — NO z−10 here (vs FUN_004124a4); every branch clears the
   type word. Projectile type ids ARE weapon-stat ids (the 7j.13
   impact damage FUN_00419aff(0x65/0x66) reads the projectile's
   own type). Callers: 4 in FUN_00412010 (the 3 probe-hit sites
   + axis-counter expiry 0x412425/0x41243f) + 2 in FUN_004197d4
   = the projectile-vs-ROBOT proximity walker (|dx| < 0x10 Q8 ∧
   |dz| < 0x20 vs robot rec @0x4c69e4+0xA8·i, z@+8 → expire on
   robot hit, then 0x65 damage lookups) — the ROBOT-HIT arm of
   the projectile family [census-level].
5. **7j.10 addendum (splash gates + eviction)** [verified in the
   depth-1 dumps]: FUN_0041bd78(x,y,z) scans UP from min(z,7)
   for the first level with DAT byte 0 ∧ seen byte 0 (else 7).
   FUN_00424355 stages only when the target level is DAT-empty ∧
   TOT word == 0 ∧ tile-claim byte[0x46af58 + tile] == 0
   (0x46af58 = the 7j.10 tile-claim bank — third reader found);
   when all 250 splash records are active it EVICTS the max-age
   record and flushes it with FUN_0042394a(oldx, oldy, oldz,
   0, 0).

Corpus-path verdict: unchanged from 7j.13 — no corpus producer
reaches any fire/impact site, so the engine seam stays NONE
(D62, docs-only). CLOSED this unit: the sibling resolver's
dispatch (terrain-structure hp − no robot/floor path), its three
bank writes + the new 0x4cccf8/0x20 array + 1-based id
convention, the terrain-height probe semantics (+ the [0x4edd60]
height-bank array), both disburser arg maps, and the projectile
type word. Re-opens cleanly at: the 0x4cccf8 array PRODUCER
(mission-load stager), the FUN_00410823 anim-machine internals,
and FUN_00419aff's per-weapon table layout.

## 7j.15 Amendment 2026-08-21 (worker efff097c, the weapon-fire
family THIRD HOP: FUN_00419aff + the 0x4cccf8 producer census)

Method: `ExwWeaponFire3.java` + `ExwWeaponFire4.java`
(-process BEDLAM.EXW -noanalysis), dumps =
`ghidra-project/exw-weaponfire3.txt` / `exw-weaponfire4.txt`
(full decompile + listing + all-28-caller 0x40 arg windows; xref
census with ±0x18 context; producer full decode + caller).
All facts below [verified] against those dumps unless tagged.

1. **FUN_00419aff = the WEAPON/PROJECTILE DAMAGE TABLE — a pure
   id switch, no table walk.** EAX = weapon/projectile id → EAX
   = damage. EDX is pushed/popped untouched (plain register
   helper); the `push 1` at the fire sites is consumed
   downstream as FUN_0041a894's score flag — the 7j.13
   "(weapon_id, field)" second-arg hypothesis is CLOSED as an
   ERRATUM: beside the id, the ONLY selector is the global
   DIFFICULTY dword 0x46cbf8 (d, values 0..2):
   | id | damage |
   |---|---|
   | 2 / 3 / 4 | 20 / 30 / 40 |
   | 5 | 75 |
   | 0xc | 5000 |
   | 0xd | 312 |
   | 0x1a | 75 |
   | 0x24 | 400 |
   | 0x29 | 250 |
   | 0x65 | (d+1)·50, d=2 → 200 |
   | 0x66 | (d+1)·300, d=2 → 1200 |
   | 0x67 / 0x68 | (d+1)·75, d=2 → 300 |
   | all other ids | 1 |
   Coherence: the 7j.14 cosmetic anim kinds ({0xE,0xF,0x13,0x17,
   0x1A,0x1F}→K0xC, 9..0xB clear-only) all land in damage-1
   buckets; the four real robot weapons pinned 7j.13
   (5/0x1a/0x24/0x29) hit 75/75/400/250; only the four
   projectile types 0x65..0x68 are difficulty-scaled (enemy
   fire; d=2 overrides the linear (d+1)·k with a flat larger
   constant via the branchless `ADD` idiom at 0x419bf1).
2. **DAT_0046cbf8 = the DIFFICULTY dword (0..2)** [verified]:
   cycled at NameEntryScreen 0x43ab7e `(d+1)%3` (toggle →
   FUN_00445b5c redraw, itself a reader); persisted in the
   campaign save (0x43c3a6: word read right after [0x46ae70]);
   money context 0x43aaa3 `IMUL d,0x1f4` (=500·d vs 0xfa0=4000
   base); GameMain 0x41c568: zone [0x4edd8c]==7 temporarily
   forces d=2 around FUN_0044771c then restores EBP (saved
   value); campaign-start write 0x41c14a. 44 refs total; heavy
   readers: FUN_00412f34 ×13, the FUN_00403938 sidebar ×10,
   FUN_00416458 ×5.
3. **Caller census (28 sites, verified)**: FUN_00410823 ×16
   (the fire controller — 8 beyond the FUN_0041a894-adjacent
   sites, incl. non-impact stat reads), FUN_004190bc ×6 (stat
   reads off the 0x4cff98-family record bank — the bank
   FUN_00416458 clears at load; a second stat consumer, likely
   a panel/preview), FUN_00412010 ×4 (projectile tick: the
   0x65/0x66 impacts + expiry paths), FUN_004197d4 ×1 (the
   robot-hit walker re-reads 0x65), FUN_00418fca ×1 (weapon id
   word [0x4c71f2+2·i] out of the anim-rec family).
4. **The 0x4cccf8 PRODUCER = FUN_004170a6, the ".TRT"
   mission-file section loader** [verified]; sole caller
   FUN_00416458 at 0x416487 — the mission-load dispatcher
   (clears 0x4cff98/0xac44 B + 0x4dabdc/0xf00 B, calls the TRT
   loader, then opens ".NME"; extension tags .NME/.TRT/.POS/.BDG
   @0x457a57..0x457a6d — §7j.29 erratum: the earlier ".MOFO" in
   this list was the dead tail of the fatal string 0x457a3c, not
   a tag). FUN_004170a6 itself:
   (a) clears 8000 B at 0x4cccf8 via FUN_00402965 (ECX=0x1f40)
   = the FULL 250-record bank (capacity pin; 0x4cccf8 is .bss,
   no file backing); (b) FUN_0041dbbed(0x4dca0c, ".TRT") stages
   the section bytes, FUN_0041cd90 inits the reader;
   (c) count = FUN_0041cccb(&0x46ccd4, 2); (d) per record
   i < count: three FUN_0041cccb(ptr, 4) reads → x, y, z staged
   at +0x10/+0x14/+0x18 (stager frame, see (e)); hp@+0xC =
   250 + (250·[0x46ae8c])/27 integer-div ([0x46ae8c] = linear
   mission 1..26 per GAMETHREAD → hp 259 at m1 … 490 at m26);
   +0x00 = 1, +0x04 = 1 (the active dword), +0x08 = 0 (unknown
   scratch dword — no producer found this unit).
   (e) **base refinement of 7j.14**: the stager frame base is
   0x4cccfc, i.e. one dword BELOW 7j.14's resolver frame
   (0x4cccf8, active@+0). All 7j.14 offsets (hp@+0x10,
   x@+0x14, y@+0x18, z@+0x1C) remain correct in its frame; the
   extra +0x00=1 dword sits at −4 there; the external 1-based
   idiom (0x4cccd8+id·0x20 = active of rec id−1) is unchanged
   (0x4cccd8 = the id-0 slot one record below the array).
   (f) the stager ALSO stamps tile byte 0x66 at
   byte[[0x4edd58] + x + y·[0x4eddec] + z·[0x4eddf4]] — the 3D
   per-level tile bank (w=[0x4eddec], plane stride
   [0x4eddf4]); 0x66 = the terrain-structure tile, sibling of
   the 0x62 trap tile — and word 1 at
   word[[0x4ede20] + 2·(x + y·w + z·w·h)] (a second 3D word
   bank, pointer slot @0x4ede20). Both banks' consumers stay
   open [census]. Consumers of the record array found by the
   count-xref census: FUN_0041bc1c (damage, 7j.14),
   FUN_00417264 (scans active; reads the +0x08 scratch dword
   frame-off and y), FUN_00419943 (RandA/IDIV scatter vs map
   globals [0x4edde4]/[0x4edde8] — a placement search),
   FUN_0041ee20 (scans active).
5. **FORMATS-MISSION §14 TRT anchored**: the third u32 per
   record is the z LEVEL (census values 0..6 = levels;
   per-zone bands: ZONEA all 1, ZONEB all 2), NOT a type enum
   — records are destructible terrain-structure placements
   (the FUN_0041bc1c hp family, hp scaled by linear mission).
   The "turrets?" reading retires as primary; turret-vs-static
   behaviour moves to the open consumers above.
6. Corpus-path verdict: unchanged — the weapon family stays
   unwired (no corpus producer reaches a fire site; D63,
   docs-only). CLOSED this unit: FUN_00419aff's full layout
   (the queue item), the 0x46cbf8 difficulty identity +
   writers, the producer census (FUN_004170a6/.TRT/the
   FUN_00416458 chain), the 250-record capacity, and the TRT
   third-field anchoring. Re-opens cleanly at: the FUN_00410823
   anim-machine internals, the FUN_00417264/FUN_00419943/
   FUN_0041ee20 structure consumers, the two 3D banks' other
   consumers, and the FUN_004190bc 0x4cff98 record family.

## 7j.16 Amendment 2026-08-21 (worker 16f43187, the .TRT
CONSUMER hop — the three scanners + the two 3D banks + the
+0x08 scratch producer)

Method: `XRefList`/`DecompList`/new `DumpAscii`/`DumpRange`
(-process BEDLAM.EXW -noanalysis), dumps =
`ghidra-project/exw-trtconsumers.txt` / `exw-trtcallees.txt` /
`exw-trtcallees2.txt` / `exw-trtxref{,2,3}.txt` /
`exw-objtypewords.txt` / `exw-trtstrings.txt` / `exw-pickret.txt`
(+ corpus size/byte checks on game-data). All facts [verified]
against those dumps unless tagged.

1. **FUN_00417264 = the TRT structure ANIMATION/FIRE state
   machine — "turrets?" RESTORED as the primary reading.**
   Sole caller MissionShell @0x44807b (the mission tick loop —
   it runs every frame). Canonical record frame (active base
   0x4cccf8 + i·0x20, 7j.14's): `{active@+0, state@+4,
   anim_frame@+8, fire_ctr@+0xC, hp@+0x10, x@+0x14, y@+0x18,
   z@+0x1C}` — the "+0x08 scratch dword" of 7j.15 is the
   ANIMATION FRAME and its runtime producer is THIS function
   (no file producer exists; the loader just zeroes it —
   closes 7j.15 item (d)/D63's open point). Machine:
   - state 1 = idle. Active records probe the nearest robot
     via FUN_00417c00(px, py, &dist) (octile, below); dist
     < 0x81 → state 2. Inactive (destroyed) records skip to
     the death branch: ≠1/≠4 → (frame==7 ? 4 : 3).
   - state 2 = alert: frame 0→7, each step writing TOT mirror
     word = frame+1 (FUN_00417210); at frame 7 → state 6.
   - states 5/6/7/8 = AIM south/north/west/east (octant by
     dominant |dx|/|dy| toward the probed robot, recomputed
     live): frames ramp to a per-direction top (0xB/7/9/0xD),
     at the top call FUN_00417698 = FIRE, then a 4-step
     muzzle flash (mirror words 0x17..0x1A s / 0xF..0x12 n /
     0x13..0x16 w / 0x1B..0x1E e — word = ctr+base, ctr
     1..4), then ramp back down. Lost target → ramp down.
   - state 3 = death anim (frame 8..0xB ramps), state 4 =
     settle-to-0 → state 1 (destroyed structures idle out).
   - FUN_00417210(idx, n): TOT mirror word @0x4796bc +
     2z + (rowoff[y] + x)·0x1E = n+1 (rowoff = the
     0x4ea900 pitch table). FUN_00417652(idx) = frame remap
     0xF→7, 6→0xE (skip frames).
2. **FUN_00417698 = the FIRE routine** [verified]: per aim
   state, scans the robot bank 0x4c69e4/0xA8 (count
   DAT_0046ccbc, active@0x4c6a60): target iff |lateral
   offset| < 0x28 px in the aimed lane AND robot is beyond
   the structure in that direction AND |(z_struct −
   (z_robot>>8 + 0x1F))>>5| < 2 (≈2 levels). No target →
   fire_ctr(+0xC)=0, restore frame word. Target: fire_ctr
   0→1 (arm), and on odd ctr → FUN_0041286f (free-projectile
   slot: first rec @0x4cc654+i·0x22 with type word 0, ~50
   slots, ret −1 — confirms 7j.14's free convention) then
   stages **projectile type 0x66**: x = tile·0x2000 + 0xF00,
   y likewise, z = tile<<0xD, +0x16 dword = 0x14, vx/vy words
   = unit direction (S/N/W/E per the 0x00/0x80/0x40/0xC0
   lane flag). Projectile 0x66 = damage (d+1)·300 (7j.15) —
   the heaviest enemy projectile: structures ARE shooting
   sentry turrets. They never move (x/y/z are never written
   after load).
3. **The two 3D banks are the mission map FILE VOLUMES**
   [verified, FUN_0041dc5a = the map loader, sole caller
   MissionShell @0x447b3a]:
   - `[0x4ede20]` ← **".TOT"** (tag @0x4587d9; path builder
     FUN_0041dbed(mission-dir 0x4dca0c, tag) → static
     0x4dca4c; opener FUN_0041cd90 → global handle
     0x4eba20): word-per-voxel volume, header **u16 W,
     u16 H** (4 B, verified: ZONEA/M1.TOT `19 00 4b 00` =
     25×75; 30004 = 4 + 2·25·75·8) then 8 z-planes of W·H
     u16. W→_DAT_004eddec, H→_DAT_004eddf0, plane pitch
     W·H→_DAT_004eddf4; ptr skips +4.
   - `[0x4edd58]` ← **".DAT"**: byte-per-voxel volume, same
     4-byte {W,H} header skipped (+4), 8 planes of W·H u8
     (ZONEA 15004 = 4 + 25·75·8). Post-load sanitize: any
     tile byte > 0x7F → 0. ArenaAlloc sizes (FUN_0041d954):
     word 0x27104, byte 0x13884 → max map 100×100×8 (ZONEB
     TOT 160004 = exactly that).
   - Same loader also fills: `[0x4edd60]` ← ".CGR" (the 3D
     height banks), `[0x4ede1c]` ← ".BIN" (header word →
     DAT_0046cdb8), `[0x4edd9c]` ← ".MIN", 0x45cdda ←
     ".LNG"/".LNK" variant by _DAT_004eba1c, then opens
     ".PAD" and parses up to 999 records {x@+2, y@+4,
     z@+6} into the 0x4e44f8 8-byte slots (active word@+0
     set 1; x==0xFFFF ends) stamping **0xFF into the DAT
     volume at (x,y,z)** — the §7c.5/FORMATS §10 pad
     materializer, now anchored from the loader side.
   - **FUN_00440a2d (caller FUN_00440dc2) = the TOT-volume →
     TOT-mirror MATERIALIZER** — the key consumer of the
     word bank: for a 7×7 tile block × 8 z: word≠0 ∧ DAT
     byte==0 → mirror word@0x4796bc(+row pitch 0x1E) =
     word, seen@0x4796cc = 1. This is how the TRT word-1
     stamp becomes the visible structure sprite (frame 1);
     the animator then drives the mirror directly. The
     [0x4ede20] census: producer FUN_004170a6, loader
     FUN_0041dc5a, restore FUN_0044661b (re-loads .TOT/
     .BIN/.DAT, tags @0x459795 — the "EDITOR\ZONE" restore
     path), materializer FUN_00440a2d, boot census
     FUN_00407e11. The [0x4edd58] byte-volume census: ~30
     readers (renderer FUN_00403938 ×many, TOT-writer
     family FUN_00423xxx/FUN_00422xxx, probes FUN_0041eaa1/
     FUN_0041e2xx, resolvers FUN_0041a894/FUN_0041bc1c/
     FUN_0041bd78, splash FUN_00424355, trap pair
     FUN_0040fe93/FUN_0040ff92) + the 0x62/0x66 tile
     semantics pinned earlier.
4. **FUN_00419943 = the map-click PICK** [verified; sole
   caller FUN_00410644 @0x41068e, itself MissionShell
   @0x448021]: (a) hit-tests the screen RECT list 0x4787c4
   stride 0x20 {center-x@+8, center-y@+0xC, w@+0x14} (count
   [0x46ccd8]; the list is WRITTEN by the renderer
   FUN_00403938 @0x403c93 — the on-screen hot-rect list)
   with octile cost FUN_0041ebf8, early-out < 4, best wins;
   (b) else screen→iso IDIV ((p−0xF0)·[0x4ede54])/0x1E0 vs
   camera [0x4edde4]/[0x4edde8], then a TRT-array scan in
   iso space (±0x10..0x30 box). Return: 0 = open ground,
   k+1 = rect k, **(idx+1)|0x2000 = TRT structure**
   (0x419aed `LEA EAX,[ESI+1]; OR AH,0x20`); tail-call
   FUN_00418a9f = an EMPTY stub (patch vestige).
   FUN_00410644 = the click ORDER dispatcher: writes the
   order target {x,y,z} = 0x4dd484/0x4dd488/0x4dd48c
   (ground iso / rect corner+type / structure tile-center —
   the 0x2000 branch reads rec x/y/z at 0x4cccec+0x20·idx =
   the canonical +0x14/+0x18/+0x1C), consumed by the robot
   behaviour family (FUN_00409138 ×6, FUN_0040a56f/
   0xa7a1/0xace8/0xaf98/0xb615/0xa9ff — see leads).
5. **FUN_0041ee20 = the SCANNER overlay drawer** [verified;
   sole caller FUN_0041ec81 @0x41edcd, itself MissionShell
   @0x48142]: FUN_0041ec81 = the corner scanner widget
   (cursor box x 0x1EE..0x272 × y 0xC3..0x147 + scroll
   flags → active flag 0x46ccb8; grow animation
   [0x4edd68] 0→0x40 step 4; asset "GAMEGFX\SCANNER.BIN",
   tag string right after the file tags). When fully open
   it calls FUN_0041ee20(cx, cy) with the SELECTED ROBOT
   position ([0x46cbd4]+[0x46cbdc] robot) and draws marker
   icons via FUN_00402572 (128×128 transparent blitter →
   the [0x4eddb8] overlay, clip 0x80) around anything
   within 0x80: **8 = TRT structure** (first loop — the
   task's "active scan"), 4 = 0x4cffbc bank, 1/2 =
   robots (selected-group vs rest, gate [0x46cbd4]/
   [0x46cbd8]), 5/6 = linked object pairs (blink), 7/0xD
   = tile markers (strength 0x465daa ≠ 0 vs claim byte
   0x46af58), 9/0xA = objects (alive flags), 0xB = the
   0x4dcdb8 arrival records, 0xC = the 0x4e44f8 pads.
6. **FUN_00417c00 = nearest-active-robot octile probe**
   (robot bank 0x4c69e4/0xA8, returns idx + dist out) —
   callers besides the turret machine: FUN_00412a98,
   FUN_00412f34 ×4, FUN_00417e2f (both [0x46cbf8] readers
   from the 7j.15 backlog = the robot targeting family).
   **FUN_0041ebf8 = octile distance** max(|dx|,|dy|) +
   min/2 — 51 sites (FUN_00412f34 ×15 pathfinder,
   FUN_004190bc ×8, FUN_0040de9c debris ×3, FUN_00440e45,
   FUN_0041a028, FUN_0040b9f6 ×3, …).
7. **7j.13-erratum correction (the object-type table field
   map — supersedes the uncommitted 22c1c14b draft's +0/+2/
   +4 shift)**: the draft's own anchors disprove it — every
   dword read is SAR'd 16 (0x41a857 `MOV EDX,[EBX+
   0x4dedf2]; SAR EDX,0x10` = word@B+2, NOT word@B+0;
   likewise 0x41aa02 → word@B+4; 0x41aaf9 `[EDX+0x4dedf6]`
   SAR → word@B+6; bank size 0x41a6fc..0x41a71a = word@+2 ·
   word@+4 · word@+6). So the ORIGINAL 7j.13 W/H/D offsets
   stand: **W=X-extent word@B+2, H=Y-extent word@B+4,
   D=z-depth word@B+6, word@B+0 unconsumed [open], hp
   dword@B+8** (0x41a7d3, plain dword — no shift). The
   draft's CONFIRMED contributions: count word@+0x12, the
   5×8B destroy-effect entries @+0x16..+0x3E (selector word
   @+0x16+8k — `MOV AX,[EAX+0x4dee08]` @0x41ac58, 9-case
   jump table @0x41a870), the 4 W·H·D-word template-bank
   ptrs @+0x3E/+0x42/+0x46/+0x4A, and the exact 0x4E
   closure. Corroboration: FUN_0041ee20's object icon
   centers use word@+2 for x and word@+4 for y.
8. Corpus-path verdict: unchanged (D64, docs-only — no
   engine write sites reached; the turret animator/fire
   stays unwired like the rest of the weapon family).
   CLOSED this unit: the task's three scanners, both 3D
   banks (identity + consumers), the +0x08 producer, the
   "turrets?" question (YES: animate + shoot, never move).
   New leads for the queue: FUN_00412a98/FUN_00412f34/
   FUN_00417e2f (the robot targeting/aim family — probe +
   0x46cbf8 readers), the order-target 0x4dd484 robot
   behaviour family, FUN_00440dc2 (the materializer's
   caller — scroll restamp?), the 0x4787c4/0x47879c rect
   record (corner@+0/+4 vs center@+8/+0xC, z@+0x10,
   w@+0x14, type@+0x1C — [hypothesis] from the two views),
   FUN_0044661b's EDITOR\ZONE restore context, and the
   [0x4ede24] 7×7 screen-address table (FUN_00440a2d head).

## 7j.17 Amendment 2026-08-21 (worker 3f4f7c10, the ROBOT
TARGETING/AIM family — adopted from three provider-outage-killed
runs 2026-08-21 19:15/19:34/19:40, logs
agent-31790e94/agent-08f6fa30/agent-0ce3a285)

Method: the three dead runs had already produced the Ghidra
dumps (`ghidra-project/exw-robottarget.txt` = the four main
decompiles, `-xrefs.txt`/`exw-robottarget2.txt` = xref
censuses, `exw-robottarget3.txt` = 11 helper decompiles,
`exw-robottarget4.txt` = 10 caller decompiles,
`-asm.txt` = the 0x413f40..0x413fc0 SAR check; produced
19:05..19:29, gitignored). This unit re-verified every claim
against those dumps + objdump and wrote it up — NO new Ghidra
run. All facts [verified] against the dumps unless tagged.

1. **FUN_00412f34 (9546 B) = the 0x4cff98 CRITTER-ACTOR
   controller** — sole caller MissionShell @0x447fe1 (every
   frame). Count DAT_0046cc2c (written by the mission-load
   dispatcher FUN_00416458 @0x41646d; also read by the sidebar
   FUN_00403938, the scanner FUN_0041ee20 (icon 4), the debris
   physics FUN_0040de9c and FUN_004244a1). Record frame,
   stride 0x7E, Q13 coords (asm: SAR 0xD @0x413f8c/0x413fa4 —
   the decompile's `>>5` tails are artifacts):
   {state w@+0, substeps w@+2 (dword@+0>>16), timerC w@+6,
   MODE w@+0xC (dword@+0xA>>16), anim-ctr w@+0xE,
   attack-target xyz d@+0x2A/+0x2E/+0x32, x/y/z d@+0x36/
   +0x3A/+0x3E, home xyz d@+0x42/+0x46/+0x4A, z-restore
   d@+0x4E, seek-dir d@+0x50, countdown w@+0x56 (dword@+0x54
   >>16), dir w@+0x58, frame w@+0x5A, 8 corner-z words
   +0x60..+0x6E, facing d@+0x70>>16, y-bob d@+0x74>>16,
   target-robot w@+0x7A (dword@+0x78>>16), fuse w@+0x7C,
   active w@+0x24}. Machine:
   - state 1 WANDER: per substep, dir-steppers ±6 Q13 with
     wall probe FUN_0041f8f9 + map bounds
     (x < _DAT_004eddec·0x20 px etc.); blocked/new-dir →
     pause 8..27; direction 25% random else toward nearest
     robot (FUN_00417af2); z clamp FUN_00418250.
   - state 2 SINE-WALK SHOOTER: pos += sin/cos heading
     (FUN_0041eb65/77) ·0x14; 1/128 SFX _DAT_004edffc; every
     4th substep picks a random ALIVE robot (FUN_0041ec1c)
     and if octile>>8 < (2−d)·−0x40+300 = 172/236/300 px
     fires projectile 0x65 into the 50×0x22 bank 0x4cc654
     (FUN_0041286f free slot) aimed at the robot ±0x1F00
     scatter.
   - state 3 CHASE-COMBAT: nearest-robot probe FUN_00417c00;
     home-leash octile(home−pos) > 400 → mode 10 return;
     dist > 200 ∧ mode 2 → mode 10; dist < 200 ∧ leash ok ∧
     mode ∉{2,3} → mode 3 approach; dist < 100 → mode 2
     attack. Mode 3: re-aim every 9 frames (atan2
     FUN_00425498 snapped to 32-sector) + pathfinder step
     FUN_0041571c(idx, heading) gated by the walk-pattern
     dword table [0x454b48 + ctr·4]. Mode 2: re-aim + fire
     0x67 with full 3D velocity (dx·0x800/dist etc., z via a
     second octile); >4 shots → reset. Mode 10: aim home +
     pathfinder.
   - states 4/5/6 MIXED-AI (per-MODE dispatch): mode 0xB
     DORMANT — respawn countdown vs the DIFFICULTY-indexed
     delay table DAT_00454edc[d]; wake → rand dir, pause 6,
     timer 200/0x96, SFX _DAT_004edfe0. Mode 7 DYING: 0x28
     frames → mode 0xB (+0x5dc timer in state 3). Mode 6
     BALLISTIC: drift ±0xF, ground probe FUN_0041e411;
     landing → 8× debris KIND 6 (FUN_00420608) + 5×
     FUN_00424355 chunks + splash FUN_0041a14f(x,y,(z+0x15)·
     0x100, 0x18) — a reachable producer for the 7j.10/
     7j.1 effect-row bank. Mode 9 SEEK: dominant-axis
     steppers FUN_00417f2c/17fe8/180c0/1813d (y−1/x+1/y+1/
     x−1) each → FUN_00415490. Mode 2 RANGE-ATTACK
     (dist<500): FUN_0040db9e(robot, 2, heading<<6, 1, −1)
     [identity open]. Mode 5: brief rise then mode 3.
   - state 7 CLOSE-COMBAT: steer (atan2 + FUN_00412a19
     clamp) + sin/cos move; engage leash
     (d+1)·0x40+600 = 640/704/768; point-blank
     (dist<0x50) projectile 0x69 — NEW type, absent from the
     7j.15 damage table (→ "else 1") — fired at fire rate
     every 32/16/8 frames for d=0/1/2, {type 0x69, z=6,
     +0x1A=0, +0x1E=0x18}; attack-break odds d=0: 1/8,
     d=1: 1/16, d=2: never (RandA gates @0x41353e/56/6e).
   - FLEE scan (states 5/6, order active [0x4dc6bc]≠0,
     mode ∉{0xb,7,6,5,9,2}): scan the 400×0x36 projectile
     bank 0x4c71f4 for types 9/0xA/0xB within 300 px → steer
     away (heading ±0x80), pause 200, mode 10.
   - Epilogue per active critter: presence mark — byte 1 at
     [[0x4ea900 + (y>>13)·4] + [0x46af4c] + (x>>13)]
     (asm-verified); z-settle FUN_004182c3; moved? →
     FUN_0040ff92 (the 7j.13 tile-0x62 trap re-probe).
   - DIFFICULTY: 12 direct [0x46cbf8] sites in
     0x412f34..0x41547E [objdump-verified; the dead-run's
     ×13 double-counted a shared load] — respawn delay,
     0x65 range, engage leash, fire rate, break odds. AMENDS
     the 7j.15 ledger row: the dial does NOT scale only
     projectile damage — it drives critter behavior too.
2. **FUN_00417e2f = the SUICIDE-BOMB trigger** (the case-1
   gate): nearest-robot FUN_00417c00 dist < 0x30 px →
   deactivate + 8× debris KIND 1 (FUN_00420608) + 8×
   FUN_00424355 chunk rings. Returns FUN_004180b9's (NOP)
   leaked EAX — the case-1 `== 0` guard is EAX-leak
   tolerant [hypothesis: asm tail sets it].
3. **FUN_00412a98 = the 0x4dabdc POI/PERSONNEL controller**
   — count DAT_0046cbf0 (FUN_00416458 write @0x416f6e).
   Record, stride 0x1E: {active w@+0, state w@+4, heading
   w@+8, timer w@+0xA, x/y/z d@+0xE/+0x12/+0x16}. Ground
   clamp FUN_0041e411 every tick. States: 1 idle (timer 0xB,
   then 1/16 → 2 startle-aim or 3 wander, timer 10..25);
   2 settle (7 → 1); 3 walk-out (FUN_00415b6c wall-walker);
   4 FLEE-TO-EXIT: nearest of the FIVE 0x1C-stride exit/
   threat slots @0x4e662c {active d@+0, kind d@+4 (== 2 =
   exit — §7j.19 reread: the field is the craft PHASE, 2 =
   landed/OPEN), x/y d@+8/+0xC, d@+0x18} via FUN_00417c64 (octile);
   trigger: exit within 0x180 ∧ 1/16 (state ∉ 4..7);
   arrival < 0x10 → state 5; 5 ESCAPE: active=0, rescue
   progress _DAT_004eba0c++, quota _DAT_004eba10 = 0x32,
   clear the slot's +0x18, SFX _DAT_004edfa8 (screen-wide
   −1,−1, mode 3), then FUN_00448b80(5000); 6/7 panic pair.
   The exit-bank producer = FUN_0041fa51 @0x41fabb [open
   head — MRK/NME candidate]. Reset: MissionShell
   0x44792d/33 zeroes quota/progress at mission start.
4. **FUN_00409138 = the COMMAND-RECORD consumer** (7j.13's
   "robot behaviour pass" — now pinned): records @0x4dd4a0
   stride 0x80, count DAT_0046cbe0; sole caller MissionShell
   @0x448030, immediately after FUN_00410644 (click order)
   + FUN_00449c94 (the record BUILDER — it reads the current
   order target 0x4dd484 to stage records). Record: {robot-id
   short@+1 (id = rec·DAT_0046cbd8 + v — per-player id
   bases DAT_0046cbd4), spot short@+3, FLAGS byte@+5:
   bit0 SELECT → order words → DAT_0046cc30[id]/DAT_0046cc60
   [id] + auto-arm (state@+0xC := 1, target@+0x74 :=
   1000000) when state ∉ 2..5; bit1 ORDER →
   _DAT_004dd484/88/8C := xyz words@+7/+9/+0xB,
   _DAT_004dc6bc := 1, clears 0x4eb940..50; bit4 →
   FUN_00449c82/FUN_0041c9f0}. WEAPON dispatch per robot: 7
   slots {id w@+0x36+8k, ammo w@+2, cooldown w@+6}, enable
   mask w@+0x6E, alive d@+0x7C; weapon id−2 ∈ 0..0x26 =
   the 39-case switch @0x40a08e:
   - w 2/3/4 → FUN_0040b615 orders 3/2/1; w 6/7/8 →
     FUN_0040af98 orders 0/1/2; w 0x18/0x19 → FUN_0040a56f
     1/2; w 0x21/0x22/0x23 → FUN_0040ace8 3/6/9; w 0x25/
     0x26/0x27/0x28 → FUN_0040a7a1 1/2/4/6; w 0xE →
     FUN_0040a9ff.
   - PROJECTILE spawners into the 400×0x36 bank 0x4c71f4
     (FUN_00412848 free slot; record {type w@+0, owner w@+2,
     ttl d@+0xA, xyz d@+0x12/+0x16/+0x1A, vxyz d@+0x1E/
     +0x22/+0x26, class d@+0x2A (0/4), arc d@+0x2E}):
     w 9/0xA/0xB → 1× type = id; w 0x10/0x11/0x12 → 2/4/6×
     type 0xF (vel>>2, scatter ±0x20 around the ORDER
     TARGET, ttl RandA&0xF+1, arc 0x900−RandA&0x2FF, class
     4); w 0x14/0x15/0x16 → type 0x13 (vel>>1); w 0x1B/
     0x1C → 4/6× 0x1A (3D vel incl. order z 0x4dd48C, ttl
     0x33−RandA&0xF..0x42, arc 0xB00−…); w 0x1D/0x1E → 4/
     6× 0x1F (ttl 0x32+RandA&0xF..0x41); w 0x20 → 1× 0x24
     straight missile (ttl 0, cd 5, SFX _DAT_004edfac,
     angle pair FUN_0041eb7d/ebc1 → arc). All aimed at the
     ORDER TARGET; ammo−1, 0 → clear enable bit; cooldown
     8 (0x24: 5); SFX pair _DAT_004edf94/_DAT_004edfe4
     one-shot-gated by _DAT_004eb950.
   - All weapons empty after a firing pass → auto-rearm
     first slot with id≠0 ∧ ammo≠0 + FUN_004239ef(0x1C..
     0x21, playerIdx) — the weapon-empty/auto-switch
     message family, per player 0/1/2 (id bases
     DAT_0046cbd4/+/+2, guard 2/3 players DAT_0046cbd8).
   - Idle path ([0x4dc6bc]==0, every 4th frame): player-type
     robot present → DAT_0046ccec = 2 (sidebar redraw) +
     idle AI ticks FUN_0040af98(10)/FUN_0040ace8(9)/
     FUN_0040a56f(2)/FUN_0040a7a1(6). Epilogue: recharge
     tick — every robot's enabled weapons with nonzero
     cooldown decrement (7-slot loop over
     DAT_0046ccbc robots).
   - The record bank = the per-frame COMMAND RING shared by
     local input and MP networking: NameEntryScreen writes
     the count; readers FUN_00449c94 ×7, FUN_0040cca0 ×3,
     the MP lobby FUN_00448ef1 ×3, FUN_00440e45 (THE SHOP),
     FUN_0043d00b ×3, FUN_0041ca2e, FUN_00445b5c,
     FUN_0044a38a ×2.
5. **FUN_00448b80 = the MISSION-OBJECTIVE RESOLVER** (arg =
   event type: 5000 = rescue, else = destroyed object-type
   id; gated [0x4edb88]≠2 (no-MP) ∧ zone [0x4edd8c]≠7):
   6 slots × 0x20 @0x4eaaee {remaining w@+2 (dword@+0>>16),
   TYPE w@+6 (dword@+4>>16), status w@+0xC (0xFFFF = done),
   quota w@+0x1E}. Type-5000: progress [0x4eba0c] ≥ quota →
   done. Else: kill-stat bank [0x46cbf4]+type·0x14 {x0, y0,
   …, +0xC&0x3FFF = id-table index} → wipe the 0x1E-stride
   scenery-mirror rows 0x4796d7/d8 over the object's W×H
   (id-table 0x4dedf2 words) → done. Slot completes →
   FUN_004239ef(0x26 first slot / 0x27 others, 3) +
   DAT_0046cd00 := 1/2 + [0x46ccfc] := 0x20 +
   [0x4eb8b8+slot·4] := 1; non-5000 partial → msg 0x34 +
   state 4; ALL 6 done → msgs 0x28 + 0x29, state 3. Zone-7
   special: counter [0x46cce0]−− per destroyed type
   0x44..0x47 (mirror wipe) → 0 → msgs 0x28/0x29, state 3,
   [0x46ccc4] := 0x32. DAT_0046cd00 = the objective phase
   state {1 first, 2 done, 3 all-complete, 4 partial}.
6. **Helper identities** [all verified this unit]:
   FUN_00417c64(i,&d) = nearest-of-5 exit slots (above);
   FUN_00417ba1 = a second nearest-robot probe (Q13 in);
   FUN_00417af2/FUN_004181bd = dominant-axis direction
   toward the nearest robot (0 = −y, 1 = +x, 2 = +y,
   3 = −x — matches the mode-9 steppers);
   FUN_0041f8f9 = the 8-sample walk probe (offset tables
   0x4543e4/0x454404 step 4; same-level FUN_0041e231 + DAT
   height-diff ≤ 3 via FUN_0041eb4c);
   FUN_004186fc = standing-on-scenery check (mirror byte
   0x4796d5[row·0x1E] ≠ 0 → FUN_00418250 reposition);
   FUN_004182c3 = the 8-corner z-settle (x/y snapped to
   tile centers +0x13/+0x0B, z → FUN_0041e411 floor,
   probe-z words +0x60..+0x6E);
   FUN_0041e411 = the FLOOR probe (FUN_0041eaa1's sibling):
   z level z>>5 clamp 0..7, level try +1 then −2, per-TYPE
   entries {[0x4edd60+2+(type−1)·4] = dword offset} with an
   in-tile 0x20×0x20 HEIGHT BYTE map @(x&31)+(y&31)·32 at
   bank+off+6; floor = level·0x20 + byte; byte 0x1F =
   top-of-stack (peek level+1). This anchors the open
   "[0x4edd60] height-bank family" backlog item — note
   7j.16 already pins [0x4edd60] = the .CGR loader target,
   so the CGR bank doubles as the per-type height maps;
   FUN_004180b9 = NOP; FUN_0041642d(idx,n) = anim counter
   w@+0xE wrap at n; FUN_0041286f = free slot, 50×0x22 bank
   0x4cc654; FUN_00412848 = free slot, 400×0x36 bank
   0x4c71f4; FUN_0041a14f = effect-row spawner for the
   0x20-stride rows @0x4cec38 (the 7j.1 boot-cleared bank —
   now with a reachable producer via critter death);
   FUN_00415b6c = the POI wall-walker (8-way FUN_0040cc5e
   probes, Q13); FUN_00415ff2 = the critter step mover
   (same probe family).
7. **Census folds (from exw-robottarget2/-xrefs):**
   - 0x4dd484/88/8C order target — writers FUN_00410644
     (×3, click) + FUN_00409138 (bit1 records); readers
     FUN_00409138 ×6, FUN_0040af98 ×3, FUN_0040a56f/
     0xa7a1/0xace8/0xb615/0xa9ff ×2 each, FUN_00449c94.
     CLOSES the residual "0x4dd484 reader census" left by
     7j.16.
   - 0x46cbe0 command count — see item 4 (MP-family census).
   - 0x46cc2c/0x46cbf0 — producers FUN_00416458 (load);
     drawing consumers sidebar FUN_00403938 + scanner
     FUN_0041ee20; physics FUN_0040de9c reads BOTH (debris
     vs critter/POI collision family).
   - The 47-site k1..k20 census (7j.11 item 6) and the
     28-site damage-table census (7j.15 item 3) were
     re-read and stand unchanged; this unit adds their
     first non-weapon corpus-reachable PRODUCERS: critter
     death → k1 (FUN_00417e2f) and k6 (state-4/5/6 mode-6
     landing) + FUN_00424355 + FUN_0041a14f(0x18).
8. Corpus-path verdict: docs-only (D65) — no engine change;
   the critter/POI/command/objective families stay unwired
   like the rest of the weapon family (weapons never fire
   in the gates; critters would tick but their LOADER
   section inside FUN_00416458 — which mission file feeds
   0x4cff98/0x4dabdc/0x4e662c — is the next bounded head
   [.NME/.POS family candidate, cf. FORMATS §9/§12]).

## 7j.18 Amendment 2026-08-21 (worker a840f0af, the
critter/POI/exit LOADER hop — .NME grammar CLOSED)

Method: one `-process BEDLAM.EXW -noanalysis` run, dumps =
`ghidra-project/exw-critterpoi-loader.txt` (full decompile of
FUN_00416458/FUN_0041fa51/FUN_00449c94/FUN_0040db9e) +
`exw-critterpoi-xrefs.txt` / `-xrefs2.txt` (censuses) +
`exw-critterpoi-asm.txt` (asm of the two count writes + the
exit-slot producer) + `exw-critterpoi-str.txt` (section string
bytes) + a corpus exact-consumption check
(/tmp/opencode/nme_check.py, all 37 NME files). All facts
[verified] against those artifacts unless tagged.

1. **FUN_00416458 = the .NMI-.NME critter+personnel LOADER.**
   Prologue (@0x416461..0x416496, asm-verified): clears
   0xAC44 B @0x4cff98 + 0xF00 B @0x4dabdc (FUN_00402965 ×2),
   resets the critter count `DAT_0046cc2c = 0` @0x41646d (the
   queue's count write = this RESET; the counter then
   increments per spawned critter), calls the TRT loader
   FUN_004170a6, then stages **".NME"** (string @0x457a57,
   bytes verified `2e 4e 4d 45`) into the shared staging
   buffer 0x4dca0c and inits the reader FUN_0041cd90. What
   follows is **8 sequential `u16 count + count×rec`
   sections in a FIXED order** (16 FUN_0041cccb call sites,
   census-verified) — the whole .NME file feeds the critter
   bank (sections 1..7) and the POI bank (section 8):
   - **S1 (10 B rec)** → critter state 2 (sine-walk shooter):
     per record spawns `w1 + [0x46cbf8]` (difficulty + spawn
     base, typ. 4); x = (w3 + scatter(5)−2)·0x2000, y =
     (w4 + scatter(5)−2)·0x2000 (FUN_0041ec1c jitter); z =
     0xC000; variant param @+0x18 = scatter(4)+3, NEGATED
     when w2 (the flag word) ≠ 0; +0x5A = RandA&7; hp
     base 0xAF.
   - **S2 (10 B rec)** → state 1 (wander): spawns difficulty+3
     each; x = w3·0x20+0x10, y = w4·0x20+0x10; z from a DAT
     volume search: start plane z=6 at (w3,w4), walk DOWN
     while tile==0 or tile>3, take the first level whose
     tile ∈ 1..3 AND the cell above is 0; z =
     level·0x20+0x1F (also @+0x4E); skip if none; +0x56 =
     scatter(10)+10; hp base 0xC8.
   - **S3 (8 B rec)** → state 5: spawns `max(difficulty,1)`
     (d=1 → RandA&1+1); x = w2·0x2000+0xF00, y =
     w3·0x2000+0xF00; z = floor probe FUN_0041e411(x>>8,
     y>>8, w1<<5); 8 octile dists @+0x60 via the direction
     tables 0x4543e4/0x454404 (4 B × 8 entries); +0x0C=8,
     +0x10=0x72, +0x02=3, +0x0E=5; hp base 0x96.
   - **S4 (8 B rec)** → state 4 (the seek steppers): spawns
     (difficulty>>1)+2; x = w2·0x20+0xF, y = w3·0x20+0xF; z
     = probe (w1); all 8 octiles set = z; +0x0C=9,
     +0x10=RandA&3, +0x02=6, +0x0E=0; hp base 0xC8.
   - **S5 (10 B rec)** → state 3 (chase): ONE each; x =
     w3·0x2000+0xF00, y = w4·0x2000+0xF00 with home stored
     @+0x42/+0x46 and home z @+0x4A; z = probe (w2<<5);
     +0x10 = +0x12 = w1<<6 (timer/leash); +0x7A = −1; hp
     base 0x5DC (1500).
   - **S6 (8 B rec)** → state 6: one each; x = w2·0x2000+
     0xF00, y = w3·0x2000+0xF00; z = probe (w1<<5);
     +0x0C=8, +0x10=0x72, +0x02=3, +0x0E=5; hp base 0x96.
   - **S7 (6 B rec)** → state 7 (close combat): spawns
     max(difficulty,1); x = w1·0x2000+0xF00, y =
     w2·0x2000+0xF00; z FIXED 0xDF; +0x0C=3, +0x10 =
     scatter(0xFF, y); hp base 0x9C4 (2500).
   - **S8 (8 B rec) = the POI/personnel section**: resets
     `DAT_0046cbf0 = 0` @0x416f6e (the queue's POI count
     write), then per record spawns **4 POIs** at
     x = ((RandA&0x1F) + w2<<5)·0x100, y = ((RandA&0x1F) +
     w3<<5)·0x100, z = probe (w1<<5); seeds {+0 active=1,
     +2 0x32 (50 — same constant as the escape panic
     [0x4eba10]), +4 5, +6 1, +8 heading RandA&7}. If
     7j.17's state map (state@+4) holds, **personnel spawn
     directly in state 5 = ESCAPE** — consistent with the
     escape counter objective ([0x4eba0c]++, 5000 pts via
     FUN_00448b80).
   Every section stamps +0x00 = the 7j.17 state word
   {1,2,3,4,5,6,7}, +0x02 = species/type word {1,3,6},
   +0x24 = 1 (word), +0x06 = hp =
   base+(base·difficulty)/27. Epilogue: FUN_0041cd42
   (reader close) + FUN_004180b9 (**empty stub**,
   decompile-verified). The staging buffer 0x4dca0c is
   shared scratch (TRT/NME/MAP loaders all reuse it).
2. **Corpus verification [verified]:** the 8-section
   schedule consumes all 37 shipped .NME files exactly
   (36/37 byte-exact; ZONEA/MISSION1.NME leaves a 16-B
   orphan tail, words (1,0,18,0,66,0,1,0), that no game
   code reads — editor dregs). Field stats: w0 ≡ 1 in every
   non-empty section (marker, never read); S1 w1 ≤ 8;
   8-B w1 ≤ 7 (z level); all coords ≤ 99, zone-in-bounds.
   FORMATS-MISSION §9 rewritten from this decode — the old
   "header (n1,n2)" = the first two section counts, the old
   "(count,type)" = count + first word of record 1.
3. **FUN_0041fa51 = the EXIT-PAD ACTIVATOR** (the 5×0x1C
   exit-slot producer @0x41fabb): arg = a 0x4e44f8 PAD slot
   index (the .PAD runtime slots); dedup against the 5-dword
   id registry @0x46cd20 (skip if already active), else take
   the first −1 slot and stamp exit rec {+0 = 1, +4 = 1,
   x@+8 = pad.x·0x20+0xF, y@+0xC = pad.y·0x20+0xF (the pad
   slot's word x/y@+2/+4, asm-verified), +0x10 = 0x400,
   +0x14 = 0}. So exits are ELEVATOR PADS switched on at
   runtime. Sole caller FUN_00433980 @0x43900e (the pad
   trigger handler — not decoded this unit). Exit-bank
   consumers (census): FUN_00412a98 (POI flee, 7j.17) +
   **FUN_0041fbb1** @0x41fcf8 (new consumer — open).
4. **7j.17 leftovers folded:**
   - **FUN_00449c94 = the LOCAL COMMAND-RECORD BUILDER**
     [verified]: builds the 0x4dd4a0 stride-0x80 record for
     the local player slot (_DAT_004edb90 = robot id):
     byte@+0 = robot id, byte@+1 = the command code (the
     switch selector 1..4), then code-specific payload words
     (case 1: order selector 0x46cbdc, order target
     0x4dd484/88/8C, flag byte DAT_004ddb20 → optional
     MP/robot-bank words incl. 0x46cd04/08; case 2/3:
     network quit/join flavored; case 4: 7×(0x62·slot +
     0xE·k + 0x4de664) stat pairs + 10 B from 0x4e444c);
     record length @0x4eba08; then a broadcast loop over
     all slots (count DAT_0046cbe0) via FUN_00449b60 with
     network error paths ("NETWORK ERROR", "QUIT FROM
     NETWORK GAME" strings @0x459af4..0x459b4c,
     FUN_0044a38a/FUN_00420100 = the send family). The
     local-input side of the 0x4dd4a0 ring is CLOSED.
   - **FUN_0040db9e = the critter ranged-attack APPLIER on
     robots** [verified]: (robot_idx, mult, seed, ecx,
     param_5): FUN_0040e230(robot, ecx, [0x476fe4 +
     param_5·0xC]) = the SP damage core (7g) with a
     0xC-stride weapon-param table @0x476fe4 (param_5 = −1
     in the 7j.17 critter call → entry @0x476fd8, the
     critter's own weapon); if mult ≠ 0: robot word
     [0x4c69f4 + idx·0xA8] = 0xFFFF (a stun/disable mark on
     the ROBOT bank base word) + FUN_0040c536(idx, …,
     dist·mult>>7) — a timed effect scaled by the octile
     distance (FUN_0041eb65/FUN_0041eb77 = the dist family).
   - **[0x4eb8b8+slot·4] objective-done bank census**
     [verified]: consumers = MissionShell @0x4486ec,
     FUN_0044425c ×4 (@0x444775/0x444934/0x44496a/0x444813),
     FUN_00448b80 @0x448dee — all inside the
     mission-objective family (the resolver 7j.17 + the
     shell progress display + a 0x4442xx helper); no other
     readers. Identity: per-slot objective completion flags.
   - Projectile type 0x69 vs the FUN_00419aff damage table:
     NOT folded (would need the damage-table else-path dump;
     stays open, low priority).
5. Corpus-path verdict: docs + tooling (D66) — the
   inspector's heuristic NME walker is replaced by the exact
   8-section schedule (engine/bedlam-assets parse_nme + a
   corpus exact-consumption test); no sim/engine behavior
   change (critters/POIs still do not tick in the gates;
   their loader is now anchored for the P4.2 differential
   harness).

## 7j.19 Amendment 2026-08-21 (worker 90c04773, the
EXIT/ESCAPE RUNTIME family — rescue loop CLOSED end-to-end)

Method: three `-process BEDLAM.EXW -noanalysis` runs; dumps =
`ghidra-project/exw-exitfamily.txt` (FUN_0041fbb1 +
FUN_00433980 full decompile) + `exw-exitfamily2.txt`
(FUN_0040b9f6/FUN_00422e5e/FUN_004223b8) + `exw-exitfamily3.txt`
(FUN_0041faf0/FUN_0041fb4b/FUN_00424a6f) + `-xrefs` censuses +
`-asm` windows; cross-reads of the 7j.16/7j.17 artifacts
(exw-simtail, exw-missionrender2, exw-critterpoi-*). All facts
[verified] against those artifacts unless tagged.

1. **FUN_0041fbb1 = the ESCAPE-CRAFT ANIMATION RUNTIME**
   (MissionShell @0x448012, every frame, between FUN_004238af
   and FUN_004204ea). Three machines over one shared 0x1C
   record frame {active d@+0, PHASE d@+4, x d@+8, y d@+0xC,
   altitude d@+0x10, toggle d@+0x14, dwell d@+0x18}:
   - Machine 1: the 5 exit slots @0x4e662c. **REINTERPRETS
     the 7j.17 "+4 kind" field — it is a PHASE**: 1
     descending (altitude 0x400 → −0x20/frame while ≥0x101,
     then (v>>2)·3 shrink → land), 2 LANDED (flicker word =
     (RandA&7)==0, toggle ^=1, dwell++ > 0x78 → depart), 3
     departing (altitude += alt>>2+1, x −= toggle·4; > 0x200
     → active=0). The POI flee gate "kind==2" (7j.17) =
     flee only to LANDED elevators.
   - Machine 2: the extraction DROPSHIP, single slot
     @0x4e6610..0x4e6628 (same frame). Phase-1 landing fires
     the EXTRACTION SWEEP: every robot (count DAT_0046ccbc)
     with alive@+0x7C ≠ 0 ∧ state w@+0xC ∈ {3,4} → state :=
     5, timer@+0x90 := 0x28, _DAT_004dc680++ (extracted-robot
     counter), order target@+0x74 := 10000000, SFX
     FUN_0043a48e(_DAT_004edfe0,0,x>>8,y>>8,2). Phase-2 dwell
     starts at 10 (vs 0x78 for exits). Phase-3 end →
     active=0 ∧ _DAT_004dc67c := 1 = the extraction-complete
     flag (readers MissionShell 0x4486d5 + FUN_0044425c ×2
     @0x444aff/0x444b88; reset MissionShell 0x4478b3).
   - Machine 3: per-robot ESCAPE-POD bank @0x4e64c0 stride
     0x1C (one per robot, count DAT_0046ccbc), gated by
     latch [0x46aed4+idx·4]==0 — the per-robot no-extract
     latch (writers FUN_0040e230 = the SP death core (7g),
     FUN_00449c94/0044a38a (MP), FUN_00408e99, GameMain
     0x41c40d). Phase-1 landing fires the POD PAYOUT once:
     robot state := 6, timer@+0x90 := 0x28, alive@+0x7C := 1,
     +0x78 := 100·word@+0x94+5000 (points), SFX
     _DAT_004edfe0, per-player FUN_004239ef(p,p) msg.
2. **FUN_0041faf0 = the DROPSHIP DEPLOYER**: stamps
   {active=1, phase=1, toggle=0, altitude=0x200, x =
   beacon.x·0x20, y = beacon.y·0x20} from the extraction
   beacon words 0x4eabb4/0x4eabb6; clears the beacon display
   0x4eabb0/0x4eabb2. Sole caller MissionShell @0x44832f ∧
   0x448375 (asm-verified): when the beacon countdown word
   0x4eabb2 hits 0, OR every robot is dead/state-3. Beacon
   writer = FUN_004247b5 — CLOSED §7j.20 (zone pad script
   FUN_00433980 @0x433cfb arms it; FUN_004248c8 = the spread-claim
   picker FUN_0040b9f6 consumes to auto-walk robots to the beacon).
3. **FUN_0041fb4b(idx) = the POD SPAWNER**: stamps pod slot
   {1, phase 1, 0, altitude 0x400, x/y = robot pos>>8}.
   Sole caller FUN_0040b9f6 when the per-robot countdown
   w@+0x2C (0x4c6a10) hits 0 (msgs 9/10/0xB per player, then
   the pod anim). The 0x4c6a10 producers — CLOSED §7j.20: the
   FUN_0040cca0 spawn stagger (SP+MP) + the FUN_0040e230 MP
   respawn (0x28).
4. **FUN_00433980 = the ZONE PAD-TRIGGER SCRIPT DISPATCHER**
   (3185 B): arg = robot idx; sole caller FUN_0040b9f6
   @0x40bd58, invoked when robot state ∈ {1,4} ∧ order word
   DAT_0046cc30[idx] ≠ −1. Head: **FUN_00422e5e(x>>8, y>>8,
   z) = the PAD-TILE PROBE** — DAT-volume byte == 0xFF (a
   .PAD mark, 7j.16 loader) → scan the 999×8B .PAD slot bank
   @0x4e44f8 for an {x,y,z} tile match → slot index (with
   the 0x4eb9fc revisit-latch + 0x4eb9f4 counter), else −1.
   The slot id indexes a giant switch on zone _DAT_004edd8c:
   - ELEVATOR/TELEPORT ids (0x13..0x18, 0x5, 0xE, …): robot
     state := 2 (in transit), target := 1000000, order words
     −1, pos := scripted coords·0x2000+0x1000 (dword tables
     0x4dcdbc..0x4dd330), arrival platform +0x84 := 0..6,
     event latch −1-pair + countdown 10 (per-destination
     dwords 0x4dcdd4..0x4dd330);
   - message ids (≥0x3d range per zone) → FUN_00424a6f(id) =
     the zone MESSAGE SHOWER (string table @0x458ca7 "BOOT
     CAMP"…, SFX _DAT_004edfd0, per-id latch 0x4eb5f8);
   - door ids → FUN_004223b8(rect, 1|2) = the DOOR toggler
     over the 45×0x10 trigger-rect bank @0x4dcae8 (TOT
     stamps FUN_004235e4/FUN_004235bf over the rect W×H, SFX
     0x23/0x24);
   - **case 0x1B → FUN_004223b8(0x13, 2) + FUN_0041fa51(slot)
     — the SOLE exit-pad activation**: robot steps on the
     scripted pad → exit elevator deploys. The personnel-
     rescue loop is now CLOSED end-to-end: .PAD load (7j.16)
     → FUN_00433980 script → FUN_0041fa51 activator (7j.18)
     → FUN_0041fbb1 lands it (phase 2) → FUN_00412a98 POI
     flee (7j.17) → [0x4eba0c]++ → FUN_00448b80(5000)
     objective resolver (7j.17).
5. **Consumer censuses CLOSED:**
   - [0x4eba0c] rescue progress: writers MissionShell
     0x447933 (reset), FUN_00412a98 0x412b58 (++); readers
     MissionShell 0x448402 (display), FUN_00412a98 0x412b4a,
     FUN_00448b80 0x448ce1 (progress ≥ quota). No others.
   - [0x4eba10] rescue quota word: writers MissionShell
     0x44792d (reset) + 0x4483ac, FUN_00412a98 0x412b32
     (=0x32); reader MissionShell 0x448386 ONLY — display
     state; the objective check reads the slot quota
     w@+0x1E (0x4eaaee+slot·0x20), not this word.
   - 0x4e6610 dropship: renderer FUN_00403938 0x40707e
     (draws it) + MissionShell 0x44831c (spawn check);
     producer FUN_0041faf0, animator FUN_0041fbb1.
   - 0x4e64c0 pods: spawner FUN_0041fb4b, animator
     FUN_0041fbb1, renderer FUN_00403938 (the
     exw-missionrender2 0x406d6c loop: iso projection with
     the altitude/shadow math).
6. Residuals (small, queued): CLOSED 2026-08-21 by §7j.20 (the
   beacon writer FUN_004247b5, the spread-claim picker
   FUN_004248c8 body, the 0x4c6a10 pod-countdown producers).
   Still open: the full per-zone FUN_00433980 case table beyond
   the head (≈28 pad ids × 7 zones — mechanical, decode per zone
   only when P4.2 needs it; §7j.20 item 2 gives the ~25 armer
   pairs as an index); FUN_00424a6f string table contents.
7. Corpus-path verdict: docs-only (D67) — no engine change;
   the escape family stays unwired like the rest of the
   mission runtime (nothing escapes in the gates; all
   producers/consumers are now anchored for the P4.2
   differential harness).

## 7j.20. The extraction BEACON + POD-COUNTDOWN producers (2026-08-21,
worker c7269abe, claim 1 — closes the 7j.19 residuals; docs-only D68)

Sources: `-process BEDLAM.EXW -noanalysis` DecompList/XRefList runs
(ghidra-project/exw-beacon.txt, exw-beacon2.txt, exw-beacon-xrefs.txt,
process-exw-beacon*.log) + the 7j.19 dumps (exw-exitfamily2.txt =
FUN_0040b9f6/FUN_00433980) + a full-objdump census of every
`0x4c6a10`-displacement site (no absolute 0x4c6a10 reference exists).

1. **FUN_004247b5 = the EXTRACTION-BEACON ARMER** [verified decompile
   0x4247b5..0x424882 + xref census]: sole caller is
   **FUN_00433980 @0x433cfb — the zone pad-trigger script dispatcher**
   (7j.19 item 4). NOT a click handler: the old §6.4 "~0x433cbc
   robot-sprite click family" guess predated 7j.19's identification of
   the enclosing function — 0x433cbc lies inside FUN_00433980's body.
   Register args (EAX/EDX/EBX/ECX) = (robot pos_x>>13, pos_y>>13,
   robot z word@+0x08, robot idx): the beacon is armed AT THE
   TRIGGERING ROBOT'S TILE. Body:
   ```
   if (word@0x4eabb0 != 0) return          // one beacon at a time
   word@0x4eabb2 = 0x197                   // dropship countdown (407)
   word@0x4eabb0 = 1                       // ARM
   alive = # of first DAT_0046cbd8 robots with alive@+0x7C != 0
                                           // player-0 group (SP: the
                                           // whole squad; MP: 1 robot)
   if (alive == 1) word@0x4eabb2 = 0       // last robot: deploy NOW
   0x4eabb4/6/8 = tile x / tile y / z      // 0x4eabb8 z is a DEAD
                                           // STORE (xref census: zero
                                           // readers; the dropship
                                           // stamps only x/y)
   robot[idx].state = 3                    // halt (sweepable)
   FUN_004248c8(&tx,&ty)                   // spread claim
   robot.pos = (tx<<13, ty<<13)            // teleport to tile origin,
                                           // no +0xF00 offset (7b.5)
   FUN_004239ef(0x2a,3)                    // SFX 0x2A
   ```
2. **The armer call sites = ~25 (zone, .PAD slot) pairs** [mechanical
   parse of the FUN_00433980 decompile — the shared tail label
   switchD_00439754_caseD_15 + the common pad-switch slot-6 body; the
   exact per-zone table stays the deferred decode]: zone 1 slots
   {8, 0x10, 0x12, 0x18}, zone 2 {4, 5, 7, 0xE, 0x11}, zone 3
   {0, 1, 6, 0xF, 0x15}, zone 4 {0, 2, 0x10, 0x15, 0x16}, zone 5
   {8, 9 ×2, 0x3D}, + the shared pad-switch tail (slot 6) for the
   zones that reach it. Semantics: stepping a robot onto one of those
   scripted pad tiles arms the extraction beacon there — the zone's
   extraction pad. **The extraction trigger chain is now CLOSED
   end-to-end**: pad script (FUN_00433980) → armer FUN_004247b5
   (stations the trigger robot, state 3) → FUN_0040b9f6 auto-walks
   squad robots within 6 tiles to spread claims → MissionShell expiry
   gate (0x4eabb2 == 0 ∨ all robots dead/state-3, 7j.19 item 2) →
   FUN_0041faf0 dropship → FUN_0041fbb1 landing sweep (states 3/4 → 5)
   → _DAT_004dc680++ / _DAT_004dc67c complete.
3. **FUN_004248c8 = the SPREAD-CLAIM picker** [verified decompile
   0x4248c8..0x4249bd; callers FUN_004247b5 @0x424865 +
   FUN_0040b9f6 @0x40c08f]: scans the 12×u16 claim array 0x4eabba for
   the first free slot (scan bound DAT_0046ccbc = robot count), marks
   it 1 (claims are NEVER released — one-shot per mission, no clear
   site exists), and returns the beacon tile offset by slot:
   0 (0,0), 1 (+1,0), 2 (−1,0), 3 (0,−1), 4 (0,+1), 5 (−1,−1),
   6 (+1,−1), 7 (−1,+1), 8 (+1,+1), **9 (−2,0), 10 (0,−2), 11 (+2,0)**
   [completes 7b.6's slots 0..8]. Slot ≥ 12 → returns WITHOUT writing
   the out-params — both callers then store UNINITIALIZED locals into
   robot pos/target (a 13th+ consumer gets a garbage tile; faithful).
4. **The w@+0x2C (0x4c6a10) pod-countdown writers** [verified objdump
   census — all 10 displacement sites accounted]:
   - **FUN_0040cca0 spawn tail @0x40d132 = the mission-start DEPLOY
     STAGGER** [verified decompile]: after staging the squad, for each
     player (outer bound DAT_0046cbe0; 1 in SP) and each of that
     player's DAT_0046cbd8 robots: `w@+0x2C = 1 + k·(2000 −
     m·1000/27)`, k = robot index within the group, m = DAT_0046ae8c
     (linear mission number clamp((zone−2)·5 + level − 1, 1, 26)) →
     step 1037..1963 sub-ticks (≈173..327 frames at 6 sub-ticks/frame)
     between successive pod landings; each group's first robot lands
     at 1.
   - **FUN_0040e230 death-core MP branch @0x40e89d**: respawn re-drop
     `w@+0x2C = 0x28` (40 sub-ticks ≈ 6.7 frames) before the MRK /
     FUN_0041ec1c scatter reseed.
   - Reader/decrementer = FUN_0040b9f6's per-robot walk [7j.19 item 3
     + exw-exitfamily2.txt]: `w@+0x2C != 0` REPLACES the entire robot
     brain (movement, armor pass, pad scripts, beacon approach — all
     skipped) per sub-tick; hitting 0 fires FUN_0041fb4b(idx) pod-anim
     + msgs 9/10/0xB. The six 0x4073xx..0x4078xx dword reads are NOT
     +0x2C consumers — `sar edx,0x10` takes word@+0x2E (anim clamp ≤ 5
     for the SELECTED robot, idx DAT_0046cbd4) [verified asm 0x407377].
   Net semantics: **+0x2C = the per-robot DROP-POD descent timer** —
   robots deploy inactive-in-pod, staggered per group; the 0x4e64c0
   pod bank (7j.19 machine 3) is therefore the mission-start deploy
   pod AND the MP respawn pod, on top of its extraction role.
   [Corrections folded: §7g.3's "MP respawn sets +0x2C = 0x28; no SP
   producer known, always 0" — the SP producer IS this spawn stagger;
   the record-table row's "(1, 1+(2000−m·1000/27), …)" ellipsis was
   the per-robot k multiples.]
5. Bonus census [asm one-liner]: the FUN_0040cca0 tail also stamps
   4×0xC records at **0x4c71c4** (ending exactly at the 0x4c71f4 bank
   base) with the SELECTED robot's {x>>8, y>>8, z}; the renderer
   FUN_00403938 writes the same bank (0x403994/0x4039d2/0x403a27,
   dword-indexed per player) — 0x4c71c4 = the per-player selected-
   robot anchor records, spawn-seeded + renderer-updated.
6. Corpus-path verdict: docs-only (D68) — no engine change; the
   extraction family stays anchored-but-unwired for the P4.2
   differential harness (nothing escapes in the gates).

## 7j.21. The ARRIVAL-PRODUCER family decoded: FUN_00425da4 = the
ELEVATOR-RIDE STAGER (2026-08-21, worker b67abe61, claim 1 — docs-only D69)

Sources: `-process BEDLAM.EXW -noanalysis` runs (dumps
ghidra-project/exw-arrival1.txt = DecompList 0042034c+00425da4,
exw-arrival1-asm.txt = DumpRange 425d90..426200/426800..426a00/
4065c0..406700, exw-arrival2.txt = DecompList 0041a4f8+004223b8,
exw-arrival2-asm.txt = 41a4f8..41a590, exw-arrival3-asm.txt =
447760..4478c0/447a40..447c00/42034c..4204ea/402965..402976,
process-exw-arrival*.log) + the 7j.19 exitfamily dumps. All
[verified] asm/decompile unless tagged.

1. **FUN_00402965 = memset-0** [asm 0x402965..0x402974]: EAX:=0,
   ECX = byte count, EDI = dst; the SHR/STOSB/STOSW/REP STOSD
   unroll. 176 callers. Anchors every "clear bank" in this family.
2. **FUN_00425da4 (0x425da4..0x42c41e, 26 234 B, sole caller
   MissionShell @0x447b4e) = the ARRIVAL/ELEVATOR RECORD STAGER**,
   run once in the mission-load block (between FUN_00422171 and
   FUN_0041ec68, i.e. right before the robot spawn FUN_0040cca0
   and the .NME loader FUN_00416458) [verified boot asm
   0x447b3a..0x447b94]:
   - head: `FUN_00402965(0x4dcdb8, 0x654)` — clear all 45 records
     (45 × 0x24 = 0x654, the §7j.11 walk bound confirmed).
   - dispatch: `zone = [0x4edd8c]−1`; > 6 → return; jump table
     0x425d88 (7 zone cases). Per-zone branches gate on mode
     `[0x4edb88]` (== 2 → MP/present variant or skip) and mission
     `[0x4edd88]` (sub-switches; e.g. zone 1 stages ONLY for
     SP mission 1).
   - staging = straight-line FIXED-ADDRESS stores (the §7j.11
     "register-addressed" gloss was a Watcom-scheduling artifact;
     every site is a literal `[0x004dcXXX]` store). Per record:
     `+0x00 := 1` (active), `+0x04/+0x08/+0x0C` marker tile ←
     .PAD slot words (u16 x/y/z at `0x4e44f8 + slot·8 + 2`;
     slot +0 word skipped), `+0x10/+0x14/+0x18` destination :=
     immediates (tile x, tile y, level z), `+0x20 := −1`.
     **The countdown `+0x1C` is NEVER written by any producer**
     — records stage DORMANT (0 = skipped silently by the
     scheduler and the draw pass). The queue's "register-addressed
     countdown writes" premise is REFUTED; all arming is runtime
     (see 5).
   - record layout CORRECTION vs §7j.11: `+0x04` x, `+0x08` y,
     `+0x0C` z — three tile words (not "two x/y coord pairs").
   - worked example, zone 1 (mode ≠ 2 ∧ mission == 1): records
     0..6 — rec0 dest (8,0x39,2) marker .PAD slot 0; rec1..rec5
     dest (8,0x1A,5) markers .PAD slots 10..14; rec6 dest
     (0xE,0x20,1) marker .PAD slot 15.
   - per-zone record high-water marks (mission branches stage
     contiguous subsets from record 0): Z1 0..6 · Z2 0..16 ·
     Z3 0..16 · Z4 0..8 · Z5 0..9 · Z6 0..14 · Z7 0..6.
   - does NOT touch 0x4c71c4 — the §7j.20 per-player anchor bank
     is spawn-seed only (question CLOSED).
3. **The 0x4dcae8 rect-list boundary RESOLVED** (refutes the
   7j.12 "same producer family" hypothesis): the MissionShell
   boot block AFTER FUN_00425da4 sets ECX=0x2d0/EDI=0x4dcae8 at
   0x447b6c/0x447b71, calls FUN_0041a4f8 (the .BIN object loader;
   its own ECX/EDI uses are push/pop-guarded, so the caller's
   pair survives), then `FUN_00402965` @0x447b7b clears
   0x2d0 bytes at 0x4dcae8 — ending EXACTLY AT 0x4dcdb8
   (0x4dcae8 + 45·0x10 = 0x4dcdb8). The 45×0x10 door-rect list
   is a separate, adjacent bank; the arrival staging is untouched
   by the clear. Door-rect consumers use indexes 0..0x24 (slot 44
   never used as a door).
4. **FUN_004223b8 = the DOOR OPEN/CLOSE stepper** [decompile;
   re-anchored from §7j.19]: arg1 = rect index, arg2 = wanted
   state (1 open / 2 close); rect at 0x4dcae8+idx·0x10 =
   {+0 state, +2 x, +4 w, +6 y, +8 h, +0xA type}; guard
   state ≠ wanted ∧ state < 3; redraws the wall strip
   (FUN_004245c9, (x·0x20+w·0x10, y·0x20+h·0x10)); per cell tests
   the type-DB door-tile words (0x4796d5/0x4796d6 & 0x7f) and
   stamps/clears type<<4 via FUN_004235e4/FUN_004235bf; state
   word written back; SFX FUN_004239ef(0x23/0x24, 3) + bank
   0x4edfb0 once per transition. 86 callers (FUN_00433980 door
   pads).
5. **FUN_0042034c walk semantics CORRECTED** [asm
   0x42034c..0x4204ea]: the walk STOPS at the first inactive
   record — `CMP [ESI+0x4dcdb8],0; JZ 0x41e176` (0x41e176 = the
   shared Watcom pop-epilogue, also the 0x654-bound exit), so
   live records must be a contiguous active run from record 0
   (exactly what the stager produces). §7j.11's "active==0 →
   mark +0x20=−1, next" is wrong: −1 @0x4203fb is written only
   on the FIRE path. Countdown==0 records are skipped silently
   (not decremented, not drawn). Fire path re-verified: SFX at
   countdown==0xA (FUN_0043a48e bank 0x4edfe0, x<<5/y<<5 from the
   MARKER); on reaching 0 the robot@[+0x20] is teleported to the
   DEST (+0x10/+0x14/+0x18 → x<<13, y<<13, z·0x20−1), the DEST
   tile's platform burns (both gate banks 0x465daa/0x460dfa
   zeroed) and its first water level is cleared
   (FUN_0042394a(x,y,z,0,0)); z resettled via FUN_0041e231; the
   8 robot z-words +0x18..+0x26 filled; robot+0x0C word := 0
   (XOR-at-loop-end, exits the riding state); +0x20 := −1.
6. **The RUNTIME ARMER = FUN_00433980's elevator-ride cases**
   (§7j.19 family, now bound to the record structure; decompile
   exw-exitfamily.txt): each case targets ONE record: guard
   `+0x20 != −1` (ride busy) → return; else rider robot's state
   word@+0x0C := 2 (riding), +0x74/+0x84 order fields zeroed,
   pre-position the rider at the record's own MARKER
   (marker.x·0x2000+0x1000, marker.y·0x2000+0x1000), then
   `countdown := 10` and `+0x20 := rider`. Every armed countdown
   in the program is 10 (census of the `= 10` sites: records
   0..~23 across the zones). Net semantics: **the 45-record
   array = the ELEVATOR/TELEPORT RIDE PIPELINE** — the boot
   stager writes the per-(zone, mode, mission) elevator table
   (marker = boarding pad, dest = scripted arrival cell); a pad
   step arms a record; 10 ticks later the scheduler materializes
   the robot at the dest, burning the platform tile there.
7. **The record DRAW PASS decoded** (was census-only) [asm
   0x4065e5..0x4066e3]: loops the records; active==0 → exit the
   loop (consistent with the scheduler's stop); countdown==0 →
   skip; projects the MARKER tile isometrically
   (x' = [ESP+0x34] + (u−v) + 0x10d, y' = [ESP+0x4c] +
   ((u+v)>>1) + 0xac − (z<<5), u = (x<<5)+0x10−camera[0x4edde4],
   v = (y<<5)+0x10−camera[0x4edde8]); screen-bounds 0x23F/0x1FD;
   draws sprite 0x12E (FUN_0040798e, bank [0x46af38]) with
   width **w = clamp(11−countdown, 0, 9)** — the marker flashes
   only during the 10-tick ride and grows to full width just
   before materialization. The §7j.16 scanner-icon consumer
   (FUN_0041ee20 icon 0xB) is unchanged, second consumer.
8. Engine verdict: docs-only (D69) — the elevator ride pipeline
   stays anchored-but-unwired for the P4.2 differential harness
   (which must step a robot onto a scripted elevator pad to
   exercise it).

## 7j.22 Amendment 2026-08-21 (worker 27e4f048, the weapon-fire
family HEAD: the FUN_00410823 weapon-anim machine decoded)

Method: `DecompList`/`DumpRange`/`DumpAscii`
(-process BEDLAM.EXW -noanalysis), dumps =
`ghidra-project/exw-weaponanim.txt` (full 6102-B decompile) +
`exw-weaponanim-asm.txt` (full listing 0x410823..0x412000) +
`exw-weaponanim2.txt` (FUN_0041879d + FUN_0041874c) +
`exw-weaponanim2-data.txt`/`exw-weaponanim3-data.txt` (the
0x456bf0/0x456c78 artillery tables + burst-list bytes). All
facts [verified] against those dumps unless tagged.

1. **FUN_00410823(arg EAX = phase 0..3) = the WEAPON-ANIM/
   PROJECTILE TICK** over the WHOLE 400×0x36 bank 0x4c71f4
   (loop tail asm 0x411f95: idx++, EBP += 0x36, bound 0x190) —
   one walk per call; MissionShell calls it 4× per frame (the
   §2 i=0..3 loop). The type word@+0 IS the weapon-stat id
   (dword@(rec−2)>>0x10 at every read site). Record layout
   CLOSED (extends the §7j.17 map): w@+0 type (0 = free slot),
   d@+2 owner robot idx, d@+6 TARGET selector (type 0x29 only),
   d@+0xA tick counter, x/y/z Q13 d@+0x12/+0x16/+0x1A, vx/vy
   d@+0x1E/+0x22, vz d@+0x26 (straight types) , d@+0x2A class,
   d@+0x2E arc (= ballistic z-velocity, gravity −0x100/tick;
   = heading byte &0xFF for 0x29), d@+0x32 trail link (−1 =
   none). TWO class semantics [verified]: LAUNCH DELAY for
   0x24/0x29 (countdown each call, fly only at 0), DETONATION
   CYCLE COUNT for 0xF/0x13 (ttl≥0x65 → ttl=0, class−−; class
   0 → the 4-quadrant detonation).
2. **Phase cadence** [asm]: types 9..0xB tick ONLY on the
   phase-0 call (`CMP [ESP],0` @0x411583) = 1×/frame; the
   actor hit-tests run ONLY on odd phases (`TEST byte[ESP],1`
   @0x410bc4/0x411803/0x411e5a) = 2×/frame; everything else
   ticks all 4 calls.
3. **Types 2..4 = BULLETS (ray-walk with lagged commit)**
   @0x41087e: per call up to 2 sub-steps (x+=vx, y+=vy, z+=vz,
   ttl+=2), testing robot-lane hit (FUN_0041879d), MP actor
   (FUN_0041874c, [0x4edb88]==2 only), floor (FUN_0041e231(x>>8,
   y>>8,z>>8) > z>>8), bounds/ttl>99; the walk then ROLLS BACK
   one step and hit paths re-add it — 2 cells tested, 1
   committed per call (anti-tunnel lookahead). Terrain hit →
   FUN_0041a894(damage=FUN_00419aff(id), score flag 1) +
   FUN_0041bc1c + FUN_004124a4 disburser (K2, ±3 jitter); actor
   hit → disburser only (damage applied inside the lane);
   expire → type := 0.
4. **Type 5 = SHELL (K3 smoke trail)** @0x410b5a: one full move
   per call; robot-lane hit (odd phases) → store pos + disburser
   (K3); terrain hit → impact pair (FUN_00419aff(5) = 75) +
   disburser + free; else pos/ttl update AND a per-tick K3
   debris FUN_00420608(x>>8, y>>8, (z>>8)−10, 3, 0, owner) —
   the in-flight trail is the same kind the disburser emits.
5. **Types 9..0xB = ARTILLERY (scripted burst) @0x411583:
   fall 0x200/tick to the FUN_0041e411 floor (settle floor<<8);
   ttl==0x18 ∧ owner-robot word@+0x2A == player TYPE
   [0x4edb90] → FUN_004245c9 wall-strip redraw (spotter
   reveal); ttl≥0x20: while ttl−0x20 < dword[0x456c78+4·id]
   walk the i16 (Δy,Δx) pair list PTR[0x456bf0 + 4·(ttl−0x20)]
   (sentinel first-short 500) firing FUN_004244a1 (the
   5000-damage scripted blast) at tile+offset, 50%
   (FUN_00402975 = RandA [identity pinned @0x4116b5]) K0xB
   debris at the record center per pair; ttl>0x22 → disburser
   (9..0xB clear-only per the §7j.14 kind map). TABLES dumped
   [verified]: durations w9→2, w0xA→4, w0xB→7 frames; the 7
   per-frame lists @0x45687c/0x4568a2/0x4568d4/0x456936/
   0x456998/0x456a1a/0x456adc = expanding square rings (frame 0
   = 7-cell cluster around center, later frames radius-2/-3
   rings) — 7 pointers exactly cover max index 6.
6. **The BALLISTIC/BOUNCE family = {0xE, 0xF, 0x13, 0x17, 0x1A,
   0x1F}** (the §7j.14 K0xC cosmetic-debris set — now the
   in-flight semantics) @0x410ce6: z += arc, arc −= 0x100
   (gravity); x/y wall contact (FUN_0041e231 floor > z) →
   0xE/0x17 bounce (v = −v), 0x1A damped bounce (v = −(v>>1)),
   0x13/0x1F full stop (vx/vy/vz/arc := 0); FLOOR contact →
   0xE: full vertical bounce + horizontal halving + a 3-cell
   scripted detonation (FUN_004244a1 at center, (x−vx·4,
   y−vy·4), (x−vx·4, y+vy·4) — kill-anything bursts every
   contact), 0x17: bounce + 3-CLONE SPLIT (FUN_00412848 free
   slots, type 0x17, damped (vx,vy) rotated to (vy,−vx),
   (−vy,vx), (−vx,−vy)), 0xF/0x1F: damped roll (vx>>=1,
   vy>>=1, arc>>=2), 0x1A..0x1E: damped bounce (arc = −arc
   −arc>>1 unless 0xE); settle onto scenery: type-DB byte
   0x4796d5-family ≠ 0 → z = floor<<8. COMMON TAIL @0x41138e:
   ttl++; weapon 0xE ∧ link ≠ −1 ∧ ttl odd → append the PREV
   position to the SMOKE-TRAIL ring (see item 8). EXPIRY
   (ttl≥0x65): 0xF/0x13 → ttl=0, class−−; class==0 (any
   ballistic type) → FUN_004124a4 + the "weapon 0x1A"
   FOUR-QUADRANT detonation (the §7j.13 sites 0x410e59..0x410e9e
   re-anchored: 4× FUN_0041a894 + 4× FUN_0041bc1c,
   damage = FUN_00419aff(0x1A) = 75, at ±0x1000 Q13 half-tile
   offsets) + trail-link clear. Types 0xC/0xD, 0x10..0x12,
   0x14..0x16, 0x18/0x19, 0x1B..0x1E, 0x20..0x23, 0x25..0x28,
   >0x29: NO tick in this function (0x10..0x12/0x1B..0x1E are
   the spawner-side scatter ids of §7j.17, not tick types).
7. **Type 0x24 = ROCKET @0x411744**: class countdown (launch
   delay) → straight flight (x/y/z += v, NO gravity, z<0 →
   z=0x1000 + K6 disburser + free); actor lanes (odd phases,
   MP gate for the robot lane); floor via FUN_0041e231 → impact
   pair FUN_00419aff(0x24) = 400 + disburser (K6); ttl>0x64 or
   out-of-bounds → free; bounds-exit clears the trail link.
8. **Type 0x29 = HOMING MISSILE @0x411905**: class = launch
   delay; TARGET dword@+6 [asm 0x411974..0x411a04]: bit 0x1000
   → robot ((t&0xFFF)−1)·0xA8, aim (z+0x15)<<8; bit 0x2000 →
   TRT structure (t&0x1FFF)·0x20 off the 0x4cccec stager frame,
   tile·0x2000+0x1500; else → critter t−1 via FUN_004128ec +
   z+0x1500. Homing: z eases ±0x200/tick clamp [0,0xFF00],
   ground-lift +0x200; heading byte (+0x2E) steered by the
   FUN_0041ebf8/FUN_0041eb7d/FUN_0041ebc1 angle family +
   FUN_00412a19 clamp (turn·4); velocity 2·(cos>>4, sin>>4)
   (FUN_0041eb65/FUN_0041eb77); OBSTACLE AVOIDANCE loop ±0x40
   in 4-sector steps (FUN_0041e411 floor tests; blocked →
   heading := candidate, z += 0x600 climb). Target-dead gates:
   critter state==7/zero-word @0x4cffa2+0x7E·(t−1) or structure
   active==0 (0x4cccd8 idiom) → disburser + fizzle; floor
   (FUN_0041e411) → impact pair FUN_00419aff(0x29) = 250 +
   disburser (K9); ttl>0xC8 (201) or bounds → free.
9. **The actor hit-test FRONT DOORS** (7j.13's "muzzle-offset
   target" gloss replaced): FUN_0041879d(owner, x, y, z,
   weapon) = the CRITTER lane — 3-row presence-grid prefilter
   (dwords at rows [0x4ea900 + (y>>13)·4 −4/0/+4] + row ptr +
   (x>>13)−1, the §7j.17 presence marks) then per-critter
   FUN_004190bc(critter, owner, x, y, z, weapon, MODE 2);
   FUN_0041874c(owner, x, y, z, weapon) = the OTHER-ROBOT lane
   (MP only, [0x4edb88]==2 at every caller): per-robot
   FUN_00418fca(robot, x, y, z, weapon, 2), skipping the owner.
   **This CORRECTS the §7j.15/queue hypothesis that
   FUN_004190bc is a panel/preview**: it is the CRITTER
   hit-test/damage applier (its 6 FUN_00419aff stat reads =
   per-critter damage lookups), and FUN_00418fca is the ROBOT
   sibling — both called with mode 2 from this family
   [internals still open, head-bounded].
10. **The 0x4e66b8 SMOKE-TRAIL bank** (immediately after the
    5×0x1C exit slots 0x4e662c): stride 0x68 = {d@+0 active,
    d@+4 ring counter &7, 8×0xC xyz ring @+8..+0x67}; the
    weapon-0xE ballistic tick appends {x−vx, y−vy, z−arc} (the
    previous position) every 2nd tick; cleared at free/expiry/
    detonation (d@(0x4e66b8 + link·0x68) := 0, sites
    0x410d51/0x410eac/0x411744-family bounds path). The link
    slot allocation (writer of +0x32 ≠ −1) stays OPEN.
    [CLOSED by §7j.23 item 8: allocator FUN_00412a4a, 20
    slots; writer FUN_0040a9ff at mortar spawn.]
11. Engine verdict: docs-only (D70) — unchanged from §7j.13:
    no corpus producer reaches any fire site; the family stays
    anchored-but-unwired for P4.2. Re-opens cleanly at:
    FUN_004190bc/FUN_00418fca internals (the critter/robot
    damage application + the remaining FUN_00419aff reads), the
    0x4e66b8 slot allocator, projectile type 0x69, and the
    draw pass that consumes the trail rings.

## 7j.23 Amendment 2026-08-21 (worker ad591680, the weapon-fire
family TAIL head: the ACTOR HIT APPLIERS + the trail allocator)

Method: `DecompList`/`DumpRange`/`StoreScan` (new scanner;
finds computed refs) — all `-process BEDLAM.EXW -noanalysis`;
dumps = `ghidra-project/exw-hitters.txt` (FUN_004190bc,
FUN_00418fca, FUN_00419aff, FUN_0041879d, FUN_0041874c) +
`exw-hitters-asm.txt` (listing 0x418fca..0x419760) +
`exw-hitters2.txt` (FUN_0041ebf8/00421fc2/0041a028) +
`exw-hitters3.txt` (FUN_0040a9ff/00412a4a) + `exw-hitters4.txt`
(FUN_0040e230 head) + `exw-hitters-scan.txt` (whole-program
operand scan for 0x4c7226/0x4e66b8/0x4e66bc). All facts
[verified] against those dumps unless tagged.

1. **FUN_004190bc(critter EAX, owner EDX, x EBX, y ECX, z
   stack, weapon param_6, mode param_7) = the CRITTER HIT
   APPLIER** over the 0x7E-stride critter bank (§7j.17 base
   0x4cff98, count [0x46cc2c]). Record words now anchored:
   **kind w@+0x00** (the .NME section states {2,1,5,4,3,6,7}
   of §7j.18 ARE the switch cases 1..7), attacker u16@+0x04,
   **hp s16@+0x06** (dword@+4 >>0x10 = the death test),
   **state w@+0x0C** (task's "+0xC"), sub-timer w@+0x0E,
   knock heading d@+0x10, impact x/y d@+0x1C/+0x20, xyz Q13
   d@+0x36/+0x3A/+0x3E, timer w@+0x56, knock vx/vy w@+0x74/
   +0x76, **hit flash w@+0x7C**, **presence w@+0x24** (task's
   "+0x24 word"; ==0 → return 0). Signature confirmed = the
   §7j.22 lane call; a THIRD caller exists in the renderer:
   FUN_00403938 @0x4190bc-call `exw-missionrender.txt:4186`
   = FUN_004190bc(critter, owner −1, x<<5, y<<5, z<<5, weapon
   0xC, mode 2) — the w0xC=5000 direct blast.
2. **Mode semantics** [verified]: mode 2 = full 3-D test
   (octile FUN_0041ebf8(dx,dy) < 0x20 on x/y AND |dz| < kind
   threshold); mode 1 = x/y test ONLY (z ignored — used by no
   in-corpus caller found yet; the lanes always pass 2).
   Per-kind coordinate storage: kinds 1/4 store x/y in CELL
   units (raw compare vs the caller's >>8 args); kinds 2/3/5/
   6/7 store Q13 (>>8 at compare). z thresholds: 1/2/4/5/6
   <0x20, 3 <0x24, 7 <0x40 (tall). Kinds 3..7 additionally
   hard-guard state w@+0x0C ∉ {6,7,0xB} (dying/ballistic/
   dormant immunity — same states as the §7j.17 machine).
3. **Hit application** [verified]: hp@+0x06 −= FUN_00419aff
   (weapon) (the §7j.15 damage table — the 7j.22 gloss
   "per-critter damage lookups" is CORRECTED: the lookup is
   per-WEAPON, critter-kind-independent); attacker@+0x04 :=
   owner&0xFFFF; flash w@+0x7C := 1; kinds 4/5/6/7 state :=
   5 (stun) and impact x/y@+0x1C/+0x20 := x/y<<8 (kind 3
   stores them raw). Death (hp ≤ 0) dispatches per kind:
   1→FUN_00418835, 2→FUN_004188d0, 3→FUN_00418aa6,
   4→FUN_00418ca4(critter,weapon), 5/6→FUN_00418e26(critter,
   weapon), 7→FUN_0041896c — the per-kind death-handler
   family (kinds 4/5/6 take the weapon id = weapon-dependent
   drops/effects). Survivors: kinds 3/4/5/6 call FUN_00421fc2
   (hit juice) and kinds 4/5/6 additionally 25% (RandA&3==0,
   owner ≠ −1) stage the KNOCKBACK effect FUN_0041a028;
   kind 7 instead computes its own knock vector IN-RECORD
   (heading d@+0x10 = angle(impact−critter)+0x80 away-jitter,
   vx/vy w@+0x74/+0x76 = cos/sin>>6, state := 5, timer@+0x56
   := 0 — no SFX call, no effect row).
4. **FUN_00421fc2(x,y) = impact SFX**: gated [0x4ede58]≠0;
   RandB()%3 picks one of the three banks 0x4edf7c/0x4edf80/
   0x4edf84 (the §7j.17 critter/POI trio) → FUN_0043a48e
   (bank, 0, x, y, 2).
5. **FUN_0041a028(x Q13, y Q13, z Q13, robot_x Q13, robot_y
   Q13) = KNOCKBACK effect row stager** — a SECOND spawner
   for the
   0x20-stride effect rows @0x4cec38 (besides FUN_0041a14f,
   §7j.17): FUN_0041a494 allocates the row; stores x/y/z Q13;
   heading = atan2 family (FUN_0041eb7d/0041ebc1 = angle
   +0x80 flip = AWAY from the shooter) ± RandA&0x1F−0x10;
   vx/vy = cos/sin (FUN_0041eb65/77) >>8 into w@+0xE/+0x12
   (row-relative: cos@+0xE, sin@+0x12, ttl d@+0x16 :=
   RandA&0x3F+0x1F, kind w@+0x1A := FUN_0041ec1c(5,0)+3,
   w@+0x1C/+0x1E := 0, w@+0 := 0); then FUN_00420608((x>>8)+1,
   (y>>8)+1, max((z>>8)−0x20,0), 10, 0, −1) — the "critter
   flies away" juice. FUN_0041ebf8 = OCTILE distance
   max+min/2 (also the §7j.22 prefilter metric).
6. **FUN_00418fca(robot EAX, x EDX, y EBX, z ECX, weapon
   param_5, mode param_6) = the OTHER-ROBOT HIT APPLIER**:
   presence d@+0x7C≠0; |x−(d@+4>>8)|<0x20 ∧ |y−(d@+8>>8)|<
   0x20 ∧ (mode 2: |z−d@+0xC|<0x30 — z RAW, no shift, AND no
   octile: pure box test); on hit FUN_0040e230(robot,
   FUN_00419aff(w@rec+0), d@rec+2 owner) then clamp hp d@+0x78
   ≥ 0; returns 1. Owner-damage (own shots) is NOT excluded
   here — the exclusion is the caller's idx≠owner skip.
7. **FUN_0040e230(robot, damage, owner) = the ROBOT DAMAGE
   APPLIER** [head decode, first ~120 of 268 decompile
   lines]: guards state w@+0x0C == 2 (dying) skip / presence;
   state 3 (extract/depart, §7j.19) → active-shield d@+0x88
   := 0x20; damage gated by
   d@+0x8C==0 ∨ d@+0x88≠0; alarm: w@+0x34==0 → damage-counter
   d@+0xA4 += 3, >100 ∧ view == [0x4edb90] → per-slot warning
   SFX 0x10/0x11/0x12 (FUN_004239ef, robot ==
   [0x46cbd4]+k, k<2 gated [0x46cbd8]) then w@+0x34 := 100
   (cooldown), counter := 0; shield-down path: hit count
   w@+0x2E++, hp d@+0x78 −= damage, tier warnings vs 5000+
   100·variant(d@+0x94): cross → SFX 0x2B/0x2C/0x2D, ≤50% →
   0x13/0x14/0x15, ≤12.5% → 0x16/0x17/0x18 (per-slot);
   shield-up path: d@+0x88 −= damage clamp 0 (absorb);
   death (hp < 1) MP mode [0x4edb88]==2: 0xC-stride
   scoreboard @0x4ebaa8 per robot {score d@+0, flag d@+4,
   d@+8 := 0xB} — killer(owner≠victim): flag 1, score++ cap
   999; victim: score−− clamp 0, flag 0 (SP tail not decoded
   here; FUN_0040e230 is also a [0x46aed4+idx·4] no-extract
   latch writer per §7j.19).
8. **Trail allocator CLOSED (§7j.22 item 10 open point)**:
   `StoreScan` found the whole-program writer set of the
   +0x32 link. **FUN_00412a4a = the SMOKE-TRAIL SLOT
   ALLOCATOR**: linear scan of 20 slots (0x4e66b8 + i·0x68,
   bound offset 0x820 = 20·0x68), first with d@+0 == 0, else
   −1. **FUN_0040a9ff(robot, slot, mask, rec_idx) = the MORTAR
   SPAWNER** (fire dispatcher §7j.17 case 0xC helper): arg3
   (mask) == −1 = maskless variant (trail alloc, no ammo
   bookkeeping); else ammo w@(robot·0xA8+8·slot+0x38)--, 0 →
   weapon mask XOR w@+0x6E; if the slot's weapon id
   (w@(robot·0xA8+8·slot+0x36)) == 0xE → slot state
   w@+0x2C := 0xC (slower reload) + allocate trail slot,
   record d@+0x32 := slot, trail slot d@+0 := 1, ring 8×0xC
   zeroed; NON-mortar spawns set d@+0x32 := 0 (sentinel —
   harmless, only type 0xE reads it). Then SFX _DAT_004edf94
   (§7j.17 robot-fire bank); ballistics = /8-normalized unit
   vector toward the order target (0x4dd484/88/8C) ×2,
   ttl 0x32, arc 0x500, spawn +0x15 above the robot; record
   type := the slot weapon id. (The dispatcher's OWN case 0xE
   spawns
   2× type-0xF jittered sub-shells, NOT 0xE — the 0xE record
   itself comes only from this helper.) Remaining consumer
   open: the trail-ring DRAW pass (FUN_00403938 reads the
   link @0x404464; FUN_00412a4a's CMP is the allocator probe
   itself).

## 7j.24 Amendment 2026-08-21 (worker 0f986419, the CRITTER
DEATH-HANDLER family + the FUN_0040e230 SP-tail addendum)

Method: `DecompList`/`XRefList` + read-only objdump spot-checks
(0x418835, 0x418aa6 regions), all `-process BEDLAM.EXW
-noanalysis`. Dumps: `ghidra-project/exw-dead1.txt` (the six
handlers + FUN_00424355 + FUN_0041a14f — predecessor WIP from
worker ad591680's 7j.23 session tail, adopted), `exw-dead2.txt`
(FUN_00421f4c/FUN_0041a494/FUN_0042382c), `exw-dead3.txt`
(FUN_004238ea), `exw-hitters4.txt` (FUN_0040e230 FULL body —
7j.23 head dump already contained the whole death tail),
`exw-dead4.txt` (FUN_00418a9f + the family xref census),
`exw-dead5.txt` (FUN_0040dce0). All facts [verified] against
those dumps + objdump unless tagged. Record layout per §7j.23
(base 0x4cff98 + idx·0x7E: kind w@+0x00, attacker u16@+0x04,
hp s16@+0x06, state w@+0x0C, sub-timer w@+0x0E, impact d@+0x1C/
+0x20, presence w@+0x24, xyz d@+0x36/+0x3A/+0x3E, timer w@+0x56,
knock vx/vy w@+0x74/+0x76, flash w@+0x7C).

1. **THE SIX PER-KIND DEATH HANDLERS** [verified; Watcon args
   EAX = critter idx, EDX = weapon id (k4/k5/6 only); FUN_00420608
   = the §7j.11 debris stager (px args, kind, delay, owner)]:

   | kind | handler | record writes | debris (FUN_00420608) | rows (FUN_0041a14f) | SFX | bounty |
   |---|---|---|---|---|---|---|
   | 1 | FUN_00418835 (155 B) | state := 7, presence := 0, d@+0x52 := 0 | 1× k1 @ (x,y,z raw — k1 px-scale), delay 0 | — | — | +0x1E (30) |
   | 2 | FUN_004188d0 (156 B) | presence := 0, state := 7 | 1× k0xD @ (x>>8, y>>8, z>>8), delay 0 | — | — | +0x32 (50) |
   | 3 | FUN_00418aa6 (510 B) | state := 7, timer w@+0x56 := 0, d@+0x52 := 0 | 1× k7 @ (x>>8, y>>8, z RAW Q13 → stager-clamps 0xFF) delay 0; then 3× k6 @ (x>>8±(RandA&0xF)−7, y>>8±(RandA&0xF)−7, z RAW+(RandA&0xF)−7) delays 0/2/4 | — | FUN_00421f4c(x>>8, y>>8) | +500 |
   | 4 | FUN_00418ca4 (386 B, +weapon) | w@+0x02 := 1 (the §7j.17 "substeps" word — plausibly the death-anim rate [inferred]), hp := 0, state := 6, d@+0x52 := 0, timer w@+0x56 := 6 | 1× k7 @ (x,y,z raw — k4 px-scale) delay 1; weapon ∈ {0x24, 0x29, 0xC} → 3× k7 @ (x±(RandA&0x1F)−0xF, y±…, z+(RandA&0xF)−7) delays 1/2/3 | weapon-gated: 8 rows @ ((x<<8), (y<<8), (z+0x15)·0x100) | FUN_00421f4c(x, y raw) | +0x4B (75) |
   | 5/6 | FUN_00418e26 (420 B, +weapon) | w@+0x02 := 1, hp := 0, state := 6, sub-timer w@+0x0E := 0, d@+0x52 := 0 | 1× k7 @ (x>>8, y>>8, z RAW → 0xFF) delay 1; weapon ∈ {0x24, 0x29, 0xC} → 3× k7 @ (x>>8±(RandA&0x1F)−0xF, …, z+(RandA&0xF)−7) delays 1/2/3 | weapon-gated: 12 (0xC) rows @ (x, y RAW Q13, (z+0x15)·0x100) | FUN_00421f4c(x>>8, y>>8) | +0x96 (150) |
   | 7 | FUN_0041896c (307 B) | state := 6, w@+0x78 := 1 (low word of the §7j.17 +0x78 dword — semantics open) | 3× k7 @ (x>>8±(RandA&0x3F)−0x1F, y>>8±…, 0xFF−(RandA&0xF)) delays 1/2/3 — gibs falling from the top; then 1× k0xD @ (x>>8, y>>8, z RAW → 0xFF) delay 0 | — | FUN_0043a48e(_DAT_004edff8, 0, x>>8, y>>8, 3) FIRST | +1000 |

   Kinds 1/2/3 die INSTANTLY (state 7 + presence cleared — no
   corpse anim); kinds 4..7 enter the DYING anim (state 6, the
   §7j.17 controller mode 7 runs 0x28 frames). The per-kind
   coordinate split (k1/k4 px-raw vs others Q13 >>8) confirms
   §7j.23 item 2 exactly [asm-verified 0x418adc/0x418af2: `sar 8`
   on x/y, z raw]. Weapon gate {0x24 rocket, 0x29 homing, 0xC
   5000-blast} = "killed by an explosive" → the 3 extra k7 debris
   + the 8/12-row splash. Debris-kind census (§7j.11 item 6)
   unchanged — all these sites were already listed; the kinds now
   have producers: k1 (small chunk), k6 (chunk), k7 (pure-anim
   no-ring), k0xD (tumble-family shared body w/ k1).
2. **THE BOUNTY GATE** [verified, identical in all six]: attacker
   u16@+0x04 ≠ −1 (0xFFFF) AND robot[attacker].type w@+0x2A
   (d@robot+0x28 >>0x10) == [0x4edb90] (the player-type word) →
   score `_DAT_004dd40c` += bounty AND `DAT_0046ccf0 := 2` (the
   score-strip refresh flag, same mechanism as the §7j.6 item 6
   pickup awards). Env deaths (attacker −1) and other-player
   kills award nothing. The §7j.6 item 9 `_DAT_004dd40c` writer
   census gains these six sites (0x418867/0x4188e6/0x418a92/
   0x418d3e/0x418f04/0x418955 region).
3. **SECOND DISPATCHER: FUN_0040dce0 = the DEBRIS-CRUSH death
   dispatcher** [verified — CORRECTS the §7j.23 implication that
   FUN_004190bc is the only dispatch site; xref census: sole
   caller FUN_0040de9c @0x40e13b (the §7j.7 debris
   physics/collision tick)]. Args (critter idx EAX, mag EDX,
   heading EBX, dmg ECX). Guards: w@+0x02 ∉ {7,2} [field meaning
   open — the §7j.17 "substeps" word], mag > 2, dmg ≠ 0. Then
   FUN_0040eb3c(idx, dmg) applies the damage; FUN_004128ec reads
   xyz; knock = pos + sin/cos(heading)·mag (FUN_0041eb65/77);
   move FUN_00412998(idx, x', y', −1) when kind == 7 OR
   FUN_0041e9a2(x'>>8, y'>>8, idx) ≠ 0 [wall test, census-noted];
   then hp ≤ 0 (d@+0x04 >>0x10 ≤ 0) → attacker w@+0x04 := −1 and
   per-kind dispatch: k4 → FUN_00418ca4(idx, 0) (NO weapon → no
   explosive drops), k5/6 → FUN_00418e26(idx, **0x24**) (as-if
   rocket → FULL explosive drops) but SKIPPED while state
   w@+0x0C ∈ {5,6} (stunned/mid-death crush is absorbed — impact
   x/y d@+0x1C/+0x20 still get the knocked pos), k1/2/3/7 →
   plain. So flying debris CAN kill critters with attacker −1
   (no bounty) — a second corpus-independent producer family for
   the §7j.11 debris kinds.
4. **FUN_00421f4c(x, y) = the CRITTER-DEATH SFX trio** [verified
   via exw-dead2]: gated [0x4ede58] ≠ 0, RandB()%3 → one of banks
   0x4edf88/0x4edf8c/0x4edf90 → FUN_0043a48e(bank, 0, x, y, 2).
   Structural twin of the §7j.23 impact trio FUN_00421fc2
   (0x4edf7c/80/84) — a fourth §7j.17 SFX-bank triple. k7 instead
   uses the dedicated bank _DAT_004edff8 with push 3 (the only
   push-3 caller known). A StoreScan for stores to these bank
   words found none [direct stores absent — runtime pointer
   init, unresolved; low priority].
5. **FUN_0041a14f(x Q13, y Q13, z Q13, count) = the 0x4cec38
   effect-row SPAWNER, fully decoded** [verified]: per row:
   slot = FUN_0041a494 (ages EVERY row's w@+0 — CORRECTS the
   §7j.23 knockback gloss "w@+0 0": w@+0 is an AGE word,
   incremented once per spawn call — then returns the MAX-age
   row: always-evict LRU over 0x50 = 80 rows × 0x20 = the 0xA00
   bank, consistent with §7j.1); writes {age w@+0 := 0, x d@+2,
   y d@+6, z d@+0xA, cos d@+0xE = FUN_0041eb65(rand)>>8, sin
   d@+0x12 = FUN_0041eb77(rand)>>8, d@+0x16 := (RandA&7)·0x10 +
   0x80, w@+0x1A := i if i < 8 else FUN_0041ec1c(5,0)+3
   [inferred: a sprite/variant id — deterministic 0..7 walk for
   counts ≤ 8, random beyond], w@+0x1C := 0, w@+0x1E := 0}.
   Callers: FUN_00418ca4 (8 rows), FUN_00418e26 (12 rows),
   FUN_00412f34 @0x413244 (controller ballistic landing, 0x18
   rows — §7j.17). FUN_0041a028 (§7j.23 knockback) is a parallel
   writer with a different +0x16 ttl (RandA&0x3F+0x1F).
6. **§7j.17 expectation CORRECTED**: the death handlers call
   FUN_00424355 ZERO times — no splash rings from the death
   dispatch itself. The critter-path FUN_00424355 producers
   remain the CONTROLLER's mode-6 ballistic landing (5 chunks,
   §7j.17 item 1) and the suicide-bomb trigger FUN_00417e2f
   (8 rings, §7j.17 item 2). FUN_00424355 itself re-verified
   against exw-dead1: gates = map bounds, z ≤ 7, type-DB
   z-word 0 + volume 0 (FUN_0041eb28), claim byte
   [0x46af58-bank+tile] == 0; 250 (0xFA) slots stride 0xA at
   0x4e9776 {x w@+2, y w@+4, z w@+6, delay w@+8, age w@+0xA};
   alloc = first age-0 else max-age (FUN_0042394a(old x,y,z,0,0)
   cancels the evicted record) — matches §7j.10 exactly.
7. **FUN_00418a9f = a NOP stub** [verified, 0x418a9f..0x418aa6,
   empty body]: called at the end of the k3 handler AND from
   FUN_004197d4/FUN_00419943/FUN_00419c7c (+ conditional jump
   from FUN_00419f62) — a cut-feature/placeholder hook; the k3
   call does nothing.
8. **ADDENDUM: the FUN_0040e230 SP tail — CONFIRMED + closed**
   [verified against the full exw-hitters4 body; extends §7j.7
   item 6 / §7j.23 item 6]: on hp < 1 the SHARED tail (both
   modes) = MP-mode scoreboard first (mode 2 only: killer ==
   victim ∨ killer == −1 → SUICIDE flag 0 + score−− clamp 0;
   else killer flag 1 + score++ clamp 999; victim always flag 0,
   score−− clamp 0, state d@+8 := 0xB — the clamp/suicide gate
   is new vs §7j.23), then FUN_0042382c(idx) + DAT_0046ccec := 3
   + the seven order words +0x38..+0x68 (step 8) zeroed + 5×
   kind-5 debris (2× RandA each, z = robot z dword + 8k, delays
   0/2/4/6/8). SP gate ([0x4edb88]==0 ∨ respawn_ok[idx]): if idx
   == [0x46cbd4]+[0x46cbdc] → _DAT_004ede34 := 1;
   alive d@+0x7C := 0, drop d@+0x80 := 0, hp d@+0x78 := 0,
   d@+0x9C := 1, armor w@+0x30 := 0, death SFX 0x19/0x1A/0x1B per
   squad slot. MP else-branch = full respawn: selected-robot
   death-spot marker {0x4ea8ec/f0 := x/y>>8, 0x4ea8f4 := z,
   0x4ea8f8 := 0x20}; +0xA0 := 0, alive := 0, order slots
   [0x466cc30+idx·4] := −1, d@+0x9C := 1, [0x466cc60+idx·4] :=
   −1, d@+0xB0 := 0, shield +0x88 := 0, hp := 0, hit-flash
   w@+0x2E := 0, state w@+0x0C := 0, armor := 0, w@+0x10 := −1,
   w@+0x16 := −1, w@+0x5E := 0, variant w@+0x18 := RandA&3, pod
   timer w@+0x2C := 0x28; mode 2 → random MRK marker
   (FUN_0041ec1c(0xC)) → pos := marker·0x2000+0xF00, z :=
   word·0x20−1; 8 z-words +0x1A..+0x28 := z; probe re-seed
   FUN_0041e897(x>>8, y>>8, idx); 7-slot weapon re-copy from
   0x4de664 (0x62-stride/type: w@+0x36/+0x38 := table words,
   first-nonzero sets order-bits w@+0x5E |= 1<<k); 2-entry
   equipment switch from 0x4deafc (0x1C-stride/type, ids
   0x2A→+0x8C, 0x2B→+0x94, 0x2C→+0x98 = value·200,
   0x2D/0x2E→[0x46ae94+type·4] := 1/2).
9. **FUN_0042382c = the robot-death BLAST-EFFECT stager — first
   confirmed producer of the 0x4eb638 bank** [verified via
   exw-dead2/3]: gate = tile claim/reveal byte
   [0x46af58-bank + [[0x4ea900+(y>>0xD)·4]+[0x46af4c]+x>>0xD]]
   == 0; slot = FUN_004238ea (32 slots, first d@+0xC == 0 else
   MIN-d@+0xC — LRU); writes 0x4eb638+slot·0x14
   {d@+0 := robot.x, d@+4 := robot.y, d@+8 := robot z dword
   (d@+0x08), d@+0xC := 1 (age), d@+0x10 := 0}. The 0x4eb638
   32×0x14 bank is the MISSIONVIEW §5d "platform loop" bank —
   its draw pass consumes robot-death blast records [inferred
   identity: same base address; the draw-side decode stays
   backlog].

Engine seam: NONE this unit (docs-only, D72) — critters load
outside the current corpus gates; the death family re-opens
when §7j.18's .NME staging lands engine-side. Pins untouched.

## 7j.25 Amendment 2026-08-21 (worker 399aeff4, the
FUN_0041a894 destroy-tail effect-entry map + FUN_0041a225)

Fresh objdump of 0x41a860..0x41bc10 (jump-table bytes +
destroy tail) + 0x41a225..0x41a310 + 0x41a4cc..0x41a4f2 + the
0x43a2xx bank-name loader + DGROUP strings 0x458e7b..0x458f9a.
Closes the 7j.22/7j.23-promised 9-case selector decode. All
[verified] asm unless tagged.

1. **The destroy tail = TERRAIN RESTORE then a FIVE-EFFECT
   loop.** Order inside the hp≤0 path (0x41a95d onward):
   hp:=0/flags|=0x40 → notify [0x46cce4]:=2 → zone≠1 →
   FUN_00448b80(idx) → FUN_00422e0a(id) → FUN_00422600(id) →
   **GER gate REFINED (7j.13 gloss corrected)**: the
   type@+0xE==0xb ∧ [0x4eba1c]==1 test at 0x41a9ac jumps to
   the `ret 0` epilogue — the record IS already marked
   destroyed and the trigger producers DID run, but the
   restore/effect/score/chain tail is SKIPPED whole.
2. **Terrain restore [verified 0x41a9c3..0x41ac0b]**: nested
   loops i<H (word@+4), j<W (word@+2) over the footprint,
   z-level loop clamped to z0+D max 8; per cell (linear
   template index (z·H+i)·W+j [shape verified]):
   TOT-mirror z-word [0x4796bc+0x1E·tile+2·z] := template
   bank ptr@type+0x46 word; seen byte 0x4796cc[tile+z] :=
   (bank@type+0x4A word == 0); DAT volume byte
   [0x4edd58-plane] := LOW BYTE of the bank@type+0x4A word.
   So scratch banks @+0x46/+0x4A are the SAVED UNDER-TERRAIN
   (the 7j.13 "4 scratch banks" now have their first two
   consumers; @+0x3E/+0x42 remain unread here [open]).
3. **The five-effect loop [verified 0x41ac10..0x41b73a]**:
   m = 0..4 (byte offset [esp+8] = 8m, exit at 0x28), entry =
   type+0x16+8m (abs 0x4dee08+78·id). Selector word@entry+0;
   `dec; cmp ax,8; ja` → only 1..9 dispatch (else skip),
   `jmp [table 0x41a870 + 4·(sel−1)]` — table entries
   (idx 0..8): 0x41ac77/0x41b298/0x41b3c1/0x41b4ea/0x41b613/
   0x41b1fe/0x41b11c/0x41af96/0x41ae53. Payload words:
   w2=word@entry+2 = x TILE offset, w4=word@entry+4 = y TILE
   offset, w6=word@entry+6 = z-level offset — all relative to
   the destroyed object's 0x46cbf4 record dwords +0/+4/+8
   (pinned 0x41a90f..0x41a936). Stage position = (w+base)·0x20
   (+ sub-tile dx/dy below; Q5: 0x10 = half tile, 0x20 = tile).
   FUN_00420608 stack args in push order (first push = param
   → +0x28, last push = delay → +0x24; callee `ret 8` —
   pinned by the case-6/7 [esp+0x10c]/[esp+0x114] save/reload
   pattern): param = the outer score flag (cases 1..5,8,9) or
   −1 (cases 6/7); delay = counter+m (cases 1,8,9 — the chain
   counter + entry index), 0 (2..7); case 8's shower adds
   (i>>3) and reads the pre-increment counter+2m slot.
4. **The 9-case debris-kind map [verified per body]**
   (counter = the [esp+0x10] chain counter; splashes =
   FUN_0041bd78 water-z probe (clamp 7) + FUN_00424355):

   | sel | body | kind | (dx,dy) Q5 | delay | extra effects |
   |---|---|---|---|---|---|
   | 1 | 0x41ac77 | 14 | (+0xF,+0xF) | ctr+m | FUN_0041a225(rec.x+w2, y+w4, z+w6, ctr+m) + 1+4 splashes @ (x,y) RandA&1 jitter |
   | 2 | 0x41b298 | 18 | (+0x10,+0x30) | 0 | 4× splash loop |
   | 3 | 0x41b3c1 | 17 | (+0x30,+0x10) | 0 | 4× splash loop |
   | 4 | 0x41b4ea | 16 | (+0x20,−0x10) | 0 | 4× splash loop |
   | 5 | 0x41b613 | 19 | (−0x20,0) | 0 | 4× splash loop |
   | 6 | 0x41b1fe | 10 | (+0x10,+0x20) | 0 | RandB&1 SFX (below) @ (x,y) |
   | 7 | 0x41b11c | 10 | (+0x20,+0x10) | 0 | RandB&1 SFX (below) @ (x,y) |
   | 8 | 0x41af96 | 14 ×(1+24) | (+0xF,+0xF) | ctr+m (+i>>3) | 24-iter shower: k14 AT THE WATER Z (probe), ±3-tile jitter (RandA&7−3) x/y, +RandA&3 z, each + splash + FUN_0041a225 |
   | 9 | 0x41ae53 | 20 | (+0xF,+0xF,+0xF) | ctr+m | 3×3 splash ring (x−1..x+1 × y−1..y+1), delay ctr+2+RandA&3 |

   Reading: sel 2..5 = four single-gib throws at different
   sub-tile bearings (k18/k17/k16/k19 — the 7j.11 seq-table
   kinds {80..104} = the big-chunk walks); 6/7 = quiet k10
   collapses + thud; 1/8/9 = watery demolitions (splash-heavy);
   8 = a 24-particle k14 demolition shower. The 7j.11 k14/k16..k20
   sites inside FUN_0041a894 are ALL this loop (0x41ace7/0x41b002/
   0x41b0dc case 1/8; 0x41b554/0x41b42b/0x41b302/0x41b67a cases
   4/3/2/5; 0x41aebf case 9) — the family geography closes.
5. **FUN_0041a225 = the 0x4cf638 EFFECTS-BANK stager — first
   producer of the MISSIONVIEW §5d "effects loop" bank
   [verified 0x41a225..0x41a310]**: args (x tile, y tile, z
   level); converts x/y<<5, z<<13; 12 iterations of the
   allocator FUN_0041a4cc (80 slots × 0x1E @0x4cf638 = 0x960 B
   — matching the 7j.1 boot-clear bound; free iff word@+0x18
   == 0; −1 = skip); per filled slot:
   +0x00 = ((x<<5)+RandB&0x1F)<<8 − 0x1000, +0x04 same for y
   (Q13 + jitter), +0x08 = z<<13 + 0xF00, +0x0C/+0x10 =
   (RandB&0x3F)<<7 − 0x1000 (±velocities), +0x14 = RandB&0x7FF
   + 0x1770 (ttl 6000..), +0x18 = word FUN_0041ec59(3) (the
   ACTIVE/sprite word — 0 = free), +0x1C = word RandB()&7.
   FUN_0041ec59(3)'s exact role open [census]; callers today:
   destroy-tail cases 1/8 only.
6. **The case-6/7 SFX pair = DEADMAN1/DEADMAN2 [verified]**:
   RandB()&1 → FUN_0043a48e(bank 0x4edfb8 = SOUND\SFX\
   DEADMAN1.RAW / 0x4edfbc = SOUND\SFX\DEADMAN2.RAW, 0, x, y,
   push 2). Bank slots are .bss pointers filled by the
   sequential name loader 0x43a29b..0x43a368 (FUN_0043a39c =
   name→bank); strings 0x458f41/0x458f58 verified by direct
   DGROUP byte dump. The SAME pair is the crush SFX of the
   7j.24 debris-crush dispatcher FUN_0040dce0 (0x40dc62) —
   the pair is the shared destruction-thud family. (Loader
   order around them: …0x4edfa8←0x458f03, 0x4edfb0←0x458f19,
   0x4edfb4←0x458f2d = the MISSILE1/POWERUP/ELEV-adjacent
   run; full name walk left to the SFX unit.)
7. **The 160-vs-0xA8 stride anomaly at 0x4c69e4 RESOLVED —
   it was a census arithmetic slip, not a second array**
   [verified 0x40fe9e..0x40feb6]: FUN_0040fe93 computes its
   record offset as `shl eax,2; add esi; shl eax,2; add esi`
   = 21·idx (NOT 20·idx), then loads `[21·idx·8 + 0x4c69e4]`
   — stride 21·8 = 168 = **0xA8, the canonical robot stride**.
   The 7j.13 "20·i << 3 = 160" gloss dropped the second
   `add eax,esi`. FUN_0040fe93 body re-anchored: robot idx arg
   → x/y = dwords +0/+4 >>13 (tiles), z = dword +8 >>5;
   FUN_0041eb4c type-DB byte == 0x62 ∧ grid word ≠ 0 →
   FUN_0041a894(tile·0x2000, ctr 0, damage 100, no score);
   destroyed → 5× k12 debris at (x·0x20+RandA&0x1F,
   y·0x20+RandA&0xF, z·0x20+0x10+RandA&0x1F), delays
   0/2/4/6/8, param −1. Sole callers [verified census]:
   FUN_0040fe93 ← robots()/FUN_0040b9f6 @0x40bc44 (the
   phase-1 walk); FUN_0040ff92 ← the critter controller
   FUN_00412f34 @0x413fd7 — both actors trigger floor traps.
8. **.BDG grammar CLOSED + corpus census [verified, python
   framing per the parse; 37/37 files EOF-exact]**: NO
   header — records start at offset 0. Per record: control
   u16; ≠1 → record is JUST 2 B (empty row); ==1 → W/H/D
   u16×3, hp i32, chain u16, type i32 (objective/score code),
   5×8B effect entries (selector u16 + x/y/z u16 tile
   offsets), then FOUR on-disk template banks of 2·W·H·D
   bytes each (the arena banks are FILE data, not
   runtime-built — refines 7j.13). The loader caps at 0x11A
   (282) records. Corpus: every file carries exactly 282
   records (10434 total, 7907 active) and my walk consumes
   every file to the last byte. Selector census (nonzero
   entries): sel1 ×11098, sel2 ×1490, sel3 ×1385, sel4 ×402,
   sel5 ×330, sel6 ×304, sel7 ×316, sel8 ×178, sel9 ×56 —
   ZERO out-of-range selectors (the dispatcher's 1..9 gate
   covers the whole shipped vocabulary). Type dword top:
   15/5/30/11/120/90/20/40/10/60/180/270; W/H/D mostly
   (1,1,1)..(1,1,4), max (3,3,3). Reading: sel 1 (plain k14
   puff + effects) is the default debris; sel 2/3 (k18/k17)
   the common single-gib variants; sel 4..9 rare specials.
9. **Corpus verdict: unchanged** — the effect loop needs a
   destroyed destructible object; the corpus gates destroy
   none (weapons/traps stay unwired engine-side). Engine
   seam: NONE (docs-only, D73).

## 7j.26 Amendment 2026-08-21 (worker 7658328a, the MISSIONVIEW
§5d DRAW TAILS — effects + platform consumer passes; docs-only,
D74; sources: exw-missionrender2/3.txt + exw-brfdrop.txt
(FUN_00401e39) + exw-simtail.txt (mover/tick/loader) + fresh
objdump of FUN_0041a225/FUN_0041a4cc 0x41a225..0x41a4f2)

The two last-undecoded §5d consumer passes of the FUN_00403938
render tail, closing the 7j.25 queue item:

1. **The effects loop consumer (0x4cf638) DECODED** [verified asm
   0x406c86..0x406d60]: draws every record with u16@+0x18 != 0 ∧
   u16@+0x1A == 0 via the DIRECT blit FUN_00401e39 — img =
   u16@+0x16 * 8 + (u16@+0x1C & 7) (DEBRIS.BIN images 0..23), the
   frame counter u16@+0x1C is incremented IN THE DRAW, bank ESI =
   [0x4eddb4] = DEBRIS.BIN, dest EDI = [0x4ede18] (640×640
   backbuffer, row stride 0x280). sy base 0x100 (−0xC vs the
   robot loop) + the SECOND shake table 0x454518 (robots use
   0x45450c; both indexed by the DAT_0046cce4 quake countdown);
   z@+0x08 is Q13 (px = z>>8). Clip 0≤sx<0x23f, 0≤sy<0x23e.
2. **The 7j.25 producer field map CORRECTED** [verified objdump
   0x41a225..0x41a319]: the "+0x14 ttl RandB&0x7FF+0x1770" is the
   **vz** (rising, 6000..12069/frame) — its high word u16@+0x16
   (0..2) IS the sprite group (the producer never writes +0x16
   separately); "+0x18 = FUN_0041ec59(3)" is the ACTIVE word only
   (≈8% stillborn, freed by the next alloc scan); **u16@+0x1A =
   the producer's 4th register arg ECX = SPAWN DELAY** (the
   destroy-tail delay counter), decremented by the mover before
   any physics/draw. Full map in RE-EXW-MISSIONVIEW §5e.
3. **The mover FUN_00419f62 pinned** (MissionShell tick call
   0x44813d): delayed → +0x1A−−; else x+=vx, y+=vy, z+=vz, kill
   (+0x18 := 0) iff x/y/z < 0 ∨ x>>13 ≥ [0x4eddec] ∨ y>>13 ≥
   [0x4eddf0] ∨ z>>13 > 0xB — rising sparks die at the z=12
   ceiling in ~8..16 ticks. FUN_0041a4cc = plain first-fit scan
   for +0x18 == 0 (the "12 tries" is the caller's spawn loop).
4. **FUN_0041ec59 identity PINNED** [verified decomp 0x41ec59]:
   `RandB() / (0x8000/n − 1)` clamped to n−1 — a bounded-uniform
   random helper on the 15-bit RandB. In the effects producer it
   only arms the active word (value never read otherwise).
5. **The platform loop consumer (0x4eb638) DECODED** [verified
   decomp 0x4067a1..0x406832]: draws via the ENQUEUE path
   FUN_0040798e (not the direct blit!) with bank **DAT_0046af54 =
   GAMEGFX\SMOKER.BIN** (stager FUN_0041df10 @0x41dfb1): base =
   SMOKER frame 0, mode 300 at (sx, sy); smoke column = SMOKER
   frame d@+0x10+1, **mode 0x12d (DARKPAL flush)** at sy−0x20;
   enqueue coords (px+0xb, py+0xb), layer z>>5 — the exact §5d
   robot-loop form (z@+0x08 raw Q5). The anim tick FUN_004238af
   (MissionShell call 0x447fff) cycles d@+0x10: ++ and wrap
   0x10→4 — drawn column sequence 2..16 intro then 5..16 loop;
   the claim word d@+0x0C never clears (slot reuse = the 7j.24
   MIN-age allocator). So the "platform" records ARE the robot-
   death blast: ground puff + darkening smoke column.
6. **The FUN_00401e39 direct draw_IMG codec DECODED** [verified
   decomp+asm 0x401e39..0x401f83; 8street `draw_IMG_in_buffer`
   now re-anchored]: same .BIN container as the enqueue path but
   with the layout CORPUS-VERIFIED: **u16 count at word0, int32
   dir at bank+2+4*img, offset relative to its own slot** (asm
   0x401e40 = 4·img+2; 24/24 DEBRIS + 160/160 DANTE images parse,
   every RLE stream consumes exactly to the next image); hdr u16
   flags {bit1 → two s16 hotspot words, order (yoff,xoff); bit0 =
   RLE}, then u16 w, u16 h. Plain consumer: arg2 = 0/≠0 opaque/
   transparent flag, dest EDI + y*0x280 + x, NO palette modes.
   RLE control words: bit15 = skip run (word&0xFFF) — painted as
   ZERO bytes when opaque; else literal raw copy (no per-byte
   zero test); bit14 = EOL. Uncoded: plain copy vs per-byte
   zero-skip. Byte-granular transparency ONLY via RLE skips
   (coded) or zero-skip (uncoded) — same rule as the §5 flush
   codec. Callers: render tail ×4 (0x406d56/0x406eee/0x407077/
   0x4071ce), map overlay FUN_004089b1, boot/attract + title/
   menus — the game's general UI/direct blitter. Bank counts
   pinned: DEBRIS 24 imgs, SMOKER 17 (= blast frames 0..16),
   DROPSHIP 210 (64×64 tiles for the 7×7 ring grids; many 0×0
   stubs skipped instantly by the w/h==0 guard).
7. **BONUS context — the three DROPSHIP ring passes** (same asm
   block, recorded for the pod-descent/P4.2 work): per-robot bank
   0x4e64c0 (robot-count bound) + 6 standalone rings
   0x4e6610..0x4e66b8 (drawn as 1 + 5), records 0x1C {active d@+0,
   x d@+8, y d@+0xC, alt d@+0x10, img-group d@+0x14}; each active
   ring draws a 7×7 grid of 0x40-stride tiles, img =
   group*0x23 + 7*row + col, bank [0x4edd64] = **GAMEGFX\
   DROPSHIP.BIN** (ArenaAlloc(0x25990)); sx/sy bases 0x90/0xd0;
   robot-indexed sy subtracts the robot z as well. Bank
   geography: 12×0x1C robot rings + 6×0x1C standalone = ends
   exactly at the trail-ring bank 0x4e66b8 (7j.22/23). Ring
   producers (pod-descent stagger) remain open. **Bonus: the
   [0x4ede24/0x4ede28] backlog "7×7 screen-address table"
   re-pinned as the terrain RESTAMP list** [verified decomp
   0x406a8c..0x406c73]: count + 3-dword records {dest row,
   tile-x, tile-y} blitted through FUN_00401471 (border tile
   FUN_00408030 outside the window, full LNK path inside);
   writers = FUN_00440a2d (TOT-mirror materializer — so it IS
   the scroll/camera restamp stager), FUN_0043d00b, FUN_0041d954.
   Also noted: an open state-machine pass at 0x4c71f4 (states
   <0x13, splash/screen-effect sequences) between the platform
   and effects loops.
8. **Corpus verdict: unchanged** — both passes consume records
   whose producers sit off the corpus path (no deaths, no
   destroy-tail, no pod descent in the crop gates). Engine seam:
   NONE (docs-only, D74).

## 7j.27. The DROPSHIP ring PRODUCERS + pod-descent family CLOSED
(2026-08-22, worker e635cb76, claim 1 — docs-only, D75; sources:
the 7j.19 decompile dumps exw-exitfamily.txt (FUN_0041fbb1) +
exw-exitfamily2.txt (FUN_0040b9f6) + exw-exitfamily3.txt
(FUN_0041faf0/FUN_0041fb4b) re-read against a fresh FULL .text
objdump `ghidra-project/exw-text-objdump.txt` (0x401000..0x460000,
absolute-operand census of 0x4e64c0..0x4e66dc); no Ghidra run
needed — all sites [verified] against objdump)

Closes the 7j.26 queue item: the writer census for the ring banks
is COMPLETE (every absolute/displacement reference accounted), the
per-tick animator write map is decoded, and the 7j.26 consumer
gloss "7×7 grid" is corrected to the true 5×7 tile grid.

1. **The ring-record writer census** [verified objdump — refs to
   0x4e64c0..0x4e66b8 are exactly these; the apparent extra site
   0x40ab21 `[esi+0x4e6658],1` is the 7j.23 mortar trail-ring
   writer's displacement form (base 0x4e66b8−0x60), not a ring
   writer]:
   - **POD BANK 0x4e64c0 (12 × 0x1C, robot-indexed)**:
     RESET = FUN_0040cca0 @0x40cd3d — memset-0 of 0x150 (12×0x1C)
     at every mission spawn, immediately after the 0x7e0 robot-bank
     clear; SPAWN = FUN_0041fb4b(idx); ANIMATE = FUN_0041fbb1
     machine 3. The trigger chain: FUN_0040b9f6 decrements the
     w@+0x2C drop-pod countdown per robot per sub-tick (7j.20) →
     0-hit → FUN_0041fb4b(idx) + msgs FUN_004239ef(9/10/0xB, 0/1/2)
     for the player's first three robots (idx == selected/selected
     +1/selected+2, gated by DAT_0046cbd8) [verified decompile
     0x4c6a10 branch + asm].
   - **DROPSHIP 0x4e6610 (1 × 0x1C)**: RESET = MissionShell
     @0x447a7e (memset 0x1C); SPAWN = FUN_0041faf0; ANIMATE =
     FUN_0041fbb1 machine 2; readers = renderer 0x40707e +
     MissionShell spawn check 0x44831c.
   - **EXIT SLOTS 0x4e662c (5 × 0x1C)**: RESET = MissionShell
     @0x447a8d (memset 0x8C); SPAWN = FUN_0041fa51 (7j.18);
     ANIMATE = FUN_0041fbb1 machine 1; **NEW writer:
     FUN_00412a98 @0x412b60 — the POI-rescue path stamps
     dwell(+0x18) := 0** when a personnel POI escapes ([0x4eba0c]++
     + SFX 0x4edfa8 + FUN_00448b80(5000) in the same block):
     the landed elevator's 0x78-tick dwell RESTARTS per rescue, so
     one elevator can ferry multiple POIs. Readers = renderer
     0x406f1f, POI flee phase==2 gates 0x412ae2/0x412b94,
     nearest-exit scan FUN_00417c64 @0x417c90.
2. **The stamp field maps (all three spawners)** [verified decomp]:
   - FUN_0041faf0 (dropship): {active=1, phase=1, img-group=0,
     alt=0x200, x=beacon.x<<5, y=beacon.y<<5} + clears beacon
     0x4eabb0/0x4eabb2 (the x/y tile words 0x4eabb4/6 SURVIVE —
     see item 4).
   - FUN_0041fb4b(idx) (pod): {1, 1, img-group=0, alt=0x400,
     x=robot.x>>8, y=robot.y>>8} — Q13 → Q5 pixel coords.
   - FUN_0041fa51 (exit, 7j.18): {1, 1, pad.x·0x20+0xF,
     pad.y·0x20+0xF, alt=0x400, img-group=0}.
3. **The animator per-tick write map** (FUN_0041fbb1, MissionShell
   @0x448012 per FRAME — shared by all three machines over the
   0x1C frame {active@+0, phase@+4, x@+8, y@+0xC, alt@+0x10,
   img-group@+0x14, dwell@+0x18}) — **the 7j.19 "+0x14 toggle"
   gloss is superseded: +0x14 is the IMG-GROUP selector of the
   7j.26 consumer (img = group·0x23 + 7·row + col over
   DROPSHIP.BIN's 210 = 6×35 images)** [verified decompile +
   asm 0x41fbc1..0x41fecc]:
   - phase 1 DESCEND: img-group := (img-group+1)&1 EVERY TICK
     (a 2-frame animation — groups 0/1); alt := alt−0x20 while
     alt ≥ 0x101, else alt := (alt>>2)·3; alt < 1 → alt := 0,
     phase := 2 (exits also dwell := 0; dropship dwell := 10 AND
     fires the 7j.19 extraction sweep: robots alive ∧ state ∈
     {3,4} → state 5, timer 0x28, [0x4dc680]++).
   - phase 2 LANDED: alt := ((RandA()&7)==0) — a 0/1-px vertical
     jitter; img-group toggles 0↔1; exits: dwell++ > 0x78 →
     phase 3; dropship: dwell−− == 0 → phase 3; **pods: phase 2
     lasts exactly ONE TICK** — it fires the 7j.19 POD PAYOUT
     (robot state := 6 = RELEASED from the pod, timer 0x28,
     alive := 1, points := 100·w@+0x94+5000, SFX 0x4edfe0, msgs)
     and immediately sets phase := 3 (no dwell use).
   - phase 3 DEPART: alt := alt + (alt>>2) + 1 (accelerating
     rise); **x −= img-group·4** (leftward drift scaled by frame);
     img-group := (img-group < 5) ? img-group+1 : 4 — ramps
     2,3,4,5 then oscillates 4↔5; alt > 0x200 → active := 0
     (dropship also _DAT_004dc67c := 1 = extraction complete).
     Net: ALL SIX DROPSHIP.BIN groups are reachable — 0/1 =
     descent/landed flicker, 2..5 = departure frames.
   - Timing (for the P4.2 harness): descent from alt 0x400 ≈ 24
     frames (−0x20) + ≈ 17 frames (×0.75 shrink) ≈ 41 frames;
     dropship 0x200 ≈ 25 frames; depart ≈ 45 frames to 0x201.
     Robots are brain-frozen (w@+0x2C, 7j.20) until their pod's
     one-tick phase 2 releases them — the stagger step is
     1+k·(2000−m·1000/27) SUB-TICKS ≈ 173..327 frames between
     successive pods, so pods overlap in the air.
4. **The no-extract latch 0x46aed4 (machine-3 gate) census
   completed** [verified objdump]: boot RESET = GameMain
   @0x41c408 (memset 0x30 = 12 dwords) — NOT per-mission; set by
   FUN_0040e230 (SP death core), FUN_00449c94/FUN_0044a38a (MP),
   FUN_00408e99. **FUN_0040e230's MP respawn branch @0x40e7a1 is
   itself gated by the latch (≠0 → skip respawn)** — the latch is
   a per-robot "no more pods" flag: it freezes a mid-flight pod
   record (the animator skips it) AND refuses the MP re-drop.
5. **The 7j.26 ring-grid gloss CORRECTED** [verified asm
   0x40707e..0x4071d5 + pods 0x406dc6..0x406ec5]: the tile grid is
   **7 COLUMNS × 5 ROWS of 0x40-px tiles** (448×320 px), not 7×7 —
   col loop 0..6 (col += 0x40), row loop until sy−0xa0+0x140, row
   word += 7 per row; img = group·0x23 + 7·row + col with 0x23 =
   35 = 7·5 EXACTLY one group per ring frame. sx = (dx−dy)+0x90,
   sy = ((dx+dy)>>1)+0xd0+shake−alt; the dropship sy ALSO
   subtracts word@0x4eabb8 (the beacon z) — always 0 (the armer
   never writes it), so the "dead store" 7j.20 note stands but a
   reader exists at 0x4070c0. Pods' sy additionally subtracts the
   robot's own z d@+0x08 (the pod shadow tracks the robot's
   elevation); blit = FUN_00401e39(img, transp=1, sx, sy, bank
   [0x4edd64] DROPSHIP.BIN, dest [0x4ede18]).
6. **The 0x4c71f4 "state-machine pass" head-decoded** (the 7j.26
   open note, bounded add-on) [verified asm 0x404131..0x404182 +
   0x404d27..0x404d60]: it is the **PROJECTILE MID-FLIGHT DRAW
   dispatch** inside FUN_00403938 — `ax = type word@+0` of the
   400×0x36 weapon-anim bank, switch: 5 → 0x404187 (shell,
   iso-projection of x/y@+0x12/+0x16 Q13), 9..0xB → 0x404567
   (artillery burst), 0xE → 0x40436e (mortar), 0xF/0x13 → 0x4042a3
   (damped ballistic family), 0x17 → 0x404d08-side (3-clone
   split), 0x24 → 0x40464e (rocket), 0x29 → 0x404916 (homing),
   2..4/6..8/0xC..0x12 → generic 0x40427a. A sibling dispatch at
   0x404d65 walks the 50×0x22 projectile bank 0x4cc654
   (FUN_00412010's, weapon-id-keyed) with states 0x65..0x69 →
   jump table 0x403908 {0x404eb1, 0x404f8a(skip/next),
   0x404fac, 0x404ffc, 0x404d96} — the "splash/screen-effect
   sequences" of the 7j.26 note are these per-type draw bodies;
   full per-type math stays queued (bounded, with the trail-ring
   draw 0x404464 consumer).
7. Corpus-path verdict: docs-only (D75) — no engine change; the
   pod-descent family stays unwired in the gates (no pods deploy
   in the crop corpus), but the whole deploy→descend→release→
   depart state machine is now anchored for the P4.2 differential
   harness, which must model it (first seconds of every mission).

## 7j.28. The PROJECTILE MID-FLIGHT DRAW family CLOSED
(2026-08-22, worker ffec42cf, claim 1 — docs-only, D76; sources:
the full-.text objdump `ghidra-project/exw-text-objdump.txt` only (an
analyzeHeadless was running — no Ghidra per discipline), re-read of the
§7j.27 item 6 head decode, + corpus .BIN header/count verification of
the newly-named banks. All facts [verified asm] unless tagged.)

Closes the 7j.27 queue item: the last undecoded consumer block of the
FUN_00403938 render tail — the 400×0x36 weapon-anim bank's per-type
draw bodies, the 0x4e66b8 trail-ring draw consumer (the §7j.22 item 10
/ §7j.23 item 8 open point), and the sibling 50×0x22 walk (part 2
below). Part 1 = the 400×0x36 dispatch family.

1. **The dispatch, fully mapped** [verified asm 0x404131..0x404182 +
   0x404d08..0x404d60]: loop `[esp+0xa8]` = record OFFSET 0..0x5460
   (= 400·0x36, `add 0x36` at 0x404281, end-check `cmp 0x5460` at
   0x40428b → falls into the 50×0x22 walk at 0x404d65); `ax` = type
   word@[0x4c71f4+off]. Primary chain: <0x13 → secondary 0x404d27;
   ==0x13 → damped base 0x20; <0x1F → secondary 0x404d08; ==0x1F →
   damped base 0x18; <0x24 → 0x40427a; ==0x24 → rocket; ==0x29 →
   homing; else → 0x40427a. Secondary 0x404d27 (types 0..0x12):
   5 → shell 0x404187; 9..0xB → artillery 0x404567; 0xE → mortar
   0x40436e; 0xF → damped base 0x20 (0x4042a3); else → 0x40427a.
   Secondary 0x404d08 (types 0x14..0x1E): 0x17 → damped base 0x28
   (0x4042aa); 0x1A → damped base 0x18 (0x40429c); else → 0x40427a.
   **CORRECTION of the 7j.27 item 6 gloss: 0x40427a is the shared
   LOOP-NEXT (advance offset / next record), NOT a "generic draw
   body"** — types 0..4, 6..8, 0xC, 0xD, 0x10..0x12, 0x14..0x16,
   0x18, 0x19, 0x1B..0x1E, 0x20..0x23, 0x2A+ have NO mid-flight draw
   (hitscan/instant or drawn only at effect stage). **And 0x17 is
   NOT a "3-clone split" here — it is a damped-ballistic DRAW variant
   (base 0x28); the 3-clone split is tick-side only (§7j.13).**
   Damped family total: 0xF/0x13 → WEAPONS frame base 0x20; 0x17 →
   base 0x28; 0x1A/0x1F → base 0x18.
2. **The sprite banks, NAMED + corpus-verified** [verified objdump
   writer census + string reads + corpus headers]: boot loader
   (GameMain arena block 0x41d9dc..0x41da54 + FUN_0041cc7f loads
   0x41df59..0x41dfb4):
   - **[0x4eddbc] = GAMEGFX\WEAPONS.BIN** (ArenaAlloc 0x5208; corpus
     file 0x4F86 bytes, header count 70 imgs) — ALL of shell /
     artillery / mortar / damped bodies + the mortar smoke puffs.
   - **[0x46af30] = GAMEGFX\SHRIKE.BIN** (0x1F40; 0x1EFF, **exactly
     64 imgs**) — the rocket body, frame = ((dir+0x7E)&0xFF)>>2 ∈
     0..63 (64-direction rotation).
   - **[0x46af2c] = GAMEGFX\REAPER.BIN** (0x1770; 0x1428, **64
     imgs**) — the homing body, frame = (dir&0xFF)>>2 (no bias).
   - **[0x46af34] = GAMEGFX\SMOKE.BIN** (0x7D0; 0x676, **exactly 4
     imgs**) — rocket + homing exhaust puffs, frame min(i>>1,3).
   - **[0x4edd7c] = GAMEGFX\GENERAL.BIN** (0x1F7E8; 0x1F73A, 153
     imgs) — the homing TARGET-LOCK RETICLE (item 5) + the whole
     0x407xxx sidebar/UI reader family (~20 read sites).
   - (context pins: [0x46af38] TELEPORT.BIN 10 imgs = the §7j.21
     arrival marker; [0x46af3c] NUMBERS.BIN; [0x46af40] FLAGS.BIN —
     the boot loader string block 0x45884e..0x4588c3 in order.)
3. **Shell (type 5, 0x404187)** [verified]: iso project (below);
   frame = d@+0x0E counter ++ per DRAW (0x404202), wraps 7 → 3 —
   cycles 3..7; bank WEAPONS, mode 0x12C; call tail 0x404270 shared
   with artillery.
4. **Artillery (9..0xB, 0x404567)**: same projection; frame =
   8 + (d@+0x0E++)&7 — cycles 8..15 (the second 8-frame strip);
   bank WEAPONS, mode 0x12C. Mortar (0xE, 0x40436e): STATIC frame
   1, bank WEAPONS, mode 0x12C, then the trail loop (item 6).
5. **Damped family (0xF/0x13/0x17/0x1A/0x1F, 0x4042af)**: frame =
   base (0x18/0x20/0x28 by type, item 1) + wobble: iff vx d@+0x1E ≠ 0
   AND (|vx|>0x40 ∨ |vy|>0x40) → += [0x46ae68]&7 (the global tick
   counter — animated while flying fast, static when slow/at rest);
   bank WEAPONS, mode 0x12C; sy anchor 0x108 (not 0x110).
   **Homing (0x29, 0x404916)**: REAPER frame (dir)>>2, mode 0x12C;
   then the RETICLE: target word d@+6 — 0 → none; bit 0x1000 →
   robot idx (w&0xFFF)−1 @0x4c69e4+idx·0xA8 {x,y d@+0/+4, z d@+8<<8};
   bit 0x2000 → critter idx w&0x1FFF @0x4cccec+idx·0x20 {x,y d@+0/+4
   <<13 +0x2000, z d@+8<<13 −0x800}; else FUN_004128ec(w−1, &x,&y,&z)
   out-lookup (the third target class — TRT/structure, per §7j.13
   0x29 target sel). Reticle drawn at target+(2,2) px, sy anchor
   0xF0 (floats above), frame = [0x46ae68]/3+2, bank GENERAL,
   mode 0x12C.
6. **The trail-ring draw consumer CLOSED** (the §7j.22 item 10
   open point) [verified asm 0x40442f..0x404562]: after the mortar
   body, loop i = 0..7, pos = **0x4e66b8 + link(d@+0x32)·0x68 + 8 +
   i·0xC** {x,y,z Q13} — i.e. the 8 ring positions at record +8
   (past {active@+0, ring@+4}); each projected iso (frame =
   **0x10 + ([0x46ae68]+i)&7** — the 16..23 WEAPONS strip, mode
   **0x12E**), per-puff screen bounds 0..0x23F/0..0x23E AND map
   bounds (dx < [0x4eddec]<<5, dy < [0x4eddf0]<<5 — the only body
   family that map-clips). The slot's active/ring words are NOT
   read — all 8 positions always drawn (the producer ring-zeroes
   them, §7j.22). Skip-any → continue loop.
7. **Rocket (0x24) + exhaust (0x40464e + 0x404717)**: SHRIKE body
   frame ((dir+0x7E)&0xFF)>>2, mode 0x12C; then up to 8 exhaust
   puffs at dist 0x20+0x10·i along **dir+0x7E** (≈ opposite travel —
   behind; FUN_0041eb65/77 = **COS/SIN byte-angle Q15** lookups of
   table [0x46cbd0], ×dist >>15 <<8 added to x/y then >>8), count
   gated by **d@+0xA (the tick/TTL) /4 > i** (the §7j.13 "+0xA
   tick" — TTL quartiles: the trail shortens as the rocket ages),
   each ±(RandA&3−2) px jitter, frame min(i>>1,3), bank SMOKE,
   mode 0x12D, screen+map bounds. Homing's variant: 4 puffs, dist
   0x10+0x08·i, dir+0x80, same bank/frame/mode (0x404b22 loop).
8. Shared math: dx = (x d@+0x12 >>8) − [0x4edde4] (camX Q5), dy =
   (y d@+0x16 >>8) − [0x4edde8]; sx = dx−dy+0x110+[esp+0x34],
   sy = (dx+dy)>>1 + anchor(0x110; damped 0x108; reticle 0xF0) +
   [esp+0x38] − (z d@+0x1A >>8) + [esp+0x48] (the renderer's
   scroll/shake offsets); bounds sx∈[0,0x23F], sy∈[0,0x23E].
   Draw call shape: **FUN_0040798e(EAX sx, EBX bank, ECX dx, EDX
   sy, stack: dy, frame, z>>13 tiles, mode 0x12C/0x12D/0x12E)**
   (7j.26's mode words; the 7j.21 "sprite 0x12E" gloss for the
   marker was this same 4th stack arg = MODE).
9. **The sibling 50×0x22 walk (part 2)** [verified asm
   0x404d65..0x4050d4 + jump-table read from the binary]: entered
   when the 400×0x36 loop's offset hits 0x5460 (0x40428b). Walks
   offsets 0..0x6A4 (= 50·0x22); type word@[0x4cc654+off];
   `−0x65 cmp 4 ja skip` → jump table **0x403908** (read from
   file: 0x404eb1 / 0x404f8a / 0x404fac / 0x404ffc / 0x404d96):
   - **0x65 → 0x404eb1**: single sprite, iso with z d@+0xA as Q13
     (sy −= z>>8; z-arg z>>13 clamp ≥0), WEAPONS frame
     **(g_frame_count&3)+0x3C** (60..63), mode 0x12C, +shake.
   - **0x66 → 0x404f8a = LOOP-NEXT: NOT drawn mid-flight** (the
     heavy (d+1)·300 TRT bolt is invisible — 7j.16).
   - **0x67 → 0x404fac**: enters the 0x404eb1 tail — draw IDENTICAL
     to 0x65 (frames 0x3C..0x3F, mode 0x12C).
   - **0x68 → 0x404ffc**: same projection (z Q13, shake), frame
     **(g_frame_count&3)+0x38** (56..59), mode 0x12C.
   - **0x69 → 0x404d96 — the vertical BEAM column** (the §7j.22/23
     open-item type): NO shake term; sy base −= (d@+0xA<<5)+8 —
     **+0xA re-used as the TOP z LEVEL** (not Q13); loop edi from
     d@+0xA DOWN TO d@+0x1A (the bottom level), one sprite per
     level, **sy += 0x20 per level**; frame
     **0x34+((g_frame_count+edi)&3)** (52..55, animated down the
     column), z-arg = current level (clamped ≥0), bank WEAPONS,
     mode **0x12E**; per-level screen bounds; skip-any → continue.
   Shared: dx = (x d@+2 >>8)−[0x4edde4], dy = (y d@+6 >>8)−
   [0x4edde8]; the call's x/y args read d@+2/d@+6 directly; fields
   +0x12/+0x16 (vx/vy per the tick) are NOT read by any draw body.
   Loop-next 0x404f8a: offset += 0x22; == 0x6A4 → the render tail
   continues at **0x4050d5** (next stage, beyond this unit).
10. **The 7j.27 "splash/screen-effect sequences" gloss RESOLVED**:
    the 0x403908 bodies are the four WEAPONS.BIN anim strips
    0x34..0x37 (beam column), 0x38..0x3B (0x68), 0x3C..0x3F
    (0x65/0x67) — the bank's 70 images cover strips 0..7 (damped
    wobble 0x18..0x2F at bases 0x18/0x20/0x28), 1 & 3..7 (shell),
    8..15 (artillery), 0x10..0x17 (mortar puffs), 0x34..0x3F (the
    0x4cc654 family) with room to spare. Corpus counts: WEAPONS 70,
    SHRIKE 64, REAPER 64, SMOKE 4 imgs — all exact-consumption
    arenas (§7j.26 FORMATS §18 pattern).
11. Corpus-path verdict: docs-only (D76) — no engine change; the
    whole mid-flight draw family (both banks) now lands with the
    P4.2 differential harness, which can watch the WEAPONS/SHRIKE/
    REAPER/SMOKE blit sequences directly.


## 7j.29. The ".MOFO loader" RETIRED — string-tail misparse; FUN_00415490 = the mode-9 SEEK acquisition dispatcher (2026-08-22, worker 0a08a5e1 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run)

**Headline: there is NO .MOFO loader and NO .MOFO file format.**
The queue premise traced to a misparse of DGROUP 0x457a3c..0x457a6d
(bytes re-read from BEDLAM.EXW @file-off 0x5603c):

```
0x457a3c  "Buggered direction in MOFO\0"   (the ONLY string here)
0x457a57  ".NME\0"   0x457a5c ".TRT\0"   0x457a61  4x00 pad
0x457a64  ".POS\0"   0x457a69 ".BDG\0"
```

0x457a4c = "MOFO\0" is the dead TAIL of the fatal-message string,
not a 5th sibling tag. Evidence [verified]:
- ZERO code references to 0x457a4c across .text (objdump immediate
  scan of the full 0x401000..0x460000 dump; the earlier Ghidra XREF
  probe block exw-critterpoi-xrefs.txt also returned empty for every
  offset probed in this string block).
- The byte sequence ".MOFO" exists in NEITHER BEDLAM.EXW NOR
  BEDLAM.EXD; no *.MOFO file exists anywhere in game-data (corpus
  walked read-only; MANIFEST.sha256 verified clean before/after).
- The four real tags carry EXACTLY ONE reference each, at the four
  already-closed loaders: .NME@0x457a57 → 0x41648c (FUN_00416458,
  §7j.18), .TRT@0x457a5c → 0x4170c3 (FUN_004170a6, §7j.15),
  .POS@0x457a64 → 0x41a55d and .BDG@0x457a69 → 0x41a5d6 (both
  FUN_0041a4f8, §7j.25). The extension-tag family at this
  dispatcher is CLOSED at four members; the 7j.15 gloss "section
  strings .MOFO/.NME/.TRT/.POS/.BDG at 0x457a4c..0x457a65" is
  corrected to ".NME/.TRT/.POS/.BDG @0x457a57..0x457a6d".
- EXD twin: "Buggered direction in MOFO" @file-off 0x9d86c — the
  message ships in both binaries.

**The string's sole consumer decoded — FUN_00415490(idx) = the
mode-9 SEEK per-step target-acquisition dispatcher** [verified]:

- Prologue @0x415496: ecx = idx·0x7E (critter bank 0x4cff98 frame);
  eax = dword@+0x10; `cmp eax,3; ja 0x4156fe` (the FATAL);
  `jmp [eax·4+0x415480]` — table bytes re-read from the binary:
  {0x4154b6, 0x415549, 0x4155da, 0x41566b}, all landing on code.
- **+0x10 is DUAL-PURPOSE**: (a) the 8-bit wander HEADING (0..255)
  in the steer paths — 0x413495..0x4134bf: read, angle-clamp add
  (FUN_00412a19, §7j.17), `&0xFF`, stored back, then sin/cos
  FUN_0041eb65/FUN_0041eb77 `>>6` deltas add to +0x36/+0x3A; heading
  steers ±0x40 (0x413ec8..0x413efb); anim word w@+0x56 :=
  (heading&0x3F)+0x20 @0x414330; and (b) the 2-bit SEEK DIRECTION
  0..3 in mode 9 — the mode-9 entry writes it together with mode
  w@+0xC := 9: the 0xB-dormant wake path 0x4143b9..0x414421 sets
  `RandA()&3` @0x4143d5 (plus timer w@+0x6 := 0xC8, pause w@+0x2
  := 6, SFX [0x4edfe0] via the FUN_0043a48e family), the mode-2
  sub-state-4 retreat sets 9 @0x414534, and the re-picker writes
  either `eax&3` @0x414310 or FUN_004181bd's return @0x414320 into
  +0x10 (identity of FUN_004181bd otherwise open).
- The mode-9 walk loop dispatches the SAME dword through a SECOND
  4-way table @0x412ef8 = {0x414346, 0x41443b, 0x41446f, 0x4144a3}
  (`cmp 3 / ja skip` @0x4144e1 — non-fatal there): each body calls
  its axis stepper FUN_00417f2c (y−1) / FUN_00417fe8 (x+1) /
  FUN_004180c0 (y+1) / FUN_0041813d (x−1); step OK → move the
  critter one unit (dec/inc of +0x3A/+0x36) and CALL FUN_00415490;
  step blocked → anim w@+0x56 := 0, skip.
- **FUN_00415490's four cases** = directional forward-acquisition
  probes over the robot bank (base 0x4c69e4, stride 0xA8, active
  d@+0x7C, count [0x46ccbc] — the §7j.28 row's bank): the walk axis
  carries the one-sided tight window (−4..+0xF = target up to 0xF
  AHEAD), the crossing axis |Δ|<0x18, z |Δ|<0x18 raw. Case↔direction
  coherence (all four verified against their stepper):
  dir 0 (y−1): critter_y − robot_y>>8 ∈ (−4,0xF];
  dir 1 (x+1): robot_x>>8 − critter_x ∈ (−4,0xF];
  dir 2 (y+1): robot_y>>8 − critter_y ∈ (−4,0xF];
  dir 3 (x−1): critter_x − robot_x>>8 ∈ (−4,0xF] — with the
  crossing box reading robot y RAW (not >>8) — faithful quirk,
  unique to case 3 [hypothesis: oversight; scale left open].
  On hit: target-robot idx w@+0x7A := i, mode w@+0xC := 2
  (RANGE-ATTACK — §7j.17's mode-2 semantics), anim w@+0x56 := 0.
- **The FATAL path** (dword >3 = corrupted seek direction):
  0x4156fe `call 0x420100` (fade cancel, DESIGN-RENDER §6 / cancel
  semantics per DECISIONS D13) → `mov eax,0x457a3c; call 0x44d2ac`
  (console print: 0x44ffec lookup + buffered-putc 0x44f34b family,
  RE-EXW-TICK) → `mov eax,1; call 0x44d2da` = the documented FATAL
  EXIT (RE-EXW-GAMETHREAD 0x43d478 idiom: fn-pointer teardown then
  CRT exit, no return path) → jmp the walker's common tail
  0x4180b9. "Buggered direction in MOFO" is thus an internal
  assertion message; MOFO = the developers' codename for the
  critter subsystem.
- Sole caller: 0x414368 (inside FUN_00412f34's mode-9 walk). The
  §7j.17 ledger gloss "dominant-axis steppers ... each →
  FUN_00415490" is refined: the steppers are called BY the walk
  loop, and FUN_00415490 fires after each successful step.

Corpus-path verdict: docs-only — no engine change (mode 9 exists
only for .NME-seeded seekers; no corpus gate reaches a corrupted
direction, and the fatal is a crash path by construction).


## 7j.30. The SFX/GFX BANK-NAME WALK — the complete bank→name map + the FUN_0043a48e play family (2026-08-22, worker 7972b334 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt + DGROUP bytes re-read from BEDLAM.EXW, no Ghidra run)

Sec 9 item 5's DATA PREREQUISITE delivered: every durable
bank-pointer cell in 0x4edfXX/0x46afXX now carries its file name.
Method [verified]: two independent extractors over the full .text
objdump (a strict 3-instruction window matcher and the looser
predecessor state machine from the interrupted 09:55 WIP — adopted,
validated row-by-row, 100% agreement after window widening; the
single artifact row `BEEP5→0x46af0c` rejected as the staging-cell
false pair) + the DGROUP string table re-read from the binary
(PE: DGROUP VA 0x454000 = file 0x52600, section table verified
this run). 202 durable assignments, zero unnamed durable cells;
raw dump = ghidra-project/exw-banknames.txt, generators
/tmp/opencode/sfxwalk.py + /tmp/opencode/sfxcensus.py.

**The register/load idiom pair** [verified]:
- SFX: `mov eax,NAME; call FUN_0043a36e|FUN_0043a39c; mov ds:CELL,eax`
  — both callees share an identical head: stage the .RAW through the
  scratch cell 0x46af0c (`mov edx,[0x46af0c]; ecx=8; ebx=0x2b11;
  call 0x41cc7f` = the file→arena loader) then
  `0x44c64c(eax=[0x46af0c], edx=[0x4eded4], push 1)` returns the
  VOICE-BASE handle stored into the cell. FUN_0043a36e registers
  ONE voice; FUN_0043a39c registers FOUR (0x44c64c + 0x44c828 ×3,
  channel words edx=2..4) — Watcom clone pair, heads byte-identical.
  **The SFX cells hold voice-base handles, not data pointers.**
- GFX/PAL: `mov eax,NAME; mov edx,ds:CELL; call 0x41cc7f` (the
  0x41cc7f return value is the arena bank pointer; several sites
  reorder the edx read before the name load or interleave a
  `mov ecx,0x302`-style mode/stride arg — the 17 widened-window
  pairs all verified in-dump).
- **Language variants**: `cmp ds:0x4eba1c,1; jne` picks `NAMEG.BIN`
  vs `NAME.BIN` into the SAME cell (SENTRYG/SENTRY→0x4edda4,
  BIOMEX3G/BIOMEX3→0x4edda8, BLOWUPG/BLOWUP→0x4edd6c,
  BIOMEX1G/BIOMEX1→0x4edda0); 0x4eba1c = the parsed language INDEX
  (0x46cbb4 holds the LANGUAGE.GER content/handle, loaded
  0x41c1f5). Edition gate quirk @0x41d912: `[0x4edd8c] > 4` loads
  the GRILLA(G) family else the BIOMEX1(G) family — both into
  0x4edda0.

**FUN_0043a48e = the SFX PLAY/STEAL function** [verified, head]:
args (eax=voice-base handle, ebx=x, ecx=y, edi=[esp+0x1c]=age,
stack=priority-family); ebx=ecx=−1 → defaults vol 0x7f / pan
0x8000 (the whole-menu family passes −1,−1); else FUN_0043a3e0
(x,y → signed pan byte, via 0x41ebf8 angle field, clamp ±0x7ff0>>8)
and FUN_0043a447 (x,y → volume word, clamp ±0x7fff +0x8000 bias)
against the LISTENER cells 0x4edde4/0x4edde8 (x/y). Channel pick:
probe the base's 4 voices via 0x44c5ac (free query); none free →
steal: scan priority words `[0x4ee1c2+2·v]>>16` vs required and
age words `[0x4ee2e2+2·v]`, lowest/oldest wins; start via
0x44c904(base+slot); tail packs pan<<8 back into the arrays.
Voice-state census: 0x4ee1c2/0x4ee1c4 = ONE priority array (odd
base, dword read takes the high word), 0x4ee2e2/0x4ee2e4 = the
age/pan array, 0x4ee1c0 = reset-with-listener cell (0x4478b9,
mission-load reset block). Speech BYPASSES this path: indexed
record pick `mov eax,[eax*8+0x4ee014]` (A variant) /
`[eax+0x4ee018]` (B) then 0x44c8c4 direct play (vol 0x7f00) —
sites 0x423b85/0x423b57.

**The complete SFX map** (cell = file; all [verified] in-dump):

*Mission set — FUN_0043a1d3, the 27-register block 0x43a1d3..0x43a36d:*
0x4edf60 MIDIGUN, 0x4edf64 BOOM1, 0x4edf68 BOOM2, 0x4edf6c BOOM3,
0x4edf70 **MIDIGUN AGAIN** (duplicate file, distinct cell — quirk,
no consumer found for 0x4edf70), 0x4edf74 SQUISH2, 0x4edf78 SQUISH3,
0x4edf7c HURT1, 0x4edf80 HURT2, 0x4edf84 HURT3, 0x4edf88 DEATH1,
0x4edf8c DEATH2, 0x4edf90 DEATH3, 0x4edf94 PLASMA (readers
0x409273/0x40ab52/0x40b03b = the 7j.17 robot-fire family ✓),
0x4edf98 RICOCHT1, 0x4edf9c RICOCHT2, 0x4edfa0 RICOCHT3,
0x4edfa4 RICOCHT4, 0x4edfa8 POWERUP (10 readers = the 7h pickup
family ✓), 0x4edfac MISSILE1, 0x4edfb0 ELEV1, 0x4edfb4 ELEV2,
0x4edfb8 DEADMAN1, 0x4edfbc DEADMAN2 (7j.25 ✓), 0x4edfd0 TEXTBOX1,
0x4edfd8 BEEP5, 0x4edfdc BEEP5 (BEEP5 twice, distinct cells —
per-screen re-registration shares the file).

*Screen sets (re-registered per screen, same cells reused):*
0x4edfc0 MENU1 / 0x4edfc4 MENU2 (title 0x43a6d0 family, selector
0x43e973, debrief-aliased 0x4ee00c/0x4ee010 at 0x44448e/0x44449d,
read 0x44545f/0x4454b7); briefing 0x43d150: 0x4edfc8 BEEP1,
0x4edfcc BEEP4, 0x4edfd4 BEEP7 (+BEEP5/TEXTBOX1 shares above);
selector 0x43e946: + 0x4edfe8 DOOROPEN (read 0x43f003/0x43f094),
0x4edfec DOORCLSE (read 0x43f04c); shop 0x440f7e re-runs the
BEEP set; map/debrief 0x444436 adds nothing new.

*Mission-extra set — MissionShell block 0x447bb2..0x447c3b:*
0x4edfe0 BEAMIN (pod release — 7j.27 ✓, readers incl. 0x412dd7/
0x4136de/0x41376b the critter wake family), 0x4edfe4 THROW
(readers 0x409646/0x4098db/0x409b11 = robot fire w6/7/8 ✓),
0x4edff0 BIOFIRE (reader 0x413e4b), 0x4edff4 PEXPLODE (reader
0x421dc4), 0x4edff8 CACODETH (reader 0x418982 = k7 death ✓),
0x4edffc SQUAWK (reader 0x4152bd), 0x4ee000 GRUNT1 / 0x4ee004
GRUNT2 / 0x4ee008 GRUNT3 (readers 0x421ef2/0x421f01/0x421f17 =
the critter-hit SFX dispatcher family).

*Speech bank — boot loader block 0x41cf4a..0x41d4d6 (FUN called
from 0x41c2b4):* 8-byte records at 0x4ee014 = {A-variant handle
@+0, B-variant handle @+4}, record i @ 0x4ee014+8i, i = SPCH## :
SPCH00..SPCH14 A+B, SPCH15 A only, SPCH16..SPCH33 A+B, SPCH34 A
only, SPCH35..SPCH42 A only, SPCH43..SPCH51 A+B, SPCH52 A only
(95 files, 53 records; 11 unpopulated B-slots — the 10 mid-table
cells 0x4ee090/0x4ee128/0x4ee130/0x4ee138/0x4ee140/0x4ee148/
0x4ee150/0x4ee158/0x4ee160/0x4ee168 plus the record-52 tail
0x4ee1b8, all zero refs, never stored [verified]). Pair slot
order FLIPS at SPCH16: records 0..14 store A@+0/B@+4, records
16..33 and 43..51 store B@+0/A@+4 (verified at 0x41d111..0x41d12f:
SPCH16A→0x4ee098 = rec16+4, SPCH16B→0x4ee094 = rec16+0; same
at 17/33/43..48); singles always occupy +0. The head-level
readers pick SLOT, not variant — 0x423b85 reads +0, 0x423b57
reads +4 — so the +0 slot carries the A file for records 0..14
and the B file for records 16..51. Faithful quirk recorded
[verified]; whether any caller actually reaches records 16+
via these two sites is open.

**The complete GFX/PAL name map** (selection; full dump in
ghidra-project/exw-banknames.txt):
- 0x46afXX durable: 0x46af2c REAPER, 0x46af30 SHRIKE, 0x46af34
  SMOKE, 0x46af38 TELEPORT, 0x46af3c NUMBERS, 0x46af40 FLAGS,
  0x46af44 SHIELD, 0x46af48 ROBNUMS, 0x46af50 DIGITS, 0x46af54
  SMOKER (all cross-check 7j.26/7j.28 ✓).
- 0x46afXX unnamed-by-design (census): 0x46af0c = the universal
  LOAD STAGING cell (31 refs — every SFX/GFX load passes through
  it; boot-written 0x41c2ab, GFX-loader-copied to 0x46af20
  0x41d680); 0x46af4c = the DAT volume pointer (the §7j.17
  presence-mark formula bank — loaded from the mission .DAT, no
  fixed name); 0x46af58 = a 0x2710-B runtime arena (0x41db89
  alloc, readers 0x41f191/0x422931/0x423858 = beacon/scan
  family); 0x46af5c = struct-array base (address-taken only);
  0x46af04/08/10/14/1c/20/24 = loader bookkeeping + a MP state
  cell (0x46af08, written 0x449cab, cmp==1).
- Sprites: 0x4edd64 DROPSHIP (arena 0x25990, load 0x41d81c),
  0x4edd6c BLOWUP(+G), 0x4edd7c GENERAL, 0x4edd80 SCANNER,
  0x4edd84 SPIDER, 0x4edda0 BIOMEX1/GRILLA(+G), 0x4edda4 SENTRY(+G),
  0x4edda8 BIOMEX3(+G), 0x4eddbc WEAPONS, 0x4eddb0 VICERA,
  0x4eddb4 DEBRIS, 0x4ede2c DANTE, 0x4ede30 TERRA, 0x4ede7c SMLFONT,
  0x46cbbc TABLE, 0x46cbc4 FULLFONT (re-loaded 0x447798 MissionShell),
  0x46cbc8 HUMANS (boot 0x41d8cd + re-load 0x41e07e), 0x46cbcc
  IDIOTGFX, 0x46cbd0 SINTABLE, 0x46cdac MONOFONT, 0x46cdb0 TINYFONT,
  0x46cdb4 BRIEF; 0x4eddac CACO (arena 0x59d8 — the critter bank ✓).
- Palettes SHARE cells per screen role: 0x4edbf8 = current-screen
  palette (LOADPAL 0x41c88e boot, GAMEPAL, BRFPAL, SELECTOR,
  SHOPPAL, DB_PAL — one cell, six names, last-load-wins);
  0x4edbfc = TXPAL1/TXPAL2/TXPAL3; 0x4edc00 = DARKPAL/SELDARK/
  DARKPALS. Do NOT treat these as stable identities across screens.

Corpus-path verdict: docs-only, no engine change — the map is the
data prerequisite for the future mission-SFX tier (sec 9 backlog:
the tier itself remains unimplemented; MENU1/MENU2-style mixer
instruments stay out of the hashed core per §5 note).

Corrections: none needed — 7j.25's DEADMAN pair, 7j.17's fire
banks, 7h.2's POWERUP, 7j.27's BEAMIN all re-confirmed cell-exact.


| constant | value | anchor |
|---|---|---|
| robot record base/stride/count | 0x4c69e4 / 0xA8 / DAT_0046ccbc | 0x40c536, 0x40b9f6 |
| Q13 per tile | 0x2000 | 0x440cfe9 region, robots() |
| Q5 per tile | 0x20 | 0041e897 bounds, 0041e231 |
| spawn center offset | 0xF00 | FUN_0040cca0 |
| probe tables X/Y | 0x4543e4 / 0x454404 (±11/+12/0 set) | PE bytes |
| climb limit | 4 z-units | 0041e897 |
| per-sub-tick stride | 0x400 Q13 (1/8 tile) | 0x40bea7..bf06 |
| arrival radius | 0x1400 Q13 (~0.625 tile) | 0x40be1d |
| click order radius | 0xC0 Q5 (6 tiles) | 0x40c080 |
| order validity window | 0x197 frames | 0x4247d4 |
| sub-ticks per frame | 6 (phases 0..5) | MissionShell loop |
| angle helpers | dist=max+min/2; 64-sector table; fold 0..0x100 | 0x41ebf8/0x41eb7d/0x41ebc1 |
| facing codes | N=0x00 E=0x40 S=0x80 W=0xC0 none=0xFFFF | 0x40c692 et al. |
| anim phase | ((angle+4)&0xFF)>>3 (32 sectors) | 0x40c536 tail |
| robots per zone | <3|7→1, 3→2, else 3 | FUN_0040cca0 |
| map size globals | DAT_004eddec (w) / DAT_004eddf0 (h) tiles | 0041e897 |
| +0x18 fade | every frame, all w*h tiles, nonzero → −1 (FUN_00424051 head) | §7j.10 |
| debris kinds | 20-kind jump table 0x4205b8; +0x1C=kind, +0x20=physics class 0/1/2/3/6, +0x24←[esp+0x20] delay, +0x28←[esp+0x24] param; ring kinds 1(+13/14/15)/3/4/5/6+12/9/11/20 (nine 3×3), 2/8 single center 3/4, 7/10/16..19 none | §7j.11 |
| debris seq tables | 11 tables, i16 −1-terminated, DGROUP 0x454424..0x454510 (k5-family {5..16}; k2/k8 {0..4}; …{44..104} = k16..20) | §7j.11 |
| debris arrival SFX | FUN_00421e60 3-way (banks 0x4edf64/68/6c, push 2) · FUN_00421dec 4-way (0x4edf98/9c/a0/a4, push 1); both gated [0x4ede58]≠0 | §7j.11 |
| arrival ride tick | FUN_0042034c epilogue 0x448076: 45 rec @0x4dcdb8 stride 0x24 {active, marker xyz tile, dest xyz, countdown, robot slot}; walk STOPS at first inactive (contiguous run from rec 0); countdown 0 = dormant skip; ==0xA SFX bank 0x4edfe0 at marker; →0 teleport robot to dest + burn platform (both gate banks) + FUN_0042394a(x,y,z,0,0) water clear | §7j.11, §7j.21 |
| elevator stager | FUN_00425da4 (MissionShell boot @0x447b4e): clear 45 records then per-(zone [0x4edd8c] 1..7, mode [0x4edb88], mission [0x4edd88]) fixed-address staging; marker ← .PAD slot u16 x/y/z @0x4e44f8+slot·8+2; dest := immediates; +0x20:=−1; countdown never written (dormant); Z1 0..6, Z2/Z3 0..16, Z4 0..8, Z5 0..9, Z6 0..14, Z7 0..6 | §7j.21 |
| elevator ride armer | FUN_00433980 ride cases: guard +0x20≠−1; rider state@+0x0C:=2, pre-position at marker+0x1000, countdown:=10, +0x20:=rider; all armed countdowns = 10 | §7j.19, §7j.21 |
| arrival marker draw | FUN_00403938 tail 0x4065e5..0x4066e3: skip inactive/countdown-0; isometric marker tile; sprite 0x12E (FUN_0040798e, bank [0x46af38]) width clamp(11−countdown, 0, 9) | §7j.21 |
| memset-0 | FUN_00402965(EAX=0, ECX=len bytes, EDI=dst); 176 callers | §7j.21 |
| door-rect list boundary | 0x4dcae8..0x4dcdb8 = 45×0x10 door rects (0x2d0); MissionShell clears it @0x447b7b AFTER the stager — ends EXACTLY at the arrival base, no overlap; door consumers use idx 0..0x24 | §7j.21 |
| door open/close | FUN_004223b8(idx, state 1/2): rect {+0 state,+2 x0,+4 y0,+6 w,+8 h,+0xA variant} (§7j.34-corrected; the §7j.21 w/y/h permutation retired); state<3 only; anim-complete tile test low7(+0x1A)==+0x19 → FUN_004235e4 (state 1: +0x1A:=0x80) / FUN_004235bf (state 2: +0x1A:=0), +0x19 := variant<<4; FUN_004245c9 wall redraw; SFX 0x23/0x24 bank ELEV1 0x4edfb0; 86 callers (FUN_00433980 pads) | §7j.21, §7j.34 |
| door animator tick | FUN_00423081 (sole caller MissionShell epilogue 0x44808f, after the creep tick 0x44808a): walks the 45 rects; state≥3 = AUTO doors (countdown@+0xC −1 per tick; at 0 → animate; on completion XOR bit7, re-target +0x19, countdown 0x14, SFX ELEV2 0x4edfb4 — cycles forever); state 1/2 = SCRIPTED doors (animate to target, stop); per tile with low7(+0x1A)≠+0x19: walk planes down (bit7: 5, else 6) → DAT volume door-frame byte 0x40+2·nibble (bit7, even) / 0x5F−2·nibble (clear, odd) at the 0x4eaac8-table level; low7++ ; nibble wrap → FINISH PAIR: FUN_004236c6+00423740 (close: DAT seen 1/0 + STACK PUSH-UP word[z+1]:=word[z], plane0:=0 if S+E neighbors are door tiles) / FUN_00423650+004235fb (open: DAT 0 + STACK DROP word[z]:=word[z+1], top cleared — the level leaves the stack); [0x4eaae8] = the 9th z-plane offset | §7j.34 |
| tile word grid | word[0x460dfa+2·tile]: 0 = empty, 0x7d2 hazard, 0x7d3 phase-clamp, 0x7d4 platform, else object id+1 → rec n−1 @0x46cbf4 (stride 0x14 {x,y,z,id,flags,hp}) | §7j.12 |
| platform strength bank | word[0x465daa+2·tile] = platform hp (build 300 via trigger / 199 via creep; weaken −damage; <0 → destroy: clear water z-word + both banks + 5× k7 debris); ring spread when ≥100 ∧ (hit <200 ∨ new <100) | §7j.12 |
| platform family | damage FUN_00422693 ← weapon ray 0x41a8ff; spread ring FUN_00422832/FUN_004228ce (8-tile, needs empty z-word + planeA 0 + planeB 1 + no robot, writes water z-word + 0x7d4 + strength + scorch+4); creep tick FUN_00422a9c (1/32, ray over water, tip→FUN_00422832(…,199)); site latches 0x4dc5c8/cc | §7j.12 |
| 0x7d2/0x7d3 stamper | FUN_00422f18 (load 0x447b8f): z-word ∈ [0x454a20+4z, +4] → 0x7d2; ∈ [0x454a3c+4z, +4] → 0x7d3; zone bases 0x7d2 {0x20,0x49,0x49,0x34e,0x49,0x77,0x77} / 0x7d3 {0x49,0x77,0x77,0x49,0x4e,0x4e,0x349} | §7j.12 |
| type-DB tail stamper | FUN_00422fd1 (load 0x447ba3): 45 rec @0x4dcae8 stride 0x10 {state,x0,y0,w,h,variant,cd,flag}; STATE@+0 ≥ 3 (§7j.34: the 7j.12 "word@+2" qualifier was the wrong field) → byte 0x4796d5 = variant<<4, byte 0x4796d6 = (state==3?0:0x80) | §7j.12, §7j.34 |
| delayed trigger timers | 32 rec @0x4ea828 stride 0x18 {payload(lo/hi ids), cd(8)}; tick FUN_00422cc2 (epilogue 0x448085): expiry → SFX 0x4239ef(0x22,3), rec flags 0x40, z-plane-A clear, FUN_0041bd54(x,y,z,floor_word[0x454a90+4·zone]) | §7j.12 |
| fast z-writer | FUN_0041bd54(x,y,z,word): word@0x4796bc+30·tile+2z + seen=1 (FUN_0042394a without the DAT volume byte) | §7j.12 |
| scorch increment | FUN_0042223c(x,y,v): byte 0x4796d4 += v clamp 7 (platform damage/build use v=4) — 2nd producer beside FUN_00422287 | §7j.12 |
| weapon impact resolver | FUN_0041a894(x Q13, y Q13, chain ctr ecx, damage ebx, [stack] score flag): tile from x/y>>13; grid word 0/0x7d2/0x7d3 → ret 0 (pass); 0x7d4 → FUN_00422693; n>0 → rec n−1 hp−=damage, destroyed → flags 0x40 + tail → ret 1; ret 1 only on destroy | §7j.13 |
| object type table | 0x4dedf2, 0x4E stride, 282 recs from the mission file (FUN_0041a4f8, load call 0x447b76): W@+2, H@+4, D@+6 (word@+0 unconsumed [open]; 7j.13 erratum + 7j.16 verification), hp@+8, chain@+0xC, type@+0xE (0xb = score 10), count@+0x12, 5×8B effect entries @+0x16..+0x3E (selectors +0x16+8k → 9-case table 0x41a870 — map §7j.25), 4 W·H·D-word template banks @+0x3E/+0x42/+0x46/+0x4A (arena 0x46ad5c; disk order +0x3E,+0x46,+0x42,+0x4A interleaved; +0x46/+0x4A = the UNDER-terrain pair consumed by the destroy restore §7j.25; +0x3E/+0x42 = the CURRENT-state pair ≡ shipped TOT/DAT at footprints — DEAD EDITOR PAYLOAD, zero readers §7j.32) — exact 0x4E fit; footprint stamper FUN_0041a7f0 (word = rec idx+1 over W×H at spawn) | §7j.13, §7j.32 |
| chain detonation | destroy tail walks the object's 4 perimeter edges; chainable neighbor (id-table word@+0xC ≠ 0, alive) → recurse FUN_0041a894(pos, ctr+1@RandA&3==0, damage 1000); score [0x4dd40c] += type (0xb → 10) when stack flag ≠ 0 | §7j.13 |
| destroy-tail effect entries | 5 × 8B @type+0x16+8m (m 0..4, exit @+0x28): selector word@entry+0 ∈ 1..9 → jump table 0x41a870 idx sel−1; payload w2/w4/w6 @entry+2/+4/+6 = x/y TILE + z-level offsets off the 0x46cbf4 record; sel1→k14(+0xF,+0xF)+FUN_0041a225+5 splashes, sel2..5→k18/k17/k16/k19 single gibs at (+0x10,+0x30)/(+0x30,+0x10)/(+0x20,−0x10)/(−0x20,0)+4-splash loop, sel6/7→k10 at (+0x10,+0x20)/(+0x20,+0x10)+DEADMAN SFX (delay 0, param −1), sel8→k14 ×25 demolition shower @water z (±3-tile RandA&7−3 jitter, delay ctr+2m+i>>3), sel9→k20+3×3 splash ring (delay ctr+2+RandA&3); stager delay = chain-ctr+m (sel1/8/9); PRECEDED by the footprint W×H×D terrain RESTORE (TOT-mirror z-words ← bank@type+0x46, seen + DAT volume ← bank@type+0x4A, linear (z·H+i)·W+j); GER gate: type 0xb ∧ GER skips the whole restore/effect/score/chain tail (record still marked destroyed + triggers fired) | §7j.25 |
| effects-bank stager | FUN_0041a225(x,y,z tiles, delay ECX) — FIRST producer of the MISSIONVIEW §5d/§5e "effects loop" bank 0x4cf638: 80 slots × 0x1E (=0x960, the 7j.1 boot-clear bound), free iff word@+0x18==0 (first-fit allocator FUN_0041a4cc, 12-try spawn loop); record {x,y Q13+RandB&0x1F jitter<<8 −0x1000, z<<13+0xF00, vx/vy (RandB&0x3F)<<7−0x1000, vz@+0x14 RandB&0x7FF+0x1770 RISING (high word = sprite group 0..2 → DEBRIS.BIN img group*8+frame&7), active u16@+0x18 = FUN_0041ec59(3) (~8% stillborn), delay u16@+0x1A = ECX arg, frame u16@+0x1C = RandB&7}; callers: destroy-tail cases 1/8; mover FUN_00419f62 (kill off-map/ceiling z>>13>0xB); consumer = the §5e direct draw (7j.26) | §7j.25, §7j.26 |
| .POS + .BDG loader | FUN_0041a4f8 (mission load 0x447b76): opens ".POS" (str 0x457a64) → 2000×0x10 reads into the 0x46cbf4 object-instance array (id≠−1 scan → count 0x46cbe8) — CONFIRMS FORMATS §12 feeds the destructible array; opens ".BDG" (str 0x457a69) → the 0x4dedf2 type table: NO file header, ≤282 VARIABLE records — control u16 (≠1 → 2 B row), else W/H/D u16, hp i32, chain u16, type i32, 5×8B effect entries, FOUR on-disk template banks 2·W·H·D B each (slot order +0x3E,+0x46,+0x42,+0x4A — §7j.32); +0x12 count = nonzero selectors, computed at load; arena cursor 0x46ad5c; tail seeds instance hp@+0x10 ← type hp@+8 + stamps the claim grid per footprint. Corpus 37/37 EOF-exact, exactly 282 recs/file (7907 active), selectors ONLY 1..9 (§7j.25 item 8) | §7j.25, §7j.32 |
| .BDG template-bank semantics | 2×2 roles (§7j.32 corpus proof, ZONEA/M1 434/435 cells): CURRENT pair (+0x3E TOT words, +0x42 DAT words) ≡ the SHIPPED .TOT/.DAT at the .POS footprints — editor stamp payload, ZERO runtime readers (triple census: slot addresses, +0x3e/+0x42 displacements, arena walk); UNDER pair (+0x46, +0x4A) = the pre-building terrain, consumed ONLY by the destroy restore (mirror words ← +0x46; seen=(+0x4A word==0), DAT volume=+0x4A low byte); value domains b1/b2 tile words ≤1868, b3 ≤102, b4 ≤512; overlap footprints = last-.POS-slot-wins in the shipped TOT | §7j.32 |
| TOT-mirror tile record | ONE 0x1E-B record per tile @0x4796bc+0x1E·tile (unifies the scattered tail-byte families, §7j.32): +0x00..+0x0F = the 8 plane words (+2·z); +0x10..+0x17 = the 8 SEEN bytes (restore writes @0x4796cc = base+0x10+z); +0x18 scorch (7j.8/7j.9; the scorch→damage reader 0x40bc60 §7j.34); +0x19 = the door/scenery TARGET-TAG byte (variant<<4; the animator stops at low7==+0x19; readers 0x406bd6/0x406bf9 renderer adjacency, 0x4110cb fire anchor, 0x418735 standing-on-scenery, 0x4237c5/da neighbor test); +0x1A = {bit7 door PHASE, bits0-6 running FRAME COUNTER} (§7j.34: the 7j.12/7j.32 "door byte bit7" gloss refined — one half of the 15-frame slide machine; renderer Y-bias −nibble·0x500 @0x406c5c); +0x1B/+0x1C = the OBJECT-HEIGHT pair (z0, z0+D) — stamped by the objective pass FUN_0044889a (0x448963/75 + 0x448b4f/61), cleared by FUN_00448b80 (0x448c25/2c + 0x448d65/6c), read by the intact-vs-rubble draw pick (0x406891/0x4068ec); +0x1D ZERO traffic (71-site census §7j.34 — padding, closed) | §7j.32, §7j.34 |
| objective-building family | FUN_0044889a (zone gate [0x4edd8c]==7): counts type ids 0x44..0x47 into [0x46cce0] + stamps the +0x1B/+0x1C heights; FUN_00448b80(idx) = the destroy-tail "notify" (SP-only): [0x46cce0]−−, heights cleared, at ZERO → FUN_004239ef(0x28,3)+(0x29,3) + 0x46cd00:=3 / 0x46ccfc:=0x20 / 0x46ccc4:=0x32 (extraction-arm lights, 7j.20 cross-ref); edition≠7 = the script-objective path (0x4eaaee/0x4eaaf2/0x4eab0c walk, tables 0x4557f8/0x456810, code 0x1388) head-decoded | §7j.32 |
| TRT death stamp | FUN_0041bc1c tail (FORMATS §14 resolver): mirror plane word := word@[0x454a04+4·zone] (per-zone rubble table), seen := 1, DAT volume byte := 0, k15 debris FUN_00420608(×0x20 coords, param −1 delay 0) + splash FUN_00424355 at the FUN_0041bd78 water z — the .BDG-tail death shape minus the restore (no under-bank) | §7j.32 |
| mission family loader | FUN_0041dc5a (after path builder FUN_0044670c = "EDITOR\"+"ZONE"+[0x4edd8c]+0x40+"\MISSION"+n): loads .TOT/.DAT/.CGR/.BIN/.MIN then the language gate `cmp [0x4eba1c],1` → .LNG else .LNK, then .PAD @0x41de44 — the eight tags are ONE 5-B-stride table @0x4587d9..0x4587fc (no ninth entry); buffer/cell pairs 0x4dca0c/[0x4ede20], 0x4dca0c/[0x4edd58], 0x4dca8c/[0x4edd60], 0x4dca8c/[0x4ede1c], 0x4dca8c/[0x4edd9c]; second .TOT/.BIN/.DAT site 0x446623..0x446677 (tags 0x459795/0x45979a/0x45979f) | §7j.33 |
| editor-only extension set | ZERO string (case-insensitive byte census) in EXW/EXD/EXE/DIRECTX exes for: .BLD, .CTG, .COL, .MAP, .PTH, .TXT — the runtime never opens them (only "SAVED.BDL" @0x4597d6 = the savegame, unrelated); .BLD = the editor SOURCE of .BDG (record j ≡ BDG non-empty j: same hp/chain/type heads, same four template banks; FORMATS §17 grammar verified) | §7j.33 |
| destruction-thud SFX pair | banks 0x4edfb8 = SOUND\SFX\DEADMAN1.RAW / 0x4edfbc = DEADMAN2.RAW (loader 0x43a29b..0x43a368, strings 0x458f41/0x458f58): RandB&1 pick, FUN_0043a48e(bank,0,x,y,push 2); consumers = destroy-tail cases 6/7 (0x41b19c/0x41b1ac) + the debris-crush dispatcher FUN_0040dce0 (0x40dc62) | §7j.25 |
| projectile mid-flight draw | FUN_00403938 @0x404131 (after the 7j.27 ring passes): walk 400×0x36 offsets 0..0x5460; type w@+0 → 5 shell (WEAPONS 3..7, counter d@+0xE wraps 7→3), 9..0xB artillery (WEAPONS 8..15), 0xE mortar (WEAPONS frame 1 static + 8-puff trail 0x10+(tick+i)&7 mode 0x12E), 0xF/0x13/0x17/0x1A/0x1F damped (WEAPONS base 0x20/0x20/0x28/0x18/0x18 + (tick&7) iff |vx|>0x40 ∨ |vy|>0x40, anchor 0x108), 0x24 rocket (SHRIKE ((dir+0x7E)&0xFF)>>2 = 64-dir; ≤8 SMOKE puffs dist 0x20+0x10·i behind, count = d@+0xA/4), 0x29 homing (REAPER dir>>2; GENERAL reticle @ target d@+6 {0x1000 robot 0x4c69e4/0xA8, 0x2000 critter 0x4cccec/0x20, else FUN_004128ec} frame tick/3+2, anchor 0xF0; 4 SMOKE puffs dist 0x10+0x08·i); all FUN_0040798e modes 0x12C/0x12D; other types NOT drawn; banks WEAPONS/SHRIKE/REAPER/SMOKE/GENERAL = [0x4eddbc]/[0x46af30]/[0x46af2c]/[0x46af34]/[0x4edd7c] | §7j.28 |
| projectile tick | FUN_00412010: 50 rec @0x4cc654 stride 0x22 {active, x, y, z Q13, vx, vy, vz}; per-frame +=v; terrain probe FUN_0041eaa1; impact → FUN_004126dc + FUN_0041a894(damage = FUN_00419aff(0x65/0x66)) + FUN_0041bc1c; the MID-FLIGHT DRAW walk §7j.28 (types 0x65/0x67/0x68 single WEAPONS 0x3C/0x3C/0x38-strip sprites, 0x69 the per-level beam column 0x34-strip, 0x66 NOT drawn) | §7j.13, §7j.28 |
| weapon-anim tick | FUN_00410823(phase 0..3, MissionShell 4×/frame): walks ALL 400 records 0x4c71f4 stride 0x36; record {w@+0 type=weapon id (0 free), d@+2 owner, d@+6 target sel (0x29), d@+0xA tick, xyz@+0x12/16/1A Q13, vxy@+0x1E/22, vz@+0x26, class@+0x2A (0x24/0x29 launch delay; 0xF/0x13 detonation cycles), arc@+0x2E (ballistic z-vel g=−0x100/t; 0x29 heading byte), trail link@+0x32}; per-type: 2..4 bullet 2-substep lookahead ray (commit 1), 5 shell + K3 trail, 9..0xB artillery burst (phase 0 only), {0xE,0xF,0x13,0x17,0x1A,0x1F} ballistic bounce family (0xE 3-blast mortar, 0x17 3-clone split, 0xF/0x13/0x1F damped), 0x24 rocket (launch delay, no gravity), 0x29 homing (robot 0x1000-bit/critter/TRT 0x2000-bit target, terrain-avoid steering, ttl 201); the per-type MID-FLIGHT DRAW map §7j.28 (types not listed there are NOT drawn mid-flight) | §7j.13, §7j.22, §7j.28 |
| artillery burst tables | durations dword[0x456c78+4·id]: w9→2, w0xA→4, w0xB→7 frames; per-frame i16 (Δy,Δx) pair lists (500 sentinel) via PTR[0x456bf0+4·(ttl−0x20)] → 7 lists @0x45687c..0x456adc (frame 0 = 7-cell cluster, then radius-2/-3 rings); each pair = FUN_004244a1 scripted 5000-blast + 50% (RandA) K0xB debris at center | §7j.22 |
| actor hit-test lanes | FUN_0041879d(owner,x,y,z,weapon) = critter lane (3-row presence-grid prefilter @0x4ea900 rows ±4 → FUN_004190bc(critter,owner,x>>8,y>>8,z>>8,weapon,mode 2), first hit returns; count [0x46cc2c]); FUN_0041874c = other-robot lane (MP-gated, FUN_00418fca(robot,…,2), skips owner, count [0x46ccbc]); odd phases only (2×/frame); third caller = renderer FUN_00403938 (weapon 0xC blast, owner −1, args <<5) | §7j.22, §7j.23 |
| critter hit applier | FUN_004190bc(critter,owner,x,y,z,weapon,mode): presence w@+0x24; kind switch w@+0x00 (1..7 = the .NME section states); mode 2 = octile<0x20 on x/y + z-box (kinds 1/4 cell-unit coords, 2/3/5/6/7 Q13; z 0x20/0x24/0x40), mode 1 = x/y only; kinds 3..7 immune while state w@+0x0C ∈ {6,7,0xB}; hit → hp w@+0x06 −= FUN_00419aff(weapon), attacker w@+0x04, flash w@+0x7C, kinds 4..7 state := 5; death per kind 1→FUN_00418835 2→FUN_004188d0 3→FUN_00418aa6 4→FUN_00418ca4(+weapon) 5/6→FUN_00418e26(+weapon) 7→FUN_0041896c (§7j.24; the debris-crush dispatcher FUN_0040dce0 is the second dispatch site) | §7j.23 |
| robot hit applier | FUN_00418fca(robot,x,y,z,weapon,mode): presence d@+0x7C; box test \|dx\|,\|dy\| < 0x20 (d@+4/+8 >>8) + mode-2 \|dz\| < 0x30 (d@+0xC raw); hit → FUN_0040e230(robot, FUN_00419aff(w@rec+0), d@rec+2 owner) + hp d@+0x78 clamp ≥0 | §7j.23 |
| robot damage applier | FUN_0040e230(robot,damage,owner): state w@+0x0C==2 skip; state 3 → shield d@+0x88 := 0x20; gate d@+0x8C==0 ∨ d@+0x88≠0; alarm w@+0x34==0 → counter d@+0xA4 += 3, >100 → SFX 0x10/11/12 per player slot + w@+0x34 := 100; shield-down: hitcount w@+0x2E++, hp d@+0x78 −= dmg, tier SFX 0x2B/0x2C/0x2D, 0x13..0x15 (≤50%), 0x16..0x18 (≤12.5%) vs 5000+100·variant d@+0x94; shield-up: d@+0x88 absorb clamp 0; death MP: scoreboard 0xC-stride @0x4ebaa8 {score d@+0, flag d@+4, d@+8 := 0xB} suicide gate killer==victim∨−1, killer++ cap 999/victim−− clamp 0; shared tail: FUN_0042382c blast record + DAT_0046ccec := 3 + 7 order words zeroed + 5× k5 debris; SP tail: selected→[0x4ede34] := 1, alive/drop/hp := 0, +0x9C := 1, armor 0, SFX 0x19/1A/1B; MP respawn: full reset + variant RandA&3, pod 0x28, MRK reposition, weapon/equipment re-copy | §7j.23, §7j.24 |
| critter knockback juice | kinds 4/5/6 survive-hit 25% (RandA&3==0, owner ≠ −1) → FUN_0041a028(x,y,z Q13, robot x,y Q13): 2nd spawner of the 0x4cec38 0x20-stride effect rows (row {w@+0 0, xyz d@+2/+6/+0xA, cos d@+0xE, sin d@+0x12, ttl d@+0x16 = RandA&0x3F+0x1F, kind w@+0x1A = FUN_0041ec1c(5,0)+3}), heading away-from-shooter ±0x10 jitter + FUN_00420608(x+1,y+1,max(z−0x20,0),10,0,−1); kind 7 in-record knock instead (heading d@+0x10, vx/vy w@+0x74/+0x76 = cos/sin>>6) | §7j.23 |
| impact SFX trio | FUN_00421fc2(x,y): [0x4ede58]≠0, RandB()%3 → one of banks 0x4edf7c/0x4edf80/0x4edf84 → FUN_0043a48e(bank,0,x,y,2) — the critter-hit spark sound | §7j.23 |
| octile distance | FUN_0041ebf8(dx,dy) = max(\|dx\|,\|dy\|) + min/2 — the hit metric (and §7j.22 prefilter) | §7j.23 |
| mortar smoke-trail bank | 0x4e66b8 stride 0x68 {d@+0 active, d@+4 ring&7, 8×0xC xyz}: weapon-0xE tick appends prev pos {x−vx, y−vy, z−arc} every 2nd tick; link = record d@+0x32; SLOT ALLOCATOR CLOSED = FUN_00412a4a (20 slots, first active==0, else −1); allocated at spawn by FUN_0040a9ff when the robot slot weapon == 0xE (link := slot, active := 1, ring zeroed; non-mortar link := 0); cleared on free/detonate; DRAW PASS CLOSED §7j.28: FUN_00403938 @0x40442f draws all 8 ring positions (base +8+i·0xC, active/ring words unread) as WEAPONS.BIN frames 0x10+(tick+i)&7, mode 0x12E, screen+map clipped | §7j.22, §7j.23, §7j.28 |
| critter death handlers | six per-kind handlers over bank 0x4cff98 (idx EAX; k4/k5-6 take weapon EDX): k1 FUN_00418835 state 7+presence 0+1× k1 debris; k2 FUN_004188d0 state 7+presence 0+1× k0xD; k3 FUN_00418aa6 state 7+timer 0+1× k7+3× k6 (delays 0/2/4)+FUN_00421f4c; k4 FUN_00418ca4 w@+0x02 := 1, hp 0, state 6, timer 6, 1× k7, weapon {0x24,0x29,0xC} → 3× k7 + 8 effect rows; k5/6 FUN_00418e26 w@+0x02 := 1, hp 0, state 6, sub-timer 0, 1× k7, weapon-gated 3× k7 + 12 rows; k7 FUN_0041896c state 6, w@+0x78 := 1, 3× k7 falling gibs (z 0xFF−r) + 1× k0xD, SFX FUN_0043a48e(0x4edff8,…,3); k1/k4 px-raw coords, others Q13 >>8, z raw-Q13 → stager-clamped 0xFF | §7j.24 |
| critter bounty gate | all six handlers: attacker w@+0x04 ≠ −1 ∧ robot[attacker].type w@+0x2A == [0x4edb90] → score [0x4dd40c] += 30/50/500/75/150/1000 (k1/k2/k3/k4/k5-6/k7) + DAT_0046ccf0 := 2 (score-strip refresh, = the §7j.6 pickup mechanism); env kills award nothing | §7j.24 |
| debris-crush death dispatcher | FUN_0040dce0(idx, mag, heading, dmg), sole caller = the debris physics tick FUN_0040de9c @0x40e13b: guards w@+0x02 ∉ {7,2} ∧ mag > 2 ∧ dmg ≠ 0; damage FUN_0040eb3c; sin/cos·mag knock + move FUN_00412998 (kind 7 ∨ wall test FUN_0041e9a2); hp ≤ 0 → attacker := −1 + per-kind death dispatch (k4 weapon 0, k5/6 weapon 0x24 = full explosive drops, k5/6 state ∈ {5,6} absorbed) — the SECOND death dispatch site besides FUN_004190bc | §7j.24 |
| critter-death SFX trio | FUN_00421f4c(x,y): [0x4ede58]≠0, RandB()%3 → banks 0x4edf88/0x4edf8c/0x4edf90 → FUN_0043a48e(bank,0,x,y,2); twin of the impact trio FUN_00421fc2 (0x4edf7c/80/84) | §7j.24 |
| effect-row spawner | FUN_0041a14f(x,y,z Q13,count): rows 0x4cec38 stride 0x20 via allocator FUN_0041a494 (ages every row w@+0, returns MAX-age — always-evict LRU, 80 rows); row {age 0, xyz d@+2/+6/+0xA, cos/sin d@+0xE/+0x12, d@+0x16 = (RandA&7)·0x10+0x80, id w@+0x1A = i (<8) else FUN_0041ec1c(5,0)+3, w@+0x1C/+0x1E 0}; callers: k4 death (8), k5/6 death (12), controller ballistic landing (0x18); FUN_0041a028 (§7j.23 knockback) is the parallel writer w/ different +0x16 | §7j.24 |
| robot-death blast bank | 0x4eb638, 32 × 0x14 {x d@+0, y d@+4, z-dword d@+8, age/claim d@+0xC, frame d@+0x10} — the MISSIONVIEW §5d/§5e "platform loop" bank; PRODUCER = FUN_0042382c(idx) from the FUN_0040e230 death tail: gate = 0x46af58 claim byte == 0 at the robot tile, slot = FUN_004238ea (first age 0 else MIN-age); anim tick FUN_004238af (frame ++ wrap 0x10→4); CONSUMER (7j.26) = enqueue pair SMOKER.BIN frame 0 mode 300 + frame d@+0x10+1 mode 0x12d (DARKPAL) at sy−0x20 | §7j.24, §7j.26 |
| direct blit codec | FUN_00401e39(img, transp 0/≠0, x, y; ESI bank, EDI dest) — the shared draw_IMG consumer: .BIN = u16 count word0 + int32 dir at bank+2+4*img (offset rel. own slot; corpus-verified 24/24 DEBRIS, 160/160 DANTE), hdr {flags u16 (bit1 hotspot (yoff,xoff) s16×2, bit0 RLE), w, h; w/h==0 → instant skip}; RLE words bit15=skip(→zero-paint when opaque)/literal raw copy, bit14=EOL; dest EDI+y*0x280+x stride 0x280; NO palette modes (vs the §5 flush codec FUN_00401471); counts: DEBRIS 24, SMOKER 17, DROPSHIP 210 | §7j.26 |
| effects mover | FUN_00419f62 (MissionShell @0x44813d): delay −− else x+=vx/y+=vy/z+=vz; kill +0x18:=0 iff x/y/z<0 ∨ x>>13≥[0x4eddec] ∨ y>>13≥[0x4eddf0] ∨ z>>13>0xB | §7j.26 |
| platform anim tick | FUN_004238af (MissionShell @0x447fff): for active 0x4eb638 records d@+0x10++, wrap 0x10→4 (drawn smoke column 2..16 intro, 5..16 loop) | §7j.26 |
| bounded random helper | FUN_0041ec59(n) = RandB()/(0x8000/n − 1) clamped n−1 — uniform-ish [0,n−1] on the 15-bit RandB | §7j.26 |
| dropship ring banks | 0x4e64c0 (12 × 0x1C robot-indexed) + 0x4e6610..0x4e66b8 (6 × 0x1C standalone) {active d@+0, PHASE d@+4, x d@+8, y d@+0xC, alt d@+0x10, img-group d@+0x14, dwell d@+0x18}; consumer draws 7-COL × 5-ROW grids of 0x40 tiles (448×320 px — the 7j.26 "7×7" corrected §7j.27), img = group*0x23 + 7*row+col, bank [0x4edd64] = DROPSHIP.BIN (ArenaAlloc 0x25990; 210 = 6 groups × 35); ends at the trail bank 0x4e66b8; producers CLOSED §7j.27 (resets: FUN_0040cca0 @0x40cd3d pods 0x150 + MissionShell 0x447a7e/0x447a8d; spawners FUN_0041fa51/FUN_0041faf0/FUN_0041fb4b; animator FUN_0041fbb1; + the 0x412b60 exit-dwell reset) | §7j.26, §7j.27 |
| terrain restamp list | [0x4ede24] ptr + [0x4ede28] count → 3-dword records {dest row (y·0x280 basis), tile-x, tile-y}; render-tail readers 0x4067a6/0x406b32 blit each via FUN_00401471 (border tile FUN_00408030 off-window, full LNK path in-window); writers FUN_00440a2d (= the TOT-mirror materializer = the scroll/camera restamp stager), FUN_0043d00b, FUN_0041d954 — resolves the backlog "7×7 screen-address table" hypothesis | §7j.26 |
| NOP stub | FUN_00418a9f (0x418a9f..0x418aa6, empty): called by the k3 death handler + FUN_004197d4/00419943/00419c7c (+ jump from FUN_00419f62) — cut-feature hook | §7j.24 |
| tile-0x62 trap pair | FUN_0040fe93 (robots() caller @0x40bc44) / FUN_0040ff92 (critter FUN_00412f34 @0x413fd7): type-DB byte 0x62 ∧ grid ≠ 0 → FUN_0041a894(damage 100, no score); destroyed → 5× k12 debris (±RandA jitter, delays 0/2/4/6/8). The 0x4c69e4 "160-B stride" was a census slip — TRUE stride 0xA8 (21·idx·8, §7j.25 item 7); anomaly CLOSED | §7j.13, §7j.25 |
| weapon damage table | FUN_00419aff(EAX id) → EAX damage: 2→20, 3→30, 4→40, 5→75, 0xc→5000, 0xd→312, 0x1a→75, 0x24→400, 0x29→250, 0x65→(d+1)·50 [d=2→200], 0x66→(d+1)·300 [d=2→1200], 0x67/0x68→(d+1)·75 [d=2→300], else 1; 28 callers | §7j.15 |
| difficulty scalar | dword 0x46cbf8, 0..2: cycled (d+1)%3 at NameEntryScreen, save-persisted, zone-7 temporarily forces 2 (GameMain); scales projectile damage 0x65..0x68 (7j.15) AND critter behavior (7j.17: respawn delay DAT_00454edc[d], 0x65 range 172/236/300, engage leash 640/704/768, point-blank fire rate 32/16/8 frames, attack-break 1/8·1/16·never; 12 objdump sites in FUN_00412f34) | §7j.15/§7j.17 |
| critter-actor controller | FUN_00412f34 (MissionShell @0x447fe1): bank 0x4cff98 stride 0x7E count DAT_0046cc2c (FUN_00416458 @0x41646d — the .NME loader, §7j.18); frame §7j.17 item 1 — state 1 wander / 2 sine-walk shooter (0x65) / 3 chase (0x67) / 4·5·6 mixed-AI (modes 0xB dormant·7 dying·6 ballistic→k6 debris+splash·9 seek·2 range) / 7 close-combat (0x69); presence byte mark [[0x4ea900+(y>>13)·4]+[0x46af4c]+(x>>13)] := 1 | §7j.17/§7j.18 |
| critter seek-acquisition dispatcher | FUN_00415490(idx): dword@+0x10 (dual-purpose: wander heading 0..255 / mode-9 seek direction 0..3) `cmp 3; ja FATAL` → table 0x415480; 4 directional forward-acquisition probes vs the robot bank 0x4c69e4/0xA8 (tight −4..+0xF ahead on the walk axis, |Δ|<0x18 crossing + z; case 3 reads robot y RAW — quirk); hit → target w@+0x7A, mode w@+0xC := 2, anim w@+0x56 := 0; >3 → "Buggered direction in MOFO" 0x457a3c fatal (fade-cancel 0x420100 + print 0x44d2ac + FATAL EXIT 0x44d2da); the mode-9 walk dispatches the same dword via table 0x412ef8 → steppers 0x417f2c/0x417fe8/0x4180c0/0x41813d (y−1/x+1/y+1/x−1), step-OK → move one unit + call FUN_00415490 | §7j.29 |
| mission extension tags | DGROUP 0x457a57 ".NME" / 0x457a5c ".TRT" / 0x457a64 ".POS" / 0x457a69 ".BDG" — exactly one reference each (0x41648c/0x4170c3/0x41a55d/0x41a5d6 = the four CLOSED loaders §7j.18/§7j.15/§7j.25); 0x457a4c "MOFO\0" = dead tail of the fatal string 0x457a3c, ZERO refs, no ".MOFO" bytes in EXW or EXD, no *.MOFO corpus file — the ".MOFO loader" RETIRED | §7j.29 |
| suicide-bomb trigger | FUN_00417e2f: nearest robot (FUN_00417c00) < 0x30 px → deactivate + 8× debris k1 + 8× FUN_00424355 rings | §7j.17 |
| POI/personnel controller | FUN_00412a98: bank 0x4dabdc stride 0x1E count DAT_0046cbf0 (FUN_00416458 @0x416f6e — the .NME section-8 loader, §7j.18: 4 POIs per record, spawn state 5 ESCAPE); {active@0, state@4 (1 idle/2 settle/3 walk/4 flee/5 ESCAPE/6·7 panic), heading@8, timer@0xA, xyz@0xE/+0x12/+0x16}; escape → [0x4eba0c]++, [0x4eba10]=0x32, FUN_00448b80(5000); walker FUN_00415b6c | §7j.17/§7j.18 |
| exit/threat slots | 5 × 0x1C @0x4e662c {active d@+0, PHASE d@+4 (1 descend / 2 landed-OPEN / 3 depart — §7j.19 reread of the 7j.17 "kind"), x/y d@+8/+0xC, altitude d@+0x10, img-group d@+0x14 (7j.27: the animator's per-tick DROPSHIP.BIN frame selector), dwell d@+0x18 — RESET TO 0 BY FUN_00412a98 @0x412b60 on each POI rescue (multi-POI elevators), cleared on escape}; nearest scan FUN_00417c64 (gate phase==2); producer CLOSED §7j.18: FUN_0041fa51 = the EXIT-PAD ACTIVATOR (arg = a 0x4e44f8 .PAD slot index; dedup registry 5×d @0x46cd20; stamps {1, 1, pad.x·0x20+0xF, pad.y·0x20+0xF, 0x400, 0}; sole caller FUN_00433980 case 0x1B @0x43900e (§7j.19); animator FUN_0041fbb1 §7j.19; boot reset MissionShell 0x447a8d | §7j.17/§7j.18/§7j.19/§7j.27 |
| escape-craft animator | FUN_0041fbb1 (MissionShell @0x448012, per frame): 3 machines over the 0x1C frame {active@+0, phase@+4, x@+8, y@+0xC, alt@+0x10, img-group@+0x14, dwell@+0x18} — the 5 exits + the dropship @0x4e6610 + the per-robot pods @0x4e64c0 (gated [0x46aed4+idx·4]==0, the no-extract latch: boot-clear GameMain 0x41c408, writers FUN_0040e230/FUN_00449c94/FUN_0044a38a/FUN_00408e99 — the latch ALSO gates the MP respawn @0x40e7a1); dropship landing = extraction sweep (states 3/4 → 5, _DAT_004dc680++, SFX _DAT_004edfe0), depart → _DAT_004dc67c=1 (complete; readers MissionShell 0x4486d5 + FUN_0044425c ×2); pod landing = payout 100·w@+0x94+5000 + state 6 (robot RELEASED) + msg. §7j.27 per-tick write map: phase 1 alt −0x20/(v>>2)·3 + img-group toggles 0↔1; phase 2 alt := (RandA&7)==0 jitter, exits dwell++>0x78, dropship dwell−−, pods ONE TICK then payout; phase 3 alt += (alt>>2)+1, x −= group·4, group ramps 2..5 then oscillates 4↔5, alt>0x200 → active 0 | §7j.19, §7j.27 |
| dropship deployer | FUN_0041faf0: stamps 0x4e6610 {active 1, phase 1, img-group 0, alt 0x200, x beacon.x<<5, y beacon.y<<5} from beacon 0x4eabb4/0x4eabb6, clears 0x4eabb0/0x4eabb2 (x/y words SURVIVE — renderer 0x4070c0 reads the always-0 z word 0x4eabb8 as a no-op sy nudge); caller MissionShell @0x44832f/0x448375 (countdown 0x4eabb2 == 0 ∨ all robots dead/state-3); beacon armer FUN_004247b5 [§7j.20]; boot reset MissionShell 0x447a7e | §7j.19, §7j.27 |
| pod spawner | FUN_0041fb4b(idx): stamps 0x4e64c0+idx·0x1C {active 1, phase 1, img-group 0, alt 0x400, x/y = robot pos>>8 (Q13→Q5)}; caller FUN_0040b9f6 when countdown w@0x4c6a10+idx·0xA8 == 0 (msgs 9/10/0xB for the player's first 3 robots); the 0x4c6a10 producers [§7j.20]; bank reset = FUN_0040cca0 @0x40cd3d (memset 0x150 = 12 records, every mission spawn) | §7j.19, §7j.27 |
| extraction-beacon armer | FUN_004247b5(EAX tx, EDX ty, EBX z, ECX idx): guard 0x4eabb0; 0x4eabb2 = 0x197 (0 if player-0 alive-count == 1); 0x4eabb0 = 1; 0x4eabb4/6/8 = tile trio (z dead store); robot.state = 3; spread-teleport FUN_004248c8; SFX 0x2A. Sole caller FUN_00433980 @0x433cfb = ~25 (zone, .PAD slot) extraction pads | §7j.20 |
| spread-claim picker | FUN_004248c8(&tx,&ty): first free slot of 12×u16 0x4eabba (bound DAT_0046ccbc), marks 1, returns beacon tile + {center, 8 neighbors, (−2,0),(0,−2),(+2,0)}; ≥12 → out-params untouched (callers store garbage); claims never released; callers FUN_004247b5 @0x424865 + FUN_0040b9f6 @0x40c08f | §7j.20 |
| pod-deploy countdown writers | w@robot+0x2C (0x4c6a10): FUN_0040cca0 spawn tail @0x40d132 stagger 1+k·(2000−m·1000/27) per player group (m = linear mission 0x46ae8c); FUN_0040e230 MP respawn @0x40e89d = 0x28; reader/decrementer FUN_0040b9f6 (brain gate) | §7j.20 |
| per-player selected anchor | 0x4c71c4: 4×0xC {x>>8, y>>8, z}, spawn-seeded by FUN_0040cca0 tail (selected robot idx DAT_0046cbd4), renderer-updated FUN_00403938 @0x403994/0x4039d2/0x403a27; sits immediately before the 0x4c71f4 bank base | §7j.20 |
| pad-trigger dispatcher | FUN_00433980 (3185 B, caller FUN_0040b9f6 @0x40bd58 when state∈{1,4} ∧ order 0x46cc30[idx]≠−1): FUN_00422e5e = the PAD-TILE PROBE (DAT byte 0xFF → 999×8B .PAD slot scan @0x4e44f8 → slot id; revisit latch 0x4eb9fc/counter 0x4eb9f4); per-zone switch on 0x4edd8c — elevator rides [§7j.21: arm the matching 0x4dcdb8 record — rider state@+0x0C := 2, pos := the record's own marker x/y ·0x2000+0x1000 (dwords 0x4dcdbc..0x4dd330), countdown := 10, +0x20 := rider], messages FUN_00424a6f (strings 0x458ca7…, latch 0x4eb5f8), doors FUN_004223b8 over the 45×0x10 rects @0x4dcae8, case 0x1B = the exit-pad activation | §7j.19, §7j.21 |
| critter/POI (.NME) loader | FUN_00416458 (the mission-load dispatcher's critter hop): stages ".NME" (@0x457a57) → 8 fixed-order sections (widths 10/10/8/8/10/8/6/8) feeding critter states {2,1,5,4,3,6,7} + 4 POIs/record; spawn multipliers by difficulty; hp = base+(base·d)/27, bases 0xAF/0xC8/0x96/0x5DC/0x9C4; corpus-exact on all 37 files (ZONEA/M1 16-B orphan tail unread) | §7j.18 |
| command-record consumer | FUN_00409138 (MissionShell @0x448030 after FUN_00410644+FUN_00449c94): records 0x4dd4a0 stride 0x80 count DAT_0046cbe0; flags byte@+5 (bit0 select→0x46cc30/0x46cc60 + auto-arm, bit1 order→0x4dd484/88/8C, bit4); 39-case weapon switch (id−2): order dispatchers FUN_0040b615/0xaf98/0xa56f/0xace8/0xa7a1/0xa9ff + projectile spawners into the 400×0x36 bank 0x4c71f4 (types 0x9..0xB/0xF/0x13/0x1A/0x1F/0x24, aimed at the order target, ammo/enable/cooldown bookkeeping, auto-rearm + msgs 0x1C..0x21) | §7j.17 |
| mission-objective resolver | FUN_00448b80(type: 5000 = rescue, else destroyed object type): 6×0x20 slots @0x4eaaee {remaining w@+2, type w@+6, status w@+0xC, quota w@+0x1E}; kill-stats [0x46cbf4]+type·0x14; mirror-row wipe 0x4796d7/d8; msgs 0x26/0x27/0x34, all-done 0x28+0x29; DAT_0046cd00 = phase state 1/2/3/4; zone-7 counter [0x46cce0] types 0x44..0x47 | §7j.17 |
| floor probe | FUN_0041e411(px,py,z): level try +1/−2; per-type height entry [0x4edd60+2+(type−1)·4] → in-tile 0x20×0x20 byte map @(x&31)+(y&31)·32 at +6; floor = level·0x20 + byte; 0x1F = top-of-stack (sibling of FUN_0041eaa1 §7j.14) | §7j.17 |
| walk/settle helpers | FUN_0041f8f9 8-sample walk probe (0x4543e4/0x454404, level ∧ height-diff ≤3); FUN_004186fc standing-on-scenery (mirror 0x4796d5); FUN_004182c3 8-corner z-settle (snap +0x13/+0x0B); FUN_0041642d anim ctr wrap; FUN_0041286f 50×0x22 free slot; FUN_00412848 400×0x36 free slot | §7j.17 |
| terrain-structure loader | FUN_004170a6 (call 0x416487 in the dispatcher FUN_00416458): ".TRT" section @staging buf 0x4dca0c; clears 250×0x20 @0x4cccf8; count→[0x46ccd4]; rec (canonical frame, active@0x4cccf8): active=1, state=1, frame=0, fire=0, hp=250+(250·mission)/27, x/y/z tiles; stamps tile 0x66 @byte[[0x4edd58]+x+y·w+z·w·h] + word 1 @word[[0x4ede20]+2(x+y·w+z·w·h)] (the .DAT/.TOT file volumes) | §7j.15 |
| TRT anim/fire machine | FUN_00417264 (MissionShell @0x44807b, every frame): states 1 idle→2 alert (frames 0..7→TOT word frame+1)→5/6/7/8 aim S/N/W/E (octant vs nearest robot FUN_00417c00 dist<0x81)→FUN_00417698 fire at frame top + 4-frame muzzle (words 0x17..0x1E); 3/4 = death/settle; FUN_00417210(idx,n) = mirror word n+1; FUN_00417652 = frame remap 0xF→7, 6→0xE | §7j.16 |
| TRT fire routine | FUN_00417698: lane test |lateral|<0x28 px + direction + ≤2 levels vs robot bank 0x4c69e4/0xA8; arms fire_ctr@+0xC; odd ctr → FUN_0041286f free slot → projectile type 0x66 (damage (d+1)·300) @0x4cc654+slot·0x22 {x,y tile·0x2000+0xF00, z<<0xD, +0x16=0x14, unit vx/vy}; structures never move | §7j.16 |
| map volume loader | FUN_0041dc5a (MissionShell @0x447b3a): ".TOT"→[0x4ede20] (u16 W,u16 H header + 8 planes W·H u16 → [0x4eddec]/[0x4eddf0]/[0x4eddf4]), ".DAT"→[0x4edd58] (same header, u8 planes, >0x7F sanitized→0), ".CGR"→[0x4edd60], ".BIN"→[0x4ede1c] (word→[0x46cdb8]), ".MIN"→[0x4edd9c], .LNG/.LNK→0x45cdda, ".PAD"→999×8B slots 0x4e44f8 stamping 0xFF; FUN_0044661b = the EDITOR\ZONE restore reload; FUN_0041dbed/FUN_0041cd90 = path/section opener (handle 0x4eba20) | §7j.16 |
| TOT materializer | FUN_00440a2d (caller FUN_00440dc2): 7×7 tiles × 8 z: TOT word≠0 ∧ DAT byte==0 → mirror word@0x4796bc = word + seen@0x4796cc; bridges the .TOT volume into the runtime mirror (how TRT word-1 stamps become visible) | §7j.16 |
| map-click pick | FUN_00419943 (caller FUN_00410644 ← MissionShell @0x448021): rect list 0x4787c4/{center@+8/+0xC, w@+0x14} count [0x46ccd8] (written by renderer FUN_00403938) with octile cost FUN_0041ebf8; else screen→iso ((p−0xF0)·[0x4ede54])/0x1E0 + TRT scan; ret 0=ground / k+1=rect / (idx+1)\|0x2000=structure; FUN_00418a9f = empty stub | §7j.16 |
| click order target | {x,y,z} = 0x4dd484/0x4dd488/0x4dd48c written by FUN_00410644 (ground iso / rect / structure tile-center) AND by FUN_00409138 (command-record bit1, words@+7/+9/+0xB); readers FUN_00409138 ×6, FUN_0040af98 ×3, FUN_0040a56f/0xa7a1/0xace8/0xb615/0xa9ff ×2 each, FUN_00449c94 | §7j.16/§7j.17 |
| scanner overlay | FUN_0041ec81 (MissionShell @0x48142): corner widget box 0x1EE..0x272×0xC3..0x147, grow [0x4edd68]→0x40, asset GAMEGFX\SCANNER.BIN; FUN_0041ee20(cx,cy) around the SELECTED robot ([0x46cbd4]+[0x46cbdc]): icons via FUN_00402572 (128×128 blitter→[0x4eddb8]) — 1/2 robots sel/rest, 4=0x4cffbc, 5/6 linked blink, 7/0xD tiles, 8=TRT, 9/0xA objects, 0xB arrivals, 0xC pads | §7j.16 |
| nearest-robot probe | FUN_00417c00(px,py,&dist): octile over robot bank, ret idx; callers: turret machine + FUN_00412a98, FUN_00412f34 ×4, FUN_00417e2f (the robot targeting family). FUN_0041ebf8 = octile distance max+min/2 (51 sites) | §7j.16 |
| terrain-structure array | recs @0x4cccf8 + i·0x20, i < [0x46ccd4] — {active@+0, hp@+0x10, x tile@+0x14, y@+0x18, z@+0x1C}; externally 1-based (dword[0x4cccd8+id·0x20] = rec id−1 active; 0x4cccd8 = id-0 guard) | §7j.14 |
| terrain damage resolver | FUN_0041bc1c(x Q13, y Q13, damage): match rec by tile → hp−=damage; hp≤0 → active=0 + floor word [0x454a04+4·zone] → TOT @0x4796bc+30·tile+2z, seen @0x4796cc, DAT volume=0, debris K0xF, splash at first free level | §7j.14 |
| terrain-height probe | FUN_0041eaa1(x Q5, y Q5, z): DAT volume byte 0 → miss; else height = [0x4edd60] bank ptr (h−1)·4+2, +6 header, byte[(y&31)·32+(x&31)]; hit iff z ≤ (z>>5)·0x20 + height | §7j.14 |
| weapon-anim disburser | FUN_004124a4(rec idx): rec 0x4c71f4+0x36·i (400 slots, free-slot FUN_00412848), kind word@+0; w2..4→K2 (±3 jitter), 5→K3, 0x24→K6, 0x29→K9, {0xE,0xF,0x13,0x17,0x1A,0x1F}→K0xC; z−10; 9..0xB clear-no-debris | §7j.14/§7j.17 |
| projectile disburser | FUN_004126dc(rec idx): rec 0x4cc654+0x22·i, TYPE word@+0 (0=free; NOT plain "active"); 1→K2, 0x65→K0x14, 0x66→K8, 0x67/0x68→K4; coords z NO −10; robot-hit expiry via FUN_004197d4 (|dx|<0x10 Q8, |dz|<0x20) | §7j.14 |
| splash gates/eviction | FUN_0041bd78: first z ≥ min(z,7) with DAT 0 ∧ seen 0; FUN_00424355 gates: DAT-empty ∧ TOT word 0 ∧ claim byte[0x46af58+tile]=0; full ring → evict max-age + FUN_0042394a flush | §7j.14 |
| splash records | 250 × 0xA @0x4e9778 {x,y,z,delay,age}; ticks in the epilogue | §7j.10 |
| splash life | stamps water_base[zone]@age1, base+0x16@age40, frees @age≥47; body odd frames only | §7j.10 |
| z-structure writer | FUN_0042394a: zword@rec+2z, seen@rec+0x10+z, DAT volume byte | §7j.10 |
| DAT volume read | FUN_0041eb28(x,y,z): byte[DAT+tile+zoff[z]], 0xFF→1 | §7j.10 |
| tile-claim bank | 0x46af58, 10000 B arena @mission-load; order-marker writer 0x425556 | §7j.10 |
| sidebar select strips | [0x1E7,0x217]/[0x219,0x249]/[0x24B,0x27B] × y[5,0x35]; F1/F2/F3 latches 0x4edc0c/10/14 | §6c.2 |
| sidebar order rows | x[0x1E9,0x275] × y[0x57,0xB8]; row=(y-0x57)/14 clamp ≤6; keys 1..7 latches 0x4edc18+4k | §6c.3/4 |
| sidebar redraw flag | DAT_0046ccec countdown: set 2/3 by producers, dec+FUN_00408403 in the FUN_00403938 tail | 0x407205 |
| map-toggle strip | x[0x213,0x24D] × y[0x1B5,0x1CF]; MSpace latch 0x4edc08; writes 0x4eb8dc=5, toggles 0x4edba0 | §6c.1 |
| map overlay draw | FUN_004089b1: clear 0x4b000, TABLE.BIN img0 480×480 @(0,0), stamps row'=0x80+r+c−2z / col'=0xf0−2r+2c, markers 0x55..0x59; non-returning tail | §7e |
| territory variant | byte@0x4c420c+tile; zeroed 0x27d8 by MissionShell; 11×11 max-stamp rings 7..1 (dwords 0x454cf8) around robots (FUN_00408dcc ← robots() state 2) | §7e |
| MAPTRAN ramps | u32@(0x4dd464+4i) ← GAMEGFX\MAPTRAN{i}.TRN (256 B each, i 0..7); ramp[mask byte] = palette byte | §7e |
| PALTRAN ramps | u32@(0x4dd444+4i) ← GAMEGFX\PALTRAN{i}.TRN, slot 0 NULLed after load (MISSIONVIEW §8.2 producer closed) | §7e |
| LNK map lookup | cw = word@(0x45cdda + 2*w) — the LNK image doubles as the map type→mask index; masks = .MIN bank [0x4edd9c] (16 B/cw) | §7e |
| pickup range tables | A/B dwords @0x454a58/0x454a74 (7 terrain sets, 4-word closed groups → cases 1/3/2/4 + 9/7/8); floor word table @0x454a90 | §7h |
| map present | FUN_00401107 map mode: 480×480 from backbuffer base, stride 640; button chrome 0x8f/0x5f/0x5e @ (0x213,0x1b5) | §7e |
| backbuffer | [0x4ede18] = ArenaAlloc(0x64000) = 640×640; overlay clears 0x4b000 (480 rows) | §7e |
| order table | 7×0x0E groups @ 0x4de664+type*0x62; group word0/+0x36+8i (default probe), word1/+0x38+8i (gate) | §6c.6 |
| DAT tables | z-base@0x4eaacc, y-line@0x4ea900 | 0041eb28 |
| loader | load_mission@0041dc5a; paths@0x44670c; sweep ≥0x80→0 planes 0..6; PAD→DAT 0xFF @ plane=kind | 7c |
| CGR height byte | CGR[2+4(type−1)+dir[type−1]+6+(sy<<5)+sx] (no codec) | 0x41e328, 7c |
| MRK word 3 | spawn z level (z = w3<<5 − 1) | 0x40d06d, 7c |
| CGR/DB ptrs | DAT_004edd60 (CGR), DAT_004edd58 (DAT), 0x4796bc/cc (type DB 0x1E stride) | 0041e231, 00407e11 |
| viewport cache | DAT_004ede24 36×36×12 B (screen off + tile deltas), count DAT_004ede28 | 00407e11, MISSIONVIEW §2 |
| terrain bank | BIN→0x4ede1c (MISSION{A..G}.BIN sprites), LNK→0x45cdda = per-frame anim link | MISSIONVIEW §1/§4 |
| dither noise bank | 0x4e6ed8 (2048 B .bss ring, cursor 0x4ddb30), bytes {0,0xFF}, `RandB()&3==0` 25%; boot fill MissionShell 0x447b13, churn 15 B/frame 0x448147 | §7i |
| dither blit | FUN_00401ae6(y,h,x,w,src_off,mode): mode 0 = rep-movsb full copy (dead/unoccupied boxes), mode ≠ 0 = nonzero-only overlay (hit flash); reseed `RandB()&0x1ff` when src_off+96 ≥ 0x800; seed `(RandB()&0x7fff)/15` clamp ≤ 0x7f5 | §7i |
| SFX bank→name map (COMPLETE) | 202 durable assignments, zero unnamed durable cells: mission set 0x4edf60..0x4edfbc+ELEV/BEEP/TEXTBOX = 27 registers by FUN_0043a1d3 (MIDIGUN dup at 0x4edf70 quirk), screen sets MENU1/2+BEEP1/4/5/7+TEXTBOX1+DOOROPEN/DOORCLSE (0x4edfc0..0x4edfec, cells reused per screen; 0x4ee00c/0x4ee010 = the debrief MENU1/2 alias), mission-extra 0x4edfe0..0x4ee008 (BEAMIN/THROW/BIOFIRE/PEXPLODE/CACODETH/SQUAWK/GRUNT1..3), speech 0x4ee014+8i 53 records {A,B} (95 files, 11 empty +4 slots, pair slot-order flip at SPCH16); GFX 0x46af2c..0x46af54 + 0x4eddXX/0x4edeXX/0x46cbXX families + G-variant picks (language index 0x4eba1c==1, edition gate [0x4edd8c]>4 → GRILLA family); palettes SHARE role cells (0x4edbf8 current-screen PAL ×6 names, 0x4edbfc TXPAL1..3, 0x4edc00 DARKPAL family); full dump ghidra-project/exw-banknames.txt | §7j.30 |
| SFX register/play family | FUN_0043a36e = 1-voice register, FUN_0043a39c = 4-voice register (clone pair; stage via scratch cell 0x46af0c → arena 0x2b11 → 0x44c64c returns the VOICE-BASE handle — SFX cells hold handles, not pointers); FUN_0043a48e = play/steal (x,y=−1,−1 → vol 0x7f/pan 0x8000; else FUN_0043a3e0 pan / FUN_0043a447 vol vs listener 0x4edde4/0x4edde8; 4-voice probe 0x44c5ac, steal by priority [0x4ee1c2+2v]>>16 + age [0x4ee2e2+2v], start 0x44c904); speech bypasses it (indexed slot pick + 0x44c8c4 direct, vol 0x7f00) | §7j.30 |
| hot-rect click-target array | ONE array base 0x4787bc (record 0; the dispatcher's 1-based view 0x47879c = base−0x20), stride 0x20, 8 dwords {+0 world X, +4 world Y, +8 hit-box X origin, +0xC Y origin, +0x10 z, +0x14 w, +0x18 h, +0x1C type}, count [0x46ccd8] cap 0x77 (extent ..0x47969c), per-frame reset @0x403a9a; writers = 7 sites ALL in FUN_00403938: w1 0x403c87 robots MP-only ([0x4edb88]==2 ∧ ≠local player) type (idx+1)\|0x1000 w/h 0x40 z=rec+8+0x21 corner tile+0xB; w2-w7 0x4056f1/0x4058b8/0x405c4d/0x405f7b/0x406142/0x4062c6 critter .NME paths (state ∉{6,7,0xB}; w7 {6,7}) type idx+1, z ∈ {[crit+0x3E] raw/+0x20/+0x10/>>8}, w ∈ {0x3C,0x40} h 0x40 | §7j.31 |
| click picker | FUN_00419943 (only caller = dispatcher 0x41068e): scans hot rects i<[0x46ccd8], box = origin+(w/2,h/2) ± (w/2,h/2); priority = octile FUN_0041ebf8 max(\|dx\|,\|dy\|)+min/2, early-out <4; returns i+1; ground fallback = iso (mx−0xF0)·[0x4ede54]/0x1E0 + camera + TRT active-scan (x/y/z @+0x14/18/1C ×0x20, windows −0x10..+0x30) → 0x2000\|(idx+1) else 0 | §7j.31 |
| click order dispatcher | FUN_00410644 (MissionShell @0x448021; gates mouse≠−1/[0x4ede14]≠0/[0x4edba0]==0/mx<0x1E0): picked → type cell [0x46cc00] (NEW pin); bit13 TRT: rec(id−1) via −0xC-bias base 0x4cccec, coords ×0x20+0x10 → ORDER TARGET 0x4dd484/88/8c; bit12 robot: corner +0/+4 + z +0x10; else critter: corner + FUN_004128ec(id−1)>>8+0x15; ground: camera+view-mouse z0; tail [0x4ddb20]\|=2 order latch (NEW pin) + [0x4ede00]:=−1 consume | §7j.31 |

## 7j.31. The HOT-RECT CLICK-TARGET RECORD — one 0x20-stride array; writer census + both reader families (2026-08-22, worker aa62f5ed claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run)

Closes the queue's standing "0x4787c4/0x47879c hot-rect record" item
(backlog + item 2). Supersedes/extends the §7j.16-era skeleton rows
("map-click pick" / "click order target") with the full grammar,
writer census, and type-word semantics. HYPOTHESIS CONFIRMED: ONE
0x20-stride record with
both reader views — with one refinement: +8/+0xC is the hit-box
ORIGIN (the picker adds w/2,h/2 to get the box center), not a
"center". Full traffic census: exactly 7 writer sites (all in the
renderer FUN_00403938), 1 picker, 1 order dispatcher; nothing else
in .text touches the array.

### The record grammar [verified]
- Array base 0x4787bc (record 0), stride 0x20, 8 dwords/record;
  count cell [0x46ccd8] (u32); cap 0x77 (119) records → extent
  0x4787bc..0x47969c. The queue's "0x47879c" = base − 0x20 — the
  DISPATCHER's 1-based view (it reads record id−1 via
  [id·0x20 + 0x47879c-family]); the picker/writers use the
  0-based 0x4787c4-family bases. Both are the SAME array.
- Reset per frame: FUN_00403938 prologue @0x403a98-0x403a9a zeroes
  [0x46ccd8] (single reset; per-frame rebuilt renderer scratch).
- Fields (rel. record base):
  - +0x00 world/corner X (robot writer: tile x + 0xB; critter
    writers: tile x = [crit+0x0C]>>8, one path stores raw Q8)
  - +0x04 world/corner Y (analogous)
  - +0x08 screen hit-box ORIGIN X
  - +0x0C screen hit-box ORIGIN Y
  - +0x10 Z (click-priority z)
  - +0x14 W (box width)
  - +0x18 H (box height)
  - +0x1C TYPE word: bits 0..11 = 1-based bank id;
    bit 12 (0x1000) = robot class; bit 13 (0x2000) NEVER stored in
    a record (it exists only as the picker's TRT-scan return value)

### Type word values [verified]
- 0 = ground (no record; dispatcher's ground branch)
- n plain (1-based .NME critter idx) = critter
- n|0x1000 = robot (0x4c69e4/0xA8 bank)
- 0x2000|n = TRT terrain-structure id (picker ground-scan return
  only; resolved by the dispatcher through the TRT bank)

### The writer census — 7 sites, ALL in FUN_00403938 [verified]
- w1 @0x403c87-0x403cf1 (the queue's 0x403c93): ROBOTS, gated
  [0x4edb88]==2 (network sessions only; D89: SP sets 0) AND robot ≠
  the local player's ([0x46cbd4]+[0x46cbdc] ≠ loop idx). Walks
  0x4c69e4/0xA8/[0x46ccbc] via the sprite-draw loop (visible
  robots only: screen bounds 0..0x23F ∧ [rec+0x4c6a60]≠0). Writes
  corner = tile x/y +0xB ([rec+0]/[+4] >>8), origin = the draw's
  screen coords, z = [rec+8]+0x21, w = h = 0x40,
  type = (idx+1)|0x1000. ⇒ SP writes NO robot rects — SP
  click-orders are ground/critter/TRT only.
- w2 @0x4056f1-0x405767: critter draw path — corner [esp+0xf4]/
  [esp+0x4], origin [esp+0x194]/edi−0x48, z = [crit+0x3E]+0x20,
  w = 0x3C (!), h = 0x40, type = idx+1.
- w3 @0x4058b8 + shared tail 0x4058f2-0x405926: z = [crit+0x3E]
  raw, w = h = 0x40, type = idx+1 (tail: corner Y +4 @0x4058f2,
  z +0x10 @0x4058fe, h +0x18 @0x40590b, type @0x405918,
  count++/cap @0x40591e).
- w4 @0x405c4d: z = [crit+0x3E]+0x10 → tail; w = h = 0x40.
  (Corner [esp+0xf0]/[esp+0x120]; the 0x405961 prologue stores
  [crit+0x0C] RAW Q8 — no >>8.)
- w5 @0x405f7b-0x405ff0: z = [crit+0x3E]+0x10, w = h = 0x40;
  corners [esp+0xec]/[esp+0x11c] (the 0x405c9b prologue:
  [crit+0x0C]/[crit+0x10] >>8 — tile units).
- w6 @0x406142-0x40618b: z = [crit+0x3E]>>8 (the ONLY scaled
  writer — Q8→tile), w = h = 0x40 → tail.
- w7 @0x4062c6-0x4062f3: the POI draw path (0x406190 prologue —
  [crit+0x0C]/[crit+0x10]>>8 corners, [rec+0x4d0008]:=1 visible
  stamp, 0x4eddac bank); state filter {6,7} only (dormant 0xB
  still gets a rect); z = [crit+0x3E] raw via tail, w = h = 0x40.
- All critter writers: walk the .NME bank 0x4cff98/0x7E/
  [0x46cc2c] (§7j.17/§7j.18), state word [crit+0x0A] ∉ {6,7,0xB}
  (except w7: {6,7} only), dispatched per-kind by the jump table
  @0x40391c (kind word [crit+0x00], 1..6) — path→site attribution
  beyond the above not traced (bounded unit).

### Reader family 1 — the PICKER FUN_00419943 [verified]
- Called only by the dispatcher @0x41068e. Prologue pre-transforms
  the mouse (0x419951-0x41998c): view-space mx/my from
  [0x4ede00]/[0x4ede04] +0x40 iso terms.
- Hot-rect loop i = 0..[0x46ccd8]−1 (reads record i):
  w = [+0x14], h = [+0x18], cx = [+0x08]+w/2, cy = [+0x0C]+h/2;
  HIT iff |mx−cx| < w/2 ∧ |my−cy| < h/2. Priority =
  FUN_0041ebf8(mx−cx, my−cy) = OCTILE distance
  max(|dx|,|dy|)+min(|dx|,|dy|)/2; keeps the min; early-out <4.
  Returns i+1 (= the record's stored type id).
- Ground fallback (no hit): iso back-transform
  (mx−0xF0)·[0x4ede54]/0x1E0 EXACT (the D83 EXD pick-twin
  FUN_0002a271 form) + camera [0x4edde4]/[0x4edde8] → world point;
  then scans ACTIVE TRT recs (active dword @0x4cccf8+i·0x20 ≠ 0,
  count [0x46ccd4]): box windows X ∈ (x·32−z·32−0x10, +0x40),
  Y ∈ (y·32−z·32−0x10, +0x40) using TRT x/y/z @+0x14/+0x18/+0x1C
  (§7j.14 pins); hit → returns 0x2000|(idx+1); else 0.

### Reader family 2 — the ORDER DISPATCHER FUN_00410644 [verified]
- Called by MissionShell @0x448021. Gates: [0x4ede00]/[0x4ede04]
  ≠ −1 (fresh mouse), [0x4ede14] ≠ 0, [0x4edba0] == 0, mx < 0x1E0
  (left of the sidebar).
- Calls the picker; re-transforms the mouse to view space
  ((mx−0xF0)·[0x4ede54]/0x1E0; y via [0x4edd54]·15·32/[0x4ede54] +
  (my−0xF0)·[0x4ede54]/0x1E0 + 0x15).
- picked == 0 → GROUND order: ORDER TARGET 0x4dd484/88 =
  camera + transformed mouse, z 0x4dd48c = 0, type cell
  [0x46cc00] := 0.
- picked & 0x2000 (bit 13; picker TRT-scan only): id &= 0x1FFF;
  reads TRT rec(id−1) through the −0xC-bias base 0x4cccec
  (= fields +0x14/+0x18/+0x1C of rec(id−1); the §7j.28 ledger
  gloss "critter 0x4cccec/0x20" is hereby corrected — the bank is
  TRT, "0x2000" is the target-descriptor class name); x/y/z each
  ×0x20 +0x10 (box CENTER: the picker tested −0x10 corners) →
  0x4dd484/88/8c; [0x46cc00] := the raw flagged id.
- else: type = [id·0x20+0x4787b8] → [0x46cc00]; bit-12 test
  (byte 0x46cc01 & 0x10):
  - ROBOT: corner [+0]/[+4] → 0x4dd484/88; z [+0x10] → 0x4dd48c
    (0x41077d-0x410795).
  - else CRITTER: corner [+0]/[+4] → 0x4dd484/88; z =
    FUN_004128ec(type−1) >>8 +0x15 → 0x4dd48c.
- Tail: [0x4ddb20] |= 2 (order-pending latch bit — NEW pin; the
  EXW cell of the D83 EXD order-active family) and
  [0x4ede00] := −1 (consume the click).

### Implications for the P4.2 seams [hypothesis, seam-relevant]
- SP click-orders can never be robot-targeted (w1 is network-only)
  — the E engine's click seam must not fabricate robot-targeted
  orders in SP scenarios (S2's ground-order seam is correct).
- Order-target units are per-class: robot/critter = tile ints
  (+0xB robot corner bias, +0x15 critter z bias, +0x21/+0x20/
  +0x10 z biases per writer); TRT = field·32+16. The E-side order
  seam must reproduce the per-class formulas exactly against the
  D82 ORDER-TARGET cells 0x4dd484/88/8c.
- New watch candidates: [0x46ccd8] count + the type cell
  [0x46cc00] + latch bit [0x4ddb20]&2 (TI/T4 family; not yet in
  watches.toml — additive when the harness needs click parity).

## 7j.32. The .BDG TEMPLATE-BANK READERS — plane↔mirror mapping CLOSED; +0x3E/+0x42 = DEAD EDITOR PAYLOAD; the 0x1E-B mirror-record grammar + the objective-height family (2026-08-22, worker ce347a0e claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run; corpus probes read-only over game-data, scratch /tmp)

Closes the 7j.25 open item ("the @+0x3E/+0x42 readers — which
bank feeds which restore word"). HEADLINE: **+0x46/+0x4A are the
only banks any code reads; +0x3E/+0x42 are loaded into the arena
and NEVER consumed by any .text site — they are the editor's
CURRENT-state stamp payload, and the shipped .TOT/.DAT already
carry that state (bank1 ≡ shipped TOT word at footprints
434/435 cells, bank3 ≡ shipped DAT byte 434/435 on ZONEA/M1)**.

1. **The loader's bank DISK ORDER is interleaved vs the slot
   order [verified 0x41a71d..0x41a782]**: the four reads march
   the arena cursor [0x46ad5c] and store the pointers in the
   order +0x3E (1st read, 0x41a727), **+0x46 (2nd, 0x41a742)**,
   **+0x42 (3rd, 0x41a75d)**, **+0x4A (4th, 0x41a77c)**. So the
   on-disk template-bank order is `+0x3E, +0x46, +0x42, +0x4A`
   (FORMATS §16 refined).
2. **The reader census [verified, three independent scans]**:
   (a) absolute-address scan for the slot addresses 0x4dee30/
   0x4dee34 (rec+0x3E/+0x42 across all 282 recs) — the ONLY
   hits are the loader's four stores + the restore's +0x46/
   +0x4A loads (0x41ab59/0x41ab72/0x41ab8a); (b) displacement
   scan for `[reg+0x3e]`/`[reg+0x42]` forms over the whole
   objdump — zero type-table hits; (c) arena scan (0x46ad5c/
   0x46ad60) — loader-only + the boot arena allocator 0x41d9c8.
   The complete 0x4dedf2 traffic census (20 sites): loader ×5,
   footprint stamper 0x41a857 (W/H), destroy resolver 0x41a9d7,
   chain-walk ×4 (W/H/D/chain/score), rubble-draw 0x408ce9 +
   minimap 0x41f65e (W/H only), MissionShell ×5 (0x448804
   script-step W/H + destroyed-bit 0x40 test; 0x448938/0x448b24
   objective counters; 0x448bfb/0x448d3b FUN_00448b80) — NONE
   touch the +0x3E/+0x42 slots.
3. **The destroy-restore mapping re-verified instruction-exact
   [0x41a9c3..0x41ac0b]**: z-loop runs z0..min(z0+D,8) (bound
   cell [esp+0x48], 7-vs-8 clamp at 0x41ab0c); per cell the
   template linear index is `(z'·H+i)·W+j` with z' = z−z0
   (accumulator form: (k·H)·W + i·W + j, k per z-iteration —
   same value); **+0x46 bank word → the TOT-mirror plane word
   @0x4796bc+0x1E·tile+2·z** (mirror word addr starts 2·z0 at
   0x41aa33, +2 per z); **+0x4A bank word → seen byte
   @0x4796cc+0x1E·tile+z := (word==0) AND DAT volume byte
   @[[0x4edd58]+z·w·h+tile] := word&0xFF** (loaded twice,
   0x41ab72 and 0x41ab8a). Tile = (y+i)·w+(x+j); **H is the
   y-extent, W the x-extent**; [0x4eddec]=map w, [0x4eddf0]=h
   (also bounds-checked by the chain walk 0x41b803/0x41b920).
4. **Corpus role proof [verified, ZONEA/MISSION1: 213 instances,
   435 footprint cells]**: bank1(+0x3E) word == the SHIPPED TOT
   plane word at (x+j, y+i, z0+z') in **434/435** cells; bank3
   (+0x42) word == the SHIPPED DAT byte in **434/435**. bank2/
   bank4 match only 11/435 and 155/435 (coincidences where
   building type == ground type). Global value domains (37
   files, 7907 active recs, 67269 words/bank): b1/b2 = tile
   words (max 1868/1802, 34%/52% zero); b3 = DAT-domain words
   (max 102, 91% nonzero, top 1/2/10/3); b4 ≤ 512 (49% zero
   → seen=1). READING: the four banks are a 2×2 —
   {CURRENT-state pair (+0x3E TOT words, +0x42 DAT volume),
   UNDER-terrain pair (+0x46, +0x4A)}. The editor stamped the
   CURRENT pair into the shipped .TOT/.DAT at the footprints;
   the game never needs to re-stamp, hence zero readers. The
   destroy restore re-instates the UNDER pair into the runtime
   mirror/seen/DAT volume. **The "runtime spawn-stamp pass"
   hypothesis is RETIRED** — no such code exists; buildings
   arrive pre-baked in the mission files.
5. **The one mismatch is a faithful overlap artifact**: tile
   (14,29,z1) ZONEA/M1 is covered by BOTH .POS slot 97 (type
   63, 1×2×3, b1=806) and slot 207 (type 0, 1×1×1, b1=53) —
   the shipped TOT holds 53 = LAST-SLOT-WINS; layered destroys
   there restore per-type templates (slot 207's b2=1189 already
   contains slot 97's building). Editor data quirk, not an
   engine rule.
6. **THE 0x1E-BYTE MIRROR-RECORD GRAMMAR (0x4796bc, per tile)
   [verified — unifies three prior families]**: `+0x00..+0x0F`
   = the 8 TOT plane words (+2·z); `+0x10..+0x17` = the 8 SEEN
   bytes (+0x10+z — the restore's 0x4796cc writes, seen base =
   mirror base+0x10); `+0x18` = scorch (FUN_0042223c/87, 7j.8/
   7j.9); `+0x19` = type-DB variant byte 0x4796d5 (7j.12
   stamper FUN_00422fd1, variant<<4); `+0x1A` = door/type byte
   0x4796d6 (7j.12, bit7=0x80 door flag); **`+0x1B/+0x1C` = the
   OBJECT-HEIGHT pair (z0, z0+D)** — NEW: stamped per footprint
   tile by the MissionShell objective pass FUN_0044889a
   (0x448963/0x448975: z := instance z@+8, then z+D via
   type-rec byte@+6) and CLEARED (both := 0) by the destroy
   notify FUN_00448b80 (0x448c25/0x448c2c); read by the
   draw/occlusion family (e.g. 0x4068ec/0x406907/0x406a0e/
   0x406a1a — the intact-vs-rubble tile pick alongside the
   plane-0 word test 0x406891). `+0x1D` = ZERO traffic in
   .text (padding/unused). This RETIRES the scattered "+0x18/
   +0x19/+0x1A tail bytes" open items as ONE record grammar
   (MISSIONVIEW §8 cross-ref); +0x1D stays formally open.
7. **FUN_0044889a + FUN_00448b80 = the OBJECTIVE-BUILDING
   family [verified]**: FUN_0044889a (zone/edition gate
   [0x4edd8c]==7 at 0x4488be — the ZONEG path; the cell is the
   zone index 1..7 per 7j.21, 7j.30's "edition" gloss): counts
   instances with type id&0x3FFF ∈
   [0x44,0x47] into **[0x46cce0]** (the objective counter) and
   stamps the +0x1B/+0x1C heights over their footprints; the
   edition≠7 path (0x448c94) is the SCRIPT-driven objective
   walk (0x4eaaee count / 0x4eaaf2 instance idx / 0x4eab0c
   anim word, script tables 0x4557f8/0x456810, special code
   0x1388=5000 two-word script op) — head-decoded only, out of
   scope here. FUN_00448b80(instance idx) — the "notify" call
   of the 7j.25 destroy tail — SP-only ([0x4edb88]==2 skips at
   0x448b91): decrements [0x46cce0], clears the footprint
   heights, and at ZERO fires **FUN_004239ef(0x28,3) +
   FUN_004239ef(0x29,3)** (the 7j.20 beacon-armer SFX family)
   and stages **0x46cd00:=3, 0x46ccfc:=0x20, 0x46ccc4:=0x32**
   — the extraction-arm family lights up when the last
   objective building falls (7j.20 cross-ref).
8. **FUN_0041bc1c death stamp (TRT/turret resolver, FORMATS §14
   family — new detail)**: on hp ≤ 0 the mirror plane word :=
   word@[0x454a04+4·zone] (per-zone rubble word table),
   seen := 1, DAT volume byte := 0, then debris
   FUN_00420608(x·0x20,y·0x20,z·0x20, kind ecx=0xF, param −1,
   delay 0) + splash FUN_00424355 at the FUN_0041bd78 water-z
   probe — the same death shape as the .BDG tail (7j.25) minus
   the restore (turrets have no under-bank).
9. **Corpus verdict**: unchanged — nothing destroys in the
   gates; the restore/height families stay off the corpus
   path. E-side seam: NONE required for the template banks
   (skip or keep the dead payload — arena layout is
   reader-free; keeping the load is the faithful option). New
   watch candidates if objective parity is ever needed:
   [0x46cce0] counter + the +0x1B/+0x1C height bytes via the
   0x4796bc row.

## 7j.33. THE .BLD RECORD WALK + THE MISSION FILE-FAMILY CENSUS — .BLD is EDITOR-ONLY (zero runtime readers); the compiled-pair relationship to .BDG (2026-08-22, worker fc88ecf3 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run; corpus probes read-only over game-data, scratch /tmp/opencode)

Ticket: the .BLD names/graphics sibling of the .BDG reader
(§7j.32 residual; FORMATS §17). HEADLINE: **the EXW runtime
never opens a .BLD file — there is no .BLD loader.** The byte
sequence "BLD" (case-insensitive, dot or not) occurs ZERO times
in BEDLAM.EXW, BEDLAM.EXD, BEDLAM.EXE, cd-root copies, and all
three DIRECTX exes. The only `.BDL` string in EXW DGROUP is
`"SAVED.BDL"` @0x4597d6 — the SAVEGAME file (with "HISCORES",
"Player", "SAVEGAME", "EMPTY" neighbors), unrelated to the
EDITOR\ZONE* library. The queue-note hypothesis "the loader
call should sit near the .NME/.POS/.BDG loader family" is
therefore RETIRED.

### The complete runtime file-extension census [verified]

- **Mission family loader FUN_0041dc5a** (post-call of the path
  builder FUN_0044670c): loads in order `.TOT`(0x4587d9)→buf
  0x4dca0c/[0x4ede20], `.DAT`(0x4587de)→0x4dca0c/[0x4edd58],
  `.CGR`(0x4587e3)→0x4dca8c/[0x4edd60], `.BIN`(0x4587e8)→
  0x4dca8c/[0x4ede1c], `.MIN`(0x4587ed)→0x4dca8c/[0x4edd9c],
  then the language gate `cmp [0x4eba1c],1`: ==1 → `.LNG`
  (0x4587f2) else `.LNK` (0x4587f7), and `.PAD`(0x4587fc) later
  @0x41de44. All eight tags are ONE contiguous 5-B-stride
  string table @0x4587d9..0x4587fc (8 entries, no ninth).
- **Second load site FUN_0044461b-family @0x446623-0x446677**:
  `.TOT`(0x459795)/`.BIN`(0x45979a)/`.DAT`(0x45979f) re-loads
  against the same buffer/cell pairs (title/overview path).
- **Path builder FUN_0044670c**: appends `"EDITOR\"` (0x4597a4)
  + `"ZONE"` (0x4597ac) + zone char (`[0x4edd8c]+0x40` →
  'A'..'G') + `"\MISSION"` (0x4597b1) + mission number; the
  caller appends the extension. The game therefore reads the
  EDITOR\ZONE{A..G}\MISSION{n}.{ext} tree directly. A second
  identical string triple sits at 0x4597ba..0x4597c7.
- **Other loaders already pinned:** `.NME` 0x457a57 (§7j.18),
  `.TRT` 0x457a5c (§7j.15), `.POS` 0x457a64 + `.BDG` 0x457a69
  (§7j.25, FUN_0041a4f8, mission-load chain 0x447b3a..0x447c00),
  `.MRK` 0x457a34.
- **Consequence (editor-only set):** of the 20 extensions
  shipped in EDITOR\ZONE*\, exactly SIX never appear as strings
  anywhere in EXW: **.BLD, .CTG, .COL, .MAP, .PTH, .TXT**.
  Notably .CTG (FORMATS §6) is ALSO never loaded — the runtime
  picks ONE of .LNK/.LNG per language gate, never .CTG. The
  FORMATS §0.1 extension census gains this split (§0.2 there).

### The .BLD record grammar [verified, corpus-anchored]

Probes: sequential walk driven by the sibling .BDG's (W,H)
(below) + name-run scans over all 43 files; ZONEA/C/D/E + ZONEF
M2/M4/M7 walk PERFECT (name@+0x60 printable, all four bank-slot
heads == the BDG bank[0] values, bank-1 array == bank[1:…]) —
ZONEA 197/197, ZONEC 132/132…163/163, ZONED 197..198/…, ZONEE
229..245/…, i.e. 7 286 of 7 907 records byte-validated before
the two documented desync classes.

- **Header (12 B, constant in all 43):** u16 "54" (0x3435) +
  u16 1 + u16 1 + u16 0 + u16 {1,3,5} + u16 0. The 5th u16 =
  1 (zones A/B/C/D/F-zone-file/G), 3 (zone E), 5 (zone F
  mission files) — asset-set/palette id [open].
- **Records start at +0xC, one per BDG NON-EMPTY record,
  same order** (ZONEA/M1: BLD 197 = BDG 282 − 85 tail EMPTYs;
  the EMPTY 2-B BDG rows have NO BLD counterpart).
- **Record length = 137 + 64·W·H + tail_extra**, W/H/D from
  the same-index BDG record. This subsumes the FORMATS §17
  "201 + k×64" name-delta observation (201 = 137+64·1; every
  observed delta ≡ 201 mod 64). The old "64-B extension
  blocks" are NOT free-floating blocks — they are the four
  template-bank slots below.
- **Record layout:**
  - +0x00 u32 head: [+0]=H (y-extent; 197/197 for H>1),
    [+4]=hp, [+8]=chain, [+0xC]=type (all == the BDG values;
    ZONEA census), [+0x10..0x2F] flag/count words, mostly
    0/1/2 [open]. W and D are NOT stored anywhere in the
    record (u32/u16 sweeps over +0x00..0x5F found no match).
  - +0x60 name, NUL-padded, ~33 B field (data resumes +0x81).
  - +0x81 FOUR template-bank slots, each **16·W·H bytes**:
    slot+0 u16 = bank[cell 0]; slot+2.. = u16 array
    bank[1 : 1+min(n−1,16)] (n = W·H·D; first 16 after cell 0
    — n>17 truncates, e.g. "small plane 2" n=18 keeps 16);
    rest zero pad. The slot values ARE the four BDG banks
    (+0x3E/+0x42/+0x46/+0x4A, §7j.32) — verified equal at
    every walked record incl. all four heads.
  - then a **variable tail** (≥8 B): standard = two u32(1)s;
    "sub tunnel wall" (ZONEB/M1 idx 86) = 12 B (1,5,4,0x1194);
    "small plane 2" (idx 92) = 16 B (1,1,1,0xFFFF…);
    ZONEA's last record "EXIT POINT" carries 320 B extra
    (all zero) — tunnel/animated/exit annotations [open].
- **File end:** zero fill after the last record (≥12 B; e.g.
  ZONEA 332 B). There is NO record terminator and NO stored
  record count: **.BLD is not self-delimiting** — a parser
  needs the sibling .BDG's per-record (W,H) (or a name-scan
  heuristic). This answers FORMATS §17's "exact record
  terminator": none exists.

### The compiled-pair relationship [verified]

BLD record j ≡ BDG non-empty record j: same head scalars, same
four template banks, same order — .BLD is the editor SOURCE
format and .BDG the compiled runtime export (u16 fields widened
to u32, name text dropped, banks compacted 2·W·H·D words,
effects/chain retained). The r=0.985 size correlation
(FORMATS §16) follows. Zone-level BLDs have NO zone-level BDG
(only mission BDGs ship) and are byte-shared across the known
zone pairs: MISSIONA.BLD ≡ MISSIONF.BLD, MISSIONB.BLD ≡
MISSIONG.BLD (md5-verified) — the A/F, B/G sharing pattern
extends to the BLDs (mission-level BLDs are NOT shared).

### Desync classes [open, bounded]

1. ZONEB/ZONEG (7 files each) + ZONEF/M6: the walk desyncs at
   a handful of records (~j=87/125/154) where the BLD record
   is LONGER than 137+64·W·H — ZONEB/M1 has exactly two
   nonconforming name-deltas (405 after "sub tunnel wall",
   537 after "small plane 2") = the variable tails above;
   the remaining ZONEB/M1 "bad" rows are cascade effects of
   stepping past those two records with the BDG-derived
   length. Re-walking with per-record tail resync (scan
   forward to the next plausible name@+0x60) should close
   these; not done in this unit.
2. ZONEC/M2+M3: BDG non-empty count exceeds the BLD name count
   by exactly 1 (162 vs 161, 163 vs 162) — one record with a
   sub-3-char or empty name breaks the name-scan (the BDG
   walk itself was NOT re-run with the deterministic parser
   for those two files) [open].

### E-side seam consequences

NONE. The runtime never reads .BLD; the engine's mission
loader needs no BLD path, and no watch/injection surface
touches it. The FORMATS §17 record grammar is documentation-
only (tooling for corpus inspection); the .BDG loader path is
already covered by §7j.25/§7j.32.

## 7j.34. THE MIRROR-RECORD TAIL CENSUS — the DOOR ANIMATOR family decoded; +0x19/+0x1A semantics unified; the 0x4dcae8 rect-grammar conflict resolved; +0x1D confirmed padding (2026-08-22, worker a42c6027 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run, no corpus read)

Ticket: the MISSIONVIEW §8 type-DB tail producers — the
+0x1a/+0x1b/+0x1c bytes of the 0x4796bc mirror rows, the
§7j.12-vs-§7j.32 door-byte re-verification, and the remaining
writer/reader anchoring. Method: absolute-address census of
0x4796d4..0x4796d9 over the full .text objdump (71 sites, every
access in this family is the absolute `[reg+0x4796dX]` or
`[idx*2+0x4796dX]` form — no displacement aliases), then bounded
decodes of the seven container functions. HEADLINE: **+0x19 is
the door/scenery TARGET-TAG byte and +0x1A is a packed
{bit7 phase, bits0-6 running frame counter} — the "door byte"
is one half of a 15-frame sliding-door animation machine
(FUN_00423081, the MissionShell epilogue tick @0x44808f) that
writes door-frame DAT volume bytes 0x40..0x5E and SHIFTS the
tile z-stack when a door finishes opening/closing.**

1. **The 0x4dcae8 45×0x10 rect-list grammar RESOLVED** (the
   §7j.12-vs-§7j.21 conflict; verified from BOTH consumers'
   loop/register arithmetic — the row-start table 0x4ea900 is
   indexed by word@+4+dy, the column is word@+2+dx, the loops
   run over word@+6 (x-extent w) and word@+8 (y-extent h)):
   `{+0 state, +2 x0, +4 y0, +6 w, +8 h, +0xA variant/type
   byte, +0xC countdown, +0xE SFX-due flag}`. §7j.12's map was
   CORRECT; §7j.21's restatement "{+2 x, +4 w, +6 y, +8 h}" was
   WRONG (w/y/h permuted) — row 4504 rewritten. The state
   domain: 0 = inactive/end (all three walkers stop at the
   first 0); **1/2 = SCRIPTED doors** (toggled by pad scripts);
   **≥3 = AUTO-CYCLING doors** (timed, animate forever). The
   §7j.12 "records with word@+2 (type) ≥ 3" qualifier cited the
   WRONG FIELD — the gate reads **word@+0 (state) ≥ 3**
   (0x423001 `cmp eax,3; jl skip`, eax = dword[0x4dcae6+i·0x10]
   >>16 = word@+0; same gate in the animator 0x4230c2).
2. **FUN_00422fd1 (load call 0x447ba3) re-verified
   instruction-exact**: state ≥ 3 rects stamp every W×H tile:
   `+0x19 := byte@rect+0xA << 4` and `+0x1A := 0` (state==3) /
   `0x80` (state ≥ 4) (writers 0x423061/70/78 — the §7j.12
   VALUES were right, the fields now anchored). So at mission
   load an auto door starts with bit7 = its initial phase and
   +0x19 = its first target frame count.
3. **FUN_00423081 = the DOOR/SCENERY ANIMATOR TICK** (1342 B,
   sole caller MissionShell @0x44808f — the epilogue slot right
   after FUN_0042205c@0x448080, FUN_00422cc2@0x448085,
   FUN_00422a9c@0x44808a) [verified 0x423081..0x4235bf]:
   - state ≥ 3 (auto doors): countdown word@+0xC ≠ 0 →
     decrement, done; at 0, if word@+0xE ≠ 0 → door SFX
     FUN_0043a48e(bank ELEV1 [0x4edfb0], y0·32, x0·32, 0, 2)
     once, clear +0xE; then animate the rect tiles.
   - state 1/2 (scripted doors): animate directly (no
     countdown/+0xE path).
   - per tile: SKIP if low7(+0x1A) == +0x19 (target reached —
     the door is settled); else walk the tile's plane words
     DOWN from the top for the first nonzero level (bit7 SET
     starts at plane 5/edx=6, bit7 CLEAR at plane 6/edx=7) and
     write the DAT volume door-frame byte at that level via
     [0x4edd58] + row-table + z-plane dwords: **bit7 SET:
     0x40 + 2·nibble (EVEN series, table [edx·4+0x4eaad0]);
     bit7 CLEAR: 0x5F − 2·nibble (ODD series, table
     [edx·4+0x4eaacc])** — the tile graphic slides through the
     0x40..0x5E door frames (a NEW documented DAT byte domain
     beside the §7j.32 b3 ≤ 102 census); then
     low7(+0x1A) += 1 (bit7 preserved, 0x4231aa/0x423330/
     0x423405/0x42355a).
   - nibble wrap (test 0xF == 0, every 16 frames) → the FINISH
     PAIR: bit7-set path FUN_004236c6 + FUN_00423740; bit7-clear
     path FUN_00423650 + FUN_004235fb.
   - completion (low7 == +0x19) of a state ≥ 3 door → the
     AUTO-TOGGLE @0x4231e6: bit7 XOR 0x80, +0x19 :=
     byte@rect+0xA<<4 (re-target), countdown := 0x14 (20),
     +0xE := 1, and SFX FUN_0043a48e(bank ELEV2 [0x4edfb4],
     y0·32, x0·32, 0, 2) once per tick (latch [esp+0x1c]).
     Auto doors cycle open↔close forever with 20-tick pauses.
4. **The FINISH pairs = the z-stack moves** [verified]:
   - **FUN_004235fb = the stack DROP** (an OPEN completes):
     plane words word[z] := word[z+1] for z=0..6, plane 7 := 0;
     seen bytes shift likewise, seen[7] := 0 (0x423619..).
     The door's level LEAVES the tile stack — the tile becomes
     passable/empty at that level.
   - **FUN_00423740 = the stack PUSH-UP** (a CLOSE completes):
     plane words word[z+1] := word[z] for z=0..6 (the stack
     rises; the bottom slot keeps its old word), seen likewise;
     then IF the south (y+1) AND east (x+1) neighbor tiles both
     have +0x19 ≠ 0 → plane word 0 := 0 (0x4237c5/0x4237da are
     the NEIGHBOR +0x19 reads — the last two undocumented tail
     sites). At map edges the shift runs unconditionally.
   - FUN_004236c6 (close DAT stamp): walk planes 7..0 for the
     first nonzero (residual edx); DAT byte at
     [row+col+[edx·4+0x4eaad0]] := 1; if edx < 6 also
     [edx·4+0x4eaad4] (one level up) := 0.
   - FUN_00423650 (open DAT stamp): same walk; if edx ≠ 0 →
     DAT byte [edx·4+0x4eaacc] := 0; ALWAYS the extra-plane
     byte [[0x4eaae8]] := 0 — 0x4eaae8 = dword index 8 of the
     0x4eaac8 z-plane table, i.e. a NINTH plane offset exists
     (beyond the 8 stack levels).
   - The z-plane dword table family is ONE table at 0x4eaac8
     (indices 0..8+): 0x4eaacc = index 1, 0x4eaad0 = index 2,
     0x4eaad4 = index 3 — the ±1 entry shifts between the
     bit7 paths are level+1 arithmetic, not separate tables.
5. **FUN_004223b8 = the SCRIPTED DOOR stepper** (86 callers,
   ALL in the 0x433xxx-0x435xxx FUN_00433980 pad-script family;
   verified 0x4223b8..0x4225cf): args (rect idx, wanted ∈
   {1,2}); guard state ≠ wanted ∧ state < 3 (scripted doors
   only); wall-strip redraw FUN_004245c9(x0·0x20+w·0x10,
   y0·0x20+h·0x10); per rect tile whose low7(+0x1A) == +0x19
   (the §7j.21 "door-tile words" test = the ANIM-COMPLETE
   gate, now explained): wanted==1 → FUN_004235e4 (+0x19 :=
   word@+0xA<<4, +0x1A := 0x80, 0x4224cd) / wanted==2 →
   FUN_004235bf (+0x1A := 0, 0x422571) — the two byte-helpers
   (0x4235bf/0x4235e4, verified: same row-table tile math,
   bl → +0x19, dl ∈ {0, 0x80} → +0x1A, sole callers are these
   two sites); state word := wanted; SFX FUN_004239ef(0x23,3)
   + FUN_0043a48e(ELEV1 [0x4edfb0]) once per transition. A
   scripted door animates to the new target and stops (the
   auto-toggle is the ≥3 path only).
6. **The renderer readers (FUN_00403938)** [verified
   0x406bd6..0x406c6e]: the strip draw reads +0x19 of the tile
   (0x406bd6) and, when 0 with y>0, of the NORTH neighbor
   (tile − map_w, 0x406bf9) — door-strip adjacency; then
   +0x1A (0x406c3b): low7==0 → the plain path (0x4067e4);
   bit7 clear → 0x4067cf; **bit7 set ∧ counter ≠ 0 (mid-anim)
   → the tile draws with Y-bias −nibble·0x500** (0x406c5c —
   the door's on-screen SLIDE). The +0x1B/+0x1C intact-vs-
   rubble draw pick (0x406891/0x4068ec/0x406907) is §7j.32's,
   unchanged.
7. **The other +0x18/+0x19 readers anchored**: (a)
   FUN_0040b9f6 0x40bc60 — robot state==1 on a scorched tile
   (+0x18 ≠ 0) → FUN_004100b7(robot, 0x14) (the fire-damage
   family), else the pod-countdown word@0x4c6a14 −= 10 path
   (§7j.20): scorch under a standing robot HURTS it — the
   +0x18 reader that closes scorch→damage; (b) FUN_00410823
   0x4110cb — the fire controller stores a quantized (<<8)
   anchor into anim-record word @robot+0x4c720e when standing
   on a +0x19 tile (the 7j.17 "reposition" note now
   instruction-anchored); (c) FUN_004186fc 0x418735 — the
   §7j.17 standing-on-scenery check (+0x19 ≠ 0), unchanged.
8. **The +0x1B/+0x1C second stamp/clear pairs** [verified]:
   FUN_0044889a stamps the OBJECT-HEIGHT pair in TWO identical
   W×H walks (0x448963/75 + 0x448b4f/61 — the second computes
   +0x1C := z0 + type byte@+6 via `add cl,ch` @0x448b5e);
   FUN_00448b80 clears it in TWO walks (0x448c25/2c +
   0x448d65/6c). Same grammar as §7j.32 documented; the row
   now carries all four sites.
9. **+0x1D (0x4796d9) = ZERO traffic CONFIRMED**: every
   access to the tail in the entire .text is an absolute
   0x4796d4..0x4796d8 form (71 sites census); 0x4796d9 has
   zero. Padding/unused — the §7j.32 "[open]" tag closes.
10. **Corpus-path verdict**: doors never animate in the gates
    (no pad scripts step on door triggers; ZONEA/M1's rect
    list ships zero state ≥ 1 records — the stamper's
    45-record walk terminates immediately), so the engine seam
    stays NONE this unit (never-invent). P4.2 note: a door
    scenario would need a scripted .PAD step-on through
    FUN_00433980 (the S2/S6 seam pattern); the watch surface
    is +0x19/+0x1A (+0x1B/+0x1C per §7j.32) via the 0x4796bc
    row form, and the DAT volume door-frame bytes 0x40..0x5E
    are a divergence class of their own.

## 9. Open items (next slices)


0a. ~~The critter/POI/exit LOADER section inside FUN_00416458~~ —
   CLOSED 2026-08-21 §7j.18: .NME is the sole feeder (8 fixed
   sections → critter states 2/1/5/4/3/6/7 + 4 POIs/record,
   corpus-exact 37/37); FUN_0041fa51 = the exit-pad activator
   (runtime, from .PAD slots); FUN_0040db9e + FUN_00449c94 +
   the 0x4eb8b8 census folded. The exit runtime CLOSED
   2026-08-21 §7j.19: FUN_0041fbb1 = the escape-craft animator
   (exits + dropship + pods; the "+4 kind" is a PHASE),
   FUN_00433980 = the zone pad-trigger script dispatcher (case
   0x1B = the exit activation; elevator rides/messages/doors),
   FUN_0041faf0/FUN_0041fb4b = the dropship/pod spawners, and
   the [0x4eba0c]/[0x4eba10] censuses are CLOSED (§7j.19 item
   5). The beacon writer FUN_004247b5 + probe FUN_004248c8 +
   the 0x4c6a10 pod-countdown producers are CLOSED §7j.20
   (pad-script armer + spread picker + deploy stagger). The
   0x425xxx arrival-producer family is CLOSED §7j.21
   (FUN_00425da4 = the elevator-ride stager; the ride armer +
   walk + marker draw bound to the record structure). Still
   open from that head: the full per-zone FUN_00433980 case
   table and the per-zone record↔pad arm mapping (deferred
   until P4.2 needs it).
   Projectile type 0x69 vs the FUN_00419aff damage table
   remains open (low priority). The FUN_00410823 weapon-anim
   machine is CLOSED 2026-08-21 §7j.22 (the full per-type
   tick — bullets 2..4 / shell 5 / artillery 9..0xB /
   ballistic {0xE,0xF,0x13,0x17,0x1A,0x1F} / rocket 0x24 /
   homing 0x29, the artillery burst tables, the 0x4e66b8
   smoke-trail bank, and the two actor hit-test front doors;
   FUN_004190bc re-identified as the critter hit applier,
   killing the §7j.15 panel hypothesis). Family remainders:
   the 0x4e66b8 slot allocator CLOSED §7j.23, and the
   trail-ring draw pass CLOSED §7j.28 (with the whole
   mid-flight draw family, both banks).
0b. ~~The residual 0x4dd484 reader census~~ — CLOSED 7j.17
   (full writer/reader census in the ledger row).
0. ~~The isometric viewport draw chain~~ — DECODED 2026-08-21,
   docs/RE-EXW-MISSIONVIEW.md: init_tiles cache geometry + TOT→typeDB
   mirror, LNK as the per-frame tile animation link, BIN as the
   terrain sprite bank, FUN_00401471 blit codec, FUN_00403938 terrain
   loop, FUN_00401107 present window.
1. ~~The mission file-load + table-build pass~~ — DECODED 2026-08-21,
   amendment 7c: load_mission@0041dc5a (paths, TOT/DAT/CGR/BIN/MIN/LNK
   loads, y-line/z-base tables, ≥0x80 sweep, PAD 0xFF marks) + markers →
   robots (MRK word 3 = spawn z level). Engine seam: the P4 corpus gate
   builds Terrain from raw DAT+PAD+CGR bytes with these rules.
2. FUN_00440e45 (10661 B, GameMain call #2) identity — not the gameplay
   loop [verified negative]; likely the inter-mission shell (shop/map
   room) [hypothesis].
3. Phase semantics of robots()' extra passes (fields 0x4c6a16/18/88/8c)
   and the state 1 producers (patrol?).
4. ~~Sidebar order buttons beyond selection~~ — DECODED 2026-08-21,
   §6c: order keys 1..7 + the 7-row click strip (gate word +0x38+8k,
   bits word +0x6E, redraw countdown DAT_0046ccec consumed by the
   FUN_00403938 tail via FUN_00408403). The sidebar DRAW passes
   DECODED 2026-08-21 §6c.8 (FUN_00408403 rows, FUN_004072bf
   portraits, FUN_0040807f bars, FUN_004085ce score strip; banks
   GENERAL/SMLFONT/NUMBERS/SCANNER). The HP/armor bars + the score
   strip were DECODED 2026-08-21 amendment §7f and WIRED the same
   day (host-staged vitals + campaign session state, D52); the
   vitals PRODUCERS LANDED 2026-08-21 (amendment 7g + D53): hp/
   armor/hit_flash/alarm/shield family are hash-covered sim
   fields, `MissionSim::apply_damage` is the FUN_0040e230 SP core
   (the debris/death RNG interleaving included — 10 shared draws),
   the armor pad charge/bleed + the phase-0 shield family run per
   frame, and the sim hashes were re-pinned ONCE for that reason.
   Still host-seamed: the FUN_0040eba0 TILE-WORD PRODUCER (the
   type-DB mirror + probe-latch walk, DECODED 2026-08-21 §7h —
   the dispatch decode + the case-1/2/3/7 bodies landed as
   pickup_case/apply_pickup seams the same day; case 4
   remains the D52 seam), the 0x7d2 hazard caller, and the
   projectile callers. The dead/hit dither (FUN_00401ae6 + the
   0x4e6ed8 bank) was DECODED 2026-08-21 amendment §7i and WIRED
   the same day (D55: the noise ring + churn + the portrait-pass
   blit on the sim hit_flash field). Remaining open after
   that: the 0x4dc5d0 blink producer, and the keyboard-latch
   wiring (P2e button map). The map-overlay family
   (_DAT_004edba0/FUN_004089b1/FUN_00401107) was DECODED 2026-08-21
   amendment §7e and wired engine-side the same day. The
   name/count row TEXT landed 2026-08-21 (amendment 7d + D51: the
   loadout is host-staged session state; names via the pinned
   FUN_00420260 table).
5. ~~The 0x62-stride robot-type stats table at 0x4de664 — file source
   question~~ — CLOSED 2026-08-21, REFUTED by amendment 7d: NOT
   TABLE.BIN (that is the map-overlay backdrop bank, sole reader
   FUN_004089b1); the table is .bss session state written only by the
   shop FUN_00440e45 / save-load / MP lobby; player TYPE word@0x4edb90
   = 0 all SP (GameMain 0x41c34c); fresh campaign = all-zero loadout
   (money 4000, shop before every mission). Name switch FUN_00420260
   pinned (7d.5).
