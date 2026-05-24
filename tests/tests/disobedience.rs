//! Behavioral emulator tests for CheckForDisobedience (engine/battle/core.asm).
//!
//! CheckForDisobedience determines whether a traded Pokemon obeys the player.
//! Returns Z flag clear (NZ) = move allowed, Z flag set (Z) = move denied.
//!
//! Rules:
//! - Own Pokemon (OT ID == player ID): always obeys
//! - Link battle: always obeys
//! - Traded Pokemon: badge-gated level thresholds
//!   - No badges: level 10
//!   - Cascade Badge (bit 1): level 30
//!   - Rainbow Badge (bit 3): level 50
//!   - Marsh Badge (bit 5): level 70
//!   - Earth Badge (bit 7): level 101 (effectively always obeys)
//! - Level > threshold: random chance of nap, do nothing, hit self, or random move

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

const LINK_STATE_BATTLING: u8 = 0x04;
const PARTYMON_STRUCT_LENGTH: u16 = 0x2C;
const STRUGGLE: u8 = 0xA5;

const BADGE_CASCADE: u8 = 1 << 1;
const BADGE_RAINBOW: u8 = 1 << 3;
const BADGE_MARSH: u8 = 1 << 5;
const BADGE_EARTH: u8 = 1 << 7;

fn setup_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("CheckForDisobedience");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h
}

fn setup_traded(h: &mut TestHarness, level: u8, badges: u8) {
    h.write_mem(sym_addr("wLinkState"), 0x00);
    h.write_mem(sym_addr("wPlayerMonNumber"), 0x00);
    h.write_mem(sym_addr("wPlayerID"), 0x12);
    h.write_mem(sym_addr("wPlayerID") + 1, 0x34);
    let ot_id = sym_addr("wPartyMon1OTID");
    h.write_mem(ot_id, 0x56);
    h.write_mem(ot_id + 1, 0x78);
    h.write_mem(sym_addr("wBattleMonLevel"), level);
    h.write_mem(sym_addr("wObtainedBadges"), badges);
}

fn setup_own(h: &mut TestHarness, level: u8, slot: u8) {
    h.write_mem(sym_addr("wLinkState"), 0x00);
    h.write_mem(sym_addr("wPlayerMonNumber"), slot);
    h.write_mem(sym_addr("wPlayerID"), 0x12);
    h.write_mem(sym_addr("wPlayerID") + 1, 0x34);
    let ot_id = sym_addr("wPartyMon1OTID") + slot as u16 * PARTYMON_STRUCT_LENGTH;
    h.write_mem(ot_id, 0x12);
    h.write_mem(ot_id + 1, 0x34);
    h.write_mem(sym_addr("wBattleMonLevel"), level);
    h.write_mem(sym_addr("wObtainedBadges"), 0x00);
}

/// Run CheckForDisobedience to completion via ret → TRAP_ADDR.
/// Only safe for deterministic paths (link battle, own pokemon, level ≤ threshold).
fn run_check(h: &mut TestHarness) -> bool {
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience"));
    h.step_to(TRAP_ADDR);
    !h.gb.cpu_i().zero()
}

// ─── Link battle ───────────────────────────────────────────────────

#[test]
fn link_battle_always_obeys() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 100, 0x00);
    h.write_mem(sym_addr("wLinkState"), LINK_STATE_BATTLING);
    assert!(run_check(&mut h), "link battle should always obey");
}

// ─── Own Pokemon ───────────────────────────────────────────────────

#[test]
fn own_pokemon_always_obeys() {
    let mut h = setup_fixture();
    setup_own(&mut h, 100, 0);
    assert!(run_check(&mut h), "own Pokemon always obeys");
}

#[test]
fn own_pokemon_second_slot_obeys() {
    let mut h = setup_fixture();
    setup_own(&mut h, 100, 1);
    assert!(run_check(&mut h), "own Pokemon in party slot 1 should obey");
}

#[test]
fn partial_ot_id_match_still_treated_as_traded() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("wLinkState"), 0x00);
    h.write_mem(sym_addr("wPlayerMonNumber"), 0x00);
    h.write_mem(sym_addr("wPlayerID"), 0x12);
    h.write_mem(sym_addr("wPlayerID") + 1, 0x34);
    let ot_id = sym_addr("wPartyMon1OTID");
    h.write_mem(ot_id, 0x12); // high byte matches
    h.write_mem(ot_id + 1, 0xFF); // low byte differs
    h.write_mem(sym_addr("wBattleMonLevel"), 11); // above no-badge threshold
    h.write_mem(sym_addr("wObtainedBadges"), 0x00);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience"));
    h.step_to(sym_addr("CheckForDisobedience.loop1"));
    assert_eq!(
        h.pc(),
        sym_addr("CheckForDisobedience.loop1"),
        "partial OT ID match (high byte only) should be treated as traded"
    );
}

