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
   MP respawn sets +0x2C = 0x28; no SP producer known, always 0),
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
   phases > 4). The word-array producer is OPEN (the FUN_0040fe93
   0x62 tile consumer reads the same array; likely the trigger
   plane family) — the engine leaves both unwired (never-invent).
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
   0x24) at 0x4dcdb8, layout `+0x00 active · +0x04/+0x0C two
   x/y coord pairs (0x4dcdbc/0x4dcdc0) · +0x10/+0x14/+0x18
   spawn x/y/z (0x4dcdc8/0x4dcdcc/0x4dcdd0) · +0x1C countdown
   (0x4dcdd4) · +0x20 target robot slot (0x4dcdd8, −1 when
   fired/skip)`. Per record: active==0 → mark +0x20=−1, next;
   countdown==0xa → SFX FUN_0043a48e(bank 0x4edfe0, 0, x<<5,
   y<<5, push 2); decrement; still ≠0 → next; on reaching 0:
   gate `word[0x465daa + 2*tile] != 0` (a per-tile word bank)
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
   [census-only]). The gate banks: 0x460dfa written by weapon
   fire (0x41a84f) + the 0x4227xx platform family (0x7d2/0x7d3/
   0x7d4 tile words), read by the armor pass (0x40fef4/0x410018)
   + the splash tick (0x41b8xx); 0x465daa written/read by the
   platform family + read at 0x41f0fd. The records ALSO have a
   draw pass in the FUN_00403938 tail (0x4065f8..0x4066a3
   reads +0/+0x1C/+0x04/+0x0C) [census-only]. **Verdict for the
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
| +0x18 fade | every frame, all w*h tiles, nonzero → −1 (FUN_00424051 head) | §7j.10 |
| debris kinds | 20-kind jump table 0x4205b8; +0x1C=kind, +0x20=physics class 0/1/2/3/6, +0x24←[esp+0x20] delay, +0x28←[esp+0x24] param; ring kinds 1(+13/14/15)/3/4/5/6+12/9/11/20 (nine 3×3), 2/8 single center 3/4, 7/10/16..19 none | §7j.11 |
| debris seq tables | 11 tables, i16 −1-terminated, DGROUP 0x454424..0x454510 (k5-family {5..16}; k2/k8 {0..4}; …{44..104} = k16..20) | §7j.11 |
| debris arrival SFX | FUN_00421e60 3-way (banks 0x4edf64/68/6c, push 2) · FUN_00421dec 4-way (0x4edf98/9c/a0/a4, push 1); both gated [0x4ede58]≠0 | §7j.11 |
| arrival scheduler | FUN_0042034c epilogue 0x448076: 45 rec @0x4dcdb8 stride 0x24 {active, xy×2, spawn xyz, countdown, robot slot}; gate word banks 0x465daa/0x460dfa (2·tile); fires FUN_0042394a(x,y,z,0,0) clearing the first water z-word | §7j.11 |
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
