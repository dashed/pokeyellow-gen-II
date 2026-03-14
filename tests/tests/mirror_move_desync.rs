//! ROM byte tests for the Mirror Move link battle desync fix.
//!
//! Bug: when the player's trapping move was originally selected via Mirror
//! Move and the link opponent switches during its continuation, the code
//! only checked for Metronome as a "special" move source — Mirror Move was
//! missing. This caused one console to see Mirror Move while the other saw
//! the trapping move, desynchronizing the battle.
//!
//! Fix: add `cp MIRROR_MOVE` check alongside the existing `cp METRONOME`.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Mirror_Move_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode / move constants ─────────────────────────────────────────

const CP_N: u8 = 0xFE; // cp n
const JR_Z: u8 = 0x28; // jr z, e
const JR_NZ: u8 = 0x20; // jr nz, e
const LD_A_HL: u8 = 0x7E; // ld a, [hl]
const LD_ADDR_A: u8 = 0xEA; // ld [nn], a

const METRONOME: u8 = 0x76;
const MIRROR_MOVE: u8 = 0x77;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn fix_is_in_bank_0f() {
    assert_eq!(
        sym_bank("MainInBattleLoop"),
        0x0F,
        "MainInBattleLoop should be in bank $0F"
    );
}

#[test]
fn set_special_move_label_exists() {
    let set_addr = sym_addr("MainInBattleLoop.setSpecialMove");
    let skip_addr = sym_addr("MainInBattleLoop.specialMoveNotUsed");
    assert!(
        set_addr < skip_addr,
        ".setSpecialMove (${:04X}) should be before .specialMoveNotUsed (${:04X})",
        set_addr,
        skip_addr
    );
}

#[test]
fn cp_metronome_precedes_jr_z_to_set_special() {
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("MainInBattleLoop"));

    let set_addr = sym_addr("MainInBattleLoop.setSpecialMove");

    // Working backwards from .setSpecialMove:
    // The ld a, [hl] is 7 bytes before .setSpecialMove
    // ld a, [hl]        ; -7
    // cp METRONOME       ; -6, -5
    // jr z, .setSpecial  ; -4, -3
    // cp MIRROR_MOVE     ; -2, -1
    // jr nz, .skip       ; (this is 2 bytes at .setSpecialMove - 4... actually let me just scan)

    // Find cp METRONOME before .setSpecialMove
    let base = set_addr - 8; // scan area
    let mut found = false;
    for offset in 0..6 {
        let addr = base + offset;
        if rom(&mut h, addr) == CP_N && rom(&mut h, addr + 1) == METRONOME {
            // Next should be jr z
            assert_eq!(
                rom(&mut h, addr + 2),
                JR_Z,
                "After cp METRONOME, expected jr z ($28)"
            );
            // Verify target is .setSpecialMove
            let jr_offset = rom(&mut h, addr + 3) as i8;
            let target = (addr + 4).wrapping_add(jr_offset as u16);
            assert_eq!(
                target, set_addr,
                "jr z after cp METRONOME should target .setSpecialMove (${:04X}), got ${:04X}",
                set_addr, target
            );
            found = true;
            break;
        }
    }
    assert!(
        found,
        "cp METRONOME ($FE $76) not found before .setSpecialMove"
    );
}

#[test]
fn cp_mirror_move_follows_metronome_check() {
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("MainInBattleLoop"));

    let set_addr = sym_addr("MainInBattleLoop.setSpecialMove");
    let skip_addr = sym_addr("MainInBattleLoop.specialMoveNotUsed");

    // cp MIRROR_MOVE should be at setSpecialMove - 4 (cp nn = 2 bytes, jr nz = 2 bytes)
    let cp_addr = set_addr - 4;
    assert_eq!(
        rom(&mut h, cp_addr),
        CP_N,
        "Expected cp n ($FE) at .setSpecialMove - 4"
    );
    assert_eq!(
        rom(&mut h, cp_addr + 1),
        MIRROR_MOVE,
        "Expected MIRROR_MOVE ($77) as cp operand"
    );

    // jr nz to .specialMoveNotUsed
    assert_eq!(
        rom(&mut h, cp_addr + 2),
        JR_NZ,
        "Expected jr nz ($20) after cp MIRROR_MOVE"
    );
    let jr_offset = rom(&mut h, cp_addr + 3) as i8;
    let target = (cp_addr + 4).wrapping_add(jr_offset as u16);
    assert_eq!(
        target, skip_addr,
        "jr nz after cp MIRROR_MOVE should target .specialMoveNotUsed (${:04X}), got ${:04X}",
        skip_addr, target
    );
}

#[test]
fn set_special_move_writes_selected_move() {
    // .setSpecialMove should write A to wPlayerSelectedMove via ld [nn], a.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("MainInBattleLoop"));

    let set_addr = sym_addr("MainInBattleLoop.setSpecialMove");
    assert_eq!(
        rom(&mut h, set_addr),
        LD_ADDR_A,
        "Expected ld [nn], a ($EA) at .setSpecialMove"
    );
}

#[test]
fn special_move_not_used_is_3_bytes_after_set() {
    // .setSpecialMove: ld [nn], a = 3 bytes → .specialMoveNotUsed
    let set_addr = sym_addr("MainInBattleLoop.setSpecialMove");
    let skip_addr = sym_addr("MainInBattleLoop.specialMoveNotUsed");
    assert_eq!(
        skip_addr - set_addr,
        3,
        ".specialMoveNotUsed should be 3 bytes after .setSpecialMove (ld [nn],a = 3 bytes)"
    );
}

#[test]
fn ld_a_hl_reads_move_before_checks() {
    // Before the cp METRONOME check, ld a, [hl] loads the player's actual move.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("MainInBattleLoop"));

    let set_addr = sym_addr("MainInBattleLoop.setSpecialMove");
    // ld a, [hl] is 9 bytes before .setSpecialMove:
    // ld a,[hl](1) + cp METRONOME(2) + jr z(2) + cp MIRROR_MOVE(2) + jr nz(2) = 9
    let ld_addr = set_addr - 9;
    assert_eq!(
        rom(&mut h, ld_addr),
        LD_A_HL,
        "Expected ld a, [hl] ($7E) to load the move from wBattleMonMoves"
    );
}
