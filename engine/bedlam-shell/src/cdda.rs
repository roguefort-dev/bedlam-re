//! The P7 CDDA user-supply + local-cache surface (PLAN §6 P7
//! "CDDA: user-supplied original tracks (WAV/CD), optional local
//! lossy cache generated on first run — never redistributed";
//! docs/P7-PORTS.md §4, the D221 contract; implementation D223).
//!
//! GROUNDED FACTS (all already landed, VERIFIED — no new RE in this
//! unit): the original CD is MIXED-MODE — track 1 is the data
//! track, tracks 2..8 are SEVEN CDDA audio tracks (~206–225 s each,
//! 44.1 kHz 16-bit stereo; the corpus carries the WAV rips
//! `BEDLAM02..08.WAV` — GROUNDWORK.md, RESEARCH-8STREET.md, header
//! shape re-read first-hand for this unit: RIFF + fmt PCM 1/2 ch/
//! 44100/16-bit + data, in the install dir); the EXW drives them
//! through the MCI CD-audio path (RE-EXW-MAINLOOP.md), i.e. CD
//! audio NEVER entered the sampled mixer — it is presentation
//! bucket by the original's own construction (D17 b).
//!
//! THE LOOKUP (documented, user-supplied, SILENT MISS). Music
//! resolves through an ordered probe of USER-OWNED locations for
//! each of the seven tracks. Per track the candidate file names
//! (first match wins) are `BEDLAM0N.WAV` (the corpus rip name) then
//! `TRACK0N.WAV` (the generic ripper name), N = 2..8 (the CD track
//! number — track 1 is data), matched case-insensitively. The
//! search roots, in priority order:
//!
//! 1. the explicit `--music-dir DIR` / `BEDLAM_MUSIC_DIR` override;
//! 2. the user's music dir `$XDG_DATA_HOME/bedlam/music` (default
//!    `$HOME/.local/share/bedlam/music`);
//! 3. the game's own install directory (where the packaged game's
//!    user drops the rips; in the repo layout that is the
//!    operator's corpus copy and is only ever READ).
//!
//! A MISS IS MUSIC SILENT + one stderr note — never fatal, never a
//! task (the 8street comparator's own CDDA-disabled build is
//! standing evidence the game runs music-silent; P7-PORTS §4).
//! Per-track misses are per-track silence; nothing is guessed.
//!
//! THE OPTIONAL LOCAL LOSSY CACHE (generated on first run, into a
//! USER-OWNED dir, never redistributed). The cache home is
//! `$XDG_CACHE_HOME/bedlam` (default `$HOME/.cache/bedlam`, the
//! platform equivalent on Windows `$LOCALAPPDATA/bedlam/cache`) —
//! NEVER `game-data/`, never the repo, never any build artifact:
//! the startup guard REFUSES a cache home inside the game install
//! tree or inside a git work tree, and the default construction
//! can never land there. Each found track transcodes ONCE into
//! `<cache>/music/trackNN.bcda` (NN = CD track 02..08): the whole
//! 16-bit PCM track IMA-ADPCM-encoded (4:1, integer math, the
//! repo's no-deps posture — a real lossy codec, self-contained,
//! deterministic) behind a small header that records the SOURCE
//! IDENTITY (length + FNV-1a-64 over the source bytes). On every
//! run the identity is recomputed: a match is FRESH (no work), a
//! mismatch REGENERATES the entry (the user's WAV/CD source stays
//! the source of truth). The cache exists to cut decode cost and
//! disk; it is a derived copy (the D21 originals-or-derivatives
//! rule applied to audio) and is NEVER redistributed — the engine
//! never bundles, commits or ships it. `--no-music-cache` opts out;
//! any cache error (unparseable WAV, unwritable dir, …) skips that
//! entry with a note, never fatal.
//!
//! PARITY BOUNDS (D17 b / D212 posture): the music path stays OUT
//! of the sim — this module reads user-owned files, writes the
//! user-owned cache, prints notes, and NOTHING else. It never
//! touches `ModeConfig`, `SimConfig`, the host, the mixer, or any
//! hash; the headless smoke (the hashed-trajectory surface) never
//! runs it (the binary notes + ignores the flags there, like every
//! window-host option).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The seven CDDA audio tracks of the mixed-mode original (track 1
/// is the data track; VERIFIED — GROUNDWORK.md, P7-PORTS §4).
pub const CDDA_TRACK_COUNT: usize = 7;

/// The env var selecting the user-supplied music directory (the
/// `--music-dir` CLI flag's env twin; the flag wins).
pub const MUSIC_DIR_ENV: &str = "BEDLAM_MUSIC_DIR";

/// The cache blob magic: a Bedlam CDDA cache v1 entry.
pub const BLOB_MAGIC: [u8; 8] = *b"BCDDAC01";

/// The fixed blob header length (magic + shape + identity + size).
pub const BLOB_HEADER_LEN: usize = 43;

/// The IMA ADPCM step-size table (the standard 89-entry ladder).
const IMA_STEP_TABLE: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// The IMA ADPCM step-index adjustment per 3-bit code.
const IMA_INDEX_TABLE: [i8; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

// ---------------------------------------------------------------------
// The track model + the documented lookup
// ---------------------------------------------------------------------

/// The CD track number of CDDA music track `index` (0-based): the
/// mixed-mode original's track 1 is the data track, so music track
/// 0 is CD track 2 … music track 6 is CD track 8.
pub const fn cd_track_no(index: usize) -> u32 {
    index as u32 + 2
}

/// The candidate file names for music track `index`, in the
/// documented match order: the corpus rip name first, the generic
/// ripper name second (matched case-insensitively).
pub fn track_candidates(index: usize) -> [String; 2] {
    let n = cd_track_no(index);
    [format!("BEDLAM{n:02}.WAV"), format!("TRACK{n:02}.WAV")]
}

/// A resolved user-supplied source track (name match + file size).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackSource {
    pub path: PathBuf,
    pub len: u64,
}

/// The seven-track lookup outcome: `tracks[i]` is the resolved
/// source for music track `i`, or `None` (that track is SILENT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyReport {
    pub tracks: Vec<Option<TrackSource>>,
}

impl SupplyReport {
    pub fn found_count(&self) -> usize {
        self.tracks.iter().flatten().count()
    }
}

/// The user's music dir (search root 2): `$XDG_DATA_HOME/bedlam/music`,
/// defaulting to `$HOME/.local/share/bedlam/music`.
fn user_music_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir).join("bedlam").join("music"));
        }
    }
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("bedlam")
            .join("music")
    })
}

