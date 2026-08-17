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

## Census (game-data-2, 989 files, tools/inspect unmodified)
- BIN sprite banks: 191/191 parse, 11,008 PNGs — SAME RLE16 + dir format as B1
- CGR tile banks: 36/36 parse, 4,608 tiles — SAME dual codec (raw/byterle)
- MAP/TOT/COL grid16: 102/102; DAT grid8: 34/34; 34 plane0 map renders
- TRT/MRK/POS/PAD/PTH/NME/BDG: 34 each — same table formats
- RAW pcm8: 106; PAL: 83 (+ 10 variants); TRN: 16; SMK: 5; MIN 7; LNK/LNG 43+7
- BDL: OPTIONS + CONFIG parse (2 unknown-variant = CONFIG.BDL 61B + MIRAGE dupe)
- 89% first-pass parse with ZERO Bedlam-2-specific code.

## Divergences vs Bedlam 1 (data layer)
- 6 zones (A-F) vs 7; missions per zone differ; MISSION5 mostly absent
- No MRW/MRS music scores; audio = 106 RAW (LOOPS dir = music?) + HMI .386
- Single LANGUAGE.ENG (no multi-language set)
- DOS exe only (BEDLAM.EXE 672KB LE/Watcom + DOS4GW) — no Win95 build in this rip
- UNIVBE/UVCONFIG + SETUP present (VESA SVGA 480p-1440p native support)
- New: editor MISSION?.BIN zone banks inside EDITOR/ZONE? dirs

## Implications
1. bedlam-assets transfers nearly 1:1 — same formats, second corpus for fuzz/coverage
2. Second RE target: BEDLAM.EXE (DOS canon, same HMI stack) for logic divergence
3. Engine must parameterize content (zones/missions/units), not hardcode B1 counts
