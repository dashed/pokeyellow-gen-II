//! ROM byte tests for the Save Surf exploit fix.
//!
//! Bug: `wPlayerMovingDirection` ($D527) is inside the saved main data block
//! ($D2F6-$DA7F). When saved while holding a D-Pad direction, the stale
//! direction persists through save/load. On reload, `UpdatePlayerSprite` in
//! `EnterMap` reads this stale value and sets `wSpritePlayerStateData1FacingDirection`
//! to match — making `GetTileAndCoordsInFrontOfPlayer` and `IsNextTileShoreOrWater`
//! check the wrong tile. This allows surfing onto non-water tiles.
//!
//! Fix: Zero `wPlayerMovingDirection` after loading the main data block in
//! `LoadMainData`, preventing the stale direction from propagating.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Save_Surf_exploit>

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

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn load_main_data_in_bank_1c() {
    assert_eq!(
        sym_bank("LoadMainData"),
        0x1C,
        "LoadMainData should be in bank $1C"
    );
}

#[test]
fn player_moving_direction_inside_main_data_block() {
    let dir = sym_addr("wPlayerMovingDirection");
    let start = sym_addr("wMainDataStart");
    let end = sym_addr("wMainDataEnd");
    assert!(
        dir >= start && dir < end,
        "wPlayerMovingDirection (${dir:04X}) should be inside \
         wMainDataStart (${start:04X})..wMainDataEnd (${end:04X})"
    );
}

// ─── THE FIX: xor a + ld [wPlayerMovingDirection], a ─────────────────

#[test]
fn player_moving_direction_zeroed_after_main_data_load() {
    let rom = rom();
    let bank = sym_bank("LoadMainData") as u32;

    // Search for xor a ($AF) + ld [wPlayerMovingDirection], a ($EA $27 $D5)
    let dir_addr = sym_addr("wPlayerMovingDirection");
    let lo = (dir_addr & 0xFF) as u8;
    let hi = (dir_addr >> 8) as u8;

    let mut found = false;
    let check_sum_matched = sym_addr("LoadMainData.checkSumMatched");
    for i in check_sum_matched..0x8000 {
        if at(&rom, bank, i) == 0xAF
            && at(&rom, bank, i + 1) == 0xEA
            && at(&rom, bank, i + 2) == lo
            && at(&rom, bank, i + 3) == hi
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Load routine should contain xor a / ld [wPlayerMovingDirection], a \
         after .checkSumMatched to prevent Save Surf exploit"
    );
}

#[test]
fn sprite_facing_direction_inside_sprite_data_block() {
    let facing = sym_addr("wSpritePlayerStateData1FacingDirection");
    let start = sym_addr("wSpriteDataStart");
    let end = sym_addr("wSpriteDataEnd");
    assert!(
        facing >= start && facing < end,
        "wSpritePlayerStateData1FacingDirection (${facing:04X}) should be inside \
         wSpriteDataStart (${start:04X})..wSpriteDataEnd (${end:04X})"
    );
}
