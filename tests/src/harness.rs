use boytacean::gb::{GameBoy, GameBoyMode};
use boytacean::pad::PadKey;
use boytacean::state::{SaveStateFormat, StateManager};

use crate::rom::{check_rom_staleness, set_memory_limit, validate_rom, validate_sym_file};
use crate::{rom_path, sym_path, SymbolTable};

/// Maximum number of CPU instructions to execute before giving up in `step_to`.
/// Prevents infinite loops if the target address is never reached.
const MAX_STEPS: u32 = 10_000_000;

/// A test harness wrapping the boytacean Game Boy emulator.
///
/// Provides high-level helpers for loading the Pokemon Yellow ROM,
/// stepping to specific addresses, reading/writing memory, and
/// managing save states for test fixture reuse.
pub struct TestHarness {
    pub gb: GameBoy,
    pub sym: SymbolTable,
    total_cycles: u64,
}

pub fn pad_key_to_u8(key: &PadKey) -> u8 {
    match key {
        PadKey::Up => 1,
        PadKey::Down => 2,
        PadKey::Left => 3,
        PadKey::Right => 4,
        PadKey::Start => 5,
        PadKey::Select => 6,
        PadKey::A => 7,
        PadKey::B => 8,
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl TestHarness {
    /// Create a new harness by loading the built ROM and symbol table.
    ///
    /// Initializes the emulator in DMG mode (Pokemon Yellow is a DMG game
    /// with GBC color enhancements), loads the ROM, and jumps to post-boot
    /// state (PC = $0100).
    pub fn new() -> Self {
        let rom_path = rom_path();
        let rom_data = std::fs::read(&rom_path).unwrap_or_else(|e| {
            panic!(
                "\n\n\
                 ╔══════════════════════════════════════════════════════════╗\n\
                 ║  ROM FILE NOT FOUND                                     ║\n\
                 ╠══════════════════════════════════════════════════════════╣\n\
                 ║  Could not read: {}\n\
                 ║  Error: {}\n\
                 ║                                                         ║\n\
                 ║  Build the ROM first:  make pokeyellow.gbc              ║\n\
                 ╚══════════════════════════════════════════════════════════╝\n",
                rom_path.display(),
                e
            )
        });

        // Safety net: cap virtual memory to prevent runaway emulation
        // from consuming all system memory if validation somehow misses
        // a problem.
        set_memory_limit();

        // Validate ROM BEFORE loading into the emulator.
        // Without this, invalid ROM data causes runaway memory usage.
        validate_rom(&rom_data);
        validate_sym_file();

        // Warn (don't fail) if ROM is older than source files.
        check_rom_staleness();

        let sym = SymbolTable::load(sym_path().to_str().unwrap());

        let mut gb = GameBoy::new(Some(GameBoyMode::Dmg));
        gb.load(true).unwrap();
        gb.load_rom(&rom_data, None).unwrap();
        gb.load_boot_state();

        Self {
            gb,
            sym,
            total_cycles: 0,
        }
    }

    /// Create a headless harness with PPU and APU disabled for faster execution.
    ///
    /// Use this when tests only need CPU + memory and don't inspect the
    /// framebuffer or audio output.
    pub fn new_headless() -> Self {
        let mut harness = Self::new();
        harness.gb.set_ppu_enabled(false);
        harness.gb.set_apu_enabled(false);
        harness
    }

    // ── Memory access ───────────────────────────────────────────────

    /// Read a byte from any address (ROM, WRAM, HRAM, IO).
    pub fn read_mem(&mut self, addr: u16) -> u8 {
        self.gb.read_memory(addr)
    }

    /// Write a byte to any address (WRAM, HRAM, IO, MBC registers).
    pub fn write_mem(&mut self, addr: u16, value: u8) {
        self.gb.write_memory(addr, value);
    }

    // ── CPU stepping ────────────────────────────────────────────────

    /// Run until the program counter reaches `addr`.
    ///
    /// This only compares against the 16-bit PC value. For banked ROM
    /// addresses, ensure the correct bank is selected first via
    /// [`select_rom_bank`].
    ///
    /// Panics if `MAX_STEPS` instructions execute without reaching `addr`.
    pub fn step_to(&mut self, addr: u16) -> u32 {
        let mut cycles = 0u32;
        let mut steps = 0u32;
        while self.gb.cpu_i().pc() != addr {
            cycles += self.gb.clock() as u32;
            steps += 1;
            if steps >= MAX_STEPS {
                panic!(
                    "step_to(${addr:04X}): did not reach target after {MAX_STEPS} instructions \
                     (PC=${:04X})",
                    self.gb.cpu_i().pc()
                );
            }
        }
        cycles
    }

    /// Execute a single CPU instruction. Returns cycles consumed.
    pub fn clock(&mut self) -> u16 {
        self.gb.clock()
    }

    /// Run the CPU one instruction at a time, checking PC after each.
    /// Returns the cycles consumed by the instruction that landed on `addr`.
    ///
    /// Unlike `step_to`, this uses `clock_step` which skips device clocking
    /// when the target is hit, useful for precise breakpoint injection.
    pub fn step_until(&mut self, addr: u16) -> u32 {
        let mut cycles = 0u32;
        let mut steps = 0u32;
        loop {
            let c = self.gb.clock_step(addr) as u32;
            cycles += c;
            steps += 1;
            if self.gb.cpu_i().pc() == addr {
                return cycles;
            }
            if steps >= MAX_STEPS {
                panic!(
                    "step_until(${addr:04X}): did not reach target after {MAX_STEPS} instructions \
                     (PC=${:04X})",
                    self.gb.cpu_i().pc()
                );
            }
        }
    }

    // ── Frame control ───────────────────────────────────────────────

    /// Advance the emulator by `n` frames.
    pub fn run_frames(&mut self, n: u32) -> u32 {
        let mut cycles = 0u32;
        for _ in 0..n {
            let c = self.gb.next_frame();
            cycles += c;
            self.total_cycles += c as u64;
        }
        cycles
    }

    /// Get the current PPU frame counter.
    pub fn frame_count(&mut self) -> u16 {
        self.gb.ppu_frame()
    }

    // ── Bank switching ──────────────────────────────────────────────

    /// Select a ROM bank via the MBC5 bank register.
    ///
    /// Writes the bank number to $2000 (low 8 bits) and $3000 (bit 8).
    /// For Pokemon Yellow (MBC5), banks 0x00–0xFF use $2000 only.
    pub fn select_rom_bank(&mut self, bank: u8) {
        self.gb.write_memory(0x2000, bank);
        // MBC5 bit 8 at $3000 — always 0 for banks <= 255
        self.gb.write_memory(0x3000, 0x00);
    }

    // ── Register access helpers ─────────────────────────────────────

    /// Get the current program counter.
    pub fn pc(&self) -> u16 {
        self.gb.cpu_i().pc()
    }

    /// Set the program counter directly.
    pub fn set_pc(&mut self, addr: u16) {
        self.gb.cpu().set_pc(addr);
    }

    /// Set the stack pointer.
    pub fn set_sp(&mut self, addr: u16) {
        self.gb.cpu().set_sp(addr);
    }

    /// Get register A.
    pub fn a(&self) -> u8 {
        self.gb.cpu_i().a
    }

    /// Set register A.
    pub fn set_a(&mut self, val: u8) {
        self.gb.cpu().a = val;
    }

    /// Get register B.
    pub fn b(&self) -> u8 {
        self.gb.cpu_i().b
    }

    /// Set register B.
    pub fn set_b(&mut self, val: u8) {
        self.gb.cpu().b = val;
    }

    // ── Symbol resolution ───────────────────────────────────────────

    /// Resolve a symbol to (bank, addr). Panics if not found.
    pub fn resolve(&self, label: &str) -> (u8, u16) {
        self.sym
            .resolve(label)
            .unwrap_or_else(|| panic!("symbol '{label}' not found in .sym file"))
    }

    /// Resolve a symbol and return just the 16-bit address.
    /// For banked addresses, you still need to call `select_rom_bank` first.
    pub fn addr_of(&self, label: &str) -> u16 {
        self.resolve(label).1
    }

    /// Resolve a symbol and return the ROM bank number.
    pub fn bank_of(&self, label: &str) -> u8 {
        self.resolve(label).0
    }

    // ── State management ────────────────────────────────────────────

    /// Save the full emulator state to a byte vector.
    pub fn save_state(&mut self) -> Vec<u8> {
        StateManager::save(&mut self.gb, Some(SaveStateFormat::Bosc), None)
            .expect("failed to save emulator state")
    }

    /// Restore emulator state from a previously saved byte vector.
    pub fn load_state(&mut self, data: &[u8]) {
        StateManager::load(data, &mut self.gb, None, None).expect("failed to load emulator state");
    }

    // ── Stack helpers ───────────────────────────────────────────────

    /// Push a 16-bit word onto the stack (SP decreases by 2).
    pub fn push_word(&mut self, word: u16) {
        let sp = self.gb.cpu_i().sp();
        let new_sp = sp.wrapping_sub(2);
        self.gb.cpu().set_sp(new_sp);
        // Game Boy is little-endian: low byte at lower address
        self.gb.write_memory(new_sp, (word & 0xFF) as u8);
        self.gb
            .write_memory(new_sp.wrapping_add(1), (word >> 8) as u8);
    }

    // ── Cycle tracking ──────────────────────────────────────────────

    /// Get the total accumulated CPU cycles since harness creation.
    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    // ── Joypad input ────────────────────────────────────────────────

    /// Press a key, run `frames` frames, then release the key.
    pub fn press(&mut self, key: PadKey, frames: u32) {
        let raw = pad_key_to_u8(&key);
        self.gb.key_press(key);
        self.run_frames(frames);
        self.gb.key_lift(PadKey::from_u8(raw));
    }

    /// Press and hold a key, then run `frames` frames (key stays held).
    pub fn hold(&mut self, key: PadKey, frames: u32) {
        self.gb.key_press(key);
        self.run_frames(frames);
    }

    /// Release a previously held key.
    pub fn release(&mut self, key: PadKey) {
        self.gb.key_lift(key);
    }

    // ── Memory polling ──────────────────────────────────────────────

    /// Run frames until `pred(mem[addr])` is true, up to `max_frames`.
    /// Returns `true` if the predicate was satisfied.
    pub fn wait_for_memory(
        &mut self,
        addr: u16,
        pred: impl Fn(u8) -> bool,
        max_frames: u32,
    ) -> bool {
        for _ in 0..max_frames {
            if pred(self.read_mem(addr)) {
                return true;
            }
            self.run_frames(1);
        }
        pred(self.read_mem(addr))
    }

    // ── Framebuffer / SRAM access ───────────────────────────────────

    /// Capture the current framebuffer as RGB888 pixels (160x144x3 bytes).
    pub fn capture_screenshot(&mut self) -> Vec<u8> {
        self.gb.frame_buffer_eager()
    }

    /// Read the current SRAM data.
    pub fn ram_data(&mut self) -> Vec<u8> {
        self.gb.ram_data_eager()
    }

    /// Set SRAM data.
    pub fn set_ram_data(&mut self, data: Vec<u8>) {
        self.gb.set_ram_data(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_loads_rom() {
        let rom_path = rom_path();
        if !rom_path.exists() {
            eprintln!("skipping: ROM not built yet");
            return;
        }
        let h = TestHarness::new();
        // After boot, PC should be at $0100 (ROM entry point)
        assert_eq!(h.pc(), 0x0100, "PC should be at $0100 after boot");
    }

    #[test]
    fn harness_read_write_wram() {
        let rom_path = rom_path();
        if !rom_path.exists() {
            eprintln!("skipping: ROM not built yet");
            return;
        }
        let mut h = TestHarness::new();
        // Write to WRAM and read back
        h.write_mem(0xC000, 0x42);
        assert_eq!(h.read_mem(0xC000), 0x42);
    }

    #[test]
    fn harness_bank_switching() {
        let rom_path = rom_path();
        if !rom_path.exists() {
            eprintln!("skipping: ROM not built yet");
            return;
        }
        let mut h = TestHarness::new();
        // Select bank $0F and verify we can read from it
        h.select_rom_bank(0x0F);
        // Read from the banked region — should not panic
        let _byte = h.read_mem(0x4000);
    }

    #[test]
    fn harness_save_load_state() {
        let rom_path = rom_path();
        if !rom_path.exists() {
            eprintln!("skipping: ROM not built yet");
            return;
        }
        let mut h = TestHarness::new();
        h.write_mem(0xC000, 0xAB);

        let state = h.save_state();
        assert!(!state.is_empty(), "saved state should not be empty");

        // Mutate memory
        h.write_mem(0xC000, 0x00);
        assert_eq!(h.read_mem(0xC000), 0x00);

        // Restore and verify
        h.load_state(&state);
        assert_eq!(
            h.read_mem(0xC000),
            0xAB,
            "state restore should bring back original value"
        );
    }
}
