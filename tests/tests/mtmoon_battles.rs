//! ROM byte tests for the Mt. Moon B2F battle-disable softlock fix.
//!
//! Bug: After beating the super nerd in Mt. Moon B2F, `BIT_NO_BATTLES` in
//! `wStatusFlags4` is set while the player is in the fossil area. If the
//! player uses Escape Rope, Dig, or Teleport while in that zone, they
//! leave the map with `BIT_NO_BATTLES` still set, suppressing all random
//! encounters on every subsequent map until they return to Mt. Moon B2F.
//!
//! Fix: Clear `BIT_NO_BATTLES` in `MtMoonB2FResetScripts` so the flag is
//! always cleaned up when the map's scripts are reset (e.g. after a
//! battle loss that warps the player away). The per-frame check in
//! `MtMoonB2F_Script` already handles the flag when the player is on the
//! map, but `MtMoonB2FResetScripts` now provides a safety net.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("MtMoonB2FResetScripts"));
    h
}

// Z80/SM83 opcodes
const LD_HL_IMM: u8 = 0x21; // ld hl, nn
const XOR_A: u8 = 0xAF; // xor a
const RES_4_HL: [u8; 2] = [0xCB, 0xA6]; // res 4, [hl]
const SET_4_HL: [u8; 2] = [0xCB, 0xE6]; // set 4, [hl]

// WRAM address (little-endian)
const W_STATUS_FLAGS4: u16 = 0xD72D;

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn reset_scripts_in_bank_12() {
    assert_eq!(sym_bank("MtMoonB2FResetScripts"), 0x12);
}

#[test]
fn reset_scripts_in_banked_range() {
    let addr = sym_addr("MtMoonB2FResetScripts");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── The fix: res BIT_NO_BATTLES in MtMoonB2FResetScripts ───────────

#[test]
fn reset_scripts_has_ld_hl_wstatusflag4() {
    // `ld hl, wStatusFlags4` ($21 $2D $D7) should appear in MtMoonB2FResetScripts.
    let mut h = banked_harness();
    let start = sym_addr("MtMoonB2FResetScripts");
    let end = sym_addr("MtMoonB2FSetScript");
    let lo = (W_STATUS_FLAGS4 & 0xFF) as u8;
    let hi = (W_STATUS_FLAGS4 >> 8) as u8;
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
    assert!(
        found,
        "ld hl, wStatusFlags4 not found in MtMoonB2FResetScripts"
    );
}

#[test]
fn res_4_hl_follows_ld_hl_statusflags4() {
    // After `ld hl, wStatusFlags4`, expect `res 4, [hl]` ($CB $A6).
    let mut h = banked_harness();
    let start = sym_addr("MtMoonB2FResetScripts");
    let end = sym_addr("MtMoonB2FSetScript");
    let lo = (W_STATUS_FLAGS4 & 0xFF) as u8;
    let hi = (W_STATUS_FLAGS4 >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == lo
            && rom(&mut h, addr + 2) == hi
        {
            assert_eq!(
                rom(&mut h, addr + 3),
                RES_4_HL[0],
                "CB prefix expected after ld hl, wStatusFlags4"
            );
            assert_eq!(
                rom(&mut h, addr + 4),
                RES_4_HL[1],
                "res 4, [hl] ($A6) expected after ld hl, wStatusFlags4"
            );
            return;
        }
    }
    panic!("ld hl, wStatusFlags4 not found in MtMoonB2FResetScripts");
}

#[test]
fn clearing_after_xor_a() {
    // `xor a` ($AF) should precede the `ld hl, wStatusFlags4` (A=0 preserved
    // for the subsequent MtMoonB2FSetScript which uses A as the script index).
    let mut h = banked_harness();
    let start = sym_addr("MtMoonB2FResetScripts");
    let end = sym_addr("MtMoonB2FSetScript");
    let lo = (W_STATUS_FLAGS4 & 0xFF) as u8;
    let hi = (W_STATUS_FLAGS4 >> 8) as u8;
    let mut xor_a_addr: Option<u16> = None;
    let mut ld_hl_addr: Option<u16> = None;
    for addr in start..end {
        if rom(&mut h, addr) == XOR_A {
            xor_a_addr = Some(addr);
        }
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == lo
            && rom(&mut h, addr + 2) == hi
        {
            ld_hl_addr = Some(addr);
        }
    }
    let xa = xor_a_addr.expect("xor a not found in MtMoonB2FResetScripts");
    let lh = ld_hl_addr.expect("ld hl, wStatusFlags4 not found");
    assert!(
        xa < lh,
        "xor a at {:#06X} should precede ld hl, wStatusFlags4 at {:#06X}",
        xa,
        lh
    );
}

#[test]
fn clearing_immediately_before_set_script() {
    // The `res 4, [hl]` should end right at MtMoonB2FSetScript (i.e., the
    // res instruction's last byte + 1 == MtMoonB2FSetScript address).
    let mut h = banked_harness();
    let start = sym_addr("MtMoonB2FResetScripts");
    let end = sym_addr("MtMoonB2FSetScript");
    let lo = (W_STATUS_FLAGS4 & 0xFF) as u8;
    let hi = (W_STATUS_FLAGS4 >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == LD_HL_IMM
            && rom(&mut h, addr + 1) == lo
            && rom(&mut h, addr + 2) == hi
        {
            // ld hl, nn (3 bytes) + res 4, [hl] (2 bytes) = 5 bytes total
            assert_eq!(
                addr + 5,
                end,
                "res 4, [hl] should end at MtMoonB2FSetScript ({:#06X}), but ends at {:#06X}",
                end,
                addr + 5
            );
            return;
        }
    }
    panic!("ld hl, wStatusFlags4 not found in MtMoonB2FResetScripts");
}

// ─── Context: fossil area set/res still present ──────────────────────

#[test]
fn fossil_area_still_sets_no_battles() {
    // The original `set BIT_NO_BATTLES, [hl]` in `MtMoonB2F_Script` should
    // still exist (the per-frame fossil area check that disables battles).
    let mut h = banked_harness();
    let start = sym_addr("MtMoonB2F_Script");
    let end = sym_addr("MtMoonB2FFossilAreaCoords");
    let mut found = false;
    for addr in start..end {
        if rom(&mut h, addr) == SET_4_HL[0] && rom(&mut h, addr + 1) == SET_4_HL[1] {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "set 4, [hl] (BIT_NO_BATTLES) not found in MtMoonB2F_Script"
    );
}

#[test]
fn fossil_area_still_clears_no_battles() {
    // The original `res BIT_NO_BATTLES, [hl]` in `.enable_battles` should
    // still exist (the per-frame fossil area check that re-enables battles).
    let mut h = banked_harness();
    let start = sym_addr("MtMoonB2F_Script");
    let end = sym_addr("MtMoonB2FFossilAreaCoords");
    let mut found = false;
    for addr in start..end {
        if rom(&mut h, addr) == RES_4_HL[0] && rom(&mut h, addr + 1) == RES_4_HL[1] {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "res 4, [hl] (BIT_NO_BATTLES) not found in MtMoonB2F_Script .enable_battles"
    );
}
