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
1. [READY] [id=p6-scaling-options] [gate=p6-scaling-options] P6
   resolution-independence unit per PLAN §6 "Resolution
   independence + GPU rendering ... (nearest/integer default;
   fit/fill/smooth options)": the SCALING SELECTION — expose the
   already-landed bedlam-platform ScaleMode (Integer the parity
   default / Fit / Fill) + FilterMode (Nearest the parity default
   / Linear = smooth) as a platform presentation knob riding
   WindowOptions.present in bedlam-shell, selected by new CLI
   flags — a PURE mapping over plain data with NO purist
   arbitration (the original was a fixed 640x480 DOS framebuffer
   with no scaling mode to preserve; D200 layering — the knob is
   OUT of ModeConfig and selects nothing in the host beyond the
   existing PresentConfig the already-landed GPU scale path
   consumes; both pacing arms accept it identically; the Fill
   cursor-uv handling already exists window-side). NO new RE
   needed (a pure modern platform surface over landed code, zero
   new binary claims — if any RE claim becomes necessary, commit
   the artifact first). Bounds: default = Integer + Nearest
   exactly as shipped (pinned: the derived SimConfig
   bit-identical under every selection — the sim, the ModeConfig
   and every hash untouched by construction; the canonical
   640x480 indexed frame + palette ride unchanged, goldens stay
   resolution-agnostic; the headless path owns no surface so the
   flags are noted + ignored there, the --fullscreen posture);
   bedlam-shell only, no engine change; catalog stays EMPTY;
   wire gate p6-scaling-options as the ELEVENTH P6
   required_gates entry; fmt + clippy on touched crates;
   gates-validator green; MANIFEST clean; no Ghidra run; own
   Nudge-Worker trailer.

## Done
1. DONE (2026-08-28, claim 1 — commits 63d58ac + bece1cf + 9b2599f
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
2. DONE (2026-08-28, claim 1 — commits f49315f + aa6673c + 1b42327
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
3. DONE (2026-08-28, claim 1 — commit 8784da1 by worker 7aed939f,
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
4. DONE (2026-08-28, claim 1 — commit 44c6f2d by worker 754e7c94,
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
5. DONE (2026-08-28, claim 1 — commits fe5bf72 + 37aaddf by worker
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
6. DONE (2026-08-28, claim 1 — commit 9a96a60 by worker 2a90eb65,
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
7. DONE (2026-08-28, claim 1 — commit b4babe3 by worker e56b4ef6,
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
8. DONE (2026-08-28, claim 1 — commit c225c81 by worker 458a7e98,
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
9. DONE (2026-08-28, claim 1 — commit 9d39368 by worker 21604df0,
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
10. DONE (2026-08-28, claim 1 — commit e0bc7fb by worker 6e45232f,
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
11. DONE (2026-08-28, claim 1 — commit f608207 by worker ec090fa6,
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
12. DONE (2026-08-28, claim 1 — substantive commits 0829187 + 65505ea
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
