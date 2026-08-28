//! The W6 canonical dump emitter (DESIGN-DIFFHARNESS.md §6a + §10-W6,
//! D85) — the E side of the differ.
//!
//! Two halves:
//!
//! 1. **The field maps** ([`TickState`] → [`emit_frame`]): the
//!    canonical record grammar per DESIGN §6a — little-endian, no
//!    padding, fixed field order per row, registry ids. This is the
//!    CONTRACT: W7's normalizer must convert O1/O2 raw guest bytes
//!    into the same grammar. Rows the engine does not model are
//!    E-gaps (listed in §6a) and are simply not emitted.
//! 2. **The runner** ([`run_canonical`]): consumes the SAME scenario
//!    grammar as the O1 side (the D82 shared seam — literally the
//!    `diffharness::runner` parser), drives `GameHost` one mission
//!    frame per boundary (tick + present), snapshots the state at the
//!    frame tail, and stitches the dump through the SAME
//!    validation/encode path as O1 captures (`runner::stitch` +
//!    `encode_dump`, channel E). Byte-deterministic by construction.
//!
//! Frame model (§6a): one record per `pump_frame(dt=4)`; the ANCHOR
//! record is the tail of the FIRST mission tick (`frame_no` 0, the TS
//! statics ride it); then strictly increasing; total records equal
//! anchor plus `frames`. `frame-counter` is the PRE-increment value
//! (`sim.frame()−1`), matching the O1 dump point (its counter
//! increments only after the flip).
//!
//! Scenario step semantics (§6a): walk phase may carry ONLY `boot`
//! steps (any other walk step — the S0W menu-walk shape — is rejected;
//! the E menu-walk seam waits on the P2e button bit-map); `keystore`
//! maps through the pinned EMPTY scan map (no engine keyboard
//! consumer yet; P 0x19 rejected per the §2 pause rule); `order x y
//! z` is the click-order seam (target recorded + `arm_order_at_robot`
//! at the tile-exact alive robot); `command <hex bytes>` is the
//! CONSUMED fire seam (W12-S3-prep, §7j.37): the payload stages as
//! the next COMMAND record in the sim ring and the pumped frame's
//! MissionShell pass consumes it (FUN_00409138 — a ≥14 B payload
//! fails loud); `pad` is still rejected naming the S6 extraction
//! seam; `boot difficulty=d` OVERRIDES the fresh-session difficulty
//! default (§7j.64/A: the GameMain boot head writes DIFFICULTY := 1
//! at 0x41c14a — E's no-boot-step default is that 1, S0-12b/D154)
//! and seeds the campaign money via the engine's own
//! `menu::start_score` formula + the sim's difficulty dword (the
//! scaled damage rows); the seed applies on EVERY run — the
//! name-entry fresh-campaign arm 0x43aaca re-seeds money at every
//! campaign start (§7j.64/C).
//! The `markers` header key (D91) stages extra squad robots through
//! the existing `load_mission(staged_markers)` seam after the MRK
//! robots — the walk seam (the click-order moves only the OTHER
//! robots in radius, so order→walk scenarios stage a walker). The
//! `loadout` header key (W12-S3, grammar v1.3) stages per-robot
//! weapon slots + the enable mask through `stage_robot_weapons` —
//! the fire-path seam S3's COMMAND steps consume (the original fills
//! the slots at spawn from the session table; an E-side staging
//! seam, recorded never fabricated). The `zone` + `pickup` header
//! keys (W12-S5, grammar v1.5, D108) stage the campaign episode slot
//! (the host seam standing in for the campaign/save-load shells) and
//! the mission's own .TOT pickup surface (after any destroy staging,
//! then the hazard stamper — the original mission-load order) — both
//! EQUIVALENCE seams recorded in the plan; S5/S5B run ZONEB set 2.
//! The `mission` header key (grammar v1.8, the P5 per-zone
//! disposition family) selects the zone's within-zone mission:
//! 1..=5 through the campaign mask (the campaign-advance slot
//! state whose first-uncompleted sub is that mission), 6..=7
//! through the SELECT MP write pair (`stage_select_mission`, the
//! §7j.73 MP-only files).
//!
//! T2 tier (W12-S3): the scenario's tier list gates the two bank
//! rows — `weapon-anim-bank` (the 400×0x36 blob, byte layout = the
//! guest record) + `projectile-bank` (the 50×0x22 blob, 7 mapped
//! fields + the zeroed +0x1A/+0x1E tail). Full bank by design (the
//! O1 raw side has no count cell; slot identity is the watched
//! state); they stay OUT of `state_hash` (the W6 split).

use std::fmt;
use std::path::{Path, PathBuf};

use bedlam_core::destroy::{
    DebrisRecord, ObjectInstance, ObjectTypeTable, SplashRecord, TerrainStructure,
};
use bedlam_core::input::InputFrame;
use bedlam_core::mission::Robot;
use bedlam_core::sim::SimConfig;
use bedlam_core::weapon::{CommandRecord, EnemyProjectile, WeaponRecord, WeaponSlot};
use bedlam_game::{ByteSource, GameConfig, GameError, GameHost, SceneAction};
use diffharness::dump::{Channel, DumpHeader, FrameRecord};
use diffharness::hash::sha256;
use diffharness::runner::{Scenario, Step, StitchError, Stitched, Transcript};
use diffharness::Watch;

/// Host sub-ticks per canonical frame: the 240 Hz grid quantized to
/// the 60 Hz tick (exactly one executed tick per frame; the runner
/// errors on any other cadence).
const DT_SUBTICKS: u32 = 4;

/// Mission-phase scan codes with no E-side consumer are rejected only
/// when the engine could never honor them: P-pause freezes the shell
/// in a present-only spin (DESIGN §2 — the runner must never inject
/// it mid-scenario).
const BANNED_SCANS: [u8; 1] = [0x19];

// ---------------------------------------------------------------------
// The canonical field maps (DESIGN §6a)
// ---------------------------------------------------------------------

/// One frame's engine state, snapshotted at the frame tail. Built by
/// the runner from `(&GameHost, session)` or by hand in tests (the
/// synthetic fixture — every field is plain data on purpose).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickState<'a> {
    /// Pre-increment mission frame counter (the O1 dump-point value).
    pub frame_no: u64,
    pub rand_a_state: u64,
    pub rand_b_state: u64,
    pub score: i32,
    pub money: i32,
    pub difficulty: u32,
    pub zone: u32,
    pub mission: u32,
    pub mode: u32,
    /// The DERIVED linear-mission-m cell (§7j.64/D — clamp of
    /// 5·(zone−2)+mission−1 into 1..=26; S0-12b/D154).
    pub linear: u32,
    pub robots: &'a [Robot],
    /// The armed click order (beacon-family + spread-claims source).
    pub order: Option<bedlam_core::mission::Order>,
    /// The surviving beacon tile words 0x4eabb4/6/8 (§7j.40/4 —
    /// FUN_0041faf0 clears only the flag/window pair): the
    /// beacon-family row's post-deploy source. `None` on every
    /// click-seam scenario (the pre-S6 approximation keeps the
    /// all-zero clear).
    pub beacon_latch: Option<(i32, i32, i32)>,
    /// The surviving spread-claim words 0x4eabba (never released —
    /// §7j.20/3): the spread-claims row's post-deploy source.
    pub claims_latch: [bool; 12],
    /// The extraction dropship record (the T3 `dropship-frame` row,
    /// W12-S6): `Some` only on pad-step scenarios.
    pub dropship: Option<bedlam_core::mission::CraftRecord>,
    pub selected: usize,
    pub blink_cursor: i32,
    /// The ORDER-seam write (persists like the 0x4dd484 cells).
    pub order_target: (i32, i32, i32),
    pub armor_pads: &'a [u8],
    /// TS statics ride the anchor frame only.
    pub map_wh: Option<(u32, u32)>,
    /// The staged tile-claim bank (S0-11b, §7j.63) — the
    /// `static-claim-bank` TS row's source, anchor frame only like
    /// every TS static. Empty slice = unstaged (never on
    /// `run_canonical` paths — `load_mission` stages the full
    /// 0x2710 image; hand-built fixtures choose their own).
    pub claim_bank: &'a [u8],
    /// The player TYPE word [0x4edb90] (§7j.68/D159) — the
    /// `static-player-type` TS row's source, anchor frame only.
    /// 0 on every canonical path (SP; the sim has no setter).
    pub player_type: u16,
    /// The T2 watch surfaces (W12-S3): the two projectile banks,
    /// emitted WHOLE per frame — never in `state_hash` (the W6
    /// split: watched bank rows are their own dump blobs).
    pub weapon_bank: &'a [WeaponRecord],
    pub enemy_bank: &'a [EnemyProjectile],
    /// The critter-family watch surfaces (W12-S8), gated on the
    /// scenario's `critters = 1` key — `None` on S0..S7 (their
    /// pinned bytes carry no critter rows). The bank is the T2
    /// `critter-bank` row and the effect-row bank the T3
    /// `effect-rows` row — both EXD-aliased (D162, §5i) and
    /// cross-channel compared since the subset-form O1 arms
    /// landed. NEVER in `state_hash` (the W6 split).
    pub critter: Option<CritterView<'a>>,
    /// The destroy-family watch surfaces (W12-S4), gated on the
    /// scenario's `destroy = 1` staging key — `None` on S0..S3
    /// (their pinned bytes carry no destroy rows). NEVER in
    /// `state_hash` (the W6 split).
    pub destroy: Option<DestroyView<'a>>,
}

