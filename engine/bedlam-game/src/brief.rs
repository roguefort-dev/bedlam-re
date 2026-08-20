//! Briefing intro pair (P5, D37): the EXW briefing screen movie
//! head - the drop-ship pass then the zone backdrop - as a
//! presentation-only flow (D17 bucket b: no sim, no scene-hash
//! contact; the host stages it and the FSM never sees it).
//!
//! EXW anchors (RE-EXW-GAMETHREAD.md, "Briefing screen + BRF_DROP
//! play site" D37 section, 2026-08-20, Ghidra dump
//! ghidra-project/exw-brfdrop.txt):
//! - FUN_0043d00b IS the briefing screen [verified decompile]: it
//!   loads the briefing asset set, builds the zone-movie name,
//!   plays the movies, runs the mission-map UI and exits into the
//!   region loading screen; the real gameplay is FUN_00440e45
//!   AFTER it returns 1.
//! - ORDER [verified asm 0043d447..0043d490]: BRF_DROP.SMK (the
//!   literal at 0x4591f7) opens FIRST at every movie-enabled
//!   briefing (gate DAT_0046cca4), full screen (_SmackToBuffer dst
//!   height 0x1e0 = 480 rows, the 640x480 raster 1:1, no
//!   letterbox), plays ONE pass, then hands off to the pre-built
//!   BRF_{zone}{level}.SMK ring (name buffer DAT_004dca0c) which
//!   plays until the UI exits - no close bound, the ring wraps.
//! - ONE PASS [verified decompile]: the handoff check is
//!   framecount(+0xc) - 1 == frame_index(+0x370), i.e. the drop
//!   renders frames 0..=count-2 = count-1 frames - the SAME render
//!   count as the FUN_0044567c modal runner (mechanism differs:
//!   frame-index equality vs loop counter). The corpus drop (30
//!   frames, non-ring, silent) renders 29 of its 30 frames; the
//!   handoff is mandatory because a non-ring Smack simply stops.
//! - UNSKIPPABLE [verified decompile]: no skip gate is consulted;
//!   the cursor handlers (GO button included) arm only after the
//!   handoff fired - the player cannot leave the screen while the
//!   drop plays.
//! - FATAL on open failure [verified asm]: a dedicated error
//!   (0x45920c) + fn-pointer teardown + CRT exit for the drop, a
//!   generic %s error for the backdrop - both opens are hard
//!   requirements of a movie-enabled briefing.
//!
//! The flow reuses the D31 [crate::movie::MoviePlayer] clock: the
//! x240-us fixed-step grid, per-frame palette fold and mixer-native
//! audio handoff (the corpus pair is silent, but the mechanism
//! stands - a drop/backdrop carrying a track would queue its
//! frame-0 packet at start/handoff like the boot attract).
//! Sequencing is time-exact: the handoff fires when the drop
//! display budget of (frames-1) * period elapses, and
//! [MoviePlayer::advance_limited] guarantees the decode count never
//! exceeds frames-1 even under starvation bursts (the EXW handoff
//! bound made starvation-proof).

use crate::loading::Plane;
use crate::movie::{MoviePlayer, UNITS_PER_SUBTICK, UNITS_PER_US};
use crate::GameError;

/// Flow phase. Staged = built but waiting for the Brief scene;
/// Drop = the drop movie is playing its one pass; Backdrop = the
/// zone backdrop ring owns the plane for the rest of the scene (it
/// never ends - leaving the scene drops the whole flow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BriefPhase {
    Staged,
    Drop,
    Backdrop,
}

/// The two-movie briefing intro: the drop-ship pass (one EXW pass)
/// then the endless zone backdrop ring.
#[derive(Debug)]
pub struct BriefIntro {
    drop: MoviePlayer,
    backdrop: MoviePlayer,
    /// One-pass decode target for the drop = frames - 1 total
    /// decoded frames INCLUDING the construction decode of frame 0
    /// [EXW handoff bound: frames 0..=frames-2 render].
    drop_target: u32,
    /// x240-us elapsed on the drop; the handoff fires when this
    /// reaches drop_target * period.
    elapsed: u64,
    /// Frames decoded on the drop INCLUDING the construction
    /// decode of frame 0.
    decoded: u32,
    phase: BriefPhase,
}

