//! W4 runner support — the DOSBox-X diff-mode data formats + the stitcher
//! (DESIGN-DIFFHARNESS.md §3/§10-W4).
//!
//! The DH-G0 channel audit (docs/RUNTIME.md, 2026-08-22) established that
//! the pinned flathub DOSBox-X has NO integrated debugger and only a
//! log-only JS API, so the capture CHANNEL is not yet re-pinned. This
//! module is therefore deliberately channel-AGNOSTIC: whatever instrument
//! lands at DH-G0 (self-built debug DOSBox-X, GameLink, ptrace) only has
//! to emit the small **DBXCAP transcript** pinned here, and this code
//! turns it into the W3 dump + digest manifest.
//!
//! Two formats:
//!
//! **Scenario grammar v1.7** (`scenarios/*.scen`, committed):
//! ```text
//! # comment / blank lines
//! scenario = S0              ; id (dump header; <=255 bytes)
//! tiers = T0,TS              ; watch tiers this scenario captures
//! anchor = mission-start     ; symbolic anchor event (optional)
//! frames = 2                 ; per-frame records after the anchor frame
//! launch = DOS4GW.EXE BEDLAM.EXD   ; autoexec launch line (optional)
//! markers = 18,73,1; 20,74,1 ; extra squad robots (D91, v1.2)
//! loadout = 1,0x7,9:2        ; weapon slots per robot (D103, v1.3)
//! destroy = 1                ; stage .BDG/.POS/.TRT (D105, v1.4)
//! zone = "B"                 ; episode-slot zone letter (D108, v1.5)
//! pickup = 1                 ; stage the .TOT pickup surface (D108, v1.5)
//! platforms = 1              ; arm the epilogue creep tick (D113, v1.6)
//! critters = 1               ; stage .NME + arm the critter controller (D114, v1.7)
//! step 10                    ; advance N frames, no input      (runner)
//! capture                    ; force a frame dump              (runner)
//! until-anchor mission-start ; run to the anchor event         (runner)
//! keystore 0x1f=1,0x2a=0     ; KEYSTATE write, ONE frame boundary (W5)
//! order 29 18 0              ; ORDER target write, one boundary (W5)
//! pad 3                      ; .PAD step-on order, one boundary  (W5)
//! command fire 1 5 [flags 2] ; COMMAND record write, one boundary (W5)
//! boot difficulty=1          ; pre-mission BOOT setup, frame 0    (W5)
//! ```
//! Step directives are validated but do not drive the stitcher (the
//! transcript is the ground truth for what was captured). The W5
//! injection steps are the DESIGN §5 vocabulary as runner-side writes:
//! one script line per frame boundary, applied between the previous
//! frame's present and this frame's input read. `until-anchor` splits
//! the schedule for the compiler (dbx-plan): injection steps before it
//! are the WALK phase (pre-mission, e.g. the scripted menu walk), steps
//! after it are MISSION phase (anchor-relative frame numbers); with no
//! `until-anchor` step every injection step is mission phase. `boot`
//! steps apply at frame 0 (the arm stop, pre-walk).
//!
//! **DBXCAP transcript v1** (produced by the capture channel; lives under
//! runtime/ only — asset-derived data per D77 hygiene):
//! ```text
//! DBXCAP v1                  ; mandatory first directive
//! # comment
//! frame 7                    ; start a frame record (frame_no u64)
//! frame 7 1                  ; optional injected flag 0|1 (default 0)
//! watch frame-counter 07000000   ; hex bytes for the CURRENT frame
//! watch robot-bank            ; no hex = empty blob (count-driven 0)
//! ```
//! TS static-after-load rows ride the anchor frame as ordinary watch
//! rows (W3 convention). Watches accumulate until the next `frame`/EOF.
//!
//! Stitch validation (the anti-ghost guards): every transcript id must
//! exist in the committed registry, its tier must be among the
//! scenario's tiers, and — for the O1 channel — its `exd_addr` must be
//! non-empty (EXW-only rows never enter an EXD dump; gaps stay explicit).

use crate::dump::{self, Channel, DumpHeader, FrameRecord};
use crate::hash::{hex_lower, sha256};
use crate::Watch;
use std::fmt;

// ---------------------------------------------------------------------
// Scenario

/// One staged weapon-slot group (grammar v1.3 `loadout` key, W12-S3):
/// plain data — the canonical runner expands the pairs into the
/// engine's 7-slot array through `stage_robot_weapons`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadoutRobot {
    /// Robot bank index (0-based; the MRK robots then the `markers`
    /// order — the same indexing the COMMAND record's id uses).
    pub robot: usize,
    /// The enable mask word (robot +0x6E; bit k = slot k fires).
    pub mask: u16,
    /// (weapon-stat id, ammo) per slot, listing order = slot order.
    pub slots: Vec<(u16, i16)>,
}

