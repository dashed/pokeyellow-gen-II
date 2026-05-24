use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

fn setup_hit_chance_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("CalcHitChance");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h
}

fn calc_hit_chance(h: &mut TestHarness, base_accuracy: u8, accuracy_mod: u8, evasion_mod: u8) -> u8 {
    h.write_mem(sym_addr("hWhoseTurn"), 0x00);
    h.write_mem(sym_addr("wPlayerMoveAccuracy"), base_accuracy);
    h.write_mem(sym_addr("wPlayerMonAccuracyMod"), accuracy_mod);
    h.write_mem(sym_addr("wEnemyMonEvasionMod"), evasion_mod);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("CalcHitChance"));
    h.step_to(TRAP_ADDR);
    h.read_mem(sym_addr("wPlayerMoveAccuracy"))
}

const STAT_MOD_RATIOS: [(u16, u16); 13] = [
    (25, 100), // stage 1
    (28, 100), // stage 2
    (33, 100), // stage 3
    (40, 100), // stage 4
    (50, 100), // stage 5
    (66, 100), // stage 6
    (1, 1),    // stage 7 (default)
    (15, 10),  // stage 8
    (2, 1),    // stage 9
    (25, 10),  // stage 10
    (3, 1),    // stage 11
    (35, 10),  // stage 12
    (4, 1),    // stage 13
];

fn expected_hit_chance(base: u8, accuracy_mod: u8, evasion_mod: u8) -> u8 {
    let reflected_evasion = 14 - evasion_mod;

    let acc_ratio = STAT_MOD_RATIOS[(accuracy_mod - 1) as usize];
    let eva_ratio = STAT_MOD_RATIOS[(reflected_evasion - 1) as usize];

    // The game does two sequential multiply-divide passes on a 3-byte multiplicand,
    // keeping intermediate results in hQuotient. We replicate the integer truncation.
    let mut val: u32 = base as u32;

    // Pass 1: accuracy modifier
    val = val * acc_ratio.0 as u32 / acc_ratio.1 as u32;
    if val == 0 {
        val = 1;
    }

    // Pass 2: reflected evasion modifier
    val = val * eva_ratio.0 as u32 / eva_ratio.1 as u32;
    if val == 0 {
        val = 1;
    }

    // Clamp to [1, 255]
    val.clamp(1, 255) as u8
}

#[test]
fn default_stages_no_change() {
    let mut h = setup_hit_chance_fixture();
    for base in [100u8, 200, 255, 1, 50] {
        let result = calc_hit_chance(&mut h, base, 7, 7);
        assert_eq!(
            result, base,
            "accuracyMod=7, evasionMod=7 should leave accuracy {base} unchanged, got {result}"
        );
    }
}

#[test]
fn accuracy_up_1_stage() {
    let mut h = setup_hit_chance_fixture();
    let result = calc_hit_chance(&mut h, 100, 8, 7);
    let expected = expected_hit_chance(100, 8, 7);
    assert_eq!(result, expected, "accuracyMod=8, evasionMod=7, base=100");
    assert_eq!(result, 150, "100 * 15/10 = 150");
}

#[test]
fn accuracy_down_1_stage() {
    let mut h = setup_hit_chance_fixture();
    let result = calc_hit_chance(&mut h, 100, 6, 7);
    let expected = expected_hit_chance(100, 6, 7);
    assert_eq!(result, expected, "accuracyMod=6, evasionMod=7, base=100");
    assert_eq!(result, 66, "100 * 66/100 = 66");
}

#[test]
fn evasion_up_1_stage() {
    let mut h = setup_hit_chance_fixture();
    // evasionMod=8 → reflected = 14-8 = 6 → ratio 66/100
    let result = calc_hit_chance(&mut h, 100, 7, 8);
    let expected = expected_hit_chance(100, 7, 8);
    assert_eq!(result, expected, "accuracyMod=7, evasionMod=8, base=100");
    assert_eq!(result, 66, "100 * 1/1 * 66/100 = 66");
}

#[test]
fn both_max_stages() {
    let mut h = setup_hit_chance_fixture();
    // accuracyMod=13 (4/1), evasionMod=1 → reflected=13 (4/1)
    // 200 * 4 = 800 → clamped intermediate, then * 4 = doesn't matter, capped at 255
    let result = calc_hit_chance(&mut h, 200, 13, 1);
    assert_eq!(result, 255, "both max stages should cap at 255");
}

#[test]
fn both_min_stages() {
    let mut h = setup_hit_chance_fixture();
    // accuracyMod=1 (25/100), evasionMod=13 → reflected=1 (25/100)
    // 200 * 25/100 = 50, then 50 * 25/100 = 12
    let result = calc_hit_chance(&mut h, 200, 1, 13);
    let expected = expected_hit_chance(200, 1, 13);
    assert_eq!(result, expected, "both min stages: base=200");
    assert_eq!(result, 12, "200 * 25/100 * 25/100 = 12");
}

#[test]
fn result_minimum_is_1() {
    let mut h = setup_hit_chance_fixture();
    // accuracyMod=1 (25/100), evasionMod=13 → reflected=1 (25/100)
    // 1 * 25/100 = 0 → clamped to 1, then 1 * 25/100 = 0 → clamped to 1
    let result = calc_hit_chance(&mut h, 1, 1, 13);
    assert_eq!(result, 1, "minimum result should be 1, not 0");
}

#[test]
fn result_capped_at_255() {
    let mut h = setup_hit_chance_fixture();
    // accuracyMod=13 (4/1), evasionMod=7 (1/1)
    // 255 * 4 = 1020 → capped at 255
    let result = calc_hit_chance(&mut h, 255, 13, 7);
    assert_eq!(result, 255, "result should be capped at 255");
}

#[test]
fn sweep_all_stage_combinations() {
    let mut h = setup_hit_chance_fixture();
    let base: u8 = 200;
    let mut failures = Vec::new();

    for acc_mod in 1..=13u8 {
        for eva_mod in 1..=13u8 {
            let result = calc_hit_chance(&mut h, base, acc_mod, eva_mod);
            let expected = expected_hit_chance(base, acc_mod, eva_mod);
            if result != expected {
                failures.push(format!(
                    "base={base}, accMod={acc_mod}, evaMod={eva_mod}: expected {expected}, got {result}"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Stage combination failures ({}/169):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
