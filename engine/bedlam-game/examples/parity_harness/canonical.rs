//! The W6 canonical dump emitter (DESIGN-DIFFHARNESS.md §6a + §10-W6,
//! D85) — the E side of the differ.
//!
//! Two halves:
//!
//! 1. **The field maps** ([`TickState`] → [`emit_frame`]): the
//!    canonical record grammar per DESIGN §6a — little-endian, no
//!    padding, fixed field order per row, registry ids. This is the
//!    CONTRACT: W7's normalizer must convert O1/O2 raw guest bytes
//!    into the same grammar. Rows the engine does not model are
//!    E-gaps (listed in §6a) and are simply not emitted.
//! 2. **The runner** ([`run_canonical`]): consumes the SAME scenario
//!    grammar as the O1 side (the D82 shared seam — literally the
//!    `diffharness::runner` parser), drives `GameHost` one mission
//!    frame per boundary (tick + present), snapshots the state at the
//!    frame tail, and stitches the dump through the SAME
//!    validation/encode path as O1 captures (`runner::stitch` +
//!    `encode_dump`, channel E). Byte-deterministic by construction.
//!
//! Frame model (§6a): one record per `pump_frame(dt=4)`; the ANCHOR
//! record is the tail of the FIRST mission tick (`frame_no` 0, the TS
//! statics ride it); then strictly increasing; total records equal
//! anchor plus `frames`. `frame-counter` is the PRE-increment value
//! (`sim.frame()−1`), matching the O1 dump point (its counter
//! increments only after the flip).
//!
//! Scenario step semantics (§6a): walk phase may carry ONLY `boot`
//! steps (any other walk step — the S0W menu-walk shape — is rejected;
//! the E menu-walk seam waits on the P2e button bit-map); `keystore`
//! maps through the pinned EMPTY scan map (no engine keyboard
//! consumer yet; P 0x19 rejected per the §2 pause rule); `order x y
//! z` is the click-order seam (target recorded + `arm_order_at_robot`
//! at the tile-exact alive robot); `command`/`pad` are rejected
//! naming the missing engine seams; `boot difficulty=d` seeds the
//! campaign money via the engine's own `menu::start_score` formula.
//! The `markers` header key (D91) stages extra squad robots through
//! the existing `load_mission(staged_markers)` seam after the MRK
//! robots — the walk seam (the click-order moves only the OTHER
//! robots in radius, so order→walk scenarios stage a walker).

use std::fmt;
use std::path::{Path, PathBuf};

use bedlam_core::input::InputFrame;
use bedlam_core::mission::Robot;
use bedlam_core::sim::SimConfig;
use bedlam_game::{ByteSource, GameConfig, GameError, GameHost, SceneAction};
use diffharness::dump::{Channel, DumpHeader, FrameRecord};
use diffharness::hash::sha256;
use diffharness::runner::{Scenario, Step, StitchError, Stitched, Transcript};
use diffharness::Watch;

/// Host sub-ticks per canonical frame: the 240 Hz grid quantized to
/// the 60 Hz tick (exactly one executed tick per frame; the runner
/// errors on any other cadence).
const DT_SUBTICKS: u32 = 4;

/// Mission-phase scan codes with no E-side consumer are rejected only
/// when the engine could never honor them: P-pause freezes the shell
/// in a present-only spin (DESIGN §2 — the runner must never inject
/// it mid-scenario).
const BANNED_SCANS: [u8; 1] = [0x19];

// ---------------------------------------------------------------------
// The canonical field maps (DESIGN §6a)
// ---------------------------------------------------------------------

/// One frame's engine state, snapshotted at the frame tail. Built by
/// the runner from `(&GameHost, session)` or by hand in tests (the
/// synthetic fixture — every field is plain data on purpose).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickState<'a> {
    /// Pre-increment mission frame counter (the O1 dump-point value).
    pub frame_no: u64,
    pub rand_a_state: u64,
    pub rand_b_state: u64,
    pub score: i32,
    pub money: i32,
    pub difficulty: u32,
    pub zone: u32,
    pub mission: u32,
    pub mode: u32,
    pub linear: u32,
    pub robots: &'a [Robot],
    /// The armed click order (beacon-family + spread-claims source).
    pub order: Option<bedlam_core::mission::Order>,
    pub selected: usize,
    pub blink_cursor: i32,
    /// The ORDER-seam write (persists like the 0x4dd484 cells).
    pub order_target: (i32, i32, i32),
    pub armor_pads: &'a [u8],
    /// TS statics ride the anchor frame only.
    pub map_wh: Option<(u32, u32)>,
}

