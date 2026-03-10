//! ROM byte tests for the Strength boulder smoke puff OAM corruption fix.
//!
//! Bug: `AdjustOAMBlockYPos2` adjusts OAM entry Y-coordinates for the
//! boulder dust animation. When Y >= 112 (off-screen), the code was
//! supposed to hide the sprite by setting Y to 160. Instead, it did
//! `dec hl` (backing into the **previous** OAM entry's attribute byte),
//! wrote 160 there (corrupting palette/flip/priority), then `[hli]`
//! advanced back to the current Y. The smoke puff sprites displayed
//! with wrong attributes when pushing boulders.
//!
//! Fix: Remove the `dec hl / ld a, 160 / ld [hli], a` sequence.
//! Replace with a simple conditional: if Y >= 112, set A = 160 before
//! writing to `[hl]`. Saves 2 bytes in bank $1E.
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("AdjustOAMBlockYPos2"));
    h
}

// ─── AdjustOAMBlockYPos2 structural tests ────────────────────────────

#[test]
fn adjust_oam_y_is_in_bank_1e() {
    assert_eq!(
        sym_bank("AdjustOAMBlockYPos2"),
        0x1E,
        "AdjustOAMBlockYPos2 should be in bank $1E"
    );
}

#[test]
fn adjust_oam_y_starts_with_ld_de_obj_size() {
    let mut h = rom_harness();
    let addr = sym_addr("AdjustOAMBlockYPos2");
    // ld de, OBJ_SIZE ($04) → $11 $04 $00
    assert_eq!(rom(&mut h, addr), 0x11, "ld de, nn opcode");
    assert_eq!(rom(&mut h, addr + 1), 0x04, "OBJ_SIZE = 4");
    assert_eq!(rom(&mut h, addr + 2), 0x00, "OBJ_SIZE high byte = 0");
}

#[test]
fn loop_reads_coord_adjustment_amount() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("AdjustOAMBlockYPos2.loop");
    // ld a, [wCoordAdjustmentAmount] → $FA lo hi
    assert_eq!(rom(&mut h, loop_addr), 0xFA, "ld a, [nn] opcode at .loop");
}

#[test]
fn loop_adds_b_and_compares_112() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("AdjustOAMBlockYPos2.loop");
    // After: ld a, [wCoordAdjustmentAmount] (3) / ld b, a (1) / ld a, [hl] (1)
    // = offset +5 from .loop: add b
    let add_b = loop_addr + 5;
    assert_eq!(rom(&mut h, add_b), 0x80, "add b opcode");
    // cp 112 → $FE $70
    assert_eq!(rom(&mut h, add_b + 1), 0xFE, "cp n opcode");
    assert_eq!(rom(&mut h, add_b + 2), 112, "compare threshold = 112");
}

#[test]
fn jr_c_targets_no_overflow() {
    let mut h = rom_harness();
    let no_overflow = sym_addr("AdjustOAMBlockYPos2.noOverflow");
    let loop_addr = sym_addr("AdjustOAMBlockYPos2.loop");
    // jr c is at loop + 8 (after ld a [nn] (3) / ld b,a (1) / ld a,[hl] (1) / add b (1) / cp 112 (2))
    let jr_c_addr = loop_addr + 8;
    assert_eq!(rom(&mut h, jr_c_addr), 0x38, "jr c opcode");
    let jr_offset = rom(&mut h, jr_c_addr + 1) as i8;
    let jr_pc = jr_c_addr + 2; // PC after reading jr instruction
    let target = (jr_pc as i32 + jr_offset as i32) as u16;
    assert_eq!(target, no_overflow, "jr c should target .noOverflow");
}

#[test]
fn ld_a_160_before_no_overflow() {
    let mut h = rom_harness();
    let no_overflow = sym_addr("AdjustOAMBlockYPos2.noOverflow");
    // ld a, 160 ($3E $A0) is at .noOverflow - 2
    assert_eq!(
        rom(&mut h, no_overflow - 2),
        0x3E,
        "ld a, n opcode before .noOverflow"
    );
    assert_eq!(
        rom(&mut h, no_overflow - 1),
        160,
        "immediate value = 160 (off-screen Y)"
    );
}

#[test]
fn no_overflow_writes_to_hl() {
    let mut h = rom_harness();
    let no_overflow = sym_addr("AdjustOAMBlockYPos2.noOverflow");
    // .noOverflow: ld [hl], a → $77
    assert_eq!(rom(&mut h, no_overflow), 0x77, "ld [hl], a at .noOverflow");
}

#[test]
fn no_dec_hl_in_function() {
    // The bug was `dec hl` ($2B) between the `cp 112` and `ld [hl], a`.
    // Verify there is no $2B (dec hl) between .loop and .noOverflow.
    let mut h = rom_harness();
    let loop_addr = sym_addr("AdjustOAMBlockYPos2.loop");
    let no_overflow = sym_addr("AdjustOAMBlockYPos2.noOverflow");
    for addr in loop_addr..no_overflow {
        assert_ne!(
            rom(&mut h, addr),
            0x2B,
            "dec hl ($2B) should not appear between .loop and .noOverflow (found at {:#06X})",
            addr
        );
    }
}

#[test]
fn no_hli_store_in_function() {
    // The bug used `ld [hli], a` ($22) to write to wrong address then advance.
    // Verify there is no $22 between .loop and the ret.
    let mut h = rom_harness();
    let loop_addr = sym_addr("AdjustOAMBlockYPos2.loop");
    let no_overflow = sym_addr("AdjustOAMBlockYPos2.noOverflow");
    // Check from loop to noOverflow (the conditional block area)
    for addr in loop_addr..no_overflow {
        assert_ne!(
            rom(&mut h, addr),
            0x22,
            "ld [hli], a ($22) should not appear between .loop and .noOverflow (found at {:#06X})",
            addr
        );
    }
}

#[test]
fn function_ends_with_ret() {
    let mut h = rom_harness();
    // After .noOverflow: ld [hl], a (1) / add hl, de (1) / dec c (1) / jr nz (2) / ret (1)
    let no_overflow = sym_addr("AdjustOAMBlockYPos2.noOverflow");
    let ret_addr = no_overflow + 5;
    assert_eq!(rom(&mut h, ret_addr), 0xC9, "ret opcode at end of function");
}
