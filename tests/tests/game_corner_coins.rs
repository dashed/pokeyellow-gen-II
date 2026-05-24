//! ROM byte tests for the Game Corner 10-coin tile oversight fix.
//!
//! Bug: The Rocket Game Corner has hidden coins at coordinates (12, 15), but
//! that tile contains a slot machine used by an NPC (Gambler at 11, 15).
//! The player cannot stand on or interact with the tile, making the 10 coins
//! permanently inaccessible.
//!
//! Fix: Remove the hidden coin entry at (12, 15) from both `HiddenCoinCoords`
//! and the Game Corner's hidden events list.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I>
//! Reference: <https://glitchcity.wiki/wiki/Game_Corner_10_coins_tile_oversight>

use pokeyellow_tests::{sym_addr, sym_bank};

// ─── Helpers ─────────────────────────────────────────────────────────

fn rom() -> Vec<u8> {
    std::fs::read("../pokeyellow.gbc").expect("ROM not found")
}

fn rom_offset(bank: u32, addr: u16) -> usize {
    if bank == 0 {
        addr as usize
    } else {
        (bank * 0x4000 + (addr as u32 - 0x4000)) as usize
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn hidden_coin_coords_in_bank_1d() {
    assert_eq!(sym_bank("HiddenCoinCoords"), 0x1D);
}

#[test]
fn hidden_coin_coords_has_11_entries() {
    // After removing (12, 15), there should be 11 entries (was 12)
    let rom = rom();
    let bank = sym_bank("HiddenCoinCoords") as u32;
    let addr = sym_addr("HiddenCoinCoords");
    let off = rom_offset(bank, addr);

    let mut count = 0;
    while rom[off + count * 3] != 0xFF {
        count += 1;
        assert!(count <= 20, "Too many entries — missing terminator?");
    }
    assert_eq!(
        count, 11,
        "Expected 11 hidden coin entries (12 minus removed (12,15))"
    );
}

#[test]
fn no_hidden_coin_at_12_15() {
    // Verify no entry has x=12, y=15
    let rom = rom();
    let bank = sym_bank("HiddenCoinCoords") as u32;
    let addr = sym_addr("HiddenCoinCoords");
    let off = rom_offset(bank, addr);

    let mut i = 0;
    while rom[off + i * 3] != 0xFF {
        let y = rom[off + i * 3 + 1];
        let x = rom[off + i * 3 + 2];
        assert!(
            !(x == 12 && y == 15),
            "Found removed hidden coin entry at (12, 15) — index {i}"
        );
        i += 1;
    }
}

#[test]
fn other_hidden_coins_preserved() {
    // Spot-check some known entries still exist
    let rom = rom();
    let bank = sym_bank("HiddenCoinCoords") as u32;
    let addr = sym_addr("HiddenCoinCoords");
    let off = rom_offset(bank, addr);

    let expected = [(0u8, 8u8), (11, 7), (15, 8), (9, 12), (10, 16)];
    for (ex, ey) in &expected {
        let mut found = false;
        let mut i = 0;
        while rom[off + i * 3] != 0xFF {
            let y = rom[off + i * 3 + 1];
            let x = rom[off + i * 3 + 2];
            if x == *ex && y == *ey {
                found = true;
                break;
            }
            i += 1;
        }
        assert!(found, "Expected hidden coin at ({ex}, {ey}) not found");
    }
}

#[test]
fn hidden_coin_coords_terminated() {
    let rom = rom();
    let bank = sym_bank("HiddenCoinCoords") as u32;
    let addr = sym_addr("HiddenCoinCoords");
    let off = rom_offset(bank, addr);

    // Walk to end
    let mut i = 0;
    while rom[off + i * 3] != 0xFF {
        i += 1;
    }
    assert_eq!(
        rom[off + i * 3],
        0xFF,
        "Table should end with $FF terminator"
    );
}
