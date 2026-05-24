//! ROM byte tests for the defrost move forcing fix.
//!
//! Bug: when a frozen Pokémon is defrosted by the opponent's Fire-type move
//! mid-turn, it attacks using a stale/wrong `wSelectedMove` value.  This
//! causes link desync (different move on each Game Boy) and allows PP
//! underflow (using a move with 0 PP).
//!
//! Fix: in `CheckDefrost`, after clearing the freeze status, set
//! `wXSelectedMove = CANNOT_MOVE` ($FF) so the defrosted Pokémon skips its
//! turn.  Uses `dec a` (a=0→$FF) for byte efficiency.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Defrost_move_forcing>

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

const DEC_A: u8 = 0x3D;
const LD_ADDR_A: u8 = 0xEA; // ld [nn], a
const XOR_A: u8 = 0xAF;
const AND_N: u8 = 0xE6; // and n

// WRAM addresses from sym file
const W_PLAYER_SELECTED_MOVE: u16 = 0xCCDC;
const W_ENEMY_SELECTED_MOVE: u16 = 0xCCDD;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn check_defrost_is_in_bank_0f() {
    assert_eq!(
        sym_bank("CheckDefrost"),
        0x0F,
        "CheckDefrost should be in bank $0F (Battle Core section)"
    );
}

#[test]
fn check_defrost_entry_tests_frz_bit() {
    // CheckDefrost starts with `and 1 << FRZ` = `and $20` = E6 20
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckDefrost"));
    let base = sym_addr("CheckDefrost");
    assert_eq!(
        rom(&mut h, base),
        AND_N,
        "CheckDefrost should start with `and` opcode"
    );
    assert_eq!(
        rom(&mut h, base + 1),
        0x20,
        "CheckDefrost should mask with 1 << FRZ ($20)"
    );
}

#[test]
fn player_path_has_dec_a_ld_enemy_selected_move() {
    // Between CheckDefrost and CheckDefrost.opponent, look for:
    // dec a ($3D) + ld [wEnemySelectedMove], a ($EA $DD $CC)
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckDefrost"));
    let start = sym_addr("CheckDefrost");
    let end = sym_addr("CheckDefrost.opponent");
    let lo = (W_ENEMY_SELECTED_MOVE & 0xFF) as u8;
    let hi = (W_ENEMY_SELECTED_MOVE >> 8) as u8;
    let pattern = [DEC_A, LD_ADDR_A, lo, hi];
    assert!(
        find_pattern(&mut h, start, end, &pattern).is_some(),
        "Player path should have `dec a / ld [wEnemySelectedMove], a` to prevent defrosted enemy from attacking"
    );
}

#[test]
fn opponent_path_has_dec_a_ld_player_selected_move() {
    // Between CheckDefrost.opponent and CheckDefrost.common, look for:
    // dec a ($3D) + ld [wPlayerSelectedMove], a ($EA $DC $CC)
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckDefrost"));
    let start = sym_addr("CheckDefrost.opponent");
    let end = sym_addr("CheckDefrost.common");
    let lo = (W_PLAYER_SELECTED_MOVE & 0xFF) as u8;
    let hi = (W_PLAYER_SELECTED_MOVE >> 8) as u8;
    let pattern = [DEC_A, LD_ADDR_A, lo, hi];
    assert!(
        find_pattern(&mut h, start, end, &pattern).is_some(),
        "Opponent path should have `dec a / ld [wPlayerSelectedMove], a` to prevent defrosted player from attacking"
    );
}

#[test]
fn player_path_xor_a_before_dec_a() {
    // Verify `xor a` ($AF) precedes `dec a` ($3D) in the player path,
    // ensuring a=0 before `dec a` produces $FF = CANNOT_MOVE.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckDefrost"));
    let start = sym_addr("CheckDefrost");
    let end = sym_addr("CheckDefrost.opponent");
    let lo = (W_ENEMY_SELECTED_MOVE & 0xFF) as u8;
    let hi = (W_ENEMY_SELECTED_MOVE >> 8) as u8;
    let dec_a_addr = find_pattern(&mut h, start, end, &[DEC_A, LD_ADDR_A, lo, hi])
        .expect("dec a + ld [wEnemySelectedMove], a should exist");
    // Search backwards for xor a
    let mut found_xor = false;
    for addr in (start..dec_a_addr).rev() {
        if rom(&mut h, addr) == XOR_A {
            found_xor = true;
            break;
        }
    }
    assert!(
        found_xor,
        "xor a ($AF) should precede dec a in player path (proves a=0 before dec)"
    );
}

#[test]
fn opponent_path_xor_a_before_dec_a() {
    // Verify `xor a` ($AF) precedes `dec a` ($3D) in the opponent path.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckDefrost"));
    let start = sym_addr("CheckDefrost.opponent");
    let end = sym_addr("CheckDefrost.common");
    let lo = (W_PLAYER_SELECTED_MOVE & 0xFF) as u8;
    let hi = (W_PLAYER_SELECTED_MOVE >> 8) as u8;
    let dec_a_addr = find_pattern(&mut h, start, end, &[DEC_A, LD_ADDR_A, lo, hi])
        .expect("dec a + ld [wPlayerSelectedMove], a should exist");
    let mut found_xor = false;
    for addr in (start..dec_a_addr).rev() {
        if rom(&mut h, addr) == XOR_A {
            found_xor = true;
            break;
        }
    }
    assert!(
        found_xor,
        "xor a ($AF) should precede dec a in opponent path (proves a=0 before dec)"
    );
}

#[test]
fn check_defrost_common_is_jp_print_text() {
    // CheckDefrost.common should be `jp PrintText` ($C3 lo hi).
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckDefrost"));
    let common = sym_addr("CheckDefrost.common");
    assert_eq!(
        rom(&mut h, common),
        0xC3,
        "CheckDefrost.common should be `jp` opcode"
    );
}

#[test]
fn player_path_still_clears_enemy_mon_status() {
    // `ld [wEnemyMonStatus], a` ($EA lo hi) should still exist in player path
    // (the fix adds code AFTER the status clear, not replacing it).
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckDefrost"));
    let start = sym_addr("CheckDefrost");
    let end = sym_addr("CheckDefrost.opponent");
    let enemy_status = sym_addr("wEnemyMonStatus");
    let lo = (enemy_status & 0xFF) as u8;
    let hi = (enemy_status >> 8) as u8;
    assert!(
        find_pattern(&mut h, start, end, &[LD_ADDR_A, lo, hi]).is_some(),
        "Player path should still clear wEnemyMonStatus"
    );
}

#[test]
fn opponent_path_still_clears_battle_mon_status() {
    // `ld [wBattleMonStatus], a` should still exist in opponent path.
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CheckDefrost"));
    let start = sym_addr("CheckDefrost.opponent");
    let end = sym_addr("CheckDefrost.common");
    let battle_status = sym_addr("wBattleMonStatus");
    let lo = (battle_status & 0xFF) as u8;
    let hi = (battle_status >> 8) as u8;
    assert!(
        find_pattern(&mut h, start, end, &[LD_ADDR_A, lo, hi]).is_some(),
        "Opponent path should still clear wBattleMonStatus"
    );
}
