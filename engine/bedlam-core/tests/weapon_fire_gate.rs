//! The W12-S3-prep weapon-fire gate: the COMMAND-record consumer +
//! the two projectile banks + the per-type tick of
//! `bedlam_core::weapon` [RE-EXW-SIM §7j.37 — every behavioral
//! assertion below cites the verified decode].
//!
//! The NO-INJECT INVARIANT is the unit's first-class constraint: with
//! no staged COMMAND records `advance_frame` must not draw RandA,
//! touch a bank, or move a robot differently than the pre-S3 engine
//! (the corpus-gated S0/S1/S2 canonical chains pin that end-to-end;
//! here it is asserted on the rng stream + the banks).

use bedlam_core::mission::{AngleTable, MissionSim, Terrain, STATE_ORDERED};
use bedlam_core::weapon::{
    weapon_damage, CommandRecord, EnemyProjectile, WeaponRecord, WeaponSlot, ENEMY_BANK_SLOTS,
    WEAPON_BANK_SLOTS,
};

/// Plane 0 solid (type 1), planes 1..7 empty — every floor settles
/// to level 0. The 16×16 sub-tile block at each tile's origin corner
/// carries height byte 0x10 (the floor-contact fixture: a projectile
/// whose z>>8 < 0x10 over those sub-tiles reads floor 0x10 > z>>8).
fn flat_terrain(w: i32, h: i32) -> Terrain {
    let n = (w * h) as usize;
    let mut dat = vec![0u8; 8 * n];
    for b in dat.iter_mut().take(n) {
        *b = 1;
    }
    let mut heights = [0u8; 1024];
    for sy in 0..16 {
        for sx in 0..16 {
            heights[sy * 32 + sx] = 0x10;
        }
    }
    Terrain::from_parts(w, h, dat, vec![heights]).unwrap()
}

/// SINTABLE-shaped angles: the full 256-word sine ramp + the
/// words[2..66] thresholds (the real file's dual-use shape, §7j.37/2).
fn sintable_angles() -> AngleTable {
    let mut words = [0i16; 256];
    for (a, w) in words.iter_mut().enumerate() {
        let v = ((a as f64) * std::f64::consts::PI / 128.0).sin() * 32767.0;
        *w = v.round() as i16;
    }
    // words[2..66] of the real ramp already form the ascending
    // quarter-sine thresholds — no overwrite needed.
    AngleTable::from_sintable_words(&words).unwrap()
}

fn sim() -> MissionSim {
    let mut s = MissionSim::new(flat_terrain(32, 32), sintable_angles(), 0x1E240);
    s.spawn_robot((16, 16, 1));
    s
}

fn slot(id: u16, ammo: i16) -> WeaponSlot {
    WeaponSlot {
        id,
        ammo,
        cooldown: 0,
    }
}

/// A bit1 fire record for robot 0 aimed at a target tile.
fn fire_record(flags: u8, x: i16, y: i16, z: i16) -> CommandRecord {
    CommandRecord {
        marker: 1,
        id: 0,
        spot: 0,
        flags,
        x,
        y,
        z,
    }
}

/// Active weapon-bank records.
fn active(sim: &MissionSim) -> Vec<&WeaponRecord> {
    sim.weapon_bank().iter().filter(|r| r.active()).collect()
}

// ---------------------------------------------------------------------
// The damage table (FUN_00419aff, §7j.15/1)
// ---------------------------------------------------------------------

#[test]
fn damage_table_pinned_rows() {
    assert_eq!(weapon_damage(2, 0), 20);
    assert_eq!(weapon_damage(3, 0), 30);
    assert_eq!(weapon_damage(4, 0), 40);
    assert_eq!(weapon_damage(5, 0), 75);
    assert_eq!(weapon_damage(0xC, 0), 5000);
    assert_eq!(weapon_damage(0xD, 0), 312);
    assert_eq!(weapon_damage(0x1A, 0), 75);
    assert_eq!(weapon_damage(0x24, 0), 400);
    assert_eq!(weapon_damage(0x29, 0), 250);
    // Everything else = 1 (the cosmetic ballistic set).
    assert_eq!(weapon_damage(0xE, 0), 1);
    assert_eq!(weapon_damage(0xF, 2), 1);
    assert_eq!(weapon_damage(0x69, 0), 1); // the open 0x69 question
}

