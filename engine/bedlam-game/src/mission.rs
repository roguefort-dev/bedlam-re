//! MissionScene — the Mission-scene composition of the two
//! corpus-verified halves (DESIGN-GAME sec 11, added 2026-08-21):
//! bedlam-core `MissionSim` (the P2d/P4 sim slice) + bedlam-render
//! `MissionView` (the isometric viewport + robot entity overlay).
//!
//! NO decoding lives here — every behavior is anchored to an
//! already-RE-pinned EXW fact:
//! - staging: load_mission@0041dc5a + load_markers@0040cca0
//!   [RE-EXW-SIM sec 7c] — file bytes in, terrain + angle table +
//!   spawned robots out, `MissionShell` RNG reseed 0x1E240 [sec 1];
//! - per-frame: the MissionShell loop order — input (mouse_l_click)
//!   BEFORE the six unit-manager phases [sec 1], so the click seam
//!   runs before `advance_frame`;
//! - left viewport input creates movement commands (0x40b835),
//!   using the separate flag-1 command consumer; viewport clicks are
//!   x < 0x1E0, x >= 0x1E0 runs the sidebar producer [sec 6c —
//!   select strips + order rows + the redraw countdown];
//! - present: the viewport pass order enqueue -> terrain -> window
//!   [RE-EXW-MISSIONVIEW secs 5d/7].
//!
//! [design] tags below are reimplementation choices documented in
//! DESIGN-GAME sec 11, not RE claims.

use bedlam_core::frame::{
    CURSOR_BOOT_X, CURSOR_BOOT_Y, CURSOR_MAX_X, CURSOR_MAX_Y, CURSOR_MIN_X, CURSOR_MIN_Y,
};
use bedlam_core::hash::StateHash;
use bedlam_core::input::InputFrame;
use bedlam_core::mission::{
    AngleTable, DamageOutcome, MissionSim, PickupOutcome, Terrain, STATE_MOVING,
};
use bedlam_core::rng::Pcg32;
use bedlam_core::weapon::CommandRecord;
use bedlam_render::map_overlay::{MapOverlay, OverlayRobot};
use bedlam_render::mission_view::{
    present_window, DebrisSpriteView, DrawParams, EffectRowView, MissionView, RobotView,
    VIEW_BUF_LEN,
};
use bedlam_render::ui_bank::{draw_glyph, draw_sprite, sprite_geometry};
use bedlam_render::Vga6;

use crate::loading::Plane;
use crate::GameError;

/// Sidebar robot-select strip x-ranges `[lo, hi]` per squad slot
/// (inclusive; slot 2's `[0x24B,0x27B]` is the asm's [0x24A< x <0x27C]
/// encoding) [RE-EXW-SIM sec 6c.2, asm 0x40d220..0x40d3b0].
pub const SIDEBAR_SELECT_STRIPS: [(i32, i32); 3] = [(0x1E7, 0x217), (0x219, 0x249), (0x24B, 0x27B)];
/// Sidebar robot-select strip y-range, inclusive [sec 6c.2].
pub const SIDEBAR_SELECT_STRIP_Y: (i32, i32) = (5, 0x35);
/// Sidebar order-row button rect, inclusive [sec 6c.4, asm 0x40d659].
pub const SIDEBAR_ORDER_RECT: (i32, i32, i32, i32) = (0x1E9, 0x275, 0x57, 0xB8);
/// Order-row pitch/first-row y: `row = (y - 0x57) / 14`, clamped to
/// 6 (7 rows exactly covering the rect height) [sec 6c.4].
pub const SIDEBAR_ORDER_ROW: (i32, i32) = (0x57, 14);
/// Order-row sprite x positions — the row body + the count well
/// [sec 6c.8a, asm 0x4084c1/0x4084dd: FUN_00401ca2 @ (0x1EB, y) and
/// (0x25A, y)]. GENERAL.BIN geometry: body 108x11 (x 0x1EB..0x257),
/// well 27x11 (x 0x25A..0x275).
pub const SIDEBAR_ROW_SPRITE_X: (i32, i32) = (0x1EB, 0x25A);
/// First order-row body y + pitch [sec 6c.8a: y = 0x59 + 14*i].
pub const SIDEBAR_ROW_SPRITE_Y: (i32, i32) = (0x59, 14);
/// Order-row sprite ids from GENERAL.BIN [sec 6c.8a]: armed rows
/// draw 0x47 + 0x4A, unarmed rows 0x49 + 0x4C.
pub const SIDEBAR_ROW_SPRITES: [(u16, u16); 2] = [(0x47, 0x4A), (0x49, 0x4C)];
/// Select-portrait sprite ids [sec 6c.8d, FUN_004072bf]: slot k
/// draws `base_sel + k` (selected) or `base_unsel + k`, at
/// (0x1E7 + 0x32*k, 5) — 48x48 sprites filling strip y 5..0x35.
pub const SIDEBAR_PORTRAIT_IDS: (u16, u16) = (0x12, 0x15);
/// Select-portrait x base + pitch (the strip x positions) + y.
pub const SIDEBAR_PORTRAIT_XY: (i32, i32, i32) = (0x1E7, 0x32, 5);
/// The blink-cursor sprite family + position [RE-EXW-SIM 7j.6, asm
/// 0x407420..0x407989]: when the cursor selector `_DAT_004dc5d0` ∈
/// {1,2,3} the portrait pass tail draws GENERAL.BIN sprite
/// `(g_frame_count & 3) + 0x51` at (0x1F0 + 0x32*slot, 0xD) — the
/// four-frame blink over the selected slot's portrait.
pub const SIDEBAR_BLINK_SPRITE: (u16, i32, i32, i32) = (0x51, 0x1F0, 0x32, 0xD);
/// The effect-row count + geometry [RE-EXW-SIM 7j.1]: 10 rows of
/// 16 B at 0x4dc5d4 (boot memset 0xa0, MissionShell 0x447a1a).
pub const EFFECT_ROWS: usize = 10;
/// The row tick [7j.3, FUN_0042205c, MissionShell 0x448080]: z
/// rises this much per frame while at or below the cap, then the
/// row frees.
pub const EFFECT_ROW_RISE: i32 = 6;
/// The row life cap [7j.3]: `z > 0x190` frees the row.
pub const EFFECT_ROW_Z_MAX: i32 = 0x190;
/// The debris-stager z clamp [7j.5, FUN_00420608 head]: the staged
/// z is clamped into `[0x20, 0xFF]` before the record write.
pub const DEBRIS_Z_CLAMP: (i32, i32) = (0x20, 0xFF);
/// The debris record count + stride [7j.5]: 128 slots × 0x30 B at
/// 0x476fbc (cleared 0x1800 B by the reset family FUN_0041a4f8).
pub const DEBRIS_SLOTS: usize = 128;
/// The death-debris kind [7g.6/7j.5]: the five staged records the
/// SP death tail enqueues through FUN_00420608.
pub const DEBRIS_KIND_DEATH: i32 = 5;
/// The kind-5 sequence table [7j.7, DGROUP 0x454424, bytes
/// verified]: sprite ids 5..0x10 then the −1 terminator — a
/// 13-frame tumble; the tick walks it one step per frame after the
/// start delay and frees the record at the terminator.
pub const DEBRIS_KIND5_SEQ: [i16; 13] = [5, 6, 7, 8, 9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, -1];
/// The dither noise ring length [RE-EXW-SIM 7i.2]: 0x800 bytes at
/// 0x4e6ed8 (.bss — runtime state, no EXE bytes).
pub const DITHER_BANK_LEN: usize = 0x800;
/// The dither churn rate [7i.2, asm 0x448147..0x448195]: 15
/// cursor-advancing re-randoms per MissionShell frame epilogue
/// (unconditional — overlay frames churn too).
pub const DITHER_CHURN: usize = 15;
/// The blit seed formula [7i.3, FUN_0041ec59(0x7f6, 0x30)]:
/// `(rand & 0x7fff) / 15` clamped ≤ 0x7f5 (divisor 0x8000/0x7f6-1).
pub const DITHER_SEED: (u32, usize) = (15, 0x7F5);
/// The per-row reseed look-ahead [7i.1, asm 0x401b14..0x401b20]:
/// reseed when `src_off + 2*48 ≥ 0x800` (a full 48x48 blit reads
/// 2304 B > the ring, so every full blit reseeds at least once).
pub const DITHER_LOOKAHEAD: usize = 96;
/// The static byte pair [7i.2]: the bank content is strictly
/// `RandB()&3 == 0 ? 0xFF : 0x00` — 25% white noise.
pub const DITHER_WHITE: u8 = 0xFF;
/// Order-row NAME text x + COUNT text x [sec 6c.8a, asm 0x408507/
/// 0x408539], both at y = 0x5B + 14*i (the row body +2), color
/// 0x24 [asm 0x4084f8/0x40853e], drawn through SMLFONT
/// (FUN_00408913, 5x7 glyphs).
pub const SIDEBAR_ROW_TEXT: (i32, i32, i32, i32, u8) = (0x1ED, 0x25C, 0x5B, 14, 0x24);
/// The map-toggle strip rect, inclusive [sec 6c.1, asm
/// 0x40d19d..0x40d21a]: fires when the 5-frame lockout is spent
/// (`_DAT_004eb8dc == 0`; the screen-mode gate ∉ {1,7} is always
/// true inside the Mission screen).
pub const MAP_TOGGLE_RECT: (i32, i32, i32, i32) = (0x213, 0x24D, 0x1B5, 0x1CF);
/// The toggle re-fire lockout [7e.5]: the strip writes 5 into
/// `_DAT_004eb8dc`, MissionShell decrements it per frame
/// (0x44871d..0x44872a) — no other consumer.
pub const MAP_TOGGLE_LOCKOUT: i32 = 5;
/// The map button chrome [7e.5, asm 0x40724e..0x4072b2]: GENERAL.BIN
/// sprite 0x5E (map closed) at (0x213, 0x1b5) — the strip's own
/// rect — drawn every NON-overlay frame at the tail of the sidebar
/// passes. The 0x5F (open) branch is dead code in the EXW (the
/// overlay draw never returns to it) and 0x8F is the other-screen
/// look (modes 1/7), so the mission always draws 0x5E.
pub const MAP_BUTTON_SPRITE: (u16, i32, i32) = (0x5E, 0x213, 0x1B5);
/// The HP/armor bar x base + slot pitch [RE-EXW-SIM 7f.1, asm
/// 0x408103/0x408195/0x408287: slot x = 0x1E8 + 0x32*k, the bar
/// under each select portrait]. GENERAL.BIN.
pub const SIDEBAR_BAR_X: (i32, i32) = (0x1E8, 0x32);
/// The HP bar y + the armor bar y [7f.1: FUN_00401ca2 @ (slot_x,
/// 0x3C) and (slot_x, 0x49)].
pub const SIDEBAR_BAR_Y: (i32, i32) = (0x3C, 0x49);
/// The HP bar sprite mapping [7f.1]: `hp = min(hp, 5000)` (signed);
/// `hp < 1 → 0x46` else `0x46 - hp*0x2E/5000` — ids 0x18 (full)
/// .. 0x46 (empty). (empty sprite, full sprite, denominator, scale)
pub const SIDEBAR_HP_BAR: (u16, u16, i32, i32) = (0x46, 0x18, 5000, 0x2E);
/// The armor bar sprite mapping [7f.1]: gate `word == 0 → 0x8E`
/// else `armor = min(armor, 2500)`; `0x8E - armor*0x2E/2500`
/// clamped `≤ 0x8D` — ids 0x60 (full) .. 0x8E (empty; the gate
/// sprite doubles as the tiny-armor cap result).
pub const SIDEBAR_ARMOR_BAR: (u16, u16, i32, i32, u16) = (0x8E, 0x60, 2500, 0x2E, 0x8D);
/// The score-strip icon + y [7f.2, FUN_004085ce]: NUMBERS.BIN
/// sprite 0xA (the 100x11 score icon) at (0x1FE, 0x18E), then nine
/// UNSIGNED score digits at the exact x table (irregular pitch —
/// thousands groups).
pub const SCORE_STRIP_ICON: (u16, i32, i32) = (0xA, 0x1FE, 0x18E);
/// Score digit x positions, 10^8..10^0 [7f.2, asm 0x408614..0x40878a].
pub const SCORE_STRIP_XS: [i32; 9] = [
    0x202, 0x20C, 0x216, 0x222, 0x22C, 0x236, 0x242, 0x24C, 0x256,
];
/// The money icon + y [7f.2]: NUMBERS.BIN sprite 0xB (the 74x11
/// money icon) at (0x20B, 0x1A4), then six SIGNED money digits.
pub const MONEY_STRIP_ICON: (u16, i32, i32) = (0xB, 0x20B, 0x1A4);
/// Money digit x positions, 10^5..10^0 [7f.2, asm 0x4088a1..0x40890e].
pub const MONEY_STRIP_XS: [i32; 6] = [0x211, 0x21B, 0x225, 0x231, 0x23B, 0x245];
/// The score/money pickup award table [7f.6, FUN_0040eba0 case 4]:
/// `RandA()&1` picks the row (0 = score, 1 = money), `RandA()&3`
/// the amount. Canonical home: `bedlam_core::mission` (the sim's
/// case-4 draws) — re-exported here for the presentation callers.
pub use bedlam_core::mission::PICKUP_AWARDS;
/// The fresh-campaign session state [RE-EXW-SIM 7d.4 + 7f.9]: score
/// starts 0 (GameMain boot write 0x41c44e), money starts 4000
/// (GameMain campaign init 0x41c5ec — `4000 - 500*difficulty`, and
/// the difficulty-0 campaign is the modeled default).
pub const FRESH_CAMPAIGN: (i32, i32) = (0, 4000);

/// The compiled-in weapon-name switch `FUN_00420260` [verified
/// decompile + PE string bytes 0x4589DD..0x458C11, RE-EXW-SIM 7d.5]:
/// group word0 (the name index) -> the row label. Every unlisted
/// index (incl. 0, 1, 5, 0xD, 0xF, 0x17, 0x1A, 0x24, 0x29) falls to
/// "ERROR"; index 0 never reaches a draw (it is the row gate).
pub fn weapon_name(index: u16) -> &'static str {
    match index {
        2 => "NEEDLER CANNON #1",
        3 => "NEEDLER CANNON #2",
        4 => "NEEDLER CANNON #3",
        6 => "PLASMA CANNON X1",
        7 => "PLASMA CANNON X2",
        8 => "PLASMA CANNON X3",
        9 => "HADES BOMB #1",
        10 => "HADES BOMB #2",
        0xB => "HADES BOMB #3",
        0xE => "FLAME BOMB",
        0x10 => "PROXIMITY MINE X2",
        0x11 => "PROXIMITY MINE X4",
        0x12 => "PROXIMITY MINE X6",
        0x14 => "PRESSURE MINE X2",
        0x15 => "PRESSURE MINE X4",
        0x16 => "PRESSURE MINE X6",
        0x18 => "FRAG GRENADE #1",
        0x19 => "FRAG GRENADE #2",
        0x1B => "BOUNCY GRENADE X4",
        0x1C => "BOUNCY GRENADE X6",
        0x1D => "STICKY GRENADE X4",
        0x1E => "STICKY GRENADE X6",
        0x20 => "ROCKET PACK X1",
        0x21 => "ROCKET PACK X3",
        0x22 => "ROCKET PACK X6",
        0x23 => "ROCKET PACK X9",
        0x25 => "REAPER PACK X1",
        0x26 => "REAPER PACK X2",
        0x27 => "REAPER PACK X4",
        0x28 => "REAPER PACK X6",
        0x2A => "AUTO SHIELDING",
        0x2B => "BATTERY PACK",
        0x2C => "THERMAL DAMPER",
        0x2D => "SCANNER LEVEL 2",
        0x2E => "SCANNER LEVEL 3",
        _ => "ERROR",
    }
}

/// One weapon-table group: the two words the mission reads. Word0 =
/// the weapon NAME index into [`weapon_name`] (0 = no weapon in the
/// group — the row gate). Word1 = the AMMO count (the click/key
/// gate + the displayed count, clamped 9999 at draw). The other
/// five words of the EXW group (price/category/item/stock/owned)
/// are shop-side state the mission never reads [RE-EXW-SIM 7d.2].
pub type WeaponGroup = (u16, u16);

/// The BATTERY PACK name index [RE-EXW-SIM 7f.8]: the equipment
/// group whose word1 is the battery stat — the spawn HP bonus
/// `+100*battery` in the dropship-landing formula.
pub const WEAPON_BATTERY_PACK: u16 = 0x2B;

/// The per-robot weapon table row: 7 groups [sec 6c.6 — the
/// 0x62-stride table row the spawn stats-copy reads].
pub type WeaponLoadout = [WeaponGroup; 7];

/// The sidebar presentation half [RE-EXW-SIM sec 6c; D17 split —
/// none of this enters the sim state hash]: the selected squad slot
/// (`DAT_0046cbdc`), the redraw countdown (`DAT_0046ccec`: producers
/// set 2, the draw tail decrements while nonzero and runs the sidebar
/// redraw pass FUN_00408403 — modeled here as the countdown alone),
/// the per-robot order-bits word (+0x6E), and the per-robot WEAPON
/// LOADOUT — the 7 groups of the 0x62-stride session table at
/// 0x4de664 [7d: .bss session state written by the shop/save/MP
/// paths, NOT file-loaded; the fresh-campaign default is EMPTY]. The
/// host stages a loadout through [`MissionScene::set_weapon_loadout`]
/// (the D51 seam) exactly when a test wants rows on screen.
#[derive(Debug, Default)]
struct Sidebar {
    selected: usize,
    redraw: i32,
    order_bits: Vec<u16>,
    weapons: Vec<WeaponLoadout>,
    scanners: [u8; 12],
    /// The blink-cursor selector `_DAT_004dc5d0` [RE-EXW-SIM 7j.6]:
    /// the SELECTED robot's slot + 1 (1..3), 0 = no cursor.
    /// MissionShell zeroes it at entry (0x447871); the select-ack
    /// blocks in the robots() walk (0x40c1ae..0x40c25e) set it when
    /// the selection lands. [hypothesis: modeled as set-on-select —
    /// the per-frame ack gating is not fully pinned, so the cursor
    /// stays 0 until the sidebar select path fires (never-invent:
    /// the spawn pre-selection may light it from frame 1 in EXW).]
    cursor: i32,
}

impl Sidebar {
    /// Per-robot state at spawn [sec 6c.6 over the real table rule,
    /// 7d.4]: loadout EMPTY (the fresh-campaign default — the shop
    /// runs before every mission and only purchases fill groups), so
    /// the spawn stats-copy arms NO bit; selected slot 0
    /// (load_markers 0x40ce0e), redraw 0 (the MissionShell entry
    /// reset 0x4478bf). hp/armor live on the SIM robots now (the
    /// damage unit, D52 follow-up: spawn 5000/0, 7f.8).
    fn new(robots: usize) -> Sidebar {
        Sidebar {
            selected: 0,
            redraw: 0,
            order_bits: vec![0; robots],
            weapons: vec![[(0, 0); 7]; robots],
            scanners: [0; 12],
            cursor: 0,
        }
    }
}

