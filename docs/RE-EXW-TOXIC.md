# Toxic-floor damage in the robot pass

[verified EXW, 2026-09-06] Re-read 0x40bbd2..0x40bc49 from BEDLAM.EXW.
After the robot/body phase gate, x/y Q13 values shift by 13 and index
row-table 0x4ea900 plus object-grid word 0x460dfa. This is a horizontal
tile test, with no z comparison. At 0x40bc1d, grid word 0x7d2 and phase
zero call FUN_0040e230(robot, 15, -1) at 0x40bc38. Nonzero phases skip
this call. The trap call follows at 0x40bc44, then the remaining robot
pass. The preceding 0x7d3 phase clamp is a separate behavior.

[verified native] WorldAssets already stamps 0x7d2/0x7d3 from the eight
mirror layers. MissionSim::robots_phase does not read those words, so
standing on a stamped toxic tile never causes the original 15/frame hit.
Use apply_damage to preserve shields, transit immunity, death and RNG;
materialize its death debris as the existing debris-hit caller does.
Do not add a walking-only or selected-robot-only gate.

[verified live, limited] Original and native entered the same Boot Camp
mottled pool after TOXIC WASTE. Native remained visibly alive indefinitely.
On the next observation the original had left the mission and was playing
a movie. The transition itself was not captured; death from standing in
the pool is an inference supported by the original warning and damage
code, not a timed live measurement. Full failure/debrief parity remains
unverified.

## Stamper correction found by the real-map regression

[verified EXW] 0x422f77/83 and 0x422fa3/af load raw zone 0x4edd8c,
shift it by two, and index tables without subtracting one. Raw dwords:
0x454a20: {0x20,0x49,0x49,0x34e,0x49,0x77,0x77,0x49};
0x454a3c: {0x49,0x4e,0x4e,0x349,0x4e,0x7c,0x7c,0x4e}.
Boot Camp therefore uses toxic base 0x49 and clamp base 0x4e. Existing
native tables incorrectly inserted a zero entry and shifted zone A.
The 0x7d3 table also had incorrectly transcribed values.

The upper checks at 0x422f98/0x422fc4 use JG, not JGE: ranges include
base+4 (five animation frames). For each layer, clamp stamps first,
toxic second; higher layers can overwrite lower ones. Correct all three
facts together. The regression initially found NO toxic tiles in the
actual Boot Camp map despite production-style staging, exposing the
producer defect independently of the missing damage consumer.
