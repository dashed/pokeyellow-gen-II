//! ROM byte tests for the Repel step counting oversight fix.
//!
//! Bug: `TryDoWildEncounter` decrements `wRepelRemainingSteps` on every call,
//! including when called from the direction-change path (when BIT_TURNING is
//! set in wMiscFlags). This means turning in place or changing direction
//! before walking costs an extra repel step.
//!
//! Fix: Before decrementing repel steps, check `BIT_TURNING` in `wMiscFlags`.
//! If the player is only turning (not moving), skip the decrement but still
//! apply the repel's encounter level filter.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_step_counting_oversight>

use pokeyellow_tests::{sym_addr, sym_bank};

fn rom() -> Vec<u8> {
    std::fs::read("../pokeyellow.gbc").expect("ROM not found")
}

fn rom_offset(bank: u32, addr: u16) -> usize {
    (bank * 0x4000 + (addr as u32 - 0x4000)) as usize
}

fn at(rom: &[u8], bank: u32, addr: u16) -> u8 {
    rom[rom_offset(bank, addr)]
}

// ─── Structural test ─────────────────────────────────────────────────

#[test]
fn try_do_wild_encounter_in_bank_04() {
    assert_eq!(
        sym_bank("TryDoWildEncounter"),
        0x04,
        "TryDoWildEncounter should be in bank $04"
    );
}

// ─── THE FIX: BIT_TURNING check before repel decrement ──────────────

#[test]
fn repel_check_loads_repel_steps_first() {
    let rom = rom();
    let bank = sym_bank("TryDoWildEncounter") as u32;
    let not_on_door = sym_addr("TryDoWildEncounter.notStandingOnDoorOrWarpTile");

    // After callfar IsPlayerJustOutsideMap (8) + jr z (2) = +10 from .notStandingOnDoorOrWarpTile
    let repel_check = not_on_door + 10;

    // ld a, [wRepelRemainingSteps] ($FA)
    assert_eq!(at(&rom, bank, repel_check), 0xFA, "Expected ld a,[nn] for wRepelRemainingSteps");
    let lo = at(&rom, bank, repel_check + 1);
    let hi = at(&rom, bank, repel_check + 2);
    let addr = u16::from_le_bytes([lo, hi]);
    assert_eq!(addr, sym_addr("wRepelRemainingSteps"), "Should load wRepelRemainingSteps");

    // and a ($A7)
    assert_eq!(at(&rom, bank, repel_check + 3), 0xA7, "Expected and a");

    // jr z, .next ($28)
    assert_eq!(at(&rom, bank, repel_check + 4), 0x28, "Expected jr z to .next");
}

#[test]
fn turning_check_reads_misc_flags_and_bit_turning() {
    let rom = rom();
    let bank = sym_bank("TryDoWildEncounter") as u32;
    let not_on_door = sym_addr("TryDoWildEncounter.notStandingOnDoorOrWarpTile");

    // The BIT_TURNING check is at +16 from .notStandingOnDoorOrWarpTile
    // (after callfar[8] + jr z[2] + ld a,[nn][3] + and a[1] + jr z[2] = 16)
    let turning_check = not_on_door + 16;

    // ld a, [wMiscFlags] ($FA)
    assert_eq!(at(&rom, bank, turning_check), 0xFA, "Expected ld a,[nn] for wMiscFlags");
    let lo = at(&rom, bank, turning_check + 1);
    let hi = at(&rom, bank, turning_check + 2);
    let addr = u16::from_le_bytes([lo, hi]);
    assert_eq!(addr, sym_addr("wMiscFlags"), "Should load wMiscFlags");

    // bit BIT_TURNING, a = CB 57 (bit 2, a)
    assert_eq!(at(&rom, bank, turning_check + 3), 0xCB, "Expected CB prefix");
    assert_eq!(at(&rom, bank, turning_check + 4), 0x57, "Expected bit 2, a (BIT_TURNING)");

    // jr nz, .next ($20)
    assert_eq!(at(&rom, bank, turning_check + 5), 0x20, "Expected jr nz to skip decrement");
}

#[test]
fn jr_nz_on_turning_targets_next() {
    let rom = rom();
    let bank = sym_bank("TryDoWildEncounter") as u32;
    let not_on_door = sym_addr("TryDoWildEncounter.notStandingOnDoorOrWarpTile");

    let jr_addr = not_on_door + 21; // jr nz at turning_check + 5
    let offset = at(&rom, bank, jr_addr + 1) as i8;
    let target = (jr_addr + 2).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("TryDoWildEncounter.next"),
        "jr nz on BIT_TURNING should target .next (skip decrement)"
    );
}

// ─── Normal decrement path still works ───────────────────────────────

#[test]
fn normal_path_reloads_and_decrements_repel_steps() {
    let rom = rom();
    let bank = sym_bank("TryDoWildEncounter") as u32;
    let not_on_door = sym_addr("TryDoWildEncounter.notStandingOnDoorOrWarpTile");

    // After the turning jr nz (2 bytes at +21) = +23
    let dec_path = not_on_door + 23;

    // ld a, [wRepelRemainingSteps] (must reload since A was clobbered by wMiscFlags)
    assert_eq!(at(&rom, bank, dec_path), 0xFA, "Expected ld a,[nn] reload");
    let lo = at(&rom, bank, dec_path + 1);
    let hi = at(&rom, bank, dec_path + 2);
    let addr = u16::from_le_bytes([lo, hi]);
    assert_eq!(addr, sym_addr("wRepelRemainingSteps"), "Reload wRepelRemainingSteps");

    // dec a ($3D)
    assert_eq!(at(&rom, bank, dec_path + 3), 0x3D, "Expected dec a");
}
