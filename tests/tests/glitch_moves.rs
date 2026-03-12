//! ROM byte tests for the glitch move index bounds check fix.
//!
//! Bug: Move data is stored in the `Moves` table (165 entries, 6 bytes each).
//! When the game looks up data for a glitch move (ID > NUM_ATTACKS = $A5) or
//! NO_MOVE (ID 0), it reads past the end of the table into `BaseStats` data,
//! producing garbage PP values, effect IDs, and other fields. This causes
//! variable PP display and unpredictable move effects.
//!
//! Fix: At each of the 7 call sites that index into the `Moves` table, add
//! `cp NUM_ATTACKS / jr c, .validMoveId / xor a` after the `dec a` that
//! converts the 1-based move ID to a 0-based index. This clamps out-of-range
//! indices to 0 (POUND), ensuring valid data is always read.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/Glitch_move>
//!   - <https://glitchcity.wiki/wiki/Glitch_move>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

// ─── Opcode constants ────────────────────────────────────────────────

const CP_N: u8 = 0xFE;
const JR_C: u8 = 0x38;
const XOR_A: u8 = 0xAF;
const NUM_ATTACKS: u8 = 0xA5; // 165 = STRUGGLE

// ─── Helpers ─────────────────────────────────────────────────────────

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Switch to the correct ROM bank and verify the 5-byte bounds check pattern
/// at `.validMoveId - 5`: `cp NUM_ATTACKS / jr c, .validMoveId / xor a`.
fn assert_bounds_check(h: &mut TestHarness, label: &str) {
    let bank = sym_bank(label);
    h.select_rom_bank(bank);

    let valid = sym_addr(label);
    let cp_addr = valid - 5;

    assert_eq!(rom(h, cp_addr), CP_N, "{label}: expected `cp` opcode at -5");
    assert_eq!(
        rom(h, cp_addr + 1),
        NUM_ATTACKS,
        "{label}: cp immediate should be NUM_ATTACKS ($A5)"
    );
    assert_eq!(
        rom(h, cp_addr + 2),
        JR_C,
        "{label}: expected `jr c` opcode at -3"
    );
    assert_eq!(
        rom(h, cp_addr + 3),
        0x01,
        "{label}: jr c displacement should be 1 (skip xor a)"
    );
    assert_eq!(
        rom(h, cp_addr + 4),
        XOR_A,
        "{label}: expected `xor a` at -1"
    );
}

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn heal_party_bounds_check() {
    let mut h = TestHarness::new_headless();
    assert_bounds_check(&mut h, "HealParty.validMoveId");
}

#[test]
fn get_max_pp_bounds_check() {
    let mut h = TestHarness::new_headless();
    assert_bounds_check(&mut h, "GetMaxPP.validMoveId");
}

#[test]
fn add_party_mon_write_move_pp_bounds_check() {
    let mut h = TestHarness::new_headless();
    assert_bounds_check(&mut h, "AddPartyMon_WriteMovePP.validMoveId");
}

#[test]
fn dont_abandon_learning_bounds_check() {
    let mut h = TestHarness::new_headless();
    assert_bounds_check(&mut h, "DontAbandonLearning.validMoveId");
}

#[test]
fn write_mon_moves_bounds_check() {
    let mut h = TestHarness::new_headless();
    assert_bounds_check(&mut h, "WriteMonMoves.validMoveId");
}

#[test]
fn get_current_move_bounds_check() {
    let mut h = TestHarness::new_headless();
    assert_bounds_check(&mut h, "GetCurrentMove.validMoveId");
}

#[test]
fn read_move_bounds_check() {
    let mut h = TestHarness::new_headless();
    assert_bounds_check(&mut h, "ReadMove.validMoveId");
}

#[test]
fn all_sites_in_banked_rom() {
    // All 7 sites should be in banked ROM (not HOME), so the fix has zero
    // impact on the critically-full ROM0.
    let labels = [
        "HealParty.validMoveId",
        "GetMaxPP.validMoveId",
        "AddPartyMon_WriteMovePP.validMoveId",
        "DontAbandonLearning.validMoveId",
        "WriteMonMoves.validMoveId",
        "GetCurrentMove.validMoveId",
        "ReadMove.validMoveId",
    ];
    for label in &labels {
        let bank = sym_bank(label);
        assert_ne!(bank, 0x00, "{label} should NOT be in HOME (bank $00)");
    }
}
