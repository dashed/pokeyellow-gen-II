//! ROM byte tests for the splash screen extra stars fix.
//!
//! Bug: In `AnimateShootingStar`, `wMoveDownSmallStarsOAMCount` is
//! incremented by 6 (`add 6`) instead of 4 after each wave of 4 small
//! stars is placed. The extra 2 stars per wave are off-screen and
//! invisible, but waste OAM entries and CPU cycles.
//!
//! Fix: Change `add 6` to `add 4`. Zero ROM growth — only the
//! immediate operand changes.
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("AnimateShootingStar"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn animate_shooting_star_in_bank_1c() {
    assert_eq!(sym_bank("AnimateShootingStar"), 0x1C);
}

// ─── Loop counters ──────────────────────────────────────────────────

#[test]
fn outer_loop_counter_is_6() {
    let mut h = rom_harness();
    let small_loop = sym_addr("AnimateShootingStar.smallStarsLoop");
    // ld c, 6 → $0E $06 is at smallStarsLoop - 2
    assert_eq!(
        rom(&mut h, small_loop - 2),
        0x0E,
        "ld c, n opcode before .smallStarsLoop"
    );
    assert_eq!(
        rom(&mut h, small_loop - 1),
        0x06,
        "outer loop counter should be 6 (4 real waves + 2 empty)"
    );
}

#[test]
fn inner_loop_counter_is_4() {
    let mut h = rom_harness();
    let inner = sym_addr("AnimateShootingStar.smallStarsInnerLoop");
    // ld c, 4 → $0E $04 is at smallStarsInnerLoop - 2
    assert_eq!(
        rom(&mut h, inner - 2),
        0x0E,
        "ld c, n opcode before .smallStarsInnerLoop"
    );
    assert_eq!(
        rom(&mut h, inner - 1),
        0x04,
        "inner loop counter should be 4 (stars per wave)"
    );
}

#[test]
fn init_oam_count_is_24() {
    let mut h = rom_harness();
    let init_loop = sym_addr("AnimateShootingStar.initSmallStarsOAMLoop");
    // ld a, 24 → $3E $18 is at initSmallStarsOAMLoop - 2
    assert_eq!(
        rom(&mut h, init_loop - 2),
        0x3E,
        "ld a, n opcode before .initSmallStarsOAMLoop"
    );
    assert_eq!(
        rom(&mut h, init_loop - 1),
        0x18,
        "OAM init count should be 24 ($18)"
    );
}

// ─── OAM count cap ──────────────────────────────────────────────────

#[test]
fn cp_24_caps_oam_count() {
    let mut h = rom_harness();
    let next2 = sym_addr("AnimateShootingStar.next2");
    // Working backwards from .next2:
    //   -5: add N (2 bytes)
    //   -7: jr z (2 bytes)
    //   -9: cp 24 (2 bytes)
    let cp_addr = next2 - 9;
    assert_eq!(rom(&mut h, cp_addr), 0xFE, "cp n opcode");
    assert_eq!(
        rom(&mut h, cp_addr + 1),
        0x18,
        "cp operand should be 24 ($18)"
    );
}

// ─── THE FIX: add 4, not add 6 ──────────────────────────────────────

#[test]
fn add_operand_is_4() {
    let mut h = rom_harness();
    let next2 = sym_addr("AnimateShootingStar.next2");
    // add N is at .next2 - 5
    let add_addr = next2 - 5;
    assert_eq!(rom(&mut h, add_addr), 0xC6, "add n opcode");
    assert_eq!(
        rom(&mut h, add_addr + 1),
        0x04,
        "add operand should be 4 (one per star placed), not 6"
    );
}

#[test]
fn add_operand_is_not_buggy_6() {
    let mut h = rom_harness();
    let next2 = sym_addr("AnimateShootingStar.next2");
    let add_addr = next2 - 5;
    assert_ne!(
        rom(&mut h, add_addr + 1),
        0x06,
        "add operand must NOT be 6 — that was the bug"
    );
}

// ─── Wave pointer table ─────────────────────────────────────────────

#[test]
fn wave_pointer_table_has_4_real_waves_and_2_empty() {
    let mut h = rom_harness();
    let table = sym_addr("SmallStarsWaveCoordsPointerTable");
    let wave1 = sym_addr("SmallStarsWave1Coords");
    let empty = sym_addr("SmallStarsEmptyWave");

    // First entry should point to SmallStarsWave1Coords
    let lo = rom(&mut h, table) as u16;
    let hi = rom(&mut h, table + 1) as u16;
    assert_eq!(
        lo | (hi << 8),
        wave1,
        "first wave pointer should be SmallStarsWave1Coords"
    );

    // 5th and 6th entries (at +8 and +10) should point to SmallStarsEmptyWave
    let lo5 = rom(&mut h, table + 8) as u16;
    let hi5 = rom(&mut h, table + 9) as u16;
    assert_eq!(
        lo5 | (hi5 << 8),
        empty,
        "5th wave pointer should be SmallStarsEmptyWave"
    );

    let lo6 = rom(&mut h, table + 10) as u16;
    let hi6 = rom(&mut h, table + 11) as u16;
    assert_eq!(
        lo6 | (hi6 << 8),
        empty,
        "6th wave pointer should be SmallStarsEmptyWave"
    );
}
