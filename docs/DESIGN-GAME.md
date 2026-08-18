# DESIGN-GAME - bedlam-game crate design note (P3; implements D17, D9, D12)

Status: DESIGN PINNED by this note; the crate skeleton implementing secs 3-8
lands in the same unit (the LAST P3 charter crate). Mirrors the DESIGN-RENDER
/ DESIGN-AUDIO flow (note first, code second).

## 1. The contract

`GameHost` is the composition root of the engine: per host frame it quantizes
dt, advances the deterministic sim + scene FSM (hashed bucket a), converts
scene intent into render frames (pure, via bedlam-render) and drives the music
mixer from the .MRS event stream (un-hashed bucket b). It holds no clock, no
I/O, no threads, no floats; all file bytes enter through injected
byte-source/sink traits (sec 8). Gameplay content is P4; this skeleton pins
the FSM topology, the pump wiring and the determinism boundary.

## 2. RE basis (what the original does, with anchors)

| # | Fact | Anchor | Tag |
|---|------|--------|-----|
| 1 | EXW shell: GameThread@0044dea0 trampolines into GameMain@0041c050 = boot init, first-run intro movies (GTLOG_US/UK.SMK, LOGO_US/UK.SMK), name entry (FUN_0043a5fc), then the EPISODE LOOP (7 zones x 5 levels) | RE-EXW-GAMETHREAD | verified |
| 2 | EXW episode loop body: menu/modal runner FUN_0043e7d4 then mission loop FUN_0043d00b (poll, sim/render, PresentCopy@00425a1e, PresentEnd@00425a03) then outcome switch via FUN_0044771c (advance / restart-checkpoint / quit / retry) then zone-complete (ZONEDONE.SMK, END.SMK for zone 7, FadeSetup 10-step) | RE-EXW-GAMETHREAD | verified structure |
| 3 | Linear mission stride: clamp((zone-2)*5 + level-1, 1, 26); zone 7 = endgame | RE-EXW-GAMETHREAD | verified |
| 4 | B2 shell: GameInit@0x2f731 = OPTIONS.BDL presence check (spawns SETUP.EXE when missing), LANGUAGE select, RNG seeds 123456/234567, tick install, palette alloc, EPISODE LOOP: briefing (BriefingScreen@0x5498b), MissionRun@0x57651, post-mission hub, MapRoomSelect@0x50a87 (player picks the sub-mission; gated by completed mask), zone-complete cutscene; exit linear > 27 | RESEARCH-BEDLAM2-CENSUS sec 6.1/7.3 | verified |
| 5 | B2 campaign state: g_campaign_linear/mask, stage slot, g_money; save records 5 x 61 B @0x8b1d4 {mask, stage-slot, linear, ..., money, stats}; sub-mission does NOT auto-advance | RESEARCH-BEDLAM2-CENSUS sec 7.3 | verified |
| 6 | Scene music is per-screen .MRS: BRIEF / DEBRIEF / OPTIONS / SELECT / SHOP .MRS in SOUND/MIDI (song slot 3 = the one MusicPump@00402bac sequences; load_midi FUN_00403642 builds base+".MRS") | RE-EXW-MUSIC sec 1 | verified |
| 7 | MusicPump semantics: every 100Hz tick, per enabled chunk (0x26-stride state): while delta==0 dispatch event (note-on/off, rest = skip, 0xFE conditional restart gated by loop flag word@0045cdc0[song], 0xFF unconditional restart = re-init all chunks of the channel), then delta--; table-B delay seeds the first countdown; shipped streams end in a freeze word = natural stop | RE-EXW-MUSIC sec 1/2 | verified |
| 8 | CONFIG.BDL (root, 61 B) is an installer/SB artifact EXW never reads; EXW persistence = SAVED.BDL + HISCORES only. OPTIONS.BDL (41 B, B2 SETUP-owned): backbuffer/actionpan/language/cd_audio/playername[8]/volume/code_no_title/midi/sound/installdrive (bedlam-assets bdl.rs, byte-verified round-trip) | RE-EXW-MUSIC sec 4, GROUNDWORK | verified |
| 9 | Pause: P latch toggles pause; MissionShell@0044771c busy-waits for P again, clears all latches | RE-EXW-INPUT | verified |