#[test]
fn damage_table_difficulty_rows_linear_then_flat_d2() {
    // 0x65: (d+1)·50, d=2 flat 200.
    assert_eq!(weapon_damage(0x65, 0), 50);
    assert_eq!(weapon_damage(0x65, 1), 100);
    assert_eq!(weapon_damage(0x65, 2), 200);
    // 0x66: (d+1)·300, d=2 flat 1200.
    assert_eq!(weapon_damage(0x66, 0), 300);
    assert_eq!(weapon_damage(0x66, 1), 600);
    assert_eq!(weapon_damage(0x66, 2), 1200);
    // 0x67/0x68: (d+1)·75, d=2 flat 300.
    assert_eq!(weapon_damage(0x67, 0), 75);
    assert_eq!(weapon_damage(0x68, 1), 150);
    assert_eq!(weapon_damage(0x67, 2), 300);
    assert_eq!(weapon_damage(0x68, 2), 300);
}

// ---------------------------------------------------------------------
// The payload grammar + the no-inject invariant
// ---------------------------------------------------------------------

#[test]
fn command_payload_parses_14_byte_fields() {
    // The ORIGINAL grammar (re-verified vs the FUN_00409138 decompile,
    // ghidra-project/exw-robottarget.txt:74-113): byte@+6 is the
    // builder FILLER the consumer never reads; the words live at
    // +7/+9/+0xB. The filler byte 0x0A here proves the offsets (the
    // pre-S3 transcription read x at +6 and would see 0x0A15).
    let payload = [
        0x01, 0x00, 0x00, 0x0A, 0x00, 0x02, 0x0A, 0x15, 0x00, 0x49, 0x00, 0x01, 0x00, 0x00,
    ];
    let rec = CommandRecord::from_payload(&payload).expect("parses");
    assert_eq!(rec.marker, 0x01);
    assert_eq!(rec.id, 0);
    assert_eq!(rec.spot, 10);
    assert_eq!(rec.flags, 0x02);
    assert_eq!(rec.x, 21);
    assert_eq!(rec.y, 73);
    assert_eq!(rec.z, 1);
    assert!(CommandRecord::from_payload(&payload[..13]).is_none());
}

