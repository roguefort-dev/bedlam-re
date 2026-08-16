# Bedlam (1996) — Decompilation & Modern Reimplementation Plan (v2 — post-research, pre-panel-consolidation)

Goal: a stable, smooth, cross-platform (Linux→Windows→macOS) Rust reimplementation of
Bedlam (GT Interactive / Mirage, 1996) that runs the original assets, fixes the game's
slowdowns/quirks/bugs, adds modern controls (WASD, 1-4 weapon hotkeys, rebinding),
switches the engine from frame-locked to time-based simulation, and is engineered to be
highly testable and debuggable throughout, so the project never gets stuck.

Verified groundwork: docs/GROUNDWORK.md. Verified tooling research: docs/RESEARCH.md.
Summary of facts: hybrid DOS/Win95 release; all binaries Watcom C/C++ 10.x (register
calling convention: args EAX/EDX/EBX/ECX, callee cleans); BEDLAM.EXW (Win32/DirectX3,
~336KB code) is the real Win95 game; BEDLAM.EXD (DOS/4GW+HMI+VESA, 655KB) is the DOS
build; launcher variants + SETUP.EXE are small auxiliaries; assets are simple custom
formats (8-bit palettized sprite banks, palette LUTs, 37 missions of editor data,
headerless 8-bit PCM audio at 11025 Hz mono [confirmed via prior-work port code; verify
in EXD HMI init], custom .MRW/.MRS music, Smacker video, 7 CDDA tracks, 6 language files).

## Prior work (research-verified 2026-08-17) — changes the calculus

**8street/Bedlam** (github.com/8street/Bedlam): a C++(+ASM) reconstruction of the
original executable, "compiles and fully playable in single player mode", ported to
SDL2/SDL2_mixer/libsmacker-1.2.0, Windows+Linux builds, 109 commits, updated 2025.
Companion: ReversedBedlam (partial reversing notes) and BedlamTools (asset viewer).
Author's IDA database of the original is linked from its README.

How we use it (decision):
1. **Cross-reference oracle**: when our RE of EXW/EXD leaves a semantic question open
   (a format field, an AI constant, the .MRW player), the 8street code answers it in
   minutes instead of days. It is a *reading reference*, treated exactly like our own
   decompiler output — facts get written into our spec docs with provenance tags.
2. **Not a porting source**: we do not transliterate its C++ into Rust. Its architecture
   is a 1:1 reconstruction of 1996 code (global state, raw pointers, 11kHz mixer) — the
   opposite of the modernized engine we are building. A Rust port of it would inherit
   its frame-locked design and be a derivative-of-derivative with worse legal posture.
3. **Behavior oracle #2**: its binaries run the same original assets; useful as a
   second golden source alongside DOSBox/Wine where the original build is inconvenient.
4. **IP posture**: our repo stays engine-only, RE-informed (see Strategy); we cite
   8street in docs where it resolved a question. No copying from it either.
5. Its existence proves feasibility (full parity is reachable) and de-risks P5.

What it does NOT give us (why this project is still worth doing): no time-based sim,
no modern controls, no 60fps uncapped rendering, no native 3-OS build, no testability
infrastructure, 32-bit-only, no fuzzed parsers, no determinism guarantees.

## Strategy: RE-informed clean reimplementation ("black-box parity")

We do NOT aim for byte-identical decompiled C that recompiles (Devilution-style), and we
do NOT port 8street's C++. Rationale:
1. Watcom register-based calling conventions + 1996 codegen make automatic decompilers
   unreliable; hand-cleaning 336KB of i386 assembly to compilable C is the slowest path
   to a *modernized* engine.
2. The goal is behavioral parity + modernization (timing, controls, bug fixes), which
   requires rewriting the engine anyway.
3. Asset formats (the durable value) are fully recoverable with small custom parsers.

Method: static-analysis the original EXEs to recover **algorithms, data structures,
constants, and file-format semantics**; document as behavior specs; implement a clean
Rust engine; verify against the original via deterministic tests and visual/audio
goldens captured from DOSBox (DOS build), Wine (Win95 build), and the 8street binaries
(secondary). Every spec claim carries provenance: [addr EXW/EXD @VA] / [8street file:line]
/ [black-box observation] — and a confidence tag.

