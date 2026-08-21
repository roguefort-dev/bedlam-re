- CLOSED 2026-08-21 (P4 GAMEPAL mission present tail COMPLETE, commits
  663ddba + 7c25bfd, worker 1776dc60 claim 1): the mission viewport
  presents in color. DESIGN-GAME sec 11 amended (design commit
  663ddba) then implemented (7c25bfd): GAMEGFX\GAMEPAL.PAL (770 B,
  the parse_vga770 LOADPAL family; RE-EXW-MISSIONVIEW sec 6 GAMEPAL
  -> 0x4edbf8, RE-EXW-SIM sec 7c.3 the 0x302-B mission-load copy)
  joined the Mission fetch set in the GAMEGFX tail - SINTABLE,
  DANTE, GAMEPAL, then MRK (10 files) - folds with the exact
  loading_palette rule (>>2 lossless on 6-bit file values) and OWNS
  the mission plane: MissionScene carries the folded [Vga6; 256],
  plane() returns its own palette, render_now no longer passes the
  host stand-in, the frame palette IS GAMEPAL with palette_dirty
  every frame (MovieFrame seam; the indexed->RGBA window upload
  stays platform-side). Signatures: MissionScene::stage +
  GameHost::load_mission grew gamepal; the chain passes bytes[8]
  GAMEPAL, bytes[9] MRK. Corpus gate re-pinned ONCE (documented in
  the gate header): spawn frame a79fcada30ec5e50, mid-walk
  1b75b68ce66019e1; sim pins 36ddc86345c8351c / f35db41f0efb858d and
  the render-gate pins UNCHANGED; new structural pins frame.palette
  == folded GAMEPAL + palette_dirty + 254/256 non-black (entry 1 =
  6-bit 0x3E,0x3A,0x39). Headless smoke 25 fetches (GAMEPAL.PAL
  770 B) two-run byte-identical exit 0; parity harness
  byte-identical to the D28 anchors (chain 0xcae25cd08d7cbc08, sim
  0x72979d5d9dedc832, frame 0x87263f149564ad25, audio
  0xc862e45d2e95ad29); all workspace tests green; fmt + clippy -D
  warnings clean; release ok; MANIFEST verified after the corpus
  reads; D46 records the choices. P4 slice remaining: audio output
  rates, the Escape-exit window fix (queued next).
- CLOSED 2026-08-21 (P4 mission SCENE step COMPLETE, commits 26a11ef
  + e6de264, worker 74fa370e claim 1): the playable-slice composition
  landed. bedlam-game/src/mission.rs MissionScene per DESIGN-GAME
  sec 11 (design committed by predecessor a835cefc as a6317c5, whose
  WIP - the shared dat_plane_bytes loader + the public
  project_robot seam - was adopted and landed first as 26a11ef):
  staging = Terrain::from_mission_bytes + AngleTable(SINTABLE 2..66)
  + MissionSim seed 0x1E240 + robots_per_player(zone) MRK spawns +
  staged markers (the 0x46cbe0 network seam) + MissionView over the
  swept PRE-PAD planes with DANTE staged; lifecycle = movie pattern
  (inert until Mission, activate fixes the camera at robot 0 Q5,
  drop after leaving); per-frame = pointer integrate -> left-EDGE
  click seam (viewport x < 0x1E0, enqueue-projection hit box 0x20,
  nearest octagonal wins, arm AT the robot) -> advance_frame;
  present = enqueue_robots -> draw_terrain -> present_window ->
  480x480 at canonical (0,0) + black sidebar, one render per pump.
  Host: load_mission/mission_slot/mission_asset_names (episode
  arithmetic), the tick-loop drive, sync_mission, mission plane
  first in render_now. Shell chain: the Mission 9-file fetch set +
  stage_scene wiring + the GameGfxSource EDITOR tier for '/' names;
  headless smoke = 24 fetches, 20 mission pumps, two runs
  byte-identical. Corpus gate tests/mission_scene_gate.rs: scene
  frames pinned spawn 51ef4fe93eaaed77 / mid-walk 7bae11a5c7f34ab6
  + sim hashes 36ddc86345c8351c / f35db41f0efb858d, scripted
  click->arm at the projection (tile (21,73), snap to origin,
  state 3), walker state 4 live anim, sidebar black, two-run
  identity; the render-gate pins stay untouched. Parity harness
  output BYTE-IDENTICAL to the D28 anchors (the mission is inert on
  unstaged paths). D45 records the [design] choices. 422 workspace
  tests green, fmt+clippy clean, release ok, MANIFEST verified,
  pushed. P4 slice remaining: the GAMEPAL/window present tail +
  sidebar (queued next), audio rates, Escape-exit fix.
- CLOSED 2026-08-21 (P4 mission RENDER half 2 - ENTITIES, commits
  007237e + 186050b, worker e08e64c2 claim 1): the robot entity
  overlay decoded and wired onto the pinned frame. RE notes
  RE-EXW-MISSIONVIEW sec 5b-5d: per-frame bucket-grid clear (ECX
  0xa200 @0x46cdbc) + arena reset; FUN_0040798e node/bucket/
  insertion semantics (48-B nodes, bucket (wx>>5 - camTx +9)*4 +
  (wy>>5 - camTy +9)*0x90 + layer*0x1440, sort = wx+wy ascending,
  stable after equals, head-insert); the terrain-loop flush site
  (per cache cell per layer, gate 0..0x24, next @+0x20); FUN_0040179b
  asm-authoritative (directory entry 2+4*id with the fmt word SKIPPED,
  forced u16-RLE, literal runs RAW-copied with NO zero-skip - mode
  0x130 paints 0xFF, 0x12c/300 plain, 0x12d/0x12e TXPAL1 64-KiB
  composition / 0x12f DARKPAL XLAT only with the water flag on);
  the robot loop field map (sx/sy iso projection + 0x23f clip,
  shield sy-0x48 mode 0x12e, body DANTE[anim], variant/overlay/
  +0x20 sprites; spawn defaults => DANTE[anim] + DANTE[0x20]);
  SIM sec 3 correction: the deploy countdown is u16@+0x16, +0x14 is
  the frame-base word. Engine: mission_view.rs SpriteList +
  RobotView + enqueue_robots + flush_node + the draw_terrain flush;
  corpus gate: ZONEA/MISSION1 spawned robot + order-walking second
  robot from MissionSim on real bytes drawn with real DANTE.BIN
  (160 sprites) - spawn frame pinned 8d2c559df035b75b, mid-walk frame
  8804f9deec6b1fee, terrain pin 90a9e929eea24ced kept as the
  no-entities regression pin. 5 hermetic entity tests; 413 workspace
  tests green, fmt+clippy clean, MANIFEST verified, pushed.
