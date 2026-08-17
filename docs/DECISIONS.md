# Decisions Log

## D1 — Canon (2026-08-17)
BEDLAM.EXW (Win95/DirectX) = canonical build. EXD = hardware-coupled canon.
8street pinned refs: Bedlam@a8622e6, ReversedBedlam@d5bf760, BedlamTools@9a32f25
(cloned to ~/Documents/bedlam-refs). See PLAN.md section 0.

## D2 — Tooling install status (2026-08-17)
Host has NO passwordless sudo. Pending user action (one command):
  sudo pacman -S ghidra jdk21-openjdk rizin wine
dosbox-staging / dosbox-x: not in pacman repos — flathub or AUR or AppImage; needed
before P4 goldens, not blocking P1/P2-prep.
Available already: rustc 1.97.1, cargo, gcc/clang, binutils, python 3.14.

## D3 — tools/inspect v0 dependencies (2026-08-17)
Tool-only (not engine): serde, serde_json, image (PNG out). Engine crate deps remain
undecided until P4 spikes per owner directive; candidates in RESEARCH.md.

## D4 — First corpus pass results (2026-08-17, tools/inspect v0)
1069 files walked. Parsed: .PAL 770B (52 — 2B hdr + 768B VGA 6-bit), .TRN (16),
.RAW -> WAV 11025Hz-mono-unverified (149), .SMK headers (35).
Unknown .PAL variants: 256B x3 (DARKPAL, DARKPALS, SELDARK), 98B x2 (CONSPAL, FULLPAL),
65536B x3 (TXPAL1-3 = 256x256 byte tables, likely palette crossfade matrices).
NEGATIVE RESULT: u16-count + u32-offset directory hypothesis REJECTED for GAMEGFX
.BIN banks (167 files, 0 fit). Correct layout to come from 8street bin_file.cpp
analysis + EXW loader RE (P2b). Mission extensions pending (2 agents running).
game-data integrity: MANIFEST.sha256 verified clean after the pass.

## D5 — Backups (2026-08-17)
game-data copied to ~/Backups/bedlam-re/game-data (1069 files verified by count).
Offsite copy pending (user picks provider). Repo has no remote yet.

## D6 — Remote (2026-08-17)
Public repo: https://github.com/roguefort-dev/bedlam-re (user choice: personal account;
a future RogueFortDevelopment org can receive a zero-loss transfer). Pre-push audit
clean: only docs/tools/CI/manifest tracked; game-data/ + derived/ + .ghidra-project/
ignored. Branch renamed master -> main.

## D7 — Toolchain install complete (2026-08-17)
Installed via pacman: ghidra 12.1.2 (/opt/ghidra), jdk21-openjdk 21.0.11, rizin 0.8.2,
radare2 6.1.4, wine 11.15 + wine-mono + winetricks, dosbox 0.74-3, ffmpeg n9.0.1,
sox 14.8, gdb 17.2, python-capstone 5.0.7, python-unicorn 2.1.4, flatpak 1.18.1.
Pending: dosbox-staging/-x (flathub/AppImage, no sudo needed), lib32-sdl2 (only for a
native comparator build; Wine fallback exists).

## D8 — Autonomy loop (2026-08-17)
Mechanism: systemd user timer bedlam-nudge.timer (60s, linger on, survives reboot;
crontab binary absent on host - cronie not installed - timer chosen instead).
tools/nudge.sh: exits on PLAN-COMPLETE / PAUSE / fresh heartbeat (<7 min) / flock held;
else spawns: opencode2 run --auto continuation agent bound to AGENTS.md contract
(one bounded work unit per run, NEXT.md queue, manifest checks, small commits, push).
Compaction config (global opencode.jsonc): auto=true, keep=15000, buffer=60000
(triggers preflight compaction when ~30% of a 200k-token context remains; earlier
than the 20k default). Human controls: touch .state/PAUSE to suspend;
touch .state/PLAN-COMPLETE to stop forever (agents create it themselves when
PLAN.md P0-P7 all pass).

## D9 — EXW startup/loop architecture resolved (2026-08-17)
Via Ghidra BedlamWatcom project (watcall cspec): game loop = game thread +
periodic timeSetEvent callback (FUN_0041bfb6 = frame driver candidate); the
message pump is UI-only. Semantic names applied in project: WinMain /
InitInstance / MsgPump / TimerInit / TimerCallback / WatcomCrtStartup /
BedlamWndProc. Analysis scripts tracked in tools/ghidra-scripts/; raw dumps
stay local (gitignored). docs/exw-functions.txt = function DB snapshot
(675 fns). Full write-up: docs/RE-EXW-MAINLOOP.md.


