//! Emulator-based tests for TransformEffect_ (engine/battle/move_effects/transform.asm).
//!
//! TransformEffect_ copies the target's species, types, catch rate, moves,
//! DVs, and stats to the user, setting PP to 5 for each copied move (0 for
//! empty slots). It also sets the TRANSFORMED bit in BattleStatus3.
//!
//! Test approach: skip the animation/text portion (which calls Bankswitch,
//! GetMonName, PrintText) and start execution at the `pop bc` instruction
//! after all animation code. Step to `.copyStats` and verify all copied data.
//!
//! Two documented unfixed bugs in the INVULNERABLE check are also tested:
//! 1. On enemy's turn, `ldh a, [hWhoseTurn]` overwrites `ld a, [wEnemyBattleStatus1]`
//! 2. On player's turn, it reads wPlayerBattleStatus1 (own) instead of wEnemyBattleStatus1

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

const TRANSFORMED: u8 = 3;
const INVULNERABLE: u8 = 6;

fn setup_transform_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    let bank = sym_bank("TransformEffect_");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h
}

/// Run the data-copy portion of TransformEffect_ for a player-turn Transform.
///
/// Sets up enemy mon with known test values, pre-loads the stack as if the
/// animation section had already executed, and runs from `pop bc` (after
/// animations) to `.copyStats` (before GetMonName/PrintText).
fn run_player_transform() -> TestHarness {
    let mut h = setup_transform_fixture();

    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wPlayerBattleStatus3"), 0x00);

    // Enemy mon data (source for player-turn Transform)
    h.write_mem(sym_addr("wEnemyMonSpecies"), 0x54);
    h.write_mem(sym_addr("wEnemyMonType1"), 0x17);
    h.write_mem(sym_addr("wEnemyMonType2"), 0x17);
    h.write_mem(sym_addr("wEnemyMonCatchRate"), 0xBE);

    let enemy_moves = sym_addr("wEnemyMonMoves");
    h.write_mem(enemy_moves, 0x56);
    h.write_mem(enemy_moves + 1, 0x62);
    h.write_mem(enemy_moves + 2, 0x21);
    h.write_mem(enemy_moves + 3, 0x00); // empty slot

    h.write_mem(sym_addr("wEnemyMonDVs"), 0xAB);
    h.write_mem(sym_addr("wEnemyMonDVs") + 1, 0xCD);

    h.write_mem(sym_addr("wEnemyMonLevel"), 25);
    h.write_mem(sym_addr("wEnemyMonMaxHP"), 0x00);
    h.write_mem(sym_addr("wEnemyMonMaxHP") + 1, 50);

    h.write_mem(sym_addr("wEnemyMonAttack"), 0x00);
    h.write_mem(sym_addr("wEnemyMonAttack") + 1, 55);
    h.write_mem(sym_addr("wEnemyMonDefense"), 0x00);
    h.write_mem(sym_addr("wEnemyMonDefense") + 1, 40);
    h.write_mem(sym_addr("wEnemyMonSpeed"), 0x00);
    h.write_mem(sym_addr("wEnemyMonSpeed") + 1, 90);
    h.write_mem(sym_addr("wEnemyMonSpecial"), 0x00);
    h.write_mem(sym_addr("wEnemyMonSpecial") + 1, 50);

    // Pre-fill player battle mon with 0xFF to make copies obvious
    let player_base = sym_addr("wBattleMonSpecies");
    for i in 0..32 {
        h.write_mem(player_base + i, 0xFF);
    }

    // The animation code already executed push hl, push de, push bc.
    // For player's turn: hl=wEnemyMonSpecies, de=wBattleMonSpecies, bc=wPlayerBattleStatus3
    h.set_sp(0xDFF0);
    h.push_word(sym_addr("wEnemyMonSpecies"));
    h.push_word(sym_addr("wBattleMonSpecies"));
    h.push_word(sym_addr("wPlayerBattleStatus3"));

    // pop bc is 12 bytes after .gotAnimToPlay:
    //   call Bankswitch(3) + ld hl(3) + ld b(2) + pop af(1) + call nz,Bankswitch(3)
    let pop_bc = sym_addr("TransformEffect_.gotAnimToPlay") + 12;
    h.set_pc(pop_bc);

    h.step_to(sym_addr("TransformEffect_.copyStats"));

    h
}

