//! ROM byte tests for the item duplication glitch fix.
//!
//! Bug: When encountering MissingNo. or other glitch Pokémon, `IndexToPokedex`
//! returns Pokédex number 0. The code does `dec a` (wrapping 0→255) then calls
//! `FlagAction` with bit index 255 on `wPokedexSeen`.  This writes to byte 31
//! of the 19-byte bitfield — 12 bytes out of bounds, into `wBagItems` (the 6th
//! item's quantity byte), adding 128 to it.
//!
//! Fix: After `IndexToPokedex`, check `and a` on the returned Pokédex number.
//! If zero (invalid), skip the `FlagAction` call entirely.  Applied at all three
//! call sites: `LoadEnemyMonData`, `AddPartyMon`, and `_SendNewMonToBox`.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/Item_duplication_glitch>
//! Reference: <https://glitchcity.wiki/wiki/Old_man_glitch>

use pokeyellow_tests::{sym_addr, sym_bank};

// ─── Opcode constants ────────────────────────────────────────────────

const AND_A: u8 = 0xA7; // and a
const JR_Z: u8 = 0x28; // jr z, n
const DEC_A: u8 = 0x3D; // dec a

// ─── Helpers ─────────────────────────────────────────────────────────

fn rom() -> Vec<u8> {
    std::fs::read("../pokeyellow.gbc").expect("ROM not found")
}

fn rom_offset(bank: u32, addr: u16) -> usize {
    if bank == 0 {
        addr as usize
    } else {
        (bank * 0x4000 + (addr as u32 - 0x4000)) as usize
    }
}

fn at(rom: &[u8], bank: u32, addr: u16) -> u8 {
    rom[rom_offset(bank, addr)]
}

// ─── LoadEnemyMonData tests (bank $0F) ───────────────────────────────

#[test]
fn load_enemy_mon_data_in_bank_0f() {
    assert_eq!(sym_bank("LoadEnemyMonData"), 0x0F);
}

#[test]
fn load_enemy_and_a_guard_present() {
    // After ld a, [wPokedexNum], there should be: and a / jr z / dec a
    let rom = rom();
    let skip = sym_addr("LoadEnemyMonData.skipInvalidDex");
    let bank: u32 = 0x0F;

    // Scan backwards from .skipInvalidDex to find ld a,[nn] / and a / jr z / dec a
    // The jr z should target .skipInvalidDex
    let mut found = false;
    for offset in 2..30u16 {
        let check = skip - offset;
        if at(&rom, bank, check) == JR_Z {
            let jr_operand = at(&rom, bank, check + 1) as i8;
            let target = (check as i32 + 2 + jr_operand as i32) as u16;
            if target == skip {
                // Verify and a before jr z
                assert_eq!(
                    at(&rom, bank, check - 1),
                    AND_A,
                    "Expected `and a` before `jr z`"
                );
                // Verify dec a after jr z (the normal path)
                assert_eq!(
                    at(&rom, bank, check + 2),
                    DEC_A,
                    "Expected `dec a` after `jr z` (normal path)"
                );
                found = true;
                break;
            }
        }
    }
    assert!(
        found,
        "Could not find `and a / jr z` guard in LoadEnemyMonData"
    );
}

#[test]
fn load_enemy_skip_label_exists() {
    // .skipInvalidDex label should exist
    let skip = sym_addr("LoadEnemyMonData.skipInvalidDex");
    assert!(skip > 0x4000, ".skipInvalidDex should be in banked ROM");
}

// ─── AddPartyMon tests (bank $03) ────────────────────────────────────

#[test]
fn add_party_mon_in_bank_03() {
    assert_eq!(sym_bank("_AddPartyMon"), 0x03);
}

#[test]
fn add_party_mon_and_a_guard_present() {
    let rom = rom();
    let skip = sym_addr("_AddPartyMon.skipPokedexFlags");
    let bank: u32 = 0x03;

    let mut found = false;
    for offset in 2..40u16 {
        let check = skip - offset;
        if at(&rom, bank, check) == JR_Z {
            let jr_operand = at(&rom, bank, check + 1) as i8;
            let target = (check as i32 + 2 + jr_operand as i32) as u16;
            if target == skip {
                assert_eq!(
                    at(&rom, bank, check - 1),
                    AND_A,
                    "Expected `and a` before `jr z` in AddPartyMon"
                );
                assert_eq!(
                    at(&rom, bank, check + 2),
                    DEC_A,
                    "Expected `dec a` after `jr z` in AddPartyMon"
                );
                found = true;
                break;
            }
        }
    }
    assert!(found, "Could not find `and a / jr z` guard in AddPartyMon");
}

#[test]
fn add_party_mon_skip_label_exists() {
    let skip = sym_addr("_AddPartyMon.skipPokedexFlags");
    assert!(skip > 0x4000, ".skipPokedexFlags should be in banked ROM");
}

// ─── _SendNewMonToBox tests (bank $03) ───────────────────────────────

#[test]
fn send_new_mon_to_box_and_a_guard_present() {
    let rom = rom();
    let skip = sym_addr("_AddEnemyMonToPlayerParty.skipBoxPokedexFlags");
    let bank: u32 = 0x03;

    let mut found = false;
    for offset in 2..30u16 {
        let check = skip - offset;
        if at(&rom, bank, check) == JR_Z {
            let jr_operand = at(&rom, bank, check + 1) as i8;
            let target = (check as i32 + 2 + jr_operand as i32) as u16;
            if target == skip {
                assert_eq!(
                    at(&rom, bank, check - 1),
                    AND_A,
                    "Expected `and a` before `jr z` in _SendNewMonToBox"
                );
                assert_eq!(
                    at(&rom, bank, check + 2),
                    DEC_A,
                    "Expected `dec a` after `jr z` in _SendNewMonToBox"
                );
                found = true;
                break;
            }
        }
    }
    assert!(
        found,
        "Could not find `and a / jr z` guard in _SendNewMonToBox"
    );
}

#[test]
fn send_new_mon_to_box_skip_label_exists() {
    let skip = sym_addr("_AddEnemyMonToPlayerParty.skipBoxPokedexFlags");
    assert!(
        skip > 0x4000,
        ".skipBoxPokedexFlags should be in banked ROM"
    );
}
