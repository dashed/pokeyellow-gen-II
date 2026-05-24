use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

fn setup_randomize_fixture() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("RandomizeDamage");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h
}

fn randomize_damage(h: &mut TestHarness, damage: u16) -> u16 {
    let w_damage = sym_addr("wDamage");
    h.write_mem(w_damage, (damage >> 8) as u8);
    h.write_mem(w_damage + 1, (damage & 0xFF) as u8);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(sym_addr("RandomizeDamage"));
    h.step_to(TRAP_ADDR);
    let hi = h.read_mem(w_damage) as u16;
    let lo = h.read_mem(w_damage + 1) as u16;
    (hi << 8) | lo
}

fn expected_min(damage: u16) -> u16 {
    ((damage as u32) * 217 / 255) as u16
}

#[test]
fn damage_0_unchanged() {
    let mut h = setup_randomize_fixture();
    let result = randomize_damage(&mut h, 0);
    assert_eq!(result, 0, "damage=0 should return unchanged");
}

#[test]
fn damage_1_unchanged() {
    let mut h = setup_randomize_fixture();
    let result = randomize_damage(&mut h, 1);
    assert_eq!(result, 1, "damage=1 should return unchanged");
}

#[test]
fn damage_2_is_randomized() {
    let mut h = setup_randomize_fixture();
    let mut saw_1 = false;
    let mut saw_2 = false;
    for _ in 0..200 {
        let result = randomize_damage(&mut h, 2);
        assert!(
            result >= expected_min(2) && result <= 2,
            "damage=2: result {result} out of range [{}, 2]",
            expected_min(2)
        );
        if result == 1 {
            saw_1 = true;
        }
        if result == 2 {
            saw_2 = true;
        }
        if saw_1 && saw_2 {
            break;
        }
    }
    assert!(
        saw_1,
        "damage=2: expected to see result=1 (floor(2*217/255)=1) in 200 trials"
    );
    assert!(saw_2, "damage=2: expected to see result=2 in 200 trials");
}

#[test]
fn damage_100_within_range() {
    let mut h = setup_randomize_fixture();
    let min = expected_min(100);
    for _ in 0..200 {
        let result = randomize_damage(&mut h, 100);
        assert!(
            result >= min && result <= 100,
            "damage=100: result {result} out of range [{min}, 100]"
        );
    }
}

#[test]
fn damage_500_within_range() {
    let mut h = setup_randomize_fixture();
    let min = expected_min(500);
    for _ in 0..200 {
        let result = randomize_damage(&mut h, 500);
        assert!(
            result >= min && result <= 500,
            "damage=500: result {result} out of range [{min}, 500]"
        );
    }
}

#[test]
fn damage_999_within_range() {
    let mut h = setup_randomize_fixture();
    let min = expected_min(999);
    for _ in 0..200 {
        let result = randomize_damage(&mut h, 999);
        assert!(
            result >= min && result <= 999,
            "damage=999: result {result} out of range [{min}, 999]"
        );
    }
}

#[test]
fn distribution_is_approximately_uniform() {
    let mut h = setup_randomize_fixture();
    let mut counts = [0u32; 16]; // bins for values 85..=100
    let trials = 2000;
    let min = expected_min(100); // 85
    for _ in 0..trials {
        let result = randomize_damage(&mut h, 100);
        assert!(
            result >= min && result <= 100,
            "damage=100: result {result} out of range [{min}, 100]"
        );
        let idx = (result - min) as usize;
        if idx < counts.len() {
            counts[idx] += 1;
        }
    }
    let distinct = counts.iter().filter(|&&c| c > 0).count();
    assert!(
        distinct >= 10,
        "expected at least 10 distinct values in [{min}, 100] over {trials} trials, got {distinct}: {counts:?}"
    );
}
