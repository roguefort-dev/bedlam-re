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
