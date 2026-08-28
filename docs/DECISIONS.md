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



## D22 - B2 (DOS) timing architecture = same two-clock design as EXW; parity budget carries unchanged (2026-08-18)

RE verdict from the B2 entry/tick naming run (RESEARCH-BEDLAM2-CENSUS.md sec
6.2): Bedlam 2 DOS installs a 100.01 Hz PIT INT-8 hardware ISR on demand
(divisor 0x2e9b, DOS INT 21h AH=25h vector set, NOT DPMI; immediate EOI,
drop-not-queue reentrancy lock) driving seven counters, the 12.5 Hz palette
bank cycle over 0x90..0x97 (byte-identical behavior to the EXW
TimerCallback), 50 Hz mouse polling + clamping, and the play-clock divider;
presentation is vblank-locked (double-poll of 0x3da bit 3). That is the same
architecture the EXW analysis found (100 Hz Win timer + present-paced
render), so the reimplementation parity model of D16 (fixed-rate sim,
present-paced frames, hashed satellites keyed to the 100 Hz service clock)
applies to BOTH builds with no new timing concept. B2-specific deltas the
engine must parameterize: game-coordinate space 320x240 (EXW 640x480 - the
mouse clamp globals prove it; canonical-frame policy D9 keeps EXW 640x480 as
the parity anchor, B2 coordinate handling is an open engine question), and
zone/mission progression via lookup tables (6 zones x {4 regular + 2 alt},
27 linear missions) instead of the EXW 7x5 arithmetic.

## D23 - B2 mission-loop pacing verified present-paced (vblank); INT8 counters are services/timeouts only (2026-08-18)

Episode-loop run (RESEARCH-BEDLAM2-CENSUS.md sec 7): the B2 (DOS) mission
loop iterates exactly once per PresentFlip, and PresentFlip is a VESA page
flip that waits vblank (0x3da double-poll via WaitVRetrace, gated by
g_wait_vsync) - with ZERO reads of any of the seven INT8 counters inside the
loop. Counter roles are now fully classified: audio tick/position bases
(0x801a6, 0x80010), ISR-internal phases (0x11f0c8), 100 Hz screen-timeout
base (0x11f0c4), a 50 ms micro-delay (0x11f0b0), and two dead counters
(0x11f158, 0x11f0b4). This is the concrete DOS-side confirmation of D16:
fixed-rate sim + present-paced frames, 100 Hz service clock for satellites.
Implication for bedlam-core: the existing hashed-60Hz Sim + present-paced
presentation split is the right shape for BOTH builds; nothing in the B2
timing fabric argues for a different architecture. New engine-relevant B2
facts to parameterize: 640x480x8 physical framebuffer (VESA 0x101, 640-byte
stride, double-buffered pages bank {0,5}) with a 320x240 logical/mouse
space (2x pixel scale) - vs EXW native 640x480 logical; and audio at 11025
Hz sharing IRQ0 with the game tick in the DOS build (HMI-style PIT
reprogramming) - a PC-specific detail the reimplementation abstracts behind
the fixed-rate service clock.

## D24 - P3 presentation skeletons landed; wgpu pinned at 27.0.1 for the skeleton (2026-08-18)

D20 initial target implemented (ff8fb17 bedlam-render + d2b7fb8
bedlam-platform). bedlam-render: pure state -> canonical Frame (640x480x8
indices + 256x6-bit palette + palette_dirty), parity_hash = FNV-1a over
indices then 6-bit palette, fixed pass order, camera clamp 9..631/9..463,
alpha ignored with prev_sim = None (golden config). bedlam-platform: pure
Integer/Fit/Fill + nearest/linear presentation geometry, wgpu parity
pipeline (R8Uint index texture per frame, R32Uint packed 6-bit palette
uploaded only on palette_dirty - the 004ee9b6 handshake analog,
fullscreen-triangle WGSL expands Original v<<2 default / Full then scales;
indices never interpolated). Dependency pin: wgpu 27.0.1 + pollster 0.4.0
(Cargo.lock) - a mature line rather than 30.x latest; the FINAL version
call stays with the P4 dependency spike (PLAN sec 6 P4 item 1) and a later
major bump is presentation-only surgery because goldens and the parity
hash never touch the GPU path (CPU-side over the canonical frame).
Boundary verified: platform consumes bedlam-render Frames only (never
bedlam-core), no clock in either crate, forbid(unsafe_code), the only
float in render is the presentation alpha. Workspace 153 tests green,
fmt + clippy -D warnings clean; the GPU test skips gracefully without an
adapter so the ubuntu/windows CI matrix stays deterministic.

## D25 - bedlam-audio volume domain: linear Q8 gains over the EXW product; dB curve stays documented, not reproduced (2026-08-18)

The shipped EXW delivers per-note volume to DirectSound as hundredths of
decibels: SetVolume = ((master * vol) / 0x30 - 0x7f) * 0x7d0 >> 7 (RE-EXW-
MUSIC sec 6, formulas straight from the listing). The reimplementation mix
graph keeps the INTEGER PRODUCT (master * vol) / 48 as the gain domain -
master 0..=127, raw stream volume byte - but linearizes it to a Q8 gain
min(256, (master * vol * 256) / (127 * 48)) instead of walking the dB curve.
Rationale: the dB-to-linear conversion has no exact integer form worth
faking, the DirectSound internal conversion is not part of the binary we
are matching, and PLAN P4 pins audio parity to a correlation band on the
downsampled mix (never exact bytes), which does not require the dB curve.
The linear map is monotone in the same product and clamps at unity because
the DS domain is attenuation-only. Same-unit companions: pitch keeps
RATIO_TABLE 16.16 values verbatim as the per-sample phase step (unity
0x10000 replays at 11025 Hz = the SetFrequency (ratio * 0x2b11) >> 16 fact),
and pan is a linear balance with the shipped game pinned to center.
DESIGN-AUDIO.md sec 6 carries the full semantics; bedlam-audio implements
them with 23 tests (9 unit + 14 determinism gates, miri-clean).


## D26 - bedlam-game scene actions are HASHED per-tick edge derivations, not per-frame input events (2026-08-18)

The EXW polls input per frame (KeySink@0041be05 latches sampled at
100 Hz; MouseSink@0041bf35 bit0/bit1) - per-frame business, i.e. D17
bucket b, unhashable. The scene FSM in bedlam-game nevertheless needs
deterministic scene ACTIONS, so SceneFsm::tick level-samples the SAME
consumed per-tick InputFrame the sim consumed (mouse bits 0/1, edge =
rising transition against the previous consumed tick) and derives
Advance/Back INSIDE the hashed bucket; the latch byte (prev_mouse)
is itself hashed state. This mirrors the 100 Hz KeySink latch shape
while keeping the whole scene machine a pure function of the tick
input sequence - which is what makes the 15/60/240 Hz host-hash
identity test exact (per-frame edge detection could never be hashed).
A held button across a scene boundary does not re-fire (the P-latch
clear analog, RE-EXW-INPUT); UI intents (Options/Quit) and sim
outcomes (MissionComplete/MissionFail) stay host-applied via
fsm.apply. DESIGN-GAME sec 7 carries the boundary table.

## D27 - MusicPump selects the MELODY chunk, never the chunk-1 loop timer (2026-08-18)

Every shipped .MRS has chunk 0 disabled and chunk 1 = the LOOP TIMER:
a single unconditional pattern RESTART whose delta equals its table-B
entry equals the song length (RE-EXW-MUSIC sec 2, assets corpus test).
Walking "the first enabled chunk" would therefore sequence a one-event
timer and stay silent. MusicPump::new picks the first enabled chunk
whose walk yields at least one Note event (melody streams start at
chunk 2 in the shipped corpus); restart() rebuilds the script from the
same bytes as the loop analog. The terminal-kind corpus gate accepts
Freeze | Restart for enabled chunks, mirroring validate_mrs_song
(the chunk-1 restart IS the loop mechanism, not an anomaly).
Companion bedlam-audio fix: Mixer::load_script anchors dispatch at
the ATTACH cursor (script tick 0 = the attach frame), because the
scene pump swaps scripts mid-stream on every scene change and
absolute-cursor dispatch would fire the entire past at once.

## D28 - Parity harness v0 pins: example-not-crate, 184-sample audio pull, sibling .MRW banks for a non-degenerate audio baseline (2026-08-18)

c61d7f7 adds engine/bedlam-game/examples/parity_harness.rs (P4 kickoff,
CPU half only; D24 defers the GPU/device half). Four pins:

1. LOCATION: an example, not crate code and not a tools/inspect arm.
   The bedlam-game hermetic rule (no fs) holds; std::fs on the game
   tree exists ONLY inside the example, behind FsSource (ByteSource
   resolving top-level names then SOUND/MIDI, mirroring the install
   tree). cargo run --example is also the cheapest possible arm - no
   workspace membership, no new deps, no CI change.
2. AUDIO PULL: 184 samples/frame = ceil(11025/60), even length. Mixer
   output is chunking-invariant on its Q16 grid (D25), so the pull rate
   only decides how much stream lands per frame - the harness PINS it
   so the audio stream FNV is reproducible across runs, machines and
   toolchains. Pin, never derive: 183.75 is not integral and the drift
   would make per-frame comparisons meaningless.
3. MRW BANKS: a .MRS script without its sibling .MRW bank loads zero
   waves and the audio stream is all-zero - byte-identical for ANY
   implementation, worthless as a parity anchor. The harness loads the
   sibling bank for every track (54 waves across the 5 shipped tracks),
   mirroring the EXW mrw_load/load_midi pairing (RE-EXW-MUSIC sec 6).
4. BASELINE ANCHOR: report format bedlam-parity-harness/0; the embedded
   walk (775 frames = 775 ticks, seed 0, dt 4 subticks) yields scene
   chain 0xcae25cd08d7cbc08, sim 0x72979d5d9dedc832, frame parity
   0x87263f149564ad25, audio stream 0xc862e45d2e95ad29 (176994 nonzero
   samples). Same script + same corpus MUST reproduce these on any
   OS/toolchain before a wine/DOSBox runtime diff means anything; drift
   in these values with unchanged inputs = an engine determinism bug,
   never a harness bug to paper over.

## D29 - DOSBox-X differential-harness sandbox + config pins (2026-08-18)

Context: P4 runtime half, unattended-safe subparts (the interactive launch
stays gated). The harness compares the original B2 DOS build under the
pinned DOSBox-X against the D28 CPU baseline of parity_harness.

1. SANDBOX: the flathub app carries a STATIC FINISH ARG filesystems=home
   (whole home rw). Per-path :ro override grants are illusory - the
   permission union takes the most permissive grant. Verified empirically
   via flatpak info --show-permissions. Correct posture: --reset, then
   --nofilesystem=home + --filesystem=<repo>/runtime. game-data becomes
   invisible to the emulator (write isolation by construction) and the
   corpus is reached via an rsync scratch copy runtime/harness-corpus
   (writable C: for saves); output goes to runtime/harness-out as D:.
2. CORE PIN: core=normal (interpreter) - watchpoint/debugger accuracy and
   the most reproducible core across hosts; dynamic recompilation caches
   are not a stable baseline for watch diffs. cputype=pentium pinned
   explicitly (auto = a 486 tolerating Pentium insns, a vaguer target).
3. CYCLES: fixed 60000 (approx Pentium-100 class) as the STARTING PIN -
   chosen, not calibrated: the value is frozen until the first interactive
   calibration run; any change is a pin change + golden re-baseline (D19
   discipline). fixed (not auto/max) so the per-frame cycle budget is
   host-independent.
4. MACHINE/VIDEO: svga_s3 for VESA VBE banked mode 0x101 (B2 pages {0,5},
   census 7.7d; UNIVBE must NOT run - svga_s3 supplies VBE natively);
   vmemsize=2 MB (dual 300KB pages + cursor block) in the [video] section
   (its canonical home in 2026.08.02); memsize=16.
5. GOLDEN-PATH PURITY: render scaler=none + aspect=false (raw framebuffer);
   mixer sample accurate=true rate=48000 + sbtype=sb16 220/7/1/5 (the HMI
   driver class the B2 corpus probes) so the 11025 Hz native stream is
   host-resampled exactly once, reproducibly.
6. HEADLESS SMOKE GATE: driver smoke mode boots DOS on dummy A/V, mounts the
   scratch corpus + D:, dirs the corpus root into D:SMOKETST.TXT and exits;
   GATE = the file lists BEDLAM.EXE (672399 B) and DOS4GW.EXE (265396 B).
   This is the unattended-safe regression test for the whole sandbox+
   conf+driver stack; it passed first-hand 2026-08-18.
7. WATCH SET: census-verified addresses only (RNG pairs 0x11ef18/1a +
   0x11ef1c/1e per census L301/307, fade 0x11ef88, palette bank 0x11f138,
   display start 0x11ef38, flip lock 0x8008e, campaign linear 0x12576c,
   mode flag 0x11f11c, timeout 0x11f0c4, ISR phase 0x11f0c8, saves
   0x8b1d4) + PresentFlip@0x1066b as the frame trigger + PcmMixerService@
   0x136e0 for the audio side. Debugger command names and the startup.js
   route are UNCERTAIN until verified in-session (checklist in the
   skeleton). Three addresses from a dead sibling draft (0x11ef7c, 0x11f0c0,
   0x11efc4) had zero census/STATE backing despite verified tags and were
   DROPPED - close-out verification is not optional even for ghost
   survivors (75b17a8 lesson, second occurrence).

## D30 - P4 SMK video decoder: `smk` 0.1.0 behind a codec-neutral seam (2026-08-19)

Context: PLAN sec 4 P4 spike - pure-Rust smk fork (RESEARCH default) vs
libsmacker-sys; decided unattended per the queued unit.

1. CHOICE: `smk` 0.1.0 (crates.io, A. Rosetti, libsmacker 1.2.0 port) as
   the decode backend, WRAPPED. bedlam-assets/src/smk.rs exposes a
   codec-neutral `SmkStream` API (own info / frame-status / track-meta
   types + AssetsError mapping); the smk crate is an implementation
   detail the engine never names. Swapping to a vendored fork or
   libsmacker-sys later = rewriting one wrapper module, no engine-wide
   change. bedlam-assets stays buffer-in/buffer-out, deterministic, no
   filesystem/clock in library code, forbid(unsafe) in OUR crate.
2. BACKEND AUDIT (verified 2026-08-19 from the crate source): zero
   `unsafe` in src, only dependency `log` (no-op without a logger, so no
   nondeterminism), memory path (`open_memory`) touches no fs; error
   surface is a small closed enum -> stable mapping to
   AssetsError::SmkTruncated / SmkInvalid / BadMagic. Loud upstream
   caveats: f64 in its SmkInfo (our API re-derives exact integer
   us-per-frame from the raw header field), audio tracks ship DISABLED
   by default (our wrapper enables video + all tracks deterministically).
3. LICENSE CAVEAT: smk 0.1.0 is LGPL-2.1-or-later. Static Rust linking
   means any DISTRIBUTED binary carries LGPL obligations (source
   availability + relinkable objects, or backend swap). Current
   personal-use posture: acceptable; recorded so it cannot be forgotten.
   Vendored-fork follow-up queued only if the posture changes.
4. HEADER-LAYOUT BUGFIX riding this unit: the P1-era `parse_smk_header`
   read tree_sizes [52,56,60,64] + audio_rates from offset 68; the real
   layout is tree_chunk_size@52, tree_size[4]@56..72, audio_rate[7]@72..
   100. Evidence 3-way: smk-crate parse order, libsmacker header layout,
   and TITLE.SMK physically (0x44 holds 0x6c50 = a tree size; 0x48 holds
   0xc0002b11 = textbook exists|DPCM|mono|8bit|11025Hz rate dword).
   Inspect CLI smk JSON keeps its schema; field VALUES corrected.
5. GATE: corpus-skipping TITLE.SMK test (tests/smk_title_gate.rs) pins
   640x320 / 1227 frames / 66660 us per frame (15 fps) / no ring / no
   y-scale / track 0 = DPCM 8-bit mono 11025 Hz, and proves two full
   decode passes produce identical SHA-256 chains over pixels+palette
   and identical audio packet counts/bytes. Fingerprints recorded in
   NEXT.md only - decoded media never enters git.


### D30 follow-up - seam implemented, vendored patch, gate passed (2026-08-19)

1. SmkStream seam implemented exactly per the lib.rs/tests contract:
   SmkStreamInfo (integer us_per_frame re-derived from the raw rate field),
   SmkFrameStatus, SmkYScale, SmkAudioTrackMeta (Copy) and SmkAudioCodec
   {Raw,Dpcm,Bink} derived from the container rate dwords. enable_all(0xFF)
   happens inside open(); the forbid(unsafe_code) lib.rs WIP was adopted.
2. PANIC/OOM POLICY: SmkStream::open pre-validates structure and caps every
   allocation the backend would make: raster <= 16 MiB and 4-aligned in
   both axes (the backend writes whole 4x4 blocks unguarded), frames in
   1..=1_000_000, per-tree size <= 4 MiB, per-track max-buffer <= input
   size, and frame table + tree chunk + sum(chunk sizes) proven inside the
   buffer (-> SmkTruncated). After validation the backend can only fail
   with mapped typed errors (SmkDecode/SmkTruncated/BadMagic).
3. VENDORED PATCH (NOTICE.md): render_dpcm clamps the stream-declared
   unpacked size to the track buffer (upstream indexed out of bounds on
   malformed data and left buffer_size > buffer, which made audio_data
   slice-panic) and errors when the buffer cannot even hold the initial
   samples. Clamp chosen over reject: behavior on well-formed streams is
   unchanged and TITLE.SMK output is byte-identical; reject would also be
   defensible if playback ever needs strictness.
4. HEADER-LAYOUT FIX implemented: parse_smk_header now reads
   tree_size[4]@56..72 and audio_rate[7]@72..100 per D30; inspect CLI
   schema unchanged, field values corrected.
5. GATE PASSED (TITLE.SMK, corpus-skipping, two identical full passes):
   frames=1227 video_sha256=6aa75c55a68ab877429fea4216e730f62c281b46f75b3d27f2437fb8cd82cdd1
   audio_sha256=73fdee8e95328c4733e3b0f135bc26af975cbd335081e79590a8c1929940c6e3
   packets=1212 audio_bytes=901752 (duration-consistent with 81.8 s at
   11025 Hz mono). Fingerprints only; decoded media never enters git.
6. Mid-run incident, recorded for provenance: at 13:37 an unattributed
   writer appended the interrupted predecessor smk.rs draft (438 lines,
   self-inconsistent, and with one invariant that would reject TITLE.SMK:
   requiring unpacked tree sizes to fit inside the compressed tree chunk)
   onto the freshly written module. No process held the file afterward and
   it did not recur (45 s canary plus the full test window). Usable test
   ideas were ported to the contract API; the raw fragment is archived at
   .state/scratch/smk-predecessor-tail-20260819.rs.

## D31 - P5 title-movie playback integration: compose-level MovieFrame, x240-us fixed-step MoviePlayer, mixer PCM stream bus (2026-08-19)

Context: NEXT item 1 - integrate decoded TITLE.SMK playback into
GameHost/presentation. Restart-lineage overlap: TWO unattended workers
(1787165989 stopped 22:57, 1787172789 stopped ~22:58) left interleaved
WIP on the same unit; claim holder 1787161109 reconciled both slices
per the AGENTS.md adoption rules (all fragments archived under
.state/scratch/collision2-20260819T2256/ and the earlier
mixer-collision snapshot). Both lineages independently converged on
the same D31 shape below; the reconciliation unified them.

1. COMPOSITING (amends DESIGN-RENDER sec 8.4 per its own deviation
   clause): the canonical Frame IS the presentation seam in this
   headless-first architecture, so a movie is NOT kept on a parallel
   path - RenderInput carries an optional borrowed MovieFrame
   (bedlam-render compose) and render() REPLACES the scene pipeline
   with it while present: centered (letterbox, anchor y=80 for the
   640x320 title raster), clipped, never scaled, palette_dirty every
   frame (the decoder swaps palettes per frame; the row-dirty rule
   applies). Placement [design]: exact centering until the EXW
   title-screen RE (FUN_0044567c movie-runner body) lands. ONE blit
   implementation: Frame::blit_indexed (frame.rs); blit.rs free fns
   delegate; the superseded render/movie.rs owned-snapshot type was
   dropped (archived).
2. PALETTE FOLD: the vendored decoder expands 6-bit Smacker palette
   components through PALMAP = (v << 2) | (v >> 4); since
   PALMAP[v] >> 2 == v for v < 64, MoviePlayer folds decoded 8-bit
   entries back with >> 2 to the canonical Vga6 form - lossless both
   ways. bedlam-smk PALMAP made pub for the pinned proof
   (NOTICE.md export patch).
3. CLOCK: MoviePlayer (bedlam-game movie.rs) drives the SmkStream seam
   on the host fixed-step grid in x240-us integer units (1 sub-tick =
   1_000_000; frame period = us_per_frame * 240): no dt division, no
   floats, no wall clock; fractional periods bank (TITLE = 15.9984
   sub-ticks per frame at 15 fps). Ring streams wrap; non-ring latch
   finished and hold the last frame; a 4096-frame runaway guard drops
   the accumulator.
4. AUDIO STREAM BUS (bedlam-audio mixer): queue_pcm_u8 queues decoded
   native-format PCM (u8 mono 11025 Hz - exactly the DPCM track 0
   decode of TITLE.SMK) on a FIFO byte channel consumed one byte per
   stereo output frame UNDER the voices; gain follows the master knob
   at mix time (a stream has no spawn point to snapshot). Cap
   STREAM_CAP_BYTES = 16 MiB (16x headroom over the whole 901752-B
   TITLE track) fails loud (AudioError::StreamOverflow) instead of
   dropping; the host treats overflow as stop-movie (deterministic
   self-termination, never a pump error). Chunking-invariance
   unit-pinned alongside the existing determinism suite.
5. HOST LIFECYCLE (D17 bucket b, provably hash-free): GameHost
   load_movie(scene, bytes) is INERT until the FSM enters the target
   scene (Title): entry starts playback and queues frame-0 audio;
   leaving the scene drops the slot and clears the stream; mid-play
   decode failure or audio overflow self-terminates identically. The
   per-pump scene-hash chain with and without a movie is
   byte-identical (unit + gate pinned).
6. GATE: tests/title_playback_gate.rs (bedlam-game, corpus-skipping)
   drives a FULL TITLE.SMK playback through GameHost at 60 Hz and pins
   (a) exact pacing - frame k decodes on pump ceil(k*15_998_400/
   4_000_000)-1 after the Boot->Title transition pump (playback starts
   mid-pump on the transition), sampled k = 1,2,3,5,600,1226; (b) the
   composite frame equals an independent SmkStream walk (full 640x320
   raster + folded palette) at frames 1/600/1226; (c) two full
   playbacks byte-identical (SHA-256 over per-pump frame parity hashes
   + rendered audio); (d) scene-hash isolation vs a movieless host.
   Workspace 280 tests green; the existing D30 double-decode gate is
   untouched; manifests verified before/after the corpus-touching runs.


## D32 - P5 cutscene-movie selection + SMK corpus inventory gate (2026-08-20)

Context: NEXT item 1 - extend the D31 playback integration to the
remaining cutscene movies (game-data SMK corpus inventory first) and
wire the Cutscene scene to LOAD_UK/US.BIN-backed movie selection.
Interrupted-WIP adoption: an interrupted predecessor lineage left this
unit ~90% built in the tree (engine/bedlam-game/src/movies.rs untracked,
host.rs/lib.rs modified, smk_corpus_gate.rs untracked); claim holder
1bd01455 adopted and validated it per the AGENTS.md adoption rules
(snapshot archived under /tmp/opencode/wip-snapshot-1bd01455/), then
finished docs + queue bookkeeping.

1. CORPUS INVENTORY (tests/smk_corpus_gate.rs, bedlam-assets): opens
   EVERY .SMK under game-data/BEDLAM/GAMEGFX through the SmkStream
   seam and pins, per file: raster (w x h), frame count, us/frame,
   ring flag, y-scale, and audio track 0 shape; the directory listing
   must match the table exactly both ways (a new corpus file fails the
   gate, a dropped one too). Ignored regen_inventory test is the only
   documented way to re-print the table.
2. REJECT-OR-MAP VERDICT (the D31 policy applied corpus-wide): every
   corpus file MAPS onto the existing playback path, nothing is
   rejected. Facts [verified 2026-08-20 against the corpus, confidence:
   pinned by the gate]: 34 files total - 25x BRF_{B..F}{1..5} + BRF_DROP
   (640x480, 33330 us ~30 fps, silent, BRF rings 512 frames); END/
   GAMEOVER/GTLOG_{UK,US}/LOGO_{UK,US}/ZONEDONE (640x480, 66660 us
   15 fps, DPCM mono/8-bit/11025 track 0; all ring except GAMEOVER);
   SHOP (640x480, 61 frames, 25000 us 40 fps, ring, DPCM); TITLE
   (already D31). ALL y-scale flags are SmkYScale::None - the D31
   letterbox-blit-never-scale compositing needs no y-scale handling,
   and no corpus file is rejected. All periods divide the x240-us grid
   exactly (33330x240 = 7_999_200; 25000x240 = 6_000_000;
   66660x240 = 15_998_400), so banking accumulators pace every file
   with zero rounding. The single audio shape corpus-wide is the one
   the D31 stream bus already consumes natively (no resampling owed).
3. MOVIE SELECTION MODULE (engine/bedlam-game movies.rs): pure name
   arithmetic over hashed state + caller region - selection is
   presentation-side (D17 bucket b), hash-free, and the host stays
   byte-source-free (DESIGN-GAME sec 8). cutscene_name(stage) [verified
   vs EXW LAB_0041c69e: ZONEDONE.SMK every zone completion, END.SMK on
   the _DAT_004edd8c == 7 endgame arm; EXW reads the counter BEFORE
   its post-movie ++ while Episode::complete() advances the stage
   BEFORE the Cutscene entry, so the endgame arm is exactly
   stage >= MAX_STAGE]. Region (the DAT_0046ae64 reimplementation:
   0 = UK, nonzero = US [verified via the string selects at
   RE-EXW-GAMETHREAD.md:145-146,225,280]) backs the LOAD_UK/US.BIN +
   LOADPAL/LOADPALU.PAL loading-screen pair [verified same call site]
   and the LOGO_{UK,US}/GTLOG_{UK,US} variant pairs; Region is
   deliberately NOT a GameConfig field (it is not an OPTIONS.BDL
   field; entangling it would deviate from the typed view) - callers
   pass it with the movie-name fns. briefing_name(zone_letter, sub)
   covers the BRF_{B..F}{1..5} corpus and rejects the rest [corpus
   census; the zone-letter map is not yet RE-ved - letter taken
   verbatim]. logo/gtlog/gameover/shop/title names cover their
   play sites [logo/gtlog verified; gameover/shop/briefing corpus-
   pinned, play sites not yet RE-d to the movie call].
4. CUTSCENE WIRING (host.rs): GameHost::cutscene_name() reads
   movies::cutscene_name over the hashed stage slot; load_cutscene()
   = load_movie(Scene::Cutscene, bytes) - the D31 lifecycle verbatim:
   inert until the FSM enters Cutscene (entry starts playback +
   queues frame-0 audio), dropped + stream cleared on leaving, hash
   chain untouched. Unit-pinned: selection walks the FULL_MASK cadence
   through all 7 zones (END exactly on the 7th zone completion, held
   at the MAX_STAGE ceiling), and load_cutscene plays + drops on the
   Cutscene scene like the Title movie.
5. GATES: smk_corpus_inventory_is_pinned (corpus, manifest-bracketed)
   + 5 movies.rs unit tests + 2 host.rs lifecycle tests; workspace 257
   tests green, fmt + clippy -D warnings clean, MANIFEST.sha256
   verified before AND after the corpus-touching runs. The D30/D31
   gates are untouched.

Not done here (queued): the post-cutscene BETWEEN.BIN + loading-screen
display, the boot LOGO/GTLOG attract sequence, briefing/shop backdrops
as live scenes - all callers of these name fns, once their host scenes
exist.

## D33 - P5 shop + briefing backdrop wiring: GameHost load_shop/load_briefing, movies briefing_name_for_slot (2026-08-20)

Context: NEXT item 1 - wire the Shop + Brief scene movie backdrops
into GameHost per the D31/D32 lifecycle (the D32 not-done-here
list). Clean tree at a56f669; no predecessor WIP to adopt.

1. SLOT-KEYED BRIEFING SELECTION (movies::briefing_name_for_slot
   (stage, mask) -> Option<String>): stage -> zone letter, lowest
   unset mask bit + 1 -> sub. Letter map [design, anchored both
   ways]: stage 1 = the BootCamp intro and stages 7..=8 = the
   endgame zone / post-endgame ceiling (EXW zone counter 1..7,
   7 = endgame, RE-EXW-GAMETHREAD fact table) select None - no BRF_A
   / BRF_G exists in the corpus, so the honest selection there is
   no briefing movie; stages 2..=6 map onto EXW zones 2..=6 =
   letters B..=F, exactly the 25-file BRF_{B..F}{1..5} corpus
   domain (the linear formula clamp((zone-2)*5 + level - 1, 1, 26)
   walks zones 2..6 = the 25 lettered levels; the D32 gate pins the
   domain). Sub arithmetic = the SAME lowest-unset-bit selection
   Episode::complete applies [design, open Q5]; the observable
   FULL_MASK cadence (masks 0, 1, 3, 7) selects BRF_*{1..4} only,
   so BRF_*5 stays corpus-resident but cadence-unreachable (the EXW
   5-level cadence files, like the mostly-absent B2 MISSION5,
   census sec 1). Total over the whole (stage, mask) domain: a
   transitional full mask 0b1111 lands on sub 5 (still corpus),
   masks with bit 4+ set and saturated masks land on sub > 5 ->
   briefing_name rejects -> None (an explicit sub < 8 shift guard
   keeps the walk defined). Note: the FSM deliberately carries the
   B2 FULL_MASK cadence (DESIGN-GAME fact 3) while the corpus is
   the EXW 5-level one; this map is the reconciliation point and
   gets re-anchored when the EXW zone/level reader RE lands.
2. HOST WIRING (host.rs, D31/D32 shape verbatim): GameHost
   briefing_name() reads movies::briefing_name_for_slot over the
   hashed episode slot (pure name arithmetic, byte-source-free);
   load_briefing(bytes) = load_movie(Scene::Brief, bytes) and
   load_shop(bytes) = load_movie(Scene::Shop, bytes) - the D31
   lifecycle: inert until the FSM enters the target scene (entry
   starts playback + queues frame-0 audio on tracks that have one -
   the BRF rings are silent), dropped + stream cleared on leaving,
   hash chain untouched (the D31 isolation test covers the generic
   load_movie path). The Shop selection stays the constant
   movies::shop_name() (SHOP.SMK, 61-frame 40 fps ring, D32 gate) -
   no host getter for a constant. Re-load per entry is the caller
   pattern (load_movie replaces the slot), which is what the Brief
   scene needs: the backdrop changes per mission.
3. TESTS (D32 shape): 3 movies.rs units (letter map + None domains;
   sub-follows-mask-bits incl. the transitional and rejected masks;
   every Some over the whole slot domain stays inside the 25-name
   corpus set) + 3 host.rs units (selection walks the FULL_MASK
   campaign: boot camp -> None, stages 2..=6 subs 1..4 ->
   BRF_B1..BRF_F4, endgame zone + MAX_STAGE ceiling -> None;
   load_briefing lifecycle inert-on-Title -> started-on-Brief ->
   dropped-on-exit; load_shop via the MissionFail -> Debrief ->
   Shop path, same lifecycle). Workspace 281 tests green, fmt +
   clippy -D warnings clean, MANIFEST.sha256 verified before AND
   after the corpus-touching runs.

Not done here (queued): BRF_DROP.SMK as the boot-camp/endgame brief
fallback (its play site is not yet located), the post-cutscene BETWEEN.BIN
+ loading-screen flow, the boot LOGO/GTLOG attract sequence.

## D34 - P5 post-cutscene loading flow: GameHost BETWEEN interlude + region loading screen (2026-08-20)

Context: NEXT item 1 - the EXW LAB_0041c69e zone-transition tail
(ZONEDONE.SMK -> BETWEEN.BIN -> LOAD_UK/US.BIN + LOADPAL/LOADPALU.PAL
-> forced DAC tail -> text row -> FadeSetup 10) as the GameHost flow,
on top of the DONE decode gate (8cc4951). Engine WIP from interrupted
predecessor 3977d55d (max-steps exit after verification, before
commit) adopted per AGENTS.md: snapshot under /tmp/opencode/
wip-snapshot-f807449c/, one doc-grammar fix applied, all gates
re-run green by the adopting run.

1. FLOW MODULE (game loading.rs, presentation-only, D17 bucket b:
   no sim/scene-hash contact - hash-isolation test): LoadingFlow
   phases Staged -> Between -> Loading. decode_entry0 runs BETWEEN/LOAD
   banks through the validated sprites parser (single-image 640x480
   corpus gate; any decodable entry 0 accepted - EXW draws entry 0);
   short/undecodable -> typed GameError::BadLoadingAsset, staging
   rejected without state change. loading_palette folds the 770B PAL
   through parse_vga770 + >>2 (lossless 6-bit round trip, the PALMAP
   argument) then FORCES entries 224..=255 = 0x3f3f3f - DAC commit
   buffer bytes 0x2a2..0x301, boundary-exact (0x2a2-2)/3=224,
   32 entries [verified fill; the later font-ramp copy into the same
   region belongs to the FULLFONT pass, queued].
2. PHASE SEMANTICS (host sync_loading): Between arms on Cutscene once
   the cutscene movie finished/absent - the interlude still owns the
   plane under the STANDING host palette (EXW makes no DAC change
   between movie return and LOADPAL commit; which palette the DAC
   held is open, FUN_0044567c exit path). Loading arms on the first
   post-cutscene Select entry: the LOAD raster under fade_palette
   (integer lerp from black, step k = target*k/10, monotone,
   drift-free), fade paced 10 steps x 20 ms at 50 Hz on the SAME
   x240-us accumulator grid as movie periods (chunking-invariant,
   pinned: 12x4-sub-tick pumps = 200 ms = step 10; saturation holds
   full brightness). A skip-advance before movie end bypasses the
   interlude visual but still runs the loading screen (the EXW tail
   is unconditional after the movie call) [design: no BETWEEN hold
   duration RE-ed]. Leaving the flow scenes, or reaching the endgame
   arm (stage == MAX_STAGE: END.SMK + credits loads NO BETWEEN/LOAD
   [verified code path]), drops an active flow; a staged flow waits.
   Zone math = the cutscene_name reconciliation: EXW reads
   _DAT_004edd8c pre-increment, so just-completed zone = stage - 1;
   tail armed stages 2..=7 (completed zones 1..=6), text row y=0x82
   x=150/180/210 + 260 only for completed zone 6 [verified coords];
   pinned as TextRow flow state for the FULLFONT glyph pass (not yet
   RE-ed) so it consumes without re-deriving zone logic [design].
   Region variant = movies::Region path selection only (UK==US
   byte-for-byte, 8cc4951 gate). The 310000/300000 FUN_0041db89
   allocs are decode scratch - internal representation, parity
   budget T3, not reproduced. render_now: loading plane > movie
   plane > scene pipeline (full-screen 640x480 centers at origin =
   the 8cc4951 gate 1:1 no-letterbox blit).
3. HOST SURFACE: load_interlude(bin) / load_loading_screen(bin, pal)
   stage parts into one slot (staged-merge, active-replace - the
   load_movie semantics per part); introspection loading_phase /
   loading_fade_step / loading_text_row; lib exports LoadingPhase +
   TextRow; movie.rs UNITS_* -> pub(crate) for the accumulator.
4. TESTS: 8 loading.rs units (decode incl. multi-entry entry-0-wins
   + rejections; fold + tail boundaries + short-PAL; fade lerp
   monotonicity; 50 Hz accumulator pacing + chunking invariance +
   saturation + staged-idle; text columns; arming domain; planes per
   phase) + 6 host.rs units (full lifecycle through the first zone
   transition incl. palette assertions at fade 0 and 10; scene-hash
   isolation with/without flow; FULL_MASK campaign walk to MAX_STAGE
   - endgame drop + zone-6 4th column + mid-zone Select inertness;
   skip-advance; interlude-without-movie; error paths). Workspace
   311 tests green, fmt + clippy -D warnings clean, MANIFEST.sha256
   verified before AND after the corpus-touching runs.

Not done here (queued): the FULLFONT.BIN glyph pass over the pinned
text row (FUN_0043c87c + the buf+0x2a2 font-ramp copy), BRF_DROP
play site, boot LOGO/GTLOG attract sequence.

## D35 - 2026-08-20: FULLFONT loading-text glyph pass (P5, item 1)

Context: NEXT item 1 - the D34 follow-up: draw the loading-screen
text through the pinned TextRow flow state from FULLFONT.BIN,
reproducing the four FUN_0043c87c draws + the font-ramp copy into
DAC buf+0x2a2 per the LAB_0041c69e tail. Engine WIP of interrupted
predecessors (Ghidra RE artifacts 12:23-12:34 + font.rs/language.rs/
font_gate.rs/pal ramp) adopted per AGENTS.md and completed by this
run (host-test rename tail, host font staging test, doc corrections).

1. RE CORRECTION (supersedes the D34 reading): the FUN_0043c87c args
   are (EAX=str, EDX=FULLFONT bank, EBX=draw ROW, ECX=glyph entry
   base 0x82) - the four values 0x96/0xb4/0xd2/0x104 are draw ROWS
   150/180/210/260 and x0 is computed INSIDE the drawer as
   0x140 - total/2 (each line centers on x 320). D34 recorded y=0x82
   with x-columns; the pair was swapped [verified: full decompile,
   exw-font-drawer.txt; FUN_00401ca2 dest = locked surface +
   ECX(pitch-rows) + EBX(col) settles row vs col]. TextRow flow
   state renamed to rows accordingly.
2. DRAWER (game font.rs): two passes (measure, draw); c >= 0x80
   remaps through FUN_00410493 (match table + 31-stub jumptable,
   objdumped) to (base char, accent id); k = c - 0x21; k < 0 -> pen
   += 9; else transparent RLE16 blit at entry 0x82 + k and pen += w
   + 2 (w = u16@+6 via FUN_00402a12, flags-0x0003 layout). Hotspot
   u16@+2 adds to dest ROW, u16@+4 to dest COLUMN (baseline anchoring
   dy 0/5/10/15). Accent id 1..=4 blits overlay entry 0x82+0x6b+id
   (238 diaeresis..241 circumflex) at the same pen. Two shipped
   quirks kept verbatim: e-/o-diaeresis stubs default to the dash
   base under the diaeresis; k > 0x78 -> dash + diaeresis
   [verified: stub bodies 0x4104c0..0x410650]. Bounds-clipped blits
   [deviation: Rust never writes out of bounds].
3. STRINGS (assets language.rs): the four draws read LANGUAGE
   [MENU_ITEMS] table entries 0x45 / 0x46 / zone+0x51 / 0x58 (the
   DAT_0046bc4c/0046bc7c/0046bfdc globals = table base 0046af5c +
   idx*0x30 - arithmetic identity). Parser reproduces the EXW scan
   (heading seek, skip bytes < 0x21, entry copy bytes >= 0x20, tab/
   CR/LF terminate, 96-entry cap) [verified: exw-font-strings.txt +
   exw-menu-parse.txt; deviation: bounds by buffer end, typed Ok,
   never panics - EXW trusts its 81000-byte alloc].
4. FONT RAMP (assets pal.rs parse_font_ramp): FULLPAL.PAL = 98 B =
   2-byte lead (e0 20 = first entry 224, count 32) + 32 6-bit RGB
   triples. EXW order in the tail: fill DAC buf+0x2a2..0x301 with
   0x3f + commit (TRANSIENT, pre-text), draw the four text rows,
   THEN copy the ramp over the same entries and arm FadeSetup -
   the fade TARGET carries the ramp; under the D34 from-black fade
   design the transient 0x3f fill is never displayed, so the flow
   applies the ramp to the target and drops the forced tail from
   loading_palette (entries 224..=255 keep folded file values when
   no ramp is staged) [verified copy; design where tagged].
5. HOST: load_loading_font(font_bin, fullpal, language) stages all
   three parts (staged-merge, inert until Loading entry; typed
   staging errors; no hashed-state contact). enter_loading now runs
   the four draws onto the loading raster in EXW order (still ->
   0x3f transient -> draws -> ramp -> fade) with missing-part skip
   [deviation: EXW always has all three].
6. CORPUS GATE (assets tests/font_gate.rs): FULLFONT.BIN 390 entries
   (333 RLE16|hotspot glyphs + 57 empty), 10 pinned glyphs (shape +
   hotspot + pixel sha256), ASCII-run pixel set exactly {0} U
   {233..=244}, baseline dy set {0,5,10,15}; FULLPAL lead bytes +
   full ramp sha; all six LANGUAGE files pinned (file sha + entries
   0x45/0x46/0x52..=0x58, incl. FRE/GER/SPA high-bit accent bytes
   that exercise the remap); the drawer arithmetic re-measured
   independently over the real bank widths (7 pinned totals +
   x0). Ignored regen test = the only documented regen path.
7. TESTS: 4 font.rs units (remap table incl. quirks; measure/pen
   rules incl. empty-slot gap; transparent/clipped/overlaid draw;
   bank rejections) + 3 language.rs + 2 pal.rs + 4 gate units +
   loading.rs enter_loading unit (rows drew + ramp applied + bare
   flow) + host: lifecycle updated, new loading_font_pass unit
   (raster rows + ramp tail + staging error), campaign walk stages
   the font and pins the zone-6 fourth-row raster draw. Workspace
   green, fmt + clippy -D warnings clean, MANIFEST.sha256 verified
   before AND after the corpus-touching runs.

Not done here (queued): BRF_DROP play site, boot LOGO/GTLOG attract
sequence, native executable shell, ZONEA/MISSION1 vertical slice.

## D36 - 2026-08-20: boot attract sequence GTLOG+LOGO (P5, item 1)

Context: NEXT item 1 - the D32 not-done tail: play the region-variant
publisher pair on the Boot scene. RE prerequisite landed by the
predecessor as 4e9ccbb (RE-EXW-GAMETHREAD.md "Boot attract arm RE").
Rust WIP of the same interrupted predecessor (died on transport error
after the docs commit; boot.rs + boot_attract_gate.rs + host/lib/
movie/movies diffs) adopted per AGENTS.md, independently re-validated
and completed by this run (clippy tail: u64::from removal in boot.rs,
needless borrows in the gate test).

1. RE [verified decompile, docs 4e9ccbb]: runner FUN_0044567c(name,
   arg2) - movies gate DAT_0046cca4; clears the 480x640 plane TWICE
   at entry of EVERY call (the plane between the two movies and
   before TITLE is black); frame loop `for (f=1; f<frames; f++)`
   renders frames-1 frames - ring movies play EXACTLY ONE bounded
   pass, final frame never decoded/rendered/played; dst height =
   480-2*arg2 (boot pair arg2=0 = full-screen 640x480 1:1; TITLE
   replay arg2=0x50 = 320 rows at y=80 - VERIFIES the D31 exact-
   centering design note with EXW arithmetic); per-frame palette
   apply of all 256 entries from the Smack struct (+0x6c) when the
   frame changed it (+0x68) - the D31 per-frame palette_dirty shape;
   skip gate 004edbc4 zeroed at GameMain entry, armed only inside
   NameEntryScreen around the TITLE replay -> the BOOT pair is
   UNSKIPPABLE in EXW (full one pass, no input abort).
2. FLOW MODULE (game boot.rs, presentation-only, D17 bucket b):
   BootAttract Staged -> Playing -> Done. Both MoviePlayers open +
   decode frame 0 at construction (D31 contract); per-movie one-pass
   target = frames-1; sequencing time-exact on the shared x240-us
   grid - a movie switches when (frames-1)*period elapsed (final
   frame shown its full period) or a non-ring stream latched
   finished; entry-audio rule per movie (start() hands the GTLOG
   frame-0 packet; the switch appends the LOGO frame-0 packet);
   Done holds the last raster until the scene drops the flow.
3. MoviePlayer::advance_limited(dt, max_frames): advance with a HARD
   decode cap (the EXW loop bound made starvation-proof - a burst
   spanning many periods still decodes at most the cap; leftover
   accumulator kept warm, retired with the movie). Ordinary scene
   movies (whole-file TITLE/cutscenes) keep uncapped advance().
4. movies.rs: boot_pair(region) -> [GTLOG, LOGO] in EXW play order
   (GameMain 0041c37a/0041c397, region DAT_0046ae64 both times,
   bracketed by Smacker init/shutdown); provenance on title_name/
   logo_name/gtlog_name upgraded from [corpus] to [verified play
   site].
5. HOST: load_boot_attract stages inert; sync_boot starts on the
   Boot scene (frame-0 audio queued one pump before any decode),
   drops the flow + clears the stream on leaving Boot; pump_boot
   rides the D31 stream bus and self-terminates on decode error or
   overflow (presentation self-terminates, never a pump error);
   render_now plane precedence loading > boot > movie (mutually
   exclusive by scene). No input path exists - EXW parity. Hash
   isolation unit-pinned (chain identical with/without the attract).
6. CORPUS GATE (game tests/boot_attract_gate.rs, skips when corpus
   absent): both region pairs driven to Done at the 60 Hz host pace;
   max decoded frame = frames-2 (GTLOG 68 / LOGO 69 of 70/71, D32
   counts) - one pass, ring NEVER wraps; switch + Done pump counts
   pinned by the closed formula ((frames-1) periods of 66_660 us);
   continuous in-order DPCM audio (>100 kB per pair) with the LOGO
   frame-0 packet exactly at the switch; two independent runs
   byte-identical (SHA-256 over the per-pump observation chain).
   No decoded media enters git - only hashes asserted.
7. TESTS: 5 boot.rs units (staged silence; one-pass bound + exact
   switch timing incl. Done-hold; starvation-burst cap; 2-frame
   movies render exactly one frame; bad bytes reject at construction)
   + 1 movies.rs unit (pair order per region) + 2 host units (Boot
   lifecycle incl. stream clear on exit; scene-hash isolation) + the
   corpus gate. Workspace 335 tests green / 0 failed, fmt + clippy
   -D warnings clean, MANIFEST.sha256 verified before AND after the
   corpus-touching runs.

Not done here (queued): BRF_DROP play site, native executable shell,
ZONEA/MISSION1 vertical slice.

## D37 - 2026-08-20: BRF_DROP briefing intro pair + ring-Last fix (P5, item 1)

Context: NEXT item 1 - the D33 not-done tail: BRF_DROP.SMK, the
only BRF_* file with no briefing_name mapping. The RE prerequisite
landed by the interrupted predecessor as 3a2981d
(RE-EXW-GAMETHREAD.md D37 section), which died on a transport
error after the docs commit, leaving the Rust unwired; this run
adopted and completed it (commit bba01fe).

1. RE [verified, docs 3a2981d, summarized]: FUN_0043d00b IS the
   briefing screen (prior gameplay-advance gloss corrected); the
   BRF_DROP play site is asm 0043d447..0043d490 - the literal at
   0x4591f7 opens FIRST at every movie-enabled briefing (gate
   DAT_0046cca4), full screen (dst height 0x1e0 = 480 rows), ONE
   pass (handoff = frame index reaching count-1 -> count-1
   renders), then hands off to the constructed BRF_{zone}{level}.SMK
   ring (name builder 0043d1b7..0043d335, letter =
   zone@004edd8c + 0x40, zones 2..=6 = B..=F - the D33 open note
   resolved) which rings until the UI exit; both open failures are
   FATAL; the GO button arms only after the handoff (unskippable).
2. FLOW: brief.rs BriefIntro (Staged -> Drop -> Backdrop) on the
   D31 clock. The drop pass is hard-capped by advance_limited
   (frames-1 decoded frames - the same count as the modal runner);
   the backdrop ring advances UNBOUNDED (the flow never reports an
   end - the scene exit drops it); the entry-audio rule applies at
   start and at the handoff (the corpus pair is silent).
3. HOST: load_briefing(drop, backdrop) stages inert with the
   sync_movie semantics (a staged flow waits on any scene; an
   active one drops + clears the stream on leaving Brief);
   pump_brief rides the D31 stream bus and self-terminates on
   decode error / overflow; render precedence
   loading > boot > brief > movie. Hash isolation unit-pinned.
4. MOVIE FIX (found by the new corpus gate): the seam contract
   says Last = the last frame of a stream OR OF A RING PASS - the
   vendored ring decoder returns Last at the closing slot of every
   cycle and wraps to frame 1 on the next call (frame 0 is the
   setup frame, never replayed; ring total = frames + 1).
   advance_limited latched finished on ring-Last, freezing ANY
   ring at its first cycle end - latent since D31 (no consumer had
   run a ring past one cycle: the boot attract caps at frames-1).
   Ring-Last now continues; non-ring Last/Done unchanged. New
   movie.rs ring unit pins the wrap + audio order; SHOP.SMK (D33,
   ring) inherits the fix.
5. CORPUS GATE (game tests/brief_gate.rs, skips when the corpus is
   absent): BRF_DROP + BRF_B1 at the 60 Hz host pace - drop max
   decoded frame 28 (29/30 rendered), handoff at the closed-form
   pump 58 ((frames-1) * 7_999_200 units on the x240-us grid),
   zero PCM bytes (the corpus pair is silent), the 512-frame ring
   reaches its closing slot 512 and wraps exactly 512 -> 1 while
   still playing (2+ full cycles driven), two independent runs
   byte-identical (SHA-256 over the per-pump observation chain).
6. TESTS: 5 brief.rs units (staged silence; one-pass bound + exact
   handoff timing incl. the non-ring backdrop hold; starvation-
   burst cap; 2-frame drop passes in one period; bad bytes reject
   at construction) + 1 movie.rs ring unit + 2 host units
   (lifecycle incl. entry audio + stream clear on exit; scene-hash
   isolation) + the corpus gate. Workspace 343 tests green / 0
   failed (recounted at D37 close-out on the bba01fe tree; the
   bba01fe message said 342 - one short), fmt + clippy -D warnings
   clean, MANIFEST.sha256 verified before AND after the
   corpus-touching runs.
Not done here (queued): native executable shell (P4 window/audio
integration), ZONEA/MISSION1 vertical slice.

## D38 - 2026-08-20: provisional shell button layout (P4 shell input seam)

Context: the P4 input adapter (bedlam-shell input.rs) needs SOME bit
assignment to translate winit events into
`bedlam_core::input::InputFrame.buttons`, but the engine-side bit
contract is still unpinned - the EXW scan-code keystore map
(docs/RE-EXW-INPUT.md, 12 edge latches, arrows +0x80 remap) binds a
different mechanism and its engine-side binding lands with the P2e
input RE.

Decision: the SHELL owns a provisional layout (module `button`:
UP/DOWN/LEFT/RIGHT/FIRE bits 0-4, WEAPON1-4 bits 5-8, ESCAPE bit 9)
documented as ours, not the EXW keystore. The seam is pinned so P2e
only has to shrink this module to a pure winit->engine-event
translator and move the layout into bedlam-core:
1. `map_physical_key(PhysicalKey) -> Option<ShellKey>` is the pure,
   unit-pinned mapping table (winit's `KeyEvent` carries a
   `pub(crate) platform_specific` field and CANNOT be constructed
   outside winit - the thin `map_winit_key(&KeyEvent)` wrapper is
   covered by the corpus smoke instead; this replaced a predecessor
   test that could never compile).
2. `ShellInput` accumulates between pumps and `tick()` snapshots:
   held buttons PERSIST across ticks (the FSM derives edges itself -
   D26 hashed per-tick latches), pointer deltas are consumed per
   tick, i16-saturating; focus loss clears held state so an alt-tab
   cannot stick a key.
3. Mapping shape per PLAN sec 6 P6 modern defaults: WASD+arrows
   move, mouse aims, left button fires, 1-4 weapon hotkeys, Escape
   backs. Wheel provisionally maps to Up/Down (menu stepping); the
   1996 build had no wheel. Original-scheme rebinding stays P6.

## D39 - 2026-08-20: winit 0.30.13 is the window host (P4 shell step 1)

Context: PLAN sec 4 dependency spike for the native shell - winit
was the default candidate; this records the choice + pinned version.

1. CHOICE: winit 0.30.13 (Cargo.lock) + pollster 0.4 for blocking
   GPU init; wgpu itself stays consumed through the bedlam-platform
   re-export so the workspace pins exactly one wgpu (27.0.1).
2. winit 0.30 SHAPE (the part that cost a predecessor session): the
   window is created INSIDE `resumed()` via
   `ActiveEventLoop::create_window` (the pre-run `EventLoop` form
   is deprecated) and held behind `Arc<Window>`, because wgpu's
   surface needs an owned window handle to give `Surface<'static>`
   a lifetime that outlives `run_app`. `about_to_wait` is
   borrow-scoped: clock read -> pump -> stage -> redraw request.
3. GPU: `ParityGpu::new_for_surface` added to bedlam-platform (the
   headless `new()` and the surface path share the low-power /
   default-limits / no-features device contract; pollster blocking
   is window-host-only, never on the sim path).
4. LOOP: Determinism Charter honored - the measured frame delta
   feeds `FixedStepClock::advance` (pure u128 integer banking,
   anti-spiral clamp DEFAULT_MAX_CATCHUP=4 pumps, surplus DROPPED
   not fast-forwarded) which decides only HOW MANY identical 60 Hz
   pumps run (fixed dt = 4 sub-ticks on the 240 Hz grid); present
   is the D20 PARITY path (640x480 indexed upload, palette expand,
   integer-scale, Fifo vsync).
5. HEADLESS DISCIPLINE: the window path is runtime-gated behind
   `--window` / `BEDLAM_SHELL=1`; the default binary path is the
   headless smoke (fixed 600 pumps, scripted walk, neutral input)
   whose report is byte-identical across runs. The smoke walks the
   full wired chain and fetches exactly GTLOG/LOGO/TITLE/ZONEDONE/
   BETWEEN/LOAD_UK/LOADPAL/FULLFONT/FULLPAL/LANGUAGE.ENG.
6. CORPUS SOURCE: `GameGfxSource` is the ONLY fs reader in the
   engine crates; two-tier lookup GAMEGFX/<name> then <root>/<name>
   (LANGUAGE.* sits at the install root - EXW read them from its
   CWD; bare names only, separators/parent hops rejected).
7. LESSON (from the four step-capped worker sessions this WIP
   survived): the original `gpu.rs` blocker was a stray apostrophe
   - `Surface<'_'>` instead of `Surface<'_>` - which rustc lexed as
   a const-generic argument and reported as a misleading E0107
   "struct takes 0 generic arguments". When a trait/generic error
   looks impossible against the vendored source, check for stray
   quote characters before suspecting the toolchain.
8. TESTS: 13 bedlam-shell units (5 clock banking/clamp cases, 7
   input seam cases incl. the disjoint-bit pin, scene_assets chain
   pin) + the binary smoke (byte-identical x2). Workspace 356
   green / 0 failed (343 + 13).

## D40 - 2026-08-20: cpal 0.18.2 is the audio output device (P4 shell step 2)

Context: NEXT item 1 - make the D31 audio stream bus + the entry-
audio sites audible. PLAN sec 4 names cpal the default candidate;
this records the choice + pinned version + the device-feed contract.

1. CHOICE: cpal 0.18.2 (Cargo.lock; the crate's first use in the
   workspace). Dependency lives in bedlam-shell ONLY - the mixer in
   bedlam-audio stays hermetic (integer math, no I/O, no floats -
   DESIGN-AUDIO), and the mixed byte stream stays un-hashed (D17
   bucket b: audio is NOT hashed; re-proven empirically below).
2. THREAD SHAPE: GameHost/mixer are main-thread-only (not Send, by
   design), so the cpal callback (its own realtime thread) never
   touches them. The ONE crossing point is a bounded ring of ready
   interleaved-stereo i16 frames behind a plain mutex (poison-
   tolerant: a panicking producer must not turn the callback into
   an error storm). The window loop is the ONLY producer (watermark
   fill toward 736 frames ~67 ms after every pump batch); the
   callback is the only consumer.
3. DEVICE CONFIG: prefer the supported range that CONTAINS the
   mixer-native 11025 Hz (stereo, then mono, then any channel
   count) pinned exactly at 11025 - resampling is NOT owed at the
   native rate (task wording); this machine's default device
   (PulseAudio over ALSA) accepted 11025 Hz 2ch directly. No such
   range falls back to the device default through a Q16
   nearest-neighbor frame stepper (output n reads input
   floor(n*step/65536); pinned: 44100 = each frame exactly 4x,
   48000 step 15053 with pinned sample-hold positions, 8000 step
   90317 with pinned skip counts). Underrun = EXACT [0,0] frames
   (mixer bus semantics); a full ring drops the OLDEST frames
   (lateness skipped, never accumulated). Mono devices take the
   floor average (l+r)>>1; >2 channels repeat L/R (even=L, odd=R);
   sample formats convert through cpal's dasp Sample conversions.
4. RUNTIME GATE (D39 discipline): the stream is built ONLY inside
   the window host (run_window, after boot staging); headless and
   tests never open a device. No device / no workable config = a
   stderr note and a silent run - audio is best-effort, the game
   itself never depends on it. An #[ignore]d opt-in probe test
   (cargo test -- --ignored) opens the real device and drains
   silence; measured device startup latency before the first
   callback pull is ~100-200 ms, invisible to the 16 ms steady-
   state refills.
5. HEADLESS SMOKE DRAIN: the walk now mixes 184 frames (ceil of
   11025/60) per pump off the host bus into a discard sink,
   counting frames + non-silent samples in the report (110400
   frames = exactly 600x184; 158092 non-silent samples = the
   TITLE.SMK track and the other entry-audio sites actually
   producing PCM on the walk).
6. GATES: two smoke runs byte-identical (full stdout diff); the
   scene hash (696adb1cd110e062) and frame parity hash
   (cce30c983b97b16d) are IDENTICAL to the pre-change binary -
   the audio drain provably hashes nothing; MANIFEST.sha256
   verified before AND after the corpus runs; workspace 366
   tests / 0 failed (356 + 10 new audio units); fmt + clippy -D
   warnings clean.
Not done here (queued): the menu/ZONEA/MISSION1 playable vertical
slice (P4 exit) - the shell now has window, input, present AND
audio; the slice needs the P2d/P2g tails.

## D41 - 2026-08-20: title-menu screen = NameEntryScreen 0043a5fc (P2g, item 1)

Context: NEXT item 1 - the first UI-archaeology slice, prerequisite
for the P4 vertical-slice menu step. RE notes landed as
docs/RE-EXW-TITLEMENU.md (commits 3eb3092 + cf75108, raw dump
ghidra-project/exw-titlemenu.txt via ExwTitleMenu.java -process pass;
jump tables decoded from the raw EXW image because they are data blobs
inside the text stream).

1. NameEntryScreen@0043a5fc IS the title/options menu (8.7 KB, one
   function, GameMain its only caller at the outer restart point).
   Menu state: builder FUN_00445b5c(id 1..5) -> count word 004eabd2 +
   7 string slots 004eabd4.. stride 0x30 + id 0046ae7c; drawer
   FUN_0044653a bottom-anchored (row 0x1d6 - count*0x18, 24 px rows).
2. Hit model is a STRIP, not rects: x in (0xdc,0x1a4), y in
   (top,0x1d6), item = (y-top)/0x18 [verified asm 0043a934..0043a996].
   Click = g_scroll_flags != 0; hover SFX MENU1 / click SFX MENU2;
   attract counter >= 0x300 -> skippable TITLE.SMK replay (the only
   skippable movie, gate 004edbc4 armed around FUN_004459f7).
3. All item actions anchored (0x43aad5..0x43b097): single-player start
   (score seed 4000 - difficulty*500), difficulty cycle 0..2,
   name entry (ENTER = keystore[0x1c] exits; config persisted by
   FUN_0042540c = the CONFIG.BDL writer), HOF, CREDIT_1..13 pages,
   quit-confirm; multiplayer menu (player count 2..12 cycled by
   left/right CLICK on the count item); save-load menu (slot stride
   0xb4, completion bitmask restored via FUN_004474ef).
4. NEGATIVE finding: MENU_ITEMS entries 47..58 ("Options" + the
   Double Buffer..No CD Audio toggles) have ZERO xrefs in EXW - there
   is no separate options screen; the options ARE main-menu items.
5. Corpus pin: glyph base 0x82 vs 0 = same shapes, green vs blue
   FULLPAL ramp slices (233..244 vs 244..255) - menu selection
   renders green, unselected blue.
Unblocked: the P4 menu step (PLAN sec 6 P4) can now be implemented
against pinned addresses; the P2d sim tail remains its other input.

## D42 - 2026-08-20: title menu engine step (P4-menu, item 1)

Context: NEXT item 1 - implement the D41 findings as the P4 vertical
slice's menu step. All EXW-verified facts carry their anchors in
docs/RE-EXW-TITLEMENU.md; this entry logs the ENGINE-SIDE choices the
RE does not decide (T2/T3 territory per the parity budget).

1. OWNERSHIP: while a title menu is staged on Scene::Title, the menu
   IS the Title input path (EXW NameEntryScreen owns its loop): the
   host feeds the FSM NEUTRAL frames (scene ticks still count, no
   generic click-advance), and menu outcomes map to explicit intents -
   Start -> SceneAction::Advance, quit-accepted -> SceneAction::Quit.
   A host without a staged menu keeps the D26 generic click-advance
   (back-compat: the headless walk and determinism scripts).
2. CLICK MODEL: EXW dispatches on g_scroll_flags != 0 (the button
   LEVEL with a 4-tick SFX debounce, RE sec 3). The engine models a
   click as any-button PRESS EDGE per executed tick - the same
   edge-at-the-tick-grid discipline as the FSM (D26); holding a
   button fires once. [deviation, T2]
3. ATTRACT REPLAY: the menu's idle counter (reset on hover/click,
   EXW 0043a8b0) fires at >= 0x300; the host replays by RESTARTING
   the staged Title-scene movie slot in place (MoviePlayer::restart:
   rewind + frame-0 decode + frame-0 audio requeue - the EXW replay
   re-opens TITLE.SMK through FUN_004459f7). No staged title movie ->
   the attract is a silent no-op and the counter restarts. The
   replay is SKIPPABLE (EXW skip gate 004edbc4): any button/key press
   edge during the replay finishes the player in place; the first
   entry play stays unskippable (D31). The idle counter does not
   advance while a Title movie plays (EXW's menu loop starts after
   the movie).
4. BACKDROP: the menu plane is a 640x480 BLACK canvas + the text
   strip (the EXW attract arm's clear-screen semantics). The EXW
   backdrop buffer content (0x64000 alloc at 0043a5fc, filled by the
   draw cycle's PresentCopy re-blit) is NOT yet pinned - queued as an
   open question in RE-EXW-TITLEMENU sec 8 rather than guessed.
   Text geometry IS pinned: row 0x1d6 - count*0x18 bottom anchor,
   24-px rows, glyph base 0x82 selected (green set) vs 0 (blue set).
5. STUBS (this slice): HOF / credits item actions are inert; the
   save-load menu builds 5 "EMPTY" slots + Cancel (EXW literal
   0x45980f) with slot clicks inert (no save corpus yet); coop /
   head2head exits are inert (multiplayer lobby = future slice);
   CONFIG.BDL persistence (FUN_0042540c) is deferred to the config
   writer slice; the OPTIONS music (load_midi at 0043a739) rides
   the future D27/MRS Title wiring; the name seeds EMPTY
   (fresh-BSS EXW state) and applies the "GOD" default (literal
   0x459078) on empty name-entry exit [inferred: FUN_0044efb3(name,
   0x459078) default argument].
6. NAME ENTRY: entering the mode is reachable and visible (cursor
   blink = bank entry 0x8e - the 0x82-set slot for char 0x2d - at
   x = 0x146 + (width("Name: ")+width(name))/2, row of item 3, shown
   while (tick & 0xc) != 0); TYPING rides an explicit host API
   (menu_type_char / menu_backspace) because InputFrame.buttons is
   still the provisional D38 bitmask (no text path until P2e lands);
   in the hashed-input path any click edge exits the mode [deviation:
   EXW exits on ENTER, keystore[0x1c]].
7. SFX: MENU1.RAW/MENU2.RAW stage as mixer instruments 0xE0/0xE1
   (outside the music-script instrument domain) played via note_on at
   unity ratio / volume 48 (unity at master 127), both debounced 4
   ticks. Fetch tier: bare name -> GAMEGFX -> root -> SOUND/SFX (the
   EXW "SOUND\SFX\MENU1.RAW" path, GameGfxSource third tier).
8. HASH ISOLATION (D17): the menu is presentation-only. Staging +
   the whole interactive loop leave the scene-hash chain identical to
   a NEUTRAL-input run (the FSM sees neutral frames either way);
   unit + corpus pinned. The score seed (4000 - difficulty*500) is
   exposed for the P2d sim-tail wiring, not consumed yet.

LANDED (commits 57413b0 docs + 0a10a54 menu/host + 7ff713e chain/
gate): 393 workspace tests / 0 failed; fmt + clippy -D warnings
clean; headless smoke two runs byte-identical with scene/frame/audio
parity IDENTICAL to the D40-complete baseline 143e60d; corpus gate
tests/menu_gate.rs pins the table, geometry, the green/blue ramp
slices end-to-end, start handoff, SFX audibility and the TITLE.SMK
restart; MANIFEST verified before and after.

## D43 - 2026-08-20: provider-side transport failures never charge the task

The 2026-08-20 provider incident (`Invalid
zai-coding-plan/openai-compatible-chat stream event`, also hitting the
watchdog check model as `Invalid opencode/openai-compatible-chat stream
event`) killed nine nudge spawns within seconds of their first model
call plus two mid-run sessions (20:42-22:59). nudge-agent.sh classified
the instant deaths `client-error` (the transport grep did not know the
signature) and charged every one of them to the task fail counter:
task 4f6a0d2b reached 11 fails and kept refreshing 15-minute cooldowns
for ~2.5h AFTER the provider recovered - the loop stood down while
fully healthy. One charged run (f2d86578) had even committed attributed
work (98fc0b0) before its session died.

CHOICE: extend the transport signature grep with
`Invalid <provider>/openai-compatible-chat stream event`, and give
`kind=transport` the same accounting exemption step-cap already had:
log loudly, retain the claim for the DEAD_CLAIM_TTL retry backoff
(which throttles spawn churn during a live incident, plus MAXSPAWN=16/h
above it), but never touch taskfails/taskcooldown. Task failure state
is reserved for failures attributable to the work itself
(no-progress, genuine client errors). Provider-incident escalation is
the llm-watchdog job, not the task counter. Rate-limit keeps charging
(account-side, worker-behavior-coupled). Contaminated live state
(taskfails/taskcooldown for 4f6a0d2b) was cleared by the same repair;
verified post-fix: fresh glm-5.3 probe PROBE-OK rc=0, both nudge test
harnesses PASS.

Watchdog-Repair: llm-watchdog 86278 1787260264

## D44 - 2026-08-21: idle-log reaper for hung opencode2 clients

Provider stream death has a second, quieter failure mode beyond the
2026-08-20 instant-exit signatures (D43): the opencode2 client can
print `Error: Transport` and then NEVER exit. Observed 2026-08-21
01:32: worker 82523e41 on queue item 1 (task 230a7a38b991ed5f) wrote
its first ~110KB of log in 75s, hit the provider transport error, and
then sat in do_epoll_wait at zero CPU with a frozen agent log for 30+
minutes. The process was alive, so: the controller saw a live locked
claim and logged "concurrency full - standing down" every minute; the
documented provider-side 300s zero-stream watchdog never fired (the
stream was already dead); only the outer `timeout 3900` could clear
it - one provider hiccup burning the entire 65-minute single-slot
budget. Two earlier llm-watchdog checks (01:28, 01:38, 01:52) read
this same stall as healthy; the 02:02 check proved the hang (fd
positions all at EOF, 0 CPU ticks over 5s, no file modified anywhere
since 01:35) and requested repair.

CHOICE: nudge-agent.sh now supervises the client itself. The
`timeout 3900 opencode2 run` invocation moved into the background and
a poll loop watches the agent-log mtime while the process lives: once
the log has been silent >= NUDGE_IDLE_LIMIT (default 900s - far above
any legitimate silent stretch like a cold cargo build, and
env-overridable for tests) the wrapper TERMs then KILLs the client
and marks the run `reaped`. Reaped runs classify as `kind=transport`
via an explicit `reaped` branch (they may carry no log signature at
all), so a hang gets exactly the D43 accounting: provider-side, not
charged to taskfails/taskcooldown, claim retained for the
DEAD_CLAIM_TTL retry backoff, transport log line annotated with
"(idle-log reaper)". A provider hiccup now costs <= ~15 min of the
slot instead of 65, and the llm-watchdog keeps owning incident
escalation. The interrupted predecessor WIP (mission scene step, 4x
E0308 in bedlam-core) was preserved untouched for the next worker to
adopt, per the stand-down rules. Verified: test-nudge-claims.sh with
two new hang-signature mocks (with and without the `Error: Transport`
line), plus controller/queue/network harnesses, all PASS. Known
pre-existing, out of scope here: test-llm-watchdog.sh has a rare
pause-release timing flake under load (llm-watchdog.sh itself is
unchanged by this repair).

Watchdog-Repair: llm-watchdog 776518 1787270729

## D45 - 2026-08-21: MissionScene composition choices (P4 scene step)

CONTEXT: DESIGN-GAME sec 11 (commit a6317c5) composes the corpus-
verified halves - bedlam-core MissionSim + bedlam-render MissionView
- into the Mission scene state. Every EXW behavior is anchored
(RE-EXW-SIM secs 1/6/7c, RE-EXW-MISSIONVIEW secs 5d/7); the open
[freedom]s were reimplementation choices needing one home:

CHOICES (all tagged [design] in the code):
1. CAMERA: fixed at the first spawned robot's Q5 position on scene
   activation (the EXW cam pair _DAT_004edde4/8 points at the spawn;
   scroll input stays out of the slice - the next unit owns it).
2. CLICK HIT BOX: |dx| <= 0x20 and |dy| <= 0x20 around the enqueue
   projection, nearest octagonal screen distance wins, ties to the
   lowest index (the EXW walks the actual sprite outlines
   ~0x433cbc; half the 64-px sprite cell is the stand-in until an
   outline test is RE-pinned).
3. PRESENT PATH: the 480x480 window blits at canonical (0,0) of a
   640x480 plane (viewport + black sidebar, the sec-6.2 screen
   split) handed through the MovieFrame seam - a 640x480 movie
   centers at (0,0), so no new compose path exists; GAMEPAL and
   sidebar art are the following unit.
4. FETCH ORDER: the 9-file mission fetch set = the load_mission
   path families (per-mission TOT/DAT/PAD, zone CGR/BIN/LNK, then
   the GAMEGFX tail SINTABLE/DANTE, then MRK) - a chain convention
   for deterministic fetch logs, not a pinned EXW load order (the
   binary's verified order is TOT, DAT, CGR, BIN, MIN, LNK + the
   separate PAD/MRK staging).
5. ZONE/MISSION SELECTION: stage -> zone letter (clamp 0..6),
   mission = lowest-unset mask bit + 1 - the same arithmetic
   briefing_name_for_slot uses; the HOST owns it
   (mission_slot/mission_asset_names) so the chain and the staging
   zone cannot disagree.
6. LIFECYCLE: the movie pattern - staged inert (no tick, no plane,
   parity-identical), activate on Mission entry (the entry pump
   renders but does not tick), drop on LEAVING after entry, a
   staged-but-never-entered mission stays staged.

EVIDENCE: 422 workspace tests green; corpus gate
tests/mission_scene_gate.rs pins the scene-composed ZONEA/MISSION1
frames (spawn 51ef4fe93eaaed77, mid-walk 7bae11a5c7f34ab6 + the two
sim hashes) with two-run identity while the render-gate pins stay
untouched; the parity harness output is BYTE-IDENTICAL to the D28
anchors (scene 0xcae25cd08d7cbc08, sim 0x72979d5d9dedc832, frame
0x87263f149564ad25, audio 0xc862e45d2e95ad29) - the mission is
provably inert on paths that never stage it.

Nudge-Worker: 74fa370e-5260-47d4-8c03-9986e7c86ef3

## D46 - 2026-08-21: GAMEPAL mission present tail (P4)

CONTEXT: D45 choice 3 left the mission plane presenting under the
host palette stand-in (all black on the corpus gate). This unit
stages GAMEGFX\GAMEPAL.PAL (770 B, the parse_vga770 LOADPAL format
family) with the mission and makes it the mission plane palette.

CHOICES (tagged [design] in the code):
1. FETCH POSITION: GAMEPAL.PAL joins the Mission fetch set inside
   the GAMEGFX tail - SINTABLE, DANTE, GAMEPAL - before MRK, making
   the set 10 files. The chain convention stays load_mission-path
   files, then GAMEGFX family, then markers (D45 choice 4); the EXW
   anchor is the FUN_0041df10 staging family (GAMEPAL among the
   mission sprite banks + palettes, RE-EXW-MISSIONVIEW sec 6).
2. PALETTE FOLD: GAMEPAL folds with the exact loading_palette rule
   (parse_vga770 then >>2 - lossless for 6-bit file values), so the
   mission palette is the same canonical [Vga6; 256] the loading
   screens and movies carry; no new palette form exists.
3. OWNERSHIP: MissionScene owns the folded palette; plane() drops
   the host-palette parameter and returns its own. The plane still
   rides the MovieFrame seam, so the mission frame's palette IS
   GAMEPAL with palette_dirty every frame (the movie convention) -
   the indexed->RGBA window upload stays platform-side untouched.
4. PIN REGENERATION: Frame::parity_hash covers the palette, so the
   two scene-gate FRAME pins moved once (spawn 51ef4fe93eaaed77 ->
   a79fcada30ec5e50, mid-walk 7bae11a5c7f34ab6 -> 1b75b68ce66019e1);
   both sim pins and every observation pin are unchanged, and the
   render-gate pins (mission_view_gate) never touch a palette.

EVIDENCE: corpus GAMEPAL has 254/256 non-black entries (entry 1 =
6-bit (0x3E,0x3A,0x39)) pinned structurally; the gate also pins
frame.palette == folded GAMEPAL + palette_dirty. Headless smoke 25
fetches (GAMEPAL.PAL 770 B), two runs byte-identical, exit 0.
Parity harness BYTE-IDENTICAL to the standing anchors (chain
0xcae25cd08d7cbc08, sim 0x72979d5d9dedc832, frame
0x87263f149564ad25, audio 0xc862e45d2e95ad29) - GAMEPAL never loads
on unstaged paths. All workspace tests green, fmt + clippy -D
warnings clean, MANIFEST verified after the corpus reads.

Nudge-Worker: 1776dc60-7f7e-4546-b875-fd9210b9836d

## D47 - 2026-08-21: modern device output rates at the audio edge (P4)

Context: NEXT item 1 - D40 opened the cpal output stream at the
mixer-native 11025 Hz whenever the device allowed it (period-correct
but the host stack's own resampling then owns the quality, out of
our control). This moves the rate/format choice to a deliberate
preference order at the DEVICE BOUNDARY only.

1. RATE POLICY: prefer 48000 Hz, then 44100 Hz, then the mixer-native
   11025 Hz (resampling still not owed at native), then the device
   default config. The preference is evaluated over the device's
   supported-config RANGES (a range containing the rate is pinned via
   try_with_sample_rate, so a wide 44100-96000 range opens at 48000,
   not its minimum). RATE DOMINATES the within-rate ranking: 48000
   mono beats 44100 stereo.
2. WITHIN A RATE: channels first (stereo, mono, other - D40 order
   kept), then sample format S16 before F32 before anything else
   (S16 is a pure widening of the ring's i16; F32 is the float
   default of modern hosts; the winning range's REAL format is still
   what gets built - the preference only ranks ranges). Exact ties
   keep the first-listed range (stable device enumeration).
3. NEGOTIATION SEAM: cpal 0.18's SupportedStreamConfigRange is not
   constructible outside the crate, so the choice is a PURE function
   choose_output_config(&[OutputConfigSpec]) -> Option<(index, rate)>
   over a neutral spec struct; open_default maps the real ranges into
   specs and maps the winner back. The fallback matrix (48000/44100/
   native/wide-range/none/empty) is unit-pinned without a device.
4. RESAMPLER: the D40 Q16 nearest-neighbor frame stepper is extended
   with LINEAR INTERPOLATION - output n reads input position
   n*step/65536 and blends the bracketing frames, round to nearest,
   ties toward +inf, i64 internally (|delta| up to 65535 times frac
   up to 0xFFFF overflows i32). A LONE buffered frame edge-HOLDS (the
   blend never reaches ahead into underrun); an empty ring stays
   EXACT [0,0] silence; at the native rate the phase residue is
   always 0, so the passthrough stays EXACT and un-interpolated (the
   D40 native passthrough pin is unchanged). The mixer bus and the
   parity stream remain 11025 Hz stereo byte-faithful - nothing
   upstream of the ring can observe the device rate.
5. GATES: 428 workspace tests / 0 failed (28 shell lib incl. the
   new negotiation matrix, 44.1k/48k interpolated-ramp pins with
   hand-computed literals, downsample blend pin, sample-format
   mapping pins i16/f32/u8 silence + both full scales, u8 128/255
   end-to-end through the D31 bus into the ring); fmt + clippy -D
   warnings clean; headless smoke two runs byte-identical AND
   byte-identical to the pre-change binary (scene 696adb1cd110e062,
   frame parity cce30c983b97b16d, audio 110400/158092 unchanged);
   parity harness IDENTICAL on all four anchors (chain
   0xcae25cd08d7cbc08, sim 0x72979d5d9dedc832, frame
   0x87263f149564ad25, audio 0xc862e45d2e95ad29); MANIFEST verified
   before and after the corpus reads; live-device probe (opt-in
   --ignored) now opens 48000 Hz 2ch i16 on this machine's default
   device (was 11025 Hz) and drains it cleanly.

Nudge-Worker: 2cd16045-bf39-46b3-9175-f71326aca6a2

## D48 - 2026-08-21: ordered window-host teardown (the Escape SIGSEGV)

Context: NEXT item 1 - pressing Escape in `bedlam-shell --window`
exited via SIGSEGV (operator coredump 422346, 2026-08-21 00:56).
The queue named cpal teardown as a suspect; the coredump stack says
otherwise, and the fix follows the evidence.

1. ROOT CAUSE (decoded from coredumps 422346 + the live repro
   1150695, identical stacks): the crash is on the MAIN thread
   inside wgpu/EGL teardown, not audio - ShellApp drop -> WindowHost
   drop -> CoreBindGroup::drop_slow (the LAST wgpu object, from the
   parity pipeline) -> wgpu_core Global::drop_slow ->
   wgpu_hal::gles::egl::Instance drop -> libEGL_mesa ->
   wl_proxy_marshal_flags, SEGV_MAPERR. wgpu tears EGL down LAZILY,
   at the drop of the last object holding an Arc<Global>; that
   teardown (eglTerminate through Mesa) marshals Wayland requests
   THROUGH THE WINDOW'S PROXIES. The old WindowHost declared
   `window: Arc<Window>` FIRST, so field-order drop released it
   before `pipeline` - the proxies were dead by the time the lazy
   Global drop walked them.
2. TEARDOWN CONTRACT: after the winit loop ends (Escape,
   CloseRequested, staging-fatal, or the auto-exit hook - all the
   same ActiveEventLoop::exit path), teardown is ORDERED in
   run_window: (a) the AudioDevice drops FIRST, (b) then every
   wgpu/EGL object (the whole WindowHost) while the winit window is
   still alive, (c) the window Arc is released only afterwards.
   Field orders back the contract structurally: WindowHost declares
   `window` LAST (drop-last), ShellApp declares `audio` BEFORE
   `gfx`; the explicit take/drop block in run_window makes the
   order load-bearing at the exit site even if fields are later
   reordered.
3. DEAD-FEED GUARD (the cpal half of the queue item, belt-and-
   braces): AudioFeed carries an Arc<AtomicBool> alive flag shared
   with the device callback. AudioDevice::drop quiets the feed,
   pauses, then drops the stream; a LATE callback invocation (some
   hosts fire during teardown) checks the flag BEFORE locking and
   writes EXACT silence (u8 midpoint 128) without touching the
   ring; fill_from renders nothing once quiet. The guard is
   sticky - memory safety was never in question (the callback holds
   its own Arc to the ring state), this pins determinism: a dead
   stream can neither play stale samples nor race the teardown.
   The callback body is factored into silence()/drain() so both
   halves are unit-testable without a device.
4. REPRO/REGRESSION GATE: WindowOptions::auto_exit_after (shell
   binary: env BEDLAM_WINDOW_EXIT_MS=<millis>) fires the same exit
   path as Escape after a deadline, so the window teardown is
   exercisable without a human. Live A/B on this machine (Wayland +
   Mesa EGL + PipeWire, device 48000 Hz 2ch i16): pre-fix binary
   exit 139 + coredump 1150695 (stack == 422346); fixed binary exit
   0 twice, no new coredump. Escape, window-close, and the hook
   share the one post-loop teardown block, so the close path is
   covered by the same verification.
5. GATES: 431 workspace tests / 0 failed (31 shell lib: +3 dead-
   feed guard tests - quiet feed renders nothing + sticky, dead
   callback exact u8/128 silence with ring untouched, live drain
   identity map); fmt + clippy -D warnings clean; headless smoke
   two runs byte-identical (frame parity cce30c983b97b16d, audio
   110400/158092 - the D47 baseline); MANIFEST verified after the
   corpus reads.

Nudge-Worker: 34bd8958-77b4-40c7-a8d0-b1ecf3126b30

## D50 - 2026-08-21: sidebar art pass — wire the bank-faithful chrome, never invent the data

Context: RE-EXW-SIM 6c.8 decoded the whole sidebar art family
(FUN_00408403 rows, FUN_004072bf portraits, FUN_0040807f bars,
FUN_004085ce score strip; banks GENERAL/SMLFONT/NUMBERS/SCANNER),
but the engine sim models none of the data those passes read
beyond the sidebar state itself (no weapon name indices, no ammo
counts, no +0x78 HP / +0x2E armor, no score/money).

1. WHAT DRAWS: only passes whose every input exists — the row
   chrome (sprites 0x47/0x4A armed, 0x49/0x4C unarmed at the EXW
   positions, gated by the availability mask bit standing in for
   the name-index word) on the redraw countdown, and the select
   portraits (0x12+slot / 0x15+slot, squad-size + alive gates;
   the HP gate is trivially satisfied while HP is unmodeled)
   every present. The initial countdown is 2 on activate
   (MissionShell 0x447c74). All from the REAL GENERAL.BIN bytes —
   the corpus gate pins real shipped pixels, not synth shapes.
2. WHAT DOES NOT DRAW, deliberately: name/count text (the type
   table's name indices + ammo counts are open — TABLE.BIN
   backlog), HP/armor bars, the score strip (score/money sim
   state), the deploy panel + blink cursor (overlay family /
   0x4dc5d0 producer open). Never invent pixels for unmodeled
   data; the plane keeps its persisted pixels between redraws
   exactly like the EXW back buffer.
3. SMLFONT.BIN stages with the mission even though no text draws
   yet (the DESIGN-GAME 12-file tail): the next slice (type
   table) consumes it, and staging-only costs 4038 B.
4. GATES: corpus frame pins regenerated ONCE (spawn
   018eba568d9b3bae, mid-walk 4a3abd2de43f31df; sim pins
   byte-identical — the D17 presentation-only split holds); the
   sidebar-black structural pin became a sidebar-carries-art pin.
   Workspace tests + fmt + clippy -D warnings clean; headless
   smoke two-run byte-identical with the two new fetches;
   MANIFEST verified.

Nudge-Worker: 49294e3c-af62-4b24-b2fa-7a12980d8eb6

## D51 - 2026-08-21: the weapon table is host-staged session state; fresh-campaign default is EMPTY

RE-EXW-SIM 7d refuted the TABLE.BIN hypothesis (TABLE.BIN = the
map-overlay backdrop bank; the 0x4de664 loadout is .bss session state
written only by shop FUN_00440e45 / save-load / MP lobby; player TYPE
0x4edb90 = 0 all SP; a fresh campaign enters the pre-mission shop with
money 4000 and an ALL-ZERO row). Engine consequence, replacing the D49
[design] all-7 default:

1. MissionScene models the per-robot weapon loadout as the 7 groups
   the EXW spawn copy reads — (name_idx: u16, ammo: u16) per group,
   staged by the host (GameHost::load_mission seam, like markers).
   set_order_availability and the all-7 mask are REMOVED; availability
   = name_idx != 0 (the EXW row gate), ammo = word1 clamped 9999 at
   draw, spawn copies word0/word1/word1 and arms 1 << first group
   with word_idx != 0 (no group => armed bits 0 — faithful).
2. The DEFAULT is the faithful fresh-campaign EMPTY loadout (all
   groups zero => no rows, no armed weapons). Nothing draws that the
   original would not; tests that need rows stage them explicitly.
3. Row TEXT now draws from real data: names via the pinned
   FUN_00420260 switch (the 39-entry compiled-in table embedded as
   data + index mapping, RE 7d.5), counts "%04i", both through
   SMLFONT at the EXW coords (0x1ED/0x25C, 0x5B+14i), color 0x24.
4. GATES: frame pins regenerate ONCE (rows only appear where the
   gate script stages a loadout; default-path frames lose the row
   chrome D50 drew because the real gate says no rows); sim pins
   byte-identical unless the staged loadout changes spawn state —
   the corpus gate stages none by default so D17/D50 sim pins hold.
   Tests + fmt + clippy -D warnings; headless smoke two-run
   byte-identical; MANIFEST verified.

Nudge-Worker: 4b75846d-3486-4bcd-be7c-fbeff298deec

## D52 - 2026-08-21: hp/armor are host-staged vitals until the damage path lands; the strip rides campaign session state

RE-EXW-SIM 7f decoded the whole family (FUN_0040807f bars,
FUN_004085ce strip, FUN_0040e230 damage, FUN_0040eba0 pickups, the
FUN_004100b7 armor pads, the dropship-landing hp init). Engine
seam:

1. WHAT LANDED: the bars (exact asm sprite arithmetic over
   per-robot Vitals {hp, armor} staged in the Sidebar — the D17
   presentation half) + the score strip (NUMBERS.BIN, the 23rd
   mission-chain asset; score/money as MissionScene session state,
   fresh campaign 0/4000) in the CORRECTED FUN_00403938 tail order
   (portraits -> bars -> strip countdown -> rows countdown ->
   chrome). The default vitals are faithful: hp = 5000 + 100*battery
   (the landing formula; battery = the BATTERY PACK 0x2B group's
   word1, re-derived by set_weapon_loadout), armor = 0 (nothing
   charges it before the pads). The empty armor bar 0x8E DRAWS —
   the original shows it every frame on a fresh campaign. The
   portrait pass gains the exact hp >= 1 gate; the case-4 pickup
   producer is a host seam (PICKUP_AWARDS by two rand_a draws from
   the shared sim stream + countdown 2).
2. WHY STAGED, NOT SIM FIELDS: the unit's constraint — sim pins may
   move only when the damage path genuinely lands. FUN_0040e230 is
   decoded, but its death path interleaves the debris family (5x
   FUN_00420608 with 2 RandA each — the shared stream), the
   robot-death pass FUN_00409138, and the SFX bookkeeping; landing
   it half-way would be worse than staging. The follow-up damage
   unit promotes hp/armor (+ the shield pool +0x88 family) to real
   hash-covered sim fields, wires apply_damage, and RE-PINS the sim
   hashes deliberately with that reason.
3. WHAT DOES NOT DRAW, deliberately: the dead/hit dither overlay
   (FUN_00401ae6 + the 0x4e6ed8 512-B mask bank — decode queued),
   the deploy panel + blink cursor (0x46ccf8/0x4dc5d0 producers
   open). Money is modeled >= 0 (every producer adds; the signed
   strip divide is identical on that domain).
4. GATES: frame pins regenerated ONCE (spawn 9ecd7691d388bbfa,
   mid-walk 333d128dc812d547, overlay 1504c600819e724c — the stale
   sidebar carries the bars/strip pixels, armed 86a788ff93bd78a5);
   sim pins byte-identical (36ddc86345c8351c / f35db41f0efb858d /
   64ef1ddbc65cba47). 41 suites green (2 new unit tests: the bar
   sprite arithmetic table + the bars/strip/pickup seam flow), fmt
   + clippy -D warnings clean, headless smoke two-run
   byte-identical, MANIFEST verified.

Nudge-Worker: 36c9e956-335d-48f4-b6f8-a988c6eba472

## D53 - 2026-08-21: hp/armor + damage land as real sim fields; the sim pins move once for that reason

Context: D52 staged the bars over presentation vitals precisely so
the sim pins would hold until the damage path genuinely landed.
This unit lands it (RE-EXW-SIM 7g pre-decode committed first by the
interrupted predecessor run that also wrote the implementation WIP;
this run validated the WIP against the exw-missionrender decompile
line-by-line - the alarm accumulator placement (before the shield
absorb, at the top of the damage path), the auto-shield gate
`charges != 0 && shield == 0`, the debris draw order (RandA#1 -> y,
RandA#2 -> x), drop = 0 (not 1) on the SP death clear, and the
death writes all match - then finished the pins/docs):

1. WHAT LANDED (bedlam-core): the Robot damage fields - hp (+0x78),
   armor (+0x30), hit_flash (+0x2E), alarm (+0x34), alarm_ctr
   (+0xA4), shield (+0x88), shield_charges (+0x8C), shield_boost
   (+0xA0), battery (+0x94), armor_pool (+0x98), kind (+0x2A),
   death_flag (+0x9C) - all hash-covered; spawn hp = the
   dropship-landing 5000 + 100*battery (set_battery seam re-runs
   it); `MissionSim::apply_damage` = the FUN_0040e230 SP core
   (state-2/alive gates, the ordered state-3 -> shield 0x20
   conversion, the auto-shield idle, the alarm trip at ctr > 100 on
   the player type, shield absorb vs hit_flash-then-hp subtract,
   and the SP death subset incl. the five debris staged from the
   SHARED stream - 10 RandA draws; DamageOutcome carries the
   presentation half); the phase-0 pre-walk (alarm/alarm_ctr decay,
   shield -2 clamp, the +0xA0 booster 10000/150 family); the
   phase-1 armor pass (pad byte -> FUN_004100b7 +20 behind the
   +0x98 pool else -10 bleed, clamp 3000/0, set_armor_pads seam for
   the MISSIONVIEW sec 8.1-open producer - all-zero on the shipped
   corpus, so armor bleeds); the portrait-pass hit_flash clamp-5
   decay in advance_frame.
2. WHAT LANDED (bedlam-game): the Sidebar Vitals staging DROPPED -
   the bars/portraits read the sim robot fields directly;
   set_weapon_loadout lands battery through sim.set_battery (so a
   BATTERY PACK group now moves the sim hash - the armed gate test
   documents the battery-less case stays at the spawn defaults);
   the apply_damage host seam stages the death sidebar-redraw
   countdown (DAT_0046ccec = 3). set_campaign/pickup seams kept.
3. DELIBERATELY NOT MODELED: the +0x32 word decay (producer
   unknown, always 0), the 0x7d2/0x7d3 tile-word hazard/phase-clamp
   family (producer open - never-invent), the seven order-word
   clears on death, the MP respawn branch, and all SFX/FUN_0042382c
   presentation. The damage producers themselves (the projectile
   callers, the 0x7d2 hazard) stay host-seamed - apply_damage is
   the seam they will call.
4. GATES: sim pins RE-PINNED ONCE with this reason (post-spawn
   1cc7b8e125165988 / post-arm 5b9c2fd5d85f9adc / arrival
   d8eeb3e608af0be4; scene spawn 1cc7b8e125165988 / click
   0bf4fb534d6b3bd5 / overlay 78a16ba63607d197 - spawn hp 5000 is
   the only nonzero new hash input); FRAME pins byte-identical
   (9ecd7691d388bbfa / 333d128dc812d547 / 1504c600819e724c /
   86a788ff93bd78a5 - the bars draw the same 5000/0 values); 41
   suites green (465 tests, 8 new damage/armor/shield/hash unit
   tests), fmt + clippy -D warnings clean, headless smoke two-run
   byte-identical AND equal to the recorded baselines (scene
   696adb1cd110e062, frame parity cce30c983b97b16d), MANIFEST
   verified.

Nudge-Worker: 416ca029-3c29-4b69-b978-09fb4222af4d

## D54 - 2026-08-21: the pickup consumer lands as decode + sim seam; the tile-word producer stays host-seamed

1. DECISION (bedlam-core + bedlam-game): the FUN_0040eba0 pickup
   family (RE-EXW-SIM 7h) lands in two pieces. (a) The DISPATCH is
   a pure decode `pickup_case(tile_word, terrain_set)` over the
   verified DGROUP range tables 0x454a58/0x454a74 (closed 4-word
   groups -> cases 1/3/2/4 and 9/7/8; 28 pickup words per terrain
   set). (b) The case BODIES are the sim seam
   `MissionSim::apply_pickup(robot, case)`: case 1 drop=1000, case
   2 shield=1000, case 3 hp +=2500 clamp 5000, case 7
   shield_boost=200 - writes to the already-hash-covered D53
   fields, so the hash moves exactly when invoked. Game side adds
   the thin MissionScene::pickup host seam; case 4 stays the D52
   pickup_score_money producer (session state + the strip
   countdown + the two shared-stream draws). Cases 8 (ammo) and 9
   (episode staging) remain unlanded - their producers are
   mission-shell slices, not vitals.
2. WHY: the caller-side producer (the type-DB mirror word read +
   the DAT z-plane consume + the mirror floor-word swap, 7h.3)
   needs the 0x4796bc per-tile mirror the engine Terrain does not
   model (MISSIONVIEW sec 8 open producers) - landing it is its
   own slice, and never-invent forbids guessing the mirror from
   the DAT planes alone. The decode + bodies are independently
   verifiable now.
3. NOT MODELED: the 0x43a48e SFX queue entry, the 0x4dc5d0
   sprite-effect row staging (the per-case ids 1/6/7/0xE ride
   PickupOutcome for that future slice), the MP FUN_00425647
   tail, and the terrain-set-per-zone mapping ([hypothesis: set =
   zone+1] recorded in 7h.4, untested until the producer lands).
4. GATES: no corpus path invokes the seam - sim/frame pins
   byte-identical at the recorded baselines (scene
   696adb1cd110e062, parity cce30c983b97b16d); workspace green
   (+4 pickup tests), fmt + clippy clean, smoke two-run
   byte-identical, MANIFEST verified.

Nudge-Worker: 66831068-5861-4218-8409-6b1e3d3f360e

## D55 - 2026-08-21: the dead/hit dither lands as pure presentation; the 0x4e6ed8 "mask bank" refuted as runtime noise state

1. DECISION (bedlam-game): the FUN_00401ae6 dither family
   (RE-EXW-SIM 7i) lands entirely inside the MissionScene
   presentation half. (a) The noise ring: a 2048-byte binary
   {0x00, 0xFF} bank (25% white) + a persistent cursor — boot
   filled at activate (the MissionShell staging 0x447b13 analog),
   churned 15 bytes/frame at the present epilogue (0x448147 —
   unconditional, overlay frames included). (b) The blit in the
   portrait pass: per slot, dead/hp<1 or beyond-squad -> mode 0
   (full static REPLACES the box — the EXW dithers the unoccupied
   boxes EVERY frame, asm 0x4073d8/0x4073fc); alive with sim
   hit_flash != 0 -> the portrait draws THEN mode 1 overlays only
   the nonzero bytes (the portrait survives under zeros). The pass
   READS hit_flash and never decrements it — the 7g.8 decay stays
   the sim per-frame tick (D53 hash-covered field).
2. CORRECTION (RE): the queued-unit "512-B mask bank" gloss is
   refuted — 0x4e6ed8 lies in .bss: the bank is RUNTIME state,
   0x800 bytes, produced by RandB draws (fill + churn); 512 is
   the reseed mask (RandB()&0x1ff), not the size. The blit
   reseeds (not wraps) when src+96 >= 0x800; the per-blit seed is
   FUN_0041ec59(0x7f6,0x30) = (RandB()&0x7fff)/15 clamp 0x7f5.
3. STREAM MODEL: all dither draws (fill/churn/seeds/reseeds) come
   from the ONE shared mission RandB stand-in (charter T3) —
   edge_rng renamed rand_b — consumed in the EXW per-frame order
   [7i.4]: terrain edge variants -> dither draws -> churn. The
   sidebar block moved AFTER the terrain pass in present() to
   mirror the FUN_00403938 order; pixel output is unchanged
   (disjoint plane halves: viewport [0,480) vs sidebar [480,640)).
4. NOT MODELED: the EXW bit-stream itself (T3 stand-in); the
   0x4ddb30 cursor is NOT reset at activate (the fill writes the
   bank only; the EXW cursor is .bss zero and MissionShell does
   not zero it per mission).
5. GATES: sim pins byte-identical (spawn 1cc7b8e125165988, click
   0bf4fb534d6b3bd5, overlay 78a16ba63607d197, armed-sim
   unchanged); frame pins RE-PINNED ONCE with the reason recorded
   in the gate header (spawn 7fdada56b10f1cad, walk
   58ea10373e8d4284, overlay 1d70e0bd059f5ae0, armed
   6050d20755b2d852 — ZONEA spawns a 1-robot squad, so the two
   beyond-squad boxes carry static every frame); the overlay
   gate's "stale sidebar" reference re-anchored to the
   last-presented frame (the per-blit seed draws make normal
   sidebars differ frame-to-frame, exactly like the EXW); 41
   suites green (+1 dither unit test), fmt + clippy clean, smoke
   two-run byte-identical AND equal to the recorded baselines
   (scene 696adb1cd110e062, parity cce30c983b97b16d — the smoke
   hashes are end-of-journey cutscene state, mission frames never
   entered them), MANIFEST verified.

Nudge-Worker: efc8b1e0-9dfb-4f2d-a0ae-1688ac88db6f

## D56 - 2026-08-21: the 0x4dc5d0 effect-row family + the debris stager land as presentation over the landed sim outcomes

1. RE FIRST (RE-EXW-SIM 7j, committed before the code): the 10
   effect rows are 16-B records {x, y, z, id} at 0x4dc5d4 (ids at
   0x4dc5e0 + 0x10k — the FUN_00422038 allocator scans those,
   falls back to row 9); every FUN_0040eba0 case tail stages one
   via `row = {pos_x>>8, pos_y>>8, z+0x20, id}` with the ids
   {1,6,7,1,0xE,0xC,0xD} per case {1,2,3,4,7,8,9}; the
   FUN_0042205c tick rises z += 6 to the 0x190 cap then frees
   (MissionShell epilogue 0x448080, BEFORE the draw); the draw
   pass (FUN_00403938 tail) enqueues FLAGS.BIN sprite id−1 at
   layer 0x12c with its own +0x118/+0x124 projection; the scalar
   `_DAT_004dc5d0` is a SEPARATE variable — the blink-cursor
   selector (the selected robot's slot + 1, producers the robots()
   select-ack blocks 0x40c1ae..0x40c25e + the MissionShell entry
   zero, consumer the 0x407420 sidebar switch drawing GENERAL
   0x51+(frame&3) at (0x1F0+0x32k, 0xD)). The debris stager
   FUN_00420608 = 128 slots × 0x30 B at 0x476fbc, z clamp
   0x20..0xFF, first-free-else-min-seq eviction, 20-kind jump
   table; kind 5 (the death tail) writes the record + SIX
   FUN_00422287 scorch-ring calls (the type-DB per-tile +0x18
   byte writer — the MISSIONVIEW §8.1-open producer, with the
   armor-pad interaction caveat recorded); the FUN_00420549 tick
   walks the per-kind i16 sequence table (kind 5: sprites
   5..0x10, −1 frees); the draw pass reads BLOWUP(B/G).BIN
   (region-gated), layer 0x12c for kinds 3/7/0xA else 0x12e.
2. ENGINE: bedlam-render MissionView gains the Flags/Blowup
   NodeBanks + set_flags_bank/set_blowup_bank +
   enqueue_effects(rows, debris, cam) with the verbatim
   projections/bounds/modes (0x12c is plain copy per MISSIONVIEW
   5c); bedlam-game MissionScene carries EffectRows (10 rows,
   alloc/stage/tick) + DebrisFx (128 records, stage_kind5/tick
   over DEBRIS_KIND5_SEQ) as D17 presentation state; the damage
   seam stages the five outcome rows (z already +8k in the D53
   outcome, delay = 2k [hypothesis: the Watcon stack-slot alias
   maps +0x24 to the caller's 2k counter — flagged for P4.2]),
   the pickup seam stages the row with the outcome's effect id,
   present() ticks debris then rows (the 0x448076/0x448080
   epilogue order, overlay frames included) then enqueues; the
   blink cursor = Sidebar.cursor (0 until the select-strip
   select-ack — the MissionShell entry zero; [hypothesis: the
   EXW per-frame ack may light it from spawn, left 0 per
   never-invent until a corpus capture arbitrates]) drawn in the
   portrait-pass tail. FLAGS.BIN + BLOWUP.BIN join the mission
   chain (23 → 25 files; the BLOWUPG region variant is a host
   path choice, unmodeled). The six scorch writes are NOT staged
   (the 7j.8 armor-pad caveat needs its re-verify first).
3. GATES: every existing pin UNMOVED — the effects draw nothing
   on the default corpus path and the cursor stays 0 until a
   select click, so the scene gates pass byte-identical (spawn
   7fdada56b10f1cad ... armed 6050d20755b2d852, sim pins
   unchanged) and the smoke is two-run byte-identical AND equal
   to the recorded baselines (scene 696adb1cd110e062, parity
   cce30c983b97b16d, audio 110400/158092; the fetch list grows
   to 25 with FLAGS 14478 B + BLOWUP 150034 B). NEW: three
   enqueue_effects render units + six game units (alloc/tick,
   clamp/LRU/seq-walk, the death/pickup stagings, the cursor
   ack) + the corpus gate zonea_effect_rows_and_debris_draw_and_
   expire (the LNK walk makes consecutive frames differ, so the
   draw evidence is a CONTROL-host diff at the same pump index —
   identical pumps + identical death, the only divergence the
   FLAGS icon — plus full two-run determinism over the effects
   journey and the expiry state assertions). 41 suites green,
   fmt + clippy clean, MANIFEST verified.

Nudge-Worker: 6ab53863-71dc-4010-b6eb-fa9a3f724411

## D57 - 2026-08-21: the scorch ring lands in the sim death tail - the +0x18 reader is raw, no mask

1. RE FIRST (RE-EXW-SIM 7j.9, committed before the code): the
   7j.8 caveat is RESOLVED. The robots() phase-1 armor reader
   (0x40bc57..0x40bc9f) tests the RAW record +0x18 byte != 0 -
   no mask, no value family; FUN_00422287 (0x422287, whole
   re-verified) writes that SAME byte (same 0x4796d4 + tile*0x1E
   addressing, sar>>5 tile from world, map bounds, value >= 8
   clamped to 7). Scorch values and armor pads SHARE the byte:
   a death genuinely arms 3x3 armor-pad tiles around each debris
   - quirky but verified original semantics.
2. CORRECTION to 7j.5/D56: the kind-5 ring is NINE 3x3 tile
   writes, not six - TL/L/BL/T/C/B/TR/R/BR at world +-0x20
   (= tile +-1 after the writer's >>5) with corners 1, edges 2,
   center 4, in that exact order (0x421476..0x421291 incl. the
   shared tail entry); a death = 5 debris x 9 = 45 writes,
   overlapping rings last-write-wins in staging order. Census:
   SEVEN in-family producers (kinds 3, 4, 5, 6+12 shared, 9, 11,
   20 - identical rings, corner value 1 per kind) + ONE external
   census-only producer FUN_00424051 (five same-tile re-rolls,
   values 3..6 then 1..4 - unidentified purpose, unwired).
3. ENGINE: MissionSim::scorch_write models FUN_00422287 over the
   existing armor_pads type-DB mirror (zero-padded growth on
   first write - the default corpus stays all-zero until a
   death); the apply_damage death tail stages the nine ring
   writes per debris row in the EXW order. armor_pads remains
   hashed only through its armor effect; no corpus gate stages a
   death before its pins, so EVERY pin stays unmoved. The
   scene's DebrisFx is untouched - scorch is sim state, not
   presentation; the other six ring kinds + FUN_00424051 stay
   unwired (no corpus-path producer yet).

Nudge-Worker: 11384359-21d7-4dbe-8130-1d504d6c2511

## D58 - 2026-08-21: FUN_00424051 = the epilogue tick - the +0x18 fade lands, every pad/scorch byte is transient; the splash system stays unwired

1. RE FIRST (RE-EXW-SIM 7j.10, committed before the code): the D57
   item-5 "unidentified" producer is the per-frame MISSION-EPILOGUE
   TICK (call 0x447ff0, immediately after the debris tick
   FUN_00420549) and it does two things. (a) The GLOBAL +0x18 FADE:
   every nonzero armor-pad/scorch byte on the whole map decays by 1
   EVERY frame, unconditionally - so the D57 ring is TRANSIENT (a
   value-4 center arms its pads for exactly four phase-1 passes)
   and permanent map pads CANNOT exist (confirms MISSIONVIEW 8.1:
   no static +0x18 producer). (b) The WATER-SPLASH EVENT TICK: a
   250-record array @0x4e9778 (stride 0xA {x,y,z,delay,age}) -
   weapon impacts (11 stager callers in the FUN_0041a894 weapon
   family, one co-staging debris) stamp the zone water sprite at
   the first free z-level (FUN_0041bd78), drain down through
   empty levels on odd frames (g_frame_count&1), absorb when water
   is directly below, re-stamp water_base+0x16 at age 40, dry up
   and free at age >= 47, scorching the tile every tick (the
   seven-word 7j.9 item-5 re-roll writes). Supporting family:
   FUN_0042394a = the z-structure writer (TOT z-word + seen byte +
   DAT volume byte - the map-edit primitive), FUN_0041eb28 = the
   DAT volume read (NOT a visibility test - corrects 7j.9's
   guess), FUN_00424355 = the stager (claim-bank gated, LRU
   eviction with a cancel call).
2. ENGINE: the fade lands at the advance_frame tail (epilogue
   position): iterate armor_pads, nonzero -> -1. Corpus-safe -
   armor_pads has no corpus producer and set_armor_pads is
   test-only, so all pins UNMOVED (smoke two-run byte-identical AT
   the baselines: scene 696adb1cd110e062, parity cce30c983b97b16d,
   audio 110400/158092). The two permanent-pad unit tests now
   stage the max value 7 (outlives their frames); a new unit test
   covers the decay, the single-charge value-1 case, and the full
   death-ring fade. The splash system stays UNWIRED - no
   corpus-path producer (weapons never fire in the gates);
   re-open when the weapon family decodes.

Nudge-Worker: 89d34b53-1d5c-4d36-ab77-7cf704547435

## D59 - 2026-08-21: the FUN_00420608 kind census is docs-only - no debris kind edits terrain beyond the rings; kinds 1/13/14/15 DO ring (7j.9 corrected); the 0x4203a5 z-writer call belongs to the sibling arrival scheduler

1. RE (7j.11): the queue's 0x4203a5 question is answered
   NEGATIVE for debris - the FUN_0042394a call sits in
   FUN_0042034c (the delayed-arrival scheduler, epilogue
   0x448076), NOT in a kind body; the stager contains zero
   type-DB references and zero z-writer calls. The complete
   20-kind table is now pinned (seq tables, physics classes,
   init sizes, per-kind ring behavior, the two arrival-SFX
   helpers + the k11 LCG gate) plus the full 47-site caller
   census: every kind except 5 (the death tail) belongs to the
   weapon-fire/impact families, the platform/destructible
   family, the selection chaser, or FUN_004244a1 - all off the
   current corpus path.
2. ENGINE: no change this unit - the census feeds the LATER
   debris-stager widening beyond kind 5 (backlog). The landed
   kind-5 death model (D53/D57) is unaffected by the 7j.9
   ring-census correction because kinds 1/2/8 etc. have no
   engine producers yet; when the widening lands it must model
   the k1/k20 shared-tail ring, the k2/k8 single-center writes
   (values 3/4), and the +0x20 physics classes (0/1/2/3/6 ->
   FUN_0040de9c) per the 7j.11 table.

Nudge-Worker: 804e8c9d-76fc-4936-a020-a83282838d7e

## D60 - 2026-08-21: the platform/destructible family decode is docs-only - the gate banks are an object-word grid + platform strength; the 0x7d2/0x7d3 and type-DB +0x19/+0x1a producers close

1. RE (7j.12): word[0x460dfa+2*tile] is NOT a TOT mirror but a
   runtime OBJECT-PRESENCE grid (0 empty / 0x7d2 hazard /
   0x7d3 phase-clamp / 0x7d4 platform / n>0 = destructible
   object record n-1 at 0x46cbf4, stride 0x14 {x,y,z,id,flags,
   hp}); word[0x465daa+2*tile] is the PLATFORM STRENGTH word.
   FUN_00422693 (weapon ray 0x41a8ff) weakens (strength -=
   damage, scorch +4 via the NEW increment writer FUN_0042223c,
   ring spread when >=100 and (hit<200 or new<100)) or destroys
   (FUN_0042394a(x,y,z,0,0) clears the water z-word + both
   banks + 5 kind-7 debris @0x4227b9). FUN_00422832/FUN_004228ce
   build platform tiles (empty z-word + planeA 0 + planeB 1 +
   no robot -> water z-word create @0x422a54 + 0x7d4 + strength
   300 trigger / 199 creep); FUN_00422a9c is the 1/32 creep tick
   over water rays from the site latch 0x4dc5c8/cc. FUN_00422f18
   stamps 0x7d2/0x7d3 at load from the 0x454a20/0x454a3c
   per-zone z-word ranges (closes 7g.5); FUN_00422fd1 stamps the
   type-DB +0x19 (variant<<4) / +0x1a (0 / 0x80 by type) from
   the 45x0x10 rectangle list at 0x4dcae8 (closes MISSIONVIEW
   8.1's +0x1a; +0x1b/+0x1c stay open); FUN_00422cc2 is the
   32-timer delayed-trigger tick whose expiry writes the
   0x454a90 bare-floor z-word via FUN_0041bd54 (fast z-writer).
2. ENGINE: no change this unit - the family's callers (weapon
   ray, MissionShell load/epilogue, 0x433xxx scripts) are all
   off the corpus path; banks/timers stay unwired
   (never-invent). Re-open points: the weapon-fire family (the
   grid's object-stamp loop + the ray dispatch), the 0x425xxx
   arrival producers, and any future mission-load seam for the
   0x7d2/0x7d3 stamper.

Nudge-Worker: 5aa2d164-5a28-4d42-805a-7b2f629bd29f

## D61 - 2026-08-21: the weapon-fire first hop is docs-only - FUN_0041a894 is the per-tile impact resolver (no walk); the ray lives in the callers; the object type table closes

1. RE (7j.13): FUN_0041a894(x Q13, y Q13, chain ctr, damage,
   [stack] score flag) = the WEAPON-IMPACT OBJECT RESOLVER -
   bounds-check, read grid word[0x460dfa+2*tile], dispatch (0/
   0x7d2/0x7d3 pass-through ret 0; 0x7d4 -> FUN_00422693
   platform damage ret 0; n>0 -> object rec n-1: hp -= damage,
   destroyed -> flags 0x40 + destroy tail ret 1). It does NOT
   step a ray: the WALK is the projectile tick FUN_00412010 (50
   rec @0x4cc654 stride 0x22, ballistic x/y/z += v, terrain
   probe FUN_0041eaa1) plus the robot fire controller
   FUN_00410823 (8 sites, weapons 5/0x1a blast x4/0x24/0x29,
   damage = FUN_00419aff(id,1)), the tile-0x62 trap pair
   FUN_0040fe93/FUN_0040ff92 (damage 100, 5x k12 debris), the
   script blast FUN_004244a1 (damage 5000), and 4 chain
   self-calls (perimeter walks, damage 1000). The 7j.12
   "object-stamp loop 0x41a84f" is FUN_0041a7f0 (footprint
   stamper) invoked from the mission-load restamp pass
   FUN_0041a4f8, which also parses the OBJECT TYPE TABLE
   (0x4dedf2, 0x4E stride, 282 recs from the mission file: W/H/D,
   hp, chain, type, jitter words, 4 scratch banks).
2. ENGINE: no change this unit - all 17 call sites are
   player/script-driven and off the corpus path; the impact
   resolver, projectile tick, fire controller and type table
   stay unwired (never-invent). Re-open points: FUN_0041bc1c
   (the terrain/robot sibling resolver), the FUN_00410823
   weapon-anim machine, the 160-vs-0xA8 stride anomaly at
   0x4c69e4, and the type-table's remaining words.

Nudge-Worker: b7f866b6-9b16-4d83-ab08-cc080284ee3b

## D62 - 2026-08-21: the weapon-fire second hop is docs-only - FUN_0041bc1c is the terrain-STRUCTURE resolver (new 0x4cccf8 array); the probe is a per-pixel height test; both disbursers are per-record debris kind maps

1. RE (all [verified] vs ghidra-project/exw-weaponfire2.txt,
   ExwWeaponFire2.java): FUN_0041bc1c(x Q13 eax, y Q13 edx,
   damage ebx) resolves damage against the TERRAIN-STRUCTURE
   array @0x4cccf8 + i*0x20 (i < [0x46ccd4]; {active@+0,
   hp@+0x10, x@+0x14, y@+0x18, z@+0x1C}; externally 1-based via
   dword[0x4cccd8+id*0x20], 0x4cccd8 = the id-0 guard). hp -=
   damage; survivors take NO other write; hp<=0 -> active=0 +
   the floor word [0x454a04+4*zone] stamped into the TOT mirror
   (0x4796bc+30*tile+2z) + seen byte (0x4796cc) + DAT volume
   cleared + debris K0xF + splash at the first free level. NO
   robot-armor branch, NO platform call - the 7j.13 "terrain/
   robot" question closes TERRAIN-only. FUN_0041eaa1(x/y Q5, z)
   = the projectile terrain-height probe: DAT volume byte 0 ->
   miss; else the 32x32 height bank behind [0x4edd60] entry
   (h-1)*4+2 (+6 header), hit iff z <= (z>>5)*0x20 + bankbyte;
   3 sites in FUN_00412010. FUN_004124a4 = the weapon-anim
   disburser (rec 0x4c71f4+0x36*i, kind word@+0 = weapon id:
   2..4->K2 jitter, 5->K3, 0x24->K6, 0x29->K9, {0xE,0xF,0x13,
   0x17,0x1A,0x1F}->K0xC, 9..0xB clear-only, z-10). FUN_004126dc
   = the projectile disburser (rec 0x4cc654+0x22*i, +0 = TYPE
   word 0=free - refines 7j.13 "active": 1->K2, 0x65->K0x14,
   0x66->K8, 0x67/0x68->K4, no z-10; FUN_004197d4 = the
   robot-hit expiry walker |dx|<0x10 Q8, |dz|<0x20). Projectile
   type ids = weapon-stat ids. Splash addendum: FUN_00424355
   gates (DAT-empty AND TOT word 0 AND claim byte[0x46af58]) +
   max-age eviction via FUN_0042394a.
2. ENGINE: no change - unchanged corpus verdict from 7j.13
   (D61); all fire/impact sites stay player/script-driven and
   unwired. Re-open points: the 0x4cccf8 array PRODUCER
   (mission-load stager), the [0x4edd60] height-bank family,
   FUN_00410823 internals, FUN_00419aff's table layout, and the
   projectile-record z encoding (site-1 arg shape).

Nudge-Worker: d37fb3a2-9df1-482a-88c5-20504c5bb254

## D63 - 2026-08-21: the weapon-fire third hop is docs-only - FUN_00419aff is a pure id->damage switch scaled by the NEW difficulty dword 0x46cbf8; the 0x4cccf8 terrain-structure producer is the ".TRT" mission-section loader; the TRT third field is the z level

1. RE (all [verified] vs ghidra-project/exw-weaponfire3.txt +
   exw-weaponfire4.txt, ExwWeaponFire3.java/ExwWeaponFire4.java):
   FUN_00419aff(EAX id) = the WEAPON/PROJECTILE DAMAGE TABLE, a
   pure switch, NO table walk: 2->20, 3->30, 4->40, 5->75,
   0xc->5000, 0xd->312, 0x1a->75, 0x24->400, 0x29->250,
   0x65->(d+1)*50, 0x66->(d+1)*300, 0x67/0x68->(d+1)*75 (d=2
   flat overrides 200/1200/300), else 1. The 7j.13 second-arg
   "(weapon_id, field)" reading is an ERRATUM - EDX passes
   through untouched; the only second selector is DAT_0046cbf8
   = the DIFFICULTY dword (0..2): cycled (d+1)%3 at
   NameEntryScreen, save-persisted, money 500*d vs 4000 base,
   zone-7 temporarily forces d=2 (GameMain around FUN_0044771c).
   28 callers: FUN_00410823 x16, FUN_004190bc x6 (stat reads off
   the 0x4cff98 bank - a second consumer, likely panel/preview),
   FUN_00412010 x4, FUN_004197d4, FUN_00418fca. The 0x4cccf8
   PRODUCER = FUN_004170a6, the ".TRT" section loader (sole
   caller FUN_00416458 0x416487, the mission-load dispatcher
   clearing 0x4cff98/0xac44 + 0x4dabdc/0xf00 then opening .NME;
   sibling tags .MOFO/.NME/.TRT/.POS/.BDG): clears the FULL
   250-rec bank (0x1f40 B), count u16 -> [0x46ccd4], per rec
   3x u32 reads -> stager frame 0x4cccfc {+0=1, +4 active=1,
   +8=0 scratch (no producer found), +0xC hp = 250 + (250 *
   [0x46ae8c])/27 = 250+250*linear-mission/27 -> 259..490,
   +0x10 x, +0x14 y, +0x18 z}; ALSO stamps tile byte 0x66 into
   the 3D tile bank byte[[0x4edd58]+x+y*w+z*w*h] and word 1
   into a second 3D word bank word[[0x4ede20]+2(x+y*w+z*w*h)].
   7j.14's resolver frame (base 0x4cccf8) is +4 - all its
   offsets stand; the external 1-based idiom unchanged.
   FORMATS-MISSION sec 14 ANCHORED: the TRT third u32 is the
   z LEVEL (0..6), not a type enum - "turrets?" retired as
   primary; open consumers FUN_00417264/FUN_00419943/
   FUN_0041ee20.
2. ENGINE: no change - unchanged corpus verdict from 7j.13/7j.14;
   the fire/impact family stays player/script-driven and
   unwired. Re-open points: the FUN_00410823 anim-machine
   internals, the structure consumers above, the two 3D banks'
   other consumers, the FUN_004190bc 0x4cff98 record family,
   and the +0x08 scratch dword producer.

Nudge-Worker: efff097c-b4e9-41a0-b4ce-fcdc7fbf713e


## D64 - 2026-08-21: the .TRT consumer hop is docs-only - TRT structures are SHOOTING SENTRY TURRETS (animate + fire, never move); the two 3D banks are the ".TOT"/".DAT" map file volumes

1. RE-EXW-SIM 7j.16 pins the three 0x4cccf8-array scanners and
   closes every open point of the 7j.15 unit:
   - FUN_00417264 (MissionShell tick 0x44807b) = the TRT
     ANIMATION/FIRE state machine. Canonical rec frame
     (active@0x4cccf8): {active@+0, state@+4, anim_frame@+8,
     fire_ctr@+0xC, hp@+0x10, x@+0x14, y@+0x18, z@+0x1C}. The
     "+0x08 scratch dword" IS the animation frame; its runtime
     producer is this machine (no file producer - 7j.15/D63
     open point closed). States: 1 idle -> 2 alert (frames 0..7
     -> TOT mirror word frame+1 via FUN_00417210) -> 5/6/7/8
     aim S/N/W/E (octant toward the nearest robot,
     FUN_00417c00 octile probe, dist<0x81) -> FUN_00417698
     FIRE at the frame top + a 4-frame muzzle flash (mirror
     words 0x17..0x1E); 3/4 = death/settle for destroyed
     structures. FUN_00417652 = frame remap 0xF->7, 6->0xE.
   - FUN_00417698 = FIRE: per aim lane, target iff
     |lateral|<0x28 px beyond the structure AND <=2 z-levels
     (robot bank 0x4c69e4/0xA8, count 0x46ccbc); arms
     fire_ctr@+0xC, on odd ctr stages PROJECTILE TYPE 0x66
     (damage (d+1)*300, the heaviest enemy projectile) into
     the 0x4cc654 bank via FUN_0041286f (free-slot finder -
     confirms 7j.14's type-word-0-free convention). The
     7j.15 "turrets? retired" note is itself retired:
     FORMATS-MISSION sec 14 re-anchored - TRT = turret
     placements. Structures NEVER move.
   - The two 3D banks are the mission map FILE VOLUMES
     (FUN_0041dc5a = the map loader, MissionShell 0x447b3a):
     [0x4ede20] = ".TOT" (u16 W, u16 H header + 8 planes W*H
     u16; corpus-verified ZONEA 30004 = 4+2*25*75*8, ZONEB
     160004 = the 100x100x8 arena max), [0x4edd58] = ".DAT"
     (same header, u8 planes, >=0x80 sanitized to 0, 0xFF =
     pad stamps, 0x66 = turret tile). Same loader: .CGR ->
     height banks, .BIN -> [0x4ede1c] (word -> 0x46cdb8),
     .MIN, .LNG/.LNK variant, .PAD -> 999x8B slots 0x4e44f8.
     KEY CONSUMER FUN_00440a2d (caller FUN_00440dc2) = the
     TOT-volume -> TOT-mirror MATERIALIZER (7x7 tiles x 8 z:
     word!=0 AND DAT byte==0 -> mirror word + seen) - how the
     TRT word-1 stamp becomes the visible sprite frame;
     FUN_0044661b = the save/EDITOR\ZONE restore reload.
   - FUN_00419943 = the map-click PICK (rect list 0x4787c4
     written by the renderer FUN_00403938, octile cost
     FUN_0041ebf8, else screen->iso IDIV + TRT box test;
     ret 0 = ground / k+1 = rect / (idx+1)|0x2000 =
     structure; FUN_00418a9f = an EMPTY stub). FUN_00410644
     (MissionShell 0x448021) = the click ORDER dispatcher ->
     order target {x,y,z} 0x4dd484/88/8C consumed by the
     robot behaviour family. FUN_0041ec81/FUN_0041ee20 = the
     corner SCANNER widget (GAMEGFX\SCANNER.BIN) drawing
     marker icons (8 = TRT structures) around the selected
     robot via FUN_00402572 (128x128 blitter -> 0x4eddb8).
   - 7j.13-erratum correction: the uncommitted 22c1c14b
     draft's W/H/D +0/+2/+4 shift is WRONG (its dword>>16
     anchors consume word@+2/+4/+6 - instruction-proven at
     0x41a857/0x41aa02/0x41aaf9/0x41a6fc); original 7j.13
     offsets stand, word@+0 = unconsumed [open]. The draft's
     5x8B effect-entry block (+0x16 selectors, 9-case table
     0x41a870), count@+0x12, template banks @+0x3E..+0x4A and
     the 0x4E closure are CONFIRMED and kept.
2. ENGINE: no change - corpus verdict unchanged (the turret
   animator/fire stays unwired like the rest of the weapon
   family). Re-open leads queued: the robot targeting family
   (FUN_00412a98/FUN_00412f34/FUN_00417e2f - the other
   FUN_00417c00 callers and both 0x46cbf8 readers), the
   0x4dd484 order-target robot behaviour family,
   FUN_00440dc2 (materializer caller - scroll restamp?),
   the 0x4787c4 hot-rect record layout, and the FUN_00410823
   anim machine (unchanged from 7j.13).

Nudge-Worker: 16f43187-a265-44ee-8a03-96137fcb721a

## 2026-08-21 P4 7j.17 — the ROBOT TARGETING/AIM family
(adopt unit: three provider-outage-killed runs landed as one
docs hop; worker 3f4f7c10)

1. DOCS (RE-EXW-SIM 7j.17 + ledger rows + open items; all
   re-verified against the dead runs' on-disk Ghidra dumps
   exw-robottarget*.txt/-xrefs/-asm, NO new Ghidra run):
   - FUN_00412f34 = the 0x4cff98 CRITTER-ACTOR controller
     (stride 0x7E, count DAT_0046cc2c <- FUN_00416458
     @0x41646d; sole caller MissionShell @0x447fe1). States
     1 wander / 2 sine-walk shooter (projectile 0x65 at a
     random alive robot, range gate (2-d)*-0x40+300) /
     3 chase-combat (FUN_00417c00 probe, modes 2 attack ->
     projectile 0x67 full-3D velocity / 3 approach +
     pathfinder FUN_0041571c / 8 idle / 10 return-home,
     leash 400) / 4-5-6 mixed-AI (modes 0xB dormant with
     the DIFFICULTY respawn-delay table DAT_00454edc[d],
     7 dying 0x28, 6 ballistic -> landing floor probe +
     8x debris k6 + 5x FUN_00424355 + splash
     FUN_0041a14f(0x18), 9 seek-steppers, 2 range attack
     FUN_0040db9e) / 7 close-combat (point-blank 0x69,
     fire rate 32/16/8 by d, break odds 1/8·1/16·never,
     leash (d+1)*0x40+600). Presence byte mark
     [[0x4ea900+(y>>13)*4]+[0x46af4c]+(x>>13)] := 1
     (SAR 0xD asm-verified; decompile >>5 = artifact).
     Q13 coords x@+0x36/y@+0x3A/z@+0x3E CONFIRMED.
   - Difficulty dial AMENDED: 12 objdump sites in
     FUN_00412f34 - it drives critter behavior (respawn
     delay, ranges, fire rates, break odds), not only
     projectile damage (7j.15's "scales only damage" row
     corrected).
   - FUN_00417e2f = the SUICIDE-BOMB trigger (< 0x30 px ->
     deactivate + 8x debris k1 + rings).
   - FUN_00412a98 = the 0x4dabdc POI/PERSONNEL controller
     (stride 0x1E, count DAT_0046cbf0 <- FUN_00416458
     @0x416f6e): flee-to-exit machine over FIVE 0x1C
     exit/threat slots @0x4e662c (kind 2, nearest via
     FUN_00417c64, producer FUN_0041fa51 [open]); escape ->
     [0x4eba0c]++ progress, [0x4eba10]=0x32 quota,
     FUN_00448b80(5000).
   - FUN_00409138 = the COMMAND-RECORD consumer (7j.13's
     "robot behaviour pass" pinned): records 0x4dd4a0
     stride 0x80 count DAT_0046cbe0 (the per-frame command
     ring - builder FUN_00449c94, MP lobby/SHOP readers);
     flags byte@+5 (bit0 select/auto-arm, bit1 ORDER ->
     0x4dd484/88/8C); the 39-case weapon switch: order
     dispatchers FUN_0040b615/0xaf98/0xa56f/0xace8/0xa7a1/
     0xa9ff + projectile spawners into the 400x0x36 bank
     0x4c71f4 (types 0x9..0xB/0xF/0x13/0x1A/0x1F/0x24 aimed
     at the order target; ammo/enable/cooldown bookkeeping;
     auto-rearm + msgs 0x1C..0x21 FUN_004239ef).
   - FUN_00448b80 = the MISSION-OBJECTIVE RESOLVER (6x0x20
     slots @0x4eaaee; type 5000 rescue vs kill-stats
     [0x46cbf4]+type*0x14 + mirror-row wipe 0x4796d7/d8;
     msgs 0x26/0x27/0x34, all-done 0x28+0x29 -> phase state
     DAT_0046cd00; zone-7 types 0x44..0x47 counter
     [0x46cce0]).
   - Helper identities: FUN_0041e411 floor probe (the
     [0x4edd60]=.CGR height-bank semantics: per-type entries
     + in-tile 0x20x0x20 byte maps -> closes the height-bank
     backlog head), FUN_0041f8f9 walk probe, FUN_004186fc
     standing check, FUN_004182c3 z-settle, FUN_00417af2/
     FUN_004181bd dominant-axis steer, FUN_00412848 400x0x36
     free slot, FUN_0041286f 50x0x22 free slot,
     FUN_0041a14f effect-row spawner (0x4cec38 bank gets
     its first reachable producer), FUN_004180b9 NOP.
   - Census folds: the residual 0x4dd484 reader census
     CLOSED (writers FUN_00410644 + FUN_00409138; readers
     FUN_00409138 x6, FUN_0040af98 x3, 0xa56f/0xa7a1/0xace8/
     0xb615/0xa9ff x2, FUN_00449c94); 0x46cbe0 MP-family
     census; 0x46cc2c/0x46cbf0 producers + sidebar/scanner/
     physics consumers. The 7j.11 47-site and 7j.15 28-site
     censuses re-read, unchanged; critter death adds their
     first non-weapon producers (k1, k6, FUN_00424355,
     FUN_0041a14f).
2. ENGINE: no change (D65, docs-only) - the critter/POI/
   command/objective families stay unwired like the rest of
   the weapon family. Next bounded head queued: the
   critter/POI/exit LOADER section inside FUN_00416458
   (which mission file feeds 0x4cff98/0x4dabdc/0x4e662c).

Nudge-Worker: 3f4f7c10-b73d-4662-8d35-0d770246bdd3

## 2026-08-21 P4 7j.18 — the critter/POI/exit LOADER hop
(worker a840f0af)

1. RE: FUN_00416458's critter hop decoded — ".NME" (@0x457a57,
   bytes verified) is the SOLE feeder of both banks: 8
   fixed-order u16-count sections (widths 10/10/8/8/10/8/6/8)
   → critter states 2/1/5/4/3/6/7 + 4 POIs per section-8
   record (personnel spawn in state 5 ESCAPE, flee-to-exit).
   Corpus-exact on all 37 files (ZONEA/M1 has a 16-B orphan
   tail no game code reads). FORMATS-MISSION §9 rewritten;
   the old "header (n1,n2)/(count,type)" grammar was a
   mis-split of the fixed schedule. FUN_0041fa51 = the
   EXIT-PAD ACTIVATOR (runtime: .PAD slot index → one of the
   5 exit slots @0x4e662c, dedup registry @0x46cd20; caller
   FUN_00433980 = pad trigger handler [open]). 7j.17
   leftovers folded: FUN_00449c94 (local command-record
   builder + MP broadcast), FUN_0040db9e (critter ranged
   attack on robots: 0x476fe4 0xC-stride weapon-param table
   + robot stun word 0xFFFF @0x4c69e4+idx·0xA8 +
   FUN_0040c536 timed effect), [0x4eb8b8+slot·4] census
   (objective-done flags: MissionShell + FUN_0044425c +
   FUN_00448b80 only). New open: exit consumer FUN_0041fbb1,
   pad trigger FUN_00433980, projectile 0x69 vs damage
   table.
2. ENGINE: tooling change (D66) — the inspector's heuristic
   NME walker replaced by the exact 8-section schedule
   (engine/bedlam-assets parse_nme + corpus exact-
   consumption test). No sim behavior change (critters/POIs
   still unwired; loader now anchored for P4.2).

Nudge-Worker: a840f0af-b732-44df-ae91-3caaa1de5960

## 2026-08-21 P4 7j.20 — the extraction beacon + pod-countdown producers
(worker c7269abe)

1. RE (docs-only, RE-EXW-SIM §7j.20): the extraction beacon armer
   FUN_004247b5 is called ONLY by the zone pad-trigger dispatcher
   FUN_00433980 @0x433cfb — the §6.4/§7c.8 "robot-sprite click
   family ~0x433cbc" attribution is REVOKED (0x433cbc lies inside
   FUN_00433980's body; the pre-7j.19 enclosing-function guess was
   never re-checked). ~25 (zone, .PAD slot) pairs are extraction
   pads. FUN_004248c8 = spread-claim picker (slots 9/10/11 =
   (−2,0)/(0,−2)/(+2,0); ≥12 leaves caller locals uninitialized).
   The w@robot+0x2C pod countdown: an SP producer EXISTS — the
   FUN_0040cca0 spawn stagger 1+k·(2000−m·1000/27) (m = linear
   mission) — refuting the "no SP producer known, always 0" gloss;
   MP respawn writes 0x28. +0x2C = drop-pod descent timer; the
   0x4e64c0 pod bank serves deploy + respawn + extraction.
   P4.2 consequence: the differential harness must inject a pad
   step-on (not a click) to arm extraction; mission-start frames
   must model the staggered pod landings (the first seconds of a
   mission are pod descent, robots frozen).
2. ENGINE: no change (D68) — the extraction family stays
   anchored-but-unwired.

Nudge-Worker: c7269abe-e8c7-4c92-9a9b-568763c70e8f

## 2026-08-21 P4 7j.22 — the FUN_00410823 weapon-anim machine
(worker 27e4f048)

1. RE (docs-only, RE-EXW-SIM §7j.22): FUN_00410823 = the
   WEAPON-ANIM/PROJECTILE TICK over the whole 400×0x36 bank
   0x4c71f4 (4 calls/frame, phase 0..3; artillery ticks phase-0
   only, actor hit-tests odd phases only). The 0x36 record
   layout CLOSED: target selector d@+6 (type 0x29: 0x1000-bit
   robot / 0x2000-bit TRT structure / plain critter idx), class
   d@+0x2A = launch delay (0x24/0x29) OR detonation cycle count
   (0xF/0x13), arc d@+0x2E = ballistic z-velocity with gravity
   −0x100/tick (heading byte for 0x29), trail link d@+0x32.
   Per-type machines: bullets 2..4 = 2-substep lookahead ray
   (commit 1 — anti-tunnel); type 5 shell + K3 trail; artillery
   9..0xB = scripted-burst family with DURATION dwords
   0x456c78+4·id (w9→2/w0xA→4/w0xB→7 frames) over 7 expanding-
   ring (Δy,Δx) lists @0x45687c.. via PTR[0x456bf0] + ttl-24
   index, 500-sentinel; the §7j.14 K0xC debris set {0xE,0xF,
   0x13,0x17,0x1A,0x1F} = the BALLISTIC bounce family (0xE
   mortar = bounce + 3-cell 5000-blast per contact + smoke
   trail ring bank 0x4e66b8 0x68-stride; 0x17 = 3-clone split;
   0xF/0x13 = ttl-cycle submunitions detonating as the §7j.13
   four-quadrant "weapon 0x1A" blast — those 4 sites
   re-anchored to the detonation path); 0x24 rocket (launch
   delay, straight, 400 dmg); 0x29 homing missile (robot/
   critter/structure target lock, ±0x40 heading-search terrain
   avoidance, ttl 201). The two actor hit-test front doors
   pinned: FUN_0041879d = critter lane (presence-grid
   prefilter → FUN_004190bc mode 2), FUN_0041874c = MP
   other-robot lane (FUN_00418fca mode 2) — CORRECTING the
   §7j.15 "FUN_004190bc = panel/preview" hypothesis: it is the
   critter hit/damage applier. RandA identity re-pinned at
   0x4116b5 (FUN_00402975).
2. ENGINE: no change (D70) — the weapon family stays
   anchored-but-unwired for P4.2 (the harness must inject
   orders to exercise it).

Nudge-Worker: 27e4f048-ad51-4479-a42a-54e91ec114c3

## 2026-08-21 P4 7j.25 — the destroy-tail effect-entry map is docs-only (D73); FUN_0041a225 is the first 0x4cf638 producer; the 160-B stride anomaly was a census slip

1. RE: the FUN_0041a894 destroy tail = TERRAIN RESTORE (template
   banks @type+0x46/+0x4A → TOT-mirror z-words + seen + DAT
   volume) then a FIVE-effect loop over the type-table entries
   @+0x16+8m: selector 1..9 → jump table 0x41a870; kinds
   1→k14+FUN_0041a225+5 splashes, 2/3/4/5→k18/k17/k16/k19
   single gibs at fixed sub-tile bearings, 6/7→k10 + the
   DEADMAN1/2 thud pair, 8→k14×25 water-level demolition
   shower, 9→k20 + 3×3 splash ring; payload words = tile
   offsets off the object record; delays ride the chain
   counter + entry index. The GER gate skips the whole tail
   for type-0xb objects (record still dies + triggers fire).
   FUN_0041a225 = the FIRST producer of the MISSIONVIEW §5d
   effects bank 0x4cf638 (80×0x1E, free-slot word@+0x18,
   jittered Q13 particles, ttl 6000+). The 7j.13
   "160-B stride at 0x4c69e4" anomaly CLOSED: 21·idx·8 =
   0xA8 canonical — a census arithmetic slip, no second array.
2. ENGINE: no change — the destroy path needs a destroyed
   destructible object; corpus gates destroy none. The effect
   families re-open with the P4.2 differential harness
   (injected weapon fire).

Nudge-Worker: 399aeff4-03bf-4c9c-8569-83f955528215

## 2026-08-21 P4 7j.26 — the §5d draw tails are docs-only (D74); FUN_00401e39 is the shared direct blit; SMOKER.BIN is the blast column bank

1. RE: both remaining MISSIONVIEW §5d consumer passes decoded.
   The effects loop (0x4cf638) draws DEBRIS.BIN images 0..23
   (group*8 + frame&7, counter++ in the draw) through the DIRECT
   blit FUN_00401e39 into the 640 backbuffer, sy base 0x100 with
   the second shake table 0x454518; the 7j.25 field map
   corrected: dword@+0x14 is the RISING vz (6000..12069, high
   word = the sprite group), u16@+0x1A = the spawn delay (ECX
   arg), and FUN_0041ec59(n) = a bounded-uniform
   RandB()/(0x8000/n−1) helper. The platform loop (0x4eb638)
   uses the ENQUEUE path: SMOKER.BIN frame 0 (mode 300) + the
   smoke column frame d@+0x10+1 in mode 0x12d (DARKPAL flush)
   at sy−0x20, tick FUN_004238af cycling 2..16 intro / 5..16
   loop. FUN_00401e39 itself decoded: same .BIN container as
   the flush codec but a plain 0/≠0 transparency flag, no
   palette modes, dest stride 0x280 — 8street's
   draw_IMG_in_buffer re-anchored to EXW. Bonus: the three
   DROPSHIP ring passes (banks 0x4e64c0 + 0x4e6610..0x4e66b8,
   img = group*0x23 + 7*row+col over 7×7 0x40-stride grids,
   bank [0x4edd64] = DROPSHIP.BIN) recorded for the pod-descent
   work; producers stay open.
2. ENGINE: no change — both passes consume records whose
   producers (deaths, destroy-tail, pod descent) sit off the
   corpus path. They re-open with the P4.2 differential harness.

Nudge-Worker: 7658328a-90b8-4b01-8412-4118fad91579


## 2026-08-22 P4 7j.27 — the DROPSHIP ring producers are docs-only (D75); +0x14 is the img-group selector; the ring grid is 7×5 not 7×7

1. RE: the pod-descent family writer census is COMPLETE against a
   full .text objdump. Ring banks 0x4e64c0 (12 pods) +
   0x4e6610 (dropship) + 0x4e662c (5 exits): resets =
   FUN_0040cca0 @0x40cd3d (memset 0x150, every mission spawn) +
   MissionShell 0x447a7e/0x447a8d; spawners FUN_0041fb4b(idx)
   (pods: alt 0x400, x/y = robot pos>>8, from the 7j.20 w@+0x2C
   countdown 0-hit in FUN_0040b9f6, msgs 9/10/0xB per player),
   FUN_0041faf0 (dropship: alt 0x200, beacon tile<<5),
   FUN_0041fa51 (exits, 7j.18); animator FUN_0041fbb1 (all three
   machines, per frame). The 7j.19 "+0x14 toggle" gloss is
   superseded: +0x14 is the DROPSHIP.BIN img-group selector the
   7j.26 consumer reads — toggles 0↔1 every tick during descent/
   landed (phases 1-2), ramps 2,3,4,5 then oscillates 4↔5 during
   departure (phase 3) with x −= group·4 and alt += (alt>>2)+1;
   all six 210-image groups reachable. Pod phase 2 lasts ONE tick
   and releases the robot (state 6, alive 1, payout
   100·w@+0x94+5000). New third writer: FUN_00412a98 @0x412b60
   resets an exit's dwell (+0x18) := 0 per POI rescue (multi-POI
   elevators). The no-extract latch 0x46aed4: boot-cleared in
   GameMain 0x41c408 (not per-mission) and ALSO gates the MP
   respawn (FUN_0040e230 @0x40e7a1). CORRECTION: the ring grid is
   7 columns × 5 rows of 0x40 tiles (0x23 = 35 = 7·5 = exactly
   one group), not 7×7; dropship sy subtracts the beacon z word
   0x4eabb8 (always 0 — the 7j.20 "dead store" has one no-op
   reader at 0x4070c0). The 0x4c71f4 pass head-decoded: the
   projectile mid-flight draw dispatch (type switch 0x404141 +
   the 0x4cc654 50×0x22 sibling, states 0x65..0x69 → jump table
   0x403908); per-type math queued.
2. ENGINE: no change — no pods deploy in the corpus gates. The
   family re-opens with the P4.2 harness, which must model the
   deploy→descend→release→depart machine (~41-frame pod descent,
   stagger 173..327 frames between pods).

Nudge-Worker: e635cb76-8052-487a-8ac7-ebc65f357947
## 2026-08-22 P4 7j.28 — the projectile mid-flight draw family is docs-only (D76); the five projectile banks named; 0x40427a is loop-next not a draw body

1. RE: the last undecoded consumer block of the FUN_00403938 render
   tail is CLOSED (worker ffec42cf, claim 1, commits 9a1d205+):
   the 400×0x36 dispatch (0x404141 primary + 0x404d27/0x404d08
   secondaries) maps types 5/9..0xB/0xE/0xF/0x13/0x17/0x1A/0x1F/
   0x24/0x29 to draw bodies in WEAPONS/SHRIKE/REAPER/SMOKE/GENERAL
   .BIN (banks [0x4eddbc]/[0x46af30]/[0x46af2c]/[0x46af34]/
   [0x4edd7c], named from the boot loader string block 0x45884e..
   0x4588c3 + corpus count-verified: SHRIKE/REAPER exactly 64
   direction frames, SMOKE exactly 4). CORRECTIONS: 0x40427a is
   the shared LOOP-NEXT (unlisted types are NOT drawn mid-flight —
   there is no "generic draw body"), and 0x17 draws as a damped
   variant (WEAPONS base 0x28), not the "3-clone split" (that is
   tick-side only). The mortar trail-ring consumer CLOSED (8 puffs
   @ 0x4e66b8+link·0x68+8+i·0xC, WEAPONS frames 0x10+(tick+i)&7,
   mode 0x12E, active/ring words unread). The homing reticle
   decode (target word d@+6 → robot/critter/FUN_004128ec classes,
   GENERAL.BIN frame tick/3+2). The 50×0x22 walk CLOSED (jump
   table 0x403908 read from file: 0x65/0x67/0x68 single WEAPONS
   strip sprites 0x3C/0x3C/0x38, 0x69 the per-level beam column
   strip 0x34 with +0xA re-used as the TOP z level and +0x1A as
   the bottom, 0x66 NOT drawn). FUN_0040798e call shape pinned
   (EAX sx, EBX bank, ECX dx, EDX sy, stack: dy, frame, z tiles,
   mode 0x12C/0x12D/0x12E) — the 7j.21 "sprite 0x12E" gloss was
   this mode arg.
2. ENGINE: no change — the corpus gates fire no weapons; the
   family re-opens with the P4.2 differential harness, which can
   watch the WEAPONS/SHRIKE/REAPER/SMOKE blit sequences directly.

Nudge-Worker: ffec42cf-326a-47ae-a396-c02215f5eeb8

## 2026-08-22 P4.2 — the differential-harness architecture is D77: EXD/DOSBox-X is the scripted-differential instrument, EXW stays the canon of record; dumps are runtime-only, fingerprints in git

1. CONTEXT: PLAN sec 6 P4.2 (budgeted ~2 weeks) requires the differential
   harness design doc before any harness code (queue item 1, 2026-08-22).
   Written as docs/DESIGN-DIFFHARNESS.md (worker 4d7b9a5b, claim 1,
   docs-only, no engine change).
2. THE DECISION (D77): three original-side channels + the engine —
   O1 = BEDLAM.EXD under the pinned DOSBox-X (D29 sandbox model) is the
   PRIMARY scripted differential oracle (PLAN names DOSBox-X debugger
   memory-watches; the sandbox is B2-proven on the same DOS4GW LE class;
   observation never patches the original binaries). O2 = BEDLAM.EXW under
   the pinned Wine prefix is the CANON TIEBREAK/spot-check channel (every
   RE'd address verbatim; host ptrace watcher, ticket W11). O3 =
   instrumented 8street = second comparator only, late (W10, test-only per
   PLAN sec 0/1). EXD is the instrument of observation; EXW remains the
   canon of record — every EXW<->EXD divergence the harness surfaces is
   classified `original-divergence`, lands in docs/DIVERGENCES.md, and is
   arbitrated by O2. The differ compares CANONICAL RECORDS (never raw
   guest bytes) in five modes (STRUCTURAL / T1-exact / T1-timing /
   T2-tolerant / T3-statistical) per the PLAN 0b budget — the harness is a
   divergence meter + structural-error catcher + regression tripwire, NOT
   an all-zone tick-parity gate.
3. INJECTION DISCIPLINE: no host-level synthetic input ever; the runner
   writes the game's own seams at the frame trigger — g_keystore
   0x4edc44 / cursor 0x4eddc4/8 / mouse 0x4dc6e4 (RE-EXW-INPUT), ORDER
   writes to the 0x4dd484/88/8c target + 0x46cc30/60 move words, COMMAND
   records at 0x4dd4a0 for weapon fire (the 7j.22 route — never raw
   input), and .PAD step-on orders for extraction arming (7j.20). Frame
   alignment = g_frame_count 0x46ae68 <-> engine tick; dump point = the
   MissionShell epilogue/present tail.
4. HYGIENE: dumps derive from original memory = asset-derived data: they
   live only under runtime/harness-out (git-ignored); git carries the
   watch registry, scenario scripts, and dump-chain FNV-1a-64 fingerprints
   only (the 6-P4.3 goldens policy).
5. BUILD ORDER: W1 EXD import + EXW->EXD address map (docs/RE-EXD-MAP.md)
   -> W2 watch registry (tools/diffharness/watches.toml, every row
   ledger-anchored with an anchor-resolution test) -> W3 dump schema ->
   W4 DOSBox-X runner -> W5 injector -> W6 engine canonical dump emitter
   (parity_harness --canonical) -> W7 differ -> W8 S1/S2 end-to-end ->
   W9 gates/CI -> W10 O3 -> W11 O2 ptrace channel -> W12 S3-S8 scenario
   depth. The open-hypothesis dispositions (pod stagger, debris 2k
   start-delay, blink-cursor-from-spawn, ring overlap = statically moot
   per 7j.10 + confirming read, mid-flight blits = T2 render-side, out of
   state-diff scope) are tabulated in the doc sec 8.

Nudge-Worker: 4d7b9a5b-55db-4c69-b440-862e2adc029a

## 2026-08-22 P4.2/W3 — the dump schema is D78: BDLD-tagged LE records, registry-order canonical, D28-style chain; the FNV util is mirrored not depended-on

Context: DESIGN-DIFFHARNESS §3/W3 — one versioned frame-record stream
shared by O1/O2/O3/E, pure Rust tools-side, the crate stays
zero-dependency (offline CI).

1. WIRE GRAMMAR (schema_ver 1, pinned in tools/diffharness/src/dump.rs
   module docs): Stream := Header Frame* Trailer; Header := "BDLD"
   u16:schema_ver u8:channel [u8;32]:build_sha256 u8-len scenario
   u16:pin_count pins; Frame := "BDLD" u64:frame_no u8:injection_applied
   u16:watch_count (id + u32:len + raw bytes)* u64:frame_digest;
   Trailer := "BDLT" u64:frame_count u64:chain_digest. All LE. Channel
   codes: 1=O1 EXD/DOSBox-X, 2=O2 EXW/Wine, 3=O3 8street, 4=E engine.
2. DIGEST + CHAIN: frame_digest = FNV-1a-64 over the frame's canonical
   bytes INCLUDING the leading "BDLD" tag (domain separation: a dump
   digest can never equal the engine's untagged StateHash of the same
   field bytes). chain_digest = the D28/parity_harness construction
   verbatim: incremental Fnv1a64 fed write_u64(frame_digest) per frame
   in order — so a dump chain and a scene-hash chain are comparable
   fingerprints. Decode recomputes and verifies EVERY frame digest, the
   frame count, and the chain; integrity violations are hard errors.
3. CANONICAL ORDER = the committed registry's file order. encode_dump
   canonicalizes (stable sort by registry index) and rejects ids not in
   watches.toml + duplicate ids per frame, so identical observed state
   encodes byte-identically on every channel (verified: frame digests
   identical across all four Channel values). frame_no must strictly
   increase (encode AND decode) — the counter never rewinds; gaps are
   runner/differ business.
4. HASH UTIL = MIRROR, not a dependency: bedlam-core pulls thiserror,
   which would break the zero-dep guarantee. The mirror
   (tools/diffharness/src/hash.rs) is pinned to the engine's PUBLIC
   expected outputs ("" -> 0xcbf29ce484222325, "a" -> 0xaf63dc4c8601ec8c,
   "foobar" -> 0x85944171f73967e8, the LE write_u32/u64 vectors) by
   tests/dump_schema.rs::engine_hash_vectors; either side drifting fails
   that test.
5. CONVENTIONS (no extra record types): TS static-after-load rows ride as
   one frame record at the mission-start frame; TI injection rows hold
   POST-injection values; T4 event payloads reuse the WatchRecord
   envelope with per-row payload grammar deferred to W5+; empty watch
   blobs are LEGAL (count-driven extents hit 0, e.g. empty projectile
   bank before first fire). Dump blobs stay asset-derived:
   runtime/harness-out only, fingerprints in git (D77 hygiene).

Nudge-Worker: 6f14cea1-e317-4016-8a1a-55054fed36f0

## 2026-08-22 P4.2/W4 — the DOSBox-X O1 channel audit is D79: the pinned flathub runtime has NO debugger and log-only JS; W4 lands channel-agnostic staging + the DBXCAP stitcher, live runs blocked on a channel re-pin

1. REVERSAL OF A D29 ASSUMPTION (recorded in RUNTIME.md "DH-G0 channel
   audit", all [verified] on the pinned binary via strings + reference
   conf + upstream source at the banner commit e522642 + three headless
   behavioral probes): the flathub DOSBox-X 2026.08.02 was built WITHOUT
   the integrated debugger (configure.ac --enable-debug default off;
   flathub passes only --enable-sdl2; debuggerrun=debugger and
   -break-start parse but are inert), and its Duktape startup.js engine,
   while present and running, exposes a LOG-ONLY API (_emu.emulator/
   version/log, console.log, Buffer/CBOR with no I/O) — no memory reads,
   no hooks. The D29 claim "the shipped binary carries the integrated
   debugger" misread a config help string + coincidental junk strings;
   corrected in RUNTIME.md.
2. GET-VISIBLE-BY-DEFAULT GOTCHA: LOG(LOG_MISC,*) (incl. all JS
   console.log) requires [log] misc = true or it is invisible in the
   logfile — pinned behaviorally, recorded in RUNTIME.md.
3. DH-G0 PRECONDITION CHANGES: before any live trigger automation, O1
   needs a CHANNEL RE-PIN. Options left OPEN (no decision made here):
   (a) self-build DOSBox-X at a pinned commit with --enable-debug=heavy
   inside runtime/ (D29 conf pins carry over; D19 deliberate-pin
   discipline applies), (b) GameLink GC4 IPC feasibility for DPMI
   linear reads (it is compiled in; address model unproven),
   (c) promote the O2 ptrace channel (W11) to primary. The choice is an
   operator/interactive decision; the queue item for it follows W4.
4. W4 DELIVERABLES LANDED (unattended-safe slice per the W4 ticket
   split clause): tools/runtime/dosbox-harness.sh gains the `diff` mode
   (stage/run/stitch); EXD corpus scratch at runtime/harness-corpus-exd
   (game-data/BEDLAM rsync; the B2 scratch stays untouched); scenario
   grammar v1 + S0/S1 scenario files under tools/diffharness/scenarios/;
   the zero-dep `dbx-stitch` bin consuming a channel-agnostic DBXCAP
   capture transcript -> W3 BDLD dump + JSON digest manifest (file
   sha256 + frame count + chain digest; dumps stay runtime/-only per
   D77 hygiene, only synthetic test vectors are committed). The live
   game automation stays [BLOCKED]-on-DH-G0-channel-repin and
   interactive-gated.

Nudge-Worker: d35c7066-4f7f-4c3a-a8b3-0afaead3049d

## 2026-08-22 P4.2/DH-G0 — the O1 capture channel re-pin is D80: repo-local self-built DOSBox-X at e522642 with --enable-debug=heavy; flathub pin stays as sandbox baseline; PTY automation is the driver shape

1. THE DECISION (queue item 1's stated DEFAULT absent operator input;
   options a/b/c were left open by D79): O1's capture channel = a
   self-built DOSBox-X at upstream commit e522642 (the flathub pin's own
   banner commit — same code, different configure flags), built
   out-of-tree under gitignored runtime/ with `--enable-sdl2
   --enable-debug=heavy` (C_DEBUG + C_HEAVY_DEBUG verified in config.h).
   The flathub runtime remains installed (D29 sandbox baseline + golden
   runs without the debugger); the self-build is the INSTRUMENT build.
   Host toolchain recorded as part of the pin (gcc 16.2.1, SDL2 2.32.70,
   ncursesw 6.6, autotools; RUNTIME.md "DH-G0 channel re-pin").
2. WHY NOT (b)/(c): GameLink is compiled in but client-poll oriented for
   real-mode games with an unproven DPMI/flat-linear model — adopting it
   buys a second research project instead of a channel; O2-ptrace abandons
   the DOS-side oracle and re-shuffles D77's topology. The self-build
   needs zero RE changes: the watch skeleton's full command surface
   (BP/BPINT/BPLM/RUNWATCH/MEMDUMPBIN/SMV/D-DV-DP) is source-pinned in
   this tree at e522642 (RUNTIME.md section, line refs).
3. KEY NEW FACT (source-pinned): the Linux debugger REFUSES to open unless
   isatty(0/1/2) — automation MUST drive the binary under a host PTY
   (debug.cpp:5042-5064). The D79 "inert -break-start" observation on
   flathub was the missing C_DEBUG, not this gate.
4. AUTOMATION SHAPE: PTY driver feeds ParseCommand lines (ncurses getch
   input), per frame-tail bp hit: N× MEMDUMPBIN (fixed name MEMDUMP.BIN,
   host CWD, overwrite per call → driver renames between reads) → slice
   per watch → DBXCAP transcript → existing W4 dbx-stitch. Injection (W5)
   rides SMV linear writes. Behaviorally verified at the unit's headless
   smoke probe; the live game diff stays interactive-gated.
5. PROBE VERDICTS (2026-08-22, headless, no game; tools/runtime/
   dbx-capgen.py + dosbox-harness.sh dbgprobe): channel GREEN —
   -break-start prompt over PTY, BPINT-8 hit surrogate across 3 frames,
   9/9 MEMDUMPBIN round-trips, RUNWATCH resume/hit cycles, DBXCAP
   transcript with real state deltas (pre-boot zeros → POST IVT/BDA →
   DOS-kernel vectors). SMV linear write + readback, BPLM linear
   memory-change bp arm + fire: VERIFIED. Three channel gotchas pinned
   and baked into the driver (RUNTIME.md "D80 CHANNEL GOTCHAS"): the
   [log] logfile is REWRITTEN at debugger init (acks must be count-
   matched over full reads, not seek-tailed), a permanent PTY drain is
   mandatory (ncurses redraws fill the ~64KB pty buffer → wrefresh
   deadlocks), and each command needs a ~1.0s post-ack settle
   (0.01s-gap input stalls tens of seconds). Configure correction vs
   the first commit: --disable-sdlnet --disable-avcodec (host lacks
   SDL2_net; ffmpeg 8 broke upstream's avcodec code — neither is used
   by the harness).

Nudge-Worker: 4deb0081-12f4-4fdd-a60e-36363119d216

## 2026-08-22 P4.2/DH-G0-live prep — the S0 capture-plan design is D81: CS-register addressing (no numeric selector), BPLM boot-trap → BP arm sequence, runtime cell resolution in capgen

1. THE SELECTOR QUESTION DISSOLVES (source-pinned, RUNTIME.md "S0 live
   channel mechanics" #1): GetHexValue resolves REGISTER NAMES in the
   default MEMDUMPBIN/BP parse path — `CS:001195F0` resolves through
   SegValue(cs) → cached base + offset at any in-game stop. The plan and
   the runner therefore carry NO selector parameter; the BP ack line
   echoes the numeric selector into the logfile as the per-run pin
   record. The queue's INT3-at-entry proof step is replaced by: SELINFO
   CS base==0 runtime guard + the BP ack echo (both automatic, both in
   the logfile).
2. BOOT-TRAP ORDER (RUNTIME.md #2): BP locations resolve EAGERLY at arm
   time (pre-boot arming mis-resolves); BPLM is LAZY (per-instruction
   linear compare). Live flow: `BPLM 1195F0` at the parked halt → first
   post-boot write stop (LeLoader copy / first screen-loop INC) →
   SELINFO flat-CS guard (retry loop for non-flat stops) → `BPDEL *` +
   `BP CS:0005A6EB` → per-frame RUNWATCH capture loop. Anchor frame =
   first BP hit = mission frame 2's dump point (the trap fires past
   frame 1's tail; alignment rides the frame-counter watch).
3. RESOLUTION AT CAPTURE TIME, NOT PLAN-BUILD TIME: capgen plan v2
   carries `resolve` rows (u32 cell reads at the arm stop) + arithmetic
   addr/len expressions over them ($map_w etc.) — the TOT/DAT volume
   extents (4+16·w·h / 4+8·w·h, FORMATS §2/§4) and the pointer-cell
   banks resolve from THE SAME session's memory (no cross-run pointer
   staleness; the DESIGN "plan-build time or capture time" question is
   answered: capture time).
4. STAGED-CONF CHANNEL FLIP: `diff stage` rewrites debuggerrun=watch →
   debugger in the STAGED runtime/ conf copy (watch mode free-runs past
   the parked halt; queued commands never execute). The canon conf and
   every D29 sim pin are untouched.
5. EXPECTATION SETTING (RUNTIME.md #5): the frame counter has NO reset
   (14 INC sites incl. menu screens) — interactive S0 double-runs are
   expected to differ ONLY in the frame-counter (+RNG-churn) watch
   bytes (T2/T3 classes per DESIGN §6). Byte-identical chains are the
   W5 scripted-walk property; the live gate records "identical modulo
   those cells" + the diff detail as its verdict form.

Nudge-Worker: fa49e9cf-487a-4005-8bba-83ac6e2b6776

## 2026-08-22 P4.2/W5 — the injector lands as grammar + SMV emitter + count-cell compiler, with the O1 alias gaps as hard gates (D82)

1. GRAMMAR v1.1 (shared seam): scenario files gain keystore/order/pad/
   command/boot steps (one frame boundary per line; `until-anchor`
   splits walk phase from mission phase; boot is walk-phase only).
   The ENGINE side consumes the same steps (W6) — one script drives
   both sides per DESIGN §5. Command payloads are RAW hex bytes
   (≤0x80) by design: the builder-side field packing is pinned
   (§7j.17) but the sugar lands with S3, not guessed now.
2. THE O1 WRITE PRIMITIVE IS SMV (linear), not SM: SMV is the
   D80-behaviorally-verified primitive; capgen converts the plan's
   SEG:EXPR addr forms — `CS:` = the flat-identity (base 0, boot-guard
   asserted, linear == the EXD offset, bounded to the image top),
   numeric segs = real-mode seg<<4 (probe form). Byte tokens only
   (never register names). The command-ring append is a capgen OP
   (read count u32 via MEMDUMPBIN on the plan's own SEG:OFF form,
   write payload zero-extended to the stride, bump count) — proven
   headless by `dbgprobe inject` (GREEN: write-then-readback ordering,
   count 0→1, `frame N 1` injected flags; gate + flow regression-green).
3. T1 COMPILES COUNT-DRIVEN (the W5 ticket list): robot-bank
   $robot_count·0xA8 (0x11958c), trt-array $trt_count·0x20 (0x11949c),
   object-instances *(0x119584) $obj_count·0x14 (0x119554) as resolve
   rows + len expressions; grids derive from the map w/h cells.
   S1.scen compiles (capture-plans/S1.json committed + byte-pinned).
   TWO anti-fabrication calls: selection-triple dumps only the 4
   verified alias bytes (the 12-byte triple would read the EXD
   cursor/squad gap cells), beacon-family dumps its five u16-spaced
   cells (10 B — the registry 0x18 extent gloss would read into
   spread-claims).
4. THE §5 SEAM ROWS STAY GAPS ON O1: keystore/order-target/command
   ring (+difficulty) have no EXD aliases yet (the EXD input twin is
   NOT FUN_0002ec12 — probe exd-input-probe.txt shows only the
   P-latch spin; RE-EXD-MAP W5 note). dbx-plan REFUSES any scenario
   carrying those steps, naming the seam — no unanchored address ever
   enters a plan. The compiler's alias-gated paths are proven by tests
   against a fabricated-address registry (never committed). Two new
   TI registry rows (inj-command-ring 0x4dd4a0 / inj-command-count
   0x46cbe0, anchored at the §7j.17 ledger row) formalize the seam.
5. WALK-PHASE INJECTION IS FUTURE WORK BY DESIGN: the scripted menu
   walk needs a per-frame walk driver (BPLM-on-frame-counter stops
   during menu screens + mission-start detection) — that is its own
   unit once the keystore alias lands; the interactive S0 session
   walks manually meanwhile.

Nudge-Worker: 683a65d6-c1ae-485b-9188-cd9413234442

## D83 — 2026-08-22: EXD input-twin census closes the four W5 seam gaps (keystore / order target / command ring + count / difficulty)

Worker ef11271c claim 2 (queue item 2). Four Ghidra `-process
BEDLAM.EXD -noanalysis` probe passes (EXDInputTwin{,2,3,4}.java).

1. THE FOUR SEAMS ARE PINNED (RE-EXD-MAP §4/§5/§5c, all dual+ anchored):
   keystore 0x894d4 (AnyKeyWait twin FUN_00030792 + the INT-9 hook
   KeySink with the OR-0x80 arrow remap + the installer memset),
   order target 0x10e0a4/a8/ac (+ order-active 0x10e140), command ring
   0x9255c stride 0x80 + count 0x119588 (builder FUN_0005b066 /
   consumer FUN_00019ee9 = the EXW 00449c94/00409138 twins; the EXD
   MissionShell trio position is EXACT), difficulty 0x119558 (the
   7j.17 172/236/300 formula + the respawn-delay table twin 0x81050 in
   the epilogue tick FUN_00023967).
2. REGISTRY + COMPILER LIFT: watches.toml rows filled (difficulty,
   order-target, inj-key-state, inj-remapped-keys derived, both
   inj-command rows); the TI exd-emptiness test rule narrows to T2-T4
   (TI rows are aliased where their seam is pinned); dbx-plan gains
   the order-target resolution form (3 contiguous u32 = 12-byte watch
   with a spacing anti-ghost assert); the step-compiler tests now run
   against the REAL registry (the fabricated-registry helper is
   replaced by a cleared-registry gap-prover). S0/S1 plans regenerated
   (difficulty + order-target leave _deferred, entering the watch
   sets); a scratch scenario proves keystore/order/command steps
   compile end-to-end to the anchored addresses (incl. the remapped
   arrow byte CS:0008959F = 0x894d4+0xcb).
3. DIVERGENCE SEEDS 6-7 (RE-EXD-MAP §7): EXD critter attack-break
   gates are frame-counter+timer masks (0x1f/0xf/0x7 per d=0/1/2) vs
   EXW RandA gates (1/8, 1/16, never) — different randomness source
   AND inverted mapping, a live T2/T3 diff class; EXD-only staging
   cells (order word 0x10e15c, command flags 0x11a51a, held-keys
   counter 0x107534) = watch-artifact class. BONUS (T2-ready, not
   registered): projectile bank 0x980d4 ×0x36 field-exact, ScanToChar
   tables 0x8077a/0x8097a.
4. NEXT HEAD: the scripted-menu-walk driver unit is unblocked (the
   keystore alias landed); the pad step still needs the capgen runtime
   pad-slot op.

Nudge-Worker: ef11271c-539d-4331-9689-ffc84b2848ee

## D84 — 2026-08-22: the scripted-menu-walk driver lands (W5-walk): stop-indexed walk rows, arm-at-walk-end, resolve_at=anchor

Worker 845abdc5 claim 2 (queue item 2; commits 59ec9a5 + b67dcaa).
Design first (RUNTIME.md "W5 walk driver"), implementation follows.

1. THE STOP MODEL (derived from pinned facts; RUNTIME.md section):
   the BPLM boot trap on the frame-counter cell 0x1195f0 doubles as
   the walk driver — one stop per counter-writing screen frame. A
   walk row applied at stop i (SMV between the INC and the next loop
   input read) becomes screen frame i+1's input. Keystore writes
   re-arm per input (the AnyKeyWait twin FUN_00030792 consumes the
   byte on read; polling menus need explicit 0 releases). The anchor
   BP CS:0005A6EB arms only AT THE LAST WALK STOP (BPDEL * drops the
   BPLM first) — mission-start detection with no stop-type ambiguity
   during the walk. Walk rows are plain writes only (command ops are
   mission-phase seams).
2. resolve_at=anchor FIXES A LATENT D81 GAP: the loader statics (map
   w/h 0x1074b8/0x10748c, TOT/DAT/claim pointer cells) are
   MISSION-load values; the legacy arm-stop read (first post-boot
   counter write = an early pre-mission screen) reads pre-mission
   bytes, so len exprs (4+16·w·h, ...) evaluated from garbage.
   dbx-plan now emits "resolve_at": "anchor" for ALL plans and capgen
   reads the resolve cells at the anchor stop (mission start) — S0/S1
   regenerated; legacy plans keep the arm position (dbgprobe flow/
   inject unchanged, regression-green).
3. GRAMMAR/COMPILER: walk-phase keystore steps compile to stop-indexed
   rows (boot at the accept stop, Advance consumes stops, runaway
   guard 1M); order/pad/command refused in walk phase ("not menu-walk
   steps"); the walk_watches calibration trio (walk-mode/zone/mission)
   is registry-derived (anti-ghost holds for calibration rows).
   S0W.scen + capture-plans/S0W.json committed — the walk schedule is
   a STRUCTURAL DRAFT; stop indices calibrate at the first live
   session via the transcript's per-stop `# walk stop N <id> <hex>`
   comments (then the schedule is pure data).
4. VERIFICATION: `dbgprobe walk` GREEN headless (probe conf, NO game;
   walk loop + stop indexing incl. a pure-skip stop + write-then-read
   calibration notes + arm-at-walk-end + the anchor-position resolve
   feeding expr lens); dbgprobe gate/flow/inject regression-GREEN;
   52 diffharness tests (4 new walk tests), workspace fmt+clippy
   clean. THE PAD STEP KEEPS ITS OWN UNIT: the capgen runtime
   pad-slot read op stays out of scope deliberately (needs the
   §7j.20 pad census semantics; S6 will pull it).
5. WHAT THIS MAKES POSSIBLE: S0/S1 captures unattended-reproducible
   with byte-identical chains (frame-counter/RNG menu churn becomes
   script-determined, not operator-timing-dependent) — DH-G1's
   headless form. Known limits recorded in RUNTIME.md (counter-silent
   screens are transparent; a schedule overrunning into the mission
   anchors mid-mission, detectable via watch values; queued SMV
   relies on acks as before).

Nudge-Worker: 845abdc5-ded1-4499-9286-85bbebcccfdc

## D85 — 2026-08-22: the W6 engine dump emitter lands (`--canonical`, channel E): the §6a canonical record grammar + the shared scenario seam

Context: DESIGN §10-W6 — the E side of the differ. parity_harness (the
bedlam-game example) gains `--canonical --scenario <path>`: drive GameHost
over a v1.1 scenario file and emit the per-tick W3 dump (channel E)
through the SAME validation/encode path as O1 captures
(`runner::stitch` + `encode_dump`; the E dump is byte-deterministic by
construction). Decisions:

1. CANONICAL RECORD GRAMMAR (DESIGN §6a, the W6 deliverable): the watch
   blobs are the CONTRACT — E writes engine state directly; W7's
   normalizer must convert O1/O2 raw bytes into the same grammar. T0/T1
   field maps committed (frame counter pre-increment at the tail, RNG
   A/B as channel-native state words, score/money/difficulty/zone/
   mission/mode/linear-m, the robot bank as the modeled Robot field
   list in the state_hash order, selection-triple 4-B alias form per
   the D83 precedent, blink-cursor, per-player-selected, order-target,
   move-target-words, beacon-family, spread-claims, the +0x18 byte
   family, static-map-wh). Every unmapped row is an explicit E-gap
   (listed in §6a) — missing-on-E is a STRUCTURAL finding, never
   silent.
2. FRAME MODEL: one record per `pump_frame(dt=4)` = tick + present
   (the render epilogue runs the RandB churn — dumps represent genuine
   engine frames); anchor = tail of the FIRST mission tick (frame_no
   0), then strictly increasing; total = anchor + `frames` (the
   stitcher contract). Audio not pulled (state-only, §0).
3. SHARED SEAM (D82): the emitter consumes the SAME `runner::Scenario`
   parser. Walk phase must be empty (the E menu-walk seam waits on the
   P2e button bit-map — S0W-shaped scenarios are rejected naming it);
   keystore maps to InputFrame via the pinned EMPTY map (no engine
   keyboard consumer yet; scan 0x19 P-pause rejected per §2); order =
   the click-order seam (target recorded + `arm_order_at_robot` at the
   tile-exact alive robot — the EXW 0x20-px screen pick is the
   documented approximation); boot difficulty seeds the campaign money
   via the engine's own `menu::start_score` formula; command/pad are
   REJECTED naming the missing engine seams (fire family / extraction
   arming — S3/S6 pair with their producers per W12).
4. PLACEMENT: the field maps live in
   engine/bedlam-game/examples/parity_harness/canonical.rs (shared by
   the example and the corpus-gated test via `#[path]`); diffharness is
   a bedlam-game DEV-dependency only (zero-dep workspace member — the
   engine production dependency graph is untouched). Three new
   read-only accessors (MissionSim::rand_a_state/armor_pads,
   MissionScene::rand_b_state); no engine behavior changed.
5. VERIFICATION: synthetic comparison fixture (hand-built TickState →
   hand-expected blob bytes + pinned digests — pins the §6a byte
   grammar), synthetic MissionSim run (dump decode + pinned chain), and
   corpus-gated S0/S1 runs (401/3 records, byte-identical double runs,
   pinned chain digests — re-baseline deliberately, the fingerprint
   discipline). Dumps stay runtime-only (§3 hygiene); git carries the
   chain digests in the test.

Nudge-Worker: 1f758667-a545-4efc-b1ca-975af330fcb1

COMPLETION ADDENDUM (2026-08-22, worker 36f752cd, claim 2, commit
54d781a): the implementation + verification landed as promised above,
with three recorded deltas from the design-first text:

1. WALK-PHASE CORRECTION: D85 item 3 said "walk phase must be empty" —
   that made the grammar's walk-phase-only BOOT steps (the difficulty
   seed) unreachable dead code. The landed rule: walk may carry ONLY
   `boot` steps; any other walk step still names the P2e InputFrame
   seam. DESIGN §6a amended to match. Verified by the seam-gate test
   (boot difficulty=2 → money 3000 via menu::start_score + the
   `difficulty=2` header pin).
2. E-STAGING NOTE: `run_canonical` stages the host-default marker set
   (no network-marker override — the mission_scene_gate staged (18,73,1)
   stand-in is NOT auto-staged), so ZONEA/MISSION1 is a SINGLE-robot
   squad on the E channel: the order armer's window-0 single-robot
   special case clears the order on the arming pump's own window tick
   (the ledger behavior). The order seam is proven instead by robot
   state-3 + the tile-origin snap in the robot bank; the SURVIVING
   order (window 0x197→0x196) is the synthetic two-robot fixture.
   W8's first full O1↔E diff must pin whether the original SP fills
   the 0x46cbe0 override (robot-count parity).
3. PINS: fixture frame digest b359f7d282db7cb8; synthetic-sim chain
   ea0bc53dc95ff0b2; S0 chain 8901789a88cf61fe (3 records); S1 chain
   1c4e7b4c9d9b0947 (401 records); both corpus runs byte-identical on
   double runs. static-map-wh E source = the TOT-header map size
   (25×75, w·h=1875 — the 30004/15004 cross-check is FILE bytes, not
   dims).

Nudge-Worker: 36f752cd-4fdc-4d2e-9926-3c672ff37ecf

## D86 — 2026-08-22: the capgen pad op lands (W5-pad): runtime pad-slot
read + order-target write, Step::Pad un-gated

Worker 85dedea3 claim 2 (queue item 2). Design first (DESIGN §5.4 OP
FORM + §7 census + RUNTIME "W5 pad op"), implementation follows.

1. THE OP (capgen `{op:"pad"}` inject form): the PAD step's target
   tile is READ from the pad bank AT CAPTURE TIME (bank+slot·8, 8-B
   record via MEMDUMPBIN through the bank's own SEG form), never baked
   from the .PAD file at compile time — the staged mission decides
   which slots exist. Validation FAILS LOUD: active u16@+0 == 1 (the
   7j.16 loader's parsed-slot mark) AND x u16@+2 != 0xFFFF (the file
   terminator); a scenario targeting a slot the mission never loaded
   is a capture error naming the slot, never a silent garbage order.
   The write: {x,y,z} → three i32-LE words to the order-target triple
   (EXD 0x10e0a4/a8/ac). Tile coords are the shared-grammar contract
   (the E order seam compares robot tiles; the beacon armer takes
   pos>>13). The op writes only the ORDER — the robot's arrival arms
   extraction in-game (FUN_00433980 → FUN_004247b5).
2. DBX-PLAN UN-GATE: `pad <slot>` compiles to the op row; the bank
   address comes from the `static-pad-slots` registry row and the
   three targets from `order-target` — every address registry-derived
   (anti-ghost). The bank is a READ anchor: its gap error is distinct
   from the step_rows WRITE-seam rule (which keeps covering
   order-target). Slot bound 0..998 re-checked (runner already pins
   it; the op double-checks).
3. THE CENSUS AS DATA: the §7j.20 item 2 ~25 extraction-pad (zone,
   slot) pairs are committed in DESIGN §7 as the S6 slot picker
   (zone 1 {8,0x10,0x12,0x18} … + the shared slot-6 tail); the op's
   runtime validation guards a wrong zone/slot pairing.
4. VERIFICATION: `dbgprobe pad` headless GREEN (probe conf, NO game:
   a seeded fake pad bank at 0000:0600 — slot 2 carries the real
   ZONEA/MISSION1.PAD record 0 (5,61,0) with active=1 — the op writes
   05000000 3D000000 00000000 to the triple at 0000:0620, frame 1
   carries the injected flag; the NEGATIVE plan targets slot 3
   (active=0) and capgen must exit non-zero naming the slot).
   dbgprobe gate/flow/inject/walk regression-GREEN; dbx-plan step
   tests cover the un-gated compile + both registry gap errors;
   capture-plans byte-pinned tests unchanged (S0/S1/S0W carry no pad
   steps). Workspace test/fmt/clippy green; manifest clean.
5. SCOPE NOTES: the E side still rejects pad steps naming the S6
   engine seam (extraction arming — W12 pairs it); S6 itself (the
   scenario + live capture) is a later unit that pulls this op.

Nudge-Worker: 85dedea3-2ef6-47c7-a088-03a058aba96f

## D87 — 2026-08-22: the W7 differ lands — normalizer + comparison modes + report/fingerprint manifest

1. RE BASIS: RE-EXD-MAP §8 (this unit) — the EXD robot-record field
   map provenance-tagged (x@+0/y@+4/z@+8/state@+0x0C/drop@+0x2C/
   stop@+0x74/hp@+0x78/alive@+0x7C; z@+0x08 pinned NEW via the
   per-player anchor writer's `d@(0xf6d3c+i)+0x20` read) + the
   coverage-gap census (the 26 unmapped canonical record fields are
   COVERAGE findings, never zero-filled-and-compared) + the seed-#1
   EXW-front discrepancy recorded OPEN (the O2 normalizer uses the
   RE-EXW-SIM §3 evidence table; W11's first live EXW capture
   arbitrates).
2. THE DIFFER (tools/diffharness/src/differ.rs + bin/dbx-diff):
   channel normalizers (E parses the §6a canonical grammar; O1
   converts raw guest bytes per the §8 map; O2 uses the EXW table
   where its row forms match); MODES — DoubleRun (O1 vs O1: the
   DH-G1 verdict instrument — identical modulo the frame-counter T2
   + rng T3 classes) and CrossChannel (per-field classes per DESIGN
   §6 with O2 ARBITRATION: O2 agrees with O1 → engine-bug (E the
   outlier), O2 agrees with E → original-divergence (engine keeps
   EXW, log to DIVERGENCES.md), no O2 → provisional engine-bug).
   T3 rows never bit-compare; the DRAW-COUNT check (state-change
   counts per side) is the statistical gate. Alignment = the record
   frame_no (NOT the frame-counter watch — the O1 counter never
   resets, menu frames included); a constant shift ≤8 is applied and
   reported as a T1-timing note (T2 budget), worse misalignment is
   structural.
3. CLASS POLICY (refines §6): a `coverage` bucket distinct from
   `structural` — row/field coverage asymmetry (the §6a E-gap list,
   the §8 normalizer gaps) is metered + reported, NEVER SILENT, but
   notes rather than fails the verdict (it changes only when coverage
   deliberately changes); structural VALUE mismatches (counts,
   statics bytes, injection schedule, draw counts) FAIL. T2 diffs
   within the quantum are counted suppressed; beyond it report-only.
   Verdicts: PASS / PASS-WITH-NOTES / FAIL. Report: meter,
   first-divergence {frame,row,field,both values,class}, event-timing
   table (mechanical change-frame census per row), both chains;
   manifest_json = the git-carried fingerprint (dumps stay runtime/).
4. VERIFIED: tests/differ.rs 15 gates (hand-built EXD fixtures — an
   independent transcription of §8; the W6 canonical literal re-used
   as the shared-field contract; modes, arbitration both ways,
   coverage 26-gap math, shift, determinism); corpus-gated
   differ_gate.rs — S0/S1 run_canonical dumps (pinned chains
   8901789a88cf61fe / 1c4e7b4c9d9b0947 re-asserted) × the INVERSE
   normalizer fabrication → cross PASS-WITH-NOTES with exactly
   2+26 coverage findings on S1 + the one T2 counter note + zero
   engine-bug/structural; double-run PASS modulo counter/RNG and
   FAIL on a money perturbation. CLI smoke-tested on the real S0 E
   dump. Workspace test/fmt/clippy green; manifest clean.
5. FOLLOW-UPS: the EXD robot back-half offsets (a bounded probe pins
   the remaining 26 fields when a live S1 needs them); the O2
   static-map-wh + EXW-front pins (W11); the S1 live session consumes
   dbx-diff for its DH-G1 verdict step.

Nudge-Worker: c594df62-4614-47f6-b32d-f96b7a04db19

## D88 — EXD robot-record back half pinned; drop_countdown rebound +0x2C→+0x80 (2026-08-22, W7-followup, worker 03be9318 claim 2)

1. PROBE: two Ghidra `-process BEDLAM.EXD -noanalysis` passes
   (tools/ghidra-scripts/EXDRobotBackhalf{,2}.java; dumps
   ghidra-project/exd-robot-backhalf{,2}.txt). Hop 1 = program-wide
   immediate census of the 0xf6d34..0xf6ddc robot-base family (every
   hypothesis offset has traffic; dominant forms `[i·0xA8 + const]`
   and `[i·0x15 · 8 + const]`) + the FUN_0001c7dc disasm/decompile.
   Hop 2 = decompiles of the writer family: FUN_0001ef61 (damage
   applier, EXW 0040e230 twin), FUN_0001d9cd (spawn initializer, EXW
   0040cca0 twin — variant/kind/facing/probe-seed/stat-switch),
   FUN_0001d274 (robot_move — dir/facing/anim), FUN_0001e440 (probes),
   FUN_00020dea (pad charge), FUN_000180a1 (portrait pass),
   FUN_0005961c (SP all-dead sweep), FUN_00020fd5 (order cooldowns).
2. PINNED (RE-EXD-MAP §8 table rewritten, per-field provenance):
   dir_byte +0x0E, facing +0x10, anim +0x12, variant +0x18,
   probe_z[8] +0x1A..+0x29, kind +0x2A, hit_flash +0x2E, armor +0x30
   (i16), alarm +0x34, shield +0x88, shield_charges +0x8C, battery
   +0x94, armor_pool +0x98, death_flag +0x9C, shield_boost +0xA0,
   alarm_ctr +0xA4 — every EXW §3/§7f/§7g offset coincides in EXD
   with the semantic twin EXACT (damage order, stat switch 0x2A/0x2B/
   0x2C ×200, hp ceiling b·100+5000, booster 10000/150, anim formula,
   RandA&3, facing cardinals). death_flag READER pinned (FUN_0005961c
   all-dead sweep) closing the 7g.6 note. Canonical coverage gaps 26
   → 3 (target_present/x/y only).
3. CORRECTION (D87 sec-8 table): canonical drop_countdown binds raw
   +0x80 (the phase-4/5 gate `phase<4 ∨ phase·32 < d` + decrement +
   reinforcement countdown — the ENGINE field exact semantics,
   mission.rs `phase < 4 || phase*32 < drop_countdown`), NOT +0x2C.
   +0x2C is the mission-start pod-DESCENT timer (stagger + freeze
   gate) which the engine does not model canonically. EXD_ROBOT_MAP +
   EXW_ROBOT_MAP rebound; the differ gate inverse fixture follows.
   Both EXW and EXD split the two cells identically (SIM §3).
4. NOTES: EXD decrements alarm_ctr (+0xA4) 1/phase-0-pass when
   nonzero — no EXW decay is documented (7g.1 evidence gap, divergence
   -seed candidate until a live S1 diff). The move-target extent
   formula PINNED (cap-bounded ≤ 12; the 0x60-B span at 0xf75ec
   covers x[12]+y[12]) — the deferred dbx-plan row can now be filled
   (follow-up: plan row + normalizer splice to take coverage 3 → 0).
   The W1 "(14,644 B)" size belonged to FUN_0001476d, not FUN_0001c7dc
   (2,712 B per-phase tick) — sec-1b corrected.

Nudge-Worker: 03be9318-237b-4c9c-aa78-83bc504a48ef

## D89 — 2026-08-22: the W8 robot-count override pin — original SP does NOT fill 0x46cbe0; robot-count parity across EXW/EXD/E (W8-prep, worker b0656949 claim 2)

1. THE QUESTION (D85/DESIGN §10-W8): E stages the host-default
   markers → ZONEA single-robot squad; if the ORIGINAL SP spawned the
   full squad via the 0x46cbe0 network-marker override, robot-count
   diffs would be a staging artifact, not findings.
2. THE PIN (docs-only; no new Ghidra run — the EXW disasm was already
   in ghidra-project/exw-text-objdump.txt, extracted verbatim to
   ghidra-project/exw-spawncount-asm.txt with the desync/realignment
   note; EXD twin = exd-robot-backhalf2.txt lines 414-540):
   - EXW FUN_0040cca0 @0x40cd4c..0x40ce23: per_player [0x46cbd8] :=
     zone rule (zone [0x4edd8c]: <3∨==7→1, ==3→2, else 3); total
     [0x46ccbc] := per_player; the override branch is gated on
     `[0x4edb88] != 0` @0x40cd8d (EXD twin: mode [0x1075d8] == 0
     branch) — network sessions only, where total := [0x46cbe0],
     per_player := 1, markers record[i]+0x2A := i. Instruction-for-
     instruction the EXD twin (cap 0x11950c := 0x119588, count
     0x11958c := 1).
   - SP staging source (RE-EXW-TITLEMENU §4 [verified]): "New Single
     Player Game" @0x43aaa3 sets 0x4edb88 := 0 AND 0x46cbe0 := 1; the
     MP lobby sets 0x4edb88 := 1 (Coop) / 2 (Head2Head).
3. ANSWER + CONSEQUENCES: the override never fires in SP — SP ZONEA
   banks ONE robot in EXW, EXD, and E alike. Robot-count parity
   holds; robot-count diffs in SP scenarios are a genuine finding
   class. NO E-side staging seam changes (the conditional deliverable
   is moot). CORRECTION recorded: EXW 0x46ccbc = TOTAL (EXD cap
   0x11950c twin), EXW 0x46cbd8 = PER-PLAYER (EXD 0x11958c twin) —
   RE-EXD-MAP §5 robot-bank row + RE-EXW-SIM §7c.7 corrected (in SP
   both equal the zone rule, so all prior SP evidence held; future MP
   scenarios must bound the bank dump by the CAP cell). Faithful
   quirk: the SP marker write hits record[12]+0x2A (stale MRK-copy
   counter, one past the bank) in BOTH twins; harmless (EXW: inside
   the 0x4c71c4 anchor bank, re-stamped by the same function's tail;
   EXD: dead gap) — no diff surface.

Nudge-Worker: b0656949-cebf-46d7-b08c-1bcdff462127

## D90 — 2026-08-22: W7-followup2 — the move-target plan row FILLED + the differ splice; robot coverage 3 → 0 (worker 3595c744 claim 2)

1. CONTEXT: D88 pinned the move-target extent (per-robot u32 ×2 by
   ABSOLUTE robot id over the cap cell 0x11950c, ≤ 12; the fixed 0x60-B
   span at EXD 0xf75ec covers x[12]+y[12]) but the plan row stayed
   deferred and the O1/O2 normalizer refused the row (UnpinnedForm);
   the canonical target trio was the last 3 of 34 robot leaves without
   an O1 source.
2. DECISION: (a) dbx-plan emits the row as one Span{CS:000F75EC,
   len 96} (the x/y array pair asserted 0x30 apart; extent 0x60 pinned
   in watches.toml — the old "u16 arrays" gloss superseded);
   capture-plans/S1.json regenerated (S0 untouched, T0/TS only).
   (b) The differ SPLICES: normalize_frame (O1/O2) pre-parses the span
   (x[i] u32 @+4i, y[i] u32 @+0x30+4i, present = x ≠ −1, absent
   canonicalizes to 0/0/0 mirroring the E §6a row) and folds
   target_present/x/y into the robot-bank row bounded by the SAME
   frame's robot-bank count; the span carries NO standalone raw row —
   the E move-target-words row deliberately stays an E-only ROW-level
   coverage note (no duplicate comparison of the same bytes). A lone
   span (no bank row), a short span, or > 12 robots is a loud error.
   UnpinnedForm removed (no deferred raw forms remain).
3. UNIT CHECK (the Q5 question): both sides are Q5 — EXD writers are
   `tile<<5` (spawn −1 fill, order consumer, beacon auto-order,
   arrive-clear) and the engine's `Robot::target` is
   `dest_tile·Q5_PER_TILE(0x20)` — raw i32 comparison, no shift.
4. VERIFIED: differ_gate S1 coverage 2+3 → 2 (blink-cursor +
   move-target-words rows E-only, ZERO robot field gaps; cross
   PASS-WITH-NOTES, double-run PASS modulo counter/RNG, FAIL on money
   perturbation; pinned chains 8901789a88cf61fe / 1c4e7b4c9d9b0947
   re-asserted); S0/S1 plan byte-pin tests green; 52 workspace suites
   green, fmt+clippy clean, manifest clean. NOTE: S1 is passive (no
   order) — a live present=1 target first compares when the S2 ORDER
   scenario lands; the present=1 splice path is covered meanwhile by
   the unit fixtures + the O1==E shared-field contract test.

Nudge-Worker: 3595c744-f77a-4b9e-993c-bba6c59b29fb

## D91 — 2026-08-22: W8-s2 — the S2 order scenario staging key `markers`; the E walk needs a second robot (worker 7faaeb53 claim 2)

1. CONTEXT (the staging question, settled BEFORE authoring S2): the
   queue's S2 item expects a present=1 move-target window, an arrival
   clear, and live beacon/claims transitions. In the verified
   click-order model (RE-EXW-SIM §5c / FUN_004247b5 = the beacon armer,
   the E `arm_order_at_robot`) the CLICKED robot only snaps to spread
   slot 0 — its own tile — and the walk is performed by OTHER alive
   robots inside the 6-tile order radius that consume the order in
   phases 0..3 (mission.rs robots_phase). D89 pins the SP squad at the
   zone rule with NO override (ZONEA banks exactly 1 robot on EXW,
   EXD, and E), so a single-robot scenario can never walk — the W6 SO
   seam gate's window-0 immediate-clear is the whole story there.
   mission_corpus_gate's scripted walk stages a second robot at
   (18,73) for exactly this reason ("staged marker (host seam)").
2. DECISION: grammar v1.2 adds a scenario-level header key
   `markers = x,y,z[; x,y,z ...]` — extra squad markers staged after
   the MRK robots (deterministic indices), bounded so MRK+markers ≤ 12
   (the bank cap cell discipline, D89's count-cell note). (a) E stages
   them through the EXISTING `load_mission(staged_markers)` host seam
   — the D89 SP staging RULE is untouched; markers are scenario-level
   additions (the same seam mission_corpus_gate drives via
   spawn_robot). (b) O1 has NO equivalent write (fabricating an 0xA8
   robot record + count bump would be ghost staging): dbx-plan
   compiles markers scenarios and records the markers in an explicit
   `_e_staging` plan field — the registry gap discipline applied to
   staging (named, never fabricated). The live O1 capture of a
   markers scenario banks the MRK squad only; its robot-count diff vs
   E is the recorded scenario seam, never an engine finding.
3. S2 SHAPE: markers = 18,73,1 (the corpus-gate walker);
   `order 21 73 1` at boundary 1 (the arm at the MRK robot's tile —
   the E pick form); frames=16. Expected values (the corpus-gate
   fixtures): with 2 alive the window is 0x197 (NOT the single-robot
   window-0 clear — the order survives the arming pump and decrements
   while the walker walks); the walker claims spread slot 1 =
   order tile + (1,0) = (22,73), arrives ≈7 frames in snapped one
   tile short at the (21,73) origin (west-approach ARRIVE_RADIUS
   0x1400 semantics, pos &= ~0x1FFF); the order clears when all
   alive robots are state-3 (flag → 0, claims → all 0); the
   walker's move-target stays present=1 after arrival (state-4
   arrival keeps the target).
4. VERIFIED (commits a9e6964 + 786c9fb): grammar v1.2 parse tests
   (triples, hex, arity, the 9-marker cap); the canonical S2 run —
   17 records, chain 809f4961b7757da4 pinned, byte-identical double
   run — with the full walk timeline asserted in
   canonical_dump_gate::corpus_s2_order_walk: frame 1 (the order
   step's pump) arms the beacon (flag 1, window 0x197−1 = 406 after
   the arming pump's decrement — the single-robot window-0 clear does
   NOT fire at 2 alive), claims slots 0+1, the clicked robot state-3
   snapped at its own tile origin with NO target, the walker state-4
   with present=1 target (22,73) Q5 + stop_dist 1,000,000; frames
   1..6 the walk (state 4, monotone tile crossings 18→19→20→21,
   beacon flag 1 throughout); frame 7 the arrival clear (state 4→3,
   pos snapped one tile SHORT of the slot target at the (21,73)
   origin — the west-approach ARRIVE_RADIUS 0x1400 semantics,
   exactly the mission_corpus_gate fixture — beacon flag 0 + claims
   all 0 once every alive robot is state-3; the walker KEEPS its
   move-target so present=1 persists to the last frame). The
   differ_gate S2 row: fabricated O1 through the inverse normalizer
   carries the present=1 span both ways (the D90 splice) — cross
   PASS-WITH-NOTES with exactly the 2 E-only row findings, ZERO
   robot field gaps; double-run PASS modulo counter/RNG, FAIL on
   money perturbation. dbx-plan compiles S2: the order step's
   3-cell order-target write at frame 1 + the `_e_staging` plan
   field naming the markers seam (a test pins that NO inject row
   touches the robot bank/count cells — staging is never fabricated
   on O1); capture-plans/S2.json committed + byte-pinned. Workspace
   52 suites green (565 tests), fmt+clippy clean, manifest clean
   around the corpus reads.
5. LIVE-SESSION NOTE: any future live S2 capture re-stages the plan
   the same way as S0/S1 (dbx-plan scenarios/S2.scen --out
   runtime/harness-out/diff/S2/capture-plan.json). The live O1 run
   banks the MRK squad ONLY (1 robot) — its robot-count diff vs E is
   the `_e_staging` scenario seam; the differ's live verdict for S2
   reads the robot-bank row findings against that seam, and the
   order→walk comparison on O1 waits on the live click-order path
   (the 0x10e0a4 triple write alone does not arm an order in the
   original; the W8 "seam approximation" note in DESIGN §6a stands
   until a live session refines it).

Nudge-Worker: 7faaeb53-0c41-43f2-abe2-1ae7228eace0

## D92 — 2026-08-22: W9 — the DH-G3 CI leg + the corpus-skip sweep; menu_gate fixed as the sweep's one finding (worker cd3ebd73 claim 2)

1. DECISION (a): the corpus-gated harness set is wired into CI as a
   NAMED workflow job (.github/workflows/ci.yml `diffharness`:
   `cargo test -p diffharness` + `cargo test -p bedlam-game --test
   canonical_dump_gate --test differ_gate`) instead of relying on
   the blanket `cargo test --workspace` matrix step. Rationale: the
   DH-G3 leg is now auditable by job name, and CI continuously
   proves the SKIP-CLEANLY property (any test touching game-data
   without a `corpus_present()` guard fails the job — this exact
   failure mode shipped undetected in menu_gate until the sweep).
   What CI proves vs the live session is written at DESIGN §9 DH-G3:
   CI proves compile + skip-cleanly + the corpus-FREE tests (the
   synthetic §6a fixture, dump schema, registry anchors, stitch
   replay, differ units); the pinned-chain corpus assertions
   (8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4) run
   wherever a corpus is present — dev/operator machines run the
   same commands; original-side O1/O2/O3 runs NEVER run in CI
   (pinned emulator, desktop-gated; unchanged per §9).
2. THE SWEEP (b): empirical, not grep-only — a fresh git clone to a
   scratch dir (a faithful CI-checkout sim: game-data is never
   committed) + `cargo test --workspace --no-fail-fast`. Result: 51
   targets ok, exactly ONE non-skipping corpus dependency —
   engine/bedlam-game/tests/menu_gate.rs, 3 of 5 tests
   (table_geometry_and_color_sets, start_hands_off_with_the_seed,
   sfx_audible) called `corpus_host()` whose `.expect("corpus
   present but LANGUAGE.ENG missing")` PANICS on the absent corpus
   instead of skipping — despite the file header claiming "Skips
   when the corpus is absent (CI)". All other corpus suites
   (assets corpus/font_gate/loading/smk_title/smk_corpus, core
   mission_corpus_gate, render mission_view_gate, game
   boot_attract/brief/music/title_playback/mission_scene/
   canonical_dump/differ) skip cleanly.
3. FIX (c): `menu_gate` gains `fn corpus_present()` (LANGUAGE.ENG =
   the first staged file, the suite's own marker) and the three
   unguarded tests return with the file's existing "corpus absent:
   skipping" eprintln — identical pattern to the two already-guarded
   tests; `corpus_host`'s expect messages stay as corrupt-corpus
   tripwires (only reachable when the corpus IS present now).
   Verified: the clone run goes 52/52 green; the corpus-PRESENT
   workspace run keeps all 5 menu_gate tests executing for real.
4. RECIPE (recorded in DESIGN §9 CI-wiring note): re-run the
   corpus-free clone test whenever a new corpus gate lands; the
   named CI job enforces it from now on.
5. VERIFIED: fmt/clippy clean; workspace test green with the corpus
   present; manifest clean around the corpus-touching runs.

Nudge-Worker: cd3ebd73-10b5-4032-a085-6a30022ce8ea


## D93 — 2026-08-22: P4/FORMATS — the ".MOFO loader" RETIRED; the loader-tag family CLOSED at four members (worker 0a08a5e1 claim 2)

1. ANSWERED (queue item 2, negative result with proof): there is NO
   .MOFO loader, file, or grammar. The premise traced to a misparse:
   0x457a4c "MOFO\0" is the dead tail of the fatal string "Buggered
   direction in MOFO" @0x457a3c — ZERO code references (full .text
   immediate scan of ghidra-project/exw-text-objdump.txt + the
   earlier empty Ghidra XREF block); ".MOFO" bytes absent from BOTH
   BEDLAM.EXW and BEDLAM.EXD; no *.MOFO file anywhere in game-data
   (corpus walked read-only, MANIFEST verified clean both sides).
2. The extension-tag family in that DGROUP block is exactly .NME
   @0x457a57 / .TRT @0x457a5c / .POS @0x457a64 / .BDG @0x457a69,
   one reference each at the four CLOSED loaders (0x41648c /
   0x4170c3 / 0x41a55d / 0x41a5d6 = §7j.18/§7j.15/§7j.25). The 7j.15
   gloss "section strings .MOFO/.NME/.TRT/.POS/.BDG" is corrected
   (erratum landed in §7j.15 text + §7j.29).
3. BONUS PIN (the string's sole consumer): FUN_00415490(idx) = the
   mode-9 SEEK per-step target-acquisition dispatcher — dword@+0x10
   is dual-purpose (wander heading 0..255 in the steer paths; the
   2-bit seek direction 0..3 in mode 9, seeded RandA()&3 at the
   0xB-dormant wake); `cmp 3; ja fatal` → "Buggered direction in
   MOFO" via the standard fatal idiom (fade-cancel 0x420100 + print
   0x44d2ac + FATAL EXIT 0x44d2da); 4-way table 0x415480 = four
   directional forward-acquisition probes vs the robot bank (tight
   −4..+0xF ahead on the walk axis, |Δ|<0x18 cross + z; case 3
   reads robot y RAW — faithful quirk); hit → target w@+0x7A,
   mode w@+0xC := 2 (RANGE-ATTACK), anim w@+0x56 := 0. A second
   table 0x412ef8 dispatches the four axis-steppers
   (0x417f2c/0x417fe8/0x4180c0/0x41813d = y−1/x+1/y+1/x−1).
4. Corpus-path verdict: docs-only; no engine change (the fatal is
   a crash path by construction; no corpus gate reaches mode 9
   with a corrupted direction). FORMATS §0.1 landed; 2 new ledger
   rows + 1 erratum in RE-EXW-SIM §7j.29 (commit 03e8c3b).

Nudge-Worker: 0a08a5e1-c1ab-431b-880b-094e6ba40017

## D94 — 2026-08-22: P4/RE — the SFX/GFX bank-name walk COMPLETE; 202 durable assignments, zero unnamed durable cells (worker 7972b334 claim 2)

1. METHOD: two independent extractors over the full .text objdump
   (a strict 3-instruction window matcher written this run +
   the interrupted 09:55 predecessor state machine — adopted as
   WIP, validated row-by-row; 17 window-widening additions all
   verified in-dump; ONE artifact row rejected: the BEEP5→0x46af0c
   "pair" is the staging-cell false positive of the loose machine)
   + DGROUP strings re-read from BEDLAM.EXW (PE DGROUP VA 0x454000
   = file 0x52600, section table verified). No Ghidra run; manifest
   clean before/after the corpus read.
2. HEADLINES: the SFX cells hold VOICE-BASE handles (FUN_0043a36e
   1-voice / FUN_0043a39c 4-voice registers staging via scratch
   cell 0x46af0c); FUN_0043a48e = the play/steal function (default
   vol 0x7f/pan 0x8000 at x=y=−1, else position→pan/vol vs the
   listener cells 0x4edde4/0x4edde8; steal by the priority/age
   arrays 0x4ee1c2/0x4ee2e2); speech is a 53-record {A,B} bank at
   0x4ee014 (8-B records, 95 files, pair slot-order FLIPS at
   SPCH16, 11 unpopulated +4 slots) and BYPASSES FUN_0043a48e
   (0x44c8c4 direct). Language G-variants share cells via the
   index gate 0x4eba1c==1 + the edition gate [0x4edd8c]>4 → the
   GRILLA family; palette cells are per-ROLE shared slots
   (0x4edbf8 = current-screen PAL ×6 names — never treat as a
   stable identity). MIDIGUN is registered TWICE (0x4edf60 +
   0x4edf70, the latter consumer-less).
3. All prior bank pins re-confirmed cell-exact (7j.25 DEADMAN,
   7j.17 fire banks, 7h.2 POWERUP, 7j.27 BEAMIN, 7j.26/7j.28 GFX
   banks); corrections: none. Deliverable = RE-EXW-SIM §7j.30 +
   2 ledger rows; raw dump ghidra-project/exw-banknames.txt
   (git-ignored evidence), generators /tmp/opencode/sfxwalk.py +
   sfxcensus.py (scratch). The sec-9 "Mission SFX tier" backlog
   item DATA prerequisite is now met; the tier itself stays
   unimplemented (presentation stays out of the hashed core).

Nudge-Worker: 7972b334-1999-4c48-82c5-ae0582b17a0d

## D95 — 2026-08-22: P4/RE — the hot-rect click-target record CLOSED: one 0x20-stride array, 7 writer sites in FUN_00403938, octile picker + class dispatcher; SP click-orders are never robot-targeted (worker aa62f5ed claim 2)

1. THE HYPOTHESIS IS CONFIRMED with one refinement: the
   0x4787c4/0x47879c family is ONE 0x20-stride record array —
   base 0x4787bc (record 0), count [0x46ccd8], cap 0x77, extent
   ..0x47969c, per-frame reset @0x403a9a inside the renderer
   FUN_00403938. 0x47879c = base−0x20 = the ORDER DISPATCHER's
   (FUN_00410644, MissionShell @0x448021) 1-based view; the
   picker (FUN_00419943) and all writers use 0-based
   0x4787c4-family bases. Fields: +0/+4 world corner, +8/+0xC
   hit-box ORIGIN (refinement: NOT "center" — the picker adds
   w/2,h/2), +0x10 z, +0x14 w, +0x18 h, +0x1C type. Grammar +
   full traffic census (7 writers, 1 picker, 1 dispatcher,
   nothing else) in RE-EXW-SIM §7j.31 + 3 ledger rows.
2. TYPE WORD: bits 0..11 = 1-based bank id; 0x1000 = robot
   (w1 @0x403c87, gated [0x4edb88]==2 ∧ robot ≠ local player —
   MP-only, so SP writes NO robot rects); plain id = critter
   (w2-w7, the .NME bank draw paths); 0x2000|id = TRT structure
   — NEVER a stored record value, only the picker's ground-scan
   RETURN (or ah,0x20 @0x419af0 after the −0x10/+0x40 world-box
   test over active TRT recs), resolved by the dispatcher via
   the TRT −0xC-bias base 0x4cccec (fields +0x14/+0x18/+0x1C of
   rec(id−1), ×0x20+0x10). The 7j.28 ledger gloss "critter
   0x4cccec/0x20" is CORRECTED: the bank is TRT ("0x2000" is
   the projectile target-descriptor class name). Picker priority
   = FUN_0041ebf8 OCTILE distance max(|dx|,|dy|)+min/2,
   early-out <4.
3. SEAM CONSEQUENCES (the P4.2 payoff): (a) SP click-orders can
   never be robot-targeted — the E click seam must not fabricate
   them (S2's ground-order seam validated); (b) order-target
   units are per-class (robot/critter tile ints with +0xB/+0x15/
   +0x21/+0x20/+0x10 biases; TRT field·32+16) — the E order seam
   reproduces the formulas against the D82 cells 0x4dd484/88/8c;
   (c) NEW pins for the watch set when click parity is needed:
   type cell [0x46cc00], order latch [0x4ddb20]&2 (EXW cell of
   the D83 EXD order-active family), count [0x46ccd8].
   Docs-only; no engine change; objdump-only (no Ghidra run).
   Next queued: the operator S0 live session remains item 1
   (interactive); this closes the hot-rect unit.

Nudge-Worker: aa62f5ed-46c8-4b65-98f4-7b0bbc54929e

## D96 — 2026-08-22: P4/RE — the .BDG template-bank readers CLOSED: +0x46/+0x4A are the only consumed banks (the UNDER pair); +0x3E/+0x42 = DEAD EDITOR PAYLOAD (the CURRENT pair ≡ shipped TOT/DAT); the 0x1E-B mirror-record grammar unified (worker ce347a0e claim 2)

1. THE QUESTION (7j.25 open): which bank feeds which restore
   word, and who reads slots +0x3E/+0x42. ANSWER, three-legged:
   (a) loader bank DISK ORDER pinned — the four file banks land
   in slots +0x3E, +0x46, +0x42, +0x4A IN THAT ORDER
   (interleaved vs the slot layout; 0x41a71d..0x41a782);
   (b) reader census over the full .text objdump (slot absolute
   addresses + displacement forms + arena walk) — +0x3E/+0x42
   have ZERO readers; +0x46 feeds the TOT-mirror plane words,
   +0x4A feeds seen=(word==0) + DAT volume=low byte (restore
   re-verified instruction-exact: linear (z'·H+i)·W+j, z ∈
   [z0, min(z0+D,8)), mirror word +2·z, seen +0x10+z);
   (c) corpus proof of ROLE: bank1(+0x3E) ≡ shipped TOT word and
   bank3(+0x42) ≡ shipped DAT byte at every .POS footprint
   (434/435 ZONEA/M1 cells; the single miss = a genuine footprint
   overlap, last-.POS-slot-wins) — the CURRENT-state pair is
   already baked into the shipped mission files, so the game
   never re-stamps; the "runtime spawn-stamp pass" hypothesis is
   RETIRED. E-side: no seam change (terrain load already carries
   buildings; the destroy restore needs only the UNDER pair).
2. BONUS DECODES: (a) the TOT-mirror tile record grammar
   (0x1E B @0x4796bc: plane words +2·z, seen bytes +0x10+z,
   +0x18 scorch, +0x19/+0x1A variant/door, +0x1B/+0x1C the
   OBJECT-HEIGHT pair, +0x1D unused) — unifies the scattered
   MISSIONVIEW §8 tail-byte families; (b) FUN_0044889a /
   FUN_00448b80 = the OBJECTIVE-BUILDING family (zone==7 gate;
   counter [0x46cce0] over types 0x44..0x47; at zero → SFX
   0x28/0x29 + extraction-arm cells 0x46cd00/0x46ccfc/0x46ccc4);
   (c) .POS word 2 = the BASE Z LEVEL (FORMATS §12 "kind" gloss
   corrected); (d) FUN_0041bc1c's TRT death stamp (per-zone
   rubble word 0x454a04, FORMATS §14 family).
   Docs-only; no engine change; objdump-only + read-only corpus
   probes (scratch /tmp/opencode). Next queued: the .BLD record
   walk (item 3).

Nudge-Worker: ce347a0e-c2b8-4a25-9960-72473bedb8a8

## D97 — 2026-08-22: P4/FORMATS — the .BLD record walk CLOSED as an EDITOR-ONLY format: zero runtime readers in every shipped executable; BLD = the editor SOURCE that compiles to .BDG; the runtime file-family census lands (worker fc88ecf3 claim 2)

1. NEGATIVE RESULT (definitive, byte census): the sequence
   "BLD" (case-insensitive) occurs ZERO times in BEDLAM.EXW,
   BEDLAM.EXD, BEDLAM.EXE, the cd-root copies, and the three
   DIRECTX exes. There is NO .BLD loader; the queue-note
   hypothesis "loader near the .NME/.POS/.BDG family" is
   RETIRED. The only `.BDL` string in EXW = "SAVED.BDL"
   @0x4597d6 — the savegame. Editor-only set (six extensions
   with zero runtime references): .BLD .CTG .COL .MAP .PTH
   .TXT (FORMATS §0.2); notably .CTG is never loaded (the
   language gate picks .LNK or .LNG, FUN_0041dc5a,
   `cmp [0x4eba1c],1`).
2. RUNTIME CENSUS: the mission family loader FUN_0041dc5a
   loads .TOT/.DAT/.CGR/.BIN/.MIN/.LNG-or-.LNK/.PAD from the
   8-entry 5-B-stride tag table @0x4587d9..0x4587fc, against
   paths built by FUN_0044670c ("EDITOR\"+"ZONE"+zone+
   "\MISSION"+n); second .TOT/.BIN/.DAT site 0x446623..0x446677.
   RE-EXW-SIM §7j.33 + 2 ledger rows.
3. BLD GRAMMAR (corpus-anchored, FORMATS §17 rewritten):
   record length = 137 + 64·W·H + variable tail (subsumes the
   old "201+64k" rule — the "extension blocks" are four
   template-bank slots of 16·W·H B whose values ARE the BDG
   banks); head u32s = H/hp/chain/type == the BDG record's;
   name@+0x60; record j ≡ BDG NON-EMPTY record j (BLD 197 =
   BDG 282 − 85 EMPTY, ZONEA/M1); no terminator, no count —
   NOT self-delimiting (parse needs the sibling BDG's W,H);
   zero fill after the last record; 12-B header with the 5th
   u16 = 1/3/5 by zone [open]. Verified 7 286/7 907 records
   (ZONEA/C/D/E + ZONEF M2/M4/M7 fully; ZONEB/G + ZONEF M6
   desync at a few variable-tail records — bounded, §7j.33).
   BONUS: zone D DOES ship mission-level BLDs (FORMATS §0 row
   corrected); zone-level BLDs byte-shared A≡F and B≡G.
4. E-side seam: NONE (the engine never needs a BLD path).
   Docs-only; no engine change; objdump-only + read-only corpus
   probes (scratch /tmp/opencode); manifest verified before and
   after. Next queued: the MISSIONVIEW §8 mirror producers
   (the pickup tile-word producer unblock).

Nudge-Worker: fc88ecf3-fd23-459f-99f6-8b9811141b66

## D98 — 2026-08-22: P4/RE — the MISSIONVIEW §8 type-DB tail census CLOSED: +0x19/+0x1A = the sliding-door animation machine (FUN_00423081 epilogue tick); the 0x4dcae8 rect grammar resolved (7j.21 w/y/h permutation retired); +0x1D padding confirmed (worker a42c6027 claim 2)

1. THE QUESTION (queue item 2 / NEXT item 3): the +0x1a/+0x1b/
   +0x1c tail-byte producers of the 0x4796bc mirror rows, plus
   the §7j.12-vs-§7j.32 door-byte re-verification. METHOD:
   absolute census of 0x4796d4..0x4796d9 over the full .text
   objdump (71 sites — every access in this family is absolute
   [reg+0x4796dX]/[idx*2+0x4796dX], no displacement aliases),
   then bounded decodes of the seven container functions; no
   Ghidra run, no corpus read.
2. RE-VERIFICATION VERDICT: the §7j.12 stamper VALUES were
   right but TWO field citations were wrong — the "type ≥ 3"
   qualifier reads word@+0 (STATE), not word@+2; and §7j.21's
   restatement of the rect grammar had w/y/h permuted. The
   resolved 0x4dcae8 45×0x10 record: {+0 state, +2 x0, +4 y0,
   +6 w, +8 h, +0xA variant byte, +0xC countdown, +0xE SFX-due}.
   State domain: 0 = end-of-list; 1/2 = SCRIPTED doors (pad-
   script toggled); ≥3 = AUTO-CYCLING doors (timed).
3. HEADLINE DECODE: FUN_00423081 (sole caller MissionShell
   epilogue @0x44808f, after the platform-creep tick) = the
   DOOR ANIMATOR. +0x19 = the door's TARGET-TAG byte
   (variant<<4); +0x1A = {bit7 phase, bits0-6 running frame
   counter}; per tick each unfinished door tile writes a
   DAT-volume door-frame byte (0x40+2·nibble even/closing,
   0x5F−2·nibble odd/opening — a new documented DAT byte
   domain) at its walk-down stack level and increments the
   counter; every 16 frames the FINISH pair runs (FUN_004236c6/
   00423740 close: DAT seen 1/0 + z-stack PUSH-UP;
   FUN_00423650/004235fb open: DAT 0 + z-stack DROP — the
   door's level enters/leaves the tile stack); the counter
   stops at low7 == +0x19; auto doors then XOR bit7, re-target,
   pause 0x14 ticks, and cycle forever (SFX ELEV1/ELEV2 banks
   0x4edfb0/0x4edfb4). The renderer draws mid-anim door tiles
   with a −nibble·0x500 Y-bias (0x406c5c) — the door slide.
   The 0x4237c5/0x4237da mystery readers = FUN_00423740's
   south+east neighbor door-tile test before clearing plane 0.
4. READER ANCHORS completed: 0x40bc60 (FUN_0040b9f6) — scorch
   (+0x18) under a state-1 robot deals fire damage
   (FUN_004100b7(robot,0x14)) vs the pod-countdown path: the
   scorch→damage leg closes; 0x4110cb (FUN_00410823) — the
   fire controller's door-tile reposition anchor (robot anim
   word 0x4c720e); the +0x1B/+0x1C second stamp/clear walks in
   FUN_0044889a (0x448b4f/61) and FUN_00448b80 (0x448d65/6c);
   +0x1D = zero traffic (padding — the 7j.32 "[open]" closes).
5. Engine seam: NONE (ZONEA/M1 ships zero active door rects —
   the stamper walk terminates immediately; nothing animates in
   the gates; never-invent). P4.2 hooks recorded in §7j.34
   item 10 (a door scenario = a scripted .PAD step-on through
   FUN_00433980; watch surface = +0x19/+0x1A via the 0x4796bc
   row + the DAT door-frame byte class).
6. Deliverables: RE-EXW-SIM §7j.34 + 2 rewritten + 1 new
   ledger row; MISSIONVIEW §2 update + §8.1 CLOSED; FORMATS §2
   mirror-grammar cross-ref refreshed. This UNBLOCKS the 7h.3
   pickup tile-word producer (the mirror-row semantics are now
   fully enumerated).

Nudge-Worker: a42c6027-6acd-491d-b4a7-47ae4b4ae69f

## D99 — 2026-08-22: P4/RE — the 7h.3 pickup tile-word producer CLOSED: staging = init_tiles (ALL nonzero TOT words mirror-staged, DAT gates only SEEN), the terrain set = zone+1 confirmed, ZONEA/M1 stages ZERO pickup cells (the harness path never fires; ZONEB 601 / ZONEF 149 do) (worker f461ea05 claim 2)

1. THE STAGING PRODUCER (the piece 7h.3 hunted): init_tiles@00407e11
   (0x407fb0..0x407ff8) copies EVERY nonzero TOT plane word into the
   0x4796bc mirror at mission load — the DAT byte gates ONLY the seen
   flag (byte @+0x10+z := 1 when DAT[z]==0). The §2/§7j.16 "word needs
   DAT==0" gloss is CORRECTED: that gate is the seen flag + the
   FUN_00440a2d incremental restamp path, not word staging — FORMAT
   docs §2 amended. Pickup words ride the ordinary TOT volume.
2. THE PROBE LATCH (get_z_pos): {z→0x4dc688, x→0x4dc68c, y→0x4dc690}
   written at FOUR sites (the z / z+1 / z−2 empty-search probes + the
   slope-continuity z+1 probe when the CGR byte == 0x1F), each gated
   on the probed DAT plane byte == 3, last-write-wins, no auto-clear.
   THE CONSUME PROTOCOL (sole consumer): the robots()
   move-toward-target block clears [0x4dc688] := −1 (0x40bef2) →
   robot_move (0x40bf06; the 8 footprint probes ±11/±12 Q5 + center
   set the latch on type-3 cells) → test ≠ −1 (0x40bf0b) → mirror-word
   range test per the set tables → DAT byte := 0 + mirror word :=
   floor word [0x454a90+4·set] + seen := 1 + {x,y,z} staged at
   0x4dc6ac/b0/b4 (MP-only FUN_00425647 tails of cases 8/1) →
   FUN_0040eba0 dispatch. The wander-family robot_move call (0x40dc0e)
   has no clear/test — a robot collects a pickup when ANY of the 9
   probes of one move sub-tick touches the cell (±0.34..0.38 tile
   reach; no standing-on required).
3. THE TERRAIN SET [0x4edd8c] = zone_index+1 — the 7h hypothesis
   CONFIRMED and sharpened: the mission path builder writes the zone
   letter as 'A'+set−1 (0x446771/0x446879/0x4468d2). Writer census:
   GameMain boot := 1; campaign episode advance set++ (0x41c9e5, the
   7-episode loop = zones A..G in order); save-load restore (0x43c2b8,
   movsx from the 0xB4-stride save record); the network episode
   advance (0x43f34b); the MP mission picker rows 1..10 → sets 2..6
   (0x43edcb..0x43ee3d, MP-ONLY — the SP branch 0x43ee48 never writes
   the set). SP fresh ZONEA runs set 1.
4. THE CORPUS VERDICT (read-only probes, /tmp/opencode): a pickup
   cell = DAT byte 3 ∧ TOT word in the set range. ZONEA/M1: 80
   type-3 cells, ZERO in set-1 range (their words 0x81..0x84/0x131/
   0x230-family/0x28D/0x53D are set-2/5 shapes or non-pickup trigger
   scenery — inert under set 1). Campaign-set census: ZONEB 601
   pickup cells (M1 152 … M7 18; cases 1/2/3/4), ZONEF 149, zones
   C/D/E/G none. S0/S1/S2 never fire the machinery.
5. Engine seam: NONE (the corpus path does not fire; never-invent —
   the D98 pattern). P4.2 HOOKS: S5's pickup leg must run ZONEB or
   ZONEF (DESIGN §7 S5 row updated) and needs the E-side producer
   first: TOT words beside the DAT planes in Terrain + set = zone+1 +
   the probe-latch/clear→move→test protocol + the consume writes +
   the apply_pickup dispatch; until then O1 captures on those zones
   diverge by construction (structural rows, never findings).
6. Deliverables: RE-EXW-SIM §7h.4 + the §7h seam note superseded +
   1 rewritten + 2 new ledger rows + §9 item 4 refresh; FORMATS §2
   staging correction + §4 type-3 substrate note; DESIGN-DIFFHARNESS
   S5 row. registry_anchors green; manifest clean before AND after
   the corpus probes; docs-only, no engine pins touched.

Nudge-Worker: f461ea05-7a4d-4663-8955-eab1766f74a4

## D100 — 2026-08-22: P4/RE — the MISSIONVIEW §8 water-flag/anim remainder CLOSED: u32[0x456ca8] = a STATIC ping-pong const {0..7,7..0} (no runtime producer; the static branch = the scorch byte as the PALTRAN ramp index); [0x4edbd4] ≡ 1 in every mission (water-off paths dead); ZONEA/M1 stages ZERO water (worker 57ba8753 claim 2)

1. THE ANIM SEQUENCE "PRODUCER" IS THE FILE IMAGE: full-.text census
   — u32[0x456ca8] has exactly two sites (readers 0x40691a/0x406a2c
   in the terrain loop) and ZERO writers; DGROUP (0x552a8 in the PE)
   carries {0,1,2,3,4,5,6,7, 7,6,5,4,3,2,1,0} — a 16-phase
   ping-pong over the 8 PALTRAN ramps (0x4dd444, slot 0 NULL =
   plain). The STATIC branch indexes the same ramps by the +0x18
   SCORCH byte (scorch n → ramp n — scorch darkening IS ramp
   selection); anim-window (+0x1b/+0x1c) tiles are ZONEG-only.
2. THE WATER FLAG IS A CONSTANT IN SHIPPED PLAY: [0x4edbd4]'s only
   persistent writer is the campaign-boot defaults FUN_004252c0
   @0x4252d8 (:= 1, every "New Single Player Game"); the one other
   write pair (0x41c649/0x41c65a) is a scoped save/restore around
   the SELECTOR screen FUN_0043e7d4 (esi = the mission-index reg =
   1 on both paths). No CONFIG/options/save/MP writer exists — the
   0x12d/0x12e/0x12f "water-off" plain-copy arms and the remap-XLAT
   gate's flag==0 arm are dead code in every shipped session; E may
   hard-code water-ON.
3. 7j.12 OFF-BY-ONE CORRECTED: all zone tables (0x454a20/0x454a3c/
   0x454aac/0x454ae4 — one contiguous u32 array at 0x1C strides) are
   indexed by the RAW set [0x4edd8c] 1..7; entry 0 is the previous
   array's tail. Set-indexed bases landed in the §7j.35 ledger rows.
4. CORPUS VERDICT: water sprite words stage ONLY in ZONEB/M1 (12),
   ZONEB/M6 (78), ZONEC/M4 (33), ZONED/M1 (1), ZONEF/M7 (4824);
   ZONEA/M1 ZERO in both the sprite range and the platform/splash
   word range (which appears in NO shipped file — runtime-only).
   Side finding: 44 0x7d2 hazard cells in ZONEA/M1 → the load
   stamper pre-stages the 0x460dfa hazard grid in every gate run
   (the 7g.5 robots() hazard path is live).
5. ENGINE: docs-only (D98/D99 pattern — the corpus path does not
   fire for the gates); DrawParams.remap stays the host seam
   (pixel-side selection, out of the 0b state budget). P4.2 hooks
   on the DESIGN §7 S5 row: a water leg shares the S5 zone-walk
   staging, must run ZONEB/M1|M6, ZONEC/M4 or ZONEF/M7. Deliverables:
   RE-EXW-SIM §7j.35 + 2 new + 1 rewritten ledger rows; MISSIONVIEW
   §8.2 closure + §1/§3/§4/§5c refreshes; DESIGN S5 row.

Nudge-Worker: 57ba8753-c3b0-471f-b960-5c67704d0b41

## D101 — 2026-08-22: P4/RE — the [0x4ede1c] BIN-bank content consumers CLOSED: container grammar pinned (u16 count + SELF-relative u32 directory at +2, 11/11 banks); every content reader is a pixel blit; the 9-sprite radar stamp is the bank's only runtime writer and its output is NEVER drawn (vestigial) — the bank is render-only presentation, NO differ watch row (worker d6b238f4 claim 2)

1. CONTAINER GRAMMAR (the unit's grammar deliverable): u16[bank+0]
   = sprite COUNT (989..1872; → 0x46cdb8 — a WRITE-ONLY cell, no
   .text reader; blits mask id&0xFFF instead); directory entry =
   bank+2+4·id, sprite = entry + u32[entry] SELF-relative
   (monotone, in-file, all 11 shipped banks incl. the B6/D5/E6
   mission-extras; last record runs to EOF). Records = u16
   fmt/dy/dx/gate/rows + stream; FUN_00401471 (0x401477..0x4014c8):
   fmt≥4 u8-RLE, 1..3 u16-RLE, 0 raw, gate==0 → RETURN, rows==0 →
   RETURN; FUN_0040167a reads gate but IGNORES it; FUN_0040179b =
   +2 head, gate skipped. ALL real terrain sprites are fmt 7.
   MISSIONVIEW §4's "bank + u32[bank+4+id*4]" CORRECTED to the
   self-relative form (§5c was right); FORMATS §18's "MISSION*.BIN
   follow the same layout" cross-ref UPGRADED assumed→VERIFIED.
2. READER CENSUS (12 absolute [0x4ede1c] sites, complete): loaders
   (0x41d670, .BIN leg 0x41dcc6/dd22, restore reload 0x446649/6fa)
   + THREE content-reader clusters — the terrain loop (4 ESI loads),
   the RESTAMP DRAWER FUN_00440dc2 (0x440d1c/0x440d93: type-DB word
   → FUN_00401471 into the backbuffer; the draw side of the 7j.26
   FUN_00440a2d stager), and FUN_00401010 = the 9-sprite RADAR
   STAMP (the bank's ONLY runtime content writer).
3. THE STAMP IS VESTIGIAL: [0x4edd94] := u32[0x454b00+4·set]
   (bases {1168,1773,1592,1168,58,58,1773}); the 9 records are
   fmt-0 stubs (6-B head + 4096-B image, span 0x1006) with
   gate=rows=0 as shipped and forever (the stamp writes only
   image+0x20.., a 5× downsample + 2:1 deshear of the 480×480
   viewport at the camera, 3 pages/call × 3 calls); NOTHING ever
   draws them — no reader of [0x4edd94]/0x454b00 besides the
   stamp/boot, LNK is IDENTITY on all 63 family ids in all 7 zones,
   and gate-0 short-circuits the blit. Zones A/B/C/D reference all
   9 ids from TOT words (E/F/G none) — those tiles render NOTHING.
   The stamp still runs every present (head of FUN_00401107) —
   wasted writes, zero observable effect.
4. §0b VERDICT (the unit's budget question): the bank is RENDER-
   ONLY presentation — every content reader is a pixel blit, the
   bank never feeds simulation state (get_z_pos reads .CGR, not
   BIN; the type-DB words index INTO the bank, never the reverse),
   and the only in-place writer reads the backbuffer and writes
   never-drawn scratch. DECISION: NO differ watch row for the bank,
   its directory, or 0x46cdb8 (write-only = below the emptiness-
   rule threshold); the state surface stays the TOT words/type-DB
   mirror rows (already covered). E-side needs only the 7j.35
   seam list (u8-RLE + per-tile remap); the scratch family/stamp
   need NOT be modeled.
5. Deliverables: RE-EXW-SIM §7j.36 + 2 new + 2 rewritten ledger
   rows (terrain bank; 7j.16 count gloss); MISSIONVIEW §1 census +
   §4 directory/gate corrections; FORMATS §18 cross-ref upgrade.
   registry_anchors green; manifest clean before AND after the
   corpus probes.

Nudge-Worker: d6b238f4-c0d3-4954-b02b-ede9b5eba5a4

## D102 — 2026-08-22: P4.2/W12-S3-prep — the E-side weapon-fire COMMAND producer LANDED in bedlam-core::weapon (the consumer FUN_00409138 subset + the two projectile banks + the per-type ticks + the damage table), RE-verified decode-exact against the existing local dumps (§7j.37); the S0/S1/S2 canonical chains stay BYTE-IDENTICAL (the no-inject invariant is a first-class pinned constraint) (worker 95ab9206 claim 2)

1. RE BASIS (§7j.37, dumps-only — exw-robottarget.txt the consumer
   decompile, exw-weaponanim.txt/-asm.txt the tick, the objdump angle
   family, ONE read-only corpus read of SINTABLE.BIN): the dispatch
   decode re-verified field-exact — fire gates mask ∧ cooldown==0 ∧
   ammo≠0; the inline spawn cases (artillery 1× type=id pos+0x100
   z=(z+0x15)<<8 cooldown 0 + UNCONDITIONAL mask clear; mines 2/4/6×
   types 0xF/0x13 with the 4-RandA-draw jitter/ttl/arc shape, class 4
   BOTH families; grenades 4/6× 0x1A/0x1F 3D vz ttl 0x32∓/＋RandA&0xF
   arc 0xB00−/0x900−RandA&0x2FF class 0 trail:=0; rocket ttl 0
   cooldown 5 arc=angle-pair NO RandA); the auto-rearm + loop-exit
   recharge; the bit0 pointer-bump quirk (bit0∧bit1 records read the
   triple from +0xB/+0xD/+0xF — E documents it and models the +7/+9/
   +0xB words only); the idle-tick gate is deploy-delay ≠ 0 ∧
   frame&3==0. SINTABLE.BIN = the full 256-word byte-angle sine ramp
   (FUN_0041eb65/77 = pure word lookups at a / a−0x40; the sector
   thresholds are words[2..66] of the same array — dual-use file
   table). Bullets: 2 tested sub-steps but NET TWO committed steps
   (3 moves − 1 rollback, tick += 6 — CORRECTS the 7j.22 "1
   committed" gloss); bullets free ONLY at tick>99. Artillery burst
   window indexes the duration table BY TYPE. Homing steering exact
   (heading := heading + angle-diff·4; vel = 2·(sin[h]>>4,
   sin[h−0x40]>>4); LEFT-first ±4-sector avoidance with the left-OOB
   z+=0x600 climb).
2. ENGINE SHAPE (the D85 W6 pattern extended): the banks + the ring
   are sim state but NOT in state_hash — they are the S3 T2 watch
   surface (their own dump rows, like the robot bank blob). Robot
   gains weapons[7] + weapon_mask (the +0x36../+0x6E record fields);
   staging is host-seamed (stage_robot_weapons — the D51 pattern;
   the S3.scen follow-up adds the scenario key beside `markers`).
   The consumer runs at the TOP of advance_frame (MissionShell
   order), the 4× enemy pass after the 6 robot phases. The
   difficulty dword 0x46cbf8 is staged into the sim (the scaled
   damage rows).
3. NO-INJECT INVARIANT (the unit's constraint, pinned THREE ways):
   unit-level (no staged records → zero RandA draws, banks all-free,
   order flag clear), the corpus-gated S0/S1/S2 chains re-run
   BYTE-IDENTICAL, and the canonical seam-gate test proves a
   well-formed flags=2 record with no staged slots fires nothing.
4. E-GAPS left OPEN (documented in the module head, the differ's
   expected finding classes until their units land): the five
   AI-order family spawn internals (w2..8/0x18/0x19/0x21..0x28),
   FUN_0040a9ff (mortar w0xE), the impact APPLICATION (FUN_0041a894/
   0041bc1c need the terrain-structure bank — S4), the debris
   disbursers, the SFX/message families (T4), the enemy-fire
   producers of the 0x22 bank (critter family), FUN_004197d4, the
   smoke-trail ring bank, and the family bookkeeping [hypothesis:
   the families mirror the inline ammo/cooldown/mask shape].
5. Deliverables: §7j.37 + 1 rewritten + 2 new ledger rows;
   DESIGN-DIFFHARNESS §6a/§10-W12; canonical.rs Command consumed;
   weapon_fire_gate.rs (28 tests). Workspace 100% green (565+28),
   fmt+clippy clean, registry_anchors green, manifest clean both
   sides. NEXT: the S3.scen/canonical-chain unit.

Nudge-Worker: 95ab9206-6d30-4fb1-8fd1-222e6d78e780

## D103 — 2026-08-22: P4.2/W12-S3 — the S3.scen + canonical-chain unit COMPLETE: the weapon/projectile T2 bank rows live end-to-end (canonical emission + EXD aliases + differ normalizers both channels + differ_gate row), grammar v1.3 gains the `loadout` staging key; the EXD twins for both banks PINNED (0x980d4 / 0x10e174) + a pre-S3 COMMAND payload off-by-one fixed (+7/+9/+0xB); S0/S1/S2 chains stay BYTE-IDENTICAL, S3 pinned 49193732e6dbc546 (engine+tests by worker 0bef7bae claim 2 — RE hop + fix + loadout key + canonical chain committed; the differ/registry leg left uncommitted by session death, ADOPTED + VALIDATED + COMPLETED by continuation worker 16ebe0c4 claim 2)

1. RE HOP (774eed4, -process BEDLAM.EXD -noanalysis + EXDProjBank.java
   → ghidra-project/exd-projbank.txt): the EXD twins of the two S3 T2
   banks are PINNED and now REGISTERED (RE-EXD-MAP §5c). Weapon-anim
   bank = **0x980d4** ×0x36 — the free-slot finder twin FUN_00023295
   walks stride 0x36 with the bound `iVar1 < 0x5460` = 400·0x36
   EXACT (slot-count re-confirm), and the tick twin FUN_000212f2
   (the enemy-×4 family) performs the 0x17 3-CLONE SPLIT at the
   0x980d4 base (`(&DAT)[slot·0x1b] := 0x17`, parent xyz copy,
   damped v −= v>>1) — the §7j.37 clone split EXACT. Projectile
   bank = **0x10e174** ×0x22 — the tick twin FUN_00022a52 (50-slot
   walk, type 0x65..0x68 = the §7j.28 draw-dispatch family) +
   FUN_0002a0f7 (the odd-i robot-hit lane). TAIL WORDS beyond the
   7 E-modeled fields: +0x1A a clamp-0..7 counter, +0x1E a −1
   countdown whose zero CLEARS the type — E models no producer;
   they parse on BOTH channels so a live O1 tail surfaces as a T2
   finding, never silence (the coverage discipline, not a gap).
2. THE COMMAND PAYLOAD FIX (ae8be6b, pre-S3 off-by-one):
   from_payload read x/y/z ONE BYTE EARLY (+6/+8/+0xA), folding the
   builder's +6 filler byte (SP: rand&0xF) into the target words —
   the correct grammar is words@+7/+9/+0xB exactly as RE-EXD-MAP
   §5c/D83 pinned and the FUN_00409138 decompile shows
   (exw-robottarget.txt:74-113: local_e0 = rec+7; the bit0 reads
   then bump +2; the MP-only mask block bumps first, SP never takes
   it). Fixtures re-authored with the filler byte 0x0A proving the
   offsets. 28/28 weapon_fire_gate green.
3. GRAMMAR v1.3 `loadout` (a928ad8, runner.rs): per-robot
   `idx,mask,id:ammo[,...]` entries (';'-separated) staged through
   the EXISTING stage_robot_weapons host seam — the D51/markers
   discipline (E-side staging seam, recorded never fabricated).
   Bounds mirror the original structures: idx 0..11 (12-robot cap),
   mask 0..0x7F (7 slots), ids 2..0x28 (the consumer dispatch
   domain), ammo 1..0x7FFF (positive i16), ≤7 slots, no mask bit
   beyond the staged list (auto-rearm never arms an empty slot).
4. CANONICAL + S3.scen (af5c2b8): parity_harness --canonical emits
   weapon-anim-bank (u32 count + the FULL 400×0x36 — the record
   field order IS the guest layout, no compaction picked because the
   bank IS the watch surface: a compact active-records form would
   hide free-slot reuse; documented as the §7 form) +
   projectile-bank (50×0x22, the 7 mapped fields + the zeroed
   +0x1A/+0x1E tail), both T2-tier-gated like every row; the
   order-target row now mirrors the COMMAND triple write. S3.scen:
   tiers T0,T1,T2,TS; markers 18,73,1 (the S2 walker = rocket
   robot 1); loadout robot 0 = one slot per INLINE-spawn class
   (artillery 9/0xA/0xB, prox 0x10→0xF, pressure 0x14→0x13, bouncy
   0x1B→0x1A, sticky 0x1D→0x1F) + robot 1 rocket 0x20→0x24; 8
   COMMAND volleys over 133 records / 132 frames — the cadences
   (8/5), the unconditional artillery disarm, the per-record ammo
   gate, the rearm cascade walking slots 0..2, the all-empty no-op,
   and the FULL spawn/active/free lifecycle of every family (the
   mines' 4-cycle class ladder frees ~frame 100; everything free at
   the tail). Chain 49193732e6dbc546 pinned, double-run
   byte-identical. SCOPE REFINEMENT (documented in the scen header
   vs the §7 row's "every modeled class"): bullets 2..4 / shell 5 /
   ballistic 0x17 / homing 0x29 have NO inline spawn case in the
   modeled dispatch — their records are born in the unmodeled
   AI-order families + the mortar; S3 fires what the COMMAND path
   can actually source, and a live O1 firing the other slots
   surfaces as the differ's coverage class ("S3 findings name
   them") rather than a silent gap.
5. THE DIFFER LEG (the adopted WIP, completed this run): both banks
   normalize on BOTH channels through the SAME field walk
   (weapon: count + 14 fields ×400 — kind u16 w@+0, owner d@+2,
   target d@+6, tick d@+0xA, draw_ctr d@+0xE, xyz Q13 d@+0x12/
   +0x16/+0x1A, v d@+0x1E/+0x22/+0x26, class d@+0x2A, arc d@+0x2E,
   trail d@+0x32; projectile: count + 9 fields ×50 — kind w@+0,
   xyz d@+2/+6/+0xA, v d@+0xE/+0x12/+0x16, tail_ctr +0x1A,
   tail_cdn +0x1E). E parses u32 count + records (count ≠ slot
   total = truncated dump, FAIL LOUD); O1 parses the BARE guest
   span (no count cell on the guest — the free-slot walk is the
   bound; RE-EXD-MAP §5c). watches.toml: exd_addr 0x980d4 /
   0x10e174 filled, exd_status verified, layouts rewritten
   field-exact; registry_anchors' W1 T2-empty rule narrowed to
   exempt exactly these two rows. differ_gate: S3 joins the
   s0_s1_cross_and_double_run loop (inv_frame fabricates the O1
   banks as the bare spans) — cross PASS-WITH-NOTES with exactly
   the 2 E-only row findings, ZERO field gaps, ZERO T2 diffs;
   double-run PASS modulo counter/RNG.
6. VERIFIED (this run, full re-validation of the whole unit):
   diffharness suite green (incl. the 2 new bank-row tests —
   same-walk-both-channels, wrong-count/short-span fail-loud);
   canonical_dump_gate 6/6 (S3 chain + S0/S1/S2 pins
   8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4 all
   re-asserted — the no-inject invariant holds); differ_gate green
   over S0/S1/S2/S3; fmt applied clean; manifest clean before AND
   after the corpus-reading runs. Deliverables: DESIGN-DIFFHARNESS
   §7 v1.3 note + §10-W12 S3-LANDED; watches.toml; differ.rs;
   registry_anchors exception. NEXT: S4 (the destroy family) gates
   on the S3 finding set from a live session; the unattended tail
   is the §7j/FORMATS backlog + W10/W11.

Nudge-Worker: 16ebe0c4-74ee-48e1-bf36-73e981ed114f

## D104 — 2026-08-22: P4.2/W12-S4-prep — the E-side destroy family LANDED in bedlam-core::destroy (staging seams + the two resolvers + the destroy tail + the 20-kind debris stager + the splash/blast/platform/trap lanes + both disbursers + the weapon-tick impact wiring); the S3 canonical chain re-pinned ONCE 49193732e6dbc546 → e29f76f5585401e1 (RE 7j.38/7j.39 by worker 460d294e claim 2, committed dcc8865 + acf09ff; engine+tests built by worker d57a4dec claim 2 but left uncommitted by a SECOND session death — ADOPTED + INDEPENDENTLY RE-VALIDATED + COMMITTED + PUSHED by continuation worker 3e93a4b1 claim 2)

1. RE BASIS (committed first, dcc8865 §7j.38 + acf09ff §7j.39,
   objdump-only from the existing local dumps): the destroy-family
   RNG census (the five-effect draw table 8/8/8/8/8/0/0/72/9 for
   sel 1..9, the chain-walk one-roll-per-QUALIFYING-candidate
   protocol word>0 → alive → chain≠0 → draw, the platform 2×5,
   the trap 3×5, the k11 SFX-gate draw, the blast k6 1-in-8 gate),
   the rubble 0x454a04/water 0x454ae4/7-artillery-pair-lists
   DGROUP tables, the FUN_004244a1 script-blast internals, the
   impact-pair call orders (0x29 REVERSED vs 0x24), the chain-walk
   geometry CORRECTED (4 perimeter walks, the walk-2 (Y+W)<<13
   recursion quirk), the debris allocator (first-free else
   min-seq LRU), and the class-0 quadrant body (disburser FIRST;
   0x1a damage even for dying 0xF/0x13).
2. STAGING (host seams per the D51 pattern): .BDG type table
   (≤282 variable rows, the §7j.25 grammar; the four template
   banks kept in DISK order +0x3E/+0x46/+0x42/+0x4A), .POS
   instances (2000×16 B, footprint word idx+1 + hp re-stamp),
   .TRT structures (hp = 250 + 250·m/27), the 0x7d2/0x7d3 hazard
   stamper over staged mirror words, the TOT-mirror/seen banks
   (staged EMPTY — the init_tiles TOT fill is the S5 pairing),
   and the language latch (the GER gate).
3. RESOLVERS + TAIL: FUN_0041a894 (pass-through on
   empty/hazard/clamp words; the platform 0x7d4 → FUN_00422693;
   survivor = pure subtract; immune −1; destroy → the tail),
   FUN_0041bc1c (rubble stamp + seen + DAT-zero + k15 + splash),
   the tail = objective notify (zone 7, kinds 0x44..0x47) → the
   GER skip → the +0x46/+0x4A RESTORE (linear (z·H+i)·W+j, z ∈
   [z0, min(z0+D,8)), seen := under_dat==0) → the five-effect
   loop → the score award (0xb → 10) → the four chain walks
   (recursive resolver at 1000, forwarded flag).
4. IMPACT WIRING (the §7j.39/2 orders): bullets/shell floor =
   OBJECT→STRUCTURE→disburser(K2)+free; 0x24 floor = OBJECT→
   STRUCTURE→disburser(K6)+free; 0x29 floor = STRUCTURE→OBJECT→
   disburser(K9)+free (REVERSED, faithful); the artillery burst =
   the pair-list walk per tick−0x20 with the script blast + the
   50% k11 gate (past-window: tick ≤ 0x22 silent, else the
   disburser); the mortar 0xE 3-cell at the POST-halving
   offsets; the class-0 expiry = disburser→4×OBJECT(0x1a)→4×
   STRUCTURE (the 0xF disburser arm is the raw-asm NO-OP — a 0xF
   mine PERSISTS past class 0, the §7j.14/3 map corrected; the
   mine-proximity family is the §7j.39/8 E-gap); the projectile
   type-1/2/3 branches (0x65 → K20+clear — the §7j.13/5 "no
   deactivate" gloss superseded: the disburser's clear IS the
   deactivation, the open edge documented for S4+ coverage).
5. GATES + THE S3 RE-PIN: destroy_gate (16 tests, fully
   synthetic/CI-safe: the BDG roundtrip incl. the disk bank
   order, the TRT hp formula, footprints/hazards, survivor/
   immune/destroyed, the GER gate, the §7j.38 draw-count table
   via a same-seed reference sim, chain detonation + the
   non-chainable zero-draw, the rubble stamp, the stager gates,
   the platform destroy/weaken draw counts, the trap lane, the
   disburser arms, the blast robot box-lane, the 217-pair table
   shape, the no-inject pass-through). weapon_fire_gate updated
   to the corrected 0xF-persist/0x13-free and 0x65-clear
   behavior (28/28). S0/S1/S2 chains BYTE-IDENTICAL
   (8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4 —
   the no-inject invariant holds); S3 re-pinned ONCE to
   e29f76f5585401e1 (the burst pairs + the stager gates draw the
   shared stream — BEFORE any O1 S3 capture exists; the frame-100
   class ladder + the persistent-0xF tail re-derived).
6. THE D104 DIFFER CONTRACT: the armor-pad-reads/typedb-fade-byte
   rows canonicalize BOTH channels to the last-nonzero prefix —
   E's +0x18 bank is lazily materialized (it keeps its grown
   length after the ≤7-frame fade zeroes the tail bytes) while
   the guest grid is full-size; identical content now
   canonicalizes identically (subsumes the §6a "len 0 == all-zero
   grid" equivalence). The destroy family's scorch writes land in
   that same bank — this is the contract that keeps the widened
   scorch writers differ-safe.
7. E-GAPS (documented, S4/S7 findings name them): the splash TICK
   body (odd-frame fall/absorb, the per-tick 5-draw scorch
   re-roll, the water stamps), the platform spread ring + creep
   tick (S7), the trigger producers FUN_00422e0a/FUN_00422600
   (S7 no-ops), the FUN_0041a225 effects bank (RandB-fed), the
   critter area-damage lane (no critter bank in E), the debris
   physics pass FUN_0040de9c, the objective at-zero extraction-
   arm tail (the S6 seam), every SFX family (T4), and the §7j.39/8
   open items (the 0x1F floor arm dispatch reading + the mine
   proximity checks — the audit unit).

Nudge-Worker: 3e93a4b1-0132-4e70-a159-e60f176207a5

## D105 — 2026-08-22: P4.2/W12-S4 — the S4.scen + canonical-chain unit COMPLETE: the destroy-family dump rows live end-to-end on T1/T3 (grammar v1.4 `destroy = 1` staging key, canonical blob forms, differ normalizers BOTH channels, differ_gate S4 row), chain pinned 2ddd15ea50c8a14d; the MissionShell gains the destroy-score fold (zero without staged destructibles — the no-inject invariant: S0/S1/S2/S3 chains re-asserted BYTE-IDENTICAL) (engine+tests built by a predecessor session and left uncommitted by session death — ADOPTED + FIXED + VALIDATED + COMPLETED by continuation worker 65f39dff claim 2)

1. GRAMMAR v1.4 `destroy = 1` (runner): strictly `1`, once per
   scenario, fail-loud on anything else (a typo'd value must never
   silently skip the staging AND its dump rows). Stages the mission's
   OWN .BDG/.POS/.TRT through the EXISTING `stage_destroy_family`
   host seam and gates the destroy rows (S0..S3 bytes untouched).
   This key is an EQUIVALENCE seam, not a fabrication seam: the
   original loads the same three files natively at mission load
   (FUN_0041a4f8 + FUN_004170a6), so the staged CONTENT is identical
   on both channels — dbx-plan records it in `_e_staging` with the
   equivalence note + the recorded pre-S5 divergence (E stages the
   TOT-mirror/seen banks EMPTY; the init_tiles TOT fill is the S5
   pairing per §7h.4/D99).
2. S4.scen (ZONEA/MISSION1, 49 records): the TRAP leg (a marker
   robot on the tile-0x62 cell — resolver-100 no-score destroy at
   the anchor, 5× k12 + sel-9 k20 + the 3×3 splash ring + the
   restore into the empty-staged mirror), the ARTILLERY leg (a
   marker gunner firing 9/0xA/0xB at its own tile — ring 0
   script-blasts the .TRT turret with the rubble stamp, rings
   4..6 cascade the chainable cluster through recursive
   1000-damage detonations, the blast box damages the gunner —
   the faithful §7j.23 robot lane), and the SURVIVOR leg (two
   grenade volleys on a multi-hp structure — pure monotone
   subtract, never destroyed). Bullets/shell/homing stay the
   documented producer E-gaps (differ coverage, never silence).
3. CANONICAL FORMS (the §6a destroy rows, OUT of state_hash per
   the W6 split): object-instances = u32 count + 23-B
   {slot,x,y,z,id,flags,hp} keyed by .POS slot (the guest id==-1
   dead slots never ride; the guest COUNT cell is capture
   plumbing, never compared); trt-array = u32 count + 20-B
   {active,hp,x,y,z}; tile-word-grid + platform-strength = the
   SHARED bare w·h·2 span (both channels identical — a length
   mismatch is a structural finding); typedb-mirror-rows =
   COMPACT-ACTIVE {tile u16, 8×(word u16, seen u8)} with the
   nonzero-tile filter applied on BOTH channels (the O1 full
   0x1E-stride grid canonicalizes through the same filter); the
   T3 pair debris-stager (42-B FULL bank) + splash-records
   (10-B FULL bank) is E-ONLY until their EXD aliases land —
   row-level coverage findings, never fabricated bytes.
4. DIFFER: the O1 normalizers (the guest object 0x14-stride
   count-bounded walk skipping dead slots, the TRT 0x20-stride
   stride-offset map {+0,+0x10,+0x14,+0x18,+0x1C}, the mirror
   tile filter) + the destroy count words classed Structural.
   differ_gate S4 = cross PASS-WITH-NOTES (exactly the 4 E-only
   rows, zero field gaps, the single T2 counter note).
   CONTINUATION FIXES to the predecessor WIP (all caught by the
   gates, none reached a commit): the trt inverse-fabricator
   0x10..0x24 slice overrun, the O1 count-cell stride guards
   ((len−4)%stride, not len%stride), and the mirror canonical
   parser misreading the compact 26-B tail with the raw 0x1E
   layout (words at 2+3z/seen at 4+3z) — the field walk now
   takes extracted per-z pairs so the two layouts cannot cross.
5. THE SCORE FOLD (mission.rs): the destroy tail's award folds
   into the campaign score cell in the MissionShell advance
   (take_destroy_score — zero without staged destructibles,
   the S0..S3 no-inject invariant re-asserted
   8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4 /
   e29f76f5585401e1).
6. NEXT-GATE NOTES: a live S4 capture needs the dbx-plan T3-tier
   unit first (the S3 T2-tier precedent — SUPPORTED_TIERS is
   still T0/T1/TS); the mirror-rows O1 walk relies on the
   recorded empty-staging divergence until S5 lands init_tiles.

Nudge-Worker: 65f39dff-5b6e-4da0-b7f8-d85cb435ce96

## D106 — 2026-08-22: QUEUE HYGIENE — completed Now items move to the Done log at end of run, and the scheduler now mechanically skips a first-word DONE marker (nudge-free-items.py gains the DONE check + a test-nudge-queue.sh case) — a marker-blind scheduler respawned the finished W12-S4 item forever while real work (W12-S5-prep, dbx-plan-tiers) starved behind stale "2. DONE ..." blocks; this unit re-verified item 2 genuinely closed at HEAD (differ_gate 7/7, destroy_gate 16/16, canonical_dump_gate full-chain assert, weapon_fire_gate 28/28, registry_anchors 2/2, manifest clean) and removed the five stale DONE blocks, renumbering W12-S5-prep→2 and dbx-plan-tiers→3 (worker 78203f4f claim 2)

Nudge-Worker: 78203f4f-22e4-4024-bcfa-88f79b85ac6a

## D107 — 2026-08-22: P4.2/W12-S5-prep — the E-side pickup producer LANDED in bedlam-core::mission (stage_pickup_surface = init_tiles staging + the zone/set cell; the clear→move→test→fire consume protocol in robots_phase; apply_pickup widened to case-4 score/money pending awards with the MissionShell fold + host-seamed cases 8/9). RE notes first (§7h.5, commit ad43c12): the range/floor tables are indexed zone_index-0-based — the 0x454a04..0x454ac8 DGROUP family is one contiguous run of 7-dword tables at exact 0x1C strides (no head slots, so base+(cell−1)·4), corpus-confirmed by the D99 ZONEA-inert/ZONEB-601 censuses; the PRE-EXISTING destroy.rs zone tables (RUBBLE/HAZARD/WATER) were landed raw-cell-indexed with unused heads — flagged, corpus-dead, left for the S5/S7 differ rows to arbitrate. The latch clear (0x40bef2) is UNCONDITIONAL like EXW — the ZONEA walk path never latches (all three mission_corpus_gate hash pins survived untouched) and the corpus gate proves the staged surface is hash-invisible + fires ZERO traffic on ZONEA/M1 (the D99 census re-derived live: 80 cells, the exact word multiset pinned) while ZONEB/M1 stages 152 live cells (case-4 dominant). S0..S4 canonical chains BYTE-IDENTICAL (2ddd15ea50c8a14d joins the pinned set asserted). PICKUP_AWARDS moved core-side (the sim draws it); bedlam-game re-exports. Next: the W12-S5 scenario unit (grammar v1.5 staging key + S5.scen on ZONEB + the canonical pickup rows) (worker f32193a2 claim 2)

Nudge-Worker: f32193a2-ecfe-4387-8fbe-68f136fe4444

## D108 — 2026-08-22: P4.2/W12-S5 — the S5/S5B pickup scenarios LANDED (grammar v1.5 `zone`/`pickup` keys + the episode-slot host seam + the zone-row O1 normalizer). FOUR decisions recorded: (1) ZONE STAGING = the D51 host seam `GameHost::stage_episode_slot(stage, mask)` standing in for the campaign-advance (0x41c9e5) / save-load-restore (0x43c2b8) shells the engine does not model; the grammar key is `zone = "B"` (letter A..G → stage, mask 0 → MISSION1, linear stays the fresh-slot 0 — a live campaign-walk O1 session carries its own linear/mission counters, so the plan records the zone seam and the linear diff is the live-capture seam, never an E fabrication). **SUPERSEDED 2026-08-25 (S0-12b/D154):** the "linear stays the fresh-slot 0 / never fabricated" stance predates the §7j.64/D decode — the guest cell [0x46ae8c] is NOT a campaign-progress counter at all but a DERIVED cell `clamp(5·(zone−2)+mission−1, 1, 26)` recomputed by GameMain from the CURRENT slot every episode (fresh/staged slots read 1 on ZONEA/M1 and ZONEB/M1 alike). Emitting the derivation is therefore not a fabricated seam but the cell's actual write semantics; the canonical row + the TRT hp tier selector now read the derived value, and the live-capture seam D108 reserved is exactly the played-campaign slot (a zone/mission pair the corpus never stages) — still never fabricated. (2) THE TWO-SCENARIO SPLIT: cases 1 and 3 are never co-walkable (nearest pair z3 (26,21)↔(76,10) = 61 octagonal tiles; one order's claims all lie within ORDER_RADIUS 0xC0 of the order tile), and two sequential orders need the first order CLEARED — all-alive-state-3 is impossible while the next leg's robots stand idle (state-3 robots can never re-claim, verified EXW subset), so the only in-model path is the 0x197-frame window expiry whose ~407 idle frames × ~340 KB/record of REAL mirror rows (every ZONEB tile is active: 15,102 words + 52,715 seen) is not a shippable dump — S5 (c1/c2/c4 row-21 trio) + S5B (c3+c4 row-10 five) cover cases 1..4 in 16+19 records. (3) DAT-BYTE VISIBILITY ANSWERED: the consume's DAT := 0 needs NO dedicated differ row — the mirror word (:= table-C floor)/seen carry the pickup observation, the walkability change rides the robot-bank rows, and the raw DAT volume is not a guest span any watch row carries. (4) THE ZONE-ROW CONVENTION: E's canonical zone is the 0-based mission-slot index while the guest cell (EXW 0x4edd8c / EXD 0x107500) is the 1-based set (D99) — the O1 normalizer maps cell−1 (S0..S4 chains unaffected; first exercised by S5/S5B). Staging order pinned: destroy family → stage_pickup_surface (the engine load-order note) → the §7j.12/6 hazard stamper (30 ZONEB grid cells: 24× 0x7d2 + 6× 0x7d3), matching the original mission-load order. Case-3 observability note: the walker spawns at the hp clamp ceiling (5000), so the case-3 +2500 body is value-invisible — the consume/dispatch still ride the rows; a pre-damaged-walker variant is the live follow-up (worker c2aba48b claim 2)

Nudge-Worker: c2aba48b-1e33-43f3-9ea5-19b4cabe8a1d

## D109 — 2026-08-22: P4.2/dbx-plan-tiers — dbx-plan compiles the T2/T3 tiers, and the O1 bank-row blob grammar gains the COUNT-PREFIX form (capgen `prefix` sub-row). THREE decisions recorded: (1) THE ALIASED T2 BANKS ARE FULL FIXED SPANS — weapon-anim (0x980d4, 400*0x36 = 0x5460) and projectile (0x10e174, 50*0x22 = 0x6A4) emit with NO count cell and NO count bound, because the differ's O1 normalizers walk the WHOLE banks (the guest free-slot walk is the bound; a truncated dump is a BadLength error, fail loud); every unaliased T2/T3 row (mortar/critter/POI + all 14 T3 rows incl. debris-stager/splash-records) STAYS an explicit `_deferred` coverage gap — never emitted on O1, the differ reports them E-only (a future aliased row compiles only through a deliberate form: indirect or count-driven extents die loudly, never a guessed address). (2) THE COUNT-PREFIX GRAMMAR: the differ pins trt-array/object-instances O1 blobs as u32 count + records, but the count cell (0x11949c / 0x119554) is not contiguous with the banks — capgen watch rows gain a `prefix` {addr, len} sub-row (dump the 4-byte cell first, concatenate into the one blob; headless-proven in the flow probe with the BDA COM1/COM2 prefix), and dbx-plan emits `Prefixed` for those two rows (robot-bank stays the bare span its normalizer defines — asymmetric BY CONTRACT, not accident). Without this a live O1 S1/S4/S5 capture would fail row normalization structurally on both rows. (3) OBJECT-INSTANCES DUMPS THE WHOLE 2000-SLOT BANK (the D108 queue note): the ZONEB .POS surface carries live slots past dead holes (max slot 1128 over 1096 live), so the count-bounded `$obj_count*0x14` span silently dropped 32 live objects and broke the count field — the row is now the prefix + the fixed `2000*0x14` bank (dead id==-1 slots skipped in the walk) and `$obj_count` retires as a resolve symbol (the count VALUE rides the blob head every frame). BONUS FIX: the D103 loadout `_e_staging` mask emitted a JSON hex literal (0x7f — unparseable; S3 is the first compilable loadout-bearing plan, which surfaced it); masks are decimal now. capture-plans/S3+S4.json committed + byte-pinned; S1/S2/S5/S5B regenerated (prefix/full-bank rows); S0/S0W untouched. Workspace 54 suites / 632 tests green, fmt+clippy clean, all four dbgprobe modes GREEN headless, manifest clean (worker 33a28c84 claim 2)

Nudge-Worker: 33a28c84-b6ed-433a-8d56-5b8fe32f9b74

## D110 — 2026-08-22: P4.2/W12-S5C — the case-3 OBSERVABILITY variant S5C LANDED: the pre-damaged-walker corridor walk that closes D108's value-invisibility gap (S5B's walker spawns AT the 5000 clamp, so apply_pickup case 3's +2500 read 5000→5000). FOUR decisions recorded: (1) THE SPEND = the S4 artillery pattern, staged ON the walker: a third marker banks the gunner at the walker's own tile (73,10,3 — ≤5 tiles from the order tile, inside ORDER_RADIUS), its loadout arms 9/0xA/0xB (1 ammo each), and the frame-1 command fires all three records at the gunner's tile (bursts land at the FIRING robot, §7j.38/5). The §7j.23 robot lane (312/pair) box-reaches a marker-staged robot (+0xF00 = Q5 offset 15 of 32) from exactly FOUR list-0 pairs ({T,T+1}×{Ty,Ty+1}) per burst → 3 records × 4 × 312 = 3744 spend at frame 32 (tick 0x20) on the walker AND the gunner (both survive at 1256); the 0xB's outer ring spends the CLICKER 624 at frame 36. ALL damage lands pre-order while every robot is state 0/3 — the hp path (a state-4 robot converts damage to a shield tick, 7g.1), so the order must arm AFTER the burst window closes: frame 37. (2) CASE 3 IS NOW FULLY VALUE-VISIBLE: at frame 41 the walker heals the EXACT +2500 (PICKUP_HEALTH) — 1256 → 3756 UNCLAMPED (strictly better than the ticket's clamp-tolerant bar). The gunner claims its own spread slot and walks one robot behind the whole way — lower index moves first in the robots phase, so it reaches no unconsumed cell (case 3 is single-use) and its hp stays 1256 through the tail: a same-run negative control for the heal. (3) THE CENSUS WEAKENS BY DESIGN: S5B's whole-map "exactly the six cells" assert does NOT hold for S5C — the burst rings' 5000-damage structure/object resolvers detonate the destroy CHAIN CASCADE (232 cells change, off-corridor restores + seen clears out to columns 63..81); all of it is deterministic destroy-family state carried by the SAME aliased T1 rows (the differ passes with exactly the 2 S1-class findings — the cascade is not a new coverage class), and the test asserts the corridor cells + the hp schedule instead of the whole map. (4) TIERS STAY T0/T1/TS: the artillery stages k6/k11 debris + splashes (T3 families) but the S5C purpose is the T1 hp observation — the T3 rows stay unwatched, S4/S3 already cover them. Mechanics: 55 records (anchor + 54), chain e0999fcb3455d3ef pinned, double-run byte-identical; differ_gate S5C row joined; dbx-plan compiles (4 inject rows: the frame-1 command append CS:0009255C + the frame-37 order triple CS:0010E0A4/A8/AC; the loadout seam in _e_staging); capture-plans/S5C.json committed + byte-pinned; the command record's triple = the order tile in the grammar's RAW Q5 words (2496,320,3) per the verified record convention. S0..S5B chains re-asserted BYTE-IDENTICAL (no engine change — docs/scenario/tests/plan only) (worker 82d5a27f claim 2)

Nudge-Worker: 82d5a27f-71df-4043-a811-19c9f8655bbb

## D111 — 2026-08-22: QUEUE HYGIENE #2 — a worker's end-of-run queue note RE-QUEUED an already-closed unit (the S5C state commit 105d9aa resurrected the MISSIONVIEW §8 water-flag/anim remainder, closed ~15 units earlier as D100/§7j.35 by bee4336+60f7d3b), and the claim spawned a worker onto finished work. LESSON recorded: when re-queuing an item read off an OLD queue note (e.g. "queued by D99"), grep the Done log + DECISIONS for the item's headline BEFORE writing it back into Now — the D106 marker fix only catches in-place "N. DONE" blocks, not a stale hand re-queue (a Done-log-aware semantic dedupe in the scheduler was judged too fuzzy to automate; nudge-free-items.py untouched). This unit re-verified the closure genuinely green at HEAD: §7j.35 delivers every requested artifact (u32[0x456ca8] = STATIC DGROUP ping-pong const {0..7,7..0}, 2 readers 0x40691a/0x406a2c / zero writers; [0x4edbd4] ≡ 1 every mission — sole persistent writer FUN_004252c0@0x4252d8 + the 0x41c649/0x41c65a SELECTOR save/restore bracket, water-off arms incl. the 0x12d/0x12e/0x12f gates dead code; the 7j.12 zone-table off-by-one ledger correction; the §0b verdict = no new differ watch row; MISSIONVIEW §8 all four items closed) — with INDEPENDENT spot-checks this run: objdump re-grep confirms both censuses instruction-exact, the file-image read at 0x552a8 confirms the const byte-exact, registry_anchors 2/2, MANIFEST clean before AND after the read-only probe. Stale Now item removed (W12-S6 renumbered to item 2), the stale Backlog PROMOTED note folded to CLOSED (worker e444e1cd claim 2)

Nudge-Worker: e444e1cd-c13a-4631-8262-ce2f996c4670

## D112 — 2026-08-23: P4.2/W12-S6 — the EXTRACTION scenario S6 LANDED: the scripted .PAD step-on arms the beacon through the real producer and the dropship runs its full cycle end-to-end. FOUR decisions recorded: (1) THE WALK IS COMMAND-DRIVEN, NOT ORDER-DRIVEN: EXW arms the beacon family 0x4eabb0 through exactly ONE caller — the pad-script armer FUN_004247b5, sole caller the dispatcher FUN_00433980 — a click NEVER arms it (§7j.40/5), and E's `order` step arms the beacon directly (the documented click-seam approximation), so a pad trigger can never fire while an order is pending (the armer's one-beacon-at-a-time head gate). The faithful route is the ORIGINAL's own move mechanism: a COMMAND record with flags bit0 SELECT (FUN_00409138 bit0, §7j.37/1) writes the move-target words + auto-arms state 1 + stop 1e6 — a walking robot with a target and NO pending order, exactly the dispatcher's dual gate (state ∈ {1,4} ∧ move-target ≠ −1). (2) THE SLOT DEVIATES FROM THE QUEUE NOTE, DELIBERATELY: the queue's `pad 8` gloss "(the D86 census slot-0 record (5,61,0))" predates §7j.40's verified census — slot 8 = (2,14,1) is a LEVEL-1 pad (the probe matches z>>5; a ground robot at z 0x1F can never match LEVEL 1), and (5,61,0) is slot 0's record. S6 targets slot 0x12 = 18 = (19,70,0), a GROUND member of the zone-1 census set {8,0x10,0x12,0x18}: terrain-probed walkable at floor z 0x1F, east neighbor (20,70) blocked, column 19 open y67..y73 — the walk = two COMMAND legs (west to inside tile (19,73), then due north crossing the pad mid-walk so the sub-tick dispatcher probe fires with state 1 + target present). (3) THE SINGLE-ROBOT WINDOW-0 DEPLOY: the MRK squad alone banks exactly 1 robot (D89), so the beacon window is 0 and the MissionShell beacon block — the SAME frame's epilogue, after the robots phase — deploys immediately at the trigger frame (FUN_0041faf0 gated on the pad producer tag + craft-inactive); the armed-beacon observation is carried by the SURVIVING tile/claims latch (FUN_0041faf0 clears only the flag/window pair — the beacon-family row reads {0,0,19,70,31} from f13 on, the claims 0x4eabba persist with slot 0 claimed). Full pinned timeline: f13 trigger+deploy {1,1,608,2240,512,0,0}; f14..34 descent (8×−0x20 then (alt>>2)·3 shrink, group flip); f35 LANDING + the extraction sweep (walker state 3→5, stop 1e6); f36..44 dwell (the RandA&7==0 altitude jitter — exactly one hit at f40, a shared-stream draw); f45..68 departure (alt += (alt>>2)+1, x −= group·4); f69 alt 567 > 0x200 → inactive + complete, frozen to the tail. (4) THE .PAD TERMINATOR BUG found + fixed: the pre-existing parser's `if x == -1` break was dead code (`u16 as i32` is never −1), so the runtime slot bank collected the 0xFFFF terminator + fill records past the live run — the DAT marks were accidentally correct (the out-of-bounds guard skipped the fill) but the D86 missing-slot rejection could never fire; the check now compares the u16 against 0xFFFF (ZONEA/M1 = exactly 114 live slots, terminator-bounded). Mechanics: S6.scen (T0/T1/T3/TS, 75 records), chain c96f0735df1059ea pinned, double-run byte-identical; corpus_s6_pad_extraction gates the full timeline; the dropship-frame differ normalizer (7 i32 leaves, E-only — exd_status unmapped, a coverage finding, never fabricated); differ_gate S6 row (cross PASS-WITH-NOTES, exactly the 2 S1-class findings + the dropship row, zero field gaps); capture-plans/S6.json compiled + byte-pinned (3 injects: the pad op — the static-pad-slots bank read CS:0000F63C, slot 18, the order-target triple — + the two command records; NO staging seam rows: the run banks the MRK squad only and the zone cell is the live game's own staging on O1, an equivalence). S0..S5C chains re-asserted BYTE-IDENTICAL (the terminator fix trims only never-hashed fill records). Workspace 54 suites green, fmt+clippy clean, manifest clean both sides. The engine leg (the extraction family, edafd02) + the §7j.40 decode (631bd28) landed by predecessor worker 8d32d85d; the harness/gate/plan/docs legs adopted its interrupted WIP (canonical.rs + canonical_dump_gate.rs left uncommitted at session death) and completed it (worker 4d92bb13 claim 2)

Nudge-Worker: 4d92bb13-2edd-4de1-a80b-6810812baf34

## D113 — 2026-08-23: P4.2/W12-S7 — the PLATFORM-DYNAMICS scenario S7 LANDED: the build/spread/creep/destroy lifecycle in one ZONEA/MISSION1 run, through the real producers end-to-end. FIVE decisions recorded: (1) THE RING-GATE CORRECTION (§7j.41/3): the weaken→spread gate is (old ≥ 200 ∧ new < 200) ∨ (old ≥ 100 ∧ new < 100) — the §7j.12/2 gloss "strength ≥ 100 and (diff < 200 or new < 100)" is REJECTED by the asm (it would build for old ∈ [100,200) ∧ new ∈ [100,200), which the code refuses); the creep-site latch [0x4dc5c8]/[0x4dc5cc] := (x,y) fires ONLY on the weaken→ring path — the DESTROY path latches NOTHING (second §7j.12 gloss corrected). (2) THE TRIGGER DISPATCHER (§7j.41/1): FUN_00422600 is the per-zone BRIDGE-BUILD trigger — the destroyed instance's TYPE id must EQUAL the zone's code (zone table 0x4225e4: zones 1/4 → 5, 2/7 → 0x84, 5 → 0x2f, 6 → 0x2710 never-code; zone 3 SUB-DISPATCHED by the WITHIN-ZONE MISSION NUMBER [0x4edd88] through 0x4225d0 — that cell's second use pinned), and the strength-300 ring builds at the DYING INSTANCE'S OWN record (x,y,z) — the §7j.12/9 "matching a record id" gloss corrected. (3) THE PER-FRAME RandA GATE-DRAW FINDING (§7j.41/4): the creep tick FUN_00422a9c draws ONE RandA AT ENTRY, UNCONDITIONALLY — the MissionShell epilogue calls it every frame on every mission, so the ORIGINAL consumes one RNG draw per frame even with no platform staged (plus 2 jitter draws on lucky frames). E runs the tick ARMED under the new grammar v1.6 `platforms = 1` key so S0..S6 chains stay BYTE-IDENTICAL while S7 is faithful from frame 0; the E-side stream gap on unarmed scenarios stays until a deliberate re-baseline, and on live O1 captures the rng-state rows are the channel-finding class (budgeted — the plan's _e_staging note records the equivalence, never a fabricated write). (4) THE VOLUME-2 PLATFORM WRITE: the ring build stages the water z-structure at VOLUME 2 (seen 0) — corpus-pinned in S7 (the zone-1 water base 0x25D at z2 with seen 0, vs the plateau word 226 seen 0 below) — the platform is a walkable water plane whose collision comes from the typedb volume, not the seen flag. (5) THE ONE-RUN INSTRUMENT: all four observables in a single run at the ZONEA/M1 census site — .POS slot 74 @ (3,57,2) is the mission's ONLY type-5 (zone-1 code) instance (hp 75, W1 H1 D2); the marker gunner stands ON it so its own quadrant blocks three of the eight ring tiles (five build — the live-robot-quadrant gate OBSERVED, not just decoded); the same burst's pair-7 destroys the fresh (4,56) (the destroy observable, 5× k7 debris, same frame); four pre-build grenade volleys (f18-f30, aimed (2,54)/(3,54)) detonate f32-35 ON the fresh platforms — two 75-hits per tile walk both ring gates (300→150 spread-builds the north row; 150→75 latches the site) and the third destroys (75−75 ≤ 0, 5× k7 each, 20 total by f35); the armed creep then grows the bridge from the f34 latch — first 199 tile f449, 22 creep tiles by f1240, tail static (27 field tiles = 22 creep + 5 survivors). Mechanics: S7.scen (T0/T1/T3/TS, 1361 records, chain b41db389f3ad8947, double-run byte-identical; the scenario's creep-schedule COMMENTS were stale pre-calibration text — corrected to the pinned timeline, chain unchanged, comments are not dump input); corpus_s7_platform_dynamics gates the full timeline (field snapshots, 0x7d4 grid words, the 0x25D/seen-0 water semantics, the k7 census 5/20, the creep schedule); differ_gate S7 row (cross PASS-WITH-NOTES, exactly the 2 S1-class findings + the debris/splash E-only pair, zero field gaps — the platform rows fabricate as the identity spans both channels share); capture-plans/S7.json compiled + byte-pinned (34 anchor + 25 per-frame, 5 command injects CS:0009255C, the markers/loadout/destroy/pickup/platforms seams in _e_staging); the §8 ledger rows rewritten with the §7j.41 corrections. S0..S6 chains re-asserted BYTE-IDENTICAL. The §7j.41 decode (984a078) + engine producers (ea2f259: platform_ring_build/platform_tile_build with all gates, platform_creep_tick, the FUN_00422600 destroy-tail trigger, the volume-2 write) + the scenario leg (b9cbcf3) landed by predecessor worker 56d80c42 claim 2 which died at session end before the differ/plan/docs legs; this run adopted the pushed state and completed them (4c6c068 differ + 13bae85 plan) (worker 0b66f6a6 claim 2)

Nudge-Worker: 0b66f6a6-342f-4b77-8143-c367d6926ecd

## D114 — 2026-08-23: P4.2/W12-S8 — the CRITTER-ENGAGEMENT family LANDED: the controller subset, the fire cycle, the death handlers, and the S8 scenario, end-to-end. SIX decisions recorded: (1) THE STAGING+ARM KEY (grammar v1.7 `critters = 1`): the ORIGINAL loads .NME natively at EVERY mission load (FUN_00416458) and runs the controller FUN_00412f34 UNGATED (MissionShell 0x447fe1) — the loader's 10 kind-4 heading draws + the controller's per-frame draws are consumed on O1 on every scenario, so E stages + arms per scenario (the D113 `platforms` pattern, bigger) and the S0..S7 chains stay BYTE-IDENTICAL; the E-side stream gap on unarmed scenarios is recorded, never fabricated. (2) THE §7j.43 CORRECTIONS (each asm-verified over the predecessor WIP + the §7j.42 glosses): the d=2 mode-2 break roll NEVER fires (0x413e81 jumps straight to the substep burn — the "always" gloss was inverted; the ±(facing+0x40) strafe rolls on EVERY break path); the kind-5/6 mode-6 dive aims AT THE IMPACT and steps the REVERSED heading (the record's heading keeps the aim); the wake and the mode-5 flip RE-DISPATCH the same substep; the kind-4 mode-6 leash reads impact (not home) with max(countdown,2) as the dive speed and countdown==0 as the ONLY mode-7 transition; the ENGAGE band geometry is point-blank RETREAT <0x60 / transition [0x60..0x80] / approach 0x80..leash with aim+FACING (no +0x80); the kind-4 RAW-px scale for the staging + stepper probes (the WIP applied Q13 to both — the critters never moved), the dominant-axis reads the ASKING critter (the WIP read critter[0]), and the sine-word reads are SIGNED (a u16 view loses every negative step). (3) THE LANDING-PRODUCERS NON-CLAIM: ZERO staging calls exist in the corpus kind bodies (0x413600..0x414600) — the §7j.17 "mode-6 landing stages 8×k6 + 5 splash + 0x18 rows" expectation belongs to the corpus-dead k7 body; both corpus mode-6→7 transitions write only the mode/counter. (4) THE EFFECT-ROW BANK (§7j.24/5): FUN_0041a14f modeled as the 80-row LRU allocator with 3 draws/row + 1 per overflow id row (k4 death = 8 rows, k5/6 = 12 — the stream-relevant part); the row CONTENTS (cos/sin values) are the modeled sine-word lookup, an E-only row-content approximation. (5) THE COVERAGE SPLIT: the critter bank (T2) + effect rows (T3) are E-ONLY (no EXD alias — coverage findings, never fabricated); the ALIASED observables are the RNG stream, the robot bank (the 0x68 damage + the melee damage/knock lanes), the projectile bank (the 0x68 fire cycle), and the score bounty. (6) THE BOUNTY GATE GAP: robot-owned critter kills need bullet records whose inline spawns do not exist (the S3-documented AI-order family E-gap) — the corpus deaths are script kills (attacker −1, no bounty); the gate is pinned by the §7j.24 decode only. Mechanics: S8.scen (T0/T1/T2/T3/TS, 121 records, chain b5ae3f8be91c7449, double-run byte-identical; the gunner at (18,13) — the FLAT row: a plateau marker puts the burst one z-level high and the §7j.23 z-box misses everything); corpus_s8_critter_engagement gates the lifecycle (the 16-critter staging census, the 0x68 fire frames, the gunner's hit-flash, the 9-death burst window, the 80-row effect turnover, the dive/dying/dormant tails, the survivors); differ_gate S8 row (cross PASS-WITH-NOTES, exactly the 2 S1-class + the critter/effect E-only pair, zero field gaps); capture-plans/S8.json compiled + byte-pinned (36 anchor + 27 per-frame, 1 command inject CS:0009255C, the markers/loadout/critters seams in _e_staging). The §7j.42 RE decode (b3e78cb + 05f0d95) landed by predecessor worker f9af5743 claim 2; the engine leg was left uncommitted by session death and ADOPTED + corrected + completed this run (8786c9e + the differ/plan/docs legs) (worker 40dd9473 claim 2)

Nudge-Worker: 40dd9473-24c3-49a9-8dc9-6e509af676d0

## D115 — 2026-08-23: P4.2/debris-physics — the FUN_0040de9c DEBRIS-PHYSICS family LANDED end-to-end: the tick lifecycle + the three collision walks in bedlam-core, the debris-damage observability pairing on S4/S7/S8, and the FIVE-chain re-baseline the physics turn-on forces. FIVE decisions recorded: (1) THE +0x20 SEMANTICS (§7j.44/1): the physics word is a COUNTDOWN, not a class index — the pass decrements it on exit and the tick gate stops calling at 0, so a phys-6 chunk moves/damages for exactly 6 frames; the 0x454510 dword table is NOT a param table for this function (no table read exists — knock_mult = min(phys,3), critter radius = min(16·phys+0x20,0x60), mag = kind==12?25:2 are all arithmetic in the CURRENT value). The 7j.11/5 census task is thereby CLOSED-by-disproof. (2) THE PRODUCER SURFACE the engine models: the robot lane (no gate — ALIVE ∧ state≠2 ∧ octile>>8 < 0x40 → the W12-S8 FUN_0040db9e dispatcher: mag damage + facing −1 + the ≤3-px robot_move knock), the terrain-gated critter lane (the 3-row plane-0 dword probe gates ONLY this walk; per-kind getter/setter scales; the §7j.24 crush dispatcher with the REGISTER-GLOSS CORRECTION — edx is the knock multiplier AND the sin/cos factor, ecx the 2/25 hp subtraction), and the POI squash lane stays E-ONLY documented (no POI bank staged engine-side — the §7j.18 .NME section-8 loader unmodeled). The death tail in the robot lane stages five k5 per kill (delay 2k, param −1). (3) THE RE-BASELINE SCOPE: landing the pass turns physics-class chunks into MUTATORS on the aliased T1 robot bank, so every scenario that stages one moves its chain — NOT only the destroy-staging S4/S7: S3 moves via the MINE/GRENADE EXPIRY k12/k3 chunks (0xE/0x13/0x17/0x1A/0x1F weapon records disburse to kind 12 = the mag-25 family, rocket to k6; the 50% k11 artillery gate stays phys-0 = no physics), S5C via the burst-spread chips, S8 via the fire-window spread. Pinned: S3 e29f76f5585401e1 → 9a11efa03baafb64, S4 2ddd15ea50c8a14d → 35fa3a9234cbff37, S5C e0999fcb3455d3ef → 786fd87565b67f4a, S7 b41db389f3ad8947 → ecdce5472df6a324, S8 b5ae3f8be91c7449 → 44d806b81bd1b1ff. S0/S1/S2/S5/S5B/S6 chains BYTE-IDENTICAL (no physics debris on their paths — the staging-key discipline holds; the tick lifecycle itself never hashes, the debris ring is the T3 surface). (4) THE S5C CONSUME-ORDER FLIP: the burst-spread chips knock both stacked robots px-level before the order, so the case-3 probe order flips — the GUNNER (r3) reaches the cell first (heals the exact +2500, 1246 → 3746 unclamped) and the WALKER becomes the unhealed negative control; the heal VALUE stays exact — the scenario's purpose is intact, and an O1 capture arbitrates the flip (recorded as the expected live-session question). (5) THE OBSERVABILITY PAIRING (the queue's ask): census FIRST, then extend exactly where debris damage lands in-scenario — corpus_s4 (the knock-widened cascade: 15 destroyed, the freed debris ring at 60 live records — the tick now frees finished chunks, the old never-free 128 saturation is gone; the tick lifecycle is the §7j.44 observable), corpus_s7 (the standing gunner's chunk-field schedule: 19 hp-change frames f32..f50, 1248 total debris spend, static through the creep tail), corpus_s8 (the burst-window chips: −2 after each 0x68 hit pair, end 3041). No new S-variant needed — the S7/S8 paths already arm physics-class bursts (the queue's conditional satisfied on the "if debris damage lands" branch). The differ contract is unchanged (the debris-stager T3 row stays E-only; the physics lanes surface through the ALREADY-ALIASED robot/critter banks — zero new rows, zero field gaps). The §7j.44 decode (d467471) + engine legs (cebc178: mission.rs epilogue slot + destroy.rs tick/pass/lanes + critter in-crate gates + destroy_gate +5) landed by predecessor worker a5ef2370 claim 2 which died at session end mid-re-baseline; the uncommitted gate pins were ADOPTED + INDEPENDENTLY re-verified + COMPLETED this run (b2c89af: the re-baseline commit with the damage-lane assertions + one fmt fix) (worker 07ce0c25 claim 2)

Nudge-Worker: 07ce0c25-20b0-4402-a9d2-510d2c4dd925

## D116 — 2026-08-23: P4/RE — THE RE-EXW-SIM §9 ITEMS 2-3 REMAINDER CLOSED (docs-only, §7j.45): FUN_00440e45 = THE SHOP SCREEN (full decode) + the robots() extra-phase/timer semantics + the state-1 producer census. SIX decisions recorded: (1) THE SHOP IDENTITY: the §7d/RE-EXW-MUSIC hypothesis is CONFIRMED instruction-exact and RESOLVED to SHOP-ONLY — the entry loads GAMEGFX\{DARKPALS.PAL, WEAPICON/CONLITE/SHOPFONT/SHOPLITE.BIN} + the BEEP1/4/7/5 SFX cells + "SOUND\MIDI\SHOP", plays the "GAMEGFX\SHOP.SMK" intro gated by NEW PIN [0x46cca4] (the animations config flag; SMK-off loads GAMEGFX\SHOPPAL.PAL), and returns 0 = continue / 1 = abort — retiring GAMETHREAD's "[inferred] zone/level manager" glosses. (2) TWO NEW CAMPAIGN STATE FACTS: the MONEY FLOOR ([0x46ae70] := max(·,100) at every shop entry) and the MP/zone-7 LOCKOUT ARRAY (0x46cd48..0x46cd80, 16 dwords := 1 when mode==2 ∨ zone==7; value 2 = transient, normalized at exit) — plus the 9-category SHOP CATALOG grammar @0x4ea288 (0xA0 blocks, 0x10 items: name/price/pack-ammo/avail; cat 8 = the 5 equipment chassis 0x2A..0x2E with the 0x2D/0x2E mutex) staged from immediates by FUN_0044395b — fully regenerable for any future engine-side shop seam. (3) THE WEAPON-GROUP LAYOUT CORRECTION (§7d.2): the 7 words are +0 name, +2 ammo, +4 shop-artifact (unconsumed — the auto-loadout stores the robot TYPE there, a live-register artifact), +6 price, +8 category, +0xA item, +0xC owned; §7d's "price, category, item_idx" sat one slot low. The chassis table 0x4deafc (2 rows/type) shares the layout and its +2 word = the SHIELD-CHARGE count. (4) THE MP SHOP SYNC CONFIRMED (the queue's hypothesis): the exit appends the type-4 SHOP-LOADOUT COMMAND record via FUN_00449c94(4, 0x4e43e0) (the 63-B staging struct MissionShell 0x44853e + the save path 0x4475fd consume) and then walks players p < [0x46cbe0] (D89's COMMAND-record count/MP robot override) copying each 0x80-stride record's 7 (name,ammo) pairs into 0x4de664+p·0x62 — a FOURTH weapon-table writer family beside shop/save-load/lobby. (5) THE ROBOTS() FIELD IDENTITIES (§9 item 3): +0x32 = the BURN cooldown (:= 0x64 by FUN_004100b7's scorch lane, gate ==0 — scorched tiles re-burn every ~100 frames; +0x30 = the paired −0xA/phase-1 accumulator), +0x34 = the ALARM cooldown, +0xA4 (0x4c6a88) = the alarm COUNTER and EXW DOES decay it 1/frame in the phase-0 pre-pass (the D90 question CLOSED; the queue's "0x4c6a8c" tail has ZERO sites — the intended pair was +0x88/+0x8C), +0x88 = SHIELD POINTS (−2/frame; 0x20 per charge/state-3; 0x2710 INVULN while the +0xA0 flash runs — with the player-robot palette strobe ladder), +0x8C = the SHIELD CHARGES (spawn = the equipment-chassis row word+2 via the 0x40cc8c 5-slot jump table; a hit with charges≠0∧shield==0 consumes one), +0x70 = the REINFORCEMENT delay with NEW PIN [0x4de658] = the pending-arrival gate (:= 0x80 at the threshold arm; the dword 0xC below the weapon-table base), and the 0x7d3 tile gate CORRECTED to the countdown-dependent bound phase ≤ (+0x80==0 ? 2 : 4). (6) THE STATE-1 CENSUS: COMPLETE word-write census of +0x0C — EXACTLY ONE producer of state 1 exists, the FUN_00409138 COMMAND-bit0 arm (0x40a37b, := 1 + stop := 0xF4240); there is NO patrol semantics, and SP never produces state 1 (why the S6 walk needed the COMMAND-inject seam, D112). Engine consequence: NONE (docs-only — the P4 slice already models the command arm + the shield family host-side). Verified: registry_anchors green, manifest clean before and after the read-only string probes, no Ghidra run, no corpus write (worker c607288e claim 2)

Nudge-Worker: c607288e-0f78-44de-8aee-c6ebdc03cffd

## D117 — 2026-08-23: P4/RE — THE FUN_00433980 PER-ZONE CASE TABLE + THE FUN_00424a6f MESSAGE SYSTEM CLOSED (docs-only, §7j.46; the 7j.19 item-6 residual + the promoted Backlog bullet retired). FIVE decisions recorded: (1) THE DISPATCH STRUCTURE: the zone switch is a 7-entry table @0x433964 (A..G → 0x43399f/0x434058/0x435bda/0x4386c5/0x432c8e/0x439323/0x439ae2) on zone−1; each zone gates MODE [0x4edb88] (SP/Coop share tables; H2H gets missions 1..2 rides-only blocks) then MISSION [0x4edd88] (cascade or 5-entry table: B 0x4331d0, D 0x433650, F 0x433950); each mission block probes FUN_00422e5e on the robot x/y/z (bank 0x4c69e4 stride 0xA8) and dispatches on the .PAD slot id via Watcom binary-search cascades or slot tables. The flat exw-text-objdump MISPARSES the table farm 0x43301c..0x433963 — clean objdump windows were required (method note for any future walk of this region). (2) THE RIDE-RECORD BANK: the 7j.19/7j.21 "dword tables 0x4dcdbc..0x4dd330" are ONE 0x24-stride record bank {+0/+4 dest tile-x/y, +0x18 countdown latch :=10, +0x1C rider-in-use gate −1/idx}; 16 records pinned (gates 0x4dcdd8..0x4dcff4); arm = state :=2 (in transit) + +0x74 :=0 + +0x84 := arrival platform 0..0xE + orders −1 + pos := dest·0x2000+0x1000 (x in-block, y via shared tail 0x43475f). (3) THE ACTION CENSUS: 21 SP BEACON slots (A M1 0x10; B 0x18/4/1/0/8; C 0xA/0xE/0x15/0x16/0x3D; D 8/7/0xF/0x10/9; F 0x12/0x11/0/0x15/0x1A — cross-checking 7j.20's "~25 pairs"), EXIT activations = DOOR+FUN_0041fa51 PAIRS in zones F (4+ per mission) and G (M1 slot 0), NOT the old single "case 0x1B" (that gloss retired); zone E = VERIFIED NEGATIVE (its entry lands in the 0x430b27..0x433030 rect/dest overlay-restamp family and EXITS — no probe, no cases, no beacon; with the 5-pop exit-thunk 0x426030 vs the 6-push prologue recorded as a compiler-shared-sibling quirk, most plausibly a never-exercised SP path). (4) THE MESSAGE SYSTEM: FUN_00424a6f = the ZONE-A-M1-ONLY message shower (sole caller 0x433d07), SP-only, per-id show-once latch word 0x4eb5f8+2·id := 1; the "string table" is the LANGUAGE.{ENG,GER,SPA,FRE,ITL,DCH} FILE blob [0x46cbb4] (alloc 0x13C68, load 0x41c1fb, lang selector [0x4eba1c]) scanned for [BOOT_CAMP_%03i] sections by FUN_00424679 — LANGUAGE.ENG = 421 sections (BOOT_CAMP_000..014 = exactly the 15 zone-A M1 message slots; OBJECTIVE/MARKER/OVERVIEW/CREDIT/MENU_ITEMS/WARNINGS feed the briefing readers, NOT the dispatcher); box = MONOFONT 0x46cdb0 word-wrap, window 0x4eaab8 {x=0xF0−w/2, y=200}, bank 0x4e8818, SFX TEXTBOX1 0x4edfd0. (5) THE LATCH/TIMER SEMANTICS (the queue's ask): [0x4eaac0] := 0xFDE8 at show; FUN_00425010 = the per-frame ticker/drawer (sole caller MissionShell 0x448381) decrements it; the FUN_00409138 COMMAND sites 0x40a2bc (≥8 frames)/0x40a396 (≥44) DISMISS the box on a player command; 0x40c570 gates the state-0 robot write while the box shows; MissionShell 0x44790f resets at mission start. CLARIFICATION: the producers' cited "msgs 9/10/0xB/0x1C..0x21/0x26-0x29" are FUN_004239ef SFX ids, NOT text messages — the text-message producer set is exactly the 15 BOOT_CAMP cases. Engine consequence: NONE (docs-only; no new watch rows — the latch/timer/window cells are SP-UI presentation; the pad-case semantics are already modeled through the S6 beacon armer, the .PAD probe, and the exit activator seams). Verified: manifest clean before AND after the read-only probes, no Ghidra run, no corpus write; the FULL generated case table is committed as RE-EXW-SIM §7j.46 8-bis (worker 0c2df9b4 claim 2)

Nudge-Worker: 0c2df9b4-0478-47b9-a446-4a167007ddda

## D118 — 2026-08-23: QUEUE HYGIENE #3 — the queued ".BDG TEMPLATE-BANK ↔ RESTORE-WORD MAPPING" item REMOVED AS ALREADY-CLOSED (D96/§7j.32, 2026-08-22): its text was stale pre-D96 state copied from the Backlog's own RETIRED-D93 bullet; the closure re-verified genuinely green at HEAD with a FRESH parser + fresh objdump greps, every leg reproducing byte-identically. FOUR decisions recorded: (1) THE CATCH + THE LESSON EXTENSION: item 2 as queued ("the @+0x3E/+0x42 template-bank readers are STILL OPEN") described the state BEFORE §7j.32 landed (2026-08-22, worker ce347a0e, commits 4210f55 + f554bee); the closure sat in BOTH the Done log AND D96 at queue-write time (the 23a33f8 close-out). The item text was lifted from the Backlog's RETIRED-D93 bullet, whose parenthetical "+the .BDG template-bank plane↔mirror-word mapping … @+0x3E/+0x42 readers still open" was itself stale (written pre-§7j.32 the same day and never annotated closed). D111's pre-write grep covers the Done log + DECISIONS headline — but a COPIED stale parenthetical defeats that grep. LESSON: when a queued/promoted item's text is copied from an old bullet, the bullet's own parenthetical must be re-checked against DECISIONS before the copy lands (and stale closed clauses inside retired bullets get annotated CLOSED in place — done this run, so this source cannot tempt a third re-queue). (2) THE INDEPENDENT RE-VERIFICATION (fresh evidence, not a re-read of §7j.32's notes): (a) the loader's bank DISK ORDER instruction-exact — the four cursor-marching reads ([0x46ad5c]) store slot +0x3E first (0x41a727), +0x46 (0x41a742), +0x42 (0x41a75d), +0x4A (0x41a77c) → disk order +0x3E,+0x46,+0x42,+0x4A; (b) the destroy restore instruction-exact 0x41a9c3..0x41ac0b: +0x46 bank word → the TOT-mirror plane word @0x4796bc+0x1E·tile+2z (load 0x41ab59 → write 0x41ab6b), +0x4A word → seen @0x4796cc+z := (word==0) (0x41ab72 → 0x41ab80, sete), +0x4A&0xFF → the DAT volume byte @[0x4edd58]+z·w·h+tile (re-load 0x41ab8a → write 0x41abdb), the template linear index (z'·H+i)·W+j (accumulator at 0x41ab49..0x41ab55); (c) the ZERO-READER census both legs: absolute 0x4dee30/0x4dee34 = the loader's own two stores ONLY, and the whole-objdump displacement scan finds 6×[reg+0x3e] + 12×[reg+0x42] sites with NONE type-table-relative (the 0x43b4xx/0x43b5xx pair family, misparsed data bytes at 0x420223/0x4205f7/0x4248ab/0x424ce2/0x425d2f, the 0x425d88 rcr + the 0x43302b/0x433537 overlay-restamp adds, [esp+…] stack forms) — cross-checked against the re-enumerated 0x4dedf2 base traffic (19 sites: loader ×4-5, stamper 0x41a857, resolver 0x41a9d7, chain-walk 0x41b7e4..0x41bbe6, rubble-draw 0x408ce9, minimap 0x41f65e, MissionShell 0x448804..0x448d3b — none touches +0x3E/+0x42); arena 0x46ad5c = loader-only (10 sites, 0x41a5ec..0x41a793); (d) the CORPUS ROLE PROOF with a fresh parser built from FORMATS §2/§4/§12/§16 alone: ZONEA/M1 = 282 BDG records (197 non-empty), 211 BDG-typed instances (213 used .POS slots − 2 idx=0xFFFFFFFF slots 211/212), 435 footprint cells — bank1(+0x3E) ≡ shipped TOT word 434/435, bank2(+0x46) 11/435, bank3(+0x42) ≡ shipped DAT byte 434/435, bank4(+0x4A)&0xFF 155/435, and the single bank1 miss IS the documented overlap cell (14,29,z1): slot 97 (idx 63, 1×2×3, b1=806) vs slot 207 (idx 0, 1×1×1, b1=53), shipped 53 = last-.POS-slot-wins. ALL FIVE numbers byte-identical to §7j.32 items 4-5. (3) METHOD NOTE (new; the one genuinely new artifact of this unit): the TOT word-plane reader's header is NATIVE-UNIT — §2's "u16 w + u16 h + 8 × w·h u16 planes" means the planes start at BYTE 4 = WORD 2, and a byte-unit `4 + wordoff` then ×2 double-counts the header into an effective 8-byte one; this run's own first pass produced a FALSE 67/435 alarm exactly that way while the u8 DAT path was immune (plane-major w·h stride, y·w+x within plane; ZONEA header w=25, h=75). Any future TOT-plane probe should reuse the corrected word-unit addressing. (4) VERDICT: every deliverable the stale item listed already existed at HEAD (the reader-side anchor census = §7j.32 items 1-2; the plane↔word mapping = §7j.32 items 3-4 + FORMATS §16 TEMPLATE-BANK SEMANTICS; the §17 + §2 notes; the ledger row ".BDG template-bank semantics") — engine consequence NONE, no new watch rows; queue item 2 removed and the TOT plane-6/7 semantics unit queued in its place (pre-queue grep performed: zero closure of "plane 6/7" in DECISIONS/RE-EXW-SIM/Done log — the D118 discipline applied to its own successor). Verified: registry_anchors 2/2 green, MANIFEST.sha256 clean before AND after the read-only probes (objdump greps + the corpus reads), no Ghidra run, no corpus write (worker e26508a9 claim 2)

Nudge-Worker: e26508a9-1db4-432a-b843-b96426ffa541
## D119 — 2026-08-23: P4/RE — THE TOT PLANE-6/7 SEMANTICS CLOSED (docs-only, §7j.47; the FORMATS §2 open item + the D118-promoted Backlog bullet). FOUR decisions recorded: (1) THE RENDERER VERDICT: plane-6/7 mirror words DO draw — NO z≥6 gate exists in ANY consumer family. The terrain z-stack draw loop inside FUN_00403938 (0x4067cf..0x406c73, consuming the [0x4ede24]/[0x4ede28] restamp list) runs z 0..7 (outer `cmp ebx,0x8` @0x406863/66, chain `cmp [esp+0x3c],0x8` @0x40695c) with the Block-1 restart draw (0x406882..0x406941) gated on mirror word≠0 ALONE (NO seen check — `cmp WORD [eax+0x4796bc],0; je` @0x406891), so when the Block-2 contiguous chain (seen∧word, 0x40696e/0x40697b) breaks at plane m, the outer z catches up to m and re-fires Block 1: EVERY nonzero plane word 0..7 draws (seen only short-circuits the fast path); the stack cursor k ([esp+0x3c]) resets to 0 per record (@0x406c00/08, `xor eax,eax` unconditional on both door-tag branches); screen culling is plane-agnostic. init_tiles stages all 8 planes (z loop `cmp esi,0x8` @0x407fce). The adjacent walkers are equally unbounded: the overlay scanner 0x408a49..0x408ade (planes 1..7) and the per-plane range consumer 0x42035c..0x4203a5 (planes 0..7, the [0x454ae4+4·zone] window → FUN_0042394a). (2) THE CORPUS CENSUS (37 missions, full sweep, D118 word-unit addressing): 36/37 missions carry plane-6/7 words (only ZONEG/M1 zero); 8 016 + 2 882 nonzero words in 9 296 cells (6 414 p6-only / 1 280 p7-only / 1 602 both); 6 504 overlay cells (nonzero at planes 0..5) vs 2 792 standalone (floating z=6/7 sprites, drawn by the catch-up path); value domain IDENTICAL to planes 1..5 (p6 35..1868, p7 36..1868 vs p1..p5 33..1868); DAT bytes at the words overwhelmingly 1 (seen=1 at load for ~93%). THE TALL-TOWER SHAPE: the words are per-level sprite ids of multi-storey structures — ZONEA/M1 tile (17,25) = the one zone-A cell, column [454,1354,1355,1356] at z=4..7 (the famous \"1355/1356 adjacent integers\" are the z-6/z-7 sprite ids); ZONEB/M1 (88,19) tops at 1868; 1755→1753 descending ramps; 1153..1161 sequential multi-tile runs. (3) THE ~2000-ENTRY TARGET-TABLE HYPOTHESIS REFUTED on three legs: the \"≤1868 just under 2000\" nearness is a property of the tile-word grammar (planes 1..5 reach 1868 too); resolving every plane-6/7 value as a .POS slot gives 9 217 live / 1 681 empty in their own missions (coincidental live-fraction, not a linkage — ZONEA's 1355/1356 hit EMPTY slots); and the words are consumed as sprite ids by the ordinary stack draw path (p7==p6+1 at only 83/9 296 cells). FORMATS §2 paragraph + the §12 cross-reference + the cross-file LIKELY row all rewritten CLOSED; §7j.47 is the anchor. (4) ENGINE CONSEQUENCE: NONE — E already stages every nonzero plane word (D107 stage_pickup_surface), so planes 6/7 are uniform today; no new watch rows (the words live inside the existing typedb-mirror/TOT rows); a future tall-tower walk scenario exercises the same chain with robots() probes z-bounded by their own families, not the draw stack. Verified: registry_anchors 2/2 green, MANIFEST.sha256 clean before AND after the read-only corpus probes (TOT/DAT/POS of all 37 missions), objdump-only (no Ghidra run), no corpus write (worker f29066bd claim 2)

Nudge-Worker: f29066bd-d224-445a-a0b1-030d649cedfd

## D120 — 2026-08-23: P4/RE — THE MISSIONVIEW §5d TAIL CLOSED (docs-only, §7j.48; the D119-promoted item 2; adopts + validates the interrupted predecessor WIP in docs/RE-EXW-MISSIONVIEW.md §5d — every claim re-verified against the objdump before landing). FIVE decisions recorded: (1) TWO §5d LABEL CORRECTIONS, re-anchored to the 7j.28/7j.30 corpus-string census: cell 0x46af38 = TELEPORT.BIN (10 imgs, corpus-verified u16 count) — the state-5/6 draw (mode 0x12e, sy−0x48, clamp(10−wobble/4, 0..9) @0x403de6..0x403e71) is the TELEPORT BEAM not a "shield" (the 0..9 clamp fits the 10-image bank); cell 0x46af44 = SHIELD.BIN (4 imgs) — the +0x88-gated draw (frame u16@+0x18, mode 0x12c @0x403ef4..0x403f29) is the SHIELD not a "variant sprite" (4 frames = the RandA()&3 spawn init + the (+1)&3 post-loop shimmer @0x403cf7 — a variant would not shimmer on a 2-bit cycle). (2) THE STAGING VERDICT: alloc + load at EVERY MissionShell head, unconditional — single call sites on the straight-line head of FUN_0044771c (0x447860 → FUN_0041d954 arena pass: TELEPORT 0x6d60 / NUMBERS 0xfa0 / FLAGS 0x3a98 / ROBNUMS 0xbb8 / SHIELD 0x1b58; 0x447b3f → FUN_0041df10 LoadFile pass: TELEPORT@0x41df99, SHIELD@0x41dfe9, ROBNUMS@0x41dff9); NO mission-type gate, NO MP gate — a SP frame can never observe any of these cells == 0 (the 7j.30 "GAMEGFX load-always" = load-always per mission). TINYFONT [0x46cdb0] = ArenaAlloc(0x189c) @0x41d62f in FUN_0041d4e9, loaded in the same pass family. (3) ROBNUMS.BIN IS DEAD DATA: full-binary census gives [0x46af48] exactly ONE reader = its own LoadFile site 0x41dffe — staged, never drawn (cut feature; the name says the digits were meant for robot-number plates). The actual plate font is TINYFONT (118 glyphs, 0x21-based: glyph index = ASCII − 0x21; shared with the map markers 0x408fe6/0x40907a and the sidebar text 0x423cd9/0x423cf7). (4) THE FULL NAME-PLATE GRAMMAR: gate [0x4edb88]≠0 @0x403fb9 (any MP/demo mode; SP never — the ==2 arm @0x403c62 is the DIFFERENT 7j.31 MP hot-rect consumer); per char i < strlen: glyph g = [0x4e4458 + id*9 + i], skip if g > 0x40 (the jl arm is dead — g zero-extended), enqueue TINYFONT frame g at x = sx + u32[0x4e44c8 + id*4] + 6·i, mode 0x12c; 0x4e44c8 is the id-indexed CENTERING table (NOT per-char): writer 0x447ce0..0x447d85 in MissionShell — memset slot, toupper-copy (FUN_0044f067) of the raw name (0x4e43e0, 9 B/slot, [0x46cbe0] slots) storing c−0x21, then u32[id] = 0x20 − (strlen·6)>>1 = 32 − 3·strlen; the ≤0x40 filter passes every glyph the toupper grammar produces. (5) THE UNSTAGED-FLUSH CLAUSE RETIRED: NO unstaged-skip exists anywhere — FUN_0040798e's only early-outs are bx/by < 0 (bank stored in the node untested), FUN_0040179b's only skip is the unknown-mode RET @0x4017e0 (mode-based, not bank-based; every drawn mode derefs the bank directory unchecked) — an unstaged bank would FAULT, not skip, and per (2) it can never happen in shipped play; E-side consequence: the renderer needs NO unstaged-skip logic, staging may stay lazy per bank, the seam is unobservable. No new watch rows (plate glyphs are MP-only; SP chains never touch them). Verified: objdump-only (no Ghidra run), read-only corpus probes (game-data/BEDLAM/GAMEGFX TELEPORT/SHIELD/ROBNUMS/TINYFONT headers: 10/4/9/118), MANIFEST.sha256 clean before AND after, registry_anchors 2/2 green (worker 328b7651 claim 2)

Nudge-Worker: 328b7651-7d1f-4433-9fc5-6957650a7808

## D121 — 2026-08-23: P4/RE — THE FUN_00440dc2 IDENTITY CLOSED (docs-only, §7j.49; the 7j.26 Backlog "REMAINS open slim" clause). FIVE decisions recorded: (1) THE CALLER CENSUS IS COMPLETE AND CLOSED: FUN_00440dc2 has EXACTLY ONE call site (0x43dfb3, inside FUN_0043dc65 = the per-OBJECTIVE brief panel renderer), and a raw little-endian dword scan of the whole EXW file for 0x00440dc2/0x00440a2d/0x00440c34 returns ZERO hits — no jump-table or function-pointer refs; the call graph is a strict closed trio FUN_0043dc65 → FUN_00440dc2 → {FUN_00440a2d, FUN_00440c34}, one site per edge. The "tail jumps into the caller" red flag that kept the clause open is DECODED: 0x43c801/0x43c802 and 0x43f49e are MULTI-ENTRY SHARED-EPILOGUE GADGETS (Watcom -ox) — 6-pop/5-pop variants of pop ebp/edi/esi/edx/ecx/ebx + ret — so all three functions return normally; every jmp 0x43c801/0x43c802 site across 0x43c8..0x4472 is another function sharing the gadget. (2) THE IDENTITY: FUN_0043d00b = the MISSION BRIEF screen (GameMain 0x41c4d5, ret 2 = launch → [0x46ae74]); it allocates its OWN buffers ([0x46cbb0] := alloc(0x10100) 256×256 cache, [0x4ede18] := alloc(0x64000), restamp list [0x4ede24] := alloc(0x24c)) and zeroes the OBJECTIVE BANK 24×14 B @0x4e9628 (+0/+2 marker x/y, +4/+6 TOT row/col, +8 counter, +0xA render-current latch; staged by the BRIEF text parser 0x43e5b1..0x43e7b2; panel name "OBJECTIVE_%c%c" from [0x4edd8c]/[0x4edd88], strings 0x4592b6/0x4592c1). FUN_00440dc2(record) = the OBJECTIVE-MINIMAP SNAPSHOTTER: FUN_00440a2d(row,col) stages the 49-entry restamp list + materializes TOT→mirror (7×7×8 window) + ZEROES the whole 0x64000 backbuffer; FUN_00440c34 draws each record's 8-z stack (dest = list dest − z·0x5000 = 32 px/level iso height, bounds [bb, bb+0x5a000), FUN_00401471 EBX=0 — the 7j.36 sites 0x440d1c/0x440d93 live HERE, not in FUN_00440dc2); then a plain 2× DOWNSAMPLE dest[r][c] = bb[(64+2r)·0x280 + 64 + 2c] (the same (64,64) window geometry the mission present reads, MISSIONVIEW §7) into the cache; [0x4dc6c0] := 1. Consumer 0x43d9a2: flag-gated FUN_00402a28 = transparent [0x4edbfc]-remapped 256×256 blit into the panel buffer. The `mov ecx,0x10000` @0x440de2 is the pre-set zero-fill count (Watcom scheduling), not a stager arg — the stager clobbers ECX and both callees preserve it. (3) THE FRAME-FLOW/MID-FRAME QUESTION (§1 ordering) CLOSED BY SCREEN LIFECYCLE: FUN_00403938 is called ONLY from MissionShell (0x447c9b/0x448094); FUN_0043d00b never calls it; the two screens hold SEPARATE [0x4ede18] allocations — FUN_00440dc2 CANNOT wipe a mission frame, and no in-game invocation path exists at all. (4) GLOSS CORRECTIONS LANDED (history preserved, pointers added): FUN_00440a2d is NOT "the scroll/camera restamp stager" of the in-game viewport — it is the BRIEF minimap window stager; the [0x4ede24] cell is a PER-SCREEN REUSE (BRIEF 49×12 list vs mission 1296×12 viewport cache from FUN_0041d954 — the in-game render-tail list producer); the 7j.36 cluster-(b) "restamp drawer FUN_00440dc2" = FUN_00440c34, BRIEF-only; the §7j.16 "materializer caller — scroll restamp?" lead RESOLVED. The in-game restamp mechanism itself (render tail 0x406a8c..0x406c73) is untouched by this correction. (5) ENGINE CONSEQUENCE: NONE — the BRIEF screen is outside the P4 mission-diff scope; cells 0x46cbb0/0x4dc6c0/0x4e9628 are BRIEF-lifecycle only; no new watch rows, no E-side work. Verified: objdump-only (no Ghidra run), read-only DGROUP string probes + raw-dword pointer scan of BEDLAM.EXW, MANIFEST.sha256 clean before AND after, registry_anchors green (worker 21c18e9e claim 2)

Nudge-Worker: 21c18e9e-8dfd-4b8c-86e1-4a07f579f032

## D122 — 2026-08-23: P4/RE — THE PROJECTILE-TYPE-0x69 DAMAGE-TABLE RESIDUE CLOSED (docs-only, §7j.50; the 7j.18 "NOT folded" note + the queue's "(d+1)·300 as type 0x66" hypothesis resolved). FIVE decisions recorded: (1) THE ELSE PATH IS DUMPED — FUN_00419aff has NO memory table (base/stride N/A): it is a compiled binary jump tree with `eax := 1` pre-set at entry (0x419b05) and the ELSE = the plain fall-through stubs (0x419b57 [w>0x68], 0x419c2c [0x2A..0x64], 0x419c50 [0xE..0x19], 0x419c5e [6..11]) plus a Watcom CROSS-FUNCTION SHARED-EPILOGUE gadget at 0x418aa1 (pop esi/edx/ecx/ebx; ret — pops exactly FUN_00419aff's own four entry pushes; same family as the 7j.49 multi-entry epilogues) reached by FIVE arms: two carrying the default 1 (w<2, 0x1B..0x23) and three carrying the d≠2 difficulty products (0x65→50·(d+1), 0x66→300·(d+1), 0x67/0x68→75·(d+1) — the 75·(d+1) staged via the EBX clobber 0x419b34..40, ECX carrying the d=2 constant 75·(d+1)+0x4B=300). The §7j.17 key table re-verified instruction-exact, incl. the w≥0x69 final else. (2) THE 0x69 VERDICT: the per-level BEAM column is a 0x4cc654-bank STATE (producer = the k7 close-combat leg @0x4135a2 behind the d-indexed fire-rate gates 32/16/8 frames, {z=6, TTL 0x18, +0x1A=0}), and its impact handler (FUN_00412010 arm 0x412042) NEVER queries the table at its own id — it passes the LITERAL weapon id 0x65 (0x41215a) → 50/100/200 by d via FUN_0041a894, terrain-only. NO caller anywhere passes 0x69 (29-site census); the hypothetical else-1 never materializes; the §7j.16 "(→ 'else 1')" guess CORRECTED. (3) THE "(d+1)·300 as type 0x66" HYPOTHESIS REFUTED for 0x69 — that key belongs to the TRT-bolt state 0x66 alone (producer FUN_00417698 @0x417a5c, `[eax*2+0x4cc654]` scaled write; a GUIDED STEPPER ≤10 substeps/frame with contact classes 1/2/3 — class 2 terrain contact damages via 0x412449 key 0x66 + FUN_0041bc1c; classes 1/3 die) — and 0x66 NEVER damages robots: FUN_004197d4 (the projectile-vs-robot proximity walker, |dx|<0x10 ∧ |dy|<0x10 ∧ |dz|<0x20 Q8) admits states 0x65/0x67/0x68 ONLY. Complete state-word census (25 sites): 4 readers, 12 zero-writes, exactly FIVE producers (k2 0x65 @0x41540e, TRT 0x66 @0x417a5c, k3 0x67 @0x414b79, k5/6 0x68 @0x413def, k7 0x69 @0x4135a2); the tick dispatch = jump table 0x411ffc on state−0x65 ∈ 0..4. (4) THE PER-STATE IMPACT-KEY MAP supersedes the §7j.14 "reads the projectile's own type" gloss (only 0x67/0x68 self-key, via the `[+0x4cc652]>>16` dword trick @0x4122f7/0x41992d): 0x65/0x69 → literal 0x65 (terrain), 0x66 → literal 0x66 (terrain), 0x67/0x68 → own state (terrain AND robot); the beam additionally OSCILLATES its probe counter (k := min(k+1,7) at frame top, k−− on contact 0x4120e9) so a blocked level re-probes ⇒ the beam is a PERSISTENT per-frame terrain DoT (debris kind 0x14 + RandA±7 spread + SFX each contact), dying silently only at TTL 0; FUN_004126dc's 0x69 arm = silent shared-epilogue return (defensive; never invoked by the beam). (5) ENGINE CONSEQUENCE: the future E-side k7 leg must model the beam as per-frame 0x65-keyed terrain damage with the k-oscillation and NO robot damage; the 0x66 TRT bolt is terrain-only 300/600/1200. Docs-only — no code, no watch rows (the 0x4cc654 bank is T2-class). Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run, no corpus read), registry_anchors green (worker 6bb948aa claim 2)

Nudge-Worker: 6bb948aa-e263-4ca6-b485-1863c726b76a

## D123 — 2026-08-23: P4/RE — THE FUN_00419756 IDENTITY CLOSED (docs-only, §7j.51; the D122-queued residual "0x66 never damages robots — what DOES the bolt interact with"). FOUR decisions recorded: (1) THE IDENTITY: FUN_00419756(x,y,z Q13) = a first-alive ROBOT-BANK OCCUPANCY BOX — it walks the robot bank 0x4c69e4/0xA8 (count [0x46ccbc], ALIVE gate +0x7C ≠ 0) and returns 1 on the FIRST record with |Δ(x>>8)|<0x10 ∧ |Δ(y>>8)|<0x10 ∧ |z@+8 raw − z>>8|<0x20. Of the queue's four candidates (robot octile / critter bank / TRT structures / tile words) only the robot bank is right, and it is a BOX, not octile (no FUN_0041ebf8; plain per-axis abs compares); first-match-in-bank-order — a presence predicate, not a nearest-scan. (2) THE SCALE MATCH: all three axes normalize to Q5 (32/tile) — probe x/y Q13 (tile·0x2000) >>8, robot z@+8 STORED Q5 (§3 +0x08) so its raw read needs no shift; the thresholds are ±<0.5 tile lateral and ±<1 z level; FUN_004197d4's robot lane uses the IDENTICAL box (0x419856/0x419876/0x419893) — the §7j.13 item-4 walker and this probe share one geometry, and the apparent z asymmetry is scale-matching, not a quirk. (3) THE CLASS-3 VERDICT: class 3 IS the "hit an actor but no robot damage" leg — CONFIRMED and stronger: the path performs NO damage query of ANY kind (FUN_004126dc disburser → kind-8 debris + state := 0; no FUN_00419aff, no FUN_0041a894/0x41bc1c, no FUN_0040e230). ALIVE ROBOTS are a pure BLOCKER for the TRT bolt: it stops at the first robot box it enters and dies as cosmetic debris; its (d+1)·300 damage is EXCLUSIVELY the class-2 terrain/structure contact. Two §7j.50/6 gloss fixes landed with history preserved: the probe takes all THREE args (the "(x,y)" form corrected — ebx = the record's unstepped z), and the "vz ≠ 0 → break" leg only SKIPS the height probe (substeps continue; the §7j.16 spawn vz 0x14 reads as a ~2-frame terrain-arming delay, occupancy tested every substep); the write-back reverts the contact substep BEFORE the class dispatch, so class-3 debris spawns pre-contact. (4) ENGINE CONSEQUENCE: NONE today (docs-only; the 0x4cc654 bank is T2-class, no watch rows); when the E-side TRT fire routine lands, its stepper must reproduce the blocker box verbatim — without it bolts would fly through the squad and detonate on terrain behind (a death-position divergence), and the class-3 death must spawn kind-8 debris at the post-revert position with zero damage. Caller census: exactly ONE site 0x4123ae (exw-functions + full-objdump grep, no jump-table refs). Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run, no corpus read), registry_anchors 2/2 green, MANIFEST.sha256 clean before AND after (worker 9a23356a claim 2)

Nudge-Worker: 9a23356a-7e27-4f3b-a365-6776f708ded0

## D124 — 2026-08-23: P4/RE — THE DEBRIS ARRIVAL-SFX PAIR CLOSED (docs-only, §7j.52; the 7j.11 item-4 residue — FUN_00421e60 118 B/11 callers + FUN_00421dec 116 B/2 callers, the k20-tail / center-write legs' per-ring arrival consumers). FIVE decisions recorded: (1) BOTH BODIES INSTRUCTION-EXACT: FUN_00421e60(x Q5 EAX, y Q5 EDX) = gate [0x4ede58]==0 → shared-family epilogue 0x41dc51, else RandB() signed-idiv-3 → cells 0x4edf64/0x4edf68/0x4edf6c, play FUN_0043a48e(handle,0,x,y,priority 2); FUN_00421dec = same shape, RandB()&3 → jump table @0x421ddc {0x421e07/20/2d/3a} → cells 0x4edf98/0x4edf9c/0x4edfa0/0x4edfa4, play priority 1 — one voice-steal class BELOW the BOOM trio; the flat objdump MISPARSES 0x421dd9..0x421deb (16 B table + padding — decoded from the raw stream, the §7j.46 method note repeated). (2) EVERY CELL NAMED via the §7j.30 anchor (the queue's ask): BOOM1/BOOM2/BOOM3 and RICOCHT1/RICOCHT2/RICOCHT3/RICOCHT4. (3) THE RNG CORRECTION — the unit's one prior-text fix, landed with history preserved: §7j.11 item 4's "RandA()%3" named the WRONG draw; both bodies call FUN_004029b6 = RandB (state 0x4ede4c), matching the §7j.23/24 sibling trios' already-correct "RandB()%3"; RandA (FUN_00402975, state 0x4ede48) is drawn ONLY by k11's local ~50% play gate (al&1 @0x420e87). Differ classes: the bank pick = T4 (unmodeled, per the destroy-tail RandB language); k11's gate = a modeled RandA draw-count (matters only if k11 gains a corpus producer). (4) THE TRIGGER (the queue's ask): all 13 call sites live INSIDE the FUN_00420608 kind legs and fire at DEBRIS-STAGE time (entity creation, BEFORE the record fields are written — "arrival" = arrival on the field, not a landing tick); TWELVE of 13 share one shape — per-leg in-map bounds recheck of the raw Q5 args (x/y ≥ 0, x < [0x4eddec]<<5, y < [0x4eddf0]<<5; fail → ret-8, no record, no SFX) then the UNCONDITIONAL call; the 13th (k11 @0x420e93) alone adds the RandA&1 gate, drawing TWO different RNGs on one leg. Kind→leg mapping re-verified byte-exact against the 20-entry jump table @0x4205b8 (6+12 and 1+13/14/15 body-sharing confirmed). Caller census COMPLETE: raw little-endian dword scan of BEDLAM.EXW for both entry addresses → ZERO hits (no jump-table/function-pointer refs) — the 13 direct calls are the entire graph. (5) CORPUS REACHABILITY + ENGINE CONSEQUENCE: the only corpus-reachable producer today remains k5 via apply_damage (§7j.11 item 6) → the only reachable arrival-SFX site is k5's e60 leg @0x421364 (one RandB + one BOOM1/2/3 at the death position, priority 2, gated [0x4ede58]≠0); FUN_00421dec has NO corpus-reachable caller (k2/k8 live in the weapon-fire families). Engine consequence NONE today (docs-only, no watch rows — the cells sit under the existing sfx-master-gate/SFX-register rows); the future beyond-k5 E-side stager must draw ONE RandB per staging (T4) at the spawn position, k11 one RandA first, and the 2-vs-1 priority split is audible only via FUN_0043a48e's steal order (no dump-visible state). ADJACENT CENSUS (one line): a third sibling FUN_00421ed6 = the GRUNT1/2/3 trio (RandB()%3 → 0x4ee000/04/08, priority 2; callers 0x413ba0/0x413f2a, zero raw-dword refs) — the arrival-SFX family is now four decode-complete members (0x421dec, 0x421e60, 0x421ed6, + the §7j.23/24 twins). Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run), one read-only raw-dword scan of game-data/cd-root/BEDLAM.EXW, MANIFEST.sha256 clean before AND after, registry_anchors 2/2 green (worker a553aa84 claim 2)

Nudge-Worker: a553aa84-800d-4909-ab51-fcd29c76a79a

## D125 — 2026-08-23: P4/RE — THE FUN_004239ef SFX-MESSAGE DISPATCHER CLOSED (docs-only, §7j.53; the 17-citation "never body-decoded" residue + the Backlog "Mission SFX tier" select-ack/armer-click bullet). FIVE decisions recorded: (1) THE IDENTITY — FUN_004239ef(id, channel) is the RADIO-WARNING poster for a 4-channel message queue @0x4eb954 (stride 0x28: eight id+1 words +0..+0x1C, insert index +0x20 wrap 8, voice handle +0x24; dedupe scan per id per channel; ids 0x19..0x1B FLUSH their own channel then post at slot 0; whole queue + display ring MissionShell-zeroed 0x4479de/0x4479fc). Channels 0/1/2 = the three squad slots (UNIT 1/2/3), channel 3 = system/HQ — drained FIRST by the consumer FUN_00423a85 (MissionShell @0x447ff5, once per frame, channels 3→0, oldest slot first, ONE message per channel per frame): the voice leg (skipped for text-only ids 0xF/0x29; gated [0x4eb93c] audio handle ∧ [0x4ede5c] ∧ [0x4ede58]) keeps the slot queued while FUN_0044c5ac(handle−1) reports playing, else starts the take pick **A/B = RandA (FUN_004029b6) bit0** from speech record 0x4ee014+8·id via the 0x44c8c4 bypass (vol 0x7f00), handle := ret+1; the consume leg clears the slot, rolls the 4×0x26 on-screen display ring @0x4ea13c {text[0x20], typewriter reveal u16 +0x22, valid u16 +0x24}, and stages the id's TEXT from 0x46c18c+id·0x30; the render tail (same function) draws the 4-line staircase with per-record reveal counters (char tables 0x454c20/0x454b70, ≥0x80 remap FUN_00410493). (2) THE 53-ID → LINE MAP IS CORPUS-NAMED, not guessed: the text table is GameMain-loaded (0x41c2ff) from the **[WARNINGS]** section of the active LANGUAGE.* file (name string 0x457ac9; sibling [MENU_ITEMS] 0x457abe → 0x46af5c), and ALL SIX locales (DCH/ENG/FRE/GER/ITL/SPA) carry exactly 53 line-records in the same order — every one of the 55 call sites reconciled to its line (0/1/2 HAS NOW ARRIVED = the §7j.19 per-player release posts; 3..8 heat; 9/0xA/0xB ARRIVAL IS IMMINENT = the §7j.20 pod arm; 0xC..0xE+0xF the DANGER-TARGETTED + AERIAL-BOMBARDMENT pair; 0x10..0x18 hits/half-power/critical; 0x19..0x1B IS TOAST = the flush triple; 0x1C..0x21 weapon-change/out-of-weapons; 0x22 fence-off; 0x23/0x24 section raise/lower; 0x25 "X" placeholder with ZERO sites; 0x26/0x27/0x34 objectives; 0x28/0x29 CONGRATULATIONS pair; 0x2A EVACUATION COMMENCED = the §7j.20 armer "click"; 0x2B..0x33 battery/damper/ammo). FORMATS §22 added (the LANGUAGE.* container grammar). (3) TWO GLOSS CORRECTIONS LANDED with history preserved: §7f.6's "select SFX FUN_004239ef(0xC+k)" — nothing there is a select sound; the 0x40c1c1..0x40c24f posts are the DANGER/bombardment WARNING pair and the accompanying [0x4dc5d0] := slot+1 is the attention-draw (the cell facts of §7f.6 stand); and §7j.37's "those ids are SFX ids, not text messages" — they are BOTH speech and WARNINGS text; the BOOT_CAMP hint-box system is the separate channel it always was. (4) ONE CONTENT NOTE recorded on §7g.5 (no mechanism change): the posted 0xC..0xE+0xF pair's corpus text is the targeting/bombardment warning in all six languages — the "reinforcement ARRIVAL" reading of the ANNOUNCEMENT is not supported by the WARNINGS corpus; §7g.5's staging facts (delay table, [0x4de658] 0x80 latch, marker scatter) unchanged. (5) ENGINE CONSEQUENCE: NONE today — the queue/ring are SP-UI presentation cells with zero engine reads; each SPOKEN line consumes ONE RandA draw (T3/T4 budget class, joins the existing SFX-pick accounting); no new watch rows; when the E-side ever models radio warnings, the drain order (system channel first, one per channel per frame, oldest-first, text-only 0xF/0x29) and the flush-on-death ids 0x19..0x1B are the parity-relevant semantics. Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run), read-only corpus probes (BEDLAM.EXW DGROUP strings via the 0x457a3c anchor delta; the six LANGUAGE.* [WARNINGS]/[MENU_ITEMS] sections), MANIFEST.sha256 clean before AND after, registry_anchors green (worker d1578d5c claim 2)

Nudge-Worker: d1578d5c-e965-4c97-8d1a-653f95cd0b10

## D126 — 2026-08-23: P4/RE — THE 0x4ea238 AERIAL-BOMBARDMENT SHELL FAMILY + [0x4de658] CENSUS CLOSED (docs-only, §7j.54; the queued "8-jittered-marker scatter" unit + the D125 content-note arbitration). FIVE decisions recorded: (1) THE GRAMMAR: bank 0x4ea238 = 8 shell records × 10 bytes (0x50, MissionShell memset 0x447a51/56): {u16 x@+0, u16 y@+2 (world-PIXEL ground point), u16 fall-z@+4 (writer 0xFF, −0x20/frame, pinned := ground+1 at impact), u16 start-delay@+6 (writer 0x20+2·i, −1/frame, inert while ≠ 0), u16 valid@+8}; §3's passing "10-byte records" note = this bank. Writer = the robots() idle-arm tail 0x40c25e..0x40c351: 8 shells, x = robot.px + RandA&0x7F−0x3F (ONE draw per attempt, pre-gate), y = robot.py − 0x80 + i·0x20 (deterministic fan, NO y jitter), tile-bounds-gated (fail = shell dropped). (2) THE RESOLVER: FUN_00423e1c (MissionShell @0x447ffa, 1/frame) is the shell TICK — head decs [0x4de658]; per record: fall until get_z_pos(x,y,z) ≥ z (−0x20/frame), then SIX kind-6 debris (3 RandA each, jittered ±0x3F/±0x3F/+0x3F) + NINE FUN_004244a1 5000-damage script blasts over the 3×3 tile patch (tx−1..+2 × ty−1..+2, z_tile+1 if <7) + blink-cursor clear + valid clear; its record-0 impact block 0x423e7c..0x423ed5 (SP ∧ record 0 ∧ cursor ≠ selected+1 ∧ cursor-robot is player-type) stages a chase-camera cut. The §7j item-6/§7g "selection chaser / re-points the selection" gloss is RETIRED (it never writes the selection — its only selection act is clearing the cursor). (3) THE SIBLING IDENTITY: FUN_004245c9 = a 5-instruction CHASE-CAMERA OVERRIDE STAGER (0x4245c9..0x4245e5: {x,y,z} → 0x4de648/4c/50, const 0xF → 0x4de654), NOT a "wall-strip redraw" — consumer FUN_00403938 0x4039b0..0x403a42 swaps the camera-point ring slot (0x4c71c4/c8/cc) to the staged triple for 15 frames ([0x4de654]−− per frame; robots() 0x40b885 gates the recenter off; MissionShell clear 0x4478ad); FULL caller census = FOUR sites (door stepper 0x422427, delayed-trigger expiry 0x422e55, artillery spotter reveal 0x41173a, bombardment record-0 impact 0x423ed5) — all "wall-strip redraw"/"wall redraw" glosses (§7j.19, §7j.21, §7j.22 ×2, the door ledger row) corrected in place with history preserved; the door's real wall redraw is the per-tile FUN_004235e4/FUN_004235bf stamping. (4) THE [0x4de658] LEDGER ROW CLOSED: it is the salvo COOLDOWN latch (the dword 0xC below the weapon-table base, NOT part of it): arm write 0x80 @0x40c27f, arm gate read @0x40c18b, read+dec 1/frame @0x423e25..0x423e32, MissionShell clear @0x447877; the 0x442ba7 text match is the D89 SHOP loadout-mirror displacement alias (eax = p·0x62+0xE → ≥ 0x4de672), NOT an access. (5) THE D125 ARBITRATION — CLOSED: OFFENSIVE BOMBARDMENT. The shells are neither targeting reticles nor arrival beacons — they ARE the bombardment (falling sprites, GENERAL.BIN 0x12C via FUN_0040798e, descending 32 px/frame in the iso projection, renderer 0x4066e4..0x4067a6); each impact is a 9×5000-damage kill-anything barrage centered ON THE IDLE ROBOT that tripped +0x70 (SP: only the SELECTED robot accumulates; thresholds {400,300,200,5000} frames; ordering resets it) — the game punishes idling, exactly what the six-language warning pair announces; §7g.5's "reinforcement ready/ARRIVAL" gloss RETIRED (the §7h powerup case-1 drop(+0x80)=1000 family is the REAL reinforcement mechanism and stands). ENGINE CONSEQUENCE: NONE today — no corpus scenario exercises the idle threshold (S0..S7 keep the selected robot ordered/active); if ever modeled, a salvo costs 8 RandA at arm + ≈27–29 per impacting shell (T2/T3-class through FUN_004244a1 + kind-6 disburser), and [0x4ea238]/[0x4de658] are additive watch-row candidates, deliberately NOT in the first golden. NOTE: this unit ADOPTED + fully re-verified interrupted same-item WIP already present in the worktree (its §7j.54 forward-references, ledger rows and gloss corrections were staged but the section itself was missing; its 2-caller FUN_004245c9 census was corrected to 4). Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run, no corpus read), MANIFEST.sha256 clean before AND after, registry_anchors green (worker ed78ecdc claim 2)

Nudge-Worker: ed78ecdc-dfee-4579-bd97-d59f879e8787

## D127 — 2026-08-23: P4/RE — THE HEAT-MACHINE WARNING FAMILY CLOSED (docs-only, §7j.55; the queued unit — §7j.53's twelve un-named producer sites 0x4101d7/0x41025e ch 0/1/2 + the §7j.45 scorch-lane relation). FIVE decisions recorded: (1) THE FAMILY DECODE — FUN_004100b7 (0x4100b7..0x4102b6, sole caller = the robots() phase-1 pass 0x40bc72, amount 0x14 when the robot's tile +0x18 SCORCH byte ≠ 0) is the HEAT-IN machine: the +0x98 DAMPER pool (equipment stat 0x2C ×200, spawn 0x40d013 / MP-respawn 0x40ea59 — the same chassis switch as 0x2A shield-charges/+0x8C and 0x2B battery/+0x94) absorbs first (pool −= amt; >0 return; ≤0 → zero + "UNIT n DAMPER EXHAUSTED" ids 0x2E/0x2F/0x30 ONCE + return — the damper-breaking pass adds NO heat); pool==0 → word@+0x30 += amt (i16 wrap) clamp ≤ 0xBB8(3000), with EDGE-triggered threshold crossings keyed OLD-vs-NEW: 0x753(1875) → "TEMPERATURE CRITICAL" ids 6/7/8 (@0x41025e/0x410280/0x4102ac), 0x9C4(2500) → "HAS OVERHEATED" ids 3/4/5 (@0x4101d7/0x4101f9/0x41021d), old ≥ 0x9C4 → FUN_004102b6 EVERY pass, old ≥ 0x753 → early return; rising heat escalates CRITICAL→OVERHEATED (the strings read as "about to" vs "has"); one huge add crossing both posts BOTH (overheat first); every triple uses the standard idx == [0x46cbd4]+k gated [0x46cbd8]>k squad-slot dispatch, one post per event. FUN_004102b6 (0x4102b6..0x4103ed, sole caller 0x41019a) = the AMMO COOK-OFF: RandA()&0x7F==0 (1/128 per pass) ∧ w=RandA()&7<7 → drain = max(1, ammo@+0x38+8w >> 3), ammo −= drain with floor 1 (empty slot → 1 quirk, unobservable until cooked); player-type victim → [0x46ccec]:=2 (sidebar presentation); +0x32==0 → "UNIT n LOSING AMMO" ids 0x31/0x32/0x33 + word@+0x32 := 100 — one warning per 100 frames. (2) THE TERMINOLOGY ARBITRATION — §7j.45 item 4's "armor/pool/charge ticks" vocabulary (written pre-§7j.53) is SUPERSEDED: +0x30 = the HEAT accumulator, +0x98 = the DAMPER, the sidebar FUN_0040807f "armor bar" = the HEAT gauge (its full scale 2500 IS the overheat threshold), and the "drain-before-charge design intent unclear" tag RETIRED (the damper absorbs heat by design); corrections landed history-preserved in §3 (+0x30/+0x32 rows), §7f.4 item 1, §7j.45 item 4. (3) THE +0x32 CELL CLOSED — §7j.45 Part B's "producer unknown" residue: sole writer = the cook-off tail 0x4103e3, sole reader = its own 0x41036e gate, decay = the robots() pre-walk dec-gated-≠0 trio 0x40bab7..0x40bac6 (alongside +0x34/+0xA4); the §3 "scorched tiles re-burn every ~100 frames" gloss RETIRED — the cell is the LOSING-AMMO warning cooldown, no tile-burn role exists; the +0x34/+0xA4 alarm pair has ZERO traffic in the family (FUN_0040e230's cells; only the shared decay walk ties them). (4) THE CELL CENSUS — word@+0x30: writers = the phase-1 bleed (0x40bc7d/0x40bc98), FUN_004100b7 (0x41016c/0x410187), SP-death reset (0x40eacf), MP-respawn reset (0x40e864; the 0x40e6e2 text match is the seven-order-words walk record+0x38..+0x68, NOT a +0x30 site — displacement-aware filtering required); readers = FUN_004100b7 + the gauge ×3 slots (0x408129/0x408252/0x40837d) + the bleed check 0x40bc85. dword@+0x98: the two stats-copies + FUN_004100b7's drain/zero, reader = FUN_004100b7 alone. (5) CORPUS REACHABILITY — UNREACHABLE BY CONSTRUCTION (the §7j.53 note confirmed with the mechanism): the only pad-armer is the scorch byte (death rings 1/2/4 §7j.9 + platform weaken/build +4 §7j.41, clamp 7, fade −1/frame §7j.10) → one write arms a tile ≤7 frames → ≤ +140 heat per event chain; crossing 0x753 needs ≥94 net armed passes = the byte re-written ≥~14× within ~94 frames under a PARKED robot — the corpus stages no such cascade (S4/S5C destroys are structures/critters, S7 platform events are sparse, a robot's own death resets its +0x30), so ids 3..8 + 0x2E..0x33 never post and FUN_004102b6 never runs in S0..S8; below 0x9C4 the machine is FULLY DETERMINISTIC (both RandA draws live in the cook-off) and mutates only in-span robot-bank bytes E models verbatim — the pinned chains hold. ENGINE CONSEQUENCE: NONE today; E's deliberate omissions (warnings, cook-off, +0x32 decay) are unobservable in corpus; recorded seam — a future sustained-scorch scenario MUST add FUN_004102b6 verbatim (gate draws + ammo drain would diverge the RNG stream AND the weapon banks). The "armor-pad-reads" watch id keeps its legacy name (registry anchor load-bearing; the byte is the scorch byte). Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run, no corpus read — the corpus probes were the already-committed §7j.53/§7j.9/§7j.41/§7j.10 facts), registry_anchors 2/2 green, MANIFEST.sha256 clean before AND after (worker 19d79ca9 claim 2)

Nudge-Worker: 19d79ca9-bf45-4924-b098-29555435ba79

## D128 — 2026-08-23: P4/RE — THE [0x4edbd8] CAMERA-GATE CELL + [0x4ede54] CENSUS CLOSED (docs-only, §7j.56; the queued unit — the §7j.54 precondition cell + the robots() 0x40b8aa recenter speed-factor reader). FOUR decisions recorded: (1) THE [0x4edbd8] VERDICT — it is the "ACTIONPAN" value of the REGISTRY key HKCU\Software\Mirage\Bedlam\1.00 (the whole config family is registry I/O, .idata pinned: FUN_0044ed40 = RegCreateKeyExA(HKCU, "Software\Mirage\Bedlam\1.00", KEY_ALL_ACCESS) → hKey [0x4ef770]; FUN_0044ede4 = the bounded loader — RegQueryValueExA writes the cell DIRECTLY, absent/malformed ⇒ the ecx default @0x44ee23..27, out-of-bounds ⇒ same; FUN_0044ed98 = query-then-RegSetValueExA self-heal writer; FUN_0044eee0 = the REG_SZ create-if-missing used for DEFAULTNAME="Player"): 4-site census = the two §7j.54 readers EXACTLY (0x4039b0 camera-slot swap; 0x40b875 recenter gate w/ the [0x4de654] leg 0x40b885 — address refined; the double-je @0x40b87f is a dead Watcom artifact) + the loader registration @0x42535c (boot, caller 0x41c129) + the saver read @0x42545c (name-entry exit 0x43b03b + 0x41c59b). Bounds [0,1], DEFAULT 1 ⇒ ACTION PANS ARE ON in a default install; the cell is .bss + session-constant, NO game-state/mission-phase/UI writer exists (EXW has no options screen for it). The family pattern cross-checks via INSTALLDRIVE (bounds ['A','Z'] default 'C') and SOUND (bounds [0, current-volume]). (2) THE "CONFIG.BDL" GLOSS RETIRED (TITLEMENU §4 corrected history-preserved + the ledger row): the byte string "CONFIG.BDL" has ZERO occurrences in BEDLAM.EXW (only "CONFIG.SYS file, or" in an error message); the on-disk game-data CONFIG.BDL/OPTIONS.BDL are DOS-build leftovers EXW never opens — SAVED.BDL (savegames) is the only referenced .BDL. (3) THE [0x4ede54] VERDICT — it is the VIEWPORT ZOOM (vertical viewport height in backbuffer rows, clamp [0xF0,0x1E0] = [240,480]), NOT a plain speed constant: 26-site census — writers = the zoom-key handler (the FUN_0042034c tail 0x4204ea..0x420548: ±0x10/frame on scan 0x4E/0x0D in vs 0x4A/0x0C out, keystore 0x4edc92/0x4edc51/0x4edc8e/0x4edc50, clamps 0x420528/0x42053e) + the MissionShell init store 0x447883 (leftover-edx — the 0x1E0 @0x44784a does not provably survive FUN_004034ef (`imul edx,edx,0x26` @0x403570)/FUN_0041d954 (xor tails); benign: ≥480 dispatches 1:1 and the first keypress re-clamps) + the temp save/restore pair in FUN_00401107's [0x4ede34] path (v := 480−min([0x4ede34],479), 0x4012c7/0x4012e5/0x4012f1); readers = FUN_00401107 the zoom blitter (Q16 magnify (v<<16)/480 → cells 0x454060/68/64/5c, source offset (480−v)/2, ≥480 → rep-movs 1:1, [0x4edba0] map-overlay → the map path, called from the two MissionShell render sites 0x447ca0/0x448094) + the recenter speed (cursor−240)·v/480 @0x40b89e/0x40b8c5 + the cursor un-zoom mappers 0x4106a1/0x4106d4/0x419a41. [0x4ede34] census pointer recorded (9 sites; producers 0x40d286/0x40d311/0x40d398 + :=1 @0x40ea8b MP-respawn + the MissionShell frame cluster 0x4480af/0x4480d6/0x448121; identity open — follow-up candidate). (4) ENGINE/DIFFER CONSEQUENCES: ZOOM none — no corpus scenario presses zoom keys, the cell is deterministic per mission, touches zero RNG/robot-bank bytes, presentation-only ⇒ no differ rows (recorded for future E-side render parity). ACTIONPAN one live-channel confund RECORDED: default-1 means the §7j.54 pans are live on a default install; the O1 capture machine's registry could hold a stale 0 which would silently disable pans on the original while E models them — the S0 live-session fingerprint step (queue item 1) should record [0x4edbd8] + the five sibling config cells once; a one-frame additive watch row is the remedy if it ever bites (deliberately NOT in the first golden). Verified: objdump-only from ghidra-project/exw-text-objdump.txt + read-only string/import probes of game-data/cd-root/BEDLAM.EXW (no Ghidra run, no corpus write), MANIFEST.sha256 clean before AND after, registry_anchors green (worker 21e88d3b claim 2)

Nudge-Worker: 21e88d3b-d497-4960-a8e5-817051113b1f

## D129 — 2026-08-23: P4/RE — THE ROBOT +0x9C DEATH-FLAG READER CENSUS CLOSED (docs-only, §7j.57; the queued unit — §7j.45 item 6 open point). FOUR decisions recorded: (1) BOTH PRODUCER VALUES PINNED = 1: the SP/other tail 0x40eac0 `mov [eax*8+0x4c6a80],edx` (edx := 1 @0x40eab4, reached when [0x4edb88]==0 SP ∨ no-extract latch [idx*4+0x46aed4]≠0) and the MP respawn tail 0x40e82a `mov [ebp+0x4c6a80],edi` (edi := 1 @0x40e807, reached when MP ∧ latch==0) — the queue "MP-respawn reset" phrasing is a MISNOMER corrected in place (the respawn re-init does NOT clear +0x9C; the respawned MP slot stays death-flagged, harmless because the sole reader is SP-only). (2) THE SOLE READER = the SP SQUAD-WIPE FAIL DETECTOR FUN_0044764c..0x44770a (decoded whole; sole caller MissionShell 0x44870d gated [0x4dc67c]==0 = extraction NOT complete — a wiped squad after extraction never fails): MP → ret 0; walks the squad records [0x46cbd4]..+[0x46cbd8]−1, FIRST +0x9C==0 → ret 0 (alive); all dead ∧ [0x4ede34]==0x1E0 (the death-wipe cell at its terminal 480 — set :=1 at selected-robot death 0x40ea8b, zeroed per-mission 0x44787d/on click-select 0x40d286) → FUN_0042391d + FUN_00425a03 (+cond. FUN_0042595a) + FUN_00425bf5 + the [0x46cca4]-gated anim string 0x459852 → ret 1 → MissionShell returns 3 (the fail/debrief transition; ret 2 = launch). Semantics: +0x9C = the MISSION-FAIL liveness oracle, DISTINCT from +0x7C alive / +0x78 hp (both re-staged by MP respawn; this never is). (3) LIFECYCLE CLOSED — no literal zero-writer exists; the clear is the mission-staging WHOLE-BANK ZERO-FILL: FUN_0040cca2 @0x40cd29..38 — ecx := 0x7E0; edi := 0x4c69e4; call FUN_0041cd42 (the [0x4eba20] file rewind — edi/ecx callee-saved, NOT its args); call FUN_00402965 (the §7j.21 memset-0 row) zeroes 0x7E0 = 12·0xA8 bytes = the WHOLE 12-SLOT BANK (NEW FACT: the robot bank is 12 slots; [0x46ccbc] counts staged robots); the only immediate-load of 0x4c69e4 in the binary (no bulk-copy/save-load path touches the bank), and the per-record staging walk 0x40ce70..0x40d0a0 never writes +0x9C — every mission entry starts flag-clean, wiped squads cannot leak across missions. (4) THE §7j.55 SIDEBAR CROSS-QUESTION ANSWERED NO + one adjacent census: the heat-family sidebar row pass never reads +0x9C ([0x46ccec] sole reader 0x407205 — it is a FLASH-COUNTDOWN in the [0x46ccf0]/[0x46ccf8] timer family, ≠0 → dec → FUN_00408403; writers death :=3/cook-off :=2/click-select :=2); the queue "likely dead-robot per-frame handling" hypothesis retired — the reader is a mission-level control gate. ENGINE/DIFFER CONSEQUENCE: NONE — E already conforms (death_flag := 1 in the SP death subset; fresh per-mission records ≡ the whole-bank zero-fill) and death_flag is already a +0x9C U16 field leaf of the T1 robot-bank differ row (upper word always 0; dword stores of 1). Deliverables: §7j.57 + the §3 +0x9C row + 2 ledger rows (squad-wipe fail detector, robot-bank zero-fill) + the §7j.45 item-6 closure + this entry. Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run, no corpus read), registry_anchors 2/2 green, MANIFEST.sha256 clean before AND after (worker 18039414 claim 2)

Nudge-Worker: 18039414-f443-4aa2-833a-48c536285664

## D130 — 2026-08-23: P4/RE — THE [0x4ede34] DEATH-WIPE CENSUS CLOSED (docs-only, §7j.58; the queued unit — the §7j.56/B pointer). FIVE decisions recorded: (1) THE VALUE GRAMMAR — [0x4ede34] is the CLOSING-IRIS death-wipe progress cell: 0 inactive; `:=1` ARM at selected-robot SP death (sole site 0x40ea8b in FUN_0040e230's SP/other tail, dying == [0x46cbd4]+[0x46cbdc]; MP NEVER arms — the MP branch posts the sibling marker latch instead and respawns); `+=0x28` (+40/frame) by the sole MissionShell frame-cluster writer 0x4480af (immediately after the present call 0x448099); terminal `:=0x1E0` @0x4480d6 when cell+40 ≥ 480; `:=0` cancels = the three squad-slot click-select strips 0x40d286/0x40d311/0x40d398 (selecting an ALIVE squadmate aborts the iris — the §6c.2 strips are cancels) + the auto-reselect cancel 0x448121 (a Watcom xor-of-equals zero) + the per-mission reset 0x44787d (ecx provably 0 through the reset block). (2) WHO/WHAT INCREMENTS (the queue ask, prime candidate confirmed): the MissionShell frame cluster IS the incrementer — and at terminal it runs the AUTO-RESELECT PASS: walk squad slots 0..[0x46cbd8), gate ALIVE(+0x7C) ∧ TYPE(+0x2A = dword@+0x28>>16) == [0x4edb90] (the global player-type word) ∧ slot ≠ [0x46cbdc] → [0x46cbdc] := slot (NO break — LAST eligible slot wins), flash [0x46ccec] := 3, cell := 0, [0x4ea8f8] := 0. No eligible mate → cell PARKS at 480 — exactly the D129 fail-detector conjunct (0x4476a2): in SP "no cancel" ⟺ squad wiped; the two fail conjuncts are one event observed twice; a MP wipe is impossible (SP-only machine, consistent with the SP-only detector). (3) WHAT THE TEMP RENDER SHOWS: FUN_00401107's temp path = push [0x4ede54]; v := 480−min(cell,479); call 0x4012f7 = FULL-SCREEN fill-0 (0x40129e, 480×0x78 dwords to the visible page) THEN the centered v×v SHRINK of the 480×480 backbuffer window (source = the normal path's base+fine-cam offset math, no source-centering; scales 0x454060/68 := (0x1E0<<16)/v — the INVERSE of the normal zoom; dest centered at (240,240) via (480−v)/2·(pitch+1); row routine 0x401430 = the horizontal SHRINK twin of the normal path's 0x4013e8 STRETCH, with a second vertical sub-pixel accumulator between rows); restore. Meanwhile FUN_00403938's head gate 0x403952 (address-attribution corrected from §7j.56/B's "FUN_00401107 gate") SKIPS the whole render body → the backbuffer holds the last pre-death world frame: a 13-frame CLOSING IRIS on a FROZEN snapshot (479×479 → 1×1 dot, ~40 px/frame per side), user zoom untouched. The queue's "cinema/wipe effect" hypothesis confirmed with the iris geometry pinned. (4) THE [0x4ea8f8] SIBLING DECODED = the MP death-position marker countdown: sole ≠0 producer 0x40e7ef := 0x20 (FUN_0040e230 MP branch, dying == selected, posting (rec+0)>>8/(rec+4)>>8/(rec+8) into [0x4ea8ec/f0/f4]); consumer = the FUN_00403938 head 0x403974..0x4039a5 (while ≠0: copy the trio into [0x46ccdc]·12+0x4c71cc/c4/c8 — the §7j.20 selected-anchor ring, consumer = the §7j.54 chase camera — and dec); zeroed in tandem with the iris cell at every cancel + per-mission 0x4478f1; SP never sets it. [Cross-ref: the destination bank is already census-closed — 0x4c71c4 = the §7j.20 "per-player selected anchor" bank (4×0xC {x>>8, y>>8, z}), consumer = the §7j.54 chase-camera ring reader — the death post makes the camera HOLD the dead robot's position; no follow-up needed.] (5) ONE VALUE CORRECTION: §6c.6e's auto-reselect flash "ebx(2)" is wrong — ebx := 3 @0x4480de (the death-flash duration class); corrected in place history-preserved. ENGINE/DIFFER CONSEQUENCE: NONE — presentation-only (render path + screen-transition control flow; zero RNG, zero robot-bank bytes, no dump-surface cell; the fail-waits-for-wipe timing is the same D129 out-of-surface class); recorded for future E-side render parity (fill-0, centered v×v, (480<<16)/v Q16 shrink both axes, +40/frame, frozen source). Deliverables: §7j.58 (A–F) + 2 ledger rows (death-wipe iris cell, MP death-position marker countdown) + the §7j.56/B pointer closure + §6c.2/§6c.6e corrections + the MISSIONVIEW zoom-path precision note + this entry. Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run, no corpus read), registry_anchors green, MANIFEST.sha256 clean before AND after (worker 27b33f6c claim 2)

Nudge-Worker: 27b33f6c-28cd-4654-920c-f6fc615de44c

## D131 — 2026-08-23: P4/RE — THE [0x4dc5d0] BLINK-CURSOR PRODUCER CENSUS CLOSED (docs-only, §7j.59; the queued unit — the §6c.6d "producer open" residue). FIVE decisions recorded: (1) THE MECHANICAL CENSUS — exactly SEVEN .text references to 0x4dc5d0 in the whole objdump (0x401000..0x460000), no other addressing form: 5 writers (0x40c1d7 :=ebx=1, 0x40c217 :=edi=2, 0x40c254 :=3 imm, 0x423fef :=ecx=0, 0x447871 :=ecx=0) + 2 readers (0x407428 the §6c.6d portrait gate, 0x423e91 the §7j.54 chase-camera impact gate). (2) THE VALUE GRAMMAR — {0,1,2,3} only; the three :=k+1 writes are the UNROLLED per-slot strips of the robots() idle-arm tail [0x40c1ae..0x40c25e]: k=0 `idx==[0x46cbd4]` NO size gate → warnings (0xC,0)+(0xF,0) → :=1; k=1 `idx==base+1 ∧ [0x46cbd8]>1` → (0xD,1)+(0xF,1) → :=2; k=2 `idx==base+2 ∧ [0x46cbd8]>2` → (0xE,2)+(0xF,2) → :=3; all three share the salvo tail (+0x70 idle :=0, [0x4de658]:=0x80, the 8-shell scatter). CORRECTION of the 2026-08-21 amendment item 6 (history preserved): its "value = the SELECTED robot's SLOT + 1" gloss is an SP COINCIDENCE — the write names the ENDANGERED robot's own squad slot (in MP every idle robot arms; the arm gate idx==[0x46cbd4]+[0x46cbdc] and the write gate idx==[0x46cbd4]+k are different comparisons). (3) THE {1,2,3} GATE SEMANTICS pinned against the effect-row family — the consumer 0x407420..0x407449 is a LITERAL 1/2/3 x-dispatch (1→0x1F0, 2→0x222, 3→0x254, sprite (frame&3)+0x51 GENERAL.BIN via FUN_00401ca2 at y=0xD); **0 AND >3 both draw NOTHING** (the >3 branch is dead-defensive — no writer ever stores >3). 1/2/3 are NOT blink classes and NOT FLAGS.BIN ids: the 10×16-B effect-row array at 0x4dc5d4..0x4dc67c (ids 1..0xE at 0x4dc5e0+r·0x10) is a DISJOINT array 4 B above the scalar — no site crosses the boundary (the FUN_00422038 allocator scans 0x4dc5e0+k·0x10 only); the §6c.6d "sprite-list field" naming corrected to "warning field". The second reader PROVES the index semantics arithmetically: the impact block computes the bank index `[0x46cbd4]+([0x4dc5d0]−1)` ×0xA8 (endangered ≠ selected ∧ player-type → FUN_004245c9 chase-camera cut). (4) LIFECYCLE — 0 at mission entry (0x447871, the §7j.58 reset cascade) → :=endangered-slot+1 at the idle-threshold arm → 0 at the FIRST shell impact (0x423fef, the per-record completion tail also freeing the record; ≈ arm+32..46 delay + ~8 fall frames) → re-armable only after the 0x80 cooldown AND a fresh idle threshold; ordering resets the idle counter so an actively-used squad never blinks. (5) ENGINE/DIFFER CONSEQUENCE: NONE — both readers are SP-UI presentation, zero sim reads, zero RNG; the DESIGN §5 S1 "blink-cursor-from-spawn" hypothesis is now STATICALLY DECIDABLE (constant 0 on every corpus scenario — no scripted horizon reaches the {400,300,200,5000}-frame idle table); the watches.toml layout "u32 (0 or slot+1)" stays accurate; the EXD twin remains the recorded W1 gap with the 7-site census as its anchor template. Deliverables: §7j.59 (A–E) + the amendment-item-6 correction/supersession note + the §6c.6d gate + engine-seam text fixes + the DESIGN watch/hypothesis row notes + this entry. Verified: objdump-only from ghidra-project/exw-text-objdump.txt (no Ghidra run, no corpus read), registry_anchors green, MANIFEST.sha256 clean before AND after (worker 0329338f claim 2)

Nudge-Worker: 0329338f-bbae-48f6-91c4-67b88ae13c44
## D132 — 2026-08-23: P4/RE — THE EXD BLINK-CURSOR TWIN CENSUS CLOSED (the queued unit; RE-EXD-MAP §5/§5e + watches.toml fills + §7j.59.E addendum; the last sidebar-family W1 gap). FOUR decisions recorded: (1) THE TWIN = [0x0010e108], EXACTLY 7 .text sites mirroring the §7j.59/D131 EXW census ONE-FOR-ONE: writers = the three idle-arm strips (0x1cef1 :=1 via ecx, 0x1cf2c :=2 imm, 0x1cf72 :=3 via ecx ⟷ 0x40c1d7/0x40c217/0x40c254 — register-vs-immediate k=1/k=2 choice is a Watcom codegen swap, semantics identical: k=0 posts (0xC,0)+(0xF,·,1), k=1 (0xD,1)+(0xF,1) behind [0x11958c]>1, k=2 (0xE,2)+(0xF,2) behind >2, all via the warning-post twin FUN_00034972 ≡ FUN_004239ef) + the impact-completion tail 0x34f7f :=0 (⟷ 0x423fef, inside the shell-resolver FUN_00034d89 ≡ FUN_00423e1c, after the 3×3 nine-blast patch FUN_00035406, with the record-valid word clear) + the MissionShell reset 0x59842 :=0 (⟷ 0x447871, in the zero-cascade); readers = the portrait-pass blink gate 0x186dc (⟷ 0x407428, inside FUN_000180a1 ≡ FUN_004072bf; the IDENTICAL `(frame&3)+0x51` sprite, literal 1/2/3 x-dispatch 0x1F0/0x222/0x254, y=0xD, bank [0x1074fc] ≡ [0x4edd7c], draw FUN_000111fa ≡ FUN_00401ca2, 0 AND >3 draw nothing) + the chase-camera record-0 impact gate 0x34e25 (⟷ 0x423e91; the IDENTICAL ([base]+[cursor]−1)·0xA8 → kind@+0x2A == player-type [0x1075c0] ∧ endangered≠selected → cut FUN_0003552e ≡ FUN_004245c9). VALUE GRAMMAR, gate set (SP idx==[slot]+[base], MP every-idle, state==0, threshold [0x8105c][difficulty] = {400,300,200,5000} BYTE-IDENTICAL to EXW 0x454ee8, zone∉{1,7}, latch==0, mode≠2), and the shared arm tail (idle +0x70:=0, salvo latch :=0x80, 8-shell scatter bank 0x8f0b2 ≡ 0x4ea238) all EXACT. (2) THE SELECTION-TRIPLE LABEL-SWAP CORRECTION (history preserved in RE-EXD-MAP §5): the EXW twin of EXD 0x11954c is 0x46cbdc (the SELECTED-SLOT cell — the auto-switch/cmd-builder/key-handler cell; EXW-side writes at 0x448109/0x448111), NOT 0x46cbd4 as the original row claimed; the former "cursor gap" is now pinned as EXD 0x11955c ≡ 0x46cbd4 (the SQUAD-BASE cell) — quad-anchored by the arm-strip compare swap (0x40c1b2 ↔ 0x1cecc), the chase-gate read order (0x423e8c/0x423e9c ↔ 0x34e20/0x34e30), the global-index computation (0x4480c1 ↔ 0x5a871 `[0x11955c]+[0x11954c]` ×0xA8), and the ref-count shapes (~100-site heavy UI cell ↔ ~40-site sparse cell); squad size 0x11958c ≡ 0x46cbd8 per the W8-prep correction. ALL THREE selection cells now mapped — the selection-triple W1 gap CLOSED. (3) TEN §5e CASCADE/ASSET ALIASES pinned as by-products (all [verified]): salvo-cooldown latch 0x4de658→0x1081fc; 8-shell bank 0x4ea238→0x8f0b4 (8×0xA records, grammar identical both sides {x w@+0, y w@+2, fall w@+4 seed 0xFF, start-delay w@+6, valid w@+8}; the 0x4ea236/0x8f0b2 sites = the dword@base−2≫16 x-read idiom — landing-run correction of the first draft's 0x8f0b2/+2/+4 phrasing, re-verified instruction-exact vs EXW 0x40c323..0x40c348/0x423e46..0x424040/0x4066f4); map-overlay flag 0x4edba0→0x1075bc; viewport zoom 0x4ede54→0x107448 (the D128 cell, :=0x1e0 at the cascade head); idle table 0x454ee8→0x8105c; GENERAL.BIN ptr 0x4edd7c→0x1074fc; + the function twins FUN_004239ef→FUN_00034972, FUN_00423e1c→FUN_00034d89, FUN_004245c9→FUN_0003552e, FUN_004072bf→FUN_000180a1. (4) METHOD + CONSEQUENCE: the census substrate is a NEW TOOL + artifact — tools/exd-relod.py (committed) parses the LE header/page-map/fixup tables mirroring the yetmorecode LeLoader and applies the 23,338 off32/sel16 fixups at the loader base policy, emitting a relocated linear image + intel objdump of obj1 0x10000..0x72800; VERIFIED BYTE-EXACT against the Ghidra import (all 9 W1 anchor instructions reproduce: 0x5a6eb call 0x10670, 0x596f9 RNG plant, 0x2eb91 0x2e9b, 0x2b1a1/a9/b1 + 0x1ca6b resolvers, 0x3572c 0x197, 0x33985 0xfe37c scaled store, 0x1feca 0x1388); the generated listing ghidra-project/exd-text-objdump.txt stays LOCAL per the /ghidra-project/ gitignore convention (same as exw-text-objdump.txt — regenerate on demand with the committed tool). watches.toml: blink-cursor exd_addr filled + gap closed, selection-triple re-aliased + layout renamed; registry_anchors updated (blink-cursor leaves the gap set; the selection-triple check now requires the D132 citation). Engine consequence NONE (docs+registry+tool only; both blink readers remain SP-UI presentation; the S1 hypothesis row reads constant 0 on BOTH channels by identical construction). Verified: MANIFEST.sha256 clean before AND after the read-only corpus probes (objdump byte reads of BEDLAM.EXW DGROUP + the EXD parse), no Ghidra run (BEDLAM.EXD stays single-import), registry_anchors 2/2 green after the fills (worker 4fe7f1e9 claim 2 — interrupted before landing; LANDED + RE-VALIDATED at HEAD by worker c653b51a claim 2: every census claim re-verified instruction-exact against the committed-tool objdump + the EXW twin windows, incl. the 7-site grep, the arm-strip/portrait/chase/reset windows, the [0x8105c] table bytes from the relocated image, and the 0x423e8c/0x423e9c read order; the §5e record-base correction above is this landing pass's)

## D133 — 2026-08-23: P4/RE — THE EXD NO-EXTRACT-LATCH TWIN CENSUS CLOSED (the queued unit; W1 schema gap #2; RE-EXD-MAP §5/§5f + watches.toml + dbx-plan emission + RE-EXW-SIM writer-claim corrections). FOUR decisions recorded: (1) THE TWIN = [0x000f929c], 12 .text sites with 8 readers ONE-FOR-ONE against the EXW 8-reader census + the boot-clear pair: 0x19c71 ⟷ 0x408ef7 (FUN_00408e99 death-anim walk; ≠0 → image 0x65, else the [idx·0xC+0x8b618]-selected table pair 0x82e5a/0x82e8a ⟷ 0x456ce8/0x456d18); 0x1f4cf ⟷ 0x40e7a1 (death core: MP cell [0x1075d8] ⟷ [0x4edb88] ∧ latch==0 → the respawn re-init with staging quad 0x107768/0x107764/0x10776c/0x107770 ⟷ 0x4ea8ec/f0/f4/f8 and the −1 move-target pair [idx·4+0xf75ec]/[0xf761c] ⟷ 0x46cc30/0x46cc60 — which TRIPLE-CONFIRMS the §5 robot-bank base 0xf6d34 ≡ 0x4c69e4 via the [ebp+0xf6d34]/+8/+0xC reads and the [idx·0xA8+0xf6dd0]:=1 death-flag store ⟷ [i·0xA8+0x4c6a80]); 0x30c87 ⟷ 0x4200db (escape-pod animator gate, pod bank 0x8d314 ⟷ 0x4e64c0 stride 0x1C); 0x5b1cc/0x5b34a/0x5b51c ⟷ 0x449dc8/0x449ee8/0x44a08c (MP cycler trio: cursor [0x107688] vs current [0x1075c0], switch FUN_0006209c ⟷ FUN_00449b60 over records 0x9255c ⟷ 0x4dd4a0); 0x5b7ea (`cmp edi(0),[latch]` — codegen swap) ⟷ 0x44a322; 0x5b89c ⟷ 0x44a3d2 (endgame census: marks 0x8b744+0x30 ⟷ 0x4eba30, count 0x10760c ⟷ 0x4edb8c); boot memset(0xf929c, 0x30 = 12 dwords) @0x2cd41 ⟷ (0x46aed4, 0x30) @0x41c412 — the 12-slot extent EXACT both sides, NOT per-mission. (2) THE WRITER ASYMMETRY (headline): EXD has exactly ONE setter — FUN_0005bb71 @0x5bba0 `mov [edx·4+0xf929c],esi(1)` (callers 0x5ba27 the MP lobby tally + 0x1b2bd), the DOS MP LOBBY ROBOT-PICK: [0x1195dc]:=idx, [0x1195bc]:=0x32, call FUN_000347a3(idx), alive@+0x7C := 0, memset(0x9255c+idx·0x80, 0x80), then a latch==0 census cmp 2 + message 0x8720b — plus the EXD-only lobby type-tally walk 0x5ba83 (16-dword bank 0x8b5d4, staging-rec type byte @+6, <0x10 gate). EXW has NO setter — census-complete: the full literal-site sweep over exw-text-objdump.txt finds exactly 9 sites (8 cmp + the memset pair), NO other constant-bearing instruction can address the array, and no memset/rep-movs span overlaps it (the sibling clear 0x447aa6 covers 0x46ae94+0x30, 0x10 short). THE COMMITTED §7j.19/§7j.27 "writers FUN_0040e230/FUN_00449c94/FUN_0044a38a/FUN_00408e99" CLAIM IS CORRECTED — those four functions are READERS (all four SIM spots amended in place, history noted). SEMANTICS CORRECTED: the latch is the per-robot CLAIMED/CONSUMED flag (a lobby pick claims; claimed robots get no pods, no MP re-drop, no cycler switch, and the death core takes the SP tail) — on EXW every gate takes the ==0 path at runtime (pods extract everyone, MP respawn unrestricted, all robots cycler-available). ENGINE consequence: NONE for SP corpus scenarios (the latch is 0 on both channels by identical construction — the claim path is MP-lobby-only and EXW cannot even take it); the differ's future MP scenarios must model the EXD-only claim semantics if a DOS-MP oracle row is ever captured. (3) FOURTEEN §5f cascade aliases pinned as by-products (all [verified]): MP-mode 0x4edb88→0x1075d8; current-robot 0x4edb90→0x1075c0 (refines D132's "player-type" gloss); MP staging records 0x4dd4a0→0x9255c; marks 0x4eba30→0x8b744; endgame counts 0x4edb8c/0x4eba28→0x10760c/0x107660; cycler cursor 0x4eba00→0x107688; cycler word/result 0x4eba08/0x4dc6e0→0x11a9a6/0x10e0c0; msg gate 0x4edc45→0x894d5; switch fn FUN_00449b60→FUN_0006209c; msg-post pair 0x44d2ac/0x44d2da→0x5ef05/0x5ef33; death-anim selector family 0x4ebaa0→0x8b60c (+tables 0x456ce8/0x456d18→0x82e5a/0x82e8a); pod bank 0x4e64c0→0x8d314; respawn staging quad 0x4ea8ec/f0/f4/f8→0x107768/0x107764/0x10776c/0x107770; memset fn FUN_00402965→0x12206. PLUS the EXW per-TYPE sibling census 0x46ae94+type·4 (writers 0x40d01b/0x40d028/0x40ea61/0x40ea6a, readers ==1/==2, clear 0x447aa6) recorded so it is never conflated with the latch. (4) REGISTRY/PLAN: watches.toml no-extract-latch row filled (exd_addr 0xf929c+i·4 with count cell 0x11958c, extent count*4, verified, D133 note); dbx-plan emits the row as a count-driven bare span (robot_count symbol); registry_anchors gap set reduced to {sfx-master-gate} + a D133 citation check; capture-plans S1..S8 regenerated (S0/S0W untouched — no T1 rows). Verified: MANIFEST.sha256 clean before AND after the read-only corpus probes (objdump greps only, no Ghidra run), registry_anchors 2/2 green, diffharness test suite green (worker 36c6f950 claim 2)

## D134 — 2026-08-23: P4/RE — THE EXD SFX-MASTER-GATE TWIN CENSUS CLOSED (the queued unit; the LAST W1 schema gap; RE-EXD-MAP §4/§5g + watches.toml + dbx-plan emission). FOUR decisions recorded: (1) THE TWIN = [0x10743c], pinned by the queue's own anchor: the EXD BOOM-trio twin FUN_00032de9 (gate `cmp [0x10743c],0` @0x32df1) is shape-identical to EXW FUN_00421e60 (gate @0x421e68) — same push-order prologue (ebx/ecx/esi/edi/ebp/eax), same `mov ebp,edx`, same RandB (EXD 0x12257 ⟷ EXW 0x4029b6) signed-idiv-3 dispatch over the BOOM cell trio, same shared play tail: EXD `call 0x4c584` @0x32f95 = THE PLAY TWIN FUN_0004c584 ⟷ FUN_0043a48e (its own master gates at 0x4c593 entry + 0x4c9a9 mid-body, the fail leg setting the drop-flag [0x1195f4]:=1 ⟷ [0x46ae78]:=1). Whole-objdump censuses: EXW 19 literal sites, EXD 18, no displacement/address-load strays. READER families map ONE-FOR-ONE: the arrival/impact FIVE (EXD 0x32d88 RICOCHT quad cells 0x11a918/10/14/20 ⟷ 0x421df9; 0x32df1 BOOM ⟷ 0x421e68; 0x32e68 GRUNT trio cells 0x11a8b8/b4/b0 — order REVERSED vs EXW 0x4ee000/04/08 ⟷ 0x421ede; 0x32eda DEATH 0x11a948/0x11a8d8/0x11a8dc ⟷ 0x421f54; 0x32f49 HURT 0x11a938/30/34 ⟷ 0x421fca), the music-sequencer TRIO (0x13e0f/0x13f26/0x1406a ⟷ 0x4033df/0x4034fa/0x40364d, second gate [0x107578] ⟷ [0x4edbe0]), the radio-warning queue consumer (0x34a8e ⟷ 0x423af7, gates SPEECH [0x10766c]≠0 ∧ [0x107444] ∧ [0x10743c] ∧ id∉{0xF,…} — the EXW twin's first gate reads its edi ARG, independently confirming [0x10766c]≡[0x4eb93c] SPEECH + [0x107444]≡[0x4ede5c]), the driver-sync wait (0x3696f in FUN_00036966 ⟷ 0x425bfe in FUN_00425bf5, spin call 0x60472([0x11a898]) ⟷ 0x44c600), and the MissionShell volume-key pair (0x59eae/0x59f29 ⟷ 0x447e72/0x447efd, with the EXACT `imul [vol],0x147; sar 7` scale — pinning [0x1081f0]≡[0x4ddb2c] — and the [0x107570] OR-leg ⟷ [0x4edbe8]). EXD-only: the frame-tick music hook 0x12767 ([gate]∧[sister]∧[music] → call 0x135ef, `inc [0x801a0]`). (2) THE WRITERS + THE CONFIG DIVERGENCE (the "who sets it" answer): EXW = the sound init FUN_0043a144 (0x43a198 :=1 / 0x43a1b1 :=0, sole caller GameMain 0x41c33f, raw-dword scan zero), with the VALUE sourced from the Win32 REGISTRY (boot loader FUN_004252c0 @0x42530a loads HKCU "SOUND" via FUN_0044ede4; the saver FUN_0042540c @0x4253f3 reads it back into the RegSetValueExA writer at the name-entry exit; init forces CLEAR through [0x4ee9b0]:=-1 when the gate is 0); EXD = the sound init FUN_0004be7d (0x4c0c8 :=1 / 0x4bf85 :=0; callers 0x2cc70 boot AND 0x5b03f title — the already-on guard `cmp [0x107444],0` @0x4bed3 makes re-calls idempotent), which parses the FILE **CONFIG.BDL** (the runtime install-dir buffer [0x9237c] + "CONFIG.BDL" 0x867ea; companions "r+b"/"NO SOUND FX"/"Sound initialisation failed"; probes FUN_0005f23f→[0x1076ac] + FUN_0005f471; parse failure → CLEAR) — the DOS file-config vs Win32 registry port seam, the EXD side of the D128 "CONFIG.BDL retired on EXW" story. BOTH init branch pairs write the SAME tandem cells: sister gate ([0x4ede5c]/[0x107444]), SPEECH clear ([0x4eb93c]/[0x10766c]), the 0xfe000 mixer-arena cell ([0x46ae84]/[0x119620]), and the 16-entry voice-table fill loop ([0x4eada8]/[0x8b938], eax=0x10..0xa0 step 0x10, value 0x3e8) — INSTRUCTION-EXACT. (3) THE BANK-NAME WALK + 19 §5g CASCADE ALIASES: FUN_0004c121 (the §4 lead, called @0x5982a) loads via FUN_0004c3dd with names past the shared "SOUND\SFX\" prefix — BOOM1/2/3→0x11a944/40/3c, SQUISH2/3→0x11a950/0x11a94c, HURT1/2/3→0x11a938/30/34, DEATH1/2/3→0x11a948/0x11a8d8/0x11a8dc, PLASMA→0x11a8e4, RICOCHT1..4→0x11a918/10/14/20, MISSILE1→0x11a8e0, POWERUP→0x11a924, ELEV1/2→0x11a8e8/0x11a91c, DEADMAN1/2→0x11a8d4/0x11a8d0, BEEP5/TEXTBOX1/MIDIGUN→0x11a92c/0x11a8f8/0x11a954(+dups) — while the GRUNT trio rides the MissionShell-head walk @0x59b79..0x59c09 (FUN_0004c384 flavor, BEAMIN/THROW/PEXPLODE/BIOFIRE/CACODETH/SQUAWK companions — the §7j.30/D120 mission-bank family). Cascade alias table (all [verified], §5g): 0x4ede5c→0x107444; 0x4eb93c→0x10766c; 0x46ae84→0x119620; 0x4eada8→0x8b938; 0x4edbe0→0x107578; 0x4edbe8→0x107570; 0x4ddb2c→0x1081f0; 0x46ae78→0x1195f4; 0x4edec5→byte 0x80333; the six bank-cell trios/quads one-for-one (GRUNT order reversed); + the function twins FUN_0043a48e→FUN_0004c584, FUN_00425bf5→FUN_00036966, FUN_0043a144→FUN_0004be7d. (4) REGISTRY/PLAN/CONSEQUENCE: watches.toml sfx-master-gate row filled (0x10743c, verified, D134 note) — THE W1 REGISTRY GAP SET IS NOW EMPTY (registry_anchors gap list emptied + a new hard check: NO row may carry exd_status=gap; sfx-master-gate requires the twin + the D134 citation); dbx-plan now emits the row on every T0 scenario (the runner's NoExdAddress fixture fabricates a synthetic gap since none remain); ALL 12 capture plans regenerated (S0/S0W INCLUDED this time — the row is T0: 20 anchor/11 per-frame on the S0 shape, deferred 7→6; S3 10→9; S4/S6/S7 21→20; S8 24→23); 93 diffharness tests + 13 canonical_dump_gate green (E's W6 row list is untouched — sfx-master-gate stays a documented E-gap exactly like no-extract-latch after D133; a future E config model can emit constant 1), fmt+clippy clean. ENGINE consequence NONE (session-constant config scalar); one S0 fingerprint-step companion recorded: a capture machine with sound DISABLED dumps 0 where E assumes the row — one dbgprobe read of [0x10743c] at the anchor stop settles it (the D128 ACTIONPAN pattern). Verified: objdump-only from the committed exw/exd listings + read-only string probes of game-data (vma↔fileoff via the DEADMAN1 anchor); MANIFEST.sha256 clean before AND after; no Ghidra run (worker 2a9f1b9f claim 2). LANDED + RE-VALIDATED at HEAD by worker e104cbd0 claim 2 (the wrapper's respawn after the notes author's transport death): the full impl leg re-verified INDEPENDENTLY — every census family re-derived from scratch against the committed objdumps before adoption (the BOOM-trio anchor FUN_00032de9, the play twin FUN_0004c584 head/body, the init branch pair, the trio/arrival/radio/vol-key readers, the FUN_0004c121 name walk incl. the DEADMAN1 string anchor, the OPTIONS.BDL divergent writer), 93 diffharness + 13 canonical_dump_gate green, fmt+clippy clean, MANIFEST clean pre+post — and ONE prior-text defect CORRECTED with history preserved: the headline census counts are EXW 18 literal sites ('4ede58') / EXD 17 ('10743c'), NOT 19/18 (the first draft counted the EXD init's second CALLER 0x5b03f as a census site and mis-summed the EXW side against its own 14-cmp enumeration); the true one-for-one is 13 reader sites with EXW-only {0x43a16c init pre-check, 0x42530a loader address-take, 0x4253f3 saver read} and EXD-only {0x4c593 play-twin entry gate, 0x12767 frame-tick hook}. TWO GLOSS CORRECTIONS ride along: (a) 0x43a79e is NOT "inside FUN_0043a48e" — it is the master half of the options-handler drop-flag pair 0x43a795/0x43a79e (fail → [0x46ae78]:=1); the EXW play twin itself is ungated, its callers gate; (b) the EXD twin of that pair is 0x4c9a0/0x4c9a9 in the SAME sister-then-master order — the first draft's "arg order swapped vs EXW 0x43a79e" was an artifact of mispairing 0x4c593 (the EXD-only play-twin ENTRY gate, redundant with the per-family caller gates) with the drop-flag gate. RE-EXD-MAP §4/§5g + the watches.toml note carry the corrected counts; this decision entry's own "19/18" figures above stand as recorded history.

## D135 — 2026-08-23: P4/RE — THE EXW BANK-CELL TWIN CROSS-CHECK CLOSED (the queued unit; the D134 §5g leftovers; docs-only; RE-EXD-MAP §5g-bis). THREE decisions recorded: (1) THE TWO MISSION WALKS ARE STORE-FOR-STORE ORDINAL-IDENTICAL — EXW FUN_0043a1d3 (stores 0x43a1d8..0x43a368) and EXD FUN_0004c121 (stores 0x4c130..0x4c2b6) write the same 27 registers in the same order (MIDIGUN, BOOM×3, MIDIGUN-dup, SQUISH2/3, HURT×3, DEATH×3, PLASMA, RICOCHT×4, MISSILE1, POWERUP, ELEV1/2, DEADMAN×2, BEEP5, TEXTBOX1, BEEP5#2), and the MissionShell-head walks are ordinal-identical too (EXW 0x447bb7..0x447c3b ⟷ EXD 0x59b83..0x59c09: BEAMIN, THROW, PEXPLODE, BIOFIRE, CACODETH, SQUAWK, GRUNT1/2/3, no re-ordering — the EXW twin of the §5g FUN_0004c384-flavor head walk is the plain MissionShell-cascade block, no separate flavor on EXW). (2) THE 17 LEFTOVER ALIASES PINNED with 1:1 READER-COUNT PARITY on every cell (whole-objdump reader censuses both sides, the `mov ds:CELL,eax` store form excluded; all readers feed the play twins FUN_0043a48e/FUN_0004c584): MIDIGUN 0x4edf60→0x11a954 (2⟷2, robot weapon fire — robot-stride index + robot-array coords 0x4c69e8, §7j.17); MIDIGUN-dup 0x4edf70→0x11a958 (0⟷0 — consumer-less on BOTH sides, the D94 quirk is a twin quirk); SQUISH2 0x4edf74→0x11a950 + SQUISH3 0x4edf78→0x11a94c (2⟷2 each, critter contact/melee via critter-bank coords 0x4cec3e/a); POWERUP 0x4edfa8→0x11a924 (9⟷9, §7h.2/§7j.30 pickup family); MISSILE1 0x4edfac→0x11a8e0 (3⟷3, §7j.17); ELEV1 0x4edfb0→0x11a8e8 (3⟷3) + ELEV2 0x4edfb4→0x11a91c (2⟷2) — TRT structure/elevator move (after the 0x4239ef(eax=0x23) structure call, structure coords ≫16≪5, §7j.41 family); BEEP5 #1 0x4edfdc→0x11a92c (6⟷6) + #2 0x4edfd8→0x11a8ec (6⟷6) — paired BY ORDINAL (walk position), independently confirmed by the briefing re-registration twins (EXW 0x43d17d/0x43d18c/0x43d19b → 0x4edfdc/0x4edfd0/0x4edfd8 ⟷ EXD 0x4f343/0x4f352/0x4f361 → 0x11a92c/0x11a8f8/0x11a8ec, same order); TEXTBOX1 0x4edfd0→0x11a8f8 (2⟷2, text-box print, −1,−1 idiom); BEAMIN 0x4edfe0→0x11a900 (8⟷8, §7j.27+critter wake); THROW 0x4edfe4→0x11a90c (5⟷5, robot fire w6/7/8); BIOFIRE 0x4edff0→0x11a8c0 (1⟷1); PEXPLODE 0x4edff4→0x11a8bc (1⟷1, arrival/impact family head region); CACODETH 0x4edff8→0x11a8c4 (1⟷1, k7 death); SQUAWK 0x4edffc→0x11a774 (1⟷1). METHOD: D94's EXW walk re-verified independently this run (register idioms grepped in exw-text-objdump.txt, all 20 name strings re-read from BEDLAM.EXW DGROUP at PE VA 0x454000=file 0x52600, EXD stores re-confirmed in the committed exd-text-objdump.txt) — count corrections vs §7j.30 phrasing: NONE (its "10 refs" for POWERUP = 9 readers + 1 write; the dump's ref counts include register stores). (3) CONSEQUENCE: NONE — docs-only; the SFX cells are presentation-tier (out of the hashed core per §7j.30/sec-9), zero watch-row/E-side changes; the §5g alias ledger is now complete for every cell named by the two bank walks (27 mission + 9 head-walk registers minus the §5g-already-pinned 13). Verified: objdump-only (no Ghidra run), MANIFEST.sha256 clean before AND after the read-only BEDLAM.EXW string probes (worker 9a48b338 claim 2)

Nudge-Worker: 9a48b338-f1bd-4bce-9ab8-78d8e1ef5009

## D136 — 2026-08-23: P4/E-W6 — THE SFX-MASTER-GATE + NO-EXTRACT-LATCH E-GAP EMISSION DECIDED: **EMIT NOW** (the queued decision unit; the W6-followup to D133/D134; canonical.rs emit_frame + the differ normalizers + the DESIGN §6a amendment + the deliberate chain re-baseline). THE DECISION: E emits BOTH former E-gap rows on every qualifying scenario — **sfx-master-gate := constant 1** (T0, rides every frame incl. the anchor; the E engine's sound-on construction assumption — E has no audio config model under §0's state-only scope and every dispatch the gate guards is presentation-tier) and **no-extract-latch := u32 count + count all-zero u32 words** (T1; count = the robot-bank count `robots().len()` — D133's headline: the latch is MP-lobby-claimed ONLY, never set on any SP path, so E's SP corpus construction is the all-zero bank by the same construction as the guest boot memset; the O1 plan dumps the same count-driven `$robot_count*4` bare span, D133/D134's dbx-plan form). RATIONALE (emit now vs keep-deferred): (1) D134 already landed the sfx row on EVERY O1 capture plan (all 12 regenerated, S0/S0W included — the row is T0), so every live capture dumps it on the original side; an E-side gap would convert a clean both-channel row into a permanent one-sided coverage finding in EVERY live verdict — noise in the DH-G1 gate reading; (2) both values are session-constant construction scalars — zero behavioral modeling risk, no engine seam touched (D133/D134 both recorded "engine consequence NONE"); (3) the re-baseline cost is mechanical and fully automated NOW (the 12 corpus chain pins + the synthetic pin in canonical_dump_gate.rs + the 11 differ_gate table rows) and only grows with every future scenario/chain pin; (4) timing: the live S0 session (queue item 1) is operator-gated but turnkey — landing this before it makes the S0 verdict clean (S0 is T0+TS: exactly the sfx row rides it; the latch rides the T1 scenarios S1+). CANONICAL FORMS + DIFFER WIRING: sfx-master-gate joins the existing u32 scalar normalizer groups on all three channels (E/O1/O2, field "value"); a live capture machine with sound DISABLED dumps 0 where E emits 1 → the T0 default field class (EngineBug) fires LOUD — intended, and the D134 fingerprint companion (one dbgprobe read of [0x10743c] at the anchor stop) stays the remedy, the D128 ACTIONPAN pattern. no-extract-latch is COUNT-PREFIXED (the §6a canonical style): the E form is `u32 count + count*4 zero bytes`; the O1/O2 normalizers convert the bare guest span by prepending `len/4`; the count field is classed STRUCTURAL like every other count word, so the robot-count scenario seams (D91/D103/D108 `_e_staging`) surface on it exactly as they already do on robot-bank.count — never a new finding class; the per-slot `slots[i]` fields default exact (all zero on SP). CONSEQUENCE: the E W6 row list grows by 2 (T0 10→11 rows/frame, T1 +1 on every T1 scenario); every canonical chain re-baselines DELIBERATELY (the live-session O1 comparisons pin against the NEW values from the landing commit); the DESIGN §6a E-gaps list drops both rows AND is corrected in place for staleness (history preserved): the D85-era list still named the destroy-family five + "all T2/T3" as gaps although W12-S3/S4/S6/S8 landed their emitters gated on the staging keys — the accurate current gap set after D136 is {variant-flag-bytes (T1); mortar-trail-bank, poi-bank (T2); the unmodeled T3 banks rising-debris/blast-bank/arrival-rides/door-rects/trigger-timers/pod-ring/exit-ring/objective-slots/escape-counters/tile-claims; s0-trigger (S0); every TS row except static-map-wh; all T4; all TI}. No capture-plan/O1-side change rides (D133/D134 already emit both rows there — this unit closes the E twin only, the exact inverse of the D133/D134 precedent "closing the EXD twin does NOT force E to emit"). LANDED in the SAME unit by worker ec979f34 claim 2 (commits 3e3bace decision + cfc6b4c impl): canonical.rs emit_frame (sfx row in the T0 block, latch after spread-claims in T1), differ normalizer arms on all three channels + the Structural count class, the differ_gate inv_frame latch fabrication (bare-span strip; the sfx row is identity), the synthetic grammar pins for both rows (sfx = `01 00 00 00`; latch = `01 00 00 00` + 4 zero bytes on the 1-robot fixture), the full chain re-baseline (S0 dac1cfd17bc7ede3, S1 a18cb11ac8e4314e, S2 d6649ce272ad6d96, S3 f4f5b4351e976ed5, S4 63ab5ac7679f6de7, S5 8a718339e0702fd6, S5B b72f57e0b8e7042b, S5C de5b80a6177aecdd, S6 c27bff339929339d, S7 b0db22840310e82a, S8 29fa2f400a10974b, synthetic 6517d1c0b7169446 + the synthetic frame digest c0268bf499a505c1), the DESIGN §6a table rows + E-gaps amendment (incl. the staleness correction), the DESIGN §9 gate-example pins, and the queue item-1 (S0 session) pin supersession note; verified green: 93 diffharness + 13 canonical_dump_gate + the differ_gate cross/double table + 76 bedlam-core + 132 bedlam-game lib tests, fmt + clippy clean, MANIFEST.sha256 clean before AND after the corpus-reading runs; the differ's cross-channel coverage counts are UNCHANGED on every scenario (both channels carry both rows — the rows compare clean, exactly the "cleanest for the live S0 verdict" outcome the decision wanted).

Nudge-Worker: ec979f34-56b3-4053-b3b2-b3b5afacd6ca

## D137 — 2026-08-23: P4.2/W11-prep — THE O2 STATIC-MAP-WH CAPTURE-FORM PIN (the queued SMALL unit; closes the LAST deliberate zero-field differ row). THREE decisions recorded: (1) THE EXW CENSUS (RE-EXW-SIM §7j.60, [verified]): the map w/h cells are [0x4eddec] (W) / [0x4eddf0] (H) — EXACTLY 6 writer stores in the whole .text (found only by grepping BOTH `mov DWORD PTR ds:` AND the short A3 `mov ds:` form — a DWORD-PTR-only grep undercounts by half): the map-volume loader FUN_0041dc5a pre-clear pair @0x41dd52/0x41dd5b + the two sign-extended u16 .TOT header stores W @0x41dd6a (+0) / H @0x41dd83 (+2) off the [0x4ede20] volume pointer, and the EDITOR\ZONE restore reload FUN_0044661b pair @0x446688/0x4466a1 (same header shape); every remaining site (95 W / 83 H) is a reader — the D124 debris bounds, the §7j.26 effects-mover kill bounds, the map-overlay loops, the cursor clamps, the §7j.47 restamp bound — no game-state writer exists; the sibling [0x4eddf4] = the W·H plane stride (loader `imul` @0x41dd88 → store @0x41dda5), NOT part of the row. (2) THE SPAN-FORM ASYMMETRY + THE PIN: the EXD twin pair (w 0x1074b8 / h 0x10748c, RE-EXD-MAP §5b [verified]) sits 0x2c apart with h LOW (the O1 row's 0x30 span: h@+0x00, w@+0x2c), while the EXW pair sits 0x24 apart with w LOW — the port shuffled the intervening neighbors and REVERSED the field order relative to address order, so the O2 raw form is NOT the O1 form relabelled. THE PIN: the O2 capture form = ONE contiguous span covering exactly the two cells (the EXD precedent; product cell excluded both sides) — base 0x4eddec, len 0x28 (= 0x24+4), w@+0x00, h@+0x24; the future W11 capgen emits this, `normalize_o2_row`'s static-map-wh arm parses it into the canonical (w, h) fields (the pre-pin zero-field arm would have rendered the row as 2 field-level coverage gaps on any E-vs-O2 compare). (3) CONSEQUENCE: engine NONE (the E canonical row already carries the TOT-header w/h since D85); differ: the O2 arm gains the parse, the differ_gate O2 fabrication is channel-aware since (inv_frame emits the EXD 0x30 span under O1, the EXW 0x28 span under O2), the tiebreak lanes re-verified, and a NEW E-vs-O2 cross assertion proves the row COMPARES CLEAN end-to-end through the real O2 normalizer path (coverage stays exactly 1 on S1 = move-target-words; the cross suite's S0 expect_coverage stays 0); registry row layout note amended; RE-EXD-MAP §5b row extended with the asymmetry. RE notes committed BEFORE the impl per the stream-survival rule. Verified: objdump-only from the committed exw-text-objdump.txt (+ the exw-gamemainhop.txt decomp cross-check), no Ghidra run, no corpus read; MANIFEST.sha256 clean before AND after; the full differ_gate suite + fmt + clippy green (worker a3532435 claim 2).

## D137-CORRECTION (2026-08-24, D138 landing unit): THE "0x24 APART / 0x28 SPAN" ARITHMETIC IN D137(2)/§7j.60 C-D WAS IMPOSSIBLE. The D137 pin claimed the EXW map w/h pair "sits 0x24 apart with w LOW" and pinned the O2 capture form as "base 0x4eddec, len 0x28 (= 0x24+4) — w@+0x00, h@+0x24". Both cells the claim itself quotes disprove it: 0x4eddf0 − 0x4eddec = **4** (adjacent u32s; the stride cell 0x4eddf4 immediately after; 0x4eddec+0x24 = 0x4ede10 ≠ 0x4eddf0). The §7j.60 A-table store sites (`mov ds:0x4eddec,eax` W / `mov ds:0x4eddf0,eax` H @0x41dd6a/0x41dd83 + the EDITOR pair + the 95/83 reader census) were ALWAYS correct — only the C/D gap arithmetic was fabricated (pattern-matched to the EXD 0x2c-gap story without computing). CAUGHT by the dbx-plan O2 compiler's registry-derived span assert (the first consumer that had to COMPUTE the span from the registry cells). CORRECTED PIN: O2 capture form = base 0x4eddec, **len 8 — w@+0x00, h@+0x04**; the field-order asymmetry story SURVIVES (O1's low cell is h, O2's low cell is w — the O2 form is still not the O1 form relabelled). Fixes riding D138: §7j.60 C/D + RE-EXD-MAP §5b + the watches.toml layout note corrected in place (history preserved); normalize_o2_row's static-map-wh arm (need 8, h@+4 — the 0x28 arm would have made every REAL live O2 capture fail the row length check structurally); the differ.rs o2_row_forms/o2_frame fixtures + the differ_gate inv_frame O2 fabrication; the dbx-plan O2 emitter + its map-wh resolve geometry assert.

## D138 — 2026-08-24: P4.2/W11-prep — DBX-PLAN O2 CHANNEL SUPPORT + THE D137 ARITHMETIC CORRECTION (the queued SMALL unit; the headless W11 prerequisite on the plan side). THREE decisions recorded: (1) **THE O2 PLAN FORM** (`dbx-plan --channel o2`, o1 default byte-identical to every committed plan — the existing `s*_plan_matches_committed_artifact` gates prove it): every watch/resolve/step address swaps to the registry row's `exw_addr` canon cell in flat `0x`-prefixed linear form (the W11 host ptrace driver reads EXW addresses directly, DESIGN §2 O2 — zero translation); the DOSBox boot/arm command machinery (env, boot_commands, arm_commands, boot_timeout/boot_retries) is REPLACED by a registry-derived `trigger` object {site = the s0-trigger EXW PresentEnd row 0x425a03, frame_counter = the EXW g_frame_count 0x46ae68}; `resolve_at=anchor` and the frames contract are channel-neutral; walk-phase keystore scenarios are REFUSED on o2 (the BPLM stop-indexed menu walk is DOSBox/O1 machinery, D84 — the channel flag never invents capture semantics); mission-phase inject/boot rows DO emit on the EXW seam cells (frame = the Nth trigger hit; injection-on-O2 is W11 driver policy — the rows are data). Byte-pinned artifact `capture-plans/S1-o2.json` (the tiebreak-lane scenario) + 4 new unit tests (the form asserts, the committed-artifact pin, the walk refusal, the step EXW-cell emission). (2) **THE D137 ARITHMETIC CORRECTION** (see the D137-CORRECTION entry above — the headline of this unit): the plan compiler's registry-derived span assert caught that D137's "EXW cells 0x24 apart / O2 = the 0x28 span @0x4eddec h@+0x24" was ARITHMETICALLY IMPOSSIBLE for the cells it quotes (0x4eddf0−0x4eddec = 4 — adjacent u32s, stride cell 0x4eddf4 right after). CORRECTED PIN: the O2 capture form = the 8-byte span @0x4eddec, w@+0x00/h@+0x04; the field-order asymmetry vs the EXD 0x30 span (h LOW) SURVIVES. Fixed everywhere the wrong form had landed: §7j.60 C/D, RE-EXD-MAP §5b, the watches.toml layout note, DESIGN §10-W7's pin paragraph, `normalize_o2_row`'s static-map-wh arm (need 8, h@+4 — the 0x28 arm would have failed every REAL live O2 capture structurally on the row length check), the differ.rs o2_row_forms/o2_frame fixtures, the differ_gate inv_frame fabrication. The corrected triangle (plan ↔ differ normalizer ↔ gate fabrication) is re-verified green. (3) **TWO REGISTRY CORRECTIONS the address swap forced** (both citing committed pins, no new RE): (a) robot-bank + no-extract-latch `exw_addr` count-cell parentheticals named 0x46ccbc (the TOTAL/cap twin) where the O1 semantic (EXD 0x11958c PER-PLAYER) twins 0x46cbd8 — the W8-prep count-mapping correction recorded 2026-08-22 but never propagated into the registry rows; the O2 `$robot_count` resolve now reads 0x46cbd8 (values coincide in SP; the semantic twin is the correct binding); (b) selection-triple's EXW list is FIELD-ordered (base/selected/size) but NOT ascending (selected 0x46cbdc is the HIGHEST; size 0x46cbd8 sits between) — the O2 row dumps cells[1] = 0x46cbdc (the D132 SELECTED-SLOT pairing) with geometry asserts, while O1 keeps cells[0] = 0x11954c (the EXD field-ordered list). The O2 emission SET is channel-symmetric modulo gaps: EXD-unmapped T2/T3 coverage rows stay deferred on BOTH channels (the differ's O2 arms cover exactly the aliased set; widening is a deliberate W11-era decision), and static-cursor-clamp (the EXD-only row) defers on o2 — 36 anchor + 28 per-frame + 7 deferred on the S1 shape. Verified: diffharness 98 tests green (incl. the 4 new + all 12 committed-plan byte pins), differ_gate + canonical_dump_gate corpus suites green, fmt + clippy clean, MANIFEST.sha256 clean before AND after the corpus-reading runs, no Ghidra run, no corpus write (worker c44a3c8b claim 2).

## D139 — 2026-08-24: P4.2/W11-prep — DBX-STITCH O2 TRANSCRIPT CHANNEL SUPPORT (the queued SMALL unit; the stitch side of the D138 plan form — the last headless-reachable W11 piece before the ptrace driver). THREE decisions recorded: (1) **THE CHANNEL-THREADED ANTI-GHOST RULE**: `runner::stitch` validates every transcript id against the registry through the DUMP HEADER's channel, not a global rule — O1 keeps its `exd_addr` rule verbatim (W4) and O2 gains the mirror: `exw_addr` must be non-empty or the stitch fails LOUD (`StitchError::NoExwAddress`, carrying the row's `note`). The rules are deliberately PER-CHANNEL MIRRORS, never global: a T3 row with NO EXD alias but a live EXW cell (e.g. debris-stager, exw 0x476fbc — unpinnable on O1, E-only coverage there) dumps LEGALLY on O2 (the EXW cell IS the canon on that channel and the D138 plan emits it), while the one EXD-only registry row (static-cursor-clamp, TS — empty exw_addr, the host-space cursor clamps per RE-EXW-INPUT §4) rejects on O2 and stays legal on O1 (it carries the EXD pair 0x1074ac/0x1074b0). Pre-D139 the O2 path enforced NOTHING — a fabricated-or-driver O2 transcript could carry any registry row; the differ would have surfaced the mismatch only downstream as coverage noise. Engine and O3 channels carry no address rule (E fabricates from engine state, O3 is W10). (2) **`dbx-stitch --channel o1|o2`** (o1 default, O1 behavior byte-identical): selects the dump-header channel, hence the address rule + the manifest's channel field ("O2:EXW/Wine"); the build identity on an o2 run should be the watched EXW binary. The W3 stitch/encode/digest/chain machinery was already channel-agnostic by DESIGN §3 — the O2 transcript needs no new formats, which is exactly why the fabricated O2 tiebreak lanes of D87/4591f52 were already correct-by-construction and stay green unchanged. (3) **VERIFICATION**: new `runner::tests::stitch_o2_channel_rules` (the D138 row forms end-to-end: the 8-byte ADJACENT static-map-wh span w@+0x00/h@+0x04 stitches + decodes channel-marked; the static-cursor-clamp LOUD rejection; the per-channel mirror both ways on debris-stager/static-cursor-clamp) + new `differ_gate::s0_o2_transcript_stitch_channel_rule` (the real S0 run fabricated through the existing channel-aware `inv_frame`, stitched under O2 THROUGH the enforced exw_addr rule, decoded channel-marked with the 8-byte span intact; the EXD-only row appended to the anchor frame refuses; the same row re-fabricated on the O1 forms stitches clean). No production engine change; no registry change; no plan artifact change (the D138 S1-o2.json pin untouched). With this unit the O2 triangle is channel-complete headless: plan (dbx-plan D138) ↔ differ normalizer + tiebreak (D137/D138) ↔ stitch (D139); the host ptrace driver itself remains the operator-gated W11 unit. Verified: diffharness suite green, differ_gate corpus suites green, fmt + clippy clean, MANIFEST.sha256 clean, no Ghidra run (worker 74bae49c claim 2).

## D140 — 2026-08-24: P4.2/W11-prep — THE CAPGEN-O2 TRANSCRIPT EMITTER SKELETON (the queued SMALL unit; the runtime-side producer of the O2 DBXCAP — the headless plan→driver→transcript→stitch→differ loop closed before any Wine session). THREE decisions recorded: (1) **THE CONTRACT SPLIT + DBXFEED v1**: the W11 ptrace driver (still operator-gated) services the D138 o2 plan — trigger hits at `trigger.site` 0x425a03, `process_vm_readv` per plan row — and logs a **DBXFEED v1** read/write log (`DBXFEED v1` / `kind synthetic|driver` / `hit <n>` blocks / `read|write <addr> <len> <hex>`); `tools/runtime/capgen-o2.py` is the pure plan interpreter + transcript emitter that validates the feed against its own walk 1:1 (every read's addr+len, hit numbering = capture frames with the anchor as hit 1 — the same numbering `compile_steps` pins for inject rows, resolving the D138 comment's loose "Nth trigger hit after the anchor" wording — and the full inject arithmetic RE-DERIVED: plain writes byte-exact, `op:command` ring appends from the logged count-cell read including base+count*stride + zero-extension + the count bump, `op:pad` step-ons with the D86 loader-mark check and the xyz triple writes). ONE walker (`plan_walk`) drives both the validator and `--synthesize-feed` (the reference mini-driver — deterministic LCG bytes per (addr, hit) with internally-consistent resolve statics, prefix counts and the +1-per-hit frame counter), so the generator and the checker can never diverge. Synthetic feeds carry `kind synthetic` and mark the transcript SYNTHETIC (anti-ghost, the s0-replay fixture precedent); the frame-counter alignment check (trigger.frame_counter) warns + records a transcript comment on drift. (2) **THE FRAME-1 ROW-SET FINDING (headline discovery)**: on EVERY committed plan the per-frame rows are a SUBSET of `anchor_watches` — the anchor list IS the frame-1 row set (TS statics + a full T0 sweep ride frame 1), so a literal `anchor_watches + watches` concatenation emits DUPLICATE ids and the stitcher's `canonicalize_frame` rejects `DuplicateWatchId` (dump.rs). capgen-o2 emits the deduped union keep-first; **the same landmine exists in the O1 `dbx-capgen.py` frame-1 path** (`dump_rows(frame, anchor_watches + watches ...)`) — every live O1 session would fail at `diff stitch` until it dedupes the same way (the dbgprobe gates never see it: the probe plans carry no anchor_watches; queued as its own small unit). The fabricated-transcript test lanes never saw it either because they build from E frames whose ids are unique by construction. (3) **THE HEADLESS SMOKE (all green, `tools/runtime/capgen-o2-smoke.sh`, unattended-safe: no Wine/ptrace/game/corpus read, MANIFEST clean pre+post)**: (a) dbx-plan --channel o2 byte-pins the committed S1-o2.json; (b) the full 401-frame S1-o2 chain — synthesize feed → emit transcript → `dbx-stitch --channel o2` against the real S1 scenario (20.5 MB dump, chain b436fa77642c94fc, manifest channel O2:EXW/Wine, frame_count 401 = the scenario 400 + anchor contract) → `dbx-diff` self-cross PASS 0 findings (full decode + normalize_o2_row intake, both channels O2); (c) the D139 loud rejection re-proven at the CLI (a `static-cursor-clamp` row spliced into the transcript refuses with NoExwAddress); (d) the emitter's own contract (a feed truncated at hit 401 refuses loud); (e) the inject grammar end-to-end: S3-o2 compiles its 8 `op:command` rows on EXW cells and the chain runs (frame-1 injected flag set, stitched, chain 52f6044c2033cb34); (f) emitter determinism (re-emission byte-identical). With this unit the headless O2 loop is COMPLETE end-to-end; the only remaining W11 piece is the operator-gated ptrace driver, whose observable contract is now spelled by DBXFEED v1. No Rust change; docs + tools only. Verified: py_compile (no flake8 on host), diffharness 82 tests green (24 lib + 35 + 20 + 3 suites incl. the S1-o2 byte-pin), cargo fmt --check + clippy -p diffharness clean, MANIFEST.sha256 clean before AND after, no Ghidra run (worker 3b207215 claim 2).

## D141 — 2026-08-24: P4.2/D140-followup — THE O1 DBX-CAPGEN FRAME-1 DEDUPE FIX (the queued SMALL unit; closes the D140(2) landmine BEFORE the operator S0 live session). dbx-capgen.py's frame-1 path concatenated `anchor_watches + watches` literally — on every committed plan the per-frame rows are a SUBSET of anchor_watches, so a live O1 session would emit DUPLICATE watch ids at frame 1 and `diff stitch` rejects the transcript (`canonicalize_frame` DuplicateWatchId, dump.rs). FIX: frame 1 now dumps the DEDUPED union keep-first via the new module-level `dedupe_frame1_rows()` — mirroring the capgen-o2 semantics exactly — and the transcript summary line reports the deduped frame-1 count. VERIFIED headless, unattended-safe, no game, no corpus read: (a) the committed check `tools/runtime/capgen-frame1-dedupe-check.py` imports the REAL shipped function (never a copy) and proves over all 13 committed plans that the frame-1 list has unique ids, EQUALS the anchor list in anchor order (the subset property), every per-frame id rides the anchor set, and the landmine expression is absent from the source (the raw concat would have duplicated 11-30 ids per plan = every per-frame row); (b) all dbgprobe probes re-GREEN through the changed path (gate, flow, inject, walk, pad both legs) — the probes carry anchor_watches but their anchor/per-frame id sets are DISJOINT (zero overlap), which is why the gates never tripped the duplicate pre-fix; (c) py_compile clean. No Rust change (no cargo needed); MANIFEST.sha256 clean. Docs: RUNTIME.md D140 finding note closed out. (worker 9f4a1111 claim 2)
## D142 — 2026-08-24: P4.2/W10-prep — THE O3 8STREET COMPARATOR FEASIBILITY (the last unstarted W item, docs-only). The landing study is **docs/O3-8STREET-COMPARATOR.md** (referenced from DESIGN-DIFFHARNESS §10-W10). Decisions: (1) **PIN + TOOLCHAIN**: O3 rebuilds `github.com/8street/Bedlam` @ a8622e663d35c00c331a88880c20abfefccdc0eb (tree f9df7045; bedlam.asm da77a5e4…, bedlam_data.inc e26105c0… sha256s recorded in the note) — the IDA disassembly of Win95 BEDLAM.EXW + C++/SDL2 shell; Linux build = clang -m32 + JWasm + vendored libsmacker + i686 SDL2/SDL_mixer ≥2.0.12 (their CI recipe proves reproducibility); the repo carries NO top-level license → stays local/test-only, nothing enters this repo. The FIRST build is OPERATOR-GATED (sudo apt + network for JWasm/SDL); compile.sh alone is unattended-safe after; O3 runs against a STAGED corpus copy under runtime/ — the reconstruction WRITES SAVES/+BEDLAM.LOG, so game-data/ is never its working folder. (2) **THE MEMORY-LAYOUT FINDINGS (headline)**: 8street resolves every game cell by SYMBOL NAME (ld places the ELF; IDA names are the only stable handles) — never map EXW addresses by arithmetic. bedlam_data.inc is a sequential mirror of EXW .data(0x454000)/.bss(0x45B000..0x4EFB60) BUT carries **8 drift defects** (first at the 0x4DC6CC..E0 gap: seven anonymous `dd ?` where the IDA names imply four; growing +48 by 0x4DE660, correcting to −208 by 0x4EDD5C, −1188 by 0x4EEE08) — a full-drift simulation + directive census (only db/dw/dd/align exist) pinned the ledger. CROSS-VALIDATION: simulated emission positions of 8street semantic symbols re-anchor EXACTLY onto independently-pinned registry cells — current_money≡money 0x46ae70 (Δ0), difficulty≡0x46cbf8 (Δ0), robots_available≡the D89 per-player cell 0x46cbd8 (Δ0), game_mode≡mode 0x4edb88, zone/zone_level≡zone/mission 0x4edd8c/88, rnd_seed1/2≡rng-state-a/b 0x4ede48/4c, sound_enable≡sfx-master-gate 0x4ede58, mission_square≡static-tot-volume 0x4ede20 — zone/rnd/sfx all land at precisely the ledger's −208, proving the symbols ARE our cells displaced only by filler defects. Row resolution = three cases: (a) named .inc symbols (hook references them directly), (b) anonymous filler (fork adds zero-size labels via the drift-aware simulation + writer-xref check; safe symbol+delta arithmetic only in the Δ0 region <0x4DC6D0), (c) C++-shell cells (PRESSED_KEY_ARR/CURSOR_POS/GAME_UPDATE_TIMER are extern "C" in CPP_sources). The EXW frame-counter cell 0x46ae68 is DEAD in 8street — the hook numbers frames itself (equivalence seam, never a finding). (3) **THE HOOK FAMILY + INTAKE**: H1 frame-tail = the game_level wait site [ASM] loc_448730:99697 (post-redraw/present ≡ EXW 0x425a03); H2 anchor = loop-head first entry loc_447E6A:98943 (post-load, TS statics settled); H3 inject = the D77 §3 seams through the three cases; H4 the hook emits **DBXCAP v1 directly** — reusing stitch→encode→chain→differ unchanged (the D139/D140 pattern minus the driver, in-process). Remaining IN-REPO units (unattended-safe, no engine change): dbx-stitch --channel o3 + the O3 anti-ghost rule (exw_addr mirror) and the differ O3 field map + o3-seam classification (differ.rs currently rejects Channel::O3Street). Never-comparable classes (o3-seam, never findings): registry-backed config TS rows (8street reads OPTIONS.BDL/file-existence where EXW reads HKCU registry — RE-EXW-TITLEMENU §7j.56/D128), sfx-gate writer, volume-key scancode swap, speech-always-on, CDDA-off; the 9ms-timer deviation is WALL-CLOCK only (per-frame logic consumes tick counts — frame-indexed diffs stay valid); SP robot-count parity holds (robots_available/game_mode writes verified in the disassembly at [ASM] 18053-65/81710+). VERDICT: feasible; rebuild operator-gated and PARKED until a three-way tiebreak is wanted (D77 says late); the two in-repo diffharness units may land any time. No engine/Rust change; MANIFEST clean before AND after the clone reads (clones are outside game-data/, bracket is belt-and-braces). (worker 5ae99a92 claim 2)

## D143 — 2026-08-24: P4.2/W10-impl-a — DBX-STITCH --CHANNEL O3 + THE O3 ANTI-GHOST RULE (the queued SMALL unit; the first of the two in-repo W10 units left by D142 §5/§8 — the stitch side). THREE decisions recorded: (1) **THE O3 ADDRESS RULE = THE O2 MIRROR**: `runner::stitch` now binds the O3 dump header's channel to the registry `exw_addr` cell exactly as O2 (D139) binds it — `Channel::O3Street` with an empty `exw_addr` row fails LOUD with the SAME `StitchError::NoExwAddress` (carrying the row's note). Rationale (D142 §3): the 8street reconstruction rebuilds EXW state — same cells, same layouts — so a row with no EXW canon cell can never legitimately appear in an O3 dump; the one live-registry EXD-only row (static-cursor-clamp, TS — the host-space cursor clamps, RE-EXW-INPUT §4) rejects on O3 exactly as on O2, while EXD-gap rows with live EXW cells (the T3 effect rows, e.g. debris-stager 0x476fbc) remain LEGAL there (per-channel mirrors, never global — the O1 `exd_addr` rule is untouched). The runner module doc + the NoExwAddress doc/Display amended ("O2/O3"). (2) **`dbx-stitch --channel o3`**: the CLI accepts o1|o2|o3 (o1 default, O1/O2 behavior unchanged); an O3 run's build identity is the 8street build sha256 and the manifest names "O3:8street". The W3 encode/decode machinery was already channel-complete (Channel::O3Street code 3 landed D78) — no format change. (3) **GATES (fabricated transcripts, the D140 smoke pattern; the differ O3 rejection stays the DOCUMENTED D142 §5 gap until the W10-impl-b field map lands — asserted as state, never papered over)**: new `runner::tests::stitch_o3_channel_rules` — an O3 transcript in the O2 raw form (static-map-wh = the D138 8-byte ADJACENT EXW span) stitches + decodes channel-marked code 3 with manifest O3:8street; static-cursor-clamp refuses LOUD; debris-stager stitches clean on O3 yet still refuses on O1 (the mirror both ways); determinism byte-identical re-stitch; `normalize_dump` on the O3 dump still refuses UnsupportedChannel. New `differ_gate::s0_o3_transcript_stitch_channel_rule` — the REAL S0 E run (chain dac1cfd17bc7ede3) fabricated through `inv_frame` with the new O3Street arm (O3 raw forms = the O2 forms; the test's static-map-wh match arm now shares the EXW 8-byte span construction) stitches THROUGH the enforced rule, decodes channel-marked with the span intact, byte-identical re-stitch + chain, and the EXD-only row appended to the anchor frame refuses at stitch. CLI smoke: o3 manifest/dump + determinism, ghost row refusal (registry note shown), `o4` error. No engine change (the differ_gate test-file fabrication lane only); no registry change; no plan artifacts (O3 rows are O2-form by construction — the plans are reused as-is per D142 §5). VERIFIED: diffharness full suite green, bedlam-game 191 green (differ_gate 4 lanes, canonical_dump_gate 13, corpus read), fmt + clippy clean, MANIFEST.sha256 clean before AND after the corpus runs, no Ghidra run. (worker a42f254c claim 2)

Nudge-Worker: a42f254c-ab47-4897-9837-b5a3467144be

## D144 — 2026-08-24: P4.2/W10-impl-b — THE DIFFER O3 FIELD MAP + O3-SEAM CLASSIFICATION (the queued SMALL unit; the second and LAST in-repo W10 unit — W10 in-repo work is now COMPLETE, only the operator-gated 8street rebuild + live captures remain). Spec committed first as O3-8STREET-COMPARATOR §5a (commit 7d28bc2). THREE decisions recorded: (1) **THE O3 FIELD MAP = THE O2 MAP VERBATIM**: `normalize_o3_row` delegates to `normalize_o2_row` with zero normalization differences — the reconstruction rebuilds EXW state (same cells, same layouts, D142 §3), so O3 raw rows are O2-form; `normalize_frame`'s guest arm now covers O1|O2|O3 with a per-channel dispatch and the `UnsupportedChannel` rejection is GONE (the D142 §5 differ gap closed). The D90 move-target splice + the lone-span guard apply identically on O3. (2) **THE SEAM LEDGER (§5a)**: `Class::O3Seam` (name `o3-seam`, NON-failing — PASS-WITH-NOTES at worst) is assigned when ANY compare side (A, B, or the tiebreak T) is `Channel::O3Street` and the row matches the ledger, which is TWO registry-driven matchers: row-id (the live `sfx-master-gate` — 8street feeds sound_enable from SAVES/OPTIONS.BDL where EXW reads HKCU per D128 / EXD parses CONFIG.BDL per D134 / E dumps constant 1 per D136) and `exw_addr` BASE-CELL (the whole D128 §7j.56 registry-config family: 0x4ede58 SOUND gate, 0x4ede5c sister, 0x4eb93c SPEECH forced-always-on, 0x4edbd8 ACTIONPAN, 0x46cca4 CINEMATICS, 0x4eba1c LANGUAGE SDL-locale auto-detect, 0x4e444c DEFAULTNAME — so future config rows are caught automatically before any id joins the table). Semantics: equality stays SILENT (a clean capture self-crosses PASS 0 findings); divergence reports the ledger reason VERBATIM, never Structural/EngineBug/T2 notes; row/field COVERAGE asymmetry on a seam row classifies o3-seam too (never coverage noise); and seam rows are EXCLUDED from tiebreak arbitration — an OPTIONS.BDL-fed vote is not canon evidence, so a seam row never produces EngineBug/OriginalDivergence while O3 participates. TWO deliberate NO-CLASSIFIER omissions (§5a, decisions not oversights): the volume-key scancode pair (0xC8/D0→0x48/0x50, [ASM] 98948/98990) is a TRIGGER deviation, not a FEED deviation — the volume cell 0x4ddb2c is written by the same handler with the same ×0x147≫7 math (D134) and captures inject COMMANDs, never raw keys, so an arrow-key drift on a live O3 capture is a GENUINE finding, never a seam; CDDA-disabled is a behavior deviation with no EXW canon watch cell — it surfaces as exactly the three-way disagreement O3 exists to localize. (3) **GATES**: `differ_gate::s0_o3_transcript_stitch_channel_rule` extended past its old rejection assertion — (1) the fabricated O3 transcript of the REAL S0 E run (chain dac1cfd17bc7ede3) SELF-CROSSES PASS with ZERO findings; (2) a seeded sfx-master-gate divergence on one side → exactly ONE o3-seam finding (row/field/reason asserted, verdict PASS-WITH-NOTES, zero EngineBug/Structural); (3) the SAME perturbation on money still FAILs EngineBug (the ledger is selective, never a blanket suppressor); (4) a SYNTHETIC registry row on the ACTIONPAN cell 0x4edbd8 (TS tier, live exw_addr, stitched through the real O3 address rule) seam-classifies end-to-end via the cell matcher; (5) the same seeded pair under O2 headers → plain EngineBug FAIL (the class binds the O3 channel only, the D139/D142 per-channel pattern); the static-cursor-clamp stitch refusal still asserted (W10-impl-a, unchanged). `runner::tests::stitch_o3_channel_rules` (e) flipped to the landed state (normalize succeeds, row-count + frame-counter/static-map-wh presence). No engine change; no registry change (the synthetic row is test-local); no plan artifacts (O3 rows are O2-form — plans reused as-is per D142 §5). VERIFIED: diffharness 80 tests green, differ_gate corpus lane green, fmt + clippy clean, MANIFEST.sha256 clean before AND after the corpus run, no Ghidra run. (worker 59d0e7d5 claim 2)

## D145 — 2026-08-25: Static differential proof became the default for T1 semantics

For deterministic T1 behavior, parity means **same deterministic input/state bytes →
same canonical output bytes/fields**. The default proof is therefore an independently
reconstructed EXW/EXD logic oracle compared byte/field-exact against Rust, rather than
a live capture. Static EXW disassembly remains canon. The oracle must not reuse the
production parser, loader, normalizer, or inverse generator; it must pin exact corpus
identities, use the full corpus where applicable, and prove test sensitivity with a
temporary mutation.

Evidence already landed: `bd91c10` covers 37 TOT→DAT/PAD post-load images; `56918c5`
covers 7 CGR banks × 128 maps; `390acb9` covers 37 mission TOT/DAT/BIN/LNK transforms.
The strict S0 semantic ledger is now **6/27 independently covered rows**:
`static-map-wh`, `static-dat-volume`, `static-cgr-volume`, `static-tot-volume`,
`static-bin-terrain`, and `static-lnk-map`. **21 rows remain; S0 is not complete.**

Operator-live S0 retired as a semantic-parity prerequisite. Live capture remains
optional for channel/address qualification and irreducible hardware, timing, or
perceptual behavior; the differential harness remains useful for dynamic seams and
divergence. Static differential tests may supersede semantic rows when they provide
stronger whole-corpus evidence. Current responsive-capture WIP is preserved, but no
longer blocks semantic work. This decision does not statically close T0 RNG/session
behavior or trigger/address placement.

## D146 — 2026-08-25: P4/static-parity/S0-07 — the retained PAD-slot bank row `static-pad-slots` independently covered (strict S0 coverage 7/27)

THREE decisions recorded. (1) **THE STAGED-BANK SEMANTICS ARE NOW FULLY PINNED**
(instruction-level re-verification of the EXW PAD staging loop
`FUN_0041dc5a` @0x41de44..0x41df03, committed as RE-EXW-SIM §7c.5 +
FORMATS §10 amendments): the whole 999×8 bank at 0x4e44f8 is
memset-0 BEFORE parsing (`FUN_00402965` @0x41de62 — the stos-ladder
memset), so no cross-mission stale tail is possible; the loop stages
each record's `x` word BEFORE the 0xFFFF check (even the terminator's
0xFFFF lands in the bank), exits on `sar 16 == -1` @0x41defa leaving
the terminator slot exactly `{active=0, x=0xFFFF, y=0, z=0}` (y/z
never read, active never written), and all slots past the terminator
stay all-zero with their file bytes never read (ZONEB/M3's orphan
record is invisible to the runtime bank). The EXD twin
(0x2e7a0..0x2e85d) is the IDENTICAL algorithm from the same source —
memset twin 0x12206, u16-read twin 0x2d5c8, same terminator check/
active:=1/DAT stamp/999 bound. The watches.toml layout string was
corrected (u16 active word, not "u8 pad id"). (2) **THE ORACLE**
(commit cd70efe, `engine/bedlam-core/tests/static_pad_slot_differential.rs`):
an independent bytes-only transcription of the loop (no production
parser/loader/terrain helper reused) builds the exact 7992-byte staged
image + live-run length for all 37 shipped missions and compares it
field-exact against the Rust target's retained bank —
`Terrain::pad_slots` (the live run, file order, active implicitly 1)
materialized into the same 8-byte record form. The INACTIVE surface
(terminator slot bytes, all-zero tail) is unretained by Rust and is
asserted against the statically pinned constants — never fabricated
as Rust output; the omission is unobservable through the retained
seams because every original consumer gates on active≠0 (probe
§7j.40/1, elevator stager §7j.21, scanner icon FUN_0041ee20). Corpus
identity pins: canonical 37-mission set, level tally
{0:310,1:173,2:51,3:50,4:62,5:47,6:8}=701, live-run extremes 2..114,
ZONEA/M1=114 (slot 0 (5,61,0) … slot 113 (18,24,4)), ZONEB/M3=6 with
the ignored orphan (51,16,3) at index 7; every live record in the TOT
volume (the original's write is unchecked; shipped values in range).
(3) **SENSITIVITY PROVEN BY TEMPORARY IN-MEMORY MUTATION**: a live
x-byte flip moves exactly that slot's staged x field; flipping every
byte of the post-terminator orphan leaves the staged bank
byte-identical (a parser that over-reads — the D112 dead-break bug
class — fails this); rewriting the terminator record live extends the
run by exactly one and re-stages the old terminator slot active.
Verified: bedlam-core suite green (9 binaries), fmt + clippy clean
(workspace), MANIFEST.sha256 clean before AND after the corpus reads,
no Ghidra run. Strict S0 independent coverage is now **7/27 rows**
(`static-map-wh`, `static-dat-volume`, `static-cgr-volume`,
`static-tot-volume`, `static-bin-terrain`, `static-lnk-map`,
`static-pad-slots`); 20 rows remain. (worker f25d060f claim 1)

## D147 — 2026-08-25: P4/static-parity/S0-08 — the y_line/z_base table row `static-yline-zbase` independently covered (strict S0 coverage 8/27)

THREE decisions recorded. (1) **THE TABLE SEMANTICS ARE NOW FULLY
PINNED AND ONE GLOSS CORRECTED** (instruction-level re-verification of
the EXW table-build loops, objdump-only from `ghidra-project/
exw-text-objdump.txt` / `exd-text-objdump.txt`, committed as RE-EXW-SIM
§7c.3 + the RE-EXD-MAP row amendment): **y_line has h dwords at
0x4ea900 (y·w for y in 0..h−1), NOT "h+1 dwords"** — the loop bound is
`h·4` under `jl` (@0x41ddbe; the §7c.3 gloss claimed a boundary entry
at h that the code never stages and no consumer ever reads; the sweep's
y bound is h @0x41de07). **z_base has exactly 8 dwords at
0x4eaacc..0x4eaae8** (z·w·h, stored factored as w·(z·h) with the
offset pre-incremented — the store base 0x4eaac8 / EXD 0x107714 is an
ADJACENT SCREEN-SCALE CELL, not a table entry: EXW writer 0x424da6, EXD
zeroed @0x14794; the watches.toml exd_addr dropped the bogus third
cell and the layout string now carries the exact extents). Census: the
four stores 0x41ddb1/0x41ddd9/0x4466c7/0x4466ef are the ONLY writers of
the two spans program-wide; the SECOND producer pair @0x4466bd..0x4466f8
(FUN_0044661b, called from the brief-screen loadout site 0x43d1a5 that
loads FULLFONT/BRIEF/palettes/SFX + the mission .TOT/.BIN/.DAT into
fresh arenas) re-runs both loops instruction-for-instruction — no
0x302 copy, no sweep, no PAD on that path. EXD twin 0x2e713..0x2e74b:
y_line 0x8b78c (h dwords), z_base 0x107718..0x107734 — identical
algorithm. (2) **THE ORACLE** (`static_yline_zbase_differential.rs`):
Rust retains NO such bank (indexes z·w·h + y·w + x inline) — the row's
parity content reduces to the retained dims plus the exact staged
extents, so the unit compares a TOT-header-only transcription of both
loops (bytes only, no production parser/loader/helper reused) against
a test-only representation built from `Terrain::size()` across all 37
missions, byte/field-exact, and PINS the corpus invariants that make
the reduction sound: **TOT[0..4] == DAT[0..4] on every shipped
mission** (the original builds the tables from the TOT header while
Rust takes its dims from the DAT header — the divergence is real but
unobservable on the corpus, and the gate asserts the agreement rather
than assuming it), dims {25×75 ZONEA/M1, 100×100 ×35, 100×25
ZONEG/M1}, DAT sizes exactly 4+8·w·h, the volume-identity boundary
z_base[7]+y_line[h−1]+(w−1) == 8·w·h−1, z_base[0]==y_line[0]==0. NO
production seam added: retaining the tables in `Terrain` would be
fabricated parity (no Rust consumer reads them — the values are pure
(w,h) functions and unobservable through the inline indexing). (3)
**SENSITIVITY PROVEN BY TEMPORARY IN-MEMORY MUTATION**: a TOT w-byte
bump changes every y_line entry y≥1 and every z_base entry z≥1
(y_line[0]/z_base[0] pinned 0) and makes the differential FAIL against
the un-mutated Rust side (a TOT/DAT header disagreement is rejected,
not absorbed); a TOT h-byte bump grows the y_line EXTENT by one entry
while leaving the existing entries byte-identical (the h-entry count
is load-bearing); a DAT header bump makes the Rust loader reject the
file outright. Verified: bedlam-core suite green, fmt + clippy clean,
MANIFEST.sha256 clean before AND after the corpus reads, no Ghidra
run. Strict S0 independent coverage is now **8/27 rows** (the D146
seven + `static-yline-zbase`); 19 rows remain. (worker 2b25b994 claim 1)

## D148 — 2026-08-25: P4/static-parity/S0-09 — the .BDG type-table row `static-type-table` independently covered (strict S0 coverage 9/27)

THREE decisions recorded. (1) **THE STAGING SEMANTICS ARE NOW FULLY
PINNED + ONE FORMATS ERRATUM** (instruction-level re-verification of
the EXW loader leg FUN_0041a4f8 @0x41a5d6..0x41a7ef + the EXD twin
FUN_0002adb4, committed as RE-EXW-SIM §7j.61 + FORMATS §16/§17
amendments + the RE-EXD-MAP row re-pin): the WHOLE 282×0x4E = 0x55EC-B
table at 0x4dedf2 AND 0x9C40 B of the bank arena are memset-0 before
every load (no cross-mission stale tail — the D146 finding repeated
one bank further); the raw control word is STAGED at row+0 BEFORE the
==1 test (0 on all 2527 corpus empty rows); empty rows leave +2..+0x4E
memset-0 including the four bank pointer slots; count@+0x12 = the
NONZERO-SELECTOR count computed on ACTIVE rows only (census
{0:554, 1:3755, 2:1304, 3:884, 4:506, 5:904} — 554 active rows carry
0, it is not a presence flag); the four banks are read into CONSECUTIVE
arena slots in DISK ORDER (cursor += 2·W·H·D per read) so the §7j.32
current/under interleave lives ONLY in the row pointer slots; the bank
byte count is recomputed from the STAGED W/H/D. Displacement census:
0x4dee04 (count) has exactly ONE .text site = the loader store —
**write-only state**; +0x3E/+0x42 stores only (dead editor payload,
§7j.32 confirmed); +0x46/+0x4A read by the destroy restore
(0x41ab59/72/8a); selectors 0x4dee08 = the loader count loop + the
destroy-tail cases. **FORMATS §16's "max (3,3,3)" footprint claim was
WRONG: 113 distinct tuples, W/H ≤ 10, D ≤ 8, max (10,10,5) = 500
cells at ZONEF/M1 #184**; hp domain reaches −1 (signed on disk);
chain domain {0,1}. (2) **THE ORACLE** (commit fcb8fb2,
`static_type_table_differential.rs`): an independent bytes-only
transcription of the loop (no production parser/loader/destroy helper
reused) compared FIELD-EXACT against the Rust target's retained bank —
`ObjectTypeTable::from_bdg_bytes` (staged verbatim into
MissionSim::object_types by stage_destroy_family) — across all 37
missions: classification, W/H/D/hp/chain/type, all five effect
entries, and the four banks under the §7j.32 disk→slot mapping, plus
the arena layout (consecutive slots, per-mission span 6728..27288 <
0x9C40). The two write-only surfaces are deliberately NOT retained and
NO seam is fabricated: the count word is pinned through the derivation
identity (the original's count == the nonzero-selector count of the
RETAINED effects) and the control word through the corpus-pinned 0/1
classification. (3) **SENSITIVITY PROVEN BY TEMPORARY IN-MEMORY
MUTATION**: bank-byte / hp-byte / selector-value bumps each move
exactly the staged field in BOTH sides (agreement under mutation +
detection against the clean side; ZONEA/M1 rec-0's four bank words
53/1189/2/0 are pairwise distinct so no slot permutation can absorb a
bank bump, and 7904/7907 rows have non-identical banks); a 1→2
selector bump keeps the derived count 1 (presence-only identity); a
control flip 1→0 desyncs the grammar (oracle EOF-exact precondition +
target None); a 0→1 empty-row flip record-shorts the file — the oracle
rejects while the target ACCEPTS an EOF-short walk (a documented,
corpus-unreachable divergence from the original's memset-padding
bounded loop, still caught by the differential's row-extent check);
trailing bytes are rejected by the target, stricter than the original.
Verified: bedlam-core suite green (10 test binaries), fmt + clippy
clean, MANIFEST.sha256 clean before AND after the corpus reads, no
Ghidra run. Strict S0 independent coverage is now **9/27 rows** (the
D147 eight + `static-type-table`); 18 rows remain. (worker e473f5db claim 1)

## D149 — 2026-08-25: P4/static-parity/S0-10 — the `.MIN` mask-bank row `static-min-bank` independently covered on the original side; Rust retention deliberately NONE (presentation-half)

**(1) RE first (0ebb184).** The EXW/EXD `.MIN` loader + bank + consumers
re-verified instruction-by-instruction from the objdump texts
(RE-EXW-SIM §7j.62): ArenaAlloc 0x7530 @0x41dabd..0x41dac7 with NO
memset anywhere (FUN_0041db89 is a pure cursor bump); the loader leg
@0x41dcd8..0x41dcf3 is a verbatim whole-file read of the ZONE-scoped
`EDITOR\ZONEX\MISSIONX.MIN` (the second string triple 0x4597ba..0x4597c7)
— no header skip, no transform, no 0x7530 cap; the displacement census
closes at exactly 3 .text sites with ONE runtime reader, the 4×4
territory stamp FUN_00402ab8 (mask byte 0 → transparent, else XLAT
MAPTRAN[variant]; cw = LNK/LNG word[TOT word], cw==0 skipped), which is
also the FIRST verified runtime consumer of the LNK permutation table
(FORMATS §5's rotation hypothesis gains its anchor). EXD twins identical
(0x2e3f0/0x2e641/0x12df3). Whole-corpus census: 7 zone files (A≡D
byte-identical), reachable-entry sets per zone under BOTH language
gates, and the **stale-tail-never-read proof** — every nonzero reachable
cw·16+16 ≤ file size, so the un-zeroed arena tail beyond the file prefix
is dead bytes at runtime.

**(2) Oracle (cec30a7).** `static_min_bank_differential.rs` transcribes
the loader + consumer projection independently (no production reuse) and
pins corpus identities (per-zone census, per-mission max cw, A≡D,
LNK-vs-LNG variant fact, TOT type max 1868 < 8192). **The actual side is
deliberately empty**: the bank is presentation-half (D17) — its only
consumer writes backbuffer pixels, never engine state, never in the hash
surface — and bedlam-core retains nothing. No seam is added: a retained
`Vec<u8>` with zero Rust consumers would be fabricated parity. The
row's parity status = original-side pinned + Rust absence documented.
Sensitivity proven by temporary in-memory mutation (reachable-entry flip
moves exactly one stamp pixel; dead-tail flip touches no reachable
surface; a poisoned LNK lookup one-past-the-file is caught by the bound;
a synthetic >0x7530 file is rejected loudly while the original read is
uncapped — never shipped).

**(3) Queued gaps (concrete, one line each):** resolve the dbx-plan
deferred `static-min-bank` extent to PtrCell 0x7530 (file untouched this
unit — in flight with unrelated O1-boot WIP), and the presentation-phase
map-overlay territory-stamp producer (S5+ display work, D17/D50 scope).
Strict S0 independent coverage is now **10/27 rows** (the D148 nine +
`static-min-bank`); 17 rows remain. (worker 95c99db8 claim 1)
## D150 — 2026-08-25: P4/static-parity/S0-11 — the tile-claim bank row `static-claim-bank` independently covered original-side with a CONCRETE Rust staging gap queued (strict S0 coverage 11/27)

**(1) RE first (2646ce8).** The 0x46af58/0x119564 claim bank decoded
whole (RE-EXW-SIM §7j.63): exactly 7 `.text` sites EXW with a 7-for-7
EXD twin census — the alloc pointer store, FOUR readers (the splash
stager 0x4243e4, the platform tile build 0x422931, the death-blast
smoke producer 0x423858, and the NEW radar marker-0xd gate 0x41f191),
the memset load, and the write. **The §7j.10 "ORDER marker family
0x425556" gloss RETIRED** — no order-marker writer exists; 0x425556 is
the inner store of FUN_004254e1, the MISSION-LOAD initializer:
memset-0 of the whole 10000-B bank, then the stamp of the ACTIVE
PREFIX (first `state==0` stops the walk) of the 45×0x10 door-rect list
0x4dcae8 (`{+0 state,+2 x0,+4 y0,+6 w,+8 h,+0xA variant}` — §7j.34
grammar re-confirmed from the sar-16 loads), `claim[line[y0+row]+x0+col]
:= 1`, NO bounds checks. The rect source = FUN_0042c4a0's per-zone
HARDCODED store farm (zone table 0x42c484 ×7, mode gate [0x4edb88]==2,
mission tables ×5 for zones 2..6, ==1-only for zones 1 and 7) after the
0x447b7b whole-bank memset — fully deterministic per (zone, mission,
mode); arena-staleness moot (the memset runs every mission). The arena
side re-verified: the bank is the 7th per-mission bump block after the
0x41d955 cursor reset to the [0x46af20] watermark.

**(2) Oracle (76a14c6).** `static_claim_bank_differential.rs` = the
independent all-37-mission oracle: the pinned 368-row rect farm
(`tests/data/claim_rects.rs`, concrete-interpreter transcription with
THREE cases hand-verified instruction-by-instruction — Z1M1, Z3M1,
Z7M1), the initializer transcription over per-mission TOT-header map
dims, and the corpus identity pins (A=1/B..F=7/G=1 missions, 25×75 /
100×100 / 100×25 dims, per-mission claimed-tile census, the exact
ZONEA/M1 59-tile set, total 3049, the 10 all-zero missions) +
four-part sensitivity (rect widening, mid-record deactivation proving
the prefix rule, map_w row arithmetic, compute-only off-map proof of
the unchecked write).

**(3) The actual side is deliberately absent — a documented GAP, not
parity.** bedlam-core hardcodes claim==0 in the three staged gates;
both halves of its justification ("host-staged zeros", "the D82
order-marker writers are the unmodeled seam") are DISPROVEN by §7j.63
(comments corrected in-code; the original REFUSES splash/platform/
death-blast staging on the 59 claimed ZONEA/M1 tiles where Rust
allows). The concrete seam is QUEUED as the next unit (S0-11b): stage
the claim bank in MissionSim from the pinned rect tables, read it in
the three gates, emit the canonical TS row, re-baseline the canonical
chain pins — deliberately not landed here (it moves every E-side chain
and belongs to its own bounded unit; no fabricated parity in the
meantime). Strict S0 independent coverage is now **11/27 rows** (the
D149 ten + `static-claim-bank`); 16 rows remain. (worker eeafac37
claim 1)

## D151 — 2026-08-25: P4/static-parity/S0-11b — the claim-bank staging seam LANDED (row `static-claim-bank` closed BOTH sides; every canonical chain re-baselined)

**(1) The seam.** `static-claim-bank` leaves the D150 gap set: the
rect farm promoted from the oracle's `tests/data/claim_rects.rs` into
`bedlam-core/src/claim_rects.rs` (byte-identical, pinned by a new
oracle test), `MissionSim::stage_claim_bank(zone_set, mission)` =
the §7j.63/C initializer transcription (memset-0 the 0x2710 arena +
stamp the ACTIVE PREFIX of the 45-rect door-rect list via
`line[y] = y*map_w`, `map_w` from `terrain.size()` — the DAT dims the
TOT header agrees with, static_loader_differential's own pin; the
in-arena guard is charter-only, unreachable on shipped data), staged
at EVERY `GameHost::load_mission` — NOT a scenario key, matching the
original's unconditional 0x447b85 call (deterministic, input-free,
no RNG draws, no hashed fields — `state_hash` untouched by design).

**(2) The reader gates.** `stage_splash` + `platform_tile_build` read
`claim_byte(tile)` in the §7j.63 gate order (after the mirror word /
object-grid checks); an UNSTAGED bank reads 0 — the pre-seam behavior
every hand-built sim keeps. The §7j.63/F "three modeled readers"
phrasing is corrected by this unit: the THIRD reader (the FUN_0042382c
death-blast smoke producer) is HOST-SEAMED presentation (§7j.24,
bedlam-game `apply_damage`) — no sim gate exists for it and none is
fabricated. Gate proof: `claim_seam_tests` (destroy.rs) pins the
refusal on rec-0's ZONEA/M1 tile (2,51) with an unstaged-pass control,
an unclaimed-tile control on the SAME staged sim, and the A/M2
all-zero re-stage clearing the refusal.

**(3) Parity closed BOTH sides.** The oracle gains the actual side:
`claim_staging_matches_the_independent_image` runs the engine staging
over a synthetic terrain of every shipped mission's TOT dims and
asserts `claim_bank() ==` the independent transcription (37/37), plus
`promoted_rect_farm_is_byte_identical` (the production table vs the
oracle's own copy). Row `static-claim-bank` is now strict-coverage
CLOSED — the first S0 static row closed both sides rather than
original-side only. Strict S0 independent coverage stays 11/27 rows
(the row was already counted by D150; this closes its Rust half).

**(4) The canonical TS row.** `emit_frame` emits `static-claim-bank`
on the anchor frame (TS) as the RAW arena image — no count prefix, no
field map: the O1 plan dumps the same fixed 10000-B span through the
0x119564 pointer cell and O2 through 0x46af58, so all three channels
ride the differ's byte-passthrough arm and compare clean with ZERO
differ changes (the static-map-wh fixed-extent precedent; the row
fabricates identity through the differ_gate catch-all).

**(5) The re-baseline (deliberate, the D136 precedent).** Every
canonical chain pin moved via the new row: fixture digest
c0268bf499a505c1→1335f953d7da3c82, synthetic 6517d1c0b7169446→
9e5efdc3fff70d88, S0 dac1cfd17bc7ede3→b9b57b68e95f482a, S1
a18cb11ac8e4314e→da833e535f833dcc, S2 d6649ce272ad6d96→
43110d921137da19, S3 f4f5b4351e976ed5→fdd9fae3de7a3ef9, S4
63ab5ac7679f6de7→f35b5e45b26891ea, S5 8a718339e0702fd6→
744950e2d3753d04, S5B b72f57e0b8e7042b→28bfea820bfb05ac, S5C
de5b80a6177aecdd→be8cf733f1d078c2, S6 c27bff339929339d→
80066717ee97b67f, S7 b0db22840310e82a→9b81586f58687994, S8
29fa2f400a10974b→acced68c68c14fa6. LIVE-SESSION COMPARISONS PIN
AGAINST THESE from this commit.

**(6) Corpus reachability ANSWERED (tested, not assumed).** The task
asked whether any staged corpus scenario stages on a claimed tile
(chains moving via gate behavior vs the row alone): NO — every
timeline assertion in canonical_dump_gate (S4's 3×3 splash ring at
(12..14,14..16) + the 250-splash saturation, S7's five builds + 20 k7
debris + the 22 creep tiles across (3..9,53..57), S6/S8's events)
passed UNCHANGED against the now-staged bank, so no staged
splash/platform-tile/death-blast lands on a claimed ZONEA/M1 or
ZONEB/M1 tile in S0..S8 and the chains moved ONLY via the TS row.
The refusal semantics remain proven by the unit gate test (future
scenarios staging on claimed tiles will diverge from O1 exactly as
the original does). (worker ab778f23 claim 1)

## D152 — 2026-08-25: P4/static-parity/S0-12a — dbx-plan `static-min-bank` extent RESOLVED to the pinned PtrCell 0x7530 (the row leaves the plan `_deferred` set on both channels; no strict-coverage delta)

**(1) The resolution.** The bank extent pinned by §7j.62/D149 (the
`.MIN` ArenaAlloc `mov eax,0x7530` — 30000 B, stale tail beyond the
verbatim zone-file prefix proven never read) now drives the capture
plan: dbx-plan's deferred arm
`"static-cgr-volume" | "static-bin-terrain" | "static-min-bank"`
splits, and `static-min-bank` resolves to
`Form::PtrCell { cell, len_expr: "0x7530" }` under a new dedicated arm
guarding `row.extent == "0x7530 (30000 B)"` (the watches.toml extent
moves off "bank-sized" to that pinned form — the registry stays the
fail-loud source: a moved pin dies at plan compile, never silently).
New resolve symbol `min_ptr` (EXW 0x4edd9c / EXD 0x107538) joins
`claim_ptr`/`tot_ptr`/`dat_ptr`/`obj_ptr` in both PtrCell maps; the
emitted anchor row is
`{ "id": "static-min-bank", "addr": "$min_ptr", "len": "0x7530" }`.

**(2) Blast radius.** All 13 committed capture-plan artifacts
regenerated (S0..S8, S0W, S1-o2): the `_deferred` entry
"static-min-bank (bank-sized)" is gone, the anchor row + resolve row
added (O1 anchor rows +1 per TS-bearing scenario, deferred −1: S0/S0W
21/5, S1/S2/S4..S7 38 anchor with 5 or 19 deferred by tier shape,
S3/S8 40/8, o2 37/6). Test count asserts re-pinned (s0 21/5, s1
21+17/5, o2 37/6 symmetry, s2 21+17, s3 +10/8, s4 +10/19) and a new
min_ptr/0x7530 span assert rides the s0 artifact pins.
`static-cgr-volume`/`static-bin-terrain` stay deferred — their sizes
remain unpinned. No engine/differ code touched: this is infra only
(the O1-boot WIP in dbx-plan.rs/capgen is a different owner's
in-flight unit and was deliberately NOT staged).

**(3) Coverage bookkeeping.** NOT an S0 strict-coverage row (same class
as the S0-11b engine hop and D150's oracle hop — infra, not a row):
`static-min-bank` remains CLOSED original-side only (S0-10/D149; Rust
retention deliberately none, presentation-half D17). Strict S0
independent coverage stays 11/27. RE-EXW-SIM §7j.62/F updated: the
"queued separately" note now records the resolution landed. (worker
ee030ded claim 1)

## D153 — 2026-08-25: P4/static-parity/S0-12 — the eight fresh-session T0 campaign/config rows independently covered (score, money, difficulty, zone, mission, mode, linear-mission-m, sfx-master-gate): FIVE closed both sides, THREE named gaps queued as the S0-12b seam unit (strict S0 coverage 19/27)

**(1) RE first (cda35f2, RE-EXW-SIM §7j.64).** Whole-objdump
write-form censuses per cell + instruction-level decode of every
writer block, all [verified] against ghidra-project/exw-text-objdump.txt:
(a) the GameMain boot-init head 0x41c05c..0x41c176 — **mode := 0
(0x41c145) and DIFFICULTY := 1 (0x41c14a, ebx re-set 1 at 0x41c12e) —
the §7j.15/2 "campaign-start write" gloss CORRECTED: it is the boot
default and the fresh value is 1, not 0**; (b) the episode-loop slot
boot 0x41c41c..0x41c44e — zone := 1, mission := 1 (edx=1), score :=
the FUN_0043a5fc fresh-path return 0; (c) the name-entry
fresh-campaign arm 0x43aaa3..0x43aad0 — money := 4000−500·d (imul
0x1f4 off 0xfa0) + mode := 0 again, so **the untouched-toggle fresh
boot carries money 3500**, not the d=0 value 4000 the E default
assumes; (d) **linear-mission-m 0x46ae8c is a DERIVED cell, not a
counter**: m = clamp(5·(zone−2)+mission−1, floor 1, cap 26), recomputed
every episode at 0x41c520..0x41c556 (3 writes, all GameMain; the other
11 xrefs are readers) — fresh (1,1) = 1, correcting the D108
"0-based-progress-counter" E-side assumption; (e) the sfx-master-gate
fresh value = 1: FUN_004252c0 loads HKCU "SOUND" through the D128
bounded loader with bounds [0,1] and DEFAULT ecx=edx=1 (edx preserved
through the FUN_00444ed40 HKCU DATA probe) — the D134/D136
classification is value-exact for the default machine. Full writer
census table per cell in §7j.64/F (10/13/6/8/6/6/3/2 writes).

**(2) Oracle (this unit).**
`engine/bedlam-game/tests/static_campaign_config_differential.rs`
(the first static oracle in bedlam-game — the rows' E half IS the
canonical harness, which bedlam-core cannot see; the file re-exports
parity_harness the canonical_dump_gate way): (a) the original-side
transcription — the fresh-scalar table consts, the linear derivation
function + a hand-computed 13-row spot table + the all-37-mission
corpus census (floor cases exactly 3, max 26, sum 482), and the money
formula per d∈{0,1,2}; (b) the E-side comparison — the S0 canonical
anchor frame's eight T0 rows judged against the transcription: score 0,
mission 1, mode 0, sfx 1 CLOSED both sides (zone via the pinned
D99/D108 +1 normalization); (c) the three gaps pinned LOUD with the
original value named in each assertion message (difficulty 0 vs 1,
money 4000 vs 3500, linear 0 vs 1) — they flip visibly when the seam
lands, never silently; (d) the boot-key seam proof: `boot difficulty=1`
expresses the original fresh-boot state exactly (money 3500,
difficulty row 1) through the existing start_score formula — the gap
is the fresh DEFAULT, not a missing mechanism.

**(3) The seam is QUEUED (S0-12b), not landed here.** Closing the three
gap rows means re-pinning the canonical fresh session (difficulty
default 1 → money 3500) and emitting the linear row through the
derivation instead of `episode().linear()` — a deliberate full-chain
re-baseline of every canonical pin (the D136/D151 machinery), plus a
D108 supersession note (the "never fabricated" seam stance predates the
§7j.64/D decode). That is its own bounded unit with its own decision
entry; no fabricated parity in the meantime (the gap rows stay loud).
watches.toml money/difficulty/linear layout notes corrected to the
§7j.64 facts (layout strings never feed the capture plans — no
regeneration, no plan bytes moved).

**(4) Coverage bookkeeping.** Eight S0 strict-coverage rows counted:
strict S0 independent coverage is now **19/27 rows** (the D152-era 11 +
score, money, difficulty, zone, mission, mode, linear-mission-m,
sfx-master-gate); 8 rows remain (S0-13 rng-state-a/b +
static-dither-noise, S0-14 s0-trigger/frame-counter, S0-15
static-order-table, S0-16 static-player-type, S0-17
static-cursor-clamp). THE D146-NOTE OFF-BY-ONE RE-AUDIT (mandated
when S0-12 lands): the 27-row registry = the tier-S0 `s0-trigger` row
+ the 26 T0/TS watches.toml rows (11 T0 + 15 TS); the covered set is
the 11 TS rows {tot-volume, dat-volume, cgr-volume, bin-terrain,
lnk-map, min-bank, pad-slots, map-wh, claim-bank, yline-zbase,
type-table} + this unit's eight T0 rows = 19; the uncovered eight
list above = 19+8 = 27 ✓ — the predecessor's "18-row remainder"
prose was the miscount (16 is the true remainder at D152), and the
assignment list 8+3+2+1+1+1 = 16 is consistent. (worker 0f91b0d7
claim 1)

## D154 — 2026-08-25: P4/static-parity/S0-12b — the fresh-session campaign/config seam LANDED (difficulty 1 + money 3500 + the linear-mission-m DERIVED cell): the three D153 gaps closed BOTH sides; every canonical chain re-baselined deliberately

**(1) THE SEAM (canonical.rs, the E-side harness).** Three coupled
changes, all pinned to the §7j.64 decode (D153): (a) the fresh-session
difficulty DEFAULT is 1 — the GameMain boot-head write (§7j.64/A,
0x41c14a); `boot difficulty=d` now OVERRIDES a default instead of gating
the seed (an explicit `boot difficulty=0` is expressible again — the old
`difficulty != 0` gate conflate the two). (b) the campaign seed runs on
EVERY run: `set_campaign(0, start_score(d))` + `sim.set_difficulty(d)`
— the name-entry fresh-campaign arm (§7j.64/C, 0x43aaa3..0x43aad0)
re-seeds money 4000−500·d at every campaign start, so the untouched-
toggle fresh boot carries 3500, and the sim's difficulty-scaled damage
rows (§7j.15/1) now run at the ORIGINAL's fresh d=1 rather than the
mis-modeled 0. (c) `linear-mission-m` is emitted through the DERIVED
cell `clamp(5·(zone−2)+mission−1, 1, 26)` from the CURRENT
`mission_slot()` (fresh ZONEA/M1 → 1, ZONEB/M1 → 1) — the §7j.64/D
GameMain recompute at 0x41c520..0x41c556, NEVER the episode progress
counter `episode().linear()`; the destroy staging's TRT hp tier
selector (the [0x46ae8c] reader `250+250·m/27`, §7j.15/4-e) reads the
same derived value. The D108 supersession note is recorded at D108
(its "never-fabricated linear seam" stance predates the decode).

**(2) THE ACCEPTANCE GATE flipped visibly.** The three LOUD gap
assertions in `static_campaign_config_differential.rs` (D153(c)) now
assert EQUALITY against the original-side transcription (difficulty 1
== 0x41c14a, money 3500 == 0x43aaca with d=1, linear 1 == the 0x41c550
floor of 5·(1−2)+1−1) — verified failing-then-passing around the seam,
never silently re-baselined; the boot-key proof now asserts the derived
row on the explicit `boot difficulty=1` run too. ALL EIGHT §7j.64/G
rows are now closed both sides.

**(3) THE DELIBERATE FULL-CHAIN RE-BASELINE (the D136/D151 machinery).**
All 11 canonical corpus chains re-pinned in canonical_dump_gate +
differ_gate (S0 5ab9df44ca3ba0c6, S1 0224dcc5f4631460, S2
04dfa60b7262a474, S3 95375e99ba27990a, S4 a8deea56f9308102, S5
359d9131fb51a86c, S5B 18a27532aeb7858e, S5C 0095d08b9f92d51b, S6
7c4437ee14e9c7ab, S7 f8e83317ca7c5f8a, S8 0d1482d01f57b2b1 — live O1
comparisons pin against these from this commit); the synthetic fixture
digest is UNCHANGED (9e5efdc3fff70d88 — the hand-built TickState never
read the session defaults); the differ_gate coverage counts are
UNCHANGED on every scenario (the T0 rows compare clean on both
channels). The tiebreak lanes' money examples re-based 4000→3500
(perturbation −7 → 3493/3497).

**(4) CONTENT RE-DERIVATIONS the difficulty-1 semantics forced (each
re-derived from the new run, never blind re-pins).** S4: the .TRT
turret hp tier moves 250→259 (m=1: 250+250/27) — the ring-0 destroy
hp −4750→−4741, the survivors hold 259. S5: the money base 4000→3500
("no money draw on this seed"). S5C: the money base + the two award
folds (4150→3650, tail 4210→3710; the award values themselves are
difficulty-independent — the 3744 burst spend is id 0xD = constant
312/pair). S8 re-derived wholesale (the critter family is difficulty-
scaled end-to-end, §7j.42): staging 17 critters now (7 kind-5 + 10
kind-4 — the d=1 spawn roll (RandA&1)+1 banks one extra kind-5), hp
155/207 (base+base·1/27), the 0x68 lane 150/hit ((d+1)·75, §7j.15/1 —
first hit f5, tail 1132 by f39), 10 divers/dormant (respawn table 900
frames). ONE LATENT TEST BUG fixed in passing: the S8 hit-flash walk
read the robot-bank at stride 0x54/+0x2E — a WRONG record offset that
passed on a neighbor field; the 94-B record +62 is the pinned §6a
hit_flash (21 flashed frames f0..31 at d=1). Frame counts unchanged on
every scenario; every double-run byte-identical.

**(5) Docs.** DESIGN §6a rows (score/money, difficulty, the
zone/mission/mode/linear quartet — the LINEAR AMENDMENT supersedes
D108's fresh-slot-0 note), §6 seam grammar (the boot-key override
semantics), the §7 S5 row + the §10-W12 S5 landing note corrected
in place history-preserved; RE-EXW-SIM §7j.64 landing note appended.
watches.toml UNTOUCHED (the D153 layout-note correction was already
plan-neutral). No capture-plan/O1-side change rides — the rows' O1
forms are value-carrying u32 scalars. Verified: workspace release
tests green (bedlam-core + bedlam-game incl. the 13 canonical_dump_gate
and 4 differ_gate corpus lanes + diffharness), fmt + clippy clean,
MANIFEST.sha256 clean before AND after. The unrelated O1-boot WIP
(dbx-plan.rs boot_trap/entry + dbx-capgen.py + dosbox-harness.sh +
RUNTIME.md + capture-plan deltas) preserved untouched. Strict S0
coverage stays 19/27 (this unit closes the Rust/E half of rows the
D153 oracle already counted). (worker 52f0a9f0 claim 1)

## D155 — 2026-08-25: P4/static-parity/S0-13 — the RNG pair + dither-noise rows (`rng-state-a`, `rng-state-b`, `static-dither-noise`) independently covered ORIGINAL-side; E stays the charter T3 statistical stand-in (strict S0 coverage 22/27)

**(1) RE first (b2e522c).** RE-EXW-SIM §7j.65: RandA/RandB
(0x402975/0x4029b6) are the SAME 0x41-byte step on two dword states —
the byte shuffle builds the 40-bit chain dl:ax:bx = S<<8, the three
rcr's rotate it right 1, and **dl' = S>>25 is discarded**, so the
closed form is **S' = ((S<<7)+S+0x361962E9) mod 2^32** — a SHIFT-7,
not a wrap rotate (the 8street "ror33ish" gloss retired, re-anchored
to the EXW instructions per policy); the return value is the NEW HIGH
WORD (u16; consumers mask it). Seed-plant write census COMPLETE: boot
plants BOTH (0x41c0cd B=234567 / 0x41c0d3 A=123456); MissionShell
reseeds **A ONLY** (0x447728, first body instruction) — B is carried
across missions within a session. Dither bank/cursor census COMPLETE:
cursor := 0 per mission (0x4478f7, ecx of the staging block); fill =
exactly 2048 RandB draws (0x447b13..0x447b3a, cursor untouched);
churn = 15 draws/frame advance-then-draw-then-write (0x448147..0x448195,
wrap ≥ 0x800 → 0; the signed < 0 arm is dead defensive code); the blit
(§7i/1 re-verified) only READS — its `RandB()&0x1ff` reseed is a
read-offset pick, never a bank write. Call census: 158 direct RandA /
27 direct RandB sites.

**(2) Oracle (dc6c99d).** `bedlam-game/tests/static_rng_differential.rs`
(the S0-12 canonical-harness home): the instruction-faithful step
transcription (shuffle + rcr chain with the discard) cross-proven
against the closed form over 128 walked states + edges; first-eight
state literals both chains (A 123456 → 0x370C6529 → …, B 234567 →
0x37E71AB0 → …); the A-only reseed vs boot-only B; the fill (first-16
bytes, 526/2048 white census, post-fill state 0xA564DC47); the churn
frame (cursors 1..=15 pair table, post-frame state 0xF52E04EE, the
137-frame refresh identity); the blit reseed offsets + per-blit seed
formula literals. **Sensitivity proven both directions in-memory**:
a one-ulp add-tail mutation fails the cross-proof; a plausible-wrong
shuffle fails five literal pins.

**(3) The E-side classification — Rust determinism is NOT the oracle.**
The three rows close ORIGINAL-side; the E half pins the charter-T3
seam facts and nothing else: the `seed=0x1e240` canonical pin (the
stand-ins carry the original's per-mission A-reseed constant; the
original's boot-only B has no E mirror — the shared mission stream
re-arms per mission, a documented statistical divergence never
bit-compared), 8-byte row presence, stream liveness (the differ's
draw-count signal), and `static-dither-noise` DELIBERATELY ABSENT on
E (presentation-half D17 — the bank never enters the dump/hash; the
row is O1-side coverage; fabricating an emission would be fake parity,
the D149 static-min-bank precedent). No bit comparison with the
transcribed chains exists or is claimed.

**(4) Coverage bookkeeping.** THREE strict S0 rows counted: strict S0
independent coverage is now **22/27 rows** (the D154-era 19 +
rng-state-a + rng-state-b + static-dither-noise); 5 rows remain
(S0-14 s0-trigger/frame-counter, S0-15 static-order-table, S0-16
static-player-type, S0-17 static-cursor-clamp — 4 items — plus the
s0-trigger tier-S0 row those unit notes name). watches.toml layout
notes corrected to the §7j.65 facts (plan-neutral — layout strings
never feed the capture plans; no plan bytes moved, the capture-plan
deltas in the tree are the unrelated O1-boot WIP preserved untouched).
DESIGN-DIFFHARNESS § RNG-states row re-anchored to §7j.65. Verified:
workspace release tests green (bedlam-game incl. the new 7-oracle +
canonical_dump_gate 13 + differ_gate 4 corpus lanes, diffharness,
bedlam-core), fmt + clippy clean, MANIFEST.sha256 clean before AND
after. (worker 77b1c512 claim 1)

## D156 — 2026-08-25: P4/static-parity/S0-14 — the s0-trigger/frame-counter ordering RESOLVED + the DYNAMIC-ONLY placement class (and the D81 "no counter reset" claim corrected): the two rows close by MECHANISM, tracked separately from static closure

**(1) THE ORDERING (RE-EXW-SIM §7j.66, objdump-only).** The EXW
MissionShell loop tail decoded instruction-for-instruction: the P-pause
gate `cmp [0x4edc34]` @0x4485de (MP never pauses), the NORMAL-path
PresentEnd CALL @**0x4486c9** — THE O2/W11 dump point — the pause-path
present @0x44861f (draw PAUSED, spin on the latch, unpause, then `jmp`
PAST the normal call), and the register-form counter increment
@0x4486ce-da ALWAYS after the flip: exactly one present + exactly one
increment per loop pass, both paths. PresentEnd (FUN_00425a03) has
**62 direct call sites** in .text — a BP at the FUNCTION ENTRY the
registry's `exw_addr` names fires on every menu/loading/cinematic
present on the way to the mission, so it is NOT a usable trigger; the
call-site pin resolves the W11 deferral, and the W11 O2 plan regen
moves `trigger.site` 0x00425A03 → 0x004486C9 (the registry row keeps
the function entry as the canon-of-record address — plan-neutral, no
plan bytes moved in this unit). The EXD twin (0x5a6eb CALL +
0x5a6f0-fd register-form inc, RE-EXD-MAP §2) has IDENTICAL order.

**(2) THE D81 CORRECTION.** "NO counter reset exists (14 INC sites
incl. menu screens)" is WRONG on the reset half: an INC-form census
misses mov stores (the trap the W1 EXD census documented for the same
cell family). The eight BOUNDED CINEMATIC screens each RESET the
counter to 0 (`xor reg; mov [0x46ae68],reg`) and reuse it as their
duration timer, exiting at counter == bound {100, 200, 300}; the five
interactive menu screens count cumulatively (inc BEFORE the present
there — the opposite in-loop order). The 14 increment sites stand
(13 INC + the mission-tail register form). Consequence: the
mission-entry value C₀ = a DETERMINISTIC FUNCTION OF THE SCRIPTED
MENU WALK, not a boot-frame total — and the T2 budget consequence is
UNCHANGED (deterministic per script, so double-run compares it
byte-exact; E-vs-O1 never bit-compares it). differ.rs's alignment
note corrected in place; the EXD menu-screen reset family is recorded
as an open cross-check (not blocking — the EXW side is the canon of
record and a live S0W anchor stop pins C₀ empirically).

**(3) THE DYNAMIC-ONLY PLACEMENT (the S0-14 classification ask).**
`s0-trigger` (tier S0, extent 0 — the dump point itself) and
`frame-counter` (T0 — the timing cell) carry NO statically-closeable
state: the trigger row has no comparable bytes at all, and the counter
is the deliberately-never-bit-compared T2 cell. They close under the
new **dynamic-only placement** disposition — coverage = the ordering
pin + the machinery built on it (the capture plans arm the anchor BP;
the dump schema aligns by `frame_no`; the differ classes the row
`T2Reported`; E already emits the mission-relative pre-increment
`sim.frame()−1`, so O1/O2 = E + C₀). Strict S0 accounting becomes:
**22 rows static-closed + 2 dynamic-only dispositioned + 3 static
remaining (S0-15 static-order-table, S0-16 static-player-type, S0-17
static-cursor-clamp) = 27** — the two classes are reported
separately from here on; "static closure" never counts these rows.

**(4) VERIFICATION.** `engine/bedlam-game/tests/
static_frame_counter_differential.rs` (the S0-07..S0-13 static-oracle
convention, two halves): the original-side transcription (the tail as
a state machine — pre-increment dump, one inc per pass, the pause pass
presents but does not fire the BP; the census tables — 13+1 increments,
the eight (reset, cmp, bound, inc) cinematic rows, 62 call sites; the
walk model falsifying the boot-total reading of C₀) + the E-side/
differ classification (the canonical S0 run asserts counter ==
frame_no strictly from 0, 4 B, the anchor = the first mission tick;
the TRANSCRIBED O1 model `counter = C₀ + k` vs E through the differ
lands exactly on `Class::T2Reported` — PassWithNotes, zero
engine-bug/structural/coverage findings, frame-counter the only
reporting row; the identical-script double-run compares byte-exact).
watches.toml s0-trigger/frame-counter layout/note re-anchored to
§7j.66 (plan-neutral — layout strings never feed the capture plans;
the capture-plan deltas in the tree remain the unrelated O1-boot WIP,
preserved untouched; this unit staged only its own paths).
MANIFEST.sha256 clean before AND after (no corpus read). (worker
9c711d0c claim 1)

## D157 — 2026-08-25: P4/static-parity/S0-15 — the `static-order-table` TS row independently covered ORIGINAL-side (geometry pinned both ends 12×0x62=0x498, the whole-writer/reader census closed with TWO new GameMain writer families + the §7d.2(c) lobby gloss corrected to READ direction, and the fresh-session image proven all-zero); E stays a deliberate loud E-gap under the charter no-fabricated-parity class (strict S0 coverage 23/27 static + 2 dynamic-only)

Six decisions recorded (RE-EXW-SIM §7j.67, oracle
`bedlam-game/tests/static_order_table_differential.rs`):

1. **GEOMETRY (the extent is now a NUMBER)**: the order/weapon table
   is 12 rows × 0x62 = 0x498 bytes, EXW 0x4de664..0x4deafb / EXD
   0x91ee4..0x9237b — pinned from BOTH ends: the GameMain boot
   zero-init immediate (`mov ecx,0x498; mov edi,0x4de664` @0x41c3d6
   / EXD 0x2cd0f) and the successor chassis table (EXW 0x4deafc
   ADJACENT with its own 0x150 = 12×0x1C memset; EXD 0x9240c past a
   0x90-B path-string buffer at 0x9237c — a channel layout
   divergence worth recording). 12 rows matches the 12-slot robot
   bank (D129) and the 12 chassis rows: row index = chassis TYPE
   0..11 (MP contexts equate it with the player ordinal).
2. **WRITER CENSUS — §7d.2's THREE-family list was INCOMPLETE**: (a)
   the boot zeroing is an EXPLICIT GameMain memset (the ".bss-zeroed"
   gloss upgraded; the intervening `call 0x43a48d` is a single-`ret`
   no-op stub; the EXD twin call 0x4c7a5 is a 2-cell config copy — a
   minor boot divergence); (b) a NEW episode-reset memset block
   0x41ca06 (called 0x41c5f1) wipes table+chassis on the episode
   transition; (c) a NEW post-mission loadout RECAPTURE block
   0x41ca2e (called 0x41c665/82/89) pools every robot's group-ammo
   word by type, divides by squad size [0x46cbd8] (idiv; quotient→
   word@+2 ammo + item via FUN_0041cb38; the quotient==0 path writes
   the REMAINDER to word@+0 — the `xor edx,eax` quirk); plus the
   known save-restore (FUN_0044745e case 2, the 49-word map with the
   +0xA/+0xC loop-carry displacements 0x4de660/0x4de662 — NOT a
   pre-base header), the shop family, and the §7j.45 shop-exit MP
   mirror (0x442ba7's 0x4de658 displacement is the +0xE carry to the
   group +2 ammo word — eax ≥ 0xE always, never the salvo latch).
3. **READER CENSUS + THE §7d.2(c) CORRECTION**: five families with
   ordinal-identical EXD twins — the spawn copy (§6c.6: robot
   +0x36/+0x38/+0x3A + default order bits = 1<<first group whose
   word0 ≠ 0), the MP respawn re-copy (FUN_0040e230 §7j.24), the
   shop reads (the 0x4403d3 row-text feeder via the −2 dword carry),
   the SAVED.BDL writer (FUN_0044693a, identified by the 0x4597d1
   "SAVED.BDL" string; stages mode/score/money + the full 49-word
   row + the chassis row), and the MP lobby FUN_00448ef1 — which
   READS word@+0/+2 into the outgoing 0x4dd4a0 staging and NEVER
   writes the table (§7d.2(c)'s "5 writer sites" staged the buffer;
   the table's only incoming MP mutation is the shop-exit mirror).
   The adjacent equipment-chassis "extras switch" (EXW 0x40cf96 /
   EXD 0x1dc66, base 0x4deafc/0x9240c: shield charges/variant/
   battery := slot word@+2, slot cleared) is the chassis-side
   sibling consumer, out of the row's window.
4. **THE FRESH-SESSION STATIC IMAGE = ALL-ZERO 0x498**: boot memset →
   no SAVED.BDL restores → the pre-mission shop mutates nothing
   without purchases → the MissionShell-entry image is 1176 zero
   bytes, deterministic; and it is a FIXED POINT of the writer cycle
   (zero ammo pools → idiv → zero writes). A nonzero table WOULD
   matter (order bits arm, group words copy into the robot record) —
   the falsification direction the oracle pins both ways.
5. **E-SIDE CLASSIFICATION = the charter no-fabricated-parity class**
   (the D149/D155 precedent, NOT the volume-parsing clause): the
   loadout is host-session state whose producers are outside the
   engine; E has no loadout model and the canonical robot record is
   the 94-B modeled subset with neither the +0x36/+0x38 group words
   nor the +0x6E order-bits word. The oracle asserts the row stays
   ABSENT on every canonical frame (the dither-row precedent) and
   guards the 94-B record length on the T1-carrying S1 run. The row
   closes ORIGINAL-side only; strict S0 coverage 23/27 static + 2/27
   dynamic-only, 2 static rows remain (S0-16 static-player-type,
   S0-17 static-cursor-clamp).
6. **QUEUED FOLLOW-UP**: the dbx-plan extent hop — the deferred
   `static-order-table` arm resolving to the now-pinned fixed 0x498
   span (Form::Fixed at a direct .bss address, the S0-12a PtrCell
   precedent) + the 13 capture-plan regenerations. watches.toml
   layout note amended PLAN-NEUTRALLY (extent string unchanged —
   zero plan bytes moved; the capture-plan deltas in the tree remain
   the unrelated O1-boot WIP, preserved untouched; this unit staged
   only its own paths).

## D158 — 2026-08-25: P4/static-parity/S0-15a — the dbx-plan `static-order-table` extent hop LANDED (infra, the S0-12a precedent): the deferred arm resolves to the D157-pinned fixed 0x498 span as Form::Fixed at the DIRECT .bss addresses (EXW 0x4de664 / EXD 0x91ee4 — never pointer-indirect, so the min-bank PtrCell form deliberately does NOT apply); all 13 capture-plan artifacts regenerated; NOT a strict-coverage row — strict S0 stays 23/27 static + 2/27 dynamic-only

The D157 §6 follow-up landed exactly as queued (worker cb67f182,
claim 2):

1. **THE FORM (S0-12a mechanics, Fixed not PtrCell)**: the deferred
   arm dies loudly if the row ever gains the `indirect` flag (the
   table IS the .bss image — a pointer cell would be a fabricated
   indirection), requires a parsable channel address, and guards the
   exact extent string `"0x498 (12x0x62 rows)"` against drift before
   planning `Form::Fixed { len: 0x498 }`. The emitted row on O1 is
   `{ "id": "static-order-table", "addr": "CS:00091EE4", "len": 1176 }`
   (registry anchor order, between claim-bank and player-type); on
   O2 `{ "addr": "0x004DE664", "len": 1176 }` — the EXW flat form.
   No resolve symbol (a fixed span needs none).
2. **watches.toml**: extent `"0x62-stride rows"` → `"0x498 (12x0x62
   rows)"` (the one plan-feeding byte that moved in this unit); the
   layout tail note now records the resolution (D158).
3. **COUNT RE-PINS (the anchor set grows by one everywhere)**:
   S0/S0W 21→22 anchor / 5→4 deferred; S1 21+17→22+17 / 5→4;
   S1-o2 37→38 / 6→5 (the O2 mirror of the same row — cursor-clamp
   still the sole EXD-only refusal); S2 21+17→22+17; S3 +10→+11 TS
   anchor, 8→7 deferred; S4 +10→+11, 19→18. The new span asserts pin
   both channel forms (CS:00091EE4 / 0x004DE664, len 1176); the
   `extent_forms_parse` pin gains `parse_extent("0x498 (12x0x62
   rows)") == Some(0x498)` (the retired pre-D158 string stays a None
   parser pin — '-' is not a delimiter).
4. **ALL 13 ARTIFACTS REGENERATED** — S0, S0W, S1, S1-o2, S2..S8;
   each delta is EXACTLY two lines (the anchor row added, the
   `_deferred` entry dropped) against its pre-hop blob.
5. **NOT a strict-coverage row** (infra hop, the S0-12a class):
   `static-order-table` remains closed ORIGINAL-side only (D157, the
   charter no-fabricated-parity class — E emits nothing, asserted by
   the D157 oracle); strict S0 coverage stays 23/27 static + 2/27
   dynamic-only. The remaining deferred TS arms after this hop:
   static-cgr-volume, static-bin-terrain (bank-sized, unpinned),
   static-lnk-map (map-sized, unpinned), static-yline-zbase
   (table-sized, unpinned) + the T2/T3 unaliased sets.
6. **VERIFICATION, STAGED CONTENT FIRST**: the exact committed
   combination (HEAD + only this unit's hunks — the in-flight
   O1-boot WIP in dbx-plan.rs/capture-plans/RUNTIME.md/capgen
   deliberately EXCLUDED, preserved untouched for its owner) was
   proven in a scratch crate: full diffharness suite green (25 lib +
   35 dbx-plan + 21 differ + 15 registry_anchors + 2 dump_schema +
   3 stitch_replay = 101 tests, 0 failed), fmt clean, every
   artifact's scratch delta vs HEAD verified 2-line extent-only.
   The LIVE worktree (WIP + this unit layered) is ALSO fully green
   (same 101) with the regenerated artifacts carrying both change
   sets; clippy clean on the crate. The index was loaded with the
   scratch blobs (hash-object + update-index) so the commit contains
   exactly the verified staged content while the working tree keeps
   the predecessor's WIP intact. MANIFEST clean before and after; no
   corpus read, no Ghidra run.

## D159 — 2026-08-25: P4/static-parity/S0-16 — the `static-player-type` row independently covered and closed BOTH SIDES through the canonical anchor seam (the D154 class, NOT the D157 no-fabricated-parity class): the fresh-SP value pinned 0 on BOTH channels, the whole writer census closed (EXW 6 = boot + 5 MP-lobby, 4 of them the −1 error exit; EXD 2 = boot twin + the MP serial-sync writer), the save family proven READ-only; all canonical chains re-baselined deliberately (strict S0 coverage 24/27 static + 2 dynamic-only)

Worker 89591972, claim 1. RE-EXW-SIM §7j.68 carries the full
decode; the unit excludes every MP value/writer semantics from its
closure claim (the task charter — a later named task owns the
lobby/sync families).

1. **THE RE (§7j.68, objdump-only; raw-dword scan the D133
   technique)**: EXW census 113 literal sites, 6 writers — the
   boot writer `xor eax,eax → [0x4edb90]` @0x41c344/0x41c34c
   (unconditional GameMain boot, inside the CINEMATICS
   [0x46cca4]:=1 sandwich around the sound init FUN_0043a144 —
   the §7d.3 "bootattract" gloss superseded by D134's sound-init
   identification) + 5 sites in the MP lobby FUN_00448ef1: FOUR −1
   error exits (0x44918a/0x4493e0/0x4497f1/0x4498e6 — the
   "no local player" sentinel) and ONE success writer 0x449a5c
   (TYPE := the local player's ORDINAL in the 0x4ee450 walk vs
   [0x46cbe0]). EXD census 117 sites, 2 writers — the boot twin
   @0x2cc7b/0x2cc84 (same CINEMATICS sandwich, cell pair
   [0x1194d8]≡[0x46cca4], around FUN_0004be7d; the successor call
   preserves the EXW tail's instruction ordinal) + the MP
   serial-sync writer @0x5b026..0x5b030 (`call 0x62100; and
   eax,0xffff` — the "Quit from synchronising" / "Found %i
   players, but could only sync %i !" path; the DOS port has NO
   lobby family). The raw file image holds EXACTLY 113
   occurrences of 0x004edb90 (all .text disp32 operands, DGROUP/
   .idata/.reloc zero — no aliasing pointer cell) and ZERO of
   0x004edb92; EXD zero of 0x1075c2 — the cell is DWORD-written,
   WORD-consumed (the two spawn kind stamps are the only WORD
   reads), so extent "2" captures the consumed word and the high
   words are dead. The save family is READ-ONLY both channels:
   the type is NEVER SAVED.BDL state (the save derives the name
   from it lea eax,[eax+eax·8] into 0x4e43e0; the restore copies
   the row INTO type·0x62). Reader families: the spawn kind stamp
   + first-robot cell (instruction-exact EXW 0x40cdec/0x40cdfb ⟷
   EXD 0x1db19/0x1db28), the "my robot" gate `dword@robot+0x28
   sar 16 == [type]` (the dominant mission family incl. the D132
   chase gate 0x423eb0/0x34e44), the §7j.67 row-index imul 0x62/
   0x1C families, the per-TYPE sibling 0x46ae94+type·4 (D133
   boundary exclusion), and the panel/walk reads.
2. **THE CLASSIFICATION — both sides, genuinely modeled (the
   D154 seam precedent)**: `MissionSim::player_type: u16` is
   constructed 0 with NO setter (the census's writer set is
   boot+MP, both outside the mission sim — the constant IS the
   faithful SP model), and three REAL gates consume it (alarm
   trip §7g.1, critter bounty §7j.24/2, the case-4 pickup host
   seam). Robots construct `kind: 0` (the SP kind-stamp model)
   and `kind` rides `state_hash` while `player_type` stays
   unhashed — exactly the original's surface. The E half of the
   row = the canonical ANCHOR emission `static-player-type`
   (u16 LE, 00 00, anchor frame only) — byte passthrough on
   every channel, NO differ change (the D136 static-map-wh
   precedent). Fresh-SP image = 0 deterministic on both channels
   (boot is the only SP-path writer; save never restores the
   cell; MP gated off in SP).
3. **THE ORACLE** (`engine/bedlam-game/tests/
   static_player_type_differential.rs`): the original-side
   transcription (boot writer semantics both channels incl. the
   CINEMATICS sandwich + the MP writer census pins + the save
   READ-only proof), the spawn-consumer transcription (kind
   stamp + first-robot cell + the my-robot gate arithmetic), the
   E-side pins (sim constant 0, no setter exists, robots kind 0,
   the three consumer gates' behavior at kind==type, the anchor
   row bytes == 00 00, canonical chain moved-by-the-row), and
   the sensitivity direction (a nonzero type changes the gates'
   outcomes — the falsification the row exists to catch).
4. **RE-BASELINE**: every canonical chain digest re-pinned (the
   anchor row lands on every scenario that wants TS — all 12
   plans carry the row as an anchor watch); differ_gate corpus
   lanes re-pinned; coverage counts unchanged.
5. **VERIFIED**: workspace release tests green (bedlam-game +
   bedlam-core + diffharness incl. canonical_dump_gate 13 +
   differ_gate 4 corpus lanes), fmt + clippy clean on touched
   files, MANIFEST clean before AND after (no corpus write; the
   EXD linear image rebuilt to /tmp/opencode scratch only). The
   unrelated O1-boot WIP preserved untouched (unit staged only
   its own paths).

## D160 — 2026-08-26: P4/static-parity/S0-17 — the `static-cursor-clamp` row DECODED AND RECLASSIFIED as hardware/input-profile-only (the third S0 disposition class, beside static-closed and dynamic-only): the "EXD-only 240x320 clamp maxima" premise DISPROVEN on all three counts, the real constants pinned BOTH channels, and the DOS/classic-input adapter re-pinned to them

Worker 6027a7bf, claim 1. RE-EXD-MAP §5h carries the full decode
(objdump whole-census, no Ghidra run). THREE decisions recorded:

(1) **THE GLOSS DISPROVEN.** The cells 0x1074ac/0x1074b0 are the LIVE
hardware-cursor POSITION pair — Y@0x1074ac, X@0x1074b0 — the EXW
`g_cursor_x/y @0x4eddc4/0x4eddc8` twins (identity locked two ways:
the INT-33h mickey axes + the in-mission hotspot twins carrying
IDENTICAL literals 0x1ee/0x271/0xc3/0x146 @EXD 0x2f6d9 ⟷ EXW
0x41ec9d). The 0xf0/0x140 dwords are the GameInit boot-CENTER
literals (X=320, Y=240 of 640×480; instruction-exact twins EXD
0x2c79a..0x2c7b2 ⟷ EXW 0x41c083..0x41c09b, in the RNG-seed boot
sandwich). The space is 640×480 on BOTH channels (EXD VESA 0x101 mode
set @0x1259a + the ×640 sprite-stride @0x1297e), NOT "320×240". The
REAL clamp box **[9,631]×[9,463]** lives in the EXD poll handler
0x12615..0x12659 (INT 33h AX=0003 buttons → g_mouse_flags twin
[0x1074a4], AX=000B mickeys, integrate-then-clamp) and is the EXW
ScrollUpdate@0x425b2e..0x425b84 box VERBATIM (+9 margin; the 9 = the
24×24 cursor-sprite hotspot offset −9 @0x12970..0x12992). Writer
census CLOSED: 4 stores / 2 functions (boot + poll), no memset span
covers the cells; 119 sites bucketed (82 cmp-imm hit-tests, ~33
reads, the 100Hz ISR family with iret@0x1287c: poll gates + the
hardware-cursor redraw-on-move + the drag anchors 0x1074d8/d4 and
0x107498/9c ⟷ EXW 0x4eddf8/fc and 0x4ede00/04 + the sidebar
cursor-sprite gate X≥480; 9 callers of the poll).

(2) **THE CLASSIFICATION.** The pair is host hardware-cursor state —
written by the boot plant + a hardware poll, redrawn from an
interrupt, driven by raw mickeys — the D17 non-hashed bucket on BOTH
channels, NEVER read by the deterministic sim. The row is therefore
classified **hardware/input-profile-only**, never counted as static
parity (S0 final ledger: 24/27 static + 2/27 dynamic-only + 1/27
hardware/input-profile-only). The registry row keeps id/exd_addr/
extent/tier and stays EXD-address-only BY DOCUMENTED CHOICE (the EXW
twin cells exist but stay unnamed so the row remains the D139/D143
EXD-only anti-ghost vehicle — the stitch-refusal gates stay green
unchanged); layout/note/anchor re-pinned to the truth (plan-neutral:
plans embed only id/addr/len).

(3) **THE ADAPTER RE-PIN (the Rust half).** The DOS/classic-input
adapter — `bedlam_core::input::InputFrame` (mouse_dx/dy DELTAS =
exactly the EXD INT-33h mickey model) + `bedlam_core::frame::
FrameState` — previously clamped [0,639]×[0,479] from (0,0) with
"exact EXW addresses TBD pending P2e input RE". Re-pinned to the
twin-verified constants: clamp **9..=631 / 9..=463** and the
FrameState boot plants the CENTER (320,240) (the GameInit twin).
FrameState stays non-hashed (the D17 determinism guarantee re-pinned,
not weakened: cursor trajectories still never move sim hashes).
Oracle `static_cursor_clamp_differential.rs`: the poll-handler
transcription (EXD branch order incl. the ≥631/≥463 saturation
edges), the twin literal pins (9/0x277/9/0x1cf and the boot-center
literals both channels), the VESA/hotspot space pins, adapter
equality over scripted mickey walks from the boot center, and
sensitivity BOTH directions (the old [0,639]-from-(0,0) adapter FAILS
the comparison; a swapped or margin-less box fails; the original-side
branch-order mutations fail). NO canonical chain moves (the cursor is
not canonical state — no differ/O1/O2/O3 surface touched); no corpus
read; no Ghidra run.

**(4) ADDENDUM 2026-08-26 — the GAME-LAYER re-pin (the named P2e
package, worker 80491508 claim 1).** The scene cursor models landed
on the same constants: `TitleMenu` (menu.rs) and `MissionScene`
(mission.rs) now import `bedlam_core::frame::{CURSOR_*}` — boots at
the GameInit center (320,240), clamps into [9,631]×[9,463] every
integrate (the two `CLAMP DIVERGENCE` annotations replaced by the
faithful pin). AUDITS VERIFIED: (a) the menu hit-strip
(0xdc,0x1a4)×(top,0x1d6) sits inside the box — x (220,420) well
inside; top ≥ 302 > 9; the 0x1d6 (470) bottom is EXCLUSIVE while
the cursor max y is 463, and at y=463 the row index (463−302)/0x18
= 6 = the LAST row of a count-7 strip, so NO strip row is lost to
the clamp (documented at the STRIP_* consts); (b) the mission
click-seam targets are all inside the box (sidebar gate 480 ≤ 631;
every scripted test target ≤ (630,453)); the gate test's
absolute-from-(0,0) aim deltas were converted to target-driven
deltas via a shared `aim` helper (5 sites). NO canonical chain
moved: canonical runs feed `InputFrame::default()` (no mickeys, no
clicks), and the cursor is D17-bucket presentation state — the
canonical_dump_gate 13/13 + differ_gate 4/4 corpus lanes re-asserted
the pinned chains unchanged; workspace release tests 733/0.

## D161 — 2026-08-26: P4.2/S0-registry-tail `ts-extent-arms` — the LAST FOUR deferred dbx-plan extents resolved (cgr-volume, bin-terrain, lnk-map, yline-zbase); S0/S0W reach ZERO deferred rows

THREE decisions recorded. (1) **THE EXTENTS** (RE-EXW-SIM §7j.69,
both channels instruction-anchored, corpus cross-checked,
MANIFEST clean before and after): `static-cgr-volume` →
PtrCell len `0x20562` — the UNIFORM 132354-B file image (u16 count
128 + 512-B self-relative directory + 128×1030-B records; all 44
shipped .CGR exactly that), deliberately NOT the 0x20788 arena
(D152's MIN arena choice was forced by varying files; the CGR corpus
is uniform, so the file image is the tighter pin AND keeps the
passthrough compare free of the 646-B stale tail). `static-bin-terrain`
→ PtrCell len `0x258960` — the BOOT-PASS arena (EXW alloc 0x41d666 /
EXD 0x2e098 — NOT in FUN_0041d954; the successor instruction loads
GENERAL.BIN into the sibling bank), the MIN situation exactly (files
vary 2041594..2443943, the count word lives inside the bank; stale
tail never read — directory-relative readers; GAMEGFX/SHOPLITE.BIN at
3081801 is a DIFFERENT bank family, not a counterexample).
`static-lnk-map` → Form::Fixed 0x4000 at the DIRECT .bss targets
(EXW 0x45cdda / EXD 0x10336c) — the u16[8192] image, all 44 .LNK +
7 .LNG exactly 16384 B; the "(0x8000)" gloss in §7c.2/MISSIONVIEW §1
had NO loader immediate anywhere and RETIRES. `static-yline-zbase` →
TWO SPANS: the registry id keeps the y-line table (CountExpr
`4*$map_h` — the D147 h-dwords pin re-verified on the EXD loops) and
the z-base plane table rides the DERIVED id
`static-yline-zbase#zbase` (Fixed 32, the 8-dword table) — the two
tables are non-contiguous with a channel-DIFFERENT gap (EXW 0x1cc /
EXD ~0x7c000), so no single span can mirror the layout across
channels; capgen's keep-first dedupe never drops the companion
(distinct ids) and the differ's static-* passthrough compares each
span byte-exact. New resolve symbols cgr_ptr/bin_ptr in both PtrCell
maps. (2) **BLAST RADIUS**: all 13 capture-plan artifacts regenerated
on BOTH channels — the four rows leave `_deferred` everywhere (S0/
S0W/S1/S2/S5/S5B/S5C hit 0 deferred; S3 3 = the unaliased T2 trio;
S4/S6/S7 14 = the T3 set; S8 17; S1-o2 1 = the EXD-only
static-cursor-clamp); anchor counts +5 per TS-bearing scenario (27
on S0/S0W, 44 on the T1 scenarios, 46 S3/S8, 43 on o2). Test count
asserts re-pinned (s0 27/0, s1 27+17/0, o2 43/1, s2 27+17, s3
frame+16/3, s4 11+17+16/14) + the row_ids lookups strip the `#`
companion suffix + new span asserts on both channels (cgr_ptr 0x4edd60/
0x107540, bin_ptr 0x4ede1c/0x107434, lnk 0x45cdda/0x10336c len
16384, yline 4*$map_h + #zbase 32). watches.toml extent strings moved
off the symbolic placeholders; RE-EXD-MAP §5 rows re-pinned with the
extent provenance. (3) **VERIFICATION, BOTH WAYS** (the S0-15a
precedent for the shared-worktree overlap): the EXACT COMMITTED
CONTENT was verified in a scratch crate = HEAD + only this unit's
dbx-plan hunks (the interrupted O1-boot WIP's boot_trap deltas
extracted OUT of the patch by hunk census: 16 of 20 hunks are this
unit's, the 4 WIP hunks touch only boot_trap/BPLM/boot_note text) —
all 101 diffharness tests green there, plans regenerated from that
scratch binary (the S0-vs-HEAD delta is exactly the 2 resolve rows +
5 anchor rows + 4 dropped deferred entries); AND the live worktree
(both change sets stacked) is green with fmt + clippy clean. The WIP
owner's boot_trap deltas remain INTACT in the worktree files — the
commit stages the scratch blobs via hash-object/update-index, so
`git diff HEAD` after this commit shows ONLY their deltas again. Zero
canonical-chain movement (plan-only infra, the D152 class). Strict S0
coverage unchanged (24/27 static + 2/27 dynamic-only + 1/27
hardware/input-profile-only — the registry was already fully
dispositioned; this closes the plan-side tails). (worker d093c3ef
claim 1)

## D162 — 2026-08-26: P4.2/W-survey `t2t3-alias-census` — the LAST EXD-alias class closed: all 17 `unmapped` T2/T3 watch rows carry dual-anchored EXD aliases (RE-EXD-MAP §5i); registry `verified`, plans emit them; the differ subset-form extraction arms are the named follow-up

(1) **METHOD (objdump-only, no Ghidra run):** the MissionShell
boot-clear cluster is ordinal-identical both channels (19 memset
pairs, EXW 0x4479c5.. ⟷ EXD 0x59994.., memset twin
FUN_00402965⟷FUN_00012206) and every one of the 17 rows is then
confirmed by an independent accessor twin (loader/tick/walker/
allocator/resolver, instruction-for-instruction) — the W1 dual-anchor
rule holds for every row. Headline pins: mortar-trail 0x91574,
critter 0x10e81c + count 0x1194dc, poi 0x971d4 + count 0x119580,
debris-stager 0x93064, effect-rows 0x9d534, rising-debris 0xa1684,
blast 0x8c284, splash 0x107774, arrival-rides 0x10da48, door-rects
0x92c64, trigger-timers 0x91d94, pod-ring 0x8d314 (third independent
pin — the §5f lobby census), exit-ring 0x108138, dropship 0x1081c4,
objective-slots 0x8c182 + phase 0x1194cc, escape-counters
0x107674/0x107680, tile-claims *(0x119564) (re-confirming the TS row
independently). **Divergence-seed #5 warning recorded:** EXW's
pod→dropship→exit→trail contiguity does NOT survive in EXD —
EXW-relative adjacency is never evidence. (2) **SCOPE SPLIT:** the
registry fills + dbx-plan emission forms land NOW (fixed spans via
the generic arm; critter/poi as CountExpr rows over the new count
cells with resolve symbols critter_count/poi_count; tile-claims as
the second PtrCell row; escape-counters as a per-channel span — the
EXD pair sits 0xC apart while EXW is adjacent); the differ needs
NOTHING for E-gap rows (O1-only rows surface as coverage findings by
design), but the four subset-form rows E emits (critter-bank
74-of-0x7E, effect-rows 28-of-0x20, debris-stager, splash-records)
need O1 extraction arms + inv_frame fabrication — the D87 field-map
class, queued as the next unit (their differ.rs "E-only" comments
ride that unit too). (3) The DESIGN §4 T2/T3 tables carry the EXD
column; the S6/S8 "no EXD alias" claims amended in place (the rows
are alias-complete; E-only remains a DIFFER-coverage state only).
(worker 03cc1ea3 claim 1)

## D163 — 2026-08-26: P4.2/W-followup `t2t3-differ-arms` — the four D162 subset-form O1 extraction arms LANDED: debris-stager/splash-records/critter-bank/effect-rows go cross-channel; dropship-frame is the one remaining E-only T3 row

(1) **THE ARMS** (RE notes first, 965796b — DESIGN §6a's new
subset-arm table): the four rows whose E canonical record is a
SUBSET of the guest record normalize on the O1 side by walking the
GUEST full span and projecting E's modeled fields at the guest
offsets — the D87 field-map class, zero field gaps by construction
(every canonical leaf sources from the guest). critter-bank (T2):
the `$critter_count*0x7E` span → 23 leaves (kind w@+0, species
w@+2, attacker i16@+4, hp i16@+6, mode w@+0xC, anim w@+0xE, heading
d@+0x10, impact d@+0x1C/+0x20, presence w@+0x24, target
d@+0x2A/+0x2E/+0x32, xyz d@+0x36/+0x3A/+0x3E, home d@+0x42/+0x46,
death_ctr d@+0x52, countdown w@+0x56 zero-extended, facing w@+0x72,
target_robot i16@+0x7A, fuse w@+0x7C), cap 350 = 0xAC44/0x7E.
effect-rows (T3): the 80×0x20 LRU bank → 8 leaves (age w@+0, x
d@+2, y d@+6, z d@+0xA, cos d@+0xE, sin d@+0x12, ttl d@+0x16, id
w@+0x1A). debris-stager (T3): the 128×0x30 ring → active u8@+0,
kind d@+0x1C, delay d@+0x24, **seq d@+0x18 — the DUAL field the
engine splits** (§7j.44: E keeps the LRU-eviction role as its
global staging counter `debris_seq`, the walk-cursor role as
`anim`): the projection carries the raw guest +0x18; its value
diverges from E's counter BY CONSTRUCTION and stays silent only
because the row is T3 (never bit-compared) — if the row is ever
re-tiered, this offset pair is the known encoding difference
(recorded in the arm's doc comment + the §6a table). splash-records
(T3): the 250×0xA bank → identity, count synthesized from the fixed
bank. critter-bank/effect-rows `count` join the STRUCTURAL count
words (a count mismatch is a staging divergence, never a T2/T3
budget item). The O2 alias list takes all four — the maps were
pinned EXW-side (§7j.11/§7j.24-5/§7j.17/§7j.10) and §5i closed the
EXD twins, so the guest-span projections are shared. **dropship-frame
stays the one E-only T3 row** (D162 pinned 0x1081c4 and the plan
emits it, but the O1 normalizer arm — the full-record identity
form, its canonical record IS the guest 0x1C craft record
field-for-field — is its own named follow-up; the differ.rs/
canonical.rs comments now state exactly that). (2) **THE TEST
FLIP** (51ba11a): the differ_gate `inv_frame` E-only `continue`
arms for the quartet drop, replaced by the guest-span fabrications
(each canonical field placed back at its guest offset; presence/
countdown narrow word↔i32 at the word; the debris table INDEX
zero-extends into the +0x2C pointer word — E models the index, not
the pointer). expect_coverage re-derived: S4 3→1, S7 3→1, S8 3→1,
S6 stays 1+1 (dropship); the S4/S7 destroy set + the S8 critter/
effect/projectile set now assert COMPARE-CLEAN (no findings at
all). NO canonical chain moved (the emitter edits are comment-only
— the E-side bytes are untouched, verified by the 13/13
canonical_dump_gate + the pinned-chain asserts inside differ_gate).
(3) **VERIFIED**: diffharness 101/101, differ_gate 4/4 (110 s),
canonical_dump_gate 13/13, bedlam-game release 232/0, fmt + clippy
clean on both crates, MANIFEST clean before AND after (no corpus
read, no Ghidra run). The unrelated O1-boot WIP (dbx-plan.rs,
capgen, harness, RUNTIME/RE-EXW-SIM docs, capture-plan deltas)
preserved untouched — this unit staged only its own five paths.
Queued: `dropship-identity-arm` as the new item 1. (worker
bb808d77 claim 1)

## D164 — 2026-08-26: P4.2/W-followup `dropship-identity-arm` — the dropship-frame full-record IDENTITY O1 arm LANDED: the LAST E-only T3 row goes cross-channel; zero E-only T3 rows remain
**Status:** LANDED (commit 6d4ea58, worker f2e721b3 claim 1).
**Provenance:** §7j.40/6 (the craft-record decode, D112); D162
§5i (the EXD twin 0x1081c4); DESIGN-DIFFHARNESS §6a (the D163
subset-arm table named the identity form as its own follow-up).
[verified] **(1) THE FORM:** unlike the D162 quartet, the
dropship row is NOT subset-form — E's canonical 0x1C craft record
IS the guest record FIELD-FOR-FIELD (active u32@+0, phase@+4,
x@+8, y@+0xC, alt@+0x10, group@+0x14, dwell@+0x18 — EXW 0x4e6610 /
EXD 0x1081c4, plan len 28), so no projection is needed: the O1 arm
(`normalize_o1_row`) delegates to the E-side field walk verbatim
(`normalize_engine_row` — the tile-word-grid/platform-strength
identity precedent). The O2 alias list takes the row (same
layout, EXW-pinned), so O3 (which normalizes through O2) takes it
too — a real O2/O3 capture compares it cross-channel as-is. **(2)
THE TEST FLIP** (differ_gate): the `inv_frame` `dropship-frame`
E-only `continue` arm DROPS — the row fabricates identity through
the scalar catch-all (`w.bytes.clone()`), exactly the D132
blink-cursor precedent; expect_coverage S6 1+1 → 1; the S6 lane
gains a COMPARE-CLEAN assert on the row (no findings at all: row-
or field-level). The stale E-only comments in differ.rs +
canonical.rs close (the emitter edit is comment-only — ZERO
canonical chain movement, re-asserted 13/13 canonical_dump_gate +
the pinned chains inside differ_gate). **(3) CONSEQUENCE:**
move-target-words is now the ONLY E-only row (row- or field-level)
on every scenario S0..S8 — the T3 tier is alias-complete and
cross-channel end-to-end. **(4) VERIFIED:** diffharness suite all
green, differ_gate 4/4 (110 s), canonical_dump_gate 13/13,
bedlam-game release 232/0, fmt clean + clippy clean on both
crates (bedlam-game + diffharness), MANIFEST clean (no corpus
write, no Ghidra run). The unrelated O1-boot WIP (dbx-plan.rs,
capgen, harness, RUNTIME/RE-EXW-SIM docs, capture-plan deltas)
preserved untouched — this unit staged only its own four paths.
(worker f2e721b3 claim 1)

## D165 — 2026-08-26: P4/infra `o1-responsive-boot-land` — the RESPONSIVE code-BP O1 boot path ADOPTED + LANDED: non-walk capture plans drop the heavy BPLM frame-counter trap; walk plans (S0W) retain BPLM/RUNWATCH; input-flush + strict logfile-bracket machinery makes every stop fail-closed provable
**Status:** LANDED (worker 29669e49 claim 1 — ADOPTION of an
interrupted predecessor WIP deliberately preserved through D162/
D163/D164 by their workers; no .state/PAUSE ever covered it).
**Provenance:** D81 (the original BPLM boot-trap flow), D84 (the
walk driver that depends on BPLM stops), RUNTIME.md "S0 live
channel mechanics" (all responsive-path facts [source-pinned] on
the D80 build tree at e522642 — core_normal.cpp:160-180,
debug.cpp:460-479,585-586,752-810,2324-2379,2600-2668,2861-2869,
4913-4924,5078-5086,6218-6252; debug_gui.cpp:584-587); RE-EXD-MAP
§1a (EXD entry 0x5fbb0). [verified by adoption run]
**(1) WHY:** an armed BPLM makes the heavy build's normal core call
`DEBUG_HeavyIsBreakpoint` (the breakpoint-list walk +
`mem_readb_checked`) on EVERY instruction — a per-instruction tax
paid across an entire multi-minute mission capture, for a trap
that only ever needs to fire ONCE (the boot bridge). The mission
anchor is a code BP anyway, and code BPs are checked by the
normal core's cheap CS:EIP compare branch — so after the anchor is
armed the capture can run plain RUN. BPLM-as-boot-trap existed
only because BP locations resolve EAGERLY at arm time (a game BP
armed at the real-mode pre-boot halt mis-resolves).
**(2) THE RESPONSIVE BRIDGE** (boot_trap:"entry" plans): `BPINT 21
4B` stops at DOS EXEC (still real mode); `BPDEL *` + a FRESH EMPTY
BPLIST proves the trap dropped; `BP 5FBB:0000` armed while still
in real mode resolves eagerly to linear 0x0005FBB0 = the verified
EXD entry (5FBB<<4); a fresh BPLIST proves it is the SOLE
breakpoint; plain RUN; at the stop `EV CS EIP CR0` must show
EIP==0x0005FBB0 ∧ CR0.PE=1 and `SELINFO CS` must show base==0 ∧
limit≥0x12583e — this RETIRES the old INT3-at-entry checklist
item (the pmode flat-entry proof now needs NO guest-code
modification); BPDEL * again; the plan's arm_commands (BPDEL * +
`BP CS:0005A6EB`) run through the strict path with the selector
pin; every mission-frame wait is plain RUN. S0W-class WALK plans
keep BPLM + RUNWATCH + the flat-guard retry loop (stop-indexed
menu walking NEEDS the memory-driven stops); legacy v1 probe
plans keep their shape.
**(3) THE STRICT MACHINERY** (the part that makes it trustworthy):
(a) INPUT QUEUED BEHIND RUN/RUNWATCH IS DISCARDED AT RE-ENTRY —
`DEBUG_Enable` calls `DEBUG_FlushInput` (ncurses drains getch())
before drawing the stopped debugger, so a probe sent behind a
running machine is NOT a stop barrier; readiness = a fresh
`NOTICE:` marker sent AFTER a stop candidate and actually landing
in the logfile, with PTY output marks so split/combined re-entry
redraws are never lost and one global deadline bounding settle +
resume + every probe retry (expiry emits no later write). (b)
Strict queries (`EV`, `SELINFO`, `BPLIST`, `BP`, `BPINT`, `BPDEL`)
are bracketed by unique `ADDLOG` begin/end nonces so a zero-overlap
logfile replacement cannot make stale output look fresh; BPLIST
additionally parses fail-closed (heading + 73-dash separator +
contiguous indices + fully parseable rows). (c) BPLM stops (walk +
legacy paths) use the fresh `DEBUG: Memory breakpoint ...` logfile
line as the first-stage signal, then the same bounded readiness
probing. (d) LogTail.expect re-bases when the logfile WRAPS (live
2026-08-24: base=23 unreachable after wrap).
**(4) LANDED:** dbx-plan.rs (boot_note split + `"boot_trap":
"entry"` emitted for walk-less O1 plans + 2 content asserts);
dbx-capgen.py (+610 lines: strict brackets, resume_until_hit,
entry validation, strict arm, memory-signal BPLM waits);
dosbox-harness.sh (2 lines, comment-only); RUNTIME.md (152 lines:
the responsive protocol + the flush/33 ms-redraw source anchors);
13 regenerated capture plans (S0..S8 non-walk carry boot_trap/
entry + drop boot_commands; S0W comment-only); 2 new py test files
(18 unit tests). RIDER RE CORRECTION (re-verified against the
binary this run): the EXD serial-sync bracket string is "Quit from
sychronising" — the original's own typo, exactly one occurrence in
the file, zero "synchronising"; the 0x871a3/0x871ba address
arithmetic (len+1) confirmed in-file. **(5) VERIFIED (this
adoption run):** cargo build + full diffharness suite 101/101
(release), py 18/18, fmt+clippy clean, all 13 plan artifacts
byte-match `dbx-plan` regeneration to a scratch dir (S1-o2 the
untouched control, also byte-identical), all 4 headless dbgprobe
gates GREEN (gate / flow / inject / walk — the legacy BPLM paths
regression-proven through the new resume machinery), MANIFEST
clean before and after (read-only corpus probes only, no Ghidra
run). (worker 29669e49 claim 1)

## D166 — 2026-08-26: P4/RE-objdump `fe93-stride-alias-census` — the "160-B stride at 0x4c69e4" question CLOSED FOR GOOD: independently re-verified as a CENSUS ARITHMETIC SLIP (no second array); the stale §7j.13 OPEN marker amended (docs-only). HEADLINE: the queue item re-asked §7j.11's last open [census] point ("either a SECOND array aliasing the bank base or an original-code quirk — unpinned since 7j.11") UNAWARE that D73/§7j.25 item 7 had already resolved it on 2026-08-21; the residue was the §7j.13 amendment's own `OPEN:` marker (the item's "§7j.11" label was a misnomer — the gloss lives in §7j.13's text, which cites "the 7j.11 k12 sites"). This unit independently re-derived EVERYTHING from ghidra-project/exw-text-objdump.txt and closed the residue. (1) STRIDE ARITHMETIC re-decoded instruction-exact: 0x40fe9c `mov esi,eax` / 0x40fe9e `shl eax,2` / 0x40fea1 `add eax,esi` / 0x40fea3 `shl eax,2` / 0x40fea6 `add eax,esi` → eax = 21·idx (the Watcom ×21 idiom = ((idx<<2)+idx)<<2+idx); the three loads `[eax*8+0x4c69e4]`/`+0x4c69e8`/`+0x4c69ec` then address x/y/z dwords @+0/+4/+8 of the SAME robot record with effective stride 21·8 = 168 = 0xA8 — the canonical stride; the 7j.13 "20·i << 3 = 160" gloss dropped the second `add eax,esi`. (2) CALLER CENSUS re-run over the full objdump text: exactly ONE reference to 0x40fe93 in the entire binary (the direct call 0x40bc44) and ZERO jump-table encodings (byte pattern `93 fe 40 00` — no hits); the site sits in FUN_0040b9f6's per-robot walk with idx pinned by the loop tail 0x40c483..0x40c491: idx ([esp+0x20]) ∈ [0, [0x46ccbc]) — the robot count, ≤ 12. (3) EXTENT MAP: no 20×160 array exists anywhere at the base; the only bank is the D129 12×0xA8 = 0x7E0 zero-fill span 0x4c69e4..0x4c71c4 (FUN_00402965, ecx=0x7E0 @0x40cd29..38); with idx ≤ 11 the highest byte touched is base+0x740 (0x4c7124), well inside; even under the erroneous 160 gloss idx 13 would be needed to cross the extent (count caps at 12), and the strides disagree at every idx ≥ 1, so the instruction decode is dispositive. (4) VERDICT: QUIRK (census slip), NO second array — the queue's alternative "second aliased array" is DISPROVEN. Registry/plan consequence audit: NONE (per the item's own gate — a real second array does not exist; watches.toml and the dbx-plan robot-bank row were always pinned 0xA8; nothing moves). Deliverables: the §7j.13 OPEN→CLOSED amendment, the §7j.25 item 7 addendum (this census), the constant-ledger stride-proof anchor on the robot-bank row, and this entry. Verified: objdump-only (no Ghidra run, no corpus read, no game-data touch), MANIFEST.sha256 clean before AND after. (worker 690e3606 claim 1)

Nudge-Worker: 690e3606-8408-432b-a22f-d64dbfd47a03

## D167 — 2026-08-26: P4/RE-objdump `exd-menu-reset-census` — the §7j.66 open residue CLOSED: the EXD menu-screen counter-RESET family censused from the whole-text objdump and the twin census HOLDS instruction-form-exact (53/53 references; 13 INC + 1 register + 8 zero-writes; bound sequence 200-100-300-200-100-100-300-200 identical; the DEBRIEF twin pinned 0x5638d)

The D156 EXW census's open cross-check ("the EXD menu-screen reset
family is NOT yet censused" — stale even then on the "no EXD
whole-text objdump exists" clause, since ghidra-project/
exd-text-objdump.txt has existed since 2026-08-23/D162) is closed by
a whole-objdump census of every [0x1195f0] reference (RE-EXD-MAP §2b;
objdump-only, no Ghidra run, no corpus read, MANIFEST clean before
AND after). (1) FORM SPLIT EXACT: 53 .text references — 13 INC-form
sites (8 cinematic-loop + 5 menu), 1 register-form mission tail
(0x5a6f0-fd, the §2 pin re-verified), 8 zero-writes, 31 reads
(22 standalone + 8 loop-head cmps + the tail load) — the EXW split
identical down to every bucket. (2) THE EIGHT RESETS: every one
`xor reg; [rider call may ride between xor and store]; mov
[0x1195f0],reg; cmp bound; loop {draw; call 0x1256c; PRESENT
0x10670; inc; jmp}` — present-then-inc, the EXW cinematic order
exact; bound sequence in address order 200/100/300/200/100/100/300/
200 IDENTICAL to EXW; 6/8 xor-registers register-exact, sites 3+6
regalloc-shifted to ebx (semantically void); the riders are
register-preserving setup (0x2d4c3 ×3, 0x503a2 ×1) — the EXW
site-1 FUN_0041cbf0 rider idiom, more frequent in EXD. (3)
CONTAINMENT: all eight in ONE function — the EXD DEBRIEF twin of
EXW FUN_0044425c (RE-EXW-MUSIC screen table), entry 0x5638d, called
from the EXD GameMain @0x2cf3f (the 0x41c610 twin; the GameMain
delta arithmetic predicted ≈0x2cf43). Evidence: shared [esp+0x520]
frame slot + shared exit 0x56835 + intra-family draw helpers
0x574f4/0x5763a; b2-functions.txt NOT used (B2 layout drifts from
EXD in this region). (4) THE FIVE CUMULATIVE MENUS: 0x4d212,
0x4f6b4, 0x4f6fc, 0x4fc17, 0x5148b — each inc immediately followed
by the 0x10670 present (inc-THEN-present, the EXW interactive-menu
order exact; EXW twins OPTIONS ×1 / BRIEF ×3 / SELECT ×1 — the 1/3/1
per-screen split). Per-function attribution of the EXD five: future
work (no EXD function table; ordinal/order/count pinned). (5) VERDICT:
the twin census holds ORDINALLY, instruction-form-exact — no
divergence in count, form, bound sequence, in-loop order, or
containment topology. C₀ CONSEQUENCE: NONE — the §7j.66/D model
carries to EXD verbatim (C₀ = the scripted menu walk's leftover;
O1/O2 = E + C₀; the T2 class absorbs it on both binaries; a live S0W
anchor stop still pins C₀ empirically). Deliverables: RE-EXD-MAP
§2b (the census), the §7j.66 addendum (open note closed), the
DESIGN-DIFFHARNESS frame-counter row note amended (no new ledger
row — the row exists), the differential test extended with the EXD
tables + the ordinal-match assertions (7/7 green), this entry.
(worker e2dded59 claim 1)

## D168 — 2026-08-26: P4/RE-objdump `exd-menu-fn-attribution` — the D167 §2b/C residue CLOSED: the five EXD cumulative-menu INC sites attributed to their screen functions, the 1/3/1 OPTIONS/BRIEF/SELECT split HOLDS, and the EXD screen layout order proven identical to EXW (docs-only)

The one residue the D167 exd-menu-reset-census left open. Bounded
objdump-only unit (NO Ghidra launch; b2-functions.txt untouched);
substrate: ghidra-project/exd-text-objdump.txt + the
tools/exd-relod.py linear image (rebuilt read-only to /tmp/opencode
scratch — the image maps VMA == raw offset in the object2 string
pool, verified by the code immediates 0x86b21 "Name: "/0x86b28
"GOD"). FOUR deliverable points: (1) THE ATTRIBUTION — OPTIONS INC
0x4d212 ∈ entry **0x4c80c**; BRIEF INCs 0x4f6b4/0x4f6fc/0x4fc17 ∈
entry **0x4f1d1**; SELECT INC 0x5148b ∈ entry **0x50953**; SHOP
(0x52fd7) and DEBRIEF (0x5638d, §2b/B) complete the five-screen
layout, ORDER-IDENTICAL to EXW (FUN_0043a5fc < FUN_0043d00b <
FUN_0043e7d4 < FUN_00440e45 < FUN_0044425c). The 1/3/1 per-screen
split HOLDS; no divergence. (2) THE ANCHORS (≥2 §3 classes per
screen): the per-screen "SOUND\MIDI\<NAME>" basename loads
(OPTIONS 0x4c93f / BRIEF 0x4f5e9 / SELECT 0x50ba2 / SHOP 0x5316e /
DEBRIEF 0x5646d — the last +0xe0 inside §2b's pinned DEBRIEF
entry, cross-validating the method; all five through the ONE
common callee **0x1405f** = the EXD load-music-by-basename twin,
BRIEF following with `mov eax,3; call 0x13e04` = music START on
song slot 3, the EXW MusicPump "slot 3 only" fact intact) + the
GameMain dispatch in the EXW call order (0x2cd6e→0x4c80c looped
with the ≥7/[0x119624]≠0 re-init recall, 0x2ce0c→0x4f1d1,
0x2ce45→0x52fd7, 0x2cf3f→0x5638d, 0x2cf7b→0x50953, flanked by the
§5g sound-init/free-voices pair) + the instruction-context shapes
(OPTIONS `mov eax,0x8e; call 0x111fa` ⟷ `0x401ca2` §5h pair; BRIEF
#1/#3 double text-draws at 0x82/0x104·0xbe/0xdc over the
0x30-spaced buffer pairs 0xf7b8c/0xf7bbc ⟷ 0x46b49c/0x46b4cc;
BRIEF #2 the eax=3→0x5b066⟷0x449c94 + eax=0xa→0x2ec12⟷0x41e215 +
0x302-memset 0x12206⟷0x425a1e tail; SELECT triple xor-ecx draws +
post-present mode cmp [0x1075d8]⟷[0x4edb88] §4-pinned; BRIEF #3
post-present cinematics cmp [0x1194d8]⟷[0x46cca4] S0-16/D159
confirming). (3) THE BOUNDARY PROOF: whole-objdump call-target
census — NO direct-call target inside (entry, last INC] for any
group (next targets 0x4e934/0x4fe28/0x5159d, each body closing at a
frame-matched exit tail right before); every entry carries the
family prologue 53 51 52 56 57 55 (OPTIONS 0x4c80c decoded from raw
bytes — the committed objdump desyncs 0x4c7b0..0x4c989, hiding it,
the same desync EXW suffers before 0x43a5fc); the three bodies
share the epilogue trampoline **0x51d11** (pop ebp/edi/esi/edx/
ecx/ebx; ret); BRIEF's INCs all use the 0x650 frame's slots, the
OPTIONS INC uses the prologue's [ebp+0xa]/[ebp+0xe] arena slots;
BRIEF's entry→INC offsets uniformly −9 on EXD (0x4EC/0x534/0xA4F →
0x4E3/0x52B/0xA46), SELECT −0x13, OPTIONS +0x662. NEW helper
identity: **0x4e9a8 ≡ EXW 0x43c87c** (menu-text draw; 4 GameMain
calls per side). (4) CONSEQUENCES: C₀ model unchanged (§2b/E);
registry/ledger NONE (attribution metadata, no new row). Residue
QUEUED: the EXD music-LOADER chain twins (0x1405f / 0x13e04 / the
.MRS/.MRW literals 0x95050/0x95055) vs EXW RE-EXW-MUSIC §1.
Verified: objdump-only, no Ghidra run, no corpus write, MANIFEST
clean before AND after. (worker 4e41bf00 claim 1)

## D169 — 2026-08-26: P4/RE-objdump `exd-music-loader-census` — the D168 residue CLOSED: the EXD music-LOADER chain decoded whole and proven a FAITHFUL PORT of the EXW RE-EXW-MUSIC §1 chain (docs-only)

The D168-queued residue (the EXD music-loader twins). Bounded
objdump-only unit (NO Ghidra launch; no corpus write; MANIFEST clean
before AND after); substrate ghidra-project/exd-text-objdump.txt +
exw-text-objdump.txt + a read-only LE-header image→file map of
BEDLAM.EXD (obj2 page 105 anchored on the probe-1 literal 0x850a3,
content-verifying image 0x85050 = ".MRW\0.MRS\0"). QUEUE CORRECTION:
the D168 residue note's literal addresses "0x95050/0x95055" were a
typo — the real image addresses are 0x85050 (".MRW") / 0x85055
(".MRS"), the two `mov esi` immediates in the loaders (same +5
adjacency as EXW 0x457a21/0x457a1c but ORDER REVERSED — recorded as
RE-EXD-MAP §7 seed 8). (1) THE CHAIN DECODED: EXD 0x1405f = load_midi
(head INSTRUCTION-EXACT twin of FUN_00403642 through the whole
prefix: the two entry gates [0x10743c]/[0x107444] ⟷ [0x4ede58]/
[0x4ede5c], stop slot 3, free-voices, wipe, loop-flag store
[0x87730+2·song] ⟷ [0x45cdc0], strlen, load_mrs call with the file-
base cell 0x894d0 ⟷ 0x45cdd0, the W0/W1 stores, the tables-A/B/C
pointer math, the per-chunk data-ptr/position-counter/init-state
fills at the SAME 0x50/0x28 strides, and the load_mrw tail call);
0x13e04 = MusicStart (four gates in the EXW order incl. the §5g pair
0x13e1c⟷0x4033ec and the NEW hardware-cell alias 0x107654⟷0x4ee9b0;
play flag 0x80338+2·song ⟷ 0x45b010); 0x13f1e = MusicStop; 0x1401a
= the 8×0x14×4 voice-table wipe; 0x4c7a5 = free voices; 0x14409 =
load_mrs (scas-strcat twin with ".MRS"@0x85055, open→size-probe→
ArenaAlloc 0x2e4b2 ⟷ FUN_0041db89→LoadFile-whole 0x2d57c with the
DOS layer named: open 0x2d65a/read 0x2d5c8/probe 0x2d62b/close
0x2d60c); 0x14254 = load_mrw+mrw_load MERGED (u16 n_inst + off/size
record pairs; per-wave record: sound-arena alloc 0x2e4fe, data
+0xF0, size +0xC, rate 0x2B11/8-bit/mono — the DirectSound
constants VERBATIM); 0x138aa = MrsNextEvent (4⟷4 callers, the
0x7531 >30000 reposition constant verbatim, delta stored to chunk
state +0x18 = 0x87770). Fourteen NEW bank-cell aliases pinned
(§5j/B table — the W0/W1 cells, three table ptrs, per-chunk
ptrs/counters/state, the -1 voice-id table 0x87758, the loop-flag
0x87730, the play flag 0x80338, the hardware cell 0x107654).
(2) THE CENSUSES: load_midi callers 5⟷5 (the five screens,
ordinal-identical, ALL song slot 3, own basenames, compiler
fingerprints verbatim — OPTIONS' pre-load stop, SELECT's leftover
`mov edx,0xa`, DEBRIEF's `mov edi,<cell>` rider); MusicStart 6⟷5
(the EXD 6th = the title path 0x5b049 after the sound init — the
same EXD-only title-caller family §5g documented); MusicStop 13⟷11
(the two EXD-only sites EXPLAINED: 0x30ebd inside the shared OOM
fatal helper both arena allocators call — allocator-coupled, not
gameplay; 0x59825 inside the EXD MissionShell FUN_000596ed — a
draw/present/stop/free-voices teardown, EXW MissionShell has one
stop [what verified, why medium]); free-voices 3⟷4 (EXD's third
caller = 0x4c121, free-voices + SFX-reload of MIDIGUN/BOOM1).
(3) VERDICT: faithful port; sole divergences the literal order,
the load_mrw/mrw_load merge, the DOS voice-record stomp vs
DirectSound, and the census extras above. Registry/ledger/C₀
consequence NONE (loader internals are not S0 watches; no
watches.toml/dbx-plan row references any of these cells).
Deliverables: RE-EXD-MAP §5j (the §5g-bis-style addendum) + the
RE-EXW-MUSIC §1 cross-ref + this entry. Residue: NONE — the D168
queue item fully discharged. (worker 829d719c claim 1)

## D171 — 2026-08-26: P4/gate `p4-trigger-contract` — the O2 operational trigger pinned at the emitter: S1-o2 regenerated with `trigger.site = 0x004486C9` (callee `0x00425A03` + EXD `0x0005A6EB` preserved) + the D161 companion-id/count-seed plumbing the locked gates required

NUMBERING: D170 is reserved by the controller's 878c03f STATE.md
"P4 closure scope corrected" note (no DECISIONS entry written); this
entry takes D171 to avoid contradicting committed controller
numbering. The adopted predecessor WIP's two "D170" strings
(watches.toml note, dbx-plan test comment) were corrected to D171.

THE CONTRACT (gate p4-trigger-address; D156, RE-EXW-SIM §7j.66/W11):
the O2 plan's ptrace trigger must be the MissionShell NORMAL-path
PresentEnd CALL SITE 0x4486c9, not the callee entry — PresentEnd
FUN_00425a03 has ~62 direct call sites (the D156 Ghidra reference
census, pinned as constant 62 by static_frame_counter_differential;
an independent exw-text-objdump.txt call-ENCODING count returns 61 —
a method nuance, immaterial: both ≫ 1, so the entry fires on every
menu/loading/cinematic present and is not a frame-tail trigger).
Anchor re-verified this unit, byte-exact at exw-text-objdump.txt:75966:
`4486c9: e8 35 d3 fd ff  call 0x425a03` with the counter read+inc
immediately after (0x4486ce `mov ebp,ds:0x46ae68`; `inc ebp` — the
pre-increment dump ordering, D156). LANDED: `O2_TRIGGER_SITE: u64 =
0x4486c9` const in dbx-plan.rs (an instruction call site has no
registry cell home, so the operational site is pinned at the
EMITTER); S1-o2.json regenerated (trigger + _comment; emitter
determinism proven byte-identical by smoke step (f)); watches.toml
s0-trigger note records the landing. PRESERVED: the registry row's
exw_addr = 0x425a03 (callee canon-of-record) and exd_addr = 0x5a6eb
(the EXD dump-point twin) — all four facts locked by the exact-text
gate tools/check-p4-trigger-contract.py.

ENABLING PLUMBING (why the unit is larger than one line): the locked
capgen smoke drives the committed S1-o2 plan end-to-end, and its rows
(landed D161/D162) already carry companion-span ids
(`static-yline-zbase#zbase`) and count-symbol spans
(`$critter_count*0x7E`, `$poi_count*0x1E`) the dump/stitch/differ
chain had never learned. Taught: (1) `dump::companion_base()` — the
D161 `<base>#<key>` convention (registry ids never contain `#`);
`canonicalize_frame` binds companions to the BASE row's position via
tuple key (companion sorts after base, transcript-order independent;
an unknown base still refuses loudly — anti-ghost); (2) stitch
(runner.rs) binds `<base>#<key>` watch rows to the BASE registry row
so tier + channel address checks apply to the row the span derives
from (full id kept in the dump); (3) the differ's tier_of/seam_of
fall back to the base row; (4) capgen-o2.py seeds `_count`-suffixed
resolve cells with COUNT_SEED=4 (a fabricated POINTER value explodes
count*stride span lengths — the name convention is the load-bearing
dbx-plan resolve symbol) and raises the sanity ceiling 0x100000 →
0x400000 (static-bin-terrain's 0x258960 boot-pass arena legitimately
exceeds 1 MiB; 4 MiB keeps anti-ghost headroom above every pinned
extent). New tests: canonicalize_binds_companion_spans_to_the_base_
row_order, stitch_accepts_companion_span_ids.

GATES: the three locked gate commands GREEN — dbx-plan 35/35; capgen
smoke ALL GREEN (incl. determinism re-emit + manifest); exact-text
check-p4-trigger-contract.py. ZERO CANONICAL-CHAIN MOVEMENT proven
(canonicalize_frame + differ lookups touched): canonical_dump_gate
13 + differ_gate 4 + static_frame_counter_differential 7; full
workspace 736 passed / 0 failed; fmt clean; clippy 0 warnings;
MANIFEST clean before AND after (no corpus read).

WIP ADOPTION: the unit adopted interrupted same-slot predecessor WIP
(session ebca31ce, died between the locked gates and the corpus
gates); git status/diff recorded to .state/scratch/eb9917a1/, all of
it preserved, only the two D170→D171 strings corrected. (worker
eb9917a1 claim 1)

## D172 — 2026-08-26: P4/gate `p4-static-proof-scope` — the `s0-dispositions` gate reconciled to execute the COMPLETE closed static evidence: all 13 static differential oracle suites + the render load-seam oracle + the registry anchor guard (4 commands, 90 tests)

THE GAP: the controller's 878c03f manifest shipped `s0-dispositions`
with ONE command (static_frame_counter_differential only — the D156
timing oracle). The D145-D164 closure evidence is far wider: the
static-parity rows S0-07..S0-17 landed eleven more differential
suites across bedlam-core and bedlam-game, the pre-D145 TS rows
(cgr-volume S0-05, the TOT-driven loader S0-06) landed two more in
bedlam-core, and NO gate ran ANY bedlam-core or bedlam-render test —
the static proof had unguarded halves while the ledger claimed full
disposition.

RECONCILIATION (gate `s0-dispositions`, docs/required-gates.toml):
commands 1..4 run every oracle that proves the 27-row S0 registry
(verified tier census: S0=1 + T0=11 + TS=15 = 27, the D160 final
ledger 24/27 static-closed + 2/27 dynamic-only placement [D156:
s0-trigger + frame-counter] + 1/27 hardware/input-profile-only
[D160: static-cursor-clamp]):
- bedlam-core, 7 suites / 16 tests — pad-slots D146, yline-zbase
  D147, type-table D148, min-bank D149, claim-bank D150+D151,
  cgr-volume + loader (the pre-D145 TS rows).
- bedlam-game, 6 suites / 42 tests — campaign/config D153+D154,
  RNG+dither D155, frame-counter/s0-trigger ordering D156 (the
  timing dispositions), order-table D157, player-type D159,
  cursor-clamp D160.
- bedlam-render --lib, 30 tests — the mission-view load-seam oracle
  (EXW TOT/DAT/BIN/LNK transform = the tot-volume, dat-volume,
  bin-terrain, lnk-map, map-wh TS rows); the whole lib surface runs
  so a renamed-or-vanished oracle cannot silently pass (cargo name
  filters are fail-OPEN — rejected for gate use).
- diffharness registry_anchors, 2 tests — the registry substrate
  guard (W2 anti-ghost; the D161/D162 verified EXD fills anchored).
tracked_paths pinned to the 13 suite files + registry_anchors.rs +
watches.toml + the controller's original four (PLAN, DESIGN,
RUNTIME, STATE). The corpus (11 S0-S8 capture plans) and the other
gates are UNTOUCHED: scripted-slice-scenes and diffharness-plumbing
remain the separate automated S0-S8/differ gates per the item
charter; S0W calibration, live O1/O2/O3, cycles/audio, hardware and
perceptual checks stay EXCLUDED and not queued (D170 scope, restated
as a manifest comment so the exclusion is machine-visible).

VERIFIED this run at the exact gate argv (warm target): core 16/16,
game 42/42, render lib 30/30 (incl.
all_shipped_missions_exw_tot_dat_bin_lnk_transform_matches_mission_
view), registry_anchors 2/2 — 90/90 green; manifest parses
(tomllib, schema required-gates-v1, 4 commands all --locked
--offline against the /usr/bin/cargo allowlist, 19 tracked_paths
all git-tracked); tools/test-validate-required-gates.py 13/13 (the
manifest edit breaks no hermetic validator contract);
tools/test-autonomy-remaining-gaps.sh unaffected (its p4 binding
mocks are hermetic); MANIFEST clean before AND after (no corpus
write; the oracle runs are read-only corpus consumers). PLAN.md P4
acceptance + DESIGN-DIFFHARNESS §CI-wiring note name the gate
content. NUMBERING: D170 reserved by the controller, D171 =
p4-trigger-contract, this entry D172. (worker 7003f272 claim 1)

## D173 — 2026-08-26: P4/gate `p4-dependency-spikes` — the FINAL presentation dependency decision recorded (wgpu stays 27.0.1; winit 0.30.13 window integration; the R8Uint + packed-R32Uint-palette upload path; adapter-free-safe headless acquisition) — closes the D24 "final version call" deferral

Context: PLAN sec 6 P4 item 1 (dependency spikes decided here —
"wgpu version/window integration and indexed-palette upload path")
+ PLAN sec 4 ("DECISIONS.md records each choice + evidence"). D24
pinned wgpu 27.0.1 + pollster 0.4.0 for the P3 skeleton but
explicitly left "the FINAL version call ... with the P4 dependency
spike"; D39 added winit 0.30.13 as the window host. This entry is
that final call, executed as the `dependency-spikes` required gate
(docs/required-gates.toml: two bounded offline locked commands over
tracked_paths Cargo.lock + engine/bedlam-platform/src/gpu.rs +
docs/DECISIONS.md — a docs-only unit; no engine code moved).

1. THE DECISION (final for P4 closure):
   a. wgpu = 27.0.1, UNCHANGED from D24. Cargo.lock today: wgpu
      27.0.1 (+ wgpu-core 27.0.3 / wgpu-hal 27.0.4 / wgpu-types
      27.0.1); exactly ONE workspace pin via the bedlam-platform
      re-export (`pub use wgpu;`, engine/bedlam-platform/src/lib.rs)
      — bedlam-platform asks `wgpu = "27"`, bedlam-shell carries NO
      direct wgpu dependency (the D39 single-pin rule holds). No
      30.x bump: the 27 line is the mature one, the parity pipeline
      uses only baseline WebGPU surface (see c), and a later major
      bump stays presentation-only surgery — goldens + the parity
      hash are CPU-side over the canonical Frame and never touch
      the GPU path (D24/D20).
   b. Window integration = winit 0.30.13 + pollster 0.4.0 (the D39
      shape carried as final: window created inside `resumed()`
      behind `Arc<Window>`, `about_to_wait` borrow-scoped clock ->
      pump -> stage -> redraw; pollster blocking is window-host
      ONLY, never on the sim path).
   c. Indexed-palette upload path = the D24 pipeline, FINAL:
      per-frame re-upload of the 640x480 R8Uint index texture
      (write_texture, bytes_per_row 640) + a 256x1 R32Uint palette
      texture whose entries pack the 6-bit VGA triple
      r | g<<6 | b<<12, re-uploaded ONLY on frame.palette_dirty or
      first upload (the 004ee9b6 handshake analog, DESIGN-RENDER
      sec 2 fact 7); the fullscreen-triangle WGSL expands 6->8 bits
      (Original v<<2 default, Full (v<<2)|(v>>4)) and NEVER
      interpolates indices — bilinear mixes the expanded RGB of
      four neighbors only. R8Uint/R32Uint + TEXTURE_BINDING +
      textureLoad are baseline WebGPU: both device paths open with
      DEFAULT limits and NO optional features (gpu.rs
      new_headless/new_for_surface share that contract), so the
      path needs no adapter capability beyond the baseline.
   d. Adapter policy = ADAPTER-FREE-SAFE: `ParityGpu::new_headless`
      requests a low-power adapter with `compatible_surface: None`
      and returns Option; on hosts with no adapter (pure-CI
      containers) GPU tests SKIP, never fail — this gate is
      hermetically green wherever the crate compiles. The surface
      host path (`new_for_surface`) mirrors the same low-power /
      default-limits / no-features device contract so both hosts
      behave alike.

2. EVIDENCE (this run, the exact gate argv):
   - `/usr/bin/cargo test --release --locked --offline -p
     bedlam-platform` — 9/9 green (scale 8: integer/fit/fill +
     uv-crop geometry; headless 1: `parity_offscreen_roundtrip`,
     which RAN on this host's adapter — no skip marker under
     --nocapture, 0.19-0.22 s — a real offscreen 1280x960 present +
     readback probing the Original expansion 63 -> 252 AND the
     palette_dirty=false palette-reuse pass).
   - `/usr/bin/cargo build --release --locked --offline -p
     bedlam-shell` — Finished release, exit 0 (compiles the winit
     0.30.13 window host + platform + game chain).
   - MANIFEST.sha256 clean before AND after; no game-data/ or
     derived/ write; no Ghidra run.

3. CONSEQUENCE: the D24 deferral is closed; P4 item 1 (dependency
   spikes) now has every element decided in-tree — SMK D30, cpal
   D40, winit D39, and the presentation stack here. No engine
   change (docs-only unit); zero canonical-chain movement by
   construction (presentation never feeds sim or hashed state,
   D12/D17/D20).

NUMBERING: D170 reserved (controller, no entry), D171 =
p4-trigger-contract, D172 = p4-static-proof-scope; this entry D173.
(worker 71effd2b claim 1)

## D174 — gates-validator unit landed; live-facts hardening assertion re-anchored (watchdog repair 2026-08-26)

1. CONTEXT: queue item `p4-required-gates-manifest` churned through
   three structured failures (d7f85d22 client-error, 579650c9
   transport, d6f199cb client-error; all client rc=137 SIGKILL,
   progress=0, queue unchanged). Journal evidence shows the model
   client died to host-level memory exhaustion (node V8
   FatalProcessOutOfMemory traces, 20Gi swap in use) under an
   unrelated concurrent workload on this host — not to any bedlam
   machinery defect. Each session rebuilt the same ~450-line WIP,
   verified it, and died before committing; additionally every
   session burned its tail re-diagnosing
   tools/test-final-hardening-red.sh, which was RED AT HEAD since
   b24f772 moved the three finished P4 units to ## Done.

2. DECISIONS:
   a. The interrupted WIP is ADOPTED AND LANDED whole (commit 9f2a049)
      after full re-verification: writable-bind gate policy, the
      env-probe containment-evidence gate, strict manifest schema,
      private per-command /tmp + scratch HOME, controller-side
      mountpoint pre-creation, 20/20 validator tests.
   b. The hardening case 'five-unit P4 contract retires stale live
      facts' no longer pins the one-time five-item live snapshot
      (with d145/d164/static-evidence/timing body facts that retire
      into the ## Done ledger as units complete, per the D106
      convention). It now asserts the durable invariants: the active
      set is a contiguous front-first-consumed tail of the canonical
      five-id P4 contract, only canonical ids, and the retired
      interactive/perceptual phrasing never returns.
   c. The rc=137 churn itself is environmental (host memory); no
      wrapper classification change is made — a SIGKILLed client is
      indistinguishable from a broken one at the wrapper boundary,
      and the existing transport exemptions already cover the
      provider-side stream deaths.

3. EVIDENCE (this run): tools/test-validate-required-gates.py 20/20;
   tools/test-final-hardening-red.sh PASS (all categories);
   tools/test-reviewer-security-red.sh PASS;
   tools/test-autonomy-remaining-gaps.sh PASS (contains the full
   llm-watchdog suite); strict queue parser RUNNABLE after the
   rewrite; game-data MANIFEST clean before and after.

NUMBERING: D173 = presentation dependency decision; this entry D174.
(watchdog repair 364897 1787768451, adopting WIP from workers
d7f85d22, 579650c9, d6f199cb)

## D175 — 2026-08-27: P5/gate `p5-zone-gate-scaffold` — the per-zone parity LEDGER format decided (docs/P5-MISSION-LEDGER.toml, schema p5-mission-ledger-v1: 37 mission rows, disposition pending|green, catalog_refs feeding P6 triage) + the fail-closed checker wired as the FIRST P5 required gate

Context: PLAN §6 P5 ("Parity completion (per-zone gates)") requires
"an automated completeness gate validates every zone" and "a committed,
schema-validated per-bug ledger" (the original-behavior catalog) that
"feeds P6 triage". The P4 closure (972748d) queued this scaffold as the
first P5 unit: land the per-zone parity LEDGER + the first P5 required
gate before any zone work starts, so every zone unit from now on has a
machine-checkable completion contract (the P4 pattern: gates land as
evidence lands).

1. THE ENUMERATION (read-only, VERIFIED): the 37 shipped missions were
   enumerated from game-data/BEDLAM/EDITOR/ZONE*/MISSION*.TOT — ZONEA
   MISSION1; ZONEB..ZONEF MISSION1..7 each; ZONEG MISSION1 — with
   sha256sum -c MANIFEST.sha256 --quiet clean BEFORE and AFTER. The .TOT
   mission-total file is the runtime-loaded mission identity
   (FORMATS-MISSION §0.2 runtime extension census; the zone-level
   lettered MISSION{A..G}.* files carry no .TOT). Arithmetic
   self-check: TOT = u16 w + u16 h + 8·w·h u16 planes = 4+16·w·h bytes
   (FORMATS-MISSION §2) — 25×75→30004, 100×100→160004, 100×25→40004,
   all 37 files match their zone's expected size; the census equals the
   independent FORMATS-MISSION §0 table (1+7·5+1, 354 375 tiles).

2. THE LEDGER FORMAT (the decision):
   a. Artifact docs/P5-MISSION-LEDGER.toml, TOML (the repo convention —
      required-gates.toml, watches.toml; stdlib tomllib, no deps), schema
      string "p5-mission-ledger-v1" fail-closed.
   b. One [[mission]] row per mission: id "ZONE{L}-MISSION{n}", zone,
      mission, disposition, catalog_refs. Why per-mission rows (not
      per-zone): the acceptance shape is per zone but the work and the
      original-behavior catalog are per mission ("repro, affected
      missions"); zone status is DERIVED (all missions green), never
      stored — one source of truth per fact, no two-field drift.
   c. disposition ∈ {pending, green} — deliberately minimal. Every
      mission starts pending. No "failed"/"blocked" state: the ledger
      records CLOSURE, not attempt history (history lives in NEXT.md
      Done entries + DECISIONS.md); a mission that fails verification
      stays pending. Adding states later bumps the schema string and
      checker together.
   d. catalog_refs = original-behavior catalog entry ids observed on the
      mission (non-empty, unique, whitespace-free strings): the P6
      triage feed named by the plan. A green mission may carry zero refs
      (no divergences found); refs while pending record findings as they
      land.
   e. Per-file digests are NOT duplicated into the ledger: MANIFEST.sha256
      already pins every corpus file and is checked at every gate run —
      the ledger pins identity (zone, mission), not content.

3. THE CHECKER (tools/check-p5-zone-ledger.py + 18-case hermetic suite
   tools/test-p5-zone-ledger.py, all fail-closed): ledger schema/row
   validation (unknown keys, dup ids, id↔zone/mission agreement,
   disposition enum, catalog_ref hygiene); corpus enumeration re-derived
   READ-ONLY at runtime and pinned to the 37-mission zone shape (A:1,
   B-F:7, G:1 — drift anywhere fails loud); ledger set == corpus set;
   and CROSS-ARTIFACT SAFETY with docs/required-gates.toml: a
   p5-zone-{a..g} completion gate present in P5 required_gates requires
   that zone fully green (wiring can never run ahead of closure), and
   manifest P5 status green requires ALL 37 green (a premature phase
   flip fails even with an empty gate list — the validator's
   all-gates-pass semantics alone would let an empty green P5 pass).
   game-data never appears in tracked_paths/corpus (never git-tracked);
   the checker reads the corpus read-only exactly like MANIFEST
   verification, PATH-free under the validator's bwrap.

4. GATE WIRING: P5 required_gates = ["p5-zone-gate-scaffold"] (the
   FIRST entry), commands = the checker + its test suite on
   tracked_paths [P5-ZONE-GATES.md, P5-MISSION-LEDGER.toml, both tools,
   required-gates.toml]; no corpus key. The gate validates ledger
   COMPLETENESS + consistency, NOT zone completion: green from the
   moment it lands (0/37 is the honest scaffold state). Per-zone gates
   p5-zone-{a..g} land as zones close; P5 stays pending until then.
   Acceptance shape recorded VERBATIM in docs/P5-ZONE-GATES.md §1 with
   the seven-criterion decomposition (incl. the DM carve-out as scope,
   not check).

5. VERIFIED THIS RUN: checker OK on the real repo (37 missions, 0/37
   green); test suite 18/18; manifest TOML re-parsed (9 gates, 8 phase
   rows, P5 required_gates exactly one entry); gates-validator 22/22;
   canonical_dump_gate 13/13 (controls — no engine change, docs/tools
   only); MANIFEST clean before AND after; no Ghidra run; no corpus
   write. (worker 05e2d7ae claim 1)

## D176 — 2026-08-27: P5 `p5-mission-load-census` — the all-37-mission READ-ONLY load census VERDICT: every mission loads; the zone work is three named SEMANTIC gap classes (G1 episode-slot/SELECT shell, G2 critter states, G3 zone-BIN variant RE), none parser-sized; ledger unchanged

Context: the P5 opener (D175) queued the load census as the sizing
step before any zone-parity work: drive OUR engine load seams (the
bedlam-render mission-view load path + the bedlam-core/bedlam-assets
loader family — the seams the S0–S8 canonical corpus already exercises
on ZONEA-shaped content) against every one of the 37 ledger missions'
runtime file family (FORMATS-MISSION §0.2), READ-ONLY, and record the
per-mission/per-zone GAP TABLE (docs/P5-ZONE-GATES.md §6) BEFORE any
loader change.

1. THE EXECUTABLE CENSUS (engine/bedlam-game/tests/
   mission_load_census.rs, corpus-gated, deterministic): per mission —
   the canonical 25-name fetch + GameHost::load_mission through
   stage_episode_slot where the slot reaches the mission
   (MissionScene::stage + claim bank directly where it cannot), then
   the destroy family (BDG/POS/TRT), the pickup surface (TOT), the
   critter family (NME), the full bedlam-assets parser family over
   every runtime file, and a scripted frame run (FSM Boot→Mission + 9
   frames host-side; activate + 8 tick/present direct-side; panics
   CAUGHT and recorded as gaps). The pinned table
   (census_matches_pinned_table) is the §6 doc table's machine form —
   D28 fingerprint discipline; census_print_table --ignored prints the
   full columns.

2. THE VERDICT (VERIFIED): ALL 37 LOAD — zero load failures, zero
   parser refusals, zero frame-run panics; destroy/pickup/parsers/frames
   ok on every row; every TOT header re-derives the §2 dims table
   independently. ZONEA-MISSION1 is the only zero-gap mission (the
   canonical corpus's own mission). No mission is unloadable-by-corpus
   → the ledger stays 37×pending (dispositions flip only on
   zone-parity evidence).

3. THE THREE GAP CLASSES (all SEMANTIC, none parser-sized → nothing
   landed this unit; all queued as their own units):
   a. G1 episode-slot seam (10 missions: B–F missions 6–7): FULL_MASK
      pins four sub-slots per stage (B2 @0x81d9a), so stage_episode_slot
      derives missions 1–5 only; the census staged 6–7 DIRECTLY (the
      load_mission body verbatim minus the host) — they load and run
      clean. Fix = the SELECT mission-choice shell, one unit.
   b. G2 critter family scope (26 missions: B–F M1–5 + ZONEG-M1):
      .NME hosts states the controller refuses (§7j.42/6 accepts
      MixedState5+SeekSteppers only) — Shooters/Wanderers/Chasers/
      BallisticState6/CloseCombat/Personnel; ZONEA-M1 passes (the
      modeled slice), the ten 16-byte all-zero .NME missions (B–F
      M6/M7) pass trivially. Per-state AI units.
   c. G3 zone-BIN variant naming (3 missions: ZONEB-M6, ZONED-M5,
      ZONEE-M6): the corpus ships mission-number terrain banks beside
      the zone-level MISSION{L}.BIN; our fetch always builds the
      zone-level name. Override rule = the open RESEARCH-8STREET §3
      question, unresolved against EXW (LIKELY, not VERIFIED). One
      EXW-anchored RE unit.

4. VERIFIED THIS RUN: census 1/1 (pinned 37 rows) + the ignored print
   probe; canonical_dump_gate 13/13; bedlam-game release suite 234/0;
   fmt + clippy clean on the touched crate; MANIFEST clean before AND
   after; no Ghidra run; no corpus write; no canonical-chain movement
   (test-only addition). (worker 7e59f4d7 claim 1, item
   p5-mission-load-census)

## D177 — 2026-08-27: queue grammar restored after the transport-killed end-of-run rewrite (watchdog repair 3644831) — wrapped `[gate=…]` tags and shared umbrella gate ids are the two recurring INVALID-DEADLOCKED shapes; follow-up units keep id==gate

The `p5-mission-load-census` worker (7e59f4d7) finished green (4803d58
PUSHED) but a transport error killed its session between the end-of-run
`.state/NEXT.md` rewrite and the commit, stranding an uncommitted queue
that the strict parser rejected (rc=2 → controller refused idle/spawn for
~6.5 h, the second INVALID-DEADLOCKED of this class after 6355fba).

1. THE TWO GRAMMAR BREACHES (both introduced by hand-wrapping the item
   opening): (a) item 2's `[gate=p5-zone-gate-scaffold]` tag hard-wrapped
   across lines — metadata tags are single-token by
   tools/nudge-free-items.py METADATA_RE, and the canonical
   status/id/gate prefix must sit same-line on the numbered line; (b)
   items 2–4 all carried `[gate=p5-zone-gate-scaffold]`, breaching the
   duplicate-gate rule (:458) — the queue gate tag is per-item IDENTITY
   (the claim binds id+gate), not the required-gates umbrella.
2. THE RULE (same as 6355fba, now pinned by a regression case in
   tools/test-nudge-queue.sh): every active item gets its OWN gate id,
   self-named id==gate (p5-critter-state-g2-wanderers,
   p5-select-shell-g1, p5-zone-bin-variant-g3), and the gate tag is
   never line-wrapped. The umbrella gate a unit must keep green stays
   in its Bounds prose, not in its identity tag.
3. WIP ADOPTED, NOT REDONE: the interrupted rewrite's content is
   preserved verbatim (census done entry, the renumbered queue, the
   successor items 2–4 from the census gap classes G1/G2/G3); only the
   three item-opening lines were rewrapped/regated. The pre-P4 Done-log
   history the rewrite trimmed stays in git at 4803d58.
4. VERIFIED THIS RUN: `nudge-free-items.py --state-v1` → RUNNABLE 1 2 3 4
   (rc=0); tools/test-nudge-queue.sh PASS including the new
   wrapped-metadata rejection; MANIFEST clean before AND after; no
   corpus read beyond the manifest check; no engine change.

## D178 — 2026-08-27: P5 `p5-zonea-mission1-parity` — ZONEA-MISSION1 flipped green (the FIRST zone-parity disposition) with its executable evidence + the `p5-zone-a` completion gate wired; the SAVED/OPTIONS.BDL original-import seam landed EXW-anchored (§7j.70); the CI cross-OS channel honestly recorded RED-for-environment with a queued repair

Context: the queue head (the p5-mission-load-census follow-up) —
ZONEA/MISSION1 to green per the P5-ZONE-GATES §1 acceptance shape,
the ledger flip IN THE SAME COMMIT as the evidence, and (ZONEA having
exactly one mission) the option to wire `p5-zone-a` when the
checker's cross-artifact rule stays green.

1. THE SAVE SEAM (engine, bounded): bedlam-game save.rs = the
   original SAVED.BDL import, anchored by the fresh RE-EXW-SIM
   §7j.70 decode of the restore arm (slot stride 0xB4=180, name@+0,
   mask dword@+8, zone SIGNED word@+0xC -> the 0x4edd8c write, score
   @+0xE, money@+0x12, difficulty@+0x16; empty predicate = zero
   dword@+0x0C; the mask replay marks prior zones fully complete then
   the current zone's bits) — the 8street layout is now EXW-anchored,
   no longer cited. READ-ONLY + bounds-checked by construct (exact
   900 B, slot < 5, empty predicate, zone 1..=8 with mask a sub-mask
   of FULL_MASK[zone] — never guess; the missions-6/7 SELECT shape
   stays rejected loud until G1 lands). GameHost::import_saved_slot
   stages through the D51 seam (exactly the restore's zone-cell +
   mask-replay effect); money/score/difficulty are RETURNED
   (sim-side, DESIGN-GAME sec 3). No writer exists or is owed (new
   saves use the new versioned format, PLAN §6 P5).

2. THE EVIDENCE (tests, the p5-zone-a gate commands):
   zonea_mission1_parity.rs — the per-criterion aggregation: the 8
   ZONEA S-scenarios run their FULL declared budgets crash-free with
   two-run byte identity; the T1 spot table (FULL_MASK @0x81d9a, the
   first-unset-bit selection, the 4000-500d economy seed, the 25-name
   fetch chain); the anchor TS statics re-derived INDEPENDENTLY from
   the TOT header + the §7j.64/D154 fresh scalars; the REAL
   SAVED/OPTIONS.BDL import (slot 0 "PLAYER"/zone 2/mask 0/money
   580/difficulty 1 -> stages ZONEB-MISSION1; the four EMPTY slots
   rejected; OPTIONS volume 75/name "Player") + bounded deterministic
   fuzz (header bit-flip sweeps, truncations, size attacks — Ok/Err
   only). canonical_dump_gate (pinned chains), differ_gate (structural
   spot check), mission_scene_gate (T2 key-moment frames), determinism
   + hash_fixture (replay-hash pins, verified on TWO toolchains:
   stable + nightly, identical) and mission_corpus_gate (T1 deep
   rules) run as the gate's commands.

3. THE HONEST CROSS-OS RECORD (criterion 5): the CI matrix
   (ubuntu+windows) is the designed cross-OS enforcement channel but
   is RED repo-wide for ENVIRONMENT reasons predating this unit
   (>=100 consecutive failures: alsa-sys needs libasound2-dev on the
   ubuntu runner's clippy/build step; the miri job trips file
   isolation on a corpus-gated bedlam-core suite; every failure lands
   BEFORE any test executes — windows only ever fail-fast-cancels).
   NOT a determinism finding: the hashed state is integer-only,
   little-endian by format contract, float-free. The machine evidence
   this unit pins is the fixtures + cross-toolchain equality; the CI
   channel repair is QUEUED as its own unit (ci-cross-os-repair), not
   silently counted.

4. THE DM CARVE-OUT (criterion 7): noted, not checked — DM is
   mode-level (same maps under netplay; no DM map variants exist in
   the corpus). The carve-out's checkable legs (map loads; local SP
   semantics) are criteria 1-2's evidence.

5. THE FLIP + WIRING: docs/P5-MISSION-LEDGER.toml ZONEA-MISSION1 ->
   green (catalog_refs empty: zero divergences observed; legitimate
   per the §3 rule) IN THE SAME COMMIT as the evidence; p5-zone-a
   wired into P5's required_gates with the two evidence commands
   (checker rule 5 green: zone A fully green). P5 stays pending
   (1/37; B-G open behind the G1/G2/G3 census classes).

6. VERIFIED THIS RUN: zonea_mission1_parity 6/6; bedlam-game release
   245/0 (234 + 5 save.rs units + 6 evidence tests); canonical_dump /
   differ / mission_scene / determinism / hash_fixture /
   mission_corpus_gate individually green in gate form
   (--release --locked --offline); fmt + clippy -D warnings clean on
   the touched crate (the 7 workspace clippy notes are pre-existing
   bedlam-core test-file lints under clippy 1.97, untouched files);
   checker OK (ZONEA 1/1 green); gates-validator suite 22/22 + the
   bound P5 phase validation green at the flip commit; MANIFEST clean
   before AND after every corpus-touching run; no Ghidra run; no
   canonical-chain movement (new evidence only re-asserts existing
   pins; the S-flows' chains are pinned once, in canonical_dump_gate).
   (worker 42041a21 claim 1, item p5-zonea-mission1-parity)

## D179 — 2026-08-27: P5 `p5-critter-state-g2-wanderers` — the FIRST G2 critter-state unit: the kind-1 Wanderer (.NME section 2) landed engine-side whole (loader walk + controller), the census re-pinned deliberately (the WanderersxNN refusal component dropped from every row, no row flipped clean), and the §7j.18 hp-scalar gloss CORRECTED to the linear mission m
THREE decisions recorded. (1) **THE RE (§7j.71, committed BEFORE the impl per the stream-survival rule — commits 2195999 + 49aeeeb)**: the k1 controller body 0x414c96..0x415216 decoded whole from the committed objdump: the door-tile entry gate (FUN_004186fc reads the §7j.12 30-B type-DB variant byte at the presence-mark linear index — an E-gap, no engine mirror), the suicide-bomb trigger (FUN_00417e2f: nearest robot < 0x30 px → presence 0 + 8 debris/splash pairs, 5 draws each = 40; the return convention is EXPLICIT mov eax,1/xor eax,eax — CORRECTING §7j.17/2's EAX-leak hypothesis), the (countdown, DIR) substep machine with the IDLE SQUASH semantics (0x4151a5 resets the pause to 1, so the 8..15/12..27 re-pick constants never take effect — the runtime inter-walk pause is 2 substeps), the DIR jump table {0→y−6, 1→x+6, 2→y+6, 3→x−6} @0x412f08, the ±6 RAW-px steppers (kind 1 is px-scale like kind 4, NOT Q13 — the §7j.17 "±6 Q13" gloss corrected), the 8-sample wall probe FUN_0041f8f9 (footprint (−11,−11)/(−11,+12)/(+12,−11)/(+12,+12)/(0,−11)/(0,+12)/(−11,0)/(+12,0) from the 0x4543e4/0x454404 tables; floor_z==z exactly ∧ RAW DAT tile ≤ 3), the toward-robot picker FUN_00417af2 (y-axis wins ties; cx>rx→3 else 1), the death path FUN_00418250 (mode 7 + presence 0 always; the debris quirk compares px vs tile-width so it near-never fires), and the S2 loader walk made exact (DIR seed −1 at spawn — a NEW pin; one FUN_0041ec1c(10)+10 draw per spawned critter — the section's only draw; the z search = first RAW tile ∈ 1..3 scanning down from level 6 with air above). **The hp scalar for EVERY .NME section is 200-style base + base·[0x46ae8c]/27 with [0x46ae8c] = the LINEAR MISSION m (§7j.64/D153), NOT difficulty — §7j.18's "difficulty" gloss is corrected** (imul census: 0xAF/0xC8/0xC8/0x5DC all ×0x46ae8c). (2) **THE LANDING (bounded)**: stage_critters accepts S2 (kind 1, RAW-px, DIR −1, hp = 200+(200·m)/27 via MissionSim::linear — the destroy staging sets it first in the canonical order); the k1 controller lands in bedlam-core::critter (11 new unit tests: staging seeds + draw budget, z-search gates, squash/pick/walk cycle, walk-end, blocked probe, toward-robot axes, suicide 40-draw budget); three new CritterRecord fields (dir w@+0x58, frame w@+0x5A, z_restore d@+0x4E) NOT serialized in the canonical critter-bank blob → no chain movement. **The S3/S4 hp scalars HOLD the §7j.18 difficulty form deliberately**: the S8 canonical chain stages ZONEA S3+S4 and the m-swap would move its pinned T2 bytes — no scenario exercises S2, so per the queue contract NO canonical chain is touched (canonical_dump_gate 13/13 green unchanged); the S3/S4 alignment rides the next G2 unit where a scenario justifies the re-pin. (3) **THE CENSUS RE-PIN (deliberate, D28 discipline)**: `unmodeled_nme_sections` mirrors the engine acceptance set (Wanderers added); every G2 refusal row drops its WanderersxNN component; NO row flips to clean (every Wanderers-hosting mission — 24 of the 26 refusers — also hosts Chasers/BallisticState6/Personnel/etc.); the census_print_table output committed as the provenance (docs/evidence/p5-g2-wanderers-census-table.txt, all 37 rows) + P5-ZONE-GATES §6.2/G2 + §6.3 updated together. Verified: bedlam-core 88/88 (77+11 new); bedlam-game release 245/0 (census 1/1 re-pinned; canonical_dump_gate 13/13 — chains untouched; zonea_mission1_parity 6/6; determinism + differ_gate green); fmt + clippy -D warnings clean on the touched lib (the two remaining all-targets warnings pre-exist in destroy.rs/test targets); gates-validator 22/22; inspect baseline ok; MANIFEST clean before AND after every corpus run; no Ghidra run (objdump + the committed dumps only); staged paths explicit (worker 58b640c3 claim 1).

## D180 — 2026-08-27: queue grammar restored a second time after the D179 completion rewrite (watchdog repair 30933) — the wrapped-gate and prose-bracket breaches; the whole-tag authoring rule now lives in AGENTS.md + the NEXT.md header every rewrite copies
The `p5-critter-state-g2-wanderers` worker (58b640c3) finished green
(2195999 + 49aeeeb + c60c0ba, all PUSHED) and wrote its end-of-run
`.state/NEXT.md` rewrite — but the rewrite carried TWO grammar breaches
in new item 2 (ballistic6), so `boundary_completion_rewrite` correctly
refused to sanction it (nudge-agent.sh: the completion-window check
re-parses the queue), the model died at its own finish line as a
`preflight-mismatch`/`launch-boundary` failure, and the invalid queue
stranded the controller (INVALID-DEADLOCKED rc=2, refusing idle/spawn)
until this watchdog repair.
1. THE TWO BREACHES (both in the hand-written item 2 opening/prose):
   (a) the `[gate=p5-critter-state-g2-ballistic6]` tag hard-wrapped
   across lines — the SAME shape as D177's incident, second recurrence;
   (b) a prose address bracket `[0x46ae8c]` in the RIDER sentence — the
   parser reads EVERY bracket segment in an active item as a tag
   (TAG_RE), and a no-`=` segment fails as "unknown status/tag". The
   parser surfaces one fail() at a time, so (b) hid behind (a).
2. THE REPAIR (WIP adopted, not redone — the D177 precedent): only the
   item-2 opening was rewrapped into the canonical same-line shape
   (`[READY] [id=…] [gate=…] P5` + prose continuation, exactly the
   previous wanderers item's shape) and the prose bracket dropped
   (bare `0x46ae8c`, the house style of items 1/3/4). Everything else
   in the worker's rewrite is preserved verbatim: its Done entry
   (commits re-verified PUSHED), the renumbered queue, the new
   ballistic6 item with the S3/S4 hp rider, and the untouched
   Done-log bracket mentions (the Done section is never parsed).
3. THE PREVENTION SEAM (why this recurs): D177 pinned the parser's
   rejection (test-nudge-queue.sh) but told no AUTHOR. The rule now
   lives where every worker reads it first: AGENTS.md workflow step 7
   (tags stay WHOLE on the item's first numbered line, prose starts
   same-line, brackets are never prose) + an AUTHORING RULE line in the
   NEXT.md QUEUE CONVENTION header that each rewrite copies forward.
4. THE FAILURE ACK: the 58b640c3 preflight-mismatch is resolved as
   replaced-task — the failure's identity (p5-critter-state-g2-
   wanderers) is in the Done log, absent from the active queue, and
   its substantive work is landed+pushed; remediation = the commit
   carrying this entry (it changes .state/NEXT.md, the postcondition
   archive-failures requires).
5. VERIFIED THIS RUN: nudge-free-items.py --state-v1 → RUNNABLE 1 2 3 4
   (rc=0, all items READY id==gate); tools/test-nudge-queue.sh PASS
   including the D177 wrapped-metadata rejection; MANIFEST clean before
   AND after; no corpus read beyond the manifest check; no engine
   change. (watchdog repair token llm-watchdog 30933 1787866581)

## D181 — 2026-08-27: infra `ci-cross-os-repair` — the CI matrix RED-for-environment state repaired (ubuntu alsa + miri file-isolation), restoring the designed cross-OS enforcement channel for P5-ZONE-GATES §7 row 5
1. VERDICT (re-verified via gh before touching anything, run 33116062233
   and >=100 prior): every failure is BEFORE any test executes, both
   causes environmental, neither a code/determinism finding. (a) The
   ubuntu build leg dies at `cargo clippy --workspace --all-targets`
   in the alsa-sys v0.4.0 build script — cpal (a bedlam-shell dep) on
   Linux needs the ALSA headers + pkg-config, which ubuntu-latest does
   not preinstall. (b) The miri job dies at the FIRST filesystem probe
   in bedlam-core's corpus-gated suites: under Miri's default file
   isolation, `open`/`stat` are ABORTING unsupported operations, so the
   skip paths themselves (fs::read in mission_corpus_gate, is_dir in
   the static_* differentials) never get to return their clean
   NotFound->skip result. (c) The windows leg was never its own
   failure — it compiles through clippy and gets fail-fast-cancelled
   behind ubuntu every run.
2. THE FIX (workflow-only, zero engine/test-code change, commit carries
   this entry): .github/workflows/ci.yml gains (a) a Linux-only
   `sudo apt-get install -y libasound2-dev pkg-config` step in the
   build matrix before the cargo commands, and (b)
   `MIRIFLAGS: -Zmiri-isolation-error=warn` on the miri step — the
   warn policy turns isolated file ops into clean syscall errors, so
   the corpus probes return Err/None/false and the suites skip exactly
   as they do on any corpus-less checkout. Chosen over cfg!(miri)
   guards in 9 test files: one line vs churn, identical semantics
   (miri's charter is UB detection over the unit suites — corpus IO
   never runs there), and it keeps the normal cargo-test skip paths
   byte-identical for dev machines and the matrix leg.
3. VERIFIED LOCALLY BEFORE PUSH: a clean clone of HEAD (no game-data,
   exactly a CI checkout) — fmt OK, clippy --workspace --all-targets
   OK, and `cargo test --workspace` green (every corpus-gated suite
   skips cleanly); `MIRIFLAGS=-Zmiri-isolation-error=warn cargo
   +nightly miri test -p bedlam-core -p bedlam-audio` green end to
   end (mission_corpus_gate + all static_* differentials skip via the
   warn-policy error returns; the unit suites all pass). The pushed
   run's green matrix (both legs) is the in-repo evidence recorded in
   NEXT.md.
4. THE WINDOWS LEG THEN CAUGHT A REAL ENGINE BUG (the channel working
   as designed, first windows test run ever): run 33120043475 failed
   EXACTLY ONE test — destroy_gate's k7 debris walk ("freed at the -1
   terminator"), passing on ubuntu in the same run. ROOT CAUSE:
   stage_debris resolved the staged seq-table INDEX by std::ptr::eq
   against DEBRIS_SEQ_TABLES with an `.unwrap_or(0)` fallback; rustc
   gives NO guarantee the slice returned by debris_kind_config is
   pointer-identical to the array entry. A release-profile probe at
   177c953 (worktree, all 20 kinds staged, read debris_bank().table)
   returned table 0 FOR EVERY KIND — the pointer match NEVER succeeds
   in release builds, on Linux too. Debug builds passed (k7 included)
   only by constant-merging luck, and the canonical S4 pin
   (1357af61ef082cb5) had silently encoded the buggy all-table-0 walk.
   Windows-msvc debug simply didn't merge either — exposing in CI what
   every release build had been doing all along.
5. THE ENGINE FIX (commit carrying this addendum): content equality
   (`.position(|t| *t == table)` + expect) — all 11 tables are
   pairwise distinct, so content IS the deterministic identity; each
   kind again walks its OWN §7j.11 table on every platform and
   profile. Two regression pins added in destroy.rs
   (debris_table_index_tests): the staged per-kind table index, and
   the per-kind terminator walk (frees exactly at len-1 ticks, the
   sole -1 sitting at each table's last index).
6. DELIBERATE RE-BASELINE (the one canonical chain movement, both
   pins in the same commit as the fix): canonical_dump_gate
   corpus_s4_destroy_family digest and differ_gate's S4 row
   1357af61ef082cb5 -> 21520352000ca4bf — the corrected walk.
   Verified: bedlam-game release 245/0 (the D179 baseline count,
   S4/S0-S8/determinism/zonea/census all green), diffharness 103/0,
   bedlam-core release 151/0 (incl. the two new pins), fmt + clippy
   -D warnings clean on the touched lib, gates-validator 22/22,
   MANIFEST clean before AND after every corpus run.

## D182 — 2026-08-28: P5 `p5-critter-state-g2-ballistic6` — the SECOND G2 critter-state unit: BallisticState6 (.NME section 6) staged engine-side through the already-landed k5/6 shared body, the census re-pinned (the BallisticState6xNN component dropped, no row flipped clean), and the D179 RIDER landed — the S3/S4 hp scalars aligned to the 0x46ae8c linear-mission m with the S8 canonical chain re-baselined deliberately
1. RE FIRST (§7j.72, decompile-only from the COMMITTED
   exw-critterpoi-loader.txt — no Ghidra run): the S6 staging block
   walked exact — ONE critter per 8-B record at EVERY difficulty
   (no inner spawn loop; the S3/S7 multiplier preambles and their
   RandA draws absent), ZERO stream draws, the S3 stamps verbatim
   (kind 6, species 3, mode 8, anim 5, heading 0x72, the w1-level
   floor probe FUN_0041e411(x>>8,y>>8,w1<<5), countdown 0), no home
   stamps (only S5 writes +0x42/+0x46), hp = 0x96+(m·0x96)/0x1B on
   the SAME [0x46ae8c] cell — closing the §7j.71/1 imul census:
   EVERY section's scalar is the linear mission m, none difficulty.
2. ENGINE (bedlam-core::critter): `stage_critters` accepts section
   6 — the E S3 block verbatim with kind 6 and ONE draw-free spawn
   (the k5/6 dispatch arm `5 | 6` predates it from W12-S8; home =
   spawn rides the E S3 convention). 3 new unit tests (the
   one-each/draw-free/stamps pin at d=0 and d=3, the file-order
   S2→S3→S4→S6 staging, the scalar proof: d=3/m=5 → hp 177/237
   where the difficulty form said 166/222, m=0 → the base).
3. THE CENSUS RE-PIN (the D28 fingerprint rule): BallisticState6xNN
   dropped from all 26 hosting rows; NO row flipped clean (every
   host also carries Shooters/Chasers/CloseCombat/Personnel;
   ZONEA-MISSION1 stays the sole clean row). The print-table output
   committed as provenance (docs/evidence/
   p5-g2-ballistic6-census-table.txt); P5-ZONE-GATES §6.2/G2 + the
   §6.3 table + the §6.4 rollup re-baselined together.
4. THE CHAIN DECISION (the scenario the D179 queue item named as
   justifying the re-pin): S8 stages ZONEA S3+S4 under
   `critters = 1` with NO `destroy = 1`, and `MissionSim::linear`
   is destroy-staged → m = 0 there → the staged hp drops 155→150
   (kind-5) / 207→200 (kind-4) and the S8 chain moves:
   canonical_dump_gate corpus_s8 + differ_gate's S8 row
   10c78a7144cf6d3d -> bac6a3053cedfebd (both re-baselined in this
   unit's commits, pins + assert comments updated together). The
   ORIGINAL at ZONEA/M1 reads the derived cell (clamp to 1) → hp
   155/207; the divergence is S8's own deliberate no-destroy
   staging — the same "0 when unstaged" class D179 accepted for S2,
   documented §7j.72/4. Paths staging destroy before critters (the
   canonical order, every census row) read the true m. The S8
   death-timeline asserts survived as inequalities (diving ≥ 8 at
   f39, dormant == diving at f120) — only the chain + hp equality
   pins moved.
5. Verified: bedlam-core release 155/0 (18 critter tests), bedlam-
   game release 245/0 (canonical_dump_gate 13/13 re-pinned,
   differ_gate 4/4, census green), diffharness 103/0, fmt clean,
   clippy clean on the touched files (the 5 remaining workspace
   warnings pre-exist at HEAD), gates-validator 22/22 at the final
   commit, MANIFEST clean before AND after every corpus run.

## D183 — 2026-08-28: P5 `p5-select-shell-g1` — the G1 SELECT mission-choice shell landed: the census G1 question answered (missions 6-7 are the MP-ONLY files — the SELECT MP write pair + the load-time +5), the sibling host seam `stage_select_mission` + the five-bit save/SELECT domain (`SELECT_FULL_MASK`) + the mission-derivation saturation landed, the census re-pinned (ten `select:clean` rows), NO canonical chain movement

1. **THE RE (a5c3a71, §7j.73, objdump-only from the committed
   exw-text-objdump.txt — no Ghidra run):** the runtime
   mission-number source is the SELECT screen's write pair
   {zone cell 0x4edd8c, mission cell 0x4edd88}, read from its
   strategic-map PIXEL→ID grid. The SP arm (0x43ee48..0x43ee9d)
   writes missions 1..5 per zone ONLY (26 hot spots = ZONEA{1} +
   5×{B..F} = MAX_LINEAR; zone G has none — it is the
   campaign-advance endgame); the MP arm (0x43edc2..0x43ee43)
   writes BOTH cells from 10 list rows → {zone 2..6, mission
   1..2}; and `build_mission_paths` @0x4467df ADDS 5 to the
   mission cell in mode 2 — **so ZONE{B..F}/MISSION{6,7} are the
   MP-only missions, and NO stage mask (the 4-bit B2 FULL_MASK or
   the 5-bit EXW save shape) can ever express them: they are not
   campaign sub-missions.** The save-restore replay tests FIVE
   mask bits (0x43c2bf..0x43c36c, subs 1..5) — the EXW save
   domain is `mask ⊆ 0b11111`; the 27-record completion bank
   (0x4decae, 0x144/0xC) is the SELECT screen's own state
   (FUN_004474ef/44751c). RE notes committed BEFORE the engine
   change (the stream-survival rule).
2. **THE SEAM (the landing commit):** a SIBLING of the D51
   campaign seam, not an extension — `Episode::stage_slot`'s
   ACCEPTED mask domain widens to `SELECT_FULL_MASK` [0,1,31×7]
   (stage 1 keeps its single sub: ZONEA's one bank record), while
   `Episode::complete` still walks FULL_MASK (the canonical S5
   zone-staging semantics INTACT — verified: canonical_dump_gate
   13/13 + differ_gate 4/4 + determinism green, NO chain moved).
   New `SceneFsm::stage_select_mission(zone 2..=6, mission 1..=2)`
   plants the MP write pair as staging-ONLY state (NOT in
   scene_hash — the D31 movie pattern, pinned by test);
   `GameHost::mission_slot()` applies the +5
   (`SELECT_MP_FILE_OFFSET = 5`) so the pair names
   ZONE{B..F}/MISSION{6,7} exactly as the original loads them;
   campaign staging CLEARS the pair (the restore/advance shells
   rewrite the cells). `mission_number_for_mask` now SATURATES at
   5 — the SP SELECT domain — so the campaign path can never name
   an MP file (the invariant is property-tested over the whole
   five-bit domain).
3. **THE D178 RIDER:** save.rs's import domain widened to
   SELECT_FULL_MASK — a SELECT-shaped save (bit 4 = sub 5
   complete, e.g. the full 0b11111 zone-complete mask) imports +
   stages cleanly now; what stays rejected loud is anything past
   bit 4 (no original writer can produce it — the bank has 27
   records). The §7j.70 "missions-6/7 SELECT shape stays
   rejected" note retired; the mask-0x10 rejection test became
   the select_shape_imports acceptance + a 0x20 rejection.
4. **THE CENSUS RE-PIN (deliberate, D28 fingerprint rule):** the
   ten B-F missions-6/7 rows moved from the direct fallback to
   the SELECT seam — all ten `select:clean` (empty .NME, full
   budget, frames ok; the load column gains the `select` value).
   The §6.1 headline gains the ten clean rows; §6.2/G1 marked
   LANDED; §6.3/§6.4 re-baselined; the provenance artifact is
   docs/evidence/p5-g1-select-census-table.txt.
5. Verified: bedlam-game release 249/0 (+4 net new tests:
   the fsm seam, the host seam + hash invariance, the save
   SELECT-shape import; the census re-pin rides the existing
   census_matches_pinned_table), fmt + clippy clean on the
   touched files, MANIFEST clean before AND after every corpus
   run, no Ghidra run.

## D184 — 2026-08-28: P5 `p5-zone-bin-variant-g3` — the G3 zone-BIN variant question CLOSED with a NO-SWAP verdict: the EXW runtime always loads the zone-level `MISSION{L}.BIN` (path2 is letter-only, unconditional); the three shipped mission-number variant banks are runtime-dead editor residue; our engine rule VERIFIED, engine untouched, census NOT re-pinned

Three decisions recorded:

1. **THE VERDICT (EXW-anchored, RE-EXW-SIM §7c.9, [verified]).** The
   open question (RESEARCH-8STREET OPEN QUESTIONS #3; the P5 census
   G3 class, D176): do ZONEB/MISSION6, ZONED/MISSION5, ZONEE-MISSION6
   load their mission-number `.BIN` variants instead of the zone-level
   `MISSION{L}.BIN`? **NO.** `build_mission_paths`@0x44670c (walked
   whole, 0x44670c..0x446907) builds path2@0x4dca8c — the base for
   `.CGR/.BIN/.MIN/.LNG/.LNK` — as `EDITOR\`+`ZONE`+chr(0x40+
   [0x4edd8c])+`\MISSION`+chr(0x40+[0x4edd8c]): the zone letter
   appended TWICE, NO itoa leg, NO conditional (the function's only
   branch remains the G1/D183 +5 on path1's mission number when
   [0x4edb88]==2). The `.BIN` consumers are exactly two, both on
   path2, both immediately after their own builder call: load_mission
   @0x41dcbc (tag 0x4587e8) and the brief-reload twin FUN_0044661b
   @0x446644 (tag 0x45979a). The joined name lives in the
   concat-private 0x40-B buffer 0x4dca4c (one 3×0x40 family with the
   two path buffers; only concat@0x41dbed touches it), opened
   cwd+name "r+b" by open@0x41cd90. A complete 29-site path-buffer
   census names every consumer: path2 = the five family tags only;
   path1 = `.TOT/.DAT/.PAD` + the `.MRK/.NME/.TRT/.POS/.BDG` loader
   family + the `GAMEGFX\BRF_{L}{level}` movie-name scratch reuse
   (AFTER the twin's load — no interference) + the save-path reuse. A
   whole-image ASCII string census finds NO hardcoded
   `ZONE?\MISSIONn.*` literal. The EXD twin agrees (load block
   0x2e5c3, builder 0x58606, `.BIN` on path2 0x92f34, tag table
   byte-verified at linear 0x862a9, builder tail letter-only). Data
   corroboration [DATA, read-only]: only zone-level `.MIN` ship, each
   16× the ZONE-level BIN count (B 1872 / D 1450 / E 1455 — never the
   variant counts 1443/1443/1120; a swap would desync the minimap
   walk), and ZONEB/MISSION6.BIN ≡ ZONED/MISSION5.BIN byte-identical
   (sha256 5735b08a3e08853e…, a shared dev/deathmatch bank).
2. **THE DISPOSITION.** Engine untouched — `mission_asset_names`'
   `{ZONE{L}/MISSION{L}.BIN}` rule is verified correct as-is. The
   census is NOT re-pinned: its rows already loaded the zone-level
   bank and stayed green; the G3 mention was a docs-side open flag,
   now resolved (P5-ZONE-GATES §6.2/G3 CLOSED, §6.3 row notes, §6.4
   rollup, confidence tags; FORMATS-MISSION §0.2 + §23 corroboration;
   RESEARCH-8STREET §1.0/§1.1/§7 glosses corrected + OPEN QUESTIONS
   #3 ANSWERED — the 8street "loaded only when the mission has its
   own" gloss was wrong, now superseded by the EXW anchor per the
   8street policy).
3. **CONSEQUENCE.** The P5 zone-parity surface narrows to G2 (the
   critter states + the S8 personnel/POI bank) — the G2 Shooters unit
   is the queue head. No ledger movement, no canonical-chain
   movement, no loader change (test- and docs-only unit).
   Verified: objdump-only from the committed
   ghidra-project/exw-text-objdump.txt + exd-text-objdump.txt (no
   Ghidra run), tag/name string bytes read from the pinned binaries
   with `sha256sum -c MANIFEST.sha256 --quiet` clean BEFORE and AFTER
   every corpus-read, gates-validator 22/22 at the bookkeeping commit,
   no Rust change (fmt/clippy N/A).

## D185 — 2026-08-28: autonomy `queue-parser-completion-window` — the strict queue preflight stops classifying the AGENTS.md step-7 completion window (owner claim over a departed id, wrapper alive in its grace or dead awaiting the reaper) as INVALID-DEADLOCKED; capability-based classification instead, launch bindings stay byte-strict (watchdog repair 1253812)

1. **THE FAULT.** The step-7 contract has the WORKER rewrite NEXT.md
   (claimed item → ## Done) as its final act, but the canonical
   `N-owner.claim` is released only by the wrapper epilogue after model
   exit (or by the reaper's DEAD_CLAIM_TTL if the wrapper was killed).
   Between those two events the claim names an id that left the active
   set — the sanctioned completion shape — yet
   `nudge-free-items.py::validate_v2_claim` failed the WHOLE preflight
   on it (`active queue identity mismatch`, rc=2), so any controller
   tick inside the window forced a watchdog repair. Five forced
   repairs in 28h (2026-08-27 22:17, 23:36; 2026-08-28 00:50, 01:55,
   02:32); the 02:32 one stopped the wrapper's unit mid-grace, killing
   the epilogue that would have unlinked the claim — orphaning
   `1-owner.claim` over a COMPLETED and PUSHED unit (`7326236`,
   worker cef2f815) and wedging the loop: preflight rc=2, reaper TTL
   not yet aged, controller refusing idle/spawn.
2. **THE FIX (capability-based, parser-side).** An OWNER claim whose
   (id, gate) left the active set no longer fails the preflight: an
   unlocked one has no ownership capability and never suppresses work
   (the pre-existing `claimed_ordinals` contract); a locked one only
   holds its slot (→ CLAIMED-RUNNING) while its live wrapper finishes
   inside the wrapper-enforced boundary grace (900s, D-b0059c4). The
   reaper still deletes the residue after DEAD_CLAIM_TTL, and
   `resume_glm` reaps with TTL=0 before restarting the controller.
   UNCHANGED and still fatal: reservation claims with a departed
   identity (they authorize a launch), and any owner claim whose
   identity MATCHES the active queue but whose body/hash binding does
   not (tamper protection). Precedent: identity-less lock-v1 owner
   claims never bound to item identity at all.
3. **CONSEQUENCE.** A controller tick that lands inside a completion
   window now sees RUNNABLE/CLAIMED-RUNNING and stands down or spawns
   the successor — no repair cycle, no wrapper kill, no orphaned
   claim. Regression-pinned in test-lock-v2-adversarial.sh
   (`case_completion_claim_is_not_a_deadlock`: unlocked residue →
   RUNNABLE; locked window → CLAIMED-RUNNING; reservation mismatch →
   fatal; matching-identity hash tamper → fatal). All ten automation
   suites green (queue, claims, controller, watchdog, adversarial,
   waiting-automatic, automation-failure, remaining-gaps,
   final-hardening, systemd-unit-sync). No queue-grammar change, no
   claim-file changes, no game-data access.

## D185 — 2026-08-28: P5 `p5-critter-state-g2-shooters` — the kind-2 SHOOTERS state LANDED (the .NME S1 staging + the k2 sine-walk shooter body); the census Shooters class CLOSED with ONE deliberate row flip (ZONED-MISSION5 clean — the queue's "no row flips clean" expectation falsified by that row)

Four decisions recorded:

1. **THE S1 LOADER WALK (EXW-anchored, RE-EXW-SIM §7j.74/1,
   [verified]).** Per 10-B record (w1 spawn base, w2 variant flag,
   w3/w4 x/y tile): spawn count `w1+difficulty` CLAMPED ≥ 1
   (0x4164eb — a NEW pin the §7j.18 gloss lacked); per attempt two
   FUN_0041ec1c(5) scatter draws set x/y = (tile+pick−2)·0x2000
   (Q13), then the MAP-BOUNDS DROP GATE (x≤0 ∨ x>>13 ≥ W ∨ y≤0 ∨
   y>>13 ≥ H → no critter, count NOT incremented, both draws already
   consumed — a NEW pin absent from §7j.18); on pass the stamps are
   species 1, z 0xC000 FIXED, heading 0, anim RandA&7, variant
   pick(4)+3 NEGATED by the w2 flag, hp = 0xAF+(m·0xAF)/27 (the
   0x4165db imul site, m = [0x46ae8c] the linear mission — the
   closed census), state 2, timer (RandA&0x1F)−0xF at +0x72 (a dead
   stamp for kind 2 — the body never reads it; the DRAW is
   stream-live). DRAW BUDGET: 2 per dropped attempt, 5 per landed
   critter.
2. **THE k2 CONTROLLER BODY (0x415216..0x415466, §7j.74/2,
   [verified]).** Substeps = the record's species word (≡ 1 for S1;
   the k4/k5/6 convention). Per substep: anim := (anim+1)&0xF;
   heading := (heading+variant)&0xFF — the variant IS the curve
   rate, the w2 flag's negation turns it; the sine walk advances
   x/y by (cos/sin word ·0x14)>>8 with NO bounds gate/wall probe/z
   change; TWO always-consumed RandA gates — the 1/128 SQUAWK pulse
   (FUN_0043a48e, the [0x4edffc] voice base; the play is T4 E-gap,
   verified draw-free) and the 1/4 fire chance (RandA&3==0 —
   CORRECTS §7j.17's "every 4th substep": a per-substep CHANCE);
   the fire arm bounded-picks a robot SLOT over [0x46ccbc] (skip
   when the +0x7C alive word is 0), takes the FIRST-FREE 0x4cc654
   slot (FUN_0041286f's identity pinned — the shared 50×0x22
   allocator, twin of the engine's enemy_free_slot), aims at the
   robot with a ±0x1F00 Q13 jitter per axis, gates on the 2-D
   octile `dist>>8 < 300−(2−d)·64` (the dz is DEAD for the gate —
   FUN_0041ebf8 never reads its third argument; it only feeds the
   velocity stamp), and stamps projectile 0x65 at the critter
   position with the RAW direction>>5 velocity (NOT
   octile-normalized — closer targets fly slower bolts, unlike the
   mode-2 0x68 lane). The kind-2 z cell is Q13 (the S1 0xC000 = 6
   levels) — the documented exception to the record's Q5-z rule.
3. **ENGINE CONSEQUENCE (landed).** `stage_critters` accepts .NME
   section 1 (kind 2; the bounds-drop gate and the 2/5-draw attempt
   budget exact); the k2 body lands in bedlam-core::critter with 10
   unit tests (staging seeds/clamp/drop-gate/draw budget, the
   heading precession + walk deltas on the sin(a·π/128) table, the
   0x65 stamp with the jitter-invariant and RAW >>5 velocity
   check, the range gate + dead-robot skip, the exact
   5-draw fire frame, the presence gate). The new `variant` record
   field is NOT serialized in the canonical bank blob (the §7j.71
   dir/frame/z_restore convention) and NO canonical scenario stages
   S1 — ZERO chain movement (canonical_dump_gate 13/13 +
   differ_gate 4/4 + determinism re-asserted green).
4. **THE CENSUS RE-PIN (deliberate, D28; ONE row flip).** The
   ShootersxNN component dropped from all 17 hosting rows AND
   ZONED-MISSION5 FLIPPED CLEAN — it was the one host whose only
   unmodeled section was Shootersx4, so the queue item's "expect NO
   row flip clean — Chasers remain on every host" was FALSE for
   exactly that row (its Chasers count was zero); documented here +
   in P5-ZONE-GATES §6.1/§6.2/§6.3/§6.4. The G2 class is now
   Chasers + CloseCombat + the S8 personnel bank (25 missions);
   12/37 missions now LOAD clean (ZONEA-M1, the ten MP, ZONED-M5)
   while the zone-parity ledger stays 1/37 green (dispositions flip
   only on zone-parity evidence, §3/§5 — unchanged). Provenance:
   docs/evidence/p5-g2-shooters-census-table.txt. Verified:
   bedlam-core + bedlam-game release suites green, fmt + clippy
   clean on the touched crates, gates-validator 22/22, inspect
   baseline ok (1069 files), MANIFEST clean before AND after every
   corpus read, no Ghidra run.


## D186 — 2026-08-28: autonomy `p5-critter-state-g2-chasers-r2` — the zero-progress transport-death re-issue: worker fa32df63 (kind=transport, provider HTTP 502) died before any work, the wrapper released the claim with the queue byte-unchanged, and the pending nudge-failure-v1 artifact would force every watchdog cycle into urgent repair; the failed task identity is REPLACED by a fresh re-issue (id/gate `p5-critter-state-g2-chasers` → `p5-critter-state-g2-chasers-r2`, scope/bounds verbatim) in the remediation commit, archiving the failure as replaced-task (watchdog repair 1671051)

The G2 CHASERS unit stays the queue head, still READY, still the same
work (RE first, engine second, census re-pin, all bounds unchanged).
Only the identity is renewed: the archive contract
(`nudge-state.py archive-failures`) releases a failure record only
when its `(ordinal, id, gate)` triple leaves the active `## Now`
section under a remediation commit that itself rewrites the queue —
a transport death at zero progress has no WIP to adopt (unlike the
d6f235c and b0059c4 precedents, where the dead worker had finished
substantively), so the sanctioned exit is the test-contract shape
(tools/test-automation-failure-watchdog.sh: replace the failed
identity, keep a READY successor active). Rationale: leaving the
artifact unarchived churns one full repair session per watchdog
cycle against a healthy queue; renumbering alone would satisfy the
letter of the check while lying about intent — the identity is what
failed, so the identity is what gets re-issued.

Verified this run: strict queue parser rc=0 (`RUNNABLE 1 2`) before
and after the rewrite; the failure artifact identity
(fa32df63-64d6-48e3-8218-3f730be10307.json, device 52, inode
6475634, sha256 c04fd2b0…ba0ed) matches the wrapper snapshot
byte-for-byte; no claim, cooldown, or taskfail entry binds the old
id; no docs reference the old id literally; no game-data touch, no
Rust change (fmt/clippy N/A); failure-watchdog harness contract
re-run green. Note: D185 was minted twice (autonomy + shooters);
this entry takes D186 without renumbering history.

## D187 — 2026-08-28: P5 `p5-critter-state-g2-chasers-r2` — the kind-3 CHASERS state LANDED (the .NME S5 staging + the k3 distance-ladder body); the census Chasers class CLOSED with TWELVE deliberate row flips (the Chasers-only hosts ZONEB M1-5, ZONEC M1/M2/M4/M5, ZONED M1-4 — 24/37 load clean), the second and largest no-flip-expectation falsification after D185's ZONED-M5

THREE decisions recorded. (1) **THE RE (§7j.75, committed BEFORE the
impl per the stream-survival rule — commit c0c8279)**: the S5 loader
block decoded draw-free and exact — ONE critter per record at every
difficulty (no spawn loop, no stream draws), x/y = tile·0x2000+0xF00
(Q13), z = the FUN_0041e411 floor probe at level w2, the 8 corner-z
words, and home x/y/z staged (S5 is the ONE home-stamping section);
the §7j.18 "+0x10 = +0x12" gloss CORRECTED — the second w1<<6 stamp
is the DWORD at +0x14, the preserved spawn heading; species 8, MODE
0 (awake-idle, not 8), hp 1500+(1500·m)/27 (the linear-m cell, as
every section). The k3 body 0x4145c1..0x414c96 decoded whole: NO
substep loop (species is NOT a substep count for kind 3 — it has
THREE other roles: the 8-frame spawn GRACE read as the R2 gate, the
0x20 return-home WALK BUDGET stamped by rules R1/R4, and the wake
clear); the target-liveness flip runs BEFORE the mode dispatch
(dormant/dying included); the dormant block carries a NEW pin — the
TELEPORT-HOME at EXACTLY delay−0x14 (20 frames before the wake,
restoring the +0x14 spawn heading); the wake stamps hp FLAT 1500 (no
m scalar); the 4-rule distance ladder made exact (R1 dist>200 ∧
mode2, R2 species==0 ∧ dist<200 ∧ leash<400 ∧ mode∉{3,2}, R3
dist<100, R4 leash≥400); the mode-3/0xA bodies re-aim every 9 frames
through the 8-SECTOR SNAP ((angle+0xF)&0xFF)>>5&7)<<5 and step on
the raw DGROUP walk table [0,0,1,1,0,0,0,1,1,1] = 6 steps/10 frames;
mode 2 fires 0x67 EVERY frame (the §7j.17 ">4 shots → reset" gloss
is the 5-frame aim-countdown wrap 0→4→0, not a fire gate) with the
LIVE-robot 3-D octile velocity (the 0x68 lane's exact math); the
pathfinder FUN_0041571c decoded whole — the open sine-step
(cos/sin>>5) + the WALL-FOLLOW ladder on the record word w@+0x5E
(NOT the kind-1 DIR +0x58), every blocked exit copying sector →
heading; the walk gate FUN_0040cc27→FUN_0041e9a2 reads its z
reference from the dword@+0x5E>>16 = the FIRST CORNER-Z WORD w@+0x60
(why the loader stages the corner words) and settles z on pass; the
whole k3 chain is DRAW-FREE (zero RandA/FUN_0041ec1c sites in the
body and every helper — the first critter section with zero per-frame
stream draws). (2) **THE LANDING (bounded)**: `stage_critters`
accepts section 5 (kind 3) + the k3 body in bedlam-core::critter
with 11 unit tests; the FUN_0040cc27 gate refactored into the shared
`walk_gate` (critter_step_heading behavior-identical); three new
CritterRecord fields (home_z, spawn_heading, seek_sector) NOT
serialized in the canonical blob → no chain movement (verified:
canonical_dump_gate 13/13 + differ_gate 4/4 + determinism green; no
canonical scenario stages S5 — ZONEA/M1 hosts S3+S4 only). (3)
**THE CENSUS RE-PIN (deliberate, D28)**: `unmodeled_nme_sections`
adds Chasers; the ChasersxNN component dropped from all 17 hosting
rows AND TWELVE Chasers-only hosts FLIPPED CLEAN — ZONEB M1-5,
ZONEC M1/M2/M4/M5, ZONED M1-4 — 24/37 load clean (was 12); the
queue's "expect NO row flip clean unless a host carries no other
unmodeled state" carve-out exercised at scale (every B/D campaign
mission and most of C hosted Chasers ONLY); provenance
docs/evidence/p5-g2-chasers-census-table.txt; the G2 residue =
CloseCombat (kind 7) + the S8 personnel/POI bank (13 missions); the
ledger stays 1/37 (dispositions flip only on zone-parity evidence).
Verified: bedlam-core release suites green (114 lib incl. 11 new),
bedlam-game release green (census 1/1 re-pinned; canonical_dump_gate
13/13 + differ_gate 4/4 + determinism green), fmt + clippy clean on
the touched files (the destroy.rs/static-claim-test warnings
pre-exist), gates-validator 22/22, inspect baseline ok (1069
files), MANIFEST clean before AND after every corpus read, no
Ghidra run (worker bc51a491 claim 1, commits c0c8279 + 542ec3f).


## D188 — 2026-08-28: autonomy `queue-grammar-prose-bracket-3` — the THIRD strict-grammar breach in a completion rewrite (watchdog repair 2157361): the Chasers worker's step-7 rewrite carried memory-cell notation as prose brackets — `[0x4eba0c]++ + [0x4eba10]=0x32` inside the S8 item body — and the parser (`item 2: unknown status/tag [0x4eba0c]`, rc=2 INVALID-DEADLOCKED) refused every subsequent spawn; the false launch-boundary preflight-mismatch on bc51a491 is archived replaced-task

The worker bc51a491 finished `p5-critter-state-g2-chasers-r2`
SUBSTANTIVELY (c0c8279 + 542ec3f + ac7445a, all pushed; HEAD ==
origin/main at repair time) — its step-7 queue rewrite is the dirty
WIP this repair adopts byte-for-byte except for the four bracket
characters. The queue-sha the failure record captured as
`queue_after` (95715352…ac65) is exactly that rewrite, so the
preflight-mismatch evidence ("queue changed after model start") is
the completion bookkeeping itself, not corruption — the same false
shape as watchdog repair 1007791 (b0059c4 grace widening); the
actual defect is only the bracket grammar. Remediation: debracket
the two memory cells in the S8 item (`cell 0x4eba0c++ + cell
0x4eba10=0x32`, meaning verbatim), leaving the two READY items and
the whole Done log untouched; the failure (bc51a491…json, device
52, inode 6569728, sha256 dedfef8e…fff35, ordinal 1,
id/gate p5-critter-state-g2-chasers-r2) is archived replaced-task
— the completed identity left `## Now` for the Done log, its
successors (closecombat head, S8 personnel second) stay READY and
claimable. Rule reaffirmed (D177/D180 and the NEXT.md header every
rewrite copies): in an ACTIVE item every `[` must open a canonical
tag — RE address notation must stay bracket-free (bare `0x…`,
`cell 0x…`, `word 0x…`); brackets remain free prose ONLY under
`## Done`/`## Backlog`. Verified this run: strict parser rc=0
(`RUNNABLE 1 2`) before commit and after; item-v2 identities match
the intended heads; failure-snapshot record matches the live
artifact byte-for-byte; failure-watchdog harness contract green;
no game-data touch, no Rust change (fmt/clippy N/A).

## D189 — 2026-08-28: P5 `p5-critter-state-g2-closecombat` — the kind-7 CLOSE-COMBAT state LANDED (the .NME S7 staging + the k7 steer/beam body + the kind-7 knock lane); the census CloseCombat class CLOSED with ONE deliberate row flip (ZONEC-MISSION3 — the one CloseCombat-ONLY host, no Personnel), the third no-flip-expectation falsification after D185/D187

FOUR decisions recorded. (1) **THE RE (§7j.76, committed BEFORE the
impl per the stream-survival rule — commit 533eaac)**: the S7 loader
block decoded exact — the spawn count is the §7j.18 S3 cascade made
precise {d=0→1, d=1→(RandA&1)+1, d=2→2, d≥3→1} (NOT "max(d,1)"),
and the roll is ONE SECTION-LEVEL draw computed BEFORE the record
loop (0x416e36..0x416e80 — even an EMPTY section draws at d=1);
x/y = tile·0x2000+0xF00 (Q13), z FIXED 0xDF (Q5 by value — 6·0x20+
0x1F, NO floor probe, NO home stamps, NO bounds gate), anim 0,
countdown 0, heading = FUN_0041ec1c(0xFF) (the ONLY per-critter
draw), MODE 3 (ACTIVE from frame 0 — never dormant), species 1,
hp 2500+(2500·m)/27. The k7 body 0x412f52..0x41367c decoded whole:
mode 7 dying despawns on the FIFTH frame (countdown++ > 4 → hp 0 ∧
presence 0); mode 6 ballistic integrates the in-record knock triple
×2/frame with the +2/frame fall-rate ramp (cap 0x18) and the floor
LANDING TEST, the landing staging 8 debris (kind 6, delays 1..8) +
5 claim-gated splash tiles (z level (z>>5)+2 clamp 7, delays 1..5)
+ 24 effect rows — the §7j.43/2 "0x18 k7-only" pin anchored; mode 5
is the KNOCK DRIFT (countdown++ first, >10 → mode 3 — TEN drift
frames, and the tail engage runs the flip substep against the STALE
scan cells); every other mode runs ONLY the nearest-robot scan (a
dormant k7 is inert). The engage tail (mode 3 ∧ sticky dist < 0x320
— a FLAT gate, CORRECTING the §7j.42 "(d+1)·0x40+600" k5/6-leash
gloss): a nonzero countdown only decrements; else the ±1 STEER
(FUN_00412a19 decoded: wrap8(aim−heading), δ≥0x80 → −1 else +1,
equal → 0) at the LIVE scan robot with the critter side
LOW-BYTE-SCRUBBED, the cos/sin>>6 move (no wall probe), and the
TWO-CONJUNCT fire gate — point-blank sticky-dist < 0x50 ∧ the
(g_frame_count+idx) modulo 0x1F/0xF/0x7 by difficulty (≥3 NEVER —
§7j.16's "32/16/8" made exact, idx-staggered) — stamping projectile
0x69 {x/y Q13 post-move, z LITERAL 6, counter 0, TTL 0x18, NO
velocity} + the 6-frame countdown recharge. The whole approach/fire
chain is DRAW-FREE (the only body draws are the landing's 122).
(2) **THE ENGINE (0ab42a3)**: stage_critters accepts section 7 and
the k7 body lands in bedlam-core::critter with 11 unit tests (50
critter tests green, bedlam-core 154); the weapon→critter hit lane
specializes kind 7 (the away heading, the in-record vx/vy
cos/sin>>6, mode 5 + countdown 0 — no juice roll). (3) **THE STAGING
CONVENTION RIDER**: the engine models the d-cascade roll
PER-RECORD (the landed-S3 convention of §7j.72), so an EMPTY S7
consumes no draw — deliberately deviating from the asm's
unconditional section-level roll to keep the canonical S8 chain
byte-identical (the queue's no-chain-movement bound); the asm truth
is recorded in §7j.76 and the S3+S7 section-level-roll re-baseline
(moving the S8 chain deliberately) is queued as its own unit.
(4) **THE CENSUS RE-PIN (D28 fingerprint rule)**: the CloseCombatxNN
component dropped from all 8 hosting rows AND ZONEC-MISSION3 FLIPPED
CLEAN — the one CloseCombat-ONLY host (CloseCombatx4, no Personnel);
the queue's "every CloseCombat host also carries Personnel today, so
the expectation should HOLD" expectation was wrong for that single
row, documented + deliberate (the D185 ZONED-M5 / D187 twelve-row
precedent); 25/37 load clean (was 24); the G2 residue = the S8
personnel/POI bank ALONE (12 missions — ZONEE M1-5, ZONEF M1-5,
ZONEG-M1); the ledger stays 1/37 green; provenance
docs/evidence/p5-g2-closecombat-census-table.txt; P5-ZONE-GATES
§6.2/§6.3/§6.4 re-baselined. The k7 DEATH handler (FUN_0041896c —
kind flips to 6, w@+0x78 := 1, 3 falling gibs + 1× k0xD + CACODETH
+ bounty +1000) stays the documented unlanded §7j.24 subset
alongside k1/k2/k3, as does the 0x69 beam TICK/impact (§7j.50).

## D190 — 2026-08-28: autonomy `transport-death-end-of-run` (watchdog repair 2797116): the CloseCombat worker 7c028ff1 completed all three task commits (533eaac + 0ab42a3 + 1e18478) and then died at a provider transport timeout (client_rc=124, progress=1) BEFORE the gates battery, the queue rewrite, and the push; the retry worker e264f8b5 died identically while re-reading state — the repair adopts the commits verbatim, finishes the end-of-run bookkeeping, and archives the transport failure replaced-task

FOUR facts recorded. (1) THE STALL: two consecutive provider
transport deaths left a fully-landed task unrecorded — HEAD at
1e18478 with a queue still offering the same item, three unpushed
commits, and a structured nudge-failure-v1 artifact (kind
transport, queue_unchanged true). No task-side defect existed:
the worker's own log shows the census re-pin commit landing, then
the transport cut. (2) THE ADOPTION: the repair changes NO engine
code — it adopts 533eaac/0ab42a3/1e18478 as the task's completion
(D189 already records the landing decisions), re-verifies the
focused release battery at 1e18478 (bedlam-game:
mission_load_census census_matches_pinned_table ok +
canonical_dump_gate + differ_gate + determinism +
mission_scene_gate + zonea_mission1_parity; bedlam-core:
hash_fixture + mission_corpus_gate — full battery exit 0),
confirms MANIFEST clean, rewrites the queue (the CloseCombat item
moves to Done; the S8 personnel/POI item becomes the head), and
pushes 4913a65..1e18478 plus this repair commit. (3) THE CONTRACT
NOTE: per the 05e14378 precedent, the gates-validator battery runs
AFTER the bookkeeping commit — the repair runs it at its own
clean head; the earlier validator complaint at 0ab42a3 (required
tracked path differs from HEAD) was the dirty-tree precondition,
not a gate failure. (4) THE ARCHIVE: the failure artifact
7c028ff1-976c-4676-b09e-1539318d6a36.json is acknowledged
replaced-task with remediation_commit equal to this repair commit
(the commit that carries the NEXT.md rewrite establishing the
postcondition: ordinal 1 is p5-personnel-poi-s8, strict parser
rc 0); the dead claim 1-owner.claim is left for the wrapper's
reaper (workers never touch claim files). Queue grammar kept
strict: no prose brackets in the surviving active item, tags
whole on the first line.

## D191 (2026-08-28, p5-personnel-poi-s8): the S8 PERSONNEL/POI bank
LANDED — the LAST G2 census class; G2 EMPTY, 37/37 load clean

The unit closed the census G2 tail end-to-end in three commits
(5219569 RE + b80aa45 engine/census + this bookkeeping). (1) RE
FIRST (§7j.77, objdump-only from the committed loader decompile +
exw-text-objdump.txt — no Ghidra run): the .NME S8 loader walk made
exact AND the §7j.18/1 seed list CORRECTED — the 2026-08-21 reading
transposed the +4/+6 stores; the asm (0x417076/0x41707e) seeds
STATE 1 = IDLE (personnel do NOT spawn in state 5 ESCAPE), the dead
angle seed 5 sits at +6, and +2's 0x32 is the HP word (the
FUN_0040dc1b damage lane decrements it; death → state 6 panic with
ONE RandB draw for the death sound). The whole controller
FUN_00412a98 body: the per-frame prologue (z re-settle + the
nearest-exit scan writing the [esp] distance cell), the head 1/16
flee lane (exit within 0x180 + PHASE 2), the idle/settle/walk-out
machine (the 0xC0 split, the 1/16 gates, the nearest-robot settle
aim), the flee walk (exit re-aim sector<<5, the walker, the 10000
never-expire sentinel, the abort-to-idle), the 0x10 arrival →
ESCAPE (timer −1, 10 ticks) → the award (active 0, [0x4eba0c]++,
[0x4eba10] 0x32, exit dwell reset, FUN_00448b80(5000)), the 6→7
panic tail, and the walker FUN_00415b6c whole (the ≤4 floor gate —
NOT the critter walk_gate's 3 — plus the quadrant ladder). Corpus
census: ELEVEN hosting files (the queue prose's "13 missions" was
an arithmetic slip), 125 records → 500 POIs. (2) ENGINE (b80aa45):
crate::poi — the PoiRecord bank + the section-8 staging (three
RandA draws per POI: x/y in-tile scatter + heading, the w1-level
floor probe, hp the literal 0x32 with NO m-scalar — the ONE .NME
bank without the formula), the controller subset under the SAME
critter-family arm (MissionShell 0x447fe6 adjacency), the
host-staged 5-slot exit seam (the §7j.19 controller-read subset),
the damage-lane seam, +5000 through the score-pending fold; 15 unit
tests; the bank NOT hashed (the W6 split). E-GAPS documented: the
RandB sound pick, both SFX, the death effect, the MissionShell
banner countdown, the animator, and the blast-debris CALLER of the
damage lane (0x40e158 — the debris bank has no behavior tick
engine-side). (3) CENSUS RE-PIN (deliberate, D28): the
PersonnelxNN component dropped from all 11 rows (ZONEE M1-5,
ZONEF M1-5, ZONEG M1) and EVERY ONE FLIPPED CLEAN — CloseCombat had
already landed (D189) so Personnel was each row's last unmodeled
section; 37/37 load clean; the ledger stays 1/37 green (dispositions
flip only on zone-parity evidence); provenance
docs/evidence/p5-g2-personnel-census-table.txt. ZERO canonical
chain movement (ZONEA/M1 hosts no S8 section — verified by the
byte-exact 8-section walk): canonical_dump_gate + differ_gate +
determinism green. P5-ZONE-GATES §6.1/§6.2/§6.3/§6.4 re-baselined.
Verified: bedlam-core 201 tests green (15 new), bedlam-game 20
suites green, fmt + clippy clean on the touched crates, gates-
validator 22/22, inspect baseline ok (1069 files), MANIFEST clean
before AND after every corpus read, no Ghidra run. Zone parity work
continues per PLAN §6 as per-zone DISPOSITION evidence (the §7
ZONEA pattern).

## D192 (2026-08-28, p5-zone-b-disposition): ZONE B CLOSED — the
first 7-mission zone flips green (8/37); the zone-parity harness
GENERALIZED to any ledger mission

The unit landed in two commits (2980e8b harness + the evidence/flip
commit). (1) GENERALIZATION (2980e8b): the D178 ZONEA shape lifted
to a ZoneSpec-parameterized suite (engine/bedlam-game/tests/
zone_mission_parity.rs) so the P5-ZONE-GATES §1 criterion table is
executable for ANY ledger mission; the scenario grammar gained the
v1.8 `mission = <1..=7>` header key (requires `zone`; 6..7 pinned
to zones B..F — the SELECT write-arm domain; fail-loud
range/duplicate/pairing gates, parser tests, and dbx-plan records
the `_e_staging` seam note), and the canonical runner's staging arm
selects the seam per §7j.73: campaigns 1..=5 through the CAMPAIGN
episode slot at the completion mask whose first-uncompleted sub is
exactly the mission (mask (1<<(m-1))-1 — mission_number_for_mask
inverts it), the MP files 6..=7 through the SELECT write pair
ALONE (campaign staging would clear the pair). (2) ZONEB EVIDENCE
+ FLIP (same commit, the §5 cross-artifact rule): all seven
missions' criterion-1 battery (boot + 120-frame passive + 48-frame
full-staging destroy/pickup/platforms/critters, two-run byte
identity, full declared budgets) + the committed ZONEB flows
S5/S5B/S5C re-run; the anchor TS/T0 statics re-derived from each
TOT header (100×100, 160004 B) + the §7j.64 formula (linear =
clamp(n-1, 1, 26) — M1/M2 both floor to 1); the T1 spot table
(FULL_MASK arithmetic, start_score, the per-mission fetch chain
with the D184 zone-level CGR/BIN/LNK pin, the seam domains incl.
campaign-clears-select); the shipped SAVED.BDL slot-0 campaign =
ZONEB/MISSION1 import + bounded fuzz; the DM carve-out is
LOAD-BEARING for M6/M7 (the MP-only files: maps load through the
SELECT seam + local SP semantics re-derived). P5-ZONE-GATES §8
documents the per-criterion table; the ledger flips
ZONEB-MISSION1..7 green (catalog_refs = [] — no divergences
observed); the p5-zone-b gate joins the P5 required_gates
(offline evidence commands only). Verified: bedlam-game 21 suites
green (canonical_dump_gate 13/13 — ZERO canonical chain movement;
differ_gate 4/4; determinism; census 37/37 unchanged;
zonea_mission1_parity 6/6; zone_mission_parity 5/5), bedlam-core
green, diffharness green incl. the new parser tests, fmt + clippy
clean, gates-validator all-green, MANIFEST clean before and after
every corpus read, no Ghidra run. The remaining zones C..G are
DISPOSITION-side units instantiating the ZoneSpec parameter.

## D193 (2026-08-28, p5-zone-c-disposition): ZONE C CLOSED — the
SECOND 7-mission zone flips green (the ledger 15/37) and the
zone-parity harness carries the CLOSED-ZONE LIST. (1) HARNESS SHAPE
(f4ab798): the D192 single-`ZONE`-const suite lifted to
`ZONES: &[ZoneSpec]` (B then C) so a zone's disposition unit
APPENDS its spec and the closed set never loses its executable
evidence — the alternative (re-instantiating the const per zone)
would strand every earlier `p5-zone-{b..}` gate on a suite that no
longer exercises its zone; both gates now run the same command over
the same file. (2) ZONEC EVIDENCE + FLIP (same commit, the §5
cross-artifact rule): the FIRST PURE instantiation — zone C ships
NO committed .scen flows (scenarios-tree grep verified), so the
generated per-mission battery IS the whole criterion-1 leg: all 21
flows (P5CM1A..P5CM7C: boot, 120-frame passive, 48-frame
full-staging destroy/pickup/platforms/critters) full declared
budgets, dumps verify, two-run byte identity — NO engine gap
surfaced on any ZONEC mission (the unit's stopping condition did
not trigger); the anchor TS/T0 statics re-derived from each TOT
header (100×100, 160004 B — all seven verified) + the §7j.64
formula (zone C: linear = clamp(5+m−1, 1, 26) = m+4); the T1 spot
table per zone (FULL_MASK arithmetic, start_score, the 25-name
fetch chain with the D184 zone-level CGR/BIN/LNK pin
`EDITOR\ZONEC\MISSIONC.*`, the seam domains at stage 3 / zone cell
3 incl. campaign-clears-select); the criterion-6 SAVED/OPTIONS
import tests stay FILE-LEVEL (the shipped slot-0 campaign IS
ZONEB/MISSION1 — hardcoded to zone B, never derived from the list;
the zone-C campaign staging rides criterion 2's seam legs + the
fuzz's in-model staging assert). P5-ZONE-GATES §9 documents the
per-criterion table (the §8 pattern) + the closed-zone-list tail
note on §8; the ledger flips ZONEC-MISSION1..7 green
(catalog_refs = []); the p5-zone-c gate joins the P5
required_gates (offline evidence commands only; 12 gates total);
the ledger test pin re-baselined 8/37 to 15/37 + the ZONEC 7/7
line (deliberate, same commit — the D28 fingerprint discipline,
D192 precedent). Verified: zone_mission_parity 5/5 (both zones),
bedlam-game suites green (canonical_dump_gate 13/13 — ZERO
canonical chain movement; differ_gate 4/4; determinism; census
37/37 unchanged), bedlam-core 201/0, diffharness 104/0, fmt +
clippy clean, check-p5-zone-ledger OK + hermetic suite 18/18, the
gates validator all-green at the flip commit, MANIFEST clean
before and after every corpus read, no Ghidra run. The remaining
zones D..G are pure ZONES-append units in the §9 shape.

## D194 — 2026-08-28: autonomy `preflight-mismatch` + `INVALID-DEADLOCKED` (watchdog repair 3709375): the zone-C worker 4016c154 landed both task commits (f4ab798 + dcfdcc8) and WROTE its end-of-run queue rewrite, then died at a transport error before the bookkeeping commit and the push — the controller saw the launch boundary change (preflight-mismatch, reason launch-boundary) AND the rewritten queue fail the strict parser (rc=2), refusing idle/spawn
FOUR facts recorded. (1) THE DEADLOCK: the worker's completion
rewrite carried the prose bracket `&[ZoneSpec]` inside the ACTIVE
item — the third recurrence of the D177/D180 class (a Rust slice
type copied into queue prose; the strict parser reads every bracket
as a tag and rejects `unknown status/tag [ZoneSpec]`). With rc=2
the controller refuses to spawn, so the loop stalls with a valid,
finished task and an unclaimable queue. (2) THE REPAIR (D190
pattern, adopted verbatim): fix the single bracket to
`(ZoneSpec-slice)` — no other byte of the rewrite touched — restore
the worker's parked STATE.md edit (/tmp/opencode/STATE.md.p5c,
saved when the HEAD-bound validator battery required a clean
tracked-path tree), land the bookkeeping commit, push the two
stranded task commits with it, and archive the failure
replaced-task (identity (1, p5-zone-c-disposition,
p5-zone-c-disposition) absent from the resulting queue — its
successor p5-zone-d-disposition is the head item). (3) THE LESSON
(D180 restated, now third recurrence): queue prose NEVER carries
square brackets — not Rust types (`&[ZoneSpec]`), not arrays
(`[]`), not optional markers; the Done section tolerates them, the
active item does not. Workers writing queue text that quotes Rust
signatures must paraphrase slice/array types parenthetically. (4)
VERIFIED: strict parser rc=0 RUNNABLE 1 at the bookkeeping commit;
check-p5-zone-ledger 15/37 with ZONEC 7/7; zone_mission_parity
5/5; the nudge-queue parser suite green; MANIFEST clean before and
after every corpus read; no Ghidra run.

## D195 (2026-08-28, p5-zone-d-disposition): ZONE D CLOSED — the
THIRD 7-mission zone flips green (the ledger 22/37). The unit is
the FIRST PURE ZONES-APPEND disposition (the shape every remaining
zone E/F/G takes): no harness change, no grammar change, no engine
change — the suite-append commit 681db03 adds the D spec (letter D,
missions 1..=7, dims 100x100, committed flows NONE — no committed
.scen stages zone D) and nothing else; the flip commit lands the §10
table, the ledger rows, the p5-zone-d gate and the 15/37->22/37
test-pin re-baseline together (the §5 cross-artifact rule, the D28
fingerprint discipline). Zone-D-distinct fact: M5 carries the G3
variant bank ZONED/MISSION5.BIN — re-verified byte-identical to
ZONEB/MISSION6.BIN (the D184 twin), RUNTIME-DEAD editor residue
under the no-swap verdict, so it adds NO evidence leg beyond the
zone-level fetch-chain assert (criterion 2's EDITOR\ZONED\MISSIOND.*
pin). Evidence: the generated battery IS the whole criterion-1 leg
— P5DM1A..P5DM7C all 21 flows full declared budgets, dumps verify,
two-run byte identity; NO engine gap surfaced on any ZONED mission
(the stopping condition did not trigger); the anchor statics
re-derived from each TOT header (100x100, 160004 B — all seven) +
the §7j.64 formula (zone D: linear = clamp(5*2+m-1, 1, 26) = m+9,
M1..M7 = 10..16, no clamp bite); the T1 spot table per zone (FULL_MASK
arithmetic, start_score, the 25-name fetch chain, the seam domains
at stage/zone-cell 4 incl. campaign-clears-select); criterion 6
stays FILE-LEVEL (the shipped slot-0 campaign IS ZONEB/MISSION1).
P5-ZONE-GATES §10 documents the per-criterion table; the ledger
flips ZONED-MISSION1..7 green (catalog_refs = []); the p5-zone-d
gate joins P5 required_gates (13 gates); the ledger test pin
re-baselined 15/37 to 22/37 + the ZONED 7/7 line (deliberate, same
commit). Verified: zone_mission_parity 5/5 (three zones, 16.07s),
bedlam-game suites green (canonical_dump_gate 13/13 — ZERO canonical
chain movement; differ_gate 4/4; determinism; census 37/37
unchanged), bedlam-core + diffharness green, fmt + clippy clean,
check-p5-zone-ledger OK + hermetic suite 18/18, gates-validator
suite 22/22 + the HEAD-bound battery all-green at the flip commit
(13 gates), MANIFEST clean before AND after every corpus read, no
Ghidra run. The remaining zones E/F/G are pure ZONES-append units
in the §9/§10 shape.

## D196 (2026-08-28, p5-zone-e-disposition): ZONE E CLOSED — the
FOURTH 7-mission zone flips green (the ledger 29/37). The unit is
the SECOND PURE ZONES-APPEND disposition (the §9/§10 shape, what
zone F/G take next): no harness change, no grammar change, no
engine change — the suite-append commit 9410e0d adds the E spec
(letter E, missions 1..=7, dims 100x100, committed flows NONE — no
committed .scen stages zone E) and nothing else; the flip commit
lands the §11 table, the ledger rows, the p5-zone-e gate and the
22/37->29/37 test-pin re-baseline together (the §5 cross-artifact
rule, the D28 fingerprint discipline). Zone-E-distinct fact: M6
carries the THIRD G3 variant bank ZONEE/MISSION6.BIN — its OWN
bank (1508806 B, sha256-verified distinct), NOT the
ZONEB/MISSION6.BIN dev twin (ZONED/MISSION5.BIN is that twin) —
RUNTIME-DEAD editor residue under the D184 no-swap verdict (the
runtime unconditionally fetches the zone-level MISSIONE.BIN,
1968763 B), so it adds NO evidence leg beyond the zone-level
fetch-chain assert (criterion 2's EDITOR\ZONEE\MISSIONE.* pin).
Evidence: the generated battery IS the whole criterion-1 leg —
P5EM1A..P5EM7C all 21 flows full declared budgets, dumps verify,
two-run byte identity; NO engine gap surfaced on any ZONEE mission
(the stopping condition did not trigger); the anchor statics
re-derived from each TOT header (100x100, 160004 B — all seven) +
the §7j.64 formula (zone E: linear = clamp(5*3+m-1, 1, 26) = m+14,
M1..M7 = 15..21, no clamp bite); the T1 spot table per zone
(FULL_MASK arithmetic, start_score, the 25-name fetch chain, the
seam domains at stage/zone-cell 5 incl. campaign-clears-select);
criterion 6 stays FILE-LEVEL (the shipped slot-0 campaign IS
ZONEB/MISSION1). P5-ZONE-GATES §11 documents the per-criterion
table; the ledger flips ZONEE-MISSION1..7 green (catalog_refs =
[]); the p5-zone-e gate joins P5 required_gates (14 gates); the
ledger test pin re-baselined 22/37 to 29/37 + the ZONEE 7/7 line
(deliberate, same commit). Verified: zone_mission_parity 5/5 (four
zones), bedlam-game suites green (canonical_dump_gate 13/13 — ZERO
canonical chain movement; differ_gate 4/4; determinism; census
37/37 unchanged), bedlam-core green, fmt + clippy clean,
check-p5-zone-ledger OK 29/37 + hermetic suite 18/18,
gates-validator suite 22/22 + the HEAD-bound battery all-green at
the flip commit (14 gates), MANIFEST clean before AND after every
corpus read, no Ghidra run. The remaining zones F/G are pure
ZONES-append units in the §9/§10/§11 shape.

## D197 (2026-08-28, p5-zone-f-disposition): ZONE F CLOSED — the
FIFTH 7-mission zone flips green (the ledger 36/37). The unit is
the THIRD PURE ZONES-APPEND disposition (the §9/§10/§11 shape,
what zone G takes next): no harness change, no grammar change, no
engine change — the suite-append commit 99bb89a adds the F spec
(letter F, missions 1..=7, dims 100x100, committed flows NONE — no
committed .scen stages zone F) and nothing else; the flip commit
lands the §12 table, the ledger rows, the p5-zone-f gate and the
29/37->36/37 test-pin re-baseline together (the §5 cross-artifact
rule, the D28 fingerprint discipline). Zone-F-distinct facts:
(a) zone F ships NO mission-number variant bank AT ALL — the only
zone-F .BIN is the zone-level MISSIONF.BIN (1464679 B) — so the
D184 no-swap verdict needs no variant caveat (unlike D/E, which
each pinned a runtime-dead variant bank first) and the zone-level
fetch-chain assert (criterion 2's EDITOR\ZONEF\MISSIONF.* pin) is
the whole G3 leg; (b) the §7j.64 linear formula reaches its EDGE:
linear = clamp(5*4+m-1, 1, 26) = m+19, M1..M7 = 20..26 — M7 = 26
EXACTLY TOUCHES the clamp ceiling without a bite, the first
mission of the ledger to reach it. Evidence: the generated battery
IS the whole criterion-1 leg — P5FM1A..P5FM7C all 21 flows full
declared budgets, dumps verify, two-run byte identity; NO engine
gap surfaced on any ZONEF mission (the stopping condition did not
trigger); the anchor statics re-derived from each TOT header
(100x100, 160004 B — all seven) + the §7j.64 formula; the T1 spot
table per zone (FULL_MASK arithmetic, start_score, the 25-name
fetch chain, the seam domains at stage/zone-cell 6 incl.
campaign-clears-select); criterion 6 stays FILE-LEVEL (the shipped
slot-0 campaign IS ZONEB/MISSION1). P5-ZONE-GATES §12 documents
the per-criterion table; the ledger flips ZONEF-MISSION1..7 green
(catalog_refs = []); the p5-zone-f gate joins P5 required_gates
(15 gates); the ledger test pin re-baselined 29/37 to 36/37 + the
ZONEF 7/7 line (deliberate, same commit). Verified:
zone_mission_parity 5/5 (five zones), bedlam-game suites green
(canonical_dump_gate 13/13 — ZERO canonical chain movement;
differ_gate 4/4; determinism; census 37/37 unchanged), bedlam-core
green, fmt + clippy clean, check-p5-zone-ledger OK 36/37 +
hermetic suite 18/18, gates-validator suite 22/22 + the HEAD-bound
battery all-green at the flip commit (15 gates), MANIFEST clean
before AND after every corpus read, no Ghidra run. The remaining
zone G is a pure ZONES-append unit in the §9/§10/§11/§12 shape and
closes the ledger.

## D198 — 2026-08-28: watchdog repair 314485 finishes the
watchdog-killed zone-F worker b5bce035 (the D190/D194 pattern,
third recurrence) and retires the stale no-progress marker that
caused the kill. Chain of evidence: the 09:39 wrapper pass recorded
automation-failures/224613cc (kind no-progress, gate
p5-zone-e-disposition) even though that run had in fact completed
its unit — the failure record itself shows queue_change=modified
and the run's completion commit efd1112 IS origin/main — so the
09:49 watchdog pass hit the force_repair path on the still-unacked
marker (no supervisor session is consulted on that path), paused
autonomy, and terminated the live zone-F worker mid HEAD-bound
battery with BOTH its commits (99bb89a + 29cfc3f) already landed
and every unit bound verified per its log. Remediation, smallest
concrete cause only, no tooling change: (1) re-validate the killed
worker's evidence first-hand (ledger OK 36/37 + ZONEF 7/7,
hermetic 18/18, zone_mission_parity 5/5 at 29cfc3f, parser rc=0,
nudge-queue suite PASS, MANIFEST bracketed clean); (2) land its
end-of-run bookkeeping (queue + STATE + this entry) with the
failure-ack bound to this commit — resolution replaced-task: the
failed gate's identity left the active queue when ZONEG became the
head — and push the stranded commit pair. Structural note for the
record: the recurring kill window is a completing worker's final
bookkeeping+push stretch, where it holds no in-flight commit the
wrapper can count as progress, so a transport hiccup there converts
a completing run into a no-progress marker that the next watchdog
pass spends on killing its successor; the ack protocol is the
designed retirement path and works whenever a repair actually
finishes a unit. The next worker picks up ZONEG (one mission; the
ledger closes 37/37).

## D199 (2026-08-28, p5-zone-g-disposition): ZONE G CLOSED — the
LAST ledger mission flips green and the P5 mission ledger reads
37/37 (every shipped mission green, every zone closed). The unit
is the FIFTH PURE ZONES-APPEND disposition in the §9/§10/§11/§12
family, with the census-forced ONE-seam delta: zone G is the first
ONE-mission zone in the closed list, and its zone cell 7 lies
OUTSIDE the SELECT write arm's 2..=6 domain with no MP file
shipping for G (§7j.73), so the suite-append commit 0829187 adds
the G spec (letter G, missions 1..=1, dims 100x25, committed flows
NONE — no committed .scen stages zone G) AND derives the SELECT
write-pair legs of zone_t1_rules_spot from the zone's own mission
range (zones B..=F exercise the identical legs they always did;
the write-arm reject domain still checks (7,1) loud); no grammar
change, no engine change. The flip commit lands the §13 table, the
ledger row, the p5-zone-g gate and the 36/37->37/37 test-pin
re-baseline together (the §5 cross-artifact rule, the D28
fingerprint discipline). Zone-G-distinct facts: (a) the
census-pinned NON-SQUARE mission — TOT 100x25 @ 40004 B
(4+16·w·h), re-verified first-hand, the only non-100x100 shipped
map besides zone A's 25x75; (b) ONE mission staged through the
CAMPAIGN episode-slot seam alone (stage 7 / mask 0); (c) like F,
NO mission-number variant bank AT ALL — the only zone-G .BIN is
the zone-level MISSIONG.BIN (2443943 B) — so the D184 no-swap
verdict needs no variant caveat and the zone-level fetch-chain
assert (criterion 2's EDITOR\ZONEG\MISSIONG.* pin) is the whole
G3 leg; (d) M1's .NME is a REAL 1144 B bank, not the 16-byte MP
empty; (e) linear = clamp(5·5+m−1, 1, 26) = m+24, M1 = 25 — ONE
BELOW the clamp ceiling, no bite. Evidence: the generated battery
IS the whole criterion-1 leg — P5GM1A/B/C all 3 flows full
declared budgets (3/121/49 records), dumps verify, two-run byte
identity; NO engine gap surfaced on the ZONEG mission (the
stopping condition did not trigger); the anchor statics
re-derived from the TOT header + the §7j.64 formula; the T1 spot
table (FULL_MASK arithmetic, start_score, the 25-name fetch
chain, the seam domains at stage 7); criterion 6 stays FILE-LEVEL
(the shipped slot-0 campaign IS ZONEB/MISSION1). P5-ZONE-GATES
§13 documents the per-criterion table; the ledger flips
ZONEG-MISSION1 green (catalog_refs = []); the p5-zone-g gate joins
P5 required_gates (16 gates); the ledger test pin re-baselined
36/37 to 37/37 + the ZONEG 1/1 line (deliberate, same commit).
Verified: zone_mission_parity 5/5 (six zones, 27.43s),
bedlam-game suites green (canonical_dump_gate 13/13 — ZERO
canonical chain movement; differ_gate 4/4; determinism; census
37/37 unchanged), bedlam-core green, fmt + clippy clean,
check-p5-zone-ledger OK 37/37 + hermetic suite 18/18,
gates-validator suite 22/22 + the HEAD-bound battery all-green at
the flip commit (16 gates), MANIFEST clean before AND after every
corpus read, no Ghidra run. P5's mission side is DONE: what
remains of P5 is its PHASE-CLOSE disposition only (the P4
pattern: the required-gates.toml P5 status flip pending->green +
the bound phase verdict).

## D200 — 2026-08-28: P6/gate `p6-modernization-scaffold` — the ModeConfig seam + the bug-triage rubric + the original-behavior catalog format decided (docs/P6-MODERNIZATION.md; catalog schema p6-behavior-catalog-v1, seeds EMPTY) + the fail-closed checker wired as the FIRST P6 required gate

Context: PLAN §6 P6 (Modernization — default = modern; classic
available). The P5 phase close (f608207) queued this opener per the
D175 pattern: the machine-checkable contract lands BEFORE any
behavior change it grades. Bounds: no engine change, no harness
change, no Ghidra run; decisions + contract artifacts only.

1. THE MODECONFIG SEAM (the decision; PLAN §6 verbatim anchored in
   docs/P6-MODERNIZATION.md §1): fixes land directly in the engine —
   no bug-complete-faithful core to preserve (the 99% target
   simplification); classic mode shrinks to a small purist toggle set
   covering FEEL-CONTESTED items only (timing lock, control scheme,
   selected original-behavior catalog entries classified for
   preservation by the deterministic rubric of point 2 with a
   decision record, with regression tests); mode is ONE immutable
   ModeConfig injected at sim construction (never mutated mid-run —
   a mode change is a new sim); the test surface is the purist
   toggles, never the full feature cross-product. Binding
   consequence recorded with it: ModeConfig covers
   sim-behavior-affecting choices only — presentation/platform
   options (window mode, vsync, resolution, scaling, HD pack,
   refresh rate) are NOT mode toggles; display rate never enters the
   sim (Determinism Charter, PLAN §3/§6). The seam is DECIDED here;
   no engine code lands this unit (the first P6 engine unit
   implements ModeConfig + the toggle plumbing, behind this gate).

2. THE RUBRIC (PLAN §6 verbatim anchored in P6-MODERNIZATION.md §2):
   per catalog entry — crash/data-loss → fix everywhere;
   gameplay-coupled → classic preserves / modern fixes; cosmetic →
   fix in modern. Fixed = deviation from the catalog established by
   mechanically applying the rubric and recording regression
   evidence — not vibes. Encoded AS CODE: class → terminal
   disposition (closed-fix-everywhere / closed-preserve-classic /
   closed-fix-modern); the checker rejects every other closure pair.

3. THE CATALOG FORMAT (docs/P6-BEHAVIOR-CATALOG.toml, schema
   p6-behavior-catalog-v1, spec P6-MODERNIZATION.md §3): [[entry]]
   rows with id (unique, whitespace-free — the P5 ledger
   catalog_refs target), title, class, observed ∈ {original,
   divergence} (provenance: how the behavior surfaced), repro
   (non-empty deterministic evidence pointer), missions (non-empty,
   duplicate-free, ⊆ the P5 ledger mission ids — "affected
   missions"), disposition ∈ {open, closed-fix-everywhere,
   closed-fix-modern, closed-preserve-classic}, evidence (non-empty
   IFF closed — the regression-evidence anchor; a
   closed-preserve-classic closure's evidence must cover BOTH arms:
   the modern fix and the classic preservation through its toggle),
   purist_toggle (iff closed-preserve-classic; unique across the
   catalog — the ModeConfig toggle id), provenance (DECISIONS/RE
   anchor + confidence tag). R1-R7 mechanical rules in
   P6-MODERNIZATION.md §3.

4. THE SEEDING POLICY: the catalog seeds EMPTY — all 37 ledger
   missions closed green with catalog_refs = [] (machine-verified by
   the P5 checker), i.e. P5 recorded zero divergences and zero
   repro'd original-behavior observations worth classifying; the
   empty catalog is the honest post-P5 state (the D175 "0/37 is the
   honest scaffold state" principle). Entries land ONLY on recorded
   evidence with a repro: observed = "original" (oracle run,
   8street navigation re-anchored to EXW/EXD per repo policy, or
   RE-verified mechanism with doc anchor + confidence) OR observed =
   "divergence" (a repro'd engine divergence found during P6+ work).
   BOTH classes are accepted — after P5 parity our engine
   faithfully reproduces original behaviors, so an original bug
   surfaces as NO divergence at all; a divergence-only policy would
   starve the catalog of exactly the feel-contested items classic
   mode exists to preserve (expected dominant source: "original").
   Speculative or retrospective seeding (forum posts, unanchored
   memory, "probably a bug") is forbidden.

5. THE CHECKER + GATE: tools/check-p6-behavior-catalog.py +
   30-case hermetic suite tools/test-p6-behavior-catalog.py, all
   fail-closed (rubric-as-code incl. all six wrong-class closures;
   evidence discipline both directions; toggle discipline incl.
   duplicate/whitespace/wrong-disposition; mission grounding; the
   BIDIRECTIONAL catalog_refs join — every ledger ref resolves to an
   entry, every entry mission is a ledger id; manifest rules: a
   non-empty P6 required_gates starts with p6-modernization-scaffold
   AND defines it, P6 status green requires zero open entries).
   Real-repo pins: entries 0, 37 ledger ids, 0 refs resolve — move
   only with a deliberate catalog change, same commit. LAYERING (one
   source of truth per fact): ledger schema/corpus binding = the P5
   checker's job; mission identity here = the ledger; this checker
   reads ONLY committed docs — no corpus read at all (game-data
   never appears in tracked_paths/corpus). Gate wiring: P6
   required_gates = ["p6-modernization-scaffold"] (the FIRST entry;
   R6 keeps it first as more P6 gates land), commands = checker +
   suite, tracked_paths = the five artifacts, no corpus key, no
   writable (suite fixtures under the validator scratch HOME). The
   gate grades the CONTRACT, not phase completion: green from the
   moment it lands; P6 status stays pending.

6. VERIFIED THIS RUN: checker OK on the real repo (entries 0, 37
   ledger ids, 0 refs resolve); suite 30/30; gates-validator suite
   22/22; check-p5-zone-ledger OK 37/37 with the edited manifest (P5
   cross-artifact rules unaffected); manifest TOML re-parsed (8
   phase rows, P6 pending with exactly one required gate, 17 gates,
   last = p6-modernization-scaffold); controls BEFORE the change:
   zone_mission_parity 5/5 (27.33s) + canonical_dump_gate 13/13 at
   clean HEAD 0c81387; MANIFEST clean before AND after every corpus
   read; no Ghidra run; no engine change; no harness change.
   (worker 6e45232f claim 1, unit bedlam-nudge-item1-6e45232f)

## D201 — 2026-08-28: P6/engine `p6-modeconfig-seam` — the ONE immutable ModeConfig implemented and injected at sim construction (default = modern; the two plan-named purist axes `timing-lock`/`control-scheme`), config-not-state layering, and the completion gate wired as the SECOND P6 required gate

Context: PLAN §6 P6 + D200 (the scaffold contract, e0bc7fb) — the
FIRST engine unit behind the p6-modernization-scaffold gate. Bounds
kept: no canonical-chain movement (a modern default must leave the
canonical S0-S8 chains byte-identical or the seam is wrong), no
harness change, no Ghidra run.

1. THE SEAM, IMPLEMENTED (bedlam-core/src/mode.rs — new module):
   `ModeConfig` is a private-field Copy struct with NO `&mut self`
   method anywhere; the only way to a different mode is the
   consuming `with(axis, arm)` builder returning a NEW value, so
   immutability is the type's shape, not a convention. It rides
   `SimConfig.mode` into `Sim::new` (sim construction), is carried
   privately by `Sim`, and is read-only everywhere: `Sim::mode()`,
   `SimDriver::mode()` (forwarding), `GameHost::mode()`. No setter
   at any layer — a mode change is a new sim (new SimConfig ->
   new Sim/driver/host), exactly D200's binding consequence.
   Default = `ModeConfig::MODERN` (PLAN §6 "default = modern");
   `ModeConfig::CLASSIC` is the all-purist preset; per-axis mixing
   composes through `with`.

2. THE INITIAL PURIST TOGGLE SET (the two plan-named FEEL-CONTESTED
   axes only): `PuristToggle::TimingLock` with concrete id
   "timing-lock" and `PuristToggle::ControlScheme` with concrete id
   "control-scheme" — the ids D200/P6-MODERNIZATION §1 deferred to
   "the first P6 engine unit that implements the seam" are now
   pinned. Id rules mirror catalog checker R3 (non-empty,
   whitespace-free, unique across the set; `from_id` fails closed).
   RESERVED NAMESPACE: the catalog's future purist_toggle ids must
   not collide with these two plan-named ids (checker-side
   enforcement lands with the first catalog entry). The set grows
   ONLY through rubric-classified closed-preserve-classic catalog
   entries — never ad hoc. The CATALOG ITSELF STAYS EMPTY (D200
   seeding policy: entries land only on recorded evidence with a
   repro; none exists yet, and this unit adds none).

3. CONFIG-NOT-STATE LAYERING (the determinism decision): ModeConfig
   is config like the seed and time base — deliberately NOT part of
   `Sim::state_hash` and NOT serialized into snapshots/replays
   (FORMAT_VERSION stays 1; STATE_LEN and every pinned hash
   unchanged — the P5 hash fixtures and canonical chains are
   byte-stable by construction). A restore ADOPTS the mode of the
   SimConfig it is restored under (restoring IS constructing a new
   sim). Rationale: the two initial axes are host-side policies —
   timing lock selects PACING POLICY (frame-locked classic vs the
   modern accumulator), control scheme selects INPUT MAPPING —
   neither has an in-sim consumer, so a replay's trajectory is
   arm-independent today. When a later unit gives an axis or a
   catalog toggle an in-sim consumer, THAT unit decides whether the
   replay/snapshot headers start recording the mode (with a
   FORMAT_VERSION bump then, not now). Pin: the seam unit itself is
   INERT — same seed + input stream yields the identical hashed
   trajectory in both arms (test mode_is_config_not_state_the_seam_
   lands_inert); the canonical chains are pinned by that inertness
   plus canonical_dump_gate 13/13.

4. PRESENTATION STAYS OUTSIDE (Determinism Charter): ModeConfig
   carries no Hz, no resolution, no vsync/window/scaling knob of any
   kind; display rate never enters the sim in any arm. The
   timing-lock axis is a pacing-policy selector, not a rate input.

5. GATE WIRING: `p6-modeconfig-seam` is the SECOND P6 required gate
   (required_gates = [p6-modernization-scaffold, p6-modeconfig-
   seam]; R6 keeps the scaffold first). Commands = bedlam-core --lib
   (the mode/sim/frame seam suite) + bedlam-game --lib (the host
   read-point suite), both `--release --locked --offline`, both
   hermetic (no corpus key, no writable). Test surface per D200:
   the purist toggles, both arms, never the feature cross-product.

6. VERIFIED THIS RUN: bedlam-core --lib 147/0 (was 146: +4 mode
   tests + 3 sim seam tests net of none), bedlam-game --lib 142/0
   (+1 host seam test), bedlam-core determinism 12/0 + hash_fixture
   (pinned constants untouched), bedlam-render determinism green;
   fmt clean + clippy clean on every touched file (the 7 pre-existing
   bedlam-core warnings from D151 untouched); controls BEFORE the
   change at clean HEAD b625559: zone_mission_parity 5/5 +
   canonical_dump_gate + determinism all rc=0 (27.42s parity); the
   same battery green AFTER the seam; gates-validator suite 22/22;
   check-p6-behavior-catalog OK (catalog still empty, R6 satisfied
   with the second gate); MANIFEST clean before AND after every
   corpus read; no Ghidra run.
   (worker 21604df0 claim 1, unit p6-modeconfig-seam)

## D202 — 2026-08-28: autonomy/watchdog — the THIRD invalid-queue-rewrite recurrence (a MISSING `[gate=]` tag on a queued successor); the step-7 rewrite is not finished until the strict parser accepts it in the authoring run itself

Context: watchdog repair token 1187807 (queue INVALID-DEADLOCKED,
parser rc=2, controller refusing idle/spawn). Worker 21604df0
finished `p6-modeconfig-seam` completely (9d39368, PUSHED) and then
rewrote the queue per AGENTS.md step 7 — but queued item 2
(`p6-control-scheme-surface`) carried only `[id=…]`, no
`[gate=…]`. One missing tag, two machine consequences:

1. THE KILL AT THE FINISH LINE: nudge-agent.sh
   boundary_completion_rewrite() requires the strict parser to
   validate the rewritten file before granting the completion
   grace; an unparseable rewrite is classified as a corrupt
   mutation, so the wrapper terminated the completer 117s after
   its own commit and recorded a structured preflight-mismatch /
   launch-boundary failure — the SECOND time a completer died for
   a wrapper-side reason, but this time the fault was genuinely in
   the artifact (the rewrite), not the window.

2. THE DEADLOCK: the controller's next tick parsed the still-
   invalid queue (rc=2, "item 2: missing required gate metadata"),
   declared INVALID-DEADLOCKED, and refused to spawn — with the
   queue unparseable, no worker can ever run to fix it; only a
   watchdog repair can break the cycle. D177 (wrapped tag) and
   D180 (rule restated) did not remove the failure class because
   the rule is enforced only at the wrapper's preflight, AFTER the
   authoring run ended.

DECISION (binding on every future step-7 rewrite): a queue rewrite
is not finished when the words are down — it is finished when
`python3 tools/nudge-free-items.py .state/NEXT.md .state/claims
--state-v1` exits 0 in the SAME run, before the bookkeeping
commit. Every queued item carries BOTH `[id=…]` AND `[gate=…]`
whole on its first numbered line (an id tag alone is invalid);
prose never uses square brackets (D180). Parser acceptance is part
of authoring, exactly like fmt/clippy for Rust.

REPAIR (this commit): inserted the one missing
`[gate=p6-control-scheme-surface]` tag into the killed worker's
item 2, reflowed only that block (every word preserved; the split
identifier `p6-control-scheme-/surface` re-joined), left the
worker's rewrite otherwise verbatim, completed its stranded step-8
STATE.md bookkeeping, and acked the structured failure
(replaced-task: `(1, p6-modeconfig-seam, p6-modeconfig-seam)` is
absent from the resulting queue, succeeded by the two D201
axis-consumer units). The queue again parses rc=0 RUNNABLE 1 2.
No engine, harness, or corpus change; no canonical-chain
movement; no Ghidra run.
(watchdog repair 1187807; unit p6-modeconfig-seam stands at
9d39368 by worker 21604df0)

## D203 — 2026-08-28: P6/engine `p6-timing-lock-surface` — the timing-lock purist axis's FIRST REAL CONSUMER: the present pacing policy selected from the immutable mode at the HOST/PRESENT seam (modern = accumulator-driven decoupled present, classic = the original frame-locked present-coupled pacing), and the completion gate wired as the THIRD P6 required gate

Context: PLAN §6 P6 "time-based simulation" + D200 (scaffold) + D201
(the inert ModeConfig seam at 9d39368). Bounds kept: the consumer
lands at the HOST/PRESENT seam only; no canonical-chain movement
(canonical_dump_gate 13/13 + zone_mission_parity 5/5 + determinism
green BEFORE (c942bd9) and AFTER; the modern default stays
byte-identical); no harness change; no Ghidra run; the catalog stays
EMPTY (a plan-named axis unit is not a catalog entry — D200 seeding
policy).

1. THE CONSUMER (engine/bedlam-game/src/host.rs):
   `GameHost::present_pacing()` reads the timing-lock arm of the
   IMMUTABLE mode and maps it to a `PresentPacing` policy — MODERN =
   `Decoupled`: every host frame is presentable (zero-tick
   high-refresh frames included; the accumulator-driven present the
   PLAN §6 high-refresh requirement names and the shell clock
   bedlam-shell/src/clock.rs feeds); CLASSIC = `FrameLocked`: the
   original frame-locked present-coupled pacing [verified,
   RE-EXW-PACER §3 / D16: the FUN_0043d00b loop pass and its
   PresentEnd are ONE event, g_frame_count++ exactly once per flip,
   no software frame clock] — a host frame is presentable only when
   it executed >= 1 logic tick. `GameHost::should_present()` is the
   per-host-frame gate the platform's present loop asks; before the
   first pump the pre-rendered boot frame is presentable in BOTH
   arms (the platform must blit once to have anything on screen).
   On the original 60 Hz display class the classic lock presents
   every flip (indistinguishable from the original); on faster
   hosts the VISIBLE refresh follows the fixed tick, never the
   display — the purist cadence without a blocking loop.

2. A POLICY, NEVER A HZ (Determinism Charter): the logic tick stays
   FIXED at the original rate in BOTH arms; display rate never
   enters the sim or the state hash. The decision rides the
   un-hashed presentation bucket ONLY: a private `last_pump_ticks:
   Option<u32>` (recorded by pump_frame before anything else) feeds
   the gate — it cannot reach the sim, the state hash or the scene
   hash. The D17 accumulator is pacing-policy-neutral in every arm
   (documented on SimDriver). Pin:
   `timing_lock_pacing_never_touches_the_hashed_buckets` — the same
   pump script (dt sequence + inputs, tick-carrying and zero-tick
   frames mixed) yields the IDENTICAL executed-tick sequence, sim
   tick count, sim state hash and scene hash in both arms, while
   should_present differs somewhere (the consumer is real, not
   inert). CONFIG-NOT-STATE unchanged: FORMAT_VERSION stays 1, no
   hash pin moves.

3. TEST SURFACE (per D200): the ONE purist toggle, both arms —
   selection (incl. the control-scheme axis as the
   axis-independence control, never a cross-product sweep), the
   modern high-refresh shape (240x dt=1 pumps: 60 ticks, 240
   presents), the classic lock (same script: 60 presents, gate =
   executed>0; boot presentable), the classic 60 Hz shape
   (dt=4: present every flip; a banked 3-sub-tick frame is the only
   no-present shape), and the hash-inertness pin above.

4. GATE WIRING: `p6-timing-lock-surface` is the THIRD P6 required
   gate (required_gates = [scaffold, modeconfig-seam,
   timing-lock-surface]; R6 keeps the scaffold first). Commands =
   bedlam-game --lib (the host present-seam suite) + bedlam-core
   --lib (the mode/frame policy-neutral docs-bearing modules), both
   --release --locked --offline, hermetic (no corpus key, no
   writable).

5. SCOPE NOTE: the platform loop wiring (the window shell consuming
   should_present, mode plumbing through the shell config) is a
   LATER P6 unit — this unit lands the seam-side policy, its
   contract and its pins.

6. VERIFIED THIS RUN: bedlam-game --lib 147/0 (+5 pacing tests),
   bedlam-core --lib 147/0; controls green BEFORE at clean HEAD
   c942bd9 AND AFTER: zone_mission_parity 5/5 + canonical_dump_gate
   13/13 + determinism 4/4; check-p6-behavior-catalog OK (catalog
   still empty, R6 satisfied with the third gate) + its suite
   30/30; gates-validator suite 22/22; fmt + clippy clean on the
   touched crates; MANIFEST clean before AND after every corpus
   read; no Ghidra run.
   (worker 458a7e98 claim 1, unit p6-timing-lock-surface)
