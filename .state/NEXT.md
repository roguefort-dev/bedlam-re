# NEXT - task queue (top first; rewrite this file at end of every run)

QUEUE CONVENTION (2026-08-22, D106): a completed unit's entry MOVES to
the '## Done' log at end of run - never stays in '## Now' as 'N. DONE ...'
(the scheduler mechanically skips a first-word DONE marker, but the
renumbered queue keeps every open item claimable by number).
AUTHORING RULE (D180, second recurrence after D177): every
[status]/[id]/[gate]/[probe]/[retry] metadata tag stays WHOLE on the
item's first numbered line, prose starting same-line after the tags —
never wrap INSIDE a tag; the strict parser rejects it (rc=2,
INVALID-DEADLOCKED) and the worker dies at its own finish line.
## Now

1. [READY] [id=mission-room-journey-v1] [gate=mission-room-journey-v1] ORIGINAL MISSION ROOM AND ARMOURY — replace the blank Brief/Select click-through with the illustrated mission-selection room, Boot Camp selection and equipment screen through ShellController. Anchor behavior and assets to EXW specs and compare the live EXD reference sequence in docs/PLAYTEST-2026-09-06.md. The selected loadout must reach Mission naturally.
   ACCEPTANCE: actual window play from New Game through Boot Camp region, Armoury, equipment choice and DONE into the mission; controller input regression over the read-only corpus; original STANDARD Auto loadout cross-checked with EXW before pinning values; no SceneAction injection; fmt/clippy and affected tests green, existing canonical controls unchanged unless separately justified by EXW evidence; manifest clean around corpus reads. Missing mission outcomes and tutorial behavior remain separate work.

2. [READY] [id=menu-journey-v1] [gate=menu-journey-v1] MENU JOURNEY PRODUCT GATE — the FIRST gate classified evidence="product" in docs/required-gates.toml, wired under P6 using the completed production input seam and shared controller: a real menu journey — boot to title, into options, toggling a setting, back out, the quit path — driven through the ShellController over production scenes with observable assertions and no injected actions.
   ACCEPTANCE: new named test menu_journey_gate green in the bedlam-shell battery; wiring the product gate flips no phase green by itself (the validator v2 suite stays green, proving the gate stays rejectable should its evidence ever regress to synthetic); controls green; MANIFEST clean around corpus reads.

3. [READY] [id=mission-ground-input-v1] [gate=mission-ground-input-v1] ORDINARY GROUND MOVEMENT — the original Boot Camp robot walks to ground clicks and follows the player camera; the current MissionScene::click_robot ignores empty ground. Re-anchor the original viewport-to-world target and selected-unit order behavior to EXW, wire ordinary movement through production input, and exercise it in the real window against DOSBox.
   ACCEPTANCE: production controller regression clicks empty reachable ground and observes selected-unit movement; camera and sidebar behavior compared with original; overlay/sidebar clicks retain their own dispatch; canonical controls remain pinned or any evidence-driven change is explicitly reviewed and documented; fmt/clippy and affected tests green; manifest clean. Record tutorial-trigger behavior as a concrete next task if not covered by this bounded movement unit.

4. [READY] [id=mission-outcome-v1] [gate=mission-outcome-v1] MISSION-OWNED OUTCOME RESOLVER — MissionScene::tick in engine/bedlam-game/src/mission.rs returns an outcome owned by the mission scene itself; the shell maps it to the debrief transition; the headless harness stops injecting SceneAction::MissionComplete while the production input seam continues to forbid that injection.
   D263 LIVE: white objective rows now match original B2 runway. Both games reach southwest green gate; original has active enemies, native none. Next priority: production NME staging plus critter rendering together (core accepts all8 sections), before further mission-completion comparison. Native PID2718912 paused(575,2579,31), HP5000; original PID2400491 paused matching junction after abort/restart with ammo600/cash2750. Original Escape from MAP returned to room and discarded prior weapon; EXW audit pending. See latest PLAYTEST section.
   D262 LIVE: rebuilt full STANDARD Boot Camp-to-B2 run verifies blue fence bars at matching original runway spawn. Native PID2688217 paused B2 frame120, original PID2400491 still at spawn. White objective markers remain absent: six B2 groups at EXW table0x454f80 verified in RE-EXW-MISSIONVIEW.md; objective staging/destruction/completion is the next gameplay gap.
   LIVE D261: full STANDARD window play now extracts from Boot Camp, plays ZONEDONE without skip, returns to Zone B room and launches B-2 retaining ammo420/cash3000/score3070. Native PID2650028 and original PID2400491 both paused at B-2 runway. Original ammo420/cash3250/score10220; random gold-award parity remains open, not a proven completion bonus. Missing dropship art and native B-2 radar targets remain visible gaps, alongside NME staging and other-zone objectives.
   PROGRESS D261: selected Boot Camp extraction now recaptures live inventory/cash/score and returns through one-pass ZONEDONE/loading to a fresh Zone B room. Real-corpus controller boundary test launches B-2 retaining state, with declared near-beacon position fixture. Game200/shell150 and production clippy pass; canonical actuals unchanged, still3 pass/10 fail. Fresh window replay, other-zone objective verdicts and harness replacement remain open; no product completion claimed.
   PROGRESS D260: MissionScene::tick returns Running/ExtractionComplete/Failed; host consumes Failed. Scene regression waits through actual craft phases and reports extraction only after departure. Game198/shell149 tests and production clippy pass; canonical actuals unchanged from D259 (still3 pass/10 fail). Successful objective classification, campaign/loadout recapture, Boot Camp debrief bypass, movie/room handoff and harness replacement remain open.
   ACCEPTANCE: cargo test --release --locked --offline -p bedlam-game green with new outcome tests; zone_mission_parity, canonical_dump_gate, and determinism green with ZERO canonical-chain movement — outcome resolution must not perturb replay hashes; cargo fmt and clippy clean; MANIFEST clean around corpus reads.

5. [READY] [id=zonea-trace-v1] [gate=zonea-trace-v1] ZONE A PRODUCTION TRACE PRODUCT GATE — a natural production-path trace of ZONEA-MISSION1 from mission start through the mission-owned outcome into debrief, driven end to end through the ShellController with real input over the read-only corpus; wired as the second product gate and cited as the product evidence that may later flip ZONEA-MISSION1 green in docs/P5-MISSION-LEDGER.toml.
   ACCEPTANCE: new named test zonea_production_trace green; the ledger checker green with the row still unproven unless the trace meets the full v2 product bar in the same commit that flips it; the validator v2 suites green; MANIFEST.sha256 clean before and after every corpus read.
   PROGRESS 2026-09-06: radar and its hold legend compared live with DOSBox; production destructible-world loading and simulation/render tile propagation connected under D247. Plasma Cannon now emits visible damaging shots, compared live with the original. Plasma impact bursts now render and were compared live with the original, with dispatch ordering anchored at EXW 0x410cad. Generator shutdown is now anchored in docs/RE-EXW-FENCE.md: type 0x85 at (16,63,1) schedules linked types 0x7f/0x84; its timer and origin-only shutdown are now implemented and compared live with DOSBox (D250). The real-corpus damage/frame regression verifies all 13 linked segments. Camera cut, spoken cue and other-set mappings remain open. False extraction at welcome/fence tutorial pads fixed under D251; native can continue to LASER FENCE. The first TELEPORTER ride now reaches the same raised platform in original and native (D252), and native movement resumes afterward. Seven Boot Camp rides and their boarding effect are wired; other zone tables and sound remain open. PAD 1 elevator now implemented under D253: all eighteen tiles lower, and both native and DOSBox robots crossed the strip into the scaffold area. Seven scripted Boot Camp rectangles, packed animation frames and DAT/word/seen stack shifts are connected; camera/SFX and other zones remain open. Continued both games to the blue-pad and TOXIC WASTE tutorials. Actual PAD 2 raising and a neighboring-platform approach now pass with the real SINTABLE movement table; live blue-lift activation subsequently verified in both games (see latest playtest section). D254 corrects toxic hazard tables/ranges and phase-zero damage; actual-map test proves idle death at frame 334. Fresh 46fd941 live replay now confirms toxic death. D255 wires the twelve-increment death wipe, survivor selection and squad failure into GAMEOVER.SMK and Title; fresh native toxic death now leaves the mission and returns to Title. Successful extraction/high-score entry remain open. Resume fresh original/native play toward the safe pool crossing and subsequent pads; D256 trace now verifies native blue raising. Both original and native have crossed the toxic pool alive using the western approach to the exit ramp, activated green PAD 3, and reached the joined brown platform. Native is at (532,1200,z63) with stable hp530; original is at the same landmark. Elevator 4/4 and toxic 2/2 regressions pass, including the twenty-cell dual-lift crossing. Continued both to the glass-structure road; original pop-up sentry is active, native lacked its producer. D257 now implements TRT animation/fire and the instruction-anchored 0x66 substep handler; five core tests and an actual-map road-sentry regression pass. Rebuilt native PID2151744 completed the natural route on STANDARD, reached the road sentry and visibly verified opening, firing, retraction and glass-structure destruction against the live DOSBox reference. Native is alive at (352,1028,z31), hp1300; original PID2106527 is paused beside the broken glass structure. Secondary K8 impact debris does damage and knockback despite zero direct projectile damage; RE notes corrected and a two-tick damage regression added. Both games subsequently reached HIDDEN PADS and lowered the five-level scaffold section, then entered its passage. Actual-map five-level stack/crossing regression passes (elevator tests 5/5). Native POWER UPS tutorial observed; now at (88,1104,z31), hp1296, obstructed among scaffold pillars/gold structures. Original is paused inside the same scaffold area. Original collected the first gold structure and gained10 cash, exposing native subtract-one pickup indexing. D258 corrects raw set1..7 A/B/floor lookups; actual-map gold collection/movement regression passes. Rebuilt native D258 has now naturally reached and collected the gold row: PID2373747 at (108,1140,z31), hp2832, cash3030, score6015, Plasma X2 ammo418; trace /tmp/bedlam-pickup-live2.log. Collection and collision clearing are live-verified. Original PID2106527 exited; scratch reference has now been replayed through the matching player-selected Plasma X2 purchase (500 cash/600 rounds), and new PID2400491 has reached the toxic-pool entrance after the generator, first teleporter and blue lift. It has now crossed the pool, activated green lifts, passed the firing road sentry and lowered scaffold, and observed POWER-UPS inside the passage. SIGSTOP-paused after collecting gold, Plasma580, cash3010, score22015. Use relative pointer exec session64458 for movement, then brief gameplay clicks; one-second holds overshoot pads. The live gold comparison is restored. Native centered at x112 to pass the narrow pillar gaps and now stands at (108,1600,z31), hp2832, Plasma418, cash3410, score33015. The obstruction was an offset approach; no collision change needed. New actual-map regression protects gold clearance through rows47/50 while keeping pillars solid. Main route now established in native: return via (108,1036), east to (416,1036), north under the archway to (416,640,z31), HP2832, Plasma418, cash3410, score33015. Northern type86 generator/fence shutdown live-observed; western ramp and blue PAD8 four-cell raising/crossing work. PAD10 teleports to extraction roof (256,832,z159). Native now stops at (528,816,z159), target(568,816), after the congratulations tutorial, HP2684, Plasma294, cash3560, score33105. D259 resolves the beacon restoration defect: original also blocks the intact tower, but shooting it collapses it and starts extraction. Native destruction indexed UNDER banks by world z instead of local z-z0. Fixed with 27 destroy and 1 actual-map extraction tests passing; the latter reaches extraction complete. Rebuilt normal-input window replay now verifies both generators collapse, beacon destruction clears collision and robot enters extraction state5 at (544,800,z159), hp2806, score70. Five further seconds remain in Mission: successful host outcome and dropship presentation are still missing. PID2584686 paused there. EXW successful Boot Camp return is1 via0x4481f4, not4; caller/movie audit is next. Original PID2400491 has now naturally completed Boot Camp extraction, played the evacuation movie and returned to the mission room; it is paused there. Later progress supersedes the older original death/checkpoint descriptions below. Original reached the matching extraction/archway tutorial but died under sentry fire before crossing; PID2400491 is paused in the title intro, not a resumable mission checkpoint. Original underpass traversal remains unverified. Award RNG parity remains open. Ammo/episode pickup bodies remain incomplete now that the correct indexing reaches them; exact combat timing/trajectory parity and ramp-edge catches remain open. See latest playtest section for locations and evidence. --trace-gameplay provides read-only input/position/height/health/target/lift evidence. Canonical S3 now ends early in GameOver and reports a typed capture error rather than a missing-mission panic. Canonical S5/S5b/S5c newly differ and S7 actual changed as documented; pins remain unchanged. Audit other zones' extraction unions and account for the documented S6 digest change. Object targeting, other weapons and the mission outcome remain unproven. See docs/PLAYTEST-2026-09-06.md for the reproducible live locations and checks.

6. [READY] [id=autonomy-suite-rot-v1] [gate=autonomy-suite-rot-v1] AUTONOMY SUITE ROT REPAIR — the pre-existing red cases recorded in D240, proven at HEAD b7af042 in an isolated worktree with the synthesis change absent: the test-autonomy-remaining-gaps completion-validation case and six test-reviewer-security-red completion and validator cases still write required-gates-v1 fixtures that the D238 validator refuses outright, so both suites stay red for a reason no current unit owns; move the fixtures to required-gates-v2 with honest evidence classifications while preserving each case's pinned adversarial property.
   ACCEPTANCE: both suites PASS clean end to end and stay PASS under the simulated worker environment pinned by the D240 hermetic guards; no pinned property weakened — fixtures change schema, assertions keep their teeth; tools/test-validate-required-gates.py stays green; bash -n and py_compile clean.


## Backlog

Canonical dump drift: seven corpus assertions fail identically before and after
the D247 world connection, verified against isolated HEAD 2238fbe. Diagnose
the earlier drift without blindly replacing digests; actual values and logs
are recorded in docs/PLAYTEST-2026-09-06.md. No product gate is green on this basis.

## Done
1. DONE (2026-09-06, interactive, commit 39e66fb): shell-input-seam-v1 and shell-controller-v1 share ShellController between window and smoke, accept typed ProductionInput, isolate synthetic actions in controller::harness, and stage before the next input. Real-input journey and failed-entry tests pass; compile-fail tests reject fabricated completion and mutable host access. Shell 149 passed/one ignored, doc tests 2 passed, fmt/clippy and canonical controls green; historical smoke scene/frame/audio unchanged. Rebuilt window reaches Mission and MAP works. This is engineering progress, not a product-complete journey.
2. DONE (2026-09-06, interactive, commits 0b9e96d and 1510eac): live original/remaster comparison repaired window-to-mission pointer alignment and restored original title artwork/palette. Original reference reached Boot Camp through mission room and Auto equipment; ground movement and tutorial panel observed. Missing mission room, equipment and ground movement are queued above. No mission/phase completion claimed.
1. DONE (2026-09-02, claim 1, slot c5b582e3 — worker c5b582e3 was interrupted
   provider-side by a rate limit, rc=137 progress=0, with the work complete
   on disk but uncommitted; the WIP was adopted, verified, and finished by
   watchdog repair 1204180, commit 1faa0f8, PUSHED, plus this
   bookkeeping commit): SCHEDULER `queue-synthesis-v1` — DETERMINISTIC
   FAILED-PRODUCT-GATE SYNTHESIS, THE CONTROLLER HOOK (D240): on a
   REQUIRED-QUEUE-EMPTY parse whose sealed full-battery validation failed,
   the completion branch of tools/nudge.sh now calls the new
   tools/nudge-state.py synthesize-product-work, which publishes READY
   items synthesized ONLY from product-class failures — a wired
   evidence="product" gate whose own evidence ran and is red (the item
   cites the gate id, the first red command's argv when grammar-safe and
   lint-clean else argv withheld, and the exit code; command-less failing
   gates are dependency consequences and never synthesize) and a phase
   wiring zero product gates (a synth-wire wiring item) — while red
   non-product gates, dependency-blocked product gates without a red
   product root, error-shaped validator/sandbox/corpus/harness reports,
   stale HEAD bindings, phase-run reports, non-empty queues, and claim
   residue all refuse WITHOUT touching the queue (byte-identical, reason
   logged, fall-through to the structured completion-missing beacon
   exactly as before). Publication is fail-closed: SAFE_ID identities,
   strict-grammar validation of the candidate queue BEFORE replace_publish
   (queue-locked), a BOUNDS line on every synthesized item — no
   synthesized item ever asserts a phase status, only the validator flips
   phases — and the item is ordinary claimable work on the next tick, the
   designated mechanical source for the later 37-mission product backlog.
   THE HERMETIC ENV FIX CLASS (found live by the interrupted worker,
   finished by the repair): suites run from inside a nudge worker session
   inherited NUDGE_OWNER_FD/NUDGE_CLAIM_IDENTITY so the agent under test
   skipped its claim-owner-exec re-exec and died at launch preflight
   claim-invalid — test-nudge-claims, test-waiting-automatic, and
   test-nudge-controller strip the wrapper vars, and the repair extends
   the identical guard to test-lock-v2-adversarial,
   test-automation-failure-watchdog, test-final-hardening-red,
   test-autonomy-remaining-gaps, and test-reviewer-security-red, each
   reproduced red under a simulated worker environment before the guard
   and restored to its clean-env verdict after it
   (test-nudge-transport-markers verified green in the real worker
   context). VERIFIED first-hand: tools/test-queue-synthesis.py 15/15;
   tools/test-nudge-controller.sh PASS end-to-end including the new
   live-hook tests 13 and 14; tools/test-nudge-claims.sh,
   tools/test-waiting-automatic.sh, and tools/test-nudge-queue.sh PASS;
   tools/test-validate-required-gates.py 34/34; the five guarded suites at
   their clean-env verdicts under simulation; py_compile and bash -n
   clean; no engine change, no manifest edit, no corpus read. The
   pre-existing D238-era fixture rot in test-autonomy-remaining-gaps and
   test-reviewer-security-red is recorded and queued as item 6, NOT fixed
   here.

