use pokeyellow_tests::{
    load_rom_bytes, measure_cycles_to, rom_path, sym_addr, sym_bank, sym_path, SymbolTable,
    TestHarness,
};

/// Game Boy opcode for `ld c, N` (two-byte instruction: 0x0E followed by N).
const LD_C_OPCODE: u8 = 0x0E;

/// The original 30-frame delay that our patch reduces.
const OLD_DELAY_FRAMES: u8 = 30;

/// The new 10-frame delay used after saving.
const NEW_DELAY_FRAMES: u8 = 10;

fn load_test_data() -> (Vec<u8>, SymbolTable) {
    let rom = load_rom_bytes(rom_path().to_str().unwrap());
    let sym = SymbolTable::load(sym_path().to_str().unwrap());
    (rom, sym)
}

/// Extract the bytes of `SaveMenu` (up to the next label `SaveTheGame_YesOrNo`).
fn save_menu_bytes<'a>(rom: &'a [u8], sym: &SymbolTable) -> &'a [u8] {
    let start = sym
        .rom_offset("SaveMenu")
        .expect("SaveMenu not found in symbol table");
    let end = sym
        .rom_offset("SaveTheGame_YesOrNo")
        .expect("SaveTheGame_YesOrNo not found in symbol table");
    assert!(
        end > start,
        "SaveTheGame_YesOrNo ({end:#x}) should be after SaveMenu ({start:#x})"
    );
    &rom[start..end]
}

#[test]
fn save_delay_reduced_from_30_to_10() {
    let (rom, sym) = load_test_data();
    let code = save_menu_bytes(&rom, &sym);

    // There must be no `ld c, 30` (0x0E 0x1E) in SaveMenu.
    let has_old_delay = code
        .windows(2)
        .any(|w| w[0] == LD_C_OPCODE && w[1] == OLD_DELAY_FRAMES);
    assert!(
        !has_old_delay,
        "SaveMenu should not contain `ld c, {OLD_DELAY_FRAMES}` (old 30-frame delay)"
    );

    // There must be at least one `ld c, 10` (0x0E 0x0A).
    let new_delay_count = code
        .windows(2)
        .filter(|w| w[0] == LD_C_OPCODE && w[1] == NEW_DELAY_FRAMES)
        .count();
    assert!(
        new_delay_count > 0,
        "SaveMenu should contain at least one `ld c, {NEW_DELAY_FRAMES}` (reduced delay)"
    );
}

#[test]
fn save_prompt_removed() {
    let (rom, sym) = load_test_data();
    let code = save_menu_bytes(&rom, &sym);

    // WouldYouLikeToSaveText still exists as a label but must not be
    // referenced from SaveMenu.  A reference would appear as the
    // little-endian address word in a `ld hl, WouldYouLikeToSaveText`
    // instruction (opcode 0x21 followed by addr_lo, addr_hi).
    let (_, addr) = sym
        .resolve("WouldYouLikeToSaveText")
        .expect("WouldYouLikeToSaveText not found in symbol table");
    let addr_lo = (addr & 0xFF) as u8;
    let addr_hi = (addr >> 8) as u8;

    let has_ref = code
        .windows(3)
        .any(|w| w[0] == 0x21 && w[1] == addr_lo && w[2] == addr_hi);
    assert!(
        !has_ref,
        "SaveMenu should not reference WouldYouLikeToSaveText ({addr:#06x}); \
         the save prompt should be removed"
    );
}

// ─── Scenario 13: Save file SRAM round-trip ───────────────────────

/// A safe WRAM return address.
const TRAP_ADDR: u16 = 0xC100;

