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
| +0x30 | u16 | HEAT accumulator [re-labeled §7j.55 from the §7j.45 "burn-damage/armor" gloss per §7j.53's corpus strings]: +0x14 per phase-1 pass ON-scorch (FUN_004100b7 @0x40bc72, behind the +0x98 damper), −0xA per pass off-scorch (clamp 0); clamp ≤ 0xBB8; 0x753 crossing → "TEMPERATURE CRITICAL", 0x9C4 → "HAS OVERHEATED"; sidebar HEAT gauge scale 2500; SP death/MP respawn reset 0 | 0x4c6a14 |
| +0x32 | u16 | LOSING-AMMO warning cooldown [gloss corrected §7j.55; was "BURN cooldown ... scorched tiles re-burn"]: := 0x64 by the FUN_004102b6 cook-off tail @0x4103e3 (gate ==0), dec 1/walk (robots() pre-walk 0x40bab7, gate ≠0); sole producer/sole reader = the cook-off — no tile-burn role exists | 0x4c6a16 |
| +0x34 | u16 | ALARM cooldown: set by the FUN_0040e230 alarm path (with +0xA4), dec 1/frame; sidebar reads dword@+0x34 [§7j.45] | 0x4c6a18 |
| +0x36/+0x38/+0x3A | u16×3 | per-order stats-group copy i (8-byte groups, i=0..6): word0 = group availability (spawn default probe), word1 = the sidebar order gate (copied twice) [§6c.6] | 0x4c6a1a/1c/1e, spawn 0x40cf05..0x40cf42 |
| +0x6E | u16 | ORDER BITS (bit i = order i active; bits 0..6 toggled by keys 1..7 / the 7 sidebar order rows; spawn default = 1 << first available) | 0x4c6a52, §6c |
| +0x70 | i32 | IDLE-TIME BOMBARDMENT ARM counter (§7j.54 CORRECTED the old "reinforcement/ resurrect delay" gloss: ++ at phase 0 while state==0 — SP only for the SELECTED robot (idx == [0x46cbd4]+[0x46cbdc]), MP for every idle robot — vs difficulty table DAT_00454ee8[[0x46cbf8]] = {400, 300, 200, 5000≈never}; threshold ∧ zone ∉{1,7} ∧ [0x4de658]==0 ∧ mode≠2 → the aerial-BOMBARDMENT salvo: SFX 0xC/0xD/0xE + blink [0x4dc5d0] + [0x4de658]:=0x80 + the 8-shell scatter into 0x4ea238 (§7j.54); cleared by the states-3/5 block + the arm tail — i.e. ORDERING the robot resets the idle timer) | 0x4c6a54 |
| +0x74 | i32 | stop distance for the active order (1000000 = go all the way) | 0x4c6a58 |
| +0x78 | i32 | (label corrected 2026-08-21, §6c.7: this row had drifted +4 — alive is +0x7C) — | — |
| +0x7C | i32 | alive flag (0 = slot free; sidebar select gate + armer's one-alive count) | 0x4c6a60 |
| +0x80 | i32 | countdown (decrements when ≠0; gates phases 4/5: serviced iff > phase*32) | 0x4c6a64 |
| +0x88 | i32 | SHIELD POINTS: −2/frame clamp 0 (phase-0 pre-pass); 0x20 per consumed charge / on state-3 (FUN_0040e230); 0x2710 while +0xA0 flash runs; renderer 0x403ef4 [§7j.45] | 0x4c6a6c |
| +0x8C | i32 | SHIELD CHARGES: spawn seeds word@chassis_row+2 (the equipment-chassis 0x2A..0x2E jump table 0x40cc8c); a hit with charges≠0 ∧ shield==0 consumes one → shield 0x20 [§7j.45] | 0x4c6a70 |
| +0x90 | i32 | dying countdown (states 5/6 → despawn/revive) | 0x4c6a74 |
| +0x9C | i32 | DEATH FLAG — the mission-fail liveness oracle [§7j.57/D129 closes the reader census]: set-only := 1 in BOTH FUN_0040e230 death tails (SP 0x40eac0 edx=1; MP respawn 0x40e82a edi=1 — the respawn re-init does NOT clear it); sole reader = the SP squad-wipe fail detector 0x44764c (any squad record +0x9C==0 → alive; all dead ∧ [0x4ede34]==0x1E0 → fail seq → MissionShell ret 3; MP ret 0 immediately); cleared ONLY by the mission-staging whole-bank zero-fill FUN_00402965(ecx=0x7E0, edi=0x4c69e4) @0x40cd38 — the bank is 12 slots; DISTINCT from +0x7C alive / +0x78 hp (both re-staged by MP respawn, this is not) | 0x4c6a80, §7j.57 |
| +0xA0 | i32 | hit-flash/fade countdown (nonzero → shield := 10000 + the player-robot palette strobe ladder; −1/frame) [§7j.45] | 0x4c6a84 |
| +0xA4 | i32 | ALARM COUNTER (§7g +3 on alarm; decays 1/frame phase-0 pre-pass — the D90 EXW-decay question CLOSED §7j.45) | 0x4c6a88 |

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

- Timers decrement (fields +0x32/+0x34 (0x4c6a16/18), +0xA4 (0x4c6a88 —
  the "+0x9C" gloss was an offset erratum, corrected 2026-08-23 §7j.45:
  +0x32 = BURN cooldown, +0x34 = ALARM cooldown, +0xA4 = alarm COUNTER,
  all dec 1/frame in the phase-0 pre-pass), +0x88 (0x4c6a6c -= 2 = SHIELD
  POINTS, with the 0x2710 flash-invuln + the +0x8C (0x4c6a70) charge
  machine), +0xA0 (0x4c6a84 dying/flash with FadeSetup side effects —
  phase 0 block, §7j.45).
- Body gate: `state's +0x2C countdown == 0` AND
  `(phase < 4) || (phase*32 < field_7C)` — i.e. phases 0..3 always run;
  phases 4/5 only while field_7C > 128/160 [verified expression;
  interpretation: field_7C is a drop/animation countdown that buys the
  extra sub-ticks — hypothesis].
- TOT tile-type specials: type 0x7d3 gates phase skips (CORRECTED §7j.45:
  on a 0x7d3 tile the body runs only while phase ≤ (+0x80 == 0 ? 2 : 4)),
  0x7d2 (phase 0) triggers FUN_0040e230(robot, 0xF, -1) [verified reads
  via 0x4ea900 + TOT mirror DAT_00460df8].
- Reinforcement ready [GLOSS CORRECTED §7j.54 — it is the IDLE-TIME
  BOMBARDMENT arm, not a reinforcement]: idle counter +0x70 vs
  DAT_00454ee8[DAT_0046cbf8] → slot SFX (0xC/0xD/0xE) + scatter of 8
  jittered SHELL records into 0x4ea238 (10-byte records, the falling
  salvo — full grammar + resolver §7j.54).
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
   regardless of the alive gate. `_DAT_004ede34/_DAT_004ea8f8`
   [identity CLOSED §7j.58/D130]: 4ede34 = the death-wipe iris
   cell — these strips are its CLICK-SELECT CANCELS (selecting an
   alive squadmate aborts the iris); 4ea8f8 = the MP death-position
   marker countdown, zeroed in tandem.
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
   0x222/0x254, 0xD) when the warning field `0x4dc5d0` ∈
   {1,2,3} (PRODUCER CENSUS-CLOSED §7j.59: the bombardment-warning
   squad-slot selector — value = the ENDANGERED robot's slot+1;
   0 and >3 both draw nothing). 0x4071E8 `FUN_0040807f` EVERY
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
   `DAT_0046cbdc` it writes 0x46ccec = ebx(3) [value CORRECTED
   §7j.58/D130 from this pass's "(2)": ebx := 3 @0x4480de],
   clearing `_DAT_004ede34`/`_DAT_004ea8f8`).

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
   (overlay family; the 0x4dc5d0 producer is census-closed
   §7j.59 — still not wired: it fires only on the idle-time
   bombardment arm, which no scripted scenario reaches).

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
3. **Table build** [verified 0x41dd1d..0x41dde3; the y_line/z_base pair
   re-verified instruction-by-instruction 2026-08-25 for the S0-08
   static-parity row — CORRECTS this item's old "h+1 dwords" gloss]:
   - `0x302` bytes copied from `0x4edbf8` → `0x4ddb34`, and
     `word[BIN_buf]` → `DAT_0046cdb8` [staged data, consumer unidentified];
   - `w = s16[TOT]` → `DAT_004eddec`, `h = s16[TOT+2]` → `DAT_004eddf0`
     (the TOT cursor advances 4; `DAT_004eddf4 = w*h`);
   - **DAT cursor +4** (skips the on-disk w/h header — the payload IS the
     plane-major u8 plane set);
   - **y_line: `y_line[y] = y*w` for y in 0..h−1 at `0x4ea900` — h
     dwords, NOT h+1** (the old gloss was wrong). Loop @0x41ddaa..0x41ddbe:
     eax (byte offset) and edx (value) start 0, ecx = w, bound ebx = h·4
     (`shl ebx,2` @0x41dda2 on the h cell 0x4eddf0); body stores
     `[eax+0x4ea900] = edx`, then `eax += 4`, `edx += ecx`; condition
     `cmp eax,ebx / jl` @0x41ddbc/e — the body runs for eax ∈
     {0,4,…,4(h−1)}: exactly h entries. There is NO boundary entry at h
     (the value h·w = one plane size is never staged), and every consumer
     stays in 0..h−1 (the ≥0x80 sweep reads y_line[0..h−1], y bound h
     @0x41de07; get_from_dat_file's callers clamp y < h).
   - **z_base: `z_base[z] = z*w*h` for z in 0..7 at `0x4eaacc` — 8
     dwords.** Loop @0x41ddc0..0x41dde2: ecx = h, eax = 0, edx = 0; head
     @0x41ddcb: ebx = w, `imul ebx,edx` (the value is stored FACTORED as
     w·(z·h)), `eax += 4` BEFORE the store, `edx += h` after the imul —
     iteration k stores `[eax+0x4eaac8]` with eax ∈ {4,…,0x20}, i.e. the
     8 dwords at 0x4eaacc..0x4eaae8, value w·(k·h) using the
     PRE-increment k: z_base[0] = 0, z_base[7] = 7·w·h (the plane-7 base).
   - **The store-BASE cells 0x4eaac8 / EXD 0x107714 are NOT table
     entries** — the dword at the base belongs to the adjacent
     screen-scale family (EXW writer 0x424da6 inside the 0x424d52 block;
     EXD twin cell zeroed at init 0x14794); the loop's pre-incremented
     eax never touches it. [census: the four table stores
     0x41ddb1/0x41ddd9/0x4466c7/0x4466ef are the ONLY writers of the two
     table spans program-wide]
   - **SECOND producer pair** [verified 0x4466bd..0x4466f8 inside
     FUN_0044661b]: a brief-screen loadout path (call site 0x43d1a5 —
     allocates fresh DAT/TOT arenas 0x13884/0x4b000 @0x43d0cc..0x43d10f,
     loads FULLFONT/BRIEF/palettes/SFX + the mission .TOT/.BIN/.DAT,
     strings 0x4590f9..0x4591ae / 0x459795..0x45979f) re-reads the TOT
     header into the same cells and re-runs BOTH loops
     instruction-for-instruction (y_line @0x4466c7, bound h·4, `jl`
     @0x4466d4; z_base @0x4466ef, base +0x4eaac8, eax ∈ {4..0x20},
     `jne` @0x4466f8) — same tables; that path does NOT re-copy the
     0x302 header block, re-run the ≥0x80 sweep, or stage PAD.
   - **EXD twin** [verified 0x2e713..0x2e74b, the same load block whose
     PAD leg is item 5's twin]: y_line at `0x8b78c` — same loop shape
     (store / `add eax,4` / `add edx,ecx`, bound h·4, `jl` @0x2e725): h
     entries, cells w 0x1074b8 / h 0x10748c / w·h 0x1074e4; z_base stores
     `[eax+0x107714]` with eax ∈ {4,…,0x20} → the 8 dwords live at
     **0x107718..0x107734**. Algorithm identical from the same source.
   - **Rust retention: NONE** — the engine indexes `dat[z·w·h + y·w + x]`
     inline (`Terrain::dat_type`); the tables are a pure (w,h) function
     whose semantic content is the retained dims (`Terrain::size`).
     Independently covered by the S0-08 oracle
     `engine/bedlam-core/tests/static_yline_zbase_differential.rs` (D147):
     a TOT-header-only transcription of both loops vs the Rust target
     across all 37 missions, with the pinned corpus invariants —
     **TOT[0..4] == DAT[0..4] on every shipped mission** (the original
     builds the tables from the TOT header while Rust takes its dims from
     the DAT header; observably identical only because the corpus pins
     the agreement), dims {25×75 ZONEA/M1, 100×100 ×35, 100×25 ZONEG/M1}.
4. **Runtime sweep** [verified 0x41dde4..0x41de43]: every DAT byte ≥ 0x80
   in planes 0..6 is set to 0 (plane 7 untouched). The shipped corpus has
   0 such bytes in ZONEA (the 0xFF seen in-plane there is PAD-written
   post-sweep), so this only matters for editor/padded data.
5. **PAD staging** [verified 0x41de44..0x41df03; loop re-verified
   instruction-by-instruction 2026-08-25 for the S0-07 static-parity
   row]: PAD is read into `0x4e44f8` (0x1f38 = exactly 999×8, no
   slack) as 8-byte staged records `{u16 active@+0, u16 x@+2, u16
   y@+4, u16 kind@+6}` (disk 6-byte `(x, y, kind)` unpacked 2 bytes at
   a time). EXACT loop semantics: (a) **pre-zero** —
   `FUN_00402965(ecx=0x1f38, edi=0x4e44f8)` @0x41de62 is the
   stos-ladder memset (byte/word/`rep stosd`), so the WHOLE 999-slot
   bank is cleared on every .PAD load — no cross-mission stale tail is
   possible; (b) `while i < 999` (`cmp esi,0x3e7` @0x41ded4): read u16
   → staged `x@bank+8i+2` (persisted BEFORE the check — even the
   terminator's 0xFFFF lands in the bank); reload the staged dword@+0,
   `sar 16`, compare `-1` @0x41defa — on 0xFFFF EXIT (rewind
   `FUN_0041cd42` @0x41df03), leaving the terminator slot as
   **{active=0, x=0xFFFF, y=0, z=0}** (y/z never read); otherwise read
   y@+4, read z@+6, **active word :=1** @0x41de8c, then for i in 0..999: if `x != -1`: flag word set 1, and
   **`DAT[kind·w·h + y·w + x] = 0xFF`** with NO bounds check on kind/x/y
   [verified absence — shipped kind values are 0..6 and the 0x13884
   arena covers the largest map, so real writes stay in the allocation];
   (c) slots past the terminator stay all-zero — the file bytes after
   the terminator are never read (ZONEB/M3's orphan record is invisible
   to the runtime bank). The EXD twin is the IDENTICAL algorithm
   compiled from the same source: FUN_0002e55a PAD leg
   @0x2e7a0..0x2e85d — memset twin 0x12206 @0x2e7be (edi=0x8f63c,
   ecx=0x1f38), u16 read twin 0x2d5c8, same `sar`/-1 terminator check
   @0x2e7f0, same active:=1 @0x2e809, same DAT stamp @0x2e84d (base
   0x107518, w 0x1074b8, w·h 0x1074e4), same 999 bound @0x2e851. Rust
   retention: `Terrain::pad_slots` keeps the LIVE RUN ONLY (records
   before the terminator, file order, active implicitly 1); the
   inactive terminator/tail slots are unretained — unobservable
   through the retained seams because every original consumer gates on
   active≠0 (probe §7j.40/1, elevator stager §7j.21, scanner icon
   FUN_0041ee20).
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
 9. **The zone-BIN variant verdict — the G3 question CLOSED 2026-08-28**
    [verified: full disasm walk 0x44670c..0x446907 + tag/name string
    bytes read from the PE + a whole-image ASCII string census;
    objdump-only from the committed ghidra-project/exw-text-objdump.txt
    and exd-text-objdump.txt, no Ghidra run; worker cef2f815 claim 1]:
    does any mission load the mission-number `.BIN`
    (ZONEB/MISSION6.BIN, ZONED/MISSION5.BIN, ZONEE/MISSION6.BIN)
    instead of the zone-level `MISSION{L}.BIN`? **NO — the runtime
    NEVER opens the mission-number variants; the zone-level rule is
    absolute.** The evidence chain:
    - **path2 construction is unconditional and letter-only.**
      build_mission_paths@0x44670c (the §7c.1 builder) walked whole:
      path2@0x4dca8c = `EDITOR\`@0x4597ba + `ZONE`@0x4597c2 +
      chr(0x40+[0x4edd8c]) (append @0x446879..0x4468a8) +
      `\MISSION`@0x4597c7 + chr(0x40+[0x4edd8c]) AGAIN
      (@0x4468d2..0x446901) — then straight into the shared epilogue
      (`add esp,4; jmp 0x43c802` @0x446904/0x446907). There is NO
      itoa leg and NO conditional anywhere in path2's construction;
      the function's only branch remains the §7j.73/D183 +5 on
      path1's mission number when [0x4edb88]==2 (@0x4467ca..0x4467e2).
    - **the .BIN consumers are exactly two, both on path2, both
      builder-fresh.** concat(path2, `.BIN`@0x4587e8) @0x41dcbc
      inside load_mission (called at 0x41dc63, the function's first
      instruction after the prologue) and @0x446644 inside the
      FUN_0044661b brief-reload twin (builder called at 0x44661e,
      its first instruction; tag @0x45979a = `.BIN`, byte-verified).
      The joined name lives in the concat-private 0x40-B buffer
      0x4dca4c (concat@0x41dbed zeroes + fills it; the ONLY code
      touching 0x4dca4c is concat itself) and is opened
      cwd(0x4de544)+name `"r+b"`@0x457bdb by open@0x41cd90 via
      read_file@0x41cc7f. The three globals are one 3×0x40 family:
      path1@0x4dca0c | joined@0x4dca4c | path2@0x4dca8c.
    - **complete path-buffer census (29 EXW sites).** path2
      consumers: `.CGR`@0x41dca0, `.BIN`@0x41dcbc, `.MIN`@0x41dcd8,
      `.LNG`/`.LNK`@0x41dd09 (load_mission) + `.BIN`@0x446644 (the
      brief twin) + the builder's own stores — nothing else. path1
      consumers: `.TOT`/`.DAT`/`.PAD` (load_mission), the
      `.MRK`@0x40ccbd / `.NME`@0x416491 / `.TRT`@0x4170c8 /
      `.POS`@0x41a562 / `.BDG`@0x41a5f5 loader family (tags
      0x457a34/57/5c/64/69, byte-verified), the `GAMEGFX\BRF_`
      movie-name scratch reuse @0x43d1bc/0x43d277 (strings
      0x4591c2/0x4591d4 = `GAMEGFX\BRF_` + zone letter + itoa level +
      `.SMK`@0x4591cf/`.BIN`@0x4591e3 — the brief movie/subtitle
      pair, written AFTER the twin's load so it never interferes),
      and the save-file name reuse in the 0x43d5xx/0x43d6xx
      error/probe paths. No site concats any tag onto path2 other
      than the five family tags above.
    - **whole-image string census:** NO hardcoded
      `ZONE?\MISSIONn.*` literal exists in EXW (the only
      mission-name building strings are `EDITOR\`, `ZONE`,
      `\MISSION` + the extension tags; every other `.BIN` literal
      is a fixed GAMEGFX bank). The 8street boot check
      `EDITOR/ZONEA/MISSIONA.BIN` (main.cpp step 1) is
      reconstruction-side, not an EXW literal.
    - **EXD twin agrees** [verified]: the load block 0x2e5c3 calls
      builder 0x58606 then concats `.TOT`@0x2e5d1/`.DAT`@0x2e5f2/
      `.PAD`@0x2e7a5 on path1 0x92f74 and `.CGR`@0x2e60e/
      `.BIN`@0x2e62a/`.MIN`@0x2e646/`.LNG`/`.LNK`@0x2e672 on path2
      0x92f34; the tag table at linear 0x862a9..0x862cc is
      byte-verified against the overlay (file 0x9eaa9, delta
      −0x18800); the builder's second zone-char append @0x587d3
      runs straight into the epilogue (`jmp 0x51d12` @0x58801) —
      no itoa. (The overlay's linear↔file mapping is piecewise, so
      the brief-twin's own tag triplet stays structurally identified
      only — by its path1/path2/path1 leg bases, matching the EXW
      twin's `.TOT/.BIN/.DAT` order exactly.)
    - **data corroboration** [DATA, read-only]: only zone-level
      `.MIN` files ship, each sized 16× the ZONE-level BIN count
      (B 1872, D 1450, E 1455) — never the variant counts
      (1443/1443/1120); since EXW always loads the zone-level MIN
      (path2), a runtime variant-BIN swap would desync the minimap
      walk. ZONEB/MISSION6.BIN ≡ ZONED/MISSION5.BIN byte-identical
      (sha256 5735b08a3e08853e…, 2,189,466 B, word0 count 1443) — a
      shared development/deathmatch bank, distinct from both zone
      banks; ZONEE/MISSION6.BIN (1,508,806 B, count 1120) likewise
      distinct from MISSIONE.BIN (1,968,763 B, count 1455).
    Consequence: our engine's `mission_asset_names`
    `{ZONE{L}/MISSION{L}.BIN}` rule is VERIFIED correct as-is
    (engine untouched); the three variant files are editor-side
    residue, never wired into any runtime path. Closes
    RESEARCH-8STREET OPEN QUESTIONS #3 and the P5 census G3 class
    (docs/P5-ZONE-GATES §6.2/G3, D184).

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
   [CORRECTED 2026-08-23 §7j.45/4: the group word map is +0 name_idx,
   +2 ammo, +4 shop-artifact (unconsumed), +6 price, +8 category, +0xA
   item_idx, +0xC owned — this item's "price, category, item_idx" sat
   one slot low; and the shop exit adds a FOURTH writer family: the MP
   sync mirrors every player's type-4 COMMAND record into
   0x4de664+p·0x62.]
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
     **RE-LABELED 2026-08-23 (§7j.55/D127)**: this is the HEAT
     gauge, not armor — +0x30 is the HEAT accumulator, and the
     bar's full scale 2500 IS the "HAS OVERHEATED" threshold;
     empty = not heated.
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
   **TERMINOLOGY SUPERSEDED 2026-08-23 (§7j.55/D127)**: this
   item's "armor"/"pool"/"charge ticks" vocabulary predates
   §7j.53's corpus-verified WARNING strings — the family is the
   HEAT machine: +0x30 = the HEAT accumulator (0x753 crossing
   → "TEMPERATURE CRITICAL" ids 6/7/8, 0x9C4 → "HAS OVERHEATED"
   ids 3/4/5), +0x98 = the DAMPER ("DAMPER EXHAUSTED" ids
   0x2E..0x30), FUN_004102b6 = the AMMO COOK-OFF ("LOSING AMMO"
   ids 0x31..0x33, 1/128 per pass, drain = ammo>>3 floor 1),
   and the drain-before-charge design is CLEAR: the damper
   absorbs heat before the accumulator builds — the "intent
   unclear" tag is RETIRED. Full decode §7j.55.
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
   census'd — CLOSED 2026-08-23 §7j.57/D129: both death tails
   store 1; the sole reader = the SP squad-wipe fail detector
   0x44764c; the MP respawn tail also stores 1, no reset),
   armor = 0, SFX 0x19/0x1a/0x1b + the selected-robot
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
[Superseded 2026-08-22 §7h.4: the producer chain is now DECODED
end-to-end (init_tiles staging + the four-site type-3 latch + the
clear→move→test consume + set = zone+1) and the corpus verdict is
ZERO pickup cells on ZONEA/M1 — the seam stays host-seamed BY
CORPUS FACT, not by unknowns; P4.2 hooks D99.]

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
   all xrefs — the COMPLETE 7-site census + value grammar landed
   later as §7j.59/D131, which CORRECTS the value gloss]: value =
   the ENDANGERED robot's squad SLOT + 1 (1..3) — this item's
   original "the SELECTED robot's SLOT + 1" gloss holds only in SP,
   where the sole arming robot is necessarily the selected one
   (§7j.54); in MP every idle robot arms and the write names the
   tripped robot's own slot.
   Producers: the robots() per-robot walk 0x40c1ae..0x40c25e — when
   `idx == [0x46cbd4] + k` (k = 0..2, squad-window base 0x46cbd4)
   and squad size `0x46cbd8 > k`: posts the radio-warning PAIR
   `FUN_004239ef(0xC+k, k)` + `FUN_004239ef(0xF, k)` — per §7j.53
   these are the "DANGER - UNIT n TARGETTED FOR / IMMINENT AERIAL
   BOMBARDMENT" WARNINGS lines, NOT a select sound (the former
   "select SFX" gloss corrected; the walk is §7g.5's threshold
   announcer) — and `[0x4dc5d0] = k+1` (the cursor write is the
   attention-draw on the endangered unit); MissionShell
   entry zeroes it (0x447871); FUN_00423e1c — GLOSS CORRECTED
   §7j.54: NOT a "selection chaser" — it is the BOMBARDMENT
   SHELL TICK/RESOLVER (sole caller MissionShell 0x447ffa; its
   record-0 impact block 0x423e7c..0x423ed5, gated SP ∧ cursor ≠
   selected+1 ∧ cursor-robot is player-type, stages a 15-frame
   chase-CAMERA cut to the impact via FUN_004245c9 → 0x4de648,
   see §7j.54; it never writes the selection), and
   its exit path 0x423fef clears both the cursor and word 0x4ea240.
   Consumer = the 7f.4 sidebar switch [0x407420..0x407989, verified]:
   `edx = [0x4dc5d0]`; edx ∈ {1,2,3} → blink-cursor sprite
   `FUN_00401ca2((g_frame_count & 3) + 0x51, 1, x, 0xD)` from
   GENERAL.BIN (0x4edd7c) at x = 0x1F0 / 0x222 / 0x254 (slot k =
   edx−1); any other value → nothing (0x4072b8 skip). [§7j.59
   supersedes this item as the authoritative census: 5 writers +
   2 readers, per-writer addresses, the {0,>3}-draw-nothing
   dispatch, the effect-row disjointness pin, the lifecycle.]
7. **FUN_00420549 = the debris tick** [verified whole; MissionShell
   epilogue @0x448076]: per active record: if `+0x24 (delay) != 0` →
   decrement, skip; else `+0x18 (seq) += 1`, read
   `(i16)table[+0x2C][seq]`: `== −1` → `+0 = 0` (done, slot freed);
   else if `+0x20 (physics flag) != 0` → `FUN_0040de9c(idx)`.
   FUN_0040de9c = the per-frame debris PHYSICS + collision
   pass — DECODED WHOLE as §7j.44 (the flag is a COUNTDOWN;
   three collision walks: robots/critters/POIs; sole caller =
   this tick @0x420585). The kind-5 table at 0x454424 [bytes verified]:
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
  bank alloc'd at mission load 0x41d9d7. CORRECTED by §7j.63: the
  "written 1 by the ORDER marker family 0x425556" gloss was WRONG —
  0x425556 is the inner store of FUN_004254e1, the MISSION-LOAD
  initializer that memsets the bank 0 and stamps the door-rect tile
  claims; readers: the platform stager 0x423858, this stager, the
  death-blast producer, and the radar marker-0xd gate 0x41f191 —
  the per-tile DOOR-RECT CLAIM bank). Allocation: first
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
   COUNTDOWN seed (0 = no physics; 1/2/3/6 = run FUN_0040de9c,
   which DECREAMENTS it per frame — §7j.44 corrects this
   gloss: never a 0x454510 table index; the params are
   arithmetic); `+0x24` ← [esp+0x20] (start
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
   push 2) [CORRECTED §7j.52: the draw is **RandB**
   (FUN_004029b6), not RandA; cells = BOOM1/2/3; fires at STAGE
   time — full decode there]. **FUN_00421dec = the 4-way
   variant** [verified]: RandA()&3 (jump table 0x421ddc) → banks
   0x4edf98/0x4edf9c/0x4edfa0/0x4edfa4, push 1 [draw also RandB
   per §7j.52; cells = RICOCHT1..4]; sole callers kinds 2+8.
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
   DIFFERENT table — NOT the FUN_0040de9c params (§7j.44: the
   physics params are arithmetic in the +0x20 countdown;
   0x454510 remains census-only, unconsumed by this family).
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
   (FUN_00423e1c — §7j.54: the bombardment shell resolver,
   not a "selection chaser"), 0x424536
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
   (FUN_00422693), the bombardment shell resolver
   (FUN_00423e1c, §7j.54) and
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
   0x422f9a/0x422fc6; later z can overwrite earlier). Correction
   2026-09-06 (RE-EXW-TOXIC.md): raw zone indexes with NO subtraction;
   the eight dwords at 0x454a20 are
   {0x20,0x49,0x49,0x34e,0x49,0x77,0x77,0x49}, and at 0x454a3c
   {0x49,0x4e,0x4e,0x349,0x4e,0x7c,0x7c,0x4e}. Upper bounds are
   inclusive (JG), five frames. Within a layer 0x7d3 stamps first,
   then 0x7d2. The prior transcription/index interpretation was wrong.
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
   stride 6 `{dword payload, word@+4 countdown}` (corrected by
   RE-EXW-FENCE.md; increments at 0x422c7f/0x422dd6); producers
   FUN_00422c9b (find-free + set countdown 8) and FUN_00422e0a
   (payload = FUN_00439c20() result, then rec-id match →
   FUN_004245c9(x<<5, y<<5, z<<5) — the §7j.54 chase-camera
   cut — census). On countdown 0 the
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
     7j.11 k12 sites 0x40ff7e/0x4100a8). CLOSED [was OPEN:
     "FUN_0040fe93 indexes 0x4c69e4 with a 160-byte stride
     (20·i << 3) while the canonical robot stride is 0xA8" —
     second array or quirk?]: a CENSUS ARITHMETIC SLIP, not a
     second array — 0x40fe9e..0x40fea6 computes eax = 21·idx
     (`mov esi,eax; shl eax,2; add eax,esi; shl eax,2; add
     eax,esi` — the Watcom ×21 idiom; the gloss dropped the
     second `add`), so `[eax*8+0x4c69e4]` has stride 21·8 =
     168 = 0xA8, the canonical robot stride. Resolved by
     §7j.25 item 7 (D73, 2026-08-21); independently
     re-verified 2026-08-26 with the full caller/extent
     census (sole call site 0x40bc44, idx ∈ [0,[0x46ccbc]]
     ≤ 12, no jump-table refs, extent map inside the D129
     12×0xA8 bank) — see the §7j.25 item 7 addendum [D166].
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
   type word. Projectile type ids ARE weapon-stat ids — CORRECTED
   2026-08-23 §7j.50: only the 0x67/0x68 terrain/robot impact legs
   read the record's OWN state word (the [+0x4cc652]>>16 dword
   trick); the 0x65/0x69 legs pass the LITERAL 0x65, the 0x66 leg
   the literal 0x66. Callers: 4 in FUN_00412010 (the 3 probe-hit sites
   + axis-counter expiry 0x412425/0x41243f) + 2 in FUN_004197d4
   = the projectile-vs-ROBOT proximity walker (|dx| < 0x10 Q8 ∧
   |dz| < 0x20 vs robot rec @0x4c69e4+0xA8·i, z@+8 → expire on
   robot hit, then 0x65 damage lookups; §7j.50: states 0x65/0x67/
   0x68 admitted only — 0x66/0x69 never damage robots) — the
   ROBOT-HIT arm of
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
     UPDATE (§7j.49, 2026-08-23): the mechanism stands, but
     the screen context is BRIEF-ONLY — FUN_00440a2d runs
     solely inside FUN_00440dc2 (the objective-minimap
     snapshotter); the in-game full mirror build is
     init_tiles (MISSIONVIEW §2), not this path.
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
   caller — scroll restamp? RESOLVED §7j.49: the BRIEF
   objective-minimap snapshotter, BRIEF-only), the 0x4787c4/0x47879c rect
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
     (dist<0x50) projectile 0x69 — a 0x4cc654-bank STATE (the
     per-level BEAM column), NOT a 400×0x36 weapon type; the
     "absent from the 7j.15 damage table (→ 'else 1')" guess is
     CORRECTED 2026-08-23 §7j.50: the impact re-keys to the
     LITERAL 0x65 (50/100/200 by d, per-frame at the blocked
     level; terrain-only, never robots) — fired at fire rate
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
     FUN_00448b80). **[CORRECTED 2026-08-28 §7j.77/2 — the +4/+6
     stores were TRANSPOSED in this 2026-08-21 reading: the asm
     (0x417076 stores 0x4dabe2 = +6 := 5; 0x41707e stores
     0x4dabe0 = +4 := 1) seeds STATE 1 = IDLE and the DEAD
     angle seed 5 at +6; +2's 0x32 is the HP word (the
     FUN_0040dc1b damage lane), not a timer. Personnel spawn
     IDLE and only ESCAPE via the flee lane — §7j.77 decodes
     the whole controller.]**
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
     CLOSED 2026-08-23 §7j.50/D122 — the else path is dumped
     (inline jump tree, no memory table), no caller ever passes
     0x69, and the beam's impact re-keys to the LITERAL 0x65
     (50/100/200 by d, terrain-only, never robots).
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
     latch [0x46aed4+idx·4]==0 — the per-robot CLAIMED/
     no-extract latch (readers FUN_0040e230 = the SP death
     core (7g), FUN_00449c94/0044a38a (MP), FUN_00408e99,
     GameMain 0x41c40d = the sole writer, a boot memset —
     D133 CORRECTION: the original "writers" list named the
     reader functions; the EXW setter set is EMPTY, the EXD
     twin 0xf929c is set :=1 only by the DOS MP lobby pick
     FUN_0005bb71 — RE-EXD-MAP §5). Phase-1 landing fires
     the POD PAYOUT once:
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
   - message ids → FUN_00424a6f(id) = the zone MESSAGE SHOWER —
     §7j.46-corrected: zone A / mission 1 ONLY, ids 0..0xE = the
     BOOT_CAMP_%03i sections of the LANGUAGE.* blob (NOT an
     in-memory "string table @0x458ca7" — that is just the name
     prefix; the "≥0x3d range per zone" gloss retired), SFX
     _DAT_004edfd0 = TEXTBOX1, per-id latch 0x4eb5f8;
   - door ids → FUN_004223b8(rect, 1|2) = the DOOR toggler
     over the 45×0x10 trigger-rect bank @0x4dcae8 (TOT
     stamps FUN_004235e4/FUN_004235bf over the rect W×H, SFX
     0x23/0x24);
   - **exit-pad activations → DOOR + FUN_0041fa51(slot) pairs**
     (§7j.46: NOT one case — zone F M1..M5 + zone G M1 carry them;
     the old "case 0x1B @0x43900e = the SOLE activation" gloss
     retired): robot steps on the
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
   CLOSED 2026-08-23 by §7j.46: the full per-zone FUN_00433980 case
   table (all zones/modes; the ride-record bank grammar 0x4dcdbc
   stride 0x24; the 21 beacon slots; the zone-F/G EXIT pairs; zone E
   = verified negative) + the FUN_00424a6f message table (the
   LANGUAGE.* section system, the 15 BOOT_CAMP ids, the latch/timer
   semantics — §7j.46).
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
   (per-tile FUN_004235e4/FUN_004235bf stamping; the
   FUN_004245c9 call beside it is the §7j.54 CHASE-CAMERA
   cut, (x·0x20+w·0x10, y·0x20+h·0x10) — the old "wall-strip
   redraw" attribution of THAT call retired); per cell tests
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
   [0x4edb90] → FUN_004245c9 — §7j.54: the chase-CAMERA
   cut to the shell (the "wall-strip redraw (spotter
   reveal)" attribution retired); ttl≥0x20: while ttl−0x20 < dword[0x456c78+4·id]
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
   latch READER per §7j.19 — D133: the earlier "writer"
   gloss is corrected, the EXW setter set is empty).
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
   ADDENDUM 2026-08-26 (the `fe93-stride-alias-census` unit;
   queue hygiene: the queue re-asked this point as OPEN — D73
   had already closed it, and the §7j.13 `OPEN:` marker was the
   residue; everything below independently re-derived from
   ghidra-project/exw-text-objdump.txt): (a) stride arithmetic
   re-decoded instruction-exact — 0x40fe9c `mov esi,eax`,
   0x40fe9e `shl eax,2`, 0x40fea1 `add eax,esi`, 0x40fea3
   `shl eax,2`, 0x40fea6 `add eax,esi` → eax = 21·idx; the
   three loads `[eax*8+0x4c69e4]` / `+0x4c69e8` / `+0x4c69ec`
   = x/y/z dwords @+0/+4/+8 of the SAME 0xA8-stride record
   (sar 0xd / 0xd / 0x5, z clamped ≥ 0 — the §3 field layout).
   (b) caller census re-run over the FULL objdump text:
   exactly ONE reference to 0x40fe93 anywhere (the direct call
   0x40bc44) and ZERO jump-table encodings (byte pattern
   `93 fe 40 00` — no hits). The call site sits in
   FUN_0040b9f6's per-robot walk; the idx range is pinned by
   the loop tail 0x40c483..0x40c491: idx ([esp+0x20]) runs
   [0, [0x46ccbc]) — the robot count, ≤ 12 (the D129 12-slot
   bank). (c) extent map: NO 20×160 array exists anywhere at
   the base — the only bank is the D129 12×0xA8 = 0x7E0
   zero-fill span 0x4c69e4..0x4c71c4 (FUN_00402965,
   ecx=0x7E0 @0x40cd29..38); with idx ≤ 11 the highest byte
   FUN_0040fe93 touches is base + 11·0xA8 + 8 = +0x740
   (0x4c7124), well inside. Even under the erroneous 160 gloss
   idx would have to reach 13 before crossing the extent (and
   count is capped at 12), but the two strides disagree at
   every idx ≥ 1 (160 ≠ 168), so the instruction decode is the
   dispositive evidence. VERDICT: QUIRK — census arithmetic
   slip; NO second array. Registry/plan consequence: NONE —
   watches.toml and the dbx-plan robot-bank row were always
   pinned stride 0xA8; nothing moves (D166).
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
   writers = FUN_00440a2d (TOT-mirror materializer — UPDATE
   §7j.49: BRIEF-screen only, the objective-minimap window
   stager, never an in-game writer), FUN_0043d00b (the BRIEF
   alloc), FUN_0041d954 (the mission arena/viewport-cache
   install — the in-game list producer).
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
   completed** [verified objdump; D133 CORRECTION of the original
   writer claim]: boot RESET = GameMain @0x41c408 (memset 0x30 =
   12 dwords) — NOT per-mission, and it is the ONLY writer: the
   full literal-site census (9 sites = 8 cmp readers + the memset
   pair; no memset/rep-movs span overlaps the array) proves the
   EXW setter set is EMPTY — the four functions originally listed
   as writers (FUN_0040e230, FUN_00449c94/FUN_0044a38a,
   FUN_00408e99) are READERS. **FUN_0040e230's MP respawn branch
   @0x40e7a1 is itself gated by the latch (≠0 → skip respawn)** —
   the latch is a per-robot CLAIMED flag: it freezes a mid-flight
   pod record (the animator skips it) AND refuses the MP re-drop.
   The EXD twin 0xf929c (RE-EXD-MAP §5, D133) mirrors all 8
   readers + the boot memset and adds the ONE setter the EXW
   build lacks: the DOS MP lobby robot-pick FUN_0005bb71
   (@0x5bba0 :=1) — on EXW every latch gate therefore takes the
   ==0 path at runtime (pods extract everyone, MP respawn
   unrestricted, cyclers see all robots as available).
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
   ∨ |vy d@+0x22| > 0x40 → += [0x46ae68]&7 (the global tick
   counter). [CORRECTION 2026-08-22, worker bbbdedec, re-verified
   asm 0x404321..0x404353: `test ebx(vx),ebx / jne 0x404349` fires
   the wobble for ANY vx≠0 BEFORE any range check — the vx
   magnitude tests at 0x404335/0x40433a only execute when vx==0
   and compare the constant 0 (vacuous); the vy arm wobbles only
   when |vy|>0x40. Previous text said "vx≠0 AND (|vx|>0x40 ∨
   |vy|>0x40)" — wrong for 0<|vx|≤0x40 ∧ |vy|≤0x40 (wobbles).
   Static only when vx==0 ∧ |vy|≤0x40.]
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
     heavy (d+1)·300 TRT bolt is invisible — 7j.16; §7j.50: the
     0x66 producer is the TRT fire routine @0x417a5c, its damage
     key fires ONLY on the terrain contact class 2, and it never
     damages robots).
   - **0x67 → 0x404fac**: enters the 0x404eb1 tail — draw IDENTICAL
     to 0x65 (frames 0x3C..0x3F, mode 0x12C).
   - **0x68 → 0x404ffc**: same projection (z Q13, shake), frame
     **(g_frame_count&3)+0x38** (56..59), mode 0x12C.
   - **0x69 → 0x404d96 — the vertical BEAM column** (the §7j.22/23
     open-item type; §7j.50 CLOSED the damage question: producer =
     the k7 close-combat leg @0x4135a2 {z=6, TTL 0x18, +0x1A=0},
     terrain-only damage key LITERAL 0x65 per frame at the blocked
     level, never robots): NO shake term; sy base −= (d@+0xA<<5)+8 —
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
| robot record base/stride/count | 0x4c69e4 / 0xA8 / DAT_0046ccbc | 0x40c536, 0x40b9f6, 0x40fe93 stride proof (×21-idiom, §7j.25 item 7 + D166 re-verify) |
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
| debris arrival SFX | FUN_00421e60 3-way BOOM1/2/3 (cells 0x4edf64/68/6c, priority 2) · FUN_00421dec 4-way RICOCHT1..4 (0x4edf98/9c/a0/a4, priority 1); both gated [0x4ede58]≠0, pick = RandB (T4); fire at STAGE time on in-map bounds alone (k11 + RandA&1 ~50% gate) | §7j.11, §7j.52 |
| arrival ride tick | FUN_0042034c epilogue 0x448076: 45 rec @0x4dcdb8 stride 0x24 {active, marker xyz tile, dest xyz, countdown, robot slot}; walk STOPS at first inactive (contiguous run from rec 0); countdown 0 = dormant skip; ==0xA SFX bank 0x4edfe0 at marker; →0 teleport robot to dest + burn platform (both gate banks) + FUN_0042394a(x,y,z,0,0) water clear | §7j.11, §7j.21 |
| elevator stager | FUN_00425da4 (MissionShell boot @0x447b4e): clear 45 records then per-(zone [0x4edd8c] 1..7, mode [0x4edb88], mission [0x4edd88]) fixed-address staging; marker ← .PAD slot u16 x/y/z @0x4e44f8+slot·8+2; dest := immediates; +0x20:=−1; countdown never written (dormant); Z1 0..6, Z2/Z3 0..16, Z4 0..8, Z5 0..9, Z6 0..14, Z7 0..6 | §7j.21 |
| elevator ride armer | FUN_00433980 ride cases: guard +0x20≠−1; rider state@+0x0C:=2, pre-position at marker+0x1000, countdown:=10, +0x20:=rider; all armed countdowns = 10 | §7j.19, §7j.21 |
| arrival marker draw | FUN_00403938 tail 0x4065e5..0x4066e3: skip inactive/countdown-0; isometric marker tile; sprite 0x12E (FUN_0040798e, bank [0x46af38]) width clamp(11−countdown, 0, 9) | §7j.21 |
| memset-0 | FUN_00402965(EAX=0, ECX=len bytes, EDI=dst); 176 callers | §7j.21 |
| door-rect list boundary | 0x4dcae8..0x4dcdb8 = 45×0x10 door rects (0x2d0); MissionShell clears it @0x447b7b AFTER the stager — ends EXACTLY at the arrival base, no overlap; door consumers use idx 0..0x24 | §7j.21 |
| door open/close | FUN_004223b8(idx, state 1/2): rect {+0 state,+2 x0,+4 y0,+6 w,+8 h,+0xA variant} (§7j.34-corrected; the §7j.21 w/y/h permutation retired); state<3 only; anim-complete tile test low7(+0x1A)==+0x19 → FUN_004235e4 (state 1: +0x1A:=0x80) / FUN_004235bf (state 2: +0x1A:=0), +0x19 := variant<<4; FUN_004245c9 = the §7j.54 chase-camera cut (old "wall redraw" attribution retired); SFX 0x23/0x24 bank ELEV1 0x4edfb0; 86 callers (FUN_00433980 pads) | §7j.21, §7j.34, §7j.54 |
| door animator tick | FUN_00423081 (sole caller MissionShell epilogue 0x44808f, after the creep tick 0x44808a): walks the 45 rects; state≥3 = AUTO doors (countdown@+0xC −1 per tick; at 0 → animate; on completion XOR bit7, re-target +0x19, countdown 0x14, SFX ELEV2 0x4edfb4 — cycles forever); state 1/2 = SCRIPTED doors (animate to target, stop); per tile with low7(+0x1A)≠+0x19: walk planes down (bit7: 6, else 7) → DAT volume door-frame byte 0x40+2·nibble (bit7, even) / 0x5F−2·nibble (clear, odd) at plane level+1 (bit7) or level (clear), table 0x4eaacc; low7++ ; nibble wrap → FINISH PAIR: FUN_004236c6+00423740 (close: DAT seen 1/0 + STACK PUSH-UP word[z+1]:=word[z], plane0:=0 if S+E neighbors are door tiles) / FUN_00423650+004235fb (open: DAT 0 + STACK DROP word[z]:=word[z+1], top cleared — the level leaves the stack); [0x4eaae8] = plane 7 offset (corrected by RE-EXW-ELEVATORS.md) | §7j.34 |
| tile word grid | word[0x460dfa+2·tile]: 0 = empty, 0x7d2 hazard, 0x7d3 phase-clamp, 0x7d4 platform, else object id+1 → rec n−1 @0x46cbf4 (stride 0x14 {x,y,z,id,flags,hp}) | §7j.12 |
| platform strength bank | word[0x465daa+2·tile] = platform hp (build 300 via the FUN_00422600 zone-code trigger at the dying instance's own record / 199 via creep; weaken −damage; ≤0 → destroy: clear water z-word + both banks + 5× k7 debris, NO site latch); CORRECTED §7j.41/3 ring gate = (old ≥ 200 ∧ new < 200) ∨ (old ≥ 100 ∧ new < 100) (the 7j.12 "≥100 ∧ (hit <200 ∨ new <100)" gloss rejected by the asm) | §7j.12, §7j.41 |
| platform family | damage FUN_00422693 ← weapon ray 0x41a8ff; trigger build FUN_00422600 (destroy-tail; id == the zone code — a TYPE-row match; zone table 0x4225e4, zone 3 sub-keyed by the WITHIN-ZONE MISSION NUMBER [0x4edd88] via 0x4225d0); spread ring FUN_00422832/FUN_004228ce (8-tile row-major, needs both banks 0 + claim 0 + no live robot in the SE 2×2 + z ≥ 1 + empty z-word + plane-A byte 0 + plane-B(z−1) volume 1 — CORRECTED §7j.41/2 — writes water z-word at volume 2 (seen 0) + 0x7d4 + strength + scorch+4); creep tick FUN_00422a9c (the §7j.41/4 PER-FRAME RandA gate draw at entry — unconditional, one draw every mission frame; +2 jitter draws on lucky frames; water ray walk; tip→FUN_00422832(…,199)); site latches 0x4dc5c8/cc on the WEAKEN→RING path only (CORRECTED §7j.41/3) | §7j.12, §7j.41 |
| 0x7d2/0x7d3 stamper | FUN_00422f18 (load 0x447b8f): z-word ∈ [0x454a20+4z, +4] → 0x7d2; ∈ [0x454a3c+4z, +4] → 0x7d3; CORRECTED §7j.35: tables indexed by the RAW set [0x4edd8c] 1..7 → set-indexed bases 0x7d2 {0x49,0x49,0x34E,0x49,0x77,0x77,0x49} / 0x7d3 {0x4E,0x4E,0x349,0x4E,0x7C,0x7C,0x4E} (the 7j.12 prose lists were entries 0..6, one zone off; entry 0 = the previous array's tail) | §7j.12, §7j.35 |
| type-DB tail stamper | FUN_00422fd1 (load 0x447ba3): 45 rec @0x4dcae8 stride 0x10 {state,x0,y0,w,h,variant,cd,flag}; STATE@+0 ≥ 3 (§7j.34: the 7j.12 "word@+2" qualifier was the wrong field) → byte 0x4796d5 = variant<<4, byte 0x4796d6 = (state==3?0:0x80) | §7j.12, §7j.34 |
| delayed trigger timers | 32 rec @0x4ea828 stride 6 {payload(lo/hi ids), cd(8)}; tick FUN_00422cc2 (epilogue 0x448085): expiry → SFX 0x4239ef(0x22,3), rec flags 0x40, z-plane-A clear, FUN_0041bd54(x,y,z,floor_word[0x454a90+4·zone]) | §7j.12 |
| fast z-writer | FUN_0041bd54(x,y,z,word): word@0x4796bc+30·tile+2z + seen=1 (FUN_0042394a without the DAT volume byte) | §7j.12 |
| scorch increment | FUN_0042223c(x,y,v): byte 0x4796d4 += v clamp 7 (platform damage/build use v=4) — 2nd producer beside FUN_00422287 | §7j.12 |
| weapon impact resolver | FUN_0041a894(x Q13, y Q13, chain ctr ecx, damage ebx, [stack] score flag): tile from x/y>>13; grid word 0/0x7d2/0x7d3 → ret 0 (pass); 0x7d4 → FUN_00422693; n>0 → rec n−1 hp−=damage, destroyed → flags 0x40 + tail → ret 1; ret 1 only on destroy | §7j.13 |
| object type table | 0x4dedf2, 0x4E stride, 282 recs from the mission file (FUN_0041a4f8, load call 0x447b76): W@+2, H@+4, D@+6 (word@+0 unconsumed [open]; 7j.13 erratum + 7j.16 verification), hp@+8, chain@+0xC, type@+0xE (0xb = score 10), count@+0x12, 5×8B effect entries @+0x16..+0x3E (selectors +0x16+8k → 9-case table 0x41a870 — map §7j.25), 4 W·H·D-word template banks @+0x3E/+0x42/+0x46/+0x4A (arena 0x46ad5c; disk order +0x3E,+0x46,+0x42,+0x4A interleaved; +0x46/+0x4A = the UNDER-terrain pair consumed by the destroy restore §7j.25; +0x3E/+0x42 = the CURRENT-state pair ≡ shipped TOT/DAT at footprints — DEAD EDITOR PAYLOAD, zero readers §7j.32) — exact 0x4E fit; footprint stamper FUN_0041a7f0 (word = rec idx+1 over W×H at spawn) | §7j.13, §7j.32 |
| chain detonation | destroy tail walks the object's 4 perimeter edges; chainable neighbor (id-table word@+0xC ≠ 0, alive) → recurse FUN_0041a894(pos, ctr+1@RandA&3==0, damage 1000); score [0x4dd40c] += type (0xb → 10) when stack flag ≠ 0 | §7j.13 |
| destroy-tail effect entries | 5 × 8B @type+0x16+8m (m 0..4, exit @+0x28): selector word@entry+0 ∈ 1..9 → jump table 0x41a870 idx sel−1; payload w2/w4/w6 @entry+2/+4/+6 = x/y TILE + z-level offsets off the 0x46cbf4 record; sel1→k14(+0xF,+0xF)+FUN_0041a225+5 splashes, sel2..5→k18/k17/k16/k19 single gibs at (+0x10,+0x30)/(+0x30,+0x10)/(+0x20,−0x10)/(−0x20,0)+4-splash loop, sel6/7→k10 at (+0x10,+0x20)/(+0x20,+0x10)+DEADMAN SFX (delay 0, param −1), sel8→k14 ×25 demolition shower @water z (±3-tile RandA&7−3 jitter, delay ctr+2m+i>>3), sel9→k20+3×3 splash ring (delay ctr+2+RandA&3); stager delay = chain-ctr+m (sel1/8/9); PRECEDED by the footprint W×H×D terrain RESTORE (TOT-mirror z-words ← bank@type+0x46, seen + DAT volume ← bank@type+0x4A, linear (z·H+i)·W+j); GER gate: type 0xb ∧ GER skips the whole restore/effect/score/chain tail (record still marked destroyed + triggers fired) | §7j.25 |
| script-blast entry | FUN_004244a1(x_tile, y_tile, z) [§7j.39/1, verified 0x4244a1..0x4245c4]: splash FUN_00424355(delay 0) → FUN_0041bc1c(5000) → FUN_0041a894(ctr 0, 5000, flag 1) → k6 debris on a 1-IN-8 RandA gate (test al,7; delay = a 2nd draw &7, param −1) → z' = clamp(z−1, ≥1) → ALL-critter hits (FUN_004190bc, weapon 0xC, owner −1) + ALL-robot hits (FUN_00418fca, weapon 0xD, box ±0x20/±0x30 Q5); callers = the artillery burst pairs + the mortar-0E 3-cell (§7j.39/4) | §7j.39 |
| effects-bank stager | FUN_0041a225(x,y,z tiles, delay ECX) — FIRST producer of the MISSIONVIEW §5d/§5e "effects loop" bank 0x4cf638: 80 slots × 0x1E (=0x960, the 7j.1 boot-clear bound), free iff word@+0x18==0 (first-fit allocator FUN_0041a4cc, 12-try spawn loop); record {x,y Q13+RandB&0x1F jitter<<8 −0x1000, z<<13+0xF00, vx/vy (RandB&0x3F)<<7−0x1000, vz@+0x14 RandB&0x7FF+0x1770 RISING (high word = sprite group 0..2 → DEBRIS.BIN img group*8+frame&7), active u16@+0x18 = FUN_0041ec59(3) (~8% stillborn), delay u16@+0x1A = ECX arg, frame u16@+0x1C = RandB&7}; callers: destroy-tail cases 1/8; mover FUN_00419f62 (kill off-map/ceiling z>>13>0xB); consumer = the §5e direct draw (7j.26) | §7j.25, §7j.26 |
| .POS + .BDG loader | FUN_0041a4f8 (mission load 0x447b76): opens ".POS" (str 0x457a64) → 2000×0x10 reads into the 0x46cbf4 object-instance array (id≠−1 scan → count 0x46cbe8) — CONFIRMS FORMATS §12 feeds the destructible array; opens ".BDG" (str 0x457a69) → the 0x4dedf2 type table: NO file header, ≤282 VARIABLE records — control u16 (≠1 → 2 B row), else W/H/D u16, hp i32, chain u16, type i32, 5×8B effect entries, FOUR on-disk template banks 2·W·H·D B each (slot order +0x3E,+0x46,+0x42,+0x4A — §7j.32); +0x12 count = nonzero selectors, computed at load; arena cursor 0x46ad5c; tail seeds instance hp@+0x10 ← type hp@+8 + stamps the claim grid per footprint. Corpus 37/37 EOF-exact, exactly 282 recs/file (7907 active), selectors ONLY 1..9 (§7j.25 item 8) | §7j.25, §7j.32 |
| .BDG template-bank semantics | 2×2 roles (§7j.32 corpus proof, ZONEA/M1 434/435 cells): CURRENT pair (+0x3E TOT words, +0x42 DAT words) ≡ the SHIPPED .TOT/.DAT at the .POS footprints — editor stamp payload, ZERO runtime readers (triple census: slot addresses, +0x3e/+0x42 displacements, arena walk); UNDER pair (+0x46, +0x4A) = the pre-building terrain, consumed ONLY by the destroy restore (mirror words ← +0x46; seen=(+0x4A word==0), DAT volume=+0x4A low byte); value domains b1/b2 tile words ≤1868, b3 ≤102, b4 ≤512; overlap footprints = last-.POS-slot-wins in the shipped TOT | §7j.32 |
| TOT-mirror tile record | ONE 0x1E-B record per tile @0x4796bc+0x1E·tile (unifies the scattered tail-byte families, §7j.32): +0x00..+0x0F = the 8 plane words (+2·z); +0x10..+0x17 = the 8 SEEN bytes (restore writes @0x4796cc = base+0x10+z); +0x18 scorch (7j.8/7j.9; the scorch→damage reader 0x40bc60 §7j.34); +0x19 = the door/scenery TARGET-TAG byte (variant<<4; the animator stops at low7==+0x19; readers 0x406bd6/0x406bf9 renderer adjacency, 0x4110cb fire anchor, 0x418735 standing-on-scenery, 0x4237c5/da neighbor test); +0x1A = {bit7 door PHASE, bits0-6 running FRAME COUNTER} (§7j.34: the 7j.12/7j.32 "door byte bit7" gloss refined — one half of the 15-frame slide machine; renderer Y-bias −nibble·0x500 @0x406c5c); +0x1B/+0x1C = the OBJECT-HEIGHT pair (z0, z0+D) — stamped by the objective pass FUN_0044889a (0x448963/75 + 0x448b4f/61), cleared by FUN_00448b80 (0x448c25/2c + 0x448d65/6c), read by the intact-vs-rubble draw pick (0x406891/0x4068ec); +0x1D ZERO traffic (71-site census §7j.34 — padding, closed) | §7j.32, §7j.34 |
| objective-building family | FUN_0044889a (zone gate [0x4edd8c]==7): counts type ids 0x44..0x47 into [0x46cce0] + stamps the +0x1B/+0x1C heights; FUN_00448b80(idx) = the destroy-tail "notify" (SP-only): [0x46cce0]−−, heights cleared, at ZERO → FUN_004239ef(0x28,3)+(0x29,3) + 0x46cd00:=3 / 0x46ccfc:=0x20 / 0x46ccc4:=0x32 (extraction-arm lights, 7j.20 cross-ref); edition≠7 = the script-objective path (0x4eaaee/0x4eaaf2/0x4eab0c walk, tables 0x4557f8/0x456810, code 0x1388) head-decoded | §7j.32 |
| TRT death stamp | FUN_0041bc1c tail (FORMATS §14 resolver): mirror plane word := word@[0x454a04+4·zone] (per-zone rubble table), seen := 1, DAT volume byte := 0, k15 debris FUN_00420608(×0x20 coords, param −1 delay 0) + splash FUN_00424355 at the FUN_0041bd78 water z — the .BDG-tail death shape minus the restore (no under-bank) | §7j.32 |
| mission family loader | FUN_0041dc5a (after path builder FUN_0044670c = "EDITOR\"+"ZONE"+[0x4edd8c]+0x40+"\MISSION"+n): loads .TOT/.DAT/.CGR/.BIN/.MIN then the language gate `cmp [0x4eba1c],1` → .LNG else .LNK, then .PAD @0x41de44 — the eight tags are ONE 5-B-stride table @0x4587d9..0x4587fc (no ninth entry); buffer/cell pairs 0x4dca0c/[0x4ede20], 0x4dca0c/[0x4edd58], 0x4dca8c/[0x4edd60], 0x4dca8c/[0x4ede1c], 0x4dca8c/[0x4edd9c]; second .TOT/.BIN/.DAT site 0x446623..0x446677 (tags 0x459795/0x45979a/0x45979f) | §7j.33 |
| editor-only extension set | ZERO string (case-insensitive byte census) in EXW/EXD/EXE/DIRECTX exes for: .BLD, .CTG, .COL, .MAP, .PTH, .TXT — the runtime never opens them (only "SAVED.BDL" @0x4597d6 = the savegame, unrelated); .BLD = the editor SOURCE of .BDG (record j ≡ BDG non-empty j: same hp/chain/type heads, same four template banks; FORMATS §17 grammar verified) | §7j.33 |
| destruction-thud SFX pair | banks 0x4edfb8 = SOUND\SFX\DEADMAN1.RAW / 0x4edfbc = DEADMAN2.RAW (loader 0x43a29b..0x43a368, strings 0x458f41/0x458f58): RandB&1 pick, FUN_0043a48e(bank,0,x,y,push 2); consumers = destroy-tail cases 6/7 (0x41b19c/0x41b1ac) + the debris-crush dispatcher FUN_0040dce0 (0x40dc62) | §7j.25 |
| projectile mid-flight draw | FUN_00403938 @0x404131 (after the 7j.27 ring passes): walk 400×0x36 offsets 0..0x5460; type w@+0 → 5 shell (WEAPONS 3..7, counter d@+0xE wraps 7→3), 9..0xB artillery (WEAPONS 8..15), 0xE mortar (WEAPONS frame 1 static + 8-puff trail 0x10+(tick+i)&7 mode 0x12E), 0xF/0x13/0x17/0x1A/0x1F damped (WEAPONS base 0x20/0x20/0x28/0x18/0x18 + (tick&7) iff vx≠0 ∨ |vy|>0x40 [corrected 2026-08-22 bbbdedec], anchor 0x108), 0x24 rocket (SHRIKE ((dir+0x7E)&0xFF)>>2 = 64-dir; ≤8 SMOKE puffs dist 0x20+0x10·i behind, count = d@+0xA/4), 0x29 homing (REAPER dir>>2; GENERAL reticle @ target d@+6 {0x1000 robot 0x4c69e4/0xA8, 0x2000 critter 0x4cccec/0x20, else FUN_004128ec} frame tick/3+2, anchor 0xF0; 4 SMOKE puffs dist 0x10+0x08·i); all FUN_0040798e modes 0x12C/0x12D; other types NOT drawn; banks WEAPONS/SHRIKE/REAPER/SMOKE/GENERAL = [0x4eddbc]/[0x46af30]/[0x46af2c]/[0x46af34]/[0x4edd7c] | §7j.28 |
| projectile tick | FUN_00412010: 50 rec @0x4cc654 stride 0x22 {state u16@+0, x@+2, y@+6, z@+0xA Q13, vx@+0xE, vy@+0x12, vz@+0x16, +0x1A counter, +0x1E TTL}; dispatch state−0x65 ∈ 0..4 → table 0x411ffc {0x65 mover, 0x66 guided stepper ≤10 substeps + contact classes 1/2/3, 0x67/0x68 shared ballistic, 0x69 the beam}; terrain probe FUN_0041eaa1; impact damage keys §7j.50 (0x65/0x69 → literal 0x65, 0x66 → literal 0x66 + FUN_0041bc1c, 0x67/0x68 → OWN state via the [+0x4cc652]>>16 trick) → FUN_0041a894; producers = exactly 5 (k2 0x65/k3 0x67/k5-6 0x68/k7 0x69 + the TRT 0x66 @0x417a5c); the MID-FLIGHT DRAW walk §7j.28 (0x65/0x67/0x68 WEAPONS strips, 0x69 the per-level beam column, 0x66 loop-next invisible) | §7j.13, §7j.28, §7j.50 |
| TRT-bolt robot-occupancy probe | FUN_00419756(x,y,z Q13), sole caller 0x4123ae (the 0x66 stepper's per-substep test): walks the robot bank 0x4c69e4/0xA8, count [0x46ccbc], ALIVE gate d@+0x7C≠0 → 1 on the FIRST record with \|Δ(x>>8)\|<0x10 ∧ \|Δ(y>>8)\|<0x10 ∧ \|z@+8 (Q5 raw) − z>>8\|<0x20 (±<0.5 tile lateral, ±<1 level z — the FUN_004197d4 box exactly; NOT octile); hit ⇒ contact class 3 = die via disburser (kind-8 debris, state := 0, pre-contact position after the substep revert) with ZERO damage queries — alive robots are a pure BLOCKER for the 0x66 bolt | §7j.51 |
| weapon-anim tick | FUN_00410823(phase 0..3, MissionShell 4×/frame): walks ALL 400 records 0x4c71f4 stride 0x36; record {w@+0 type=weapon id (0 free), d@+2 owner, d@+6 target sel (0x29), d@+0xA tick, xyz@+0x12/16/1A Q13, vxy@+0x1E/22, vz@+0x26, class@+0x2A (0x24/0x29 launch delay; 0xF/0x13 detonation cycles), arc@+0x2E (ballistic z-vel g=−0x100/t; 0x29 heading byte), trail link@+0x32}; per-type: 2..4 bullet 2-substep lookahead ray (commit 1), 5 shell + K3 trail, 9..0xB artillery burst (phase 0 only), {0xE,0xF,0x13,0x17,0x1A,0x1F} ballistic bounce family (0xE 3-blast mortar, 0x17 3-clone split, 0xF/0x13/0x1F damped), 0x24 rocket (launch delay, no gravity), 0x29 homing (robot 0x1000-bit/critter/TRT 0x2000-bit target, terrain-avoid steering, ttl 201); the per-type MID-FLIGHT DRAW map §7j.28 (types not listed there are NOT drawn mid-flight) | §7j.13, §7j.22, §7j.28 |
| artillery burst tables | durations dword[0x456c78+4·id]: w9→2, w0xA→4, w0xB→7 frames; per-frame i16 (Δy,Δx) pair lists (500 sentinel) via PTR[0x456bf0+4·(ttl−0x20)] → 7 lists @0x45687c..0x456adc (frame 0 = 7-cell cluster, then radius-2/-3 rings); each pair = FUN_004244a1 scripted 5000-blast + 50% (RandA) K0xB debris at center | §7j.22 |
| actor hit-test lanes | FUN_0041879d(owner,x,y,z,weapon) = critter lane (3-row presence-grid prefilter @0x4ea900 rows ±4 → FUN_004190bc(critter,owner,x>>8,y>>8,z>>8,weapon,mode 2), first hit returns; count [0x46cc2c]); FUN_0041874c = other-robot lane (MP-gated, FUN_00418fca(robot,…,2), skips owner, count [0x46ccbc]); odd phases only (2×/frame); third caller = renderer FUN_00403938 (weapon 0xC blast, owner −1, args <<5) | §7j.22, §7j.23 |
| critter hit applier | FUN_004190bc(critter,owner,x,y,z,weapon,mode): presence w@+0x24; kind switch w@+0x00 (1..7 = the .NME section states); mode 2 = octile<0x20 on x/y + z-box (kinds 1/4 cell-unit coords, 2/3/5/6/7 Q13; z 0x20/0x24/0x40), mode 1 = x/y only; kinds 3..7 immune while state w@+0x0C ∈ {6,7,0xB}; hit → hp w@+0x06 −= FUN_00419aff(weapon), attacker w@+0x04, flash w@+0x7C, kinds 4..7 state := 5; death per kind 1→FUN_00418835 2→FUN_004188d0 3→FUN_00418aa6 4→FUN_00418ca4(+weapon) 5/6→FUN_00418e26(+weapon) 7→FUN_0041896c (§7j.24; the debris-crush dispatcher FUN_0040dce0 is the second dispatch site) | §7j.23 |
| robot hit applier | FUN_00418fca(robot,x,y,z,weapon,mode): presence d@+0x7C; box test \|dx\|,\|dy\| < 0x20 (d@+4/+8 >>8) + mode-2 \|dz\| < 0x30 (d@+0xC raw); hit → FUN_0040e230(robot, FUN_00419aff(w@rec+0), d@rec+2 owner) + hp d@+0x78 clamp ≥0 | §7j.23 |
| robot damage applier | FUN_0040e230(robot,damage,owner): state w@+0x0C==2 skip; state 3 → shield d@+0x88 := 0x20; gate d@+0x8C==0 ∨ d@+0x88≠0; alarm w@+0x34==0 → counter d@+0xA4 += 3, >100 → SFX 0x10/11/12 per player slot + w@+0x34 := 100; shield-down: hitcount w@+0x2E++, hp d@+0x78 −= dmg, tier SFX 0x2B/0x2C/0x2D, 0x13..0x15 (≤50%), 0x16..0x18 (≤12.5%) vs 5000+100·variant d@+0x94; shield-up: d@+0x88 absorb clamp 0; death MP: scoreboard 0xC-stride @0x4ebaa8 {score d@+0, flag d@+4, d@+8 := 0xB} suicide gate killer==victim∨−1, killer++ cap 999/victim−− clamp 0; shared tail: FUN_0042382c blast record + DAT_0046ccec := 3 + 7 order words zeroed + 5× k5 debris; SP tail: selected→[0x4ede34] := 1, alive/drop/hp := 0, +0x9C := 1, armor 0, SFX 0x19/1A/1B; MP respawn: full reset + variant RandA&3, pod 0x28, MRK reposition, weapon/equipment re-copy | §7j.23, §7j.24 |
| squad-wipe fail detector | FUN_0044764c..0x44770a (sole caller MissionShell 0x44870d, gated [0x4dc67c]==0 = extraction incomplete): MP → ret 0; walks squad [0x46cbd4]..+[0x46cbd8]−1 — FIRST record with death-flag +0x9C==0 → ret 0 (someone alive); ALL dead ∧ [0x4ede34]==0x1E0 (death wipe finished) → FUN_0042391d + FUN_00425a03 (+cond. FUN_0042595a) + FUN_00425bf5 + [0x46cca4]-gated anim string 0x459852 → ret 1 → MissionShell ret 3 (fail screen; ret 2 = launch) | §7j.57 |
| robot-bank zero-fill | mission staging FUN_0040cca2 @0x40cd29..38: FUN_0041cd42 (file rewind [0x4eba20]) then FUN_00402965(ecx=0x7E0, edi=0x4c69e4) zeroes the WHOLE 12-slot bank (0x7E0 = 12·0xA8) — the ONLY +0x9C clear + the 0x4e64c0/0x150 sibling; the only immediate-load of 0x4c69e4 in the binary | §7j.57 |
| critter knockback juice | kinds 4/5/6 survive-hit 25% (RandA&3==0, owner ≠ −1) → FUN_0041a028(x,y,z Q13, robot x,y Q13): 2nd spawner of the 0x4cec38 0x20-stride effect rows (row {w@+0 0, xyz d@+2/+6/+0xA, cos d@+0xE, sin d@+0x12, ttl d@+0x16 = RandA&0x3F+0x1F, kind w@+0x1A = FUN_0041ec1c(5,0)+3}), heading away-from-shooter ±0x10 jitter + FUN_00420608(x+1,y+1,max(z−0x20,0),10,0,−1); kind 7 in-record knock instead (heading d@+0x10, vx/vy w@+0x74/+0x76 = cos/sin>>6) | §7j.23 |
| impact SFX trio | FUN_00421fc2(x,y): [0x4ede58]≠0, RandB()%3 → one of banks 0x4edf7c/0x4edf80/0x4edf84 → FUN_0043a48e(bank,0,x,y,2) — the critter-hit spark sound | §7j.23 |
| octile distance | FUN_0041ebf8(dx,dy) = max(\|dx\|,\|dy\|) + min/2 — the hit metric (and §7j.22 prefilter) | §7j.23 |
| mortar smoke-trail bank | 0x4e66b8 stride 0x68 {d@+0 active, d@+4 ring&7, 8×0xC xyz}: weapon-0xE tick appends prev pos {x−vx, y−vy, z−arc} every 2nd tick; link = record d@+0x32; SLOT ALLOCATOR CLOSED = FUN_00412a4a (20 slots, first active==0, else −1); allocated at spawn by FUN_0040a9ff when the robot slot weapon == 0xE (link := slot, active := 1, ring zeroed; non-mortar link := 0); cleared on free/detonate; DRAW PASS CLOSED §7j.28: FUN_00403938 @0x40442f draws all 8 ring positions (base +8+i·0xC, active/ring words unread) as WEAPONS.BIN frames 0x10+(tick+i)&7, mode 0x12E, screen+map clipped | §7j.22, §7j.23, §7j.28 |
| critter death handlers | six per-kind handlers over bank 0x4cff98 (idx EAX; k4/k5-6 take weapon EDX): k1 FUN_00418835 state 7+presence 0+1× k1 debris; k2 FUN_004188d0 state 7+presence 0+1× k0xD; k3 FUN_00418aa6 state 7+timer 0+1× k7+3× k6 (delays 0/2/4)+FUN_00421f4c; k4 FUN_00418ca4 w@+0x02 := 1, hp 0, state 6, timer 6, 1× k7, weapon {0x24,0x29,0xC} → 3× k7 + 8 effect rows; k5/6 FUN_00418e26 w@+0x02 := 1, hp 0, state 6, sub-timer 0, 1× k7, weapon-gated 3× k7 + 12 rows; k7 FUN_0041896c state 6, w@+0x78 := 1, 3× k7 falling gibs (z 0xFF−r) + 1× k0xD, SFX FUN_0043a48e(0x4edff8,…,3); k1/k4 px-raw coords, others Q13 >>8, z raw-Q13 → stager-clamped 0xFF | §7j.24 |
| critter bounty gate | all six handlers: attacker w@+0x04 ≠ −1 ∧ robot[attacker].type w@+0x2A == [0x4edb90] → score [0x4dd40c] += 30/50/500/75/150/1000 (k1/k2/k3/k4/k5-6/k7) + DAT_0046ccf0 := 2 (score-strip refresh, = the §7j.6 pickup mechanism); env kills award nothing | §7j.24 |
| debris-crush death dispatcher | FUN_0040dce0(idx, knock_mult EDX, heading EBX, dmg ECX), sole caller = the debris physics tick FUN_0040de9c @0x40e13b: guards w@+0x00(kind) ∉ {7,2} ∧ knock_mult > 2 ∧ dmg ≠ 0 (register gloss CORRECTED §7j.44/4 — 7j.24's mag/dmg names were swapped); damage FUN_0040eb3c(idx, dmg) = `if presence { hp w@+0x06 −= dmg }`; sin/cos·knock_mult>>8 + per-kind setter move FUN_00412998(idx, x', y', −1) (kind 7 always, else wall test FUN_0041e9a2); hp ≤ 0 → attacker := −1 + per-kind death dispatch (k4 weapon 0, k5/6 weapon 0x24 = full explosive drops, k5/6 state ∈ {5,6} absorbed) — the SECOND death dispatch site besides FUN_004190bc | §7j.24, §7j.44 |
| debris physics pass | FUN_0040de9c(idx), sole caller the debris tick FUN_00420549 @0x420585 (MissionShell epilogue 0x448076, after the phases, before the armor fade): +0x20 phys = a COUNTDOWN (dec per frame — class 6 runs 6 frames); knock_mult = min(phys,3), critter radius = min(16·phys+0x20, 0x60), mag = kind==12 ? 25 : 2; ROBOT lane (no gate): ALIVE ∧ state≠2 ∧ octile(ΔQ13)>>8 < 0x40 → FUN_0040db9e(idx, knock_mult, heading, mag, debris slot) = FUN_0040e230(robot, mag, owner=debris.+0x28) + facing −1 + robot_move knock (sin·k>>7, cos·k>>7); TERRAIN gate (3-row DAT dword probe rows y−1..y+1 col x−1 at the debris tile−1 — any nonzero → critters) gates ONLY the critter walk; CRITTER lane: presence ∧ mode ∉ {7,6,0xB}, getter FUN_004128ec per-kind scale, |Δ|<0x8000 ∧ octile>>8 < radius, falloff=((radius−1)−dist)>>3 → FUN_0040dce0; POI lane always: active w@+0 ∧ w@+2 ∉ {5,6,7}, octile>>8 < 0x30 ∧ |Δz|<0x20 → FUN_0040dc1b(poi, (0x40−dist)>>2): w@+2 −= mag, ≤0 → panic w@+4:=6 + timer 0 + DEADMAN SFX + k10 debris (E-only, no POI bank) | §7j.44 |
| critter position get/set | FUN_004128ec getter (0x4128d0 table: kinds 1/4 x/y/z <<8, kind 2 raw, kinds 3/5/6/7 x/y raw z <<8) / FUN_00412998 setter (0x41297c table: kinds 1/4 args >>8, others raw; z arg −1 → z untouched) | §7j.44 |
| critter-death SFX trio | FUN_00421f4c(x,y): [0x4ede58]≠0, RandB()%3 → banks 0x4edf88/0x4edf8c/0x4edf90 → FUN_0043a48e(bank,0,x,y,2); twin of the impact trio FUN_00421fc2 (0x4edf7c/80/84) | §7j.24 |
| effect-row spawner | FUN_0041a14f(x,y,z Q13,count): rows 0x4cec38 stride 0x20 via allocator FUN_0041a494 (ages every row w@+0, returns MAX-age — always-evict LRU, 80 rows); row {age 0, xyz d@+2/+6/+0xA, cos/sin d@+0xE/+0x12, d@+0x16 = (RandA&7)·0x10+0x80, id w@+0x1A = i (<8) else FUN_0041ec1c(5,0)+3, w@+0x1C/+0x1E 0}; callers: k4 death (8), k5/6 death (12), controller ballistic landing (0x18 — the k7 body only, §7j.43/2); FUN_0041a028 (§7j.23 knockback) is the parallel writer w/ different +0x16. LANDED (W12-S8/D114: the death-handler callers — the E-ONLY T3 `effect-rows` row) | §7j.24/§7j.43 |
| robot-death blast bank | 0x4eb638, 32 × 0x14 {x d@+0, y d@+4, z-dword d@+8, age/claim d@+0xC, frame d@+0x10} — the MISSIONVIEW §5d/§5e "platform loop" bank; PRODUCER = FUN_0042382c(idx) from the FUN_0040e230 death tail: gate = 0x46af58 claim byte == 0 at the robot tile, slot = FUN_004238ea (first age 0 else MIN-age); anim tick FUN_004238af (frame ++ wrap 0x10→4); CONSUMER (7j.26) = enqueue pair SMOKER.BIN frame 0 mode 300 + frame d@+0x10+1 mode 0x12d (DARKPAL) at sy−0x20 | §7j.24, §7j.26 |
| direct blit codec | FUN_00401e39(img, transp 0/≠0, x, y; ESI bank, EDI dest) — the shared draw_IMG consumer: .BIN = u16 count word0 + int32 dir at bank+2+4*img (offset rel. own slot; corpus-verified 24/24 DEBRIS, 160/160 DANTE), hdr {flags u16 (bit1 hotspot (yoff,xoff) s16×2, bit0 RLE), w, h; w/h==0 → instant skip}; RLE words bit15=skip(→zero-paint when opaque)/literal raw copy, bit14=EOL; dest EDI+y*0x280+x stride 0x280; NO palette modes (vs the §5 flush codec FUN_00401471); counts: DEBRIS 24, SMOKER 17, DROPSHIP 210 | §7j.26 |
| effects mover | FUN_00419f62 (MissionShell @0x44813d): delay −− else x+=vx/y+=vy/z+=vz; kill +0x18:=0 iff x/y/z<0 ∨ x>>13≥[0x4eddec] ∨ y>>13≥[0x4eddf0] ∨ z>>13>0xB | §7j.26 |
| platform anim tick | FUN_004238af (MissionShell @0x447fff): for active 0x4eb638 records d@+0x10++, wrap 0x10→4 (drawn smoke column 2..16 intro, 5..16 loop) | §7j.26 |
| bounded random helper | FUN_0041ec59(n) = RandB()/(0x8000/n − 1) clamped n−1 — uniform-ish [0,n−1] on the 15-bit RandB | §7j.26 |
| dropship ring banks | 0x4e64c0 (12 × 0x1C robot-indexed) + 0x4e6610..0x4e66b8 (6 × 0x1C standalone) {active d@+0, PHASE d@+4, x d@+8, y d@+0xC, alt d@+0x10, img-group d@+0x14, dwell d@+0x18}; consumer draws 7-COL × 5-ROW grids of 0x40 tiles (448×320 px — the 7j.26 "7×7" corrected §7j.27), img = group*0x23 + 7*row+col, bank [0x4edd64] = DROPSHIP.BIN (ArenaAlloc 0x25990; 210 = 6 groups × 35); ends at the trail bank 0x4e66b8; producers CLOSED §7j.27 (resets: FUN_0040cca0 @0x40cd3d pods 0x150 + MissionShell 0x447a7e/0x447a8d; spawners FUN_0041fa51/FUN_0041faf0/FUN_0041fb4b; animator FUN_0041fbb1; + the 0x412b60 exit-dwell reset) | §7j.26, §7j.27 |
| terrain restamp list | [0x4ede24] ptr + [0x4ede28] count → 3-dword records {dest row (y·0x280 basis), tile-x, tile-y}; render-tail readers 0x4067a6/0x406b32 blit each via FUN_00401471 (border tile FUN_00408030 off-window, full LNK path in-window); CELL IS PER-SCREEN REUSE (§7j.49): BRIEF = 49×12 list (alloc 0x24c @0x43d0bd, writer FUN_00440a2d = the objective-minimap window stager, BRIEF-only) / mission = 1296×12 viewport cache (alloc 0x3cc0, writer FUN_0041d954) — resolves the backlog "7×7 screen-address table" hypothesis | §7j.26/§7j.49 |
| NOP stub | FUN_00418a9f (0x418a9f..0x418aa6, empty): called by the k3 death handler + FUN_004197d4/00419943/00419c7c (+ jump from FUN_00419f62) — cut-feature hook | §7j.24 |
| tile-0x62 trap pair | FUN_0040fe93 (robots() caller @0x40bc44) / FUN_0040ff92 (critter FUN_00412f34 @0x413fd7): type-DB byte 0x62 ∧ grid ≠ 0 → FUN_0041a894(damage 100, no score); destroyed → 5× k12 debris (±RandA jitter, delays 0/2/4/6/8). The 0x4c69e4 "160-B stride" was a census slip — TRUE stride 0xA8 (21·idx·8, §7j.25 item 7); anomaly CLOSED | §7j.13, §7j.25 |
| weapon damage table | FUN_00419aff(EAX id) → EAX damage: 2→20, 3→30, 4→40, 5→75, 0xc→5000, 0xd→312, 0x1a→75, 0x24→400, 0x29→250, 0x65→(d+1)·50 [d=2→200], 0x66→(d+1)·300 [d=2→1200], 0x67/0x68→(d+1)·75 [d=2→300], else 1 (inline jump tree, NO memory table — the else-path dump + per-state impact-key map §7j.50: no caller ever passes 0x69; the 0x69 beam re-keys to literal 0x65); 29 callers | §7j.15, §7j.50 |
| difficulty scalar | dword 0x46cbf8, 0..2: cycled (d+1)%3 at NameEntryScreen, save-persisted, zone-7 temporarily forces 2 (GameMain); scales projectile damage 0x65..0x68 (7j.15) AND critter behavior (7j.17: respawn delay DAT_00454edc[d], 0x65 range 172/236/300, engage leash 640/704/768, point-blank fire rate 32/16/8 frames, attack-break 1/8·1/16·never; 12 objdump sites in FUN_00412f34) | §7j.15/§7j.17 |
| critter-actor controller | FUN_00412f34 (MissionShell @0x447fe1): bank 0x4cff98 stride 0x7E count DAT_0046cc2c (FUN_00416458 @0x41646d — the .NME loader, §7j.18); kind table 0x412f18 {k1 0x414c96, k2 0x415216, k3 0x4145c1, k4 0x414079, k5/6 0x41367c shared, k7 0x412f52}; per-frame: presence w@+0x24==0 skip, fuse/hit-flash w@+0x7C decrement, kind dispatch, epilogue (presence mark byte 1, 8-corner z-settle, moved→trap re-probe); state 4 body: species w@+0x02 = SUBSTEPS/frame, modes {0xB dormant (countdown vs 0x454edc[d] → wake mode 9 + hp 0xC8 + species 6 + RandA&3 dir), 7 dying 0x28→0xB, 6 ballistic, 9 seek walk (re-picker 25% RandA&3 / 75% FUN_004181bd + pause 0x20..0x5F + 4-way steppers ±1 + FUN_00415490 per step), 2 range-attack (dist<0x1F4: countdown==4→re-seek else FUN_0040db9e(target,2,heading<<6,1,−1), substep-0 countdown++)}; state 5/6 body: 1/32 facing drift w@+0x72, modes {0xB dormant (BEAMIN at table−9, wake mode 8 + hp 0x96 + species 3 + FUN_0041ec1c(0xFF) heading), 0xA pause→8, 7 dying, 6 ballistic, 5 rise, 8 ENGAGE (gate [0x4dd410]≡0 SP; FUN_00417c00 nearest-alive octile-px; dist<0x60 ∧ leash (d+1)·0x40+0x258 ∧ >0x80 → 1/128 FUN_00421ed6 + aim+step)}; k1/k2/k3/k7 bodies §7j.17 (state 1 wander / 2 sine-walk shooter 0x65 / 3 chase 0x67 / 7 close-combat 0x69). LANDED engine-side (bedlam-core::critter, W12-S8/D114: the k4/k56 subset; the §7j.42 band/roll glosses corrected by §7j.43 — the point-blank RETREAT band, the d=2 never-rolls break, the impact-aimed dives) | §7j.17/§7j.18/§7j.42/§7j.43 |
| critter→robot ranged attack | FUN_0040db9e(robot, mult, seed, damage, param_5): damage word = dword[0x476fe4 + 0x30·param_5] (CORRECTED §7j.42/4: stride 0x30; param_5=−1 → 0x476FB4) → FUN_0040e230(robot, damage-seed=1, owner=the table dword); mult≠0 → robot w@+0x10 := 0xFFFF + FUN_0040c536(idx, cos(seed)·mult>>7, seed, sin(seed)·mult>>7) = the stun/knock applier (SP gate [0x4eaac0]==0, state∉{3,5}: w@+0x0E := seed, walk-probe-gated x/y += v, +0x10 := −1). LANDED (W12-S8/D114: apply_damage + the move_possible-gated knock, Q13 scale) | §7j.42/§7j.43 |
| critter seek-acquisition dispatcher | FUN_00415490(idx): dword@+0x10 (dual-purpose: wander heading 0..255 / mode-9 seek direction 0..3) `cmp 3; ja FATAL` → table 0x415480; 4 directional forward-acquisition probes vs the robot bank 0x4c69e4/0xA8 (tight −4..+0xF ahead on the walk axis, |Δ|<0x18 crossing + z; case 3 reads robot y RAW — quirk); hit → target w@+0x7A, mode w@+0xC := 2, anim w@+0x56 := 0; >3 → "Buggered direction in MOFO" 0x457a3c fatal (fade-cancel 0x420100 + print 0x44d2ac + FATAL EXIT 0x44d2da); the mode-9 walk dispatches the same dword via table 0x412ef8 → steppers 0x417f2c/0x417fe8/0x4180c0/0x41813d (y−1/x+1/y+1/x−1), step-OK → move one unit + call FUN_00415490 | §7j.29 |
| mission extension tags | DGROUP 0x457a57 ".NME" / 0x457a5c ".TRT" / 0x457a64 ".POS" / 0x457a69 ".BDG" — exactly one reference each (0x41648c/0x4170c3/0x41a55d/0x41a5d6 = the four CLOSED loaders §7j.18/§7j.15/§7j.25); 0x457a4c "MOFO\0" = dead tail of the fatal string 0x457a3c, ZERO refs, no ".MOFO" bytes in EXW or EXD, no *.MOFO corpus file — the ".MOFO loader" RETIRED | §7j.29 |
| suicide-bomb trigger | FUN_00417e2f: nearest robot (FUN_00417c00) < 0x30 px → deactivate + 8× debris k1 + 8× FUN_00424355 rings | §7j.17 |
| POI/personnel controller | FUN_00412a98: bank 0x4dabdc stride 0x1E count DAT_0046cbf0 (FUN_00416458 @0x416f6e — the .NME section-8 loader, §7j.18): field map + the WHOLE body decoded §7j.77 [verified] — {active w@+0, HP w@+2 (0x32, the FUN_0040dc1b damage lane), state w@+4 (1 idle/2 settle/3 walk/4 flee/5 ESCAPE/6·7 panic), DEAD seed w@+6 (5), heading w@+8, timer w@+0xA, exit slot w@+0xC, xyz d@+0xE/+0x12/+0x16, draw word d@+0x1A}; spawn state 1 IDLE (the §7j.18 "+4 5" was a transposed store); escape → [0x4eba0c]++, [0x4eba10]=0x32, FUN_00448b80(5000); walker FUN_00415b6c (the quadrant ladder + the ≤4 floor gate); scans FUN_00417c64 (exits) / FUN_00417c00 (robots) | §7j.17/§7j.18/§7j.77 |
| exit/threat slots | 5 × 0x1C @0x4e662c {active d@+0, PHASE d@+4 (1 descend / 2 landed-OPEN / 3 depart — §7j.19 reread of the 7j.17 "kind"), x/y d@+8/+0xC, altitude d@+0x10, img-group d@+0x14 (7j.27: the animator's per-tick DROPSHIP.BIN frame selector), dwell d@+0x18 — RESET TO 0 BY FUN_00412a98 @0x412b60 on each POI rescue (multi-POI elevators), cleared on escape}; nearest scan FUN_00417c64 (gate phase==2); producer CLOSED §7j.18: FUN_0041fa51 = the EXIT-PAD ACTIVATOR (arg = a 0x4e44f8 .PAD slot index; dedup registry 5×d @0x46cd20; stamps {1, 1, pad.x·0x20+0xF, pad.y·0x20+0xF, 0x400, 0}; sole caller FUN_00433980 case 0x1B @0x43900e (§7j.19); animator FUN_0041fbb1 §7j.19; boot reset MissionShell 0x447a8d | §7j.17/§7j.18/§7j.19/§7j.27 |
| escape-craft animator | FUN_0041fbb1 (MissionShell @0x448012, per frame): 3 machines over the 0x1C frame {active@+0, phase@+4, x@+8, y@+0xC, alt@+0x10, img-group@+0x14, dwell@+0x18} — the 5 exits + the dropship @0x4e6610 + the per-robot pods @0x4e64c0 (gated [0x46aed4+idx·4]==0, the CLAIMED/no-extract latch: boot-clear GameMain 0x41c408 = the sole EXW writer (memset; D133 — the original FUN_0040e230/FUN_00449c94/FUN_0044a38a/FUN_00408e99 "writers" are readers; EXD twin 0xf929c adds the MP-lobby-pick setter) — the latch ALSO gates the MP respawn @0x40e7a1); dropship landing = extraction sweep (states 3/4 → 5, _DAT_004dc680++, SFX _DAT_004edfe0), depart → _DAT_004dc67c=1 (complete; readers MissionShell 0x4486d5 + FUN_0044425c ×2); pod landing = payout 100·w@+0x94+5000 + state 6 (robot RELEASED) + msg. §7j.27 per-tick write map: phase 1 alt −0x20/(v>>2)·3 + img-group toggles 0↔1; phase 2 alt := (RandA&7)==0 jitter, exits dwell++>0x78, dropship dwell−−, pods ONE TICK then payout; phase 3 alt += (alt>>2)+1, x −= group·4, group ramps 2..5 then oscillates 4↔5, alt>0x200 → active 0 | §7j.19, §7j.27 |
| dropship deployer | FUN_0041faf0: stamps 0x4e6610 {active 1, phase 1, img-group 0, alt 0x200, x beacon.x<<5, y beacon.y<<5} from beacon 0x4eabb4/0x4eabb6, clears 0x4eabb0/0x4eabb2 (x/y words SURVIVE — renderer 0x4070c0 reads the always-0 z word 0x4eabb8 as a no-op sy nudge); caller MissionShell @0x44832f/0x448375 (countdown 0x4eabb2 == 0 ∨ all robots dead/state-3); beacon armer FUN_004247b5 [§7j.20]; boot reset MissionShell 0x447a7e | §7j.19, §7j.27 |
| pod spawner | FUN_0041fb4b(idx): stamps 0x4e64c0+idx·0x1C {active 1, phase 1, img-group 0, alt 0x400, x/y = robot pos>>8 (Q13→Q5)}; caller FUN_0040b9f6 when countdown w@0x4c6a10+idx·0xA8 == 0 (msgs 9/10/0xB for the player's first 3 robots); the 0x4c6a10 producers [§7j.20]; bank reset = FUN_0040cca0 @0x40cd3d (memset 0x150 = 12 records, every mission spawn) | §7j.19, §7j.27 |
| extraction-beacon armer | FUN_004247b5(EAX tx, EDX ty, EBX z, ECX idx): guard 0x4eabb0; 0x4eabb2 = 0x197 (0 if player-0 alive-count == 1); 0x4eabb0 = 1; 0x4eabb4/6/8 = tile trio (z dead store); robot.state = 3; spread-teleport FUN_004248c8; SFX 0x2A. Sole caller FUN_00433980 @0x433cfb = ~25 (zone, .PAD slot) extraction pads | §7j.20 |
| spread-claim picker | FUN_004248c8(&tx,&ty): first free slot of 12×u16 0x4eabba (bound DAT_0046ccbc), marks 1, returns beacon tile + {center, 8 neighbors, (−2,0),(0,−2),(+2,0)}; ≥12 → out-params untouched (callers store garbage); claims never released; callers FUN_004247b5 @0x424865 + FUN_0040b9f6 @0x40c08f | §7j.20 |
| pod-deploy countdown writers | w@robot+0x2C (0x4c6a10): FUN_0040cca0 spawn tail @0x40d132 stagger 1+k·(2000−m·1000/27) per player group (m = linear mission 0x46ae8c); FUN_0040e230 MP respawn @0x40e89d = 0x28; reader/decrementer FUN_0040b9f6 (brain gate) | §7j.20 |
| per-player selected anchor | 0x4c71c4: 4×0xC {x>>8, y>>8, z}, spawn-seeded by FUN_0040cca0 tail (selected robot idx DAT_0046cbd4), renderer-updated FUN_00403938 @0x403994/0x4039d2/0x403a27; sits immediately before the 0x4c71f4 bank base | §7j.20 |
| pad-trigger dispatcher | FUN_00433980 (sole caller FUN_0040b9f6 @0x40bd58 when state∈{1,4} ∧ order 0x46cc30[idx]≠−1): FUN_00422e5e = the PAD-TILE PROBE (DAT byte 0xFF → 999×8B .PAD slot scan @0x4e44f8 → slot id; revisit latch 0x4eb9fc/counter 0x4eb9f4); zone switch jmp [zone−1 ·4+0x433964] on 0x4edd8c (A..G → 0x43399f/0x434058/0x435bda/0x4386c5/0x432c8e/0x439323/0x439ae2) → per-mode [0x4edb88] / per-mission [0x4edd88] gates → per-mission slot cascade/table → actions: ELEVATOR RIDE (ride-record bank 0x4dcdbc stride 0x24 {+0/+4 dest tile, +0x18 latch :=10, +0x1C rider gate −1/idx}; robot state :=2, +0x84 := arrival plat 0..0xE, pos := dest·0x2000+0x1000, y-stamp tail 0x43475f), DOOR FUN_004223b8(rect 0..0x25, 1\|2), BEACON FUN_004247b5 (21 SP slots), EXIT pairs DOOR+FUN_0041fa51(slot) (zones F/G), MSG FUN_00424a6f (zone A M1 only); zone E = rect/dest overlay restage, NO case actions (§7j.46/4); H2H = rides-only tables | §7j.19, §7j.21, §7j.46 |
| zone-A message shower | FUN_00424a6f(id) (sole caller 0x433d07): SP-only ([0x4edb88]==0), per-id show-once latch word 0x4eb5f8+2·id := 1, SFX TEXTBOX1 0x4edfd0, name = "BOOT_CAMP_%03i" (prefix 0x458ca7, fmt 0x458cb2), section finder FUN_00424679 in the LANGUAGE.{ENG..DCH} blob [0x46cbb4] (alloc 0x13C68 @0x41d64d, load 0x41c1fb, lang selector [0x4eba1c]), `]`-terminated line records 0x46-stride, TINYFONT 0x46cdb0 wrap, window 0x4eaab8 {x=0xF0−w/2, 200}, bank 0x4e8818, timer [0x4eaac0] := 0xFDE8; ticker/drawer FUN_00425010 (MissionShell 0x448381) decrements; COMMAND sites 0x40a2bc/0x40a396 dismiss (strictly more than 8/20 timer decrements); 0x40c570 gates the state-0 write; ids 0..0xE = BOOT_CAMP_000..014 in LANGUAGE.ENG (421 sections) | §7j.46 |
| ride-record bank | 0x4dcdbc stride 0x24 ≥16 records (gates +0x1C: 0x4dcdd8..0x4dcff4); filled from .PAD slot records 0x4e44f8 by the 0x426058-family stagers at dispatch; the 7j.19/7j.21 "dword tables 0x4dcdbc..0x4dd330" = this bank | §7j.19, §7j.21, §7j.46 |
| critter/POI (.NME) loader | FUN_00416458 (the mission-load dispatcher's critter hop): stages ".NME" (@0x457a57) → 8 fixed-order sections (widths 10/10/8/8/10/8/6/8) feeding critter states {2,1,5,4,3,6,7} + 4 POIs/record; spawn multipliers by difficulty; hp = base+(base·d)/27, bases 0xAF/0xC8/0x96/0x5DC/0x9C4; corpus-exact on all 37 files (ZONEA/M1 16-B orphan tail unread) | §7j.18 |
| command-record consumer | FUN_00409138 (MissionShell @0x448030 after FUN_00410644+FUN_00449c94): records 0x4dd4a0 stride 0x80 count DAT_0046cbe0 (count NOT reset by the consumer; the recharge pass runs once at the loop exit for EVERY robot × 7 slots: enabled ∧ cooldown≠0 → −−); spot@+3 → robot+0x14 + cursor angle 0x4dc678; robot = rec_idx·0x46cbd8 + id@+1; bit0 select → move-target words (state ∉ 3/4/5) + auto-arm state:=1/stop:=10⁶ (state ∉ 2..5) — and the bit0 block BUMPS the word pointer +2, so bit0∧bit1 records take the order triple from +0xB/+0xD/+0xF; bit1 order → 0x4dc6bc:=1, triple 0x4dd484/88/8c, 0x4eb940..50 clear, then (alive ∧ deploy-delay+0xA0==0 ∧ state≠2) the 7-slot loop with FIRE GATES mask∧cooldown==0∧ammo≠0: artillery w9..0xB INLINE 1× (type=id, pos+0x100, z=(z+0x15)<<8, no velocity, cooldown 0, mask bit XORed UNCONDITIONALLY); mines w0x10..0x12/0x14..0x16 INLINE 2/4/6× types 0xF/0x13 (2×RandA jitter ±0x20 on the order target, octile>>3 normalize, vel>>2/>>1, ttl RandA&0xF+1, arc 0x900−RandA&0x2FF, class 4, cooldown 8, 4 draws/record); grenades w0x1B..0x1E INLINE 4/6× types 0x1A/0x1F (3D vz from the order z, ttl 0x32∓/＋RandA&0xF, arc 0xB00−/0x900−RandA&0x2FF, class 0, trail:=0); rocket w0x20 INLINE 1× type 0x24 (no jitter, ttl 0, cooldown 5, arc := angle_pair); the AI-order families w2/3/4,w6/7/8,w0x18/0x19,w0x21..0x23,w0x25..0x28 with counts 3/2/1, 0/1/2, 1/2, 3/6/9, 1/2/4/6 (internals OPEN); auto-rearm first id≠0∧ammo≠0 slot when the mask emptied + msgs 0x1C..0x21; idle AI ticks (10/9/2/6) when deploy-delay≠0 ∧ frame&3==0 | §7j.17, §7j.37 |
| mission-objective resolver | FUN_00448b80(type: 5000 = rescue, else destroyed object type): 6×0x20 slots @0x4eaaee {remaining w@+2, type w@+6, status w@+0xC, quota w@+0x1E}; kill-stats [0x46cbf4]+type·0x14; mirror-row wipe 0x4796d7/d8; msgs 0x26/0x27/0x34, all-done 0x28+0x29; DAT_0046cd00 = phase state 1/2/3/4; zone-7 counter [0x46cce0] types 0x44..0x47 | §7j.17 |
| floor probe | FUN_0041e411(px,py,z): level try +1/−2; per-type height entry [0x4edd60+2+(type−1)·4] → in-tile 0x20×0x20 byte map @(x&31)+(y&31)·32 at +6; floor = level·0x20 + byte; 0x1F = top-of-stack (sibling of FUN_0041eaa1 §7j.14) | §7j.17 |
| walk/settle helpers | FUN_0041f8f9 8-sample walk probe (0x4543e4/0x454404, level ∧ height-diff ≤3); FUN_004186fc standing-on-scenery (mirror 0x4796d5); FUN_004182c3 8-corner z-settle (snap +0x13/+0x0B); FUN_0041642d anim ctr wrap; FUN_0041286f 50×0x22 free slot; FUN_00412848 400×0x36 free slot | §7j.17 |
| terrain-structure loader | FUN_004170a6 (call 0x416487 in the dispatcher FUN_00416458): ".TRT" section @staging buf 0x4dca0c; clears 250×0x20 @0x4cccf8; count→[0x46ccd4]; rec (canonical frame, active@0x4cccf8): active=1, state=1, frame=0, fire=0, hp=250+(250·mission)/27, x/y/z tiles; stamps tile 0x66 @byte[[0x4edd58]+x+y·w+z·w·h] + word 1 @word[[0x4ede20]+2(x+y·w+z·w·h)] (the .DAT/.TOT file volumes) | §7j.15 |
| TRT anim/fire machine | FUN_00417264 (MissionShell @0x44807b, every frame): states 1 idle→2 alert (frames 0..7→TOT word frame+1)→5/6/7/8 aim S/N/W/E (octant vs nearest robot FUN_00417c00 dist<0x81)→FUN_00417698 fire at frame top + 4-frame muzzle (words 0x17..0x1E); 3/4 = death/settle; FUN_00417210(idx,n) = mirror word n+1; FUN_00417652 = frame remap 0xF→7, 6→0xE | §7j.16 |
| TRT fire routine | FUN_00417698: lane test |lateral|<0x28 px + direction + ≤2 levels vs robot bank 0x4c69e4/0xA8; arms fire_ctr@+0xC; odd ctr → FUN_0041286f free slot → projectile type 0x66 (damage (d+1)·300) @0x4cc654+slot·0x22 {x,y tile·0x2000+0xF00, z<<0xD, +0x16=0x14, unit vx/vy}; structures never move | §7j.16 |
| map volume loader | FUN_0041dc5a (MissionShell @0x447b3a): ".TOT"→[0x4ede20] (u16 W,u16 H header + 8 planes W·H u16 → [0x4eddec]/[0x4eddf0]/[0x4eddf4]), ".DAT"→[0x4edd58] (same header, u8 planes, >0x7F sanitized→0), ".CGR"→[0x4edd60], ".BIN"→[0x4ede1c] (u16[bank+0] = the sprite COUNT → write-only cell [0x46cdb8], §7j.36), ".MIN"→[0x4edd9c], .LNG/.LNK→0x45cdda, ".PAD"→999×8B slots 0x4e44f8 stamping 0xFF; FUN_0044661b = the EDITOR\ZONE restore reload; FUN_0041dbed/FUN_0041cd90 = path/section opener (handle 0x4eba20) | §7j.16 |
| TOT materializer | FUN_00440a2d (sole caller FUN_00440dc2, §7j.49): 7×7 tiles × 8 z: TOT word≠0 ∧ DAT byte==0 → mirror word@0x4796bc = word + seen@0x4796cc; BRIEF-screen only (the objective-minimap window; the in-game full mirror build is init_tiles §MISSIONVIEW 2) | §7j.16/§7j.49 |
| map-click pick | FUN_00419943 (caller FUN_00410644 ← MissionShell @0x448021): rect list 0x4787c4/{center@+8/+0xC, w@+0x14} count [0x46ccd8] (written by renderer FUN_00403938) with octile cost FUN_0041ebf8; else screen→iso ((p−0xF0)·[0x4ede54])/0x1E0 + TRT scan; ret 0=ground / k+1=rect / (idx+1)\|0x2000=structure; FUN_00418a9f = empty stub | §7j.16 |
| click order target | {x,y,z} = 0x4dd484/0x4dd488/0x4dd48c written by FUN_00410644 (ground iso / rect / structure tile-center) AND by FUN_00409138 (command-record bit1, words@+7/+9/+0xB); readers FUN_00409138 ×6, FUN_0040af98 ×3, FUN_0040a56f/0xa7a1/0xace8/0xb615/0xa9ff ×2 each, FUN_00449c94 | §7j.16/§7j.17 |
| scanner overlay | FUN_0041ec81 (MissionShell @0x48142): corner widget box 0x1EE..0x272×0xC3..0x147, grow [0x4edd68]→0x40, asset GAMEGFX\SCANNER.BIN; FUN_0041ee20(cx,cy) around the SELECTED robot ([0x46cbd4]+[0x46cbdc]): icons via FUN_00402572 (128×128 blitter→[0x4eddb8]) — 1/2 robots sel/rest, 4=0x4cffbc, 5/6 linked blink, 7/0xD tiles, 8=TRT, 9/0xA objects, 0xB arrivals, 0xC pads | §7j.16 |
| nearest-robot probe | FUN_00417c00(px,py,&dist): octile over robot bank, ret idx; callers: turret machine + FUN_00412a98, FUN_00412f34 ×4, FUN_00417e2f (the robot targeting family). FUN_0041ebf8 = octile distance max+min/2 (51 sites) | §7j.16 |
| terrain-structure array | recs @0x4cccf8 + i·0x20, i < [0x46ccd4] — {active@+0, hp@+0x10, x tile@+0x14, y@+0x18, z@+0x1C}; externally 1-based (dword[0x4cccd8+id·0x20] = rec id−1 active; 0x4cccd8 = id-0 guard) | §7j.14 |
| terrain damage resolver | FUN_0041bc1c(x Q13, y Q13, damage): match rec by tile → hp−=damage; hp≤0 → active=0 + floor word [0x454a04+4·zone] → TOT @0x4796bc+30·tile+2z, seen @0x4796cc, DAT volume=0, debris K0xF, splash at first free level | §7j.14 |
| terrain-height probe | FUN_0041eaa1(x Q5, y Q5, z): DAT volume byte 0 → miss; else height = [0x4edd60] bank ptr (h−1)·4+2, +6 header, byte[(y&31)·32+(x&31)]; hit iff z ≤ (z>>5)·0x20 + height | §7j.14 |
| weapon-anim disburser | FUN_004124a4(rec idx): rec 0x4c71f4+0x36·i (400 slots, free-slot FUN_00412848), kind word@+0; w2..4→K2 (±3 jitter), 5→K3, 0x24→K6, 0x29→K9, {0xE,0xF,0x13,0x17,0x1A,0x1F}→K0xC; z−10; 9..0xB clear-no-debris | §7j.14/§7j.17 |
| weapon-anim tick (400×0x36 bank) | FUN_00410823(phase 0..3), 4×/frame from MissionShell: bullets 2..4 = 2 tested sub-steps, net TWO committed steps per call (3 moves − 1 rollback; tick += 6 — corrects the 7j.22 "1 committed" gloss; actor/terrain hits re-add + disburser K2 + impact pair via FUN_00419aff(type); records FREE ONLY at tick>99 — impacts do not kill them); shell 5 = 1 move, free on bounds/tick>100/z-OOB, critter-lane hit (odd phases) stores (x,y)+K3 debris at z>>8−10, MP-lane hit → disburser, floor hit → impact pair 75 + disburser + FREE; artillery 9..0xB phase-0-only, tick++, fall 0x200/tick to FUN_0041e411 settle (floor<<8), tick==0x18 ∧ player-kind → FUN_004245c9, burst window tick−0x20 < dword[0x456c78+4·TYPE] (durations 2/4/7 BY TYPE) walking the pair lists PTR[0x456bf0+4·(tick−0x20)] (sentinel 500, FUN_004244a1 5000-damage blasts + 50% K0xB), past the window → disburser + free; ballistic family gravity arc −0x100/tick with the per-type bounce/roll semantics (7j.22 item 6); rocket 0x24 class-countdown launch delay → straight flight, floor → 400 impact, ttl>0x64/bounds → free; homing 0x29 launch delay → z-ease ±0x200 clamp [0,0xFF00] + ground-lift ≥(z>>8)−4, heading := (heading + angle-diff·4)&0xFF over the target Q13 delta, vel = 2·(sin[heading]>>4, sin[heading−0x40]>>4), forward probe FUN_0041e56d, avoidance ±4-sector LEFT-first (left-OOB also climbs z+=0x600), dead-target gates → disburser+fizzle, floor → 250 impact, ttl>0xC8/bounds → free | §7j.22, §7j.37 |
| byte-angle sine table | SINTABLE.BIN (512 B = 256 i16): word[a] = round(sin(a·π/128)·32767) [corpus-verified]; FUN_0041eb65 "cos" = movsx word[base+(a&0xFF)·2], FUN_0041eb77 "sin" = the same at (a−0x40)&0xFF; the 64-word sector-scan threshold table (FUN_0041eb7d, base+4) = words[2..66] of the same array — one dual-use file table at [0x46cbd0] | §7j.37 |
| projectile disburser | FUN_004126dc(rec idx): rec 0x4cc654+0x22·i, TYPE word@+0 (0=free; NOT plain "active"); 1→K2, 0x65→K0x14, 0x66→K8, 0x67/0x68→K4, 0x69→SILENT shared-epilogue return (no debris, no clear — defensive; the beam handler never calls it, §7j.50); coords z NO −10; robot-hit expiry via FUN_004197d4 (|dx|<0x10 Q8, |dz|<0x20; states 0x65/0x67/0x68 ONLY — 0x66/0x69 never damage robots) | §7j.14, §7j.50 |
| splash gates/eviction | FUN_0041bd78: first z ≥ min(z,7) with DAT 0 ∧ seen 0; FUN_00424355 gates: DAT-empty ∧ TOT word 0 ∧ claim byte[0x46af58+tile]=0; full ring → evict max-age + FUN_0042394a flush | §7j.14 |
| splash records | 250 × 0xA @0x4e9778 {x,y,z,delay,age}; ticks in the epilogue | §7j.10 |
| splash life | stamps water_base[zone]@age1, base+0x16@age40, frees @age≥47; body odd frames only | §7j.10 |
| z-structure writer | FUN_0042394a: zword@rec+2z, seen@rec+0x10+z, DAT volume byte | §7j.10 |
| DAT volume read | FUN_0041eb28(x,y,z): byte[DAT+tile+zoff[z]], 0xFF→1 | §7j.10 |
| tile-claim bank | 0x46af58, 10000 B arena @mission-load; NOT order-written — FUN_004254e1 @0x447b85 memsets 0 then stamps the door-rect tile claims (the §7j.10 gloss corrected) | §7j.10, §7j.63 |
| sidebar select strips | [0x1E7,0x217]/[0x219,0x249]/[0x24B,0x27B] × y[5,0x35]; F1/F2/F3 latches 0x4edc0c/10/14 | §6c.2 |
| sidebar order rows | x[0x1E9,0x275] × y[0x57,0xB8]; row=(y-0x57)/14 clamp ≤6; keys 1..7 latches 0x4edc18+4k | §6c.3/4 |
| sidebar redraw flag | DAT_0046ccec countdown: set 2/3 by producers, dec+FUN_00408403 in the FUN_00403938 tail | 0x407205 |
| map-toggle strip | x[0x213,0x24D] × y[0x1B5,0x1CF]; MSpace latch 0x4edc08; writes 0x4eb8dc=5, toggles 0x4edba0 | §6c.1 |
| map overlay draw | FUN_004089b1: clear 0x4b000, TABLE.BIN img0 480×480 @(0,0), stamps row'=0x80+r+c−2z / col'=0xf0−2r+2c, markers 0x55..0x59; non-returning tail | §7e |
| territory variant | byte@0x4c420c+tile; zeroed 0x27d8 by MissionShell; 11×11 max-stamp rings 7..1 (dwords 0x454cf8) around robots (FUN_00408dcc ← robots() state 2) | §7e |
| MAPTRAN ramps | u32@(0x4dd464+4i) ← GAMEGFX\MAPTRAN{i}.TRN (256 B each, i 0..7); ramp[mask byte] = palette byte | §7e |
| PALTRAN ramps | u32@(0x4dd444+4i) ← GAMEGFX\PALTRAN{i}.TRN, slot 0 NULLed after load (MISSIONVIEW §8.2 producer closed) | §7e |
| LNK map lookup | cw = word@(0x45cdda + 2*w) — the LNK image doubles as the map type→mask index; masks = .MIN bank [0x4edd9c] (16 B/cw) | §7e |
| pickup range tables | A/B dwords @0x454a58/0x454a74 (7 terrain sets, 4-word closed groups → cases 1/3/2/4 + 9/7/8); floor word table @0x454a90; the set [0x4edd8c] = zone_index+1 ('A'+set−1 = the path zone letter, §7h.4) | §7h, §7h.4 |
| pickup probe latch | {z@0x4dc688, x@0x4dc68c, y@0x4dc690}: FOUR writer sites in get_z_pos (z / z+1 / z−2 empty-search / slope z+1), each gated on the probed DAT plane byte == 3, last-write-wins, no auto-clear; SOLE consumer = the robots() move-toward-target block: clear −1 (0x40bef2) → robot_move (0x40bf06) → test ≠ −1 (0x40bf0b) → mirror-word range test → DAT byte := 0 + mirror word := floor word + seen := 1 + {x,y,z} staged 0x4dc6ac/b0/b4 (MP-only FUN_00425647 tails) → FUN_0040eba0; corpus: ZERO pickup cells ZONEA/M1 (set 1), 601 ZONEB (set 2) + 149 ZONEF (set 6), zones C/D/E/G none | §7h.4 |
| TOT mirror staging | init_tiles@00407e11 (MissionShell load): EVERY nonzero TOT plane word → mirror @0x4796bc+30·tile+2z (DAT byte UNCONDITIONAL); the DAT==0 gate is the SEEN flag only (byte @+0x10+z := 1) — the pickup/decoration words at DAT≠0 cells DO stage (corrects the §7j.16 gloss; FUN_00440a2d restamp = the incremental word+seen path) | §7h.4 |
| TOT plane-6/7 semantics | CLOSED §7j.47/D119: planes 6/7 = ordinary z-levels 6/7 (tall-structure tops; per-level sprite ids, e.g. ZONEA/M1 (17,25) column [454,1354,1355,1356] at z=4..7); NO z≥6 gate in ANY consumer — the FUN_00403938 restamp z-stack loop runs z 0..7 (outer `cmp 8` @0x406863, chain @0x40695c; Block-1 restart draw @0x406882..0x406941 gates on word≠0 ALONE, no seen — so every nonzero plane word draws), the overlay scanner 0x408a49..0x408ade walks planes 1..7, the range consumer 0x42035c..0x4203a5 planes 0..7; corpus: 36/37 missions (only ZONEG/M1 zero; 8 016+2 882 words, 6 504 overlay / 2 792 standalone); value domain ≡ planes 1..5 (35..1868 vs 33..1868); the FORMATS §2 plane-value=POS-slot hypothesis REFUTED (9 217 live/1 681 empty .POS resolutions = coincidence; ZONEA's 1355/1356 hit empty slots; p7==p6+1 at only 83/9 296 cells) | §7j.47 |
| §5d tail: robot plates + bank staging | CLOSED §7j.48/D120: §5d item-1 = TELEPORT.BIN (10 imgs, 0x46af38 — beam, not shield; clamp 0..9 fits), item-3 = SHIELD.BIN (4 imgs, 0x46af44 — RandA()&3 spawn + (+1)&3 shimmer); TELEPORT/SHIELD/ROBNUMS alloc (FUN_0041d954: 0x6d60/0x1b58/0xbb8) + LoadFile (FUN_0041df10) at EVERY MissionShell head (@0x447860/0x447b3f, straight-line, no gate — SP included); ROBNUMS.BIN (9 imgs) has ZERO game readers = dead data; MP name plates draw TINYFONT (0x46cdb0, 118 glyphs, ASCII−0x21) glyphs `sx + u32[0x4e44c8+id*4] + 6*i`, gate [0x4edb88]≠0 @0x403fb9 (SP never), filter g ≤ 0x40, centering table = 32−3·strlen per id (writer 0x447ce0..0x447d85, toupper + −0x21, raw names 0x4e43e0 9 B/slot); NO unstaged-skip in enqueue/flush (only early-outs: bx/by<0 + unknown-mode RET) — bank==0 never reaches the flush (loads precede the first frame) | §7j.48 |
| map present | FUN_00401107 map mode: 480×480 from backbuffer base, stride 640; button chrome 0x8f/0x5f/0x5e @ (0x213,0x1b5) | §7e |
| backbuffer | [0x4ede18] = ArenaAlloc(0x64000) = 640×640; overlay clears 0x4b000 (480 rows) | §7e |
| order table | 7×0x0E groups @ 0x4de664+type*0x62; group word0/+0x36+8i (default probe), word1/+0x38+8i (gate) | §6c.6 |
| DAT tables | z-base@0x4eaacc, y-line@0x4ea900 | 0041eb28 |
| loader | load_mission@0041dc5a; paths@0x44670c; sweep ≥0x80→0 planes 0..6; PAD→DAT 0xFF @ plane=kind | 7c |
| CGR height byte | CGR[2+4(type−1)+dir[type−1]+6+(sy<<5)+sx] (no codec) | 0x41e328, 7c |
| MRK word 3 | spawn z level (z = w3<<5 − 1) | 0x40d06d, 7c |
| CGR/DB ptrs | DAT_004edd60 (CGR), DAT_004edd58 (DAT), 0x4796bc/cc (type DB 0x1E stride) | 0041e231, 00407e11 |
| viewport cache | DAT_004ede24 36×36×12 B (screen off + tile deltas), count DAT_004ede28 | 00407e11, MISSIONVIEW §2 |
| terrain bank | BIN→0x4ede1c (MISSION{A..G}.BIN sprites), LNK→0x45cdda = per-frame anim link; §7j.36 census: content readers = the terrain loop (0x40692e/0x4069f5/0x406a40/0x406b15) + the BRIEF minimap drawer FUN_00440c34 (0x440d1c/0x440d93, type-DB word → FUN_00401471 into the backbuffer; sole caller FUN_00440dc2 §7j.49) — pixel paths only | MISSIONVIEW §1/§3/§4, §7j.36/§7j.49 |
| BRIEF objective bank | 24×14 B @0x4e9628: +0/+2 marker x/y, +4/+6 TOT row/col (snapshot window), +8 counter, +0xA render latch; staged by the BRIEF text parser (0x43e5b1..0x43e7b2); FUN_0043dc65 = per-objective panel + FUN_00440dc2 = the minimap snapshotter (7×7×8-z render → 2× downsample → 256×256 cache [0x46cbb0] alloc 0x10100, flag [0x4dc6c0], consumer blit FUN_00402a28 @0x43d9a2); BRIEF screen FUN_0043d00b (GameMain 0x41c4d5, ret 2 = launch) | §7j.49 |
| BIN container grammar | u16[bank+0] = sprite COUNT (→ write-only cell 0x46cdb8, no .text reader; blits mask id&0xFFF); directory entry = bank+2+4·id, sprite = entry + u32[entry] SELF-relative (monotone, in-file, 11/11 banks incl. B6/D5/E6); record = u16 fmt@+0/dy@+2/dx@+4/gate@+6/rows@+8 + stream; FUN_00401471: fmt≥4 u8-RLE, 1..3 u16-RLE, 0 raw, gate==0 → RETURN, rows==0 → RETURN; FUN_0040167a reads gate but IGNORES it; FUN_0040179b = +2 head, gate skipped; all real terrain sprites fmt 7; MISSIONVIEW §4 "bank + u32[bank+4+id*4]" CORRECTED to the self-relative form | §7j.36 |
| BIN 9-sprite scratch family + stamp | [0x4edd94] := u32[0x454b00+4·set] @0x4479b4 — bases {0x490,0x6ED,0x638,0x490,0x3A,0x3A,0x6ED}; records = 6-B stub {fmt=0,dy=64,dx=64} + 4096-B image (span 0x1006), image[0..3]=0 → gate/rows 0 → UNDRAWABLE forever (stamp never writes image[0..0x1F]); FUN_00401010 (0x401010/0x40108b, head of the PRESENT FUN_00401107, every present) downsamples the 480×480 viewport 5× and deshears (2:1) 9 tiles at image+0x20 row-stride 0x40 page-step 0x806 — but NO code ever draws them (LNK identity on all 63 ids ×7 zones; [0x4edd94]/0x454b00 sole readers = the stamp/boot); A/B/C/D TOTs reference all 9 ids (E/F/G none) and render NOTHING (gate-0 return) — VESTIGIAL | §7j.36 |
| dither noise bank | 0x4e6ed8 (2048 B .bss ring, cursor 0x4ddb30), bytes {0,0xFF}, `RandB()&3==0` 25%; boot fill MissionShell 0x447b13, churn 15 B/frame 0x448147 | §7i |
| dither blit | FUN_00401ae6(y,h,x,w,src_off,mode): mode 0 = rep-movsb full copy (dead/unoccupied boxes), mode ≠ 0 = nonzero-only overlay (hit flash); reseed `RandB()&0x1ff` when src_off+96 ≥ 0x800; seed `(RandB()&0x7fff)/15` clamp ≤ 0x7f5 | §7i |
| SFX bank→name map (COMPLETE) | 202 durable assignments, zero unnamed durable cells: mission set 0x4edf60..0x4edfbc+ELEV/BEEP/TEXTBOX = 27 registers by FUN_0043a1d3 (MIDIGUN dup at 0x4edf70 quirk), screen sets MENU1/2+BEEP1/4/5/7+TEXTBOX1+DOOROPEN/DOORCLSE (0x4edfc0..0x4edfec, cells reused per screen; 0x4ee00c/0x4ee010 = the debrief MENU1/2 alias), mission-extra 0x4edfe0..0x4ee008 (BEAMIN/THROW/BIOFIRE/PEXPLODE/CACODETH/SQUAWK/GRUNT1..3), speech 0x4ee014+8i 53 records {A,B} (95 files, 11 empty +4 slots, pair slot-order flip at SPCH16); GFX 0x46af2c..0x46af54 + 0x4eddXX/0x4edeXX/0x46cbXX families + G-variant picks (language index 0x4eba1c==1, edition gate [0x4edd8c]>4 → GRILLA family); palettes SHARE role cells (0x4edbf8 current-screen PAL ×6 names, 0x4edbfc TXPAL1..3, 0x4edc00 DARKPAL family); full dump ghidra-project/exw-banknames.txt | §7j.30 |
| SFX register/play family | FUN_0043a36e = 1-voice register, FUN_0043a39c = 4-voice register (clone pair; stage via scratch cell 0x46af0c → arena 0x2b11 → 0x44c64c returns the VOICE-BASE handle — SFX cells hold handles, not pointers); FUN_0043a48e = play/steal (x,y=−1,−1 → vol 0x7f/pan 0x8000; else FUN_0043a3e0 pan / FUN_0043a447 vol vs listener 0x4edde4/0x4edde8; 4-voice probe 0x44c5ac, steal by priority [0x4ee1c2+2v]>>16 + age [0x4ee2e2+2v], start 0x44c904); speech bypasses it (indexed slot pick + 0x44c8c4 direct, vol 0x7f00) | §7j.30 |
| radio-warning dispatcher | FUN_004239ef(id, channel): 4-channel message queue 0x4eb954, stride 0x28 {8 id+1 words +0..+0x1C, insert idx +0x20 wrap 8, voice handle +0x24}; dedupe per id per channel; ids 0x19..0x1B flush their channel then post at slot 0; 55 call sites = the 53-line [WARNINGS] id map (§7j.53); channels 0/1/2 = squad slots, 3 = system (drained first) | §7j.53 |
| radio-warning consumer | FUN_00423a85 (MissionShell @0x447ff5, per frame): channels 3→0, oldest slot first, one per channel per frame; voice leg (ids 0xF/0x29 skip; gates [0x4eb93c]/[0x4ede5c]/[0x4ede58]): still-playing poll 0x44c5ac keeps the slot queued, else play take A/B = **RandA bit0** from 0x4ee014+8·id via 0x44c8c4 (vol 0x7f00), handle := ret+1; consume leg: slot := 0, roll the 4×0x26 display ring 0x4ea13c {text[0x20], reveal u16 +0x22, valid u16 +0x24} (active = record 3, latches 0x4ea1d0/d2), stage text 0x46c18c+id·0x30 (WARNINGS table, GameMain-loaded from LANGUAGE.*), typewriter render tail (tables 0x454c20/0x454b70); both structures MissionShell-cleared @0x4479de/0x4479fc | §7j.53 |
| heat machine | CLOSED §7j.55/D127: FUN_004100b7 = the HEAT-IN (sole caller robots() phase-1 0x40bc72, amount 0x14 on a nonzero +0x18 scorch byte): the +0x98 DAMPER (equipment stat 0x2C ×200, spawn 0x40d013/MP-respawn 0x40ea59) absorbs first — pool −= amt, >0 return, ≤0 → zero + "DAMPER EXHAUSTED" ids 0x2E..0x30 ONCE + return (no heat that pass); pool==0 → word@+0x30 += amt (i16 wrap) clamp 0xBB8, edge-triggered crossings 0x753 → "TEMPERATURE CRITICAL" ids 6/7/8 (@0x41025e/0x410280/0x4102ac), 0x9C4 → "HAS OVERHEATED" ids 3/4/5 (@0x4101d7/0x4101f9/0x41021d), old ≥ 0x9C4 → FUN_004102b6 EVERY pass; FUN_004102b6 = the AMMO COOK-OFF (sole caller 0x41019a): RandA&0x7F==0 (1/128), w = RandA&7 <7, drain = max(1, ammo@+0x38+8w >>3), ammo −= drain floor 1 (empty slot → 1 quirk), player-type → [0x46ccec]:=2, +0x32==0 → "LOSING AMMO" ids 0x31..0x33 + +0x32 := 100; +0x30 census: bleed −0xA/clamp0 + SP-death/MP-respawn resets + the sidebar HEAT gauge FUN_0040807f ×3 (scale 2500 = the overheat threshold); corpus UNREACHABLE by construction (scorch byte ≤7, fade 1/frame → crossing 0x753 needs ≥14 same-tile writes ≤94 frames under a parked robot; below 0x9C4 zero RNG) | §7j.55 |
| hot-rect click-target array | ONE array base 0x4787bc (record 0; the dispatcher's 1-based view 0x47879c = base−0x20), stride 0x20, 8 dwords {+0 world X, +4 world Y, +8 hit-box X origin, +0xC Y origin, +0x10 z, +0x14 w, +0x18 h, +0x1C type}, count [0x46ccd8] cap 0x77 (extent ..0x47969c), per-frame reset @0x403a9a; writers = 7 sites ALL in FUN_00403938: w1 0x403c87 robots MP-only ([0x4edb88]==2 ∧ ≠local player) type (idx+1)\|0x1000 w/h 0x40 z=rec+8+0x21 corner tile+0xB; w2-w7 0x4056f1/0x4058b8/0x405c4d/0x405f7b/0x406142/0x4062c6 critter .NME paths (state ∉{6,7,0xB}; w7 {6,7}) type idx+1, z ∈ {[crit+0x3E] raw/+0x20/+0x10/>>8}, w ∈ {0x3C,0x40} h 0x40 | §7j.31 |
| click picker | FUN_00419943 (only caller = dispatcher 0x41068e): scans hot rects i<[0x46ccd8], box = origin+(w/2,h/2) ± (w/2,h/2); priority = octile FUN_0041ebf8 max(\|dx\|,\|dy\|)+min/2, early-out <4; returns i+1; ground fallback = iso (mx−0xF0)·[0x4ede54]/0x1E0 + camera + TRT active-scan (x/y/z @+0x14/18/1C ×0x20, windows −0x10..+0x30) → 0x2000\|(idx+1) else 0 | §7j.31 |
| click order dispatcher | FUN_00410644 (MissionShell @0x448021; gates mouse≠−1/[0x4ede14]≠0/[0x4edba0]==0/mx<0x1E0): picked → type cell [0x46cc00] (NEW pin); bit13 TRT: rec(id−1) via −0xC-bias base 0x4cccec, coords ×0x20+0x10 → ORDER TARGET 0x4dd484/88/8c; bit12 robot: corner +0/+4 + z +0x10; else critter: corner + FUN_004128ec(id−1)>>8+0x15; ground: camera+view-mouse z0; tail [0x4ddb20]\|=2 order latch (NEW pin) + [0x4ede00]:=−1 consume | §7j.31 |
| terrain anim sequence | u32[16] @0x456ca8 = STATIC DGROUP const {0,1,2,3,4,5,6,7,7,6,5,4,3,2,1,0} — a 16-phase ping-pong over the 8 PALTRAN ramp slots (0x4dd444; slot 0 = NULL = plain blit); NO runtime producer (2 readers only: 0x40691a seen-level draw + 0x406a2c chase column, both `seq[frame&0xf]` → ramp → FUN_00401471/0040167a); the STATIC branch = the +0x18 scorch byte as the ramp index (scorch n → ramp n — the PALTRAN ramps double as scorch darkening); branch pick = the +0x1b/+0x1c anim window (nonzero in ZONEG only, §7j.32) | §7j.35 |
| water flag | [0x4edbd4] ≡ 1 in every mission: sole persistent writer = FUN_004252c0 @0x4252d8 (campaign-boot defaults, := 1, called 0x41c129 in FUN_0041c050); scoped save/restore bracket 0x41c649/0x41c65a around the SELECTOR screen FUN_0043e7d4 (GAMEGFX\SELECTOR.BIN/.PAL; esi = the mission-index reg = 1 on both paths); NO config/options/save/MP writer — "water off" render paths (remap-XLAT 0x4014d3/0x401566; the 0x12d/0x12e/0x12f plain-copy gates 0x4017a3/0x4020b5/0x40229d; chase pick 0x4069c7) are dead code in shipped play; water tables set-indexed 1..7: sprite family 0x454aac {0x15F,0x4B3,0x5B8,0x15F,0x141,0xFB,0x4B3} +0x1E, stamped-word base 0x454ae4 {0xBD,0x3BD,0x5E8,0xBD,0xEC,0xC3,0x3BD} +0xE; corpus: water sprites stage ONLY in ZONEB/M1(12)/M6(78), ZONEC/M4(33), ZONED/M1(1), ZONEF/M7(4824); ZONEA/M1 ZERO (but 44 0x7d2 hazard cells → the load stamper pre-stages the 0x460dfa grid in the gates) | §7j.35 |
| pad-tile probe | FUN_00422e5e(x Q5, y Q5, z word): tile = arg>>5 (sar), LEVEL = z>>5; RAW DAT-volume byte (FUN_0041eb4c, NOT the 0xFF→1 remap) ≠ 0xFF → −1; else FIRST 999×8-B .PAD slot @0x4e44f8 with active≠0 ∧ x/y/LEVEL match (dword>>16 reads of the +2/+4/+6 words); repeat-of-last-slot → latch 0x4eb9fc := −2 + counter 0x4eb9f4++ (still returns the slot); the LEVEL-vs-file-level fact: MRK word-3 L spawns z = L·0x20−1 = LEVEL L−1 | §7j.40 |
| MissionShell beacon block | 0x448291..0x448381: (sprite draw head, every-8th-frame gate); window decrement when nonzero; GATE dword@0x4e6610 ≠ 0 (dropship in flight) skips ALL; window == 0 → FUN_0041faf0; else ALL robots state-3-or-dead (w@+0xC / d@+0x7E scan, bound DAT_0046ccbc) → FUN_0041faf0 + window := 0; a beacon expiring mid-flight stays ARMED at window 0 | §7j.40 |
| dropship deployer | FUN_0041faf0 [unconditional, full body]: dropship@0x4e6610 := {active 1, phase 1, x = beacon_x·0x20, y = beacon_y·0x20, alt 0x200, group 0, dwell 0}; word@0x4eabb0 := 0 + word@0x4eabb2 := 0 (the flag/window pair ONLY — the tile words 0x4eabb4/6 SURVIVE; the claims 0x4eabba are never cleared anywhere); sole caller the beacon block above (2 sites 0x44832f/0x448375) | §7j.27, §7j.40 |
| extraction sweep | FUN_0041fbb1 machine-2 phase-1 landing tail: every robot alive ∧ state ∈ {3,4} → state := 5, timer@0x90 := 0x28 (outside the 31-leaf pin — E-gap), stop_dist@+0x74 := 10000000, [0x4dc680]++ (extracted), SFX 0x4edfe0 (presentation); phase-2 dwell 10 → phase 3; phase-3 alt > 0x200 → active := 0 ∧ [0x4dc67c] := 1 (complete); phase-2 jitter alt := (RandA()&7)==0 = a SHARED-STREAM draw | §7j.19, §7j.27, §7j.40 |
| SHOP screen | FUN_00440e45 (GameMain call #2, §7d.4 flow; 0x440e45..0x4437e5): entry money floor [0x46ae70] := max(·,100); loads GAMEGFX\{DARKPALS.PAL,WEAPICON.BIN,CONLITE.BIN,SHOPFONT.BIN,SHOPLITE.BIN} + BEEP1/4/7/5 SFX cells + "SOUND\MIDI\SHOP"; SMK "GAMEGFX\SHOP.SMK" intro gated [0x46cca4]≠0 (else GAMEGFX\SHOPPAL.PAL); the buy/sell/auto-loadout/confirm state machine over the weapon table 0x4de664 (group layout +0 name, +2 ammo, +4 artifact, +6 price, +8 category, +0xA item, +0xC owned) + chassis table 0x4deafc (2 rows × 0xE, same layout, +2 = shield charges for equipment ids 0x2A..0x2E); exit ret 0 (abort [0x4edb50] → ret 1) | §7j.45 |
| SHOP catalog | 9 category blocks @0x4ea288 stride 0xA0 (staged by FUN_0044395b from immediates): header +0x00 x0/+0x04 y0/+0x10 yoff/+0x14 count/+0x18 colw/+0x1C rowh; items stride 0x10 first at cat+0x20: name@+0x20 (FUN_00420260 idx), price@+0x24, pack-ammo@+0x28, avail@+0x2C; cat 0 = NEEDLERS (2/3/4, 100/250/400, 300/400/500), cat 1 = PLASMA (9/A/B, 500/700/900, ammo 1), cat 8 = CHASSIS @0x4ea788 (ids 0x2A..0x2E, 0x2D/0x2E mutex — FUN_00443870; free-slot/dedup finders FUN_00443870/FUN_004437ea) | §7j.45 |
| SHOP MP sync | exit: FUN_00449c94(4, 0x4e43e0) appends the type-4 SHOP-LOADOUT COMMAND record (63 B staging struct 0x4e43e0 = 7×9, consumed MissionShell 0x44853e + save 0x4475fd); then the player walk p < [0x46cbe0]: 7 (name,ammo) word pairs from record 0x4dd4a0+p·0x80 (+1 byte skip) → 0x4de664+p·0x62+g·0xE{,+2} — the per-player loadout mirror (D89's 0x46cbe0 count bounds it) | §7j.45 |
| SHOP availability array | 0x46cd48..0x46cd80 = 15 dwords; := 1 at shop entry when [0x4edb88]==2 (MP) ∨ [0x4edd8c]==7 (final zone); value 2 = transient (exit normalize: cols 0x46cd48/5c/70+i for i∈{0,4,8,0xC} == 2 → 1) | §7j.45 |
| SHOP category rank table | dword[0x456c7c + 4·cat] — the auto-loadout bubble-sort key over the 7 weapon groups (swap via 3× FUN_00402aaa through scratch 0x4dec4c) | §7j.45 |
| bombardment salvo cooldown | [0x4de658] (the dword 0xC below the weapon table base 0x4de664): := 0x80 by the robots() +0x70 threshold arm (the aerial-bombardment salvo — §7j.54 corrected the old "reinforcement pending gate"/"pending-arrival" gloss); gates the next arm while ≠ 0; FULL census §7j.54: arm write 0x40c27f, arm gate read 0x40c18b, read+dec 0x423e25..0x423e32 (FUN_00423e1c head, 1/frame), MissionShell clear 0x447877; the 0x442ba7 match is a weapon-table displacement alias ([eax+0x4de658], eax ≥ 0xC → ≥ 0x4de664), NOT a real access | §7j.45, §7j.54 |
| aerial-bombardment marker bank | 0x4ea238, 8 × 10-byte records (0x50 total, MissionShell memset 0x447a51): {u16 x@+0, u16 y@+2 (screen-pixel ground point), u16 fall-z@+4 (starts 0xFF, −0x20/frame), u16 start-delay@+6 (0x20+2i, −1/frame), u16 valid@+8}; writer = the robots() idle arm 0x40c25e..0x40c351 (8 shells: x = robot.px + RandA&0x7F − 0x3F, y = robot.py − 0x80 + i·0x20, tile-bounds-gated); tick/resolver = FUN_00423e1c (MissionShell @0x447ffa; NOT a "selection chaser" — §7j.54): fall until get_z_pos(x,y,+4) ≥ +4 → SIX kind-6 debris (3 RandA each) + NINE FUN_004244a1 5000-damage script blasts over the 3×3 tile patch (tx−1..tx+2, ty−1..ty+2, z_level+1) + cursor clear + valid clear; renderer 0x4066e4..0x4067a6 (FUN_00403938 draw tail): +8≠0 ∧ +6==0 → iso-project, +4 fall-z subtracted from the screen axis (sprite visibly descends 32px/frame), GENERAL.BIN sprite 0x12C via FUN_0040798e | §7j.54 |
| chase-camera override staging | FUN_004245c9(x,y,z) = a 5-instruction STAGER 0x4245c9..0x4245e5: {x,y,z} → 0x4de648/4c/50 + const 0xF → 0x4de654 (retires the "wall-strip redraw" gloss family — §7j.19/§7j.21/§7j.22 + the door row — and §7j item-6's "selection chaser" for its sibling); consumer FUN_00403938 0x4039b0..0x403a42: [0x4edbd8]≠0 ∧ [0x4de654]≠0 → the camera-point ring slot (0x4c71c4/cc/c8, 4-slot ring [0x46ccdc]) loads the staged triple instead of the selected robot's pos; [0x4de654]−− per frame; second consumer robots() 0x40b885 gates the camera-recenter block off while ≠ 0; MissionShell clears 0x4de654 (0x4478ad); FULL caller census (4, §7j.54): door stepper FUN_004223b8 @0x422427 + delayed-trigger expiry FUN_00422e0a @0x422e55 (§7j.12) + artillery spotter reveal @0x41173a (§7j.22) + the bombardment record-0 impact (SP ∧ record 0 ∧ cursor ≠ selected+1 ∧ cursor-robot is player-type, 0x423e7c..0x423ed5) | §7j.54 |
| ACTIONPAN registry gate | [0x4edbd8] = the "ACTIONPAN" value of HKCU\Software\Mirage\Bedlam\1.00 (REGISTRY, not a file — the "CONFIG.BDL" gloss RETIRED §7j.56: the string has zero binary refs, on-disk CONFIG.BDL/OPTIONS.BDL are DOS leftovers): 4-site census — readers = exactly the §7j.54 pair (0x4039b0 camera-slot swap; 0x40b875 recenter gate w/ the [0x4de654] leg 0x40b885); writers = the boot config family ONLY (loader FUN_004252c0 → FUN_0044ede4("ACTIONPAN",&cell, bounds [0,1], default 1) @0x42535c — RegQueryValueExA writes the cell directly, absent/malformed ⇒ default 1 = pans ON; saver FUN_0042540c @0x42545c via FUN_0044ed98/RegSetValueExA at the name-entry exit 0x43b03b + 0x41c59b); .bss; NO game-state/mission-phase/UI writer — session-constant enable bit for the whole chase-camera subsystem | §7j.56 |
| viewport zoom cell | [0x4ede54] = the vertical viewport height (ZOOM) in backbuffer rows, clamp [0xF0,0x1E0]=[240,480]: ±0x10/frame on keys (scan 0x4E/0x0D in, 0x4A/0x0C out — keystore 0x4edc92/0x4edc51/0x4edc8e/0x4edc50), the FUN_0042034c tail 0x4204ea..0x420548; per-mission init = the leftover-edx store 0x447883 (0x1E0 @0x44784a not provably surviving FUN_004034ef/FUN_0041d954 — benign: ≥480 dispatches 1:1, first keypress re-clamps); consumers: FUN_00401107 the zoom blitter (Q16 magnify scale (v<<16)/480 → 0x454060/68 + halves 0x45405c/64, source offset (480−v)/2; ≥480 → 1:1 rep-movs; [0x4edba0] map-overlay ≠0 → the map path; [0x4ede34]≠0 → temp v := 480−min([0x4ede34],479) w/ save/restore 0x4012c7/0x4012e5/0x4012f1), the recenter speed (cursor−240)·v/480 @0x40b89e/0x40b8c5, the cursor un-zoom mappers 0x4106a1/0x4106d4/0x419a41; NO corpus writer (no scenario presses zoom keys) → no differ rows | §7j.56 |
| death-wipe iris cell | [0x4ede34] = the CLOSING-IRIS death-wipe progress: 0 inactive; `:=1` at selected-robot SP death (FUN_0040e230 SP tail 0x40ea8b — MP never arms, posts the marker latch instead); `+=0x28`/frame by the MissionShell frame cluster (0x4480af, after the present call); terminal `:=0x1E0` @0x4480d6 when ≥480 + the AUTO-RESELECT pass (last ALIVE player-type squad slot ≠ selected → select it, flash [0x46ccec]:=3, cancel cell:=0 via xor-of-equals 0x448121 + [0x4ea8f8]:=0; no eligible mate → parks at 480 = the fail-detector conjunct 0x4476a2, §7j.57); cancels: 3 click-select strips 0x40d286/0x40d311/0x40d398 + per-mission zero 0x44787d; consumers: FUN_00401107 gate 0x401119 → temp v := 480−min(cell,479) render (fill-0 + centered v×v SHRINK of the FROZEN frame, §7j.58 C) + FUN_00403938 head 0x403952 skips the render body during the wipe; presentation-only → no differ rows | §7j.58 |
| MP death-position marker countdown | [0x4ea8f8] := 0x20 at MP selected-robot death (FUN_0040e230 MP branch 0x40e7ef, posting the dying position (rec+0/+4)>>8, rec+8 into [0x4ea8ec/f0/f4]); consumer = FUN_00403938 head 0x403974..0x4039a5: while ≠0 copies the trio into [0x46ccdc]·12 + 0x4c71cc/c4/c8 (the §7j.20 per-player selected-anchor ring — consumer = the §7j.54 chase-camera reader: the camera HOLDS the dead robot's position) + dec; zeroed in tandem with the iris cell at every cancel site + per-mission init 0x4478f1; SP never sets it (SP arms the iris instead); presentation-only → no differ rows | §7j.58 |
| robot shield-charge machine | d@robot+0x88 shield points (−2/frame clamp 0; 0x20 per charge/state-3; 0x2710 while +0xA0 flash) + d@+0x8C charges (spawn = word@chassis_row+2 via the 0x40cc8c 5-slot jump table, chassis ids 0x2A..0x2E; hit consume FUN_0040e230 @0x40e2a4) | §7j.45 |
| scorch-lane timers | w@+0x32 burn cooldown := 0x64 (FUN_004100b7 @0x4103e3, gate ==0 @0x41036e), dec 1/frame (phase-0 pre-pass); w@+0x30 accumulator −0xA/phase-1 off-scorch; alarm w@+0x34 cooldown + d@+0xA4 counter (dec 1/frame — D90's question closed) | §7j.45 |
| robot state-1 producer | THE ONLY writer of state 1 = FUN_00409138 COMMAND bit0 @0x40a37b (:= 1 + stop := 0xF4240) — no patrol semantics; SP never produces state 1 (full census §7j.45 Part B/4) | §7j.45 |

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
  ((mx−0xF0)·zoom/480; y = ((camera_z·480/zoom + my−240)·zoom)/480 + 21,
  signed truncating divisions in this order).
- picked == 0 → GROUND order: ORDER TARGET 0x4dd484/88 =
  (camera_x + (view_x >> 1) + view_y, camera_y − (view_x >> 1) + view_y),
  arithmetic shift; z 0x4dd48c = 0, type cell
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
     starts at plane 6/edx=6, bit7 CLEAR at plane 7/edx=7) and
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
   - FUN_00423650 (open DAT stamp): walk planes 7..1, default
     level 0; if edx != 0, clear DAT at that level. Always clear
     DAT plane 7 through [0x4eaae8].
   - Correction verified against ordinary z writers 0x4239ac/d5:
     the eight-plane table begins at 0x4eaacc (plane 0), with
     0x4eaad0 = plane 1 and 0x4eaae8 = plane 7. The previous
     ninth-plane inference incorrectly included adjacent state
     at 0x4eaac8. See RE-EXW-ELEVATORS.md for the bounded audit.
5. **FUN_004223b8 = the SCRIPTED DOOR stepper** (86 callers,
   ALL in the 0x433xxx-0x435xxx FUN_00433980 pad-script family;
   verified 0x4223b8..0x4225cf): args (rect idx, wanted ∈
   {1,2}); guard state ≠ wanted ∧ state < 3 (scripted doors
   only); FUN_004245c9(x0·0x20+w·0x10,
   y0·0x20+h·0x10) = the §7j.54 chase-camera cut (the old
   "wall-strip redraw" attribution of this call retired); per rect tile whose low7(+0x1A) == +0x19
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

## 7h.4. THE PICKUP TILE-WORD PRODUCER (7h.3) — CLOSED: the staging is init_tiles (ALL nonzero TOT words mirror-staged; the DAT byte gates only SEEN), the terrain set = zone+1 CONFIRMED, and the ZONEA corpus verdict is ZERO pickup cells (2026-08-22, worker f461ea05 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run; corpus probes read-only over game-data, scratch /tmp/opencode)

Closes the queue's standing 7h.3 item (the last open piece of the
§7h pickup consumer unit): where the pickup words COME FROM, how the
probe latch walks, and whether the machinery stages on the harness
corpus path. All items [verified] asm unless tagged.

1. **THE STAGING PRODUCER = init_tiles@00407e11, and it copies EVERY
   nonzero TOT word** [verified 0x407fb0..0x407ff8]: the load-time
   mirror build clears 0x4ab50 B at 0x4796bc (0x407ee4) then walks
   every tile × 8 z: `cx = TOT word; test cx,cx; je skip; word[edx +
   0x4796bc] = cx` — the mirror write is UNCONDITIONAL on the DAT
   byte; only the SEEN flag is DAT-gated: `if byte[DAT + z·pitch]
   == 0 → byte[+0x10+z mirror] = 1`. CORRECTION to the §2/§7j.16
   gloss ("copies every nonzero TOT word whose DAT byte is 0"):
   that DAT==0 condition is the SEEN gate (and the incremental
   FUN_00440a2d restamp path), NOT a word-staging gate — the pickup
   substrate rides the ordinary TOT volume into the mirror at load
   (seen=0 until the draw walks it).
2. **The probe latch walk — get_z_pos writes the trigger triple at
   FOUR sites, all gated on the probed DAT plane byte == 3**
   [verified 0x41e231..0x41e410]: `{z→0x4dc688, tile_x→0x4dc68c,
   tile_y→0x4dc690}` after (a) the level-z probe (0x41e282), (b) the
   z+1 empty-search probe (0x41e2c1), (c) the z−2 empty-search probe
   (0x41e302), (d) the slope-continuity z+1 probe when the CGR byte
   == 0x1F (0x41e399, z = esi+1 from [esp]). Last write wins within
   one call; the latch is a persistent dword with NO auto-clear —
   absolute census: the ONLY other 0x4dc688/8c/90 traffic is the
   consumer pair below (clear + test). Type 3 is NOT in the empty
   set {0, 0x2A} (§4), so pickup cells are solid — the robot probes
   them walking PAST, not through.
3. **The consume protocol — one call site, clear→move→test→fire**
   [verified 0x40bef2..0x40bff8; full 0x4dc688-family census]: ONLY
   the robots() move-toward-target block (state ∈ {1,4}, target ≠
   −1, non-arrive branch): `[0x4dc688] := −1` (0x40bef2) →
   robot_move (0x40bf06; its move_is_possible runs the 8 footprint
   probes (±11/±12 Q5) + the center settle — each calls get_z_pos,
   so a probe TOUCHING a type-3 cell sets the latch) → `cmp −1; je
   skip` (0x40bf0b) → the fire block: mirror word =
   `dword[0x4796ba + 30·tile + 2z] >> 16` of the latched cell;
   range test `A ≤ w < A+0x10` (signed) else `B ≤ w < B+0xC` per
   the set tables; on hit (a) `DAT[z][tile] := 0` (consumed from
   the collision plane — the cell becomes empty), (b) mirror word
   := floor word `word[0x454a90 + 4·set]`, (c) seen :=
   `byte[0x4796cc + 30·tile + z] = 1`, (d) {x,y,z} staged at
   0x4dc6ac/b0/b4 — read ONLY by the MP tails
   (`[0x4edb88]==2` → FUN_00425647) of case 8 (0x40eda7) and case 1
   (0x40ef57), staging the pickup tile for the network session —
   then `FUN_0040eba0(word, robot_idx)` (the §7h dispatch). The
   OTHER robot_move call site (0x40dc0e, the wander/drift family)
   has NO clear/test — latches set there linger until the next
   move-toward-target clear. So a robot collects a pickup when any
   of the 9 probes of ONE move sub-tick touches the cell — no
   standing-on required (±11/12 Q5 ≈ ±0.34..0.38 tile reach).
4. **The terrain set [0x4edd8c] = zone_index+1 — CONFIRMED and
   sharpened** [verified]: the mission path builder writes the zone
   letter as `'A' + set − 1` (`mov al,[0x4edd8c]; add al,0x40` at
   0x446771/0x446879/0x4468d2) — ZONEA ⇔ set 1, ZONEB ⇔ set 2, …
   ZONEG ⇔ set 7. Writer census (absolute, all sites): GameMain
   boot := 1 (0x41c41c..430, with [0x4edd88] := 1 and the campaign
   episode counter [esp+0x31c] seeded); the campaign episode
   advance `set++; episode++; loop while episode < 7`
   (0x41c9d6..0x41c9e5 → 0x41c454 — the 7-episode SP campaign
   walks sets 1..7 = zones A..G in order); the save-load restore :=
   `movsx word` from the 0xB4-stride save record +4 (0x43c2b3..b8,
   behind FUN_0044745e); the network-session episode advance :=
   word from the stack pair alongside FUN_00449c94(2) (0x43f341..b);
   and the MP mission picker (0x43edcb/0x43ede8/0x43ee04/
   0x43ee18/0x43ee3d) mapping MP list rows 1..10 → sets 2..6 —
   MP-ONLY (gated `[0x4edb88]==2` at 0x43edb9; the SP branch
   0x43ee48 only DERIVES [0x4edd88] = mission-within-set, never
   writes the set). The §7h "set = zone+1, boot-consistent"
   hypothesis is CONFIRMED with zone A 0-based.
5. **THE CORPUS VERDICT — ZONEA/M1 stages ZERO pickup cells; the
   harness corpus path does NOT fire** [verified, read-only probes
   /tmp/opencode/pickup-probe{,2}.py over the shipped TOT/DAT]: a
   pickup cell = DAT plane byte 3 ∧ TOT word in the set range.
   ZONEA/M1 (SP fresh boot → set 1, ranges [0x4E,0x5D) ∪
   [0x75,0x80)): the map has 80 DAT==3 cells, but their words
   (0x81..0x84 ×37, 0x131, 0x230..0x237 ×44, 0x28D, 0x53D) fall in
   NO set-1 range — the 0x81..0x84/0x53D words are set-2/5 ranges
   only (case-4 score/money and case-8 ammo shapes: A+12..15 /
   B+8), INERT under set 1. Campaign-set census across the corpus:
   ZONEB (set 2) stages 601 pickup cells (M1 152 / M2 107 / M3 93 /
   M4 106 / M5 95 / M6 30 / M7 18 — cases 1/2/3/4 all present,
   case-4 dominant); ZONEF (set 6) 149 cells (cases 1/2/3/4);
   zones C/D/E/G stage NONE under their campaign sets. So S0/S1/S2
   (all ZONEA/M1 fresh-SP boots) never fire the pickup machinery,
   and the probe-latch/consume machinery is invisible on the
   current harness path.

Engine seam (superseded by §7h.5 — the producer LANDED in the
engine 2026-08-22; this paragraph's (a)-(d) list is the spec it
was built against). The watch surface for a pickup scenario: the
mirror row word + seen + the DAT plane byte at the consumed cell,
plus the case-4 score/money pair (the D52 seam fields).

## 7h.5. THE E-SIDE PICKUP PRODUCER PAIRING — the engine mapping + the range-table INDEXING derivation (2026-08-22, worker f32193a2 claim 2, W12-S5-prep; the §7h.4 decode implemented engine-side, docs-first hop)

**Superseded 2026-09-06:** the subtract-one indexing deduction below
is contradicted by EXW 0x40bf3c/0x40bf48, 0x40bfb8 and 0x40ebaa.
The original Boot Camp gold pickup was collected live for +10 cash.
See RE-EXW-PICKUP-INDEX.md for exact raw-set addressing and corrected
census; the claimed zero-pickup corpus verdict below is not authoritative.


The engine-modeling notes for the W12-S5-prep unit. Nothing here
re-decodes the binary — it pins WHICH §7h.4 site maps to which
engine seam and settles the one open gloss the implementation
needed. [verified] = §7h.2/§7h.4 asm; [derived] = this hop.

1. **The range/floor tables are indexed by zone_index (0-based),
   NOT the raw cell — [derived, structural].** The DGROUP family
   0x454a04(rubble)/0x454a20(hazard-7d2)/0x454a3c(7d3)/
   0x454a58(A)/0x454a74(B)/0x454a90(C)/0x454aac(water-sprite) is
   ONE contiguous run of 7-dword tables at exact 0x1C strides —
   there is NO unused head slot inside A/B/C (a `base + cell*4`
   read with cell ∈ 1..7 would run off the end of table A into
   table B's first dword, which is nonsense for a per-set
   selector). The consistent reading is `base + (cell−1)*4` =
   zone_index 0-based, which is exactly the form the landed
   `pickup_case(word, set)` + its corpus-validated tests already
   use (7h.2): ZONEA(idx 0) → A=0x4E, ZONEB(idx 1) → A=0x75, …
   — and the §7h.4/5 corpus probe CONFIRMS it behaviorally (the
   0x81..0x84 words are case-4 only under idx 1 = ZONEB, INERT
   under ZONEA's idx-0 ranges; under the raw-cell reading ZONEA
   itself would fire them, contradicting the shipped-map
   verdict). The floor-word table C rides the same form:
   `[0x70b, 0x48f, 0x24c, 0x368, 0x48f, 0x39, 0x39]` indexed by
   zone_index. CAVEAT recorded: the destroy-family tables in
   `destroy.rs` (RUBBLE_WORD/HAZARD_7D2/7D3/WATER_RANGE) were
   landed as 8-entry arrays indexed by the RAW cell with an
   "unused head" — under the same structural argument their
   heads should be dropped and the values shifted (e.g. 7j.35's
   zone-A hazard base 0x49 vs destroy.rs's cell-1 entry 0x20).
   That is a pre-existing, corpus-dead question (the hazard
   stamp runs only in synthetic destroy_gate tests; no canonical
   chain covers it) — left untouched by this unit, flagged for
   the S5/S7 differ rows to arbitrate.
2. **The wiring map** [derived; every EXW site from §7h.4/2-3]:
   - init_tiles@00407e11 → `MissionSim::stage_pickup_surface` (a
     NEW host seam beside the destroy unit's
     `stage_terrain_mirror`, which stages words only): parses the
     mission `.TOT` volume (`u16 w + u16 h + 8 × w·h u16`
     plane-major, FORMATS §2), copies EVERY plane word into
     `mirror_words` (the pre-cleared mirror makes the
     nonzero-filter equivalent to a plain copy), stages
     `mirror_seen[tile·8+z] := 1` exactly when the swept+PAD DAT
     volume byte `dat[z·w·h + tile] == 0`, and writes the
     terrain-set cell `zone := zone_index+1` (D99). The heights
     pair (+0x1B/+0x1C) is NOT staged — its producer is the
     zone-7 objective family (§7j.32), corpus-dead elsewhere.
   - the FOUR get_z_pos type-3 latch sites → ALREADY MODELED:
     `Terrain::floor_z` writes `last_trigger` at exactly the four
     §7h.4/2 sites (level probe / z+1 empty-search / z−2
     empty-search / the 0x1F slope z+1), last-write-wins, no
     auto-clear.
   - the consumer 0x40bef2 clear → `robots_phase`'s
     move-toward-target else-branch: `last_trigger := None` (the
     −1 sentinel) immediately BEFORE `robot_move`, then the
     0x40bf0b `≠ −1` test immediately AFTER → `fire_pickup`.
     UNCONDITIONAL (the EXW clear runs in every move sub-tick,
     armed or not — with no staged mirror words the range test
     reads word 0 and never fires, so S0..S4 stay byte-identical;
     the only observable delta on old paths is the latch clear
     itself, which no canonical row covers).
   - the fire block 0x40bf18..0x40bff8 → `fire_pickup(idx, z, tx,
     ty)`: word = `mirror_words[(ty·w+tx)·8+z]`; `pickup_case`
     over the staged set; on a case — (a) `dat_write(tx,ty,z,0)`
     (the collision-plane consume; the cell becomes EMPTY —
     walkable-through afterward), (b) `mirror_words[·] :=
     PICKUP_FLOOR_WORD[zone_index]` (table C), (c)
     `mirror_seen[·] := 1`, (d) the MP-only 0x4dc6ac/b0/b4
     staging is SP-only-unreachable (gated [0x4edb88]==2) —
     unwired by design; then the §7h.2 dispatch.
   - the dispatch → `apply_pickup` WIDENED: cases 1/2/3/7 write
     the robot fields as landed (7h.2); case 4 [7f.6] draws
     `row = RandA()&1` then `amount = [1000,2000,5000,10000] /
     [10,50,100,250][RandA()&3]` on the sim stand-in stream and
     stages a pending (score, money) award the MissionShell folds
     beside the destroy-score fold (the [0x4dd40c]/[0x46ae70]
     cells are shell session state); cases 8 (ammo, effect 0xC)
     and 9 (episode, effect 0xD) return their effect ids with NO
     field writes — host-seamed (the robot weapons[7] bank is the
     D51 host seam, W12-S3; no shipped mission stages case-8/9
     cells: the 0x53D word is a set-5 shape INERT on ZONEA, and
     the ZONEB/ZONEF censuses show cases 1..4 only).
3. **The corpus-dead invariant, engine-side** [derived from
   §7h.4/5]: with the surface fully staged on ZONEA/MISSION1
   (set 1), every DAT==3 cell's staged word decodes to `None`
   under idx 0 — a walk over them latches and clears but never
   fires. The corpus gate asserts exactly this (zero fire
   traffic: mirror/seen/DAT unchanged, zero award) across the
   S2-style order walk, making the staging a provable no-op on
   the S0..S4 paths (the pinned chains 8901789a88cf61fe /
   1c4e7b4c9d9b0947 / 809f4961b7757da4 / 49193732e6dbc546 /
   2ddd15ea50c8a14d re-assert byte-identical).

## 7j.35. THE MISSIONVIEW §8 WATER-FLAG/ANIM REMAINDER — u32[0x456ca8] = a STATIC ping-pong const (producer = the file image); [0x4edbd4] ≡ 1 for every mission (no gameplay writer exists); ZONEA/M1 stages ZERO water (2026-08-22, worker 57ba8753 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run; DGROUP bytes + corpus TOTs re-read read-only, scratch /tmp/opencode)

Closes MISSIONVIEW §8 item 2 (the last open §8 row): the 16-entry
anim sequence producer and the water-flag producer. All [verified]
asm/DGROUP/corpus unless tagged.

1. **u32[0x456ca8] IS STATIC DGROUP DATA — there is no runtime
producer.** Full-.text census (objdump 0x401000..0x460000, every
addressing form): the table has exactly TWO sites, both READERS
in the terrain loop of FUN_00403938 — 0x40691a (the seen-level
draw) and 0x406a2c (the seen-chase column) — and ZERO writers
(no `mov [x*4+0x456ca8]`, no immediate-EDI bulk setup). The file
image carries the values (PE DGROUP VA 0x454000 = file 0x52600,
read at 0x552a8): `u32[16] = {0,1,2,3,4,5,6,7, 7,6,5,4,3,2,1,0}`
— a 16-phase PING-PONG (triangle wave) over the 8 PALTRAN ramp
slots. Reader form (both sites): `frame = u32[0x456ca8 +
(g_frame_count@0x46ae68 & 0xf)*4]` → `remap = u32[0x4dd444 +
frame*4]` → the blit (FUN_00401471, or FUN_0040167a on the water
path). With ramp slot 0 NULLed after load (§7e) the cycle renders
as plain, r1, r2, … r7, r7, r6, … r1, plain — an 8-on-8-off
shimmer; the same 0x4dd444 ramps are the ones the STATIC branch
indexes by the +0x18 SCORCH byte (0x406923: `edx :=
byte[0x4796d4+30·tile]` → 0x406935 `ebx := u32[0x4dd444+edx*4]`),
i.e. **the PALTRAN ramps double as the scorch-darkening tables**
— scorch n draws ramp n, scorch 0 = plain. The branch pick is
the +0x1b/+0x1c anim WINDOW (`z < +0x1b ∨ z ≥ +0x1c → static`):
outside the window the scorch byte rules, inside it the ping-pong
rules. Producers of nonzero windows are the ZONE-7 objective
family only (§7j.32) — no non-ZONEG mission ever animates a
tile, and the +0x18 scorch is the transient 7j.8/9/10 ring
(fades ≤7 frames, already modeled in bedlam-core).
2. **The water flag [0x4edbd4] ≡ 1 during EVERY mission —
"water off" is unreachable in shipped gameplay.** Writer census
(complete, 3 instructions in .text):
   - **0x4252d8, FUN_004252c0 (the campaign-boot defaults
     initializer, called at 0x41c129 inside FUN_0041c050 on
     every "New Single Player Game")**: `0x4edbd4 := 1` (beside
     `0x4edbf0 := 1`, `0x4edbe0 := 1`, `0x4edbe8 := 2`,
     `0x4ddb2c := 0x4B`). This is the ONLY persistent write —
     no CONFIG.BDL restore, no OPTIONS-menu toggle, no save-load
     write, no MP path touches it (0x4edbe0 is a DIFFERENT,
     options-writable cell — cleared by the stub FUN_0043a1c8
     @0x43a1cb and gated in the renderer at 0x4033ec/0x403503;
     0x4edbe8's reader cluster is the MissionShell family;
     neither feeds 0x4edbd4).
   - **0x41c649 + 0x41c65a, FUN_0041c050 (campaign loop)**: a
     scoped save/restore bracket — `0x46ae80 := flag; flag :=
     esi; call FUN_0043e7d4; flag := 0x46ae80`. FUN_0043e7d4 is
     the robot SELECTOR screen (loads GAMEGFX\SELECTOR.BIN/.PAL,
     0x4592d8/0x4592ed); esi is the loop's mission-index register
     (= 1 on both observed paths: init 0x41c09e, reset
     0x41c4ad), so even transiently the flag stays 1 — the
     bracket is a conservative guard, and it is OUTSIDE the
     MissionShell lifetime regardless.
   Readers (complete): the remap-XLAT gate in FUN_00401471
   (0x4014d3/0x401566), the `flag==0 → plain copy` gates of the
   0x12d/0x12e/0x12f mode dispatches in FUN_0040179b (0x4017a3)
   and the direct-codec family (0x4020b5/0x40229d, mode cell
   [0x4edd5c]), and the terrain chase-column water pick
   (0x4069c7). **Consequence for the §8 item's stated goal: the
   0x12d/0x12e/0x12f flush remaps may permanently assume
   water-ON semantics — the `flag == 0` branches are dead code
   in every shipped session (missions, MP, save-load).**
3. **Water-range table CORRECTION (7j.12 item 6 off-by-one):**
every consumer indexes the zone tables by the RAW set value
[0x4edd8c] = 1..7 (`shl edx,2` of the cell, verbatim at
0x422f77..f89 / 0x422fa3..fb5 / 0x4226f0..701 / 0x422a40..a4b /
0x422bbd..be2 / 0x4069d8..9e4 / 0x42411b..126), so table entry 0
is UNUSED (it is the previous array's tail — the
0x454a20..0x454ae4 family is ONE contiguous u32 array chopped at
0x1C strides). Effective SET-INDEXED (1..7 = A..G) bases,
DGROUP-read:
   - 0x7d2 hazard words (extent +4): 0x454a20[1..7] =
     {0x49, 0x49, 0x34E, 0x49, 0x77, 0x77, 0x49};
   - 0x7d3 clamp words (+4): 0x454a3c[1..7] =
     {0x4E, 0x4E, 0x349, 0x4E, 0x7C, 0x7C, 0x4E};
   - water SPRITE family (+0x1E, the renderer/splash test):
     0x454aac[1..7] = {0x15F, 0x4B3, 0x5B8, 0x15F, 0x141, 0xFB,
     0x4B3};
   - water stamped-WORD base (+0xE, the platform/splash word
     test): 0x454ae4[1..7] = {0xBD, 0x3BD, 0x5E8, 0xBD, 0xEC,
     0xC3, 0x3BD}.
   The 7j.12 prose lists were entries 0..6 of the raw array
   (shifted one zone); the instruction bases 0x454a20/0x454a3c
   stand.
4. **THE CORPUS VERDICT — ZONEA/M1 stages ZERO water; the
harness corpus path does NOT fire** [read-only probe
/tmp/opencode/watercorpus.py, all 37 mission TOTs, per-zone
set-indexed ranges]: water SPRITE-family words stage in exactly
five missions — ZONEB/M1 (12 cells), ZONEB/M6 (78), ZONEC/M4
(33), ZONED/M1 (1), ZONEF/M7 (4824 — a water-heavy mission);
ZONEA/M1: **0 water cells in both the sprite range
[0x15F,0x17D) and the stamped-word range [0xBD,0xCB)**. The
platform/splash WORD family (0x454ae4 base) appears in ZERO
shipped files — it is runtime-only (platforms/splashes never
stage in the gates: weapons never fire, platforms unstaged,
§7j.12 corpus verdict). Side finding: the 0x7d2 HAZARD words DO
stage widely (the load-time stamper FUN_00422f18 runs every
mission): ZONEA/M1 carries 44 cells in [0x49,0x4D) → 44
hazard-grid words @0x460dfa at boot (the 7g.5 robots() hazard
path is LIVE in the gates; 0x7d3: 0 cells — zone A's
[0x4E,0x52) range is empty). High scores elsewhere: ZONEC/M4
481, ZONED/M4 394, ZONEB/M6 288+29, ZONEC/M3 286, ZONEB/M7 166,
ZONEG/M1 170, ZONEF/M3 193; ZONEE stages no 0x7d2 (54+16 0x7d3
cells in M6/M7).
5. Engine seam (this unit, D98/D99 pattern): the corpus path
does NOT fire for ZONEA/M1 — NO engine code this unit. The
bedlam-render `DrawParams.remap` stays the host seam (the
frame-index/scorch/ping-pong selection is pixel-side, out of the
0b state-diff budget; scorch STATE is already hash-covered via
the 7j.10 fade). P4.2 hooks (D100): a water-leg scenario must
run ZONEB/M1, ZONEB/M6, ZONEC/M4 or ZONEF/M7 (F/M7 = the
4824-cell water mission); E needs, before such a scenario,
(a) the per-tile remap selection (static scorch-index ramp /
anim-window ping-pong — the 16-word const above), (b) the water
sprite-range branch in the chase path (FUN_0040167a/TXPAL1),
(c) the water flag pinned ON (it is a constant in shipped play
— no E-side toggle exists to model); the watch surface is the
terrain mirror rows (water words already covered by the mirror
row) — nothing new to watch. The 0x12d/0x12e/0x12f flush
semantics may hard-code water-ON.

## 7j.36. THE [0x4ede1c] BIN-BANK CONTENT CONSUMERS — container grammar pinned (u16 count + SELF-relative u32 directory at +2); every content reader is a pixel blit; the 9-sprite radar stamp is the only in-place writer and its output is NEVER drawn (vestigial); the bank is render-only presentation (2026-08-22, worker d6b238f4 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run; DGROUP bytes + corpus banks/TOTs/LNKs re-read read-only, scratch /tmp/opencode)

Closes the 7j.16 residue (NEXT item: the [0x4ede1c] bank's CONTENT
readers, the sprite record grammar, and the §0b state-vs-presentation
question). All [verified] asm/DGROUP/corpus unless tagged.

1. **Complete [0x4ede1c] traffic census (12 absolute .text sites —
   nothing else references the cell):**
   - LOADERS (writers of the pointer): 0x41d670 (FUN_0041dc5a arena
     install), the `.BIN` leg 0x41dcc6/0x41dd22 (tag 0x4587e8 via
     staging 0x4dca8c; `u16[bank+0] → 0x46cdb8` @0x41dd37), and the
     FUN_0044661b EDITOR\ZONE restore reload 0x446649/0x4466fa with
     the same store @0x446702. **0x46cdb8 is WRITE-ONLY in .text**
     (2 writer sites, zero readers): the sprite count is stored but
     no code ever bounds-checks an id against it — the blits mask
     `id & 0xFFF` instead (0x401477/0x40167a/0x4017e6 family).
   - CONTENT readers, exactly three clusters:
     a. **FUN_00403938 terrain loop** — 0x40692e/0x4069f5/0x406a40/
        0x406b15 load ESI for the FUN_00401471/FUN_0040167a blits
        (MISSIONVIEW §3; 0x4069f5 is the 4th site, added to the §1
        census of three).
     b. **the BRIEF-minimap drawer** (UPDATE §7j.49 2026-08-23:
        the sites 0x440d1c/0x440d93 belong to FUN_00440c34,
        called ONLY by FUN_00440dc2 = the BRIEF objective-minimap
        snapshotter; the earlier "scroll/camera RESTAMP DRAWER
        FUN_00440dc2" gloss is corrected) —
        `FUN_00401471(EAX = u16[type-DB word @
        0x4796bc+…], EBX = 0 remap, EDI = dest)` with the dest
        bounds-checked into the backbuffer window
        ([0x4ede18] .. +0x5a000) — the draw side of the
        stager FUN_00440a2d (renders a type-DB word back
        into the BRIEF screen's OWN backbuffer; pixel path
        only, never the mission pass).
     c. **FUN_00401010 = the 9-sprite RADAR STAMP** (item 3) — the
        ONLY writer into bank content at runtime.
2. **Container grammar — instruction-exact AND corpus-verified on
   all 11 shipped MISSION*.BIN banks** (7 zone-level + B6/D5/E6):
   - `u16[bank+0]` = sprite COUNT (A/D 1450, B/G 1872, C 1743,
     E 1455, F 989, B6/D5 1443, E6 1120) — this is the word the
     loader stores to 0x46cdb8.
   - **Directory: entry = bank + 2 + 4·id; sprite record =
     entry + u32[entry] (SELF-relative).** Verified monotone with
     every entry in-file in all 11 banks; the last record runs to
     EOF (tails 10..1835 B are the final record's own extent).
   - Record grammar (normal sprites): u16 fmt@+0, dy@+2, dx@+4,
     gate@+6, rows@+8, then the stream (dest = EDI + dy·0x280 + dx).
     FUN_00401471 dispatch (0x401487..0x4014c8): fmt ≥ 4 → u8-RLE;
     fmt 1..3 → u16-RLE; fmt 0 → raw; **gate == 0 → RETURN (draws
     nothing); rows == 0 → RETURN.** FUN_0040167a parses the same
     head but **reads gate and IGNORES it** (0x4016ad, no test) —
     rows 0 still draws nothing; FUN_0040179b takes the +2 view
     (fmt skipped), skips gate, rows@+8, always-u16-RLE decode.
   - **MISSIONVIEW §4 CORRECTED:** "sprite = bank + u32[bank +
     4 + id*4]" is wrong in both base and anchor. The correct form
     (asm 0x401477..0x401485, the same `4·id+2` idiom at
     0x40108b/0x40167f/0x4017e6 + 13 more sites of the generic
     bank-draw family) is the §5c form: entry = bank + 4·id + 2,
     sprite = entry + u32[entry]. GAMEGFX .BIN banks share it
     (FORMATS §18 cross-ref now VERIFIED, not assumed).
   - fmt census [corpus]: every zone bank carries fmt 7 (u8-RLE)
     for ALL real terrain sprites + EXACTLY 9 fmt-0 records (the
     scratch family, item 3). gate==0 ≡ rows==0 id sets: A 83,
     B 47, F 40 (the 9 + assorted unused slots — A: {212, 641,
     662, 859–872, 1134–1142, 1168–1176, 1221–1246, 1357–1378};
     B: {575, 704–706, 1362–1372, 1435–1442, 1503–1512, 1772–1781,
     1801, 1869–1871}; F: {58–66, 448, 458, 758, 961–988}) —
     inert-by-construction sprite slots (draw nothing via 01471;
     most also 0179b-dead via rows 0).
3. **The 9-sprite SCRATCH family + the stamp (the bank's only
   runtime mutation — and it is VESTIGIAL):**
   - Set table: `[0x4edd94] := u32[0x454b00 + 4·set]` @0x4479b4
     (mission-boot reset; set = RAW [0x4edd8c] 1..7) — bases
     {0x490, 0x6ED, 0x638, 0x490, 0x3A, 0x3A, 0x6ED} = sprite ids
     {1168, 1773, 1592, 1168, 58, 58, 1773}; +9 always ≤ count.
     [0x4edd94] has exactly 4 refs total: the boot write + 3 reads
     inside the stamp itself.
   - Family record layout [corpus]: a 6-byte stub {fmt=0, dy=64,
     dx=64} + a 4096-B image — span-to-next exactly 0x1006, i.e.
     the data starts at +6 (NOT the 10-byte head; in the §4 view
     gate/rows = image bytes [0..3] = ZERO as shipped) — so the
     family is UNDRAWABLE by every blit, and stays so: the stamp
     never writes image[0..0x1F]. Images are ~empty (24..39
     nonzero bytes of 4096, none in the first 0x20).
   - **FUN_00401010** (entry 0x401010, helper 0x40108b): runs at
     the head of the PRESENT function FUN_00401107 (call 0x401107;
     callers 0x447ca0/0x448099 = MissionShell) on EVERY present.
     It samples the backbuffer at the camera ([0x4ede18] + 0xa040
     + the §5d iso sub-tile cam offset), source grid +5 px in x
     and +0xc80 (5 rows) in y — a 5× downsample of the 480×480
     viewport — and writes 1 byte per row step at dest row stride
     0x40 into THREE consecutive sprites per call at image+0x20
     (page step 0x806 lands exactly at image(K+i)+0x20 for
     i = 0,1,2 — records 0x1006 apart; verified arithmetically),
     the outer column loop stepping +1 with an extra +0x40 every
     2nd iteration = dest (row ≈ j/2, col 32+j) — a 2:1 SHEAR, the
     iso→top-down deshear. Three calls at source +0xa0 steps
     (ids +0/+3/+6, 3 sprites each) tile the whole window into
     the 9 sprites: a 3×3 radar-style capture, dest footprint
     cols 32..63 rows 0..46 of each image.
   - **THE OUTPUT IS NEVER DRAWN [complete census]:** no [0x4edd94]
     reader besides the stamp; 0x454b00 has no other reader; the
     terrain loop could only reach the family via TOT words → LNK,
     and **LNK is IDENTITY on all 63 family ids in all 7 zones**
     (self-map, no cycle — static); gate/rows stay 0. Zones A/B/C/D
     DO reference all 9 family ids from TOT plane words (E/F/G
     reference none) — those tiles stage/mirror normally but the
     blit returns at gate==0: **they render NOTHING**. The stamp +
     family is a shipped-inert radar/in-world-monitor capture
     (the stamp still runs every present — wasted writes, zero
     observable effect).
4. **§0b STATE-vs-PRESENTATION verdict — the bank is RENDER-ONLY
   presentation; NO differ watch row:**
   - every content reader is a pixel blit into the backbuffer
     (2a/2b); the bank never feeds simulation state (get_z_pos
     reads .CGR heights, never BIN; the type-DB words index INTO
     the bank, never the reverse);
   - the only in-place writer (the stamp) reads the backbuffer and
     writes never-drawn scratch — no state coupling;
   - 0x46cdb8 is write-only (below the emptiness-rule threshold —
     no observable divergence can originate there).
   DIFFER consequence: no watch row for the bank, its directory, or
   0x46cdb8; the state surface stays the TOT words / type-DB mirror
   rows (already covered). E-side: the terrain blit seam is exactly
   the 7j.35 list (u8-RLE decode + per-tile remap selection) — the
   scratch family and the stamp need NOT be modeled at all.

## 7j.37. THE S3-PREP RE ADDENDUM — the COMMAND dispatch + the
weapon-anim tick re-verified field-exact from the existing local
dumps (consumer FUN_00409138 ghidra-project/exw-robottarget.txt,
tick FUN_00410823 ghidra-project/exw-weaponanim.txt/-asm.txt, angle
family ghidra-project/exw-text-objdump.txt 0x41eb65..0x41ebf7); the
SINTABLE.BIN cos/sin identity CLOSED against the corpus file
(2026-08-22, worker 95ab9206 claim 2; objdump/dump-only, NO new
Ghidra run; one read-only corpus probe of SINTABLE.BIN, scratch
/tmp/opencode — prep for the E-side W12-S3 weapon-fire producer)

Purpose: the E-side fire model needs the spawn fields and per-tick
mechanics at transcription fidelity. Everything below is
[verified] against the named dumps unless tagged. This refines
7j.17 item 4 and 7j.22 items 3/4/5/8 — no ledger row is
contradicted; three rows are rewritten/added.

1. **THE CONSUMER DISPATCH, field-exact** (FUN_00409138,
   exw-robottarget.txt:5..650):
   - Ring: records @0x4dd4a0 stride 0x80, count DAT_0046cbe0; the
     loop processes record i then i+1 — and the RECHARGE PASS runs
     ONCE at the do-while exit (also when count==0): for EVERY
     robot × 7 slots, `(enable-bit set) ∧ cooldown≠0 → cooldown−−`
     (slot cooldown = u16@(rec+0x36+8k+6), i.e. 0x4c6a20+8k). The
     count is NOT reset by the consumer [the reset writer stays
     unpinned; the W5 injector appends at count].
   - Per record: spot short@+3 → robot +0x14 (frame-base word) AND
     the cursor angle `_DAT_004dc678`; robot idx = record_index·
     DAT_0046cbd8 + id short@+1 (SP: the raw id); MP-only
     (0x4edb88≠0): robot enable mask +0x6E := word@+7.
   - **flags bit0 SELECT**: if state ∉ {3,4,5} → move-target words
     DAT_0046cc30/60[robot] := words@+7/+9 (raw Q5 words); if
     alive ∧ state ∉ {2,3,4,5} → state@+0x0C := 1, stop_dist@+0x74
     := 1000000. QUIRK [decompile-faithful]: the bit0 block
     advances the record word pointer +2 shorts, so a record with
     bit0∧bit1 set writes the ORDER triple from words@+0xB/+0xD/
     +0xF (bit1 alone reads +7/+9/+0xB).
   - **flags bit1 ORDER** (the fire arm): 0x4dc6bc := 1; the
     order-target triple 0x4dd484/88/8c := the words above;
     clears 0x4eb940..0x4eb950 (7 dwords, the one-shot SFX latch
     family); then per robot (alive ∧ deploy-delay +0xA0 == 0 ∧
     state ≠ 2): player-kind → sidebar redraw 2; THE 7-SLOT
     WEAPON LOOP:
     * **FIRE GATES** [verified]: `(mask@+0x6E >> k) & 1) ∧
       cooldown@slot == 0 ∧ ammo@slot ≠ 0` — the slot fires. A
       slot passing the gates sets the pass-fired flag even for
       out-of-switch ids (id−2 > 0x26).
     * **w 9/0xA/0xB ARTILLERY (inline)**: 1 record, type :=
       the slot id; x := pos_x+0x100, y := pos_y+0x100,
       z := (robot.z+0x15)<<8; tick@+0xA := 0, draw-ctr@+0xE := 0,
       owner := robot; NO velocity/class/arc (falls); ammo−1;
       **mask bit XORed UNCONDITIONALLY** (one-shot per arm);
       cooldown := 0; SFX 0x4edf94.
     * **w 0x10/11/12 (prox mines) and 0x14/15/16 (pressure
       mines)**: 2/4/6 records (both families — the 0x14-row count
       was elided in 7j.17, the loop bound is 2/4/6 here), types
       0xF / 0x13; per record (gates re-check ammo>0 PER RECORD;
       free slot −1 or vx==vy==0 → skip the spawn but the ammo
       bookkeeping of THAT record still ran): jitter = TWO RandA
       draws (`RandA()&0x3F − 0x20 + order_target_x/y`); octile
       normalization `dist8 = octile((tgt−muzzle)<<8) >> 3` (0→1,
       min 1); `vx = ((tgt_x − pos_x>>8)·0x10000)/dist8` (vy
       likewise); vx := vx>>2 (0xF) or >>1 (0x13), vy likewise;
       spawn {x := pos_x, y := pos_y, z := (z+0x15)<<8, vz := 0,
       ttl := RandA&0xF + 1, arc := 0x900 − RandA&0x2FF, class :=
       4} (the 0x13 family also class 4 — the 7j.17 "class 0/4"
       split applies 0 to the GRENADES, not the mines); ammo−1,
       0 → mask bit clear; cooldown := 8; SFX 0x4edfe4 once per
       pass (latch 0x4eb950). **4 RandA draws per spawned record**
       (2 jitter + 1 ttl + 1 arc).
     * **w 0x1B/0x1C (bouncy) and 0x1D/0x1E (sticky)**: 4/6
       records, types 0x1A / 0x1F; **3D velocity**: `vz := 0` if
       order z == 0 else `(order_z − (muzzle_z>>8))·0x10000/
       dist8`; ttl := 0x32 − RandA&0xF (0x1A: 35..50) / 0x32 +
       RandA&0xF (0x1F: 50..65) [corrects the 7j.17 "0x33−" and
       "..0x42" glosses]; arc := 0xB00 − RandA&0x2FF (0x1A) /
       0x900 − RandA&0x2FF (0x1F); class := 0; trail word +0x32 :=
       0; the same 4-draw jitter+ttl+arc shape; cooldown 8.
     * **w 0x20 ROCKET (inline)**: 1 record, type 0x24, NO jitter
       (aim at the RAW order target); `vx = ((tgt_x − pos>>8)·
       0x10000)/oct8` (vy likewise, vz 3D like the grenades);
       ttl := 0, class := 0; arc@+0x2E := the angle pair result
       (FUN_0041eb7d(ratio of |dx|·0x80/(octile>>8)) +
       FUN_0041ebc1 quadrant fold — E's AngleTable::angle_byte);
       cooldown := 5; SFX 0x4edfac; ammo−1, 0 → mask clear.
     * **w 2/3/4, 6/7/8, 0x18/0x19, 0x21..0x23, 0x25..0x28** →
       the AI-ORDER FAMILIES with count args 3/2/1, 0/1/2, 1/2,
       3/6/9, 1/2/4/6: FUN_0040b615 / FUN_0040af98 / FUN_0040a56f
       / FUN_0040ace8 / FUN_0040a7a1. Their internals stay OPEN
       (the bullet/plasma/frag/rocket-pack/reaper spawn bodies);
       the inline bookkeeping (ammo/cooldown/mask) does NOT run
       for them here — whatever they do is inside the family.
     * **AUTO-REARM** [verified]: after the slot loop, if the
       robot's mask == 0 ∧ something passed the gates this pass:
       first slot with id≠0 ∧ ammo≠0 → mask bit set + the
       weapon-switch messages FUN_004239ef(0x1C/0x1D/0x1E,
       playerIdx) (per-player guards 0x46cbd4/0x46cbd8); if the
       mask is STILL 0 → the all-empty messages (0x1F/0x20/0x21).
   - Idle path [refines 7j.17]: the AI idle ticks
     (FUN_0040af98(10)/0xace8(9)/0xa56f(2)/0xa7a1(6)) run when
     alive ∧ **deploy-delay +0xA0 ≠ 0** ∧ frame&3 == 0 — not on
     the 0x4dc6bc gate.
2. **SINTABLE.BIN IS THE BYTE-ANGLE SINE TABLE** [verified:
   corpus 512-B file + asm 0x41eb65/0x41eb77]: word[a] =
   round(sin(a·π/128)·32767) over a = 0..255 (w[0]=0, w[0x40]=
   32767, w[0x80]=−1, w[0xC0]=−32767). FUN_0041eb65 (COS) returns
   `movsx word[base + (angle&0xFF)·2]`; FUN_0041eb77 (SIN) =
   the same lookup at `(angle−0x40)&0xFF`; the 64-word threshold
   table of the sector scan (FUN_0041eb7d, base+4) = words[2..66]
   of the SAME array — the table is dual-use. (The "cos/sin"
   names are the callers' intent; the values are the sine ramp at
   two phase offsets.) E stages the full 256-word array; the
   lookups are pure reads.
3. **BULLETS (types 2..4) — the exact sub-step loop** [verified
   decompile]: per call, up to 2 sub-steps are TESTED; the loop
   structure is move-first/test-after, and the THIRD iteration also
   moves (then sets the done flag) — so a no-hit call performs 3
   moves − 1 rollback = **TWO net committed steps** and tick += 6
   (this CORRECTS 7j.22 item 3's "2 cells tested, 1 committed" —
   the cells-tested count is right, the net move is 2). Each
   sub-step: `x+=vx, tick+=2, y+=vy, z+=vz`, then (sub-step ≤ 2)
   test bounds/`z>>13>7`/tick>99 → free; the CRITTER lane →
   actor-hit; the MP robot lane → actor-hit; floor test
   `get_z_pos(x>>8,y>>8,z>>8) > z>>8` → terrain-hit. After the loop
   the position is ROLLED BACK one step (`pos −= v`);
   actor-hit re-adds the step and runs the disburser (K2);
   terrain-hit re-adds, applies the impact pair
   (FUN_00419aff(type) via FUN_0041a894 + FUN_0041bc1c) and the
   disburser — **and the record does NOT free on impact**: bullets
   expire ONLY at tick>99 [refines 7j.22 item 3's "expire → type :=
   0" — that free is the ttl path alone].
4. **SHELL (type 5)** [verified]: one move per call; bounds/tick>
   100/z OOB → free; on ODD phases the critter lane runs — a HIT
   stores (x,y) and emits the K3 debris at z>>8−10
   (FUN_00420608(...,3,0,owner)) [refines 7j.22 item 4: the trail
   is the LANE-HIT body, not a per-tick emit]; the MP robot lane
   hit → disburser only; the floor hit → impact pair (75) +
   disburser + FREE; else pos/tick update (tick += 1).
5. **ARTILLERY (9..0xB)** [verified]: phase-0 call only; tick++;
   `floor = FUN_0041e411(x>>8,y>>8,z>>8); floor < z>>8 → z −=
   0x200 else z = floor<<8`; tick==0x18 ∧ owner-kind == player →
   FUN_004245c9 = the §7j.54 chase-camera cut (the old
   "wall-strip redraw (presentation)" attribution retired); the burst
   window is `tick−0x20 < dword[0x456c78 + 4·TYPE]` — **the
   duration table is indexed BY TYPE** (durations 9→2, 0xA→4,
   0xB→7; entries 0..8 unused), NOT id−9; inside the window the
   pair list `PTR[0x456bf0 + 4·(tick−0x20)]` (sentinel 500) fires
   FUN_004244a1 per pair with the 50% K0xB debris; past the
   window → the disburser tail + free (the 7j.22 ">0x22" gloss =
   the w9 arithmetic; wA/wB free at tick 0x24/0x27 by the same
   comparison).
6. **HOMING (0x29) steering, exact** [verified decompile]:
   z-eases toward the target z ±0x200/tick (clamp [0,0xFF00]);
   ground-lift `FUN_0041e411(x>>8,y>>8,z>>8) ≥ (z>>8)−4 → z +=
   0x200` (clamp again); `dx/dy = target_Q13 − (pos&~0xFF)`; the
   angle pair over `|dx|·0x80/(octile>>8)`; heading@+0x2E :=
   `(heading + FUN_00412a19(angle, heading)·4) & 0xFF` (the
   signed byte-diff helper, turn clamp = the diff itself ×4);
   velocity step `2·(word[heading]>>4, word[heading−0x40]>>4)`;
   forward floor probe FUN_0041e56d (a 4e411 sibling) at the new
   x/y — blocked (floor ≥ z>>8) → the AVOIDANCE LOOP over offsets
   0,4,8..0x3C: candidate = (heading−off)&0xFF FIRST (left), then
   (heading+off)&0xFF; a clear candidate becomes the heading; the
   LEFT leg that goes out-of-bounds ALSO climbs z += 0x600 (the
   right leg does not — asymmetric [faithful]). Then tick++ and
   the bounds/ttl/dead-target/floor-impact chain of 7j.22 item 8.
7. Engine consequence (the S3-prep seam): everything above is
   transcription-ready. E-gaps that stay open on the E side: the
   five AI-order family internals (w2..8/0x18/0x19/0x21..0x28
   spawn bodies), the mortar family FUN_0040a9ff internals
   (w0xE), the artillery burst-pair APPLICATION (FUN_004244a1
   needs the terrain-structure bank — S4), the disburser/debris
   tails (off-path), the SFX/message families (T4), the 0x22-bank
   spawn producers (enemy fire — the critter family), and the
   0x69-vs-table question — CLOSED 2026-08-23 §7j.50.

## 7j.38. THE S4-PREP RE ADDENDUM — the destroy-family RNG-draw
census + the chain-walk geometry + the four missing DGROUP tables
(2026-08-22, worker 1b45efab claim 2; objdump/dump-only from the
COMMITTED ghidra-project/exw-destroytail-asm.txt + one read-only
DGROUP probe of BEDLAM.EXW over the PE section table, scratch
/tmp/opencode/s4prep-dgroup-probe.py; prep for the E-side W12-S4
impact-application + destroy-resolver producer unit)

Purpose: the E-side destroy model must consume RandA in the
original's exact order/count (rand_a is hashed — a draw-count
error is a chain divergence the moment a destroy happens in an
S4+ scenario). Everything below is [verified] against the named
dump unless tagged. No prior row is contradicted; this section
makes 7j.25's effect table DRAW-EXACT and pins the tables the
resolver/structure-death need.

1. **THE FIVE-EFFECT LOOP, DRAW-EXACT** (case bodies in
   exw-destroytail-asm.txt; "R" = one FUN_00402975 RandA draw,
   "B" = one FUN_004029b6 RandB draw — RandB feeds only the
   T4 SFX bank pick, unhashed/unmodeled):
   - sel 1 (0x41ac77): k14 debris (0x41ace7) → the 12-slot
     effects-bank stager FUN_0041a225 (0x41ad2d — RandB draws
     ONLY, unmodeled T3) → ONE plain splash at the entry center
     (probe 0x41ad75 + FUN_00424355 0x41ad84, NO jitter draws) →
     a 4-iteration loop (0x41adce..0x41adc8): per iteration 2R
     (x−=R&1 at 0x41ade7, y−=R&1 at 0x41ae18) + probe (0x41ae3b,
     → z or 7) + splash (0x41ada2). **Total 8 RandA.**
   - sel 2 (0x41b298): k18 debris (0x41b302) → 4-iteration
     splash loop (2R + probe + splash each; draw sites
     0x41b337/0x41b368). **Total 8 RandA, NO plain splash.**
   - sel 3 (0x41b3c1): k17 (0x41b42b); draws 0x41b460/0x41b491.
     **8 RandA.**
   - sel 4 (0x41b4ea): k16 (0x41b554); draws 0x41b589/0x41b5ba.
     **8 RandA.**
   - sel 5 (0x41b613): k19 (0x41b67a); draws 0x41b6b2/0x41b6e3.
     **8 RandA.**
   - sel 6 (0x41b1fe) / 7 (0x41b11c): k10 debris (0x41b26e /
     0x41b185) + ONE RandB draw (0x4029b6 at 0x41b273/0x41b18a)
     &1 → the DEADMAN1/DEADMAN2 bank pick (T4, unmodeled).
     **0 RandA.**
   - sel 8 (0x41af96): k14 at the entry center (0x41b002) →
     24-iteration shower (0x41b063..0x41b111, i < 0x18): per
     iteration 3R (x = base + R&7 − 3 @0x41b063, y = base + R&7 −
     3 @0x41b07c, z = base + R&3 @0x41b086) + the water-z probe
     (0x41b0ac) + k14 debris at (x,y,probe_z) (0x41b0dc) +
     splash (0x41b0eb) + FUN_0041a225 (0x41b0fa); the delay =
     counter + (i>>3). **Total 72 RandA.**
   - sel 9 (0x41ae53): k20 debris (0x41aebf) → ONE plain probe
     at (x−1, y−1) (0x41af0b, no draws, z clamped ≤ 7) → a 3×3
     double loop (outer [c, c+3), inner [r, r+3) — the ring
     x−1..x+1 × y−1..y+1): per cell 1R (&3 added to the DELAY
     arg, 0x41af6b) + splash (0x41af8b). **Total 9 RandA.**
   The loop runs m = 0..4 over the 5 entries; selector 0 or >9
   skips the entry with NO draws.
2. **THE FOUR PERIMETER CHAIN WALKS** (0x41b771..0x41bc06;
    entry x/y = the destroyed instance's footprint origin, W/H =
    its type row's extent words). [GEOMETRY CORRECTED
    2026-08-22 §7j.39/5 — this item's walk-2/3 labels were
    garbled; the raw-asm walks are: 1 the N row (y−1, x+j,
    j ∈ [−1, W]), 2 the S row (y+H, x+j, j ∈ [−1, W], the
    recursion passing (y+W)<<13 — a faithful quirk), 3 the W
    edge (x−1, y+j, j ∈ [0, H)), 4 the E edge (x+W, y+j,
    j ∈ [0, H)); the per-candidate protocol below stands]:
   - Walk 1 — the N row: y' = y−1 fixed; the x Q13 accumulator
     starts at the x−1 tile and steps +0x2000; j runs −1..W
     (bound W+1, 0x41b7dd..0x41b7f0).
   - Walk 2 — the W edge: x' = x−1 fixed (accumulator x·0x2000 −
     0x2000, 0x41b8bb..0x41b8cc); the row index walks j from −1
     while j < W+1 (0x41b8f7 reads the W word@row+0 — the bound
     is W for a VERTICAL walk, a faithful original quirk
     [verified bytes; flagged hypothesis on intent]).
   - Walk 3 — the S row: y' = y+H fixed ([esp+0x98] rows, H read
     at 0x41ba34); x walks j from −1, bound W+1 (0x41ba2d).
   - Walk 4 — the E edge: x' = x+W (0x41bb46..0x41bb6c adds W);
     the row walks j from 0, bound H (0x41bb31/0x41bb40 reads
     H, exit jle).
   - Per candidate tile (all walks): skip unless 0 < x' < w ∧
     0 < y' < h (STRICT — `test/jle` + map-word compares); the
     grid word −1 (signed) > 0; instance[word−1] id dword <
     0x4000 (alive); type-row(id).chain word ≠ 0; then ONE RandA
     draw (`&3 == 0 → counter++` at 0x41b871/0x41b99b/0x41bbbd/
     0x41bbcc — the roll ALWAYS draws, the counter bump is the
     1-in-4) and the recursive
     FUN_0041a894(x'_q13, y'_q13, counter, 1000, forwarded
     score flag). The recursion's own destroy tail re-walks
     (depth-first, shared counter).
3. **THE RUBBLE WORD TABLE 0x454a04** [DGROUP bytes, PE-mapped]:
   {0xFFFFFFFF, 0x20, 0x20, 0x348, 0x20, 0x20, 0x20} — zone-
   indexed 1..7 (index 0 is the unused cell); zone 3 (ZONEC)
   restores word 0x348, every other zone word 0x20. This is the
   FUN_0041bc1c death stamp source (7j.32/8).
4. **THE WATER TABLES** [DGROUP bytes, recorded for the S4
   splash-tick pairing]: base 0x454aac {0x24C, 0x15F, 0x4B3,
   0x5B8, 0x15F, 0x141, 0xFB} (zone 1..7 — index 0 unused), range
   base 0x454ae4 {0x25D, 0xBD, 0x3BD, 0x5E8, 0xBD, 0xEC, 0xC3}
   (range = [base, base+0xE) per 7j.12).
5. **THE ARTILLERY BURST PAIR LISTS** (0x456bf0 ptrs {0x45687c,
   0x4568a2, 0x4568d4, 0x456936, 0x456998, 0x456a1a, 0x456adc};
   each list = (Δy,Δx) i16 pairs until the first-short 500
   sentinel): expanding SQUARE rings — list 0 = the full 3×3
   block INCLUDING the center (9), 1 = radius-2 ring (12), 2 =
   radius-3 ring (24), 3 = radius-4 (24), 4 = radius-5 (32), 5 =
   radius-6 (48), 6 = radius-7 ring with a 2-pair TAIL DUPLICATE
   ((−6,−5),(−6,−4) repeat — the original fires those two tiles
   TWICE [faithful]); 68 pairs. Durations table 0x456c78 re-read:
   {1,7,2,6,4,3,1,8,5, 2, 4, 7} — idx 9/0xA/0xB = 2/4/7 ✓ (the
   7j.37 landing re-confirmed).
6. **Scope notes for the E model** (what S4-prep models vs what
   its scenarios pair later): the SPLASH TICK body (the 7j.10
   odd-frame fall/absorb + the per-tick 5-draw scorch re-roll +
   the water stamps at ages 1/40/47) stays UNMODELED until a
   scenario exercises water (S4's T3/T4 coverage names it — never
   silence); the stager itself (gates + 250×0xA bank + eviction)
   IS modeled — the five-effect loop's splash calls are recorded
   rows, and the stage gates consume NO RNG. The platform SPREAD
   ring FUN_00422832 + the CREEP tick = the S7 seam (they need
   the zone water range + the robot-presence checks); the
   resolver's platform ENTRY (FUN_00422693 destroy/weaken
   arithmetic) IS modeled over host-staged strength/grid words.
   The trigger producers FUN_00422e0a/FUN_00422600 (bridge
   builds) are S7-routed no-ops. The trap lane's intra-walk order
   vs the armor pass (FUN_0040fe93 @0x40bc44 sits between the
   pad-byte read 0x40bbab and the charge call 0x40bc60) is
   modeled armor-first with the exact interleaving unpinned
   [hypothesis — matters only when a trap shares a pad tile,
   corpus-never].

## 7j.39. THE S4-PREP ENGINE-LANDING ADDENDUM — FUN_004244a1
internals + the impact-pair call orders + the chain-walk geometry
correction + the debris allocator (2026-08-22, worker 460d294e
claim 2; objdump-only from the COMMITTED ghidra-project/
exw-text-objdump.txt + exw-weaponanim-asm.txt, no Ghidra run;
prep for the E-side W12-S4 landing — every fact below was read
off the raw asm this run)

1. **FUN_004244a1 = the SCRIPT BLAST, whole body**
   [verified 0x4244a1..0x4245c4]: args (x_tile, y_tile, z_level).
   Bounds x/y vs map_w/h → silent exit. Then IN ORDER: (a)
   FUN_00424355 splash stage, delay arg ECX = 0; (b)
   FUN_0041bc1c(x<<13, y<<13, 5000) — the STRUCTURE resolver
   FIRST; (c) `push 1` + FUN_0041a894(x<<13, y<<13, ctr 0,
   5000, flag 1) — the OBJECT resolver second; (d) k6 debris
   gate: ONE RandA, `test al,7` → stages ONLY when (al&7)==0
   (1-in-8, NOT 50% — corrects the 7j.13 census gloss), delay =
   a SECOND RandA draw &7, kind 6, param −1, coords (x<<5,
   y<<5, z<<5); (e) z' = clamp(z−1, min 1); (f) ALL-CRITTER
   area damage: FUN_004190bc(i, owner −1, x<<5, y<<5, z'<<5,
   weapon 0xC, mode 2) for i < [0x46cc2c]; (g) ALL-ROBOT area
   damage: FUN_00418fca(i, x<<5, y<<5, z'<<5, weapon 0xD, mode
   2) for i < [0x46ccbc] (the TOTAL count — D89) — the §7j.23
   box test |dx|,|dy| < 0x20 (Q5) + mode-2 |dz| < 0x30 (robot
   z raw) → FUN_0040e230(FUN_00419aff(weapon), owner −1).
2. **The weapon-tick IMPACT-PAIR call orders** [verified
   exw-weaponanim-asm.txt]: bullets 2..4 = FUN_0041a894
   (0x410ae0) then FUN_0041bc1c (0x410af9), both
   FUN_00419aff(kind), `push 1`; shell 5 floor = pair
   (0x410c8d/0x410ca2) + disburser + FREE; ROCKET 0x24 floor =
   OBJECT (0x4118ad) → STRUCTURE (0x4118c2) → disburser
   (0x4114eb) → free; HOMING 0x29 floor = STRUCTURE (0x411f24)
   → OBJECT (0x411f3f) → disburser (0x411f4a) → free — the 0x29
   order is REVERSED vs 0x24 [faithful]. The rocket z<0 path
   (0x411787: z := 0x1000, kind := 0) ALSO disburses.
3. **The class-0 EXPIRY QUADRANT body** [verified
   0x410d5c..0x410eb2]: at tick > 0x64, kinds 0xF/0x13 first
   tick := 0, class−−; class ≠ 0 → continue; class == 0 →
   FUN_004124a4 disburser FIRST (0x410dc4), then 4×
   FUN_0041a894 at (x±0x1000, y±0x1000) quadrants in the order
   (+,+), (+,−), (−,+), (−,−) (0x410de1..0x410e42, damage
   FUN_00419aff(0x1a) `push 1` — the 0x1a damage even when the
   dying record is 0xF/0x13!), then 4× FUN_0041bc1c over the
   same quadrants (0x410e59..0x410e9e), then the trail-ring
   slot clear, then free.
4. **The mortar 0xE floor-contact 3-cell detonation**
   [verified 0x411298..0x411338]: after the bounce arm (with
   the arm's vx/vy HALVING already applied — 0xE runs the
   shared halving at 0x41153), kind 0xE fires FUN_004244a1 ×3:
   (x>>13, y>>13, z>>13), ((x−vx·4)>>13, (y−vy·4)>>13, z>>13),
   ((x−vx·4)>>13, (y+vy·4)>>13, z>>13) — x/y = the committed
   post-wall position ([ESP+0x78]/[ESP+0x80] = the saved record
   x/y), vx/vy the POST-halving velocities.
5. **THE CHAIN-WALK GEOMETRY CORRECTED** (§7j.38/2's walk-2/3
   labels were garbled; this is the raw-asm order
   0x41b771..0x41bc06): the tail runs FOUR walks over the
   destroyed record's footprint origin (X, Y) + the type W/H —
   WALK 1 the N row: y' = Y−1 fixed, x' = X+j, j ∈ [−1, W]
   (bound W+1); WALK 2 the S row: y' = Y+H (candidate check),
   x' = X+j, j ∈ [−1, W]; WALK 3 the W edge: x' = X−1 fixed,
   y' = Y+j, j ∈ [0, H); WALK 4 the E edge: x' = X+W, y' = Y+j,
   j ∈ [0, H). Per candidate (all walks): STRICT bounds
   0 < x' < w ∧ 0 < y' < h (jle/jge skips); the grid word −1
   signed > 0; instance[word−1] id dword < 0x4000 (alive);
   type-row(id).chain word ≠ 0; ONE RandA (the roll ALWAYS
   draws; &3 == 0 → counter++); then the recursive
   FUN_0041a894(x'_q13, y'_q13, counter, 1000, forwarded
   flag). QUIRKS [faithful]: walk 2's RECURSION passes
   EDX = (Y + **W**)<<13 (0x41b9b5..0x41b9cd reads the +0
   dword) while its candidate gate checked Y+H — identical on
   every corpus type (W == H throughout §7j.25/8), divergent
   only for hypothetical non-square footprints; walk 1's
   yline read is the 0x4ea8fc-base idiom = line[Y−1].
6. **The debris stager FUN_00420608 head + allocator**
   [verified 0x420608..0x4206b0]: bounds x < 0 ∨ y < 0 ∨
   (x>>5) ≥ w ∨ (y>>5) ≥ h → NO staging; z clamped [0x20,
   0xFF]; ALLOCATION = first slot with dword@+0 == 0 (0..127),
   else the MIN +0x18 seq slot (LRU evict — the +0x18 seq is
   the age key); kind 1..20 else exit (before the dispatch).
7. **The artillery burst body, arg-exact** [verified
   0x4115eb..0x4116e1]: the pair list PTR[0x456bf0 +
   4·(tick−0x20)] walks (Δy, Δx) i16 pairs until the 500
   sentinel; per pair FUN_004244a1(x_tile + Δx, y_tile + Δy,
   z clamped ((z>>8)+0x3F)>>5 cap 7), then ONE RandA `test
   al,1` → on odd, k11 debris at ((x_tile<<5)+0xF,
   (y_tile<<5)+0xF, (z_level<<5)+0x10), kind 11, delay 0,
   param = the record's owner dword@+2. Past the window:
   tick ≤ 0x22 → silent exit (w9's exact-0x22 end), else the
   FUN_004124a4 disburser tail.
8. **OPEN (deliberately NOT changed this unit)**: the 0x1F
   floor-contact ARM — the raw dispatch sends 0x1F to the
   damped-roll arm 0x41133a (with 0x13), while §7j.22/§7j.37
   and the landed engine place it in the vertical-bounce arm;
   fixing it moves the pinned S3 chain and needs its own
   re-pin unit. Also open: the 0xF/0x13 mine per-tick
   proximity detonation checks at the FUN_00410823 tail
   (0x411457..0x4114f0/0x4114f5.. — the DAT-volume row tests +
   FUN_00417cde thresholds 0x40/0x20) and the 0x17 tail
   tick−− (0x41148, net-0 per pass) — both recorded for the
   mine/0x17 audit unit.
9. **Engine consequence**: the E-side landing of all the above
   is chain-neutral for S0/S1/S2 (no weapons fire) but NOT for
   S3 — the artillery burst pairs draw (the k6 1/8 gate per
   FUN_004244a1 + the k11 50% gate + the stager's k11 SFX-gate
   draw) with or without destructibles staged, so the S3
   canonical chain re-pins ONCE at this landing (the D103
   note: no O1 S3 capture exists yet — the dbx-plan T2-tier
   unit precedes any live S3).

## 7j.40. THE S6 EXTRACTION TRIGGER CHAIN — the MissionShell beacon
block + FUN_0041faf0 + the FUN_00422e5e pad probe, instruction-exact
(2026-08-22, worker 8d32d85d claim 2, W12-S6 prep; objdump-only from
ghidra-project/exw-text-objdump.txt + a read-only corpus probe of
ZONEA/MISSION1.PAD — no Ghidra run, no corpus write)

Purpose: the E-side S6 extraction producer needs the beacon-expiry
deploy semantics, the pad-tile probe's exact match keys, and the
dropship spawner's field map at transcription fidelity. Everything
below is [verified] against the objdump unless tagged.

1. **FUN_00422e5e = the PAD-TILE PROBE, full body** [verified
   0x422e5e..0x422f08]: args (EAX x Q5, EDX y Q5, EBX z word);
   `tile_x = x>>5`, `tile_y = y>>5`, `LEVEL = z>>5` (all sar 5);
   reads the RAW DAT-volume byte via FUN_0041eb4c — `and eax,0xff;
   cmp eax,0xff` — i.e. the RAW plane byte (NOT dat_type's 0xFF→1
   remap); ≠ 0xFF → return −1. Then the slot scan: 999 records,
   8-B stride @0x4e44f8, FIRST match wins — a record matches iff
   its active u16@+0 ≠ 0 ∧ dword@+0>>16 == tile_x (the +2 x word)
   ∧ dword@+2>>16 == tile_y (the +4 y word) ∧ dword@+4>>16 ==
   LEVEL (the +6 z word). On the FIRST matching slot edx: if
   edx ≠ [0x4eb9fc] (the revisit latch) → return edx; else
   (repeat of the last-returned slot) [0x4eb9fc] := −2,
   [0x4eb9f4]++ (the counter), still return edx.
   **KEY Z-LEVEL FACT**: the .PAD record's z word (the file's
   level) must equal the robot's `z>>5` — a marker-staged robot
   at MRK word-3 level L spawns at z = L·0x20−1, i.e. LEVEL
   L−1 (the −1 puts the center in the level BELOW the marker
   word until the floor settles). A GROUND pad (file level 0)
   therefore matches robots standing on the ground deck
   (z 0..0x1F), and a level-1 pad (e.g. ZONEA/M1 slot 8
   (2,14,1)) only matches a robot whose floor reached 0x20+.
   [Corpus probe, ZONEA/MISSION1.PAD: 999 6-B records, 114
   live (slot 0 = (5,61,0) … slot 113 = (18,24,4)), terminator
   0xFFFF at slot 114. CORRECTED 2026-09-06: zone-1 SP mission-1
   extraction is ONLY slot 0x10 at (17,25,4); slots 0x12 and
   0x18 are tutorial pads, slot 8 is a door action. The old union
   census caused a live movement lock; see RE-EXW-MISSION-ROOM.md
   extraction dispatch correction.]
   The E model may skip the revisit latch: after a trigger the
   beacon arms (the armer's one-at-a-time gate) and the robot
   halts state 3, so a repeat probe of the same slot is inert
   on the S6 path [derived].
2. **The dispatcher call gate** [verified 0x40bd43..0x40bd58]: in
   FUN_0040b9f6's per-robot walk the pad dispatcher FUN_00433980
   runs BEFORE the state-{1,4} move math, gated ONLY by the
   move-target word `cmp [0x46cc30+…],0xffffffff` (a robot with
   a target); the walk itself then re-checks state ∈ {1,4}
   (0x40bd6a..0x40bd72) — so the ARMER's state-3 halt takes
   effect the same sub-tick, before that robot's move. E models
   the dual gate as `state ∈ {1,4} ∧ target.is_some()`, probe
   before the order-consumption/move of the same iteration
   (EXW runs the dispatcher after the armor pass, before the
   walk — E inserts it between the phase-1 armor block and the
   order-consumption block).
3. **The MissionShell beacon block** [verified 0x448291..0x448381,
   instruction-exact]:
   ```
   ; 0x448291..0x448301: the beacon sprite draw (presentation,
   ;   gated test byte@0x4eabb2,0x7 — every 8th frame)
   si = word@0x4eabb2
   if si != 0: word@0x4eabb2 = si−1          ; the window tick
   if dword@0x4e6610 (dropship active) != 0: SKIP ALL  ; 0x448323
   if word@0x4eabb2 == 0:
       FUN_0041faf0()                        ; deploy — 0x44832f
   else:
       count robots with state==3 (w@+0xC) OR alive==0 (d@+0x7E)
       if count == DAT_0046ccbc:             ; ALL halted-or-dead
           FUN_0041faf0()                    ; deploy — 0x448375
           word@0x4eabb2 = 0                 ; (xor edx,ecx = 0)
   ```
   So EVERY beacon expiry (window 0 OR all-state-3/dead) deploys
   the dropship, gated only on no dropship already in flight; a
   beacon expiring while the craft is active stays ARMED at
   window 0 (the block is skipped) and deploys the frame after
   the craft goes inactive [derived from the gate order]. The
   decrement precedes the gate, exactly as E's tail block already
   models.
4. **FUN_0041faf0 = the DROPSHIP DEPLOYER, full body** [verified
   0x41faf0..0x41fb4a]: unconditional —
   `dropship := {active 1, phase 1, x = beacon_tile_x·0x20,
   y = beacon_tile_y·0x20, alt 0x200, group 0, dwell 0}`;
   `word@0x4eabb0 := 0` (beacon flag), `word@0x4eabb2 := 0`
   (window). **The beacon TILE words 0x4eabb4/6 SURVIVE the
   deploy** (only the flag/window pair is cleared) and the
   spread-claim array 0x4eabba is NEVER cleared anywhere
   (§7j.20/3) — post-deploy O1 beacon-family rows read
   {0, 0, tile_x, tile_y, tile_z} + the surviving claims.
5. **THE PRODUCER-TAG SEAM DECISION** [derived, design]: EXW arms
   0x4eabb0 through EXACTLY ONE caller — the pad-script armer
   FUN_004247b5 (sole caller FUN_00433980, §7j.20/1); a plain
   click never arms the beacon (the click path writes the
   COMMAND-ring move-target words, §7j.37/1 bit0). E's `order`
   scenario step (arm_at_robot at the clicked tile) is therefore
   the DOCUMENTED click-seam approximation (DESIGN §6a), and its
   expiry behavior stays the pre-S6 clear (the S0..S5C chains are
   byte-pinned). The S6 pad path is the faithful producer: the
   pad-armed beacon's expiry deploys + persists the tile/claim
   words exactly as items 3/4 pin. The asymmetry is deliberate
   and disappears when a live session re-anchors the click seam.
6. **The animator phase map (machine 2)** [§7j.27/3 re-read,
   confirmed]: phase 1 `{group := (group+1)&1; alt := alt−0x20 if
   alt ≥ 0x101 else (alt>>2)·3; alt < 1 → alt := 0, phase := 2,
   dwell := 10, EXTRACTION SWEEP}`; the sweep [§7j.19/1] = every
   robot alive ∧ state ∈ {3,4} → state := 5, timer@+0x90 := 0x28
   (the +0x90 word is OUTSIDE the 31-leaf canonical pin — an
   E-gap row-wise, robot state/stop_dist carry the observation),
   stop_dist (+0x74) := 10000000, [0x4dc680]++ (extracted
   counter), SFX (presentation). Phase 2 `{alt := (RandA()&7)==0
   (0/1 jitter — a SHARED-STREAM draw), group ^= 1, dwell−− == 0
   → phase 3}`; phase 3 `{alt += (alt>>2)+1; x −= group·4;
   group := group<5 ? group+1 : 4; alt > 0x200 → active := 0 ∧
   [0x4dc67c] := 1 (extraction complete)}`. Timing from deploy:
   land ≈ 25 frames (8×−0x20 then ×0.75 shrink), dwell 10,
   depart ≈ 24 frames from alt 0.
7. **The extraction-arm cells stay the objective family's**
   [§7j.32/5 re-read]: 0x46cd00/0x46ccfc/0x46ccc4 are written by
   the objective resolver (all-6-done / zone-7 at-zero) — NOT by
   the beacon/dropship chain. On ZONEA/M1 the script-objective
   staging (tables 0x4557f8/0x456810) is head-decoded only, so
   E's cells read 0 and any live O1 divergence there is the
   recorded script-objective E-gap, never a fabricated row. The
   destroy-notify zone-7 at-zero tail (the three cells + SFX)
   lands engine-side with this unit (the destroy.rs "S6-seam
   E-gap" note closes); [0x4eba0c]/[0x4eba10] (rescue
   progress/timer) stay unmodeled — the POI-rescue family is
   out of S6's path entirely.
8. **Corpus-path verdict**: S6 is the first scenario whose
   canonical rows carry the real producer chain — beacon (T1,
   existing rows + the surviving-words latch), dropship-frame
   (T3, E-only row), the swept robot states (T1 robot-bank).
   S0..S5C stay byte-identical (the pad path is the only deploy
   route; nothing pads in them).

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
   Projectile type 0x69 vs the FUN_00419aff damage table —
   CLOSED 2026-08-23 §7j.50. The FUN_00410823 weapon-anim
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
2. ~~FUN_00440e45 (10661 B, GameMain call #2) identity~~ — CLOSED 2026-08-23
   §7j.45 Part A: THE SHOP SCREEN, fully decoded (assets/SMK/SFX/music,
   the money floor ≥100, the MP/zone-7 15-dword availability array
   0x46cd48..0x46cd80, the 9-category catalog @0x4ea288 with immediate
   data, the auto-loadout/sell/buy-stage/confirm state machine, the
   weapon/chassis group layouts, and the MP loadout sync via the type-4
   COMMAND record bounded by [0x46cbe0] — the hypothesis confirmed;
   no map-room code lives here).
3. ~~Phase semantics of robots()' extra passes (fields 0x4c6a16/18/88/8c)
   and the state 1 producers (patrol?)~~ — CLOSED 2026-08-23 §7j.45
   Part B: the phase-0 pre-pass timer decays pinned (+0x32 BURN cooldown
   :=100 by FUN_004100b7; +0x34 ALARM cooldown; +0xA4 alarm counter DOES
   decay 1/frame — the D90 question closed; +0x88 shield −2/frame with
   the 10000 flash-invuln and the +0x8C CHARGE machine sourced from the
   equipment-chassis row word+2; +0x70 = the reinforcement delay with the
   [0x4de658] pending gate — both glosses §7j.54-corrected: the
   IDLE-TIME BOMBARDMENT arm + salvo cooldown); phases 4/5 gate re-verified; the 0x7d3
   countdown-dependent phase bound CORRECTED; state 1 has EXACTLY ONE
   producer — the FUN_00409138 COMMAND bit0 arm (no patrol semantics;
   SP never produces it).
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
   Still host-seamed: the FUN_0040eba0 TILE-WORD PRODUCER —
   DECODED 2026-08-22 §7h.4 (init_tiles staging + the four-site
   type-3 probe latch + the clear→move→test consume + set =
   zone+1; ZONEA/M1 stages ZERO pickup cells, ZONEB/ZONEF stage
   hundreds — the seam stays host-seamed by corpus fact, P4.2
   hooks D99; the dispatch decode + the case-1/2/3/7 bodies
   landed as pickup_case/apply_pickup seams §7h; case 4
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

## 7j.41. THE S7 PLATFORM-DYNAMICS DECODE — the trigger dispatcher
whole + the ring/creeper instruction-exact re-read + THE PER-FRAME
RNG FINDING (2026-08-23, worker 56d80c42 claim 2; objdump-only from
the COMMITTED ghidra-project/exw-text-objdump.txt + the read-only
corpus probes /tmp/opencode/s7probe{,2}.py, no Ghidra run; prep for
the E-side W12-S7 landing — every fact below was read off the raw
asm this run)

1. **FUN_00422600 — the per-zone BRIDGE-BUILD TRIGGER DISPATCHER,
   whole body** [verified 0x422600..0x422692 + the table bytes
   0x4225d0..0x422600]. Entry (eax = the destroyed instance's id
   dword, edx = the 0x46cbf4 record index): `ecx := id`; zone =
   [0x4edd8c]−1 ∈ 0..6 → `jmp [0x4225e4 + 4·(zone−1)]` — the zone
   code table [dword@0x4225e4.., bytes read this run]:
   zone 1 → 5; zone 2 → 0x84; **zone 3 → an MP-MODE SUB-DISPATCH**
   (0x422648: [0x4edd88]−1 ∈ 0..4 → `jmp [0x4225d0+4·idx]` → codes
   {0x6f, 0x7e, 0x80, 0x79, 0x88}; **[0x4edd88] = the WITHIN-ZONE
   MISSION NUMBER** [§1 load_mission paths, this pins its second
   use] — missions 1..5 pick the code, 6/7 match nothing); zone 4 →
   5; zone 5 → 0x2f; zone 6 → 0x2710 (a NEVER-code — no type row
   index reaches 10000); zone 7 → 0x84. `cmp ecx, ebx; jne ret` —
   the id must EQUAL the zone's code; then `rec = ptr[0x46cbf4] +
   0x14·record_index` and `FUN_00422832(rec.x@+0, rec.y@+4,
   rec.z@+8, ecx = 0x12C)` — **destroying an instance whose TYPE
   ROW INDEX equals the zone's trigger code builds a strength-300
   ring at the INSTANCE'S OWN (x,y,z)**. The §7j.12/9 gloss
   ("matching a record id") is corrected: the match is on the TYPE
   id, the BUILD SITE is the dying instance's record.
2. **FUN_00422832 — the SPREAD RING, call order pinned** [verified
   0x422832..0x4228cd]: saves (x−1, y−1) locals, then EIGHT
   FUN_004228ce calls in ROW-MAJOR N→S order: (x−1,y−1), (x,y−1),
   (x+1,y−1), (x−1,y), (x+1,y), (x−1,y+1), (x,y+1), (x+1,y+1) —
   the center tile is NEVER built. FUN_004228ce(eax=x, edx=y,
   ecx=strength, ebx=z) **gates in instruction order** [verified
   0x4228ce..0x422a3c]: bounds x/y ≥ 0 ∧ < [0x4eddec]/[0x4eddf0];
   `word[0x465daa+2·tile] == 0` (strength bank empty);
   `word[0x460dfa+2·tile] == 0` (object grid empty);
   `byte[0x46af58-bank + tile] == 0` (the tile-claim arena — the
   order-marker writers are the D82 seam, host-staged zeros);
   **the LIVE-ROBOT scan**: over all [0x46ccbc] records at
   0x4c6a60+i·0xA8, alive@[+0] ≠ 0, tile_x = (x_q13>>8 − 0xC)>>5,
   tile_y = (y_q13>>8 − 0xC)>>5 — the candidate tile is BLOCKED iff
   it is one of {(tx,ty), (tx,ty+1), (tx+1,ty), (tx+1,ty+1)} (the
   robot's tile + its E/S/SE neighbors — a +0xF00-offset marker
   robot blocks exactly its own tile's quadrant); `z ≠ 0`;
   `byte[0x4796ba+30·tile+2z]>>16 == 0` — the TOT-mirror z-word at
   the BUILD LEVEL is empty; `byte[DAT + tile + [z·4+0x4eaacc]] ==
   0` — the DAT VOLUME at level z is empty; **`byte[DAT + tile +
   [z·4+0x4eaac8]] == 1`** — the §7j.12/3 "plane-B" gate ANCHORED:
   the 0x4eaacc z_base table (z·w·h, §1 load_mission item 3) sits
   at 0x4eaacc, so `[z·4+0x4eaac8]` = z_base[z−1] for every z ≥ 1 —
   **PLANE B = THE DAT VOLUME AT LEVEL z−1 MUST BE 1** (build one
   level above a volume-1 surface). WRITES: FUN_0042394a(x, y, z,
   [0x454ae4+4·zone], volume 2) — the water z-structure (word =
   the zone stamped-WORD base, seen := (volume==0) = 0, DAT
   volume := 2); `word[0x460dfa+2·tile] := 0x7d4`;
   `word[0x465daa+2·tile] := strength`; FUN_0042223c(x, y, 4)
   (scorch +4).
3. **FUN_00422693 — the WEAKEN tail RE-READ, two §7j.12 glosses
   CORRECTED** [verified 0x4227c5..0x422831]: after
   `strength := new` + the +4 scorch: eax = the OLD strength;
   `old < 0xC8 → check-B; else new < 0xC8 → BUILD; else check-B`
   where check-B = `old < 0x64 → exit; new ≥ 0x64 → exit; BUILD`.
   **The ring gate is (old ≥ 200 ∧ new < 200) ∨ (old ≥ 100 ∧ new <
   100)** — NOT the §7j.12/2 "strength ≥ 100 and (diff < 200 or
   new < 100)" (that gloss builds for old ∈ [100,200) ∧ new ∈
   [100,200), which the asm REJECTS). The ring, when gated through,
   passes the NEW strength. **The creep-site latch
   [0x4dc5c8]/[0x4dc5cc] := (x, y) happens ONLY on the weaken→ring
   path (0x42281e)** — the DESTROY path (0x422746..0x4227c3)
   stores NO site (the §7j.12/2 "both paths store the site" gloss
   corrected); the destroy path = the FUN_0042394a(x,y,z,0,0)
   water-clear + both banks zeroed + the 5× k7 debris (delay
   0/2/4/6/8, 2 RandA each — confirmed).
4. **FUN_00422a9c — the CREEP tick, whole body** [verified
   0x422a9c..0x422c70]: **RandA draw #1 AT ENTRY, UNCONDITIONALLY
   (the 1/32 gate `test al,0x1f`)** — THE PER-FRAME RNG FINDING:
   the MissionShell epilogue calls this function EVERY frame (call
   0x44808a, straight-line, no branch around it — verified), so
   THE ORIGINAL CONSUMES ONE RandA PER FRAME on every mission even
   with no platform staged, plus TWO more (the x/y jitters) on
   every 1-in-32 lucky frame — an E-side stream gap on every
   scenario until a deliberate re-baseline (D113). Then: x =
   [0x4dc5c8] + (RandA&7) − 3 (draw #2), y = [0x4dc5cc] +
   (RandA&7) − 3 (draw #3); bounds; `word[0x465daa+2·tile] ≠ 0`
   (a platform must stand at the seed); the FIRST z whose mirror
   z-word is in the water range [0x454ae4+4·zone, +0xE) (z == 8 →
   exit); RandA&3 (draw #4) → jump table 0x422a8c = {0: (x, y−1)
   up; 1: (x+1, y) right; 2: (x, y+1) down; 3: (x−1, y) left}; the
   WALK reads the CURRENT tile's z-word at the found z (the seed
   tile passes by construction), steps, bounds-checks per step
   (OOB mid-walk → exit, NO build), loops while the z-word is in
   the water range; the first NON-water tile ends the walk —
   **step BACK one onto the last water tile** → bounds →
   `FUN_00422832(tip_x, tip_y, z, 0xC7)` (strength 199) +
   [0x4dc5c8/cc] := the tip. Bridges grow one ring per lucky
   frame from the last damaged/built site.
5. **CORPUS PROBE (read-only)**: the FUN_004228ce substrate
   (volume@(z−1) == 1 ∧ volume@z == 0 ∧ TOT word@z == 0) exists in
   EVERY zone (ZONEA/M1: 1361 tiles; the widest 9.8k in ZONEF/M1).
   **ZONEA/MISSION1 hosts the full S7 story in one spot**: the
   zone-1 code-5 instance = .POS slot 74 @ (3,57,2), type row 5 =
   (W1 H1 D2 hp 75 chain 0 kind 8) — one artillery blast (or one
   grenade detonation, hp 75) destroys it; ALL EIGHT (3,57)
   neighbors are z2 substrate (volume@z1 == 1 under them). A
   marker robot at (3,57) blocks its own E/S/SE quadrant
   ((3,58),(4,57),(4,58)) → five platforms build.
6. Engine routing this unit (D113): the spread ring, the creep
   tick, and the trigger build land in `bedlam-core::destroy` /
   `advance_frame`; the creep tick runs ARMED (grammar
   `platforms = 1`) so S0..S6 chains stay byte-identical while S7
   is faithful from frame 0; FUN_00422e0a (the delayed-trigger
   payload producer, FUN_00439c20 census-unidentified) and the
   timer tick FUN_00422cc2 STAY no-ops (documented E-gaps — the
   S7 destroy arms no timers in E).

   LANDED 2026-08-23 (W12-S7): engine ea2f259, S7.scen + the
   corpus timeline gate b9cbcf3 (chain b41db389f3ad8947), the
   differ_gate row + capture-plans/S7.json 4c6c068/13bae85 — see
   DESIGN §7 S7 row + D113.

## 7j.42. THE S8 CRITTER-CONTROLLER DECODE — the whole-kind dispatch, the state-4/5 mixed-AI mode machines, and the critter→robot damage lane instruction-exact (2026-08-23, worker f9af5743 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run; corpus probes read-only in /tmp/opencode)

Method: raw-table bytes extracted PE-exact from BEDLAM.EXW
(image base 0x400000; BEGTEXT RVA 0x1000 = file 0x400) +
instruction-walk of 0x412f34..0x41547e (the whole controller,
scratch /tmp/opencode/critter_ctrl.txt). All facts [verified]
against that dump unless tagged.

1. **THE MAIN LOOP, instruction-exact** (corrects the §7j.17
   gloss in two places): prologue 0x412f34 zeroes idx ([esp+0x24])
   and the record OFFSET ([esp+0xc4] — the record pointer idiom is
   `[reg + 0x4cff98 + field]` with reg = idx·0x7E, so a
   displacement D addresses field D−0x4cff98). Loop head 0x413ff6:
   idx vs count [0x46cc2c]; **presence w@+0x24 == 0 → skip the
   critter whole**; save x/y (d@+0x36/+0x3A → [esp+0x38]/+0x3c);
   **fuse w@+0x7C: nonzero → DECREMENT, every frame, BEFORE the
   dispatch** (0x414032..0x41404c — this is the SAME word the
   §7j.23 hit-flash write sets to 1: flash = a fuse the main loop
   burns, unified); then `ax = w@+0x00; dec; cmp 6; ja` → the
   7-entry KIND table @0x412f18 (PE-extract): **k1 → 0x414c96,
   k2 → 0x415216, k3 → 0x4145c1, k4 → 0x414079, k5 → 0x41367c,
   k6 → 0x41367c (shared body), k7 → 0x412f52**. Kind w@+0x00 is
   the .NME section's state word (§7j.18/§7j.23) — CORRECTS the
   §7j.17 "state w@+0" naming (the word IS the kind; the runtime
   MODE is w@+0xC). Epilogue per critter (0x413f8f..0x413fdc,
   run from the state bodies AND the invalid-kind skip): the
   presence mark (byte 1 at [row-ptr + DAT-volume + x>>13] with
   row-ptr = [0x4ea900 + (y>>13)·4], asm 0x413f8f..0x413fa7), the
   8-corner z-settle FUN_004182c3, moved? → FUN_0040ff92 (the
   tile-0x62 trap re-probe), offset += 0x7E, idx++.
2. **STATE 4 (seek steppers) — the body 0x414079** [verified]:
   entry zeroes the substep counter [esp+0x104], saves 6 record
   copies, jmp the dispatch head 0x414374. Head: `species w@+0x02
   ≤ substep-count → exit` — **the SPECIES word is the SUBSTEPS-PER-
   FRAME count** (S4 stamps 6; the wake re-stamps 6); the loop
   runs `species` iterations per frame. Mode dispatch (mode =
   dword@+0xA>>16 = w@+0xC):
   - **mode 0xB dormant** (0x41439e): countdown w@+0x56 vs
     DAT_00454edc[difficulty]; below → substep-0 increments it
     (0x4140b8); at/above → WAKE: presence := 1, anim w@+0xE := 0,
     countdown := 0, seek-dir d@+0x10 := RandA()&3, mode := 9,
     species := 6, hp w@+0x06 := 0xC8 (200), SFX BEAMIN 0x4edfe0
     (FUN_0043a48e, x>>8, y>>8, 2). §7j.29's wake-path record
     confirmed; its "+0x6 timer/+0x2 pause" gloss corrected to
     hp/substeps (the S4 loader stamps +0x02 = 6 = substeps, not
     "pause").
   - **mode 7 dying** (0x4140d7): counter dword@+0x52++, anim
     := 0; ≥ 0x28 → mode := 0xB, countdown := 0, BEAMIN.
   - **mode 6 ballistic** (0x414123): drift vs home ±0x8000 Q13
     leash, aim atan2 → FUN_00415ff2 step; ≥ 8 → mode := 7,
     counter := 0 (the landing → death-dive of §7j.17's mode-6
     family — full landing producers documented §7j.17 item 1).
   - **mode 9 SEEK walk** (tail 0x4142bd → the §7j.29 walk):
     gate dist < 0x1F4 (500 px); countdown w@+0x56 == 0 → the
     RE-PICKER: RandA, (al&3)==0 → heading := RandA&3 (25%
     random) else heading := FUN_004181bd(idx) (dominant-axis
     direction toward the nearest robot), then countdown :=
     (RandA&0x3F)+0x20; countdown ≠ 0 → 0x4144d7: countdown −= 1,
     4-way dispatch (dir 0 FUN_00417f2c y−1: OK → y--, call
     FUN_00415490; blocked → countdown := 0 | dir 1 FUN_00417fe8
     x+1: x++ … | dir 2 FUN_004180c0 y+1: y++ … | dir 3
     FUN_0041813d x−1: x−− …), table @0x412ef8 PE-exact
     {0x414346, 0x41443b, 0x41446f, 0x4144a3} ✓ §7j.29.
   - **mode 2 RANGE-ATTACK** (0x4144f8): gate dist < 0x1F4;
     countdown == 4 → mode := 9 (re-seek); else FIRE
     **FUN_0040db9e(robot = w@+0x7A, mult = 2, seed =
     heading<<6, damage = 1, param_5 = −1)** (asm 0x414549..
     0x414575 — §7j.17's gloss anchored); substep-0 →
     countdown++ (so a fresh acquisition fires `species` hits per
     frame for 4 frames then re-seeks).
3. **STATE 5/6 (the shared body 0x41367c)** [verified]: entry —
   RandA gate (al&0x1F)==0 (1/32) → facing w@+0x72 :=
   (RandA&0x1F)−0xF idle drift; then the mode ladder:
   - **mode 0xB dormant** (0x413a28): countdown w@+0x56 vs
     DAT_00454edc[difficulty]; below → substep-0 increments +
     the countdown == table−9 gate plays BEAMIN (the pre-wake
     sound, 0x4136bc..0x4136fc); at/above → WAKE: anim := 0,
     countdown := 0, presence := 1, heading d@+0x10 :=
     FUN_0041ec1c(0xFF) (a RandA&0x7FFF-bucketed pick, body
     0x41ec1c: `idiv 0x8000/n` clamp n−1), species := 3, mode
     := 8, hp := 0x96 (150 — the k5/6 base, §7j.18 S3/S6).
   - mode 0xA (0x41370e): countdown == 0 → mode := 8, anim := 2
     (a timed pause → re-engage).
   - mode 7 dying (0x41373b): anim := 0, counter++ ≥ 0x28 →
     mode 0xB, countdown 0, BEAMIN. mode 6 ballistic
     (0x41378a): atan2 at home, FUN_00415ff2 step, anim++ ≥ 8 →
     mode 7 + counter 0. mode 5 (0x41383d): anim > 1 → mode 8 +
     anim 2; else aim home + step (§7j.17's "brief rise").
   - **mode 8 ENGAGE** (0x413a98): gate [0x4dd410] == 0 (SOLE
     text reference = this cmp — a computed-store MP cell, ≡ 0
     on SP); FUN_00417c00(x>>8, y>>8, &dist) — the
     nearest-ALIVE-robot scan (0x4c69e4 bank, stride 0xA8, alive
     d@+0x7C ≠ 0, RAW octile FUN_0041ebf8 on px deltas, sentinel
     (idx 0, 10,000,000) when none, asm 0x417c00..0x417c5c);
     dist < 0x60 (96 px) → 0x413b63: leash `(d+1)·0x40 + 0x258`
     (600/664/728) ∧ dist > 0x80 → 1/128 (RandA&0x7F==0)
     FUN_00421ed6(x>>8, y>>8) [identity CLOSED by §7j.52/D124:
     the GRUNT1/2/3 trio — RandB()%3, cells 0x4ee000/04/08,
     priority 2, [0x4ede58]-gated; the §7j.42 "juice/squawk
     family" guess right in kind], aim heading := angle(robot−critter)+0x80&0xFF →
     step; dist ≥ leash or ≤ 0x80 → 0x413c20 (the
     approach/retreat path — §7j.17 mode-3/10 family);
     dist ≥ 0x60 → aim + FUN_00415ff2 step toward (0x413aae
     tail).
4. **FUN_0040db9e — the critter ranged-attack applier, whole
   body** [verified 0x40db9e..0x40dc13; CORRECTS the §7j.18/4
   table gloss]: damage = `dword[0x476fe4 + 0x30·param_5]`
   (stride 0x30 = 48, NOT 0xC — the §7j.17 "0xC-stride" was
   wrong; param_5 = −1 → **0x476FB4**), then FUN_0040e230(robot,
   damage-seed ecx = 1, owner = the table dword) — so the critter
   lane is damage 1/hit with the owner tag = dword@0x476fb4.
   mult ≠ 0 (the mode-2 call passes 2) → the STUN/KNOCK half:
   robot w@+0x10 := 0xFFFF (via `[0x4c69f4 + 0xA8·idx]`), then
   FUN_0040c536(idx, cos(seed)·mult>>7, seed, sin(seed)·mult>>7):
   SP gate [0x4eaac0] == 0, robot state ∉ {3,5} → robot
   w@+0x0E := seed, walk-probe gate FUN_0041e897((x+vx)>>8,
   (y+vy)>>8, …) → x += vx, y += vy, w@+0x10 := −1 (asm
   0x40c536..0x40c604). §7j.17's "dist·mult>>7" gloss corrected
   (the scale factors are cos/sin of the SEED, not the distance).
5. **The corpus census** (read-only, /tmp/opencode/nme_census.py,
   the §7j.18 exact schedule): **ZONEA/MISSION1.NME hosts
   critters** — S3/MixedState5 = 6 records
   {(1,1,18,9),(1,1,18,8),(1,1,18,7),(1,1,7,8),(1,1,7,7),
   (1,1,7,6)} and S4/SeekSteppers = 5 records
   {(1,1,13,9),(1,1,22,8),(1,1,22,6),(1,2,3,6),(1,2,2,7)}; at
   difficulty 0 the loader spawns 6 kind-5 (max(d,1) = 1 each,
   ACTIVE mode 8) + 10 kind-4 ((d>>1)+2 = 2 each, ACTIVE mode 9,
   one RandA&3 heading draw each) = **16 critters, all ACTIVE
   from frame 0** (neither ZONEA family spawns dormant — the
   dormant path is only reached after a death: mode 7 → 0xB).
   ZONEB/M1 (S5's zone) hosts 63 critters (24 wander + 20 state-5
   + 10 chase + 9 state-6) — the S5-family chains are unaffected
   (no `critters` key → no staging, no draws).
   **STREAM CONSEQUENCE (the D113 pattern, bigger)**: the
   original loads .NME natively at EVERY mission load and the
   controller (MissionShell 0x447fe1, ungated) runs all ACTIVE
   critters EVERY frame — the ZONEA/M1 loader alone consumes 10
   RandA (the kind-4 headings) at load + per-frame draws
   (kind-5: 1/frame each idle-gate + the 1/32 extras; kind-4:
   the walk re-picker draws on countdown expiry) — an E-side
   stream gap on S0..S7 until a deliberate re-baseline; S8+ arms
   the family (`critters = 1`) and models them.
6. Engine routing this unit (D114): the critter bank + the .NME
   staging host seam + the state-4/5 controller subset (the
   corpus kinds — k1/l2/k3/k7 bodies stay E-gaps until a zone
   hosting them needs them) land in `bedlam-core::critter`;
   the damage lane lands as FUN_0040e230's existing
   `apply_damage` + the stun/knock half; the §7j.24 death
   handlers land beside them (bounty → score, debris → the
   destroy ring). SFX stays T4/E-gap (BEAMIN, the FUN_00421ed6
   juice, the death trios).
7. **THE KIND-5/6 MODE-2/3/8 FIRE CYCLE** [verified — the
   §7j.42/3 engage path completed]: mode 8 with dist in
   [0x60, leash] → **TRANSITION, not attack** (0x413c20):
   mode := 2, anim := 0, countdown := (RandA&0x1F)+0xA, aim
   heading := angle(robot−critter) (octile-normalized atan2),
   ATTACK-TARGET staged d@+0x2A/+0x2E/+0x32 := robot x/y/z;
   dist beyond the leash or < 0x60 → substep skip (a FAR
   critter is fully quiet beyond the entry gate draw).
   MODE 2 (0x413d00): substep-0 → anim++; anim ≤ 1 → tail;
   anim > 1 → anim := 0, FUN_0041286f free slot (50×0x22
   bank); slot ≠ −1 → spawn **projectile type 0x68** at the
   critter x/y (z+0x10)<<8, velocity = the octile-normalized
   (dx·0x800/dist, dy·0x800/dist) + vz =
   ((target_z+4)−(z+0x10))·0x10/denominator — a full 3-D aim
   at the STAGED target; SFX BIOFIRE 0x4edff0 (no stream
   draw); tail: countdown−−, == 0 ∨ slot −1 → the break roll
   (d=0: RandA&7==0; d=1: RandA&0xF==0; d=2: always) → mode
   := 3, anim := 2, countdown := 6, RandA&1 → heading ±0x40.
   MODE 3 (0x413f06): 1/128 juice (RandA&0x7F==0 →
   FUN_00421ed6 — the draw ALWAYS consumed), step
   FUN_00415ff2(idx, heading), substep-0 → anim wrap 6
   (FUN_0041642d), countdown−− → 0 → mode := 8, anim := 2.
   So the engaged cycle is 8↔2↔3 and EVERY mode consumes its
   documented draws; the 0x68 records are the first
   enemy-produced rows on the ALIASED projectile-bank T2 row.
   The kind-5/6 RESPAWN-delay table DAT_00454edc (DGROUP,
   file-extract) = {1500, 900, 600} per difficulty.

## 7j.43. THE S8 ENGINE-LANDING ADDENDUM — the §7j.42 WIP corrections (asm re-verification during the W12-S8 engine leg) + the corpus-engagement findings (2026-08-23, worker 40dd9473 claim 2; objdump-only re-walks of 0x41367c..0x414596, no Ghidra run)

The engine leg adopted the interrupted predecessor WIP against
the committed §7j.42 notes and re-verified every mode body
against the objdump. All facts [verified] against the same
ghidra-project/exw-text-objdump.txt.

1. **FIVE corrections to the §7j.42 glosses + the WIP** [each
   asm-verified this unit]:
   - **The kind-5/6 mode-2 break roll at d=2 NEVER fires**
     (0x413e81: `cmp difficulty,1; jne 0x4139fd` — d≥2 jumps
     straight to the substep burn, NO draw): the §7j.42/7
     "(d=2: always)" gloss was inverted. d=2 breaks ONLY on
     countdown==0 ∨ slot−1 — and the ±(FACING+0x40) strafe roll
     (0x413ebf, one draw) runs on EVERY break path including
     those (the strafe delta is facing+0x40, not the bare 0x40).
   - **The kind-5/6 mode-6 dive aims AT THE IMPACT and steps the
     REVERSED heading** (0x413793..0x413804: the angle of
     (impact−critter), `lea edx,[eax+0x80]` into FUN_00415ff2,
     the record's heading field keeps the AIM); the WIP aimed at
     HOME and stepped forward. Mode 5 (rise) writes the at-impact
     heading with NO step (0x413871..0x4138da) and its anim>1
     flip RE-DISPATCHES engage the SAME substep (`jmp 0x4136fc`
     at 0x41386c) — as does the dormant WAKE (0x413a93).
   - **The kind-4 mode-6 leash reads IMPACT not home**
     (0x41412c: `(x<<8)−impact_x` — Q13), the countdown decrement
     is UNCONDITIONAL at substep 0 (not nested under the
     leash-out), and the ONLY mode-7 transition is
     countdown==0 (0x41424f) — the WIP's anim≥8 path was
     invented. The dive speed multiplier is
     max(countdown, 2) (0x4141a4: the dword@+0x54 sar-16 read).
   - **The ENGAGE band geometry** [exact, 0x413ad1..0x413cfb]:
     dist<0x60 → POINT-BLANK RETREAT (heading := aim+0x80+facing,
     step, the substep-0 anim-6 wrap) — the §7j.42/3 "quiet
     freeze" gloss was wrong; 0x60≤dist≤0x80 → the TRANSITION;
     0x80<dist<leash → the juice draw (always consumed) +
     heading := aim+FACING with NO +0x80 (the WIP added it);
     ≥leash → quiet. The kind-4 WAKE also falls into the seek
     tail the same substep (0x414421: `jmp 0x4142bd`) — the WIP
     had this right.
   - **The kind-4 walk-probe/staging scale**: the §7j.18 S4
     loader stamps RAW px (= Q5) x/y — the staging z probe and
     the stepper probes read floor_z(x, y, …) DIRECTLY (no >>8,
     bounds x>>5), while the mixed kinds' Q13 coords shift. The
     WIP applied the Q13 scale to both. Same for
     FUN_004181bd's dominant-axis deltas (the asking critter's
     OWN record — the WIP read critter[0] for everyone) and the
     SIGNED sine-word reads (the table is i16; a u16 view loses
     every negative step — the engine's homing-steering site has
     the same unsigned read and is flagged for its own unit).
2. **The mode-6 LANDING producers do NOT exist on the corpus
   kind bodies** [verified — corrects §7j.17's "landing → 8× k6 +
   5× splash + 0x18 rows" claim]: ZERO calls to FUN_0041a14f /
   FUN_00424355 / FUN_00420608 / FUN_00421f4c in
   0x413600..0x414600 (the k5/6 + k4 bodies); both corpus
   mode-6→7 transitions (0x413815 kind-5, 0x41424f kind-4) write
   ONLY the mode/counter. The 0x413244 0x18-row call sits in the
   k7 body (0x412f52.. — corpus-dead). The §7j.17 expectation
   applies to kinds the corpus never stages.
3. **THE CORPUS ENGAGEMENT FINDINGS** (the S8 canonical run,
   chain b5ae3f8be91c7449, D114):
   - ZONEA/MISSION1 terrain: rows 5-10 and 13+ at x12..23 are
     floor 31 EXCEPT a 95-plateau at (16..19, 11) + the (15,·)/
     (20,·)/(12..15,12) 63-strips — a marker staged on the
     plateau puts the artillery burst one z-level high and the
     §7j.23 z-box (|Δz|<0x20) misses every critter. The S8
     gunner stages at (18,13) — the flat row.
   - The (18,·) pack APPROACHES under the juice draw and crosses
     the transition band within frames (2-3 critters cycling
     fire/chase); the burst's ring lists kill the approached
     pack + the walked-in kind-4s (9 dead by f39: mode-6 dives,
     mode-7 dying 0x28, mode-0xB dormancy — the d=0 respawn
     table 1500 frames out).
   - The bounty gate stays DARK on the corpus path: the blast is
     a script kill (attacker −1), and robot-owned critter kills
     need bullet records — whose inline spawns do not exist
     (the S3-documented AI-order family E-gap). The gate is
     pinned synthetically only.
   - The kind-4 far quad at (3,6)/(2,7) z2 never moves (the
     probes fail on the 63-wall band around them) — a faithful
     blocked-path freeze, carried as seek-mode census weight.
4. **Engine routing LANDED (D114)**: bedlam-core::critter — the
   bank + the .NME staging host seam (the §7j.18 schedule, the
   unmodeled kinds refused fail-loud) + the k4/k56 controller
   subset + the FUN_0041a14f effect-row bank (the §7j.24/5 LRU
   allocator, 3 draws/row + 1 per overflow id row) + the §7j.24
   death handlers (the {0x24,0x29,0xC} weapon-gated debris +
   rows) + the odd-pass FUN_004197d4 walker + the FUN_004190bc
   applier at the bullet-substep and script-blast lanes. The
   critter bank + effect rows are the E-ONLY T2/T3 coverage rows
   (never in state_hash — the W6 split); the ALIASED
   observables: the RNG stream, the robot bank, the projectile
   bank (the 0x68s), the score bounty. Grammar v1.7
   `critters = 1`; the S0..S7 chains byte-identical without it.

## 7j.44. THE FUN_0040de9c DEBRIS-PHYSICS DECODE — the whole per-frame pass: the countdown semantics, the three collision walks, and the param arithmetic (2026-08-23, worker a5ef2370 claim 2; objdump-only from ghidra-project/exw-text-objdump.txt 0x40de9c..0x40e21b + the helper bodies 0x40db9e/0x40dc1b/0x40dce0/0x40eb3c/0x4128ec/0x412998/0x41e9a2/0x41eb65..0x41ebf8, no Ghidra run)

1. **THE +0x20 PHYSICS WORD IS A COUNTDOWN, not a class index**
   [verified whole]: FUN_0040de9c ends \`dec [R+0x20]\`
   (0x40e20d, the fall-through exit) — every physics frame
   decrements it, and the TICK gate (FUN_00420549 @0x420590,
   \`+0x20 != 0\` → call) stops calling when it reaches 0. A
   class-6 chunk therefore moves/damages for exactly 6 frames,
   class-1 for one. **The 0x454510 dword table is NOT this
   function's param table** — FUN_0040de9c reads no table: the
   two params are ARITHMETIC in the CURRENT (pre-decrement)
   phys value [verified]:
   - knock_mult (robots) = \`min(phys, 3)\` (0x40def4).
   - radius (critters) = \`min(16·phys + 0x20, 0x60)\` (0x40dfc6):
     phys 1→0x30, 2→0x40, 3→0x50, 6→0x60. A class-6 chunk's
     critter radius decays 96/96/96/80/64/48 across its frames.
   - mag_const (both lanes) = \`kind == 12 ? 25 : 2\` (0x40dec6) —
     the +0x1C kind word, nothing else. **Only the destroy-tail
     k12 five-chunk burst carries mag 25**; every other physics
     kind (1/3/4/6/8/9/13..20) deals mag 2.
2. **THE ROBOT LANE = the ALIVE-robot Q13 damage walk**
   [verified 0x40df06..0x40dfbc]: over ALL \`[0x46ccbc]\` robot
   records (the TOTAL count, D89), gates \`d@+0x7C != 0\` (ALIVE)
   ∧ \`word@+0x0C != 2\` (the unaligned \`dword@+0x0A sar 16\`
   read; state 2 = dead). Δ = robot Q13 pos (d@+0/+4) −
   (debris x/y \`<<8\`) — the debris Q5 pair up-scaled. Octile
   (FUN_0041ebf8: max+min/2) \`>>8 ≥ 0x40\` (64 px = 2 tiles)
   skips; 0→1. heading = the SAME atan2 pair the engine already
   models (FUN_0041eb7d 64-bucket bin-search over the sin-table
   quarter at [0x46cbd0]+4 + FUN_0041ebc1 quadrant fold —
   byte-identical to \`AngleTable::angle_byte\`). Then
   **FUN_0040db9e(idx, knock_mult, heading, mag_const,
   debris_slot)** — the W12-S8 critter ranged-attack
   dispatcher, already landed: \`FUN_0040e230(robot,
   damage=mag_const, owner=debris.+0x28 param)\` + facing
   w@+0x10 := −1 + the FUN_0040c536 knock
   \`(sin·k>>7, cos·k>>7)\` = \`robot_move\`. So a robot under a
   rolling chunk takes **2 (25 for k12) damage per frame per
   chunk**, plus a ≤3-px knock. NO terrain gate on this lane.
3. **THE TERRAIN GATE gates ONLY the critter lane** [verified
   0x40dfbc..0x40e03f]: the 3-row DAT-volume dword probe at
   tile (x>>5 −1, y>>5 −1) — rows y−1/y/y+1 via the y-line
   table 0x4ea900 + [0x46af4c] + column x−1, each read a DWORD
   (covers columns x−1..x). ANY nonzero dword → the critter
   walk runs; ALL zero → skip straight to the POI walk. I.e.
   debris over EMPTY ground cannot hit critters (the 3×2
   up-left block is the cover test).
4. **THE CRITTER LANE** [verified 0x40e03f..0x40e144]: over
   \`[0x46cc2c]\` critters, gates presence w@+0x24 ≠ 0 ∧ mode
   w@+0x0C ∉ {7, 6, 0xB}. Position via **FUN_004128ec = the
   per-kind critter position GETTER** [verified whole + its
   0x4128d0 jump table]: kinds 1/4 → (x,y) \`<<8\` + z \`<<8\`
   (native Q5), kind 2 → x/y/z raw (native Q13), kinds 3/5/6/7
   → x/y raw, z \`<<8\`. Pre-filters |Δx|,|Δy| < 0x8000 (4
   tiles, overflow guard) then octile>>8 < radius. dmg_falloff
   = \`((radius−1) − dist_px) >> 3\` (0x40e138 — always ≥ 0
   since dist < radius). Then **FUN_0040dce0(idx, dmg_falloff,
   heading, mag_const)** — the §7j.24 debris-crush dispatcher;
   NOTE the 7j.24 arg gloss had mag/dmg REGISTER-swapped: the
   body proves ebp(EDX) is the >2-gated knock multiplier AND
   the sin/cos·mag factor, while ecx(ECX) — the mag_const
   2/25 — is what FUN_0040eb3c subtracts from hp
   (\`FUN_0040eb3c(idx, dmg)\` = \`if presence { hp w@+0x06 −=
   dmg }\`, 21 bytes, verified whole). So: falloff > 2 required
   (dist < radius − ~24), hp −= 2/25, knock = pos +
   (sin·falloff>>8, cos·falloff>>8) stored per-kind via
   **FUN_00412998 = the per-kind position SETTER** (0x41297c
   jump table: kinds 1/4 take the args \`>>8\`, others raw; z
   arg −1 leaves z untouched), the move gated kind 7 always /
   otherwise **FUN_0041e9a2 = the 8-corner critter walk probe**
   (the ±corner-offset tables 0x4543e4/0x454404, per-corner
   bounds + FUN_0041e411 z-probe + |z−corner| ≤ 4), then hp ≤
   0 → attacker := −1 + the per-kind death dispatch (k4 weapon
   0, k5/6 weapon 0x24 skipped in modes {5,6} — §7j.24/3).
5. **THE POI LANE runs ALWAYS (no terrain gate)** [verified
   0x40e145..0x40e20c]: over \`[0x46cbf0]\` POI/personnel records
   (0x4dabdc/0x1E, the FUN_00412a98 bank §7j.17), gates active
   w@+0 ≠ 0 ∧ w@+2 ∉ {5,6,7}. Δx/Δy = POI xyz d@+0xE/+0x12 −
   debris<<8 (Q13); octile>>8 ≥ 0x30 (48 px) skips; then
   |POI z d@+0x16 − debris z| ≥ 0x20 skips. Hit →
   **FUN_0040dc1b(poi, mag = (0x40 − dist_px)>>2)** [verified
   whole]: \`w@+2 −= mag\`; on w@+2 ≤ 0 → w@+4 := 6 (PANIC
   state) + w@+0xA := 0 (timer) + the RandB&1 DEADMAN1/2 thud
   SFX pick (0x4edfb8/0x4edfbc) + a kind-10 debris staged at
   the POI pos (delay 0, param −1). The personnel-squash
   effect. E-side this lane is DEAD CODE (no POI bank is
   staged — the §7j.18 .NME section-8 loader is not modeled);
   documented, not landed.
6. **Callers + corpus census**: FUN_0040de9c has exactly ONE
   caller — the tick FUN_00420549 @0x420585 (the MissionShell
   epilogue 0x448076, i.e. AFTER the robot phases, BEFORE the
   armor-pad fade FUN_00424051). S7's destroy tail stages
   k14/k16..19/k20 (the five-effect loop) + k15 (TRT death) +
   k6 + FIVE k12 — every one phys-6 EXCEPT k10; S8's critter
   deaths stage k7 only (phys 0 — no physics); S4's destroy
   legs stage the same five-effect/k12 family; S0..S3/S5/S6
   stage nothing. The ROBOT lane is aliased (robot bank
   hashed), the critter lane is aliased on S8 (bank live), the
   POI lane stays E-only.
7. **LANDED 2026-08-23 (D115)**: the engine leg lives in
   bedlam-core (commit cebc178 — `debris_tick`/`debris_physics`/
   `debris_critter_lane` in destroy.rs, the MissionShell epilogue
   slot in mission.rs per item 6's call order, the +0x18 anim
   split on DebrisRecord; the POI lane documented, not landed) +
   the re-baseline commit b2c89af (the five chain moves the
   turn-on forces: S3/S4/S5C/S7/S8 — mines/grenades expire to
   k12 mag-25 chunks so even non-destroy scenarios move; the
   damage-lane assertions on corpus_s4/s7/s8 = the observability
   pairing). The 7j.11/5 census task is CLOSED-BY-DISPROOF (item
   1 — no param table exists); the "debris-stager ENGINE
   widening" Backlog bullet's physics-class clause is DONE (the
   k2/k8 scorch + k1/k20 ring clauses were already landed with
   the 7j.11 stager — the bullet RETIRES).

## 7j.45. THE FUN_00440e45 SHOP DECODE (§9 item 2 CLOSED) + the robots() extra-phase/state-1 semantics (§9 item 3 CLOSED) (2026-08-23, worker c607288e claim 2; objdump-only from ghidra-project/exw-text-objdump.txt 0x440e45..0x4437e5 + the helper bodies 0x4437ea/0x443870/0x44395b + FUN_0040b9f6 0x40b9f6..0x40c536 + FUN_0040e230 0x40e270..0x40e2b0 + FUN_004100b7 0x41036e..0x4103e3 + FUN_00409138 0x40a37b; DGROUP strings re-read read-only from BEDLAM.EXW, delta 0x401A00; no Ghidra run, no corpus write)

PART A — **FUN_00440e45 = THE SHOP SCREEN, fully decoded** [verified
instruction-exact]. The §7d/RE-EXW-MUSIC identification is CONFIRMED and the
"inter-mission shell (shop/map room)" hypothesis RESOLVED to SHOP-ONLY (no
map-room code lives here; the map room is FUN_0043e7d4 per §7d.4).

1. **Entry sequence 0x440e45..0x441251** [verified]:
   - ArenaAlloc (FUN_0041db89, EAX=size) ×5: 0x4b500 = the 640×480 shop
     screen buffer; 0x30d40 = WEAPICON.BIN dest; 0x1770 = CONLITE.BIN dest;
     0x14ff0 = SHOPFONT.BIN dest (alloc arg carries ecx=0xe10 too — the
     scratch-bank size); 0x2f4d60 = SHOPLITE.BIN dest. .bss scratch memset-0
     (FUN_00402965): 0x4e8818(0xe10), 0x4e7ed8(0x4a0), 0x4e8378(0x5a0),
     0x4ea288(0x150) — the last is the CATALOG bank cleared before the stager.
   - Loads via FUN_0041cc7f(eax=name@DGROUP, edx=dest): "GAMEGFX\DARKPALS.PAL"
     @0x459498 → palette cell 0x4edc00, "GAMEGFX\WEAPICON.BIN" @0x4594ad,
     "GAMEGFX\CONLITE.BIN" @0x4594c2, "GAMEGFX\SHOPFONT.BIN" @0x4594d6,
     "GAMEGFX\SHOPLITE.BIN" @0x4594eb — the §7d bank list lands EXACTLY.
   - SFX registers via FUN_0043a39c: BEEP1→0x4edfc8, BEEP4→0x4edfcc,
     BEEP7→0x4edfd4, BEEP5→0x4edfd8 (the shop's MENU-set; 7j.30 per-screen
     cell reuse confirmed). Music: FUN_00403642("SOUND\MIDI\SHOP" @0x459550,
     3). Palette: FUN_0041cbf0([0x4edbf8], 0xa).
   - **SMK intro** [0x46cca4 ≠ 0 gate — NEW pin = the "play animations"
     config flag]: eax = FUN_0041ce69("GAMEGFX\SHOP.SMK" @0x459560); eax==0 →
     FUN_00420100 + print "ERROR: COULD NOT OPEN SHOP SMACK\n" @0x459571 +
     FATAL EXIT (FUN_00444d2da(1)). Frame loop 0x4410b4..0x441142: i <
     [smk+0xc] (frame count) ∧ [0x4edb50]==0; audio track gate [smk+0x68] →
     FUN_00402aaa(0x300, smk+0x6c) + FUN_004258d0; decode FUN_0045304a /
     advance FUN_00453044 / present pair / frame-timer wait FUN_00453038;
     close FUN_0045303e + FUN_00425851. [0x46cca4]==0 → skip the SMK and load
     "GAMEGFX\SHOPPAL.PAL" @0x459593 into [0x4edbf8] instead.
   - Backdrop: FUN_00401e39(img 0, transp 1, 0,0; ESI=SHOPLITE, EDI=screen).
   - **MONEY FLOOR: `if ([0x46ae70] < 0x64) [0x46ae70] := 0x64`** @0x4411a0 —
     shop entry guarantees money ≥ 100 (a NEW state fact for the campaign
     model; D51 fresh-campaign 4000 unaffected).
   - **MP/zone-7 AVAILABILITY: if ([0x4edb88]==2 || [0x4edd8c]==7) → the 15
     consecutive dwords 0x46cd48..0x46cd80 := 1** @0x4411e1..0x441251 —
     NEW pin: the item-availability flag array (value 2 = transient, see the
     exit normalize below). MP mode 2 OR the zone-7 (final) mission enables
     all 15 flags; multiplayer then disables categories 2 and 8 in the
     catalog stager. See RE-EXW-MISSION-ROOM.md for the exact mapping.
   - call FUN_0044395b = the CATALOG STAGER (see 2).
2. **The catalog grammar** [verified, FUN_0044395b 0x44395b..]: 9 category
   blocks @0x4ea288, stride 0xA0. Header: +0x00 x0, +0x04 y0, +0x10 click-radius,
     +0x14 item-count, +0x18 col-width, +0x1C row-height. Items (stride 0x10,
     first at cat+0x20): name-word@+0x20 (a FUN_00420260 name index), price
     @+0x24, pack-ammo @+0x28, available @+0x2C. Cat 0 = NEEDLER CANNON
     #1/#2/#3 (names 2/3/4; prices 100/250/400; ammo 300/400/500) — a table
     of immediates, fully regenerable from the same range. Category 8 =
     the CHASSIS block (base 0x4ea788): the 5 equipment chassis ids
     0x2A..0x2E (AUTO SHIELDING / BATTERY PACK / THERMAL DAMPER / SCANNER
     LEVEL 2/3, §7d.5), whose +2 word = the SHIELD-CHARGE count (Part B).
     0x2D/0x2E are a MUTEX pair (FUN_00443870 refuses owning both).
3. **The interaction loop** 0x441257..0x4437b6 [verified; input via the
   shared mouse cells 0x4eddc4/0x4eddc8 + click latch 0x4eddcc + the
   debounce local, polled inside the draw/present family]:
   - locals: [esp+0x3a0] = selected-item name idx (−1 none), [esp+0x3ac]
     category, [esp+0x3b0] item idx, [esp+0x3a4] staged SPEND, [esp+0x3a8]
     staged AMMO, [esp+0x3b8] page, [esp+0x39c] debounce.
   - **AUTO-LOADOUT button** (rect 0x1e2..0x20d × 0x189..0x19a, highlight
     img 2): (a) SELL-ALL — walk the 7 weapon groups (0x4de664+type·0x62,
     stride 0xE): every owned group refunds money += word@+6 (the PRICE)
     and zeroes {+0, +2, +6, +0xC}; then the 2 chassis rows (0x4deafc +
     type·0x1C, stride 0xE) likewise; (b) BEEP4; (c) if money ≥ 0x960
     (2400) ∧ a chassis is staged ([0x4ea7f4] ≠ 0) buy one via
     FUN_00443870(4); (d) n = FUN_0041ec59(5)+3 (3..7) random purchases:
     cat = rnd(9), item = rnd(count), gate [item+0x2C] ≠ 0, ≤50 tries each;
     category 8 → the chassis writer, else FUN_004437ea free slot; each buy
     writes the group and decrements money; then an ammo TOP-UP pass over
     all owned groups (money < price → mark, else ammo += pack, money −=
     price) and a bubble SORT of the 7 groups by dword[0x456c7c + 4·cat]
     (NEW pin: the 9-entry category-RANK table; 3× FUN_00402aaa swap
     through scratch 0x4dec4c).
   - **SELL**: click a loadout row — weapon rows x∈[0x217,0x27c],
     y∈[0x154,0x19b], row = (my−0x154)/0xA clamp ≤6 (0x441acf); chassis
     rows x∈[0x220,0x27c], y∈[0x1a0,0x1b3], row clamp ≤1 (0x441e4c).
     Owned gate word@+0 ≠ 0 → BEEP7 + money += word@+6 + zero
     {+0,+2,+6,+0xC} + [0x4dc6a8] := 0 + select the item + redraw its
     category panel (FUN_00401e39 img = cat+1 over SHOPLITE) + the icon
     grid (FUN_00402a56, cat geometry fields) + names (FUN_0042014e /
     FUN_00420260 name switch ×2 = item + weapon-name strings) + text
     draws (FUN_0043e183, fmt 0x4595a7 '%03i/%03i' = the stock readout).
   - **BUY**: catalog item click (hover row from the marker walk) selects
     {name, price, pack-ammo, item} (debounce 3, BEEP4 on new selection;
     unavailable [item+0x2C]==0 → deselect). The **+** button
     (x∈[0x271,0x27a] × y∈[0x13c,0x14d]) accumulates spend += price ∧
     ammo += pack while spend+price ≤ money (BEEP5, debounce 8, the
     name/cash redraws, fmt 0x4595c0 'CASH: %i %s: %i'); the **−** button
     (x∈[0x1e1,0x1ea]) reverses both (minimum one pack: 0x442486..48a rejects a
     resulting amount <= 0). Scanner items (8,3)/(8,4) reject both
     quantity controls (0x442260..28a and 0x442490..4b5). The **CONFIRM** button
     (x∈[0x1e2,0x20d] × y∈[0x154,0x165], highlight img 0) commits:
     weapon → slot = FUN_004437ea(item, cat) (pass 1 = DEDUP: an owned
     name → −1; pass 2 = first word@+0==0 free slot, else −1); on slot ≥ 0
     write the group {+0 name, +2 staged ammo, +4 := 0, +6 staged spend,
     +8 category, +0xA item, +0xC := 1} and money −= spend; chassis →
     slot = FUN_00443870(item) (same-name → that slot; the 0x2D/0x2E
     mutex; else first free of 2); write {+0 name, +2 staged ammo, +4 := 0,
     +6 staged spend, +8 := 8, +0xA item} (+0xC untouched). [The
     AUTO-LOADOUT writer differs only at +4: it stores 7 minus the outer attempt index
     there — this initializes the shop label animation counter; the
     subsequent drawer increments it toward 9 (2026-09-06 correction).]
   - **DONE button** (x∈[0x238,0x267] × y∈[0x1be,0x1d8], highlight img 5,
     gated [0x4dc694] ≠ 0 — the enable flag written by the icon-grid
     drawer FUN_00440287 @0x44029d/0x4402e5): requires an OWNED weapon
     group (else stays); SP → the exit sequence; MP → draw the two
     waiting boxes (FUN_0043c87c over 0x46b49c/0x46b4cc, y 0xbe/0xdc)
     then the same exit + the MP sync (5).
   **2026-09-06 correction:** +4 is the shop icon animation counter, not
   unconsumed shop state: 0x4402c2..eb increments it toward 9 and clears
   the DONE-ready flag while an owned row is still animating. DONE scans
   for a nonzero name word, so it requires at least one owned weapon;
   the former "FREE weapon group" wording reversed the predicate.
   See RE-EXW-MISSION-ROOM.md for the exact scan and gate addresses.
4. **Weapon-table writer census refined vs §7d.2** [verified]: the group
   7-word layout is +0 name_idx, +2 ammo, +4 shop animation counter,
   +6 price, +8 category, +0xA item_idx, +0xC owned — §7d's
   "ammo, price, category, item_idx" word map was off by one slot (its
   price/category/item_idx sit at +6/+8/+0xA). The chassis table
   0x4deafc + type·0x1C carries TWO 0xE rows with the same layout; its
   +2 word = the SHIELD-CHARGE count for equipment chassis (Part B) and
   its +6 = the refund price.
5. **Exit + the MP SHOP SYNC** [verified, 0x442ae2..0x442c3e]: fade-out
   (memset the palette 0x302, FUN_0041cbf0(pal, 0xa)), FUN_0041fa3f(0),
   FUN_0041d714(0x90); **FUN_00449c94(4, 0x4e43e0)** — appends the
   type-4 SHOP-LOADOUT COMMAND record (the 63-B = 7×9 staging struct at
   0x4e43e0, consumed by MissionShell 0x44853e + the save path 0x4475fd;
   callee-fills from the current table [inferred from call order]); then
   **the player walk bounded by [0x46cbe0]** (the COMMAND-record count /
   MP robot-count override, D89 — the queue's "0x46cbe0 command-count
   read = MP shop sync" hypothesis CONFIRMED): for each player p <
   [0x46cbe0], read the 7 (name, ammo) word pairs from the 0x80-stride
   record at 0x4dd4a0 + p·0x80 (first byte skipped) and write them into
   THAT player's table row 0x4de664 + p·0x62 (+g·0xE / +g·0xE+2) — every
   player's loadout is exchanged through the COMMAND ring and mirrored
   per-type. Then the lockout normalize: for i ∈ {0,4,8,0xC}, the three
   columns 0x46cd48+i / 0x46cd5c+i / 0x46cd70+i == 2 → 1; input flush;
   ret 0. The ABORT path ([0x4edb50] ≠ 0, set by ESC in the input layer)
   → flush + ret 1 (= GameMain's quit outcome, GAMETHREAD case 2).

PART B — **the robots() extra-phase + timer-field semantics + the state-1
producers** (§9 item 3) [verified]:
1. **Phase-0 PRE-PASS** (FUN_0040b9f6 head, `test eax,eax; jne` — runs
   once per frame for phase 0, over ALL robots): w@+0x32 ≠ 0 → −1;
   w@+0x34 ≠ 0 → −1; d@+0xA4 ≠ 0 → −1; d@+0x88 ≠ 0 → −2 clamp 0; and the
   **flash block**: while d@+0xA0 ≠ 0 → **d@+0x88 := 0x2710 (10000 — the
   hit-flash INVULNERABILITY shield)** + d@+0xA0−−, and when the robot's
   type word@+0x2A == [0x4edb90] (the player's own robot) the palette
   ladder runs: +0xA0 was 0xC8 → FUN_0041cbf0(0x4de13c, 5), was 0xC4 →
   FUN_0041cbf0(0x4dde38, 0xF), then the alternating-value ladder
   ({0..2},{5,6},{8,9},{0xE..0x12},{0x1A..0x1F},{0x22..0x27},
   {0x2A..0x30}) → FUN_004258d0([0x4edbf8]) — the strobing damage-flash
   the §7i dither family draws under.
2. **Field identities pinned** (completing §3):
   - **+0x32 (0x4c6a16) = the BURN/SCORCH cooldown**: producer
     FUN_004100b7 @0x4103e3 (`[idx·0x15·8 + 0x4c6a16] := 0x64`, gated
     [+0x32]==0 @0x41036e) — each scorch-lane hit (robots phase 1:
     byte@0x4796d4+tile·0x1E ≠ 0 → FUN_004100b7(idx, 0x14)) sets 100;
     decays 1/frame in the pre-pass ⇒ scorched tiles re-burn every ~100
     frames. The OFF-scorch branch decays w@+0x30 by 0xA (signed, clamp
     0) — +0x30 is the burn-damage accumulator paired with the lane.
   - **+0x34 (0x4c6a18) = the ALARM cooldown**: producer FUN_0040e230
     @0x40e386 (the §7g alarm SFX path sets it with the counter); decays
     1/frame; readers = the sidebar draw FUN_00408403 (dword@+0x34 =
     {alarm, order-group-word0}) ×2 + FUN_0040aa71.
   - **d@+0x88 (0x4c6a6c) = SHIELD POINTS, full machine**: −2/frame
     (pre-pass, clamp 0); 0x20 (32) per consumed charge or on state-3
     (FUN_0040e230 @0x40e27a); **0x2710 while the +0xA0 flash runs**;
     renderer reader 0x403ef4 (the shield glow).
   - **d@+0x8C (0x4c6a70) = SHIELD CHARGES**: producer = spawn
     FUN_0040cca0 @0x40cfbd — word@chassis_row+2 via the 5-slot jump
     table 0x40cc8c gated on the chassis NAME word ∈ 0x2A..0x2E (the
     five EQUIPMENT chassis, §7d.5) — i.e. the chassis "+2" word IS the
     charge count the shop stages as "ammo"; consumer = FUN_0040e230
     @0x40e2a4 (hit ∧ charges ≠ 0 ∧ shield == 0 → charge−− ∧ shield :=
     0x20); the MP respawn re-stages it (0x40e837 region). The §7g
     "gate d@+0x8C==0 ∨ d@+0x88≠0" gloss resolves to exactly this
     charge-consume test.
   - **d@+0xA4 (0x4c6a88) = the alarm COUNTER** (§7g's += 3 applier):
     decays 1/frame in the pre-pass — the D90 question ("EXW 7g.1
     documents no decay") is CLOSED: EXW DOES decay it, once per frame,
     phase 0 only. (The queue's "0x4c6a8c" tail has ZERO sites — the
     intended pair was +0x88/+0x8C.)
   - **d@+0x70 (0x4c6a54) = the IDLE-TIME BOMBARDMENT ARM counter**
     (§3/§5's "deploy-delay"/"reinforcement delay" gloss CORRECTED
     by §7j.54 — nothing in the family stages an arrival): cleared at 0x40c003
     (the states-3/5 block — ORDERING the robot resets the
     idle timer) and 0x40c271 (the arm tail); incremented at 0x40c16e gated
     state==0 ∧ phase==0 vs dword[0x454ee8 + 4·[0x46cbf8]] (the
     difficulty-scaled idle table {400, 300, 200, 5000≈never}; SP
     accumulates ONLY the SELECTED robot — idx == [0x46cbd4]+
     [0x46cbdc], 0x40c0fc..0x40c12c — MP every idle robot); at threshold ∧ zone ∉ {1,7} ∧
     **[0x4de658] == 0** ∧ mode ≠ 2 → the aerial-BOMBARDMENT salvo: SFX
     0xC/0xD/0xE per squad slot (FUN_004239ef) + blink-cursor
     [0x4dc5d0] := slot+1 + **[0x4de658] := 0x80** + the 8-jittered-
     SHELL scatter into 0x4ea238 (the falling salvo — resolver, draw
     and the D125 arbitration in §7j.54). NEW pin: **[0x4de658]** = the
     salvo COOLDOWN latch (the dword 0xC below the weapon-table
     base 0x4de664; 0x80 while a salvo runs) — full census §7j.54.
     [§7j.53 CONTENT NOTE → ARBITRATED §7j.54: the posted
     pair's corpus text is "DANGER - UNIT n TARGETTED FOR" +
     "IMMINENT AERIAL BOMBARDMENT" (all six LANGUAGE.* files) —
     and the resolver CONFIRMS the announcement: each falling
     shell impacts into NINE 5000-damage kill-anything script
     blasts over a 3×3 tile patch centered on the targetted
     robot — an OFFENSIVE bombardment, NOT a reinforcement
     arrival. The §7g.5/§7f.6 "reinforcement ARRIVAL" reading
     is RETIRED.]
3. **Extra-phase semantics (phases 4/5)**: the §5 gate expression is
   re-verified exact — body runs only while +0x80 (the drop_countdown,
   D90's raw +0x80) > phase·0x20 (128/160) — and +0x80 decrements every
   sub-tick when ≠ 0 (0x40bcb2). The tile-0x7d3 gate is CORRECTED: on a
   0x7d3 tile the body runs only while phase ≤ (+0x80 == 0 ? 2 : 4)
   (0x40bc00..0x40bc17) — §5's flat "0x7d3 gates phase skips" missed the
   countdown-dependent bound.
4. **The state-1 producers — COMPLETE word-write census of +0x0C**
   (0x4c69f0): 0x40a37b (FUN_00409138 COMMAND bit0: := 1 + stop :=
   0xF4240 — the ONLY state-1 writer), 0x40bd0f (:= 0, state-6 expiry),
   0x40be52 (:= 3, arrive 4→3), 0x40be80 (:= 0, arrive 1→0), 0x40c0a2
   (:= 4, the beacon/order consumption), 0x40c587 (robot_move freeze :=
   0), 0x40e858 (FUN_0040e230 MP respawn), 0x41fdbd (extraction sweep :=
   5), 0x41ff67 (pod landing := 6), 0x4203f4 (elevator-ride completion :=
   0), 0x424853 (beacon armer := 3), 0x433a31+ ×6 (elevator rides := 2).
   **There is NO patrol semantics**: state 1 = the COMMAND-ARMED move
   (a network-select record auto-arming move-to-target) — in SP nothing
   produces it, which is why the S6 walk needed the COMMAND-inject seam
   (D112) and why E models it only through `command` records.

Engine consequence: NONE this unit (docs-only). The P4 slice already
models the command-bit0 arm (S6) and the shield family host-side; the NEW
facts for any future shop/mission-boundary engine work are the money
floor, the catalog grammar + immediate data range, the lockout array, the
MP loadout sync via the type-4 COMMAND record, and the +0x8C charge
machine's chassis-row source.

## 7j.46. THE COMPLETE FUN_00433980 PER-ZONE CASE TABLE + THE FUN_00424a6f/LANGUAGE.* MESSAGE SYSTEM (2026-08-23, worker 0c2df9b4 claim 2, D117; objdump-only — targeted `objdump -d/-s` windows on game-data/BEDLAM/BEDLAM.EXW (READ-ONLY; manifest clean before AND after) + read-only DGROUP string probes + read-only LANGUAGE.ENG/EDITOR corpus probes; no Ghidra run, no corpus write)

Closes the 7j.19 item-6 residual (the full per-zone case table) + the
queue's FUN_00424a6f message-table unit. Method: clean re-disassembly of
0x426000..0x43a000 (the flat exw-text-objdump.txt misparses the table
farm 0x43301c..0x433963 — jump-table DATA decoded as code), then a
static cascade walker (Watcom binary-search switch decode: `cmp eax,K;
jb T` => T covers (prev,K-1], a `jbe` in the SAME flag shadow => ==K, a
bare `jbe` => (prev,K], plus `jmp [eax*4+base]` tables with a `cmp eax,N;
ja` bound). Every leaf action hand-verified against raw dumps (spot
checks at 0x433a14, 0x433c8e, 0x433cbc, 0x433e89, 0x43475f, 0x438927,
0x439000).

### 1. The dispatch STRUCTURE [verified]

```
FUN_00433980(eax = robot idx)            ; sole caller FUN_0040b9f6 @0x40bd58
  push ebx,ecx,edx,esi,edi,ebp           ; 6-save prologue
  eax = [0x4edd8c] - 1                   ; zone cell A..G -> 0..6
  if > 6 -> ret                          ; shared 6-pop epilogue 0x42602f
  jmp [eax*4 + 0x433964]                 ; THE zone table (7 entries):
    A(1)->0x43399f  B(2)->0x434058  C(3)->0x435bda  D(4)->0x4386c5
    E(5)->0x432c8e  F(6)->0x439323  G(7)->0x439ae2
```
Each zone entry gates on **mode [0x4edb88]** (0 SP / 1 Coop / 2 Head2Head,
§7j.33) then on **mission [0x4edd88]** (within-zone 1..5(9)):
- SP/Coop: a mission cascade or a 5-entry `jmp [eax*4+T]` table
  (A: ==1 cascade only; B: table 0x4331d0; C: cascade; D: table
  0x433650; F: table 0x433950; G: ==1 only);
- H2H: missions 1..2 get their own probe+case blocks (B 0x434f90/
  0x43548f, C 0x437688/0x43805f, D 0x4387a1-family, F 0x4393ad/0x439434;
  zone A H2H runs the probe at 0x4339ad and DISCARDS the result — MP
  training-camp pads are inert, only the revisit latch updates).
Each mission block reads the robot x/y/z (bank **0x4c69e4, stride
0xA8**: +0 x, +4 y, +8 z), calls the pad-tile probe **FUN_00422e5e
(x>>8, y>>8, z)** and dispatches on the returned .PAD slot id.

### 2. The four action families [verified]

- **ELEVATOR/TELEPORT RIDE** — the "dword tables 0x4dcdbc..0x4dd330"
  of 7j.19/7j.21 are ONE **ride-record bank, base 0x4dcdbc, stride
  0x24**: record = {+0x00 dest tile-x, +0x04 dest tile-y, +0x18 the
  countdown LATCH (:= 10 at arm), +0x1C the RIDER-IN-USE gate (-1 =
  free; := robot idx at arm)}. 16 records pinned live (gates
  0x4dcdd8, 0x4dcdfc, 0x4dce20, 0x4dce44, 0x4dce68, 0x4dce8c,
  0x4dceb0, 0x4dced4, 0x4dcef8, 0x4dcf1c, 0x4dcf40, 0x4dcf64,
  0x4dcf88, 0x4dcfac, 0x4dcfd0, 0x4dcff4 = base+0x24·k+0x1C). Arm:
  gate==-1 else ret; robot state w@+0x0C := 2 (in transit),
  d@+0x74 := 0, **d@+0x84 := the arrival platform 0..0xE** (per case),
  order 0x46cc30[idx] := -1, 0x46cc60[idx] := -1, pos.x :=
  dest_x·0x2000+0x1000, latch := 10, gate := idx; the shared tail
  0x43475f stamps pos.y := dest_y·0x2000+0x1000. (The 7j.21
  "marker x/y·0x2000+0x1000 from dwords 0x4dcdbc.." gloss = these
  record words; the staging blocks that FILL them from .PAD slot
  records 0x4e44f8 are the 0x426058-family — e.g. it reads slot
  records and stamps 0x4dcdbc/c0/c4.. 0x42606c..0x426105.)
- **DOOR** — FUN_004223b8(rect, 1|2) over rect ids 0..0x25 of the
  45×0x10 rect bank 0x4dcae8 (7j.34-corrected grammar). Multi-door
  cases (up to 4 rects per pad) share one thunk (e.g. 0x43890c,
  0x438a05, 0x4394ae).
- **EXTRACTION BEACON** — FUN_004247b5 (sole call site 0x433cfb in the
  0x433cbc block). **21 SP beacon slots** (cross-checks 7j.20's "~25
  (zone,slot) pairs" which counted H2H variants too): zone A M1 0x10;
  zone B M1..M5 0x18/0x04/0x01/0x00/0x08; zone C M1..M5
  0x0A/0x0E/0x15/0x16/0x3D; zone D M1..M5 0x08/0x07/0x0F/0x10/0x09;
  zone F M1..M5 0x12/0x11/0x00/0x15/0x1A.
- **EXIT-PAD ACTIVATION** — FUN_0041fa51(slot) (7j.18) — zone A's
  7j.19 "case 0x1B @0x43900e" gloss corrected: exit activations are
  NOT one case — they are DOOR+EXIT pairs in zone F (M1 slot 8, M2
  slots 0xC/0x12, M3 slots 0x10/0x11, M4 slots 0x12/0x13, M5 slots
  5/7/0x19/0x1B) and zone G (M1 slot 0), via thunks 0x439000
  (DOOR(0,1)+EXIT), 0x439018, 0x438f97, 0x4395ab, 0x439664, 0x439673,
  0x4396df, 0x4396ee, 0x43975c, 0x439775. The activated pad id =
  the .PAD slot id at the trigger (ebx carries it into FUN_0041fa51).

### 3. ZONE A / MISSION 1 — the full case table [verified]

```
slot 0x00 RIDE gate 0x4dcdd8          slot 0x0B RIDE 0x4dce20 plat 2
slot 0x01 DOOR(0,2)                   slot 0x0C RIDE 0x4dce44 plat 3
slot 0x02 DOOR(1,1)                   slot 0x0D RIDE 0x4dce68 plat 4
slot 0x03 DOOR(2,1)+DOOR(3,2)         slot 0x0E RIDE 0x4dce8c plat 5
slot 0x04..0x07 DOOR(4,2)             slot 0x0F RIDE 0x4dceb0 plat 6
slot 0x08 DOOR(5,1)                   slot 0x10 BEACON
slot 0x09 DOOR(6,2)                   slot 0x11..0x15 MSG(0)
slot 0x0A RIDE gate 0x4dcdfc plat 1   slot 0x16..0x1D MSG(1)
                                      slot 0x1E..0x2B MSG(2)
                                      slot 0x2C..0x33 MSG(3)
                                      slot 0x34..0x38 MSG(4)
                                      slot 0x39..0x3D MSG(5)
                                      slot 0x3E..0x42 MSG(6)
                                      slot 0x43..0x48 MSG(7)
                                      slot 0x49..0x4B MSG(8)
                                      slot 0x4C..0x4F MSG(9)
                                      slot 0x50..0x51 MSG(0xA)
                                      slot 0x52..0x55 MSG(0xB)
                                      slot 0x56..0x66 MSG(0xC)
                                      slot 0x67..0x69 MSG(0xD)
                                      slot 0x6A..0x71 MSG(0xE)
```
(Message slots are RANGES — 97 .PAD slots, 0x11 through 0x71 inclusive, carry the 15 hints.)

### 4. Zones B..G SP census (compact; rides by gate k =
base 0x4dcdbc+0x24·k; the FULL generated table is §8-bis below) [verified]

- **B** M1(≤0x18): rides 0..7, doors 5/8/7 + door-6 ×15 slots +
  beacon 0x18; M2(≤4): DOOR(0,2) ride0 ride1 DOOR(1,2) beacon;
  M3(≤4): 4-door burst, beacon 1, DOOR(4,1), 4-door burst;
  M4(≤0x11): beacon 0, rides 0/1/2/3/4, doors; M5(≤9): doors +
  rides 0/4 + beacon 8.
- **C** M1(≤0xA): doors + rides 0/1 + beacon 0xA; M2(≤0xF): door
  clusters + rides 0/1/2/3/4 + beacon 0xE; M3(≤0x17): rides 0/1/2/3 +
  doors + beacon 0x15 + rides 4/5; M4(≤0x17): rides 0..0xE (the full
  15-record bank!) + doors + beacon 0x16 + ride 0xF; M5(≤0x3D): the
  door-heavy finale (rects 5..0x25, many multi-slot) + rides 0/1/3/4 +
  beacon 0x3D.
- **D** M1(≤8): 4×4-door bursts + rides 0/1/2/3 + beacon 8;
  M2(≤7): doors + rides 0/1/2 + beacon 7; M3(≤0xF): doors +
  rides 0..3 + beacon 0xF; M4(≤0x10): doors + rides 0/1 +
  beacon 0x10; M5(≤9): doors + rides 0/1/5 + beacon 9.
- **E** — **VERIFIED NEGATIVE**: the zone entry 0x432c8e lands in the
  0x430b27..0x433030 "zone overlay" word-stamp family (restages the
  rect bank 0x4dcae8..0x4dcc22 + ride-dest tables), then
  `cmp [0x4edd88],1; je 0x432f8b` (M1 restamps rect 0) and EXITS —
  **no probe, no slot cases, no beacon, no messages on the zone-E
  path**. Zone E missions ride the shared machinery with NO .PAD
  triggers of their own. [hypothesis on the oddity: the overlay
  blocks exit via the 5-pop thunk 0x426030 (0x42602f minus `pop ebp`)
  while FUN_00433980's prologue pushes 6 — the overlay family is
  compiler-shared with a 5-save sibling function; if the zone-E path
  ever ran, the mis-pop would corrupt the caller frame, so the SP
  zone-E entry is most plausibly never exercised — every other zone
  path uses the 6-pop exit. Recorded as a quirk, not a blocker: no
  E-side seam depends on it.]
- **F** M1(≤0x13): rides 0..7 + DOOR(10,1)+EXIT(10)/DOOR(9,2)/
  DOOR(8,1) + 4-door bursts + beacon 0x12 + DOOR(20,2); M2(≤0x12):
  rides 0..0xD + EXIT pairs + beacon 0x11; M3(≤0x12): 5-door burst +
  rides + EXIT pairs + beacon 0; M4(≤0x15): rides 0..0xD + EXIT
  pairs + beacon 0x15; M5(≤0x1B): doors + EXIT pairs + rides 2/3 +
  beacon 0x1A.
- **G** M1: slot 0 → DOOR(0,1)+EXIT(0), slot 1 → DOOR(1,2). No
  beacon (the G campaign finale extracts by other means).
- **H2H variants** (missions 1..2): rides-only tables (B M1 ≤0x18,
  B M2 ≤0x14, C M1 ≤0x26, C M2 ≤0x1B, F M1 ≤0xE, F M2 ≤6) + D's
  0x4387a1 family — no doors/beacons/messages in MP pad triggers
  (the MP geometry changes are the ride/dest staging only).

### 5. FUN_00424a6f = the ZONE-A BOOT_CAMP MESSAGE SHOWER [verified]

Sole caller 0x433d07 (the zone-A M1 MSG cases). Full decode:
```
FUN_00424a6f(eax = msg id):
  if ([0x4edb88] != 0) return        ; SP ONLY — no message boxes in MP
  if (word[0x4eb5f8 + 2*id] != 0) return   ; per-id show-once latch
  word[0x4eb5f8 + 2*id] := 1               ; ARM latch
  FUN_0043a48e(_DAT_004edfd0=TEXTBOX1.RAW, -1, 0, 2)   ; the box SFX
  name = "BOOT_CAMP_" (0x458ca7) + sprintf("%03i", id) (fmt 0x458cb2)
  FUN_00424679(name)   ; section finder (below)
  parse until byte ']' : per line {dword cursor-ptr, char text[0x42]}
  (0x46-stride records) ; authored line widths measured via
  FUN_00402a12(TINYFONT 0x46cdb0, c-0x21)+1, space=3
  window struct 0x4eaab8 := {x = 0xF0 - maxw/2 (centered), 0xC8=200,
    w = (maxw+4)/5+2, h = (lines-1)*9+0xA)/7+2}
  clear the 3600-B display bank 0x4e8818 ; per-line FUN_0043e2eb
  draw box frame FUN_00424e9f(...) ; timer [0x4eaac0] := 0xFDE8 (65000)
```
**The string table is a FILE, not DGROUP**: FUN_00424679 scans the
language blob for `[NAME]` markers — buffer [0x46cbb4] = the boot
alloc 0x13C68 (0x41d64d) filled ONCE at 0x41c1f5/0x41c1fb
(FUN_0041cc7f) with **LANGUAGE.{ENG,GER,SPA,FRE,ITL,DCH}** (selector
switch on [0x4eba1c] at 0x41c1e3, paths 0x457a70..0x457ab1; ENG
69,485 B fits the 81,000-B alloc; miss => "Could not find a
menu_heading" 0x458c5f + FUN_00420100/exit). LANGUAGE.ENG carries
**421 `[NAME]` sections** (`[NAME].../]` grammar, the same 0x5D
terminator the parser tests): BOOT_CAMP_000..014 (the 15 zone-A ids —
exactly the §3 MSG census), OBJECTIVE_{A..F}{1..5}_{00..},
MARKER_A..F, OVERVIEW_A..F, DM_OVERVIEW_B..F, CREDIT_1..13,
MENU_ITEMS, WARNINGS (the OBJECTIVE/MARKER/OVERVIEW families feed the
briefing readers at 0x41c2e1/0x41c309/0x447111/0x43ddd1/0x43e3d4/
0x43e52a — the FUN_00424679 caller census — NOT the pad dispatcher).

### 6. The latch/timer semantics [verified]

- **latch 0x4eb5f8+2·id** := 1 at show — show-once per mission (the
  MissionShell boot resets the family). NO id ever writes a countdown
  there (the 7j.19 "per-id latch" gloss stands, value pinned to 1).
- **timer [0x4eaac0]** := 0xFDE8 at show; **FUN_00425010 = the
  per-frame message-box ticker/drawer** (sole caller MissionShell
  0x448381): if timer==0 ret; timer−−; draw the box (window struct +
  0x46cdb0 glyphs). So the box self-expires after 65000 frames ≈ 18
  min — practically until dismissed.
- **DISMISSAL = a COMMAND**: inside FUN_00409138 (the COMMAND
  consumer, §7j.37) the bit0-SELECT arm site 0x40a2bc checks
  `[0x4eaac0] < 0xFDE0` (i.e. more than 8 timer decrements have elapsed) ⇒
  timer := 0, and the sibling site 0x40a396 (threshold 0xFDD4, more than 20 timer decrements) clears likewise — any player select/fire command dismisses
  the on-screen hint after a minimum display. A third reader 0x40c570
  (`cmp 0x4eaac0,0; je`) gates the state-0 robot-write at 0x40c587 —
  the box holds the freeze write while visible. MissionShell
  0x44790f resets the timer at mission start.
- **Producers' "msgs 9/10/0xB/0x1C..0x21/0x26-0x29" CLARIFIED**:
  those ids (cited at the pod spawner/POI loops in 7j.17/7j.19)
  are FUN_004239ef RADIO-WARNING ids — spoken WARNINGS lines +
  on-screen text both (§7j.53 SUPERSEDES this section's earlier
  "SFX ids, not text messages" framing; they were never
  BOOT_CAMP hint-box texts). The text-message producer set of
  the 15 zone-A BOOT_CAMP cases of §3 (the hint-box system,
  [0x4eaac0]) is a SEPARATE channel and stands unchanged.

### 7. Engine/differ consequences

Docs-only unit (D117): no engine change, no new watch rows needed for
S0..S8 (the latch/timer/window cells are SP-UI presentation; the
pad-case semantics are already modeled through the existing seams —
the beacon armer (S6), the .PAD probe, and the exit activator). For
any future live-message expectation: a live zone-A/M1 capture walking
a hint pad should show [0x4eaac0] := 0xFDE8 + the per-id latch bit —
candidate additive watch rows if ever needed (not in the first
golden).

### 8-bis. The FULL generated case table [verified, static decode]

Complete static decode, all zones/modes (slot `HHh (lo..hi) ->
handler : actions`; RIDE gate k = ride record 0x4dcdbc+0x24·k whose
in-use word is +0x1C; plat = the +0x84 arrival-platform stamp; dest =
the record's +0x00/+0x04 tile words; BEACON = FUN_004247b5;
DOOR = FUN_004223b8(rect,state); EXIT = FUN_0041fa51(.PAD slot);
MSG = FUN_00424a6f(id)):

```
========================================================================
ZONE A  entry 0x43399f
  H2H @0x4339fa (mode==2 gate; missions 1..2 -> probe blocks)
  M1 @0x433d5e dispatch@0x433da6 slots<=0x71:
    slot 00h (0) -> 0x433a14 : RIDE 0x4dcdd8 dest=0x4dcdd8,0x4dcdbc) latch=0x4dcdd4]
    slot 01h (1) -> 0x433acd : DOOR(rect=0,state=2)
    slot 02h (2) -> 0x43402c : DOOR(rect=1,state=1)
    slot 03h (3) -> 0x433ad9 : DOOR(rect=2,state=1) + DOOR(rect=3,state=2)
    slot 06h (4..6) -> 0x438927 : DOOR(rect=4,state=2)
    slot 07h (7) -> 0x438927 : DOOR(rect=4,state=2)
    slot 08h (8) -> 0x433af7 : DOOR(rect=5,state=1)
    slot 09h (9) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 0Ah (10) -> 0x433f53 : RIDE 0x4dcdfc plat=1 dest=0x4dcdfc,0x4dcde0) latch=0x4dcdf8]
    slot 0Bh (11) -> 0x433b01 : RIDE 0x4dce20 plat=2 dest=0x4dce20,0x4dce04) latch=0x4dce1c]
    slot 0Ch (12) -> 0x433bbe : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 0Dh (13) -> 0x433c62 : RIDE 0x4dce68]
    slot 0Eh (14) -> 0x433e89 : RIDE 0x4dce8c plat=5 dest=0x4dce8c,0x4dce70) latch=0x4dce88]
    slot 0Fh (15) -> 0x433c8e : RIDE 0x4dceb0]
    slot 10h (16) -> 0x433cc8 : BEACON-ARM
    slot 15h (17..21) -> 0x433d05 : MSG(0) + call_424a6f
    slot 1Dh (22..29) -> 0x433e52 : MSG(1) + call_424a6f
    slot 2Bh (30..43) -> 0x433d11 : MSG(2) + call_424a6f
    slot 33h (44..51) -> 0x433d18 : MSG(3) + call_424a6f
    slot 38h (52..56) -> 0x433d1f : MSG(4) + call_424a6f
    slot 3Dh (57..61) -> 0x433e36 : MSG(5) + call_424a6f
    slot 42h (62..66) -> 0x433d26 : MSG(6) + call_424a6f
    slot 48h (67..72) -> 0x433d2d : MSG(7) + call_424a6f
    slot 4Bh (73..75) -> 0x433d34 : MSG(8) + call_424a6f
    slot 4Fh (76..79) -> 0x433e0c : MSG(9) + call_424a6f
    slot 51h (80..81) -> 0x433d3b : MSG(10) + call_424a6f
    slot 55h (82..85) -> 0x433d42 : MSG(11) + call_424a6f
    slot 66h (86..102) -> 0x433d49 : MSG(12) + call_424a6f
    slot 69h (103..105) -> 0x433d50 : MSG(13) + call_424a6f
    slot 71h (106..113) -> 0x433d57 : MSG(14) + call_424a6f
  H2H @0x4339fa (mode==2 gate; missions 1..2 -> probe blocks)
========================================================================
ZONE B  entry 0x434058
  M1 @0x43552f dispatch@0x435579 slots<=0x18:
    slot 00h (0) -> 0x435507 : DOOR(rect=5,state=1) + DOOR(rect=8,state=1) + DOOR(rect=7,state=1)
    slot 01h (1) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 02h (2) -> 0x438945 : DOOR(rect=6,state=1)
    slot 03h (3) -> 0x438945 : DOOR(rect=6,state=1)
    slot 04h (4) -> 0x438945 : DOOR(rect=6,state=1)
    slot 05h (5) -> 0x438945 : DOOR(rect=6,state=1)
    slot 06h (6) -> 0x438945 : DOOR(rect=6,state=1)
    slot 07h (7) -> 0x438945 : DOOR(rect=6,state=1)
    slot 08h (8) -> 0x438945 : DOOR(rect=6,state=1)
    slot 09h (9) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Ah (10) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Bh (11) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Ch (12) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Dh (13) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Eh (14) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Fh (15) -> 0x438945 : DOOR(rect=6,state=1)
    slot 10h (16) -> 0x438945 : DOOR(rect=6,state=1)
    slot 11h (17) -> 0x438945 : DOOR(rect=6,state=1)
    slot 12h (18) -> 0x438945 : DOOR(rect=6,state=1)
    slot 13h (19) -> 0x438945 : DOOR(rect=6,state=1)
    slot 14h (20) -> 0x438945 : DOOR(rect=6,state=1)
    slot 15h (21) -> 0x438945 : DOOR(rect=6,state=1)
    slot 16h (22) -> 0x438945 : DOOR(rect=6,state=1)
    slot 17h (23) -> 0x438945 : DOOR(rect=6,state=1)
    slot 18h (24) -> 0x433cbc : BEACON-ARM
  M2 @0x43558a dispatch@0x4355d4 slots<=0x4:
    slot 00h (0) -> 0x433acd : DOOR(rect=0,state=2)
    slot 01h (1) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 02h (2) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 03h (3) -> 0x438430 : DOOR(rect=1,state=2)
    slot 04h (4) -> 0x433cbc : BEACON-ARM
  M3 @0x43565c dispatch@0x4356a6 slots<=0x4:
    slot 00h (0) -> 0x4355e5 : DOOR(rect=0,state=1) + DOOR(rect=0,state=1) + DOOR(rect=2,state=1) + DOOR(rect=3,state=1)
    slot 01h (1) -> 0x433cbc : BEACON-ARM
    slot 02h (2) -> 0x435616 : DOOR(rect=4,state=1)
    slot 03h (3) -> 0x435620 : DOOR(rect=5,state=2) + DOOR(rect=6,state=2) + DOOR(rect=7,state=2) + DOOR(rect=8,state=2)
    slot 04h (4) -> 0x435620 : DOOR(rect=5,state=2) + DOOR(rect=6,state=2) + DOOR(rect=7,state=2) + DOOR(rect=8,state=2)
  M4 @0x4357c8 dispatch@0x435812 slots<=0x11:
    slot 00h (0) -> 0x433cbc : BEACON-ARM
    slot 01h (1) -> 0x4356b7 : RIDE 0x4dcdd8]
    slot 02h (2) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 03h (3) -> 0x4341f4 : RIDE 0x4dce20]
    slot 04h (4) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 05h (5) -> 0x4356e3 : DOOR(rect=0,state=1)
    slot 06h (6) -> 0x433acd : DOOR(rect=0,state=2)
    slot 07h (7) -> 0x438945 : DOOR(rect=6,state=1)
    slot 08h (8) -> 0x438945 : DOOR(rect=6,state=1)
    slot 09h (9) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Ah (10) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Bh (11) -> 0x438945 : DOOR(rect=6,state=1)
    slot 0Ch (12) -> 0x4356ed : RIDE 0x4dce68]
    slot 0Dh (13) -> 0x434307 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 0Eh (14) -> 0x435719 : DOOR(rect=7,state=2) + DOOR(rect=8,state=2) + DOOR(rect=9,state=2) + DOOR(rect=10,state=2)
    slot 0Fh (15) -> 0x435755 : DOOR(rect=11,state=1) + DOOR(rect=12,state=1) + DOOR(rect=13,state=1) + DOOR(rect=14,state=1)
    slot 10h (16) -> 0x4343c4 : RIDE 0x4dceb0]
    slot 11h (17) -> 0x43578c : DOOR(rect=19,state=2) + DOOR(rect=20,state=2) + DOOR(rect=21,state=2) + DOOR(rect=22,state=2)
  M5 @0x435b81 dispatch@0x435bc9 slots<=0x9:
    slot 00h (0) -> 0x433acd : DOOR(rect=0,state=2)
    slot 01h (1) -> 0x438430 : DOOR(rect=1,state=2)
    slot 02h (2) -> 0x435823 : RIDE 0x4dcdd8]
    slot 03h (3) -> 0x4358a9 : <no-action>
    slot 04h (4) -> 0x435943 : <no-action>
    slot 05h (5) -> 0x4359fe : <no-action>
    slot 06h (6) -> 0x435ab9 : RIDE 0x4dce68]
    slot 07h (7) -> 0x435b77 : DOOR(rect=2,state=2)
    slot 08h (8) -> 0x433cbc : BEACON-ARM
    slot 09h (9) -> 0x4392b7 : DOOR(rect=6,state=2)
  H2H @0x4354ea (mode==2 gate; missions 1..2 -> probe blocks)
  H2H-M1 @0x434f90 dispatch@0x434fd8 slots<=0x18:
    slot 00h (0) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 01h (1) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 02h (2) -> 0x4341f4 : RIDE 0x4dce20]
    slot 03h (3) -> 0x43421e : RIDE 0x4dce44]
    slot 04h (4) -> 0x43424a : RIDE 0x4dce68 plat=4 dest=0x4dce4c,0x4dce50)]
    slot 05h (5) -> 0x434307 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 06h (6) -> 0x4343c4 : RIDE 0x4dceb0]
    slot 07h (7) -> 0x4343ee : RIDE 0x4dced4]
    slot 08h (8) -> 0x43441a : RIDE 0x4dcef8 plat=8 dest=0x4dcedc,0x4dcee0)]
    slot 09h (9) -> 0x4344d7 : RIDE 0x4dcf1c plat=9 dest=0x4dcf00,0x4dcf04)]
    slot 0Ah (10) -> 0x434594 : RIDE 0x4dcf40]
    slot 0Bh (11) -> 0x4345be : RIDE 0x4dcf64]
    slot 0Ch (12) -> 0x4345ea : RIDE 0x4dcf88 plat=c dest=0x4dcf6c,0x4dcf70)]
    slot 0Dh (13) -> 0x4346a7 : RIDE 0x4dcfac plat=d dest=0x4dcf90,0x4dcf94)]
    slot 0Eh (14) -> 0x43476b : RIDE 0x4dcfd0 plat=e dest=0x4dcfb4,0x4dcfb8)]
    slot 0Fh (15) -> 0x434828 : RIDE 0x4dcff4 plat=f]
    slot 10h (16) -> 0x4348e7 : RIDE 0x4dd018 plat=10]
    slot 11h (17) -> 0x4349a4 : RIDE 0x4dd03c plat=11]
    slot 12h (18) -> 0x434a61 : RIDE 0x4dd060 plat=12]
    slot 13h (19) -> 0x434b1e : RIDE 0x4dd084 plat=13]
    slot 14h (20) -> 0x434bdd : RIDE 0x4dd0a8 plat=14]
    slot 15h (21) -> 0x434c9a : RIDE 0x4dd0cc plat=15]
    slot 16h (22) -> 0x434d57 : RIDE 0x4dd0f0 plat=16]
    slot 17h (23) -> 0x434e14 : RIDE 0x4dd114 plat=17]
    slot 18h (24) -> 0x434ed3 : RIDE 0x4dd138 plat=18]
  H2H-M2 @0x43548f dispatch@0x4354d9 slots<=0x14:
    slot 00h (0) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 01h (1) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 02h (2) -> 0x434fe9 : RIDE 0x4dce20 plat=2]
    slot 03h (3) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 04h (4) -> 0x43424a : RIDE 0x4dce68 plat=4 dest=0x4dce4c,0x4dce50)]
    slot 05h (5) -> 0x434307 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 06h (6) -> 0x435165 : RIDE 0x4dceb0 plat=6]
    slot 07h (7) -> 0x435224 : RIDE 0x4dced4 plat=7 dest=0x4dceb8,0x4dcebc)]
    slot 08h (8) -> 0x43441a : RIDE 0x4dcef8 plat=8 dest=0x4dcedc,0x4dcee0)]
    slot 09h (9) -> 0x4344d7 : RIDE 0x4dcf1c plat=9 dest=0x4dcf00,0x4dcf04)]
    slot 0Ah (10) -> 0x4352e1 : RIDE 0x4dcf40]
    slot 0Bh (11) -> 0x43539c : RIDE 0x4dcf64 plat=b dest=0x4dcf48,0x4dcf4c)]
    slot 0Ch (12) -> 0x438621 : DOOR(rect=15,state=2)
    slot 0Dh (13) -> 0x438621 : DOOR(rect=15,state=2)
    slot 0Eh (14) -> 0x438621 : DOOR(rect=15,state=2)
    slot 0Fh (15) -> 0x435459 : DOOR(rect=15,state=1)
    slot 10h (16) -> 0x435459 : DOOR(rect=15,state=1)
    slot 11h (17) -> 0x435459 : DOOR(rect=15,state=1)
    slot 12h (18) -> 0x4345ea : RIDE 0x4dcf88 plat=c dest=0x4dcf6c,0x4dcf70)]
    slot 13h (19) -> 0x4346a7 : RIDE 0x4dcfac plat=d dest=0x4dcf90,0x4dcf94)]
    slot 14h (20) -> 0x435463 : RIDE 0x4dcfd0]
========================================================================
ZONE C  entry 0x435bda
  H2H @0x4380b8 (mode==2 gate; missions 1..2 -> probe blocks)
  M1 @0x438128 dispatch@0x438172 slots<=0xa:
    slot 00h (0) -> 0x4381ad : DOOR(rect=5,state=2)
    slot 01h (1) -> 0x433a14 : RIDE 0x4dcdd8 dest=0x4dcdd8,0x4dcdbc) latch=0x4dcdd4]
    slot 02h (2) -> 0x4380d6 : DOOR(rect=6,state=1) + DOOR(rect=7,state=2)
    slot 03h (3) -> 0x43890c : DOOR(rect=2,state=2) + DOOR(rect=3,state=2) + DOOR(rect=4,state=2)
    slot 04h (4) -> 0x433acd : DOOR(rect=0,state=2)
    slot 05h (5) -> 0x4380f4 : DOOR(rect=1,state=1) + DOOR(rect=9,state=1) + DOOR(rect=10,state=1) + DOOR(rect=11,state=1)
    slot 06h (6) -> 0x43890c : DOOR(rect=2,state=2) + DOOR(rect=3,state=2) + DOOR(rect=4,state=2)
    slot 07h (7) -> 0x4376e3 : RIDE 0x4dcdfc]
    slot 08h (8) -> 0x43890c : DOOR(rect=2,state=2) + DOOR(rect=3,state=2) + DOOR(rect=4,state=2)
    slot 09h (9) -> 0x43890c : DOOR(rect=2,state=2) + DOOR(rect=3,state=2) + DOOR(rect=4,state=2)
    slot 0Ah (10) -> 0x433cbc : BEACON-ARM
  M2 @0x4383c1 dispatch@0x438409 slots<=0xf:
    slot 00h (0) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 01h (1) -> 0x433acd : DOOR(rect=0,state=2)
    slot 02h (2) -> 0x438ad6 : DOOR(rect=11,state=2) + DOOR(rect=12,state=2)
    slot 03h (3) -> 0x438183 : DOOR(rect=2,state=2) + DOOR(rect=3,state=2) + DOOR(rect=4,state=2) + DOOR(rect=5,state=2)
    slot 04h (4) -> 0x4381bc : DOOR(rect=13,state=2) + DOOR(rect=14,state=2) + DOOR(rect=15,state=2) + DOOR(rect=16,state=2)
    slot 05h (5) -> 0x4381f8 : DOOR(rect=7,state=1) + DOOR(rect=8,state=1) + DOOR(rect=9,state=1) + DOOR(rect=10,state=1)
    slot 06h (6) -> 0x43822f : RIDE 0x4dcdd8]
    slot 07h (7) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 08h (8) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 09h (9) -> 0x4382e0 : DOOR(rect=18,state=2)
    slot 0Ah (10) -> 0x4382e0 : DOOR(rect=18,state=2)
    slot 0Bh (11) -> 0x433f53 : RIDE 0x4dcdfc plat=1 dest=0x4dcdfc,0x4dcde0) latch=0x4dcdf8]
    slot 0Ch (12) -> 0x435cd0 : RIDE 0x4dce20]
    slot 0Dh (13) -> 0x4382ef : RIDE 0x4dce44]
    slot 0Eh (14) -> 0x433cbc : BEACON-ARM
    slot 0Fh (15) -> 0x43831d : RIDE 0x4dce20 plat=4 dest=0x4dce4c,0x4dce50)]
  M3 @0x43843f dispatch@0x438489 slots<=0x17:
    slot 00h (0) -> 0x433a14 : RIDE 0x4dcdd8 dest=0x4dcdd8,0x4dcdbc) latch=0x4dcdd4]
    slot 01h (1) -> 0x433f53 : RIDE 0x4dcdfc plat=1 dest=0x4dcdfc,0x4dcde0) latch=0x4dcdf8]
    slot 02h (2) -> 0x433b01 : RIDE 0x4dce20 plat=2 dest=0x4dce20,0x4dce04) latch=0x4dce1c]
    slot 03h (3) -> 0x433bbe : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 04h (4) -> 0x43841a : DOOR(rect=9,state=2)
    slot 05h (5) -> 0x438424 : DOOR(rect=0,state=2) + DOOR(rect=1,state=2)
    slot 06h (6) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 07h (7) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 08h (8) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 09h (9) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 0Ah (10) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 0Bh (11) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 0Ch (12) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 0Dh (13) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 0Eh (14) -> 0x4392b7 : DOOR(rect=6,state=2)
    slot 0Fh (15) -> 0x43564d : DOOR(rect=8,state=2)
    slot 10h (16) -> 0x43564d : DOOR(rect=8,state=2)
    slot 11h (17) -> 0x4381ad : DOOR(rect=5,state=2)
    slot 12h (18) -> 0x4381ad : DOOR(rect=5,state=2)
    slot 13h (19) -> 0x43890c : DOOR(rect=2,state=2) + DOOR(rect=3,state=2) + DOOR(rect=4,state=2)
    slot 14h (20) -> 0x4380e5 : DOOR(rect=7,state=2)
    slot 15h (21) -> 0x433cbc : BEACON-ARM
    slot 16h (22) -> 0x433c62 : RIDE 0x4dce68]
    slot 17h (23) -> 0x433e89 : RIDE 0x4dce8c plat=5 dest=0x4dce8c,0x4dce70) latch=0x4dce88]
  M4 @0x4384a9 dispatch@0x4384f3 slots<=0x17:
    slot 00h (0) -> 0x433acd : DOOR(rect=0,state=2)
    slot 01h (1) -> 0x4356e3 : DOOR(rect=0,state=1)
    slot 02h (2) -> 0x435bfe : RIDE 0x4dcdd8]
    slot 03h (3) -> 0x435c2c : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 04h (4) -> 0x435cd0 : RIDE 0x4dce20]
    slot 05h (5) -> 0x435cfc : RIDE 0x4dce44 plat=3 dest=0x4dce44,0x4dce28) latch=0x4dce40]
    slot 06h (6) -> 0x435db7 : RIDE 0x4dce68 plat=4 dest=0x4dce68,0x4dce4c) latch=0x4dce64]
    slot 07h (7) -> 0x435e74 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 08h (8) -> 0x438430 : DOOR(rect=1,state=2)
    slot 09h (9) -> 0x435f18 : RIDE 0x4dceb0 plat=6 dest=0x4dceb0,0x4dce94) latch=0x4dceac]
    slot 0Ah (10) -> 0x435fd3 : RIDE 0x4dced4 plat=7 dest=0x4dced4,0x4dceb8) latch=0x4dced0]
    slot 0Bh (11) -> 0x43608e : RIDE 0x4dcef8 plat=8 dest=0x4dcef8,0x4dcedc) latch=0x4dcef4]
    slot 0Ch (12) -> 0x43614b : RIDE 0x4dcf1c plat=9 dest=0x4dcf00,0x4dcf04)]
    slot 0Dh (13) -> 0x4361ef : RIDE 0x4dcf40 dest=0x4dcf40,0x4dcf24)]
    slot 0Eh (14) -> 0x4362a7 : RIDE 0x4dcf64 plat=b dest=0x4dcf64,0x4dcf48) latch=0x4dcf60]
    slot 0Fh (15) -> 0x436362 : RIDE 0x4dcf88 plat=c dest=0x4dcf88,0x4dcf6c) latch=0x4dcf84]
    slot 10h (16) -> 0x43641f : RIDE 0x4dcfac plat=d dest=0x4dcf90,0x4dcf94)]
    slot 11h (17) -> 0x4364c3 : RIDE 0x4dcfd0 plat=e dest=0x4dcfd0,0x4dcfb4) latch=0x4dcfcc]
    slot 12h (18) -> 0x43849a : DOOR(rect=2,state=1)
    slot 13h (19) -> 0x43560c : DOOR(rect=3,state=1)
    slot 14h (20) -> 0x433af7 : DOOR(rect=5,state=1)
    slot 15h (21) -> 0x438927 : DOOR(rect=4,state=2)
    slot 16h (22) -> 0x433cbc : BEACON-ARM
    slot 17h (23) -> 0x43657e : RIDE 0x4dcff4 plat=f dest=0x4dcff4,0x4dcfd8) latch=0x4dcff0]
  M5 @0x43866c dispatch@0x4386b4 slots<=0x3d:
    slot 00h (0) -> 0x4382e0 : DOOR(rect=18,state=2)
    slot 01h (1) -> 0x4382e0 : DOOR(rect=18,state=2)
    slot 02h (2) -> 0x4382e0 : DOOR(rect=18,state=2)
    slot 03h (3) -> 0x4382e0 : DOOR(rect=18,state=2)
    slot 04h (4) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 05h (5) -> 0x438513 : DOOR(rect=8,state=2) + DOOR(rect=35,state=2) + DOOR(rect=36,state=2) + DOOR(rect=37,state=2)
    slot 06h (6) -> 0x43855e : DOOR(rect=6,state=2) + DOOR(rect=32,state=2) + DOOR(rect=33,state=2) + DOOR(rect=34,state=2)
    slot 07h (7) -> 0x438630 : DOOR(rect=5,state=2) + DOOR(rect=26,state=2) + DOOR(rect=27,state=2) + DOOR(rect=28,state=2)
    slot 08h (8) -> 0x43859a : DOOR(rect=7,state=2) + DOOR(rect=29,state=2) + DOOR(rect=30,state=2) + DOOR(rect=31,state=2)
    slot 09h (9) -> 0x43854f : DOOR(rect=25,state=1)
    slot 0Ah (10) -> 0x433acd : DOOR(rect=0,state=2)
    slot 0Bh (11) -> 0x435bfe : RIDE 0x4dcdd8]
    slot 0Ch (12) -> 0x435c2c : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 0Dh (13) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 0Eh (14) -> 0x435cd0 : RIDE 0x4dce20]
    slot 0Fh (15) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 10h (16) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 11h (17) -> 0x435cfc : RIDE 0x4dce44 plat=3 dest=0x4dce44,0x4dce28) latch=0x4dce40]
    slot 12h (18) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 13h (19) -> 0x435db7 : RIDE 0x4dce68 plat=4 dest=0x4dce68,0x4dce4c) latch=0x4dce64]
    slot 14h (20) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 15h (21) -> 0x438603 : DOOR(rect=19,state=2) + DOOR(rect=20,state=2) + DOOR(rect=15,state=2)
    slot 16h (22) -> 0x4382e0 : DOOR(rect=18,state=2)
    slot 17h (23) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 18h (24) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 19h (25) -> 0x438504 : DOOR(rect=21,state=2)
    slot 1Ah (26) -> 0x4394db : DOOR(rect=14,state=2)
    slot 1Bh (27) -> 0x4357b9 : DOOR(rect=22,state=2)
    slot 1Ch (28) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 1Dh (29) -> 0x43883a : DOOR(rect=11,state=2)
    slot 1Eh (30) -> 0x435746 : DOOR(rect=10,state=2)
    slot 1Fh (31) -> 0x4385d6 : DOOR(rect=24,state=2) + DOOR(rect=23,state=2) + DOOR(rect=13,state=2)
    slot 20h (32) -> 0x4385d6 : DOOR(rect=24,state=2) + DOOR(rect=23,state=2) + DOOR(rect=13,state=2)
    slot 21h (33) -> 0x4385d6 : DOOR(rect=24,state=2) + DOOR(rect=23,state=2) + DOOR(rect=13,state=2)
    slot 22h (34) -> 0x4385d6 : DOOR(rect=24,state=2) + DOOR(rect=23,state=2) + DOOR(rect=13,state=2)
    slot 23h (35) -> 0x4385d6 : DOOR(rect=24,state=2) + DOOR(rect=23,state=2) + DOOR(rect=13,state=2)
    slot 24h (36) -> 0x435746 : DOOR(rect=10,state=2)
    slot 25h (37) -> 0x435746 : DOOR(rect=10,state=2)
    slot 26h (38) -> 0x435746 : DOOR(rect=10,state=2)
    slot 27h (39) -> 0x435746 : DOOR(rect=10,state=2)
    slot 28h (40) -> 0x43883a : DOOR(rect=11,state=2)
    slot 29h (41) -> 0x43883a : DOOR(rect=11,state=2)
    slot 2Ah (42) -> 0x43883a : DOOR(rect=11,state=2)
    slot 2Bh (43) -> 0x43883a : DOOR(rect=11,state=2)
    slot 2Ch (44) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 2Dh (45) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 2Eh (46) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 2Fh (47) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 30h (48) -> 0x4357b9 : DOOR(rect=22,state=2)
    slot 31h (49) -> 0x4357b9 : DOOR(rect=22,state=2)
    slot 32h (50) -> 0x4357b9 : DOOR(rect=22,state=2)
    slot 33h (51) -> 0x4357b9 : DOOR(rect=22,state=2)
    slot 34h (52) -> 0x4394db : DOOR(rect=14,state=2)
    slot 35h (53) -> 0x4394db : DOOR(rect=14,state=2)
    slot 36h (54) -> 0x4394db : DOOR(rect=14,state=2)
    slot 37h (55) -> 0x438504 : DOOR(rect=21,state=2)
    slot 38h (56) -> 0x438504 : DOOR(rect=21,state=2)
    slot 39h (57) -> 0x438504 : DOOR(rect=21,state=2)
    slot 3Ah (58) -> 0x438504 : DOOR(rect=21,state=2)
    slot 3Bh (59) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 3Ch (60) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 3Dh (61) -> 0x433cbc : BEACON-ARM
  H2H @0x4380b8 (mode==2 gate; missions 1..2 -> probe blocks)
  H2H-M1 @0x437688 dispatch@0x4376d2 slots<=0x26:
    slot 00h (0) -> 0x435bfe : RIDE 0x4dcdd8]
    slot 01h (1) -> 0x435c2c : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 02h (2) -> 0x435cd0 : RIDE 0x4dce20]
    slot 03h (3) -> 0x435cfc : RIDE 0x4dce44 plat=3 dest=0x4dce44,0x4dce28) latch=0x4dce40]
    slot 04h (4) -> 0x435db7 : RIDE 0x4dce68 plat=4 dest=0x4dce68,0x4dce4c) latch=0x4dce64]
    slot 05h (5) -> 0x435e74 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 06h (6) -> 0x435f18 : RIDE 0x4dceb0 plat=6 dest=0x4dceb0,0x4dce94) latch=0x4dceac]
    slot 07h (7) -> 0x435fd3 : RIDE 0x4dced4 plat=7 dest=0x4dced4,0x4dceb8) latch=0x4dced0]
    slot 08h (8) -> 0x43608e : RIDE 0x4dcef8 plat=8 dest=0x4dcef8,0x4dcedc) latch=0x4dcef4]
    slot 09h (9) -> 0x43614b : RIDE 0x4dcf1c plat=9 dest=0x4dcf00,0x4dcf04)]
    slot 0Ah (10) -> 0x4361ef : RIDE 0x4dcf40 dest=0x4dcf40,0x4dcf24)]
    slot 0Bh (11) -> 0x4362a7 : RIDE 0x4dcf64 plat=b dest=0x4dcf64,0x4dcf48) latch=0x4dcf60]
    slot 0Ch (12) -> 0x436362 : RIDE 0x4dcf88 plat=c dest=0x4dcf88,0x4dcf6c) latch=0x4dcf84]
    slot 0Dh (13) -> 0x43641f : RIDE 0x4dcfac plat=d dest=0x4dcf90,0x4dcf94)]
    slot 0Eh (14) -> 0x4364c3 : RIDE 0x4dcfd0 plat=e dest=0x4dcfd0,0x4dcfb4) latch=0x4dcfcc]
    slot 0Fh (15) -> 0x43657e : RIDE 0x4dcff4 plat=f dest=0x4dcff4,0x4dcfd8) latch=0x4dcff0]
    slot 10h (16) -> 0x436639 : RIDE 0x4dd018 plat=10 latch=0x4dd014]
    slot 11h (17) -> 0x4366f6 : RIDE 0x4dd03c plat=11]
    slot 12h (18) -> 0x43679a : RIDE 0x4dd060 plat=12 latch=0x4dd05c]
    slot 13h (19) -> 0x436855 : RIDE 0x4dd084 plat=13 latch=0x4dd080]
    slot 14h (20) -> 0x436910 : RIDE 0x4dd0a8 plat=14 latch=0x4dd0a4]
    slot 15h (21) -> 0x4369cd : RIDE 0x4dd0cc plat=15]
    slot 16h (22) -> 0x436a71 : RIDE 0x4dd0f0 plat=16 latch=0x4dd0ec]
    slot 17h (23) -> 0x436b2c : RIDE 0x4dd114 plat=17 latch=0x4dd110]
    slot 18h (24) -> 0x436be7 : RIDE 0x4dd138 plat=18 latch=0x4dd134]
    slot 19h (25) -> 0x436ca4 : RIDE 0x4dd15c plat=19]
    slot 1Ah (26) -> 0x436d48 : RIDE 0x4dd180 plat=1a latch=0x4dd17c]
    slot 1Bh (27) -> 0x436e03 : RIDE 0x4dd1a4 plat=1b latch=0x4dd1a0]
    slot 1Ch (28) -> 0x436ebe : RIDE 0x4dd1c8 plat=1c latch=0x4dd1c4]
    slot 1Dh (29) -> 0x436f7b : RIDE 0x4dd1ec plat=1d]
    slot 1Eh (30) -> 0x43701f : RIDE 0x4dd210 plat=1e latch=0x4dd20c]
    slot 1Fh (31) -> 0x4370da : RIDE 0x4dd234 plat=1f latch=0x4dd230]
    slot 20h (32) -> 0x437195 : RIDE 0x4dd258 plat=20 latch=0x4dd254]
    slot 21h (33) -> 0x437252 : RIDE 0x4dd27c plat=21]
    slot 22h (34) -> 0x4372f6 : RIDE 0x4dd2a0 plat=22 latch=0x4dd29c]
    slot 23h (35) -> 0x4373b1 : RIDE 0x4dd2c4 plat=23 latch=0x4dd2c0]
    slot 24h (36) -> 0x43746c : RIDE 0x4dd2e8 plat=24 latch=0x4dd2e4]
    slot 25h (37) -> 0x437529 : RIDE 0x4dd30c plat=25]
    slot 26h (38) -> 0x4375cd : RIDE 0x4dd330 plat=26 latch=0x4dd32c]
  H2H-M2 @0x43805f dispatch@0x4380a7 slots<=0x1b:
    slot 00h (0) -> 0x433a14 : RIDE 0x4dcdd8 dest=0x4dcdd8,0x4dcdbc) latch=0x4dcdd4]
    slot 01h (1) -> 0x4376e3 : RIDE 0x4dcdfc]
    slot 02h (2) -> 0x437711 : RIDE 0x4dce20 plat=2 dest=0x4dce04,0x4dce08)]
    slot 03h (3) -> 0x435cfc : RIDE 0x4dce44 plat=3 dest=0x4dce44,0x4dce28) latch=0x4dce40]
    slot 04h (4) -> 0x433c62 : RIDE 0x4dce68]
    slot 05h (5) -> 0x4377cf : RIDE 0x4dce8c]
    slot 06h (6) -> 0x4377fd : RIDE 0x4dceb0 plat=6 dest=0x4dce94,0x4dce98)]
    slot 07h (7) -> 0x435fd3 : RIDE 0x4dced4 plat=7 dest=0x4dced4,0x4dceb8) latch=0x4dced0]
    slot 08h (8) -> 0x4378a1 : RIDE 0x4dcef8]
    slot 09h (9) -> 0x4378cd : RIDE 0x4dcf1c plat=9 dest=0x4dcf1c,0x4dcf00) latch=0x4dcf18]
    slot 0Ah (10) -> 0x43798a : RIDE 0x4dcf40]
    slot 0Bh (11) -> 0x4362a7 : RIDE 0x4dcf64 plat=b dest=0x4dcf64,0x4dcf48) latch=0x4dcf60]
    slot 0Ch (12) -> 0x437a2b : RIDE 0x4dcf88]
    slot 0Dh (13) -> 0x437a57 : RIDE 0x4dcfac plat=d dest=0x4dcfac,0x4dcf90) latch=0x4dcfa8]
    slot 0Eh (14) -> 0x437b14 : RIDE 0x4dcfd0 plat=e dest=0x4dcfb4,0x4dcfb8)]
    slot 0Fh (15) -> 0x43657e : RIDE 0x4dcff4 plat=f dest=0x4dcff4,0x4dcfd8) latch=0x4dcff0]
    slot 10h (16) -> 0x437bb8 : RIDE 0x4dd018]
    slot 11h (17) -> 0x437be4 : RIDE 0x4dd03c plat=11 latch=0x4dd038]
    slot 12h (18) -> 0x437ca1 : RIDE 0x4dd060 plat=12]
    slot 13h (19) -> 0x436855 : RIDE 0x4dd084 plat=13 latch=0x4dd080]
    slot 14h (20) -> 0x437d45 : RIDE 0x4dd0a8]
    slot 15h (21) -> 0x437d71 : RIDE 0x4dd0cc plat=15 latch=0x4dd0c8]
    slot 16h (22) -> 0x437e2e : RIDE 0x4dd0f0 plat=16]
    slot 17h (23) -> 0x436b2c : RIDE 0x4dd114 plat=17 latch=0x4dd110]
    slot 18h (24) -> 0x437ed2 : RIDE 0x4dd138]
    slot 19h (25) -> 0x437efe : RIDE 0x4dd15c plat=19 latch=0x4dd158]
    slot 1Ah (26) -> 0x437fbb : RIDE 0x4dd180 plat=1a]
    slot 1Bh (27) -> 0x436e03 : RIDE 0x4dd1a4 plat=1b latch=0x4dd1a0]
========================================================================
ZONE D  entry 0x4386c5
  M1 @0x4388a2 dispatch@0x4388ec slots<=0x8:
    slot 00h (0) -> 0x4387be : DOOR(rect=0,state=2) + DOOR(rect=1,state=2) + DOOR(rect=2,state=2) + DOOR(rect=3,state=2)
    slot 01h (1) -> 0x4387e5 : DOOR(rect=4,state=2) + DOOR(rect=5,state=2) + DOOR(rect=6,state=2) + DOOR(rect=7,state=2)
    slot 02h (2) -> 0x43880d : DOOR(rect=8,state=2) + DOOR(rect=9,state=2) + DOOR(rect=10,state=2) + DOOR(rect=11,state=2)
    slot 03h (3) -> 0x438849 : DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2) + DOOR(rect=15,state=2)
    slot 04h (4) -> 0x435823 : RIDE 0x4dcdd8]
    slot 05h (5) -> 0x438876 : RIDE 0x4dcdfc]
    slot 06h (6) -> 0x4341f4 : RIDE 0x4dce20]
    slot 07h (7) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 08h (8) -> 0x433cbc : BEACON-ARM
  M2 @0x43894f dispatch@0x438999 slots<=0x7:
    slot 00h (0) -> 0x433acd : DOOR(rect=0,state=2)
    slot 01h (1) -> 0x4388fd : DOOR(rect=1,state=2) + DOOR(rect=1,state=2) + DOOR(rect=3,state=2) + DOOR(rect=4,state=2)
    slot 02h (2) -> 0x438936 : DOOR(rect=5,state=2) + DOOR(rect=6,state=1)
    slot 03h (3) -> 0x4380e5 : DOOR(rect=7,state=2)
    slot 04h (4) -> 0x435823 : RIDE 0x4dcdd8]
    slot 05h (5) -> 0x438876 : RIDE 0x4dcdfc]
    slot 06h (6) -> 0x4341f4 : RIDE 0x4dce20]
    slot 07h (7) -> 0x433cbc : BEACON-ARM
  M3 @0x4389aa dispatch@0x4389f4 slots<=0xf:
    slot 00h (0) -> 0x438424 : DOOR(rect=0,state=2) + DOOR(rect=1,state=2)
    slot 01h (1) -> 0x435b77 : DOOR(rect=2,state=2)
    slot 02h (2) -> 0x433ae8 : DOOR(rect=3,state=2)
    slot 03h (3) -> 0x438927 : DOOR(rect=4,state=2)
    slot 04h (4) -> 0x435620 : DOOR(rect=5,state=2) + DOOR(rect=6,state=2) + DOOR(rect=7,state=2) + DOOR(rect=8,state=2)
    slot 05h (5) -> 0x439655 : DOOR(rect=9,state=1)
    slot 06h (6) -> 0x435746 : DOOR(rect=10,state=2)
    slot 07h (7) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 08h (8) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 09h (9) -> 0x434fe9 : RIDE 0x4dce20 plat=2]
    slot 0Ah (10) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 0Bh (11) -> 0x43883a : DOOR(rect=11,state=2)
    slot 0Ch (12) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 0Dh (13) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 0Eh (14) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 0Fh (15) -> 0x433cbc : BEACON-ARM
  M4 @0x438a5f dispatch@0x438aa7 slots<=0x10:
    slot 00h (0) -> 0x433acd : DOOR(rect=0,state=2)
    slot 01h (1) -> 0x438430 : DOOR(rect=1,state=2)
    slot 02h (2) -> 0x43849a : DOOR(rect=2,state=1)
    slot 03h (3) -> 0x433ae8 : DOOR(rect=3,state=2)
    slot 04h (4) -> 0x435616 : DOOR(rect=4,state=1)
    slot 05h (5) -> 0x4381ad : DOOR(rect=5,state=2)
    slot 06h (6) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 07h (7) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 08h (8) -> 0x438a05 : DOOR(rect=8,state=2) + DOOR(rect=9,state=2) + DOOR(rect=10,state=2) + DOOR(rect=7,state=2)
    slot 09h (9) -> 0x438a05 : DOOR(rect=8,state=2) + DOOR(rect=9,state=2) + DOOR(rect=10,state=2) + DOOR(rect=7,state=2)
    slot 0Ah (10) -> 0x438a05 : DOOR(rect=8,state=2) + DOOR(rect=9,state=2) + DOOR(rect=10,state=2) + DOOR(rect=7,state=2)
    slot 0Bh (11) -> 0x438a32 : DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2) + DOOR(rect=11,state=2)
    slot 0Ch (12) -> 0x438a32 : DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2) + DOOR(rect=11,state=2)
    slot 0Dh (13) -> 0x438a32 : DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2) + DOOR(rect=11,state=2)
    slot 0Eh (14) -> 0x438621 : DOOR(rect=15,state=2)
    slot 0Fh (15) -> 0x438621 : DOOR(rect=15,state=2)
    slot 10h (16) -> 0x433cbc : BEACON-ARM
  M5 @0x438dd1 dispatch@0x438e1b slots<=0x9:
    slot 00h (0) -> 0x43560c : DOOR(rect=3,state=1)
    slot 01h (1) -> 0x438ab8 : DOOR(rect=9,state=2) + DOOR(rect=10,state=2) + DOOR(rect=11,state=2) + DOOR(rect=12,state=2)
    slot 02h (2) -> 0x438af9 : DOOR(rect=8,state=1)
    slot 03h (3) -> 0x4356b7 : RIDE 0x4dcdd8]
    slot 04h (4) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 05h (5) -> 0x438b03 : <no-action>
    slot 06h (6) -> 0x438b9b : <no-action>
    slot 07h (7) -> 0x438c56 : <no-action>
    slot 08h (8) -> 0x438d13 : RIDE 0x4dce8c]
    slot 09h (9) -> 0x433cbc : BEACON-ARM
  H2H @0x4387a1 (mode==2 gate; missions 1..2 -> probe blocks)
========================================================================
ZONE E  entry 0x432c8e
========================================================================
ZONE F  entry 0x439323
  M1 @0x43954e dispatch@0x439596 slots<=0x13:
    slot 00h (0) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 01h (1) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 02h (2) -> 0x434fe9 : RIDE 0x4dce20 plat=2]
    slot 03h (3) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 04h (4) -> 0x43424a : RIDE 0x4dce68 plat=4 dest=0x4dce4c,0x4dce50)]
    slot 05h (5) -> 0x434307 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 06h (6) -> 0x435165 : RIDE 0x4dceb0 plat=6]
    slot 07h (7) -> 0x435224 : RIDE 0x4dced4 plat=7 dest=0x4dceb8,0x4dcebc)]
    slot 08h (8) -> 0x438f97 : DOOR(rect=10,state=1) + EXIT-ACTIVATE(pad=10)
    slot 09h (9) -> 0x43841a : DOOR(rect=9,state=2)
    slot 0Ah (10) -> 0x438af9 : DOOR(rect=8,state=1)
    slot 0Bh (11) -> 0x4394ae : DOOR(rect=11,state=2) + DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2)
    slot 0Ch (12) -> 0x4394ae : DOOR(rect=11,state=2) + DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2)
    slot 0Dh (13) -> 0x4394ae : DOOR(rect=11,state=2) + DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2)
    slot 0Eh (14) -> 0x4394ae : DOOR(rect=11,state=2) + DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2)
    slot 0Fh (15) -> 0x4394ae : DOOR(rect=11,state=2) + DOOR(rect=12,state=2) + DOOR(rect=13,state=2) + DOOR(rect=14,state=2)
    slot 10h (16) -> 0x4394ea : DOOR(rect=15,state=1) + DOOR(rect=16,state=1)
    slot 11h (17) -> 0x439503 : DOOR(rect=7,state=2) + DOOR(rect=17,state=2) + DOOR(rect=18,state=2) + DOOR(rect=19,state=2)
    slot 12h (18) -> 0x433cbc : BEACON-ARM
    slot 13h (19) -> 0x43953f : DOOR(rect=20,state=2)
  M2 @0x4395ba dispatch@0x439604 slots<=0x12:
    slot 00h (0) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 01h (1) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 02h (2) -> 0x434fe9 : RIDE 0x4dce20 plat=2]
    slot 03h (3) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 04h (4) -> 0x43424a : RIDE 0x4dce68 plat=4 dest=0x4dce4c,0x4dce50)]
    slot 05h (5) -> 0x434307 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 06h (6) -> 0x435165 : RIDE 0x4dceb0 plat=6]
    slot 07h (7) -> 0x435224 : RIDE 0x4dced4 plat=7 dest=0x4dceb8,0x4dcebc)]
    slot 08h (8) -> 0x43441a : RIDE 0x4dcef8 plat=8 dest=0x4dcedc,0x4dcee0)]
    slot 09h (9) -> 0x4344d7 : RIDE 0x4dcf1c plat=9 dest=0x4dcf00,0x4dcf04)]
    slot 0Ah (10) -> 0x4352e1 : RIDE 0x4dcf40]
    slot 0Bh (11) -> 0x43539c : RIDE 0x4dcf64 plat=b dest=0x4dcf48,0x4dcf4c)]
    slot 0Ch (12) -> 0x439000 : DOOR(rect=0,state=1) + EXIT-ACTIVATE(pad=0)
    slot 0Dh (13) -> 0x43402c : DOOR(rect=1,state=1)
    slot 0Eh (14) -> 0x43849a : DOOR(rect=2,state=1)
    slot 0Fh (15) -> 0x4345ea : RIDE 0x4dcf88 plat=c dest=0x4dcf6c,0x4dcf70)]
    slot 10h (16) -> 0x4346a7 : RIDE 0x4dcfac plat=d dest=0x4dcf90,0x4dcf94)]
    slot 11h (17) -> 0x433cbc : BEACON-ARM
    slot 12h (18) -> 0x4395ab : DOOR(rect=12,state=1) + EXIT-ACTIVATE(pad=12)
  M3 @0x439682 dispatch@0x4396ca slots<=0x12:
    slot 00h (0) -> 0x433cbc : BEACON-ARM
    slot 01h (1) -> 0x439619 : DOOR(rect=5,state=1) + DOOR(rect=6,state=1) + DOOR(rect=7,state=1) + DOOR(rect=8,state=1) + DOOR(rect=9,state=1)
    slot 02h (2) -> 0x438ae5 : DOOR(rect=12,state=2)
    slot 03h (3) -> 0x4394cc : DOOR(rect=13,state=2) + DOOR(rect=14,state=2)
    slot 04h (4) -> 0x4356b7 : RIDE 0x4dcdd8]
    slot 05h (5) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 06h (6) -> 0x4341f4 : RIDE 0x4dce20]
    slot 07h (7) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 08h (8) -> 0x4356ed : RIDE 0x4dce68]
    slot 09h (9) -> 0x434307 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 0Ah (10) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 0Bh (11) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 0Ch (12) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 0Dh (13) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 0Eh (14) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 0Fh (15) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 10h (16) -> 0x439664 : DOOR(rect=17,state=2) + EXIT-ACTIVATE(pad=17)
    slot 11h (17) -> 0x439673 : DOOR(rect=18,state=2) + EXIT-ACTIVATE(pad=18)
    slot 12h (18) -> 0x43578c : DOOR(rect=19,state=2) + DOOR(rect=20,state=2) + DOOR(rect=21,state=2) + DOOR(rect=22,state=2)
  M4 @0x4396fd dispatch@0x439747 slots<=0x15:
    slot 00h (0) -> 0x43407e : RIDE 0x4dcdd8 dest=0x4dcdbc,0x4dcdc0)]
    slot 01h (1) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 02h (2) -> 0x434fe9 : RIDE 0x4dce20 plat=2]
    slot 03h (3) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 04h (4) -> 0x43424a : RIDE 0x4dce68 plat=4 dest=0x4dce4c,0x4dce50)]
    slot 05h (5) -> 0x434307 : RIDE 0x4dce8c plat=5 dest=0x4dce70,0x4dce74)]
    slot 06h (6) -> 0x435165 : RIDE 0x4dceb0 plat=6]
    slot 07h (7) -> 0x435224 : RIDE 0x4dced4 plat=7 dest=0x4dceb8,0x4dcebc)]
    slot 08h (8) -> 0x43441a : RIDE 0x4dcef8 plat=8 dest=0x4dcedc,0x4dcee0)]
    slot 09h (9) -> 0x4344d7 : RIDE 0x4dcf1c plat=9 dest=0x4dcf00,0x4dcf04)]
    slot 0Ah (10) -> 0x4352e1 : RIDE 0x4dcf40]
    slot 0Bh (11) -> 0x43539c : RIDE 0x4dcf64 plat=b dest=0x4dcf48,0x4dcf4c)]
    slot 0Ch (12) -> 0x4381ad : DOOR(rect=5,state=2)
    slot 0Dh (13) -> 0x435459 : DOOR(rect=15,state=1)
    slot 0Eh (14) -> 0x4381e9 : DOOR(rect=16,state=2)
    slot 0Fh (15) -> 0x4345ea : RIDE 0x4dcf88 plat=c dest=0x4dcf6c,0x4dcf70)]
    slot 10h (16) -> 0x4346a7 : RIDE 0x4dcfac plat=d dest=0x4dcf90,0x4dcf94)]
    slot 11h (17) -> 0x43841a : DOOR(rect=9,state=2)
    slot 12h (18) -> 0x4396df : DOOR(rect=13,state=1) + EXIT-ACTIVATE(pad=13)
    slot 13h (19) -> 0x4396ee : DOOR(rect=11,state=1) + EXIT-ACTIVATE(pad=11)
    slot 14h (20) -> 0x435782 : DOOR(rect=14,state=1)
    slot 15h (21) -> 0x433cbc : BEACON-ARM
  M5 @0x439a4f dispatch@0x439a7d slots<=0x1b:
    slot 00h (0) -> 0x435616 : DOOR(rect=4,state=1)
    slot 01h (1) -> 0x435616 : DOOR(rect=4,state=1)
    slot 02h (2) -> 0x433af7 : DOOR(rect=5,state=1)
    slot 03h (3) -> 0x433af7 : DOOR(rect=5,state=1)
    slot 04h (4) -> 0x438945 : DOOR(rect=6,state=1)
    slot 05h (5) -> 0x43975c : DOOR(rect=7,state=1) + EXIT-ACTIVATE(pad=7)
    slot 06h (6) -> 0x438af9 : DOOR(rect=8,state=1)
    slot 07h (7) -> 0x4396ee : DOOR(rect=11,state=1) + EXIT-ACTIVATE(pad=11)
    slot 08h (8) -> 0x43976b : DOOR(rect=12,state=1)
    slot 09h (9) -> 0x4394db : DOOR(rect=14,state=2)
    slot 0Ah (10) -> 0x4394db : DOOR(rect=14,state=2)
    slot 0Bh (11) -> 0x4394db : DOOR(rect=14,state=2)
    slot 0Ch (12) -> 0x4381da : DOOR(rect=15,state=2) + DOOR(rect=16,state=2)
    slot 0Dh (13) -> 0x4381da : DOOR(rect=15,state=2) + DOOR(rect=16,state=2)
    slot 0Eh (14) -> 0x4381da : DOOR(rect=15,state=2) + DOOR(rect=16,state=2)
    slot 0Fh (15) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 10h (16) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 11h (17) -> 0x4382d1 : DOOR(rect=17,state=2)
    slot 12h (18) -> 0x4382e0 : DOOR(rect=18,state=2)
    slot 13h (19) -> 0x439775 : <no-action>
    slot 14h (20) -> 0x4397ea : <no-action>
    slot 15h (21) -> 0x43985a : RIDE 0x4dce20]
    slot 16h (22) -> 0x4398cb : RIDE 0x4dce44]
    slot 17h (23) -> 0x439941 : <no-action>
    slot 18h (24) -> 0x4399b5 : <no-action>
    slot 19h (25) -> 0x4396df : DOOR(rect=13,state=1) + EXIT-ACTIVATE(pad=13)
    slot 1Ah (26) -> 0x439a29 : BEACON-ARM
    slot 1Bh (27) -> 0x439a40 : DOOR(rect=19,state=2) + EXIT-ACTIVATE(pad=19)
  H2H @0x439491 (mode==2 gate; missions 1..2 -> probe blocks)
  H2H-M1 @0x4393ad dispatch@0x4393f5 slots<=0xe:
    slot 00h (0) -> 0x4356b7 : RIDE 0x4dcdd8]
    slot 01h (1) -> 0x434137 : RIDE 0x4dcdfc plat=1 dest=0x4dcde0,0x4dcde4)]
    slot 02h (2) -> 0x4341f4 : RIDE 0x4dce20]
    slot 03h (3) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 04h (4) -> 0x4356ed : RIDE 0x4dce68]
    slot 05h (5) -> 0x439349 : RIDE 0x4dcdd8]
    slot 06h (6) -> 0x4343c4 : RIDE 0x4dceb0]
    slot 07h (7) -> 0x435224 : RIDE 0x4dced4 plat=7 dest=0x4dceb8,0x4dcebc)]
    slot 08h (8) -> 0x439355 : RIDE 0x4dcef8]
    slot 09h (9) -> 0x4344d7 : RIDE 0x4dcf1c plat=9 dest=0x4dcf00,0x4dcf04)]
    slot 0Ah (10) -> 0x434594 : RIDE 0x4dcf40]
    slot 0Bh (11) -> 0x43539c : RIDE 0x4dcf64 plat=b dest=0x4dcf48,0x4dcf4c)]
    slot 0Ch (12) -> 0x439381 : RIDE 0x4dcf88]
    slot 0Dh (13) -> 0x4346a7 : RIDE 0x4dcfac plat=d dest=0x4dcf90,0x4dcf94)]
    slot 0Eh (14) -> 0x43476b : RIDE 0x4dcfd0 plat=e dest=0x4dcfb4,0x4dcfb8)]
  H2H-M2 @0x439434 dispatch@0x43947e slots<=0x6:
    slot 00h (0) -> 0x435823 : RIDE 0x4dcdd8]
    slot 01h (1) -> 0x438876 : RIDE 0x4dcdfc]
    slot 02h (2) -> 0x4341f4 : RIDE 0x4dce20]
    slot 03h (3) -> 0x4350a8 : RIDE 0x4dce44 plat=3 dest=0x4dce28,0x4dce2c)]
    slot 04h (4) -> 0x43424a : RIDE 0x4dce68 plat=4 dest=0x4dce4c,0x4dce50)]
    slot 05h (5) -> 0x439408 : RIDE 0x4dce8c]
    slot 06h (6) -> 0x4343c4 : RIDE 0x4dceb0]
========================================================================
ZONE G  entry 0x439ae2
  M1 @0x439aa6 dispatch@0x439ad2 slots<=0x1:
    slot 00h (0) -> 0x439a92 : DOOR(rect=0,state=2) + EXIT-ACTIVATE(pad=0)
    slot 01h (1) -> 0x439a9c : DOOR(rect=1,state=2)   ; rect/state inherit live regs
```

## 7j.47. THE TOT PLANE-6/7 SEMANTICS — CLOSED: planes 6/7 are ordinary z-levels 6/7 of the word stack (tall-structure tops); they STAGE and DRAW like every other plane (no z≥6 gate anywhere); the ~2000-entry target-table hypothesis REFUTED (2026-08-23, worker f29066bd claim 2, D119; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run; read-only corpus probes over game-data TOT/DAT/POS, scratch /tmp/opencode; manifest clean before AND after)

Method: instruction walk of every 0x4796bc/0x4796cc consumer family in the
0x406xxx draw range (the §7j.32 reader sites 0x406891/0x4068ec/0x406907/
0x406a0e/0x406a1a + their enclosing loop 0x4067cf..0x406c73) + init_tiles
0x407e11 staging + the two non-draw mirror walkers, then a full-corpus
census (37 missions × 8 planes; TOT grammar per FORMATS §2 with the D118
word-unit addressing).

### 1. The renderer verdict — plane-6/7 words DO draw [verified]

The terrain z-stack draw loop (in FUN_00403938; consumes the
[0x4ede24]/[0x4ede28] RESTAMP LIST built by init_tiles for the initial
full screen + appended by FUN_00440a2d on scroll) walks per record:
`[esp+0x154]` = z 0..7 (`cmp ebx,0x8; jge next-record` @0x406863/0x406866;
loop-next 0x406821: mirror ptr +2 @0x406840, screen y −0x5000 @0x40683a,
overlay scan ptr +0x1440 @0x406843) and `[esp+0x3c]` = k, the stack-draw
cursor, reset to 0 for EVERY record (@0x406c00/0x406c08 — `xor eax,eax`
unconditional on both the +0x19-door-tag branches):
- **Block 1** (the restart draw, 0x406882..0x406941): fires when k == z;
  gate = mirror word@[record+2z] ≠ 0 ALONE (`cmp WORD PTR [eax+0x4796bc],0;
  je 0x406941` @0x406891 — **NO seen check**). Draws via the LNK remap
  (word → u16@(0x45cdda+2·word), write-back @0x4068e3), the palette pick
  (objective-height pair +0x1B/+0x1C in range [z0 ≤ z < z0+D] → the
  §7j.35 ping-pong const 0x456ca8 @0x40691a; else scorch +0x18 @0x406928)
  and FUN_00401471 (bank [0x4ede1c], PALTRAN ramps 0x4dd444).
- **Block 2** (the contiguous chain, 0x40695c..0x406a59): k++ while
  k < 8 ∧ seen@[record+0x10+k] ≠ 0 (@0x40696e) ∧ word@[record+2k] ≠ 0
  (@0x40697b); same remap/palette + the §7j.35 water special-case
  (word within [u32@0x454aac+4·[0x4edd8c], +0xE) → FUN_0040167a instead,
  @0x4069c7..0x4069fb).
- **Catch-up**: when the chain breaks at plane m (seen==0 or word==0),
  the outer z loop reaches m and Block 1 re-fires (word-only gate), so
  **every plane 0..7 whose word is nonzero draws** (seen only short-cuts
  the contiguous fast path, never suppresses a word). Screen-bounds
  culling (0x40689f..0x4068bb / 0x4069a2..0x4069b0 vs backbuffer
  [0x4ede18]) and the 0x24×0x24 occlusion bounds (0x406a87..0x406ab1)
  are plane-agnostic.
- init_tiles stages ALL 8 planes: z loop `cmp esi,0x8` @0x407fce, word≠0
  → mirror @0x407fe4, DAT byte gates ONLY seen @0x407ff1 (§7h.4 shape).
- The two adjacent non-draw walkers are equally unbounded: the terrain
  overlay scanner 0x408a49..0x408ade walks planes 1..7 (ebp 2..0xE step
  2, `cmp ebp,0x10` @0x40889; LNK remap @0x408aab, nonzero → 0x402ab8),
  and the per-plane range consumer 0x42035c..0x4203a5 walks planes 0..7
  (edx 0..0xE step 2, `cmp edx,0x10` @0x420360) testing the mirror word
  against [0x454ae4+4·[0x4edd8c]] (+0xE window) → FUN_0042394a.
**No z ≥ 6 gate, skip, or special case exists in any of the four
families.** A TOT plane-6/7 word stages, remaps, palette-picks and blits
exactly like a plane-0..5 word; the only z-dependent draw behavior in the
loop is the screen-y offset (−0x5000 per level).

### 2. The corpus census [verified, full 37-mission sweep]

- 36/37 missions carry plane-6/7 words (ZONEG/MISSION1 is the only zero;
  totals: 9 296 cells, 8 016 nonzero plane-6 words + 2 882 plane-7 words
  — matching FORMATS §2; cells: 6 414 p6-only / 1 280 p7-only / 1 602
  both). Heaviest: ZONEB/M5 (1 105), ZONEF/M3 (1 008), ZONEE/M2 (526).
- **Overlay vs standalone**: 6 504 cells ALSO nonzero at planes 0..5
  (upper levels of an existing column), 2 792 standalone (planes 0..5 all
  zero — floating z=6/7 sprites; drawn by the Block-1 catch-up path).
- **Value domain identity**: global nonzero domains plane 0 = 1..1494,
  planes 1..5 = 33..1868 (maxes 1866/1867/1868/1866/1867), plane 6 =
  35..1868, plane 7 = 36..1868 — the SAME word population; 8 738/10 898
  plane-6/7 values also occur at planes 0..5 of the same mission.
- **The tall-tower shape**: the words are per-level sprite ids of
  multi-storey structures. ZONEA/M1 tile (17,25) — the corpus only zone-A
  cell — is one tower column: TOT = [0,0,0,0,454,1354,1355,1356], DAT =
  [1,0,0,0,1,1,1,1] (base at z=4, three stacked levels; the famous
  \"1355/1356 adjacent integers\" are simply the z-6/z-7 sprite ids).
  ZONEB/M1 (88,19) = [0,345,345,303,1866,1867,**1868**]; ZONEB/M1
  (26..30,26) = descending ramps 1755,1754,1754,1754 → **1753**;
  ZONEB/M1 (19..26, 88..90) = sequential 1153..1161 across adjacent
  tiles (one multi-tile building with sequential per-level ids).
- DAT bytes at the plane-6/7 words: overwhelmingly 1 (5 558/8 016 at p6,
  1 733/2 882 at p7), 0 in ~7% → seen=1 staged at load for ~93%, seen=0
  cells still draw via the catch-up restart; the full DAT value spread
  (2/3/10/37/99..102…) is the ordinary decoration/type mix.

### 3. The ~2000-entry target-table hypothesis — REFUTED [verified]

FORMATS §2 plane-value=POS-slot reading (values ≤ 1868 \"just under\"
the 2000-slot .POS count) is dead on three independent legs:
1. **Domain coincidence**: the tile-word domain tops at 1868 at planes
   1..5 TOO (pre-LNK terrain-type ids); the \"just under 2000\" nearness
   is a property of the word grammar, not of .POS.
2. **No linkage**: resolving every plane-6/7 value as a .POS slot in its
   own mission gives 9 217 live / 1 681 empty — the coincidental live
   fraction of dense 2000-slot banks, not a semantic map (a real linkage
   resolves 100%); ZONEA/M1 only pair (1355/1356) hits EMPTY slots.
3. **The words draw**: §7j.47/1 — they are consumed as sprite ids by the
   ordinary terrain stack path (LNK remap → .BIN blit), the same as
   planes 0..5. p7 == p6+1 holds at only 83/9 296 cells — the ZONEA
   adjacent-pair pattern is not systematic.

### 4. Consequences

- FORMATS §2 plane-6/7 paragraph closed (this section is its anchor);
  the §2 \"what RE must confirm: the plane-6/7 target table\" item CLOSED.
- E-side: no seam change — stage_pickup_surface already stages EVERY
  nonzero plane word (D107), so E treats planes 6/7 uniformly today;
  a future scenario that walks a tall tower (z up to 7) exercises the
  same chain, with the caveat that robots() pathing probes remain
  z-bounded by their own families (not by the draw stack).
- Differ/watch set: nothing new to watch — plane 6/7 words live inside
  the EXISTING typedb-mirror/TOT rows.

## 7j.48. THE MISSIONVIEW §5d TAIL — the MP ROBNUMS name plates + the SHIELD/TELEPORT/ROBNUMS bank staging + the unstaged-flush question CLOSED (2026-08-23, worker 328b7651 claim 2, D120; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run; read-only corpus probes over game-data/BEDLAM/GAMEGFX bank headers; manifest clean before AND after; adopts + validates the interrupted predecessor WIP in docs/RE-EXW-MISSIONVIEW.md §5d)

The §5d robot-entity enqueue items 1/3 carry TWO label corrections
(cell names re-anchored to the 7j.28/7j.30 corpus-string census), and
the Backlog tail (bank staging + unstaged-flush + full name-plate
grammar) is decoded below. SP is affected by NONE of it (§7j.48/5).

### 1. The label corrections (the draws themselves were already right)

- **[0x46af38] = GAMEGFX\TELEPORT.BIN (10 imgs, corpus u16 count)** —
  the §5d item-1 draw (state u16@+0x0c ∈ {5,6}, mode 0x12e, y =
  sy−0x48, frame = clamp(10 − wobble/4, 0..9), wobble = i32@+0x90) is
  the TELEPORT BEAM, not a "shield" (asm 0x403de6..0x403e71; bank
  load `mov ebx,[0x46af38]` @0x403e62; the 0..9 clamp matches the
  10-image bank exactly). The same bank serves the §5e platform
  loops and the 7j.21 arrival-marker draw (sprite 0x12E width-clamped
  by countdown).
- **[0x46af44] = GAMEGFX\SHIELD.BIN (4 imgs)** — the §5d item-3 draw
  (i32@+0x88 ≠ 0 → frame = u16@+0x18, mode 0x12c @0x403ef4..0x403f29)
  is the SHIELD sprite, not a "variant sprite": the 4-image count
  matches both the spawn initialization `u16@+0x18 := RandA()&3`
  (FUN_0040cca0) and the post-loop shimmer `(+1)&3` (0x403cf7:
  `mov ax,[esi+0x4c69fc]; inc eax; and al,3` — robot stride 0xa8,
  +0x18 word). A "variant" would not shimmer on a 2-bit cycle.

### 2. WHO STAGES the banks — alloc+load at MissionShell HEAD, unconditional

Exactly ONE call site each, both on the straight-line head of
MissionShell = FUN_0044771c (0x44771c..0x44874b; RE-EXW-SIM §1, 8street
"game_level" — runs every mission, SP and MP alike):

- 0x447860 → FUN_0041d954: the arena-allocator pass, a straight run
  of ArenaAlloc(size)→cell stores. Bank buffer sizes: TELEPORT
  0x6d60, NUMBERS 0xfa0, FLAGS 0x3a98, ROBNUMS 0xbb8, SHIELD 0x1b58
  (= the DEBRIS size, [0x4eddb4]), DANTE 0x14c08 ([0x4ede2c]).
- 0x447b3f → FUN_0041df10: the LoadFile pass — a straight run of
  `FUN_0041cc7f(nameString, bankCell)` loads for the whole GAMEGFX
  entity-bank set: DANTE@0x41df19, …, TELEPORT@0x41df99, NUMBERS
  0x41dfa9, FLAGS 0x41dfb9, SHIELD@0x41dfe9, ROBNUMS@0x41dff9,
  DIGITS 0x41e039, SMOKER 0x41e059 (string→cell map per 7j.30/D94).
  TINYFONT's buffer [0x46cdb0] = ArenaAlloc(0x189c) @0x41d62f inside
  FUN_0041d4e9 (called from the MissionShell family), loaded by its
  own GAMEGFX site in the same pass family.

**Verdict: NO mission-type gate, NO MP gate, NO boot-only subtlety —
the three banks are allocated and loaded at EVERY MissionShell entry
before the first entity loop can run.** A SP frame can never observe
any of these cells == 0. (The 7j.30 "GAMEGFX load-always" phrasing is
precise: load-always per mission.)

### 3. ROBNUMS.BIN is DEAD DATA — the plates draw TINYFONT glyphs

Full-binary census of the three cells:
- [0x46af38] TELEPORT: readers 0x403e62 (robot teleport beam) +
  0x4051fc/0x4056ae/0x405c11/0x405f3f (§5e platform/blast loops) +
  0x4066d2 (7j.21 arrival marker) + 0x41df9e (its own load). LIVE.
- [0x46af44] SHIELD: readers 0x403f1a (robot shield sprite) +
  0x41dfee (its own load). LIVE.
- [0x46af48] ROBNUMS: readers 0x41dffe ONLY — its own LoadFile site.
  **ZERO game readers. ROBNUMS.BIN (9 imgs, corpus) is staged and
  never drawn: dead data** (a left-over from a cut feature — the
  bank name says the digits were meant to be robot-number plates).

The actual name-plate font is **TINYFONT = [0x46cdb0]** (118 glyphs,
corpus u16 count; a 0x21-based font — glyph index = ASCII − 0x21):
readers 0x403c32 (the §5d plate enqueue), 0x408fe6/0x40907a
(FUN_004089b1 map-overlay markers) and 0x423cd9/0x423cf7 (sidebar
text family) — the same shared tiny-font draw.

### 4. The full MP name-plate grammar

Per visible+alive robot, AFTER the five sprite enqueues (asm
0x403fb9..0x403c5d):

- GATE: `cmp [0x4edb88],0; je skip` @0x403fb9 — the plate loop runs
  only when the mode cell ≠ 0 (any MP/demo variant); SP NEVER draws
  plates. (The ==2 arm at 0x403c62 is a DIFFERENT consumer: the MP
  hot-rect record 0x4787c4 write for the selected robot — 7j.31.)
- For i = 0.. while i < strlen(name_id): glyph byte g = name
  storage `[0x4e4458 + id*9 + i]`; skip the glyph if g > 0x40 (the
  `test/jl` arm @0x403c1b is dead — g is zero-extended); enqueue
  TINYFONT frame = g at `x = sx + u32[0x4e44c8 + id*4] + 6*i`
  (running x accumulates +6 per char @0x403bb6), same sy, mode 0x12c
  (plain raw copy — no palette compose).
- id = robot array index (the loop counter over the 0x4c69e4 array,
  high half of the i32@+0x28 slot read @0x403bd7).
- **0x4e44c8 is the id-indexed CENTERING table, NOT per-char**:
  writer loop 0x447ce0..0x447d85 inside MissionShell:
  memset(slot,0,9) @0x402965; copy raw name chars while source
  `[0x4e43e0 + id*9 + i] >= 0x20` (max 8), storing
  `toupper(c) − 0x21` (FUN_0044f067 = toupper, a..z → −0x20;
  store @0x447cf7); then `u32[0x4e44c8+id*4] = 0x20 −
  (strlen*6)>>1` = **32 − 3·strlen** (0x447d1b..0x447d45) — the
  half-width centering offset so the plate centers on the robot.
  So the stored table IS the glyph-index form (A..Z → 0x20..0x39,
  0..9 → 0x0F..0x18, space → 0) and the ≤0x40 filter passes every
  glyph the toupper grammar can produce (it only trims upper(c) >
  0x61, i.e. punctuation TINYFONT lacks).
- Raw names live at 0x4e43e0 (9 B/slot, [0x46cbe0] slots — writers
  0x43c607/0x442b0b/0x4475fd/0x44853e = the name-entry/roster
  family); the glyph table 0x4e4458 + centering 0x4e44c8 are rebuilt
  from them at every MissionShell entry (loop gated only on the
  roster count — built in SP too, harmlessly, since the DRAW is the
  gated half).

### 5. The unstaged-flush question — CLOSED BY NEVER-OBSERVABLE

The Backlog clause "nodes enqueue, flush skips while unstaged" is
RETIRED: there is NO unstaged-skip ANYWHERE in the pair.
- Enqueue FUN_0040798e (0x40798e..): the ONLY early-outs are bx/by
  < 0 (0x4079ed/0x4079f5); the bank pointer is stored into the node
  (+4) with no zero test.
- Flush FUN_0040179b (0x40179b..): mode dispatch first (0x130/0x12c/
  0x12d/0x12e/0x12f, any other mode → RET @0x4017e0 — the ONLY
  skip in the pair, mode-based not bank-based); every drawn mode
  immediately derefs ESI = node.bank (`and eax,0xfff; … add esi;
  mov eax,[esi]` @0x4017e1+) with NO null check.
So an unstaged bank would fault/garbage-read, not skip — and per
§7j.48/2 it can never happen in shipped play (the banks load before
the first frame). **E-side consequence: the renderer needs no
unstaged-skip logic; staging may stay lazy per bank, the seam is
unobservable because bank==0 never reaches the flush in the
original.** No new watch rows (the plate glyphs are MP-only; the SP
chains never touch them; a future MP scenario would compare TINYFONT
glyph rows through the existing sidebar-text family pins).

## 7j.49. THE FUN_00440dc2 IDENTITY — CLOSED: the BRIEF-screen OBJECTIVE-MINIMAP SNAPSHOTTER (per-objective tile-window render + 2× downsample into a 256×256 cache); the whole FUN_00440a2d/FUN_00440c34 family is BRIEF-ONLY — the 7j.26 "scroll/camera restamp" in-game gloss is CORRECTED (2026-08-23, worker 21c18e9e claim 2; objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra run; read-only DGROUP string probes + a raw-dword pointer scan of BEDLAM.EXW; manifest clean before AND after)

Closes the Backlog "REMAINS open slim: FUN_00440dc2's own identity"
clause (the 7j.26 residue). All [verified] asm/corpus-string unless
tagged.

1. **Caller census — COMPLETE, closed by instruction + data scan.**
   FUN_00440dc2 has EXACTLY ONE call site: 0x43dfb3, inside
   FUN_0043dc65 (0x43dc65..0x43e183) = the per-OBJECTIVE brief panel
   renderer (see item 3). A raw little-endian dword scan of the whole
   EXW file for 0x00440dc2 (and 0x00440a2d, 0x00440c34) returns ZERO
   hits — no jump-table/function-pointer references exist. The call
   graph is a strict closed trio, each edge with exactly one site:
   FUN_0043dc65 →(0x43dfb3)→ FUN_00440dc2 →(0x440de7)→ FUN_00440a2d
   and →(0x440dec)→ FUN_00440c34. Nothing else invokes any of them.
2. **The shared-epilogue tail-jumps DECODED** (kills the "jmp into
   the caller" red flag that kept this open): 0x43c801 is a
   MULTI-ENTRY shared epilogue gadget — `pop ebp; pop edi; pop esi;
   pop edx; pop ecx; pop ebx; ret` (6-pop variant); entering at
   0x43c802 skips the `pop ebp` (5-pop variant); 0x43f49e is the
   5-pop variant inside FUN_0043f49b. FUN_00440dc2 (pushes
   ebx,ecx,edx,esi,edi = 5 regs) RETURNS NORMALLY to 0x43dfb8 via
   the 0x43c802 entry; FUN_00440c34 (6 pushes + 0x28 locals,
   `add esp,0x28` @0x440dba) returns via 0x43c801; FUN_00440a2d
   (5 pushes + 0x30 locals + 2 arg saves, `add esp,0x38` @0x440c2c)
   returns via 0x43f49e. All three are ordinary call/return
   functions; the many `jmp 0x43c801/0x43c802` sites across
   0x43c8..0x4472 are other functions sharing the gadget (classic
   Watcom -ox shared epilogue).
3. **The screen context: FUN_0043d00b = the MISSION BRIEF screen**
   (called from GameMain 0x41c4d5 after the zone/mission table walk
   0x4decb2; returns 2 = "launch" → [0x46ae74] := mission). Its init
   (0x43d00b..) allocates the screen's OWN buffers — notably
   [0x46cbb0] := alloc(0x10100) (256×256 + 256 guard) @0x43d06b..7c,
   [0x4ede18] := alloc(0x64000) backbuffer @0x43d0ae..b8, the
   49×12 restamp list [0x4ede24] := alloc(0x24c) @0x43d0bd..c7,
   [0x4edd58] DAT @0x43d0cc..d6 — and zeroes the objective bank
   (0x150 B @0x4e9628 @0x43d3be..c8) + the cache flag [0x4dc6c0] := 0
   @0x43d023. **[0x4ede18]/[0x4ede24] are per-screen cell reuses**:
   the mission screen reallocates its own (0x64000 backbuffer via
   FUN_0041dc5a install, 1296×12 viewport cache via FUN_0041d954) —
   the two screens never share an allocation.
4. **The objective record bank @0x4e9628 (24 × 14 B) grammar**
   [verified writers 0x43e5b1/0x43e62c/0x43e6ac/0x43e71c/0x43e7b2]:
   +0/+2 = marker map x/y (words), +4 = TOT ROW, +6 = TOT COL (the
   snapshot window origin), +8 = a per-objective counter (FUN_0043f5b1
   tick @0x43f5df), +0xA = render-current latch. Staged by the BRIEF
   text parser from the stream cursor [0x46cbb8] (bounded by
   [0x46cbb4]+0x13c68) via atoi FUN_0044effa; the 0x4e7ed8/0x4e8378
   0x4a0-byte tables are its text staging. The panel name is built
   from strings 0x4592b6 `OBJECTIVE_` + 0x4592c1 `%02i` with zone
   [0x4edd8c]+0x40 and mission [0x4edd88]+0x30 (0x43dcf7..0x43dd7e).
5. **FUN_0043dc65 = the objective panel renderer**: entry gate
   word@[record+0xA] ≠ 0 → return (0x43dc79); else zero ALL 24 latch
   words (0x43dc9c loop) then set its own := 1 (0x43dcc3) — so each
   render INVALIDATES the other objectives' latches: per BRIEF frame,
   the walk at 0x43d860 (records with +0/+2 ≠ 0 within ±7 of the map
   camera [0x4eddc4]/[0x4eddc8]) re-renders every active objective in
   index order, and non-selected ones also blit map markers + play a
   reveal beep via FUN_0043a48e on the 0x4edfc8/0x4edfd8 voice cells.
   After the snapshot (below) it clamps/places the label panel
   (x ≤ 0x26c, y ≤ 0x140 — 0x43dfd7..0x43e01f).
6. **FUN_00440dc2 body (EAX = record index) — the snapshotter:**
   a. EAX := record word@+4 (TOT row), EDX := word@+6 (TOT col)
      (dword reads sar 16 @0x440dce/0x440dd5).
   b. `mov ecx,0x10000` @0x440de2 is NOT a stager argument (FUN_00440a2d
      clobbers ECX at 0x440a43) — it is the PRE-SET ECX for the later
      zero-fill (Watcom scheduling), surviving both calls because both
      callees push/pop ECX.
   c. call FUN_00440a2d(row, col): (i) stage the restamp list — up to
      49 records {abs dest = [0x4ede18] + y·0x280 + x, tile-row,
      tile-col} over the 7×7 iso walk from screen (0x100,0x100), kept
      iff 0≤x<0x240 ∧ 0≤y<0x240 (0x440a37..0x440acd, count
      [0x4ede28]); (ii) materialize TOT→mirror for the 7×7×8 window
      at (row,col): mirror word@0x4796bc tile·0x1E+z·2, seen@0x4796cc
      (word≠0 ∧ DAT byte==0) — the 7j.16 mechanism, screen context
      corrected here; (iii) **ZERO the ENTIRE 0x64000 backbuffer**
      (0x440c1c: FUN_00402965(0x64000, [0x4ede18])) — the clean slate
      for the snapshot.
   d. call FUN_00440c34 = the DRAWER (this function, not FUN_00440dc2,
      owns the 7j.36-pinned sites 0x440d1c/0x440d93): walks the
      restamp list; per record, per z ∈ 0..7: mirror word =
      [0x4796bc + tile·0x1E + z·2]; z=0 draws on word≠0 (0x440ceb),
      z≥1 needs seen ∧ word≠0 (0x440d5f/0x440d73); dest = record.dest
      − z·0x5000 — each z level draws 0x5000 B = 32 pixel rows HIGHER
      (the iso z-height step), bounds-checked into
      [backbuffer, backbuffer+0x5a000) — then FUN_00401471(word,
      bank=[0x4ede1c], EBX=0, EDI=dest).
   e. zero 0x10000 B at [0x46cbb0] (FUN_00402965 with ECX from b).
   f. **the 2× DOWNSAMPLE**: 256 rows × 256 bytes,
      dest[r·256+c] = backbuffer[(64+2r)·0x280 + 64 + 2c] — source
      base +0xa040 = pixel (64,64), row pitch 0x500 = 2·0x280, byte
      stride 2 (0x440e02..0x440e34) — a plain 2× subsample (no
      deshear) of the 512×512-pixel window into the packed cache.
   g. [0x4dc6c0] := 1 @0x440e36 — the "minimap cached" flag (sole
      other writer: the BRIEF-init reset 0x43d023).
   h. return via the 5-pop gadget 0x43c802 → 0x43dfb8.
7. **The consumer** (same frame, after the walk): 0x43d9a2 — if
   [0x4dc6c0] ≠ 0: FUN_00402a28(EDI = panel buffer [esp+0x624]
   (alloc'd @0x43d5cf), ESI = [0x46cbb0]) = the 256×256 TRANSPARENT
   palette-remapped blit (source byte 0 → skip; else
   [0x4edbfc]-LUT remap; dest pitch 0x280). Net effect: the panel
   shows the snapshot of the LAST active objective rendered this
   frame — consistent with the ±7 camera window normally holding the
   current objective.
8. **(c) mid-frame / terrain-pass ordering — CLOSED BY SCREEN
   LIFECYCLE.** FUN_00440dc2 NEVER runs during the mission render
   pass: FUN_00403938 is called only from the MissionShell region
   (0x447c9b/0x448094); FUN_0043d00b never calls it. The BRIEF screen
   owns a SEPARATE [0x4ede18] allocation, so the full-buffer zero in
   FUN_00440a2d(iii) cannot wipe a mission frame, and the "can it run
   mid-frame vs the terrain pass" question (MISSIONVIEW §1) is
   answered: there is no in-game path at all. **7j.26 gloss
   CORRECTED**: FUN_00440a2d is NOT the in-game scroll/camera
   restamper — it is the BRIEF minimap window stager; in-game
   scrolling is the 36×36 viewport-cache walk in FUN_00403938's
   terrain loop, a different structure that merely REUSES the
   [0x4ede24]/[0x4ede28] cells (BRIEF: 49×12 restamp list; mission:
   1296×12 viewport cache). Same correction applies to the 7j.36
   cluster-(b) gloss "scroll/camera RESTAMP DRAWER" (→ BRIEF minimap
   drawer) and the §7j.16 lead "materializer caller — scroll
   restamp?" (→ objective snapshotter).
9. Engine consequence: NONE (docs-only — the BRIEF screen is outside
   the P4 mission-diff scope; the cells 0x46cbb0/0x4dc6c0/0x4e9628
   are BRIEF-lifecycle only). No new watch rows; no E-side work.

## 7j.50. THE FUN_00419aff ELSE-PATH DUMP + THE PROJECTILE-0x69 VERDICT — CLOSED: the per-level BEAM column re-keys its damage query to the LITERAL 0x65 (never consults the table at its own id, never the else); the (d+1)·300 damage belongs to the TRT-bolt state 0x66 alone; + the complete 0x4cc654-bank producer/impact census (2026-08-23, worker 6bb948aa claim 2, D122; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read)

Closes the §7j.18 low-priority residue "projectile type 0x69 vs the
FUN_00419aff damage table — NOT folded (would need the damage-table
else-path dump)". All addresses below are instruction-exact from the
full-.text objdump.

1. **FUN_00419aff ELSE PATH — fully dumped.** There is NO memory
   table: the resolver is a compiled binary jump tree (base/stride:
   N/A), reached by `cmp` chains at 0x419b0a/0x419b15/0x419b26/
   0x419b2f/0x419b45/0x419b4e and 0x419c14..0x419c4a/0x419c55..72.
   `EAX := 1` is pre-set at entry (0x419b05); the ELSE is the plain
   fall-through returning it (stubs 0x419b57 [w>0x68], 0x419c2c
   [w=0x2A..0x64], 0x419c50 [w=0xE..0x19], 0x419c5e [w=6..11]) plus
   TWO shared-epilogue arms carrying the default (w<2 and
   0x1B≤w<0x24 → `jb/jne 0x418aa1`). **0x418aa1 is a Watcom
   cross-function shared-epilogue gadget** (0x418aa1..0x418aa5 =
   `pop esi/edx/ecx/ebx; ret` — pops EXACTLY FUN_00419aff's own
   four entry pushes; same gadget family as the 7j.49 multi-entry
   epilogues): FIVE branch arms jump there — the two default-EAX
   ones above plus the three difficulty arms below. The full key
   census re-verified instruction-exact (the §7j.17 table stands):
   | w | damage | arm |
   |---|---|---|
   | <2 | 1 (else) | 0x419c66→0x418aa1 |
   | 2/3/4/5 | 20/30/40/75 | 0x419c72/0x419c63+0x419c72/0x419b66/0x419b70 |
   | 6..11 | 1 (else) | 0x419c5e |
   | 0xC / 0xD | 5000 / 312 | 0x419b8e / 0x419b98 |
   | 0xE..0x19 | 1 (else) | 0x419c50 |
   | 0x1A | 75 | 0x419b70 |
   | 0x1B..0x23 | 1 (else) | 0x419c17→0x418aa1 |
   | 0x24 | 400 | 0x419b7a |
   | 0x25..0x28 | 1 (else) | 0x419c2c |
   | 0x29 | 250 | 0x419b84 |
   | 0x2A..0x64 | 1 (else) | 0x419c2c |
   | 0x65 | 50·(d+1); d=2 → +50 = 200 | 0x419bcc (d≠2 → 0x418aa1) |
   | 0x66 | 300·(d+1); d=2 → +300 = 1200 | 0x419ba2 (d≠2 → 0x418aa1) |
   | 0x67/0x68 | 75·(d+1); d=2 → 75·3+75 = 300 | 0x419bf1/0x419c07 (d≠2 → 0x418aa1) |
   | ≥0x69 | 1 (else) | 0x419b57 |
   (d = difficulty [0x46cbf8]; the d=2 legs add a flat constant via
   the `ADD`/`LEA` idioms at 0x419be9/0x419bc2/0x419c00; the 75·(d+1)
   product is staged in EBX at 0x419b34..0x419b40 — EBX clobber
   noted because ECX carries the d=2 constant 75·(d+1)+0x4B.)
   **FUN_00419aff(0x69) would return 1** — but no caller ever asks
   (point 4).
2. **The 0x4cc654-bank STATE-word census (complete, 25 sites).**
   Every `mov` touching a state-word address 0x4cc654+k·0x22:
   4 readers (0x404d75 the §7j.28 draw walk, 0x412021 the tick
   dispatch, 0x4126ea the disburser, 0x41980f the robot-hit walker),
   12 zero-writes (7 tick deaths 0x412079/096/1eb/2cc/42a/479/490 +
   5 disburser deaths 0x412741/778/7af/7e6/81d), and exactly
   **FIVE producers**:
   | state | producer site | family |
   |---|---|---|
   | 0x65 | 0x41540e (k2 sine-walk shooter; range gate 0x12C−(2−d)·0x40) | critter bolt (ballistic vx/vy) |
   | 0x66 | 0x417a5c (FUN_00417698 TRT fire routine; `[eax*2+0x4cc654]` scaled form) | TRT structure bolt |
   | 0x67 | 0x414b79 (k3 chase, octile-aimed) | critter bolt |
   | 0x68 | 0x413def (k5/6 ENGAGE, octile-aimed, vz set) | critter bolt |
   | 0x69 | 0x4135a2 (k7 close-combat; §7j.16's fire-rate gates 32/16/8 frames) | the BEAM column |
   (The earlier "0x66 has no producer" reading was a grep-truncation
   artifact — the 0x417a5c site uses the scaled-index operand form.)
3. **The tick dispatch (FUN_00412010 head)**: state−0x65 bounds 0..4
   (0x412021..0x41203a) → jump table 0x411ffc {0x65→0x41216b mover,
   0x66→0x412307 guided stepper, 0x67/0x68→0x41224c shared ballistic,
   0x69→0x412042 the beam}. Record layout re-confirmed: u16 state@+0,
   x@+2, y@+6, z@+0xA, vx@+0xE, vy@+0x12, vz@+0x16, +0x1A counter,
   +0x1E TTL (dwords, Q13/Q8 as §7j.13).
4. **THE 0x69 VERDICT (the queue question).** The per-level BEAM
   column is a 0x4cc654-bank STATE, not a 400×0x36 weapon type. Its
   handler (0x412042) NEVER calls FUN_00419aff with its own id:
   - per frame: TTL@+0x1E −− (0x412085; spawn value 0x18; at 0 the
     record dies SILENTLY — state := 0, no debris); counter@+0x1A
     k := min(k+1,7); terrain probe FUN_0041eaa1(x>>8, y>>8,
     (z−k)·0x20) — the column DESCENDS one level per frame;
   - on contact at level z−k: k −− (0x4120e9 — the probe level is
     RE-TESTED next frame ⇒ the blocked level takes damage EVERY
     FRAME while the TTL lasts), RandA±7-spread debris kind 0x14 via
     FUN_00420608 (0x412102..0x412153), then **`mov eax,0x65`**
     (0x41215a) → FUN_00419aff → FUN_0041a894(x, y, damage, score
     flag 0) at the beam's (x,y) — the record does NOT die, does NOT
     call FUN_004126dc;
   - **damage = the 0x65 row: 50/100/200 by d** — through the table,
     at a DIFFERENT key, never the else. The §7j.16 k7 note
     "(→ 'else 1')" is CORRECTED. The "(d+1)·300 as type 0x66"
     hypothesis is REFUTED for 0x69: that damage belongs to the TRT
     bolt alone (point 5).
   - **0x69 NEVER damages robots**: FUN_004197d4 (the projectile-vs-
     robot proximity walker, |dx|<0x10 ∧ |dy|<0x10 ∧ |dz|<0x20 Q8)
     admits states 0x65/0x67/0x68 ONLY (0x419816..0x419836: 0x66
     falls in the `cmp ax,0x67; jb` skip, 0x69 in the final
     `jmp 0x4198b7` skip). The beam is terrain-only.
5. **The per-state IMPACT-KEY map (supersedes the §7j.13/§7j.14
   "reads the projectile's own type" gloss — only the 0x67/0x68
   terrain leg self-keys):**
   | state | terrain/object key (FUN_0041a894) | robot key (FUN_0040e230 via FUN_004197d4) |
   |---|---|---|
   | 0x65 | literal 0x65 @0x412211 + disburser K0x14 | literal 0x65 @0x41989f |
   | 0x66 | literal 0x66 @0x412449 (class-2 contact; + 0x41bc1c) + disburser K8 | NEVER (filtered) |
   | 0x67/0x68 | OWN state word (the `[+0x4cc652]>>16` dword trick @0x4122f7/0x41992d) + disburser K4 | OWN state word @0x41992d |
   | 0x69 | literal 0x65 @0x41215a, per-frame, no disburser, no death | NEVER (filtered) |
   So a robot hit by a 0x67/0x68 bolt takes 75/150/300, by a 0x65
   bolt 50/100/200; the TRT 0x66 bolt NEVER hits robots at all
   (consistent with its §7j.28 loop-next invisibility — the heavy
   (d+1)·300 key exists ONLY on its terrain contact).
6. **The state-0x66 handler decoded (0x412307)** — a GUIDED STEPPER,
   ≤10 substeps/frame (0x412351..0x4123a1): per substep x+=vx, y+=vy;
   out-of-bounds → contact class 1 (silent die 0x41248e);
   FUN_00419756(x,y)≠0 → class 3 (disburser + die 0x41241f);
   vz@+0x16 ≠ 0 → break (0x4123f3 vz−−); else height probe
   FUN_0041e231(x>>8,y>>8) > z>>8 → class 2 (disburser + damage
   key 0x66 + 0x41a894 + 0x41bc1c + die 0x412436..0x41247f). The
   write-back reverts the last substep (0x4123ff..0x412411). The
   §7j.13 "type-1/2/3" arms are these contact CLASSES of the 0x66
   handler (1/2/3) — relabeled here by state id. [IDENTITY CLOSED
   §7j.51: the probe takes (x,y,z) — all three args — and is the
   first-alive ROBOT-BANK OCCUPANCY BOX; the "vz ≠ 0 → break" leg
   only SKIPS the height probe (substeps continue); the §7j.16
   spawn vz 0x14 = a ~2-frame terrain-arming delay, occupancy
   tested every substep]
7. **FUN_004126dc disburser — 0x69 arm pinned**: the kind switch
   (state 1→K2 @0x412716, 0x65→K0x14 @0x41274d, 0x66→K8 @0x4127f2,
   0x67→K4 @0x412784, 0x68→K4 @0x4127bb) falls through to the shared
   epilogue `jmp 0x411ff3` for 0x69 (0x412711) — NO debris, NO state
   clear; defensive only (the beam handler never calls the
   disburser). The §7j.14 row gains the 0x69 arm.
8. Caller census cross-check: 29 `call 0x419aff` sites total
   (§7j.17's 28 + the 0x41215a beam site), across FUN_00410823
   (the 400-bank fire/tick controller), FUN_004190bc-family stat
   readers, FUN_00412010 ×5 (0x41215a/0x412211/0x412218+0x4122f7
   entry/0x412449/0x412462 — was ×4), FUN_004197d4 ×2
   (0x41989f/0x4198a9). NONE passes 0x69.
9. Engine consequence: the E-side critter k7 close-combat leg (when
   it lands) must model the beam as a PERSISTENT per-frame terrain
   DoT keyed 0x65 with the k-oscillation (re-damage the blocked
   level every frame, TTL 24) and must NOT damage robots; the 0x66
   TRT bolt is terrain-only damage 300/600/1200. Docs-only unit —
   no code, no watch rows (the 0x4cc654 bank is T2-class).

## 7j.51. THE FUN_00419756 IDENTITY — CLOSED: the TRT-bolt class-3 probe is a first-alive ROBOT-BANK OCCUPANCY BOX (±<0.5 tile lateral, ±<1 level z — NOT octile, NOT critters, NOT TRT structures, NOT tile words); the class-3 death is the "hit an actor, ZERO damage of any kind" leg (2026-08-23, worker 9a23356a claim 2, D123; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read) [verified]

**2026-09-06 correction:** “ZERO damage of any kind” in this historical
heading is limited to direct projectile damage. The K8 disburser causes
secondary debris damage and knockback; see RE-EXW-SENTRIES.md, “Secondary
impact damage”, for the call-chain anchors and live observation.


1. **The body (126 B @0x419756..0x4197d3, instruction-exact).**
   `FUN_00419756(x Q13 EAX, y Q13 EDX, z Q13 EBX) → 0/1`:
   - prologue shelves the args: esi := x>>8, edi := y>>8, ebp :=
     z>>8 (three `sar 0x8` @0x419762..0x419768), ecx := robot
     index 0, ebx := bank byte offset 0 (Watcom wccc — the
     `push ecx` @0x419756 is the alignment local, popped in the
     epilogue);
   - the loop `cmp ecx,ds:0x46ccbc; jge miss` (@0x419776) walks
     rec := 0x4c69e4 + 0xA8·ecx:
     - `cmp [ebx+0x4c6a60],0; je next` (@0x41977e) — ALIVE gate
       (+0x7C); dead slots never block;
     - `|(rec.x@+0)>>8 − (x>>8)| ≥ 0x10` → next (abs via
       cdq/xor/sub @0x41978d..0x41979a);
     - `|(rec.y@+4)>>8 − (y>>8)| ≥ 0x10` → next (0x4197a2..0x4197af);
     - `|rec.z@+8 (RAW) − (z>>8)| ≥ 0x20` → next (0x4197b1..0x4197c1);
     - all three pass → `mov eax,1` + epilogue (@0x4197c3);
   - miss epilogue `xor eax,eax` + return (@0x4197cd).
   FIRST match in bank order wins (lowest index) — it is a presence
   predicate, not a nearest-scan (no distance returned).
2. **The box geometry is scale-matched, not quirky.** All three axes
   normalize to Q5 (32 units per tile/level): probe x/y are Q13
   (tile·0x2000) so `>>8` lands at 1/32-tile units, threshold 0x10
   = less than half a tile; robot z@+8 is STORED Q5 (§3 row +0x08,
   clamped 0..0xFF = 0..~8 levels) so its RAW read is already in
   the probe's `z>>8` scale — threshold 0x20 = less than one z
   level. The apparent asymmetry (no `>>8` on the robot z) is the
   scale match. FUN_004197d4's robot lane uses the IDENTICAL box
   (|Δ(x>>8)|<0x10 @0x419856, |Δ(y>>8)|<0x10 @0x419876,
   |z@+8 raw − proj.z@+0x4cc65e>>8|<0x20 @0x419893) — the §7j.13
   item-4 walker and this probe share one geometry. NOT an octile
   test (no FUN_0041ebf8 call; plain per-axis abs compares).
3. **What it is NOT (the queue's four candidates).** Not the
   critter bank 0x4cff98 (zero refs in the body), not TRT
   structures (the bolt's structure damage rides the class-2 height
   probe FUN_0041e231 + FUN_0041a894/FUN_0041bc1c, §7j.50/6), not a
   tile-word test (no TOT/DAT/mirror reads). Caller census: EXACTLY
   ONE call site 0x4123ae (exw-functions 1-caller + full-objdump
   grep: only the definition @0x419756 + that call; no jump-table
   refs).
4. **Caller context refined (§7j.50/6 amendment, two gloss fixes).**
   The state-0x66 guided stepper tests per substep in THIS order:
   x+=vx ∧ y+=vy → x/y bounds vs [0x4eddec]/[0x4eddf0]<<0xd and z
   bounds 0 ≤ z < 0x10000 (z = the record's UNSTEPPED z@+0x4cc65e,
   read once at handler entry into [esp+0x24] @0x412315 — the bolt
   never moves in z) → contact class 1; **FUN_00419756(x, y, z)** —
   the §7j.50/6 "(x,y)" gloss corrected: ALL THREE args are passed
   (ebx = z @0x412391) → class 3; THEN vz@+0x16 ≠ 0 → vz−− and
   SKIP the height probe only (@0x4123c0..0x4123fa — the §7j.50/6
   "break" gloss corrected: the substep loop CONTINUES, only the
   terrain test is skipped; the §7j.16 spawn value +0x16 = 0x14
   thus reads as a 20-substep (~2-frame) terrain-arming delay,
   while the OCCUPANCY probe runs unconditionally every substep);
   else FUN_0041e231(x>>8,y>>8) > z>>8 → class 2. The write-back
   (0x4123ff..0x412411) reverts the contact substep BEFORE the
   class dispatch, so the class-3 debris spawns at the pre-contact
   position.
5. **THE VERDICT (the queue's residual question).** Class 3 IS the
   "hit an actor but no robot damage" leg — CONFIRMED, and
   stronger: the class-3 path performs NO damage query of ANY kind.
   0x41241f: `FUN_004126dc(slot)` (the disburser reads the record's
   own state 0x66 → debris kind 8, §7j.50/7) then state := 0
   (@0x41242a, belt-and-braces beside the disburser's own clear) —
   NO FUN_00419aff, NO FUN_0041a894/0x41bc1c, NO FUN_0040e230 on
   the path. What the TRT bolt interacts with: ALIVE ROBOTS are a
   pure BLOCKER — the bolt stops at the first robot box it enters
   and dies as cosmetic kind-8 debris; its heavy (d+1)·300 damage
   is EXCLUSIVELY the class-2 terrain/structure contact. This
   closes the §7j.50 residual ("0x66 never damages robots — what
   DOES the bolt interact with"): the squad — as an obstruction,
   never as a target.
6. **Engine consequence: NONE today** (docs-only; the 0x4cc654 bank
   is T2-class, no watch rows). When the E-side TRT fire routine
   (the FUN_00417698 producer family) lands, its stepper must
   reproduce the robot-blocker box verbatim — without it a bolt
   would fly through the squad and detonate on the terrain behind
   it (a death-POSITION divergence: class 3 vs class 2 at a
   different tile); the class-3 death must spawn kind-8 debris at
   the post-revert pre-contact position with zero damage, and the
   probe is first-match-in-bank-order (lowest robot index blocks).

## 7j.52. THE DEBRIS ARRIVAL-SFX PAIR — CLOSED: FUN_00421e60 = the BOOM1/2/3 spawn trio (RandB()%3, play priority 2), FUN_00421dec = the RICOCHT1..4 quad (RandB()&3, priority 1); both fire at STAGE time on in-map bounds alone (k11 alone adds a RandA&1 ~50% play gate); the bank-pick draw is RandB — item 4's "RandA()%3" CORRECTED; corpus-reachable today ONLY via the k5 damage-death leg (2026-08-23, worker a553aa84 claim 2, D124; objdump-only from ghidra-project/exw-text-objdump.txt + one raw-dword scan of BEDLAM.EXW — no Ghidra run; manifest clean before AND after) [verified]

1. **The two bodies, instruction-exact.**
   - `FUN_00421e60(x Q5 EAX, y Q5 EDX)` (118 B @0x421e60..0x421ed5):
     shelves y→ebp, spills x to stack; gate `[0x4ede58]==0` →
     shared-family epilogue `jmp 0x41dc51` (add esp,4; pop
     ebp/edi/esi/ecx/ebx; ret — the §7j.49-style Watcom shared
     gadget); else `call FUN_004029b6` (**RandB**, state word
     0x4ede4c) then signed `idiv 3`: rem 0 → cell ds:0x4edf64,
     rem 1 → 0x4edf68, rem 2 → 0x4edf6c; play
     `FUN_0043a48e(handle=eax, 0, x EBX, y ECX, priority 2)` at
     the debris position. The rem-2 leg re-pushes the remainder
     itself as the priority arg (value 2 on that path — Watcom
     value reuse, identical effect to the literal `push 2` of
     the other two legs).
   - `FUN_00421dec(x Q5 EAX, y Q5 EDX)` (116 B @0x421dec..0x421e5f):
     same prologue/gate/epilogue shape (x→ebp, y spilled);
     **RandB()&3** → jump table @0x421ddc {0x421e07, 0x421e20,
     0x421e2d, 0x421e3a} → cells 0x4edf98 / 0x4edf9c / 0x4edfa0 /
     0x4edfa4; play priority **1** — one steal class BELOW the
     BOOM trio (§7j.30: steal by priority + age). METHOD NOTE:
     the flat objdump's linear pass MISPARSES 0x421dd9..0x421deb
     (the 16 B table + 3 B entry padding) — the table bytes were
     decoded from the raw stream (cf. §7j.46's table-farm note).
   - **Every cell named** (§7j.30 anchor, the queue's ask):
     0x4edf64 **BOOM1**, 0x4edf68 **BOOM2**, 0x4edf6c **BOOM3**
     (the 3-way); 0x4edf98 **RICOCHT1**, 0x4edf9c **RICOCHT2**,
     0x4edfa0 **RICOCHT3**, 0x4edfa4 **RICOCHT4** (the 4-way).
2. **THE RNG CORRECTION (the one prior-text fix this unit).**
   §7j.11 item 4 says the 3-way "picks RandA()%3" — WRONG draw
   identity: both bodies call **FUN_004029b6 = RandB** (@0x421ea9
   and @0x421e47), exactly like the sibling trios FUN_00421fc2 /
   FUN_00421f4c (§7j.23/§7j.24, both already documented as
   "RandB()%3"). RandA = FUN_00402975 (state 0x4ede48) is drawn
   ONLY by k11's local play gate (item 4's second sentence is
   correct). The twins are byte-identical 16-bit LCGs (add tail
   0x62E9/0x3619) over adjacent state words 0x4ede48 / 0x4ede4c.
   Differ consequence: the bank pick is a **T4 draw** (unmodeled —
   the destroy-tail census language "RandB feeds only the T4 SFX
   bank pick" now covers this pair too); k11's gate is a RandA
   draw (modeled draw-count — relevant only if k11 ever gains a
   corpus producer).
3. **The 13 call sites — all inside FUN_00420608 kind legs; the
   trigger is STAGE time, not a landing tick.** e60 ×11: k16
   0x4206ed, k17 0x4207a1, k18 0x42084e, k19 0x4208f6, k20
   0x42099e, k6+12 0x420cfb, k9 0x42101a, k1(+13/14/15 shared
   body) 0x4212d7, k5 0x421364, k4 0x4218ae, k11 0x420e93; dec
   ×2: k2 0x421619, k8 0x421762 (kind→leg re-verified against
   the 20-entry jump table @0x4205b8, byte-exact incl. the 6+12
   and 1+13/14/15 body sharing). TWELVE of 13 sites share ONE
   trigger shape: a per-leg in-map bounds RECHECK of the raw Q5
   args (x≥0, y≥0, x < [0x4eddec]<<5, y < [0x4eddf0]<<5 — width/
   height words; fail → 0x421dd0 `ret 8`, NO record written, NO
   SFX) then the UNCONDITIONAL call with (x, y) — the sound fires
   at debris-CREATION, before the record fields are written (k16:
   call @0x4206ed, record writes from 0x4206f2). "Arrival SFX" =
   the debris' arrival ON THE FIELD. The sole local gate: k11
   (RandA @0x420e82 → `test al,1` @0x420e87 → je skip) — a ~50%
   play chance; that leg draws TWO different RNGs (RandA gate +
   the RandB pick inside the callee). Caller-census completeness:
   raw little-endian dword scan of BEDLAM.EXW for 0x00421e60 /
   0x00421dec → ZERO hits (no jump-table or function-pointer
   refs) — the 13 direct calls are the entire graph.
4. **Corpus reachability (the queue's ask, re-anchored to the
   §7j.11 item-6 caller census).** The ONLY corpus-reachable
   debris producer today is **k5 via apply_damage** (FUN_0040e230
   death tail, call 0x40e771) — therefore the only reachable
   arrival-SFX site is **k5's FUN_00421e60 leg @0x421364**: every
   damage-death of a robot in a corpus scenario draws ONE RandB
   and plays one BOOM1/2/3 at the death position (priority 2)
   iff [0x4ede58]≠0. FUN_00421dec has NO corpus-reachable caller
   (k2/k8 producers are all in the weapon-fire/impact families,
   §7j.11 item 6 — outside the corpus path).
5. **Adjacent census (one line, not the pair):** a THIRD sibling
   FUN_00421ed6 (~0x421ed6..0x421f4b) = the GRUNT1/2/3 trio
   (RandB()%3 → cells 0x4ee000/0x4ee004/0x4ee008, priority 2,
   same shape/epilogue); callers 0x413ba0/0x413f2a (outside the
   debris family; zero raw-dword refs). The arrival-SFX family
   now has four decode-complete members: 0x421dec (RICOCHT 4-way
   p1), 0x421e60 (BOOM 3-way p2), 0x421ed6 (GRUNT 3-way p2), plus
   the §7j.23/24 twins 0x421fc2/0x421f4c (HURT/DEATH trios).
6. **Engine consequence: NONE today** (docs-only, no code, no
   watch rows — the cells already sit under the sfx-master-gate /
   SFX-register rows). When the E-side debris stager widens beyond
   k5 (backlog), each widening kind's staging must draw ONE RandB
   for the BOOM pick (T4, no chain effect) at the spawn position;
   k11 additionally consumes one RandA draw BEFORE the pick; the
   BOOM-vs-RICOCHT priority split (2 vs 1) is audible only
   through FUN_0043a48e's voice-steal order — no dump-visible
   state either way.

## 7j.53. THE FUN_004239ef SFX-MESSAGE DISPATCHER — DECODED WHOLE: it is the RADIO-WARNING system (4-channel speech+text message queue → per-frame drain → spoken WARNINGS line + on-screen typewriter history); the 53 ids = the [WARNINGS] records of LANGUAGE.*; ids 0xF/0x29 text-only, 0x19..0x1B channel-flush, take A/B = RandA bit0 (2026-08-23, worker d1578d5c claim 2, D125; objdump-only from ghidra-project/exw-text-objdump.txt + read-only corpus probes of BEDLAM.EXW DGROUP strings and all six LANGUAGE.* files — no Ghidra run; manifest clean before AND after) [verified]

The 17-site citation name "SFX-message dispatcher" is now
body-decoded (0x4239ef..0x423a84 whole; consumer
FUN_00423a85 0x423a85..0x423e18 whole; sole consumer caller =
MissionShell @0x447ff5, once per frame). FUN_004239ef is NOT a
beep-picker: every id it queues drives BOTH a spoken WARNINGS
line (speech bank, §7j.30's 53 {A,B} records) AND an on-screen
radio-warning text (the [WARNINGS] table). Channels 0/1/2 = the
three squad slots (UNIT 1/2/3), channel 3 = the system/HQ
channel (drained FIRST, priority over robots).

**PRODUCER FUN_004239ef(EAX id, EDX channel)** [verified]:
per-channel record @0x4eb954 + channel·0x28, 0x28-stride,
4 channels (0..3, cells 0x4eb954..0x4eb9f3):
- +0x00..+0x1C: EIGHT message words, value = id+1 (0 = empty)
- +0x20: insert index (post at slot[idx], idx++, wrap 8→0 —
  a full ring silently overwrites the oldest pending message)
- +0x24: current VOICE handle for the channel (consumer-side;
  0 = silent)
Scan 0x423a0e..0x423a19 first dedupes: if id+1 already queued in
THAT channel → return (one pending instance per id per channel).
Ids 0x19..0x1B (0x423a20..0x423a4c) = CHANNEL-FLUSH: zero all 8
words + insert index of the caller's own channel, THEN post at
slot 0 — i.e. "UNIT n IS TOAST" is a flush+announce (a dead
robot's pending warnings die with it). Queue + display ring are
zeroed at MissionShell entry (memset 0x4eb954 ×0xA0, 0x4ea13c
×0x98 @0x4479de/0x4479fc) — no other writers exist (full
traffic census of 0x4eb954/74/78: producer, consumer, this
reset only).

**CONSUMER FUN_00423a85 (the tick reader)** [verified]: walks
channels 3→2→1→0 (offset 0x78 decreasing by 0x28, 0x423c6a..c78;
channel 3 first = system warnings preempt robot chatter). Per
channel: scan the 8 slots OLDEST-FIRST (starts at the channel's
+0x20 insert index, 0x423c8c; ≤8 probes, first non-zero word →
id = word−1). If none → next channel. If found:
- VOICE LEG (skipped for ids 0xF and 0x29 — text-only
  continuation lines; and gated [0x4eb93c]≠0 (audio-system
  handle, writers 0x41d4db/0x425401/0x43a1b7) ∧ [0x4ede5c]≠0 ∧
  [0x4ede58]≠0 (the two speech-enable config cells, boot-zeroed
  0x41c15f; 0x4ede58 also gates the §7j.52 arrival-SFX pair)):
  if the channel's +0x24 handle ≠ 0, poll it
  (FUN_0044c5ac(handle−1): eax=0 ⇒ finished, edx = refreshed
  handle); still-playing ⇒ leave the message queued, next
  channel (the slot is the channel's "now playing" latch).
  Silent ⇒ start: take pick = **RandA (FUN_004029b6) bit0** —
  odd ∧ record.B ≠ 0 → the +4 word, else the +0 word of
  speech record id @0x4ee014+8·id (§7j.30's 53 {A,B} table,
  95 SPCH files); play direct via 0x44c8c4 (edx=0, ecx=0x7f00,
  ebx=0x10000 — the D94 speech bypass, vol 0x7f), handle :=
  ret+1 → +0x24. Each spoken line therefore consumes ONE RandA
  draw (T3/T4 budget class, same as every other SFX pick).
- CONSUME LEG (runs when the voice finished, never started, or
  the gates are off): slot word := 0 (0x423ba6..bb1); roll the
  display-history ring (below) if the active line's +0x24
  latch ≠ 0 (WORD 0x4ea1d2); stage the id's TEXT: clear + copy
  the NUL-terminated string from **0x46c18c + id·0x30** (the
  WARNINGS table) into the active display record
  (0x44ec90 clear + byte copy 0x423c51..c67), set the active
  record's +0x22/+0x24 (0x4ea1d0/0x4ea1d2) := 1
  (0x423c2d/c34), then next channel. One message per channel
  per frame — a busy channel serializes its warnings.

**DISPLAY RING** [verified]: 4 records × 0x26 @0x4ea13c
(text[0x20] @+0, reveal-counter u16 @+0x22, valid u16 @+0x24);
records 0..2 = the visible history, record 3 (0x4ea1ae) = the
line being typed. Roll = copy {0x20-B text, +0x22, +0x24} of
record k+1 → record k, k=0..2 (word pair first 0x423bdd..bf9,
then rep-movs 0x20 @0x423c06). The render tail (same function,
after all channels): walks the 4 records as a 4-line staircase
(x = +0x15a + k·8), typewriter reveal (+0x22 increments per
frame vs the +0x20 dword hi-word char state; ≥0x80 = done →
+0x22/+0x24 cleared 0x423d2a..d37), per-char x-offset tables
0x454c20/0x454b70, glyph metrics 0x402884/0x402a12, chars
≥0x80 remapped FUN_00410493 (accented locales), glyphs
[0x46cdb0].

**THE 53-ID → LINE MAP [corpus-verified]**: the text table is
loaded at GameMain boot from the **[WARNINGS]** section of the
active LANGUAGE.* file (name string "WARNINGS" @0x457ac9, loader
0x424679 + 53 × 0x30 record walk 0x41c2ff..0x41c325 into
0x46c18c; sibling "MENU_ITEMS" @0x457abe → 0x46af5c, walk bounds
0x1200 = 64 records of the 96 in the section [observed; reader
internals not decoded]). All six corpus languages (DCH/ENG/FRE/
GER/ITL/SPA) carry EXACTLY 53 records, same order. ENG text,
id = call-site census (55 sites, every one reconciled):
- 0/1/2 "UNIT n HAS NOW ARRIVED" — pod release, one per player
  p: FUN_004239ef(p, p) @0x41ffd4/0x41fff7/0x420024 (§7j.19's
  citation).
- 3/4/5 "UNIT n HAS OVERHEATED", 6/7/8 "UNIT n TEMPERATURE
  CRITICAL" — heat machine @0x4101d7/0x41025e (ch 0/1/2).
- 9/0xA/0xB "UNIT n ARRIVAL IS IMMINENT" — pod-descent arm,
  FUN_0041fb4b caller @0x40c4d6/0x40c4fc/0x40c52c (§7j.20's
  "msgs 9/10/0xB" citation).
- 0xC/0xD/0xE "DANGER - UNIT n TARGETTED FOR" + 0xF "IMMINENT
  AERIAL BOMBARDMENT" — the §7g.5/§7f.6 threshold walk
  0x40c1c1..0x40c24f posts the PAIR (0xC+k, k)+(0xF, k). **The
  §7f.6 "select SFX" gloss is CORRECTED**: nothing here is a
  select sound; the walk is §7g.5's counter-threshold announcer
  (robot bank +0x70 delay vs difficulty table 0x454ee8, zone ∉
  {1,7}, cooldown latch [0x4de658]=0x80, blink-cursor
  [0x4dc5d0]:=slot+1 — the cursor write is the attention-draw,
  §7f.6's cell facts stand) and the ANNOUNCED CONTENT is the
  targeting/bombardment warning, per the corpus text in all six
  languages. The §7j.37 claim that 4239ef ids are "SFX ids, not
  text messages" is likewise CORRECTED: they are both (speech +
  WARNINGS text); the BOOT_CAMP hint-box system of §3/§7j.37 is
  the SEPARATE text channel it always was.
- 0x10/0x11/0x12 "UNIT n IS TAKING HITS" — FUN_0040e230 damage
  applier @0x40e31f.. (§7g citation).
- 0x13/0x14/0x15 "UNIT n DOWN TO HALF POWER" @0x40e4c5..;
  0x16/0x17/0x18 "UNIT n POWER AT CRITICAL" @0x40e567.. (same
  family).
- 0x19/0x1A/0x1B "UNIT n IS TOAST" — death path @0x40eae3.. =
  the CHANNEL-FLUSH triple.
- 0x1C/0x1D/0x1E "UNIT n AUTO WEAPON CHANGE"; 0x1F..0x21 "UNIT
  n OUT OF WEAPONS" — weapon watcher @0x40a105..0x40a1d2
  (§7j.37's weapon-switch citation).
- 0x22 "LASER FENCE DEACTIVATED" — delayed-trigger expiry
  (§7j.12) + fence family @0x422cdc (ch3).
- 0x23 "SECTION RAISED" @0x4224e5, 0x24 "SECTION LOWERED"
  @0x422592 — the elevator/section mover (ch3; §7j.21/§7j.46
  citations).
- 0x25 "X" — placeholder, ZERO call sites (the only unposted id
  besides none).
- 0x26 "MAIN OBJECTIVE COMPLETED" @0x448dd9, 0x27 "SUB
  OBJECTIVE COMPLETED" @0x448e45, 0x34 "PART OBJECTIVE
  COMPLETED" @0x448e6f — objective family (ch3; §7j.32).
- 0x28 "CONGRATULATIONS" + 0x29 "ALL OBJECTIVES COMPLETED" —
  the MISSION-COMPLETE pair @0x448c64/0x448c78 and the second
  site 0x448ec1/0x448ed5 (ch3; §7j.32's citation). 0x29 is the
  second text-only id — the pair types out as one two-line
  congratulation.
- 0x2A "EVACUATION COMMENCED" — extraction-beacon armer
  FUN_004247b5 @0x42488d (ch3; §7j.20's "SFX 0x2A" = the
  ticket's "armer click").
- 0x2B/0x2C/0x2D "UNIT n BATTERY EXHAUSTED" @0x40e424..;
  0x2E/0x2F/0x30 "UNIT n DAMPER EXHAUSTED" @0x41010a/0x41012c/
  0x410153→tail-jmp 0x4102ac; 0x31..0x33 "UNIT n LOSING AMMO"
  @0x41038c.. (ch 0/1/2 each).

**CORPUS REACHABILITY** (SP, corpus scenarios): ids 0/1/2 + 9/
0xA/0xB fire on every ZONEA drop (S0-class boots); 0x10..0x18 +
0x19..0x1B + 0x2B..0x33 need combat/attrition (damage paths,
S1-class); 0x22..0x24 need fences/elevators (ZONEB/F surfaces);
0x26..0x2A + 0x34 need objective completion/extraction (S5/S6-
class); ids 0x3..0x8 need the heat machine (no corpus scenario
exercises it today — mechanism + unreachability proof §7j.55:
the scorch byte's 7-cap + 1/frame fade bound +0x30 below ~140
absent ≥14 same-tile re-scorches under a parked robot). The
queue/render cells are UI-presentation
state — no engine or watch-row consequence (the spoken-line
RandA draw joins the existing T3/T4 budget class).

## 7j.54. THE 0x4ea238 AERIAL-BOMBARDMENT SHELL FAMILY + THE FUN_004245c9 CHASE-CAMERA STAGER — DECODED WHOLE: the "8-jittered-marker scatter" is a FALLING-SHELL salvo (8×10-byte shell records, staggered falls, nine 5000-damage script blasts per impact); FUN_00423e1c = its tick/resolver (NOT a "selection chaser"); FUN_004245c9 = a 15-frame chase-camera stager (NOT a "wall-strip redraw", 4 callers); the D125 content note ARBITRATED: an OFFENSIVE bombardment of the idle robot — the §7g.5 "reinforcement ARRIVAL" gloss RETIRED (2026-08-23, worker ed78ecdc claim 2, D126; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run; adopts + fully re-verifies the interrupted same-item WIP whose edits were already staged in this file; manifest clean before AND after) [verified]

The queue unit decoded the whole 0x4ea238 family: the writer
(the robots() idle-arm scatter §7g.5/§7j.45 cite), the resolver
FUN_00423e1c (formerly "the selection chaser"), the renderer
reader 0x40671a, the MissionShell clear 0x447a56, and the
[0x4de658] cooldown latch census; FUN_004245c9 fell out as a
necessary sibling decode (the resolver's record-0 impact calls
it — and it is NOT what three earlier sections called it).

**1. THE BANK GRAMMAR** [verified writer 0x40c25e..0x40c351 +
resolver 0x423e46..0x424048 + renderer 0x4066e4..0x4067a6]:
bank 0x4ea238 = 8 shell records × 10 bytes (0x50; MissionShell
entry memsets it, 0x447a51 ecx=0x50 / 0x447a56 edi=0x4ea238 via
the 0x402965 helper). Record `i` at 0x4ea238+i·0xA:
- +0 u16 **x** — world-PIXEL ground point (Q0; get_z_pos/tile
  views do >>5, the renderer subtracts the camera cells
  0x4edde4/0x4edde8 directly)
- +2 u16 **y** — world-pixel ground point
- +4 u16 **fall-z** — writer starts 0xFF; −0x20 (32 px) per
  frame while falling; the resolver pins it := ground+1 at
  impact (0x423ee8)
- +6 u16 **start-delay** — writer sets 0x20+2·i (staggered
  launch); −1 per frame (0x42403d); NO fall, NO draw, NO
  resolve while ≠ 0
- +8 u16 **valid** — 1 = live shell; cleared at impact
So §3's passing "10-byte records" note = this grammar (the
"markers" are the shells themselves — see the arbitration).

**2. THE WRITER = the robots() idle-arm scatter** [verified
0x40c25e..0x40c351, the §7g.5/§7f.6 threshold tail]: after the
idle counter +0x70 reaches the difficulty threshold
DAT_00454ee8[[0x46cbf8]] (SP: only the SELECTED robot
accumulates — the 0x40c0fc..0x40c12c gate `mode==0 ∧ state==0
∧ idx == [0x46cbd4]+[0x46cbdc]`; MP: every state-0 robot) with
zone ∉ {1,7} ∧ [0x4de658]==0 ∧ mode≠2, the announced warning
pair posts (§7j.53 ids 0xC/0xD/0xE + 0xF per occupied squad
slot, 0x40c1c1..0x40c24f) and the scatter runs: +0x70 := 0
(0x40c271) + [0x4de658] := 0x80 (0x40c27f), then 8 shells i =
0..7: `x = (robot.x Q8 >>8) + RandA&0x7F − 0x3F` (ONE RandA per
shell attempt, drawn BEFORE the bounds gate), `y = (robot.y Q8
>>8) − 0x80 + i·0x20` (deterministic fan, no y jitter — an
0xE0-px y-stride column straddling the robot, py−0x80 …
py+0x60), tile-bounds gate x≥0 ∧ y≥0 ∧
(y>>5)<[0x4eddf0] ∧ (x>>5)<[0x4eddec] (a failed shell is
simply dropped), then fall-z := 0xFF, start-delay := 0x20+2·i,
valid := 1. The salvo is centered ON THE IDLE ROBOT — x jitter
±0x3F px, the y fan straddling it with a slight up-screen
bias.

**3. THE RESOLVER = FUN_00423e1c, the shell tick** [verified
whole 0x423e1c..0x424048; sole caller MissionShell @0x447ffa,
once per frame, immediately after the §7j.53 radio-warning
consumer @0x447ff5]. Head 0x423e25..0x423e32: [0x4de658] ≠ 0 →
−− (the cooldown decay lives HERE — see census 5). Then per
record: valid==0 → skip; start-delay ≠ 0 → −−, skip.
- FALL LEG: ebx = fall-z; eax = get_z_pos(x, y, fall-z)
  (FUN_0041e231, the §6 z-query); `ground < fall-z` → fall-z
  −= 0x20 (0x424001). get_z_pos's z-clamp side effect: its
  "==3" branch latches the probe cell 0x4dc688/8c/90 (§6 note).
- IMPACT LEG (ground ≥ fall-z):
  - **record-0 camera cut** 0x423e7c..0x423ed5 — gates
    [0x4edb88]==0 (SP) ∧ record INDEX 0 ∧ [0x46cbdc]+1 ≠
    [0x4dc5d0] (cursor ≠ selected+1 — i.e. the blink-cursor is
    ON some OTHER robot than the selected one) ∧ cursor-robot
    ([0x46cbd4]+cursor−1) word@+0x2A == [0x4edb90] (player
    TYPE) → FUN_004245c9(x, y, fall-z): a 15-frame
    chase-CAMERA cut to the first impact (see 6).
  - **blast tail** 0x423eda..0x423ffc: fall-z := ground+1; SIX
    kind-6 debris (FUN_00420608, ecx=6, owner −1) at jittered
    (x+r&0x7F−0x3F, y+r&0x7F−0x3F, z+r&0x3F) — 3 RandA each,
    18 draws; then tile = (x>>5, y>>5, fall-z>>5), tile-z
    +1 if < 7 (0x423fa5); NINE FUN_004244a1 script blasts
    over the 3×3 patch x_tile−1..+2 × y_tile−1..+2 (outer x,
    inner y, 0x423fb6..0x423fe6) — the §7j.39 kill-anything
    5000-damage entry (splash + critter/robot sweeps, each
    call 1 RandA gate + a 1-in-8 kind-6 debris tail ⇒ ≈27–29
    RandA per impacting shell total); then [0x4dc5d0] := 0
    (blink-cursor off) and valid := 0 (0x423fef/0x423ff5).
    The resolver never touches the selection — the old §7j
    item-6 "selection chaser / re-points the selection" gloss
    is RETIRED (its only selection-adjacent act is CLEARING
    the blink cursor at impact).

**4. THE RENDERER** [verified 0x4066e4..0x4067a6, inside
FUN_00403938's draw tail]: loops the 8 records (stride 0xA to
0x50); gates valid≠0 ∧ start-delay==0 (0x4066f4/0x4066fe);
iso-projects (x−camx, y−camy: screen-x = dx−dy+0x10D, base
screen-y = (dx+dy)/2+0xAC+scroll) and SUBTRACTS fall-z from the
screen-y axis (0x406758) — the sprite starts 255 px above its
ground point and visibly DESCENDS 32 px/frame; bounds-culls
0..0x23F/0..0x23E; draws GENERAL.BIN sprite 0x12C via
FUN_0040798e (bank cell 0x4edd7c, 0x40678d..0x40679c). The
reader 0x40671a the queue cited = the record's y-word load.

**5. [0x4de658] CENSUS — the salvo COOLDOWN latch, closed**
[full objdump traffic census]: arm write 0x80 @0x40c27f; arm
precondition read (==0) @0x40c18b; read+dec 1/frame
@0x423e25..0x423e32 (the resolver head — so the latch is the
128-frame salvo cooldown); MissionShell entry clear @0x447877.
The ONLY other text match, 0x442ba7 (`WORD [eax+0x4de658]`,
eax = p·0x62+0xE), is the §7j.45/D89 SHOP MP loadout-mirror
write 0x4de664+p·0x62+g·0xE — a displacement ALIAS (address ≥
0x4de672), NOT an access to the latch. No other writers or
readers exist. (The 0x80 dword sits 0xC below the weapon-table
base 0x4de664 — it is NOT part of the weapon table.)

**6. FUN_004245c9 = the CHASE-CAMERA OVERRIDE STAGER** [verified
0x4245c9..0x4245e5 — 5 instructions, no redraw of anything]:
eax/edx/ebx (x,y,z) → [0x4de648]/[0x4de64c]/[0x4de650], and
const 0xF → [0x4de654] (the 15-frame countdown). Consumers:
(a) FUN_00403938 0x4039b0..0x403a42 — while [0x4edbd8]≠0 ∧
[0x4de654]≠0 the camera-point ring slot (0x4c71c4/c8/cc,
4-slot ring [0x46ccdc]) loads the STAGED triple instead of the
selected robot's pos (Q8>>8), and [0x4de654]−− per frame —
a 15-frame camera cut to the staged point; (b) robots()
0x40b885 — the camera-RECENTER block is gated OFF while
[0x4de654]≠0; MissionShell clears [0x4de654] @0x4478ad.
[§7j.56 refinement: [0x4edbd8] = the ACTIONPAN registry config
flag, DEFAULT 1 = ON; the recenter's [0x4edbd8] read is
@0x40b875, the [0x4de654] leg @0x40b885.]
**FULL caller census (4, verified)**: door/section stepper
FUN_004223b8 @0x422427 (§7j.21/§7j.34); delayed-trigger expiry
FUN_00422e0a @0x422e55 (§7j.12); artillery spotter reveal
@0x41173a (FUN_00410823 types 9..0xB at ttl==0x18, §7j.22);
bombardment record-0 impact @0x423ed5 (this section). ALL
"wall-strip redraw"/"wall redraw" glosses for this function
(§7j.19, §7j.21, §7j.22 ×2, the door ledger row) are RETIRED —
every caller stages a LOOK-AT-ME camera cut; the wall redraw
that accompanies doors is the per-tile FUN_004235e4/FUN_004235bf
stamping around it, not this call.

**7. THE D125 ARBITRATION — CLOSED: OFFENSIVE BOMBARDMENT.**
The announced pair ("DANGER - UNIT n TARGETTED FOR / IMMINENT
AERIAL BOMBARDMENT", all six languages) is CONFIRMED by the
mechanism: each shell's impact converts into NINE 5000-damage
kill-anything script blasts over a 3×3-tile patch centered on
the scatter center = the IDLE ROBOT'S OWN POSITION (±0x3F px x
jitter, the y fan straddling it) — the salvo punishes the PLAYER'S
idle unit (difficulty idle thresholds {400,300,200,5000}
frames ≈ 6.7/5/3.3/83 s at 60 fps; ordering the robot resets
+0x70 via the states-3/5 block). The markers are NEITHER
targeting reticles NOR arrival beacons — they are the SHELLS
THEMSELVES (falling sprites, §7g's kind-6 debris + §7j.39
blasts at the end). §7g.5's "reinforcement ready/ARRIVAL"
gloss and §7f.6's threshold-walk framing are RETIRED (the
in-place corrections landed with this unit; §7h's separate
"reinforcement staging" powerup case-1 family — drop(+0x80)
=1000, §7h item 1 — is a DIFFERENT, real reinforcement
mechanism and stands).

**8. Engine/differ consequences**: NONE today — the bank, the
latch, and the camera cells are SP-UI/staging state with zero
engine reads, and NO corpus scenario exercises the idle
threshold (S0..S7 keep the selected robot ordered or active;
an SP capture would need the SELECTED robot left state-0 for
400/300/200/5000 frames). If ever modeled: the salvo costs
8 RandA at arm + ≈27–29 per impacting shell (T2/T3-class
terrain/kill traffic through FUN_004244a1 + kind-6 disburser);
[0x4ea238] and [0x4de658] are additive watch-row candidates,
deliberately NOT in the first golden.

## 7j.55. THE HEAT MACHINE — FUN_004100b7 (heat-in) + FUN_004102b6 (ammo cook-off) DECODED WHOLE: the §7j.45 "armor/pool" gloss RE-LABELED HEAT/DAMPER per §7j.53's corpus strings; +0x30 = HEAT accumulator, +0x98 = DAMPER pool, +0x32 = the LOSING-AMMO cooldown (its "producer unknown" residue CLOSED); corpus reachability = the warnings/cook-off are UNREACHABLE by construction (2026-08-23, worker 19d79ca9 claim 2, D127; docs-only; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; manifest clean; registry_anchors 2/2 green) [verified]

§7j.53 named the twelve call sites (ids 3..8 "OVERHEATED"/
"TEMPERATURE CRITICAL", 0x2E..0x30 "DAMPER EXHAUSTED",
0x31..0x33 "LOSING AMMO") but left their containing family
undecoded; §7j.45 item 4 had the mechanics under a pre-§7j.53
"armor" reading. This unit decodes both functions
instruction-exact, arbitrates the terminology, and closes the
reachability question.

**1. FUN_004100b7 = the HEAT-IN machine** [verified
0x4100b7..0x4102b6, sole caller = the robots() phase-1 pass
0x40bc72 with amount 0x14 on a nonzero scorch byte — the SAME
byte==0 branch bleeds −0xA clamp ≥0 @0x40bc7d/0x40bc98]:
args eax = robot idx, edx = amount. `amount == 0` → return
(@0x4100be). Record address = idx·0xA8 + 0x4c69e4 (the §3 bank).
- **DAMPER branch** (dword@+0x98 ≠ 0): `pool −= amount`
  (@0x4100e1); still > 0 → return; ≤ 0 → `pool := 0`
  (@0x4100f9) + post "UNIT n DAMPER EXHAUSTED"
  (FUN_004239ef(0x2E+k, k) @0x41010a/0x41012c/0x410158→tail
  0x4102ac) + return. The pass that breaks the damper adds NO
  heat; the triple fires ONCE (pool stays 0 until the
  MP-respawn stats-copy re-arms it). +0x98 = **the DAMPER**
  (equipment stat 0x2C, word×200 @0x40d013 spawn / 0x40ea59
  MP respawn — the same chassis switch as 0x2A shield charges
  +0x8C / 0x2B battery +0x94, §7j.45).
- **HEAT branch** (+0x98 == 0): `word@+0x30 += amount` (16-bit
  wrap @0x41015d..0x41016c), clamp new > 0xBB8 (3000) → 0xBB8
  (@0x41017f..0x41018e). Threshold logic keyed on OLD
  (pre-add: dword@+0x2E sar 0x10) vs NEW:
  (a) old ≥ 0x9C4 (2500) → **FUN_004102b6 EVERY pass**
      (@0x410190..0x41019a — the cook-off attempt below);
  (b) old < 2500 ∧ new ≥ 2500 → "UNIT n HAS OVERHEATED"
      (FUN_004239ef(3+k, k) @0x4101d7/0x4101f9/0x41021d);
  (c) old ≥ 0x753 (1875) → return (@0x410222..0x410228 — the
      already-critical short-circuit);
  (d) old < 1875 ∧ new ≥ 1875 → "UNIT n TEMPERATURE CRITICAL"
      (FUN_004239ef(6+k, k) @0x41025e/0x410280/0x4102ac — the
      ch-2 leg SHARES the 0x4102ac tail call with the damper
      id 0x30 arm).
  Both crossings are EDGE-triggered; a single huge add crossing
  both posts BOTH (overheat first, critical second). Rising
  heat escalates 1875 CRITICAL → 2500 OVERHEATED (the CRITICAL
  warning PRECEDES the OVERHEAT state — the corpus strings read
  as "about to" vs "has"). All four triples use the standard
  per-squad-slot dispatch: `idx == [0x46cbd4]+k` gated
  `[0x46cbd8] > k`, one post per event (current player's squad
  only).

**2. FUN_004102b6 = the AMMO COOK-OFF** [verified
0x4102b6..0x4103ed, sole caller 0x41019a; arg eax = robot idx]:
- Gate 1: `RandA() & 0x7F == 0` — 1/128 per pass (@0x4102be;
  phase 1 runs once per frame → ~0.47 cooks/sec while
  overheated).
- Gate 2: `w = RandA() & 7` must be < 7 (@0x4102cb) → uniform
  over the SEVEN weapon slots (a rejected 7 aborts, no drain).
- Drain: `ammo = word@(record + 0x38 + 8w)` (the slot's word1 —
  §6c.6's "order gate" ≡ §7j.37's ammo, the same cell);
  `drain = ammo >> 3`, 0 → 1; `ammo −= drain`, then result ≤ 1
  → `:= 1` (@0x41031a..0x410336) — **the last round never
  cooks off** (floor 1, and an empty slot stays 0: 0 ≤ 1 → :=1
  would ARM an empty slot to 1 — the ≥2500 precondition means
  this quirk is only observable in a cooked unit).
- If `type(+0x2A) == [0x4edb90]` (a player-type robot):
  `[0x46ccec] := 2` (@0x410344..0x410358) — the sidebar_control
  cell (the same producer value as §7j.45's player-present
  walk; presentation).
- Rate limiter: `word@+0x32 == 0` → post "UNIT n LOSING AMMO"
  (FUN_004239ef(0x31+k, k) @0x41038c/0x4103ae/0x4103d2) +
  `word@+0x32 := 100` (@0x4103e3) — one warning per 100 frames
  max while overheated.
- RNG: ONE RandA per failed gate-1, TWO per attempt that
  reaches the slot pick (draw-count class T3/T4; relevant only
  if ever corpus-reachable, see 5).

**3. THE +0x32 CELL CLOSED** (§7j.45 Part B's "producer
unknown" residue): writers = FUN_004102b6's tail ALONE
(:= 0x64 @0x4103e3); decay = the robots() pre-walk dec-gated-≠0
trio site 0x40bab7..0x40bac6 (alongside +0x34/+0xA4 — dec 1
per walk each); reader = the 0x41036e gate ALONE. The §3
"scorched tiles re-burn every ~100 frames" gloss RETIRED —
the cell is the **LOSING-AMMO warning cooldown**, nothing
else. The +0x34/+0xA4 ALARM pair has NO relation to this
family (zero traffic in 0x4100b7..0x4103f2) — they belong to
FUN_0040e230; the only tie is the shared decay walk.

**4. FULL CELL CENSUS** [objdump traffic, displacement-aware]:
- word@+0x30 (0x4c6a14; the dword@+0x2E sar-16 view): writers =
  the phase-1 bleed (−0xA + clamp-0 @0x40bc7d/0x40bc98),
  FUN_004100b7 (add 0x41016c / clamp 0x410187), the SP death
  reset (:= 0 @0x40eacf — inside the SP branch of the death
  tail), the MP respawn reset (:= 0 @0x40e864). NOTE: the
  0x40e6e2 `mov WORD [eax+0x4c6a14],di` match is the
  seven-order-words zeroing walk (eax = idx·0xA8 + 8k, k =
  8..0x38 → effective record+0x38..+0x68, §7j.45 item 6), NOT
  a +0x30 site. Readers = FUN_004100b7 internal + the sidebar
  gauge FUN_0040807f ×3 slots (0x408129/0x408252/0x40837d —
  §7f.4's "armor bar": clamp 2500, sprites 0x60..0x8E; **the
  gauge's FULL SCALE = the OVERHEATED threshold 2500 exactly**
  — re-labeled the HEAT gauge, §7f.4 gloss corrected) + the
  bleed clamp check 0x40bc85. NO other traffic.
- dword@+0x98 (0x4c6a7c): writers = the spawn stats-copy case
  0x2C (word×200 @0x40d013), the MP-respawn stats-copy
  (@0x40ea59, same formula), FUN_004100b7's drain/zero; reader
  = FUN_004100b7 ALONE. Fresh campaign: all-zero stats →
  damper 0 → scorch heats immediately.

**5. CORPUS REACHABILITY — UNREACHABLE BY CONSTRUCTION** [the
§7j.53 note CONFIRMED with the mechanism]: the only pad-armer
is the type-DB +0x18 SCORCH byte (robot-death rings 1/2/4
corner/edge/center §7j.9; platform weaken/build +4 §7j.41;
clamp 7; the unconditional −1/frame fade §7j.10). One write
keeps a tile armed ≤ 7 frames → ≤ +140 heat per event chain;
crossing 1875 needs ≥ 94 NET armed passes = the byte
re-written ≥ ~14× within ~94 frames under a PARKED robot. The
corpus stages no robot deaths on parked robots at all (S4/S5C
destroys are structures/critters; S7's platform events are
sparse single writes; the death of the standing robot itself
resets its OWN +0x30 to 0), so +0x30 leaves 0 in every
canonical run except ≤140-scale S7-class wiggles — and BELOW
2500 the machine is FULLY DETERMINISTIC (zero RNG — both RandA
draws live in FUN_004102b6) mutating only +0x30/+0x98, both
in-span robot-bank bytes E models verbatim (armor_charge/
bleed/resets). The pinned chains therefore hold; warning ids
3..8 + 0x2E..0x33 never post on corpus paths.

**6. Engine/differ consequences**: NONE today. E's state side
is exact (the damper absorb + accumulator add/clamp, the
bleed, the SP/MP resets); the presentation legs (warnings) and
the unreachable cook-off are correctly omitted; E's
deliberately-unmodeled +0x32 decay is unobservable (the sole
producer never runs in corpus). IF a future scenario arms
sustained scorch under a parked robot (≥14 same-tile writes in
≤94 frames — e.g. a scripted kill-cascade), E MUST add
FUN_004102b6 verbatim (the gate RandA draws + the ammo drain
would otherwise diverge the RNG stream AND the weapon banks) —
recorded seam; additive watch rows only then. The
"armor-pad-reads" watch id keeps its legacy name (the registry
anchor is load-bearing; the byte is the scorch byte).

## 7j.56. THE [0x4edbd8] CAMERA-GATE CELL + THE [0x4ede54] ZOOM CELL — CLOSED: [0x4edbd8] = the ACTIONPAN REGISTRY config flag (HKCU\Software\Mirage\Bedlam\1.00, DEFAULT 1 = pans ENABLED; the "CONFIG.BDL" gloss retired — the string has zero binary references); [0x4ede54] = the viewport ZOOM height (240..480 backbuffer rows, ± keys) (2026-08-23, worker 21e88d3b claim 2, D128; docs-only; objdump-only from ghidra-project/exw-text-objdump.txt + read-only string/import probes of game-data/cd-root/BEDLAM.EXW (.idata parsed to name IAT 0x4f010c = RegQueryValueExA); no Ghidra run, no corpus write; manifest clean before AND after) [verified]

**A. [0x4edbd8] = ACTIONPAN — a REGISTRY-backed config flag,
session-constant, default ON.** Complete text census: FOUR
sites.

*Readers (exactly two, both the §7j.54 consumers):*
1. FUN_00403938 `cmp [0x4edbd8],0` @0x4039b0 — with
   [0x4de654]≠0 the camera-point ring slot loads the staged
   triple (§7j.54 (a); the pan camera cut).
2. robots() recenter gate @0x40b875 (`mov edi,[0x4edbd8];
   test; je do-recenter`) + the second leg 0x40b885
   (`cmp [0x4de654],0; jne skip`) — the address refinement of
   §7j.54's "0x40b885" citation: [0x4edbd8] is read at
   0x40b875; ==0 → recenter runs unconditionally; ≠0 → recenter
   suppressed ONLY while a pan countdown is live (the dead
   `je 0x40b970` @0x40b87f immediately after the taken-je is a
   Watcom branch-shape artifact, never taken). No other reader
   exists in the binary.

*Writers (exactly the config family, zero game-state writers):*
the boot loader FUN_004252c0 registers the parse
`FUN_0044ede4(eax="ACTIONPAN"@0x458ccf, edx=&0x4edbd8,
ebx=4, ecx=1, stack(min=0,max=1))` @0x42535c; the saver
FUN_0042540c re-persists the cell @0x42545c via
`FUN_0044ed98(eax="ACTIONPAN"@0x458d1f, edx=[0x4edbd8],
ebx=4)`. The string "ACTIONPAN" appears EXACTLY TWICE in the
binary (loader key + saver name — file offsets 0x572cf/0x5731f,
i.e. VA 0x458ccf/0x458d1f); the cell itself is .bss
(zero-init; PE: BEGTEXT 0x401000/DGROUP 0x454000/.bss
0x45b000..0x4efa00, ImageBase 0x400000).

*The config family is REGISTRY, not file I/O* [.idata parsed:
0x4f0108=RegCreateKeyExA, 0x4f010c=RegQueryValueExA,
0x4f0110=RegSetValueExA (ADVAPI32)]:
- FUN_0044ed40 (0x44ed40..0x44ed83) = the opener:
  RegCreateKeyExA(HKEY_CURRENT_USER (0x80000001),
  "Software\Mirage\Bedlam\1.00"@0x456e28, 0, "DATA",
  KEY_ALL_ACCESS 0xF003F, ..., &hKey) → hKey [0x4ef770];
  called at the loader head 0x4252f6.
- FUN_0044ede4 (0x44ede4..0x44ee94) = the bounded loader:
  RegQueryValueExA(hKey, name, 0, &type@stack, edx-dest,
  &cb) — the value is written DIRECTLY into the dest cell;
  SUCCESS (eax==0) → switch on cb: 2 → sign-extended WORD
  bounds-check, 4 → DWORD bounds-check vs the stack pair
  (min@+0x1C, max@+0x20), else keep; FAIL (value absent) →
  dest := low byte of the ecx DEFAULT @0x44ee23..27 (+ a
  FUN_0044ed98 self-heal call); out-of-bounds → same default
  rewrite @0x44ee79..8a (WORD variant 0x44ee54..65). The
  family pattern cross-checks: INSTALLDRIVE (0x42539e) uses
  bounds ['A'(0x41),'Z'(0x5A)] default 'C'(0x43); SOUND
  bounds [0, current-volume]; DEFAULTNAME via the string
  sibling FUN_0044eee0 (create-if-missing REG_SZ "Player", 8
  bytes, 0x4253bb..0x4253cf).
  **⇒ ACTIONPAN post-boot ∈ {0,1} with DEFAULT 1 (ON)** —
  absent or malformed registry value ⇒ pans enabled; only an
  explicit stored 0 disables them.
- FUN_004252c0 (loader) is called ONCE per boot @0x41c129
  (GameMain init; it also pre-seeds [0x4edbd4]=[0x4edbf0]=
  [0x4edbe0]=1, [0x4edbe8]=2, [0x4ddb2c]=0x4B); the saver
  FUN_0042540c runs at the name-entry exit 0x43b03b (the
  TITLEMENU §4 path) + 0x41c59b (FUN_0044ed98
  query-then-RegSetValueExA @0x44edb7/0x44edbe, ebx = the
  REG_DWORD type word 4 [inferred]; FUN_0044ed84 = the hKey
  user @0x44ed86, 19 B — close/flush-class [inferred]).
- **THE "CONFIG.BDL" GLOSS RETIRED**: the byte sequence
  "CONFIG.BDL" occurs ZERO times in BEDLAM.EXW (only
  "CONFIG.SYS file, or" inside an error string @0x45860f);
  the on-disk game-data CONFIG.BDL/OPTIONS.BDL are DOS-build
  leftovers EXW never opens (SAVED.BDL is the only referenced
  .BDL — savegames, different family). The TITLEMENU §4 note
  is corrected in place below.

*In-game identity:* [0x4edbd8] is a per-SESSION constant
enable bit for the entire §7j.54 chase-camera subsystem — no
mission phase, no game state, no menu UI writes it (EXW has no
options screen for it; TITLEMENU's unreferenced MENU_ITEMS
"Options" row survives only in the DOS string table).

**B. [0x4ede54] = the VIEWPORT ZOOM (vertical viewport height
in backbuffer rows, clamp [0xF0,0x1E0] = [240,480]).**
Complete text census: 26 sites, no indirect refs.

*Writers (three families):*
1. **MissionShell per-mission init** 0x447883
   `mov [0x4ede54],edx` — in the straight-line reset block
   (xor ecx,ecx at 0x44785e, then the zero-stores 0x4dc678/
   0x4edba0/0x4dc5d0/0x4de658/0x4ede34, then this store).
   CAVEAT [dataflow pinned]: the edx loaded at 0x44784a
   (=0x1E0, the FUN_004034ef music arg) does NOT provably
   survive the three intervening calls — FUN_004034ef's last
   edx write is `imul edx,edx,0x26` @0x403570 and FUN_0041d954
   zeroes edx on its xor tails (@0x41db1c/0x41db45); the
   stored dword is formally callee-leftover. BENIGN by
   construction: every consumer dispatches v≥480 as the 1:1
   copy, and the first zoom keypress re-clamps into
   [240,480] — the mission-start zoom reads full-width
   regardless [inferred].
2. **The zoom-key handler** = the FUN_0042034c tail
   (0x4204ea..0x420548): key held (scan 0x4E keypad-plus ∨
   0x0D '=') → `add [0x4ede54],0x10` @0x4204fc; (scan 0x4A
   keypad-minus ∨ 0x0C '-') → `sub ...,0x10` @0x420515;
   then clamp floor 0xF0 @0x420528 / ceiling 0x1E0
   @0x42053e. Key cells = the g_keystore (base 0x4edc44)
   at +0x4E/+0x0D/+0x4A/+0x0C = 0x4edc92/0x4edc51/0x4edc8e/
   0x4edc50. FUN_0042034c (prologue 0x42034c, sole caller
   MissionShell 0x448076) = the overlay-word range consumer
   of D119 with this input tail.
3. **The temp-override save/restore pair** inside
   FUN_00401107's [0x4ede34] path: v_old pushed 0x4012c7,
   `v := 0x1E0 − min([0x4ede34],0x1DF)` @0x4012e5
   (guaranteed ∈ [1,480]), scaled render, `restore` @0x4012f1.

*Readers (four families):*
1. **FUN_00401107 = the ZOOM BLITTER** (called from the two
   MissionShell render sites 0x447ca0/0x448094 — the same
   pair as FUN_00403938): dispatch — [0x4edba0]≠0 (the
   §6c.1 map-overlay toggle) → 1:1 map copy 0x401266..0x40129d;
   [0x4ede34]≠0 → the temp path above; else v=[0x4ede54]:
   v ≥ 0x1E0 → plain rep-movs 1:1 (0x401178..0x4011a4);
   v < 480 → the SCALED MAGNIFY path: Q16 scale
   0x454068 = 0x454060 = (v<<16)/480, halves 0x454064/
   0x45405c (0x4011aa..0x4011ef; the temp path uses the
   INVERSE (0x1E0<<16)/v @0x40134a..0x40137f), source
   window offset (480−v)/2 rows (0x4011f9/0x401208), per-row
   sub-pixel stepping through the Q16 pair — a software
   2×-max vertical magnifier of the 640-wide backbuffer
   ([0x4ede18], 0xA0-byte rows) onto the 480-visible screen.
   (The §7e "map present" row's FUN_00401107 attribution
   covers the map path; the zoom path is this census's
   addition.)
2. **The camera-recenter speed factor** (robots()
   0x40b89e/0x40b8c5): new camera target +=
   (cursor−240)·v/480 per axis — full-speed at zoom-out
   (480), half-speed at max zoom-in (240).
3. **The cursor un-zoom mappers** (screen→world): 0x4106a1/
   0x4106d4 (the FUN_004106xx family — cursor cells
   [0x4ede00]/[0x4ede04] minus 240, ×v/480, feeding the
   0x4eddf8/0x4eddfc lane; gated off while [0x4edba0]≠0
   @0x410675) and 0x419a41 (the same transform inside the
   0x4198xx octile-scan family near FUN_0041ebf8).
4. (The [0x4ede34] temp path itself reads v 5×:
   0x4012c7/0x40134a/0x40136b/0x401389/0x4013ac.)

*[0x4ede34] census pointer (adjacent cell) — **CLOSED by §7j.58/D130** (2026-08-23): it is the CLOSING-IRIS death-wipe cell — `:=1` at selected-robot SP death 0x40ea8b, +0x28/frame 0x4480af, terminal 0x1E0 + auto-reselect 0x4480d6/0x448121, cancels = click-select ×3 + per-mission 0x44787d; FUN_00401107 renders fill-0 + centered v×v shrink of the frozen frame, FUN_00403938 skips its render body during the wipe; the [0x4ea8f8] sibling = the MP death-position marker countdown. Full grammar in §7j.58.*

**C. Engine/differ consequences.** ZOOM: none — no corpus
scenario presses the zoom keys (the harness injects COMMAND
records/.PAD, never raw keyboard), the cell is deterministic
per mission (init + no writer), touches zero RNG and zero
robot-bank bytes, and the render-path difference is
presentation-only → NO differ rows; the zoom machine is
recorded for the future E-side render parity only (v∈[240,480],
±16/frame, the Q16 magnifier grammar). ACTIONPAN: **one live-
channel confund to record** — default 1 means the §7j.54 pans
are LIVE on a default install; the O1 capture machine's
registry could hold a stale 0 (any pre-DOS-era install or a
hand-edit) which would silently disable pans on the original
while E models them → the S0 live-session fingerprint step
(item 1) should record [0x4edbd8] (and the five sibling config
cells) once; a ONE-FRAME additive watch row is the remedy if
it ever bites (deliberately NOT in the first golden). The
§7j.54 machinery itself is unchanged (its staging row and the
[0x4de654] countdown already cover the pan state).

## 7j.57. THE ROBOT +0x9C DEATH FLAG — READER CENSUS CLOSED: the sole reader is the SP SQUAD-WIPE FAIL DETECTOR (FUN_0044764c → MissionShell ret 3); lifecycle CLOSED — set-only := 1 in both FUN_0040e230 death tails, cleared ONLY by the mission-staging WHOLE-BANK ZERO-FILL (0x7E0 = 12·0xA8 bytes — the bank is 12 slots); the §7j.55 sidebar cross-question answered NO (2026-08-23, worker 18039414 claim 2, D129; docs-only; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; MANIFEST.sha256 clean before AND after; registry_anchors 2/2 green) [verified]

Closes §7j.45 item 6 open point (the queue pre-census re-run, displacement-aware: exactly THREE text sites for 0x4c6a80 — the two §7j.23/24 producers + ONE reader — plus the implicit clear via the bank base; every +0x9c] text match is an [esp+0x9c] stack slot, no register-base displacement form and no *8+0x9c scaled form exists).

**A. The two producer values PINNED** (the queue ask — both are **1**, so the queue "MP-respawn reset" phrasing is a misnomer, corrected history-preserved in §7j.45 item 6):

1. SP/other tail @0x40eac0 `mov [eax*8+0x4c6a80],edx` — edx := 1 @0x40eab4, no intervening def. Reached when [0x4edb88]==0 (SP) OR the no-extract latch [idx*4+0x46aed4]≠0 (the §7j.24-8 "SP gate" — the SP-style death bookkeeping: [0x4ede34]:=1 when idx is the SELECTED robot [0x46cbd4]+[0x46cbdc], alive/+0x80/hp := 0, heat +0x30 := 0, per-slot death SFX 0x19/0x1A/0x1B).
2. MP respawn tail @0x40e82a `mov [ebp+0x4c6a80],edi` — edi := 1 @0x40e807, no intervening def. Reached when [0x4edb88]≠0 ∧ [idx*4+0x46aed4]==0 (the §7j.24-8 full respawn re-init: new position from the 0x4e6430 spawn table, variant RandA&3, pod timer 0x28, weapon/equipment re-copy, alive/hp/heat/shield re-staged). **The re-init does NOT clear +0x9C — the respawned MP slot stays death-flagged** (harmless: the sole reader is SP-only, see B).

Both sit in FUN_0040e230 death epilogue (gate [0x46cd0c]==0 at the head); the §7j.23/24 decode stands unchanged.

**B. The sole reader — the SP SQUAD-WIPE FAIL DETECTOR** (FUN_0044764c..0x44770a, decoded whole):

- [0x4edb88]≠0 (MP) → xor eax,eax ret 0 — SP only, exactly.
- Walks the player squad records [0x46cbd4] .. [0x46cbd4]+[0x46cbd8]−1 (0xA8 stride, loop 0x44768e..0x44769e): the FIRST record with **+0x9C == 0 → return 0** (someone alive); +0x9C≠0 = dead → skip.
- All squad records dead ∧ [0x4ede34] == 0x1E0 → the FAIL SEQUENCE: FUN_0042391d ([0x4eddc0]:=0 + FUN_0044b3f8), FUN_00425a03 (FUN_0044acf4 + [0x4edb3c]:=0 + FUN_0044ad18), optional FUN_0042595a (gated [0x4edbe8]≠0 ∧ [0x4edbec]≠0), FUN_00425bf5, then the [0x46cca4]-gated animation posting ([0x46af0c]:=[0x46af20], FUN_0042582a(0x800,0), string 0x459852 via FUN_0044567c, FUN_00425851) → **eax := 1**.
- Sole caller: MissionShell 0x44870d, gated [0x4dc67c]==0 = **extraction NOT complete** (§7j.27 dropship cell; the alternate branch 0x4486e4 handles [0x4eb8b8]≠0/[0x4edd8c]==1 → ret 4). Result 1 → eax := 3 → **MissionShell returns 3** (the fail/debrief screen transition; cf. ret 2 = launch). A wiped squad AFTER extraction completed never fails — the detector stops running.
- The [0x4ede34]==0x1E0 conjunct = the death-wipe viewport cell at its terminal 480 value: set := 1 at the selected robot death (0x40ea8b), zeroed per-mission (0x44787d) and on click-select (0x40d286) — i.e. the fail waits for the death wipe to FINISH before transitioning. The cell own grammar is CLOSED by §7j.58/D130 (the closing-iris machine).

**Semantics verdict:** +0x9C is the MISSION-FAIL liveness oracle — DISTINCT from +0x7C (alive: the select/AI gate, zeroed at death but RE-SET by the MP respawn) and +0x78 (hp). Within a mission it is set-only (once 1, never 0); a dead MP bot respawns with alive/hp restored but +0x9C still 1 — legal because nothing in MP ever reads it.

**C. Lifecycle CLOSED — where the flag is cleared.** NO literal zero-writer exists; the clear is the mission-staging WHOLE-BANK ZERO-FILL: FUN_0040cca2 @0x40cd29..0x40cd38 — ecx := 0x7E0; edi := 0x4c69e4; call FUN_0041cd42 (the [0x4eba20] file rewind; edi/ecx are callee-saved, NOT its args); call FUN_00402965 = the memset-0 ledger row (§7j.21: EAX=0, ECX=len, EDI=dst) → zeroes **0x7E0 = 12·0xA8 bytes = the whole 12-SLOT bank** (new fact: the bank is 12 slots; [0x46ccbc] counts the staged robots). A sibling zero-fill FUN_00402965(ecx=0x150, edi=0x4e64c0) follows it (aux bank, out of scope). The per-record staging walk 0x40ce70..0x40d0a0 never writes +0x9C — so EVERY mission entry starts with the flag clean for all slots (bss-zero at process start; wiped squads cannot leak into a re-entered mission). The only site in the binary that loads 0x4c69e4 as an immediate is this zero-fill — no bulk-copy/save-load restore path touches the bank.

**D. The §7j.55 sidebar cross-question answered: NO.** The heat-family sidebar row pass never reads +0x9C. [0x46ccec] census this unit: the sole reader is 0x407205 (the per-frame sidebar updater pass: cell ≠0 → dec → FUN_00408403) — [0x46ccec] is a FLASH-COUNTDOWN cell in the [0x46ccf0]/[0x46ccf8] timer family (≠0 → dec → FUN_004085ce / FUN_00401ca2(0x12,1,…)), values 2/3 = flash durations; writers incl. death :=3 (0x40e6d2), cook-off :=2 (0x410358), click-select :=2 (0x40d280 family), MissionShell head/frame sites. The queue "likely the dead-robot per-frame handling" hypothesis is retired: the reader is the fail detector, a mission-level control-flow gate, not per-robot handling.

**E. Engine/differ consequence: NONE.** E already conforms (mission.rs death tail `death_flag = 1` with alive=false/drop_countdown=0/hp=0/armor=0 per the SP subset; fresh records per mission ≡ the whole-bank zero-fill) and `death_flag` is already a field leaf of the T1 robot-bank differ row (+0x9C, U16 — upper word always 0 since every write is a dword store of 1). No watch rows, no differ changes; the fail detector itself is a screen-transition gate outside the dump surface.

## 7j.58. THE [0x4ede34] DEATH-WIPE CELL — CENSUS CLOSED: it is the CLOSING-IRIS death wipe (value grammar 0 → 1 at selected-robot SP death → +0x28/frame → terminal 0x1E0 = auto-reselect/fail-detector conjunct); the temp render = full-screen fill-0 + a centered v×v SHRINK of the FROZEN world frame (v := 480−min(cell,479)); the [0x4ea8f8] sibling = the MP-only death-position marker countdown (0x20 frames) (2026-08-23, worker 27b33f6c claim 2, D130; docs-only; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; MANIFEST.sha256 clean before AND after; registry_anchors green) [verified]

Closes the §7j.56/B census pointer (queue item: pin the value grammar + WHAT the temp render shows). Complete displacement-aware text census: **13 sites** for 0x4ede34, no indirect/register-displacement refs (absolute-only addressing).

**A. Site census + value grammar.** Writers by value:
- **`:= 1` (ARM)** — sole site 0x40ea8b: FUN_0040e230 SP/other tail, when the dying robot IS the selected one (idx == [0x46cbd4]+[0x46cbdc], the D129 decode; reached when [0x4edb88]==0 SP ∨ no-extract latch). MP never arms — the MP branch posts the [0x4ea8f8] marker latch instead (E below) and respawns.
- **`+= 0x28` (+40/frame)** — sole site 0x4480af, the MissionShell frame cluster (B below).
- **`:= 0x1E0` (TERMINAL clamp)** — sole site 0x4480d6, same cluster (when cell+40 ≥ 480).
- **`:= 0` (CANCEL)** — five sites: the three squad-slot click-select strips 0x40d286/0x40d311/0x40d398 (§6c.2 — selecting an ALIVE squadmate y∈[5,0x35], x∈[0x1E7,0x217]/[0x219,0x249]/[0x24B,0x27B]; also zero [0x4ea8f8] in tandem), the auto-reselect cancel 0x448121 (B below — a Watcom xor-of-equals zero: `xor ecx,edi` where the branch just proved ecx==edi), and the per-mission reset 0x44787d (MissionShell straight-line block: xor ecx,ecx @0x44785e, callee-saved through FUN_0041d954, zero-stores 0x4dc678/0x4edba0/0x4dc5d0/0x4de658/[0x4ede34]/… — [0x4ea8f8] zeroed too @0x4478f1, ecx provably still 0).

Readers (four):
1. **FUN_00401107** (the zoom blitter, MissionShell present call 0x448099): dispatch 0x40110c [0x4edba0] map-overlay first, then gate 0x401119 `cell≠0` → the temp path (C below); read 0x4012cd inside it.
2. **FUN_00403938** (the world/sidebar render, present call 0x448094): head gate 0x403952 `cell≠0` → jmp 0x4071d5 (the shared tail: map-present FUN_004089b1 only in map mode, FUN_004072bf, FUN_0040807f, the [0x46ccf0]/[0x46ccec] frame timers) — the whole render body is SKIPPED while the wipe runs. (Address-attribution correction to §7j.56/B: 0x403952 is FUN_00403938's gate, not "a FUN_00401107 gate".)
3. **FUN_0044764c** 0x4476a2 `cmp 0x1E0` — the §7j.57/D129 squad-wipe fail-detector conjunct (the wipe must FINISH before the fail fires).
4. **MissionShell frame machine** 0x44809e (B below).

**B. The frame machine + timeline** (MissionShell main loop, immediately AFTER the present call 0x448099):
```
44809e: edi := [0x4ede34]; test; je done        ; only runs when armed
4480ac: ebp := edi + 0x28; [0x4ede34] := ebp    ; +40 per frame
4480b5: cmp ebp,0x1E0; jl done                  ; <480 → keep wiping
4480c1: eax := [0x46cbd4]*0xA8                  ; terminal block:
4480d0: edi := [0x4edb90]                       ;   the global PLAYER-TYPE word
4480d6: [0x4ede34] := 0x1E0                     ;   clamp TERMINAL
4480de: ebx := 3                                ;   flash duration 3
        walk slot edx = 0..[0x46cbd8)-1, record := [0x46cbd4]+edx:
          skip dead (record+0x7C alive == 0)
          skip non-player (record TYPE +0x2A (=`dword@+0x28>>16`) ≠ [0x4edb90])
          skip currently-selected (edx == [0x46cbdc])
          → [0x46cbdc] := edx (AUTO-SELECT); [0x46ccec] := 3
            [0x4ede34] := 0 (xor-of-equals); [0x4ea8f8] := 0; CONTINUE walk
```
NO break on match — the LAST eligible slot wins. The eligible set = ALIVE player-type squad slots ≠ the dead selected one — i.e. any living squadmate (squad slots are player-type by construction, +0x2A row). Timeline: death frame cell=1 → blit v=479; frames 2..12: cell = 41..441 → v = 439..39; frame 12's increment hits 481 → clamp 480 + reselect. Cancel → next frame renders normal (v_old restored). No cancel → cell PARKS at 480: every later frame blits v := 480−min(480,479) = **1** (a 1×1 dot) until the fail detector (all-dead ∧ 0x1E0) fires. In SP "no cancel" ⟺ squad wiped — the two fail-detector conjuncts are the same event observed twice. MP never arms, so the whole machine is SP-only (consistent with D129's detector being SP-only).

**C. WHAT the temp render shows — the closing iris.** FUN_00401107 temp path 0x4012c7..0x4012f6: push [0x4ede54] (v_old); `v := 0x1E0 − min(cell,0x1DF)` @0x4012e5 (guaranteed ∈[1,480]); call 0x4012f7; restore. 0x4012f7:
1. **FIRST call 0x40129e** = the full-screen fill: 480 rows × 0x78 dwords of 0 to the visible page ([0x4edb3c]/[0x4edb40]) — palette-0/black.
2. Source window = the SAME base + fine-cam scroll-offset math as the normal path (backbuffer [0x4ede18]+0xA040, colAdj/rowAdj from [0x4edde4]/[0x4edde8] &0x1F) — but WITHOUT the normal path's (480−v)/2 source-centering add; the full 480×480 window is used.
3. Scales = the INVERSE of the normal zoom path: 0x454068/0x454060 := (0x1E0<<16)/v with halves 0x454064/0x45405c.
4. DEST is centered instead: edi := [0x4edb3c] + (480−v)/2 + (480−v)/2·[0x4edb40] — the v×v box centered at (240,240).
5. v row iterations of the row routine 0x401430 — the horizontal SHRINK twin of the normal path's 0x4013e8 stretch: v dest bytes ← 480 source bytes (`movsb; dec esi; esi += acc>>16; acc += (480<<16)/v`); between rows the source advances by whole 640-byte rows on a second sub-pixel accumulator (0x4013c9..0x4013dc) — over v rows the source covers exactly 480 rows.
Because FUN_00403938 skips its render body while the wipe runs (A.2), the backbuffer holds the last pre-death world frame — the iris shrinks a FROZEN snapshot [skip verified; frozen-frame consequence inferred]. Net effect: on the selected robot's SP death the view freezes and closes like an iris — a centered square window shrinking 40 px/frame per side from 479×479 to a 1×1 dot on black over ~13 frames; the user zoom [0x4ede54] is save/restored around it, untouched. This is the death cinema effect; the queue's "wipe/cinema" hypothesis confirmed with the iris geometry pinned.

**D. The §6c.6e flash-value correction.** §6c.6e says the auto-reselect "writes 0x46ccec = ebx(2)" — the code loads ebx := 3 @0x4480de, so the auto-reselect flash is **3** (same duration class as the FUN_00409138 death flash, per the [0x46ccec] values-2/3 grammar in §7j.57 D). Corrected in place below.

**E. The [0x4ea8f8] sibling — the MP death-position marker countdown** (8-site census): sole `≠0` producer 0x40e7ef := 0x20, in FUN_0040e230's MP branch (reached when [0x4edb88]≠0 ∧ extract-latch==0 ∧ dying == selected): posts the dying robot's position — [0x4ea8ec] := (rec+0x00)>>8, [0x4ea8f0] := (rec+0x04)>>8, [0x4ea8f4] := (rec+0x08) — then arms the 32-frame countdown. Sole reader/consumer: the FUN_00403938 head block 0x403974..0x4039a5 (runs only when the wipe is NOT active — the wipe gate 0x403952 precedes it): while ≠0, copies the trio into `[0x46ccdc]·12 + 0x4c71cc/0x4c71c4/0x4c71c8` — inside the §7j.56 DROPSHIP-ring 0x4c71b8 12-byte-stride bank region — and decrements (0x40399f). Exact rendering consumer of those cells not decoded (presentation-only, out of unit scope). Zero sites: the three click-select tandems (0x40d28c/0x40d317/0x40d39e), the auto-reselect cancel (0x448127), per-mission init (0x4478f1). Semantics: a ~32-frame hold of the dead selected robot's last position in the camera-anchor ring for the MP respawn HUD; cancelled by selecting another robot. The SP tail (0x40ea77) does NOT post it — SP death arms the iris instead. [Cross-ref, post-commit: the destination bank is ALREADY decoded — 0x4c71c4 = the §7j.20 ledger "per-player selected anchor" bank (4×0xC {x>>8, y>>8, z}) and its consumer is the §7j.54 chase-camera ring reader (SIM 0x40b8aa neighborhood, [0x4de654]≠0 loads the staged triple — SIM.md ~8901): the death post makes the camera system HOLD the dead robot's position instead of the live selected anchor. So the marker consumer is census-closed by §7j.20/§7j.54; no follow-up unit needed.]

**F. Engine/differ consequence: NONE.** The wipe is presentation-only (render-path + screen-transition control flow): zero RNG draws, zero robot-bank bytes, no dump-surface cell. The fail-detector TIMING consequence (fail waits for the 12-frame wipe to finish) is the same D129 class — a MissionShell screen-transition gate outside the dump surface. E's death tail already conforms; no watch rows, no differ changes. Future E-side render-parity note only: the iris grammar (fill-0, centered v×v, (480<<16)/v Q16 shrink both axes, +40/frame, frozen source) is now fully specified for the presentation layer.

## 7j.59. THE `_DAT_004dc5d0` BLINK-CURSOR CELL — CENSUS CLOSED: it is the BOMBARDMENT-WARNING squad-slot selector (value = the ENDANGERED robot's squad slot + 1 — the 2026-08-21 item-6 "SELECTED robot's slot" gloss corrected to an SP coincidence); exactly 7 .text sites (5 writers + 2 readers), all decoded; the {1,2,3} portrait gate = a literal 1/2/3 x-dispatch where 0 AND >3 both draw nothing; fully DISJOINT from the 0x4dc5d4 effect-row array (2026-08-23, worker 0329338f claim 2, D131; docs-only; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; MANIFEST.sha256 clean before AND after; registry_anchors green) [verified]

**A. The mechanical census.** A whole-objdump grep (`.text` 0x401000..0x460000) finds EXACTLY 7 references to 0x4dc5d0 — the complete site set, no other addressing form exists (Watcom absolute statics; no lea / register-indirect site):

| site | kind | code |
|---|---|---|
| 0x40c1d7 | WRITE 1 | `mov [0x4dc5d0],ebx` (ebx=1, arm strip k=0) |
| 0x40c217 | WRITE 2 | `mov [0x4dc5d0],edi` (edi=2, arm strip k=1) |
| 0x40c254 | WRITE 3 | `mov dword [0x4dc5d0],0x3` (arm strip k=2) |
| 0x423fef | WRITE 0 | `mov [0x4dc5d0],ecx` (ecx=0, shell impact-completion tail) |
| 0x447871 | WRITE 0 | `mov [0x4dc5d0],ecx` (ecx=0, MissionShell per-mission reset) |
| 0x407428 | READ | the §6c.6d portrait-pass blink gate |
| 0x423e91 | READ | the §7j.54 chase-camera record-0 impact gate |

**B. The value grammar — {0,1,2,3} only, and what 1/2/3 ARE.** The three `:= k+1` writes are the UNROLLED per-slot strips of the robots() idle-arm tail (§7g.5/§7j.54), one per squad slot [0x40c1ae..0x40c25e, verified]:

- k=0 @0x40c1ae..0x40c1d7: `idx == [0x46cbd4]` (NO size gate — slot 0 always exists) → posts the warning PAIR `FUN_004239ef(0xC, 0)` + `FUN_004239ef(0xF, 0)` (§7j.53 ids: DANGER—UNIT 1 TARGETTED / IMMINENT AERIAL BOMBARDMENT) → `[0x4dc5d0] := ebx = 1`.
- k=1 @0x40c1dd..0x40c217: `idx == [0x46cbd4]+1` ∧ `[0x46cbd8] > 1` → posts `(0xD, 1)+(0xF, 1)` → `[0x4dc5d0] := edi = 2`.
- k=2 @0x40c21d..0x40c254: `idx == [0x46cbd4]+2` ∧ `[0x46cbd8] > 2` → posts `(0xE, 2)+(0xF, 2)` → `[0x4dc5d0] := 3` (immediate).

All three strips fall through the SHARED tail 0x40c25e..0x40c351: the robot's +0x70 idle counter := 0, `[0x4de658] := 0x80` (salvo cooldown latch), the 8-shell scatter into 0x4ea238 (§7j.54). So the written value = **the ENDANGERED robot's slot-within-the-local-squad + 1**. CORRECTION (landed in the 2026-08-21 amendment item 6 below, history preserved): its gloss "value = the SELECTED robot's SLOT + 1" holds only as an SP COINCIDENCE — in SP the sole robot whose idle counter arms is the selected one (§7j.54 gate `idx == [0x46cbd4]+[0x46cbdc]`), but in MP every idle robot arms, and the arm gate and the write gate are DIFFERENT comparisons: the write names the tripped robot's own slot.

The `:= 0` pair:
- 0x423fef — FUN_00423e1c's per-record impact-completion tail: after the 6 kind-6 debris + the 3×3 nine-blast patch (§7j.54), `xor ecx,ecx; xor edi,edi` → `[0x4dc5d0] := 0` + the record's valid word `[rec+8] := 0` (0x423fef/0x423ff5, rec 0 ⇒ 0x4ea240). ANY of the 8 shells that lands first clears the cursor.
- 0x447871 — the MissionShell per-mission reset cascade (§7j.58's block: `xor ecx,ecx` @0x44785e, callee-saved across FUN_0041d954; the cursor store sits between the map-overlay 0x4edba0 and salvo-cooldown 0x4de658 zeroes it travels with).

No writer ever stores any other value; the consumer's `>3` branch is dead-defensive.

**C. The {1,2,3} gate semantics — pinned against the effect-row family.**
1. The consumer @0x407420..0x407449 is a LITERAL 1/2/3 dispatch, not a bit test [verified]: `cmp edx,2; jb → (cmp edx,1; je) 0x407948` / `jbe 0x407962` / `cmp edx,3; je 0x407973` / `else jmp 0x4072b8`. Value 1/2/3 selects the portrait-strip x = 0x1F0/0x222/0x254 (squad slot edx−1); **0 AND >3 both draw NOTHING** (0x407449 / 0x407989 skips). Shared draw 0x407948..0x407958: `FUN_00401ca2((g_frame_count & 3) + 0x51, 1, x, 0xD)` from GENERAL.BIN ([0x4edd7c]) — the 4-frame blink animation. The §6c.6d text was right on the sprite; its "(its producer is open)" residue and the "sprite-list field" naming are now closed/corrected.
2. 1/2/3 are NOT blink classes and NOT FLAGS.BIN effect ids: the effect-row family (10×16 B rows at 0x4dc5d4..0x4dc67c, ids 1..0xE at 0x4dc5e0+r·0x10, drawn as FLAGS.BIN sprite id−1) is a DISJOINT array 4 B ABOVE the scalar — no site reads or writes across the boundary (the FUN_00422038 allocator scans 0x4dc5e0+k·0x10 only). The cell is not part of any sprite/effect LIST; it is the warning-cursor scalar. The queue's "what 1/2/3 mean — blink classes?" resolves: they are 1-based squad-slot indices, exactly one cursor position.
3. The second reader PROVES the index semantics arithmetically: FUN_00423e1c's record-0 impact block 0x423e7c..0x423ed5 (SP ∧ rec 0) computes the endangered robot's BANK INDEX `[0x46cbd4] + ([0x4dc5d0]−1)` ×0xA8 → record `+0x00 >> 16 == [0x4edb90]` (player-type) ∧ `[0x46cbdc]+1 ≠ [0x4dc5d0]` (endangered ≠ selected) → FUN_004245c9 chase-camera cut (§7j.54's gate, now site-exact).

**D. Lifecycle.** 0 at mission entry (0x447871) → := endangered-slot+1 at the idle-threshold arm (with the DANGER/BOMBARDMENT warning pair, SFX, the salvo scatter) → 0 at the FIRST shell impact (0x423fef — the earliest shell lands ≈ arm+32..46 frames of start-delay + ~8 frames of fall) → re-armable only after the [0x4de658] cooldown ticks to 0 AND the idle counter re-thresholds. Ordering the robot resets the idle counter (§7j.54), so an actively-used squad never blinks.

**E. Engine/differ consequence: NONE.** Both readers are SP-UI presentation (portrait blink, camera cut); zero sim-state reads, zero RNG draws. The DESIGN §5 S1 "blink-cursor-from-spawn" hypothesis is now decidable STATICALLY: the cell is 0 from spawn and stays 0 unless the idle threshold trips — no corpus scenario reaches it (§7j.54's idle table {400,300,200,5000} frames vs the scripted scenario horizons), so E's never-fabricating presentation is faithful and the S1 watch row should read constant 0 on both channels. The watches.toml layout "u32 (0 or slot+1)" stays accurate (the slot in question is the endangered one). **EXD TWIN PINNED (D132, 2026-08-23 — closes the W1 gap this section recorded): [0x10e108], EXACTLY 7 .text sites mirroring the census above one-for-one** (arm strips 0x1cef1/0x1cf2c/0x1cf72 ⟷ 0x40c1d7/0x40c217/0x40c254; impact-clear 0x34f7f ⟷ 0x423fef; MissionShell reset 0x59842 ⟷ 0x447871; portrait gate 0x186dc ⟷ 0x407428 with the identical (frame&3)+0x51 / 0x1F0/0x222/0x254 / y=0xD dispatch; chase gate 0x34e25 ⟷ 0x423e91 with the identical ([base]+[cursor]−1)·0xA8 kind-vs-player-type arithmetic); value grammar, gate set, and the idle table bytes all identical both builds; the census also produced the selection-triple label-swap correction + ten §5e cascade/asset aliases — RE-EXD-MAP §5/§5e.

## 7j.60. THE [0x4eddec]/[0x4eddf0] MAP W/H CELLS — WRITER/READER CENSUS + THE W11 O2 CAPTURE-FORM PIN: the last deliberate zero-field differ row closes (2026-08-23, worker a3532435 claim 2, D137; RE notes committed BEFORE the impl per the stream-survival rule; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; MANIFEST.sha256 clean before AND after) [verified]

**A. The mechanical census.** Whole-objdump greps over the `.text` listing find 97 references to 0x4eddec and 85 to 0x4eddf0 (absolute `ds:`-displacement addressing throughout; no lea/register-indirect form exists for either cell). The WRITER census is EXACTLY 6 stores — found only by grepping BOTH store encodings (`89 1d` `mov DWORD PTR ds:` AND the short A3 form `mov ds:`; a DWORD-PTR-only grep undercounts by half):

| site | store | function |
|---|---|---|
| 0x41dd52 | `mov DWORD PTR ds:0x4eddf0,ebx` (ebx=0) | FUN_0041dc5a — pre-clear, before the header parse (xor ebx,ebx @0x41dd35) |
| 0x41dd5b | `mov DWORD PTR ds:0x4eddec,ebx` (ebx=0) | FUN_0041dc5a — pre-clear pair |
| 0x41dd6a | `mov ds:0x4eddec,eax` = `movsx` u16[ptr+0] | FUN_0041dc5a — **W** from the .TOT header ([0x4ede20] pointer) |
| 0x41dd83 | `mov ds:0x4eddf0,eax` = `movsx` u16[ptr+2] | FUN_0041dc5a — **H** from the .TOT header |
| 0x446688 | `mov ds:0x4eddec,eax` = `movsx` u16[ptr+0] | FUN_0044661b — the EDITOR\ZONE restore reload (§7j.16), same header shape off [0x4ede20] |
| 0x4466a1 | `mov ds:0x4eddf0,eax` = `movsx` u16[ptr+2] | FUN_0044661b — same |

FUN_0041dc5a = the map-volume loader (sole MissionShell call site 0x447b3a; §7j.16 ledger row) — the decomp walk (ghidra-project/exw-gamemainhop.txt 2178..2191) matches instruction-for-instruction. Immediately after the two header stores it derives the plane stride `imul` H·W → **[0x4eddf4]** @0x41dda5 and fills the row-base table 0x4ea900 (h entries) — the same shape FUN_0044661b repeats at 0x4466a6..0x4466b2.

Every remaining site (95 for W / 83 for H) is a READER; no game-state writer exists. The reader families already pinned in earlier sections: the D124 debris-stage bounds (`x < [0x4eddec]<<5 ∧ y < [0x4eddf0]<<5`, all 12 in-map recheck legs), the §7j.26 effects-mover kill bounds (`x>>13 ≥ [0x4eddec] ∨ y>>13 ≥ [0x4eddf0] ∨ z>>13 > 0xB`), the map-overlay loops (0x408a19/0x408a6a), the cursor clamps (0x46cd04/0x46cd08, exw-input-readers), the §7j.47 restamp z-stack bound, the per-tile territory walk (§7 row-loop `0..H`/`0..W`) — the cells are the map-dims oracle for the whole engine.

**B. The value grammar.** Both cells are u32 holding SIGN-EXTENDED u16 .TOT header words (w @+0, h @+2; FORMATS §2 + the §7j.16 ledger row). For every corpus map the values are the TOT header dims (ZONEA/MISSION1 = 25×75 — the D85 canonical finding). The sibling [0x4eddf4] = the W·H plane stride; it is NOT part of this watch row.

**C. The EXD twins + the span-form asymmetry.** RE-EXD-MAP §5b [verified]: EXW **0x4eddec (W) / 0x4eddf0 (H)** ⟷ EXD **0x1074b8 (W) / 0x10748c (H)**; EXW plane-stride 0x4eddf4 ⟷ EXD w·h product cell 0x1074e4. THE GAP IS ASYMMETRIC between the builds: the EXD pair sits **0x2c apart with h LOW** (h 0x10748c < w 0x1074b8 — the O1 row's 0x30 span reads h@+0x00, w@+0x2c), while the EXW pair sits **4 apart — adjacent u32 cells, w LOW** (w 0x4eddec, h 0x4eddf0, the stride cell 0x4eddf4 immediately after; 0x4eddf0−0x4eddec = 4). The field order IS reversed relative to address order (O1's low cell is h, O2's low cell is w), so the O2 raw form is still NOT the O1 form with relabelled fields. **[CORRECTED 2026-08-24, D138 — this section + the D137(2) pin originally recorded the EXW pair as "0x24 apart", an arithmetic impossibility for the cells both quote (0x4eddec+0x24 = 0x4ede10 ≠ 0x4eddf0); the dbx-plan O2 compiler's registry-derived span assert caught it. The A-table store sites + the whole reader census were always correct; only the C/D gap arithmetic was fabricated.]**

**D. THE W11 PIN — the O2 capture form.** One contiguous span covering EXACTLY the two cells, mirroring the EXD precedent (one contiguous read, both cells, product cell excluded both sides): **base 0x4eddec, len 8 — w @+0x00, h @+0x04** [CORRECTED 2026-08-24, D138; originally recorded as len 0x28 with h@+0x24 — impossible per C]. `normalize_o2_row`'s static-map-wh arm parses this into the canonical (w, h) fields; the W11 capgen/ptrace driver emits the span. The differ_gate O2 fabrication emits the same 8-byte form and the E-vs-O2 cross compares the row CLEAN (the pre-pin zero-field arm would have shown the row as 2 field-level coverage gaps).

**E. Engine/differ consequence.** Engine: NONE — the E canonical row already carries the TOT-header w/h (D85: staged from the .TOT header, e.g. 25/75 on ZONEA/M1); the pin only defines the GUEST capture form. Differ: `normalize_o2_row` gains the parse; the O1 arm (0x2c span, h low) and the E arm (bare w,h pair) are unchanged; the fabricated-O2 arbitration lanes and a new E-vs-O2 cross assertion exercise the arm headless. Registry row `static-map-wh` layout note amended with the form.

## 7j.61. THE .BDG TYPE-TABLE STAGING — RE-VERIFIED INSTRUCTION-BY-INSTRUCTION (table+arena memset pre-zero, the control word staged at +0 BEFORE the test, the count word computed on ACTIVE rows only, the four-bank arena arithmetic) + ONE FORMATS §16 CORPUS ERRATUM (footprint max is (10,10,5), NOT (3,3,3)) (2026-08-25, worker e473f5db claim 1, D148, the P4/static-parity/S0-09 unit; objdump-only from ghidra-project/exw-text-objdump.txt + exd-text-objdump.txt — no Ghidra run; read-only corpus census of all 37 .BDG files; MANIFEST.sha256 clean before AND after) [verified]

**A. The loader walk (EXW FUN_0041a4f8, .BDG leg 0x41a5d6..0x41a7ef).**

1. **TWO memset pre-clears before the file is even opened** (the D146 PAD finding repeated for this loader, and one bank further): `FUN_00402965(ecx=0x55ec, edi=0x4dedf2)` @0x41a501 zeroes the WHOLE 282×0x4E = 21996-B table (0x55EC is exactly 282·0x4E — no slack), and `FUN_00402965(ecx=0x9c40, edi=[0x46ad60])` @0x41a52f zeroes 40000 B of the bank ARENA whose base pointer lives at 0x46ad60. At .BDG open the cursor is reset to the arena base: `[0x46ad5c] := [0x46ad60]` @0x41a5e5. No cross-mission stale tail can survive in the table rows, their count words, their pointer slots, or the arena.
2. **Loop bound: exactly 282 records.** EXW counts records (`cmp ecx,0x11a; jge done` @0x41a638); the EXD twin bounds on the row OFFSET (`cmp ebp,0x55ec; jne` @0x2b05b) — the same 282 (0x55EC = 282·0x4E).
3. **The control word is STAGED at row+0 before the ==1 test** (reads at 0x41a649..0x41a650, test `sar` of the dword at row−2 @0x41a655..0x41a661). The §7j.25 ledger's "word@+0 unconsumed [open]" is closed mechanically: +0 holds the raw disk control word (1 on active rows; 0 on every one of the 2527 corpus empty rows — no other value occurs). No consumer reads it.
4. **Empty rows (control ≠ 1) advance 2 disk bytes and write NOTHING else**: bytes +2..+0x4E of the row stay memset-0 — head, count, effects, and all four bank pointer slots (NULL). The count loop is SKIPPED on this path (`jne 0x41a623` bypasses it; EXD `jne 0x2b044`).
5. **Active rows** stage control@+0, W@+2, H@+4, D@+6 (u16), hp i32@+8, chain u16@+0xC, type i32@+0xE, then FIVE 8-B effect entries (selector/dx/dy/dz u16) at +0x16+8m, m=0..4 (`cmp ecx,0x28` exit @0x41a6f7 — ecx walks +8 per entry). All fields are read 2/4 B at a time through `FUN_0041cccb(eax=dest, edx=len)` = the file-read helper (same helper the .POS leg uses for its 2000×0x10 reads @0x41a579).
6. **The bank byte count is recomputed from the STAGED words, not the disk** (0x41a6fc..0x41a71a: dword@row+0 `sar 16` = W, @+2 = H, @+4 = D — i.e. the staged W/H/D drive everything downstream): `ebx = 2·W·H·D` bytes per bank.
7. **The four banks are read into CONSECUTIVE arena slots in DISK ORDER, cursor += ebx after each read** (0x41a71d..0x41a793): read#1 at the cursor → slot **+0x3E** (`[k·0x4E+0x4dee30]` @0x41a727), read#2 → **+0x46** (0x4dee38 @0x41a742), read#3 → **+0x42** (0x4dee34 @0x41a75d), read#4 → **+0x4A** (0x4dee3c @0x41a77c). This is the §7j.32 interleave at instruction level, and it adds the arena-shape fact: **the arena image is exactly the file's bank bytes concatenated in file order** — the current/under interleave lives ONLY in the row's pointer slots, never in arena layout.
8. **count@+0x12 is computed AFTER the banks, on the STAGED selectors, for ACTIVE rows only**: a 5-step loop (`add ebx,8` under `cmp ebx,ecx(=0x28)` @0x41a609..0x41a61e) counts selector words ≠ 0 at `[k·0x4E+0x4dee08]` (= row+0x16) and stores the sum at `[k·0x4E+0x4dee04]` (= row+0x12) @0x41a61d.
9. **Tail** (0x41a7a8..0x41a7ef): close file; walk the live instances — `hp@[instance+0x10] := dword@[type·0x4E+0x4dedfa]` (= row+8, the staged hp) @0x41a7d3, then the footprint stamper FUN_0041a7f0(ecx=instance idx+1, eax=x, edx=y, ebx=type) per instance.

**B. Displacement census — the reader side, once and for all (whole-objdump grep, absolute `ds:`/register-displacement forms; the row k·0x4E-indexed family):**

| displacement | row field | sites | verdict |
|---|---|---|---|
| 0x4dee04 | count@+0x12 | **1** — the loader store @0x41a61d | **ZERO runtime readers.** The load-computed count is write-only state. |
| 0x4dee30 | bank slot +0x3E (CURRENT TOT) | 1 — the loader store @0x41a727 | dead editor payload (§7j.32 confirmed at displacement level) |
| 0x4dee34 | bank slot +0x42 (CURRENT DAT) | 1 — the loader store @0x41a75d | dead editor payload |
| 0x4dee38 | bank slot +0x46 (UNDER TOT) | 2 — store @0x41a742 + **reader @0x41ab59** (destroy restore TOT-mirror words) | live |
| 0x4dee3c | bank slot +0x4A (UNDER DAT) | 3 — store @0x41a77c + **readers @0x41ab72/0x41ab8a** (seen byte + DAT volume) | live |
| 0x4dee08 | selectors @+0x16 | 20 — the loader count loop @0x41a610 + the destroy-tail effect cases (0x41ac58..0x41b73a) | live |

**C. The EXD twin FUN_0002adb4 (0x2adb4..0x2b0ae) is instruction-for-instruction the same source**: table 0x108428 (memset 0x55EC via twin 0x12206 @0x2adc7), arena pointer cell 0x119604 + cursor 0x1195f8 (memset 0x9C40 @0x2adf0, cursor reset @0x2ae93/0x2ae9d), read twin 0x2d5c8, control test @0x2aecd, same field offsets, slots 0x108466/0x10846e/0x10846a/0x108472 (= +0x3E/+0x46/+0x42/+0x4A) @0x2afb7/0x2afd2/0x2aff1/0x2b00e, count loop 0x2b02c..0x2b03e storing `[ebp+0x10843a]` (row+0x12), instance hp seed `[eax·0x4E+0x108430]` @0x2b086.

**D. CORPUS ERRATUM + census (all 37 files, byte-exact EOF consumption, exactly 282 records each — 10434 total, 7907 active, 2527 empty; re-confirming §16/§7j.25):**
- **FORMATS §16's footprint claim "corpus mostly (1,1,1)..(1,1,4), max (3,3,3)" is WRONG on the max**: there are **113 distinct (W,H,D)** tuples; W ≤ 10, H ≤ 10, D ≤ 8; the largest is **(10,10,5) = 500 cells** (ZONEF/MISSION1 record #184); (1,1,1) alone covers 3581 records. (3,3,3) is merely one of the common mid cubes (82).
- Empty-row control value: **0 on all 2527** empty rows (never any other ≠1 value).
- Selector domain on disk: 0 + exactly 1..9 (23976 zeros; 11098/1490/1385/402/330/304/316/178/56 for 1..9 — re-pins §7j.25 item 8).
- Nonzero-selector count census (the value the count word takes on active rows): 0→554, 1→3755, 2→1304, 3→884, 4→506, 5→904. **554 active rows stage count 0** — the count word is NOT a presence flag.
- Arena span: 6728 B (min) .. **27288 B (ZONEF/MISSION1)** per mission — always < the 0x9C40 memset bound.
- hp domain: −1, 0, 1, 5, … 9000, 15000, 18900 (NEGATIVE hp exists on disk); chain domain {0,1} only; type census head: 15×1528, 5×814, 30×723, 11×516, 120×507, 0×667, 20×420, 90×475, 999999×2.
- ZONEA/MISSION1 anchors: record 0 = control 1, (W,H,D)=(1,1,1), hp=150, chain=1, type=15, effects [(1,0,0,0),0,0,0,0] (count 1), banks [53]/[1189]/[2]/[0] (the +0x4A word 0 ⇒ restore seen=1); 197 active / 85 empty; 26 active rows with count 0; record #19 carries five selector-1 entries.

**E. Engine/differ consequence.** Rust retains the row bank (`ObjectTypeTable::from_bdg_bytes` in `destroy.rs`, staged verbatim into `MissionSim::object_types` by `stage_destroy_family` @mission.rs:725 — the four banks under their disk-order names, all head fields, effects; empty rows = `ObjectType::default()` ≙ the memset-0 row). **The count word is NOT retained (stays 0) — deliberately**: it is a pure function of the retained effect selectors (nonzero count) AND, per census B, write-only in the original; adding it would be fabricated parity, so none is added. The staged word@+0 (disk control) is likewise unretained (0/1 classification retained instead) — also write-only. The watch row `static-type-table` (base 0x4dedf2, 282×0x4E) stays accurate as captured; its layout note gains the count/arena semantics. The S0-09 oracle (`static_type_table_differential.rs`) is the independent whole-corpus differential built from this section.

## 7j.62. THE `.MIN` MASK BANK — RE-VERIFIED INSTRUCTION-BY-INSTRUCTION (arena alloc 0x7530 with NO zeroing, a VERBATIM whole-file read with no header/transform/memset — unlike PAD/BDG — a zone-scoped path, and exactly ONE runtime reader: the 4×4 territory stamp) + THE STALE-TAIL-NEVER-READ CORPUS PROOF (2026-08-25, worker 95c99db8 claim 1, D149, the P4/static-parity/S0-10 unit; objdump-only from ghidra-project/exw-text-objdump.txt + exd-text-objdump.txt — no Ghidra run; read-only corpus census of all 7 .MIN + 37 TOT + LNK/LNG files; MANIFEST.sha256 clean before AND after) [verified]

**A. The allocation (FUN_0041d954, the GameMain arena pass; EXW 0x41dabd..0x41dac7):**
`mov eax,0x7530; call 0x41db89; mov ds:0x4edd9c,eax` — ArenaAlloc(30000) with
the result stored in cell [0x4edd9c]. FUN_0041db89 (0x41db89..0x41dbd4) only
bumps the arena cursor 0x46af0c (with the 0x46af10 high-water check +
0x420100 grow/abort path) and returns the OLD cursor — **no zeroing anywhere**
(contrast: the PAD slots @0x41de62 and the BDG table/arena §7j.61 are memset-0
BY THEIR LOADERS; the MIN bank is never memset by anyone).

**B. The loader leg (FUN_0041dc5a, the mission family loader; EXW 0x41dcd8..0x41dcf3):**
`edx=0x4587ed` (the ".MIN" tag, entry 5 of the 8×5-B tag table
0x4587d9..0x4587fc), `eax=0x4dca8c` (the ZONE-scoped path buffer),
`ebx=[0x4edd9c]`, `call 0x41dbed` (path concat), `edx=ebx`,
`call 0x41cc7f` (LoadFile). The path buffer 0x4dca8c is built by FUN_0044670c
(0x446820..0x44690f) as the SECOND string triple's product:
"EDITOR\\"(0x4597ba) + "ZONE"(0x4597c2) + zone letter([0x4edd8c]+0x40) +
"\\MISSION"(0x4597c7) + zone letter AGAIN — i.e.
`EDITOR\ZONE{X}\MISSION{X}.MIN`, **ZONE-scoped, not mission-scoped** (corpus
confirms: exactly 7 files MISSIONA.MIN..MISSIONG.MIN; every mission load of a
zone re-reads the identical bytes). FUN_0041cc7f(eax=name, edx=dest) opens
(FUN_0041cd90), sizes (0x44e30b → 0x4eded4), rewinds (0x44e217), then
FUN_0041cccb reads EXACTLY `size` bytes into dest in ≤0x80000 chunks
(0x41cccb..0x41cd13) — **the whole file lands in the bank verbatim: no header
skip (unlike TOT/DAT's +4), no transform (unlike DAT's ≥0x80→0 sanitize), no
bounds check against 0x7530** (a hypothetical >30000 B .MIN would clobber the
following arena allocations — unchecked in the original, never shipped; the
tightest real file is 29952 B, 48 B under). Consequence for the tail: bytes
[0x7530·0 …) beyond the file prefix are STALE ARENA — but see D.

**C. The consumer census (whole-objdump grep of 0x4edd9c): exactly 3 .text
sites** — the alloc store @0x41dac7, the loader read @0x41dce2, and ONE
runtime reader @0x402acb inside FUN_00402ab8 (0x402ab8..0x402af6) = the 4×4
territory stamp:
```
edx = (edx*5)<<7 + ecx + [0x4ede18]   ; dest = backbuffer + y'*640 + x'
eax <<= 4                              ; cw*0x10
esi = [0x4edd9c] + eax                 ; mask = MIN bank + cw*16
ecx = 4 rows, edx = 4 cols:
  al = [esi]; al==0 → transparent; else al = xlat EBX (ramp base); [edi] = al
  row pitch 0x27c (= 640−4)
```
The sole caller loop (0x408a8e..0x408ae3, the §7e/§6c.9 MAP overlay pass):
per tile column × 8 z-planes (ebp = 0,2,..0xE) of the TOT mirror (base
0x4796bc, 0x1E row pitch — the dword read at 0x4796ba+tile·0x1E+ebp takes the
mirror word AT 0x4796bc+…+ebp as its upper half, so **the lookup key = the
raw TOT u16 word of (tile, plane)**): `edx = word@[edx*2+0x45cdd8] sar 16`
= **cw = LNK_word[type]** (the zone-level MISSIONX.LNK/LNG image at 0x45cdda,
16384 B = 8192 words; loaded by the same family loader @0x41dd09..0x41dd18
behind the language gate `cmp [0x4eba1c],1` → .LNG 0x4587f2 else .LNK
0x4587f7); the resolved word is written back into the live mirror
@0x4796bc; `cw==0 → stamp skipped` (`test edx,edx; je` @0x408abd); the XLAT
ramp = **MAPTRAN[territory-variant byte @[tilebase+0x4c420c]]**
(`mov al,[ebx+0x4c420c]; mov ebx,[eax*4+0x4dd464]` @0x408ac7/0x408acf — §7e
items 2–4). This pins the FIRST verified runtime consumer of the LNK image's
permutation: **LNK cycles = rotation/variant links between adjacent 16-B mask
entries** — FORMATS §5's "next orientation" hypothesis gains its anchor (the
lookup is a plain table read here, not a chain walk).

**D. Corpus census (all 7 zones × their missions; sizes exact, all files
16-B entry multiples, all < 0x7530):**

| zone | MIN bytes | entries | reachable nz cw (LNK/LNG/union) | max cw (LNK/LNG) | all-zero entries in union | distinct mask bytes (max) |
|---|---|---|---|---|---|---|
| A | 23200 | 1450 | 349 / 337 / 349 | 1356 / 1356 | 9 | 119 (254) |
| B | 29952 | 1872 | 1180 / 1146 / 1180 | 1868 / 1868 | 11 | 181 (254) |
| C | 27888 | 1743 | 1054 / 1026 / 1055 | 1741 / 1706 | 12 | 180 (254) |
| D | 23200 | 1450 | 1008 / 988 / 1008 | 1356 / 1356 | 10 | 176 (254) |
| E | 23280 | 1455 | 949 / 929 / 954 | 1398 / 1400 | 9 | 154 (254) |
| F | 15824 | 989 | 632 / 632 / 632 | 960 / 960 | 9 | 170 (254) |
| G | 29952 | 1872 | 271 / 271 / 271 | 1834 / 1834 | 2 | 100 (223) |

- **ZONEA/MISSIONA.MIN ≡ ZONED/MISSIOND.MIN byte-for-byte** (same content,
  different reachable sets because the zones' TOT/LNK differ).
- Reachable = { LNK_or_LNG[type] : type ∈ all TOT u16 words of every mission
  of the zone, all 8 planes } — a strict SUPERSET of the runtime overlay
  lookups (the 0x408a49 pass walks a tile window of the mirror; the census
  takes every word). **Every nonzero reachable cw satisfies cw·16+16 ≤
  file size in all 7 zones under BOTH the LNK and LNG gates** (tightest:
  ZONEB 1868·16+16 = 29904 ≤ 29952; ZONEG 1834·16+16 = 29360 ≤ 29952) —
  **the stale arena tail beyond the file prefix is NEVER read at runtime.**
- The language gate is NOT cosmetic for this bank: the LNK and LNG images
  reach DIFFERENT entry sets (A: 349 vs 337; C: union 1055 > either alone;
  E: union 954 > LNK's 949) — localized map overlays rotate differently.
- Max TOT type over the whole corpus = 1868 < 8192 words — every lookup
  stays inside the 16384-B LNK/LNG image too.
- Per-mission max reachable cw (identity pin): A/M1 1356; B/M1..M5 1868,
  B/M6 1812, B/M7 1814; C/M1..M4 1706, C/M5 1741, C/M6..M7 1633; D/M1..M5
  1356, D/M6..M7 1344; E/M1 1398/1400, E/M2 1390/1389, E/M3 1384, E/M4
  1361, E/M5 1375, E/M6..M7 1347; F/M1..M5 960, F/M6 915, F/M7 809;
  G/M1 1834 (LNK/LNG where they differ).

**E. The EXD twin is instruction-for-instruction the same source**: alloc
`mov eax,0x7530; call 0x2e4b2; mov ds:0x107538,eax` @0x2e3e6..0x2e3f0
(same 0x7530), loader leg @0x2e641..0x2e658 (tag 0x862bd ".MIN", buffer
0x92f34, cell [0x107538], concat twin 0x2e55a, LoadFile twin 0x2d57c), and
the reader twin FUN_00012df3 @0x12df3..0x12e31 (identical 4×4/0x27c/XLAT
body, backbuffer 0x10745c); its sole caller loop 0x197da..0x19841 mirrors
EXW 0x408a8e..0x408ae3 exactly (mirror 0xac1e2/0xac1e4 at 0x1E pitch, LNK
lookup `[edx*2+0x10336a] sar 16` = word@0x10336c+2·type, cw==0 skip
@0x1981d, variant byte 0x9ee34, ramp table 0x92bfc). The 0x107538 census
is likewise exactly 3 sites.

**F. Engine/differ consequence.** The `.MIN` bank is a PRESENTATION-half,
verbatim file image (D17): its only consumer is the map-overlay territory
stamp — backbuffer bytes, never engine state, never in the hash surface.
Rust retains NOTHING of it (no `min` field anywhere in bedlam-core), and no
seam is added by this unit: a retained `Vec<u8>` with zero Rust consumers
would be fabricated parity. The watch row `static-min-bank` (EXW 0x4edd9c /
EXD 0x107538, extent "0x7530 (30000 B)" = the pinned 0x7530 arena alloc with
the zone-file prefix 15824..29952 B) keeps its O2 capture form; the dbx-plan
deferred extent is RESOLVED to `Form::PtrCell { cell, len_expr: "0x7530" }`
(D152, S0-12a — landed after this section's queue note; the row left the
plan `_deferred` list on both channels, `min_ptr` resolve symbol). The S0-10
oracle
(`static_min_bank_differential.rs`) is the independent whole-corpus
differential built from this section: loader transcription (verbatim
prefix, stale tail proven unreachable), the LNK/LNG→cw consumer projection
over all 37 missions, the stamp semantics, and the corpus identity pins.

## 7j.63. THE TILE-CLAIM BANK (0x46af58) INITIALIZATION DECODED WHOLE — it is the DOOR-RECT TILE CLAIM map: a per-mission memset-0 + the stamp of the ACTIVE PREFIX of the 45-rect door list; the §7j.10 "ORDER marker family 0x425556" gloss RETIRED; a NEW 4th reader (the radar marker-0xd gate) (2026-08-25, worker eeafac37 claim 1, the P4/static-parity/S0-11 unit; objdump-only from ghidra-project/exw-text-objdump.txt + exd-text-objdump.txt — no Ghidra run; read-only byte probes of BEDLAM.EXW; scratch /tmp/opencode; MANIFEST.sha256 clean before AND after) [verified]

**A. The whole `.text` census: EXACTLY 7 sites, EXW ⟷ EXD 7-for-7.**
A raw grep of `0x46af58` over exw-text-objdump.txt (and the byte-verified
call at 0x447b7b re-read from the EXW file image, delta 0x400C00) closes the
displacement census — no site writes through a stale register copy (any
writer must load the pointer first, and every load site is accounted):

| # | EXW | EXD twin | kind | function/role |
|---|-----|----------|------|---------------|
| 1 | 0x41d9d7 | 0x2e300 | pointer store | mission-load arena pass (D below) |
| 2 | 0x41f191 | 0x2fb8c | READER | NEW: the radar/map-overlay pass FUN_0041ee20 — `cmp byte[edx+claim],0; je skip` gates marker id 0xd (vs marker 7 on the other branch): claimed tiles draw door/marker 0xd on the overlay |
| 3 | 0x422931 | 0x33857 | READER | FUN_004228ce platform tile build gate (§7j.41/2) |
| 4 | 0x423858 | 0x347cf | READER | FUN_0042382c death-blast smoke producer gate (§7j.24) |
| 5 | 0x4243e4 | 0x35349 | READER | FUN_00424355 splash stager gate (§7j.10/§7j.14) |
| 6 | 0x4254ec | 0x36589 | memset load | FUN_004254e1 head (E below) |
| 7 | 0x425556 | 0x365d4 | WRITE | FUN_004254e1 stamp loop: `mov byte[ecx+claim],1` |

**No order-marker writer exists at all.** The §7j.10 gloss "written 1 by the
ORDER marker family 0x425556" is RETIRED: 0x425556 is the inner-loop store of
FUN_004254e1 — the MISSION-LOAD initializer (C below). The ledger row
"tile-claim bank … order-marker writer 0x425556" and the engine-side comments
("the D82 order-marker writers are the unmodeled seam") are corrected by this
section: there is exactly ONE writer family and it runs once per mission load.

**B. The initialization chain (MissionShell mission load, verified
byte-order 0x447b6c..0x447b8a):**
```
447b6c: mov ecx,0x2d0
447b71: mov edi,0x4dcae8        ; (dead setup — see below)
447b76: call 0x41a4f8           ; .POS/.BDG loader (push/pop-guarded ecx/edi — pair survives, unused)
447b7b: call 0x402965           ; MEMSET-0: 0x2d0 B at 0x4dcae8 — clears ALL 45 rect records
447b80: call 0x42c4a0           ; the per-zone/mission HARDCODED rect filler
447b85: call 0x4254e1           ; THE CLAIM INITIALIZER (C below)
```
(§7j.21 item 3's "MissionShell clears it @0x447b7b" confirmed byte-exact —
an early terminal-transcription slip in this unit's own log was corrected
against the file image: the target IS 0x402965 = memset, NOT 0x402975 = RandA;
the two helpers are 10 bytes apart and near-identical in call shape.)

**C. FUN_004254e1 = the claim initializer (EXW 0x4254e1..0x425567; EXD twin
0x3657e..0x365fe instruction-equivalent, rect bank alias 0x92c64, line-table
alias 0x8b78c):**
```
ecx := 0x2710 (10000); edi := [0x46af58]; call FUN_00402965   ; MEMSET-0 the WHOLE bank
ebp := 0                                                       ; rect record offset
loop: if ebp >= 0x2d0 -> ret
      if word@0x4dcae8+ebp == 0 -> ret      ; STOPS at the first inactive record
      for esi (row) in 0 .. word@(rect+8)   ; h  (loaded as dword@rect+6 >> 16)
        for edx (col) in 0 .. word@(rect+6) ; w  (loaded as dword@rect+4 >> 16)
          ebx := word@(rect+4) + row        ; y0 (loaded as dword@rect+2 >> 16)
          ecx := word@(rect+2) + col        ; x0 (loaded as dword@rect+0 >> 16)
          ecx += [ebx*4 + 0x4ea900]         ; tile = line[y] + x
          byte@[[0x46af58] + ecx] := 1      ; CLAIM
      ebp += 0x10
```
The sar-16 field loads confirm the §7j.34 grammar exactly — {+0 state, +2 x0,
+4 y0, +6 w, +8 h, +0xA variant}. **NO bounds checks anywhere** (same trust as
the §7j.21 door stepper): a malformed rect writes out of the tile range (and
potentially past the 10000-B bank); corpus data is well-formed (F below).

**D. The arena side (re-verified from the D149 substrate):** FUN_0041d954
resets the arena cursor [0x46af0c] := the post-boot watermark [0x46af20]
(0x41d955) then bump-allocates the per-mission chain; the claim bank is the
7th block (`mov eax,0x2710; call 0x41db89; mov ds:0x46af58,eax` @
0x41d9cd..0x41d9d7) — the SAME absolute span every mission (deterministic
order), arena memory NEVER zeroed by the allocator. The cross-mission
staleness this would imply is MOOT for this bank: the initializer's memset-0
(C) runs every mission load before any reader. (Contrast: the `.MIN` bank
D149 — no memset anywhere, stale tail proven unreachable.)

**E. The rect source — FUN_0042c4a0 = a per-zone HARDCODED store farm**
(0x42c4a0..0x4330xx; zone dispatch `jmp [[0x4edd8c]-1)*4 + 0x42c484]`, 7
entries: zone1→0x42c4bc, 2→0x42c660, 3→0x42d805, 4→0x42f0fc, 5→0x430de0,
6→0x43181b, 7→0x433007; >7 → ret 0x426030). Each zone case gates on mode
([0x4edb88]==2 → skip) then the within-zone mission number [0x4edd88] and
writes a subset of the 45 rect records as IMMEDIATE constants
(`mov eRg,imm; mov word ds:bank+off,Rg`). Because the whole bank was
memset-0 at 0x447b7b, every record the case does NOT write stays inactive —
the stamped set is fully deterministic per (zone, mission, mode), and
cross-mission staleness in the RECT bank is moot too. **Zone 1 (ZONEA), mode
≠ Head2Head, mission 1 — the S0 corpus mission — fills exactly records
0..6** (verified instruction-by-instruction + a scripted register tracker,
scratch /tmp/opencode/rect_extract.py):

| rec | state | x0 | y0 | w | h | variant |
|-----|-------|----|----|---|---|---------|
| 0 | 1 | 2 | 51 | 9 | 2 | 1 |
| 1 | 2 | 9 | 44 | 3 | 2 | 1 |
| 2 | 2 | 16 | 35 | 2 | 5 | 1 |
| 3 | 1 | 18 | 35 | 2 | 5 | 1 |
| 4 | 1 | 4 | 32 | 1 | 3 | 5 |
| 5 | 2 | 2 | 10 | 2 | 2 | 1 |
| 6 | 1 | 16 | 11 | 4 | 2 | 2 |

records 7..44 stay 0 → the stamper stops after record 6. All tiles land
inside the ZONEA 25×75 map (line[y] = y·25; max x 19 < 25, max y 52 < 75);
the rects are pairwise disjoint → **exactly 59 claimed tiles** for ZONEA/M1.
Zone-1 missions ≠1 write NOTHING (the case gates mission==1) → the rect bank
stays all-zero → the claim bank stays all-zero after the memset.

**F. Engine/differ consequence (the S0-11 gap, queued):** bedlam-core models
the claim byte as a hardcoded 0 ("host-staged zeros — the D82 order-marker
writers are the unmodeled seam", destroy.rs stage_splash/platform_tile_build)
— **both halves of that comment are now disproven**: the bank is NOT zero at
mission load (the door-rect stamp runs before the first frame), and the
writers are not order-marker/D82-seam writers (they are deterministic,
input-free, hardcoded-data mission-load staging). The fresh-session
static-after-load image for ZONEA/M1 = 59 bytes of 1 at fixed tile indices
(59 disjoint tiles, list in the oracle test) in an otherwise zero 10000-B
bank. The three modeled readers gate on claim==0 → on a claimed tile the
original REFUSES the splash/platform-tile/death-blast where Rust ALLOWS it;
the canonical E emission and the O1/O2 dumps would diverge on this row for
every mission whose (zone,mission) has a rect case. The concrete gap:
stage the claim bank in Rust from the pinned rect tables (data: this
section E + the zone-2..7 census in the oracle test), read it in the three
modeled gates, emit it in the canonical TS row — the S0-12..S0-17 slices
stay untouched. No fabricated parity: the rect constants are file-free
hardcode, so the seam is fully deterministic.

**F-bis. SEAM LANDED (S0-11b, D151, 2026-08-25):** the gap above is
CLOSED. `bedlam-core/src/claim_rects.rs` = the promoted rect farm
(byte-identical to the oracle's transcription copy, pinned by test);
`MissionSim::stage_claim_bank(zone_set, mission)` = the §7j.63/C
initializer transcription, called at EVERY `GameHost::load_mission`
(the original's unconditional 0x447b85 call — no scenario key);
`stage_splash` + `platform_tile_build` read the byte in the gate
order above; the canonical `static-claim-bank` TS row emits the raw
arena image (DESIGN §6a). The "three modeled readers" phrase in F is
corrected: the death-blast smoke producer (FUN_0042382c) is
host-seamed presentation (§7j.24) — no sim gate exists for it.
Corpus reachability answered NO (tested): no staged S0..S8 scenario
stages on a claimed tile; the canonical chains moved via the TS row
only. Oracle parity closed both sides (37/37 missions).

The all-37-mission independent oracle (test/core, S0-11) transcribes the
rect store farm for zones 1..7 from the objdump (register-tracked, each zone
case's gates hand-read), rebuilds line[] per mission TOT header, computes
the expected claim image per mission, and pins the corpus identities
(per-mission claimed-tile counts, the ZONEA/M1 59-tile set, the
all-zero missions).

## 7j.64. THE EIGHT FRESH-SESSION T0 CAMPAIGN/CONFIG SCALARS — the whole-cell boot/derive chain decoded (score 0x4dd40c, money 0x46ae70, difficulty 0x46cbf8, zone 0x4edd8c, mission 0x4edd88, mode 0x4edb88, linear-mission-m 0x46ae8c, sfx-master-gate 0x4ede58) (2026-08-25, worker 0f91b0d7 claim 1, D153, the P4/static-parity/S0-12 unit; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; read-only string probes of BEDLAM.EXW (VA 0x454000 = file 0x52600, the D135 anchor); MANIFEST.sha256 clean before AND after) [verified]

Method: whole-objdump write-form censuses per cell (`mov [dword] ds:CELL,
reg/imm` forms only — every other xref is a read), then instruction-level
decode of each writer block. "Fresh session" = boot → title → NEW campaign
(no save slot) → name entry accepted without touching the difficulty toggle
→ mission 1 load — the S0 capture shape.

**A. The GameMain boot-init head (0x41c05c..0x41c176) — the mode +
difficulty fresh writes.** `xor ebx,ebx` @0x41c0b7 … `mov ebx,0x1`
@0x41c12e, `xor eax,eax` @0x41c138, then the store fan:
`[0x4dc6cc]:=eax`, **`[0x4edb88]:=eax` (MODE := 0)** @0x41c145,
**`[0x46cbf8]:=ebx` (DIFFICULTY := 1)** @0x41c14a, `[0x4edbec]:=eax`,
`[0x4dc6c8]:=eax`, `[0x4dc6c4]:=eax`, `[0x4ede5c]:=eax`,
`[0x4e44c4]:=edx(−1)`, `[0x4edbdc]:=ebx(1)`, `[0x4edb6c]:=ebx(1)`,
`[0x4edbf0]:=ebx(1)`. **The §7j.15/2 "campaign-start write 0x41c14a"
gloss CORRECTED: it is the BOOT default, and the fresh value is 1, not 0**
(ebx re-set to 1 six instructions before the store; no intervening def).
The name-entry toggle (0x43ab7e `(d+1)%3` → 0x43ab85/0x43ab92) and the
campaign save-load (0x43c3a6) are the only other difficulty writers (6
write sites total; the zone-7 force/restore pair 0x41c578/0x41c58d is a
call-scoped temp, §7j.15/2 — it wraps FUN_0044771c and restores EBP).

**B. The episode-loop slot boot (0x41c41c..0x41c44e) — zone + mission +
score fresh writes.** `edx:=1` @0x41c41c, `eax:=0`, `ecx:=0`:
**`[0x4edd8c]:=edx` (ZONE := 1)** @0x41c42a, **`[0x4edd88]:=edx`
(MISSION := 1)** @0x41c430; then `call FUN_0043a5fc` (the title/name-entry
state machine — returns 0 on the fresh-new-campaign path) and
`[0x46ae78]:=ecx`, `[0x46ae74]:=ecx`, **`[0x4dd40c]:=ecx` (SCORE := 0)**
@0x41c44e. The §7f.9 "GameMain boot 0 (0x41c44e)" gloss stands, refined:
the 0 is FUN_0043a5fc's fresh-path return.

**C. The name-entry fresh-campaign arm (0x43aaa3..0x43aad0) — money +
mode re-writes.** A jump-table arm of the name-entry state machine
(`jmp [[0x46ae7c]−1]*4 + 0x43a5e8`): `eax := [0x46cbf8]·0x1f4` (500·d),
`edx := 0xfa0` (4000), `esi := 0`, `edi := 1`,
**`[0x4edb88]:=esi` (MODE := 0 again)** @0x43aab9, `[0x46cbe0]:=edi`,
`edx := 4000 − 500·d`, **`[0x46ae70]:=edx` (MONEY := 4000−500·d)**
@0x43aaca. This is the §7d.4 "title start 4000−500·diff" site, now
address-pinned. Combined with A: **fresh money = 4000−500·1 = 3500 on an
untouched-toggle fresh boot** (d cycled at name entry re-seeds the same
formula; mode-2 variant 0x5DC is MP-only, excluded from S0's SP shape).

**D. linear-mission-m 0x46ae8c is a DERIVED cell, not a progress
counter (0x41c520..0x41c556).** Exactly 3 write sites, ALL in the
GameMain episode loop right after the SHOP return:
```
eax := [0x4edd8c] (zone) − 2
eax := eax·5 + [0x4edd88] (mission) − 1        ; lea eax,[eax+eax*4]; dec
[0x46ae8c] := eax                                ; 0x41c534
if eax >= 0x1b: [0x46ae8c] := 0x1a (26)          ; cap  @0x41c53e
if 1 > [0x46ae8c]: [0x46ae8c] := 1               ; floor @0x41c550
```
i.e. **m = clamp(5·(zone−2) + mission − 1, floor 1, cap 26)**, recomputed
from the CURRENT slot every episode — never persisted, never a counter
(the other 11 xrefs are readers: the TRT hp formula 250+250·m/27
(§7j.15/4-e), the pod-stagger 2000−m·1000/27 (§7j.20), FUN_00444d07).
Zone/mission pairs: zone 2 m1..m7 → m 1..6; zone 3 → 5..11; … zone 7
m1/m2 → 25/26. **Fresh (zone 1, mission 1): 5·(−1)+1−1 = −5 → floor →
m = 1.** This corrects the E-side model assumption (canonical.rs reads
`episode().linear()`, the 0-based campaign-progress counter, D108's
"never fabricated" seam): the guest cell is 1 at the S0 anchor while E
emits 0 — a live-capture divergence on this row, exactly the class
S0-12 exists to surface.

**E. sfx-master-gate fresh value = 1 (the loader default), pinned.**
FUN_004252c0 @0x4252f0..0x42530f: `[0x4ddb2c]:=0x4b` (volume default 75),
`call FUN_00444ed40` (the HKCU\Software\Mirage\Bedlam "DATA" probe —
pushes/pops edx, edx UNTOUCHED = the `1` from 0x4252c9), `push edx(1)`
(max), `ebx:=4` (REG_DWORD), `ecx:=edx(1)` (DEFAULT), `push 0` (min),
`eax:="SOUND"`, `edx:=0x4ede58`, `call FUN_0044ede4` — the D128 bounded
loader: absent/malformed/out-of-bounds ⇒ the DEFAULT (ecx) is written to
the cell (and re-saved). Same shape as SPEECH/CINEMATICS/ACTIONPAN
(bounds [0,1], default 1). **So the D134/D136 classification is
value-exact for the default machine: guest fresh = 1 = E's constant 1**
(a sound-DISABLED capture machine dumps 0 — the D136 loud finding,
unchanged).

**F. Whole-cell writer censuses (write forms only, EXW; EXD twins per
RE-EXD-MAP §4/§7):**

| cell | writes | families |
|---|---|---|
| score 0x4dd40c | 10 | GameMain slot boot 0x41c44e (fresh 0); campaign-restart restore 0x41c5e2 (from the pre-SHOP mirror [0x4eb934], stored 0x41c4fa); save-load 0x43c388 + the 6-slot family 0x4188xx..0x418fxx; the FUN_00444ca2 tail (0x444ca2 — the debrief/payout fold, §7f.9) |
| money 0x46ae70 | 13 | name-entry fresh-campaign 0x43aaca (4000−500·d); campaign-restart 0x41c5ec (mirror [0x4eb938]); save-load 0x43c0dd/0x43c117/0x43c395; SHOP floor 0x4411a9 (min 100, §7j.45) + buy/sell 0x4414b2/0x4414ec; the pickups family 0x40f0a1/c0/de/100 |
| difficulty 0x46cbf8 | 6 | boot 0x41c14a (fresh 1); name-entry cycle 0x43ab85/0x43ab92 ((d+1)%3); save-load 0x43c3a6; zone-7 force/restore 0x41c578/0x41c58d (temp) |
| zone 0x4edd8c | 8 | slot boot 0x41c42a (fresh 1); campaign-advance 0x41c9e5; save-load 0x43c2b8; mission-select 0x43edcb/0x43ede8/0x43ee18/0x43ee3d (the §7j.1 mission-number→set map); MP lobby 0x43f34b |
| mission 0x4edd88 | 6 | slot boot 0x41c430 (fresh 1); per-episode advance 0x41c4b9; mission-select 0x43ea9b/0x43ee5a/0x43eedc; MP lobby 0x43f360 |
| mode 0x4edb88 | 6 | boot 0x41c145 (fresh 0); name-entry start 0x43aab9 (0 again); save/load 0x43c0e2/0x43c11d/0x43c54a; MP lobby 0x43f373 (2) |
| linear-mission-m 0x46ae8c | 3 | the D derivation trio 0x41c534/0x41c53e/0x41c550 |
| sfx-master-gate 0x4ede58 | 2 | FUN_0043a144 set/clear 0x43a198/0x43a1b1 (D134 — registry-driven, fresh 1) |

**G. The fresh-session T0 table + E verdicts (the S0-12 coverage):**

| row | original fresh | E canonical fresh | verdict |
|---|---|---|---|
| score | 0 | 0 | CLOSED both sides |
| money | 3500 (4000−500·1) | 4000 (d=0 default; the boot-key seam `start_score(d)` is formula-exact) | GAP: fresh default d=0 vs boot 1 — S0-12b |
| difficulty | 1 (boot) | 0 | GAP: fresh default — S0-12b |
| zone | 1 | 0 (0-based stage; O1 normalizer maps cell−1, D99/D108) | CLOSED both sides (the pinned normalization) |
| mission | 1 | 1 | CLOSED both sides |
| mode | 0 | 0 (hardcoded SP) | CLOSED both sides |
| linear-mission-m | 1 (floor of the D formula) | 0 (episode progress counter) | GAP: derived cell vs counter — S0-12b |
| sfx-master-gate | 1 (registry default) | 1 (D136 constant) | CLOSED under the D134/D136/D144 machine-config seam |

The three gaps are one queued seam unit (S0-12b): pin the canonical fresh
session to the boot-block defaults (difficulty 1 → money 3500 via the
existing start_score seam) and emit the linear row through the D
derivation — a deliberate full-chain re-baseline (the D136/D151
machinery), never a silent default change. No row is silently counted:
the gaps stay loud until the seam lands.

**LANDED 2026-08-25 (S0-12b/D154):** the seam closed all three gaps BOTH
sides — the canonical fresh default is difficulty 1 (the boot write
re-created; `boot difficulty=d` overrides), the campaign seed runs on
every run (money 3500 + the sim damage rows at the original's fresh d=1),
and the emitted row + the TRT hp tier selector read the derived cell (the
§7j.64/D formula from the CURRENT mission_slot()). The oracle's three
loud gap assertions flipped to equality pins; every canonical chain
re-baselined deliberately (see D154 for the digest table).

## 7j.65. THE RNG PAIR + THE DITHER-NOISE BANK — initialization/evolution decoded whole (rng-state-a 0x4ede48, rng-state-b 0x4ede4c, static-dither-noise 0x4e6ed8 + cursor 0x4ddb30) (2026-08-25, worker 77b1c512 claim 1, D155, the P4/static-parity/S0-13 unit; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; MANIFEST.sha256 clean before AND after) [verified]

Whole-objdump write-form census per cell (every `0x4ede48/0x4ede4a/
0x4ede4c/0x4ede4e`, `0x4ddb30`, `0x4e6ed8` displacement) + instruction
decodes of both step functions and all six producer loops. All items
[verified] against the asm.

**A. The step functions — RandA @0x402975 and RandB @0x4029b6 are the
SAME 0x41-byte algorithm on two independent dword states** (A halves
0x4ede48/0x4ede4a, B halves 0x4ede4c/0x4ede4e — lo stored at the cell,
hi at cell+2):

```
402975  movzx eax, WORD [cell+2]     ; ax = hi(S)
40297c  movzx ebx, WORD [cell]       ; bx = lo(S)
402983  mov   esi, eax               ; si = hi(S)   (saved)
402985  mov   edi, ebx               ; di = lo(S)   (saved)
402987  mov   dl, ah                 ; -- byte shuffle: the 40-bit
402989  mov   ah, al                 ;    chain dl:ax:bx := S << 8
40298b  mov   al, bh                 ;    dl=hi15..8, ax=hi7..0|lo15..8,
40298d  mov   bh, bl                 ;    bx=(lo7..0)<<8
40298f  xor   bl, bl                 ; bl=0 (and CF := 0)
402991  rcr   dl, 1                  ; -- 40-bit rotate-right-1 through
402993  rcr   ax, 1                  ;    CF across dl:ax:bx
402996  rcr   bx, 1
402999  add   bx, di                 ; + lo(S)
40299c  adc   ax, si                 ; + hi(S) + carry
40299f  add   bx, 0x62e9             ; + 0x62E9
4029a4  adc   ax, 0x3619             ; + 0x3619 + carry
4029a8  mov   WORD [cell], bx        ; new lo
4029af  mov   WORD [cell+2], ax      ; new hi
4029b5  ret                          ; eax = new hi (u16)
```

Closed form: the shuffle puts S in the top 32 bits of the 40-bit chain
(dl:ax:bx = S<<8, low 8 bits zero); the three rcrs rotate the chain
right exactly 1 (incoming CF = 0 from the xor; outgoing bit 0 = 0), so
the chain holds **S<<7** and its low 32 bits — everything the add chain
reads — are `(S<<7) & 0xFFFFFFFF`, while the top byte dl' = S>>25 is
**DISCARDED** (never read again). With the 16-bit add/adc pairs
performing plain 32-bit addition:

  **S' = ((S << 7) + S + 0x361962E9) mod 2^32**

— a SHIFT-7, not a wrap rotate: the 8street gloss "ror33ish" and the
RE-EXW-MUSIC "carry-mixed" prose are retired by this decode (their
carry-chain description matches the instruction sequence but the
wrapped bits go nowhere; per the 8street policy this is re-anchored to
the EXW instructions above). **Return value: eax = the NEW HIGH WORD**
(u16 — movzx zeroed eax's top at entry and every later op is 16-bit);
consumers mask the return (`test al,3`, `and eax,0x1ff`), i.e. they
read bits of the new hi word. First values, both seeds: A 123456 →
923,559,209 (0x370C6529) → 4,082,654,354 (0xF3585C92) → …; B 234567 →
937,892,528 (0x37E71AB0) → 1,636,685,209 (0x618DD599) → … (the S0-13
oracle pins the first eight states of each chain as literals).

**B. The seed plants — the COMPLETE writer census** (every writer of
the two dword cells in .text; nothing but these six instructions):

| site | form | semantics |
|---|---|---|
| 0x41c0cd `mov [0x4ede4c],edi` + 0x41c0d3 `mov [0x4ede48],eax` | GameMain boot block (edi=0x39447 set 0x41c0a8, eax=0x1e240 set 0x41c0ad) | BOOT plants BOTH: A := 123456, B := 234567 (the block directly precedes the §7j.64/A boot head) |
| 0x447728 `mov DWORD [0x4ede48],0x1e240` | MissionShell FUN_0044771c — the FIRST body instruction after the 0x658-byte stack frame | PER-MISSION reseed of **A ONLY** — B is carried across missions within a session (never reseeded post-boot) |
| 0x4029a8/0x4029af, 0x4029e9/0x4029f0 | the step functions' own lo/hi stores | the evolution |

**C. The dither-noise bank 0x4e6ed8 (0x800 B) + cursor 0x4ddb30 — the
COMPLETE writer census** (every displacement site; the blit only READS):

- cursor: staging clear **0x4478f7** `mov [0x4ddb30],ecx` — ecx = 0
  (xor'd at 0x44785e, the ~40-resets staging block, SIM §1) → **cursor
  := 0 at every MissionShell entry**; churn advance stores 0x448164 /
  wrap-store 0x448178. No other writer or reader.
- bank: the fill's two store arms 0x447b17 (0xFF) / 0x447b32 (0x00) and
  the churn's 0x448150 (0xFF) / 0x44818d (0x00). Nothing else writes.
- **boot fill 0x447b13..0x447b3a** (decoded): `xor ebp,ebp`; loop for
  i in 0..0x800: `call RandB` (0x447b27); `test al,3`; ==0 →
  `bank[i] = 0xFF` else `bank[i] = 0`; `inc ebp; cmp 0x800; jge exit`.
  **Exactly 2048 draws — one per byte — and the cursor is untouched.**
- **per-frame churn 0x448147..0x448195** (decoded): loop k in 0..15:
  `ecx = cursor; inc ecx; cursor = ecx; if (ecx >= 0x800 || ecx < 0
  signed) cursor = 0` (store-then-normalize); `call RandB` (0x44817d);
  `test al,3`; `bank[cursor] = 0xFF/0x00` (both arms RE-READ the
  normalized cursor). **15 draws/frame, advance-then-draw-then-write.**
  Full-ring refresh period ceil(0x800/15) = 137 frames. The `< 0`
  signed arm is dead defensive code (cursor only ever 0..0x7FF).
- **the blit (§7i/1) re-verified unchanged**: reads only — per row,
  `src_off + 2·width − 0x800 ≥ 0` → `src_off = RandB() & 0x1ff`
  (0x401b22..0x401b39 / the mode-0 twin 0x401b90..0x401ba7), i.e. the
  reseed draw is a READ-OFFSET pick, never a bank write.

**D. The call census** (direct `e8` rel32 only): **158 RandA sites,
27 RandB sites** in .text. The dither family's RandB draws: the fill
(0x447b27), the churn (0x44817d), the per-blit seeds
(FUN_0041ec59@0x41ec61 — the §7i/3 `(RandB()&0x7fff)/15` clamp ≤
0x7f5, called from the FUN_004072bf portrait pass with eax=0x7f6 at
0x4072fb/0x4073bb/…), the intra-row reseeds (0x401b26, 0x401b94). The
per-frame interleave order stays §7i/4: terrain edges → dither
seeds/reseeds → churn; the fill's 2048 draws precede frame 0.

**E. The three rows' S0-13 verdicts** (original-side closure; the E
side is the charter T3 statistical stand-in — D155):

| row | original init | original evolution | E side |
|---|---|---|---|
| rng-state-a | boot 123456 (0x41c0d3) + per-mission reseed 123456 (0x447728) | S' = ((S<<7)+S+0x361962E9) mod 2^32 per RandA draw | PCG32 stand-in, draw-count-compared only (AcceptedT3) — never bit-compared |
| rng-state-b | boot 234567 (0x41c0cd), never reseeded | same step function | same stand-in class |
| static-dither-noise | cursor 0 + 2048 RandB draws at MissionShell staging (§C) | 15 RandB draws/frame at the epilogue; bank ∈ {0x00,0xFF}, `RandB()&3==0 → 0xFF` | presentation-half (D17): the bank never enters the dump/hash; the row is O1-side coverage |

## 7j.66. THE S0 DUMP-TRIGGER ORDERING + THE g_frame_count WRITER CENSUS (the s0-trigger / frame-counter registry rows) — the MissionShell tail decoded, the PresentEnd call-site ambiguity RESOLVED, and the D81 "no counter reset" claim CORRECTED: the eight bounded cinematic screens RESET the counter (2026-08-25, worker 9c711d0c claim 1, D156, the P4/static-parity/S0-14 unit; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; MANIFEST.sha256 clean before AND after) [verified]

Whole-objdump census of every 0x46ae68 .text reference (53 hits; 14
increment forms + 8 zero-writes + reads) + instruction decode of the
MissionShell loop tail. All items [verified] against the asm.

**A. THE MISSION-LOOP TAIL (the EXW twin of the EXD S0 dump site
0x5a6eb — the §1 pseudocode "PresentEnd(); g_frame_count++" decoded
instruction-for-instruction):**

```
4485de  cmp  [0x4edc34], 0        ; P-pause latch
4485e5  je   0x4486c9             ; not paused → NORMAL present path
4485eb  mov  ebp, [0x4edb88]      ; network-session flag
4485f3  jne  0x4486c9             ; MP never pauses → NORMAL path
        ; --- SP pause path (scenarios never take it: P 0x19 banned):
4485f9  [0x4edb64] := 0 ; [0x4edc34] := 0
        ; draw the PAUSED overlay (text 0x46ba6c via FUN_0043cc3e)
44861f  call 0x425a03             ; PresentEnd (the PAUSE-screen flip)
448624  FUN_0041e215(0x19)        ; consume the P keypress
44862e  spin while [0x4edc34]==0  ; the P-pause spin
448637  FUN_0041e215(0x19, 0)     ; consume the unpause press
        ; clear the 0x4edb50 → 0x4edc0c latch family ; [0x4edb64] := 1
4486c7  jmp  0x4486ce             ; pause already presented — skip
4486c9  call 0x425a03             ; PresentEnd — THE MISSION-LOOP FLIP
4486ce  mov  ebp, [0x46ae68]      ; --- counter increment, register form
4486d4  inc  ebp                  ;    (the unrelated [0x4dc67c] read
4486da  mov  [0x46ae68], ebp      ;     at 0x4486d5 rides the middle)
```

Exactly ONE PresentEnd + exactly ONE counter increment per loop pass
(the pause path presents at 0x44861f then jumps past 0x4486c9). The
counter increment ALWAYS FOLLOWS the present, both paths.

**B. THE S0-TRIGGER RESOLUTION (the O2/W11 deferral closed).**
PresentEnd = FUN_00425a03 has **62 direct call sites** in .text
(menus, loading screens, cinematics, the pause redraw) — a code BP at
the FUNCTION ENTRY 0x425a03 fires on every present on the way to the
mission, so it is NOT the frame-tail dump trigger. The frame-tail dump
point is the **NORMAL-PATH CALL SITE 0x4486c9** (BP before the call
executes = after the last state writer, before the flip) — the exact
twin of EXD 0x5a6eb (CALL FUN_00010670) with its own register-form
increment at 0x5a6f0-0x5a6fd right after (RE-EXD-MAP §2 — order
IDENTICAL, verified). The registry s0-trigger row keeps exw_addr
0x425a03 as the function canon (plan-neutral; the committed O2 plan's
trigger.site 0x00425A03 regenerates to 0x004486C9 at the W11 plan
regen — recorded in the row note + D156).

**C. THE WRITER CENSUS (D81 CORRECTED).** The D81-era claim "NO
counter reset exists (14 INC sites incl. menu screens)" is WRONG on
the reset half: an INC-form census misses the register/mov stores
(the exact trap the W1 EXD census documented for the mission tail —
RE-EXD-MAP §2 "why the INC-only census first missed it"). The 14
increment sites stand (13 INC + the mission-tail register form), but
there are ALSO **8 zero-writes**: the eight BOUNDED CINEMATIC SCREEN
loops reset the counter to 0 at entry and use it as their
screen-duration timer, exiting with counter == bound:

| reset site | bound loop | duration | INC site |
|---|---|---|---|
| 0x44466f (`xor ebx` @0x444668; FUN_0041cbf0 rides between xor and store) | cmp 0xc8 @0x444675 | 200 | 0x44469b |
| 0x4446e4 (`xor esi`) | cmp 0x64 @0x4446ea | 100 | 0x44470d |
| 0x4449f9 (`xor ecx`) | cmp 0x12c @0x4449ff | 300 | 0x444a3a |
| 0x444c4b (`xor ecx`) | cmp 0xc8 @0x444c51 | 200 | 0x444c77 |
| 0x444f87 (`xor edx`) | cmp 0x64 @0x444f8d | 100 | 0x444fb0 |
| 0x445167 (`xor edx`) | cmp 0x64 @0x44516d | 100 | 0x445190 |
| 0x44526c (`xor ebx`) | cmp 0x12c @0x445278 | 300 | 0x4452a2 |
| 0x4453b7 (`xor esi`) | cmp 0xc8 @0x4453bd | 200 | 0x4453e7 |

(the loop bodies draw, `call 0x425a03`, then `inc` — present-then-inc,
same order as the mission tail). The other five INC sites are the
INTERACTIVE menu screens — NO reset, cumulative counting, and there
the in-loop order is inc-THEN-present: 0x43afa0, 0x43d4f7, 0x43d53f,
0x43da5a, 0x43f31f. The counter is a REUSED global: cinematic screens
hijack it as a duration timer, menu screens count frames on it, and
only the mission-loop tail (0x4486ce-da) uses it as the frame pacer.

**D. THE DUMP-POINT VALUE SEMANTICS (the frame-counter row).**
MissionShell's HEAD (loading-screen presents @0x4476b3/0x4477fa/
0x447840 + the staging chain) contains NO counter writer — the
mission loop inherits whatever the last pre-mission screen left.
With no reset in the mission loop either, the dumped value at dump k
(1-based) = **C₀ + (k−1)** where C₀ = the counter at mission-loop
entry = a DETERMINISTIC FUNCTION OF THE SCRIPTED MENU WALK (fixed
for the S0W scripted walk; arbitrary only across different walks),
NOT a boot-frame total. At the dump BP the counter is PRE-increment
for the current pass (the increment is the tail's NEXT action). E's
canonical emission already matches: `frame-counter = sim.frame()−1`
(canonical.rs — mission-relative, pre-increment), so **O1/O2 value =
E value + C₀**, a per-script constant. The machinery already built on
exactly this: dump records align by schema frame_no (never the
counter watch), the differ classes the row `T2Reported`, and the
double-run verdict stays "identical chains modulo the T2/T3 cells"
(the counter is deterministic per script — the reset discovery makes
it MORE deterministic, never less). The D87 differ-doc phrasing "the
O1 counter never resets" is corrected in place to the precise form:
never resets IN THE MISSION LOOP; the menu-path resets are what make
C₀ walk-determined. EXD twin cross-check: **CLOSED (D167,
2026-08-26, RE-EXD-MAP §2b)** — the EXD whole-text objdump census of
the counter twin [0x1195f0] returns the EXACT EXW form split (53
references; 13 INC + 1 register form + 8 zero-writes + 31 reads; the
bound sequence 200/100/300/200/100/100/300/200 identical; all eight
resets inside the DEBRIEF twin 0x5638d, the FUN_0044425c twin called
from GameMain @0x2cf3f; cinematic present-then-inc vs menu
inc-then-present orders exact). The twin census holds ordinally,
instruction-form-exact — the C₀ model carries to EXD verbatim.

**E. THE S0-14 CLASSIFICATION (D156).** The two rows are
DYNAMIC-ONLY — they carry no statically-closeable state: s0-trigger
is the dump POINT itself (extent 0, a breakpoint — its "coverage" IS
this ordering pin + the capture machinery arming it), and
frame-counter is the T2 timing cell (deliberately never
bit-compared). They close under the new **dynamic-only placement**
disposition, tracked separately from static closure: strict S0
accounting becomes 22 rows static-closed + 2 dynamic-only
dispositioned + 3 static remaining (S0-15 static-order-table, S0-16
static-player-type, S0-17 static-cursor-clamp) = 27.

## 7j.67. THE 0x62-STRIDE ORDER/WEAPON TABLE (0x4de664) — geometry pinned (12 rows × 0x62 = 0x498, BOTH ends anchored), the whole-writer/reader census closed with TWO NEW GameMain writer families the §7d.2 census missed, the §7d.2(c) "MP lobby writer" gloss corrected to READ direction, and the fresh-session static image proven all-zero (2026-08-25, worker af39f59b claim 1, D157, the P4/static-parity/S0-15 unit; objdump-only from ghidra-project/exw-text-objdump.txt + the tools/exd-relod.py linear image (rebuilt to /tmp/opencode scratch); read-only byte probes of BEDLAM.EXW/BEDLAM.EXD; no Ghidra run; MANIFEST.sha256 clean before AND after) [verified]

Whole-objdump census of every displacement resolving into
[0x4de664, 0x4deafc) (the 0x4de6/0x4de7/0x4de8/0x4de9/0x4dea
families, 119 raw hits, boundary families excluded by address) +
instruction decode of every writer block. All items [verified]
against the asm.

**A. GEOMETRY — the extent is pinned from BOTH ends.** The table is
**12 rows × 0x62 = 0x498 bytes**, EXW 0x4de664..0x4deafb, EXD
0x91ee4..0x9237b. Proofs: (1) the GameMain boot zero-init immediate
`mov ecx,0x498; mov edi,0x4de664` @0x41c3d6..0x41c3db (EXD twin
`mov ecx,0x498; mov edi,0x91ee4` @0x2cd0f..0x2cd14) — the WHOLE-span
memset, and 0x498/0x62 = 12 exactly; (2) the successor structure:
EXW the 12-row chassis table at 0x4deafc (0x150 = 12×0x1C, its own
boot memset @0x41c3f9), EXD the chassis twin at 0x9240c (memset
@0x2cd32) — the EXW tables are ADJACENT (order table end == chassis
base), while the EXD layout carries a **0x90-B path-string buffer at
0x9237c** between them (read as a strcpy SOURCE by the config
initializer FUN_0004be7d @0x4be89, staged with the 0x867f4
"CONFIG.BDL" suffix; zero at rest in the linear image). The
12-row/type domain matches the 12-slot robot bank (D129) and the
12×0x1C chassis rows: **row index = chassis TYPE 0..11** (the MP
contexts equate it with the player ordinal — the §7j.45 per-player
mirror 0x4de664+p·0x62).

**B. WRITER CENSUS (EXW, 6 families — §7d.2's list had 3 + the
§7j.45 mirror; TWO GameMain families were missing):**

1. **Boot zero-init** [0x41c3d6..0x41c3e5]: `mov ecx,0x498; mov
   edi,0x4de664; call 0x43a48d; call 0x402965`. The intervening
   `call 0x43a48d` is a **single-`ret` no-op stub** (whole function
   = `c3`) — ecx/edi survive; the memset FUN_00402965 zeroes the
   whole table. The §7d.2 ".bss-zeroed at boot" gloss UPGRADED: the
   zeroing is an EXPLICIT GameMain boot instruction sequence, and
   the row count is pinned by its immediate. EXD twin note: the EXD
   intervening call 0x4c7a5 is NOT a bare stub — it copies two
   config cells ([0x107668]→[0x107698]) before the memset — a minor
   boot-order divergence, harmless to ecx/edi.
2. **Episode-reset memset** [0x41ca06..0x41ca29, called 0x41c5f1]:
   the GameMain episode-transition block that also stores
   [0x46ae74]/[0x4edb50] (new-episode state) re-memsets the whole
   table (0x498 @0x41ca0b..0x41ca15) + the chassis 0x150
   (@0x41ca1a..0x41ca24) — the fresh-episode loadout wipe. EXD twin
   0x2d2d6..0x2d2f4.
3. **Post-mission loadout RECAPTURE** [0x41ca2e..0x41cb33, called
   0x41c665/0x41c682/0x41c689] — NEW FAMILY, missed by §7d.2: after
   MissionShell returns, GameMain pools every robot's group-ammo
   word (robot +0x38+8j) into a per-(type,group-j) stack
   accumulator (walk bounded by [0x46ccbc], the robot count), then
   per player p < [0x46cbe0] and group j: `v = pooled[p·0x1c+j·4]`
   idiv **[0x46cbd8] (squad size)**; quotient ≠ 0 → word@+2 := q
   (ammo) and word@+0xA := FUN_0041cb38(ammo, group, player) (the
   catalog item from the +6/+8 price/category words and the
   0x4ea2ac/0x4ea2b0 tables); quotient == 0 → word@+0 := remainder
   (the `xor edx,eax` quirk @0x41cae0 — r ^ 0 = r). Writes
   0x41cae2/0x41cb0b/0x41cb24; reads 0x41cb51/0x41cb5b. EXD twin
   0x2d398/0x2d3a1/0x2d3b1/0x2d3bc (+0x2d422/0x2d42c reads).
4. **Save-load restore** [FUN_0044745e case 2, 0x43c3c3..0x43c42a]:
   the saved row copied word-for-word, 7 groups × 7 words (+0,+2,
   +4,+6,+8,+0xA,+0xC per group — the two last via the loop-carry
   displacements 0x4de660/0x4de662 after `add eax,0xe`; NOT a
   pre-base header). The boot call @0x41c417 runs the initializer;
   on a FRESH session (no SAVED.BDL) nothing restores. EXD twin
   0x4e583..0x4e5e9.
5. **Shop family** [FUN_00440e45]: buy full-group write
   0x4417f3..0x44183d, clear 0x441485..0x4414ab, ammo adjust
   0x4418da/0x4418f7, staging 0x441e1d..0x441e3f, sell-all 7-word
   clear 0x442821..0x442886 — the §7d.2(a)/§7j.45 census unchanged.
6. **Shop-exit MP mirror** [0x442b97/0x442ba7, in the §7j.45 exit
   0x442ae2..0x442c3e]: word@+0 := name, word@+2 := ammo from the
   0x4dd4a0+p·0x80 record (first byte skipped); the 0x442ba7
   displacement 0x4de658 is the +0xE loop-carry (eax ≥ 0xE always →
   target = group +2; the §7j.54 "alias, never the latch" ruling
   holds, now with the exact decode).

**C. READER CENSUS (EXW, 5 families; every one has a 1:1 EXD
twin — the two walks are ordinal-identical):**

1. **Spawn copy** [load_markers 0x40cefd/0x40cf18/0x40cf33 ⟷ EXD
   0x1dbc1/0x1dbdc/0x1dbf7]: the §6c.6 7-group copy into robot
   +0x36/+0x38/+0x3A + the default order-bits derivation (bits :=
   1 << first i with group word0 ≠ 0). The ADJACENT equipment-
   chassis consumption (the "0x2a/0x2b/0x2c extras switch"):
   EXW 0x40cf96..0x40d031 on base 0x4deafc ⟷ EXD 0x1dc66..0x1dcff
   on base 0x9240c — per chassis slot (2 × 0xE), a 5-case switch on
   the slot's name word ∈ 0x2A..0x2E: shield charges := signed slot word@+2
   (robot +0x8C), battery := signed word@+2 (+0x94), damper := signed
   word@+2 ×0xC8 (+0x98) — then these three cases clear the slot's
   +0/+2/+6 words (consumed). Scanner cases 0x2D/0x2E set the
   type-indexed 0x46ae94 bank to 1/2 and retain their rows; see
   RE-EXW-MISSION-ROOM.md equipment deployment boundary. This is chassis-family (out of the order-table
   window) but is the table's spawn-side sibling consumer; §7j.45/5
   gloss confirmed at both channels.
2. **MP respawn re-copy** [FUN_0040e230 0x40e97c/0x40e997 ⟷ EXD
   0x1f690/0x1f6ab]: the §7j.24 "weapon/equipment re-copy".
3. **Shop reads** [0x4402cd..0x443913 ⟷ EXD 0x52464..0x5599b]: the
   row-text feeder (0x4403d3, dword@[eax+0x4de662]>>16 = group word
   0 via the −2 carry, feeding FUN_00420260), the auto-loadout
   search (0x443823/0x443859 — eax = 62t computed as
   (t<<2−t)<<4+t), buy/sell/clear guards.
4. **SAVED.BDL writer** [FUN_0044693a (function identified by the
   0x4597d1 "SAVED.BDL" string) 0x446ce1..0x446d81 ⟷ EXD
   0x58bef..0x58c73]: stages mode/score/money words then the FULL
   row (7 groups × 7 words, `imul edx,[0x4edb90],0x62` + group
   walk) then the chassis row — the byte-exact inverse of the
   restore (B4).
5. **MP lobby exchange** [FUN_00448ef1 0x449f94/0x449fbd ⟷ EXD
   0x5b3fc/0x5b425]: READS word@+0 and word@+2 of the player's row
   into the 0x4eba04-cursor record build (the outgoing network
   staging). **§7d.2(c) CORRECTED**: FUN_00448ef1 NEVER writes the
   table — its "5 writer sites" stage the 0x4dd4a0 buffer; the
   table's only incoming MP mutation is the shop-exit mirror (B6).

Boundary exclusions (address-adjacent, different families): the
chase-camera override cells 0x4de648..0x4de654 (§7j.54), the salvo
cooldown latch 0x4de658 (§7j.54), the chassis table 0x4deafc+
(§7j.45). The pre-base displacements 0x4de660/0x4de662 and the
−2/−4 carry reads are phantom (always in-window after the register
arithmetic: eax ≥ 2 / ≥ 0xE on every path).

**D. THE FRESH-SESSION STATIC IMAGE = ALL-ZERO 0x498 (the TS row's
O1/O2 anchor content).** Derivation: boot memset (B1) → the
save-load initializer runs but restores nothing on a fresh session
(no SAVED.BDL) → the SP episode walk visits the shop before mission
1 (§7d.4) but a fresh campaign makes no purchases (money 3500,
zero input) and the shop mutates the table ONLY on buy/sell/auto
actions (§7d.2a) → **the MissionShell-entry image is 1176 zero
bytes**, deterministic. The SP player TYPE [0x4edb90] := 0 (§7d.3)
makes row 0 the live row; rows 1..11 stay zero for the whole SP
campaign (only the MP families touch them). Sim-side effect of the
all-zero image: the spawn copy plants zero group words and the
default order-bits derivation finds no nonzero word0 → bits 0 → no
order rows (§6c.6); no RNG, no hash surface. A NONZERO table WOULD
matter (bits = 1<<first-word0, copies land in the robot record) —
the falsification direction the oracle pins.

**E. THE S0-15 CLASSIFICATION (D157).** The row closes
ORIGINAL-SIDE only, the charter no-fabricated-parity class (the
D149/D155 precedent): the loadout is HOST-SESSION state whose
producers (shop, save-load, MP exchange) are outside the E engine —
E has NO loadout model, no shop screen, and the canonical
robot-bank record is the 94-B modeled subset that contains neither
the +0x36/+0x38/+0x3A group words nor the +0x6E order-bits word —
so an E emission would be fabricated parity. The row stays a
deliberate, loud E-gap (absent from every canonical frame, asserted
by the oracle), and the capture-plan extent hop LANDED (D158:
dbx-plan's deferred arm resolves to the pinned 0x498 fixed span at
a fixed address as Form::Fixed — the S0-12a precedent, Fixed here
since the table is direct .bss, not pointer-indirect; all 13
capture-plan artifacts regenerated, count asserts re-pinned).
watches.toml layout note amended plan-neutrally; the extent string
moved to "0x498 (12x0x62 rows)" by D158.

## 7j.68. THE PLAYER TYPE CELL (0x4edb90) — the fresh-SP value pinned 0 on BOTH channels, the whole writer census closed (EXW 6 = the boot writer + FIVE MP-lobby sites, FOUR of them the −1 error exit; EXD 2 = the boot twin + ONE MP serial-sync writer), the save family proven READ-ONLY (the type is never saved — the restore and the SAVED.BDL writer only INDEX by it), and the row closed BOTH SIDES through the canonical anchor seam (2026-08-25, worker 89591972 claim 1, D159, the P4/static-parity/S0-16 unit; objdump-only from ghidra-project/exw-text-objdump.txt + the tools/exd-relod.py linear image rebuilt to /tmp/opencode/s0-16 scratch; read-only byte probes of BEDLAM.EXW/BEDLAM.EXD; no Ghidra run; MANIFEST.sha256 clean before AND after) [verified]

Whole-objdump displacement census of 0x4edb90 (EXW, 113 literal
sites — the raw-dword scan of the whole BEDLAM.EXW file image finds
EXACTLY 113 occurrences of 0x004edb90, every one mapping via
BEGTEXT raw 0x400/va 0x1000 to a .text disp32 operand, DGROUP/
.idata/.reloc hold ZERO — no initialized pointer cell can alias;
zero occurrences of 0x004edb92 anywhere) and of 0x1075c0 (EXD, 117
.text sites; zero of 0x1075c2). All items [verified] against the
asm unless noted.

**A. THE CELL IS DWORD-WRITTEN, WORD-CONSUMED.** Every writer on
both channels is a 32-bit store; the plan extent "2" captures the
consumed word (the only WORD-sized reads are the two spawn-side
kind stamps: EXW 0x40cdec/0x44a2af, EXD 0x1db19 — see D). The
upper words 0x4edb92/0x1075c2 have NO independent reference in
either census; they are owned by the dword writers (nonzero only
transiently, on the MP error paths that store −1). The adjacent
cells are different rows: 0x4edb88 the MP respawn gate, 0x4edb8c
the MP endgame count (D133), EXD 0x1075bc the map-overlay cell.

**B. EXW WRITERS — exactly 6, and the §7d.3 gloss upgraded.**

1. **The GameMain boot writer** [0x41c344..0x41c34c]:
   `xor eax,eax; mov [0x4edb90],eax` — TYPE := 0, UNCONDITIONAL,
   immediately after the sound-init call `call 0x43a144`
   (FUN_0043a144, the D134 identification — the §7d.3 "bootattract
   decompile" gloss superseded) and INSIDE the CINEMATICS sandwich:
   [0x46cca4]:=1 before the call, restored @0x41c346, THEN the type
   store. The successor call 0x41c351 is the radio-warning poster
   FUN_004239ef family (§7j.53).
2. **MP lobby, FOUR −1 error exits** [FUN_00448ef1, the network
   object at 0x4ee490]: 0x44918a (esi=0xffffffff staged @0x449155,
   the [esp+0x7c] path after the 0x4edc45 debug gate), 0x4493e0
   (edi=[0x4ee8ac]==−1 — the unset local-slot case — same debug
   gate), 0x4497f1 (ebx=−1 after a FAILED vtable+0x8 network call,
   test eax,eax), 0x4498e6 (the literal −1 store after the
   vtable+0x10/+0x8 failure pair). All four `jmp 0x449a6d` (the
   function exit) — −1 = the "no local player" sentinel.
3. **MP lobby, the ONE success writer** [0x449a5c]: `mov
   [0x4edb90],eax` where eax = movsx(dx) of the loop index that
   walks the player-id array [0x4ee450] against the local id
   [0x4ee44c], bounded by the player count [0x46cbe0] — TYPE :=
   the local player's ORDINAL in the network player list (the
   domain the §7j.45 mirror equates with the 12-row table index).
**EXCLUDED from this slice by charter** (S0-16 task text): every
MP writer's value semantics — a later named task must cover the
lobby/sync families before any MP closure claim.

**C. EXD WRITERS — exactly 2 (the DOS port has no lobby family).**

1. **The boot twin** [0x2cc5f..0x2cc8a]: the SAME CINEMATICS
   sandwich with the cell pair [0x1194d8]≡[0x46cca4] := 1 around
   the config/sound init `call 0x4be7d` (FUN_0004be7d, the D134
   function twin), restore @0x2cc75, then `xor ebx,ebx` @0x2cc7b;
   `mov [0x1075c0],ebx` @0x2cc84 — TYPE := 0, and the successor
   call 0x2cc8a (FUN_00034895, the warning-post twin) preserves
   the instruction ORDINAL of the EXW tail exactly.
2. **The MP serial-sync writer** [0x5b026..0x5b030]: `call
   0x62100; and eax,0xffff; mov [0x1075c0],eax` — in the
   link-negotiation path bracketed by the strings "Quit from
   sychronising" (0x871a3 — the original's own typo, single
   occurrence in the file) and "Found %i players, but could only
   sync %i !" (0x871ba) — TYPE := the 16-bit result of the serial
   driver sync (the local player id). SP never enters this path
   (the cycler trio that consumes it is the EXD MP family at
   0x5b1cc+, D133).

**D. THE READER CENSUS (107 EXW reads / 115 EXD reads) — five
families, every EXW family with its 1:1 EXD twin.**

1. **The spawn-side kind stamp + first-robot cell** [EXW
   0x40cdec/0x40cdfb ⟷ EXD 0x1db19/0x1db28, instruction-exact]:
   `robot[i].kind@+0x2A := WORD[0x4edb90]` (the 0xA8-stride
   record, eax = 21·i scaled ×8) then `first-robot := [type] ·
   [robot count]` (EXW imul [0x46cbd8] → [0x46cbd4], the selected
   offset [0x46cbdc] := 0; EXD imul [0x11958c] → [0x11955c],
   [0x11954c] := 0 — the D132/D133 addition-order pairing, the
   cells swapped by channel as already recorded).
2. **The "my robot" gate** (the dominant family, ~all mission
   readers): `dword@robot+0x28 sar 16 == [type]` — EXW 0x408b9b,
   0x40a1ef (equal → the 0x41c9f0 episode-transition helper),
   0x40e2fd, 0x41034b, 0x4188a4/0x418940/0x418a75/0x418c73/
   0x418df6/0x418f98 (the name-entry/walk family), 0x423eb0 (the
   D132 chase-camera record gate), EXD twins 0x19910/0x1b2c2/
   0x1f02e/0x2106b/0x291bf-family/0x34e44 — the SAME comparison
   the Rust alarm/bounty/pickup gates model.
3. **The row-index family** (the §7j.67 table consumers): imul by
   0x62 (order rows) / 0x1C (chassis rows) across the shop
   0x4402c2..0x443923, the SAVED.BDL writer 0x4469cc..0x446e0a,
   the brief 0x43ead0..0x43f35a (cmp 0 gates), the restore
   0x43c37a..0x43c7ac (cmp −1/0), MissionShell 0x4475f5/0x4480d0,
   and the MP lobby/cycler 0x449cca..0x44a3af. **READ-ONLY on the
   save path [verified]**: neither FUN_0044745e (restore) nor
   FUN_0044693a (save) ever stores the cell — the type is NOT
   SAVED.BDL state; the save writes the row INDEXED by the current
   type (0x4469cc even derives the save name from it: lea
   eax,[eax+eax*8] into the 0x4e43e0 name table), the restore
   copies the row back into `0x4de664 + type·0x62` (§7j.67/B4).
4. **The per-TYPE sibling bank** [0x41ee2b]: `mov eax,[type]; cmp
   [eax*4+0x46ae94],1` — the variant-flag array 0x46ae94+type·4
   (the D133 "DIFFERENT array" warning — boundary exclusion).
5. **The remaining panel/walk reads** (0x40bb25 the robot-record
   walk, 0x40a476/0x40e2fd/0x40f077/0x40f189 the respawn/pickup
   family, 0x41170e) — all equality/indexing consumers, none
   store.

**E. THE FRESH-SP IMAGE = 0 (both channels, deterministic).** The
boot writer is the ONLY SP-path writer (unconditional, value 0);
the save path never restores the cell; the MP writers are gated on
network/lobby entry which SP never takes. So the TS row's captured
2 bytes are 00 00 for the whole SP campaign — and the value is a
genuine ENGINE constant on the E side too (below), not an
E-fabricated zero.

**F. THE RUST COMPARISON + THE S0-16 CLASSIFICATION (D159) —
the row closes BOTH SIDES through the canonical anchor seam (the
D154 precedent), NOT the D157 no-fabricated-parity class.** E
models the cell genuinely: `MissionSim::player_type: u16`
constructed 0 with NO setter (the whole census's writer set is
boot+MP, both outside the mission sim — the constant IS the
faithful SP model), and three REAL consumer gates read it: the
alarm trip (`alarm_ctr > 100 ∧ kind == player_type` → alarm :=
100, ctr := 0 — the §7g.1 transcription), the critter bounty gate
(`robots[attacker].kind == player_type` → score += 75/150 +
strip_arm, §7j.24/2), and the case-4 pickup seam (the host seam's
`type == [0x4edb90]` gate trivially 0==0 — documented at the
seam). The spawn-side consumers match too: Rust robots are
constructed `kind: 0` (the SP kind-stamp model) and `kind` IS in
`state_hash` (mission.rs) while `player_type` stays unhashed —
exactly the original's surface (the cell selects whose robot, the
robot record carries the state). The row's E half is therefore the
canonical anchor emission `static-player-type` = u16 LE of the sim
cell (00 00, anchor frame only, byte passthrough on every channel
— no differ change, the D136 static-map-wh precedent); all
canonical chains re-baselined deliberately. watches.toml layout
note corrected plan-neutrally (extent stays "2"); RE-EXD-MAP §5
row re-pinned with the D132-gloss refinement recorded and the MP
writer census.

## 7j.69. THE FOUR DEFERRED TS EXTENTS PINNED (cgr-volume, bin-terrain, lnk-map, yline-zbase) — both channels instruction-anchored, corpus cross-checked, and ONE stale gloss retired (§7c.2/MISSIONVIEW §1 "(0x8000)" for the LNK buffer has no immediate anywhere in the loader path) (2026-08-26, worker d093c3ef claim 1, D161, the P4.2/S0-registry-tail `ts-extent-arms` unit; objdump-only from ghidra-project/exw-text-objdump.txt + exd-text-objdump.txt — no Ghidra run; read-only corpus size census (.CGR/.BIN/.LNK/.LNG) with MANIFEST.sha256 clean before AND after) [verified]

The S0-15a/D158 follow-up: the last four `_deferred` dbx-plan arms
resolve. Every extent below is read off the verified load path (the
alloc/loader instructions, not a gloss) and cross-checked against the
shipped corpus.

**A. `static-cgr-volume` — the CGR height bank: extent = the uniform
132354-B file image (0x20562), NOT the 0x20788 arena.**
- Allocation [verified EXW 0x41d95f..0x41d969]: `mov eax,0x20788; call
  0x41db89; mov ds:0x4edd60,eax` — ArenaAlloc(133000), the FIRST alloc
  of the GameMain mission arena pass FUN_0041d954. EXD twin
  [0x2e288..0x2e292]: `mov eax,0x20788; call 0x2e4b2; mov
  ds:0x107540,eax` — ordinal-identical first alloc of the EXD arena
  pass (same pass shape: 0x20788 CGR, 0x13884 DAT, 0x3cc0 viewport...).
- Load [verified EXW 0x41dca0..0x41dcb7]: zone-scoped path2 0x4dca8c +
  the ".CGR" tag 0x4587e3 (tag-table entry 2), concat FUN_0041dbed,
  whole-file read FUN_0041cc7f into [0x4edd60] — VERBATIM (no header
  skip, no transform; the §7j.62/B MIN class). EXD twin
  [0x2e613..0x2e658]: same tag/path/read shape into [0x107540].
- Content [FORMATS §18, VERIFIED 44/44]: every shipped .CGR is exactly
  132354 B = u16 count 128 + the 512-B self-relative u32 directory +
  128 × 1030-B records (6-B `{0,32,32}` header + 1024 raw height-map
  bytes), the last record ending exactly at EOF. The get_z_pos reader
  (§7c.6 `CGR[2 + 4·(type−1) + dir[type−1] + 6 + ...]`) never leaves
  the file image: the 646-B arena tail (133000−132354) is stale and
  unread — the §7j.62/D stale-tail class.
- EXTENT DECISION: unlike `.MIN` (D149/D152, where the arena 0x7530
  was pinned BECAUSE the files vary 15824..29952 and no tighter pin
  exists), the CGR corpus is UNIFORM — the tightest pin is the file
  image itself, pinned as the constant 0x20562 (132354). This also
  keeps the row's cross-channel byte-passthrough compare CLEAN (a
  0x20788 extent would drag 646 stale arena bytes into every dump that
  differ between the DOS and Win32 heaps).

**B. `static-bin-terrain` — the BIN sprite bank: extent = the
0x258960 arena (the MIN precedent; no tighter pin exists).**
- Allocation [verified EXW 0x41d666..0x41d670]: `mov eax,0x258960;
  call 0x41db89; mov ds:0x4ede1c,eax` — ArenaAlloc(2460000). NOTE this
  is NOT in FUN_0041d954 (the mission arena pass): it rides the
  EARLIER boot alloc family (0x41d5xx..0x41d6xx), whose successor
  instruction 0x41d685 loads the GENERAL.BIN tag 0x45865a into the
  SIBLING bank [0x4edd7c] — the terrain BIN bank is boot-allocated,
  mission-loaded. EXD twin [0x2e098..0x2e0a2]: `mov eax,0x258960;
  call 0x2e4b2; mov ds:0x107434,eax`, successor tag 0x8610b into
  [0x1074fc] — the same boot-pass shape.
- Loads [verified]: the mission family loader ".BIN" tag 0x4587e8 →
  [0x4ede1c] @0x41dcbc..0x41dcd3, plus the second site FUN_0044661b
  (the EDITOR\ZONE restore reload) tag 0x45979a → [0x4ede1c]
  @0x44663f..0x446656. Whole-file verbatim; the header word
  u16[bank+0] is copied to the write-only cell 0x46cdb8 @0x41dd32
  (EXD twin 0x11a4a8 @0x2e6a0).
- Corpus cross-check [read-only, 44 .BIN sizes]: the 0x4ede1c
  candidates (7 zone MISSION{A..G}.BIN + the 2 mission-level BINs
  ZONED/MISSION5 + ZONEB/MISSION6) span 2041594..2443943 B — all
  inside 0x258960 (stale tails 16..418 KB by zone, never read: every
  content reader reaches sprites only through the self-relative
  directory, §7j.36). `GAMEGFX/SHOPLITE.BIN` (3081801 B) is a
  DIFFERENT bank family ([0x4edd7c] GENERAL.BIN-class), never loaded
  into 0x4ede1c — its >arena size is not a counterexample.
- EXTENT DECISION: the shipped sizes VARY and the byte length is not
  derivable from any outside cell (the count word lives INSIDE the
  bank at +0, behind the pointer) — exactly the D149/D152 `.MIN`
  situation, so the pin is the ARENA constant 0x258960 (2460000 B).

**C. `static-lnk-map` — the LNK/LNG link table: extent = 0x4000 (the
u16[8192] table), a DIRECT .bss span — and the "(0x8000)" gloss
retires.**
- Load [verified EXW 0x41dcf4..0x41dd18]: language gate
  `cmp [0x4eba1c],1` → ".LNG" (0x4587f2) else ".LNK" (0x4587f7), concat
  on path2, then `mov edx,0x45cdda; call 0x41cc7f` — the file lands
  DIRECTLY at the fixed .bss address 0x45cdda (never through a pointer
  cell; the registry row is correctly NOT indirect). EXD twin
  [0x2e65d..0x2e681]: gate on 0x10768c, tags 0x862c2/0x862c7,
  `mov edx,0x10336c; call 0x2d57c`.
- Readers [verified]: the territory-stamp lookup
  `cw = word@[type*2 + 0x45cdda]` (§7j.62/C, @0x408a8e family) and the
  terrain renderer's destructive chain-advance; EXD twins
  `word@[eax*2+0x10336c]` @0x177dc/0x178c3 plus the dword VIEW at
  base−2 (`dword@[edx*2+0x10336a]` @0x19809 — the §7d.1 "0x45cdd8
  table" twin, same image, shifted index base).
- Table [FORMATS §5, VERIFIED 44/44 + 7/7]: exactly 16384 B = 8192 ×
  u16, every shipped .LNK and .LNG (§7j.62/C already pins "16384 B =
  8192 words" at the loader).
- GLOSS CORRECTION: the "(0x8000)" buffer gloss in §7c.2 item 2 and
  RE-EXW-MISSIONVIEW §1 has NO immediate in the loader path (no alloc,
  no size argument anywhere near 0x41dd13/0x2e67c — the load is a
  whole-file read with no bound) and contradicts the verified uniform
  16384-B corpus; it retires. The extent pin is the TABLE: 0x4000 at
  0x45cdda (EXW) / 0x10336c (EXD), the Form::Fixed direct-span
  analogue of the S0-15a order-table resolution.

**D. `static-yline-zbase` — TWO non-contiguous tables, so the row
emits TWO spans (the registry id keeps the y-line table; the z-base
plane table rides the derived id `static-yline-zbase#zbase`).**
- Semantics already pinned by S0-08/D147 (§7c.3); this unit re-verified
  the EXD build loops first-hand [0x2e70b..0x2e74b]: y_line = h dwords
  at 0x8b78c (bound `ebx = h<<2` @0x2e70b, `jl` @0x2e727 — h entries,
  NOT h+1); z_base = exactly 8 dwords at 0x107718..0x107734 (loop eax
  = 4,8,…,0x20 @0x2e73d..0x2e748 — the store base 0x107714 is the
  adjacent pre-incremented screen-scale cell, never a table entry).
  EXW twins 0x41ddaa..0x41dde2 + the second producer
  0x4466bd..0x4466f8 (FUN_0044661b, §7c.3/D147).
- WHY TWO SPANS: the two tables are non-contiguous on BOTH channels and
  the inter-table gap DIFFERS per channel (EXW 0x4ea900 → 0x4eaacc,
  0x1cc apart; EXD 0x8b78c → 0x107718, ~0x7c000 apart) — no single
  span can mirror the layout across channels, and the differ's
  static-* rows are byte-passthrough (a one-span row would either dump
  half a megabyte of unrelated EXD memory or compare dirty). The row
  therefore resolves to: `static-yline-zbase` = the y-line table
  (CountExpr, len `4*$map_h` — the live h cell, the T1 grid precedent)
  and `static-yline-zbase#zbase` = the z-base plane table (Fixed 32).
  capgen's keep-first dedupe never drops either (distinct ids); both
  compare byte-exact cross-channel (the build loops are
  instruction twins and w/h agree per mission).
- EXTENT PINS: y-line = 4·h bytes (h dwords); z-base = 32 bytes
  (8 dwords).

**E. Consequence for the capture plans:** the four rows leave the
`_deferred` set on BOTH channels (S0/S0W reach ZERO deferred rows);
anchor counts gain cgr/bin/lnk/yline/zbase (+5) on every TS-bearing
scenario. Zero canonical-chain movement (plan-only infra, the
S0-12a/D152 class).

## 7j.70. THE SAVED.BDL RESTORE HEADER WALK — the slot grammar EXW-anchored at the instruction level (stride/name/mask/zone/score/money/difficulty offsets + the empty-slot predicate), retiring the 8street-layout citation for the P5 save-import seam (2026-08-27, worker 42041a21 claim 1, item p5-zonea-mission1-parity; objdump-only from ghidra-project/exw-text-objdump.txt — no Ghidra run; read-only byte probe of the shipped game-data/BEDLAM/SAVED.BDL + OPTIONS.BDL with MANIFEST.sha256 clean before and after) [verified]

The P5 zone acceptance (PLAN §6 P5 / P5-ZONE-GATES §1 criterion 6)
requires the ORIGINAL SAVED/OPTIONS.BDL import to be read-only,
bounds-checked and fuzzed. The 180-B slot layout was previously only
8street-cited ([CPP] save.cpp / [ASM] save_game — RESEARCH-8STREET
§2 `.BDL` row); the 8street policy demands EXW re-anchoring before
it can back an engine seam. Decode of the save-load restore arm (the
name-entry jump-table case at 0x43c258, slot dispatch 0x43c26e):

```
43c26e: imul eax, edx, 0xb4      ; slot stride = 0xB4 = 180 bytes
43c274: mov  edx, 0x4eae58       ; the 5-slot staging buffer (5*180=900)
43c27b: mov  eax, [edx+0xc]      ; dword@slot+0xC
43c283: test eax,eax; je 43c558  ; ZERO -> the EMPTY-slot exit arm
43c289: mov  ecx,8; mov edi,0x4e444c; mov esi,edx
43c295: call 0x44745e            ; memcpy(name_cell, slot+0, 8)   name @+0x00
43c2a2: add  edi,8               ; cursor := slot+8
43c2b6: mov  esi,[edi]           ; mask  = dword @+0x08
43c2b3: movsx eax,WORD [eax]     ; zone  = SIGNED word @+0x0C (slot+0xC)
43c2b8: mov  ds:0x4edd8c,eax     ; ZONE cell := zone (the §7j.64/F writer)
43c371..383: score = dword @+0x0E -> 0x4dd40c
43c390..395: money = dword @+0x12 -> 0x46ae70
43c3a0..a6: difficulty = movsx word @+0x16 -> 0x46cbf8
43c3ae..: the 7x7-word weapon row walk @+0x18.. (the §7j.64/B4 decode,
          player-type-indexed imul 0x62), then the 0x1C-stride chassis
          row (0x4deafc, §7j.67 boundary) — already pinned, not re-walked
```

VERIFIED facts pinned by this walk:
- SLOT GRAMMAR (EXW, addresses above): stride 0xB4 (180); name 8 B at
  +0x00; completed-missions bitmask dword at +0x08; zone SIGNED word
  at +0x0C; hiscore/score dword at +0x0E; money dword at +0x12;
  difficulty SIGNED word at +0x16; weapon rows from +0x18. This
  independently CONFIRMS the 8street header offsets — now anchored,
  no longer cited.
- EMPTY-SLOT PREDICATE: the restore tests the DWORD at +0x0C (the
  zone word widens to a dword read) against zero and branches to the
  0x43c558 exit arm — a slot whose zone dword is zero is NEVER
  restored (the shipped file's four "EMPTY" slots are exactly this
  shape: name "EMPTY", all-zero payload).
- MASK SEMANTICS: after the zone cell write, the arm replays
  completion zone-by-zone (0x43c2bf..0x43c2fb: FUN_004474ef(zone',
  sub=1..5) for every zone' < zone), then for the CURRENT zone marks
  exactly the set bits of the saved mask (0x43c306..0x43c36c: tests
  si&1/2/4/8/0x10 -> FUN_004474ef(zone, sub)). I.e. the mask is the
  CURRENT stage's completed-sub bitmask — entering zone N implies all
  subs of zones < N complete. Our Episode {stage, mask} models this
  exactly (fsm.rs stage_slot validation: stage 1..=8, mask ⊆
  FULL_MASK[stage]); the restore's zone cell is our stage value
  verbatim (zone cell fresh = 1 = ZONEA, §7j.64/B).
- SHIPPED CORPUS (read-only probe, MANIFEST-clean): SAVED.BDL = 900 B
  = 5 x 180 exactly (the stride arithmetic closes on the real file);
  slot 0 = "PLAYER", mask 0, zone 2 (-> ZONEB), score 0xA40B, money
  0x244 (580), difficulty 1; slots 1..4 = "EMPTY" + zero payload (the
  empty predicate holds). OPTIONS.BDL = 41 B (backbuffer 1, actionpan
  1, language 0, cd_audio 2, name "Player", volume 75, code_no_title
  1, midi 1, sound 1, installdrive 'C') — the typed import
  (bedlam-game config.rs over assets bdl.rs) is already the reader;
  volume 75 <= 100 keeps the domain check clean on the real bytes.

ENGINE CONSEQUENCE (landed this unit, bedlam-game save.rs): the
original-save import seam is the bounded header walk above —
exactly-900 length check, slot index < 5, the EXW empty predicate,
signed-word zone in 1..=8 with mask ⊆ FULL_MASK[zone] (never guess),
money/score/difficulty RETURNED not staged (sim-side per DESIGN-GAME
sec 3, the §7j.64 cell census), and the staging itself through the
existing stage_episode_slot seam (the 0x43c2b8 zone-cell write + the
mask replay are precisely what that seam models, D51). No writer
exists and none is owed for parity (new saves use the new versioned
format per PLAN §6 P5).

## 7j.71. THE KIND-1 WANDERER — the .NME S2 loader walk + the k1
controller body (0x414c96..0x415216) + its helper family decoded
whole (2026-08-27, worker 58b640c3 claim 1, item
p5-critter-state-g2-wanderers; objdump-only from the committed
ghidra-project/exw-text-objdump.txt + objdump -s table reads of
BEDLAM.EXW + the §7j.18 loader decompile
ghidra-project/exw-critterpoi-loader.txt — no Ghidra run; read-only
corpus byte access with MANIFEST.sha256 clean before and after)
[verified]

Method: the k1 body + FUN_00417af2/FUN_004186fc/FUN_00418250/
FUN_0041f8f9/FUN_0041e231/FUN_00417e2f walked instruction-by-
instruction from the committed objdump; the DIR jump table 0x412f08
and the 8-sample offset tables 0x4543e4/0x454404 read as raw bytes
(objdump -s). All facts [verified] against those artifacts unless
tagged.

1. **THE S2 LOADER WALK (the §7j.18 S2 gloss made exact).** The
   decompile's second section (10-B recs; w0 marker, w1 unused,
   w2 unused, w3 = x tile, w4 = y tile): per record spawns
   `[0x46cbf8]+3` (difficulty+3) each; x = w3·0x20+0x10,
   y = w4·0x20+0x10 (RAW px — NOT Q13; the k1 steppers ±6 and the
   bounds `width·0x20` confirm the scale); the z SEARCH walks the
   DAT volume DOWN from level 6 at tile (w3,w4): continue while
   tile==0, on the first non-air tile level L accept iff tile(L)
   ∈ 1..3 (a >3 tile is remembered in iVar4 but the spawn gate
   re-reads tile(L) and rejects it), then the STAND gate requires
   tile(L+1)==0 (air above); z = z-restore d@+0x4E = L·0x20+0x1F.
   NEW PINS (absent from §7j.18): **DIR w@+0x58 := 0xFFFF at
   spawn** (a fresh wanderer is idle), anim/frame w@+0x5A := 0,
   species w@+0x02 := 1, state w@+0x00 := 1, presence w@+0x24 := 1,
   countdown w@+0x56 := FUN_0041ec1c(10)+10 (one bounded-pick draw
   per spawned critter — the section's only stream draw; "scatter"
   = FUN_0041ec1c, NOT RandA&n). **hp w@+0x06 :=
   200 + (200·[0x46ae8c])/27 — the scalar is the LINEAR MISSION m
   (§7j.64/D153), NOT difficulty: CORRECTS the §7j.18 gloss
   "hp = base+(base·difficulty)/27"** (the imul census: S1 0xAF
   @0x4165db, S2 0xC8 @0x416793, S4 0xC8 @0x416a65, S5 0x5DC
   @0x416b7c, all ×[0x46ae8c] — the S3/S6 0x96 sites use the same
   cell via ecx/ebp loads).
2. **THE k1 CONTROLLER BODY** (kind table 0x412f18 case 1). Entry
   sequence: (a) FUN_004186fc(idx) — the DOOR-TILE GATE: linear
   index = (y>>5)·W+(x>>5) (the presence-mark geometry); index out
   of [0, W·H) → clean; else byte[0x4796d5 + 30·index] ≠ 0 (the
   per-tile variant/door flag, 30-B type-DB rows, §7j.12) →
   FUN_00418250(idx) death; (b) FUN_00417e2f(idx) — the
   SUICIDE-BOMB trigger; **return convention is EXPLICIT
   (CORRECTS §7j.17/2's "EAX-leak" hypothesis): the far path
   `xor eax,eax` @0x417f25 → 0 = continue wandering; the explode
   path `mov eax,1` @0x417f1b → 1 = skip the body this frame**;
   explode (nearest-robot octile < 0x30 px, FUN_00417c00):
   presence := 0 + 8 iterations (delay = counter>>1) of {3 jitter
   draws (z+(RandA&0xF), y+(RandA&0x3F)−0x1F, x+(RandA&0x3F)−0x1F)
   + 1× debris KIND 1 FUN_00420608 (draw-free, deterministic
   max-age allocator) + TWO FUN_0041ec1c(3) draws (the splash
   y/x tile picks: y = pick+y_tile−1, x = pick+x_tile−1,
   z = z_tile+1) + the FUN_00424355 SPLASH stager (draw-free)} =
   5 draws × 8 = 40 draws; (c) the substep loop. [CORRECTED
   in-place 2026-08-27: the first write of this item counted 4
   draws/iteration — it missed the second FUN_0041ec1c site
   @0x417ef8.]
3. **THE WANDER STATE MACHINE (species substeps per frame; species
   ≡ 1 for S2 spawn and nothing re-stamps it).** State = the
   (countdown w@+0x56, DIR w@+0x58) pair; DIR ∈ {0,1,2,3} = walk
   direction, −1 = idle. Per substep, HEAD FIRST: countdown−−, then
   - countdown > 0 ∧ DIR == −1 → the IDLE SQUASH @0x4151a5:
     {DIR := −1, countdown := 1, z := z-restore} — the idle pause
     lasts exactly ONE substep after the dec, so the 8..15/12..27
     pause words below NEVER take effect as written (they are
     squashed to 1 on their next substep) — the RUNTIME pause
     between walks is 2 substeps total [verified asm; the
     §7j.17 "pause 8..27" gloss describes the squashed constants];
   - countdown > 0 ∧ DIR ∈ 0..3 → WALK: `jmp [0x412f08 + DIR·4]`
     — the DIR table (bytes @0x412f08): **{0 → 0x414fb9 y−6,
     1 → 0x414d56 x+6, 2 → 0x414e40 y+6, 3 → 0x4150af x−6}** —
     the SAME 4-way convention as the mode-9 steppers/§7j.29
     acquisition; the step attempt (one per substep):
     z-band gate (z < 0 ∨ z ≥ 0x100 → FUN_00418250 death, and the
     case CONTINUES — the substep loop does not re-check
     presence); map-bounds gate on the STEPPED coordinate vs
     [0x4eddec]·0x20 (x) / [0x4eddf0]·0x20 (y) → out: {DIR := −1,
     countdown := (RandA&0xF)+0xC, z := z-restore} (one draw);
     else the wall probe FUN_0041f8f9(stepped, other, z-restore)
     — pass: COMMIT the stepped coordinate (z unchanged); fail:
     {DIR := −1, countdown := (RandA&0xF)+0xC} (one draw, NO
     z-restore);
   - countdown ≤ 0 ∧ DIR ≠ −1 → WALK-END @0x414d30: {DIR := −1,
     countdown := (RandA&7)+8} (one draw);
   - countdown ≤ 0 ∧ DIR == −1 → the PICK @0x414f89:
     countdown := (RandA&0xF)+0xA (draw 1); (RandA&3)==0 (draw 2)
     → DIR := RandA()&3 (draw 3) else DIR := FUN_00417af2(idx);
     then anim/frame w@+0x5A := DIR (both paths, @0x414d24).
   DRAW BUDGET per substep: walk-step 0; idle-squash 0; walk-end 1;
   pick 2 (+1 on the 25% branch).
4. **FUN_00417af2(idx) — the toward-robot 4-way picker** (no draws,
   no difficulty): nearest ALIVE robot (FUN_00417ba1, sentinel
   10_000_000, robot bank +0x7C alive word); cy/ry on robot +4,
   cx/rx on robot +0 (both >>8); **y-axis wins ties** (DX ≤ DY →
   y): cy ≥ ry → 0 else 2; the x branch (DX > DY): cx > rx → 1
   else 3; the cy==ry ∧ DX==0 degenerate lands on 0 (block-1 edge).
5. **FUN_0041f8f9(x, y, z) — the WANDER WALL PROBE (8 samples,
   returns 1 pass / 0 fail)**: sample offsets from the dword tables
   0x4543e4 (x) / 0x454404 (y) = **(−11,−11), (−11,+12), (+12,−11),
   (+12,+12), (0,−11), (0,+12), (−11,0), (+12,0)** (the 3×3
   footprint minus the center); per sample: bounds (sx/sy in [0,
   W/H·0x20)); FUN_0041e231(sx, sy, min(z,0xFF)) must return EXACTLY
   the passed z (the floor/surface probe = the engine floor_z
   model, FUN_0041e231 = "get_z_pos"); the DAT volume tile at
   (sx>>5, sy>>5, z>>5) via FUN_0041eb4c must be ≤ 3 (air or
   standable); any failure → 0.
6. **FUN_00418250(idx) — the wanderer DEATH path** (the z-band and
   door-tile exits): MODE w@+0x0C := 7, presence w@+0x24 := 0; iff
   x/y/z all in bounds (x < W, y < H, z < 8 — TILE units) spawn 1×
   debris KIND 1 at (x<<8, y<<8, z<<8) via FUN_00420608 (px→Q13);
   the k1 body IGNORES mode entirely — the observable is
   presence := 0 (skip next frame) + the debris row.
7. **DIFFICULTY SCALING — there is NONE in the k1 body** (zero
   [0x46cbf8] sites in 0x414c96..0x415216); the wanderer's
   difficulty coupling is loader-side only (spawn count d+3) plus
   the hp scalar above (linear mission m). The §7j.15 "12 sites in
   0x412f34..0x41547E" census stands (they live in the k4/k56/k7
   bodies).
8. EPILOGUE: the k1 tail 0x4151f0 computes the presence-mark cell
   (y>>5 row ptr + x>>5 — RAW-px scale like kind 4) and jumps the
   shared common tail 0x413fa7 (presence-mark byte + z-settle +
   trap re-probe — the documented no-draw E-gaps, module doc of
   bedlam-core::critter).
ENGINE CONSEQUENCE (landed this unit): S2 accepted by
stage_critters (kind 1, RAW-px coords, DIR −1 spawn seed, hp =
200+(200·m)/27 via MissionSim::linear — §7j.71/1's [0x46ae8c] pin
for the S2 rows), the k1 controller body + the door gate (skipped
E-gap — no door-bank mirror engine-side; documented) + the suicide
trigger + the squash semantics land in bedlam-core::critter; the
8-sample probe is modeled as floor_z(sample)==z ∧ raw DAT tile ≤ 3
with the offset footprint exact. The S3/S4 hp scalars HOLD the
§7j.18 difficulty form DELIBERATELY: the S8 canonical chain stages
ZONEA S3+S4 and the m-scalar swap would move its pinned T2
critter-bank bytes — no scenario exercises S2, so no chain is
touched (the queue contract); the alignment rides the next G2
unit (§7j.72/3 — landed there 2026-08-28). The canonical
critter-bank blob is UNTOUCHED (new record fields dir/frame/
z_restore are not serialized; no chain movement).

## 7j.72. THE BALLISTIC-STATE-6 LOADER BLOCK + THE S3/S4 HP-SCALAR
ALIGNMENT (2026-08-28, worker b03463e5 claim 1, item
p5-critter-state-g2-ballistic6; decompile-only from the COMMITTED
ghidra-project/exw-critterpoi-loader.txt — the §7j.18 dump — no
Ghidra run; corpus bytes read only by the census/gate test runs,
MANIFEST.sha256 clean before and after) [verified]

1. **THE S6 STAGING BLOCK walked exact** (the fourth staged section
   of the FUN_00416458 schedule, file order S1..S8): per 8-B record
   — **NO inner spawn loop** (ONE critter per record at EVERY
   difficulty: the S3/S7 multiplier preambles and their RandA draws
   are absent) and **NO stream draws of any kind** (zero RandA /
   FUN_0041ec1c sites in the block). Stamps: x d@+0x36 =
   w2·0x2000+0xF00, y d@+0x3A = w3·0x2000+0xF00 (Q13), z d@+0x3E =
   FUN_0041e411(x>>8, y>>8, w1<<5) — the floor probe, the S3 call
   shape verbatim; the 8 octile dists w@+0x60..+0x6E = the
   direction-table probes (0x4543e4/0x454404) at the spawn z (the
   same loop as S3); kind w@+0x00 = 6, species w@+0x02 = 3, mode
   w@+0x0C = 8, anim w@+0x0E = 5, heading d@+0x10 = 0x72, presence
   w@+0x24 = 1, countdown w@+0x56 = 0 — **the S3 stamps verbatim**
   with the kind word 6. NO home stamps (only S5 writes
   +0x42/+0x46/+0x4A).
2. **hp w@+0x06 = 0x96 + ([0x46ae8c]·0x96)/0x1B** — the same
   linear-mission m cell §7j.71/1 pinned for every section (here a
   plain DAT_0046ae8c load; 0x96 = 150, 0x1B = 27). This is the
   last hp site the §7j.18 difficulty gloss covered; with it the
   imul census closes: EVERY section's scalar is [0x46ae8c].
3. **ENGINE CONSEQUENCE (landed this unit)**: `stage_critters`
   accepts .NME section 6 (kind 6 → the shared k5/6 mixed body,
   already landed §7j.42/3 — the dispatch arm `5 | 6` predates the
   staging); the block mirrors the E S3 staging verbatim with kind
   6 and ONE draw-free spawn. The E `home_x/home_y = spawn`
   convention rides S3's (the ORIGINAL leaves home zeroed for
   S3/S6 — the E leash seam's documented convention, unchanged).
4. **THE S3/S4 SCALAR ALIGNMENT (the D179 rider landed)**: S3/S4 hp
   now read the same m (`MissionSim::linear`) as S2 — the §7j.18
   difficulty hold retired. CHAIN DECISION (deliberate re-baseline,
   the scenario the D179 queue item named): the S8 canonical
   scenario stages ZONEA S3+S4 under `critters = 1` with NO
   `destroy = 1`, and `linear` is destroy-staged → **m = 0 there**
   → the staged hp drops 155→150 (kind-5) and 207→200 (kind-4) and
   the S8 canonical chain moves (canonical_dump_gate corpus_s8 +
   differ_gate's S8 row re-baselined in the same commits). The
   ORIGINAL at ZONEA/M1 reads the derived cell clamp(5·(zone−2)+
   mission−1, 1, 26) = 1 → hp 155/207; the divergence is the S8
   scenario's own deliberate no-destroy staging (its header's
   empty-bank note) — the same "0 when unstaged" class D179
   accepted for S2, and the same asymmetry its destroy-family
   staging already carries. Paths that stage destroy BEFORE
   critters (the canonical order; every census row) read the true
   derived m and stay faithful.

## 7j.73. THE SELECT MISSION-CHOICE SHELL — the runtime mission-number source decoded whole: the SELECT screen write pair (the 26-hot-spot SP arm + the 10-row MP arm), the load-time +5 that makes MISSION6/7 the MP-only files, the 27-record completion bank, and the FIVE-bit save-mask domain that closes the census G1 question (2026-08-28, worker 05e14378 claim 1, item p5-select-shell-g1; objdump-only from the committed ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus read; MANIFEST.sha256 clean before and after) [verified]

The census G1 class (P5-ZONE-GATES §6.2) asked how the original
selects a sub-mission past the 4-bit stage mask — the FULL_MASK=15
table is the **B2** save shape (@0x81d9a, RESEARCH-BEDLAM2-CENSUS
§7; a cross-binary borrow), and the question was what EXW's runtime
actually writes. Method: instruction walks of the SELECT screen
family (EXW entry FUN_0043e7d4 = EXD twin 0x50953, RE-EXD-MAP §2c)
0x43eb67..0x43eedc, the restore replay 0x43c2bf..0x43c36c, the path
builder 0x44670c..0x4467f0, and the campaign cell writers
0x41c41c..0x41c4b9 / 0x41c9d6..0x41c9e5. All items [verified]
against the committed objdump.

1. **THE MISSION-DERIVATION BLOCK (the click handler).** Gated by
   [0x4eddcc] != 0 ∧ [0x4edb90] == 0 (0x43ed77..0x43ed8b — a click
   latch and a screen-mode cell). The hot id is read from a
   PIXEL→ID grid: `eax = [0x4eddc8]·0x280 + [0x4eddc4] + esi` where
   0x4eddc8/0x4eddc4 are the mouse y/x cells and esi = the grid
   base ([esp+0x318] := an arena block +0xC, staged 0x43ec0a..14;
   0x280 = 640 = the screen stride); `al = byte[eax]` = the hot-spot
   id under the cursor (0x43ed91..0x43edb4). The write arm then
   dispatches on `[0x4edb88]` (mode): != 2 → the SP arm, == 2 → the
   MP arm (0x43edb9..0x43edbc).
2. **THE SP ARM (0x43ee48..0x43ee9d) writes the MISSION cell only,
   zone-checked — missions 1..5, never 6/7.** The id dispatch
   (every range test [verified]):
   - id 1 ∧ zone cell [0x4edd8c] == 1 → mission := 1 (ZONEA's one
     mission; 0x43ee48..0x43ee60 writes [0x4edd88] := [0x4edd8c] = 1);
   - id 2..6 ∧ zone == 2 → mission := id − 1 (ZONEB missions 1..5);
   - id 7..0xB ∧ zone == 3 → mission := id − 6 (ZONEC 1..5);
   - id 0xC..0x10 ∧ zone == 4 → mission := id − 0xB (ZONED 1..5);
   - id 0x11..0x15 ∧ zone == 5 → mission := id − 0x10 (ZONEE 1..5);
   - id 0x16..0x1A ∧ zone == 6 → mission := id − 0x15 (ZONEF 1..5).
   Every arm writes [0x4edd88] at the shared tail 0x43eedc and
   NEVER writes the zone cell — the SP zone stays the campaign's
   (boot 0x41c41c..430 {zone 1, mission 1}; per-episode advance
   mission := 1 @0x41c4ad/0x41c4b9; campaign-advance zone++ @
   0x41c9d6..0x41c9e5, loop while episode < 7 = sets 1..7 = A..G).
   The id domain is exactly 26 hot spots = ZONEA{1} + 5×{B..F} =
   MAX_LINEAR (the linear counter's 26 SP missions; zone G has NO
   hot spot — it is the campaign-advance endgame, not selectable).
   **So SP play selects missions 1..5 per zone only; the 4-bit B2
   stage mask and the 5-mission SP cadence agree — no SP path ever
   writes 6 or 7.**
3. **THE MP ARM (0x43edc2..0x43ee43) writes BOTH cells — 10 list
   rows → {zone 2..6, mission 1..2}.** Row id (the same grid byte)
   1..2 → zone := 2, mission := id; 3..4 → zone := 3, mission :=
   id−2; 5..6 → zone := 4, mission := id−4; 7..8 → zone := 5,
   mission := id−6; 9..0xA → zone := 6, mission := id−8 (the zone
   writes 0x43edcb/0x43ede8/0x43ee18/0x43ee3d; the mission write
   shared tail 0x43eedc). The §7h.4/§7j.64/F "mission-number→set
   map" gloss is this arm — MP-ONLY, gated `[0x4edb88] == 2`
   (0x43edb9), fed by the MP lobby (mode 2 write 0x43f373, zone
   0x43f34b, mission 0x43f360).
4. **THE LOAD-TIME +5 — MISSION6/MISSION7 ARE THE MP-ONLY FILES
   [the G1 answer].** `build_mission_paths`@0044670c builds path1
   `EDITOR\ZONE{chr(0x41+[0x4edd8c])}\MISSION{<n>}` with
   `<n>` = the mission cell, EXCEPT: 0x4467ca `cmp [0x4edb88],2`;
   0x4467df `add eax,0x5` — **in MP mode the mission FILE number is
   the mission cell + 5** (itoa base 10 @0x4467f0, helper 0x44d291).
   Path2 (the zone-level MISSION{X}.CGR/BIN/MIN/LNK family) uses
   the zone letter only — unchanged. So the MP pair {zone 2..6,
   mission 1..2} loads `ZONE{B..F}/MISSION{6,7}.*`: **missions 6-7
   of every zone B-F are the two MULTIPLAYER missions, selected by
   the SELECT screen's MP list arm and renamed at load; they are
   not campaign sub-missions at all, which is why no stage mask —
   4-bit B2 or 5-bit EXW — can ever express them.** (Consistent:
   all ten B-F .NME files for missions 6/7 are 16-byte empties —
   no critter scripting for human-opponent maps, P5-ZONE-GATES
   §6.3.)
5. **THE 27-RECORD COMPLETION BANK (the SELECT screen's own state,
   NOT the save mask).** At 0x4decae: 0x144 bytes = **27 records**
   of 0xC = {+0 mission/sub, +4 zone, +8 done-flag} — one per
   linear mission (ZONEA{1} + 5×{B..F} + ZONEG{1} = 27).
   FUN_004474ef(zone, sub) marks the matching record done
   (0x447500..0x447517); FUN_0044751c(zone, sub) returns the record
   index or −1 (0x44751c..0x44754f). The SELECT screen reads it for
   the hot-spot draw state (0x43eb67..0x43ebcc: FUN_0044751c over
   the current {zone, mission} → done ? 0 : (next ? 1 :
   selected ? 2) — the 0/1/2 draw-state cells passed to the
   map-overlay helper 0x440888; precisely: done≠0 or no record → 0,
   not-done → 1, not-done ∧ record.mission == [0x4edd88] → 2).
   The CAMPAIGN dispatch also counts
   it (0x41c485..0x41c4ad: per-record done count of the current
   zone).
6. **THE SAVE MASK IS FIVE BITS (EXW), not the B2 four.** The
   restore replay (§7j.70's walk, now instruction-exact) marks
   subs 1..5 for EVERY zone' < current (0x43c2bf..0x43c2fb — edx
   literals 1,2,3,4,5 into FUN_004474ef) and then tests the saved
   mask dword's bits 0x1/0x2/0x4/0x8/**0x10** for the current zone
   (0x43c306..0x43c36c → FUN_004474ef(zone, sub=1..5)). So the EXW
   save-mask domain is `mask ⊆ 0b11111` per stage — FIVE sub-slots
   (missions 1..5), matching the SP arm's 5-mission cadence; the
   B2 table {0,1,0xF×6} is B2's own 4-sub campaign (B2's rip is
   "missing MISSION5.* in 5 of 6 zones" — its zones ship 4 SP
   missions; RESEARCH-BEDLAM2-CENSUS). Bits past 0x10 are never
   read by the restore (silent no-ops); unmatched (zone, sub)
   record lookups are no-ops too (zones A/G have only sub-1
   records). The engine's FULL_MASK stays the stage-ADVANCE table
   the canonical S5 semantics walk (D-keeping, deliberate); the
   save/SELECT domain is the 5-bit shape above.
7. **ENGINE SEAM (landed this unit):** the SELECT write pair is the
   runtime mission source, so the host models it as a SIBLING seam
   — `GameHost::stage_select_mission(zone 2..=6, mission 1..=2)`
   (the MP arm's exact write domain) planting the pair; the host's
   `mission_slot()` applies the +5 (mission.rs
   `SELECT_MP_FILE_OFFSET = 5`) so the pair names
   ZONE{B..F}/MISSION{6,7} exactly as the original loads them;
   campaign staging (`stage_episode_slot`, the restore/advance
   stand-in) clears the pair. The save-import domain widens to the
   5-bit SELECT_FULL_MASK (save.rs; the §7j.70 "missions-6/7 SELECT
   shape stays rejected" note retires — what stays rejected is
   anything past bit 4, which no original writer can produce);
   `mission_number_for_mask` saturates at 5 (the SP SELECT domain
   is 1..=5 — the campaign path can never name an MP file).

## 7j.74. THE KIND-2 SHOOTER — the .NME S1 loader walk + the k2
controller body (0x415216..0x415466) + its helper family decoded
whole (2026-08-28, worker 5ee1c0ce claim 1, item
p5-critter-state-g2-shooters; objdump-only from the committed
ghidra-project/exw-text-objdump.txt + the §7j.18 loader decompile
ghidra-project/exw-critterpoi-loader.txt — no Ghidra run, no
corpus read) [verified]

Method: the S1 block 0x4164b0..0x41664b and the k2 body
0x415216..0x415466 walked instruction-by-instruction from the
committed objdump; the helpers FUN_0041eb65/FUN_0041eb77/
FUN_0041ebf8/FUN_0041286f re-walked whole. All facts [verified]
against those artifacts unless tagged.

1. **THE S1 LOADER WALK (the §7j.18 S1 gloss made exact).** The
   decompile's first section (10-B recs; w0 marker, w1 spawn
   base, w2 flag, w3 = x tile, w4 = y tile):
   - spawn count := **w1 + [0x46cbf8] (difficulty), clamped to ≥ 1
     at 0x4164eb..0x4164f6** [NEW pin — the §7j.18 gloss named the
     sum but not the clamp]; then per ATTEMPT, in draw order:
   - **draw 1** FUN_0041ec1c(5) → x d@+0x36 := (w3 + pick − 2)·
     0x2000 (Q13; the caller's edx carries w3, `sar edx,0x10`
     @0x416510, the callee preserves it — the decompile's
     CONCAT22 second arg is a register-carry artifact); **draw 2**
     the same for y d@+0x3A := (w4 + pick − 2)·0x2000 (jitter
     −2..+2 tiles per axis);
   - **THE MAP-BOUNDS DROP GATE** [NEW pin, absent from §7j.18]:
     x ≤ 0 ∨ x>>0xD ≥ [0x4eddec] (map W) ∨ y ≤ 0 ∨ y>>0xD ≥
     [0x4eddf0] (map H) → the attempt is DROPPED (0x416558..
     0x416580: `jle`/`jge` to the inner-loop increment) — NO
     critter, count NOT incremented, but both scatter draws are
     already consumed;
   - on pass, the stamps (order as written): species w@+0x02 := 1,
     z d@+0x3E := 0xC000 (FIXED — 6 levels in the record's Q13 z
     convention; see /4 below), heading d@+0x10 := 0, **draw 3**
     anim w@+0x5A := RandA()&7 (0x4165a5..0x4165ba — the `xor
     dh,ah` idiom zeroes the high byte: net RandA&7), presence
     w@+0x24 := 1, **draw 4** variant d@+0x18 :=
     FUN_0041ec1c(4)+3 ∈ [3,7), **hp w@+0x06 := 0xAF +
     ([0x46ae8c]·0xAF)/27** (the imul site @0x4165db — the §7j.71/1
     census's S1 row; signed idiv, m = the linear mission cell),
     state w@+0x00 := 2, **draw 5** timer w@+0x72 :=
     (RandA()&0x1F)−0xF ∈ [−15,+15] (0x41660f..0x41662b), then
     iff w2 ≠ 0 the variant d@+0x18 is NEGATED (0x416632..
     0x41663f), count++.
   - **DRAW BUDGET: 2 per dropped attempt, 5 per landed critter.**
2. **THE k2 CONTROLLER BODY** (kind table 0x412f18 case 2; entered
   per frame per critter with ebp := idx·0x7E). The substep count
   = the record's species word (0x415239..0x415248 reads the
   dword@+0x00 high word — the SAME "species = substeps/frame"
   convention as the k4/k5/6 bodies, §7j.42/2; S1 stamps 1 and
   nothing re-stamps it). Per substep, in order:
   - anim w@+0x5A := (anim+1)&0xF (0x41524e..0x41525a);
   - **heading d@+0x10 := (heading + variant)&0xFF** (0x415261..
     0x415274) — the variant IS the curve rate: a positive variant
     turns one way, a NEGATIVE (w2-flagged) variant the other;
   - the SINE WALK: x += (FUN_0041eb65(heading)·0x14)>>8 and
     y += (FUN_0041eb77(heading)·0x14)>>8 (0x41527a..0x4152ac,
     `imul 0x14; sar 8`) — NO bounds gate, NO wall probe, NO
     z change (the k2 body never touches d@+0x3E after staging);
   - **the SFX pulse** (0x4152b2..0x4152d6): ONE RandA draw ALWAYS;
     (RandA&0x7F)==0 → FUN_0043a48e(eax=[0x4edffc] the SQUAWK
     voice-base, ebx=x>>8, ecx=y>>8, edx=0, stack 2) — the §7j.30
     census's 0x4152bd reader identified; the play itself draws
     NOTHING (verified: zero RandA/FUN_0041ec1c sites inside
     0x43a48e) and is the T4 E-gap engine-side;
   - **the fire gate** (0x4152db..0x4152e2): ONE RandA draw ALWAYS;
     (RandA&3)≠0 → 3/4 of substeps end here [CORRECTS the §7j.17
     "every 4th substep" gloss — it is a per-substep 1/4 CHANCE,
     the same RandA-gate form as the k1/k7 bodies];
   - the fire attempt: pick := FUN_0041ec1c([0x46ccbc]) — a
     bounded pick over the ROBOT COUNT cell (the 12-slot 0xA8-
     stride bank 0x4c69e4, count [0x46ccbc], D129); skip if the
     robot's alive word @+0x7C == 0 (0x4152f8..0x415307); slot :=
     FUN_0041286f — **the FIRST-FREE scan of the 50×0x22
     projectile bank 0x4cc654** (0x41286f..0x412895: word@+0 == 0
     → its index; full → −1 → skip) [NEW pin: FUN_0041286f's
     identity — it is the shared 0x4cc654 allocator, twin of the
     mode-2/0x68 lane's engine `enemy_free_slot`];
   - the aim: dirx := robot.x@+0 − x + ((RandA()&0x3F)·0x100 −
     0x1F00), diry := robot.y@+4 − y + ((RandA()&0x3F)·0x100 −
     0x1F00) (two more draws — the ±31/±32-px Q13 jitter ≈ ±1
     tile), dirz := (robot.z@+8 << 8) − z (robot z is Q5, D123;
     the critter z is Q13 here — see /4);
   - the RANGE GATE: dist := max(FUN_0041ebf8(dirx, diry), 1) >> 8
     (px) < **0x12C − (2−[0x46cbf8])·0x40** = 172/236/300/364/428
     for d=0..4 (0x4153d7..0x4153f1). **FUN_0041ebf8(x,y) is the
     2-D OCTILE max(|x|,|y|)+min/2 — the dz is DEAD for the gate**
     (the body computes dirz into ecx but the helper never reads
     it; dirz only feeds the velocity stamp) [NEW pin].
   - the STAMP (in range): projectile slot {w@+0x00 := 0x65
     (§7j.17's "shooter 0x65"), d@+0x02 := the critter x,
     d@+0x06 := y, d@+0x0A := z (Q13 — 6 levels at spawn),
     d@+0x0E/+0x12/+0x16 := dirx>>5 / diry>>5 / dirz>>5} — the
     velocity is the RAW direction >>5, NOT octile-normalized
     (closer targets → slower bolts; unlike the mode-2 0x68
     lane's dx·0x800/dist form, §7j.42/7).
   - **DRAW BUDGET per substep: 2 always (the two gates) + 1 pick
     + 2 jitter on the 1/4 arm = 2 or 5; the walk itself is
     draw-free.**
3. **THE HELPERS.** FUN_0041eb65(h) = `i16[*[0x46cbd0] + (h&0xFF)]`
   and FUN_0041eb77(h) = the same word at `(h−0x40)&0xFF` — the
   shared 256-word cos/sin table pair (cos(h−0x40) = sin) the
   engine models as `sine_word`; FUN_0041ebf8(x,y) =
   max(|x|,|y|) + min(|x|,|y|)/2 (2-D octile); FUN_0041286f = the
   first-free 0x4cc654 slot scan (index or −1).
4. **THE k2 Z SCALE IS Q13 — an exception to the module's "z is
   Q5 for every kind" rule.** The S1 z stamp 0xC000 is 6 levels
   in Q13 (0x2000/level), the projectile z stamp passes it through
   RAW, and the dirz math shifts the robot's Q5 z up by 8 to meet
   it (0x4153c4..0x4153cc). The kind-4/5/6 z cells stay Q5 (the
   (z+0x10)<<8 spawn convention); the kind-1 z is Q5-by-value
   (L·0x20+0x1F, the RAW-px family). The k2 body never re-derives
   z (no floor probe), so the scale only matters at staging, the
   dirz arithmetic, and the projectile stamp — all pinned above.
5. **DIFFICULTY COUPLING — loader-side only** (spawn count w1+d,
   and the RANGE GATE 300−(2−d)·64 inside the body; zero other
   [0x46cbf8] sites in 0x415216..0x415466). The hp scalar is the
   linear mission m (the closed imul census — S1's 0x4165db was
   its first-cited site).
ENGINE CONSEQUENCE (landed this unit): S1 accepted by
stage_critters (kind 2, species 1, Q13 coords incl. z 0xC000,
variant ±(pick(4)+3) by the w2 flag, anim RandA&7, timer
(RandA&0x1F)−0xF at +0x72, hp = 175+(175·m)/27, the bounds-drop
gate with the 2-draw attempt budget); the k2 body lands as the
sine-walk + the two always-draw gates + the 1/4 aimed 0x65 fire
(the squawk play is the T4 E-gap — the GATE draw is consumed
faithfully). No canonical scenario stages S1 (the S8 scenario
stages ZONEA S3+S4 only) → no chain movement. The canonical
critter-bank blob is UNTOUCHED (the new `variant` record field is
not serialized — the §7j.71 dir/frame/z_restore convention).

## 7j.75. THE KIND-3 CHASER — the .NME S5 loader walk + the k3
controller body (0x4145c1..0x414c96) + its helper family decoded
whole: the distance-ladder mode selector, the THREE roles of the
species word, and the pathfinder's wall-follow sector w@+0x5E
(2026-08-28, worker bc51a491 claim 1, item
p5-critter-state-g2-chasers-r2; objdump-only from the committed
ghidra-project/exw-text-objdump.txt + the §7j.18 loader decompile
ghidra-project/exw-critterpoi-loader.txt + raw DGROUP table reads
of BEDLAM.EXW (0x454b48/0x454edc) — no Ghidra run; read-only
corpus byte access with MANIFEST.sha256 clean before and after)
[verified]

Method: the S5 block of the committed loader decompile and the k3
body 0x4145c1..0x414c96 walked instruction-by-instruction from
the committed objdump; the helpers FUN_00417c00/FUN_00425498/
FUN_0041571c/FUN_0040cc27/FUN_0041e9a2/FUN_0040f277/
FUN_0041286f/FUN_0041eb7d/FUN_0041ebc1 re-walked whole; the
walk-pattern dword table 0x454b48 and the delay table 0x454edc
read as raw DGROUP bytes (VA 0x454000 = file 0x52600, the D135
anchor). All facts [verified] against those artifacts unless
tagged.

1. **THE S5 LOADER WALK (the §7j.18 S5 gloss made exact).** The
   decompile's fifth section (10-B recs; w0 marker, w1 = heading
   scalar, w2 = z level, w3 = x tile, w4 = y tile): **NO inner
   spawn loop — ONE critter per record at EVERY difficulty — and
   NO stream draws of any kind** (zero RandA/FUN_0041ec1c sites
   in the block; the S6 §7j.72/1 shape). Stamps in write order:
   x d@+0x36 := w3·0x2000+0xF00 and y d@+0x3A :=
   w4·0x2000+0xF00 (Q13); **home_x d@+0x42 := x, home_y d@+0x46
   := y** (S5 is the ONE section that stamps home — §7j.72/1's
   "only S5 writes +0x42/+0x46/+0x4A"); target-robot w@+0x7A :=
   0xFFFF (−1, no target); z d@+0x3E := FUN_0041e411(x>>8, y>>8,
   w2<<5) — the S3/S6 floor-probe shape verbatim (Q5 z, the
   record's z convention for kinds 3/4/5/6); the 8 corner-z words
   w@+0x60..+0x6E := the direction-table probes
   (0x4543e4/0x454404) at the spawn z (the S3/S6 loop verbatim);
   **home_z d@+0x4A := z** (after the corner loop — S5's third
   home stamp); **heading d@+0x10 := w1<<6 AND the wake-heading
   cell d@+0x14 := w1<<6** [CORRECTS the §7j.18 "+0x10 = +0x12 =
   w1<<6 (timer/leash)" gloss — the second stamp is the DWORD at
   +0x14, the preserved spawn heading the dormant teleport
   restores]; state w@+0x00 := 3, hp w@+0x06 := 0x5DC +
   ([0x46ae8c]·0x5DC)/27 (1500+(1500·m)/27 — the linear-mission
   scalar, the closed imul census's S5 site), presence w@+0x24 :=
   1, **MODE w@+0x0C := 0** (NOT 8 — the awake-idle state), **NO
   countdown stamp** (the bank memset leaves w@+0x56 = 0),
   **species w@+0x02 := 8** (the spawn-grace counter — see /3),
   count++.
2. **THE k3 CONTROLLER BODY** (kind table 0x412f18 case 3 →
   0x4145c1; entered once per frame per critter — **NO substep
   loop: the body never reads species as a substep count**,
   unlike k1/k2/k4/k5/6). In order:
   - **(a) TARGET-LIVENESS CHECK** (0x4145c1): target w@+0x7A ≠
     −1 ∧ robot[target] alive word @+0x7C == 0 (bank 0x4c69e4,
     stride 0xA8) → MODE := 8, countdown := 0, target := −1
     (species untouched). Runs BEFORE the mode dispatch — even a
     dormant (0xB) or dying (7) chaser whose target died flips to
     the awake-idle mode 8.
   - **(b) MODE 0xB DORMANT** (0x41460d): countdown <
     [0x454edc + difficulty·4] → countdown++ (word inc) and, on
     the frame the counter EXACTLY equals delay−0x14 (0x4146a0:
     `lea eax,[edi-0x14]`), the **TELEPORT-HOME BLOCK**
     [NEW pin, absent from §7j.17/§7j.42]: heading := the +0x14
     wake-heading cell, x := home_x, y := home_y, z := home_z —
     the chaser re-materializes at home 20 frames BEFORE waking.
     countdown ≥ delay → **WAKE** (0x414649): presence := 1,
     countdown := 0, **species := 0**, MODE := 8, **hp := 0x5DC
     FLAT** (1500, no m scalar — unlike the loader's staged hp),
     then straight to the epilogue (the ladder does not run this
     frame).
   - **(c) MODE 7 DYING** (0x4146f3): hp := 0, death_ctr
     (dword@+0x52)++, < 0x28 → epilogue; else MODE := 0xB,
     countdown := 0 (the death_ctr is NOT reset — the k5/6
     convention).
   - **(d) SPECIES DECREMENT** (0x414731): species > 0 →
     species−− (floor at 0; runs for every mode that reaches the
     ladder, i.e. all but 0xB/7).
   - **(e) THE DISTANCE LADDER** (0x41474e..0x414912) — ONE
     FUN_00417c00 nearest-alive-robot probe
     (x>>8/y>>8 → {out: dist px octile, ret: robot idx}; sentinel
     idx 0 / dist 10_000_000 when none) + the home leash =
     FUN_0041ebf8(home_x−x, home_y−y) on the >>8'd pair; then
     FOUR rules IN ORDER (the §7j.17 gloss made exact; the mode
     dispatch re-reads MODE after the ladder, so a flip RUNS THE
     NEW BODY THE SAME FRAME):
     R1 (0x4147ce): dist > 200 (0xC8) ∧ MODE == 2 → MODE := 10,
       countdown := 0, **species := 0x20 (32)**, target := −1
       [the 0x20 stamps SPECIES — the return-home WALK DURATION,
       NOT countdown; corrects the naive "countdown := 32"
       reading — the register-arg walk shows edx→+0x02, edi→
       +0x56];
     R2 (0x41481f): **species == 0** ∧ dist < 200 ∧ leash < 400
       (0x190) ∧ MODE ∉ {3,2} → MODE := 3, target := robot,
       countdown := 0 [NEW pin: the species==0 gate — the 8-frame
       spawn GRACE; a fresh chaser cannot approach for 8 frames];
     R3 (0x41487e): dist < 100 (0x64) ∧ MODE ≠ 2 → MODE := 2,
       target := robot, countdown := 0;
     R4 (0x4148c1): leash ≥ 400 ∧ MODE ≠ 10 → MODE := 10,
       countdown := 0, species := 0x20, target := −1 (a mode-3
       chaser past the leash flips home mid-chase).
   - **(f) MODE 3 APPROACH** (0x41492b): countdown == 0 →
     countdown := 9 + heading := the 8-SECTOR SNAP of
     FUN_00425498(aim at robot[target]'s LIVE x/y): ((angle+0xF)
     &0xFF)>>5 &7)<<5 (the +15 = half-sector rounding — headings
     land on {0,0x20,...,0xE0}); then the WALK GATE: dword
     [0x454b48 + countdown·4] ≠ 0 → FUN_0041571c(idx, heading)
     (the pathfinder step, /4); then countdown−− (the tail
     0x414c59 — the dec is AFTER the table read, so the table
     sees the fresh 9 on aim frames; table[0] is never read).
   - **(g) MODE 2 ATTACK** (0x4149eb): countdown == 0 → the same
     snap-aim at robot[target]; countdown++ (word inc) then
     countdown > 3 → countdown := 0 (the 5-frame aim cycle
     0→1→2→3→4→0); then the fire EVERY FRAME (the §7j.17 ">4
     shots → reset" gloss = this countdown wrap, NOT a fire
     gate): slot := FUN_0041286f (the first-free 0x4cc654 scan,
     §7j.74/2) — full (−1) → done (no countdown dec); else the
     0x67 STAMP with the FULL 3-D octile-normalized velocity at
     the LIVE robot position — dx = robot.x>>8 − x>>8, dy =
     robot.y>>8 − y>>8 (px), dz = (robot.z+4) − (z+0x10) (Q5,
     the +4 muzzle/center offsets); dist =
     max(FUN_0041ebf8(dx,dy),1); vx = dx·0x800/dist, vy =
     dy·0x800/dist; vz = dz·0x8000 / max(octile(dist<<4, dz<<4),
     1) (the second-octile z denominator, the mode-2 0x68 lane's
     exact math, §7j.42/7); projectile {w@+0x00 := 0x67,
     d@+0x02 := x (Q13), d@+0x06 := y, d@+0x0A := (z+0x10)<<8,
     d@+0x0E := vx, d@+0x12 := vy, d@+0x16 := vz}. No jitter, no
     range gate (the ladder already owns the bands), no draws.
   - **(h) MODE 0xA RETURN-HOME** (0x414bbc): countdown == 0 →
     countdown := 9 + the snap-aim at HOME (FUN_00425498 with
     ebx/ecx = home_x/home_y>>8); then the same walk-table gate
     + FUN_0041571c + countdown−− (the shared tail).
   - **(i) MODES 0 AND 8 have NO body** — awake-idle: only the
     ladder acts (the loader spawns MODE 0; the target-death flip
     and the wake set MODE 8 — the same role).
   - **(j) TAIL** (0x414c67): the presence-mark cell prep (y>>13
     row ptr + x) → the shared epilogue 0x413fa2.
3. **THE THREE ROLES OF THE SPECIES WORD for kind 3** [NEW pin —
   the "species = substeps" rule of §7j.42/2 does NOT hold here]:
   (1) the S5 spawn grace — stamped 8 by the loader, decremented
   per frame at (d), and READ as the R2 gate (species == 0);
   (2) the return-home walk duration — stamped 0x20 (32) by R1/R4
   and decremented to 0 (a mode-10 chaser walks home for 32
   frames, then R2 can re-fire when a robot is near — species is
   the walk budget, NOT a substep count);
   (3) the wake clears it (0 — a woken chaser can approach
   immediately).
4. **FUN_0041571c(idx, heading) — THE PATHFINDER STEP**
   (0x41571c..0x415b6e) [walked whole; the §7j.17 "pathfinder
   step" pin made instruction-exact]:
   - dx = FUN_0041eb65(heading)>>5 (cos word), dy =
     FUN_0041eb77(heading)>>5 (sin) — the shared 256-word
     table pair;
   - OPEN PATH: FUN_0040cc27(idx, dx, dy) passes → x += dx,
     y += dy, **sector w@+0x5E := (heading+0x20)&0xC0** (the
     4-way mirror; NOT the kind-1 DIR +0x58 — 0x4cfff6 is the
     record's +0x5E word, between frame +0x5A and the corner
     words +0x60), heading UNCHANGED, → the tail helper;
   - BLOCKED: z := the entry z (restore), then the WALL-FOLLOW
     ladder dispatched on the CURRENT sector word: each arm first
     re-tries its own axis move (±0x200 Q13 = 2 px, NO sector
     write on the keep-move), then the two PERPENDICULAR
     candidates (each on success: sector := the move's sector,
     the ±0x200 step), all fail → no move. The perpendicular
     ORDER: sector 0x00 (−y) and 0x80 (+y) arms key on the
     HEADING arg (≥ 0x80 → the −x/−x-side candidate first, else
     the +x-side); sector 0x40 (+x) and 0xC0 (−x) arms key on
     the DY component (sin(heading)>>5 > 0x80 → the +y candidate
     first, else −y) [the two different keys are literal asm —
     two arms use the live edi heading, two the ebp dy]; every
     ladder exit copies sector w@+0x5E → heading d@+0x10 (the
     tail 0x415b44) — after any blocked frame the heading IS the
     wall-follow direction; the sector convention 0x00=−y,
     0x40=+x, 0x80=+y, 0xC0=−x (the DIR-table axes);
   - every exit calls FUN_0040f277(idx) — the presence-gated
     8-corner z-settle family (FUN_0041e411 + FUN_004222ce/
     FUN_0042343, the documented no-draw E-gap, module doc).
5. **FUN_0040cc27(idx, dx, dy) → FUN_0041e9a2((x+dx)>>8,
   (y+dy)>>8, idx)** — the TRY-MOVE gate [walked whole]: the
   8-sample footprint probe at the candidate cell with the z
   reference read from **the dword@+0x5E>>16 = w@+0x60 — the
   FIRST CORNER-Z WORD** (why the loader stages the corner
   words); per sample (the 0x4543e4/0x454404 ±11/±12 offsets):
   map bounds (px vs [0x4eddec]/[0x4eddf0]·0x20) ∧ floor :=
   FUN_0041e411(sx, sy, z) ≠ 0 ∧ |floor − z| ≤ 4; on PASS the
   gate SETTLES z d@+0x3E := FUN_0041e411(x, y, min(z,0xFF))
   (the center floor, clamped-high probe) and rewrites the 8
   corner words with the sample floors; returns pass.
6. **THE WALK-PATTERN TABLE 0x454b48** [raw DGROUP bytes,
   file 0x53148]: 10 dwords = **[0,0,1,1,0,0,0,1,1,1]** indexed
   by the live countdown (1..9; 0 never read — the aim sets 9
   first). The 10-frame mode-3/0xA walk cycle steps on countdown
   {9,8,7,3,2} = **6 steps per 10 frames** (3 quick, 3 rest, 2
   quick, 2 rest). The dwords past index 9 (0x67/0x62/0x92/
   0xA4/...) are a DIFFERENT table — never read by the k3 body
   (the countdown domain is 0..9 precisely BECAUSE R1/R4 stamp
   the 32 into species, not countdown; a naive countdown-32
   reading would overflow into that data — the register-arg walk
   rules it out).
7. **THE DELAY TABLE** [0x454edc, file 0x534dc]: 1500/900/600/
   400 for d=0..3 (the k4/k5/6 RESPAWN_DELAYS pin [1500,900,600]
   corroborated byte-exact; the 4th entry exists at d=3 — the
   engine's min(d,2) clamp convention stays, as the landed kinds).
8. **DIFFICULTY COUPLING — the dormant delay read only** (zero
   other [0x46cbf8] sites in the body). **DRAW BUDGET: the whole
   k3 chain is DRAW-FREE** — zero RandA/FUN_0041ec1c sites in
   0x4145c1..0x414c96 and in every helper reached
   (FUN_00417c00/FUN_0041ebf8/FUN_0042548→FUN_0041eb7d/ebc1
   [leaf table lookups]/FUN_0041286f/FUN_0041571c→FUN_0040cc27→
   FUN_0041e9a2→FUN_0041e411/FUN_0040f277). The S5 staging is
   draw-free too — a Chasers-only .NME consumes ZERO stream
   draws at load and per frame (the first such critter section;
   S6 is draw-free at load only).
ENGINE CONSEQUENCE (landed this unit): S5 accepted by
stage_critters (kind 3, Q13 x/y + Q5 z, home x/y/z staged, spawn
heading + the +0x14 wake-heading cell = w1<<6, species 8, MODE 0,
hp = 1500+(1500·m)/27 via MissionSim::linear); the k3 body lands
as the target-liveness flip + the dormant/wake/teleport machine +
the species triple role + the 4-rule ladder + the mode 3/2/0xA
bodies with the 8-sector snap aim and the every-frame 0x67 fire
(the live-robot 3-D octile aim); FUN_0041571c lands as the
open-path sine step + the wall-follow ladder on the new sector
word w@+0x5E (modeled walk gate: the documented 8-sample-probe
E-gap approximation — bounds + the center floor band, as the
landed kinds); the walk table [0,0,1,1,0,0,0,1,1,1] lands as the
const. No canonical scenario stages S5 (ZONEA/M1 hosts S3+S4
only — the §6.3 census) → no chain movement. The canonical
critter-bank blob is UNTOUCHED (the new home_z/spawn_heading/
seek_sector record fields are not serialized — the §7j.71
dir/frame/z_restore convention).

## 7j.76. THE KIND-7 CLOSE-COMBAT — the .NME S7 loader walk + the
k7 controller body (0x412f52..0x41367c) + the FUN_00412a19 STEER
helper + the in-record knock/mode-5 producer decoded whole
(2026-08-28, worker 7c028ff1 claim 1, item
p5-critter-state-g2-closecombat; objdump-only from the committed
ghidra-project/exw-text-objdump.txt + the §7j.18 loader decompile
ghidra-project/exw-critterpoi-loader.txt — no Ghidra run, no
corpus read) [verified]

Method: the S7 block of the committed loader decompile (the
section between the S6 block and the S8 POI block) and the k7 body
0x412f52..0x41367c walked instruction-by-instruction from the
committed objdump; the helpers FUN_00412a19/FUN_00417c00/
FUN_0041eb7d/FUN_0041ebc1/FUN_0041eb65/FUN_0041eb77/FUN_0041ebf8/
FUN_0041286f/FUN_0041a14f/FUN_00420608/FUN_00424355 and the
weapon→critter hit lane 0x419200..0x419390 re-walked whole. All
facts [verified] against those artifacts unless tagged.

1. **THE S7 LOADER WALK (the §7j.18 S7 gloss made exact).** The
   decompile's seventh section (6-B recs; w0 marker, w1 = x tile,
   w2 = y tile). The SPAWN COUNT is the §7j.18 S3 cascade made
   exact — NOT "max(difficulty,1)": d=0 → 1, d=1 →
   (RandA()&1)+1, d=2 → 2, d≥3 → 1 — and the count roll is ONE
   SECTION-LEVEL draw (computed BEFORE the record loop, so at d=1
   the section consumes exactly one RandA before any per-critter
   draw). Per spawned critter, in write order:
   - x d@+0x36 := w1·0x2000+0xF00 and y d@+0x3A :=
     w2·0x2000+0xF00 (Q13 tile cells — the S3/S5/S6 shape);
   - **z d@+0x3E := 0xDF FIXED** (Q5 by value — 6·0x20+0x1F, the
     level-6-top constant; NO floor probe, NO corner-z loop, NO
     home stamps — S7 stages none of the S5 home triple);
   - anim w@+0x0E := 0, countdown w@+0x56 := 0;
   - presence w@+0x24 := 1;
   - **heading d@+0x10 := FUN_0041ec1c(0xFF)** — the bounded
     RandA&0x7FFF-bucketed pick (§7j.42/3's idiv 0x8000/n clamp
     n−1 form) → 0..0xFE; the SECTION'S ONLY per-critter draw;
   - mode w@+0x0C := **3** (approach — ACTIVE from frame 0, never
     dormant), species w@+0x02 := 1, kind w@+0x00 := 7;
   - **hp w@+0x06 := 0x9C4 + (m·0x9C4)/27** (2500+(2500·m)/27 —
     the linear-mission scalar [0x46ae8c] as at every section's
     imul site; the decompile line `(iVar6*0x9c4)/0x1b + 0x9c4`);
   - count++.
   **NO map-bounds gate** (unlike S1 §7j.74/1 — an out-of-map tile
   stages anyway at the Q13 arithmetic). DRAW BUDGET: 1 at load
   iff d=1 (the count roll) + 1 per spawned critter.
2. **THE k7 CONTROLLER BODY** (kind table 0x412f18 case 7 →
   0x412f52; entered once per frame per critter; the record
   pointer idiom is idx·0x7E + 0x4cff98, and fields past +0x71
   address as 0x4d00xx — w@+0x72/74/76/78 = the facing/knock-vx/
   knock-vy/fall-rate cells). The substep count = the species word
   (the §7j.42/2 convention; S7 stamps 1). Per substep, in the
   original's order:
   - **HEAD**: species ≤ substep → exit (to the shared epilogue
     prep 0x41364e — the presence-mark cell y>>13 row + x>>13).
   - **MODE 7 DYING** (0x413618): countdown w@+0x56++ (word inc);
     countdown > 4 → **hp := 0 ∧ presence := 0** — a landed k7
     despawns on the FIFTH dying frame (mode 7 is entered by the
     mode-6 landing with countdown 0; the substep loop continues
     through the clear — the main loop skips it next frame).
   - **MODE 6 BALLISTIC** (0x412f99): the in-record knock triple
     integrates — newX := x + w@+0x74·2, newY := y + w@+0x76·2,
     newZ := z − w@+0x78, and the fall-rate ramps
     **w@+0x78 += 2 while < 0x18** (a 2/frame gravity ramp capped
     24); then the standard clamps (each ≥ 1; x/y edge-clamped to
     the map `[0x4eddec/df0]<<0xD − 1` form; z ≥ 1), the floor
     probe FUN_0041e411(newX>>8, newY>>8, newZ), and the LANDING
     TEST: `floor < newZ ∧ newZ ≠ 1` → the NO-LANDING path
     (0x413249: write the clamped newX/newY/newZ, keep mode 6,
     tail); else **LAND** (0x4130c5): z := the floor (post the
     ≥1 clamp), mode := 7, countdown := 0, then the LANDING
     EFFECTS in order —
     (a) **8× debris**: per i=1..8, three RandA draws →
         FUN_00420608(x>>8 + (RandA&0x3F)−0x1F, y>>8 +
         (RandA&0x3F)−0x1F, z + (RandA&0xF), kind 6, delay i,
         −1) — the 128-slot stager §7j/5, the chunk kind;
     (b) **5× splash tiles**: per i=1..5, two RandA draws →
         FUN_00424355((x>>13)+(RandA&3)−2, (y>>13)+(RandA&3)−2,
         min((z>>5)+2, 7), delay i) — the §7j.10/14 claim-gated
         tile stager, the staggered delay = the loop counter;
     (c) **24 effect rows**: FUN_0041a14f(x, y, (z+0x15)<<8,
         0x18) — the §7j.24/5 LRU row spawner (3-4 draws per row;
         the "controller ballistic landing (0x18 — the k7 body
         only)" pin §7j.43/2 confirmed at 0x413220..0x413248).
   - **MODE 5 KNOCK DRIFT** (0x413303): countdown++ FIRST, then
     countdown > 10 (0xA) → mode := 3, countdown := 0 (the tail
     engage runs THIS substep against the STALE scan cells — the
     [esp+0x28] dist is whatever the last default-path substep
     left); else the drift: newX := x + w@+0x74·2, newY :=
     y + w@+0x76·2, the same ≥1 + edge clamps, x/y written (NO z
     change). TEN drift frames, then back to approach.
   - **DEFAULT — every other mode** (0x4133c0): the
     nearest-alive-robot scan FUN_00417c00(x>>8, y>>8) → the
     per-frame stack cells dist@0x28/idx@0x2c (dist px octile,
     sentinel idx 0 / 10,000,000 when none). NOTHING else — modes
     0/8/9/0xA/0xB/2/4 have no k7 body (a dormant k7 is inert).
   - **THE POST-MODE TAIL** (0x4133e7): re-read mode; **mode == 3
     ∧ scan-dist < 0x320 (800 px)** → the ENGAGE block; countdown
     ≠ 0 → countdown−− (a recharging k7 neither aims nor moves);
     countdown == 0 →
     (a) **THE AIM + STEER**: dx := robot[scan-idx].x@+0 −
         (x & !0xFF), dy := robot.y − (y & !0xFF) (the LOW-BYTE
         SCRUB on the critter side — a within-tile aim
         quantization; robot x/y are the Q13 bank cells); dist :=
         max(FUN_0041ebf8(dx,dy)>>8, 1); aim :=
         FUN_0041eb7d/FUN_0041ebc1 on |dx|·0x80/dist (the shared
         angle-byte family); **heading := (heading +
         FUN_00412a19(aim, heading)) & 0xFF** — the ±1 STEER
         below;
     (b) **THE MOVE**: x := x + cos(heading)>>6, y :=
         y + sin(heading)>>6 (FUN_0041eb65/77 — the shared sine
         pair, arith shift), the same ≥1 + edge clamps; NO wall
         probe, NO z change;
     (c) **THE FIRE GATE — TWO CONJUNCTS**: scan-dist < 0x50
         (80 px point-blank) ∧ the FRAME-PHASE MODULO
         ([0x46ae68] g_frame_count + critter idx): d=0 → &0x1F==0
         (every 32), d=1 → &0xF==0 (every 16), d=2 → &0x7==0
         (every 8), **d≥3 → NEVER** (the jne at 0x413575 falls
         through to the tail) — §7j.16's "fire-rate gates
         32/16/8" made exact, idx-staggered;
     (d) **THE STAMP** (both gates pass + FUN_0041286f free):
         projectile slot {w@+0x00 := 0x69 (the BEAM — §7j.50),
         d@+0x02 := x, d@+0x06 := y (Q13, post-move),
         **d@+0x0A := 6 LITERAL** (NOT Q13 — the §7j.17 "z=6"
         pin), d@+0x1A := 0 (counter), d@+0x1E := 0x18 (TTL 24)};
         NO velocity stamps (the beam column is stationary — the
         bank's stale-velocity carryover is unobservable in the
         engine's modeled tick); then **countdown := 6** — the
         fire RECHARGE (with the ≥8 modulo periods the modulo
         gate dominates at every difficulty).
3. **FUN_00412a19(aim, heading) — THE STEER HELPER** [NEW pin,
   0x412a19..0x412a49]: aim == heading → 0; else δ :=
   wrap-to-byte-range(aim − heading) (the +0x100/−0x100 wrap into
   [−0x100, 0xFF]); **δ < 0x80 → +1 else −1** (the 0x80 tie turns
   clockwise) — a ±1-per-substep turn toward the aim by the
   shorter arc. The queue's "steer" gloss anchored.
4. **THE WEAPON→CRITTER KNOCK LANE for kind 7** (0x419200..0x419390
   inside the hit applier — the §7j.23 mode-2 lane): a present k7
   in mode ∈ {6,7} is IMMUNE (0x419229..0x419264 — plus the 0xB
   arm at 0x419395 for the second pass variant); the px box ≤ 0x20
   + the weapon-z band; on hit, in order: hp −= damage
   (FUN_00419aff weapon-keyed), attacker w@+0x04 := owner,
   fuse/hit-flash w@+0x7C := 1, impact d@+0x1C/+0x20 :=
   (shooter x,y)<<8 (the SAME impact pair the k4/k5/6 knock
   stages), **heading := (angle_byte(crit − shooter) + 0x80)&0xFF
   — the AWAY heading, then the in-record knock vector
   w@+0x74 := cos(heading)>>6, w@+0x76 := sin(heading)>>6** (the
   §7j.23 "kind 7 in-record knock instead" pin made exact);
   hp ≤ 0 → FUN_0041896c (the k7 death handler — §7j.24: kind := 6
   + w@+0x78 := 1 + 3 falling-gib debris + 1× k0xD + CACODETH +
   bounty +1000 — STILL the unlanded §7j.24 subset alongside
   k1/k2/k3); else **mode := 5, countdown := 0** — the knock
   drift producer. (The k4/k5/6 lane instead stages
   FUN_0041a028 juice on a 25% roll — kind 7 has NO juice roll.)
5. **DIFFICULTY COUPLING**: loader-side none beyond the count
   cascade (the hp scalar is m, not d); body-side the fire modulo
   alone (32/16/8 by d). **DRAW BUDGET: the k7 chain is draw-free
   on the approach/move/fire path** (zero RandA/FUN_0041ec1c
   sites in 0x412f90..0x4135e7 — the scan, aim, steer, move and
   the 0x69 stamp consume nothing); the ONLY body draws are the
   mode-6 landing's 24 (debris) + 10 (splash) + the effect rows;
   the S7 staging draws 1/critter (+1 at d=1).
6. **The §7j.42 k7 gloss CORRECTED**: the "engage leash
   (d+1)·0x40+600" is the k5/6 mode-8 band (§7j.42/3), NOT k7 —
   the k7 engage gate is the FLAT dist < 0x320 (800), and the
   point-blank fire band is dist < 0x50; "32/16/8-frame cadence" =
   the (g_frame+idx) modulo gates (idx-staggered, d≥3 never), and
   the fire carries a 6-frame countdown recharge on top. The
   mode-6 entry producer was NOT found in the surveyed writers
   (the death handler flips kind to 6 instead) — the k7 mode-6
   body is landed faithfully as decoded with the producer
   observation noted [corpus-dead path, like §7j.42's noted
   gaps].
ENGINE CONSEQUENCE (landed this unit): S7 accepted by
stage_critters (kind 7, Q13 x/y + FIXED Q5 z 0xDF, heading =
bounded_pick(0xFF), mode 3, species 1, hp = 2500+(2500·m)/27, the
d-cascade count — the roll modeled PER-RECORD per the landed-S3
staging convention of §7j.72, so an EMPTY S7 consumes no draw
[the asm's roll is section-level and unconditional at d=1,
0x416e36..0x416e80 — a recorded engine deviation; fixing it for
S3+S7 together would move the canonical S8 chain and is queued as
its own re-baseline unit]); the k7 body
lands as the dying-despawn + the ballistic landing machine (the
fall-rate ramp, the floor landing test, the 8-debris/5-splash/
24-row effects) + the 10-frame knock drift + the default scan +
the steer-aim-move-0x69 fire chain with the two-conjunct gate and
the 6-frame recharge; the knock lane specializes kind 7 in the
weapon→critter hit applier (the away heading + the in-record
vector + mode 5). The new record fields (knock_vx/knock_vy/
fall_rate/scan_robot/scan_dist) are NOT serialized — the canonical
74-B critter-bank blob is untouched. No canonical scenario stages
S7 (ZONEA/M1 hosts S3+S4 only) → no chain movement. The k7 DEATH
handler (§7j.24) stays the documented unlanded subset (the
k1/k2/k3 precedent); the 0x69 beam TICK/impact (§7j.50's
terrain-only 50/100/200 re-key) stays the enemy_tick E-gap.

## 7j.77. THE S8 PERSONNEL/POI BANK — the .NME section-8 loader walk
made exact + the POI controller FUN_00412a98 whole + the damage lane
FUN_0040dc1b + the walker FUN_00415b6c + the two scans — the LAST G2
census class (2026-08-28, worker 0ecf083b claim 1, item
p5-personnel-poi-s8; objdump-only from the committed
ghidra-project/exw-critterpoi-loader.txt +
ghidra-project/exw-text-objdump.txt — no Ghidra run; read-only .NME
corpus census with MANIFEST.sha256 clean before and after)
[verified]

Method: the §7j.18 committed decompile of FUN_00416458 (its S8
block = loader lines 300-328) re-walked against the raw asm
(0x416fd9..0x417094) + the controller 0x412a98..0x412f21, the
walker 0x415b6c..0x415ff1, the exit scan 0x417c64..0x417cdd, the
robot scan 0x417c00..0x417c63, the damage lane 0x40dc1b..0x40dcc3,
the gate pair FUN_0040cc5e/0x41e859, and the cos/sin readers
0x41eb65/0x41eb77. **Reading convention (the sar-16 idiom):** the
Watcom code reads a record word at +2k as `dword@[base+2k−2] >>
16`; every ">>16" read below is a WORD read at the stated offset.

1. **THE FIELD MAP (0x1E-stride POI bank 0x4dabdc, count
   DAT_0046cbf0, 0xF00 B memset-0 at loader entry 0x41647d)**
   [verified, correcting §7j.17 item 3's unnamed +2/+6]:
   - w@+0x00 ACTIVE (1 present, 0 gone — the escape-complete
     clears it);
   - w@+0x02 HP — seeded 0x32 (50) by the loader, DECREMENTED by
     the ranged-attack applier's POI lane (FUN_0040dc1b
     `sub [poi+2], dx` @0x40dc2a; ≤0 → death, item 5). This is
     the "0x32" the §7j.18 gloss read as a timer;
   - w@+0x04 STATE {1 idle, 2 settle, 3 walk-out, 4 flee-to-exit,
     5 ESCAPE, 6/7 panic} — seeded 1 (IDLE);
   - w@+0x06 DEAD SEED 5 — stamped by the loader, read by NOTHING
     (every controller/animator read in the census is the >>16
     form = w@+8); a dead stamp like S1's +0x72 timer word;
   - w@+0x08 HEADING word — three writers: the loader seeds
     RandA()&7 RAW (0..7); the settle/walk-out/flee aims stamp
     ((dir+0xF)&0xFF)>>5&7<<5 (the 32-sector quantized octile
     direction, 0..0xE0); the walker tail copies the draw word's
     low part (one of {0,0x40,0x80,0xC0}). The cos/sin walk reads
     THIS word (0x412be9/0x412e60 — NOT +6);
   - w@+0x0A TIMER — multiplexed per state (idle counter /
     settle counter / walk budget / escape countdown / panic
     counter); seeded 0 by the memset;
   - w@+0x0C EXIT SLOT — the flee-begin stash (0x412afb); read by
     the state-4 validity re-check and the escape-complete dwell
     reset; seeded 0 by the memset;
   - d@+0x0E/+0x12/+0x16 x/y/z — Q13 x/y (0x2000 per tile), Q5 z;
   - d@+0x1A DRAW word — the walker's sprite quadrant
     (angle+0x20)&0xC0, copied to w@+8 at the walker tail.
2. **THE S8 LOADER WALK (FUN_00416458 tail, 0x416f6e..0x417094)**
   [verified; CORRECTS §7j.18/1's seed list — the 2026-08-21
   reading SWAPPED the +4/+6 stores]: `DAT_0046cbf0 = 0` @0x416f6e
   (the POI count reset), then ONE `u16 count + count×8-B record`
   section (the 8th and last of the §7j.18 schedule); per record,
   a fixed `do { } while (n < 4)` inner loop spawns exactly FOUR
   POIs (no difficulty term, no count draw):
   - draw order (exactly three RandA draws per POI):
     (a) x@+0xE = ((RandA()&0x1F) + (w2<<5)) · 0x100 — a RANDOM
     in-tile offset 0..0x1F00 instead of the S3/S5/S6 fixed
     +0xF00 (Q13: tile·0x2000 + r·0x100);
     (b) y@+0x12 = ((RandA()&0x1F) + (w3<<5)) · 0x100;
     (c) z@+0x16 = FUN_0041e411(x>>8, y>>8, w1<<5) — the SAME
     floor-probe family as S3/S5/S6 (draw-free);
     (d) heading w@+8 = RandA()&7 (the `xor dh,ah` byte juggle at
     0x41704a is the Watcom RandA return extraction — value &7);
   - the stamps: active 1, **state 1 (IDLE — personnel spawn
   IDLE; the "spawn directly in state 5 = ESCAPE" inference of
   §7j.18/1 is RETIRED, the stores were transposed)**, hp 0x32,
   dead +6 seed 5, timer/exit 0 (the memset).
   - hp: NONE scaling — the POI bank carries NO hp-formula word
   (no imul site in the block; the 0x32 is a literal) — the ONE
   .NME section whose hp is not base+(base·m)/27.
3. **THE CONTROLLER FUN_00412a98** (MissionShell call @0x447fe6;
   prologue 53 51 52 56 57 55) [verified whole]:
   - PER-ACTIVE-POI PROLOGUE (every frame): z :=
     FUN_0041e411(x>>8, y>>8, z) (the z re-settle); exit_idx :=
     FUN_00417c64(i, &[esp]) — the nearest-exit scan over the
     five 0x1C exit slots @0x4e662c (skip inactive@+0, octile
     dist FUN_0041ebf8 on (x−poi.x)>>8/(y−poi.y)>>8 vs
     slot x@+8/y@+0xC, best-fit, sentinel 0x989680) returning the
     slot index AND writing the best distance into the caller's
     [esp] scratch cell;
   - HEAD DISPATCH (states 1/2/3 only): ONE RandA draw —
     (RandA()&0xF) ≠ 0 → skip (a 1/16 lane); [esp] ≥ 0x180 (no
     open exit within 384 octile-px) → skip; exit[exit_idx].
     PHASE@+4 ≠ 2 (not landed-OPEN, §7j.19) → skip; else
     FLEE-BEGIN: state := 4, w@+0xC := exit_idx;
   - STATE 4 fast lane (before the main dispatch): [esp] < 0x10
     ∧ exit[w@+0xC].PHASE == 2 → ESCAPE-BEGIN: state := 5,
     timer := −1 (0xFFFF), the drop-in sound FUN_0043a48e
     (0x4edfe0, 2, x>>8, 0, y>>8);
   - STATE 5 ESCAPE: timer++; timer ≥ 10 → COMPLETE: active := 0,
     [0x4eba0c]++ (the escape counter — MissionShell resets it
     at 0x447933, the HUD tail reads it at 0x448402/0x448ce1),
     [0x4eba10] := 0x32 (the panic timer — the MissionShell tail
     0x448386 decrements it while >0, the banner-message lane),
     exit[w@+0xC].dwell@+0x18 := 0 (the multi-POI elevator
     reset, §7j.19's reader), the escape sound FUN_0043a48e
     (0x4edfa8, 3, …), and **FUN_00448b80(0x1388) — the 5000-pt
     award** (the zone-7 objective/score family);
   - STATE 4 FLEE body: exit[w@+0xC] inactive OR PHASE ≠ 2 →
     ABORT (timer := 0, state := 1); else re-aim w@+8 :=
     sector<<5 at the exit (FUN_00425498 octile dir, the same
     quantizer as the settle aim), then the walker
     FUN_00415b6c(i, cos(w@+8)>>6, sin(w@+8)>>6, w@+8) (the
     cos/sin readers 0x41eb65/0x41eb77 are the [0x46cbd0] table
     lookups — sin(a) = cos(a−0x40), draw-free), then timer−−;
     timer < 0 → timer := 0x2710 (10000 — the never-expire
     sentinel; the flee walk is otherwise unbounded);
   - STATE 1 IDLE: timer ≤ 10 → timer++ (done); timer > 10:
     [esp] ≥ 0xC0 (no open exit within 192) → the WALK-OUT gate;
     else ONE RandA draw, (RandA()&0xF) ≠ 0 → the WALK-OUT gate;
     hit → SETTLE-BEGIN: state := 2, timer := 0, [esp] :=
     nearest-robot dist (FUN_00417c00 — the alive-gated +0x7C
     scan over the 0xA8-stride robot bank, distance written
     through the ebx out-pointer; the DIST write is transient —
     nothing reads [esp] again before the next frame's exit scan
     overwrites it), w@+8 := sector<<5 aimed AT THE NEAREST
     ROBOT (face the rescuer);
   - the WALK-OUT gate (state 1 → 3): ONE RandA draw,
     (RandA()&0xF) ≠ 0 → done; hit → state := 3, timer :=
     (RandA()&0xF)+10 (the 10..25-frame walk budget), w@+8 :=
     (RandA()&7)<<5 (a random 8-way heading);
   - STATE 3 WALK-OUT: timer == 0 → state := 1; else timer−− and
     the walker FUN_00415b6c(i, cos(w@+8)>>6, sin(w@+8)>>6, w@+8);
   - STATE 2 SETTLE: timer++; timer > 8 → timer := 0, state := 1
     (a 9-frame stand-and-face);
   - STATE 6 PANIC-1: timer++; timer > 5 → state := 7;
   - STATE 7 PANIC-2: timer := 0 (every frame; inert — the dead
     POI stays ACTIVE forever, the animator draws the corpse);
   - per-frame draw budget: states 1/2/3 = 1 draw minimum (the
     head gate) + the state-1 gates (≤3 more on a transition
     frame); states 4/5/6/7 = ZERO draws.
4. **THE WALKER FUN_00415b6c(i, dx, dy, angle)** [verified]:
   saves z; if the move gate FUN_0040cc5e(i, dx, dy) passes →
   x += dx, y += dy, draw word := (angle+0x20)&0xC0; else
   restores z and walks the QUADRANT LADDER on the CURRENT draw
   word {0: y−0x200 then dx by angle≥0x80 (−x/0xC0 first, else
   +x/0x40); 0x40: x+0x200 then dy by dx>0x80 (+y/0x80 first,
   else −y/0); 0x80: y+0x200 then dx by angle≥0x80; 0xC0:
   x−0x200 then dy by dx>0x80} — each axis attempt gate-tested,
   first pass applies ±0x200 and rewrites the draw word, all
   blocked → no move; the tail copies the draw word's low word
   into w@+8. The gate core FUN_0041e859: floor :=
   FUN_0041e231(x>>8, y>>8) at the TARGET px; pass iff
   |floor − z| ≤ 4 (the critter walk_gate's 3 does NOT apply
   here), and z := floor on pass (the walker's z-restore on the
   blocked path is defensive — a failed gate never writes z).
5. **THE DAMAGE LANE FUN_0040dc1b(poi_idx, dmg)** (the
   ranged-attack applier's POI arm; reached from the critter
   0x65/0x67/0x68 walker family) [verified]: hp@+2 −= dmg;
   hp > 0 → done; hp ≤ 0 → DEATH: state := 6, timer := 0, ONE
   **RandB** draw (0x4029b6 — the second seed pair
   0x4ede4c/0x4ede4e, §7j.65) picking between two death sounds
   (FUN_0043a48e at 0x4edfb8/0x4edfbc), then the effect spawn
   FUN_00420608(x>>8, y>>8, z, …, 0xA, 0, −1) at the corpse.
   The 6→7→inert tail rides the controller (item 3).
6. **CORPUS CENSUS (read-only)** [verified]: exactly ELEVEN of
   the 37 .NME files host S8 records — ZONEE M1-5 (12/12/12/12/
   13), ZONEF M1-5 (9/9/9/9/19), ZONEG M1 (9) — 125 records →
   500 staged POIs (4 each), every file consumed byte-exact
   (orphan 0). The queue prose's "13 missions" is an arithmetic
   slip (5+5+1 = 11); ZONEE/ZONEF M6/M7 and ZONEG host none
   beyond M1. ZONEA/M1's S8 count is 0 → **landing S8 staging
   cannot move ANY canonical chain** (no canonical scenario
   stages personnel).
7. ENGINE CONSEQUENCE (landed this unit): stage_critters accepts
   section 8 → a `poi.rs` bank (PoiRecord: active/hp/state/
   heading/timer/exit/x/y/z/draw-word) staged with the exact
   three-draw schedule; the modeled controller subset = the
   per-frame prologue (z re-settle + the exit scan seam), the
   states 1/2/3 idle-settle-walk machine with the 1/16 lanes,
   the walker (free move + the quadrant ladder + the ≤4 gate),
   the 4-flee/5-escape lane over a host-staged 5-slot exit seam
   (active/phase/x/y/dwell — the §7j.19 family's controller-read
   subset; FUN_0041fa51's producer side stays unlanded), the
   escape award through the score-pending fold (+5000) + the
   escape counter + the panic cell, and the damage lane's state
   flip. The RandB sound pick, both SFX, the FUN_00420608 death
   effect, the MissionShell banner, and the animator (0x405186
   family — presentation, reads state/heading only) are the
   documented E-gaps. NOT hashed (the W6 split — the critter-bank
   precedent); no canonical chain movement.

## Plasma command-entry correction (2026-09-06)

The earlier sections 7j.17/7j.37 and their census summary mark the
w6/7/8 FUN_0040af98 body as an open AI-order family. Its normal
command entry is now decoded and implemented; docs/RE-EXW-PLASMA.md
is authoritative for this path. It emits type-5 shots (one aimed
shot plus 0/1/2 jittered extras), spends ammo per shot and uses
cooldown 2. The old placeholder's cooldown 8 and no-spawn behavior
were incorrect. The mask=-1 idle entry and sound gate remain open.
The type-5 draw in section 7j.28 starts with OLD counters 0,1,2
before the repeating 3..7 strip; increment occurs before clipping.