fn u32b(v: u32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// Encode the robot bank: u32 count + per-robot records in the
/// `MissionSim::state_hash` field order (the pinned modeled-field
/// list — the W7 normalizer must emit the same order).
fn robot_bank_blob(robots: &[Robot]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + robots.len() * 96);
    b.extend_from_slice(&(robots.len() as u32).to_le_bytes());
    for r in robots {
        b.push(u8::from(r.alive));
        b.extend_from_slice(&r.pos_x.to_le_bytes());
        b.extend_from_slice(&r.pos_y.to_le_bytes());
        b.extend_from_slice(&r.z.to_le_bytes());
        b.extend_from_slice(&r.state.to_le_bytes());
        b.extend_from_slice(&r.dir_byte.to_le_bytes());
        b.extend_from_slice(&r.facing.to_le_bytes());
        b.extend_from_slice(&r.anim.to_le_bytes());
        b.extend_from_slice(&r.variant.to_le_bytes());
        for z in r.probe_z {
            b.extend_from_slice(&z.to_le_bytes());
        }
        b.extend_from_slice(&r.stop_dist.to_le_bytes());
        match r.target {
            None => {
                b.push(0);
                b.extend_from_slice(&0i32.to_le_bytes());
                b.extend_from_slice(&0i32.to_le_bytes());
            }
            Some((tx, ty)) => {
                b.push(1);
                b.extend_from_slice(&tx.to_le_bytes());
                b.extend_from_slice(&ty.to_le_bytes());
            }
        }
        b.extend_from_slice(&r.drop_countdown.to_le_bytes());
        b.extend_from_slice(&r.hp.to_le_bytes());
        b.extend_from_slice(&r.armor.to_le_bytes());
        b.extend_from_slice(&r.hit_flash.to_le_bytes());
        b.extend_from_slice(&r.alarm.to_le_bytes());
        b.extend_from_slice(&r.kind.to_le_bytes());
        b.extend_from_slice(&r.shield.to_le_bytes());
        b.extend_from_slice(&r.shield_charges.to_le_bytes());
        b.extend_from_slice(&r.shield_boost.to_le_bytes());
        b.extend_from_slice(&r.battery.to_le_bytes());
        b.extend_from_slice(&r.armor_pool.to_le_bytes());
        b.extend_from_slice(&r.alarm_ctr.to_le_bytes());
        b.extend_from_slice(&r.death_flag.to_le_bytes());
    }
    b
}

