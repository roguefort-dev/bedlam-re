//! Audio determinism gates (DESIGN-AUDIO sec 11): the mix stream is
//! byte-identical for the same event script, waves and knob sequence, under
//! ANY host buffer chunking. This is the audio analog of the bedlam-core
//! same-script same-hash gate (D17: audio is never hashed; byte-identity is
//! the stronger, ordinary-CI-checkable property).

use bedlam_audio::{
    AudioError, Mixer, MusicCommand, MusicScript, MAX_VOICES, SAMPLE_RATE, SUB_VOICES_PER_INST,
};

/// Deterministic pseudo-wave (no RNG needed): a 37-step triangular-ish ramp
/// wrapped, as centered 8-bit unsigned PCM.
fn ramp_wave(n: usize) -> Vec<u8> {
    (0..n).map(|i| 128 + ((i % 37) as u32 * 2) as u8).collect()
}

/// The i16 the mixer converts a u8 sample into.
fn centered(b: u8) -> i16 {
    ((b as i16) - 128) * 256
}

fn build_script() -> MusicScript {
    let mut s = MusicScript::new();
    s.push(
        0,
        MusicCommand::NoteOn {
            instrument: 0,
            ratio: 0x10000,
            volume: 42,
        },
    )
    .unwrap();
    s.push(
        5,
        MusicCommand::NoteOn {
            instrument: 1,
            ratio: 0x0C000,
            volume: 30,
        },
    )
    .unwrap();
    s.push(
        41,
        MusicCommand::NoteOn {
            instrument: 0,
            ratio: 0x20000,
            volume: 20,
        },
    )
    .unwrap();
    s.push(50, MusicCommand::NoteOff { instrument: 0 }).unwrap();
    s.push(
        99,
        MusicCommand::NoteOn {
            instrument: 1,
            ratio: 0x10000,
            volume: 9,
        },
    )
    .unwrap();
    s
}

fn fresh_mixer() -> Mixer {
    let mut m = Mixer::new();
    m.load_wave(0, &ramp_wave(600)).unwrap();
    m.load_wave(1, &ramp_wave(4000)).unwrap();
    m.set_master_volume(127);
    m.load_script(build_script());
    m
}

const TOTAL_FRAMES: usize = SAMPLE_RATE as usize; // one second of mix

fn render_chunked(m: &mut Mixer, chunks: &[usize]) -> Vec<i16> {
    let mut out = Vec::new();
    let mut done = 0;
    let mut ci = 0;
    while done < TOTAL_FRAMES {
        let n = chunks[ci % chunks.len()].min(TOTAL_FRAMES - done);
        let mut buf = vec![0i16; 2 * n];
        m.render(&mut buf).unwrap();
        out.extend_from_slice(&buf);
        done += n;
        ci += 1;
    }
    out
}

#[test]
fn same_script_same_bytes() {
    let a = render_chunked(&mut fresh_mixer(), &[4096]);
    let b = render_chunked(&mut fresh_mixer(), &[4096]);
    assert_eq!(a, b);
    assert!(a.iter().any(|&s| s != 0), "stream is not silence");
}

#[test]
fn chunking_is_invariant() {
    // THE gate: host frame rate only changes how the same byte stream is
    // sliced, never a sample value (DESIGN-AUDIO sec 4-5).
    let whole = render_chunked(&mut fresh_mixer(), &[TOTAL_FRAMES]);
    for chunks in [
        vec![1usize],
        vec![7],
        vec![64],
        vec![1, 7, 64, 512, 4096],
        vec![3, 5, 11, 13],
    ] {
        let got = render_chunked(&mut fresh_mixer(), &chunks);
        assert_eq!(whole, got, "chunk pattern {:?}", chunks);
    }
}

#[test]
fn unity_pitch_is_passthrough() {
    // master 127 x vol 48 = unity gain, pan center: output equals the
    // converted source samples exactly, sample for sample.
    let mut m = Mixer::new();
    let pcm = ramp_wave(32);
    m.load_wave(0, &pcm).unwrap();
    m.set_master_volume(127);
    let mut buf = vec![0i16; 80];
    let r = m.note_on(0, 0x10000, 48).unwrap();
    assert_eq!((r.instrument, r.sub), (0, 0));
    m.render(&mut buf).unwrap();
    for f in 0..32 {
        assert_eq!(buf[2 * f], centered(pcm[f]), "frame {}", f);
        assert_eq!(buf[2 * f + 1], centered(pcm[f]), "frame {}", f);
    }
    // one-shot: exact silence after the wave ends, voice freed immediately
    for f in 32..40 {
        assert_eq!(buf[2 * f], 0, "tail frame {}", f);
        assert_eq!(buf[2 * f + 1], 0, "tail frame {}", f);
    }
    assert!(!m.voice_playing(0, 0), "one-shot freed the voice");
}

