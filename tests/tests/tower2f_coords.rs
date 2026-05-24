//! ROM byte tests for the Pokémon Tower 2F rival encounter coordinate
//! array terminator fix.
//!
//! Bug: `PokemonTower2FRivalEncounterEventCoords` ends with `db $0F`
//! instead of `db -1` ($FF). `ArePlayerCoordsInArray` scans pairs of
//! (Y, X) bytes until it reads $FF, so with $0F the scan reads past the
//! array into subsequent code bytes, potentially matching garbage
//! coordinates.
//!
//! Fix: Change `db $0F` to `db -1`. One-byte change, zero ROM growth.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PokemonTower2FRivalEncounterEventCoords"));
    h
}

// dbmapcoord x, y emits: db y, x
// dbmapcoord 15, 5 → db 5, 15 → $05 $0F
// dbmapcoord 14, 6 → db 6, 14 → $06 $0E
const COORD1_Y: u8 = 5;
const COORD1_X: u8 = 15;
const COORD2_Y: u8 = 6;
const COORD2_X: u8 = 14;
const TERMINATOR: u8 = 0xFF; // db -1

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn coords_in_bank_18() {
    assert_eq!(sym_bank("PokemonTower2FRivalEncounterEventCoords"), 0x18);
}

#[test]
fn coords_in_banked_range() {
    let addr = sym_addr("PokemonTower2FRivalEncounterEventCoords");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── Coordinate data tests ──────────────────────────────────────────

#[test]
fn first_coord_pair_correct() {
    // dbmapcoord 15, 5 → db $05 $0F
    let mut h = banked_harness();
    let base = sym_addr("PokemonTower2FRivalEncounterEventCoords");
    assert_eq!(rom(&mut h, base), COORD1_Y, "first coord Y should be 5");
    assert_eq!(
        rom(&mut h, base + 1),
        COORD1_X,
        "first coord X should be 15"
    );
}

#[test]
fn second_coord_pair_correct() {
    // dbmapcoord 14, 6 → db $06 $0E
    let mut h = banked_harness();
    let base = sym_addr("PokemonTower2FRivalEncounterEventCoords");
    assert_eq!(
        rom(&mut h, base + 2),
        COORD2_Y,
        "second coord Y should be 6"
    );
    assert_eq!(
        rom(&mut h, base + 3),
        COORD2_X,
        "second coord X should be 14"
    );
}

#[test]
fn terminator_is_ff() {
    // The 5th byte (after 2 coord pairs) should be $FF (db -1).
    let mut h = banked_harness();
    let base = sym_addr("PokemonTower2FRivalEncounterEventCoords");
    assert_eq!(
        rom(&mut h, base + 4),
        TERMINATOR,
        "terminator should be $FF, got {:#04X}",
        rom(&mut h, base + 4)
    );
}

// ─── Array size validation ──────────────────────────────────────────

#[test]
fn array_is_exactly_5_bytes() {
    // 2 coordinate pairs (4 bytes) + 1 terminator = 5 bytes.
    // The next label (PokemonTower2FDefeatedRivalScript) should be at base + 5.
    let base = sym_addr("PokemonTower2FRivalEncounterEventCoords");
    let next = sym_addr("PokemonTower2FDefeatedRivalScript");
    assert_eq!(
        next - base,
        5,
        "expected 5 bytes between coords and next label, got {}",
        next - base
    );
}

#[test]
fn caller_uses_are_player_coords_in_array() {
    // The coords array is used by `call ArePlayerCoordsInArray` ($CD lo hi).
    // Verify this call appears in the function preceding the array.
    let mut h = banked_harness();
    let base = sym_addr("PokemonTower2FRivalEncounterEventCoords");
    let are_coords = sym_addr("ArePlayerCoordsInArray");
    let lo = (are_coords & 0xFF) as u8;
    let hi = (are_coords >> 8) as u8;
    // Search backwards from the coords array (within the containing function)
    let search_start = sym_addr("PokemonTower2FDefaultScript");
    let mut found = false;
    for addr in search_start..base {
        if rom(&mut h, addr) == 0xCD && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "call ArePlayerCoordsInArray not found before coords array"
    );
}

// ─── Regression: no $0F terminator ──────────────────────────────────

#[test]
fn terminator_is_not_0f() {
    // The old buggy terminator was $0F. Verify it's no longer used.
    let mut h = banked_harness();
    let base = sym_addr("PokemonTower2FRivalEncounterEventCoords");
    assert_ne!(
        rom(&mut h, base + 4),
        0x0F,
        "terminator should NOT be old buggy value $0F"
    );
}
