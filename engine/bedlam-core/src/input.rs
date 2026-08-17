//! Per-tick deterministic input frames.
//!
//! Determinism Charter (docs/PLAN.md sec 7): all original entropy/timing
//! reads are modeled as deterministic per-tick inputs; a replay is exactly a
//! seed plus the ordered list of these frames.

/// Encoded size of one [`InputFrame`]: fixed 12-byte little-endian layout
/// (buttons u32, mouse_dx i16, mouse_dy i16, mouse_buttons u8, 3 pad bytes).
pub const ENCODED_LEN: usize = 12;

/// One tick's worth of sampled input, consumed by `Sim::tick`.
///
/// Bit assignments:
/// - `buttons`: keyboard bit assignment deliberately UNASSIGNED pending the
///   P2e input RE. Ground truth so far: in EXW the keyboard serves
///   hotkeys/volume/pause/any-key only and gameplay pointing is the mouse
///   (docs/RE-EXW-INPUT.md) — the real button map will be anchored there.
/// - `mouse_buttons`: bit 0 = left, bit 1 = right (EXW `g_mouse_flags`
///   @004dc6e4, docs/RE-EXW-INPUT.md).
/// - `mouse_dx` / `mouse_dy`: per-tick pointer deltas in 640x480 game
///   space (EXW `CursorToGame`, docs/RE-EXW-TICK).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct InputFrame {
    /// Keyboard button bitmask; bit assignment pending P2e input RE.
    pub buttons: u32,
    /// Horizontal pointer delta for this tick (game space).
    pub mouse_dx: i16,
    /// Vertical pointer delta for this tick (game space).
    pub mouse_dy: i16,
    /// Mouse button bitmask: bit 0 = left, bit 1 = right.
    pub mouse_buttons: u8,
}

impl InputFrame {
    /// Encode into `out` (little-endian, fixed layout):
    /// buttons u32 @0, mouse_dx i16 @4, mouse_dy i16 @6, mouse_buttons u8
    /// @8, zero pad @9..12.
    ///
    /// Precondition (engine, debug-asserted): `out.len() >= ENCODED_LEN`.
    /// The replay/snapshot parsers bounds-check before calling.
    pub fn to_bytes(&self, out: &mut [u8]) {
        debug_assert!(out.len() >= ENCODED_LEN, "input buffer too small");
        out[0..4].copy_from_slice(&self.buttons.to_le_bytes());
        out[4..6].copy_from_slice(&self.mouse_dx.to_le_bytes());
        out[6..8].copy_from_slice(&self.mouse_dy.to_le_bytes());
        out[8] = self.mouse_buttons;
        out[9..ENCODED_LEN].fill(0);
    }

    /// Decode from the 12-byte little-endian layout (see [`Self::to_bytes`]).
    ///
    /// Lenient by design: pad bytes are ignored and this performs no bounds
    /// policy of its own — the replay parser does the checks. Precondition
    /// (engine, debug-asserted): `bytes.len() >= ENCODED_LEN`.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= ENCODED_LEN, "input slice too small");
        InputFrame {
            buttons: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            mouse_dx: i16::from_le_bytes([bytes[4], bytes[5]]),
            mouse_dy: i16::from_le_bytes([bytes[6], bytes[7]]),
            mouse_buttons: bytes[8],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_including_negative_deltas() {
        let frame = InputFrame {
            buttons: 0x8000_0001,
            mouse_dx: -300,
            mouse_dy: -1,
            mouse_buttons: 0b11,
        };
        let mut buf = [0xFFu8; ENCODED_LEN];
        frame.to_bytes(&mut buf);
        assert_eq!(InputFrame::from_bytes(&buf), frame);
    }

    #[test]
    fn exact_byte_layout() {
        let frame = InputFrame {
            buttons: 0x0302_0100,
            mouse_dx: 0x0504,
            mouse_dy: 0x0706,
            mouse_buttons: 0x08,
        };
        let mut buf = [0xAAu8; ENCODED_LEN];
        frame.to_bytes(&mut buf);
        assert_eq!(
            buf,
            [
                0x00, 0x01, 0x02, 0x03, // buttons LE
                0x04, 0x05, // mouse_dx LE
                0x06, 0x07, // mouse_dy LE
                0x08, // mouse_buttons
                0x00, 0x00, 0x00 // pad, always zeroed on encode
            ]
        );
    }
}
