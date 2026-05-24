//! Behavioral emulator tests for OneHitKOEffect_ (engine/battle/move_effects/one_hit_ko.asm).
//!
//! OneHitKOEffect_ handles OHKO moves (Fissure, Horn Drill, Guillotine).
//! - Compares user speed vs target speed (16-bit big-endian comparison).
//! - User faster or equal: wDamage = 0xFFFF, wCriticalHitOrOHKO = 2.
//! - User slower: wDamage = 0, wMoveMissed = 1, wCriticalHitOrOHKO = 0xFF.
//! - No display calls — returns cleanly via `ret`.

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
    let bank = sym_bank("OneHitKOEffect_");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h
}

struct OhkoResult {
    damage: u16,
    critical_hit_or_ohko: u8,
    move_missed: u8,
}

fn run_ohko(whose_turn: u8, player_speed: u16, enemy_speed: u16) -> OhkoResult {
    let mut h = setup_fixture();

    h.write_mem(sym_addr("hWhoseTurn"), whose_turn);

    let ps = sym_addr("wBattleMonSpeed");
    h.write_mem(ps, (player_speed >> 8) as u8);
    h.write_mem(ps + 1, (player_speed & 0xFF) as u8);

    let es = sym_addr("wEnemyMonSpeed");
    h.write_mem(es, (enemy_speed >> 8) as u8);
    h.write_mem(es + 1, (enemy_speed & 0xFF) as u8);

    h.write_mem(sym_addr("wMoveMissed"), 0x00);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("OneHitKOEffect_"));
    h.step_to(TRAP_ADDR);

    let wd = sym_addr("wDamage");
    let hi = h.read_mem(wd) as u16;
    let lo = h.read_mem(wd + 1) as u16;

    OhkoResult {
        damage: (hi << 8) | lo,
        critical_hit_or_ohko: h.read_mem(sym_addr("wCriticalHitOrOHKO")),
        move_missed: h.read_mem(sym_addr("wMoveMissed")),
    }
}

#[test]
fn player_faster_deals_ohko() {
    let r = run_ohko(0x00, 200, 100);
    assert_eq!(r.damage, 0xFFFF, "OHKO damage should be 65535");
    assert_eq!(r.critical_hit_or_ohko, 2, "OHKO flag should be 2");
    assert_eq!(r.move_missed, 0, "move should not miss");
}

#[test]
fn player_slower_misses() {
    let r = run_ohko(0x00, 100, 200);
    assert_eq!(r.damage, 0, "miss should deal 0 damage");
    assert_eq!(r.move_missed, 1, "wMoveMissed should be 1");
    assert_eq!(
        r.critical_hit_or_ohko, 0xFF,
        "wCriticalHitOrOHKO should stay at initial $FF on miss"
    );
}

#[test]
fn equal_speed_deals_ohko() {
    let r = run_ohko(0x00, 150, 150);
    assert_eq!(r.damage, 0xFFFF, "equal speed should OHKO (user wins tie)");
    assert_eq!(r.critical_hit_or_ohko, 2);
    assert_eq!(r.move_missed, 0);
}

#[test]
fn enemy_turn_faster_deals_ohko() {
    let r = run_ohko(0x01, 100, 200);
    assert_eq!(
        r.damage, 0xFFFF,
        "enemy's turn: enemy faster → OHKO should succeed"
    );
    assert_eq!(r.critical_hit_or_ohko, 2);
    assert_eq!(r.move_missed, 0);
}

#[test]
fn enemy_turn_slower_misses() {
    let r = run_ohko(0x01, 200, 100);
    assert_eq!(r.damage, 0, "enemy's turn: enemy slower → should miss");
    assert_eq!(r.move_missed, 1);
    assert_eq!(r.critical_hit_or_ohko, 0xFF);
}

#[test]
fn max_speed_vs_one() {
    let r = run_ohko(0x00, 0xFFFF, 0x0001);
    assert_eq!(r.damage, 0xFFFF, "max speed should always OHKO");
    assert_eq!(r.critical_hit_or_ohko, 2);
}

#[test]
fn speed_differ_by_one_faster() {
    let r = run_ohko(0x00, 101, 100);
    assert_eq!(r.damage, 0xFFFF, "speed+1 should still OHKO");
    assert_eq!(r.critical_hit_or_ohko, 2);
}

#[test]
fn speed_differ_by_one_slower() {
    let r = run_ohko(0x00, 100, 101);
    assert_eq!(r.damage, 0, "speed-1 should miss");
    assert_eq!(r.move_missed, 1);
}

#[test]
fn both_speeds_zero() {
    let r = run_ohko(0x00, 0, 0);
    assert_eq!(r.damage, 0xFFFF, "zero vs zero → equal → OHKO succeeds");
    assert_eq!(r.critical_hit_or_ohko, 2);
}

#[test]
fn high_byte_speed_comparison() {
    let r = run_ohko(0x00, 0x0200, 0x01FF);
    assert_eq!(r.damage, 0xFFFF, "512 vs 511: high byte matters → OHKO");
    assert_eq!(r.critical_hit_or_ohko, 2);
}
