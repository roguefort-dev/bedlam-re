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
1. [READY] [id=p6-high-refresh-interpolation] [gate=p6-high-refresh-interpolation] P6
   present-quality unit per PLAN §6 "time-based simulation" — the
   camera/scroll interpolation of the modern decoupled present:
   zero-tick high-refresh host frames currently recompose from
   latest state only (the D203/D205 shape); this unit lands the
   PLAN §6 composition policy — interpolate CAMERA/SCROLL ONLY
   between the last executed logic tick and the present (the
   accumulator fraction), NEVER sprite positions (grid-quantized
   1996 sprites had no sub-pixel positions; interpolating them
   manufactures motion the original never showed; the sub-pixel
   blitter stays default-off and out of scope), classic arm
   unchanged (the frame-locked pacing presents only after a tick —
   nothing to interpolate). RE first: decode where the EXW
   scroll/camera state lives in the frame path, write committed RE
   notes anchored to EXW/EXD addresses, then implement as a
   presentation-bucket policy selected from the timing-lock arm of
   the plumbed mode. Bounds: the shell fixed-step clock/pump
   contract and the hashed trajectory stay untouched — no
   canonical-chain movement, no harness change; test surface stays
   the ONE purist toggle both arms where practical; catalog stays
   EMPTY; wire gate p6-high-refresh-interpolation as the SIXTH P6
   required_gates entry; fmt + clippy on touched crates;
   gates-validator green; MANIFEST clean; no Ghidra run; own
   Nudge-Worker trailer.
## Done
1. DONE (2026-08-28, claim 1 — commit 9a96a60 by worker 2a90eb65,
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
2. DONE (2026-08-28, claim 1 — commit b4babe3 by worker e56b4ef6,
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
3. DONE (2026-08-28, claim 1 — commit c225c81 by worker 458a7e98,
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
4. DONE (2026-08-28, claim 1 — commit 9d39368 by worker 21604df0,
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
   setter at any layer. CONFIG-NOT-STATE: not hashed, not serialized
   (FORMAT_VERSION unchanged, STATE_LEN + every P5 hash pin
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
5. DONE (2026-08-28, claim 1 — commit e0bc7fb by worker 6e45232f,
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
6. DONE (2026-08-28, claim 1 — commit f608207 by worker ec090fa6,
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
7. DONE (2026-08-28, claim 1 — substantive commits 0829187 + 65505ea
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
