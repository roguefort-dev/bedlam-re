//! The EXW title menu (P4, D41/D42): NameEntryScreen@0043a5fc's menu
//! model, strip hit test, bottom-anchored draw and item actions, as a
//! presentation-only flow (D17 bucket b: staging and the whole
//! interactive loop never touch the scene hash - the FSM is fed
//! NEUTRAL frames while the menu owns the Title input path, D42.1).
//!
//! Provenance: docs/RE-EXW-TITLEMENU.md (Ghidra -process pass, raw
//! dump ghidra-project/exw-titlemenu.txt) + corpus decode of
//! LANGUAGE.ENG [MENU_ITEMS] and the EXW string literals (PE section
//! mapping, D42). Every constant carries its anchor; engine-side
//! choices the RE does not decide are logged as D42 items.
//!
//! EXW facts this module reproduces [all verified unless tagged]:
//! - Builder FUN_00445b5c(id): menus 1/2/3/5 with the item tables
//!   below; count word 004eabd2 + 7 slots @004eabd4 stride 0x30.
//! - Draw FUN_0044653a: row_base = 0x1d6 - count*0x18 (bottom
//!   anchored), item i at row_base + i*0x18, glyph base 0x82 for
//!   i == sel (green FULLPAL set) else 0 (blue set), centered rows.
//! - Hit test 0043a934..0043a996: x in (0xdc, 0x1a4), y in
//!   (top, 0x1d6), index = (y - top)/0x18 clamped to [0, count).
//! - Hover change -> SFX MENU1, click -> SFX MENU2, both debounced
//!   4 ticks (local_b8); hover/click reset the idle counter 0046cbec.
//! - Idle >= 0x300 -> TITLE.SMK attract replay (skippable - the ONLY
//!   skippable movie, gate 004edbc4).
//! - Menu 1 dispatch: start (score seed 4000 - difficulty*500),
//!   saved-game menu, difficulty cycle (d+1) mod 3, name entry,
//!   HOF / credits, quit-confirm; click outside the strip -> the
//!   multiplayer menu with players reset to 2.
//! - Menu 2: coop / head2head (stubs here), players item cycled by
//!   button bit (left ++ / right --, wrap 2..=12), Main Menu.
//! - Menu 3: 5 save slots + Cancel (slots hold the "EMPTY" literal
//!   0x45980f when no save exists - the whole corpus state here).
//! - Menu 5: Quit to Windows (accepted) / Main Menu.
//! - Name entry: "Name:"(31) + " "(0x459779) + the 8-char buffer,
//!   cursor = bank entry 0x8e at 0x146 + (width("Name: ")+
//!   width(name))/2 on the item-3 row, blink while (frame & 0xc) != 0,
//!   empty-on-exit defaults to "GOD" (0x459078) [inferred].

use bedlam_core::frame::{
    CURSOR_BOOT_X, CURSOR_BOOT_Y, CURSOR_MAX_X, CURSOR_MAX_Y, CURSOR_MIN_X, CURSOR_MIN_Y,
};
use bedlam_core::input::InputFrame;
use bedlam_render::Vga6;

use crate::font::LoadingFont;
use crate::loading::Plane;
use crate::GameError;

/// Hit-strip x bounds, exclusive [verified: 0xdc < g_cursor_x <
/// 0x1a4].
///
/// Cursor-box audit [D160 P2e package]: the strip region
/// (0xdc,0x1a4)x(top,0x1d6) sits inside the reachable cursor box
/// [9,631]x[9,463] — x (220,420) well inside; top ≥ 0x1d6−7·0x18 =
/// 302, above the box floor 9; and 0x1d6 (470) is EXCLUSIVE while
/// the cursor max y is 463, so every strip row stays hoverable
/// under the box clamp (at y=463 the row index (463−302)/0x18 = 6
/// = the LAST row of a count-7 strip — no row is cut off).
pub const STRIP_X_MIN: i32 = 0xdc;
pub const STRIP_X_MAX: i32 = 0x1a4;
/// Hit-strip bottom = the bottom anchor row, exclusive [verified:
/// g_cursor_y < 0x1d6]. 0x1d6 = 470 > CURSOR_MAX_Y (463): the
/// exclusive bound is unreachable tail, not a lost row (see the
/// STRIP_X_MIN audit note).
pub const STRIP_Y_MAX: i32 = 0x1d6;
/// Menu row height [verified: (y - top)/0x18 and i*0x18].
pub const ROW_H: i32 = 0x18;
/// Slot capacity [verified: 7 slots @004eabd4 stride 0x30].
pub const SLOTS: usize = 7;
/// Attract idle threshold [verified: 0046cbec >= 0x300].
pub const ATTRACT_IDLE: u32 = 0x300;
/// SFX debounce [verified: local_b8 = 4 ticks].
pub const SFX_DEBOUNCE: u8 = 4;
/// Mixer instrument of MENU1.RAW (hover) [D42.7: outside the music
/// instrument domain].
pub const SFX_HOVER: u16 = 0xE0;
/// Mixer instrument of MENU2.RAW (click) [D42.7].
pub const SFX_CLICK: u16 = 0xE1;
/// Unity playback ratio (0x10000 = the wave's own 11025 Hz).
pub const SFX_RATIO_UNITY: u32 = 0x1_0000;
/// Unity note volume (48 = full at master 127, the EXW 0x30 divisor).
pub const SFX_VOLUME: u8 = 48;
/// The menu SFX pair in EXW fetch order [verified: SfxLoad
/// "SOUND\SFX\MENU1.RAW" then MENU2.RAW at screen entry].
pub const MENU_SFX_NAMES: [&str; 2] = ["MENU1.RAW", "MENU2.RAW"];
/// Name-entry cursor bank entry [verified: glyph entry 0x8e - the
/// 0x82-base set's slot for char 0x2d].
pub const CURSOR_CHAR: u8 = 0x2d;
/// Name-entry cursor pen base [verified: x = .../2 + 0x146].
pub const CURSOR_X_BASE: i32 = 0x146;
/// Cursor blink mask [verified: shown while (g_frame_count & 0xc)
/// != 0 - 12 of every 16 ticks].
pub const BLINK_MASK: u64 = 0xc;
/// Player-name capacity [verified: 8-char buffer @004e444c].
pub const NAME_MAX: usize = 8;
/// Default name on empty name-entry exit [inferred: FUN_0044efb3
/// (name, 0x459078) - the literal is "GOD"].
pub const DEFAULT_NAME: &[u8] = b"GOD";
/// Empty save-slot placeholder [verified: literal 0x45980f].
pub const EMPTY_SLOT: &[u8] = b"EMPTY";
/// "Name:" row join [verified: literal 0x459779 = " "].
pub const NAME_SEP: &[u8] = b" ";
/// Single-player score seed at difficulty d [verified: DAT_0046ae70
/// = 4000 - difficulty*500].
pub fn start_score(difficulty: u8) -> i32 {
    4000 - i32::from(difficulty) * 500
}