/// The ordered search roots for the documented lookup (see the
/// module docs): explicit override, then the user's music dir, then
/// the game's own install directory. PURE — no filesystem access.
pub fn search_roots(explicit: Option<&Path>, install_dir: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(dir) = explicit {
        roots.push(dir.to_path_buf());
    }
    if let Some(dir) = user_music_dir() {
        roots.push(dir);
    }
    roots.push(install_dir.to_path_buf());
    roots
}

/// Resolve the seven tracks over the search roots (first match
/// wins, roots in priority order, candidates in name order). NEVER
/// FAILS: an unreadable root contributes nothing; a matched file's
/// size is 0 if metadata fails. This is the whole lookup — the
/// silent-miss posture starts here.
pub fn resolve_supply(roots: &[PathBuf]) -> SupplyReport {
    let mut tracks: Vec<Option<TrackSource>> = vec![None; CDDA_TRACK_COUNT];
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue; // unreadable/missing root: contributes nothing
        };
        // Deterministic order regardless of the platform listing.
        let mut names: Vec<(String, PathBuf)> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    entry.path(),
                )
            })
            .collect();
        names.sort_by(|a, b| a.0.cmp(&b.0));
        for (index, slot) in tracks.iter_mut().enumerate() {
            if slot.is_some() {
                continue;
            }
            for candidate in track_candidates(index) {
                let Some(found) = names
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(&candidate))
                else {
                    continue;
                };
                let len = fs::metadata(&found.1).map(|m| m.len()).unwrap_or(0);
                *slot = Some(TrackSource {
                    path: found.1.clone(),
                    len,
                });
                break;
            }
        }
    }
    SupplyReport { tracks }
}

/// The one-line supply note (stderr; the SILENT MISS wording).
pub fn supply_note(report: &SupplyReport, roots: &[PathBuf]) -> String {
    let found = report.found_count();
    let first_root = roots
        .first()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "a user music directory".to_string());
    match found {
        CDDA_TRACK_COUNT => {
            format!("music: {found}/{CDDA_TRACK_COUNT} user-supplied CDDA tracks resolved")
        }
        0 => format!(
            "music: no user-supplied CDDA tracks found - music SILENT (never fatal); \
             supply BEDLAM02..08.WAV (or TRACK02..08.WAV) rips under {first_root}"
        ),
        n => format!(
            "music: {n}/{CDDA_TRACK_COUNT} user-supplied CDDA tracks resolved; the \
             missing tracks play SILENT (never fatal); supply BEDLAM02..08.WAV (or \
             TRACK02..08.WAV) rips under {first_root}"
        ),
    }
}

// ---------------------------------------------------------------------
// WAV (RIFF) parsing — the user-supplied source shape
// ---------------------------------------------------------------------

/// Why a source did not parse (each is a per-track SKIP, never fatal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavError {
    NotRiff,
    Truncated,
    NotPcm(u16),
    Not16Bit(u16),
    NoData,
}

impl std::fmt::Display for WavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WavError::NotRiff => write!(f, "not a RIFF/WAVE file"),
            WavError::Truncated => write!(f, "truncated RIFF structure"),
            WavError::NotPcm(format) => write!(f, "unsupported WAV format {format} (want PCM)"),
            WavError::Not16Bit(bits) => write!(f, "unsupported WAV depth {bits} (want 16-bit)"),
            WavError::NoData => write!(f, "no data chunk"),
        }
    }
}

/// The parsed shape of a 16-bit PCM WAV file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavData {
    pub sample_rate: u32,
    pub channels: u16,
    pub data_offset: usize,
    pub data_len: usize,
}

fn u16le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Parse a whole WAV file image: RIFF header, then a chunk walk
/// (odd-sized chunks are word-aligned by one pad byte), requiring a
/// `fmt ` chunk of uncompressed PCM (format 1) at 16 bits. The
/// corpus rips are exactly this shape (verified first-hand:
/// `fmt ` 16 bytes, PCM, 2 channels, 44100 Hz, 16-bit, `data`
/// immediately following).
pub fn parse_wav(bytes: &[u8]) -> Result<WavData, WavError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::NotRiff);
    }
    let mut off = 12;
    let mut fmt: Option<(u16, u32)> = None;
    let mut data: Option<(usize, usize)> = None;
    while off + 8 <= bytes.len() {
        let id = &bytes[off..off + 4];
        let size = u32le(bytes, off + 4) as usize;
        let body = off + 8;
        if body.checked_add(size).is_none_or(|end| end > bytes.len()) {
            return Err(WavError::Truncated);
        }
        if id == b"fmt " && size >= 16 {
            let format = u16le(bytes, body);
            let channels = u16le(bytes, body + 2);
            let rate = u32le(bytes, body + 4);
            let bits = u16le(bytes, body + 14);
            if format != 1 {
                return Err(WavError::NotPcm(format));
            }
            if bits != 16 {
                return Err(WavError::Not16Bit(bits));
            }
            fmt = Some((channels, rate));
        } else if id == b"data" {
            data = Some((body, size));
        }
        // Word alignment: odd chunks carry one pad byte.
        off = body + size + (size & 1);
    }
    match (fmt, data) {
        (Some((channels, sample_rate)), Some((data_offset, data_len))) if channels >= 1 => {
            Ok(WavData {
                sample_rate,
                channels,
                data_offset,
                data_len,
            })
        }
        (Some(_), None) => Err(WavError::NoData),
        _ => Err(WavError::Truncated),
    }
}

// ---------------------------------------------------------------------
// IMA ADPCM — the cache's lossy codec (integer math, no deps)
// ---------------------------------------------------------------------

/// One channel's IMA ADPCM coder state (predictor + step index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImaAdpcm {
    predictor: i16,
    step_index: u8,
}

impl ImaAdpcm {
    pub const fn new() -> ImaAdpcm {
        ImaAdpcm {
            predictor: 0,
            step_index: 0,
        }
    }

