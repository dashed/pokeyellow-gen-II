//! Emulator-based tests for the Counter stale damage fix.
//!
//! Counter doubles `wDamage` and deals it back. Two bugs:
//!
//! 1. **Stale damage from switch/battle**: `wDamage` was never cleared on
//!    switch-in or between battles, so Counter could use stale damage.
//!    Fix: clear `wDamage` in `InitBattleVariables`, `EnemySendOutFirstMon`,
//!    and `SendOutMon`.
//!
//! 2. **Own-damage reflection**: `wDamage` is shared by both sides. When the
//!    Counter target can't move (frozen, asleep, fully paralyzed, confused
//!    self-hit), `wDamage` retains stale or self-inflicted damage, allowing
//!    Counter to reflect the user's own damage instead of the opponent's.
//!    Fix: clear `wDamage` in sleep, frozen, and MonHurtItselfOrFullyParalysed
//!    paths on both player and enemy sides.
//!
//! Reference: https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Counter_glitches
//!
//! Test approach: run HandleCounterMove (or its `.counterableType` sub-path)
//! with controlled `wDamage` and move selection values, then check whether
//! Counter hit (wMoveMissed=0, wDamage doubled) or missed (wMoveMissed=1).
//! We stop at the `call MoveHitTest` instruction for "hit" cases
//! since MoveHitTest requires full battle state that's out of scope here.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Move constants.
const COUNTER: u8 = 0x44;
const TACKLE: u8 = 0x21; // Normal-type, base power > 0
const NORMAL_TYPE: u8 = 0x00;
const FIGHTING_TYPE: u8 = 0x01;
const FIRE_TYPE: u8 = 0x14;

/// A safe WRAM trap address.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Result of running HandleCounterMove.
#[derive(Debug, PartialEq)]
enum CounterResult {
    /// Counter hit — wMoveMissed=0, wDamage was doubled.
    Hit { damage: u16 },
    /// Counter missed — wMoveMissed=1 or wDamage was 0.
    Missed,
}

/// Run HandleCounterMove.counterableType with the given wDamage value.
///
/// This enters AFTER the move-type checks, so it only tests the
/// wDamage > 0 check and the doubling logic.
fn run_counterable_type(damage_hi: u8, damage_lo: u8) -> CounterResult {
    let w_damage = sym_addr("wDamage");
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("HandleCounterMove"));

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Set wDamage (big-endian)
    h.write_mem(w_damage, damage_hi);
    h.write_mem(w_damage + 1, damage_lo);

    // Initialize wMoveMissed (HandleCounterMove sets it to 1 at entry,
    // but .counterableType is past that point — we set it to 1 to match)
    h.write_mem(sym_addr("wMoveMissed"), 0x01);

    h.set_pc(sym_addr("HandleCounterMove.counterableType"));

    // 4 bytes after .noCarry = `call MoveHitTest` instruction (no direct label)
    let call_move_hit_test = sym_addr("HandleCounterMove.noCarry") + 4;

    for _ in 0..300 {
        let pc = h.pc();
        // Miss path: function returns via `ret z` → lands at TRAP_ADDR
        if pc == TRAP_ADDR {
            return CounterResult::Missed;
        }
        // Hit path: reaches `call MoveHitTest` — damage already doubled
        if pc == call_move_hit_test {
            let hi = h.read_mem(w_damage);
            let lo = h.read_mem(w_damage + 1);
            return CounterResult::Hit {
                damage: (hi as u16) << 8 | lo as u16,
            };
        }
        h.gb.clock();
    }
    panic!(
        "HandleCounterMove did not reach a decision point within 300 instructions (PC=${:04X})",
        h.pc()
    );
}

