
#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic(ms: i32) -> Vec<u8> {
        let mut d = vec![0u8; SMK_HEADER_MIN];
        d[0..4].copy_from_slice(b"SMK2");
        d[4..8].copy_from_slice(&320u32.to_le_bytes());
        d[8..12].copy_from_slice(&200u32.to_le_bytes());
        d[12..16].copy_from_slice(&100u32.to_le_bytes());
        d[16..20].copy_from_slice(&(ms as u32).to_le_bytes());
        d[20..24].copy_from_slice(&0xABCDu32.to_le_bytes());
        for i in 0..7 {
            d[24 + i * 4..28 + i * 4].copy_from_slice(&((i as u32) * 100).to_le_bytes());
            d[72 + i * 4..76 + i * 4].copy_from_slice(&((i as u32) * 7).to_le_bytes());
        }
        // byte 52 = tree chunk size; 56..72 = the four tree maxima
        d[52..56].copy_from_slice(&0x5555u32.to_le_bytes());
        for (i, off) in [56usize, 60, 64, 68].iter().enumerate() {
            d[*off..*off + 4].copy_from_slice(&((i as u32) * 11).to_le_bytes());
        }
        d
    }

    #[test]
    fn parse_header_fields() {
        let d = synthetic(40);
        let h = parse_smk_header(&d).unwrap();
        assert_eq!(h.magic, "SMK2");
        assert_eq!((h.width, h.height, h.frames), (320, 200, 100));
        assert_eq!(h.ms_per_frame_raw, 40);
        assert_eq!(h.fps_desc(), "25 fps");
        assert_eq!(h.us_per_frame(), 40_000);
        assert_eq!(h.flags, 0xABCD);
        assert_eq!(h.audio_sizes, vec![0, 100, 200, 300, 400, 500, 600]);
        assert_eq!(h.tree_sizes, vec![0, 11, 22, 33]);
        assert_eq!(h.audio_rates, vec![0, 7, 14, 21, 28, 35, 42]);
    }

    #[test]
    fn negative_ms_is_us_encoding() {
        let h = parse_smk_header(&synthetic(-1)).unwrap();
        assert_eq!(h.fps_desc(), "100000 fps (us-per-frame encoding: 10us)");
        assert_eq!(h.us_per_frame(), 10);
        let h = parse_smk_header(&synthetic(-417)).unwrap();
        // us = 4170 -> 1000000/4170 = 239
        assert_eq!(h.fps_desc(), "239 fps (us-per-frame encoding: 4170us)");
        assert_eq!(h.us_per_frame(), 4_170);
        // i32::MIN must not panic
        let h = parse_smk_header(&synthetic(i32::MIN)).unwrap();
        assert!(h.fps_desc().contains("us-per-frame encoding"));
        assert_eq!(h.us_per_frame(), 21_474_836_480);
    }

    #[test]
    fn zero_ms_uses_backend_default_interval() {
        let h = parse_smk_header(&synthetic(0)).unwrap();
        assert_eq!(h.us_per_frame(), 100_000);
    }

    #[test]
    fn rejects_short_and_bad_magic() {
        assert_eq!(parse_smk_header(b"SMK2"), Err(AssetsError::BadMagic));
        let mut d = synthetic(40);
        d[0..4].copy_from_slice(b"XVID");
        assert_eq!(parse_smk_header(&d), Err(AssetsError::BadMagic));
    }

    #[test]
    fn smk4_magic_accepted() {
        let mut d = synthetic(40);
        d[0..4].copy_from_slice(b"SMK4");
        assert_eq!(parse_smk_header(&d).unwrap().magic, "SMK4");
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 31337u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 4, 103, 104, 105, 4096] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_smk_header(&d);
            let _ = SmkStream::open(&d);
        }
    }

    // ---- synthetic full-stream builders ----

    fn synth_header(
        w: u32,
        h: u32,
        frames: u32,
        ms: i32,
        flags: u32,
        sizes: [u32; 7],
        rates: [u32; 7],
    ) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"SMK2");
        d.extend_from_slice(&w.to_le_bytes());
        d.extend_from_slice(&h.to_le_bytes());
        d.extend_from_slice(&frames.to_le_bytes());
        d.extend_from_slice(&(ms as u32).to_le_bytes());
        d.extend_from_slice(&flags.to_le_bytes());
        for s in sizes {
            d.extend_from_slice(&s.to_le_bytes());
        }
        d.extend_from_slice(&1u32.to_le_bytes()); // tree chunk: 1 zero byte
        for _ in 0..4 {
            d.extend_from_slice(&0u32.to_le_bytes()); // tree maxima: unused
        }
        for r in rates {
            d.extend_from_slice(&r.to_le_bytes());
        }
        d.extend_from_slice(&0u32.to_le_bytes()); // dummy
        d
    }

    /// 4x4, 2 frames, 25 fps, one raw-PCM 8-bit mono 11025 Hz audio track.
    /// Trees: four absent Huff16 trees (8 zero bits). Frame 0 carries a
    /// palette (entry 0 = PALMAP[1,2,3], rest zero) + audio + trailing
    /// video pad; frame 1 carries audio + pad (video re-runs, stays zero).
    fn synth_stream() -> Vec<u8> {
        let mut d = synth_header(
            4,
            4,
            2,
            40,
            0,
            [16, 0, 0, 0, 0, 0, 0],
            [0x4000_0000 | 11_025, 0, 0, 0, 0, 0, 0],
        );
        d.extend_from_slice(&17u32.to_le_bytes()); // frame0: 16B + keyframe bit
        d.extend_from_slice(&8u32.to_le_bytes()); // frame1: 8B
        d.push(0x03); // frame0 type: palette + audio track 0
        d.push(0x02); // frame1 type: audio track 0
        d.push(0x00); // tree chunk: four absent trees
        // frame 0 chunk (16B): palette subchunk (2*4=8), audio (4+2), pad
        d.extend_from_slice(&[0x02, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0x00, 0x00]);
        d.extend_from_slice(&[0x06, 0x00, 0x00, 0x00, 0xAA, 0x55]);
        d.extend_from_slice(&[0x00, 0x00]);
        // frame 1 chunk (8B): audio (4+3) + pad
        d.extend_from_slice(&[0x07, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33, 0x00]);
        d
    }

    /// Same as [synth_stream] but ring-flagged with a third (wrap) frame.
    fn synth_ring_stream() -> Vec<u8> {
        let mut d = synth_header(
            4,
            4,
            2,
            40,
            0x01,
            [16, 0, 0, 0, 0, 0, 0],
            [0x4000_0000 | 11_025, 0, 0, 0, 0, 0, 0],
        );
        d.extend_from_slice(&17u32.to_le_bytes());
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&8u32.to_le_bytes()); // ring wrap frame
        d.push(0x03);
        d.push(0x02);
        d.push(0x02);
        d.push(0x00);
        d.extend_from_slice(&[0x02, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0x00, 0x00]);
        d.extend_from_slice(&[0x06, 0x00, 0x00, 0x00, 0xAA, 0x55]);
        d.extend_from_slice(&[0x00, 0x00]);
        d.extend_from_slice(&[0x07, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33, 0x00]);
        d.extend_from_slice(&[0x06, 0x00, 0x00, 0x00, 0x44, 0x55, 0x00, 0x00]);
        d
    }

    #[test]
    fn decode_synthetic_content() {
        let data = synth_stream();
        let mut s = SmkStream::open(&data).expect("open synthetic");
        let m = s.info().clone();
        assert_eq!((m.width, m.height), (4, 4));
        assert_eq!(m.frames, 2);
        assert_eq!(m.us_per_frame, 40_000);
        assert!(!m.ring_frame);
        assert_eq!(m.y_scale, SmkYScale::None);
        assert_eq!(
            m.audio[0],
            Some(SmkAudioTrackMeta {
                codec: SmkAudioCodec::Pcm,
                channels: 1,
                bitdepth: 8,
                rate_hz: 11_025,
            })
        );
        for t in 1..7 {
            assert!(m.audio[t].is_none(), "track {t} absent");
        }
        assert_eq!(s.first_frame().unwrap(), SmkFrameStatus::More);
        assert_eq!(s.frame_index(), 0);
        assert_eq!(s.pixels(), vec![0u8; 16]);
        assert_eq!(s.palette()[0], [0x04, 0x08, 0x0C]);
        assert_eq!(s.palette()[1], [0, 0, 0]);
        assert_eq!(s.palette().len(), 256);
        assert_eq!(s.audio_packet(0), Some(&[0xAAu8, 0x55][..]));
        assert_eq!(s.next_frame().unwrap(), SmkFrameStatus::Last);
        assert_eq!(s.frame_index(), 1);
        assert_eq!(s.audio_packet(0), Some(&[0x11u8, 0x22, 0x33][..]));
        assert_eq!(s.next_frame().unwrap(), SmkFrameStatus::Done);
        assert_eq!(s.next_frame().unwrap(), SmkFrameStatus::Done);
        assert_eq!(s.audio_packet(7), None);
    }

    #[test]
    fn decode_twice_is_identical() {
        let data = synth_stream();
        let run = || {
            let mut s = SmkStream::open(&data).unwrap();
            let mut acc = Vec::new();
            let mut status = s.first_frame().unwrap();
            loop {
                acc.extend_from_slice(&s.frame_index().to_le_bytes());
                acc.extend_from_slice(s.pixels());
                for e in s.palette() {
                    acc.extend_from_slice(e);
                }
                if let Some(p) = s.audio_packet(0) {
                    acc.extend_from_slice(p);
                }
                match status {
                    SmkFrameStatus::Last | SmkFrameStatus::Done => break,
                    SmkFrameStatus::More => status = s.next_frame().unwrap(),
                }
            }
            acc
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn ring_stream_stops_at_declared_frames() {
        let data = synth_ring_stream();
        let mut s = SmkStream::open(&data).unwrap();
        assert!(s.info().ring_frame);
        let mut frames = 0u32;
        let mut status = s.first_frame().unwrap();
        loop {
            assert_eq!(s.frame_index(), frames);
            frames += 1;
            match status {
                SmkFrameStatus::Last | SmkFrameStatus::Done => break,
                SmkFrameStatus::More => status = s.next_frame().unwrap(),
            }
        }
        assert_eq!(frames, 2, "declared frame count, not the ring wrap");
    }

    #[test]
    fn single_frame_stream_is_last_immediately() {
        let mut d = synth_header(4, 4, 1, 40, 0, [0; 7], [0; 7]);
        d.extend_from_slice(&0u32.to_le_bytes()); // frame0 size 0
        d.push(0x00); // frame0 type
        d.push(0x00); // tree chunk
        let mut s = SmkStream::open(&d).unwrap();
        assert_eq!(s.first_frame().unwrap(), SmkFrameStatus::Last);
        assert_eq!(s.next_frame().unwrap(), SmkFrameStatus::Done);
    }

    #[test]
    fn truncation_sweep_stable_and_no_panic() {
        let data = synth_stream();
        let drain = |d: &[u8]| -> String {
            let mut s = match SmkStream::open(d) {
                Err(e) => return format!("open-err/{e}"),
                Ok(s) => s,
            };
            let mut n = 0u32;
            let status = match s.first_frame() {
                Err(e) => return format!("first-err/{e}"),
                Ok(st) => st,
            };
            n += 1;
            let mut status = status;
            loop {
                match status {
                    SmkFrameStatus::Last | SmkFrameStatus::Done => return format!("ok/{n}"),
                    SmkFrameStatus::More => match s.next_frame() {
                        Err(e) => return format!("next-err/{n}/{e}"),
                        Ok(st) => {
                            if st != SmkFrameStatus::Done {
                                n += 1;
                            }
                            status = st;
                        }
                    },
                }
            }
        };
        let pass1: Vec<String> = (0..=data.len()).map(|l| drain(&data[..l])).collect();
        let pass2: Vec<String> = (0..=data.len()).map(|l| drain(&data[..l])).collect();
        assert_eq!(pass1, pass2);
        assert_eq!(pass1.last().unwrap(), "ok/2");
    }

    #[test]
    fn garbage_never_panics() {
        let mut s = 987_654_321u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 1, 10, 104, 105, 141, 300] {
            let mut d = vec![b"S", b"M", b"K", b"2"];
            d.extend((0..len).map(|_| next()));
            if let Ok(mut st) = SmkStream::open(&d) {
                let _ = st.first_frame();
                let _ = st.next_frame();
            }
        }
    }

    #[test]
    fn hostile_headers_rejected_without_allocation() {
        // dimension bomb: 0xFFFFFFFF square would request ~1.8e19 bytes
        let d = synth_header(u32::MAX, u32::MAX, 1, 40, 0, [0; 7], [0; 7]);
        assert!(matches!(
            SmkStream::open(&d),
            Err(AssetsError::SmkDecode("frame dimensions out of bounds"))
        ));
        // zero frames would index an empty table on first decode
        let d = synth_header(4, 4, 0, 40, 0, [0; 7], [0; 7]);
        assert!(matches!(
            SmkStream::open(&d),
            Err(AssetsError::SmkDecode("zero frame count"))
        ));
        // existing track with a sub-4B buffer
        let d = synth_header(
            4,
            4,
            1,
            40,
            0,
            [2, 0, 0, 0, 0, 0, 0],
            [0x4000_0000 | 11_025, 0, 0, 0, 0, 0, 0],
        );
        assert!(matches!(
            SmkStream::open(&d),
            Err(AssetsError::SmkDecode("audio track buffer below 4 bytes"))
        ));
        // Bink-flagged audio
        let d = synth_header(
            4,
            4,
            1,
            40,
            0,
            [16, 0, 0, 0, 0, 0, 0],
            [0x4000_0000 | 0x0800_0000, 0, 0, 0, 0, 0, 0],
        );
        assert!(matches!(
            SmkStream::open(&d),
            Err(AssetsError::SmkDecode("Bink-flagged audio track"))
        ));
        // audio buffer bigger than the whole input
        let d = synth_header(
            4,
            4,
            1,
            40,
            0,
            [0x0100_0000, 0, 0, 0, 0, 0, 0],
            [0x4000_0000 | 11_025, 0, 0, 0, 0, 0, 0],
        );
        assert!(matches!(
            SmkStream::open(&d),
            Err(AssetsError::SmkDecode("audio track buffer exceeds input"))
        ));
    }

    #[test]
    fn allocation_bombs_in_frame_tables_rejected() {
        // declared chunk sizes far beyond the input
        let mut d = synth_header(4, 4, 2, 40, 0, [0; 7], [0; 7]);
        d.extend_from_slice(&0xFFFF_FFFCu32.to_le_bytes());
        d.extend_from_slice(&0xFFFF_FFFCu32.to_le_bytes());
        d.push(0x00);
        d.push(0x00);
        d.push(0x00);
        assert!(matches!(SmkStream::open(&d), Err(AssetsError::SmkTruncated)));
        // frame tables themselves truncated (declared frames without table bytes)
        let d = synth_header(4, 4, 100, 40, 0, [0; 7], [0; 7]);
        assert!(matches!(SmkStream::open(&d), Err(AssetsError::SmkTruncated)));
        // tree alloc size larger than the whole tree chunk
        let mut d = synth_header(4, 4, 1, 40, 0, [0; 7], [0; 7]);
        d.extend_from_slice(&0u32.to_le_bytes()); // frame0 size 0
        d.push(0x00); // frame0 type
        d.push(0x00); // tree chunk: 1 byte
        d[56..60].copy_from_slice(&1_000u32.to_le_bytes()); // tree 0 claims 1000B
        assert!(matches!(
            SmkStream::open(&d),
            Err(AssetsError::SmkDecode("tree size exceeds chunk"))
        ));
    }

    #[test]
    fn lying_dpcm_unpack_size_returns_error_not_panic() {
        // DPCM track claiming a 16 MiB unpack size into a 16-byte buffer:
        // whatever the backend does (bounds error, or a caught panic) must
        // surface as Err, never as a propagated panic.
        let mut d = synth_header(
            4,
            4,
            1,
            40,
            0,
            [16, 0, 0, 0, 0, 0, 0],
            [0xC000_0000 | 11_025, 0, 0, 0, 0, 0, 0],
        );
        d.extend_from_slice(&21u32.to_le_bytes()); // frame0: 20B + keyframe bit
        d.push(0x03); // palette + audio track 0
        d.push(0x00); // tree chunk: four absent trees
        d.extend_from_slice(&[0x02, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0x00, 0x00]); // palette
        d.extend_from_slice(&[0x0C, 0x00, 0x00, 0x00]); // audio subchunk size 12
        // unpack size 0x00FFFFFF, initial bit 1, mono/8-bit flags 0, then
        // absent Huff8 trees + terminators + zero payload
        d.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0x00, 0x01, 0x00, 0x00, 0x00]);
        let mut s = SmkStream::open(&d).expect("header-level guards pass");
        match s.first_frame() {
            Ok(_) => panic!("lying DPCM unpack must not decode"),
            Err(e) => {
                assert!(
                    matches!(e, AssetsError::SmkInvalid | AssetsError::SmkTruncated),
                    "unexpected error: {e}"
                );
            }
        }
    }
}
