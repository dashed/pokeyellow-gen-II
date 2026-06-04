//! ROM byte tests for the catch rate RNG bias fix.
//!
//! Bug: The original capture algorithm uses rejection sampling to constrain
//! Rand1 to [0,200] (Great Ball) or [0,150] (Ultra/Safari Ball). Because the
//! Gen I RNG (`Random_`) is rDIV-based, consecutive calls produce correlated
//! values when the loop iteration count is deterministic. This correlates
//! Rand1 with Rand2, causing significant catch rate bias — Ultra Balls can
//! perform worse than Poke Balls, and Safari Zone Pokemon are much harder
//! to catch than intended.
//!
//! Fix: Replace rejection sampling with multiplication-based range reduction.
//! `Rand1 = Random * scale / 256` maps [0,255] uniformly onto [0,B] with
//! exactly one RNG call, eliminating the timing correlation.
//!
//! Reference: https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Catch_rate_RNG_oversight
//! Reference: https://glitchcity.wiki/wiki/RNG_correlation_(Generation_I)

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("ItemUseBall"));
    h
}

// ─── Structural tests ──────────────────────────────────────────────

#[test]
fn item_use_ball_is_in_bank_03() {
    assert_eq!(sym_bank("ItemUseBall"), 0x03);
}

#[test]
fn loop_label_calls_random_once() {
    // At .loop: call Random ($CD lo hi) — exactly one call, no looping back
    let mut h = rom_harness();
    let loop_addr = sym_addr("ItemUseBall.loop");
    assert_eq!(rom(&mut h, loop_addr), 0xCD, "call opcode at .loop");
    // Verify it's calling Random
    let random_addr = sym_addr("Random");
    let lo = (random_addr & 0xFF) as u8;
    let hi = (random_addr >> 8) as u8;
    assert_eq!(rom(&mut h, loop_addr + 1), lo, "Random address low byte");
    assert_eq!(rom(&mut h, loop_addr + 2), hi, "Random address high byte");
}

#[test]
fn no_rejection_loop_between_loop_and_check_for_ailments() {
    // Verify there is no `jr` instruction that jumps back to .loop
    // between .loop and .checkForAilments. The original bug had
    // `jr c, .loop` for rejection sampling.
    let mut h = rom_harness();
    let loop_addr = sym_addr("ItemUseBall.loop");
    let check_addr = sym_addr("ItemUseBall.checkForAilments");

    for addr in loop_addr..check_addr {
        let opcode = rom(&mut h, addr);
        // jr c = $38, jr = $18 — check if any jr jumps back to .loop
        if opcode == 0x38 || opcode == 0x18 {
            let offset = rom(&mut h, addr + 1) as i8;
            let target = (addr as i32 + 2 + offset as i32) as u16;
            assert_ne!(
                target, loop_addr,
                "Found jr back to .loop at {addr:#06x} — rejection sampling not removed"
            );
        }
    }
}

#[test]
fn great_ball_scale_loads_201() {
    // At .greatBallScale: ld a, 201 ($3E $C9)
    let mut h = rom_harness();
    let addr = sym_addr("ItemUseBall.greatBallScale");
    assert_eq!(rom(&mut h, addr), 0x3E, "ld a, n opcode");
    assert_eq!(rom(&mut h, addr + 1), 201, "Great Ball scale factor = 201");
}

#[test]
fn ultra_safari_scale_loads_151() {
    // Before .greatBallScale: ld a, 151 ($3E $97) + jr .scaleRand1 ($18 xx)
    let mut h = rom_harness();
    let great_addr = sym_addr("ItemUseBall.greatBallScale");
    // ld a, 151 is at greatBallScale - 4 (ld a,n = 2 bytes, jr = 2 bytes)
    let ultra_ld = great_addr - 4;
    assert_eq!(
        rom(&mut h, ultra_ld),
        0x3E,
        "ld a, n opcode for Ultra/Safari"
    );
    assert_eq!(
        rom(&mut h, ultra_ld + 1),
        151,
        "Ultra/Safari Ball scale factor = 151"
    );
    // Followed by jr to .scaleRand1
    assert_eq!(
        rom(&mut h, ultra_ld + 2),
        0x18,
        "jr opcode after Ultra scale"
    );
}