// ─── No badges: threshold 10 ──────────────────────────────────────

#[test]
fn no_badges_level_below_threshold_obeys() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 5, 0x00);
    assert!(
        run_check(&mut h),
        "traded level 5 with no badges (threshold 10) should obey"
    );
}

#[test]
fn no_badges_level_at_threshold_obeys() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 10, 0x00);
    assert!(
        run_check(&mut h),
        "traded level 10 at exact threshold should obey"
    );
}

// ─── Badge thresholds ──────────────────────────────────────────────

#[test]
fn cascade_badge_raises_threshold_to_30() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 25, BADGE_CASCADE);
    assert!(
        run_check(&mut h),
        "cascade badge (threshold 30) should allow level 25"
    );
}

#[test]
fn rainbow_badge_raises_threshold_to_50() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 45, BADGE_RAINBOW);
    assert!(
        run_check(&mut h),
        "rainbow badge (threshold 50) should allow level 45"
    );
}

#[test]
fn marsh_badge_raises_threshold_to_70() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 65, BADGE_MARSH);
    assert!(
        run_check(&mut h),
        "marsh badge (threshold 70) should allow level 65"
    );
}

#[test]
fn earth_badge_allows_any_level() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 100, BADGE_EARTH);
    assert!(
        run_check(&mut h),
        "earth badge (threshold 101) should allow level 100"
    );
}

#[test]
fn highest_badge_determines_threshold() {
    let mut h = setup_fixture();
    setup_traded(
        &mut h,
        100,
        BADGE_CASCADE | BADGE_RAINBOW | BADGE_MARSH | BADGE_EARTH,
    );
    assert!(
        run_check(&mut h),
        "all badges should use earth badge threshold (101)"
    );
}

// ─── Disobedience path entry ───────────────────────────────────────

#[test]
fn above_threshold_enters_random_check() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 11, 0x00);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience"));
    h.step_to(sym_addr("CheckForDisobedience.loop1"));
    assert_eq!(
        h.pc(),
        sym_addr("CheckForDisobedience.loop1"),
        "traded level 11 with no badges (threshold 10) should enter random check"
    );
}

#[test]
fn cascade_badge_level_31_enters_random_check() {
    let mut h = setup_fixture();
    setup_traded(&mut h, 31, BADGE_CASCADE);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience"));
    h.step_to(sym_addr("CheckForDisobedience.loop1"));
    assert_eq!(
        h.pc(),
        sym_addr("CheckForDisobedience.loop1"),
        "traded level 31 with cascade badge (threshold 30) should enter random check"
    );
}

// ─── State initialization ──────────────────────────────────────────

#[test]
fn clears_disobedient_flag_on_entry() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("wMonIsDisobedient"), 0x01);
    setup_own(&mut h, 50, 0);
    run_check(&mut h);
    assert_eq!(
        h.read_mem(sym_addr("wMonIsDisobedient")),
        0x00,
        "wMonIsDisobedient should be cleared to 0 on entry"
    );
}

// ─── Nap outcome ───────────────────────────────────────────────────

#[test]
fn nap_sets_sleep_status() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("wLinkState"), 0x00);
    h.write_mem(sym_addr("wBattleMonStatus"), 0x00);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience.monNaps"));
    h.step_to(sym_addr("CheckForDisobedience.printText"));
    let status = h.read_mem(sym_addr("wBattleMonStatus"));
    assert!(
        (1..=7).contains(&status),
        "nap should set sleep turns 1-7 in wBattleMonStatus, got {status}"
    );
}

// ─── Do nothing outcome ────────────────────────────────────────────

#[test]
fn do_nothing_reaches_print_text() {
    let mut h = setup_fixture();
    h.write_mem(sym_addr("wLinkState"), 0x00);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience.monDoesNothing"));
    h.step_to(sym_addr("CheckForDisobedience.printText"));
    assert_eq!(
        h.pc(),
        sym_addr("CheckForDisobedience.printText"),
        "monDoesNothing should reach printText"
    );
}

