//! ROM byte tests for the division by 0 fix.
//!
//! Bug: when the attacker's Attack/Special > 255, both offense and defense
//! stats are right-shifted by 2 (divided by 4) to fit in 8-bit registers.
//! If the defender's Defense/Special < 4, the shift makes it 0, causing
//! `CalculateDamage` to divide by 0 (infinite loop freeze).  The same
//! crash occurs when Reflect/Light Screen doubles a 512+ defense to 1024+,
//! which also scales to 0 after the >>2 shift.
//!
//! Fix: after the >>2 defense shift, clamp `c` (defense) to minimum 1,
//! mirroring the existing attack clamp.  +5 bytes per path (player/enemy),
//! +10 bytes total in bank $0F.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Division_by_0>

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

const LD_A_C: u8 = 0x79;
const AND_A: u8 = 0xA7;
const JR_NZ: u8 = 0x20;
const INC_C: u8 = 0x0C;
const SRL_B: u8 = 0xCB; // prefix for srl b (CB 38)
const SRL_B_OP: u8 = 0x38;
const SRL_H: u8 = 0xCB; // prefix for srl h (CB 3C)
const SRL_H_OP: u8 = 0x3C;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn get_damage_vars_is_in_bank_0f() {
    assert_eq!(
        sym_bank("GetDamageVarsForPlayerAttack"),
        0x0F,
        "GetDamageVarsForPlayerAttack should be in bank $0F"
    );
    assert_eq!(
        sym_bank("GetDamageVarsForEnemyAttack"),
        0x0F,
        "GetDamageVarsForEnemyAttack should be in bank $0F"
    );
}

#[test]
fn player_path_has_defense_clamp() {
    // Between .scaleStats and .next, look for: ld a,c / and a / jr nz / inc c
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GetDamageVarsForPlayerAttack"));
    let start = sym_addr("GetDamageVarsForPlayerAttack.scaleStats");
    let end = sym_addr("GetDamageVarsForPlayerAttack.next");
    let pattern = [LD_A_C, AND_A, JR_NZ];
    let clamp_addr = find_pattern(&mut h, start, end, &pattern);
    assert!(
        clamp_addr.is_some(),
        "Player path should have `ld a,c / and a / jr nz` defense clamp between .scaleStats and .next"
    );
    // Verify inc c follows the jr nz target
    let addr = clamp_addr.unwrap();
    let jr_offset = rom(&mut h, addr + 3) as u16;
    // jr nz skips over inc c (1 byte), so offset should be 1
    assert_eq!(jr_offset, 1, "jr nz should skip exactly 1 byte (inc c)");
    assert_eq!(rom(&mut h, addr + 4), INC_C, "inc c should follow jr nz");
}

#[test]
fn enemy_path_has_defense_clamp() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GetDamageVarsForEnemyAttack"));
    let start = sym_addr("GetDamageVarsForEnemyAttack.scaleStats");
    let end = sym_addr("GetDamageVarsForEnemyAttack.next");
    let pattern = [LD_A_C, AND_A, JR_NZ];
    let clamp_addr = find_pattern(&mut h, start, end, &pattern);
    assert!(
        clamp_addr.is_some(),
        "Enemy path should have `ld a,c / and a / jr nz` defense clamp between .scaleStats and .next"
    );
    let addr = clamp_addr.unwrap();
    let jr_offset = rom(&mut h, addr + 3) as u16;
    assert_eq!(jr_offset, 1, "jr nz should skip exactly 1 byte (inc c)");
    assert_eq!(rom(&mut h, addr + 4), INC_C, "inc c should follow jr nz");
}

