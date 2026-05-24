//! Emulator-based tests for the "Status-curing items remove stat modifiers" fix.
//!
//! Bug: Using a status-curing item (Burn Heal, Parlyz Heal, Full Heal, Full
//! Restore, etc.) on the active battle Pokémon copies raw party stats into
//! wBattleMonStats, wiping out all stat stage modifiers (+1 through +6 /
//! −1 through −6) and badge boosts. The useless `predef DoubleOrHalveSelectedStats`
//! call that followed did nothing (variables always 0).
//!
//! Fix: Replace the CopyData + predef with a call to `.reapplyStatModsAfterCure`,
//! a new subroutine that:
//!   1. Clears BADLY_POISONED in wPlayerBattleStatus3
//!   2. Calls CalculateModifiedStats (reapplies stat stages from wPlayerMonStatMods)
//!   3. Calls ApplyBadgeStatBoosts (reapplies badge boosts)
//!
//! Both Path 1 (.cureStatusAilment — individual cures + Full Heal) and Path 2
//! (Full Restore HP healing) now call this subroutine. +9 bytes in bank $03.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// BADLY_POISONED is bit 0 of wPlayerBattleStatus3.
const BADLY_POISONED_BIT: u8 = 0;

/// Stat modifier neutral value (stage 0 = index 7 in StatModifierRatios).
const STAT_MOD_NEUTRAL: u8 = 7;

/// A safe WRAM trap address.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Write a big-endian u16 to two consecutive WRAM bytes.
fn write_u16(h: &mut TestHarness, addr: u16, val: u16) {
    h.write_mem(addr, (val >> 8) as u8);
    h.write_mem(addr + 1, val as u8);
}

/// Read a big-endian u16 from two consecutive WRAM bytes.
fn read_u16(h: &mut TestHarness, addr: u16) -> u16 {
    let hi = h.read_mem(addr) as u16;
    let lo = h.read_mem(addr + 1) as u16;
    (hi << 8) | lo
}

// ─── ROM byte verification ─────────────────────────────────────────

/// Helper: read a ROM byte from bank $03 at the given address.
fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Create a harness with the ItemUseMedicine bank selected for ROM byte reads.
fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("ItemUseMedicine"));
    h
}

#[test]
fn rom_bytes_subroutine_clears_badly_poisoned() {
    let mut h = rom_harness();
    let reapply = sym_addr("ItemUseMedicine.reapplyStatModsAfterCure");
    // $5970: ld hl, wPlayerBattleStatus3 ($D063)  →  $21 $63 $D0
    // $5973: res 0, [hl]  →  $CB $86
    assert_eq!(rom(&mut h, reapply), 0x21, "ld hl opcode");
    assert_eq!(rom(&mut h, reapply + 1), 0x63, "wPlayerBattleStatus3 lo");
    assert_eq!(rom(&mut h, reapply + 2), 0xD0, "wPlayerBattleStatus3 hi");
    assert_eq!(rom(&mut h, reapply + 3), 0xCB, "CB prefix");
    assert_eq!(rom(&mut h, reapply + 4), 0x86, "res 0, [hl]");
}

#[test]
fn rom_bytes_subroutine_callfar_calculate_modified_stats() {
    let mut h = rom_harness();
    let reapply = sym_addr("ItemUseMedicine.reapplyStatModsAfterCure");
    let calc_stats = sym_addr("CalculateModifiedStats");
    let bankswitch = sym_addr("Bankswitch");
    // callfar CalculateModifiedStats expands to:
    //   ld hl, target  →  $21 lo hi
    //   ld b, bank     →  $06 $0F
    //   call Bankswitch →  $CD lo hi
    let base = reapply + 9; // after xor a + ld [wCalculateWhoseStats], a
    assert_eq!(rom(&mut h, base), 0x21, "ld hl opcode");
    assert_eq!(
        rom(&mut h, base + 1),
        (calc_stats & 0xFF) as u8,
        "CalculateModifiedStats lo"
    );
    assert_eq!(
        rom(&mut h, base + 2),
        (calc_stats >> 8) as u8,
        "CalculateModifiedStats hi"
    );
    assert_eq!(rom(&mut h, base + 3), 0x06, "ld b opcode");
    assert_eq!(
        rom(&mut h, base + 4),
        sym_bank("CalculateModifiedStats"),
        "bank"
    );
    assert_eq!(rom(&mut h, base + 5), 0xCD, "call opcode");
    assert_eq!(
        rom(&mut h, base + 6),
        (bankswitch & 0xFF) as u8,
        "Bankswitch lo"
    );
    assert_eq!(
        rom(&mut h, base + 7),
        (bankswitch >> 8) as u8,
        "Bankswitch hi"
    );
}

