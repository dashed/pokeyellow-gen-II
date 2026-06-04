//! Emulator-based tests for the Lt. Surge gym trash can second lock fix.
//!
//! The bug: `TrashCanRandom.three` returns the result in register `b` instead
//! of `a`.  The caller (`TrashCanRandom`) reads from `a`, so for trash cans
//! with 3 valid neighbor pairs, the table offset is the raw random value
//! (0-255) instead of the intended [0,1,2].  This causes garbage second lock
//! selections — most commonly trash can 0 (top-left) regardless of which can
//! had the first lock.
//!
//! The fix: Rewrite `.three` to return the result in `a` using `jr nc` /
//! `xor a` / `ld a, 1` / `inc a`.  Same byte count (18 bytes), zero ROM growth.
//!
//! Reference: https://bulbapedia.bulbagarden.net/wiki/Vermilion_Gym#Gym_puzzle

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Orthogonal adjacency map for the 15 trash cans (3 rows × 5 columns).
///
/// ```text
///  0  1  2  3  4
///  5  6  7  8  9
/// 10 11 12 13 14
/// ```
const ADJACENT: &[&[u8]] = &[
    &[1, 3],         // 0
    &[0, 2, 4],      // 1
    &[1, 5],         // 2
    &[0, 4, 6],      // 3
    &[1, 3, 5, 7],   // 4
    &[2, 4, 8],      // 5
    &[3, 7, 9],      // 6
    &[4, 6, 8, 10],  // 7
    &[5, 7, 11],     // 8
    &[6, 10, 12],    // 9
    &[7, 9, 11, 13], // 10
    &[8, 10, 14],    // 11
    &[9, 13],        // 12
    &[10, 12, 14],   // 13
    &[11, 13],       // 14
];

fn is_valid_second_lock(first_can: u8, second_can: u8) -> bool {
    second_can == 0xFF || ADJACENT[first_can as usize].contains(&second_can)
}

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_three_has_jr_nc_not_ld_b() {
    // The fix replaces `ld b, 0` ($06 $00) after `cp $55` with `jr nc, .three_not_zero` ($30 xx)
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    let bank = sym_bank("TrashCanRandom.three");
    h.select_rom_bank(bank);

    let three = sym_addr("TrashCanRandom.three");

    // .three layout:
    //   +0: call Random     (CD xx xx)
    //   +3: swap a          (CB 37)
    //   +5: cp $55          (FE 55)
    //   +7: jr nc, ...      (30 xx)  ← the fix (was: ld b, 0 = 06 00)
    assert_eq!(h.read_mem(three + 5), 0xFE, "expected cp imm at .three+5");
    assert_eq!(h.read_mem(three + 6), 0x55, "expected $55 immediate");
    assert_eq!(
        h.read_mem(three + 7),
        0x30,
        "expected jr nc ($30) at .three+7 — the fix replaces ld b,0 ($06)"
    );
}

#[test]
fn rom_bytes_three_has_xor_a_for_zero_case() {
    // After jr nc, the zero case uses `xor a` ($AF) + `ret` ($C9)
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    let bank = sym_bank("TrashCanRandom.three");
    h.select_rom_bank(bank);

    let three = sym_addr("TrashCanRandom.three");

    // +9: xor a  ($AF)
    // +10: ret   ($C9)
    assert_eq!(h.read_mem(three + 9), 0xAF, "expected xor a at .three+9");
    assert_eq!(h.read_mem(three + 10), 0xC9, "expected ret at .three+10");
}

#[test]
fn rom_bytes_three_not_zero_has_ld_a_1() {
    // .three_not_zero: cp $AA, ld a, 1, ret c, inc a, ret
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    let bank = sym_bank("TrashCanRandom.three_not_zero");
    h.select_rom_bank(bank);

    let tnz = sym_addr("TrashCanRandom.three_not_zero");

    assert_eq!(h.read_mem(tnz), 0xFE, "expected cp imm at .three_not_zero");
    assert_eq!(h.read_mem(tnz + 1), 0xAA, "expected $AA immediate");
    assert_eq!(h.read_mem(tnz + 2), 0x3E, "expected ld a, imm ($3E)");
    assert_eq!(h.read_mem(tnz + 3), 0x01, "expected immediate $01");
    assert_eq!(h.read_mem(tnz + 4), 0xD8, "expected ret c ($D8)");
    assert_eq!(h.read_mem(tnz + 5), 0x3C, "expected inc a ($3C)");
    assert_eq!(h.read_mem(tnz + 6), 0xC9, "expected ret ($C9)");
}

