# Mission failure after the selected robot dies

## Reproduced product defect

[verified live, 2026-09-06] Release 46fd941 was launched afresh through
New Game, Boot Camp, Armoury and a manually purchased Plasma Cannon X2.
The normal generator/teleporter/red-pad route reaches TOXIC WASTE. Entering
the pool now kills the robot; the robot disappears and its portrait becomes
static. The scene stays on the pool indefinitely. No failure movie or return
transition occurs. This verifies the toxic fix and exposes the missing
mission-owned failure path. The first Auto attempt chose unsupported
Needler weapons, so it was restarted; Auto is random, not a fixed loadout.

Original DOSBox previously left this same pool mission and displayed a
movie. The exact live death-to-movie boundary was not captured. The code
below identifies the movie and its timing gate independently.

## Failure detector, freshly re-anchored

[verified EXW] FUN_0044764c at 0x44764c..0x44770a:
- network mode 0x4edb88 must be zero (single player).
- Walk only the player squad, base 0x46cbd4 and count 0x46cbd8.
- Any record +0x9c death_flag == 0 returns zero. Do not substitute HP
  or alive: this is a distinct field, retained across MP respawn.
- All flags set still requires death-wipe cell 0x4ede34 == 480.
- Run cleanup, then, if movies enabled, play GAMEGFX/GAMEOVER.SMK
  (literal 0x459852, runner 0x44567c at 0x4476f8). Return one.
MissionShell caller 0x44870d is disabled after extraction completes and
maps this result to its return value three (SIM 7j.57).

## Wipe and survivor handling

[verified existing decode, SIM 7j.58] Selected robot SP death arms the
wipe at one. After presentation, 0x44809e adds 40, clamps at 480, and
walks squad slots to auto-select eligible live player-type survivors.
The walk does not break: last eligible slot wins. Selecting an alive
squadmate manually cancels the wipe. The source world frame freezes;
0x4012ba clears the destination and shrinks a centered square using
v=480-min(wipe,479), Q16 sampling. The terminal failure gate is twelve
increments after arming, not an immediate all-dead transition.

## Native work remaining

MissionScene::tick returns no outcome. RuntimeHost calls it then ticks
the FSM without inspecting squad death. SceneAction::MissionFail exists,
but only enters the generic Debrief placeholder; its default Advance
path leads to Shop. Do not silently reuse that placeholder as proof of
the original GAMEOVER lifecycle. Trace GameMain's handling of return
three and wire the actual movie/return route together with the wipe.
Keep simulation hashes unaffected by presentation timing. Tests must
cover survivor auto-selection, manual cancellation, death_flag semantics,
extraction-complete suppression, exact wipe boundary, and no campaign
completion award on failure. Then replay the toxic death in both windows.

## Host route decision

[verified existing GameMain decode, RE-EXW-GAMETHREAD] Switch on
MissionShell return minus one: return three enters case two, runs
0x41ca2e/0x447550, sets quit-current-game and goes to outer_restart.
Implement a distinct GameOver movie scene returning to Title, with no
Episode::complete call. The generic Debrief-to-Shop fail placeholder is
not this route. Post-game score-entry details remain separate work.
