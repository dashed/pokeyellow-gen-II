//! Behavioral emulator tests for HazeEffect_ (engine/battle/move_effects/haze.asm).
//!
//! HazeEffect_ resets all stat mods to 7 (neutral), copies unmodified stats back
//! to battle stats, cures non-volatile status (entire byte) for the TARGET only,
//! and clears volatile statuses (CONFUSED, X_ACCURACY, MIST, PUMPED, SEEDED,
//! Bad Poison, Reflect, Light Screen) for BOTH sides. Disabled moves are cleared
//! for both sides. TRANSFORMED bit and other BattleStatus1 bits are preserved.
//!
//! Test approach: step_to a computed address just before PlayCurrentMoveAnimation
//! call (after all state changes, before display routines).

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

const NUM_STAT_MODS: u16 = 8;

const CONFUSED: u8 = 7;
const USING_X_ACCURACY: u8 = 0;
const PROTECTED_BY_MIST: u8 = 1;
const GETTING_PUMPED: u8 = 2;
const HAS_SUBSTITUTE_UP: u8 = 4;
const NEEDS_TO_RECHARGE: u8 = 5;
const USING_RAGE: u8 = 6;
const SEEDED: u8 = 7;
const BADLY_POISONED: u8 = 0;
const HAS_LIGHT_SCREEN_UP: u8 = 1;
const HAS_REFLECT_UP: u8 = 2;
const TRANSFORMED: u8 = 3;

const FRZ_BIT: u8 = 5;
const BRN_BIT: u8 = 4;

fn setup_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("HazeEffect_");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h
}

fn animation_call_addr() -> u16 {
    sym_addr("HazeEffect_.cureVolatileStatuses") + 24
}

fn run_haze(whose_turn: u8) -> TestHarness {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), whose_turn);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));

    let stop = animation_call_addr();
    assert_eq!(
        h.read_mem(stop),
        0x21,
        "expected ld hl,nn opcode at animation call site"
    );
    h.step_to(stop);
    h
}

fn read_be16(h: &mut TestHarness, addr: u16) -> u16 {
    let hi = h.read_mem(addr) as u16;
    let lo = h.read_mem(addr + 1) as u16;
    (hi << 8) | lo
}

fn write_be16(h: &mut TestHarness, addr: u16, val: u16) {
    h.write_mem(addr, (val >> 8) as u8);
    h.write_mem(addr + 1, (val & 0xFF) as u8);
}

// ─── Stat mod reset ───────────────────────────────────────────────

#[test]
fn resets_player_stat_mods_to_7() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    let base = sym_addr("wPlayerMonAttackMod");
    for i in 0..NUM_STAT_MODS {
        h.write_mem(base + i, 13);
    }

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureStatuses"));

    for i in 0..NUM_STAT_MODS {
        assert_eq!(
            h.read_mem(base + i),
            7,
            "player stat mod byte {i} should be reset to 7"
        );
    }
}

#[test]
fn resets_enemy_stat_mods_to_7() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    let base = sym_addr("wEnemyMonAttackMod");
    for i in 0..NUM_STAT_MODS {
        h.write_mem(base + i, 1);
    }

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureStatuses"));

    for i in 0..NUM_STAT_MODS {
        assert_eq!(
            h.read_mem(base + i),
            7,
            "enemy stat mod byte {i} should be reset to 7"
        );
    }
}

// ─── Unmodified stats → battle stats copy ─────────────────────────

#[test]
fn copies_player_unmodified_stats_to_battle_stats() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    let unmod = sym_addr("wPlayerMonUnmodifiedAttack");
    let battle = sym_addr("wBattleMonAttack");
    let stats: [(u16, u16); 4] = [(0, 0), (2, 2), (4, 4), (6, 6)];
    let values: [u16; 4] = [120, 80, 95, 110];

    for (i, val) in values.iter().enumerate() {
        write_be16(&mut h, unmod + stats[i].0, *val);
        write_be16(&mut h, battle + stats[i].1, 0xFFFF);
    }

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureStatuses"));

    for (i, val) in values.iter().enumerate() {
        let actual = read_be16(&mut h, battle + stats[i].1);
        assert_eq!(
            actual, *val,
            "player battle stat {i} should be copied from unmodified"
        );
    }
}

