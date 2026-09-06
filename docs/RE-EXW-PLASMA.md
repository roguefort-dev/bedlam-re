# Plasma Cannon spawn path

2026-09-06, verified against ghidra-project/exw-text-objdump.txt;
canonical routine FUN_0040af98, 0x40af98..0x40b3be.

The normal command dispatcher routes weapon IDs 6/7/8 here with
extra-shot counts 0/1/2. These weapon IDs are not the spawned record
type: every emitted record has type 5, whose existing shell tick
resolves impacts with damage 75. The prior Rust Family placeholder
only spent ammo and emitted no record; missing rendering was not the
only cause of invisible, ineffective Plasma Cannon fire.

The first shot acquires a free slot (0x412848, first type-zero record
among 400). With a normal slot mask, decrement ammo, clear that mask
bit on zero, and set cooldown to 2 (0x40afb8..0x40b00d). No free slot
means no spending. Muzzle is raw robot x/y Q13 and `(robot.z+21)<<8`.
Aim denominator is max(1, dist_octagonal of rounded Q13 deltas / 8);
horizontal velocity is `(targetQ5-(position>>8))<<16 / denominator`,
signed division. A zero target z means vz=0; otherwise apply the same
division to target z minus muzzle z in Q5. If vx=vy=0, return after
spending without stamping a record (0x40b124..0x40b12a).

Record stores at 0x40b138..0x40b199 set type=5, owner, tick=0,
draw_counter=0, x/y/z, vx/vy/vz. Other fields are not cleared.
The optional extras repeat free-slot and positive-ammo gates, then
spend one ammo and set cooldown 2 per shot (0x40b23d..0x40b29e).
Each extra uses two RandA draws: target x/y plus `(rand&31)-16`
(0x40b2e4..0x40b30b), then the same velocity/stamp math. Degenerate
extra aim exits the burst after spending and drawing randomness.

The mask=-1 idle-fire entry omits bookkeeping; it is separate from
normal command fire and is not covered by the initial implementation.
Sound calls and their shared 0x4eb944 gate remain presentation work.
Type-5 drawing is the WEAPONS.BIN body at 0x404187; it requires a
separate renderer connection, not sprites for weapon IDs 6/7/8.

Draw cross-check (0x404187..0x404275): type 5 uses the OLD draw
counter as its frame, increments the stored counter before clipping,
and when the old value is >=7 draws 7 and stores 3. Thus a freshly
spawned zero counter draws 0,1,2,3,4,5,6,7,3,...; it does not start
at frame 3. Screen x is col-adjust + dx-dy + 0x110; y is row-adjust
+ shake + (dx+dy)/2 + 0x110 - (z>>8), with arithmetic shifts.
Clip x to 0..0x23f exclusive and y to 0..0x23e exclusive. Enqueue
world x/y in Q5, layer z>>13, WEAPONS bank, mode 0x12c.
