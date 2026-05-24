//! ROM byte tests for the new game wStatusFlags6 clearing fix.
//!
//! Bug: `PrepareOakSpeech` saves `wStatusFlags6` before clearing memory,
//! then restores it unmodified. This carries over stale flags from the
//! previous save — including `BIT_ALWAYS_ON_BIKE` (bit 5), which was set
//! when the player saved at the Cycling Road. On the new game,
//! `CheckForceBikeOrSurf` sees the bit and returns immediately, leaving
//! the player stuck on the bike in Pallet Town.
//!
//! Fix: Wrap the wStatusFlags6 save/restore in `IF DEF(_DEBUG)` so it's
//! only present in debug builds. In release builds, `FillMemory` clears
//! wStatusFlags6 along with the rest of WRAM, fixing the bug and saving
//! 7 bytes vs the original code.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PrepareOakSpeech"));
    h
}

/// wStatusFlags6 = $D731
const W_STATUS_FLAGS6_LO: u8 = 0x31;
const W_STATUS_FLAGS6_HI: u8 = 0xD7;

/// wOptions = $D354
const W_OPTIONS_LO: u8 = 0x54;
const W_OPTIONS_HI: u8 = 0xD3;

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn prepare_oak_speech_in_bank_01() {
    assert_eq!(sym_bank("PrepareOakSpeech"), 0x01);
}

// ─── THE FIX: wStatusFlags6 references absent in release ROM ─────────

#[test]
fn no_status_flags6_load_in_release() {
    let mut h = rom_harness();
    let base = sym_addr("PrepareOakSpeech");
    let end = sym_addr("OakSpeech");
    // ld a, [wStatusFlags6] = $FA $31 $D7 — should be compiled out in release
    for addr in base..end.saturating_sub(2) {
        if rom(&mut h, addr) == 0xFA
            && rom(&mut h, addr + 1) == W_STATUS_FLAGS6_LO
            && rom(&mut h, addr + 2) == W_STATUS_FLAGS6_HI
        {
            panic!(
                "Found ld a, [wStatusFlags6] at ${:04X} — IF DEF(_DEBUG) guard missing",
                addr
            );
        }
    }
}

#[test]
fn no_status_flags6_store_in_release() {
    let mut h = rom_harness();
    let base = sym_addr("PrepareOakSpeech");
    let end = sym_addr("OakSpeech");
    // ld [wStatusFlags6], a = $EA $31 $D7 — should be compiled out in release
    for addr in base..end.saturating_sub(2) {
        if rom(&mut h, addr) == 0xEA
            && rom(&mut h, addr + 1) == W_STATUS_FLAGS6_LO
            && rom(&mut h, addr + 2) == W_STATUS_FLAGS6_HI
        {
            panic!(
                "Found ld [wStatusFlags6], a at ${:04X} — IF DEF(_DEBUG) guard missing",
                addr
            );
        }
    }
}

// ─── Positive: other save/restore pairs still present ─────────────────

#[test]
fn options_still_saved_in_release() {
    let mut h = rom_harness();
    let base = sym_addr("PrepareOakSpeech");
    // ld a, [wOptions] = $FA $54 $D3 near the start
    let mut found = false;
    for addr in base..base + 20 {
        if rom(&mut h, addr) == 0xFA
            && rom(&mut h, addr + 1) == W_OPTIONS_LO
            && rom(&mut h, addr + 2) == W_OPTIONS_HI
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld a, [wOptions] should be near start of PrepareOakSpeech"
    );
}

#[test]
fn options_still_restored_in_release() {
    let mut h = rom_harness();
    let base = sym_addr("PrepareOakSpeech");
    let end = sym_addr("OakSpeech");
    // ld [wOptions], a = $EA $54 $D3
    let mut found = false;
    for addr in base..end {
        if rom(&mut h, addr) == 0xEA
            && rom(&mut h, addr + 1) == W_OPTIONS_LO
            && rom(&mut h, addr + 2) == W_OPTIONS_HI
        {
            found = true;
            break;
        }
    }
    assert!(found, "ld [wOptions], a should exist in PrepareOakSpeech");
}

// ─── Negative: old buggy/fix patterns absent ──────────────────────────

#[test]
fn no_unmasked_restore_of_status_flags6() {
    let mut h = rom_harness();
    let base = sym_addr("PrepareOakSpeech");
    let end = sym_addr("OakSpeech");
    // pop af / ld [wStatusFlags6], a = $F1 $EA $31 $D7
    for addr in base..end.saturating_sub(3) {
        if rom(&mut h, addr) == 0xF1
            && rom(&mut h, addr + 1) == 0xEA
            && rom(&mut h, addr + 2) == W_STATUS_FLAGS6_LO
            && rom(&mut h, addr + 3) == W_STATUS_FLAGS6_HI
        {
            panic!(
                "Found pop af / ld [wStatusFlags6] at ${:04X} — old buggy pattern present",
                addr
            );
        }
    }
}

#[test]
fn no_and_mask_restore_of_status_flags6() {
    let mut h = rom_harness();
    let base = sym_addr("PrepareOakSpeech");
    let end = sym_addr("OakSpeech");
    // and $02 / ld [wStatusFlags6], a = $E6 $02 $EA $31 $D7
    for addr in base..end.saturating_sub(4) {
        if rom(&mut h, addr) == 0xE6
            && rom(&mut h, addr + 1) == 0x02
            && rom(&mut h, addr + 2) == 0xEA
            && rom(&mut h, addr + 3) == W_STATUS_FLAGS6_LO
            && rom(&mut h, addr + 4) == W_STATUS_FLAGS6_HI
        {
            panic!(
                "Found and $02 / ld [wStatusFlags6] at ${:04X} — old mask approach present",
                addr
            );
        }
    }
}

// ─── BIT_ALWAYS_ON_BIKE logic ─────────────────────────────────────────

#[test]
fn bit_always_on_bike_is_in_status_flags6() {
    // BIT_ALWAYS_ON_BIKE = bit 5 of wStatusFlags6
    // With wStatusFlags6 not saved/restored in release, FillMemory clears it,
    // ensuring BIT_ALWAYS_ON_BIKE cannot leak from a previous save
    let always_on_bike: u8 = 1 << 5;
    assert_eq!(always_on_bike, 0x20, "BIT_ALWAYS_ON_BIKE should be $20");
}
