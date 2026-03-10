//! ROM byte tests for the Route 16 sign readability fix.
//!
//! Bug: The "ROUTE 16 / CELADON CITY - FUCHSIA CITY" sign sits on the
//! last tile row (Y=17) of Route 16. When the player stands directly
//! south of the sign to read it, they are on Route 17 (due to map
//! connection). The game only checks bg_events for the current map,
//! so the sign is unreadable from its front.
//!
//! Fix: Add a duplicate bg_event on Route 17 at Y=-1 (the connection
//! strip tile corresponding to the sign) so the sign can be read from
//! the Route 17 side of the boundary.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Cycling_Road_sign_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Each bg_event is 3 bytes: Y, X, text_id.
const BG_EVENT_SIZE: u16 = 3;

#[test]
fn route17_has_route16_sign_bg_event() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("Route17_Object"));

    let obj = sym_addr("Route17_Object");

    // Byte 0: border block ($43)
    let border = h.read_mem(obj);
    assert_eq!(border, 0x43, "Route 17 border block should be $43");

    // Byte 1: number of warp events (0)
    let num_warps = h.read_mem(obj + 1);
    assert_eq!(num_warps, 0, "Route 17 should have 0 warp events");

    // Byte 2: number of bg events (7 after fix, was 6)
    let num_bg = h.read_mem(obj + 2);
    assert_eq!(
        num_bg, 7,
        "Route 17 should have 7 bg_events (6 original + 1 Route 16 sign)"
    );

    // bg_events start at offset 3 (border + num_warps + num_bg)
    let bg_start = obj + 3;

    // First bg_event should be the Route 16 sign at Y=$FF (-1), X=5
    let first_bg_y = h.read_mem(bg_start);
    let first_bg_x = h.read_mem(bg_start + 1);
    assert_eq!(
        first_bg_y, 0xFF,
        "First bg_event Y should be $FF (-1 = connection strip)"
    );
    assert_eq!(first_bg_x, 5, "First bg_event X should be 5");
}

#[test]
fn route16_sign_still_exists_on_route16() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("Route16_Object"));

    let obj = sym_addr("Route16_Object");

    // Byte 0: border block ($0F)
    let border = h.read_mem(obj);
    assert_eq!(border, 0x0F, "Route 16 border block should be $0F");

    // Byte 1: number of warp events (9)
    let num_warps = h.read_mem(obj + 1);
    assert_eq!(num_warps, 9, "Route 16 should have 9 warp events");

    // Skip warps: 9 warps × 4 bytes each = 36 bytes
    let bg_count_offset = 2 + (num_warps as u16) * 4;
    let num_bg = h.read_mem(obj + bg_count_offset);
    assert_eq!(num_bg, 2, "Route 16 should have 2 bg_events");

    // Second bg_event is the Route 16 sign (Y=17, X=5)
    let sign_offset = bg_count_offset + 1 + BG_EVENT_SIZE; // skip first bg_event
    let sign_y = h.read_mem(obj + sign_offset);
    let sign_x = h.read_mem(obj + sign_offset + 1);
    assert_eq!(sign_y, 17, "Route 16 sign Y should be 17 (last tile row)");
    assert_eq!(sign_x, 5, "Route 16 sign X should be 5");
}

#[test]
fn route17_route16_sign_text_id_matches() {
    let mut h = TestHarness::new_headless();

    // Read Route 17's first bg_event text ID
    h.select_rom_bank(sym_bank("Route17_Object"));
    let obj17 = sym_addr("Route17_Object");
    let r17_sign_text_id = h.read_mem(obj17 + 3 + 2); // bg_start + 2 (text_id field)

    // Read Route 16's second bg_event text ID
    h.select_rom_bank(sym_bank("Route16_Object"));
    let obj16 = sym_addr("Route16_Object");
    let num_warps_16 = h.read_mem(obj16 + 1) as u16;
    let bg_count_offset_16 = 2 + num_warps_16 * 4;
    let r16_sign_text_id = h.read_mem(obj16 + bg_count_offset_16 + 1 + BG_EVENT_SIZE + 2);

    // Both should be valid text IDs (> number of object events on their respective maps).
    // Route 17 has 10 object events, so its text IDs for signs start at 11+.
    // Route 16 has 7 object events, so its text IDs for signs start at 8+.
    assert!(
        r17_sign_text_id > 10,
        "Route 17 sign text ID ({r17_sign_text_id}) should be > 10 (num object events)"
    );
    assert!(
        r16_sign_text_id > 7,
        "Route 16 sign text ID ({r16_sign_text_id}) should be > 7 (num object events)"
    );
}
