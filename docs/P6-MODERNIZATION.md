# Bedlam (1996) — P6 Modernization: ModeConfig Seam, Triage Rubric, Behavior Catalog

**Scope:** the P6 opener per `docs/PLAN.md` §6 (P6). This document pins
(1) the ModeConfig seam decision VERBATIM from the plan, (2) the bug-triage
rubric VERBATIM from the plan (normative for every catalog entry),
(3) the committed original-behavior catalog format
(`docs/P6-BEHAVIOR-CATALOG.toml`, schema `p6-behavior-catalog-v1`) that the
P5 mission ledger's `catalog_refs` feed, (4) the catalog seeding policy, and
(5) how the first P6 required gate (`p6-modernization-scaffold`) lands in
`docs/required-gates.toml` BEFORE any behavior change (the D175 scaffold
pattern: the machine-checkable contract precedes the work it checks).

**Provenance:** unit `p6-modernization-scaffold` (D200). No engine change,
no harness change, no Ghidra run; this unit lands decisions + contract
artifacts only. The P5 ledger fact cited below (37/37 green, all
`catalog_refs` empty) is machine-verified by `tools/check-p5-zone-ledger.py`
(the `p5-zone-gate-scaffold` gate).

**Confidence tags:** the quotes in §1/§2 are VERBATIM plan text (byte-bound
by git history; amend here whenever the plan text changes — the checker does
NOT textually bind these quotes, the git history does). The catalog format,
seeding policy, and gate wiring are a DECISION (D200), not an RE claim. The
post-P5 seeding fact (empty refs) is VERIFIED (ledger + checker, 2026-08-28).

---

## 1. The ModeConfig seam (VERBATIM from PLAN §6, P6)

The following is quoted byte-for-byte from `docs/PLAN.md` §6
("P6 — Modernization (default = modern; classic available)"). It is the
architecture decision P6 implements; D200 makes it binding:

> Architecture (simplified by the 99% target): fixes land directly in the engine —
> there is no bug-complete-faithful core to preserve. Classic mode shrinks to a small
> purist toggle set covering feel-contested items only (timing lock, control scheme,
> selected catalog entries classified for preservation by a deterministic rubric and
> decision record, with regression tests). Mode is one immutable ModeConfig
> injected at sim construction; test surface = the purist toggles, not 2^features.

Consequences D200 records as binding (all grounded in the quote above and
the Determinism Charter in PLAN §3):

- **Fixes land directly in the engine.** There is no dual-core split (no
  "bug-complete-faithful core + modern shell"): the one engine carries every
  fix, and classic mode selects original behavior through the small toggle
  set only.
- **Classic mode = a small purist toggle set**, exactly three sources:
  (i) the timing lock, (ii) the control scheme, and (iii) selected
  original-behavior catalog entries classified for preservation by the
  deterministic rubric (§2) with a decision record and regression tests.
  Nothing else is a classic/modern axis.
- **Mode is ONE immutable `ModeConfig` injected at sim construction.** It is
  never mutated mid-run; a mode change is a new sim. The purist toggles it
  carries are the catalog's `purist_toggle` ids (§3) plus the two plan-named
  axes (timing lock, control scheme) whose concrete ids land with the first
  P6 engine unit that implements the seam.
- **The test surface is the purist toggles, never the full feature
  cross-product.** P6 tests parameterize over the small toggle set (the
  closed catalog entries that carry a `purist_toggle`), never over
  2^(modern features).
- **`ModeConfig` covers sim-behavior-affecting choices only.** Presentation
  and platform options (window mode, vsync, resolution, scaling mode, HD
  pack, refresh rate) are NOT mode toggles: per the plan, display rate
  NEVER enters the sim (Determinism Charter); the logic tick stays fixed at
  the original rate in every mode.

Bounds of this unit: the seam is DECIDED here; no engine code lands with it
(the first P6 engine unit implements `ModeConfig` and the toggle plumbing).

