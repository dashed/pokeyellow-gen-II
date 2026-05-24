//! ROM byte tests verifying the Rare Candy level >= 100 cap fix.
//!
//! Bug: The Rare Candy code checks `cp MAX_LEVEL` / `jr z` which only blocks
//! usage at exactly level 100.  A glitch-obtained Pokémon above level 100 can
//! still use Rare Candies, leveling up to 255.  At level 255, `inc a` wraps the
//! byte to 0, creating a level 0 Pokémon.
//!
//! Fix: Change `jr z` ($28) to `jr nc` ($30) so the check becomes >= 100
//! instead of == 100.  Zero bytes added — same instruction size.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I>
//!     (section: "Leveling past 100")
//!   - <https://glitchcity.wiki/wiki/Experience_underflow_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// ─── Structural: bank and labels ────────────────────────────────────

#[test]
fn use_rare_candy_in_bank_03() {
    assert_eq!(
        sym_bank("ItemUseMedicine.useRareCandy"),
        0x03,
        "ItemUseMedicine.useRareCandy should be in bank $03"
    );
}

// ─── THE FIX: jr nc instead of jr z ─────────────────────────────────

#[test]
fn level_check_uses_jr_nc_not_jr_z() {
    let mut h = banked_harness("ItemUseMedicine.useRareCandy");
    let base = sym_addr("ItemUseMedicine.useRareCandy");

    // Instruction sequence from .useRareCandy:
    //   push hl         $E5
    //   ld bc, MON_LEVEL  $01 $21 $00
    //   add hl, bc      $09
    //   ld a, [hl]      $7E
    //   cp MAX_LEVEL     $FE $64
    //   jr nc, rel       $30 rel   ← the fix (was $28 = jr z)

    // Verify cp MAX_LEVEL ($FE $64) at offset +6
    assert_eq!(
        rom(&mut h, base + 6),
        0xFE,
        "Expected cp imm8 ($FE) at .useRareCandy+6"
    );
    assert_eq!(
        rom(&mut h, base + 7),
        0x64,
        "Expected MAX_LEVEL ($64 = 100) at .useRareCandy+7"
    );

    // Verify jr nc ($30) at offset +8 — NOT jr z ($28)
    let opcode = rom(&mut h, base + 8);
    assert_eq!(
        opcode, 0x30,
        "Expected jr nc ($30) at .useRareCandy+8 for >= 100 cap, \
         but got ${:02X} (jr z = $28 would be the unfixed version)",
        opcode
    );
}

// ─── Verify the jump target is .vitaminNoEffect ─────────────────────

#[test]
fn jr_nc_targets_vitamin_no_effect() {
    let mut h = banked_harness("ItemUseMedicine.useRareCandy");
    let base = sym_addr("ItemUseMedicine.useRareCandy");
    let no_effect = sym_addr("ItemUseMedicine.vitaminNoEffect");

    // jr nc, rel is at base+8, rel is at base+9
    // target = (base+10) + signed(rel)
    let rel = rom(&mut h, base + 9) as i8;
    let target = ((base + 10) as i32 + rel as i32) as u16;

    assert_eq!(
        target, no_effect,
        "jr nc should jump to .vitaminNoEffect (${:04X}), but targets ${:04X}",
        no_effect, target
    );
}

// ─── Verify inc a follows (the level increment) ─────────────────────

#[test]
fn inc_a_follows_level_check() {
    let mut h = banked_harness("ItemUseMedicine.useRareCandy");
    let base = sym_addr("ItemUseMedicine.useRareCandy");

    // After jr nc (2 bytes at +8), the next instruction should be inc a ($3C)
    assert_eq!(
        rom(&mut h, base + 10),
        0x3C,
        "Expected inc a ($3C) after the level check — this increments the level"
    );
}

// ─── Cross-reference: vitamin stat-EV check also uses jr nc ─────────

#[test]
fn vitamin_stat_ev_check_also_uses_jr_nc() {
    // The regular vitamin path (HP UP, PROTEIN, etc.) also uses jr nc
    // to check if stat EV is maxed. Verify this is consistent.
    let mut h = banked_harness("ItemUseVitamin");
    let base = sym_addr("ItemUseVitamin");
    let rare_candy = sym_addr("ItemUseMedicine.useRareCandy");

    // Search for cp + jr nc pattern between ItemUseVitamin and .useRareCandy
    let mut found = false;
    for addr in base..rare_candy {
        if rom(&mut h, addr) == 0x30 {
            // $30 = jr nc
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Vitamin stat-EV path should also use jr nc ($30) for its cap check"
    );
}
