//! Behavioral emulator tests for RecoilEffect_ (engine/battle/move_effects/recoil.asm).
//!
//! RecoilEffect_ handles self-damage from recoil moves (Take Down, Double-Edge,
//! Submission) and Struggle. Recoil = wDamage / 4 for normal moves, wDamage / 2
//! for Struggle. Minimum recoil is 1. Recoil is subtracted from the user's HP,
//! flooring at 0 if recoil exceeds current HP.
//!
//! Test approach: step_to `.getHPBarCoords` label, which is after HP subtraction
//! (including the zero-floor path) but before display routines.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

const STRUGGLE: u8 = 0xA5;

fn setup_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("RecoilEffect_");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
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

fn run_recoil(whose_turn: u8, move_num: u8, current_hp: u16, max_hp: u16, damage: u16) -> u16 {
    let mut h = setup_fixture();

    h.write_mem(sym_addr("hWhoseTurn"), whose_turn);

    let wd = sym_addr("wDamage");
    write_be16(&mut h, wd, damage);

    if whose_turn == 0 {
        h.write_mem(sym_addr("wPlayerMoveNum"), move_num);
        let hp = sym_addr("wBattleMonHP");
        let mhp = sym_addr("wBattleMonMaxHP");
        write_be16(&mut h, hp, current_hp);
        write_be16(&mut h, mhp, max_hp);
    } else {
        h.write_mem(sym_addr("wEnemyMoveNum"), move_num);
        let hp = sym_addr("wEnemyMonHP");
        let mhp = sym_addr("wEnemyMonMaxHP");
        write_be16(&mut h, hp, current_hp);
        write_be16(&mut h, mhp, max_hp);
    }

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("RecoilEffect_"));
    h.step_to(sym_addr("RecoilEffect_.getHPBarCoords"));

    let hp_addr = if whose_turn == 0 {
        sym_addr("wBattleMonHP")
    } else {
        sym_addr("wEnemyMonHP")
    };
    read_be16(&mut h, hp_addr)
}

#[test]
fn normal_recoil_is_quarter_damage() {
    let hp = run_recoil(0x00, 0x24, 200, 200, 100);
    assert_eq!(hp, 175, "recoil = 100/4 = 25, HP = 200-25 = 175");
}

#[test]
fn struggle_recoil_is_half_damage() {
    let hp = run_recoil(0x00, STRUGGLE, 200, 200, 100);
    assert_eq!(hp, 150, "Struggle recoil = 100/2 = 50, HP = 200-50 = 150");
}

#[test]
fn minimum_recoil_is_1() {
    let hp = run_recoil(0x00, 0x24, 200, 200, 1);
    assert_eq!(hp, 199, "damage 1 / 4 = 0 → clamped to 1, HP = 200-1 = 199");
}

#[test]
fn struggle_minimum_recoil_is_1() {
    let hp = run_recoil(0x00, STRUGGLE, 200, 200, 1);
    assert_eq!(hp, 199, "Struggle: damage 1 / 2 = 0 → clamped to 1");
}

#[test]
fn recoil_floors_hp_at_zero() {
    let hp = run_recoil(0x00, 0x24, 10, 200, 200);
    assert_eq!(hp, 0, "recoil 50 > HP 10 → HP floored at 0");
}

#[test]
fn struggle_recoil_floors_at_zero() {
    let hp = run_recoil(0x00, STRUGGLE, 10, 200, 200);
    assert_eq!(hp, 0, "Struggle recoil 100 > HP 10 → HP floored at 0");
}

#[test]
fn exact_lethal_recoil() {
    let hp = run_recoil(0x00, 0x24, 25, 200, 100);
    assert_eq!(hp, 0, "recoil = 25, HP = 25 → 25-25 = 0");
}

#[test]
fn non_lethal_recoil() {
    let hp = run_recoil(0x00, 0x24, 26, 200, 100);
    assert_eq!(hp, 1, "recoil = 25, HP = 26 → 26-25 = 1");
}

#[test]
fn enemy_turn_affects_enemy_hp() {
    let hp = run_recoil(0x01, 0x24, 200, 200, 100);
    assert_eq!(hp, 175, "enemy's turn: enemy takes recoil = 25");
}

#[test]
fn enemy_turn_player_hp_unchanged() {
    let mut h = setup_fixture();

    h.write_mem(sym_addr("hWhoseTurn"), 0x01);
    write_be16(&mut h, sym_addr("wDamage"), 100);
    h.write_mem(sym_addr("wEnemyMoveNum"), 0x24);
    write_be16(&mut h, sym_addr("wEnemyMonHP"), 200);
    write_be16(&mut h, sym_addr("wEnemyMonMaxHP"), 200);
    write_be16(&mut h, sym_addr("wBattleMonHP"), 150);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("RecoilEffect_"));
    h.step_to(sym_addr("RecoilEffect_.getHPBarCoords"));

    let player_hp = read_be16(&mut h, sym_addr("wBattleMonHP"));
    assert_eq!(
        player_hp, 150,
        "player HP should be unchanged on enemy's turn"
    );
}

#[test]
fn large_damage_recoil() {
    let hp = run_recoil(0x00, 0x24, 500, 500, 400);
    assert_eq!(hp, 400, "recoil = 400/4 = 100, HP = 500-100 = 400");
}

#[test]
fn damage_3_recoil_rounds_to_0_then_clamped() {
    let hp = run_recoil(0x00, 0x24, 200, 200, 3);
    assert_eq!(hp, 199, "damage 3 / 4 = 0 → clamped to 1, HP = 199");
}