**Implementation status (D201, 2026-08-28, gate `p6-modeconfig-seam`):**
the seam is IMPLEMENTED in `engine/bedlam-core/src/mode.rs` —
`ModeConfig` rides `SimConfig.mode` into `Sim::new` (sim construction),
is carried unmutated and read-only through `Sim::mode()` /
`SimDriver::mode()` / `GameHost::mode()` (no setter at any layer; a mode
change is a new sim). Default = `ModeConfig::MODERN` (PLAN §6). The two
plan-named axis ids are pinned: **`timing-lock`** and **`control-scheme`**
(`PuristToggle::id()`, fail-closed `from_id`). The ids are a RESERVED
namespace: catalog `purist_toggle` ids must not collide with them
(checker-side enforcement lands with the first catalog entry). The axes
are config, not state: not hashed, not serialized (FORMAT_VERSION
unchanged); a restore adopts the expected config's mode. The unit lands
inert — neither axis has an in-sim consumer yet, so the canonical chains
are byte-identical under the modern default (pinned by
`canonical_dump_gate` and the seam's inertness test).

**Implementation status (D203, 2026-08-28, gate
`p6-timing-lock-surface`): the timing-lock axis's FIRST CONSUMER** —
present pacing at the HOST/PRESENT seam (`engine/bedlam-game/src/
host.rs`). `GameHost::present_pacing()` maps the axis arm to a
`PresentPacing` policy — MODERN = `Decoupled` (the accumulator-driven
present: every host frame is presentable, zero-tick high-refresh frames
included — the PLAN §6 high-refresh present the shell clock
`bedlam-shell/src/clock.rs` feeds), CLASSIC = `FrameLocked` (the original
frame-locked present-coupled pacing, RE-EXW-PACER §3 [verified / D16]:
one sim/render frame per display flip, no software frame clock — a host
frame is presentable only when it executed ≥ 1 logic tick). The gate the
platform asks per host frame is `GameHost::should_present()`; before the
first pump the pre-rendered boot frame is presentable in both arms. The
policy is a POLICY, never a Hz: the logic tick stays FIXED at the
original rate in BOTH arms, and the decision rides the un-hashed
presentation bucket only (a private `last_pump_ticks` field, D17 b —
pinned by `timing_lock_pacing_never_touches_the_hashed_buckets`: the
same pump script yields the identical executed-tick sequence, sim tick
count, state hash and scene hash in both arms while `should_present`
differs). The accumulator itself (D17) is pacing-policy-neutral in every
arm. The control-scheme axis gained its consumer with the NEXT unit
(D204, below); the catalog stays empty (a plan-named axis unit is not
a catalog entry). The platform loop wiring (the window shell consuming
`should_present`, mode plumbing through the shell config) is a
LATER P6 unit — this unit lands the seam-side policy and its contract.

**Implementation status (D204, 2026-08-28, gate
`p6-control-scheme-surface`): the control-scheme axis's FIRST
CONSUMER** — the input mapping policy at the PLATFORM/INPUT seam
(`engine/bedlam-shell/src/input.rs`). `ControlScheme`
(MODERN/CLASSIC) is selected from the immutable `ModeConfig` via
`ControlScheme::for_mode` (the `control-scheme` arm only; the
timing-lock arm never moves it). MODERN maps physical keys through
the caller's remappable **`Bindings`** table (the D38 seam table
as data: WASD + arrows move, 1-4 weapon hotkeys, Escape,
Space/Enter advance — "full remap": bind/unbind/replace), maps the
**wheel to ZOOM** (a presentation-bucket accumulator consumed via
`ShellInput::take_zoom`, never the sim input — it replaces the
provisional D38 wheel→Up/Down mapping) and maps a default
**gamepad** table (dpad moves, South fires, East backs, Start
confirms; analog-stick conversion is deliberately absent future
modern work). CLASSIC is the FIXED original EXW scheme,
re-anchored [verified, RE-EXW-INPUT secs 5-7]: keyboard =
hotkeys/volume/pause/any-key ONLY, gameplay pointing is the mouse,
Left/Right arrows dead 3-way — among the game-semantic slots this
seam carries **ESC is the one original key binding**; the original
digits/M/Space/P semantics target slots the seam does not model
yet and join with the P2e engine-side button map (never invented,
the D50 rule); the wheel and gamepad are DEAD in classic (the §7
control model is exactly KeyEvent/MouseEvent/CursorPos), and the
classic arm ignores `Bindings` (the original offered no
rebinding). **Seam inertness generalized (the D201 property at the
mapping boundary):** the scheme maps physical input to the
game-semantic `InputFrame` BEFORE the sim — the frame is the whole
contract, so the same InputFrame = the same trajectory in both
arms (pinned host-side by
`control_scheme_mapping_never_touches_the_hashed_buckets`, with
`buttons` bit 0 held so a scheme leak fails loud), while the arms
differ UPSTREAM in what a physical stream maps to (pinned at the
shell seam: the same W-hold/click stream → UP|WEAPON2 frames in
modern, movement-neutral frames in classic). The mouse path is
scheme-INVARIANT. The canonical chains are untouched (the parity
paths feed InputFrame directly, upstream of the mapper). The
catalog stays empty. The platform plumbing (shell config → mode →
`GameHost` + the mapper, classic/modern selectable at the platform
level) is the NEXT P6 unit (`p6-present-loop-wiring`); until then
`ShellInput::new()` defaults to the modern scheme and the window
path already routes through the scheme-aware
`ShellInput::set_physical_key` (the seam is live, the selection
default-modern).

