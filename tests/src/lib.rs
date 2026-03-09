pub mod benchmark;
pub mod debug;
pub mod golden;
pub mod harness;
pub mod input;
pub mod link;
pub mod rom;
pub mod symbols;

pub use benchmark::{measure_cycles, measure_cycles_to};
pub use debug::{CpuSnapshot, JoypadSnapshot};
pub use golden::{compare_screenshot, golden_dir, save_screenshot, should_generate};
pub use harness::TestHarness;
pub use input::InputScript;
pub use link::LinkEndpoint;
pub use rom::{load_rom_bytes, rom_path, sym_path};
pub use symbols::SymbolTable;

use std::sync::OnceLock;

static GLOBAL_SYM: OnceLock<SymbolTable> = OnceLock::new();

fn global_sym() -> &'static SymbolTable {
    GLOBAL_SYM.get_or_init(|| SymbolTable::load(sym_path().to_str().unwrap()))
}

/// Resolve a symbol to its (bank, addr) tuple. Panics if not found.
pub fn sym(label: &str) -> (u8, u16) {
    global_sym()
        .resolve(label)
        .unwrap_or_else(|| panic!("symbol '{label}' not found in pokeyellow.sym"))
}

/// Resolve a symbol to its 16-bit address. Panics if not found.
pub fn sym_addr(label: &str) -> u16 {
    sym(label).1
}

/// Resolve a symbol to its ROM bank number. Panics if not found.
pub fn sym_bank(label: &str) -> u8 {
    sym(label).0
}
