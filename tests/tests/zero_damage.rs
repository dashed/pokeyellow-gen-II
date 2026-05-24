//! ROM byte tests for the 0 damage glitch fix.
//!
//! Bug: When a damaging move's damage calculation yields 0 against a dual-type
//! Pokemon that doubly resists the move (0.25x effectiveness), the game treats
//! it as a miss ("Attack missed!") instead of dealing minimum 1 damage. This
//! happens because floor(2/4) = floor(3/4) = 0, and the code equates 0 damage
//! with type immunity.
//!
//! Fix: After finding 0 damage, check `wDamageMultipliers & EFFECTIVENESS_MASK`.
//! If zero, the target is truly immune — set `wMoveMissed`. If non-zero, the
//! move connected but rounded to 0 — clamp damage to 1 instead of missing.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("AdjustDamageForMoveType"));
    h
}

// Z80/SM83 opcodes
const OR_B: u8 = 0xB0;
const JR_NZ: u8 = 0x20;
const JR_Z: u8 = 0x28;
const LD_A_MEM: u8 = 0xFA; // ld a, [nn]
const AND_IMM: u8 = 0xE6; // and n
const LD_HL_IMM8: u8 = 0x36; // ld [hl], n
const INC_A: u8 = 0x3C;
const LD_MEM_A: u8 = 0xEA; // ld [nn], a

// WRAM addresses
const W_DAMAGE_MULTIPLIERS: u16 = 0xD05A;
const W_MOVE_MISSED: u16 = 0xD05E;
const EFFECTIVENESS_MASK: u8 = 0x7F;

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn adjust_damage_in_bank_0f() {
    assert_eq!(sym_bank("AdjustDamageForMoveType"), 0x0F);
}

