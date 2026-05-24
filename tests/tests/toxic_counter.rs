//! ROM byte tests for Toxic counter glitch fixes.
//!
//! Bug 1 (Toxic + Leech Seed): Leech Seed damage uses the same N counter
//! as Toxic, causing escalating drain each turn instead of flat maxHP/16.
//!
//! Bug 2 (Toxic + Rest): Rest clears the status byte but doesn't reset
//! the Toxic N counter or BADLY_POISONED flag, so subsequent poison/burn/
//! Leech Seed damage escalates from the old N value.
//!
//! Fix 1: Leech Seed calls `HandlePoisonBurnLeechSeed_DecreaseOwnHP_NoToxic`
//! which sets a flag to skip the Toxic multiplier.
//!
//! Fix 2: Rest now resets `wXToxicCounter` to 0 and clears BADLY_POISONED.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Toxic_counter_glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Read a 16-bit little-endian value from ROM
fn rom16(h: &mut TestHarness, addr: u16) -> u16 {
    let lo = rom(h, addr) as u16;
    let hi = rom(h, addr + 1) as u16;
    (hi << 8) | lo
}

// ─── Opcode constants ────────────────────────────────────────────────

const CALL_NN: u8 = 0xCD; // call nn
const LD_A_1: [u8; 2] = [0x3E, 0x01]; // ld a, 1
const XOR_A: u8 = 0xAF; // xor a
const PUSH_AF: u8 = 0xF5; // push af
const POP_AF: u8 = 0xF1; // pop af
const JR_NZ: u8 = 0x20; // jr nz, n
const RES_0_HL: [u8; 2] = [0xCB, 0x86]; // res 0, [hl] (BADLY_POISONED = bit 0)

// ─── Fix 1: Leech Seed calls NoToxic variant ────────────────────────

#[test]
fn no_toxic_entry_point_exists() {
    assert_eq!(
        sym_bank("HandlePoisonBurnLeechSeed_DecreaseOwnHP_NoToxic"),
        0x0F
    );
    // NoToxic should be 3 bytes before the normal entry point
    // (ld a, 1 = 2 bytes + db $06 = 1 byte)
    let no_toxic = sym_addr("HandlePoisonBurnLeechSeed_DecreaseOwnHP_NoToxic");
    let normal = sym_addr("HandlePoisonBurnLeechSeed_DecreaseOwnHP");
    assert_eq!(
        normal - no_toxic,
        3,
        "NoToxic entry should be 3 bytes before normal entry"
    );
}

#[test]
fn no_toxic_entry_sets_flag() {
    // NoToxic: ld a, 1 ($3E $01) / db $06 (ld b, n trick)
    // Normal:  xor a ($AF)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let no_toxic = sym_addr("HandlePoisonBurnLeechSeed_DecreaseOwnHP_NoToxic");
    assert_eq!(rom(&mut h, no_toxic), LD_A_1[0], "Expected `ld a, 1`");
    assert_eq!(rom(&mut h, no_toxic + 1), LD_A_1[1]);
    assert_eq!(
        rom(&mut h, no_toxic + 2),
        0x06,
        "Expected db $06 (ld b, n trick)"
    );

    let normal = sym_addr("HandlePoisonBurnLeechSeed_DecreaseOwnHP");
    assert_eq!(
        rom(&mut h, normal),
        XOR_A,
        "Expected `xor a` at normal entry"
    );
}

#[test]
fn normal_entry_has_push_af() {
    // After xor a at normal entry, the next instructions should include push af
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let normal = sym_addr("HandlePoisonBurnLeechSeed_DecreaseOwnHP");
    // xor a (1) + push hl (1) + push hl (1) + push af (1) = offset +3
    assert_eq!(
        rom(&mut h, normal + 3),
        PUSH_AF,
        "Expected `push af` after push hl / push hl"
    );
}

#[test]
fn non_zero_damage_checks_flag() {
    // At .nonZeroDamage: pop af / and a / jr nz, .noToxic
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let nzd = sym_addr("HandlePoisonBurnLeechSeed_DecreaseOwnHP.nonZeroDamage");
    assert_eq!(rom(&mut h, nzd), POP_AF, "Expected `pop af`");
    assert_eq!(rom(&mut h, nzd + 1), 0xA7, "Expected `and a` (opcode $A7)");
    assert_eq!(rom(&mut h, nzd + 2), JR_NZ, "Expected `jr nz`");
}

#[test]
fn leech_seed_calls_no_toxic() {
    // The Leech Seed damage path should call HandlePoisonBurnLeechSeed_DecreaseOwnHP_NoToxic
    // We find it by looking for `call NoToxic` followed by `call IncreaseEnemyHP`
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let no_toxic_addr = sym_addr("HandlePoisonBurnLeechSeed_DecreaseOwnHP_NoToxic");
    let inc_hp_addr = sym_addr("HandlePoisonBurnLeechSeed_IncreaseEnemyHP");

    // Scan HandlePoisonBurnLeechSeed for `call NoToxic / call IncreaseEnemyHP` pattern
    let start = sym_addr("HandlePoisonBurnLeechSeed");
    let end = sym_addr("HandlePoisonBurnLeechSeed_DecreaseOwnHP_NoToxic");

    let mut found = false;
    for addr in start..end.saturating_sub(6) {
        if rom(&mut h, addr) == CALL_NN
            && rom16(&mut h, addr + 1) == no_toxic_addr
            && rom(&mut h, addr + 3) == CALL_NN
            && rom16(&mut h, addr + 4) == inc_hp_addr
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Leech Seed path should call NoToxic then IncreaseEnemyHP"
    );
}

// ─── Fix 2: Rest resets Toxic counter ────────────────────────────────

#[test]
fn rest_resets_toxic_counter() {
    // HealEffect_.resetToxicPlayer should contain:
    //   res BADLY_POISONED, [hl]  ($CB $86)
    //   xor a                     ($AF)
    //   ld [de], a                ($12)
    let mut h = TestHarness::new();
    let bank = sym_bank("HealEffect_.resetToxicPlayer");
    h.select_rom_bank(bank);

    let rtp = sym_addr("HealEffect_.resetToxicPlayer");
    assert_eq!(rom(&mut h, rtp), RES_0_HL[0], "Expected CB prefix");
    assert_eq!(rom(&mut h, rtp + 1), RES_0_HL[1], "Expected res 0, [hl]");
    assert_eq!(rom(&mut h, rtp + 2), XOR_A, "Expected `xor a`");
    assert_eq!(
        rom(&mut h, rtp + 3),
        0x12,
        "Expected `ld [de], a` (zero the counter)"
    );
}

#[test]
fn rest_effect_in_banked_rom() {
    let bank = sym_bank("HealEffect_.resetToxicPlayer");
    assert_ne!(bank, 0, "Rest Toxic reset should be in banked ROM");
}
