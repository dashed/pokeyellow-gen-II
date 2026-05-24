//! ROM byte tests for the ED tile emulator compatibility fix.
//!
//! Bug: `LoadEDTile` manually copies the ED tile to VRAM during HBlank
//! instead of using the standard `CopyVideoDataDouble` function. This was
//! GameFreak's workaround for the MBC3→MBC5 bank 0 mapping difference
//! (Red/Blue's MBC3 treated bank 0 as bank 1; Yellow's MBC5 does not).
//! The manual HBlank copy works on real hardware and accurate emulators,
//! but fails on poorly-coded emulators that don't handle HBlank timing.
//!
//! Fix: Replace the manual HBlank copy with `jp CopyVideoDataDouble`,
//! which uses proper V-blank DMA timing for maximum emulator compatibility.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/Character_encoding_(Generation_I)>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("LoadEDTile"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn load_ed_tile_in_bank_01() {
    assert_eq!(sym_bank("LoadEDTile"), 0x01);
}

#[test]
fn ed_tile_data_is_8_bytes() {
    let start = sym_addr("ED_Tile");
    let end = sym_addr("ED_TileEnd");
    assert_eq!(
        end - start,
        8,
        "ED tile should be exactly 8 bytes (1 tile in 1bpp)"
    );
}

// ─── THE FIX: uses CopyVideoDataDouble instead of HBlank loop ───────

#[test]
fn load_ed_tile_uses_jp_copy_video_data_double() {
    let mut h = rom_harness();
    let base = sym_addr("LoadEDTile");
    // ld de, ED_Tile → $11 (3 bytes)
    // ld hl, vFont tile $70 → $21 (3 bytes)
    // lb bc, BANK, count → $01 (3 bytes, ld bc nn)
    // jp CopyVideoDataDouble → $C3 (3 bytes)
    let jp_offset = base + 9; // after 3 instructions of 3 bytes each
    assert_eq!(
        rom(&mut h, jp_offset),
        0xC3,
        "fourth instruction should be jp ($C3)"
    );
    let target_lo = rom(&mut h, jp_offset + 1);
    let target_hi = rom(&mut h, jp_offset + 2);
    let target = u16::from_le_bytes([target_lo, target_hi]);
    assert_eq!(
        target,
        sym_addr("CopyVideoDataDouble"),
        "jp target should be CopyVideoDataDouble"
    );
}

#[test]
fn tile_count_operand_is_1() {
    let mut h = rom_harness();
    let base = sym_addr("LoadEDTile");
    // lb bc, BANK, count → ld bc, $BBCC at offset +6
    // opcode $01, then low byte = count, high byte = bank
    assert_eq!(rom(&mut h, base + 6), 0x01, "lb bc opcode");
    assert_eq!(
        rom(&mut h, base + 7),
        0x01,
        "tile count should be 1 (one 1bpp tile)"
    );
}

#[test]
fn bank_operand_matches_ed_tile() {
    let mut h = rom_harness();
    let base = sym_addr("LoadEDTile");
    // lb bc, BANK(ED_Tile), count → ld bc, $BBCC at offset +6
    // low byte = count (1), high byte = bank
    let bank_byte = rom(&mut h, base + 8);
    assert_eq!(
        bank_byte,
        sym_bank("ED_Tile"),
        "bank byte should match BANK(ED_Tile)"
    );
}

#[test]
fn ld_de_points_to_ed_tile() {
    let mut h = rom_harness();
    let base = sym_addr("LoadEDTile");
    // ld de, ED_Tile → $11 lo hi
    assert_eq!(
        rom(&mut h, base),
        0x11,
        "first instruction should be ld de, nn"
    );
    let addr_lo = rom(&mut h, base + 1);
    let addr_hi = rom(&mut h, base + 2);
    let addr = u16::from_le_bytes([addr_lo, addr_hi]);
    assert_eq!(addr, sym_addr("ED_Tile"), "ld de should point to ED_Tile");
}

// ─── Negative: no HBlank polling loop ────────────────────────────────

#[test]
fn no_hblank_stat_polling_in_load_ed_tile() {
    let mut h = rom_harness();
    let base = sym_addr("LoadEDTile");
    let end = sym_addr("ED_Tile");
    // ldh a, [rSTAT] = $F0 $41 — should not be present in the fixed code
    for addr in base..end.saturating_sub(1) {
        if rom(&mut h, addr) == 0xF0 && rom(&mut h, addr + 1) == 0x41 {
            panic!(
                "Found ldh a, [rSTAT] at ${:04X} — manual HBlank polling still present",
                addr
            );
        }
    }
}

// ─── Function is compact ─────────────────────────────────────────────

#[test]
fn load_ed_tile_is_12_bytes() {
    let start = sym_addr("LoadEDTile");
    let end = sym_addr("ED_Tile");
    assert_eq!(
        end - start,
        12,
        "LoadEDTile should be exactly 12 bytes (4 instructions of 3 bytes each)"
    );
}
