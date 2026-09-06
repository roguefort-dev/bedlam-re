# Extraction beacon approach audit

2026-09-06, continuation after the native Boot Camp roof playthrough.

[observed] Native reaches the extraction roof naturally through PAD10, then
stops at Q5(528,816), z159 while targeting(568,816). The congratulations
hint appears and can be dismissed, but no extraction starts. Do not remove
collision or expand the trigger radius without original evidence.

[verified, read-only corpus] PAD16 is(17,25,4). Raw DAT at this tile has
level4=1 and levels5,6,7=1; its neighbors have level4=1 and upper levels0.
TOT words at levels4..7 are1c6,54a,54b,54c. POS slot96 is typec4 at
(17,25,5). Thus the solid beacon column is shipped data, not introduced by
production object staging. The current center is outside tile17 (x544..575).

[verified, EXW instructions] The walk dispatcher gate at40bd16..40bd58
requires robot state1/4 and order word other than-1. Boot Camp SP enters
433d5e via434045..43404d. It loads the robot's own x/y and z, shifts x/y
by8, and calls422e5e at433da1. That probe shifts all three coordinates
by5 (422e6e..74), requires raw DAT byteff (422e7d..8c), then scans active
PAD records for exact x/y/level equality (422e9e..ed4). It is not a
neighboring collision-probe lookup or a radius test.

[verified, EXW instructions] Slot16 reaches the armer: compare slot0xf
at433da6 (lower routes to433e5c, equal to433c8e), compare0x3e at433db5
routes16..61 to433e16, compare0x1e then0x11 routes exactly16 to433cbc.
The latter loads the robot coordinates and calls4247b5 at433cfb. This
supports the current Boot Camp extraction slot, not the old union of pads.
The loader at41ded0 stamps only the PAD level toff; it does not clear the
beacon's upper column. Nearby hint slots106..113 select message14 at
433d57 ->433d07 ->424a6f, rather than directly invoking the armer.

[open] Determine how the original makes PAD16 reachable: audit the original
terrain/probe handling and beacon object lifecycle, and replay the original
to the roof. No behavior change follows from the evidence above alone.
Original reference life previously ended under sentry fire; PID2400491 is
paused in the title intro. Native PID2373747 retains the blocked roof state.
The successful outcome resolver is also still incomplete, but must not be
used to paper over the unreachable trigger. Manifest checks passed around
these corpus/disassembly reads. No original bytes/assets are committed.

## Height-reader cross-check

[verified, EXW instructions] A second bounded audit rules out a special
PAD pass-through in the floor reader. At41eb3e..41eb49 the ordinary DAT
reader maps ff to type1. The floor routine calls it at41e274, reads the
type's CGR height at41e328..41e353, and, when that height is31, probes the
next plane at41e376..41e388. A nonempty upper type reads its own CGR
height at41e3c5..41e3f2; nonzero adds32 to the base at41e400. No PAD or
object-type exception occurs in this path. This agrees with the current
`Terrain::floor_z` behavior for the beacon column; changing ff to air
would contradict the original reader and affect every pad.

[observed] The native window still shows the robot beside the beacon on
the roof, with Plasma294, score33105 and cash3560. The original English
Boot Camp message14 instructs the player to reach the beacon; it does
not instruct the player to destroy it. This text alone does not establish
how collision becomes traversable. Next audit the mission initialization
and object/terrain mutations, and obtain the original roof observation.
No collision workaround or extraction completion was introduced. Corpus
manifest checks passed before and after the language-file read.

## Original roof comparison and restore-index defect

[observed, original EXD via DOSBox] Replayed STANDARD with purchased Plasma
X2, traversed the pool, green lifts, underpass, northern generator, PAD8
and northern teleporter. Reached the roof alive. The intact beacon stops
the robot in the original too. Firing Plasma at it collapses the tower,
awards40 points (180 ->220), and the pending movement enters the pad.
The dropship appears with the evacuation warning; subsequently the game
enters an evacuation movie. Thus an intact-beacon collision bypass would
be incorrect. Original PID2400491 is paused in that movie, not at title.

[verified, corpus] BDG row196 is1×1×3, hp400, score/type40. Its UNDER
TOT bank is(1331,0,0), UNDER DAT bank(0,0,0), at POS(17,25,5). The
restore must clear world levels5..7 while retaining PAD16 at level4.

[observed, native] Firing at the same tower awards40 (33105 ->33145),
but leaves its sprite and collision intact. Native remains(528,816,159),
HP2544, Plasma234. This isolates the defect to destruction restoration.

[verified, EXW] RE-EXW-SIM7j.32/3 already correctly specifies local
z'=z-z0 for bank indexing. Rechecked41aaf5 (local counter initialized0),
41ab1b (world z starts at instance z),41ab36..41ab55 (local z·H·W plus
footprint offset), and41ab59/72/8a (UNDER bank reads). The native
`destroy_tail` instead uses absolute `zz` in its bank index, then skips
indices beyond W·H·D. For z0=5,D=3 it skips every beacon cell. Correct
the bank index to use zz-oz, preserving world z for DAT/mirror writes.
Protect nonzero origins, multiple layers/footprint cells, upper-plane
clamping, and the actual Boot Camp beacon's movement/extraction seam.
Manifest checks passed around corpus reads; no original bytes committed.