1. DONE (2026-09-02, claim 1, slot 9c719592 — commit 2ef3678 by worker
   9c719592, PUSHED, plus this bookkeeping commit): SCHEDULER
   FOUNDATION `scheduler-gate-cache-v1` — THE PER-GATE FINGERPRINT
   VERDICT CACHE (D239), the D238 follow-up that ends the full-battery
   re-runs: tools/validate-required-gates.py gains an OPT-IN
   --gate-cache DIR accelerator (absent flag = byte-identical behavior
   for every existing caller; NO gate command changes, no engine
   change, no manifest edit). The per-gate basis fingerprint binds the
   HEAD commit, the whole tracked-tree fingerprint, the required-gates
   manifest sha256, the MANIFEST.sha256 corpus digest, the validator
   bytes, and the gate's own slice (commands, tracked-path +
   command-script digests, timeout, evidence, writable, depends); any
   tracked change anywhere re-runs by design, a cached green is reused
   only on an exact basis+id+schema match, verdicts are keyed by
   sha256(gate id) under the cache dir, only greens of executed gates
   are remembered (dependency failures never consult or write), and
   corrupt/foreign/oversized/symlinked entries fail closed to a re-run
   and are replaced honestly. In-root cache paths must be gitignored +
   untracked + symlink-free (loud refusal otherwise; canonical repo
   path /gate-cache, gitignored), out-of-root host paths allowed for
   the sealed-controller shape. A hit replays green command verdicts
   byte-identically, so reports stay deterministic across miss-to-hit.
   VERIFIED first-hand: new hermetic suite tools/test-gate-cache.py
   15/15 (deterministic hit, miss-remembers-green, basis-change
   re-runs on gate-input AND unrelated tracked changes, corrupt +
   oversized fail-closed-to-re-run, six poison shapes refused,
   symlinked entry refused, dirty tracked path rejected despite cache,
   dependency failure writes nothing and stays red, phase-run reuse,
   out-of-root host cache, disabled-by-default never writes or reads,
   the three in-root path refusals); tools/test-validate-required-
   gates.py 34/34 unchanged; py_compile clean; MANIFEST.sha256 clean
   before and after (hermetic fixtures, no corpus read); REAL-REPO
   bounded --phase P7 smoke at the landing commit: cold run 84.7s
   executing all 7 gates green with 7 entries remembered, warm run
   1.0s all-hits with entries untouched and reports byte-identical
   (an 84x collapse on P7 alone; the full 37-gate battery scales the
   same). Queue head is now queue-synthesis-v1 (the controller hook
   for deterministic failed-product-gate synthesis), with the
   shell-input-seam / shell-controller / menu-journey /
   mission-outcome / zonea-trace units behind it.

1. DONE (2026-09-02, no queue claim — slice 1 required-gates-v2-contract ordered by .state/AUTONOMY-PAUSE-CHECKPOINT.md under its exact pause token; substantive commit f3f9ad8 plus this bookkeeping commit, both PUSHED): THE AUDITED FALSE-COMPLETION REVOCATION (DECISIONS.md D238) — docs/required-gates.toml moves to schema required-gates-v2: every one of the 37 gates carries a validated evidence classification (all non-product: supporting 8, static 9, paperwork 9, corpus-required 8, synthetic 1, infrastructure 2; commands byte-identical), phase statuses are vocabulary-checked, a green phase is structurally rejected without a wired product gate, plan completion requires every phase product-green, and the revoked v1 schema is refused outright — the old manifest can never validate again. P0-P6 green->pending; P7 green->engineering-green; bounded --phase runs now emit phase-verdict-v2 with product_complete false, so the legacy .state/P4..P7-COMPLETE markers and old reports are non-authoritative residue (left on disk, nothing reads them, pinned by test). docs/P5-MISSION-LEDGER.toml moves to p5-mission-ledger-v2: all 37 rows unproven with the v1 parity evidence preserved per-row in supporting_evidence citing p5-zone-a..g; the checker decouples gate wiring from zone green (citation linkage instead) and demands product evidence for any future green row; the real-repo test pin re-baselined 0/37 green, 37 unproven, same commit. VERIFIED first-hand: test-validate-required-gates 31/31, test-p5-zone-ledger 22/22, check-p5-zone-ledger OK over the read-only corpus, check-p6-behavior-catalog OK + 30/30, check-p7-ports-map OK + 29/29, test-p6-hd-asset-research 27/27, py_compile clean, git diff --check clean, MANIFEST clean before and after; no gate battery run, no engine change, no corpus write. QUEUE SEEDED (above): seven dependency-ordered machine tracer units — the per-gate fingerprint cache first (ends the ~30min/~12GB re-runs), then failed-product-gate synthesis (the designated source for the later 37-mission product backlog), the input seam, the ShellController, the menu-journey product gate, the mission outcome resolver, and the Zone A production trace. Dependencies are encoded by queue order under the pinned concurrency ceiling of one and lowest-ordinal claiming — the controller spawns the first unclaimed READY item only — so no WAITING-AUTOMATIC probe is required: none of the seven waits on an external machine event. With the queue nonempty the controller's completion branch cannot fire at all (it runs only on REQUIRED-QUEUE-EMPTY), so the intentionally-red product state drives item 1 normally and can never beacon completion-missing into watchdog-repair churn.
1. DONE (2026-08-28, claim 1 — commit 97fb49e by worker
   78919433, PUSHED, plus this bookkeeping commit): P7
   PHASE-CLOSE BOOKKEEPING `p7-phase-close` — the SURVEYED VERDICT
   + the STATUS FLIP + the BOUND VERDICT ARTIFACT (D231; the
   d01a7b7 P6 / f608207 P5 / 972748d P4 phase-close pattern).
   (a) THE SURVEY (DECISIONS.md D231, carried by the flip commit
   97fb49e): every PLAN §6 P7 sentence walked against the
   p7-ports-map-v1 registry — Linux native + Flatpak (rows
   linux-native D222 + flatpak-manifest D225), Windows installer
   (D227), macOS universal2 through automated CI (D229), the
   external-conditions sentence honored as the JOIN DISCIPLINE
   (the three recorded exclusions macos-runner-availability /
   signing-keys / publication-stores — R8 rows carrying a note,
   never a gate; re-verified: none of the seven P7 gate blocks
   requires a store, a key, or a runner — no corpus key, no
   writable, no credential anywhere), CI artifacts per push
   (D222), the CDDA user-supply sentence (D223), SteamDeck
   defaults stretch (D224), plus the §12 milestone gate "3-OS
   artifacts" (the same three OS rows) — every sentence
   gate-green landed or EXPLICITLY excluded, nothing silently
   dropped; P8 recorded OUTSIDE the required-gates authority
   (the manifest enumerates exactly P0-P7; PLAN §6 P8 stays
   future work per the plan's own ordering). (b) THE FLIP:
   docs/required-gates.toml P7 status pending->green — P0-P7
   ALL GREEN for the first time; a bounded phase run still
   forces plan_complete false exactly as designed and the GLOBAL
   verdict stays the controller's alone. (c) THE BOUND VERDICT
   re-emitted AT the flip commit with the exact P4/P5/P6-shaped
   command: /usr/bin/python3 tools/validate-required-gates.py
   --root . --report .state/p7-phaseclose-gates-report.json
   --phase P7 --phase-output .state/P7-COMPLETE — status=passed,
   ALL 7 P7 GATES GREEN, every command rc=0 under bwrap
   containment, .state/P7-COMPLETE phase-complete-v1 re-bound to
   97fb49e (producer required-gates-validator, emitted by the
   validator itself). VERIFIED first-hand: check-p7-ports-map OK
   before AND after the flip (7 engineering, 7 landed, 0 pending +
   3 recorded exclusions — the R6 rule satisfied: green with zero
   unfinished engineering); test-p7-ports-map 29/29 and
   test-validate-required-gates 22/22 before and after the
   manifest edit; the survey anchors re-checked in-tree
   (packaging/dev.roguefort.bedlam.yml + .desktop,
   packaging/bedlam-shell.nsi + windows-installer-README.txt,
   .github/workflows/macos-universal2.yml + the ci.yml artifact /
   flatpak / windows-installer jobs, engine/bedlam-shell/src/
   cdda.rs, engine/bedlam-shell/src/platform.rs + the Stretch arm
   in engine/bedlam-platform/src/scale.rs); the tracked tree
   stayed clean through the whole HEAD-bound battery (STATE.md /
   NEXT.md edits parked until after the verdict); MANIFEST clean
   before and after (no P7 gate reads the corpus; no Ghidra run,
   no engine change, no gate-command change, no registry edit —
   the registry already stood 7 landed / 0 pending at 9437ac7).
   THE REQUIRED QUEUE IS NOW EMPTY: every P0-P7 phase is green in
   the manifest and no required item remains; the controller's
   fixed bounded offline validation over docs/required-gates.toml
   owns every global completion claim from here (workers never
   assert it — the completion contract).
   NOTE (watchdog repair 1787945546, D232): the recorded
   rate-limit failure for this run was a CLASSIFIER FALSE
   POSITIVE, not a provider quota death — the client exited rc=0
   progress=1 with the final summary fully streamed, the flip
   97fb49e and the bookkeeping 89905f3 both PUSHED, and the queue
   above already emptied (strict parser REQUIRED-QUEUE-EMPTY).
   The old rate-limit grep's bare `rate limit` substring matched
   the D230 watch-item prose quoted in this worker's own
   transcript (the D230 watch item, fired). Fixed in the repair
   commit (error-shaped markers only + the extended live-extracted
   regression suite); the structured failure is adjudicated
   required-empty per D232 — the resulting required queue IS empty
   and stays empty.
   NOTE (watchdog repair 1787949148, D233): the completion-missing
   failure recorded at 22:02:27Z was NOT a gates failure — the
   controller's terminal completion validation was KILLED BY ITS
   OWN WRAPPER CAP: complete-from-head ran the sealed HEAD
   validator under a flat 1800s subprocess timeout while the
   validator's own bounded contract for this manifest is the sum
   of len(commands) x timeout_seconds = 82680s across the 37
   gates (every command of a gate runs with the gate's declared
   timeout_seconds; cold per-command cargo targets make hours of
   wall time legitimate). Two attempts died at exactly 1800s
   (f56pann5 at 22:02:26, vasjoy4_ at 22:52:27) each recording
   completion-missing — pure churn, the loop could never emit
   plan-complete-v1. Fixed in the repair commit: the wrapper
   budget is now DERIVED from the sealed manifest (1800s floor +
   the declared per-command bound, 84480s at this HEAD), so the
   outer cap can never truncate a legitimately bounded run; the
   per-gate bounds, containment, and fail-closed semantics are
   untouched. The structured failure is adjudicated required-empty
   per D233 — the required queue IS empty and stays empty; the
   controller's next complete-from-head run owns the global
   verdict exactly as the completion contract demands.

   NOTE (watchdog repair 1787956349, D235): the completion-missing
   failure recorded at 23:52:20Z is the ONE-LAST MARKER D234's
   POSTCONDITION explicitly predicted — the sealed validation that
   rejected then started 23:22:27 against a89ce5a under a PRE-FIX
   parent controller (the D234 benign classifier did not exist on
   disk before 23:44:15), so when the D234 repair commit cff1b9b
   itself landed mid-run the run ended basis-changed and the old
   code beaconed. NO CODE CHANGE was needed or made this repair:
   the D233 derived budget and the D234 classifier stand as landed,
   re-verified first-hand (tools/test-nudge-controller.sh PASS
   end-to-end 67s including the deterministic race test 11; the
   live controller tick of 00:22:27 sealed i00g7bng at HEAD cff1b9b
   under POST-FIX code and runs now). The marker is adjudicated
   required-empty per D235 — the required queue IS empty and stays
   empty, P0-P7 all green, strict parser rc=0. This repair commit
   will itself end the in-flight i00g7bng run basis-changed —
   BENIGNLY now, log + exit 0 — and the next tick re-validates
   this HEAD from scratch; the global verdict stays the
   controller's alone.

   NOTE (watchdog repair 1787962939, D236): the completion-missing
   failure recorded at 2026-08-29T00:22:17Z is a NEW defect class,
   neither a D234 tail nor a classifier miss: the sealed run
   completed well inside its manifest-derived budget and was
   rejected with HEAD required-gates validator failed rc=1, and
   the published report names the single failing gate —
   gates-validator, whose command is the validator suite itself,
   tools/test-validate-required-gates.py, rc=1 (p4-machine-verdict
   fell with it as a pure dependency consequence; the other 35
   gates green). The suite is FLAKY, two independent intermittent
   defects, both reproduced first-hand this repair and both fixed
   in the repair commit: (a) the reap test tied the escaped
   descendant's touch deadline to the kill deadline (sleep 1
   against timeout_seconds=1) — under the sealed runs' real load
   (12.4G RSS peak, ~4 saturated cores, swap) the awakened touch
   can beat the kill path (Python timeout raise, killpg SIGTERM,
   the 50ms TERM-to-KILL spacing, namespace teardown), so the
   sentinel appears and the suite fails; the touch now sits at
   sleep 5 with a fail-fast poll — seconds of scheduling margin,
   the pinned property unchanged: a reaped descendant must never
   touch. (b) the sealed-root test's read-only walk raced git's
   DETACHED auto-maintenance, which briefly creates and unlinks
   git objects maintenance.lock after the fixture commit —
   reproduced 2-of-7 runs as FileNotFoundError at the walk's
   stat; fixtures now set maintenance.auto false and the seal and
   unseal walks skip paths that vanish beneath them (a vanished
   file needs no seal; skipping is the correct walk, not a
   weaker one). Verified: 10-of-10 suite runs green including 4
   under 24 busy-loop CPU hogs, tools/test-nudge-controller.sh
   PASS end-to-end, strict parser rc=0 REQUIRED-QUEUE-EMPTY
   before and after this NOTE, manifest clean before and after.
   The marker is adjudicated required-empty per D236 — the
   required queue IS empty and stays empty, P0-P7 all green; the
   next controller tick re-validates this HEAD from scratch and,
   with the gate suite deterministic, the flake class that
   beacons completion-missing is closed.

   NOTE (watchdog repair 1788037173, D237): the completion-missing
   failure recorded at 2026-08-29T20:59:32Z is a NEW defect
   class, the first HOST-ENVIRONMENT failure of the completion
   era: the machine rebooted at 22:57 (tmpfs /tmp wiped) and the
   first post-boot controller pass died BEFORE any gate ran —
   complete-from-head's mkdtemp over /tmp/opencode raised ENOENT
   because nothing recreated the completion staging root after
   the wipe; the controller had been healthy all day (13
   consecutive accepted validations 13:11:52-22:30:44). The
   staging root is controller-owned infrastructure, so the
   controller now creates it itself: completion_scratch_base in
   tools/nudge-state.py recreates the root 0o700, idempotently,
   and refuses a symlink or non-directory root; both host-side
   completion call sites (complete-from-head mkdtemp,
   accept-completion TemporaryDirectory) stage through it; the
   sealed validator needs nothing (its scratch_base already
   falls back to a HOME-based root). The controller suite gained
   deterministic test 12 pinning the defect class (mkdtemp into
   a wiped root ENOENTs) and proving recreate/idempotent/refuse
   semantics, and test 11's fixture now mkdir -p's the shared
   root so the suite itself is reboot-proof. Verified:
   tools/test-nudge-controller.sh PASS end-to-end, test-12 body
   green standalone, py_compile and bash -n clean, strict parser
   rc=0 REQUIRED-QUEUE-EMPTY before and after this NOTE, MANIFEST
   clean before and after, required-gates manifest untouched.
   The marker is adjudicated required-empty per D237 (the sixth,
   after D232/D233/D234/D235/D236) — the required queue IS empty
   and stays empty, P0-P7 all green at this HEAD; the next
   controller tick re-validates this HEAD from scratch and the
   reboot-wipe class that beacons completion-missing is closed.

