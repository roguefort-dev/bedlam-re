# Pinned runtimes (DOSBox-X, Wine) - P4

Provenance: created 2026-08-18 by the P4-prep run; harness section added the
same day by the P4 runtime unit (D29). Purpose: PLAN sec P4 items 2-3
(differential harness via DOSBox-X debugger; golden pipeline with pinned
dosbox/wine versions + configs). Everything lives under repo-local gitignored
runtime/ so AGENTS.md rule "never modify files outside the repo" holds and pins
are reproducible on a fresh clone of the tooling (runtime contents themselves
are re-downloadable artifacts, never committed).

## DOSBox-X (differential harness target)

- Channel: flathub com.dosbox_x.DOSBox-X, user install with
  XDG_DATA_HOME=<repo>/runtime/xdg (whole user flatpak lives inside the repo).
- PIN: version 2026.08.02, flathub commit
  fa89039ca01aca36d9031f287d69b885d7510fb24499e9c33e1db420ab6ccdb2
  (2026-08-15, runtime org.freedesktop.Platform 25.08).
- Why flathub and not an AppImage: upstream GitHub releases ship NO Linux
  binaries at all - verified 2026-08-18 across the last 6 tags (2026.06.02 ..
  2026.08.02 + osfree twins): assets are Windows/macOS/hx-dos only. Flathub is
  the official Linux channel (linked from dosbox-x.com) and is current with
  upstream (2026.08.02 = latest tag). The queue item said AppImage; that
  channel does not exist anymore, decision recorded as D19.
- ~~Debugger presence (harness requirement): the shipped binary carries the
  integrated debugger (strings: INT-3 auto-breakpoint config text, BP-style
  commands)~~ SUPERSEDED 2026-08-22 — see "DH-G0 channel audit" below: the
  strings were a config help text ("If set, a breakpoint on INT 3 is
  automatically set up at startup" — an unrelated option's description) plus
  coincidental junk ("BP A" inside other strings). The pinned flathub build
  has NO integrated debugger. Headless smoke --version exits 0.
- Wrapper: tools/runtime/dosbox-x.sh (sets XDG_DATA_HOME, exec flatpak run).
- Upgrade policy: NEVER update blindly. A new pin is a deliberate decision:
  install to a NEW commit, smoke-test, re-baseline goldens, update this file.

## DOSBox-X harness sandbox + config (D29, 2026-08-18)

Target: game-data-2 = the B2 DOS build (BEDLAM.EXE + DOS4GW.EXE, LE image).

SANDBOX MODEL (the load-bearing fact): the flatpak STATIC FINISH ARG grants
filesystems=home - the whole home directory rw. Per-path :ro override grants
are therefore ILLUSORY (permissions union; most permissive wins). Correct
posture, applied and verified via flatpak info --show-permissions:

  flatpak override --user --reset com.dosbox_x.DOSBox-X
  flatpak override --user --nofilesystem=home --filesystem=<repo>/runtime com.dosbox_x.DOSBox-X

Effective: home revoked, ONLY runtime/ visible. game-data is INVISIBLE to
the emulator - write isolation by construction. Consequences:
- The corpus is reached via an rsync scratch copy: runtime/harness-corpus
  (writable C: so the game can save; the canon corpus is never mounted).
- tools/ is invisible too: the driver deploys the conf copy to
  runtime/harness-out/run.conf with the mounts appended.
- Output dir runtime/harness-out is D: (dumps, captures, saves, logs).

CONFIG PIN: tools/runtime/dosbox-x-harness.conf is the canon; the driver
prepares the run copy. Pins + rationale:
- machine=svga_s3: VESA VBE for banked mode 0x101 (B2 pages {0,5}, census
  7.7d); UNIVBE.EXE must NOT be run inside the sandbox (svga_s3 supplies VBE).
- core=normal + cputype=pentium: interpreter core for watchpoint accuracy
  (dynamic recompilers make traps unreliable) + the most reproducible core.
- cycles=fixed 60000: D29 STARTING PIN (approx Pentium-100 class). Calibration
  (audio dropouts?) happens at the first interactive run; any change is a
  deliberate pin change + golden re-baseline per D19.
- memsize=16, vmemsize=2 ([video] section - the canonical home in 2026.08.02;
  vesa 0x101 dual page = 2 x 300KB + cursor block).
- render scaler=none aspect=false: raw framebuffer for pixel goldens.
- mixer sample accurate=true rate=48000 + sblaster sb16 220/7/1/5: the class
  the B2 HMI driver set (HMIDET/HMIDRV/HMIMDRV.386) probes; single
  reproducible host resample of the 11025 Hz native stream.
- log debuggerrun=watch: the integrated debugger in watch mode (game runs
  free, watches report without freezing emulation).

DRIVER: tools/runtime/dosbox-harness.sh {prepare|smoke|shell|game}.
- smoke = headless validation with a FILE gate (SDL dummy A/V, -exit,
  -time-limit 90, dir c: > D:SMOKETST.TXT). GATE = SMOKETST.TXT lists
  both BEDLAM.EXE (672399 B) and DOS4GW.EXE (265396 B). Verified PASS
  2026-08-18 (first-hand + a dead-sibling run that used the same driver).
- game = the actual BEDLAM.EXE launch: INTERACTIVE-GATED (desktop + debugger
  session); unattended runs must not use it.

WATCH PLAN: tools/runtime/dosbox-watch.skeleton.txt pins the B2 watch set
(census-verified addresses incl. the RNG pairs 0x11ef18/1a + 0x11ef1c/1e),
the PresentFlip@0x1066b frame trigger, the PcmMixerService@0x136e0 audio
dump, and the calibration checklist. Debugger command names (BPINT/BPLM/D)
and the startup.js route get verified at the first interactive session.

## DH-G0 channel audit (2026-08-22, W4; all facts [verified] on THIS pin)

Method: binary strings + reference conf shipped in the flatpak, upstream
source at the binary's own banner commit (e522642b8c86d87cd4e58ffb2961fa30608c119a;
note the flathub manifest.json names 784240ad as the git source — the banner
commit is the ground truth for code), plus headless behavioral probes
(DOS shell only, no game launch, SDL dummy A/V, sandbox-visible runtime/
paths; probe dir runtime/harness-out/dbgprobe/).