/// The per-frame critter-family surfaces (W12-S8, DESIGN §7 S8
/// row + the registry's T2 critter-bank / T3 effect-rows rows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CritterView<'a> {
    /// The 0x4cff98 bank (count cell 0x46cc2c).
    pub critters: &'a [bedlam_core::critter::CritterRecord],
    /// The 0x4cec38 effect-row bank (§7j.24/5).
    pub effect_rows: &'a [bedlam_core::critter::EffectRow],
}

/// The per-frame destroy-family surfaces (W12-S4, DESIGN §7 S4 row
/// and the registry's T1/T3 destroy rows). All blobs are their own
/// dump rows; the forms are pinned in DESIGN §6a and mirrored by
/// the W7 differ normalizers (engine form / O1 guest form).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyView<'a> {
    /// The staged + live object instances (.POS order).
    pub objects: &'a [ObjectInstance],
    /// The .TRT terrain structures (turrets).
    pub structures: &'a [TerrainStructure],
    /// The object-presence grid words (tile-major).
    pub tile_grid: &'a [u16],
    /// The platform-strength words (tile-major).
    pub platform: &'a [u16],
    /// The TOT-mirror plane words (tile-major, 8 per tile).
    pub mirror_words: &'a [u16],
    /// The TOT-mirror seen bytes (tile-major, 8 per tile).
    pub mirror_seen: &'a [u8],
    /// The 128-slot debris ring (full bank — slot identity watched).
    pub debris: &'a [DebrisRecord],
    /// The 250-slot splash bank (full bank).
    pub splashes: &'a [SplashRecord],
}

fn u32b(v: u32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// Encode the 400×0x36 weapon-anim bank (the T2 row, W12-S3): u32
/// slot count + the records in the §6a canonical field order — which
/// IS the guest 0x36 layout byte-for-byte (type w@+0, owner, target,
/// tick, draw_ctr, xyz, v, class, arc, trail — 2 + 13·4 = 0x36, no
/// gaps: the EXD 0x980d4 twin writes the same offsets, RE-EXD-MAP
/// §5c). FULL BANK by design: the O1 raw side has no count cell to
/// bound a compact form, and slot-index identity (first-free
/// allocation) is the state the row watches.
fn weapon_bank_blob(bank: &[WeaponRecord]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + bank.len() * 0x36);
    b.extend_from_slice(&(bank.len() as u32).to_le_bytes());
    for r in bank {
        b.extend_from_slice(&r.kind.to_le_bytes());
        b.extend_from_slice(&r.owner.to_le_bytes());
        b.extend_from_slice(&r.target.to_le_bytes());
        b.extend_from_slice(&r.tick.to_le_bytes());
        b.extend_from_slice(&r.draw_ctr.to_le_bytes());
        b.extend_from_slice(&r.x.to_le_bytes());
        b.extend_from_slice(&r.y.to_le_bytes());
        b.extend_from_slice(&r.z.to_le_bytes());
        b.extend_from_slice(&r.vx.to_le_bytes());
        b.extend_from_slice(&r.vy.to_le_bytes());
        b.extend_from_slice(&r.vz.to_le_bytes());
        b.extend_from_slice(&r.class.to_le_bytes());
        b.extend_from_slice(&r.arc.to_le_bytes());
        b.extend_from_slice(&r.trail.to_le_bytes());
    }
    b
}

/// Encode the 50×0x22 projectile bank (the T2 row, W12-S3): u32 slot
/// count + records {type w@+0, x, y, z, vx, vy, vz} — the first
/// 0x1A bytes of the guest stride (the +0x1A clamp-counter and the
/// +0x1E countdown tail words are E-gaps, RE-EXD-MAP §5c — the O1
/// normalizer maps only the 7 fields; the tail is a documented
/// coverage gap, never fabricated).
fn enemy_bank_blob(bank: &[EnemyProjectile]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + bank.len() * 0x22);
    b.extend_from_slice(&(bank.len() as u32).to_le_bytes());
    for r in bank {
        b.extend_from_slice(&r.kind.to_le_bytes());
        b.extend_from_slice(&r.x.to_le_bytes());
        b.extend_from_slice(&r.y.to_le_bytes());
        b.extend_from_slice(&r.z.to_le_bytes());
        b.extend_from_slice(&r.vx.to_le_bytes());
        b.extend_from_slice(&r.vy.to_le_bytes());
        b.extend_from_slice(&r.vz.to_le_bytes());
        b.extend_from_slice(&[0u8; 8]); // +0x1A/+0x1E tail (E-gap, zero)
    }
    b
}

/// Encode the object-instance row (W12-S4): u32 live count + per
/// live record `{slot u16, x i32, y i32, z i32, id u32, flags u8,
/// hp i32}` — the id dword carries the guest shape (low byte = the
/// type-table row, +0x08 bit = the destroyed 0x40 flag byte). The
/// O1 normalizer walks the guest 0x14-stride bank (count cell
/// bounded, dead id==-1 records skipped) into the same fields keyed
/// by slot — the guest count cell itself is capture plumbing, not
/// canonical state (never compared).
fn objects_blob(objects: &[ObjectInstance]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + objects.len() * 23);
    b.extend_from_slice(&(objects.len() as u32).to_le_bytes());
    for o in objects {
        b.extend_from_slice(&o.slot.to_le_bytes());
        b.extend_from_slice(&o.x.to_le_bytes());
        b.extend_from_slice(&o.y.to_le_bytes());
        b.extend_from_slice(&o.z.to_le_bytes());
        b.extend_from_slice(&(o.id as u32 & 0xFF).to_le_bytes());
        b.push(if o.destroyed { 0x40 } else { 0 });
        b.extend_from_slice(&o.hp.to_le_bytes());
    }
    b
}

/// Encode the TRT structure row (W12-S4): u32 count + per record
/// `{active i32, hp i32, x i32, y i32, z i32}` — the resolver
/// read-set of FUN_0041bc1c (§7j.14). The loader's scratch words
/// (+0x04/+0x08/+0x0C of the 0x20 stride) are the turret-AI
/// E-gap, out of the row.
fn structures_blob(structures: &[TerrainStructure]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + structures.len() * 20);
    b.extend_from_slice(&(structures.len() as u32).to_le_bytes());
    for s in structures {
        b.extend_from_slice(&i32::from(s.active).to_le_bytes());
        b.extend_from_slice(&s.hp.to_le_bytes());
        b.extend_from_slice(&s.x.to_le_bytes());
        b.extend_from_slice(&s.y.to_le_bytes());
        b.extend_from_slice(&s.z.to_le_bytes());
    }
    b
}

