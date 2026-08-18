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


## 7. Episode-loop progression, INT8-counter verdicts, mission pacing (2026-08-18 episode-loop run)

[provenance: two more -process BEDLAM.EXE -noanalysis passes (B2EpisDump from
the transport-killed 02:0x run, adopted + verified; B2EpisClose, B2EpisNames -
this run), dumps ghidra-project/b2-epis.txt + b2-epis-close.txt + prior
b2-decomp-all.txt; confidence: high - every claim below read from decompile or
listing; the dead-run intermediate notes were re-derived from artifacts, not
trusted.]

### 7.1 The seven INT8 counters - final verdict (task a)
ISR body decompiled in full this run (327 B). Order of service: read
reentrancy flag -> EOI (OUT 20h,20h) -> if not re-entered: lock -> optional
PcmMixerService -> increment all 7 -> ClockDivider100Hz -> palette-bank
advance (while [0x11f138] in [0x90,0x98): on (g_isr_phase & 7)==0, wrap
0x97->0x90) -> odd-tick (50 Hz) mouse block -> unlock. The mixer call is
gated by g_snd_drv_active@0x11ef50 && g_snd_enabled@0x11ef24 &&
g_snd_service_arm@0x11f0e0 (NOT by the flip lock - that gates the mouse
block). Counter roles:
- 0x801a6 g_ctr_snd_a: AUDIO TICK BASE. Stub pair SndStubResetTicks@0x12ecf
  (zeroes 0x801a6 + 0x801aa) / SndStubElapsedMs@0x12ee4 (returns ticks * 10
  = ms); both installed by SoundStubInstall@0x12eb0 into the runtime pointer
  table 0x81f98/0x81fa0 + ISR patch slot 0x1279b4 when sound init fails.
- 0x80010 g_ctr_snd_b: AUDIO POSITION base, set via SndSetPos80010@0x607b0
  (8-byte orphan, EDX param). 0x801a6/0x80010 live in the low driver-data
  region - sound-system bookkeeping, not game timing.
- 0x11f158 g_ctr_dead1: DEAD. Zero readers anywhere (listing census,
  getReferencesTo, full 671-fn sweep all agree).
- 0x11f0c8 g_isr_phase (prev g_int8_ctr0): ISR-INTERNAL ONLY - palette-bank
  phase (& 7) and mouse phase (& 1); TickInstall save/restore aside.
- 0x11f0c4 g_ctr_timeout: the 100.01 Hz TIMEOUT base. WaitTicks100Hz@0x3264b
  = zero + spin CMP/JG (the sweep decompile dropped the 2-instr loop; listing
  in b2-epis-close.txt pins it). 10 wait sites: MapRoomSelect x7 (2000 ticks
  = 20 s screens, click-escapable), FUN_0005dbad @750, FUN_0005e4cf x2 @500.
- 0x11f0b4 g_ctr_dead2: DEAD (same 3-way proof).
- 0x11f0b0 g_ctr_delay5: 5-tick (50 ms) micro-delay in FUN_0005eaf9.
NO counter gates sim or render. The 50 Hz odd-tick mouse block also draws the
software cursor into the visible page (gated additionally by g_banked_video,
g_flip_lock == 0, and a cursor-shape/320x240 mismatch check against the page
block at g_page_ptr_b) - that is what g_flip_lock@0x8008e protects during
page flips, correcting the earlier suspicion it gated the mixer.

### 7.2 B2 audio = IRQ0-shared PCM driver (bonus architecture)
SoundInit@0x50033 (NO SOUND FX branch -> SoundStubInstall; success ->
SoundDriverInstall@0x685f0): driver struct @0x1276dc, rate 0x2b11 = 11025 Hz
(the SAME native rate EXW feeds DirectSound), PIT divisor 1193181/rate ->
0x1280b8. Real callbacks: SndDrvArmPit@0x68740 reprograms PIT ch0 (0x43/0x40,
divisor @0x82594) when armed - the audio sample clock and the 100.01 Hz game
tick SHARE IRQ0 HMI-style; SndDrvElapsedMsHiRes@0x686d0 = tick count * 1000 /
rate + (divisor - current PIT count) * 1000 / 1193181, monotonic-clamped -
vs the stub flat ticks*10. PcmMixerService@0x136e0 walks 20 channel records
(stride 0x26, bank 0x86a98 x 0x2f8) spawning/freeing sub-voices
(MixVoiceAlloc/MixVoiceFree/MixEventFeFf). g_snd_handles@0x8abf8 (176 B) is
RUNTIME-FILLED with sound handles - the earlier endgame-dispatch guess is
corrected; MissionRun tail compares them against the two music-loop handles.

