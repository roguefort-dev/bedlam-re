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
//! **Scenario grammar v1** (`scenarios/*.scen`, committed):
//! ```text
//! # comment / blank lines
//! scenario = S0              ; id (dump header; <=255 bytes)
//! tiers = T0,TS              ; watch tiers this scenario captures
//! anchor = mission-start     ; symbolic anchor event (optional)
//! frames = 2                 ; per-frame records after the anchor frame
//! launch = DOS4GW.EXE BEDLAM.EXD   ; autoexec launch line (optional)
//! step 10                    ; advance N frames, no input      (runner)
//! capture                    ; force a frame dump              (runner)
//! until-anchor mission-start ; run to the anchor event         (runner)
//! ```
//! Step directives are validated but do not drive the stitcher (the
//! transcript is the ground truth for what was captured). W5 extends the
//! vocabulary with injection steps (keystore/order/command/pad per
//! DESIGN §5); they are NOT accepted yet — unknown directive = error.
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
    /// Validated step directives in file order (runner metadata).
    pub steps: Vec<Step>,
}

/// Scenario step directives (grammar v1 — runner directives only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Advance { frames: u64 },
    Capture,
    UntilAnchor { name: String },
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

impl Scenario {
    /// Parse scenario grammar v1.
    pub fn parse(src: &str) -> Result<Scenario, ScenarioError> {
        let mut id: Option<String> = None;
        let mut tiers: Option<Vec<String>> = None;
        let mut anchor: Option<String> = None;
        let mut frames: Option<u64> = None;
        let mut launch: Option<String> = None;
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
        Ok(Scenario {
            id,
            tiers,
            anchor,
            frames,
            launch,
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
        assert!(
            Scenario::parse("scenario = X\ntiers = T0\nframes = 1\nkeystore 0x1f=1\n").is_err()
        );
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

        // T0 row with an explicit EXD gap (difficulty) on the O1 channel
        let t = Transcript::parse(
            "DBXCAP v1\nframe 1\nwatch difficulty 00\nframe 2\nwatch frame-counter 00\n",
        )
        .unwrap();
        match stitch(&s, &t, &hdr, &r) {
            Err(StitchError::NoExdAddress { id, .. }) => assert_eq!(id, "difficulty"),
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