#[test]
fn copies_enemy_unmodified_stats_to_battle_stats() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    let unmod = sym_addr("wEnemyMonUnmodifiedAttack");
    let battle = sym_addr("wEnemyMonAttack");

    for i in 0..8u16 {
        h.write_mem(unmod + i, 0x30 + i as u8);
        h.write_mem(battle + i, 0xFF);
    }

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureStatuses"));

    for i in 0..8u16 {
        assert_eq!(
            h.read_mem(battle + i),
            0x30 + i as u8,
            "enemy battle stat byte {i} should be copied"
        );
    }
}

// ─── Non-volatile status cure (target only) ───────────────────────

#[test]
fn player_turn_cures_enemy_freeze() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 1 << FRZ_BIT);
    h.write_mem(sym_addr("wEnemySelectedMove"), 0x00);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureVolatileStatuses"));

    assert_eq!(
        h.read_mem(sym_addr("wEnemyMonStatus")),
        0,
        "enemy freeze should be cured"
    );
    assert_eq!(
        h.read_mem(sym_addr("wEnemySelectedMove")),
        0xFF,
        "frozen target's selected move should be set to $FF"
    );
}

#[test]
fn player_turn_cures_enemy_sleep() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x03);
    h.write_mem(sym_addr("wEnemySelectedMove"), 0x00);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureVolatileStatuses"));

    assert_eq!(h.read_mem(sym_addr("wEnemyMonStatus")), 0);
    assert_eq!(h.read_mem(sym_addr("wEnemySelectedMove")), 0xFF);
}

#[test]
fn player_turn_does_not_cure_player_status() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wBattleMonStatus"), 1 << FRZ_BIT);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x00);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureVolatileStatuses"));

    assert_eq!(
        h.read_mem(sym_addr("wBattleMonStatus")),
        1 << FRZ_BIT,
        "player's own status should NOT be cured on player's turn"
    );
}

#[test]
fn enemy_turn_cures_player_status() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x01);
    h.write_mem(sym_addr("wBattleMonStatus"), 1 << FRZ_BIT);
    h.write_mem(sym_addr("wPlayerSelectedMove"), 0x00);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureVolatileStatuses"));

    assert_eq!(
        h.read_mem(sym_addr("wBattleMonStatus")),
        0,
        "player status should be cured when enemy uses Haze"
    );
    assert_eq!(h.read_mem(sym_addr("wPlayerSelectedMove")), 0xFF);
}

#[test]
fn burn_cured_but_no_move_prevention() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 1 << BRN_BIT);
    h.write_mem(sym_addr("wEnemySelectedMove"), 0x37);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(sym_addr("HazeEffect_.cureVolatileStatuses"));

    assert_eq!(
        h.read_mem(sym_addr("wEnemyMonStatus")),
        0,
        "burn should be cured"
    );
    assert_eq!(
        h.read_mem(sym_addr("wEnemySelectedMove")),
        0x37,
        "burn without freeze/sleep should NOT prevent action"
    );
}

// ─── Volatile status clearing (both sides) ────────────────────────

#[test]
fn clears_confused_both_sides() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x00);
    h.write_mem(sym_addr("wPlayerBattleStatus1"), 1 << CONFUSED | 0x04);
    h.write_mem(sym_addr("wEnemyBattleStatus1"), 1 << CONFUSED);

    let stop = animation_call_addr();
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(stop);

    let ps1 = h.read_mem(sym_addr("wPlayerBattleStatus1"));
    assert_eq!(
        ps1 & (1 << CONFUSED),
        0,
        "player CONFUSED should be cleared"
    );
    assert_eq!(ps1 & 0x04, 0x04, "other BattleStatus1 bits preserved");

    let es1 = h.read_mem(sym_addr("wEnemyBattleStatus1"));
    assert_eq!(es1 & (1 << CONFUSED), 0, "enemy CONFUSED should be cleared");
}

