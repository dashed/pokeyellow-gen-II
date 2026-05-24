//! Emulator-based tests for the Substitute 0 HP bug fix.
//!
//! The original SubstituteEffect_ only checked for underflow (carry) when
//! subtracting quarter HP from current HP. If currentHP == quarterHP, the
//! subtraction gives 0 with no carry, so the substitute was created leaving
//! the Pokemon with 0 HP — alive but at 0 health.
//!
//! Our fix adds `ld e,a / or d / jr z, .notEnoughHP` after the carry check,
//! rejecting the substitute if the remaining HP would be exactly 0.
//!
//! Test approach: run from `.notEnemy` with WRAM set up for known HP values,
//! then check if the `.notEnoughHP` path was taken (substitute rejected) or
//! the `HAS_SUBSTITUTE_UP` bit was set (substitute accepted).

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// HAS_SUBSTITUTE_UP = bit 4.
const HAS_SUBSTITUTE_UP_BIT: u8 = 4;

/// A safe WRAM trap address.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Outcome of attempting to use Substitute.
#[derive(Debug, PartialEq)]
enum SubstituteResult {
    /// Substitute was rejected (not enough HP).
    Rejected,
    /// Substitute was accepted (HAS_SUBSTITUTE_UP set, HP subtracted).
    Accepted {
        remaining_hp: u16,
        substitute_hp: u8,
    },
}

/// Attempt to use Substitute with the given max HP and current HP.
///
/// Runs from `.notEnemy` and checks whether `.notEnoughHP` was reached
/// or the substitute was successfully created.
fn try_substitute(max_hp: u16, current_hp: u16) -> SubstituteResult {
    let not_enemy = sym_addr("SubstituteEffect_.notEnemy");
    let not_enough_hp = sym_addr("SubstituteEffect_.notEnoughHP");
    let already_has_substitute = sym_addr("SubstituteEffect_.alreadyHasSubstitute");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("SubstituteEffect_"));

    let w_battle_mon_hp = sym_addr("wBattleMonHP");
    let w_battle_mon_max_hp = sym_addr("wBattleMonMaxHP");
    let w_player_substitute_hp = sym_addr("wPlayerSubstituteHP");

    // Set up WRAM
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    // Max HP (big-endian)
    h.write_mem(w_battle_mon_max_hp, (max_hp >> 8) as u8);
    h.write_mem(w_battle_mon_max_hp + 1, max_hp as u8);

    // Current HP (big-endian)
    h.write_mem(w_battle_mon_hp, (current_hp >> 8) as u8);
    h.write_mem(w_battle_mon_hp + 1, current_hp as u8);

    // Clear battle status (no existing substitute)
    h.write_mem(sym_addr("wPlayerBattleStatus2"), 0x00);
    h.write_mem(w_player_substitute_hp, 0x00);

    // At .notEnemy entry, registers must be:
    // HL = wBattleMonMaxHP, DE = wPlayerSubstituteHP, BC = wPlayerBattleStatus2
    h.gb.cpu().set_hl(w_battle_mon_max_hp);
    h.gb.cpu().set_de(w_player_substitute_hp);
    h.gb.cpu().set_bc(sym_addr("wPlayerBattleStatus2"));

    // Write traps at both possible continuation points
    // .notEnoughHP loads HL with a text pointer then jumps to PrintText.
    // We can detect arrival by checking PC.
    // For the success path, the code writes HP then does set HAS_SUBSTITUTE_UP, [hl]
    // then loads options and calls Bankswitch — which will crash in headless.
    //
    // Strategy: use step_until to advance one instruction at a time until
    // we either reach .notEnoughHP or .alreadyHasSubstitute or see the
    // HAS_SUBSTITUTE_UP bit get set in wPlayerBattleStatus2.

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h.set_pc(not_enemy);

    // Step until we reach one of the decision points.
    // Run up to 200 instructions max to avoid infinite loops.
    for _ in 0..200 {
        let pc = h.pc();
        if pc == not_enough_hp || pc == already_has_substitute {
            return SubstituteResult::Rejected;
        }
        // Check if HAS_SUBSTITUTE_UP was set (substitute accepted)
        if h.read_mem(sym_addr("wPlayerBattleStatus2")) & (1 << HAS_SUBSTITUTE_UP_BIT) != 0 {
            let hp_high = h.read_mem(w_battle_mon_hp) as u16;
            let hp_low = h.read_mem(w_battle_mon_hp + 1) as u16;
            return SubstituteResult::Accepted {
                remaining_hp: (hp_high << 8) | hp_low,
                substitute_hp: h.read_mem(w_player_substitute_hp),
            };
        }
        h.gb.clock();
    }
    panic!("SubstituteEffect_ did not reach a decision point within 200 instructions");
}

