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
| +0x2C | u16 | reinforcement/deploy timer slot (1, 1+(2000-m*1000/27), ...) | 0x4c6a10 |
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
[verified]; robots() consults `(&DAT_004796d4)[tile_type*0x1E]` for tile
damage classes. init_tiles also builds the 36×36 ISO viewport tile cache
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
4. **Click-on-robot arm**: the robot-sprite click handler (~0x433cbc,
   robot hit-test family) calls **FUN_004247b5(pos_x>>13, pos_y>>13, z,
   robot_idx)** — i.e. the order is armed AT THE CLICKED ROBOT'S TILE:
   ```
   if (word@0x4eabb0 != 0) return          // one pending order at a time
   word@0x4eabb2 = 0x197                    // validity window (407 frames)
   word@0x4eabb0 = 1                        // ARM (also: if alive==1, window=0)
   word@0x4eabb4/6/8 = order tile trio      // x, y, z
   robot[idx].state = 3
   FUN_004248c8(&tx,&ty); robot.pos = (tx<<13, ty<<13)   // spread-assign
   SFX 0x2A
   ```
   **FUN_004248c8** = spread/claim search: finds a free slot in the 12×u16
   claim array 0x4eabba, then a 12-case jumptable (table@0x424898) offsets
   the order tile by {0, ±1 on x, ±1 on y — the 8 neighbors + center
   variants} → each consumer gets a distinct destination tile near the
   click [verified asm 0x4248ca..0x424985].
5. **Consumption** (robots() §5): every robot within 6 tiles of the order
   tile (state ∉ {3,4,5}) gets state 4 + its own spread-tile target + stop
   distance 1e6 → walks per §5. The order expires via the MissionShell
   timer (or early once all robots are state-3/dead) → **FUN_0041faf0**
   clears 0x4eabb0/2 and stages the dropship animation target
   (0x4e6610-family = the reinforcement-drop visual) [verified].
   Net effect: one click = nearby robots converge + a reinforcement drop
   marker at the site [inferred semantics].

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
  move-target pair (DAT_0046cc30/60[idx]), order globals 0x4eabb0/2/4/6/8
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
   DAT_0046ccbc = robots_per_player: zone<3||zone==7 → 1, zone 3 → 2,
   else 3; overridden to `0x46cbe0` when `0x4edb88 != 0` — network
   player count) takes MRK record i VERBATIM:
   `pos = (x<<13)+0xF00, (y<<13)+0xF00`, `z = word3<<5 − 1` — so **MRK
   word 3 is the spawn Z LEVEL** (1 = ground), not a "type"; a word-3=0
   marker seeds z −1 and only settles on a height-≤3 ground tile
   (amendment 7b.4). The 0x62-stride stats copy, variant RandA()&3, the
   probe seeding, and the one settle probe match §§3/7b exactly.
8. **Order armer callers** [verified 0x433cfb]: the only call site of
   FUN_004247b5 is the robot-sprite click family ~0x433cbc (sprite hit
   test → arm at the clicked robot's tile, robot state 2 = selected
   writers nearby at 0x433c7f/0x433cab). FUN_00424a6f (the nearby call
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

## 8. Constants ledger (all [verified] unless tagged)

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
| sidebar select strips | [0x1E7,0x217]/[0x219,0x249]/[0x24B,0x27B] × y[5,0x35]; F1/F2/F3 latches 0x4edc0c/10/14 | §6c.2 |
| sidebar order rows | x[0x1E9,0x275] × y[0x57,0xB8]; row=(y-0x57)/14 clamp ≤6; keys 1..7 latches 0x4edc18+4k | §6c.3/4 |
| sidebar redraw flag | DAT_0046ccec countdown: set 2/3 by producers, dec+FUN_00408403 in the FUN_00403938 tail | 0x407205 |
| map-toggle strip | x[0x213,0x24D] × y[0x1B5,0x1CF]; MSpace latch 0x4edc08; writes 0x4eb8dc=5, toggles 0x4edba0 | §6c.1 |
| map overlay draw | FUN_004089b1: clear 0x4b000, TABLE.BIN img0 480×480 @(0,0), stamps row'=0x80+r+c−2z / col'=0xf0−2r+2c, markers 0x55..0x59; non-returning tail | §7e |
| territory variant | byte@0x4c420c+tile; zeroed 0x27d8 by MissionShell; 11×11 max-stamp rings 7..1 (dwords 0x454cf8) around robots (FUN_00408dcc ← robots() state 2) | §7e |
| MAPTRAN ramps | u32@(0x4dd464+4i) ← GAMEGFX\MAPTRAN{i}.TRN (256 B each, i 0..7); ramp[mask byte] = palette byte | §7e |
| PALTRAN ramps | u32@(0x4dd444+4i) ← GAMEGFX\PALTRAN{i}.TRN, slot 0 NULLed after load (MISSIONVIEW §8.2 producer closed) | §7e |
| LNK map lookup | cw = word@(0x45cdda + 2*w) — the LNK image doubles as the map type→mask index; masks = .MIN bank [0x4edd9c] (16 B/cw) | §7e |
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

## 9. Open items (next slices)

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
   vitals PRODUCERS land with the damage unit: the damage
   application FUN_0040e230 + its death/debris/RNG interplay
   (7f.5), the armor pad charge/bleed (7f.7), the health/shield
   pickups (7f.6 cases 2/3) — promoting hp/armor to real sim fields
   then re-pins the sim hashes deliberately. Remaining open after
   that: the 0x4dc5d0 blink producer, the dead/hit dither
   (FUN_00401ae6 + the 0x4e6ed8 bank), and the keyboard-latch
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