#[test]
fn no_inject_path_is_inert() {
    // 50 frames with a live robot: no RandA draws, no bank traffic,
    // no order-flag writes (the S0/S1/S2 chain guarantee, asserted
    // at the sources the chains hash: the rng stream + the banks).
    let mut s = sim();
    let rng_before = s.rand_a_state();
    let hash_before = s.state_hash();
    for _ in 0..50 {
        s.advance_frame();
    }
    assert_eq!(s.rand_a_state(), rng_before, "no draws without commands");
    assert!(s.weapon_bank().iter().all(|r| !r.active()));
    assert!(s.enemy_bank().iter().all(|r| r.kind == 0));
    assert!(!s.command_order_active());
    assert_ne!(s.state_hash(), hash_before); // the frame counter moves
                                             // Determinism: two runs identical (the bank fields are excluded
                                             // from the hash but must not perturb the hashed set either).
    let mut a = sim();
    let mut b = sim();
    for _ in 0..10 {
        a.advance_frame();
        b.advance_frame();
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

// ---------------------------------------------------------------------
// The consumer: flags + fire gates + bookkeeping (§7j.37/1)
// ---------------------------------------------------------------------

#[test]
fn bit0_select_writes_target_and_auto_arms() {
    let mut s = sim();
    s.robots_mut()[0].state = 0;
    s.stage_command_record(fire_record(1, 21 << 5, 73 << 5, 0));
    s.consume_commands();
    let r = &s.robots()[0];
    assert_eq!(r.target, Some((21 << 5, 73 << 5)), "raw Q5 word target");
    assert_eq!(r.state, 1, "auto-arm state := 1");
    assert_eq!(r.stop_dist, 1_000_000);
    // State 3/4/5 robots take no target; state 2..5 no auto-arm.
    let mut s = sim();
    s.robots_mut()[0].state = STATE_ORDERED;
    s.stage_command_record(fire_record(1, 21 << 5, 73 << 5, 0));
    s.consume_commands();
    let r = &s.robots()[0];
    assert_eq!(r.target, None);
    assert_eq!(r.state, STATE_ORDERED);
}

#[test]
fn bit1_order_writes_the_triple_and_flag() {
    let mut s = sim();
    s.stage_command_record(fire_record(2, 21, 73, 1));
    s.consume_commands();
    assert_eq!(s.command_order_target(), (21, 73, 1));
    assert!(s.command_order_active());
}

#[test]
fn fire_gates_mask_cooldown_ammo() {
    // Masked-off, cooling, and dry slots never fire [verified].
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(9, 5),
            slot(9, 5),
            slot(9, 5),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        0b0000_1000, // only slot 3 armed — but slot 3 is EMPTY
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    assert!(active(&s).is_empty(), "empty armed slot fires nothing");

    let mut s = sim();
    let cooling = WeaponSlot {
        id: 9,
        ammo: 5,
        cooldown: 3,
    };
    s.stage_robot_weapons(
        0,
        [
            cooling,
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    assert!(active(&s).is_empty(), "cooling slot fires nothing");
    // The recharge pass decrements it (enabled ∧ cooldown≠0).
    assert_eq!(s.robots()[0].weapons[0].cooldown, 2);
}

#[test]
fn artillery_spawns_one_record_and_disarms() {
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(9, 3),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    let (px, py, pz) = (s.robots()[0].pos_x, s.robots()[0].pos_y, s.robots()[0].z);
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    let act = active(&s);
    assert_eq!(act.len(), 1, "w9 spawns exactly 1 record");
    let rec = act[0];
    assert_eq!(rec.kind, 9, "type = the weapon id");
    assert_eq!(rec.x, px + 0x100);
    assert_eq!(rec.y, py + 0x100);
    assert_eq!(rec.z, (pz + 0x15) * 0x100);
    assert_eq!(rec.tick, 0);
    assert_eq!((rec.vx, rec.vy, rec.vz), (0, 0, 0), "no velocity — falls");
    let r = &s.robots()[0];
    assert_eq!(r.weapons[0].ammo, 2);
    assert_eq!(r.weapons[0].cooldown, 0);
    // The unconditional disarm emptied the mask, then the AUTO-REARM
    // set slot 0's bit back (first slot with id≠0 ∧ ammo≠0 — ammo
    // remains). The observable sequence is the rearm outcome.
    assert_eq!(r.weapon_mask, 1, "disarm, then auto-rearm re-arms slot 0");
}

#[test]
fn mines_spawn_burst_with_pinned_shape() {
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(0x10, 9),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    let act = active(&s);
    assert_eq!(act.len(), 2, "prox mine X2 spawns 2");
    for rec in act {
        assert_eq!(rec.kind, 0xF);
        assert_eq!(rec.class, 4, "mines are class 4");
        assert!((1..=16).contains(&rec.tick), "ttl = RandA&0xF+1");
        assert!(
            (0x900 - 0x2FF..=0x900).contains(&rec.arc),
            "arc = 0x900 − RandA&0x2FF"
        );
        // vel>>2: the 16-tile aim keeps both axes nonzero.
        assert_ne!(rec.vx, 0);
        assert_ne!(rec.vy, 0);
    }
    let r = &s.robots()[0];
    assert_eq!(r.weapons[0].ammo, 7);
    // Cooldown 8 at fire, then the loop-exit recharge pass of the
    // SAME consume call decrements it (the original's order).
    assert_eq!(r.weapons[0].cooldown, 7);
    assert_eq!(r.weapon_mask, 1, "mask bit kept while ammo remains");
    // Pressure mines X6 → 6 records type 0x13, vel>>1 (faster).
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(0x16, 9),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    let act = active(&s);
    assert_eq!(act.len(), 6);
    assert!(act.iter().all(|r| r.kind == 0x13 && r.class == 4));
}

#[test]
fn grenades_spawn_3d_bursts_with_ttl_ranges() {
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(0x1B, 9),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 1));
    s.consume_commands();
    let act = active(&s);
    assert_eq!(act.len(), 4, "bouncy X4");
    for rec in act {
        assert_eq!(rec.kind, 0x1A);
        assert_eq!(rec.class, 0);
        assert_eq!(rec.trail, 0, "the +0x32 := 0 write");
        assert!(
            (0x32 - 0xF..=0x32).contains(&rec.tick),
            "ttl 0x32 − RandA&0xF"
        );
        assert!(
            (0xB00 - 0x2FF..=0xB00).contains(&rec.arc),
            "arc 0xB00 − RandA&0x2FF"
        );
        assert_ne!(rec.vz, 0, "3D velocity from the order z");
    }
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(0x1E, 9),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 1));
    s.consume_commands();
    let act = active(&s);
    assert_eq!(act.len(), 6, "sticky X6");
    assert!(act
        .iter()
        .all(|r| r.kind == 0x1F && (0x32..=0x32 + 0xF).contains(&r.tick)));
}

