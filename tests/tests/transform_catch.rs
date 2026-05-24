//! Emulator-based tests for the Transform/Ditto catch bug fix.
//!
//! The bug: When catching a transformed wild Pokémon, ItemUseBall assumed it
//! was a Ditto and set wEnemyMonSpecies2 to DITTO. A non-Ditto wild Pokémon
//! could use Transform via Mirror Move, and catching it would yield a Ditto
//! instead of the actual species.
//!
//! The fix: Remove the `ld a, DITTO / ld [wEnemyMonSpecies2], a` and change
//! `jr z, .notTransformed` to `jr nz, .skip6`. When TRANSFORMED is set,
//! wEnemyMonSpecies2 already holds the original species (Transform only
//! overwrites wEnemyMonSpecies, not wEnemyMonSpecies2). -7 bytes in bank $03.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// TRANSFORMED is bit 3 of wEnemyBattleStatus3.
const TRANSFORMED_BIT: u8 = 3;

/// DITTO species constant ($4C = 76).
const DITTO: u8 = 0x4C;
/// MEW species constant ($15 = 21) — used as test species.
const MEW: u8 = 0x15;

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_no_ditto_assignment() {
    // Verify that the code does NOT contain `ld a, DITTO` ($3E $4C) followed by
    // `ld [wEnemyMonSpecies2], a` ($EA $D7 $CF) between the transform check
    // and .notTransformed.
    let not_transformed = sym_addr("ItemUseBall.notTransformed");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("ItemUseBall"));

    // The 7 bytes before .notTransformed should be:
    //   ld hl, wEnemyBattleStatus3 ($21, lo, hi) = 3 bytes
    //   bit TRANSFORMED, [hl] ($CB $5E) = 2 bytes
    //   jr nz, .skip6 ($20, offset) = 2 bytes
    // NOT:
    //   $3E $4C (ld a, DITTO)
    let transform_check_start = not_transformed - 7;
    let ld_hl = h.read_mem(transform_check_start);
    let bit_cb = h.read_mem(transform_check_start + 3);
    let bit_op = h.read_mem(transform_check_start + 4);
    let jr_nz = h.read_mem(transform_check_start + 5);

    assert_eq!(
        ld_hl, 0x21,
        "Expected ld hl ($21) at transform check start, got ${ld_hl:02X}"
    );
    assert_eq!(
        bit_cb, 0xCB,
        "Expected CB prefix for bit instruction, got ${bit_cb:02X}"
    );
    assert_eq!(bit_op, 0x5E, "Expected bit 3,[hl] ($5E), got ${bit_op:02X}");
    assert_eq!(
        jr_nz, 0x20,
        "Expected jr nz ($20) — jump to .skip6 when transformed, got ${jr_nz:02X}"
    );
}

#[test]
fn rom_bytes_jr_nz_targets_skip6() {
    // Verify the jr nz offset points to .skip6
    let not_transformed = sym_addr("ItemUseBall.notTransformed");
    let skip6 = sym_addr("ItemUseBall.skip6");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("ItemUseBall"));

    let jr_addr = not_transformed - 2; // address of the jr nz instruction
    let jr_offset = h.read_mem(jr_addr + 1) as i8;
    let target = (jr_addr + 2).wrapping_add(jr_offset as u16);

    assert_eq!(
        target, skip6,
        "jr nz offset should target .skip6 (${skip6:04X}), got ${target:04X}"
    );
}

// ─── Behavioral: transformed Pokémon keeps original species ────────

