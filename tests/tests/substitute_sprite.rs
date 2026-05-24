//! ROM byte tests for the Substitute sprite vanishing fix.
//!
//! Bug: When Self-Destruct or Explosion breaks the target's Substitute,
//! AttackSubstitute.nullifyEffect unconditionally zeros the move effect,
//! preventing ExplodeEffect from running.  The user survives with full HP
//! while their sprite vanishes (from the explosion animation).
//!
//! Fix: In AttackSubstitute.nullifyEffect, check if the move effect is
//! EXPLODE_EFFECT before zeroing it.  If it is, skip the nullification
//! so ExplodeEffect runs and properly faints the user.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Substitute_sprite_vanishing>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const LD_A_HL: u8 = 0x7E; // ld a, [hl]
const CP_N: u8 = 0xFE; // cp n
const JR_Z: u8 = 0x28; // jr z, n
const XOR_A: u8 = 0xAF; // xor a
const LD_HL_A: u8 = 0x77; // ld [hl], a
const JP_NN: u8 = 0xC3; // jp nn

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn attack_substitute_in_bank_0f() {
    assert_eq!(sym_bank("AttackSubstitute"), 0x0F);
}

#[test]
fn nullify_effect_checks_explode_effect() {
    // At .nullifyEffect, the fix inserts:
    //   ld a, [hl]           ($7E)
    //   cp EXPLODE_EFFECT    ($FE nn)
    //   jr z, .dontNullify   ($28 nn)
    //   xor a                ($AF)
    //   ld [hl], a           ($77)
    // .dontNullify:
    //   jp DrawHUDsAndHPBars ($C3 nn nn)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let nullify = sym_addr("AttackSubstitute.nullifyEffect");

    assert_eq!(
        rom(&mut h, nullify),
        LD_A_HL,
        "Expected `ld a, [hl]` at .nullifyEffect"
    );
    assert_eq!(rom(&mut h, nullify + 1), CP_N, "Expected `cp n` opcode");
    assert_eq!(
        rom(&mut h, nullify + 3),
        JR_Z,
        "Expected `jr z` to skip nullification"
    );
    assert_eq!(
        rom(&mut h, nullify + 5),
        XOR_A,
        "Expected `xor a` for nullification"
    );
    assert_eq!(
        rom(&mut h, nullify + 6),
        LD_HL_A,
        "Expected `ld [hl], a` to store zeroed effect"
    );
}

#[test]
fn jr_z_targets_dont_nullify() {
    // The jr z should target .dontNullify (the jp DrawHUDsAndHPBars)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let nullify = sym_addr("AttackSubstitute.nullifyEffect");
    let dont_nullify = sym_addr("AttackSubstitute.dontNullify");

    // jr z is at nullify + 3, operand at nullify + 4
    let jr_addr = nullify + 3;
    let jr_operand = rom(&mut h, jr_addr + 1) as i8;
    let target = (jr_addr as i32 + 2 + jr_operand as i32) as u16;

    assert_eq!(
        target, dont_nullify,
        "jr z should target .dontNullify ({:#06X}), got {:#06X}",
        dont_nullify, target
    );
}

#[test]
fn dont_nullify_has_jp() {
    // .dontNullify should have jp DrawHUDsAndHPBars
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let dont_nullify = sym_addr("AttackSubstitute.dontNullify");

    assert_eq!(
        rom(&mut h, dont_nullify),
        JP_NN,
        "Expected `jp nn` at .dontNullify"
    );
}

#[test]
fn cp_operand_is_explode_effect() {
    // The cp operand should be EXPLODE_EFFECT ($07)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let nullify = sym_addr("AttackSubstitute.nullifyEffect");
    let operand = rom(&mut h, nullify + 2);

    // EXPLODE_EFFECT is defined in constants/move_effect_constants.asm
    // We can verify it's non-zero and matches the expected value
    assert_ne!(operand, 0, "EXPLODE_EFFECT should be non-zero");
    // EXPLODE_EFFECT = $07 in Gen I
    assert_eq!(
        operand, 0x07,
        "Expected EXPLODE_EFFECT ($07), got {:#04X}",
        operand
    );
}