- CLOSED 2026-08-21 (P4 mission RENDER half, commits 02363f6 + 889d6b0,
  worker b9aaaa38 claim 1): the isometric viewport draw chain decoded
  and rendering ZONEA/MISSION1 to a hash-pinned frame. New
  docs/RE-EXW-MISSIONVIEW.md (ghidra dumps exw-missionrender{,2,3}.txt,
  scripts ExwMissionRender{,2,3}.java, -process -noanalysis x3):
  init_tiles@00407e11 = the 36x36 2:1 iso viewport cache at
  DAT_004ede24 (grid origin (0x130,-0x100), +32/+16 steps, sticky
  anchor 17, 467 cells) + the TOT 8-plane word mirror into the
  0x1e-stride type-DB records at 0x4796bc (8 words + 8 seen bytes at
  +0x10 + zero-filled tail); LNK = the PER-FRAME tile ANIMATION link
  (word -> LNK[word] walked and memoized back every drawn frame);
  BIN = MISSION{A..G}.BIN the terrain sprite bank (u16 count + u32
  offsets relative to entry 2+4*id); FUN_00401471 blit codec (fmt 0
  raw 64x64 skip-0 / fmt 1-3 u16 RLE bit15-ctrl bit14-eol low12 /
  fmt>=4 u8 RLE bit7-ctrl bit6-eol low6+1; stride 640; XLAT remap);
  FUN_00403938 terrain loop (camera tiles, 8-layer bottom-up walk,
  0x5000/level, seen-chase columns, 0x59b00 draw cap, off-map edges
  via FUN_00408030 per zone); sprite-list enqueue FUN_0040798e +
  flush FUN_0040179b (entity overlay seam, decoded not yet wired);
  present FUN_00401107 = the 480x480 window at buf+0xa040 + fine-cam
  offset (camera 0 -> (96,64)). Engine: bedlam-render mission_view.rs
  (MissionView + DrawParams + present_window, hermetic, per-write
  bounds) + corpus gate mission_view_gate.rs: cache geometry/anchor
  pins, deck mirror + seen semantics, codec pixels on sprite 0,
  one-LNK-step-per-frame walk (visible tiles only - off-camera words
  frozen, layer-0 cap respected - faithful), frame hash pinned
  90a9e929eea24ced (camera (0,0), frame 0), two-run byte identity,
  zone-0 fixed edges vs zone-1 stream sensitivity. Corrected en route:
  the cache anchor is 17 (first in-bounds cell (12,4)), not 21; TOT
  plane stride is the standard w*h*2 (decompiler artifact fixed from
  asm). 407 workspace tests green, fmt+clippy clean, release build
  ok, MANIFEST verified, pushed.
- CLOSED 2026-08-21 (P4 slice tail, commits 5381bea + c4f615a +
  055879e, worker d8c46c88 claim 1): the mission file-load +
  table-build pass decoded and wired. docs/RE-EXW-SIM.md amendment 7c:
  load_mission@0041dc5a (EDITOR\ZONE{x}\MISSION{n} / zone-level path
  prefixes from build_mission_paths@0044670c; TOT/DAT/CGR/BIN/MIN/LNK
  arena loads; map w/h from the TOT header; y-line table 0x4ea900 =
  y*w for y in 0..=h, z-base 0x4eaacc = z*w*h for z in 0..=7; >=0x80
  sweep planes 0..6; PAD records staged 8-byte and written
  DAT[kind*w*h+y*w+x]=0xFF with NO bounds check; CGR height byte at
  2+4(type-1)+dir[type-1]+6+(sy<<5)+sx - RAW 1024-byte 32x32 height
  maps, NO codec, correcting FORMATS-MISSION 18; MRK word 3 = spawn
  z LEVEL feeding z=word3*0x20-1, robot i takes record i verbatim;
  the order armer FUN_004247b5 has a single caller, the robot-sprite
  click family 0x433cbc - the verified move producer stays the
  order/walk path, and no shipped mission spawns two markers within
  the 6-tile order radius, so a second walker on a real map is a
  staged marker, exactly what the network override 0x46cbe0 does).
  FORMATS-MISSION rows updated (DAT/MRK/PAD/CGR semantics confirmed).
  Engine: Terrain::from_mission_bytes (hermetic loader rules) +
  corpus gate engine/bedlam-core/tests/mission_corpus_gate.rs - ZONEA
  25x75 loader pin (deck floor z 31, type-37 wall column reads z 1 =
  climb 30 = the real-map wall, PAD mark materialises), MRK[0]
  (21,73,1) spawn settle z 31, staged second robot order->walk 4
  tiles east on the real bytes (arrival snap from the west lands one
  tile short of the target origin - faithful 0x1400-radius + grid-snap
  semantics), state hash pinned at spawn/arm/arrival with the 7-frame
  EXW cadence + two-run identity, and ZONEB/MISSION1 MRK[0] (27,71,3)
  settling at z 95 on the 3-deep deck stack. All workspace tests
  green, fmt+clippy clean, release build ok, MANIFEST verified,
  pushed. P4 slice remaining: the isometric viewport RENDER half -
  queued as the next Now item (init_tiles@00407e11 + the draw chain).