/// The 10 pickup effect rows [RE-EXW-SIM 7j; D17 split — never
/// enters the sim hash]: the 0xa0-B array at 0x4dc5d4, one row per
/// pickup fire, rising and vanishing. Staged by the pickup host
/// seam through [`MissionScene::pickup`] (the case-tail writes at
/// 0x40ed5e..0x40f26c), ticked by FUN_0042205c each frame
/// (MissionShell 0x448080), drawn by the FUN_00403938 tail pass
/// through the sprite list (FLAGS.BIN sprite id−1, 7j.4).
#[derive(Debug)]
struct EffectRows {
    /// `{x, y, z, id}` per row — id 0 = free.
    rows: [(i32, i32, i32, i32); EFFECT_ROWS],
}

impl EffectRows {
    fn new() -> EffectRows {
        EffectRows {
            rows: [(0, 0, 0, 0); EFFECT_ROWS],
        }
    }

    /// The slot allocator FUN_00422038 [7j.2]: the first row whose
    /// id word is 0, else 9 when all busy (reuse the last row).
    fn alloc(&self) -> usize {
        for (k, r) in self.rows.iter().enumerate() {
            if r.3 == 0 {
                return k;
            }
        }
        EFFECT_ROWS - 1
    }

    /// The case-tail row write [7j.1]: `{pos_x>>8, pos_y>>8,
    /// z+0x20, id}` into the allocated row.
    fn stage(&mut self, x: i32, y: i32, z: i32, id: i32) {
        let r = self.alloc();
        self.rows[r] = (x, y, z, id);
    }

    /// The row tick FUN_0042205c [7j.3]: per active row
    /// `z <= 0x190 → z += 6` else `id = 0`.
    fn tick(&mut self) {
        for r in self.rows.iter_mut() {
            if r.3 == 0 {
                continue;
            }
            if r.2 <= EFFECT_ROW_Z_MAX {
                r.2 += EFFECT_ROW_RISE;
            } else {
                *r = (0, 0, 0, 0);
            }
        }
    }
}

/// One debris-stager record [7j.5]: the modeled subset of the 0x30-B
/// EXW record — the draw/tick gates (active, delay, seq) plus the
/// flush inputs (x, y, z, kind). The +0x10/+0x14 init words (0x40),
/// the +0x20 physics flag (0 for kind 5 — the FUN_0040de9c callback
/// never runs on the death-debris path), and the +0x28 param have no
/// modeled consumer and stay out.
#[derive(Debug, Clone, Copy, Default)]
struct DebrisRec {
    active: bool,
    x: i32,
    y: i32,
    z: i32,
    kind: i32,
    /// The sequence counter (+0x18) — also the LRU eviction key.
    seq: i32,
    /// The start delay (+0x24): the tick decrements it before the
    /// seq walk; the draw pass skips delayed records.
    delay: i32,
}

/// The 128-slot debris stager [RE-EXW-SIM 7j.5/7j.7; D17 split]:
/// the 0x1800-B array at 0x476fbc. The death tail stages five
/// kind-5 records through [`MissionScene::apply_damage`]; the tick
/// (FUN_00420549, MissionShell 0x448076) walks the per-kind i16
/// sequence table and frees the record at the −1 terminator; the
/// FUN_00403938 tail draws active, undelayed records from
/// BLOWUP.BIN (layer 0x12c for kinds 3/7/0xA, else 0x12e).
#[derive(Debug)]
struct DebrisFx {
    recs: [DebrisRec; DEBRIS_SLOTS],
}

impl DebrisFx {
    fn new() -> DebrisFx {
        DebrisFx {
            recs: [DebrisRec::default(); DEBRIS_SLOTS],
        }
    }

    /// The kind-5 staging [7j.5]: clamp z into `[0x20, 0xFF]`, take
    /// the first inactive slot else the one with the SMALLEST seq
    /// counter (the LRU eviction — FUN_00420608 head 0x420666..),
    /// then the record write (seq 0, the 2k start delay
    /// [hypothesis: the +0x24 slot aliases the caller's 2k counter —
    /// the Watcon stack-arg mapping is not fully pinned; flagged
    /// for the P4.2 differential harness]).
    fn stage_kind5(&mut self, x: i32, y: i32, z: i32, delay: i32) {
        let z = z.clamp(DEBRIS_Z_CLAMP.0, DEBRIS_Z_CLAMP.1);
        let slot = {
            let mut best = None;
            for (k, r) in self.recs.iter().enumerate() {
                if !r.active {
                    best = Some(k);
                    break;
                }
                if best.is_none() || r.seq < self.recs[best.unwrap()].seq {
                    best = Some(k);
                }
            }
            best.unwrap_or(DEBRIS_SLOTS - 1)
        };
        self.recs[slot] = DebrisRec {
            active: true,
            x,
            y,
            z,
            kind: DEBRIS_KIND_DEATH,
            seq: 0,
            delay,
        };
    }

    /// The tick FUN_00420549 [7j.7]: per active record — delay != 0
    /// → decrement and hold; else seq += 1 and read the kind-5
    /// table: the −1 terminator frees the record. (The +0x20
    /// physics callback is 0 for kind 5 — never invoked here.)
    fn tick(&mut self) {
        for r in self.recs.iter_mut() {
            if !r.active {
                continue;
            }
            if r.delay != 0 {
                r.delay -= 1;
                continue;
            }
            r.seq += 1;
            let word = match DEBRIS_KIND5_SEQ.get(r.seq as usize) {
                Some(&w) => w as i32,
                None => -1,
            };
            if word == -1 {
                *r = DebrisRec::default();
            }
        }
    }

    /// The flush-facing view of the active records (the draw pass
    /// gates are the consumer's).
    fn views(&self) -> Vec<DebrisSpriteView> {
        self.recs
            .iter()
            .map(|r| DebrisSpriteView {
                active: r.active,
                x: r.x,
                y: r.y,
                z: r.z,
                kind: r.kind,
                seq: match DEBRIS_KIND5_SEQ.get(r.seq as usize) {
                    Some(&w) => w as i32,
                    None => -1,
                },
                delay: r.delay,
            })
            .collect()
    }
}

/// The dither noise ring [RE-EXW-SIM 7i; D17 split — never enters
/// the sim hash]: the 2048-byte static bank at 0x4e6ed8 + its
/// persistent cursor 0x4ddb30 (both .bss in the EXW). Content is
/// binary {0x00, DITHER_WHITE}, 25% white. All random draws come
/// from the caller's shared mission RandB stand-in — the EXW
/// interleaves fill/churn/seeds/reseeds with the terrain edge
/// variants on the ONE RandB stream [7i.4]; the engine consumes
/// its stand-in in the same per-frame order (terrain edges →
/// dither draws → churn), so the stream lives on
/// [`MissionScene::rand_b`], not here.
#[derive(Debug)]
struct Dither {
    bank: Vec<u8>,
    cursor: usize,
}

impl Dither {
    fn new() -> Dither {
        Dither {
            bank: vec![0; DITHER_BANK_LEN],
            cursor: 0,
        }
    }

    /// The boot fill [7i.2, MissionShell staging 0x447b13..0x447b3a]:
    /// 2048 RandB draws, `rand&3 == 0 ? 0xFF : 0x00`. Runs once per
    /// mission entry ([`MissionScene::activate`]); the cursor is
    /// .bss zero at load and the fill does not touch it.
    fn fill(&mut self, rng: &mut Pcg32) {
        for b in self.bank.iter_mut() {
            *b = if rng.next_u32() & 3 == 0 {
                DITHER_WHITE
            } else {
                0
            };
        }
    }

    /// The per-frame churn [7i.2, the MissionShell epilogue
    /// 0x448147..0x448195]: 15 times — advance the cursor (wrapping
    /// ≥ 0x800 → 0), then re-randomize the byte AT the advanced
    /// cursor. The whole ring refreshes every ≈137 frames.
    fn churn(&mut self, rng: &mut Pcg32) {
        for _ in 0..DITHER_CHURN {
            self.cursor += 1;
            if self.cursor >= DITHER_BANK_LEN {
                self.cursor = 0;
            }
            self.bank[self.cursor] = if rng.next_u32() & 3 == 0 {
                DITHER_WHITE
            } else {
                0
            };
        }
    }

    /// The blit seed [7i.3, FUN_0041ec59(0x7f6, 0x30)]:
    /// `(rand & 0x7fff) / 15` clamped ≤ 0x7f5. One draw per blit.
    fn seed(rng: &mut Pcg32) -> usize {
        let (div, max) = DITHER_SEED;
        (((rng.next_u32() & 0x7FFF) / div) as usize).min(max)
    }

    /// The static blit FUN_00401ae6(y, 48, x, 48, src_off, mode)
    /// [7i.1] at the fixed portrait box y = 5: 48 rows of 48 bytes
    /// from the ring at `src_off`, per row FIRST the wrap check
    /// `src_off + DITHER_LOOKAHEAD ≥ 0x800 → src_off = rand & 0x1ff`
    /// (a random reseed into the ring head, not a sequential wrap).
    /// `masked = false` (mode 0, the DEAD/UNOCCUPIED path) copies
    /// every byte including zeros — the box content is REPLACED;
    /// `masked = true` (mode 1, the HIT-FLASH path) writes only the
    /// nonzero bytes — the portrait under zero bytes survives.
    fn blit(&mut self, rng: &mut Pcg32, plane: &mut [u8], pw: usize, x: i32, y: i32, masked: bool) {
        let size = (DITHER_LOOKAHEAD / 2) as i32;
        debug_assert_eq!(size, 48);
        // Charter: never panic on a malformed plane — the sidebar
        // box is in-bounds by construction (x ≤ 0x27B, y+48 ≤ 0x35).
        let in_bounds = x >= 0
            && y >= 0
            && x + size <= pw as i32
            && (y + size) as usize * pw + (x + size) as usize <= plane.len();
        if !in_bounds {
            return;
        }
        let size = size as usize;
        let mut off = Self::seed(rng);
        for row in 0..size {
            if off + DITHER_LOOKAHEAD >= DITHER_BANK_LEN {
                off = (rng.next_u32() & 0x1FF) as usize;
            }
            let dst = (y as usize + row) * pw + x as usize;
            for col in 0..size {
                let b = self.bank[off + col];
                if !masked || b != 0 {
                    plane[dst + col] = b;
                }
            }
            off += size;
        }
    }
}

/// The spawn stats-copy armer over a loadout row [sec 6c.6, verified
/// asm 0x40ceb2..0x40cf70]: `1 << first group whose word0 != 0`,
/// 0 when no group carries a weapon (the fresh-campaign case — the
/// EXW `found` flag simply never sets).
const fn spawn_order_bits(groups: &WeaponLoadout) -> u16 {
    let mut bits = 0u16;
    let mut i = 0;
    while i < 7 {
        if groups[i].0 != 0 {
            bits = 1 << i;
            break;
        }
        i += 1;
    }
    bits
}

/// Robots spawned per player from the MRK records
/// [load_markers, RE-EXW-SIM sec 7c.7, verified]: zones {0,1,2,7} -> 1,
/// zone 3 -> 2, else 3.
pub fn robots_per_player(zone: i32) -> usize {
    if zone < 3 || zone == 7 {
        1
    } else if zone == 3 {
        2
    } else {
        3
    }
}

/// The zone index a campaign stage slot plays [design; the B2
/// order[8] zone table @0x81dba is the DOS-side anchor, the EXW path
/// arithmetic is `EDITOR\ZONE{chr(0x41+zone)}`, sec 7c.1]: stage 1
/// (boot camp) -> zone 0 (A), stages 2..=7 -> zones 1..=6 (B..=G),
/// the endgame stage cap stays at zone 6.
pub fn zone_for_stage(stage: u8) -> i32 {
    (i32::from(stage) - 1).clamp(0, 6)
}

/// The SELECT MP mission-file offset [RE-EXW-SIM §7j.73, verified]:
/// `build_mission_paths` @0x4467df adds 5 to the mission cell when
/// the mode cell [0x4edb88] reads 2 (multiplayer) — the SELECT
/// screen's MP write pair `{zone 2..=6, mission 1..=2}` therefore
/// loads `ZONE{B..F}/MISSION{6,7}.*`: missions 6-7 are the MP-only
/// files, not campaign sub-missions (no stage mask can express
/// them — the census G1 verdict).
pub const SELECT_MP_FILE_OFFSET: i32 = 5;

/// The mission number for a stage's completion mask [design; the
/// Episode::complete lowest-unset-bit arithmetic, the same selection
/// briefing_name_for_slot uses for the BRF letter index]: first
/// uncompleted sub + 1, SATURATED at 5 — the SP SELECT domain
/// [RE-EXW-SIM §7j.73: the SP arm writes missions 1..5 per zone
/// only], so the campaign path can never name an MP file (6/7 come
/// only from the staged SELECT pair).
pub fn mission_number_for_mask(mask: u8) -> i32 {
    let mut sub = 0u8;
    while mask >> sub & 1 != 0 {
        sub += 1;
    }
    (i32::from(sub) + 1).min(5)
}

/// The mission asset names in fetch order [design chain convention:
/// the load_mission path-1 trio (TOT/DAT/PAD, sec 7c.1), then the
/// zone-level path-2 pair (CGR/BIN) + LNK, then the GAMEGFX staging
/// family tail (SINTABLE, DANTE, GAMEPAL, GENERAL, SMLFONT — staged
/// after the mission files in MissionShell, sec 7c header; GAMEPAL
/// is the mission plane palette, MISSIONVIEW sec 6; GENERAL +
/// SMLFONT are the sidebar art banks, sec 6c.8c) and the markers.
/// Names carry the `EDITOR` tree sub-path with '/' separators for
/// the mission files; the byte source resolves them under
/// `EDITOR/` or `GAMEGFX/` [see bedlam-shell GameGfxSource].
pub fn mission_asset_names(zone: i32, mission: i32) -> Vec<String> {
    let zone_dir = format!("ZONE{}", (b'A' + zone as u8) as char);
    let zone_file = format!("MISSION{}", (b'A' + zone as u8) as char);
    let per_mission = format!("{zone_dir}/MISSION{mission}");
    [
        format!("{per_mission}.TOT"),
        format!("{per_mission}.DAT"),
        format!("{per_mission}.PAD"),
        format!("{zone_dir}/{zone_file}.CGR"),
        format!("{zone_dir}/{zone_file}.BIN"),
        format!("{zone_dir}/{zone_file}.LNK"),
        "SINTABLE.BIN".to_string(),
        "DANTE.BIN".to_string(),
        "GAMEPAL.PAL".to_string(),
        "GENERAL.BIN".to_string(),
        "SMLFONT.BIN".to_string(),
        format!("{per_mission}.MRK"),
        // The map-overlay family tail [RE-EXW-SIM 7e]: TABLE.BIN
        // (FUN_0041df10's `GAMEGFX\TABLE.BIN` mission-init staging),
        // the 8 MAPTRAN ramps (FUN_00422171), and the ZONE `.MIN`
        // mask bank (load_mission's `.MIN` load at 0x41dcd8 — the
        // zone-file base, like CGR/BIN/LNK) — appended after MRK so
        // the established indices hold.
        "TABLE.BIN".to_string(),
        "MAPTRAN0.TRN".to_string(),
        "MAPTRAN1.TRN".to_string(),
        "MAPTRAN2.TRN".to_string(),
        "MAPTRAN3.TRN".to_string(),
        "MAPTRAN4.TRN".to_string(),
        "MAPTRAN5.TRN".to_string(),
        "MAPTRAN6.TRN".to_string(),
        "MAPTRAN7.TRN".to_string(),
        format!("{zone_dir}/{zone_file}.MIN"),
        // The score-strip bank [RE-EXW-SIM 7f.9]:
        // `LoadFile("GAMEGFX\NUMBERS.BIN", DAT_0046af3c)` — the
        // mission-init staging, sole consumer FUN_004085ce.
        // Appended last so the established indices hold.
        "NUMBERS.BIN".to_string(),
        // The effect banks [RE-EXW-SIM 7j.4/7j.9]:
        // `LoadFile("GAMEGFX\FLAGS.BIN", 0x46af40)` (the 10 pickup
        // effect rows) + `LoadFile("GAMEGFX\BLOWUP.BIN", 0x4edd6c)`
        // (the debris stager — the region-1 BLOWUPG.BIN variant is
        // the host's path choice, unmodeled region wiring).
        "FLAGS.BIN".to_string(),
        "BLOWUP.BIN".to_string(),
    ]
    .to_vec()
}

/// The staged mission: sim + viewport + the following camera, plus the
/// presentation buffers. INERT until [`MissionScene::activate`] (the
/// host calls it on the Mission-scene entry; DESIGN-GAME sec 11
/// LIFECYCLE, the D31/D37 movie pattern).
#[derive(Debug)]
pub struct MissionScene {
    sim: MissionSim,
    view: MissionView,
    zone: i32,
    /// Q5 camera pair (the EXW `_DAT_004edde4/8` scroll anchor) —
    /// Four rendered-frame anchors, averaged with height subtracted from
    /// both axes (EXW 0x4039df..0x403b39).
    cam_q5: (i32, i32),
    cam_height: i32,
    cam_history: [(i32, i32, i32); 4],
    cam_next: usize,
    /// Pointer in 640x480 screen space, clamped on every integrate
    /// [the menu D42 pattern; EXW clamps in the ISR]. Pinned to the
    /// twin-verified model [D160/RE-EXD-MAP §5h, the P2e package]:
    /// boots at the GameInit center (320,240), clamps into
    /// [9,631]x[9,463].
    cursor: (i32, i32),
    /// Left-button level at the last consumed tick (the D26 hashed
    /// edge-latch analog; only the left bit matters to this seam).
    prev_buttons: u8,
    /// The shared mission RandB stand-in (charter T3 — the EXW
    /// 16-bit-pair stream at 0x4ede4c/0x4ede4e is NOT mirrored).
    /// ONE stream consumed in the EXW per-frame order [7i.4]: the
    /// terrain edge variants (MISSIONVIEW sec 7) → the dither
    /// family draws (fill at activate / seeds + reseeds in the
    /// portrait pass / churn at the frame epilogue).
    /// `Pcg32::new(0x1E240, 0)` — the MissionShell stand-in seed;
    /// zone 0 = ZONEA draws fixed edges and consumes none.
    rand_b: Pcg32,
    /// The dither noise ring [RE-EXW-SIM 7i, D55] — the 0x4e6ed8
    /// static bank + its cursor, filled at activate, churned at
    /// the frame epilogue, read by the portrait pass.
    dither: Dither,
    /// The 10 pickup effect rows [RE-EXW-SIM 7j.1, D17 split]:
    /// staged by the pickup seam, ticked + drawn per frame.
    effect_rows: EffectRows,
    /// The 128-slot debris stager [RE-EXW-SIM 7j.5, D17 split]:
    /// staged by the damage seam on death, ticked + drawn per
    /// frame.
    debris: DebrisFx,
    /// The 0x64000 viewport buffer (DAT_004ede18).
    buf: Vec<u8>,
    /// The 640x480 presentation plane: the 480x480 present window at
    /// (0,0) — the EXW mission screen is viewport [0,480)x[0,480) +
    /// sidebar [480,640) [sec 6.2], NOT letterbox-centered.
    plane: Vec<u8>,
    /// The folded GAMEPAL palette the mission plane presents under
    /// [MISSIONVIEW sec 6: GAMEPAL loads into the 0x4edbf8 0x302-B
    /// blob the mission-load pass copies to 0x4ddb34, SIM sec 7c.3].
    palette: [Vga6; 256],
    /// GAMEGFX\GENERAL.BIN staged bytes (`_DAT_004edd7c`): the
    /// sidebar art bank — select portraits 0x12..0x17, order-row
    /// chrome 0x47/0x49 + 0x4A/0x4C, HP/armor bars [sec 6c.8c].
    general: Vec<u8>,
    /// `LoadFile("GAMEGFX\SMLFONT.BIN", _DAT_004ede7c)`.
    /// SMLFONT.BIN (63 glyphs) is the sidebar text bank [6c.8c];
    /// no text draws until the type table lands (never invented).
    smlfont: Vec<u8>,
    /// `LoadFile("GAMEGFX\NUMBERS.BIN", DAT_0046af3c)` [7f.9]: the
    /// score/money strip bank (12 sprites: digits 0..9 9x11, 0xA
    /// 100x11, 0xB 74x11); sole consumer the strip pass.
    numbers: Vec<u8>,
    /// `LoadFile("GAMEGFX\TABLE.BIN", [0x46cbbc])` — the strategic
    /// map backdrop bank, image 0 a 480×480 RLE sprite [7e.1b].
    table: Vec<u8>,
    /// The strategic-map overlay half [RE-EXW-SIM 7e; D17 split —
    /// outside the sim hash]: the `.MIN` mask bank + the 8 MAPTRAN
    /// ramps + the territory variant bytes.
    overlay: MapOverlay,
    /// The overlay draw-mode bit `_DAT_004edba0` [7e.5]: toggled by
    /// the map strip, zeroed at MissionShell entry (0x44786b).
    overlay_on: bool,
    /// The re-fire lockout `_DAT_004eb8dc` [7e.5].
    map_lockout: i32,
    /// The sidebar presentation half [sec 6c; D17 split — outside
    /// the sim hash].
    sidebar: Sidebar,
    /// The campaign session state the strip reads [RE-EXW-SIM 7f.6/
    /// 7f.9, D52]: score `_DAT_004dd40c` + money `DAT_0046ae70` —
    /// EXW globals written by GameMain/shop/pickups/save-load (none
    /// of those shells are modeled, so the host stages the campaign
    /// point through [`MissionScene::set_campaign`]; producers only
    /// add). FRESH_CAMPAIGN = (0, 4000).
    score: i32,
    money: i32,
    /// The score-strip countdown `0x46ccf0` [7f.3]: producers set 2,
    /// the present tail decrements while nonzero and draws the strip
    /// (MissionShell entry zero 0x4478c5, the post-load trigger
    /// 0x447c7a sets 2 — the strip draws on the entry frames).
    strip: i32,
    /// Presents executed (the one-render-per-host-frame rhythm).
    render_count: u64,
    active: bool,
}

