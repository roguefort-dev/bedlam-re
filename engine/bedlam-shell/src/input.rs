//! The input adapter (P4 step 1) + the control-scheme purist axis's
//! consumer (P6, D204).
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
//! # The control-scheme axis (P6, D204)
//!
//! The [`PuristToggle::ControlScheme`] arm of the immutable
//! [`ModeConfig`] selects the INPUT MAPPING POLICY at this seam, the
//! platform/input layer (PLAN sec 6 P6 "Modern controls: WASD, 1-4
//! hotkeys, full remap, wheel zoom, gamepad; original scheme
//! selectable"):
//!
//! - **MODERN** = the [`Bindings`] table (WASD + arrows move, 1-4
//!   weapon hotkeys, Escape, Space/Enter advance), FULLY REMAPPABLE;
//!   the wheel ZOOMS (a presentation-bucket accumulator, never the
//!   sim input); a default gamepad map is live (dpad moves, South
//!   fires, East backs, Start confirms).
//! - **CLASSIC** = the ORIGINAL EXW scheme, fixed (not remappable -
//!   the original offered no rebinding): keyboard = hotkeys, volume,
//!   pause, any-key ONLY; gameplay pointing is the MOUSE
//!   [verified, RE-EXW-INPUT sec 6: no keyboard reader exists for the
//!   scroll path, Left/Right arrows dead 3-way]. Among the
//!   game-semantic slots the current `InputFrame` carries, ESC is the
//!   one original keyboard binding; the rest of the original key set
//!   (Up/Down = music volume - an un-hashed host audio action; M/Space
//!   map latch; P pause; digits 1..7 order rows) targets semantics
//!   this seam does not model yet and joins when the P2e engine-side
//!   button map lands - never invented here. The wheel and gamepad
//!   are DEAD in the classic arm (the 1996 control model is exactly
//!   KeyEvent/MouseEvent/CursorPos, RE-EXW-INPUT sec 7).
//!
//! SEAM INERTNESS (the D201 property generalized): the scheme maps
//! PHYSICAL input to the game-semantic `InputFrame` BEFORE the sim -
//! the frame is the whole contract. The same `InputFrame` yields the
//! same trajectory in both arms (the sim never sees the scheme), and
//! the arms differ only in what physical input MAPS TO. The mouse
//! path (deltas + left/right buttons) is scheme-INVARIANT: the
//! original is mouse-driven and the modern scheme keeps it.

use bedlam_core::input::InputFrame;
use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};
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
    pub const ADVANCE: u32 = 1 << 10;
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
    Advance,
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
            ShellKey::Advance => button::ADVANCE,
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
        PhysicalKey::Code(KeyCode::Space)
        | PhysicalKey::Code(KeyCode::Enter)
        | PhysicalKey::Code(KeyCode::NumpadEnter) => Some(ShellKey::Advance),
        PhysicalKey::Code(_) | PhysicalKey::Unidentified(_) => None,
    }
}

/// Translate a winit key event into (key, pressed). `None` for keys
/// the MODERN-DEFAULT table does not bind (see
/// [`map_physical_key`]); the scheme-aware path is
/// [`ControlScheme::map_key`] through [`ShellInput::set_physical_key`].
pub fn map_winit_key(event: &KeyEvent) -> Option<(ShellKey, bool)> {
    let pressed = event.state == ElementState::Pressed;
    map_physical_key(event.physical_key).map(|key| (key, pressed))
}

/// Mouse button -> `InputFrame` mouse-bit mask (bit 0 left, bit 1
/// right - the EXW `g_mouse_flags` contract, RE-EXW-INPUT sec 3).
/// Scheme-INVARIANT: the mouse path is identical in both arms (the
/// original is mouse-driven and the modern scheme keeps it);
/// middle/other buttons are unbound buttons.
pub fn map_mouse_button(b: MouseButton) -> Option<u8> {
    match b {
        MouseButton::Left => Some(1),
        MouseButton::Right => Some(2),
        _ => None,
    }
}

