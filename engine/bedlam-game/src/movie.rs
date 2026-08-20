//! MoviePlayer - decoded SMK playback for presentation (D31, PLAN
//! P5 playback integration). Wraps the bedlam-assets SmkStream seam and
//! drives it from the fixed-step host clock: the caller hands each
//! pump the SAME dt_subticks the SimDriver consumed (the 240 Hz
//! sub-tick grid, bedlam-core frame.rs), and the player decodes whole
//! movie frames on an exact integer accumulator - never dt math, never
//! a float, never a wall clock.
//!
//! Time unit: 1/240_000_000 s. One sub-tick = 1_000_000 units, one
//! microsecond = 240 units, so both the frame period (us_per_frame *
//! 240) and the host dt (sub-ticks * 1_000_000) are exact integers and
//! the accumulator is drift-free by construction (no rounding ever).
//! TITLE.SMK: 66_660 us = 15_998_400 units = 15.9984 sub-ticks per
//! frame at 15 fps.
//!
//! Determinism boundary (D17 bucket b): the movie is PRESENTATION. It
//! never touches the sim, the scene hash, or any hashed bucket; its
//! outputs (raster + palette + queued PCM) flow through the canonical
//! Frame seam (bedlam-render blit) and the mixer byte-stream channel
//! (bedlam-audio queue_pcm_u8) only. The headless decode gate stays
//! tests/smk_title_gate.rs in bedlam-assets (double decode, byte
//! identity); this layer adds the CLOCK contract on top.

use bedlam_assets::smk::{SmkAudioCodec, SmkFrameStatus, SmkStream, SmkStreamInfo};
use bedlam_audio::SAMPLE_RATE;
use bedlam_render::Vga6;

use crate::GameError;

/// Fixed-step host grid: sub-ticks per second (bedlam-core
/// SUBTICKS_PER_TICK * 60 Hz = 4 * 60). The player itself never
/// imports bedlam-core: presentation may not depend on the sim crate.
pub const SUBTICKS_PER_SECOND: u64 = 240;

/// Units per sub-tick (1 sub-tick = 1/240 s = 1_000_000 x240-us units).
pub(crate) const UNITS_PER_SUBTICK: u64 = 1_000_000;

/// Units per microsecond (1 us = 240 units of 1/240_000_000 s).
pub(crate) const UNITS_PER_US: u64 = 240;

/// Runaway-dt guard: a single advance decoding more movie frames than
/// this drops the excess (accumulator reset) instead of spinning. 4096
/// frames is ~4.5 min of 15 fps content in one host frame; unreachable
/// for real hosts, load-bearing against hostile/inverted clocks.
const MAX_FRAMES_PER_ADVANCE: u32 = 4096;

/// One decoded-movie playback state (D31). Construct from raw .SMK
/// bytes (frame 0 decodes immediately so the first presented frame is
/// ready before playback starts), then feed advance(dt_subticks) with
/// the same dt the host pumped.
#[derive(Debug)]
pub struct MoviePlayer {
    stream: SmkStream,
    /// Frame period in x240-us units: us_per_frame * 240.
    period: u64,
    /// Elapsed x240-us units since the last decoded frame.
    acc: u64,
    /// Non-ring streams end; ring streams wrap forever (the seam
    /// reports More eternally, matching the container).
    finished: bool,
    /// Audio bytes decoded but not yet handed to the mixer, in decode
    /// order. Frame 0 lands here at construction; the host queues it
    /// when the target scene is ENTERED, not at load.
    pending_audio: Vec<u8>,
}

impl MoviePlayer {
    /// Open the stream and decode frame 0 (plus its audio packet).
    /// Fails with the typed AssetsError mapping of the seam; a passing
    /// open has already been structurally validated (D30).
    pub fn new(data: &[u8]) -> Result<MoviePlayer, GameError> {
        let mut stream = SmkStream::open(data)?;
        stream.first_frame()?;
        let period = stream.info().us_per_frame * UNITS_PER_US;
        let mut player = MoviePlayer {
            stream,
            period,
            acc: 0,
            finished: false,
            pending_audio: Vec::new(),
        };
        player.collect_audio();
        Ok(player)
    }