#[test]
fn rom_bytes_subroutine_callfar_apply_badge_stat_boosts() {
    let mut h = rom_harness();
    let reapply = sym_addr("ItemUseMedicine.reapplyStatModsAfterCure");
    let badge_boosts = sym_addr("ApplyBadgeStatBoosts");
    // callfar ApplyBadgeStatBoosts at offset +17 in the subroutine
    let base = reapply + 17;
    assert_eq!(rom(&mut h, base), 0x21, "ld hl opcode");
    assert_eq!(
        rom(&mut h, base + 1),
        (badge_boosts & 0xFF) as u8,
        "ApplyBadgeStatBoosts lo"
    );
    assert_eq!(
        rom(&mut h, base + 2),
        (badge_boosts >> 8) as u8,
        "ApplyBadgeStatBoosts hi"
    );
    assert_eq!(rom(&mut h, base + 3), 0x06, "ld b opcode");
    assert_eq!(
        rom(&mut h, base + 4),
        sym_bank("ApplyBadgeStatBoosts"),
        "bank"
    );
}

#[test]
fn rom_bytes_subroutine_ends_with_ret() {
    let mut h = rom_harness();
    let reapply = sym_addr("ItemUseMedicine.reapplyStatModsAfterCure");
    // Subroutine is 26 bytes, ret at offset +25
    assert_eq!(
        rom(&mut h, reapply + 25),
        0xC9,
        "subroutine should end with ret"
    );
}

#[test]
fn rom_bytes_path1_calls_subroutine() {
    let mut h = rom_harness();
    let reapply = sym_addr("ItemUseMedicine.reapplyStatModsAfterCure");
    // Path 1: after `ld [wBattleMonStatus], a`, the next instruction is
    // `call .reapplyStatModsAfterCure` → $CD lo hi
    let call_addr = sym_addr("ItemUseMedicine.checkMonStatus") + 22;
    assert_eq!(rom(&mut h, call_addr), 0xCD, "call opcode");
    assert_eq!(
        rom(&mut h, call_addr + 1),
        (reapply & 0xFF) as u8,
        "subroutine addr lo"
    );
    assert_eq!(
        rom(&mut h, call_addr + 2),
        (reapply >> 8) as u8,
        "subroutine addr hi"
    );
}

#[test]
fn rom_bytes_path2_push_call_pop() {
    let mut h = rom_harness();
    let reapply = sym_addr("ItemUseMedicine.reapplyStatModsAfterCure");
    // Path 2: push de ($D5), call .reapplyStatModsAfterCure ($CD lo hi), pop de ($D1)
    let push_addr = sym_addr("ItemUseMedicine.updateInBattleData") + 28;
    assert_eq!(rom(&mut h, push_addr), 0xD5, "push de");
    assert_eq!(rom(&mut h, push_addr + 1), 0xCD, "call opcode");
    assert_eq!(
        rom(&mut h, push_addr + 2),
        (reapply & 0xFF) as u8,
        "subroutine addr lo"
    );
    assert_eq!(
        rom(&mut h, push_addr + 3),
        (reapply >> 8) as u8,
        "subroutine addr hi"
    );
    assert_eq!(rom(&mut h, push_addr + 4), 0xD1, "pop de");
}

#[test]
fn rom_bytes_no_copy_data_or_predef() {
    let mut h = rom_harness();
    // Verify Path 1 now has jp .doneHealing right after the call.
    // Path 1 in-battle section: xor a (1), ld [wBattleMonStatus] (3), call (3), jp (3)
    let jp_addr = sym_addr("ItemUseMedicine.checkMonStatus") + 25;
    assert_eq!(rom(&mut h, jp_addr), 0xC3, "jp opcode after call");
}

// ─── Behavioral tests ──────────────────────────────────────────────

/// Set up a harness ready to call `.reapplyStatModsAfterCure` directly.
///
/// Sets the ItemUseMedicine bank, hLoadedROMBank, disables interrupts/timers,
/// and places a NOP+STOP trap for return.
fn setup_subroutine_fixture() -> TestHarness {
    let bank = sym_bank("ItemUseMedicine");
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    // Trap for ret
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Not a link battle (so badge boosts apply)
    h.write_mem(sym_addr("wLinkState"), 0x00);

    h
}

