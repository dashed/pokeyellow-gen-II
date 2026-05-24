//! ROM byte tests for the slot machine wheel 2 early stop false positive fix.
//!
//! Bug: `SlotMachine_StopWheel2Early.sevenAndBarMode` calls
//! `SlotMachine_FindWheel1Wheel2Matches` but doesn't check the Z flag
//! result. When no match is found (NZ), DE points to
//! `wSlotMachineWheel2BottomTile`. The subsequent `ld a, [de]` /
//! `cp HIGH(SLOTSBAR) + 1` / `ret nc` then incorrectly stops the
//! wheel if the bottom tile happens to be a 7 or bar — even though
//! there's no matching symbol on wheel 1. This reduces the player's
//! odds on the lucky machine.
//!
//! Fix: Add `ret nz` after the call to bail out when no match exists.
//! +1 byte in bank $0D.
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SlotMachine_StopWheel2Early"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn stop_wheel2_early_in_bank_0d() {
    assert_eq!(
        sym_bank("SlotMachine_StopWheel2Early"),
        0x0D,
        "SlotMachine_StopWheel2Early should be in bank $0D"
    );
}

// ─── sevenAndBarMode structure ───────────────────────────────────────

#[test]
fn seven_bar_mode_starts_with_call() {
    let mut h = rom_harness();
    let sev = sym_addr("SlotMachine_StopWheel2Early.sevenAndBarMode");
    // call SlotMachine_FindWheel1Wheel2Matches → $CD xx xx
    assert_eq!(rom(&mut h, sev), 0xCD, "call opcode at .sevenAndBarMode");
}

#[test]
fn ret_nz_after_find_matches_call() {
    let mut h = rom_harness();
    let sev = sym_addr("SlotMachine_StopWheel2Early.sevenAndBarMode");
    // ret nz at offset +3 (after 3-byte call instruction) → $C0
    assert_eq!(
        rom(&mut h, sev + 3),
        0xC0,
        "ret nz ($C0) should follow the call — this is the fix"
    );
}

#[test]
fn ld_a_de_after_ret_nz() {
    let mut h = rom_harness();
    let sev = sym_addr("SlotMachine_StopWheel2Early.sevenAndBarMode");
    // ld a, [de] at offset +4 → $1A
    assert_eq!(
        rom(&mut h, sev + 4),
        0x1A,
        "ld a, [de] at +4 (loads matching tile value)"
    );
}

#[test]
fn cp_slotsbar_plus_1_threshold() {
    let mut h = rom_harness();
    let sev = sym_addr("SlotMachine_StopWheel2Early.sevenAndBarMode");
    // cp HIGH(SLOTSBAR) + 1 at offset +5 → $FE $07
    assert_eq!(rom(&mut h, sev + 5), 0xFE, "cp n opcode at +5");
    assert_eq!(
        rom(&mut h, sev + 6),
        0x07,
        "compare threshold = $07 (HIGH(SLOTSBAR) + 1)"
    );
}

#[test]
fn ret_nc_after_compare() {
    let mut h = rom_harness();
    let sev = sym_addr("SlotMachine_StopWheel2Early.sevenAndBarMode");
    // ret nc at offset +7 → $D0
    assert_eq!(
        rom(&mut h, sev + 7),
        0xD0,
        "ret nc at +7 (return if tile is not 7 or bar)"
    );
}

#[test]
fn stop_wheel_immediately_follows() {
    let sev = sym_addr("SlotMachine_StopWheel2Early.sevenAndBarMode");
    let stop = sym_addr("SlotMachine_StopWheel2Early.stopWheel");
    // .stopWheel should be at sev + 8 (call(3) + ret nz(1) + ld a,[de](1) + cp(2) + ret nc(1))
    assert_eq!(
        sev + 8,
        stop,
        ".stopWheel should be at .sevenAndBarMode + 8"
    );
}

// ─── .stopWheel clears slip counter ──────────────────────────────────

#[test]
fn stop_wheel_xors_a_and_stores() {
    let mut h = rom_harness();
    let stop = sym_addr("SlotMachine_StopWheel2Early.stopWheel");
    // xor a → $AF
    assert_eq!(rom(&mut h, stop), 0xAF, "xor a at .stopWheel");
}