impl BriefIntro {
    /// Build the staged flow from the two .SMK byte blobs the
    /// caller fetched under [crate::movies::BRIEFING_DROP_NAME]
    /// and [crate::movies::briefing_name] (drop first). Both
    /// streams open and decode frame 0 immediately (the D31
    /// construction contract); playback starts on [Self::start].
    pub fn new(drop: &[u8], backdrop: &[u8]) -> Result<BriefIntro, GameError> {
        let drop = MoviePlayer::new(drop)?;
        let drop_target = drop.info().frames.saturating_sub(1);
        let backdrop = MoviePlayer::new(backdrop)?;
        Ok(BriefIntro {
            drop,
            backdrop,
            drop_target,
            elapsed: 0,
            decoded: 1,
            phase: BriefPhase::Staged,
        })
    }

    /// Begin the drop pass: returns its frame-0 audio packet (the
    /// D31 entry semantics - frame-0 audio queues when playback
    /// starts, not at construction; the corpus drop is silent, so
    /// this is empty there). A repeat call is a no-op returning
    /// nothing.
    pub fn start(&mut self) -> Vec<u8> {
        if self.phase != BriefPhase::Staged {
            return Vec::new();
        }
        self.phase = BriefPhase::Drop;
        self.drop.take_audio()
    }

    /// Feed one host-frame dt (sub-ticks on the 240 Hz grid).
    /// Returns the audio bytes decoded since the last call, in
    /// decode order, with the backdrop frame-0 packet appended at
    /// the handoff (the entry-audio rule applied per movie). In
    /// [BriefPhase::Backdrop] the ring advances UNBOUNDED - the
    /// EXW backdrop plays until the UI exits the scene, so the
    /// flow itself never reports an end; the host drops it on the
    /// scene exit.
    pub fn advance(&mut self, dt_subticks: u32) -> Result<Vec<u8>, GameError> {
        if self.phase == BriefPhase::Staged {
            return Ok(Vec::new());
        }
        if self.phase == BriefPhase::Backdrop {
            self.backdrop.advance(dt_subticks)?;
            return Ok(self.backdrop.take_audio());
        }
        let mut audio = Vec::new();
        self.elapsed += u64::from(dt_subticks) * UNITS_PER_SUBTICK;
        if self.decoded < self.drop_target {
            let budget = self.drop_target - self.decoded;
            self.decoded += self.drop.advance_limited(dt_subticks, budget)?;
            audio.extend_from_slice(&self.drop.take_audio());
        }
        // The pass is over when its whole display budget elapsed
        // (the final shown frame held its full period) or the
        // non-ring stream ended early (EXW: the handoff is the
        // frame index reaching count-1; a stream that ends sooner
        // reaches its own bound first).
        let period = self.drop.info().us_per_frame * UNITS_PER_US;
        let over = self.elapsed >= u64::from(self.drop_target) * period || self.drop.finished();
        if over {
            self.phase = BriefPhase::Backdrop;
            audio.extend_from_slice(&self.backdrop.take_audio());
        }
        Ok(audio)
    }

    /// Current phase.
    pub fn phase(&self) -> BriefPhase {
        self.phase
    }

    /// Zero-based frame index of the movie on screen: the drop
    /// while its pass plays (Staged reports the held frame 0),
    /// else the backdrop ring.
    pub fn frame_index(&self) -> u32 {
        match self.phase {
            BriefPhase::Backdrop => self.backdrop.frame_index(),
            BriefPhase::Staged | BriefPhase::Drop => self.drop.frame_index(),
        }
    }

