//! ROM byte tests for the AI trainer HUD update fix.
//!
//! Bug: When an AI trainer uses a healing item (Potion, Super Potion, Hyper
//! Potion, Full Restore, Full Heal), the function DrawEnemyHUDAndHPBar is
//! never called. This means:
//! - Status icons don't update until after the player's turn (Full Heal,
//!   Full Restore)
//! - HP bar color doesn't update when HP crosses a color threshold (all
//!   healing items)
//!
//! Fix: Add `callfar DrawEnemyHUDAndHPBar` via a shared
//! `DrawEnemyHUDAndDecrementAICount` label:
//! 1. AIPrintItemUseAndUpdateHPBar falls through to the shared label (+8 bytes)
//! 2. AIUseFullHeal jumps to the shared label (+6 bytes)
//!
//! +14 bytes in bank $0E.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// FULL_HEAL item constant ($34).
const FULL_HEAL: u8 = 0x34;

// ─── Opcode constants ────────────────────────────────────────────────

const LD_HL_NN: u8 = 0x21;
const LD_B_N: u8 = 0x06;
const CALL_NN: u8 = 0xCD;
const JP_NN: u8 = 0xC3;
const LD_A_N: u8 = 0x3E;
const LD_NN_A: u8 = 0xEA;

// ─── Helpers ─────────────────────────────────────────────────────────

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("AIPrintItemUseAndUpdateHPBar"));
    h
}

fn read_u16_le(h: &mut TestHarness, addr: u16) -> u16 {
    let lo = rom(h, addr) as u16;
    let hi = rom(h, addr + 1) as u16;
    (hi << 8) | lo
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

// ─── Shared label: DrawEnemyHUDAndDecrementAICount ───────────────────

#[test]
fn rom_bytes_shared_label_has_callfar_draw_enemy_hud() {
    let mut h = rom_harness();
    assert_callfar_at(
        &mut h,
        sym_addr("DrawEnemyHUDAndDecrementAICount"),
        sym_addr("DrawEnemyHUDAndHPBar"),
        sym_bank("DrawEnemyHUDAndHPBar"),
    );
}

#[test]
fn rom_bytes_shared_label_ends_with_jp_decrement_ai_count() {
    let mut h = rom_harness();
    let draw_enemy_hud_and_decrement = sym_addr("DrawEnemyHUDAndDecrementAICount");
    let decrement_ai_count = sym_addr("DecrementAICount");
    // callfar is 8 bytes, jp follows at +8
    let jp_addr = draw_enemy_hud_and_decrement + 8;
    assert_eq!(rom(&mut h, jp_addr), JP_NN, "Expected jp nn");
    assert_eq!(
        read_u16_le(&mut h, jp_addr + 1),
        decrement_ai_count,
        "Expected jp DecrementAICount (${decrement_ai_count:04X})"
    );
}

// ─── Potion/Full Restore path: AIPrintItemUseAndUpdateHPBar ──────────

#[test]
fn rom_bytes_potion_path_falls_through_to_shared_label() {
    // AIPrintItemUseAndUpdateHPBar: call(3) + hlcoord(3) + xor(1) + ld[nn],a(3) + predef(5) = 15
    // The next byte should be the start of DrawEnemyHUDAndDecrementAICount
    let ai_print = sym_addr("AIPrintItemUseAndUpdateHPBar");
    let draw_hud_decrement = sym_addr("DrawEnemyHUDAndDecrementAICount");
    let fallthrough_addr = ai_print + 15;
    assert_eq!(
        fallthrough_addr, draw_hud_decrement,
        "AIPrintItemUseAndUpdateHPBar should fall through to shared label"
    );
}

// ─── Full Heal path: AIUseFullHeal ───────────────────────────────────

#[test]
fn rom_bytes_full_heal_loads_full_heal_item() {
    let mut h = rom_harness();
    let ai_use_full_heal = sym_addr("AIUseFullHeal");
    // After two calls (3+3=6 bytes): ld a, FULL_HEAL
    let ld_a_addr = ai_use_full_heal + 6;
    assert_eq!(rom(&mut h, ld_a_addr), LD_A_N, "Expected ld a, n");
    assert_eq!(
        rom(&mut h, ld_a_addr + 1),
        FULL_HEAL,
        "Expected FULL_HEAL (${FULL_HEAL:02X})"
    );
    // ld [wAIItem], a
    let ld_nn_a_addr = ld_a_addr + 2;
    assert_eq!(rom(&mut h, ld_nn_a_addr), LD_NN_A, "Expected ld [nn], a");
    let w_ai_item = sym_addr("wAIItem");
    assert_eq!(
        read_u16_le(&mut h, ld_nn_a_addr + 1),
        w_ai_item,
        "Expected wAIItem (${w_ai_item:04X})"
    );
}

#[test]
fn rom_bytes_full_heal_calls_ai_print_item_use_() {
    let mut h = rom_harness();
    let ai_use_full_heal = sym_addr("AIUseFullHeal");
    let ai_print_item_use_ = sym_addr("AIPrintItemUse_");
    // After: call(3) + call(3) + ld a,n(2) + ld[nn],a(3) = 11 bytes
    let call_addr = ai_use_full_heal + 11;
    assert_eq!(rom(&mut h, call_addr), CALL_NN, "Expected call nn");
    assert_eq!(
        read_u16_le(&mut h, call_addr + 1),
        ai_print_item_use_,
        "Expected call AIPrintItemUse_ (${ai_print_item_use_:04X})"
    );
}

#[test]
fn rom_bytes_full_heal_jumps_to_shared_label() {
    let mut h = rom_harness();
    let ai_use_full_heal = sym_addr("AIUseFullHeal");
    let draw_enemy_hud_and_decrement = sym_addr("DrawEnemyHUDAndDecrementAICount");
    // After: call(3) + call(3) + ld a,n(2) + ld[nn],a(3) + call(3) = 14 bytes
    let jp_addr = ai_use_full_heal + 14;
    assert_eq!(rom(&mut h, jp_addr), JP_NN, "Expected jp nn");
    assert_eq!(
        read_u16_le(&mut h, jp_addr + 1),
        draw_enemy_hud_and_decrement,
        "Expected jp DrawEnemyHUDAndDecrementAICount (${draw_enemy_hud_and_decrement:04X})"
    );
}