### 7.3 Episode-loop progression (task b)
GameInit campaign loop (fresh: zone=1 mission=1 linear=0; boot call to
MapRoomSelect loads saves -> restore linear/slot/mask from record). Per
iteration: briefing (BriefingScreen@0x5498b, mode-2 BriefingMode2@0x5deb3)
-> MissionRun@0x57651 returns outcome (0 = completed) -> FUN_0005eaf9
post-mission hub (re-pins RNG 123456, stats, 5-tick waits) -> case-1
advance: g_campaign_linear++ ; g_campaign_mask |= 1 << (sub-1) ; if mask ==
full-mask[slot] (dwords @0x81d9a = {0, 1, 0xf x6}) then stage-slot++ and the
ZONE-COMPLETE cutscene (FULLFONT/FULLPAL + LOAD_UK|US.BIN + LOADPAL, the
200/100-tick fade loops seen in the dump tail); slot != 9 draws the next
stage title. Exit: linear > 0x1a (27 missions) or quit flag. IMPORTANT: sub
(g_sub_mission) does NOT auto-advance - the PLAYER picks it in MapRoomSelect
(mission select gated by the completed mask; slot1 -> sub=1, slot8+mission3
-> sub=2 hardcoded). Save records = 5 x 61 B @g_save_records 0x8b1d4:
{+0 mask, +4 stage-slot, +8 linear (word), +10 DAT_00125df4, +14 money
DAT_00126810, +18 word, +20 stats blocks copied from 0x91a54/0x918a4};
written by the 5-row save dialog in the MissionRun tail. MapRoomSelect also
loads BRF_{APPL,BANA,CAKE,DONU,EGG,FRYU,GRAV}.BIN per stage slot 2..8 +
SAVEICON + MAPROOM1/2.RAW loops - the per-stage map-room backdrops.
RESIDUAL: CLOSED in 7.7b (static arithmetic, playthrough NOT needed) - 25 distinct table indices and 27 completed missions are different counters; the gap = the two endgame completions at stage-slot 8.

### 7.4 Zone-letter dword[0] = 0x19 sentinel (task d)
Value 25 is NEVER produced: the minimum formula index is order[1] + 1 = 1.
The boot plants zone=1/mission=1 as code constants, and every consumer
indexes zone-param tables (0x80be4/0x80c04/0x80c24/0x80c44/0x80c64) by
DAT_0011f024 with values 1..8. Values 7/8 = special screens (code branches
on DAT_0011f024 < 3 / == 3 / != 7). Index 0 = padding/sentinel, not
intro/endgame state.

### 7.5 Mission pacing = present-paced, counters do not gate (task e) -> D23
MissionRun main loop LAB_00057947: 9 octile sensor distances to fixed points
(DistOctile@0x330e6 = max + half-min), unit hit-tests, mouse-region UI
cascade, unit purchase/dispatch against money DAT_00126810 (RandBelowB@
0x33147 = bounded RNG over RngStepB for enemy spawns), then PresentFlip ->
loop. PresentFlip@0x1066b = VESA page flip: bank pair {0,5}, display start
0 <-> 0x11ef38 (0x200), ISR lock + flip lock held across bank ops
(VesaSetWindow@0x12ac8 wraps 4f05 with the lock), WaitVRetrace@0x10856
double-poll of 0x3da bit 3 gated by g_wait_vsync, then a 0x96-dword cursor
block copy g_page_ptr_a -> g_page_ptr_b. VESA-off fallback = plain
WaitVRetrace. ZERO INT8-counter reads inside the mission loop. VERDICT: the
B2 sim/render iteration is vblank-locked exactly like EXW (D16); the 100 Hz
ISR is services-only. Video = VESA mode 0x101 640x480x8 requested
(g_vesa_mode_req@0x801ce), 64 KB window at A000 validated, granularity
shift recorded, LFB pointer captured @0x11f148, 640-byte row stride in the
blitters vs 320x240 mouse/logical space = 2x pixel scale; BankWrite64K@
0x12572 moves 64 KB through the A000 window. RESIDUAL: CLOSED in 7.7c/d - 4f02 sets BANKED 0x101 (LFB bit never constructed anywhere, BX = caller passthrough, g_lfb_ptr write-only dead); display start 0x200 = SCANLINE units.

