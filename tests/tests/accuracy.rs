//! Emulator-based tests for the 1/256 miss glitch fix and optimal rounding.
//!
//! Uses Strategy C (breakpoint + register injection): run to the `cp b`
//! instruction at $0F:$6800 (immediately after `call BattleRandom`), inject
//! register A with the desired random value, then continue to hit/miss.
//!
//! The modified `.doAccuracyCheck` code:
//! ```text
//! 0F:67FA: 78          ld a, b
//! 0F:67FB: 3C          inc a              ; Z iff b == $FF (1-byte `cp $FF`)
//! 0F:67FC: C8          ret z              ; N=255 always hits
//! 0F:67FD: CD xx xx    call BattleRandom
//! 0F:6800: B8          cp b               ; ← inject A here
//! 0F:6801: 38 06       jr c, .accuracyHit ; random < accuracy → hit
//! 0F:6803: 20 05       jr nz, .moveMissed ; random > accuracy → miss
//! 0F:6805: CB 78       bit 7, b           ; random == accuracy
//! 0F:6807: 28 01       jr z, .moveMissed  ; N < 128 → miss
//! 0F:6809: C9          .accuracyHit (ret) ; N >= 128 → hit
//! 0F:680A: AF          .moveMissed (xor a, then sets wMoveMissed=1)
//! ```

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Offset from doAccuracyCheck to the `cp b` instruction (our injection point).
/// Sequence: ld a,b (1) + inc a (1) + ret z (1) + call BattleRandom (3) = 6.
const CP_B_OFFSET: u16 = 6;

/// A safe WRAM address to use as a return target after the accuracy check.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Set up the harness for an accuracy check and return a snapshot.
///
/// The snapshot captures state right before `doAccuracyCheck` with:
/// - ROM bank $0F selected
/// - SP pointing to a valid stack with TRAP_ADDR as return address
/// - hWhoseTurn = 0 (player)
/// - wMoveMissed = 0 (reset)
/// - PC at DO_ACCURACY_CHECK
fn setup_accuracy_fixture() -> (TestHarness, Vec<u8>) {
    let mut h = TestHarness::new_headless();

    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    h.select_rom_bank(sym_bank("MoveHitTest"));

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wMoveMissed"), 0x00);

    h.set_pc(sym_addr("MoveHitTest.doAccuracyCheck"));

    let state = h.save_state();
    (h, state)
}

/// Run an accuracy check with the given accuracy and random value.
///
/// Returns `true` if the move hits, `false` if it misses.
fn check_accuracy(h: &mut TestHarness, accuracy: u8, random: u8) -> bool {
    let do_accuracy_check = sym_addr("MoveHitTest.doAccuracyCheck");
    let cp_b_after_rng = do_accuracy_check + CP_B_OFFSET;

    h.set_b(accuracy);

    if accuracy == 255 {
        h.set_pc(do_accuracy_check);
        h.step_to(TRAP_ADDR);
    } else {
        h.set_a(random);
        h.set_pc(cp_b_after_rng);
        h.step_to(TRAP_ADDR);
    }

    h.read_mem(sym_addr("wMoveMissed")) == 0
}

