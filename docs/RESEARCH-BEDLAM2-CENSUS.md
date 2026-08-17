# Bedlam 2: Absolute Bedlam — acquisition + compatibility census (2026-08-17)

## Source (provenance)
- Internet Archive item: msdos_Bedlam_2_-_Absolute_Bedlam_1997
  https://archive.org/details/msdos_Bedlam_2_-_Absolute_Bedlam_1997
- File: Bedlam_2_-_Absolute_Bedlam_1997.zip (33,011,792 B)
  sha256: see ~/Backups/bedlam2/bedlam2.zip.sha256
- Scene release per CLASS.NFO: Absolute Bedlam (c) GT Interactive, released
  1997-11-02 by CLASS (supplier TALENT UK). File dates 1996-12-24 (dev build).
  Never officially released (unfinished game); this is a cracked retail-preview rip.
- Backup: ~/Backups/bedlam2/ (zip + hash). Staged at game-data-2/ (gitignored,
  MANIFEST-2.sha256 verified).

## Completeness verdict
Cross-check of every data filename referenced in 8street/Bedlam2 sources (150
literals) against the rip: only SAVED.BDL + HISCORE.BDL absent = runtime-created
save files. The rip is COMPLETE w.r.t. everything the code loads. Missing
MISSION5.* in 5 of 6 zones + scattered gaps = authentic unfinished-build state
(8street: "some level files seem were never created"), not rip damage.

## Census (game-data-2, 989 files, tools/inspect unmodified; re-verified 2026-08-17 with current tooling, see re-census section)
- BIN sprite banks: 191/191 parse, 11,008 PNGs — SAME RLE16 + dir format as B1
- CGR tile banks: 36/36 parse, 4,608 tiles — SAME dual codec (raw/byterle)
- MAP/TOT/COL grid16: 102/102; DAT grid8: 34/34; 34 plane0 map renders
- TRT/MRK/POS/PAD/PTH/NME/BDG: 34 each — same table formats
- RAW pcm8: 106; PAL: 83 (+ 10 variants); TRN: 16; SMK: 5; MIN 7; LNK/LNG 43+7
- BDL: 2 parsed = both OPTIONS.BDL (root + MIRAGE/AB_BED byte-dupe);
  2 unknown-variant = both CONFIG.BDL 61B (root 67bba211 + MIRAGE 4a805ec9)
- 90.0% parse (890/989) with ZERO Bedlam-2-specific code

## Re-census 2026-08-17 23:0x (after the music-decoder promotion)
Re-ran tools/inspect over game-data-2 -> derived-2 now that the mrs arm
is a real dumper (7325d23 + 3530a1b) and diffed against the 04:30
stub-era summary: BYTE-IDENTICAL (989 files, 0 breakdown and 0 per-file
status diffs). decode-song has no B2 input: the corpus contains ZERO
.MRS/.MRW files - SOUND/MIDI/ exists but is an EMPTY directory (0
entries; absent from MANIFEST-2 because sha256 manifests cannot list
empty dirs). The mrs/mrw arms are therefore provably no-ops on B2.
Same run refreshed the stale B1 census (derived/ dated 02:40, still
mrs:partial): now mrs:parsed 5/5 + mrw:parsed 5/5 with loop ticks
331/400/1476/1600/3388 == the RE-EXW-MUSIC.md 3b invariants; B1 =
950/1069 (88.9%) parsed vs B2 890/989 (90.0%). MANIFEST.sha256 and
MANIFEST-2.sha256 both verified OK before AND after the runs.
[provenance: derived*/summary.json diff + sha256 manifests; confidence: high]


## B2 Ghidra import plan - DOS4GW LE loader (researched 2026-08-18)

Goal of this section: pick the loader path and write the concrete import runbook
for BEDLAM.EXE (B2, DOS canon target) - and B1 BEDLAM.EXD, which is the same
format class. No Ghidra project changes were made in this run (research only).

