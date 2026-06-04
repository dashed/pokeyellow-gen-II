//! Link cable regression tests — verify the accuracy fix works in link battle mode.
//!
//! In link battles, `BattleRandom` reads from `wLinkBattleRandomNumberList`
//! (a pre-shared list of 10 bytes) instead of hardware RNG. Our accuracy fix
//! (the `bit 7, b` optimal rounding at `doAccuracyCheck`) must work correctly
//! with either RNG source.
//!
//! These tests run the FULL `doAccuracyCheck` code path (including the
//! `call BattleRandom` that reads from the link RNG list), unlike the
//! Strategy C accuracy tests which inject register A to bypass BattleRandom.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// LINK_STATE_BATTLING = $04.
const LINK_STATE_BATTLING: u8 = 0x04;

/// A safe WRAM address for the return trap.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Set up the harness for a link-mode accuracy check.
///
/// Unlike the Strategy C fixture (which bypasses BattleRandom), this runs
/// the full code path: `doAccuracyCheck` → `call BattleRandom` → reads
/// from `wLinkBattleRandomNumberList`.
fn setup_link_accuracy_fixture(rng_list: &[u8; 10]) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    h.select_rom_bank(sym_bank("MoveHitTest"));

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

/// Run a link-mode accuracy check. Returns `true` if the move hits.
///
/// BattleRandom reads the next value from wLinkBattleRandomNumberList
/// (at the current index) and compares it against the accuracy value.
fn check_link_accuracy(h: &mut TestHarness, accuracy: u8) -> bool {
    h.write_mem(sym_addr("wMoveMissed"), 0x00);
    h.set_b(accuracy);
    h.set_pc(sym_addr("MoveHitTest.doAccuracyCheck"));

    // Reset SP so ret works (push_word was called in setup)
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    h.step_to(TRAP_ADDR);
    h.read_mem(sym_addr("wMoveMissed")) == 0
}

// ─── Scenario 17: Link battle accuracy regression ────────────────

#[test]
fn link_battle_rng_list_hit_miss() {
    // Pre-fill list: [50, 200, 128, 127, 0, 254, 100, 150, 250, 0]
    let list: [u8; 10] = [50, 200, 128, 127, 0, 254, 100, 150, 250, 0];
    let mut h = setup_link_accuracy_fixture(&list);

    // Test 1: accuracy=200, random=50 (list[0]): 50 < 200 → hit
    assert!(
        check_link_accuracy(&mut h, 200),
        "random=50 < accuracy=200 should hit"
    );

    // Test 2: accuracy=100, random=200 (list[1]): 200 > 100 → miss
    assert!(
        !check_link_accuracy(&mut h, 100),
        "random=200 > accuracy=100 should miss"
    );

    // Test 3: accuracy=128, random=128 (list[2]): equal, bit 7 set → hit (optimal rounding)
    assert!(
        check_link_accuracy(&mut h, 128),
        "random==accuracy=128: bit 7 set → should HIT (optimal rounding)"
    );

    // Test 4: accuracy=127, random=127 (list[3]): equal, bit 7 clear → miss (optimal rounding)
    assert!(
        !check_link_accuracy(&mut h, 127),
        "random==accuracy=127: bit 7 clear → should MISS (optimal rounding)"
    );

    // Test 5: accuracy=1, random=0 (list[4]): 0 < 1 → hit
    assert!(
        check_link_accuracy(&mut h, 1),
        "random=0 < accuracy=1 should hit"
    );

    // Test 6: accuracy=254, random=254 (list[5]): equal, bit 7 set → hit
    assert!(
        check_link_accuracy(&mut h, 254),
        "random==accuracy=254: bit 7 set → should HIT"
    );
}

#[test]
fn link_battle_rng_index_advances() {
    let list: [u8; 10] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    let mut h = setup_link_accuracy_fixture(&list);
    let w_rng_index = sym_addr("wLinkBattleRandomNumberListIndex");

    // Index starts at 0
    assert_eq!(h.read_mem(w_rng_index), 0);

    // First call reads list[0], index becomes 1
    check_link_accuracy(&mut h, 200);
    assert_eq!(
        h.read_mem(w_rng_index),
        1,
        "RNG list index should advance to 1 after first call"
    );

    // Second call reads list[1], index becomes 2
    check_link_accuracy(&mut h, 200);
    assert_eq!(
        h.read_mem(w_rng_index),
        2,
        "RNG list index should advance to 2 after second call"
    );
}

#[test]
fn link_battle_255_always_hits() {
    // N=255 always hits via `cp $FF / ret z` — BattleRandom is never called
    let list: [u8; 10] = [0; 10]; // list contents don't matter
    let mut h = setup_link_accuracy_fixture(&list);

    assert!(
        check_link_accuracy(&mut h, 255),
        "N=255 should always hit (ret z before BattleRandom)"
    );

    // Index should NOT advance since BattleRandom was never called
    assert_eq!(
        h.read_mem(sym_addr("wLinkBattleRandomNumberListIndex")),
        0,
        "N=255 should not call BattleRandom (index unchanged)"
    );
}
