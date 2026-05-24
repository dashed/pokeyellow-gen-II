//! Emulator-based tests for the healing move HP comparison fix.
//!
//! The bug: Recover/Softboiled/Rest compare current HP to max HP to check if
//! the user is already at full health. The comparison uses `cp [hl]` for the
//! high byte, then `sbc [hl]` for the low byte, but only checks Z after `sbc`.
//! When maxHP - currentHP is exactly 255 or 511, the `sbc` result is 0 despite
//! the high bytes differing, causing the move to incorrectly fail.
//!
//! The fix: replace `cp [hl]` with `sub [hl]`, save the high byte difference
//! in C, and use `or c` after `sbc` so Z is only set when BOTH bytes match.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Move constants.
const RECOVER: u8 = 0x69;

/// A safe WRAM address for the return trap.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Outcome of the HP-full check in HealEffect_.
#[derive(Debug, PartialEq)]
enum HealCheckResult {
    /// Reached .failed — move thinks HP is full.
    Failed,
    /// Reached .healHP — move proceeds to heal.
    Heals,
}

/// Run HealEffect_.healEffect with given current HP and max HP.
/// Returns whether the move fails (HP "full") or proceeds to heal.
fn check_heal(current_hp: u16, max_hp: u16) -> HealCheckResult {
    let heal_effect = sym_addr("HealEffect_.healEffect");
    let heal_failed = sym_addr("HealEffect_.failed");
    let heal_hp = sym_addr("HealEffect_.healHP");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("HealEffect_"));

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);

    let w_battle_mon_hp = sym_addr("wBattleMonHP");
    let w_battle_mon_max_hp = sym_addr("wBattleMonMaxHP");

    // Player's turn
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    // Set move to Recover (not REST, to avoid the sleep path)
    h.write_mem(sym_addr("wPlayerMoveNum"), RECOVER);

    // Set current HP (big-endian: high byte at addr, low byte at addr+1)
    h.write_mem(w_battle_mon_hp, (current_hp >> 8) as u8);
    h.write_mem(w_battle_mon_hp + 1, (current_hp & 0xFF) as u8);

    // Set max HP
    h.write_mem(w_battle_mon_max_hp, (max_hp >> 8) as u8);
    h.write_mem(w_battle_mon_max_hp + 1, (max_hp & 0xFF) as u8);

    // At .healEffect: A = move number, DE = wBattleMonHP, HL = wBattleMonMaxHP
    // (these are set by the caller code before .healEffect, but since we start
    // AT .healEffect, the first instruction is `ld b, a` which saves A to B)
    h.set_a(RECOVER);
    h.gb.cpu().set_de(w_battle_mon_hp);
    h.gb.cpu().set_hl(w_battle_mon_max_hp);

    h.set_pc(heal_effect);

    // Step until we reach .failed or .healHP
    for _ in 0..100 {
        let pc = h.pc();
        if pc == heal_failed {
            return HealCheckResult::Failed;
        }
        if pc == heal_hp {
            return HealCheckResult::Heals;
        }
        h.gb.clock();
    }
    panic!(
        "HealEffect did not reach a decision point within 100 instructions (PC=${:04X})",
        h.pc()
    );
}

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_sub_instead_of_cp() {
    let heal_effect = sym_addr("HealEffect_.healEffect");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("HealEffect_"));

    // At .healEffect+1 (after ld b,a): ld a,[de] = $1A
    // At .healEffect+2: sub [hl] = $96 (was cp [hl] = $BE)
    let sub_opcode = h.read_mem(heal_effect + 2);
    assert_eq!(
        sub_opcode, 0x96,
        "Expected sub [hl] ($96) at .healEffect+2, got ${sub_opcode:02X}"
    );
}

#[test]
fn rom_bytes_ld_c_a_and_or_c_present() {
    let heal_effect = sym_addr("HealEffect_.healEffect");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("HealEffect_"));

    // Byte layout from .healEffect:
    // +0: ld b,a ($47)
    // +1: ld a,[de] ($1A)
    // +2: sub [hl] ($96)
    // +3: inc de ($13)
    // +4: inc hl ($23)
    // +5: ld c,a ($4F) ← NEW
    // +6: ld a,[de] ($1A)
    // +7: sbc [hl] ($9E)
    // +8: or c ($B1) ← NEW
    let ld_c_a = h.read_mem(heal_effect + 5);
    let or_c = h.read_mem(heal_effect + 8);
    assert_eq!(
        ld_c_a, 0x4F,
        "Expected ld c,a ($4F) at .healEffect+5, got ${ld_c_a:02X}"
    );
    assert_eq!(
        or_c, 0xB1,
        "Expected or c ($B1) at .healEffect+8, got ${or_c:02X}"
    );
}

// ─── Bug scenario: HP 255 below max ────────────────────────────────

#[test]
fn heal_does_not_fail_when_hp_255_below_max() {
    // maxHP=512 ($0200), currentHP=257 ($0101), gap=255
    // OLD BUG: cp→sbc→Z=1 (fails). FIX: sub→or c→Z=0 (heals).
    let result = check_heal(0x0101, 0x0200);
    assert_eq!(
        result,
        HealCheckResult::Heals,
        "Recover should NOT fail when HP is 255 below max"
    );
}

#[test]
fn heal_does_not_fail_when_hp_255_below_max_variant() {
    // maxHP=400 ($0190), currentHP=145 ($0091), gap=255
    let result = check_heal(0x0091, 0x0190);
    assert_eq!(
        result,
        HealCheckResult::Heals,
        "Recover should NOT fail when HP is 255 below max (variant)"
    );
}

// ─── Bug scenario: HP 511 below max ────────────────────────────────

#[test]
fn heal_does_not_fail_when_hp_511_below_max() {
    // maxHP=703 ($02BF), currentHP=192 ($00C0), gap=511
    // This is the Chansey scenario (max possible HP in Gen 1).
    let result = check_heal(0x00C0, 0x02BF);
    assert_eq!(
        result,
        HealCheckResult::Heals,
        "Recover should NOT fail when HP is 511 below max"
    );
}

// ─── Normal behavior: full HP still fails ──────────────────────────

#[test]
fn heal_fails_when_hp_equals_max() {
    // maxHP=300, currentHP=300 → should fail
    let result = check_heal(0x012C, 0x012C);
    assert_eq!(
        result,
        HealCheckResult::Failed,
        "Recover should fail when HP is already full"
    );
}

#[test]
fn heal_fails_when_hp_equals_max_high_values() {
    // maxHP=703 ($02BF), currentHP=703 → should fail
    let result = check_heal(0x02BF, 0x02BF);
    assert_eq!(
        result,
        HealCheckResult::Failed,
        "Recover should fail when HP is already full (703)"
    );
}

// ─── Normal behavior: partial HP heals ─────────────────────────────

#[test]
fn heal_succeeds_when_hp_1_below_max() {
    let result = check_heal(0x012B, 0x012C);
    assert_eq!(
        result,
        HealCheckResult::Heals,
        "Recover should succeed when HP is 1 below max"
    );
}

#[test]
fn heal_succeeds_when_hp_256_below_max() {
    // maxHP=512, currentHP=256, gap=256
    let result = check_heal(0x0100, 0x0200);
    assert_eq!(
        result,
        HealCheckResult::Heals,
        "Recover should succeed when HP is 256 below max"
    );
}

#[test]
fn heal_succeeds_when_hp_is_1() {
    let result = check_heal(0x0001, 0x02BF);
    assert_eq!(
        result,
        HealCheckResult::Heals,
        "Recover should succeed when HP is 1"
    );
}
