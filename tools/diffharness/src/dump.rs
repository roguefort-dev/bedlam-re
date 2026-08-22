//! W3 dump schema — the versioned frame-record stream shared by O1/O2/O3/E
//! (DESIGN-DIFFHARNESS.md §3).
//!
//! One dump file = header + N frame records + trailer, all
//! little-endian, all lengths explicit. Grammar (schema_ver 1):
//!
//! ```text
//! Stream  := Header Frame* Trailer
//! Header  := "BDLD" u16:schema_ver u8:channel [u8;32]:build_sha256
//!            u8:sid_len sid:utf8
//!            u16:pin_count (u8:pin_len pin:utf8)*
//! Frame   := "BDLD" u64:frame_no u8:injection_applied
//!            u16:watch_count
//!            (u8:id_len id:utf8 u32:len raw[len])*
//!            u64:frame_digest
//! Trailer := "BDLT" u64:frame_count u64:chain_digest
//! ```
//!
//! Integrity: `frame_digest` = FNV-1a 64 over the frame's canonical bytes
//! (everything from the leading `"BDLD"` tag up to but excluding the
//! digest word itself). The `"BDLD"` prefix doubles as domain separation
//! — a dump digest can never collide with the engine's untagged
//! `StateHash` of the same field bytes. `chain_digest` = the D28-style
//! chain: incremental `Fnv1a64` fed `write_u64(frame_digest)` per frame
//! in order — the exact construction `parity_harness` uses for its
//! per-tick scene-hash chain, so a dump chain and a scene-hash chain are
//! comparable fingerprints.
//!
//! Canonical watch order = the committed registry's file order
//! (`watches.toml`); `encode_dump` enforces it via `canonicalize_frame`,
//! so identical observed state encodes to identical bytes on every
//! channel. Decode verifies digests + chain but stays registry-agnostic
//! (syntax + integrity only — semantic membership is the differ's job).
//!
//! Conventions pinned here (no extra record types):
//! - TS static-after-load rows ride as one frame record at the
//!   mission-start frame (§4: dumped once, hash-compared).
//! - TI injection-surface rows are ordinary per-frame records holding
//!   the POST-injection values (§5: watched AND written).
//! - T4 event-capture payloads use the same `WatchRecord` envelope; the
//!   per-row payload grammar is a W5+ pin, not schema surface.
//! - An empty watch blob is legal (count-driven extents legitimately
//!   hit 0, e.g. an empty robot bank before the pods open).
//! - `frame_no` strictly increases across the stream (the frame counter
//!   never rewinds, pauses included — SIM §1); gaps are runner/differ
//!   business, decode only rejects reordering.

use crate::hash::{fnv1a64, DumpDigest, Fnv1a64};
use crate::Watch;
use std::collections::BTreeSet;
use std::fmt;

/// Wire tag: header magic, frame-record prefix, and digest domain tag.
pub const MAGIC: [u8; 4] = *b"BDLD";
/// Wire tag: trailer prefix.
pub const MAGIC_TRAILER: [u8; 4] = *b"BDLT";

/// Dump schema version (bump = wire-format change; the differ refuses
/// cross-version comparison).
pub const SCHEMA_VER: u16 = 1;

/// Max byte length of one string field (ids, scenario, one pin) — the
/// on-wire length prefixes are u8.
pub const MAX_STR: usize = 255;

/// The dump channel — which side produced this stream (DESIGN §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// O1: BEDLAM.EXD under the pinned DOSBox-X (primary instrument).
    O1ExdDosboxX,
    /// O2: BEDLAM.EXW under pinned Wine (canon tiebreak).
    O2ExwWine,
    /// O3: instrumented 8street build (late second comparator).
    O3Street,
    /// E: the Rust engine (parity_harness `--canonical`, W6).
    Engine,
}

impl Channel {
    pub fn code(self) -> u8 {
        match self {
            Channel::O1ExdDosboxX => 1,
            Channel::O2ExwWine => 2,
            Channel::O3Street => 3,
            Channel::Engine => 4,
        }
    }

    pub fn from_code(code: u8) -> Option<Channel> {
        match code {
            1 => Some(Channel::O1ExdDosboxX),
            2 => Some(Channel::O2ExwWine),
            3 => Some(Channel::O3Street),
            4 => Some(Channel::Engine),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Channel::O1ExdDosboxX => "O1:EXD/DOSBox-X",
            Channel::O2ExwWine => "O2:EXW/Wine",
            Channel::O3Street => "O3:8street",
            Channel::Engine => "E:engine",
        }
    }
}

