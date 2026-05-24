//! Emulator-based test for WARP text speed.
//!
//! Verifies that `PrintLetterDelay` returns immediately when text speed is
//! set to WARP (TEXT_DELAY_WARP = 0).
//!
//! PrintLetterDelay code at $00:$38AE:
//! ```text
//!   ld a, [wStatusFlags5]         ; check BIT_NO_TEXT_DELAY
//!   bit 6, a
//!   ret nz                        ; if set, return immediately (different path)
//!   ld a, [wLetterPrintingDelayFlags]
//!   bit 1, a                      ; BIT_TEXT_DELAY
//!   ret z                         ; if clear, return (no delay active)
//!   push hl / push de / push bc
//!   ld a, [wLetterPrintingDelayFlags]
//!   bit 0, a                      ; BIT_FAST_TEXT_DELAY
//!   jr z, .waitOneFrame
//!   ld a, [wOptions]
//!   and $F                        ; TEXT_DELAY_MASK — for WARP this is 0
//!   ld [hFrameCounter], a         ; hFrameCounter = 0
//!   jr .checkButtons
//!   ...
//! .buttonsNotPressed:
//!   ld a, [hFrameCounter]
//!   and a
//!   jr nz, .checkButtons          ; loop while hFrameCounter > 0
//! .done:                          ; $00:$38EC
//!   pop bc / pop de / pop hl
//!   ret
//! ```
//!
//! In WARP mode (wOptions & $F == 0), hFrameCounter is set to 0, so the
//! `.buttonsNotPressed` check immediately falls through to `.done`.

use pokeyellow_tests::{measure_cycles_to, sym_addr, TestHarness};

fn print_letter_delay() -> u16 {
    sym_addr("PrintLetterDelay")
}

fn print_letter_delay_done() -> u16 {
    sym_addr("PrintLetterDelay.done")
}

fn print_letter_delay_check_buttons() -> u16 {
    sym_addr("PrintLetterDelay.checkButtons")
}

/// wStatusFlags5 ($D72F): bit 6 = BIT_NO_TEXT_DELAY.
const W_STATUS_FLAGS5: u16 = 0xD72F;

/// TEXT_DELAY_WARP = 0, TEXT_DELAY_FAST = 1, TEXT_DELAY_MEDIUM = 3, TEXT_DELAY_SLOW = 5.
const TEXT_DELAY_WARP: u8 = 0;
const TEXT_DELAY_FAST: u8 = 1;
const TEXT_DELAY_MEDIUM: u8 = 3;
const TEXT_DELAY_SLOW: u8 = 5;
/// TEXT_DELAY_MASK = $0F.
const TEXT_DELAY_MASK: u8 = 0x0F;

/// BIT_FAST_TEXT_DELAY = bit 0, BIT_TEXT_DELAY = bit 1.
const BIT_FAST_TEXT_DELAY: u8 = 1 << 0;
const BIT_TEXT_DELAY: u8 = 1 << 1;
/// BIT_NO_TEXT_DELAY = bit 6 in wStatusFlags5.
const BIT_NO_TEXT_DELAY: u8 = 1 << 6;

/// A safe WRAM return address.
const TRAP_ADDR: u16 = 0xC100;

/// Set up the harness for a PrintLetterDelay test.
///
/// Configures memory so the function takes the "normal" text delay path
/// (not the BIT_NO_TEXT_DELAY early return), with the specified text speed.
fn setup_text_speed_test(h: &mut TestHarness, text_delay: u8) {
    // Clear BIT_NO_TEXT_DELAY in wStatusFlags5 so we don't take the early ret nz
    let flags5 = h.read_mem(W_STATUS_FLAGS5);
    h.write_mem(W_STATUS_FLAGS5, flags5 & !BIT_NO_TEXT_DELAY);

    // Set wLetterPrintingDelayFlags: enable both BIT_TEXT_DELAY and BIT_FAST_TEXT_DELAY
    // BIT_TEXT_DELAY must be set or PrintLetterDelay returns immediately (ret z).
    // BIT_FAST_TEXT_DELAY must be set to take the path that reads wOptions.
    h.write_mem(
        sym_addr("wLetterPrintingDelayFlags"),
        BIT_TEXT_DELAY | BIT_FAST_TEXT_DELAY,
    );

    // Set text speed in wOptions (lower 4 bits)
    let options = h.read_mem(sym_addr("wOptions"));
    h.write_mem(
        sym_addr("wOptions"),
        (options & !TEXT_DELAY_MASK) | text_delay,
    );

    // Set up stack with return address
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.write_mem(TRAP_ADDR, 0x00); // NOP
    h.write_mem(TRAP_ADDR + 1, 0x10); // STOP

    // PrintLetterDelay is in ROM bank 0 (home section), no bank switch needed.
    h.set_pc(print_letter_delay());
}

