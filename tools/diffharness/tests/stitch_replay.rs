//! W4 stitch pipeline integration tests: the committed S0 scenario +
//! the synthetic replay fixture produce a decodable dump whose digests
//! are pinned as a committed vector (the fingerprint form git carries —
//! DESIGN §3 hygiene: the fixture bytes are synthetic, so the vector is
//! pipeline determinism, never a claim about game memory).

use diffharness::dump::{self, Channel, DumpHeader};
use diffharness::registry;
use diffharness::runner::{self, Scenario, Transcript};

const SCEN: &str = include_str!("../scenarios/S0.scen");
const CAP: &str = include_str!("fixtures/s0-replay.dbxcap");

/// All synthetic bytes XOR-folded per frame are arbitrary; what is
/// pinned is that identical input stitches byte-identically and decodes
/// with matching digests. The vector below was produced by this test's
/// first green run and is now the regression fingerprint.
const EXPECTED_CHAIN: &str = "1685e11311ae5b21";

fn stitched() -> (Vec<u8>, runner::Manifest) {
    let s = Scenario::parse(SCEN).expect("S0.scen parses");
    let t = Transcript::parse(CAP).expect("fixture parses");
    assert_eq!(
        s.frames + 1,
        t.frames.len() as u64,
        "fixture matches the S0 frame contract"
    );
    let mut hdr = DumpHeader::new(Channel::O1ExdDosboxX, [0x42; 32], s.id.clone());
    hdr.push_pin("dosbox-x=unpinned-channel-fixture");
    hdr.push_pin("core=normal");
    hdr.push_pin("cputype=pentium");
    hdr.push_pin("cycles=fixed 60000");
    let st = runner::stitch(&s, &t, &hdr, &registry()).expect("stitch ok");
    (st.bytes, st.manifest)
}

#[test]
fn committed_scenario_files_parse() {
    for (name, src) in [
        ("S0", include_str!("../scenarios/S0.scen")),
        ("S1", include_str!("../scenarios/S1.scen")),
    ] {
        let s = Scenario::parse(src).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(s.id, name);
        assert!(!s.tiers.is_empty());
    }
}

#[test]
fn replay_fixture_decodes_and_is_deterministic() {
    let (bytes_a, manifest_a) = stitched();
    let (bytes_b, manifest_b) = stitched();
    assert_eq!(bytes_a, bytes_b, "stitch must be byte-deterministic");
    assert_eq!(manifest_a, manifest_b);

    let dump = dump::decode_dump(&bytes_a).expect("decode + all digests verify");
    assert_eq!(dump.header.scenario, "S0");
    assert_eq!(dump.header.channel, Channel::O1ExdDosboxX);
    assert_eq!(dump.frames.len(), 3);
    assert_eq!(dump.frames[0].frame_no, 7);
    assert!(!dump.frames[0].injection_applied);
    assert!(dump.frames[2].injection_applied);

    // TS statics ride the anchor frame; T0 rows are present every frame.
    assert!(dump.frames[0].watch("static-map-wh").is_some());
    assert!(dump.frames[0].watch("static-type-table").is_some());
    for f in &dump.frames {
        assert!(f.watch("frame-counter").is_some());
        assert!(f.watch("rng-state-a").is_some());
    }

    // Registry-canonical order: frame-counter precedes rng-state-a in
    // watches.toml, regardless of transcript line order.
    let ids: Vec<&str> = dump.frames[0]
        .watches
        .iter()
        .map(|w| w.id.as_str())
        .collect();
    let fc = ids.iter().position(|i| *i == "frame-counter").unwrap();
    let ra = ids.iter().position(|i| *i == "rng-state-a").unwrap();
    assert!(fc < ra);

    // The manifest chain matches the decoded trailer chain.
    assert_eq!(manifest_a.chain_digest, format!("{}", dump.trailer.chain));
    assert_eq!(manifest_a.frame_count, 3);
    assert_eq!(manifest_a.frame_no_first, Some(7));
    assert_eq!(manifest_a.frame_no_last, Some(9));
}

#[test]
fn chain_fingerprint_vector() {
    let (_, manifest) = stitched();
    assert_eq!(
        manifest.chain_digest, EXPECTED_CHAIN,
        "dump chain fingerprint changed — intentional? update the vector + fingerprints"
    );
}
