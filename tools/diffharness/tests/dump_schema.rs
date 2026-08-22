//! W3 dump-schema tests: roundtrip, integrity (tamper detection),
//! canonicalization against the committed registry, chain construction,
//! and the hash-util cross-check against the engine's expected outputs.

use diffharness::dump::{
    canonicalize_frame, chain_digest, decode_dump, encode_dump, frame_digest, Channel, DumpError,
    DumpHeader, FrameRecord, SCHEMA_VER,
};
use diffharness::hash::{fnv1a64, DumpDigest, Fnv1a64};
use diffharness::registry;

// ---------------------------------------------------------------------
// Hash util: the engine cross-check (W3 ticket fallback clause).
//
// bedlam-core depends on thiserror, so the zero-dep guard crate mirrors
// its FNV-1a 64 util. These vectors are the engine's PUBLIC expected
// outputs (engine/bedlam-core/src/hash.rs tests + D28 baseline
// construction): if either side drifts, this fails.

/// Two real sha256-sized filler values for header tests.
fn fake_sha(seed: u8) -> [u8; 32] {
    [seed; 32]
}

#[test]
fn engine_hash_vectors() {
    // engine hash.rs `public_vectors`
    assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
    assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x85944171f73967e8);
    // engine hash.rs `write_u32_equals_four_le_bytes` (canonical LE rule)
    let mut h = Fnv1a64::new();
    h.write_u32(0xDEAD_BEEF);
    assert_eq!(h.finish(), fnv1a64(&[0xEF, 0xBE, 0xAD, 0xDE]));
    let mut h = Fnv1a64::new();
    h.write_u64(0x0102_0304_0506_0708);
    assert_eq!(
        h.finish(),
        fnv1a64(&[0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01])
    );
}

#[test]
fn chain_construction_is_the_d28_parity_harness_one() {
    // parity_harness.rs: `let mut chain = Fnv1a64::new(); ...
    // chain.write_u64(h);` per tick, `chain.finish()` at the end.
    let digests = [DumpDigest(0x1111), DumpDigest(0x2222), DumpDigest(0x3333)];
    let mut manual = Fnv1a64::new();
    for d in digests {
        manual.write_u64(d.0);
    }
    assert_eq!(chain_digest(&digests).0, manual.finish());
    // Empty dump -> bare offset basis (same as parity_harness's zero-tick
    // chain: an empty sequence hashes to the basis).
    assert_eq!(chain_digest(&[]).0, 0xcbf29ce484222325);
}

// ---------------------------------------------------------------------
// Registry-driven encoding.

fn sample_header() -> DumpHeader {
    let mut h = DumpHeader::new(Channel::O1ExdDosboxX, fake_sha(0xAB), "S1-pod-stagger");
    h.push_pin("dosbox-x=2024.03.08");
    h.push_pin("core=normal");
    h.push_pin("cycles=60000");
    h
}

fn sample_frames() -> Vec<FrameRecord> {
    let mut f0 = FrameRecord::new(1000, false);
    f0.push_watch("frame-counter", 1000u32.to_le_bytes());
    f0.push_watch("rng-state-a", 123456u32.to_le_bytes());
    f0.push_watch("rng-state-b", 234567u32.to_le_bytes());
    f0.push_watch("score", 0u32.to_le_bytes());
    f0.push_watch("robot-bank", vec![0u8; 6 * 0xA8]); // count*0xA8, 6 pods
    let mut f1 = FrameRecord::new(1001, true);
    f1.push_watch("frame-counter", 1001u32.to_le_bytes());
    f1.push_watch("rng-state-a", 123457u32.to_le_bytes());
    f1.push_watch("rng-state-b", 234568u32.to_le_bytes());
    f1.push_watch("score", 10u32.to_le_bytes());
    f1.push_watch("robot-bank", vec![0u8; 6 * 0xA8]);
    vec![f0, f1]
}

#[test]
fn roundtrip_preserves_everything_and_verifies() {
    let reg = registry();
    let bytes = encode_dump(&sample_header(), &sample_frames(), &reg).unwrap();
    let dump = decode_dump(&bytes).unwrap();

    assert_eq!(dump.header, sample_header());
    assert_eq!(dump.header.schema_ver, SCHEMA_VER);
    assert_eq!(dump.frames.len(), 2);
    assert_eq!(dump.frames[0].frame_no, 1000);
    assert!(!dump.frames[0].injection_applied);
    assert!(dump.frames[1].injection_applied);
    assert_eq!(
        dump.frames[0].watch("frame-counter"),
        Some(&1000u32.to_le_bytes()[..])
    );
    assert_eq!(
        dump.frames[1].watch("score"),
        Some(&10u32.to_le_bytes()[..])
    );
    assert_eq!(dump.trailer.frame_count, 2);
    // Frame digests on the wire match a fresh recomputation.
    for (f, d) in dump.frames.iter().zip(dump.frame_digests.iter()) {
        assert_eq!(frame_digest(f).unwrap(), *d);
    }
    // Trailer chain matches the chain over the decoded digests.
    assert_eq!(dump.trailer.chain, chain_digest(&dump.frame_digests));
}

