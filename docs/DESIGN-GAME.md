# DESIGN-GAME - bedlam-game crate design note (P3; elaborates D16/D17, adds D26)

Status: IMPLEMENTED AS SKELETON (4ab051c; WIP from the 13:2x transport-
killed predecessors adopted + verified, three test-harness bugs fixed by
the close-out run). DECISIONS D26/D27 record the two non-obvious calls.
Mirrors the DESIGN-RENDER / DESIGN-AUDIO flow. This was the LAST P3
charter crate - the P3 charter set is complete.

## 1. The contract

bedlam-game is the composition layer: the scene state machine plus the
per-frame host pump that wires the sibling crates (core, render, audio,
assets) together. It owns NO per-mission game logic (PLAN P3: mission
quirks are data, a P5 hypothesis). The FSM itself is pure and hermetic:
all file I/O hides behind injected byte-source/sink traits, so the
machine replays and hashes from tests unchanged.

## 2. RE basis (what the original does, with anchors)

| # | Fact | Anchor | Tag |
|---|------|--------|-----|
| 1 | EXW: GameThread@0044dea0 is a 59-byte trampoline into GameMain@0041c050 = the game shell; the in-game advance loop FUN_0043d00b = poll -> sim/render -> PresentCopy (MemCopy 0x4b000) -> PresentEnd -> frame count++ | RE-EXW-GAMETHREAD sec 1, RE-EXW-PACER sec 1, D16 | verified |
| 2 | B2: GameInit@0x2f731 = boot shell AND episode-loop host; reads OPTIONS.BDL at boot; the player selects the sub-mission in MapRoomSelect@0x50a87 (BRF_* backdrops per stage slot 2..8) | RESEARCH-BEDLAM2-CENSUS sec 6/7 | verified |
| 3 | Episode progression: linear counter 0..26, +1 per completed mission; mask = one bit per completed sub; stage-slot advances when mask == full-mask table {0, 1, 0xf x6} @0x81d9a; zone-complete cutscene = LOAD_UK/US.BIN; boot plants stage=1 / sub=1 as constants | census sec 7 | verified |
| 4 | Screen set: title/menu (HEREIAM menu + high-score screen FUN_00448ef1), OPTIONS screens, brief / debrief / shop / select (one .MRS music track EACH: BRIEF / DEBRIEF / OPTIONS / SELECT / SHOP.MRS), mission, cutscene | game-data SOUND/MIDI corpus + census sec 6/7 | verified 5/5 |
| 5 | OPTIONS.BDL is read by the game but WRITTEN by SETUP.EXE (SETUP-owned); root CONFIG.BDL (61 B) = installer SB record, NEVER read by EXW | RE-EXW-MUSIC sec 4 | verified |
| 6 | B2 saves: 5 slots x 61 B {mask, slot(stage), linear, money, stats} @0x8b1d4; EXW persists SAVED.BDL 5x180 B + HISCORES (both parsed in bedlam-assets bdl.rs) | census sec 7, bedlam-assets bdl.rs | verified |
| 7 | Volume: UI 0..100 -> >>1 -> master 0..50 (FUN_0044c630); keyboard = hotkeys/volume/pause/any-key ONLY, gameplay pointing = mouse; key/mouse state is LEVEL-SAMPLED with edge latches (KeySink@0041be05 12 dwords, MouseSink@0041bf35 bit0/bit1) | RE-EXW-INPUT, DESIGN-AUDIO fact 3 | verified |
| 8 | Per-scene music rides the MusicPump analog: Mrs::walk -> MusicScript (absolute ticks) -> Mixer; SFX bypass the script (host-event-timed) | DESIGN-AUDIO sec 7 | pinned |

## 3. Scene FSM topology

    Boot -(boot ticks elapse)-> Title
    Title   -Advance-> Brief     Title -Options-> Options    Title -Quit-> Quit
    Options -Back->    Title
    Brief   -Advance-> Select    Brief -Back-> Title
    Select  -Advance-> Mission
    Mission -MissionComplete-> Debrief (episode progress applied)
    Mission -MissionFail-> Debrief (no progress)
    Debrief -Advance-> (zone complete pending ? Cutscene : Shop)
    Cutscene -Advance-> Select
    Shop    -Advance-> Select
    Quit = terminal