#[test]
fn player_defense_clamp_is_after_srl_b_shifts() {
    // The defense clamp must come AFTER the srl b / rr c shifts
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GetDamageVarsForPlayerAttack"));
    let start = sym_addr("GetDamageVarsForPlayerAttack.scaleStats");
    let end = sym_addr("GetDamageVarsForPlayerAttack.next");
    // Find last srl b (CB 38) before the defense clamp
    let clamp_addr = find_pattern(&mut h, start, end, &[LD_A_C, AND_A, JR_NZ])
        .expect("defense clamp should exist");
    // Find srl b before clamp
    let srl_b_addr = find_pattern(&mut h, start, clamp_addr, &[SRL_B, SRL_B_OP]);
    assert!(
        srl_b_addr.is_some(),
        "srl b should precede defense clamp in player path"
    );
}

#[test]
fn player_defense_clamp_is_before_srl_h_shifts() {
    // The defense clamp must come BEFORE the srl h / rr l shifts (attack scaling)
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GetDamageVarsForPlayerAttack"));
    let start = sym_addr("GetDamageVarsForPlayerAttack.scaleStats");
    let end = sym_addr("GetDamageVarsForPlayerAttack.next");
    let clamp_addr = find_pattern(&mut h, start, end, &[LD_A_C, AND_A, JR_NZ])
        .expect("defense clamp should exist");
    // Find srl h (CB 3C) after the clamp
    let srl_h_addr = find_pattern(&mut h, clamp_addr, end, &[SRL_H, SRL_H_OP]);
    assert!(
        srl_h_addr.is_some(),
        "srl h (attack scaling) should follow defense clamp in player path"
    );
}

#[test]
fn enemy_defense_clamp_is_after_srl_b_shifts() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GetDamageVarsForEnemyAttack"));
    let start = sym_addr("GetDamageVarsForEnemyAttack.scaleStats");
    let end = sym_addr("GetDamageVarsForEnemyAttack.next");
    let clamp_addr = find_pattern(&mut h, start, end, &[LD_A_C, AND_A, JR_NZ])
        .expect("defense clamp should exist");
    let srl_b_addr = find_pattern(&mut h, start, clamp_addr, &[SRL_B, SRL_B_OP]);
    assert!(
        srl_b_addr.is_some(),
        "srl b should precede defense clamp in enemy path"
    );
}

#[test]
fn enemy_defense_clamp_is_before_srl_h_shifts() {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GetDamageVarsForEnemyAttack"));
    let start = sym_addr("GetDamageVarsForEnemyAttack.scaleStats");
    let end = sym_addr("GetDamageVarsForEnemyAttack.next");
    let clamp_addr = find_pattern(&mut h, start, end, &[LD_A_C, AND_A, JR_NZ])
        .expect("defense clamp should exist");
    let srl_h_addr = find_pattern(&mut h, clamp_addr, end, &[SRL_H, SRL_H_OP]);
    assert!(
        srl_h_addr.is_some(),
        "srl h (attack scaling) should follow defense clamp in enemy path"
    );
}

#[test]
fn explode_effect_defense_clamp_preserved() {
    // CalculateDamage already has a defense clamp for EXPLODE_EFFECT:
    // srl c / jr nz, .ok / inc c
    // Verify this existing protection is still intact.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CalculateDamage"));
    let start = sym_addr("CalculateDamage");
    let end = start + 0x40; // search within first 64 bytes
                            // Look for: srl c (CB 39) followed shortly by inc c (0C)
    let srl_c = find_pattern(&mut h, start, end, &[0xCB, 0x39]);
    assert!(
        srl_c.is_some(),
        "CalculateDamage should still have srl c for EXPLODE_EFFECT defense halving"
    );
    // inc c should follow within a few bytes
    let srl_c_addr = srl_c.unwrap();
    let inc_c = find_pattern(&mut h, srl_c_addr, srl_c_addr + 6, &[INC_C]);
    assert!(
        inc_c.is_some(),
        "inc c should follow srl c in EXPLODE_EFFECT path (minimum defense = 1)"
    );
}