/// Run accuracy checks in parallel over the given values.
///
/// Uses chunked parallelism (one thread per CPU core) to avoid spawning
/// hundreds of threads — CI runners have low thread limits and concurrent
/// workflow runs can exhaust them.
/// `test_fn` receives (value, harness, state) and returns `Some(error_msg)` on failure.
fn run_parallel<F>(values: &[u8], test_fn: F)
where
    F: Fn(u8, &mut TestHarness, &[u8]) -> Option<String> + Sync,
{
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let test_fn = &test_fn;
    let results: Vec<Option<String>> = std::thread::scope(|s| {
        let handles: Vec<_> = values
            .chunks(values.len().div_ceil(n_threads))
            .map(|chunk| {
                s.spawn(move || {
                    let (mut h, state) = setup_accuracy_fixture();
                    chunk
                        .iter()
                        .filter_map(|&val| test_fn(val, &mut h, &state))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .map(Some)
            .collect()
    });

    let failures: Vec<&str> = results.iter().filter_map(|r| r.as_deref()).collect();
    assert!(
        failures.is_empty(),
        "Accuracy check failures:\n{}",
        failures.join("\n")
    );
}

// ─── Scenario 1: N=255 always hits ──────────────────────────────────

#[test]
fn accuracy_255_always_hits() {
    let randoms: Vec<u8> = (0..=255u8).collect();
    run_parallel(&randoms, |random, h, state| {
        h.load_state(state);
        let hit = check_accuracy(h, 255, random);
        if !hit {
            Some(format!(
                "N=255 should ALWAYS hit, but missed with random={random}"
            ))
        } else {
            None
        }
    });
}

// ─── Scenario 2: Optimal rounding N >= 128 (random == N → hit) ─────

#[test]
fn accuracy_high_values_hit_on_equal() {
    let accuracies: Vec<u8> = (128..=254u8).collect();
    run_parallel(&accuracies, |accuracy, h, state| {
        h.load_state(state);
        let hit = check_accuracy(h, accuracy, accuracy);
        if !hit {
            Some(format!(
                "N={accuracy} (>=128): random==accuracy should HIT (bit 7 set → <=)"
            ))
        } else {
            None
        }
    });
}

// ─── Scenario 3: Optimal rounding N < 128 (random == N → miss) ─────

#[test]
fn accuracy_low_values_miss_on_equal() {
    let accuracies: Vec<u8> = (1..=127u8).collect();
    run_parallel(&accuracies, |accuracy, h, state| {
        h.load_state(state);
        let hit = check_accuracy(h, accuracy, accuracy);
        if hit {
            Some(format!(
                "N={accuracy} (<128): random==accuracy should MISS (bit 7 clear → <)"
            ))
        } else {
            None
        }
    });
}

// ─── Scenario 4: Basic hit/miss ─────────────────────────────────────

#[test]
fn accuracy_random_below_always_hits() {
    let accuracies: Vec<u8> = (1..=254u8).collect();
    run_parallel(&accuracies, |accuracy, h, state| {
        let random = accuracy - 1;
        h.load_state(state);
        let hit = check_accuracy(h, accuracy, random);
        if !hit {
            Some(format!(
                "N={accuracy}, random={random}: random < accuracy should always HIT"
            ))
        } else {
            None
        }
    });
}

#[test]
fn accuracy_random_above_always_misses() {
    let accuracies: Vec<u8> = (1..=254u8).collect();
    run_parallel(&accuracies, |accuracy, h, state| {
        let random = accuracy + 1;
        h.load_state(state);
        let hit = check_accuracy(h, accuracy, random);
        if hit {
            Some(format!(
                "N={accuracy}, random={random}: random > accuracy should always MISS"
            ))
        } else {
            None
        }
    });
}

// ─── Scenario 5: Exhaustive accuracy check ──────────────────────────

#[test]
fn accuracy_exhaustive_hit_counts() {
    // Use one thread per CPU core, each handling a chunk of accuracy values.
    // This minimizes fixture setup overhead (one GameBoy init per thread)
    // while using lightweight state reset between iterations instead of
    // expensive save/load serialization.
    let accuracies: Vec<u8> = (1..=255u8).collect();
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let results: Vec<Option<String>> = std::thread::scope(|s| {
        let handles: Vec<_> = accuracies
            .chunks(accuracies.len().div_ceil(n_threads))
            .map(|chunk| {
                let chunk = chunk.to_vec();
                s.spawn(move || {
                    let bank = sym_bank("MoveHitTest");
                    let do_accuracy_check = sym_addr("MoveHitTest.doAccuracyCheck");
                    let cp_b_after_rng = do_accuracy_check + CP_B_OFFSET;

                    let mut h = TestHarness::new_headless();
                    h.gb.cpu().set_ime(false);
                    h.write_mem(0xFFFF, 0x00);
                    h.gb.set_timer_enabled(false);
                    h.gb.set_serial_enabled(false);
                    h.gb.set_dma_enabled(false);
                    h.select_rom_bank(bank);
                    h.write_mem(TRAP_ADDR, NOP);
                    h.write_mem(TRAP_ADDR + 1, STOP);
                    h.set_sp(0xDFF0);
                    h.push_word(TRAP_ADDR);
                    let base_sp = h.gb.cpu_i().sp();
                    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

                    let w_move_missed = sym_addr("wMoveMissed");
                    let mut errors: Vec<String> = Vec::new();
                    for accuracy in chunk {
                        let mut hits = 0u32;
                        for random in 0..=255u8 {
                            h.gb.cpu().set_sp(base_sp);
                            h.write_mem(w_move_missed, 0x00);
                            h.set_b(accuracy);

                            if accuracy == 255 {
                                h.set_pc(do_accuracy_check);
                            } else {
                                h.set_a(random);
                                h.set_pc(cp_b_after_rng);
                            }
                            h.step_to(TRAP_ADDR);

                            if h.read_mem(w_move_missed) == 0 {
                                hits += 1;
                            }
                        }

                        let expected_hits: u32 = if accuracy == 255 {
                            256
                        } else if accuracy >= 128 {
                            accuracy as u32 + 1
                        } else {
                            accuracy as u32
                        };

                        if hits != expected_hits {
                            errors.push(format!(
                                "N={accuracy}: expected {expected_hits} hits, got {hits}"
                            ));
                        }
                    }
                    errors
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .map(Some)
            .collect()
    });

    let failures: Vec<&str> = results.iter().filter_map(|r| r.as_deref()).collect();
    assert!(
        failures.is_empty(),
        "Accuracy check failures:\n{}",
        failures.join("\n")
    );
}

// ─── 1/256 miss glitch regression tests ──────────────────────────────
//
// The Gen 1 "1/256 miss glitch" (Bulbapedia):
//   The accuracy check used `random < accuracy` (strictly less than).
//   If the random number is exactly 255, it can NEVER be less than any
//   accuracy value (max 255), so the move always misses — even with
//   100% accuracy. This made all moves 1/256 more likely to miss.
//
// Our fix:
//   N=255 → always hit (bypass RNG with `cp $FF / ret z`)
//   N≥128, random==N → hit  (≤ comparison, optimal rounding)
//   N<128, random==N → miss (< comparison, optimal rounding)
//
// These tests directly verify the bug scenario from the description.

/// Regression: the EXACT bug — 100% accuracy move with random=255 should hit.
///
/// In the original Gen 1 code, this would miss because `255 < 255` is false.
/// Our fix bypasses RNG entirely for N=255, so this always hits.
#[test]
fn regression_1_in_256_miss_100pct_accuracy_with_random_255() {
    let (mut h, state) = setup_accuracy_fixture();
    h.load_state(&state);
    assert!(
        check_accuracy(&mut h, 255, 255),
        "1/256 miss glitch: N=255, random=255 must HIT (was the original bug)"
    );
}

/// Full MoveHitTest path: N=255 with no accuracy/evasion modifiers.
///
/// Tests the COMPLETE code path from MoveHitTest entry (not just doAccuracyCheck),
/// including the Dream Eater check, Swift check, X Accuracy check, CalcHitChance
/// modifier application, and finally the accuracy comparison.
///
/// This verifies that CalcHitChance preserves N=255 when accuracy/evasion stages
/// are at default (7), and the `cp $FF / ret z` bypass still triggers.

#[test]
fn regression_full_path_100pct_move_hits() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("MoveHitTest");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    // Player's turn
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);

    // Move with no special effect, 100% accuracy
    h.write_mem(sym_addr("wPlayerMoveEffect"), 0x00);
    h.write_mem(sym_addr("wPlayerMoveAccuracy"), 255);

    // Target is not invulnerable, not protected by mist
    h.write_mem(sym_addr("wEnemyBattleStatus1"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x00);

    // Player not using X Accuracy (we want to test the accuracy check, not bypass)
    h.write_mem(sym_addr("wPlayerBattleStatus2"), 0x00);

    // Accuracy/evasion at default stage (7 = no modifier)
    h.write_mem(sym_addr("wPlayerMonAccuracyMod"), 7);
    h.write_mem(sym_addr("wEnemyMonEvasionMod"), 7);

    // Run from MoveHitTest entry
    h.write_mem(sym_addr("wMoveMissed"), 0x00);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("MoveHitTest"));
    h.step_to(TRAP_ADDR);

    assert!(
        h.read_mem(sym_addr("wMoveMissed")) == 0,
        "Full MoveHitTest path: N=255 with default accuracy/evasion must always HIT"
    );
}

/// Swift (SWIFT_EFFECT=$11) bypasses accuracy checks entirely.
///
/// Per Bulbapedia: "In non-Japanese versions, Swift and Bide skip accuracy
/// checks and always hit, regardless of this bug."
const SWIFT_EFFECT: u8 = 0x11;

#[test]
fn swift_always_hits_bypasses_accuracy_check() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("MoveHitTest");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wPlayerMoveEffect"), SWIFT_EFFECT);
    h.write_mem(sym_addr("wPlayerMoveAccuracy"), 1); // worst possible accuracy
    h.write_mem(sym_addr("wEnemyBattleStatus1"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x00);
    h.write_mem(sym_addr("wPlayerBattleStatus2"), 0x00);

    h.write_mem(sym_addr("wMoveMissed"), 0x00);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("MoveHitTest"));
    h.step_to(TRAP_ADDR);

    assert!(
        h.read_mem(sym_addr("wMoveMissed")) == 0,
        "Swift (SWIFT_EFFECT) must always hit — bypasses accuracy check entirely"
    );
}

/// X Accuracy bypasses the accuracy check entirely.
///
/// When USING_X_ACCURACY bit is set in wPlayerBattleStatus2,
/// MoveHitTest returns immediately (hit) without calling CalcHitChance
/// or doAccuracyCheck.
const USING_X_ACCURACY_BIT: u8 = 0; // bit 0

#[test]
fn x_accuracy_always_hits() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(sym_bank("MoveHitTest"));

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wPlayerMoveEffect"), 0x00);
    h.write_mem(sym_addr("wPlayerMoveAccuracy"), 1); // worst accuracy
    h.write_mem(sym_addr("wEnemyBattleStatus1"), 0x00);
    h.write_mem(sym_addr("wEnemyMonStatus"), 0x00);

    // Set X Accuracy active
    h.write_mem(sym_addr("wPlayerBattleStatus2"), 1 << USING_X_ACCURACY_BIT);

    // Default accuracy/evasion stages
    h.write_mem(sym_addr("wPlayerMonAccuracyMod"), 7);
    h.write_mem(sym_addr("wEnemyMonEvasionMod"), 7);

    h.write_mem(sym_addr("wMoveMissed"), 0x00);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("MoveHitTest"));
    h.step_to(TRAP_ADDR);

    assert!(
        h.read_mem(sym_addr("wMoveMissed")) == 0,
        "X Accuracy must make all moves hit regardless of accuracy value"
    );
}
