//! ROM byte tests for the experience underflow fix.
//!
//! Bug: the Medium Slow growth rate formula produces a negative experience
//! value at level 1 (6/5·1³ − 15·1² + 100·1 − 140 = −54).  Since experience
//! is stored as an unsigned 3-byte value, −54 wraps to $FFFFCA (~16.7M).
//! When the game recalculates the Pokémon's level, it determines it should
//! be level 100 based on this huge stored value.
//!
//! Fix: at the end of `CalcExperience`, check if the high byte has bit 7
//! set (indicating underflow — legitimate exp values never exceed ~1.25M =
//! $1312D0).  If set, clamp all 3 bytes to 0.  +10 bytes in bank $16.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/Experience#Experience_underflow_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn find_pattern(h: &mut TestHarness, start: u16, end: u16, pattern: &[u8]) -> Option<u16> {
    if pattern.is_empty() || end <= start {
        return None;
    }
    let len = pattern.len() as u16;
    for addr in start..=(end.saturating_sub(len)) {
        if (0..len).all(|i| rom(h, addr + i) == pattern[i as usize]) {
            return Some(addr);
        }
    }
    None
}

// ─── Opcode constants ────────────────────────────────────────────────

const BIT_7_A: [u8; 2] = [0xCB, 0x7F]; // bit 7, a
const RET_Z: u8 = 0xC8;
const XOR_A: u8 = 0xAF;
const LDH_N_A: u8 = 0xE0; // ldh [n], a

// hExperience = $FF96, so ldh [hExperience], a = E0 96
const H_EXPERIENCE: u8 = 0x96;
const H_EXPERIENCE_1: u8 = 0x97;
const H_EXPERIENCE_2: u8 = 0x98;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn calc_experience_is_in_bank_16() {
    assert_eq!(
        sym_bank("CalcExperience"),
        0x16,
        "CalcExperience should be in bank $16"
    );
}

#[test]
fn add_cubed_term_has_bit_7_check() {
    // After .addCubedTerm writes hExperience, bit 7,a should follow
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CalcExperience"));
    let start = sym_addr("CalcExperience.addCubedTerm");
    let end = start + 0x30; // search within reasonable range
    let found = find_pattern(&mut h, start, end, &BIT_7_A);
    assert!(
        found.is_some(),
        "CalcExperience should have `bit 7, a` after .addCubedTerm to detect underflow"
    );
}

#[test]
fn ret_z_follows_bit_7_check() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CalcExperience"));
    let start = sym_addr("CalcExperience.addCubedTerm");
    let end = start + 0x30;
    let bit7_addr = find_pattern(&mut h, start, end, &BIT_7_A).expect("bit 7, a should exist");
    assert_eq!(
        rom(&mut h, bit7_addr + 2),
        RET_Z,
        "ret z should follow `bit 7, a` (return if non-negative)"
    );
}

#[test]
fn xor_a_follows_ret_z() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CalcExperience"));
    let start = sym_addr("CalcExperience.addCubedTerm");
    let end = start + 0x30;
    let bit7_addr = find_pattern(&mut h, start, end, &BIT_7_A).expect("bit 7, a should exist");
    // After bit 7,a (2 bytes) + ret z (1 byte) = offset 3
    assert_eq!(
        rom(&mut h, bit7_addr + 3),
        XOR_A,
        "xor a should follow ret z (zero a for clamping)"
    );
}

#[test]
fn clamp_writes_all_three_experience_bytes() {
    // After xor a, should write to hExperience, hExperience+1, hExperience+2
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CalcExperience"));
    let start = sym_addr("CalcExperience.addCubedTerm");
    let end = start + 0x30;
    let bit7_addr = find_pattern(&mut h, start, end, &BIT_7_A).expect("bit 7, a should exist");
    let clamp_start = bit7_addr + 4; // after bit 7,a + ret z + xor a
                                     // Should have: ldh [hExperience], a / ldh [hExperience+1], a / ldh [hExperience+2], a
    assert_eq!(
        rom(&mut h, clamp_start),
        LDH_N_A,
        "first clamp should be ldh [n], a"
    );
    assert_eq!(
        rom(&mut h, clamp_start + 1),
        H_EXPERIENCE,
        "first clamp should write hExperience ($96)"
    );
    assert_eq!(
        rom(&mut h, clamp_start + 2),
        LDH_N_A,
        "second clamp should be ldh [n], a"
    );
    assert_eq!(
        rom(&mut h, clamp_start + 3),
        H_EXPERIENCE_1,
        "second clamp should write hExperience+1 ($97)"
    );
    assert_eq!(
        rom(&mut h, clamp_start + 4),
        LDH_N_A,
        "third clamp should be ldh [n], a"
    );
    assert_eq!(
        rom(&mut h, clamp_start + 5),
        H_EXPERIENCE_2,
        "third clamp should write hExperience+2 ($98)"
    );
}

#[test]
fn clamp_ends_with_ret() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CalcExperience"));
    let start = sym_addr("CalcExperience.addCubedTerm");
    let end = start + 0x30;
    let bit7_addr = find_pattern(&mut h, start, end, &BIT_7_A).expect("bit 7, a should exist");
    // bit 7,a (2) + ret z (1) + xor a (1) + 3×ldh (6) = offset 10
    assert_eq!(
        rom(&mut h, bit7_addr + 10),
        0xC9, // ret
        "clamp should end with ret"
    );
}

#[test]
fn original_ldh_h_experience_preserved() {
    // The original `ldh [hExperience], a` that stores the cubed term result
    // should still exist before the bit 7 check.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CalcExperience"));
    let start = sym_addr("CalcExperience.addCubedTerm");
    let end = start + 0x20;
    let bit7_addr = find_pattern(&mut h, start, end, &BIT_7_A).expect("bit 7, a should exist");
    // Search for ldh [hExperience], a BEFORE the bit 7 check
    let original_ldh = find_pattern(&mut h, start, bit7_addr, &[LDH_N_A, H_EXPERIENCE]);
    assert!(
        original_ldh.is_some(),
        "original ldh [hExperience], a should exist before bit 7 check"
    );
}

#[test]
fn calc_d_squared_still_follows() {
    // CalcDSquared should still exist after CalcExperience (it's the next function)
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CalcExperience"));
    let calc_d_squared = sym_addr("CalcDSquared");
    let add_cubed = sym_addr("CalcExperience.addCubedTerm");
    assert!(
        calc_d_squared > add_cubed,
        "CalcDSquared should follow CalcExperience.addCubedTerm"
    );
    // CalcDSquared starts with xor a (AF)
    assert_eq!(
        rom(&mut h, calc_d_squared),
        XOR_A,
        "CalcDSquared should start with xor a"
    );
}