/// Encode the TOT-mirror row (W12-S4): u32 count + per CHANGED
/// tile `{tile u16, 8×(word u16, seen u8)}` (26 B) — compact-active
/// form: only tiles with any nonzero word/seen ride (E's banks
/// stage EMPTY until the destroy RESTORE writes; the full w·h·0x1E
/// grid is the O1 raw span, its normalizer applies the same
/// nonzero-tile filter — identical content canonicalizes
/// identically). The +0x18..+0x1D record tail (scorch/variant/
/// flag/heights) is the fade/variant rows' surface — excluded.
fn mirror_blob(words: &[u16], seen: &[u8]) -> Vec<u8> {
    let tiles = words.len() / 8;
    let mut changed: Vec<usize> = Vec::new();
    for t in 0..tiles {
        let mut nz = false;
        for z in 0..8 {
            if words[t * 8 + z] != 0 || seen[t * 8 + z] != 0 {
                nz = true;
                break;
            }
        }
        if nz {
            changed.push(t);
        }
    }
    let mut b = Vec::with_capacity(4 + changed.len() * 26);
    b.extend_from_slice(&(changed.len() as u32).to_le_bytes());
    for &t in &changed {
        b.extend_from_slice(&(t as u16).to_le_bytes());
        for z in 0..8 {
            b.extend_from_slice(&words[t * 8 + z].to_le_bytes());
            b.push(seen[t * 8 + z]);
        }
    }
    b
}

/// Encode the debris-ring row (W12-S4): u32 128 + the FULL bank —
/// `{active u8, x, y, z, init_a, init_b, seq, kind, phys, delay,
/// param i32×10, table u8}` (42 B, the E-modeled field set of the
/// 0x30 stride; the unmapped words are out of the row). FULL BANK
/// like the T2 rows: slot identity (first-free/LRU allocation) is
/// the watched state. EXD alias 0x93064 (D162, §5i); the O1
/// normalizer projects the four compared leaves off the guest
/// 0x30 records.
fn debris_blob(debris: &[DebrisRecord]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + debris.len() * 42);
    b.extend_from_slice(&(debris.len() as u32).to_le_bytes());
    for d in debris {
        b.push(u8::from(d.active));
        b.extend_from_slice(&d.x.to_le_bytes());
        b.extend_from_slice(&d.y.to_le_bytes());
        b.extend_from_slice(&d.z.to_le_bytes());
        b.extend_from_slice(&d.init_a.to_le_bytes());
        b.extend_from_slice(&d.init_b.to_le_bytes());
        b.extend_from_slice(&d.seq.to_le_bytes());
        b.extend_from_slice(&d.kind.to_le_bytes());
        b.extend_from_slice(&d.phys.to_le_bytes());
        b.extend_from_slice(&d.delay.to_le_bytes());
        b.extend_from_slice(&d.param.to_le_bytes());
        b.push(d.table);
    }
    b
}

/// Encode the splash-bank row (W12-S4): u32 250 + the FULL bank of
/// `{x i16, y i16, z i16, delay u16, age u16}` — the guest 0xA
/// stride exactly. EXD alias 0x107774 (D162, §5i); the O1 side is
/// the bare span with the count synthesized from the fixed bank.
fn splash_blob(splashes: &[SplashRecord]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + splashes.len() * 10);
    b.extend_from_slice(&(splashes.len() as u32).to_le_bytes());
    for s in splashes {
        b.extend_from_slice(&s.x.to_le_bytes());
        b.extend_from_slice(&s.y.to_le_bytes());
        b.extend_from_slice(&s.z.to_le_bytes());
        b.extend_from_slice(&s.delay.to_le_bytes());
        b.extend_from_slice(&s.age.to_le_bytes());
    }
    b
}

/// The bare u16 grid span (tile-word-grid / platform-strength):
/// both channels dump the same w·h·2 span — one shared field walk
/// in the differ (a live O1 grid whose extent differs is a
/// structural finding, fail loud).
fn grid_blob(grid: &[u16]) -> Vec<u8> {
    let mut b = Vec::with_capacity(grid.len() * 2);
    for g in grid {
        b.extend_from_slice(&g.to_le_bytes());
    }
    b
}

/// Encode the robot bank: u32 count + per-robot records in the
/// `MissionSim::state_hash` field order (the pinned modeled-field
/// list — the W7 normalizer must emit the same order).
fn robot_bank_blob(robots: &[Robot]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + robots.len() * 96);
    b.extend_from_slice(&(robots.len() as u32).to_le_bytes());
    for r in robots {
        b.push(u8::from(r.alive));
        b.extend_from_slice(&r.pos_x.to_le_bytes());
        b.extend_from_slice(&r.pos_y.to_le_bytes());
        b.extend_from_slice(&r.z.to_le_bytes());
        b.extend_from_slice(&r.state.to_le_bytes());
        b.extend_from_slice(&r.dir_byte.to_le_bytes());
        b.extend_from_slice(&r.facing.to_le_bytes());
        b.extend_from_slice(&r.anim.to_le_bytes());
        b.extend_from_slice(&r.variant.to_le_bytes());
        for z in r.probe_z {
            b.extend_from_slice(&z.to_le_bytes());
        }
        b.extend_from_slice(&r.stop_dist.to_le_bytes());
        match r.target {
            None => {
                b.push(0);
                b.extend_from_slice(&0i32.to_le_bytes());
                b.extend_from_slice(&0i32.to_le_bytes());
            }
            Some((tx, ty)) => {
                b.push(1);
                b.extend_from_slice(&tx.to_le_bytes());
                b.extend_from_slice(&ty.to_le_bytes());
            }
        }
        b.extend_from_slice(&r.drop_countdown.to_le_bytes());
        b.extend_from_slice(&r.hp.to_le_bytes());
        b.extend_from_slice(&r.armor.to_le_bytes());
        b.extend_from_slice(&r.hit_flash.to_le_bytes());
        b.extend_from_slice(&r.alarm.to_le_bytes());
        b.extend_from_slice(&r.kind.to_le_bytes());
        b.extend_from_slice(&r.shield.to_le_bytes());
        b.extend_from_slice(&r.shield_charges.to_le_bytes());
        b.extend_from_slice(&r.shield_boost.to_le_bytes());
        b.extend_from_slice(&r.battery.to_le_bytes());
        b.extend_from_slice(&r.armor_pool.to_le_bytes());
        b.extend_from_slice(&r.alarm_ctr.to_le_bytes());
        b.extend_from_slice(&r.death_flag.to_le_bytes());
    }
    b
}

/// The critter-bank blob (T2 row, W12-S8; EXD alias 0x10e81c +
/// count cell 0x1194dc — D162, §5i): u32 count +
/// count × the modeled 0x7E-record subset, the field order pinned
/// here (kind, species, attacker, hp, mode, anim, heading,
/// presence, target triple, impact pair, xyz, home pair,
/// countdown, death_ctr, target_robot, fuse, facing) — the
/// differ's `critter-bank` normalizer mirrors it.
fn critter_bank_blob(cs: &[bedlam_core::critter::CritterRecord]) -> Vec<u8> {
    // 74 B per record (the differ's critter-bank normalizer pins it).
    let mut b = Vec::with_capacity(4 + cs.len() * 74);
    b.extend_from_slice(&(cs.len() as u32).to_le_bytes());
    for c in cs {
        b.extend_from_slice(&c.kind.to_le_bytes());
        b.extend_from_slice(&c.species.to_le_bytes());
        b.extend_from_slice(&c.attacker.to_le_bytes());
        b.extend_from_slice(&c.hp.to_le_bytes());
        b.extend_from_slice(&c.mode.to_le_bytes());
        b.extend_from_slice(&c.anim.to_le_bytes());
        b.extend_from_slice(&c.heading.to_le_bytes());
        b.extend_from_slice(&u32::from(c.presence).to_le_bytes());
        b.extend_from_slice(&c.target_x.to_le_bytes());
        b.extend_from_slice(&c.target_y.to_le_bytes());
        b.extend_from_slice(&c.target_z.to_le_bytes());
        b.extend_from_slice(&c.impact_x.to_le_bytes());
        b.extend_from_slice(&c.impact_y.to_le_bytes());
        b.extend_from_slice(&c.x.to_le_bytes());
        b.extend_from_slice(&c.y.to_le_bytes());
        b.extend_from_slice(&c.z.to_le_bytes());
        b.extend_from_slice(&c.home_x.to_le_bytes());
        b.extend_from_slice(&c.home_y.to_le_bytes());
        b.extend_from_slice(&c.countdown.to_le_bytes());
        b.extend_from_slice(&c.death_ctr.to_le_bytes());
        b.extend_from_slice(&c.target_robot.to_le_bytes());
        b.extend_from_slice(&c.fuse.to_le_bytes());
        b.extend_from_slice(&c.facing.to_le_bytes());
    }
    b
}

