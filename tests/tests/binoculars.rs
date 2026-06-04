//! ROM byte tests for the binoculars NPC freeze fix.
//!
//! Bug: In the gate 2F binocular scripts (Route 12, 15, 16, 18), when the
//! player is NOT facing up at the binoculars, `GateUpstairsScript_PrintIfFacingUp`
//! sets `wDoNotWaitForButtonPressAfterDisplayingText` to TRUE (1). This causes
//! `DisplayTextID` to enter `HoldTextDisplayOpen`, which loops while the A
//! button is held — freezing all NPC sprite movement for the duration.
//!
//! Fix (two parts):
//!
//! 1. **Banked** (`scripts/Route12Gate2F.asm`): Change `ld a, TRUE` (1) to
//!    `ld a, 2` in `GateUpstairsScript_PrintIfFacingUp` when not facing up.
//!    Zero net ROM growth — only the immediate operand changes.
//!
//! 2. **HOME** (`home/text_script.asm`): Replace the original 2-way check
//!    (`and a / jr nz, HoldTextDisplayOpen`) with a 3-value dispatch:
//!    ```
//!    dec a
//!    jr z, HoldTextDisplayOpen       ; was 1 — hold text open
//!    inc a
//!    jr nz, CloseTextDisplay          ; was ≥2 — close immediately
//!    ```
//!    Value 0 falls through to `AfterDisplayingTextID` (normal wait).
//!    +3 bytes in HOME, offset by 1 byte saved via `xor a` in `CloseSRAM`.
//!
//! Reference:
//!   - <https://glitchcity.wiki/wiki/Binoculars_NPC_Pokemon_Yellow>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── HOME: 3-value dispatch in DisplayTextID ─────────────────────────

/// Helper: returns the address of the first fix byte (dec a), which is
/// at AfterDisplayingTextID - 6 (dec a / jr z / inc a / jr nz = 6 bytes).
fn dispatch_addr() -> u16 {
    sym_addr("AfterDisplayingTextID") - 6
}

#[test]
fn display_text_id_dispatch_is_in_home() {
    assert_eq!(
        sym_bank("AfterDisplayingTextID"),
        0x00,
        "AfterDisplayingTextID should be in bank $00 (HOME)"
    );
}

#[test]
fn dispatch_dec_a() {
    let mut h = TestHarness::new_headless();
    let addr = dispatch_addr();
    assert_eq!(rom(&mut h, addr), 0x3D, "dec a opcode at dispatch start");
}

#[test]
fn dispatch_jr_z_hold_text_open() {
    let mut h = TestHarness::new_headless();
    let addr = dispatch_addr();
    // jr z = $28
    assert_eq!(rom(&mut h, addr + 1), 0x28, "jr z opcode after dec a");
    // Verify target is HoldTextDisplayOpen
    let jr_offset = rom(&mut h, addr + 2) as i8;
    let jr_pc = addr + 3; // PC after reading jr instruction
    let target = (jr_pc as i32 + jr_offset as i32) as u16;
    assert_eq!(
        target,
        sym_addr("HoldTextDisplayOpen"),
        "jr z should target HoldTextDisplayOpen"
    );
}

#[test]
fn dispatch_inc_a() {
    let mut h = TestHarness::new_headless();
    let addr = dispatch_addr();
    assert_eq!(rom(&mut h, addr + 3), 0x3C, "inc a opcode after jr z");
}

#[test]
fn dispatch_jr_nz_close_text_display() {
    let mut h = TestHarness::new_headless();
    let addr = dispatch_addr();
    // jr nz = $20
    assert_eq!(rom(&mut h, addr + 4), 0x20, "jr nz opcode after inc a");
    // Verify target is CloseTextDisplay
    let jr_offset = rom(&mut h, addr + 5) as i8;
    let jr_pc = addr + 6; // PC after reading jr instruction
    let target = (jr_pc as i32 + jr_offset as i32) as u16;
    assert_eq!(
        target,
        sym_addr("CloseTextDisplay"),
        "jr nz should target CloseTextDisplay"
    );
}

#[test]
fn dispatch_falls_through_to_after_displaying_text_id() {
    // After the 6-byte dispatch, the next byte should be the start of
    // AfterDisplayingTextID. Verify it's `ld a, [nn]` ($FA).
    let mut h = TestHarness::new_headless();
    let after = sym_addr("AfterDisplayingTextID");
    assert_eq!(
        rom(&mut h, after),
        0xFA,
        "AfterDisplayingTextID should start with ld a, [nn] ($FA)"
    );
}

// ─── Banked: GateUpstairsScript_PrintIfFacingUp ─────────────────────

#[test]
fn gate_script_is_in_bank_12() {
    assert_eq!(
        sym_bank("GateUpstairsScript_PrintIfFacingUp"),
        0x12,
        "GateUpstairsScript_PrintIfFacingUp should be in bank $12"
    );
}

#[test]
fn gate_script_not_facing_up_loads_2() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GateUpstairsScript_PrintIfFacingUp"));
    let up = sym_addr("GateUpstairsScript_PrintIfFacingUp.up");
    // Between func start and .up, find `ld a, 2` ($3E $02).
    // The sequence is: ld a, [nn] (3) / cp n (2) / jr z (2) / ld a, 2 (2) / jr (2)
    // So ld a, 2 is at up - 4 (jr .done is 2 bytes before .up)
    let ld_a_addr = up - 4;
    assert_eq!(
        rom(&mut h, ld_a_addr),
        0x3E,
        "ld a, n opcode in not-facing-up path"
    );
    assert_eq!(
        rom(&mut h, ld_a_addr + 1),
        0x02,
        "immediate value should be 2 (close immediately), not 1 (TRUE)"
    );
}

#[test]
fn gate_script_facing_up_loads_0() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GateUpstairsScript_PrintIfFacingUp"));
    let up = sym_addr("GateUpstairsScript_PrintIfFacingUp.up");
    // .up: call PrintText ($CD) / xor a ($AF)
    assert_eq!(rom(&mut h, up), 0xCD, "call PrintText at .up");
    assert_eq!(
        rom(&mut h, up + 3),
        0xAF,
        "xor a (flag = 0) after call PrintText"
    );
}

// ─── CloseSRAM optimization (xor a) ──────────────────────────────────

#[test]
fn close_sram_uses_xor_a() {
    // CloseSRAM: push af / xor a / ld [rBMODE], a / ld [rRAMG], a / pop af / ret
    // push af = $F5, xor a = $AF
    let mut h = TestHarness::new_headless();
    let close_sram = sym_addr("CloseSRAM");
    assert_eq!(rom(&mut h, close_sram), 0xF5, "push af at CloseSRAM start");
    assert_eq!(
        rom(&mut h, close_sram + 1),
        0xAF,
        "xor a (not ld a, 0) saves 1 byte in HOME"
    );
}
