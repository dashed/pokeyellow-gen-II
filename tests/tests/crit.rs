//! Emulator-based tests for critical hit bug fixes:
//!
//! 1. **1/256 crit miss fix**: CriticalHitTest used `random < crit_rate` (strictly
//!    less than). When crit rate = 255, `255 < 255 = false` prevents a guaranteed
//!    crit 1/256 of the time. Fixed with `inc a / jr z` bypass at `.SkipHighCritical`.
//!
//! 2. **Focus Energy fix**: Focus Energy/Dire Hit used `srl b` (÷2) instead of
//!    `sla b` (×2), quartering the crit rate instead of quadrupling it. Fixed by
//!    swapping the branch condition (`jr nz` → `jr z`) so the ×2 path runs when
//!    Focus Energy is active.
//!
//! Test approaches:
//! - 1/256 tests: inject B at `.SkipHighCritical` with link battle deterministic RNG
//! - Focus Energy tests: run from `.calcCriticalHitProbability` with known B/move/status,
//!   trap at `.SkipHighCritical` to read the computed crit threshold in register B

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// LINK_STATE_BATTLING = $04.
const LINK_STATE_BATTLING: u8 = 0x04;
/// GETTING_PUMPED = bit 2 of wPlayerBattleStatus2.
const GETTING_PUMPED_BIT: u8 = 2;

/// Move IDs for testing high-crit vs normal moves.
const SLASH: u8 = 0xA3; // high critical hit move
const POUND: u8 = 0x01; // normal move (not in HighCriticalMoves table)

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

// ─── Focus Energy fix tests ─────────────────────────────────────────
//
// Tests run from `.calcCriticalHitProbability` with B pre-set to base_speed/2,
// then trap at `.SkipHighCritical` to read the computed crit threshold in B.
//
// The Focus Energy fix swaps the branch condition so:
//   With Focus Energy:    sla b (×2) → quadruples the final crit rate
//   Without Focus Energy: srl b (÷2) → intended normal rate
//
// Expected crit thresholds (B = base_speed/2 at entry):
//
// | Case                  | FE check | High-crit check | Final B     |
// |-----------------------|----------|-----------------|-------------|
// | No FE, normal move    | ÷2       | ÷2              | speed/8     |
// | No FE, high-crit move | ÷2       | ×4              | speed       |
// | FE, normal move       | ×2       | ÷2              | speed/2     |
// | FE, high-crit move    | ×2       | ×4              | speed×4     |

/// Compute the crit threshold by running from `.calcCriticalHitProbability`
/// to `.SkipHighCritical`. Returns register B at `.SkipHighCritical`.
fn calc_crit_threshold(half_speed: u8, move_id: u8, focus_energy: bool) -> u8 {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("CriticalHitTest"));

    // Write STOP at .SkipHighCritical to trap execution there
    // (we're in RAM-overlaid ROM area, but SkipHighCritical is in banked ROM
    //  so we write the trap at a WRAM location and redirect PC)
    // Actually, we can't write to ROM. Instead, use step_to with the address.
    // step_to checks PC each instruction, so it will stop AT SkipHighCritical.

    // Set up WRAM: move power (non-zero) and move ID
    let w_player_move_power = sym_addr("wPlayerMovePower");
    h.write_mem(w_player_move_power, 70); // any non-zero power
    h.write_mem(sym_addr("wPlayerMoveNum"), move_id);

    // Set Focus Energy status
    let status = if focus_energy {
        1 << GETTING_PUMPED_BIT
    } else {
        0
    };
    h.write_mem(sym_addr("wPlayerBattleStatus2"), status);

    // At .calcCriticalHitProbability entry:
    // HL = wPlayerMovePower, DE = wPlayerBattleStatus2
    h.gb.cpu().set_hl(w_player_move_power);
    h.gb.cpu().set_de(sym_addr("wPlayerBattleStatus2"));
    h.set_b(half_speed);

    // Stack for ret (in case code returns early — shouldn't happen with power > 0)
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h.set_pc(sym_addr("CriticalHitTest.calcCriticalHitProbability"));
    h.step_to(sym_addr("CriticalHitTest.SkipHighCritical"));

    h.b()
}

#[test]
fn focus_energy_quadruples_normal_move_crit_rate() {
    // B = 64 (base_speed/2, representing speed 128)
    // No FE, normal: 64 → srl → 32 → srl → 16
    // With FE, normal: 64 → sla → 128 → srl → 64
    let no_fe = calc_crit_threshold(64, POUND, false);
    let with_fe = calc_crit_threshold(64, POUND, true);

    assert_eq!(no_fe, 16, "no FE, normal move: speed/8 = 128/8 = 16");
    assert_eq!(with_fe, 64, "with FE, normal move: speed/2 = 128/2 = 64");
    assert_eq!(
        with_fe / no_fe,
        4,
        "Focus Energy should quadruple crit rate: {with_fe}/{no_fe}"
    );
}

#[test]
fn focus_energy_quadruples_high_crit_move_rate() {
    // B = 32 (base_speed/2, representing speed 64)
    // No FE, high-crit: 32 → srl → 16 → sla×2 → 64
    // With FE, high-crit: 32 → sla → 64 → sla×2 → 256→cap 255
    // (capped at 255, but ratio shows intent)
    let no_fe = calc_crit_threshold(32, SLASH, false);
    let with_fe = calc_crit_threshold(32, SLASH, true);

    assert_eq!(no_fe, 64, "no FE, high-crit: speed = 64");
    assert_eq!(with_fe, 255, "with FE, high-crit: speed×4 capped = 255");
    assert!(
        with_fe > no_fe,
        "FE + high-crit should exceed non-FE: {with_fe} > {no_fe}"
    );
}

#[test]
fn no_focus_energy_normal_move_gives_speed_over_8() {
    // B = 100 (speed 200). No FE, normal: 100 → srl → 50 → srl → 25
    let b = calc_crit_threshold(100, POUND, false);
    assert_eq!(b, 25, "no FE, normal: base_speed/8 = 200/8 = 25");
}

#[test]
fn no_focus_energy_high_crit_gives_speed() {
    // B = 50 (speed 100). No FE, high-crit: 50 → srl → 25 → sla×2 → 100
    let b = calc_crit_threshold(50, SLASH, false);
    assert_eq!(b, 100, "no FE, high-crit: base_speed = 100");
}

#[test]
fn focus_energy_with_max_speed_normal_caps_at_127() {
    // B = 127 (speed 254). FE, normal: 127 → sla → 254 → srl → 127
    let b = calc_crit_threshold(127, POUND, true);
    assert_eq!(b, 127, "FE, normal, speed 254: 254/2 = 127");
}

#[test]
fn focus_energy_with_max_speed_high_crit_caps_at_255() {
    // B = 127 (speed 254). FE, high-crit: 127 → sla → 254 → sla×2 → capped 255
    let b = calc_crit_threshold(127, SLASH, true);
    assert_eq!(b, 255, "FE, high-crit, speed 254: capped at 255");
}
