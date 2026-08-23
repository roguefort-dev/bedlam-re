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

## D108 — 2026-08-22: P4.2/W12-S5 — the S5/S5B pickup scenarios LANDED (grammar v1.5 `zone`/`pickup` keys + the episode-slot host seam + the zone-row O1 normalizer). FOUR decisions recorded: (1) ZONE STAGING = the D51 host seam `GameHost::stage_episode_slot(stage, mask)` standing in for the campaign-advance (0x41c9e5) / save-load-restore (0x43c2b8) shells the engine does not model; the grammar key is `zone = "B"` (letter A..G → stage, mask 0 → MISSION1, linear stays the fresh-slot 0 — a live campaign-walk O1 session carries its own linear/mission counters, so the plan records the zone seam and the linear diff is the live-capture seam, never an E fabrication). (2) THE TWO-SCENARIO SPLIT: cases 1 and 3 are never co-walkable (nearest pair z3 (26,21)↔(76,10) = 61 octagonal tiles; one order's claims all lie within ORDER_RADIUS 0xC0 of the order tile), and two sequential orders need the first order CLEARED — all-alive-state-3 is impossible while the next leg's robots stand idle (state-3 robots can never re-claim, verified EXW subset), so the only in-model path is the 0x197-frame window expiry whose ~407 idle frames × ~340 KB/record of REAL mirror rows (every ZONEB tile is active: 15,102 words + 52,715 seen) is not a shippable dump — S5 (c1/c2/c4 row-21 trio) + S5B (c3+c4 row-10 five) cover cases 1..4 in 16+19 records. (3) DAT-BYTE VISIBILITY ANSWERED: the consume's DAT := 0 needs NO dedicated differ row — the mirror word (:= table-C floor)/seen carry the pickup observation, the walkability change rides the robot-bank rows, and the raw DAT volume is not a guest span any watch row carries. (4) THE ZONE-ROW CONVENTION: E's canonical zone is the 0-based mission-slot index while the guest cell (EXW 0x4edd8c / EXD 0x107500) is the 1-based set (D99) — the O1 normalizer maps cell−1 (S0..S4 chains unaffected; first exercised by S5/S5B). Staging order pinned: destroy family → stage_pickup_surface (the engine load-order note) → the §7j.12/6 hazard stamper (30 ZONEB grid cells: 24× 0x7d2 + 6× 0x7d3), matching the original mission-load order. Case-3 observability note: the walker spawns at the hp clamp ceiling (5000), so the case-3 +2500 body is value-invisible — the consume/dispatch still ride the rows; a pre-damaged-walker variant is the live follow-up (worker c2aba48b claim 2)

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
