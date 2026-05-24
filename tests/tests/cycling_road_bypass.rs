//! ROM byte tests for the Cycling Road guard bypass fix.
//!
//! Bug: The player can enter Cycling Road without a Bicycle by holding LEFT
//! while the guard's forced-movement script pushes them RIGHT.  The d-pad
//! input overrides the scripted movement because `wJoyIgnore` is not set
//! before the guard text displays (especially in the `.next_to_counter` path).
//!
//! Fix: Set `wJoyIgnore = PAD_CTRL_PAD` immediately after the coordinate check
//! succeeds (before the guard text), blocking d-pad input for both the walk-up
//! and next-to-counter paths.  Applied to both Route 16 and Route 18 gates.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I>
//! Reference: <https://glitchcity.wiki/wiki/Go_on_Cycling_Road_without_a_Bicycle>

use pokeyellow_tests::{sym_addr, sym_bank};

// ─── Opcode constants ────────────────────────────────────────────────

const RET_NC: u8 = 0xD0; // ret nc
const LD_A_N: u8 = 0x3E; // ld a, n
const LD_NN_A: u8 = 0xEA; // ld [nn], a
const PAD_CTRL_PAD: u8 = 0xF0; // d-pad mask: Up|Down|Left|Right

// ─── Helpers ─────────────────────────────────────────────────────────

fn rom() -> Vec<u8> {
    std::fs::read("../pokeyellow.gbc").expect("ROM not found")
}

fn rom_offset(bank: u32, addr: u16) -> usize {
    (bank * 0x4000 + (addr as u32 - 0x4000)) as usize
}

fn at(rom: &[u8], bank: u32, addr: u16) -> u8 {
    rom[rom_offset(bank, addr)]
}

/// Scan forward from `start` to find `RET_NC` ($D0), return its address.
fn find_ret_nc(rom: &[u8], bank: u32, start: u16, max: u16) -> Option<u16> {
    for i in 0..max {
        if at(rom, bank, start + i) == RET_NC {
            return Some(start + i);
        }
    }
    None
}

// ─── Route 16 Gate tests ─────────────────────────────────────────────

#[test]
fn route16_gate_in_bank_12() {
    assert_eq!(sym_bank("Route16Gate1FDefaultScript"), 0x12);
}

#[test]
fn route16_gate_joy_ignore_after_ret_nc() {
    let rom = rom();
    let bank: u32 = 0x12;
    let default = sym_addr("Route16Gate1FDefaultScript");

    let ret_nc = find_ret_nc(&rom, bank, default, 30).expect("ret nc not found in Route16Gate1F");

    // After ret nc: ld a, PAD_CTRL_PAD ($3E $F0) / ld [wJoyIgnore], a ($EA xx xx)
    assert_eq!(
        at(&rom, bank, ret_nc + 1),
        LD_A_N,
        "Expected `ld a, n` after ret nc"
    );
    assert_eq!(
        at(&rom, bank, ret_nc + 2),
        PAD_CTRL_PAD,
        "Expected PAD_CTRL_PAD ($F0) operand"
    );
    assert_eq!(
        at(&rom, bank, ret_nc + 3),
        LD_NN_A,
        "Expected `ld [nn], a` (store to wJoyIgnore)"
    );
}

#[test]
fn route16_gate_joy_ignore_before_display_text() {
    // The wJoyIgnore set should come BEFORE DisplayTextID
    let rom = rom();
    let bank: u32 = 0x12;
    let default = sym_addr("Route16Gate1FDefaultScript");

    let ret_nc = find_ret_nc(&rom, bank, default, 30).expect("ret nc not found");

    // ld [wJoyIgnore], a is at ret_nc + 3..5
    // Then ld a, TEXT_xxx / ldh [hTextID], a / call DisplayTextID should follow
    let after_store = ret_nc + 6; // skip ld a,n + ld [nn],a = 5 bytes, so +6 from ret_nc
    assert_eq!(
        at(&rom, bank, after_store),
        LD_A_N,
        "Expected `ld a, TEXT_xxx` after wJoyIgnore store"
    );
}

// ─── Route 18 Gate tests ─────────────────────────────────────────────

#[test]
fn route18_gate_in_bank_12() {
    assert_eq!(sym_bank("Route18Gate1FDefaultScript"), 0x12);
}

#[test]
fn route18_gate_joy_ignore_after_ret_nc() {
    let rom = rom();
    let bank: u32 = 0x12;
    let default = sym_addr("Route18Gate1FDefaultScript");

    let ret_nc = find_ret_nc(&rom, bank, default, 30).expect("ret nc not found in Route18Gate1F");

    assert_eq!(
        at(&rom, bank, ret_nc + 1),
        LD_A_N,
        "Expected `ld a, n` after ret nc"
    );
    assert_eq!(
        at(&rom, bank, ret_nc + 2),
        PAD_CTRL_PAD,
        "Expected PAD_CTRL_PAD ($F0) operand"
    );
    assert_eq!(
        at(&rom, bank, ret_nc + 3),
        LD_NN_A,
        "Expected `ld [nn], a` (store to wJoyIgnore)"
    );
}

#[test]
fn route18_gate_joy_ignore_before_display_text() {
    let rom = rom();
    let bank: u32 = 0x12;
    let default = sym_addr("Route18Gate1FDefaultScript");

    let ret_nc = find_ret_nc(&rom, bank, default, 30).expect("ret nc not found");

    let after_store = ret_nc + 6;
    assert_eq!(
        at(&rom, bank, after_store),
        LD_A_N,
        "Expected `ld a, TEXT_xxx` after wJoyIgnore store"
    );
}

// ─── Cross-check: wJoyIgnore is cleared after push-back ──────────────

#[test]
fn route16_player_moving_right_clears_joy_ignore() {
    let rom = rom();
    let bank: u32 = 0x12;
    let moving_right = sym_addr("Route16Gate1FPlayerMovingRightScript");

    // Scan for xor a ($AF) followed by ld [wJoyIgnore], a ($EA xx xx)
    let mut found = false;
    for i in 0..20u16 {
        if at(&rom, bank, moving_right + i) == 0xAF
            && at(&rom, bank, moving_right + i + 1) == LD_NN_A
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "PlayerMovingRightScript should clear wJoyIgnore with xor a / ld [nn], a"
    );
}

#[test]
fn route18_player_moving_right_clears_joy_ignore() {
    let rom = rom();
    let bank: u32 = 0x12;
    let moving_right = sym_addr("Route18Gate1FPlayerMovingRightScript");

    let mut found = false;
    for i in 0..20u16 {
        if at(&rom, bank, moving_right + i) == 0xAF
            && at(&rom, bank, moving_right + i + 1) == LD_NN_A
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "PlayerMovingRightScript should clear wJoyIgnore with xor a / ld [nn], a"
    );
}