/// Which menu is built (EXW FUN_00445b5c ids; 4 is the multiplayer
/// load variant, never built by this slice).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuId {
    Main = 1,
    Multi = 2,
    Load = 3,
    QuitConfirm = 5,
}

/// Menu interaction phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPhase {
    /// The strip is live (hover / click / idle counting).
    Interactive,
    /// The name-entry sub-loop owns input (no hover, no idle).
    NameEntry,
    /// The attract replay is playing; only the skip is live.
    Attract,
}

/// What one menu tick wants the host to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// Menu-1 item 0 accepted: exit to the game with the score seed
    /// (4000 - difficulty*500).
    Start { score: i32 },
    /// Menu-5 item 0 accepted: quit.
    Quit,
    /// Idle hit 0x300: replay the title movie (skippable).
    Attract,
    /// Input during the replay: skip it.
    SkipAttract,
}

/// Per-tick menu report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuTick {
    /// Selection changed: play MENU1.
    pub hover_sfx: bool,
    /// A click dispatched: play MENU2.
    pub click_sfx: bool,
    /// A host-level action, if any.
    pub action: Option<MenuAction>,
}

/// The strip hit test [verified asm 0043a934..0043a996]: x strictly
/// inside (0xdc, 0x1a4), y strictly inside (top, 0x1d6) where
/// top = 0x1d6 - count*0x18; index = (y - top)/0x18 clamped to
/// [0, count). Outside -> -1.
pub fn hit(count: usize, x: i32, y: i32) -> i8 {
    let count = count as i32;
    let top = STRIP_Y_MAX - count * ROW_H;
    if x <= STRIP_X_MIN || x >= STRIP_X_MAX || y <= top || y >= STRIP_Y_MAX {
        return -1;
    }
    ((y - top) / ROW_H).clamp(0, count - 1) as i8
}

/// The title menu: model + draw plane. Built from the corpus assets
/// (LANGUAGE text, FULLFONT bank at both glyph bases, FULLPAL ramp)
/// by the host's staging call; ticked once per EXECUTED sim tick
/// with the same pending input the sim saw.
#[derive(Debug)]
pub struct TitleMenu {
    /// The [MENU_ITEMS] table, EXW order (96 entries).
    items: Vec<Vec<u8>>,
    /// Selected (green) glyph set, base 0x82.
    green: LoadingFont,
    /// Unselected (blue) glyph set, base 0.
    blue: LoadingFont,
    /// FULLPAL ramp (DAC 224..=255) for the plane palette tail.
    ramp: [[u8; 3]; 32],
    id: MenuId,
    slots: Vec<Vec<u8>>,
    count: usize,
    sel: i8,
    /// Difficulty 0..2 (EXW 0046cbf8).
    difficulty: u8,
    /// Multiplayer player count 2..=12 (EXW 0046cbe0).
    players: u8,
    /// Player name, <= 8 bytes (EXW 004e444c).
    name: Vec<u8>,
    phase: MenuPhase,
    /// Whether the host has stood the menu on Title (a menu staged
    /// on another scene stays inert - the Staged semantics of the
    /// other flows - and is dropped only after it was live once).
    entered: bool,
    /// Integrated absolute cursor (EXW g_cursor_x/y @004eddc4/c8;
    /// InputFrame carries deltas, the menu owns the position).
    /// Boots at the GameInit center (320,240) and clamps into the
    /// original [9,631]x[9,463] box every tick [D160/RE-EXD-MAP §5h
    /// — the P2e package].
    cursor: (i32, i32),
    /// Idle counter (EXW 0046cbec).
    idle: u32,
    /// SFX debounce ticks left.
    debounce: u8,
    /// Last tick's button mask (click = press EDGE, D42.2).
    prev_buttons: u8,
    /// Per-tick counter (the g_frame_count blink analog).
    blink: u64,
    /// The 640x480 owned draw plane (black + text strip, D42.4).
    plane: Vec<u8>,
    /// Plane needs a redraw (build / hover change / blink).
    dirty: bool,
    /// Score seed of the last Start action (introspection for the
    /// P2d sim-tail wiring).
    start_score_seen: Option<i32>,
}