impl MissionScene {
    /// `Terrain` from DAT+PAD+CGR, the angle table from SINTABLE
    /// words 2..66, `MissionSim` seeded 0x1E240 (the MissionShell
    /// reseed), the first `robots_override.unwrap_or(
    /// robots_per_player(zone))` MRK records spawned verbatim, then
    /// any staged markers (the host/test seam the network override
    /// 0x46cbe0 fills in the original, sec 7c.8), and the viewport
    /// over TOT + swept DAT planes + BIN + LNK with DANTE staged.
    /// GAMEPAL (770 B, the parse_vga770 family) folds to the
    /// canonical 6-bit palette and owns the plane [MISSIONVIEW
    /// sec 6]. GENERAL.BIN + SMLFONT.BIN stage as the sidebar art
    /// banks [sec 6c.8c] and NUMBERS.BIN as the score-strip bank
    /// [7f.9] plus FLAGS.BIN/BLOWUP.BIN as the effect banks [7j].
    /// Malformed bytes -> [`GameError::BadMissionAsset`],
    /// never a panic (charter); nothing is mutated on error.
    #[allow(clippy::too_many_arguments)]
    pub fn stage(
        tot: &[u8],
        dat: &[u8],
        pad: &[u8],
        cgr: &[u8],
        mrk: &[u8],
        bin: &[u8],
        lnk: &[u8],
        sintable: &[u8],
        dante: &[u8],
        gamepal: &[u8],
        general: &[u8],
        smlfont: &[u8],
        numbers: &[u8],
        flags: &[u8],
        blowup: &[u8],
        table: &[u8],
        min: &[u8],
        maptran: &[&[u8]],
        zone: i32,
        robots_override: Option<usize>,
        staged_markers: &[(i32, i32, i32)],
    ) -> Result<MissionScene, GameError> {
        let bad =
            |what: &'static str, reason: &'static str| GameError::BadMissionAsset { what, reason };
        let terrain = Terrain::from_mission_bytes(dat, pad, cgr)
            .ok_or_else(|| bad("DAT/PAD/CGR", "malformed mission terrain bytes"))?;
        let mut words = [0i16; 256];
        if sintable.len() < 512 {
            return Err(bad("SINTABLE", "shorter than 256 words"));
        }
        for (i, w) in words.iter_mut().enumerate() {
            *w = i16::from_le_bytes([sintable[2 * i], sintable[2 * i + 1]]);
        }
        let angles = AngleTable::from_sintable_words(&words)
            .ok_or_else(|| bad("SINTABLE", "short words array"))?;
        // GAMEPAL folds exactly like the loading palettes (6-bit file
        // values; the expand/fold round trip is lossless).
        let palette = crate::loading::loading_palette(gamepal)
            .map_err(|_| bad("GAMEPAL", "not a 770-byte VGA palette"))?;
        let mut sim = MissionSim::new(terrain, angles, 0x1E240);
        // MRK: 12 staged 16-B records `(flag, x, y, z-level)`; robot i
        // takes record i verbatim (flag dropped) [sec 7c.7, verified].
        let count = robots_override.unwrap_or_else(|| robots_per_player(zone));
        if mrk.len() < 16 * count {
            return Err(bad("MRK", "fewer marker records than robots"));
        }
        for i in 0..count {
            let rec = &mrk[16 * i..16 * i + 16];
            let word = |o: usize| {
                i32::try_from(u32::from_le_bytes([
                    rec[o],
                    rec[o + 1],
                    rec[o + 2],
                    rec[o + 3],
                ]))
                .unwrap_or(0)
            };
            sim.spawn_robot((word(4), word(8), word(12)));
        }
        for &marker in staged_markers {
            sim.spawn_robot(marker);
        }
        // The viewport reads the swept PRE-PAD plane bytes (the seen
        // marks compare DAT bytes against zero).
        let planes = bedlam_core::mission::dat_plane_bytes(dat)
            .ok_or_else(|| bad("DAT", "malformed plane bytes"))?;
        let mut view = MissionView::from_mission_bytes(tot, &planes, bin, lnk)
            .ok_or_else(|| bad("TOT/BIN/LNK", "malformed viewport bytes"))?;
        view.set_entity_bank(dante);
        // The effect banks [RE-EXW-SIM 7j.4/7j.9]: FLAGS.BIN (the
        // pickup effect rows) + BLOWUP.BIN (the debris stager; the
        // region-1 BLOWUPG.BIN variant is a host path choice).
        view.set_flags_bank(flags);
        view.set_blowup_bank(blowup);
        // The strategic-map overlay [RE-EXW-SIM 7e]: the mission's
        // `.MIN` mask bank + the eight 256-byte MAPTRAN ramps over
        // the map dims (malformed ramps are a staging error — the
        // EXW arena always holds eight).
        let (mw, mh) = view.size();
        let overlay = MapOverlay::new(min, maptran, mw, mh)
            .ok_or_else(|| bad("MIN/MAPTRAN", "not eight 256-byte ramps or bad map size"))?;
        let sidebar = Sidebar::new(sim.robots().len());
        let (score, money) = FRESH_CAMPAIGN;
        Ok(MissionScene {
            sim,
            view,
            zone,
            cam_q5: (0, 0),
            cam_height: 0,
            cam_history: [(0, 0, 0); 4],
            cam_next: 0,
            cursor: (CURSOR_BOOT_X, CURSOR_BOOT_Y),
            prev_buttons: 0,
            rand_b: Pcg32::new(0x1E240, 0),
            dither: Dither::new(),
            effect_rows: EffectRows::new(),
            debris: DebrisFx::new(),
            buf: vec![0u8; VIEW_BUF_LEN],
            plane: vec![0u8; 640 * 480],
            palette,
            general: general.to_vec(),
            smlfont: smlfont.to_vec(),
            numbers: numbers.to_vec(),
            table: table.to_vec(),
            overlay,
            overlay_on: false,
            map_lockout: 0,
            sidebar,
            score,
            money,
            strip: 0,
            render_count: 0,
            active: false,
        })
    }

    /// Seed the camera history from the first robot (EXW 0x40d146) and
    /// arm the initial sidebar draw (MissionShell 0x447C74 sets the
    /// redraw countdown 2 after the mission-load calls [6c.8e], so
    /// the rows draw on the entry frames). Idempotent.
    pub fn activate(&mut self) {
        if self.active {
            return;
        }
        let anchor = self.sim.robots().first().map_or((0, 0, 0), |r| {
            let (x, y) = r.q5();
            (x, y, r.z)
        });
        self.cam_history.fill(anchor);
        self.cam_next = 0;
        self.cam_height = anchor.2;
        self.cam_q5 = (anchor.0 - anchor.2, anchor.1 - anchor.2);
        self.sidebar.redraw = 2;
        // The score-strip countdown arms alongside the redraw one
        // (MissionShell 0x447C74/0x447C7A set BOTH `0x46ccec` and
        // `0x46ccf0` to 2 after the mission-load calls [7f.3/6c.8e])
        // — the strip draws on the entry frames too.
        self.strip = 2;
        // MissionShell entry zeroes the overlay bit + the re-fire
        // lockout family [7e.5, asm 0x44786b / 0x44871d].
        self.overlay_on = false;
        self.map_lockout = 0;
        // The dither bank boot fill [7i.2, MissionShell staging
        // 0x447b13]: 2048 RandB draws BEFORE the first frame — the
        // stream order the shared stand-in mirrors (edges → dither).
        self.dither.fill(&mut self.rand_b);
        self.active = true;
    }

    /// Whether the scene owns the Mission screen yet.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Current height-adjusted Q5 terrain camera.
    pub fn camera(&self) -> (i32, i32) {
        self.cam_q5
    }

    /// The pointer position in screen space.
    pub fn cursor(&self) -> (i32, i32) {
        self.cursor
    }

    /// Presents executed since staging.
    pub fn render_count(&self) -> u64 {
        self.render_count
    }

    /// The hashed sim half (gate seam).
    pub fn sim(&self) -> &MissionSim {
        &self.sim
    }

    /// The sim half, mutable (the host/test order seam — the D17
    /// presentation half never needs this).
    pub fn sim_mut(&mut self) -> &mut MissionSim {
        &mut self.sim
    }

    /// The map size in tiles (the TOT header — the overlay's
    /// territory indexing).
    pub fn view_size(&self) -> (i32, i32) {
        self.view.size()
    }

    /// The sim state hash (the D17 hashed half of the composition).
    pub fn state_hash(&self) -> StateHash {
        self.sim.state_hash()
    }

    /// One executed 60 Hz tick [DESIGN-GAME sec 11 PER FRAME; the
    /// MissionShell order, RE-EXW-SIM sec 1]: integrate the pointer
    /// from mouse deltas (the original [9,631]x[9,463] box, D160),
    /// run the click seam on a left-button EDGE — the sidebar
    /// producer at `x >= 0x1E0` [sec 6c], the robot arm below it
    /// [sec 6.4] — then `advance_frame` (the six unit-manager phases
    /// + the order-window tick). Inert until [`MissionScene::activate`].
    pub fn tick(&mut self, input: &InputFrame) {
        if !self.active {
            return;
        }
        // Integrate the pointer, pinned to the twin-verified model
        // [D160/RE-EXD-MAP §5h, the P2e package]: clamp into
        // [9,631]x[9,463] on every integrate (EXW ScrollUpdate
        // 0x425b2e..0x425b84; EXD poll 0x12615..0x12659, mickey
        // integrate-then-clamp — the 9 = the 24x24 cursor-sprite
        // hotspot offset), boot at the GameInit center (320,240) —
        // the constants are the bedlam-core frame ones (the
        // classic-input adapter's own box, D160). Click-seam audit:
        // the x >= 0x1E0 sidebar gate below IS the original gate twin
        // (EXD poll 0x1268f: [0x1074b0] >= 480 flips the cursor-sprite
        // selector; 480 <= 631, inside the box), and every scripted
        // click target in this model's tests lands inside the box.
        self.cursor.0 =
            (self.cursor.0 + i32::from(input.mouse_dx)).clamp(CURSOR_MIN_X, CURSOR_MAX_X);
        self.cursor.1 =
            (self.cursor.1 + i32::from(input.mouse_dy)).clamp(CURSOR_MIN_Y, CURSOR_MAX_Y);
        let left = input.mouse_buttons & 0x01;
        if self.prev_buttons == 0 && left != 0 {
            if self.cursor.0 >= 0x1E0 {
                self.sidebar_control();
            } else if self.overlay_on {
                // Overlay on: game-area clicks are swallowed
                // [7e.5, asm 0x40b868 — the dispatch jumps past the
                // order seam while `_DAT_004edba0` is set].
            }
        }
        if left != 0 && self.cursor.0 < 0x1E0 && !self.overlay_on {
            self.move_to_ground();
        }
        self.prev_buttons = left;
        // MissionShell's per-frame lockout decrement [7e.5,
        // 0x44871d..0x44872a].
        if self.map_lockout > 0 {
            self.map_lockout -= 1;
        }
        self.sim.advance_frame();
        // The destroy-score fold [§7j.13/3 + D104]: the destroy tail
        // accumulates the award in the sim's pending cell; the shell
        // folds it into the campaign score (the [0x4dd40c] delta the
        // score strip reads) and the strip-redraw arm rides with it
        // (presentation — the strip countdown has its own producers).
        // Zero without staged destructibles — the S0..S3 no-inject
        // invariant (the resolvers pass through on empty banks).
        let (award, _strip) = self.sim.take_destroy_score();
        if award != 0 {
            self.score += award;
        }
        // The case-4 pickup folds [7f.6 + §7h.5/2]: the sim's tile
        // consume (fire_pickup → apply_pickup case 4) draws the
        // award on the SHARED mission stream and stages it; the
        // shell folds it into the session cells the strips read
        // ([0x4dd40c]/[0x46ae70]) and arms the score-strip
        // countdown exactly like the host seam. Zero without
        // pickup fires — the no-inject invariant (ZONEA stages no
        // in-range words, §7h.4/5).
        let (ps, pm) = self.sim.take_pickup_awards();
        if ps != 0 {
            self.score = self.score.wrapping_add(ps);
            self.strip = 2;
        }
        if pm != 0 {
            self.money = self.money.wrapping_add(pm);
            self.strip = 2;
        }
        // FUN_00408dcc — the territory ring stamp runs inside
        // robots() for the moving-family machines [7e.3]; the
        // engine's mover state word is the analog [design: the EXW
        // state-2 gate maps to the walking robot].
        for robot in self.sim.robots() {
            if robot.alive && robot.state == STATE_MOVING {
                let tx = ((robot.pos_x >> 8) + 0x10) >> 5;
                let ty = ((robot.pos_y >> 8) + 0x10) >> 5;
                self.overlay.stamp_territory(tx, ty);
            }
        }
    }

    /// The sidebar producer (mouse subset of sidebar_control@0040d197
    /// [RE-EXW-SIM sec 6c]): the map-toggle strip FIRST [sec 6c.1,
    /// asm 0x40d19d — the strip precedes the select strips], then
    /// robot-select strips + the 7 order rows, gated exactly like
    /// the asm — alive robots only, squad slot within the spawned
    /// group, order availability per robot. The strip writes the
    /// 5-frame lockout and toggles the overlay bit [7e.5]. Keyboard
    /// latches (incl. the strip's MSpace twin) wait for the P2e
    /// button map.
    fn sidebar_control(&mut self) {
        let (x, y) = self.cursor;
        // Map-toggle strip [sec 6c.1]: fires iff the lockout is
        // spent; toggles the overlay bit + re-arms the lockout.
        let (x0, x1, y0, y1) = MAP_TOGGLE_RECT;
        if (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
            if self.map_lockout == 0 {
                self.overlay_on = !self.overlay_on;
                self.map_lockout = MAP_TOGGLE_LOCKOUT;
            }
            return;
        }
        // Robot-select strips [sec 6c.2]: squad slot = strip index,
        // gated by the spawned group size (the DAT_0046cbd8 analog)
        // and the target's ALIVE word.
        for (slot, &(lo, hi)) in SIDEBAR_SELECT_STRIPS.iter().enumerate() {
            if (lo..=hi).contains(&x)
                && (SIDEBAR_SELECT_STRIP_Y.0..=SIDEBAR_SELECT_STRIP_Y.1).contains(&y)
            {
                if slot < self.sim.robots().len() && self.sim.robots()[slot].alive {
                    self.sidebar.selected = slot;
                    self.sidebar.redraw = 2;
                    // The select-ack blink cursor [7j.6, the
                    // robots() blocks 0x40c1ae..0x40c25e]: cursor
                    // = the selected SLOT + 1 (the SFX pair
                    // 0xC+k/0xF is the unmodeled mission SFX tier).
                    self.sidebar.cursor = slot as i32 + 1;
                }
                return;
            }
        }
        // Order rows [sec 6c.4]: row = (y - 0x57)/14 clamped to 6,
        // gate = the selected robot's group AMMO word (record
        // +0x38+8k — the count word, NOT the name gate; sec 6c.3),
        // toggle the bit in its order-bits word.
        let (x0, x1, y0, y1) = SIDEBAR_ORDER_RECT;
        if (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
            let row = (((y - SIDEBAR_ORDER_ROW.0) / SIDEBAR_ORDER_ROW.1) as usize).min(6);
            let robot = self.sidebar.selected;
            if let Some(groups) = self.sidebar.weapons.get(robot) {
                if groups[row].1 != 0 {
                    self.sidebar.order_bits[robot] ^= 1 << row;
                    self.sidebar.redraw = 2;
                }
            }
        }
    }

    /// Left-button movement producer, EXW 0x40b892..0x40b969.
    /// Default zoom is 480; the separately traced right-button firing
    /// projection must not be used here (its vertical offset differs).
    fn move_to_ground(&mut self) {
        let selected = self.sidebar.selected;
        if self.sim.robots().get(selected).is_none() {
            return;
        }
        let (width, height) = self.sim.terrain.size();
        let vx = self.cursor.0 - 240;
        let vy = self.cursor.1 - 240 + self.cam_height - 8;
        let x = (self.cam_q5.0 + (vx >> 1) + vy).clamp(0, width * 32);
        let y = (self.cam_q5.1 - (vx >> 1) + vy).clamp(0, height * 32);
        self.sim.stage_command_record(CommandRecord {
            marker: 0,
            id: selected as i16,
            spot: 0,
            flags: 1,
            x: x as i16,
            y: y as i16,
            z: 0,
        });
    }