/// Set all 4 unmodified stats to the same base value (big-endian).
fn set_unmodified_stats(h: &mut TestHarness, atk: u16, def: u16, spd: u16, spc: u16) {
    write_u16(h, sym_addr("wPlayerMonUnmodifiedAttack"), atk);
    write_u16(h, sym_addr("wPlayerMonUnmodifiedDefense"), def);
    write_u16(h, sym_addr("wPlayerMonUnmodifiedSpeed"), spd);
    write_u16(h, sym_addr("wPlayerMonUnmodifiedSpecial"), spc);
}

/// Set all 4 stat stage modifiers (attack, defense, speed, special).
fn set_stat_mods(h: &mut TestHarness, atk_mod: u8, def_mod: u8, spd_mod: u8, spc_mod: u8) {
    let w_player_mon_attack_mod = sym_addr("wPlayerMonAttackMod");
    h.write_mem(w_player_mon_attack_mod, atk_mod);
    h.write_mem(w_player_mon_attack_mod + 1, def_mod);
    h.write_mem(w_player_mon_attack_mod + 2, spd_mod);
    h.write_mem(w_player_mon_attack_mod + 3, spc_mod);
}

/// Run the subroutine and wait for it to return.
fn run_subroutine(h: &mut TestHarness) {
    h.set_pc(sym_addr("ItemUseMedicine.reapplyStatModsAfterCure"));
    h.step_to(TRAP_ADDR);
}

#[test]
fn subroutine_clears_badly_poisoned() {
    let mut h = setup_subroutine_fixture();
    // Set BADLY_POISONED bit
    h.write_mem(sym_addr("wPlayerBattleStatus3"), 1 << BADLY_POISONED_BIT);
    // Set neutral mods and some base stats so the function has work to do
    set_unmodified_stats(&mut h, 100, 100, 100, 100);
    set_stat_mods(
        &mut h,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
    );
    h.write_mem(sym_addr("wObtainedBadges"), 0x00);

    run_subroutine(&mut h);

    let status3 = h.read_mem(sym_addr("wPlayerBattleStatus3"));
    assert_eq!(
        status3 & (1 << BADLY_POISONED_BIT),
        0,
        "BADLY_POISONED should be cleared after cure"
    );
}

#[test]
fn subroutine_reapplies_stat_stages_neutral() {
    let mut h = setup_subroutine_fixture();
    h.write_mem(sym_addr("wPlayerBattleStatus3"), 0);
    h.write_mem(sym_addr("wObtainedBadges"), 0x00); // no badges

    // Base stats: Atk=100, Def=80, Spd=120, Spc=90
    set_unmodified_stats(&mut h, 100, 80, 120, 90);
    // All neutral (stage 0 = mod 7): multiplier 1/1
    set_stat_mods(
        &mut h,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
    );
    // Set battle stats to wrong values (simulating the old buggy copy)
    write_u16(&mut h, sym_addr("wBattleMonAttack"), 50);
    write_u16(&mut h, sym_addr("wBattleMonDefense"), 40);
    write_u16(&mut h, sym_addr("wBattleMonSpeed"), 60);
    write_u16(&mut h, sym_addr("wBattleMonSpecial"), 45);

    run_subroutine(&mut h);

    // Neutral mods → stats should equal unmodified values
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonAttack")),
        100,
        "Atk should be restored to 100"
    );
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonDefense")),
        80,
        "Def should be restored to 80"
    );
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonSpeed")),
        120,
        "Spd should be restored to 120"
    );
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonSpecial")),
        90,
        "Spc should be restored to 90"
    );
}

#[test]
fn subroutine_reapplies_stat_stages_plus2() {
    let mut h = setup_subroutine_fixture();
    h.write_mem(sym_addr("wPlayerBattleStatus3"), 0);
    h.write_mem(sym_addr("wObtainedBadges"), 0x00);

    // Base Atk=100, all others 100
    set_unmodified_stats(&mut h, 100, 100, 100, 100);
    // Attack at +2 (mod index 9): ratio = 8/4 = 2.0×
    // Defense/Speed/Special at neutral (7)
    set_stat_mods(
        &mut h,
        9,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
    );
    // Set wrong battle stats
    write_u16(&mut h, sym_addr("wBattleMonAttack"), 50);

    run_subroutine(&mut h);

    // Atk = 100 * 2.0 = 200
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonAttack")),
        200,
        "Atk at +2 should be 200 (100 × 2.0)"
    );
    // Other stats unchanged from neutral
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonDefense")),
        100,
        "Def neutral"
    );
}