/// Set up the harness to run SaveGameData headless.
///
/// Disables interrupts, selects the SaveGameData bank, sets PC to SaveGameData,
/// and pushes TRAP_ADDR as the return address.
fn setup_save_fixture(h: &mut TestHarness) {
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00); // IE = 0: mask all interrupts

    h.select_rom_bank(sym_bank("SaveGameData"));

    h.write_mem(TRAP_ADDR, 0x00); // NOP
    h.write_mem(TRAP_ADDR + 1, 0x10); // STOP

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // SaveGameData sets wSaveFileStatus itself, but pre-clear it
    h.write_mem(sym_addr("wSaveFileStatus"), 0x00);

    h.set_pc(sym_addr("SaveGameData"));
}

#[test]
fn save_game_data_writes_sram() {
    let mut h = TestHarness::new_headless();

    // Write distinctive marker bytes to WRAM locations that SaveMainData
    // will copy to SRAM. These are in the main save data region.
    h.write_mem(0xD158, 0x42); // wPlayerName area
    h.write_mem(0xD356, 0xAB); // wObtainedBadges area

    setup_save_fixture(&mut h);
    let state = h.save_state();

    // Run SaveGameData to completion
    h.step_to(TRAP_ADDR);

    // Verify wSaveFileStatus was set to 2 by SaveGameData
    assert_eq!(
        h.read_mem(sym_addr("wSaveFileStatus")),
        2,
        "SaveGameData should set wSaveFileStatus to 2"
    );

    // Capture SRAM after save
    let sram_after_save = h.ram_data();
    assert!(
        !sram_after_save.is_empty(),
        "SRAM should not be empty after SaveGameData"
    );
    assert!(
        sram_after_save.iter().any(|&b| b != 0),
        "SRAM should contain non-zero data after SaveGameData"
    );

    // Run again from the same state — SRAM should be identical (deterministic)
    h.load_state(&state);
    h.write_mem(0xD158, 0x42);
    h.write_mem(0xD356, 0xAB);
    setup_save_fixture(&mut h);
    h.step_to(TRAP_ADDR);

    let sram_second = h.ram_data();
    assert_eq!(
        sram_after_save, sram_second,
        "SaveGameData should produce identical SRAM on repeated calls with same WRAM"
    );
}

#[test]
fn sram_survives_reload() {
    let mut h = TestHarness::new_headless();

    // Write marker data and run SaveGameData
    h.write_mem(0xD158, 0x42);
    setup_save_fixture(&mut h);
    h.step_to(TRAP_ADDR);

    let sram_after_save = h.ram_data();
    assert!(
        sram_after_save.iter().any(|&b| b != 0),
        "SRAM should have data after save"
    );

    // Reload ROM with the saved SRAM
    let rom_data = load_rom_bytes(rom_path().to_str().unwrap());
    h.gb.load_rom(&rom_data, Some(&sram_after_save))
        .expect("Failed to reload ROM with SRAM");

    // Read SRAM back and verify it matches
    let sram_after_reload = h.ram_data();
    assert_eq!(
        sram_after_save, sram_after_reload,
        "SRAM should survive a save → reload round-trip"
    );
}

// ─── Scenario 15: Save routine timing benchmark ──────────────────

#[test]
fn save_game_data_completes_within_budget() {
    let mut h = TestHarness::new_headless();
    setup_save_fixture(&mut h);

    // SaveGameData performs 3 block copies (main data, current box, party/dex)
    // with checksums. No VBlank waits — pure memory operations.
    // Should complete well under 1M cycles.
    let max_cycles = 1_000_000;
    let (cycles, reached) = measure_cycles_to(&mut h, TRAP_ADDR, max_cycles);

    assert!(
        reached,
        "SaveGameData should complete within {max_cycles} cycles (stopped at PC=${:04X})",
        h.pc()
    );

    // Sanity: should take a non-trivial number of cycles (it copies kilobytes)
    assert!(
        cycles > 1000,
        "SaveGameData should take more than 1000 cycles (got {cycles}), \
         something may be wrong if it returns too quickly"
    );

    eprintln!("SaveGameData completed in {cycles} cycles");
}
