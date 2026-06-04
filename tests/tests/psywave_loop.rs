//! ROM byte tests for the Psywave infinite loop fix.
//!
//! Bug: Psywave generates random damage in [1, level*1.5). For levels 0, 1,
//! or 171 (byte overflow to 0), the upper bound is ≤1, making the valid
//! range empty — the RNG rejection loop never terminates, softlocking.
//!
//! Fix: clamp the upper bound (B register) to minimum 2 before the loop
//! so the range [1, 2) = {1} always terminates. Applied to both player
//! and enemy Psywave paths.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Psywave_infinite_loop>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const CP_N: u8 = 0xFE; // cp n
const JR_NC: u8 = 0x30; // jr nc, e
const LD_B_N: u8 = 0x06; // ld b, n

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn psywave_is_in_bank_0f() {
    assert_eq!(
        sym_bank("ApplyAttackToEnemyPokemon"),
        0x0F,
        "Psywave code should be in bank $0F"
    );
}

#[test]
fn player_psywave_clamp_before_loop() {
    // The clamp (cp 2 / jr nc / ld b, 2) should appear 6 bytes before .loop.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let loop_addr = sym_addr("ApplyAttackToEnemyPokemon.loop");
    let clamp_addr = loop_addr - 6;

    // cp 2
    assert_eq!(rom(&mut h, clamp_addr), CP_N, "Expected cp n ($FE)");
    assert_eq!(rom(&mut h, clamp_addr + 1), 2, "Expected operand 2");

    // jr nc, .loop
    assert_eq!(rom(&mut h, clamp_addr + 2), JR_NC, "Expected jr nc ($30)");
    let offset = rom(&mut h, clamp_addr + 3) as i8;
    let target = (clamp_addr + 4).wrapping_add(offset as u16);
    assert_eq!(
        target, loop_addr,
        "jr nc should target .loop (${:04X}), got ${:04X}",
        loop_addr, target
    );

    // ld b, 2
    assert_eq!(
        rom(&mut h, clamp_addr + 4),
        LD_B_N,
        "Expected ld b, n ($06)"
    );
    assert_eq!(rom(&mut h, clamp_addr + 5), 2, "Expected ld b, 2");
}

#[test]
fn enemy_psywave_clamp_before_loop() {
    // Same clamp on the enemy side.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let loop_addr = sym_addr("ApplyAttackToPlayerPokemon.loop");
    let clamp_addr = loop_addr - 6;

    assert_eq!(rom(&mut h, clamp_addr), CP_N, "Expected cp n ($FE)");
    assert_eq!(rom(&mut h, clamp_addr + 1), 2, "Expected operand 2");
    assert_eq!(rom(&mut h, clamp_addr + 2), JR_NC, "Expected jr nc ($30)");

    let offset = rom(&mut h, clamp_addr + 3) as i8;
    let target = (clamp_addr + 4).wrapping_add(offset as u16);
    assert_eq!(
        target, loop_addr,
        "jr nc should target .loop (${:04X}), got ${:04X}",
        loop_addr, target
    );

    assert_eq!(
        rom(&mut h, clamp_addr + 4),
        LD_B_N,
        "Expected ld b, n ($06)"
    );
    assert_eq!(rom(&mut h, clamp_addr + 5), 2, "Expected ld b, 2");
}

#[test]
fn player_ld_b_2_immediately_precedes_loop() {
    // The ld b, 2 must be the instruction immediately before .loop
    // so it falls through into the RNG loop.
    let loop_addr = sym_addr("ApplyAttackToEnemyPokemon.loop");
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    // ld b, 2 is at loop - 2 (2 bytes: 06 02)
    assert_eq!(rom(&mut h, loop_addr - 2), LD_B_N);
    assert_eq!(rom(&mut h, loop_addr - 1), 2);
}

#[test]
fn enemy_ld_b_2_immediately_precedes_loop() {
    let loop_addr = sym_addr("ApplyAttackToPlayerPokemon.loop");
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    assert_eq!(rom(&mut h, loop_addr - 2), LD_B_N);
    assert_eq!(rom(&mut h, loop_addr - 1), 2);
}