/// The effect-rows blob (T3 row, W12-S8; EXD alias 0x9d534 —
/// D162, §5i): u32 count + the
/// fixed rows {age u16, id u16, x, y, z, cos, sin, ttl} — 28 B
/// per row (the E-modeled subset of the 0x20-stride guest row).
fn effect_rows_blob(rs: &[bedlam_core::critter::EffectRow]) -> Vec<u8> {
    let mut b = Vec::with_capacity(4 + rs.len() * 28);
    b.extend_from_slice(&(rs.len() as u32).to_le_bytes());
    for r in rs {
        b.extend_from_slice(&r.age.to_le_bytes());
        b.extend_from_slice(&r.id.to_le_bytes());
        b.extend_from_slice(&r.x.to_le_bytes());
        b.extend_from_slice(&r.y.to_le_bytes());
        b.extend_from_slice(&r.z.to_le_bytes());
        b.extend_from_slice(&r.cos.to_le_bytes());
        b.extend_from_slice(&r.sin.to_le_bytes());
        b.extend_from_slice(&r.ttl.to_le_bytes());
    }
    b
}

/// Emit one canonical frame record: the §6a rows whose tier the
/// scenario captures (`tiers` = the scenario's tier list; `anchor`
/// adds the TS rows — they ride the mission-start frame only).
/// Registry-unknown ids are impossible here (the ids are literals
/// below); `encode_dump` re-validates and orders them anyway.
pub fn emit_frame(st: &TickState, tiers: &[String], injected: bool, anchor: bool) -> FrameRecord {
    let mut f = FrameRecord::new(st.frame_no, injected);
    let want = |tier: &str| tiers.iter().any(|t| t == tier);
    if want("T0") {
        f.push_watch("frame-counter", u32b(st.frame_no as u32));
        f.push_watch("rng-state-a", st.rand_a_state.to_le_bytes());
        f.push_watch("rng-state-b", st.rand_b_state.to_le_bytes());
        f.push_watch("score", u32b(st.score as u32));
        f.push_watch("money", u32b(st.money as u32));
        f.push_watch("difficulty", u32b(st.difficulty));
        f.push_watch("zone", u32b(st.zone));
        f.push_watch("mission", u32b(st.mission));
        f.push_watch("mode", u32b(st.mode));
        f.push_watch("linear-mission-m", u32b(st.linear));
        // The SFX master gate (D136): E has no audio config model —
        // the row carries the engine's sound-on construction
        // constant 1 (every dispatch the gate guards is
        // presentation-tier; §0's state-only scope). A capture
        // machine with sound DISABLED dumps 0 here — the intended
        // loud finding, the D134 fingerprint companion (the D128
        // ACTIONPAN pattern).
        f.push_watch("sfx-master-gate", u32b(1));
    }
    if want("T1") {
        f.push_watch("robot-bank", robot_bank_blob(st.robots));
        // The 4-byte alias form (the D83 anti-fabrication precedent).
        f.push_watch("selection-triple", u32b(st.selected as u32));
        f.push_watch("blink-cursor", u32b(st.blink_cursor as u32));
        let mut players = Vec::with_capacity(48);
        let sel = st.robots.get(st.selected);
        for p in 0..4 {
            let r = if p == 0 { sel } else { None };
            let (x, y, z) = match r {
                Some(r) => (r.pos_x >> 8, r.pos_y >> 8, r.z),
                None => (0, 0, 0),
            };
            players.extend_from_slice(&x.to_le_bytes());
            players.extend_from_slice(&y.to_le_bytes());
            players.extend_from_slice(&z.to_le_bytes());
        }
        f.push_watch("per-player-selected", players);
        let mut target = Vec::with_capacity(12);
        for v in [st.order_target.0, st.order_target.1, st.order_target.2] {
            target.extend_from_slice(&v.to_le_bytes());
        }
        f.push_watch("order-target", target);
        let mut moves = Vec::with_capacity(4 + st.robots.len() * 9);
        moves.extend_from_slice(&(st.robots.len() as u32).to_le_bytes());
        for r in st.robots {
            match r.target {
                None => {
                    moves.push(0);
                    moves.extend_from_slice(&0i32.to_le_bytes());
                    moves.extend_from_slice(&0i32.to_le_bytes());
                }
                Some((tx, ty)) => {
                    moves.push(1);
                    moves.extend_from_slice(&tx.to_le_bytes());
                    moves.extend_from_slice(&ty.to_le_bytes());
                }
            }
        }
        f.push_watch("move-target-words", moves);
        let mut beacon = Vec::with_capacity(20);
        // The beacon-family row (0x4eabb0..b8): while armed, the
        // live order; post-deploy, FUN_0041faf0 clears ONLY the
        // flag/window pair — the tile words survive in the latch
        // (§7j.40/4; the click-seam scenarios keep the all-zero
        // clear — their latch is None).
        let (b_flag, b_window, b_tile) = match st.order {
            None => (0u32, 0u32, st.beacon_latch.unwrap_or((0, 0, 0))),
            Some(o) => (1u32, u32::from(o.window), o.tile),
        };
        beacon.extend_from_slice(&b_flag.to_le_bytes());
        beacon.extend_from_slice(&b_window.to_le_bytes());
        beacon.extend_from_slice(&b_tile.0.to_le_bytes());
        beacon.extend_from_slice(&b_tile.1.to_le_bytes());
        beacon.extend_from_slice(&b_tile.2.to_le_bytes());
        f.push_watch("beacon-family", beacon);
        let mut claims = Vec::with_capacity(24);
        let live_claims = st.order.map(|o| o.claims).unwrap_or(st.claims_latch);
        for c in live_claims {
            claims.extend_from_slice(&u16::from(c).to_le_bytes());
        }
        f.push_watch("spread-claims", claims);
        // The no-extract latch (D133/D136): the per-robot CLAIMED
        // flags — MP-lobby-set only, never on any SP path; E's SP
        // corpus construction is the all-zero bank (the guest boot
        // memset twin). Canonical = u32 count (the robot-bank count
        // — the O1 plan dumps the same count-driven $robot_count*4
        // span) + count zero words.
        let mut latch = Vec::with_capacity(4 + st.robots.len() * 4);
        latch.extend_from_slice(&(st.robots.len() as u32).to_le_bytes());
        latch.resize(4 + st.robots.len() * 4, 0);
        f.push_watch("no-extract-latch", latch);
        let mut pads = Vec::with_capacity(4 + st.armor_pads.len());
        pads.extend_from_slice(&(st.armor_pads.len() as u32).to_le_bytes());
        pads.extend_from_slice(st.armor_pads);
        f.push_watch("typedb-fade-byte", pads.clone());
        f.push_watch("armor-pad-reads", pads);
    }
    if want("T2") {
        f.push_watch("weapon-anim-bank", weapon_bank_blob(st.weapon_bank));
        f.push_watch("projectile-bank", enemy_bank_blob(st.enemy_bank));
    }
    // The destroy-family rows (W12-S4): gated on the scenario's
    // `destroy = 1` staging key — S0..S3 carry no destroy rows and
    // their pinned bytes stay untouched. The T1 rows ride the T1
    // tier, the debris/splash T3 rows the T3 tier (DESIGN §7 S4
    // row tiers T0/T1/T3).
    if let Some(d) = &st.destroy {
        if want("T1") {
            f.push_watch("object-instances", objects_blob(d.objects));
            f.push_watch("trt-array", structures_blob(d.structures));
            f.push_watch("tile-word-grid", grid_blob(d.tile_grid));
            f.push_watch("platform-strength", grid_blob(d.platform));
            f.push_watch(
                "typedb-mirror-rows",
                mirror_blob(d.mirror_words, d.mirror_seen),
            );
        }
        if want("T3") {
            f.push_watch("debris-stager", debris_blob(d.debris));
            f.push_watch("splash-records", splash_blob(d.splashes));
        }
    }
    // The extraction dropship row (W12-S6): the T3 craft record
    // 0x4e6610. EXD alias 0x1081c4 pinned (D162, §5i) and the O1
    // normalizer arm landed with it (D164 — the full-record
    // identity form: the canonical record IS the guest 0x1C craft
    // record field-for-field), so the row compares cross-channel.
    // Gated on the scenario's pad-step presence.
    if let Some(c) = &st.dropship {
        if want("T3") {
            let mut b = Vec::with_capacity(28);
            b.extend_from_slice(&u32::from(c.active).to_le_bytes());
            b.extend_from_slice(&c.phase.to_le_bytes());
            b.extend_from_slice(&c.x.to_le_bytes());
            b.extend_from_slice(&c.y.to_le_bytes());
            b.extend_from_slice(&c.alt.to_le_bytes());
            b.extend_from_slice(&c.group.to_le_bytes());
            b.extend_from_slice(&c.dwell.to_le_bytes());
            f.push_watch("dropship-frame", b);
        }
    }
    // The critter-family rows (W12-S8): the T2 critter bank and
    // the T3 effect-row bank are both EXD-aliased (D162, §5i) and
    // cross-channel compared since the subset-form O1 arms landed.
    // Gated on the scenario's `critters = 1` key — S0..S7 pinned
    // bytes carry neither row.
    if let Some(v) = &st.critter {
        if want("T2") {
            f.push_watch("critter-bank", critter_bank_blob(v.critters));
        }
        if want("T3") {
            f.push_watch("effect-rows", effect_rows_blob(v.effect_rows));
        }
    }
    if anchor && want("TS") {
        let (w, h) = st.map_wh.unwrap_or((0, 0));
        let mut wh = Vec::with_capacity(8);
        wh.extend_from_slice(&w.to_le_bytes());
        wh.extend_from_slice(&h.to_le_bytes());
        f.push_watch("static-map-wh", wh);
        // The tile-claim bank (S0-11b, §7j.63): the raw arena image
        // at mission load — the SAME fixed 10000-B span the O1 plan
        // dumps through the 0x119564 pointer cell (indirect,
        // extent 10000) and O2 through 0x46af58, so no count
        // prefix, no field map: byte passthrough on every channel
        // (the D136 static-map-wh fixed-extent precedent).
        f.push_watch("static-claim-bank", st.claim_bank.to_vec());
        // The player TYPE word (S0-16, §7j.68/D159): the raw u16 LE
        // of the sim cell — the SAME 2 bytes the O1 plan dumps at
        // CS:001075C0 and O2 at 0x004EDB90 (fresh-SP 00 00, pinned
        // both channels), so byte passthrough on every channel (the
        // D136 static-map-wh precedent; the cell is dword-written
        // but word-consumed — extent 2 is the consumed word).
        let mut pt = Vec::with_capacity(2);
        pt.extend_from_slice(&st.player_type.to_le_bytes());
        f.push_watch("static-player-type", pt);
    }
    f
}