- CLOSED 2026-08-20 (P2d sim-tail slice, commits c33f615 + 6280857,
  worker 778d091a claim 1): the mission-sim seam for the P4 vertical
  slice. docs/RE-EXW-SIM.md amendment 7b re-verified the contested
  facts from the binary (move_is_possible per-probe climb refs =
  probe_z[i] sar-signed with 0xFFFF = -1, no writes on any probe
  fail; dist_octagonal abs's BOTH args - always >= 0; armer snap =
  tile ORIGIN tx<<13 with no +0xF00; spread table slots 0..8; spawn
  settle best-effort - seeds L*0x20-1 can never settle a tall floor).
  engine/bedlam-core mission.rs adopted from the interrupted e1eb0092
  WIP and driven 6/9 -> 9/9: Terrain (DAT planes + CGR height
  sprites, get_z_pos search/latch/0x1F rule), Robot record subset,
  Order + spread claims, MissionSim 6-phase frames + order-window
  tick, robot_move diagonal/slide/move_x_y_who, move_possible
  per-probe, state hash over the sec-7 coverage list. 38 workspace
  test binaries green, fmt/clippy -D warnings clean, release build
  ok, MANIFEST verified x2, pushed. P4 slice inputs now complete
  except the mission file-load/table-build pass (RE-EXW-SIM sec 9
  item 1) - queued as the next Now item with the ZONEA/MISSION1
  render + one squad move.
- CLOSED 2026-08-20 (P4-menu engine step, D42, commits 57413b0 +
  0a10a54 + 7ff713e, worker 26ccbd31 claim 1): the D41 title-menu
  findings implemented. bedlam-game menu.rs = TitleMenu (builder
  semantics for menus 1/2/3/5 with count word + 7 slots, strip hit
  test, hover/click SFX debounced 4 ticks, bottom-anchored draw with
  the dual glyph bases - font.rs from_bank_at/draw_at, name entry
  with the 0x8e cursor blink + explicit typing API + GOD default,
  attract replay at idle >= 0x300 via MoviePlayer restart/finish,
  menu-1 actions incl. start (seed 4000-diff*500, cached on the
  host) and quit-confirm). GameHost: load_title_menu staging
  (LANGUAGE + FULLFONT + FULLPAL + MENU1/MENU2 RAW as instruments
  0xE0/E1), the menu OWNS the Title input path (fsm fed neutral
  frames - hash-isolated, unit + corpus pinned), staged-inert
  lifecycle, menu plane after the title movie. Shell chain: Title
  fetch set = TITLE + LANGUAGE + FULLFONT + FULLPAL + MENU1/2,
  GameGfxSource SOUND/SFX tier. Corpus gate tests/menu_gate.rs
  (MENU_ITEMS table, difficulty cycle, strip geometry, green 233..=
  244 vs blue 244..=255 ramp pin end-to-end, start handoff, SFX
  audibility, TITLE.SMK restart, scripted two-run byte-identity).
  393 workspace tests / 0 failed, fmt + clippy -D warnings clean,
  headless smoke two runs byte-identical, parity IDENTICAL to the
  D40 baseline 143e60d, MANIFEST OK x2. Open: backdrop content
  (RE-EXW-TITLEMENU sec 8), HOF/credits/save/coop stubs, CONFIG.BDL
  writer, OPTIONS music. Remaining for the P4 slice exit: ZONEA/
  MISSION1 render + one squad move (needs P2d sim tail).
