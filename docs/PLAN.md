# Bedlam (1996) — Decompilation & Modern Reimplementation Plan (FINAL v3 — panel-reviewed)

Panel: 4-reviewer multi-model review (GLM-5.3 Paranoid, Kimi Architect, GLM-5.3
Logician, GLM-5.3 Red Team) + judge verification against repo state. Verdict: strategy
sound; 2 operational blockers fixed at P0 (corpus checksum manifest, derived-output
git hygiene); canon, differential-testing, determinism, and bug-triage policies added.

Goal: a stable, smooth, cross-platform (Linux→Windows→macOS) Rust reimplementation of
Bedlam (GT Interactive / Mirage, 1996) that runs the original assets, fixes the game
slowdowns/quirks/bugs, adds modern controls (WASD, 1-4 weapon hotkeys, rebinding),
switches from frame-locked to time-based simulation, and stays testable and debuggable
throughout so the project never gets stuck.
Parity target: ~99% the same game (floor ~95%) — operationalized as the tiered parity
budget in section 0b, NOT bit-exact full parity. The divergence allowance is spent
deliberately: on the requested fixes and modernization, and on imperceptible internals.

Companion docs: GROUNDWORK.md (verified file facts), RESEARCH.md (verified tooling
facts + 8street prior work). MANIFEST.sha256 = integrity manifest of game-data/.

## 0. Canon and oracles (the reference frame — decided, not improvised)

| Question | Answer |
|---|---|
| Which build is the game? | BEDLAM.EXW (Win95/DirectX) is canonical for logic, assets, behavior. It is the primary RE target and the richest asset set (all .SMK movies + all languages — per 8street README). |
| DOS build (EXD) role | Canon for hardware-coupled behavior only: HMI audio init rates, VESA/palette quirks, PIT timing, frame-lock constants. Every EXW↔EXD divergence discovered goes into a standing docs/DIVERGENCES.md table (behavior, addresses) so ambiguity becomes lookup, not archaeology. |
| Goldens source | Reference captures come from the canonical build under a pinned Wine prefix; DOSBox-X/-staging (pinned version + config) covers DOS-canonical behavior and is the fallback where Wine DDraw fails. |
| 8street/Bedlam role | Navigation + hypothesis generation only — NOT a behavior oracle. It is a known-deviant build (44.1kHz mixer vs original 11kHz, unspecified bug fixes, crash fixes — from its own README). Its deviations list seeds docs/DIVERGENCES.md. Its crash-fix list is a lead list, never triage truth. Pin the exact commit hash we consult. |
| 8street instrumented build | Allowed as a test-only comparator: recompile it with state-dump hooks to emit ground-truth state at tick boundaries for differential testing (see Testing). Does not change the not-a-porting-source rule. |
| Tiebreak order | EXW disassembly > black-box observation of EXW > EXD (for its canon categories) > 8street (hints only). |

## 0b. Parity budget (~99% target, ~95% floor)

Divergence is budgeted by perception category, never as a global average (a global
average hides concentrated divergence: wrong movement feel is 1% of features and 100%
of the experience).

| Tier | Contents | Bar |
|---|---|---|
| T1 exact | Asset formats; game rules; weapons/damage/economy/AI-behavior data and tables; mission logic/flow; UI flow | Implemented exactly (semantics verified vs RE + 8street); defects are bugs |
| T2 perceptual | Rendering, palette, audio mix, timing feel, input latency | Perceptual thresholds + owner feel sign-off; no pixel/tick equality required |
| T3 free divergence | Exact RNG streams (recover mechanics + distributions, not the bit stream); internal numeric representation (integer math for OUR determinism, not to mirror Watcom rounding); frame-exact AI micro-reactions; spawn tick ordering; every catalogued bug fix and modernization feature | Only statistical/observational equivalence; these ARE the 1-5% |

Consequences:
- Differential harness role changes from all-zone tick-parity gate to (a) divergence
  meter, (b) structural-error catcher on the slice + targeted spots, (c) regression
  tripwire. Tick-level completeness across all 37 zones is no longer required.
- The original-behavior catalog scopes down to observable gameplay behavior only
  (what players can notice), not internal quirks.
