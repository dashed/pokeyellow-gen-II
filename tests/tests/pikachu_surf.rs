use pokeyellow_tests::{load_rom_bytes, rom_path, sym_path, SymbolTable};

/// Size of each entry in the BaseStats table (`constants/pokemon_data_constants.asm`).
const BASE_DATA_SIZE: usize = 28;

/// Offset of the TM/HM learnset bitfield within a base stats entry.
///
/// Layout: dex(1) + stats(5) + types(2) + catch(1) + exp(1) + pic_size(1) +
///         pic_ptrs(4) + moves(4) + growth(1) = 20
const BASE_TMHM: usize = 20;

/// Pikachu is Pokedex #25, so index 24 (0-based) in the BaseStats table.
const PIKACHU_INDEX: usize = 24;

/// SURF is HM03 whose `SURF_TMNUM` equals 53 (50 TMs + 3rd HM).
///
/// The `tmhm` macro (`macros/data.asm`) encodes each flag as:
///   byte = (TMNUM - 1) / 8,  bit = (TMNUM - 1) % 8  (LSB first)
///
/// For SURF: byte = 52 / 8 = 6, bit = 52 % 8 = 4.
const SURF_BYTE_OFFSET: usize = 6;
const SURF_BIT_MASK: u8 = 1 << 4;

#[test]
fn pikachu_can_learn_surf() {
    let rom = load_rom_bytes(rom_path().to_str().unwrap());
    let sym = SymbolTable::load(sym_path().to_str().unwrap());

    let base_stats = sym
        .rom_offset("BaseStats")
        .expect("BaseStats not found in symbol table");

    let pikachu = base_stats + PIKACHU_INDEX * BASE_DATA_SIZE;
    let surf_byte = rom[pikachu + BASE_TMHM + SURF_BYTE_OFFSET];

    assert!(
        surf_byte & SURF_BIT_MASK != 0,
        "Pikachu should be able to learn Surf (HM03); \
         tmhm byte 6 is {surf_byte:#04x}, expected bit 4 set",
    );
}