// ─── Structural ────────────────────────────────────────────────────

#[test]
fn transform_function_exists() {
    let bank = sym_bank("TransformEffect_");
    let addr = sym_addr("TransformEffect_");
    assert_eq!(bank, 0x3D, "TransformEffect_ should be in bank $3D");
    assert!(
        addr >= 0x4000,
        "TransformEffect_ should be in banked ROM, got ${addr:04X}"
    );
}

// ─── Data copy (player's turn: enemy → player) ────────────────────

#[test]
fn transform_sets_transformed_bit() {
    let mut h = run_player_transform();
    let status3 = h.read_mem(sym_addr("wPlayerBattleStatus3"));
    assert_ne!(
        status3 & (1 << TRANSFORMED),
        0,
        "TRANSFORMED bit should be set in wPlayerBattleStatus3, got ${status3:02X}"
    );
}

#[test]
fn transform_copies_species() {
    let mut h = run_player_transform();
    assert_eq!(
        h.read_mem(sym_addr("wBattleMonSpecies")),
        0x54,
        "player species should match enemy species"
    );
}

#[test]
fn transform_copies_types() {
    let mut h = run_player_transform();
    assert_eq!(h.read_mem(sym_addr("wBattleMonType1")), 0x17);
    assert_eq!(h.read_mem(sym_addr("wBattleMonType2")), 0x17);
}

#[test]
fn transform_copies_catch_rate() {
    let mut h = run_player_transform();
    assert_eq!(h.read_mem(sym_addr("wBattleMonCatchRate")), 0xBE);
}

#[test]
fn transform_copies_moves() {
    let mut h = run_player_transform();
    let moves = sym_addr("wBattleMonMoves");
    assert_eq!(h.read_mem(moves), 0x56, "move 1");
    assert_eq!(h.read_mem(moves + 1), 0x62, "move 2");
    assert_eq!(h.read_mem(moves + 2), 0x21, "move 3");
    assert_eq!(h.read_mem(moves + 3), 0x00, "move 4 (empty)");
}

#[test]
fn transform_sets_pp_to_5_for_nonempty_moves() {
    let mut h = run_player_transform();
    let pp = sym_addr("wBattleMonPP");
    assert_eq!(h.read_mem(pp), 5, "PP for move 1");
    assert_eq!(h.read_mem(pp + 1), 5, "PP for move 2");
    assert_eq!(h.read_mem(pp + 2), 5, "PP for move 3");
}

#[test]
fn transform_empty_move_gets_0_pp() {
    let mut h = run_player_transform();
    let pp = sym_addr("wBattleMonPP");
    assert_eq!(h.read_mem(pp + 3), 0, "PP for empty move slot should be 0");
}

#[test]
fn transform_copies_dvs() {
    let mut h = run_player_transform();
    let dvs = sym_addr("wBattleMonDVs");
    assert_eq!(h.read_mem(dvs), 0xAB, "DV byte 1");
    assert_eq!(h.read_mem(dvs + 1), 0xCD, "DV byte 2");
}

#[test]
fn transform_copies_stats() {
    let mut h = run_player_transform();
    assert_eq!(h.read_mem(sym_addr("wBattleMonAttack")), 0x00);
    assert_eq!(h.read_mem(sym_addr("wBattleMonAttack") + 1), 55);
    assert_eq!(h.read_mem(sym_addr("wBattleMonDefense")), 0x00);
    assert_eq!(h.read_mem(sym_addr("wBattleMonDefense") + 1), 40);
    assert_eq!(h.read_mem(sym_addr("wBattleMonSpeed")), 0x00);
    assert_eq!(h.read_mem(sym_addr("wBattleMonSpeed") + 1), 90);
    assert_eq!(h.read_mem(sym_addr("wBattleMonSpecial")), 0x00);
    assert_eq!(h.read_mem(sym_addr("wBattleMonSpecial") + 1), 50);
}

#[test]
fn transform_does_not_copy_level() {
    let mut h = run_player_transform();
    assert_eq!(
        h.read_mem(sym_addr("wBattleMonLevel")),
        0xFF,
        "level should NOT be copied (pre-filled 0xFF should remain)"
    );
}