- The classic-mode toggle set shrinks to feel-contested items only (timing lock,
  control scheme, a few purist flags) — see P6.
- 8street is precedent that this fidelity level reads as the same game to players.

## 1. Prior work (8street/Bedlam) — policy with teeth

Use: (1) reading reference to answer semantic questions fast; (2) test-only instrumented
comparator; (3) feasibility proof. Do NOT transliterate its C++ (a 1:1 frame-locked
32-bit reconstruction — the opposite of our target, and porting it is a
derivative-of-derivative). Anti-slide governance (mechanical, not aspirational):
- Fact policy: a spec claim is normative only when anchored [addr EXW/EXD @VA] or
  [black-box observation]. 8street citations are non-normative hints. A facts ledger
  tracks anchored/total ratio; phase gates fail if unanchored gameplay constants remain
  (P4 slice: 100% of load-bearing constants anchored; P5: gate per zone).
- Two-role agent split (cheap with AI agents): the session that reads
  disassembly/8street writes specs only; a different session writes engine code from
  the spec. No decompiled or 8street code enters the repo, ever.
- Every borrowed 8street fact is re-anchored to an address via batched anchoring
  sprints (not left as a later chore).
- Legal: engine-only repo; no assets, no asset-derived dumps (game-data/ and derived/
  git-ignored; MANIFEST.sha256 holds hashes only); the 8street author IDA database is
  used for address cross-checks only, never ingested; spend one afternoon establishing
  current rights-holder lineage, record in DECISIONS.md; keep unofficial personal-use
  framing. Reading RE material and writing specs-from-facts is the honest posture (like
  most surviving source ports) — do not overclaim clean-room.

## 2. Strategy: RE-informed clean reimplementation

NOT byte-identical decompiled C; NOT an 8street port. Watcom register CC (EAX/EDX/EBX/
ECX, callee cleans) + 1996 codegen make auto-decompilers unreliable, and the goal is
behavioral parity + modernization, which means a rewrite anyway. Asset formats are
recoverable with small parsers. Method: static-analyze EXW/EXD → recover algorithms,
data structures, constants, format semantics → provenance-tagged behavior specs →
clean Rust engine → verify against the original (Testing).

## 3. RE targets and order
1. BEDLAM.EXW (primary; DX3 import graph = platform seams). 2. BEDLAM.EXD (timing/audio
canon). 3. BEDLAM0/1/2.EXE diff (toolchain warm-up, ~27KB code, 3 variants). 4. SETUP.EXE
(.BDL semantics — pulled INTO the P2 loader pass, not left last).

## 4. Toolchain (research-verified; versions recorded in DECISIONS.md)
Ghidra 12.1.2 + yetmorecode/ghidra-lx-loader (verify 12.1.2 compat; raw-binary fallback)
+ watcall .cspec from Ghidra issue #156 / GhiOWat (args EAX/EDX/EBX/ECX; per-function
overrides for Win32 cdecl/stdcall imports). IDA Free 9.x cloud decompiler = second
opinion (PE only). Reko 0.12.3 = third opinion. rizin + rz-ghidra = CLI diffs. Oracles:
pinned Wine (EXW), pinned DOSBox-X/-staging (EXD; debugger memory watches), 8street
binaries (navigation + instrumented comparator). Asset work: our Rust tools/inspect.

Rust dependencies — decided at P4 spikes (owner directive: decide after analysis;
candidates in RESEARCH.md): presentation softbuffer+winit (default) vs pixels; SMK
pure-Rust smk fork (default; keeps unsafe policy clean) vs libsmacker-sys; audio cpal;
GM MIDI rustysynth; gamepad gilrs. DECISIONS.md records each choice + evidence.

## 5. Data safety (P0-hardened)
- game-data/ is read-only by convention AND by tool flag; writers only ever target
  scratch copies; MANIFEST.sha256 verified pre/post every tool run (fail loud).
- 3-2-1 backup: repo + game-data/ + original bin/cue to a second medium + one offsite
  (the corpus fits any free cloud tier). Push repo to a git remote as soon as one is
  chosen (currently 0 remotes).

## 6. Phases