/// The input mapping policy one host runs under - the
/// [`PuristToggle::ControlScheme`] arm of the immutable
/// [`ModeConfig`] (P6, D204; selected with [`ControlScheme::for_mode`]).
///
/// This is a POLICY selector, exactly the shape of the timing-lock
/// axis's [`PresentPacing`](bedlam_game::PresentPacing) consumer: it
/// changes what PHYSICAL input maps to, never the game-semantic
/// `InputFrame` contract itself, so the sim trajectory stays a pure
/// function of the frames (seam inertness - the sim hash is
/// scheme-independent by construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ControlScheme {
    /// The MODERN arm (default): the remappable [`Bindings`] table
    /// (WASD + arrows move, 1-4 weapon hotkeys), wheel ZOOM, and the
    /// default gamepad map.
    #[default]
    Modern,
    /// The CLASSIC arm: the original EXW scheme, FIXED (not
    /// remappable): keyboard = hotkeys/volume/pause/any-key only,
    /// gameplay pointing is the mouse, wheel and gamepad dead
    /// (RE-EXW-INPUT secs 5-7).
    Classic,
}

impl ControlScheme {
    /// The scheme for one axis arm.
    pub const fn from_arm(arm: ToggleArm) -> ControlScheme {
        match arm {
            ToggleArm::Modern => ControlScheme::Modern,
            ToggleArm::Classic => ControlScheme::Classic,
        }
    }

    /// The scheme a host's immutable [`ModeConfig`] runs its input
    /// seam under: reads the [`PuristToggle::ControlScheme`] arm and
    /// nothing else (the other axes - timing lock - never move this
    /// consumer; axis independence is pinned in the tests).
    pub const fn for_mode(mode: ModeConfig) -> ControlScheme {
        ControlScheme::from_arm(mode.arm(PuristToggle::ControlScheme))
    }

    /// Map a physical key under this scheme. MODERN consults the
    /// caller's [`Bindings`] table (full remap); CLASSIC uses the
    /// FIXED original-scheme table and IGNORES `bindings` (the
    /// original offered no rebinding).
    pub fn map_key(self, key: PhysicalKey, bindings: &Bindings) -> Option<ShellKey> {
        match self {
            ControlScheme::Modern => bindings.get(key),
            ControlScheme::Classic => classic_key(key),
        }
    }

    /// Map an abstract gamepad control under this scheme. MODERN =
    /// the default gamepad table (dpad moves, South fires, East
    /// backs, Start confirms); CLASSIC = dead (the 1996 control model
    /// is exactly KeyEvent/MouseEvent/CursorPos, RE-EXW-INPUT sec 7 -
    /// no gamepad path exists to preserve). Analog stick->digital
    /// conversion is deliberately absent (a feel-policy decision,
    /// future modern work; never classic).
    pub fn map_gamepad(self, button: GamepadButton) -> Option<ShellKey> {
        match self {
            ControlScheme::Modern => match button {
                GamepadButton::DPadUp => Some(ShellKey::Up),
                GamepadButton::DPadDown => Some(ShellKey::Down),
                GamepadButton::DPadLeft => Some(ShellKey::Left),
                GamepadButton::DPadRight => Some(ShellKey::Right),
                GamepadButton::South => Some(ShellKey::Fire),
                GamepadButton::East => Some(ShellKey::Escape),
                GamepadButton::Start => Some(ShellKey::Advance),
                GamepadButton::West
                | GamepadButton::North
                | GamepadButton::Select
                | GamepadButton::L1
                | GamepadButton::R1 => None,
            },
            ControlScheme::Classic => None,
        }
    }

    /// Whether the mouse wheel is live under this scheme: MODERN maps
    /// it to ZOOM (a presentation-bucket accumulator consumed via
    /// [`ShellInput::take_zoom`]); CLASSIC = the 1996 control model,
    /// no wheel.
    pub const fn wheel_zooms(self) -> bool {
        matches!(self, ControlScheme::Modern)
    }
}