#[test]
fn encoding_is_byte_deterministic() {
    let reg = registry();
    let a = encode_dump(&sample_header(), &sample_frames(), &reg).unwrap();
    let b = encode_dump(&sample_header(), &sample_frames(), &reg).unwrap();
    assert_eq!(a, b);
    // A different scenario id changes the header but not frame digests.
    let mut h2 = sample_header();
    h2.scenario = "S0-headless".to_string();
    let c = encode_dump(&h2, &sample_frames(), &reg).unwrap();
    assert_ne!(a, c);
    let dc = decode_dump(&c).unwrap();
    assert_eq!(dc.frame_digests, decode_dump(&a).unwrap().frame_digests);
}

#[test]
fn canonical_order_is_registry_file_order() {
    let reg = registry();
    // Feed frame watches in REVERSE registry order; the encoder must
    // emit them in registry file order.
    let mut f = FrameRecord::new(5, false);
    f.push_watch("robot-bank", vec![1u8, 2]);
    f.push_watch("frame-counter", 5u32.to_le_bytes());
    let mut manual = FrameRecord::new(5, false);
    manual.push_watch("frame-counter", 5u32.to_le_bytes());
    manual.push_watch("robot-bank", vec![1u8, 2]);
    canonicalize_frame(&mut f, &reg).unwrap();
    assert_eq!(f, manual);

    let idx = |id: &str| reg.iter().position(|w| w.id == id).unwrap();
    assert!(idx("frame-counter") < idx("robot-bank"));
    // And the wire order matches: decode preserves wire order.
    let bytes = encode_dump(&sample_header(), &[f.clone()], &reg).unwrap();
    let dump = decode_dump(&bytes).unwrap();
    assert_eq!(dump.frames[0].watches, manual.watches);
}

#[test]
fn unknown_and_duplicate_watch_ids_rejected() {
    let reg = registry();
    let mut f = FrameRecord::new(1, false);
    f.push_watch("not-a-registry-id", vec![0u8; 4]);
    match encode_dump(&sample_header(), &[f], &reg) {
        Err(DumpError::UnknownWatchId(id)) => assert_eq!(id, "not-a-registry-id"),
        other => panic!("expected UnknownWatchId, got {other:?}"),
    }

    let mut d = FrameRecord::new(1, false);
    d.push_watch("score", vec![0u8; 4]);
    d.push_watch("score", vec![0u8; 4]);
    match encode_dump(&sample_header(), &[d], &reg) {
        Err(DumpError::DuplicateWatchId { id, .. }) => assert_eq!(id, "score"),
        other => panic!("expected DuplicateWatchId, got {other:?}"),
    }
}

#[test]
fn empty_watch_blob_is_legal() {
    // Count-driven extents legitimately hit 0 (e.g. projectile bank
    // before anything fires): extent "count*0x36" with count 0.
    let reg = registry();
    let mut f = FrameRecord::new(3, false);
    f.push_watch("frame-counter", 3u32.to_le_bytes());
    f.push_watch("projectile-bank", Vec::new());
    let bytes = encode_dump(&sample_header(), &[f], &reg).unwrap();
    let dump = decode_dump(&bytes).unwrap();
    assert_eq!(dump.frames[0].watch("projectile-bank"), Some(&[][..]));
}

#[test]
fn frame_no_must_strictly_increase_on_encode_and_decode() {
    let reg = registry();
    let mut a = FrameRecord::new(10, false);
    a.push_watch("score", vec![0u8; 4]);
    let mut b = FrameRecord::new(10, false); // same -> rejected
    b.push_watch("score", vec![0u8; 4]);
    match encode_dump(&sample_header(), &[a.clone(), b], &reg) {
        Err(DumpError::FrameNoNotIncreasing { prev: 10, got: 10 }) => {}
        other => panic!("expected FrameNoNotIncreasing, got {other:?}"),
    }
    let mut c = FrameRecord::new(9, false); // backwards -> rejected
    c.push_watch("score", vec![0u8; 4]);
    match encode_dump(&sample_header(), &[a, c], &reg) {
        Err(DumpError::FrameNoNotIncreasing { prev: 10, got: 9 }) => {}
        other => panic!("expected FrameNoNotIncreasing, got {other:?}"),
    }
}

#[test]
fn zero_frame_dump_is_legal() {
    // The dump chain of an empty stream = the FNV offset basis.
    let reg = registry();
    let bytes = encode_dump(&sample_header(), &[], &reg).unwrap();
    let dump = decode_dump(&bytes).unwrap();
    assert!(dump.frames.is_empty());
    assert_eq!(dump.trailer.frame_count, 0);
    assert_eq!(dump.trailer.chain, chain_digest(&[]));
}

// ---------------------------------------------------------------------
// Integrity: every tamper must be caught.

fn encoded_sample() -> Vec<u8> {
    let reg = registry();
    encode_dump(&sample_header(), &sample_frames(), &reg).unwrap()
}

