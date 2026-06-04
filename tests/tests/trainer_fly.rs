//! ROM byte tests for the Trainer Fly / Mew glitch prevention fix.
//!
//! Bug: When a trainer spots the player, `BIT_SEEN_BY_TRAINER` is set in
//! `wMiscFlags` ($CD60). If the player warps away (Fly/Teleport/Dig/Escape
//! Rope) before the battle starts, this flag persists across map transitions
//! because `ClearVariablesOnEnterMap` never cleared it. The stale flag causes
//! the game to enter a bad state on the destination map, enabling the
//! infamous "Trainer Fly" exploit (also known as the Mew glitch).
//!
//! Fix: Add `ld hl, wMiscFlags` / `res BIT_SEEN_BY_TRAINER, [hl]` to
//! `ClearVariablesOnEnterMap` before `ret`. +5 bytes in bank $03.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/Mew_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn clear_variables_in_bank_03() {
    assert_eq!(sym_bank("ClearVariablesOnEnterMap"), 0x03);
}

#[test]
fn clear_variables_loads_misc_flags_addr() {
    let mut h = banked_harness("ClearVariablesOnEnterMap");
    let base = sym_addr("ClearVariablesOnEnterMap");
    // Scan for `ld hl, wMiscFlags` → $21 $60 $CD
    let end = base + 60; // function is ~26 bytes, generous window
    let mut found = false;
    for addr in base..end {
        if rom(&mut h, addr) == 0x21
            && rom(&mut h, addr + 1) == 0x60
            && rom(&mut h, addr + 2) == 0xCD
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld hl, wMiscFlags ($21 $60 $CD) not found in ClearVariablesOnEnterMap"
    );
}

#[test]
fn clear_variables_has_res_seen_by_trainer() {
    let mut h = banked_harness("ClearVariablesOnEnterMap");
    let base = sym_addr("ClearVariablesOnEnterMap");
    // Scan for `res 0, [hl]` → $CB $86 (res BIT_SEEN_BY_TRAINER, [hl])
    let end = base + 60;
    let mut found = false;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCB && rom(&mut h, addr + 1) == 0x86 {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "res BIT_SEEN_BY_TRAINER, [hl] ($CB $86) not found in ClearVariablesOnEnterMap"
    );
}

#[test]
fn res_comes_after_fill_memory_call() {
    let mut h = banked_harness("ClearVariablesOnEnterMap");
    let base = sym_addr("ClearVariablesOnEnterMap");
    let end = base + 60;
    // Find `call FillMemory` → $CD lo hi
    let fill_addr_val = sym_addr("FillMemory");
    let fill_lo = (fill_addr_val & 0xFF) as u8;
    let fill_hi = (fill_addr_val >> 8) as u8;
    let mut call_pos = None;
    let mut res_pos = None;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == fill_lo
            && rom(&mut h, addr + 2) == fill_hi
        {
            call_pos = Some(addr);
        }
        if rom(&mut h, addr) == 0xCB && rom(&mut h, addr + 1) == 0x86 {
            res_pos = Some(addr);
        }
    }
    assert!(call_pos.is_some(), "call FillMemory not found");
    assert!(res_pos.is_some(), "res BIT_SEEN_BY_TRAINER not found");
    assert!(
        res_pos.unwrap() > call_pos.unwrap(),
        "res BIT_SEEN_BY_TRAINER ({:#06X}) must come after call FillMemory ({:#06X})",
        res_pos.unwrap(),
        call_pos.unwrap()
    );
}

#[test]
fn res_immediately_before_ret() {
    let mut h = banked_harness("ClearVariablesOnEnterMap");
    let base = sym_addr("ClearVariablesOnEnterMap");
    let end = base + 60;
    // Find `res 0, [hl]` ($CB $86) and verify `ret` ($C9) follows
    for addr in base..end {
        if rom(&mut h, addr) == 0xCB && rom(&mut h, addr + 1) == 0x86 {
            assert_eq!(
                rom(&mut h, addr + 2),
                0xC9,
                "ret ($C9) should immediately follow res BIT_SEEN_BY_TRAINER at {:#06X}",
                addr + 2
            );
            return;
        }
    }
    panic!("res BIT_SEEN_BY_TRAINER not found in ClearVariablesOnEnterMap");
}

#[test]
fn misc_flags_outside_fill_memory_range() {
    // wMiscFlags ($CD60) must be outside the FillMemory range
    // (wWhichTrade=$CD3D .. wStandingOnWarpPadOrHole=$CD5B)
    // to confirm BIT_SEEN_BY_TRAINER was never cleared before this fix
    let misc_flags = sym_addr("wMiscFlags");
    let range_end = sym_addr("wStandingOnWarpPadOrHole");
    assert!(
        misc_flags >= range_end,
        "wMiscFlags ({:#06X}) should be at or past wStandingOnWarpPadOrHole ({:#06X})",
        misc_flags,
        range_end
    );
}

// ─── Cross-reference tests ───────────────────────────────────────────

#[test]
fn end_trainer_battle_clears_seen_by_trainer() {
    // Verify EndTrainerBattle (HOME bank) also clears BIT_SEEN_BY_TRAINER
    let mut h = TestHarness::new_headless();
    let base = sym_addr("EndTrainerBattle");
    let end = base + 40;
    let mut found = false;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCB && rom(&mut h, addr + 1) == 0x86 {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "EndTrainerBattle should also clear BIT_SEEN_BY_TRAINER (res 0, [hl])"
    );
}

#[test]
fn trainer_engage_sets_seen_by_trainer() {
    // Verify TrainerEngage.engage sets BIT_SEEN_BY_TRAINER
    let mut h = banked_harness("TrainerEngage");
    let base = sym_addr("TrainerEngage.engage");
    let end = base + 20;
    // `set 0, [hl]` → $CB $C6
    let mut found = false;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCB && rom(&mut h, addr + 1) == 0xC6 {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "TrainerEngage.engage should set BIT_SEEN_BY_TRAINER (set 0, [hl])"
    );
}
