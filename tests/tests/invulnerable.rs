//! Emulator-based tests for the Fly/Dig invulnerability persistence fix.
//!
//! The bug: when full paralysis or confusion self-hit prevents a Pokemon from
//! completing the second turn of Fly or Dig, the INVULNERABLE bit (bit 6 of
//! wPlayerBattleStatus1 / wEnemyBattleStatus1) stays set. All opponent moves
//! then miss until the status is cleared by using Fly/Dig again.
//!
//! The fix: add `(1 << INVULNERABLE)` to the AND bitmask in
//! `.MonHurtItselfOrFullyParalysed` for both player and enemy sides.
//! This changes the immediate operand from $CC to $8C (zero-byte fix).

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// BattleStatus1 bit positions.
const STORING_ENERGY: u8 = 0;
const THRASHING_ABOUT: u8 = 1;
const CHARGING_UP: u8 = 4;
const USING_TRAPPING_MOVE: u8 = 5;
const INVULNERABLE: u8 = 6;
const CONFUSED: u8 = 7;

/// Move effect constants.
const NO_ADDITIONAL_EFFECT: u8 = 0x00;

/// A safe WRAM address for the return trap.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Set up harness for a MonHurtItselfOrFullyParalysed test.
fn setup_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("CheckPlayerStatusConditions"));

    // Trap for jumps to ExecuteMoveDone (via .returnToHL / .enemyReturnToHL)
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);

    h
}

/// Run the player MonHurtItselfOrFullyParalysed code with the given initial
/// BattleStatus1 and move effect. Returns the BattleStatus1 value after the
/// AND mask is applied (stops at .NotFlyOrChargeEffect to avoid animation).
fn run_player_mask(initial_status: u8, move_effect: u8) -> u8 {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("wPlayerBattleStatus1"), initial_status);
    h.write_mem(sym_addr("wPlayerMoveEffect"), move_effect);
    h.set_pc(sym_addr(
        "CheckPlayerStatusConditions.MonHurtItselfOrFullyParalysed",
    ));
    h.step_to(sym_addr("CheckPlayerStatusConditions.NotFlyOrChargeEffect"));
    h.read_mem(sym_addr("wPlayerBattleStatus1"))
}

/// Run the enemy monHurtItselfOrFullyParalysed code with the given initial
/// BattleStatus1 and move effect. Returns the BattleStatus1 value after the
/// AND mask is applied (stops at .notFlyOrChargeEffect to avoid animation).
fn run_enemy_mask(initial_status: u8, move_effect: u8) -> u8 {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("wEnemyBattleStatus1"), initial_status);
    h.write_mem(sym_addr("wEnemyMoveEffect"), move_effect);
    h.set_pc(sym_addr(
        "CheckEnemyStatusConditions.monHurtItselfOrFullyParalysed",
    ));
    h.step_to(sym_addr("CheckEnemyStatusConditions.notFlyOrChargeEffect"));
    h.read_mem(sym_addr("wEnemyBattleStatus1"))
}

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_player_and_mask_includes_invulnerable() {
    // The AND instruction at MonHurtItselfOrFullyParalysed is:
    //   ld hl, wPlayerBattleStatus1 (3 bytes)
    //   ld a, [hl] (1 byte)
    //   and $8C (2 bytes: $E6 $8C)
    // The AND operand should be $8C = ~($01|$02|$10|$20|$40)
    let mut h = setup_fixture();
    let player_hurt = sym_addr("CheckPlayerStatusConditions.MonHurtItselfOrFullyParalysed");
    // AND instruction is at offset +4 from the label (after ld hl,nn + ld a,[hl])
    let and_opcode = h.read_mem(player_hurt + 4);
    let and_operand = h.read_mem(player_hurt + 5);
    assert_eq!(and_opcode, 0xE6, "Expected AND imm8 opcode ($E6)");
    assert_eq!(
        and_operand, 0x8C,
        "Player AND mask should be $8C (clears INVULNERABLE), got ${and_operand:02X}"
    );
}

