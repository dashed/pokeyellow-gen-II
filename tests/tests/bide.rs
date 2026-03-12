//! Emulator-based tests for two Bide bug fixes:
//!
//! 1. **Accumulated damage clearing** (link desync fix):
//!    `FaintEnemyPokemon.wild` only cleared the high byte of
//!    `wPlayerBideAccumulatedDamage`, leaving the low byte intact. This meant
//!    accumulated damage became `damage % 256` instead of 0. In link battles,
//!    the other Game Boy's `RemoveFaintedPlayerMon` correctly clears both bytes,
//!    causing a desync.
//!
//! 2. **Bide vs invulnerable targets** (Fly/Dig fix):
//!    Bide's unleash path skips `MoveHitTest` entirely, jumping straight to
//!    `HandleIfPlayerMoveMissed`/`HandleIfEnemyMoveMissed`. This bypasses the
//!    INVULNERABLE check that normal moves go through, so Bide always hits
//!    opponents using Fly or Dig. Fix adds an explicit INVULNERABLE check
//!    before the jump.
//!    Reference: https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Bide_errors

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

// ─── Bide vs invulnerable target (Fly/Dig) fix ──────────────────

const INVULNERABLE_BIT: u8 = 6; // bit 6 of BattleStatus1

/// Run the player-side Bide unleash invulnerability check.
/// Enters at `CheckPlayerStatusConditions.next` (the clearing/check portion after
/// damage doubling), pre-sets enemy BattleStatus1, and returns wMoveMissed.
fn run_player_bide_invulnerable_check(enemy_status: u8) -> u8 {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("CheckPlayerStatusConditions"));

    // HL must point to wPlayerBideAccumulatedDamage at .next entry
    // (the preceding code leaves HL there after `ld a, [hld]`)
    let w_player_bide_damage = sym_addr("wPlayerBideAccumulatedDamage");
    h.gb.cpu().set_hl(w_player_bide_damage);

    // Pre-set memory
    h.write_mem(sym_addr("wEnemyBattleStatus1"), enemy_status);
    h.write_mem(sym_addr("wMoveMissed"), 0x00);
    h.write_mem(w_player_bide_damage, 0x00);
    h.write_mem(w_player_bide_damage + 1, 0x00);

    h.set_sp(0xDFF0);
    h.set_pc(sym_addr("CheckPlayerStatusConditions.next"));

    // Step through the fix code:
    //   xor a           (1)
    //   ld [hli], a     (2)
    //   ld [hl], a      (3)
    //   ld a, BIDE      (4)
    //   ld [wPlayerMoveNum], a  (5)
    //   ld a, [wEnemyBattleStatus1]  (6)
    //   bit INVULNERABLE, a          (7)
    //   jr z, .playerBideNotBlocked  (8)
    // If INVULNERABLE set:
    //   ld a, 1                      (9)
    //   ld [wMoveMissed], a          (10)
    let steps = if enemy_status & (1 << INVULNERABLE_BIT) != 0 {
        10
    } else {
        8
    };
    for _ in 0..steps {
        h.gb.clock();
    }

    h.read_mem(sym_addr("wMoveMissed"))
}

/// Run the enemy-side Bide unleash invulnerability check.
/// Enters at `CheckEnemyStatusConditions.next`, pre-sets player BattleStatus1,
/// and returns wMoveMissed.
fn run_enemy_bide_invulnerable_check(player_status: u8) -> u8 {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("CheckEnemyStatusConditions"));

    // HL must point to wEnemyBideAccumulatedDamage at .next entry
    let w_enemy_bide_damage = sym_addr("wEnemyBideAccumulatedDamage");
    h.gb.cpu().set_hl(w_enemy_bide_damage);

    // Pre-set memory
    h.write_mem(sym_addr("wPlayerBattleStatus1"), player_status);
    h.write_mem(sym_addr("wMoveMissed"), 0x00);
    h.write_mem(w_enemy_bide_damage, 0x00);
    h.write_mem(w_enemy_bide_damage + 1, 0x00);

    h.set_sp(0xDFF0);
    h.set_pc(sym_addr("CheckEnemyStatusConditions.next"));

    let steps = if player_status & (1 << INVULNERABLE_BIT) != 0 {
        10
    } else {
        8
    };
    for _ in 0..steps {
        h.gb.clock();
    }

    h.read_mem(sym_addr("wMoveMissed"))
}

#[test]
fn player_bide_misses_invulnerable_enemy() {
    // Enemy is using Fly/Dig — Bide should miss
    let missed = run_player_bide_invulnerable_check(1 << INVULNERABLE_BIT);
    assert_eq!(
        missed, 1,
        "Player Bide should miss when enemy has INVULNERABLE set"
    );
}

#[test]
fn player_bide_hits_vulnerable_enemy() {
    // Enemy is not invulnerable — Bide should hit
    let missed = run_player_bide_invulnerable_check(0x00);
    assert_eq!(
        missed, 0,
        "Player Bide should hit when enemy is vulnerable"
    );
}

#[test]
fn player_bide_checks_only_invulnerable_bit() {
    // Other status bits set but NOT invulnerable — should still hit
    let status = 0xFF & !(1 << INVULNERABLE_BIT); // all bits except INVULNERABLE
    let missed = run_player_bide_invulnerable_check(status);
    assert_eq!(
        missed, 0,
        "Player Bide should hit when INVULNERABLE bit is clear (other bits: {status:#04x})"
    );
}

#[test]
fn enemy_bide_misses_invulnerable_player() {
    // Player is using Fly/Dig — enemy Bide should miss
    let missed = run_enemy_bide_invulnerable_check(1 << INVULNERABLE_BIT);
    assert_eq!(
        missed, 1,
        "Enemy Bide should miss when player has INVULNERABLE set"
    );
}

#[test]
fn enemy_bide_hits_vulnerable_player() {
    // Player is not invulnerable — enemy Bide should hit
    let missed = run_enemy_bide_invulnerable_check(0x00);
    assert_eq!(
        missed, 0,
        "Enemy Bide should hit when player is vulnerable"
    );
}

#[test]
fn enemy_bide_checks_only_invulnerable_bit() {
    // Other status bits set but NOT invulnerable — should still hit
    let status = 0xFF & !(1 << INVULNERABLE_BIT);
    let missed = run_enemy_bide_invulnerable_check(status);
    assert_eq!(
        missed, 0,
        "Enemy Bide should hit when INVULNERABLE bit is clear (other bits: {status:#04x})"
    );
}
