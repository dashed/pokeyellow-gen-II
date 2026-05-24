//! ROM byte tests for the Mimic level-up glitch fix.
//!
//! Bug: when a Pokémon that used Mimic levels up and learns a new move,
//! `LearnMove` copies all 4 party moves to battle moves. Since Mimic only
//! modified battle data, the party still has MIMIC — the copy overwrites
//! the Mimic'd move, reverting the effect.
//!
//! Fix: selectively copy each move slot. If party has MIMIC but battle has
//! a different move (the Mimic'd move), skip that slot to preserve it.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Mimic_level_up_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Read a ROM byte at the given address (with correct bank selected).
fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const LD_A_DE: u8 = 0x1A; // ld a, [de]
const LD_A_HLI: u8 = 0x2A; // ld a, [hli]
const CP_N: u8 = 0xFE; // cp n
const JR_Z: u8 = 0x28; // jr z, e
const LD_DE_A: u8 = 0x12; // ld [de], a
const INC_DE: u8 = 0x13; // inc de
const DEC_B: u8 = 0x05; // dec b
const JR_NZ: u8 = 0x20; // jr nz, e

const MIMIC: u8 = 0x66; // move constant: MIMIC = $66 (102 decimal)

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn learn_move_is_in_bank_01() {
    assert_eq!(
        sym_bank("LearnMove"),
        0x01,
        "LearnMove should be in bank $01"
    );
}

#[test]
fn copy_move_loop_exists() {
    // Verify the .copyMoveLoop label exists and is in the right bank.
    let loop_addr = sym_addr("DontAbandonLearning.copyMoveLoop");
    let bank = sym_bank("LearnMove");

    let mut h = TestHarness::new();
    h.select_rom_bank(bank);

    // First instruction should be ld a, [de] (read battle move)
    assert_eq!(
        rom(&mut h, loop_addr),
        LD_A_DE,
        "Expected ld a, [de] ($1A) at .copyMoveLoop start"
    );
}

#[test]
fn checks_battle_move_for_mimic() {
    // The loop reads the battle move with ld a, [de], then compares with MIMIC.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMove"));

    let loop_addr = sym_addr("DontAbandonLearning.copyMoveLoop");

    // ld a, [de]   ; +0
    // cp MIMIC     ; +1, +2
    assert_eq!(rom(&mut h, loop_addr + 1), CP_N, "Expected cp n ($FE)");
    assert_eq!(
        rom(&mut h, loop_addr + 2),
        MIMIC,
        "Expected MIMIC ($42) as cp operand"
    );
}

#[test]
fn reads_party_move_with_hli() {
    // After checking battle move, reads party move with ld a, [hli].
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMove"));

    let loop_addr = sym_addr("DontAbandonLearning.copyMoveLoop");

    // ld a, [de]   ; +0
    // cp MIMIC     ; +1, +2
    // ld a, [hli]  ; +3
    assert_eq!(
        rom(&mut h, loop_addr + 3),
        LD_A_HLI,
        "Expected ld a, [hli] ($2A) to read party move"
    );
}

#[test]
fn jr_z_skips_to_copy_when_battle_is_mimic() {
    // If battle=MIMIC, jr z jumps to .copyThisMove (always copy).
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMove"));

    let loop_addr = sym_addr("DontAbandonLearning.copyMoveLoop");
    let copy_addr = sym_addr("DontAbandonLearning.copyThisMove");

    // jr z is at +4, +5
    assert_eq!(rom(&mut h, loop_addr + 4), JR_Z, "Expected jr z ($28)");

    let offset = rom(&mut h, loop_addr + 5) as i8;
    let target = (loop_addr + 6).wrapping_add(offset as u16);
    assert_eq!(
        target, copy_addr,
        "jr z should target .copyThisMove (${:04X}), got ${:04X}",
        copy_addr, target
    );
}

#[test]
fn checks_party_move_for_mimic() {
    // After the first jr z (battle=MIMIC case), checks if party=MIMIC.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMove"));

    let loop_addr = sym_addr("DontAbandonLearning.copyMoveLoop");

    // cp MIMIC at +6, +7
    assert_eq!(
        rom(&mut h, loop_addr + 6),
        CP_N,
        "Expected second cp n ($FE)"
    );
    assert_eq!(
        rom(&mut h, loop_addr + 7),
        MIMIC,
        "Expected MIMIC ($42) as second cp operand"
    );
}

#[test]
fn jr_z_skips_copy_when_mimic_active() {
    // If party=MIMIC and battle≠MIMIC, jr z skips to .skipThisMove.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMove"));

    let loop_addr = sym_addr("DontAbandonLearning.copyMoveLoop");
    let skip_addr = sym_addr("DontAbandonLearning.skipThisMove");

    // jr z at +8, +9
    assert_eq!(
        rom(&mut h, loop_addr + 8),
        JR_Z,
        "Expected second jr z ($28)"
    );

    let offset = rom(&mut h, loop_addr + 9) as i8;
    let target = (loop_addr + 10).wrapping_add(offset as u16);
    assert_eq!(
        target, skip_addr,
        "jr z should target .skipThisMove (${:04X}), got ${:04X}",
        skip_addr, target
    );
}

#[test]
fn copy_this_move_writes_to_de() {
    // .copyThisMove should be ld [de], a.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMove"));

    let copy_addr = sym_addr("DontAbandonLearning.copyThisMove");
    assert_eq!(
        rom(&mut h, copy_addr),
        LD_DE_A,
        "Expected ld [de], a ($12) at .copyThisMove"
    );
}

#[test]
fn skip_this_move_advances_and_loops() {
    // .skipThisMove should: inc de, dec b, jr nz .copyMoveLoop.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMove"));

    let skip_addr = sym_addr("DontAbandonLearning.skipThisMove");
    let loop_addr = sym_addr("DontAbandonLearning.copyMoveLoop");

    assert_eq!(rom(&mut h, skip_addr), INC_DE, "Expected inc de ($13)");
    assert_eq!(rom(&mut h, skip_addr + 1), DEC_B, "Expected dec b ($05)");
    assert_eq!(rom(&mut h, skip_addr + 2), JR_NZ, "Expected jr nz ($20)");

    let offset = rom(&mut h, skip_addr + 3) as i8;
    let target = (skip_addr + 4).wrapping_add(offset as u16);
    assert_eq!(
        target, loop_addr,
        "jr nz should loop back to .copyMoveLoop (${:04X}), got ${:04X}",
        loop_addr, target
    );
}

#[test]
fn copy_this_move_is_one_byte_before_skip() {
    // .copyThisMove (ld [de], a) is exactly 1 byte before .skipThisMove (inc de).
    // This ensures the copy falls through to skip after writing.
    let copy_addr = sym_addr("DontAbandonLearning.copyThisMove");
    let skip_addr = sym_addr("DontAbandonLearning.skipThisMove");

    assert_eq!(
        skip_addr - copy_addr,
        1,
        ".skipThisMove should be 1 byte after .copyThisMove (ld [de],a is 1 byte)"
    );
}
