//! Emulator-based tests for the Drain/Dream Eater vs Substitute fix.
//!
//! The Swift fix broke MoveHitTest's substitute check for HP-draining moves:
//! `CheckTargetSubstitute` overwrites register A with `hWhoseTurn` (0 or 1),
//! so the subsequent `cp DRAIN_HP_EFFECT` / `cp DREAM_EATER_EFFECT` can never
//! match. Our fix adds `ld a, [de]` to reload the move effect after the call.
//!
//! Test approach: run from `.swiftCheck` with WRAM set up for known move effects
//! and substitute status, then check whether `.moveMissed` or
//! `.checkForDigOrFlyStatus` is reached.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Move effect constants.
const DRAIN_HP_EFFECT: u8 = 0x03;
const DREAM_EATER_EFFECT: u8 = 0x08;
const SWIFT_EFFECT: u8 = 0x11;
const NO_ADDITIONAL_EFFECT: u8 = 0x00;

/// HAS_SUBSTITUTE_UP = bit 4.
const HAS_SUBSTITUTE_UP_BIT: u8 = 4;

/// A safe WRAM trap address.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Outcome of the substitute/drain check in MoveHitTest.
#[derive(Debug, PartialEq)]
enum DrainCheckResult {
    /// Reached .moveMissed — move was correctly rejected.
    Missed,
    /// Reached .checkForDigOrFlyStatus — move was NOT rejected by drain check.
    Continued,
    /// Swift returned immediately (ret z).
    Returned,
}

/// Run from `.swiftCheck` with the given move effect and substitute status.
///
/// For player's turn: DE = wPlayerMoveEffect, HL = wEnemyBattleStatus1.
fn run_swift_check(move_effect: u8, target_has_substitute: bool) -> DrainCheckResult {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("MoveHitTest");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    // Trap for ret (Swift's `ret z`)
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Player's turn
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    // Set move effect in WRAM (DE will point here)
    let w_player_move_effect = sym_addr("wPlayerMoveEffect");
    h.write_mem(w_player_move_effect, move_effect);

    // Set substitute status on the enemy (target)
    let status2 = if target_has_substitute {
        1 << HAS_SUBSTITUTE_UP_BIT
    } else {
        0
    };
    h.write_mem(sym_addr("wEnemyBattleStatus2"), status2);

    // Clear INVULNERABLE and other bits in enemy BattleStatus1
    h.write_mem(sym_addr("wEnemyBattleStatus1"), 0x00);

    // At .swiftCheck: DE = &wPlayerMoveEffect, HL = &wEnemyBattleStatus1
    h.gb.cpu().set_de(w_player_move_effect);
    h.gb.cpu().set_hl(sym_addr("wEnemyBattleStatus1"));

    h.set_pc(sym_addr("MoveHitTest.swiftCheck"));

    // Step until we reach one of the decision points.
    let move_missed = sym_addr("MoveHitTest.moveMissed");
    let check_dig_fly = sym_addr("MoveHitTest.checkForDigOrFlyStatus");

    for _ in 0..500 {
        let pc = h.pc();
        if pc == move_missed {
            return DrainCheckResult::Missed;
        }
        if pc == check_dig_fly {
            return DrainCheckResult::Continued;
        }
        if pc == TRAP_ADDR {
            return DrainCheckResult::Returned;
        }
        h.gb.clock();
    }
    panic!(
        "MoveHitTest did not reach a decision point within 500 instructions (PC=${:04X})",
        h.pc()
    );
}

// ─── Drain/Dream Eater vs Substitute fix ───────────────────────────

#[test]
fn drain_move_misses_against_substitute() {
    // DRAIN_HP_EFFECT + target has substitute → should miss
    let result = run_swift_check(DRAIN_HP_EFFECT, true);
    assert_eq!(
        result,
        DrainCheckResult::Missed,
        "Drain move should miss against a substitute"
    );
}

#[test]
fn dream_eater_misses_against_substitute() {
    // DREAM_EATER_EFFECT + target has substitute → should miss
    let result = run_swift_check(DREAM_EATER_EFFECT, true);
    assert_eq!(
        result,
        DrainCheckResult::Missed,
        "Dream Eater should miss against a substitute"
    );
}

#[test]
fn drain_move_continues_without_substitute() {
    // DRAIN_HP_EFFECT + no substitute → should continue to dig/fly check
    let result = run_swift_check(DRAIN_HP_EFFECT, false);
    assert_eq!(
        result,
        DrainCheckResult::Continued,
        "Drain move should continue normally without a substitute"
    );
}

#[test]
fn dream_eater_continues_without_substitute() {
    // DREAM_EATER_EFFECT + no substitute → should continue
    let result = run_swift_check(DREAM_EATER_EFFECT, false);
    assert_eq!(
        result,
        DrainCheckResult::Continued,
        "Dream Eater should continue normally without a substitute"
    );
}

#[test]
fn normal_move_continues_with_substitute() {
    // NO_ADDITIONAL_EFFECT + substitute → should continue (not a drain move)
    let result = run_swift_check(NO_ADDITIONAL_EFFECT, true);
    assert_eq!(
        result,
        DrainCheckResult::Continued,
        "Non-drain move should continue even against a substitute"
    );
}

#[test]
fn swift_returns_immediately() {
    // SWIFT_EFFECT → should return (ret z) regardless of substitute
    let result = run_swift_check(SWIFT_EFFECT, true);
    assert_eq!(
        result,
        DrainCheckResult::Returned,
        "Swift should return immediately (always hits)"
    );
}
