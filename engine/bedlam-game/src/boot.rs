//! Boot attract sequence (P5, D36): the EXW GameMain first-run intro
//! pair - GTLOG_{UK,US}.SMK then LOGO_{UK,US}.SMK, region-selected by
//! the DAT_0046ae64 reimplementation ([`crate::movies::Region`]) - as a
//! presentation-only flow (D17 bucket b: no sim, no scene-hash contact;
//! the host stages it and the FSM never sees it).
//!
//! EXW anchors (RE-EXW-GAMETHREAD.md, Boot attract arm RE, 2026-08-20,
//! Ghidra dumps ghidra-project/exw-bootattract{,2}.txt):
//! - ORDER [verified, GameMain 0041c37a/0041c397]: GTLOG first, then
//!   LOGO, bracketed by the Smacker init/shutdown pair
//!   FUN_0042582a(0x400)/FUN_00425851.
//! - ONE PASS [verified; field identity inferred]: the runner
//!   FUN_0044567c bounds its frame loop by the header frame count -
//!   `for (f = 1; f < frames; f++)` renders frames-1 frames, so RING
//!   movies (the whole corpus attract set is rings) play exactly one
//!   bounded pass; the final frame is never decoded, rendered or
//!   audibly played. Wall duration of one movie = (frames-1) frame
//!   periods (each loop iteration renders, advances, then _SmackWait
//!   one period).
//! - GEOMETRY [verified]: the runner clears the screen (480x640 zeroed
//!   twice) at the start of EVERY call - the plane between the two
//!   movies is black, then the movie owns it; the boot pair plays at
//!   arg2 = 0 = full 640x480, no letterbox (dst height 480 - 2*arg2).
//! - UNSKIPPABLE [verified xref census]: the skip gate 004edbc4 is
//!   zeroed at GameMain entry and armed only inside NameEntryScreen
//!   around the TITLE replay - during the boot attract it reads 0, so
//!   the pair plays its full pass with no input abort.
//!
//! The flow reuses the D31 [`crate::movie::MoviePlayer`] clock: the
//! same x240-us fixed-step grid, the same per-frame palette fold and
//! the same mixer-native audio handoff. Sequencing is time-exact: a
//! movie switches when its `(frames-1) * period` budget elapses, and
//! [`MoviePlayer::advance_limited`] guarantees the decode count never
//! exceeds frames-1 even under starvation bursts (the EXW hard loop
//! bound).

use crate::loading::Plane;
use crate::movie::{MoviePlayer, UNITS_PER_SUBTICK, UNITS_PER_US};
use crate::GameError;

/// Flow phase. `Staged` = built but waiting for the Boot scene;
/// `Playing` = the pair is on screen; `Done` = both movies finished
/// their one pass and the last raster is held until the scene drops
/// the flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    Staged,
    Playing,
    Done,
}

/// The two-movie boot attract, EXW order (GTLOG then LOGO).
#[derive(Debug)]
pub struct BootAttract {
    players: Vec<MoviePlayer>,
    /// One-pass decode target per movie = frames - 1 [EXW loop bound].
    targets: Vec<u32>,
    /// x240-us elapsed on the CURRENT movie; the switch fires when
    /// this reaches target * period.
    elapsed: u64,
    idx: usize,
    /// Frames decoded on the current movie INCLUDING the construction
    /// decode of frame 0 (EXW renders frames-1 frames: frame 0 through
    /// frame frames-2).
    decoded: u32,
    phase: BootPhase,
}

impl BootAttract {
    /// Build the staged flow from the two .SMK byte blobs the caller
    /// fetched under [`crate::movies::boot_pair`] (GTLOG first). Both
    /// streams open and decode frame 0 immediately (the D31
    /// construction contract); playback starts on [`Self::start`].
    pub fn new(gtlog: &[u8], logo: &[u8]) -> Result<BootAttract, GameError> {
        let build = |data: &[u8]| -> Result<(MoviePlayer, u32), GameError> {
            let player = MoviePlayer::new(data)?;
            // frames - 1 decoded frames = the EXW one-pass render count.
            let target = player.info().frames.saturating_sub(1);
            Ok((player, target))
        };
        let (gtlog, gtlog_target) = build(gtlog)?;
        let (logo, logo_target) = build(logo)?;
        Ok(BootAttract {
            players: vec![gtlog, logo],
            targets: vec![gtlog_target, logo_target],
            elapsed: 0,
            idx: 0,
            decoded: 1,
            phase: BootPhase::Staged,
        })
    }

    /// Begin playback: returns the GTLOG frame-0 audio packet (the D31
    /// entry semantics - frame-0 audio queues when playback starts,
    /// not at construction). A repeat call is a no-op returning
    /// nothing.
    pub fn start(&mut self) -> Vec<u8> {
        if self.phase != BootPhase::Staged {
            return Vec::new();
        }
        self.phase = BootPhase::Playing;
        self.players[self.idx].take_audio()
    }

