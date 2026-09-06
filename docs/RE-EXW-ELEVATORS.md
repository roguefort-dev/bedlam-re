# Boot Camp pressure pads and moving tile stacks

## Live blocker and scope (2026-09-06)

[verified live] After the first teleport, original DOSBox reaches PRESSURE
PAD. Its text explains that red pads lower an elevator. Moving onto the
red pad in front of the long brown strip lowers that strip: the vertical
white/brown wall face disappears and the strip meets the platform floor.
Native at the same landmark retains the raised wall. Neither the native
pad dispatcher nor world staging owns this moving-tile family yet; only
static claim rectangles are staged. Native renderer has a private bias
array, initialized to zero, with no runtime producer.

## Script dispatch and initial records

[verified EXW] Zone 1, mission 1, mode != 2 is the active branch.
FUN_00433980 pad actions (EAX rect, EDX wanted, common call 0x438aef):

| PAD slot | Actions | Branch anchors |
| --- | --- | --- |
| 1 | rect 0 -> 2 | 0x434021/0x433acd |
| 2 | rect 1 -> 1 | 0x43402c..0x434033 |
| 3 | rect 2 -> 1, then rect 3 -> 2 | 0x433ad9..0x433af2 |
| 4..7 | rect 4 -> 2 | 0x434019/0x433e65 -> 0x438927 |
| 8 | rect 5 -> 1 | 0x433f44 -> 0x433af7 -> 0x4381b2 |
| 9 | rect 6 -> 2 | 0x433f4d -> 0x4392b7 |

[verified corpus] PAD 1=(5,53,1), 2=(10,46,1), 3=(15,37,1),
4..7=(5,34..31,0), 8=(2,14,1), 9=(18,13,0).

[verified EXW] Initial Boot Camp rectangles come from 0x42c4bc..0x42c65b.
The existing claim_rects::RECTS already has state/x/y/w/h. The missing
variant at record +0xA is {1,1,1,1,5,1,2}, records 0..6. All seven are
scripted (state 1 or 2); no auto-cycle branch is needed for this mission.
First rect: state 1, x=2,y=51,w=9,h=2,variant=1. The first pad selects
wanted 2, lowering those 18 tiles by one full level over 16 ticks.
Do not alter the pinned static claim table merely to add animation data.

## Runtime contract

[verified EXW] Re-anchored RE-EXW-SIM 7j.34 at 0x4223b8..0x4225cf.
The stepper returns when state already equals wanted or state >=3.
Only settled tiles (animation & 0x7f == target) are re-armed. Set
per-tile target to variant<<4 and animation to 0x80 for wanted 1,
or 0 for wanted 2; update rectangle state. It also requests a camera
cut and sound, both distinct presentation work.

[verified EXW, existing detailed decode 7j.34] Tick 0x423081 advances
per-tile packed animation each frame. Lowering uses odd DAT frame
bytes 0x5f-2*nibble; raising uses even bytes 0x40+2*nibble. At each
16-count boundary the finish pair updates DAT occupancy and shifts
all eight mirror words/seen bytes down or up one layer. Preserve the
original top-down occupied-level probes, neighbor-dependent bottom
clear on raising, and the conditional extra-plane writes. Do not
replace this with an instantaneous strip deletion or a generic wall toggle.

[verified native] MissionView::render_into already computes signed
pixel bias from the packed byte (nibble * +/-0x500 in its stride-640
buffer), but bias has no simulation connection. Add a checked route
for these per-tile bytes alongside the existing word/seen journal;
word-stack changes must reach the renderer without resetting LNK
animation on unrelated tiles. Core occupancy changes must be tested
as well as the visible strip: walking height depends on them.

Implementation completion: step on actual PAD 1 through ordinary input,
observe the 18-tile strip lower in native as in DOSBox, then walk across.
Test both directions and the five-level rect, in-flight command behavior,
frame boundary shifts, and unchanged unrelated cells. Other-zone auto
cycles, camera cue and sounds remain separate scope, not completion claims.

## DAT-index correction before implementation

[verified EXW] 0x4239ac/0x4239d5 index the DAT plane table at
0x4eaacc by the SAME z passed to the ordinary z writer. Therefore
0x4eaacc is plane 0, 0x4eaad0 plane 1, and 0x4eaae8 plane 7.
The old 7j.34 interpretation of 0x4eaae8 as a ninth plane was wrong:
0x4eaac8 is adjacent state, not the plane-table base. Lowering's finish
clears the highest occupied level when nonzero AND always plane 7.
Raising stamps one level above its highest occupied word. Native must
use these actual plane indices, not the earlier table-base gloss.