2. DONE (2026-08-28, claim 1 — commit 9437ac7 by worker
   c60dbcd6, PUSHED, plus this bookkeeping commit): P7 SEVENTH +
   LAST engineering deliverable `p7-macos-universal2-ci` — the
   SCHEDULED macOS UNIVERSAL2 CI JOB DEFINITION per PLAN §6 P7
   "macOS universal2 through automated CI" + docs/P7-PORTS.md §2
   (row macos-universal2-ci; implementation D229; the registry
   row flipped landed in the SAME commit naming the new SEVENTH
   P7 required gate per the R2 rule). (a) THE DEFINITION
   (.github/workflows/macos-universal2.yml, NEW): the scheduled
   macos-universal2 job on macos-latest — dtolnay/rust-toolchain
   @stable with BOTH targets (aarch64-apple-darwin,
   x86_64-apple-darwin), the two reproducible slice builds
   (cargo build --release --locked -p bedlam-shell --target
   aarch64-apple-darwin + --target x86_64-apple-darwin,
   deliberately not --offline), the UNIVERSAL2 JOIN (lipo
   -create over exactly the two built binaries into
   staging/bedlam-shell — ONE Mach-O carrying both slices), the
   strict bounded upload (bedlam-shell-macos-universal2,
   actions/upload-artifact@v4, if-no-files-found: error, 14-day
   retention) — engine binary only, UNSIGNED (the signing-keys
   exclusion), the corpus token ABSENT from the workflow
   entirely. THE CADENCE is PLAN §3's own posture ("automated
   scheduled macOS CI when a runner is available ... goldens
   never run on macOS CI"): weekly off-peak cron "41 4 * * 1" +
   workflow_dispatch and NO push/pull_request trigger — no push
   is ever gated on a macOS runner existing (the
   macos-runner-availability exclusion made mechanical; the
   per-push artifact surface stays the Linux + Windows ci.yml
   matrix); NO test/golden/diffharness command rides along;
   least privilege permissions contents: read. The runner itself
   is EXTERNAL: the first live execution may need ordinary CI
   fixes — exactly the exclusion's content; the gate grades the
   committed definition only. (b) THE GATE: p7-macos-universal2-
   ci wired as the SEVENTH P7 required_gates entry — command 1 =
   tools/check-p7-macos-universal2-ci.py (the D222-family
   stdlib-only YAML-subset checker over the committed workflow:
   the scheduled cadence incl. the push-trigger refusal, the
   macos-* runner label, the both-targets toolchain, the two
   exact --release --locked builds, the lipo -create join over
   exactly the two built binaries, the strict bounded upload,
   the no-test boundary, least privilege, and the signing-token
   + corpus-token denylists, comments included) + command 2 =
   check-p7-ports-map (the flip + gate join) + command 3 =
   tools/test-p7-macos-universal2-ci.py (35 fail-closed tests);
   test-p7-ports-map.py re-baselined to 7 landed / 0 pending
   (the flip fixtures now UN-land the macos row; the default
   fixture manifest wires the seventh gate; the forward shape
   becomes the GREEN PHASE itself), the D222-D227 pattern.
   VERIFIED first-hand: the checker + 35/35 suite green over the
   real repo; ports-map OK (7 engineering, 7 landed, 0 pending)
   + 29/29; test-validate-required-gates 22/22 after the
   manifest edit; p7-ci-artifacts + 22/22, p7-flatpak-manifest +
   40/40 and p7-windows-installer + 50/50 still green over the
   untouched ci.yml; the workflow re-parsed under pyyaml as an
   independent check of the family reader; controls green BEFORE
   AND AFTER (canonical_dump_gate 13/13, determinism 4/4,
   zone_mission_parity 5/5 — ZERO canonical-chain movement;
   check-p6-behavior-catalog OK both sides); MANIFEST.sha256
   clean before and after every corpus read (the gate reads no
   corpus); the bounded --phase P7 validator verdict RE-EMITTED
   at the landing commit 9437ac7: status=passed, ALL 7 P7 GATES
   GREEN, every command rc=0 under bwrap containment (report
   .state/p7-macosuniversal2-gates-report.json, head-bound to
   9437ac750224). No engine change, no corpus read by the gate,
   no new dependency, no Ghidra run, no new RE. Queued: the P7
   phase close as the SOLE remaining item (all seven engineering
   rows landed; the close = the R6 survey + the pending->green
   status flip + the bound verdict with --phase-output
   .state/P7-COMPLETE).
   NOTE (watchdog repair 1787944462, D230): the recorded
   transport failure for this run was a CLASSIFIER FALSE
   POSITIVE, not a provider death — the client exited rc=0
   progress=1 with the summary fully streamed (D226/D228
   misread the same shape); the wrapper's bare `DNS` marker
   matched the prose "reverse DNS" in the transcript. Fixed in
   the repair commit (error-shaped markers only + regression
   suite); the structured failure is adjudicated replaced-task
   per D206 — work stood complete and PUSHED, and the active
   queue above is untouched, RUNNABLE 1.

2. DONE (2026-08-28, claim 1 — commit 07a6c57 by worker
   a6aece66, PUSHED): P7 SIXTH engineering deliverable
   `p7-windows-installer` — the WINDOWS INSTALLER DEFINITION +
   ITS PER-PUSH CI BUILD per PLAN §6 P7 "Windows installer" +
   docs/P7-PORTS.md §2 (row windows-installer; implementation
   D227; the registry row flipped landed in the SAME commit
   naming the new SIXTH P7 required gate per the R2 rule). (a)
   THE DEFINITION (packaging/bedlam-shell.nsi, NEW): the
   committed NSIS script — Name "Bedlam engine", OutFile
   bedlam-shell-setup.exe, Unicode true, InstallDir
   $PROGRAMFILES64\Bedlam with RequestExecutionLevel admin +
   CRCCheck force, the minimal page flow Page directory +
   Page instfiles (uninstaller UninstPage uninstConfirm +
   instfiles — NSIS semantics re-verified FIRST-HAND against the
   manual: $PROGRAMFILES64, $OUTDIR stored as a CreateShortcut's
   working directory, the un. uninstaller-section prefix; the
   makensis script-dir-vs-cwd path ambiguity is DESIGNED AWAY —
   the CI runs makensis with working-directory: packaging == the
   script's own directory, so every relative path resolves the
   same under either rule); exactly two sections, both pinned
   instruction-for-instruction: the INSTALL body = SetOutPath
   $INSTDIR; exactly TWO File sources, both STAGED BARE NAMES
   (the engine binary + windows-installer-README.txt — the
   grammar forbids paths/wildcards, so the closed engine-only
   file set is structural); WriteUninstaller; the
   Add/Remove-Programs registration (HKLM
   ...\Uninstall\BedlamEngine DisplayName + UninstallString);
   CreateDirectory $SMPROGRAMS\Bedlam; ONE CreateShortcut onto
   the installed engine whose working directory is $INSTDIR (the
   engine's documented default lookup root sits directly inside
   the install folder; the README spells out the INSTALL_DIR
   positional too); the UNINSTALL body = the exact inverse
   (every Delete names an installed artifact — the checker
   refuses any other; the ARP key removed; RMDir on EMPTY
   directories only, the recursive switch cannot even parse). No
   Icon anywhere (no asset ever, D21). (b) THE README
   (packaging/windows-installer-README.txt, NEW): honest user
   documentation dropped next to the binary — engine-only
   boundary + supply-your-own + the documented default layout;
   the corpus token may appear ONLY inside the exact phrase
   game-data\BEDLAM (checker-enforced). (c) THE CI BUILD
   (ci.yml job windows-installer, per push on windows-latest):
   checkout + the matrix's own dtolnay toolchain; cargo build
   --release --locked -p bedlam-shell (deliberately not
   --offline); choco install nsis; the staging Copy-Item
   target\release\bedlam-shell.exe -> packaging\bedlam-shell.exe;
   makensis at ${env:ProgramFiles(x86)}\NSIS\makensis.exe with
   working-directory: packaging on THIS script; the UNSIGNED
   installer uploaded as bedlam-shell-windows-installer-x86_64
   (upload-artifact@v4, if-no-files-found: error, 14-day
   retention); Authenticode stays the signing-keys exclusion —
   the 8-token denylist enforced across script + README + the
   job, comments included, and the corpus token absent from
   script + job entirely. (d) THE GATE:
   tools/check-p7-windows-installer.py (hermetic stdlib-only
   CLOSED NSIS COMMAND GRAMMAR — unknown commands, plug-ins,
   compiler directives, labels, C-style comments, line
   continuations, unbalanced/quoted-vs-bare argument shape,
   wildcards, path separators in File sources, Delete/RMDir
   switches are all parse errors) + check-p7-ports-map (the
   flip + join) + tools/test-p7-windows-installer.py (50
   fail-closed tests); test-p7-ports-map.py re-baselined to
   6 landed / 1 pending (the canonical pending row for flip
   fixtures is now macos-universal2-ci), the D222-D225 pattern.
   VERIFIED first-hand: the checker + 50/50 suite green over the
   real repo; ports-map OK (7 engineering, 6 landed, 1 pending) +
   29/29; gates-validator 22/22 after the manifest edit;
   p7-ci-artifacts checker + 22/22 suite and p7-flatpak-manifest
   checker + 40/40 suite still green over the edited ci.yml (the
   new job carries no os-matrix so the release-matrix job stays
   unique); ci.yml re-parsed under pyyaml as an independent
   check; controls green BEFORE AND AFTER (canonical_dump_gate
   13/13, determinism 4/4, zone_mission_parity 5/5 — ZERO
   canonical-chain movement; check-p6-behavior-catalog OK both
   sides); MANIFEST.sha256 clean before and after every corpus
   read; the 5-gate baseline evidence carried over content-valid
   (only .state/DECISIONS changed since the e5474b8 verdict) and
   the bounded --phase P7 validator verdict RE-EMITTED at the
   landing commit 07a6c57: status=passed, ALL 6 P7 GATES GREEN,
   every command rc=0 under bwrap containment (report
   .state/p7-windowsinstaller-gates-report.json, head-bound to
   07a6c57774dda8d6). No engine change, no corpus read by the
   gate, no new dependency, no Ghidra run, no new RE. Queued: the
   macOS universal2 CI job definition as the new head (the LAST
   pending engineering row), then the P7 phase close.
   NOTE (watchdog repair 1787943179, D228): the worker's model
   connection died provider-side (transport, rc=0, progress=1)
   AFTER this completion rewrite and its printed final summary —
   a post-completion transport death, no work lost (07a6c57 +
   the bookkeeping ee5c5a7 both PUSHED, strict parser rc=0 on the
   rewritten queue); the structured transport failure was
   adjudicated replaced-task per the D206 checklist (all four
   items green, D211/D226/D228) and the active queue above
   stands untouched, RUNNABLE 1 2.

2. DONE (2026-08-28, claim 1 — commit e5474b8 by worker
   3ea06ba4, PUSHED): P7 FIFTH engineering deliverable
   `p7-flatpak-manifest` — the FLATPAK BUILD MANIFEST + ITS CI
   BUILD DEFINITION per PLAN §6 P7 "Linux native + Flatpak" +
   docs/P7-PORTS.md §2 (row flatpak-manifest; implementation D225;
   the registry row flipped landed in the SAME commit naming the
   new FIFTH P7 required gate per the R2 rule). (a) THE MANIFEST
   (packaging/dev.roguefort.bedlam.yml, NEW): app-id
   dev.roguefort.bedlam (the repo remote's own reverse DNS,
   checker-joined to the file stems + the CI build-bundle command
   word), org.freedesktop.Platform + Sdk at the PINNED
   runtime-version 24.08, command bedlam-shell, the CLOSED
   five-token finish-args surface (--socket=wayland,
   --socket=fallback-x11, --socket=pulseaudio, --device=dri,
   --share=ipc — no host filesystem grant, no network, no bus);
   ONE simple-build module (cargo build --release --locked -p
   bedlam-shell under the rust-stable extension, deliberately not
   --offline) installing exactly one binary + one desktop entry
   into /app; the single dir source at the repo root carries the
   checker-pinned NEVER-BUNDLE skip floor (.git, game-data,
   game-data-2, derived, derived-2, goldens, ghidra-project,
   target) — nothing from the corpus or its derivatives ever
   enters the copy. (b) THE DESKTOP ENTRY
   (packaging/dev.roguefort.bedlam.desktop, NEW): Exec == the
   command, Terminal=false, Categories=Game, NO Icon (no asset
   ever, D21). (c) THE CI BUILD (ci.yml job flatpak, per push on
   ubuntu-latest): flatpak-builder + the SAME pinned SDK//24.08 +
   rust-stable Extension (the version join), builds THIS manifest
   with build/repo dirs outside the checkout, exports the
   UNSIGNED bundle bedlam-shell.flatpak naming the app-id, uploads
   bedlam-shell-flatpak-x86_64 with if-no-files-found: error +
   14-day retention; signing-token denylist enforced across
   manifest + desktop + the job, comments included. (d) THE GATE:
   tools/check-p7-flatpak-manifest.py (hermetic stdlib YAML-subset
   schema/shape/join checker) + check-p7-ports-map (the flip) +
   tools/test-p7-flatpak-manifest.py (40 fail-closed tests);
   test-p7-ports-map.py re-baselined to 5 landed / 2 pending (the
   canonical pending row for flip fixtures is now
   windows-installer), the D222-D224 pattern. VERIFIED: baseline
   --phase P7 green at 12c118b (4 gates) BEFORE, the same verdict
   re-emitted at the landing commit e5474b8 (ALL 5 P7 GATES
   GREEN under bwrap), MANIFEST.sha256 clean before and after,
   test-validate-required-gates 22/22, p7-ci-artifacts + suite
   still green over the edited ci.yml.
   NOTE (watchdog repair 1787941613, D226): the worker's model
   connection died provider-side (transport, rc=0, progress=1)
   AFTER this completion rewrite and its printed final summary —
   a post-completion transport death, no work lost (e5474b8 +
   the bookkeeping df93006 both PUSHED, strict parser rc=0 on the
   rewritten queue); the structured transport failure was
   adjudicated replaced-task per the D206 checklist (all four
   items green, D211/D226) and the active queue above stands
   untouched, RUNNABLE 1 2 3.

