//! ROM byte tests for the Mimic PP display glitch fix.
//!
//! Bug: When Mimic copies a move, the fight menu shows the copied move's
//! max PP instead of Mimic's max PP (10).  GetMaxPP reads the move ID from
//! wBattleMonMoves (which has the copied move after Mimic), looks up its
//! base PP, and displays that.  But the current PP byte (wBattleMonPP) was
//! never changed — it still tracks Mimic's remaining uses.  This creates
//! displays like "9/5 PP" (9 Mimic PP left, Horn Drill's max of 5).
//!
//! Fix: In PrintMenuItem, before calling GetMaxPP, check if the
//! corresponding party move slot still has MIMIC.  If so, use
//! PLAYER_PARTY_DATA for GetMaxPP so it looks up Mimic's base PP (10).
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Mimic_PP_glitch>

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
const MIMIC: u8 = 0x66; // MIMIC move constant
const JR_NZ: u8 = 0x20; // jr nz, n
const LD_A_N: u8 = 0x3E; // ld a, n
const LD_HL_NN: u8 = 0x21; // ld hl, nn
const CALL_NN: u8 = 0xCD; // call nn

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn print_menu_item_in_bank_0f() {
    assert_eq!(sym_bank("PrintMenuItem"), 0x0F);
}

#[test]
fn mimic_check_present() {
    // Between .notDisabled and .gotMaxPPSource, there should be:
    //   cp MIMIC  ($FE $66)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_disabled = sym_addr("PrintMenuItem.notDisabled");
    let got_source = sym_addr("PrintMenuItem.gotMaxPPSource");

    let mut found = false;
    for addr in not_disabled..got_source {
        if rom(&mut h, addr) == CP_N && rom(&mut h, addr + 1) == MIMIC {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "`cp MIMIC` not found between .notDisabled and .gotMaxPPSource"
    );
}

#[test]
fn conditional_data_source_present() {
    // After cp MIMIC, there should be:
    //   ld a, BATTLE_MON_DATA    ($3E $04)
    //   jr nz, .gotMaxPPSource   ($20 nn)
    //   ld a, PLAYER_PARTY_DATA  ($3E $00)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_disabled = sym_addr("PrintMenuItem.notDisabled");
    let got_source = sym_addr("PrintMenuItem.gotMaxPPSource");

    let battle_mon_data = 0x04_u8; // BATTLE_MON_DATA
    let player_party_data = 0x00_u8; // PLAYER_PARTY_DATA

    let mut found = false;
    for addr in not_disabled..got_source.saturating_sub(5) {
        if rom(&mut h, addr) == CP_N
            && rom(&mut h, addr + 1) == MIMIC
            && rom(&mut h, addr + 2) == LD_A_N
            && rom(&mut h, addr + 3) == battle_mon_data
            && rom(&mut h, addr + 4) == JR_NZ
            && rom(&mut h, addr + 6) == LD_A_N
            && rom(&mut h, addr + 7) == player_party_data
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Conditional BATTLE_MON_DATA / PLAYER_PARTY_DATA pattern not found"
    );
}

#[test]
fn party_moves_lookup_present() {
    // ld hl, wPartyMon1Moves should be present (party data offset calculation)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_disabled = sym_addr("PrintMenuItem.notDisabled");
    let got_source = sym_addr("PrintMenuItem.gotMaxPPSource");
    let party_moves = sym_addr("wPartyMon1Moves");

    let mut found = false;
    for addr in not_disabled..got_source {
        if rom(&mut h, addr) == LD_HL_NN && rom16(&mut h, addr + 1) == party_moves {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "`ld hl, wPartyMon1Moves` not found (party move lookup missing)"
    );
}

#[test]
fn add_n_times_called() {
    // call AddNTimes should be present for party mon offset calculation
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_disabled = sym_addr("PrintMenuItem.notDisabled");
    let got_source = sym_addr("PrintMenuItem.gotMaxPPSource");
    let add_n_times = sym_addr("AddNTimes");

    let mut found = false;
    for addr in not_disabled..got_source.saturating_sub(2) {
        if rom(&mut h, addr) == CALL_NN && rom16(&mut h, addr + 1) == add_n_times {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "`call AddNTimes` not found (party offset calculation)"
    );
}
