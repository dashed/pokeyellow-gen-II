use pokeyellow_tests::{load_rom_bytes, rom_path, sym_path, SymbolTable};

const EVOLVE_LEVEL: u8 = 1;

// Internal IDs from constants/pokemon_constants.asm
const ALAKAZAM: u8 = 0x95;
const GOLEM: u8 = 0x31;
const MACHAMP: u8 = 0x7E;
const GENGAR: u8 = 0x0E;

fn load_test_data() -> (Vec<u8>, SymbolTable) {
    let rom = load_rom_bytes(rom_path().to_str().unwrap());
    let sym = SymbolTable::load(sym_path().to_str().unwrap());
    (rom, sym)
}

fn assert_level_evolution(
    rom: &[u8],
    sym: &SymbolTable,
    label: &str,
    level: u8,
    evolved_species: u8,
) {
    let offset = sym
        .rom_offset(label)
        .unwrap_or_else(|| panic!("symbol {label} not found"));

    assert_eq!(
        rom[offset], EVOLVE_LEVEL,
        "{label}: expected EVOLVE_LEVEL ({EVOLVE_LEVEL}) at offset+0, got {:#04x}",
        rom[offset]
    );
    assert_eq!(
        rom[offset + 1],
        level,
        "{label}: expected level {level} at offset+1, got {}",
        rom[offset + 1]
    );
    assert_eq!(
        rom[offset + 2],
        evolved_species,
        "{label}: expected species {evolved_species:#04x} at offset+2, got {:#04x}",
        rom[offset + 2]
    );
    assert_eq!(
        rom[offset + 3],
        0,
        "{label}: expected 0 terminator at offset+3, got {:#04x}",
        rom[offset + 3]
    );
}

#[test]
fn kadabra_evolves_into_alakazam_at_level_36() {
    let (rom, sym) = load_test_data();
    assert_level_evolution(&rom, &sym, "KadabraEvosMoves", 36, ALAKAZAM);
}

#[test]
fn graveler_evolves_into_golem_at_level_36() {
    let (rom, sym) = load_test_data();
    assert_level_evolution(&rom, &sym, "GravelerEvosMoves", 36, GOLEM);
}

#[test]
fn machoke_evolves_into_machamp_at_level_36() {
    let (rom, sym) = load_test_data();
    assert_level_evolution(&rom, &sym, "MachokeEvosMoves", 36, MACHAMP);
}

#[test]
fn haunter_evolves_into_gengar_at_level_36() {
    let (rom, sym) = load_test_data();
    assert_level_evolution(&rom, &sym, "HaunterEvosMoves", 36, GENGAR);
}