/// Run the full HandleCounterMove with controlled state.
fn run_full_counter(
    whose_turn: u8,
    user_move: u8,
    opponent_move: u8,
    opponent_power: u8,
    opponent_type: u8,
    damage_hi: u8,
    damage_lo: u8,
) -> CounterResult {
    let w_damage = sym_addr("wDamage");
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("HandleCounterMove"));

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    h.write_mem(sym_addr("hWhoseTurn"), whose_turn);

    if whose_turn == 0 {
        // Player's turn: player uses Counter, checks enemy's used move
        h.write_mem(sym_addr("wPlayerSelectedMove"), user_move);
        h.write_mem(sym_addr("wEnemyUsedMove"), opponent_move); // fix: was wEnemySelectedMove
        h.write_mem(sym_addr("wEnemyMovePower"), opponent_power);
        h.write_mem(sym_addr("wEnemyMoveType"), opponent_type);
    } else {
        // Enemy's turn: enemy uses Counter, checks player's used move
        h.write_mem(sym_addr("wEnemySelectedMove"), user_move);
        h.write_mem(sym_addr("wPlayerUsedMove"), opponent_move); // fix: was wPlayerSelectedMove
        h.write_mem(sym_addr("wPlayerMovePower"), opponent_power);
        h.write_mem(sym_addr("wPlayerMoveType"), opponent_type);
    }

    h.write_mem(w_damage, damage_hi);
    h.write_mem(w_damage + 1, damage_lo);
    h.write_mem(sym_addr("wMoveMissed"), 0x00);

    h.set_pc(sym_addr("HandleCounterMove"));

    // 4 bytes after .noCarry = `call MoveHitTest` instruction (no direct label)
    let call_move_hit_test = sym_addr("HandleCounterMove.noCarry") + 4;

    for _ in 0..500 {
        let pc = h.pc();
        // Miss paths: various `ret z` / `ret nz` / `ret` → land at TRAP_ADDR
        if pc == TRAP_ADDR {
            let missed = h.read_mem(sym_addr("wMoveMissed"));
            if missed != 0 {
                return CounterResult::Missed;
            }
            // ret nz at the very start (not using Counter) also lands here
            return CounterResult::Missed;
        }
        // Hit path: reaches `call MoveHitTest`
        if pc == call_move_hit_test {
            let hi = h.read_mem(w_damage);
            let lo = h.read_mem(w_damage + 1);
            return CounterResult::Hit {
                damage: (hi as u16) << 8 | lo as u16,
            };
        }
        h.gb.clock();
    }
    panic!(
        "HandleCounterMove did not reach a decision point within 500 instructions (PC=${:04X})",
        h.pc()
    );
}

// ─── Counter stale damage fix tests ──────────────────────────────────

#[test]
fn counter_misses_when_damage_is_zero() {
    // After a switch, wDamage should be 0 → Counter misses
    let result = run_counterable_type(0x00, 0x00);
    assert_eq!(
        result,
        CounterResult::Missed,
        "Counter should miss when wDamage is 0 (post-switch)"
    );
}

#[test]
fn counter_doubles_nonzero_damage() {
    // Normal case: wDamage = 50 → Counter deals 100
    let result = run_counterable_type(0x00, 50);
    assert_eq!(
        result,
        CounterResult::Hit { damage: 100 },
        "Counter should double wDamage (50 → 100)"
    );
}

#[test]
fn counter_doubles_large_damage() {
    // wDamage = 200 → Counter deals 400
    let result = run_counterable_type(0x00, 200);
    assert_eq!(
        result,
        CounterResult::Hit { damage: 400 },
        "Counter should double wDamage (200 → 400)"
    );
}

#[test]
fn counter_caps_at_ffff() {
    // wDamage = $8000 → doubled = $10000, capped to $FFFF
    let result = run_counterable_type(0x80, 0x00);
    assert_eq!(
        result,
        CounterResult::Hit { damage: 0xFFFF },
        "Counter should cap damage at $FFFF on overflow"
    );
}

#[test]
fn full_counter_hits_against_normal_type() {
    // Player uses Counter, enemy last used Tackle (Normal, power > 0), wDamage = 30
    let result = run_full_counter(0, COUNTER, TACKLE, 35, NORMAL_TYPE, 0x00, 30);
    assert_eq!(
        result,
        CounterResult::Hit { damage: 60 },
        "Counter should hit and double damage from Normal-type attack"
    );
}

