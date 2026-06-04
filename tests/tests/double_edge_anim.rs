//! ROM byte tests for the Double Edge opponent animation mirror fix.
//!
//! Bug: `Subanim_0CirclesCentering` (the circular-orbs animation used by
//! Double Edge) specifies `SUBANIMTYPE_COORDFLIP` instead of
//! `SUBANIMTYPE_HVFLIP`. When the opponent uses Double Edge, the orbs
//! appear at wrong positions instead of mirroring from the four corners,
//! because coordinate-flip swaps X/Y rather than mirroring horizontally
//! and vertically.
//!
//! Fix: Change the subanimation type byte from `SUBANIMTYPE_COORDFLIP` (3)
//! to `SUBANIMTYPE_HVFLIP` (1). One-byte change in bank $1E.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("Subanim_0CirclesCentering"));
    h
}

/// Subanimation type constants (from constants/move_animation_constants.asm).
const SUBANIMTYPE_HVFLIP: u8 = 1;
const SUBANIMTYPE_COORDFLIP: u8 = 3;

/// Encode the subanim header byte: `(type << 5) | count`.
const fn subanim_header(subanimtype: u8, count: u8) -> u8 {
    (subanimtype << 5) | count
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn subanim_in_bank_1e() {
    assert_eq!(sym_bank("Subanim_0CirclesCentering"), 0x1E);
}

#[test]
fn subanim_address_in_banked_range() {
    let addr = sym_addr("Subanim_0CirclesCentering");
    assert!(
        (0x4000..0x8000).contains(&addr),
        "expected banked ROM address ($4000-$7FFF), got {:#06X}",
        addr
    );
}

// ─── Core fix: HVFLIP instead of COORDFLIP ───────────────────────────

#[test]
fn header_uses_hvflip_not_coordflip() {
    let mut h = banked_harness();
    let addr = sym_addr("Subanim_0CirclesCentering");
    let header = rom(&mut h, addr);
    let expected = subanim_header(SUBANIMTYPE_HVFLIP, 6);
    let buggy = subanim_header(SUBANIMTYPE_COORDFLIP, 6);
    assert_eq!(
        header, expected,
        "header byte should be {:#04X} (HVFLIP|6), not {:#04X} (COORDFLIP|6); got {:#04X}",
        expected, buggy, header
    );
}

#[test]
fn header_type_bits_are_hvflip() {
    let mut h = banked_harness();
    let addr = sym_addr("Subanim_0CirclesCentering");
    let header = rom(&mut h, addr);
    let anim_type = header >> 5;
    assert_eq!(
        anim_type, SUBANIMTYPE_HVFLIP,
        "subanimation type should be SUBANIMTYPE_HVFLIP ({}), got {}",
        SUBANIMTYPE_HVFLIP, anim_type
    );
}

#[test]
fn header_frame_count_is_6() {
    let mut h = banked_harness();
    let addr = sym_addr("Subanim_0CirclesCentering");
    let header = rom(&mut h, addr);
    let count = header & 0x1F;
    assert_eq!(count, 6, "frame count should be 6, got {}", count);
}

// ─── Frame data integrity ────────────────────────────────────────────

#[test]
fn first_frame_entry_correct() {
    // db FRAMEBLOCK_44, BASECOORD_64, FRAMEBLOCKMODE_00
    let mut h = banked_harness();
    let base = sym_addr("Subanim_0CirclesCentering") + 1; // skip header
    assert_eq!(rom(&mut h, base), 0x44, "first entry: FRAMEBLOCK_44");
    assert_eq!(rom(&mut h, base + 1), 0x64, "first entry: BASECOORD_64");
    assert_eq!(
        rom(&mut h, base + 2),
        0x00,
        "first entry: FRAMEBLOCKMODE_00"
    );
}

#[test]
fn second_frame_entry_correct() {
    // db FRAMEBLOCK_45, BASECOORD_65, FRAMEBLOCKMODE_00
    let mut h = banked_harness();
    let base = sym_addr("Subanim_0CirclesCentering") + 1 + 3; // skip header + 1st entry
    assert_eq!(rom(&mut h, base), 0x45, "second entry: FRAMEBLOCK_45");
    assert_eq!(rom(&mut h, base + 1), 0x65, "second entry: BASECOORD_65");
    assert_eq!(
        rom(&mut h, base + 2),
        0x00,
        "second entry: FRAMEBLOCKMODE_00"
    );
}

#[test]
fn total_data_length_is_19_bytes() {
    // 1 header byte + 6 entries × 3 bytes = 19 bytes total.
    // The byte right after should be the header of the next subanim
    // (Subanim_0Circle_1Square_Appears), which uses SUBANIMTYPE_COORDFLIP, 1
    // → (3 << 5) | 1 = 0x61.
    let mut h = banked_harness();
    let base = sym_addr("Subanim_0CirclesCentering");
    let next = base + 19;
    let next_header = rom(&mut h, next);
    let expected_next = subanim_header(SUBANIMTYPE_COORDFLIP, 1);
    assert_eq!(
        next_header, expected_next,
        "byte at offset +19 should be next subanim header {:#04X} (COORDFLIP|1), got {:#04X}",
        expected_next, next_header
    );
}