- SCENE SET (fact 4): Boot, Title, Options, Brief, Select, Mission,
  Debrief, Shop, Cutscene, Quit. No per-mission code (PLAN P3).
- EPISODE MODEL (hashed; deliberately mirrors the B2 save shape of
  fact 6 so a later save/load is a field copy): stage 1..=8 (slot),
  mask (subs done in this stage), linear 0..=26, zone_complete_pending.
  money/stats stay sim-side (P4+) and are NOT modelled here. Sub
  selection inside a stage = lowest-unset-mask-bit [design, open Q5]
  until the Select-screen RE lands.
- ACTIONS: Advance/Back derive from per-TICK edge detection on mouse
  buttons bit0/bit1 (fact 7 level-sampled latch analog); Options/Quit
  are UI intents, MissionComplete/MissionFail are sim outcomes - all
  four applied explicitly by the host (fsm.apply).
- BOOT: a fixed 12-tick (200 ms at 60 Hz) hold [design], then Title;
  input edges during Boot are ignored (the GameGoRelease latch clear,
  RE-EXW-INPUT). Scene changes clear the edge-latch state (the P-latch
  clear analog, RE-EXW-INPUT).

## 4. Composition (the per-frame host pump)

pump_frame mirrors the FUN_0043d00b order poll -> sim -> render ->
present:

1. poll: the caller hands this frame InputFrame + dt (sub-ticks);
2. sim: SimDriver::advance quantizes dt and executes whole 60 Hz ticks;
3. scene: SceneFsm::tick once per EXECUTED tick with the same pending
   input (same grid and same pending semantics as the sim);
4. render: bedlam_render::render -> canonical 640x480 indexed Frame
   (parity configuration: prev_sim = None, alpha ignored);
5. present: the Frame is handed back to the caller (PresentCopy
   analog). bedlam-platform consumes it; bedlam-game takes NO
   dependency on bedlam-platform;
6. audio: Mixer::render at HOST pace (D17 bucket b, never hashed); the
   music script attaches per scene (sec 5) at scene-change boundaries.

Dependency direction: bedlam-game -> {bedlam-core, bedlam-assets,
bedlam-audio, bedlam-render}. It is the only crate allowed to see all
four; the MusicPump bridge lives here per DESIGN-AUDIO sec 7.

## 5. Music bridge (MusicPump)

build_script(mrs, chunk) -> (MusicScript, ScriptMeta): walk the chunk,
accumulate deltas into ABSOLUTE ticks, then per DESIGN-AUDIO sec 7:

    Note volume != 0xFF  -> NoteOn  { instrument, ratio, volume }
    Note volume == 0xFF  -> NoteOff { instrument }
    Rest                 -> advance only
    SongEnd / Restart    -> terminal (meta records the kind; the HOST
                            re-inits the walk = the loop)

Track-per-scene table (fact 4): Options -> OPTIONS.MRS, Brief ->
BRIEF.MRS, Select -> SELECT.MRS, Debrief -> DEBRIEF.MRS, Shop ->
SHOP.MRS; all other scenes None (title = TITLE.SMK video, played by
the host MoviePlayer through RenderInput.movie per DECISIONS D31;
mission
music = open question Q2). The MusicPump pre-builds the script once
per loaded file; scene changes swap Mixer scripts; restarts rebuild.

## 6. Config and save model

- OPTIONS.BDL is the ONLY config file the engine reads (fact 5).
  config.rs wraps bedlam-assets parse_options_bdl into a typed
  GameConfig: volume validated 0..=100, player name 8 sanitized graphic
  chars, flag fields as bools (nonzero), language kept as the raw u32
  code (open Q3). A 41-byte writer round-trips the typed view.
  music_master() = volume >> 1 (fact 7, the 0..50 master domain).
