//! ROM byte tests for the hidden 40-coin stash fix.
//!
//! Bug: In `HiddenCoins`, the comparison chain for coin amounts has:
//!   ```
//!   cp 40
//!   jr z, .bcd20  ; should be .bcd40
//!   ```
//! The `jr z` target is `.bcd20` instead of `.bcd40`, so the 40-coin
//! hidden stash in the Celadon Game Corner gives only 20 coins.
//! The `.bcd40` label exists but was unreachable.
//!
//! Fix: Change `jr z, .bcd20` to `jr z, .bcd40`. Zero ROM growth —
//! only the relative jump offset changes.
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("HiddenCoins"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn hidden_coins_in_bank_1d() {
    assert_eq!(sym_bank("HiddenCoins"), 0x1D);
}

// ─── BCD value labels ────────────────────────────────────────────────

#[test]
fn bcd10_loads_0x10() {
    let mut h = rom_harness();
    let addr = sym_addr("HiddenCoins.bcd10");
    // ld a, $10 → $3E $10
    assert_eq!(rom(&mut h, addr), 0x3E, "ld a, n opcode at .bcd10");
    assert_eq!(rom(&mut h, addr + 1), 0x10, "BCD value = $10 (10 coins)");
}

#[test]
fn bcd20_loads_0x20() {
    let mut h = rom_harness();
    let addr = sym_addr("HiddenCoins.bcd20");
    assert_eq!(rom(&mut h, addr), 0x3E, "ld a, n opcode at .bcd20");
    assert_eq!(rom(&mut h, addr + 1), 0x20, "BCD value = $20 (20 coins)");
}

#[test]
fn bcd40_loads_0x40() {
    let mut h = rom_harness();
    let addr = sym_addr("HiddenCoins.bcd40");
    assert_eq!(rom(&mut h, addr), 0x3E, "ld a, n opcode at .bcd40");
    assert_eq!(rom(&mut h, addr + 1), 0x40, "BCD value = $40 (40 coins)");
}

#[test]
fn bcd100_loads_0x01() {
    let mut h = rom_harness();
    let addr = sym_addr("HiddenCoins.bcd100");
    // ld a, $1 → $3E $01 (sets hCoins high byte)
    assert_eq!(rom(&mut h, addr), 0x3E, "ld a, n opcode at .bcd100");
    assert_eq!(
        rom(&mut h, addr + 1),
        0x01,
        "BCD high byte = $01 (100 coins)"
    );
}

// ─── THE FIX: jr z targets .bcd40, not .bcd20 ───────────────────────

#[test]
fn cp_40_jr_z_targets_bcd40() {
    let mut h = rom_harness();
    let bcd40 = sym_addr("HiddenCoins.bcd40");
    let bcd20 = sym_addr("HiddenCoins.bcd20");

    // Find the `cp 40 / jr z` by working backwards from .bcd40's caller.
    // The comparison chain is: cp 10 / jr z .bcd10 / cp 20 / jr z .bcd20 / cp 40 / jr z .bcd40
    // We know .bcd10 is at a known address, and before it there's `jr .bcd100` (2) + `jr z` (2) + `cp 40` (2)
    // Let's find it relative to .bcd10: the `jr .bcd100` is at .bcd10 - 2, `jr z, .bcd40` at .bcd10 - 4, `cp 40` at .bcd10 - 6
    // But there's .doNotPickUpCoins between them. Let me use a different approach.

    // Scan backwards from .bcd10 to find the code section.
    // Actually, the code is at HiddenCoins + known offset. Let me compute from HiddenCoins.
    // But it's easier to scan for `cp 40` ($FE $28) before .bcd10.
    let base = sym_addr("HiddenCoins");
    let bcd10 = sym_addr("HiddenCoins.bcd10");

    // Search for FE 28 (cp 40) between HiddenCoins and .bcd10
    let mut cp40_addr = None;
    for addr in base..bcd10 {
        if rom(&mut h, addr) == 0xFE && rom(&mut h, addr + 1) == 40 {
            cp40_addr = Some(addr);
        }
    }
    let cp40_addr = cp40_addr.expect("cp 40 ($FE $28) not found between HiddenCoins and .bcd10");

    // jr z should be at cp40_addr + 2
    let jr_addr = cp40_addr + 2;
    assert_eq!(rom(&mut h, jr_addr), 0x28, "jr z opcode after cp 40");

    let jr_offset = rom(&mut h, jr_addr + 1) as i8;
    let jr_pc = jr_addr + 2;
    let target = (jr_pc as i32 + jr_offset as i32) as u16;

    assert_eq!(
        target, bcd40,
        "jr z after cp 40 should target .bcd40 (${bcd40:04X}), not .bcd20 (${bcd20:04X})"
    );
}

#[test]
fn jr_z_does_not_target_bcd20() {
    let mut h = rom_harness();
    let bcd20 = sym_addr("HiddenCoins.bcd20");
    let base = sym_addr("HiddenCoins");
    let bcd10 = sym_addr("HiddenCoins.bcd10");

    // Find cp 40
    let mut cp40_addr = None;
    for addr in base..bcd10 {
        if rom(&mut h, addr) == 0xFE && rom(&mut h, addr + 1) == 40 {
            cp40_addr = Some(addr);
        }
    }
    let cp40_addr = cp40_addr.expect("cp 40 not found");

    let jr_addr = cp40_addr + 2;
    let jr_offset = rom(&mut h, jr_addr + 1) as i8;
    let jr_pc = jr_addr + 2;
    let target = (jr_pc as i32 + jr_offset as i32) as u16;

    assert_ne!(
        target, bcd20,
        "jr z after cp 40 must NOT target .bcd20 — that was the bug"
    );
}

// ─── Other comparison targets still correct ──────────────────────────

#[test]
fn cp_20_jr_z_targets_bcd20() {
    let mut h = rom_harness();
    let bcd20 = sym_addr("HiddenCoins.bcd20");
    let base = sym_addr("HiddenCoins");
    let bcd10 = sym_addr("HiddenCoins.bcd10");

    // Find cp 20 ($FE $14)
    let mut cp20_addr = None;
    for addr in base..bcd10 {
        if rom(&mut h, addr) == 0xFE && rom(&mut h, addr + 1) == 20 {
            cp20_addr = Some(addr);
        }
    }
    let cp20_addr = cp20_addr.expect("cp 20 not found");

    let jr_addr = cp20_addr + 2;
    assert_eq!(rom(&mut h, jr_addr), 0x28, "jr z opcode after cp 20");

    let jr_offset = rom(&mut h, jr_addr + 1) as i8;
    let jr_pc = jr_addr + 2;
    let target = (jr_pc as i32 + jr_offset as i32) as u16;

    assert_eq!(target, bcd20, "jr z after cp 20 should target .bcd20");
}
