//! ROM byte tests for the Struggle bypassing PP underflow fix.
//!
//! Bug: When a move is auto-selected (after thaw, during binding moves,
//! Hyper Beam recharge, Metronome, Mimic), the Struggle PP check is bypassed.
//! If the auto-selected move has 0 PP, `dec [hl]` underflows PP from 0 to 63,
//! also corrupting PP Up bits.
//!
//! Fix: In `.DecrementPP`, mask off PP Up bits and check actual PP before
//! decrementing. If PP is already 0, return without decrementing.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Struggle_bypassing>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const LD_A_HL: u8 = 0x7E; // ld a, [hl]
const AND_N: u8 = 0xE6; // and n
const RET_Z: u8 = 0xC8; // ret z
const DEC_HL: u8 = 0x35; // dec [hl]
const RET: u8 = 0xC9; // ret
const PP_MASK: u8 = 0x3F; // %00111111

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn decrement_pp_is_in_bank_3d() {
    assert_eq!(
        sym_bank("DecrementPP"),
        0x3D,
        "DecrementPP should be in bank $3D"
    );
}

#[test]
fn local_decrement_pp_is_in_bank_3d() {
    assert_eq!(
        sym_bank("DecrementPP.DecrementPP"),
        0x3D,
        ".DecrementPP local label should be in bank $3D"
    );
}

#[test]
fn pp_mask_guard_present() {
    // After `add hl, bc` (7 bytes into .DecrementPP), the fix inserts:
    //   ld a, [hl]     ; $7E
    //   and PP_MASK    ; $E6 $3F
    //   ret z          ; $C8
    let mut h = TestHarness::new();
    h.select_rom_bank(0x3D);

    let base = sym_addr("DecrementPP.DecrementPP");
    // .DecrementPP layout:
    //   +0: ld a, [wPlayerMoveListIndex] (3 bytes: $FA lo hi)
    //   +3: ld c, a (1 byte: $4F)
    //   +4: ld b, 0 (2 bytes: $06 $00)
    //   +6: add hl, bc (1 byte: $09)
    //   +7: ld a, [hl]  ← fix starts here
    //   +8: and PP_MASK
    //  +10: ret z
    //  +11: dec [hl]
    //  +12: ret

    let fix_start = base + 7;
    assert_eq!(
        rom(&mut h, fix_start),
        LD_A_HL,
        "Expected `ld a, [hl]` at .DecrementPP+7"
    );
    assert_eq!(
        rom(&mut h, fix_start + 1),
        AND_N,
        "Expected `and n` opcode at .DecrementPP+8"
    );
    assert_eq!(
        rom(&mut h, fix_start + 2),
        PP_MASK,
        "Expected PP_MASK ($3F) operand at .DecrementPP+9"
    );
    assert_eq!(
        rom(&mut h, fix_start + 3),
        RET_Z,
        "Expected `ret z` at .DecrementPP+10"
    );
}

#[test]
fn dec_hl_follows_guard() {
    // The original `dec [hl]` / `ret` must still follow the guard
    let mut h = TestHarness::new();
    h.select_rom_bank(0x3D);

    let base = sym_addr("DecrementPP.DecrementPP");
    let dec_addr = base + 11;

    assert_eq!(
        rom(&mut h, dec_addr),
        DEC_HL,
        "Expected `dec [hl]` at .DecrementPP+11"
    );
    assert_eq!(
        rom(&mut h, dec_addr + 1),
        RET,
        "Expected `ret` at .DecrementPP+12"
    );
}

#[test]
fn struggle_check_still_at_entry() {
    // The main DecrementPP entry still checks for Struggle (cp STRUGGLE / ret z)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x3D);

    let entry = sym_addr("DecrementPP");
    // ld a, [de] = $1A
    assert_eq!(
        rom(&mut h, entry),
        0x1A,
        "Expected `ld a, [de]` at DecrementPP entry"
    );
    // cp STRUGGLE = $FE $A5 (STRUGGLE = $A5 = 165)
    assert_eq!(
        rom(&mut h, entry + 1),
        0xFE,
        "Expected `cp n` opcode at DecrementPP+1"
    );
    assert_eq!(
        rom(&mut h, entry + 2),
        0xA5,
        "Expected STRUGGLE constant ($A5) at DecrementPP+2"
    );
    // ret z
    assert_eq!(
        rom(&mut h, entry + 3),
        RET_Z,
        "Expected `ret z` (skip Struggle) at DecrementPP+3"
    );
}