/// The FIXED original-scheme key table (the CLASSIC arm;
/// RE-EXW-INPUT.md is the anchor for every row):
///
/// - `Escape` -> [`ShellKey::Escape`]: the ESC edge latch
///   (004edb50) + the exit paths throughout [verified, sec 2/5].
/// - `W/A/S/D` -> None: the original binds no movement keys -
///   "keyboard = hotkeys + volume + pause + any-key-continue ONLY;
///   all gameplay pointing/scrolling is mouse" [verified, sec 6].
/// - Arrows -> None at the button level: Up/Down are the MUSIC
///   VOLUME stepper (an un-hashed host audio action, sec 5), and
///   Left/Right are DEAD [verified 3-way, sec 6].
/// - `1..4` -> None: "1-4 weapon hotkeys" is the MODERN feature
///   (PLAN sec 6); the original digits 1..7 are order-row/menu
///   hotkeys targeting semantics this seam does not model yet.
/// - `Space`/`Enter` -> None: the original Space rides the M/Space
///   map-toggle latch (004edc08), a host-scene concern; it is not
///   the modern Advance confirm.
///
/// Rows join as the P2e engine-side button map lands - never
/// invented here (the D50 never-invent rule).
fn classic_key(key: PhysicalKey) -> Option<ShellKey> {
    match key {
        PhysicalKey::Code(KeyCode::Escape) => Some(ShellKey::Escape),
        PhysicalKey::Code(_) | PhysicalKey::Unidentified(_) => None,
    }
}

/// A remappable physical-key binding table - the MODERN arm's "full
/// remap" (PLAN sec 6 P6). One physical key binds at most ONE
/// [`ShellKey`] ([`Bindings::bind`] replaces); several physical keys
/// MAY bind the same semantic (the default table binds both WASD and
/// the arrows to movement). Bounded linear scan - tables are small
/// (the default is 14 rows).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bindings {
    entries: Vec<(PhysicalKey, ShellKey)>,
}

impl Bindings {
    /// The MODERN default table (the D38 seam table, now the modern
    /// arm's data): WASD + arrows move, 1-4 weapon hotkeys, Escape
    /// opens/backs, Space/Enter/NumpadEnter confirm.
    pub fn modern_default() -> Bindings {
        use KeyCode as K;
        use ShellKey as S;
        Bindings {
            entries: vec![
                (PhysicalKey::Code(K::ArrowUp), S::Up),
                (PhysicalKey::Code(K::KeyW), S::Up),
                (PhysicalKey::Code(K::ArrowDown), S::Down),
                (PhysicalKey::Code(K::KeyS), S::Down),
                (PhysicalKey::Code(K::ArrowLeft), S::Left),
                (PhysicalKey::Code(K::KeyA), S::Left),
                (PhysicalKey::Code(K::ArrowRight), S::Right),
                (PhysicalKey::Code(K::KeyD), S::Right),
                (PhysicalKey::Code(K::Digit1), S::Weapon(1)),
                (PhysicalKey::Code(K::Digit2), S::Weapon(2)),
                (PhysicalKey::Code(K::Digit3), S::Weapon(3)),
                (PhysicalKey::Code(K::Digit4), S::Weapon(4)),
                (PhysicalKey::Code(K::Escape), S::Escape),
                (PhysicalKey::Code(K::Space), S::Advance),
                (PhysicalKey::Code(K::Enter), S::Advance),
                (PhysicalKey::Code(K::NumpadEnter), S::Advance),
            ],
        }
    }

    /// (Re)bind one physical key to a semantic, replacing that key's
    /// previous binding if any. Binding to `Weapon(n > 4)` stores the
    /// row, but the semantic has no button bit, so it maps to nothing
    /// at press time (callers drop it - same as [`ShellInput::set_key`]).
    pub fn bind(&mut self, key: PhysicalKey, to: ShellKey) {
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = to;
        } else {
            self.entries.push((key, to));
        }
    }

    /// Remove a physical key's binding (the key maps to nothing).
    pub fn unbind(&mut self, key: PhysicalKey) {
        self.entries.retain(|(k, _)| *k != key);
    }

    /// The semantic a physical key is bound to under this table.
    pub fn get(&self, key: PhysicalKey) -> Option<ShellKey> {
        self.entries
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, s)| *s)
    }
}

/// An abstract gamepad control (device-agnostic; the platform layer
/// translates its device events onto this set). The modern default
/// map is [`ControlScheme::map_gamepad`]; the classic arm maps none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    /// The bottom face button (A on Xbox layouts, Cross on PlayStation).
    South,
    /// The right face button (B / Circle).
    East,
    /// The left face button (X / Square).
    West,
    /// The top face button (Y / Triangle).
    North,
    Start,
    Select,
    L1,
    R1,
}