    /// Feed one host-frame dt (sub-ticks on the 240 Hz grid). Returns
    /// the audio bytes decoded since the last call, in decode order,
    /// with the NEXT movie frame-0 packet appended at a switch (the
    /// entry-audio rule applied per movie). After [`BootPhase::Done`]
    /// the call is a no-op returning nothing (the last raster holds).
    pub fn advance(&mut self, dt_subticks: u32) -> Result<Vec<u8>, GameError> {
        if self.phase != BootPhase::Playing {
            return Ok(Vec::new());
        }
        let mut audio = Vec::new();
        self.elapsed += u64::from(dt_subticks) * UNITS_PER_SUBTICK;
        let target = self.targets[self.idx];
        if self.decoded < target {
            let budget = target - self.decoded;
            let decoded = self.players[self.idx].advance_limited(dt_subticks, budget)?;
            self.decoded += decoded;
            audio.extend_from_slice(&self.players[self.idx].take_audio());
        }
        // The pass is over when its whole display budget elapsed (the
        // final frame has SHOWN for its full period) or a non-ring
        // stream ended early.
        let period = self.players[self.idx].info().us_per_frame * UNITS_PER_US;
        let over = self.elapsed >= u64::from(target) * period || self.players[self.idx].finished();
        if over {
            self.idx += 1;
            if self.idx >= self.players.len() {
                self.phase = BootPhase::Done;
            } else {
                self.decoded = 1;
                self.elapsed = 0;
                audio.extend_from_slice(&self.players[self.idx].take_audio());
            }
        }
        Ok(audio)
    }

    /// Current phase.
    pub fn phase(&self) -> BootPhase {
        self.phase
    }

    /// Index of the movie on screen (0 = GTLOG, 1 = LOGO; after
    /// [`BootPhase::Done`] this is the LAST movie, whose raster holds).
    pub fn movie_index(&self) -> usize {
        self.idx.min(self.players.len() - 1)
    }

    /// Zero-based frame index of the movie on screen.
    pub fn frame_index(&self) -> u32 {
        self.players[self.movie_index()].frame_index()
    }

    /// The attract plane: the current movie raster + folded 6-bit
    /// palette, once playing (Staged shows nothing; the EXW runner
    /// clears the screen to black before the first frame, which the
    /// scene pipeline already provides). A full 640x480 raster centers
    /// at the origin = the EXW arg2 = 0 full-screen 1:1 blit.
    pub(crate) fn plane(&self) -> Option<Plane<'_>> {
        if self.phase == BootPhase::Staged {
            return None;
        }
        let player = &self.players[self.movie_index()];
        Some(Plane {
            w: player.info().width,
            h: player.info().height,
            pixels: player.pixels(),
            palette: player.palette(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// N-frame 4x4 SMK at 40 ms/frame, raw-PCM mono 8-bit 11025 Hz
    /// track, layout byte-compatible with the D30/movie.rs fixtures:
    /// frame 0 carries the palette chunk + a 2-byte audio packet
    /// (first), every later frame a 3-byte audio packet derived from
    /// fill (fill + frame index, three times). The raster never
    /// changes (audio-only later frames), which is all the sequencing
    /// tests need.
    fn synth(frames: u32, first: [u8; 2], fill: u8) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"SMK2");
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&frames.to_le_bytes());
        d.extend_from_slice(&40u32.to_le_bytes()); // 40 ms per frame
        d.extend_from_slice(&0u32.to_le_bytes()); // flags
        for i in 0..7u32 {
            d.extend_from_slice(&(if i == 0 { 16u32 } else { 0u32 }).to_le_bytes());
        }
        d.extend_from_slice(&1u32.to_le_bytes()); // tree chunk: 1 byte
        for _ in 0..4 {
            d.extend_from_slice(&0u32.to_le_bytes()); // tree maxima
        }
        for i in 0..7u32 {
            d.extend_from_slice(
                &(if i == 0 {
                    0x4000_0000u32 | 11_025
                } else {
                    0u32
                })
                .to_le_bytes(),
            );
        }
        d.extend_from_slice(&0u32.to_le_bytes()); // dummy
        d.extend_from_slice(&17u32.to_le_bytes()); // frame 0 chunk
        for _ in 1..frames {
            d.extend_from_slice(&8u32.to_le_bytes()); // later chunks
        }
        d.push(0x03); // frame 0 type: palette + audio track 0
        d.extend(std::iter::repeat_n(0x02, (frames - 1) as usize)); // later types
        d.push(0x00); // tree chunk: four absent trees
        d.extend_from_slice(&[0x02, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0x00, 0x00]); // palette
        d.extend_from_slice(&[0x06, 0x00, 0x00, 0x00, first[0], first[1]]); // audio
        d.extend_from_slice(&[0x00, 0x00]); // video pad
        for k in 1..frames {
            let b = fill.wrapping_add(k as u8);
            d.extend_from_slice(&[0x07, 0x00, 0x00, 0x00, b, b, b, 0x00]);
        }
        d
    }