### 7.6 Names persisted this run
BedlamWatcom:/BEDLAM.EXE: PresentFlip, PcmMixerService, MixVoiceAlloc,
MixChannelFind, MixEventFeFf, MixVoiceFree, SoundStubInstall,
SoundDriverInstall, SoundInit, RawSoundLoad, RawSoundPlay, WaitTicks100Hz,
SelectDrawBank, BankWrite64K, VesaSetWindow, VesaModeInit, MapRoomSelect,
MissionRun, DebriefScreen, BriefingScreen, BriefingMode2, DistOctile,
RandBelowB, SndStubResetTicks, SndStubElapsedMs, SndStubNop, SndSetPos80010,
SndDrvIrqTail, SndDrvElapsedMsHiRes, SndDrvArmPit (30 fns) + labels
g_ctr_snd_a/b, g_ctr_dead1/2, g_isr_phase, g_ctr_timeout, g_ctr_delay5,
g_campaign_linear/mask, g_stage_slot, g_sub_mission, g_save_slot/mask/
linear, g_money, g_save_records, g_zone, g_mission, g_mission_end,
g_page_state, g_page_bank_b, g_display_start_b, g_page_ptr_a/b, g_lfb_ptr,
g_vesa_gran_shift, g_vesa_mode_req, g_banked_video, g_flip_lock,
g_snd_drv_active/enabled, g_snd_service_arm, g_snd_handles (33 labels);
created functions at 0x12ecf/0x12ee4/0x12eef/0x607b0/0x686b0/0x686d0/0x68740.
Scripts: B2EpisDump.java (interrupted run, adopted), B2EpisClose.java,
B2EpisNames.java.
### 7.7 Residuals closed: campaign 25-vs-27, 4f02 banked-not-LFB, 0x200 scanline units, B2 fade chain (2026-08-18 residuals lane)

Scripts B2Residuals/B2Vesa4f02/B2LblFix/B2ResidVerify (all -process
BEDLAM.EXE -noanalysis, never re-imported); dumps b2-residuals.txt,
b2-vesa-4f02.txt, b2-resid-verify.txt (persistence re-check pass=14
fail=0). Provenance: [verified] = byte-dumped or decompiled this lane;
[inferred] = arithmetic forced by byte layout, marked per claim.

#### 7.7a Campaign tables, byte-pinned [verified]
@0x81d9a fullmask[8] = {0, 1, 0xf x6}; @0x81dba order[8] = {3, 0, 1, 5,
9, 13, 17, 21}; @0x81dda zone[27] = {25, 1,2,4,2,4,2,4,2,5,3,5,6,8,3,7,6,
6,7,6,5,8,3,7,3,7,8,8}; @0x81e46 mission[27] = {1,1,1,2,3,1,4,3,2,1,1,2,
1,1,2,1,3,2,2,4,3,2,4,3,3,4,3} (idx 0 of both = padding/sentinel). Name
pointer tables directly after mission: @0x81eb2 stage names {BootCamp,
Apple, Banana, Cake, Donut, Egg, Fryup, Gravy} (strings @0x8412c.., match
BRF_APPL..GRAV); @0x81ed2 zone names {BootCamp, ALPHA, BRAVO, CHARLIE,
DELTA, ECHO, FOXTROT, GOPHER} (strings @0x8415d..0x84183). Zone value 7
renders G, 8 = the special-screen family (no name entry); zone[0] = 25.

Stage i (0-based; fullmask/order/nameptr share the index domain) covers
formula idx order[i]+sub with sub = 1..4 player-picked and mask-gated:
stage 0 BootCamp (order 3, fullmask 0) idx 4..7; stage 1 Apple (order 0,
fullmask 1) idx 1..4; stage 2 Banana (order 1, fullmask 0xf) idx 2..5;
stage 3 Cake (order 5) idx 6..9; stage 4 Donut (order 9) idx 10..13;
stage 5 Egg (order 13) idx 14..17; stage 6 Fryup (order 17) idx 18..21;
stage 7 Gravy (order 21) idx 22..25. Union over stages 1..7 = exactly
{1..25} (only the Apple/Banana boundary overlaps, {2,3,4}); 28 slot/sub
selections map onto 25 distinct indices.

Campaign step list idx:zone.mission [verified, from the two tables]:
1:A1 2:B1 3:D2 4:B3 5:D1 6:B4 7:D3 8:B2 9:E1 10:C1 11:E2 12:F1 13:sp8-1
14:C2 15:G1 16:F3 17:F2 18:G2 19:F4 20:E3 21:sp8-2 22:C4 23:G3 24:C3
25:G4 (26:sp8-3 sits past the reachable set). Playable-file spread: A x1,
B x4 (missions 1,3,4,2), C x4, D x3, E x3, F x4, G x4, sp8 x2 in range.
The G steps (zone 7) have NO ZONEG directory in the corpus and the code
branches on zone != 7 - special-screen steps, not file-loaded missions.
Corpus consistency: ZONEA ships {1,2,4}+{6,7} and lacks MISSION3 exactly
because the campaign requests zone A only at idx 1 (mission 1); every
zone ships alt {6,7} = mode-2 (+5) of missions 1,2 and mission-table
values never exceed 4, so MISSION8/9 are never requested in any mode.

