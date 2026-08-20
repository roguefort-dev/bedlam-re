//! The input adapter skeleton (P4 step 1).
//!
//! Winit key/mouse events accumulate into a
//! [`bedlam_core::input::InputFrame`] once per host pump. The button
//! BIT LAYOUT below is the SHELL layout - ours, provisional (D38) -
//! because the engine-side `buttons` bit assignment is still pending
//! the P2e input RE; the EXW scan-code keystore itself is mapped in
//! docs/RE-EXW-INPUT.md and binds a different mechanism (12 edge
//! latches, arrows +0x80 remap). When P2e lands, this module shrinks
//! to a pure winit->engine-event translator and the layout moves into
//! bedlam-core.
//!
//! Mapping shape (modern defaults per PLAN sec 6 P6): WASD + arrows
//! move, mouse aims, left button fires, 1-4 weapon hotkeys, Escape
//! opens/backs. Original-scheme rebinding is future P6 work; this
//! skeleton only pins the translation SEAM, not the bindings.

use bedlam_core::input::InputFrame;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Shell button bits (PROVISIONAL layout, D38 - not the EXW keystore).
pub mod button {
    pub const UP: u32 = 1 << 0;
    pub const DOWN: u32 = 1 << 1;
    pub const LEFT: u32 = 1 << 2;
    pub const RIGHT: u32 = 1 << 3;
    pub const FIRE: u32 = 1 << 4;
    pub const WEAPON1: u32 = 1 << 5;
    pub const WEAPON2: u32 = 1 << 6;
    pub const WEAPON3: u32 = 1 << 7;
    pub const WEAPON4: u32 = 1 << 8;
    pub const ESCAPE: u32 = 1 << 9;
}

/// A logical shell key (the winit-independent half of the seam).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKey {
    Up,
    Down,
    Left,
    Right,
    Fire,
    Weapon(u8),
    Escape,
}

impl ShellKey {
    /// The shell bit for this key (see [`button`]).
    pub fn bit(self) -> u32 {
        match self {
            ShellKey::Up => button::UP,
            ShellKey::Down => button::DOWN,
            ShellKey::Left => button::LEFT,
            ShellKey::Right => button::RIGHT,
            ShellKey::Fire => button::FIRE,
            ShellKey::Weapon(1) => button::WEAPON1,
            ShellKey::Weapon(2) => button::WEAPON2,
            ShellKey::Weapon(3) => button::WEAPON3,
            ShellKey::Weapon(4) => button::WEAPON4,
            // Not a shell key: maps to nothing (callers drop it).
            ShellKey::Weapon(_) => 0,
            ShellKey::Escape => button::ESCAPE,
        }
    }
}

/// The pure physical-key half of the seam. Only physical keys are
/// mapped (KeyCode, layout-independent - the 1996 control scheme is
/// positional). Unit-pinned below; the KeyEvent wrapper is exercised
/// by the corpus-gated shell smoke because winit's `KeyEvent` carries
/// a `pub(crate) platform_specific` field and cannot be constructed
/// outside winit.
pub fn map_physical_key(key: PhysicalKey) -> Option<ShellKey> {
    match key {
        PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::KeyW) => {
            Some(ShellKey::Up)
        }
        PhysicalKey::Code(KeyCode::ArrowDown) | PhysicalKey::Code(KeyCode::KeyS) => {
            Some(ShellKey::Down)
        }
        PhysicalKey::Code(KeyCode::ArrowLeft) | PhysicalKey::Code(KeyCode::KeyA) => {
            Some(ShellKey::Left)
        }
        PhysicalKey::Code(KeyCode::ArrowRight) | PhysicalKey::Code(KeyCode::KeyD) => {
            Some(ShellKey::Right)
        }
        PhysicalKey::Code(KeyCode::Digit1) => Some(ShellKey::Weapon(1)),
        PhysicalKey::Code(KeyCode::Digit2) => Some(ShellKey::Weapon(2)),
        PhysicalKey::Code(KeyCode::Digit3) => Some(ShellKey::Weapon(3)),
        PhysicalKey::Code(KeyCode::Digit4) => Some(ShellKey::Weapon(4)),
        PhysicalKey::Code(KeyCode::Escape) => Some(ShellKey::Escape),
        PhysicalKey::Code(_) | PhysicalKey::Unidentified(_) => None,
    }
}