#[test]
fn tampered_payload_fails_frame_digest() {
    let mut bytes = encoded_sample();
    // Find a payload byte inside the first frame (after the header, the
    // tag/frame fields, id+len prefixes): locate the score payload by
    // searching the frame region for the id then skipping to its bytes.
    let header_len = {
        // header: 4+2+1+32 + (1+14) + 2 + 3*(1+len)
        4 + 2
            + 1
            + 32
            + (1 + "S1-pod-stagger".len())
            + 2
            + (1 + "dosbox-x=2024.03.08".len())
            + (1 + "core=normal".len())
            + (1 + "cycles=60000".len())
    };
    assert_eq!(&bytes[header_len..header_len + 4], b"BDLD");
    // Locate the frame-counter id string in the frame region; its
    // payload starts right after the id + the u32 len prefix.
    let frame_region = &bytes[header_len..];
    let id_pos = frame_region
        .windows(b"frame-counter".len())
        .position(|w| w == b"frame-counter")
        .expect("frame-counter id must be on the wire");
    let payload_at = header_len + id_pos + "frame-counter".len() + 4;
    assert_eq!(bytes[payload_at], 0xE8); // 1000 le
    bytes[payload_at] ^= 0xFF;
    match decode_dump(&bytes) {
        Err(DumpError::DigestMismatch {
            index: 0,
            frame_no: 1000,
            ..
        }) => {}
        other => panic!("expected DigestMismatch, got {other:?}"),
    }
}

#[test]
fn tampered_chain_and_count_fail() {
    let mut bytes = encoded_sample();
    let n = bytes.len();
    bytes[n - 1] ^= 0x01; // chain digest low byte
    match decode_dump(&bytes) {
        Err(DumpError::ChainMismatch { .. }) => {}
        other => panic!("expected ChainMismatch, got {other:?}"),
    }

    let mut bytes = encoded_sample();
    let n = bytes.len();
    // frame_count u64 sits right before the chain u64: flip a high byte.
    bytes[n - 9] ^= 0x10;
    match decode_dump(&bytes) {
        Err(DumpError::FrameCountMismatch { .. }) => {}
        other => panic!("expected FrameCountMismatch, got {other:?}"),
    }
}

#[test]
fn truncation_trailing_bytes_and_bad_magic_fail() {
    let bytes = encoded_sample();
    for cut in [0usize, 3, 10, 40, bytes.len() - 1] {
        assert!(
            matches!(decode_dump(&bytes[..cut]), Err(DumpError::TooShort)),
            "cut at {cut} must be TooShort"
        );
    }

    let mut extra = bytes.clone();
    extra.push(0x00);
    match decode_dump(&extra) {
        Err(DumpError::TrailingBytes(1)) => {}
        other => panic!("expected TrailingBytes, got {other:?}"),
    }

    let mut bad_magic = bytes.clone();
    bad_magic[0] = b'X';
    assert!(matches!(
        decode_dump(&bad_magic),
        Err(DumpError::BadMagic { .. })
    ));
}

#[test]
fn bad_schema_ver_and_channel_fail() {
    let reg = registry();
    let mut h = sample_header();
    h.schema_ver = 2;
    let bytes = encode_dump(&h, &sample_frames(), &reg).unwrap();
    match decode_dump(&bytes) {
        Err(DumpError::BadSchemaVer(2)) => {}
        other => panic!("expected BadSchemaVer, got {other:?}"),
    }

    // Hand-craft a bad channel byte (encode can't produce it).
    let mut bytes = encode_dump(&sample_header(), &[], &reg).unwrap();
    bytes[6] = 9; // header: BDLD(4) + schema u16(2) + channel u8
    match decode_dump(&bytes) {
        Err(DumpError::BadChannel(9)) => {}
        other => panic!("expected BadChannel, got {other:?}"),
    }
}

#[test]
fn all_four_channels_roundtrip() {
    let reg = registry();
    for ch in [
        Channel::O1ExdDosboxX,
        Channel::O2ExwWine,
        Channel::O3Street,
        Channel::Engine,
    ] {
        let mut h = DumpHeader::new(ch, fake_sha(1), "S0-headless");
        h.push_pin("pin=1");
        let bytes = encode_dump(&h, &sample_frames(), &reg).unwrap();
        let dump = decode_dump(&bytes).unwrap();
        assert_eq!(dump.header.channel, ch);
        // Channel changes the header only: frame digests are identical
        // across channels for the same observed state (the whole point).
        assert_eq!(
            dump.frame_digests,
            decode_dump(&encoded_sample()).unwrap().frame_digests
        );
    }
}

#[test]
fn frame_digest_is_bdld_domain_separated() {
    // The digest input begins with the BDLD tag, so it can never equal
    // the untagged FNV-1a of the same field bytes (engine StateHash
    // construction): verify the tag actually participates.
    let f = &sample_frames()[0];
    let mut untagged = Vec::new();
    let mut full = Vec::new();
    diffharness::dump::canonical_frame_bytes(f, &mut full).unwrap();
    untagged.extend_from_slice(&full[4..]); // strip the tag
    assert_ne!(frame_digest(f).unwrap().0, fnv1a64(&untagged));
}
