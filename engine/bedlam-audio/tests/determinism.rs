//! Determinism + semantics gates for the hermetic mixer (DESIGN-AUDIO
//! secs 4, 8, 11): byte-identical streams for identical event scripts
//! under any host chunking, plus the RE-anchored voice semantics -
//! note-off releases the BASE sub-voice only, drop-when-busy, one-shot
//! recycling, saturation rails, pan/volume audibility, and the exact
//! Q16 tick grid.

use bedlam_audio::{
    MusicCommand, MusicScript, Mixer, MAX_VOICES, SAMPLE_RATE, SUB_VOICES_PER_INST,
};

/// Deterministic non-silent test wave (no RNG): fixed arithmetic pattern
/// far from the 128 center so every sample is audible.
fn wave(n: usize) -> Vec<u8> {
    (0..n).map(|i| (40 + ((i * 13) % 199)) as u8).collect()
}

/// Max-amplitude square wave for saturation tests.
fn loud_wave(n: usize) -> Vec<u8> {
    (0..n).map(|i| if i % 2 == 0 { 255 } else { 0 }).collect()
}

fn script_with_events() -> MusicScript {
    let mut s = MusicScript::new();
    s.push(0, MusicCommand::NoteOn { instrument: 0, ratio: 0x10000, volume: 40 }).unwrap();
    s.push(20, MusicCommand::NoteOff { instrument: 0 }).unwrap();
    s.push(25, MusicCommand::NoteOn { instrument: 1, ratio: 0x18000, volume: 30 }).unwrap();
    s.push(60, MusicCommand::NoteOn { instrument: 0, ratio: 0x08000, volume: 50 }).unwrap();
    s.push(90, MusicCommand::NoteOff { instrument: 0 }).unwrap();
    s
}

fn armed_mixer() -> Mixer {
    let mut m = Mixer::new();
    m.load_wave(0, &wave(3000)).unwrap();
    m.load_wave(1, &wave(2500)).unwrap();
    m.load_script(script_with_events());
    m
}

fn sum_abs(buf: &[i16]) -> i64 {
    buf.iter().map(|&s| (s as i32).unsigned_abs() as i64).sum()
}

#[test]
fn same_script_same_bytes() {
    let mut a = armed_mixer();
    let mut b = armed_mixer();
    let mut sa = vec![0i16; SAMPLE_RATE as usize * 2];
    let mut sb = vec![0i16; SAMPLE_RATE as usize * 2];
    let fa = a.render(&mut sa).unwrap();
    let fb = b.render(&mut sb).unwrap();
    assert_eq!(fa, fb);
    assert_eq!(fa, SAMPLE_RATE as usize);
    assert_eq!(sa, sb, "identical scripts must mix byte-identically");
    assert!(sa.iter().any(|&s| s != 0), "stream must not be silent");
}

#[test]
fn chunking_is_invariant() {
    // One-shot reference.
    let mut big = armed_mixer();
    let mut ref_buf = vec![0i16; SAMPLE_RATE as usize * 2];
    big.render(&mut ref_buf).unwrap();

    // Same script, hostile chunking: sub-tick chunks, tick-crossing
    // chunks, and chunks landing exactly on tick boundaries.
    let mut m = armed_mixer();
    let mut out: Vec<i16> = Vec::with_capacity(SAMPLE_RATE as usize * 2);
    let mut chunk_frame = [1usize, 7, 64, 441, 512, 2];
    let mut ci = 0;
    while out.len() < SAMPLE_RATE as usize * 2 {
        let want = (SAMPLE_RATE as usize * 2 - out.len()).min(chunk_frame[ci] * 2);
        let mut piece = vec![0i16; want];
        let got = m.render(&mut piece).unwrap();
        assert_eq!(got * 2, want);
        out.extend_from_slice(&piece);
        ci = (ci + 1) % chunk_frame.len();
    }
    assert_eq!(out, ref_buf, "any host chunking yields the same bytes");
}

#[test]
fn note_off_releases_base_sub_voice_only() {
    let mut m = Mixer::new();
    m.load_wave(0, &wave(3000)).unwrap();
    let a = m.note_on(0, 0x10000, 100).unwrap();
    let b = m.note_on(0, 0x10000, 100).unwrap();
    assert_eq!((a.instrument, a.sub), (0, 0));
    assert_eq!((b.instrument, b.sub), (0, 1));

    let mut probe = [0i16; 200];
    m.render(&mut probe).unwrap();
    assert!(probe.iter().any(|&s| s != 0));

    assert!(m.note_off(0), "a base voice was sounding");
    assert!(!m.voice_playing(0, 0), "base released");
    assert!(m.voice_playing(0, 1), "sub 1 rings out (EXW quirk)");

    let mut ring = [0i16; 200];
    m.render(&mut ring).unwrap();
    assert!(ring.iter().any(|&s| s != 0), "sub 1 still audible");

    assert!(!m.note_off(0), "second note-off finds no base voice");
    // Play out the remaining sub-voice: 3000-sample wave, 200 already done.
    let mut tail = vec![0i16; 6000];
    m.render(&mut tail).unwrap();
    assert!(!m.voice_playing(0, 1), "one-shot voice ends with its wave");
}

