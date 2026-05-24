//! ROM byte tests for the Safari Zone escape via save-reset fix.
//!
//! Bug: The player can escape the Safari Zone while the step counter is
//! still active by saving inside the Safari Zone, resetting, and walking
//! back to the gate. `SafariZoneGateDefaultScript` only checks south-side
//! coordinates (near the entrance worker), so the player at the north exit
//! (returning from Safari Zone) bypasses the script and walks out freely
//! with `EVENT_IN_SAFARI_ZONE` still set. The step counter then counts
//! down in the overworld, warping the player to "Glitch City" on expiry.
//!
//! Fix: Add a coordinate check at the top of `SafariZoneGateDefaultScript`
//! for the north exit positions `(3,0)` and `(4,0)`. If the player is at
//! those coordinates and `EVENT_IN_SAFARI_ZONE` is set, redirect to
//! `SafariZoneGateLeavingSafariScript` which shows the "leave early?" dialog.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Safari_Zone_exit_glitch>
//!   - <https://bulbapedia.bulbagarden.net/wiki/Glitch_City>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SafariZoneGateDefaultScript"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn safari_zone_gate_default_script_in_bank_1d() {
    assert_eq!(sym_bank("SafariZoneGateDefaultScript"), 0x1D);
}

// ─── THE FIX: coordinate check for north exit ────────────────────────

#[test]
fn check_event_in_safari_zone_at_start() {
    let mut h = rom_harness();
    let base = sym_addr("SafariZoneGateDefaultScript");
    // CheckEvent EVENT_IN_SAFARI_ZONE expands to:
    //   ld a, [wEventFlags + 73] → $FA lo hi (3 bytes)
    //   bit 7, a → $CB $7F (2 bytes)
    // EVENT_IN_SAFARI_ZONE = 591, byte 591/8 = 73, bit 591%8 = 7
    // wEventFlags = $D746, so wEventFlags + 73 = $D746 + $49 = $D78F
    assert_eq!(rom(&mut h, base), 0xFA, "ld a, [nn] opcode");
    assert_eq!(rom(&mut h, base + 1), 0x8F, "wEventFlags+73 low byte ($8F)");
    assert_eq!(
        rom(&mut h, base + 2),
        0xD7,
        "wEventFlags+73 high byte ($D7)"
    );
    assert_eq!(rom(&mut h, base + 3), 0xCB, "bit prefix");
    assert_eq!(rom(&mut h, base + 4), 0x7F, "bit 7, a");
}

#[test]
fn jr_z_skips_safari_check_when_not_in_safari() {
    let mut h = rom_harness();
    let base = sym_addr("SafariZoneGateDefaultScript");
    // jr z, .notReturningFromSafari at offset +5
    assert_eq!(rom(&mut h, base + 5), 0x28, "jr z opcode");
    // The jump target should be .notReturningFromSafari
    let offset = rom(&mut h, base + 6) as i8;
    let target = (base + 7).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("SafariZoneGateDefaultScript.notReturningFromSafari"),
        "jr z should target .notReturningFromSafari"
    );
}

#[test]
fn call_are_player_coords_in_array_for_return_coords() {
    let mut h = rom_harness();
    let base = sym_addr("SafariZoneGateDefaultScript");
    // After CheckEvent (5 bytes) and jr z (2 bytes) = offset +7
    // ld hl, .PlayerReturningFromSafariZoneCoordsArray → $21 lo hi (3 bytes)
    // call ArePlayerCoordsInArray → $CD lo hi (3 bytes)
    assert_eq!(rom(&mut h, base + 7), 0x21, "ld hl, nn opcode");
    let hl_lo = rom(&mut h, base + 8);
    let hl_hi = rom(&mut h, base + 9);
    let hl_addr = u16::from_le_bytes([hl_lo, hl_hi]);
    assert_eq!(
        hl_addr,
        sym_addr("SafariZoneGateDefaultScript.PlayerReturningFromSafariZoneCoordsArray"),
        "ld hl should point to returning coords array"
    );
    assert_eq!(rom(&mut h, base + 10), 0xCD, "call opcode");
    let call_lo = rom(&mut h, base + 11);
    let call_hi = rom(&mut h, base + 12);
    let call_addr = u16::from_le_bytes([call_lo, call_hi]);
    assert_eq!(
        call_addr,
        sym_addr("ArePlayerCoordsInArray"),
        "call target should be ArePlayerCoordsInArray"
    );
}