#[test]
fn adjust_damage_in_banked_range() {
    let addr = sym_addr("AdjustDamageForMoveType");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── The fix: distinguish immunity from 0.25x rounding ──────────────

#[test]
fn reads_damage_multipliers_in_zero_damage_path() {
    // After `or b / jr nz`, the fix reads `ld a, [wDamageMultipliers]`.
    let mut h = banked_harness();
    let start = sym_addr("AdjustDamageForMoveType.matchingPairFound");
    let end = sym_addr("AdjustDamageForMoveType.skipTypeImmunity");
    let lo = (W_DAMAGE_MULTIPLIERS & 0xFF) as u8;
    let hi = (W_DAMAGE_MULTIPLIERS >> 8) as u8;
    // Find `or b` followed by `jr nz` followed by `ld a, [wDamageMultipliers]`
    for addr in start..end {
        if rom(&mut h, addr) == OR_B
            && rom(&mut h, addr + 1) == JR_NZ
            // +2 is jr offset byte
            && rom(&mut h, addr + 3) == LD_A_MEM
            && rom(&mut h, addr + 4) == lo
            && rom(&mut h, addr + 5) == hi
        {
            return;
        }
    }
    panic!("ld a, [wDamageMultipliers] not found after or b / jr nz");
}

#[test]
fn and_effectiveness_mask_follows() {
    // After `or b / jr nz / ld a, [wDamageMultipliers]`, expect `and EFFECTIVENESS_MASK`.
    let mut h = banked_harness();
    let start = sym_addr("AdjustDamageForMoveType.matchingPairFound");
    let end = sym_addr("AdjustDamageForMoveType.skipTypeImmunity");
    let lo = (W_DAMAGE_MULTIPLIERS & 0xFF) as u8;
    let hi = (W_DAMAGE_MULTIPLIERS >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == OR_B
            && rom(&mut h, addr + 1) == JR_NZ
            && rom(&mut h, addr + 3) == LD_A_MEM
            && rom(&mut h, addr + 4) == lo
            && rom(&mut h, addr + 5) == hi
        {
            // or b (1) + jr nz (2) + ld a (3) + and n at +6
            assert_eq!(rom(&mut h, addr + 6), AND_IMM, "expected and n opcode");
            assert_eq!(
                rom(&mut h, addr + 7),
                EFFECTIVENESS_MASK,
                "expected EFFECTIVENESS_MASK ($7F)"
            );
            return;
        }
    }
    panic!("or b / jr nz / ld a, [wDamageMultipliers] sequence not found");
}

#[test]
fn jr_z_to_type_immunity_follows() {
    // After `and EFFECTIVENESS_MASK`, expect `jr z` to the immunity path.
    let mut h = banked_harness();
    let start = sym_addr("AdjustDamageForMoveType.matchingPairFound");
    let end = sym_addr("AdjustDamageForMoveType.skipTypeImmunity");
    let lo = (W_DAMAGE_MULTIPLIERS & 0xFF) as u8;
    let hi = (W_DAMAGE_MULTIPLIERS >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == OR_B
            && rom(&mut h, addr + 1) == JR_NZ
            && rom(&mut h, addr + 3) == LD_A_MEM
            && rom(&mut h, addr + 4) == lo
            && rom(&mut h, addr + 5) == hi
        {
            // or b (1) + jr nz (2) + ld a (3) + and n (2) + jr z at +8
            assert_eq!(rom(&mut h, addr + 8), JR_Z, "expected jr z opcode");
            return;
        }
    }
    panic!("or b / jr nz / ld a, [wDamageMultipliers] sequence not found");
}

#[test]
fn clamp_damage_to_1_in_non_immune_path() {
    // `ld [hl], 1` ($36 $01) should appear between matchingPairFound and
    // skipTypeImmunity — the fix that clamps damage to 1 instead of missing.
    let mut h = banked_harness();
    let start = sym_addr("AdjustDamageForMoveType.matchingPairFound");
    let end = sym_addr("AdjustDamageForMoveType.skipTypeImmunity");
    let mut found = false;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM8 && rom(&mut h, addr + 1) == 0x01 {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld [hl], 1 (damage clamp) not found in AdjustDamageForMoveType"
    );
}

#[test]
fn type_immunity_sets_move_missed() {
    // At .typeImmunity, expect `inc a / ld [wMoveMissed], a`.
    let mut h = banked_harness();
    let addr = sym_addr("AdjustDamageForMoveType.typeImmunity");
    let lo = (W_MOVE_MISSED & 0xFF) as u8;
    let hi = (W_MOVE_MISSED >> 8) as u8;
    assert_eq!(rom(&mut h, addr), INC_A, "expected inc a at .typeImmunity");
    assert_eq!(
        rom(&mut h, addr + 1),
        LD_MEM_A,
        "expected ld [nn], a opcode"
    );
    assert_eq!(rom(&mut h, addr + 2), lo, "expected wMoveMissed lo byte");
    assert_eq!(rom(&mut h, addr + 3), hi, "expected wMoveMissed hi byte");
}

#[test]
fn clamp_before_immunity_path() {
    // The `ld [hl], 1` (clamp) must come BEFORE .typeImmunity (ordering).
    let mut h = banked_harness();
    let start = sym_addr("AdjustDamageForMoveType.matchingPairFound");
    let end = sym_addr("AdjustDamageForMoveType.skipTypeImmunity");
    let type_immunity = sym_addr("AdjustDamageForMoveType.typeImmunity");
    let mut clamp_addr: Option<u16> = None;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM8 && rom(&mut h, addr + 1) == 0x01 {
            clamp_addr = Some(addr);
            break;
        }
    }
    let ca = clamp_addr.expect("ld [hl], 1 not found");
    assert!(
        ca < type_immunity,
        "clamp at {:#06X} should come before .typeImmunity at {:#06X}",
        ca,
        type_immunity
    );
}

// ─── Behavioral tests ──────────────────────────────────────────────

// Type constants (from constants/type_constants.asm).
const NORMAL: u8 = 0x00;
const FLYING: u8 = 0x02;
const GHOST: u8 = 0x08;
const FIRE: u8 = 0x14;
const GRASS: u8 = 0x16;

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;
const EFFECTIVE: u8 = 10;

struct DamageResult {
    damage: u16,
    move_missed: u8,
    effectiveness: u8,
}

