//! ROM byte tests for the Red bar glitch fix.
//!
//! Bug: the low HP alarm continuously writes to sound channel 1 hardware
//! registers every frame, overriding all battle move SFX and suppressing
//! animations. This is the well-known "Red bar glitch."
//!
//! Fix: add a beep counter (`wLowHealthAlarmCounter`) that limits the alarm
//! to 4 beep cycles (~2 seconds). The counter is set when the alarm
//! activates and decremented each cycle; at 0 the alarm auto-disables.
//! It re-enables on the next HUD redraw, creating Gen II-like behavior.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Red_bar_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const BIT_7_HL: [u8; 2] = [0xCB, 0x7E]; // bit 7, [hl]
const SET_7_HL: [u8; 2] = [0xCB, 0xFE]; // set 7, [hl]
const JR_NZ: u8 = 0x20; // jr nz, e
const LD_A_N: u8 = 0x3E; // ld a, n
const LD_ADDR_A: u8 = 0xEA; // ld [nn], a
const LD_A_ADDR: u8 = 0xFA; // ld a, [nn]
const DEC_A: u8 = 0x3D; // dec a
const JR_Z: u8 = 0x28; // jr z, e
const RET: u8 = 0xC9; // ret

const W_LOW_HEALTH_ALARM_COUNTER: u16 = 0xCCF8;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn set_alarm_is_in_bank_0f() {
    assert_eq!(
        sym_bank("DrawPlayerHUDAndHPBar"),
        0x0F,
        "DrawPlayerHUDAndHPBar should be in bank $0F"
    );
}

#[test]
fn set_alarm_checks_bit_7_before_setting() {
    // .setLowHealthAlarm should check bit 7 (already enabled?) before
    // setting the counter, to avoid resetting mid-beep.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let addr = sym_addr("DrawPlayerHUDAndHPBar.setLowHealthAlarm");

    // ld hl, wLowHealthAlarm (3 bytes)
    // bit 7, [hl] (2 bytes)
    assert_eq!(rom(&mut h, addr + 3), BIT_7_HL[0]);
    assert_eq!(rom(&mut h, addr + 4), BIT_7_HL[1]);

    // jr nz, .alarmAlreadyOn
    assert_eq!(rom(&mut h, addr + 5), JR_NZ);
}

#[test]
fn set_alarm_initializes_counter_to_4() {
    // When alarm was off, set bit 7 then ld a, 4 / ld [wLowHealthAlarmCounter], a.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let addr = sym_addr("DrawPlayerHUDAndHPBar.setLowHealthAlarm");

    // After jr nz (2 bytes at +5,+6):
    // set 7, [hl] at +7, +8
    assert_eq!(rom(&mut h, addr + 7), SET_7_HL[0]);
    assert_eq!(rom(&mut h, addr + 8), SET_7_HL[1]);

    // ld a, 4 at +9, +10
    assert_eq!(rom(&mut h, addr + 9), LD_A_N);
    assert_eq!(
        rom(&mut h, addr + 10),
        4,
        "Counter should be initialized to 4"
    );

    // ld [wLowHealthAlarmCounter], a at +11, +12, +13
    assert_eq!(rom(&mut h, addr + 11), LD_ADDR_A);
    let lo = rom(&mut h, addr + 12);
    let hi = rom(&mut h, addr + 13);
    let target = u16::from_le_bytes([lo, hi]);
    assert_eq!(
        target, W_LOW_HEALTH_ALARM_COUNTER,
        "Should write to wLowHealthAlarmCounter (${:04X}), got ${:04X}",
        W_LOW_HEALTH_ALARM_COUNTER, target
    );
}

#[test]
fn alarm_already_on_returns() {
    // .alarmAlreadyOn should just be ret.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let addr = sym_addr("DrawPlayerHUDAndHPBar.alarmAlreadyOn");
    assert_eq!(rom(&mut h, addr), RET, "Expected ret at .alarmAlreadyOn");
}

#[test]
fn alarm_handler_decrements_counter() {
    // In Music_DoLowHealthAlarm, when timer == 0 (new beep cycle),
    // the counter is decremented before playing the tone.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("Music_DoLowHealthAlarm"));

    let alarm_addr = sym_addr("Music_DoLowHealthAlarm");

    // Search for the counter decrement pattern:
    // ld a, [wLowHealthAlarmCounter]  ; FA F8 CC
    // dec a                            ; 3D
    // ld [wLowHealthAlarmCounter], a  ; EA F8 CC
    // jr z, .disableAlarm             ; 28 xx
    let mut found = false;
    for offset in 0..40 {
        let a = alarm_addr + offset;
        if rom(&mut h, a) == LD_A_ADDR
            && rom(&mut h, a + 1) == (W_LOW_HEALTH_ALARM_COUNTER & 0xFF) as u8
            && rom(&mut h, a + 2) == (W_LOW_HEALTH_ALARM_COUNTER >> 8) as u8
            && rom(&mut h, a + 3) == DEC_A
            && rom(&mut h, a + 4) == LD_ADDR_A
            && rom(&mut h, a + 7) == JR_Z
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Counter decrement pattern (ld a / dec a / ld [nn] / jr z) not found in alarm handler"
    );
}

#[test]
fn counter_jr_z_targets_disable_alarm() {
    // The jr z after decrementing the counter should target .disableAlarm.
    let mut h = TestHarness::new();
    h.select_rom_bank(sym_bank("Music_DoLowHealthAlarm"));

    let alarm_addr = sym_addr("Music_DoLowHealthAlarm");
    let disable_addr = sym_addr("Music_DoLowHealthAlarm.disableAlarm");

    // Find the jr z
    for offset in 0..40 {
        let a = alarm_addr + offset;
        if rom(&mut h, a) == LD_A_ADDR
            && rom(&mut h, a + 1) == (W_LOW_HEALTH_ALARM_COUNTER & 0xFF) as u8
            && rom(&mut h, a + 3) == DEC_A
            && rom(&mut h, a + 7) == JR_Z
        {
            let jr_addr = a + 7;
            let jr_offset = rom(&mut h, jr_addr + 1) as i8;
            let target = (jr_addr + 2).wrapping_add(jr_offset as u16);
            assert_eq!(
                target, disable_addr,
                "jr z should target .disableAlarm (${:04X}), got ${:04X}",
                disable_addr, target
            );
            return;
        }
    }
    panic!("Counter decrement pattern not found");
}