#[test]
fn subroutine_reapplies_stat_stages_minus1() {
    let mut h = setup_subroutine_fixture();
    h.write_mem(sym_addr("wPlayerBattleStatus3"), 0);
    h.write_mem(sym_addr("wObtainedBadges"), 0x00);

    // Base Speed=200
    set_unmodified_stats(&mut h, 100, 100, 200, 100);
    // Speed at -1 (mod index 6): ratio = 2/3 = 0.667×
    set_stat_mods(
        &mut h,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        6,
        STAT_MOD_NEUTRAL,
    );

    run_subroutine(&mut h);

    // Spd = 200 * 66/100 = 132 (StatModifierRatios stage -1 = 66/100)
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonSpeed")),
        132,
        "Spd at -1 should be 132 (200 × 66/100)"
    );
}

#[test]
fn subroutine_applies_badge_boosts() {
    let mut h = setup_subroutine_fixture();
    h.write_mem(sym_addr("wPlayerBattleStatus3"), 0);

    // Boulder Badge (bit 0) boosts Attack by 9/8
    h.write_mem(sym_addr("wObtainedBadges"), 0x01);

    set_unmodified_stats(&mut h, 200, 100, 100, 100);
    set_stat_mods(
        &mut h,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
    );

    run_subroutine(&mut h);

    // Atk = 200 + 200/8 = 200 + 25 = 225
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonAttack")),
        225,
        "Atk with Boulder Badge should be 225 (200 × 9/8)"
    );
    // Defense should NOT be boosted (Thunder Badge is bit 2, not set)
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonDefense")),
        100,
        "Def without badge boost"
    );
}

#[test]
fn subroutine_applies_stat_stages_and_badge_boosts_combined() {
    let mut h = setup_subroutine_fixture();
    h.write_mem(sym_addr("wPlayerBattleStatus3"), 0);

    // All 4 badge boosts: Boulder (bit 0), Thunder (bit 2), Soul (bit 4), Volcano (bit 6)
    h.write_mem(sym_addr("wObtainedBadges"), 0x55); // bits 0, 2, 4, 6

    // Base Atk=100, Def=100, Spd=100, Spc=100
    set_unmodified_stats(&mut h, 100, 100, 100, 100);
    // Attack at +1 (mod 8): ratio = 3/2 = 1.5×
    set_stat_mods(
        &mut h,
        8,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
    );

    run_subroutine(&mut h);

    // Atk = floor(100 * 3/2) = 150, then badge: 150 + floor(150/8) = 150 + 18 = 168
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonAttack")),
        168,
        "Atk at +1 with Boulder Badge should be 168"
    );
    // Other stats at neutral with badges: 100 + 12 = 112
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonDefense")),
        112,
        "Def neutral with Thunder Badge should be 112"
    );
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonSpeed")),
        112,
        "Spd neutral with Soul Badge should be 112"
    );
    assert_eq!(
        read_u16(&mut h, sym_addr("wBattleMonSpecial")),
        112,
        "Spc neutral with Volcano Badge should be 112"
    );
}

#[test]
fn subroutine_preserves_other_status3_bits() {
    let mut h = setup_subroutine_fixture();
    // Set multiple bits in wPlayerBattleStatus3 including BADLY_POISONED
    h.write_mem(sym_addr("wPlayerBattleStatus3"), 0xFF);
    h.write_mem(sym_addr("wObtainedBadges"), 0x00);
    set_unmodified_stats(&mut h, 100, 100, 100, 100);
    set_stat_mods(
        &mut h,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
        STAT_MOD_NEUTRAL,
    );

    run_subroutine(&mut h);

    let status3 = h.read_mem(sym_addr("wPlayerBattleStatus3"));
    assert_eq!(
        status3 & (1 << BADLY_POISONED_BIT),
        0,
        "BADLY_POISONED bit should be cleared"
    );
    assert_eq!(
        status3 & !(1 << BADLY_POISONED_BIT),
        0xFE,
        "other bits in wPlayerBattleStatus3 should be preserved"
    );
}
