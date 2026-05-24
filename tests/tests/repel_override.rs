//! ROM byte tests for the Repel effect override fix.
//!
//! Bug: Using a Repel, Super Repel, or Max Repel while one is already active
//! unconditionally overwrites `wRepelRemainingSteps`, wasting any remaining
//! steps from the previous use. For example, using a Max Repel (250 steps)
//! then a Repel (100 steps) discards 150 steps.
//!
//! Fix: In `ItemUseRepelCommon`, check if `wRepelRemainingSteps` is nonzero
//! before writing. If already active, jump to `ItemUseFailed` with a
//! "repel's effect is still active" message. Matches Gen II+ behavior.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_effect_override>

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
fn item_use_repel_common_in_bank_03() {
    assert_eq!(
        sym_bank("ItemUseRepelCommon"),
        0x03,
        "ItemUseRepelCommon should be in bank $03"
    );
}

#[test]
fn all_repel_types_converge_to_common() {
    // ItemUseRepel: ld b, 100 then falls through to ItemUseRepelCommon
    // ItemUseSuperRepel: ld b, 200 then jp ItemUseRepelCommon
    // ItemUseMaxRepel: ld b, 250 then jp ItemUseRepelCommon
    let rom = rom();
    let bank = sym_bank("ItemUseRepel") as u32;

    // ItemUseRepel: ld b, 100 ($06 $64)
    let repel = sym_addr("ItemUseRepel");
    assert_eq!(at(&rom, bank, repel), 0x06, "ItemUseRepel: ld b, n");
    assert_eq!(
        at(&rom, bank, repel + 1),
        100,
        "ItemUseRepel: b = 100 steps"
    );

    // ItemUseSuperRepel: ld b, 200 ($06 $C8) + jp ItemUseRepelCommon
    let super_repel = sym_addr("ItemUseSuperRepel");
    assert_eq!(
        at(&rom, bank, super_repel + 1),
        200,
        "SuperRepel: b = 200 steps"
    );

    // ItemUseMaxRepel: ld b, 250 ($06 $FA) + jp ItemUseRepelCommon
    let max_repel = sym_addr("ItemUseMaxRepel");
    assert_eq!(
        at(&rom, bank, max_repel + 1),
        250,
        "MaxRepel: b = 250 steps"
    );
}

// ─── THE FIX: wRepelRemainingSteps check ──────────────────────────────

#[test]
fn checks_repel_remaining_steps_before_writing() {
    let rom = rom();
    let bank = sym_bank("ItemUseRepelCommon") as u32;
    let base = sym_addr("ItemUseRepelCommon");

    // After battle check (ld a,[wIsInBattle] + and a + jp nz = 7 bytes)
    // The fix: ld a, [wRepelRemainingSteps] at +7
    assert_eq!(
        at(&rom, bank, base + 7),
        0xFA,
        "Expected ld a,[nn] ($FA) to load wRepelRemainingSteps"
    );
    let lo = at(&rom, bank, base + 8);
    let hi = at(&rom, bank, base + 9);
    let addr = u16::from_le_bytes([lo, hi]);
    assert_eq!(
        addr,
        sym_addr("wRepelRemainingSteps"),
        "Should load from wRepelRemainingSteps"
    );

    // and a at +10
    assert_eq!(at(&rom, bank, base + 10), 0xA7, "Expected and a ($A7)");

    // jr nz, .alreadyActive at +11
    assert_eq!(at(&rom, bank, base + 11), 0x20, "Expected jr nz ($20)");
}

#[test]
fn jr_nz_targets_already_active() {
    let rom = rom();
    let bank = sym_bank("ItemUseRepelCommon") as u32;
    let base = sym_addr("ItemUseRepelCommon");

    let jr_addr = base + 11;
    let offset = at(&rom, bank, jr_addr + 1) as i8;
    let target = (jr_addr + 2).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("ItemUseRepelCommon.alreadyActive"),
        "jr nz should target .alreadyActive"
    );
}

// ─── Normal path still writes steps ──────────────────────────────────

#[test]
fn normal_path_writes_repel_steps() {
    let rom = rom();
    let bank = sym_bank("ItemUseRepelCommon") as u32;
    let base = sym_addr("ItemUseRepelCommon");

    // After jr nz (2 bytes at +11) = offset +13
    // ld a, b ($78) at +13
    assert_eq!(at(&rom, bank, base + 13), 0x78, "Expected ld a, b ($78)");

    // ld [wRepelRemainingSteps], a ($EA lo hi) at +14
    assert_eq!(at(&rom, bank, base + 14), 0xEA, "Expected ld [nn], a ($EA)");
    let lo = at(&rom, bank, base + 15);
    let hi = at(&rom, bank, base + 16);
    let addr = u16::from_le_bytes([lo, hi]);
    assert_eq!(
        addr,
        sym_addr("wRepelRemainingSteps"),
        "Should write to wRepelRemainingSteps"
    );
}

// ─── Already-active path ─────────────────────────────────────────────

#[test]
fn already_active_loads_text_and_jumps_to_item_use_failed() {
    let rom = rom();
    let bank = sym_bank("ItemUseRepelCommon.alreadyActive") as u32;
    let already = sym_addr("ItemUseRepelCommon.alreadyActive");

    // ld hl, RepelAlreadyActiveText ($21 lo hi)
    assert_eq!(at(&rom, bank, already), 0x21, "Expected ld hl,nn ($21)");
    let hl_lo = at(&rom, bank, already + 1);
    let hl_hi = at(&rom, bank, already + 2);
    let hl_addr = u16::from_le_bytes([hl_lo, hl_hi]);
    assert_eq!(
        hl_addr,
        sym_addr("RepelAlreadyActiveText"),
        "ld hl should point to RepelAlreadyActiveText"
    );

    // jp ItemUseFailed ($C3 lo hi)
    assert_eq!(at(&rom, bank, already + 3), 0xC3, "Expected jp ($C3)");
    let jp_lo = at(&rom, bank, already + 4);
    let jp_hi = at(&rom, bank, already + 5);
    let jp_addr = u16::from_le_bytes([jp_lo, jp_hi]);
    assert_eq!(
        jp_addr,
        sym_addr("ItemUseFailed"),
        "jp should target ItemUseFailed"
    );
}

#[test]
fn repel_already_active_text_entry_exists() {
    let rom = rom();
    let bank = sym_bank("RepelAlreadyActiveText") as u32;
    let text_addr = sym_addr("RepelAlreadyActiveText");

    // text_far = $17
    assert_eq!(
        at(&rom, bank, text_addr),
        0x17,
        "Expected text_far command byte ($17)"
    );
}
