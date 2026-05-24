//! ROM byte tests for the Mr. Fuji rescue warp fix.
//!
//! Bug: After saving Mr. Fuji from Pokémon Tower 7F, the game warps the
//! player to Mr. Fuji's house but doesn't set `BIT_STANDING_ON_WARP` in
//! `wMovementFlags`. The player must move one tile before they can step on
//! a warp tile and leave, making the transition feel broken.
//!
//! Fix: Add `ld hl, wMovementFlags` / `set BIT_STANDING_ON_WARP, [hl]`
//! before the existing `wStatusFlags3` / `BIT_WARP_FROM_CUR_SCRIPT` set.
//! +5 bytes in bank $18.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PokemonTower7FWarpToMrFujiHouseScript"));
    h
}

// Z80/SM83 opcodes
const LD_HL_IMM: u8 = 0x21; // ld hl, nn
const SET_2_HL: [u8; 2] = [0xCB, 0xD6]; // set 2, [hl]
const SET_3_HL: [u8; 2] = [0xCB, 0xDE]; // set 3, [hl]

// WRAM addresses (little-endian)
const W_MOVEMENT_FLAGS: u16 = 0xD735;
const W_STATUS_FLAGS3: u16 = 0xD72C;

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn function_in_bank_18() {
    assert_eq!(sym_bank("PokemonTower7FWarpToMrFujiHouseScript"), 0x18);
}

#[test]
fn function_in_banked_range() {
    let addr = sym_addr("PokemonTower7FWarpToMrFujiHouseScript");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── The fix: set BIT_STANDING_ON_WARP in wMovementFlags ────────────

#[test]
fn ld_hl_w_movement_flags_present() {
    // `ld hl, wMovementFlags` ($21 $35 $D7) should appear in the function.
    let mut h = banked_harness();
    let start = sym_addr("PokemonTower7FWarpToMrFujiHouseScript");
    let end = sym_addr("PokemonTower7F_TextPointers");
    let lo = (W_MOVEMENT_FLAGS & 0xFF) as u8;
    let hi = (W_MOVEMENT_FLAGS >> 8) as u8;
    let mut found = false;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == lo
            && rom(&mut h, addr + 2) == hi
        {
            found = true;
            break;
        }
    }
    assert!(found, "ld hl, wMovementFlags not found in function");
}

#[test]
fn set_2_hl_follows_ld_hl_movement_flags() {
    // After `ld hl, wMovementFlags`, expect `set 2, [hl]` ($CB $D6).
    let mut h = banked_harness();
    let start = sym_addr("PokemonTower7FWarpToMrFujiHouseScript");
    let end = sym_addr("PokemonTower7F_TextPointers");
    let lo = (W_MOVEMENT_FLAGS & 0xFF) as u8;
    let hi = (W_MOVEMENT_FLAGS >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == lo
            && rom(&mut h, addr + 2) == hi
        {
            // ld hl, nn is 3 bytes; set 2, [hl] follows at +3
            assert_eq!(
                rom(&mut h, addr + 3),
                SET_2_HL[0],
                "CB prefix expected after ld hl, wMovementFlags"
            );
            assert_eq!(
                rom(&mut h, addr + 4),
                SET_2_HL[1],
                "set 2, [hl] ($D6) expected after ld hl, wMovementFlags"
            );
            return;
        }
    }
    panic!("ld hl, wMovementFlags not found");
}

// ─── Context: wStatusFlags3 / BIT_WARP_FROM_CUR_SCRIPT still present ─

#[test]
fn ld_hl_w_status_flags3_present() {
    // `ld hl, wStatusFlags3` ($21 $2C $D7) should still appear.
    let mut h = banked_harness();
    let start = sym_addr("PokemonTower7FWarpToMrFujiHouseScript");
    let end = sym_addr("PokemonTower7F_TextPointers");
    let lo = (W_STATUS_FLAGS3 & 0xFF) as u8;
    let hi = (W_STATUS_FLAGS3 >> 8) as u8;
    let mut found = false;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == lo
            && rom(&mut h, addr + 2) == hi
        {
            found = true;
            break;
        }
    }
    assert!(found, "ld hl, wStatusFlags3 not found in function");
}