Hygiene: specs hold semantics/structures/constants (facts), never code transcripts;
no original assets or asset-derived dumps are redistributed (game-data/ and derived/
git-ignored; engine requires the user's own copy of the game).

## Primary RE target & order

1. **BEDLAM.EXW** (Win32/DirectX) — primary. Win32 APIs give clean seams; PE imports
   enumerate the platform surface. Cross-ref 8street continuously.
2. **BEDLAM.EXD** (DOS) — hardware-coupled details: HMI audio init (confirm 11025 Hz),
   VGA/VESA mode + palette, PIT/timing constants, original frame-rate assumptions.
3. **BEDLAM0/1/2.EXE launcher diff** — toolchain warm-up on ~27KB of code, 3 variants.
4. SETUP.EXE — only for .BDL file semantics.

## Toolchain (research-verified; install in P1 kickoff)

RE:
- **Ghidra 12.1.2** — primary. PE32 i386 + MZ loaders built in; scriptable.
- **yetmorecode/ghidra-lx-loader v12.0.x** — LE/LX loader for DOS/4GW EXD (verify
  compat under 12.1.2; raw-binary fallback if not).
- **Watcom watcall .cspec** from Ghidra issue #156 / 0xBEEEF-GhiOWat (args
  EAX/EDX/EBX/ECX; per-function overrides for Win32 cdecl/stdcall imports — the known
  wart). Build our own Watcom FID pack from Open Watcom CLIB as a P2 stretch (contrib back).
- **IDA Free 9.x** — cloud x86 decompiler, second opinion on PE (non-commercial).
- **Reko 0.12.3** — third opinion on PE32 (no LE/LX).
- **rizin + rz-ghidra** — quick CLI disasm/diff.
Reference environments: DOSBox-staging (EXD), Wine (EXW), 8street binaries (oracle).
Asset work: our own Rust `tools/inspect` (product code, not throwaway); ImHex ad-hoc.

Rust dependencies (decision-forcing at P3/P4 spikes; candidates research-verified):
- Presentation: `softbuffer` 0.4.8 + `winit` 0.30.13 (default: software 8-bit fb,
  integer scale; simplest+most maintained) — vs `pixels` 0.17.2 if GPU post-processing
  is wanted later. Decide by P4 spike. SDL3 (`sdl3` 0.18, pre-1.0 churn) rejected as
  default for now; revisit on evidence.
- Audio out/mixing: `cpal` 0.18.1 (custom mix graph pulling 8-bit PCM + synth).
- GM MIDI synth: `rustysynth` 1.3.6 (pure Rust, built-in SMF sequencer, SF2).
- SMK decode: pure-Rust `smk` 0.1.0 (2026, unproven — vendor/fork) else `libsmacker`/
  `libsmacker-sys` bindings; ffmpeg-next only as offline extraction tool.
- Gamepad: `gilrs` 0.11 (+ winit for kb/mouse).
Final call recorded per-crate in docs/DECISIONS.md after spikes — per owner directive
"decide after the files are analyzed".

## Workstreams & phases

### P0 — Project setup (DONE 2026-08-17)
Repo ~/Documents/bedlam-re: docs/, engine/ (workspace), game-data/ (reference copy,
git-ignored, 383MB; dup 148MB ISO excluded), goldens/, tools/. Git initialized.

### P1 — Data archaeology (asset formats) — parallel to P2
tools/inspect (Rust CLI) parses and dumps every format to PNG/JSON/WAV (outputs to
git-ignored derived/): .BIN sprite banks → atlas+JSON; .PAL/.TRN/TXPAL; the 17-extension
mission set → map viewer (tiles, markers, enemy spawns); .RAW PCM → WAV (11025 Hz
mono assumed; confirm from EXD); .MRW/.MRS → decoded music events (cross-ref 8street
player + HMI MIDI family docs); .SMK → frames+audio; LANGUAGE.* → text DB (runtime
read from user copy; never committed); .BDL → readable/editable.
Exit: every file parsed (round-trip byte-identical where lossless) or documented
unknown-with-hypotheses+provenance. Fuzzing (cargo-fuzz) seeded with original files.
Parsers treat input as untrusted from day one: Result-based, bounds-checked, no panics.

### P2 — Executable RE (Ghidra project)
Import EXW+EXD (lx-loader), apply watcall cspec, find main loop via WinMain→pump.
Function DB committed. Subsystem order, each → spec doc (with provenance+confidence)
feeding Rust implementation:
 a. init & main loop, timing (PIT/vsync waits; frame-lock; /NOSYNC path)
 b. resource loading FIRST among deep dives (file open/read sites → format semantics;
    closes P1 gaps; unblocks parsers)
 c. renderer (DDraw surfaces, palette transitions .TRN/TXPAL, blitter, scanner, iso math)
 d. simulation core (entities, AI, pathfinding .PTH/.LNK, ballistics, damage, economy,
    mission triggers; cross-check EXD + 8street)
 e. input (keyboard/mouse handling; .data control table → rebinding evidence)
 f. audio (HMI init rates; SFX triggers; .MRW sequencer; speech)
 g. UI/menus, briefing, shop, save/load
 h. Smacker path
Exit: specs sufficient for a vertical slice; open questions listed with the address
that answers each.

### P3 — Rust engine skeleton (starts with P1 parsers; no game logic)
Crates: bedlam-assets (ALL decoders incl. SMK — pure, deterministic, buffer-in/out);
bedlam-core (deterministic sim ONLY: no I/O/threads/clock; fixed timestep 1 tick = 1
original frame; integer/fixed-point math or pinned soft-float, no ambient libm;
original PRNG recovered; snapshot/restore; input-log replay + per-tick state hash);
bedlam-render (indexed framebuffer + palette; software blit; contract: render produces
indexed fb+palette, platform presents); bedlam-audio (mix graph, device — thin);
bedlam-platform (window/input/gamepad per-OS, thin); bedlam-game (scene FSM, config,
save/load — no per-mission logic; mission quirks are data). tools/ (inspect, viewer,
golden capture/diff, replay runner). CI on 3 OSes from the first window.

