//! ROM byte tests for the Index #000 post-capture fix.
//!
//! Bug: `wCapturedMonSpecies` is used as a boolean flag where 0 means "no
//! capture."  When a Pokémon with species index #000 ('M) is caught, the
//! capture flag remains 0, so the battle loop's `and a / jr nz` check sees
//! "no capture" and the battle continues — spawning an invisible Ditto.
//!
//! Fix: reorder stores so `wCurPartySpecies` and `wPokedexNum` receive the
//! real species first, then `or 1` before storing to `wCapturedMonSpecies`
//! to guarantee the flag is non-zero for any species index.  +2 bytes.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Index_%23000_post-capture>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Read a ROM byte at the given address (with correct bank selected).
fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Search for a byte pattern within [start, end).
fn find_pattern(h: &mut TestHarness, start: u16, end: u16, pattern: &[u8]) -> Option<u16> {
    if pattern.is_empty() || end <= start {
        return None;
    }
    let len = pattern.len() as u16;
    for addr in start..=(end.saturating_sub(len)) {
        if (0..len).all(|i| rom(h, addr + i) == pattern[i as usize]) {
            return Some(addr);
        }
    }
    None
}

// ─── Opcode constants ────────────────────────────────────────────────

const LD_A_ADDR: u8 = 0xFA; // ld a, [nn]
const LD_ADDR_A: u8 = 0xEA; // ld [nn], a
const OR_N: u8 = 0xF6; // or n
const AND_A: u8 = 0xA7; // and a
const JR_NZ: u8 = 0x20; // jr nz, e

// WRAM addresses
const W_CAPTURED_MON_SPECIES: u16 = 0xD11B;
const W_CUR_PARTY_SPECIES: u16 = 0xCF90;
const W_POKEDEX_NUM: u16 = 0xD11D;
const W_ENEMY_MON_SPECIES: u16 = 0xCFE4;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn item_use_ball_is_in_bank_03() {
    assert_eq!(
        sym_bank("ItemUseBall"),
        0x03,
        "ItemUseBall should be in bank $03"
    );
}

#[test]
fn or_1_before_captured_mon_species_store() {
    // The fix inserts `or 1` ($F6 $01) before `ld [wCapturedMonSpecies], a`.
    // Search between .skip6 and .skipShowingPokedexData for the pattern.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("ItemUseBall"));
    let start = sym_addr("ItemUseBall.skip6");
    let end = sym_addr("ItemUseBall.skipShowingPokedexData");
    let lo = (W_CAPTURED_MON_SPECIES & 0xFF) as u8;
    let hi = (W_CAPTURED_MON_SPECIES >> 8) as u8;
    let pattern = [OR_N, 0x01, LD_ADDR_A, lo, hi];
    assert!(
        find_pattern(&mut h, start, end, &pattern).is_some(),
        "`or 1 / ld [wCapturedMonSpecies], a` should exist (ensures non-zero capture flag)"
    );
}

#[test]
fn cur_party_species_stored_before_or_1() {
    // wCurPartySpecies must be stored BEFORE or 1, so it gets the real species.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("ItemUseBall"));
    let start = sym_addr("ItemUseBall.skip6");
    let end = sym_addr("ItemUseBall.skipShowingPokedexData");
    let party_lo = (W_CUR_PARTY_SPECIES & 0xFF) as u8;
    let party_hi = (W_CUR_PARTY_SPECIES >> 8) as u8;
    let party_store = find_pattern(&mut h, start, end, &[LD_ADDR_A, party_lo, party_hi])
        .expect("ld [wCurPartySpecies], a should exist");
    let or_1 = find_pattern(&mut h, start, end, &[OR_N, 0x01]).expect("or 1 should exist");
    assert!(
        party_store < or_1,
        "wCurPartySpecies store ({:#06X}) should come before or 1 ({:#06X})",
        party_store,
        or_1
    );
}

