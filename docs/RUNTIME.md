# Pinned runtimes (DOSBox-X, Wine) - P4 prep

Provenance: created 2026-08-18 by the P4-prep run. Purpose: PLAN sec P4 item 2-3
(differential harness via DOSBox-X debugger; golden pipeline with pinned
dosbox/wine versions + configs). Everything lives under repo-local gitignored
runtime/ so AGENTS.md rule "never modify files outside the repo" holds and pins
are reproducible on a fresh clone of the tooling (runtime contents themselves
are re-downloadable artifacts, never committed).

## DOSBox-X (differential harness target)

- Channel: flathub `com.dosbox_x.DOSBox-X`, user install with
  `XDG_DATA_HOME=<repo>/runtime/xdg` (whole user flatpak lives inside the repo).
- PIN: version **2026.08.02**, flathub commit
  `fa89039ca01aca36d9031f287d69b885d7510fb24499e9c33e1db420ab6ccdb2`
  (2026-08-15, runtime org.freedesktop.Platform 25.08).
- Why flathub and not an AppImage: upstream GitHub releases ship NO Linux
  binaries at all - verified 2026-08-18 across the last 6 tags (2026.06.02 ..
  2026.08.02 + osfree twins): assets are Windows/macOS/hx-dos only. Flathub is
  the official Linux channel (linked from dosbox-x.com) and is current with
  upstream (2026.08.02 = latest tag). The queue item said AppImage; that
  channel does not exist anymore, decision recorded as D19.
- Debugger presence (harness requirement): the shipped binary carries the
  integrated debugger (strings: INT-3 auto-breakpoint config text, BP-style
  commands). Headless smoke `--version` exits 0.
- Wrapper: `tools/runtime/dosbox-x.sh` (sets XDG_DATA_HOME, exec flatpak run).
- Upgrade policy: NEVER `update` blindly. A new pin is a deliberate decision:
  install to a NEW commit, smoke-test, re-baseline goldens, update this file.

## Wine prefix for EXW (golden pipeline comparator)

- wine: system `wine 11.15` (/usr/bin/wine, CachyOS). NOTE: wow64 mode -
  `WINEARCH=win32` is REJECTED ("not supported in wow64 mode"); the prefix is a
  single 64-bit prefix and 32-bit PEs run through the WoW64 layer (syswow64
  populated, 890 DLLs). This is the supported modern route for a 1996 Win32
  Watcom app on this host.
- Prefix: `<repo>/runtime/wine-exw`, created via `wineboot -u` with
  `WINEDLLOVERRIDES="mscoree,mshtml,winemenubuilder.exe=d"` (no mono, no gecko,
  no desktop menu entries, nothing downloaded) and `WINEDEBUG=-all`.
- Verified: `wine reg query` works (persona "Windows 10 Pro"); dosdevices
  c:/z: symlinks sane; registry files present.
- Target: game-data/BEDLAM/BEDLAM.EXW = PE32 i386 GUI, 5 sections (file(1)).
  Side observation for the B2 agent: game-data BEDLAM.EXE also reads as PE32
  by file(1) - the LE/DOS4GW reading in RESEARCH-BEDLAM2-CENSUS.md may need a
  note that file(1) misclassifies LE; do not trust file(1) for the LE image.
- Wrapper: `tools/runtime/wine-exw.sh` (sets WINEPREFIX + overrides, exec wine).
- wine upgrade policy: same as DOSBox-X - the system wine version is part of
  the pin (record 11.15 here); host upgrades invalidate goldens, re-baseline.

## Explicitly NOT done here (follow-ups queued)

1. Launching BEDLAM.EXW under the prefix - needs a desktop session and
   DirectDraw; do it interactively, not from an unattended run.
2. Flathub sandbox filesystem access for game-data (needs
   `flatpak override --user --filesystem=` once the harness exists).
3. DOSBox-X harness config (cycles=pinned, machine type, debugger watch
   scripting) - P4 work that builds on PLAN sec P4 item 2.

game-data/ was only read (ls/file), manifest verified before and after.
