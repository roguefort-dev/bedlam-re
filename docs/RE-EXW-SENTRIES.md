# Pop-up sentries: production connection notes

2026-09-06, interactive continuation. Provenance: retained EXW disassembly
`ghidra-project/exw-text-objdump.txt`, decompilation
`exw-trtconsumers.txt` and `exw-trtcallees.txt`; no reference-project code.

## Live trigger

[observed] After crossing the toxic pool and PAD 3 lifts, the original
Boot Camp has a pop-up sentry on the striped pad beside the road and glass
structure. It animates and fires. The native runtime at 5b5ca79 only stages
TRT destruction records; it has no sentry animation/fire producer. The
original process is paused at this landmark for the implementation comparison.

## Corrections and implementation contract

[verified] Loader 0x4170a6 stages 0x20-byte records at 0x4cccf8:
active, state, frame, fire counter, HP, tile x/y/z. Initial state is 1,
frame/counter zero. It stamps DAT 0x66 and tile word 1. Word writer
0x417210 writes its argument **plus one**, including flash-frame arguments.
Preserve the separate seen byte when updating a word.

[verified] 0x417264 skips inactive records entirely. States 3/4 retract
after losing proximity; they are not a dead-record animation (corrects the
older section 7j.16 summary). Nearest alive robot uses octile distance,
strictly below 129; ties retain the first robot. State 1 opens through
frames 1..7, then state 6. States 5/6/7/8 aim negative Y/positive Y/positive
X/negative X, desired frames 11/7/9/13, with X winning equal axis distance.
The rotation wrap helper 0x417652 maps 15 to 7 and 6 to 14. Retraction
rotates toward frame 7 then lowers to 0/state 1. Exact branch order is
retained in `exw-trtconsumers.txt`.

[verified] 0x417698 scans the robot bank for a directional lane with
lateral distance below 40; this scan has no alive gate. Preserve its odd
height expression `abs((trt.z - ((robot.z >> 8) + 31)) >> 5) < 2`.
No lane resets the counter and writes the resting aiming frame. A lane
starts counter 1; odd counters allocate the first free enemy slot.
Flash arguments are counter plus 0x16/0x0e/0x12/0x1a for states 5/6/7/8,
then the counter increments and wraps 5 to 1. Full bank still animates.
Projectile kind 0x66 starts at tile x/y times 8192 plus 0xf00, z level
times 8192; velocity is axial +/-255 and the field called vz is 20.

[verified] The 0x66 handler at 0x412307 executes up to ten XY substeps
per call, **never changes z**, and treats vz as a terrain-arming countdown.
Bounds precede alive-robot occupancy (0x419756), then countdown decrement
or floor probe. Occupancy is strict per-axis Q5 distance below 16/16/32;
it disburses and clears the projectile with zero robot damage. Terrain
contact disburses, then damages objects with score flag zero and structures
using damage key 0x66. Damage is 300/600/1200 by difficulty (7j.50).

[verified] 0x412407/0x412409 unconditionally subtract one XY velocity
before write-back, even on ten substeps without collision: net movement
is nine velocities per call. Impact debris uses that reverted position;
terrain resolvers restore the contact position at 0x412444/0x41244e.
Bounds removal is silent. No TTL or visible projectile sprite is added.

[verified] Dispatcher entry 0x412010 ignores the incoming phase; the
loop at 0x448044..0x448066 calls it four times per mission frame. Thus the
older 7j.51 description of twenty substeps as approximately two frames
is inaccurate: it is two dispatcher calls. Sentry animation/fire ticks
once after debris, at 0x44807b. Newly fired shots first step next frame.

[open] Production implementation and renewed live comparison remain
required. This note does not establish a mission product gate or normalize
the live runs' different difficulty, ammunition and damage histories.
