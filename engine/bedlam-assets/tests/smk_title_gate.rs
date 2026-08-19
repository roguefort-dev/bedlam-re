//! TITLE.SMK decode gate (D30). Skips when the corpus is absent (CI);
//! when present it pins the header facts and proves two independent full
//! decode passes are byte-identical: SHA-256 chains over pixels+palette
//! per frame and over every decoded audio packet, plus per-frame packet
//! size logs, counts, and byte totals. Fingerprints go into .state run
//! notes only - decoded media never enters git. game-data access here is
//! read-only and the test run is bracketed by MANIFEST.sha256 checks at
//! the shell level.

use std::path::PathBuf;

use bedlam_assets::smk::{SmkAudioCodec, SmkFrameStatus, SmkStream, SmkYScale};
use sha2::{Digest, Sha256};

fn title() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/GAMEGFX/TITLE.SMK")
}

fn hex(d: &[u8]) -> String {
    d.iter().map(|b| format!("{:02x}", b)).collect()
}

#[test]
fn title_smk_decode_gate() {
    let path = title();
    if !path.exists() {
        eprintln!("skipping: {} not present", path.display());
        return;
    }
    let data = std::fs::read(&path).expect("read TITLE.SMK");

    // Header facts pinned first (cheap open, no decode).
    {
        let info = SmkStream::open(&data)
            .expect("open TITLE.SMK")
            .info()
            .clone();
        assert_eq!((info.width, info.height), (640, 320), "raster size");
        assert_eq!(info.frames, 1227, "declared frame count");
        assert_eq!(info.us_per_frame, 66_660, "frame interval (15 fps)");
        assert!(!info.ring_frame, "not a ring stream");
        assert_eq!(info.y_scale, SmkYScale::None);
        let t0 = info.audio[0].expect("audio track 0 present");
        assert_eq!(t0.codec, SmkAudioCodec::Dpcm);
        assert_eq!((t0.channels, t0.bitdepth, t0.rate_hz), (1, 8, 11_025));
        for t in 1..7 {
            assert!(info.audio[t].is_none(), "track {} absent", t);
        }
    }

    let run = || -> (String, String, u64, usize, usize, Vec<(u32, usize)>) {
        let mut s = SmkStream::open(&data).expect("open TITLE.SMK");
        let mut video = Sha256::new();
        let mut audio = Sha256::new();
        let mut frames = 0u64;
        let mut packets = 0usize;
        let mut audio_bytes = 0usize;
        let mut packet_log: Vec<(u32, usize)> = Vec::new();
        let mut status = s.first_frame().expect("first frame decodes");
        loop {
            video.update(s.pixels());
            for c in s.palette() {
                video.update(c);
            }
            let mut n = 0usize;
            if let Some(p) = s.audio_packet(0) {
                if !p.is_empty() {
                    packets += 1;
                    audio_bytes += p.len();
                    audio.update(p);
                    n = p.len();
                }
            }
            packet_log.push((s.frame_index(), n));
            frames += 1;
            match status {
                SmkFrameStatus::Last | SmkFrameStatus::Done => break,
                SmkFrameStatus::More => status = s.next_frame().expect("frame decodes"),
            }
        }
        (
            hex(&video.finalize()),
            hex(&audio.finalize()),
            frames,
            packets,
            audio_bytes,
            packet_log,
        )
    };

    let r1 = run();
    let r2 = run();
    assert_eq!(r1, r2, "two full decode passes must be identical");
    assert_eq!(r1.2, 1227, "decoded frame count");
    assert!(r1.3 > 0, "audio packets present");
    assert!(r1.4 > 0, "audio bytes present");
    eprintln!(
        "TITLE.SMK gate: frames={} video_sha256={} audio_sha256={} packets={} audio_bytes={}",
        r1.2, r1.0, r1.1, r1.3, r1.4
    );
}
