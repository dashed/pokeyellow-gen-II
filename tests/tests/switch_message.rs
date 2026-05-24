//! Emulator-based tests for the switch-out message HP underflow fix.
//!
//! The bug: When computing switch-out messages, the code subtracts currentHP
//! from lastSwitchInHP (16-bit unsigned). If the enemy healed since switch-in,
//! this underflows, producing a garbage percentage and an incorrect message
//! (e.g. "Good!" when no damage was dealt).
//!
//! The fix: After `sbc b` (high byte subtraction), check carry flag. If set
//! (underflow), skip the Multiply/Divide computation entirely and return
//! EnoughText directly via .gainedHP (pop bc/de, load HL, ret).

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Which switch-out message text was selected.
#[derive(Debug, PartialEq)]
enum SwitchMessage {
    /// "Enough! Come back!" — 0% damage (HP stayed the same or enemy healed)
    Enough,
    /// "Come back!" — 1-29% damage
    ComeBack,
    /// "OK! Come back!" — 30-69% damage
    OKExclamation,
    /// "Good! Come back!" — 70%+ damage
    Good,
}

/// Write a 16-bit big-endian value to two consecutive addresses.
fn write_u16_be(h: &mut TestHarness, addr: u16, val: u16) {
    h.write_mem(addr, (val >> 8) as u8);
    h.write_mem(addr + 1, (val & 0xFF) as u8);
}

/// Run the PlayerMon2Text text_asm callback with given HP values.
/// Returns which switch-out message text was selected.
fn check_switch_message(current_hp: u16, switch_in_hp: u16, max_hp: u16) -> SwitchMessage {
    let bank = sym_bank("PlayerMon2Text");
    // text_asm entry, 5 bytes after PlayerMon2Text
    let text_asm_entry = sym_addr("PlayerMon2Text") + 5;
    let enough_text = sym_addr("EnoughText");
    let come_back_text = sym_addr("ComeBackText");
    let ok_exclamation_text = sym_addr("OKExclamationText");
    let good_text = sym_addr("GoodText");

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(bank);
    // Tell Bankswitch/homecall which bank to restore after far calls
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    // Set up trap
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    // Stack: push TRAP_ADDR as return address for the final `ret`
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Set WRAM values
    write_u16_be(&mut h, sym_addr("wEnemyMonHP"), current_hp);
    write_u16_be(&mut h, sym_addr("wLastSwitchInEnemyMonHP"), switch_in_hp);
    write_u16_be(&mut h, sym_addr("wEnemyMonMaxHP"), max_hp);

    // Start at the text_asm callback entry (after text_far/text_asm bytes)
    h.set_pc(text_asm_entry);

    // Run until we reach the trap (after the callback's ret)
    h.step_to(TRAP_ADDR);

    // HL holds the selected text label pointer
    let hl = h.gb.cpu_i().hl();
    if hl == enough_text {
        SwitchMessage::Enough
    } else if hl == come_back_text {
        SwitchMessage::ComeBack
    } else if hl == ok_exclamation_text {
        SwitchMessage::OKExclamation
    } else if hl == good_text {
        SwitchMessage::Good
    } else {
        panic!("Unexpected HL=${hl:04X} — not a known text label")
    }
}

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_entry_is_push_de() {
    // text_asm entry, 5 bytes after PlayerMon2Text
    let text_asm_entry = sym_addr("PlayerMon2Text") + 5;

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("PlayerMon2Text"));

    // text_asm callback should start with push de ($D5)
    let opcode = h.read_mem(text_asm_entry);
    assert_eq!(
        opcode, 0xD5,
        "Expected push de ($D5) at TEXT_ASM_ENTRY, got ${opcode:02X}"
    );
}

