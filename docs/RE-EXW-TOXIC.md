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