## D9 — Arbitrary-resolution output (2026-08-17, user requirement)
User wants 1080p (any resolution) support. Decision: resolution independence lives
entirely in the presentation layer. bedlam-render always produces the canonical
640x480 indexed fb + palette (parity + goldens anchor there); bedlam-platform
scales to any output. Default integer nearest-neighbor + 4:3 pillarbox; fit/fill/
smooth options; hi-res world composite + extended widescreen viewport are allowed
as later opt-in non-parity modes (gameplay-affecting, UI-flagged). Recorded in
PLAN.md P3 (render contract) + P6 (modernization). Smacker video: decode native,
present scaled. No changes to sim/timing (100Hz/20fps untouched).

## D10 - EXW tick architecture resolved; sim/render loop located (2026-08-17)
100Hz TimerCallback@0044de58 = SERVICE tick (mouse poll via GetCursorPos +
clamp+store, 5 free-running counters, 50Hz sub-gate, input-driven scroll
clamp 9..631/9..463, 8-frame palette cycle 0x90..0x97 @12.5Hz). The sim/render
game loop is the WORKER THREAD started by GameThreadStart@0044d9c0
(CreateThread-style via .data slot 00457874, start 0x0044dea0, stack 0x1000,
handle 004ef698, id 004ef694) - the 8street 20fps claim stays unanchored
until 0044dea0..0044dfec is decompiled (top P2 task). F key = screenshot to
numbered BMP (FKeyHandler@0044ceb0), NOT fullscreen toggle. AppActivate@
0044b1c0 = system-palette management, not pause. Names applied in project:
TickWorker / MousePosHandler / ThreadSpawnThunk / FKeyHandler / AppActivate.
Write-up: docs/RE-EXW-TICK.md (script: tools/ghidra-scripts/ExwTickFollowup.java).

## D11 — Bedlam 2 support scope (2026-08-17, user requirement)
User wants Bedlam 2: Absolute Bedlam (1997) supported in the same engine. Evidence
acquired same day: IA rip (complete vs code refs, provenance in RESEARCH-BEDLAM2-CENSUS.md);
data layer confirmed format-identical (89% first-pass parse with unmodified tools).
Decision: ONE engine, parameterized content. bedlam-assets serves both corpora;
bedlam-game grows per-game content packs (B1/B2) with shared systems code; parity
work continues B1-first (canon EXW), B2 gets its own RE target (DOS BEDLAM.EXE)
and its own goldens. B2-specific divergences (zones A-F, no MRS music, VESA modes)
are tracked in the census doc until a B2 plan phase is cut (post-P5 gate).

## D12 — High refresh rate support, 240Hz+ (2026-08-17, user requirement)
Same split as D9: sim untouched, presentation free. Accumulator loop presents
vsync-locked at any refresh or uncapped; logic remains fixed-timestep at the
original rate (charter-safe). Camera/scroll interpolation makes high refresh
visibly worth it; sprite sub-pixel interpolation stays an off-by-default option.
Frame pacing at 240Hz added to game-feel proxies. Historical note: Bedlam 2
itself shipped VESA modes up to 1440p — the devs were already moving this way.

## D13 — EXW pacing verdict: 50Hz gate, not 20fps (2026-08-17)
> SUPERSEDED in part by D15 (2026-08-17 tick2 run): 004ede10 turned out to be
> the palette-fade countdown, not a frame gate. 20fps stays refuted; the
> 50Hz parity assumption is withdrawn pending the FUN_0043d00b body.

GameThread@0044dea0 (worker thread) decompiled: 59-byte trampoline around
GameMain@0041c050. Neither contains Sleep/timeGetTime/WaitForSingleObject —
the 8street "20fps sim/render" claim is REFUTED for EXW at this depth. Pacing
architecture [verified]: 100Hz timeSetEvent -> TickWorker -> counter 004edbc8
bit0 + gate 004ede10 -> 50Hz update (FUN_00425901); the gameplay advance
(FUN_0043d00b, second hop) reads the same gate. Parity budget therefore
assumes 50Hz fixed logic tick (with 12.5Hz palette phase) until the second
hop (FUN_0043d00b / FUN_00440e45 bodies) proves a further subdivision — any
such rate must derive from the 50Hz gate. Write-up:
docs/RE-EXW-GAMETHREAD.md (script: tools/ghidra-scripts/ExwGameThread.java).

