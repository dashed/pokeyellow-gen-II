//! ROM byte tests for the invisible tree glitch fix.
//!
//! Bug: After cutting a tree near a map border (e.g. Route 14/15), crossing
//! the map connection rebuilds wOverworldMap from ROM (restoring the tree),
//! but wTileMap and VRAM retain the stale "no tree" tiles. This creates an
//! invisible collision wall that the player cannot re-cut because
//! `_GetTileAndCoordsInFrontOfPlayer` reads from wTileMap which may disagree
//! with what VRAM actually displays.
//!
//! Fix: At `.storeTile`, if wTileMap reports $3D (cut tree tile), verify
//! against VRAM via `ReadTileFromVram`. If VRAM has a different tile (the
//! tree hasn't been visually redrawn yet), use the VRAM value instead.
//! This ensures `wTileInFrontOfPlayer` matches the player's visual state.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Invisible_tree_glitch>
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)#Invisible_tree>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Helper to read a ROM byte at a banked address.
fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Create a TestHarness with the correct ROM bank selected.
fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("_GetTileAndCoordsInFrontOfPlayer"));
    h
}

// ─── Screen coordinate storage tests ─────────────────────────────────

#[test]
fn facing_down_stores_coords_8_11() {
    let mut h = rom_harness();
    // After: ld a,[wYCoord](3) + ld d,a(1) + ld a,[wXCoord](3) + ld e,a(1)
    //      + ld a,[wSpritePlayerStateData1FacingDirection](3) + and a(1) + jr nz(2) = 14 bytes
    let store_start = sym_addr("_GetTileAndCoordsInFrontOfPlayer") + 14;
    // ld a, 8 → $3E $08
    assert_eq!(
        rom(&mut h, store_start),
        0x3E,
        "ld a, imm8 opcode (x coord)"
    );
    assert_eq!(
        rom(&mut h, store_start + 1),
        8,
        "x coord = 8 for facing down"
    );
    // ld [wTempColCoords], a → $EA lo hi (3 bytes)
    assert_eq!(rom(&mut h, store_start + 2), 0xEA, "ld [addr], a opcode");
    // ld a, 11 → $3E $0B
    assert_eq!(
        rom(&mut h, store_start + 5),
        0x3E,
        "ld a, imm8 opcode (y coord)"
    );
    assert_eq!(
        rom(&mut h, store_start + 6),
        11,
        "y coord = 11 for facing down"
    );
}

#[test]
fn facing_up_stores_coords_8_7() {
    let mut h = rom_harness();
    let branch = sym_addr("_GetTileAndCoordsInFrontOfPlayer.notFacingDown");
    // After cp SPRITE_FACING_UP (2 bytes) + jr nz (2 bytes) = offset 4
    let store_start = branch + 4;
    assert_eq!(rom(&mut h, store_start), 0x3E, "ld a, imm8 (x)");
    assert_eq!(rom(&mut h, store_start + 1), 8, "x coord = 8 for facing up");
    assert_eq!(rom(&mut h, store_start + 5), 0x3E, "ld a, imm8 (y)");
    assert_eq!(rom(&mut h, store_start + 6), 7, "y coord = 7 for facing up");
}

#[test]
fn facing_left_stores_coords_6_9() {
    let mut h = rom_harness();
    let branch = sym_addr("_GetTileAndCoordsInFrontOfPlayer.notFacingUp");
    // After cp SPRITE_FACING_LEFT (2 bytes) + jr nz (2 bytes) = offset 4
    let store_start = branch + 4;
    assert_eq!(rom(&mut h, store_start), 0x3E, "ld a, imm8 (x)");
    assert_eq!(
        rom(&mut h, store_start + 1),
        6,
        "x coord = 6 for facing left"
    );
    assert_eq!(rom(&mut h, store_start + 5), 0x3E, "ld a, imm8 (y)");
    assert_eq!(
        rom(&mut h, store_start + 6),
        9,
        "y coord = 9 for facing left"
    );
}

#[test]
fn facing_right_stores_coords_10_9() {
    let mut h = rom_harness();
    let branch = sym_addr("_GetTileAndCoordsInFrontOfPlayer.notFacingLeft");
    // After cp SPRITE_FACING_RIGHT (2 bytes) + jr nz (2 bytes) = offset 4
    let store_start = branch + 4;
    assert_eq!(rom(&mut h, store_start), 0x3E, "ld a, imm8 (x)");
    assert_eq!(
        rom(&mut h, store_start + 1),
        10,
        "x coord = 10 for facing right"
    );
    assert_eq!(rom(&mut h, store_start + 5), 0x3E, "ld a, imm8 (y)");
    assert_eq!(
        rom(&mut h, store_start + 6),
        9,
        "y coord = 9 for facing right"
    );
}