fn run_damage_scenario(
    move_type: u8,
    def_type1: u8,
    def_type2: u8,
    initial_damage: u16,
) -> DamageResult {
    let w_damage = sym_addr("wDamage");
    let w_damage_multipliers = sym_addr("wDamageMultipliers");
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("AdjustDamageForMoveType");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    h.write_mem(sym_addr("wMoveType"), move_type);
    h.write_mem(w_damage_multipliers, EFFECTIVE);
    h.write_mem(w_damage, (initial_damage >> 8) as u8);
    h.write_mem(w_damage + 1, (initial_damage & 0xFF) as u8);
    h.write_mem(W_MOVE_MISSED, 0);

    h.set_b(move_type);
    h.gb.cpu()
        .set_de(((def_type1 as u16) << 8) | def_type2 as u16);

    h.set_pc(sym_addr("AdjustDamageForMoveType.skipSameTypeAttackBonus"));
    h.step_to(sym_addr("AdjustDamageForMoveType.done"));

    let damage_hi = h.read_mem(w_damage) as u16;
    let damage_lo = h.read_mem(w_damage + 1) as u16;

    DamageResult {
        damage: (damage_hi << 8) | damage_lo,
        move_missed: h.read_mem(W_MOVE_MISSED),
        effectiveness: h.read_mem(w_damage_multipliers) & 0x7F,
    }
}

#[test]
fn behavioral_quarter_effective_clamps_to_1() {
    // Grass vs Fire/Flying = NVE × NVE = 0.25×.
    // With base damage 2: floor(2 * 0.5) = 1, floor(1 * 0.5) = 0.
    // The fix should clamp damage to 1 instead of reporting a miss.
    let r = run_damage_scenario(GRASS, FIRE, FLYING, 2);
    assert_eq!(
        r.damage, 1,
        "0.25× damage should clamp to 1, got {}",
        r.damage
    );
    assert_eq!(r.move_missed, 0, "move should NOT be flagged as missed");
    assert!(
        r.effectiveness > 0,
        "effectiveness should be non-zero (not immune)"
    );
}

#[test]
fn behavioral_immune_sets_move_missed() {
    // Normal vs Ghost (mono-type) = immune (0×).
    // Damage should be 0, wMoveMissed should be set to 1.
    let r = run_damage_scenario(NORMAL, GHOST, GHOST, 100);
    assert_eq!(r.damage, 0, "immune damage should be 0, got {}", r.damage);
    assert_eq!(r.move_missed, 1, "immune should set wMoveMissed to 1");
    assert_eq!(r.effectiveness, 0, "effectiveness should be 0 for immunity");
}

#[test]
fn behavioral_normal_damage_passes_through() {
    // Fire vs Grass (mono-type) = SE (2×).
    // Damage 100 → 200. No miss, positive effectiveness.
    let r = run_damage_scenario(FIRE, GRASS, GRASS, 100);
    assert_eq!(r.damage, 200, "2× damage should be 200, got {}", r.damage);
    assert_eq!(r.move_missed, 0, "SE hit should NOT set wMoveMissed");
    assert_eq!(r.effectiveness, 20, "effectiveness should be 20 (SE)");
}

#[test]
fn behavioral_nve_halves_damage() {
    // Fire vs Fire (mono-type) = NVE (0.5×).
    // Damage 100 → 50. No miss.
    let r = run_damage_scenario(FIRE, FIRE, FIRE, 100);
    assert_eq!(r.damage, 50, "0.5× damage should be 50, got {}", r.damage);
    assert_eq!(r.move_missed, 0, "NVE hit should NOT set wMoveMissed");
    assert_eq!(r.effectiveness, 5, "effectiveness should be 5 (NVE)");
}

#[test]
fn behavioral_neutral_preserves_damage() {
    // Normal vs Normal (mono-type) = neutral (1×).
    // No TypeEffects entry → damage unchanged.
    let r = run_damage_scenario(NORMAL, NORMAL, NORMAL, 100);
    assert_eq!(
        r.damage, 100,
        "neutral damage should be 100, got {}",
        r.damage
    );
    assert_eq!(r.move_missed, 0, "neutral hit should NOT set wMoveMissed");
    assert_eq!(r.effectiveness, 10, "effectiveness should be 10 (neutral)");
}

#[test]
fn behavioral_quarter_effective_higher_damage_survives() {
    // Grass vs Fire/Flying = 0.25×.
    // With base damage 100: floor(100 * 0.5) = 50, floor(50 * 0.5) = 25.
    // Damage is > 0, so the clamp path is never entered.
    let r = run_damage_scenario(GRASS, FIRE, FLYING, 100);
    assert_eq!(r.damage, 25, "0.25× of 100 should be 25, got {}", r.damage);
    assert_eq!(r.move_missed, 0, "should NOT be flagged as missed");
}