/// Emit one canonical frame record: the §6a rows whose tier the
/// scenario captures (`tiers` = the scenario's tier list; `anchor`
/// adds the TS rows — they ride the mission-start frame only).
/// Registry-unknown ids are impossible here (the ids are literals
/// below); `encode_dump` re-validates and orders them anyway.
pub fn emit_frame(st: &TickState, tiers: &[String], injected: bool, anchor: bool) -> FrameRecord {
    let mut f = FrameRecord::new(st.frame_no, injected);
    let want = |tier: &str| tiers.iter().any(|t| t == tier);
    if want("T0") {
        f.push_watch("frame-counter", u32b(st.frame_no as u32));
        f.push_watch("rng-state-a", st.rand_a_state.to_le_bytes());
        f.push_watch("rng-state-b", st.rand_b_state.to_le_bytes());
        f.push_watch("score", u32b(st.score as u32));
        f.push_watch("money", u32b(st.money as u32));
        f.push_watch("difficulty", u32b(st.difficulty));
        f.push_watch("zone", u32b(st.zone));
        f.push_watch("mission", u32b(st.mission));
        f.push_watch("mode", u32b(st.mode));
        f.push_watch("linear-mission-m", u32b(st.linear));
    }
    if want("T1") {
        f.push_watch("robot-bank", robot_bank_blob(st.robots));
        // The 4-byte alias form (the D83 anti-fabrication precedent).
        f.push_watch("selection-triple", u32b(st.selected as u32));
        f.push_watch("blink-cursor", u32b(st.blink_cursor as u32));
        let mut players = Vec::with_capacity(48);
        let sel = st.robots.get(st.selected);
        for p in 0..4 {
            let r = if p == 0 { sel } else { None };
            let (x, y, z) = match r {
                Some(r) => (r.pos_x >> 8, r.pos_y >> 8, r.z),
                None => (0, 0, 0),
            };
            players.extend_from_slice(&x.to_le_bytes());
            players.extend_from_slice(&y.to_le_bytes());
            players.extend_from_slice(&z.to_le_bytes());
        }
        f.push_watch("per-player-selected", players);
        let mut target = Vec::with_capacity(12);
        for v in [st.order_target.0, st.order_target.1, st.order_target.2] {
            target.extend_from_slice(&v.to_le_bytes());
        }
        f.push_watch("order-target", target);
        let mut moves = Vec::with_capacity(4 + st.robots.len() * 9);
        moves.extend_from_slice(&(st.robots.len() as u32).to_le_bytes());
        for r in st.robots {
            match r.target {
                None => {
                    moves.push(0);
                    moves.extend_from_slice(&0i32.to_le_bytes());
                    moves.extend_from_slice(&0i32.to_le_bytes());
                }
                Some((tx, ty)) => {
                    moves.push(1);
                    moves.extend_from_slice(&tx.to_le_bytes());
                    moves.extend_from_slice(&ty.to_le_bytes());
                }
            }
        }
        f.push_watch("move-target-words", moves);
        let mut beacon = Vec::with_capacity(20);
        match st.order {
            None => {
                beacon.extend_from_slice(&0u32.to_le_bytes());
                beacon.extend_from_slice(&0u32.to_le_bytes());
                beacon.extend_from_slice(&0i32.to_le_bytes());
                beacon.extend_from_slice(&0i32.to_le_bytes());
                beacon.extend_from_slice(&0i32.to_le_bytes());
            }
            Some(o) => {
                beacon.extend_from_slice(&1u32.to_le_bytes());
                beacon.extend_from_slice(&(o.window as u32).to_le_bytes());
                beacon.extend_from_slice(&o.tile.0.to_le_bytes());
                beacon.extend_from_slice(&o.tile.1.to_le_bytes());
                beacon.extend_from_slice(&o.tile.2.to_le_bytes());
            }
        }
        f.push_watch("beacon-family", beacon);
        let mut claims = Vec::with_capacity(24);
        match st.order {
            None => claims.extend_from_slice(&[0u8; 24]),
            Some(o) => {
                for c in o.claims {
                    claims.extend_from_slice(&u16::from(c).to_le_bytes());
                }
            }
        }
        f.push_watch("spread-claims", claims);
        let mut pads = Vec::with_capacity(4 + st.armor_pads.len());
        pads.extend_from_slice(&(st.armor_pads.len() as u32).to_le_bytes());
        pads.extend_from_slice(st.armor_pads);
        f.push_watch("typedb-fade-byte", pads.clone());
        f.push_watch("armor-pad-reads", pads);
    }
    if anchor && want("TS") {
        let (w, h) = st.map_wh.unwrap_or((0, 0));
        let mut wh = Vec::with_capacity(8);
        wh.extend_from_slice(&w.to_le_bytes());
        wh.extend_from_slice(&h.to_le_bytes());
        f.push_watch("static-map-wh", wh);
    }
    f
}

// ---------------------------------------------------------------------
// The runner (scenario → engine frames → stitched dump)
// ---------------------------------------------------------------------

/// Canonical-run failures (scenario shape, engine seams, IO).
#[derive(Debug)]
pub struct CanonicalError(pub String);

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "canonical: {}", self.0)
    }
}

impl std::error::Error for CanonicalError {}

impl From<StitchError> for CanonicalError {
    fn from(e: StitchError) -> Self {
        CanonicalError(format!("stitch: {e}"))
    }
}

impl From<GameError> for CanonicalError {
    fn from(e: GameError) -> Self {
        CanonicalError(format!("game: {e}"))
    }
}

/// Filesystem byte source for a canonical run: the install tree plus
/// the mission/graphics subtrees the asset names assume (the
/// `EDITOR\` / `GAMEGFX\` prefixes RE-EXW names carry as path halves).
pub struct MissionSource {
    root: PathBuf,
}

