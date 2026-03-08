use pokeyellow_tests::{load_rom_bytes, rom_path, sym_path, SymbolTable};

// Number of encounter slots per table
const NUM_WILDMONS: usize = 10;
// Total bytes for one encounter block: 1 rate byte + 10 * 2 (level, species)
const WILDDATA_LENGTH: usize = 1 + NUM_WILDMONS * 2;

// Internal IDs from constants/pokemon_constants.asm
const WEEDLE: u8 = 0x70;
const KAKUNA: u8 = 0x71;
const EKANS: u8 = 0x6C;
const RAICHU: u8 = 0x55;
const MEOWTH: u8 = 0x4D;
const KOFFING: u8 = 0x37;
const WEEZING: u8 = 0x8F;
const JYNX: u8 = 0x48;
const ELECTABUZZ: u8 = 0x35;
const MAGMAR: u8 = 0x33;
const EEVEE: u8 = 0x66;
const HITMONLEE: u8 = 0x2B;
const HITMONCHAN: u8 = 0x2C;
const OMANYTE: u8 = 0x62;
const KABUTO: u8 = 0x5A;
const MEW: u8 = 0x15;

fn load_test_data() -> (Vec<u8>, SymbolTable) {
    let rom = load_rom_bytes(rom_path().to_str().unwrap());
    let sym = SymbolTable::load(sym_path().to_str().unwrap());
    (rom, sym)
}

/// Search for a species in a grass/water encounter table.
/// `table_offset` points to the encounter rate byte; the 10 (level, species)
/// pairs start at `table_offset + 1`.
fn encounter_table_contains(rom: &[u8], table_offset: usize, species_id: u8) -> bool {
    for i in 0..NUM_WILDMONS {
        let species_offset = table_offset + 1 + i * 2 + 1;
        if rom[species_offset] == species_id {
            return true;
        }
    }
    false
}

/// Assert a species is present in the grass encounter table for the given map label.
fn assert_in_grass(rom: &[u8], sym: &SymbolTable, label: &str, species: u8, name: &str) {
    let offset = sym
        .rom_offset(label)
        .unwrap_or_else(|| panic!("symbol {label} not found"));
    assert!(
        rom[offset] > 0,
        "{label}: grass encounter rate is 0, expected encounters"
    );
    assert!(
        encounter_table_contains(rom, offset, species),
        "{label}: {name} ({species:#04x}) not found in grass encounter table"
    );
}

/// Assert a species is present in the water encounter table for the given map label.
/// Water data follows immediately after grass data (at offset + WILDDATA_LENGTH).
fn assert_in_water(rom: &[u8], sym: &SymbolTable, label: &str, species: u8, name: &str) {
    let grass_offset = sym
        .rom_offset(label)
        .unwrap_or_else(|| panic!("symbol {label} not found"));
    let water_offset = grass_offset + WILDDATA_LENGTH;
    assert!(
        rom[water_offset] > 0,
        "{label}: water encounter rate is 0, expected encounters"
    );
    assert!(
        encounter_table_contains(rom, water_offset, species),
        "{label}: {name} ({species:#04x}) not found in water encounter table"
    );
}

// --- Route 2: Weedle, Kakuna ---

#[test]
fn route_2_has_weedle() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "Route2WildMons", WEEDLE, "WEEDLE");
}

#[test]
fn route_2_has_kakuna() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "Route2WildMons", KAKUNA, "KAKUNA");
}

// --- Route 4: Ekans ---

#[test]
fn route_4_has_ekans() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "Route4WildMons", EKANS, "EKANS");
}

// --- Power Plant: Raichu, Electabuzz ---

#[test]
fn power_plant_has_raichu() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "PowerPlantWildMons", RAICHU, "RAICHU");
}

#[test]
fn power_plant_has_electabuzz() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "PowerPlantWildMons", ELECTABUZZ, "ELECTABUZZ");
}

// --- Route 5: Meowth ---

#[test]
fn route_5_has_meowth() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "Route5WildMons", MEOWTH, "MEOWTH");
}

// --- Pokemon Mansion: Koffing, Weezing, Magmar ---

#[test]
fn pokemon_mansion_1f_has_koffing() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "PokemonMansion1FWildMons", KOFFING, "KOFFING");
}

#[test]
fn pokemon_mansion_2f_has_koffing() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "PokemonMansion2FWildMons", KOFFING, "KOFFING");
}

#[test]
fn pokemon_mansion_2f_has_weezing() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "PokemonMansion2FWildMons", WEEZING, "WEEZING");
}

#[test]
fn pokemon_mansion_3f_has_magmar() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "PokemonMansion3FWildMons", MAGMAR, "MAGMAR");
}

#[test]
fn pokemon_mansion_b1f_has_magmar() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "PokemonMansionB1FWildMons", MAGMAR, "MAGMAR");
}

// --- Seafoam Islands B4F: Jynx (grass), Omanyte (water), Kabuto (water) ---

#[test]
fn seafoam_islands_b4f_has_jynx() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "SeafoamIslandsB4FWildMons", JYNX, "JYNX");
}

#[test]
fn seafoam_islands_b4f_water_has_omanyte() {
    let (rom, sym) = load_test_data();
    assert_in_water(&rom, &sym, "SeafoamIslandsB4FWildMons", OMANYTE, "OMANYTE");
}

#[test]
fn seafoam_islands_b4f_water_has_kabuto() {
    let (rom, sym) = load_test_data();
    assert_in_water(&rom, &sym, "SeafoamIslandsB4FWildMons", KABUTO, "KABUTO");
}

// --- Route 16: Eevee ---

#[test]
fn route_16_has_eevee() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "Route16WildMons", EEVEE, "EEVEE");
}

// --- Victory Road 2F: Hitmonlee, Hitmonchan ---

#[test]
fn victory_road_2f_has_hitmonlee() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "VictoryRoad2FWildMons", HITMONLEE, "HITMONLEE");
}

#[test]
fn victory_road_2f_has_hitmonchan() {
    let (rom, sym) = load_test_data();
    assert_in_grass(
        &rom,
        &sym,
        "VictoryRoad2FWildMons",
        HITMONCHAN,
        "HITMONCHAN",
    );
}

// --- Cerulean Cave B1F: Mew ---

#[test]
fn cerulean_cave_b1f_has_mew() {
    let (rom, sym) = load_test_data();
    assert_in_grass(&rom, &sym, "CeruleanCaveB1FWildMons", MEW, "MEW");
}