#[test]
fn clears_status2_volatile_bits() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x00);

    let all_volatile =
        (1 << USING_X_ACCURACY) | (1 << PROTECTED_BY_MIST) | (1 << GETTING_PUMPED) | (1 << SEEDED);
    let preserved = (1 << HAS_SUBSTITUTE_UP) | (1 << NEEDS_TO_RECHARGE) | (1 << USING_RAGE);
    h.write_mem(sym_addr("wPlayerBattleStatus2"), all_volatile | preserved);
    h.write_mem(sym_addr("wEnemyBattleStatus2"), all_volatile);

    let stop = animation_call_addr();
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(stop);

    let ps2 = h.read_mem(sym_addr("wPlayerBattleStatus2"));
    assert_eq!(
        ps2 & all_volatile,
        0,
        "player Status2 volatile bits should be cleared"
    );
    assert_eq!(
        ps2 & preserved,
        preserved,
        "player SUBSTITUTE/RECHARGE/RAGE should be preserved"
    );

    let es2 = h.read_mem(sym_addr("wEnemyBattleStatus2"));
    assert_eq!(es2 & all_volatile, 0, "enemy Status2 volatile bits cleared");
}

#[test]
fn clears_status3_volatile_bits_preserves_transformed() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x00);

    let volatile3 = (1 << BADLY_POISONED) | (1 << HAS_LIGHT_SCREEN_UP) | (1 << HAS_REFLECT_UP);
    h.write_mem(
        sym_addr("wPlayerBattleStatus3"),
        volatile3 | (1 << TRANSFORMED),
    );
    h.write_mem(sym_addr("wEnemyBattleStatus3"), volatile3);

    let stop = animation_call_addr();
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(stop);

    let ps3 = h.read_mem(sym_addr("wPlayerBattleStatus3"));
    assert_eq!(
        ps3 & volatile3,
        0,
        "player Status3 volatile bits should be cleared"
    );
    assert_eq!(
        ps3 & (1 << TRANSFORMED),
        1 << TRANSFORMED,
        "TRANSFORMED bit should be preserved"
    );

    let es3 = h.read_mem(sym_addr("wEnemyBattleStatus3"));
    assert_eq!(es3 & volatile3, 0, "enemy Status3 volatile bits cleared");
}

// ─── Disabled moves ───────────────────────────────────────────────

#[test]
fn clears_disabled_moves_both_sides() {
    let mut h = run_haze(0x00);

    assert_eq!(h.read_mem(sym_addr("wPlayerDisabledMove")), 0);
    assert_eq!(h.read_mem(sym_addr("wEnemyDisabledMove")), 0);
    assert_eq!(h.read_mem(sym_addr("wPlayerDisabledMoveNumber")), 0);
    assert_eq!(h.read_mem(sym_addr("wEnemyDisabledMoveNumber")), 0);
}

#[test]
fn clears_disabled_moves_from_nonzero() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x00);
    h.write_mem(sym_addr("wPlayerDisabledMove"), 0x2A);
    h.write_mem(sym_addr("wEnemyDisabledMove"), 0x3B);
    h.write_mem(sym_addr("wPlayerDisabledMoveNumber"), 0x02);
    h.write_mem(sym_addr("wEnemyDisabledMoveNumber"), 0x01);

    let stop = animation_call_addr();
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("HazeEffect_"));
    h.step_to(stop);

    assert_eq!(
        h.read_mem(sym_addr("wPlayerDisabledMove")),
        0,
        "player disabled move should be cleared"
    );
    assert_eq!(
        h.read_mem(sym_addr("wEnemyDisabledMove")),
        0,
        "enemy disabled move should be cleared"
    );
    assert_eq!(h.read_mem(sym_addr("wPlayerDisabledMoveNumber")), 0);
    assert_eq!(h.read_mem(sym_addr("wEnemyDisabledMoveNumber")), 0);
}
