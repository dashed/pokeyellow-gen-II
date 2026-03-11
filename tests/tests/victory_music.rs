//! ROM byte tests for the battle victory music timing fix.
//!
//! Bug: `FaintEnemyPokemon` plays victory music for wild battles at
//! `.wild_win` before checking if the player's party is alive. When both
//! the player's last Pokémon and the wild Pokémon faint simultaneously
//! (e.g., via Explosion or Self-Destruct), the victory music plays even
//! though the player lost the battle.
//!
//! Fix: Before the wild/trainer branch, call `AnyPartyAlive` and push the
//! result. At `.wild_win`, pop and skip victory music if the party is dead
//! (Z flag set). The trainer path also pops to balance the stack. +10 bytes
//! in bank $0F.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("FaintEnemyPokemon"));
    h
}

/// Scan for `call AnyPartyAlive` ($CD lo hi) between two addresses.
fn find_call_any_party_alive(h: &mut TestHarness, start: u16, end: u16) -> Option<u16> {
    let target = sym_addr("AnyPartyAlive");
    let lo = (target & 0xFF) as u8;
    let hi = (target >> 8) as u8;
    for addr in start..end {
        if rom(h, addr) == 0xCD && rom(h, addr + 1) == lo && rom(h, addr + 2) == hi {
            return Some(addr);
        }
    }
    None
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn faint_enemy_pokemon_in_bank_0f() {
    assert_eq!(sym_bank("FaintEnemyPokemon"), 0x0F);
}

#[test]
fn any_party_alive_in_bank_0f() {
    assert_eq!(sym_bank("AnyPartyAlive"), 0x0F);
}

// ─── Core fix: AnyPartyAlive called before victory music ─────────────

#[test]
fn call_any_party_alive_before_wild_win() {
    let mut h = banked_harness();
    let base = sym_addr("FaintEnemyPokemon");
    let wild_win = sym_addr("FaintEnemyPokemon.wild_win");
    let call_addr = find_call_any_party_alive(&mut h, base, wild_win);
    assert!(
        call_addr.is_some(),
        "call AnyPartyAlive not found between FaintEnemyPokemon and .wild_win"
    );
}

#[test]
fn push_af_after_any_party_alive_check() {
    // After `call AnyPartyAlive` (3 bytes), expect: ld a, d ($7A) / and a ($A7) / push af ($F5)
    let mut h = banked_harness();
    let base = sym_addr("FaintEnemyPokemon");
    let wild_win = sym_addr("FaintEnemyPokemon.wild_win");
    let call_addr = find_call_any_party_alive(&mut h, base, wild_win).unwrap();
    let after = call_addr + 3;
    assert_eq!(rom(&mut h, after), 0x7A, "ld a, d after call AnyPartyAlive");
    assert_eq!(rom(&mut h, after + 1), 0xA7, "and a after ld a, d");
    assert_eq!(rom(&mut h, after + 2), 0xF5, "push af to save alive status");
}

// ─── Wild win path: pop af + conditional skip ────────────────────────

#[test]
fn wild_win_pops_af_and_skips_on_zero() {
    // At .wild_win: pop af ($F1) / jr z, .sfxplayed ($28 xx)
    let mut h = banked_harness();
    let wild_win = sym_addr("FaintEnemyPokemon.wild_win");
    assert_eq!(
        rom(&mut h, wild_win),
        0xF1,
        "pop af at .wild_win to retrieve alive status"
    );
    assert_eq!(
        rom(&mut h, wild_win + 1),
        0x28,
        "jr z opcode (skip victory music when party dead)"
    );
}

#[test]
fn wild_win_jr_z_targets_sfxplayed() {
    let mut h = banked_harness();
    let wild_win = sym_addr("FaintEnemyPokemon.wild_win");
    let sfxplayed = sym_addr("FaintEnemyPokemon.sfxplayed");
    // jr z is at wild_win+1 (opcode) / wild_win+2 (offset), PC after = wild_win+3
    let jr_offset = rom(&mut h, wild_win + 2) as i8;
    let jr_pc = wild_win + 3;
    let target = (jr_pc as i32 + jr_offset as i32) as u16;
    assert_eq!(
        target, sfxplayed,
        "jr z should target .sfxplayed (skip victory music)"
    );
}

#[test]
fn victory_music_after_skip_check() {
    // After pop af (1) + jr z (2), the original code follows:
    // call EndLowHealthAlarm ($CD lo hi) / ld a, MUSIC_DEFEATED_WILD_MON ($3E xx) / call PlayBattleVictoryMusic
    let mut h = banked_harness();
    let wild_win = sym_addr("FaintEnemyPokemon.wild_win");
    let after_skip = wild_win + 3; // after pop af + jr z
    let end_alarm = sym_addr("EndLowHealthAlarm");
    let lo = (end_alarm & 0xFF) as u8;
    let hi = (end_alarm >> 8) as u8;
    assert_eq!(
        rom(&mut h, after_skip),
        0xCD,
        "call opcode for EndLowHealthAlarm"
    );
    assert_eq!(
        rom(&mut h, after_skip + 1),
        lo,
        "EndLowHealthAlarm low byte"
    );
    assert_eq!(
        rom(&mut h, after_skip + 2),
        hi,
        "EndLowHealthAlarm high byte"
    );
}

// ─── Trainer path: pop af to balance stack ───────────────────────────

#[test]
fn trainer_path_pops_af_before_sfxplayed() {
    // In the trainer faint SFX path, pop af ($F1) + jr .sfxplayed ($18 xx)
    // appear between .sfxwait and .wild_win (just before the wild path).
    let mut h = banked_harness();
    let sfxwait = sym_addr("FaintEnemyPokemon.sfxwait");
    let wild_win = sym_addr("FaintEnemyPokemon.wild_win");
    let sfxplayed = sym_addr("FaintEnemyPokemon.sfxplayed");
    // Scan for pop af ($F1) followed by jr ($18) in the trainer region
    let mut found = false;
    for addr in sfxwait..wild_win {
        if rom(&mut h, addr) == 0xF1 && rom(&mut h, addr + 1) == 0x18 {
            // Verify the jr targets .sfxplayed
            let jr_offset = rom(&mut h, addr + 2) as i8;
            let jr_pc = addr + 3; // PC after reading the jr instruction
            let target = (jr_pc as i32 + jr_offset as i32) as u16;
            assert_eq!(target, sfxplayed, "jr should target .sfxplayed");
            found = true;
            break;
        }
    }
    assert!(
        found,
        "pop af + jr .sfxplayed not found in trainer path (between .sfxwait and .wild_win)"
    );
}
