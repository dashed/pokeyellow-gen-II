//! ROM byte tests for the missing dungeon maps fix.
//!
//! Bug: `GetBattleTransitionID_IsDungeonMap` determines which battle
//! transition animation plays (dungeon-style vs outdoor-style). The
//! dungeon map lists in `data/maps/dungeon_maps.asm` were missing
//! several obvious dungeon maps: Victory Road 2F/3F, all Rocket
//! Hideout floors, Pokémon Mansion 1F, Seafoam Islands B1F-B4F,
//! Power Plant, Diglett's Cave, and Silph Co. 9F-11F.
//!
//! Fix: Add the missing maps to DungeonMaps1 (exact match) and
//! DungeonMaps2 (range check) lists.
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("DungeonMaps1"));
    h
}

/// Read all bytes from DungeonMaps1 until $FF terminator.
fn read_dungeon_maps1(h: &mut TestHarness) -> Vec<u8> {
    let mut result = Vec::new();
    let mut addr = sym_addr("DungeonMaps1");
    loop {
        let b = rom(h, addr);
        if b == 0xFF {
            break;
        }
        result.push(b);
        addr += 1;
    }
    result
}

/// Read all range pairs from DungeonMaps2 until $FF terminator.
fn read_dungeon_maps2(h: &mut TestHarness) -> Vec<(u8, u8)> {
    let mut result = Vec::new();
    let mut addr = sym_addr("DungeonMaps2");
    loop {
        let lo = rom(h, addr);
        if lo == 0xFF {
            break;
        }
        let hi = rom(h, addr + 1);
        result.push((lo, hi));
        addr += 2;
    }
    result
}

/// Check if a map ID is recognized as a dungeon map by the lists.
fn is_dungeon_map(maps1: &[u8], maps2: &[(u8, u8)], map_id: u8) -> bool {
    if maps1.contains(&map_id) {
        return true;
    }
    for &(lo, hi) in maps2 {
        if map_id >= lo && map_id <= hi {
            return true;
        }
    }
    false
}

// ─── Map ID constants (from constants/map_constants.asm) ─────────────

// Original DungeonMaps1 entries
const VIRIDIAN_FOREST: u8 = 0x33;
const ROCK_TUNNEL_1F: u8 = 0x52;
const SEAFOAM_ISLANDS_1F: u8 = 0xC0;
const ROCK_TUNNEL_B1F: u8 = 0xE8;

// Newly added DungeonMaps1 entries
const POKEMON_MANSION_1F: u8 = 0xA5;
const VICTORY_ROAD_2F: u8 = 0xC2;
const VICTORY_ROAD_3F: u8 = 0xC6;
const POWER_PLANT: u8 = 0x53;
const DIGLETTS_CAVE: u8 = 0xC5;

// Newly added DungeonMaps2 range endpoints
const SILPH_CO_9F: u8 = 0xE9;
const SILPH_CO_11F: u8 = 0xEB;
const SEAFOAM_ISLANDS_B1F: u8 = 0x9F;
const SEAFOAM_ISLANDS_B4F: u8 = 0xA2;
const ROCKET_HIDEOUT_B1F: u8 = 0xC7;
const ROCKET_HIDEOUT_B4F: u8 = 0xCA;

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn dungeon_maps_in_bank_1c() {
    assert_eq!(sym_bank("DungeonMaps1"), 0x1C);
    assert_eq!(sym_bank("DungeonMaps2"), 0x1C);
}

#[test]
fn dungeon_maps1_has_9_entries() {
    let mut h = rom_harness();
    let maps = read_dungeon_maps1(&mut h);
    assert_eq!(
        maps.len(),
        9,
        "DungeonMaps1 should have 9 entries (4 original + 5 new)"
    );
}

#[test]
fn dungeon_maps2_has_7_ranges() {
    let mut h = rom_harness();
    let ranges = read_dungeon_maps2(&mut h);
    assert_eq!(
        ranges.len(),
        7,
        "DungeonMaps2 should have 7 ranges (4 original + 3 new)"
    );
}

// ─── Original entries still present ──────────────────────────────────

#[test]
fn original_maps1_entries_present() {
    let mut h = rom_harness();
    let maps = read_dungeon_maps1(&mut h);
    assert!(maps.contains(&VIRIDIAN_FOREST), "VIRIDIAN_FOREST missing");
    assert!(maps.contains(&ROCK_TUNNEL_1F), "ROCK_TUNNEL_1F missing");
    assert!(
        maps.contains(&SEAFOAM_ISLANDS_1F),
        "SEAFOAM_ISLANDS_1F missing"
    );
    assert!(maps.contains(&ROCK_TUNNEL_B1F), "ROCK_TUNNEL_B1F missing");
}

