//! Emulator-based test for the Mew wild encounter in Cerulean Cave B1F.
//!
//! Verifies the encounter loading pipeline works at runtime: calling
//! `LoadWildData` with the correct map ID populates `wGrassMons` with
//! encounter data that includes Mew.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// CERULEAN_CAVE_B1F map ID = $E3.
const CERULEAN_CAVE_B1F: u8 = 0xE3;
/// MEW species constant = $15.
const MEW: u8 = 0x15;

/// TRAP_ADDR for return.
const TRAP_ADDR: u16 = 0xC100;

// ─── Scenario 18: Mew encounter via LoadWildData ─────────────────

#[test]
fn load_wild_data_includes_mew_in_cerulean_cave_b1f() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    // Set current map to Cerulean Cave B1F
    h.write_mem(sym_addr("wCurMap"), CERULEAN_CAVE_B1F);

    // Select ROM bank containing LoadWildData
    h.select_rom_bank(sym_bank("LoadWildData"));

    // Set up stack with return address
    h.write_mem(TRAP_ADDR, 0x00); // NOP
    h.write_mem(TRAP_ADDR + 1, 0x10); // STOP
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Call LoadWildData
    h.set_pc(sym_addr("LoadWildData"));
    h.step_to(TRAP_ADDR);

    // Verify encounter rate is non-zero (map has encounters)
    let grass_rate = h.read_mem(sym_addr("wGrassRate"));
    assert!(
        grass_rate > 0,
        "Cerulean Cave B1F should have a non-zero encounter rate (got {})",
        grass_rate
    );

    // Search all 10 encounter slots for Mew
    let mut found_mew = false;
    let mut mew_level = 0u8;
    for slot in 0..10u16 {
        let level = h.read_mem(sym_addr("wGrassMons") + slot * 2);
        let species = h.read_mem(sym_addr("wGrassMons") + slot * 2 + 1);
        if species == MEW {
            found_mew = true;
            mew_level = level;
        }
    }

    assert!(
        found_mew,
        "Mew (${:02X}) should appear in Cerulean Cave B1F encounter table at runtime",
        MEW
    );
    assert!(
        mew_level > 0,
        "Mew's encounter level should be non-zero (got {})",
        mew_level
    );
}

#[test]
fn load_wild_data_encounter_table_has_10_valid_species() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    h.write_mem(sym_addr("wCurMap"), CERULEAN_CAVE_B1F);
    h.select_rom_bank(sym_bank("LoadWildData"));
    h.write_mem(TRAP_ADDR, 0x00);
    h.write_mem(TRAP_ADDR + 1, 0x10);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("LoadWildData"));
    h.step_to(TRAP_ADDR);

    // Every slot should have a valid species (non-zero) and level
    for slot in 0..10u16 {
        let level = h.read_mem(sym_addr("wGrassMons") + slot * 2);
        let species = h.read_mem(sym_addr("wGrassMons") + slot * 2 + 1);
        assert!(
            species > 0,
            "Slot {} species should be non-zero (got ${:02X})",
            slot,
            species
        );
        assert!(
            level > 0,
            "Slot {} level should be non-zero (got {})",
            slot,
            level
        );
    }
}