#[test]
fn transform_does_not_copy_max_hp() {
    let mut h = run_player_transform();
    let max_hp = sym_addr("wBattleMonMaxHP");
    assert_eq!(h.read_mem(max_hp), 0xFF, "maxHP hi should NOT be copied");
    assert_eq!(h.read_mem(max_hp + 1), 0xFF, "maxHP lo should NOT be copied");
}

// ─── Stat mods and unmodified stats via .copyBasedOnTurn ──────────

#[test]
fn transform_copies_unmodified_stats() {
    let mut h = setup_transform_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    // Source: enemy unmodified stats
    let enemy_unmod = sym_addr("wEnemyMonUnmodifiedAttack");
    for i in 0..8u16 {
        h.write_mem(enemy_unmod + i, 0x10 + i as u8);
    }

    // Dest: player unmodified stats (pre-fill with 0xFF)
    let player_unmod = sym_addr("wPlayerMonUnmodifiedAttack");
    for i in 0..8u16 {
        h.write_mem(player_unmod + i, 0xFF);
    }

    // Set up for .copyBasedOnTurn: hl=source, de=dest
    h.gb.cpu().set_hl(enemy_unmod);
    h.gb.cpu().set_de(player_unmod);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("TransformEffect_.copyBasedOnTurn"));

    h.step_to(TRAP_ADDR);

    for i in 0..8u16 {
        assert_eq!(
            h.read_mem(player_unmod + i),
            0x10 + i as u8,
            "unmodified stat byte {i} should be copied"
        );
    }
}

#[test]
fn transform_copies_stat_mods() {
    let mut h = setup_transform_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    let enemy_mods = sym_addr("wEnemyMonStatMods");
    for i in 0..8u16 {
        h.write_mem(enemy_mods + i, 0x07 + i as u8);
    }

    let player_mods = sym_addr("wPlayerMonStatMods");
    for i in 0..8u16 {
        h.write_mem(player_mods + i, 0xFF);
    }

    h.gb.cpu().set_hl(enemy_mods);
    h.gb.cpu().set_de(player_mods);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("TransformEffect_.copyBasedOnTurn"));

    h.step_to(TRAP_ADDR);

    for i in 0..8u16 {
        assert_eq!(
            h.read_mem(player_mods + i),
            0x07 + i as u8,
            "stat mod byte {i} should be copied"
        );
    }
}

// ─── INVULNERABLE check bugs ──────────────────────────────────────

/// On player's turn, the code reads wPlayerBattleStatus1 (own status) instead
/// of wEnemyBattleStatus1 (target status) for the INVULNERABLE check. This
/// means Transform against a Fly/Dig user always succeeds.
#[test]
fn invulnerable_bug_player_turn_reads_own_status() {
    let mut h = setup_transform_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wEnemyBattleStatus1"), 1 << INVULNERABLE);
    h.write_mem(sym_addr("wPlayerBattleStatus1"), 0x00);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("TransformEffect_"));

    h.step_to(sym_addr("TransformEffect_.hitTest"));

    let a = h.a();
    assert_eq!(
        a, 0x00,
        "BUG: A = wPlayerBattleStatus1 (${a:02X}), not wEnemyBattleStatus1 ($40)"
    );
    assert_eq!(
        a & (1 << INVULNERABLE),
        0,
        "BUG: INVULNERABLE bit clear — Transform proceeds against invulnerable target"
    );
}

/// On enemy's turn, `ldh a, [hWhoseTurn]` (value 1) overwrites the loaded
/// wEnemyBattleStatus1 before the INVULNERABLE check. bit 6 of 1 is always 0,
/// so the check always passes regardless of actual battle status.
#[test]
fn invulnerable_bug_enemy_turn_uses_hwhose_turn() {
    let mut h = setup_transform_fixture();
    h.write_mem(sym_addr("hWhoseTurn"), 0x01);
    h.write_mem(sym_addr("wPlayerBattleStatus1"), 1 << INVULNERABLE);
    h.write_mem(sym_addr("wEnemyBattleStatus1"), 0x00);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("TransformEffect_"));

    h.step_to(sym_addr("TransformEffect_.hitTest"));

    let a = h.a();
    assert_eq!(
        a, 0x01,
        "BUG: A = hWhoseTurn ({a}), not wEnemyBattleStatus1"
    );
    assert_eq!(
        a & (1 << INVULNERABLE),
        0,
        "BUG: bit 6 of hWhoseTurn=1 is 0 — INVULNERABLE check always passes on enemy's turn"
    );
}
