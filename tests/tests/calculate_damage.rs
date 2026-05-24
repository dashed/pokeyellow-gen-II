//! Behavioral emulator tests for the CalculateDamage function.
//!
//! CalculateDamage is the core damage formula used in every battle.
//! Formula: damage = ((2*level/5 + 2) * basePower * attack / defense) / 50 + 2
//! Capped at MAX_NEUTRAL_DAMAGE (999).
//!
//! Inputs via registers: B=attack, C=defense, D=basePower, E=level.
//! Output: wDamage (2 bytes, big-endian at $D0D6).

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

const EXPLODE_EFFECT: u8 = 0x07;

fn setup_damage_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("CalculateDamage");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h
}

fn calc_damage(h: &mut TestHarness, level: u8, attack: u8, defense: u8, base_power: u8) -> u16 {
    let w_damage = sym_addr("wDamage");
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wPlayerMoveEffect"), 0x00);
    h.write_mem(w_damage, 0x00);
    h.write_mem(w_damage + 1, 0x00);
    h.set_b(attack);
    h.gb.cpu().c = defense;
    h.gb.cpu().d = base_power;
    h.gb.cpu().e = level;
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CalculateDamage"));
    h.step_to(TRAP_ADDR);
    let hi = h.read_mem(w_damage) as u16;
    let lo = h.read_mem(w_damage + 1) as u16;
    (hi << 8) | lo
}

fn calc_damage_with_effect(
    h: &mut TestHarness,
    level: u8,
    attack: u8,
    defense: u8,
    base_power: u8,
    effect: u8,
) -> u16 {
    let w_damage = sym_addr("wDamage");
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wPlayerMoveEffect"), effect);
    h.write_mem(w_damage, 0x00);
    h.write_mem(w_damage + 1, 0x00);
    h.set_b(attack);
    h.gb.cpu().c = defense;
    h.gb.cpu().d = base_power;
    h.gb.cpu().e = level;
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CalculateDamage"));
    h.step_to(TRAP_ADDR);
    let hi = h.read_mem(w_damage) as u16;
    let lo = h.read_mem(w_damage + 1) as u16;
    (hi << 8) | lo
}

/// Compute expected damage using the same integer arithmetic as the Game Boy.
fn expected_damage(level: u8, base_power: u8, attack: u8, defense: u8) -> u16 {
    if base_power == 0 {
        return 0;
    }
    let step1 = (level as u32 * 2) / 5;
    let step2 = step1 + 2;
    let step3 = step2 * base_power as u32;
    let step4 = step3 * attack as u32;
    let step5 = step4 / defense as u32;
    let step6 = step5 / 50;
    let capped = step6.min(997);
    (capped + 2) as u16
}

// ─── Test: basic damage formula ────────────────────────────────────

#[test]
fn basic_damage_formula() {
    let mut h = setup_damage_fixture();
    let damage = calc_damage(&mut h, 50, 100, 100, 40);
    let expected = expected_damage(50, 40, 100, 100);
    assert_eq!(
        damage, expected,
        "Level 50, Atk 100, Def 100, Power 40: expected {expected}, got {damage}"
    );
}

// ─── Test: minimum damage is MIN_NEUTRAL_DAMAGE (2) ────────────────

#[test]
fn minimum_damage_is_2() {
    let mut h = setup_damage_fixture();
    let damage = calc_damage(&mut h, 1, 1, 255, 1);
    assert_eq!(
        damage, 2,
        "Minimum possible damage should be MIN_NEUTRAL_DAMAGE (2)"
    );
}

// ─── Test: maximum damage caps at MAX_NEUTRAL_DAMAGE (999) ─────────

#[test]
fn maximum_damage_caps_at_999() {
    let mut h = setup_damage_fixture();
    let damage = calc_damage(&mut h, 255, 255, 1, 255);
    assert_eq!(
        damage, 999,
        "Damage should be capped at MAX_NEUTRAL_DAMAGE (999)"
    );
}