**Implementation status (D205, 2026-08-28, gate
`p6-present-loop-wiring`): the platform wiring — BOTH consumers
connected to the REAL platform loop**
(`engine/bedlam-shell/src/window.rs` + `main.rs`). The platform
selects ONE immutable `ModeConfig` — `WindowOptions.mode` (default
= modern; the binary's `--classic` selects the `CLASSIC` preset)
— and routes it into BOTH construction sites:
`host_sim_config` (the mode rides `SimConfig` into
`GameHost::new` as config, never state) and `shell_input_for`
(the SAME plumbed mode selects the mapper's `ControlScheme` via
`ControlScheme::for_mode` — the D204 consumer's platform
selection; until this unit the window path ran default-modern).
The WINDOW PRESENT LOOP honors the D203 gate through
**`present_due`** (pure delegation to `GameHost::should_present`,
consulted at the present site): MODERN presents every vsync
(zero-tick high-refresh frames recompose and present too);
CLASSIC holds the previously presented image on zero-tick host
frames — the original frame-locked present-coupled pacing, so on
faster hosts the visible refresh follows the fixed logic tick,
never the display rate. Loop liveness is preserved by gating at
the PRESENT SITE ONLY: the redraw request stays unconditional, so
the vsync-paced loop keeps pumping in both arms (gating the
request itself would stall a quiet classic loop — there would be
no event to wake it). The shell fixed-step clock/pump contract
and the hashed trajectory are untouched (pinned shell-side by
`platform_mode_plumbing_never_touches_the_hashed_trajectory`: the
same pump script through hosts built from both platform options
yields the identical executed-tick sequence, sim tick count, state
hash, scene hash and frame parity hash). The headless smoke path
stays neutral/modern by construction — it is the
hashed-trajectory surface and owns no present loop or mapper. The
catalog stays empty (a plan-named wiring unit is not a catalog
entry).

**Implementation status (D207, 2026-08-28, gate
`p6-high-refresh-interpolation`): the present-quality unit — the
composition policy of the modern decoupled present.** RE first:
`docs/RE-EXW-CAMERA.md` (committed before the implementation)
collects the EXW camera/scroll traffic — the Q5 pixel pair
`_DAT_004edde4/8`, its tick-boundary writers (the `robots()`
recenter @`0x40b875..0x40b8c5`, the `FUN_004245c9` chase-camera
override), its frame-path readers (`FUN_00403938` the viewport
renderer, `FUN_00401107` the present fine-offsets) — and records
the NEGATIVE the policy rests on: NO sub-tick camera interpolation
exists anywhere in the decoded original (the Q5 fixed point is
sub-PIXEL precision within integer tick updates, never sub-TICK).
The implementation lands the PLAN §6 composition policy at the
HOST/PRESENT seam: `GameHost` stages a presentation-bucket
`prev_sim` (the sim as of the pump BEFORE the last executed tick;
D17 b — never hashed, never serialized; `Sim` gains a `Clone`
derive for exactly this), the pump path still renders the PURE
parity configuration (prev=None, alpha=0 — zero canonical-chain
movement), and `GameHost::recompose(alpha)` re-renders the
presented frame from LATEST state with the camera lerped
`(prev → cur) · alpha` — **interpolate CAMERA/SCROLL ONLY**
(sprites stay grid-quantized; the sub-pixel blitter stays
default-off and out of scope), selected by `camera_interpolation()`
= the Decoupled arm ONLY: the CLASSIC frame-locked arm is a NO-OP
(it presents only after a tick — the exact tick-state camera of
the original, nothing to interpolate). The alpha is the shell
clock's accumulator fraction (`FixedStepClock::fraction`,
`banked_ns / PUMP_PERIOD_NS` saturated — the one float in the
integer clock, presentation-side only), and the present site pairs
it with the gate (`present_camera_alpha`) before every upload:
zero-tick high-refresh frames now carry the interpolated camera
sweep, while the 60 Hz steady state reads fraction 1.0 — the
interpolated camera IS the parity camera, so the modern arm adds
no latency on the original display class and the sweep only
becomes visible when the display outpaces the fixed tick rate.
The shell fixed-step clock/pump contract and the hashed trajectory
are untouched (pinned host-side by
`camera_interpolation_never_touches_the_hashed_buckets` and
platform-side by
`present_site_recompose_never_touches_the_hashed_trajectory`: the
same pump script with the modern arm recomposing at the clock
fractions = identical executed ticks, tick count, state hash,
scene hash). The catalog stays empty (a plan-named composition
unit is not a catalog entry).