- CONFIG.BDL is NOT modelled anywhere (fact 5: never read by EXW).
- Saves: EXW SAVED.BDL + HISCORES via bedlam-assets; the B2 5x61 B
  shape is documented (fact 6) and mirrored by the episode fields. All
  persistence crosses the injected traits ONLY.

## 7. Determinism boundary (D17 + D26)

- HASHED (the scene bucket): scene id, per-scene tick counter, episode
  progression (stage/mask/linear/zone_complete_pending), boot countdown,
  and the per-tick edge-latch state. Hash = FNV-1a 64 (bedlam-core
  hash crate reused) over canonical LE fields with the tag BDLG.
- UNHASHED (D17 bucket b): the SimDriver accumulator, FrameState, the
  Mixer and audio pump, the rendered Frame bytes.
- D26 (the non-obvious call): EXW input polling is per-frame (bucket
  b), but the SCENE ACTIONS here derive per tick by level-sampling the
  consumed tick input - mirroring the 100 Hz-sampled KeySink latches -
  so action derivation sits INSIDE the hash. This is what makes the
  15/60/240 Hz host test exact; per-frame edge detection could never
  be hashed.

## 8. Hermetic rule

forbid(unsafe_code); no fs / clock / threads anywhere in the crate;
every byte crossing the boundary passes through the injected
ByteSource / ByteSink traits (host.rs). thiserror only; NO new
dependencies.

## 9. Testing and goldens

- FSM unit: transition table, episode progression incl. mask fill ->
  stage advance -> cutscene, linear cap, Quit terminal.
- tests/determinism.rs: same WALL-TIME input script (phases aligned to
  16-sub-tick boundaries so a 15 Hz host can represent them exactly)
  -> identical scene hash at 15/60/240 Hz; pure-FSM replay determinism
  + divergence on a different script.
- tests/music_corpus.rs: walk-vs-script equivalence over the 5 shipped
  .MRS files (every enabled chunk): same note sequence at the same
  absolute ticks, non-decreasing, terminal Freeze everywhere, note
  volumes inside the observed band, chunk 0 disabled in every file.
  Skips when game-data is absent (CI).
- Gates: cargo fmt, cargo clippy -D warnings, workspace tests 177+.

## 10. Open questions (each names its answer source)

- Q1: OPTIONS.BDL flag semantics (backbuffer/actionpan/cd_audio/midi/
  sound/code_no_title values) -> P2g UI RE pass.
- Q2: mission-scene music source (no MISSION*.MRS ships) -> P2 audio
  inventory; the track table gains a mission row then.
- Q3: language code space (B2 LANGUAGE.* select in GameInit) -> same
  P2g pass.
- Q4: B2 61 B slot layout (money/stats field offsets) -> P2g save RE;
  the EXW 180 B SAVED.BDL already parses.
- Q5: Select-screen sub selection (replaces the lowest-unset
  placeholder) -> P5 gameplay RE.

## 11. Mission scene composition (P4, added 2026-08-21)

The `Mission` scene state gets its presentation module
(`src/mission.rs`, `MissionScene`) — the composition of the two
corpus-verified halves: bedlam-core `MissionSim` (the sim slice) and
bedlam-render `MissionView` (the isometric viewport). NO new RE: every
behavior below is anchored to an already-decoded EXW fact, listed
inline. Bounded unit: staged-inert lifecycle + per-frame drive + the
robot-click order seam, hash-pinned headless; window/surface and
sidebar work stay OUT (the following unit). The GAMEPAL present tail
(added 2026-08-21, this section's PRESENT bullet) stages the mission
palette with the mission — fetch set 10 files.

- STAGING [RE-EXW-SIM sec 7c]: `load_mission(tot, dat, pad, cgr, bin,
  lnk, sintable, dante, gamepal, mrk, zone, robots_override,
  staged_markers)` builds `Terrain::from_mission_bytes` + `AngleTable` (SINTABLE words
  2..66), seeds `MissionSim::new(.., 0x1E240)` (the MissionShell
  reseed, sec 1), spawns the first `robots_override.unwrap_or(
  robots_per_player(zone))` MRK records verbatim (`robots_per_player`:
  zone<3||zone==7 → 1, zone 3 → 2, else 3, sec 7c.7), then any
  `staged_markers` (the host/test seam the 0x46cbe0 network override
  fills in the original — sec 7c.8), builds
  `MissionView::from_mission_bytes(tot, swept dat planes, bin, lnk)`
  and stages DANTE via `set_entity_bank`. GAMEPAL folds to the
  canonical 6-bit palette (`parse_vga770` + `>> 2`, the
  `loading_palette` rule) and owns the mission plane palette.
  Malformed bytes → `GameError::BadMissionAsset`, never a panic.
- LIFECYCLE [movie/brief pattern, D31/D37]: a staged mission is INERT
  until the FSM enters `Scene::Mission`; activation fixes the camera
  at the first robot's Q5 position (the EXW cam pair DAT_004edde4/8
  pointing at the spawn; scroll input out of scope — [design] fixed
  for the slice). Leaving the scene drops the mission (the flow never
  ends on its own, like the briefing backdrop).
