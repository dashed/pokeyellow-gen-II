//! ROM byte tests for the Hyper Beam + Sleep move glitch fix.
//!
//! Bug: `SleepEffect` checked `NEEDS_TO_RECHARGE` and, if set, jumped
//! directly to `.setSleepCounter`, skipping: (1) accuracy checks
//! (`MoveHitTest`), (2) existing status checks, and (3) Toxic counter
//! reset.  This meant sleep moves always hit recharging targets and
//! overwrote any existing status.
//!
//! Fix: remove the `bit NEEDS_TO_RECHARGE` test and `jr nz` bypass.
//! Keep `res NEEDS_TO_RECHARGE` so recharge is still cleared, but fall
//! through to normal accuracy/status checks.  −4 bytes in bank $0F.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Hyper_Beam_%2B_Sleep_move_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn find_pattern(h: &mut TestHarness, start: u16, end: u16, pattern: &[u8]) -> Option<u16> {
    if pattern.is_empty() || end <= start {
        return None;
    }
    let len = pattern.len() as u16;
    for addr in start..=(end.saturating_sub(len)) {
        if (0..len).all(|i| rom(h, addr + i) == pattern[i as usize]) {
            return Some(addr);
        }
    }
    None
}

// ─── Opcode constants ────────────────────────────────────────────────

// NEEDS_TO_RECHARGE = bit 5
const BIT_5_A: [u8; 2] = [0xCB, 0x6F]; // bit 5, a (the BUG opcode)
const RES_5_A: [u8; 2] = [0xCB, 0xAF]; // res 5, a (the fix: clear recharge)
const JR_NZ: u8 = 0x20;
const LD_A_BC: u8 = 0x0A; // ld a, [bc]
const LD_BC_A: u8 = 0x02; // ld [bc], a
const LD_A_DE: u8 = 0x1A; // ld a, [de]
const CALL: u8 = 0xCD;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn sleep_effect_is_in_bank_0f() {
    assert_eq!(
        sym_bank("SleepEffect"),
        0x0F,
        "SleepEffect should be in bank $0F"
    );
}

#[test]
fn no_bit_5_a_in_sleep_effect() {
    // The fix removes `bit NEEDS_TO_RECHARGE, a` (CB 6F).
    // Verify it's NOT present between .sleepEffect and .notAlreadySleeping.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SleepEffect"));
    let start = sym_addr("SleepEffect.sleepEffect");
    let end = sym_addr("SleepEffect.notAlreadySleeping");
    let found = find_pattern(&mut h, start, end, &BIT_5_A);
    assert!(
        found.is_none(),
        "SleepEffect should NOT have `bit 5, a` (the recharge bypass test was removed)"
    );
}

#[test]
fn no_jr_nz_bypass_in_sleep_effect() {
    // The fix removes the `jr nz, .setSleepCounter` that skipped checks.
    // Verify no `jr nz` (20 xx) between .sleepEffect and .notAlreadySleeping
    // targets .setSleepCounter.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SleepEffect"));
    let start = sym_addr("SleepEffect.sleepEffect");
    let end = sym_addr("SleepEffect.notAlreadySleeping");
    let set_sleep = sym_addr("SleepEffect.setSleepCounter");
    // Search for jr nz that would jump to setSleepCounter
    let mut addr = start;
    while addr < end {
        if rom(&mut h, addr) == JR_NZ {
            let offset = rom(&mut h, addr + 1) as i8;
            let target = (addr as i32 + 2 + offset as i32) as u16;
            assert_ne!(
                target, set_sleep,
                "Should not have jr nz targeting .setSleepCounter (the recharge bypass)"
            );
        }
        addr += 1;
    }
}

#[test]
fn res_5_a_still_present() {
    // `res NEEDS_TO_RECHARGE, a` (CB AF) should still be present to clear recharge.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SleepEffect"));
    let start = sym_addr("SleepEffect.sleepEffect");
    let end = sym_addr("SleepEffect.notAlreadySleeping");
    let found = find_pattern(&mut h, start, end, &RES_5_A);
    assert!(
        found.is_some(),
        "SleepEffect should still have `res 5, a` to clear NEEDS_TO_RECHARGE"
    );
}

#[test]
fn ld_bc_a_follows_res_5_a() {
    // After `res 5, a`, `ld [bc], a` should store the modified BattleStatus2.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SleepEffect"));
    let start = sym_addr("SleepEffect.sleepEffect");
    let end = sym_addr("SleepEffect.notAlreadySleeping");
    let res_addr = find_pattern(&mut h, start, end, &RES_5_A).expect("res 5, a should exist");
    assert_eq!(
        rom(&mut h, res_addr + 2),
        LD_BC_A,
        "ld [bc], a should follow res 5, a"
    );
}

#[test]
fn ld_a_de_follows_ld_bc_a() {
    // After storing BattleStatus2, the code should load the target's status byte
    // via `ld a, [de]` (normal status check path).
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SleepEffect"));
    let start = sym_addr("SleepEffect.sleepEffect");
    let end = sym_addr("SleepEffect.notAlreadySleeping");
    let res_addr = find_pattern(&mut h, start, end, &RES_5_A).expect("res 5, a should exist");
    // res 5,a (2) + ld [bc],a (1) = offset 3
    assert_eq!(
        rom(&mut h, res_addr + 3),
        LD_A_DE,
        "ld a, [de] (status check) should follow ld [bc], a"
    );
}

#[test]
fn move_hit_test_call_preserved() {
    // `call MoveHitTest` should still exist between .notAlreadySleeping and .setSleepCounter.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SleepEffect"));
    let start = sym_addr("SleepEffect.notAlreadySleeping");
    let end = sym_addr("SleepEffect.setSleepCounter");
    let move_hit_test = sym_addr("MoveHitTest");
    let lo = (move_hit_test & 0xFF) as u8;
    let hi = (move_hit_test >> 8) as u8;
    let found = find_pattern(&mut h, start, end, &[CALL, lo, hi]);
    assert!(
        found.is_some(),
        "call MoveHitTest should exist between .notAlreadySleeping and .setSleepCounter"
    );
}

#[test]
fn sleep_effect_starts_with_ld_a_bc() {
    // .sleepEffect should start with `ld a, [bc]` to load BattleStatus2.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SleepEffect"));
    let start = sym_addr("SleepEffect.sleepEffect");
    assert_eq!(
        rom(&mut h, start),
        LD_A_BC,
        "SleepEffect.sleepEffect should start with ld a, [bc]"
    );
}