#[test]
fn rocket_spawns_straight_with_angle_arc() {
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(0x20, 9),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    let act = active(&s);
    assert_eq!(act.len(), 1);
    let rec = act[0];
    assert_eq!(rec.kind, 0x24);
    assert_eq!(rec.tick, 0);
    // The robot holds the +0xF00 sub-tile spawn offset, so the
    // tile-exact target is NORTH-EAST: a small N..E sector heading
    // (the angle pair over the Q13 delta).
    let h = rec.arc & 0xFF;
    assert!(h < 0x40, "arc = the angle-pair heading: {h:#x}");
    let r = &s.robots()[0];
    assert_eq!(
        r.weapons[0].cooldown, 4,
        "cooldown 5 at fire, −1 at the same-pass recharge"
    );
    assert_eq!(r.weapon_mask, 1);
}

#[test]
fn family_weapons_route_without_spawning() {
    // w2 needler: dispatched to FUN_0040b615 (count 3) — the spawn
    // body is the E-gap; the bookkeeping runs [hypothesis].
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(2, 5),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    assert!(active(&s).is_empty(), "family internals are an E-gap");
    let r = &s.robots()[0];
    assert_eq!(r.weapons[0].ammo, 4);
    assert_eq!(
        r.weapons[0].cooldown, 7,
        "8 at fire, −1 at the same-pass recharge"
    );
    assert_eq!(r.weapon_mask, 1, "ammo remains — bit kept");
}

