//! Emulator-based tests for the Transform move-swap fix.
//!
//! Vanilla Yellow blocks move swapping entirely when transformed (SELECT in
//! battle menu is ignored). Our fix allows reordering the battle move copies
//! (wBattleMonMoves/PP) with SELECT while transformed, but skips writing to
//! the party data (wPartyMon1Moves/PP) so the real moveset is preserved.
//!
//! Test approach: set up SwapMovesInMenu with controlled wBattleMonMoves,
//! wPartyMon1Moves, wCurrentMenuItem, and wMenuItemToSwap, then run to
//! `.doneSwap` or `MoveSelectionMenu` and verify which memory regions changed.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Bit 3 of wPlayerBattleStatus3.
const TRANSFORMED_BIT: u8 = 3;

/// A safe WRAM trap address.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Set up the harness for running SwapMovesInMenu.
///
/// Initializes battle moves to [A, B, C, D] and party moves to [W, X, Y, Z]
/// with corresponding PP values, then configures a swap of positions `from`
/// and `to` (1-indexed, matching wCurrentMenuItem/wMenuItemToSwap).
fn setup_swap_fixture(transformed: bool, from: u8, to: u8) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("SwapMovesInMenu"));

    // Trap at WRAM address for return
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    let battle_moves = sym_addr("wBattleMonMoves");
    let battle_pp = sym_addr("wBattleMonPP");
    let party_moves = sym_addr("wPartyMon1Moves");
    let party_pp = sym_addr("wPartyMon1PP");

    // Battle moves: [0x21, 0x22, 0x23, 0x24] (Tackle-ish placeholder IDs)
    h.write_mem(battle_moves, 0x21);
    h.write_mem(battle_moves + 1, 0x22);
    h.write_mem(battle_moves + 2, 0x23);
    h.write_mem(battle_moves + 3, 0x24);

    // Battle PP: [35, 25, 15, 10]
    h.write_mem(battle_pp, 35);
    h.write_mem(battle_pp + 1, 25);
    h.write_mem(battle_pp + 2, 15);
    h.write_mem(battle_pp + 3, 10);

    // Party moves (slot 0): [0xA1, 0xA2, 0xA3, 0xA4]
    h.write_mem(party_moves, 0xA1);
    h.write_mem(party_moves + 1, 0xA2);
    h.write_mem(party_moves + 2, 0xA3);
    h.write_mem(party_moves + 3, 0xA4);

    // Party PP (slot 0): [30, 20, 10, 5]
    h.write_mem(party_pp, 30);
    h.write_mem(party_pp + 1, 20);
    h.write_mem(party_pp + 2, 10);
    h.write_mem(party_pp + 3, 5);

    // First party mon (index 0)
    h.write_mem(sym_addr("wPlayerMonNumber"), 0x00);

    // No disabled move
    h.write_mem(sym_addr("wPlayerDisabledMove"), 0x00);

    // Set transform status
    if transformed {
        h.write_mem(sym_addr("wPlayerBattleStatus3"), 1 << TRANSFORMED_BIT);
    } else {
        h.write_mem(sym_addr("wPlayerBattleStatus3"), 0x00);
    }

    // wMenuItemToSwap = `from` (1-indexed), wCurrentMenuItem = `to` (1-indexed)
    // These are the two positions to swap.
    h.write_mem(sym_addr("wMenuItemToSwap"), from);
    h.write_mem(sym_addr("wCurrentMenuItem"), to);

    h
}

/// Read the 4 battle moves as an array.
fn read_battle_moves(h: &mut TestHarness) -> [u8; 4] {
    let base = sym_addr("wBattleMonMoves");
    [
        h.read_mem(base),
        h.read_mem(base + 1),
        h.read_mem(base + 2),
        h.read_mem(base + 3),
    ]
}

/// Read the 4 battle PP values as an array.
fn read_battle_pp(h: &mut TestHarness) -> [u8; 4] {
    let base = sym_addr("wBattleMonPP");
    [
        h.read_mem(base),
        h.read_mem(base + 1),
        h.read_mem(base + 2),
        h.read_mem(base + 3),
    ]
}

