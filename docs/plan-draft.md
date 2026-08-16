# Bedlam (1996) — Decompilation & Modern Reimplementation Plan (DRAFT v1 for panel review)

Goal: a stable, smooth, cross-platform (Linux→Windows→macOS) Rust reimplementation of
Bedlam (GT Interactive / Mirage, 1996) that runs the original assets, fixes the game's
slowdowns/quirks/bugs, adds modern controls (WASD, 1-4 weapon hotkeys, rebinding),
switches the engine from frame-locked to time-based simulation, and is engineered to be
highly testable and debuggable throughout, so the project never gets stuck.

Verified groundwork facts live in docs/GROUNDWORK.md (read first). Summary: hybrid
DOS/Win95 release; all binaries Watcom C/C++ 10.x (register calling convention);
BEDLAM.EXW (Win32/DirectX3, ~336KB code) is the real game; BEDLAM.EXD (DOS/4GW+HMI+VESA)
is the DOS build; launcher variants + SETUP.EXE are small auxiliaries; assets are simple
custom formats (8-bit palettized sprite banks, palette LUTs, 37 missions of editor data,
raw PCM audio, custom .MRW music, Smacker video, CDDA tracks, 6 language files).

## Strategy: RE-informed clean reimplementation ("black-box parity")

We do NOT aim for byte-identical decompiled C that recompiles (Devilution-style).
Rationale:
1. Watcom register-based calling conventions + 1996 compiler codegen make automatic
   decompilers unreliable; hand-cleaning 336KB of i386 assembly to compilable C is the
   slowest possible path to a *modernized* engine.
2. The stated goal is behavioral parity + modernization (timing, controls, bug fixes),
   which requires rewriting the engine anyway.
3. Asset formats (the durable value) are fully recoverable with small custom parsers.

Instead: static-analysis the original EXEs in a disassembler to recover **algorithms,
data structures, constants, and file-format semantics**, write those up as docs, and
implement a clean Rust engine verified against the original via deterministic tests and
visual/audio goldens captured from DOSBox (DOS build) and Wine (Win95 build).