#[test]
fn scale_rand1_calls_multiply() {
    // At .scaleRand1: after setup, there should be a call Multiply ($CD lo hi)
    let mut h = rom_harness();
    let scale_addr = sym_addr("ItemUseBall.scaleRand1");
    let check_addr = sym_addr("ItemUseBall.checkForAilments");
    let multiply_addr = sym_addr("Multiply");
    let lo = (multiply_addr & 0xFF) as u8;
    let hi = (multiply_addr >> 8) as u8;

    // Scan from .scaleRand1 to .checkForAilments for call Multiply
    let mut found = false;
    for addr in scale_addr..check_addr {
        if rom(&mut h, addr) == 0xCD && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "call Multiply not found between .scaleRand1 and .checkForAilments"
    );
}

#[test]
fn poke_ball_path_bypasses_scaling() {
    // After the Master Ball check, cp POKE_BALL followed by jr z, .checkForAilments
    // This means Poke Ball goes directly to .checkForAilments without scaling
    let mut h = rom_harness();
    let loop_addr = sym_addr("ItemUseBall.loop");
    let check_addr = sym_addr("ItemUseBall.checkForAilments");

    // Scan for cp POKE_BALL ($FE xx) followed by jr z ($28 offset)
    // POKE_BALL constant value — let's find it by looking at the pattern
    // after call Random + ld b,a + ld hl,wCurItem + ld a,[hl] + cp MASTER_BALL + jp z + cp POKE_BALL + jr z
    let mut found_poke_jr_z = false;
    for addr in loop_addr..check_addr.saturating_sub(4) {
        if rom(&mut h, addr) == 0xFE {
            // This could be cp n; check if next is jr z
            let next = addr + 2;
            if rom(&mut h, next) == 0x28 {
                // Check if the jr z target is .checkForAilments
                let offset = rom(&mut h, next + 1) as i8;
                let target = (next as i32 + 2 + offset as i32) as u16;
                if target == check_addr {
                    found_poke_jr_z = true;
                    break;
                }
            }
        }
    }
    assert!(
        found_poke_jr_z,
        "Poke Ball path should jr z directly to .checkForAilments"
    );
}

#[test]
fn scale_result_read_from_h_product_plus_2() {
    // After call Multiply, the code should read ldh a, [hProduct + 2]
    // hProduct + 2 is an HRAM address. ldh a, [n] = opcode $F0
    let mut h = rom_harness();
    let scale_addr = sym_addr("ItemUseBall.scaleRand1");
    let check_addr = sym_addr("ItemUseBall.checkForAilments");
    let multiply_addr = sym_addr("Multiply");
    let mul_lo = (multiply_addr & 0xFF) as u8;
    let mul_hi = (multiply_addr >> 8) as u8;

    // Find call Multiply, then check the instruction after it
    for addr in scale_addr..check_addr.saturating_sub(5) {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == mul_lo
            && rom(&mut h, addr + 2) == mul_hi
        {
            // Next instruction should be ldh a, [hProduct + 2]
            let ldh_addr = addr + 3;
            assert_eq!(
                rom(&mut h, ldh_addr),
                0xF0,
                "ldh a, [n] opcode after call Multiply"
            );
            // hProduct + 2 low byte (HRAM offset)
            let h_product = sym_addr("hProduct");
            let expected_offset = ((h_product + 2) & 0xFF) as u8;
            assert_eq!(
                rom(&mut h, ldh_addr + 1),
                expected_offset,
                "should read hProduct + 2 (high byte of 16-bit result)"
            );
            // Followed by ld b, a ($47)
            assert_eq!(
                rom(&mut h, ldh_addr + 2),
                0x47,
                "ld b, a to store scaled Rand1"
            );
            return;
        }
    }
    panic!("call Multiply not found in .scaleRand1");
}