#[test]
fn pokedex_num_stored_before_or_1() {
    // wPokedexNum must be stored BEFORE or 1, so it gets the real species.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("ItemUseBall"));
    let start = sym_addr("ItemUseBall.skip6");
    let end = sym_addr("ItemUseBall.skipShowingPokedexData");
    let dex_lo = (W_POKEDEX_NUM & 0xFF) as u8;
    let dex_hi = (W_POKEDEX_NUM >> 8) as u8;
    let dex_store = find_pattern(&mut h, start, end, &[LD_ADDR_A, dex_lo, dex_hi])
        .expect("ld [wPokedexNum], a should exist");
    let or_1 = find_pattern(&mut h, start, end, &[OR_N, 0x01]).expect("or 1 should exist");
    assert!(
        dex_store < or_1,
        "wPokedexNum store ({:#06X}) should come before or 1 ({:#06X})",
        dex_store,
        or_1
    );
}

#[test]
fn ld_a_enemy_mon_species_before_stores() {
    // The sequence starts with `ld a, [wEnemyMonSpecies]` ($FA lo hi).
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("ItemUseBall"));
    let start = sym_addr("ItemUseBall.skip6");
    let end = sym_addr("ItemUseBall.skipShowingPokedexData");
    let ems_lo = (W_ENEMY_MON_SPECIES & 0xFF) as u8;
    let ems_hi = (W_ENEMY_MON_SPECIES >> 8) as u8;
    let load_addr = find_pattern(&mut h, start, end, &[LD_A_ADDR, ems_lo, ems_hi])
        .expect("ld a, [wEnemyMonSpecies] should exist");
    let or_1 = find_pattern(&mut h, start, end, &[OR_N, 0x01]).expect("or 1 should exist");
    assert!(
        load_addr < or_1,
        "ld a, [wEnemyMonSpecies] ({:#06X}) should come before or 1 ({:#06X})",
        load_addr,
        or_1
    );
}

#[test]
fn battle_loop_still_checks_captured_mon_species() {
    // The battle loop at UseBagItem.checkIfMonCaptured should still have
    // `ld a, [wCapturedMonSpecies] / and a / jr nz`.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("UseBagItem.checkIfMonCaptured"));
    let start = sym_addr("UseBagItem.checkIfMonCaptured");
    let end = sym_addr("UseBagItem.returnAfterCapturingMon");
    let lo = (W_CAPTURED_MON_SPECIES & 0xFF) as u8;
    let hi = (W_CAPTURED_MON_SPECIES >> 8) as u8;
    let load_addr = find_pattern(&mut h, start, end, &[LD_A_ADDR, lo, hi])
        .expect("ld a, [wCapturedMonSpecies] should exist in battle loop");
    // and a should follow the load
    assert_eq!(
        rom(&mut h, load_addr + 3),
        AND_A,
        "`and a` should follow ld a, [wCapturedMonSpecies]"
    );
    // jr nz should follow and a
    assert_eq!(
        rom(&mut h, load_addr + 4),
        JR_NZ,
        "`jr nz` should follow `and a` (jump to capture handling)"
    );
}

#[test]
fn captured_mon_species_cleared_after_capture() {
    // In .returnAfterCapturingMon, wCapturedMonSpecies should be cleared
    // back to 0 (xor a / ld [wCapturedMonSpecies], a).
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("UseBagItem.returnAfterCapturingMon"));
    let start = sym_addr("UseBagItem.returnAfterCapturingMon");
    // returnAfterCapturingMon is small — scan 16 bytes.
    let lo = (W_CAPTURED_MON_SPECIES & 0xFF) as u8;
    let hi = (W_CAPTURED_MON_SPECIES >> 8) as u8;
    let found = find_pattern(&mut h, start, start + 16, &[LD_ADDR_A, lo, hi]);
    assert!(
        found.is_some(),
        "wCapturedMonSpecies should be stored (cleared) in .returnAfterCapturingMon"
    );
}