## 3. Scene FSM topology [design; shapes from facts 1-5]

    Boot -> Intro(first run only) -> Title
      Title -> NameEntry -> Briefing -> Mission
      Title -> Options (return -> Title)
    Hub = Select/MapRoom (+ Shop): the B2 episode-loop residence
      Briefing -> Mission; Mission outcome switch (fact 2):
        advance   -> next level -> Briefing (zone complete -> Cutscene -> Hub)
        restart   -> Briefing (checkpoint restore)
        retry     -> Briefing
        quit      -> Title
      Mission ~ Pause (modal, fact 9; returns to Mission)
    Cutscene(zone/end) -> Hub (zone+1) / End (linear > last) -> Title

The FSM is a SceneId + explicit transition set, not a call-stack: EXW
implements these as nested loops in GameMain, B2 as loops in GameInit; the
REIMPL flattens them into one state machine (the canonical skeleton
transition set; per-mission content is P4). Campaign progress (facts 3/5)
rides the FSM state, never the sim.

## 4. Host pump composition (per frame)

    input -> SimDriver::advance(dt_subticks)   [bedlam-core, bucket a]
          -> SceneFsm::step(input events)      [this crate, bucket a]
          -> Frame emit via bedlam-render      [pure; stub content]
          -> music pump: service-tick deltas -> Mixer::render [bucket b]

The host NEVER lets scene changes touch the sim clocks mid-tick; a scene
transition takes effect at the next tick boundary (the original re-enters
its loops at present boundaries the same way).

## 5. Music bridge (the MusicPump analog; DESIGN-AUDIO sec 7)

`music.rs`: Mrs -> MusicScript for the one song slot the pump sequences.

- Walk every ENABLED chunk (start table != 0xffff; fact 7 - the pump runs all
  chunks of the song in parallel), accumulate each chunk delta stream into
  ABSOLUTE ticks seeded from its table-B delay, interleave by absolute tick
  (stable: chunk order, then walk order), map Note{volume != 0xFF} ->
  NoteOn{instrument, ratio, volume}, volume == 0xFF -> NoteOff{instrument}
  (the base-only release quirk lives in the mixer, not here).
- Rest events vanish (they only advance time). SongEnd terminates the walk
  (script stops). Restart: unconditional 0xFF (or 0xFE with loop flag) means
  the HOST re-inits the script (loop); the bridge itself emits a one-pass
  script and reports the restart so the host can loop. Every shipped stream
  ends in Freeze (natural stop), so the corpus scripts are one-shot.
- SFX bypass the script (host-event note_on, per DESIGN-AUDIO sec 7).

## 6. Config and save model [facts 5/8]

- `GameConfig`: typed view of OPTIONS.BDL via bedlam-assets::bdl
  (language, player name, volume, midi/sound toggles, cd_audio, drive) plus
  `Default` for missing-file boot (B2 spawns SETUP; the skeleton just boots
  with defaults and flags config_present: false).
- `Campaign` (in-FSM): linear 0..=26, zone 1..=7, level 1..=5, completed
  mask, stage slot, money (facts 3/5). Save RECORDS (5 x 61 B) are P4
  persistence; this crate only defines the sink trait (sec 8).

## 7. Determinism boundary (D17)

- HASHED (bucket a): sim state (bedlam-core Sim::state_hash) AND scene FSM
  state (SceneFsm::state_hash: scene id, tick-in-scene, campaign fields,
  outcome latch, quit flag) - scene state is canonical sim-visible state.
- UNHASHED (bucket b): render frame artifacts, cursor/presentation, the
  music mixer and its pump. The music pump advances by SERVICE TICKS (the
  sim 100Hz satellite, integer), never by host dt, so replaying a session
  reproduces the same mix stream bit-for-bit without it being hashed.

