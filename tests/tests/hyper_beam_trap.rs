//! ROM byte tests for the Hyper Beam auto-selection glitch fix.
//!
//! Bug: `TrappingEffect` calls `ClearHyperBeam` before the accuracy check.
//! If the trapping move misses, the target's `NEEDS_TO_RECHARGE` is already
//! cleared, so on the target's turn Hyper Beam fires again without recharging
//! (and can underflow PP).
//!
//! Fix: Remove `ClearHyperBeam` from `TrappingEffect`.  Instead, clear the
//! recharge flag in `.HeldInPlaceCheck` (player) and `.checkIfTrapped` (enemy)
//! — only when the target is genuinely trapped (trapping move hit).
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Hyper_Beam_automatic_selection_glitch>
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Struggle_bypassing>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// ─── TrappingEffect: no ClearHyperBeam call ─────────────────────────

#[test]
fn trapping_effect_no_clear_hyper_beam() {
    let mut h = banked_harness("TrappingEffect");
    let trap_effect = sym_addr("TrappingEffect.trappingEffect");
    let set_counter = sym_addr("TrappingEffect.setTrappingCounter");

    let clear_hb = sym_addr("ClearHyperBeam");
    let chb_lo = (clear_hb & 0xFF) as u8;
    let chb_hi = (clear_hb >> 8) as u8;

    // Verify no `call ClearHyperBeam` ($CD lo hi) between .trappingEffect and .setTrappingCounter
    for addr in trap_effect..set_counter {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == chb_lo
            && rom(&mut h, addr + 2) == chb_hi
        {
            panic!(
                "Found call ClearHyperBeam at ${:04X} in TrappingEffect — \
                 should have been removed (Hyper Beam auto-selection fix)",
                addr
            );
        }
    }
}

// ─── Player side: .HeldInPlaceCheck clears recharge ─────────────────

#[test]
fn held_in_place_clears_player_recharge() {
    let mut h = banked_harness("CheckPlayerStatusConditions.HeldInPlaceCheck");
    let base = sym_addr("CheckPlayerStatusConditions.HeldInPlaceCheck");

    // After the USING_TRAPPING_MOVE check, there should be:
    //   ld hl, wPlayerBattleStatus2   ($21 lo hi)
    //   res NEEDS_TO_RECHARGE, [hl]   ($CB $AE — res 5, [hl])
    let player_bs2 = sym_addr("wPlayerBattleStatus2");
    let bs2_lo = (player_bs2 & 0xFF) as u8;
    let bs2_hi = (player_bs2 >> 8) as u8;

    let mut found = false;
    for addr in base..base + 20 {
        if rom(&mut h, addr) == 0x21
            && rom(&mut h, addr + 1) == bs2_lo
            && rom(&mut h, addr + 2) == bs2_hi
        {
            // Check for res 5, [hl] ($CB $AE) following
            if rom(&mut h, addr + 3) == 0xCB && rom(&mut h, addr + 4) == 0xAE {
                found = true;
                break;
            }
        }
    }
    assert!(
        found,
        "ld hl, wPlayerBattleStatus2 / res NEEDS_TO_RECHARGE, [hl] \
         should be in .HeldInPlaceCheck"
    );
}

// ─── Enemy side: .checkIfTrapped clears recharge ────────────────────

#[test]
fn check_if_trapped_clears_enemy_recharge() {
    let mut h = banked_harness("CheckEnemyStatusConditions.checkIfTrapped");
    let base = sym_addr("CheckEnemyStatusConditions.checkIfTrapped");

    // ld hl, wEnemyBattleStatus2 ($21 lo hi) / res 5, [hl] ($CB $AE)
    let enemy_bs2 = sym_addr("wEnemyBattleStatus2");
    let bs2_lo = (enemy_bs2 & 0xFF) as u8;
    let bs2_hi = (enemy_bs2 >> 8) as u8;

    let mut found = false;
    for addr in base..base + 20 {
        if rom(&mut h, addr) == 0x21
            && rom(&mut h, addr + 1) == bs2_lo
            && rom(&mut h, addr + 2) == bs2_hi
        {
            if rom(&mut h, addr + 3) == 0xCB && rom(&mut h, addr + 4) == 0xAE {
                found = true;
                break;
            }
        }
    }
    assert!(
        found,
        "ld hl, wEnemyBattleStatus2 / res NEEDS_TO_RECHARGE, [hl] \
         should be in .checkIfTrapped"
    );
}

// ─── TrappingEffect still sets USING_TRAPPING_MOVE ──────────────────

#[test]
fn trapping_effect_still_sets_flag() {
    let mut h = banked_harness("TrappingEffect.trappingEffect");
    let trap_effect = sym_addr("TrappingEffect.trappingEffect");

    // set USING_TRAPPING_MOVE, [hl] = $CB $EE (set 5, [hl])
    let mut found = false;
    for addr in trap_effect..trap_effect + 15 {
        if rom(&mut h, addr) == 0xCB && rom(&mut h, addr + 1) == 0xEE {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "set USING_TRAPPING_MOVE, [hl] ($CB $EE) should still be in TrappingEffect"
    );
}

// ─── Structural ─────────────────────────────────────────────────────

#[test]
fn all_in_bank_0f() {
    assert_eq!(sym_bank("TrappingEffect"), 0x0F);
    assert_eq!(
        sym_bank("CheckPlayerStatusConditions.HeldInPlaceCheck"),
        0x0F
    );
    assert_eq!(sym_bank("CheckEnemyStatusConditions.checkIfTrapped"), 0x0F);
}