- CLOSED 2026-08-20 (P4 native shell step 2, D40, commits 58eb8a6 +
  c48cd91 + 143e60d, worker e76159bb claim 1): platform audio output.
  cpal 0.18.2 (bedlam-shell only; mixer stays hermetic, un-hashed):
  bounded stereo-frame ring (4096 frames; full = drop OLDEST, underrun
  = exact [0,0]) behind a poison-tolerant mutex - the ONE thread
  crossing; window loop the ONLY producer (watermark fill 736 frames
  after each pump batch), cpal callback the only consumer. Device
  config pinned at the native 11025 Hz when any supported range
  contains it (stereo > mono > other; this machine's Pulse/ALSA
  default accepted 11025/2ch live - #[ignore]d probe), else device
  default through a Q16 nearest-neighbor frame stepper (4x = exact
  repeats; 48k/8k step values + sample-hold positions unit-pinned);
  mono floor-average (l+r)>>1; formats via dasp conversions; no
  device = stderr note + silent run, never fatal. Headless smoke now
  drains 184 frames/pump off the host bus (110400 = 600x184, 158092
  non-silent samples) - scene/frame hashes IDENTICAL to the pre-
  change binary, two runs byte-identical, MANIFEST OK x2, workspace
  366 tests / 0 failed, fmt + clippy -D warnings clean. Next per
  queue: menu/ZONEA/MISSION1 playable vertical slice (P4 exit).
- CLOSED 2026-08-20 (P4 native shell step 1, D38/D39, commit 493fbd5,
  landed by the watchdog repair agent after a step-cap death spiral):
  bedlam-shell crate = window + surface + fixed-step present loop.
  FixedStepClock (pure u128 integer banking, anti-spiral clamp 4,
  surplus dropped not fast-forwarded); input seam map_physical_key
  pinned (winit KeyEvent has a pub(crate) field - NOT constructible
  outside winit; predecessor test rewritten); D31-D37 chain fetch
  layer (scene_assets + stage_boot/stage_scene); env-gated (--window
  / BEDLAM_SHELL=1) winit 0.30.13 + wgpu surface host (Arc<Window> ->
  Surface<'static>, Fifo vsync, D20 PARITY present); headless smoke
  (600 fixed pumps, scripted campaign walk, two runs byte-identical,
  fetch set exactly the 10 D31-D37 corpus files); two-tier
  GameGfxSource (GAMEGFX/<name> then <root>/<name> - LANGUAGE.ENG at
  install root). bedlam-platform +ParityGpu::new_for_surface. The
  WIP survived FOUR GLM workers killed at the opencode2 step cap
  (orchestrator default agent, steps:60, edit denied) - cumulative
  work by 486a18e1/8d2f7acc/3a5e5f9e/f24c9332, fixed (impossible
  KeyEvent test, saturate-bank assertion, usage string) + verified +
  landed by repair agent 410671: 356 workspace tests green / 0
  failed, fmt, clippy -D warnings, MANIFEST OK before AND after.
  CONTROLLER FIX (same repair): nudge workers now launch with
  --agent build (no step cap, edit allowed); step-cap truncations
  classify as 'step-cap' and no longer feed the taskfails/cooldown
  spiral; the llm-watchdog check prompt flags the signature. Next per
  queue: native shell step 2 (cpal audio output).
- CLOSED 2026-08-20 (P5 BRF_DROP briefing intro pair, D37, commits
  3a2981d + bba01fe + 40b3700): the BRF_DROP play site located and
  wired - the EXW briefing screen (FUN_0043d00b; RE corrected the
  prior gameplay-advance gloss) opens BRF_DROP.SMK FIRST at every
  movie-enabled briefing (asm 0043d447..0043d490), one full-screen
  pass, then the constructed BRF_{zone}{level}.SMK backdrop ring
  until UI exit (letter = zone + 0x40, zones 2..=6 = B..=F; D33
  open note resolved; open failures fatal; GO arms after handoff).
  Engine: bedlam-game brief.rs BriefIntro Staged->Drop->Backdrop
  (drop hard-capped frames-1, starvation-proof; backdrop ring
  unbounded; entry audio at start + handoff); GameHost
  load_briefing on the D31 lifecycle (inert-until-Brief, drop +
  stream clear on exit, hash isolation unit-pinned); latent D31
  MoviePlayer ring-Last bug FIXED (rings froze at their first
  cycle end; now wrap 512->1 and continue; SHOP.SMK inherits the
  fix). Corpus gate tests/brief_gate.rs: drop max frame 28 =
  29/30 rendered, handoff at closed-form pump 58, zero PCM, 2+
  ring cycles, two runs byte-identical. Code by predecessor
  3d88a359 (died after bba01fe leaving the DECISIONS/RE-EXW docs
  uncommitted; adopted + 342->343 test recount corrected by this
  run), verified + queue-closed by run 5a637669 (claim 1): 343
  workspace tests green / 0 failed, fmt + clippy -D warnings
  clean, MANIFEST.sha256 OK before AND after. All P5 D31-D37
  movie/play sites now wired. Next per queue: native executable
  shell step 1 (window + surface + fixed-step present loop, P4).
- CLOSED 2026-08-20 (P5 boot attract sequence, D36, commit 8738a03):
  the region-variant publisher pair plays on the Boot scene. RE
  prerequisite landed by predecessor as 4e9ccbb (RE-EXW-GAMETHREAD
  "Boot attract arm RE": FUN_0044567c runner - one-pass bound
  frames-1, dst 480-2*arg2 geometry incl. the TITLE replay arg2=0x50
  letterbox that verifies D31 centering, per-frame 256-entry palette,
  screen cleared twice per call, skip gate 004edbc4 => boot pair
  unskippable). Engine: bedlam-game boot.rs BootAttract
  Staged->Playing->Done (EXW order GTLOG then LOGO, movies::boot_pair,
  time-exact switch at (frames-1)*period on the x240-us grid, entry
  audio per movie, Done holds the last raster);
  MoviePlayer::advance_limited hard decode cap (EXW loop bound,
  starvation-proof); GameHost load_boot_attract on the D31 lifecycle
  (inert-until-Boot, dropped + stream cleared on exit, scene-hash
  untouched - unit-pinned). Corpus gate tests/boot_attract_gate.rs:
  both region pairs to Done at 60 Hz, max decoded frame = frames-2
  (68/69 of 70/71 - ring never wraps), switch/Done pump counts by
  closed formula, continuous in-order DPCM >100 kB per pair, two
  runs byte-identical. Rust WIP of interrupted predecessor 19dc859e
  (died on transport error after the docs commit) adopted, validated
  + completed by run 7d041b7e (claim 1; clippy tail only). 335
  workspace tests green / 0 failed, fmt + clippy -D warnings clean,
  manifest OK x2. All D31-D36 movie play sites now wired. Next per
  P5: BRF_DROP.SMK play-site RE (queue item 1).
- CLOSED 2026-08-20 (P5 FULLFONT loading-text glyph pass, D35, this
  commit): the four LAB_0041c69e text draws + the FULLPAL font-ramp
  copy run in GameHost. bedlam-game font.rs = FUN_0043c87c (measure/
  draw passes, x0 = 0x140 - total/2, space +9 / glyph w+2, RLE16
  transparent blit, hotspot dy->row dx->col baseline anchoring,
  FUN_00410493 accent remap with the shipped e-/o-diaeresis dash
  quirks, overlay glyphs at entry 0x82+0x6b+id = 238..=241);
  bedlam-assets language.rs = the LANGUAGE.* [MENU_ITEMS] table
  (strings = entries 0x45/0x46/zone+0x51/0x58; the DAT_0046bc4c/7c/
  bfdc globals are table base + idx*0x30); pal.rs parse_font_ramp =
  the 98B FULLPAL ramp (lead e0 20) that replaces fade-target
  entries 224..=255 after the draws (EXW order: 0x3f transient ->
  draws -> ramp -> FadeSetup). D34 row/y swap CORRECTED: 0x82 is the
  glyph entry base; 150/180/210/260 are draw ROWS. Host
  load_loading_font stages inert; corpus gate tests/font_gate.rs
  (FULLFONT 390 entries / 333 glyphs, ASCII pixel set {0} U
  {233..=244}, dy {0,5,10,15}; FULLPAL + 6 LANGUAGE files pinned;
  independent width re-measures). 15 new units; 326 workspace tests
  green, fmt + clippy -D warnings clean, manifest OK x2. WIP of
  interrupted predecessors adopted + completed by run 315d2af1
  (claim 1). Next per P5: boot attract LOGO/GTLOG sequence (queue
  item 1).
- CLOSED 2026-08-20 (P5 post-cutscene loading flow, D34, d834f08): the
  EXW LAB_0041c69e zone-transition tail runs in GameHost as a
  presentation-only flow (bedlam-game loading.rs, LoadingFlow
  Staged->Between->Loading): BETWEEN.BIN entry 0 owns the Cutscene
  plane after the cutscene movie ends (standing host palette); the
  region-variant loading screen (LOAD_UK/US.BIN + LOADPAL/LOADPALU,
  path-selection only) owns the Select plane with the 10-step 20 ms
  50 Hz fade on the x240-us accumulator grid; DAC tail entries
  224..=255 forced 0x3f (buf bytes 0x2a2..0x301); text row pinned
  (y=0x82, x=150/180/210, zone-6 +260, stage-1 pre-increment
  reconciliation) as TextRow state for the queued FULLFONT glyph
  pass; endgame arm (MAX_STAGE) drops the flow; skip-advance still
  runs the loading screen; scene-hash untouched (D17-b). 14 new
  units; 311 workspace tests green, fmt + clippy clean, manifest OK
  x2. WIP of interrupted predecessor 3977d55d adopted, doc fix + D34
  DECISIONS entry + bookkeeping by run f807449c (claim 1). Next per
  P5: FULLFONT.BIN glyph pass over the pinned text row (queue item
  1).
- CLOSED 2026-08-20 (P5 loading-screen asset path, this commit): the
  LAB_0041c69e zone-transition tail assets are decoded + PINNED
  (bedlam-assets tests/loading_gate.rs, 3 tests + ignored regen):
  BETWEEN.BIN / LOAD_UK.BIN / LOAD_US.BIN are single-image 640x480
  rle16 banks (flags=3, hot=(0,0)) through the existing
  sprites::parse_bin_images - no decoder changes owed; 1:1 blit into
  the 640x480x8 render Frame (no letterbox/scale). LOADPAL/LOADPALU:
  770B VGA palettes, 244 distinct, entry0 black/entry1 white.
  CORPUS FACT: LOAD_UK == LOAD_US and LOADPAL == LOADPALU
  byte-for-byte - the EXW region split selects paths, not content;
  doc note added at Region::loading_pal (bedlam-game movies.rs).
  Content pinned via file sha-heads + decoded-plane sha256s. Next per
  P5: the post-cutscene loading-screen FLOW in GameHost (queue item 1).
- CLOSED 2026-08-20 (P5 shop + briefing backdrops, D33, 1b3ef85): Shop
  and Brief scenes play their SMK backdrops through the D31 movie
  lifecycle - GameHost::load_shop (SHOP.SMK 61-frame 40 fps ring behind
  the shop UI), GameHost::briefing_name + load_briefing
  (BRF_{B..F}{sub}.SMK from the hashed episode slot;
  movies::briefing_name_for_slot: stages 2..=6 -> letters B..=F = the
  25-file corpus domain, sub = lowest-unset mask bit + 1 = the
  Episode::complete arithmetic, boot camp + endgame stages -> None - no
  BRF_A/BRF_G exists in the corpus). 6 new units (3 selection incl. the
  corpus-domain cross-check, 3 host lifecycle through the FULL_MASK
  campaign). Commit landed by worker a1ad7346 which died after push,
  before the queue rewrite; run ed15e708 (claim 1) adopted +
  independently re-validated: workspace 294 tests green / 0 failed with
  all 6 D33 units passing, fmt + clippy -D warnings clean,
  MANIFEST.sha256 OK before AND after the corpus runs. Next per P5:
  loading-screen asset path (BIN image-bank decode), then the
  Cutscene->Select flow.
- CLOSED 2026-08-20 (P5 cutscene movies + corpus inventory, D32): every
  game-data SMK inventoried and PINNED (bedlam-assets smk_corpus_gate:
  34 files, formats/rates/ring/y-scale/audio shapes; listing must match
  the table both ways). Reject-or-map verdict: ALL MAP onto the D31
  playback path, none rejected - y-scale None corpus-wide (no scaling
  logic owed), all periods exact on the x240-us grid, the single audio
  shape (DPCM mono 8/11025) is already stream-bus-native. Movie
  selection module (bedlam-game movies.rs): cutscene_name over the
  hashed stage (ZONEDONE.SMK; END.SMK at the endgame = stage >=
  MAX_STAGE, EXW pre-increment vs FSM post-increment reconciled and
  unit-pinned through the FULL_MASK cadence), Region (DAT_0046ae64)
  backing LOAD_UK/US.BIN + LOADPAL(U).PAL + LOGO/GTLOG variants,
  briefing_name over BRF_{B..F}{1..5}. Host wiring:
  GameHost::cutscene_name + load_cutscene = the D31 lifecycle on
  Scene::Cutscene (inert-until-scene, dropped on exit, hash-free).
  Workspace 257 tests green, fmt + clippy -D warnings clean,
  MANIFEST.sha256 verified before AND after the corpus runs. Next:
  Shop/Brief backdrop wiring, then the post-cutscene loading screen.
# STATE - project snapshot (update when phase changes)

- CLOSED 2026-08-19 (P5 title-movie playback, D31): TITLE.SMK plays
  through GameHost end-to-end - MoviePlayer fixed-step x240-us clock,
  compose-level MovieFrame (scene pipeline replaced while a movie
  plays, centered letterbox, palette fold PALMAP>>2 lossless), mixer
  PCM stream bus (native u8 mono 11025 Hz FIFO under voices, loud
  16 MiB cap), inert-until-scene host lifecycle with scene-hash
  isolation pinned. Full-playback gate green (pacing exact vs the
  accumulator math, composite byte-identical to an independent
  SmkStream walk, two playbacks identical). Workspace 280 green,
  fmt/clippy clean, manifests OK x2. Next per PLAN sec 6 P5: extend to
  cutscene movies + per-zone parity gates.

- CLOSED 2026-08-19 (P4 SMK decode gate, smk-stream unit): headless TITLE.SMK
  decode gate green via the codec-neutral SmkStream seam (D30) over vendored
  smk 0.1.0 - 640x320, 1227 frames, 66660us/frame, DPCM mono 8-bit 11025 Hz
  track 0; two full decode passes byte-identical (video/audio SHA-256 chains
  in NEXT.md run notes); vendored backend DPCM panic patch documented in
  bedlam-smk/NOTICE.md. fmt/clippy/tests green, manifests OK. Next phase per
  PLAN sec 6: P5 playback integration (TITLE.SMK into GameHost/presentation).

- CLOSED 2026-08-18 (P2 cosmetic tail, 119ba2d+b6620c0+007fbe5+4ace8a6):
  B2 census sec-7 residuals ALL CLOSED (census sec 7.7a-e). Campaign
  tables byte-pinned (order[8] = {3,0,1,5,9,13,17,21}; full 27-step
  idx list; 25 distinct indices = union over stages 1..7). 25-vs-27
  RESOLVED by static arithmetic - no playthrough needed: linear counts
  completions (27), formula indices are distinct table slots (25); the
  gap = two endgame completions at stage-slot 8 via the OOB order[8] =
  zone[0] sentinel hop (0x81dba + 8*4 = 0x81dda exactly). 4f02 =
  BANKED 0x101 (BX verbatim caller passthrough at 0x12439, zero 0x4101
  constructions in the 671-fn sweep, g_lfb_ptr + g_vesa_mode_req
  write-only dead). Display start 0x200 = SCANLINE units (page-B bank 5
  = 0x50000 = 0x200 x 640-byte pitch; 4f07 DX-scanline form). B2 fade
  chain named + documented (B2FadeStep@0x126c8 8.8-fixed 768ch serviced
  at 50 Hz in the ISR &1 sub-block - RATE CORRECTED on close-out verify,
  identical to EXW 200 ms fade, no divergence; setup/cancel/dacread/
  dacupload/fadewait + 3 labels persisted;
  B2LblFix repaired 2 mislabels, primaries restored). Persistence
  re-verified 14/14 (B2ResidVerify). No import (1x -process
  -noanalysis); manifests OK x2. P2 cosmetic queue EMPTY; P4 runtime
  half remains, interactive-gated.

- CLOSED 2026-08-18 (P2 cosmetic, 8f5f18f+94a65da): EXW DD surface
  creation-order CONFIRMED (RE-EXW-TICK new section): 004ee9bc =
  flip-chain head/primary; 004ee9c0 = implicit backbuffer (fullscreen
  GetAttachedSurface) / offscreen staging (windowed) - g_dd_surf_staging
  correct in both modes; FUN_0044a9ac = DDStagingProbe (sentinel
  survive-a-flip readback -> g_staging_persistent 004ee9e4); 004ee9b4
  dual-use corrected (lo = master vol, hi = palette re-attach flag;
  RE-EXW-MUSIC addendum). Trampoline CrtThreadTrampoline@00451fbc +
  usage roles were already persisted by the tick-sat run; this pass
  added the creation-order proof + names. No import; manifests OK x2.
  P2 cosmetic queue now: only the B2 census sec-7 residuals item (in
  flight). P3 charter complete; P4 runtime half still interactive-gated.

- CLOSED 2026-08-18 (P4 kickoff code half, c61d7f7): headless parity
  harness v0 example landed (engine/bedlam-game/examples/parity_harness.rs,
  D28): GameHost driven end-to-end over a recorded input script, JSON
  report with per-tick scene-hash chain + frame parity + sim hash + audio
  stream hash; .MRW banks loaded per track (audible baseline); verified
  byte-identical across runs; fmt + clippy -D warnings clean; workspace
  204 green unchanged; manifests OK x2. P4 runtime half (wine/DOSBox
  comparisons vs this CPU baseline) = next, needs interactive desktop.

- CLOSED 2026-08-18 (game unit, 4ab051c+7e3e472): P3 CHARTER SET COMPLETE.
  bedlam-game = the LAST charter crate (assets/core/render/platform/audio/
  game all landed as skeletons). Scene FSM (10 scenes, B2 episode shape
  {stage,mask,linear} + FULL_MASK@0x81d9a, D26 hashed per-tick edge
  latches), host pump in FUN_0043d00b order, MusicPump bridge (D27
  melody-chunk + attach-anchored mixer dispatch), typed OPTIONS.BDL.
  Workspace 204 tests green, fmt + clippy -D warnings clean, manifests
  OK x2. Next phase per PLAN sec 6: P4 (harness/playable) - first item
  = dependency/version spike + runtime smoke, needs interactive desktop
  for wine-exw (do NOT run unattended).

- CLOSED 2026-08-18 (P4 runtime unit, unattended subparts, 79227e5+11c8d9c+b951e7c):
  D28 anchors REPRODUCED byte-identically x2 runs (scene
  0xcae25cd08d7cbc08, sim 0x72979d5d9dedc832, frame 0x87263f149564ad25,
  audio 0xc862e45d2e95ad29; reports cmp-identical). DOSBox-X harness
  LANDED: flatpak static-home finish arg DISCOVERED (per-dir :ro grants
  illusory) -> sandbox hardened (home revoked, runtime-only, verified via
  flatpak info), corpus via rsync scratch copy, pinned conf (svga_s3/
  core=normal/cputype=pentium/cycles=fixed 60000/vmemsize=2/scaler=none/
  sample-accurate sb16), driver prepare/smoke/shell/game, watch skeleton
  (census-verified watch set; PresentFlip frame trigger; 3 ghost addresses
  dropped), HEADLESS SMOKE GATE PASS first-hand (SMOKETST.TXT lists both
  EXEs). D29. Interactive half still gated: wine EXW launch + DOSBox-X
  golden-run calibration/checklist (RUNTIME.md follow-ups).
  Post-restart re-verification 17:56-18:0x (worker 1787068533):
  smoke gate re-run FIRST-HAND - PASS (rc=0, both EXEs at pinned
  sizes), sandbox posture verified via override file + flatpak
  override --show --user (!home + runtime only; note: without
  --user the CLI prints empty under env-based XDG_DATA_HOME),
  manifests OK x2 bracketing - harness stack stable across the
  4th restart of this lane.

- Phase: P1 essentially complete; P2 well underway. P3 UNDERWAY (bedlam-core skeleton DONE 2026-08-18): decoders
- Phase: P1 essentially complete; P2 well underway. P3 UNDERWAY (bedlam-core skeleton DONE 2026-08-18): decoders
  promoted to workspace crate engine/bedlam-assets (pure, inspect CLI output
  byte-identical, D14); MUSIC FORMATS DECODED IN RUST 2026-08-17: music.rs
  module (MRS container + full event-stream walk + RATIO_TABLE verbatim from
  EXW, MRW bank with wave ranges, byte-exact rebuilds) + decode-song CLI +
  inspect mrs dumper + corpus invariants (see RE-EXW-MUSIC.md 3b). EXW outer architecture +
  100Hz tick + game worker thread FULLY mapped (GameThread@0044dea0 = 59-byte
  trampoline -> GameMain@0041c050 = real game shell/loop; 7x5 zone/level
  structure; RNG seeds 123456/234567). RATES (D15): 100Hz service tick /
  50Hz palette fade while fading / 12.5Hz palette cycle; 004ede10 = fade
  countdown (NOT a frame gate - D13 50Hz parity claim withdrawn); sim/render
  rate UNKNOWN pending FUN_0043d00b/FUN_00440e45 bodies. Tick satellites
  fully mapped: fade engine (FadeStep/FadeSetup/SetPaletteRGB), CursorToGame
  (window->640x480), DDRAW init/shutdown chain + object slots, thread spawn
  via Watcom CRT ThreadSpawnImpl@0045204b -> real CreateThread. Names applied in BedlamWatcom project (WinMain..
  AppActivate, TickWorker.., GameThread/GoFlagSet/GameMain - see
  docs/RE-EXW-MAINLOOP.md, docs/RE-EXW-TICK.md, docs/RE-EXW-GAMETHREAD.md).
  EXD import still pending.
- CLOSED 2026-08-18 (b2-import run): B2 DOS IMPORT DONE - ghidra-lx-loader
  built from source vs our exact 12.1.2 install (zero version risk),
  installed to userSettings/Extensions; import command + 3 gotchas in
  RESEARCH-BEDLAM2-CENSUS.md sec 5 (-loader LeLoader forced; MzLoader
  otherwise claims LE first). BedlamWatcom:/BEDLAM.EXE analyzed: 671 fns,
  blocks 0x10000/0x80000-0x1304ee, entry 0x66a60, 24041 applied fixups.
  First cross-build parity fact: RNG seeds 123456/234567 identical in B2
  (FUN_0002f731 game-init) and EXW (004ede48/4c). B2 pipeline = -process
  BEDLAM.EXE -noanalysis from here on (NEVER re-import).
- CLOSED 2026-08-18 (b2 entry/tick run, 2df7664+c3b1552+9b4d119): B2
  entry chain named + TICK SOURCE FOUND + zone/mission stride located
  (census sec 6, D22). _entry@0x66a60 -> CrtInitChain@0x6b1bc (argc/argv
  g_argc@0x1280d4/g_argv@0x1280d8) -> GameInit@0x2f731 = boot + episode
  loop shell (seeds RNG 123456/234567 as code constants at 0x11ef1c/18).
  Tick = 100.01 Hz PIT INT-8 ISR (divisor 0x2e9b, DOS INT21 AH=25h vector,
  immediate EOI, drop-not-queue reentrancy): 7 counters, 12.5 Hz palette
  banks 0x90..0x97 (same as EXW), 50 Hz mouse poll+clamp vs 320x240 coords,
  play-clock divider; present = vblank double-poll 0x3da (WaitVRetrace).
  Same two-clock architecture as EXW -> D16 parity budget carries to DOS.
  Zone/mission = lookup tables (order[8]@0x81dba, zone letters@0x81dda,
  mission[27]@0x81e46; +5 when mode==2 -> MISSION{6,7} corpus files; 6
  zones x {4 regular + 2 alt}, 27 linear missions). 15 fns + 16 labels
  persisted in BedlamWatcom:/BEDLAM.EXE.
- CLOSED 2026-08-18 (miri+hash-CI run): PLAN sec 7 DETERMINISM CI GATE DONE
  (1501ab9 + 014597b). (a) Miri CLEAN on this host: rustup component add
  --toolchain nightly-x86_64-unknown-linux-gnu miri (miri 0.1.0
  771916f902 2026-08-08, on the existing nightly; rustc 1.99.0-nightly
  b07e5a086 2026-08-07), then cargo +nightly miri test -p bedlam-core =>
  41 unit + 12 determinism tests green, ZERO UB findings (111.5s + 40.9s;
  re-run with the new fixture green too). (b) Committed per-tick hash
  fixture: engine/bedlam-core/tests/hash_fixture.rs - 600-tick fixed
  integer script (seed 123456, fade window armed ticks 101..200) pins 13
  milestone StateHash values + FNV-1a chain over all 601 hashes
  (EXPECTED_CHAIN 0x760d221bec3b3b99); runs in the ordinary cargo test
  matrix => cross-OS/toolchain hash drift fails loud per tick; ignored
  print_fixture is the ONLY documented regeneration path (intentional
  hashed-state changes + FORMAT_VERSION bump). (c) ci.yml miri job:
  ubuntu-latest, dtolnay/rust-toolchain@nightly + miri component,
  cargo +nightly miri test -p bedlam-core per push/PR. Workspace now 154
  tests green (fixture +1), fmt + clippy -D warnings clean, manifests OK
  x2. Next P3: bedlam-audio mix-graph skeleton (design note first), then
  bedlam-game scene-FSM skeleton.
- Repo: github.com/roguefort-dev/bedlam-re (main). Local: ~/Documents/bedlam-re
- Autonomy: tools/nudge.sh + systemd user timer bedlam-nudge.timer (60s) + crontab
  fallback. Heartbeat: .state/heartbeat (stale > 7 min => spawn continuation run).
  Stop conditions: .state/PLAN-COMPLETE (forever) or .state/PAUSE (temporary).
  NOTE: touch heartbeat around every long shell command (Ghidra ~2min) or a
  second agent gets spawned mid-run (happened 2026-08-17, see NEXT.md run notes).
- Backups: game-data copy at ~/Backups/bedlam-re/game-data (1069 files). Original
  bin/cue on Desktop. Offsite: NOT YET (user to arrange).
- CLOSED 2026-08-17 (input-map run): EXW input/control map - scan-code
  keystore @004edc44 (arrows +0x80 remap), 12 edge latches, mouse flags
  @004dc6e4 (dbl-click dead), Up/Down=volume P=pause, Left/Right arrows
  DEAD (3-way proof), camera=cursor+drag only - docs/RE-EXW-INPUT.md.
- Known open: GameMain second hop - FUN_0043d00b (per-frame sim/render; its
  004ede10 read = fade-status, real rate mechanism unknown - D15) +
  FUN_00440e45 (zone/level manager) + divider consumers (FUN_00448ef1,
  FUN_00402b48); music chain FULLY closed incl. sub-voice start path (SubVoiceStart = SetFrequency ratio*11025 / SetVolume / SetPan / Play; table C + 0xFE loop flag + pending-restart all DEAD); .BLD/.CTG (editor-only), PAL variant
  renderers, EXD import (needs LE loader ext), goldens pipeline (P4).
  Parity budget: NO committed logic rate (D15 withdrew D13 50Hz).
  CLOSED 2026-08-17 (tick2 run): GoFlagSet caller = FUN_0041e19d; fade
  engine, cursor mapping, DDRAW init chain, thread-spawn slot all mapped
  (docs/RE-EXW-TICK.md tick2 section).
- CLOSED 2026-08-17: music format chain fully RE-d + byte-validated
  (.MRW layout, .MRS container + complete event grammar, MusicPump=song 3,
  ratio table @00454174; CONFIG.BDL = installer SB-setup record, EXW never
  reads it; RNG seeds consumed by RandA@00402975 / RandB@004029b6) - see
  docs/RE-EXW-MUSIC.md.

- CLOSED 2026-08-18 (bedlam-core run): P3 sim core skeleton DONE in
  engine/bedlam-core (f15eb60+7396491+889cbef) - D17 hybrid timing: hashed
  60Hz Sim (300Hz microstep satellites per DESIGN-RENDER sec 6) + non-hashed
  per-frame FrameState + 240Hz sub-tick SimDriver accumulator; PCG32, Q16.16
  fx, in-crate FNV-1a state hash, versioned b"BDLR" replay + b"BDLS"
  snapshot; 132 tests green, clippy clean, manifest OK. Next P3: render
  crate skeleton (design note a3ad066), Miri + cross-OS hash CI.

- CLOSED 2026-08-18 (episode-loop run, 928748d+7bfac4b+aff1ae8 + adopted dead-run
  B2EpisDump): B2 EPISODE LOOP + COUNTER VERDICTS DONE (census sec 7, D23).
  All 7 INT8 counters classified - NONE gate sim/render (2 audio bases
  0x801a6/0x80010, 2 DEAD 0x11f158/0x11f0b4, ISR phases 0x11f0c8, 100Hz
  timeout base 0x11f0c4 w/ WaitTicks100Hz, 50ms delay 0x11f0b0). Mission loop
  = present-paced VESA page flip + vblank (D16 architecture CONFIRMED on
  DOS, D23). Episode progression: linear 0..26 + per-stage-slot completed
  mask (full-mask table 0x81d9a) + stage-slot advance w/ zone-complete
  cutscene; sub = PLAYER-selected in MapRoomSelect (mission-select UI, BRF_*
  backdrops); saves = 5 x 61B records {mask,slot,linear,money,stats}.
  B2 audio = IRQ0-shared 11025 Hz PCM driver (PIT reprogram on arm; stub
  ms-clock x10 vs hi-res tick+PIT-phase) - same native rate as EXW.
  Video = VESA 0x101 640x480x8, dual pages bank {0,5} display-start {0,
  0x200}, 640-byte stride + 320x240 logical space = 2x scale. Zone letters
  dword[0]=25 = sentinel (unreachable index). 30 fns + 33 labels persisted;
  orphan stub/driver callbacks created as functions. Open residuals queued:
  27-vs-25 step accounting, LFB-vs-banked 4f02 variant, 0x200 units,
  FUN_000126c8 satellite.
- CLOSED 2026-08-18 (render+platform unit): P3 PRESENTATION SKELETONS DONE
  (ff8fb17 + d2b7fb8, D24). engine/bedlam-render = pure state->canonical
  640x480x8 Frame + 6-bit palette + FNV parity hash, fixed pass order
  (world->sprites->rows->overlays->entities), camera clamp, palette_dirty
  derivation, 12 tests. engine/bedlam-platform = pure scale/uv geometry
  (Integer default/Fit/Fill) + wgpu 27.0.1 parity pipeline (index tex per
  frame + packed palette tex on dirty + fullscreen-triangle WGSL
  palette-expand/scale, Original v<<2 default), offscreen GPU round-trip
  test that skips without an adapter, 9 tests. Workspace 153 green, clippy
  -D warnings + fmt clean. Provenance: code landed by the 03:00 worker
  whose client died transport rc=1 at 03:05 while its server session
  finished both commits (03:07, 03:17) then died before the queue rewrite;
  the 03:32 respawn verified the work (153 green incl. real GPU test,
  fmt/clippy clean, manifests OK x2) instead of redoing it and closed the
  unit. Next P3: Miri over bedlam-core + per-tick hash CI job.
- CLOSED 2026-08-18 (audio unit, triple-agent night): P3 AUDIO MIX-GRAPH
  SKELETON DONE in engine/bedlam-audio (846ebab + b684bee + 00c2260 +
  b950b44 + a8f26f8). DESIGN-AUDIO.md pinned first (mix topology voices ->
  master bus -> device; 11025 Hz native both builds; Q16 tick grid 441/4
  samples = exact; D25 linear-Q8 volume over the EXW (master*vol)/48
  product, dB curve documented not reproduced; note-off-releases-BASE
  quirk kept; audio NOT hashed per D17 b - byte-identity of the mix stream
  is the gate). Crate: hermetic integer Mixer (forbid unsafe, no floats,
  no I/O/clock), flat 20-voice pool (B2 walker) tagged (instrument, sub
  0..3) (EXW mrw 4 sub-voices), 16.16 phase step = RATIO_TABLE verbatim,
  Q8 volume x pan gains snapshotted at spawn (EXW reads master per
  SubVoiceStart only), i32 bus + symmetric clamp, S16 stereo interleaved
  out; MusicScript = absolute-tick NoteOn/NoteOff list with
  no-bedlam-assets coupling (mapping lands in bedlam-game); render
  dispatches events at exact Q16 positions chunking-invariantly.
  9 unit + 14 determinism tests (same script => byte-identical buffer
  across 1/7/64/512-frame chunkings, base-only note-off, drop-when-full,
  one-shot recycling, saturation clamp, tick-grid exactness at frame 441),
  workspace 177 green (+23), fmt + clippy -D warnings clean, miri CLEAN
  (9+12 tests, zero UB; integration suite ~292s under miri - ci.yml miri
  job extended to -p bedlam-audio, acceptable CI cost). DECISIONS D25.
  Deliverable survived a duplicate-spawn storm (three agents on item 1:
  0162 silent death, 0711 = this run, 1260 transport death mid-verify; a
  watch run contaminated then cleaned the lane and deleted the uncommitted
  test file - regenerated from the /tmp/opencode generator; boundary bugs
  it would have caught were caught by the restored suite: immediate
  one-shot free + event-on-exact-boundary ordering). Next P3: bedlam-game
  scene-FSM skeleton (LAST charter crate).

