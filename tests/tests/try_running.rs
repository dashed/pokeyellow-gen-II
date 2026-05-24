//! Behavioral emulator tests for TryRunningFromBattle (engine/battle/core.asm).
//!
//! TryRunningFromBattle determines whether the player can flee a wild battle.
//! It receives HL=pointer to player speed, DE=pointer to enemy speed (both
//! 2-byte big-endian). Several conditions guarantee escape (ghost battle,
//! safari, link, player speed >= enemy speed). Otherwise the escape formula is:
//!
//!   threshold = (playerSpeed * 32) / ((enemySpeed / 4) % 256) + 30 * (attempts - 1)
//!
//! A random byte < threshold means escape. Returns carry set = escaped.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

const BATTLE_TYPE_SAFARI: u8 = 2;
const BATTLE_TYPE_RUN: u8 = 3;
const LINK_STATE_BATTLING: u8 = 4;

fn setup_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("TryRunningFromBattle");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h
}

fn setup_wild_battle(h: &mut TestHarness) {
    h.write_mem(sym_addr("wIsInBattle"), 1);
    h.write_mem(sym_addr("wBattleType"), 0);
    h.write_mem(sym_addr("wLinkState"), 0);
    h.write_mem(sym_addr("wCurMap"), 0);
    h.write_mem(sym_addr("wNumRunAttempts"), 0);
}

fn set_speeds(h: &mut TestHarness, player_speed: u16, enemy_speed: u16) {
    let ps = sym_addr("wBattleMonSpeed");
    let es = sym_addr("wEnemyMonSpeed");
    h.write_mem(ps, (player_speed >> 8) as u8);
    h.write_mem(ps + 1, (player_speed & 0xFF) as u8);
    h.write_mem(es, (enemy_speed >> 8) as u8);
    h.write_mem(es + 1, (enemy_speed & 0xFF) as u8);
}

fn start_try_running(h: &mut TestHarness) {
    h.gb.cpu().set_hl(sym_addr("wBattleMonSpeed") as u16);
    h.gb.cpu().set_de(sym_addr("wEnemyMonSpeed") as u16);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("TryRunningFromBattle"));
}

fn compute_expected_threshold(player_speed: u16, enemy_speed: u16, run_attempts: u8) -> u16 {
    let enemy_div4_mod256 = ((enemy_speed / 4) & 0xFF) as u16;
    if enemy_div4_mod256 == 0 {
        return 256;
    }
    let player_times_32 = (player_speed as u32) * 32;
    let quotient = (player_times_32 as u16) / enemy_div4_mod256;
    quotient as u16 + 30 * (run_attempts.saturating_sub(1)) as u16
}

// ─── Deterministic escape paths ────────────────────────────────────

#[test]
fn ghost_battle_always_escapes() {
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);

    let try_running = sym_addr("TryRunningFromBattle");
    h.write_mem(try_running, 0xAF); // xor a — sets Z flag (ghost)
    h.write_mem(try_running + 1, NOP);
    h.write_mem(try_running + 2, NOP);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "ghost battle should reach .canEscape"
    );
}

#[test]
fn safari_battle_always_escapes() {
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    h.write_mem(sym_addr("wBattleType"), BATTLE_TYPE_SAFARI);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "safari battle should reach .canEscape"
    );
}

#[test]
fn run_battle_type_always_escapes() {
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    h.write_mem(sym_addr("wBattleType"), BATTLE_TYPE_RUN);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "RUN battle type should reach .canEscape"
    );
}

#[test]
fn link_battle_always_escapes() {
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    h.write_mem(sym_addr("wLinkState"), LINK_STATE_BATTLING);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "link battle should reach .canEscape"
    );
}

#[test]
fn trainer_battle_cannot_run() {
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    h.write_mem(sym_addr("wIsInBattle"), 2); // trainer battle

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.trainerBattle"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.trainerBattle"),
        "trainer battle should reach .trainerBattle"
    );
}

// ─── Speed-based escape paths ──────────────────────────────────────

#[test]
fn player_speed_equal_always_escapes() {
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    set_speeds(&mut h, 100, 100);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "equal speed should reach .canEscape (StringCmp: nc when equal)"
    );
}

#[test]
fn player_speed_greater_always_escapes() {
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    set_speeds(&mut h, 200, 100);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "player faster should reach .canEscape"
    );
}

#[test]
fn enemy_speed_div4_mod256_zero_escapes() {
    // Enemy speed = 1024: 1024/4 = 256, 256 % 256 = 0 → always escape
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    set_speeds(&mut h, 1, 1024);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "enemy speed where (speed/4)%256==0 should escape"
    );
}