1. NO INTEGRATED DEBUGGER in this pin. configure.ac gates the debugger
   behind --enable-debug (default OFF: "Debugger not enabled"); the flathub
   manifest builds with --enable-sdl2 only. Behavioral: `debuggerrun =
   debugger` and `-break-start` both parse and are INERT (three probes:
   piped stdin, PTY, plain — boot runs straight through, no debugger
   console, no break). The BP/BPLM/BPINT/MEMDUMP command table is absent
   from the binary. Consequence: the D29/D77 "watch-mode debugger" O1
   instrument DOES NOT EXIST in this runtime; BPINT/BPLM/D names are
   UNPINNABLE here (the skeleton's UNCERTAINs resolve negative).
2. Duktape ECMAScript IS compiled in and runs [script] startup.js once at
   boot (before the machine loop). Enumerated API (behavioral probe,
   Object.getOwnPropertyNames): `_emu = {emulator:"DOSBox-X", version,
   log(fn), _js{...}}`, `console.log` (same function), plus Node-ish
   Buffer/CBOR polyfills with NO I/O attached. There is NO memory access,
   NO callback/per-frame hook, NO file API. JS scripting is log-only and
   cannot be the dump instrument.
3. LOG CHANNEL GATE: console.log/LOG(LOG_MISC,*) output is invisible
   unless `[log] misc = true` (the [log] advanced channel list). This bit
   the original probes; any future JS-side diagnostics must enable it.
4. GameLink (GC4 shared-memory IPC, src/gamelink/) IS compiled in (config
   keys "gamelink master/snoop/load address" + output_gamelink present).
   It is client-driven polling designed for real-mode games; whether it
   can read DPMI/flat linear addresses (LeLoader EXD objects 0x10000 /
   0x80000) is an OPEN feasibility question for the channel re-pin.

IMPLICATION for DH-G0/O1: the trigger surface must be re-pinned before any
live debugger automation. Options (decision pending, not made here):
(a) self-build DOSBox-X at a pinned commit with --enable-debug=heavy
    inside runtime/ (keeps every D29 conf pin; one-time deliberate pin
    change per D19 discipline + smoke + manifest bracketing);
(b) GameLink feasibility study for linear-address reads (open question);
(c) escape hatch per DESIGN §11: promote the O2 ptrace channel (W11)
    to primary instrument.
The W4 runner ships unattended-safe staging + the channel-agnostic capture
transcript format + the stitcher (see DESIGN §3/§10-W4); the live-run piece
is [BLOCKED]-on-DH-G0-channel-repin.

## DH-G0 channel re-pin (2026-08-22, queue item 1) — DECISION: option (a), self-build at e522642 (D80)

DECISION (the queue item's stated DEFAULT, no operator input): the O1
capture channel becomes a REPO-LOCAL SELF-BUILT DOSBox-X at the SAME
upstream commit as the flathub pin's banner — e522642b8c86d87cd4e58ffb2961fa30608c119a
— configured `--enable-sdl2 --enable-debug=heavy`. Rationale: the watch
skeleton's entire command surface (BPINT/BPLM/D) exists in this tree
(source-pinned below); the flathub runtime stays installed as the
D29-proven sandbox baseline; conf pins carry over unchanged; GameLink
(option b) is client-poll real-mode oriented with an unproven DPMI/flat
linear model and would still need a host client + probing, and O2-ptrace
(option c) abandons the DOS-side oracle entirely. Recorded as D80.

BUILD LAYOUT (everything under gitignored runtime/, nothing committed):
- runtime/dosbox-x-src/   = upstream checkout at e522642 (shallow fetch,
  verified `git rev-parse HEAD`); source is READ-ONLY reference for the
  facts below — never patched (oracle rule: observation only).
- runtime/dosbox-x-build/ = autotools out-of-tree build (autogen.sh run
  FROM THE SOURCE DIR — running it from the build dir fails: aclocal
  needs configure.ac CWD), configured
  `../dosbox-x-src/configure --enable-sdl2 --enable-debug=heavy
  --disable-sdlnet --disable-avcodec`
  (sdlnet: host lacks SDL2_net headers — only modem/IPX need it;
  avcodec: host ffmpeg 8 removed AVCodec::sample_fmts, upstream code
  at e522642 does not compile against it — capture-to-video only;
  neither feature touches the harness),
  config.h verified: `#define C_DEBUG 1` + `#define C_HEAVY_DEBUG 1`.
  Built binary sha256 24f71092885df7ebd6ebc92c7cbf0edf…, banner
  "DOSBox-X version 2026.07.02 SDL2" (= the e522642 banner commit;
  the flathub 2026.08.02 release shipped this same banner).
- Host toolchain recorded as part of the pin: gcc 16.2.1, SDL2 2.32.70
  (pkg-config), ncursesw 6.6.20251230, autoconf 2.73, automake 1.18.1,
  libtool 2.6.2, gnu make 4.4.1, 32-way build.
- configure gate [source-pinned, configure.ac:1144-1156]:
  --enable-debug requires curses (else AC_MSG_ERROR) and defines C_DEBUG;
  `=heavy` additionally defines C_HEAVY_DEBUG. The flathub build passed
  only --enable-sdl2 → no debugger (matches the D79 audit).

DEBUGGER COMMAND SURFACE [source-pinned at e522642, src/debug/debug.cpp
unless noted; BEHAVIORALLY VERIFIED 2026-08-22 on this build via the
dbgprobe channel probes — see "D80 channel probe results" below]:
- ENTRY: `-break-start` (sdlmain.cpp:7517,10152 → DEBUG_EnableDebugger)
  or conf `debuggerrun` (debug_gui.cpp:933-939): "debugger"→0 (sit at
  prompt), "normal"→1 (auto-RUN on entry), "watch"→2 (auto-RUNWATCH on
  entry) — applied at debug.cpp:5100-5101. [verified: mode 0 via
  -break-start]
- PTY GATE [source-pinned, debug.cpp:5042-5064]: on Linux the debugger
  REFUSES to open unless isatty(0)&&isatty(1)&&isatty(2). Automation
  therefore runs the binary under a host PTY (python3 pty). The D79
  "inert debuggerrun/-break-start" probes on the flathub build were
  never gated by this — that build simply lacked C_DEBUG. [verified:
  PTY session opens the debugger, banner + prompt live]
- RUNWATCH (debug.cpp:2668): run with breakpoints ACTIVE; on a bp hit
  the loop re-enters the debugger prompt (the watch-mode shape D29
  assumed). RUN = plain resume. [verified: resume→hit→prompt→resume
  cycles over multiple frames]
- FRAME TRIGGER PRIMITIVES:
  - BP [seg]:[off] — code breakpoint (the present-tail site; live unit).
  - BPLM [linear] — LINEAR memory-change breakpoint (C_HEAVY_DEBUG
    only, i.e. our build). [verified: `BPLM 46C` armed ("Set linear
    memory breakpoint at 0000046C") and FIRED on the next write
    ("Memory breakpoint : 0000:046C - 00 -> AA"), stopping the machine]
  - BPINT [nr] [ah] [al] — interrupt breakpoint. [verified: `BPINT 8`
    as the probe hit surrogate across 3 frames]
- BULK READ (the dump primitive):
  - MEMDUMPBIN [seg]:[off] [len] → MEMDUMP.BIN raw bytes (fixed name,
    HOST CWD, "wb" = overwrite per call) — debug.cpp:2021-2027,6002-6020.
    [verified: 9/9 round-trips, correct lengths, rename-between-calls]
  - MEMDUMP [seg]:[off] [len] → MEMDUMP.TXT hex text (same fixed-name
    + CWD shape) — debug.cpp:2013-2019,5965-6000. [source-pinned only]
  The PTY driver renames the file between calls (the emitter side of
  the DBXCAP transcript).
- INJECTION PRIMITIVE (W5): SMV [linear] [val].. — set memory at a
  LINEAR address (debug.cpp, listed in HELP); SM [seg]:[off] segmented
  twin; SR reg val. [verified: `SMV 46C AA BB CC DD` → readback at
  0040:006C = aabbccdd; ack line "DEBUG: Memory changed (4 bytes)";
  linear==seg<<4 confirmed in real mode]
- ADDRESS MODEL [source-pinned, debug.cpp:460-479 GetAddress]:
  seg:off resolves — current-CS → SegPhys(cs)+off; pmode selector →
  descriptor base+off (LinMakeProt); real mode → seg<<4+off. Under
  DOS4GW the game's flat selectors have base 0, so `sel:linear` with
  the runtime flat selector (pinned via SELINFO/LDT at DH-G0) reads
  the LE objects at their linear addresses (0x10000/0x80000). The
  INT3-at-_entry proof (EXD entry 0x5fbb0) pins this conversion at the
  first interactive session, as the watch skeleton requires. [real-mode
  linear verified; the pmode/flat-selector path is the live unit's
  first checklist item]
- VIEWS: D seg:off / DV linear / DP physical; SELINFO, GDT/LDT/IDT,
  EMU MEM/MACHINE for pinning the selector facts. [source-pinned only]

D80 CHANNEL GOTCHAS (behaviorally pinned 2026-08-22 — the driver bakes
these in, do not "simplify" them away):
1. THE [log] LOGFILE GETS REWRITTEN at debugger init (truncate +
   re-emit): a seek/offset-based tailer anchors into the stale
   pre-rewrite copy and never sees acks (measured: cursor 2700 >
   filesize 2212 while the ack sat in the file). Ack matching MUST be
   COUNT-based over full reads (tools/runtime/dbx-capgen.py LogTail).
2. A PERMANENT PTY DRAIN IS MANDATORY: ncurses redraws several KB per
   update and the pty kernel buffer is ~64KB; once full, wrefresh()
   blocks and the whole debugger loop deadlocks (first command acks in
   ~40ms, then everything stalls until the buffer is read).
3. SETTLE 1.0s AFTER EACH ACK before the next send: input written
   ~0.01s after the previous ack sits in the tty queue for tens of
   seconds; input sent after a 1.0s gap acks in ~40ms. (Mechanism
   not isolated; empirically bisected across dbgprobe2 runs.)
4. Ack lines land in the [log] logfile (DEBUG_ShowMsg: fprintf+fflush
   per message, debug_gui.cpp:744) — the logfile is the ack channel,
   NOT the ncurses screen (redraws re-emit old pane text).

D80 channel probe results (2026-08-22, this host, this build; probe
transcript runtime/harness-out/dbgprobe2/capture.dbxcap — plumbing-only
ids, never stitched): 3 frames × 3 watches. Frame 1 (the -break-start
pre-boot halt): IVT/BIOS/BDA all zeros (machine parked at F000:FFF0).
Frame 2 (after RUNWATCH → POST → BPINT-8 hit): IVT vectors f000:ca60,
BDA shows COM1 0x3F8 / COM2 0x2F8 / LPT1 0x0378 — real BIOS state.
Frame 3: INT 1-3 vectors moved to 0070:000e (DOS kernel vectors took
over between timer ticks). Per-command cost ≈ 1.05s (the settle) —
S0-shape captures (≈13 watches × 3 frames) ≈ 45s.

Next in this unit: build the binary, behavioral smoke (headless SDL
dummy + PTY: -break-start enters the prompt, MEMDUMPBIN round-trips,
RUN/RUNWATCH parse) — converting the tags above to [verified] on THIS
self-built pin — then wire the DBXCAP emitter driver. NO game diff:
the live game run stays interactive-gated (FORCE_DIFF_RUN=1).

UNIT CLOSE-OUT (2026-08-22): build DONE (--disable-sdlnet
--disable-avcodec added: host lacks SDL2_net headers and ffmpeg 8
removed AVCodec::sample_fmts — both features are irrelevant to the
harness; binary 144MB, `--version` banner "2026.07.02 SDL2" = the
e522642 banner, sha256 24f71092885df7ebd6ebc92c7cbf0edf...). Behavioral
probe DONE (see "D80 channel probe results"). Emitter WIRED:
tools/runtime/dbx-capgen.py (PTY + count-based log acks + MEMDUMPBIN
slicing → DBXCAP) driven by `dosbox-harness.sh dbgprobe` (unattended,
no game) and `diff capture` (FORCE_DIFF_RUN=1, needs the staged
capture-plan.json — the DH-G1 live unit's deliverable). The live game
run + the pmode flat-selector proof (INT3 at EXD _entry 0x5fbb0, BP at
the present-tail 0x5a6eb) + cycles calibration remain INTERACTIVE-GATED
for the next unit (S0 live + DH-G1 determinism).

## S0 live channel mechanics (2026-08-22, queue item 1 prep; all facts
## [source-pinned] on the D80 build's tree at e522642 unless noted)

These facts retire the queue item's INT3/SELINFO-first ordering: the live
capture plan needs NO numeric selector parameter at all.

1. THE `CS:` REGISTER-NAME FORM ELIMINATES THE SELECTOR PARAM
   [source-pinned, debug.cpp:1547-1680 + 2013-2019 + 2540-2544]:
   `GetHexValue` (the argument parser MEMDUMPBIN/BP/BPM use) accepts
   REGISTER NAMES in its default path — `CS` resolves to
   `SegValue(cs)` (line 1638), `DS`/`ES`/… likewise, plus `+`/`-`
   expression tails. Therefore, at any stop INSIDE game code:
   - `MEMDUMPBIN CS:001195F0 4` → seg = SegValue(cs) = the game flat CS
     selector VALUE (numeric), and GetAddress(seg==SegValue(cs)) returns
     `SegPhys(cs)+offset` = cached base (0 under DOS4GW flat) + the EXD
     linear address. No SELINFO step, no hardcoded selector.
   - `BP CS:0005A6EB` arms the registry s0-trigger row; the ack line
     ("DEBUG: Set breakpoint at %04X:%04X", debug.cpp:2544) ECHOES the
     numeric selector — the flat-selector pin lands in the logfile for
     free, per run.
   - `GetAddress` current-CS path is limit-check-FREE (debug.cpp:470-472
     uses the cached base directly), so watch reads up to object2 top
     0x12583e cannot fail on a limit gate.
2. BP ARMING IS EAGER, BPLM IS LAZY — THE BOOT-TRAP ORDER FOLLOWS
   [source-pinned, debug.cpp:585 SetAddress → GetAddress at ARM time]:
   `BP <seg>:<off>` resolves its location WHEN ARMED. Armed at the
   pre-boot `-break-start` halt (real mode, GDT empty) a game BP
   mis-resolves (real-mode seg<<4) and never fires. `BPLM <linear>`
   (BKPNT_MEMORY_LINEAR) is the opposite: it stores only the linear
   offset and the value-compare happens per-instruction at CHECK time
   (debug.cpp:787-810) — so the live flow arms `BPLM 1195F0` (the EXD
   frame-counter cell, watches.toml row frame-counter) at the pre-boot
   halt; it fires on the first post-boot write to that cell (LeLoader
   object2 copy — 0x1195f0 sits inside object2 — and/or the first
   screen-loop INC; 14 INC sites exist, exd-probe2 census), giving a
   guaranteed stop with the machine in game context, where the real
   `BP CS:0005A6EB` is then armed.
3. SELINFO RIDES THE LOGFILE [source-pinned, debug.cpp:2861-2869]:
   SELINFO output goes through DEBUG_ShowMsg (the [log] logfile), 3
   lines: "SelectorInfo CS:", "CS: b:XXXXXXXX type:..", "    l:XXXXXXXX
   ..". GetLimit applies the granularity bit (cpu.h:436-441), so a flat
   4GB descriptor prints l:FFFFFFFF. capgen parses b:/l: as a RUNTIME
   GUARD: a stop is armable iff base==0 (limit>=0x12583e belt+braces).
   Stops in non-flat context (LeLoader stub, real mode) retry: BPDEL *,
   re-arm BPLM, RUNWATCH again (bounded retries).
4. `debuggerrun = watch` WOULD FREE-RUN [source-pinned, RUNTIME.md D80
   entry]: watch mode auto-RUNWATCHes at debugger entry — with no
   breakpoints yet the machine boots through the parked halt and queued
   PTY commands never execute (input is only processed at stops). The
   canon conf tools/runtime/dosbox-x-harness.conf pins watch mode (fine
   for its original purpose); `diff stage` therefore rewrites the STAGED
   copy (runtime/-only) to `debuggerrun = debugger` — a channel-mode
   flip, not a sim-pin change (cycles/machine/core/mixer untouched).
5. FRAME-COUNTER IS SESSION-LIFETIME — S0 ANCHOR NOISE EXPECTATION
   [verified, exd-probe2/probe8 census]: [0x1195f0] has NO reset store
   anywhere in the image — only INCs (mission tail 0x5a6f0-fd + 13 other
   screen-loop tails: FUN_0004c80c/0004f1d1/00050953/0005638d). The
   title/menu screens increment the SAME cell, so the counter value (and
   any menu-RNG churn) at mission start is OPERATOR-TIMING-DEPENDENT
   across interactive S0 runs. DESIGN §6 already classes frame counters
   T2-tolerant and RNG T3-statistical: the S0 double-run verdict is
   "identical chains modulo the frame-counter (+RNG) watch bytes";
   byte-identical chains need the W5 scripted menu walk (the DH-G1
   headless S1 form). The live checklist records this so a
   counter-only chain diff is NOT misread as a channel failure.
6. LIVE SESSION NEEDS REAL SDL VIDEO [derived]: capgen's default env is
   SDL dummy A/V (headless-safe), but dummy video has NO keyboard — the
   operator cannot walk the title menu. Plan `env` entries
   (SDL_VIDEODRIVER="" = unset → the desktop X server) flip this; audio
   stays dummy for capture runs (real audio only for the cycles
   calibration listen test).
7. THE ANCHOR-FRAME OFF-BY-A-TAIL [derived from 2]: the BPLM boot trap
   fires AFTER mission frame 1's present tail (the counter INC sits
   past the CALL at 0x5a6eb), so the armed BP's first hit = mission
   frame 2's dump point. capgen frame 1 = that hit; alignment is by the
   frame-counter watch value (DESIGN §2), so the one-frame shift is a
   recorded constant, not a divergence.

## S0 LIVE SESSION CHECKLIST (interactive; the machinery is landed + headless-verified — this is all that remains)

Everything below was prepared by the unattended units (commits f659db5
+ d5550a3 + ee2f0d4): capgen plan v2 (boot trap → flat guard → arm →
resolve → anchor/per-frame capture), dbx-plan (scenario + registry →
plan), the committed S0 plan artifact, the staged-conf channel flip.
The flow machinery itself is proven headless by `dbgprobe flow`
(BPLM 46C trap → arm → resolve com1=0x3f8 → expr rows = real IVT/BDA
bytes; no game, unattended-safe — safe to re-run any time).

0. Preconditions (already done once; re-run if runtime/ was cleaned):
   - tools/runtime/dosbox-harness.sh dbgprobe gate   # legacy channel
   - tools/runtime/dosbox-harness.sh dbgprobe flow   # v2 machinery
   - tools/runtime/dosbox-harness.sh diff stage tools/diffharness/scenarios/S0.scen
   - cp tools/diffharness/capture-plans/S0.json runtime/harness-out/diff/S0/capture-plan.json
     (regenerate instead if the registry changed:
      cargo run -q -p diffharness --bin dbx-plan -- tools/diffharness/scenarios/S0.scen --out …)
   - sha256sum -c MANIFEST.sha256 (bracket the corpus rsync)
1. CAPTURE RUN A (desktop with X; the game window takes the keyboard,
   the debugger rides the PTY — do not type debugger commands into the
   game window):
     FORCE_DIFF_RUN=1 tools/runtime/dosbox-harness.sh diff capture tools/diffharness/scenarios/S0.scen
   capgen parks at -break-start, arms `BPLM 1195F0`, RUNWATCHes; the
   game boots — WALK THE TITLE MENU to ZONEA/MISSION1 (new campaign).
   At the first mission frame tail the trap fires; capgen checks the
   flat CS (SELINFO base==0 — a loader-stub stop retries automatically),
   arms `BP CS:0005A6EB`, then captures 3 records (anchor + 2); the
   loader statics (map w/h, TOT/DAT/claim pointers) are read AT THE
   ANCHOR STOP (D84 resolve_at=anchor — they are mission-load values;
   the pre-mission arm stop reads garbage). stderr prints the
   selector pin + resolved cells; the transcript header records them.
   W5-walk note (D84, landed after this checklist was written): the
   same session can calibrate S0W's draft walk schedule — run
   `diff capture tools/diffharness/scenarios/S0W.scen` once (it stops
   per menu frame; the transcript's `# walk stop N walk-mode ...`
   comments map menu transitions to stop indices) and rewrite the
   placeholder stop counts in S0W.scen from them.
2. STITCH + RECORD:
     tools/runtime/dosbox-harness.sh diff stitch tools/diffharness/scenarios/S0.scen
   → runtime/harness-out/diff/S0/S0.bdld + S0.manifest.json (dumps stay
   runtime/-only). MOVE RUN A ASIDE (mv S0.bdld S0.A.bdld etc — run B
   overwrites). Record in this file: chain digest, dump sha256, the
   selector pin, w/h, the three volume pointers.
3. CAPTURE RUN B: repeat 1–2 (fresh boot, walk the menu again).
4. DH-G1 VERDICT (the queue's (d), expectation per fact #5 above):
   the W7 differ is the instrument (D87) —
     cargo run -q -p diffharness --bin dbx-diff -- \
       runtime/harness-out/diff/S0/S0.A.bdld runtime/harness-out/diff/S0/S0.B.bdld \
       --report runtime/harness-out/diff/S0/S0.G1.txt \
       --manifest runtime/harness-out/diff/S0/S0.G1.json
   (two O1 dumps auto-select double-run mode). GREEN = verdict PASS:
   the frame-counter diffs land in the suppressed/T2 budget and the
   rng rows are T3 (never bit-compared); the report lists any cell
   that moved beyond it. ANY other row diff = an engine/channel
   finding (the report names frame/row/field/both values): record
   and stop (do not hand-tune). `cmp S0.A.bdld S0.B.bdld` remains the
   raw cross-check — divergent bytes must fall ONLY inside the
   frame-counter / rng-state-a / rng-state-b blob ranges (T2/T3
   cells, menu-timing dependent). Byte-identical chains INCLUDING
   those cells need the W5 scripted walk — that is DH-G1's
   headless-S1 form, not this session's goal.
5. CYCLES CALIBRATION (queue's (e)): one more capture session with
   audio live (edit the staged plan: "env": {"SDL_VIDEODRIVER": "",
   "SDL_AUDIODRIVER": ""}) and LISTEN for audio dropouts during the
   mission; if starved, re-pin cycles DELIBERATELY per D19 + DECISIONS.
6. Close out: verdicts + fingerprints here, DECISIONS.md if any pin
   changed, .state/NEXT.md item closed, STATE.md if the phase moved.

CPU BASELINE (the other side of the diff): cargo run --release --example
parity_harness -p bedlam-game -- --out report.json; D28 anchors (reproduced
byte-identically twice this unit): scene chain 0xcae25cd08d7cbc08, sim
0x72979d5d9dedc832, frame 0x87263f149564ad25, audio 0xc862e45d2e95ad29.

## W5 injector probe (D82, 2026-08-22 — `dbgprobe inject`, unattended-safe, no game)

Proves the DESIGN §5 write machinery on the live channel, headless:
- `SMV <linear> <byte tokens>` is the write primitive (D80-verified;
  ack "DEBUG: Memory changed (N bytes)"). capgen converts plan addr
  forms: `CS:off` → linear == off (flat identity, boot-guard pinned,
  bounded ≤ 0x12583e); numeric `seg:off` → seg<<4+off (probe form).
  Every token is a 2-hex-digit BYTE (never a register name).
- Plan v2 keys: `boot_writes` (SMV at the arm stop, after resolve,
  before frame 1) and `inject` rows keyed by capture frame, applied at
  the frame stop BEFORE the watch dumps → the transcript record gets
  `frame N 1` (injection_applied in the W3 dump).
- The `command` op = the 0x4dd4a0-ring append shape: read the u32
  count through the plan's own SEG:OFF form (MEMDUMPBIN resolves
  register names/selectors itself), SMV the payload zero-extended to
  the stride at base+count·stride, SMV count+1 LE.
- Gate: tools/runtime/dbgprobe-inject-plan.json — boot write at
  0000:0500 (beefcafe, classic scratch), marker re-writes frames 1-2,
  command append frame 3 (count 0000:0510, ring 0x520, stride 0x10);
  the gate script asserts the readbacks + the injected flags. GREEN
  2026-08-22; `dbgprobe gate` + `flow` regression-green after.
- The O1 SEAM GATES: keystore/order-target/command-ring/difficulty
  EXD aliases are registry gaps — dbx-plan refuses scenarios carrying
  those steps (named seams; RE-EXD-MAP W5 note: the EXD input twin is
  NOT FUN_0002ec12, that is only the P-latch spin).

## W5 walk driver (D84 design, 2026-08-22 — the scripted menu walk; `dbgprobe walk`)

Replaces the human title-menu walk between boot and mission start so S0/S1
captures become unattended-reproducible (byte-identical chains — the
frame-counter/RNG menu churn becomes script-determined).

STOP MODEL [derived from pinned facts — RE-EXD-MAP §5c + RUNTIME "S0 live
channel mechanics" #2/#5]:
- Walk stops = BPLM hits on the frame-counter cell 0x1195f0 (the SAME trap
  that serves as the v2 boot trap — it stays armed). One stop per
  counter-writing screen frame; stop 0 = the accepted boot-trap stop
  (first post-boot flat-CS counter write = an early pre-mission screen).
- A write applied at stop i (SMV between the INC and the next screen
  loop's input read) becomes screen frame i+1's input — the same
  boundary semantics as mission inject rows (§ "W5 injector probe").
- Keystore re-arm is mandatory per input: the AnyKeyWait twin
  FUN_00030792 CONSUMES the byte on read (scans 1..0xFE, skips both
  shifts 0x2a/0x36, clears the matched cell), while polling menus leave
  bytes set. A press = write 1 at stop i; menus that poll without
  consuming need an explicit 0 (release) at a later stop. Cells (EXD):
  keystore base 0x894d4, ESC +0x01, ENTER +0x1c, arrows via the OR-0x80
  ISR remap → +0xc8/+0xcb/+0xcd/+0xd0 (up/left/right/down).
- Anchor detection: BP CS:0005A6EB (the registry s0-trigger) is armed
  only AT THE LAST WALK STOP (arm_commands = BPDEL * — drops the BPLM —
  + BP). The machine then free-runs through the mission load; the first
  anchor hit = mission start = capture frame 1. No stop-type ambiguity
  exists during the walk (only the BPLM is armed).

PLAN v3 KEYS (capgen; v2 keys unchanged, all optional):
- `walk`: list of `{"stop": N, "addr": "CS:...", "bytes": "hex"}` plain
  writes (the InjectWrite::Plain form; command-op rows are mission-phase
  only — a menu walk needs no ring appends). Walk stops run 1..max(N).
- `walk_watches`: optional calibration rows `[{id,addr,len}]` dumped at
  EVERY walk stop; values ride the transcript as `# walk stop N <id>
  <hex>` comment lines (parser skips them) so a calibration run maps
  menu transitions to stop indices mechanically.
- `resolve_at`: "arm" (default, legacy) or "anchor". dbx-plan emits
  "anchor" for ALL generated plans: the loader statics (map w/h
  0x1074b8/0x10748c, TOT/DAT/claim pointer cells) are MISSION-load
  values — read at the legacy arm stop (stop 0, pre-mission) they carry
  pre-mission bytes, so len exprs like 4+16·w·h would evaluate from
  garbage. Anchor-stop resolve reads them at mission start where they
  are valid. This also fixes the walk-less S0/S1 flow (D81 latent gap).
- boot_writes move to right after the boot-trap accept (identical stop
  for walk-less plans — no behavior change; pre-walk for walk plans).

FLOW (walk plan): boot trap accept → boot_writes → walk stops (per-stop
  writes + optional calibration dumps) → arm at the last walk stop →
  RUNWATCH → anchor hit (frame 1) → resolve → inject(frame 1) → dumps →
  frames 2+ as today.

KNOWN LIMITS (documented, not blockers):
- Screens that never write the frame counter (if any — e.g. SMK
  playback, unrecorded) are TRANSPARENT: no stops fire during them;
  a write scheduled "during" one lands at the next counter-writing
  frame. The 14-INC census covers the screen loops; whether the intro
  video player INCs is a live-calibration fact.
- A schedule that overruns into the mission (bug) leaves the BPLM
  stopping on mission frames and the anchor then lands mid-mission —
  detectable via the frame-counter/mission watch values, never silent.
- SMV input sent while the machine runs queues until the next stop
  (same proceed-anyway + ack-reliance as the frame loop).

S0W calibration: the committed S0W.scen walk schedule is a STRUCTURAL
DRAFT (stop counts are placeholders) — the first live session calibrates
the indices via `walk_watches` output (see the S0 checklist note); the
schedule itself is then just data (no code change). Headless
verification: `dbgprobe walk` GREEN 2026-08-22 (walk loop + stop
indexing incl. a pure-skip stop + write-then-read calibration notes +
arm-at-walk-end + resolve_at=anchor feeding expr lens; probe conf, no
game, BDA tick cell 0x46C as the surrogate counter); `dbgprobe
gate/flow/inject` regression-GREEN after the capgen restructure.

## W5 pad op (D86, 2026-08-22 — `dbgprobe pad`, unattended-safe, no game)

The DESIGN §5.4 PAD step's runtime read op (the capgen `{op:"pad"}`
inject form). Row shape:
`{frame, op:"pad", bank:"SEG:EXPR", slot:N, target:["SEG:EXPR" ×3]}`.

- The READ goes through the bank row's own SEG form + `slot·8`
  (MEMDUMPBIN, offset pre-evaluated by capgen — hex off in the command);
  SMV writes use the linear conversion as for every inject row.
- Record = 8 B `{u16 active@+0, x@+2, y@+4, z@+6}` (FORMATS §10 /
  7j.16 loader: active word set 1 per parsed record, x==0xFFFF ends).
  VALIDATION FAILS LOUD: active != 1 or x == 0xFFFF → capture error
  naming the slot + words (a scenario targeting a slot the staged
  mission never loaded must never emit a garbage order).
- The WRITE: {x,y,z} as three i32-LE words to the order-target cells
  (EXD 0x10e0a4/0xa8/0xac; tile coords — the shared-grammar contract).
  The game does the rest: a robot arriving on the tile arms extraction
  (FUN_00433980 → FUN_004247b5, §7j.20).
- Real O1 addresses are ALWAYS registry-derived (dbx-plan: bank from
  `static-pad-slots` — a READ anchor with its own gap error; target
  cells from `order-target`; probe plans deliberately use fabricated
  low memory and are never stitched).
- The extraction-pad census (which slot is the zone's extraction pad)
  is committed in DESIGN §7 (S6 authoring data; §7j.20 item 2).
- Gate: tools/runtime/dbgprobe-pad-plan.json seeds a fake pad bank at
  0000:0600 (slot 2 = the real ZONEA/MISSION1.PAD record 0
  (5,61,0) with active=1; slot 3 = inactive), one pad op at frame 1
  writes the triple at 0000:0620, watches assert the readback + the
  injected flag; the NEGATIVE plan (slot 3) asserts the fail-loud
  validation (capgen exits non-zero naming the slot).

## W7 the differ (D87, 2026-08-22 — `dbx-diff`; DESIGN §6 + RE-EXD-MAP §8)

`cargo run -q -p diffharness --bin dbx-diff -- <a.bdl> <b.bdl>
[--tiebreak <o2.bdl>] [--mode double-run|cross] [--t2-quantum N]
[--report out.txt] [--manifest out.json]`

- Both dumps are integrity-verified (decode) before any compare; a
  scenario mismatch between them is a hard error.
- MODE auto: two O1 dumps → `double-run` (the DH-G1 verdict — every
  row byte-exact except the budgeted classes: frame-counter T2,
  rng-state-a/b T3-never-bit-compared, both sides' DRAW COUNTS must
  still match); otherwise → `cross-channel` (E vs O1: per-field
  classes + coverage findings + O2 arbitration when --tiebreak is
  given: O2 siding with O1 → engine-bug; siding with E →
  original-divergence (a NOTE — engine keeps EXW; log DIVERGENCES.md);
  none → provisional engine-bug).
- NORMALIZATION (never raw cross-implementation bytes): E parses the
  §6a canonical grammar; O1 converts per the RE-EXD-MAP §8 map
  (robot: only the 8 pinned EXD offsets; every other field is a
  `coverage` finding — metered + reported, never silent, never
  fabricated); O2 uses the RE-EXW-SIM §3 table (seed-#1 EXW-front
  conflict OPEN until W11).
- VERDICTS: PASS / PASS-WITH-NOTES (only budgeted findings) / FAIL
  (any structural value mismatch, engine-bug, or watch-artifact);
  exit code non-zero on FAIL only. The manifest JSON is the
  git-carried fingerprint (meter + chains + dump sha256s + first
  divergence); dump blobs stay runtime/-only.
- Verified corpus-gated: S0/S1 canonical E dumps × the inverse
  normalizer (tests/differ_gate.rs) — cross PASS-WITH-NOTES with
  exactly the expected coverage set; double-run PASS modulo
  counter/RNG; FAIL on any other byte diff.

## Wine prefix for EXW (golden pipeline comparator)

- wine: system wine 11.15 (/usr/bin/wine, CachyOS). NOTE: wow64 mode -
  WINEARCH=win32 is REJECTED ("not supported in wow64 mode"); the prefix is
  a single 64-bit prefix and 32-bit PEs run through the WoW64 layer (syswow64
  populated, 890 DLLs). This is the supported modern route for a 1996 Win32
  Watcom app on this host.
- Prefix: <repo>/runtime/wine-exw, created via wineboot -u with
  WINEDLLOVERRIDES="mscoree,mshtml,winemenubuilder.exe=d" (no mono, no gecko,
  no desktop menu entries, nothing downloaded) and WINEDEBUG=-all.
- Verified: wine reg query works (persona "Windows 10 Pro"); dosdevices
  c:/z: symlinks sane; registry files present.
- Target: game-data/BEDLAM/BEDLAM.EXW = PE32 i386 GUI, 5 sections (file(1)).
  Side observation for the B2 agent: game-data BEDLAM.EXE also reads as PE32
  by file(1) - the LE/DOS4GW reading in RESEARCH-BEDLAM2-CENSUS.md may need a
  note that file(1) misclassifies LE; do not trust file(1) for the LE image.
- Wrapper: tools/runtime/wine-exw.sh (sets WINEPREFIX + overrides, exec wine).
- wine upgrade policy: same as DOSBox-X - the system wine version is part of
  the pin (record 11.15 here); host upgrades invalidate goldens, re-baseline.

## Explicitly NOT done here (follow-ups queued)

1. Launching BEDLAM.EXW under the wine prefix - needs a desktop session and
   DirectDraw; do it interactively, not from an unattended run.
2. The interactive DOSBox-X golden run: game-mode launch calibration
   (cycles pin), debugger command-name verification (BPINT/BPLM/D forms,
   linear conversion via INT3 at _entry), first watch dumps vs the D28 CPU
   anchors - all desktop-gated, checklist in dosbox-watch.skeleton.txt.

game-data/ was only read; manifests verified before and after.