#[test]
fn sets_leaving_safari_script_on_coord_match() {
    let mut h = rom_harness();
    let base = sym_addr("SafariZoneGateDefaultScript");
    // After ld hl (3) + call (3) + jr nc (2) = offset +15
    // ld a, SCRIPT_SAFARIZONEGATE_LEAVING_SAFARI → $3E $05 (2 bytes)
    // ld [wSafariZoneGateCurScript], a → $EA lo hi (3 bytes)
    assert_eq!(rom(&mut h, base + 15), 0x3E, "ld a, n opcode");
    assert_eq!(
        rom(&mut h, base + 16),
        0x05,
        "SCRIPT_SAFARIZONEGATE_LEAVING_SAFARI = 5"
    );
    assert_eq!(rom(&mut h, base + 17), 0xEA, "ld [nn], a opcode");
}

// ─── Coordinate arrays ──────────────────────────────────────────────

#[test]
fn returning_coords_array_has_north_exit_positions() {
    let mut h = rom_harness();
    let base = sym_addr("SafariZoneGateDefaultScript.PlayerReturningFromSafariZoneCoordsArray");
    // dbmapcoord 3, 0 → Y=0, X=3 (stored as Y, X)
    assert_eq!(rom(&mut h, base), 0x00, "first coord Y=0");
    assert_eq!(rom(&mut h, base + 1), 0x03, "first coord X=3");
    // dbmapcoord 4, 0 → Y=0, X=4
    assert_eq!(rom(&mut h, base + 2), 0x00, "second coord Y=0");
    assert_eq!(rom(&mut h, base + 3), 0x04, "second coord X=4");
    // terminator
    assert_eq!(rom(&mut h, base + 4), 0xFF, "array terminator");
}

#[test]
fn original_worker_coords_still_present() {
    let mut h = rom_harness();
    let base = sym_addr("SafariZoneGateDefaultScript.PlayerNextToSafariZoneWorker1CoordsArray");
    // dbmapcoord 3, 2 → Y=2, X=3
    assert_eq!(rom(&mut h, base), 0x02, "worker coord 1 Y=2");
    assert_eq!(rom(&mut h, base + 1), 0x03, "worker coord 1 X=3");
    // dbmapcoord 4, 2 → Y=2, X=4
    assert_eq!(rom(&mut h, base + 2), 0x02, "worker coord 2 Y=2");
    assert_eq!(rom(&mut h, base + 3), 0x04, "worker coord 2 X=4");
    assert_eq!(rom(&mut h, base + 4), 0xFF, "array terminator");
}

// ─── Negative test ───────────────────────────────────────────────────

#[test]
fn not_returning_falls_through_to_original_worker_check() {
    let mut h = rom_harness();
    let not_returning = sym_addr("SafariZoneGateDefaultScript.notReturningFromSafari");
    // .notReturningFromSafari should start with ld hl, .PlayerNextToSafariZoneWorker1CoordsArray
    assert_eq!(
        rom(&mut h, not_returning),
        0x21,
        "ld hl, nn opcode at .notReturningFromSafari"
    );
    let hl_lo = rom(&mut h, not_returning + 1);
    let hl_hi = rom(&mut h, not_returning + 2);
    let hl_addr = u16::from_le_bytes([hl_lo, hl_hi]);
    assert_eq!(
        hl_addr,
        sym_addr("SafariZoneGateDefaultScript.PlayerNextToSafariZoneWorker1CoordsArray"),
        "should load original worker coords array"
    );
}
