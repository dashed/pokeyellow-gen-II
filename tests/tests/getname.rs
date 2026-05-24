//! ROM byte tests for the GetName TM/HM name type gate fix.
//!
//! Bug: `GetName` unconditionally checks if the name index >= HM01 and
//! redirects to `GetMachineName`, regardless of the name list type.
//! This means any name lookup (Pokémon, moves, trainers, etc.) with an
//! index >= $C4 would incorrectly return a TM/HM name. In vanilla the
//! bug is latent because NUM_POKEMON_INDEXES, NUM_ATTACKS, and
//! NUM_TRAINERS are all < HM01.
//!
//! Fix: Check `wNameListType == ITEM_NAME` before the HM01 comparison.
//! Only item name lookups can trigger the TM/HM redirect. +7 bytes in
//! HOME (ROM0).
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GetName"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn getname_in_bank_0() {
    assert_eq!(
        sym_bank("GetName"),
        0x00,
        "GetName should be in HOME (bank 0)"
    );
}

// ─── THE FIX: type check gates TM/HM redirect ──────────────────────

#[test]
fn first_instruction_loads_name_list_type() {
    let mut h = rom_harness();
    let base = sym_addr("GetName");
    // ld a, [wNameListType] → $FA lo hi
    assert_eq!(rom(&mut h, base), 0xFA, "ld a, [nn] opcode at GetName");
    // wNameListType = $D0B5
    let lo = rom(&mut h, base + 1);
    let hi = rom(&mut h, base + 2);
    assert_eq!(
        (lo, hi),
        (0xB5, 0xD0),
        "should load from wNameListType ($D0B5)"
    );
}

#[test]
fn cp_item_name_follows() {
    let mut h = rom_harness();
    let base = sym_addr("GetName");
    // cp ITEM_NAME → $FE $04 at base+3
    assert_eq!(rom(&mut h, base + 3), 0xFE, "cp n opcode after ld a");
    assert_eq!(
        rom(&mut h, base + 4),
        0x04,
        "compare operand should be ITEM_NAME ($04)"
    );
}

#[test]
fn jr_nz_skips_tm_check_for_non_items() {
    let mut h = rom_harness();
    let base = sym_addr("GetName");
    let not_machine = sym_addr("GetName.notMachine");
    // jr nz, .notMachine → $20 xx at base+11
    // After: ld a, [wNameListType] (3) + cp ITEM_NAME (2) + ld a, [wNameListIndex] (3)
    //        + ld [wNamedObjectIndex], a (3) = offset 11
    let jr_addr = base + 11;
    assert_eq!(rom(&mut h, jr_addr), 0x20, "jr nz opcode");
    let jr_offset = rom(&mut h, jr_addr + 1) as i8;
    let target = (jr_addr as i32 + 2 + jr_offset as i32) as u16;
    assert_eq!(
        target, not_machine,
        "jr nz should target .notMachine at ${not_machine:04X}"
    );
}

#[test]
fn cp_hm01_gated_by_item_check() {
    let mut h = rom_harness();
    let base = sym_addr("GetName");
    // cp HM01 → $FE $C4 at base+13 (after jr nz)
    let cp_addr = base + 13;
    assert_eq!(rom(&mut h, cp_addr), 0xFE, "cp n opcode for HM01 check");
    assert_eq!(
        rom(&mut h, cp_addr + 1),
        0xC4,
        "compare operand should be HM01 ($C4)"
    );
}

#[test]
fn jp_nc_targets_get_machine_name() {
    let mut h = rom_harness();
    let base = sym_addr("GetName");
    let machine = sym_addr("GetMachineName");
    // jp nc, GetMachineName → $D2 lo hi at base+15
    let jp_addr = base + 15;
    assert_eq!(rom(&mut h, jp_addr), 0xD2, "jp nc opcode");
    let lo = rom(&mut h, jp_addr + 1) as u16;
    let hi = rom(&mut h, jp_addr + 2) as u16;
    assert_eq!(
        lo | (hi << 8),
        machine,
        "jp nc should target GetMachineName"
    );
}

// ─── Verify name index is still stored ──────────────────────────────

#[test]
fn stores_name_index_to_named_object_index() {
    let mut h = rom_harness();
    let base = sym_addr("GetName");
    // ld a, [wNameListIndex] at base+5 → $FA lo hi
    assert_eq!(rom(&mut h, base + 5), 0xFA, "ld a, [nn] for wNameListIndex");
    // ld [wNamedObjectIndex], a at base+8 → $EA lo hi
    assert_eq!(
        rom(&mut h, base + 8),
        0xEA,
        "ld [nn], a for wNamedObjectIndex"
    );
}

// ─── Negative test ──────────────────────────────────────────────────

#[test]
fn no_unconditional_hm01_check_at_start() {
    let mut h = rom_harness();
    let base = sym_addr("GetName");
    // The old buggy code had cp HM01 at base+6 (right after ld/ld).
    // Now base+3 should be cp ITEM_NAME ($FE $04), NOT cp HM01 ($FE $C4).
    assert_ne!(
        rom(&mut h, base + 4),
        0xC4,
        "first cp operand must NOT be HM01 — that was the bug (ungated TM/HM redirect)"
    );
}