#[test]
fn doubled_pitch_skips_source_samples() {
    let mut m = Mixer::new();
    let pcm = ramp_wave(64);
    m.load_wave(0, &pcm).unwrap();
    m.set_master_volume(127);
    m.note_on(0, 0x20000, 48).unwrap();
    let mut buf = vec![0i16; 40];
    m.render(&mut buf).unwrap();
    for f in 0..20 {
        assert_eq!(buf[2 * f], centered(pcm[2 * f]), "frame {}", f);
    }
}

#[test]
fn note_off_releases_base_only() {
    // The sec 6 quirk: note_off stops sub 0; sub 1 rings out.
    let mut m = Mixer::new();
    m.load_wave(0, &ramp_wave(20000)).unwrap();
    m.set_master_volume(127);
    let a = m.note_on(0, 0x10000, 48).unwrap();
    assert_eq!(a.sub, 0);
    let mut tmp = vec![0i16; 2 * 110];
    m.render(&mut tmp).unwrap();
    let b = m.note_on(0, 0x10000, 48).unwrap();
    assert_eq!(b.sub, 1, "second concurrent note takes sub 1");
    assert!(m.note_off(0), "base was sounding");
    assert!(!m.voice_playing(0, 0), "base stopped");
    assert!(m.voice_playing(0, 1), "sub 1 rings out");
    // ...and the ringing is audible past the note_off point
    let mut buf = vec![0i16; 2 * 200];
    m.render(&mut buf).unwrap();
    assert!(buf.iter().any(|&s| s != 0), "sub 1 still contributes");
}

#[test]
fn drop_when_sub_voices_full() {
    let mut m = Mixer::new();
    m.load_wave(0, &ramp_wave(20000)).unwrap();
    m.set_master_volume(127);
    for s in 0..SUB_VOICES_PER_INST {
        let r = m.note_on(0, 0x10000, 48).unwrap();
        assert_eq!(r.sub, s);
    }
    assert!(m.note_on(0, 0x10000, 48).is_none(), "5th note dropped");
}

#[test]
fn drop_when_pool_full() {
    let mut m = Mixer::new();
    for inst in 0..(MAX_VOICES / SUB_VOICES_PER_INST) as u16 {
        m.load_wave(inst, &ramp_wave(20000)).unwrap();
    }
    m.set_master_volume(127);
    for inst in 0..(MAX_VOICES / SUB_VOICES_PER_INST) as u16 {
        for _ in 0..SUB_VOICES_PER_INST {
            assert!(m.note_on(inst, 0x10000, 48).is_some());
        }
    }
    assert!(m.note_on(0, 0x10000, 48).is_none(), "pool exhausted");
}

#[test]
fn one_shot_frees_voice_for_reuse() {
    let mut m = Mixer::new();
    m.load_wave(0, &ramp_wave(50)).unwrap();
    m.set_master_volume(127);
    m.note_on(0, 0x10000, 48).unwrap();
    let mut buf = vec![0i16; 2 * 100];
    m.render(&mut buf).unwrap();
    assert!(!m.voice_playing(0, 0));
    let again = m.note_on(0, 0x10000, 48).unwrap();
    assert_eq!(again.sub, 0, "base slot reused");
}

#[test]
fn saturation_clamps_at_the_bus() {
    // Two max-amplitude voices: 2 x 32512 must clamp to 32767, not wrap.
    let mut m = Mixer::new();
    m.load_wave(0, &[255u8; 300]).unwrap();
    m.load_wave(1, &[255u8; 300]).unwrap();
    m.set_master_volume(127);
    m.note_on(0, 0x10000, 48).unwrap();
    m.note_on(1, 0x10000, 48).unwrap();
    let mut buf = vec![0i16; 2 * 100];
    m.render(&mut buf).unwrap();
    assert!(buf.iter().all(|&s| s == 32767), "clamped, not wrapped");
}