// ─── .storeTile VRAM verification tests ──────────────────────────────

#[test]
fn store_tile_checks_cut_tree_tile() {
    let mut h = rom_harness();
    let store = sym_addr("_GetTileAndCoordsInFrontOfPlayer.storeTile");
    // cp $3D → $FE $3D
    assert_eq!(rom(&mut h, store), 0xFE, "cp imm8 opcode");
    assert_eq!(rom(&mut h, store + 1), 0x3D, "cut tree tile value $3D");
}

#[test]
fn store_tile_calls_read_tile_from_vram_on_match() {
    let mut h = rom_harness();
    let store = sym_addr("_GetTileAndCoordsInFrontOfPlayer.storeTile");
    let read_vram = sym_addr("ReadTileFromVram");
    // call z, ReadTileFromVram → $CC lo hi
    assert_eq!(rom(&mut h, store + 2), 0xCC, "call z opcode");
    assert_eq!(
        rom(&mut h, store + 3),
        (read_vram & 0xFF) as u8,
        "ReadTileFromVram addr lo"
    );
    assert_eq!(
        rom(&mut h, store + 4),
        (read_vram >> 8) as u8,
        "ReadTileFromVram addr hi"
    );
}

#[test]
fn store_tile_still_stores_to_wram() {
    let mut h = rom_harness();
    let store = sym_addr("_GetTileAndCoordsInFrontOfPlayer.storeTile");
    // After cp + call z (5 bytes): ld c, a ($4F) then ld [wTileInFrontOfPlayer], a ($EA lo hi)
    assert_eq!(rom(&mut h, store + 5), 0x4F, "ld c, a");
    assert_eq!(rom(&mut h, store + 6), 0xEA, "ld [wTileInFrontOfPlayer], a");
    // ret
    assert_eq!(rom(&mut h, store + 9), 0xC9, "ret");
}

// ─── ReadTileFromVram function tests ─────────────────────────────────

#[test]
fn read_tile_from_vram_starts_with_push_bc() {
    let mut h = rom_harness();
    let func = sym_addr("ReadTileFromVram");
    assert_eq!(rom(&mut h, func), 0xC5, "push bc");
}

#[test]
fn read_tile_from_vram_reads_scx_and_scy() {
    let mut h = rom_harness();
    let func = sym_addr("ReadTileFromVram");
    // The function reads rSCX ($FF43) and rSCY ($FF42) via ldh a, [addr]
    // Find ldh a, [rSCX] → $F0 $43 somewhere in the function
    let mut found_scx = false;
    let mut found_scy = false;
    for i in 0..70 {
        let byte = rom(&mut h, func + i);
        if byte == 0xF0 {
            let next = rom(&mut h, func + i + 1);
            if next == 0x43 {
                found_scx = true;
            }
            if next == 0x42 {
                found_scy = true;
            }
        }
    }
    assert!(found_scx, "ReadTileFromVram should read rSCX ($FF43)");
    assert!(found_scy, "ReadTileFromVram should read rSCY ($FF42)");
}

#[test]
fn read_tile_from_vram_uses_bg_tilemap_base() {
    let mut h = rom_harness();
    let func = sym_addr("ReadTileFromVram");
    // ld hl, $9800 → $21 $00 $98
    let mut found = false;
    for i in 0..60 {
        if rom(&mut h, func + i) == 0x21
            && rom(&mut h, func + i + 1) == 0x00
            && rom(&mut h, func + i + 2) == 0x98
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ReadTileFromVram should reference BG tilemap at $9800"
    );
}

#[test]
fn read_tile_from_vram_ends_with_ret() {
    let mut h = rom_harness();
    let div8 = sym_addr("ReadTileFromVram.div8");
    // .div8 is srl a × 3 + ret = 7 bytes
    // .sub20 follows: sub $20 + ret = 3 bytes
    // Total function end is at .sub20 + 2
    let sub20 = sym_addr("ReadTileFromVram.sub20");
    assert_eq!(rom(&mut h, sub20 + 2), 0xC9, ".sub20 ends with ret");
    assert_eq!(rom(&mut h, div8 + 6), 0xC9, ".div8 ends with ret");
}
