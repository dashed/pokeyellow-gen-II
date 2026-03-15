//! ROM byte tests for the Repel saving oversight fix.
//!
//! Bug: The save system does not include `wRepelRemainingSteps` in the saved
//! data blocks. When the player saves and reloads, any active Repel effect is
//! lost because the step counter resets to zero.
//!
//! Fix: Add `sRepelRemainingSteps` to SRAM (inside the checksummed game data
//! range), save `wRepelRemainingSteps` in `SaveMainData`, and restore it in
//! the load routine — matching the pattern used for `hTileAnimations`.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_saving_oversight>

use pokeyellow_tests::{sym_addr, sym_bank};

fn rom() -> Vec<u8> {
    std::fs::read("../pokeyellow.gbc").expect("ROM not found")
}

fn rom_offset(bank: u32, addr: u16) -> usize {
    (bank * 0x4000 + (addr as u32 - 0x4000)) as usize
}

fn at(rom: &[u8], bank: u32, addr: u16) -> u8 {
    rom[rom_offset(bank, addr)]
}

// ─── SRAM layout tests ──────────────────────────────────────────────

#[test]
fn sram_repel_steps_after_tile_animations() {
    let tile_anim = sym_addr("sTileAnimations");
    let repel = sym_addr("sRepelRemainingSteps");
    assert_eq!(
        repel,
        tile_anim + 1,
        "sRepelRemainingSteps should be immediately after sTileAnimations"
    );
}

#[test]
fn sram_repel_steps_inside_checksum_range() {
    let game_data = sym_addr("sGameData");
    let game_data_end = sym_addr("sGameDataEnd");
    let repel = sym_addr("sRepelRemainingSteps");
    assert!(
        repel >= game_data && repel < game_data_end,
        "sRepelRemainingSteps (${repel:04X}) should be within sGameData (${game_data:04X})..sGameDataEnd (${game_data_end:04X})"
    );
}

// ─── Save routine tests ─────────────────────────────────────────────

#[test]
fn save_main_data_writes_repel_steps() {
    let rom = rom();
    let bank = sym_bank("SaveMainData") as u32;
    let save_start = sym_addr("SaveMainData");

    // Search for `ld a, [wRepelRemainingSteps]` ($FA $DA $D0) in SaveMainData
    let w_repel = sym_addr("wRepelRemainingSteps");
    let lo = (w_repel & 0xFF) as u8;
    let hi = (w_repel >> 8) as u8;

    let mut found_load = false;
    let mut found_store = false;
    // Scan up to 256 bytes from SaveMainData
    for i in 0..256u16 {
        let addr = save_start + i;
        let b = at(&rom, bank, addr);
        // ld a, [wRepelRemainingSteps] = $FA lo hi
        if b == 0xFA && at(&rom, bank, addr + 1) == lo && at(&rom, bank, addr + 2) == hi {
            found_load = true;
            // Next should be ld [sRepelRemainingSteps], a = $EA lo hi
            let next = at(&rom, bank, addr + 3);
            assert_eq!(next, 0xEA, "Expected ld [nn],a ($EA) after loading wRepelRemainingSteps");
            let s_repel = sym_addr("sRepelRemainingSteps");
            let s_lo = at(&rom, bank, addr + 4);
            let s_hi = at(&rom, bank, addr + 5);
            let s_addr = u16::from_le_bytes([s_lo, s_hi]);
            assert_eq!(
                s_addr, s_repel,
                "Should store to sRepelRemainingSteps"
            );
            found_store = true;
            break;
        }
    }
    assert!(found_load, "SaveMainData should load wRepelRemainingSteps");
    assert!(found_store, "SaveMainData should store to sRepelRemainingSteps");
}

// ─── Load routine tests ─────────────────────────────────────────────

#[test]
fn load_routine_reads_repel_steps() {
    let rom = rom();
    // The load routine is in the same bank as save
    let bank = sym_bank("SaveMainData") as u32;

    // Search the entire bank for `ld a, [sRepelRemainingSteps]` followed by
    // `ld [wRepelRemainingSteps], a`
    let s_repel = sym_addr("sRepelRemainingSteps");
    let w_repel = sym_addr("wRepelRemainingSteps");
    let s_lo = (s_repel & 0xFF) as u8;
    let s_hi = (s_repel >> 8) as u8;

    let mut found = false;
    // Scan the bank's code area (load routine is in the upper half)
    for i in 0x4000..0x8000u16 {
        let b = at(&rom, bank, i);
        if b == 0xFA && at(&rom, bank, i + 1) == s_lo && at(&rom, bank, i + 2) == s_hi {
            // Found ld a, [sRepelRemainingSteps]
            let next = at(&rom, bank, i + 3);
            if next == 0xEA {
                let w_lo = at(&rom, bank, i + 4);
                let w_hi = at(&rom, bank, i + 5);
                let w_addr = u16::from_le_bytes([w_lo, w_hi]);
                if w_addr == w_repel {
                    found = true;
                    break;
                }
            }
        }
    }
    assert!(
        found,
        "Load routine should read sRepelRemainingSteps and write to wRepelRemainingSteps"
    );
}

// ─── Structural: save bank ──────────────────────────────────────────

#[test]
fn save_main_data_in_bank_1c() {
    assert_eq!(
        sym_bank("SaveMainData"),
        0x1C,
        "SaveMainData should be in bank $1C"
    );
}
