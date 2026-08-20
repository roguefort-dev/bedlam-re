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
| +0x14 | u16 | deploy countdown (0xFFFF when spent; decrements phase 0) | 0x4c69fa |
| +0x18 | u16 | random at spawn: RandA()&3 (variant) | 0x4c69fc |
| +0x1A..+0x29 | 8×u16 | per-probe floor z cache (written by move_is_possible; +0x1A doubles as the climb-compare z: `dword@+0x18 >> 16`) | 0x4c69fe |
| +0x2A | u16 | robot TYPE (indexes the 0x62-stride stats table at 0x4de664) | via 0x4c6a0c>>16 |
| +0x2C | u16 | reinforcement/deploy timer slot (1, 1+(2000-m*1000/27), ...) | 0x4c6a10 |
| +0x36/+0x38 | u16×2 | words of the per-type stats table | 0x4c6a1a/1c |
| +0x6E | u16 | toggle bits (bits 0/1 via keys 1/2) | 0x4c6a52 |
| +0x70 | i32 | deploy-delay counter (vs table DAT_00454ee8[DAT_0046cbf8]) | 0x4c6a54 |
| +0x74 | i32 | stop distance for the active order (1000000 = go all the way) | 0x4c6a58 |
| +0x78 | i32 | alive flag (0 = slot free) | 0x4c6a60 |
| +0x7C | i32 | countdown (decrements when ≠0; gates phases 4/5: serviced iff > phase*32) | 0x4c6a64 |
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
   x ≥ 0x1E0 → `sidebar_control@0040d197` (sidebar UI: robot-select strips
   x∈[0x1E7,0x217)/[0x219,0x249)/[0x24A,0x27C) y∈[5,0x35), F1/F2/F3 +
   keys 1/2 toggles +0x6E bits; map toggle strip x∈[0x212,0x24E)
   y∈[0x1B4,0x1D0); writes selection DAT_0046cbdc, redraw flag
   DAT_0046ccec=2). Else (map viewport): **isometric unproject**
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
| DAT tables | z-base@0x4eaacc, y-line@0x4ea900 | 0041eb28 |
| CGR/DB ptrs | DAT_004edd60 (CGR), DAT_004edd58 (DAT), 0x4796bc/cc (type DB 0x1E stride) | 0041e231, 00407e11 |

## 9. Open items (next slices)

1. The mission file-load + table-build pass (fills 0x4ea900/0x4eaacc,
   004eddec/df0; the ".TOT/.DAT/.CGR/.MIN/.PAD" loader) — required by the
   P4 render slice anyway.
2. FUN_00440e45 (10661 B, GameMain call #2) identity — not the gameplay
   loop [verified negative]; likely the inter-mission shell (shop/map
   room) [hypothesis].
3. Phase semantics of robots()' extra passes (fields 0x4c6a16/18/88/8c)
   and the state 1 producers (patrol?).
4. Sidebar order buttons beyond selection (attack/guard modes at
   FUN_0040d197 tail — not decoded this pass).
5. The 0x62-stride robot-type stats table at 0x4de664 (speed? armor?) —
   the seam currently models only the verified geometry constants.
