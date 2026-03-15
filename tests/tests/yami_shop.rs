//! ROM byte tests for the Yami Shop glitch prevention fix.
//!
//! Bug: Glitch items with unterminated names (no `@`/$50 within 20 bytes)
//! cause `GetName` to copy garbage into `wNameBuffer`. When `PlaceString`
//! later displays the name, it reads past the buffer into adjacent WRAM,
//! overflowing and corrupting the Poké Mart item list and other state.
//!
//! Fix: After `CopyData` in `GetName`, force-write `@` at the last byte
//! of `wNameBuffer` (`dec de` / `ld [de], a`). This ensures the buffer is
//! always terminated regardless of the source data. +4 bytes in HOME,
//! offset by 4 tail-call optimizations (`call BankswitchCommon / ret` →
//! `jp BankswitchCommon`). Net: +0 bytes HOME.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Yami_Shop_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Structural test ─────────────────────────────────────────────────

#[test]
fn get_name_in_bank_0() {
    assert_eq!(
        sym_bank("GetName"),
        0x00,
        "GetName should be in HOME (bank 0)"
    );
}

// ─── THE FIX: force '@' terminator after CopyData ────────────────────

#[test]
fn force_terminator_after_copy_data() {
    let mut h = TestHarness::new_headless();
    let base = sym_addr("GetName");
    let got_ptr = sym_addr("GetName.gotPtr");

    // Search for the fix pattern: ld a, "@" ($3E $50) / dec de ($1B) / ld [de], a ($12)
    let mut found = false;
    for addr in base..got_ptr {
        if rom(&mut h, addr) == 0x3E
            && rom(&mut h, addr + 1) == 0x50
            && rom(&mut h, addr + 2) == 0x1B
            && rom(&mut h, addr + 3) == 0x12
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld a, @ ($3E $50) / dec de ($1B) / ld [de], a ($12) \
         should be present in GetName before .gotPtr"
    );
}

#[test]
fn fix_comes_after_copy_data_call() {
    let mut h = TestHarness::new_headless();
    let base = sym_addr("GetName");
    let got_ptr = sym_addr("GetName.gotPtr");

    let copy_data = sym_addr("CopyData");
    let cd_lo = (copy_data & 0xFF) as u8;
    let cd_hi = (copy_data >> 8) as u8;

    let mut call_pos = None;
    let mut fix_pos = None;

    for addr in base..got_ptr {
        // call CopyData → $CD lo hi
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == cd_lo
            && rom(&mut h, addr + 2) == cd_hi
        {
            call_pos = Some(addr);
        }
        // fix pattern: $3E $50 $1B $12
        if rom(&mut h, addr) == 0x3E
            && rom(&mut h, addr + 1) == 0x50
            && rom(&mut h, addr + 2) == 0x1B
            && rom(&mut h, addr + 3) == 0x12
        {
            fix_pos = Some(addr);
        }
    }

    let call_at = call_pos.expect("call CopyData not found in GetName");
    let fix_at = fix_pos.expect("terminator fix not found in GetName");

    assert!(
        fix_at > call_at,
        "Terminator fix (${:04X}) must come after call CopyData (${:04X})",
        fix_at,
        call_at
    );
}

#[test]
fn fix_immediately_before_got_ptr() {
    let mut h = TestHarness::new_headless();
    let got_ptr = sym_addr("GetName.gotPtr");

    // The 4-byte fix should end right at .gotPtr
    // ld a, "@" ($3E $50) at gotPtr-4, dec de ($1B) at gotPtr-2, ld [de], a ($12) at gotPtr-1
    assert_eq!(
        rom(&mut h, got_ptr - 4),
        0x3E,
        "Expected ld a, imm8 ($3E) at .gotPtr-4"
    );
    assert_eq!(
        rom(&mut h, got_ptr - 3),
        0x50,
        "Expected '@' terminator ($50) at .gotPtr-3"
    );
    assert_eq!(
        rom(&mut h, got_ptr - 2),
        0x1B,
        "Expected dec de ($1B) at .gotPtr-2"
    );
    assert_eq!(
        rom(&mut h, got_ptr - 1),
        0x12,
        "Expected ld [de], a ($12) at .gotPtr-1"
    );
}

// ─── Tail-call optimization: jp instead of call/ret ──────────────────

#[test]
fn get_name_ends_with_jp_bankswitch() {
    let mut h = TestHarness::new_headless();
    let got_ptr = sym_addr("GetName.gotPtr");

    // After .gotPtr: stores wUnusedNamePointer, pops, then jp BankswitchCommon
    let bankswitch = sym_addr("BankswitchCommon");
    let bs_lo = (bankswitch & 0xFF) as u8;
    let bs_hi = (bankswitch >> 8) as u8;

    // Scan from .gotPtr for jp BankswitchCommon ($C3 lo hi)
    let mut found = false;
    for addr in got_ptr..got_ptr + 20 {
        if rom(&mut h, addr) == 0xC3
            && rom(&mut h, addr + 1) == bs_lo
            && rom(&mut h, addr + 2) == bs_hi
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "GetName should end with jp BankswitchCommon (tail-call optimization)"
    );
}

// ─── Cross-reference: GetMonName already terminates ──────────────────

#[test]
fn get_mon_name_also_terminates_buffer() {
    // GetMonName explicitly writes '@' at wNameBuffer + NAME_LENGTH - 1
    // Verify this pattern exists as a cross-reference
    let mut h = TestHarness::new_headless();
    let base = sym_addr("GetMonName");
    let end = base + 50;

    // Look for ld [hl], '@' → $36 $50 (ld [hl], imm8)
    let mut found = false;
    for addr in base..end {
        if rom(&mut h, addr) == 0x36 && rom(&mut h, addr + 1) == 0x50 {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "GetMonName should write '@' terminator (ld [hl], $50)"
    );
}