/// Accumulated per-pump input state. `tick()` snapshots and consumes
/// the deltas; held buttons and held mouse bits PERSIST across ticks
/// (the FSM derives edges itself - D26 hashed per-tick latches).
///
/// The accumulator carries the CONTROL-SCHEME POLICY (P6, D204):
/// `scheme` (selected from the immutable `ModeConfig` via
/// [`ControlScheme::for_mode`], default = modern) and the modern
/// arm's remappable [`Bindings`] table. The scheme lives ONLY on the
/// physical->semantic mapping boundary - [`ShellInput::tick`] still
/// emits the plain game-semantic `InputFrame`, so the sim never sees
/// it (seam inertness; the mouse path is scheme-invariant).
#[derive(Debug, Clone)]
pub struct ShellInput {
    buttons: u32,
    mouse_dx: i32,
    mouse_dy: i32,
    mouse_buttons: u8,
    scheme: ControlScheme,
    bindings: Bindings,
    zoom_steps: i32,
}

impl Default for ShellInput {
    /// Default = the MODERN scheme under its default binding table
    /// (mirrors `ModeConfig::default`), nothing held. Identical to
    /// [`ShellInput::new`]; the derived default would carry an EMPTY
    /// bindings table, which is not a playable modern mapping.
    fn default() -> ShellInput {
        ShellInput {
            buttons: 0,
            mouse_dx: 0,
            mouse_dy: 0,
            mouse_buttons: 0,
            scheme: ControlScheme::Modern,
            bindings: Bindings::modern_default(),
            zoom_steps: 0,
        }
    }
}

impl ShellInput {
    pub fn new() -> ShellInput {
        ShellInput::default()
    }

    /// The control scheme this accumulator maps physical input under
    /// (default = modern, mirroring `ModeConfig::default`).
    pub fn scheme(&self) -> ControlScheme {
        self.scheme
    }

    /// Set the control scheme (the host selects it from the immutable
    /// mode - `ControlScheme::for_mode`; classic/modern selectable at
    /// the platform level with the shell config plumbing). Does NOT
    /// touch held state: already-held semantics stay held (the scheme
    /// maps FUTURE physical events).
    pub fn with_scheme(mut self, scheme: ControlScheme) -> ShellInput {
        self.scheme = scheme;
        self
    }

    /// The modern arm's binding table (full remap; ignored by the
    /// classic arm - the original scheme is fixed).
    pub fn bindings(&self) -> &Bindings {
        &self.bindings
    }

    /// Replace the modern binding table. Ignored by the classic arm
    /// (its table is fixed by `classic_key`).
    pub fn set_bindings(&mut self, bindings: Bindings) {
        self.bindings = bindings;
    }

    /// Press or release a PHYSICAL key through the scheme (the
    /// platform path - `ShellInput::set_key` remains the
    /// semantic-level injector for tests). Unmapped keys are dropped.
    pub fn set_physical_key(&mut self, key: PhysicalKey, pressed: bool) {
        if let Some(mapped) = self.scheme.map_key(key, &self.bindings) {
            self.set_key(mapped, pressed);
        }
    }