// ─── New DungeonMaps1 entries ────────────────────────────────────────

#[test]
fn pokemon_mansion_1f_in_maps1() {
    let mut h = rom_harness();
    let maps = read_dungeon_maps1(&mut h);
    assert!(
        maps.contains(&POKEMON_MANSION_1F),
        "POKEMON_MANSION_1F ($A5) should be in DungeonMaps1"
    );
}

#[test]
fn victory_road_2f_in_maps1() {
    let mut h = rom_harness();
    let maps = read_dungeon_maps1(&mut h);
    assert!(
        maps.contains(&VICTORY_ROAD_2F),
        "VICTORY_ROAD_2F ($C2) should be in DungeonMaps1"
    );
}

#[test]
fn victory_road_3f_in_maps1() {
    let mut h = rom_harness();
    let maps = read_dungeon_maps1(&mut h);
    assert!(
        maps.contains(&VICTORY_ROAD_3F),
        "VICTORY_ROAD_3F ($C6) should be in DungeonMaps1"
    );
}

#[test]
fn power_plant_in_maps1() {
    let mut h = rom_harness();
    let maps = read_dungeon_maps1(&mut h);
    assert!(
        maps.contains(&POWER_PLANT),
        "POWER_PLANT ($53) should be in DungeonMaps1"
    );
}

#[test]
fn digletts_cave_in_maps1() {
    let mut h = rom_harness();
    let maps = read_dungeon_maps1(&mut h);
    assert!(
        maps.contains(&DIGLETTS_CAVE),
        "DIGLETTS_CAVE ($C5) should be in DungeonMaps1"
    );
}

// ─── New DungeonMaps2 ranges ─────────────────────────────────────────

#[test]
fn silph_co_9f_11f_range_in_maps2() {
    let mut h = rom_harness();
    let ranges = read_dungeon_maps2(&mut h);
    assert!(
        ranges.contains(&(SILPH_CO_9F, SILPH_CO_11F)),
        "SILPH_CO_9F-11F range should be in DungeonMaps2"
    );
}

#[test]
fn seafoam_islands_b1f_b4f_range_in_maps2() {
    let mut h = rom_harness();
    let ranges = read_dungeon_maps2(&mut h);
    assert!(
        ranges.contains(&(SEAFOAM_ISLANDS_B1F, SEAFOAM_ISLANDS_B4F)),
        "SEAFOAM_ISLANDS_B1F-B4F range should be in DungeonMaps2"
    );
}

#[test]
fn rocket_hideout_b1f_b4f_range_in_maps2() {
    let mut h = rom_harness();
    let ranges = read_dungeon_maps2(&mut h);
    assert!(
        ranges.contains(&(ROCKET_HIDEOUT_B1F, ROCKET_HIDEOUT_B4F)),
        "ROCKET_HIDEOUT_B1F-B4F range should be in DungeonMaps2"
    );
}

// ─── Integration: all previously-missing maps now recognized ─────────

#[test]
fn all_previously_missing_maps_recognized() {
    let mut h = rom_harness();
    let maps1 = read_dungeon_maps1(&mut h);
    let maps2 = read_dungeon_maps2(&mut h);

    let missing_maps = [
        (VICTORY_ROAD_2F, "VICTORY_ROAD_2F"),
        (VICTORY_ROAD_3F, "VICTORY_ROAD_3F"),
        (ROCKET_HIDEOUT_B1F, "ROCKET_HIDEOUT_B1F"),
        (0xC8, "ROCKET_HIDEOUT_B2F"),
        (0xC9, "ROCKET_HIDEOUT_B3F"),
        (ROCKET_HIDEOUT_B4F, "ROCKET_HIDEOUT_B4F"),
        (POKEMON_MANSION_1F, "POKEMON_MANSION_1F"),
        (SEAFOAM_ISLANDS_B1F, "SEAFOAM_ISLANDS_B1F"),
        (0xA0, "SEAFOAM_ISLANDS_B2F"),
        (0xA1, "SEAFOAM_ISLANDS_B3F"),
        (SEAFOAM_ISLANDS_B4F, "SEAFOAM_ISLANDS_B4F"),
        (POWER_PLANT, "POWER_PLANT"),
        (DIGLETTS_CAVE, "DIGLETTS_CAVE"),
        (SILPH_CO_9F, "SILPH_CO_9F"),
        (0xEA, "SILPH_CO_10F"),
        (SILPH_CO_11F, "SILPH_CO_11F"),
    ];

    for (map_id, name) in &missing_maps {
        assert!(
            is_dungeon_map(&maps1, &maps2, *map_id),
            "{name} (${map_id:02X}) should be recognized as a dungeon map"
        );
    }
}
