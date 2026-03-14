//! ROM byte tests for the Trapping sleep glitch fix.
//!
//! Bug: When a player's Pokemon is trapped (Wrap/Bind/etc.) and the player
//! uses items, wPlayerSelectedMove stays at CANNOT_MOVE ($FF). If the
//! trapping ends and the enemy puts the player to sleep, ExecutePlayerMove
//! sees $FF and skips CheckPlayerStatusConditions entirely — the sleep
//! counter never decrements, leaving the Pokemon permanently asleep.
//!
//! Fix: When wPlayerSelectedMove == CANNOT_MOVE, still call
//! CheckPlayerStatusConditions before skipping to ExecutePlayerMoveDone.
//! Same fix applied to enemy side.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Trapping_sleep_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom16(h: &mut TestHarness, addr: u16) -> u16 {
    let lo = rom(h, addr) as u16;
    let hi = rom(h, addr + 1) as u16;
    (hi << 8) | lo
}

// ─── Opcode constants ────────────────────────────────────────────────

const JR_NZ: u8 = 0x20; // jr nz, n
const CALL_NN: u8 = 0xCD; // call nn
const JP_NN: u8 = 0xC3; // jp nn
const INC_A: u8 = 0x3C; // inc a

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn player_side_fix_present() {
    // ExecutePlayerMove should have:
    //   inc a            ($3C)
    //   jr nz, .canMove  ($20 nn)
    //   call CheckPlayerStatusConditions  ($CD lo hi)
    //   jp ExecutePlayerMoveDone          ($C3 lo hi)
    // .canMove:
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let can_move = sym_addr("ExecutePlayerMove.canMove");
    // Working backward: jp nn (3) + call nn (3) + jr nz (2) + inc a (1) = 9 bytes
    let inc_addr = can_move - 9;

    assert_eq!(rom(&mut h, inc_addr), INC_A, "Expected `inc a`");
    assert_eq!(rom(&mut h, inc_addr + 1), JR_NZ, "Expected `jr nz`");
    assert_eq!(
        rom(&mut h, inc_addr + 3),
        CALL_NN,
        "Expected `call CheckPlayerStatusConditions`"
    );
    assert_eq!(
        rom16(&mut h, inc_addr + 4),
        sym_addr("CheckPlayerStatusConditions"),
        "call target should be CheckPlayerStatusConditions"
    );
    assert_eq!(
        rom(&mut h, inc_addr + 6),
        JP_NN,
        "Expected `jp ExecutePlayerMoveDone`"
    );
    assert_eq!(
        rom16(&mut h, inc_addr + 7),
        sym_addr("ExecutePlayerMoveDone"),
        "jp target should be ExecutePlayerMoveDone"
    );
}

#[test]
fn enemy_side_fix_present() {
    // Same pattern for ExecuteEnemyMove
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let can_move = sym_addr("ExecuteEnemyMove.canMove");
    let inc_addr = can_move - 9;

    assert_eq!(rom(&mut h, inc_addr), INC_A, "Expected `inc a`");
    assert_eq!(rom(&mut h, inc_addr + 1), JR_NZ, "Expected `jr nz`");
    assert_eq!(
        rom(&mut h, inc_addr + 3),
        CALL_NN,
        "Expected `call CheckEnemyStatusConditions`"
    );
    assert_eq!(
        rom16(&mut h, inc_addr + 4),
        sym_addr("CheckEnemyStatusConditions"),
        "call target should be CheckEnemyStatusConditions"
    );
    assert_eq!(
        rom(&mut h, inc_addr + 6),
        JP_NN,
        "Expected `jp ExecuteEnemyMoveDone`"
    );
    assert_eq!(
        rom16(&mut h, inc_addr + 7),
        sym_addr("ExecuteEnemyMoveDone"),
        "jp target should be ExecuteEnemyMoveDone"
    );
}

#[test]
fn both_fixes_in_bank_0f() {
    assert_eq!(sym_bank("ExecutePlayerMove"), 0x0F);
    assert_eq!(sym_bank("ExecuteEnemyMove"), 0x0F);
}

#[test]
fn can_move_labels_exist() {
    // .canMove labels should exist for both sides
    let player = sym_addr("ExecutePlayerMove.canMove");
    let enemy = sym_addr("ExecuteEnemyMove.canMove");
    assert!(player > 0, "Player .canMove label should exist");
    assert!(enemy > 0, "Enemy .canMove label should exist");
    assert!(enemy > player, "Enemy .canMove should be after player");
}