/// Translate a winit key event into (key, pressed). `None` for keys
/// the shell does not bind. See [`map_physical_key`] for the pinned
/// mapping table.
pub fn map_winit_key(event: &KeyEvent) -> Option<(ShellKey, bool)> {
    let pressed = event.state == ElementState::Pressed;
    map_physical_key(event.physical_key).map(|key| (key, pressed))
}
/// unbound buttons.
pub fn map_mouse_button(b: MouseButton) -> Option<u8> {
    match b {
        MouseButton::Left => Some(1),
        MouseButton::Right => Some(2),
        _ => None,
    }
}

/// Accumulated per-pump input state. `tick()` snapshots and consumes
/// the deltas; held buttons and held mouse bits PERSIST across ticks
/// (the FSM derives edges itself - D26 hashed per-tick latches).
#[derive(Debug, Default, Clone)]
pub struct ShellInput {
    buttons: u32,
    mouse_dx: i32,
    mouse_dy: i32,
    mouse_buttons: u8,
}

impl ShellInput {
    pub fn new() -> ShellInput {
        ShellInput::default()
    }

    /// Press or release a shell key (idempotent per bit).
    pub fn set_key(&mut self, key: ShellKey, pressed: bool) {
        let bit = key.bit();
        if bit != 0 {
            if pressed {
                self.buttons |= bit;
            } else {
                self.buttons &= !bit;
            }
        }
    }

    /// Accumulate pointer motion (window pixels; clamped to the
    /// i16 InputFrame range at the edges, saturating - a 1996 game
    /// pointer never crossed 32k in one tick).
    pub fn mouse_move(&mut self, dx: i32, dy: i32) {
        self.mouse_dx = self.mouse_dx.saturating_add(dx);
        self.mouse_dy = self.mouse_dy.saturating_add(dy);
    }

    /// Press or release a mouse button by mask bit.
    pub fn set_mouse(&mut self, mask: u8, pressed: bool) {
        if pressed {
            self.mouse_buttons |= mask;
        } else {
            self.mouse_buttons &= !mask;
        }
    }