/// One parsed scenario file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub id: String,
    pub tiers: Vec<String>,
    pub anchor: Option<String>,
    /// Per-frame records the runner must capture after the anchor frame.
    /// The stitcher requires `frames + 1` frame records in the transcript
    /// (anchor frame included; S0-style "first frame only" sets frames=0).
    pub frames: u64,
    pub launch: Option<String>,
    /// Extra squad markers staged after the MRK robots (D91, grammar
    /// v1.2 `markers` header key): the E walk seam — the click-order
    /// moves only the OTHER robots in radius, so order→walk scenarios
    /// stage a second robot. Bounded so MRK robots + markers ≤ 12
    /// (the bank cap cell discipline). No O1 write exists; consumers
    /// record the seam explicitly (never fabricate).
    pub markers: Vec<(i32, i32, i32)>,
    /// Staged weapon loadouts (grammar v1.3 `loadout` key, W12-S3):
    /// per-robot slot ids + the enable mask, staged through the
    /// engine's `stage_robot_weapons` host seam (the D51 pattern —
    /// the original fills the slots at spawn from the session table,
    /// which no engine path reaches; like `markers` this is an E-side
    /// staging seam, recorded never fabricated).
    pub loadout: Vec<LoadoutRobot>,
    /// The destroy-family staging key (grammar v1.4 `destroy = 1`,
    /// W12-S4): stage the mission's own .BDG type table + .POS
    /// instances + .TRT structures through the engine's
    /// `stage_destroy_family` host seam. The ORIGINAL loads all
    /// three files natively at mission load (FUN_0041a4f8 +
    /// FUN_004170a6, §7j.25/4) — this key exists because E's
    /// `load_mission` does not fetch them; the staged CONTENT is
    /// byte-identical to what O1 loads (no O1 write, no seam diff),
    /// so consumers record the key as an E-side equivalence seam.
    /// The key also gates the destroy-family dump rows (they ride
    /// only destroy-staging scenarios — S0..S3 bytes unchanged).
    pub destroy: bool,
    /// The episode-slot zone key (grammar v1.5 `zone = "B"`,
    /// W12-S5/D108): stage the campaign episode to the given zone
    /// letter (A..G → stage 1..7, mask 0 → MISSION1, linear stays
    /// the fresh-slot 0) through the host's `stage_episode_slot`
    /// seam — the host stands in for the campaign-advance
    /// (0x41c9e5) / save-load-restore (0x43c2b8) shells the engine
    /// does not model. `None` = the boot slot (ZONEA/MISSION1,
    /// every pre-S5 scenario). A LIVE O1 capture reaches other
    /// zones by playing the campaign, so its linear/mission
    /// counters are the live-capture seam — consumers record the
    /// zone seam, never fabricate the counters.
    pub zone: Option<char>,
    /// The pickup-surface staging key (grammar v1.5 `pickup = 1`,
    /// W12-S5/D108): stage the mission's OWN .TOT through the
    /// engine's `stage_pickup_surface` host seam (the init_tiles
    /// TOT fill + the zone/set cell) AFTER any destroy staging,
    /// then the §7j.12/6 hazard stamper — the original's
    /// mission-load order. The ORIGINAL stages the same TOT volume
    /// natively at mission load (FUN_00407e11), so the staged
    /// CONTENT is identical on both channels (an equivalence seam
    /// like `destroy`); it is a separate key because the two
    /// stagings gate different dump rows and S4 must keep its
    /// empty-staged mirror bytes (its chain is pinned).
    pub pickup: bool,
    /// The platform-family arm key (grammar v1.6 `platforms = 1`,
    /// W12-S7/D113): arm the epilogue platform CREEP tick
    /// (FUN_00422a9c, the MissionShell epilogue call 0x44808a).
    /// The ORIGINAL runs the tick EVERY frame — its 1/32 gate
    /// draws one RandA per frame from boot — while E arms it per
    /// scenario so the S0..S6 chains stay byte-identical (the
    /// per-frame gate draw on unarmed paths is the recorded
    /// E-gap, D113/§7j.41/4). Purely an E-side arming decision:
    /// O1 runs the tick natively, nothing is staged on the guest,
    /// consumers record the key in `_e_staging`.
    pub platforms: bool,
    /// The critter-family staging+arm key (grammar v1.7 `critters = 1`,
    /// W12-S8/D114): stage the mission's .NME through the
    /// FUN_00416458 spawn schedule (`stage_critters`) and ARM the
    /// controller (FUN_00412f34, MissionShell 0x447fe1). The
    /// ORIGINAL loads .NME natively at EVERY mission load and runs
    /// the controller ungated; E arms it per scenario so the
    /// S0..S7 chains stay byte-identical (the loader's kind-4
    /// heading draws + the controller's per-frame draws on
    /// unarmed paths are the recorded E-side stream gap,
    /// §7j.42/5). The critter bank is the E-ONLY T2 coverage row;
    /// the ALIASED observables are the RNG stream, the robot bank
    /// (the damage/stun lanes), the projectile bank (0x68 fire),
    /// the debris/effect-row stagings, and the score bounty.
    pub critters: bool,
    /// Validated step directives in file order (runner metadata).
    pub steps: Vec<Step>,
}

/// Scenario step directives (grammar v1.2 — runner directives + the
/// DESIGN §5 W5 injection vocabulary; v1.2 adds the scenario-level
/// `markers` staging key, D91 — see [`Scenario::markers`]).
///
/// Injection steps are FRAME-BOUNDARY writes: each step line applies at
/// the next frame boundary (between the previous frame's present and
/// this frame's input read, DESIGN §5); `step N` consumes N boundaries
/// with no writes. The first `until-anchor` splits WALK phase (before:
/// pre-mission, e.g. the scripted menu walk) from MISSION phase (after:
/// anchor-relative boundaries — the same numbering as capture frames).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Advance {
        frames: u64,
    },
    Capture,
    UntilAnchor {
        name: String,
    },
    /// KEYSTATE (§5.1): set g_keystore bytes for one frame boundary.
    /// Each entry is (scan code, held 0|1); unlisted scans unchanged.
    Keystore {
        entries: Vec<(u8, u8)>,
    },
    /// ORDER (§5.2): write the click-order target triple (skips the
    /// click/pick UI).
    Order {
        x: i32,
        y: i32,
        z: i32,
    },
    /// PAD step-on (§5.4): an ORDER whose target is .PAD slot `slot`'s
    /// tile (the sanctioned extraction armer).
    Pad {
        slot: u32,
    },
    /// COMMAND record (§5.3): append `bytes` (≤0x80) as the next
    /// record at the command ring; the runner reads the count cell,
    /// writes the record, bumps the count. Raw bytes on purpose — the
    /// builder-side field packing is pinned, the sugar comes with S3.
    Command {
        bytes: Vec<u8>,
    },
    /// BOOT setup (§5.5): pre-mission writes (frame 0, before the
    /// walk). Legal in WALK phase only.
    Boot {
        key: String,
        value: i64,
    },
}

#[derive(Debug)]
pub struct ScenarioError {
    line_no: usize,
    line: String,
    reason: String,
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scenario:{}: {} (line: {})",
            self.line_no, self.reason, self.line
        )
    }
}

fn scen_err(line_no: usize, line: &str, reason: &str) -> ScenarioError {
    ScenarioError {
        line_no,
        line: line.to_string(),
        reason: reason.to_string(),
    }
}

/// Integer with optional 0x/0X prefix, sign allowed (grammar numbers).
fn parse_num(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<i64>().ok()
    }
}