#[test]
fn rom_bytes_jr_nc_offset_targets_three_not_zero() {
    // The jr nc at .three+7 should jump to .three_not_zero
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    let bank = sym_bank("TrashCanRandom.three");
    h.select_rom_bank(bank);

    let three = sym_addr("TrashCanRandom.three");
    let tnz = sym_addr("TrashCanRandom.three_not_zero");

    // jr nc is at .three+7, occupies 2 bytes (opcode + offset)
    // Target = PC_after_jr + offset = (.three + 9) + offset
    let offset = h.read_mem(three + 8) as i8;
    let target = (three + 9).wrapping_add(offset as u16);
    assert_eq!(
        target, tnz,
        "jr nc should target .three_not_zero (${tnz:04X}), got ${target:04X}"
    );
}

// ─── Behavioral tests ──────────────────────────────────────────────

/// Set up a headless harness ready to call Yellow_SampleSecondTrashCan.
fn setup_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00); // disable all interrupt sources
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    let bank = sym_bank("Yellow_SampleSecondTrashCan");
    h.select_rom_bank(bank);
    h.write_mem(0xFFB8, bank); // hLoadedROMBank

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h
}

/// Call Yellow_SampleSecondTrashCan and return the two second-lock can indices.
/// `seed` varies the RNG to ensure different random outcomes across calls.
fn call_sample(h: &mut TestHarness, can_index: u8, seed: u8) -> (u8, u8) {
    let func = sym_addr("Yellow_SampleSecondTrashCan");
    let second_lock = sym_addr("wSecondLockTrashCanIndex");

    h.write_mem(sym_addr("wGymTrashCanIndex"), can_index);
    // Seed the RNG state to get varied results across trials
    h.write_mem(sym_addr("hRandomAdd"), seed);
    h.write_mem(sym_addr("hRandomSub"), seed.wrapping_mul(7));
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(func);
    h.step_to(TRAP_ADDR);

    let a = h.read_mem(second_lock);
    let b = h.read_mem(second_lock + 1);
    (a, b)
}

#[test]
fn sample_returns_valid_adjacent_for_three_entry_cans() {
    // Cans 1,3,5,6,8,9,11,13 have 3 entries — these exercise the fixed .three case.
    let mut h = setup_harness();
    let three_entry_cans: &[u8] = &[1, 3, 5, 6, 8, 9, 11, 13];

    for &can in three_entry_cans {
        for trial in 0u8..50 {
            let (a, b) = call_sample(&mut h, can, trial.wrapping_mul(17).wrapping_add(can));
            assert!(
                is_valid_second_lock(can, a),
                "can {can}, trial {trial}: first lock index {a} (${a:02X}) is not adjacent \
                 (valid: {:?})",
                ADJACENT[can as usize]
            );
            assert!(
                is_valid_second_lock(can, b),
                "can {can}, trial {trial}: second lock index {b} (${b:02X}) is not adjacent \
                 (valid: {:?})",
                ADJACENT[can as usize]
            );
        }
    }
}

#[test]
fn sample_returns_valid_adjacent_for_all_cans() {
    // Verify all 15 cans produce valid results.
    let mut h = setup_harness();

    for can in 0u8..15 {
        for trial in 0u8..20 {
            let (a, b) = call_sample(&mut h, can, trial.wrapping_mul(31).wrapping_add(can));
            assert!(
                is_valid_second_lock(can, a),
                "can {can}, trial {trial}: first index {a} (${a:02X}) invalid (valid: {:?})",
                ADJACENT[can as usize]
            );
            assert!(
                is_valid_second_lock(can, b),
                "can {can}, trial {trial}: second index {b} (${b:02X}) invalid (valid: {:?})",
                ADJACENT[can as usize]
            );
        }
    }
}

#[test]
fn sample_three_entry_cans_produce_all_possible_pairs() {
    // For each 3-entry can, verify that all 3 pairs can be selected
    // (i.e., the random selection covers the full range, not just one value).
    let mut h = setup_harness();

    // Can 1 has 3 pairs: (0,2), (2,4), (4,0)
    let mut seen_pairs = std::collections::HashSet::new();
    for i in 0u8..=255 {
        let pair = call_sample(&mut h, 1, i);
        seen_pairs.insert(pair);
    }
    assert!(
        seen_pairs.len() >= 3,
        "can 1: expected at least 3 distinct pairs from 200 trials, got {} ({:?})",
        seen_pairs.len(),
        seen_pairs
    );
}
