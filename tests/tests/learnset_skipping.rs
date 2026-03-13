//! ROM byte tests for the level-up learnset skipping fix.
//!
//! Bug: `LearnMoveFromLevelUp` used `jr nz` (exact level match) to check
//! whether a move should be learned, so a Pokémon that gained enough EXP
//! to skip intermediate levels would miss moves at those levels.
//!
//! Fix: change `jr nz` to `jr c` (skip only if current level < learn level),
//! so all moves at or below the current level are considered.  Save/restore
//! the learnset pointer (`push hl` / `pop hl`) so iteration continues
//! through the full learnset instead of returning after the first match.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Level-up_learnset_skipping>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Read a ROM byte at the given address (with correct bank selected).
fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const JR_C: u8 = 0x38; // jr c, e
const JR_NZ: u8 = 0x20; // jr nz, e (the old buggy opcode)
const PUSH_HL: u8 = 0xE5; // push hl
const POP_HL: u8 = 0xE1; // pop hl
const JR: u8 = 0x18; // jr e (unconditional)
const JR_Z: u8 = 0x28; // jr z, e
const LD_A_HLI: u8 = 0x2A; // ld a, [hli]
const CP_B: u8 = 0xB8; // cp b

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn learn_move_from_level_up_is_in_bank_0e() {
    assert_eq!(
        sym_bank("LearnMoveFromLevelUp"),
        0x0E,
        "LearnMoveFromLevelUp should be in bank $0E"
    );
}

#[test]
fn learnset_loop_uses_jr_c_not_jr_nz() {
    // The critical fix: jr c (skip if level < learn level) instead of
    // jr nz (skip if level != learn level).
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMoveFromLevelUp"));

    let loop_addr = sym_addr("LearnMoveFromLevelUp.learnSetLoop");

    // Instruction sequence in the loop:
    // ld a, [hli]  ; +0: learn level
    // and a         ; +1
    // jr z, .done   ; +2, +3
    // ld b, a       ; +4
    // ld a, [wCurEnemyLevel]  ; +5, +6, +7 (FA xx xx)
    // cp b          ; +8
    // ld a, [hli]   ; +9: move ID
    // jr c, .learnSetLoop  ; +10, +11  ← THE FIX (was jr nz)
    let jr_opcode_offset = loop_addr + 10;
    let opcode = rom(&mut h, jr_opcode_offset);

    assert_eq!(
        opcode, JR_C,
        "Expected jr c ($38) at .learnSetLoop+9, got ${:02X} \
         (if ${:02X} == $20, the bug is still present — jr nz means exact level match only)",
        opcode, opcode
    );
    assert_ne!(
        opcode, JR_NZ,
        "jr nz ($20) means the learnset skipping bug is still present"
    );
}

#[test]
fn push_hl_saves_learnset_pointer_after_jr_c() {
    // After jr c (which skips to loop top if level too high),
    // push hl must save the learnset pointer before HL is reused.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMoveFromLevelUp"));

    let loop_addr = sym_addr("LearnMoveFromLevelUp.learnSetLoop");
    // jr c is at loop+10 (2 bytes), push hl should be at loop+12
    let push_hl_addr = loop_addr + 12;
    let opcode = rom(&mut h, push_hl_addr);

    assert_eq!(
        opcode, PUSH_HL,
        "Expected push hl ($E5) after jr c to save learnset pointer, got ${:02X}",
        opcode
    );
}

#[test]
fn continue_learnset_restores_pointer_and_loops() {
    // .continueLearnset must: pop hl (restore learnset pointer),
    // then jr .learnSetLoop (unconditional jump back).
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMoveFromLevelUp"));

    let cont_addr = sym_addr("LearnMoveFromLevelUp.continueLearnset");
    let loop_addr = sym_addr("LearnMoveFromLevelUp.learnSetLoop");

    // pop hl
    assert_eq!(
        rom(&mut h, cont_addr),
        POP_HL,
        "Expected pop hl ($E1) at .continueLearnset"
    );

    // jr .learnSetLoop (unconditional relative jump)
    assert_eq!(
        rom(&mut h, cont_addr + 1),
        JR,
        "Expected jr ($18) after pop hl"
    );

    // Verify the jump target is .learnSetLoop
    let offset = rom(&mut h, cont_addr + 2) as i8;
    let target = (cont_addr + 3).wrapping_add(offset as u16);
    assert_eq!(
        target, loop_addr,
        "jr at .continueLearnset should jump to .learnSetLoop (${:04X}), but targets ${:04X}",
        loop_addr, target
    );
}

#[test]
fn already_known_move_jumps_to_continue_not_done() {
    // In .checkCurrentMovesLoop, when a move is already known,
    // the jr z should target .continueLearnset (not .done),
    // so the rest of the learnset is still checked.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMoveFromLevelUp"));

    let check_addr = sym_addr("LearnMoveFromLevelUp.checkCurrentMovesLoop");
    let cont_addr = sym_addr("LearnMoveFromLevelUp.continueLearnset");

    // .checkCurrentMovesLoop:
    //   ld a, [hli]   ; 2a
    //   cp d           ; ba
    //   jr z, .continueLearnset  ; 28 xx
    assert_eq!(rom(&mut h, check_addr), LD_A_HLI, "Expected ld a, [hli]");
    assert_eq!(rom(&mut h, check_addr + 1), 0xBA, "Expected cp d"); // cp d = 0xBA
    assert_eq!(
        rom(&mut h, check_addr + 2),
        JR_Z,
        "Expected jr z ($28) for already-known check"
    );

    // Verify jump target is .continueLearnset
    let offset = rom(&mut h, check_addr + 3) as i8;
    let target = (check_addr + 4).wrapping_add(offset as u16);
    assert_eq!(
        target, cont_addr,
        "jr z (already known) should jump to .continueLearnset (${:04X}), not ${:04X}",
        cont_addr, target
    );
}

#[test]
fn done_label_is_after_continue_learnset() {
    // .done must come after .continueLearnset to ensure the
    // pop hl / jr loop path doesn't accidentally fall into .done.
    let cont_addr = sym_addr("LearnMoveFromLevelUp.continueLearnset");
    let done_addr = sym_addr("LearnMoveFromLevelUp.done");

    assert!(
        done_addr > cont_addr,
        ".done (${:04X}) should be after .continueLearnset (${:04X})",
        done_addr,
        cont_addr
    );

    // .continueLearnset is exactly 3 bytes: pop hl (1) + jr xx (2) = 3
    assert_eq!(
        done_addr - cont_addr,
        3,
        ".done should be exactly 3 bytes after .continueLearnset (pop hl + jr)"
    );
}

#[test]
fn cp_b_precedes_level_comparison() {
    // Verify the comparison instruction (cp b) is in place,
    // which compares wCurEnemyLevel (A) against learn level (B).
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("LearnMoveFromLevelUp"));

    let loop_addr = sym_addr("LearnMoveFromLevelUp.learnSetLoop");
    // cp b is at loop+8 (after: 2a a7 28xx 47 FAxxxx = 8 bytes)
    assert_eq!(
        rom(&mut h, loop_addr + 8),
        CP_B,
        "Expected cp b ($B8) for level comparison"
    );
}
