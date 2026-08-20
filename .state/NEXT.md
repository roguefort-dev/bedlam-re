# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P5] Boot attract sequence (LOGO/GTLOG, D32 not-done tail): play
   the region-variant publisher movies on the Boot scene per
   RE-EXW-GAMETHREAD.md:140-150 (FUN_0044567c order: GTLOG then LOGO,
   DAT_0046ae64 selects _UK/_US - selection names already modeled in
   bedlam-game movies.rs gtlog_name/logo_name, D32). RE the EXW boot
   arm first (what owns the plane between the two movies and before
   TITLE.SMK; palette handling per the D31 standing-palette argument),
   then wire through the D31 movie lifecycle (inert-until-scene,
   dropped on exit; scene-hash untouched). Corpus facts from
   tests/smk_corpus_gate.rs (frame counts/rates already pinned).
   Provenance + confidence tags on every claim.

## Backlog (not yet started)
- BRF_DROP.SMK as the boot-camp/endgame brief fallback (play site not
  yet located; D33 not-done list).
- Add a native executable shell: window/surface, input adapter, fixed-step
  clock, and platform audio output (the movie plane + stream bus are
  ready to consume from bedlam-platform).
- Build the first menu and ZONEA/MISSION1 playable vertical slice.
- OPERATOR NOTE (carried from the 13:43 blocked-tag lineage): MANIFEST-2.sha256
  at the repo root mismatches 470 files - it documents a different tree
  snapshot (its BEDLAM.LOG entry is the sha256 of an EMPTY file). Re-anchor or
  delete it. It was never used as the integrity gate: MANIFEST.sha256 is the
  canonical AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
- 2026-08-20: P5 FULLFONT loading-text glyph pass COMPLETE (D35, this
  commit): the four LAB_0041c69e text draws + the font-ramp copy run
  in GameHost - bedlam-game font.rs reproduces FUN_0043c87c (two-pass
  measure/draw, x0 = 0x140 - total/2, space +9 / glyph w+2 advance,
  RLE16 transparent blit, hotspot dy-row/dx-col baseline anchoring,
  FUN_00410493 accent remap incl. the e-/o-diaeresis dash quirks,
  overlay glyphs at entry 0x82+0x6b+id); bedlam-assets language.rs
  parses the LANGUAGE.* [MENU_ITEMS] table (the four strings are
  entries 0x45/0x46/zone+0x51/0x58 - DAT_0046bc4c/7c/0046bfdc =
  table base + idx*0x30); pal.rs parse_font_ramp pins the 98B
  FULLPAL ramp (lead e0 20 = entry 224 count 32) that replaces
  fade-target entries 224..=255 (EXW order: 0x3f transient commit ->
  draws -> ramp copy -> FadeSetup; D34 row/y swap CORRECTED: 0x82 is
  the glyph base, 150/180/210/260 are ROWS). Host:
  load_loading_font(font,fullpal,language) stages inert; corpus gate
  tests/font_gate.rs pins FULLFONT 390 entries (333 glyphs + 57
  empty, ASCII pixels exactly {0} U {233..=244}, dy set {0,5,10,15}),
  FULLPAL + all six LANGUAGE files, and re-measures the drawer
  arithmetic over the real bank widths. 15 new units; 326 workspace
  tests green / 0 failed, fmt + clippy -D warnings clean,
  MANIFEST.sha256 verified BEFORE and AFTER the corpus runs. WIP of
  interrupted predecessors adopted (Ghidra RE artifacts + font/
  language/gate modules; completed: host-test rename tail, host
  font-staging unit, loading.rs typo, clippy, docs). Worker
  315d2af1 (claim 1).
- 2026-08-20: P5 post-cutscene loading FLOW COMPLETE (D34, commit
  d834f08): the EXW LAB_0041c69e zone-transition tail as a
  presentation-only GameHost flow - bedlam-game loading.rs
  LoadingFlow (Staged->Between->Loading): BETWEEN.BIN entry 0 owns
  the Cutscene plane once the cutscene movie ends (standing host
  palette); the region-variant loading screen (LOAD_UK/US.BIN +
  LOADPAL/LOADPALU.PAL, path-selection only) owns the Select plane,
  10-step 20 ms 50 Hz fade from black on the movie x240-us
  accumulator (chunking-invariant), DAC tail 224..=255 forced 0x3f
  (bytes 0x2a2..0x301, boundary-exact), text row pinned y=0x82
  x=150/180/210 (+260 zone 6, stage-1 reconciliation) as TextRow
  flow state for the queued FULLFONT pass; endgame arm (MAX_STAGE)
  drops the flow; skip-advance still runs the loading screen;
  scene-hash chain untouched. 14 new units (8 module + 6 host incl.
  FULL_MASK campaign walk + hash isolation). Engine WIP of
  interrupted predecessor 3977d55d adopted (snapshot
  /tmp/opencode/wip-snapshot-f807449c/), doc-grammar fix + D34
  DECISIONS entry by this run. Workspace 311 tests green / 0 failed,
  fmt + clippy -D warnings clean, MANIFEST.sha256 verified BEFORE and
  AFTER the corpus runs. Worker f807449c (claim 1).