### 1. Verified file facts (local parse; layout per Open Watcom exeflat.h)

BEDLAM.EXE (B2, 672,399 B, sha in MANIFEST-2):
- MZ stub 0x0000-0x4a8f (19 KB Watcom DOS launcher - NOT an embedded extender;
  DOS4GW.EXE ships as a separate file), e_lfanew=0x4a90 -> LE signature.
- cpu=2 (386) os=1 flags=0x200 (PM_COMPATIBLE only - INTERNAL/EXTERNAL_FIXUPS_DONE
  both UNSET). module name "BEDLAM" in resident-names table (hdr+0x2ac).
- Objects (hdr+0xc4, 2 x 24 B, exactly filling [0xc4,0xf4) up to objmap_off):
  obj1 CODE  base=0x00010000 vsize=0x66970  R+X+BIG, pages 1..103 (103 entries)
  obj2 DATA  base=0x00080000 vsize=0xb04ee  R+W+BIG, pages 104..110 (7 entries;
  only ~28 KB file-backed - the rest of the 722 KB object is implicit zero-fill
  BSS+stack). Ghidra block must span [0x80000, 0x1304ee).
- page_size=4096, last_page=0x48f, num_pages=110 = 103+7 (structural cross-check).
  Page map (hdr+0xf4, 110 x 4 B LE entries, ends exactly at rsrc_off=0x2ac):
  ALL flags=VALID(0), page numbers sequential 1..110 (raw first entry bytes
  00 00 01 00, last 00 00 6e 00; the 24-bit number reads sequential only
  big-endian-style - the lx-loader parse is the reference for the encoding;
  sequential+page_off decodes the file regardless).
- eip=0x56a60 and esp=0xb04ee are OBJECT-RELATIVE OFFSETS (LX/LE spec: start
  address = object number + offset within object) -> LINEAR entry 0x66a60
  (obj1+0x56a60), LINEAR initial esp 0x1304ee = obj2 base+vsize EXACTLY
  (top-of-object stack, the canonical Watcom layout; same coherence in B1
  EXD: 0x80000+0xa583e = 0x12583e = its obj2 top). CORRECTION 2026-08-18:
  the first version of this section (976f19f) read eip/esp as already-linear
  off an arithmetic slip; the offset-form reading is confirmed by the
  esp-equals-object-top gate in both exes and by an independent parallel
  derivation (b8f63e6). A code probe at file 0x7d860 (page 0x46) found valid
  i386 with an obj2 absolute store (89 0d 10 5e 0a 00 = mov [0x000a5e10],
  ecx) - that validated page carving, NOT the entry point; the entry itself
  is page 0x56, file 0x8d860.
- Data pages: file [0x36e00, 0xa428f) - 109 full pages + last_page 0x48f
  consumes the file EXACTLY (0x36e00 + 0x6d000 + 0x48f = 0xa428f = file size;
  no slack - strong gate).
- Fixups NOT pre-applied: fixup page table hdr+0x2b7 (111 dwords; first entries
  0x0,0x48b,0x8eb,0x1116,0x1646), records from hdr+0x473, fixup_size=0x3201a
  (~205 KB). num_impmods=0 (no imports), debug_off=0, nonres table absent,
  autodata_obj=2, num_preload=0.
BEDLAM.EXD (B1 DOS, game-data/BEDLAM/BEDLAM.EXD, 655,133 B): same class - LE,
same 2-object layout with identical bases 0x10000/0x80000, 107 pages, eip
0x4fbb0 (offset-form -> linear 0x5fbb0), esp 0xa583e (-> linear 0x12583e =
obj2 top, same gate), fixup_size=0x309c6. One loader serves both.

CRITICAL implication: because internal fixups are unapplied, every absolute
address in the raw pages is a placeholder - a carve-flat-binary raw import
yields a program whose pointers are garbage. Any import path MUST apply the LE
fixups (this kills the naive raw-binary fallback in PLAN sec P2; see the
honest fallback version below).