    /// Press or release an abstract gamepad control through the
    /// scheme (modern maps the default table; classic is dead).
    pub fn gamepad_button(&mut self, button: GamepadButton, pressed: bool) {
        if let Some(mapped) = self.scheme.map_gamepad(button) {
            self.set_key(mapped, pressed);
        }
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
    /// pointer never crossed 32k in one tick). Scheme-invariant.
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

    /// Consume a wheel delta under the scheme (P6, D204): MODERN maps
    /// the wheel to ZOOM - whole `LineDelta` lines accumulate
    /// (up/in positive), a `PixelDelta` gesture counts as one step by
    /// sign, and the accumulated steps leave through
    /// [`ShellInput::take_zoom`] for the PRESENTATION layer (camera
    /// zoom is present-bucket; it NEVER enters `InputFrame` or the
    /// sim). CLASSIC = the 1996 control model, no wheel: no-op. This
    /// REPLACES the provisional D38 wheel->Up/Down menu-stepping
    /// mapping (PLAN sec 6 P6 pins the wheel as a modern zoom
    /// feature). Returns without effect for zero deltas.
    pub fn wheel(&mut self, delta: MouseScrollDelta) {
        let steps = match delta {
            MouseScrollDelta::LineDelta(_, y) => y.trunc() as i32,
            MouseScrollDelta::PixelDelta(p) => {
                if p.y > 0.0 {
                    1
                } else if p.y < 0.0 {
                    -1
                } else {
                    0
                }
            }
        };
        if self.scheme.wheel_zooms() {
            self.zoom_steps = self.zoom_steps.saturating_add(steps);
        }
    }

    /// Take the accumulated zoom steps (modern wheel zoom) and reset
    /// the accumulator - the presentation layer's consume edge. The
    /// value is presentation-bucket ONLY: nothing here can reach
    /// `InputFrame`, the sim or any hash.
    pub fn take_zoom(&mut self) -> i32 {
        core::mem::take(&mut self.zoom_steps)
    }

    /// Snapshot the accumulated state as one tick input; pointer
    /// deltas are consumed, held buttons carry over. UNCHANGED by the
    /// control-scheme axis: the frame is the game-semantic contract
    /// both schemes map onto (seam inertness - the scheme never
    /// crosses it).
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
    /// unfocused must not stick). Pending wheel zoom is dropped with
    /// the rest of the physical state.
    pub fn clear_held(&mut self) {
        self.buttons = 0;
        self.mouse_buttons = 0;
        self.mouse_dx = 0;
        self.mouse_dy = 0;
        self.zoom_steps = 0;
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
            (KeyCode::Space, ShellKey::Advance),
            (KeyCode::Enter, ShellKey::Advance),
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

    // ---------------------------------------------------------------
    // P6 control-scheme axis consumer (D204): the ONE purist toggle,
    // both arms - selection, the two tables, remap, wheel, gamepad,
    // and the seam-inertness frame pins. The OTHER axis (timing lock)
    // appears only as the axis-independence control, never a feature
    // cross-product (D200).
    // ---------------------------------------------------------------

    use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};

    /// The scheme selects from the IMMUTABLE mode's control-scheme
    /// arm; the timing-lock arm (the other axis) never moves it.
    #[test]
    fn control_scheme_selects_from_the_immutable_mode() {
        assert_eq!(
            ControlScheme::for_mode(ModeConfig::default()),
            ControlScheme::Modern,
            "default = modern (PLAN sec 6)"
        );
        assert_eq!(
            ControlScheme::for_mode(ModeConfig::CLASSIC),
            ControlScheme::Classic,
            "the classic preset carries the purist controls arm"
        );
        let purist_controls =
            ModeConfig::default().with(PuristToggle::ControlScheme, ToggleArm::Classic);
        assert_eq!(
            ControlScheme::for_mode(purist_controls),
            ControlScheme::Classic
        );
        // Axis-independence control: flipping TIMING LOCK alone must
        // not move the control-scheme consumer.
        let purist_timing =
            ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic);
        assert_eq!(
            ControlScheme::for_mode(purist_timing),
            ControlScheme::Modern
        );
        // The arm-level constructor agrees for both arms.
        assert_eq!(
            ControlScheme::from_arm(ToggleArm::Modern),
            ControlScheme::Modern
        );
        assert_eq!(
            ControlScheme::from_arm(ToggleArm::Classic),
            ControlScheme::Classic
        );
    }