#[test]
fn transformed_pokemon_keeps_original_species() {
    // Set TRANSFORMED bit, set wEnemyMonSpecies2 to MEW, run transform check.
    // wEnemyMonSpecies2 should still be MEW (not DITTO).
    let not_transformed = sym_addr("ItemUseBall.notTransformed");
    let skip6 = sym_addr("ItemUseBall.skip6");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("ItemUseBall"));

    // Set up WRAM state
    h.write_mem(sym_addr("wEnemyBattleStatus3"), 1 << TRANSFORMED_BIT);
    h.write_mem(sym_addr("wEnemyMonSpecies2"), MEW);

    // Start at the ld hl, wEnemyBattleStatus3 instruction
    let transform_check_start = not_transformed - 7;
    h.set_pc(transform_check_start);
    h.set_sp(0xDFF0);

    // Step to .skip6
    h.step_to(skip6);

    // wEnemyMonSpecies2 should still be MEW
    let species = h.read_mem(sym_addr("wEnemyMonSpecies2"));
    assert_eq!(
        species, MEW,
        "wEnemyMonSpecies2 should remain MEW (${MEW:02X}), got ${species:02X} (DITTO=${DITTO:02X})"
    );
}

#[test]
fn transformed_pokemon_species2_not_overwritten_to_ditto() {
    // Specifically verify the old bug doesn't recur: wEnemyMonSpecies2 != DITTO
    let not_transformed = sym_addr("ItemUseBall.notTransformed");
    let skip6 = sym_addr("ItemUseBall.skip6");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("ItemUseBall"));

    // Use a different test species — Gengar ($0E)
    let gengar: u8 = 0x0E;
    h.write_mem(sym_addr("wEnemyBattleStatus3"), 1 << TRANSFORMED_BIT);
    h.write_mem(sym_addr("wEnemyMonSpecies2"), gengar);

    let transform_check_start = not_transformed - 7;
    h.set_pc(transform_check_start);
    h.set_sp(0xDFF0);

    h.step_to(skip6);

    let species = h.read_mem(sym_addr("wEnemyMonSpecies2"));
    assert_ne!(
        species, DITTO,
        "wEnemyMonSpecies2 must NOT be overwritten to DITTO"
    );
    assert_eq!(
        species, gengar,
        "wEnemyMonSpecies2 should remain Gengar (${gengar:02X}), got ${species:02X}"
    );
}

// ─── Not-transformed path still works ──────────────────────────────

#[test]
fn not_transformed_falls_through_to_save_dvs() {
    // When TRANSFORMED is NOT set, code should fall through to .notTransformed
    // which sets the TRANSFORMED bit and copies DVs.
    let not_transformed = sym_addr("ItemUseBall.notTransformed");
    let skip6 = sym_addr("ItemUseBall.skip6");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("ItemUseBall"));

    let w_enemy_mon_dvs = sym_addr("wEnemyMonDVs");
    let w_transformed_dvs = sym_addr("wTransformedEnemyMonOriginalDVs");

    // TRANSFORMED bit clear
    h.write_mem(sym_addr("wEnemyBattleStatus3"), 0x00);
    // Set known DVs
    h.write_mem(w_enemy_mon_dvs, 0xAB);
    h.write_mem(w_enemy_mon_dvs + 1, 0xCD);
    // Clear destination
    h.write_mem(w_transformed_dvs, 0x00);
    h.write_mem(w_transformed_dvs + 1, 0x00);

    let transform_check_start = not_transformed - 7;
    h.set_pc(transform_check_start);
    h.set_sp(0xDFF0);

    // Step to .skip6 (falls through .notTransformed, then reaches .skip6)
    h.step_to(skip6);

    // TRANSFORMED bit should now be set
    let status = h.read_mem(sym_addr("wEnemyBattleStatus3"));
    assert_ne!(
        status & (1 << TRANSFORMED_BIT),
        0,
        "TRANSFORMED bit should be set after .notTransformed path"
    );

    // DVs should be saved
    let dv1 = h.read_mem(w_transformed_dvs);
    let dv2 = h.read_mem(w_transformed_dvs + 1);
    assert_eq!(
        dv1, 0xAB,
        "DV byte 1 should be saved (expected $AB, got ${dv1:02X})"
    );
    assert_eq!(
        dv2, 0xCD,
        "DV byte 2 should be saved (expected $CD, got ${dv2:02X})"
    );
}
