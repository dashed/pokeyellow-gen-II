//! Emulator-based tests for the CooltrainerF AI 25% switch probability fix.
//!
//! The bug: CooltrainerFAI is missing a `ret nc` after `cp 25 percent + 1`.
//! Every other trainer AI that uses this pattern (JugglerAI, GiovanniAI,
//! CooltrainerMAI, MistyAI, etc.) returns early 75% of the time via `ret nc`,
//! but CooltrainerFAI always falls through to the HP check / switch logic.
//! This makes CooltrainerF always switch at 10-20% HP instead of only 25%
//! of the time.
//!
//! The fix: Uncomment `ret nc` after the `cp 25 percent + 1`, restoring the
//! intended 25% probability gate. +1 byte in bank $0E.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// 25 percent + 1 = 25 * 255 / 100 + 1 = 64 = $40.
const THRESHOLD: u8 = 0x40;

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_cooltrainer_f_has_ret_nc() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("CooltrainerFAI"));

    let cooltrainer_f_ai = sym_addr("CooltrainerFAI");

    // CooltrainerFAI should be: cp $40 ($FE $40), ret nc ($D0)
    let cp_opcode = h.read_mem(cooltrainer_f_ai);
    let cp_imm = h.read_mem(cooltrainer_f_ai + 1);
    let ret_nc = h.read_mem(cooltrainer_f_ai + 2);

    assert_eq!(
        cp_opcode, 0xFE,
        "Expected cp imm ($FE) at CooltrainerFAI, got ${cp_opcode:02X}"
    );
    assert_eq!(
        cp_imm, THRESHOLD,
        "Expected threshold ${THRESHOLD:02X}, got ${cp_imm:02X}"
    );
    assert_eq!(
        ret_nc, 0xD0,
        "Expected ret nc ($D0) at CooltrainerFAI+2 (the fix), got ${ret_nc:02X}"
    );
}

#[test]
fn rom_bytes_cooltrainer_m_has_ret_nc() {
    // Symmetry: CooltrainerMAI should also have cp + ret nc
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("CooltrainerMAI"));

    let cooltrainer_m_ai = sym_addr("CooltrainerMAI");

    let cp_opcode = h.read_mem(cooltrainer_m_ai);
    let cp_imm = h.read_mem(cooltrainer_m_ai + 1);
    let ret_nc = h.read_mem(cooltrainer_m_ai + 2);

    assert_eq!(cp_opcode, 0xFE, "CooltrainerMAI: expected cp ($FE)");
    assert_eq!(
        cp_imm, THRESHOLD,
        "CooltrainerMAI: expected threshold ${THRESHOLD:02X}"
    );
    assert_eq!(ret_nc, 0xD0, "CooltrainerMAI: expected ret nc ($D0)");
}

#[test]
fn rom_bytes_juggler_has_ret_nc() {
    // Symmetry: JugglerAI should also have cp + ret nc
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("JugglerAI"));

    let juggler_ai = sym_addr("JugglerAI");

    let cp_opcode = h.read_mem(juggler_ai);
    let cp_imm = h.read_mem(juggler_ai + 1);
    let ret_nc = h.read_mem(juggler_ai + 2);

    assert_eq!(cp_opcode, 0xFE, "JugglerAI: expected cp ($FE)");
    assert_eq!(
        cp_imm, THRESHOLD,
        "JugglerAI: expected threshold ${THRESHOLD:02X}"
    );
    assert_eq!(ret_nc, 0xD0, "JugglerAI: expected ret nc ($D0)");
}

// ─── Behavioral: ret nc gates the HP check ─────────────────────────

#[test]
fn cooltrainer_f_returns_early_when_random_above_threshold() {
    // Random value $FF (above $40 threshold) → ret nc taken → returns to caller
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("CooltrainerFAI");
    h.select_rom_bank(bank);

    let cooltrainer_f_ai = sym_addr("CooltrainerFAI");

    // Set up trap as return address
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // A = $FF (well above 25% threshold $40)
    h.set_a(0xFF);
    h.set_pc(cooltrainer_f_ai);

    // Step: cp $40 sets no carry (A=$FF >= $40), ret nc taken
    h.step_to(TRAP_ADDR);

    let pc = h.gb.cpu_i().pc();
    assert_eq!(
        pc, TRAP_ADDR,
        "Expected return to trap (ret nc taken), PC=${pc:04X}"
    );
}

#[test]
fn cooltrainer_f_returns_early_at_exact_threshold() {
    // Random value $40 (== threshold) → cp $40 sets Z but no carry → ret nc taken
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("CooltrainerFAI");
    h.select_rom_bank(bank);

    let cooltrainer_f_ai = sym_addr("CooltrainerFAI");

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // A = $40 (exactly at threshold: cp $40 → Z=1, C=0 → ret nc taken)
    h.set_a(THRESHOLD);
    h.set_pc(cooltrainer_f_ai);

    h.step_to(TRAP_ADDR);

    let pc = h.gb.cpu_i().pc();
    assert_eq!(
        pc, TRAP_ADDR,
        "Expected return to trap at exact threshold (ret nc taken), PC=${pc:04X}"
    );
}

#[test]
fn cooltrainer_f_falls_through_when_random_below_threshold() {
    // Random value $00 (below $40 threshold) → carry set → ret nc NOT taken
    // Falls through to `ld a, 10` at CooltrainerFAI+3
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("CooltrainerFAI");
    h.select_rom_bank(bank);

    let cooltrainer_f_ai = sym_addr("CooltrainerFAI");

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // A = $00 (below threshold: cp $40 → C=1 → ret nc NOT taken)
    h.set_a(0x00);
    h.set_pc(cooltrainer_f_ai);

    // Step to CooltrainerFAI+3 (the `ld a, 10` instruction after ret nc)
    h.step_to(cooltrainer_f_ai + 3);

    let pc = h.gb.cpu_i().pc();
    assert_eq!(
        pc,
        cooltrainer_f_ai + 3,
        "Expected fall-through to HP check (ret nc not taken), PC=${pc:04X}"
    );
}

#[test]
fn cooltrainer_f_falls_through_just_below_threshold() {
    // Random value $3F (one below $40 threshold) → carry set → falls through
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("CooltrainerFAI");
    h.select_rom_bank(bank);

    let cooltrainer_f_ai = sym_addr("CooltrainerFAI");

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // A = $3F (just below threshold $40: cp $40 → C=1 → ret nc NOT taken)
    h.set_a(THRESHOLD - 1);
    h.set_pc(cooltrainer_f_ai);

    h.step_to(cooltrainer_f_ai + 3);

    let pc = h.gb.cpu_i().pc();
    assert_eq!(
        pc,
        cooltrainer_f_ai + 3,
        "Expected fall-through just below threshold, PC=${pc:04X}"
    );
}