### P4 — Vertical slice
Boot → TITLE.SMK → menu → ZONEA/MISSION1 render → move one squad member → palette/
audio present. Acceptance: recorded replays + reference frames (DOSBox primary) +
pixel-diff harness. Dependency spikes decided here (softbuffer vs pixels; smk vs
libsmacker).

### P5 — Parity completion
All 37 missions playable start-to-finish; AI, weapons (WEAPONS.BIN), shop/economy,
briefings, speech+music+SFX, save/load, deathmatch maps, language switching.
Acceptance: per-zone replay-based acceptance (progress measurable per zone, not one
cliff); full-playthrough input logs replay deterministically on all 3 OSes.

### P6 — Modernization (toggles; default = modern, classic available)
- Time-based simulation: accumulator decouples tick rate from render, interpolated
  rendering, optional uncapped FPS; classic mode keeps original tick-lock.
- Modern controls: WASD, 1-4 hotkeys, full remap, wheel zoom, gamepad; original
  scheme selectable.
- Bug/slowdown fixes each documented in docs/FIXES.md with root cause + address
  (candidates: /NOSYNC timing class, VESA palette corruption, save fragility; plus
  8street's fixed-crash list as triage input).
- QoL: windowed/borderless/fullscreen, vsync control, volume mixers, save slots with
  metadata + opt-in autosave.

### P7 — Ports & packaging
Linux (native + Flatpak), Windows installer, macOS universal2 app; CI artifacts per
push. SteamDeck-friendly gamepad defaults as stretch.

## Multiplayer
Original Win95 used DirectPlay (modem/serial/IPX). Out of scope for v1; the
deterministic core does not preclude lockstep netplay later.

## Testing & debugging strategy
1. Determinism: hermetic bedlam-core; integer/fixed-point policy; per-tick state hash;
   input-log replays; snapshot bisection for divergence.
2. Format goldens: parsers round-trip the corpus byte-identically where lossless;
   fuzz corpus = original files.
3. Visual/audio goldens: scripted scenes in DOSBox/Wine/8street → reference frames;
   engine on same inputs → pixel-diff (exact for software renderer) + audio correlation.
4. In-engine debug: overlay (hitboxes, AI state, pathfinding graph, frame budget,
   sim/render rate), frame-step, slow-mo, scene jump, save-state editor, asset browser.
5. CI: fmt+clippy+deny+test+goldens on 3 OSes; unsafe confined to platform/audio;
   miri on core.
6. RE↔code traceability: every spec claim has provenance + confidence tag; engine
   modules link their spec; disagreements are open questions, never silent.

## Risks & mitigations
- Watcom codegen slows RE → structures/constants first; DirectX import graph
  bootstraps; 8street answers semantic questions fast.
- Sim complexity → slice by mission zone; ZONEA tutorial is simple; editor data
  (.NME/.PTH/.LNK) documents intent.
- Unknown formats (.MRW/.CTG/TXPAL) → EXW loader code + 8street cross-ref; worst case
  music ships as CDDA tracks (same soundtrack).
- Frame-locked gameplay balance → parity tick = original frame; time-based mode re-tunes
  via measured constants; toggles preserved.
- 8street disappears/legal posture → we cite, we do not depend: every fact we take from
  it is re-anchored to an address in EXW/EXD in our spec docs.
- Burnout/scope → each phase independently valuable (P1 tooling alone extracts all
  assets; P4 slice is demoable).
- Legal: engine-only repo, no assets, no asset-derived dumps; unofficial; personal use.

## Milestones (honest ranges)
P1 2-4wks · P2 4-12wks (overlaps P1/P3) · P3 2-4wks (overlaps) · P4 2-4wks · P5 8-24wks
(per-zone gates) · P6 2-6wks · P7 1-2wks. Full parity ≈ 5-12 months focused part-time;
vertical slice ~6-10 weeks. (8street's existence materially de-risks P5 toward the
low end.)

## Immediate next actions
1. Install RE stack (Ghidra 12.1.2 + lx-loader + watcall cspec; rizin; DOSBox-staging;
   Wine) and record versions in docs/DECISIONS.md.
2. tools/inspect v0: .BIN sprite dumper + .PAL → PNG (prove the toolchain).
3. Ghidra: import BEDLAM0/1/2.EXE, diff, name functions (warm-up); then EXW+EXD.
4. Clone 8street/Bedlam + BedlamTools as reading references (outside our repo).
5. docs/DECISIONS.md: dependency table; P3/P4 spike plan.
6. CI skeleton + goldens harness skeleton.