3. DONE (2026-08-28, claim 1 — commit 0daf3a7 by worker 3d906dad,
   PUSHED, plus this bookkeeping commit): P7 FOURTH engineering
   deliverable `p7-steamdeck-default` — the STEAMDECK
   PLATFORM-PROFILE DEFAULT per PLAN §6 P7 "SteamDeck defaults
   stretch" + docs/P7-PORTS.md §5 (the D221 contract, row
   steamdeck-default; implementation D224; the registry row
   flipped landed in the SAME commit naming the new FOURTH P7
   required gate per the R2 rule). (a) THE PROFILE
   (engine/bedlam-shell/src/platform.rs, NEW; bedlam-shell only):
   the SteamDeck class identified ONCE at window startup from the
   DMI sysfs identity — /sys/devices/virtual/dmi/id board_vendor
   "Valve" AND product_name "Jupiter" (the 1280x800 LCD deck) or
   "Galileo" (the 1280x800 OLED deck), trimmed + case-insensitive,
   BOTH fields required, FAIL-CLOSED to Generic on any other
   identity, missing files or a non-sysfs platform (the env is
   deliberately never consulted: STEAMDECK=1 is a Steam-session
   fact, not hardware); read-only, best-effort, never fatal; the
   identification mechanism is RECORDED in the registry row's note
   per the §5 contract. (b) THE ARM — the contract's second
   branch, recorded: the EXPLICIT ASPECT-DISTORTING STRETCH arm
   landed as a fourth ScaleMode in bedlam-platform scale.rs (the
   WHOLE frame onto the WHOLE target — no bars, no crop; Fill was
   NOT chosen: its centered crop hides the top and bottom of the
   game's own 480 rows); CLI word "stretch" joins the fail-closed
   domain integer|fit|fill|stretch; on the SteamDeck class the
   default PresentConfig scale becomes Stretch with one stderr
   note (the --scale override hint); generic platforms keep
   Integer + Nearest bit-for-bit (PresentConfig::default()
   untouched — the D215 pin
   scaling_defaults_to_the_shipped_integer_nearest stays green);
   the explicit --scale ALWAYS wins; the filter default stays
   Nearest on every platform (the contract overrides the scale arm
   only). (c) PARITY BOUNDS pinned by test (D200): the profile is
   OUT of ModeConfig, both pacing arms accept it identically
   (pinned by profile_selection_never_changes_the_gate_answers),
   and it selects NOTHING in the sim — identical sim config,
   executed ticks, tick count, state hash, scene hash AND frame
   parity hash under every class x CLI-word combination (pinned by
   profile_selection_never_touches_the_sim_or_the_hashed_
   trajectory); the palette expansion stays VgaExpand::Original;
   the headless path never probes DMI (flags noted + ignored
   exactly as before). (d) THE GATE: p7-steamdeck-default wired as
   the FOURTH P7 required_gates entry — command 1 = the hermetic
   bedlam-shell --lib battery (145/0 + 1 pre-existing ignored;
   +7 platform tests: the Valve+Jupiter/Galileo identification
   incl. casing/whitespace variance, every fail-closed shape incl.
   missing/empty fields + near-miss products, the best-effort DMI
   reader over a real scratch dir, the per-class defaults, the
   CLI-wins rule over the full 2x4 domain, the fill-the-panel
   geometry on 1280x800 (Stretch whole frame + whole panel,
   Integer the 320 px pillarbox bars the contract forbids, Fill
   the crop not chosen), the only-the-default pin; +2 window
   invariance tests; the D215 suite extended 3x2 -> 4x2 in place;
   +1 bedlam-platform integration test: the Stretch geometry
   pins); command 2 = check-p7-ports-map (the flip + join). No
   corpus key, no writable, no network, no device, no display.
   (e) THE FLIP (same commit): §5 the LANDED paragraph; §6 the
   landed-since note; the ports-map suite re-baselined
   deliberately (real-repo pin 4 landed / 3 pending + the landed
   line; the honest fixture wiring p7-steamdeck-default; the
   forward-shape test flipping flatpak-manifest at 5 landed /
   2 pending) — 29/29. Verified first-hand: fmt + clippy clean on
   both touched crates (the one pre-existing D210 test warning
   untouched); the binary wiring first-hand (help text incl. the
   stretch word, the bogus-word + missing-value rejections at
   exit 2, the headless ignore note) AND the headless smoke
   EXACTLY at the recorded baseline (scene 696adb1cd110e062 /
   parity cce30c983b97b16d / audio 110400/158092) under --scale
   stretch; the WINDOW host end to end on the live display under
   --scale stretch (exit 0; no profile note on this Generic
   desktop — the note fires only on the deck class, pinned by the
   pure tests); controls green: canonical_dump_gate 13/13,
   determinism 4/4, zone_mission_parity 5/5 (ZERO canonical-chain
   movement), check-p6-behavior-catalog OK before AND after,
   test-validate-required-gates 22/22 after the manifest edit;
   MANIFEST clean before and after every corpus read; the bounded
   --phase P7 validator verdict at 0daf3a7: status=passed, ALL 4
   P7 GATES GREEN, every command rc=0 under bwrap containment
   (report .state/p7-steamdeck-gates-report.json, head-bound to
   0daf3a7d8811). Queued: the Flatpak build manifest + CI
   definition as the new head (the next pending registry row in
   contract order), then the Windows installer, universal2, and
   the P7 phase close.

4. DONE (2026-08-28, claim 1 — commit 1dfd775 by worker d9aaa029,
   PUSHED, plus this bookkeeping commit): P7 SECOND ENGINEERING
   deliverable `p7-cdda-user-supply` — the CDDA USER-SUPPLY +
   LOCAL-CACHE surface per PLAN §6 P7 "CDDA: user-supplied original
   tracks (WAV/CD), optional local lossy cache generated on first
   run — never redistributed" + docs/P7-PORTS.md §4 (implementation
   D223; the D221 registry row cdda-user-supply, flipped landed in
   the SAME commit naming the new THIRD P7 required gate per the
   R2 rule). (a) THE LOOKUP (engine/bedlam-shell/src/cdda.rs, NEW;
   bedlam-shell only, no engine change): the 7 CDDA tracks of the
   mixed-mode CD (CD tracks 02..08, corpus shape VERIFIED
   GROUNDWORK.md; the WAV header shape re-read first-hand) resolved
   over the ordered roots (--music-dir DIR / BEDLAM_MUSIC_DIR env,
   then $XDG_DATA_HOME/bedlam/music, then the install dir),
   candidate names BEDLAM0N.WAV then TRACK0N.WAV
   case-insensitively, first match in root order; SILENT MISS = one
   stderr note, music silent, never fatal, never a task
   (resolve_supply never fails). (b) THE OPTIONAL LOCAL LOSSY
   CACHE: whole-track IMA ADPCM (the standard tables, per-channel
   coder state, nibble-packed: a REAL lossy codec at exactly 4:1,
   chosen as a dependency-free integer-math transcode — no new
   crate, cargo --offline stays green) into the USER-OWNED cache
   home ($XDG_CACHE_HOME/bedlam | ~/.cache/bedlam |
   %LOCALAPPDATA%/bedlam/cache), <cache>/music/trackNN.bcda with a
   43-byte header carrying the SOURCE IDENTITY (length + FNV-1a-64
   streamed), regenerated on mismatch (write-then-rename; corrupt
   or unparseable entries regenerate, malformed sources skip with
   a per-track reason — never fatal); --no-music-cache opts out
   (default ON = generated on first run); startup REFUSES a cache
   home inside the install tree (game-data stays read-only; both
   sides best-effort absolutized so the binary's RELATIVE default
   install dir still compares — caught first-hand) or inside any
   git work tree (.git at the root or any ancestor — never the
   repo); the cache is NEVER redistributed (the D21 rule applied
   to audio). (c) PARITY BOUNDS: CddaOptions rides
   WindowOptions::music, a PLATFORM knob OUT of ModeConfig (D200,
   D17 b, the D212 posture) that never reaches the sim config or
   any hash (pinned by cdda_surface_never_touches_the_sim_config);
   the headless path owns no surface (the binary notes + ignores
   the flags), and the smoke ran EXACTLY at the recorded baseline
   (scene 696adb1cd110e062 / parity cce30c983b97b16d / audio
   110400/158092) under the new flags. (d) THE GATE:
   p7-cdda-user-supply wired as the THIRD P7 required_gates entry —
   command 1 = the hermetic bedlam-shell --lib battery (136/0 +
   1 pre-existing ignored; +19 cdda tests: the numbering/names,
   the case-insensitive priority-ordered lookup, the silent-miss
   wording, the WAV parser incl. odd-chunk padding + every
   fail-closed shape, the ADPCM pins (silence exact, 4:1 size,
   bounded roundtrip, held-value settling), the FNV identity
   (streamed vs one-shot + known values), the blob round-trip +
   verdict-on-identity, the end-to-end cache
   generate/fresh/regenerate/corrupt cycle, the skip-with-reason
   posture, the component-wise containment + git-worktree +
   relative-install guards); command 2 = check-p7-ports-map (the
   flip + join). No corpus key, no writable (temp fixtures ride
   the validator's TMPDIR target bind). (e) THE FLIP (same
   commit): the registry row landed with the note rewritten to
   what shipped; §4 gained the LANDED paragraph; §6 the
   landed-since note; the ports-map suite re-baselined
   deliberately (real-repo pin 3 landed / 4 pending + the landed
   line; the forward-shape test 4/3; the not-in-phase-list
   fixture wiring the cdda gate) — 29/29. Verified first-hand:
   fmt + clippy clean (the one pre-existing D210 warning
   untouched); the binary --help/--music-dir/--no-music-cache
   wiring first-hand (help text, the missing-value rejection at
   exit 2, the headless ignore note); the WINDOW host end to end
   on the live display — 7/7 via --music-dir with the cache
   generating 7 entries (43-byte header + exactly 1/4 of the PCM
   bytes), a second run all FRESH, a modified source regenerating
   EXACTLY its own entry, the empty-override fall-through finding
   the corpus rips via the install-dir root, BOTH refusal guards
   firing with their notes, --no-music-cache disabling; controls
   green: canonical_dump_gate 13/13, zone_mission_parity 5/5,
   determinism green (ZERO canonical-chain movement),
   check-p6-behavior-catalog OK before AND after,
   test-validate-required-gates 22/22 after the manifest edit;
   MANIFEST clean before and after every corpus read (one
   deliberate guard-probe env var made mesa write its own cache
   under game-data — removed, manifest re-verified clean); the
   bounded --phase P7 validator verdict at 1dfd775: status=passed,
   ALL 3 P7 GATES GREEN, every command rc=0 under bwrap
   containment (report .state/p7-cdda-gates-report.json, head-bound
   to 1dfd77534cab). Queued: the SteamDeck platform-profile default
   as the new head (the next registry row in contract order), then
   Flatpak, the Windows installer, universal2 and the P7 phase
   close.

5. DONE (2026-08-28, claim 1 — commit af9cac1 by worker cf6544eb,
   PUSHED, plus this bookkeeping commit): P7 first engineering
   deliverable `p7-ci-artifacts` — the PER-PUSH CI ARTIFACT JOBS per
   PLAN §6 P7 "CI artifacts per push" + docs/P7-PORTS.md §2/§3
   (implementation D222; the D221 registry rows ci-artifacts-per-push
   + linux-native). (a) THE WORKFLOW (.github/workflows/ci.yml): two
   actions/upload-artifact@v4 steps inside the EXISTING build matrix —
   every push uploads the release binary from each leg (ubuntu-latest
   -> target/release/bedlam-shell as artifact bedlam-shell-linux-x86_64,
   THE linux-native deliverable; windows-latest ->
   target/release/bedlam-shell.exe as bedlam-shell-windows-x86_64), each
   gated on its runner.os, each if-no-files-found: error (a missing
   binary fails the build, never an empty artifact), retention-days 14
   (bounded so per-push artifacts do not accumulate at the 90-day
   default); the artifact is the ENGINE BINARY ONLY (never game-data,
   never assets, nothing corpus-derived) and UNSIGNED — no credential,
   no store, no runner dependency (the D221 signing-keys exclusion; the
   macOS leg joins with macos-universal2-ci when a runner exists); a
   top-level permissions: contents: read joins the file (least
   privilege, the frame-pacing.yml pattern). (b) THE GATE:
   p7-ci-artifacts wired as the SECOND P7 required_gates entry behind
   the scaffold — command 1 = tools/check-p7-ci-artifacts.py, the
   fail-closed offline checker over the COMMITTED workflow definition:
   it parses ci.yml with a STDLIB-ONLY YAML-SUBSET READER (the D216
   no-deps family posture; tabs in indentation, unterminated flow
   sequences, unparsable lines and trailing content are all parse
   errors — the file that ships is the file that is graded) and proves
   the four contracted properties: PER-PUSH TRIGGER (top-level
   on.push), THE RELEASE MATRIX (exactly one job running cargo build
   --release on BOTH ubuntu-latest + windows-latest), THE UPLOADS
   (both steps live in that job, action pinned @v4, non-empty names,
   exact binary paths, if-no-files-found: error), and NO SIGNING
   MATERIAL (8 denylisted credential/code-signing tokens — secrets,
   signtool, codesign, notarytool, notariz*, osslsigncode,
   authenticode, gpg — matched case-insensitively anywhere in the file,
   comments included); command 2 re-runs check-p7-ports-map (the
   registry flip + gate join); command 3 = the 22-case hermetic
   fail-closed suite (every rule proven to fail loudly incl. the
   push-trigger removal, the matrix-leg removal, the build-step
   removal, both upload-step removals, the WRONG-JOB upload refusal,
   the @v3 downgrade, the gating removal, the path/name/if-no-files
   tampers, the secrets/signtool/comment injections, and four parse
   failures; plus the minimal-synthetic pass and the real-repo pin).
   (c) THE CONTRACT FLIP (same commit, the R2 rule — the single-commit
   8fd0739 pattern chosen over the split 78c87ed pattern precisely so
   R4 is never red between halves): rows ci-artifacts-per-push +
   linux-native landed naming gate p7-ci-artifacts, notes rewritten to
   what the artifact actually is; §6 gained the landed-gate note;
   check-p7-ports-map prints the landed-rows line; its suite
   re-baselined deliberately (the real-repo pin (2 landed, 5 pending) +
   the honest-fixture default manifest now wiring p7-ci-artifacts + the
   forward-shape test at 3 landed / 4 pending). (d) BOUNDS KEPT: no
   engine change (no Rust file touched), no installer byte (the
   artifact is the already-green cargo build --release), the macOS leg
   excluded, the gate reads only committed files (no corpus key, no
   writable), no Ghidra run, no new RE. Verified first-hand: checker OK
   on the real workflow (3 jobs, push trigger, build matrix, both
   uploads, 8 denylisted tokens absent); suite 22/22; check-p7-ports-map
   OK (7 engineering, 2 landed naming p7-ci-artifacts, gate join +
   scaffold-first verified) + its suite 29/29 after the re-baseline;
   test-validate-required-gates 22/22 after the manifest edit;
   check-p6-behavior-catalog OK before AND after; MANIFEST clean (the
   unit reads no corpus); the bounded --phase P7 validator verdict at
   af9cac1: status=passed, ALL 2 P7 GATES GREEN, every command rc=0
   under bwrap containment (report .state/p7-ciartifacts-gates-
   report.json, head-bound to af9cac1e5597). Queued: the CDDA
   user-supply + local-cache unit as the new head (the next registry
   row in contract order).