    /// Container facts, frozen at open.
    pub fn info(&self) -> &SmkStreamInfo {
        self.stream.info()
    }

    /// Zero-based index of the frame decoded last.
    pub fn frame_index(&self) -> u32 {
        self.stream.frame_index()
    }

    /// Current raster (width * height palette indices, row-major).
    pub fn pixels(&self) -> &[u8] {
        self.stream.pixels()
    }

    /// Current palette as canonical 6-bit VGA entries. The seam serves
    /// the Smacker PALMAP-expanded 8-bit components; the vendored table
    /// (bedlam-smk video.rs) satisfies PALMAP[v] == (v << 2) | (v >> 4)
    /// for every 6-bit v, and that map is exactly inverted by >> 2, so
    /// the fold below is lossless (unit-pinned in the tests).
    pub fn palette(&self) -> [Vga6; 256] {
        let mut out = [[0u8; 3]; 256];
        for (dst, src) in out.iter_mut().zip(self.stream.palette()) {
            *dst = [src[0] >> 2, src[1] >> 2, src[2] >> 2];
        }
        out
    }

    /// Whether the stream has ended (non-ring streams only). A finished
    /// player keeps serving its last frame; advance becomes a no-op.
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Rewind to frame 0 and decode it (D42: the EXW attract replay
    /// re-opens TITLE.SMK through FUN_004459f7 - the engine restarts
    /// the staged player in place instead). Resets the accumulator,
    /// clears pending audio and re-collects the frame-0 packet (the
    /// D31 entry-audio rule applies to the replay too: the caller
    /// queues take_audio() when the replay starts).
    pub fn restart(&mut self) -> Result<(), GameError> {
        self.stream.first_frame()?;
        self.acc = 0;
        self.finished = false;
        self.pending_audio.clear();
        self.collect_audio();
        Ok(())
    }

    /// Latch finished while holding the last decoded frame (D42: the
    /// attract SKIP - EXW aborts FUN_0044567c's wait loop mid-pass
    /// through skip gate 004edbc4 and redraws the menu; the frozen
    /// raster is never presented because the menu plane takes over
    /// once the player stops playing).
    pub fn finish(&mut self) {
        self.finished = true;
        self.acc = 0;
    }

    /// Take the audio decoded since the last call, in decode order.
    /// Empty when nothing new decoded.
    pub fn take_audio(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending_audio)
    }

    /// Feed one host-frame dt (sub-ticks on the 240 Hz grid). Decodes
    /// every movie frame whose period has fully elapsed, banking the
    /// remainder - the same accumulator discipline as SimDriver, so a
    /// 60 Hz host and a 240 Hz host present the identical frame
    /// sequence at the identical wall times (fractional periods carry,
    /// never round). Ring streams wrap (More forever); non-ring streams
    /// latch finished on Last/Done and reset the accumulator.
    pub fn advance(&mut self, dt_subticks: u32) -> Result<(), GameError> {
        self.advance_limited(dt_subticks, u32::MAX).map(|_| ())
    }