## 2. The bug-triage rubric (VERBATIM from PLAN §6, P6)

The following is quoted byte-for-byte from `docs/PLAN.md` §6 (P6). It is
normative for every catalog entry; §3 encodes it as machine rules:

> Bug triage rubric (per catalog entry): crash/data-loss → fix everywhere;
> gameplay-coupled → classic preserves / modern fixes; cosmetic → fix in modern.
> Fixed = deviation from the catalog established by mechanically applying the rubric
> and recording regression evidence — not vibes.

The rubric as a decision table (the mechanical form §3 enforces):

| Catalog `class`        | Fix policy                                            | Terminal disposition      |
|------------------------|-------------------------------------------------------|---------------------------|
| `crash-data-loss`      | fixed everywhere (modern AND classic)                 | `closed-fix-everywhere`   |
| `gameplay-coupled`     | classic preserves / modern fixes (a purist toggle)    | `closed-preserve-classic` |
| `cosmetic`             | fixed in modern (no toggle; classic is not a look)    | `closed-fix-modern`       |

"Fixed" means: the deviation from the original is established by
mechanically applying the rubric to a catalog entry AND recording regression
evidence — the entry's `evidence` field names the test/gate/document anchor
that proves the fix (for `closed-preserve-classic` entries, the regression
evidence must cover BOTH arms: the modern fix and the classic preservation
through the entry's `purist_toggle`). No vibes.

## 3. The original-behavior catalog format (`p6-behavior-catalog-v1`)

Artifact: `docs/P6-BEHAVIOR-CATALOG.toml`, TOML (the repo convention —
`required-gates.toml`, `watches.toml`, the P5 ledger; stdlib `tomllib`, no
deps), schema string `p6-behavior-catalog-v1` fail-closed. One `[[entry]]`
per cataloged original behavior:

| Field           | Type   | Rule |
|-----------------|--------|------|
| `id`            | str    | unique, non-empty, whitespace-free — the target the P5 ledger's `catalog_refs` point at |
| `title`         | str    | non-empty one-line summary |
| `class`         | str    | one of `crash-data-loss`, `gameplay-coupled`, `cosmetic` (the rubric §2) |
| `observed`      | str    | one of `original` (repro'd observation of the original game), `divergence` (repro'd divergence of our engine from the original) |
| `repro`         | str    | non-empty deterministic repro / oracle evidence pointer (scenario, command, doc §) |
| `missions`      | [str]  | non-empty, duplicate-free, every id a `docs/P5-MISSION-LEDGER.toml` mission id ("affected missions") |
| `disposition`   | str    | one of `open`, `closed-fix-everywhere`, `closed-fix-modern`, `closed-preserve-classic` |
| `evidence`      | str    | non-empty iff closed (the regression-evidence anchor: test id / gate / doc §); empty or absent on open entries |
| `purist_toggle` | str    | present iff `closed-preserve-classic`; non-empty, whitespace-free, unique across the catalog — the `ModeConfig` toggle id that preserves the original behavior in classic mode |
| `provenance`    | str    | non-empty: DECISIONS D-id and/or RE-notes anchor + confidence tag (VERIFIED/LIKELY per repo convention) |

Mechanical rules (fail-closed, `tools/check-p6-behavior-catalog.py`):

- **R1 rubric-as-code:** a CLOSED entry's disposition must be the terminal
  disposition of its class (§2 table). An `open` entry may carry any class
  (observed + classed, fix not yet implemented/evidenced).
- **R2 evidence discipline:** closed ⇒ `evidence` non-empty; open ⇒ no
  evidence (one source of truth — an entry is closed exactly when its
  regression evidence exists).
- **R3 toggle discipline:** `purist_toggle` present iff the disposition is
  `closed-preserve-classic`, and unique across the catalog (one toggle per
  preserved behavior, one behavior per toggle).
- **R4 mission grounding:** `missions` ⊆ the P5 ledger's mission ids. The
  ledger (itself corpus-pinned by the P5 checker) is the single mission
  identity source; this checker does NOT re-enumerate `game-data/`.
- **R5 the P5 feed joins:** every `catalog_refs` value in
  `docs/P5-MISSION-LEDGER.toml` resolves to a catalog entry id (bidirectional
  with R4 — the plan's "feeds P6 triage" made mechanical).
- **R6 scaffold-first manifest:** if `docs/required-gates.toml` P6
  `required_gates` is non-empty, its FIRST entry is `p6-modernization-scaffold`
  and a `[[gate]]` with that id exists (P6 behavior gates can never be wired
  ahead of the contract that grades them — the D175 rule's P6 analogue).
- **R7 phase-gate consistency:** manifest P6 `status = "green"` requires ZERO
  open catalog entries (P6 cannot close with untriaged behaviors; necessary,
  not sufficient — the full P6 exit is PLAN §6).

Layering (one source of truth per fact): ledger schema/corpus binding =
`tools/check-p5-zone-ledger.py`; mission identity for THIS checker = the
ledger; rubric/toggle/manifest-P6 = this checker. The checker reads only
committed docs — it performs NO corpus read (hermetic; `game-data/` never
appears in `tracked_paths`/`corpus`, the never-git-tracked rule).

## 4. Seeding policy (D200)

The catalog seeds **EMPTY**, and both evidence-backed entry sources are
accepted:

- All 37 ledger missions closed green with `catalog_refs = []`
  (machine-verified). P5 parity work recorded zero divergences and zero
  repro'd original-behavior observations worth classifying — the empty
  catalog is the honest post-P5 state (the D175 "0/37 is the honest
  scaffold state" principle).
- Entries land ONLY on recorded evidence with a `repro`:
  `observed = "original"` (an observation of the original game through the
  pinned oracles, an 8street navigation reference re-anchored to EXW/EXD
  addresses per repo policy, or an RE-verified mechanism with doc anchor +
  confidence) or `observed = "divergence"` (a repro'd divergence of our
  engine from the original, found during P6+ work). Speculative or
  retrospective seeding — forum posts, unanchored memory, "probably a bug"
  — is forbidden.
- Why `original` observations must be first-class: after P5 parity our
  engine FAITHFULLY REPRODUCES original behaviors, so an original bug
  surfaces as NO divergence at all. A divergence-only policy would starve
  the catalog of exactly the feel-contested items classic mode exists to
  preserve. The expected dominant entry source is therefore
  `observed = "original"`.

## 5. Gate wiring (the first P6 required gate)

Per the D175 pattern the contract lands before any behavior change:

- `docs/required-gates.toml` P6 `required_gates = ["p6-modernization-scaffold"]`
  (the FIRST entry; R6 enforces it stays first once more P6 gates land).
- Gate commands = the fail-closed checker + its hermetic test suite
  (`tools/check-p6-behavior-catalog.py`, `tools/test-p6-behavior-catalog.py`),
  `tracked_paths` = this doc, the catalog, both tools, and the manifest.
  No `corpus` key; no `writable` (the suite fixtures live under HOME, the
  validator scratch convention).
- The gate validates the CONTRACT (format + rubric-as-code + joins), not P6
  completion: it is green from the moment it lands (0 entries is the honest
  scaffold state). P6 status stays `pending` until the phase's actual exit.
- Later P6 gates (the ModeConfig seam implementation, catalog entries +
  regression evidence, modernization surfaces per PLAN §6) land as evidence
  lands, each behind the scaffold. The seam implementation gate
  `p6-modeconfig-seam` landed 2026-08-28 as the SECOND P6 required gate
  (D201). The timing-lock axis-consumer gate `p6-timing-lock-surface`
  landed 2026-08-28 as the THIRD P6 required gate (D203; commands =
  bedlam-game --lib + bedlam-core --lib, both --release --locked --offline,
  hermetic — the host present-seam suite carries the pacing tests). The
  control-scheme axis-consumer gate `p6-control-scheme-surface` landed
  2026-08-28 as the FOURTH P6 required gate (D204; commands =
  bedlam-shell --lib + bedlam-game --lib, both --release --locked
  --offline, hermetic — the input-seam suite + the host hash pin). The
  platform wiring gate `p6-present-loop-wiring` landed 2026-08-28 as the
  FIFTH P6 required gate (D205; command = bedlam-shell --lib,
  --release --locked --offline, hermetic — the platform wiring suite:
  mode plumbing into host + mapper, the present-gate cadence pin both
  arms, and the trajectory pin). The present-quality composition gate
  `p6-high-refresh-interpolation` landed 2026-08-28 as the SIXTH P6
  required gate (D207; commands = bedlam-game --lib + bedlam-shell
  --lib, both --release --locked --offline, hermetic — the host
  composition-policy suite + the clock-fraction/present-site wiring
  suite).

## 6. P6 acceptance surface (pointer, not re-statement)

The full P6 phase definition (time-based simulation + high-refresh present,
modern controls, the rubric, resolution independence/GPU, optional HD asset
pipeline, QoL + feel proxies) is PLAN §6 (P6) — this doc does not restate it
beyond the §1/§2 verbatim quotes it operationalizes. Every P6 unit cites the
plan paragraph it implements; divergences from the plan are DECISIONS.md
entries, never silent.