#[test]
fn full_counter_hits_against_fighting_type() {
    // Player uses Counter, enemy last used a Fighting-type move
    let result = run_full_counter(0, COUNTER, TACKLE, 40, FIGHTING_TYPE, 0x00, 25);
    assert_eq!(
        result,
        CounterResult::Hit { damage: 50 },
        "Counter should hit against Fighting-type attack"
    );
}

#[test]
fn full_counter_misses_against_fire_type() {
    // Player uses Counter, enemy last used a Fire-type move → misses
    let result = run_full_counter(0, COUNTER, TACKLE, 40, FIRE_TYPE, 0x00, 50);
    assert_eq!(
        result,
        CounterResult::Missed,
        "Counter should miss against non-Normal/Fighting type"
    );
}

#[test]
fn full_counter_misses_when_damage_zero_after_switch() {
    // Post-switch scenario: enemy used Normal-type move previously,
    // but wDamage was cleared on switch → Counter misses
    let result = run_full_counter(0, COUNTER, TACKLE, 35, NORMAL_TYPE, 0x00, 0x00);
    assert_eq!(
        result,
        CounterResult::Missed,
        "Counter should miss when wDamage is 0 (cleared after switch)"
    );
}

#[test]
fn full_counter_misses_when_opponent_used_counter() {
    // Both sides used Counter → misses
    let result = run_full_counter(0, COUNTER, COUNTER, 1, FIGHTING_TYPE, 0x00, 50);
    assert_eq!(
        result,
        CounterResult::Missed,
        "Counter should miss if opponent's last move was also Counter"
    );
}

#[test]
fn full_counter_misses_when_opponent_has_zero_power() {
    // Opponent's move has 0 base power (status move) → misses
    let result = run_full_counter(0, COUNTER, TACKLE, 0, NORMAL_TYPE, 0x00, 50);
    assert_eq!(
        result,
        CounterResult::Missed,
        "Counter should miss if opponent's last move has 0 base power"
    );
}

// ─── Counter own-damage reflection fix: ROM byte tests ──────────────

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Check that `ld [wDamage], a` ($EA lo hi) appears between two addresses.
fn has_ld_w_damage(h: &mut TestHarness, start: u16, end: u16) -> bool {
    let w_damage = sym_addr("wDamage");
    let lo = (w_damage & 0xFF) as u8;
    let hi = (w_damage >> 8) as u8;
    for addr in start..end {
        if rom(h, addr) == 0xEA && rom(h, addr + 1) == lo && rom(h, addr + 2) == hi {
            return true;
        }
    }
    false
}

#[test]
fn player_sleep_path_clears_w_damage() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckPlayerStatusConditions"));
    let sleep_done = sym_addr("CheckPlayerStatusConditions.sleepDone");
    let frozen = sym_addr("CheckPlayerStatusConditions.FrozenCheck");
    assert!(
        has_ld_w_damage(&mut h, sleep_done, frozen),
        "Player sleep path should clear wDamage between .sleepDone and .FrozenCheck"
    );
}

#[test]
fn player_frozen_path_clears_w_damage() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckPlayerStatusConditions"));
    let frozen = sym_addr("CheckPlayerStatusConditions.FrozenCheck");
    let held = sym_addr("CheckPlayerStatusConditions.HeldInPlaceCheck");
    assert!(
        has_ld_w_damage(&mut h, frozen, held),
        "Player frozen path should clear wDamage between .FrozenCheck and .HeldInPlaceCheck"
    );
}

#[test]
fn player_paralysis_path_clears_w_damage() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckPlayerStatusConditions"));
    let not_fly = sym_addr("CheckPlayerStatusConditions.NotFlyOrChargeEffect");
    let bide = sym_addr("CheckPlayerStatusConditions.BideCheck");
    assert!(
        has_ld_w_damage(&mut h, not_fly, bide),
        "Player MonHurtItselfOrFullyParalysed path should clear wDamage"
    );
}

