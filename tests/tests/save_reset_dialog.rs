//! ROM byte tests for the save dialog A-button hold fix.
//!
//! Bug: After saving, `StartMenu_SaveReset` ends with `jp HoldTextDisplayOpen`,
//! which keeps the "game saved" dialog on screen as long as the player holds A.
//! This is inconsistent with the other start menu items, which all close
//! immediately via `CloseStartMenu`.
//!
//! Fix: Change `jp HoldTextDisplayOpen` → `jp CloseStartMenu` so the save
//! dialog dismisses immediately like every other start menu action.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("StartMenu_SaveReset"));
    h
}

// Z80/SM83 opcodes
const JP: u8 = 0xC3; // jp nn
const CALL: u8 = 0xCD; // call nn
const BIT_6_A: [u8; 2] = [0xCB, 0x77]; // bit 6, a

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn save_reset_in_bank_4() {
    assert_eq!(sym_bank("StartMenu_SaveReset"), 0x04);
}

#[test]
fn save_reset_in_banked_range() {
    let addr = sym_addr("StartMenu_SaveReset");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── The fix: jp CloseStartMenu ──────────────────────────────────────

#[test]
fn jp_close_start_menu_present() {
    // `jp CloseStartMenu` ($C3 lo hi) should appear in StartMenu_SaveReset.
    let mut h = banked_harness();
    let start = sym_addr("StartMenu_SaveReset");
    let end = sym_addr("StartMenu_Option");
    let target = sym_addr("CloseStartMenu");
    let lo = (target & 0xFF) as u8;
    let hi = (target >> 8) as u8;
    let mut found = false;
    for addr in start..end {
        if rom(&mut h, addr) == JP && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            found = true;
            break;
        }
    }
    assert!(found, "jp CloseStartMenu not found in StartMenu_SaveReset");
}

#[test]
fn jp_close_start_menu_at_end() {
    // `jp CloseStartMenu` should be the last instruction — its 3 bytes end
    // exactly at StartMenu_Option.
    let mut h = banked_harness();
    let end = sym_addr("StartMenu_Option");
    let target = sym_addr("CloseStartMenu");
    let lo = (target & 0xFF) as u8;
    let hi = (target >> 8) as u8;
    // jp nn is 3 bytes, so it starts at end - 3
    let jp_addr = end - 3;
    assert_eq!(rom(&mut h, jp_addr), JP, "expected jp opcode at end - 3");
    assert_eq!(
        rom(&mut h, jp_addr + 1),
        lo,
        "expected CloseStartMenu lo byte"
    );
    assert_eq!(
        rom(&mut h, jp_addr + 2),
        hi,
        "expected CloseStartMenu hi byte"
    );
}

// ─── Regression: old bug pattern absent ──────────────────────────────

#[test]
fn no_jp_hold_text_display_open() {
    // The old buggy `jp HoldTextDisplayOpen` must NOT appear in the function.
    let mut h = banked_harness();
    let start = sym_addr("StartMenu_SaveReset");
    let end = sym_addr("StartMenu_Option");
    let target = sym_addr("HoldTextDisplayOpen");
    let lo = (target & 0xFF) as u8;
    let hi = (target >> 8) as u8;
    for addr in start..end {
        if rom(&mut h, addr) == JP && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            panic!(
                "jp HoldTextDisplayOpen still present at {:#06X} — bug not fixed",
                addr
            );
        }
    }
}

// ─── Context: surrounding instructions preserved ─────────────────────

#[test]
fn call_load_screen_tiles_present() {
    // `call LoadScreenTilesFromBuffer2` should still be in StartMenu_SaveReset.
    let mut h = banked_harness();
    let start = sym_addr("StartMenu_SaveReset");
    let end = sym_addr("StartMenu_Option");
    let target = sym_addr("LoadScreenTilesFromBuffer2");
    let lo = (target & 0xFF) as u8;
    let hi = (target >> 8) as u8;
    let mut found = false;
    for addr in start..end {
        if rom(&mut h, addr) == CALL && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "call LoadScreenTilesFromBuffer2 not found in StartMenu_SaveReset"
    );
}

#[test]
fn close_start_menu_after_load_screen_tiles() {
    // `jp CloseStartMenu` should come AFTER `call LoadScreenTilesFromBuffer2`.
    let mut h = banked_harness();
    let start = sym_addr("StartMenu_SaveReset");
    let end = sym_addr("StartMenu_Option");
    let load_target = sym_addr("LoadScreenTilesFromBuffer2");
    let load_lo = (load_target & 0xFF) as u8;
    let load_hi = (load_target >> 8) as u8;
    let close_target = sym_addr("CloseStartMenu");
    let close_lo = (close_target & 0xFF) as u8;
    let close_hi = (close_target >> 8) as u8;
    let mut load_addr: Option<u16> = None;
    let mut close_addr: Option<u16> = None;
    for addr in start..end {
        if rom(&mut h, addr) == CALL
            && rom(&mut h, addr + 1) == load_lo
            && rom(&mut h, addr + 2) == load_hi
        {
            load_addr = Some(addr);
        }
        if rom(&mut h, addr) == JP
            && rom(&mut h, addr + 1) == close_lo
            && rom(&mut h, addr + 2) == close_hi
        {
            close_addr = Some(addr);
        }
    }
    let la = load_addr.expect("call LoadScreenTilesFromBuffer2 not found");
    let ca = close_addr.expect("jp CloseStartMenu not found");
    assert!(
        ca > la,
        "jp CloseStartMenu at {:#06X} should come after call LoadScreenTilesFromBuffer2 at {:#06X}",
        ca,
        la
    );
}

#[test]
fn link_connected_check_preserved() {
    // `bit 6, a` ($CB $77) — the BIT_LINK_CONNECTED check — must still be
    // present in StartMenu_SaveReset (safety regression test).
    let mut h = banked_harness();
    let start = sym_addr("StartMenu_SaveReset");
    let end = sym_addr("StartMenu_Option");
    let mut found = false;
    for addr in start..end {
        if rom(&mut h, addr) == BIT_6_A[0] && rom(&mut h, addr + 1) == BIT_6_A[1] {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "bit 6, a (BIT_LINK_CONNECTED check) not found in StartMenu_SaveReset"
    );
}