impl Scenario {
    /// Parse scenario grammar v1.
    pub fn parse(src: &str) -> Result<Scenario, ScenarioError> {
        let mut id: Option<String> = None;
        let mut tiers: Option<Vec<String>> = None;
        let mut anchor: Option<String> = None;
        let mut frames: Option<u64> = None;
        let mut launch: Option<String> = None;
        let mut markers: Vec<(i32, i32, i32)> = Vec::new();
        let mut loadout: Vec<LoadoutRobot> = Vec::new();
        let mut destroy = false;
        let mut zone: Option<char> = None;
        let mut pickup = false;
        let mut platforms = false;
        let mut critters = false;
        let mut steps: Vec<Step> = Vec::new();

        for (idx, raw) in src.lines().enumerate() {
            let line_no = idx + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let first = parts.next().unwrap_or("");
            match first {
                "step" => {
                    let n = parts
                        .next()
                        .ok_or_else(|| scen_err(line_no, line, "step needs a frame count"))?;
                    let n: u64 = n
                        .parse()
                        .map_err(|_| scen_err(line_no, line, "step frame count must be a u64"))?;
                    steps.push(Step::Advance { frames: n });
                }
                "capture" => steps.push(Step::Capture),
                "until-anchor" => {
                    let name = parts
                        .next()
                        .ok_or_else(|| scen_err(line_no, line, "until-anchor needs a name"))?;
                    steps.push(Step::UntilAnchor {
                        name: name.to_string(),
                    });
                }
                "keystore" => {
                    let mut entries = Vec::new();
                    for tok in parts {
                        let tok = tok.trim_end_matches(',');
                        let Some((scan_s, val_s)) = tok.split_once('=') else {
                            return Err(scen_err(
                                line_no,
                                line,
                                "keystore entries are scan=val pairs (e.g. 0x1f=1)",
                            ));
                        };
                        let scan: i64 = parse_num(scan_s).ok_or_else(|| {
                            scen_err(line_no, line, "keystore scan code must be an integer")
                        })?;
                        let val: i64 = parse_num(val_s).ok_or_else(|| {
                            scen_err(line_no, line, "keystore value must be 0 or 1")
                        })?;
                        if !(0..=0xFF).contains(&scan) {
                            return Err(scen_err(
                                line_no,
                                line,
                                "keystore scan code out of range 0..0xFF",
                            ));
                        }
                        if !(0..=1).contains(&val) {
                            return Err(scen_err(line_no, line, "keystore value must be 0 or 1"));
                        }
                        entries.push((scan as u8, val as u8));
                    }
                    if entries.is_empty() {
                        return Err(scen_err(
                            line_no,
                            line,
                            "keystore needs at least one scan=val pair",
                        ));
                    }
                    steps.push(Step::Keystore { entries });
                }
                "order" => {
                    let mut nums = [0i64; 3];
                    for (i, slot) in nums.iter_mut().enumerate() {
                        let tok = parts.next().ok_or_else(|| {
                            scen_err(line_no, line, "order needs x y z (3 integers)")
                        })?;
                        let _ = i;
                        *slot = parse_num(tok).ok_or_else(|| {
                            scen_err(line_no, line, "order coordinates must be integers")
                        })?;
                    }
                    if parts.next().is_some() {
                        return Err(scen_err(
                            line_no,
                            line,
                            "order takes exactly 3 values (x y z)",
                        ));
                    }
                    let [x, y, z] = nums.map(|v| v as i32);
                    steps.push(Step::Order { x, y, z });
                }
                "pad" => {
                    let tok = parts
                        .next()
                        .ok_or_else(|| scen_err(line_no, line, "pad needs a slot index"))?;
                    let slot: i64 = parse_num(tok)
                        .ok_or_else(|| scen_err(line_no, line, "pad slot must be an integer"))?;
                    if !(0..=998).contains(&slot) {
                        return Err(scen_err(
                            line_no,
                            line,
                            "pad slot out of range 0..998 (999 .PAD slots)",
                        ));
                    }
                    if parts.next().is_some() {
                        return Err(scen_err(line_no, line, "pad takes exactly one slot index"));
                    }
                    steps.push(Step::Pad { slot: slot as u32 });
                }
                "command" => {
                    let mut bytes = Vec::new();
                    for tok in parts {
                        let tok = tok.trim_end_matches(',');
                        if tok.len() != 2 || !tok.bytes().all(|b| b.is_ascii_hexdigit()) {
                            return Err(scen_err(
                                line_no,
                                line,
                                "command payload tokens are hex BYTE pairs (e.g. 01 3F)",
                            ));
                        }
                        bytes.push(u8::from_str_radix(tok, 16).expect("checked"));
                    }
                    if bytes.is_empty() {
                        return Err(scen_err(
                            line_no,
                            line,
                            "command needs at least one payload byte",
                        ));
                    }
                    if bytes.len() > 0x80 {
                        return Err(scen_err(
                            line_no,
                            line,
                            "command payload exceeds the 0x80 record stride",
                        ));
                    }
                    steps.push(Step::Command { bytes });
                }
                "boot" => {
                    let tok = parts
                        .next()
                        .ok_or_else(|| scen_err(line_no, line, "boot needs key=value"))?;
                    let Some((key, val_s)) = tok.split_once('=') else {
                        return Err(scen_err(
                            line_no,
                            line,
                            "boot entries are key=value (e.g. difficulty=1)",
                        ));
                    };
                    let value: i64 = parse_num(val_s)
                        .ok_or_else(|| scen_err(line_no, line, "boot value must be an integer"))?;
                    if !["difficulty"].contains(&key) {
                        return Err(scen_err(
                            line_no,
                            line,
                            "unknown boot key (known: difficulty)",
                        ));
                    }
                    if parts.next().is_some() {
                        return Err(scen_err(line_no, line, "boot takes exactly one key=value"));
                    }
                    steps.push(Step::Boot {
                        key: key.to_string(),
                        value,
                    });
                }
                other_key => {
                    // key = value
                    let Some(eq) = line.find('=') else {
                        return Err(scen_err(
                            line_no,
                            line,
                            "expected `key = value`, a `# comment`, or a step directive",
                        ));
                    };
                    let key = line[..eq].trim();
                    let value = line[eq + 1..].trim().trim_matches('"');
                    match key {
                        "scenario" => id = Some(value.to_string()),
                        "tiers" => {
                            let mut ts = Vec::new();
                            for t in value.split(',') {
                                let t = t.trim();
                                if t.is_empty() {
                                    return Err(scen_err(line_no, line, "empty tier in list"));
                                }
                                ts.push(t.to_string());
                            }
                            if ts.is_empty() {
                                return Err(scen_err(line_no, line, "tiers must not be empty"));
                            }
                            tiers = Some(ts);
                        }
                        "anchor" => anchor = Some(value.to_string()),
                        "frames" => {
                            frames =
                                Some(value.parse().map_err(|_| {
                                    scen_err(line_no, line, "frames must be a u64")
                                })?);
                        }
                        "launch" => launch = Some(value.to_string()),
                        "markers" => {
                            // D91: `x,y,z` triples, `;`-separated. The
                            // squad bank is capped at 12 records (the
                            // spread table + move-target arrays are
                            // 12-wide; EXD cap cell 0x11950c) and the
                            // zone rule stages up to 3 MRK robots, so
                            // at most 9 markers may ride a scenario.
                            for triple in value.split(';') {
                                let nums: Vec<i64> = triple
                                    .split(',')
                                    .map(parse_num)
                                    .collect::<Option<Vec<_>>>()
                                    .ok_or_else(|| {
                                        scen_err(
                                            line_no,
                                            line,
                                            "markers entries are x,y,z integer triples",
                                        )
                                    })?;
                                if nums.len() != 3 {
                                    return Err(scen_err(
                                        line_no,
                                        line,
                                        "markers entries are x,y,z triples (exactly 3 values)",
                                    ));
                                }
                                markers.push((nums[0] as i32, nums[1] as i32, nums[2] as i32));
                            }
                            if markers.is_empty() {
                                return Err(scen_err(line_no, line, "markers must not be empty"));
                            }
                            if markers.len() > 9 {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "markers exceed the bank cap: MRK robots + markers must be \
                                     <= 12 (max 9 markers)",
                                ));
                            }
                        }
                        "loadout" => {
                            // W12-S3 (grammar v1.3): per-robot entries
                            // `idx,mask,id:ammo[,id:ammo...]`, `;`-separated.
                            // The slots stage through the engine host seam
                            // (stage_robot_weapons); the bounds mirror the
                            // original structures — 12 robots max, 7 slots,
                            // ids in the consumer's 2..=0x28 dispatch domain,
                            // positive i16 ammo, and no mask bit beyond the
                            // staged slots (auto-rearm never arms an empty
                            // slot, so such a bit is a scenario typo).
                            for entry in value.split(';') {
                                let fields: Vec<&str> = entry
                                    .split(',')
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                if fields.len() < 3 {
                                    return Err(scen_err(
                                        line_no,
                                        line,
                                        "loadout entries are idx,mask,id:ammo[,id:ammo...]",
                                    ));
                                }
                                let robot = parse_num(fields[0]).ok_or_else(|| {
                                    scen_err(
                                        line_no,
                                        line,
                                        "loadout robot index must be an integer",
                                    )
                                })?;
                                if !(0..=11).contains(&robot) {
                                    return Err(scen_err(
                                        line_no,
                                        line,
                                        "loadout robot index out of range 0..11 (the 12-robot bank cap)",
                                    ));
                                }
                                let mask = parse_num(fields[1]).ok_or_else(|| {
                                    scen_err(line_no, line, "loadout mask must be an integer")
                                })?;
                                if !(0..=0x7F).contains(&mask) {
                                    return Err(scen_err(
                                        line_no,
                                        line,
                                        "loadout mask out of range 0..0x7F (7 weapon slots)",
                                    ));
                                }
                                let mut slots = Vec::new();
                                for f in &fields[2..] {
                                    let Some((id_s, ammo_s)) = f.split_once(':') else {
                                        return Err(scen_err(
                                            line_no,
                                            line,
                                            "loadout slots are id:ammo pairs (e.g. 9:2)",
                                        ));
                                    };
                                    let id = parse_num(id_s).ok_or_else(|| {
                                        scen_err(
                                            line_no,
                                            line,
                                            "loadout slot id must be an integer",
                                        )
                                    })?;
                                    if !(2..=0x28).contains(&id) {
                                        return Err(scen_err(
                                            line_no,
                                            line,
                                            "loadout slot id out of the dispatch domain 2..0x28",
                                        ));
                                    }
                                    let ammo = parse_num(ammo_s).ok_or_else(|| {
                                        scen_err(
                                            line_no,
                                            line,
                                            "loadout slot ammo must be an integer",
                                        )
                                    })?;
                                    if !(1..=0x7FFF).contains(&ammo) {
                                        return Err(scen_err(
                                            line_no,
                                            line,
                                            "loadout slot ammo out of range 1..0x7FFF",
                                        ));
                                    }
                                    slots.push((id as u16, ammo as i16));
                                }
                                if slots.len() > 7 {
                                    return Err(scen_err(
                                        line_no,
                                        line,
                                        "loadout exceeds 7 weapon slots",
                                    ));
                                }
                                let mask = mask as u16;
                                let staged = slots.len() as u32;
                                if (mask >> staged) != 0 {
                                    return Err(scen_err(
                                        line_no,
                                        line,
                                        "loadout mask arms a slot beyond the staged list \
                                         (auto-rearm never arms an empty slot)",
                                    ));
                                }
                                let robot = robot as usize;
                                if loadout.iter().any(|l: &LoadoutRobot| l.robot == robot) {
                                    return Err(scen_err(
                                        line_no,
                                        line,
                                        "loadout stages the same robot twice",
                                    ));
                                }
                                loadout.push(LoadoutRobot { robot, mask, slots });
                            }
                            if loadout.is_empty() {
                                return Err(scen_err(line_no, line, "loadout must not be empty"));
                            }
                        }
                        "destroy" => {
                            // W12-S4 (grammar v1.4): the boolean
                            // destroy-family staging key — `destroy = 1`
                            // stages the mission's own .BDG/.POS/.TRT
                            // through the engine host seam. Strictly `1`
                            // (a typo'd value must fail loud, not
                            // silently skip the staging + its dump rows).
                            if value.trim() != "1" {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "destroy key is boolean staging: use `destroy = 1`",
                                ));
                            }
                            if destroy {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "destroy staged twice (one key per scenario)",
                                ));
                            }
                            destroy = true;
                        }
                        "zone" => {
                            // W12-S5 (grammar v1.5, D108): the
                            // episode-slot zone letter A..G. Strictly
                            // ONE uppercase letter (a typo'd zone must
                            // fail loud — the runner would otherwise
                            // stage the wrong mission's assets).
                            let mut chars = value.chars();
                            let (Some(z), None) = (chars.next(), chars.next()) else {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "zone key is one letter A..G (e.g. `zone = \"B\"`)",
                                ));
                            };
                            if !z.is_ascii_uppercase() || !('A'..='G').contains(&z) {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "zone letter out of range A..G (the 7 campaign zones)",
                                ));
                            }
                            if zone.is_some() {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "zone staged twice (one key per scenario)",
                                ));
                            }
                            zone = Some(z);
                        }
                        "pickup" => {
                            // W12-S5 (grammar v1.5, D108): the boolean
                            // pickup-surface staging key — `pickup = 1`
                            // stages the mission's own .TOT through the
                            // engine host seam AFTER any destroy staging.
                            // Strictly `1` (same fail-loud rule as
                            // destroy).
                            if value.trim() != "1" {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "pickup key is boolean staging: use `pickup = 1`",
                                ));
                            }
                            if pickup {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "pickup staged twice (one key per scenario)",
                                ));
                            }
                            pickup = true;
                        }
                        "platforms" => {
                            // W12-S7 (grammar v1.6, D113): the boolean
                            // platform-family arm key — `platforms = 1`
                            // arms the epilogue creep tick (the original
                            // runs it every frame; the per-frame gate
                            // draw on unarmed paths is the recorded
                            // E-gap). Strictly `1` (same fail-loud
                            // rule as destroy/pickup).
                            if value.trim() != "1" {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "platforms key is boolean arming: use `platforms = 1`",
                                ));
                            }
                            if platforms {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "platforms armed twice (one key per scenario)",
                                ));
                            }
                            platforms = true;
                        }
                        "critters" => {
                            // W12-S8 (grammar v1.7, D114): the boolean
                            // critter-family staging+arm key —
                            // `critters = 1` stages the mission's .NME
                            // through the FUN_00416458 spawn schedule
                            // and arms the controller (the original
                            // runs it every mission; the loader/
                            // controller draws on unarmed paths are
                            // the recorded stream gap). Strictly `1`
                            // (same fail-loud rule as the others).
                            if value.trim() != "1" {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "critters key is boolean arming: use `critters = 1`",
                                ));
                            }
                            if critters {
                                return Err(scen_err(
                                    line_no,
                                    line,
                                    "critters armed twice (one key per scenario)",
                                ));
                            }
                            critters = true;
                        }
                        other => {
                            return Err(scen_err(
                                line_no,
                                line,
                                &format!(
                                    "unknown scenario key {other:?} (directive {other_key:?})"
                                ),
                            ));
                        }
                    }
                }
            }
        }

        let id = id.ok_or_else(|| scen_err(0, "", "missing required key `scenario`"))?;
        let tiers = tiers.ok_or_else(|| scen_err(0, "", "missing required key `tiers`"))?;
        let frames = frames.ok_or_else(|| scen_err(0, "", "missing required key `frames`"))?;
        if id.is_empty() {
            return Err(scen_err(0, "", "scenario id must not be empty"));
        }
        // BOOT writes are pre-mission by definition (§5.5): after the
        // first until-anchor they would land mid-mission — reject.
        let anchor_idx = steps
            .iter()
            .position(|s| matches!(s, Step::UntilAnchor { .. }));
        if let Some(i) = anchor_idx {
            if steps[i + 1..]
                .iter()
                .any(|s| matches!(s, Step::Boot { .. }))
            {
                return Err(scen_err(
                    0,
                    "",
                    "boot steps are walk-phase only (before until-anchor)",
                ));
            }
        }
        Ok(Scenario {
            id,
            tiers,
            anchor,
            frames,
            launch,
            markers,
            loadout,
            destroy,
            zone,
            pickup,
            platforms,
            critters,
            steps,
        })
    }

    /// Default launch line for the EXD corpus (pinned 2026-08-22:
    /// game-data/BEDLAM/BEDLAM.EXE is the PE32 launcher; the game image is
    /// the LE file BEDLAM.EXD chain-loaded via DOS4GW.EXE — launcher
    /// strings name `.\bedlam.exd` + `DOS4GW.EXE`).
    pub fn launch_line(&self) -> &str {
        self.launch.as_deref().unwrap_or("DOS4GW.EXE BEDLAM.EXD")
    }

    /// Split the step list at the FIRST `until-anchor`: everything
    /// before it is WALK phase (pre-mission: boot writes + the scripted
    /// menu walk), everything after is MISSION phase (anchor-relative
    /// boundaries, the same numbering as capture frames). With no
    /// `until-anchor` step, every step is mission phase.
    pub fn phases(&self) -> (&[Step], &[Step]) {
        match self
            .steps
            .iter()
            .position(|s| matches!(s, Step::UntilAnchor { .. }))
        {
            Some(i) => (&self.steps[..i], &self.steps[i + 1..]),
            None => (&[], &self.steps[..]),
        }
    }
}