#[test]
fn def_non_zero_label_exists_in_both_paths() {
    // Verify that the .defNonZero label was emitted (proves the fix compiled)
    let player = sym_addr("GetDamageVarsForPlayerAttack.defNonZero");
    let enemy = sym_addr("GetDamageVarsForEnemyAttack.defNonZero");
    assert!(
        player > sym_addr("GetDamageVarsForPlayerAttack.scaleStats"),
        ".defNonZero should be after .scaleStats in player path"
    );
    assert!(
        player < sym_addr("GetDamageVarsForPlayerAttack.next"),
        ".defNonZero should be before .next in player path"
    );
    assert!(
        enemy > sym_addr("GetDamageVarsForEnemyAttack.scaleStats"),
        ".defNonZero should be after .scaleStats in enemy path"
    );
    assert!(
        enemy < sym_addr("GetDamageVarsForEnemyAttack.next"),
        ".defNonZero should be before .next in enemy path"
    );
}

// ─── Behavioral tests ──────────────────────────────────────────────
//
// These actually execute GetDamageVarsForPlayerAttack and verify the
// defense clamp works at runtime, not just as ROM bytes.

const TRAP_ADDR_BH: u16 = 0xC100;

/// Run GetDamageVarsForPlayerAttack with the given defense (big-endian)
/// and a 256+ attack (to trigger the .scaleStats path).
/// Returns the C register (scaled defense) after the function returns.
fn run_player_damage_vars(defense_hi: u8, defense_lo: u8) -> u8 {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("GetDamageVarsForPlayerAttack");
    h.select_rom_bank(bank);

    h.write_mem(TRAP_ADDR_BH, 0x00); // NOP
    h.write_mem(TRAP_ADDR_BH + 1, 0x10); // STOP

    // wPlayerMovePower must be nonzero (otherwise function returns early)
    h.write_mem(sym_addr("wPlayerMovePower"), 40);
    // NORMAL type (0x00) = physical attack (below SPECIAL threshold)
    h.write_mem(sym_addr("wPlayerMoveType"), 0x00);
    // Enemy defense (big-endian: high byte at wEnemyMonDefense, low at +1)
    h.write_mem(sym_addr("wEnemyMonDefense"), defense_hi);
    h.write_mem(sym_addr("wEnemyMonDefense") + 1, defense_lo);
    // Player attack = 256 (high byte nonzero → triggers .scaleStats)
    h.write_mem(sym_addr("wBattleMonAttack"), 0x01);
    h.write_mem(sym_addr("wBattleMonAttack") + 1, 0x00);
    // No critical hit, no Reflect
    h.write_mem(sym_addr("wCriticalHitOrOHKO"), 0x00);
    h.write_mem(sym_addr("wEnemyBattleStatus3"), 0x00);
    // Level (needed for register E output)
    h.write_mem(sym_addr("wBattleMonLevel"), 50);

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR_BH);
    h.set_pc(sym_addr("GetDamageVarsForPlayerAttack"));
    h.step_to(TRAP_ADDR_BH);

    h.gb.cpu_i().c
}

#[test]
fn behavioral_defense_zero_clamp() {
    // Defense = 1: bc = [0x00, 0x01]. After >>2: bc = 0 → clamp c to 1.
    let c = run_player_damage_vars(0x00, 0x01);
    assert_eq!(
        c, 1,
        "Defense 1 after >>2 should be clamped to 1, got {c}"
    );
}

#[test]
fn behavioral_defense_3_clamps_to_1() {
    // Defense = 3: bc = [0x00, 0x03]. After >>2: 3/4 = 0 → clamp c to 1.
    let c = run_player_damage_vars(0x00, 0x03);
    assert_eq!(
        c, 1,
        "Defense 3 after >>2 should be clamped to 1, got {c}"
    );
}

#[test]
fn behavioral_defense_4_no_clamp() {
    // Defense = 4: bc = [0x00, 0x04]. After >>2: 4/4 = 1 → no clamp needed.
    let c = run_player_damage_vars(0x00, 0x04);
    assert_eq!(
        c, 1,
        "Defense 4 after >>2 should be 1 (no clamp needed), got {c}"
    );
}