    /// Encode one sample to a 3-bit code (+ sign bit in bit 3).
    pub fn encode(&mut self, sample: i16) -> u8 {
        let diff = i32::from(sample) - i32::from(self.predictor);
        let sign = u8::from(diff < 0) << 3;
        let mut diff = diff.abs();
        let step = i32::from(IMA_STEP_TABLE[self.step_index as usize]);
        let mut code = 0u8;
        let mut delta = step >> 3;
        if diff >= step {
            code |= 4;
            diff -= step;
            delta += step;
        }
        if diff >= step >> 1 {
            code |= 2;
            diff -= step >> 1;
            delta += step >> 1;
        }
        if diff >= step >> 2 {
            code |= 1;
            delta += step >> 2;
        }
        let value = i32::from(self.predictor) + if sign != 0 { -delta } else { delta };
        self.predictor = value.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        self.bump_index(code);
        code | sign
    }

    /// Decode one 3-bit code (+ sign bit) back to a sample.
    pub fn decode(&mut self, code: u8) -> i16 {
        let step = i32::from(IMA_STEP_TABLE[self.step_index as usize]);
        let mut diff = step >> 3;
        if code & 4 != 0 {
            diff += step;
        }
        if code & 2 != 0 {
            diff += step >> 1;
        }
        if code & 1 != 0 {
            diff += step >> 2;
        }
        let value = i32::from(self.predictor) + if code & 8 != 0 { -diff } else { diff };
        self.predictor = value.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        self.bump_index(code);
        self.predictor
    }

    fn bump_index(&mut self, code: u8) {
        let index = (i16::from(self.step_index) + i16::from(IMA_INDEX_TABLE[(code & 7) as usize]))
            .clamp(0, (IMA_STEP_TABLE.len() - 1) as i16);
        self.step_index = index as u8;
    }
}

impl Default for ImaAdpcm {
    fn default() -> Self {
        ImaAdpcm::new()
    }
}

/// Encode a whole track's 16-bit PCM (interleaved channels) to
/// packed IMA ADPCM nibbles (low nibble first), one coder state per
/// channel. Output is exactly `ceil(samples / 2)` bytes — a real
/// 4:1 lossy reduction of the 16-bit source.
pub fn encode_track(wav: &WavData, file: &[u8]) -> Result<Vec<u8>, WavError> {
    if wav.data_offset + wav.data_len > file.len() {
        return Err(WavError::Truncated);
    }
    let channels = wav.channels.max(1) as usize;
    let bytes = &file[wav.data_offset..wav.data_offset + wav.data_len];
    let samples = bytes.len() / 2;
    let mut states = vec![ImaAdpcm::new(); channels];
    let mut out = vec![0u8; samples.div_ceil(2)];
    for (i, chunk) in bytes.chunks_exact(2).enumerate() {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        let code = states[i % channels].encode(sample);
        if i % 2 == 0 {
            out[i / 2] |= code & 0x0F; // low nibble first
        } else {
            out[i / 2] |= (code & 0x0F) << 4;
        }
    }
    Ok(out)
}

/// Decode a whole cached track back to interleaved 16-bit PCM (the
/// cache's own reader — the round-trip pin and future playback).
pub fn decode_track(channels: u16, nibbles: &[u8], samples: usize) -> Vec<i16> {
    let channels = channels.max(1) as usize;
    let mut states = vec![ImaAdpcm::new(); channels];
    let mut out = Vec::with_capacity(samples);
    for i in 0..samples {
        let byte = nibbles.get(i / 2).copied().unwrap_or(0);
        let code = if i % 2 == 0 { byte & 0x0F } else { byte >> 4 };
        out.push(states[i % channels].decode(code));
    }
    out
}

// ---------------------------------------------------------------------
// Source identity + the cache blob
// ---------------------------------------------------------------------

/// The identity of a user-supplied source: its byte length plus an
/// FNV-1a-64 over its bytes. This is the cache KEY — the user's
/// WAV/CD source stays the source of truth; the cache entry is
/// valid exactly while this identity holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceIdentity {
    pub len: u64,
    pub fnv: u64,
}

/// FNV-1a-64 over `bytes` (a stdlib-quality mixer; deterministic,
/// platform-independent — the identity only needs to be stable, not
/// cryptographic).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Identify a source file by streaming it (chunked; a 40 MB corpus
/// rip is one sequential pass).
pub fn identify(path: &Path) -> std::io::Result<SourceIdentity> {
    let len = fs::metadata(path)?.len();
    let mut file = fs::File::open(path)?;
    let mut chunk = vec![0u8; 1 << 20];
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    loop {
        let read = file.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        for &byte in &chunk[..read] {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Ok(SourceIdentity { len, fnv: hash })
}

/// The parsed cache blob header (the identity + shape of one entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlobHeader {
    pub track: u8,
    pub sample_rate: u32,
    pub channels: u16,
    pub src_frames: u64,
    pub identity: SourceIdentity,
    pub nibble_bytes: u32,
}

/// Build one cache blob (header + nibbles) for `track`.
pub fn build_blob(
    track: usize,
    wav: &WavData,
    identity: SourceIdentity,
    nibbles: &[u8],
) -> Vec<u8> {
    let src_frames = (wav.data_len / (2 * wav.channels.max(1) as usize)) as u64;
    let mut blob = Vec::with_capacity(BLOB_HEADER_LEN + nibbles.len());
    blob.extend_from_slice(&BLOB_MAGIC);
    blob.push(track as u8);
    blob.extend_from_slice(&wav.sample_rate.to_le_bytes());
    blob.extend_from_slice(&wav.channels.to_le_bytes());
    blob.extend_from_slice(&src_frames.to_le_bytes());
    blob.extend_from_slice(&identity.len.to_le_bytes());
    blob.extend_from_slice(&identity.fnv.to_le_bytes());
    blob.extend_from_slice(&(nibbles.len() as u32).to_le_bytes());
    blob.extend_from_slice(nibbles);
    blob
}

/// Parse a cache blob's header; `None` on any shape mismatch
/// (treated as a mismatch: the entry regenerates).
pub fn parse_blob_header(bytes: &[u8]) -> Option<BlobHeader> {
    if bytes.len() < BLOB_HEADER_LEN || bytes[0..8] != BLOB_MAGIC {
        return None;
    }
    let track = bytes[8];
    if track as usize >= CDDA_TRACK_COUNT {
        return None;
    }
    Some(BlobHeader {
        track,
        sample_rate: u32le(bytes, 9),
        channels: u16le(bytes, 13),
        src_frames: u64::from_le_bytes(bytes[15..23].try_into().ok()?),
        identity: SourceIdentity {
            len: u64::from_le_bytes(bytes[23..31].try_into().ok()?),
            fnv: u64::from_le_bytes(bytes[31..39].try_into().ok()?),
        },
        nibble_bytes: u32le(bytes, 39),
    })
}

/// The per-track cache decision: FRESH (identity matches — no work)
/// or REGENERATE (absent/corrupt/mismatched — rebuild the entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheVerdict {
    Fresh,
    Regenerate,
}