    #[test]
    fn staged_flow_is_silent_until_started() {
        let mut f =
            BootAttract::new(&synth(2, [0xAA, 0x55], 0x10), &synth(2, [0xBB, 0x66], 0x20)).unwrap();
        assert_eq!(f.phase(), BootPhase::Staged);
        assert!(f.plane().is_none(), "staged shows nothing");
        assert_eq!(
            f.advance(4).unwrap(),
            Vec::<u8>::new(),
            "no-op before start"
        );
        // Start hands the GTLOG frame-0 packet (the D31 entry rule).
        assert_eq!(f.start(), vec![0xAA, 0x55]);
        assert_eq!(f.phase(), BootPhase::Playing);
        assert!(f.plane().is_some());
        assert_eq!(f.start(), Vec::<u8>::new(), "second start is a no-op");
    }

    #[test]
    fn one_pass_bound_and_exact_switch_timing() {
        // 3-frame movies at 40 ms: EXW renders frames-1 = 2 frames,
        // movie duration = 2 periods = 19.2 sub-ticks, the third
        // frame never decodes (audio included).
        let a = synth(3, [0xAA, 0x55], 0x10); // f0=[AA 55], f1=[11 11 11]
        let b = synth(3, [0xBB, 0x66], 0x40); // f0=[BB 66], f1=[41 41 41]
        let mut f = BootAttract::new(&a, &b).unwrap();
        f.start();
        // 10 sub-ticks: frame 1 decodes (its audio), the pass holds.
        assert_eq!(f.advance(10).unwrap(), vec![0x11, 0x11, 0x11]);
        assert_eq!(f.movie_index(), 0);
        assert_eq!(f.frame_index(), 1);
        // 10 more (20 total >= 19.2): frame 2 capped off, the LOGO
        // starts and its frame-0 audio queues NOW.
        assert_eq!(f.advance(10).unwrap(), vec![0xBB, 0x66]);
        assert_eq!(f.movie_index(), 1);
        assert_eq!(f.frame_index(), 0, "LOGO frame 0 held, its clock restarted");
        assert_eq!(f.advance(10).unwrap(), vec![0x41, 0x41, 0x41]);
        // 20 sub-ticks on the LOGO: the pass ends, Done holds.
        assert_eq!(f.advance(10).unwrap(), Vec::<u8>::new());
        assert_eq!(f.phase(), BootPhase::Done);
        assert_eq!(f.movie_index(), 1);
        assert_eq!(
            f.frame_index(),
            1,
            "LOGO frame 1 = frames-2 = the last EXW-rendered frame"
        );
        // Done is a strict no-op and the raster keeps holding.
        assert_eq!(f.advance(1000).unwrap(), Vec::<u8>::new());
        assert_eq!(f.frame_index(), 1);
        assert!(f.plane().is_some(), "Done holds the last raster");
    }

    #[test]
    fn starvation_burst_never_exceeds_the_pass_bound() {
        // One giant dt spanning several periods: the hard cap still
        // decodes at most budget frames (the EXW loop bound) and the
        // switch fires on the same call.
        let a = synth(3, [0xAA, 0x55], 0x10);
        let b = synth(2, [0xBB, 0x66], 0x40);
        let mut f = BootAttract::new(&a, &b).unwrap();
        f.start();
        let out = f.advance(100).unwrap();
        assert_eq!(out, vec![0x11, 0x11, 0x11, 0xBB, 0x66]);
        assert_eq!(f.movie_index(), 1);
        assert_eq!(f.frame_index(), 0);
        // LOGO (2 frames): one period finishes the whole attract.
        assert_eq!(f.advance(10).unwrap(), Vec::<u8>::new());
        assert_eq!(f.phase(), BootPhase::Done);
    }

    #[test]
    fn two_frame_movies_render_exactly_one_frame_each() {
        // EXW renders frames-1 frames: a 2-frame movie shows frame 0
        // for one period, then the pass is over.
        let a = synth(2, [0xAA, 0x55], 0x10);
        let b = synth(2, [0xBB, 0x66], 0x40);
        let mut f = BootAttract::new(&a, &b).unwrap();
        assert_eq!(f.start(), vec![0xAA, 0x55]);
        assert_eq!(f.advance(10).unwrap(), vec![0xBB, 0x66]);
        assert_eq!(f.movie_index(), 1);
        assert_eq!(f.frame_index(), 0, "LOGO frame 0, never frame 1");
        assert_eq!(f.advance(10).unwrap(), Vec::<u8>::new());
        assert_eq!(f.phase(), BootPhase::Done);
    }

    #[test]
    fn bad_bytes_reject_at_construction() {
        assert!(BootAttract::new(&[1u8, 2, 3], &synth(2, [1, 2], 3)).is_err());
        assert!(BootAttract::new(&synth(2, [1, 2], 3), &[9u8]).is_err());
    }
}