### P1 — Data archaeology (parallel to P2)
tools/inspect parses/dumps every format → derived/ (git-ignored): .BIN sprite banks →
atlas+JSON; .PAL/.TRN/TXPAL; 17-extension mission set → map viewer; .RAW → WAV
(11025Hz mono per prior-work code; confirm from EXD); .MRW/.MRS → decoded events;
.SMK → frames+audio; LANGUAGE.* → runtime text DB keyed by string ID from day one
(never committed); .BDL → readable.
Exit (quantified): every file extension has a parser; round-trip byte-identical where
lossless; remaining unknowns are a listed set with hypotheses + provenance + target
gate. Two separate bars — round-trips vs semantics-verified-against-loader-code — are
tracked separately (fuzzing proves crash-freedom, not semantics).
Parsers are untrusted-input-safe from day one (Result-based, dimensions validated
before allocation, no panics); fuzz targets assert timeouts/OOM as well as crashes.
P1 exit includes per-language encoding identification (CP437/850/ANSI?) + accent
render test (u-umlaut, e-acute, ij).
P1 does NOT block on P2: items whose semantics need loader code (e.g. .MRW) exit P1 as
documented-unknowns and ratchet green as the P2b inventory lands.

### P2 — Executable RE (Ghidra)
Import EXW+EXD (lx-loader + watcall cspec); WinMain → pump → main loop; function DB in
git. Subsystem order: a. init/loop/timing (frame-lock, /NOSYNC path; entropy
inventory: every timing/clock/vsync read modeled as a deterministic input slot) →
b. exhaustive open()/read() site inventory covering every extension in game-data/ —
sprites, maps, audio, .BDL, .SMK, SETUP.EXE included — producing parser specs that
close P1 unknowns → c. renderer → d. simulation core (cross-check EXD + 8street;
record the original numeric representation per subsystem — expect integer/fixed-point,
SINTABLE.BIN trig) → e. input (.data control map table) → f. audio → g. UI/menus/shop/
save → h. Smacker.
Exit: specs sufficient for vertical slice; open questions each name the address that
answers them; slowdown catalog (where/why/measured-how) is a P2a exit item.

### P3 — Rust engine skeleton (no game logic)
Crates: bedlam-assets (ALL decoders incl. SMK; pure, deterministic, buffer-in/out);
bedlam-core (hermetic deterministic sim: no I/O/threads/wall-clock; fixed timestep =
1 original frame; Determinism Charter below; snapshot/restore; input-log replay +
per-tick state hash; replay/snapshot format carries a version header + initial state
hash + time base from day one); bedlam-render (indexed fb + palette; contract: render
produces indexed fb + palette, platform presents — nothing above changes for GPU
later); bedlam-audio (thin mix graph/device); bedlam-platform (thin window/input/
gamepad); bedlam-game (scene FSM, config, save; no per-mission code — mission quirks
are data — stated as a hypothesis to verify in P2d, with code-defined quirk hooks
tolerated until P5 evidence settles it). Errors: thiserror in parsers, never panic on
user-supplied assets, panic = engine bug. Logging: tracing with per-subsystem targets
(doubles as RE diff tooling). CI: Linux every commit from the first window; Windows
weekly; macOS nightly/manual (owner: if possible) — goldens never on macOS CI.

### P4 — Vertical slice + the two harnesses
Boot → TITLE.SMK → menu → ZONEA/MISSION1 render → move one squad member → palette/
audio present.
1. Dependency spikes decided here (softbuffer vs pixels; smk-fork vs libsmacker-sys).
2. Differential test harness (budgeted ~2 weeks, built here — the project insurance
   policy): DOSBox-X debugger memory-watches on RE-ed structure addresses + scripted
   frame-stepped input injection → per-frame original state dumps, diffed against
   engine per-tick state; instrumented 8street build as second comparator. Endpoint
   goldens alone cannot catch emergent divergence (RNG consumption order, tick-indexed
   spawn tables, frame-granular AI reactions); this harness can, and it also detects
   inherited 8street error.
3. Golden pipeline (achievable, stated honestly): pin dosbox/wine versions + configs;
   reference = state dumps (primary) + perceptual pixel thresholds vs emulator frames;
   exact pixel-diff reserved for our renderer across OSes; audio = correlation band on
   downsampled mix, never exact. Golden media live outside git; committed artifacts are
   content fingerprints (decoded pixel bytes + palette hash — never PNG file bytes) so
   codec re-encodes cannot false-regress.