// ---------------------------------------------------------------------
// DBXCAP transcript

/// One parsed capture transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    pub frames: Vec<FrameRecord>,
}

#[derive(Debug)]
pub struct TranscriptError {
    line_no: usize,
    line: String,
    reason: String,
}

impl fmt::Display for TranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "transcript:{}: {} (line: {})",
            self.line_no, self.reason, self.line
        )
    }
}

fn cap_err(line_no: usize, line: &str, reason: &str) -> TranscriptError {
    TranscriptError {
        line_no,
        line: line.to_string(),
        reason: reason.to_string(),
    }
}

fn parse_hex(token: &str, line_no: usize, line: &str) -> Result<Vec<u8>, TranscriptError> {
    if token.is_empty() {
        return Ok(Vec::new());
    }
    if !token.len().is_multiple_of(2) || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(cap_err(
            line_no,
            line,
            "watch payload must be even-length hex",
        ));
    }
    let mut out = Vec::with_capacity(token.len() / 2);
    let b = token.as_bytes();
    for i in (0..b.len()).step_by(2) {
        let hi = (b[i] as char).to_digit(16).expect("checked") as u8;
        let lo = (b[i + 1] as char).to_digit(16).expect("checked") as u8;
        out.push(hi << 4 | lo);
    }
    Ok(out)
}