    /// Consume wheel deltas as Up/Down presses for one tick (the
    /// 1996 build had no wheel; the shell maps it to menu stepping
    /// provisionally). Returns without effect for `PixelDelta` on
    /// non-line-scroll devices.
    pub fn wheel(&mut self, delta: MouseScrollDelta) {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(p) => {
                if p.y > 0.0 {
                    1.0
                } else if p.y < 0.0 {
                    -1.0
                } else {
                    0.0
                }
            }
        };
        if lines > 0.0 {
            self.set_key(ShellKey::Up, true);
        } else if lines < 0.0 {
            self.set_key(ShellKey::Down, true);
        }
    }

    /// Snapshot the accumulated state as one tick input; pointer
    /// deltas are consumed, held buttons carry over.
    pub fn tick(&mut self) -> InputFrame {
        let dx = self.mouse_dx.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let dy = self.mouse_dy.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        self.mouse_dx = 0;
        self.mouse_dy = 0;
        InputFrame {
            buttons: self.buttons,
            mouse_dx: dx,
            mouse_dy: dy,
            mouse_buttons: self.mouse_buttons,
        }
    }

    /// Release everything (window focus loss - a held key while
    /// unfocused must not stick).
    pub fn clear_held(&mut self) {
        self.buttons = 0;
        self.mouse_buttons = 0;
        self.mouse_dx = 0;
        self.mouse_dy = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mapping table is total over the shell keys: every bound
    /// KeyCode maps to the expected ShellKey, everything else None.
    /// (Pins `map_physical_key` directly - winit's KeyEvent carries a
    /// `pub(crate) platform_specific` field and cannot be constructed
    /// outside winit, so the thin KeyEvent wrapper is covered by the
    /// corpus-gated shell smoke instead.)
    #[test]
    fn key_map_pins_the_shell_bindings() {
        let cases = [
            (KeyCode::ArrowUp, ShellKey::Up),
            (KeyCode::KeyW, ShellKey::Up),
            (KeyCode::ArrowDown, ShellKey::Down),
            (KeyCode::KeyS, ShellKey::Down),
            (KeyCode::ArrowLeft, ShellKey::Left),
            (KeyCode::KeyA, ShellKey::Left),
            (KeyCode::ArrowRight, ShellKey::Right),
            (KeyCode::KeyD, ShellKey::Right),
            (KeyCode::Digit1, ShellKey::Weapon(1)),
            (KeyCode::Digit2, ShellKey::Weapon(2)),
            (KeyCode::Digit3, ShellKey::Weapon(3)),
            (KeyCode::Digit4, ShellKey::Weapon(4)),
            (KeyCode::Escape, ShellKey::Escape),
        ];
        for (code, want) in cases {
            assert_eq!(
                map_physical_key(PhysicalKey::Code(code)),
                Some(want),
                "{code:?}"
            );
        }
        // Unbound keys are dropped, as are unidentified ones.
        assert_eq!(map_physical_key(PhysicalKey::Code(KeyCode::KeyQ)), None);
        assert_eq!(map_physical_key(PhysicalKey::Code(KeyCode::Space)), None);
        assert_eq!(
            map_physical_key(PhysicalKey::Unidentified(
                winit::keyboard::NativeKeyCode::Unidentified
            )),
            None
        );
    }

    /// Mouse buttons map onto the InputFrame bit contract (bit 0
    /// left, bit 1 right); middle/other buttons are unbound.
    #[test]
    fn mouse_button_map() {
        assert_eq!(map_mouse_button(MouseButton::Left), Some(1));
        assert_eq!(map_mouse_button(MouseButton::Right), Some(2));
        assert_eq!(map_mouse_button(MouseButton::Middle), None);
        assert_eq!(map_mouse_button(MouseButton::Back), None);
    }

    /// Press/hold/release through ticks: buttons persist while held,
    /// deltas are consumed per tick, edges belong to the FSM.
    #[test]
    fn tick_snapshots_and_consumes() {
        let mut input = ShellInput::new();
        // Idle tick: the neutral frame.
        assert_eq!(
            input.tick(),
            InputFrame {
                buttons: 0,
                mouse_dx: 0,
                mouse_dy: 0,
                mouse_buttons: 0
            }
        );
        // Press W, move, click.
        input.set_key(ShellKey::Up, true);
        input.mouse_move(5, -7);
        input.set_mouse(1, true);
        assert_eq!(
            input.tick(),
            InputFrame {
                buttons: button::UP,
                mouse_dx: 5,
                mouse_dy: -7,
                mouse_buttons: 1
            }
        );
        // Held state persists, deltas do not accumulate twice.
        assert_eq!(
            input.tick(),
            InputFrame {
                buttons: button::UP,
                mouse_dx: 0,
                mouse_dy: 0,
                mouse_buttons: 1
            }
        );
        // Release all.
        input.set_key(ShellKey::Up, false);
        input.set_mouse(1, false);
        assert_eq!(input.tick().buttons, 0);
        assert_eq!(input.tick().mouse_buttons, 0);
    }

    /// Pointer deltas saturate at the i16 rails instead of wrapping.
    #[test]
    fn mouse_deltas_saturate() {
        let mut input = ShellInput::new();
        input.mouse_move(40_000, -40_000);
        let f = input.tick();
        assert_eq!(f.mouse_dx, i16::MAX);
        assert_eq!(f.mouse_dy, i16::MIN);
        // The clamp consumed the whole bank, not just the rail.
        assert_eq!(input.tick().mouse_dx, 0);
    }

    /// Focus loss clears held state so a key held across an
    /// alt-tab cannot stick forever.
    #[test]
    fn clear_held_resets_everything() {
        let mut input = ShellInput::new();
        input.set_key(ShellKey::Fire, true);
        input.set_mouse(2, true);
        input.mouse_move(3, 3);
        input.clear_held();
        assert_eq!(input.tick(), InputFrame::default());
    }

    /// Every ShellKey bit is distinct and inside the u32 mask; the
    /// Weapon(n>4) arm is a no-op (0) by construction.
    #[test]
    fn button_bits_are_disjoint() {
        let bits = [
            ShellKey::Up.bit(),
            ShellKey::Down.bit(),
            ShellKey::Left.bit(),
            ShellKey::Right.bit(),
            ShellKey::Fire.bit(),
            ShellKey::Weapon(1).bit(),
            ShellKey::Weapon(2).bit(),
            ShellKey::Weapon(3).bit(),
            ShellKey::Weapon(4).bit(),
            ShellKey::Escape.bit(),
        ];
        for (i, a) in bits.iter().enumerate() {
            assert_ne!(*a, 0);
            for b in &bits[i + 1..] {
                assert_eq!(a & b, 0, "bits overlap: {a:b} {b:b}");
            }
        }
        assert_eq!(ShellKey::Weapon(5).bit(), 0);
    }
}
