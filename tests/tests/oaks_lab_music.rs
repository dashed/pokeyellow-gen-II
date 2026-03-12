//! ROM byte tests for the Oak's lab music V-Blank channel cut-off fix.
//!
//! Bug: In `OaksLabFollowedOakScript`, after clearing `BIT_NO_MAP_MUSIC`
//! in `wStatusFlags7`, `PlayDefaultMusic` is called immediately. If a
//! V-Blank interrupt fires between the flag clear and the music start,
//! one of the audio channels can be cut off.
//!
//! Fix: Insert `call DelayFrame` between `res BIT_NO_MAP_MUSIC, [hl]`
//! and `call PlayDefaultMusic` so a full V-Blank completes before the
//! music engine initializes all channels. +3 bytes in bank $07.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("OaksLabFollowedOakScript"));
    h
}

/// Scan for `ld hl, wStatusFlags7` ($21 $32 $D7) starting from `base`.
fn find_ld_hl_status_flags7(h: &mut TestHarness, base: u16, end: u16) -> Option<u16> {
    // wStatusFlags7 = $D732 → lo=$32, hi=$D7
    (base..end)
        .find(|&addr| rom(h, addr) == 0x21 && rom(h, addr + 1) == 0x32 && rom(h, addr + 2) == 0xD7)
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn oaks_lab_followed_oak_script_in_bank_07() {
    assert_eq!(sym_bank("OaksLabFollowedOakScript"), 0x07);
}

#[test]
fn oaks_lab_followed_oak_script_in_banked_range() {
    let addr = sym_addr("OaksLabFollowedOakScript");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── Core fix: DelayFrame inserted before PlayDefaultMusic ───────────

#[test]
fn ld_hl_status_flags7_found() {
    let mut h = banked_harness();
    let base = sym_addr("OaksLabFollowedOakScript");
    let end = base + 40; // function is small
    assert!(
        find_ld_hl_status_flags7(&mut h, base, end).is_some(),
        "ld hl, wStatusFlags7 not found in OaksLabFollowedOakScript"
    );
}

#[test]
fn res_bit_no_map_music_after_ld_hl() {
    let mut h = banked_harness();
    let base = sym_addr("OaksLabFollowedOakScript");
    let ld_hl = find_ld_hl_status_flags7(&mut h, base, base + 40).unwrap();
    let after = ld_hl + 3; // after 3-byte ld hl, imm16
                           // res 1, [hl] = CB 8E (BIT_NO_MAP_MUSIC = 1)
    assert_eq!(rom(&mut h, after), 0xCB, "CB prefix for res instruction");
    assert_eq!(
        rom(&mut h, after + 1),
        0x8E,
        "res 1, [hl] opcode (BIT_NO_MAP_MUSIC = 1)"
    );
}

#[test]
fn call_delay_frame_after_res() {
    let mut h = banked_harness();
    let base = sym_addr("OaksLabFollowedOakScript");
    let ld_hl = find_ld_hl_status_flags7(&mut h, base, base + 40).unwrap();
    let after_res = ld_hl + 3 + 2; // ld hl (3) + res (2)
    let delay_frame = sym_addr("DelayFrame");
    let lo = (delay_frame & 0xFF) as u8;
    let hi = (delay_frame >> 8) as u8;
    assert_eq!(
        rom(&mut h, after_res),
        0xCD,
        "call opcode after res (should be call DelayFrame)"
    );
    assert_eq!(rom(&mut h, after_res + 1), lo, "DelayFrame low byte");
    assert_eq!(rom(&mut h, after_res + 2), hi, "DelayFrame high byte");
}

#[test]
fn call_play_default_music_after_delay_frame() {
    let mut h = banked_harness();
    let base = sym_addr("OaksLabFollowedOakScript");
    let ld_hl = find_ld_hl_status_flags7(&mut h, base, base + 40).unwrap();
    let after_delay = ld_hl + 3 + 2 + 3; // ld hl (3) + res (2) + call DelayFrame (3)
    let play_music = sym_addr("PlayDefaultMusic");
    let lo = (play_music & 0xFF) as u8;
    let hi = (play_music >> 8) as u8;
    assert_eq!(
        rom(&mut h, after_delay),
        0xCD,
        "call opcode after DelayFrame (should be call PlayDefaultMusic)"
    );
    assert_eq!(
        rom(&mut h, after_delay + 1),
        lo,
        "PlayDefaultMusic low byte"
    );
    assert_eq!(
        rom(&mut h, after_delay + 2),
        hi,
        "PlayDefaultMusic high byte"
    );
}

#[test]
fn full_sequence_is_11_bytes() {
    // ld hl, imm16 (3) + res 1, [hl] (2) + call DelayFrame (3) + call PlayDefaultMusic (3) = 11
    let mut h = banked_harness();
    let base = sym_addr("OaksLabFollowedOakScript");
    let ld_hl = find_ld_hl_status_flags7(&mut h, base, base + 40).unwrap();
    let end_of_sequence = ld_hl + 11;
    // Byte after the sequence should be `ld a, SCRIPT_OAKSLAB_OAK_CHOOSE_MON_SPEECH`
    // which is $3E xx (ld a, imm8)
    assert_eq!(
        rom(&mut h, end_of_sequence),
        0x3E,
        "ld a, imm8 follows the music setup sequence"
    );
}

#[test]
fn delay_frame_is_in_home_bank() {
    assert_eq!(
        sym_bank("DelayFrame"),
        0x00,
        "DelayFrame must be in HOME bank to be callable from bank $07"
    );
}