// ─── Use random move guards ───────────────────────────────────────

#[test]
fn use_random_move_requires_second_move() {
    let mut h = setup_fixture();
    let moves = sym_addr("wBattleMonMoves");
    h.write_mem(moves, 0x01);
    h.write_mem(moves + 1, 0x00); // empty second slot
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience.useRandomMove"));
    h.step_to(sym_addr("CheckForDisobedience.monDoesNothing"));
    assert_eq!(
        h.pc(),
        sym_addr("CheckForDisobedience.monDoesNothing"),
        "only one move should redirect to monDoesNothing"
    );
}

#[test]
fn use_random_move_blocked_when_disabled() {
    let mut h = setup_fixture();
    let moves = sym_addr("wBattleMonMoves");
    h.write_mem(moves, 0x01);
    h.write_mem(moves + 1, 0x02);
    h.write_mem(sym_addr("wPlayerDisabledMoveNumber"), 0x01);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience.useRandomMove"));
    h.step_to(sym_addr("CheckForDisobedience.monDoesNothing"));
    assert_eq!(
        h.pc(),
        sym_addr("CheckForDisobedience.monDoesNothing"),
        "disabled move should redirect to monDoesNothing"
    );
}

#[test]
fn use_random_move_blocked_during_struggle() {
    let mut h = setup_fixture();
    let moves = sym_addr("wBattleMonMoves");
    h.write_mem(moves, 0x01);
    h.write_mem(moves + 1, 0x02);
    h.write_mem(sym_addr("wPlayerDisabledMoveNumber"), 0x00);
    h.write_mem(sym_addr("wPlayerSelectedMove"), STRUGGLE);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience.useRandomMove"));
    h.step_to(sym_addr("CheckForDisobedience.monDoesNothing"));
    assert_eq!(
        h.pc(),
        sym_addr("CheckForDisobedience.monDoesNothing"),
        "struggling should redirect to monDoesNothing"
    );
}

#[test]
fn use_random_move_blocked_when_only_one_pp() {
    let mut h = setup_fixture();
    let moves = sym_addr("wBattleMonMoves");
    h.write_mem(moves, 0x01);
    h.write_mem(moves + 1, 0x02);
    h.write_mem(moves + 2, 0x00);
    h.write_mem(moves + 3, 0x00);
    h.write_mem(sym_addr("wPlayerDisabledMoveNumber"), 0x00);
    h.write_mem(sym_addr("wPlayerSelectedMove"), 0x01);
    let pp = sym_addr("wBattleMonPP");
    h.write_mem(pp, 10);
    h.write_mem(pp + 1, 0);
    h.write_mem(pp + 2, 0);
    h.write_mem(pp + 3, 0);
    h.write_mem(sym_addr("wCurrentMenuItem"), 0x00);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience.useRandomMove"));
    h.step_to(sym_addr("CheckForDisobedience.monDoesNothing"));
    assert_eq!(
        h.pc(),
        sym_addr("CheckForDisobedience.monDoesNothing"),
        "only one move with PP should redirect to monDoesNothing"
    );
}

#[test]
fn use_random_move_sets_disobedient_flag() {
    let mut h = setup_fixture();
    let moves = sym_addr("wBattleMonMoves");
    h.write_mem(moves, 0x01);
    h.write_mem(moves + 1, 0x02);
    h.write_mem(moves + 2, 0x00);
    h.write_mem(moves + 3, 0x00);
    h.write_mem(sym_addr("wPlayerDisabledMoveNumber"), 0x00);
    h.write_mem(sym_addr("wPlayerSelectedMove"), 0x01);
    let pp = sym_addr("wBattleMonPP");
    h.write_mem(pp, 10);
    h.write_mem(pp + 1, 10);
    h.write_mem(pp + 2, 0);
    h.write_mem(pp + 3, 0);
    h.write_mem(sym_addr("wCurrentMenuItem"), 0x00);
    h.write_mem(sym_addr("wMonIsDisobedient"), 0x00);
    h.write_mem(sym_addr("wMaxMenuItem"), 2);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CheckForDisobedience.useRandomMove"));
    h.step_to(sym_addr("CheckForDisobedience.chooseMove"));
    assert_eq!(
        h.read_mem(sym_addr("wMonIsDisobedient")),
        1,
        "passing all guards should set wMonIsDisobedient to 1"
    );
}
