//! ROM byte tests verifying the Pokémon merge glitch fix.
//!
//! Bug: `_RemovePokemon`'s species-shift loop uses `inc a / jr nz` to detect
//! the `$FF` list terminator.  A glitch Pokémon with species index `$FF` is
//! indistinguishable from the terminator, causing the loop to exit early.
//! The OT/nickname/struct data (shifted by address-range `CopyDataUntil`) gets
//! out of sync with the truncated species list, creating "merged" hybrids.
//!
//! Fix: Replace the `$FF`-based terminator loop with a count-based loop.
//! Before shifting, compute how many bytes to copy from the (already-
//! decremented) count and `wWhichPokemon`, then use `dec b / jr nz`.
//! +5 bytes in banked ROM.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_merge_glitch>
//!   - <https://glitchcity.wiki/wiki/Pok%C3%A9mon_merge_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// ─── Structural: bank check ────────────────────────────────────────

#[test]
fn remove_pokemon_in_bank_01() {
    assert_eq!(
        sym_bank("_RemovePokemon"),
        0x01,
        "_RemovePokemon should be in bank $01"
    );
}

// ─── THE FIX: count-based loop instead of $FF terminator ───────────

#[test]
fn push_pop_af_pair_in_species_shift_setup() {
    let mut h = banked_harness("_RemovePokemon.gotCount");
    let start = sym_addr("_RemovePokemon.gotCount");
    let loop_start = sym_addr("_RemovePokemon.shiftMonSpeciesLoop");

    // push af ($F5) and pop af ($F1) should both exist between .gotCount and .shiftMonSpeciesLoop
    let mut found_push = false;
    let mut found_pop = false;
    for addr in start..loop_start {
        if rom(&mut h, addr) == 0xF5 {
            found_push = true;
        }
        if rom(&mut h, addr) == 0xF1 {
            found_pop = true;
        }
    }
    assert!(
        found_push,
        "push af ($F5) should exist between .gotCount and .shiftMonSpeciesLoop"
    );
    assert!(
        found_pop,
        "pop af ($F1) should exist between .gotCount and .shiftMonSpeciesLoop"
    );
}

#[test]
fn sub_c_before_loop() {
    let mut h = banked_harness("_RemovePokemon.gotCount");
    let start = sym_addr("_RemovePokemon.gotCount");
    let loop_start = sym_addr("_RemovePokemon.shiftMonSpeciesLoop");

    // sub c ($91) should exist in the setup before the loop
    let mut found = false;
    for addr in start..loop_start {
        if rom(&mut h, addr) == 0x91 {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "sub c ($91) should exist before .shiftMonSpeciesLoop — \
         computes count-based iteration limit"
    );
}

#[test]
fn loop_uses_dec_b_not_inc_a() {
    let mut h = banked_harness("_RemovePokemon.shiftMonSpeciesLoop");
    let loop_start = sym_addr("_RemovePokemon.shiftMonSpeciesLoop");

    // The loop body should be: ld a,[de] ($1A) / inc de ($13) / ld [hli],a ($22) / dec b ($05) / jr nz ($20)
    assert_eq!(
        rom(&mut h, loop_start),
        0x1A,
        "Expected ld a,[de] ($1A) at loop start"
    );
    assert_eq!(
        rom(&mut h, loop_start + 1),
        0x13,
        "Expected inc de ($13) at loop+1"
    );
    assert_eq!(
        rom(&mut h, loop_start + 2),
        0x22,
        "Expected ld [hli],a ($22) at loop+2"
    );

    // The critical fix: dec b ($05) instead of inc a ($3C)
    let opcode = rom(&mut h, loop_start + 3);
    assert_eq!(
        opcode, 0x05,
        "Expected dec b ($05) at loop+3 for count-based termination, \
         but got ${:02X} (inc a = $3C would be the unfixed version)",
        opcode
    );

    assert_eq!(
        rom(&mut h, loop_start + 4),
        0x20,
        "Expected jr nz ($20) at loop+4"
    );
}

#[test]
fn loop_jumps_back_to_start() {
    let mut h = banked_harness("_RemovePokemon.shiftMonSpeciesLoop");
    let loop_start = sym_addr("_RemovePokemon.shiftMonSpeciesLoop");

    // jr nz at loop+4, relative offset at loop+5
    // target = (loop+6) + signed(offset) should equal loop_start
    let rel = rom(&mut h, loop_start + 5) as i8;
    let target = ((loop_start + 6) as i32 + rel as i32) as u16;
    assert_eq!(
        target, loop_start,
        "jr nz should jump back to .shiftMonSpeciesLoop (${:04X}), but targets ${:04X}",
        loop_start, target
    );
}

// ─── Cross-reference: CopyDataUntil still used for OT/struct shifts ─

#[test]
fn copy_data_until_still_used_for_ot_shift() {
    let mut h = banked_harness("_RemovePokemon.gotOTsPointer");
    let ot_section = sym_addr("_RemovePokemon.gotOTsPointer");
    let end = sym_addr("_RemovePokemon.shiftMonNicks");

    let copy_data_until = sym_addr("CopyDataUntil");
    let cd_lo = (copy_data_until & 0xFF) as u8;
    let cd_hi = (copy_data_until >> 8) as u8;

    // Verify at least one `call CopyDataUntil` ($CD lo hi) exists
    let mut count = 0;
    for addr in ot_section..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == cd_lo
            && rom(&mut h, addr + 2) == cd_hi
        {
            count += 1;
        }
    }
    assert!(
        count >= 2,
        "Expected at least 2 calls to CopyDataUntil for OT/struct shifts, found {}",
        count
    );
}
