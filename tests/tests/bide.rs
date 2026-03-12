//! Emulator-based tests for the Bide accumulated damage clearing fix.
//!
//! The bug: `FaintEnemyPokemon.wild` only cleared the high byte of
//! `wPlayerBideAccumulatedDamage`, leaving the low byte intact. This meant
//! accumulated damage became `damage % 256` instead of 0. In link battles,
//! the other Game Boy's `RemoveFaintedPlayerMon` correctly clears both bytes,
//! causing a desync.
//!
//! Test approach: enter at `FaintEnemyPokemon.wild`, pre-set
//! `wPlayerBideAccumulatedDamage` to a value with non-zero low byte, step
//! through the clearing code, and verify both bytes are zeroed.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Set up and run the clearing code at `FaintEnemyPokemon.wild`.
/// Returns (high_byte, low_byte) of wPlayerBideAccumulatedDamage after clearing.
fn run_faint_enemy_clearing(damage_hi: u8, damage_lo: u8) -> (u8, u8) {
    let w_player_bide_damage = sym_addr("wPlayerBideAccumulatedDamage");
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("FaintEnemyPokemon"));

    // Pre-set wPlayerBideAccumulatedDamage to a known non-zero value
    h.write_mem(w_player_bide_damage, damage_hi);
    h.write_mem(w_player_bide_damage + 1, damage_lo);

    // Pre-set wPlayerBattleStatus1 (the res instruction reads/writes it)
    h.write_mem(sym_addr("wPlayerBattleStatus1"), 0x00);

    // Stack setup (not strictly needed since we won't ret, but good practice)
    h.set_sp(0xDFF0);

    h.set_pc(sym_addr("FaintEnemyPokemon.wild"));

    // Step through: ld hl + res + xor a + ld hl + ld [hli],a + ld [hl],a = 6 instructions
    // After these 6 instructions, the clearing is complete.
    for _ in 0..6 {
        h.gb.clock();
    }

    let hi = h.read_mem(w_player_bide_damage);
    let lo = h.read_mem(w_player_bide_damage + 1);
    (hi, lo)
}

// ─── Bide damage clearing fix ─────────────────────────────────────

#[test]
fn faint_enemy_clears_both_bide_bytes() {
    // $03 $50 → the bug would leave low byte as $50
    let (hi, lo) = run_faint_enemy_clearing(0x03, 0x50);
    assert_eq!(
        (hi, lo),
        (0, 0),
        "Both bytes of wPlayerBideAccumulatedDamage should be cleared, got ({hi:#04x}, {lo:#04x})"
    );
}

#[test]
fn faint_enemy_clears_damage_ff_ff() {
    // Maximum accumulated damage
    let (hi, lo) = run_faint_enemy_clearing(0xFF, 0xFF);
    assert_eq!(
        (hi, lo),
        (0, 0),
        "wPlayerBideAccumulatedDamage $FFFF should be fully cleared"
    );
}

#[test]
fn faint_enemy_clears_damage_00_80() {
    // Only the low byte is non-zero — the original bug wouldn't affect high byte
    let (hi, lo) = run_faint_enemy_clearing(0x00, 0x80);
    assert_eq!(
        (hi, lo),
        (0, 0),
        "wPlayerBideAccumulatedDamage $0080 should be fully cleared"
    );
}

#[test]
fn faint_enemy_already_zero_stays_zero() {
    // No damage accumulated — clearing should be a no-op
    let (hi, lo) = run_faint_enemy_clearing(0x00, 0x00);
    assert_eq!(
        (hi, lo),
        (0, 0),
        "wPlayerBideAccumulatedDamage $0000 should remain cleared"
    );
}