impl MissionSource {
    pub fn new(root: impl AsRef<Path>) -> MissionSource {
        MissionSource {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl ByteSource for MissionSource {
    fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
        for prefix in ["", "EDITOR/", "GAMEGFX/", "SOUND/MIDI/"] {
            let path = self.root.join(prefix).join(name);
            if path.is_file() {
                return std::fs::read(&path).map_err(|e| GameError::AssetMissing {
                    name: format!("{name}: {e}"),
                });
            }
        }
        Err(GameError::AssetMissing {
            name: name.to_string(),
        })
    }
}

/// The pinned scan→InputFrame map. EMPTY in W6: the engine has no
/// mission keyboard consumer yet (the P2e button bit-map assignment;
/// RE-EXW-INPUT line 95). Keystroke steps still mark their frame
/// injected; the bit-map lands here when P2e does.
fn scan_input(_entries: &[(u8, u8)]) -> InputFrame {
    InputFrame::default()
}

/// Session context fixed at run start (the boot + episode scalars).
struct Session {
    difficulty: u32,
    zone: u32,
    mission: u32,
    linear: u32,
}

/// Run one scenario canonically and stitch the channel-E W3 dump.
///
/// The scenario must be walk-phase-empty (E-gaps the menu-walk seam)
/// and may carry only `step`/`keystore`/`order` mission steps
/// (`command`/`pad` name their missing engine seams). Returns the
/// same [`Stitched`] shape the O1 stitcher produces (bytes +
/// manifest); callers write the dump under runtime/ (§3 hygiene).
pub fn run_canonical(scenario_src: &str, root: &Path) -> Result<Stitched, CanonicalError> {
    let scen = Scenario::parse(scenario_src).map_err(|e| CanonicalError(format!("{e}")))?;
    let (walk, mission) = scen.phases();
    // Walk phase: ONLY Boot steps are consumable (the difficulty seed
    // below). Any other walk step (keystore/order/command/pad — the
    // S0W menu-walk shape) names the missing seam: the E walk waits on
    // the P2e InputFrame button bit-map. The grammar itself pins Boot
    // to the walk phase (parser rejects post-anchor boot), so the
    // `unreachable!` in the mission loop below is parser-guaranteed.
    for step in walk {
        if !matches!(step, Step::Boot { .. }) {
            return Err(CanonicalError(
                "walk-phase steps have no E-side seam yet (the P2e InputFrame button \
                 bit-map assignment); run the scenario on the O1 channel"
                    .into(),
            ));
        }
    }
    // BOOT (walk-phase-only by grammar): difficulty seeds the campaign
    // money via the engine's own formula (menu.rs start_score).
    let mut difficulty = 0u32;
    for step in walk {
        if let Step::Boot { key, value } = step {
            debug_assert_eq!(key, "difficulty", "grammar pins the boot key set");
            difficulty = u32::try_from(*value).unwrap_or(0);
        }
    }

    let mut source = MissionSource::new(root);
    let config = GameConfig::load(&mut source)?;
    let palette = [[0u8, 0, 0]; 256];
    let mut host = GameHost::new(&config, &SimConfig::default(), palette);

    // Stage the mission from the episode slot's asset names. The
    // fetch-order mapping below is pinned by suffix asserts (anti-drift).
    let names = host.mission_asset_names();
    let pins: [(usize, &str); 19] = [
        (0, ".TOT"),
        (1, ".DAT"),
        (2, ".PAD"),
        (3, ".CGR"),
        (4, ".BIN"),
        (5, ".LNK"),
        (6, "SINTABLE.BIN"),
        (7, "DANTE.BIN"),
        (8, "GAMEPAL.PAL"),
        (9, "GENERAL.BIN"),
        (10, "SMLFONT.BIN"),
        (11, ".MRK"),
        (12, "TABLE.BIN"),
        (13, "MAPTRAN0.TRN"),
        (20, "MAPTRAN7.TRN"),
        (21, ".MIN"),
        (22, "NUMBERS.BIN"),
        (23, "FLAGS.BIN"),
        (24, "BLOWUP.BIN"),
    ];
    if names.len() != 25 {
        return Err(CanonicalError(format!(
            "expected 25 mission asset names, got {}",
            names.len()
        )));
    }
    for (idx, suffix) in pins {
        if !names[idx].ends_with(suffix) {
            return Err(CanonicalError(format!(
                "asset name {idx} drift: {:?} does not end in {suffix:?}",
                names[idx]
            )));
        }
    }
    let bytes: Vec<Vec<u8>> = names
        .iter()
        .map(|n| source.load(n))
        .collect::<Result<_, _>>()?;
    let maptran: Vec<&[u8]> = bytes[13..21].iter().map(Vec::as_slice).collect();
    host.load_mission(
        &bytes[0],
        &bytes[1],
        &bytes[2],
        &bytes[3],
        &bytes[4],
        &bytes[5],
        &bytes[6],
        &bytes[7],
        &bytes[8],
        &bytes[9],
        &bytes[10],
        &bytes[11],
        &bytes[23],
        &bytes[24],
        &bytes[12],
        &maptran,
        &bytes[21],
        &bytes[22],
        None,
        &scen.markers,
    )?;
    if difficulty != 0 {
        let money = bedlam_game::menu::start_score(difficulty as u8);
        host.mission_mut()
            .expect("mission staged")
            .set_campaign(0, money);
    }

    // Session scalars from the episode (fresh host: ZONEA/MISSION1).
    let (zone, mission_no) = host.mission_slot();
    let session = Session {
        difficulty,
        zone: zone as u32,
        mission: mission_no as u32,
        linear: u32::from(host.fsm().episode().linear()),
    };

    // Boot hold → Title → Brief → Select → Mission, then the
    // activation frame (the mission is INERT during it; sync_mission
    // activates at the pump tail).
    let null = InputFrame::default();
    let mut guard = 0u32;
    while host.scene() == bedlam_game::Scene::Boot {
        let executed = host.pump_frame(DT_SUBTICKS, &null);
        check_cadence(executed)?;
        guard += 1;
        if guard > 600 {
            return Err(CanonicalError("boot hold never ended".into()));
        }
    }
    host.apply(SceneAction::Advance); // Title -> Brief
    host.apply(SceneAction::Advance); // Brief -> Select
    host.apply(SceneAction::Advance); // Select -> Mission
    if host.scene() != bedlam_game::Scene::Mission {
        return Err(CanonicalError(format!(
            "FSM did not reach Mission (at {:?})",
            host.scene()
        )));
    }
    let executed = host.pump_frame(DT_SUBTICKS, &null); // activation frame
    check_cadence(executed)?;

    // The anchor record: the tail of the FIRST mission tick (TS rides).
    let executed = host.pump_frame(DT_SUBTICKS, &null);
    check_cadence(executed)?;
    let map_wh = host
        .mission()
        .map(|m| m.view_size())
        .map(|(w, h)| (w as u32, h as u32));
    let mut frames = vec![emit_frame(
        &tick_state(&host, &session, (0, 0, 0), 0, map_wh),
        &scen.tiers,
        false,
        true,
    )];

    // Mission phase: one boundary per frame; injections apply BEFORE
    // the tick (§5: between the previous present and this input read).
    let total = scen.frames + 1; // the stitcher contract
    let mut seam_target = (0i32, 0i32, 0i32);
    'outer: for step in mission {
        match *step {
            Step::Advance { frames: n } => {
                for _ in 0..n {
                    if frames.len() >= total as usize {
                        break 'outer;
                    }
                    let executed = host.pump_frame(DT_SUBTICKS, &null);
                    check_cadence(executed)?;
                    let no = frame_counter_now(&host);
                    frames.push(emit_frame(
                        &tick_state(&host, &session, seam_target, no, None),
                        &scen.tiers,
                        false,
                        false,
                    ));
                }
            }
            Step::Keystore { ref entries } => {
                if frames.len() >= total as usize {
                    break;
                }
                for (scan, _) in entries {
                    if BANNED_SCANS.contains(scan) {
                        return Err(CanonicalError(format!(
                            "keystore scan 0x{scan:02x} (P-pause) is banned mid-scenario \
                             (DESIGN §2)"
                        )));
                    }
                }
                let input = scan_input(entries);
                let executed = host.pump_frame(DT_SUBTICKS, &input);
                check_cadence(executed)?;
                let no = frame_counter_now(&host);
                frames.push(emit_frame(
                    &tick_state(&host, &session, seam_target, no, None),
                    &scen.tiers,
                    true,
                    false,
                ));
            }
            Step::Order { x, y, z } => {
                if frames.len() >= total as usize {
                    break;
                }
                seam_target = (x, y, z);
                if let Some(scene) = host.mission_mut() {
                    let pick = scene
                        .sim()
                        .robots()
                        .iter()
                        .position(|r| r.alive && r.tile() == (x, y));
                    if let Some(idx) = pick {
                        scene.sim_mut().arm_order_at_robot(idx);
                    }
                    // No robot at the tile: the pick fails (no arm),
                    // the target is still recorded — the seam write.
                }
                let executed = host.pump_frame(DT_SUBTICKS, &null);
                check_cadence(executed)?;
                let no = frame_counter_now(&host);
                frames.push(emit_frame(
                    &tick_state(&host, &session, seam_target, no, None),
                    &scen.tiers,
                    true,
                    false,
                ));
            }
            Step::Command { .. } => {
                return Err(CanonicalError(
                    "command steps need the engine fire family (S3 pairs it per \
                     DESIGN §10-W12); no E-side seam yet"
                        .into(),
                ));
            }
            Step::Pad { .. } => {
                return Err(CanonicalError(
                    "pad steps need the engine extraction arming (S6 pairs it per \
                     DESIGN §10-W12); no E-side seam yet"
                        .into(),
                ));
            }
            Step::Capture | Step::UntilAnchor { .. } => {} // runner directives
            Step::Boot { .. } => unreachable!("grammar pins boot to the walk phase"),
        }
    }
    // Past the step schedule the input stays null (§5: with no
    // injection the original polls zeros) — the `frames` budget
    // governs the capture length, not the schedule (S1-style
    // scenarios carry no steps at all).
    while frames.len() < total as usize {
        let executed = host.pump_frame(DT_SUBTICKS, &null);
        check_cadence(executed)?;
        let no = frame_counter_now(&host);
        frames.push(emit_frame(
            &tick_state(&host, &session, seam_target, no, None),
            &scen.tiers,
            false,
            false,
        ));
    }

    // Header: channel E, the engine identity as the build hash, the
    // determinism pins.
    let identity = format!("bedlam-game {}+canonical-1", env!("CARGO_PKG_VERSION"));
    let mut header = DumpHeader::new(
        Channel::Engine,
        sha256(identity.as_bytes()),
        scen.id.clone(),
    );
    header.push_pin("seed=0x1e240");
    header.push_pin(format!("dt_subticks={DT_SUBTICKS}"));
    header.push_pin(format!("difficulty={}", session.difficulty));
    header.push_pin(format!("zone={}", session.zone));
    header.push_pin(format!("mission={}", session.mission));
    header.push_pin("mode=sp");
    let reg: Vec<Watch> = diffharness::registry();
    let transcript = Transcript { frames };
    Ok(diffharness::runner::stitch(
        &scen,
        &transcript,
        &header,
        &reg,
    )?)
}

/// The pre-increment frame counter at the tail (`sim.frame()−1`).
fn frame_counter_now(host: &GameHost) -> u64 {
    host.mission().expect("mission staged").sim().frame() - 1
}

/// The per-frame emitter view of the live scene (§6a sources).
fn tick_state<'a>(
    host: &'a GameHost,
    session: &Session,
    seam_target: (i32, i32, i32),
    frame_no: u64,
    map_wh: Option<(u32, u32)>,
) -> TickState<'a> {
    let scene = host.mission().expect("mission staged");
    let sim = scene.sim();
    TickState {
        frame_no,
        rand_a_state: sim.rand_a_state(),
        rand_b_state: scene.rand_b_state(),
        score: scene.campaign().0,
        money: scene.campaign().1,
        difficulty: session.difficulty,
        zone: session.zone,
        mission: session.mission,
        mode: 0,
        linear: session.linear,
        robots: sim.robots(),
        order: sim.order(),
        selected: scene.sidebar_selected(),
        blink_cursor: scene.sidebar_cursor(),
        order_target: seam_target,
        armor_pads: sim.armor_pads(),
        map_wh,
    }
}

/// The canonical cadence contract: exactly one executed tick per
/// pumped frame at dt=4.
fn check_cadence(executed: u32) -> Result<(), CanonicalError> {
    if executed != 1 {
        return Err(CanonicalError(format!(
            "cadence break: {executed} ticks in one frame (want 1 at dt=4)"
        )));
    }
    Ok(())
}