    /// Renderer-owned four-sample selected-anchor history, EXW
    /// 0x4039df..0x403b39. Simulation subticks do not advance this ring.
    fn follow_camera(&mut self) {
        if let Some(robot) = self.sim.robots().get(self.sidebar.selected) {
            let (x, y) = robot.q5();
            self.cam_history[self.cam_next] = (x, y, robot.z);
            self.cam_next = (self.cam_next + 1) % 4;
            let (x, y, z) = self
                .cam_history
                .iter()
                .fold((0, 0, 0), |sum, p| (sum.0 + p.0, sum.1 + p.1, sum.2 + p.2));
            self.cam_height = z / 4;
            self.cam_q5 = (x / 4 - self.cam_height, y / 4 - self.cam_height);
        }
    }

    /// One present [DESIGN-GAME sec 11; MISSIONVIEW secs 5d/7]:
    /// enqueue the robots (camera Q5, shake 0, the sim frame), run
    /// the terrain pass into the 0x64000 buffer, crop the 480x480
    /// present window with the fine-camera offset, and blit it at
    /// canonical (0, 0) of the 640x480 plane. Then the SIDEBAR ART
    /// half [RE-EXW-SIM 6c.8 + 7f, the CORRECTED FUN_00403938 tail
    /// order 7f.3]: the select portraits every present
    /// (FUN_004072bf — squad-size + alive + hp gates + the dead/hit
    /// DITHER blit FUN_00401ae6 over the box, D55), the HP/armor
    /// bars every present (FUN_0040807f — the sim robot fields),
    /// the score strip on its own countdown (FUN_004085ce —
    /// NUMBERS.BIN, armed 2 at activate like the redraw one), the
    /// order-row chrome on the redraw countdown (FUN_00408403 —
    /// armed rows 0x47+0x4A, unarmed 0x49+0x4C, rows gated by the
    /// availability bit; the decrements-then-draws rhythm [asm
    /// 0x407205..0x407217]), and the map button chrome 0x5E at the
    /// tail's very end [7e.5]. THE OVERLAY REPLACES ALL OF IT when
    /// the bit is set [7e.1f]: FUN_004089b1 never returns (JMP
    /// 0x4072b8), so an overlay frame clears the viewport half,
    /// draws the strategic map (backdrop + territory stamps + robot
    /// markers) and skips the sidebar passes + chrome. The frame
    /// epilogue then churns the dither ring [7i.2 — the MissionShell
    /// epilogue runs on overlay frames too]. The deploy panel and
    /// the blink cursor stay unwired — each needs state the slice
    /// does not model (never invented); the PAD/order markers
    /// 0x57..0x59 need the unmodeled order staging. Advances the
    /// LNK walk + the shared stand-in stream once (terrain edges →
    /// dither draws → churn, the EXW RandB order 7i.4) — one render
    /// per host frame (D17 bucket b). Inert until active.
    pub fn present(&mut self) -> Option<&[u8]> {
        if !self.active {
            return None;
        }
        if self.overlay_on {
            // FUN_004089b1 [7e.1]: clear the presented 480×480 (the
            // 0x4b000 rep-stos), then the overlay draw. The sidebar
            // half keeps its stale pixels (the EXW surface is not
            // cleared past the presented window).
            for row in 0..480usize {
                let dst = row * 640;
                self.plane[dst..dst + 480].fill(0);
            }
            let robots: Vec<_> = self
                .sim
                .robots()
                .iter()
                .enumerate()
                .filter(|&(_, r)| r.alive)
                .map(|(slot, r)| OverlayRobot {
                    x: r.pos_x,
                    y: r.pos_y,
                    z: r.z,
                    selected: slot == self.sidebar.selected,
                })
                .collect();
            self.overlay.draw(
                &mut self.plane,
                640,
                &mut self.view,
                &self.table,
                &self.general,
                &robots,
            );
            // The MissionShell frame epilogue runs the effect ticks
            // on overlay frames too [7j.3/7j.7 — the epilogue calls
            // at 0x448076/0x448080 precede the overlay branch in
            // FUN_00403938]: debris first, then the rows.
            self.debris.tick();
            self.effect_rows.tick();
            // The MissionShell frame epilogue still churns the
            // dither ring on overlay frames [7i.2] — the sidebar
            // passes are skipped, the churn is not.
            self.dither.churn(&mut self.rand_b);
            self.render_count += 1;
            return Some(&self.plane);
        }
        // The MissionShell frame epilogue order [7j.3/7j.7, calls
        // 0x448076/0x448080 → the draw 0x448094]: the debris tick,
        // then the row tick, then the draw consumes the POST-tick
        // state.
        self.debris.tick();
        self.effect_rows.tick();
        // The terrain pass first [the EXW consumes the shared RandB
        // stream in this order, 7i.4: edge variants → dither draws]:
        // enqueue the robots (camera Q5, shake 0, the sim frame),
        // then the effect rows + debris records into the SAME list
        // [7j.4/7j.9 — the tail passes of FUN_00403938; bucket
        // insertion is sort-keyed, so order does not affect output],
        // run the terrain pass into the 0x64000 buffer, crop the
        // 480x480 present window with the fine-camera offset, and
        // blit it at canonical (0, 0) of the 640x480 plane (the
        // sidebar half x ≥ 480 never overlaps it — the pixel output
        // is identical to the old draw order).
        self.follow_camera();
        let robots: Vec<_> = self.sim.robots().iter().map(RobotView::from_sim).collect();
        self.view
            .enqueue_robots(&robots, self.cam_q5.0, self.cam_q5.1, 0, self.sim.frame());
        let rows: Vec<EffectRowView> = self
            .effect_rows
            .rows
            .iter()
            .map(|&(x, y, z, id)| EffectRowView { x, y, z, id })
            .collect();
        let debris = self.debris.views();
        self.view
            .enqueue_effects(&rows, &debris, self.cam_q5.0, self.cam_q5.1);
        let (cam_x, cam_y) = self.cam_q5;
        let zone = self.zone;
        self.view.draw_terrain(
            &mut self.buf,
            &mut DrawParams::new(cam_x >> 5, cam_y >> 5, zone, &mut self.rand_b),
        );
        let win = present_window(&self.buf, cam_x, cam_y)?;
        for row in 0..480usize {
            let dst = row * 640;
            self.plane[dst..dst + 480].copy_from_slice(&win[row * 480..(row + 1) * 480]);
        }
        // Then the SIDEBAR ART half [RE-EXW-SIM 6c.8 + 7f, the
        // CORRECTED FUN_00403938 tail order 7f.3]: the select
        // portraits every present (FUN_004072bf — squad-size +
        // alive + hp gates + the DITHER blit FUN_00401ae6, D55),
        // the HP/armor bars every present (FUN_0040807f — the sim
        // robot fields, D53), the score strip on its own countdown
        // (FUN_004085ce — NUMBERS.BIN, armed 2 at activate like the
        // redraw one), and the order-row chrome on the redraw
        // countdown (FUN_00408403 — armed rows 0x47+0x4A, unarmed
        // 0x49+0x4C, rows gated by the availability bit; the
        // decrements-then-draws rhythm [asm 0x407205..0x407217]).
        self.draw_sidebar_portraits();
        self.draw_sidebar_bars();
        if self.strip > 0 {
            self.strip -= 1;
            self.draw_score_strip();
        }
        if self.sidebar.redraw > 0 {
            self.sidebar.redraw -= 1;
            self.draw_sidebar_rows();
        }
        // The map button chrome [7e.5, asm 0x4072a7]: GENERAL.BIN
        // 0x5E at (0x213, 0x1b5) — the last tail draw.
        let (chrome, cx, cy) = MAP_BUTTON_SPRITE;
        draw_sprite(&mut self.plane, 640, &self.general, chrome, cx, cy, true);
        // The MissionShell frame epilogue churn [7i.2]: 15 ring
        // bytes re-randomize AFTER the render, every frame.
        self.dither.churn(&mut self.rand_b);
        self.render_count += 1;
        Some(&self.plane)
    }

    /// The FUN_00408403 order-row pass [sec 6c.8a]: 7 rows over the
    /// SELECTED robot, row i drawn iff its group word0 (the NAME
    /// index) is nonzero — no weapon, no row; armed rows (the
    /// order-bits word bit i) draw sprites 0x47 + 0x4A, unarmed rows
    /// 0x49 + 0x4C at (0x1EB, 0x59+14i) and (0x25A, 0x59+14i) from
    /// GENERAL.BIN; then the NAME text [`weapon_name`] at (0x1ED,
    /// 0x5B+14i) and the COUNT as "%04i" of the ammo word clamped
    /// 9999 (the 4-digit template @0x457A28/0x457A2E) at (0x25C,
    /// 0x5B+14i), both color 0x24 through SMLFONT [asm
    /// 0x4084f4..0x408549]. Presentation half only.
    fn draw_sidebar_rows(&mut self) {
        let robot = self.sidebar.selected;
        let Some(groups) = self.sidebar.weapons.get(robot) else {
            return;
        };
        let bits = self.sidebar.order_bits.get(robot).copied().unwrap_or(0);
        let (y0, pitch) = SIDEBAR_ROW_SPRITE_Y;
        let (name_x, count_x, text_y, text_pitch, color) = SIDEBAR_ROW_TEXT;
        for (i, &(name_idx, ammo)) in groups.iter().enumerate() {
            if name_idx == 0 {
                continue; // no weapon in this group
            }
            let armed = usize::from(bits >> i & 1 == 0);
            let y = y0 + pitch * i as i32;
            let (body, well) = SIDEBAR_ROW_SPRITES[armed];
            draw_sprite(
                &mut self.plane,
                640,
                &self.general,
                body,
                SIDEBAR_ROW_SPRITE_X.0,
                y,
                true,
            );
            draw_sprite(
                &mut self.plane,
                640,
                &self.general,
                well,
                SIDEBAR_ROW_SPRITE_X.1,
                y,
                true,
            );
            let ty = text_y + text_pitch * i as i32;
            let bank = &self.smlfont;
            draw_smlfont_text(
                &mut self.plane,
                bank,
                weapon_name(name_idx),
                color,
                name_x,
                ty,
            );
            let count = ammo.min(9999);
            let text = format!("{:04}", count);
            draw_smlfont_text(&mut self.plane, bank, &text, color, count_x, ty);
        }
    }

    /// The FUN_004072bf select-portrait pass [sec 6c.8d + 7f.4 +
    /// the dither 7i.3, D55]: per slot k of the 3-box strip, in
    /// order: inside the squad (k < spawned count) with alive AND
    /// sim hp ≥ 1 → the 48x48 portrait (0x12+k when selected,
    /// 0x15+k otherwise) at (0x1E7+0x32*k, 5), THEN if the sim
    /// hit_flash word != 0 the DITHER blit mode 1 (the sparse
    /// nonzero-only overlay — the portrait under zero bytes
    /// survives; the flash DECAY is the sim's 7g.8 per-frame tick,
    /// this pass only reads it); dead or hp < 1 → NO portrait and
    /// the blit mode 0 (the full static replaces the box); k ≥
    /// squad size → the blit mode 0 every frame (the unoccupied
    /// boxes are pure static — the EXW dithers them
    /// unconditionally, asm 0x4073d8/0x4073fc). Presentation half
    /// only.
    fn draw_sidebar_portraits(&mut self) {
        let selected = self.sidebar.selected;
        let squad = self.sim.robots().len();
        let (x0, pitch, y) = SIDEBAR_PORTRAIT_XY;
        for slot in 0..3usize {
            let x = x0 + pitch * slot as i32;
            let robot = if slot < squad {
                self.sim.robots().get(slot)
            } else {
                None
            };
            let (alive, hp, flash) = match robot {
                Some(r) => (r.alive, r.hp, u32::from(r.hit_flash)),
                None => (false, 0, 0),
            };
            if alive && hp >= 1 {
                let id = if slot == selected {
                    SIDEBAR_PORTRAIT_IDS.0
                } else {
                    SIDEBAR_PORTRAIT_IDS.1
                } + slot as u16;
                draw_sprite(&mut self.plane, 640, &self.general, id, x, y, true);
                if flash != 0 {
                    self.dither
                        .blit(&mut self.rand_b, &mut self.plane, 640, x, y, true);
                }
            } else {
                self.dither
                    .blit(&mut self.rand_b, &mut self.plane, 640, x, y, false);
            }
        }
        // The blink-cursor tail [RE-EXW-SIM 7j.6, asm
        // 0x407420..0x407989]: cursor ∈ {1,2,3} → GENERAL.BIN
        // sprite `(g_frame_count & 3) + 0x51` at
        // (0x1F0 + 0x32*slot, 0xD) — the render_count drives the
        // blink (the scene's per-frame rhythm). Any other value
        // (0 = the MissionShell entry state) draws nothing.
        let cursor = self.sidebar.cursor;
        if (1..=3).contains(&cursor) {
            let (base, x0, pitch, y) = SIDEBAR_BLINK_SPRITE;
            let frame = base + (self.render_count as u16 & 3);
            let slot = cursor - 1;
            draw_sprite(
                &mut self.plane,
                640,
                &self.general,
                frame,
                x0 + pitch * slot,
                y,
                true,
            );
        }
    }

    /// The FUN_0040807f HP + armor bar pass [RE-EXW-SIM 7f.1]:
    /// slot k (< squad size, the `DAT_0046cbd8 > k` analog) draws
    /// the HP bar at (0x1E8+0x32*k, 0x3C) and the armor bar at
    /// (slot_x, 0x49), both GENERAL.BIN transp — HP sprite
    /// `0x46 - min(hp,5000)*46/5000` (hp ≤ 0 → 0x46), armor gate
    /// `0 == word → 0x8E` else `0x8E - min(armor,2500)*46/2500`
    /// clamped ≤ 0x8D. The armor-0 case DRAWS the empty 0x8E bar —
    /// the fresh campaign shows both bars every frame exactly like
    /// the original. Every present; reads the SIM robot fields (the
    /// damage unit, D52 follow-up).
    fn draw_sidebar_bars(&mut self) {
        let (x0, pitch) = SIDEBAR_BAR_X;
        let (hp_y, armor_y) = SIDEBAR_BAR_Y;
        for (slot, robot) in self.sim.robots().iter().enumerate().take(3) {
            let x = x0 + pitch * slot as i32;
            draw_sprite(
                &mut self.plane,
                640,
                &self.general,
                hp_bar_sprite(robot.hp),
                x,
                hp_y,
                true,
            );
            draw_sprite(
                &mut self.plane,
                640,
                &self.general,
                armor_bar_sprite(robot.armor),
                x,
                armor_y,
                true,
            );
        }
    }

    /// The FUN_004085ce score/money strip [RE-EXW-SIM 7f.2]:
    /// NUMBERS.BIN transp — icon 0xA @ (0x1FE,0x18E) + the nine
    /// UNSIGNED score digits at the exact x table (10^8..10^0),
    /// icon 0xB @ (0x20B,0x1A4) + the six money digits (the EXW
    /// divides SIGNED — identical on the ≥ 0 domain the producers
    /// can reach; negative money is outside the modeled session
    /// state). On the `0x46ccf0` countdown (armed 2 at activate —
    /// MissionShell 0x447c7a), decrement-then-draw. Presentation
    /// half only (the campaign state is session, not sim).
    fn draw_score_strip(&mut self) {
        let (icon, ix, iy) = SCORE_STRIP_ICON;
        draw_sprite(&mut self.plane, 640, &self.numbers, icon, ix, iy, true);
        let score = self.score as u64;
        for (k, &x) in SCORE_STRIP_XS.iter().enumerate() {
            let digit = (score / 10u64.pow(8 - k as u32) % 10) as u16;
            draw_sprite(&mut self.plane, 640, &self.numbers, digit, x, iy, true);
        }
        let (icon, ix, iy) = MONEY_STRIP_ICON;
        draw_sprite(&mut self.plane, 640, &self.numbers, icon, ix, iy, true);
        let money = self.money;
        for (k, &x) in MONEY_STRIP_XS.iter().enumerate() {
            let digit = (money / 10i32.pow(5 - k as u32) % 10) as u16;
            draw_sprite(&mut self.plane, 640, &self.numbers, digit, x, iy, true);
        }
    }