// ---------------------------------------------------------------------
// The runner (scenario → engine frames → stitched dump)
// ---------------------------------------------------------------------

/// Canonical-run failures (scenario shape, engine seams, IO).
#[derive(Debug)]
pub struct CanonicalError(pub String);

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "canonical: {}", self.0)
    }
}

impl std::error::Error for CanonicalError {}

impl From<StitchError> for CanonicalError {
    fn from(e: StitchError) -> Self {
        CanonicalError(format!("stitch: {e}"))
    }
}

impl From<GameError> for CanonicalError {
    fn from(e: GameError) -> Self {
        CanonicalError(format!("game: {e}"))
    }
}

/// Filesystem byte source for a canonical run: the install tree plus
/// the mission/graphics subtrees the asset names assume (the
/// `EDITOR\` / `GAMEGFX\` prefixes RE-EXW names carry as path halves).
pub struct MissionSource {
    root: PathBuf,
}

impl MissionSource {
    pub fn new(root: impl AsRef<Path>) -> MissionSource {
        MissionSource {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl ByteSource for MissionSource {
    fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
        for prefix in ["", "EDITOR/", "GAMEGFX/", "SOUND/MIDI/"] {
            let path = self.root.join(prefix).join(name);
            if path.is_file() {
                return std::fs::read(&path).map_err(|e| GameError::AssetMissing {
                    name: format!("{name}: {e}"),
                });
            }
        }
        Err(GameError::AssetMissing {
            name: name.to_string(),
        })
    }
}

/// The pinned scan→InputFrame map. EMPTY in W6: the engine has no
/// mission keyboard consumer yet (the P2e button bit-map assignment;
/// RE-EXW-INPUT line 95). Keystroke steps still mark their frame
/// injected; the bit-map lands here when P2e does.
fn scan_input(_entries: &[(u8, u8)]) -> InputFrame {
    InputFrame::default()
}

/// Session context fixed at run start (the boot + episode scalars).
struct Session {
    difficulty: u32,
    zone: u32,
    mission: u32,
    /// The DERIVED linear-mission-m cell [0x46ae8c] (not the episode
    /// progress counter — §7j.64/D, S0-12b/D154).
    linear: u32,
}

/// The linear-mission-m derivation (§7j.64/D, EXW 0x41c520..0x41c556):
/// `m = clamp(5*(zone-2) + mission - 1, floor 1, cap 26)` — GameMain
/// recomputes the cell from the CURRENT zone/mission slot every
/// episode (3 writes: the store 0x41c534, the cap 0x41c53e, the
/// floor 0x41c550); it is never a persisted counter. `zone` is the
/// 1-based guest set (mission_slot()'s 0-based index + 1, the D108
/// zone-row convention), `mission` the 1-based within-zone number.
/// Fresh slot (1,1): 5*(-1)+1-1 = -5 → the floor → 1.
pub fn linear_mission_m(zone: u32, mission: u32) -> u32 {
    (5 * (zone as i32 - 2) + mission as i32 - 1).clamp(1, 26) as u32
}

/// Run one scenario canonically and stitch the channel-E W3 dump.
///
/// The scenario must be walk-phase-empty (E-gaps the menu-walk seam)
/// and may carry only `step`/`keystore`/`order` mission steps
/// (`command`/`pad` name their missing engine seams). Returns the
/// same [`Stitched`] shape the O1 stitcher produces (bytes +
/// manifest); callers write the dump under runtime/ (§3 hygiene).
pub fn run_canonical(scenario_src: &str, root: &Path) -> Result<Stitched, CanonicalError> {
    let scen = Scenario::parse(scenario_src).map_err(|e| CanonicalError(format!("{e}")))?;
    let (walk, mission) = scen.phases();
    // The extraction view flag: any pad step in the mission schedule
    // turns on the T3 dropship row (W12-S6).
    let extraction = mission.iter().any(|s| matches!(s, Step::Pad { .. }));
    // Walk phase: ONLY Boot steps are consumable (the difficulty seed
    // below). Any other walk step (keystore/order/command/pad — the
    // S0W menu-walk shape) names the missing seam: the E walk waits on
    // the P2e InputFrame button bit-map. The grammar itself pins Boot
    // to the walk phase (parser rejects post-anchor boot), so the
    // `unreachable!` in the mission loop below is parser-guaranteed.
    for step in walk {
        if !matches!(step, Step::Boot { .. }) {
            return Err(CanonicalError(
                "walk-phase steps have no E-side seam yet (the P2e InputFrame button \
                 bit-map assignment); run the scenario on the O1 channel"
                    .into(),
            ));
        }
    }
    // BOOT (walk-phase-only by grammar): an explicit `boot difficulty=d`
    // step OVERRIDES the fresh-session default; the default is the
    // GameMain boot-head write DIFFICULTY := 1 (§7j.64/A, 0x41c14a —
    // S0-12b/D154; the pre-seam default 0 mis-modeled the fresh boot).
    let mut boot_difficulty: Option<u32> = None;
    for step in walk {
        if let Step::Boot { key, value } = step {
            debug_assert_eq!(key, "difficulty", "grammar pins the boot key set");
            boot_difficulty = Some(u32::try_from(*value).unwrap_or(0));
        }
    }
    let difficulty = boot_difficulty.unwrap_or(1);

    let mut source = MissionSource::new(root);
    let config = GameConfig::load(&mut source)?;
    let palette = [[0u8, 0, 0]; 256];
    let mut host = GameHost::new(&config, &SimConfig::default(), palette);

    // The episode-slot zone staging (W12-S5, grammar v1.5 `zone`,
    // D108; grammar v1.8 `mission`, the P5 per-zone disposition
    // family): the host seam stands in for the campaign-advance
    // (0x41c9e5) / save-load-restore (0x43c2b8) shells the engine
    // does not model. Letter A..G → stage 1..7; mask 0 → MISSION1.
    // The v1.8 `mission` key selects the within-zone mission:
    // 1..=5 stage the CAMPAIGN slot at the completion mask whose
    // first-uncompleted sub selects exactly that mission (mask
    // (1<<(m−1))−1: m=1 → 0, m=2 → 0b0001, … m=5 → 0b1111 —
    // `mission_number_for_mask` inverts it); 6..=7 stage the SELECT
    // screen's MP write pair INSTEAD of the campaign slot (the
    // §7j.73 sibling seam: the MP-only files no stage mask can
    // express, and campaign staging would CLEAR the pair — the pair
    // alone carries the load, zone cell = the letter's 1-based
    // value 2..=6, mission cell m−5 the +5 offset inverts). The
    // emitted linear row no longer reads the slot's progress
    // counter — it is the DERIVED cell (see `linear_mission_m`
    // below, §7j.64/D, S0-12b/D154; the D108 "linear stays the
    // fresh-slot 0" note superseded). Must run BEFORE the asset
    // fetch (the zone drives the names + the robots-per-player
    // count).
    if let Some(letter) = scen.zone {
        let stage = u8::try_from(u32::from(letter) - u32::from(b'A') + 1)
            .expect("grammar pins the zone letter to A..G");
        match scen.mission {
            Some(m) if m >= 6 => {
                if !host.stage_select_mission(stage, m - 5) {
                    return Err(CanonicalError(format!(
                        "zone {letter} mission {m} is outside the SELECT write-pair domain \
                         (the grammar pins zones B..F for missions 6..7)"
                    )));
                }
            }
            campaign => {
                let mask = match campaign {
                    Some(m) => (1u8 << (m - 1)) - 1,
                    None => 0,
                };
                if !host.stage_episode_slot(stage, mask) {
                    return Err(CanonicalError(format!(
                        "zone {letter} mission {:?} maps to stage {stage}/mask {mask} outside \
                         the campaign slot table",
                        scen.mission
                    )));
                }
            }
        }
    }

    // Stage the mission from the episode slot's asset names. The
    // fetch-order mapping below is pinned by suffix asserts (anti-drift).
    let names = host.mission_asset_names();
    let pins: [(usize, &str); 19] = [
        (0, ".TOT"),
        (1, ".DAT"),
        (2, ".PAD"),
        (3, ".CGR"),
        (4, ".BIN"),
        (5, ".LNK"),
        (6, "SINTABLE.BIN"),
        (7, "DANTE.BIN"),
        (8, "GAMEPAL.PAL"),
        (9, "GENERAL.BIN"),
        (10, "SMLFONT.BIN"),
        (11, ".MRK"),
        (12, "TABLE.BIN"),
        (13, "MAPTRAN0.TRN"),
        (20, "MAPTRAN7.TRN"),
        (21, ".MIN"),
        (22, "NUMBERS.BIN"),
        (23, "FLAGS.BIN"),
        (24, "BLOWUP.BIN"),
    ];
    if names.len() != 25 {
        return Err(CanonicalError(format!(
            "expected 25 mission asset names, got {}",
            names.len()
        )));
    }
    for (idx, suffix) in pins {
        if !names[idx].ends_with(suffix) {
            return Err(CanonicalError(format!(
                "asset name {idx} drift: {:?} does not end in {suffix:?}",
                names[idx]
            )));
        }
    }
    let bytes: Vec<Vec<u8>> = names
        .iter()
        .map(|n| source.load(n))
        .collect::<Result<_, _>>()?;
    let maptran: Vec<&[u8]> = bytes[13..21].iter().map(Vec::as_slice).collect();
    host.load_mission(
        &bytes[0],
        &bytes[1],
        &bytes[2],
        &bytes[3],
        &bytes[4],
        &bytes[5],
        &bytes[6],
        &bytes[7],
        &bytes[8],
        &bytes[9],
        &bytes[10],
        &bytes[11],
        &bytes[23],
        &bytes[24],
        &bytes[12],
        &maptran,
        &bytes[21],
        &bytes[22],
        None,
        &scen.markers,
    )?;
    // The campaign seed runs on EVERY run (§7j.64/C, S0-12b/D154):
    // the name-entry fresh-campaign arm 0x43aaa3..0x43aad0 writes
    // money := 4000−500·d at every campaign start — the default
    // difficulty 1 seeds 3500 on an untouched-toggle fresh boot (the
    // pre-seam gate skipped the seed at d=0, the mis-modeled
    // default). `start_score` IS the 0x43aaca formula.
    {
        let money = bedlam_game::menu::start_score(difficulty as u8);
        let scene = host.mission_mut().expect("mission staged");
        scene.set_campaign(0, money);
        // The difficulty dword 0x46cbf8 seeds the sim's
        // difficulty-scaled damage rows (§7j.15/2).
        scene.sim_mut().set_difficulty(difficulty);
    }

    // The destroy-family staging (W12-S4, grammar v1.4 `destroy = 1`):
    // the mission's OWN .BDG/.POS/.TRT staged through the engine host
    // seam (`stage_destroy_family`, the D51 pattern). The ORIGINAL
    // loads all three natively at mission load (FUN_0041a4f8 +
    // FUN_004170a6, §7j.25/4) — E's `load_mission` does not fetch
    // them, so this key stages CONTENT identical to what O1 loads:
    // no O1 write, an equivalence seam recorded in the plan. Fail
    // loud on any malformed file — never guess. The key also gates
    // the destroy dump rows (S0..S3 bytes unchanged).
    if scen.destroy {
        let (zone_idx, mission_no) = host.mission_slot();
        let zone_dir = format!("ZONE{}", (b'A' + zone_idx as u8) as char);
        let per_mission = format!("{zone_dir}/MISSION{mission_no}");
        let bdg_bytes = source
            .load(&format!("{per_mission}.BDG"))
            .map_err(|e| CanonicalError(format!("destroy staging: {e}")))?;
        let pos_bytes = source
            .load(&format!("{per_mission}.POS"))
            .map_err(|e| CanonicalError(format!("destroy staging: {e}")))?;
        let trt_bytes = source
            .load(&format!("{per_mission}.TRT"))
            .map_err(|e| CanonicalError(format!("destroy staging: {e}")))?;
        let table = ObjectTypeTable::from_bdg_bytes(&bdg_bytes).ok_or_else(|| {
            CanonicalError(format!(
                "{per_mission}.BDG desynced (the FORMATS §16 grammar)"
            ))
        })?;
        if pos_bytes.len() != 16 * bedlam_core::destroy::OBJECT_INSTANCE_SLOTS {
            return Err(CanonicalError(format!(
                "{per_mission}.POS is {} B (want 16*2000, FORMATS §12)",
                pos_bytes.len()
            )));
        }
        // The TRT hp tier selector is the DERIVED cell [0x46ae8c]
        // (§7j.64/D lists the 250+250·m/27 formula among its readers;
        // S0-12b/D154 — the pre-seam `episode().linear()` counter was
        // the wrong source; fresh/staged slots now carry m from the
        // same derivation as the emitted row).
        let (zone_idx_for_m, mission_for_m) = host.mission_slot();
        let linear = linear_mission_m(zone_idx_for_m as u32 + 1, mission_for_m as u32);
        if bedlam_core::destroy::parse_trt(&trt_bytes, linear).is_none() {
            return Err(CanonicalError(format!(
                "{per_mission}.TRT desynced (the FORMATS §14 grammar)"
            )));
        }
        // [0x4edd8c] = zone_index + 1 (D99); the mirror banks stage
        // EMPTY (the init_tiles TOT fill is the S5 pairing) — the
        // restore writes land on the empty banks, faithfully.
        let zone_set = u32::try_from(zone_idx + 1).unwrap_or(1);
        let scene = host.mission_mut().expect("mission staged");
        if !scene
            .sim_mut()
            .stage_destroy_family(&table, &pos_bytes, &trt_bytes, zone_set, linear)
        {
            return Err(CanonicalError(
                "destroy staging rejected (terrain not sized / POS length)".into(),
            ));
        }
        // The within-zone mission number [0x4edd88] — the zone-3
        // bridge-trigger sub-dispatch index (§7j.41/1).
        scene.sim_mut().set_mission_no(mission_no as u32);
    }

    // The pickup-surface staging (W12-S5, grammar v1.5 `pickup = 1`,
    // D108): the mission's OWN .TOT — `bytes[0]`, the same volume the
    // terrain staged from — through the init_tiles host seam
    // (`stage_pickup_surface`), AFTER any destroy staging above (the
    // destroy staging RESETS the mirror banks — the engine
    // load-order note in its doc comment), then the §7j.12/6 hazard
    // stamper (the original's mission-load order: footprint stamp →
    // init_tiles → hazards). The ORIGINAL stages the same TOT volume
    // natively at mission load (FUN_00407e11), so the content is
    // identical on both channels — an EQUIVALENCE seam like `destroy`,
    // recorded in the plan; it is a separate key because S4's
    // empty-staged mirror bytes are chain-pinned. The set cell =
    // zone_index+1 (D99).
    if scen.pickup {
        let (zone_idx, _) = host.mission_slot();
        let zone_set = u32::try_from(zone_idx + 1).unwrap_or(1);
        let scene = host.mission_mut().expect("mission staged");
        if !scene.sim_mut().stage_pickup_surface(&bytes[0], zone_set) {
            return Err(CanonicalError(
                "pickup staging rejected (the .TOT volume desynced from the staged \
                 terrain — dims or size mismatch)"
                    .into(),
            ));
        }
        scene.sim_mut().stamp_hazard_words();
    }

    // The platform-family arm (W12-S7, grammar v1.6 `platforms = 1`,
    // D113): arm the epilogue creep tick (FUN_00422a9c, the
    // MissionShell epilogue call 0x44808a). The ORIGINAL runs the
    // tick EVERY frame from boot (its 1/32 gate draws one RandA per
    // frame); E arms it per scenario so the S0..S6 chains stay
    // byte-identical — the per-frame gate draw on unarmed paths is
    // the recorded E-gap (§7j.41/4). Purely an E-side arming
    // decision: nothing is staged on the guest (O1 runs the tick
    // natively); consumers record the key in `_e_staging`.
    if scen.platforms {
        let scene = host.mission_mut().expect("mission staged");
        scene.sim_mut().arm_platform_family();
    }

    // The critter-family staging + arm (W12-S8, grammar v1.7
    // `critters = 1`, D114): the mission's .NME through the
    // FUN_00416458 spawn schedule (`stage_critters` — the §7j.18
    // grammar, difficulty-scaled), then ARM the controller
    // (FUN_00412f34, MissionShell 0x447fe1). The ORIGINAL loads
    // .NME natively at EVERY mission load and runs the controller
    // UNGATED; E arms it per scenario so the S0..S7 chains stay
    // byte-identical (the loader's kind-4 heading draws + the
    // controller's per-frame draws on unarmed paths are the
    // recorded E-side stream gap, §7j.42/5). The critter bank is
    // the E-ONLY T2 coverage row (no EXD alias); the ALIASED
    // observables are the RNG stream, the robot bank (the
    // damage/stun lanes), the projectile bank (the 0x68 fire
    // cycle), the debris/effect-row stagings, and the score
    // bounty. A file hosting an unmodeled kind REFUSES (fail
    // loud — never spawn a brain the engine does not carry).
    if scen.critters {
        let (zone_idx, mission_no) = host.mission_slot();
        let zone_dir = format!("ZONE{}", (b'A' + zone_idx as u8) as char);
        let nme_bytes = source
            .load(&format!("{zone_dir}/MISSION{mission_no}.NME"))
            .map_err(|e| CanonicalError(format!("critter staging: {e}")))?;
        let scene = host.mission_mut().expect("mission staged");
        if scene
            .sim_mut()
            .stage_critters(&nme_bytes, difficulty)
            .is_none()
        {
            return Err(CanonicalError(
                "critter staging rejected (the .NME hosts a kind the E controller \
                 does not model — §7j.42/6)"
                    .into(),
            ));
        }
        scene.sim_mut().arm_critter_family();
    }

    // The LOADOUT staging seam (W12-S3, grammar v1.3): expand the
    // per-robot entries into the 7-slot arrays through the engine
    // host seam (`stage_robot_weapons`, the D51 pattern — the
    // original fills the slots at spawn from the session table).
    // Like `markers`, this is an E-side staging seam: no O1 write
    // exists, the plan records it. An index past the bank is a
    // scenario error (fail loud, never guess).
    for l in &scen.loadout {
        let mut slots = [WeaponSlot::default(); 7];
        for (k, &(id, ammo)) in l.slots.iter().enumerate().take(7) {
            slots[k] = WeaponSlot {
                id,
                ammo,
                cooldown: 0,
            };
        }
        let scene = host.mission_mut().expect("mission staged");
        if !scene.sim_mut().stage_robot_weapons(l.robot, slots, l.mask) {
            let n = scene.sim().robots().len();
            return Err(CanonicalError(format!(
                "loadout robot {} is not in the bank ({} robots staged; MRK \
                 robots then markers order)",
                l.robot, n
            )));
        }
    }

    // Session scalars from the episode (fresh host: ZONEA/MISSION1).
    let (zone, mission_no) = host.mission_slot();
    let session = Session {
        difficulty,
        zone: zone as u32,
        mission: mission_no as u32,
        // The linear-mission-m row = the DERIVED cell, recomputed
        // from the CURRENT slot (§7j.64/D, 0x41c520..0x41c556;
        // S0-12b/D154) — never the episode progress counter.
        linear: linear_mission_m(zone as u32 + 1, mission_no as u32),
    };

    // Boot hold → Title → Brief → Select → Mission, then the
    // activation frame (the mission is INERT during it; sync_mission
    // activates at the pump tail).
    let null = InputFrame::default();
    let mut guard = 0u32;
    while host.scene() == bedlam_game::Scene::Boot {
        let executed = host.pump_frame(DT_SUBTICKS, &null);
        check_cadence(executed)?;
        guard += 1;
        if guard > 600 {
            return Err(CanonicalError("boot hold never ended".into()));
        }
    }
    host.apply(SceneAction::Advance); // Title -> Brief
    host.apply(SceneAction::Advance); // Brief -> Select
    host.apply(SceneAction::Advance); // Select -> Mission
    if host.scene() != bedlam_game::Scene::Mission {
        return Err(CanonicalError(format!(
            "FSM did not reach Mission (at {:?})",
            host.scene()
        )));
    }
    let executed = host.pump_frame(DT_SUBTICKS, &null); // activation frame
    check_cadence(executed)?;

    // The anchor record: the tail of the FIRST mission tick (TS rides).
    let executed = host.pump_frame(DT_SUBTICKS, &null);
    check_cadence(executed)?;
    let map_wh = host
        .mission()
        .map(|m| m.view_size())
        .map(|(w, h)| (w as u32, h as u32));
    let mut frames = vec![emit_frame(
        &tick_state(
            &host,
            &session,
            (0, 0, 0),
            0,
            map_wh,
            scen.destroy,
            extraction,
            scen.critters,
        ),
        &scen.tiers,
        false,
        true,
    )];

    // Mission phase: one boundary per frame; injections apply BEFORE
    // the tick (§5: between the previous present and this input read).
    let total = scen.frames + 1; // the stitcher contract
    let mut seam_target = (0i32, 0i32, 0i32);
    'outer: for step in mission {
        match *step {
            Step::Advance { frames: n } => {
                for _ in 0..n {
                    if frames.len() >= total as usize {
                        break 'outer;
                    }
                    let executed = host.pump_frame(DT_SUBTICKS, &null);
                    check_cadence(executed)?;
                    let no = frame_counter_now(&host);
                    frames.push(emit_frame(
                        &tick_state(
                            &host,
                            &session,
                            seam_target,
                            no,
                            None,
                            scen.destroy,
                            extraction,
                            scen.critters,
                        ),
                        &scen.tiers,
                        false,
                        false,
                    ));
                }
            }
            Step::Keystore { ref entries } => {
                if frames.len() >= total as usize {
                    break;
                }
                for (scan, _) in entries {
                    if BANNED_SCANS.contains(scan) {
                        return Err(CanonicalError(format!(
                            "keystore scan 0x{scan:02x} (P-pause) is banned mid-scenario \
                             (DESIGN §2)"
                        )));
                    }
                }
                let input = scan_input(entries);
                let executed = host.pump_frame(DT_SUBTICKS, &input);
                check_cadence(executed)?;
                let no = frame_counter_now(&host);
                frames.push(emit_frame(
                    &tick_state(
                        &host,
                        &session,
                        seam_target,
                        no,
                        None,
                        scen.destroy,
                        extraction,
                        scen.critters,
                    ),
                    &scen.tiers,
                    true,
                    false,
                ));
            }
            Step::Order { x, y, z } => {
                if frames.len() >= total as usize {
                    break;
                }
                seam_target = (x, y, z);
                if let Some(scene) = host.mission_mut() {
                    let pick = scene
                        .sim()
                        .robots()
                        .iter()
                        .position(|r| r.alive && r.tile() == (x, y));
                    if let Some(idx) = pick {
                        scene.sim_mut().arm_order_at_robot(idx);
                    }
                    // No robot at the tile: the pick fails (no arm),
                    // the target is still recorded — the seam write.
                }
                let executed = host.pump_frame(DT_SUBTICKS, &null);
                check_cadence(executed)?;
                let no = frame_counter_now(&host);
                frames.push(emit_frame(
                    &tick_state(
                        &host,
                        &session,
                        seam_target,
                        no,
                        None,
                        scen.destroy,
                        extraction,
                        scen.critters,
                    ),
                    &scen.tiers,
                    true,
                    false,
                ));
            }
            Step::Command { ref bytes } => {
                if frames.len() >= total as usize {
                    break;
                }
                // The W5 seam consumed (W12-S3-prep, §10-W12): the
                // payload bytes are the COMMAND record the O1 capgen
                // appends at the ring; E stages them into the sim's
                // ring and the next frame's MissionShell pass
                // consumes them (FUN_00409138 — the fire family of
                // DESIGN §7's S3 row). The bit1 consumer writes the
                // record's triple to the 0x4dd484 cells — they
                // PERSIST like the ORDER seam's write, so the
                // order-target row mirrors the last of either.
                let scene = host
                    .mission_mut()
                    .ok_or_else(|| CanonicalError("mission not staged".into()))?;
                if !scene.sim_mut().stage_command(bytes) {
                    return Err(CanonicalError(format!(
                        "command payload too short (<14 B record): {bytes:02x?}"
                    )));
                }
                if let Some(rec) = CommandRecord::from_payload(bytes) {
                    if rec.flags & 2 != 0 {
                        seam_target = (i32::from(rec.x), i32::from(rec.y), i32::from(rec.z));
                    }
                }
                let executed = host.pump_frame(DT_SUBTICKS, &null);
                check_cadence(executed)?;
                let no = frame_counter_now(&host);
                frames.push(emit_frame(
                    &tick_state(
                        &host,
                        &session,
                        seam_target,
                        no,
                        None,
                        scen.destroy,
                        extraction,
                        scen.critters,
                    ),
                    &scen.tiers,
                    true,
                    false,
                ));
            }
            Step::Pad { slot } => {
                if frames.len() >= total as usize {
                    break;
                }
                // The W5 pad op CONSUMED (W12-S6, §7j.40): the target
                // tile is READ from the staged .PAD slot bank at run
                // time (the D86 capgen contract — bank order = file
                // record order; a slot the mission never loaded is a
                // capture error naming the slot). The op writes ONLY
                // the order-target seam triple — the robot's arrival
                // arms extraction in-game (FUN_00433980 →
                // FUN_004247b5). The step also stages the zone set
                // cell (the pad-script dispatcher keys on
                // [0x4edd8c] = zone_index+1, D99).
                let scene = host
                    .mission_mut()
                    .ok_or_else(|| CanonicalError("mission not staged".into()))?;
                let Some((px, py, pz)) = scene.sim().terrain.pad_slot(slot as usize) else {
                    return Err(CanonicalError(format!(
                        "pad slot {slot} is not in the staged mission's .PAD bank (active!=1 \
                         or the 0xFFFF terminator) — a capture error naming the slot"
                    )));
                };
                seam_target = (px, py, pz);
                scene.sim_mut().stage_zone_set(session.zone + 1);
                let executed = host.pump_frame(DT_SUBTICKS, &null);
                check_cadence(executed)?;
                let no = frame_counter_now(&host);
                frames.push(emit_frame(
                    &tick_state(
                        &host,
                        &session,
                        seam_target,
                        no,
                        None,
                        scen.destroy,
                        extraction,
                        scen.critters,
                    ),
                    &scen.tiers,
                    true,
                    false,
                ));
            }
            Step::Capture | Step::UntilAnchor { .. } => {} // runner directives
            Step::Boot { .. } => unreachable!("grammar pins boot to the walk phase"),
        }
    }
    // Past the step schedule the input stays null (§5: with no
    // injection the original polls zeros) — the `frames` budget
    // governs the capture length, not the schedule (S1-style
    // scenarios carry no steps at all).
    while frames.len() < total as usize {
        let executed = host.pump_frame(DT_SUBTICKS, &null);
        check_cadence(executed)?;
        let no = frame_counter_now(&host);
        frames.push(emit_frame(
            &tick_state(
                &host,
                &session,
                seam_target,
                no,
                None,
                scen.destroy,
                extraction,
                scen.critters,
            ),
            &scen.tiers,
            false,
            false,
        ));
    }

    // Header: channel E, the engine identity as the build hash, the
    // determinism pins.
    let identity = format!("bedlam-game {}+canonical-1", env!("CARGO_PKG_VERSION"));
    let mut header = DumpHeader::new(
        Channel::Engine,
        sha256(identity.as_bytes()),
        scen.id.clone(),
    );
    header.push_pin("seed=0x1e240");
    header.push_pin(format!("dt_subticks={DT_SUBTICKS}"));
    header.push_pin(format!("difficulty={}", session.difficulty));
    header.push_pin(format!("zone={}", session.zone));
    header.push_pin(format!("mission={}", session.mission));
    header.push_pin("mode=sp");
    let reg: Vec<Watch> = diffharness::registry();
    let transcript = Transcript { frames };
    Ok(diffharness::runner::stitch(
        &scen,
        &transcript,
        &header,
        &reg,
    )?)
}

/// The pre-increment frame counter at the tail (`sim.frame()−1`).
fn frame_counter_now(host: &GameHost) -> u64 {
    host.mission().expect("mission staged").sim().frame() - 1
}

/// The per-frame emitter view of the live scene (§6a sources).
/// `destroy` gates the destroy-family surfaces (the scenario's
/// staging key — the rows ride only destroy scenarios);
/// `extraction` gates the dropship row (the pad-step flag).
#[allow(clippy::too_many_arguments)]
fn tick_state<'a>(
    host: &'a GameHost,
    session: &Session,
    seam_target: (i32, i32, i32),
    frame_no: u64,
    map_wh: Option<(u32, u32)>,
    destroy: bool,
    extraction: bool,
    critter: bool,
) -> TickState<'a> {
    let scene = host.mission().expect("mission staged");
    let sim = scene.sim();
    TickState {
        frame_no,
        rand_a_state: sim.rand_a_state(),
        rand_b_state: scene.rand_b_state(),
        score: scene.campaign().0,
        money: scene.campaign().1,
        difficulty: session.difficulty,
        zone: session.zone,
        mission: session.mission,
        mode: 0,
        linear: session.linear,
        robots: sim.robots(),
        order: sim.order(),
        beacon_latch: sim.beacon_tile_latch(),
        claims_latch: sim.beacon_claims_latch(),
        dropship: extraction.then_some(sim.dropship()),
        selected: scene.sidebar_selected(),
        blink_cursor: scene.sidebar_cursor(),
        order_target: seam_target,
        armor_pads: sim.armor_pads(),
        map_wh,
        claim_bank: sim.claim_bank(),
        player_type: sim.player_type(),
        weapon_bank: sim.weapon_bank(),
        enemy_bank: sim.enemy_bank(),
        critter: critter.then_some(CritterView {
            critters: sim.critters(),
            effect_rows: sim.effect_rows(),
        }),
        destroy: destroy.then(|| DestroyView {
            objects: sim.objects(),
            structures: sim.structures(),
            tile_grid: sim.object_grid(),
            platform: sim.platform_bank(),
            mirror_words: sim.mirror_words(),
            mirror_seen: sim.mirror_seen_bank(),
            debris: sim.debris_bank(),
            splashes: sim.splash_bank(),
        }),
    }
}

/// The canonical cadence contract: exactly one executed tick per
/// pumped frame at dt=4.
fn check_cadence(executed: u32) -> Result<(), CanonicalError> {
    if executed != 1 {
        return Err(CanonicalError(format!(
            "cadence break: {executed} ticks in one frame (want 1 at dt=4)"
        )));
    }
    Ok(())
}