impl Transcript {
    /// Parse DBXCAP v1.
    pub fn parse(src: &str) -> Result<Transcript, TranscriptError> {
        let mut frames: Vec<FrameRecord> = Vec::new();
        let mut saw_header = false;
        let mut open = false; // a `frame` directive is active

        for (idx, raw) in src.lines().enumerate() {
            let line_no = idx + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let directive = parts.next().unwrap_or("");
            match directive {
                "DBXCAP" => {
                    if saw_header {
                        return Err(cap_err(line_no, line, "duplicate DBXCAP header"));
                    }
                    if parts.next() != Some("v1") {
                        return Err(cap_err(line_no, line, "expected `DBXCAP v1`"));
                    }
                    saw_header = true;
                }
                "frame" if saw_header => {
                    let no = parts
                        .next()
                        .ok_or_else(|| cap_err(line_no, line, "frame needs a frame_no"))?;
                    let no: u64 = no
                        .parse()
                        .map_err(|_| cap_err(line_no, line, "frame_no must be a u64"))?;
                    let injected = match parts.next() {
                        None => false,
                        Some("0") => false,
                        Some("1") => true,
                        Some(_) => {
                            return Err(cap_err(line_no, line, "injected flag must be 0 or 1"));
                        }
                    };
                    if let Some(prev) = frames.last() {
                        if prev.frame_no >= no {
                            return Err(cap_err(line_no, line, "frame_no must strictly increase"));
                        }
                    }
                    frames.push(FrameRecord::new(no, injected));
                    open = true;
                }
                "watch" if saw_header => {
                    if !open {
                        return Err(cap_err(line_no, line, "watch before any frame directive"));
                    }
                    let id = parts
                        .next()
                        .ok_or_else(|| cap_err(line_no, line, "watch needs an id"))?;
                    let bytes = parse_hex(parts.next().unwrap_or(""), line_no, line)?;
                    frames
                        .last_mut()
                        .expect("open implies a frame")
                        .push_watch(id, bytes);
                }
                _ if !saw_header => {
                    return Err(cap_err(
                        line_no,
                        line,
                        "transcript must start with `DBXCAP v1`",
                    ));
                }
                _ => {
                    return Err(cap_err(
                        line_no,
                        line,
                        "unknown directive (want frame/watch)",
                    ));
                }
            }
        }
        if !saw_header {
            return Err(cap_err(
                0,
                "",
                "empty transcript: missing `DBXCAP v1` header",
            ));
        }
        Ok(Transcript { frames })
    }
}

// ---------------------------------------------------------------------
// Stitch + manifest

/// Stitch failures (validation across scenario/registry/transcript).
#[derive(Debug)]
pub enum StitchError {
    Scenario(ScenarioError),
    Transcript(TranscriptError),
    /// Transcript watch id not in the committed registry.
    UnknownWatch(String),
    /// Watch id legal globally but its tier is not in the scenario tiers.
    TierOutOfScenario {
        id: String,
        tier: String,
        scenario: String,
    },
    /// O1 anti-ghost: registry row has no EXD address (EXW-only or gap).
    NoExdAddress {
        id: String,
        status: String,
    },
    /// Transcript frame count != scenario frames + 1 (anchor included).
    FrameCountMismatch {
        expected: u64,
        actual: u64,
    },
    Encode(dump::DumpError),
}

impl fmt::Display for StitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StitchError::Scenario(e) => write!(f, "{e}"),
            StitchError::Transcript(e) => write!(f, "{e}"),
            StitchError::UnknownWatch(id) => {
                write!(f, "transcript watch id not in the registry: {id:?}")
            }
            StitchError::TierOutOfScenario { id, tier, scenario } => write!(
                f,
                "watch {id:?} (tier {tier:?}) is not among scenario {scenario:?} tiers"
            ),
            StitchError::NoExdAddress { id, status } => write!(
                f,
                "watch {id:?} has no EXD address (exd_status {status:?}) — \
                 EXW-only/gap rows never enter an O1 dump"
            ),
            StitchError::FrameCountMismatch { expected, actual } => write!(
                f,
                "transcript has {actual} frames, scenario expects {expected} \
                 (anchor frame + `frames` post-anchor records)"
            ),
            StitchError::Encode(e) => write!(f, "dump encode failed: {e}"),
        }
    }
}

impl From<dump::DumpError> for StitchError {
    fn from(e: dump::DumpError) -> Self {
        StitchError::Encode(e)
    }
}

