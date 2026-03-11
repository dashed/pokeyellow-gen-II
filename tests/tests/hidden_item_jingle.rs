//! ROM byte tests for the hidden item jingle fade-out fix.
//!
//! Bug: In `FoundHiddenItemText`, the "item acquired" jingle
//! (`SFX_GET_ITEM_2`) can be cut off if `wAudioFadeOutControl` is
//! non-zero when the sound plays. The fade-out counter interferes
//! with the jingle playback.
//!
//! Fix: Save and clear `wAudioFadeOutControl` before playing the
//! jingle, then restore it afterward. +12 bytes in bank $1D.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("FoundHiddenItemText"));
    h
}

/// Scan for `call PlaySoundWaitForCurrent` ($CD lo hi) within a range.
fn find_call_play_sound(h: &mut TestHarness, start: u16, end: u16) -> Option<u16> {
    let target = sym_addr("PlaySoundWaitForCurrent");
    let lo = (target & 0xFF) as u8;
    let hi = (target >> 8) as u8;
    for addr in start..end {
        if rom(h, addr) == 0xCD && rom(h, addr + 1) == lo && rom(h, addr + 2) == hi {
            return Some(addr);
        }
    }
    None
}

/// Scan for `call WaitForSoundToFinish` ($CD lo hi) within a range.
fn find_call_wait_for_sound(h: &mut TestHarness, start: u16, end: u16) -> Option<u16> {
    let target = sym_addr("WaitForSoundToFinish");
    let lo = (target & 0xFF) as u8;
    let hi = (target >> 8) as u8;
    for addr in start..end {
        if rom(h, addr) == 0xCD && rom(h, addr + 1) == lo && rom(h, addr + 2) == hi {
            return Some(addr);
        }
    }
    None
}

// wAudioFadeOutControl = $CFC6
const W_AUDIO_FADE_OUT_CONTROL: u16 = 0xCFC6;

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn found_hidden_item_text_in_bank_1d() {
    assert_eq!(sym_bank("FoundHiddenItemText"), 0x1D);
}

#[test]
fn found_hidden_item_text_in_banked_range() {
    let addr = sym_addr("FoundHiddenItemText");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── Core fix: save/clear wAudioFadeOutControl before jingle ─────────

#[test]
fn load_fade_control_before_sfx() {
    // Before call PlaySoundWaitForCurrent, expect:
    // ld a, [wAudioFadeOutControl] ($FA $C6 $CF) at offset -8
    // push af ($F5) at -5
    // xor a ($AF) at -4
    // ld [wAudioFadeOutControl], a ($EA $C6 $CF) at -3
    let mut h = banked_harness();
    let base = sym_addr("FoundHiddenItemText");
    let bag_full = sym_addr("FoundHiddenItemText.bagFull");
    let call_play = find_call_play_sound(&mut h, base, bag_full).unwrap();
    // ld a, SFX_GET_ITEM_2 is at call_play - 2 (ld a, imm8 = $3E xx)
    // ld [wAudioFadeOutControl], a is at call_play - 5 ($EA lo hi)
    // xor a is at call_play - 6 ($AF)
    // push af is at call_play - 7 ($F5)
    // ld a, [wAudioFadeOutControl] is at call_play - 10 ($FA lo hi)
    let fade_load = call_play - 10;
    let lo = (W_AUDIO_FADE_OUT_CONTROL & 0xFF) as u8;
    let hi = (W_AUDIO_FADE_OUT_CONTROL >> 8) as u8;
    assert_eq!(rom(&mut h, fade_load), 0xFA, "ld a, [imm16] opcode");
    assert_eq!(rom(&mut h, fade_load + 1), lo, "wAudioFadeOutControl low");
    assert_eq!(rom(&mut h, fade_load + 2), hi, "wAudioFadeOutControl high");
}

#[test]
fn push_af_saves_fade_state() {
    let mut h = banked_harness();
    let base = sym_addr("FoundHiddenItemText");
    let bag_full = sym_addr("FoundHiddenItemText.bagFull");
    let call_play = find_call_play_sound(&mut h, base, bag_full).unwrap();
    assert_eq!(
        rom(&mut h, call_play - 7),
        0xF5,
        "push af to save fade-out state"
    );
}

#[test]
fn xor_a_and_store_clears_fade_counter() {
    let mut h = banked_harness();
    let base = sym_addr("FoundHiddenItemText");
    let bag_full = sym_addr("FoundHiddenItemText.bagFull");
    let call_play = find_call_play_sound(&mut h, base, bag_full).unwrap();
    let lo = (W_AUDIO_FADE_OUT_CONTROL & 0xFF) as u8;
    let hi = (W_AUDIO_FADE_OUT_CONTROL >> 8) as u8;
    assert_eq!(rom(&mut h, call_play - 6), 0xAF, "xor a to clear A");
    assert_eq!(
        rom(&mut h, call_play - 5),
        0xEA,
        "ld [imm16], a opcode (store 0)"
    );
    assert_eq!(
        rom(&mut h, call_play - 4),
        lo,
        "wAudioFadeOutControl low (clear)"
    );
    assert_eq!(
        rom(&mut h, call_play - 3),
        hi,
        "wAudioFadeOutControl high (clear)"
    );
}

#[test]
fn sfx_get_item_2_loaded_before_play() {
    let mut h = banked_harness();
    let base = sym_addr("FoundHiddenItemText");
    let bag_full = sym_addr("FoundHiddenItemText.bagFull");
    let call_play = find_call_play_sound(&mut h, base, bag_full).unwrap();
    // ld a, SFX_GET_ITEM_2 = $3E xx at call_play - 2
    assert_eq!(
        rom(&mut h, call_play - 2),
        0x3E,
        "ld a, imm8 for SFX_GET_ITEM_2"
    );
}

// ─── Restore: pop af + store after WaitForSoundToFinish ──────────────

#[test]
fn pop_af_restores_after_wait() {
    let mut h = banked_harness();
    let base = sym_addr("FoundHiddenItemText");
    let bag_full = sym_addr("FoundHiddenItemText.bagFull");
    let call_wait = find_call_wait_for_sound(&mut h, base, bag_full).unwrap();
    // After call WaitForSoundToFinish (3 bytes): pop af ($F1)
    assert_eq!(
        rom(&mut h, call_wait + 3),
        0xF1,
        "pop af after WaitForSoundToFinish"
    );
}

#[test]
fn restore_fade_control_after_pop() {
    let mut h = banked_harness();
    let base = sym_addr("FoundHiddenItemText");
    let bag_full = sym_addr("FoundHiddenItemText.bagFull");
    let call_wait = find_call_wait_for_sound(&mut h, base, bag_full).unwrap();
    let lo = (W_AUDIO_FADE_OUT_CONTROL & 0xFF) as u8;
    let hi = (W_AUDIO_FADE_OUT_CONTROL >> 8) as u8;
    // pop af at +3, then ld [wAudioFadeOutControl], a ($EA lo hi) at +4
    assert_eq!(
        rom(&mut h, call_wait + 4),
        0xEA,
        "ld [imm16], a opcode (restore)"
    );
    assert_eq!(
        rom(&mut h, call_wait + 5),
        lo,
        "wAudioFadeOutControl low (restore)"
    );
    assert_eq!(
        rom(&mut h, call_wait + 6),
        hi,
        "wAudioFadeOutControl high (restore)"
    );
}
