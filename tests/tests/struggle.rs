//! Emulator-based tests confirming the Struggle PP Ups fix (already fixed in Yellow).
//!
//! In Red/Blue, `AnyMoveToSelect` checked raw PP bytes without masking PP Up
//! bits. If any move had PP Ups (upper 2 bits set) but 0 actual PP, the check
//! would see non-zero and incorrectly conclude PP was available, preventing
//! Struggle from activating. Yellow fixed this with `and PP_MASK` / `and $3f`.
//!
//! These tests confirm the fix works in both code paths:
//! - No disabled move: OR all 4 PP bytes, `and PP_MASK`, check zero
//! - With disabled move: OR non-disabled PP bytes, `and $3f`, check zero

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// STRUGGLE = $A5.
const STRUGGLE: u8 = 0xA5;

/// A safe WRAM address for the return trap.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Result of the AnyMoveToSelect check.
#[derive(Debug, PartialEq)]
enum MoveCheckResult {
    /// Function returned (ret nz) — there are moves with PP available.
    HasPP,
    /// Reached .noMovesLeft — all moves exhausted, Struggle will be used.
    Struggle,
}

/// Run `AnyMoveToSelect` with the given PP values and disabled move state.
///
/// `pp`: the 4 PP bytes for wBattleMonPP (each byte = PP Up bits in upper 2 + PP in lower 6).
/// `disabled_move`: value for wPlayerDisabledMove (0 = none, $10 = move 1, $20 = move 2, etc.).
fn check_any_move_to_select(pp: [u8; 4], disabled_move: u8) -> MoveCheckResult {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("AnyMoveToSelect"));

    let any_move_to_select = sym_addr("AnyMoveToSelect");
    let no_moves_left = sym_addr("AnyMoveToSelect.noMovesLeft");

    // Trap for ret
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Set up WRAM
    let w_battle_mon_pp = sym_addr("wBattleMonPP");
    for (i, &val) in pp.iter().enumerate() {
        h.write_mem(w_battle_mon_pp + i as u16, val);
    }
    h.write_mem(sym_addr("wPlayerDisabledMove"), disabled_move);
    h.write_mem(sym_addr("wPlayerSelectedMove"), 0x00); // clear so we can verify it gets set

    h.set_pc(any_move_to_select);

    // Step until we reach TRAP_ADDR (returned) or .noMovesLeft (Struggle).
    for _ in 0..200 {
        let pc = h.pc();
        if pc == TRAP_ADDR {
            return MoveCheckResult::HasPP;
        }
        if pc == no_moves_left {
            // Verify wPlayerSelectedMove was set to STRUGGLE
            assert_eq!(
                h.read_mem(sym_addr("wPlayerSelectedMove")),
                STRUGGLE,
                "wPlayerSelectedMove should be STRUGGLE when reaching .noMovesLeft"
            );
            return MoveCheckResult::Struggle;
        }
        h.gb.clock();
    }
    panic!(
        "AnyMoveToSelect did not reach a decision point within 200 instructions (PC=${:04X})",
        h.pc()
    );
}

// ─── No disabled move path ────────────────────────────────────────

#[test]
fn all_zero_pp_triggers_struggle() {
    // All moves have 0 PP, no PP Ups → should use Struggle
    let result = check_any_move_to_select([0x00, 0x00, 0x00, 0x00], 0x00);
    assert_eq!(
        result,
        MoveCheckResult::Struggle,
        "All zero PP should trigger Struggle"
    );
}

#[test]
fn pp_ups_only_triggers_struggle() {
    // THE BUG SCENARIO: move 1 has 1 PP Up but 0 actual PP ($40 = 01_000000)
    // Without the fix, OR = $40, non-zero → incorrectly thinks PP available
    // With the fix, $40 AND PP_MASK($3F) = $00 → correctly triggers Struggle
    let result = check_any_move_to_select([0x40, 0x00, 0x00, 0x00], 0x00);
    assert_eq!(
        result,
        MoveCheckResult::Struggle,
        "PP Ups with 0 actual PP should still trigger Struggle"
    );
}

#[test]
fn multiple_pp_ups_only_triggers_struggle() {
    // Multiple moves have PP Ups but 0 actual PP
    // $40 = 1 PP Up, $80 = 2 PP Ups, $C0 = 3 PP Ups
    let result = check_any_move_to_select([0x40, 0x80, 0xC0, 0x00], 0x00);
    assert_eq!(
        result,
        MoveCheckResult::Struggle,
        "Multiple PP Ups with 0 actual PP should still trigger Struggle"
    );
}

#[test]
fn real_pp_does_not_trigger_struggle() {
    // Move 1 has 5 PP, no PP Ups → should NOT use Struggle
    let result = check_any_move_to_select([0x05, 0x00, 0x00, 0x00], 0x00);
    assert_eq!(
        result,
        MoveCheckResult::HasPP,
        "Move with real PP should not trigger Struggle"
    );
}

#[test]
fn pp_ups_with_real_pp_does_not_trigger_struggle() {
    // Move 1 has 1 PP Up + 5 PP ($45 = 01_000101) → should NOT use Struggle
    let result = check_any_move_to_select([0x45, 0x00, 0x00, 0x00], 0x00);
    assert_eq!(
        result,
        MoveCheckResult::HasPP,
        "PP Ups with real PP remaining should not trigger Struggle"
    );
}

// ─── Disabled move path ───────────────────────────────────────────

#[test]
fn disabled_move_pp_ups_only_triggers_struggle() {
    // THE BUG SCENARIO (disabled path): move 1 is disabled and has real PP,
    // move 2 has PP Ups but 0 actual PP, moves 3-4 are empty.
    // Without the fix: OR of non-disabled moves = $40 → thinks PP available
    // With the fix: $40 AND $3F = $00 → correctly triggers Struggle
    let result = check_any_move_to_select([0x05, 0x40, 0x00, 0x00], 0x10);
    assert_eq!(
        result,
        MoveCheckResult::Struggle,
        "Disabled move path: PP Ups with 0 actual PP should trigger Struggle"
    );
}

#[test]
fn disabled_move_another_has_real_pp() {
    // Move 1 is disabled, move 2 has real PP → should NOT use Struggle
    let result = check_any_move_to_select([0x05, 0x03, 0x00, 0x00], 0x10);
    assert_eq!(
        result,
        MoveCheckResult::HasPP,
        "Disabled move path: another move with real PP should not trigger Struggle"
    );
}