### 2. Loader landscape (web-verified 2026-08-18)

- Stock Ghidra 12.1.2 - including our custom ghidra-12.1.2-watcom build - has
  NO LE/LX loader: ghidra.app.util.bin.format.lx.LinearExecutable exists in
  Base.jar only as a constructor that throws NotYetImplementedException
  (verified by javap in this build; not registered in the Loader service file).
  An un-extended import treats the file as raw data.
- yetmorecode/ghidra-lx-loader (Apache-2.0, 78 stars, active) = the community
  loader. Latest release v12.0.1 (2026-01-29) targets Ghidra 12; supports
  exactly our case ("MSDOS DOS/4 LE-Style"), full page-map + fixup application
  (relocation-window integration), optional labels per fixup AND per page,
  typed headers mapped as overlay, manual object-base/selector override for
  debugger sync. Tested by its author on Watcom DOS4GW games (F1 Manager Pro,
  Redguard). https://github.com/yetmorecode/ghidra-lx-loader
- End-to-end prior art: alexbevi Harvester RE series (2026-03, linked from
  RESEARCH.md): Watcom/DOS4GW 1996 LE -> ghidra-lx-loader -> anchored
  decompilation; worked first-class. Their SB.EXE unbind step is NOT needed
  for us (Harvester exe is bound to its extender; ours is a plain LE shipped
  beside a separate DOS4GW.EXE).
- Official stance + fallthrough behavior: Ghidra issue #532 (LE loader
  support, open since 2019-04-29) - maintainers state Ghidra does not support
  LE; MzLoader only claims a file when e_lfanew == 0, so an LE with the MZ
  launcher stub falls through to Raw Binary. [folded from b8f63e6]