Acceptance: replay determinism + state-dump parity on scripted slice scenes.

### P5 — Parity completion (per-zone gates)
37 missions playable; AI, weapons, shop, briefings, speech+music+SFX, save/load,
languages. Multiplayer-only (deathmatch) content carved out of the parity exit
(defined: maps load + local semantics correct; full DM = future work with netplay).
Acceptance per zone (playthrough-based, per the 0b budget): all scripted flows
complete without crashes; T1 game rules verified against RE/8street; perceptual frame
checks at key moments (T2); differential harness spot-checks for structure (not
tick-complete); cross-OS replay hash equality of OUR engine (internal determinism).
Original save compatibility: declared IN — original SAVED/OPTIONS.BDL import is
read-only, bounds-checked, fuzzed; new saves use the new versioned format.
Original-behavior catalog is a P5 artifact (per-bug ledger: repro, affected missions,
severity, gameplay-coupling) — the input to P6 triage; owner signs it at each zone gate.

### P6 — Modernization (default = modern; classic available)
Architecture (simplified by the 99% target): fixes land directly in the engine —
there is no bug-complete-faithful core to preserve. Classic mode shrinks to a small
purist toggle set covering feel-contested items only (timing lock, control scheme,
selected catalog entries the owner marks preserve). Mode is one immutable ModeConfig
injected at sim construction; test surface = the purist toggles, not 2^features.
- Time-based simulation: accumulator decouples tick rate from render; optional uncapped
  FPS. Interpolation scoped to camera/scroll only — grid-quantized 1996 sprites had no
  sub-pixel positions; interpolating them manufactures motion the original never showed
  (sub-pixel blitter may come later as an explicit option with a feel tolerance).
- Modern controls: WASD, 1-4 hotkeys, full remap, wheel zoom, gamepad; original scheme
  selectable.
- Bug triage rubric (per catalog entry): crash/data-loss → fix everywhere;
  gameplay-coupled → classic preserves / modern fixes; cosmetic → fix in modern.
  Fixed = deviation from the catalog, decided by rubric, signed off — not vibes.
- QoL: window modes, vsync control, volume mixers, save slots + metadata + opt-in
  autosave. Game-feel proxies: input-to-present ≤ 1 original frame; animation cadence
  matches at 60Hz; no stutter under p95 frame-time budget.

### P7 — Ports and packaging
Linux native + Flatpak; Windows installer; macOS universal2 (best-effort per owner);
CI artifacts per push. CDDA: user-supplied original tracks (WAV/CD), optional local
lossy cache generated on first run — never redistributed. SteamDeck defaults stretch.

## 7. Determinism Charter (bedlam-core invariants)
- Integer/fixed-point math or bit-specified soft-float; no ambient libm, no
  transcendentals. NOTE: this is for OUR cross-OS determinism, not to mirror Watcom
  rounding (T3) — the SINTABLE approach is adopted because it is fast, exact enough,
  and sidesteps libm variance entirely.
- The PRNG mechanics and distributions are recovered from RE (what rolls happen,
  when, with what parameters) but the original bit-stream is NOT mirrored (T3) — our
  own seeded PRNG, statistically matched, is correct at this parity target.
- No unordered iteration (HashMap et al.) may influence sim state.
- All original entropy/timing reads are modeled as deterministic per-tick inputs (from
  the P2a entropy inventory); per-zone tick rate, if the original varies it, is data
  and the replay format records the time base.
- Cross-OS per-tick hash equality is a CI job from the first playable tick, not a P5
  afterthought. Miri on core. Unsafe only in platform/audio (+ vendored codec if
  libsmacker-sys wins the spike — named exception).

## 8. Multiplayer
Out of scope for v1 (original was DirectPlay modem/serial/IPX). The deterministic core
does not preclude lockstep netplay later.

