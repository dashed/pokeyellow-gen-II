//! ROM byte tests for the Jump Kick / Hi Jump Kick crash damage fix.
//!
//! Bug: `MoveHitTest.moveMissed` zeroed `wDamage` on any miss, so the crash
//! handler's `damage / 8` always produced `max(0/8, 1) = 1` HP of recoil.
//!
//! Fix: in `.moveMissed`, save `wDamage` to `wJumpKickMissDamage` before
//! zeroing.  The crash handler reads the saved copy, divides by 8, clamps
//! to minimum 1, and writes the result back to `wDamage` for
//! `ApplyDamageToPlayerPokemon` / `ApplyDamageToEnemyPokemon`.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Jump_Kick_and_Hi_Jump_Kick.27s_crash_damage>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Read a ROM byte at the given address (with correct bank selected).
fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Search for a byte pattern within [start, end).
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

const LD_ADDR_A: u8 = 0xEA; // ld [nn], a
const LD_HL_ADDR: u8 = 0x21; // ld hl, nn
const SRL_A: [u8; 2] = [0xCB, 0x3F]; // srl a
const RR_B: [u8; 2] = [0xCB, 0x18]; // rr b
const XOR_A: u8 = 0xAF; // xor a
const INC_A: u8 = 0x3C; // inc a
const OR_B: u8 = 0xB0; // or b
const JR_NZ: u8 = 0x20; // jr nz, e

// WRAM addresses
const W_DAMAGE: u16 = 0xD0D6;
const W_JUMP_KICK_MISS_DAMAGE: u16 = 0xD0D8;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn move_hit_test_is_in_bank_0f() {
    assert_eq!(
        sym_bank("MoveHitTest"),
        0x0F,
        "MoveHitTest should be in bank $0F"
    );
}

#[test]
fn move_missed_saves_damage_before_zeroing() {
    // .moveMissed should save wDamage to wJumpKickMissDamage before zeroing.
    // Look for: ld a, [hli] + ld [wJumpKickMissDamage], a
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("MoveHitTest"));
    let start = sym_addr("MoveHitTest.moveMissed");
    let end = sym_addr("MoveHitTest.playerTurn");
    let lo = (W_JUMP_KICK_MISS_DAMAGE & 0xFF) as u8;
    let hi = (W_JUMP_KICK_MISS_DAMAGE >> 8) as u8;
    // First save: ld [wJumpKickMissDamage], a
    let save_hi = find_pattern(&mut h, start, end, &[LD_ADDR_A, lo, hi]);
    assert!(
        save_hi.is_some(),
        "MoveHitTest.moveMissed should save damage high byte to wJumpKickMissDamage"
    );
    // Second save: ld [wJumpKickMissDamage + 1], a
    let save_lo = find_pattern(
        &mut h,
        save_hi.unwrap() + 3,
        end,
        &[LD_ADDR_A, lo.wrapping_add(1), hi],
    );
    assert!(
        save_lo.is_some(),
        "MoveHitTest.moveMissed should save damage low byte to wJumpKickMissDamage + 1"
    );
}

#[test]
fn move_missed_still_zeros_damage() {
    // After saving, wDamage should still be zeroed (xor a / ld [hld], a / ld [hl], a).
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("MoveHitTest"));
    let start = sym_addr("MoveHitTest.moveMissed");
    let end = sym_addr("MoveHitTest.playerTurn");
    // xor a should exist in the miss handler
    let mut found_xor = false;
    for addr in start..end {
        if rom(&mut h, addr) == XOR_A {
            found_xor = true;
            break;
        }
    }
    assert!(found_xor, "wDamage should still be zeroed (xor a present)");
}

#[test]
fn crash_handler_reads_saved_damage() {
    // The crash handler should read from wJumpKickMissDamage, not wDamage.
    // Look for: ld hl, wJumpKickMissDamage between gotTextToPrint and applyRecoil.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PrintMoveFailureText"));
    let start = sym_addr("PrintMoveFailureText.gotTextToPrint");
    let end = sym_addr("PrintMoveFailureText.applyRecoil");
    let lo = (W_JUMP_KICK_MISS_DAMAGE & 0xFF) as u8;
    let hi = (W_JUMP_KICK_MISS_DAMAGE >> 8) as u8;
    let found = find_pattern(&mut h, start, end, &[LD_HL_ADDR, lo, hi]);
    assert!(
        found.is_some(),
        "Crash handler should read from wJumpKickMissDamage (not wDamage)"
    );
}

#[test]
fn crash_handler_writes_result_to_w_damage() {
    // After dividing by 8, the crash handler should write to wDamage
    // for ApplyDamageToPlayerPokemon/ApplyDamageToEnemyPokemon.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PrintMoveFailureText"));
    let start = sym_addr("PrintMoveFailureText.gotTextToPrint");
    let end = sym_addr("PrintMoveFailureText.applyRecoil");
    let lo = (W_DAMAGE & 0xFF) as u8;
    let hi = (W_DAMAGE >> 8) as u8;
    let found = find_pattern(&mut h, start, end, &[LD_HL_ADDR, lo, hi]);
    assert!(
        found.is_some(),
        "Crash handler should write result to wDamage"
    );
}

#[test]
fn crash_handler_has_three_srl_rr_shifts() {
    // The crash handler should divide by 8: three pairs of srl a / rr b.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PrintMoveFailureText"));
    let start = sym_addr("PrintMoveFailureText.gotTextToPrint");
    let end = sym_addr("PrintMoveFailureText.applyRecoil");
    let mut count = 0;
    let pattern = [SRL_A[0], SRL_A[1], RR_B[0], RR_B[1]];
    let mut addr = start;
    while addr < end.saturating_sub(3) {
        if find_pattern(&mut h, addr, addr + 4, &pattern).is_some() {
            count += 1;
            addr += 4;
        } else {
            addr += 1;
        }
    }
    assert_eq!(
        count, 3,
        "Crash handler should have 3 srl a / rr b pairs (divide by 8)"
    );
}

#[test]
fn crash_handler_has_min_clamp_to_1() {
    // After division, or b / jr nz / inc a / ld [hl], a clamps to min 1.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PrintMoveFailureText"));
    let start = sym_addr("PrintMoveFailureText.gotTextToPrint");
    let end = sym_addr("PrintMoveFailureText.applyRecoil");
    let pattern = [OR_B, JR_NZ];
    let found = find_pattern(&mut h, start, end, &pattern);
    assert!(
        found.is_some(),
        "Crash handler should have or b / jr nz for min-1 clamp"
    );
    let or_addr = found.unwrap();
    // After jr nz (2 bytes), should be inc a / ld [hl], a
    assert_eq!(
        rom(&mut h, or_addr + 3),
        INC_A,
        "inc a should follow jr nz (clamp 0→1)"
    );
}

#[test]
fn crash_handler_calls_apply_damage() {
    // After .applyRecoil, the code should call ApplyDamageToPlayerPokemon
    // or ApplyDamageToEnemyPokemon based on hWhoseTurn.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PrintMoveFailureText"));
    let _recoil = sym_addr("PrintMoveFailureText.applyRecoil");
    let enemy = sym_addr("PrintMoveFailureText.enemyTurn");
    // .enemyTurn should exist and contain a jp instruction
    assert_eq!(
        rom(&mut h, enemy),
        0xC3,
        ".enemyTurn should be jp ApplyDamageToEnemyPokemon"
    );
}
