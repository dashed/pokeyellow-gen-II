//! ROM byte tests for the Pikachu cry in link battles fix.
//!
//! Bug: When an enemy sends out a Pikachu in battle (trainer or link),
//! EnemySendOutFirstMon always calls PlayCry, playing the standard
//! electronic cry instead of Pikachu's digitized voice cry.
//! This is inconsistent with the player send-out path (SendOutMon),
//! which checks IsThisPartyMonStarterPikachu and plays the voice cry.
//!
//! Fix: Before calling PlayCry, check if the enemy species is PIKACHU.
//! If so, play PikachuCry11 via PlayPikachuSoundClip instead.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Pikachu_cry_in_link_battles>

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

const CP_N: u8 = 0xFE; // cp n
const PIKACHU: u8 = 0x54; // PIKACHU species constant
const JR_NZ: u8 = 0x20; // jr nz, n
const LD_E_N: u8 = 0x1E; // ld e, n
const CALL_NN: u8 = 0xCD; // call nn

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn enemy_send_out_in_bank_0f() {
    assert_eq!(sym_bank("EnemySendOutFirstMon"), 0x0F);
}

#[test]
fn pikachu_species_check_before_cry() {
    // Between .next4 and .notEnemyPikachu, there should be a
    // `cp PIKACHU` ($FE $54) instruction
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let next4 = sym_addr("EnemySendOutFirstMon.next4");
    let not_pika = sym_addr("EnemySendOutFirstMon.notEnemyPikachu");

    let mut found = false;
    for addr in next4..not_pika {
        if rom(&mut h, addr) == CP_N && rom(&mut h, addr + 1) == PIKACHU {
            found = true;
            // jr nz should follow
            assert_eq!(
                rom(&mut h, addr + 2),
                JR_NZ,
                "Expected `jr nz` after `cp PIKACHU`"
            );
            break;
        }
    }
    assert!(
        found,
        "`cp PIKACHU` not found between .next4 and .notEnemyPikachu"
    );
}

#[test]
fn pikachu_voice_cry_loaded() {
    // Between the cp PIKACHU check and .notEnemyPikachu, there should be
    // `ld e, n` loading the PikachuCry11 index
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let next4 = sym_addr("EnemySendOutFirstMon.next4");
    let not_pika = sym_addr("EnemySendOutFirstMon.notEnemyPikachu");

    // Compute expected cry index: (PikachuCry11_id - PikachuCriesPointerTable) / 3
    let cry11_id = sym_addr("PikachuCry11_id");
    let table = sym_addr("PikachuCriesPointerTable");
    let expected_index = ((cry11_id - table) / 3) as u8;

    let mut found = false;
    for addr in next4..not_pika {
        if rom(&mut h, addr) == LD_E_N && rom(&mut h, addr + 1) == expected_index {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "`ld e, PikachuCry11 index` (ld e, {:#04X}) not found",
        expected_index
    );
}

#[test]
fn callfar_play_pikachu_sound_clip_present() {
    // Between .next4 and .notEnemyPikachu, there should be a
    // call to Bankswitch (callfar expands to: ld hl, addr / ld b, BANK / call Bankswitch)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let next4 = sym_addr("EnemySendOutFirstMon.next4");
    let not_pika = sym_addr("EnemySendOutFirstMon.notEnemyPikachu");
    let bankswitch = sym_addr("Bankswitch");

    let mut found = false;
    for addr in next4..not_pika.saturating_sub(2) {
        if rom(&mut h, addr) == CALL_NN && rom16(&mut h, addr + 1) == bankswitch {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "`call Bankswitch` (callfar PlayPikachuSoundClip) not found"
    );
}

#[test]
fn regular_play_cry_still_present() {
    // At .notEnemyPikachu, `call PlayCry` should still exist for non-Pikachu species
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_pika = sym_addr("EnemySendOutFirstMon.notEnemyPikachu");
    let play_cry = sym_addr("PlayCry");

    assert_eq!(
        rom(&mut h, not_pika),
        CALL_NN,
        "Expected `call PlayCry` at .notEnemyPikachu"
    );
    assert_eq!(
        rom16(&mut h, not_pika + 1),
        play_cry,
        "call target should be PlayCry"
    );
}
