//! ROM byte tests for the slot machine tile loading overrun fix.
//!
//! Bug: `LoadSlotMachineTiles` loads `$1C tiles` (28 tiles = 448 bytes)
//! of `SlotMachineTiles2` data, but the actual tile data is only `$18
//! tiles` (24 tiles = 384 bytes). The extra $40 bytes copy whatever
//! ROM data follows `SlotMachineTiles2End` into VRAM. This doesn't
//! cause visible issues during normal play since those VRAM slots
//! are overwritten before use.
//!
//! Fix: Change both `$1c tiles` to `SlotMachineTiles2End -
//! SlotMachineTiles2` (matching how `SlotMachineTiles1` is already
//! handled).
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Read a little-endian u16 from ROM at the given address.
fn rom16(h: &mut TestHarness, addr: u16) -> u16 {
    let lo = rom(h, addr) as u16;
    let hi = rom(h, addr + 1) as u16;
    (hi << 8) | lo
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn load_slot_machine_tiles_in_bank_0d() {
    assert_eq!(
        sym_bank("LoadSlotMachineTiles"),
        0x0D,
        "LoadSlotMachineTiles should be in bank $0D"
    );
}

#[test]
fn slot_machine_tiles2_data_is_24_tiles() {
    let start = sym_addr("SlotMachineTiles2");
    let end = sym_addr("SlotMachineTiles2End");
    let size = end - start;
    assert_eq!(
        size, 0x180,
        "SlotMachineTiles2 should be $180 bytes (24 tiles × 16 bytes), got ${size:04X}"
    );
}

// ─── First load (to vChars0) ─────────────────────────────────────────

#[test]
fn first_ld_bc_matches_tiles2_size() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("LoadSlotMachineTiles"));

    let base = sym_addr("LoadSlotMachineTiles");
    // Function layout:
    //   +0: call DisableLCD (3 bytes)
    //   +3: ld hl, SlotMachineTiles2 (3 bytes)
    //   +6: ld de, vChars0 (3 bytes)
    //   +9: ld bc, nn (3 bytes) ← THIS
    let ld_bc_addr = base + 9;
    assert_eq!(rom(&mut h, ld_bc_addr), 0x01, "ld bc, nn opcode at +9");

    let expected = sym_addr("SlotMachineTiles2End") - sym_addr("SlotMachineTiles2");
    let actual = rom16(&mut h, ld_bc_addr + 1);
    assert_eq!(
        actual, expected,
        "First ld bc operand should be ${expected:04X} (SlotMachineTiles2 size), got ${actual:04X}"
    );
}

#[test]
fn first_ld_bc_is_not_old_buggy_value() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("LoadSlotMachineTiles"));

    let base = sym_addr("LoadSlotMachineTiles");
    let actual = rom16(&mut h, base + 10); // operand of ld bc at +9
    assert_ne!(
        actual, 0x01C0,
        "First ld bc should NOT be $01C0 (old buggy $1C tiles value)"
    );
}

// ─── Second load (to vChars2 tile $25) ───────────────────────────────

#[test]
fn second_ld_bc_matches_tiles2_size() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("LoadSlotMachineTiles"));

    let base = sym_addr("LoadSlotMachineTiles");
    // Function layout (continued):
    //   +9:  ld bc, nn (3)
    //   +12: ld a, BANK(...) (2)
    //   +14: call FarCopyData (3)
    //   +17: ld hl, SlotMachineTiles1 (3)
    //   +20: ld de, vChars2 (3)
    //   +23: ld bc, nn (3) — SlotMachineTiles1 size
    //   +26: ld a, BANK(...) (2)
    //   +28: call FarCopyData (3)
    //   +31: ld hl, SlotMachineTiles2 (3)
    //   +34: ld de, vChars2 tile $25 (3)
    //   +37: ld bc, nn (3) ← THIS
    let ld_bc_addr = base + 37;
    assert_eq!(rom(&mut h, ld_bc_addr), 0x01, "ld bc, nn opcode at +37");

    let expected = sym_addr("SlotMachineTiles2End") - sym_addr("SlotMachineTiles2");
    let actual = rom16(&mut h, ld_bc_addr + 1);
    assert_eq!(
        actual, expected,
        "Second ld bc operand should be ${expected:04X} (SlotMachineTiles2 size), got ${actual:04X}"
    );
}

#[test]
fn second_ld_bc_is_not_old_buggy_value() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("LoadSlotMachineTiles"));

    let base = sym_addr("LoadSlotMachineTiles");
    let actual = rom16(&mut h, base + 38); // operand of ld bc at +37
    assert_ne!(
        actual, 0x01C0,
        "Second ld bc should NOT be $01C0 (old buggy $1C tiles value)"
    );
}

#[test]
fn both_tiles2_loads_use_same_size() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("LoadSlotMachineTiles"));

    let base = sym_addr("LoadSlotMachineTiles");
    let first = rom16(&mut h, base + 10);
    let second = rom16(&mut h, base + 38);
    assert_eq!(
        first, second,
        "Both SlotMachineTiles2 loads should use the same byte count"
    );
}

// ─── SlotMachineTiles1 (already correct, verify it stays correct) ────

#[test]
fn tiles1_ld_bc_matches_tiles1_size() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("LoadSlotMachineTiles"));

    let base = sym_addr("LoadSlotMachineTiles");
    // ld bc for tiles1 is at +23
    let ld_bc_addr = base + 23;
    assert_eq!(rom(&mut h, ld_bc_addr), 0x01, "ld bc, nn opcode at +23");

    let expected = sym_addr("SlotMachineTiles1End") - sym_addr("SlotMachineTiles1");
    let actual = rom16(&mut h, ld_bc_addr + 1);
    assert_eq!(
        actual, expected,
        "SlotMachineTiles1 ld bc should match its data size (${expected:04X}), got ${actual:04X}"
    );
}