## 9. RE quality control (knowledge supply chain)
Every spec claim: provenance ([addr @VA] / [black-box] / [8street file:line — hint]) +
confidence (verified-by-execution / verified-by-disasm / hypothesis). Claims tagged
verified-by-disasm require an evidence artifact (disasm snippet + cross-ref + a
falsifying black-box probe where feasible) and a second independent agent re-derivation
before the claim may be cited by engine code; 10% random re-verification; sub-high-
confidence claims block phase exit. Unknown formats stay unknown — the CDDA music
fallback makes that safe.

## 10. Execution model (AI-assisted, ~24/7 — time is not the constraint)

Parallel fan-out (concurrent agents, each with its own exit gate):
- P1: one agent per file extension (17 parsers) + per-format fuzz agents; corpus
  round-trip and MANIFEST checks run continuously in the background.
- P2: subsystem agents partitioned by address range over the shared function DB
  (integration is serialized; analysis fans out); anchoring sprints in batches.
- P4+: golden/differential corpus generation and fuzzing run as background compute
  around the clock; per-zone parity verification (P5) fans out one agent per zone.
- Two-role split (§1) is enforced mechanically: reader agents write specs, builder
  agents write Rust from specs; a verifier agent re-derives normative claims.

True bottlenecks (invest here, not in schedule padding):
1. Verification throughput — the differential harness is the rate limiter for
   everything downstream; build it first and keep it fed 24/7.
2. Serial integration points — function DB merges, harness upgrades, spec-gate
   sign-offs.
3. Human decisions — bug-catalog sign-off per zone, game-feel acceptance, legal
   posture, and any call needing the owner. These are the only wall-clock waits.

Stop-anytime ladder (value ordering if the project halts at any instant — NOT a
schedule fallback; nothing is cut for time): every phase ends shippable. If work
stops, the artifact is, in increasing order: asset tooling + format docs → + vertical
slice → + N zones passing parity gates → + modernized controls/timing → + 3-OS
packaging. Scope is never reduced for schedule reasons; only for owner preference.

## 11. Risks and mitigations
Watcom codegen → structures/constants first; DX import graph bootstraps; 8street
answers semantics fast (governed by §1). Sim complexity → per-zone gates; editor
data documents intent. Unknown formats → P2b inventory; music fallback = CDDA.
Frame-locked balance → parity tick + measured re-tune + classic toggle. 8street
disappears / legal → every fact re-anchored; engine-only repo. Agent-specific risks
(hallucinated semantics, silent consensus) → §9 QC protocol + independent re-derivation
+ differential harness as ground truth. The stuck-points of projects like this are
wrong assumptions baked early, verification gaps, and data loss — not typing speed;
the plan is shaped around exactly those (canon §0, harness §6-P4, manifest + backups §5).

## 12. Milestones (gate-ordered, not calendar-estimated)
Order is fixed; elapsed time ≈ compute time + owner decision latency. Meaningful
metrics: verified-RE-facts per day (anchored/total ratio), zones passing parity gates,
fuzz corpus coverage, differential-corpus pass rate.
P1 (gates: every extension parsed; corpus round-trips; encodings identified) → P2
(gates: loader inventory complete; slice-sufficient specs; entropy + slowdown
catalogs) → P3 (gate: deterministic core replays cross-OS hash-equal from first tick)
→ P4 (gates: slice scenes pass differential harness; dependency spikes decided) →
P5 (gate: per-zone parity + signed original-behavior catalog) → P6 (gate: classic and
modern profiles both green) → P7 (gate: 3-OS artifacts). Full parity target: all 37
zones green; scope ceiling is the full plan, not a date.

## 13. Immediate next actions
1. Install RE stack (Ghidra 12.1.2 + lx-loader + watcall cspec; rizin; DOSBox-X +
   DOSBox-staging; Wine) — versions pinned in DECISIONS.md.
2. tools/inspect v0: .BIN sprite dumper + .PAL → PNG (prove the toolchain; outputs to
   derived/; MANIFEST verified pre/post).
3. Ghidra: BEDLAM0/1/2 diff (warm-up), then EXW+EXD import.
4. Clone 8street repos at a pinned commit as reading references (outside our repo);
   start docs/DIVERGENCES.md from its README changes list.
5. Backup: second copy of game-data/ + originals + repo push to a remote.
6. DECISIONS.md (dependency table + spike plan) and CI skeleton.
