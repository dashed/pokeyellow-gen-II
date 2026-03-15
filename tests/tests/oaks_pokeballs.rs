//! ROM byte tests for the Professor Oak's Poké Balls glitch fix.
//!
//! Bug: When Professor Oak gives the player 5 Poké Balls, the script uses
//! `CheckAndSetEvent EVENT_GOT_POKEBALLS_FROM_OAK` which sets the event flag
//! BEFORE calling `GiveItem`, and never checks the carry flag for bag-full.
//! If the bag is full, the items aren't added but the text says they were,
//! and the event flag is already set — permanently losing the Poké Balls.
//!
//! Fix: Replace `CheckAndSetEvent` with `CheckEvent` (check only), add a
//! `jr nc, .no_room_for_pokeballs` carry check after `GiveItem`, and only
//! `SetEvent EVENT_GOT_POKEBALLS_FROM_OAK` on the success path. Matches
//! the pattern used by gym TM scripts (PewterGym, CeladonGym, etc.).
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Professor_Oak's_Poké_Balls_glitch>

use pokeyellow_tests::{sym_addr, sym_bank};

fn rom() -> Vec<u8> {
    std::fs::read("../pokeyellow.gbc").expect("ROM not found")
}

fn rom_offset(bank: u32, addr: u16) -> usize {
    (bank * 0x4000 + (addr as u32 - 0x4000)) as usize
}

fn at(rom: &[u8], bank: u32, addr: u16) -> u8 {
    rom[rom_offset(bank, addr)]
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn oaks_lab_script_in_bank_07() {
    assert_eq!(
        sym_bank("OaksLabOak1Text.give_poke_balls"),
        0x07,
        "Oak's lab give_poke_balls should be in bank $07"
    );
}

// ─── THE FIX: CheckEvent instead of CheckAndSetEvent ─────────────────

#[test]
fn give_poke_balls_uses_check_event_not_check_and_set() {
    let rom = rom();
    let bank = sym_bank("OaksLabOak1Text.give_poke_balls") as u32;
    let base = sym_addr("OaksLabOak1Text.give_poke_balls");

    // CheckEvent EVENT_GOT_POKEBALLS_FROM_OAK expands to:
    //   ld a, [wEventFlags + N] → $FA lo hi (3 bytes)
    //   bit B, a                → $CB $XX (2 bytes)
    // Total: 5 bytes

    // Verify ld a, [nn] ($FA) for CheckEvent
    assert_eq!(at(&rom, bank, base), 0xFA, "Expected ld a,[nn] ($FA) for CheckEvent");

    // Verify CB prefix for bit at +3
    assert_eq!(at(&rom, bank, base + 3), 0xCB, "Expected CB prefix for bit at +3");

    // At +5, should be jr nz ($20) to .come_see_me_sometimes
    // If CheckAndSetEvent were used, bytes +5/+6 would be set B,[hl] ($CB $XX)
    // instead of jr nz — so this confirms CheckEvent (not CheckAndSetEvent)
    assert_eq!(
        at(&rom, bank, base + 5),
        0x20,
        "Expected jr nz ($20) at +5 — confirms CheckEvent, not CheckAndSetEvent"
    );
}

#[test]
fn jr_nz_targets_come_see_me_sometimes() {
    let rom = rom();
    let bank = sym_bank("OaksLabOak1Text.give_poke_balls") as u32;
    let base = sym_addr("OaksLabOak1Text.give_poke_balls");

    // jr nz at offset +5
    let jr_addr = base + 5;
    assert_eq!(at(&rom, bank, jr_addr), 0x20, "Expected jr nz ($20)");
    let offset = at(&rom, bank, jr_addr + 1) as i8;
    let target = (jr_addr + 2).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("OaksLabOak1Text.come_see_me_sometimes"),
        "jr nz should target .come_see_me_sometimes"
    );
}

// ─── GiveItem and carry check ────────────────────────────────────────

#[test]
fn give_item_with_poke_ball_x5() {
    let rom = rom();
    let bank = sym_bank("OaksLabOak1Text.give_poke_balls") as u32;
    let base = sym_addr("OaksLabOak1Text.give_poke_balls");

    // After CheckEvent (5) + jr nz (2) = offset +7
    // lb bc, POKE_BALL, 5 → ld bc, nn ($01 lo hi) where lo=5 (C), hi=POKE_BALL (B)
    assert_eq!(at(&rom, bank, base + 7), 0x01, "Expected ld bc,nn ($01) for lb bc");
    // Little-endian: lo byte = C = quantity, hi byte = B = item
    assert_eq!(at(&rom, bank, base + 8), 0x05, "Expected quantity 5 (C register, lo byte)");
    assert_eq!(at(&rom, bank, base + 9), 0x04, "Expected POKE_BALL ($04, B register, hi byte)");

    // call GiveItem at +10
    assert_eq!(at(&rom, bank, base + 10), 0xCD, "Expected call ($CD) for GiveItem");
    let call_lo = at(&rom, bank, base + 11);
    let call_hi = at(&rom, bank, base + 12);
    let call_addr = u16::from_le_bytes([call_lo, call_hi]);
    assert_eq!(
        call_addr,
        sym_addr("GiveItem"),
        "call target should be GiveItem"
    );
}