/// Dump header: schema + provenance. `build_sha256` is the watched
/// binary's sha256 (EXD, EXW, or the 8street build hash); `pins` are
/// free-form `key=value` run pins (e.g. `"dosbox-x=<ver>"`,
/// `"core=normal"`, `"cycles=60000"`); `scenario` is the §7 scenario id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpHeader {
    pub schema_ver: u16,
    pub channel: Channel,
    pub build_sha256: [u8; 32],
    pub scenario: String,
    pub pins: Vec<String>,
}

impl DumpHeader {
    pub fn new(channel: Channel, build_sha256: [u8; 32], scenario: impl Into<String>) -> Self {
        DumpHeader {
            schema_ver: SCHEMA_VER,
            channel,
            build_sha256,
            scenario: scenario.into(),
            pins: Vec::new(),
        }
    }

    pub fn push_pin(&mut self, pin: impl Into<String>) {
        self.pins.push(pin.into());
    }
}

/// One watched memory object's raw bytes for one frame. `bytes.len()`
/// must equal the on-wire `len` (encode derives it; decode enforces it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchRecord {
    pub id: String,
    pub bytes: Vec<u8>,
}

impl WatchRecord {
    pub fn new(id: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        WatchRecord {
            id: id.into(),
            bytes: bytes.into(),
        }
    }
}

/// One frame: one MissionShell loop pass observed at the §2 dump point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameRecord {
    /// g_frame_count (EXW 0x46ae68) / engine tick — the alignment key.
    pub frame_no: u64,
    /// True if this frame's §5 injection was applied before the reads.
    pub injection_applied: bool,
    /// Watch payloads in canonical (registry) order.
    pub watches: Vec<WatchRecord>,
}

impl FrameRecord {
    pub fn new(frame_no: u64, injection_applied: bool) -> Self {
        FrameRecord {
            frame_no,
            injection_applied,
            watches: Vec::new(),
        }
    }

    pub fn push_watch(&mut self, id: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.watches.push(WatchRecord::new(id, bytes));
    }

    /// Look up one watch payload by id.
    pub fn watch(&self, id: &str) -> Option<&[u8]> {
        self.watches
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.bytes.as_slice())
    }
}

/// Stream trailer: frame count + the dump chain digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DumpTrailer {
    pub frame_count: u64,
    pub chain: DumpDigest,
}

/// A fully decoded + integrity-verified dump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dump {
    pub header: DumpHeader,
    pub frames: Vec<FrameRecord>,
    pub frame_digests: Vec<DumpDigest>,
    pub trailer: DumpTrailer,
}

/// Encode/decode failures. Every integrity violation is a hard error —
/// the differ never compares unverified bytes.
#[derive(Debug)]
pub enum DumpError {
    TooShort,
    BadMagic {
        at: usize,
        got: u32,
    },
    BadSchemaVer(u16),
    BadChannel(u8),
    BadInjectionByte(u8),
    BadUtf8,
    StringTooLong {
        field: &'static str,
        len: usize,
    },
    TooManyPins(usize),
    TooManyWatches {
        frame_no: u64,
        count: usize,
    },
    UnknownWatchId(String),
    DuplicateWatchId {
        frame_no: u64,
        id: String,
    },
    FrameNoNotIncreasing {
        prev: u64,
        got: u64,
    },
    DigestMismatch {
        index: usize,
        frame_no: u64,
        stored: u64,
        computed: u64,
    },
    ChainMismatch {
        stored: u64,
        computed: u64,
    },
    FrameCountMismatch {
        stored: u64,
        actual: usize,
    },
    TrailingBytes(usize),
}

