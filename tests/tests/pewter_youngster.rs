//! ROM byte tests for the Pewter Gym youngster sprite coordinate fix.
//!
//! Bug: In `PewterCityYoungsterShowsPlayerGymScript`, the youngster's
//! X screen coordinate is loaded as `$40` instead of the correct `$50`.
//! This causes a visible sprite tearing/misalignment when the youngster
//! walks away after guiding the player to the Pewter Gym.
//!
//! Fix: Change `ld a, $40` to `ld a, $50` for `hSpriteScreenXCoord`.
//! One-byte change in bank $06.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PewterCityYoungsterShowsPlayerGymScript"));
    h
}

const H_SPRITE_SCREEN_Y_COORD_LO: u8 = 0xEB;
const H_SPRITE_SCREEN_X_COORD_LO: u8 = 0xEC;
const H_SPRITE_MAP_Y_COORD_LO: u8 = 0xED;
const H_SPRITE_MAP_X_COORD_LO: u8 = 0xEE;

/// Find the coordinate loading block by scanning for
/// `ld a, $3C` ($3E $3C) / `ldh [hSpriteScreenYCoord], a` ($E0 $EB)
fn find_coord_block(h: &mut TestHarness) -> u16 {
    let base = sym_addr("PewterCityYoungsterShowsPlayerGymScript");
    for i in 0..60 {
        let addr = base + i;
        if rom(h, addr) == 0x3E
            && rom(h, addr + 1) == 0x3C
            && rom(h, addr + 2) == 0xE0
            && rom(h, addr + 3) == H_SPRITE_SCREEN_Y_COORD_LO
        {
            return addr;
        }
    }
    panic!("coordinate block not found in PewterCityYoungsterShowsPlayerGymScript");
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn script_in_bank_06() {
    assert_eq!(sym_bank("PewterCityYoungsterShowsPlayerGymScript"), 0x06);
}

#[test]
fn script_address_in_banked_range() {
    let addr = sym_addr("PewterCityYoungsterShowsPlayerGymScript");
    assert!(
        (0x4000..0x8000).contains(&addr),
        "expected banked ROM address, got {:#06X}",
        addr
    );
}

// ─── Core fix: X coordinate is $50 ──────────────────────────────────

#[test]
fn y_screen_coord_is_3c() {
    let mut h = banked_harness();
    let block = find_coord_block(&mut h);
    // ld a, $3C at block+0
    assert_eq!(rom(&mut h, block), 0x3E, "ld a, n opcode for Y coord");
    assert_eq!(rom(&mut h, block + 1), 0x3C, "Y screen coord should be $3C");
}

#[test]
fn y_coord_stored_to_h_sprite_screen_y() {
    let mut h = banked_harness();
    let block = find_coord_block(&mut h);
    // ldh [hSpriteScreenYCoord], a at block+2
    assert_eq!(rom(&mut h, block + 2), 0xE0, "ldh opcode for Y coord store");
    assert_eq!(
        rom(&mut h, block + 3),
        H_SPRITE_SCREEN_Y_COORD_LO,
        "should store to hSpriteScreenYCoord ($EB)"
    );
}

#[test]
fn x_screen_coord_is_50_not_40() {
    let mut h = banked_harness();
    let block = find_coord_block(&mut h);
    // ld a, $50 at block+4
    assert_eq!(rom(&mut h, block + 4), 0x3E, "ld a, n opcode for X coord");
    assert_eq!(
        rom(&mut h, block + 5),
        0x50,
        "X screen coord should be $50, not $40 (the original bug)"
    );
}

#[test]
fn x_coord_stored_to_h_sprite_screen_x() {
    let mut h = banked_harness();
    let block = find_coord_block(&mut h);
    // ldh [hSpriteScreenXCoord], a at block+6
    assert_eq!(rom(&mut h, block + 6), 0xE0, "ldh opcode for X coord store");
    assert_eq!(
        rom(&mut h, block + 7),
        H_SPRITE_SCREEN_X_COORD_LO,
        "should store to hSpriteScreenXCoord ($EC)"
    );
}

// ─── Map coordinate integrity ────────────────────────────────────────

#[test]
fn map_y_coord_is_22() {
    let mut h = banked_harness();
    let block = find_coord_block(&mut h);
    // ld a, 22 at block+8, ldh [hSpriteMapYCoord], a at block+10
    assert_eq!(rom(&mut h, block + 8), 0x3E, "ld a, n opcode for map Y");
    assert_eq!(rom(&mut h, block + 9), 22, "map Y coord should be 22");
    assert_eq!(rom(&mut h, block + 10), 0xE0, "ldh opcode for map Y store");
    assert_eq!(
        rom(&mut h, block + 11),
        H_SPRITE_MAP_Y_COORD_LO,
        "should store to hSpriteMapYCoord ($ED)"
    );
}

#[test]
fn map_x_coord_is_16() {
    let mut h = banked_harness();
    let block = find_coord_block(&mut h);
    // ld a, 16 at block+12, ldh [hSpriteMapXCoord], a at block+14
    assert_eq!(rom(&mut h, block + 12), 0x3E, "ld a, n opcode for map X");
    assert_eq!(rom(&mut h, block + 13), 16, "map X coord should be 16");
    assert_eq!(rom(&mut h, block + 14), 0xE0, "ldh opcode for map X store");
    assert_eq!(
        rom(&mut h, block + 15),
        H_SPRITE_MAP_X_COORD_LO,
        "should store to hSpriteMapXCoord ($EE)"
    );
}
