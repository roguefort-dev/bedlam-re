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
