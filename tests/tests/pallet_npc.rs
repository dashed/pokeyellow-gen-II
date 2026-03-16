//! ROM byte tests for the Pallet Town NPC misplacement fix.
//!
//! Bug: The FISHER NPC at (11, 14) has WALK/ANY_DIR movement, allowing
//! it to reach Oak's Lab door warp at (12, 11). Since the OVERWORLD
//! tileset marks door tiles ($1B, $58) as passable (needed for player
//! warp entry), NPCs can walk onto them.
//!
//! Fix: Change the FISHER's movement direction from ANY_DIR ($00) to
//! UP_DOWN ($01). This restricts vertical-only movement, preventing
//! the NPC from reaching the door tile (which requires eastward movement).
//! Zero ROM size change (data-only fix, $00 → $01).
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

// ─── Structural test ─────────────────────────────────────────────────

#[test]
fn pallet_town_object_in_bank_06() {
    assert_eq!(
        sym_bank("PalletTown_Object"),
        0x06,
        "PalletTown_Object should be in bank $06"
    );
}

// ─── THE FIX: fisher uses UP_DOWN not ANY_DIR ────────────────────────

#[test]
fn fisher_movement_is_up_down() {
    let mut h = banked_harness("PalletTown_Object");
    let base = sym_addr("PalletTown_Object");

    // The object data has: border(1) + warps + bg_events + object_events
    // We need to find the 3rd object event (FISHER) and check its movement direction.
    // object_event format: y+4, x+4, sprite, movement_status, movement_dir, text_id
    // (coordinates stored as map_coord + 4)

    // Fisher is at map (11, 14), stored as (11+4, 14+4) = (15, 18) = ($0F, $12)
    // Movement bytes: WALK=$FE, direction byte follows
    // Scan for the pattern: $12 $0F (y+4=18, x+4=15)... actually the format is
    // pic_id, y, x, movement, range_or_dir, text_low, text_high

    // Let me just scan for SPRITE_FISHER followed by WALK ($FE) and check the next byte
    // SPRITE_FISHER = ? Let me scan for the pattern near the end of object data

    // Object events start after bg_events. The 3rd object should be the fisher.
    // Just scan for $FE (WALK) + direction byte pairs in the object section.
    // We expect the FISHER's direction byte to be $01 (UP_DOWN), not $00 (ANY_DIR).

    // Scan the data area for consecutive $FE bytes followed by direction
    let mut walk_directions = Vec::new();
    for addr in base..base + 60 {
        if rom(&mut h, addr) == 0xFE {
            walk_directions.push((addr, rom(&mut h, addr + 1)));
        }
    }

    // There should be 2 WALK NPCs (GIRL and FISHER) — Oak is STAY ($FF)
    assert!(
        walk_directions.len() >= 2,
        "Expected at least 2 WALK ($FE) NPCs, found {}",
        walk_directions.len()
    );

    // The last WALK NPC should be the FISHER with UP_DOWN ($01)
    let (fisher_addr, fisher_dir) = walk_directions[walk_directions.len() - 1];
    assert_eq!(
        fisher_dir,
        0x01,
        "Fisher direction at ${:04X} should be UP_DOWN ($01), got ${:02X} (ANY_DIR=$00)",
        fisher_addr + 1,
        fisher_dir
    );
}

// ─── Door tile passability cross-reference ───────────────────────────

#[test]
fn overworld_door_tile_is_passable() {
    // Verify that door tile $1B IS in the Overworld passable tile list
    // This confirms NPCs CAN walk on it — hence why movement restriction is needed
    let mut h = banked_harness("Overworld_Coll");
    let base = sym_addr("Overworld_Coll");

    let mut found_1b = false;
    for addr in base..base + 25 {
        let tile = rom(&mut h, addr);
        if tile == 0xFF {
            break;
        }
        if tile == 0x1B {
            found_1b = true;
            break;
        }
    }
    assert!(
        found_1b,
        "Door tile $1B should be in Overworld_Coll (passable) — \
         this is why NPCs need movement restriction"
    );
}