// ─── Formula verification ──────────────────────────────────────────

#[test]
fn quotient_overflow_always_escapes() {
    // Player speed 255, enemy speed 4: threshold = (255*32)/((4/4)%256) = 8160 >> 255
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    set_speeds(&mut h, 255, 4);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "quotient overflow (>255) should escape"
    );
}

#[test]
fn formula_threshold_first_attempt() {
    // Player speed 50, enemy speed 200: threshold = (50*32)/((200/4)%256) = 1600/50 = 32
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    set_speeds(&mut h, 50, 200);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.compareWithRandomValue"));

    let threshold = h.read_mem(sym_addr("hQuotient") + 3);
    assert_eq!(
        threshold, 32,
        "threshold should be (50*32)/((200/4)%256) = 32, got {threshold}"
    );
}

#[test]
fn formula_threshold_second_attempt() {
    // Same speeds but 2nd attempt: threshold = 32 + 30*(2-1) = 62
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    h.write_mem(sym_addr("wNumRunAttempts"), 1); // will be incremented to 2
    set_speeds(&mut h, 50, 200);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.compareWithRandomValue"));

    let threshold = h.read_mem(sym_addr("hQuotient") + 3);
    assert_eq!(
        threshold, 62,
        "2nd attempt threshold should be 32+30 = 62, got {threshold}"
    );
}

#[test]
fn num_run_attempts_incremented() {
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    set_speeds(&mut h, 50, 200);

    assert_eq!(h.read_mem(sym_addr("wNumRunAttempts")), 0);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.compareWithRandomValue"));

    assert_eq!(
        h.read_mem(sym_addr("wNumRunAttempts")),
        1,
        "wNumRunAttempts should be incremented from 0 to 1"
    );
}

#[test]
fn high_run_attempts_carry_escapes() {
    // With threshold near 255, adding 30 causes carry → .canEscape
    // Player speed 50, enemy speed 200: base quotient = 32
    // Need attempts where 32 + 30*(n-1) > 255 → n > 8.43 → n=9
    // wNumRunAttempts = 8 (will be incremented to 9)
    let mut h = setup_fixture();
    setup_wild_battle(&mut h);
    h.write_mem(sym_addr("wNumRunAttempts"), 8);
    set_speeds(&mut h, 50, 200);

    start_try_running(&mut h);
    h.step_to(sym_addr("TryRunningFromBattle.canEscape"));
    assert_eq!(
        h.pc(),
        sym_addr("TryRunningFromBattle.canEscape"),
        "enough run attempts should cause carry overflow → escape"
    );
}

// ─── Sweep: verify threshold formula across many inputs ────────────

#[test]
fn sweep_threshold_calculation() {
    let mut h = setup_fixture();

    let player_speeds: &[u16] = &[10, 30, 50, 80, 100, 150, 200];
    let enemy_speeds: &[u16] = &[20, 60, 100, 200, 400, 800];
    let attempts: &[u8] = &[0, 1, 2, 4];

    let mut failures = Vec::new();
    let mut count = 0u32;

    for &ps in player_speeds {
        for &es in enemy_speeds {
            if ps >= es {
                continue; // player speed >= enemy → deterministic escape, skip
            }
            let enemy_div4_mod256 = ((es / 4) & 0xFF) as u16;
            if enemy_div4_mod256 == 0 {
                continue; // division by zero → deterministic escape, skip
            }

            for &att in attempts {
                let expected = compute_expected_threshold(ps, es, att + 1);
                if expected > 255 {
                    continue; // overflow → deterministic escape, skip
                }

                setup_wild_battle(&mut h);
                h.write_mem(sym_addr("wNumRunAttempts"), att);
                set_speeds(&mut h, ps, es);
                start_try_running(&mut h);
                h.step_to(sym_addr("TryRunningFromBattle.compareWithRandomValue"));

                let actual = h.read_mem(sym_addr("hQuotient") + 3) as u16;
                if actual != expected {
                    failures.push(format!(
                        "PS={ps} ES={es} att={}: expected {expected}, got {actual}",
                        att + 1
                    ));
                }
                count += 1;
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Threshold formula mismatches ({}/{count} failed):\n{}",
        failures.len(),
        failures[..failures.len().min(20)].join("\n")
    );
    assert!(
        count >= 30,
        "Should test at least 30 combinations, tested {count}"
    );
}
