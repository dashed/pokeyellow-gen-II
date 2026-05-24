//! Emulator-based tests for the Psywave link battle desync fix.
//!
//! The bug: the enemy's Psywave code (`ApplyAttackToPlayerPokemon`) accepted
//! 0 as valid damage, while the player's code (`ApplyAttackToEnemyPokemon`)
//! rejected it. In link battles, both Game Boys run the Psywave calculation
//! using a shared RNG list. If the RNG produces 0, one side retries (consuming
//! an extra RNG value) while the other doesn't — desyncing all subsequent RNG.
//!
//! The fix adds `and a / jr z, .loop` to the enemy path, matching the player.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// LINK_STATE_BATTLING = $04.
const LINK_STATE_BATTLING: u8 = 0x04;

/// A safe WRAM address for the return trap.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Set up the harness for a Psywave test in link battle mode.
fn setup_psywave_fixture(rng_list: &[u8; 10]) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("ApplyAttackToEnemyPokemon"));

    // Trap for ret
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);

    // Enable link battle mode
    h.write_mem(sym_addr("wLinkState"), LINK_STATE_BATTLING);

    // Fill the RNG list
    let w_link_battle_rng_list = sym_addr("wLinkBattleRandomNumberList");
    for (i, &val) in rng_list.iter().enumerate() {
        h.write_mem(w_link_battle_rng_list + i as u16, val);
    }
    h.write_mem(sym_addr("wLinkBattleRandomNumberListIndex"), 0x00);

    h
}

/// Run the Psywave .loop at `loop_addr`, stopping at `store_addr`.
/// B = max_damage (level * 1.5). Returns (damage_in_b, rng_index_after).
fn run_psywave_loop(
    h: &mut TestHarness,
    loop_addr: u16,
    store_addr: u16,
    max_damage: u8,
) -> (u8, u8) {
    h.set_b(max_damage);
    h.set_pc(loop_addr);

    // Step until we reach .storeDamage
    for _ in 0..500 {
        if h.pc() == store_addr {
            let damage = h.b();
            let rng_idx = h.read_mem(sym_addr("wLinkBattleRandomNumberListIndex"));
            return (damage, rng_idx);
        }
        h.gb.clock();
    }
    panic!(
        "Psywave loop did not reach .storeDamage within 500 instructions (PC=${:04X})",
        h.pc()
    );
}

// ─── Enemy Psywave: 0 damage is rejected (the fix) ──────────────