    /// The MODERN arm maps through the default binding table, which
    /// is exactly the pinned `map_physical_key` seam table (the
    /// table and the fn cannot drift).
    #[test]
    fn modern_scheme_maps_the_default_bindings() {
        let scheme = ControlScheme::Modern;
        let table = Bindings::modern_default();
        for code in [
            KeyCode::ArrowUp,
            KeyCode::KeyW,
            KeyCode::ArrowDown,
            KeyCode::KeyS,
            KeyCode::ArrowLeft,
            KeyCode::KeyA,
            KeyCode::ArrowRight,
            KeyCode::KeyD,
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Escape,
            KeyCode::Space,
            KeyCode::Enter,
            KeyCode::NumpadEnter,
            KeyCode::KeyQ,
            KeyCode::F5,
        ] {
            let key = PhysicalKey::Code(code);
            assert_eq!(
                scheme.map_key(key, &table),
                map_physical_key(key),
                "modern table drifted from the pinned seam map at {code:?}"
            );
        }
        // The default accumulator IS the modern default (never an
        // empty table - the manual Default pairs them).
        let mut input = ShellInput::new();
        input.set_physical_key(PhysicalKey::Code(KeyCode::KeyW), true);
        assert_eq!(input.tick().buttons & button::UP, button::UP);
    }

    /// The CLASSIC arm is the ORIGINAL EXW scheme, fixed: keyboard =
    /// hotkeys/volume/pause/any-key ONLY, gameplay pointing is the
    /// mouse (RE-EXW-INPUT secs 5-7). Among the game-semantic slots
    /// this seam carries, ESC is the one original key binding.
    #[test]
    fn classic_scheme_is_the_original_exw_scheme() {
        let scheme = ControlScheme::Classic;
        // The table argument is IGNORED (the original is not
        // remappable) - pass the modern table deliberately.
        let table = Bindings::modern_default();
        // ESC: the one original keyboard binding the slot set carries
        // (ESC latch 004edb50 + exit paths, RE-EXW-INPUT sec 2/5).
        assert_eq!(
            scheme.map_key(PhysicalKey::Code(KeyCode::Escape), &table),
            Some(ShellKey::Escape)
        );
        // No keyboard movement in the original (sec 6 headline):
        // WASD unbound, arrows not movement (Up/Down = music volume,
        // an un-hashed host audio action; Left/Right dead 3-way).
        for code in [
            KeyCode::KeyW,
            KeyCode::KeyA,
            KeyCode::KeyS,
            KeyCode::KeyD,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
        ] {
            assert_eq!(
                scheme.map_key(PhysicalKey::Code(code), &table),
                None,
                "classic binds no movement at {code:?}"
            );
        }
        // 1-4 weapon hotkeys are the MODERN feature; the original
        // digits (1..7) are order-row/menu hotkeys targeting
        // semantics this seam does not model yet (never invented).
        for code in [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
        ] {
            assert_eq!(scheme.map_key(PhysicalKey::Code(code), &table), None);
        }
        // The original Space rides the M/Space map-toggle latch
        // (004edc08) - a host-scene concern, not the modern Advance.
        for code in [KeyCode::Space, KeyCode::Enter, KeyCode::NumpadEnter] {
            assert_eq!(scheme.map_key(PhysicalKey::Code(code), &table), None);
        }
        // Everything else stays unbound, as in the original.
        assert_eq!(
            scheme.map_key(PhysicalKey::Code(KeyCode::KeyQ), &table),
            None
        );
        assert_eq!(
            scheme.map_key(
                PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
                &table
            ),
            None
        );
    }

    /// The mouse path is scheme-INVARIANT: the original is
    /// mouse-driven (RE-EXW-INPUT sec 6) and the modern scheme keeps
    /// it - deltas and buttons map identically in both arms.
    #[test]
    fn the_mouse_path_is_scheme_invariant() {
        for scheme in [ControlScheme::Modern, ControlScheme::Classic] {
            assert_eq!(map_mouse_button(MouseButton::Left), Some(1));
            assert_eq!(map_mouse_button(MouseButton::Right), Some(2));
            let mut input = ShellInput::new().with_scheme(scheme);
            input.mouse_move(4, -6);
            input.set_mouse(1, true);
            assert_eq!(
                input.tick(),
                InputFrame {
                    buttons: 0,
                    mouse_dx: 4,
                    mouse_dy: -6,
                    mouse_buttons: 1,
                }
            );
        }
    }