- 2026-08-20: P5 loading-screen asset path COMPLETE (8cc4951):
  BETWEEN.BIN + LOAD_UK/LOAD_US.BIN + LOADPAL/LOADPALU.PAL pinned in
  bedlam-assets tests/loading_gate.rs through the existing validated
  decoders (sprites::parse_bin_images + pal::parse_vga770 - no decoder
  changes owed). Facts pinned: all three BINs are SINGLE-IMAGE banks
  (count=1, off=4, flags=0x0003 = RLE16|hotspot, hot=(0,0), 640x480,
  decode ok, plane sha256 BETWEEN=6c706182.. LOAD=2d100f8b.., file
  sha-head pins); LOAD_UK.BIN == LOAD_US.BIN and LOADPAL.PAL ==
  LOADPALU.PAL BYTE-FOR-BYTE (region split is an EXW path selection
  only - decode parity region-independent; doc note added to
  Region::loading_pal in bedlam-game movies.rs); palettes exactly
  770B, 244 distinct colors, entry0 black/entry1 white, expanded RGB
  plane sha256 7e74c681... 1:1 blit verdict (640x480x8 native Frame -
  no letterbox/scale, unlike the 640x320 TITLE movie). Ignored regen
  test = the only documented regeneration path. Workspace 297 tests
  green / 0 failed, cargo fmt clean, clippy --workspace --all-targets
  -D warnings clean, MANIFEST.sha256 verified BEFORE and AFTER the
  corpus-touching runs. Worker ffd80ad1 (claim 1).
- 2026-08-20: P5 shop + briefing backdrops COMPLETE (D33, commit 1b3ef85,
  landed by worker a1ad7346 which died after push, before this queue
  rewrite; adopted + independently re-validated by run ed15e708 under
  claim 1): GameHost load_shop/load_briefing bind Scene::Shop /
  Scene::Brief onto the D31 movie lifecycle (inert-until-scene, dropped +
  stream cleared on exit); movies::briefing_name_for_slot maps the hashed
  episode slot onto the BRF corpus (stages 2..=6 -> BRF_{B..F}, sub =
  lowest-unset mask bit + 1 per Episode::complete open Q5; boot camp +
  endgame/ceiling stages -> None - no BRF_A/BRF_G exists). 6 new units
  (3 selection incl. corpus-domain cross-check, 3 host lifecycle through
  the FULL_MASK campaign). Re-validation: workspace 294 tests green /
  0 failed (all 6 D33 units pass), cargo fmt --check clean, clippy
  --workspace --all-targets -D warnings clean, MANIFEST.sha256 verified
  BEFORE and AFTER the corpus-touching test runs. No new commit owed:
  1b3ef85 already pushed; .state bookkeeping rides the next substantive
  commit per the no-stand-down-commit rule.
- 2026-08-20: P5 cutscene movies + corpus inventory COMPLETE (D32, this
  commit): game-data SMK corpus PINNED in bedlam-assets
  tests/smk_corpus_gate.rs (34 files: formats, frame counts, rates,
  ring flags, y-scale None corpus-wide, the one DPCM 1/8/11025 audio
  shape; listing-to-table equality both ways; ignored regen helper).
  Reject-or-map verdict per D31: every file MAPS onto the existing
  playback path - nothing rejected, no y-scale or resampling owed, all
  periods exact on the x240-us accumulator grid. bedlam-game movies.rs
  selection module (D17-b hash-free, host byte-source-free):
  cutscene_name (ZONEDONE/END at stage >= MAX_STAGE, EXW
  LAB_0041c69e pre-increment vs Episode::complete post-increment
  reconciled), Region (DAT_0046ae64) backing LOAD_UK/US.BIN +
  LOADPAL(U).PAL and LOGO/GTLOG variants, briefing_name over
  BRF_{B..F}{1..5}. GameHost::cutscene_name + load_cutscene wire the
  Cutscene scene (D31 lifecycle: inert-until-scene, dropped on exit);
  selection + lifecycle unit-pinned through the FULL_MASK cadence.
  Workspace 257 tests green, fmt + clippy -D warnings clean,
  MANIFEST.sha256 OK before AND after corpus runs. Interrupted
  predecessor WIP adopted per AGENTS.md (snapshot in
  /tmp/opencode/wip-snapshot-1bd01455/).