#### 7.7b The 25-vs-27 residual RESOLVED [verified + inferred hop]
Different counters: 25 counts DISTINCT formula indices (stages 1..7);
linear counts COMPLETED missions. GameInit loop head: continue while
linear <= 0x1a, i.e. exit after the 27th completion. Stage-clear path:
fullmask {1, 4 x6} = 25 completions carries stage_slot to 8; at slot 8
the formula reads order[8] OOB = zone[0] = the 25 sentinel [inferred -
0x81dba + 8*4 = 0x81dda exactly, and the decompiled formula has no
bounds check], giving idx 25+sub = 26..29 (idx 26 = sp8-3). Two
completions there make mask = 0b11 = 3 = fullmask[8] read OOB = order[0]
= 3 [inferred, same layout argument] AND linear = 27: slot reaches 9 (the
slot != 9 stage-title branch goes quiet = finale) and the loop-head
exits. Minimum full game = 25 + 2 = 27 completions. No contradiction with
25 distinct indices: the Apple/Banana shared indices {2,3,4} are
completable once per stage and count linear each time. REMAINING
(save-file, non-blocking): whether stage 0 BootCamp (fullmask 0 - advance
needs mask==0, which any completed bit makes impossible) is reachable in
a fresh campaign, and the menu-pick semantics (MapRoomSelect case 2
plants stage_slot = 2 with fresh money 1500 + mode 2; the off-by-one vs
0-based table indexing needs a save record to settle).

#### 7.7c 4f02 = BANKED 0x101, LFB never enabled [verified 3-way]
(1) The 4f02 site (0x12439, inside VesaModeInit@0x12290) passes caller BX
through verbatim: PUSH EBX / MOV AX,0x4f02 / INT 10 / POP EBX - no mode
construction, no OR 0x4000. (2) 0x4101 appears NOWHERE: zero hits across
the 671-function decompile sweep and every listing dump. (3) g_vesa_mode_
req@0x801ce (planted 0x101) and g_lfb_ptr@0x11f148 (captured from the
4f01 info block) are write-only - zero readers (xref + decompile census);
the LFB pointer is captured-but-dead diagnostic data. All render/copy
paths run through the A0000 64K window with 4f05 bank switches
(VesaSetWindow@0x12ac8 wraps bank reg 0x80034; page mappers 0x128df/
0x12960/0x129f2 wrap every 64K boundary: CMP reg,0xb0000 / CALL 12bc0 /
SUB reg,0x10000). Sec 7.5 residual CLOSED.

#### 7.7d Display start 0x200 = SCANLINE units [verified 2-way]
PresentFlip alternates display start 0 <-> 0x11ef38 (=0x200) while
alternating draw page {0,5}: page B = bank 5 = byte 5 x 0x10000 = 0x50000,
and 0x200 x 640 B/scanline (mode 0x101 pitch) = 0x50000 exactly. The 4f07
call form (AX=4f07, BX=0, CX=0, DX=start) is set-display-start with DX in
scanlines; only the scanline reading satisfies both facts simultaneously.
VesaModeInit plants g_display_start_b=0x200 together with g_page_bank_b=5
before the first flip. Sec 7.5 residual CLOSED.

#### 7.7e B2 fade chain (the 0x126c8 satellite + 0x11ef88 gate) [verified]
B2FadeStep@0x126c8: 768-channel stepper, 8.8 fixed acc += step pairs at
0x9f05c (short pairs), DAC record bytes = acc>>8 with byte 0 forced to 0,
then B2DacUpload@0x1082c (out 0x3c8/0x3c9 from the 0x9f058 record), then
g_b2_fade_ticks_left@0x11ef88--. Serviced by Int8TickHandler every
OTHER tick - the call sits inside the (g_int8_ctr0 & 1) sub-block shared
with the 50 Hz cursor redraw (ISR decompile in b2-epis-close.txt;
corrects the first 7.7e draft, which read the rate at the bare 100.01 Hz
tick) = 50 Hz, IDENTICAL to EXW FadeStep@00425901: both builds fade 10
steps in 200 ms (every B2FadeSetup call site passes 10). No cross-build
divergence; fade is presentation (D17 boundary), no engine change. B2FadeSetup@0x3046c
(target EAX over EDX ticks; instant path via B2FadeCancel when
g_tick_installed == 0 - the fade-enable gate IS the tick-installed flag,
which is why the 0x125e18 mislabel mattered); B2FadeCancel@0x1081a (CLI /
countdown=0 / DAC upload / STI); B2DacRead@0x10802 (in 0x3c7/0x3c9 - fades
interpolate from the LIVE hardware palette); B2FadeWait@0x3439b (spin
while countdown > EAX). Labels g_b2_fade_ticks_left@0x11ef88,
g_b2_fade_state_ptr@0x11f05c, g_b2_dac_record_ptr@0x11f058 persisted;
B2LblFix removed the two mislabels (0x125e18 g_tick_installed and 0x801ce
g_vesa_mode_req primaries restored). B2ResidVerify re-check: 14/14.


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