6. DONE (2026-08-28, claim 1 — commit 8fd0739 by worker 5c84290c,
   PUSHED, plus this bookkeeping commit): P7 opener
   `p7-ports-scaffold` — THE PORTS/PACKAGING DELIVERABLE-MAP
   CONTRACT wired as the FIRST P7 required gate (D221; the
   D175/D200/D216 scaffold pattern: the machine-checkable
   contract lands BEFORE any packaging work it grades). (a) THE
   DECISION SURFACE: docs/P7-PORTS.md pins the PLAN §6 P7 scope
   map VERBATIM (sentence-intact, whitespace-normalized binding)
   and the binding consequences: the three-OS surface, per-push
   artifacts, the CDDA + SteamDeck contracts are ENGINEERING;
   runner/signing/publication availability are EXTERNAL
   conditions recorded as exclusions EXACTLY LIKE THE P4
   LIVE-CAPTURE DIAGNOSTICS so P7 gates grade only the
   engineering (no P7 gate may depend on a store, a key, or a
   runner — the plan's non-blocking sentence made mechanical).
   (b) THE REGISTRY: schema p7-ports-map-v1 (a fenced TOML block
   in the doc, the D216 hd-asset-pins-v1 precedent) — ENGINEERING
   exactly seven (linux-native, flatpak-manifest,
   windows-installer, macos-universal2-ci, ci-artifacts-per-push,
   cdda-user-supply, steamdeck-default; all seed pending = the
   honest scaffold state) + EXTERNAL-CONDITIONAL exactly three
   (macos-runner-availability, signing-keys, publication-stores;
   never carry status/gate, always record the exclusion note).
   THE MECHANICAL RULES (tools/check-p7-ports-map.py, the P6
   catalog numbering mirrored): R1 registry discipline, R2 the
   evidence rule (an engineering deliverable is landed EXACTLY
   WHEN ITS PROVING GATE IS NAMED), R3 exact coverage sets, R4
   the gate join (a named gate exists as a block AND sits in the
   P7 phase list), R5 scaffold-first manifest wiring, R6 the
   surveyable phase-close rule (P7 green requires every
   engineering row landed), R7 boundary sentences verbatim, R8
   exclusions stay exclusions. (c) THE CDDA CONTRACT (§4,
   grounded on already-landed VERIFIED facts — GROUNDWORK.md,
   RESEARCH-8STREET.md, RE-EXW-MAINLOOP.md; no new RE): mixed-
   mode CD (track 1 data, tracks 2..8 = seven CDDA tracks);
   user-supplied originals (never bundled/committed/
   distributed), the documented lookup with SILENT MISS (music
   silent + note, never fatal — the 8street CDDA-disabled
   comparator is standing evidence the game runs music-silent),
   the optional local lossy cache generated on first run into a
   USER-OWNED dir (never game-data/, never the repo, keyed by
   source identity, never redistributed — a derived copy under
   the D21 rule), music stays out of the sim (D17 b/D212
   posture). (d) THE STEAMDECK DEFAULT (§5): a PLATFORM DEFAULT
   not a mode toggle (D200 layering) — on the 1280x800 16:10
   panel the default becomes FILL-THE-PANEL (stretch, never
   pillarbox bars), recorded over the landed D215 scale surface;
   generic platforms keep Integer + Nearest bit-for-bit (the D215
   pin must stay green); the arm choice (Fill vs an explicit
   Stretch arm) is recorded by the delivering unit; platform
   identification is that unit's scope. (e) THE GATE SHAPE P7
   CLOSES ON (§6): every engineering deliverable landed with its
   hermetic offline proving gate + the bounded --phase P7
   validator verdict green. GATE: p7-ports-scaffold wired as the
   FIRST P7 required_gates entry (checker + suite; tracked doc +
   both tools + the manifest; no corpus, no writable; P7 status
   stays pending). Verified first-hand: checker OK (7 engineering
   0 landed 7 pending + 3 recorded exclusions); suite 29/29
   (every rule fails loudly incl. both tampered verbatim-plan
   sentences, missing/extra deliverables, missing exclusion, the
   landed/pending gate discipline both ways, the exclusion-with-
   status, undefined + out-of-phase-list proving gates,
   scaffold-not-first, checker-not-run, untracked doc, premature
   green flip; + the real-repo pin + the legal landed-state
   forward shape); tools/test-validate-required-gates.py 22/22
   re-run AFTER the manifest edit (the strict manifest key schema
   applies to the new gate); controls green BEFORE (HEAD cec7466)
   AND AFTER: check-p6-behavior-catalog OK with P6 green (zero
   open entries) + the gates-validator suite; MANIFEST clean
   before and after (the gate reads no corpus); the bounded
   --phase P7 validator verdict at 8fd0739: status=passed,
   selected_phase P7, the p7-ports-scaffold gate passed with
   both commands rc=0 under bwrap-unshare-net-pid-ro containment,
   offline, head-bound to 8fd07396ef3d (report
   .state/p7-ports-gates-report.json); no engine change, no
   packaging build, no CI change, no Ghidra run. Queued: the
   per-push CI artifact jobs as the new head (the registry's
   ci-artifacts-per-push + linux-native rows), then the CDDA,
   SteamDeck, Flatpak, installer and universal2 units, then the
   P7 phase close.

7. DONE (2026-08-28, claim 1 — commit d01a7b7 by worker 7486871a,
   PUSHED, plus this bookkeeping commit): P6 phase-close
   bookkeeping `p6-phase-close` — THE SURVEYED VERDICT + the P6
   phase status FLIPPED pending->green in docs/required-gates.toml
   (P0-P6 green, P7 pending; plan_complete correctly stays false).
   The survey (DECISIONS.md D220, carried by the flip commit):
   every PLAN section 6 P6 acceptance bullet walked and
   dispositioned — gate-green landed (D200/D201 the scaffold +
   ModeConfig seam; D203/D205/D207/D208 time-based simulation,
   the platform wiring, the high-refresh camera/scroll
   interpolation, the uncapped present; D204 modern controls; the
   D200 triage-rubric + catalog contract with the catalog
   deliberately EMPTY — all 37 P5 ledger catalog_refs empty, no
   entry owed; D215 + D217 resolution independence/scaling + the
   ENHANCED opener; D216 the HD-pipeline research prerequisite;
   D208/D210/D212/D213 the QoL list; D219 the feel-proxy
   benchmark) versus EXPLICITLY deferred by plan text or decision
   (the extended viewport = a separately FLAGGED gameplay change,
   never a silent default; the sub-pixel blitter = a default-off
   later option per PLAN; HD-pack runtime consumption = future
   work per D216; further ENHANCED native passes beyond the
   opener; the versioned save-format writer = future
   config-not-state work per the D201 posture) — the Smacker
   sentence anchor re-verified first-hand (native decode =
   bedlam-smk/SmkStream + the D31 MoviePlayer, the movie frame
   GPU-scaled through the landed present path + D215), nothing
   silently dropped. Then the bound phase verdict RE-EMITTED at
   the flip commit with the exact P4/P5-shaped command:
   /usr/bin/python3 tools/validate-required-gates.py --root .
   --report .state/p6-gates-report.json --phase P6 --phase-output
   .state/P6-COMPLETE — ALL 14 P6 GATES GREEN at d01a7b7 (report
   status=passed, bounded, offline, containment
   bwrap-unshare-net-pid-ro, every command rc=0;
   .state/P6-COMPLETE phase-complete-v1 re-bound to the flip
   commit + manifest sha256 4a9678a7..., producer
   required-gates-validator, emitted by the validator itself).
   Pre-flip first-hand checks: check-p6-behavior-catalog OK with
   P6 status green (the cross-artifact rule — zero open entries,
   satisfied by the empty catalog); the tracked tree stayed clean
   through the whole HEAD-bound battery (the D193/D194 lesson —
   STATE.md/NEXT.md edits parked until after); MANIFEST clean
   before AND after every corpus read; no Ghidra run. The queue
   now carries the P7 opener as the head (the
   p5-phase-close/0c81387 pattern): p7-ports-scaffold per PLAN
   section 6, so required work stays active.

8. DONE (2026-08-28, claim 1 — commits 2b521d1 + eb4981f by worker
   73e5e9a2, both PUSHED): P6 QoL FEEL-PROXY benchmark unit
   `p6-frame-pacing-benchmark` — the plan's own closing instrument
   of the QoL sentence per PLAN §6 "An automated scheduled CI
   benchmark checks 240Hz frame pacing against a pinned hardware
   profile and thresholds; an unavailable profile creates no task
   and only excludes that platform attestation" (implementation
   D219), the last unlanded plan-named P6 piece before the phase
   exit. (a) THE HERMETIC HALF (NEW bedlam-shell pacing.rs):
   CadenceDriver::frame replays a delta trace — measured or
   synthetic frame deltas — through the EXACT present-loop
   arithmetic (FixedStepClock::advance answers pumps due, each due
   pump runs the fixed dt through GameHost::pump_frame with neutral
   input, then the loop's OWN crate-visible window::present_due /
   window::present_camera_alpha answer — the two pure-delegation
   visibilities the harness needed, zero behavior change — and the
   presenting frame recomposes at the accumulator fraction in the
   present site's order gate/alpha/recompose), summarized into the
   feel-proxy metric families: pump cadence (pumps per delta,
   dropped pumps), present-gate answers, the recompose alpha
   cadence at 240Hz, and the nearest-rank p95 frame-time
   percentile. THE LOOP-SHAPE FACT the replay pins (found
   first-hand while landing the tests): a zero-PUMP frame never
   calls pump_frame, so the gate INHERITS the last pump's answer —
   after the first tick it answers YES on every frame in BOTH
   arms; the classic arm's frame-locked hold lands at CONTENT
   level (exactly one NEW image per executed tick — 59 of 240
   frames at 240Hz — unchanged frames re-presented), the alpha
   cadence is the arm-visible difference. Trajectory-neutral
   across arms (identical pump/tick totals, tick index, state
   hash, scene hash — the D203 property re-pinned at the harness
   boundary) and deterministic. (b) THE PINNED HARDWARE PROFILE as
   committed data (PacingProfile/PINNED_240HZ: id
   pinned-240hz-desk-v1, machine class = operator desktop 240Hz
   vsync-locked display, p95 budget 5_208_333 ns = exactly 1.25
   display periods, 2400 bounded samples = 10s of cadence); the
   UNAVAILABLE-PROFILE POSTURE is mechanical and PURE (profile_for
   exact-matches the declared BEDLAM_PACING_PROFILE identity —
   nothing probes hardware, so CI runners and stray machines can
   never produce a false attestation; benchmark_report — the
   measurement binary's entire behavior except the wall-clock loop
   — answers skip-clean: exit 0 + an explicit no-attestation note,
   never a false red, never a task). (c) THE BOUNDED MEASUREMENT
   (NEW examples/frame-pacing.rs, profile-gated): the SAME driver
   against a wall clock as a 240Hz-CADENCE PROXY (sleep to the
   next display-period boundary, measure the pacing path, feed the
   measured inter-frame delta exactly as about_to_wait does); the
   display's own vsync wait is the one piece a surface-less
   benchmark cannot include — said plainly in the docs; exit 1
   exists ONLY on a matched machine whose thresholds failed (p95
   over budget OR dropped pumps — the anti-spiral clamp firing IS
   stutter). (d) THE SCHEDULED CI WIRING (NEW
   .github/workflows/frame-pacing.yml): daily cron 03:23 UTC +
   workflow_dispatch + path-filtered push/PR (continuously
   verified without running on every change); the pacing job runs
   the example — hosted runners never declare the profile, so the
   scheduled job exercises exactly the skip-clean posture; the
   240Hz attestation fires only on the pinned machine's runner.
   BOUNDS KEPT: no engine change (bedlam-shell only); the hashed
   trajectory untouched (bare hosts, no corpus asset ever staged);
   the gate reads no corpus; catalog stays EMPTY (a plan-named
   instrument is not a catalog entry); no new RE (every cited
   original fact already landed: RE-EXW-PACER §3, RE-EXW-CAMERA
   §5); no Ghidra run. GATE: p6-frame-pacing-benchmark wired as
   the FOURTEENTH P6 required_gates entry (implementation + docs +
   gate block 2b521d1; phase list eb4981f — the 78c87ed pattern)
   — commands = bedlam-shell --lib + the example's skip-clean run
   (no env profile under containment -> exit 0 + note), both
   --release --locked --offline, hermetic. Verified first-hand:
   bedlam-shell --lib 116/0 (+12 pacing tests; was 104/0 + 1
   pre-existing ignored); the measurement binary BOTH paths
   (unavailable: exit 0 + the no-attestation note; matched
   diagnostic run on the dev machine: 2400 bounded frames in
   10.1s, p95 4_207_023 ns within the 5_208_333 budget, 600 ticks
   = exactly the 60Hz sim cadence inside the 240Hz proxy, VERDICT
   ATTESTED exit 0 — a diagnostic, not a committed attestation);
   controls green: canonical_dump_gate 13/13, determinism 4/4,
   zone_mission_parity 5/5 (ZERO canonical-chain movement), the
   headless smoke EXACTLY at the recorded baseline (scene
   696adb1cd110e062 / parity cce30c983b97b16d / audio
   110400/158092); check-p6-behavior-catalog OK (catalog still
   empty, R6 satisfied with the fourteenth gate) + its suite
   rc=0; gates-validator suite rc=0; fmt + clippy clean on the
   touched crate (the one pre-existing D210 test warning
   untouched); workspace cargo check clean; the workflow YAML
   parsed (pyyaml, triggers/jobs verified); MANIFEST clean before
   AND after every corpus read; the bounded --phase P6 validator
   verdict at eb4981f: status=passed, ALL 14 P6 GATES GREEN, every
   command rc=0 under bwrap containment (report
   .state/p6-framepacing-gates-report.json, head-bound to
   eb4981f). Queued: the P6 phase-close bookkeeping unit as the new
   head (the surveyed status flip + the phase-complete-v1 verdict
   artifact, the 972748d/f608207 precedent).

9. DONE (2026-08-28, claim 1 — commits ca915fd + 24daf9f by
   worker b3083e9c, both PUSHED): P6 ENHANCED native-render OPENER
   `p6-enhanced-native-render` — the resolution bullet's big
   remaining half per PLAN §6 "ENHANCED mode is explicitly non-parity
   and renders supported world/UI passes natively; bespoke responsive
   layouts target 16:9 and 16:10 (16:10 authoring master with 16:9
   safe region), while other aspect ratios fit/letterbox/pillarbox"
   (design inputs: docs/RESEARCH-HD-ASSET-PIPELINE.md §5.A + §8, the
   p6-hd-asset-research prerequisite). (a) THE SELECTION:
   `PresentationMode` (Parity default = the shipped posture exactly /
   Enhanced) in the NEW `bedlam_platform::layout`, carried as
   `WindowOptions.presentation`; the binary's `--presentation
   parity|enhanced` fails closed at exit 2 (checked first-hand incl.
   the missing value), noted + ignored headless (the smoke at the
   recorded baseline under the flag). D200 layering, NO purist
   arbitration (the D215 posture): OUT of ModeConfig, both pacing
   arms accept it identically, selects NOTHING in the sim. (b) THE
   FIRST NATIVE PASS — the choice documented in P6-MODERNIZATION.md
   §1: the smallest HONEST pass is the MISSION-IDENTITY STRIP (every
   engine-baked pass would need a canonical-frame rewrite — forbidden
   — or ghost over scaled pixels; so the first pass is ADDITIVE in
   the layout's own margin from landed game-owned data only): a
   palette-indexed UI plane AT PRESENTATION RESOLUTION through the
   ALREADY-LANDED parity-pipeline path (`ParityPipeline::with_plane`
   + `upload_indexed` + `draw_rect`), in the responsive layout's LEFT
   pillarbox margin INSIDE the safe region, Mission scenes only —
   identity bytes (RE-EXW-SAVE FUN_004473cd semantics over
   `mission_slot`), glyphs (the LANDED pub `bedlam_render::ui_bank`
   drawer FUN_00402884 + FUN_00408913 advances), color (the game's
   own sidebar 0x24), palette (the canonical frame's own) — ZERO new
   binary claims, ZERO invented pixels, never over game pixels; the
   margins OUTSIDE the safe region stay untouched for the HD-pack
   seam; SMLFONT.BIN via the corpus source once (cached; a miss
   disables the strip, noted, never fatal; headless never fetches);
   2x integer glyph replication. (c) THE RESPONSIVE LAYOUT CONTRACT
   as pure data: 16:10 master 1920x1200, the centered 16:9 safe
   region on ANY target (largest centered ≤16:9 rect — 16:9
   full-bleed, wider pillarboxed, taller letterboxed), the world rect
   REUSING the landed `scale_rect(Fit)` shape (the Fill crop never
   applies), the margins, the ABSOLUTE cursor inverse
   `layout_cursor_to_game`. (d) PARITY BOUNDS pinned:
   bit-identical SimConfig + identical executed ticks, tick count,
   state hash, scene hash AND frame parity hash under either
   selection PLUS the canonical frame indices + palette
   byte-identical; the Parity present path runs the landed calls
   unchanged (`frame_draw_rect` answers the landed `scale_rect`
   under Parity); controls green BEFORE AND AFTER
   (canonical_dump_gate 13/13, determinism 4/4, zone_mission_parity
   5/5 — zero canonical-chain movement; goldens canonical-frame
   based and resolution-agnostic). OUT OF SCOPE kept: the extended
   viewport (separately FLAGGED gameplay change), Smacker native
   decode, further world passes, HD-pack consumption (D216 future).
   GATE: p6-enhanced-native-render wired as the THIRTEENTH P6
   required_gates entry (implementation + docs + gate block ca915fd;
   phase list 24daf9f) — command = bedlam-shell --lib, --release
   --locked --offline, hermetic. Verified first-hand: bedlam-shell
   --lib 104/0 (+12: 7 enhanced-native + 5 native; was 92/0 + 1
   pre-existing ignored); bedlam-game --lib 152/0 + bedlam-core
   --lib 147/0 untouched; headless smoke EXACTLY at the recorded
   baseline (scene 696adb1cd110e062 / parity cce30c983b97b16d /
   audio 110400/158092) under `--presentation enhanced`;
   check-p6-behavior-catalog OK (catalog EMPTY, R6 satisfied with
   the thirteenth gate) + its suite 30/30; gates-validator suite
   22/22; fmt + clippy clean on the touched crates (the one
   pre-existing D210 test warning untouched); workspace cargo check
   clean; MANIFEST clean before and after every corpus read; the
   bounded --phase P6 validator verdict at 24daf9f: status=passed,
   ALL 13 P6 GATES GREEN, every command rc=0 under bwrap containment
   (report .state/p6-enhancednative-gates-report.json, head-bound
   to 24daf9fe937f); no Ghidra run. Queued: the QoL feel-proxy
   scheduled frame-pacing benchmark as the new head (the plan's own
   closing instrument of the QoL sentence, the last unlanded
   plan-named P6 piece before the phase exit).

10. DONE (2026-08-28, claim 1 — commits 4975281 + d63c82f by worker
   b9f4e384, both PUSHED): P6 HD asset pipeline RESEARCH opener
   `p6-hd-asset-research` — docs/RESEARCH-HD-ASSET-PIPELINE.md, the
   plan's OWN named prerequisite ("exact package/model pins come from
   docs/RESEARCH-HD-ASSET-PIPELINE.md"), ADOPTED from the committed
   2026-08-18 groundwork draft (861aebe, preserved — no reset) and
   REFRESHED against PRIMARY sources with the web tools, every
   load-bearing pin re-verified FIRST-HAND 2026-08-28: (a) the FOUR
   workflow categories each carry candidate ComfyUI workflow presets +
   exact pins — 4:3 -> 16:9/16:10 background outpainting/generative fill
   (FLUX.1-Fill-dev primary: gated, flux-1-dev-non-commercial-license,
   the card's own color-shift/edge-line limitations; SD2-inpainting
   fallback demoted NEVER-PRIMARY after the model card returned 401
   login-gated first-hand; SDXL base fallback openrail++ research-only;
   the official flux_fill_outpaint_example template verified in the
   Comfy-Org listing), alpha-aware sprite/sprite-sheet upscale
   (RealESRGAN x2plus v0.2.1 + x4plus_anime_6B v0.2.2.4 zoo URLs +
   SwinIR Apache-2.0 003_realSR for tiles), seamless tile/texture
   upscale, portraits/UI art (GFPGAN v1.4 surveyed-DEFERRED: documented
   identity drift, no core node; CodeFormer EXCLUDED from distributable
   packs: NTU S-Lab non-commercial) — all in a MACHINE-CHECKABLE
   embedded registry, schema hd-asset-pins-v1; (b) tool pins refreshed:
   ComfyUI v0.34.0 (GPL-3.0 verified at the tag; PyTorch >= 2.7 since
   v0.32.0) + v0.33.1 fallback, comfy-cli v1.18.0 (v1.17.0 re-homed
   API-workflow validation) + v1.16.0 fallback, Arctic Helper v0.2.9
   unchanged + package sha256; (c) the hd-pack-manifest-v1 provenance +
   manifest schema design (source/recipe/generation/runtime/model/
   output/review groups; in-git mirror carries ids+hashes NEVER pixels);
   (d) the five-family automated gate criteria design — provenance,
   dimensions, alpha integrity, seam quality, perceptual thresholds,
   fail-closed (outputs without recorded provenance are excluded from
   shipping); (e) the runtime resolution seam sketch — stable logical
   asset ID, ALWAYS-silent fallback to originals on any miss/mismatch,
   the engine renders all text/controls/click targets/gameplay
   information (never hallucinated into generated backgrounds), a
   platform option OUT of ModeConfig (D200), ENHANCED-mode only with
   parity untouched; (f) the isolated + hardware-profiled setup posture
   (uv-managed Python 3.12, loopback-only + --disable-api-nodes, core
   nodes only, recorded hardware profile, non-corpus smoke pixels);
   SeedVR2 re-anchored Apache-2.0 core-native since PR #14424 with exact
   variant filenames (phase 2). GATE: p6-hd-asset-research wired as the
   TWELFTH P6 required_gates entry — commands = the bounded OFFLINE
   checker over the committed doc (tools/check-p6-hd-asset-research.py,
   the e0bc7fb scaffold pattern: ten required sections incl. the four
   category sections, the plan-boundary sentences verbatim under
   whitespace-normalized matching, pin-registry discipline — first-party
   https hosts, verification-window retrieval dates, licenses on every
   model pin, VERIFIED licenses on primaries, notes on deferred pins,
   categories on every model pin, per-category primary coverage +
   outpaint fallback + the comfyui/comfy-cli tool pins — and the
   cross-artifact manifest wiring) + its hermetic suite
   tools/test-p6-hd-asset-research.py. Verified first-hand: checker OK
   (10 model + 5 tool + 2 workflow-template pins; all four categories
   covered); suite 27/27 (every rule proven to fail loudly incl. the
   BOTH-sprite-primaries demotion, the unverified-license-primary, and
   the four manifest-wiring tampers; + the real-repo pin);
   check-p6-behavior-catalog OK (catalog still empty, R6 satisfied with
   the twelfth gate) + its suite 30/30; gates-validator suite 22/22;
   MANIFEST clean before and after (the gate reads no corpus); the
   bounded --phase P6 validator verdict at d63c82f: status=passed, ALL
   12 P6 GATES GREEN, every command rc=0 under bwrap containment
   (report .state/p6-hdasset-gates-report.json, head-bound to
   d63c82fe11b9); DECISIONS.md D216 records the gate design; no engine
   or workspace Rust change; no Ghidra run; RESEARCH ONLY — no
   generated assets, catalog stays EMPTY. Queued: the ENHANCED
   native-render opener as the new head (the resolution bullet's big
   remaining half, design inputs from this doc's §5.A/§8).
11. DONE (2026-08-28, claim 1 — commits 017a0f4 + 78c87ed by
   worker 8754d532, both PUSHED): P6 resolution-independence unit
   `p6-scaling-options` — the SCALING SELECTION per PLAN §6
   "Resolution independence + GPU rendering ... (nearest/integer
   default; fit/fill/smooth options)" (implementation D215), the
   resolution bullet's last small piece after the complete QoL
   list. (a) THE SURFACE: the ALREADY-LANDED bedlam-platform
   scale surface (ScaleMode Integer the parity default / Fit /
   Fill + FilterMode Nearest the parity default / Linear = smooth,
   consumed by the parity pipeline's GPU scale path +
   cursor_to_game) exposed as a platform presentation knob riding
   the EXISTING WindowOptions.present — three PURE functions in
   bedlam-shell window.rs: the fail-closed CLI word mappers
   scale_mode_from_cli/filter_mode_from_cli + the ONE composed
   mapping scaling_present_config (the binary's only route into
   present; defaults in = PresentConfig::default bit-for-bit).
   (b) D200 LAYERING, NO PURIST ARBITRATION (the D210 posture):
   the original was a FIXED 640x480 DOS framebuffer with no
   scaling mode to preserve — both pacing arms accept the
   selection identically (pinned by
   scaling_option_never_changes_the_gate_answers) and the knob
   selects NOTHING in the host beyond the PresentConfig the GPU
   scale path consumes (never ModeConfig/SimConfig/any hash;
   pinned by
   scaling_selection_never_touches_the_sim_or_the_hashed_
   trajectory over the full 3x2 selection: bit-identical
   host_sim_config + identical executed ticks, tick count, state
   hash, scene hash AND frame parity hash); the mapping touches
   ONLY the two knob fields — the palette expansion stays
   VgaExpand::Original under every selection (pinned by
   scaling_selection_is_a_pure_present_config_mapping), so the
   canonical 640x480 indexed frame + palette ride unchanged and
   goldens stay resolution-agnostic; the Fill cursor-uv handling
   stays window-side (pinned by
   fill_scaling_cursor_is_relative_only_and_filter_invariant:
   absolute mapping under Integer/Fit, relative aiming under
   Fill); the binary's --scale MODE/--filter MODE fail closed at
   exit 2 on any other word (the --save-slot domain posture).
   (c) BOUNDS KEPT: NO new RE (a pure modern platform surface
   over landed code, zero new binary claims — no RE artifact
   owed); bedlam-shell only, no engine change; the headless path
   owns no surface so the flags are noted + ignored there (it
   hashes the SOURCE frame); catalog stays EMPTY. (d) GATE:
   p6-scaling-options wired as the ELEVENTH P6 required_gates
   entry (implementation + docs 017a0f4; gate block + phase list
   78c87ed) — command = bedlam-shell --lib, --release --locked
   --offline, hermetic. Verified first-hand: bedlam-shell --lib
   92/0 (+6 in scaling_option_tests: the shipped-default pin, the
   full-domain fail-closed CLI words, the pure two-field mapping,
   the sim/trajectory pin over the full selection, the both-arms
   gate-answer invariance, the Fill cursor posture; was 86/0 + 1
   pre-existing ignored); the binary --help/--scale/--filter
   wiring checked first-hand (help text, the domain rejections at
   exit 2 incl. the missing value, the headless ignore note) AND
   the headless smoke EXACTLY at the recorded baseline (scene
   696adb1cd110e062 / parity cce30c983b97b16d / audio
   110400/158092) under --scale fill --filter linear; controls
   green: canonical_dump_gate 13/13, determinism 4/4,
   zone_mission_parity 5/5 (ZERO canonical-chain movement);
   check-p6-behavior-catalog OK (catalog still empty, R6
   satisfied with the eleventh gate) + its suite; gates-validator
   suite 22/22; fmt + clippy clean on bedlam-shell (the one
   pre-existing D210 test warning untouched); workspace cargo
   check clean; MANIFEST clean before AND after the
   corpus-reading smoke (the gate reads no corpus); the bounded
   --phase P6 validator verdict at 78c87ed: status=passed, ALL 11
   P6 GATES GREEN, every command rc=0 under bwrap containment
   (report .state/p6-scalingoptions-gates-report.json, head-bound
   to 78c87ed60ff92e5969ebc175c55fe3e719f33219); no Ghidra run.
   Queued: the HD asset pipeline RESEARCH opener as the new head
   (the plan's own named prerequisite doc; the ENHANCED
   native-render half of the resolution bullet stays the
   bullet's big remaining piece, a separately scoped unit).

12. DONE (2026-08-28, claim 1 — commits 63d58ac + bece1cf + 9b2599f
   by worker bd07c7b6, all PUSHED): P6 QoL unit `p6-save-slots` —
   the SAVE SLOTS + METADATA + OPT-IN AUTOSAVE sentence per PLAN
   §6 "QoL: ... save slots + metadata + opt-in autosave"
   (implementation D213), the LAST QoL list item after the landed
   D208 vsync control, D210 window modes and D212 volume mixers —
   THE PLAN §6 QoL LIST IS NOW DONE END TO END. (a) RE FIRST:
   docs/RE-EXW-SAVE.md committed BEFORE the implementation
   (63d58ac) — the original save surface decoded objdump-only
   from the committed exw-text-objdump.txt (plus read-only
   string probes of BEDLAM.EXW, MANIFEST clean before and after):
   the EXW persistence is REGISTRY-BACKED (value "SAVEGAME" 0x384
   = the whole 5x180 image over the slot buffer 0x4eae58, twin
   "HISCORES" 0x78; FUN_00446f4f = the loader + the first-run
   five-EMPTY initialization 0x44705d..0x44706c; the
   "<dir>SAVED.BDL<name>" path build inside the save screen
   0x44694c..0x4469dd is passed to NOTHING — the §7j.56
   CONFIG.BDL leftover pattern repeating save-side), the whole
   WRITER side decoded (FUN_004446938 = the save screen: the
   campaign-shell SAVE button gate 0x43eee1..0x43ef3e —
   single-player 0x4edb88==0 AND click AND armed flag 0x4eae54
   AND the button cursor region; the slot write arm mirrors the
   §7j.70 restore grammar instruction-for-instruction with the
   mask derived LIVE from the completion table 0x4decae; the
   whole-image registry commit 0x446e98; sel 5 = Cancel writes
   nothing), the exhaustive 0x44ed98 caller census => EXACTLY
   TWO user-initiated SAVEGAME writers — THE SHIPPED GAME NEVER
   AUTOSAVES — and FUN_004473cd = the slot metadata text (one
   space + the zone letter 'A'+zone-1 + one digit '1'..'5' per
   set mask bit; the menu line = the name space-padded to 8 +
   that text, "EMPTY" for empty rows). (b) bedlam-shell save.rs
   (NEW): SaveSlotId (the original's own FIVE-slot domain,
   0-based like the restore dispatch; default FIRST = a MODERN
   platform default, the original has no persistent selection),
   SaveSlotMetadata/SaveSlotRow/summarize_saved_bdl (the
   five-row save/load list over the engine's IMPORT-ONLY seam —
   empty slots are ROWS, broken images stay loud GameErrors),
   save_level_text/slot_menu_line (BYTE-FAITHFUL to
   FUN_004473cd), AutosavePolicy (NEVER-default-Off, pinned at
   every layer; the opt-in's gate mirrors the original's own:
   should_autosave(single_player, campaign_boundary) ONLY — never
   mid-mission, never coop/h2h) — carried as
   WindowOptions::save_slot/autosave with the binary's
   --save-slot N (1..=5, exit 2 on anything else) and --autosave
   opt-in flags (both noted + ignored headless, the --fullscreen
   posture). (c) BOUNDS KEPT: the surface lands INERT (the D201
   seam posture — NO engine write seam ships; the new versioned
   save FORMAT writer is future engine work, config-not-state
   when it lands: a restore ADOPTS the saved session, never a
   mid-run mutation, FORMAT_VERSION and every hash pin
   byte-stable; SAVED.BDL stays import-only, no writer owed or
   allowed for parity); NO bedlam-game/bedlam-core file changed —
   the sim, the ModeConfig and every hash untouched by
   construction (pinned by save_surface_never_touches_the_sim_
   config: bit-identical host_sim_config under FIRST/Off and
   LAST/On-LAST); catalog stays EMPTY. (d) GATE: p6-save-slots
   wired as the TENTH P6 required_gates entry (gate block
   bece1cf + phase list 9b2599f) — command = bedlam-shell --lib,
   --release --locked --offline, hermetic. Verified first-hand:
   bedlam-shell --lib 86/0 (+11: 10 save — the five-slot domain,
   the EXW-faithful level text (a wrong worked example in the
   first artifact draft was CAUGHT by this test and corrected
   before push: mask 0b10011 = "125" not "135"), the menu-line
   padding, the EMPTY row, the five-row summary + the metadata
   mapping + the loud rejections, the full-domain metadata sweep
   over the whole modeled stage/mask space, the never-default
   pin, the original's own autosave gate; 1 window — the
   sim-config invariance; was 75/0 + 1 pre-existing ignored); the
   binary --help/--save-slot/--autosave wiring checked first-hand
   (help text, the 1..=5 domain rejection at exit 2, the headless
   ignore note) AND the headless smoke EXACTLY at the recorded
   baseline (scene 696adb1cd110e062 / parity cce30c983b97b16d /
   audio 110400/158092) first-hand under --save-slot 3
   --autosave; bedlam-game --lib 152/0 untouched; controls green:
   canonical_dump_gate 13/13, determinism 4/4, zone_mission_
   parity 5/5 (ZERO canonical-chain movement); check-p6-behavior-
   catalog OK (catalog still empty, R6 satisfied with the tenth
   gate) + its suite 30/30; gates-validator suite 22/22; fmt +
   clippy clean on bedlam-shell (the one pre-existing D210 test
   warning untouched); workspace cargo check clean; MANIFEST
   clean before AND after every corpus read (the RE probe only;
   the gate reads no corpus); the bounded --phase P6 validator
   verdict at 9b2599f: status=passed, ALL 10 P6 GATES GREEN,
   every command rc=0 under bwrap containment (report
   .state/p6-saveslots-gates-report.json, head-bound to
   9b2599f636bbc7fa4e35365f601191d6d3f05df2); no Ghidra run.
   Queued: the resolution-independence scaling-options unit as
   the new head (the fit/fill/smooth selection over the landed
   bedlam-platform ScaleMode/FilterMode — the QoL list is
   complete, so the queue advances to the resolution bullet's
   last small piece).
13. DONE (2026-08-28, claim 1 — commits f49315f + aa6673c + 1b42327
   by worker 1b994336, all PUSHED): P6 QoL unit
   `p6-volume-mixers` — the VOLUME MIXERS presentation option per
   PLAN §6 "QoL: window modes, vsync control, volume mixers, ..."
   (implementation D212), the next QoL list item after the landed
   D208 vsync control and D210 window modes. (a) RE FIRST:
   docs/RE-EXW-MUSIC sec 7 committed BEFORE the implementation
   (f49315f) — the volume-surfaces bus-split re-anchor, every fact
   re-anchored to its owning verified section with NO new claims:
   the shipped EXW has ONE shared master bus (the master word
   004ee9b4 written only by FUN_0044c630 from the two UI volume
   paths; the mission-shell Up/Down stepper moves g_music_volume
   0..100 in +/-5 steps applied vol>>1 -> master 0..50,
   RE-EXW-INPUT sec 5; the SAME SubVoiceStart master product
   scales music AND sfx voices; SetVolume is spawn-snapshotted;
   the registry sfx-master-gate 0x4ede58 D134 is the only sfx-side
   separation, an on/off MUTE never a gain) — so the original's
   "music volume" stepper is a WHOLE-MIX master and a per-bus
   music/sfx selection is a MODERN platform addition whose default
   must equal the shipped single-bus mix bit-exactly. (b)
   bedlam-shell audio.rs: VolumeLevel 0..=100 percent (the
   original's own UI domain, clamped the original's way) +
   VolumeMixers music/sfx (default SHIPPED = both FULL) carried as
   WindowOptions::volume — a PLATFORM knob OUT of ModeConfig per
   D200 layering with NO purist arbitration (audio is presentation
   bucket, D17 b; the knob never consults the mode); THE GAIN
   APPLICATION SITE is the shell audio path ONLY: AudioFeed::
   fill_from scales the DEVICE-BOUND copy after GameHost::
   render_audio, Q8 integer math composed multiplicatively on the
   faithfully un-split engine bus (each knob alone behaves exactly
   like the original's own whole-mix stepper), unity at the
   default = an EXACT bit-identical passthrough. (c) THE BOUNDED
   RUNTIME KEY SET (the F11 posture): PageUp/PageDown step the
   music bus +/-5 and BracketRight/BracketLeft the sfx bus — the
   original's own step and 0..100 clamp — intercepted in the event
   handler BEFORE the mapper so they never reach ShellInput, all
   four keys dead to both schemes (the ORIGINAL volume keys Up/
   Down deliberately stay scheme keys); the binary's --music PCT /
   --sfx PCT select the starting levels (noted + ignored
   headless); an adjustment applies to future fills only. (d)
   BOUNDS KEPT: no engine change (bedlam-shell only); the engine's
   mixed parity stream, the sim and every hash untouched under ANY
   knob setting (pinned by volume_mixers_never_touch_the_engine_
   stream + volume_selection_never_touches_the_sim_config; the
   headless smoke at the recorded baseline scene 696adb1cd110e062
   / parity cce30c983b97b16d / audio 110400/158092 first-hand
   under --music 50 --sfx 0); catalog stays EMPTY. (e) GATE:
   p6-volume-mixers wired as the NINTH P6 required_gates entry
   (gate block aa6673c + phase list 1b42327) — command =
   bedlam-shell --lib, --release --locked --offline, hermetic.
   Verified first-hand: bedlam-shell --lib 75/0 (+10: seven audio
   — the level domain/Q8 conversion, the shared-bus composition,
   the unity bit-exact identity sweep, the scale/mute math, the
   shipped-default engine-stream pin, the knob-invariance pin,
   the future-fills-only pin; three window — the platform-only key
   set dead to both schemes, the original step/clamp, the
   sim-config invariance; was 65/0 + 1 pre-existing ignored); the
   binary --help/--music/--sfx wiring checked first-hand (help
   text, the headless ignore note); controls green:
   canonical_dump_gate 13/13, determinism 4/4 + bedlam-core 12/12,
   zone_mission_parity 5/5 (ZERO canonical-chain movement);
   check-p6-behavior-catalog OK (catalog still empty, R6 satisfied
   with the ninth gate) + its suite 30/30; gates-validator suite
   22/22; fmt + clippy clean on bedlam-shell (the one pre-existing
   D210 test warning untouched); workspace cargo check clean;
   MANIFEST clean before AND after every corpus read; the bounded
   --phase P6 validator verdict at 1b42327: status=passed, ALL 9
   P6 GATES GREEN, every command rc=0 under bwrap containment
   (report .state/p6-volumemixers-gates-report.json, head-bound to
   1b42327); no Ghidra run. Queued: the QoL save slots + metadata
   + opt-in autosave sentence as the new head (window modes, vsync
   control and volume mixers now DONE).
14. DONE (2026-08-28, claim 1 — commit 8784da1 by worker 7aed939f,
   PUSHED): P6 QoL unit `p6-window-modes` — the WINDOW MODES
   presentation option per PLAN §6 "QoL: window modes, vsync
   control, ..." (implementation D210), the direct sibling of the
   landed D208 vsync option. (a) bedlam-shell window.rs:
   `WindowMode` (Windowed default, exactly as shipped / Borderless
   borderless-fullscreen / exclusive-style Fullscreen best-effort)
   as `WindowOptions::window_mode` — a PLATFORM knob OUT of
   ModeConfig per D200 layering with NO purist arbitration this
   time: the original was a fullscreen DOS exclusive with no
   windowed mode to preserve, so both pacing arms accept the
   selection identically and the selection selects NOTHING in the
   host (pinned by window_mode_selection_never_touches_the_sim_
   or_the_hashed_trajectory: bit-identical derived SimConfig and
   identical executed ticks, sim tick count, state hash, scene
   hash AND frame parity hash under all three options; the
   present-gate/alpha answers are option-invariant in both arms
   per window_mode_option_never_changes_the_gate_answers). (b) THE
   PURE MAPPING (hermetic, no window needed): fullscreen_target
   over plain VideoModeChoice data — Windowed -> None; Borderless
   -> Borderless regardless of candidates (no mode switch
   involved); Fullscreen -> pick_exclusive_mode (largest area,
   then highest refresh, then highest bit depth — a TOTAL order,
   list-order independent), else the HONEST borderless
   degradation (an empty candidate list degrades, noted at
   configure time, never fatal — the same best-effort posture as
   the D208 surface mapping). (c) THE F11 RUNTIME TOGGLE, bounded
   and PLATFORM-ONLY: a window-manager key OUTSIDE both control
   schemes, intercepted in the event handler BEFORE the mapper so
   it never reaches ShellInput (pinned by
   f11_is_the_only_platform_toggle_key_and_is_dead_to_both_schemes
   — F11 only, and it maps to nothing in either scheme, so even a
   forwarding bug could not make it sim input), the pure
   transition toggle_fullscreen_target (leaving always returns to
   windowed; entering uses the selection's preferred shape — a
   Windowed selection enters Borderless per the universal F11
   convention), and ONE shared impure binder apply_fullscreen for
   the window build and the toggle so the two sites' shapes can
   never disagree. (d) BOUNDS KEPT: the swapchain follows the
   EXISTING Resized reconfigure path only; the shell fixed-step
   clock/pump contract untouched; catalog stays EMPTY. (e) GATE:
   p6-window-modes wired as the EIGHTH P6 required_gates entry —
   command = bedlam-shell --lib, --release --locked --offline,
   hermetic. Verified first-hand: bedlam-shell --lib 65/0 (+7
   window-mode tests; was 58/0 + 1 pre-existing ignored); the
   binary --help/--fullscreen/--borderless wiring checked
   first-hand (help text, the headless ignore note); controls
   green: canonical_dump_gate 13/13, determinism 4/4,
   zone_mission_parity 5/5 (ZERO canonical-chain movement),
   headless smoke at the recorded baseline (scene 696adb1cd110e062,
   parity cce30c983b97b16d, audio 110400/158092);
   check-p6-behavior-catalog OK (catalog still empty, R6 satisfied
   with the eighth gate) + its suite 30/30; gates-validator suite
   22/22; fmt + clippy clean on bedlam-shell; workspace cargo
   check clean; MANIFEST clean before AND after every corpus read;
   the bounded --phase P6 validator verdict at 8784da1:
   status=passed, ALL 8 P6 GATES GREEN, every command rc=0 under
   bwrap containment (report .state/p6-windowmodes-gates-report.json,
   head-bound to 8784da1); no Ghidra run. Queued: the QoL volume
   mixers unit as the new head (PLAN §6 QoL list order — window
   modes and vsync control now DONE).
   NOTE (watchdog repair 1787918690, D211): the worker's model
   connection died provider-side (transport, rc=0, progress=1)
   AFTER this completion rewrite and its printed final summary —
   a post-completion transport death, no work lost (8784da1 +
   this bookkeeping both PUSHED, strict parser rc=0 on the
   rewritten queue); the structured transport failure was
   adjudicated replaced-task per the D206 checklist (all four
   items green, D211) and item 1 above stands untouched, READY.
15. DONE (2026-08-28, claim 1 — commit 44c6f2d by worker 754e7c94,
   PUSHED, plus this bookkeeping commit): P6 present-option unit
   `p6-uncapped-present-mode` — the OPTIONAL UNCAPPED PRESENT MODE,
   the remaining half of the PLAN §6 present sentence ("vsync-
   locked present at any refresh (60/120/144/240/360Hz+) or
   uncapped"), sibling of the landed D207 interpolation policy
   (implementation D208). (a) bedlam-shell window.rs: `Vsync`
   (Locked default / Uncapped) as a PLATFORM presentation option
   on WindowOptions (D200 layering — vsync is a platform knob, OUT
   of ModeConfig; default = the vsync-locked Fifo present exactly
   as shipped; the binary's `--uncapped` selects the request,
   noted + ignored on the headless path); `effective_vsync(mode,
   requested)` — the POLICY SELECTION: the request is arbitrated
   by the SAME timing-lock arm GameHost::present_pacing reads
   (D203, agreement unit-pinned: Uncapped is effective iff
   Decoupled AND requested) — the modern Decoupled arm HONORS it,
   the classic FrameLocked arm DECLINES it and pins vsync-locked
   (RE-EXW-PACER §3 — the visible refresh follows the fixed logic
   tick, never the display rate; axis independence pinned: the
   control-scheme arm alone never declines); `surface_present_mode`
   — the PURE winit/wgpu PresentMode mapping (Locked -> Fifo
   unconditionally at any refresh; Uncapped -> Immediate when the
   surface offers it, else the HONEST Fifo fallback — Mailbox is
   NOT uncapped, it still paces to the display; best-effort
   platform knob, stderr note at configure time, never fatal). (b)
   LOOP SHAPE: no loop code changes — with the Fifo block gone the
   unconditional D205 redraw cycle free-runs: the loop presents as
   fast as it runs, every present recomposing from latest state +
   the D207 interpolated camera at the clock accumulator fraction
   (present_camera_alpha unchanged) — coherent frames BY
   CONSTRUCTION (recompose re-renders from LATEST state: idempotent
   + drift-free per present, pinned); each iteration still executes
   at most what the clock banks (fixed dt per pump — the fixed-step
   clock/pump contract untouched). (c) BOUNDS KEPT: the selection
   never enters ModeConfig/SimConfig (pinned by
   uncapped_selection_never_touches_the_hashed_trajectory:
   identical executed ticks, tick count, state hash, scene hash AND
   frame parity hash under either option) and the present-gate/
   alpha answers are option-invariant
   (vsync_option_never_changes_the_gate_answers); NO new RE needed
   (no new binary claims — rests on the committed RE-EXW-PACER §3
   + D200 layering); test surface = the ONE purist toggle, both
   arms, hermetic (no window needed); catalog stays EMPTY. (d)
   GATE: p6-uncapped-present-mode wired as the SEVENTH P6
   required_gates entry — command = bedlam-shell --lib,
   --release --locked --offline, hermetic. Verified first-hand:
   bedlam-shell --lib 58/0 (+6: the shipped-default pin, the
   policy selection both arms + the axis-independence control +
   the end-to-end decline, the pure surface mapping incl. the
   Mailbox refusal, the trajectory pin, the option-invariant gate
   answers, the uncapped loop-shape coherence/drift pin); fmt +
   clippy clean on the touched crate; binary --help/--uncapped
   wiring checked first-hand (help text, the headless ignore note);
   controls green: canonical_dump_gate 13/13, determinism 4/4,
   zone_mission_parity 5/5 (ZERO canonical-chain movement),
   headless smoke two-run byte-identical AND at the recorded
   baseline (scene 696adb1cd110e062, parity cce30c983b97b16d,
   audio 110400/158092); gates-validator suite 22/22;
   check-p6-behavior-catalog OK (catalog still empty, R6 satisfied
   with the seventh gate) + its suite 30/30; MANIFEST clean before
   AND after every corpus read; the bounded --phase P6 validator
   verdict at 44c6f2d: status=passed, ALL 7 P6 GATES GREEN, every
   command rc=0 under bwrap containment (report
   .state/p6-uncapped-gates-report.json, head-bound to 44c6f2d);
   no Ghidra run. Queued: the QoL window-modes platform unit as
   the new head (PLAN §6 QoL list order — vsync control now DONE).
16. DONE (2026-08-28, claim 1 — commits fe5bf72 + 37aaddf by worker
   ceafd198, both PUSHED): P6 present-quality unit
   `p6-high-refresh-interpolation` — the composition policy of the
   modern decoupled present per PLAN §6 "Most high-refresh frames
   carry zero new logic ticks; the frame is composed from latest
   state + camera/scroll interpolation" (implementation D207). (a) RE
   FIRST: docs/RE-EXW-CAMERA.md committed BEFORE the implementation
   (fe5bf72) — the EXW camera/scroll traffic collected with every
   fact re-anchored to EXW/EXD addresses and cross-referenced to the
   owning committed section (the Q5 pixel pair 004edde4/8 with
   camTile>>5 to 004ddb24/28 and the iso sprite math x>>8−cam; the
   scroll-source cursor pair 004eddc4/8 via ScrollUpdate 00425ab9
   clamp 9..631 x 9..463 with the EXD mickeys twin; the tick-boundary
   writers — the robots() recenter 0x40b875..0x40b8c5 at
   (cursor−240)·v/480 per axis and the FUN_004245c9 chase-camera
   override; the frame-path readers — FUN_00403938 the viewport
   renderer and FUN_00401107 the present fine-offsets) PLUS the one
   new claim the policy rests on: NO sub-tick camera interpolation
   exists anywhere in the decoded original (the Q5 fixed point is
   sub-PIXEL precision within integer tick updates, never sub-TICK;
   the zoom Q16 magnifier scales an already-rendered backbuffer). (b)
   bedlam-game host.rs: the presentation-bucket prev_sim endpoint
   staged in pump_frame (a clone of the sim as of the pump BEFORE the
   last executed tick batch; zero-tick pumps keep the previous
   endpoint; D17 b — never advanced, never hashed, never serialized;
   Sim gains a Clone derive for exactly this, documented
   presentation-snapshots-only, FORMAT_VERSION and every P5 pin
   byte-stable), render_now split into render_with(prev, alpha) with
   the PUMP path still the PURE parity configuration (prev None,
   alpha 0 — zero canonical-chain movement), and recompose(alpha)
   re-rendering the presented frame from LATEST state with the camera
   lerped (prev -> cur) · alpha — INTERPOLATE CAMERA/SCROLL ONLY
   (sprites stay grid-quantized; the sub-pixel blitter stays
   default-off and out of scope), gated by camera_interpolation() =
   the Decoupled arm ONLY: the CLASSIC frame-locked arm is a NO-OP
   (it presents only after a tick — the exact tick-state camera of
   the original, nothing to interpolate); movie/loading/boot/brief/
   menu planes REPLACE the scene pipeline so presented non-scene
   planes are interpolation-invariant by construction. (c)
   bedlam-shell: FixedStepClock::fraction() — the ACCUMULATOR
   FRACTION of the pending logic tick (banked_ns / PUMP_PERIOD_NS,
   saturated 0..=1; the one float in the integer clock,
   presentation-side only) + PUMP_PERIOD_NS const; window.rs
   present_camera_alpha pairs the gate host with the clock fraction
   and the present site feeds it to GameHost::recompose BEFORE the
   upload — zero-tick high-refresh frames now carry the interpolated
   camera sweep, while the 60 Hz steady state banks exactly the floor
   period so fraction 1.0 = the parity camera: the modern arm adds NO
   latency on the original display class and the sweep only becomes
   visible when the display outpaces the fixed tick rate. (d) BOUNDS
   KEPT: the shell fixed-step clock/pump contract and the hashed
   trajectory untouched — pinned host-side by
   camera_interpolation_never_touches_the_hashed_buckets (same pump
   script with the modern arm recomposing at the clock fractions =
   identical executed ticks, tick count, state hash, scene hash) and
   platform-side by
   present_site_recompose_never_touches_the_hashed_trajectory; the
   frame parity hash DELIBERATELY may diverge on the modern arm after
   a recompose (the interpolated camera IS the feature — the pump
   path re-renders parity regardless). (e) GATE:
   p6-high-refresh-interpolation wired as the SIXTH P6 required_gates
   entry — commands = bedlam-game --lib + bedlam-shell --lib, both
   --release --locked --offline, hermetic. Verified first-hand:
   bedlam-game --lib 152/0 (+4: policy selection both arms + the
   axis-independence control, recompose modern-only with alpha
   endpoints and purity, recompose inert before the first executed
   tick, the Determinism-Charter hash pin), bedlam-shell --lib 52/0
   (+5: three fraction pins — the 240 Hz quarter sweep, the 60 Hz
   steady 1.0, endpoint saturation — and the two present-site pins —
   the arm selection and the hashed-trajectory pin), bedlam-core
   --lib 147/0 (Clone derive only) + hash_fixture + mission_corpus_
   gate green, bedlam-render determinism 12/0; controls green:
   canonical_dump_gate 13/13, determinism 4/4, differ_gate 4/4,
   zone_mission_parity 5/5 (ZERO canonical-chain movement);
   check-p6-behavior-catalog OK (catalog still empty, R6 satisfied
   with the sixth gate) + suite 30/30; gates-validator suite 22/22;
   fmt + clippy clean on the touched crates; MANIFEST clean before
   AND after every corpus read; the bounded --phase P6 validator
   verdict at 37aaddf: status=passed, ALL 6 P6 GATES GREEN, every
   command rc=0 under bwrap containment (report
   .state/p6-highrefresh-gates-report.json, head-bound to 37aaddf);
   no Ghidra run. Queued: the optional uncapped present mode as the
   new head (the same PLAN §6 sentence's remaining half — vsync-
   locked at any refresh OR uncapped, logic fixed in both).
17. DONE (2026-08-28, claim 1 — commit 9a96a60 by worker 2a90eb65,
   PUSHED, plus this bookkeeping commit): P6 platform wiring unit
   `p6-present-loop-wiring` — the mode plumbed through the shell
   host config into BOTH platform consumers and the window present
   loop honoring the D203 gate (PLAN §6 + the D203/D204 scope
   notes; implementation D205). (a) engine/bedlam-shell/src/
   window.rs: WindowOptions.mode — ONE immutable ModeConfig
   selected at the platform level (default = modern; the binary's
   --classic selects the CLASSIC preset) — routed into BOTH
   construction sites: host_sim_config (the mode rides SimConfig
   into GameHost::new as config, never state; seed/time base stay
   defaults) and shell_input_for (the SAME plumbed mode selects
   the mapper's ControlScheme via ControlScheme::for_mode — the
   D204 consumer's platform selection; the window path ran
   default-modern until this unit). (b) THE PRESENT GATE:
   present_due — pure delegation to GameHost::should_present,
   consulted at the PRESENT SITE in ShellApp::present: modern
   presents every vsync (zero-tick high-refresh frames recompose
   and present); classic holds the previously presented image on
   zero-tick host frames (the original frame-locked present-coupled
   pacing, RE-EXW-PACER §3 verified — the visible refresh follows
   the fixed logic tick, never the display rate). Loop liveness:
   the redraw request stays UNCONDITIONAL in both arms — gating
   the request itself would stall a quiet classic loop (no event
   would wake the Wait-mode loop between ticks); only the surface
   write is gated. (c) BOUNDS KEPT: the shell fixed-step clock/
   pump contract and the hashed trajectory untouched (pinned
   shell-side by platform_mode_plumbing_never_touches_the_hashed_
   trajectory: same pump script through hosts built from both
   platform options = identical executed ticks, sim tick count,
   state hash, scene hash AND frame parity hash; presentation
   bucket only, D17 b); the headless smoke path stays neutral/
   modern (the hashed-trajectory surface owns no present loop or
   mapper); presentation options stay OUT of ModeConfig (D200
   layering). (d) GATE: p6-present-loop-wiring wired as the FIFTH
   P6 required_gates entry — command = bedlam-shell --lib,
   --release --locked --offline, hermetic. Verified first-hand:
   bedlam-shell --lib 47/0 (+5 wiring tests; was 42/0 + 1
   pre-existing ignored), bedlam-game --lib 148/0 + bedlam-core
   --lib 147/0 untouched; controls green: canonical_dump_gate
   13/13 (ZERO canonical-chain movement), zone_mission_parity
   5/5, determinism 4/4; check-p6-behavior-catalog OK (catalog
   still empty, R6 satisfied with the fifth gate) + suite OK;
   gates-validator suite OK; fmt + clippy clean on the touched
   crates; MANIFEST clean before AND after every corpus read; the
   bounded --phase P6 validator verdict at 9a96a60: status=passed,
   ALL 5 P6 GATES GREEN, every command rc=0 (report
   .state/p6-presentloop-gates-report.json, head-bound to
   9a96a60); no Ghidra run. Queued: the high-refresh
   camera/scroll interpolation as the new head.
   NOTE (watchdog repair 1787913801, D206): the worker's model
   client exited rc=1 AFTER this completion rewrite — a
   post-completion transport death, no work lost (9a96a60 + this
   bookkeeping both PUSHED, strict parser rc=0 on the rewritten
   queue); the structured client-error failure was adjudicated
   replaced-task and item 1 above stands untouched, READY.
18. DONE (2026-08-28, claim 1 — commit b4babe3 by worker e56b4ef6,
   PUSHED): P6 axis-consumer unit #2 `p6-control-scheme-surface` —
   the control-scheme purist axis's FIRST CONSUMER at the
   PLATFORM/INPUT seam (PLAN §6 + D201/D204): the axis arm selects
   the INPUT MAPPING POLICY, never the frame contract.
   (a) bedlam-shell/src/input.rs: ControlScheme (Modern/Classic)
   selected from the immutable mode via ControlScheme::for_mode
   (the control-scheme arm only; the timing-lock arm never moves
   it — axis independence pinned). MODERN maps physical keys
   through the caller's remappable Bindings table (the D38 seam
   table as data: WASD + arrows move, 1-4 weapon hotkeys, Escape,
   Space/Enter advance — full remap: bind/unbind/replace), maps
   the WHEEL to ZOOM (a presentation-bucket accumulator consumed
   via ShellInput::take_zoom, NEVER the sim input; replaces the
   provisional D38 wheel->Up/Down mapping) and maps the default
   GAMEPAD table (dpad moves, South fires, East backs, Start
   confirms; analog-stick conversion deliberately absent — future
   modern work, never classic). CLASSIC is the FIXED original EXW
   scheme, re-anchored verified RE-EXW-INPUT secs 5-7: keyboard =
   hotkeys/volume/pause/any-key ONLY, gameplay pointing is the
   mouse, Left/Right arrows dead 3-way — among the game-semantic
   slots this seam carries ESC is the ONE original key binding;
   the original digits/M/Space/P semantics target slots the seam
   does not model yet and join with the P2e engine-side button
   map (never invented, D50); wheel + gamepad DEAD in classic
   (the sec 7 control model is exactly KeyEvent/MouseEvent/
   CursorPos); the classic arm IGNORES Bindings (the original
   offered no rebinding). (b) SEAM INERTNESS GENERALIZED (the D201
   property at the mapping boundary): the scheme maps physical
   input to the game-semantic InputFrame BEFORE the sim — the
   frame is the whole contract, so the same InputFrame = the same
   trajectory in BOTH arms, pinned host-side by
   control_scheme_mapping_never_touches_the_hashed_buckets
   (bedlam-game; same frame script with buttons bit 0 held — the
   placeholder payload's hash-visible movement bit — yields the
   identical executed ticks, tick count, state hash and scene hash
   in both arms), while the arms differ UPSTREAM (pinned at the
   shell seam: the same W-hold/click stream maps to UP|WEAPON2
   frames in modern, movement-neutral frames in classic — the
   consumer is real, not inert). The MOUSE PATH is scheme-INVARIANT
   (the original is mouse-driven; modern keeps it). Wheel zoom is
   presentation-bucket ONLY (the D17 b shape). CONFIG-NOT-STATE
   unchanged: FORMAT_VERSION 1, no hash pin moves. (c) The window
   path routes through the scheme-aware ShellInput::
   set_physical_key (the seam is live; the selection is
   default-modern until the p6-present-loop-wiring platform unit
   routes the plumbed mode into it). (d) GATE:
   p6-control-scheme-surface wired as the FOURTH P6 required_gates
   entry — commands = bedlam-shell --lib + bedlam-game --lib, both
   --release --locked --offline, hermetic. Verified first-hand:
   bedlam-shell --lib 42/0 (+9 scheme tests; was 33/0 + 1
   pre-existing ignored), bedlam-game --lib 148/0 (+1), bedlam-core
   --lib 147/0 untouched; controls green: canonical_dump_gate
   13/13 (ZERO canonical-chain movement — the parity paths feed
   InputFrame directly, upstream of the mapper), zone_mission_
   parity 5/5, determinism 4/4, differ_gate 4/4, bedlam-core
   determinism 12/0 + hash_fixture green; check-p6-behavior-
   catalog OK (catalog still empty, R6 satisfied with the fourth
   gate) + suite OK; gates-validator suite OK; workspace cargo
   check clean; fmt + clippy clean on the touched crates; MANIFEST
   clean before AND after every corpus read; the bounded --phase
   P6 validator verdict at b4babe3: status=passed, ALL 4 P6 GATES
   GREEN, every command rc=0 under bwrap containment (report
   .state/p6-controlscheme-gates-report.json, head-bound to
   b4babe3931b2); no Ghidra run. Queued: the present-loop platform
   wiring as the new head (it also selects the shell mapper's
   scheme from the plumbed mode).
19. DONE (2026-08-28, claim 1 — commit c225c81 by worker 458a7e98,
   PUSHED): P6 axis-consumer unit #1 `p6-timing-lock-surface` — the
   timing-lock purist axis's FIRST REAL CONSUMER at the HOST/PRESENT
   seam (PLAN §6 P6 + D200/D201; implementation D203): the axis arm
   selects the PRESENT PACING POLICY, never a Hz. (a) engine/
   bedlam-game/src/host.rs: PresentPacing — Decoupled (the modern
   accumulator-driven present: every host frame presentable,
   zero-tick high-refresh frames included, the shape the shell clock
   bedlam-shell/src/clock.rs feeds) and FrameLocked (the ORIGINAL
   frame-locked present-coupled pacing, RE-anchored [verified,
   RE-EXW-PACER §3 / D16: one sim/render frame per display flip, no
   software frame clock — the FUN_0043d00b loop pass and its
   PresentEnd are ONE event, g_frame_count++ exactly once per flip]:
   a host frame is presentable only when it executed >= 1 logic
   tick). (b) GameHost::present_pacing() reads the timing-lock arm of
   the IMMUTABLE mode; GameHost::should_present() is the gate the
   platform present loop asks each host frame (the boot frame is
   presentable in both arms); a private last_pump_ticks field (D17 b)
   feeds the gate — presentation bucket only, it can never reach the
   sim, the state hash or the scene hash. (c) the LOGIC TICK STAYS
   FIXED at the original rate in BOTH arms; display rate NEVER enters
   the sim or the state hash (Determinism Charter), pinned by
   timing_lock_pacing_never_touches_the_hashed_buckets (same pump
   script = identical executed-tick sequence, sim tick count, state
   hash and scene hash in both arms while should_present differs —
   the consumer is real, not inert); the D17 accumulator is
   pacing-policy-neutral in every arm (SimDriver doc). Test surface =
   the ONE purist toggle, both arms (control-scheme only as the
   axis-independence control), never the feature cross-product; the
   catalog stays EMPTY. (d) GATE: p6-timing-lock-surface wired as the
   THIRD P6 required_gates entry (R6 keeps the scaffold first) —
   commands = bedlam-game --lib + bedlam-core --lib, both --release
   --locked --offline, hermetic. Verified first-hand: bedlam-game
   --lib 147/0 (+5 pacing tests), bedlam-core --lib 147/0; controls
   green BEFORE at clean HEAD c942bd9 AND AFTER: canonical_dump_gate
   13/13 (ZERO canonical-chain movement), zone_mission_parity 5/5,
   determinism 4/4, bedlam-core determinism 12/0 + hash_fixture
   green; check-p6-behavior-catalog OK (catalog empty, R6 satisfied
   with the third gate) + suite 30/30; gates-validator suite 22/22;
   workspace cargo check clean; fmt + clippy clean on the touched
   crates; MANIFEST clean before AND after every corpus read; the
   bounded --phase P6 validator verdict at c225c81: status=passed,
   all 3 P6 gates green, every command rc=0 (report
   .state/p6-timinglock-gates-report.json, head-bound to c225c819f516);
   no Ghidra run. Queued: the control-scheme axis consumer as the new
   head, the present-loop platform wiring second.
20. DONE (2026-08-28, claim 1 — commit 9d39368 by worker 21604df0,
   PUSHED): P6 engine unit `p6-modeconfig-seam` — the FIRST engine
   unit behind the p6-modernization-scaffold contract (PLAN §6 P6 +
   D200; implementation D201): the ONE immutable ModeConfig landed,
   injected at sim construction. (a) engine/bedlam-core/src/mode.rs
   (NEW): ModeConfig with private fields and NO &mut self method —
   the only way to a different mode is the consuming with(axis, arm)
   builder returning a NEW value (a mode change is a new sim);
   default = MODERN (the plan default), CLASSIC preset, per-axis
   mixing; the initial purist toggle set = exactly the two
   plan-named FEEL-CONTESTED axes with the concrete ids pinned
   (PuristToggle::TimingLock = "timing-lock", ControlScheme =
   "control-scheme" — reserved namespace, catalog purist_toggle ids
   must not collide; from_id fails closed). (b) INJECTION: the mode
   rides SimConfig.mode into Sim::new, is carried privately, and is
   read-only at Sim::mode()/SimDriver::mode()/GameHost::mode() — no
   setter at any layer. CONFIG-NOT-STATE: not hashed, not
   serialized (FORMAT_VERSION unchanged, STATE_LEN + every P5 hash pin
   byte-stable); a restore ADOPTS the expected SimConfig's mode.
   Presentation/platform options stay OUTSIDE (no Hz/vsync/resolution
   knob enters ModeConfig or the sim in any arm). The seam lands
   INERT — neither axis has an in-sim consumer yet, so the same seed
   + input stream yields the identical hashed trajectory in both
   arms (pinned by test mode_is_config_not_state_the_seam_lands_
   inert). (c) GATE: p6-modeconfig-seam wired as the SECOND P6
   required_gates entry (R6 keeps the scaffold first) — commands =
   bedlam-core --lib + bedlam-game --lib, both --release --locked
   --offline, hermetic (no corpus, no writable); docs updated
   (P6-MODERNIZATION.md §1 implementation status + §5 gate note,
   D201). Verified first-hand: bedlam-core --lib 147/0 (147 = 146 +
   new mode/sim/frame tests), bedlam-game --lib 142/0 (+1 host seam
   test), bedlam-core determinism 12/0 + hash_fixture green (pins
   untouched), bedlam-render determinism green, mode doctest green;
   fmt clean + clippy clean on every touched file (the 7 D151
   bedlam-core warnings pre-exist, untouched); canonical chains
   UNMOVED before AND after: canonical_dump_gate 13/13 +
   determinism 4/4 + zone_mission_parity 5/5 at clean HEAD b625559
   AND at 9d39368; check-p6-behavior-catalog OK (catalog still
   EMPTY, R6/R7 satisfied with the second gate), p6 suite 30/30,
   gates-validator suite 22/22, check-p5-zone-ledger OK 37/37 + its
   suite green; the bounded --phase P6 validator verdict at 9d39368:
   status=passed, both P6 gates green, all 4 commands rc=0 under
   bwrap containment (report .state/p6-seam-gates-report.json,
   head-bound to 9d393682a3ff); MANIFEST clean before AND after
   every corpus read; no Ghidra run. Queued: the timing-lock axis
   consumer as the new head, control-scheme second.
21. DONE (2026-08-28, claim 1 — commit e0bc7fb by worker 6e45232f,
   PUSHED, plus this bookkeeping commit): P6 opener
   `p6-modernization-scaffold` — the modernization CONTRACT scaffold
   landed per PLAN §6 + the D175 pattern (the machine-checkable
   contract BEFORE any behavior change it grades; bounds honored: no
   engine change, no harness change, no Ghidra run). (a) D200 in
   docs/DECISIONS.md decides the ModeConfig seam per PLAN §6
   verbatim: fixes land directly in the engine (no bug-complete-
   faithful core), classic mode = a small purist toggle set covering
   FEEL-CONTESTED items only (timing lock, control scheme, preserved
   catalog entries), mode = ONE immutable ModeConfig injected at sim
   construction, test surface = the purist toggles never the feature
   cross-product, presentation options are NOT mode toggles. (b)
   docs/P6-MODERNIZATION.md commits the bug-triage rubric VERBATIM
   from PLAN §6 (crash/data-loss fixed everywhere; gameplay-coupled
   classic-preserves/modern-fixes; cosmetic fixed in modern; fixed =
   mechanically applying the rubric + regression evidence, not
   vibes), the catalog format spec (schema p6-behavior-catalog-v1,
   fields + the R1-R7 mechanical rules), and the seeding policy. (c)
   docs/P6-BEHAVIOR-CATALOG.toml seeds EMPTY (the honest post-P5
   state: 37/37 green, all catalog_refs empty; entries land only on
   recorded evidence with a repro, observed = original or
   divergence — post-parity an original bug is a faithful
   reproduction, so original observations are the expected dominant
   source). (d) tools/check-p6-behavior-catalog.py +
   tools/test-p6-behavior-catalog.py: the fail-closed checker (30
   cases hermetic, all fail-closed: rubric-as-code over all six
   wrong-class closures, evidence discipline both directions, toggle
   discipline, mission grounding, the BIDIRECTIONAL ledger
   catalog_refs join, manifest scaffold-first + P6-green rules) wired
   as the FIRST P6 required gate in docs/required-gates.toml (P6
   pending with exactly one required gate; reads ONLY committed docs,
   no corpus key). Verified first-hand: checker OK (entries 0, 37
   ledger ids, 0 refs resolve) pre- and post-commit; suite 30/30;
   gates-validator suite 22/22; check-p5-zone-ledger OK 37/37 with
   the edited manifest; controls green BEFORE (clean HEAD 0c81387)
   AND AFTER (commit e0bc7fb): zone_mission_parity 5/5 (27.33s /
   27.20s) + canonical_dump_gate 13/13 (4.50s / 4.45s); the bounded
   HEAD-bound phase verdict at e0bc7fb via validate-required-gates.py
   --phase P6 = status passed, both gate commands rc=0 under bwrap
   containment; MANIFEST clean before AND after every corpus read; no
   canonical-chain movement. Queued: the p6-modeconfig-seam engine
   unit as the new head.
22. DONE (2026-08-28, claim 1 — commit f608207 by worker ec090fa6,
   PUSHED, plus this bookkeeping commit): P5 phase-close
   bookkeeping `p5-phase-close` — the P5 phase status FLIPPED
   pending->green in docs/required-gates.toml (P0-P5 green,
   P6-P7 pending; plan_complete correctly stays false), then the
   bound phase verdict RE-EMITTED at the flip commit with the
   exact P4-shaped command: /usr/bin/python3
   tools/validate-required-gates.py --root . --report
   .state/p5-gates-report.json --phase P5 --phase-output
   .state/P5-COMPLETE — ALL 8 P5 GATES GREEN at f608207 (report
   status=passed, bounded, offline, containment
   bwrap-unshare-net-pid-ro, every command rc=0;
   .state/P5-COMPLETE phase-complete-v1 re-bound to the flip
   commit + manifest sha256 7efb1041..., producer
   required-gates-validator, emitted by the validator itself).
   Pre-flip first-hand checks: check-p5-zone-ledger OK — the
   ledger reads 37/37 (A 1/1, B..F 7/7 each, G 1/1) and the P5
   status-green cross-artifact rule holds; the tracked tree
   stayed clean through the whole HEAD-bound battery (the
   D193/D194 lesson — STATE.md/NEXT.md edits parked until
   after); MANIFEST clean before AND after every corpus read; no
   Ghidra run. The queue then carried the P6 opener as the head
   (the p4-phase-close/5347a37 pattern): p6-modernization-scaffold
   per PLAN §6, so required work stays active.
23. DONE (2026-08-28, claim 1 — substantive commits 0829187 + 65505ea
   by worker ebf6cfca, both PUSHED): P5 `p5-zone-g-disposition` —
   ZONE G CLOSED, THE LEDGER READS 37/37: the LAST ledger mission
   flips green and P5's mission side is DONE (D199); the disposition
   is the FIFTH PURE ZONES-APPEND (the §9..§12 shape) with the ONE
   census-forced seam delta. (a) THE APPEND (0829187): the ZONEG
   ZoneSpec joined the ZONES list after B, C, D, E, F (letter G,
   missions 1..=1, dims 100x25 — the census-pinned NON-SQUARE
   mission, TOT 40004 B re-verified first-hand; committed flows
   NONE) and the SELECT write-pair legs of zone_t1_rules_spot now
   derive from the zone's own mission range (zone G's zone cell 7
   is OUTSIDE the SELECT write arm's 2..=6 domain and no MP file
   ships for G, §7j.73 — zones B..=F exercise the identical legs
   they always did; the write-arm reject domain still checks (7,1)
   loud); the battery: P5GM1A/B/C all 3 flows full declared budgets
   (3/121/49 records), dumps verify, two-run byte identity — NO
   engine gap on the ZONEG mission; zones B (21 + the committed
   S5/S5B/S5C), C (21), D (21), E (21) and F (21) re-verified in
   place. (b) THE FLIP (65505ea, the cross-artifact rule): ledger
   ZONEG-MISSION1 green (catalog_refs = []), the ledger 37/37
   (A 1/1, B..F 7/7 each, G 1/1 — EVERY shipped mission green);
   P5-ZONE-GATES §13 criterion table (linear = m+24, M1 = 25 one
   below the clamp ceiling; the zone-level MISSIONG.BIN 2443943 B
   fetch-chain pin with NO variant caveat — zone G ships no
   mission-number variant bank; the real 1144 B .NME bank; the
   zone-A-shaped DM carve-out); the p5-zone-g gate joins P5
   required_gates (16 gates); the ledger test pin re-baselined
   36/37 to 37/37 + the ZONEG 1/1 line (deliberate, same commit);
   D199. Verified first-hand at the flip commit: ledger OK 37/37 +
   ZONEG 1/1, hermetic 18/18, strict queue parser rc=0,
   zone_mission_parity 5/5 (six zones, 27.43s), canonical_dump_gate
   13/13 zero chain movement, differ_gate 4/4, determinism 4/4,
   mission_load_census green (census stays 37/37), bedlam-core
   hash_fixture + mission_corpus_gate green, fmt + clippy clean on
   the touched crate (the 7 bedlam-core warnings pre-exist from
   D151, untouched), the HEAD-bound validator battery ALL 16 GATES
   PASSED at 65505ea (bounded, offline, incl. p5-zone-g's both
   commands rc=0; the global report's status=failed/plan_complete=
   false is ONLY the pending P5-P7 phase-status semantics, not a
   gate failure — the same documented ZONEB note), MANIFEST clean
   before and after every corpus read, no Ghidra run. Queued: the
   P5 phase-close disposition as the new head (the P4 pattern).
   NOTE (watchdog repair 1787952748, D234): the completion-missing
   failure recorded at 22:47:16Z was NOT a gates failure either —
   it was the ELEVENTH-hour half of the D233 story: D233's own
   repair commit (a89ce5a, 22:45-22:51) landed mid-run of the
   controller's in-flight sealed validation vasjoy4_, so
   complete-from-head's DESIGNED atomic basis re-check correctly
   withheld the verdict ("completion basis changed during
   validation") — and the controller then MISCLASSIFIED that
   designed invalidation as completion-missing, beaconing a fresh
   failure marker whose hourly watchdog repair is FORCED to commit
   (wrapper evidence rules), which kills the next in-flight
   validation in turn: a livelock that could never converge on
   plan-complete-v1. Fixed in the repair commit: nudge.sh's
   terminal completion branch now treats a basis-change rejection
   as the benign sealed-verdict retry it is (log line + exit 0;
   the 600s tick re-validates the new HEAD from scratch), while
   every real rejection (validator rc!=0, wrapper timeout,
   malformed basis) still beacons completion-missing. The
   structured failure (controller-1787950036-2396770, ordinal 1,
   id automation-state, gate automatic-repair) is adjudicated
   required-empty per D234 — the required queue IS empty and stays
   empty; the controller's next complete-from-head run owns the
   global verdict exactly as the completion contract demands.
