//! Emulator-based tests for the 1/256 critical hit miss bug fix.
//!
//! CriticalHitTest used `random < crit_rate` (strictly less than). When
//! crit rate = 255, `255 < 255 = false` prevents a guaranteed crit 1/256 of
//! the time. Fixed with `inc a / jr z` bypass at `.SkipHighCritical`.
//!
//! Test approach: inject B at `.SkipHighCritical` with link battle deterministic RNG.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// LINK_STATE_BATTLING = $04.
const LINK_STATE_BATTLING: u8 = 0x04;

/// A safe WRAM address for the return trap.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Set up the harness for a crit check at `.SkipHighCritical`.
///
/// Uses link battle mode so BattleRandom reads from a controllable RNG list
/// instead of hardware RNG.
fn setup_crit_fixture(rng_list: &[u8; 10]) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    h.select_rom_bank(sym_bank("CriticalHitTest"));

    // Write NOP + STOP at trap address
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    // Stack setup
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Player's turn
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    // Enable link battle mode so BattleRandom uses the list
    h.write_mem(sym_addr("wLinkState"), LINK_STATE_BATTLING);

    // Fill the RNG list with known values
    let rng_list_addr = sym_addr("wLinkBattleRandomNumberList");
    for (i, &val) in rng_list.iter().enumerate() {
        h.write_mem(rng_list_addr + i as u16, val);
    }
    h.write_mem(sym_addr("wLinkBattleRandomNumberListIndex"), 0x00);

    h
}

/// Run a crit check from `.SkipHighCritical` with the given crit rate.
/// Returns `true` if a critical hit occurred.
fn check_crit(h: &mut TestHarness, crit_rate: u8) -> bool {
    let w_crit = sym_addr("wCriticalHitOrOHKO");
    h.write_mem(w_crit, 0x00);
    h.set_b(crit_rate);
    h.set_pc(sym_addr("CriticalHitTest.SkipHighCritical"));

    // Reset SP so ret works
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    h.step_to(TRAP_ADDR);
    h.read_mem(w_crit) == 1
}

// ─── 1/256 crit fix: crit rate 255 always crits ─────────────────────

#[test]
fn crit_rate_255_always_crits() {
    // With crit rate 255, BattleRandom should never be called.
    // Fill the RNG list with 255 (worst case for the old bug).
    let list: [u8; 10] = [255; 10];
    let mut h = setup_crit_fixture(&list);

    assert!(
        check_crit(&mut h, 255),
        "crit rate 255 should ALWAYS crit (1/256 bug fix)"
    );

    // RNG index should NOT advance since BattleRandom was bypassed
    assert_eq!(
        h.read_mem(sym_addr("wLinkBattleRandomNumberListIndex")),
        0,
        "crit rate 255 should bypass BattleRandom (index unchanged)"
    );
}

#[test]
fn crit_rate_255_bypasses_rng_for_all_list_values() {
    // Verify that regardless of what the RNG list contains,
    // crit rate 255 always produces a critical hit.
    for rng_val in [0u8, 1, 127, 128, 254, 255] {
        let list: [u8; 10] = [rng_val; 10];
        let mut h = setup_crit_fixture(&list);

        assert!(
            check_crit(&mut h, 255),
            "crit rate 255 must always crit regardless of RNG list value {rng_val}"
        );
    }
}

// ─── Normal crit behavior still works ────────────────────────────────

#[test]
fn crit_rate_200_random_50_crits() {
    // rotated(50) = 0b00110010 rlc×3 = 0b10010001 = 145
    // 145 < 200 → crit
    let list: [u8; 10] = [50, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_crit_fixture(&list);

    assert!(
        check_crit(&mut h, 200),
        "rotated random (from 50) < 200 should crit"
    );
}

#[test]
fn crit_rate_10_random_200_no_crit() {
    // rotated(200) = 0b11001000 rlc×3 = 0b01000110 = 70
    // 70 >= 10 → no crit
    let list: [u8; 10] = [200, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_crit_fixture(&list);

    assert!(
        !check_crit(&mut h, 10),
        "rotated random (from 200) >= 10 should NOT crit"
    );
}

#[test]
fn crit_rate_1_random_0_crits() {
    // rotated(0) = 0. 0 < 1 → crit
    let list: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_crit_fixture(&list);

    assert!(
        check_crit(&mut h, 1),
        "rotated random 0 < crit rate 1 should crit"
    );
}

#[test]
fn crit_rate_1_random_1_no_crit() {
    // rotated(1) = 0b00000001 rlc×3 = 0b00001000 = 8
    // 8 >= 1 → no crit
    let list: [u8; 10] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut h = setup_crit_fixture(&list);

    assert!(
        !check_crit(&mut h, 1),
        "rotated random (from 1) = 8 >= crit rate 1 should NOT crit"
    );
}
