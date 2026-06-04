//! ROM byte tests for the escape sprite handling glitch fix.
//!
//! Bug: When using Escape Rope, Dig, or Teleport, the player sprite
//! briefly shows garbled "ABCD" tiles (DMG) or doesn't spin (SGB)
//! during the upward movement phase. `PlayerSpinWhileMovingUpOrDown`
//! is entered with `hl` pointing to `wPlayerSpinWhileMovingUpOrDown-
//! AnimFrameDelay` instead of `wFacingDirectionList`, so
//! `SpinPlayerSprite` reads the frame delay value (2 or 3) as a
//! sprite facing index.
//!
//! Fix: At the top of `PlayerSpinWhileMovingUpOrDown`, reload `hl`
//! with `wFacingDirectionList` before calling `SpinPlayerSprite`.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Escape_sprite_handling_glitch>
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("PlayerSpinWhileMovingUpOrDown"));
    h
}

// ─── Structural tests ──────────────────────────────────────────────

#[test]
fn spin_while_moving_starts_with_ld_hl_facing_list() {
    let mut h = rom_harness();
    let addr = sym_addr("PlayerSpinWhileMovingUpOrDown");
    // ld hl, wFacingDirectionList → $21 lo hi
    assert_eq!(rom(&mut h, addr), 0x21, "ld hl, nn opcode");
    assert_eq!(rom(&mut h, addr + 1), 0x48, "wFacingDirectionList low byte");
    assert_eq!(
        rom(&mut h, addr + 2),
        0xCD,
        "wFacingDirectionList high byte"
    );
}

#[test]
fn call_spin_player_sprite_follows_ld_hl() {
    let mut h = rom_harness();
    let addr = sym_addr("PlayerSpinWhileMovingUpOrDown");
    let target = sym_addr("SpinPlayerSprite");
    // call SpinPlayerSprite → $CD lo hi (at offset +3)
    assert_eq!(rom(&mut h, addr + 3), 0xCD, "call nn opcode");
    let call_lo = rom(&mut h, addr + 4);
    let call_hi = rom(&mut h, addr + 5);
    let call_target = u16::from(call_hi) << 8 | u16::from(call_lo);
    assert_eq!(
        call_target, target,
        "call target should be SpinPlayerSprite"
    );
}

#[test]
fn fix_adds_3_bytes_before_original_code() {
    let mut h = rom_harness();
    let addr = sym_addr("PlayerSpinWhileMovingUpOrDown");
    // Original first instruction (now at offset +6): ld a, [wPlayerSpinWhileMovingUpOrDownAnimDeltaY]
    // = $FA $3D $CD (ld a, [$CD3D])
    assert_eq!(rom(&mut h, addr + 6), 0xFA, "ld a, [nn] opcode at +6");
    assert_eq!(
        rom(&mut h, addr + 7),
        0x3D,
        "wPlayerSpinWhileMovingUpOrDownAnimDeltaY low byte"
    );
    assert_eq!(
        rom(&mut h, addr + 8),
        0xCD,
        "wPlayerSpinWhileMovingUpOrDownAnimDeltaY high byte"
    );
}

#[test]
fn player_spin_while_moving_is_in_bank_1c() {
    assert_eq!(
        sym_bank("PlayerSpinWhileMovingUpOrDown"),
        0x1C,
        "PlayerSpinWhileMovingUpOrDown should be in bank $1C"
    );
}

#[test]
fn spin_player_sprite_is_in_same_bank() {
    assert_eq!(
        sym_bank("SpinPlayerSprite"),
        sym_bank("PlayerSpinWhileMovingUpOrDown"),
        "SpinPlayerSprite and PlayerSpinWhileMovingUpOrDown should be in the same bank"
    );
}

#[test]
fn facing_direction_list_is_in_wram() {
    let addr = sym_addr("wFacingDirectionList");
    assert!(
        (0xC000..=0xDFFF).contains(&addr),
        "wFacingDirectionList ({:#06X}) should be in WRAM ($C000-$DFFF)",
        addr
    );
}
