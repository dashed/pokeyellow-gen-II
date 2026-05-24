//! ROM byte tests for the slide animation tearing fix.
//!
//! Bug: `SlideDownFaintedMonPic` and `SlideTrainerPicOffScreen` modify the
//! tilemap in RAM while `hAutoBGTransferEnabled` is non-zero. During the
//! `DelayFrames` call that paces each animation step, the VBlank handler
//! transfers the partially-modified tilemap to VRAM, causing visible screen
//! tearing when trainer/Pokémon graphics slide on or off screen.
//!
//! Fix: Disable `hAutoBGTransferEnabled` before the tilemap copy loops and
//! re-enable it after completion but before `DelayFrames`, so VBlank only
//! transfers complete frames. +7 bytes per function, +14 bytes total in
//! bank $0F.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("SlideDownFaintedMonPic"));
    h
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn both_slide_functions_in_bank_0f() {
    assert_eq!(sym_bank("SlideDownFaintedMonPic"), 0x0F);
    assert_eq!(sym_bank("SlideTrainerPicOffScreen"), 0x0F);
}

// ─── SlideDownFaintedMonPic tests ────────────────────────────────────

#[test]
fn slide_down_disables_bg_transfer_before_row_loop() {
    let mut h = banked_harness();
    let row_loop = sym_addr("SlideDownFaintedMonPic.rowLoop");
    // Just before .rowLoop: ldh [hAutoBGTransferEnabled], a → $E0 $BA (2 bytes)
    // And before that: xor a → $AF (1 byte)
    // So at rowLoop - 3: $AF, rowLoop - 2: $E0, rowLoop - 1: $BA
    assert_eq!(rom(&mut h, row_loop - 3), 0xAF, "xor a before .rowLoop");
    assert_eq!(
        rom(&mut h, row_loop - 2),
        0xE0,
        "ldh [n], a opcode before .rowLoop"
    );
    assert_eq!(
        rom(&mut h, row_loop - 1),
        0xBA,
        "hAutoBGTransferEnabled offset ($BA)"
    );
}

#[test]
fn slide_down_reenables_bg_transfer_after_place_string() {
    let mut h = banked_harness();
    let slide_step = sym_addr("SlideDownFaintedMonPic.slideStepLoop");
    let end = slide_step + 80; // generous search range
                               // Find `call PlaceString` then verify `ld a, 1 / ldh [hAutoBGTransferEnabled], a` follows
    let place_string = sym_addr("PlaceString");
    let ps_lo = (place_string & 0xFF) as u8;
    let ps_hi = (place_string >> 8) as u8;
    for addr in slide_step..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == ps_lo
            && rom(&mut h, addr + 2) == ps_hi
        {
            // After call PlaceString (3 bytes): ld a, 1 / ldh [hAutoBGTransferEnabled], a
            assert_eq!(
                rom(&mut h, addr + 3),
                0x3E,
                "ld a, n opcode after PlaceString"
            );
            assert_eq!(rom(&mut h, addr + 4), 0x01, "ld a, 1");
            assert_eq!(rom(&mut h, addr + 5), 0xE0, "ldh [n], a opcode");
            assert_eq!(rom(&mut h, addr + 6), 0xBA, "hAutoBGTransferEnabled offset");
            return;
        }
    }
    panic!("call PlaceString not found in SlideDownFaintedMonPic");
}

#[test]
fn slide_down_reenable_before_delay_frames() {
    let mut h = banked_harness();
    let slide_step = sym_addr("SlideDownFaintedMonPic.slideStepLoop");
    let end = slide_step + 80;
    let delay_frames = sym_addr("DelayFrames");
    let df_lo = (delay_frames & 0xFF) as u8;
    let df_hi = (delay_frames >> 8) as u8;
    // Find the re-enable ($3E $01 $E0 $BA) and the call DelayFrames
    let mut reenable_pos = None;
    let mut delay_pos = None;
    for addr in slide_step..end {
        if rom(&mut h, addr) == 0x3E
            && rom(&mut h, addr + 1) == 0x01
            && rom(&mut h, addr + 2) == 0xE0
            && rom(&mut h, addr + 3) == 0xBA
        {
            reenable_pos = Some(addr);
        }
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == df_lo
            && rom(&mut h, addr + 2) == df_hi
            && delay_pos.is_none()
            // skip the first call (which might be CopyData or PlaceString)
            && reenable_pos.is_some()
        {
            delay_pos = Some(addr);
        }
    }
    assert!(reenable_pos.is_some(), "re-enable not found");
    assert!(
        delay_pos.is_some(),
        "call DelayFrames not found after re-enable"
    );
    assert!(
        reenable_pos.unwrap() < delay_pos.unwrap(),
        "re-enable ({:#06X}) must come before DelayFrames ({:#06X})",
        reenable_pos.unwrap(),
        delay_pos.unwrap()
    );
}

// ─── SlideTrainerPicOffScreen tests ──────────────────────────────────

