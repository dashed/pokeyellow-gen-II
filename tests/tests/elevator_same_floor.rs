//! ROM byte tests for the elevator same-floor fix.
//!
//! Bug: Selecting the floor you're already on in any elevator (Celadon Dept
//! Store, Rocket Hideout, Silph Co.) plays the shake animation and "warps"
//! to the same map, wasting time.
//!
//! Fix: After loading the destination map from `wElevatorWarpMaps`, compare
//! with `wWarpedFromWhichMap` (the floor the player entered from). If they
//! match, `ret z` — skip the elevator flag and warp entirely. +5 bytes in
//! bank $07.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I>

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
fn elevator_in_bank_07() {
    assert_eq!(
        sym_bank("DisplayElevatorFloorMenu"),
        0x07,
        "DisplayElevatorFloorMenu should be in bank $07"
    );
}

// ─── THE FIX: compare destination with source floor ──────────────────

#[test]
fn checks_warped_from_which_map() {
    let mut h = banked_harness("DisplayElevatorFloorMenu");
    let base = sym_addr("DisplayElevatorFloorMenu");
    let update = sym_addr("DisplayElevatorFloorMenu.UpdateWarp");

    // Search for ld a, [wWarpedFromWhichMap] ($FA lo hi)
    let wfwm = sym_addr("wWarpedFromWhichMap");
    let lo = (wfwm & 0xFF) as u8;
    let hi = (wfwm >> 8) as u8;

    let mut found = false;
    for addr in base..update {
        if rom(&mut h, addr) == 0xFA && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld a, [wWarpedFromWhichMap] should be present in DisplayElevatorFloorMenu"
    );
}

#[test]
fn cp_c_and_ret_z_follow_map_load() {
    let mut h = banked_harness("DisplayElevatorFloorMenu");
    let base = sym_addr("DisplayElevatorFloorMenu");
    let update = sym_addr("DisplayElevatorFloorMenu.UpdateWarp");

    let wfwm = sym_addr("wWarpedFromWhichMap");
    let lo = (wfwm & 0xFF) as u8;
    let hi = (wfwm >> 8) as u8;

    for addr in base..update {
        if rom(&mut h, addr) == 0xFA && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            // cp c ($B9) should follow
            assert_eq!(
                rom(&mut h, addr + 3),
                0xB9,
                "Expected cp c ($B9) after ld a, [wWarpedFromWhichMap]"
            );
            // ret z ($C8) should follow
            assert_eq!(
                rom(&mut h, addr + 4),
                0xC8,
                "Expected ret z ($C8) after cp c — skip if same floor"
            );
            return;
        }
    }
    panic!("ld a, [wWarpedFromWhichMap] not found");
}

// ─── Ordering: destination loaded before flag set ────────────────────

#[test]
fn destination_loaded_before_elevator_flag() {
    let mut h = banked_harness("DisplayElevatorFloorMenu");
    let base = sym_addr("DisplayElevatorFloorMenu");
    let update = sym_addr("DisplayElevatorFloorMenu.UpdateWarp");

    let wfwm = sym_addr("wWarpedFromWhichMap");
    let wfwm_lo = (wfwm & 0xFF) as u8;
    let wfwm_hi = (wfwm >> 8) as u8;

    let flags = sym_addr("wCurrentMapScriptFlags");
    let fl_lo = (flags & 0xFF) as u8;
    let fl_hi = (flags >> 8) as u8;

    let mut map_check_pos = None;
    let mut flag_set_pos = None;

    for addr in base..update {
        if rom(&mut h, addr) == 0xFA
            && rom(&mut h, addr + 1) == wfwm_lo
            && rom(&mut h, addr + 2) == wfwm_hi
        {
            map_check_pos = Some(addr);
        }
        // ld hl, wCurrentMapScriptFlags ($21 lo hi)
        if rom(&mut h, addr) == 0x21
            && rom(&mut h, addr + 1) == fl_lo
            && rom(&mut h, addr + 2) == fl_hi
            && addr > base + 20
        {
            // Only count the one after ret c (not the one at the top if any)
            flag_set_pos = Some(addr);
        }
    }

    let check_at = map_check_pos.expect("wWarpedFromWhichMap check not found");
    let flag_at = flag_set_pos.expect("wCurrentMapScriptFlags load not found");

    assert!(
        check_at < flag_at,
        "Same-floor check (${:04X}) must come BEFORE BIT_CUR_MAP_USED_ELEVATOR set (${:04X})",
        check_at,
        flag_at
    );
}

// ─── No old pattern: flag set immediately after ret c ────────────────

#[test]
fn no_flag_set_immediately_after_ret_c() {
    let mut h = banked_harness("DisplayElevatorFloorMenu");
    let base = sym_addr("DisplayElevatorFloorMenu");
    let update = sym_addr("DisplayElevatorFloorMenu.UpdateWarp");

    let flags = sym_addr("wCurrentMapScriptFlags");
    let fl_lo = (flags & 0xFF) as u8;
    let fl_hi = (flags >> 8) as u8;

    // In the old code, ret c ($D8) was immediately followed by
    // ld hl, wCurrentMapScriptFlags ($21 lo hi). The fix moves the
    // destination load between them.
    for addr in base..update {
        if rom(&mut h, addr) == 0xD8
            && rom(&mut h, addr + 1) == 0x21
            && rom(&mut h, addr + 2) == fl_lo
            && rom(&mut h, addr + 3) == fl_hi
        {
            panic!(
                "Old pattern found: ret c immediately followed by ld hl, wCurrentMapScriptFlags \
                 at ${:04X} — destination should be loaded first",
                addr
            );
        }
    }
}
