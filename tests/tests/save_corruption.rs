//! ROM byte tests for the save corruption (mid-save shutoff) fix.
//!
//! Bug: `SaveMainData` writes player name, main data, sprite data, and box
//! data to SRAM, then computes a checksum — but does NOT write party data.
//! Party data is written later by `SavePartyAndDexData`. If power is lost
//! between the two calls, SRAM contains the new box state (e.g. a deposited
//! Pokémon removed from the box) but the old party state (Pokémon still in
//! party), enabling Pokémon duplication and other exploits.
//!
//! Fix: Add `wPartyDataStart` → `sPartyData` copy to `SaveMainData`, after
//! the sprite data copy and before the checksum computation. This ensures
//! party and box data are always consistent when the checksum is written.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SaveMainData"));
    h
}

// Z80/SM83 opcodes
const LD_HL_IMM: u8 = 0x21; // ld hl, nn
const LD_DE_IMM: u8 = 0x11; // ld de, nn
const LD_BC_IMM: u8 = 0x01; // ld bc, nn
const CALL: u8 = 0xCD; // call nn

// WRAM / SRAM addresses
const W_PARTY_DATA_START: u16 = 0xD162;
const S_PARTY_DATA: u16 = 0xAF2C;
const PARTY_DATA_SIZE: u16 = 0xD2F6 - 0xD162; // wPartyDataEnd - wPartyDataStart

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn save_main_data_in_bank_1c() {
    assert_eq!(sym_bank("SaveMainData"), 0x1C);
}