## 8. Hermetic rule

`#![forbid(unsafe_code)]`; no file I/O anywhere. Bytes cross the boundary
through two traits the P4 platform injects:

    pub trait ByteSource { fn read(&self, name: &str) -> Result<Vec<u8>, GameError>; }
    pub trait ByteSink   { fn write(&self, name: &str, bytes: &[u8]) -> Result<(), GameError>; }

Asset parsing stays in bedlam-assets; this crate composes parsed types only.

## 9. Type sketch (API as implemented by the skeleton)

    pub enum SceneId { Boot, Intro, Title, NameEntry, Options, Briefing,
                       Mission, MissionOutcome, Hub, Cutscene, End }
    pub struct SceneFsm { /* current, tick_in_scene, campaign, latches */ }
    impl SceneFsm {
        pub fn new() -> SceneFsm;
        pub fn step(&mut self, input: &InputFrame) -> SceneStep;  // transitions
        pub fn scene(&self) -> SceneId;
        pub fn campaign(&self) -> &Campaign;
        pub fn state_hash(&self) -> StateHash;                    // bucket a
    }
    pub struct GameHost { /* sim_driver, fsm, mixer, music clock */ }
    impl GameHost {
        pub fn new(config: &GameConfig) -> GameHost;
        pub fn advance(&mut self, dt_subticks: u32, input: &InputFrame) -> HostFrame;
        pub fn sim(&self) -> &Sim;  pub fn scene(&self) -> &SceneFsm;
        pub fn state_hash(&self) -> StateHash;  // sim hash + scene hash layout
    }
    pub fn mrs_to_script(mrs: &Mrs) -> Result<BridgedMusic, GameError>;
    pub struct GameConfig { /* typed OPTIONS.BDL + present flag */ }
    impl GameConfig { pub fn from_options_bdl(bytes: &[u8]) -> Result<GameConfig, GameError>; }

Errors: thiserror only; no new dependencies beyond the sibling crates.

## 10. Testing

- Unit: FSM topology walk (scripted inputs drive Boot through End along every
  RE-anchored edge incl. pause modal + outcome switch), campaign stride
  (linear/zone/level round-trip over 1..=26), config parse on the real
  41-B layout + defaults.
- Integration tests/determinism.rs: same scripted input -> identical
  GameHost::state_hash at 15/60/240Hz host frame rates (dt 16/4/1 subticks).
- Integration tests/music_bridge.rs: for each of the 5 corpus .MRS:
  bridge == walk (event-for-event, absolute ticks = cumulative deltas + B
  seed), NoteOn/NoteOff split correct, script pushes into the mixer, and the
  rendered mix is byte-identical under 1/7/64-frame chunking.
- cargo fmt + clippy -D warnings; workspace suite stays green.

## 11. Open questions (each names its answer source)

- Q1: EXW Title/menu structure below GameMain (FUN_0043e7d4 modal set, shop,
  debrief screens) is only structurally known -> P2e/P4 Ghidra pass over
  FUN_0043a48d/FUN_0043e7d4/FUN_0044745e; FSM edges marked [skeleton] until then.
- Q2: scene music START moments (which screen arms which .MRS base name) -
  the 5 base names are the scene set (fact 6) but the arm points are mostly
  B2-side -> follow-up census pass; host exposes music_for_scene as data.
- Q3: save-record writer (5 x 61 B) semantics -> P4 with the harness.
- Q4: mission seeds per level (RNG planting is boot-global 123456/234567 in
  both builds) -> P4 parity harness.

## Provenance

Written 2026-08-18 by the item-1 worker from RE-EXW-GAMETHREAD, RE-EXW-MUSIC
sec 1/2/4, RE-EXW-INPUT, RESEARCH-BEDLAM2-CENSUS sec 6/7, DESIGN-RENDER
sec 5/6, DESIGN-AUDIO sec 4/7, DECISIONS D9/D12/D17, PLAN sec 6 P3. RE facts
carry anchors; FSM flattening and pump wiring are [design].