## D15 - 004ede10 = palette-fade countdown; sim/render rate UNKNOWN again (2026-08-17)
> RESOLVED by D16 (2026-08-17 pacer run): the mechanism is now known -
> present-paced (DD Lock/Flip/Blt with wait semantics), no software frame clock.
> Items (1) and (2) below stand.
The tick2 run (ghidra-project/exw-tick2.txt) decompiled FUN_00425901 =
FadeStep: 768-channel 16.16 fade accumulator -> 6-bit palette upload
(SetPaletteRGB@0044aed4) -> decrement 004ede10. FUN_0041cbf0 = FadeSetup
(arms 004ede10 = step count; GameMain uses 10-step = 200 ms fades);
FUN_00420100 cancels; GameMain clears at boot. CONSEQUENCES: (1) the D13
claim "pacing = 100Hz tick -> 50Hz gate 004ede10; parity budget 50Hz" is
WITHDRAWN - 004ede10 is nonzero only during fades, so it cannot pace the
sim/render loop. Verified rates today: 100Hz service tick, 50Hz fade
advance while fading, 12.5Hz palette cycle. (2) FUN_0043d00b reading
004ede10 is a fade-status check, not a rate gate. (3) The sim/render pacing
mechanism is UNKNOWN - the GameMain second hop (FUN_0043d00b / FUN_00440e45
bodies, divider consumers like FUN_00448ef1 reading 004edbc8) must establish
it before any parity rate is committed. The 8street 20fps claim stays
refuted at this depth (no sleep /5 divider on the game thread). Also
resolved this run (facts, recorded in RE-EXW-TICK.md): GoFlagSet caller =
FUN_0041e19d (release-the-timer at boot); 00457874 slot ->
ThreadSpawnImpl@0045204b = Watcom CRT _beginthread-style wrapper -> real
CreateThread with CRT trampoline 00451fbc; AppActivate +0x18 =
IDirectDrawPalette::SetEntries on 004ee9d0 (stock layout); surface vtables
+8-shifted past GetCaps vs stock IDirectDrawSurface (game ddraw.h has 2
extra slots there; cosmetic for RE).

## D16 - Sim/render rate = DirectDraw present-paced (vsync-locked); no software frame clock (2026-08-17)
The GameMain second hop + pacer passes (ghidra-project/exw-gamemainhop.txt,
exw-pacer.txt, exw-pacer-names.txt; scripts ExwGameMainHop/ExwPacerFollowup/
ExwPacerNames) closed D15 item (3). The mission loop (FUN_0043d00b) has NO
software rate gate: each pass = poll -> pure-logic sim/render -> PresentCopy
(SurfaceLock spin-until-DD-Lock-succeeds + 480 row MemCopies + Unlock) ->
PresentEnd -> DDFlipOrBlt (fullscreen Flip@vt+0x2c / windowed Blt@vt+0x14),
g_frame_count++ once per pass. Census: Sleep has exactly one caller (wrapper
FUN_0044e1ca, shutdown paths); WaitForSingleObject exactly one (FUN_00451b62
= Watcom CRT recursive mutex); 004edbcc waits = attract-mode input waits
(20 s timeout); 00448ef1 divider reads = menu change-detection. Therefore one
frame per completed present = display-flip rate: vsync-locked, 60Hz-class on
period hardware. Cinematics add _SmackWait (Smaker-internal timing).
PARITY BUDGET consequence: the reimplementation must NOT derive logic timing
from host vsync (breaks determinism, PLAN sec 7). Per the D9/D12 sim/present
split: canonical sim = fixed timestep at a nominal 60Hz (the period-norm
refresh; revisitable if period sources show another dominant mode),
presentation = host vsync / uncapped. Also this run: EXW surface vtable =
stock IDirectDrawSurface + ONE extra slot at 0x0c (uniform +4 from Blt@0x14
through Unlock@0x80; tick2 note corrected in RE-EXW-TICK.md); DD/palette/
clipper objects stock (8 anchors). Full write-up: docs/RE-EXW-PACER.md.

## D14 — Decoder home: engine/bedlam-assets; inspect is a thin CLI (2026-08-17)
tools/inspect's format decoders promoted into workspace crate
engine/bedlam-assets per PLAN P3: pure buffer-in/out (no fs/env/wall-clock),
thiserror-typed errors (AssetsError/CodecError) whose Display strings are a
STABLE CONTRACT (inspect embeds them verbatim in its status/detail fields),
and never panic on user-supplied bytes (panic = engine bug). tools/inspect
keeps only walking/I/O/JSON/PNG serialization. Promotion proven
behavior-preserving: inspect output over the full 1069-file corpus is
byte-identical to the pre-refactor HEAD when invoked identically;
engine/bedlam-assets/tests/corpus.rs locks that in (deterministic 80-file /
21-family sample: parse no-panic, 13 byte-exact format rebuilds, 20 rle16 +
20 byterle codec round-trips, free-fuzz across 22 buffer sizes).

## D17 — bedlam-core timing model: hybrid fixed-sim / per-frame polling (user direction, 2026-08-17)

