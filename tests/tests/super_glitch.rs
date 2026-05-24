//! ROM byte tests for Super Glitch prevention (move name lookup clamping).
//!
//! Bug: Glitch move IDs ($A6-$C3) have no defined names in the MoveNames table.
//! When the game displays such a name, GetName scans past the table into ROM,
//! then PlaceString/CopyString copies until $50 terminator, overflowing the
//! screen buffer and corrupting RAM (TMTRAINER effect / Super Glitch).
//!
//! Fix: Clamp move IDs to STRUGGLE ($A5) before name lookups in the 3 code
//! paths that call GetName directly (bypassing GetMoveName):
//! - GetCurrentMove (unified with existing Moves table clamp)
//! - EnemyCanExecuteChargingMove
//! - FormatMovesString (fight menu display)
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/Super_Glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const CP_N: u8 = 0xFE; // cp n
const JR_C: u8 = 0x38; // jr c, n
const LD_A_N: u8 = 0x3E; // ld a, n
const NUM_ATTACKS_PLUS_1: u8 = 0xA6; // NUM_ATTACKS + 1 = $A5 + 1
const STRUGGLE: u8 = 0xA5;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn get_current_move_clamp_in_bank_0f() {
    assert_eq!(sym_bank("GetCurrentMove"), 0x0F);
}

#[test]
fn get_current_move_clamps_before_name_list_index() {
    // GetCurrentMove.selected should have:
    //   cp NUM_ATTACKS + 1  ($FE $A6)
    //   jr c, .validMoveId  ($38 nn)
    //   ld a, STRUGGLE      ($3E $A5)
    // .validMoveId:
    //   ld [wNameListIndex], a
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    // The clamp is between .selected and .validMoveId
    let valid = sym_addr("GetCurrentMove.validMoveId");
    // Working backward: ld a, STRUGGLE (2) + jr c (2) + cp n (2) = 6 bytes before .validMoveId
    let clamp_start = valid - 6;

    assert_eq!(rom(&mut h, clamp_start), CP_N, "Expected `cp n`");
    assert_eq!(
        rom(&mut h, clamp_start + 1),
        NUM_ATTACKS_PLUS_1,
        "Expected NUM_ATTACKS + 1 ($A6)"
    );
    assert_eq!(rom(&mut h, clamp_start + 2), JR_C, "Expected `jr c`");
    assert_eq!(rom(&mut h, clamp_start + 4), LD_A_N, "Expected `ld a, n`");
    assert_eq!(
        rom(&mut h, clamp_start + 5),
        STRUGGLE,
        "Expected STRUGGLE ($A5)"
    );
}

#[test]
fn enemy_charging_move_clamp_present() {
    // EnemyCanExecuteChargingMove.validMoveId should be preceded by
    // the same cp/jr c/ld a clamp sequence
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let valid = sym_addr("EnemyCanExecuteChargingMove.validMoveId");
    let clamp_start = valid - 6;

    assert_eq!(rom(&mut h, clamp_start), CP_N, "Expected `cp n`");
    assert_eq!(
        rom(&mut h, clamp_start + 1),
        NUM_ATTACKS_PLUS_1,
        "Expected NUM_ATTACKS + 1"
    );
    assert_eq!(rom(&mut h, clamp_start + 2), JR_C, "Expected `jr c`");
    assert_eq!(rom(&mut h, clamp_start + 4), LD_A_N, "Expected `ld a, n`");
    assert_eq!(
        rom(&mut h, clamp_start + 5),
        STRUGGLE,
        "Expected STRUGGLE ($A5)"
    );
}

#[test]
fn format_moves_string_clamp_present() {
    // FormatMovesString.validMoveId should be preceded by the same clamp
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("FormatMovesString"));

    let valid = sym_addr("FormatMovesString.validMoveId");
    let clamp_start = valid - 6;

    assert_eq!(rom(&mut h, clamp_start), CP_N, "Expected `cp n`");
    assert_eq!(
        rom(&mut h, clamp_start + 1),
        NUM_ATTACKS_PLUS_1,
        "Expected NUM_ATTACKS + 1"
    );
    assert_eq!(rom(&mut h, clamp_start + 2), JR_C, "Expected `jr c`");
    assert_eq!(rom(&mut h, clamp_start + 4), LD_A_N, "Expected `ld a, n`");
    assert_eq!(
        rom(&mut h, clamp_start + 5),
        STRUGGLE,
        "Expected STRUGGLE ($A5)"
    );
}

#[test]
fn format_moves_string_in_bank_0e() {
    assert_eq!(
        sym_bank("FormatMovesString"),
        0x0E,
        "FormatMovesString should be in bank $0E"
    );
}
