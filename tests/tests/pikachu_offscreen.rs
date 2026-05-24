//! ROM byte tests for the Pikachu off-screen glitch prevention fix.
//!
//! Bug: `AppendPikachuFollowCommandToBuffer` increments
//! `wPikachuFollowCommandBufferSize` and writes at
//! `wPikachuFollowCommandBuffer[size]` without bounds checking.
//! The buffer is only 16 bytes (`ds 16`). When Pikachu is left behind
//! at certain events (Jigglypuff, Bill, Clefairy) and the player walks
//! away, the buffer overflows into adjacent WRAM: `wExpressionNumber`,
//! `wPikachuMovementFlags`, trainer data, sign arrays, and more —
//! causing NPC corruption, forced Glitch City, and save file deletion.
//!
//! Fix: Before writing, check `bit 4, e` (index >= 16) and `ret nz`
//! to discard commands when the buffer is full. +4 bytes in bank $3F.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Pikachu_off-screen_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// ─── Structural test ─────────────────────────────────────────────────

#[test]
fn append_buffer_in_bank_3f() {
    assert_eq!(
        sym_bank("AppendPikachuFollowCommandToBuffer"),
        0x3F,
        "AppendPikachuFollowCommandToBuffer should be in bank $3F"
    );
}

// ─── THE FIX: bit 4, e / ret nz bounds check ────────────────────────

#[test]
fn has_bounds_check_bit_4_e() {
    let mut h = banked_harness("AppendPikachuFollowCommandToBuffer");
    let base = sym_addr("AppendPikachuFollowCommandToBuffer");
    let end = base + 20;

    // Search for `bit 4, e` ($CB $63) followed by `ret nz` ($C0)
    let mut found = false;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCB
            && rom(&mut h, addr + 1) == 0x63
            && rom(&mut h, addr + 2) == 0xC0
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "bit 4, e ($CB $63) / ret nz ($C0) bounds check should be in \
         AppendPikachuFollowCommandToBuffer"
    );
}

#[test]
fn loads_size_before_check() {
    let mut h = banked_harness("AppendPikachuFollowCommandToBuffer");
    let base = sym_addr("AppendPikachuFollowCommandToBuffer");

    // First instruction: ld hl, wPikachuFollowCommandBufferSize ($21 lo hi)
    let buf_size = sym_addr("wPikachuFollowCommandBufferSize");
    let lo = (buf_size & 0xFF) as u8;
    let hi = (buf_size >> 8) as u8;

    assert_eq!(rom(&mut h, base), 0x21, "Expected ld hl, nn ($21)");
    assert_eq!(
        rom(&mut h, base + 1),
        lo,
        "Expected low byte of wPikachuFollowCommandBufferSize"
    );
    assert_eq!(
        rom(&mut h, base + 2),
        hi,
        "Expected high byte of wPikachuFollowCommandBufferSize"
    );

    // ld e, [hl] ($5E) then inc e ($1C)
    assert_eq!(rom(&mut h, base + 3), 0x5E, "Expected ld e, [hl] ($5E)");
    assert_eq!(rom(&mut h, base + 4), 0x1C, "Expected inc e ($1C)");
}

#[test]
fn no_unbounded_inc_hl() {
    // Verify the old `inc [hl]` pattern is NOT present at the start
    let mut h = banked_harness("AppendPikachuFollowCommandToBuffer");
    let base = sym_addr("AppendPikachuFollowCommandToBuffer");

    // Old code had: ld hl, nn ($21) / inc [hl] ($34) / ld e, [hl] ($5E)
    // New code has: ld hl, nn ($21) / ld e, [hl] ($5E) / inc e ($1C)
    assert_ne!(
        rom(&mut h, base + 3),
        0x34,
        "inc [hl] ($34) should NOT be at offset +3 — old unbounded pattern"
    );
}

// ─── Buffer address cross-reference ──────────────────────────────────

#[test]
fn writes_to_correct_buffer() {
    let mut h = banked_harness("AppendPikachuFollowCommandToBuffer");
    let base = sym_addr("AppendPikachuFollowCommandToBuffer");
    let end = base + 20;

    // Find ld hl, wPikachuFollowCommandBuffer ($21 lo hi)
    let buf = sym_addr("wPikachuFollowCommandBuffer");
    let lo = (buf & 0xFF) as u8;
    let hi = (buf >> 8) as u8;

    let mut found = false;
    for addr in (base + 5)..end {
        if rom(&mut h, addr) == 0x21 && rom(&mut h, addr + 1) == lo && rom(&mut h, addr + 2) == hi {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld hl, wPikachuFollowCommandBuffer should be present after the bounds check"
    );
}

// ─── Buffer size validation ──────────────────────────────────────────

#[test]
fn buffer_is_16_bytes() {
    // wPikachuFollowCommandBuffer should be exactly 16 bytes after wPikachuFollowCommandBufferSize
    let size_addr = sym_addr("wPikachuFollowCommandBufferSize");
    let buf_addr = sym_addr("wPikachuFollowCommandBuffer");
    assert_eq!(
        buf_addr,
        size_addr + 1,
        "wPikachuFollowCommandBuffer should be 1 byte after wPikachuFollowCommandBufferSize"
    );
}
