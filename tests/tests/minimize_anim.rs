//! ROM byte tests for the Link battle animation oversight fix (Minimize).
//!
//! Bug: When battle animations are disabled, PlayCurrentMoveAnimation skips
//! the Minimize animation. The MINIMIZED flag is still set by the effect
//! code, but the sprite is never visually shrunk to the diamond shape.
//! In link battles (or single-player with animations off), this creates an
//! inconsistency between the logical state and the visual appearance.
//!
//! Fix: After setting the MINIMIZED flag, check wOptions BIT_BATTLE_ANIMATION.
//! If animations were disabled, call AnimationMinimizeMon to apply the visual
//! effect. This mirrors the pattern used by Substitute and Transform.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Link_battle_animation_oversight>

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

const LD_A_NN: u8 = 0xFA; // ld a, [nn]
const BIT_7_A: [u8; 2] = [0xCB, 0x7F]; // bit 7, a (BIT_BATTLE_ANIMATION = bit 7)
const JR_Z: u8 = 0x28; // jr z, n
const LD_HL_NN: u8 = 0x21; // ld hl, nn
const CALL_NN: u8 = 0xCD; // call nn

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn stat_modifier_up_in_bank_0f() {
    assert_eq!(sym_bank("StatModifierUpEffect"), 0x0F);
}

#[test]
fn animation_check_between_flag_set_and_reshow() {
    // Between .notMinimize and .minimizeAnimPlayed, there should be:
    //   ld a, [wOptions]              ($FA lo hi)
    //   bit BIT_BATTLE_ANIMATION, a   ($CB $7F)
    //   jr z, .minimizeAnimPlayed     ($28 nn)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_min = sym_addr("UpdateStatDone.notMinimize");
    let anim_played = sym_addr("UpdateStatDone.minimizeAnimPlayed");
    let w_options = sym_addr("wOptions");

    let mut found_options_check = false;
    for addr in not_min..anim_played {
        if rom(&mut h, addr) == LD_A_NN && rom16(&mut h, addr + 1) == w_options {
            found_options_check = true;
            // bit 7, a should follow
            assert_eq!(
                rom(&mut h, addr + 3),
                BIT_7_A[0],
                "Expected CB prefix for bit instruction"
            );
            assert_eq!(
                rom(&mut h, addr + 4),
                BIT_7_A[1],
                "Expected bit 7, a (BIT_BATTLE_ANIMATION)"
            );
            // jr z should follow
            assert_eq!(
                rom(&mut h, addr + 5),
                JR_Z,
                "Expected jr z, .minimizeAnimPlayed"
            );
            break;
        }
    }
    assert!(
        found_options_check,
        "wOptions / BIT_BATTLE_ANIMATION check not found"
    );
}

#[test]
fn bankswitch_call_for_animation_minimize_mon() {
    // Between the animation check and .minimizeAnimPlayed, there should be a
    // call Bankswitch to AnimationMinimizeMon
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_min = sym_addr("UpdateStatDone.notMinimize");
    let anim_played = sym_addr("UpdateStatDone.minimizeAnimPlayed");
    let bankswitch = sym_addr("Bankswitch");

    let mut found = false;
    for addr in not_min..anim_played.saturating_sub(2) {
        if rom(&mut h, addr) == CALL_NN && rom16(&mut h, addr + 1) == bankswitch {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "call Bankswitch (AnimationMinimizeMon) not found"
    );
}

#[test]
fn ld_hl_animation_minimize_mon_present() {
    // ld hl, AnimationMinimizeMon should be present
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_min = sym_addr("UpdateStatDone.notMinimize");
    let anim_played = sym_addr("UpdateStatDone.minimizeAnimPlayed");
    let anim_minimize = sym_addr("AnimationMinimizeMon");

    let mut found = false;
    for addr in not_min..anim_played {
        if rom(&mut h, addr) == LD_HL_NN && rom16(&mut h, addr + 1) == anim_minimize {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "ld hl, AnimationMinimizeMon not found"
    );
}

#[test]
fn minimize_anim_played_label_ordering() {
    let not_min = sym_addr("UpdateStatDone.notMinimize");
    let anim_played = sym_addr("UpdateStatDone.minimizeAnimPlayed");
    let apply = sym_addr("UpdateStatDone.applyBadgeBoostsAndStatusPenalties");

    assert!(
        anim_played > not_min,
        ".minimizeAnimPlayed should be after .notMinimize"
    );
    assert!(
        apply > anim_played,
        ".applyBadgeBoostsAndStatusPenalties should be after .minimizeAnimPlayed"
    );
}