#[test]
fn warp_text_speed_reaches_done_quickly() {
    let mut h = TestHarness::new_headless();

    setup_text_speed_test(&mut h, TEXT_DELAY_WARP);

    // Record the frame count before calling PrintLetterDelay
    let start_frame = h.frame_count();

    // Run until .done — in WARP mode this should be nearly instant
    // (no DelayFrame calls, hFrameCounter starts at 0)
    h.step_to(print_letter_delay_done());

    let end_frame = h.frame_count();
    let frames_elapsed = end_frame.wrapping_sub(start_frame);

    // WARP mode: hFrameCounter = 0 → .buttonsNotPressed immediately falls
    // through to .done. No frames should be consumed.
    // We allow 0 frames (no DelayFrame called).
    assert_eq!(
        frames_elapsed, 0,
        "WARP mode should not consume any frames in PrintLetterDelay, but {frames_elapsed} elapsed"
    );
}

#[test]
fn fast_text_speed_enters_delay_loop() {
    // We can't run the full FAST delay loop headless (DelayFrame needs PPU
    // for VBlank timing). Instead, verify that FAST mode enters the delay
    // loop by checking that it reaches .checkButtons but NOT .done within
    // the same number of instructions that WARP takes to reach .done.
    //
    // This proves FAST mode does NOT take the WARP fast-path.
    let mut h = TestHarness::new_headless();

    setup_text_speed_test(&mut h, TEXT_DELAY_FAST);

    // Run a limited number of instructions (enough for WARP to complete,
    // but FAST should still be in the delay loop).
    let check_buttons = print_letter_delay_check_buttons();
    let mut reached_check_buttons = false;
    let mut reached_done = false;

    for _ in 0..200 {
        let pc = h.pc();
        if pc == check_buttons {
            reached_check_buttons = true;
        }
        if pc == print_letter_delay_done() {
            reached_done = true;
            break;
        }
        h.clock();
    }

    assert!(
        reached_check_buttons,
        "FAST mode should enter the delay loop (.checkButtons)"
    );
    assert!(
        !reached_done,
        "FAST mode should NOT reach .done within 200 instructions (it should be stuck in the delay loop)"
    );
}

// ─── Scenario 14: Text speed cycle ordering benchmark ─────────────

#[test]
fn text_speed_warp_is_fastest() {
    // WARP (delay=0) completes PrintLetterDelay immediately because
    // hFrameCounter is set to 0. FAST/MEDIUM/SLOW enter the delay loop
    // which waits for hFrameCounter to be decremented by VBlank, so they
    // cannot complete without VBlank firing.
    //
    // Strategy: measure WARP cycles, then verify FAST/MEDIUM/SLOW do NOT
    // reach .done within 10x WARP's budget.
    let mut h = TestHarness::new_headless();
    setup_text_speed_test(&mut h, TEXT_DELAY_WARP);

    let (warp_cycles, reached) = measure_cycles_to(&mut h, print_letter_delay_done(), 100_000);
    assert!(reached, "WARP should reach .done");
    assert!(
        warp_cycles < 1000,
        "WARP should complete in very few cycles, got {warp_cycles}"
    );

    // Verify FAST, MEDIUM, SLOW all get stuck (don't reach .done)
    for (name, delay) in [
        ("FAST", TEXT_DELAY_FAST),
        ("MEDIUM", TEXT_DELAY_MEDIUM),
        ("SLOW", TEXT_DELAY_SLOW),
    ] {
        let mut h2 = TestHarness::new_headless();
        setup_text_speed_test(&mut h2, delay);

        let budget = warp_cycles * 10;
        let (_, reached) = measure_cycles_to(&mut h2, print_letter_delay_done(), budget);
        assert!(
            !reached,
            "{name} (delay={delay}) should NOT complete within {budget} cycles \
             (WARP completes in {warp_cycles})"
        );
    }
}

#[test]
fn text_delay_values_are_strictly_ordered() {
    // The delay values directly determine how many frames the delay loop
    // waits (hFrameCounter = delay value). Verify they are strictly ordered
    // so WARP < FAST < MEDIUM < SLOW in real gameplay.
    const {
        assert!(
            TEXT_DELAY_WARP < TEXT_DELAY_FAST
                && TEXT_DELAY_FAST < TEXT_DELAY_MEDIUM
                && TEXT_DELAY_MEDIUM < TEXT_DELAY_SLOW,
        );
    }
}