    /// Full remap is a MODERN feature: `bind` replaces a physical
    /// key's row, `unbind` drops it, several keys may share one
    /// semantic - and the CLASSIC arm ignores the table entirely.
    #[test]
    fn modern_bindings_rebind_and_classic_ignores_them() {
        let modern = ControlScheme::Modern;
        let classic = ControlScheme::Classic;
        let mut table = Bindings::modern_default();
        // Rebind Q (unbound) to Up: now BOTH W and Q move up.
        table.bind(PhysicalKey::Code(KeyCode::KeyQ), ShellKey::Up);
        assert_eq!(
            modern.map_key(PhysicalKey::Code(KeyCode::KeyQ), &table),
            Some(ShellKey::Up)
        );
        assert_eq!(
            modern.map_key(PhysicalKey::Code(KeyCode::KeyW), &table),
            Some(ShellKey::Up)
        );
        // Unbind W: Q alone moves up.
        table.unbind(PhysicalKey::Code(KeyCode::KeyW));
        assert_eq!(
            modern.map_key(PhysicalKey::Code(KeyCode::KeyW), &table),
            None
        );
        // A physical key holds at most ONE binding (bind replaces).
        table.bind(PhysicalKey::Code(KeyCode::KeyQ), ShellKey::Down);
        assert_eq!(
            modern.map_key(PhysicalKey::Code(KeyCode::KeyQ), &table),
            Some(ShellKey::Down)
        );
        // The classic arm IGNORES the remapped table: Q stays dead
        // and ESC stays bound regardless of the modern table.
        assert_eq!(
            classic.map_key(PhysicalKey::Code(KeyCode::KeyQ), &table),
            None
        );
        assert_eq!(
            classic.map_key(PhysicalKey::Code(KeyCode::Escape), &table),
            Some(ShellKey::Escape)
        );
        // The accumulator exposes the table (set/replace round trip).
        let mut input = ShellInput::new();
        assert_eq!(input.bindings(), &Bindings::modern_default());
        input.set_bindings(table.clone());
        assert_eq!(input.bindings(), &table);
        input.set_physical_key(PhysicalKey::Code(KeyCode::KeyQ), true);
        assert_eq!(input.tick().buttons & button::DOWN, button::DOWN);
    }