/// The verdict for one cached header against one source identity.
pub fn cache_verdict(cached: Option<BlobHeader>, identity: SourceIdentity) -> CacheVerdict {
    match cached {
        Some(header) if header.identity == identity => CacheVerdict::Fresh,
        _ => CacheVerdict::Regenerate,
    }
}

/// The cache file for music track `index` under `cache_root`.
pub fn cache_entry_path(cache_root: &Path, index: usize) -> PathBuf {
    cache_root
        .join("music")
        .join(format!("track{:02}.bcda", cd_track_no(index)))
}

/// What happened to one track's cache entry during a refresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheOutcome {
    Fresh,
    Generated,
    Skipped(String),
}

/// Refresh the cache entries for every RESOLVED track under
/// `cache_root` (the guarded core — the startup glue applies the
/// never-game-data/never-repo refusals before calling this). Per
/// track: identify the source, read + verdict the cached blob, and
/// REGENERATE on mismatch (write `.tmp`, then rename — a torn entry
/// is impossible). ANY error skips that entry with its reason —
/// never fatal, the source stays playable, the next run retries.
pub fn refresh_cache(report: &SupplyReport, cache_root: &Path) -> Vec<CacheOutcome> {
    let mut outcomes = Vec::new();
    for (index, slot) in report.tracks.iter().enumerate() {
        let Some(source) = slot else {
            continue; // unresolved track: nothing to cache
        };
        let identity = match identify(&source.path) {
            Ok(identity) => identity,
            Err(err) => {
                outcomes.push(CacheOutcome::Skipped(format!(
                    "track {}: {}",
                    cd_track_no(index),
                    err
                )));
                continue;
            }
        };
        let entry = cache_entry_path(cache_root, index);
        let cached = fs::read(&entry).ok().and_then(|b| parse_blob_header(&b));
        if cache_verdict(cached, identity) == CacheVerdict::Fresh {
            outcomes.push(CacheOutcome::Fresh);
            continue;
        }
        outcomes.push(
            match generate_entry(index, &source.path, identity, &entry) {
                Ok(()) => CacheOutcome::Generated,
                Err(reason) => CacheOutcome::Skipped(reason),
            },
        );
    }
    outcomes
}

/// Generate one cache entry (parse + encode + atomic write). The
/// error is a human-readable per-track skip reason.
fn generate_entry(
    index: usize,
    source: &Path,
    identity: SourceIdentity,
    entry: &Path,
) -> Result<(), String> {
    let file = fs::read(source).map_err(|err| format!("track {}: {err}", cd_track_no(index)))?;
    let wav = parse_wav(&file).map_err(|err| format!("track {}: {err}", cd_track_no(index)))?;
    let nibbles =
        encode_track(&wav, &file).map_err(|err| format!("track {}: {err}", cd_track_no(index)))?;
    let blob = build_blob(index, &wav, identity, &nibbles);
    if let Some(parent) = entry.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("track {}: {err}", cd_track_no(index)))?;
    }
    let tmp = entry.with_extension("bcda.tmp");
    fs::write(&tmp, &blob).map_err(|err| format!("track {}: {err}", cd_track_no(index)))?;
    fs::rename(&tmp, entry).map_err(|err| format!("track {}: {err}", cd_track_no(index)))
}

/// The one-line cache note (stderr).
pub fn cache_note(outcomes: &[CacheOutcome], cache_root: &Path) -> String {
    let fresh = outcomes
        .iter()
        .filter(|o| **o == CacheOutcome::Fresh)
        .count();
    let generated = outcomes
        .iter()
        .filter(|o| **o == CacheOutcome::Generated)
        .count();
    let skipped: Vec<&String> = outcomes
        .iter()
        .filter_map(|o| match o {
            CacheOutcome::Skipped(reason) => Some(reason),
            _ => None,
        })
        .collect();
    let mut note = format!(
        "cache: {generated} generated, {fresh} fresh, in {} (lossy local \
         cache, user-owned, never redistributed)",
        cache_root.display()
    );
    if !skipped.is_empty() {
        note.push_str(&format!(
            "; skipped {} ({} and more)",
            skipped.len(),
            skipped[0]
        ));
    }
    note
}

// ---------------------------------------------------------------------
// The cache home + the never-game-data/never-repo guards
// ---------------------------------------------------------------------

/// The user-owned cache home (the D221 contract: the XDG cache home
/// or the platform equivalent — never `game-data/`, never the repo,
/// never a build artifact, by construction of the default).
pub fn cache_home() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir).join("bedlam"));
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home).join(".cache").join("bedlam"));
        }
    }
    // The Windows platform equivalent.
    std::env::var_os("LOCALAPPDATA")
        .filter(|dir| !dir.is_empty())
        .map(|dir| PathBuf::from(dir).join("bedlam").join("cache"))
}

/// Component-wise containment (a path is inside itself): `/a/b` is
/// inside `/a`, `/ab` is NOT inside `/a`. PURE — no filesystem.
pub fn path_is_inside(path: &Path, dir: &Path) -> bool {
    path.starts_with(dir)
}

/// Whether `root` sits inside a git work tree (a `.git` entry at
/// the root or any ancestor) — the never-the-repo guard's probe.
pub fn inside_git_worktree(root: &Path) -> bool {
    root.ancestors().any(|dir| dir.join(".git").exists())
}

// ---------------------------------------------------------------------
// The startup glue (window host only; never the headless path)
// ---------------------------------------------------------------------

/// The platform music-cache policy: the OPTIONAL local lossy cache
/// is ON by default (the plan's "generated on first run" posture);
/// `Disabled` is the `--no-music-cache` opt-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MusicCachePolicy {
    #[default]
    Enabled,
    Disabled,
}

/// The platform CDDA options carried on
/// [`crate::window::WindowOptions::music`]: the explicit search-dir
/// override (the `--music-dir` flag; the `BEDLAM_MUSIC_DIR` env is
/// consulted when the flag is absent) and the cache policy. D200
/// layering: a PLATFORM knob, OUT of `ModeConfig` — it selects
/// nothing in the host, never reaches the sim config or any hash.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CddaOptions {
    pub search_dir: Option<PathBuf>,
    pub cache: MusicCachePolicy,
}

