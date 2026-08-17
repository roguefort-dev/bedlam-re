# AGENTS.md — Operating rules for any agent working in this repo

You are continuing the Bedlam (1996) decompilation + Rust reimplementation project.
This file is the contract for BOTH interactive sessions and unattended continuation
runs. Read it fully before doing anything.

## Doc map (read in this order when picking up work)
1. docs/PLAN.md - THE plan: canon (sec 0), parity budget (0b), 8street policy (1), phases (6)
2. .state/NEXT.md - the current task queue (what to do NOW)
3. .state/STATE.md - project state snapshot
4. docs/GROUNDWORK.md, docs/RESEARCH-8STREET.md, docs/FORMATS-MISSION.md, docs/RESEARCH.md - verified facts
5. docs/DECISIONS.md - decision log

## Hard safety rules (never violate)
- game-data/ is READ-ONLY. Before AND after any command that could touch files in it,
  run: sha256sum -c MANIFEST.sha256 --quiet  (fail loud, stop, restore from ~/Backups if ever wrong)
- Never commit game-data/, derived/, or any original asset content. git push only code/docs.
- 8street clones (~/Documents/bedlam-refs) are navigation references ONLY. Facts taken
  from them must be re-anchored to EXW/EXD addresses in spec docs. Never copy their code.
- Never run interactive/sudo commands. Never modify files outside the repo except
  .state/ logs, ghidra-project/, and /tmp scratch.
- One bounded work unit per run. Small commits. Push when green.

## Workflow for every run
1. touch .state/heartbeat  (do this again periodically during long shell work)
2. If .state/PAUSE exists -> do nothing, exit.
3. Read .state/NEXT.md. Take the top task. If it is blocked, note why in NEXT.md and take the next.
4. Do the work. Keep it small enough to finish and verify in this run.
5. Update docs as you go (provenance + confidence tags for RE claims; DECISIONS.md for choices).
6. git add (never game-data/derived), commit with a clear message, git push.
7. Rewrite .state/NEXT.md: mark the task done (with commit hash), queue the next tasks.
8. Update .state/STATE.md if the phase/status changed.
9. touch .state/heartbeat
10. Stop. The nudge system will spawn the next unit.

## Completion
When ALL gates of docs/PLAN.md (P0..P7) are genuinely passed, create the file
.state/PLAN-COMPLETE (with a summary inside) instead of queuing more work. That file
stops the autonomous loop permanently.

## Build/test baseline
- cargo build --release / cargo run --release -q -- game-data derived  (tools/inspect)
- cargo fmt, cargo clippy before committing Rust
- Manifest check after any corpus-touching run

## Ghidra discipline (added after 2026-08-17 03:25 incident)
- NEVER launch `analyzeHeadless -import` if `pgrep -f analyzeHeadless` shows one
  running, or if the target log already contains `Import succeeded` for that
  binary. Duplicate imports stack programs in the project and waste 15 minutes.
- To work on an already-imported binary, use `-process <programname>` with
  `-noanalysis` and a postScript; do not re-import.
- BedlamWatcom project status: BEDLAM.EXW imported ONCE under
  x86:LE:32:default + openwatcomcpp cspec, single program, verified 03:33.
- If a model/transport/API error interrupts you mid-task: stop, record exactly
  what you finished and the blocker in .state/NEXT.md, commit that much, stop.
  Never leave silent partial state for the next agent to trip over.