    /// The presentation plane under the mission's OWN palette: the
    /// folded GAMEPAL staged with the mission (the host palette no
    /// longer stands in — DESIGN-GAME sec 11 PRESENT, GAMEPAL unit).
    pub(crate) fn plane(&mut self) -> Option<Plane<'_>> {
        let palette = self.palette;
        self.present().map(|pixels| Plane {
            w: 640,
            h: 480,
            pixels,
            palette,
        })
    }

    /// The folded GAMEPAL palette the plane presents under (gate
    /// seam).
    pub fn palette(&self) -> &[Vga6; 256] {
        &self.palette
    }

    /// The staged GAMEGFX\SMLFONT.BIN bytes (`_DAT_004ede7c`) — the
    /// sidebar text bank, staged for the row text slice (the
    /// name/count draws wait on the type table, RE-EXW-SIM 6c.8).
    pub fn sidebar_font_bank(&self) -> &[u8] {
        &self.smlfont
    }

    /// The selected sidebar squad slot (`DAT_0046cbdc`, sec 6c.2).
    pub fn sidebar_selected(&self) -> usize {
        self.sidebar.selected
    }

    /// The overlay draw-mode bit (`_DAT_004edba0`, sec 6c.1/7e.5) —
    /// the strategic map is being presented.
    pub fn map_overlay_on(&self) -> bool {
        self.overlay_on
    }

    /// The map-toggle re-fire lockout (`_DAT_004eb8dc`, 7e.5): the
    /// strip arms 5, each tick decrements while nonzero.
    pub fn map_lockout(&self) -> i32 {
        self.map_lockout
    }

    /// The territory variant byte at a row-major tile (FUN_00408dcc's
    /// 0x4c420c array — presentation-half state, 7e.3).
    pub fn map_territory(&self, tile: usize) -> u8 {
        self.overlay.variant(tile)
    }

    /// The sidebar redraw countdown (`DAT_0046ccec`, sec 6c.5):
    /// producers set 2, each present decrements while nonzero.
    pub fn sidebar_redraw(&self) -> i32 {
        self.sidebar.redraw
    }

    /// The robot's order-bits word (+0x6E, sec 6c.3/6c.6): bit i =
    /// order i active.
    pub fn order_bits(&self, robot: usize) -> u16 {
        self.sidebar.order_bits.get(robot).copied().unwrap_or(0)
    }

    /// The robot's weapon-loadout row (the 0x4de664+type*0x62 groups
    /// the spawn stats-copy read; 7d — host-staged session state).
    pub fn weapon_loadout(&self, robot: usize) -> Option<&WeaponLoadout> {
        self.sidebar.weapons.get(robot)
    }

    /// Stage a robot's WEAPON LOADOUT — the D51 host seam for the
    /// .bss session table at 0x4de664 [RE-EXW-SIM 7d.2]: in the
    /// original the shop FUN_00440e45 / a save-load / the MP lobby
    /// write the row before the mission; here the host stands in for
    /// them. Group word0 = the weapon NAME index (0 = empty group),
    /// word1 = the AMMO count. Runs the exact §6c.6 spawn-copy
    /// arithmetic over the new row — the robot's order-bits word
    /// re-derives as `1 << first group with word0 != 0` (0 when the
    /// row is empty), exactly what load_markers would have written.
    /// The BATTERY PACK group (0x2B) also re-derives the SIM hp via
    /// the dropship-landing formula `5000 + 100*battery`
    /// [RE-EXW-SIM 7f.8] — staging models the pre-mission point, so
    /// re-staging mid-mission re-runs the landing init (D52).
    pub fn set_weapon_loadout(&mut self, robot: usize, groups: &WeaponLoadout) {
        if let Some(slot) = self.sidebar.weapons.get_mut(robot) {
            *slot = *groups;
            self.sidebar.order_bits[robot] = spawn_order_bits(groups);
            let battery = i32::from(
                groups
                    .iter()
                    .find(|&&(name, _)| name == WEAPON_BATTERY_PACK)
                    .map_or(0, |&(_, ammo)| ammo),
            );
            self.sim.set_battery(robot, battery);
        }
    }

    /// Deploy a type's session weapons and separate chassis rows in robot
    /// order (EXW 0x40cf77..0x40d031). Consumables go to the first matching
    /// robot and disappear from the shared rows; scanners remain installed.
    pub(crate) fn deploy_loadout(
        &mut self,
        kind: u16,
        weapons: &WeaponLoadout,
        equipment: &mut [WeaponGroup; 2],
    ) {
        for robot in 0..self.sim.robots().len() {
            if self.sim.robots()[robot].kind != kind {
                continue;
            }
            self.set_weapon_loadout(robot, weapons);
            // EXW 0x40cefd..0x40cf6b copies the session groups into
            // the robot itself; sidebar rows alone cannot drive firing.
            let unit = &mut self.sim.robots_mut()[robot];
            unit.weapons = weapons.map(|(id, ammo)| bedlam_core::weapon::WeaponSlot {
                id,
                ammo: ammo as i16,
                cooldown: 0,
            });
            unit.weapon_mask = spawn_order_bits(weapons);
            for row in equipment.iter_mut() {
                let amount = i32::from(row.1 as i16);
                match row.0 {
                    0x2a => self.sim.robots_mut()[robot].shield_charges = amount,
                    0x2b => self.sim.set_battery(robot, amount),
                    0x2c => self.sim.robots_mut()[robot].armor_pool = amount * 200,
                    0x2d | 0x2e => {
                        if let Some(scanner) = self.sidebar.scanners.get_mut(usize::from(kind)) {
                            *scanner = (row.0 - 0x2c) as u8;
                        }
                    }
                    _ => continue,
                }
                if (0x2a..=0x2c).contains(&row.0) {
                    *row = (0, 0);
                }
            }
        }
    }

    /// Installed scanner level for a chassis type (EXW 0x46ae94 bank).
    pub fn scanner_level(&self, kind: u16) -> Option<u8> {
        self.sidebar.scanners.get(usize::from(kind)).copied()
    }

    /// Apply damage through the SIM — the FUN_0040e230 host seam
    /// [RE-EXW-SIM 7f.5 + 7g, the damage unit]: delegates to
    /// [`MissionSim::apply_damage`] (state-2/alive gates, the
    /// ordered/auto-shield conversions, the alarm accumulator, the
    /// shield absorb vs the hit_flash-then-hp subtract, and the SP
    /// death subset with the five shared-stream debris draws), and
    /// stages the presentation half the original performs inline:
    /// the sidebar redraw countdown `DAT_0046ccec = 3` on death (the
    /// FUN_0042382c call + the death SFX family stay host-seamed).
    pub fn apply_damage(&mut self, robot: usize, damage: i32, killer: i32) -> DamageOutcome {
        let outcome = self.sim.apply_damage(robot, damage, killer);
        if outcome.died {
            self.sidebar.redraw = 3;
            // The five debris stagings [7g.6 + 7j.5]: FUN_00420608
            // kind 5 per debris k — z = robot.z + 8k clamped
            // 0x20..0xFF inside the stager, the 2k start delay.
            // (The six FUN_00422287 scorch-ring writes per debris
            // are NOT staged — the +0x18 armor-pad interaction
            // needs the 7j.8 caveat re-verify first.)
            for (k, &(x, y, z, _)) in outcome.debris.iter().enumerate() {
                self.debris.stage_kind5(x, y, z, 2 * k as i32);
            }
        }
        outcome
    }

    /// The campaign session state the strip reads [7f.9]: score
    /// `_DAT_004dd40c`, money `DAT_0046ae70` (money modeled ≥ 0 —
    /// every producer adds; the SIGNED strip divide is identical on
    /// this domain).
    pub fn campaign(&self) -> (i32, i32) {
        (self.score, self.money)
    }

    /// The RandB-stand-in stream state, read-only (the canonical dump
    /// emitter's rng-state-b row, W6; the fill/churn draws advance it,
    /// this never does).
    pub fn rand_b_state(&self) -> u64 {
        self.rand_b.state()
    }

    /// Stage the campaign session state (D52): the GameMain campaign
    /// init / a save-load stand-in. Default FRESH_CAMPAIGN (0, 4000).
    pub fn set_campaign(&mut self, score: i32, money: i32) {
        self.score = score;
        self.money = money;
    }

    /// The score/money score-strip countdown (`0x46ccf0`, 7f.3):
    /// producers set 2, each present decrements while nonzero.
    pub fn score_strip_countdown(&self) -> i32 {
        self.strip
    }

    /// Active pickup effect rows (7j.1) — gate observability.
    pub fn effect_row_count(&self) -> usize {
        self.effect_rows.rows.iter().filter(|r| r.3 != 0).count()
    }

    /// Active debris records (7j.5) — gate observability.
    pub fn debris_active(&self) -> usize {
        self.debris.recs.iter().filter(|r| r.active).count()
    }

    /// The blink-cursor selector `_DAT_004dc5d0` (7j.6) — gate
    /// observability.
    pub fn sidebar_cursor(&self) -> i32 {
        self.sidebar.cursor
    }

    /// The FUN_0040eba0 case-4 pickup producer [RE-EXW-SIM 7f.6]:
    /// `RandA()&1` picks the row (0 = score, 1 = money), `RandA()&3`
    /// the amount from PICKUP_AWARDS, and the award sets the strip
    /// countdown 2 — exactly the asm (the original draws the two
    /// RandA values from the SHARED sim stream; the player-type gate
    /// `type == [0x4edb90]` is trivially true in SP where every
    /// robot is type 0). The tile-walk producer that fires this case
    /// is not modeled — the host/test seam stands in (never invoked
    /// on the default corpus path, so the sim pins never move).
    pub fn pickup_score_money(&mut self) {
        let row = (self.sim.rand_a() & 1) as usize;
        let amount = PICKUP_AWARDS[row][(self.sim.rand_a() & 3) as usize];
        if row == 0 {
            self.score = self.score.wrapping_add(amount);
        } else {
            self.money = self.money.wrapping_add(amount);
        }
        self.strip = 2;
    }

    /// The FUN_0040eba0 case-1/2/3/7 pickup producer host seam
    /// [RE-EXW-SIM 7h.2]: the tile-word dispatch + the caller's
    /// DAT-consume walk (7h.3) stay host-seamed (the type-DB
    /// mirror is not modeled), so the host calls this when its own
    /// tile watch fires. Delegates to `MissionSim::apply_pickup`
    /// — the case bodies write only sim fields (no sidebar
    /// countdowns, no session state) — and stages the presentation
    /// half [7j.1]: one 16-B effect row `{pos_x>>8, pos_y>>8,
    /// z+0x20, effect id}` through the FUN_00422038 allocator,
    /// exactly the shared case tail (0x40ed5e..0x40f26c). Case 4 is
    /// the separate score/money seam above (session state + the
    /// strip countdown + the two shared-stream draws).
    pub fn pickup(&mut self, robot: usize, case: u8) -> PickupOutcome {
        let outcome = self.sim.apply_pickup(robot, case);
        if outcome.applied {
            if let Some(r) = self.sim.robots().get(robot) {
                self.effect_rows
                    .stage(r.pos_x >> 8, r.pos_y >> 8, r.z + 0x20, outcome.effect);
            }
        }
        outcome
    }
}

/// The HP bar sprite for a staged hp [RE-EXW-SIM 7f.1,
/// FUN_0040807f asm 0x4080a3..0x4080f6]: signed clamp ≤ 5000,
/// `< 1 → 0x46` (empty), else `0x46 - hp*0x2E/5000` (idiv, trunc
/// toward 0 — full 5000 → 0x18).
pub fn hp_bar_sprite(hp: i32) -> u16 {
    let (_, _, denom, scale) = SIDEBAR_HP_BAR;
    let hp = hp.min(denom);
    if hp < 1 {
        SIDEBAR_HP_BAR.0
    } else {
        SIDEBAR_HP_BAR.0 - ((hp * scale) / denom) as u16
    }
}

/// The armor bar sprite for a staged armor word [7f.1, asm
/// 0x408129..0x40818b]: `0 → 0x8E` (the gate), else signed clamp
/// ≤ 2500 then `0x8E - armor*0x2E/2500` capped `≤ 0x8D` — so any
/// nonzero armor below ~55 shows the one-notch 0x8D and 2500+ shows
/// the full 0x60.
pub fn armor_bar_sprite(armor: i16) -> u16 {
    let (empty, _, denom, scale, cap) = SIDEBAR_ARMOR_BAR;
    if armor == 0 {
        return empty;
    }
    let a = i32::from(armor).min(denom);
    let s = empty - ((a * scale) / denom) as u16;
    if s > cap {
        cap
    } else {
        s
    }
}

/// The SMLFONT text draw `FUN_00408913(str, x, y, color)` [verified
/// decompile, RE-EXW-SIM 6c.8c]: chars < 0x21 advance 6 px without
/// drawing (the space), any other char draws glyph `ch - 0x21`
/// filled with `color` (FUN_00402884) and advances `w + 1`. Chars
/// at or above 0x7F remap through the FUN_00410493 codepage family
/// in the EXW — the weapon names and "%04i" counts are pure ASCII,
/// so the remap is unreachable here and the byte is skipped
/// defensively.
fn draw_smlfont_text(plane: &mut [u8], bank: &[u8], text: &str, color: u8, x: i32, y: i32) {
    let mut cx = x;
    for &b in text.as_bytes() {
        let ch = u32::from(b);
        if ch < 0x21 {
            cx += 6;
        } else if ch < 0x7F {
            let id = (ch - 0x21) as u16;
            draw_glyph(plane, 640, bank, id, color, cx, y);
            let w = sprite_geometry(bank, id).map_or(0, |g| g.0);
            cx += w + 1;
        }
    }
}