#[test]
fn pan_hard_right_leaves_a_quiet_left() {
    let mut m = Mixer::new();
    m.load_wave(0, &[255u8; 64]).unwrap();
    m.set_master_volume(127);
    m.note_on_pan(0, 0x10000, 48, 63).unwrap();
    let mut buf = vec![0i16; 8];
    m.render(&mut buf).unwrap();
    assert_eq!(buf[1], 32512, "right at unity volume gain");
    assert_eq!(buf[0], (32512i32 * 4 / 256) as i16, "left at the pan floor");
}

#[test]
fn ratio_zero_mutes_but_occupies() {
    let mut m = Mixer::new();
    m.load_wave(0, &ramp_wave(100)).unwrap();
    m.set_master_volume(127);
    let r = m.note_on(0, 0, 48).unwrap();
    assert_eq!(r.sub, 0);
    let mut buf = vec![1i16; 64];
    m.render(&mut buf).unwrap();
    assert!(buf.iter().all(|&s| s == 0), "muted = silence (Q3 policy)");
    assert!(m.voice_playing(0, 0), "slot still occupied");
    assert!(m.note_on(0, 0, 48).is_some(), "sub 1 available");
}

#[test]
fn missing_wave_drops_the_note() {
    let mut m = Mixer::new();
    m.set_master_volume(127);
    assert!(m.note_on(9, 0x10000, 48).is_none());
    let mut buf = vec![3i16; 16];
    m.render(&mut buf).unwrap();
    assert!(buf.iter().all(|&s| s == 0));
}

#[test]
fn script_events_fire_on_the_tick_grid() {
    // tick 4 = sample 441 exactly: the first non-silent frame after an
    // empty prefix is frame 441, not a host-chunk boundary.
    let mut m = Mixer::new();
    m.load_wave(0, &[255u8; 20000]).unwrap();
    m.set_master_volume(127);
    let mut s = MusicScript::new();
    s.push(
        4,
        MusicCommand::NoteOn {
            instrument: 0,
            ratio: 0x10000,
            volume: 48,
        },
    )
    .unwrap();
    m.load_script(s);
    let mut buf = vec![0i16; 2 * 445];
    m.render(&mut buf).unwrap();
    assert!(buf[..2 * 441].iter().all(|&x| x == 0), "pre-tick silence");
    assert_eq!(buf[2 * 441], 32512, "event lands on the exact sample");
    // chunking that boundary differently must not move it
    let mut m2 = Mixer::new();
    m2.load_wave(0, &[255u8; 20000]).unwrap();
    m2.set_master_volume(127);
    let mut s2 = MusicScript::new();
    s2.push(
        4,
        MusicCommand::NoteOn {
            instrument: 0,
            ratio: 0x10000,
            volume: 48,
        },
    )
    .unwrap();
    m2.load_script(s2);
    let mut out = Vec::new();
    for _ in 0..89 {
        let mut chunk = vec![0i16; 10]; // 5 frames per chunk, boundary inside
        m2.render(&mut chunk).unwrap();
        out.extend_from_slice(&chunk);
    }
    assert_eq!(&out[..], &buf[..890]);
}

#[test]
fn odd_buffer_length_is_an_error() {
    let mut m = Mixer::new();
    let mut bad = [0i16; 3];
    assert!(matches!(
        m.render(&mut bad),
        Err(AudioError::OddBufferLength { .. })
    ));
}

#[test]
fn script_attach_mid_stream_anchors_at_the_cursor() {
    // DESIGN-GAME sec 5: a script swapped in mid-stream (every scene
    // change) must play from its top, relative to the ATTACH frame -
    // not against the absolute cursor, which would fire the entire
    // past at once and mute the track.
    let mut m = Mixer::new();
    m.load_wave(0, &[255u8; 20000]).unwrap();
    m.set_master_volume(127);
    let mut pre = vec![0i16; 2 * 500];
    m.render(&mut pre).unwrap();
    assert!(pre.iter().all(|&x| x == 0), "no script yet: silence");
    let mut s = MusicScript::new();
    s.push(
        4,
        MusicCommand::NoteOn {
            instrument: 0,
            ratio: 0x10000,
            volume: 48,
        },
    )
    .unwrap();
    m.load_script(s); // tick 4 = exactly 441 frames after the attach
    let mut buf = vec![0i16; 2 * 900];
    m.render(&mut buf).unwrap();
    assert!(
        buf[..2 * 441].iter().all(|&x| x == 0),
        "attach-relative lead-in silence"
    );
    assert_eq!(buf[2 * 441], 32512, "event lands 441 frames after attach");
}