    /// The wheel ZOOMS in the modern arm (whole lines, pixel
    /// gestures by sign), the accumulator is consumed exactly once,
    /// and the zoom NEVER rides the sim input; the classic arm is the
    /// 1996 control model - no wheel, no-op.
    #[test]
    fn wheel_zooms_in_modern_and_is_dead_in_classic() {
        let mut modern = ShellInput::new();
        assert!(modern.scheme().wheel_zooms());
        modern.wheel(MouseScrollDelta::LineDelta(0.0, 2.0));
        assert_eq!(modern.take_zoom(), 2);
        assert_eq!(modern.take_zoom(), 0, "consumed exactly once");
        modern.wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
        modern.wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0.0, -40.0),
        ));
        assert_eq!(modern.take_zoom(), -2, "-1 line and one pixel gesture");
        // Presentation-bucket ONLY: a pending zoom never reaches the
        // InputFrame (the sim-input contract is scheme-blind).
        modern.wheel(MouseScrollDelta::LineDelta(0.0, 5.0));
        assert_eq!(modern.tick(), InputFrame::default());
        assert_eq!(modern.take_zoom(), 5, "the tick does not consume zoom");
        // CLASSIC: dead (the 1996 control model has no wheel).
        let mut classic = ShellInput::new().with_scheme(ControlScheme::Classic);
        assert!(!classic.scheme().wheel_zooms());
        classic.wheel(MouseScrollDelta::LineDelta(0.0, 3.0));
        classic.wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0.0, 90.0),
        ));
        assert_eq!(classic.take_zoom(), 0);
        assert_eq!(classic.tick(), InputFrame::default());
    }

    /// The default gamepad map is live in the modern arm and dead in
    /// the classic arm (the 1996 control model is exactly
    /// KeyEvent/MouseEvent/CursorPos, RE-EXW-INPUT sec 7).
    #[test]
    fn gamepad_maps_in_modern_and_is_dead_in_classic() {
        let modern = ControlScheme::Modern;
        let classic = ControlScheme::Classic;
        assert_eq!(
            modern.map_gamepad(GamepadButton::DPadUp),
            Some(ShellKey::Up)
        );
        assert_eq!(
            modern.map_gamepad(GamepadButton::DPadDown),
            Some(ShellKey::Down)
        );
        assert_eq!(
            modern.map_gamepad(GamepadButton::DPadLeft),
            Some(ShellKey::Left)
        );
        assert_eq!(
            modern.map_gamepad(GamepadButton::DPadRight),
            Some(ShellKey::Right)
        );
        assert_eq!(
            modern.map_gamepad(GamepadButton::South),
            Some(ShellKey::Fire)
        );
        assert_eq!(
            modern.map_gamepad(GamepadButton::East),
            Some(ShellKey::Escape)
        );
        assert_eq!(
            modern.map_gamepad(GamepadButton::Start),
            Some(ShellKey::Advance)
        );
        for pad in [
            GamepadButton::West,
            GamepadButton::North,
            GamepadButton::Select,
            GamepadButton::L1,
            GamepadButton::R1,
        ] {
            assert_eq!(
                modern.map_gamepad(pad),
                None,
                "{pad:?} unbound in the default"
            );
            assert_eq!(classic.map_gamepad(pad), None, "{pad:?} dead in classic");
        }
        // Through the accumulator: modern arms FIRE, classic drops it.
        let mut m = ShellInput::new();
        let mut c = ShellInput::new().with_scheme(ControlScheme::Classic);
        m.gamepad_button(GamepadButton::South, true);
        c.gamepad_button(GamepadButton::South, true);
        assert_eq!(m.tick().buttons & button::FIRE, button::FIRE);
        assert_eq!(c.tick().buttons, 0);
    }

    /// The seam-inertness frame pin (the D201 property generalized to
    /// the input seam): the SAME physical stream maps to DIFFERENT
    /// game-semantic frames per arm where the schemes differ, while
    /// the shared path (mouse, ESC) maps identically - the InputFrame
    /// is the whole contract and the scheme never crosses it. The
    /// consumer is REAL, not inert.
    #[test]
    fn the_same_physical_stream_maps_differently_per_arm() {
        let code = |c: KeyCode| PhysicalKey::Code(c);
        // A W-hold + pointer-move + fire-click pump, both arms.
        let mut modern = ShellInput::new();
        let mut classic = ShellInput::new().with_scheme(ControlScheme::Classic);
        for input in [&mut modern, &mut classic] {
            input.set_physical_key(code(KeyCode::KeyW), true);
            input.set_physical_key(code(KeyCode::Digit2), true);
            input.mouse_move(3, -1);
            input.set_mouse(1, true);
        }
        let fm = modern.tick();
        let fc = classic.tick();
        // Modern: W -> UP (the placeholder sim payload consumes bit 0,
        // so this bit is hash-visible), 2 -> WEAPON2, click rides.
        assert_eq!(fm.buttons, button::UP | button::WEAPON2);
        assert_eq!(fm.mouse_dx, 3);
        assert_eq!(fm.mouse_dy, -1);
        assert_eq!(fm.mouse_buttons, 1);
        // Classic: the SAME physical keys map to movement-neutral
        // frames - the mouse path is the only gameplay input.
        assert_eq!(
            fc.buttons, 0,
            "no keyboard semantics in the original scheme"
        );
        assert_eq!(fc.mouse_dx, 3, "the mouse path is scheme-invariant");
        assert_eq!(fc.mouse_dy, -1);
        assert_eq!(fc.mouse_buttons, 1);
        // The one shared key binding: ESC escapes in BOTH arms.
        let mut modern = ShellInput::new();
        let mut classic = ShellInput::new().with_scheme(ControlScheme::Classic);
        for input in [&mut modern, &mut classic] {
            input.set_physical_key(code(KeyCode::Escape), true);
        }
        assert_eq!(modern.tick().buttons, button::ESCAPE);
        assert_eq!(classic.tick().buttons, button::ESCAPE);
    }

    /// Focus loss drops pending zoom with the rest of the physical
    /// state (an unfocused wheel gesture must not fire later).
    #[test]
    fn clear_held_drops_pending_zoom() {
        let mut input = ShellInput::new();
        input.wheel(MouseScrollDelta::LineDelta(0.0, 4.0));
        input.clear_held();
        assert_eq!(input.take_zoom(), 0);
    }
}
