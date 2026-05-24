//! ROM byte tests for the museum fossil cry / binoculars cry fix.
//!
//! Bug 1: `DisplayMonFrontSpriteInBox` (used by museum fossils, Route 15
//! binoculars, and Fan Club pictures) never played the Pokémon's cry.
//!
//! Bug 2: The Route 15 binoculars called `PlayCry` *before*
//! `DisplayMonFrontSpriteInBox`, so the cry played while VRAM was being
//! loaded, causing Articuno's cry to distort.
//!
//! Fix: Add `call PlayCry` inside `DisplayMonFrontSpriteInBox` after
//! `AnimateSendingOutMon`, with `cp FOSSIL_KABUTOPS` / `cp FOSSIL_AERODACTYL`
//! checks to skip the cry for museum fossils. Remove the standalone
//! `call PlayCry` from `Route15GateLeftBinoculars`.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("DisplayMonFrontSpriteInBox"));
    h
}

// FOSSIL_KABUTOPS = $B6, FOSSIL_AERODACTYL = $B7
const FOSSIL_KABUTOPS: u8 = 0xB6;
const FOSSIL_AERODACTYL: u8 = 0xB7;

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn display_mon_front_sprite_in_box_in_bank_17() {
    assert_eq!(sym_bank("DisplayMonFrontSpriteInBox"), 0x17);
}

#[test]
fn display_mon_front_sprite_in_box_in_banked_range() {
    let addr = sym_addr("DisplayMonFrontSpriteInBox");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── Core fix: fossil checks + PlayCry inside DisplayMonFrontSpriteInBox ──

#[test]
fn cp_fossil_kabutops_present() {
    // Between DisplayMonFrontSpriteInBox and .skipCry, expect
    // cp FOSSIL_KABUTOPS: $FE $B6
    let mut h = banked_harness();
    let start = sym_addr("DisplayMonFrontSpriteInBox");
    let skip = sym_addr("DisplayMonFrontSpriteInBox.skipCry");
    let mut found = false;
    for addr in start..skip {
        if rom(&mut h, addr) == 0xFE && rom(&mut h, addr + 1) == FOSSIL_KABUTOPS {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "cp FOSSIL_KABUTOPS ($FE $B6) not found before .skipCry"
    );
}

#[test]
fn cp_fossil_aerodactyl_present() {
    // Between DisplayMonFrontSpriteInBox and .skipCry, expect
    // cp FOSSIL_AERODACTYL: $FE $B7
    let mut h = banked_harness();
    let start = sym_addr("DisplayMonFrontSpriteInBox");
    let skip = sym_addr("DisplayMonFrontSpriteInBox.skipCry");
    let mut found = false;
    for addr in start..skip {
        if rom(&mut h, addr) == 0xFE && rom(&mut h, addr + 1) == FOSSIL_AERODACTYL {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "cp FOSSIL_AERODACTYL ($FE $B7) not found before .skipCry"
    );
}

#[test]
fn jr_z_follows_each_fossil_compare() {
    // After each cp FOSSIL_xxx ($FE xx), expect jr z ($28) to .skipCry
    let mut h = banked_harness();
    let start = sym_addr("DisplayMonFrontSpriteInBox");
    let skip = sym_addr("DisplayMonFrontSpriteInBox.skipCry");
    let mut jr_count = 0;
    for addr in start..skip.saturating_sub(1) {
        if rom(&mut h, addr) == 0xFE
            && (rom(&mut h, addr + 1) == FOSSIL_KABUTOPS
                || rom(&mut h, addr + 1) == FOSSIL_AERODACTYL)
        {
            assert_eq!(
                rom(&mut h, addr + 2),
                0x28,
                "jr z expected after cp at {:#06X}",
                addr
            );
            jr_count += 1;
        }
    }
    assert_eq!(jr_count, 2, "expected 2 jr z instructions (one per fossil)");
}

#[test]
fn call_play_cry_before_skip_cry() {
    // Between the fossil checks and .skipCry, expect call PlayCry ($CD lo hi)
    let mut h = banked_harness();
    let start = sym_addr("DisplayMonFrontSpriteInBox");
    let skip = sym_addr("DisplayMonFrontSpriteInBox.skipCry");
    let play_cry = sym_addr("PlayCry");
    let lo = (play_cry & 0xFF) as u8;
    let hi = (play_cry >> 8) as u8;
    let mut found = false;
    for addr in start..skip {
        if rom(&mut h, addr) == 0xCD && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "call PlayCry not found between fossil checks and .skipCry"
    );
}

// ─── Binoculars: PlayCry removed ─────────────────────────────────────

#[test]
fn binoculars_no_standalone_play_cry() {
    // Route15GateLeftBinoculars should NOT contain call PlayCry.
    // The cry now plays inside DisplayMonFrontSpriteInBox.
    let mut h = banked_harness();
    let bino_start = sym_addr("Route15GateLeftBinoculars");
    let bino_text = sym_addr("Route15UpstairsBinocularsText");
    let play_cry = sym_addr("PlayCry");
    let lo = (play_cry & 0xFF) as u8;
    let hi = (play_cry >> 8) as u8;
    for addr in bino_start..bino_text {
        if rom(&mut h, addr) == 0xCD && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            panic!(
                "found standalone call PlayCry at {:#06X} in Route15GateLeftBinoculars — should have been removed",
                addr
            );
        }
    }
}

// ─── Ordering: PlayCry after AnimateSendingOutMon ────────────────────

#[test]
fn play_cry_after_animate_sending_out() {
    // The predef AnimateSendingOutMon call should appear before the fossil
    // checks and PlayCry in DisplayMonFrontSpriteInBox.
    // AnimateSendingOutMon predef uses: $CD lo hi (call Predef) preceded by
    // ld a, AnimateSendingOutMon_id. We check that the cp FOSSIL_KABUTOPS
    // address is greater than the predef call address.
    let mut h = banked_harness();
    let start = sym_addr("DisplayMonFrontSpriteInBox");
    let skip = sym_addr("DisplayMonFrontSpriteInBox.skipCry");
    // Find cp FOSSIL_KABUTOPS
    let mut fossil_cp_addr = None;
    for addr in start..skip {
        if rom(&mut h, addr) == 0xFE && rom(&mut h, addr + 1) == FOSSIL_KABUTOPS {
            fossil_cp_addr = Some(addr);
            break;
        }
    }
    let fossil_cp = fossil_cp_addr.expect("cp FOSSIL_KABUTOPS not found");
    // Find call Predef ($CD lo hi) before the fossil check
    let predef_addr_val = sym_addr("Predef");
    let lo = (predef_addr_val & 0xFF) as u8;
    let hi = (predef_addr_val >> 8) as u8;
    let mut predef_call_addr = None;
    for addr in start..fossil_cp {
        if rom(&mut h, addr) == 0xCD && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            predef_call_addr = Some(addr);
        }
    }
    let predef_call = predef_call_addr.expect("call Predef not found before fossil check");
    assert!(
        fossil_cp > predef_call,
        "fossil check at {:#06X} should come after predef call at {:#06X}",
        fossil_cp,
        predef_call
    );
}
