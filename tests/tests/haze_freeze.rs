//! Emulator-based tests for the Haze freeze / Hyper Beam recharge lockup fix.
//!
//! The bug: When the enemy freezes the player during Hyper Beam recharge, the
//! NEEDS_TO_RECHARGE bit is not cleared (unlike when the player freezes the
//! enemy). If the enemy then uses Haze to cure the freeze, wPlayerSelectedMove
//! is set to $FF (CANNOT_MOVE). On the next turn, ExecutePlayerMove bails out
//! at the CANNOT_MOVE check before reaching CheckPlayerStatusConditions, so
//! the .HyperBeamCheck that clears NEEDS_TO_RECHARGE is never reached. The
//! player is permanently locked out of selecting moves.
//!
//! The fix: Add `call ClearHyperBeam` at the start of `.freeze2`, matching
//! `.freeze1`. This clears NEEDS_TO_RECHARGE when freeze is applied, preventing
//! the softlock even if Haze later sets the selected move to $FF.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const NEEDS_TO_RECHARGE: u8 = 5;

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_freeze2_calls_clear_hyper_beam() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("FreezeBurnParalyzeEffect"));

    let freeze2 = sym_addr("FreezeBurnParalyzeEffect.freeze2");
    let clear_hyper_beam = sym_addr("ClearHyperBeam");

    // .freeze2 should start with `call ClearHyperBeam` ($CD, lo, hi)
    let call_opcode = h.read_mem(freeze2);
    let call_lo = h.read_mem(freeze2 + 1);
    let call_hi = h.read_mem(freeze2 + 2);
    let target = (call_hi as u16) << 8 | call_lo as u16;

    assert_eq!(
        call_opcode, 0xCD,
        "Expected call ($CD) at .freeze2, got ${call_opcode:02X}"
    );
    assert_eq!(
        target, clear_hyper_beam,
        "Expected call target ClearHyperBeam (${clear_hyper_beam:04X}), got ${target:04X}"
    );
}

#[test]
fn rom_bytes_freeze1_also_calls_clear_hyper_beam() {
    // Verify .freeze1 (player freezes enemy) still has the call for symmetry
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("FreezeBurnParalyzeEffect"));

    let freeze1 = sym_addr("FreezeBurnParalyzeEffect.freeze1");
    let clear_hyper_beam = sym_addr("ClearHyperBeam");

    let call_opcode = h.read_mem(freeze1);
    let call_lo = h.read_mem(freeze1 + 1);
    let call_hi = h.read_mem(freeze1 + 2);
    let target = (call_hi as u16) << 8 | call_lo as u16;

    assert_eq!(call_opcode, 0xCD, "Expected call ($CD) at .freeze1");
    assert_eq!(
        target, clear_hyper_beam,
        ".freeze1 should also call ClearHyperBeam"
    );
}

// ─── Behavioral: enemy freezes player clears recharge ───────────────

#[test]
fn freeze2_clears_needs_to_recharge() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("FreezeBurnParalyzeEffect"));

    let freeze2 = sym_addr("FreezeBurnParalyzeEffect.freeze2");

    // Enemy's turn (hWhoseTurn = 1 → ClearHyperBeam targets wPlayerBattleStatus2)
    h.write_mem(sym_addr("hWhoseTurn"), 0x01);

    // Set NEEDS_TO_RECHARGE on the player (simulating Hyper Beam recharge)
    h.write_mem(sym_addr("wPlayerBattleStatus2"), 1 << NEEDS_TO_RECHARGE);

    // Run from .freeze2 to .freeze2+3 (after ClearHyperBeam returns)
    h.set_pc(freeze2);
    h.set_sp(0xDFF0);
    h.step_to(freeze2 + 3);

    // NEEDS_TO_RECHARGE should now be cleared
    let status = h.read_mem(sym_addr("wPlayerBattleStatus2"));
    assert_eq!(
        status & (1 << NEEDS_TO_RECHARGE),
        0,
        "NEEDS_TO_RECHARGE should be cleared after enemy freeze (was ${status:02X})"
    );
}

#[test]
fn freeze2_preserves_other_status_bits() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("FreezeBurnParalyzeEffect"));

    let freeze2 = sym_addr("FreezeBurnParalyzeEffect.freeze2");

    h.write_mem(sym_addr("hWhoseTurn"), 0x01);

    // Set NEEDS_TO_RECHARGE + USING_RAGE (bit 6) + HAS_SUBSTITUTE_UP (bit 4)
    let initial = (1 << NEEDS_TO_RECHARGE) | (1 << 6) | (1 << 4);
    h.write_mem(sym_addr("wPlayerBattleStatus2"), initial);

    h.set_pc(freeze2);
    h.set_sp(0xDFF0);
    h.step_to(freeze2 + 3);

    let status = h.read_mem(sym_addr("wPlayerBattleStatus2"));
    // NEEDS_TO_RECHARGE cleared, but other bits preserved
    let expected = (1 << 6) | (1 << 4);
    assert_eq!(
        status, expected,
        "Only NEEDS_TO_RECHARGE should be cleared; expected ${expected:02X}, got ${status:02X}"
    );
}

#[test]
fn freeze2_no_recharge_is_harmless() {
    // When NEEDS_TO_RECHARGE is NOT set, ClearHyperBeam should be a no-op
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("FreezeBurnParalyzeEffect"));

    let freeze2 = sym_addr("FreezeBurnParalyzeEffect.freeze2");

    h.write_mem(sym_addr("hWhoseTurn"), 0x01);
    h.write_mem(sym_addr("wPlayerBattleStatus2"), 0x00);

    h.set_pc(freeze2);
    h.set_sp(0xDFF0);
    h.step_to(freeze2 + 3);

    let status = h.read_mem(sym_addr("wPlayerBattleStatus2"));
    assert_eq!(
        status, 0x00,
        "Status should remain 0 when no recharge was set"
    );
}

// ─── Symmetry: player freezes enemy also clears recharge ────────────

#[test]
fn freeze1_clears_enemy_needs_to_recharge() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("FreezeBurnParalyzeEffect"));

    let freeze1 = sym_addr("FreezeBurnParalyzeEffect.freeze1");

    // Player's turn (hWhoseTurn = 0 → ClearHyperBeam targets wEnemyBattleStatus2)
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyBattleStatus2"), 1 << NEEDS_TO_RECHARGE);

    h.set_pc(freeze1);
    h.set_sp(0xDFF0);
    h.step_to(freeze1 + 3);

    let status = h.read_mem(sym_addr("wEnemyBattleStatus2"));
    assert_eq!(
        status & (1 << NEEDS_TO_RECHARGE),
        0,
        "NEEDS_TO_RECHARGE should be cleared when player freezes enemy"
    );
}
