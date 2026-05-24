//! Shared battle test fixture to reduce setup duplication.
//!
//! # Example
//! ```no_run
//! use pokeyellow_tests::BattleFixture;
//!
//! let mut fix = BattleFixture::new("CalculateDamage");
//! fix.set_whose_turn(0);
//! fix.set_damage(0);
//! fix.set_a(100);
//! fix.set_pc(pokeyellow_tests::sym_addr("CalculateDamage"));
//! fix.run_to_trap();
//! assert!(fix.read_damage() > 0);
//! ```

use std::ops::{Deref, DerefMut};

use crate::{sym_addr, sym_bank, TestHarness};

pub const TRAP_ADDR: u16 = 0xC100;
pub const NOP: u8 = 0x00;
pub const STOP: u8 = 0x10;

/// A battle-focused test fixture wrapping [`TestHarness`].
///
/// Handles the boilerplate of headless initialization, interrupt disabling,
/// ROM bank selection, trap installation, and stack setup. Provides typed
/// helpers for common battle WRAM locations.
pub struct BattleFixture {
    harness: TestHarness,
}

impl BattleFixture {
    /// Create a new fixture targeting `function_label`.
    ///
    /// Performs: headless init, disable IME + all interrupts/timers/serial/DMA,
    /// select the ROM bank containing `function_label`, set hLoadedROMBank,
    /// install NOP+STOP trap at TRAP_ADDR, set SP=0xDFF0, push TRAP_ADDR as
    /// the return address.
    pub fn new(function_label: &str) -> Self {
        let mut h = TestHarness::new_headless();
        h.gb.cpu().set_ime(false);
        h.write_mem(0xFFFF, 0x00);
        h.gb.set_timer_enabled(false);
        h.gb.set_serial_enabled(false);
        h.gb.set_dma_enabled(false);
        let bank = sym_bank(function_label);
        h.select_rom_bank(bank);
        h.write_mem(sym_addr("hLoadedROMBank"), bank);
        h.write_mem(TRAP_ADDR, NOP);
        h.write_mem(TRAP_ADDR + 1, STOP);
        h.set_sp(0xDFF0);
        h.push_word(TRAP_ADDR);
        Self { harness: h }
    }

    // ── WRAM helpers ────────────────────────────────────────────────

    pub fn set_damage(&mut self, val: u16) {
        let addr = sym_addr("wDamage");
        self.harness.write_mem(addr, (val >> 8) as u8);
        self.harness.write_mem(addr + 1, (val & 0xFF) as u8);
    }

    pub fn read_damage(&mut self) -> u16 {
        let addr = sym_addr("wDamage");
        let hi = self.harness.read_mem(addr) as u16;
        let lo = self.harness.read_mem(addr + 1) as u16;
        (hi << 8) | lo
    }

    pub fn set_whose_turn(&mut self, turn: u8) {
        self.harness.write_mem(sym_addr("hWhoseTurn"), turn);
    }

    pub fn set_battle_type(&mut self, bt: u8) {
        self.harness.write_mem(sym_addr("wBattleType"), bt);
    }

    pub fn set_player_move_effect(&mut self, eff: u8) {
        self.harness.write_mem(sym_addr("wPlayerMoveEffect"), eff);
    }

    pub fn set_enemy_move_effect(&mut self, eff: u8) {
        self.harness.write_mem(sym_addr("wEnemyMoveEffect"), eff);
    }

    pub fn set_move_missed(&mut self, val: u8) {
        self.harness.write_mem(sym_addr("wMoveMissed"), val);
    }

    pub fn read_move_missed(&mut self) -> u8 {
        self.harness.read_mem(sym_addr("wMoveMissed"))
    }

    // ── Execution helpers ───────────────────────────────────────────

    /// Run until PC reaches TRAP_ADDR. Returns cycles consumed.
    pub fn run_to_trap(&mut self) -> u32 {
        self.harness.step_to(TRAP_ADDR)
    }

    /// Run until PC reaches the address of `label`. Returns cycles consumed.
    pub fn run_to(&mut self, label: &str) -> u32 {
        self.harness.step_to(sym_addr(label))
    }

    /// Set PC to the address of `label`.
    pub fn set_entry(&mut self, label: &str) {
        self.harness.set_pc(sym_addr(label));
    }

    /// Direct mutable access to the underlying harness.
    pub fn harness(&mut self) -> &mut TestHarness {
        &mut self.harness
    }
}

impl Deref for BattleFixture {
    type Target = TestHarness;
    fn deref(&self) -> &Self::Target {
        &self.harness
    }
}

impl DerefMut for BattleFixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.harness
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom_path;

    fn skip_if_no_rom() -> bool {
        if !rom_path().exists() {
            eprintln!("skipping: ROM not built yet");
            return true;
        }
        false
    }

    #[test]
    fn fixture_creates_successfully() {
        if skip_if_no_rom() {
            return;
        }
        let fix = BattleFixture::new("CalculateDamage");
        // PC remains at boot entry ($0100) — we haven't called set_entry yet
        assert_eq!(fix.pc(), 0x0100);
    }

    #[test]
    fn fixture_damage_roundtrip() {
        if skip_if_no_rom() {
            return;
        }
        let mut fix = BattleFixture::new("CalculateDamage");
        fix.set_damage(0x1234);
        assert_eq!(fix.read_damage(), 0x1234);
        fix.set_damage(0);
        assert_eq!(fix.read_damage(), 0);
        fix.set_damage(999);
        assert_eq!(fix.read_damage(), 999);
    }

    #[test]
    fn fixture_move_missed_roundtrip() {
        if skip_if_no_rom() {
            return;
        }
        let mut fix = BattleFixture::new("CalculateDamage");
        fix.set_move_missed(1);
        assert_eq!(fix.read_move_missed(), 1);
        fix.set_move_missed(0);
        assert_eq!(fix.read_move_missed(), 0);
    }

    #[test]
    fn fixture_deref_provides_harness_methods() {
        if skip_if_no_rom() {
            return;
        }
        let mut fix = BattleFixture::new("CalculateDamage");
        // Deref should let us call TestHarness methods directly
        fix.write_mem(0xC000, 0x42);
        assert_eq!(fix.read_mem(0xC000), 0x42);
    }

    #[test]
    fn fixture_set_entry_updates_pc() {
        if skip_if_no_rom() {
            return;
        }
        let mut fix = BattleFixture::new("CalculateDamage");
        fix.set_entry("CalculateDamage");
        assert_eq!(fix.pc(), sym_addr("CalculateDamage"));
    }
}