#[test]
fn jr_nc_after_give_item() {
    let rom = rom();
    let bank = sym_bank("OaksLabOak1Text.give_poke_balls") as u32;
    let base = sym_addr("OaksLabOak1Text.give_poke_balls");

    // jr nc, .no_room_for_pokeballs at +13
    assert_eq!(
        at(&rom, bank, base + 13),
        0x30,
        "Expected jr nc ($30) after GiveItem — the bag-full check"
    );
}

#[test]
fn jr_nc_targets_no_room_for_pokeballs() {
    let rom = rom();
    let bank = sym_bank("OaksLabOak1Text.give_poke_balls") as u32;
    let base = sym_addr("OaksLabOak1Text.give_poke_balls");

    // jr nc at +13
    let jr_addr = base + 13;
    assert_eq!(at(&rom, bank, jr_addr), 0x30, "Expected jr nc ($30)");
    let offset = at(&rom, bank, jr_addr + 1) as i8;
    let target = (jr_addr + 2).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("OaksLabOak1Text.no_room_for_pokeballs"),
        "jr nc should target .no_room_for_pokeballs"
    );
}

// ─── SetEvent only on success path ───────────────────────────────────

#[test]
fn set_event_after_give_item_success() {
    let rom = rom();
    let bank = sym_bank("OaksLabOak1Text.give_poke_balls") as u32;
    let base = sym_addr("OaksLabOak1Text.give_poke_balls");

    // After jr nc (2 bytes at +13) = offset +15
    // SetEvent expands to: ld hl, wEventFlags+N ($21 lo hi) + set B, [hl] ($CB $XX)
    assert_eq!(
        at(&rom, bank, base + 15),
        0x21,
        "Expected ld hl,nn ($21) for SetEvent on success path"
    );
    assert_eq!(
        at(&rom, bank, base + 18),
        0xCB,
        "Expected CB prefix for set at +18"
    );
    // set B, [hl] uses opcodes $C6 + B*8 in the CB-prefixed table
    let set_operand = at(&rom, bank, base + 19);
    assert!(
        set_operand >= 0xC6,
        "Expected set instruction (CB $C6+), got ${set_operand:02X}"
    );
}

// ─── No-room path loads correct text ─────────────────────────────────

#[test]
fn no_room_path_loads_text_pointer() {
    let rom = rom();
    let bank = sym_bank("OaksLabOak1Text.no_room_for_pokeballs") as u32;
    let no_room = sym_addr("OaksLabOak1Text.no_room_for_pokeballs");

    // .no_room_for_pokeballs: ld hl, .NoRoomForPokeballsText ($21 lo hi)
    assert_eq!(
        at(&rom, bank, no_room),
        0x21,
        "Expected ld hl,nn ($21) at .no_room_for_pokeballs"
    );
    let hl_lo = at(&rom, bank, no_room + 1);
    let hl_hi = at(&rom, bank, no_room + 2);
    let hl_addr = u16::from_le_bytes([hl_lo, hl_hi]);
    assert_eq!(
        hl_addr,
        sym_addr("OaksLabOak1Text.NoRoomForPokeballsText"),
        "ld hl should point to .NoRoomForPokeballsText"
    );

    // call PrintText at no_room+3
    assert_eq!(
        at(&rom, bank, no_room + 3),
        0xCD,
        "Expected call ($CD) for PrintText"
    );
}

#[test]
fn no_room_text_entry_exists() {
    let rom = rom();
    let bank = sym_bank("OaksLabOak1Text.NoRoomForPokeballsText") as u32;
    let text_addr = sym_addr("OaksLabOak1Text.NoRoomForPokeballsText");

    // text_far expands to: db $17, dw addr, db bank (5 bytes total)
    let first_byte = at(&rom, bank, text_addr);
    assert_eq!(
        first_byte, 0x17,
        "Expected text_far command byte ($17) at .NoRoomForPokeballsText"
    );
}