impl TitleMenu {
    /// Build the staged menu from the corpus bytes the caller
    /// fetched (LANGUAGE.* / FULLFONT.BIN / FULLPAL.PAL). The table
    /// must cover index 94 (the deepest string any menu reads); the
    /// bank must decode at both glyph bases.
    pub fn new(language: &[u8], font_bin: &[u8], fullpal: &[u8]) -> Result<TitleMenu, GameError> {
        let items = bedlam_assets::language::parse_menu_items(language)?;
        if items.len() < 95 {
            return Err(GameError::BadMenuAsset {
                what: "language table",
                reason: "fewer than 95 [MENU_ITEMS] entries",
            });
        }
        let green = crate::font::LoadingFont::from_bank_at(font_bin, crate::font::GLYPH_BASE)?;
        let blue = crate::font::LoadingFont::from_bank_at(font_bin, 0)?;
        let ramp = bedlam_assets::pal::parse_font_ramp(fullpal)?;
        let mut menu = TitleMenu {
            items,
            green,
            blue,
            ramp,
            id: MenuId::Main,
            slots: vec![Vec::new(); SLOTS],
            count: 0,
            sel: -1,
            difficulty: 0,
            players: 2,
            name: Vec::new(),
            phase: MenuPhase::Interactive,
            entered: false,
            cursor: (CURSOR_BOOT_X, CURSOR_BOOT_Y),
            idle: 0,
            debounce: 0,
            prev_buttons: 0,
            blink: 0,
            plane: vec![0u8; 640 * 480],
            dirty: true,
            start_score_seen: None,
        };
        menu.build(MenuId::Main);
        Ok(menu)
    }

    /// FUN_00445b5c: (re)build a menu by id from the current state
    /// (difficulty / players / name feed their rows). Resets the
    /// selection to -1 (the EXW builder leaves sel untouched, but
    /// every call site re-reads it as -1 on the rebuilt strip).
    fn build(&mut self, id: MenuId) {
        let item = |k: usize| -> Vec<u8> { self.items.get(k).cloned().unwrap_or_default() };
        let slots: Vec<Vec<u8>> = match id {
            MenuId::Main => vec![
                item(3),                        // New Single Player Game
                item(30),                       // Start Saved Game
                item(self.difficulty as usize), // Difficulty: SIMPLE/STANDARD/BEDLAM !!!
                self.name_row(),
                item(5),  // View Hall of Fame
                item(68), // Credits
                item(94), // Quit to Windows
            ],
            MenuId::Multi => vec![
                item(14),                                 // Start Cooperative Game
                item(15),                                 // Start Head2Head Game
                item(17 + usize::from(self.players) - 2), // Number of Players: N
                item(16),                                 // Main Menu
            ],
            MenuId::Load => vec![
                EMPTY_SLOT.to_vec(),
                EMPTY_SLOT.to_vec(),
                EMPTY_SLOT.to_vec(),
                EMPTY_SLOT.to_vec(),
                EMPTY_SLOT.to_vec(),
                item(32), // Cancel
            ],
            MenuId::QuitConfirm => vec![
                item(94), // Quit to Windows
                item(16), // Main Menu
            ],
        };
        self.count = slots.len();
        self.slots = slots;
        self.slots.resize(SLOTS, Vec::new());
        self.id = id;
        self.sel = -1;
        self.dirty = true;
    }

    /// The menu-1 name row: "Name:"(31) + " "(0x459779) + name.
    fn name_row(&self) -> Vec<u8> {
        let mut v = self.items.get(31).cloned().unwrap_or_default();
        v.extend_from_slice(NAME_SEP);
        v.extend_from_slice(&self.name);
        v
    }

    /// Current menu id.
    pub fn id(&self) -> MenuId {
        self.id
    }

    /// Item count of the built menu (EXW hiword 004eabd2).
    pub fn count(&self) -> usize {
        self.count
    }

    /// The built slot strings (index < count).
    pub fn slots(&self) -> &[Vec<u8>] {
        &self.slots
    }

    /// Hovered item index, -1 outside the strip.
    pub fn sel(&self) -> i8 {
        self.sel
    }

    /// Difficulty 0..2.
    pub fn difficulty(&self) -> u8 {
        self.difficulty
    }

    /// Multiplayer player count 2..=12.
    pub fn players(&self) -> u8 {
        self.players
    }

    /// The player name bytes.
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Interaction phase.
    pub fn phase(&self) -> MenuPhase {
        self.phase
    }

    /// Whether the host has stood this menu on Title (the host-side
    /// lifecycle latch: staged-inert until then, dropped after).
    pub(crate) fn entered(&self) -> bool {
        self.entered
    }

    /// Host lifecycle latch: mark the menu live (called by the host
    /// sync when the scene stands on Title).
    pub(crate) fn mark_entered(&mut self) {
        self.entered = true;
    }

    /// Idle counter value (0..=0x300).
    pub fn idle(&self) -> u32 {
        self.idle
    }

    /// Integrated cursor position.
    pub fn cursor(&self) -> (i32, i32) {
        self.cursor
    }

    /// Score seed of the last Start action, if any (D42.8).
    pub fn start_score_seen(&self) -> Option<i32> {
        self.start_score_seen
    }