impl fmt::Display for DumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DumpError::*;
        match self {
            TooShort => write!(f, "dump truncated: ran out of bytes"),
            BadMagic { at, got } => {
                write!(f, "bad magic at byte {at}: got 0x{got:08x}, want BDLD/BDLT")
            }
            BadSchemaVer(v) => write!(f, "unsupported schema_ver {v} (want {SCHEMA_VER})"),
            BadChannel(c) => write!(f, "unknown channel code {c} (want 1..=4)"),
            BadInjectionByte(b) => write!(f, "injection_applied byte must be 0/1, got {b}"),
            BadUtf8 => write!(f, "string field is not valid UTF-8"),
            StringTooLong { field, len } => {
                write!(f, "{field} too long: {len} bytes (max {MAX_STR})")
            }
            TooManyPins(n) => write!(f, "too many pins: {n} (max u16::MAX)"),
            TooManyWatches { frame_no, count } => {
                write!(f, "frame {frame_no}: too many watches: {count} (max u16::MAX)")
            }
            UnknownWatchId(id) => write!(f, "watch id not in the registry: {id:?}"),
            DuplicateWatchId { frame_no, id } => {
                write!(f, "frame {frame_no}: duplicate watch id {id:?}")
            }
            FrameNoNotIncreasing { prev, got } => {
                write!(f, "frame_no went backwards or repeated: {prev} then {got}")
            }
            DigestMismatch {
                index,
                frame_no,
                stored,
                computed,
            } => write!(
                f,
                "frame #{index} (frame_no {frame_no}): digest mismatch stored 0x{stored:016x} computed 0x{computed:016x}"
            ),
            ChainMismatch { stored, computed } => write!(
                f,
                "chain digest mismatch stored 0x{stored:016x} computed 0x{computed:016x}"
            ),
            FrameCountMismatch { stored, actual } => {
                write!(f, "trailer frame_count {stored} != {actual} decoded frames")
            }
            TrailingBytes(n) => write!(f, "{n} trailing bytes after the trailer"),
        }
    }
}

impl std::error::Error for DumpError {}

fn check_str(field: &'static str, s: &str) -> Result<(), DumpError> {
    if s.len() > MAX_STR {
        return Err(DumpError::StringTooLong {
            field,
            len: s.len(),
        });
    }
    Ok(())
}

/// Append the header's wire encoding.
pub fn encode_header(header: &DumpHeader, out: &mut Vec<u8>) -> Result<(), DumpError> {
    check_str("scenario", &header.scenario)?;
    if header.pins.len() > u16::MAX as usize {
        return Err(DumpError::TooManyPins(header.pins.len()));
    }
    for pin in &header.pins {
        check_str("pin", pin)?;
    }
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&header.schema_ver.to_le_bytes());
    out.push(header.channel.code());
    out.extend_from_slice(&header.build_sha256);
    out.push(header.scenario.len() as u8);
    out.extend_from_slice(header.scenario.as_bytes());
    out.extend_from_slice(&(header.pins.len() as u16).to_le_bytes());
    for pin in &header.pins {
        out.push(pin.len() as u8);
        out.extend_from_slice(pin.as_bytes());
    }
    Ok(())
}

/// Append a frame's canonical bytes (the digest input: everything from
/// the leading BDLD tag through the last watch payload).
pub fn canonical_frame_bytes(frame: &FrameRecord, out: &mut Vec<u8>) -> Result<(), DumpError> {
    if frame.watches.len() > u16::MAX as usize {
        return Err(DumpError::TooManyWatches {
            frame_no: frame.frame_no,
            count: frame.watches.len(),
        });
    }
    for w in &frame.watches {
        check_str("watch id", &w.id)?;
        if w.bytes.len() > u32::MAX as usize {
            return Err(DumpError::StringTooLong {
                field: "watch payload",
                len: w.bytes.len(),
            });
        }
    }
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&frame.frame_no.to_le_bytes());
    out.push(u8::from(frame.injection_applied));
    out.extend_from_slice(&(frame.watches.len() as u16).to_le_bytes());
    for w in &frame.watches {
        out.push(w.id.len() as u8);
        out.extend_from_slice(w.id.as_bytes());
        out.extend_from_slice(&(w.bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&w.bytes);
    }
    Ok(())
}

/// Per-frame digest: FNV-1a 64 over the canonical bytes (BDLD-tagged).
pub fn frame_digest(frame: &FrameRecord) -> Result<DumpDigest, DumpError> {
    let mut buf = Vec::new();
    canonical_frame_bytes(frame, &mut buf)?;
    Ok(DumpDigest(fnv1a64(&buf)))
}

/// Append a frame's full wire record (canonical bytes + digest).
pub fn encode_frame(frame: &FrameRecord, out: &mut Vec<u8>) -> Result<(), DumpError> {
    canonical_frame_bytes(frame, out)?;
    let d = frame_digest(frame)?;
    out.extend_from_slice(&d.0.to_le_bytes());
    Ok(())
}