/// The digest manifest — the git-carried fingerprint of one dump run
/// (DESIGN §3 hygiene: the dump blob itself stays under runtime/).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub schema_ver: u16,
    pub channel: String,
    pub scenario: String,
    pub build_sha256: String,
    pub pins: Vec<String>,
    pub frame_count: u64,
    pub frame_no_first: Option<u64>,
    pub frame_no_last: Option<u64>,
    pub dump_bytes: usize,
    pub dump_sha256: String,
    pub chain_digest: String,
}

impl Manifest {
    /// Hand-rolled JSON (zero-dep charter; no String field here contains
    /// characters needing escaping — ids/pins are validated ASCII-ish and
    /// the hashes are hex).
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"schema_ver\": {},\n", self.schema_ver));
        s.push_str(&format!("  \"channel\": \"{}\",\n", self.channel));
        s.push_str(&format!("  \"scenario\": \"{}\",\n", self.scenario));
        s.push_str(&format!("  \"build_sha256\": \"{}\",\n", self.build_sha256));
        s.push_str("  \"pins\": [");
        for (i, p) in self.pins.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("\"{p}\""));
        }
        s.push_str("],\n");
        s.push_str(&format!("  \"frame_count\": {},\n", self.frame_count));
        s.push_str(&format!(
            "  \"frame_no_first\": {},\n",
            self.frame_no_first
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into())
        ));
        s.push_str(&format!(
            "  \"frame_no_last\": {},\n",
            self.frame_no_last
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into())
        ));
        s.push_str(&format!("  \"dump_bytes\": {},\n", self.dump_bytes));
        s.push_str(&format!("  \"dump_sha256\": \"{}\",\n", self.dump_sha256));
        s.push_str(&format!("  \"chain_digest\": \"{}\"\n", self.chain_digest));
        s.push_str("}\n");
        s
    }
}

/// Stitched dump + its manifest.
#[derive(Debug, Clone)]
pub struct Stitched {
    pub bytes: Vec<u8>,
    pub manifest: Manifest,
}