User decision recorded after D16. bedlam-core uses a hybrid: (a) sim/physics
= FIXED 60Hz timestep accumulator (never dt) so replay + state hash stay
exact per PLAN sec 7; (b) input polling, UI hit-tests, cooldowns, cursor,
audio/video = per-frame at host refresh (dt acceptable) - mirrors the
original architecture (per-frame poll in the present-paced loop + 100Hz
service satellites); (c) satellite clocks as integer substeps of the sim
tick (100Hz service = 5 per 3 ticks, 50Hz fade while fading, 12.5Hz palette
cycle). Determinism test: same input script -> identical SIM state hash at
15/60/240Hz host; frame-rate-driven systems are excluded from the hash.
Spec is pinned on the queue task (.state/NEXT.md) as well.

## D18 — B2 LE import strategy: yetmorecode lx-loader into the watcom Ghidra build, raw-binary postScript as final fallback (2026-08-18)

BEDLAM.EXE (game-data-2) is a Watcom LE/DOS4GW image; stock Ghidra has no
LE/LX loader (issue #532 open since 2019, verified against local 12.1.2
Base.jar). Plan of record for the B2 import task: PRIMARY = yetmorecode/
ghidra-lx-loader (Apache-2.0, DOS/4 LE support, full fixups) installed
into ~/ghidra-12.1.2-watcom/Ghidra/Extensions (user-writable install that
carries x86openwatcom.cspec; /opt is root-owned), smoke-tested on a
throwaway project first, source-rebuilt against 12.1.2 if the prebuilt
v12.0.1 zip is version-rejected. FALLBACK = oshogbo 1.7 (no license: run
only, never copy). LAST RESORT = raw binary + -process postScript
building the two blocks from the verified anatomy table (CODE 0x10000
size 0x66970; DATA 0x80000 vsize 0xb04ee, 0x648f file-backed). One
import into BedlamWatcom ever, then -process only. Full anatomy + plan:
docs/RESEARCH-BEDLAM2-CENSUS.md (B2 Ghidra import plan section; consolidated
there 2026-08-18 after a parallel-run duplicate was folded in - entry-point
reading corrected to object-relative eip 0x56a60 -> linear 0x66a60).

## D19 — P4 runtime pinning: flathub user-install (commit-pinned, repo-local XDG_DATA_HOME) + wow64 wine prefix (2026-08-18)

The queue said "DOSBox-X AppImage"; upstream ships no Linux release binaries
anymore (verified across the last 6 tags on 2026-08-18), so the pin is the
flathub build `com.dosbox_x.DOSBox-X` 2026.08.02 @ commit fa89039c... ,
installed --user with XDG_DATA_HOME pointed INTO the repo (gitignored
runtime/xdg) so the install is repo-contained and needs no sudo. Wine side:
wow64-mode wine 11.15 (win32 WINEARCH unsupported there), one 64-bit prefix at
runtime/wine-exw with mono/gecko/menus disabled. Both carry wrapper scripts
(tools/runtime/dosbox-x.sh, wine-exw.sh) + full provenance in docs/RUNTIME.md.
Consequence: goldens/harness references are only valid against these pins;
upgrades require re-baselining (PLAN P4 "pin dosbox/wine versions + configs").

## D20 — wgpu presentation backend + dual resolution modes (user decision, 2026-08-18)

Use wgpu for GPU-accelerated presentation and modern rendering portability
(Vulkan on supported Linux systems, DX12 on Windows, Metal where supported;
backend selection remains wgpu-owned rather than raw Vulkan code). Two explicit
modes share the same deterministic simulation and assets: PARITY mode renders
the canonical 640x480 indexed framebuffer + 6-bit palette and GPU-scales it to
any output resolution; ENHANCED mode may render supported world/UI elements at
the native output resolution for additional detail. Enhanced output is
non-parity and UI-flagged. Resolution, GPU timing, interpolation, and backend
choice never feed back into simulation or hashed state. This specializes D9
and D12; it does not remove the canonical frame used by goldens and regression
tests. Initial implementation target: wgpu upload/palette-expand/fullscreen
triangle scaler, then native-resolution enhanced passes incrementally.

## D21 — Enhanced-layout aspect scope: 16:9 + 16:10 (user decision, 2026-08-18)

wgpu presentation continues to accept any output resolution, but purpose-built
ENHANCED layouts and AI-assisted background extensions are guaranteed only for
16:9 and 16:10. Author enhanced menu/background masters at 16:10 with a 16:9
safe region: controls, text, and gameplay-critical information must remain in
the shared safe region; the additional 16:10 height is optional decorative/UI
buffer. Gameplay supports both aspect ratios with resolution-independent
cursor/world mapping; any increased visible world remains an explicit
non-parity option per D9/D20. Parity mode remains canonical 4:3. Other aspect
ratios use fit/letterbox/pillarbox rather than bespoke layouts. AI-generated
asset derivatives live in external HD packs; git tracks only tooling, recipes,
manifests, masks, provenance, and hashes.