/// The one-shot window-host startup: resolve the supply (one note,
/// silent-miss wording), then — only for RESOLVED tracks with the
/// cache enabled — refresh the local cache under the guarded cache
/// home. Every failure mode is a note; nothing here can fail the
/// boot, touch the host, or write anywhere user-owned paths do not
/// already permit.
pub fn startup(explicit_dir: Option<&Path>, install_dir: &Path, policy: MusicCachePolicy) {
    let roots = search_roots(explicit_dir, install_dir);
    let report = resolve_supply(&roots);
    eprintln!("bedlam-shell: {}", supply_note(&report, &roots));
    if report.found_count() == 0 {
        return; // music silent; nothing to cache
    }
    if policy == MusicCachePolicy::Disabled {
        eprintln!("bedlam-shell: music cache: disabled by option (--no-music-cache)");
        return;
    }
    let Some(root) = cache_home() else {
        eprintln!(
            "bedlam-shell: music cache: no user-owned cache home (set XDG_CACHE_HOME); \
             running without the local cache"
        );
        return;
    };
    if path_is_inside(
        &absolute_for_compare(&root),
        &absolute_for_compare(install_dir),
    ) {
        eprintln!(
            "bedlam-shell: music cache: refused - the cache never lands in the game \
             install tree (game-data stays read-only)"
        );
        return;
    }
    if inside_git_worktree(&root) {
        eprintln!(
            "bedlam-shell: music cache: refused - the cache never lands in a repository \
             work tree"
        );
        return;
    }
    let outcomes = refresh_cache(&report, &root);
    eprintln!("bedlam-shell: music {}", cache_note(&outcomes, &root));
}

