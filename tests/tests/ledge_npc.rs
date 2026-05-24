//! ROM byte tests for the ledge-NPC collision fix.
//!
//! Bug: When the player jumps a ledge, HandleLedges.foundMatch proceeds
//! directly to the jump animation without checking if an NPC sprite
//! occupies the landing tile. This allows the player to land on top of
//! an NPC by timing the jump when an NPC walks below the ledge.
//!
//! Fix: Before starting the ledge jump, call `IsSpriteInFrontOfPlayer2`
//! with a 2-tile range ($20 pixels) to check the landing position.
//! If a sprite is found, cancel the jump with `ret nz`.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/NPC_collision_bypassing_glitch>
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Helper to read a ROM byte at a banked address.
fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Create a TestHarness with HandleLedges bank selected.
fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("HandleLedges"));
    h
}

// ─── Structural tests for the NPC collision check ────────────────────

#[test]
fn found_match_starts_with_joy_held_check() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    // ldh a, [hJoyHeld] → $F0 $B4
    assert_eq!(rom(&mut h, fm), 0xF0, "ldh a, [n] opcode");
    assert_eq!(rom(&mut h, fm + 1), 0xB4, "hJoyHeld offset");
    // and e → $A3
    assert_eq!(rom(&mut h, fm + 2), 0xA3, "and e");
    // ret z → $C8
    assert_eq!(rom(&mut h, fm + 3), 0xC8, "ret z");
}

#[test]
fn push_de_before_sprite_check() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    // After ldh(2) + and e(1) + ret z(1) = offset 4
    assert_eq!(rom(&mut h, fm + 4), 0xD5, "push de preserves direction");
}

#[test]
fn xor_a_clears_sprite_index() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    // offset 5: xor a → $AF
    assert_eq!(rom(&mut h, fm + 5), 0xAF, "xor a");
    // offset 6: ldh [hSpriteIndex], a → $E0 $8C
    assert_eq!(rom(&mut h, fm + 6), 0xE0, "ldh [n], a opcode");
    assert_eq!(rom(&mut h, fm + 7), 0x8C, "hSpriteIndex offset ($FF8C)");
}

#[test]
fn ld_d_20_sets_two_tile_range() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    // offset 8: ld d, $20 → $16 $20
    assert_eq!(rom(&mut h, fm + 8), 0x16, "ld d, imm8 opcode");
    assert_eq!(
        rom(&mut h, fm + 9),
        0x20,
        "range = $20 (32 pixels = 2 tiles)"
    );
}

#[test]
fn call_is_sprite_in_front_of_player2() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    let target = sym_addr("IsSpriteInFrontOfPlayer2");
    // offset 10: call IsSpriteInFrontOfPlayer2 → $CD lo hi
    assert_eq!(rom(&mut h, fm + 10), 0xCD, "call opcode");
    let call_lo = rom(&mut h, fm + 11);
    let call_hi = rom(&mut h, fm + 12);
    let call_target = u16::from(call_hi) << 8 | u16::from(call_lo);
    assert_eq!(
        call_target, target,
        "call target should be IsSpriteInFrontOfPlayer2"
    );
}

#[test]
fn read_sprite_index_after_call() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    // offset 13: ldh a, [hSpriteIndex] → $F0 $8C
    assert_eq!(rom(&mut h, fm + 13), 0xF0, "ldh a, [n] opcode");
    assert_eq!(rom(&mut h, fm + 14), 0x8C, "hSpriteIndex offset ($FF8C)");
    // offset 15: and a → $A7
    assert_eq!(rom(&mut h, fm + 15), 0xA7, "and a checks if sprite found");
}

#[test]
fn pop_de_and_ret_nz_cancels_jump() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    // offset 16: pop de → $D1
    assert_eq!(rom(&mut h, fm + 16), 0xD1, "pop de restores direction");
    // offset 17: ret nz → $C0
    assert_eq!(
        rom(&mut h, fm + 17),
        0xC0,
        "ret nz cancels jump if NPC present"
    );
}

#[test]
fn original_code_follows_npc_check() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    // offset 18: ld a, PAD_BUTTONS | PAD_CTRL_PAD → $3E $FF
    // (PAD_BUTTONS = $F0, PAD_CTRL_PAD = $0F, OR = $FF)
    assert_eq!(
        rom(&mut h, fm + 18),
        0x3E,
        "ld a, imm8 opcode (original code resumes)"
    );
    assert_eq!(
        rom(&mut h, fm + 19),
        0xFF,
        "PAD_BUTTONS | PAD_CTRL_PAD = $FF"
    );
}

#[test]
fn npc_check_is_exactly_14_bytes() {
    let mut h = rom_harness();
    let fm = sym_addr("HandleLedges.foundMatch");
    // The NPC check (push de through ret nz) occupies bytes 4..17 (14 bytes)
    // Verify the full sequence: D5 AF E0 8C 16 20 CD xx xx F0 8C A7 D1 C0
    let expected_prefix = [0xD5, 0xAF, 0xE0, 0x8C, 0x16, 0x20, 0xCD];
    for (i, &exp) in expected_prefix.iter().enumerate() {
        assert_eq!(
            rom(&mut h, fm + 4 + i as u16),
            exp,
            "byte {} of NPC check",
            i
        );
    }
    // After call (3 bytes at offset 10-12), verify suffix
    let expected_suffix = [0xF0, 0x8C, 0xA7, 0xD1, 0xC0];
    for (i, &exp) in expected_suffix.iter().enumerate() {
        assert_eq!(
            rom(&mut h, fm + 13 + i as u16),
            exp,
            "byte {} of NPC check suffix",
            i
        );
    }
}

#[test]
fn handle_ledges_is_in_bank_06() {
    assert_eq!(
        sym_bank("HandleLedges"),
        0x06,
        "HandleLedges should be in bank $06"
    );
}

#[test]
fn is_sprite_in_front_of_player2_is_in_home_bank() {
    assert_eq!(
        sym_bank("IsSpriteInFrontOfPlayer2"),
        0x00,
        "IsSpriteInFrontOfPlayer2 should be in HOME (bank 0)"
    );
}
