//! ROM byte tests for the lucky slot machine 7-stop fix.
//!
//! Bug: In `SlotMachine_StopWheel1Early.sevenAndBarMode`, the code
//! checks `cp HIGH(SLOTS7)` (cp $02) followed by `jr c, .stopWheel`.
//! Since all valid slot symbol HIGH bytes are >= $02, the carry flag
//! is never set — the condition is always false. The wheel never stops
//! early when a 7 appears, defeating the purpose of the lucky machine.
//!
//! Fix: Change `jr c` ($38) to `jr z` ($28). Now the wheel stops when
//! any of the 3 visible tiles is a 7 symbol (HIGH byte == $02).
//!
//! References:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SlotMachine_StopWheel1Early"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn stop_wheel1_early_in_bank_0d() {
    assert_eq!(
        sym_bank("SlotMachine_StopWheel1Early"),
        0x0D,
        "SlotMachine_StopWheel1Early should be in bank $0D"
    );
}

// ─── sevenAndBarMode loop structure ──────────────────────────────────

#[test]
fn seven_bar_mode_loop_counter_is_3() {
    let mut h = rom_harness();
    let sev = sym_addr("SlotMachine_StopWheel1Early.sevenAndBarMode");
    // ld c, $3 → $0E $03
    assert_eq!(rom(&mut h, sev), 0x0E, "ld c, n opcode at .sevenAndBarMode");
    assert_eq!(
        rom(&mut h, sev + 1),
        0x03,
        "loop counter should be 3 (bottom/middle/top tiles)"
    );
}

#[test]
fn loop_reads_tile_with_hli() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("SlotMachine_StopWheel1Early.loop");
    // ld a, [hli] → $2A
    assert_eq!(rom(&mut h, loop_addr), 0x2A, "ld a, [hli] at .loop");
}

#[test]
fn loop_compares_with_slots7_high() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("SlotMachine_StopWheel1Early.loop");
    // cp HIGH(SLOTS7) → $FE $02 at offset +1
    assert_eq!(rom(&mut h, loop_addr + 1), 0xFE, "cp n opcode");
    assert_eq!(
        rom(&mut h, loop_addr + 2),
        0x02,
        "compare operand should be $02 (HIGH(SLOTS7))"
    );
}

// ─── THE FIX: jr z instead of jr c ──────────────────────────────────

#[test]
fn jr_z_stops_wheel_on_seven() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("SlotMachine_StopWheel1Early.loop");
    // After ld a,[hli] (1) + cp $02 (2) = offset +3: jr z, .stopWheel → $28 xx
    let jr_addr = loop_addr + 3;
    assert_eq!(
        rom(&mut h, jr_addr),
        0x28,
        "Should be jr z ($28), not jr c ($38) — this is the exact bug fix"
    );
}

#[test]
fn jr_z_is_not_old_buggy_jr_c() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("SlotMachine_StopWheel1Early.loop");
    let jr_addr = loop_addr + 3;
    assert_ne!(
        rom(&mut h, jr_addr),
        0x38,
        "Must NOT be jr c ($38) — that was the buggy opcode"
    );
}

#[test]
fn jr_z_targets_stop_wheel() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("SlotMachine_StopWheel1Early.loop");
    let stop_wheel = sym_addr("SlotMachine_StopWheel1Early.stopWheel");
    let jr_addr = loop_addr + 3;
    let jr_offset = rom(&mut h, jr_addr + 1) as i8;
    let jr_pc = jr_addr + 2; // PC after reading jr instruction
    let target = (jr_pc as i32 + jr_offset as i32) as u16;
    assert_eq!(
        target, stop_wheel,
        "jr z should target .stopWheel at ${stop_wheel:04X}"
    );
}

// ─── Wheel 2 seven-and-bar mode (already correct, verify) ───────────

#[test]
fn wheel2_seven_bar_mode_uses_correct_threshold() {
    let mut h = rom_harness();
    let sev2 = sym_addr("SlotMachine_StopWheel2Early.sevenAndBarMode");
    // After call (3 bytes) + ret nz (1 byte):
    // ld a, [de] (1 byte) at +4
    // cp HIGH(SLOTSBAR) + 1 (2 bytes) at +5
    // ret nc (1 byte) at +7
    assert_eq!(rom(&mut h, sev2 + 3), 0xC0, "ret nz at +3");
    assert_eq!(rom(&mut h, sev2 + 4), 0x1A, "ld a, [de] at +4");
    assert_eq!(rom(&mut h, sev2 + 5), 0xFE, "cp n opcode at +5");
    assert_eq!(
        rom(&mut h, sev2 + 6),
        0x07,
        "compare threshold should be $07 (HIGH(SLOTSBAR) + 1)"
    );
    assert_eq!(rom(&mut h, sev2 + 7), 0xD0, "ret nc at +7");
}
