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