#[test]
fn enemy_sleep_path_clears_w_damage() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckEnemyStatusConditions"));
    let sleep_done = sym_addr("CheckEnemyStatusConditions.sleepDone");
    let frozen = sym_addr("CheckEnemyStatusConditions.checkIfFrozen");
    assert!(
        has_ld_w_damage(&mut h, sleep_done, frozen),
        "Enemy sleep path should clear wDamage between .sleepDone and .checkIfFrozen"
    );
}

#[test]
fn enemy_frozen_path_clears_w_damage() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckEnemyStatusConditions"));
    let frozen = sym_addr("CheckEnemyStatusConditions.checkIfFrozen");
    let trapped = sym_addr("CheckEnemyStatusConditions.checkIfTrapped");
    assert!(
        has_ld_w_damage(&mut h, frozen, trapped),
        "Enemy frozen path should clear wDamage between .checkIfFrozen and .checkIfTrapped"
    );
}

#[test]
fn enemy_paralysis_path_clears_w_damage() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckEnemyStatusConditions"));
    let not_fly = sym_addr("CheckEnemyStatusConditions.notFlyOrChargeEffect");
    let bide = sym_addr("CheckEnemyStatusConditions.checkIfUsingBide");
    assert!(
        has_ld_w_damage(&mut h, not_fly, bide),
        "Enemy monHurtItselfOrFullyParalysed path should clear wDamage"
    );
}

// ─── Counter link desync fix: ROM byte tests ────────────────────────

/// Check that `ld hl, addr` ($21 lo hi) loads the expected address.
fn ld_hl_target(h: &mut TestHarness, addr: u16) -> u16 {
    assert_eq!(rom(h, addr), 0x21, "expected ld hl, nn at {addr:#06x}");
    let lo = rom(h, addr + 1) as u16;
    let hi = rom(h, addr + 2) as u16;
    (hi << 8) | lo
}

#[test]
fn player_turn_checks_enemy_used_move_not_selected() {
    // When player uses Counter (hWhoseTurn=0), HandleCounterMove should
    // load HL with wEnemyUsedMove, NOT wEnemySelectedMove
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("HandleCounterMove"));
    let base = sym_addr("HandleCounterMove");
    // Layout: ldh a,[hWhoseTurn] (2) / and a (1) / ld hl, wEnemyUsedMove (3)
    let ld_hl_addr = base + 3; // offset past ldh + and a
    let target = ld_hl_target(&mut h, ld_hl_addr);
    assert_eq!(
        target,
        sym_addr("wEnemyUsedMove"),
        "Player turn: HL should load wEnemyUsedMove, not wEnemySelectedMove"
    );
    assert_ne!(
        target,
        sym_addr("wEnemySelectedMove"),
        "Player turn: HL must NOT be wEnemySelectedMove (cursor-polluted)"
    );
}

#[test]
fn enemy_turn_checks_player_used_move_not_selected() {
    // When enemy uses Counter (hWhoseTurn=1), HandleCounterMove should
    // load HL with wPlayerUsedMove, NOT wPlayerSelectedMove
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("HandleCounterMove"));
    let base = sym_addr("HandleCounterMove");
    // Layout: ldh (2) / and a (1) / ld hl (3) / ld de (3) / ld a,[nn] (3) / jr z (2)
    // = offset 14 for the enemy's ld hl
    let ld_hl_addr = base + 14;
    let target = ld_hl_target(&mut h, ld_hl_addr);
    assert_eq!(
        target,
        sym_addr("wPlayerUsedMove"),
        "Enemy turn: HL should load wPlayerUsedMove, not wPlayerSelectedMove"
    );
    assert_ne!(
        target,
        sym_addr("wPlayerSelectedMove"),
        "Enemy turn: HL must NOT be wPlayerSelectedMove (cursor-polluted)"
    );
}