/// The dump chain digest — the D28/parity_harness construction:
/// incremental FNV-1a 64 fed each frame digest as `write_u64` in order.
pub fn chain_digest(frame_digests: &[DumpDigest]) -> DumpDigest {
    let mut h = Fnv1a64::new();
    for d in frame_digests {
        h.write_u64(d.0);
    }
    DumpDigest(h.finish())
}

/// Sort a frame's watches into the committed registry's file order (the
/// canonical order), rejecting ids the registry does not know and
/// duplicate ids. Stable: equal elements never were equal.
pub fn canonicalize_frame(frame: &mut FrameRecord, reg: &[Watch]) -> Result<(), DumpError> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for w in &frame.watches {
        if !seen.insert(w.id.as_str()) {
            return Err(DumpError::DuplicateWatchId {
                frame_no: frame.frame_no,
                id: w.id.clone(),
            });
        }
    }
    let index_of = |id: &str| -> Option<usize> { reg.iter().position(|w| w.id == id) };
    // Reject unknowns BEFORE sorting so the error names the offender.
    for w in &frame.watches {
        if index_of(&w.id).is_none() {
            return Err(DumpError::UnknownWatchId(w.id.clone()));
        }
    }
    frame
        .watches
        .sort_by_key(|w| index_of(&w.id).expect("checked above"));
    Ok(())
}

/// Encode a whole dump: header + canonicalized frames + trailer.
///
/// `reg` is the committed registry (`diffharness::registry()`): it both
/// validates ids and fixes the canonical watch order, so the same
/// observed state yields byte-identical dumps on every channel.
/// `frame_no` must strictly increase across `frames`.
pub fn encode_dump(
    header: &DumpHeader,
    frames: &[FrameRecord],
    reg: &[Watch],
) -> Result<Vec<u8>, DumpError> {
    let mut out = Vec::new();
    encode_header(header, &mut out)?;
    let mut digests = Vec::with_capacity(frames.len());
    let mut prev: Option<u64> = None;
    for f in frames {
        if let Some(p) = prev {
            if f.frame_no <= p {
                return Err(DumpError::FrameNoNotIncreasing {
                    prev: p,
                    got: f.frame_no,
                });
            }
        }
        prev = Some(f.frame_no);
        let mut canon = f.clone();
        canonicalize_frame(&mut canon, reg)?;
        encode_frame(&canon, &mut out)?;
        digests.push(frame_digest(&canon)?);
    }
    let chain = chain_digest(&digests);
    out.extend_from_slice(&MAGIC_TRAILER);
    out.extend_from_slice(&(frames.len() as u64).to_le_bytes());
    out.extend_from_slice(&chain.0.to_le_bytes());
    Ok(out)
}

// ---------------------------------------------------------------------
// Decode

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DumpError> {
        let end = self.pos.checked_add(n).ok_or(DumpError::TooShort)?;
        if end > self.data.len() {
            return Err(DumpError::TooShort);
        }
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, DumpError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, DumpError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, DumpError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, DumpError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn magic(&mut self) -> Result<[u8; 4], DumpError> {
        let b = self.take(4)?;
        Ok([b[0], b[1], b[2], b[3]])
    }

    fn magic_u32(m: [u8; 4]) -> u32 {
        u32::from_le_bytes(m)
    }

    fn str_field(&mut self) -> Result<String, DumpError> {
        let len = self.u8()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| DumpError::BadUtf8)
    }
}

