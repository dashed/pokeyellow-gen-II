//! E2E test: party alive check via CPU-level emulation.
//!
//! Tests AnyPartyAlive with specific party configurations to verify the
//! game correctly identifies when all/some party members are fainted.
//! Uses a WRAM trampoline to call the ROM routine and capture the result.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

// WRAM addresses
const W_PARTY_COUNT: u16 = 0xD162;
const W_PARTY_SPECIES: u16 = 0xD163;
const W_PARTY_MON1_HP: u16 = 0xD16B;
const PARTYMON_STRUCT_LENGTH: u16 = 0x2C;

const PIKACHU: u8 = 0x54;
const PIDGEY: u8 = 0x24;
const RATTATA: u8 = 0xA5;

const RESULT_ADDR: u16 = 0xC010;

fn setup_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("AnyPartyAlive"));
    h
}

fn setup_party(h: &mut TestHarness, species: &[u8], hp_values: &[(u8, u8)]) {
    let count = species.len() as u8;
    h.write_mem(W_PARTY_COUNT, count);
    for (i, &sp) in species.iter().enumerate() {
        h.write_mem(W_PARTY_SPECIES + i as u16, sp);
    }
    h.write_mem(W_PARTY_SPECIES + count as u16, 0xFF);
    for (i, &(hp_hi, hp_lo)) in hp_values.iter().enumerate() {
        let hp_addr = W_PARTY_MON1_HP + (i as u16 * PARTYMON_STRUCT_LENGTH);
        h.write_mem(hp_addr, hp_hi);
        h.write_mem(hp_addr + 1, hp_lo);
    }
}

/// Call AnyPartyAlive via trampoline. Returns true if any mon is alive (D != 0).
fn call_any_party_alive(h: &mut TestHarness) -> bool {
    let routine = sym_addr("AnyPartyAlive");
    let lo = (routine & 0xFF) as u8;
    let hi = (routine >> 8) as u8;

    // Trampoline: call AnyPartyAlive; ld a, d; ld [RESULT], a; stop
    h.write_mem(0xC000, 0xCD);
    h.write_mem(0xC001, lo);
    h.write_mem(0xC002, hi);
    h.write_mem(0xC003, 0x7A); // ld a, d
    h.write_mem(0xC004, 0xEA); // ld [nn], a
    h.write_mem(0xC005, (RESULT_ADDR & 0xFF) as u8);
    h.write_mem(0xC006, (RESULT_ADDR >> 8) as u8);
    h.write_mem(0xC007, 0x10); // stop

    h.write_mem(RESULT_ADDR, 0xFF);
    h.write_mem(0xFFFF, 0x00); // IE = 0

    h.set_sp(0xDFF0);
    h.set_pc(0xC000);
    h.step_to(0xC007);

    h.read_mem(RESULT_ADDR) != 0
}

#[test]
fn all_fainted_returns_false() {
    let mut h = setup_harness();
    setup_party(&mut h, &[PIKACHU, PIDGEY], &[(0, 0), (0, 0)]);
    assert!(!call_any_party_alive(&mut h), "All fainted → D should be 0");
}

#[test]
fn one_healthy_returns_true() {
    let mut h = setup_harness();
    setup_party(&mut h, &[PIKACHU, PIDGEY], &[(0, 50), (0, 0)]);
    assert!(
        call_any_party_alive(&mut h),
        "One healthy → D should be nonzero"
    );
}

#[test]
fn all_healthy_returns_true() {
    let mut h = setup_harness();
    setup_party(&mut h, &[PIKACHU, PIDGEY], &[(0, 50), (0, 30)]);
    assert!(
        call_any_party_alive(&mut h),
        "All healthy → D should be nonzero"
    );
}

#[test]
fn single_healthy_mon_returns_true() {
    let mut h = setup_harness();
    setup_party(&mut h, &[PIKACHU], &[(0, 50)]);
    assert!(
        call_any_party_alive(&mut h),
        "Single healthy mon → D should be nonzero"
    );
}

#[test]
fn three_mons_only_last_healthy() {
    let mut h = setup_harness();
    setup_party(
        &mut h,
        &[PIKACHU, PIDGEY, RATTATA],
        &[(0, 0), (0, 0), (0, 20)],
    );
    assert!(
        call_any_party_alive(&mut h),
        "Last mon healthy → D should be nonzero"
    );
}

#[test]
fn high_hp_byte_counts_as_alive() {
    let mut h = setup_harness();
    // HP = 0x0100 (256) — high byte nonzero, low byte zero
    setup_party(&mut h, &[PIKACHU], &[(1, 0)]);
    assert!(call_any_party_alive(&mut h), "HP high byte nonzero → alive");
}
