//! ROM byte tests for the Silent Indigo Plateau fix.
//!
//! Bug: When a Pokémon evolves during the Champion (RIVAL3) battle,
//! EvolveMon calls StopAllMusic which kills the victory music.
//! BIT_NO_MAP_MUSIC (set by TrainerBattleVictory for RIVAL3) prevents the
//! overworld from auto-restoring any music, leaving silence until Professor
//! Oak arrives and plays Music_Cities1AlternateTempo.
//!
//! Fix: In EndOfBattle, after EvolutionAfterBattle returns, check if
//! BIT_NO_MAP_MUSIC is set and wEvolutionOccurred is non-zero.  If so,
//! replay the gym leader victory music (MUSIC_DEFEATED_GYM_LEADER).
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Silent_Indigo_Plateau>
//! Reference: <https://glitchcity.wiki/wiki/Silent_Indigo_Plateau>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom16(h: &mut TestHarness, addr: u16) -> u16 {
    let lo = rom(h, addr) as u16;
    let hi = rom(h, addr + 1) as u16;
    (hi << 8) | lo
}

// ─── Opcode constants ────────────────────────────────────────────────

const LD_HL_NN: u8 = 0x21; // ld hl, nn
const BIT_1_HL: [u8; 2] = [0xCB, 0x4E]; // bit 1, [hl] (BIT_NO_MAP_MUSIC = bit 1)
const JR_Z: u8 = 0x28; // jr z, n
const LD_A_NN: u8 = 0xFA; // ld a, [nn]
const AND_A: u8 = 0xA7; // and a
const CALL_NN: u8 = 0xCD; // call nn

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn end_of_battle_in_bank_04() {
    assert_eq!(sym_bank("EndOfBattle"), 0x04);
}

#[test]
fn victory_replay_check_present() {
    // Between EndOfBattle.evolution and EndOfBattle.skipVictoryReplay,
    // we should find:
    //   ld hl, wStatusFlags7       ($21 lo hi)
    //   bit BIT_NO_MAP_MUSIC, [hl] ($CB $4E)
    //   jr z, .skipVictoryReplay   ($28 nn)
    //   ld a, [wEvolutionOccurred] ($FA lo hi)
    //   and a                      ($A7)
    //   jr z, .skipVictoryReplay   ($28 nn)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x04);

    let evo = sym_addr("EndOfBattle.evolution");
    let skip = sym_addr("EndOfBattle.skipVictoryReplay");

    // Scan for the ld hl, wStatusFlags7 pattern
    let status_flags7 = sym_addr("wStatusFlags7");
    let evo_occurred = sym_addr("wEvolutionOccurred");

    let mut found_status_check = false;
    let mut found_evo_check = false;

    for addr in evo..skip {
        // Check for ld hl, wStatusFlags7
        if rom(&mut h, addr) == LD_HL_NN && rom16(&mut h, addr + 1) == status_flags7 {
            found_status_check = true;
            // bit 1, [hl] should follow
            assert_eq!(
                rom(&mut h, addr + 3),
                BIT_1_HL[0],
                "Expected CB prefix for bit instruction"
            );
            assert_eq!(
                rom(&mut h, addr + 4),
                BIT_1_HL[1],
                "Expected bit 1, [hl] (BIT_NO_MAP_MUSIC)"
            );
            // jr z should follow
            assert_eq!(rom(&mut h, addr + 5), JR_Z, "Expected jr z after bit check");
        }
        // Check for ld a, [wEvolutionOccurred]
        if rom(&mut h, addr) == LD_A_NN && rom16(&mut h, addr + 1) == evo_occurred {
            found_evo_check = true;
            // and a should follow
            assert_eq!(rom(&mut h, addr + 3), AND_A, "Expected `and a` after ld a");
            // jr z should follow
            assert_eq!(
                rom(&mut h, addr + 4),
                JR_Z,
                "Expected `jr z` after and a"
            );
        }
    }

    assert!(
        found_status_check,
        "BIT_NO_MAP_MUSIC check not found between .evolution and .skipVictoryReplay"
    );
    assert!(
        found_evo_check,
        "wEvolutionOccurred check not found between .evolution and .skipVictoryReplay"
    );
}

#[test]
fn stop_all_music_called_before_replay() {
    // call StopAllMusic should appear between .evolution and .skipVictoryReplay
    let mut h = TestHarness::new();
    h.select_rom_bank(0x04);

    let evo = sym_addr("EndOfBattle.evolution");
    let skip = sym_addr("EndOfBattle.skipVictoryReplay");
    let stop_all = sym_addr("StopAllMusic");

    let mut found = false;
    for addr in evo..skip.saturating_sub(2) {
        if rom(&mut h, addr) == CALL_NN && rom16(&mut h, addr + 1) == stop_all {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "call StopAllMusic not found in victory replay code"
    );
}

#[test]
fn play_music_called_for_replay() {
    // call PlayMusic should appear between .evolution and .skipVictoryReplay
    let mut h = TestHarness::new();
    h.select_rom_bank(0x04);

    let evo = sym_addr("EndOfBattle.evolution");
    let skip = sym_addr("EndOfBattle.skipVictoryReplay");
    let play_music = sym_addr("PlayMusic");

    let mut found = false;
    for addr in evo..skip.saturating_sub(2) {
        if rom(&mut h, addr) == CALL_NN && rom16(&mut h, addr + 1) == play_music {
            found = true;
            break;
        }
    }
    assert!(found, "call PlayMusic not found in victory replay code");
}

#[test]
fn skip_victory_replay_label_exists() {
    let evo = sym_addr("EndOfBattle.evolution");
    let skip = sym_addr("EndOfBattle.skipVictoryReplay");
    let reset = sym_addr("EndOfBattle.resetVariables");

    assert!(skip > evo, ".skipVictoryReplay should be after .evolution");
    assert!(
        reset > skip,
        ".resetVariables should be after .skipVictoryReplay"
    );
}