/// Read the 4 party moves (slot 0) as an array.
fn read_party_moves(h: &mut TestHarness) -> [u8; 4] {
    let base = sym_addr("wPartyMon1Moves");
    [
        h.read_mem(base),
        h.read_mem(base + 1),
        h.read_mem(base + 2),
        h.read_mem(base + 3),
    ]
}

/// Read the 4 party PP values (slot 0) as an array.
fn read_party_pp(h: &mut TestHarness) -> [u8; 4] {
    let base = sym_addr("wPartyMon1PP");
    [
        h.read_mem(base),
        h.read_mem(base + 1),
        h.read_mem(base + 2),
        h.read_mem(base + 3),
    ]
}

/// Run SwapMovesInMenu and stop at `MoveSelectionMenu` (after cleanup).
fn run_swap(h: &mut TestHarness) {
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("SwapMovesInMenu"));
    h.step_to(sym_addr("MoveSelectionMenu"));
}

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_no_transform_guard_at_entry() {
    // The old code had `ld a, [wPlayerBattleStatus3]` at SwapMovesInMenu entry.
    // After our fix, the first instruction (ignoring DEBUG) should be
    // `ld a, [wMenuItemToSwap]` ($FA, lo, hi).
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SwapMovesInMenu"));

    let entry = sym_addr("SwapMovesInMenu");
    let opcode = h.read_mem(entry);
    assert_eq!(
        opcode, 0xFA,
        "SwapMovesInMenu should start with ld a, [imm16] ($FA), got ${opcode:02X}"
    );

    // The operand should be wMenuItemToSwap ($CC35), not wPlayerBattleStatus3
    let lo = h.read_mem(entry + 1);
    let hi = h.read_mem(entry + 2);
    let addr = (hi as u16) << 8 | lo as u16;
    let expected = sym_addr("wMenuItemToSwap");
    assert_eq!(
        addr, expected,
        "First ld a should read wMenuItemToSwap (${expected:04X}), got ${addr:04X}"
    );
}

#[test]
fn rom_bytes_transform_guard_at_swap_moves_in_party_mon() {
    // .swapMovesInPartyMon should now start with `ld a, [wPlayerBattleStatus3]`
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SwapMovesInMenu"));

    let label = sym_addr("SwapMovesInMenu.swapMovesInPartyMon");
    let opcode = h.read_mem(label);
    assert_eq!(
        opcode, 0xFA,
        ".swapMovesInPartyMon should start with ld a, [imm16] ($FA)"
    );

    let lo = h.read_mem(label + 1);
    let hi = h.read_mem(label + 2);
    let addr = (hi as u16) << 8 | lo as u16;
    assert_eq!(
        addr,
        sym_addr("wPlayerBattleStatus3"),
        ".swapMovesInPartyMon should read wPlayerBattleStatus3"
    );

    // Next: bit TRANSFORMED, a ($CB $5F for bit 3, a)
    assert_eq!(h.read_mem(label + 3), 0xCB, "CB prefix for bit instruction");
    assert_eq!(
        h.read_mem(label + 4),
        0x5F,
        "bit 3, a opcode ($5F) for TRANSFORMED"
    );

    // Next: jr nz, .doneSwap ($20, offset)
    assert_eq!(
        h.read_mem(label + 5),
        0x20,
        "jr nz opcode ($20) to skip party swap"
    );
}

// ─── Behavioral: transformed — battle swap yes, party swap no ───────

#[test]
fn transformed_swap_changes_battle_moves() {
    let mut h = setup_swap_fixture(true, 1, 2);
    run_swap(&mut h);

    // Battle moves: positions 1 and 2 should be swapped
    // [0x21, 0x22, 0x23, 0x24] → [0x22, 0x21, 0x23, 0x24]
    assert_eq!(
        read_battle_moves(&mut h),
        [0x22, 0x21, 0x23, 0x24],
        "Battle moves should be swapped at positions 1 and 2"
    );
}

