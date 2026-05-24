//! ROM byte tests for the healing machine tile loading overrun fix.
//!
//! Bug: `AnimateHealingMachine` loads 3 tiles from
//! `PokeCenterFlashingMonitorAndHealBall` via `CopyVideoData`, but the
//! graphic data (`gfx/overworld/heal_machine.2bpp`) only contains 2
//! tiles (monitor + heal ball = 32 bytes). The third "tile" reads 16
//! bytes of garbage from `PokeCenterOAMData` that follows in ROM.
//!
//! Fix: Change tile count from 3 to 2. Zero ROM growth — only the
//! immediate operand changes.
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("AnimateHealingMachine"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn healing_machine_in_bank_1c() {
    assert_eq!(sym_bank("AnimateHealingMachine"), 0x1C);
}

#[test]
fn tile_data_is_exactly_2_tiles() {
    let data_start = sym_addr("PokeCenterFlashingMonitorAndHealBall");
    let data_end = sym_addr("PokeCenterOAMData");
    let size = data_end - data_start;
    assert_eq!(
        size, 32,
        "heal_machine.2bpp should be 32 bytes (2 tiles × 16 bytes)"
    );
}

// ─── THE FIX: tile count is 2, not 3 ────────────────────────────────

#[test]
fn tile_count_is_2() {
    let mut h = rom_harness();
    let base = sym_addr("AnimateHealingMachine");
    // lb bc, BANK(...), N compiles to ld bc, $BBNN (3 bytes: $01 NN BB)
    // At base+6: $01, base+7: tile_count (low byte), base+8: bank (high byte)
    assert_eq!(rom(&mut h, base + 6), 0x01, "ld bc, nn opcode for lb bc");
    assert_eq!(
        rom(&mut h, base + 7),
        0x02,
        "tile count should be 2 (monitor + heal ball), not 3"
    );
}

#[test]
fn tile_count_is_not_buggy_3() {
    let mut h = rom_harness();
    let base = sym_addr("AnimateHealingMachine");
    assert_ne!(
        rom(&mut h, base + 7),
        0x03,
        "tile count must NOT be 3 — that was the bug (reads garbage from OAM data)"
    );
}

// ─── Tile source and destination ─────────────────────────────────────

#[test]
fn ld_de_points_to_tile_data() {
    let mut h = rom_harness();
    let base = sym_addr("AnimateHealingMachine");
    let tile_data = sym_addr("PokeCenterFlashingMonitorAndHealBall");
    // ld de, nn → $11 lo hi
    assert_eq!(rom(&mut h, base), 0x11, "ld de, nn opcode");
    let lo = rom(&mut h, base + 1) as u16;
    let hi = rom(&mut h, base + 2) as u16;
    assert_eq!(
        lo | (hi << 8),
        tile_data,
        "ld de should point to PokeCenterFlashingMonitorAndHealBall"
    );
}

#[test]
fn bank_byte_matches_tile_data_bank() {
    let mut h = rom_harness();
    let base = sym_addr("AnimateHealingMachine");
    let expected_bank = sym_bank("PokeCenterFlashingMonitorAndHealBall");
    // lb bc encodes as ld bc, $BBNN → bank is at base+8
    assert_eq!(
        rom(&mut h, base + 8),
        expected_bank,
        "bank byte should match PokeCenterFlashingMonitorAndHealBall's bank"
    );
}

// ─── OAM data structure ─────────────────────────────────────────────

#[test]
fn monitor_sprite_uses_tile_7c() {
    let mut h = rom_harness();
    let oam = sym_addr("PokeCenterOAMData");
    // dbsprite format: Y, X, tile, attrs (each 4 bytes in OAM)
    // First entry: monitor at tile $7C
    assert_eq!(
        rom(&mut h, oam + 2),
        0x7C,
        "monitor sprite should use tile $7C"
    );
}

#[test]
fn heal_ball_sprites_use_tile_7d() {
    let mut h = rom_harness();
    let oam = sym_addr("PokeCenterOAMData");
    // Second OAM entry (first heal ball) at oam+4, tile at +6
    assert_eq!(
        rom(&mut h, oam + 6),
        0x7D,
        "heal ball sprites should use tile $7D"
    );
}
