//! ROM byte tests for the Walking Through Walls glitch prevention fix.
//!
//! Bug: When the Safari Zone step counter expires during a ledge jump,
//! `SafariZoneGameOver` warps the player to the Safari Zone Gate via
//! `WarpFound2` → `EnterMap`. However, `ClearVariablesOnEnterMap` never
//! cleared `BIT_LEDGE_OR_FISHING` in `wMovementFlags` or zeroed
//! `wSimulatedJoypadStatesIndex`. `CollisionCheckOnLand` checks these
//! first and unconditionally skips all tile collision when either is set,
//! letting the player walk through walls on the destination map.
//!
//! Fix: Add `res BIT_LEDGE_OR_FISHING, [hl]` on `wMovementFlags` and
//! `ld [wSimulatedJoypadStatesIndex], a` (a=0) to `ClearVariablesOnEnterMap`
//! after the `FillMemory` call. +8 bytes in bank $03.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/Walking_through_walls>
//!   - <https://glitchcity.wiki/wiki/Walk_through_walls_trick_(ledge_method)>
//!   - <https://glitchcity.wiki/wiki/Walk_through_walls_trick_(museum_guy_method)>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// ─── Structural test ─────────────────────────────────────────────────

#[test]
fn clear_variables_in_bank_03() {
    assert_eq!(
        sym_bank("ClearVariablesOnEnterMap"),
        0x03,
        "ClearVariablesOnEnterMap should be in bank $03"
    );
}

// ─── THE FIX: res BIT_LEDGE_OR_FISHING in wMovementFlags ────────────

#[test]
fn clears_ledge_or_fishing_flag() {
    let mut h = banked_harness("ClearVariablesOnEnterMap");
    let base = sym_addr("ClearVariablesOnEnterMap");
    let end = base + 60;

    // Scan for `ld hl, wMovementFlags` → $21 lo hi
    let movement_flags = sym_addr("wMovementFlags");
    let mf_lo = (movement_flags & 0xFF) as u8;
    let mf_hi = (movement_flags >> 8) as u8;

    let mut ld_hl_pos = None;
    for addr in base..end {
        if rom(&mut h, addr) == 0x21
            && rom(&mut h, addr + 1) == mf_lo
            && rom(&mut h, addr + 2) == mf_hi
        {
            ld_hl_pos = Some(addr);
            break;
        }
    }
    let ld_at =
        ld_hl_pos.expect("ld hl, wMovementFlags should be present in ClearVariablesOnEnterMap");

    // `res 6, [hl]` (BIT_LEDGE_OR_FISHING = bit 6) → $CB $B6
    assert_eq!(
        rom(&mut h, ld_at + 3),
        0xCB,
        "Expected CB prefix for res instruction at ${:04X}",
        ld_at + 3
    );
    assert_eq!(
        rom(&mut h, ld_at + 4),
        0xB6,
        "Expected B6 (res 6, [hl]) — BIT_LEDGE_OR_FISHING at ${:04X}",
        ld_at + 4
    );
}

// ─── THE FIX: zero wSimulatedJoypadStatesIndex ──────────────────────

#[test]
fn clears_simulated_joypad_states_index() {
    let mut h = banked_harness("ClearVariablesOnEnterMap");
    let base = sym_addr("ClearVariablesOnEnterMap");
    let end = base + 60;

    // Scan for `ld [wSimulatedJoypadStatesIndex], a` → $EA lo hi
    let sjsi = sym_addr("wSimulatedJoypadStatesIndex");
    let sj_lo = (sjsi & 0xFF) as u8;
    let sj_hi = (sjsi >> 8) as u8;

    let mut found = false;
    for addr in base..end {
        if rom(&mut h, addr) == 0xEA
            && rom(&mut h, addr + 1) == sj_lo
            && rom(&mut h, addr + 2) == sj_hi
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld [wSimulatedJoypadStatesIndex], a ($EA ${:02X} ${:02X}) \
         should be in ClearVariablesOnEnterMap",
        sj_lo, sj_hi
    );
}

// ─── Ordering: fix comes after FillMemory call ──────────────────────

#[test]
fn fix_comes_after_fill_memory() {
    let mut h = banked_harness("ClearVariablesOnEnterMap");
    let base = sym_addr("ClearVariablesOnEnterMap");
    let end = base + 60;

    let fill_addr = sym_addr("FillMemory");
    let fill_lo = (fill_addr & 0xFF) as u8;
    let fill_hi = (fill_addr >> 8) as u8;

    let movement_flags = sym_addr("wMovementFlags");
    let mf_lo = (movement_flags & 0xFF) as u8;
    let mf_hi = (movement_flags >> 8) as u8;

    let mut call_pos = None;
    let mut fix_pos = None;

    for addr in base..end {
        // call FillMemory → $CD lo hi
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == fill_lo
            && rom(&mut h, addr + 2) == fill_hi
        {
            call_pos = Some(addr);
        }
        // ld hl, wMovementFlags → $21 lo hi
        if rom(&mut h, addr) == 0x21
            && rom(&mut h, addr + 1) == mf_lo
            && rom(&mut h, addr + 2) == mf_hi
        {
            fix_pos = Some(addr);
        }
    }

    let call_at = call_pos.expect("call FillMemory not found");
    let fix_at = fix_pos.expect("ld hl, wMovementFlags not found");

    assert!(
        fix_at > call_at,
        "ld hl, wMovementFlags (${:04X}) must come after call FillMemory (${:04X})",
        fix_at,
        call_at
    );
}

// ─── Cross-reference: CollisionCheckOnLand uses BIT_LEDGE_OR_FISHING ─

#[test]
fn collision_check_on_land_tests_ledge_flag() {
    // Verify CollisionCheckOnLand checks BIT_LEDGE_OR_FISHING
    // This is the code path our fix protects: `bit 6, a` → $CB $77
    let mut h = TestHarness::new_headless();
    let base = sym_addr("CollisionCheckOnLand");
    let end = base + 20;

    let mut found = false;
    for addr in base..end {
        // bit 6, a → $CB $77
        if rom(&mut h, addr) == 0xCB && rom(&mut h, addr + 1) == 0x77 {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "CollisionCheckOnLand should test BIT_LEDGE_OR_FISHING (bit 6, a = $CB $77)"
    );
}

// ─── wMovementFlags is outside FillMemory range ─────────────────────

#[test]
fn movement_flags_outside_fill_memory_range() {
    // wMovementFlags must be outside the FillMemory range to confirm
    // BIT_LEDGE_OR_FISHING was never cleared before this fix
    let movement_flags = sym_addr("wMovementFlags");
    let range_end = sym_addr("wStandingOnWarpPadOrHole");
    assert!(
        movement_flags >= range_end,
        "wMovementFlags (${:04X}) should be outside FillMemory range \
         (ends at wStandingOnWarpPadOrHole ${:04X})",
        movement_flags,
        range_end
    );
}