#[test]
fn enemy_psywave_rejects_zero_damage() {
    // RNG list: [0, 30, ...]. B=75 (level 50). Should skip 0, use 30.
    let list: [u8; 10] = [0, 30, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_psywave_fixture(&list);

    let (damage, rng_idx) = run_psywave_loop(
        &mut h,
        sym_addr("ApplyAttackToPlayerPokemon.loop"),
        sym_addr("ApplyAttackToPlayerPokemon.storeDamage"),
        75,
    );
    assert_eq!(damage, 30, "Enemy Psywave should skip 0 and use 30");
    assert_eq!(
        rng_idx, 2,
        "Should consume 2 RNG values (0 rejected, 30 accepted)"
    );
}

#[test]
fn enemy_psywave_accepts_valid_nonzero() {
    // RNG list: [42, ...]. B=75. Should accept 42 immediately.
    let list: [u8; 10] = [42, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_psywave_fixture(&list);

    let (damage, rng_idx) = run_psywave_loop(
        &mut h,
        sym_addr("ApplyAttackToPlayerPokemon.loop"),
        sym_addr("ApplyAttackToPlayerPokemon.storeDamage"),
        75,
    );
    assert_eq!(damage, 42, "Enemy Psywave should accept 42");
    assert_eq!(rng_idx, 1, "Should consume 1 RNG value");
}

#[test]
fn enemy_psywave_rejects_value_at_max() {
    // RNG list: [75, 50, ...]. B=75. 75 >= 75 → reject, 50 < 75 → accept.
    let list: [u8; 10] = [75, 50, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_psywave_fixture(&list);

    let (damage, rng_idx) = run_psywave_loop(
        &mut h,
        sym_addr("ApplyAttackToPlayerPokemon.loop"),
        sym_addr("ApplyAttackToPlayerPokemon.storeDamage"),
        75,
    );
    assert_eq!(damage, 50, "Should reject 75 (>= max) and use 50");
    assert_eq!(rng_idx, 2, "Should consume 2 RNG values");
}

#[test]
fn enemy_psywave_rejects_value_above_max() {
    // RNG list: [200, 1, ...]. B=75. 200 >= 75 → reject, 1 < 75 → accept.
    let list: [u8; 10] = [200, 1, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_psywave_fixture(&list);

    let (damage, rng_idx) = run_psywave_loop(
        &mut h,
        sym_addr("ApplyAttackToPlayerPokemon.loop"),
        sym_addr("ApplyAttackToPlayerPokemon.storeDamage"),
        75,
    );
    assert_eq!(damage, 1, "Should reject 200 (>= max) and use 1");
    assert_eq!(rng_idx, 2, "Should consume 2 RNG values");
}

// ─── Player Psywave: verify same behavior ────────────────────────

#[test]
fn player_psywave_rejects_zero_damage() {
    // Same scenario as enemy test — player should also reject 0.
    let list: [u8; 10] = [0, 30, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_psywave_fixture(&list);

    let (damage, rng_idx) = run_psywave_loop(
        &mut h,
        sym_addr("ApplyAttackToEnemyPokemon.loop"),
        sym_addr("ApplyAttackToEnemyPokemon.storeDamage"),
        75,
    );
    assert_eq!(damage, 30, "Player Psywave should skip 0 and use 30");
    assert_eq!(rng_idx, 2, "Should consume 2 RNG values");
}

// ─── Link sync: both sides consume same RNG count ────────────────

#[test]
fn both_sides_consume_same_rng_for_zero() {
    // THE DESYNC SCENARIO: RNG produces 0, then a valid value.
    // Both player and enemy should reject 0 and consume 2 RNG values.
    let list: [u8; 10] = [0, 25, 0, 0, 0, 0, 0, 0, 0, 0];

    // Run player side
    let mut h1 = setup_psywave_fixture(&list);
    let (player_dmg, player_idx) = run_psywave_loop(
        &mut h1,
        sym_addr("ApplyAttackToEnemyPokemon.loop"),
        sym_addr("ApplyAttackToEnemyPokemon.storeDamage"),
        75,
    );

    // Run enemy side
    let mut h2 = setup_psywave_fixture(&list);
    let (enemy_dmg, enemy_idx) = run_psywave_loop(
        &mut h2,
        sym_addr("ApplyAttackToPlayerPokemon.loop"),
        sym_addr("ApplyAttackToPlayerPokemon.storeDamage"),
        75,
    );

    assert_eq!(
        player_idx, enemy_idx,
        "Both sides must consume the same number of RNG values (player={player_idx}, enemy={enemy_idx})"
    );
    assert_eq!(
        player_dmg, enemy_dmg,
        "Both sides must produce the same damage (player={player_dmg}, enemy={enemy_dmg})"
    );
}

#[test]
fn both_sides_consume_same_rng_for_over_max() {
    // RNG: [200, 0, 50]. Both reject 200 (>= 75), both reject 0, both accept 50.
    let list: [u8; 10] = [200, 0, 50, 0, 0, 0, 0, 0, 0, 0];

    let mut h1 = setup_psywave_fixture(&list);
    let (player_dmg, player_idx) = run_psywave_loop(
        &mut h1,
        sym_addr("ApplyAttackToEnemyPokemon.loop"),
        sym_addr("ApplyAttackToEnemyPokemon.storeDamage"),
        75,
    );

    let mut h2 = setup_psywave_fixture(&list);
    let (enemy_dmg, enemy_idx) = run_psywave_loop(
        &mut h2,
        sym_addr("ApplyAttackToPlayerPokemon.loop"),
        sym_addr("ApplyAttackToPlayerPokemon.storeDamage"),
        75,
    );

    assert_eq!(
        player_idx, enemy_idx,
        "Both sides must consume same RNG count (player={player_idx}, enemy={enemy_idx})"
    );
    assert_eq!(
        player_dmg, enemy_dmg,
        "Both sides must produce same damage (player={player_dmg}, enemy={enemy_dmg})"
    );
}

#[test]
fn enemy_psywave_minimum_damage_is_one() {
    // RNG: [1, ...]. B=75. 1 is valid (>0 and <75) → damage = 1.
    let list: [u8; 10] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_psywave_fixture(&list);

    let (damage, _) = run_psywave_loop(
        &mut h,
        sym_addr("ApplyAttackToPlayerPokemon.loop"),
        sym_addr("ApplyAttackToPlayerPokemon.storeDamage"),
        75,
    );
    assert_eq!(damage, 1, "Minimum Psywave damage should be 1");
}
