//! ROM byte tests for the audio engine pitch slide high-byte borrow fix.
//!
//! Bug: In `Audio1_InitPitchSlideVars.targetFrequencyGreater`, when
//! computing the frequency difference for pitch slides, the code
//! borrowed from the high byte of the **current** frequency instead
//! of the **target** frequency. When a borrow occurs (target_lo <
//! current_lo), the result is $200 greater than intended.
//!
//! Fix: Load `wChannelPitchSlideTargetFrequencyHighBytes` before
//! `sbc b` so the borrow applies to the target high byte. Replaces
//! 10 bytes with 8 bytes (−2 bytes in bank $02).

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("Audio1_InitPitchSlideVars"));
    h
}

// wChannelPitchSlideTargetFrequencyHighBytes = $C0A6
const W_TARGET_FREQ_HIGH: u16 = 0xC0A6;

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn init_pitch_slide_vars_in_bank_02() {
    assert_eq!(sym_bank("Audio1_InitPitchSlideVars"), 0x02);
}

#[test]
fn target_frequency_greater_in_banked_range() {
    let addr = sym_addr("Audio1_InitPitchSlideVars.targetFrequencyGreater");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── Core fix: target high byte loaded before sbc ────────────────────

#[test]
fn target_freq_greater_starts_with_ld_hl_target_high() {
    // .targetFrequencyGreater should load current freq into d/e first,
    // then do the low byte subtraction. After that, the fix loads the
    // target high byte array. We verify the ld hl at the fix site.
    let mut h = banked_harness();
    let tgt = sym_addr("Audio1_InitPitchSlideVars.targetFrequencyGreater");
    // The function loads current_hi→d, current_lo→e, does target_lo - current_lo.
    // Then the fix: ld hl, wChannelPitchSlideTargetFrequencyHighBytes
    // Scan from .targetFrequencyGreater for the pattern $21 lo hi (ld hl, imm16)
    // where lo/hi = W_TARGET_FREQ_HIGH
    let lo = (W_TARGET_FREQ_HIGH & 0xFF) as u8;
    let hi = (W_TARGET_FREQ_HIGH >> 8) as u8;
    let next2 = sym_addr("Audio1_InitPitchSlideVars.next2");
    let mut found = false;
    for addr in tgt..next2 {
        if rom(&mut h, addr) == 0x21 && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld hl, wChannelPitchSlideTargetFrequencyHighBytes not found in .targetFrequencyGreater"
    );
}

#[test]
fn sbc_b_follows_target_high_load() {
    // After ld hl, target_high (3) + add hl, bc (1) + ld a, [hl] (1),
    // the next instruction should be sbc b ($98).
    let mut h = banked_harness();
    let tgt = sym_addr("Audio1_InitPitchSlideVars.targetFrequencyGreater");
    let next2 = sym_addr("Audio1_InitPitchSlideVars.next2");
    let lo = (W_TARGET_FREQ_HIGH & 0xFF) as u8;
    let hi = (W_TARGET_FREQ_HIGH >> 8) as u8;
    for addr in tgt..next2 {
        if rom(&mut h, addr) == 0x21 && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            // ld hl, imm16 (3) + add hl, bc (1) + ld a, [hl] (1) = +5
            let sbc_addr = addr + 5;
            assert_eq!(
                rom(&mut h, sbc_addr),
                0x98,
                "sbc b opcode after loading target high byte"
            );
            return;
        }
    }
    panic!("target high byte load not found");
}

#[test]
fn sub_d_follows_sbc_b() {
    // After sbc b ($98), the next instruction should be sub d ($92).
    let mut h = banked_harness();
    let tgt = sym_addr("Audio1_InitPitchSlideVars.targetFrequencyGreater");
    let next2 = sym_addr("Audio1_InitPitchSlideVars.next2");
    let lo = (W_TARGET_FREQ_HIGH & 0xFF) as u8;
    let hi = (W_TARGET_FREQ_HIGH >> 8) as u8;
    for addr in tgt..next2 {
        if rom(&mut h, addr) == 0x21 && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            let sub_d_addr = addr + 6; // +5 (sbc b) +1
            assert_eq!(
                rom(&mut h, sub_d_addr),
                0x92,
                "sub d opcode (subtract current high byte)"
            );
            return;
        }
    }
    panic!("target high byte load not found");
}

#[test]
fn ld_d_a_saves_result() {
    // After sub d ($92), the next instruction should be ld d, a ($57... wait, that's wrong)
    // ld d, a = $57. Actually no: ld d, a = $57.
    // Wait: ld d, a is opcode $57.
    let mut h = banked_harness();
    let tgt = sym_addr("Audio1_InitPitchSlideVars.targetFrequencyGreater");
    let next2 = sym_addr("Audio1_InitPitchSlideVars.next2");
    let lo = (W_TARGET_FREQ_HIGH & 0xFF) as u8;
    let hi = (W_TARGET_FREQ_HIGH >> 8) as u8;
    for addr in tgt..next2 {
        if rom(&mut h, addr) == 0x21 && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            let ld_d_a_addr = addr + 7; // +5 sbc b, +6 sub d, +7 ld d, a
            assert_eq!(
                rom(&mut h, ld_d_a_addr),
                0x57,
                "ld d, a saves the corrected high byte difference"
            );
            return;
        }
    }
    panic!("target high byte load not found");
}

// ─── Regression: no buggy pattern ────────────────────────────────────

#[test]
fn no_ld_a_d_before_sbc_b_in_target_greater() {
    // The old buggy pattern was: ld a, d ($7A) immediately before sbc b ($98).
    // Verify this does NOT appear in .targetFrequencyGreater.
    let mut h = banked_harness();
    let tgt = sym_addr("Audio1_InitPitchSlideVars.targetFrequencyGreater");
    let next2 = sym_addr("Audio1_InitPitchSlideVars.next2");
    for addr in tgt..next2 - 1 {
        if rom(&mut h, addr) == 0x7A && rom(&mut h, addr + 1) == 0x98 {
            panic!(
                "found buggy ld a, d + sbc b pattern at {:#06X} in .targetFrequencyGreater",
                addr
            );
        }
    }
}

#[test]
fn fix_sequence_is_8_bytes() {
    // The fixed sequence: ld hl, imm16 (3) + add hl, bc (1) + ld a, [hl] (1) +
    // sbc b (1) + sub d (1) + ld d, a (1) = 8 bytes.
    // Followed by ld b, 0 ($06 $00) at .next2 - 5 or similar.
    let mut h = banked_harness();
    let tgt = sym_addr("Audio1_InitPitchSlideVars.targetFrequencyGreater");
    let next2 = sym_addr("Audio1_InitPitchSlideVars.next2");
    let lo = (W_TARGET_FREQ_HIGH & 0xFF) as u8;
    let hi = (W_TARGET_FREQ_HIGH >> 8) as u8;
    for addr in tgt..next2 {
        if rom(&mut h, addr) == 0x21 && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            // ld b, 0 should follow at addr + 8
            assert_eq!(
                rom(&mut h, addr + 8),
                0x06,
                "ld b, imm8 follows the 8-byte fix sequence"
            );
            assert_eq!(
                rom(&mut h, addr + 9),
                0x00,
                "ld b, 0 clears b after computation"
            );
            return;
        }
    }
    panic!("target high byte load not found");
}
