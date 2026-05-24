//! ROM byte tests for the Pokédex assumption glitch fix.
//!
//! Bug: `OaksLabOak1Text` checks `wPokedexOwned >= 2` to decide whether to
//! show the Pokédex rating, but does NOT check if the player actually has
//! the Pokédex.  Having >= 2 caught species before receiving the Pokédex
//! (e.g., starter + one catch) causes Oak to show the Dex rating instead
//! of accepting Oak's Parcel, permanently blocking game progression.
//!
//! Fix: Add `CheckEvent EVENT_GOT_POKEDEX / jr z, .check_for_poke_balls`
//! after the `cp 2` / `jr c` check, matching the international Red/Blue fix.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I>
//!     (section: "Pokédex assumption glitch")
//!   - <https://glitchcity.wiki/wiki/Oak%27s_Parcel_prevented_progress_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// CheckEvent EVENT_GOT_POKEDEX expands to:
//   ld a, [wEventFlags + 4]  = $FA $4A $D7
//   bit 5, a                 = $CB $6F
const W_EVENT_FLAGS_PLUS_4_LO: u8 = 0x4A;
const W_EVENT_FLAGS_PLUS_4_HI: u8 = 0xD7;
const BIT_5_A: [u8; 2] = [0xCB, 0x6F];

// ─── Structural ─────────────────────────────────────────────────────

#[test]
fn oaks_lab_oak1_text_in_bank_07() {
    assert_eq!(
        sym_bank("OaksLabOak1Text"),
        0x07,
        "OaksLabOak1Text should be in bank $07"
    );
}

// ─── THE FIX: CheckEvent EVENT_GOT_POKEDEX guard ───────────────────

#[test]
fn check_event_got_pokedex_before_rating() {
    let mut h = banked_harness("OaksLabOak1Text");
    let base = sym_addr("OaksLabOak1Text");
    let already_got = sym_addr("OaksLabOak1Text.already_got_poke_balls");

    // Between OaksLabOak1Text and .already_got_poke_balls, find:
    //   ld a, [wEventFlags + 4]  ($FA $4A $D7)
    //   bit 5, a                 ($CB $6F)
    let mut found = false;
    for addr in base..already_got {
        if rom(&mut h, addr) == 0xFA
            && rom(&mut h, addr + 1) == W_EVENT_FLAGS_PLUS_4_LO
            && rom(&mut h, addr + 2) == W_EVENT_FLAGS_PLUS_4_HI
            && rom(&mut h, addr + 3) == BIT_5_A[0]
            && rom(&mut h, addr + 4) == BIT_5_A[1]
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "CheckEvent EVENT_GOT_POKEDEX ($FA $4A $D7 $CB $6F) should be present \
         between OaksLabOak1Text and .already_got_poke_balls"
    );
}

#[test]
fn jr_z_follows_pokedex_check() {
    let mut h = banked_harness("OaksLabOak1Text");
    let base = sym_addr("OaksLabOak1Text");
    let already_got = sym_addr("OaksLabOak1Text.already_got_poke_balls");

    // Find the CheckEvent pattern, then verify jr z ($28) follows
    for addr in base..already_got {
        if rom(&mut h, addr) == 0xFA
            && rom(&mut h, addr + 1) == W_EVENT_FLAGS_PLUS_4_LO
            && rom(&mut h, addr + 2) == W_EVENT_FLAGS_PLUS_4_HI
            && rom(&mut h, addr + 3) == BIT_5_A[0]
            && rom(&mut h, addr + 4) == BIT_5_A[1]
        {
            assert_eq!(
                rom(&mut h, addr + 5),
                0x28,
                "Expected jr z ($28) after CheckEvent EVENT_GOT_POKEDEX"
            );
            return;
        }
    }
    panic!("CheckEvent EVENT_GOT_POKEDEX not found");
}

#[test]
fn jr_z_targets_check_for_poke_balls() {
    let mut h = banked_harness("OaksLabOak1Text");
    let base = sym_addr("OaksLabOak1Text");
    let already_got = sym_addr("OaksLabOak1Text.already_got_poke_balls");
    let check_for_poke_balls = sym_addr("OaksLabOak1Text.check_for_poke_balls");

    // Find the CheckEvent, then verify jr z target = .check_for_poke_balls
    for addr in base..already_got {
        if rom(&mut h, addr) == 0xFA
            && rom(&mut h, addr + 1) == W_EVENT_FLAGS_PLUS_4_LO
            && rom(&mut h, addr + 2) == W_EVENT_FLAGS_PLUS_4_HI
            && rom(&mut h, addr + 3) == BIT_5_A[0]
            && rom(&mut h, addr + 4) == BIT_5_A[1]
        {
            let jr_addr = addr + 5;
            let rel = rom(&mut h, jr_addr + 1) as i8;
            let target = ((jr_addr + 2) as i32 + rel as i32) as u16;
            assert_eq!(
                target, check_for_poke_balls,
                "jr z should target .check_for_poke_balls (${:04X}), but targets ${:04X}",
                check_for_poke_balls, target
            );
            return;
        }
    }
    panic!("CheckEvent EVENT_GOT_POKEDEX not found");
}

// ─── Original cp 2 check preserved ──────────────────────────────────

#[test]
fn original_cp_2_check_preserved() {
    let mut h = banked_harness("OaksLabOak1Text");
    let base = sym_addr("OaksLabOak1Text");
    let already_got = sym_addr("OaksLabOak1Text.already_got_poke_balls");

    // cp 2 = $FE $02, followed by jr c = $38
    let mut found = false;
    for addr in base..already_got {
        if rom(&mut h, addr) == 0xFE
            && rom(&mut h, addr + 1) == 0x02
            && rom(&mut h, addr + 2) == 0x38
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Original cp 2 ($FE $02) / jr c ($38) check should still be present"
    );
}

// ─── CheckEvent before .already_got_poke_balls, not after ───────────

#[test]
fn pokedex_check_before_already_got_poke_balls() {
    let mut h = banked_harness("OaksLabOak1Text");
    let base = sym_addr("OaksLabOak1Text");
    let already_got = sym_addr("OaksLabOak1Text.already_got_poke_balls");

    // Find cp 2 position and CheckEvent position
    let mut cp2_addr = None;
    let mut check_event_addr = None;

    for addr in base..already_got {
        if rom(&mut h, addr) == 0xFE && rom(&mut h, addr + 1) == 0x02 {
            cp2_addr = Some(addr);
        }
        if rom(&mut h, addr) == 0xFA
            && rom(&mut h, addr + 1) == W_EVENT_FLAGS_PLUS_4_LO
            && rom(&mut h, addr + 2) == W_EVENT_FLAGS_PLUS_4_HI
        {
            check_event_addr = Some(addr);
        }
    }

    let cp2 = cp2_addr.expect("cp 2 not found");
    let check = check_event_addr.expect("CheckEvent not found");

    assert!(
        check > cp2,
        "CheckEvent (${:04X}) should come AFTER cp 2 (${:04X})",
        check,
        cp2
    );
    assert!(
        check < already_got,
        "CheckEvent (${:04X}) should come BEFORE .already_got_poke_balls (${:04X})",
        check,
        already_got
    );
}