#[test]
fn auto_rearm_picks_first_loaded_slot() {
    // Slot 0 artillery (1 ammo, disarms per shot) + slot 1 needler
    // (5 ammo, NOT armed): after the pass the mask re-arms slot 1.
    let mut s = sim();
    s.stage_robot_weapons(
        0,
        [
            slot(9, 1),
            slot(2, 5),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    let r = &s.robots()[0];
    assert_eq!(r.weapon_mask, 0b10, "re-armed slot 1 (the loaded slot)");
    // …and slot 1 fires on the NEXT record (cooldown 0 on it).
    s.stage_command_record(fire_record(2, 20 * 0x20, 16 * 0x20, 0));
    s.consume_commands();
    assert_eq!(
        s.robots()[0].weapons[1].ammo,
        4,
        "slot 1 fired its family pass"
    );
}

// ---------------------------------------------------------------------
// The weapon-anim tick (§7j.37 items 3..6)
// ---------------------------------------------------------------------

#[test]
fn bullets_commit_one_step_per_call_and_survive_impacts() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 2,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000, // level 1, above the level-0 floor
        vx: 0x100,
        vy: 0,
        vz: 0,
        tick: 0,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    let rec = &s.weapon_bank()[0];
    // The decompile loop: 3 moves (the 3rd ends the pass) − 1
    // rollback = net TWO committed steps; tick += 2 per move = 6.
    assert_eq!(rec.x, 16 * 0x2000 + 2 * 0x100, "net two steps per call");
    assert_eq!(rec.tick, 6, "tick += 2 per move (3 moves)");
    s.weapon_tick(1);
    s.weapon_tick(2);
    assert_eq!(s.weapon_bank()[0].x, 16 * 0x2000 + 6 * 0x100);
}

#[test]
fn bullets_free_only_at_ttl() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 3,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000,
        vx: 0,
        vy: 0,
        vz: 0,
        tick: 96,
        ..WeaponRecord::default()
    };
    // tick 96 → sub-steps 98, 100 → 99 < 100 → BOUNDS free.
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].kind, 0);
}

#[test]
fn bullets_survive_the_floor_and_keep_flying() {
    // A bullet below the floor surface: terrain result re-adds the
    // step but the record does NOT free [verified §7j.37/3].
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 4,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x100, // z>>8 = 1 < the level-0 floor 0x20
        vx: 0x100,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    let rec = &s.weapon_bank()[0];
    assert_ne!(rec.kind, 0, "impacts do not kill bullets");
    assert_eq!(rec.x, 16 * 0x2000 + 0x100);
}

#[test]
fn shell_moves_and_frees_on_floor() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 5,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000,
        vx: 0x200,
        vy: 0,
        vz: 0,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].x, 16 * 0x2000 + 0x200);
    assert_eq!(s.weapon_bank()[0].tick, 1);
    // Drive it into the floor: z below the surface.
    s.weapon_bank_mut()[0].z = 0x100;
    s.weapon_tick(1);
    assert_eq!(s.weapon_bank()[0].kind, 0, "floor impact frees the shell");
}

#[test]
fn artillery_falls_then_settles_then_frees() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 9,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x4000, // two levels up
        ..WeaponRecord::default()
    };
    // Phase != 0: no tick.
    s.weapon_tick(1);
    assert_eq!(s.weapon_bank()[0].tick, 0);
    // Phase 0: fall 0x200, tick++.
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].z, 0x4000 - 0x200);
    assert_eq!(s.weapon_bank()[0].tick, 1);
    // Settle: at the floor (level-0 floor 0 → z clamps at 0).
    for _ in 0..40 {
        s.weapon_tick(0);
    }
    let rec = &s.weapon_bank()[0];
    assert_eq!(rec.z, 0x10 << 8, "settled ONTO the floor (floor<<8)");
    // Past the burst window (tick − 0x20 ≥ 2 for w9) → free.
    assert_eq!(rec.kind, 0);
}

#[test]
fn ballistic_gravity_and_damped_floor_roll() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 0xF,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x9000,
        vx: 0,
        vy: 0,
        arc: 0x300,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    let rec = &s.weapon_bank()[0];
    assert_eq!(rec.arc, 0x200, "gravity −0x100 per call");
    assert_eq!(rec.z, 0x9000 + 0x200, "z += arc after the decrement");
    // Floor contact over the raised block (floor 0x10 > new_z>>8),
    // approached from above so the horizontal move crosses no wall:
    // damped roll vx>>=1, vy>>=1, arc>>=2 (no sign flip).
    s.weapon_bank_mut()[0].z = 0x2000;
    s.weapon_bank_mut()[0].arc = -0x1800;
    s.weapon_bank_mut()[0].vx = 0x400;
    s.weapon_bank_mut()[0].vy = 0x200;
    s.weapon_tick(0);
    let rec = &s.weapon_bank()[0];
    assert_eq!(rec.vx, 0x200);
    assert_eq!(rec.vy, 0x100);
    assert_eq!(rec.arc, -0x1900 >> 2, "arc>>=2 on the decremented arc");
}

