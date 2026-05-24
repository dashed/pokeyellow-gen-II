//! ROM byte tests for the UpdateNPCSprite movement byte carry fix.
//!
//! Bug: `UpdateNPCSprite` computes the address of an NPC's movement byte
//! in `wMapSpriteData` by adding the sprite offset to the low byte of the
//! base address (`add l` / `ld l, a`). If this addition overflows past
//! $FF, a carry is generated but never propagated to H. The resulting
//! address is wrong, causing the NPC to read someone else's movement byte
//! and behave incorrectly (e.g. walking when it should stay, or vice versa).
//!
//! Fix: Add `jr nc, .noCarry` / `inc h` after `ld l, a` so the carry is
//! propagated to the high byte of HL. +3 bytes in bank $01.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches>
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("UpdateNPCSprite"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn update_npc_sprite_in_bank_01() {
    assert_eq!(sym_bank("UpdateNPCSprite"), 0x01);
}

#[test]
fn sprite_offset_computation_sequence() {
    let mut h = rom_harness();
    let base = sym_addr("UpdateNPCSprite");
    // ldh a, [hCurrentSpriteOffset] → $F0 xx (2 bytes)
    assert_eq!(rom(&mut h, base), 0xF0, "ldh a, [n] opcode");
    // swap a → $CB $37 (2 bytes)
    assert_eq!(rom(&mut h, base + 2), 0xCB, "CB prefix");
    assert_eq!(rom(&mut h, base + 3), 0x37, "swap a");
    // dec a → $3D (1 byte)
    assert_eq!(rom(&mut h, base + 4), 0x3D, "dec a");
    // add a → $87 (1 byte)
    assert_eq!(rom(&mut h, base + 5), 0x87, "add a (multiply by 2)");
}

#[test]
fn base_address_loads_map_sprite_data() {
    let mut h = rom_harness();
    let base = sym_addr("UpdateNPCSprite");
    // ld hl, wMapSpriteData → $21 lo hi (3 bytes) at offset +6
    assert_eq!(rom(&mut h, base + 6), 0x21, "ld hl, nn opcode");
    let lo = rom(&mut h, base + 7);
    let hi = rom(&mut h, base + 8);
    let addr = u16::from_le_bytes([lo, hi]);
    assert_eq!(
        addr,
        sym_addr("wMapSpriteData"),
        "ld hl should point to wMapSpriteData"
    );
}

#[test]
fn offset_addition_to_l_register() {
    let mut h = rom_harness();
    let base = sym_addr("UpdateNPCSprite");
    // add l → $85 at offset +9
    assert_eq!(rom(&mut h, base + 9), 0x85, "add l");
    // ld l, a → $6F at offset +10
    assert_eq!(rom(&mut h, base + 10), 0x6F, "ld l, a");
}

// ─── THE FIX: carry propagation ──────────────────────────────────────

#[test]
fn carry_propagation_jr_nc() {
    let mut h = rom_harness();
    let base = sym_addr("UpdateNPCSprite");
    // jr nc, .noCarry → $30 $01 at offset +11
    assert_eq!(
        rom(&mut h, base + 11),
        0x30,
        "jr nc opcode (carry propagation fix)"
    );
    assert_eq!(
        rom(&mut h, base + 12),
        0x01,
        "jr nc offset = 1 (skip inc h)"
    );
}

#[test]
fn carry_propagation_inc_h() {
    let mut h = rom_harness();
    let base = sym_addr("UpdateNPCSprite");
    // inc h → $24 at offset +13
    assert_eq!(
        rom(&mut h, base + 13),
        0x24,
        "inc h (propagate carry to high byte)"
    );
}

#[test]
fn movement_byte_read_after_carry_fix() {
    let mut h = rom_harness();
    let base = sym_addr("UpdateNPCSprite");
    // .noCarry: ld a, [hl] → $7E at offset +14
    assert_eq!(
        rom(&mut h, base + 14),
        0x7E,
        "ld a, [hl] reads movement byte after carry fix"
    );
}

// ─── Negative test ───────────────────────────────────────────────────

#[test]
fn no_bare_add_l_to_read_without_carry_check() {
    let mut h = rom_harness();
    let base = sym_addr("UpdateNPCSprite");
    // In the buggy code, the sequence was: add l ($85) / ld l, a ($6F) / ld a, [hl] ($7E)
    // With the fix, there MUST be jr nc ($30) between ld l, a and ld a, [hl]
    // Verify the byte after ld l, a is NOT ld a, [hl] ($7E)
    assert_ne!(
        rom(&mut h, base + 11),
        0x7E,
        "ld a, [hl] must NOT immediately follow ld l, a (carry bug)"
    );
}