    /// [`Self::advance`] with a HARD decode cap: at most `max_frames`
    /// movie frames decode from this one call (a starvation burst that
    /// spans several periods still decodes at most the cap; leftover
    /// accumulator time is kept warm rather than discarded). Returns
    /// how many frames decoded.
    ///
    /// EXW parity anchor (D36, RE-EXW-GAMETHREAD "Boot attract arm RE"):
    /// the runner FUN_0044567c bounds its frame loop by the header
    /// frame count - `for (f = 1; f < frames; f++)` renders exactly
    /// frames-1 frames no matter how starved the pacing is, so a ring
    /// movie plays exactly ONE bounded pass and its final frame is
    /// never decoded, rendered or audibly played. The boot attract
    /// enforces that bound through this cap; ordinary scene movies
    /// (TITLE and the cutscenes play whole files) keep the uncapped
    /// [`Self::advance`].
    pub fn advance_limited(&mut self, dt_subticks: u32, max_frames: u32) -> Result<u32, GameError> {
        if self.finished || max_frames == 0 {
            return Ok(0);
        }
        self.acc += u64::from(dt_subticks) * UNITS_PER_SUBTICK;
        let mut decoded = 0u32;
        while self.acc >= self.period {
            self.acc -= self.period;
            let status = self.stream.next_frame()?;
            self.collect_audio();
            decoded += 1;
            match status {
                SmkFrameStatus::More => {}
                SmkFrameStatus::Last if self.stream.info().ring_frame => {
                    // A ring stream reports Last at the closing slot
                    // of EVERY pass and wraps to frame 1 on the next
                    // advance (the seam contract: Last = the last
                    // frame of a stream OR OF A RING PASS) - a ring
                    // never ends, so finished never latches and the
                    // ring keeps decoding (D37: the briefing
                    // backdrops ring until the scene exit; found by
                    // the brief_gate corpus run freezing at the
                    // first closing slot).
                }
                SmkFrameStatus::Last | SmkFrameStatus::Done => {
                    self.finished = true;
                    self.acc = 0;
                    return Ok(decoded);
                }
            }
            if decoded >= MAX_FRAMES_PER_ADVANCE {
                self.acc = 0;
                break;
            }
            if decoded >= max_frames {
                // Cap hit mid-burst: stop decoding, keep the leftover
                // accumulator time (the caller retires this movie at
                // the cap - EXW drops the leftover wall time with it).
                break;
            }
        }
        Ok(decoded)
    }