#[test]
fn save_main_data_in_banked_range() {
    let addr = sym_addr("SaveMainData");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── The fix: party data copy in SaveMainData ────────────────────────

#[test]
fn ld_hl_w_party_data_start_present() {
    // `ld hl, wPartyDataStart` ($21 $62 $D1) should appear in SaveMainData.
    let mut h = banked_harness();
    let start = sym_addr("SaveMainData");
    let end = sym_addr("CalcCheckSum");
    let lo = (W_PARTY_DATA_START & 0xFF) as u8;
    let hi = (W_PARTY_DATA_START >> 8) as u8;
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
    assert!(found, "ld hl, wPartyDataStart not found in SaveMainData");
}

#[test]
fn ld_de_s_party_data_follows() {
    // After `ld hl, wPartyDataStart`, expect `ld de, sPartyData` ($11 $2C $AF).
    let mut h = banked_harness();
    let start = sym_addr("SaveMainData");
    let end = sym_addr("CalcCheckSum");
    let hl_lo = (W_PARTY_DATA_START & 0xFF) as u8;
    let hl_hi = (W_PARTY_DATA_START >> 8) as u8;
    let de_lo = (S_PARTY_DATA & 0xFF) as u8;
    let de_hi = (S_PARTY_DATA >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == hl_lo
            && rom(&mut h, addr + 2) == hl_hi
        {
            // ld hl is 3 bytes; ld de follows at +3
            assert_eq!(rom(&mut h, addr + 3), LD_DE_IMM, "expected ld de, nn");
            assert_eq!(rom(&mut h, addr + 4), de_lo, "expected sPartyData lo");
            assert_eq!(rom(&mut h, addr + 5), de_hi, "expected sPartyData hi");
            return;
        }
    }
    panic!("ld hl, wPartyDataStart not found");
}

#[test]
fn ld_bc_party_size_follows() {
    // After ld de, expect `ld bc, PARTY_DATA_SIZE` ($01 lo hi).
    let mut h = banked_harness();
    let start = sym_addr("SaveMainData");
    let end = sym_addr("CalcCheckSum");
    let hl_lo = (W_PARTY_DATA_START & 0xFF) as u8;
    let hl_hi = (W_PARTY_DATA_START >> 8) as u8;
    let size_lo = (PARTY_DATA_SIZE & 0xFF) as u8;
    let size_hi = (PARTY_DATA_SIZE >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == hl_lo
            && rom(&mut h, addr + 2) == hl_hi
        {
            // ld hl (3) + ld de (3) + ld bc at +6
            assert_eq!(rom(&mut h, addr + 6), LD_BC_IMM, "expected ld bc, nn");
            assert_eq!(
                rom(&mut h, addr + 7),
                size_lo,
                "expected party data size lo byte"
            );
            assert_eq!(
                rom(&mut h, addr + 8),
                size_hi,
                "expected party data size hi byte"
            );
            return;
        }
    }
    panic!("ld hl, wPartyDataStart not found");
}

#[test]
fn call_copy_data_follows() {
    // After ld bc, expect `call CopyData` ($CD lo hi).
    let mut h = banked_harness();
    let start = sym_addr("SaveMainData");
    let end = sym_addr("CalcCheckSum");
    let hl_lo = (W_PARTY_DATA_START & 0xFF) as u8;
    let hl_hi = (W_PARTY_DATA_START >> 8) as u8;
    let copy_data = sym_addr("CopyData");
    let cd_lo = (copy_data & 0xFF) as u8;
    let cd_hi = (copy_data >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == hl_lo
            && rom(&mut h, addr + 2) == hl_hi
        {
            // ld hl (3) + ld de (3) + ld bc (3) + call at +9
            assert_eq!(rom(&mut h, addr + 9), CALL, "expected call opcode");
            assert_eq!(rom(&mut h, addr + 10), cd_lo, "expected CopyData lo");
            assert_eq!(rom(&mut h, addr + 11), cd_hi, "expected CopyData hi");
            return;
        }
    }
    panic!("ld hl, wPartyDataStart not found");
}

// ─── Ordering tests ─────────────────────────────────────────────────

#[test]
fn party_copy_after_sprite_copy() {
    // The party data `ld hl` should come AFTER the sprite data `ld hl`.
    let mut h = banked_harness();
    let start = sym_addr("SaveMainData");
    let end = sym_addr("CalcCheckSum");
    const S_SPRITE_DATA: u16 = 0xAD2C;
    let sprite_de_lo = (S_SPRITE_DATA & 0xFF) as u8;
    let sprite_de_hi = (S_SPRITE_DATA >> 8) as u8;
    let party_hl_lo = (W_PARTY_DATA_START & 0xFF) as u8;
    let party_hl_hi = (W_PARTY_DATA_START >> 8) as u8;
    let mut sprite_addr: Option<u16> = None;
    let mut party_addr: Option<u16> = None;
    for addr in start..end {
        // Find ld de, sSpriteData (the sprite copy destination)
        if rom(&mut h, addr) == LD_DE_IMM
            && rom(&mut h, addr + 1) == sprite_de_lo
            && rom(&mut h, addr + 2) == sprite_de_hi
        {
            sprite_addr = Some(addr);
        }
        // Find ld hl, wPartyDataStart
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == party_hl_lo
            && rom(&mut h, addr + 2) == party_hl_hi
        {
            party_addr = Some(addr);
        }
    }
    let sa = sprite_addr.expect("ld de, sSpriteData not found");
    let pa = party_addr.expect("ld hl, wPartyDataStart not found");
    assert!(
        pa > sa,
        "party copy at {:#06X} should come after sprite copy at {:#06X}",
        pa,
        sa
    );
}

#[test]
fn party_copy_before_checksum() {
    // The party data copy should come BEFORE CalcCheckSum is called.
    let mut h = banked_harness();
    let start = sym_addr("SaveMainData");
    let end = sym_addr("CalcCheckSum");
    let party_hl_lo = (W_PARTY_DATA_START & 0xFF) as u8;
    let party_hl_hi = (W_PARTY_DATA_START >> 8) as u8;
    let calc_addr = sym_addr("CalcCheckSum");
    let calc_lo = (calc_addr & 0xFF) as u8;
    let calc_hi = (calc_addr >> 8) as u8;
    let mut party_addr: Option<u16> = None;
    let mut checksum_call_addr: Option<u16> = None;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == party_hl_lo
            && rom(&mut h, addr + 2) == party_hl_hi
        {
            party_addr = Some(addr);
        }
        if rom(&mut h, addr) == CALL
            && rom(&mut h, addr + 1) == calc_lo
            && rom(&mut h, addr + 2) == calc_hi
        {
            checksum_call_addr = Some(addr);
        }
    }
    let pa = party_addr.expect("ld hl, wPartyDataStart not found");
    let ca = checksum_call_addr.expect("call CalcCheckSum not found in SaveMainData");
    assert!(
        pa < ca,
        "party copy at {:#06X} should come before CalcCheckSum call at {:#06X}",
        pa,
        ca
    );
}