/// Validate the transcript against scenario + registry and encode the W3
/// dump (which also computes every digest + the chain).
pub fn stitch(
    scenario: &Scenario,
    transcript: &Transcript,
    header: &DumpHeader,
    reg: &[Watch],
) -> Result<Stitched, StitchError> {
    // Per-id checks: registry membership, scenario tier, O1 address rule.
    for frame in &transcript.frames {
        for w in &frame.watches {
            let row = reg
                .iter()
                .find(|r| r.id == w.id)
                .ok_or_else(|| StitchError::UnknownWatch(w.id.clone()))?;
            if !scenario.tiers.contains(&row.tier) {
                return Err(StitchError::TierOutOfScenario {
                    id: row.id.clone(),
                    tier: row.tier.clone(),
                    scenario: scenario.id.clone(),
                });
            }
            if header.channel == Channel::O1ExdDosboxX && row.exd_addr.is_empty() {
                return Err(StitchError::NoExdAddress {
                    id: row.id.clone(),
                    status: row.exd_status.clone(),
                });
            }
        }
    }

    // Frame-count contract: anchor frame + `frames` post-anchor records.
    let expected = scenario
        .frames
        .checked_add(1)
        .expect("scenario frames is small");
    let actual = transcript.frames.len() as u64;
    if expected != actual {
        return Err(StitchError::FrameCountMismatch { expected, actual });
    }

    let bytes = dump::encode_dump(header, &transcript.frames, reg)?;

    // The manifest fingerprints the encoded dump itself. Digests are
    // computed from the same canonicalized frames `encode_dump` wrote
    // (canonicalization is idempotent), so chain == the encoded chain.
    let mut digests = Vec::with_capacity(transcript.frames.len());
    for f in &transcript.frames {
        let mut canon = f.clone();
        dump::canonicalize_frame(&mut canon, reg)?;
        digests.push(dump::frame_digest(&canon)?);
    }
    let chain = dump::chain_digest(&digests);
    let first = transcript.frames.first().map(|f| f.frame_no);
    let last = transcript.frames.last().map(|f| f.frame_no);
    let dump_sha256 = hex_lower(&sha256(&bytes));
    Ok(Stitched {
        manifest: Manifest {
            schema_ver: header.schema_ver,
            channel: header.channel.name().to_string(),
            scenario: scenario.id.clone(),
            build_sha256: hex_lower(&header.build_sha256),
            pins: header.pins.clone(),
            frame_count: actual,
            frame_no_first: first,
            frame_no_last: last,
            dump_bytes: bytes.len(),
            dump_sha256: dump_sha256.clone(),
            chain_digest: format!("{chain}"),
        },
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCEN: &str = "# S0 test\nscenario = \"S0\"\ntiers = T0,TS\nframes = 2\nanchor = mission-start\nstep 5\ncapture\nuntil-anchor mission-start\n";

    fn reg() -> Vec<Watch> {
        crate::registry()
    }

    #[test]
    fn scenario_roundtrip_and_defaults() {
        let s = Scenario::parse(SCEN).unwrap();
        assert_eq!(s.id, "S0");
        assert_eq!(s.tiers, vec!["T0", "TS"]);
        assert_eq!(s.frames, 2);
        assert_eq!(s.anchor.as_deref(), Some("mission-start"));
        assert_eq!(
            s.steps,
            vec![
                Step::Advance { frames: 5 },
                Step::Capture,
                Step::UntilAnchor {
                    name: "mission-start".into()
                },
            ]
        );
        assert_eq!(s.launch_line(), "DOS4GW.EXE BEDLAM.EXD");
    }

    #[test]
    fn scenario_rejects_unknown_and_missing() {
        assert!(Scenario::parse("tiers = T0\nframes = 1\n").is_err()); // no id
        assert!(Scenario::parse("scenario = X\nframes = 1\n").is_err()); // no tiers
        assert!(Scenario::parse("scenario = X\ntiers = T0\n").is_err()); // no frames
        assert!(Scenario::parse("scenario = X\ntiers = T0\nframes = 1\nfoo = 1\n").is_err());
        assert!(Scenario::parse("scenario = X\ntiers = T0\nframes = 1\nfrobnicate 3\n").is_err());
    }

    #[test]
    fn markers_key_parses_and_bounds() {
        // D91: `x,y,z` triples, `;`-separated, staged after the MRK
        // robots; default empty.
        let base = "scenario = X\ntiers = T0\nframes = 1\n";
        assert!(Scenario::parse(base).unwrap().markers.is_empty());
        let s = Scenario::parse(&format!("{base}markers = 18,73,1\n")).unwrap();
        assert_eq!(s.markers, vec![(18, 73, 1)]);
        let s = Scenario::parse(&format!("{base}markers = 0x12,73,1; 5,0x3D,0\n")).unwrap();
        assert_eq!(s.markers, vec![(0x12, 73, 1), (5, 0x3D, 0)]);
        // malformed: non-integers, wrong arity, empty, over the cap
        assert!(Scenario::parse(&format!("{base}markers = 18,73,x\n")).is_err());
        assert!(Scenario::parse(&format!("{base}markers = 18,73\n")).is_err());
        assert!(Scenario::parse(&format!("{base}markers = 18,73,1,4\n")).is_err());
        assert!(Scenario::parse(&format!("{base}markers = \n")).is_err());
        let ten = "1,2,3; 4,5,6; 7,8,9; 10,11,12; 13,14,15; 16,17,18; \
                   19,20,21; 22,23,24; 25,26,27; 28,29,30";
        assert!(Scenario::parse(&format!("{base}markers = {ten}\n")).is_err());
    }

    #[test]
    fn loadout_key_parses_and_bounds() {
        // W12-S3 (grammar v1.3): `idx,mask,id:ammo[,...]` entries,
        // `;`-separated; default empty.
        let base = "scenario = X\ntiers = T0\nframes = 1\n";
        assert!(Scenario::parse(base).unwrap().loadout.is_empty());
        let s = Scenario::parse(&format!("{base}loadout = 0,0x01,9:2\n")).unwrap();
        assert_eq!(
            s.loadout,
            vec![LoadoutRobot {
                robot: 0,
                mask: 1,
                slots: vec![(9, 2)],
            }]
        );
        let s = Scenario::parse(&format!(
            "{base}loadout = 0,0x7F,9:2,0xA:2,0xB:2,0x10:2,0x14:2,0x1B:2,0x1D:2; 1,0x01,0x20:2\n"
        ))
        .unwrap();
        assert_eq!(s.loadout.len(), 2);
        assert_eq!(s.loadout[0].slots.len(), 7);
        assert_eq!(s.loadout[0].slots[4], (0x14, 2));
        assert_eq!(s.loadout[1].robot, 1);
        assert_eq!(s.loadout[1].slots, vec![(0x20, 2)]);
        // malformed: no pairs, bad id/ammo/mask, over the slot cap,
        // mask beyond the staged list, duplicate robot, empty key
        assert!(Scenario::parse(&format!("{base}loadout = 0,0x01\n")).is_err());
        assert!(Scenario::parse(&format!("{base}loadout = 0,0x01,1:2\n")).is_err());
        assert!(Scenario::parse(&format!("{base}loadout = 0,0x01,9:0\n")).is_err());
        assert!(Scenario::parse(&format!("{base}loadout = 0,0xFF,9:2\n")).is_err());
        assert!(Scenario::parse(&format!(
            "{base}loadout = 0,0xFF,9:2,0xA:2,0xB:2,0x10:2,0x14:2,0x1B:2,0x1D:2,0x20:2\n"
        ))
        .is_err());
        assert!(Scenario::parse(&format!("{base}loadout = 0,0x03,9:2\n")).is_err());
        assert!(Scenario::parse(&format!("{base}loadout = 0,0x01,9:2; 0,0x01,5:1\n")).is_err());
        assert!(Scenario::parse(&format!("{base}loadout = 12,0x01,9:2\n")).is_err());
        assert!(Scenario::parse(&format!("{base}loadout = \n")).is_err());
    }

    #[test]
    fn destroy_key_parses_and_gates() {
        // W12-S4 (grammar v1.4): the boolean destroy-family staging
        // key — default off (S0..S3 bytes unchanged), strictly `1`,
        // once per scenario.
        let base = "scenario = X\ntiers = T0\nframes = 1\n";
        assert!(!Scenario::parse(base).unwrap().destroy);
        assert!(
            Scenario::parse(&format!("{base}destroy = 1\n"))
                .unwrap()
                .destroy
        );
        // A typo'd value must fail loud (silently skipping the
        // staging would desync the dump rows from the scenario).
        assert!(Scenario::parse(&format!("{base}destroy = 0\n")).is_err());
        assert!(Scenario::parse(&format!("{base}destroy = true\n")).is_err());
        assert!(Scenario::parse(&format!("{base}destroy = 1\ndestroy = 1\n")).is_err());
    }

    #[test]
    fn zone_and_pickup_keys_parse_and_gate() {
        // W12-S5 (grammar v1.5, D108): the episode-slot zone letter
        // + the boolean pickup-surface staging key — defaults
        // None/false (S0..S4 bytes unchanged).
        let base = "scenario = X\ntiers = T0\nframes = 1\n";
        let s = Scenario::parse(base).unwrap();
        assert_eq!(s.zone, None);
        assert!(!s.pickup);
        let s = Scenario::parse(&format!("{base}zone = \"B\"\npickup = 1\n")).unwrap();
        assert_eq!(s.zone, Some('B'));
        assert!(s.pickup);
        // The whole campaign range, quoted or bare.
        for z in ['A', 'D', 'G'] {
            let s = Scenario::parse(&format!("{base}zone = \"{z}\"\n")).unwrap();
            assert_eq!(s.zone, Some(z));
            let s = Scenario::parse(&format!("{base}zone = {z}\n")).unwrap();
            assert_eq!(s.zone, Some(z));
        }
        // Fail loud: out of range, multi-char, lowercase, duplicate,
        // and the pickup strictness mirrors destroy.
        assert!(Scenario::parse(&format!("{base}zone = \"H\"\n")).is_err());
        assert!(Scenario::parse(&format!("{base}zone = \"AB\"\n")).is_err());
        assert!(Scenario::parse(&format!("{base}zone = \"b\"\n")).is_err());
        assert!(Scenario::parse(&format!("{base}zone = \"1\"\n")).is_err());
        assert!(Scenario::parse(&format!("{base}zone = \"A\"\nzone = \"B\"\n")).is_err());
        assert!(Scenario::parse(&format!("{base}pickup = 0\n")).is_err());
        assert!(Scenario::parse(&format!("{base}pickup = 1\npickup = 1\n")).is_err());
    }

    #[test]
    fn injection_steps_parse() {
        let s = Scenario::parse(
            "scenario = X\ntiers = T0\nframes = 1\n\
             boot difficulty=1\n\
             until-anchor mission-start\n\
             step 2\n\
             keystore 0x1f=1, 0x2a=0\n\
             order 29 18 0\n\
             pad 3\n\
             command 01 02 3F 00\n",
        )
        .unwrap();
        let (walk, mission) = s.phases();
        assert_eq!(
            walk,
            &[Step::Boot {
                key: "difficulty".into(),
                value: 1
            }]
        );
        assert_eq!(
            mission,
            &[
                Step::Advance { frames: 2 },
                Step::Keystore {
                    entries: vec![(0x1f, 1), (0x2a, 0)]
                },
                Step::Order { x: 29, y: 18, z: 0 },
                Step::Pad { slot: 3 },
                Step::Command {
                    bytes: vec![0x01, 0x02, 0x3f, 0x00]
                },
            ][..]
        );
        // no until-anchor: everything is mission phase
        let s2 = Scenario::parse("scenario = X\ntiers = T0\nframes = 1\norder 1 2 3\n").unwrap();
        assert_eq!(s2.phases().0.len(), 0);
        assert_eq!(s2.phases().1.len(), 1);
    }

    #[test]
    fn injection_step_validation() {
        let base = "scenario = X\ntiers = T0\nframes = 1\n";
        // keystore
        assert!(Scenario::parse(&format!("{base}keystore 0x100=1\n")).is_err()); // scan > 0xFF
        assert!(Scenario::parse(&format!("{base}keystore 0x1f=2\n")).is_err()); // val not 0|1
        assert!(Scenario::parse(&format!("{base}keystore 1f\n")).is_err()); // not scan=val
        assert!(Scenario::parse(&format!("{base}keystore\n")).is_err()); // empty
                                                                         // order
        assert!(Scenario::parse(&format!("{base}order 1 2\n")).is_err()); // too few
        assert!(Scenario::parse(&format!("{base}order 1 2 3 4\n")).is_err()); // too many
        assert!(Scenario::parse(&format!("{base}order 1 x 3\n")).is_err()); // non-int
                                                                            // pad
        assert!(Scenario::parse(&format!("{base}pad 999\n")).is_err()); // out of range
        assert!(Scenario::parse(&format!("{base}pad 1 2\n")).is_err()); // too many
                                                                        // command
        assert!(Scenario::parse(&format!("{base}command 1\n")).is_err()); // not a byte pair
        assert!(Scenario::parse(&format!("{base}command\n")).is_err()); // empty
        let long = "01 ".repeat(0x81);
        assert!(Scenario::parse(&format!("{base}command {long}\n")).is_err()); // > 0x80
                                                                               // boot
        assert!(Scenario::parse(&format!("{base}boot volume=5\n")).is_err()); // unknown key
        assert!(Scenario::parse(&format!("{base}boot difficulty\n")).is_err()); // no =value
        assert!(Scenario::parse(&format!("{base}until-anchor m\nboot difficulty=1\n")).is_err());
        // boot in mission phase
    }

    #[test]
    fn transcript_parse_and_errors() {
        let ok = "DBXCAP v1\nframe 7\nwatch frame-counter 07000000\nframe 8 1\nwatch rng-state-a\n";
        let t = Transcript::parse(ok).unwrap();
        assert_eq!(t.frames.len(), 2);
        assert!(!t.frames[0].injection_applied);
        assert!(t.frames[1].injection_applied);
        assert_eq!(
            t.frames[0].watch("frame-counter"),
            Some(&[7u8, 0, 0, 0][..])
        );
        assert_eq!(t.frames[1].watch("rng-state-a"), Some(&[][..]));

        assert!(Transcript::parse("").is_err()); // no header
        assert!(Transcript::parse("frame 1\n").is_err()); // header missing first
        assert!(Transcript::parse("DBXCAP v1\nwatch x 00\n").is_err()); // watch w/o frame
        assert!(Transcript::parse("DBXCAP v1\nframe 1\nwatch x zz\n").is_err()); // bad hex
        assert!(Transcript::parse("DBXCAP v1\nframe 1\nwatch x 0\n").is_err()); // odd hex
        assert!(Transcript::parse("DBXCAP v1\nframe 1\nframe 1\n").is_err()); // not increasing
        assert!(Transcript::parse("DBXCAP v2\n").is_err()); // wrong version
    }

    #[test]
    fn stitch_ok_and_deterministic() {
        let s = Scenario::parse(SCEN).unwrap();
        let cap = "DBXCAP v1\n\
                   # anchor frame (TS statics ride here)\n\
                   frame 100\n\
                   watch frame-counter 64000000\n\
                   watch static-map-wh 400300\n\
                   frame 101\n\
                   watch frame-counter 65000000\n\
                   frame 102 1\n\
                   watch frame-counter 66000000\n\
                   watch rng-state-a 4ee60200\n";
        let t = Transcript::parse(cap).unwrap();
        let mut hdr = DumpHeader::new(Channel::O1ExdDosboxX, [0xab; 32], "S0");
        hdr.push_pin("core=normal");
        let r = reg();
        let a = stitch(&s, &t, &hdr, &r).unwrap();
        let b = stitch(&s, &t, &hdr, &r).unwrap();
        assert_eq!(a.bytes, b.bytes, "stitching must be byte-deterministic");
        assert_eq!(a.manifest.chain_digest, b.manifest.chain_digest);
        // decode round-trips (W3 verifies every digest + the chain)
        let dec = dump::decode_dump(&a.bytes).unwrap();
        assert_eq!(dec.frames.len(), 3);
        assert_eq!(dec.header.scenario, "S0");
    }

    #[test]
    fn stitch_rejects_tier_exd_and_count_violations() {
        let s = Scenario::parse(SCEN).unwrap();
        let r = reg();
        let hdr = DumpHeader::new(Channel::O1ExdDosboxX, [0; 32], "S0");

        // T2 row in a T0/TS scenario
        let t = Transcript::parse(
            "DBXCAP v1\nframe 1\nwatch projectile-bank 00\nframe 2\nwatch frame-counter 00\n",
        )
        .unwrap();
        match stitch(&s, &t, &hdr, &r) {
            Err(StitchError::TierOutOfScenario { id, .. }) => assert_eq!(id, "projectile-bank"),
            other => panic!("expected TierOutOfScenario, got {other:?}"),
        }

        // T0 row with an explicit EXD gap on the O1 channel. Since the
        // D134 twin census the live registry has NO gap rows left, so
        // the fixture fabricates one (sfx-master-gate with its twin
        // blanked) — the stitcher must still refuse gap rows.
        let mut gapped = r.clone();
        for w in gapped.iter_mut() {
            if w.id == "sfx-master-gate" {
                w.exd_addr = String::new();
            }
        }
        let t = Transcript::parse(
            "DBXCAP v1\nframe 1\nwatch sfx-master-gate 00\nframe 2\nwatch frame-counter 00\n",
        )
        .unwrap();
        match stitch(&s, &t, &hdr, &gapped) {
            Err(StitchError::NoExdAddress { id, .. }) => assert_eq!(id, "sfx-master-gate"),
            other => panic!("expected NoExdAddress, got {other:?}"),
        }

        // frame-count contract: scenario wants frames+1 records
        let t = Transcript::parse("DBXCAP v1\nframe 1\nwatch frame-counter 00\n").unwrap();
        assert!(matches!(
            stitch(&s, &t, &hdr, &r),
            Err(StitchError::FrameCountMismatch {
                expected: 3,
                actual: 1
            })
        ));

        // unknown id
        let t = Transcript::parse(
            "DBXCAP v1\nframe 1\nwatch no-such-row 00\nframe 2\nwatch frame-counter 00\n",
        )
        .unwrap();
        assert!(matches!(
            stitch(&s, &t, &hdr, &r),
            Err(StitchError::UnknownWatch(_))
        ));
    }

    #[test]
    fn manifest_json_shape() {
        let s = Scenario::parse(SCEN).unwrap();
        let t = Transcript::parse(
            "DBXCAP v1\nframe 5\nwatch frame-counter 00\nframe 6\nwatch frame-counter 00\nframe 7\nwatch frame-counter 00\n",
        )
        .unwrap();
        let hdr = DumpHeader::new(Channel::O1ExdDosboxX, [1; 32], "S0");
        let st = stitch(&s, &t, &hdr, &reg()).unwrap();
        let j = st.manifest.to_json();
        assert!(j.contains("\"scenario\": \"S0\""));
        assert!(j.contains("\"frame_no_first\": 5"));
        assert!(j.contains("\"frame_no_last\": 7"));
        assert!(j.starts_with('{') && j.trim_end().ends_with('}'));
    }
}