    /// One executed sim tick. `movies_playing` = a Title-scene movie
    /// is currently playing (the first pass or the attract replay):
    /// the menu is inert except for the attract skip (D42.3).
    pub fn tick(&mut self, input: &InputFrame, movies_playing: bool) -> MenuTick {
        // Integrate the pointer (the EXW ISR's absolute cursor),
        // pinned to the twin-verified model [D160/RE-EXD-MAP §5h, the
        // P2e package]: clamp into [9,631]x[9,463] on every integrate
        // (EXW ScrollUpdate 0x425b2e..0x425b84; EXD poll
        // 0x12615..0x12659, mickey integrate-then-clamp — the 9 = the
        // 24x24 cursor-sprite hotspot offset), boot at the GameInit
        // center (320,240). The constants are the bedlam-core
        // frame ones (the classic-input adapter's own box, D160).
        self.cursor.0 =
            (self.cursor.0 + i32::from(input.mouse_dx)).clamp(CURSOR_MIN_X, CURSOR_MAX_X);
        self.cursor.1 =
            (self.cursor.1 + i32::from(input.mouse_dy)).clamp(CURSOR_MIN_Y, CURSOR_MAX_Y);
        self.blink = self.blink.wrapping_add(1);
        if self.debounce > 0 {
            self.debounce -= 1;
        }
        let clicked = self.prev_buttons == 0 && input.mouse_buttons & 0x03 != 0;
        let keyed = input.buttons != 0;
        self.prev_buttons = input.mouse_buttons & 0x03;

        if movies_playing {
            if self.phase == MenuPhase::Attract && (clicked || keyed) {
                // The skip gate: any click/key aborts the replay.
                self.phase = MenuPhase::Interactive;
                self.idle = 0;
                self.dirty = true;
                return MenuTick {
                    action: Some(MenuAction::SkipAttract),
                    ..MenuTick::default()
                };
            }
            return MenuTick::default();
        }
        if self.phase == MenuPhase::Attract {
            // The replay ran to its end on its own: back to the menu
            // (EXW: SetPaletteIndex + redraw after FUN_004459f7).
            self.phase = MenuPhase::Interactive;
            self.idle = 0;
            self.dirty = true;
            return MenuTick::default();
        }

        if self.phase == MenuPhase::NameEntry {
            // The name sub-loop: no hover, no idle; a click exits
            // [D42.6 deviation: EXW exits on ENTER, keystore 0x1c -
            // the input mask has no text/ENTER path yet].
            if clicked {
                if self.name.is_empty() {
                    self.name = DEFAULT_NAME.to_vec();
                }
                self.phase = MenuPhase::Interactive;
                self.build(MenuId::Main);
            }
            return MenuTick::default();
        }

        // Interactive: hover, click dispatch, idle.
        let new_sel = hit(self.count, self.cursor.0, self.cursor.1);
        let hover_changed = new_sel != self.sel;
        if hover_changed {
            self.sel = new_sel;
            self.dirty = true;
            self.idle = 0; // EXW 0043a8b0
        }
        let mut tick = MenuTick::default();
        if hover_changed && self.debounce == 0 {
            tick.hover_sfx = true;
            self.debounce = SFX_DEBOUNCE;
        }
        if clicked {
            self.idle = 0;
            if self.debounce == 0 {
                tick.click_sfx = true;
                self.debounce = SFX_DEBOUNCE;
            }
            tick.action = self.dispatch(input);
        }
        if !hover_changed && !clicked {
            self.idle = self.idle.saturating_add(1);
            if self.idle >= ATTRACT_IDLE {
                self.phase = MenuPhase::Attract;
                self.idle = 0;
                tick.action = Some(MenuAction::Attract);
            }
        }
        tick
    }

    /// Host callback for an attract that could not start (no staged
    /// Title movie, D42.3): back to Interactive, counter restarted.
    /// Without this the replay-end transition (movies_playing false
    /// while in Attract) would read as a finished replay.
    pub(crate) fn cancel_attract(&mut self) {
        if self.phase == MenuPhase::Attract {
            self.phase = MenuPhase::Interactive;
            self.idle = 0;
            self.dirty = true;
        }
    }

    /// The click dispatch (EXW sub-switch per menu id; tables
    /// @0x43a5b8 / @0x43a5d8 / @0x43a5e8). Stubs per D42.5: coop /
    /// head2head / HOF / credits / empty save slots are inert.
    fn dispatch(&mut self, input: &InputFrame) -> Option<MenuAction> {
        let sel = self.sel;
        match (self.id, sel) {
            (MenuId::Main, -1) => {
                // Click outside the strip: the multiplayer menu with
                // players reset [verified 0x43aad5].
                self.players = 2;
                self.build(MenuId::Multi);
                None
            }
            (MenuId::Main, 0) => {
                let score = start_score(self.difficulty);
                self.start_score_seen = Some(score);
                Some(MenuAction::Start { score })
            }
            (MenuId::Main, 1) => {
                self.build(MenuId::Load);
                None
            }
            (MenuId::Main, 2) => {
                // Difficulty cycle (d+1) mod 3, rebuild [verified
                // 0x43ab7e].
                self.difficulty = (self.difficulty + 1) % 3;
                self.build(MenuId::Main);
                None
            }
            (MenuId::Main, 3) => {
                self.phase = MenuPhase::NameEntry;
                self.dirty = true;
                None
            }
            (MenuId::Main, 4) | (MenuId::Main, 5) | (MenuId::Main, 6) => {
                // HOF / credits / quit-confirm entry: only the
                // quit-confirm builds a menu here (D42.5 stubs the
                // other two).
                if sel == 6 {
                    self.build(MenuId::QuitConfirm);
                }
                None
            }
            (MenuId::Multi, 0) | (MenuId::Multi, 1) => None, // coop/h2h stubs
            (MenuId::Multi, 2) => {
                // Player count by button bit [verified 0x43c128]:
                // bit0 (left) ++ wrapping 13 -> 2, bit1 (right) --
                // wrapping below 2 -> 12.
                if input.mouse_buttons & 1 != 0 {
                    self.players = if self.players >= 12 {
                        2
                    } else {
                        self.players + 1
                    };
                } else if input.mouse_buttons & 2 != 0 {
                    self.players = if self.players <= 2 {
                        12
                    } else {
                        self.players - 1
                    };
                }
                self.build(MenuId::Multi);
                None
            }
            (MenuId::Multi, 3) => {
                self.build(MenuId::Main);
                None
            }
            (MenuId::Load, 5) => {
                self.build(MenuId::Main);
                None
            }
            (MenuId::Load, 0..=4) => None, // EMPTY slot: inert
            (MenuId::QuitConfirm, 0) => Some(MenuAction::Quit),
            (MenuId::QuitConfirm, 1) => {
                self.build(MenuId::Main);
                None
            }
            // sel beyond the built count (a stale hover index) or a
            // click outside the strip on a non-main menu: inert.
            (_, _) => None,
        }
    }