#[test]
fn ballistic_1a_floor_bounce_damps_arc() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 0x1A,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000,
        vx: 0x400,
        vy: 0,
        arc: -0x1800,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    let rec = &s.weapon_bank()[0];
    // arc: −0x1900 (gravity) → floor contact: flip + damp
    // (−arc − (−arc>>1)) → 0x1900 − 0xC80 = 0xC80; vx halved.
    assert_eq!(rec.arc, 0xC80);
    assert_eq!(rec.vx, 0x200);
}

#[test]
fn ballistic_17_floor_contact_splits_three_clones() {
    let mut s = sim();
    // The parent staged at the LAST slot: the clones land at slots
    // 0..2 (below the walk cursor) and stay pristine this pass.
    s.weapon_bank_mut()[WEAPON_BANK_SLOTS - 1] = WeaponRecord {
        kind: 0x17,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000,
        vx: 0x400,
        vy: 0x200,
        arc: -0x1800,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    let act = active(&s);
    assert_eq!(act.len(), 4, "parent + 3 clones");
    let parent = &s.weapon_bank()[WEAPON_BANK_SLOTS - 1];
    // Parent: arc flipped + damped (0x1900 − 0xC80), velocities the
    // v − v>>1 forms (0x200, 0x100), tick +2 (split + common tail).
    assert_eq!(parent.arc, 0xC80);
    assert_eq!(parent.vx, 0x200);
    assert_eq!(parent.vy, 0x100);
    assert_eq!(parent.tick, 2);
    // The clones: rotations of (svy, −sx)-form over the damped pair
    // (vx−vx>>1, vy−vy>>1) = (0x200, 0x100) → (0x100,−0x200),
    // (−0x100,0x200), (−0x200,−0x100) [verified decompile].
    let clones: Vec<(i32, i32)> = s.weapon_bank()[..3].iter().map(|r| (r.vx, r.vy)).collect();
    assert!(clones.contains(&(0x100, -0x200)));
    assert!(clones.contains(&(-0x100, 0x200)));
    assert!(clones.contains(&(-0x200, -0x100)));
    assert!(s.weapon_bank()[..3]
        .iter()
        .all(|r| r.kind == 0x17 && r.class == 0));
}

#[test]
fn ballistic_expiry_cycles_class_and_frees_at_zero() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 0xF,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000,
        class: 2,
        tick: 0x65,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    let rec = &s.weapon_bank()[0];
    assert_eq!(rec.tick, 0, "the 0xF/0x13 cycle resets the tick");
    assert_eq!(rec.class, 1, "class decrements at expiry");
    assert_ne!(rec.kind, 0);
    s.weapon_bank_mut()[0].tick = 0x65;
    s.weapon_bank_mut()[0].class = 1;
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].class, 0);
    // class 0 at the NEXT expiry frees (the quadrant detonation is
    // the S4 application E-gap).
    s.weapon_bank_mut()[0].tick = 0x65;
    s.weapon_bank_mut()[0].class = 0;
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].kind, 0);
}

#[test]
fn rocket_launch_delay_then_flight() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 0x24,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000,
        vx: 0x100,
        class: 2,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].x, 16 * 0x2000, "held during the delay");
    assert_eq!(s.weapon_bank()[0].class, 1);
    s.weapon_tick(0);
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].class, 0);
    s.weapon_tick(0);
    assert_eq!(
        s.weapon_bank()[0].x,
        16 * 0x2000 + 2 * 0x100,
        "flies at class 0 (two flying calls so far)"
    );
}