/// A minimal hermetic mission for host tests: a 4x4 type-1 deck
/// (CGR slot 0 raw 0x1F heights), one MRK record at (1, 1, z-level
/// 1), an empty BIN (no terrain sprites draw), a zero LNK (words
/// stay put), SINTABLE-shaped angle words, an empty DANTE bank, a
/// 770-B synth GAMEPAL, and synth sidebar banks (GENERAL: tiny
/// solid sprites for the portraits 0x12..0x17 + row chrome
/// 0x47/0x49/0x4A/0x4C; SMLFONT: 63 uniform 2x2 glyphs — text
/// draws as solid runs; NUMBERS: 12 strip sprites). Files in
/// [`MissionScene::stage`] parameter order (MRK 5th, GENERAL +
/// SMLFONT after GAMEPAL).
#[cfg(test)]
pub(crate) fn synth_mission_files() -> Vec<Vec<u8>> {
    let w = 4usize;
    let h = 4usize;
    let n = w * h;
    // DAT: header + 8 planes, plane 0 all type 1 (deck).
    let mut dat = vec![0u8; 4 + 8 * n];
    dat[0..2].copy_from_slice(&(w as u16).to_le_bytes());
    dat[2..4].copy_from_slice(&(h as u16).to_le_bytes());
    for b in dat[4..4 + n].iter_mut() {
        *b = 1;
    }
    // PAD: empty.
    let pad = Vec::new();
    // CGR: 1 sprite, dir offset 0 -> body at 2+4*0+0+6... the
    // loader rule is body = dir + 4*s + 8 = 8; 2 pad bytes then
    // 1024 x 0x1F.
    let mut cgr = Vec::new();
    cgr.extend_from_slice(&1u16.to_le_bytes());
    cgr.extend_from_slice(&0u32.to_le_bytes());
    cgr.extend_from_slice(&[0u8; 2]);
    cgr.extend_from_slice(&[0x1Fu8; 1024]);
    // TOT: header + 8 zero u16 planes, EXCEPT plane 0 carries word
    // 1 on every tile (the map-overlay stamp input: LNK[1] = 2, a
    // stable loop — see the MIN mask below).
    let mut tot = vec![0u8; 4 + 16 * n];
    tot[0..2].copy_from_slice(&(w as u16).to_le_bytes());
    tot[2..4].copy_from_slice(&(h as u16).to_le_bytes());
    for i in 0..n {
        tot[4 + 2 * i..4 + 2 * i + 2].copy_from_slice(&1u16.to_le_bytes());
    }
    // BIN/LNK/MRK: empty bank, word-1/word-2 loop link table, one
    // record.
    let bin = Vec::new();
    let mut lnk = vec![0u8; 0x4000];
    lnk.fill(0);
    lnk[2..4].copy_from_slice(&2u16.to_le_bytes()); // LNK[1] = 2
    lnk[4..6].copy_from_slice(&2u16.to_le_bytes()); // LNK[2] = 2 (loop)
    let mut mrk = Vec::new();
    for word in [1u32, 1, 1, 1] {
        mrk.extend_from_slice(&word.to_le_bytes());
    }
    // SINTABLE: 256 words, thresholds ascending over 2..66.
    let mut sintable = Vec::new();
    for i in 0..256u16 {
        let t = if (2..66).contains(&i) {
            0x0647 + (i as u32 - 2) * (0x7FF5 - 0x0647) / 63
        } else {
            0
        } as u16;
        sintable.extend_from_slice(&t.to_le_bytes());
    }
    // DANTE: empty bank (entity flushes draw nothing).
    let dante = Vec::new();
    // GAMEPAL: 770-B synth palette — entry i carries the 6-bit
    // components (i*3, i*3+1, i*3+2) & 0x3F so the fold is visible.
    let mut gamepal = vec![0u8; 2];
    for i in 0..256usize {
        for c in 0..3usize {
            gamepal.push(((i * 3 + c) & 0x3F) as u8);
        }
    }
    assert_eq!(gamepal.len(), 770);
    // GENERAL: a synth UI bank — tiny 2x2 solid sprites for the
    // portraits (0x12..0x17), the row chrome (0x47/0x49 body +
    // 0x4A/0x4C well), the map markers (0x55/0x56), the map button
    // chrome (0x5E), and the HP/armor bar sprites the default
    // vitals reach (0x18 full HP, 0x2F half HP, 0x46 empty HP,
    // 0x60 full armor, 0x77 half armor, 0x8D one-notch armor,
    // 0x8E empty armor — distinct pixel values per id so tests can
    // tell them apart); everything else empty.
    let general_count = 0x8Fu16;
    let mut general = vec![0u8; 2 + 4 * general_count as usize];
    general[0..2].copy_from_slice(&general_count.to_le_bytes());
    let put = |bank: &mut Vec<u8>, id: u16, color: u8| {
        let entry = 2 + 4 * id as usize;
        let start = bank.len();
        bank.extend_from_slice(&3u16.to_le_bytes()); // flags: hotspot + RLE
        bank.extend_from_slice(&[0, 0, 0, 0]); // yhot, xhot
        bank.extend_from_slice(&2u16.to_le_bytes()); // w
        bank.extend_from_slice(&2u16.to_le_bytes()); // h
                                                     // RLE: two rows of literal-2 solid color.
        bank.extend_from_slice(&[0x02, 0x00, color, color, 0x00, 0xC0]);
        bank.extend_from_slice(&[0x02, 0x00, color, color, 0x00, 0xC0]);
        let off = (start as u32) - entry as u32;
        bank[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
    };
    for id in 0x12u16..0x18 {
        put(&mut general, id, 0x20 + id as u8);
    }
    for (id, color) in [
        (0x18u16, 0x90u8),
        (0x2F, 0x91),
        (0x46, 0x92),
        (0x60, 0x93),
        (0x77, 0x94),
        (0x8D, 0x95),
        (0x8E, 0x96),
        (0x47u16, 0xA7u8),
        (0x49, 0xB9),
        (0x4A, 0xCA),
        (0x4C, 0xDC),
        (0x55, 0xE1),
        (0x56, 0xE3),
        (0x5E, 0xE5),
    ] {
        put(&mut general, id, color);
    }
    // SMLFONT: 63 glyphs, every one a 2x2 solid mask in the shipped
    // one-word-per-row form (`0x4002|mask,mask`) so the row TEXT
    // draws land (each glyph advances w+1 = 3 px; chars < 0x21
    // advance 6).
    let mut smlfont = vec![0u8; 2 + 4 * 63];
    smlfont[0..2].copy_from_slice(&63u16.to_le_bytes());
    for id in 0..63u16 {
        let entry = 2 + 4 * id as usize;
        let start = smlfont.len();
        smlfont.extend_from_slice(&3u16.to_le_bytes()); // flags: hotspot + RLE
        smlfont.extend_from_slice(&[0, 0, 0, 0]); // yhot, xhot
        smlfont.extend_from_slice(&2u16.to_le_bytes()); // w
        smlfont.extend_from_slice(&2u16.to_le_bytes()); // h
        smlfont.extend_from_slice(&[0x02, 0x40, 0x4D, 0x4D]); // literal 2 + EOL
        smlfont.extend_from_slice(&[0x02, 0x40, 0x4D, 0x4D]); // literal 2 + EOL
        let off = (start as u32) - entry as u32;
        smlfont[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
    }
    // TABLE: the strategic-map backdrop bank — one 2x2 raw sprite at
    // id 0 (pixel 0x77) so the overlay's backdrop visibly lands.
    let mut table = Vec::new();
    table.extend_from_slice(&1u16.to_le_bytes()); // count
    table.extend_from_slice(&4u32.to_le_bytes()); // entry 0: record at 2+4
    table.extend_from_slice(&0u16.to_le_bytes()); // flags: raw
    table.extend_from_slice(&2u16.to_le_bytes()); // w
    table.extend_from_slice(&2u16.to_le_bytes()); // h
    table.extend_from_slice(&[0x77u8, 0x77, 0x77, 0x77]);
    // MIN: three 4x4 masks; mask 2 = the diagonal wedge the synth
    // TOT stamps (word 1 -> LNK[1] = 2, LNK[2] = 2 — a stable loop).
    let mut min = vec![0u8; 3 * 16];
    for r in 0..4usize {
        for c in 0..4usize {
            min[2 * 16 + r * 4 + c] = if r == c || r + c == 3 { 5 } else { 0 };
        }
    }
    // MAPTRAN: eight ramps; ramp v maps entry e -> 0x60 + 16*v + e
    // (distinct per ring so territory tests can tell them apart).
    let mut maptran = Vec::new();
    for v in 0..8u32 {
        let mut ramp = vec![0u8; 256];
        for (e, b) in ramp.iter_mut().enumerate() {
            *b = (0x60 + 16 * v + e as u32) as u8;
        }
        maptran.push(ramp);
    }
    // NUMBERS: the score-strip bank — 12 sprites (digits 0..9,
    // 0xA score icon, 0xB money icon), tiny 2x2 solids with
    // distinct per-id colors so strip tests can tell digits apart.
    let numbers_count = 0xCu16;
    let mut numbers = vec![0u8; 2 + 4 * numbers_count as usize];
    numbers[0..2].copy_from_slice(&numbers_count.to_le_bytes());
    for id in 0..numbers_count {
        let entry = 2 + 4 * id as usize;
        let start = numbers.len();
        numbers.extend_from_slice(&3u16.to_le_bytes()); // flags: hotspot + RLE
        numbers.extend_from_slice(&[0, 0, 0, 0]); // yhot, xhot
        numbers.extend_from_slice(&2u16.to_le_bytes()); // w
        numbers.extend_from_slice(&2u16.to_le_bytes()); // h
        let color = 0xF0u16 + id;
        numbers.extend_from_slice(&[0x02, 0x00, color as u8, color as u8, 0x00, 0xC0]);
        numbers.extend_from_slice(&[0x02, 0x00, color as u8, color as u8, 0x00, 0xC0]);
        let off = (start as u32) - entry as u32;
        numbers[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
    }
    // FLAGS: the effect-row icon bank (DANTE entity layout —
    // FUN_0040179b resolve): 0xE tiny 2-row sprites with distinct
    // per-id colors so effect-row tests can tell ids apart.
    let flags_count = 0xEu16;
    let mut flags = vec![0u8; 2 + 4 * flags_count as usize];
    flags[0..2].copy_from_slice(&flags_count.to_le_bytes());
    for id in 0..flags_count {
        let entry = 2 + 4 * id as usize;
        let start = flags.len();
        // {fmt, dy, dx, gate, rows} — the fmt word is skipped by
        // the flush resolve.
        flags.extend_from_slice(&3u16.to_le_bytes());
        flags.extend_from_slice(&0u16.to_le_bytes()); // dy
        flags.extend_from_slice(&0u16.to_le_bytes()); // dx
        flags.extend_from_slice(&64u16.to_le_bytes()); // gate
        flags.extend_from_slice(&2u16.to_le_bytes()); // rows
        let color = 0x40u8 + id as u8 * 2;
        flags.extend_from_slice(&[0x02, 0x00, color, color, 0x00, 0xC0]);
        flags.extend_from_slice(&[0x02, 0x00, color, color, 0x00, 0xC0]);
        let off = (start as u32) - entry as u32;
        flags[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
    }
    // BLOWUP: the debris bank (same layout) — sprites 5..0x10 (the
    // kind-5 sequence walk) with distinct colors.
    let blowup_count = 0x11u16;
    let mut blowup = vec![0u8; 2 + 4 * blowup_count as usize];
    blowup[0..2].copy_from_slice(&blowup_count.to_le_bytes());
    for id in 0..blowup_count {
        let entry = 2 + 4 * id as usize;
        let start = blowup.len();
        blowup.extend_from_slice(&3u16.to_le_bytes());
        blowup.extend_from_slice(&0u16.to_le_bytes()); // dy
        blowup.extend_from_slice(&0u16.to_le_bytes()); // dx
        blowup.extend_from_slice(&64u16.to_le_bytes()); // gate
        blowup.extend_from_slice(&2u16.to_le_bytes()); // rows
        let color = 0x80u8 + id as u8;
        blowup.extend_from_slice(&[0x02, 0x00, color, color, 0x00, 0xC0]);
        blowup.extend_from_slice(&[0x02, 0x00, color, color, 0x00, 0xC0]);
        let off = (start as u32) - entry as u32;
        blowup[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
    }
    let mut files = vec![
        tot, dat, pad, cgr, mrk, bin, lnk, sintable, dante, gamepal, general, smlfont, table, min,
    ];
    files.extend(maptran); // f[14..22] — the eight ramps in slot order
    files.push(numbers); // f[22] — the score-strip bank
    files.push(flags); // f[23] — the effect-row icon bank
    files.push(blowup); // f[24] — the debris bank
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use bedlam_core::mission::pickup_case;

    fn staged(markers: &[(i32, i32, i32)]) -> MissionScene {
        let f = synth_mission_files();
        let maptran: Vec<&[u8]> = f[14..22].iter().map(|v| v.as_slice()).collect();
        MissionScene::stage(
            &f[0], &f[1], &f[2], &f[3], &f[4], &f[5], &f[6], &f[7], &f[8], &f[9], &f[10], &f[11],
            &f[22], &f[23], &f[24], &f[12], &f[13], &maptran, 0, None, markers,
        )
        .expect("synth mission stages")
    }

    #[test]
    fn robots_per_player_table() {
        assert_eq!(robots_per_player(0), 1);
        assert_eq!(robots_per_player(1), 1);
        assert_eq!(robots_per_player(2), 1);
        assert_eq!(robots_per_player(3), 2);
        assert_eq!(robots_per_player(4), 3);
        assert_eq!(robots_per_player(7), 1);
    }

    #[test]
    fn mission_names_follow_the_zone_arithmetic() {
        assert_eq!(
            mission_asset_names(0, 1),
            vec![
                "ZONEA/MISSION1.TOT",
                "ZONEA/MISSION1.DAT",
                "ZONEA/MISSION1.PAD",
                "ZONEA/MISSIONA.CGR",
                "ZONEA/MISSIONA.BIN",
                "ZONEA/MISSIONA.LNK",
                "SINTABLE.BIN",
                "DANTE.BIN",
                "GAMEPAL.PAL",
                "GENERAL.BIN",
                "SMLFONT.BIN",
                "ZONEA/MISSION1.MRK",
                "TABLE.BIN",
                "MAPTRAN0.TRN",
                "MAPTRAN1.TRN",
                "MAPTRAN2.TRN",
                "MAPTRAN3.TRN",
                "MAPTRAN4.TRN",
                "MAPTRAN5.TRN",
                "MAPTRAN6.TRN",
                "MAPTRAN7.TRN",
                "ZONEA/MISSIONA.MIN",
                "NUMBERS.BIN",
                "FLAGS.BIN",
                "BLOWUP.BIN",
            ]
        );
        assert_eq!(
            mission_asset_names(2, 5)[3..6],
            [
                "ZONEC/MISSIONC.CGR",
                "ZONEC/MISSIONC.BIN",
                "ZONEC/MISSIONC.LNK"
            ]
        );
        assert_eq!(
            mission_asset_names(2, 5)[21],
            "ZONEC/MISSIONC.MIN",
            "the .MIN mask bank is zone-level (like CGR/BIN/LNK)"
        );
        assert_eq!(zone_for_stage(1), 0);
        assert_eq!(zone_for_stage(7), 6);
        assert_eq!(zone_for_stage(8), 6, "endgame stays at zone G");
        assert_eq!(mission_number_for_mask(0), 1);
        assert_eq!(mission_number_for_mask(0b0111), 4);
        // The SP SELECT domain saturates at 5 (§7j.73: the SP arm
        // writes missions 1..5 per zone only — the campaign path
        // can never name an MP file; 6/7 come only from the staged
        // SELECT pair).
        assert_eq!(mission_number_for_mask(0b1111), 5);
        assert_eq!(mission_number_for_mask(0b11111), 5);
        for mask in 0..=u8::MAX {
            if mask & !0b1_1111 == 0 {
                assert!(
                    mission_number_for_mask(mask) <= 5,
                    "the five-bit save domain names SP files only ({mask})"
                );
            }
        }
        assert_eq!(SELECT_MP_FILE_OFFSET, 5, "0x4467df add eax,0x5");
    }

    #[test]
    fn stage_spawns_mrk_robots_and_fixes_camera_on_activate() {
        let mut m = staged(&[(3, 1, 1)]);
        assert!(!m.is_active(), "staged inert");
        assert_eq!(m.sim().robots().len(), 2, "MRK[0] + staged marker");
        // Inert tick + present: nothing happens.
        m.tick(&InputFrame::default());
        assert!(m.present().is_none());
        m.activate();
        assert!(m.is_active());
        // Camera at robot 0 Q5: tile (1,1) + 0xF00 center -> Q5
        // (1*32+15, 1*32+15) = (47, 47).
        assert_eq!(m.camera(), (16, 16));
    }

    #[test]
    fn bad_bytes_error_without_panic() {
        let f = synth_mission_files();
        // (dat, mrk, sintable, gamepal) override slices, else the
        // synth files.
        let try_stage = |dat: &[u8], mrk: &[u8], sintable: &[u8], gamepal: &[u8]| {
            let maptran: Vec<&[u8]> = f[14..22].iter().map(|v| v.as_slice()).collect();
            MissionScene::stage(
                &f[0],
                dat,
                &f[2],
                &f[3],
                mrk,
                &f[5],
                &f[6],
                sintable,
                &f[8],
                gamepal,
                &f[10],
                &f[11],
                &f[22],
                &f[23],
                &f[24],
                &f[12],
                &f[13],
                &maptran,
                0,
                None,
                &[],
            )
        };
        assert!(matches!(
            try_stage(&f[1][..10], &f[4], &f[7], &f[9]),
            Err(GameError::BadMissionAsset {
                what: "DAT/PAD/CGR",
                ..
            })
        ));
        assert!(matches!(
            try_stage(&f[1], &f[4][..8], &f[7], &f[9]),
            Err(GameError::BadMissionAsset { what: "MRK", .. })
        ));
        assert!(matches!(
            try_stage(&f[1], &f[4], &f[7][..100], &f[9]),
            Err(GameError::BadMissionAsset {
                what: "SINTABLE",
                ..
            })
        ));
        assert!(matches!(
            try_stage(&f[1], &f[4], &f[7], &f[9][..100]),
            Err(GameError::BadMissionAsset {
                what: "GAMEPAL",
                ..
            })
        ));
    }

    #[test]
    fn deployment_consumes_chassis_once_in_robot_order() {
        let mut m = staged(&[(3, 1, 1), (3, 2, 2), (3, 3, 3)]);
        m.sim.robots_mut()[0].kind = 1;
        m.sim.robots_mut()[1].kind = 0;
        m.sim.robots_mut()[2].kind = 0;
        let mut weapons = [(0, 0); 7];
        weapons[0] = (2, 300);
        let mut equipment = [(0x2b, 5), (0x2a, 15)];
        m.deploy_loadout(0, &weapons, &mut equipment);
        assert_eq!(equipment, [(0, 0); 2]);
        assert_eq!(m.sim.robots()[0].battery, 0);
        assert_eq!(m.weapon_loadout(0), Some(&[(0, 0); 7]));
        for robot in [1, 2] {
            assert_eq!(m.weapon_loadout(robot), Some(&weapons));
            assert_eq!(m.sim.robots()[robot].weapons[0].id, 2);
            assert_eq!(m.sim.robots()[robot].weapons[0].ammo, 300);
            assert_eq!(m.sim.robots()[robot].weapon_mask, 1);
        }
        assert_eq!(m.sim.robots()[1].battery, 5);
        assert_eq!(m.sim.robots()[1].hp, 5500);
        assert_eq!(m.sim.robots()[1].shield_charges, 15);
        assert_eq!(m.sim.robots()[2].battery, 0);
        assert_eq!(m.sim.robots()[2].shield_charges, 0);
    }

    #[test]
    fn deployment_retains_scanner_and_sign_extends_damper_quantity() {
        let mut m = staged(&[(3, 1, 1)]);
        m.sim.robots_mut()[0].kind = 0;
        let mut equipment = [(0x2c, 0xffff), (0x2e, 1)];
        m.deploy_loadout(0, &[(0, 0); 7], &mut equipment);
        assert_eq!(m.sim.robots()[0].armor_pool, -200);
        assert_eq!(equipment, [(0, 0), (0x2e, 1)]);
        assert_eq!(m.scanner_level(0), Some(2));
        let mut other_type = [(0x2b, 5), (0x2d, 1)];
        m.deploy_loadout(1, &[(0, 0); 7], &mut other_type);
        assert_eq!(other_type, [(0x2b, 5), (0x2d, 1)]);
        assert_eq!(m.scanner_level(1), Some(0));
    }

    #[test]
    fn camera_tracks_four_rendered_anchors_and_subtracts_averaged_height() {
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        assert_eq!(m.camera(), (16, 16));
        m.sim.robots_mut()[0].pos_x = 79 << 8;
        m.sim.robots_mut()[0].pos_y = 111 << 8;
        m.sim.robots_mut()[0].z = 39;
        let sim_hash = m.sim.state_hash();
        for expected in [(22, 30), (28, 44), (34, 58), (40, 72)] {
            m.present().unwrap();
            assert_eq!(m.camera(), expected);
            assert_eq!(
                m.sim.state_hash(),
                sim_hash,
                "camera rendering never moves robots"
            );
        }
        assert_eq!(m.cam_height, 39);
    }

    #[test]
    fn ground_click_assigns_movement_without_spread_order_or_teleport() {
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        let before = m.sim.robots()[0].q5();
        let (cx, cy) = m.cursor();
        m.tick(&InputFrame {
            mouse_dx: (240 - cx) as i16,
            mouse_dy: (280 - cy) as i16,
            mouse_buttons: 1,
            ..Default::default()
        });
        // Height-adjusted camera (16,16), vy=40+31-8=63 -> (79,79).
        assert_eq!(m.sim.robots()[0].target, Some((79, 79)));
        assert!(m.sim.order().is_none());
        assert!(!m.sim.command_order_active(), "left movement never fires");
        assert_ne!(m.sim.robots()[0].q5(), (32, 32), "no old tile-origin snap");
        for _ in 0..40 {
            m.tick(&InputFrame::default());
        }
        assert_ne!(m.sim.robots()[0].q5(), before, "robot walks to ground");
    }

    #[test]
    fn present_blits_the_window_at_origin_once_per_call() {
        let mut m = staged(&[]);
        assert!(m.present().is_none(), "inert before activate");
        m.activate();
        let plane = m.present().expect("active presents");
        assert_eq!(plane.len(), 640 * 480);
        // The synth TOT has no words and the BIN is empty: the
        // VIEWPORT stays all zero but the LNK walk counted one
        // render...
        assert!(plane[..640 * 480]
            .chunks_exact(640)
            .all(|r| r[..480].iter().all(|&b| b == 0)));
        assert_eq!(m.render_count(), 1);
        m.present();
        assert_eq!(m.render_count(), 2);
    }

    #[test]
    fn plane_carries_the_folded_gamepal() {
        // The mission plane presents under its OWN palette: the
        // folded GAMEPAL entry i = ((i*3+c) & 0x3F) for the synth
        // file (the fold keeps the 6-bit file values exactly).
        let mut m = staged(&[]);
        m.activate();
        let mut want = [[0u8; 3]; 256];
        for (i, entry) in want.iter_mut().enumerate() {
            *entry = [
                ((i * 3) & 0x3F) as u8,
                ((i * 3 + 1) & 0x3F) as u8,
                ((i * 3 + 2) & 0x3F) as u8,
            ];
        }
        assert_eq!(m.palette(), &want);
        let plane = m.plane().expect("active plane");
        assert_eq!(plane.palette, want, "the plane palette IS GAMEPAL");
    }

    /// Sidebar click helper: aim + click, mirroring the EXW
    /// mouse_l_click -> sidebar_control dispatch (sec 6c).
    fn sidebar_click(m: &mut MissionScene, x: i32, y: i32) {
        let (cx, cy) = m.cursor();
        m.tick(&InputFrame {
            mouse_dx: (x - cx) as i16,
            mouse_dy: (y - cy) as i16,
            mouse_buttons: 0,
            ..InputFrame::default()
        });
        m.tick(&InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        });
    }

    #[test]
    fn sidebar_select_strips_follow_the_asm_gates() {
        // Two-robot squad (MRK[0] + one staged marker): strips 0/1
        // select, strip 2 is gated off (DAT_0046cbd8 analog < 3),
        // out-of-strip clicks keep state [sec 6c.2, asm
        // 0x40d220..0x40d3b0].
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        assert_eq!(m.sidebar_selected(), 0, "slot 0 selected at spawn");
        assert_eq!(m.order_bits(0), 0, "fresh-campaign loadout: no bit [7d.4]");
        assert_eq!(
            m.sidebar_redraw(),
            2,
            "activate arms the initial draw (MissionShell 0x447c74)"
        );
        // Strip 1 (x 0x219..0x249): selects slot 1, redraw = 2.
        sidebar_click(&mut m, 0x219, 5);
        assert_eq!(m.sidebar_selected(), 1);
        assert_eq!(m.sidebar_redraw(), 2);
        // Strip 2 with a 2-robot squad: gated off, nothing changes.
        m.sidebar.redraw = 0;
        sidebar_click(&mut m, 0x24B, 0x35);
        assert_eq!(m.sidebar_selected(), 1, "slot 2 gated (squad < 3)");
        assert_eq!(m.sidebar_redraw(), 0, "no fire -> no redraw");
        // Strip bounds: x = 0x218 is between strips 0/1 -> no-op;
        // y = 0x36 is one past the strip bottom -> no-op.
        sidebar_click(&mut m, 0x218, 5);
        sidebar_click(&mut m, 0x1E7, 0x36);
        assert_eq!(m.sidebar_selected(), 1);
        assert_eq!(m.sidebar_redraw(), 0);
        // Strip 0 bottom-left corner (0x1E7, 5) is INSIDE [asm
        // inclusive]: fires, back to slot 0.
        sidebar_click(&mut m, 0x1E7, 5);
        assert_eq!(m.sidebar_selected(), 0);
        assert_eq!(m.sidebar_redraw(), 2);
    }

    #[test]
    fn sidebar_order_rows_toggle_the_selected_robot() {
        // Row click on the SELECTED robot's bits word: row = (y -
        // 0x57)/14 clamp 6, gate = the group AMMO word (record
        // +0x38+8k, sec 6c.3), toggle + redraw = 2 [sec 6c.4, asm
        // 0x40d659..0x40d712]. Stage a full 7-group loadout on both
        // robots (the D51 seam; the fresh default is empty).
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        let full = |ammo0: u16| {
            let mut g = [(0u16, 0u16); 7];
            for (i, slot) in g.iter_mut().enumerate() {
                *slot = ((2 + i as u16) * 3, if i == 0 { ammo0 } else { 5 });
            }
            g
        };
        m.set_weapon_loadout(0, &full(4));
        m.set_weapon_loadout(1, &full(1));
        assert_eq!(m.order_bits(0), 1, "spawn armer = 1<<first group");
        assert_eq!(m.order_bits(1), 1);
        // Select slot 1, then click row 0 of the order rect.
        sidebar_click(&mut m, 0x219, 5);
        sidebar_click(&mut m, 0x200, 0x57);
        assert_eq!(m.order_bits(1), 0, "bit 0 toggled off");
        assert_eq!(m.order_bits(0), 1, "robot 0 untouched");
        assert_eq!(m.sidebar_redraw(), 2);
        // Row boundaries: y 0x57..0x64 = row 0, 0x65 = row 1;
        // y 0xB8 = row 6 (in), 0xB9 = out; x 0x1E9 in / 0x1E8 out /
        // 0x275 in / 0x276 out.
        sidebar_click(&mut m, 0x200, 0x64);
        assert_eq!(m.order_bits(1), 1, "y=0x64 still row 0");
        sidebar_click(&mut m, 0x200, 0x65);
        assert_eq!(m.order_bits(1) >> 1 & 1, 1, "y=0x65 is row 1");
        sidebar_click(&mut m, 0x275, 0xB8);
        assert_eq!(m.order_bits(1) >> 6 & 1, 1, "y=0xB8 is row 6");
        m.sidebar.redraw = 0;
        sidebar_click(&mut m, 0x275, 0xB9);
        sidebar_click(&mut m, 0x1E8, 0x57);
        sidebar_click(&mut m, 0x276, 0x57);
        assert_eq!(m.order_bits(1), 0b1000011, "out-of-rect clicks no-op");
        assert_eq!(m.sidebar_redraw(), 0);
        // Ammo gate: group 3 with a NAME but AMMO 0 — the sec 6c.3
        // gate word is the count, so its click neither toggles nor
        // redraws (a name-only group still DRAWS).
        let mut g = full(1);
        g[3].1 = 0;
        m.set_weapon_loadout(1, &g);
        sidebar_click(&mut m, 0x200, 0x57 + 3 * 14);
        assert_eq!(
            m.order_bits(1),
            1,
            "gated row untouched (bits re-derived 1<<first)"
        );
        assert_eq!(m.sidebar_redraw(), 0, "gate fail -> no redraw");
        // The empty default: no gate ever passes.
        m.set_weapon_loadout(1, &[(0, 0); 7]);
        for row in 0..7 {
            sidebar_click(&mut m, 0x200, 0x57 + 14 * row);
        }
        assert_eq!(m.order_bits(1), 0, "empty loadout: every row gated");
        assert_eq!(m.sidebar_redraw(), 0);
    }

    #[test]
    fn sidebar_redraw_counts_down_per_present() {
        // DAT_0046ccec: producers set 2, the draw tail decrements
        // once per frame while nonzero [sec 6c.5, asm 0x407205];
        // activate arms the INITIAL draw with 2 (0x447c74, 6c.8e).
        let mut m = staged(&[]);
        m.activate();
        assert_eq!(m.sidebar_redraw(), 2, "the entry trigger");
        m.present().expect("active presents");
        assert_eq!(m.sidebar_redraw(), 1);
        m.present();
        assert_eq!(m.sidebar_redraw(), 0);
        m.present();
        assert_eq!(m.sidebar_redraw(), 0, "sticks at zero");
    }

    #[test]
    fn sidebar_art_draws_rows_portraits_and_text() {
        // The FUN_00408403 row pass (chrome + NAME + COUNT text) +
        // the FUN_004072bf portraits [sec 6c.8]: synth GENERAL
        // sprites carry distinct colors (0x12+k -> 0x32+k portraits,
        // 0x47/0x4A armed, 0x49/0x4C unarmed), synth SMLFONT glyphs
        // are 2x2 solid masks, so the plane pins which sprite, glyph
        // run and count landed where. Rows need a STAGED loadout
        // (D51: the fresh-campaign default is empty).
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        let mut g = [(0u16, 0u16); 7];
        g[0] = (2, 30); // NEEDLER CANNON #1, 30 rounds
        g[1] = (9, 3); // HADES BOMB #1
        g[2] = (5, 7); // unlisted index -> "ERROR" text still draws
        m.set_weapon_loadout(0, &g);
        m.set_weapon_loadout(1, &g);
        let plane = m.present().expect("the entry frame draws (countdown 2)");
        let px = |p: &[u8], x: usize, y: usize| p[y * 640 + x];
        // Portraits: 2-robot squad -> slots 0 (selected, 0x12 ->
        // color 0x32) and 1 (not selected, 0x16 -> 0x36) at
        // (0x1E7,5) and (0x219,5); slot 2 is BEYOND the squad, so
        // the dither draws full static over its box every frame
        // (mode 0, D55 — the unoccupied boxes are pure noise).
        assert_eq!(px(plane, 0x1E7, 5), 0x32);
        assert_eq!(px(plane, 0x219, 5), 0x36);
        assert!(
            (0..48).all(|dy| (0..48).all(|dx| {
                let b = px(plane, 0x24B + dx as usize, 5 + dy as usize);
                b == 0 || b == DITHER_WHITE
            })),
            "slot 2 beyond the squad: static replaces the box (mode 0)"
        );
        // Rows (robot 0 selected, groups 0..2 present, bits = 1):
        // row 0 ARMED -> body 0x47 (0xA7) at (0x1EB,0x59) + well 0x4A
        // (0xCA) at (0x25A,0x59); rows 1/2 unarmed -> 0xB9/0xDC;
        // rows 3..6 carry no weapon -> nothing.
        assert_eq!(px(plane, 0x1EB, 0x59), 0xA7, "row 0 armed body");
        assert_eq!(px(plane, 0x25A, 0x59), 0xCA, "row 0 armed well");
        for i in 1..3 {
            let y = 0x59 + 14 * i;
            assert_eq!(px(plane, 0x1EB, y as usize), 0xB9, "row {i} unarmed body");
            assert_eq!(px(plane, 0x25A, y as usize), 0xDC, "row {i} unarmed well");
        }
        // The NAME text starts at (0x1ED, 0x5B): the synth glyphs
        // paint color 0x24 — glyph 0 ('N') at x 0x1ED, then advance
        // w+1 = 3 per char (space = 6).
        assert_eq!(px(plane, 0x1ED, 0x5B), 0x24, "name glyph 0 paints 0x24");
        assert_eq!(px(plane, 0x1ED + 3, 0x5B), 0x24, "advance w+1 = 3");
        assert_eq!(px(plane, 0x1ED + 2, 0x5B), 0, "inside-advance gap");
        // "NEEDLER CANNON #1": N E E D L E R <sp> C A ... — the
        // space at index 7 advances 6, so glyph 8 ('C') lands at
        // 7*3 + 6 = 27 px in.
        assert_eq!(px(plane, 0x1ED + 27, 0x5B), 0x24, "space advances 6");
        // The COUNT text at (0x25C, 0x5B): 30 -> "0030".
        assert_eq!(px(plane, 0x25C, 0x5B), 0x24, "count glyph 0 ('0')");
        assert_eq!(px(plane, 0x25C + 3, 0x5B), 0x24, "count digit 2");
        assert_eq!(
            px(plane, 0x25C + 9, 0x5B),
            0x24,
            "count digit 4 ('0' of 30)"
        );
        // Group row 1's count "0003".
        assert_eq!(px(plane, 0x25C, 0x5B + 14), 0x24);
        // Empty row 3: no body, no well, no text.
        assert_eq!(
            px(plane, 0x1EB, 0x59 + 14 * 3),
            0,
            "row 3 empty (no weapon)"
        );
        assert_eq!(px(plane, 0x1ED, 0x5B + 14 * 3), 0, "row 3 no text");
        // Selecting slot 1 moves the rows to robot 1 and the armed
        // portrait to slot 1 (both staged identically).
        sidebar_click(&mut m, 0x219, 5);
        m.present();
        assert_eq!(px(&m.plane, 0x1E7, 5), 0x35, "slot 0 now unselected (0x15)");
        assert_eq!(px(&m.plane, 0x219, 5), 0x33, "slot 1 selected (0x13)");
        assert_eq!(px(&m.plane, 0x1EB, 0x59), 0xA7, "robot 1 row 0 armed body");
        // The empty default draws NOTHING in the rows band: wipe and
        // re-trigger with an empty loadout.
        m.set_weapon_loadout(1, &[(0, 0); 7]);
        m.sidebar.redraw = 2;
        for y in 0x59..0x59 + 14 * 7 {
            for x in 0x1EB..0x276 {
                m.plane[y as usize * 640 + x] = 0;
            }
        }
        m.present();
        assert!(
            (0x59..0x59 + 14 * 7)
                .all(|y| (0x1EB..0x276).all(|x| m.plane[y as usize * 640 + x] == 0)),
            "empty loadout: no rows, no text"
        );
        // The staged font bank rides along (63 synth glyphs).
        assert_eq!(
            u16::from_le_bytes([m.sidebar_font_bank()[0], m.sidebar_font_bank()[1]]),
            63
        );
    }

    #[test]
    fn bar_sprites_map_the_exw_arithmetic() {
        // FUN_0040807f [7f.1, asm 0x4080a3..0x40818b]: clamp, empty
        // gates, the idiv step, the armor cap.
        assert_eq!(hp_bar_sprite(5000), 0x18, "full");
        assert_eq!(hp_bar_sprite(6000), 0x18, "clamp <= 5000");
        assert_eq!(hp_bar_sprite(2500), 0x46 - 23, "2500*46/5000 = 23");
        assert_eq!(hp_bar_sprite(108), 0x46, "108*46/5000 = 0 (idiv trunc)");
        assert_eq!(hp_bar_sprite(1), 0x46);
        assert_eq!(hp_bar_sprite(0), 0x46, "hp < 1 -> empty");
        assert_eq!(hp_bar_sprite(-5), 0x46, "signed");
        assert_eq!(armor_bar_sprite(0), 0x8E, "gate word == 0");
        assert_eq!(armor_bar_sprite(1), 0x8D, "0x8E-0 capped 0x8D");
        assert_eq!(armor_bar_sprite(54), 0x8D, "54*46/2500 = 0 -> capped");
        assert_eq!(armor_bar_sprite(55), 0x8D, "55*46/2500 = 1");
        assert_eq!(armor_bar_sprite(1250), 0x8E - 23, "1250*46/2500 = 23");
        assert_eq!(armor_bar_sprite(2500), 0x60, "full");
        assert_eq!(armor_bar_sprite(3000), 0x60, "clamp <= 2500");
    }

    #[test]
    fn dither_family_draws_the_7i_semantics() {
        // [RE-EXW-SIM 7i, D55] The noise ring + the portrait-pass
        // blit: boot fill 25% white binary noise, the mode-0 FULL
        // static over dead + beyond-squad boxes, the mode-1 masked
        // overlay on hit_flash (the portrait survives under zero
        // bytes), the pass never decaying the flash itself (7g.8 is
        // the sim tick), and the 15-byte/frame epilogue churn.
        let mut m = staged(&[]);
        m.activate();
        // The bank is binary {0, 0xFF} at ~25% white (2048 draws,
        // deterministic under the Pcg32 stand-in).
        let whites = m.dither.bank.iter().filter(|&&b| b == DITHER_WHITE).count();
        assert!(
            (350..=700).contains(&whites),
            "the boot fill is ~25% white ({whites})"
        );
        // 1-robot squad (zone 0): the entry frame draws the slot-0
        // portrait; slots 1/2 are BEYOND the squad -> full static
        // (the EXW dithers the unoccupied boxes every frame, asm
        // 0x4073d8/0x4073fc). Pre-paint slot 1's box with a sentinel
        // to prove the mode-0 blit REPLACES the content (the synth
        // portraits are 2x2 dots, so a sentinel stands in for
        // pixels).
        for dy in 0..48usize {
            for dx in 0..48usize {
                m.plane[(5 + dy) * 640 + 0x219 + dx] = 0x77;
            }
        }
        let plane = m.present().expect("entry frame");
        let px = |p: &[u8], x: usize, y: usize| p[y * 640 + x];
        assert_eq!(
            px(plane, 0x1E7, 5),
            0x32,
            "the portrait draws (alive, hp 5000)"
        );
        assert!(
            (0..48).all(|dy| (0..48).all(|dx| {
                let b = px(plane, 0x219 + dx as usize, 5 + dy as usize);
                b == 0 || b == DITHER_WHITE
            })),
            "beyond-squad boxes carry full static — mode 0 REPLACES the box"
        );
        let whites = (0..48 * 48usize)
            .filter(|&i| px(plane, 0x219 + i % 48, 5 + i / 48) == DITHER_WHITE)
            .count();
        assert!(whites > 100, "the static is ~25% white ({whites})");
        // hit_flash != 0 -> the mode-1 blit over the LIVE portrait:
        // sentinel-paint the box, flash, present — the box ends up
        // ONLY {sentinel, 0xFF}: zeros never overwrite (the portrait
        // survives under zero bytes) and the white specks overlay.
        for dy in 0..48usize {
            for dx in 0..48usize {
                m.plane[(5 + dy) * 640 + 0x1E7 + dx] = 0x77;
            }
        }
        m.sim.robots_mut()[0].hit_flash = 3;
        let plane = m.present().expect("hit-flash frame");
        let mut sentinel_px = 0;
        let mut white_px = 0;
        for i in 0..48 * 48usize {
            match px(plane, 0x1E7 + i % 48, 5 + i / 48) {
                DITHER_WHITE => white_px += 1,
                0x77 => sentinel_px += 1,
                _ => {}
            }
        }
        assert!(
            (0..48 * 48usize).all(|i| px(plane, 0x1E7 + i % 48, 5 + i / 48) != 0),
            "mode 1 never writes zero — the box keeps its pixels (plus the fresh 2x2 portrait dot)"
        );
        assert!(
            sentinel_px > 1000,
            "the portrait survives under zero bytes ({sentinel_px})"
        );
        assert!(
            white_px > 50,
            "flash specks overlay the portrait ({white_px})"
        );
        assert_eq!(
            m.sim.robots()[0].hit_flash,
            3,
            "the pass READS the flash; the 7g.8 decay is the sim tick"
        );
        // The epilogue churn: 15 ring bytes per frame — after 137+
        // frames every byte re-randomized, the bank stays binary.
        for _ in 0..140 {
            m.present();
        }
        assert!(
            m.dither.bank.iter().all(|&b| b == 0 || b == DITHER_WHITE),
            "the churn re-randomizes without leaving the noise alphabet"
        );
    }

    #[test]
    fn bars_and_score_strip_draw_the_exw_semantics() {
        // FUN_0040807f + FUN_004085ce [RE-EXW-SIM 7f, D52]: the bars
        // map the staged vitals through the exact asm sprite
        // arithmetic EVERY present, the strip draws the campaign
        // state on its own countdown (armed 2 at activate, the
        // pickup producer re-arms), the BATTERY PACK group
        // re-derives hp (the landing formula 7f.8), and the pickup's
        // two RandA draws advance the shared sim stream.
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        // Fresh sim vitals + campaign defaults.
        assert_eq!((m.sim.robots()[0].hp, m.sim.robots()[0].armor), (5000, 0));
        assert_eq!(m.campaign(), (0, 4000), "FRESH_CAMPAIGN (7d.4)");
        let plane = m.present().expect("entry frame");
        let px = |p: &[u8], x: usize, y: usize| p[y * 640 + x];
        // HP 5000 -> the full bar 0x18; armor 0 -> the gate sprite
        // 0x8E (the empty armor bar still DRAWS); slots 0/1 of the
        // 2-robot squad, slot 2 gated.
        assert_eq!(px(plane, 0x1E8, 0x3C), 0x90, "slot 0 full HP bar");
        assert_eq!(px(plane, 0x21A, 0x3C), 0x90, "slot 1 HP bar");
        assert_eq!(px(plane, 0x24C, 0x3C), 0, "slot 2 gated (squad < 3)");
        assert_eq!(px(plane, 0x1E8, 0x49), 0x96, "empty armor bar 0x8E draws");
        // Strip: icon 0xA + score "000000000" + icon 0xB + money
        // "004000" (the '4' at the 10^3 digit, x 0x225).
        assert_eq!(px(plane, 0x1FE, 0x18E), 0xFA, "score icon 0xA");
        assert_eq!(px(plane, 0x202, 0x18E), 0xF0, "score digit 10^8 = '0'");
        assert_eq!(px(plane, 0x256, 0x18E), 0xF0, "score digit 10^0");
        assert_eq!(px(plane, 0x20B, 0x1A4), 0xFB, "money icon 0xB");
        assert_eq!(px(plane, 0x225, 0x1A4), 0xF4, "money 10^3 = '4'");
        assert_eq!(px(plane, 0x245, 0x1A4), 0xF0, "money units '0'");
        // Re-staged sim vitals re-map the bars every present; hp 0
        // gates the portrait AND the dither now REPLACES the dead
        // robot's box with full static (mode 0, D55): the portrait
        // pixels do not survive, the box carries the {0, 0xFF}
        // noise. robots_mut is the verified test seam for direct
        // state setup (the damage path lands in the core tests).
        {
            let robots = m.sim.robots_mut();
            robots[0].hp = 2500;
            robots[0].armor = 1250;
            robots[1].hp = 0;
            robots[1].armor = 3000;
        }
        let plane = m.present().expect("bars redraw every present");
        assert_eq!(px(plane, 0x1E8, 0x3C), 0x91, "hp 2500 -> 0x2F");
        assert_eq!(px(plane, 0x1E8, 0x49), 0x94, "armor 1250 -> 0x77");
        assert_eq!(px(plane, 0x21A, 0x3C), 0x92, "hp 0 -> empty 0x46");
        assert_eq!(
            px(plane, 0x21A, 0x49),
            0x93,
            "armor 3000 clamps -> full 0x60"
        );
        assert!(
            (0..48).all(|dy| (0..48).all(|dx| {
                let b = px(plane, 0x219 + dx as usize, 5 + dy as usize);
                b == 0 || b == DITHER_WHITE
            })),
            "dead slot 1: full static replaces the portrait box (mode 0)"
        );
        assert!(
            (0..48 * 48)
                .filter(|&i| plane[(5 + i / 48) * 640 + 0x219 + i % 48] == DITHER_WHITE)
                .count()
                > 100,
            "the static carries the 25% white noise (mode 0 replaces the box)"
        );
        // The strip countdown drained (2 armed - 2 presents): a
        // campaign re-stage alone does NOT redraw the strip; only
        // the case-4 pickup producer re-arms it [7f.6].
        for y in 0x18E..0x1B0 {
            for x in 0x1FE..0x260 {
                m.plane[y as usize * 640 + x] = 0;
            }
        }
        m.set_campaign(123456789, 654321);
        m.present();
        assert_eq!(px(&m.plane, 0x1FE, 0x18E), 0, "no countdown, no strip");
        // The pickup: award + countdown 2 + the two RandA draws move
        // the shared sim stream (the hash covers it).
        let sim_before = m.sim.state_hash().0;
        m.pickup_score_money();
        assert_eq!(m.score_strip_countdown(), 2);
        let (score, money) = m.campaign();
        let scored = PICKUP_AWARDS[0].iter().any(|&a| score == 123456789 + a);
        let moneied = PICKUP_AWARDS[1].iter().any(|&a| money == 654321 + a);
        assert!(
            scored ^ moneied,
            "exactly one award row fires: ({score}, {money})"
        );
        assert_ne!(
            m.sim.state_hash().0,
            sim_before,
            "the two RandA draws advance the shared stream"
        );
        let plane = m.present().expect("the pickup re-arms the strip");
        assert_eq!(px(plane, 0x202, 0x18E), 0xF1, "score still leads '1'");
        assert_ne!(px(plane, 0x1FE, 0x18E), 0, "the icon redrew");
        // The BATTERY PACK group re-derives hp: 5000 + 100*7 [7f.8].
        let mut g = [(0u16, 0u16); 7];
        g[2] = (WEAPON_BATTERY_PACK, 7);
        m.set_weapon_loadout(0, &g);
        assert_eq!(m.sim.robots()[0].hp, 5700, "the landing formula");
        m.set_weapon_loadout(1, &[(0, 0); 7]);
        assert_eq!(m.sim.robots()[1].hp, 5000, "battery 0 -> plain 5000");
    }

    #[test]
    fn pickup_seam_lands_the_fun_0040eba0_cases() {
        // 7h.2: the case-1/2/3/7 host seam writes the sim vitals and
        // stages NO presentation state (no strip re-arm — that is
        // the case-4 producer alone, 7f.6; the effect rows + SFX
        // are unwired, the outcome's effect id stands in).
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        m.present().expect("entry frame");
        for _ in 0..2 {
            m.present();
        }
        assert_eq!(m.score_strip_countdown(), 0, "strip drained");
        // Case 3 heals 2500 clamped at 5000 (the robot spawns full).
        let out = m.pickup(0, 3);
        assert_eq!((out.applied, out.effect), (true, 7));
        assert_eq!(m.sim.robots()[0].hp, 5000, "clamp at 0x1388");
        m.sim.robots_mut()[0].hp = 1000;
        m.pickup(0, 3);
        assert_eq!(m.sim.robots()[0].hp, 3500);
        // Case 2 refills the shield pool, case 7 arms the booster,
        // case 1 stages the reinforcement drop.
        m.pickup(0, 2);
        assert_eq!(m.sim.robots()[0].shield, 1000);
        m.pickup(0, 7);
        assert_eq!(m.sim.robots()[0].shield_boost, 200);
        let out = m.pickup(0, 1);
        assert_eq!((out.applied, out.effect), (true, 1));
        assert_eq!(m.sim.robots()[0].drop_countdown, 1000);
        // None of the four re-arms the score strip; case 4 (7f.6 +
        // §7h.5/2) is applied through the same seam now: the sim
        // draws the award on the shared stream and the NEXT frame's
        // shell fold lands it (session cell + strip countdown 2).
        assert_eq!(m.score_strip_countdown(), 0);
        let (s0, m0) = m.campaign();
        let out = m.pickup(0, 4);
        assert_eq!((out.applied, out.effect), (true, 1));
        assert_eq!(m.score_strip_countdown(), 0, "the fold runs per-frame");
        m.tick(&InputFrame::default());
        let (s1, m1) = m.campaign();
        let ds = s1.wrapping_sub(s0);
        let dm = m1.wrapping_sub(m0);
        assert!(
            (PICKUP_AWARDS[0].contains(&ds) && dm == 0)
                || (PICKUP_AWARDS[1].contains(&dm) && ds == 0),
            "one table value on one side: (+{ds}, +{dm})"
        );
        assert_eq!(m.score_strip_countdown(), 2, "the award arms the strip");
        assert!(!m.pickup(9, 1).applied, "bad robot index");
        // The dispatch decode is the pure half: every set-0 pickup
        // word round-trips word -> case -> the same field family.
        assert_eq!(pickup_case(0x4E, 0), Some(1));
        assert_eq!(pickup_case(0x4E + 8, 0), Some(2));
        assert_eq!(pickup_case(0x4E + 4, 0), Some(3));
        assert_eq!(pickup_case(0x75 + 4, 0), Some(7));
    }

    #[test]
    fn weapon_names_follow_the_compiled_in_switch() {
        // FUN_00420260 [RE 7d.5]: spot rows of the exact index ->
        // string mapping, straight off the PE bytes.
        assert_eq!(weapon_name(2), "NEEDLER CANNON #1");
        assert_eq!(weapon_name(4), "NEEDLER CANNON #3");
        assert_eq!(weapon_name(8), "PLASMA CANNON X3");
        assert_eq!(weapon_name(0xE), "FLAME BOMB");
        assert_eq!(weapon_name(0x16), "PRESSURE MINE X6");
        assert_eq!(weapon_name(0x19), "FRAG GRENADE #2");
        assert_eq!(weapon_name(0x1E), "STICKY GRENADE X6");
        assert_eq!(weapon_name(0x23), "ROCKET PACK X9");
        assert_eq!(weapon_name(0x28), "REAPER PACK X6");
        assert_eq!(weapon_name(0x2A), "AUTO SHIELDING");
        assert_eq!(weapon_name(0x2B), "BATTERY PACK");
        assert_eq!(weapon_name(0x2C), "THERMAL DAMPER");
        assert_eq!(weapon_name(0x2D), "SCANNER LEVEL 2");
        assert_eq!(weapon_name(0x2E), "SCANNER LEVEL 3");
        // Unlisted indices fall to "ERROR" (0 included).
        for i in [0u16, 1, 5, 0xD, 0xF, 0x17, 0x1A, 0x24, 0x29, 0x2F, 0x99] {
            assert_eq!(weapon_name(i), "ERROR", "index {i:#x}");
        }
    }

    #[test]
    fn spawn_armer_picks_the_first_armed_group() {
        // The sec 6c.6 stats-copy armer over staged rows.
        assert_eq!(spawn_order_bits(&[(0, 0); 7]), 0, "empty row -> no bit");
        let mut g = [(0, 0); 7];
        g[4] = (0x20, 9);
        assert_eq!(spawn_order_bits(&g), 1 << 4, "first (and only) group");
        g[0] = (2, 3);
        assert_eq!(spawn_order_bits(&g), 1, "the FIRST group wins");
        // A name-only group (ammo 0) still counts for the armer
        // (the spawn copy keys on word0; the CLICK gate is word1).
        let mut h = [(0, 0); 7];
        h[2] = (9, 0);
        assert_eq!(spawn_order_bits(&h), 1 << 2);
    }

    #[test]
    fn smlfont_text_draws_the_exw_advances() {
        // FUN_00408913 over the synth bank: glyph advance w+1, the
        // space advance 6, chars >= 0x7F skipped.
        let mut plane = vec![0u8; 640 * 8];
        let bank = {
            let f = synth_mission_files();
            f[11].clone()
        };
        draw_smlfont_text(&mut plane, &bank, "A B", 0x24, 10, 1);
        let px = |x: usize| plane[640 + x];
        assert_eq!(px(10), 0x24, "'A' at x");
        assert_eq!(px(11), 0x24, "glyph 2 wide");
        assert_eq!(px(12), 0, "advance gap");
        assert_eq!(px(13), 0, "the space draws nothing");
        assert_eq!(px(19), 0x24, "'B' at 10 + 3 + 6 (w+1 + space)");
        assert_eq!(px(20), 0x24);
        // A high byte draws nothing and does not advance (the EXW
        // remap FUN_00410493 is unreachable for the ASCII names).
        let mut plane2 = vec![0u8; 640 * 8];
        draw_smlfont_text(&mut plane2, &bank, "\u{7F}A", 0x24, 0, 0);
        assert_eq!(plane2[0], 0x24, "'A' paints at x=0 (0x7F skipped)");
        assert_eq!(plane2[1], 0x24);
        assert_eq!(plane2[2], 0, "advance gap after the 2-px glyph");
    }

    #[test]
    fn sidebar_state_never_reaches_the_sim_hash() {
        // D17 split pin: identical tick counts + a sidebar click vs a
        // map-strip toggle click -> identical sim state hashes (both
        // halves are presentation-only).
        let mut a = staged(&[(3, 1, 1)]);
        let mut b = staged(&[(3, 1, 1)]);
        a.activate();
        b.activate();
        // Both carry a staged loadout (row 0 exists) so the toggle
        // actually fires on a's second click.
        let mut g = [(0, 0); 7];
        g[0] = (2, 30);
        a.set_weapon_loadout(1, &g);
        b.set_weapon_loadout(1, &g);
        // a: strip-1 select + row-0 toggle; b: two map-strip clicks
        // (the second is inside the 5-frame lockout, so the overlay
        // stays on — see the toggle test below).
        sidebar_click(&mut a, 0x219, 5);
        sidebar_click(&mut a, 0x200, 0x57);
        sidebar_click(&mut b, 0x230, 0x1C0);
        sidebar_click(&mut b, 0x230, 0x1C0);
        assert_eq!(a.state_hash(), b.state_hash());
        assert_ne!(a.order_bits(1), b.order_bits(1), "sidebar did change");
    }

    #[test]
    fn map_toggle_strip_toggles_with_lockout_and_swallows_clicks() {
        // [RE-EXW-SIM 7e.5] The strip: lockout gate, 5-frame re-arm,
        // the overlay bit; clicks in the game area are swallowed
        // while the overlay is on (asm 0x40b868).
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        assert!(!m.map_overlay_on());
        sidebar_click(&mut m, 0x230, 0x1C0);
        assert!(m.map_overlay_on(), "the strip toggles the overlay on");
        assert_eq!(m.map_lockout(), 4, "click tick + move tick consume 2 of 5");
        // A click inside the lockout window does not toggle back.
        sidebar_click(&mut m, 0x230, 0x1C0);
        assert!(m.map_overlay_on(), "locked out for 5 frames");
        for _ in 0..4 {
            m.tick(&InputFrame::default());
        }
        assert_eq!(m.map_lockout(), 0);
        sidebar_click(&mut m, 0x240, 0x1C5);
        assert!(!m.map_overlay_on(), "toggles back once the lockout spends");
        // Game-area clicks while the overlay is on are swallowed:
        // the sim hash is identical to a no-click run.
        let mut a = staged(&[(3, 1, 1)]);
        a.activate();
        sidebar_click(&mut a, 0x230, 0x1C0);
        assert!(a.map_overlay_on());
        let mut b = staged(&[(3, 1, 1)]);
        b.activate();
        sidebar_click(&mut b, 0x230, 0x1C0);
        // a clicks at the game area (would arm an order at a robot);
        // b does nothing.
        sidebar_click(&mut a, 0x10, 0x10);
        b.tick(&InputFrame::default());
        b.tick(&InputFrame::default());
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "overlay-on game-area clicks never reach the sim"
        );
    }

    #[test]
    fn map_overlay_frame_composes_backdrop_stamps_and_markers() {
        // [RE-EXW-SIM 7e.1] The overlay frame: clear + TABLE.BIN
        // backdrop at (0,0), per-(tile,z) MIN/MAPTRAN stamps at the
        // 2:1 lattice, GENERAL 0x55/0x56 robot markers; the sidebar
        // passes + the button chrome are SKIPPED (the non-returning
        // tail), so the sidebar half keeps its stale pixels.
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        // One normal frame first: the chrome 0x5E (0xE5) draws at
        // (0x213,0x1b5), the portraits land.
        let plane = m.present().expect("normal frame");
        let px = |p: &[u8], x: usize, y: usize| p[y * 640 + x];
        assert_eq!(px(plane, 0x213, 0x1B5), 0xE5, "map button chrome 0x5E");
        assert_eq!(px(plane, 0x1E7, 5), 0x32, "portrait draws on normal frames");
        // Toggle the overlay on.
        sidebar_click(&mut m, 0x230, 0x1C0);
        let plane = m.present().expect("overlay frame");
        // Backdrop 0x77 at (0,0).
        assert_eq!(px(plane, 0, 0), 0x77, "TABLE.BIN image 0 at (0,0)");
        // Stamps: synth TOT plane 0 = word 1 on every tile, LNK[1]
        // = 2 (stable loop) -> mask 2 (the X wedge, byte 5) through
        // ramp 0 (variant 0) -> color 0x60+5 = 0x65 at the lattice
        // cells; tile (0,0) z 0 lands at (row 0x80, col 0xF0).
        assert_eq!(
            px(plane, 0xF0, 0x80),
            0x65,
            "territory stamp at the lattice"
        );
        assert_eq!(px(plane, 0xF3, 0x83), 0x65, "mask corner");
        // Row 0x80 is only reachable from tile (0,0)'s local row 0
        // (every other stamp lands on later rows), so the wedge's
        // off-cell there stays clear.
        assert_eq!(px(plane, 0xF1, 0x80), 0, "the wedge's off pixels");
        // Robot marker: robot 0 spawns at Q13 (1,1)+center, z = 31
        // (Q5) -> tile (1,1), selected -> sprite 0x55 (0xE1) at px
        // 2-2+0xF0-0xC = 228, py 1+1+0x80-0x1E-(31>>4) = 99.
        assert_eq!(px(plane, 228, 99), 0xE1, "selected robot marker 0x55");
        assert_eq!(px(plane, 229, 100), 0xE1);
        // The chrome + portraits are SKIPPED on overlay frames (the
        // stale pixels survive the clear — the clear covers only the
        // viewport half).
        assert_eq!(
            px(plane, 0x213, 0x1B5),
            0xE5,
            "chrome pixel survives (stale)"
        );
        assert_eq!(px(plane, 0x1E7, 5), 0x32, "portrait pixel survives (stale)");
        // The viewport half was cleared below the backdrop's 2x2.
        assert_eq!(px(plane, 5, 5), 0, "cleared viewport half");
        // The territory rings: no robot has moved yet (state 0), so
        // every variant is 0 — the stamps all use ramp 0.
        assert_eq!(m.map_territory(0), 0);
    }

    #[test]
    fn moving_robots_stamp_territory_rings() {
        // FUN_00408dcc runs in the robots() tick for moving machines
        // [7e.3]: the walker's PATH acquires ring values that persist
        // after arrival (the variant array is a max-accumulate — the
        // corpus pattern: the order arms at robot 0, robot 1 walks
        // from (3,1) to (1,1)).
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        m.sim_mut().arm_order_at_robot(0);
        for _ in 0..8 {
            m.tick(&InputFrame::default());
        }
        let robots = m.sim().robots();
        assert_eq!(robots[1].state, 3, "walker arrived (state 3)");
        let (w, _) = m.view_size();
        let w = w as usize;
        // The 4x4 synth map clips every stamp's 11x11 scan to 16
        // accepted tiles, so the ring values stay small — but the
        // path tiles all carry rings while untouched tiles stay 0.
        assert!(
            m.map_territory(w + 1) >= 3,
            "arrival tile carries a ring ({})",
            m.map_territory(w + 1)
        );
        assert!(
            m.map_territory(3 * w + 1) >= 1,
            "the path start keeps a ring"
        );
        // The 11-wide stamps around the path cover the whole 4x4 map
        // — every tile carries some ring (no untouched tile exists
        // at this size; the render unit tests cover the unclipped
        // semantics).
        assert!(
            (0..w * w).all(|t| m.map_territory(t) >= 1),
            "the stamps blanket the small map"
        );
    }

    // --- RE-EXW-SIM 7j: the effect-row + debris-stager units -----

    #[test]
    fn effect_rows_alloc_first_free_then_last() {
        // FUN_00422038 [7j.2]: first id==0 row, else 9 (the last).
        let mut rows = EffectRows::new();
        assert_eq!(rows.alloc(), 0);
        rows.stage(1, 2, 3, 6);
        assert_eq!(rows.alloc(), 1, "row 0 busy");
        for _ in 1..EFFECT_ROWS {
            rows.stage(0, 0, 0, 7);
        }
        assert_eq!(rows.alloc(), EFFECT_ROWS - 1, "all busy -> the last");
    }

    #[test]
    fn effect_rows_tick_rises_then_frees() {
        // FUN_0042205c [7j.3]: z += 6 while <= 0x190, then id = 0.
        let mut rows = EffectRows::new();
        rows.stage(10, 20, 0x190, 6);
        rows.tick();
        assert_eq!(rows.rows[0].2, 0x196, "the cap tick still rises");
        rows.tick();
        assert_eq!(rows.rows[0].3, 0, "past the cap the row frees");
        assert_eq!(rows.rows[0].0, 0, "the freed row is zeroed");
    }

    #[test]
    fn debris_stage_clamps_z_and_lru_evicts() {
        // FUN_00420608 head [7j.5]: the z clamp 0x20..0xFF; the
        // first inactive slot; all-busy -> the SMALLEST seq.
        let mut fx = DebrisFx::new();
        fx.stage_kind5(10, 10, 0x08, 0);
        assert_eq!(fx.recs[0].z, 0x20, "low clamp");
        fx.stage_kind5(10, 10, 0x1234, 2);
        assert_eq!(fx.recs[1].z, 0xFF, "high clamp");
        // Age record 0 two ticks (delay 0), record 1 held by delay.
        fx.tick();
        fx.tick();
        assert_eq!(fx.recs[0].seq, 2);
        assert_eq!(fx.recs[1].seq, 0, "delayed records hold seq");
        // Fill the rest; the LRU (smallest seq) is record 1.
        for _ in 2..DEBRIS_SLOTS {
            fx.stage_kind5(0, 0, 0x20, 99);
        }
        assert!(fx.recs.iter().all(|r| r.active));
        fx.stage_kind5(5, 5, 0x20, 0);
        assert_eq!(fx.recs[1].x, 5, "the min-seq record was evicted");
        assert_eq!(fx.recs[1].seq, 0, "staged seq resets to 0");
    }

    #[test]
    fn debris_tick_walks_the_kind5_table_and_frees() {
        // FUN_00420549 [7j.7]: delay decrements first; then seq += 1
        // and the −1 terminator frees the record. 13 table words:
        // the 14th seq read is past the table -> free.
        let mut fx = DebrisFx::new();
        fx.stage_kind5(1, 2, 0x20, 1);
        assert!(fx.recs[0].active);
        fx.tick();
        assert_eq!(fx.recs[0].delay, 0, "delay spent, seq still 0");
        fx.tick();
        assert_eq!(fx.recs[0].seq, 1, "the seq walk starts after the delay");
        assert!(fx.recs[0].active);
        for _ in 0..13 {
            fx.tick();
        }
        assert!(!fx.recs[0].active, "the -1 terminator freed the record");
        assert_eq!(fx.recs[0].x, 0, "the freed record is default");
    }

    #[test]
    fn damage_death_stages_five_debris_and_pickup_stages_a_row() {
        // The host seams [7g.6 + 7j.1]: the death outcome's five
        // rows land as kind-5 records with the 2k delays; the
        // pickup outcome stages one row at the robot with the
        // per-case id.
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        // Kill robot 0 (state 0, alive, hp 5000): 6000 damage.
        let out = m.apply_damage(0, 6000, -1);
        assert!(out.died);
        assert_eq!(m.debris_active(), 5, "five kind-5 records");
        // Delays 0/2/4/6/8 (the 2k stagger).
        let delays: Vec<i32> = m
            .debris
            .recs
            .iter()
            .filter(|r| r.active)
            .map(|r| r.delay)
            .collect();
        assert_eq!(delays, vec![0, 2, 4, 6, 8]);
        // A shield pickup (case 2 -> id 6) at robot 1's position.
        let outcome = m.pickup(1, 2);
        assert!(outcome.applied);
        assert_eq!(outcome.effect, 6);
        assert_eq!(m.effect_row_count(), 1);
        let robot = &m.sim().robots()[1];
        assert_eq!(
            m.effect_rows.rows[0],
            (robot.pos_x >> 8, robot.pos_y >> 8, robot.z + 0x20, 6)
        );
    }

    #[test]
    fn select_strip_click_lights_the_blink_cursor() {
        // The select-ack cursor [7j.6]: 0 until a strip fires, then
        // slot + 1.
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        assert_eq!(m.sidebar_cursor(), 0, "MissionShell entry zero");
        // Click on strip 1 (the second portrait band).
        m.cursor = (0x219, 5);
        m.tick(&InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        });
        assert_eq!(m.sidebar.selected, 1);
        assert_eq!(m.sidebar_cursor(), 2, "selected slot + 1");
    }
}
