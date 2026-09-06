# Boot Camp teleport rides (2026-09-06)

[verified EXW] Rechecked RE-EXW-SIM 7j.21 against 0x433a14..0x433ac8
and 0x42034c..0x4204ea. Seven dormant records stage for zone 1, SP
mission 1: PAD 0 -> (8,57,2); PAD 10..14 -> (8,26,5); PAD 15 ->
(14,32,1). The markers are PAD coordinates. Real PAD 0 is (5,61,0),
PAD 10..14 are (2,1,1)/(7,1,0)/(12,1,0)/(17,1,0)/(21,1,0), and
PAD 15 is (16,52,0). Staging table source remains 0x425da4.

[verified EXW] Pad dispatcher requires a free rider (-1), writes robot
state 2, zeroes +0x74, stores ride index at +0x84, clears both movement
order arrays to -1, centers x/y on the marker (+0x1000 Q13), and arms
countdown 10. It leaves robot z unchanged at boarding. The scheduler
runs once per frame at 0x448076: decrement nonzero countdown; on zero
move to destination x/y <<13 (no center bias), z*32-1, then settle z
with 0x41e231, fill all eight probe heights, state 0, rider -1.

[verified EXW] Destination platform cleanup is CONDITIONAL on nonzero
strength. Clear strength and object-grid words, then scan all eight
layers in ascending order for the first water-range word and clear it
with 0x42394a(word=0,volume=0). With zero strength, skip cleanup.
The old prose saying an unconditional platform burn is too broad.
The countdown-10 marker sound and draw pass are separate presentation
work. Rendering uses TELEPORT.BIN, per 7j.21/7 and 7j.48.

[verified live original] From TELEPORTER tutorial, dismissed with a
movement click, then clicked the round floor disk. Original moved the
robot to the raised platform beside the red statue; ammo 962, score
560, cash 400 unchanged. This is the next native parity checkpoint.