#[test]
fn homing_steers_toward_the_robot_target() {
    let mut s = sim();
    // Robot 0 at (16,16); a homing missile west of it, heading N.
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 0x29,
        x: 10 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000,
        arc: 0x00,          // heading N
        target: 0x1000 | 1, // robot class, robot idx 0 (1-based)
        class: 0,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    let rec = &s.weapon_bank()[0];
    // Target due EAST → the heading turns toward 0x40 (the ±4 clamp
    // bounds the per-call turn).
    assert!(
        rec.arc > 0x00 && rec.arc <= 0x40,
        "heading steered east: {:#x}",
        rec.arc
    );
    assert!(rec.vx > 0, "velocity gained an eastward component");
    // Dead target → fizzle.
    s.robots_mut()[0].alive = false;
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].kind, 0);
}

#[test]
fn homing_launch_delay_holds() {
    let mut s = sim();
    s.weapon_bank_mut()[0] = WeaponRecord {
        kind: 0x29,
        x: 10 * 0x2000,
        y: 16 * 0x2000,
        z: 0x2000,
        target: 0x1000 | 1,
        class: 3,
        ..WeaponRecord::default()
    };
    s.weapon_tick(0);
    assert_eq!(s.weapon_bank()[0].x, 10 * 0x2000, "held during the delay");
    assert_eq!(s.weapon_bank()[0].class, 2);
}

// ---------------------------------------------------------------------
// The 50×0x22 projectile tick (FUN_00412010, 7j.13/5)
// ---------------------------------------------------------------------

#[test]
fn enemy_projectiles_move_and_free_on_terrain() {
    let mut s = sim();
    assert!(s
        .stage_enemy_projectile(EnemyProjectile {
            kind: 0x66,
            x: 16 * 0x2000,
            y: 16 * 0x2000,
            z: 0x2000,
            vx: 0x200,
            vy: 0,
            vz: 0,
        })
        .is_some());
    s.enemy_tick();
    assert_eq!(s.enemy_bank()[0].x, 16 * 0x2000 + 0x200);
    // Terrain hit: 0x66 deactivates, 0x65 does not [7j.13/5].
    s.enemy_bank_mut()[0].z = 0x100;
    s.enemy_tick();
    assert_eq!(s.enemy_bank()[0].kind, 0);
    let s2 = s.stage_enemy_projectile(EnemyProjectile {
        kind: 0x65,
        x: 16 * 0x2000,
        y: 16 * 0x2000,
        z: 0x100,
        vx: 0,
        vy: 0,
        vz: 0,
    });
    assert!(s2.is_some());
    s.enemy_tick();
    assert_ne!(s.enemy_bank()[s2.unwrap()].kind, 0, "0x65 survives terrain");
    // Bounds exit deactivates.
    s.enemy_bank_mut()[s2.unwrap()].vx = 0x100000;
    s.enemy_tick();
    assert_eq!(s.enemy_bank()[s2.unwrap()].kind, 0);
}

// ---------------------------------------------------------------------
// Integration: the frame wiring (MissionShell order)
// ---------------------------------------------------------------------

#[test]
fn frame_pipeline_consumes_and_ticks() {
    // One advance_frame: consume (recharge included) → 6 robot
    // phases → 4× the enemy pass. A staged artillery record falls
    // 4·0x200 per frame (phase-0 ticks once, so 0x200/frame) and the
    // rocket moves 4× per frame once flying.
    let mut s = sim();
    assert!(s.stage_command(&[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x80, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00,
    ]));
    assert_eq!(s.pending_commands(), 1);
    s.stage_robot_weapons(
        0,
        [
            slot(9, 1),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
            WeaponSlot::default(),
        ],
        1,
    );
    s.advance_frame();
    assert_eq!(s.pending_commands(), 0, "the frame consumed the ring");
    let act = active(&s);
    assert_eq!(act.len(), 1);
    // The spawn z + one phase-0 fall of 0x200.
    let pz = s.robots()[0].z;
    let expect_z = (pz + 0x15) * 0x100 - 0x200;
    assert_eq!(act[0].z, expect_z);
}

#[test]
fn bank_capacity_is_the_original_pin() {
    let s = sim();
    assert_eq!(s.weapon_bank().len(), WEAPON_BANK_SLOTS);
    assert_eq!(s.enemy_bank().len(), ENEMY_BANK_SLOTS);
}