#[test]
fn rom_bytes_enemy_and_mask_includes_invulnerable() {
    let mut h = setup_fixture();
    let enemy_hurt = sym_addr("CheckEnemyStatusConditions.monHurtItselfOrFullyParalysed");
    let and_opcode = h.read_mem(enemy_hurt + 4);
    let and_operand = h.read_mem(enemy_hurt + 5);
    assert_eq!(and_opcode, 0xE6, "Expected AND imm8 opcode ($E6)");
    assert_eq!(
        and_operand, 0x8C,
        "Enemy AND mask should be $8C (clears INVULNERABLE), got ${and_operand:02X}"
    );
}

// ─── Behavioral: INVULNERABLE cleared by full paralysis path ────────

#[test]
fn player_invulnerable_cleared_by_paralysis_mask() {
    // Simulate: Fly turn 1 set CHARGING_UP | INVULNERABLE, then full paralysis
    let initial = (1 << CHARGING_UP) | (1 << INVULNERABLE) | (1 << CONFUSED);
    let result = run_player_mask(initial, NO_ADDITIONAL_EFFECT);
    assert_eq!(
        result & (1 << INVULNERABLE),
        0,
        "INVULNERABLE should be cleared: result=${result:02X}"
    );
    assert_eq!(
        result & (1 << CHARGING_UP),
        0,
        "CHARGING_UP should be cleared: result=${result:02X}"
    );
    assert_ne!(
        result & (1 << CONFUSED),
        0,
        "CONFUSED should be preserved: result=${result:02X}"
    );
}

#[test]
fn enemy_invulnerable_cleared_by_paralysis_mask() {
    let initial = (1 << CHARGING_UP) | (1 << INVULNERABLE) | (1 << CONFUSED);
    let result = run_enemy_mask(initial, NO_ADDITIONAL_EFFECT);
    assert_eq!(
        result & (1 << INVULNERABLE),
        0,
        "INVULNERABLE should be cleared: result=${result:02X}"
    );
    assert_eq!(
        result & (1 << CHARGING_UP),
        0,
        "CHARGING_UP should be cleared: result=${result:02X}"
    );
    assert_ne!(
        result & (1 << CONFUSED),
        0,
        "CONFUSED should be preserved: result=${result:02X}"
    );
}

// ─── Only Fly/Dig bits are affected ─────────────────────────────────

#[test]
fn player_mask_clears_all_expected_bits() {
    // Set ALL bits and verify which are cleared
    let initial = 0xFF;
    let result = run_player_mask(initial, NO_ADDITIONAL_EFFECT);
    // Expected preserved: bits 2 (ATTACKING_MULTIPLE_TIMES), 3 (FLINCHED), 7 (CONFUSED)
    // Expected cleared: bits 0,1,4,5,6
    assert_eq!(
        result, 0x8C,
        "With all bits set, only bits 2,3,7 should survive: result=${result:02X}"
    );
}

#[test]
fn enemy_mask_clears_all_expected_bits() {
    let initial = 0xFF;
    let result = run_enemy_mask(initial, NO_ADDITIONAL_EFFECT);
    assert_eq!(
        result, 0x8C,
        "With all bits set, only bits 2,3,7 should survive: result=${result:02X}"
    );
}

// ─── Solar Beam (CHARGING_UP without INVULNERABLE) is unaffected ────

#[test]
fn player_charging_without_invulnerable_works() {
    // Solar Beam sets CHARGING_UP but NOT INVULNERABLE
    let initial = 1 << CHARGING_UP;
    let result = run_player_mask(initial, NO_ADDITIONAL_EFFECT);
    assert_eq!(
        result, 0,
        "CHARGING_UP-only should be fully cleared: result=${result:02X}"
    );
}

// ─── Bide / trapping moves still cleared ────────────────────────────

#[test]
fn player_bide_and_trapping_still_cleared() {
    let initial = (1 << STORING_ENERGY) | (1 << THRASHING_ABOUT) | (1 << USING_TRAPPING_MOVE);
    let result = run_player_mask(initial, NO_ADDITIONAL_EFFECT);
    assert_eq!(
        result, 0,
        "Bide, thrashing, trapping should all be cleared: result=${result:02X}"
    );
}