// ─── Test: defense=1 produces high damage ──────────────────────────

#[test]
fn defense_1_high_damage() {
    let mut h = setup_damage_fixture();
    let damage = calc_damage(&mut h, 100, 200, 1, 100);
    let expected = expected_damage(100, 100, 200, 1);
    assert_eq!(
        damage, expected,
        "Defense 1 should produce very high damage"
    );
    assert!(
        damage > 100,
        "Defense 1 with these stats should yield high damage, got {damage}"
    );
}

// ─── Test: base power 0 returns no damage ──────────────────────────

#[test]
fn power_0_returns_no_damage() {
    let mut h = setup_damage_fixture();
    let damage = calc_damage(&mut h, 50, 100, 100, 0);
    assert_eq!(
        damage, 0,
        "Base power 0 should cause early return with 0 damage"
    );
}

// ─── Test: level 100 produces more damage than level 50 ────────────

#[test]
fn level_100_higher_than_level_50() {
    let mut h = setup_damage_fixture();
    let damage_50 = calc_damage(&mut h, 50, 100, 100, 40);
    let damage_100 = calc_damage(&mut h, 100, 100, 100, 40);
    assert!(
        damage_100 > damage_50,
        "Level 100 ({damage_100}) should deal more damage than level 50 ({damage_50})"
    );
    assert_eq!(damage_50, expected_damage(50, 40, 100, 100));
    assert_eq!(damage_100, expected_damage(100, 40, 100, 100));
}

// ─── Test: EXPLODE_EFFECT halves defense ───────────────────────────

#[test]
fn explode_effect_halves_defense() {
    let mut h = setup_damage_fixture();
    let normal_damage = calc_damage(&mut h, 50, 100, 100, 40);
    let explode_damage = calc_damage_with_effect(&mut h, 50, 100, 100, 40, EXPLODE_EFFECT);
    let expected = expected_damage(50, 40, 100, 50);
    assert_eq!(
        explode_damage, expected,
        "EXPLODE_EFFECT should halve defense (100 → 50)"
    );
    assert!(
        explode_damage > normal_damage,
        "Halved defense should increase damage: explode={explode_damage} > normal={normal_damage}"
    );
}

// ─── Test: EXPLODE_EFFECT defense clamp (defense=1 → srl→0 → inc→1)

#[test]
fn explode_defense_clamp() {
    let mut h = setup_damage_fixture();
    let damage = calc_damage_with_effect(&mut h, 50, 100, 1, 40, EXPLODE_EFFECT);
    let expected = expected_damage(50, 40, 100, 1);
    assert_eq!(
        damage, expected,
        "EXPLODE_EFFECT with defense=1: srl makes 0, inc clamps to 1"
    );
}

// ─── Test: sweep representative inputs ─────────────────────────────

#[test]
fn sweep_representative_inputs() {
    let mut h = setup_damage_fixture();
    let levels: &[u8] = &[1, 5, 10, 25, 50, 75, 100, 150, 200, 255];
    let attacks: &[u8] = &[1, 10, 50, 100, 150, 200, 255];
    let defenses: &[u8] = &[1, 2, 10, 50, 100, 150, 200, 255];
    let powers: &[u8] = &[1, 10, 40, 80, 120, 200, 255];

    let mut failures = Vec::new();
    let mut count = 0u32;

    for &level in levels {
        for &attack in attacks {
            for &defense in defenses {
                for &power in powers {
                    let actual = calc_damage(&mut h, level, attack, defense, power);
                    let expected = expected_damage(level, power, attack, defense);
                    if actual != expected {
                        failures.push(format!(
                            "L={level} A={attack} D={defense} P={power}: expected {expected}, got {actual}"
                        ));
                    }
                    count += 1;
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Damage formula mismatches ({}/{count} failed):\n{}",
        failures.len(),
        failures[..failures.len().min(20)].join("\n")
    );
    assert!(
        count >= 100,
        "Should test at least 100 combinations, tested {count}"
    );
}