#[test]
fn transformed_swap_changes_battle_pp() {
    let mut h = setup_swap_fixture(true, 1, 2);
    run_swap(&mut h);

    // Battle PP: [35, 25, 15, 10] → [25, 35, 15, 10]
    assert_eq!(
        read_battle_pp(&mut h),
        [25, 35, 15, 10],
        "Battle PP should be swapped at positions 1 and 2"
    );
}

#[test]
fn transformed_swap_preserves_party_moves() {
    let mut h = setup_swap_fixture(true, 1, 2);
    run_swap(&mut h);

    // Party moves should be UNCHANGED
    assert_eq!(
        read_party_moves(&mut h),
        [0xA1, 0xA2, 0xA3, 0xA4],
        "Party moves must NOT change when transformed"
    );
}

#[test]
fn transformed_swap_preserves_party_pp() {
    let mut h = setup_swap_fixture(true, 1, 2);
    run_swap(&mut h);

    // Party PP should be UNCHANGED
    assert_eq!(
        read_party_pp(&mut h),
        [30, 20, 10, 5],
        "Party PP must NOT change when transformed"
    );
}

// ─── Behavioral: not transformed — both battle and party swapped ────

#[test]
fn normal_swap_changes_battle_moves() {
    let mut h = setup_swap_fixture(false, 1, 3);
    run_swap(&mut h);

    // Battle moves: positions 1 and 3 swapped
    // [0x21, 0x22, 0x23, 0x24] → [0x23, 0x22, 0x21, 0x24]
    assert_eq!(
        read_battle_moves(&mut h),
        [0x23, 0x22, 0x21, 0x24],
        "Battle moves should be swapped at positions 1 and 3"
    );
}

#[test]
fn normal_swap_changes_party_moves() {
    let mut h = setup_swap_fixture(false, 1, 3);
    run_swap(&mut h);

    // Party moves: positions 1 and 3 swapped
    // [0xA1, 0xA2, 0xA3, 0xA4] → [0xA3, 0xA2, 0xA1, 0xA4]
    assert_eq!(
        read_party_moves(&mut h),
        [0xA3, 0xA2, 0xA1, 0xA4],
        "Party moves should be swapped at positions 1 and 3"
    );
}

#[test]
fn normal_swap_changes_party_pp() {
    let mut h = setup_swap_fixture(false, 1, 3);
    run_swap(&mut h);

    // Party PP: positions 1 and 3 swapped
    // [30, 20, 10, 5] → [10, 20, 30, 5]
    assert_eq!(
        read_party_pp(&mut h),
        [10, 20, 30, 5],
        "Party PP should be swapped at positions 1 and 3"
    );
}

// ─── Edge cases ─────────────────────────────────────────────────────

#[test]
fn transformed_swap_last_two_moves() {
    // Swap positions 3 and 4 while transformed
    let mut h = setup_swap_fixture(true, 3, 4);
    run_swap(&mut h);

    assert_eq!(
        read_battle_moves(&mut h),
        [0x21, 0x22, 0x24, 0x23],
        "Battle moves 3 and 4 should be swapped"
    );
    assert_eq!(
        read_party_moves(&mut h),
        [0xA1, 0xA2, 0xA3, 0xA4],
        "Party moves must remain unchanged"
    );
}

#[test]
fn transformed_swap_clears_menu_item_to_swap() {
    let mut h = setup_swap_fixture(true, 1, 2);
    run_swap(&mut h);

    assert_eq!(
        h.read_mem(sym_addr("wMenuItemToSwap")),
        0x00,
        "wMenuItemToSwap should be cleared after swap"
    );
}

#[test]
fn normal_swap_clears_menu_item_to_swap() {
    let mut h = setup_swap_fixture(false, 2, 4);
    run_swap(&mut h);

    assert_eq!(
        h.read_mem(sym_addr("wMenuItemToSwap")),
        0x00,
        "wMenuItemToSwap should be cleared after swap"
    );
}
