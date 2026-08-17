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
- eip=0x56a60 LINEAR (an object-relative reading would exceed obj1 vsize ->
  self-contradictory; linear lands at obj1+0x46a60, file 0x7d860, where a probe
  this run found valid i386 code incl. an absolute store into obj2:
  89 0d 10 5e 0a 00 = mov [0x000a5e10], ecx). esp=0xb04ee linear (obj2+0x304ee).
- Data pages: file [0x36e00, 0x36e00+0x6d30f); 0x6d48f bytes avail (0x80 slack).
- Fixups NOT pre-applied: fixup page table hdr+0x2b7 (111 dwords; first entries
  0x0,0x48b,0x8eb,0x1116,0x1646), records from hdr+0x473, fixup_size=0x3201a
  (~205 KB). num_impmods=0 (no imports), debug_off=0, nonres table absent,
  autodata_obj=2, num_preload=0.
BEDLAM.EXD (B1 DOS, game-data/BEDLAM/BEDLAM.EXD, 655,133 B): same class - LE,
same 2-object layout with identical bases 0x10000/0x80000, 107 pages, eip=0x4fbb0
(linear), esp=0xa583e, fixup_size=0x309c6. One loader serves both.

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
   (ii) entry at 0x56a60, (iii) fixup labels present, (iv) decompile at/near
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

Fallback if BOTH loader paths fail (cost ~a day): Python applier using
exeflat.h semantics - pages sequential from 0x36e00 into [0x10000..]+[0x80000..];
fixup page table (111 dwords) + records enumerate every relocation; internal
fixups rewrite placeholders with target object VAs (bases absolute in-file, no
slide needed for static analysis); emit a flat memory image, import as raw
binary at 0x10000 with the cspec, then a postScript to set entry + create the
obj2 zero-fill tail. This is the honest version of the PLAN raw-binary fallback.

[provenance: BEDLAM.EXE/EXD header parses = this run, python over the corpus
(read-only; both manifests verified after); loader facts = upstream README,
releases page, extension.properties, build.gradle (fetched 2026-08-18);
exeflat.h layout = open-watcom-v2 master; Ghidra stub = javap of Base.jar in
ghidra-12.1.2-watcom; case study = alexbevi.com 2026-03-14. confidence: high
for file facts (structurally cross-checked 3 ways), moderate for 12.0.1-on-12.1.2
compat until the smoke test runs.]

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
