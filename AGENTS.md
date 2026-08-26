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
- Unattended nudge agents MUST NOT spawn or delegate to subagents. No nesting; each claimed slot is one glm-5.3 session.
- Every commit advances substantive code or documentation; runtime state is machine-owned evidence.

## Required-task outcomes
- Keep the claimed task executable until its completion criteria pass.
- A pending machine event becomes `WAITING-AUTOMATIC` with an executable in-repo
  probe, retry cadence, and finite timeout or deadline.
- An unexpected inability leaves `.state/NEXT.md` unchanged; the worker wrapper
  emits `.state/automation-failures/<session>.json` for watchdog repair.
- Completion is checkable: substantive work is committed and the queue is valid,
  or a bounded probe is persisted and scheduled, or a structured failure exists.

## Ownership and shared-worktree rules
- For an unattended run, the wrapper has already acquired the queue-item claim named
  in its prompt. That claim belongs to the current worker. Do not interpret your own
  owner claim as a sibling claim.
- Process liveness is NEVER ownership evidence. The persistent Ghostty/cmux/operator
  OpenCode TUI is supervisory only: its PID, age, open terminal, dirty files, prior
  decision entries, and historical stand-down commits do not reserve queue work.
  `.state/PAUSE` is the operator explicit global reservation and the only such
  reservation currently supported.
- Relevant pre-existing WIP without `.state/PAUSE` is interrupted predecessor work:
  inspect and preserve it, then adopt, validate, and continue it. Before editing,
  record `git status`/`git diff`; never reset, checkout, clean, or overwrite it.
  Stage explicit task paths only--never `git add -A` or `git commit -a` in a dirty
  shared tree. Leave unrelated WIP untouched. Ambiguous overlap keeps the queue
  unchanged and produces the structured failure outcome above.
- Interactive operators must create `.state/PAUSE` before editing queue-relevant
  files and remove it when handing the work back. Merely keeping this TUI open never
  pauses or blocks autonomy.
- The old `21cbdcb` process-liveness ownership inference and all stand-down journals
  based on it are revoked. Do not recover that rule from git history.

## Workflow for every run
1. touch .state/heartbeat  (do this again periodically during long shell work)
2. If .state/PAUSE exists -> do nothing, exit. The sole exception is a watchdog repair launched by `tools/llm-watchdog.sh`: it may proceed only when its prompt supplies a watchdog token and both `.state/PAUSE` and `.state/llm-watchdog-pause` contain that exact token. It must leave both files untouched for the wrapper to release.
3. Read .state/NEXT.md. An unattended worker works ONLY its wrapper-assigned item
   and finishes with one Required-task outcome. Interactive runs may select another
   unclaimed item.
4. Do the work. Keep it small enough to finish and verify in this run.
5. Update docs as you go (provenance + confidence tags for RE claims; DECISIONS.md for choices).
6. git add (never game-data/derived), commit with a clear message, git push.
7. Rewrite .state/NEXT.md: mark the task done (with commit hash), queue the next tasks.
8. Update .state/STATE.md if the phase/status changed.
9. touch .state/heartbeat
10. Stop. The nudge system will spawn the next unit.

## Completion
Workers never assert global completion and `.state/PLAN-COMPLETE` is never trusted
as later-run input. When the active required queue is empty, the controller runs
the fixed bounded offline validator over `docs/required-gates.toml`; only zero
active items plus every P0-P7 gate green produces an informational, atomic,
HEAD/manifest-bound `plan-complete-v1` report for that invocation. A P4 verdict
may emit `.state/P4-COMPLETE`, never global PLAN completion.

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
- If a model, transport, or API error interrupts the task, preserve substantive
  WIP and leave the queue unchanged so the wrapper records structured evidence.

## Reverse-engineering discipline (stream-survival rule)
The API client dies after 300s of zero streamed bytes (known upstream bug). A model
call that thinks silently for >5 minutes is killed mid-thought. Therefore: any
reverse-engineering or analysis-heavy step MUST be split so that no single reasoning
stretch runs long - first decode a bounded piece, immediately write the findings as
a committed RE-notes artifact (docs/RE-EXW-*.md section or task notes), then proceed.
Implementation commits follow their RE notes. Prefer many small committed hops over
one long silent think. Emit interim notes/tool output while working rather than
reasoning in silence for minutes.
