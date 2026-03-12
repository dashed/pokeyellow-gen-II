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
