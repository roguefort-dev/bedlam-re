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

CPU BASELINE (the other side of the diff): cargo run --release --example
parity_harness -p bedlam-game -- --out report.json; D28 anchors (reproduced
byte-identically twice this unit): scene chain 0xcae25cd08d7cbc08, sim
0x72979d5d9dedc832, frame 0x87263f149564ad25, audio 0xc862e45d2e95ad29.

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
