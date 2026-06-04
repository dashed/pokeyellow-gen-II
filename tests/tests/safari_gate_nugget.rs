//! ROM byte tests verifying the inaccessible Safari Zone Gate Nugget is removed.
//!
//! Bug: A hidden Nugget at coordinates (10, 1) in the Safari Zone Gate was
//! placed in the black void outside the playable area. The Itemfinder detected
//! it from nearby tiles but the player could never reach it.
//!
//! Fix: Remove the hidden item entry from both `hidden_item_coords.asm` and
//! `hidden_events.asm`. Data-only fix, saves 3 bytes (coord entry) + 4 bytes
//! (event entry).
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// ─── Verify the hidden item coordinate is removed ────────────────────

#[test]
fn no_safari_zone_gate_in_hidden_item_coords() {
    let mut h = banked_harness("HiddenItemCoords");
    let base = sym_addr("HiddenItemCoords");

    // SAFARI_ZONE_GATE map ID = $9C (from map_constants.asm)
    let safari_gate_id: u8 = 0x9C;

    // Scan through the coordinate table (3-byte entries: map, y, x)
    // terminated by $FF
    let mut addr = base;
    loop {
        let map_id = rom(&mut h, addr);
        if map_id == 0xFF {
            break;
        }
        assert_ne!(
            map_id, safari_gate_id,
            "SAFARI_ZONE_GATE (${:02X}) should NOT be in HiddenItemCoords at ${:04X} — \
             inaccessible Nugget was supposed to be removed",
            safari_gate_id, addr
        );
        addr += 3; // next entry
    }
}