#[test]
fn rom_bytes_jr_c_underflow_check() {
    // text_asm entry, 5 bytes after PlayerMon2Text
    let text_asm_entry = sym_addr("PlayerMon2Text") + 5;

    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("PlayerMon2Text"));

    // After `sbc b` there should be `jr c, .gainedHP` ($38, offset)
    // Layout from push de:
    //  +0: push de ($D5)
    //  +1: push bc ($C5)
    //  +2: ld hl, wEnemyMonHP+1 ($21, lo, hi) = 3 bytes
    //  +5: ld de, wLastSwitchInEnemyMonHP+1 ($11, lo, hi) = 3 bytes
    //  +8: ld b,[hl] ($46)
    //  +9: dec hl ($2B)
    // +10: ld a,[de] ($1A)
    // +11: sub b ($90)
    // +12: ldh [hMultiplicand+2],a ($E0, $98) = 2 bytes
    // +14: dec de ($1B)
    // +15: ld b,[hl] ($46)
    // +16: ld a,[de] ($1A)
    // +17: sbc b ($98)
    // +18: jr c, .gainedHP ($38, offset)
    let sbc_b = h.read_mem(text_asm_entry + 17);
    let jr_c = h.read_mem(text_asm_entry + 18);
    assert_eq!(sbc_b, 0x98, "Expected sbc b ($98) at +17, got ${sbc_b:02X}");
    assert_eq!(jr_c, 0x38, "Expected jr c ($38) at +18, got ${jr_c:02X}");
}

// ─── Bug scenario: enemy gained HP (underflow) ─────────────────────

#[test]
fn underflow_small_hp_gain_returns_enough() {
    // switchInHP=100, currentHP=120, maxHP=200
    // Enemy gained 20 HP → underflow → should clamp to 0% → Enough
    let result = check_switch_message(120, 100, 200);
    assert_eq!(
        result,
        SwitchMessage::Enough,
        "Enemy gained HP: should select Enough (0% damage)"
    );
}

#[test]
fn underflow_large_hp_gain_returns_enough() {
    // switchInHP=50, currentHP=200, maxHP=300
    // Enemy gained 150 HP → underflow → should clamp to 0% → Enough
    let result = check_switch_message(200, 50, 300);
    assert_eq!(
        result,
        SwitchMessage::Enough,
        "Enemy gained large HP: should select Enough (0% damage)"
    );
}

#[test]
fn underflow_cross_byte_boundary_returns_enough() {
    // switchInHP=$00FF (255), currentHP=$0100 (256), maxHP=$0200 (512)
    // Gain of 1 HP that crosses the byte boundary
    let result = check_switch_message(0x0100, 0x00FF, 0x0200);
    assert_eq!(
        result,
        SwitchMessage::Enough,
        "HP gain crossing byte boundary: should select Enough"
    );
}

// ─── Normal behavior: HP stayed the same ────────────────────────────

#[test]
fn no_damage_returns_enough() {
    // switchInHP=200, currentHP=200, maxHP=200
    let result = check_switch_message(200, 200, 200);
    assert_eq!(
        result,
        SwitchMessage::Enough,
        "No damage dealt: should select Enough"
    );
}

// ─── Normal behavior: damage dealt ──────────────────────────────────

#[test]
fn small_damage_returns_come_back() {
    // switchInHP=200, currentHP=180, maxHP=200
    // damage=20, percentage ≈ 10% → ComeBack (1-29%)
    let result = check_switch_message(180, 200, 200);
    assert_eq!(
        result,
        SwitchMessage::ComeBack,
        "10% damage: should select ComeBack"
    );
}

#[test]
fn medium_damage_returns_ok_exclamation() {
    // switchInHP=200, currentHP=100, maxHP=200
    // damage=100, percentage ≈ 50% → OKExclamation (30-69%)
    let result = check_switch_message(100, 200, 200);
    assert_eq!(
        result,
        SwitchMessage::OKExclamation,
        "50% damage: should select OKExclamation"
    );
}

#[test]
fn large_damage_returns_good() {
    // switchInHP=200, currentHP=40, maxHP=200
    // damage=160, percentage ≈ 80% → Good (70%+)
    let result = check_switch_message(40, 200, 200);
    assert_eq!(
        result,
        SwitchMessage::Good,
        "80% damage: should select Good"
    );
}