- PER FRAME [FUN_0043d00b order]: per EXECUTED 60 Hz tick — integrate
  the cursor from mouse deltas (clamp 0..639/0..479, the menu D42
  pattern), on a left-button EDGE at the tick grid run the robot
  click seam, then `advance_frame` (the MissionShell frame). The
  click seam [RE-EXW-SIM sec 6.4]: clicks land in the viewport
  (x < 0x1E0 = 480; x >= 0x1E0 is the sidebar, out of scope →
  no-op), hit-test every alive robot by the enqueue projection
  (MISSIONVIEW sec 5d) inside a 0x20-px box [design: half the 64-px
  sprite cell; the EXW walks the sprite outlines ~0x433cbc], nearest
  octagonal screen distance wins, and arm the order AT that robot
  (`arm_order_at_robot` — the EXW arms at the clicked robot's tile,
  one pending order, spread-assign, state 3).
- PRESENT [MISSIONVIEW sec 7]: every host frame while active —
  `enqueue_robots` (RobotView per robot, camera Q5, shake 0, sim
  frame), `draw_terrain` into the 0x64000 buffer (zone index =
  staging zone; edge-variant stream `Pcg32::new(0x1E240, 0)`
  [design: zone 0 = ZONEA consumes none]), `present_window`, blit the
  480x480 window at canonical (0, 0) — the EXW mission screen is
  viewport [0,480)x[0,480) + sidebar [480,640) (mouse_l_click
  x >= 0x1E0 branch), NOT letterbox-centered. Palette = GAMEPAL:
  the mission plane carries the folded GAMEGFX\GAMEPAL.PAL (770 B,
  the parse_vga770 LOADPAL family) staged with the mission
  [RE-EXW-MISSIONVIEW sec 6 — GAMEPAL loads into the 0x4edbf8
  0x302-B blob the mission-load pass copies to 0x4ddb34,
  RE-EXW-SIM sec 7c.3]; the host palette no longer stands in. The
  plane rides the MovieFrame seam, so the mission frame's palette
  IS GAMEPAL and palette_dirty every frame (the movie convention);
  the indexed->RGBA window upload stays platform-side as-is.
- DETERMINISM: the sim half is hashed (MissionSim::state_hash); the
  LNK memo walk + edge stream are presentation state (D17 bucket b)
  — one render per host frame advances the walk once, matching the
  render corpus gate's one-draw-per-frame rhythm at 1 tick/frame.
