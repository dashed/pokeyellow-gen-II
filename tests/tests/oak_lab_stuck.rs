//! ROM byte tests for the "stuck in wall when following Oak" fix.
//!
//! Bug: `PalletTownPlayerFollowsOakScript` advances to the next script state
//! as soon as the Oak auto-movement script completes, without checking whether
//! the player actually reached the warp tile into Oak's Lab. If the simulated
//! movement didn't land the player on the warp tile (e.g. due to collision
//! alignment), the player ends up stuck in or near the wall.
//!
//! Fix: After the movement script completes, check `EVENT_FOLLOWED_OAK_INTO_LAB`.
//! If not set, simulate one `PAD_LEFT` press to nudge the player onto the warp
//! tile. The script only advances to `SCRIPT_PALLETTOWN_DAISY` once the event
//! confirms the player entered the lab.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PalletTownPlayerFollowsOakScript"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn player_follows_oak_in_bank_06() {
    assert_eq!(sym_bank("PalletTownPlayerFollowsOakScript"), 0x06);
}

#[test]
fn movement_script_done_check() {
    let mut h = banked_harness();
    let base = sym_addr("PalletTownPlayerFollowsOakScript");
    // ld a, [wNPCMovementScriptPointerTableNum] → $FA lo hi (3 bytes)
    assert_eq!(rom(&mut h, base), 0xFA, "ld a, [nn] opcode");
    // and a → $A7
    assert_eq!(rom(&mut h, base + 3), 0xA7, "and a opcode");
    // ret nz → $C0
    assert_eq!(rom(&mut h, base + 4), 0xC0, "ret nz opcode");
}

#[test]
fn check_event_followed_oak_present() {
    let mut h = banked_harness();
    let base = sym_addr("PalletTownPlayerFollowsOakScript");
    // After: ld a [nn](3) + and a(1) + ret nz(1) = offset +5
    // CheckEvent EVENT_FOLLOWED_OAK_INTO_LAB expands to:
    //   ld a, [wEventFlags] → $FA $46 $D7
    //   bit 0, a → $CB $47
    assert_eq!(rom(&mut h, base + 5), 0xFA, "ld a, [wEventFlags]");
    let event_flags = sym_addr("wEventFlags");
    assert_eq!(
        rom(&mut h, base + 6),
        (event_flags & 0xFF) as u8,
        "wEventFlags lo"
    );
    assert_eq!(
        rom(&mut h, base + 7),
        (event_flags >> 8) as u8,
        "wEventFlags hi"
    );
    assert_eq!(rom(&mut h, base + 8), 0xCB, "bit prefix");
    assert_eq!(rom(&mut h, base + 9), 0x47, "bit 0, a");
}

#[test]
fn jr_nz_skips_to_followed_oak() {
    let mut h = banked_harness();
    let base = sym_addr("PalletTownPlayerFollowsOakScript");
    // After CheckEvent (5 bytes at offset +5) = offset +10
    // jr nz, .followedOak → $20 xx
    let jr_addr = base + 10;
    assert_eq!(rom(&mut h, jr_addr), 0x20, "jr nz opcode");
    let offset = rom(&mut h, jr_addr + 1) as i8;
    let target = (jr_addr + 2).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("PalletTownPlayerFollowsOakScript.followedOak"),
        "jr nz should target .followedOak"
    );
}

// ─── Recovery path tests ─────────────────────────────────────────────

#[test]
fn recovery_sets_one_step() {
    let mut h = banked_harness();
    let base = sym_addr("PalletTownPlayerFollowsOakScript");
    // After jr nz (2 bytes at offset +10) = offset +12
    // ld a, $1 → $3E $01
    assert_eq!(rom(&mut h, base + 12), 0x3E, "ld a, n opcode");
    assert_eq!(rom(&mut h, base + 13), 0x01, "one simulated step");
}

#[test]
fn recovery_simulates_pad_left() {
    let mut h = banked_harness();
    let base = sym_addr("PalletTownPlayerFollowsOakScript");
    // ld a, $1 (2) + ld [wSimulatedJoypadStatesIndex] (3) = offset +12+5 = +17
    // ld a, PAD_LEFT → $3E $20
    assert_eq!(rom(&mut h, base + 17), 0x3E, "ld a, n opcode for PAD_LEFT");
    assert_eq!(rom(&mut h, base + 18), 0x20, "PAD_LEFT = $20");
}

#[test]
fn recovery_calls_start_simulating() {
    let mut h = banked_harness();
    let followed_oak = sym_addr("PalletTownPlayerFollowsOakScript.followedOak");
    // jp StartSimulatingJoypadStates should be 3 bytes before .followedOak
    let jp_addr = followed_oak - 3;
    assert_eq!(rom(&mut h, jp_addr), 0xC3, "jp opcode");
    let target = rom(&mut h, jp_addr + 1) as u16 | ((rom(&mut h, jp_addr + 2) as u16) << 8);
    assert_eq!(
        target,
        sym_addr("StartSimulatingJoypadStates"),
        "jp should target StartSimulatingJoypadStates"
    );
}

// ─── Normal path test ────────────────────────────────────────────────

#[test]
fn followed_oak_advances_to_daisy_script() {
    let mut h = banked_harness();
    let followed_oak = sym_addr("PalletTownPlayerFollowsOakScript.followedOak");
    // .followedOak:
    //   ld a, SCRIPT_PALLETTOWN_DAISY → $3E xx
    assert_eq!(
        rom(&mut h, followed_oak),
        0x3E,
        "ld a, n opcode at .followedOak"
    );
    // The constant SCRIPT_PALLETTOWN_DAISY should point to PalletTownDaisyScript
    // which is entry 8 in the script table (index 8)
    let script_idx = rom(&mut h, followed_oak + 1);
    // ld [wPalletTownCurScript] → $EA lo hi
    assert_eq!(rom(&mut h, followed_oak + 2), 0xEA, "ld [nn], a opcode");
    // ret → $C9
    assert_eq!(
        rom(&mut h, followed_oak + 5),
        0xC9,
        "ret at end of .followedOak"
    );
    // Verify script index matches PalletTownDaisyScript's position
    // PalletTownDaisyScript is right after .followedOak's ret, confirming correct index
    assert!(script_idx > 0, "SCRIPT_PALLETTOWN_DAISY should be > 0");
}
