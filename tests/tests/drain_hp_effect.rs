//! Behavioral emulator tests for DrainHPEffect_ (engine/battle/move_effects/drain_hp.asm).
//!
//! DrainHPEffect_ handles HP-draining moves (Absorb, Mega Drain, Leech Life,
//! Dream Eater). Recovery = wDamage / 2, minimum 1. Recovered HP is added to the
//! attacker's current HP, capped at max HP.
//!
//! Test approach: step_to `.next` label, which is after the HP update math but
//! before display routines (UpdateHPBar2, DrawPlayerHUDAndHPBar, PrintText).

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

fn setup_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("DrainHPEffect_");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h
}

struct DrainResult {
    final_hp: u16,
    recovery: u16,
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

fn run_drain(whose_turn: u8, current_hp: u16, max_hp: u16, damage: u16) -> DrainResult {
    let mut h = setup_fixture();

    h.write_mem(sym_addr("hWhoseTurn"), whose_turn);

    let wd = sym_addr("wDamage");
    write_be16(&mut h, wd, damage);

    let (hp_addr, max_hp_addr) = if whose_turn == 0 {
        (sym_addr("wBattleMonHP"), sym_addr("wBattleMonMaxHP"))
    } else {
        (sym_addr("wEnemyMonHP"), sym_addr("wEnemyMonMaxHP"))
    };
    write_be16(&mut h, hp_addr, current_hp);
    write_be16(&mut h, max_hp_addr, max_hp);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("DrainHPEffect_"));
    h.step_to(sym_addr("DrainHPEffect_.next"));

    let final_hp = read_be16(&mut h, hp_addr);
    let recovery = read_be16(&mut h, wd);

    DrainResult { final_hp, recovery }
}

#[test]
fn recovery_is_half_damage() {
    let r = run_drain(0x00, 50, 200, 40);
    assert_eq!(r.recovery, 20, "recovery should be damage/2 = 20");
    assert_eq!(r.final_hp, 70, "HP should be 50 + 20 = 70");
}

#[test]
fn minimum_recovery_is_1_from_damage_1() {
    let r = run_drain(0x00, 50, 200, 1);
    assert_eq!(r.recovery, 1, "damage 1 / 2 rounds to 0, clamped to 1");
    assert_eq!(r.final_hp, 51, "HP should increase by 1");
}

#[test]
fn zero_damage_gives_recovery_1() {
    let r = run_drain(0x00, 50, 200, 0);
    assert_eq!(r.recovery, 1, "damage 0 / 2 = 0, clamped to 1");
    assert_eq!(r.final_hp, 51);
}

#[test]
fn hp_caps_at_max_hp() {
    let r = run_drain(0x00, 90, 100, 40);
    assert_eq!(r.recovery, 20, "recovery should be 20");
    assert_eq!(r.final_hp, 100, "HP 90 + 20 = 110 → capped at 100");
}

#[test]
fn exact_max_hp_not_capped() {
    let r = run_drain(0x00, 80, 100, 40);
    assert_eq!(
        r.final_hp, 100,
        "HP 80 + 20 = 100 → exactly max, no cap needed"
    );
}

#[test]
fn full_hp_stays_at_max() {
    let r = run_drain(0x00, 100, 100, 40);
    assert_eq!(r.final_hp, 100, "already at max HP → capped at 100");
}

#[test]
fn large_damage_recovery() {
    let r = run_drain(0x00, 100, 500, 400);
    assert_eq!(r.recovery, 200, "damage 400 / 2 = 200");
    assert_eq!(r.final_hp, 300, "HP 100 + 200 = 300");
}

#[test]
fn odd_damage_rounds_down() {
    let r = run_drain(0x00, 50, 200, 3);
    assert_eq!(r.recovery, 1, "damage 3 / 2 = 1 (integer division)");
    assert_eq!(r.final_hp, 51);
}

#[test]
fn even_damage_exact_half() {
    let r = run_drain(0x00, 50, 200, 100);
    assert_eq!(r.recovery, 50, "damage 100 / 2 = 50");
    assert_eq!(r.final_hp, 100);
}

#[test]
fn enemy_turn_recovers_enemy_hp() {
    let mut h = setup_fixture();

    h.write_mem(sym_addr("hWhoseTurn"), 0x01);

    let wd = sym_addr("wDamage");
    write_be16(&mut h, wd, 40);

    let enemy_hp = sym_addr("wEnemyMonHP");
    let enemy_max = sym_addr("wEnemyMonMaxHP");
    write_be16(&mut h, enemy_hp, 50);
    write_be16(&mut h, enemy_max, 200);

    let player_hp = sym_addr("wBattleMonHP");
    write_be16(&mut h, player_hp, 80);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("DrainHPEffect_"));
    h.step_to(sym_addr("DrainHPEffect_.next"));

    let final_enemy_hp = read_be16(&mut h, enemy_hp);
    let final_player_hp = read_be16(&mut h, player_hp);
    assert_eq!(final_enemy_hp, 70, "enemy HP should increase by 20");
    assert_eq!(final_player_hp, 80, "player HP should be unchanged");
}

#[test]
fn high_damage_value() {
    let r = run_drain(0x00, 100, 0xFFFF, 0xFFFE);
    assert_eq!(r.recovery, 0x7FFF, "damage 0xFFFE / 2 = 0x7FFF");
    assert_eq!(r.final_hp, 100 + 0x7FFF);
}