#[test]
fn slide_trainer_disables_bg_transfer_before_row_loop() {
    let mut h = banked_harness();
    let row_loop = sym_addr("SlideTrainerPicOffScreen.rowLoop");
    // Same pattern: xor a / ldh [hAutoBGTransferEnabled], a just before .rowLoop
    assert_eq!(
        rom(&mut h, row_loop - 3),
        0xAF,
        "xor a before trainer .rowLoop"
    );
    assert_eq!(
        rom(&mut h, row_loop - 2),
        0xE0,
        "ldh [n], a opcode before trainer .rowLoop"
    );
    assert_eq!(
        rom(&mut h, row_loop - 1),
        0xBA,
        "hAutoBGTransferEnabled offset"
    );
}

#[test]
fn slide_trainer_reenables_bg_transfer_after_rows() {
    let mut h = banked_harness();
    let row_loop = sym_addr("SlideTrainerPicOffScreen.rowLoop");
    let end = row_loop + 50;
    // After the row loop ends (jr nz, .rowLoop), find ld a, 1 / ldh [hAutoBGTransferEnabled], a
    // The row loop's jr nz is the last branch before the re-enable
    let mut found = false;
    for addr in row_loop..end {
        if rom(&mut h, addr) == 0x3E
            && rom(&mut h, addr + 1) == 0x01
            && rom(&mut h, addr + 2) == 0xE0
            && rom(&mut h, addr + 3) == 0xBA
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld a, 1 / ldh [hAutoBGTransferEnabled], a not found after trainer row loop"
    );
}

#[test]
fn slide_trainer_reenable_before_delay_frames() {
    let mut h = banked_harness();
    let slide_step = sym_addr("SlideTrainerPicOffScreen.slideStepLoop");
    let end = slide_step + 60;
    let delay_frames = sym_addr("DelayFrames");
    let df_lo = (delay_frames & 0xFF) as u8;
    let df_hi = (delay_frames >> 8) as u8;
    let mut reenable_pos = None;
    let mut delay_pos = None;
    for addr in slide_step..end {
        if rom(&mut h, addr) == 0x3E
            && rom(&mut h, addr + 1) == 0x01
            && rom(&mut h, addr + 2) == 0xE0
            && rom(&mut h, addr + 3) == 0xBA
        {
            reenable_pos = Some(addr);
        }
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == df_lo
            && rom(&mut h, addr + 2) == df_hi
        {
            delay_pos = Some(addr);
        }
    }
    assert!(
        reenable_pos.is_some(),
        "re-enable not found in trainer slide"
    );
    assert!(
        delay_pos.is_some(),
        "call DelayFrames not found in trainer slide"
    );
    assert!(
        reenable_pos.unwrap() < delay_pos.unwrap(),
        "re-enable ({:#06X}) must come before DelayFrames ({:#06X})",
        reenable_pos.unwrap(),
        delay_pos.unwrap()
    );
}

// ─── Negative test ───────────────────────────────────────────────────

#[test]
fn no_delay_frames_while_bg_transfer_disabled() {
    // Verify that in NEITHER function does DelayFrames get called
    // between the disable (xor a / ldh) and the re-enable (ld a, 1 / ldh).
    let mut h = banked_harness();
    let delay_frames = sym_addr("DelayFrames");
    let df_lo = (delay_frames & 0xFF) as u8;
    let df_hi = (delay_frames >> 8) as u8;
    for (name, base, len) in [
        (
            "SlideDownFaintedMonPic",
            sym_addr("SlideDownFaintedMonPic.slideStepLoop"),
            80u16,
        ),
        (
            "SlideTrainerPicOffScreen",
            sym_addr("SlideTrainerPicOffScreen.slideStepLoop"),
            60u16,
        ),
    ] {
        let mut disable_pos = None;
        let mut reenable_pos = None;
        for addr in base..base + len {
            if rom(&mut h, addr) == 0xAF
                && rom(&mut h, addr + 1) == 0xE0
                && rom(&mut h, addr + 2) == 0xBA
            {
                disable_pos = Some(addr);
            }
            if rom(&mut h, addr) == 0x3E
                && rom(&mut h, addr + 1) == 0x01
                && rom(&mut h, addr + 2) == 0xE0
                && rom(&mut h, addr + 3) == 0xBA
            {
                reenable_pos = Some(addr);
            }
        }
        let d = disable_pos.unwrap_or_else(|| panic!("{name}: disable not found"));
        let r = reenable_pos.unwrap_or_else(|| panic!("{name}: re-enable not found"));
        for addr in d..r {
            if rom(&mut h, addr) == 0xCD
                && rom(&mut h, addr + 1) == df_lo
                && rom(&mut h, addr + 2) == df_hi
            {
                panic!(
                    "{name}: call DelayFrames at {:#06X} between disable ({:#06X}) and re-enable ({:#06X})",
                    addr, d, r
                );
            }
        }
    }
}