#[test]
fn fifth_sub_voice_of_an_instrument_is_dropped() {
    let mut m = Mixer::new();
    m.load_wave(0, &wave(5000)).unwrap();
    for s in 0..SUB_VOICES_PER_INST {
        let r = m.note_on(0, 0x10000, 90).unwrap();
        assert_eq!(r.sub, s, "lowest free sub-voice wins");
    }
    assert!(m.note_on(0, 0x10000, 90).is_none(), "all four busy: drop");
    assert!(m.note_on(9, 0x10000, 90).is_none(), "no wave loaded: drop");
}

#[test]
fn pool_exhaustion_drops_at_twenty_voices() {
    let mut m = Mixer::new();
    for inst in 0..(MAX_VOICES / SUB_VOICES_PER_INST) as u16 {
        m.load_wave(inst, &wave(5000)).unwrap();
        for _ in 0..SUB_VOICES_PER_INST {
            assert!(m.note_on(inst, 0x10000, 90).is_some());
        }
    }
    assert!(m.note_on(0, 0x10000, 90).is_none(), "pool full: drop");
    let mut out = [0i16; 64];
    m.render(&mut out).unwrap(); // must stay panic-free while saturated
}

#[test]
fn one_shot_voices_recycle_their_slots() {
    let mut m = Mixer::new();
    m.load_wave(0, &wave(64)).unwrap();
    assert!(m.note_on(0, 0x10000, 90).unwrap().sub == 0);
    let mut out = vec![0i16; 160];
    m.render(&mut out).unwrap();
    assert!(!m.voice_playing(0, 0), "wave played out");
    let again = m.note_on(0, 0x10000, 90).unwrap();
    assert_eq!(again.sub, 0, "slot reused");
}

#[test]
fn master_bus_saturates_symmetrically() {
    let mut m = Mixer::new();
    for inst in 0..(MAX_VOICES / SUB_VOICES_PER_INST) as u16 {
        m.load_wave(inst, &loud_wave(10000)).unwrap();
        for _ in 0..SUB_VOICES_PER_INST {
            assert!(m.note_on(inst, 0x10000, 255).is_some());
        }
    }
    let mut out = [0i16; 64];
    m.render(&mut out).unwrap();
    assert!(out.contains(&32767), "positive rail reached");
    assert!(out.contains(&-32768), "negative rail reached");
}

#[test]
fn pan_is_audible_and_bounded() {
    let mk = |pan: i8| -> Vec<i16> {
        let mut m = Mixer::new();
        m.load_wave(0, &wave(2000)).unwrap();
        let r = m.note_on_pan(0, 0x10000, 200, pan);
        assert!(r.is_some());
        let mut out = [0i16; 1000];
        m.render(&mut out).unwrap();
        out.to_vec()
    };
    let right = mk(63);
    let left = mk(-63);
    // Channel-split helper: even indices L, odd indices R.
    let chan = |b: &[i16], odd: bool| -> i64 {
        sum_abs(&b.iter().enumerate().filter(|(i, _)| i % 2 == odd as usize).map(|(_, &s)| s).collect::<Vec<i16>>())
    };
    assert!(chan(&right, false) * 16 < chan(&right, true), "hard right: left ~1/64 of right");
    assert!(chan(&left, true) * 16 < chan(&left, false), "hard left: right ~1/64 of left");
    assert!(chan(&right, true) > 0 && chan(&left, false) > 0);
}

#[test]
fn master_zero_is_silence() {
    let mut m = Mixer::new();
    m.load_wave(0, &wave(2000)).unwrap();
    m.set_master_volume(0);
    assert!(m.note_on(0, 0x10000, 200).is_some(), "spawn succeeds muted");
    let mut out = [0i16; 500];
    m.render(&mut out).unwrap();
    assert!(out.iter().all(|&s| s == 0));
}

#[test]
fn script_events_fire_at_exact_sample_positions() {
    let mk = |tick: u32| -> Vec<i16> {
        let mut m = Mixer::new();
        m.load_wave(0, &wave(2000)).unwrap();
        let mut s = MusicScript::new();
        s.push(tick, MusicCommand::NoteOn { instrument: 0, ratio: 0x10000, volume: 255 }).unwrap();
        m.load_script(s);
        let mut out = vec![0i16; SAMPLE_RATE as usize * 2];
        m.render(&mut out).unwrap();
        out
    };
    let first_nonzero = |b: &[i16]| -> usize { b.iter().position(|&s| s != 0).unwrap() / 2 };
    // tick 4 = 441 samples exactly on the Q16 grid (DESIGN-AUDIO sec 5).
    assert_eq!(first_nonzero(&mk(4)), 441);
    assert_eq!(first_nonzero(&mk(0)), 0);
}

#[test]
fn odd_buffer_lengths_are_rejected() {
    let mut m = Mixer::new();
    m.load_wave(0, &wave(64)).unwrap();
    let mut out = [0i16; 5];
    assert!(m.render(&mut out).is_err());
    let mut even = [0i16; 6];
    assert_eq!(m.render(&mut even).unwrap(), 3);
}

#[test]
fn empty_mixer_renders_exact_silence() {
    let mut m = Mixer::new();
    let mut out = [3i16; 128];
    assert_eq!(m.render(&mut out).unwrap(), 64);
    assert!(out.iter().all(|&s| s == 0));
}
