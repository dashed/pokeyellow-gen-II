//! Emulator-based tests for the PP restore PP Ups fix.
//!
//! Max Ethers and Max Elixirs use `.fullyRestorePP` which loads the raw PP
//! byte and compares it to max PP. The upper 2 bits of the PP byte store
//! the PP Up count, so without masking them out, the comparison fails when
//! any PP Ups have been used — even if the move already has full PP.
//!
//! Our fix adds `and PP_MASK` to mask out the PP Up bits before comparing.
//!
//! Test approach: enter at `.fullyRestorePP` with HL pointing to a PP byte
//! and B = max PP, then check whether the function returns via the `ret z`
//! (no effect) path or the `.storeNewAmount` (restore) path.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// A safe WRAM address to store the PP byte under test.
const W_TEST_PP: u16 = 0xC200;

/// A safe WRAM trap address for ret.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Result of running .fullyRestorePP.
#[derive(Debug, PartialEq)]
enum RestoreResult {
    /// Function returned via `ret z` — no effect (PP already full).
    NoEffect,
    /// Function fell through to .storeNewAmount — PP was restored.
    Restored { new_pp_byte: u8 },
}

/// Run `.fullyRestorePP` with the given PP byte and max PP value.
///
/// `pp_byte`: The raw PP byte (upper 2 bits = PP Up count, lower 6 = current PP).
/// `max_pp`: The max PP value (already accounting for PP Ups).
fn run_fully_restore_pp(pp_byte: u8, max_pp: u8) -> RestoreResult {
    let bank = sym_bank("ItemUsePPRestore");
    let fully_restore_pp = sym_addr("ItemUsePPRestore.fullyRestorePP");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(bank);

    // Trap for ret
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Set up the PP byte at W_TEST_PP
    h.write_mem(W_TEST_PP, pp_byte);

    // HL = pointer to PP byte, B = max PP
    h.gb.cpu().set_hl(W_TEST_PP);
    h.set_b(max_pp);

    h.set_pc(fully_restore_pp);

    // Step until we reach the trap (function returned)
    for _ in 0..200 {
        let pc = h.pc();
        if pc == TRAP_ADDR {
            let new_pp_byte = h.read_mem(W_TEST_PP);
            if new_pp_byte == pp_byte {
                return RestoreResult::NoEffect;
            } else {
                return RestoreResult::Restored { new_pp_byte };
            }
        }
        h.gb.clock();
    }
    panic!(
        "fullyRestorePP did not return within 200 instructions (PC=${:04X})",
        h.pc()
    );
}

// ─── PP restore fix tests ────────────────────────────────────────────

#[test]
fn full_pp_with_3_pp_ups_is_no_effect() {
    // Base PP 15 + 3 PP Ups → max PP = 24
    // PP byte: 3 PP Ups ($C0) | current PP 24 = $D8
    // This is THE BUG: without the fix, $D8 ≠ 24 → falls through
    let pp_byte = (3 << 6) | 24; // $D8
    let result = run_fully_restore_pp(pp_byte, 24);
    assert_eq!(
        result,
        RestoreResult::NoEffect,
        "Full PP (24/24) with 3 PP Ups should be no effect"
    );
}

#[test]
fn full_pp_with_0_pp_ups_is_no_effect() {
    // Base PP 15 + 0 PP Ups → max PP = 15
    // PP byte: 0 PP Ups ($00) | current PP 15 = $0F
    let pp_byte = 15;
    let result = run_fully_restore_pp(pp_byte, 15);
    assert_eq!(
        result,
        RestoreResult::NoEffect,
        "Full PP (15/15) with 0 PP Ups should be no effect"
    );
}

#[test]
fn full_pp_with_1_pp_up_is_no_effect() {
    // Base PP 15 + 1 PP Up → max PP = 18
    // PP byte: 1 PP Up ($40) | current PP 18 = $52
    let pp_byte = (1 << 6) | 18; // $52
    let result = run_fully_restore_pp(pp_byte, 18);
    assert_eq!(
        result,
        RestoreResult::NoEffect,
        "Full PP (18/18) with 1 PP Up should be no effect"
    );
}

#[test]
fn partial_pp_with_pp_ups_restores() {
    // Base PP 15 + 3 PP Ups → max PP = 24
    // Current PP = 10 (not full)
    // PP byte: 3 PP Ups ($C0) | current PP 10 = $CA
    let pp_byte = (3 << 6) | 10; // $CA
    let result = run_fully_restore_pp(pp_byte, 24);
    // .storeNewAmount: [hl] & PP_UP_MASK + max_pp = $C0 + 24 = $D8
    assert_eq!(
        result,
        RestoreResult::Restored { new_pp_byte: 0xD8 },
        "Partial PP (10/24) with 3 PP Ups should restore to full"
    );
}

#[test]
fn empty_pp_with_pp_ups_restores() {
    // Base PP 15 + 2 PP Ups → max PP = 21
    // Current PP = 0
    // PP byte: 2 PP Ups ($80) | current PP 0 = $80
    let pp_byte = 2 << 6; // $80
    let result = run_fully_restore_pp(pp_byte, 21);
    // .storeNewAmount: $80 + 21 = $95
    assert_eq!(
        result,
        RestoreResult::Restored {
            new_pp_byte: 0x80 + 21
        },
        "Empty PP (0/21) with 2 PP Ups should restore to full"
    );
}

#[test]
fn full_pp_with_2_pp_ups_is_no_effect() {
    // Base PP 40 + 2 PP Ups → max PP = 56
    // PP byte: 2 PP Ups ($80) | current PP 56 = $B8
    let pp_byte = (2 << 6) | 56; // $B8
    let result = run_fully_restore_pp(pp_byte, 56);
    assert_eq!(
        result,
        RestoreResult::NoEffect,
        "Full PP (56/56) with 2 PP Ups should be no effect"
    );
}