- FALLBACK loader: oshogbo/ghidra-lx-loader 1.7 (2026-04-20, built for Ghidra
  12.0.4) - NO license file: fine to RUN, never copy its code. Its prebuilt
  zip crashes on version mismatch (issue #37, 2026-07-04) - concrete evidence
  that cross-minor prebuilt installs are unsafe and build-from-source is the
  sound path. yetmorecode repo last push 2026-07-02 (active beyond the
  v12.0.1 release). [folded from b8f63e6]
- Tooling side notes: Open Watcom wdis is an object-file disassembler -
  useless on a linked LE; objconv/rizin LE support unverified - do not build
  on them. String anchors for later cross-checks: DOS4GW banner at file
  0x4485 (inside the MZ stub); runtime DOS4GW string at file 0xa20f7 =
  linear 0x842f7. [folded from b8f63e6]
- Alternatives (RetDec/Reko/r2/IDA Free) all lack scriptable LE support -
  see docs/RESEARCH.md; no change to that assessment.

### 3. Version compatibility (the one open risk) and install order

No 12.1.x release of the loader exists yet. Our install is a custom 12.1.2 DEV
build (application.properties: version 12.1.2, release.name=DEV, gradle min
8.5) with x86openwatcom.cspec present. Two paths, preferred first:

a. BUILD FROM SOURCE against our exact install (removes version risk):
   clone the repo at master (51 commits; extension.properties uses @extversion@
   stamped at build time), then
   gradle -PGHIDRA_INSTALL_DIR=/home/kato/ghidra-12.1.2-watcom
   - build.gradle delegates to support/buildExtension.gradle inside the
   installation, which needs NO Ghidra source tree, just the installed Ghidra
   (+ JDK per Ghidra 12, Gradle >= 8.5). Produces a 12.1.2-stamped extension
   zip. API drift between release and master would surface as compile errors
   immediately - cheap to detect.
b. FORCE-INSTALL prebuilt v12.0.1 zip (2-minute smoke test; Ghidra accepts
   version-mismatched extensions with a warning). Loader-extension API surface
   has historically survived minor Ghidra bumps (assessment; confidence:
   moderate) - acceptable ONLY because we smoke-test on a scratch project
   before any real import (runbook step 2).

### 4. Import runbook (follow-up task; NOT to be run from this research task)

0. Guard: pgrep -f analyzeHeadless (filter out the agent own cmdline), no
   import if one is running or already succeeded for the target program name.
1. Install loader (path a preferred, b acceptable) + restart Ghidra.
2. SMOKE TEST on a throwaway project in /tmp/opencode (NOT ghidra-project/):
   import game-data-2/BEDLAM.EXE; verify (i) two memory blocks 0x10000 (X) and
   0x80000 (W) with the data block spanning to 0x1304ee (zero-fill tail!),
   (ii) entry at 0x66a60 (eip is object-relative), (iii) fixup labels present, (iv) decompile at/near
   entry produces sane watcall-ish code, (v) typed LE header overlay readable.
   If any check fails -> fix or fall back; do not proceed to the real project.
3. REAL import: NEW program in BedlamWatcom project (name e.g. BEDLAM.EXE-B2),
   language x86:LE:32:default + x86openwatcom cspec (same pick as EXW; zero
   imports means the global-cspec wart from Ghidra issue #156 cannot bite).
   Loader options ON: fixup labels, page labels, headers-as-overlay; object
   bases left at file values (0x10000/0x80000; manual override exists for
   later DOSBox-X address sync).
4. Post-import census: strings pass for asset filenames, seed the function DB,
   then the actual B2 goal - boot/init comparison vs EXW findings (divergences
   go to docs/DIVERGENCES.md per PLAN).
5. Manifest check before AND after (B2 manifest lives at repo root but entries
   are corpus-relative: cd game-data-2 && sha256sum -c ../MANIFEST-2.sha256).
6. Real-import command form (from b8f63e6, matches EXW conventions):
   ~/ghidra-12.1.2-watcom/support/analyzeHeadless ghidra-project BedlamWatcom
   -import game-data-2/BEDLAM.EXE -processor x86:LE:32:default
   -cspec openwatcomcpp   (the cspec NAME is openwatcomcpp even though the
   file is x86openwatcom.cspec - mirror AGENTS.md EXW wording; confirm the
   program list before AND after so no duplicate program lands).
   After landing, ALL B2 RE goes through -process BEDLAM.EXE -noanalysis +
   postScripts exactly like the EXW pipeline (scripts tools/ghidra-scripts/
   B2*.java, dumps to ghidra-project/ root).
   Risks: ~EXW-scale analysis time for the 420 KB code object; DOS/4
   entry-bundle semantics may need manual work even with fixups applied.

Fallback if BOTH loader paths fail (cost ~a day): Python applier using
exeflat.h semantics - pages sequential from 0x36e00 into [0x10000..]+[0x80000..];
fixup page table (111 dwords) + records enumerate every relocation; internal
fixups rewrite placeholders with target object VAs (bases absolute in-file, no
slide needed for static analysis); emit a flat memory image, import as raw
binary at 0x10000 with the cspec, then a postScript to set entry + create the
obj2 zero-fill tail. This is the honest version of the PLAN raw-binary fallback.

CONSOLIDATION NOTE 2026-08-18: a parallel sibling run (b8f63e6, spawned
during the 00:04 client-death storm) committed an independent version of
this research as a second section in this file; per the incident-#3 rule
(one canonical version, not two), its UNIQUE findings (issue #532, oshogbo
fallback + crash #37, exact file-consumption gate, object-relative eip/esp
derivation, concrete import command, tooling side notes) were folded HERE
and its section removed. Its DECISIONS D18 stands unchanged.

[provenance: BEDLAM.EXE/EXD header parses = this run, python over the corpus
(read-only; both manifests verified after); loader facts = upstream README,
releases page, extension.properties, build.gradle (fetched 2026-08-18);
exeflat.h layout = open-watcom-v2 master; Ghidra stub = javap of Base.jar in
ghidra-12.1.2-watcom; case study = alexbevi.com 2026-03-14. confidence: high
for file facts (structurally cross-checked 3 ways), moderate for 12.0.1-on-12.1.2
compat until the smoke test runs.]

### 5. Import EXECUTION record (2026-08-18, run d09e41f..this)

DONE end-to-end. Loader path (a) from sec 3 was used: built from source
(yetmorecode/ghidra-lx-loader master, clone 2026-08-18) with
gradle 8.14.3 -PGHIDRA_INSTALL_DIR=/home/kato/ghidra-12.1.2-watcom -> 12.1.2-
stamped extension zip (build clean in 18s, no API drift). Three gotchas fixed
the research gaps, all now proven:
1. EXTENSION INSTALL DIR for headless discovery is
   ~/.config/ghidra/ghidra_12.1.2_DEV/Extensions/ (userSettings/Extensions -
   verified in GhidraApplicationLayout.findExtensionInstallationDirectories
   bytecode; NO Ghidra component). The <install>/Extensions/Ghidra dir is only
   the GUI archive source. [confidence: high, probed]
2. -loader CLI matches the loader CLASS SIMPLE NAME, not the display name:
   -loader LeLoader (HeadlessOptions.setLoader -> LoaderService matches
   Class.getSimpleName()). "Linear Executable (LE-Style DOS)" is rejected.
3. MzLoader CLAIMS an LE file with MZ stub at higher priority than LeLoader
   when the loader is not forced (sec 2 fall-through-to-Raw-Binary claim was
   wrong); it then dies with "Selected Language must have a segmented address
   space" under x86:LE:32:default. Force -loader LeLoader always.
Working import command (also the one for any future LE binary):
  ~/ghidra-12.1.2-watcom/support/analyzeHeadless <projdir> <proj>     -import <file> -loader LeLoader -processor x86:LE:32:default     -cspec openwatcomcpp
Loader options were set via Java prefs (B2SetLxPrefs.java; the loader reads
Preferences.userRoot() node yetmorecode.ghidra.lx.Options for defaults because
the headless -loader-* flag surface is undocumented): fixup labels ON, page
labels ON, map-extra ON, fixup stats ON.

SMOKE TEST (scratch /tmp/opencode/ghidra-smoke, disposable): ALL 5 gates PASS.
 i  .object1 0x10000-0x7696f (420208 B) + .object2 0x80000-0x1304ed
    (722158 B - spans to 0x1304ee exactly, implicit zero-fill present).
 ii ENTRY 0x66a60 = base 0x10000 + eip 0x56a60 - loader log line confirms the
    object-relative eip reading of sec 1.
iii 24041 relocations applied (23939 = sel16:1 + off32:23938, then 102 off32),
    24023 fix_ labels.
 iv  Decompile at _entry = sane Watcom DOS4GW CRT startup (int 21h version
    probe, 0x4458 DX-marker, PSP cmdline @0x81) - exactly right for DOS extender.
 v  .image overlay 0x0-0xa428e with IMG_ header labels (23990).

REAL IMPORT: game-data-2/BEDLAM.EXE -> BedlamWatcom:/BEDLAM.EXE, full
auto-analysis green (Decompiler Switch Analysis 15.2s etc). Program list
checked before (no BEDLAM.EXE) and after (exactly one). Post-import -process
BEDLAM.EXE -noanalysis pipeline verified working (B2BootCompare pass).
Dumps: ghidra-project/b2-functions.txt (671 fns vs EXW 675 - same scale),
b2-strings.txt (414 strings, 216 file-ish), b2-boot-compare.txt.

FIRST BOOT/INIT COMPARISON vs EXW (b2-boot-compare.txt):
- RNG SEEDS IDENTICAL ACROSS BUILDS: B2 sets EBX=0x39447 (234567) @0x2f812 +
  ECX=0x1e240 (123456) @0x2f817 inside FUN_0002f731 (2882 B game-init fn,
  called from CRT init FUN_0006b1bc which passes DAT_001280d4/DAT_001280d8 -
  CORRECTED 2026-08-18 by sec 6: those two are argc/argv, the true seed
  globals are 0x11ef1c/0x11ef1c-adjacent pairs, see below); EXW pins the
  same pair as data 004ede48/004ede4c.
  A reseed site MOV [0x11ef1c],0x1e240 lives in FUN_0005eaf9 (5664 B).
  [confidence: high - exact constants + init-chain position]
- Entry chain B2: 0x66a60 _entry -> 6b1bc (CRT init: 2f731 game init,
  6d96e -> 71736 chain) - DOS CRT shape, no Win32 message pump (expected).
- Strings confirm corpus wiring: C:\MIRAGE\AB_BED\OPTIONS.BDL @0x8418e,
  BEDLAM.LOG @0x841d1, SOUND\SFX\*.RAW + GAMEGFX\*.PAL families from
  0x8465b onward - matches the B2 corpus census (second config dir, RAW-only
  audio).
Next B2 RE hooks: ALL THREE CLOSED 2026-08-18 by the naming run -> sec 6
(entry chain named; tick source = 100.01 Hz PIT INT-8 ISR; zone/mission
lookup tables located at 0x81dba/0x81dda/0x81e46).

[provenance: import + probes = this run (logs /tmp/opencode/b2-smoke.log,
b2-import.log, dumps in ghidra-project/); loader options mechanism = Options
prefs read path; manifest-2 verified OK before + mid + after. confidence:
high - every gate observed directly.]


## 6. Entry chain, tick source, zone/mission stride (2026-08-18 naming run)

[provenance: three -process BEDLAM.EXE -noanalysis passes, no re-import
(B2EntryTick, B2EntryNames, B2TblDump; commits 2df7664 + c3b1552 + this);
dumps ghidra-project/b2-entry-tick.txt, b2-decomp-all.txt (671 fns, zero
decompile failures), b2-entry-names.txt, b2-tbl-dump.txt. confidence: high -
every claim below read directly from decompile, listing, or program memory.]

### 6.1 Entry chain (named)
- _entry@0x66a60: Watcom DOS4GW CRT stub (INT 21h AH=30h, DX-marker 0x4458,
  PSP cmdline @0x81) -> CrtInitChain@0x6b1bc (stashes argc -> g_argc@0x1280d4,
  argv -> g_argv@0x1280d8 - CORRECTS the sec 5 candidate-rng guess) ->
  GameInit@0x2f731 (2882 B): OPTIONS.BDL presence check (spawns SETUP.EXE
  path when missing), mouse-driver check, LANGUAGE.{ENG,GER,SPA,FRE,ITL,DCH}
  select+load, seeds BOTH RNGs as code constants
  _DAT_0x11ef18 = 0x39447 (234567) and _DAT_0x11ef1c = 0x1e240 (123456) -
  the same two constants EXW plants at 004ede4c/004ede48 - then TickInstall,
  a 0x302-byte palette alloc, and the EPISODE LOOP. GameInit IS the B2
  GameMain-analog shell (boot + campaign loop in one function).
- RNG steppers: RngStepA@0x1220e walks the coupled 16-bit pair
  0x11ef1c/0x11ef1e (>>8 / <<7 | carry mixing, additives 0x3619 / 0x62e9),
  RngStepB@0x1224f the same over 0x11ef18/0x11ef1a; 177 and 21 call sites.
  RngReseedSite@0x5eaf9 (5664 B) re-pins [0x11ef1c] = 123456.

### 6.2 Tick source (headline)
NO INT 28h idle loop, NO DPMI (INT 31h) vector hook for timing, and the PIT
is NOT reprogrammed at boot. B2 installs a hardware-timer ISR on demand:
- TickInstall@0x32546: zeroes clock counters, DosGetVector@0x1270a (INT 21h
  AH=35h) saves the old INT-8 owner, PitProgram@0x325f9 writes OUT 0x43,0x34
  then OUT 0x40 lo/hi of divisor 0x2e9b (11931 -> 1193182/11931 = 100.01 Hz;
  variable divisor, which is why the constant-divisor census probe missed it),
  DosSetVector@0x12727 (INT 21h AH=25h) points vector 8 at the handler.
- Int8TickHandler@0x12734 (created + named this run): CLI, PUSHFD, EOI
  (OUT 0x20,0x20) sent IMMEDIATELY, reentrancy lock via XCHG [0x11ef2c],1
  (in-progress ticks are dropped, not queued), a flag-gated background call
  FUN_000136e0, then increments SEVEN counters (0x801a6, 0x80010, 0x11f158,
  0x11f0c8, 0x11f0c4, 0x11f0b4, 0x11f0b0), calls ClockDivider100Hz@0x1287b
  (hundredths -> 99 -> seconds -> 59 -> minutes -> 59 -> hours; feeds the
  %02i:%02i:%02i play-clock), runs the PALETTE BANK CYCLE: while
  [0x11f138] is inside [0x90,0x98) it advances on (0x11f0c8 & 7) == 0, i.e.
  12.5 Hz, wrapping 0x97 -> 0x90 - byte-identical behavior to the EXW
  TimerCallback bank animation - and services the MOUSE on odd ticks
  ((0x11f0c8 & 1) != 0 -> every other tick = 50 Hz): FUN_0001259f poll,
  FUN_00012a8d / FUN_000128df / FUN_00012960 clamp+store chain, clamping
  against 0x11efd8 / 0x11efd4 = 0xf0 / 0x140 -> a 320x240 game-coordinate
  space (EXW clamps the same pattern at 640x480).
- TickShutdown@0x32507: DosSetVector(8, 0) + PitProgram(0xffff) -> back to
  the stock 18.2 Hz. SystemShutdown@0x34d90 calls it in teardown.
- Present path: WaitVRetrace@0x10856 double-polls 0x3da bit 3 (wait
  deassert, then assert), gated by g_wait_vsync@0x11f130; sole caller
  FUN_0001066b (339 B blit/present helper, 9 call sites across GameInit and
  the screen functions).
- VERDICT: B2 (DOS) and EXW (Win95) share the SAME two-clock architecture -
  a ~100 Hz service interrupt (counters, 12.5 Hz palette banks, ~50 Hz mouse
  polling, play-clock divider) plus a vblank-locked present. The EXW-derived
  parity budget (D16 fixed-rate sim + present-paced frames) therefore carries
  to the DOS build unchanged; see D22.

### 6.3 Zone / mission stride (EXW 7x5 @004decb2 analog)
Decode lives in the GameInit episode loop (linear progress DAT_0012576c
exits when > 0x1a -> 27 linear missions, 0..26):
- order[8] dwords @0x81dba = {3,0,1,5,9,13,17,21} (flat start per campaign
  slot; slot = DAT_00126848, set from the save block);
- zone letters @0x81dda, 27 dwords (byte value + @ renders A.. for 1..6;
  values 7/8 = special screens; first dword 0x19 = 25 - intro/endgame
  semantics not fully pinned, raw dump in b2-tbl-dump.txt);
- mission[27] dwords @0x81e46 = {1,1,1,2,3,1,4,3,2,1,1,2,1,1,2,1,3,2,2,4,
  3,2,4,3,3,4,3} (values 1..4);
- formula: zone = zonetable[order[slot] + sub], mission = missiontable[same
  index], sub = DAT_00126858 advances per completed level inside the slot;
- path builder FUN_0005cc66 (and siblings 0x5d409/0x5cea7/0x5d4ef/0x5da22 -
  the save/load/hi-score family): "EDITOR\ZONE" + (zone + @) + "\MISSION"
  + itoa(mission), and when mode flag DAT_0011f11c == 2 the mission number
  is mission + 5 -> files 6/7.
- CROSS-CHECK vs corpus: every EDITOR/ZONE{A..F} dir ships MISSION{1,2,3,4}
  regular files plus MISSION{6,7} mode-2 files and NEVER 5 - exactly what
  the +5 rule predicts; 6 zones A..F (no G/H dirs, so zone 7/8 entries are
  menu/endgame screens, matching the DB_MAIN/GAMEOVER asset families).
- DIVERGENCE vs EXW: EXW = 7 zones x 5 levels, arithmetic
  clamp((zone-2)*5 + level - 1, 1, 26); B2 = 6 zones x (4 regular + 2 alt)
  via explicit lookup tables. Implication #3 (engine parameterizes content,
  never hardcodes B1 counts) now has the concrete B2 shape.

### 6.4 Names persisted this run
BedlamWatcom:/BEDLAM.EXE: CrtInitChain, GameInit, RngReseedSite, RngStepA,
RngStepB, TickInstall, TickShutdown, PitProgram, PortOut, DosGetVector,
DosSetVector, WaitVRetrace, ClockDivider100Hz, SystemShutdown,
Int8TickHandler@0x12734 + labels g_rng_a_seed, g_rng_b_seed, g_pit_divisor,
g_int8_ctr0, g_clock_hundredths/seconds/minutes/hours, g_clock_enabled,
g_tick_installed, g_wait_vsync, g_int8_old_cs, g_argc, g_argv,
g_screen_w_320, g_screen_h_240. Scripts: B2EntryTick.java, B2EntryNames.java,
B2TblDump.java (all javac-precompiled against the Ghidra jars after the
getMnemonic vs getMnemonicString API slip - OSGi again surfaced it only as
ClassNotFoundException; precompile is now the rule for new B2 scripts).

## Divergences vs Bedlam 1 (data layer)
- 6 zones (A-F) vs 7; missions per zone differ; MISSION5 mostly absent
- No MRW/MRS music scores (SOUND/MIDI/ present but EMPTY); audio = 106
  RAW = 92 SFX + 14 LOOPS. LOOPS names overlap the B1 MRS song slots
  (DEBRIEF/OPTIONS/SHOP plus numbered variants; B1 additionally has
  BRIEF/SELECT) => B2 ships screen music as PCM8 loops instead of MRS
  synth [confidence: moderate, name-overlap only]. LOOPS/OPTIONS.RAW is
  byte-identical to OPTIONS1.RAW (sha 165539d1...). + HMI .386 drivers
- Single LANGUAGE.ENG (no multi-language set)
- DOS exe only (BEDLAM.EXE 672KB LE/Watcom + DOS4GW) — no Win95 build in this rip
- UNIVBE/UVCONFIG + SETUP present (VESA SVGA 480p-1440p native support)
- New: editor MISSION?.BIN zone banks inside EDITOR/ZONE? dirs
- MIRAGE/AB_BED/ = second Absolute-Bedlam config dir: OPTIONS.BDL
  byte-identical dupe of root (a6b970e9...); CONFIG.BDL same 61B layout
  but DIFFERENT bytes (4a805ec9 vs 67bba211 = two frozen SB setups);
  BEDLAM.LOG zero-length (both LOGs are the empty-file sha e3b0c442...)
- pending:queued 3 = scene/util payload only, no loader warranted:
  CLASS.NFO, GAMEGFX/CHKLIST.MS, UNIVBE.INI
- PAL 256B-variant set grows vs B1: DARKPAL/DARKPALS/SELDARK (same as
  B1) + B2-only DARKPALT.PAL and BRF_TX.PAL; the 65536B (TXPAL1-3) and
  98B (CONSPAL/FULLPAL) variant sets are identical by name


## Implications
1. bedlam-assets transfers nearly 1:1 — same formats, second corpus for fuzz/coverage
2. Second RE target: BEDLAM.EXE (DOS canon, same HMI stack) for logic divergence
3. Engine must parameterize content (zones/missions/units), not hardcode B1 counts
