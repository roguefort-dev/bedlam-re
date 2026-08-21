# bedlam-re status - 21:4x 2026-08-21

- last commit: f04681d (assets: exact NME loader schedule + corpus test; docs 7f1c8fb) - close queue item 1 (P4 7j.18 critter/POI/exit loader hop: .NME grammar CLOSED, exit-pad activator pinned)
- phase: P4 (RE-first), corpus gates green, no sim behavior change this unit
- current queue head: the exit/escape runtime family (FUN_0041fbb1 + FUN_00433980) - see .state/NEXT.md

## last 5 commits
- f04681d assets(inspect): replace the heuristic NME walker with the EXW loader's exact 8-section schedule (7j.18)
- 7f1c8fb docs(sim): RE-EXW-SIM 7j.18 - the critter/POI/exit loader hop; .NME grammar CLOSED; FUN_0041fa51 = exit-pad activator; 7j.17 leftovers folded
- faf8c43 state: close queue item 1 (P4 7j.17 robot targeting/aim adopt, eaf16c0) - queue the critter/POI/exit loader head in FUN_00416458
- eaf16c0 docs(sim): RE-EXW-SIM 7j.17 - the robot targeting/aim family, adopted from three outage-killed runs
- 67d0d33 watchdog repair: classify provider HTTP 5xx deaths as transport (not client-error)

## unit summary (7j.18, worker a840f0af, claim 1)
- RE: FUN_00416458 = the .NME loader (8 fixed sections -> critter states
  2/1/5/4/3/6/7 + 4 POIs/record, corpus-exact 37/37); FORMATS §9 rewritten;
  FUN_0041fa51 = exit-pad activator; FUN_00449c94/FUN_0040db9e/0x4eb8b8 folded.
- Tooling: parse_nme exact schedule + corpus test (D66). fmt+clippy+tests green.
- Manifest verified before and after. Both commits pushed.
