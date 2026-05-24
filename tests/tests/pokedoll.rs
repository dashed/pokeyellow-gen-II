//! ROM byte tests for the Poké Doll ghost Marowak sequence break fix.
//!
//! Bug: Using a Poké Doll in the ghost Marowak battle ends the battle
//! with wBattleResult == 0 (victory). The post-battle script in
//! PokemonTower6F checks wBattleResult and sets EVENT_BEAT_GHOST_MAROWAK,
//! allowing the player to progress without acquiring the Silph Scope.
//!
//! Fix: Add `callfar IsGhostBattle` before the escape logic. If
//! IsGhostBattle returns Z (ghost battle), jump to ItemUseNotTime to
//! reject the item. This mirrors the existing Poké Ball ghost check.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Go_past_the_Marowak_ghost_without_a_Silph_Scope>
//!   - <https://bulbapedia.bulbagarden.net/wiki/Marowak_(ghost)>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

// ─── Opcode constants ────────────────────────────────────────────────

const LD_A_NN: u8 = 0xFA; // ld a, [nn]
const DEC_A: u8 = 0x3D;
const JP_NZ_NN: u8 = 0xC2;
const JP_Z_NN: u8 = 0xCA;
const JP_NN: u8 = 0xC3;
const LD_HL_NN: u8 = 0x21;
const LD_B_N: u8 = 0x06;
const CALL_NN: u8 = 0xCD;
const LD_A_N: u8 = 0x3E;
const LD_NN_A: u8 = 0xEA;

// ─── Helpers ─────────────────────────────────────────────────────────

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn read_u16_le(h: &mut TestHarness, addr: u16) -> u16 {
    let lo = rom(h, addr) as u16;
    let hi = rom(h, addr + 1) as u16;
    (hi << 8) | lo
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("ItemUsePokeDoll"));
    h
}

/// Verify a `callfar` macro expansion at the given ROM address.
/// callfar expands to: ld hl, target (3) + ld b, bank (2) + call Bankswitch (3) = 8 bytes.
fn assert_callfar_at(h: &mut TestHarness, addr: u16, target: u16, bank: u8) {
    let bankswitch = sym_addr("Bankswitch");
    assert_eq!(
        rom(h, addr),
        LD_HL_NN,
        "Expected ld hl, nn at callfar start"
    );
    assert_eq!(
        read_u16_le(h, addr + 1),
        target,
        "Expected callfar target ${target:04X}"
    );
    assert_eq!(rom(h, addr + 3), LD_B_N, "Expected ld b, n for bank");
    assert_eq!(rom(h, addr + 4), bank, "Expected bank ${bank:02X}");
    assert_eq!(rom(h, addr + 5), CALL_NN, "Expected call nn for Bankswitch");
    assert_eq!(
        read_u16_le(h, addr + 6),
        bankswitch,
        "Expected Bankswitch (${bankswitch:04X})"
    );
}

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn poke_doll_is_in_bank_03() {
    assert_eq!(
        sym_bank("ItemUsePokeDoll"),
        0x03,
        "ItemUsePokeDoll should be in bank $03"
    );
}

#[test]
fn poke_doll_starts_with_battle_check() {
    // ItemUsePokeDoll: ld a, [wIsInBattle] / dec a / jp nz, ItemUseNotTime
    let mut h = rom_harness();
    let base = sym_addr("ItemUsePokeDoll");
    let not_time = sym_addr("ItemUseNotTime");

    assert_eq!(rom(&mut h, base), LD_A_NN, "ld a, [nn] at +0");
    assert_eq!(rom(&mut h, base + 3), DEC_A, "dec a at +3");
    assert_eq!(rom(&mut h, base + 4), JP_NZ_NN, "jp nz, nn at +4");
    assert_eq!(
        read_u16_le(&mut h, base + 5),
        not_time,
        "jp nz target should be ItemUseNotTime"
    );
}

#[test]
fn poke_doll_has_callfar_is_ghost_battle() {
    // After the battle check (7 bytes), callfar IsGhostBattle at +7
    let mut h = rom_harness();
    let base = sym_addr("ItemUsePokeDoll");

    assert_callfar_at(
        &mut h,
        base + 7,
        sym_addr("IsGhostBattle"),
        sym_bank("IsGhostBattle"),
    );
}

#[test]
fn poke_doll_rejects_ghost_battle_with_jp_z() {
    // After callfar (8 bytes at +7), jp z, ItemUseNotTime at +15
    let mut h = rom_harness();
    let base = sym_addr("ItemUsePokeDoll");
    let not_time = sym_addr("ItemUseNotTime");

    assert_eq!(rom(&mut h, base + 15), JP_Z_NN, "jp z, nn at +15");
    assert_eq!(
        read_u16_le(&mut h, base + 16),
        not_time,
        "jp z target should be ItemUseNotTime"
    );
}

#[test]
fn poke_doll_escape_logic_preserved() {
    // After the ghost check (jp z at +15, 3 bytes), the original escape
    // logic: ld a, $01 / ld [wEscapedFromBattle], a / jp PrintItemUseTextAndRemoveItem
    let mut h = rom_harness();
    let base = sym_addr("ItemUsePokeDoll");

    // ld a, $01 at +18
    assert_eq!(rom(&mut h, base + 18), LD_A_N, "ld a, n at +18");
    assert_eq!(rom(&mut h, base + 19), 0x01, "immediate $01 at +19");

    // ld [wEscapedFromBattle], a at +20
    assert_eq!(rom(&mut h, base + 20), LD_NN_A, "ld [nn], a at +20");

    // jp PrintItemUseTextAndRemoveItem at +23
    assert_eq!(rom(&mut h, base + 23), JP_NN, "jp nn at +23");
    assert_eq!(
        read_u16_le(&mut h, base + 24),
        sym_addr("PrintItemUseTextAndRemoveItem"),
        "jp target should be PrintItemUseTextAndRemoveItem"
    );
}

#[test]
fn poke_doll_total_size() {
    // Total: 7 (battle check) + 8 (callfar) + 3 (jp z) + 2 (ld a) + 3 (ld [nn],a) + 3 (jp) = 26 bytes
    // Verify the next function (ItemUseGuardSpec) starts right after
    let base = sym_addr("ItemUsePokeDoll");
    let guard_spec = sym_addr("ItemUseGuardSpec");

    assert_eq!(
        guard_spec - base,
        26,
        "ItemUsePokeDoll should be 26 bytes (was 15 before fix)"
    );
}
