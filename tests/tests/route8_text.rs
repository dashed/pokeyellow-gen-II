//! ROM byte tests for the Route 8 Super Nerd battle text fix.
//!
//! Bug: The original text reads "how's your chem?" — the word "chemistry"
//! was truncated to "chem" to fit on one line, producing awkward phrasing.
//!
//! Fix: Reflow the text across a paragraph break: "how's your" / (para)
//! "chemistry grade?". The full word reads naturally with proper text box
//! scrolling. +3 bytes in text bank $28.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("_Route8SuperNerd1BattleText"));
    h
}

// Text command bytes (from constants/charmap.asm)
const TX_START: u8 = 0x00;
const PARA: u8 = 0x51;
const CONT: u8 = 0x55;
const DONE: u8 = 0x57;

// Character encodings (from constants/charmap.asm)
const CHAR_C: u8 = 0xA2; // 'c'
const CHAR_H: u8 = 0xA7; // 'h'
const CHAR_E: u8 = 0xA4; // 'e'
const CHAR_M: u8 = 0xAC; // 'm'
const CHAR_QUESTION: u8 = 0xE6; // '?'

/// Scan for a byte sequence in the ROM within a range.
fn find_bytes(h: &mut TestHarness, start: u16, end: u16, pattern: &[u8]) -> Option<u16> {
    if pattern.is_empty() {
        return None;
    }
    let search_end = end.saturating_sub(pattern.len() as u16 - 1);
    for addr in start..search_end {
        let mut matched = true;
        for (i, &byte) in pattern.iter().enumerate() {
            if rom(h, addr + i as u16) != byte {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(addr);
        }
    }
    None
}

/// Return the end of the text entry (address of DONE byte).
fn find_done(h: &mut TestHarness, start: u16) -> u16 {
    for addr in start..start + 200 {
        if rom(h, addr) == DONE {
            return addr;
        }
    }
    panic!("DONE byte not found within 200 bytes of start");
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn text_label_in_bank_28() {
    assert_eq!(sym_bank("_Route8SuperNerd1BattleText"), 0x28);
}

#[test]
fn text_label_in_banked_range() {
    let addr = sym_addr("_Route8SuperNerd1BattleText");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── Text structure tests ────────────────────────────────────────────

#[test]
fn text_starts_with_tx_start() {
    let mut h = banked_harness();
    let start = sym_addr("_Route8SuperNerd1BattleText");
    assert_eq!(
        rom(&mut h, start),
        TX_START,
        "text entry should start with TX_START"
    );
}

#[test]
fn cont_byte_present_for_hows_your() {
    // After LINE "#MON, but", there should be a CONT ($55) for "how's your"
    let mut h = banked_harness();
    let start = sym_addr("_Route8SuperNerd1BattleText");
    let done_addr = find_done(&mut h, start);
    let found = find_bytes(&mut h, start, done_addr, &[CONT]);
    assert!(found.is_some(), "CONT byte not found in text");
}

#[test]
fn para_byte_follows_hows_your() {
    // After CONT "how's your", the next control byte should be PARA ($51)
    // for "chemistry grade?"
    let mut h = banked_harness();
    let start = sym_addr("_Route8SuperNerd1BattleText");
    let done_addr = find_done(&mut h, start);
    let para = find_bytes(&mut h, start, done_addr, &[PARA]);
    assert!(
        para.is_some(),
        "PARA byte not found — text should have paragraph break"
    );
}

#[test]
fn chemistry_follows_para() {
    // After PARA ($51), the next bytes should encode "chemistry"
    // c=$A2, h=$A7, e=$A4, m=$AC, i=$A8, s=$B2, t=$B3, r=$B1, y=$B8
    let mut h = banked_harness();
    let start = sym_addr("_Route8SuperNerd1BattleText");
    let done_addr = find_done(&mut h, start);
    let para_addr = find_bytes(&mut h, start, done_addr, &[PARA]).expect("PARA not found");
    // "chemistry" = $A2 $A7 $A4 $AC $A8 $B2 $B3 $B1 $B8
    let expected = [0xA2, 0xA7, 0xA4, 0xAC, 0xA8, 0xB2, 0xB3, 0xB1, 0xB8];
    for (i, &byte) in expected.iter().enumerate() {
        assert_eq!(
            rom(&mut h, para_addr + 1 + i as u16),
            byte,
            "\"chemistry\" byte {} mismatch at offset {}",
            byte,
            i
        );
    }
}

#[test]
fn done_after_question_mark() {
    // The text should end with '?' ($E6) followed by DONE ($57)
    let mut h = banked_harness();
    let start = sym_addr("_Route8SuperNerd1BattleText");
    let done_addr = find_done(&mut h, start);
    assert_eq!(
        rom(&mut h, done_addr - 1),
        CHAR_QUESTION,
        "'?' should immediately precede DONE"
    );
}

// ─── Regression: no truncated "chem?" ────────────────────────────────

#[test]
fn no_truncated_chem_question() {
    // The old text had "chem?" ($A2 $A7 $A4 $AC $E6) followed by DONE ($57).
    // Verify this pattern does NOT appear.
    let mut h = banked_harness();
    let start = sym_addr("_Route8SuperNerd1BattleText");
    let done_addr = find_done(&mut h, start);
    let old_pattern = [CHAR_C, CHAR_H, CHAR_E, CHAR_M, CHAR_QUESTION, DONE];
    let found = find_bytes(&mut h, start, done_addr + 1, &old_pattern);
    assert!(
        found.is_none(),
        "found old truncated \"chem?\" + DONE pattern at {:#06X}",
        found.unwrap_or(0)
    );
}
