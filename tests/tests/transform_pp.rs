//! ROM byte tests for the Transform + Mirror Move/Metronome PP error fix.
//!
//! Bug: When a transformed Pokémon uses Mirror Move or Metronome,
//! `IncrementMovePP` increments the party moveset PP in the corresponding
//! slot. Since battle PP and party PP are independent during Transform,
//! this corrupts party PP (can even create non-zero PP in empty slots).
//!
//! Fix: Add a TRANSFORMED check before the party PP increment, mirroring
//! the existing check in DecrementPP. Skip the increment if transformed.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_Transform_glitches#Transform_%2B_Mirror_Move/Metronome_PP_error>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const BIT_3_HL: [u8; 2] = [0xCB, 0x5E]; // bit 3, [hl] (TRANSFORMED = bit 3)
const RET_NZ: u8 = 0xC0; // ret nz

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn increment_move_pp_in_bank_0f() {
    assert_eq!(sym_bank("IncrementMovePP"), 0x0F);
}

#[test]
fn transformed_check_present() {
    // IncrementMovePP.checkTransformed should have:
    //   bit TRANSFORMED, [hl]  ($CB $5E)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let ct = sym_addr("IncrementMovePP.checkTransformed");
    assert_eq!(rom(&mut h, ct), BIT_3_HL[0], "Expected CB prefix");
    assert_eq!(
        rom(&mut h, ct + 1),
        BIT_3_HL[1],
        "Expected bit 3, [hl] (TRANSFORMED)"
    );
}

#[test]
fn ret_nz_after_transformed_check() {
    // After `bit TRANSFORMED, [hl]` and `pop hl`, there should be `ret nz`
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let ct = sym_addr("IncrementMovePP.checkTransformed");
    // bit 3, [hl] (2 bytes) + pop hl (1 byte) + ret nz (1 byte)
    assert_eq!(
        rom(&mut h, ct + 3),
        RET_NZ,
        "Expected `ret nz` after pop hl"
    );
}

#[test]
fn check_transformed_before_update_pp() {
    // .checkTransformed must come before .updatePP
    let ct = sym_addr("IncrementMovePP.checkTransformed");
    let up = sym_addr("IncrementMovePP.updatePP");
    assert!(
        ct < up,
        "checkTransformed ({:#06X}) should be before updatePP ({:#06X})",
        ct,
        up
    );
}

#[test]
fn update_pp_still_has_inc_hl() {
    // .updatePP should still end with `inc [hl]` ($34) / `ret` ($C9)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let up = sym_addr("IncrementMovePP.updatePP");
    // ld bc, PARTYMON_STRUCT_LENGTH (3) + call AddNTimes (3) + inc [hl] (1) + ret (1)
    assert_eq!(rom(&mut h, up + 6), 0x34, "Expected `inc [hl]`");
    assert_eq!(rom(&mut h, up + 7), 0xC9, "Expected `ret`");
}