/// Best-effort absolute form of `path` for a containment compare:
/// canonicalized when it exists (resolves symlinks), else lexically
/// absolutized against the working directory (a relative install
/// dir must still be comparable against an absolute cache home).
fn absolute_for_compare(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A unique scratch dir under the system temp dir (the gate's
    /// TMPDIR is the writable target bind; nothing here ever touches
    /// the repo or the corpus). Best-effort cleanup via Drop.
    struct Scratch(PathBuf);

    fn scratch(tag: &str) -> Scratch {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("bedlam-cdda-{tag}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(dir)
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A minimal well-formed 16-bit PCM WAV image (the corpus rip
    /// shape: RIFF + fmt PCM/ch/rate/16 + data) over `samples`
    /// interleaved i16 frames from a generator.
    fn wav_bytes(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
        let mut pcm = Vec::new();
        for s in samples {
            pcm.extend_from_slice(&s.to_le_bytes());
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        let riff_len = 4 + (8 + 16) + (8 + pcm.len() as u32);
        bytes.extend_from_slice(&riff_len.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        let block_align = channels * 2;
        bytes.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&pcm);
        bytes
    }

    /// Place the seven corpus-shaped names in `dir` (contents
    /// optional — the lookup only matches names + sizes).
    fn place_tracks(dir: &Path, bodies: &[Option<Vec<u8>>]) {
        for (index, body) in bodies.iter().enumerate() {
            let name = format!("BEDLAM{:02}.WAV", cd_track_no(index));
            let bytes = body
                .clone()
                .unwrap_or_else(|| wav_bytes(2, 44_100, &[0; 16]));
            fs::write(dir.join(name), bytes).expect("place track");
        }
    }

    #[test]
    fn track_numbering_matches_the_mixed_mode_cd() {
        assert_eq!(cd_track_no(0), 2, "track 1 is the DATA track");
        assert_eq!(cd_track_no(CDDA_TRACK_COUNT - 1), 8);
        assert_eq!(
            track_candidates(0),
            ["BEDLAM02.WAV".to_string(), "TRACK02.WAV".to_string()]
        );
        assert_eq!(
            track_candidates(6),
            ["BEDLAM08.WAV".to_string(), "TRACK08.WAV".to_string()]
        );
    }

    #[test]
    fn lookup_resolves_all_seven_case_insensitively() {
        let dir = scratch("lookup-lower");
        for index in 0..CDDA_TRACK_COUNT {
            let name = format!("bedlam{:02}.wav", cd_track_no(index)); // lowercase
            fs::write(dir.0.join(name), wav_bytes(2, 44_100, &[0; 8])).unwrap();
        }
        let report = resolve_supply(std::slice::from_ref(&dir.0));
        assert_eq!(report.found_count(), CDDA_TRACK_COUNT);
        for (index, slot) in report.tracks.iter().enumerate() {
            let source = slot.as_ref().expect("resolved");
            assert!(source
                .path
                .ends_with(format!("bedlam{:02}.wav", cd_track_no(index))));
            assert_eq!(source.len, fs::metadata(&source.path).unwrap().len());
        }
    }

    #[test]
    fn lookup_prefers_bedlam_names_then_track_names() {
        let dir = scratch("lookup-names");
        // Both candidates for track 0: BEDLAM wins.
        fs::write(dir.0.join("BEDLAM02.WAV"), [0u8; 4]).unwrap();
        fs::write(dir.0.join("TRACK02.WAV"), [0u8; 6]).unwrap();
        // Only the generic name for track 1: it matches.
        fs::write(dir.0.join("TRACK03.WAV"), [0u8; 8]).unwrap();
        let report = resolve_supply(std::slice::from_ref(&dir.0));
        assert!(report.tracks[0]
            .as_ref()
            .unwrap()
            .path
            .ends_with("BEDLAM02.WAV"));
        assert!(report.tracks[1]
            .as_ref()
            .unwrap()
            .path
            .ends_with("TRACK03.WAV"));
        assert_eq!(report.found_count(), 2);
    }

    #[test]
    fn lookup_roots_follow_priority_order_and_miss_silently() {
        let high = scratch("lookup-high");
        let low = scratch("lookup-low");
        place_tracks(
            &high.0,
            &std::array::from_fn::<_, CDDA_TRACK_COUNT, _>(|_| None),
        );
        // The lower root only carries track 0; the earlier root wins.
        fs::write(low.0.join("BEDLAM02.WAV"), [0u8; 2]).unwrap();
        let missing = scratch("lookup-missing"); // exists, is empty
        let report = resolve_supply(&[high.0.clone(), low.0.clone(), missing.0.clone()]);
        assert_eq!(report.found_count(), CDDA_TRACK_COUNT);
        assert!(report.tracks[0].as_ref().unwrap().path.starts_with(&high.0));
        // An entirely empty root set is a full miss — never an error.
        let none = resolve_supply(std::slice::from_ref(&missing.0));
        assert_eq!(none.found_count(), 0);
        assert!(none.tracks.iter().all(Option::is_none));
        // A nonexistent root contributes nothing (no panic).
        let ghost = resolve_supply(&[missing.0.join("no-such-dir")]);
        assert_eq!(ghost.found_count(), 0);
    }

    #[test]
    fn supply_note_carries_the_silent_miss_posture() {
        let dir = scratch("note");
        let full = resolve_supply({
            place_tracks(
                &dir.0,
                &std::array::from_fn::<_, CDDA_TRACK_COUNT, _>(|_| None),
            );
            std::slice::from_ref(&dir.0)
        });
        let note = supply_note(&full, std::slice::from_ref(&dir.0));
        assert_eq!(
            note,
            "music: 7/7 user-supplied CDDA tracks resolved".to_string()
        );
        let empty = SupplyReport {
            tracks: vec![None; CDDA_TRACK_COUNT],
        };
        let note = supply_note(&empty, std::slice::from_ref(&dir.0));
        assert!(
            note.contains("no user-supplied CDDA tracks found"),
            "{note}"
        );
        assert!(note.contains("SILENT (never fatal)"), "{note}");
        assert!(note.contains("BEDLAM02..08.WAV"), "{note}");
        assert!(note.contains(&dir.0.display().to_string()), "{note}");
        // A partial supply names the same posture.
        let mut partial = SupplyReport {
            tracks: vec![None; CDDA_TRACK_COUNT],
        };
        partial.tracks[3] = Some(TrackSource {
            path: dir.0.join("BEDLAM05.WAV"),
            len: 4,
        });
        let note = supply_note(&partial, std::slice::from_ref(&dir.0));
        assert!(
            note.contains("1/7") && note.contains("missing tracks"),
            "{note}"
        );
    }

    #[test]
    fn wav_parser_accepts_the_corpus_shape_and_walks_odd_chunks() {
        // fmt + an odd-sized LIST chunk (one pad byte) + data: the
        // data offset must land past the pad.
        let pcm = [0i16, 1, -2, 3];
        let mut bytes = b"RIFF".to_vec();
        let body_len = 4 + (8 + 16) + (8 + 3 + 1) + (8 + pcm.len() * 2);
        bytes.extend_from_slice(&(body_len as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes()); // stereo
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&176_400u32.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"LIST");
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(b"xyz"); // odd size => pad byte next
        bytes.push(0); // the pad
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&((pcm.len() * 2) as u32).to_le_bytes());
        for s in pcm {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        let wav = parse_wav(&bytes).expect("parses");
        assert_eq!(
            wav,
            WavData {
                sample_rate: 44_100,
                channels: 2,
                data_offset: 12 + 24 + 12 + 8,
                data_len: 8,
            }
        );
    }

    #[test]
    fn wav_parser_fails_closed_on_every_malformed_shape() {
        assert_eq!(parse_wav(b"not a wav"), Err(WavError::NotRiff));
        assert_eq!(parse_wav(&[]), Err(WavError::NotRiff));
        // Truncated mid-chunk.
        let full = wav_bytes(1, 8_000, &[1, 2, 3]);
        assert_eq!(parse_wav(&full[..20]), Err(WavError::Truncated));
        assert_eq!(parse_wav(&full[..8]), Err(WavError::NotRiff));
        // Float format and 8-bit depth are refused.
        let mut float_wav = wav_bytes(1, 8_000, &[0]);
        float_wav[20] = 3; // fmt audio_format = 3 (IEEE float)
        assert_eq!(parse_wav(&float_wav), Err(WavError::NotPcm(3)));
        let mut eight_bit = wav_bytes(1, 8_000, &[0]);
        eight_bit[34] = 8; // fmt bits_per_sample = 8
        assert_eq!(parse_wav(&eight_bit), Err(WavError::Not16Bit(8)));
        // No data chunk.
        let mut no_data = b"RIFF".to_vec();
        no_data.extend_from_slice(&24u32.to_le_bytes());
        no_data.extend_from_slice(b"WAVE");
        no_data.extend_from_slice(b"fmt ");
        no_data.extend_from_slice(&16u32.to_le_bytes());
        no_data.extend_from_slice(&1u16.to_le_bytes());
        no_data.extend_from_slice(&1u16.to_le_bytes());
        no_data.extend_from_slice(&8_000u32.to_le_bytes());
        no_data.extend_from_slice(&2u32.to_le_bytes());
        no_data.extend_from_slice(&2u16.to_le_bytes());
        no_data.extend_from_slice(&16u16.to_le_bytes());
        assert_eq!(parse_wav(&no_data), Err(WavError::NoData));
    }

    #[test]
    fn adpcm_silence_is_exact_and_the_size_is_a_quarter() {
        // 100 mono silence samples: exactly 50 packed bytes, and the
        // round trip is EXACT silence (predictor stays at 0, code 0).
        let samples = [0i16; 100];
        let wav = WavData {
            sample_rate: 8_000,
            channels: 1,
            data_offset: 0,
            data_len: samples.len() * 2,
        };
        let mut file = Vec::new();
        for s in samples {
            file.extend_from_slice(&s.to_le_bytes());
        }
        let nibbles = encode_track(&wav, &file).unwrap();
        assert_eq!(nibbles.len(), 50, "exactly 4:1 vs the 16-bit source");
        assert!(nibbles.iter().all(|&b| b == 0), "silence encodes to 0");
        assert_eq!(decode_track(1, &nibbles, samples.len()), samples.to_vec());
    }

    #[test]
    fn adpcm_roundtrip_error_is_bounded_for_music_like_signals() {
        // A CONTINUOUS triangle wave (slope ±16 — fast enough to
        // move the step index): the lossy round trip must stay
        // within a small band (the codec's own step resolution),
        // and stereo interleaving must keep the channels' coders
        // separate. (A discontinuous jump cannot be slewd in one
        // sample by design — that is the codec, not a bug.)
        let mut samples: Vec<i16> = Vec::new();
        let mut value = 0i32;
        for _ in 0..4 {
            while value < 30_000 {
                samples.push(value as i16);
                value += 16;
            }
            while value > 0 {
                samples.push(value as i16);
                value -= 16;
            }
        }
        let wav = WavData {
            sample_rate: 44_100,
            channels: 2,
            data_offset: 0,
            data_len: samples.len() * 2,
        };
        let mut file = Vec::new();
        for s in &samples {
            file.extend_from_slice(&s.to_le_bytes());
        }
        let nibbles = encode_track(&wav, &file).unwrap();
        assert_eq!(nibbles.len(), samples.len().div_ceil(2));
        let back = decode_track(2, &nibbles, samples.len());
        let worst = samples
            .iter()
            .zip(&back)
            .map(|(want, got)| (i32::from(*want) - i32::from(*got)).abs())
            .max()
            .unwrap();
        assert!(worst <= 64, "max roundtrip error {worst} exceeds the band");
        // A held value settles to a tiny residual (the predictor
        // tracks a constant exactly apart from step quantization).
        let held = vec![12_345i16; 200];
        let wav = WavData {
            sample_rate: 44_100,
            channels: 1,
            data_offset: 0,
            data_len: held.len() * 2,
        };
        let mut file = Vec::new();
        for s in &held {
            file.extend_from_slice(&s.to_le_bytes());
        }
        let nibbles = encode_track(&wav, &file).unwrap();
        let back = decode_track(1, &nibbles, held.len());
        let tail = &back[100..];
        assert!(tail.iter().all(|s| (i32::from(*s) - 12_345).abs() <= 8));
    }

    #[test]
    fn fnv_identity_is_stable_and_chunking_equivalent() {
        let bytes: Vec<u8> = (0..=255u8).cycle().take(100_000).collect();
        let expected = fnv1a64(&bytes);
        // The streaming identify() must agree with the one-shot hash.
        let dir = scratch("fnv");
        let file = dir.0.join("blob.bin");
        fs::write(&file, &bytes).unwrap();
        let identity = identify(&file).unwrap();
        assert_eq!(
            identity,
            SourceIdentity {
                len: bytes.len() as u64,
                fnv: expected
            }
        );
        // Known-value pin (FNV-1a-64 of "a" and empty).
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn blob_header_round_trips_and_the_verdict_keys_on_identity() {
        let wav = WavData {
            sample_rate: 44_100,
            channels: 2,
            data_offset: 44,
            data_len: 8_000,
        };
        let identity = SourceIdentity {
            len: 8_044,
            fnv: 0x0123_4567_89ab_cdef,
        };
        let blob = build_blob(3, &wav, identity, &[0u8; 1_000]);
        let header = parse_blob_header(&blob).expect("parses");
        assert_eq!(header.track, 3);
        assert_eq!(header.sample_rate, 44_100);
        assert_eq!(header.channels, 2);
        assert_eq!(header.src_frames, 2_000); // 8000 bytes / (2*2)
        assert_eq!(header.identity, identity);
        assert_eq!(header.nibble_bytes, 1_000);
        assert_eq!(blob.len(), BLOB_HEADER_LEN + 1_000);
        // Matching identity => Fresh; anything else => Regenerate.
        assert_eq!(cache_verdict(Some(header), identity), CacheVerdict::Fresh);
        let mut moved = identity;
        moved.len += 1; // the source grew: stale
        assert_eq!(cache_verdict(Some(header), moved), CacheVerdict::Regenerate);
        let mut swapped = identity;
        swapped.fnv ^= 1; // content changed: stale
        assert_eq!(
            cache_verdict(Some(header), swapped),
            CacheVerdict::Regenerate
        );
        assert_eq!(cache_verdict(None, identity), CacheVerdict::Regenerate);
        // Corrupt blobs (bad magic, short, bad track) do not parse.
        assert_eq!(parse_blob_header(&blob[..8]), None);
        let mut bad = blob.clone();
        bad[0] = b'X';
        assert_eq!(parse_blob_header(&bad), None);
        let mut bad_track = blob.clone();
        bad_track[8] = 9; // beyond the seven tracks
        assert_eq!(parse_blob_header(&bad_track), None);
    }

    #[test]
    fn cache_generates_on_first_run_freshens_and_regenerates_on_mismatch() {
        let supply = scratch("cache-supply");
        let cache = scratch("cache-root");
        // One real (tiny) WAV the codec can transcode.
        let samples: Vec<i16> = (0..600).map(|i| (i * 137 % 4_000) as i16).collect();
        let mut bodies: [Option<Vec<u8>>; CDDA_TRACK_COUNT] = std::array::from_fn(|_| None);
        bodies[0] = Some(wav_bytes(2, 44_100, &samples));
        place_tracks(&supply.0, &bodies);
        let report = resolve_supply(std::slice::from_ref(&supply.0));
        assert_eq!(report.found_count(), CDDA_TRACK_COUNT);

        // First run: every RESOLVED track generates (the lossy cache
        // is created on first run, atomically, under music/).
        let outcomes = refresh_cache(&report, &cache.0);
        assert_eq!(outcomes, vec![CacheOutcome::Generated; CDDA_TRACK_COUNT]);
        let entry = cache_entry_path(&cache.0, 0);
        let blob = fs::read(&entry).unwrap();
        let header = parse_blob_header(&blob).expect("generated entry parses");
        let identity = identify(&report.tracks[0].as_ref().unwrap().path).unwrap();
        assert_eq!(header.identity, identity, "keyed by source identity");
        assert_eq!(header.sample_rate, 44_100);
        assert_eq!(header.channels, 2);
        assert_eq!(header.nibble_bytes as usize, blob.len() - BLOB_HEADER_LEN);
        // The cache never lands anywhere else: exactly seven files
        // under <root>/music and nothing at the root itself.
        assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 1);
        assert_eq!(fs::read_dir(cache.0.join("music")).unwrap().count(), 7);

        // Second run, nothing changed: all FRESH (no writes).
        let outcomes = refresh_cache(&report, &cache.0);
        assert_eq!(outcomes, vec![CacheOutcome::Fresh; CDDA_TRACK_COUNT]);

        // The source changes (append two bytes: identity moves, the
        // chunk walk still parses): the entry REGENERATES and the
        // new header carries the new identity.
        let mut changed = wav_bytes(2, 44_100, &samples);
        changed.extend_from_slice(b"\0\0");
        fs::write(supply.0.join("BEDLAM02.WAV"), changed).unwrap();
        let report2 = resolve_supply(std::slice::from_ref(&supply.0));
        let outcomes = refresh_cache(&report2, &cache.0);
        assert_eq!(outcomes[0], CacheOutcome::Generated);
        assert!(outcomes[1..].iter().all(|o| *o == CacheOutcome::Fresh));
        let blob = fs::read(&entry).unwrap();
        let identity2 = identify(&report2.tracks[0].as_ref().unwrap().path).unwrap();
        assert_ne!(identity, identity2);
        assert_eq!(parse_blob_header(&blob).unwrap().identity, identity2);
        // A corrupt entry (truncated) regenerates too.
        fs::write(&entry, &blob[..10]).unwrap();
        let outcomes = refresh_cache(&report2, &cache.0);
        assert_eq!(outcomes[0], CacheOutcome::Generated);
    }

    #[test]
    fn cache_skips_unparseable_sources_with_reason_and_never_fails() {
        let supply = scratch("cache-bad");
        let cache = scratch("cache-bad-root");
        let mut bodies: [Option<Vec<u8>>; CDDA_TRACK_COUNT] = std::array::from_fn(|_| None);
        bodies[0] = Some(b"garbage, not a wav at all".to_vec()); // name matches
        bodies[4] = Some(Vec::new()); // empty file
        place_tracks(&supply.0, &bodies);
        let report = resolve_supply(std::slice::from_ref(&supply.0));
        assert_eq!(report.found_count(), CDDA_TRACK_COUNT);
        let outcomes = refresh_cache(&report, &cache.0);
        // The five good tracks (indexes 1,2,3,5,6) still land; the
        // two malformed sources (garbage at index 0, empty at index
        // 4) skip with their reasons — never fatal.
        assert_eq!(outcomes[1], CacheOutcome::Generated);
        assert!(matches!(outcomes[0], CacheOutcome::Skipped(_)));
        let skipped: Vec<&String> = outcomes
            .iter()
            .filter_map(|o| match o {
                CacheOutcome::Skipped(reason) => Some(reason),
                _ => None,
            })
            .collect();
        assert_eq!(skipped.len(), 2, "{outcomes:?}");
        assert!(
            skipped.iter().any(|r| r.starts_with("track 2: not a RIFF")),
            "{skipped:?}"
        );
        assert!(
            skipped.iter().any(|r| r.starts_with("track 6: not a RIFF")),
            "{skipped:?}"
        );
        // The note carries the skip posture.
        let note = cache_note(&outcomes, &cache.0);
        assert!(note.contains("5 generated"), "{note}");
        assert!(note.contains("skipped 2"), "{note}");
        assert!(note.contains("never redistributed"), "{note}");
    }

    #[test]
    fn cache_note_counts_each_outcome_kind() {
        let root = Path::new("/never/used");
        let note = cache_note(&[CacheOutcome::Fresh, CacheOutcome::Fresh], root);
        assert_eq!(
            note,
            "cache: 0 generated, 2 fresh, in /never/used (lossy local \
             cache, user-owned, never redistributed)"
                .to_string()
        );
        let note = cache_note(&[CacheOutcome::Generated], root);
        assert!(note.contains("1 generated"), "{note}");
        assert!(!note.contains("skipped"), "{note}");
    }

    #[test]
    fn containment_guard_is_component_wise() {
        assert!(path_is_inside(Path::new("/a/b/c"), Path::new("/a")));
        assert!(path_is_inside(Path::new("/a/b"), Path::new("/a/b"))); // equal = inside
        assert!(!path_is_inside(Path::new("/ab"), Path::new("/a")));
        assert!(!path_is_inside(Path::new("/a"), Path::new("/a/b")));
        assert!(!path_is_inside(Path::new("relative"), Path::new("/abs")));
        // The exact P7 posture: a cache home under the install tree
        // (game-data) is refused; a sibling is not.
        let install = Path::new("/x/game-data/BEDLAM");
        assert!(path_is_inside(&install.join(".cache"), install));
        assert!(!path_is_inside(Path::new("/x/cache"), install));
    }

    #[test]
    fn git_worktree_guard_detects_a_worktree_root_above_the_cache() {
        let dir = scratch("git-guard");
        let worktree = dir.0.join("repo");
        fs::create_dir_all(worktree.join(".git")).unwrap();
        fs::create_dir_all(worktree.join("home").join(".cache")).unwrap();
        assert!(inside_git_worktree(
            &worktree.join("home").join(".cache").join("bedlam")
        ));
        assert!(inside_git_worktree(&worktree)); // the root itself
    }

    #[test]
    fn relative_install_dirs_compare_against_absolute_cache_homes() {
        // The startup guard must catch a cache home inside the
        // install tree even when the install dir arrives RELATIVE
        // (the binary's default `game-data/BEDLAM`) while the cache
        // home is absolute: both sides go through the best-effort
        // absolute-for-compare form.
        let rel_install = Path::new("game-data/BEDLAM");
        let rel_abs = absolute_for_compare(rel_install);
        assert!(rel_abs.is_absolute(), "{rel_abs:?}");
        assert!(rel_abs.ends_with("game-data/BEDLAM"), "{rel_abs:?}");
        // The lexical fallback branch (nonexistent path, absolute in
        // is already absolute; relative nonexistent is joined).
        let ghost = absolute_for_compare(Path::new("no/such/dir"));
        assert!(ghost.is_absolute(), "{ghost:?}");
        assert_eq!(
            absolute_for_compare(Path::new("/abs/x")),
            PathBuf::from("/abs/x")
        );
        // The containment the guard needs: an absolute cache home
        // under the absolutized install dir.
        assert!(path_is_inside(
            &rel_abs.join(".cache").join("bedlam"),
            &rel_abs
        ));
    }

    #[test]
    fn search_roots_are_ordered_and_pure() {
        let install = Path::new("/install/BEDLAM");
        let explicit = Path::new("/explicit/music");
        let roots = search_roots(Some(explicit), install);
        assert_eq!(roots.first(), Some(&explicit.to_path_buf()));
        assert_eq!(roots.last(), Some(&install.to_path_buf()));
        // No env overrides leak into a PURE probe under the default
        // test environment: the middle root is the user music dir
        // (XDG_DATA_HOME or HOME-derived) — exactly one of them.
        assert_eq!(roots.len(), 3);
    }

    #[test]
    fn cdda_options_default_is_enabled_with_no_override() {
        let opts = CddaOptions::default();
        assert_eq!(opts.search_dir, None);
        assert_eq!(opts.cache, MusicCachePolicy::Enabled);
        // The policy is the plan's posture: ON by default.
        assert_eq!(MusicCachePolicy::default(), MusicCachePolicy::Enabled);
    }
}