#[test]
fn set_3_hl_follows_ld_hl_status_flags3() {
    // After `ld hl, wStatusFlags3`, expect `set 3, [hl]` ($CB $DE).
    let mut h = banked_harness();
    let start = sym_addr("PokemonTower7FWarpToMrFujiHouseScript");
    let end = sym_addr("PokemonTower7F_TextPointers");
    let lo = (W_STATUS_FLAGS3 & 0xFF) as u8;
    let hi = (W_STATUS_FLAGS3 >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == lo
            && rom(&mut h, addr + 2) == hi
        {
            assert_eq!(
                rom(&mut h, addr + 3),
                SET_3_HL[0],
                "CB prefix expected after ld hl, wStatusFlags3"
            );
            assert_eq!(
                rom(&mut h, addr + 4),
                SET_3_HL[1],
                "set 3, [hl] ($DE) expected after ld hl, wStatusFlags3"
            );
            return;
        }
    }
    panic!("ld hl, wStatusFlags3 not found");
}

// ─── Ordering: wMovementFlags set before wStatusFlags3 set ──────────

#[test]
fn movement_flags_set_before_status_flags3() {
    // The wMovementFlags set should come before the wStatusFlags3 set.
    let mut h = banked_harness();
    let start = sym_addr("PokemonTower7FWarpToMrFujiHouseScript");
    let end = sym_addr("PokemonTower7F_TextPointers");
    let mf_lo = (W_MOVEMENT_FLAGS & 0xFF) as u8;
    let mf_hi = (W_MOVEMENT_FLAGS >> 8) as u8;
    let sf_lo = (W_STATUS_FLAGS3 & 0xFF) as u8;
    let sf_hi = (W_STATUS_FLAGS3 >> 8) as u8;
    let mut mf_addr = None;
    let mut sf_addr = None;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM {
            if rom(&mut h, addr + 1) == mf_lo && rom(&mut h, addr + 2) == mf_hi {
                mf_addr = Some(addr);
            }
            if rom(&mut h, addr + 1) == sf_lo && rom(&mut h, addr + 2) == sf_hi {
                sf_addr = Some(addr);
            }
        }
    }
    let mf = mf_addr.expect("ld hl, wMovementFlags not found");
    let sf = sf_addr.expect("ld hl, wStatusFlags3 not found");
    assert!(
        mf < sf,
        "wMovementFlags set at {:#06X} should come before wStatusFlags3 set at {:#06X}",
        mf,
        sf
    );
}

// ─── Regression: exactly two set-bit-on-hl patterns ─────────────────

#[test]
fn exactly_two_ld_hl_set_patterns() {
    // The function should have exactly 2 `ld hl, nn` + `set n, [hl]` patterns:
    // one for wMovementFlags and one for wStatusFlags3.
    let mut h = banked_harness();
    let start = sym_addr("PokemonTower7FWarpToMrFujiHouseScript");
    let end = sym_addr("PokemonTower7F_TextPointers");
    let mut count = 0;
    let mut addr = start;
    while addr < end.saturating_sub(4) {
        if rom(&mut h, addr) == LD_HL_IMM && rom(&mut h, addr + 3) == 0xCB {
            let set_byte = rom(&mut h, addr + 4);
            // CB xx where xx has bits [7:6] = 11 (SET) and [2:0] = 110 (HL)
            if (set_byte & 0xC7) == 0xC6 {
                count += 1;
                addr += 5;
                continue;
            }
        }
        addr += 1;
    }
    assert_eq!(
        count, 2,
        "expected exactly 2 `ld hl, nn` + `set n, [hl]` patterns, found {}",
        count
    );
}