    /// Type one character into the name (explicit host API for the
    /// shell's text path, D42.6; the hashed InputFrame carries no
    /// characters). Printable ASCII only, capacity 8 [verified:
    /// len < 8 -> append at 004e444c]. Returns whether it landed.
    pub fn type_char(&mut self, c: u8) -> bool {
        if self.phase != MenuPhase::NameEntry || !(0x20..=0x7e).contains(&c) {
            return false;
        }
        if self.name.len() >= NAME_MAX {
            return false;
        }
        self.name.push(c);
        self.build(MenuId::Main);
        self.phase = MenuPhase::NameEntry;
        true
    }

    /// Backspace in the name entry [verified: scan 0xe/0xd3
    /// shortens]. Returns whether a char was removed.
    pub fn backspace(&mut self) -> bool {
        if self.phase != MenuPhase::NameEntry || self.name.is_empty() {
            return false;
        }
        self.name.pop();
        self.build(MenuId::Main);
        self.phase = MenuPhase::NameEntry;
        true
    }

    /// The menu plane: the owned 640x480 raster under the host
    /// palette with the staged FULLPAL ramp folded into DAC
    /// 224..=255 (the draw-cycle ramp commit analog). Redraws lazily
    /// when dirty (every tick in name entry - the blink).
    pub(crate) fn plane(&mut self, host_palette: &[Vga6; 256]) -> Option<Plane<'_>> {
        if self.phase == MenuPhase::NameEntry || self.dirty {
            self.redraw();
        }
        let mut palette = *host_palette;
        palette[224..].copy_from_slice(&self.ramp);
        Some(Plane {
            w: 640,
            h: 480,
            pixels: &self.plane,
            palette,
        })
    }

    /// FUN_0044653a + the name-entry cursor: black canvas, item i at
    /// row_base + i*0x18 through the green (sel) or blue set, the
    /// blink cursor on the item-3 row while (blink & 0xc) != 0.
    fn redraw(&mut self) {
        self.plane.fill(0);
        let row_base = STRIP_Y_MAX - self.count as i32 * ROW_H;
        for i in 0..self.count {
            let font = if i as i8 == self.sel {
                &self.green
            } else {
                &self.blue
            };
            let row = row_base + i as i32 * ROW_H;
            font.draw(&mut self.plane, 640, &self.slots[i], row);
        }
        if self.phase == MenuPhase::NameEntry && (self.blink & BLINK_MASK) != 0 {
            let name_item = self.items.get(31).cloned().unwrap_or_default();
            let w = self.green.measure(&name_item)
                + self.green.measure(NAME_SEP)
                + self.green.measure(&self.name);
            let x = CURSOR_X_BASE + w / 2;
            let row = STRIP_Y_MAX - (self.count as i32 - 3) * ROW_H;
            self.green
                .draw_at(&mut self.plane, 640, &[CURSOR_CHAR], x, row);
        }
        self.dirty = false;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A LANGUAGE-shaped file placing the menu strings at the EXW
    /// indices. Strings use only chars the synth font bank draws
    /// (bang E I O U e i o u dash space - see font::synth).
    pub(crate) fn synth_language() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"[OTHER]\r\n[\r\nx\r\n]\r\n\r\n");
        v.extend_from_slice(b"[MENU_ITEMS]\r\n\r\n[\r\n");
        let mut emitted = 0usize;
        let mut put = |v: &mut Vec<u8>, want: usize, text: &[u8]| {
            while emitted < want {
                v.extend_from_slice(b"filler line\r\n");
                emitted += 1;
            }
            v.extend_from_slice(text);
            v.extend_from_slice(b"\r\n");
            emitted += 1;
        };
        put(&mut v, 0, b"EioU!"); // difficulty 0 (SIMPLE analog)
        put(&mut v, 1, b"EioU-"); // difficulty 1
        put(&mut v, 2, b"EioU!!"); // difficulty 2
        put(&mut v, 3, b"eI Ou!"); // New Single Player Game
        put(&mut v, 5, b"Ou Ee"); // View Hall of Fame
        put(&mut v, 14, b"Io Uu"); // coop
        put(&mut v, 15, b"Ee Ii"); // h2h
        put(&mut v, 16, b"uUeE"); // Main Menu
        put(&mut v, 17, b"Oo 2"); // players 2
        put(&mut v, 18, b"Oo 3"); // players 3
        put(&mut v, 27, b"Oo 12"); // players 12
        put(&mut v, 30, b"Ii Oo"); // Start Saved Game
        put(&mut v, 31, b"Ee:"); // Name:
        put(&mut v, 32, b"Uu"); // Cancel
        put(&mut v, 68, b"ee II"); // Credits
        put(&mut v, 94, b"uu EE"); // Quit to Windows
        v.extend_from_slice(b"]\r\n");
        v
    }

    fn menu() -> TitleMenu {
        TitleMenu::new(
            &synth_language(),
            &crate::font::synth::font_bin(),
            &crate::font::synth::fullpal_bin(),
        )
        .unwrap()
    }

    /// Stage the synth menu on a host (shared with the host tests):
    /// synth language + dual-base font + ramp + a short SFX wave.
    pub(crate) fn stage_synth_menu(host: &mut crate::host::GameHost) {
        host.load_title_menu(
            &synth_language(),
            &crate::font::synth::font_bin(),
            &crate::font::synth::fullpal_bin(),
            &[128u8, 200, 128, 90],
            &[128u8, 220, 128, 90],
        )
        .unwrap();
    }

    fn frame(dx: i32, dy: i32, buttons: u8) -> InputFrame {
        InputFrame {
            mouse_dx: dx as i16,
            mouse_dy: dy as i16,
            mouse_buttons: buttons,
            ..InputFrame::default()
        }
    }

    /// Move the cursor onto the strip center of item `i` in one
    /// tick (the exact delta), returning that tick's report.
    fn hover(menu: &mut TitleMenu, i: i8) -> MenuTick {
        let top = STRIP_Y_MAX - menu.count() as i32 * ROW_H;
        let y = top + i as i32 * ROW_H + ROW_H / 2;
        let x = (STRIP_X_MIN + STRIP_X_MAX) / 2;
        let t = menu.tick(&frame(x - menu.cursor().0, y - menu.cursor().1, 0), false);
        assert_eq!(menu.sel(), i, "hover helper landed on item {i}");
        t
    }

    /// Click (press edge) at the current cursor.
    fn click(menu: &mut TitleMenu, buttons: u8) -> MenuTick {
        let t = menu.tick(&frame(0, 0, buttons), false);
        menu.tick(&frame(0, 0, 0), false);
        t
    }

    #[test]
    fn hit_pins_the_strip_geometry() {
        // count 7: top = 0x1d6 - 7*0x18 = 302.
        assert_eq!(hit(7, 0xdc, 310), -1, "x == 0xdc excluded");
        assert_eq!(hit(7, 0xdb, 310), -1);
        assert_eq!(hit(7, 0xdd, 310), 0, "x == 0xdc+1 inside");
        assert_eq!(hit(7, 0x1a3, 310), 0, "x == 0x1a4-1 inside");
        assert_eq!(hit(7, 0x1a4, 310), -1, "x == 0x1a4 excluded");
        assert_eq!(hit(7, 320, 301), -1, "y == top excluded");
        assert_eq!(hit(7, 320, 302), -1, "y == top excluded (strict >)");
        assert_eq!(hit(7, 320, 303), 0, "first row starts at top+1");
        assert_eq!(hit(7, 320, 326), 1, "row boundary (302+24)/24 = 1");
        assert_eq!(hit(7, 320, 0x1d5), 6, "last row below the bound");
        assert_eq!(hit(7, 320, 0x1d6), -1, "y == 0x1d6 excluded");
        // count 2 (quit confirm): top = 470 - 48 = 422 (strictly
        // below top hits; 422 itself is the excluded boundary).
        assert_eq!(hit(2, 320, 423), 0);
        assert_eq!(hit(2, 320, 446), 1);
        assert_eq!(hit(2, 320, 421), -1);
        assert_eq!(hit(2, 320, 422), -1);
    }

    #[test]
    fn menu_one_builds_the_exw_table() {
        let m = menu();
        assert_eq!(m.id(), MenuId::Main);
        assert_eq!(m.count(), 7);
        let slots = m.slots();
        assert_eq!(slots[0], b"eI Ou!".as_slice(), "MENU_ITEMS 3");
        assert_eq!(slots[1], b"Ii Oo".as_slice(), "MENU_ITEMS 30");
        assert_eq!(slots[2], b"EioU!".as_slice(), "difficulty idx 0");
        assert_eq!(slots[3], b"Ee: ".as_slice(), "Name: + sep + empty");
        assert_eq!(slots[4], b"Ou Ee".as_slice(), "MENU_ITEMS 5");
        assert_eq!(slots[5], b"ee II".as_slice(), "MENU_ITEMS 68");
        assert_eq!(slots[6], b"uu EE".as_slice(), "MENU_ITEMS 94");
    }

    #[test]
    fn difficulty_cycles_and_rebuilds_the_row() {
        let mut m = menu();
        assert_eq!(m.difficulty(), 0);
        hover(&mut m, 2);
        click(&mut m, 1);
        assert_eq!(m.difficulty(), 1);
        assert_eq!(m.slots()[2], b"EioU-".as_slice());
        click(&mut m, 1);
        assert_eq!(m.difficulty(), 2);
        assert_eq!(m.slots()[2], b"EioU!!".as_slice());
        click(&mut m, 1);
        assert_eq!(m.difficulty(), 0, "(d+1) mod 3");
        assert_eq!(m.slots()[2], b"EioU!".as_slice());
    }

    #[test]
    fn start_hands_off_with_the_score_seed() {
        let mut m = menu();
        hover(&mut m, 0);
        let t = click(&mut m, 1);
        assert_eq!(t.action, Some(MenuAction::Start { score: 4000 }));
        assert_eq!(m.start_score_seen(), Some(4000));
        // Difficulty bumps the seed: 4000 - 500*d.
        hover(&mut m, 2);
        click(&mut m, 1);
        hover(&mut m, 2);
        click(&mut m, 1);
        hover(&mut m, 0);
        let t = click(&mut m, 1);
        assert_eq!(t.action, Some(MenuAction::Start { score: 4000 - 1000 }));
    }

    #[test]
    fn quit_flows_through_the_confirm_menu() {
        let mut m = menu();
        hover(&mut m, 6);
        let t = click(&mut m, 1);
        assert_eq!(t.action, None, "building the confirm menu is not quit");
        assert_eq!(m.id(), MenuId::QuitConfirm);
        assert_eq!(m.count(), 2);
        assert_eq!(m.slots()[0], b"uu EE".as_slice());
        assert_eq!(m.slots()[1], b"uUeE".as_slice());
        hover(&mut m, 0);
        let t = click(&mut m, 1);
        assert_eq!(t.action, Some(MenuAction::Quit));
        // Main Menu backs out instead.
        let mut m = menu();
        hover(&mut m, 6);
        click(&mut m, 1);
        hover(&mut m, 1);
        let t = click(&mut m, 1);
        assert_eq!(t.action, None);
        assert_eq!(m.id(), MenuId::Main);
    }

    #[test]
    fn saved_game_menu_is_empty_slots_plus_cancel() {
        let mut m = menu();
        hover(&mut m, 1);
        click(&mut m, 1);
        assert_eq!(m.id(), MenuId::Load);
        assert_eq!(m.count(), 6);
        for slot in &m.slots()[0..5] {
            assert_eq!(slot, EMPTY_SLOT);
        }
        assert_eq!(m.slots()[5], b"Uu".as_slice(), "Cancel");
        // Slot clicks are inert (no save corpus); Cancel returns.
        hover(&mut m, 0);
        let t = click(&mut m, 1);
        assert_eq!(t.action, None);
        assert_eq!(m.id(), MenuId::Load);
        hover(&mut m, 5);
        click(&mut m, 1);
        assert_eq!(m.id(), MenuId::Main);
    }

    #[test]
    fn click_outside_the_strip_opens_multiplayer() {
        let mut m = menu();
        // Cursor far left of the strip: hover stays -1.
        for _ in 0..480 {
            m.tick(&frame(10 - m.cursor().0, 10 - m.cursor().1, 0), false);
        }
        assert_eq!(m.sel(), -1);
        let t = click(&mut m, 1);
        assert_eq!(t.action, None);
        assert_eq!(m.id(), MenuId::Multi);
        assert_eq!(m.count(), 4);
        assert_eq!(m.players(), 2, "players reset to 2");
        assert_eq!(m.slots()[2], b"Oo 2".as_slice());
    }

    #[test]
    fn players_item_cycles_by_button_bit() {
        let mut m = menu();
        for _ in 0..480 {
            m.tick(&frame(10 - m.cursor().0, 10 - m.cursor().1, 0), false);
        }
        click(&mut m, 1); // -> Multi
        hover(&mut m, 2);
        click(&mut m, 1); // left button: ++
        assert_eq!(m.players(), 3);
        assert_eq!(m.slots()[2], b"Oo 3".as_slice());
        for _ in 0..9 {
            click(&mut m, 1);
        }
        assert_eq!(m.players(), 12, "climbs to 12");
        click(&mut m, 1);
        assert_eq!(m.players(), 2, "wraps at 13 -> 2");
        click(&mut m, 2); // right button: --
        assert_eq!(m.players(), 12, "wraps below 2 -> 12");
        // Main Menu returns.
        hover(&mut m, 3);
        click(&mut m, 1);
        assert_eq!(m.id(), MenuId::Main);
    }

    #[test]
    fn name_entry_types_backspaces_and_defaults_to_god() {
        let mut m = menu();
        hover(&mut m, 3);
        click(&mut m, 1);
        assert_eq!(m.phase(), MenuPhase::NameEntry);
        // Typing is the explicit API (D42.6).
        assert!(m.type_char(b'E'));
        assert!(m.type_char(b'U'));
        assert_eq!(m.name(), b"EU".as_slice());
        assert_eq!(m.slots()[3], b"Ee: EU".as_slice(), "row rebuilt per key");
        assert!(m.backspace());
        assert_eq!(m.name(), b"E".as_slice());
        for _ in 0..9 {
            m.type_char(b'I');
        }
        assert_eq!(m.name().len(), NAME_MAX, "capacity 8");
        // Click exits; non-empty name kept.
        click(&mut m, 1);
        assert_eq!(m.phase(), MenuPhase::Interactive);
        assert_eq!(m.name(), b"EIIIIIII".as_slice());
        assert_eq!(m.slots()[3], b"Ee: EIIIIIII".as_slice());
        // Empty name defaults to GOD on exit [inferred, 0x459078].
        hover(&mut m, 3);
        click(&mut m, 1);
        assert!(m.backspace(), "clear the name");
        for _ in 0..7 {
            m.backspace();
        }
        assert!(m.name().is_empty());
        click(&mut m, 1);
        assert_eq!(m.name(), DEFAULT_NAME);
        // Typing outside name entry is rejected.
        assert!(!m.type_char(b'x'));
    }

    #[test]
    fn hover_sfx_fires_on_change_with_debounce() {
        let mut m = menu();
        let mut any = false;
        let top = STRIP_Y_MAX - m.count() as i32 * ROW_H;
        let y = top + ROW_H / 2;
        let x = (STRIP_X_MIN + STRIP_X_MAX) / 2;
        for _ in 0..480 {
            let t = m.tick(&frame(x - m.cursor().0, y - m.cursor().1, 0), false);
            any |= t.hover_sfx;
        }
        assert!(any, "the entering hover change plays MENU1");
        assert_eq!(m.sel(), 0);
        // A same-row re-hover within the debounce window is silent.
        let t = m.tick(&frame(0, 0, 0), false);
        assert!(!t.hover_sfx, "no change: no sfx");
        // A change after the debounce window plays again.
        for _ in 0..5 {
            m.tick(&frame(0, 0, 0), false);
        }
        let y2 = top + ROW_H + ROW_H / 2;
        let t = m.tick(&frame(0, y2 - m.cursor().1, 0), false);
        assert!(t.hover_sfx, "fresh change after the window: MENU1");
    }

    #[test]
    fn click_sfx_fires_on_press_edge_once() {
        let mut m = menu();
        hover(&mut m, 0);
        for _ in 0..5 {
            m.tick(&frame(0, 0, 0), false); // debounce window drains
        }
        let t = m.tick(&frame(0, 0, 1), false);
        assert!(t.click_sfx);
        let t = m.tick(&frame(0, 0, 1), false);
        assert!(!t.click_sfx, "held: no re-fire (edge model, D42.2)");
        let t = m.tick(&frame(0, 0, 0), false);
        assert!(!t.click_sfx);
        // Re-press inside the 4-tick debounce window: silent.
        let t = m.tick(&frame(0, 0, 1), false);
        assert!(!t.click_sfx, "debounced");
        // After the window: the next press plays again.
        for _ in 0..4 {
            m.tick(&frame(0, 0, 1), false); // held: no edges
        }
        m.tick(&frame(0, 0, 0), false);
        let t = m.tick(&frame(0, 0, 1), false);
        assert!(t.click_sfx, "fresh press after the window: MENU2");
    }

    #[test]
    fn attract_fires_at_threshold_and_skips() {
        let mut m = menu();
        let mut fired = 0;
        for _ in 0..ATTRACT_IDLE {
            let t = m.tick(&InputFrame::default(), false);
            if t.action == Some(MenuAction::Attract) {
                fired += 1;
            }
        }
        assert_eq!(fired, 1, "fires exactly at 0x300");
        assert_eq!(m.phase(), MenuPhase::Attract);
        // During the replay (movies_playing): inert except the skip.
        let t = m.tick(&InputFrame::default(), true);
        assert_eq!(t, MenuTick::default());
        let t = m.tick(&frame(0, 0, 1), true);
        assert_eq!(t.action, Some(MenuAction::SkipAttract));
        assert_eq!(m.phase(), MenuPhase::Interactive);
        // A replay that ends on its own returns quietly (the host
        // reports movies_playing false again).
        for _ in 0..ATTRACT_IDLE {
            let t = m.tick(&InputFrame::default(), false);
            assert_ne!(t.action, Some(MenuAction::SkipAttract));
            if t.action == Some(MenuAction::Attract) {
                fired += 1;
            }
        }
        assert!(fired >= 2, "idle fires again after the replay");
        // The standalone menu (no host movie) returns from Attract
        // on the next no-movie tick - the replay-end transition the
        // host cancel and a finished replay share.
        // Walk the full replay lifecycle once: fire, play, end.
        let mut guard = 0;
        loop {
            let t = m.tick(&InputFrame::default(), false);
            if t.action == Some(MenuAction::Attract) {
                break;
            }
            guard += 1;
            assert!(guard < ATTRACT_IDLE * 2, "never re-fired");
        }
        assert_eq!(m.phase(), MenuPhase::Attract);
        let t = m.tick(&InputFrame::default(), true);
        assert_eq!(t, MenuTick::default(), "playing: inert");
        let t = m.tick(&InputFrame::default(), false);
        assert_eq!(t, MenuTick::default(), "replay end: no action, just redraw");
        assert_eq!(m.phase(), MenuPhase::Interactive);
        // Without a movie the host cancels (cancel_attract); the
        // model-side fallback is the same replay-end transition.
        let mut m = menu();
        for _ in 0..ATTRACT_IDLE {
            m.tick(&InputFrame::default(), false);
        }
        assert_eq!(m.phase(), MenuPhase::Attract);
        m.cancel_attract();
        assert_eq!(m.phase(), MenuPhase::Interactive);
        assert_eq!(m.idle(), 0);
        // Idle resets on hover.
        let mut m = menu();
        for _ in 0..100 {
            m.tick(&InputFrame::default(), false);
        }
        assert_eq!(m.idle(), 100);
        hover(&mut m, 4);
        assert_eq!(m.idle(), 0, "hover resets the counter");
        // Movies playing (the first pass): no idle counting.
        let mut m = menu();
        for _ in 0..ATTRACT_IDLE * 2 {
            m.tick(&InputFrame::default(), true);
        }
        assert_eq!(m.idle(), 0);
        assert_eq!(m.phase(), MenuPhase::Interactive);
    }

    #[test]
    fn cursor_integrates_deltas_and_clamps() {
        let mut m = menu();
        // Boot at the GameInit center [D160/RE-EXD-MAP §5h].
        assert_eq!(m.cursor(), (320, 240));
        m.tick(&frame(700, 0, 0), false);
        assert_eq!(m.cursor().0, 631);
        m.tick(&frame(0, 1000, 0), false);
        assert_eq!(m.cursor().1, 463);
        m.tick(&frame(-2000, -2000, 0), false);
        assert_eq!(m.cursor(), (9, 9));
    }

    #[test]
    fn plane_draws_bottom_anchored_rows_with_both_sets() {
        let mut m = menu();
        let pal = [[0u8, 0, 0]; 256];
        let plane = m.plane(&pal).unwrap();
        assert_eq!(plane.w, 640);
        assert_eq!(plane.h, 480);
        let px = plane.pixels;
        // Row base 302 (count 7); every item row 302..470 holds
        // glyph pixels (bang/EIOU chars from the synth font draw),
        // the rows above the strip stay black.
        let row_has = |r: usize| px[r * 640..(r + 1) * 640].iter().any(|&v| v != 0);
        for i in 0..7 {
            let r = (STRIP_Y_MAX - 7 * ROW_H + i * ROW_H) as usize;
            assert!(row_has(r), "item row {i} (row {r}) draws");
        }
        assert!(!row_has(100), "above the strip stays black");
        assert!(!row_has(0), "top row black");
        // Selecting an item turns its row green (base 0x82 set,
        // synth fills 0xF1/0xF2) vs the blue set (same shapes, other
        // pixel values): the selected row's pixel SET differs.
        let unselected: std::collections::HashSet<u8> = px[302 * 640..306 * 640]
            .iter()
            .copied()
            .filter(|&v| v != 0)
            .collect();
        let mut m2 = menu();
        hover(&mut m2, 0);
        let plane2 = m2.plane(&pal).unwrap();
        let selected: std::collections::HashSet<u8> = plane2.pixels[302 * 640..306 * 640]
            .iter()
            .copied()
            .filter(|&v| v != 0)
            .collect();
        assert!(!unselected.is_empty() && !selected.is_empty());
        assert_ne!(unselected, selected, "green vs blue ramp sets");
        // The palette tail carries the staged FULLPAL ramp.
        assert_eq!(plane.palette[224..], m2.plane(&pal).unwrap().palette[224..]);
        let ramp_entries: Vec<[u8; 3]> = plane.palette[224..].to_vec();
        assert!(
            ramp_entries.iter().any(|c| c != &[0, 0, 0]),
            "ramp folded in"
        );
    }

    #[test]
    fn short_language_and_bad_bank_reject() {
        let err = TitleMenu::new(
            b"[MENU_ITEMS]\r\n[\r\none\r\n]\r\n",
            &crate::font::synth::font_bin(),
            &crate::font::synth::fullpal_bin(),
        );
        assert!(matches!(
            err,
            Err(GameError::BadMenuAsset {
                what: "language table",
                ..
            })
        ));
        let err = TitleMenu::new(
            &synth_language(),
            b"garbage",
            &crate::font::synth::fullpal_bin(),
        );
        assert!(err.is_err());
    }
}