- SIDEBAR PRODUCER (added 2026-08-21, RE-EXW-SIM sec 6c; ART half
  added 2026-08-21, sec 6c.8; loadout + text 2026-08-21 D51; bars +
  strip 2026-08-21 sec 7f/D52): clicks at `x >= 0x1E0` run
  `sidebar_control` (the mouse subset of FUN_0040d197) INSTEAD of
  the robot arm — the robot-select strips
  (`SIDEBAR_SELECT_STRIPS` × y[5,0x35], gated by the spawned squad
  size and the target's alive word), the 7 order rows
  (`SIDEBAR_ORDER_RECT`, `row = (y-0x57)/14` clamp 6, gated by the
  weapon group's AMMO word, toggling the per-robot order-bits
  word), the map-toggle strip (7e.5), and the `DAT_0046ccec`
  redraw COUNTDOWN + the `0x46ccf0` strip countdown (producers set
  2, every present decrements while nonzero and, while nonzero,
  runs the matching pass below). All of it lives on the
  presentation half: none of it enters the sim state hash (unit
  pinned + corpus pinned — sidebar clicks never arm orders).
  WEAPONS (D51): rows are WEAPON groups — 7 × (name_idx, ammo),
  host-staged per robot (`set_weapon_loadout`, the shop/save/MP
  seam), faithful fresh-campaign default EMPTY (no rows); order
  bits default `1 << first group with name_idx != 0` [sec 6c.6].
  ART: GAMEGFX\GENERAL.BIN + GAMEGFX\SMLFONT.BIN +
  GAMEGFX\NUMBERS.BIN stage with the mission (the 23-asset chain);
  present draws, in the CORRECTED FUN_00403938 tail order [7f.3]:
  the FUN_004072bf select portraits (0x12+slot selected / 0x15+slot
  not, at (0x1E7+0x32·slot, 5), gated by squad size + alive + hp
  ≥ 1) every present; the FUN_0040807f HP/armor BARS every present
  (hp 0x18..0x46 denominating 5000 @ (0x1E8+0x32k, 0x3C), armor
  0x60..0x8E denominating 2500 @ (slot_x, 0x49), over the SIM
  robot fields — D53: hp/armor are hash-covered sim state (spawn
  hp = 5000 + 100·battery via set_battery, armor 0), the
  FUN_0040e230 damage core + the armor pad charge/bleed + the
  shield family landed in the sim; the bars read them directly);
  FUN_004085ce score STRIP on its own countdown (icon 0xA + nine
  score digits, icon 0xB + six money digits, campaign session
  state 0/4000 fresh, the case-4 pickup producer seam with two
  `rand_a` draws from the shared sim stream; the case-1/2/3/7
  pickup bodies are the D54 apply_pickup sim seam - the
  tile-word producer stays host-seamed); then the
  FUN_00408403 row chrome + NAME/COUNT text (armed 0x47+0x4A,
  unarmed 0x49+0x4C at (0x1EB/0x25A, 0x59+14i), names via the
  pinned FUN_00420260 table, counts "%04i", SMLFONT color 0x24)
  on the redraw countdown; both countdowns start at 2 on activate
  (MissionShell 0x447c74/0x447c7a). The dead/hit dither
  (FUN_00401ae6), deploy panel and blink cursor stay unwired
  (never invented). Out of scope: the keyboard latches (P2e
  button map).

## Provenance

Written 2026-08-18 by the item-1 worker (claim lock-v1 1787044723) from
RE-EXW-PACER sec 1, RE-EXW-GAMETHREAD, RE-EXW-INPUT, RE-EXW-MUSIC
sec 4, RESEARCH-BEDLAM2-CENSUS secs 6/7, DESIGN-AUDIO sec 7, DECISIONS
D16/D17, PLAN secs 6 P3 / 7. RE facts carry anchors; [design] and D26
marks are reimplementation choices, not RE claims. Confidence: high on
facts 1-8 (all verified in prior runs), high on the API shape
(mirrors the DESIGN-RENDER / DESIGN-AUDIO acceptance flow).

Section 11 added 2026-08-21 by the item-1 worker (a835cefc) — pure
composition of RE-EXW-SIM secs 1/6/7c + RE-EXW-MISSIONVIEW secs 5d/7;
no new binary decoding. GAMEPAL present tail amended 2026-08-21
(worker 1776dc60): the palette anchors are RE-EXW-MISSIONVIEW sec 6
(GAMEPAL → 0x4edbf8) + RE-EXW-SIM sec 7c.3 (the 0x302-B mission-load
copy) + the parse_vga770 corpus format (RESEARCH-8STREET .PAL row);
the fold rule and the MovieFrame palette hand-off are the established
D31/D34 conventions.