/// Decode + integrity-verify a whole dump stream. Verifies every frame
/// digest, the frame-count, and the chain digest; rejects reordered or
/// repeated `frame_no`, duplicate watch ids in one frame, non-0/1
/// injection bytes, unknown schema/channel codes, and trailing bytes.
pub fn decode_dump(bytes: &[u8]) -> Result<Dump, DumpError> {
    let mut r = Reader {
        data: bytes,
        pos: 0,
    };

    // Header.
    let m = r.magic()?;
    if m != MAGIC {
        return Err(DumpError::BadMagic {
            at: 0,
            got: Reader::magic_u32(m),
        });
    }
    let schema_ver = r.u16()?;
    if schema_ver != SCHEMA_VER {
        return Err(DumpError::BadSchemaVer(schema_ver));
    }
    let channel_code = r.u8()?;
    let channel = Channel::from_code(channel_code).ok_or(DumpError::BadChannel(channel_code))?;
    let mut build_sha256 = [0u8; 32];
    build_sha256.copy_from_slice(r.take(32)?);
    let scenario = r.str_field()?;
    let pin_count = r.u16()? as usize;
    let mut pins = Vec::with_capacity(pin_count);
    for _ in 0..pin_count {
        pins.push(r.str_field()?);
    }
    let header = DumpHeader {
        schema_ver,
        channel,
        build_sha256,
        scenario,
        pins,
    };

    // Frames until the trailer tag.
    let mut frames: Vec<FrameRecord> = Vec::new();
    let mut frame_digests: Vec<DumpDigest> = Vec::new();
    let mut prev_frame_no: Option<u64> = None;
    loop {
        let at = r.pos;
        let m = r.magic()?;
        if m == MAGIC_TRAILER {
            break;
        }
        if m != MAGIC {
            return Err(DumpError::BadMagic {
                at,
                got: Reader::magic_u32(m),
            });
        }
        // Re-encode the canonical prefix as we parse it, so the digest
        // is recomputed over exactly the bytes just consumed.
        let mut canon = Vec::with_capacity(64);
        canon.extend_from_slice(&MAGIC);

        let frame_no = r.u64()?;
        canon.extend_from_slice(&frame_no.to_le_bytes());
        let inj = r.u8()?;
        if inj > 1 {
            return Err(DumpError::BadInjectionByte(inj));
        }
        canon.push(inj);
        let watch_count = r.u16()? as usize;
        canon.extend_from_slice(&(watch_count as u16).to_le_bytes());
        let mut watches = Vec::with_capacity(watch_count);
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for _ in 0..watch_count {
            let id = r.str_field()?;
            canon.push(id.len() as u8);
            canon.extend_from_slice(id.as_bytes());
            let len = r.u32()? as usize;
            canon.extend_from_slice(&(len as u32).to_le_bytes());
            let payload = r.take(len)?;
            canon.extend_from_slice(payload);
            if !seen.insert(id.clone()) {
                return Err(DumpError::DuplicateWatchId { frame_no, id });
            }
            watches.push(WatchRecord {
                id,
                bytes: payload.to_vec(),
            });
        }
        let stored = r.u64()?;
        let computed = fnv1a64(&canon);
        if stored != computed {
            return Err(DumpError::DigestMismatch {
                index: frames.len(),
                frame_no,
                stored,
                computed,
            });
        }
        if let Some(p) = prev_frame_no {
            if frame_no <= p {
                return Err(DumpError::FrameNoNotIncreasing {
                    prev: p,
                    got: frame_no,
                });
            }
        }
        prev_frame_no = Some(frame_no);
        frames.push(FrameRecord {
            frame_no,
            injection_applied: inj == 1,
            watches,
        });
        frame_digests.push(DumpDigest(stored));
    }

    // Trailer (tag consumed above).
    let frame_count = r.u64()?;
    let chain_stored = r.u64()?;
    if frame_count as usize != frames.len() {
        return Err(DumpError::FrameCountMismatch {
            stored: frame_count,
            actual: frames.len(),
        });
    }
    let chain_computed = chain_digest(&frame_digests).0;
    if chain_stored != chain_computed {
        return Err(DumpError::ChainMismatch {
            stored: chain_stored,
            computed: chain_computed,
        });
    }
    if r.pos != bytes.len() {
        return Err(DumpError::TrailingBytes(bytes.len() - r.pos));
    }
    Ok(Dump {
        header,
        frames,
        frame_digests,
        trailer: DumpTrailer {
            frame_count,
            chain: DumpDigest(chain_stored),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_codes_roundtrip_and_reject_5() {
        for c in [
            Channel::O1ExdDosboxX,
            Channel::O2ExwWine,
            Channel::O3Street,
            Channel::Engine,
        ] {
            assert_eq!(Channel::from_code(c.code()), Some(c));
        }
        assert_eq!(Channel::from_code(0), None);
        assert_eq!(Channel::from_code(5), None);
    }

    #[test]
    fn canonical_bytes_start_with_tag_and_digest_matches_recompute() {
        let mut f = FrameRecord::new(7, false);
        f.push_watch("score", 1234u32.to_le_bytes());
        let mut buf = Vec::new();
        canonical_frame_bytes(&f, &mut buf).unwrap();
        assert_eq!(&buf[..4], b"BDLD");
        assert_eq!(frame_digest(&f).unwrap().0, fnv1a64(&buf));
    }
}
