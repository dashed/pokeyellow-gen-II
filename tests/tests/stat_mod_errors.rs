//! ROM byte tests for the stat modification errors fix.
//!
//! Three bugs in StatModifierUpEffect and StatModifierDownEffect:
//! 1. ApplyBadgeStatBoosts re-applied badge boosts to ALL stats (stacking)
//! 2. QuarterSpeedDueToParalysis applied to the wrong Pokémon
//! 3. HalveAttackDueToBurn applied to the wrong Pokémon
//!
//! Fix: remove all three erroneous calls. The individual stat is already
//! correctly recalculated from unmodified base × stage ratio.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Stat_modification_errors>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const LD_HL_NN: u8 = 0x21; // ld hl, nn
const CALL_NN: u8 = 0xCD; // call nn
const RET: u8 = 0xC9; // ret

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn effects_are_in_bank_0f() {
    assert_eq!(
        sym_bank("StatModifierUpEffect"),
        0x0F,
        "StatModifierUpEffect should be in bank $0F"
    );
}

#[test]
fn stat_up_no_badge_boost_call() {
    // .applyBadgeBoostsAndStatusPenalties in StatModifierUpEffect should
    // be: ld hl, MonsStatsRoseText / call PrintText / ret
    // NO call to ApplyBadgeStatBoosts, QuarterSpeedDueToParalysis, or HalveAttackDueToBurn
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let addr = sym_addr("UpdateStatDone.applyBadgeBoostsAndStatusPenalties");

    // ld hl, nn
    assert_eq!(rom(&mut h, addr), LD_HL_NN);
    // call PrintText
    assert_eq!(rom(&mut h, addr + 3), CALL_NN);
    // ret (NOT another call or jp)
    assert_eq!(
        rom(&mut h, addr + 6),
        RET,
        "Expected ret after PrintText — no badge boost or status penalty calls"
    );
}

#[test]
fn stat_down_no_badge_boost_call() {
    // Same check for StatModifierDownEffect
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let addr = sym_addr("UpdateLoweredStatDone.ApplyBadgeBoostsAndStatusPenalties");

    assert_eq!(rom(&mut h, addr), LD_HL_NN);
    assert_eq!(rom(&mut h, addr + 3), CALL_NN);
    assert_eq!(
        rom(&mut h, addr + 6),
        RET,
        "Expected ret after PrintText — no badge boost or status penalty calls"
    );
}

#[test]
fn stat_up_does_not_call_badge_boosts() {
    // Verify ApplyBadgeStatBoosts address is NOT called anywhere in the
    // 7-byte .applyBadgeBoostsAndStatusPenalties block.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let block_addr = sym_addr("UpdateStatDone.applyBadgeBoostsAndStatusPenalties");
    let badge_addr = sym_addr("ApplyBadgeStatBoosts");
    let badge_lo = (badge_addr & 0xFF) as u8;
    let badge_hi = (badge_addr >> 8) as u8;

    // Check that the badge boost address doesn't appear in the block
    for offset in 0..5 {
        let a = block_addr + offset;
        if rom(&mut h, a) == badge_lo && rom(&mut h, a + 1) == badge_hi {
            panic!(
                "Found ApplyBadgeStatBoosts address (${:04X}) at offset {} in stat-up block — should have been removed",
                badge_addr, offset
            );
        }
    }
}

#[test]
fn stat_down_does_not_call_badge_boosts() {
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let block_addr = sym_addr("UpdateLoweredStatDone.ApplyBadgeBoostsAndStatusPenalties");
    let badge_addr = sym_addr("ApplyBadgeStatBoosts");
    let badge_lo = (badge_addr & 0xFF) as u8;
    let badge_hi = (badge_addr >> 8) as u8;

    for offset in 0..5 {
        let a = block_addr + offset;
        if rom(&mut h, a) == badge_lo && rom(&mut h, a + 1) == badge_hi {
            panic!(
                "Found ApplyBadgeStatBoosts address (${:04X}) at offset {} in stat-down block — should have been removed",
                badge_addr, offset
            );
        }
    }
}
