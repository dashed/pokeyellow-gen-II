//! ROM byte tests for the MissingNo. / glitch Pokémon SRAM corruption fix.
//!
//! Bug: The sprite decompression routine `_UncompressSpriteData` reads the
//! first byte of compressed sprite data as dimensions (high nybble = width,
//! low nybble = height in tiles). The original `and $f` mask allows up to 15
//! tiles per dimension, but the sprite buffers (`sSpriteBuffer1`/`2`) in SRAM
//! bank 0 only hold 7×7 tiles (392 bytes each). Glitch Pokémon like MissingNo.
//! have garbage sprite data with dimensions exceeding this, causing the
//! decompression to overflow into adjacent SRAM — including `sHallOfFame`.
//!
//! In Yellow specifically, MissingNo.'s dimension byte is $00 (0×0), which
//! causes the decompression loop to never terminate properly, writing
//! arbitrary data across SRAM.
//!
//! Fix: Change `and $f` to `and $7` for both height and width, capping
//! dimensions at 7 tiles maximum. Add `jr nz / inc a` zero-guards to
//! prevent 0-dimension infinite loops (maps 0 to 1). Also optimize the
//! adjacent `wSpriteCurPosX`/`wSpriteCurPosY` clearing with `ld [hli]`
//! to save 1 byte in HOME, keeping the net cost at +5 bytes.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/MissingNo.>
//!   - <https://glitchcity.wiki/wiki/MissingNo.>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

// ─── Opcode constants ────────────────────────────────────────────────

const AND_N: u8 = 0xE6;
const JR_NZ: u8 = 0x20;
const INC_A: u8 = 0x3C;
const ADD_A: u8 = 0x87;
const LD_HLI_A: u8 = 0x22; // ld [hli], a
const LD_HL_A: u8 = 0x77; // ld [hl], a

// ─── Helpers ─────────────────────────────────────────────────────────

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn uncompress_sprite_data_is_in_home() {
    assert_eq!(
        sym_bank("_UncompressSpriteData"),
        0x00,
        "_UncompressSpriteData should be in bank $00 (HOME)"
    );
}

#[test]
fn height_mask_is_7_not_f() {
    // The `and` immediate for height should be $07 (max 7 tiles), not $0F (max 15).
    let mut h = TestHarness::new_headless();
    let base = sym_addr("_UncompressSpriteData");
    let height_nz = sym_addr("_UncompressSpriteData.heightNotZero");

    // `and $7` is 2 bytes before `jr nz`, which is 2 bytes before `.heightNotZero`,
    // plus 1 byte for `inc a`. So `and $7` is at heightNotZero - 5.
    let and_addr = height_nz - 5;
    assert_eq!(rom(&mut h, and_addr), AND_N, "expected `and` opcode");
    assert_eq!(
        rom(&mut h, and_addr + 1),
        0x07,
        "height mask should be $07, not $0F"
    );

    // Verify this is within _UncompressSpriteData
    assert!(
        and_addr > base && and_addr < base + 0x80,
        "height and should be within _UncompressSpriteData"
    );
}

#[test]
fn height_zero_guard_present() {
    // After `and $7`: `jr nz, .heightNotZero` (skips 1 byte) + `inc a`
    let mut h = TestHarness::new_headless();
    let height_nz = sym_addr("_UncompressSpriteData.heightNotZero");

    // jr nz at heightNotZero - 3, displacement at heightNotZero - 2, inc a at heightNotZero - 1
    assert_eq!(
        rom(&mut h, height_nz - 3),
        JR_NZ,
        "expected `jr nz` before .heightNotZero"
    );
    assert_eq!(
        rom(&mut h, height_nz - 2),
        0x01,
        "jr nz displacement should be 1 (skip inc a)"
    );
    assert_eq!(
        rom(&mut h, height_nz - 1),
        INC_A,
        "expected `inc a` as zero guard"
    );
}

#[test]
fn height_multiply_by_8_follows() {
    // .heightNotZero: add a / add a / add a (multiply by 8)
    let mut h = TestHarness::new_headless();
    let height_nz = sym_addr("_UncompressSpriteData.heightNotZero");

    assert_eq!(rom(&mut h, height_nz), ADD_A, "add a at +0");
    assert_eq!(rom(&mut h, height_nz + 1), ADD_A, "add a at +1");
    assert_eq!(rom(&mut h, height_nz + 2), ADD_A, "add a at +2");
}

#[test]
fn width_mask_is_7_not_f() {
    let mut h = TestHarness::new_headless();
    let width_nz = sym_addr("_UncompressSpriteData.widthNotZero");

    // `and $7` is 5 bytes before `.widthNotZero` (same layout as height)
    let and_addr = width_nz - 5;
    assert_eq!(rom(&mut h, and_addr), AND_N, "expected `and` opcode");
    assert_eq!(
        rom(&mut h, and_addr + 1),
        0x07,
        "width mask should be $07, not $0F"
    );
}

#[test]
fn width_zero_guard_present() {
    let mut h = TestHarness::new_headless();
    let width_nz = sym_addr("_UncompressSpriteData.widthNotZero");

    assert_eq!(
        rom(&mut h, width_nz - 3),
        JR_NZ,
        "expected `jr nz` before .widthNotZero"
    );
    assert_eq!(
        rom(&mut h, width_nz - 2),
        0x01,
        "jr nz displacement should be 1"
    );
    assert_eq!(
        rom(&mut h, width_nz - 1),
        INC_A,
        "expected `inc a` as zero guard"
    );
}

#[test]
fn width_multiply_by_8_follows() {
    let mut h = TestHarness::new_headless();
    let width_nz = sym_addr("_UncompressSpriteData.widthNotZero");

    assert_eq!(rom(&mut h, width_nz), ADD_A, "add a at +0");
    assert_eq!(rom(&mut h, width_nz + 1), ADD_A, "add a at +1");
    assert_eq!(rom(&mut h, width_nz + 2), ADD_A, "add a at +2");
}

#[test]
fn sprite_cur_pos_cleared_with_hli_optimization() {
    // wSpriteCurPosX and wSpriteCurPosY are adjacent in WRAM, so
    // they are cleared with `ld [hli], a / ld [hl], a` instead of
    // two separate `ld [nn], a` — saving 1 byte in HOME.
    let mut h = TestHarness::new_headless();
    let base = sym_addr("_UncompressSpriteData");

    // Scan for the `ld [hli], a` + `ld [hl], a` pattern within the
    // first 30 bytes of the function (the initialization block).
    let mut found = false;
    for offset in 0..28 {
        if rom(&mut h, base + offset) == LD_HLI_A && rom(&mut h, base + offset + 1) == LD_HL_A {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "expected ld [hli], a ($22) + ld [hl], a ($77) in init block"
    );
}