Clean-room hygiene: findings are documented as behavior specs; no decompiled code is
pasted into the repo; no original assets are redistributed (game-data/ is git-ignored;
the engine requires the user's own copy of the game).

## Primary RE target & order

1. **BEDLAM.EXW** (Win32/DirectX) — primary. Win32 APIs give clean seams: every
   DirectDraw/DirectSound/DirectPlay/Smacker call marks an engine boundary; PE imports
   enumerate the platform surface we must reimplement.
2. **BEDLAM.EXD** (DOS) — cross-reference for hardware-coupled details: HMI audio
   sample rates, VGA/VESA mode + palette details, PIT/timing constants, original
   frame-rate assumptions.
3. **BEDLAM0/1/2.EXE launcher diff** — warm-up exercise to learn the toolchain on a
   ~27KB-code binary with three versions to diff.
4. SETUP.EXE — only as needed for CONFIG.BDL/BDL file semantics.

## Tools (to be confirmed by current research — see open questions)

- Disassembler/decompiler: Ghidra (free, PE32+MZ loaders; add community Watcom calling-
  convention scripts / FID packs). IDA Free as alternate for cross-checking. Reko or
  retdec for batch hints on DOS/LE. rizin for quick CLI disasm of small routines.
- Data work: custom Rust CLI (`tools/inspect`) — hex templates are fine for exploring but
  Rust parsers become the product; ImHex for ad-hoc eyeballing.
- Reference environment: DOSBox(-staging) for the DOS build; Wine for the Win95 build
  (DirectDraw via ddraw compat wrappers may be needed); both used to generate goldens.
- SMK: use RAD Smacker public spec + SMACKW32.DLL exports as cross-ref; decode via
  existing crates/bindings if viable in Rust, else ffmpeg for goldens and a pure-Rust
  decoder as a work item.
- Rust dependencies: decided per-crate at each milestone (user directive: decide after
  analysis) — decision table maintained in docs/DECISIONS.md.

## Workstreams & phases

### P0 — Project setup (DONE 2026-08-17)
Repo at ~/Documents/bedlam-re: docs/, engine/ (Rust workspace), game-data/ (reference
copy, git-ignored, 383MB incl. CDDA WAVs; duplicate 148MB ISO excluded), goldens/,
tools/. Git initialized.

### P1 — Data archaeology (asset formats) — starts immediately, parallel to P2
Build `tools/inspect` (Rust CLI) that parses and dumps every format to PNG/JSON/WAV:
- .BIN sprite banks (u16 count + u32 offsets — confirmed) → PNG atlas + JSON metadata
- .PAL/.TRN/TXPAL — palettes & LUTs → viewable swatches
- mission set (.MAP/.TOT/.CGR/.CTG/.BLD/.COL/.LNK/.NME/.POS/.PTH/.PAD/.MRK/.BDG/.DAT/.TRT/.TXT)
  → map viewer (HTML or egui) rendering tiles + markers + enemy spawns
- .RAW PCM → WAV (rate confirmed in P2 from HMI init code); .MRW/.MRS music → decoded events
- .SMK → frame PNGs + audio (via existing decoder if usable)
- LANGUAGE.* → structured text DB
- CONFIG/SAVED/OPTIONS.BDL → readable/editable
Exit criteria: every file in game-data/ is either parsed losslessly (round-trip
byte-identical where the format is not lossy) or explicitly documented as
unknown-with-hypotheses. Fuzzing (cargo-fuzz) seeded with every original file.

### P2 — Executable RE (Ghidra project)
- Import EXW + EXD into Ghidra; apply Watcom calling-convention script; find main loop
  via WinMain → message pump; name functions as understood (function DB committed to git
  as .gzf exports + Markdown notes per subsystem).
- Subsystem order (each produces a spec doc + Rust implementation in P3/P4):
  a. init & main loop, timing (PIT/vsync waits — find the frame-lock & the /NOSYNC path)
  b. resource loading (all file open/read call sites → format semantics; closes P1 gaps)
  c. renderer (DDraw surfaces, dirty rects?, palette transitions (.TRN/TXPAL), sprite
     blitter, scanner/minimap, isometric tile math)
  d. simulation core (entities, AI, pathfinding (.PTH/.LNK), ballistics, damage,
     economy/shop, mission triggers) — the largest chunk; cross-check EXD
  e. input (keyboard/mouse handling, control map table in .data → rebinding evidence)
  f. audio (HMI init: sample rates; SFX trigger table; .MRW music sequencer; speech)
  g. UI/menus, briefing flow, shop, save/load (.BDL formats)
  h. Smacker playback path (SMACKW32 usage — we replace with our decoder)
- Exit criteria: subsystem specs sufficient to implement a vertical slice; open
  questions explicitly listed with the address of the code that answers them.

### P3 — Rust engine skeleton (starts with P1's first parsers; no game logic yet)
Workspace crates (engine/):
- bedlam-assets: all format parsers+writers (round-trip tests, fuzz)
- bedlam-core: deterministic simulation ONLY — no I/O, no threads, no wall-clock;
  fixed timestep (tick = 1 original frame); seeded PRNG; snapshot/restore of full state;
  input-log replay with per-tick state hashing
- bedlam-render: 8-bit indexed framebuffer, palette animation, sprite/tile blit —
  software first (matching original output), presented via platform layer with integer
  scaling; GPU/optional upscaler later
- bedlam-av: SMK decode, PCM mixing, MIDI synth (General MIDI soundfont)
- bedlam-platform: window/input (keyboard/mouse/gamepad)/audio out; per-OS backends
- bedlam-game: scene state machine (boot→menu→briefing→mission→shop→...), config,
  modern/classic option plumbing, save/load
- tools/: inspect CLI, asset viewer, golden capture + pixel-diff, replay runner,
  save-state explorer
Cross-platform from day one (CI on Linux/Windows/macOS once a window shows).

### P4 — Vertical slice (first playable proof)
Boot → title (TITLE.SMK) → main menu → load ZONEA/MISSION1 → render map correctly →
move one squad member with keyboard → correct palette/audio present. Input: recorded
replays + DOSBox golden screenshots for the same scenes; pixel-diff harness with
per-channel/palette tolerance.

### P5 — Parity completion
All 37 missions playable start-to-finish; AI, weapons (WEAPONS.BIN table), shop/economy,
briefings, speech+music+SFX, save/load, deathmatch maps (DM_* text sections exist),
language switching. Longest phase; fed by P2 subsystem docs increment by increment.
Parity acceptance: full playthrough captured as input logs replays deterministically on
all 3 OSes; goldens suite green.

### P6 — Modernization (each feature a toggle; default = "modern", "classic" available)
- Time-based simulation: decouple tick rate from render (accumulator, interpolated
  rendering, optional uncapped FPS). Classic mode keeps original tick-lock behavior.
- Modern controls: WASD, weapon hotkeys 1-4 (and full remap), mouse-wheel camera zoom,
  edge-scroll toggle, gamepad mapping. Keep original control scheme selectable.
- Bug/slowdown fixes, each documented in docs/FIXES.md with root cause from RE
  (candidates already: /NOSYNC timing bug class, palette corruption on VESA drivers,
  save-file fragility).
- QoL: windowed/borderless/fullscreen, vsync control, volume mixers, per-language font
  fallback, save slots with metadata + autosave (opt-in).
Exit: FIXES.md entries reference the original code addresses that justify each fix.

### P7 — Ports & packaging
Linux (native + Flatpak), Windows (installer), macOS (universal2 app). CI artifacts per
push. SteamDeck/QAM-friendly gamepad defaults as stretch.

## Multiplayer
Original Win95 version used DirectPlay (modem/serial/IPX era) — treat as out of scope
for v1 parity. The deterministic-lockstep-ready core (input-log replays are literally
lockstep) leaves netplay open as a future workstream without rework.

## Testing & debugging strategy (first-class, not an afterthought)
1. **Determinism**: bedlam-core is hermetic (no I/O/threads/clock); per-tick state hash;
   input-log replays; any divergence localizes via snapshot bisection.
2. **Format goldens**: every parser round-trips the entire 383MB corpus byte-identically
   (where lossless); property tests on writers; cargo-fuzz corpus = original files.
3. **Visual/audio goldens**: scripted scenes in DOSBox/Wine → reference frames;
   engine renders same scripted inputs → pixel-diff (exact for software renderer) and
   audio-capture correlation (rough — audio timing tolerance).
4. **In-engine debug tools**: F12 overlay (hitboxes, AI state, pathfinding graph, frame
   budget, sim/render rate), frame-step, slow-mo, scene jump, save-state editor,
   asset browser (reuses P1 viewer).
5. **CI**: fmt+clippy+deny+test+goldens on 3 OSes; goldens regenerated only via
   reviewed "reference capture" runs; unsafe confined to platform crate; miri on core.
6. **RE↔code traceability**: every subsystem spec doc lists the addresses/ranges it was
   derived from; every engine module links its spec doc; disagreements filed as open
   questions, not silently resolved.

## Risks & mitigations
- Watcom codegen slows RE → prioritize data structures & constants over full function
  decompilation; the DirectX import graph bootstraps the map quickly.
- Simulation complexity (AI/pathfinding) larger than expected → slice by mission zone;
  ZONEA tutorial is deliberately simple; editor data (.NME/.PTH/.LNK) documents intent.
- Unknown formats (.MRW music, .CTG, TXPAL) → loader code in EXW reads them; P2b closes
  gaps; worst case music ships as recorded CDDA tracks (they are the same soundtrack).
- Frame-locked gameplay balance (speeds tuned to frame count) → parity tick = original
  frame; time-based mode re-tunes via measured constants, toggles preserved.
- Single-maintainer burnout/scope → each phase has an independently valuable deliverable
  (P1 tooling alone already extracts/exports all assets; P4 slice is demoable).
- Legal: engine-only repo, no assets; name it clearly unofficial; personal-use framing.

## Milestones (rough, calendar estimates are honest ranges, not promises)
- P1 2-4 wks · P2 4-12 wks (overlaps P1/P3) · P3 2-4 wks (overlaps) · P4 2-4 wks ·
  P5 8-24 wks · P6 2-6 wks · P7 1-2 wks. Total to full parity: roughly 5-12 months of
  focused part-time effort; vertical slice in ~6-10 weeks.

## Immediate next actions
1. Install & configure RE stack (per research): Ghidra + Watcom scripts; rizin; DOSBox-
   staging; Wine; capture goldens pipeline.
2. tools/inspect v0: .BIN sprite dumper + .PAL → PNG (proves the toolchain).
3. Ghidra: import BEDLAM0/1/2.EXE, diff, name functions (warm-up); then import EXW.
4. Write docs/DECISIONS.md (dependency decision table) after P1 formats are known.
5. Set up CI skeleton + goldens harness skeleton.