    /// Collect the audio packet of the first ELIGIBLE track: decoded
    /// PCM already in the mixer native format (non-Bink codec, mono,
    /// 8-bit, 11025 Hz - exactly TITLE.SMK track 0, DPCM 8-bit mono
    /// 11025). Other track shapes are presentation-side resampling,
    /// deliberately out of scope until a corpus file needs them.
    fn collect_audio(&mut self) {
        let eligible = self.stream.info().audio.iter().position(|t| {
            t.is_some_and(|m| {
                m.codec != SmkAudioCodec::Bink
                    && m.channels == 1
                    && m.bitdepth == 8
                    && u64::from(m.rate_hz) == u64::from(SAMPLE_RATE)
            })
        });
        if let Some(track) = eligible {
            if let Some(packet) = self.stream.audio_packet(track) {
                self.pending_audio.extend_from_slice(packet);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bedlam_assets::smk::SmkAudioCodec;

    /// Minimal 2-frame 4x4 SMK at 40 ms/frame with one raw-PCM 8-bit
    /// mono 11025 Hz track: frame 0 palette entry 0 = PALMAP[1,2,3],
    /// audio packets [AA 55] then [11 22 33]. Byte-compatible with the
    /// bedlam-assets smk.rs synthetic corpus builders (D30 tests).
    fn synth_stream() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"SMK2");
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&2u32.to_le_bytes()); // frames
        d.extend_from_slice(&40u32.to_le_bytes()); // 40 ms per frame
        d.extend_from_slice(&0u32.to_le_bytes()); // flags
        for i in 0..7u32 {
            d.extend_from_slice(&(if i == 0 { 16u32 } else { 0u32 }).to_le_bytes());
        }
        d.extend_from_slice(&1u32.to_le_bytes()); // tree chunk: 1 byte
        for _ in 0..4 {
            d.extend_from_slice(&0u32.to_le_bytes()); // tree maxima: unused
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
        d.extend_from_slice(&17u32.to_le_bytes()); // frame0 chunk: 16B + key
        d.extend_from_slice(&8u32.to_le_bytes()); // frame1 chunk: 8B
        d.push(0x03); // frame0 type: palette + audio track 0
        d.push(0x02); // frame1 type: audio track 0
        d.push(0x00); // tree chunk: four absent trees
        d.extend_from_slice(&[0x02, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0x00, 0x00]); // palette
        d.extend_from_slice(&[0x06, 0x00, 0x00, 0x00, 0xAA, 0x55]); // audio subchunk
        d.extend_from_slice(&[0x00, 0x00]); // video pad
        d.extend_from_slice(&[0x07, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33, 0x00]);
        d
    }

    #[test]
    fn new_decodes_frame_zero_with_its_audio_and_palette() {
        let mut p = MoviePlayer::new(&synth_stream()).unwrap();
        assert_eq!(p.frame_index(), 0);
        assert_eq!(p.pixels().len(), 16);
        assert_eq!(p.info().us_per_frame, 40_000);
        // PALMAP[1,2,3] = [0x04,0x08,0x0C] folds back to [1,2,3].
        assert_eq!(p.palette()[0], [1, 2, 3]);
        assert_eq!(p.take_audio(), vec![0xAA, 0x55]);
        assert_eq!(p.take_audio(), Vec::<u8>::new());
    }

    #[test]
    fn period_is_exact_and_boundary_decodes_on_the_whole_sub_tick() {
        // 40 ms = 9.6 sub-ticks: 9 sub-ticks must NOT decode, the 10th
        // must, and the boundary lands on the same frame regardless of
        // how the 10 sub-ticks were chunked.
        let chunked = || {
            let mut p = MoviePlayer::new(&synth_stream()).unwrap();
            p.advance(9).unwrap();
            assert_eq!(p.frame_index(), 0, "9 sub-ticks < 9.6: no decode");
            p.advance(1).unwrap();
            (p.frame_index(), p.finished())
        };
        assert_eq!(chunked(), (1, true));
        let direct = || {
            let mut p = MoviePlayer::new(&synth_stream()).unwrap();
            p.advance(10).unwrap();
            (p.frame_index(), p.finished())
        };
        assert_eq!(direct(), (1, true));
        // Fractional bursts: 4+4+4 sub-ticks (60 Hz host pacing).
        let burst = || {
            let mut p = MoviePlayer::new(&synth_stream()).unwrap();
            for _ in 0..3 {
                p.advance(4).unwrap();
            }
            (p.frame_index(), p.finished())
        };
        assert_eq!(burst(), (1, true), "12 sub-ticks = 1.25 periods: decode 1");
    }

    #[test]
    fn finished_player_holds_the_last_frame_and_ignores_dt() {
        let mut p = MoviePlayer::new(&synth_stream()).unwrap();
        while !p.finished() {
            p.advance(4).unwrap();
        }
        assert_eq!(p.frame_index(), 1);
        let pixels = p.pixels().to_vec();
        p.advance(1_000_000).unwrap();
        assert_eq!(p.frame_index(), 1);
        assert_eq!(p.pixels(), &pixels[..]);
    }

    #[test]
    fn audio_packets_join_in_decode_order() {
        let mut p = MoviePlayer::new(&synth_stream()).unwrap();
        let first = p.take_audio();
        assert_eq!(first, vec![0xAA, 0x55]);
        while !p.finished() {
            p.advance(4).unwrap();
        }
        assert_eq!(p.take_audio(), vec![0x11, 0x22, 0x33]);
        assert_eq!(p.take_audio(), Vec::<u8>::new());
    }

    #[test]
    fn title_like_period_paces_a_60_hz_host_exactly() {
        // The synth stream re-rated to the TITLE.SMK interval
        // (66_660 us, the us-per-frame encoding). At 4 sub-ticks per
        // 60 Hz pump: 3 pumps = 12_000_000 units < 15_998_400 (no
        // decode), pump 4 = 16_000_000 units (decode, bank 1_600).
        let mut d = synth_stream();
        d[16..20].copy_from_slice(&(-6666i32).to_le_bytes());
        assert_eq!(MoviePlayer::new(&d).unwrap().info().us_per_frame, 66_660);
        let mut p = MoviePlayer::new(&d).unwrap();
        for _ in 0..3 {
            p.advance(4).unwrap();
            assert_eq!(p.frame_index(), 0, "no decode before the period");
        }
        p.advance(4).unwrap();
        assert_eq!(p.frame_index(), 1, "decode lands on pump 4");
    }

    #[test]
    fn non_eligible_tracks_are_skipped_for_audio() {
        // Same stream with the rate dword moved to 22050 Hz: no
        // eligible track, so no audio is ever collected.
        let mut d = synth_stream();
        d[72..76].copy_from_slice(&(0x4000_0000u32 | 22_050).to_le_bytes());
        let mut p = MoviePlayer::new(&d).unwrap();
        assert_eq!(p.info().audio[0].unwrap().rate_hz, 22_050);
        assert_eq!(p.info().audio[0].unwrap().codec, SmkAudioCodec::Raw);
        assert_eq!(p.take_audio(), Vec::<u8>::new());
    }

    /// 3-frame RING variant of [synth_stream] (flags bit 0 set, one
    /// extra closing-slot chunk per the container: total = frames +
    /// 1). Frame 0 carries the palette + [AA 55]; frames 1, 2 and
    /// the closing slot 3 carry distinct 3-byte packets, so the
    /// wrap back to frame 1 is observable in the audio order.
    fn synth_ring_stream() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"SMK2");
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&3u32.to_le_bytes()); // frames
        d.extend_from_slice(&40u32.to_le_bytes()); // 40 ms per frame
        d.extend_from_slice(&1u32.to_le_bytes()); // flags: ring frame
        for i in 0..7u32 {
            d.extend_from_slice(&(if i == 0 { 16u32 } else { 0u32 }).to_le_bytes());
        }
        d.extend_from_slice(&1u32.to_le_bytes()); // tree chunk: 1 byte
        for _ in 0..4 {
            d.extend_from_slice(&0u32.to_le_bytes()); // tree maxima: unused
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
        d.extend_from_slice(&17u32.to_le_bytes()); // frame0 chunk
        for _ in 0..3 {
            d.extend_from_slice(&8u32.to_le_bytes()); // frames 1..=3 chunks
        }
        d.push(0x03); // frame0 type: palette + audio track 0
        d.extend(std::iter::repeat_n(0x02, 3)); // later types
        d.push(0x00); // tree chunk: four absent trees
        d.extend_from_slice(&[0x02, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0x00, 0x00]); // palette
        d.extend_from_slice(&[0x06, 0x00, 0x00, 0x00, 0xAA, 0x55]); // audio subchunk
        d.extend_from_slice(&[0x00, 0x00]); // video pad
        for b in [0x11u8, 0x22, 0x33] {
            d.extend_from_slice(&[0x07, 0x00, 0x00, 0x00, b, b, b, 0x00]);
        }
        d
    }

    #[test]
    fn ring_stream_wraps_forever_and_never_finishes() {
        // The seam contract: for a ring, Last marks the closing
        // slot of a PASS (returned every cycle), and the wrap jumps
        // to frame 1 (frame 0 is the setup frame). A ring player
        // therefore never reports finished and keeps decoding +
        // queueing audio cycle after cycle (D37: the briefing
        // backdrops; the pre-fix player froze at the first closing
        // slot).
        let mut p = MoviePlayer::new(&synth_ring_stream()).unwrap();
        assert!(p.info().ring_frame);
        assert_eq!(
            p.take_audio(),
            vec![0xAA, 0x55],
            "frame-0 packet from the open"
        );
        assert_eq!(p.advance(10).unwrap(), ());
        assert_eq!(p.frame_index(), 1);
        assert_eq!(p.take_audio(), vec![0x11, 0x11, 0x11]);
        assert_eq!(p.advance(10).unwrap(), ());
        assert_eq!(p.frame_index(), 2);
        assert_eq!(p.take_audio(), vec![0x22, 0x22, 0x22]);
        // Frame 3 = the closing slot: the decoder reports Last
        // here, but the ring has NOT ended.
        assert_eq!(p.advance(10).unwrap(), ());
        assert_eq!(p.frame_index(), 3);
        assert_eq!(p.take_audio(), vec![0x33, 0x33, 0x33]);
        assert!(!p.finished(), "a ring pass end is not a stream end");
        // The wrap: the next advance re-decodes frame 1 (audio
        // order proves it), and bursts ride across many cycles.
        assert_eq!(p.advance(10).unwrap(), ());
        assert_eq!(p.frame_index(), 1);
        assert_eq!(p.take_audio(), vec![0x11, 0x11, 0x11]);
        assert_eq!(p.advance(1000).unwrap(), ());
        assert!(!p.finished());
        assert!(p.frame_index() < 4);
    }
}
