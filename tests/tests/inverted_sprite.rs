//! ROM byte tests for the Inverted sprite glitch fix.
//!
//! Bug: LoadFlippedFrontSpriteByMonIndex sets wSpriteFlipped = 1, then
//! LoadFrontSpriteByMonIndex validates the dex number. If the dex number
//! is invalid (0 or >151), the .invalidDexNumber path loads RHYDON and
//! returns WITHOUT clearing wSpriteFlipped. The flag stays set, causing
//! all subsequent sprites to render horizontally inverted until something
//! else (like viewing a valid Pokédex entry) clears it.
//!
//! Fix: Clear wSpriteFlipped (xor a / ld [wSpriteFlipped], a) on the
//! .invalidDexNumber path before returning.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/Inverted_sprite>
//! Reference: <https://glitchcity.wiki/wiki/Inverted_sprites>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom16(h: &mut TestHarness, addr: u16) -> u16 {
    let lo = rom(h, addr) as u16;
    let hi = rom(h, addr + 1) as u16;
    (hi << 8) | lo
}

// ─── Opcode constants ────────────────────────────────────────────────

const XOR_A: u8 = 0xAF; // xor a
const LD_NN_A: u8 = 0xEA; // ld [nn], a
const RET: u8 = 0xC9; // ret

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn load_front_sprite_in_home() {
    assert_eq!(sym_bank("LoadFrontSpriteByMonIndex"), 0x00);
}

#[test]
fn sprite_flipped_cleared_on_invalid_dex() {
    // Between .invalidDexNumber and .validDexNumber, there should be:
    //   xor a                    ($AF)
    //   ld [wSpriteFlipped], a   ($EA lo hi)
    //   ret                      ($C9)
    let mut h = TestHarness::new();

    let invalid = sym_addr("LoadFrontSpriteByMonIndex.invalidDexNumber");
    let valid = sym_addr("LoadFrontSpriteByMonIndex.validDexNumber");
    let sprite_flipped = sym_addr("wSpriteFlipped");

    // Scan for xor a / ld [wSpriteFlipped], a / ret pattern
    let mut found = false;
    for addr in invalid..valid.saturating_sub(4) {
        if rom(&mut h, addr) == XOR_A
            && rom(&mut h, addr + 1) == LD_NN_A
            && rom16(&mut h, addr + 2) == sprite_flipped
            && rom(&mut h, addr + 4) == RET
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "xor a / ld [wSpriteFlipped], a / ret not found in .invalidDexNumber path"
    );
}

#[test]
fn sprite_flipped_also_cleared_on_valid_path() {
    // .validDexNumber path should ALSO clear wSpriteFlipped (existing code)
    let mut h = TestHarness::new();

    let valid = sym_addr("LoadFrontSpriteByMonIndex.validDexNumber");
    let sprite_flipped = sym_addr("wSpriteFlipped");

    // The valid path is larger; scan first 30 bytes for the clear
    let mut found = false;
    for addr in valid..valid + 30 {
        if rom(&mut h, addr) == XOR_A
            && rom(&mut h, addr + 1) == LD_NN_A
            && rom16(&mut h, addr + 2) == sprite_flipped
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "wSpriteFlipped clear not found in .validDexNumber path (existing code broken?)"
    );
}

#[test]
fn invalid_dex_still_loads_rhydon() {
    // The .invalidDexNumber path should still load RHYDON ($01) into wCurPartySpecies
    let mut h = TestHarness::new();

    let invalid = sym_addr("LoadFrontSpriteByMonIndex.invalidDexNumber");
    let valid = sym_addr("LoadFrontSpriteByMonIndex.validDexNumber");

    let rhydon = 0x01_u8; // RHYDON species constant

    // Look for ld a, RHYDON ($3E $01) followed by ld [wCurPartySpecies], a
    let mut found = false;
    for addr in invalid..valid.saturating_sub(4) {
        if rom(&mut h, addr) == 0x3E && rom(&mut h, addr + 1) == rhydon {
            found = true;
            break;
        }
    }
    assert!(found, "ld a, RHYDON not found in .invalidDexNumber path");
}
