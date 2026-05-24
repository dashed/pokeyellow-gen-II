//! ROM byte tests for the Substitute + Confusion/Jump Kick self-damage fix.
//!
//! Bug: When a Pokémon with a Substitute hurts itself (confusion, disobedience,
//! Jump Kick/Hi Jump Kick crash), damage goes to the OPPONENT's Substitute
//! instead of the user's HP, because `AttackSubstitute` uses `hWhoseTurn`
//! to pick which side's Substitute to damage — but during self-damage,
//! `hWhoseTurn` indicates "my turn" rather than "attacking opponent."
//!
//! Fix: Self-damage paths now jump to `ApplyDamage*Direct` labels that
//! skip the Substitute check, applying damage directly to HP.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Substitute_+_Confusion_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Read a 16-bit little-endian value from ROM
fn rom16(h: &mut TestHarness, addr: u16) -> u16 {
    let lo = rom(h, addr) as u16;
    let hi = rom(h, addr + 1) as u16;
    (hi << 8) | lo
}

// ─── Opcode constants ────────────────────────────────────────────────

const JP_NN: u8 = 0xC3; // jp nn
const CALL_NN: u8 = 0xCD; // call nn
const LD_HL_NN: u8 = 0x21; // ld hl, nn
const LD_A_HLD: u8 = 0x3A; // ld a, [hld] / ld a, [hl-]

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn direct_labels_in_bank_0f() {
    assert_eq!(sym_bank("ApplyDamageToPlayerPokemonDirect"), 0x0F);
    assert_eq!(sym_bank("ApplyDamageToEnemyPokemonDirect"), 0x0F);
}

#[test]
fn direct_labels_after_substitute_check() {
    // The Direct labels must come right after the `jp nz, AttackSubstitute`
    // in each ApplyDamage function (17 bytes of preamble + sub check)
    let player_main = sym_addr("ApplyDamageToPlayerPokemon");
    let player_direct = sym_addr("ApplyDamageToPlayerPokemonDirect");
    let enemy_main = sym_addr("ApplyDamageToEnemyPokemon");
    let enemy_direct = sym_addr("ApplyDamageToEnemyPokemonDirect");

    assert_eq!(player_direct - player_main, 17);
    assert_eq!(enemy_direct - enemy_main, 17);
}

#[test]
fn no_substitute_check_in_direct_path() {
    // At the Direct label, the first instruction should be `ld a, [hld]` ($3A),
    // which is the start of HP subtraction — NOT a Substitute status check.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    assert_eq!(
        rom(&mut h, sym_addr("ApplyDamageToPlayerPokemonDirect")),
        LD_A_HLD,
        "Player Direct should start with `ld a, [hld]`"
    );
    assert_eq!(
        rom(&mut h, sym_addr("ApplyDamageToEnemyPokemonDirect")),
        LD_A_HLD,
        "Enemy Direct should start with `ld a, [hld]`"
    );
}

#[test]
fn player_confusion_targets_direct() {
    // HandleSelfConfusionDamage ends with:
    //   ld hl, wDamage + 1    ($21)
    //   jp ApplyDamageToPlayerPokemonDirect  ($C3)
    // The next symbol after HandleSelfConfusionDamage is DisplayUsedMoveText
    // (from the included used_move_text.asm).
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let next_sym = sym_addr("DisplayUsedMoveText");
    let jp_addr = next_sym - 3; // jp nn is last 3 bytes
    let ld_addr = jp_addr - 3; // ld hl, nn before that

    assert_eq!(rom(&mut h, ld_addr), LD_HL_NN, "Expected `ld hl, nn`");
    assert_eq!(rom(&mut h, jp_addr), JP_NN, "Expected `jp nn`");
    assert_eq!(
        rom16(&mut h, jp_addr + 1),
        sym_addr("ApplyDamageToPlayerPokemonDirect"),
        "jp should target ApplyDamageToPlayerPokemonDirect"
    );
}

#[test]
fn jump_kick_crash_player_targets_direct() {
    // PrintMoveFailureText player path:
    //   ld hl, wDamage + 1 / jp ApplyDamageToPlayerPokemonDirect
    // followed by .enemyTurn label
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let enemy_turn = sym_addr("PrintMoveFailureText.enemyTurn");
    let jp_addr = enemy_turn - 3;
    let ld_addr = jp_addr - 3;

    assert_eq!(rom(&mut h, ld_addr), LD_HL_NN, "Expected `ld hl, nn`");
    assert_eq!(rom(&mut h, jp_addr), JP_NN, "Expected `jp nn`");
    assert_eq!(
        rom16(&mut h, jp_addr + 1),
        sym_addr("ApplyDamageToPlayerPokemonDirect"),
        "jp should target ApplyDamageToPlayerPokemonDirect"
    );
}

#[test]
fn jump_kick_crash_enemy_targets_direct() {
    // PrintMoveFailureText.enemyTurn:
    //   ld hl, wDamage + 1 / jp ApplyDamageToEnemyPokemonDirect
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let enemy_turn = sym_addr("PrintMoveFailureText.enemyTurn");

    assert_eq!(
        rom(&mut h, enemy_turn),
        LD_HL_NN,
        "Expected `ld hl, nn` at .enemyTurn"
    );
    assert_eq!(rom(&mut h, enemy_turn + 3), JP_NN, "Expected `jp nn`");
    assert_eq!(
        rom16(&mut h, enemy_turn + 4),
        sym_addr("ApplyDamageToEnemyPokemonDirect"),
        "jp should target ApplyDamageToEnemyPokemonDirect"
    );
}

#[test]
fn enemy_confusion_targets_direct() {
    // Enemy confusion self-hit:
    //   ld hl, wDamage + 1 / call ApplyDamageToEnemyPokemonDirect / jr .monHurtItself...
    // jr (2 bytes) is right before .checkIfTriedToUseDisabledMove
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let next_label = sym_addr("CheckEnemyStatusConditions.checkIfTriedToUseDisabledMove");
    let jr_addr = next_label - 2; // jr nn
    let call_addr = jr_addr - 3; // call nn
    let ld_addr = call_addr - 3; // ld hl, nn

    assert_eq!(rom(&mut h, ld_addr), LD_HL_NN, "Expected `ld hl, nn`");
    assert_eq!(rom(&mut h, call_addr), CALL_NN, "Expected `call nn`");
    assert_eq!(
        rom16(&mut h, call_addr + 1),
        sym_addr("ApplyDamageToEnemyPokemonDirect"),
        "call should target ApplyDamageToEnemyPokemonDirect"
    );
}