// ─── Substitute 0 HP fix ────────────────────────────────────────────

#[test]
fn substitute_rejected_when_hp_equals_quarter() {
    // Max HP = 100, quarter = 25. Current HP = 25 → would leave 0 HP.
    // The fix should reject this.
    let result = try_substitute(100, 25);
    assert_eq!(
        result,
        SubstituteResult::Rejected,
        "Substitute should be rejected when HP == maxHP/4 (would leave 0 HP)"
    );
}

#[test]
fn substitute_rejected_when_hp_less_than_quarter() {
    // Max HP = 100, quarter = 25. Current HP = 20 → not enough.
    let result = try_substitute(100, 20);
    assert_eq!(
        result,
        SubstituteResult::Rejected,
        "Substitute should be rejected when HP < maxHP/4"
    );
}

#[test]
fn substitute_accepted_when_hp_above_quarter() {
    // Max HP = 100, quarter = 25. Current HP = 26 → leaves 1 HP.
    let result = try_substitute(100, 26);
    assert_eq!(
        result,
        SubstituteResult::Accepted {
            remaining_hp: 1,
            substitute_hp: 25,
        },
        "Substitute should succeed when HP > maxHP/4"
    );
}

#[test]
fn substitute_accepted_at_full_hp() {
    // Max HP = 200, quarter = 50. Current HP = 200 → leaves 150 HP.
    let result = try_substitute(200, 200);
    assert_eq!(
        result,
        SubstituteResult::Accepted {
            remaining_hp: 150,
            substitute_hp: 50,
        },
        "Substitute should succeed at full HP"
    );
}

#[test]
fn substitute_quarter_hp_rounds_down() {
    // Max HP = 103, quarter = 25 (103/4 = 25.75, rounds down).
    // Current HP = 26 → leaves 1 HP.
    let result = try_substitute(103, 26);
    assert_eq!(
        result,
        SubstituteResult::Accepted {
            remaining_hp: 1,
            substitute_hp: 25,
        },
        "Quarter HP should round down (103/4 = 25)"
    );
}

#[test]
fn substitute_rejected_at_1_hp() {
    // Max HP = 100, quarter = 25. Current HP = 1.
    let result = try_substitute(100, 1);
    assert_eq!(
        result,
        SubstituteResult::Rejected,
        "Substitute should be rejected at 1 HP"
    );
}

#[test]
fn substitute_with_high_max_hp() {
    // Max HP = 500, quarter = 125. Current HP = 126 → leaves 1 HP.
    let result = try_substitute(500, 126);
    assert_eq!(
        result,
        SubstituteResult::Accepted {
            remaining_hp: 1,
            substitute_hp: 125,
        },
        "Substitute should work with high max HP values"
    );
}

#[test]
fn substitute_with_high_max_hp_rejected_at_quarter() {
    // Max HP = 500, quarter = 125. Current HP = 125 → would leave 0 HP.
    let result = try_substitute(500, 125);
    assert_eq!(
        result,
        SubstituteResult::Rejected,
        "Substitute should be rejected when HP == maxHP/4 for high HP values"
    );
}