    /// The briefing plane: the on-screen movie raster + folded
    /// 6-bit palette, once playing (Staged shows nothing; the EXW
    /// screen movies own the plane only after the open). Both
    /// corpus movies are 640x480: the full-screen 1:1 blit,
    /// _SmackToBuffer dst height 0x1e0 = 480 rows [verified asm
    /// 0043d47d].
    pub(crate) fn plane(&self) -> Option<Plane<'_>> {
        if self.phase == BriefPhase::Staged {
            return None;
        }
        let player = match self.phase {
            BriefPhase::Backdrop => &self.backdrop,
            BriefPhase::Staged | BriefPhase::Drop => &self.drop,
        };
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
    /// track, layout byte-compatible with the D30/movie.rs
    /// fixtures (same shape as the boot.rs attract fixtures): frame
    /// 0 carries the palette chunk + a 2-byte audio packet
    /// (first), every later frame a 3-byte audio packet derived
    /// from fill (fill + frame index, three times). The raster
    /// never changes (audio-only later frames), which is all the
    /// sequencing tests need.
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
            BriefIntro::new(&synth(3, [0xAA, 0x55], 0x10), &synth(4, [0xBB, 0x66], 0x40)).unwrap();
        assert_eq!(f.phase(), BriefPhase::Staged);
        assert!(f.plane().is_none(), "staged shows nothing");
        assert_eq!(
            f.advance(4).unwrap(),
            Vec::<u8>::new(),
            "no-op before start"
        );
        // Start hands the drop frame-0 packet (the D31 entry rule).
        assert_eq!(f.start(), vec![0xAA, 0x55]);
        assert_eq!(f.phase(), BriefPhase::Drop);
        assert!(f.plane().is_some());
        assert_eq!(f.start(), Vec::<u8>::new(), "second start is a no-op");
    }

    #[test]
    fn one_pass_bound_and_exact_handoff_timing() {
        // 3-frame drop at 40 ms: EXW renders frames-1 = 2 frames
        // (0 and 1), the pass = 2 periods = 19.2 sub-ticks; frame 2
        // never decodes. 4-frame backdrop (non-ring synth): after
        // the handoff it walks its own frames and, being non-ring,
        // finishes and holds (the corpus 512-frame rings wrap
        // instead - pinned by the corpus gate).
        let mut f =
            BriefIntro::new(&synth(3, [0xAA, 0x55], 0x10), &synth(4, [0xBB, 0x66], 0x40)).unwrap();
        f.start();
        // 10 sub-ticks: drop frame 1 decodes (its audio), the pass
        // holds.
        assert_eq!(f.advance(10).unwrap(), vec![0x11, 0x11, 0x11]);
        assert_eq!(f.phase(), BriefPhase::Drop);
        assert_eq!(f.frame_index(), 1);
        // 10 more (20 total >= 19.2): frame 2 capped off, the
        // handoff fires and the backdrop frame-0 audio queues NOW.
        assert_eq!(f.advance(10).unwrap(), vec![0xBB, 0x66]);
        assert_eq!(f.phase(), BriefPhase::Backdrop);
        assert_eq!(
            f.frame_index(),
            0,
            "backdrop frame 0 held, its clock started"
        );
        assert_eq!(f.advance(10).unwrap(), vec![0x41, 0x41, 0x41]);
        assert_eq!(f.frame_index(), 1);
        assert_eq!(f.advance(10).unwrap(), vec![0x42, 0x42, 0x42]);
        assert_eq!(f.advance(10).unwrap(), vec![0x43, 0x43, 0x43]);
        // The non-ring synth backdrop finished on frame 3 and holds
        // its raster; advance stays a no-op.
        assert_eq!(f.advance(10).unwrap(), Vec::<u8>::new());
        assert_eq!(f.frame_index(), 3);
        assert!(f.plane().is_some(), "Backdrop holds the last raster");
        assert_eq!(
            f.phase(),
            BriefPhase::Backdrop,
            "no Done - the scene exit drops the flow"
        );
    }

    #[test]
    fn starvation_burst_never_exceeds_the_pass_bound() {
        // One giant dt spanning several periods: the hard cap still
        // decodes at most budget frames (the EXW handoff bound) and
        // the handoff fires on the same call.
        let mut f =
            BriefIntro::new(&synth(3, [0xAA, 0x55], 0x10), &synth(2, [0xBB, 0x66], 0x40)).unwrap();
        f.start();
        let out = f.advance(100).unwrap();
        assert_eq!(out, vec![0x11, 0x11, 0x11, 0xBB, 0x66]);
        assert_eq!(f.phase(), BriefPhase::Backdrop);
        assert_eq!(f.frame_index(), 0);
    }

    #[test]
    fn two_frame_drop_passes_in_one_period() {
        // EXW renders frames-1 frames: a 2-frame drop shows frame 0
        // for one period, then the handoff fires; frame 1 never
        // decodes.
        let mut f =
            BriefIntro::new(&synth(2, [0xAA, 0x55], 0x10), &synth(2, [0xBB, 0x66], 0x40)).unwrap();
        assert_eq!(f.start(), vec![0xAA, 0x55]);
        assert_eq!(f.advance(10).unwrap(), vec![0xBB, 0x66]);
        assert_eq!(f.phase(), BriefPhase::Backdrop);
        assert_eq!(f.frame_index(), 0, "backdrop frame 0, never drop frame 1");
    }

    #[test]
    fn bad_bytes_reject_at_construction() {
        assert!(BriefIntro::new(&[1u8, 2, 3], &synth(2, [1, 2], 3)).is_err());
        assert!(BriefIntro::new(&synth(2, [1, 2], 3), &[9u8]).is_err());
    }
}
