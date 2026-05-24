//! ROM byte tests for the Pikachu friendship item effect fix.
//!
//! Bug: `ItemUseMedicine` calls `farcall_ModifyPikachuHappiness PIKAHAPPY_USEDITEM`
//! before checking whether the item actually has an effect.  Items that fail
//! (Potion on full HP, Antidote when not poisoned, Calcium when stat EXP is
//! maxed) still increase Pikachu's friendship without being consumed, allowing
//! infinite happiness grinding.
//!
//! Fix: Remove the premature happiness call and move it to the two success
//! paths: `.doneHealing` (healing items that worked) and `.gotStatName`
//! (vitamins that boosted stats).
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I#Friendship_item_effect>
//!   - <https://glitchcity.wiki/wiki/Walking_Pikachu_happiness_glitch>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// PIKAHAPPY_USEDITEM = 2, appears as ld d, $02 ($16 $02) in the farcall macro
const LD_D_IMM: u8 = 0x16;
const PIKAHAPPY_USEDITEM: u8 = 0x02;

// ─── Structural ─────────────────────────────────────────────────────

#[test]
fn item_use_medicine_in_bank_03() {
    assert_eq!(
        sym_bank("ItemUseMedicine"),
        0x03,
        "ItemUseMedicine should be in bank $03"
    );
}

// ─── THE FIX: no happiness before effect checks ─────────────────────

#[test]
fn no_happiness_call_before_effect_checks() {
    let mut h = banked_harness("ItemUseMedicine");
    let base = sym_addr("ItemUseMedicine");

    // Between ItemUseMedicine start and .doneHealing, there should be
    // NO `ld d, PIKAHAPPY_USEDITEM` ($16 $02) — the old premature call
    // should be removed.
    //
    // We scan for the specific ld d, $02 pattern. Since ld d, imm8 can
    // appear for other reasons, we look for the full farcall pattern:
    // ld d, $02 ($16 $02) followed within a few bytes by call Bankswitch
    let bankswitch = sym_addr("Bankswitch");
    let bs_lo = (bankswitch & 0xFF) as u8;
    let bs_hi = (bankswitch >> 8) as u8;

    let no_effect = sym_addr("ItemUseMedicine.healingItemNoEffect");

    // Search between base and healingItemNoEffect for the farcall pattern
    for addr in base..no_effect {
        if rom(&mut h, addr) == LD_D_IMM && rom(&mut h, addr + 1) == PIKAHAPPY_USEDITEM {
            // Check if this is part of a farcall (call Bankswitch within 8 bytes)
            for offset in 2..8 {
                if rom(&mut h, addr + offset) == 0xCD
                    && rom(&mut h, addr + offset + 1) == bs_lo
                    && rom(&mut h, addr + offset + 2) == bs_hi
                {
                    panic!(
                        "Found premature PIKAHAPPY_USEDITEM farcall at ${:04X}, \
                         before effect checks — should have been removed",
                        addr
                    );
                }
            }
        }
    }
    // If we get here, no premature happiness call found — fix verified
}

// ─── Happiness in .doneHealing success path ─────────────────────────

#[test]
fn happiness_call_in_done_healing() {
    let mut h = banked_harness("ItemUseMedicine.doneHealing");
    let done_healing = sym_addr("ItemUseMedicine.doneHealing");
    let skip_removing = done_healing + 40; // search within 40 bytes

    let bankswitch = sym_addr("Bankswitch");
    let bs_lo = (bankswitch & 0xFF) as u8;
    let bs_hi = (bankswitch >> 8) as u8;

    // Look for ld d, PIKAHAPPY_USEDITEM ($16 $02) + call Bankswitch
    let mut found = false;
    for addr in done_healing..skip_removing {
        if rom(&mut h, addr) == LD_D_IMM && rom(&mut h, addr + 1) == PIKAHAPPY_USEDITEM {
            for offset in 2..8 {
                if rom(&mut h, addr + offset) == 0xCD
                    && rom(&mut h, addr + offset + 1) == bs_lo
                    && rom(&mut h, addr + offset + 2) == bs_hi
                {
                    found = true;
                    break;
                }
            }
        }
        if found {
            break;
        }
    }
    assert!(
        found,
        "PIKAHAPPY_USEDITEM farcall should be present in .doneHealing success path"
    );
}

// ─── Happiness in .gotStatName vitamin success path ─────────────────

#[test]
fn happiness_call_in_got_stat_name() {
    let mut h = banked_harness("ItemUseMedicine.gotStatName");
    let got_stat = sym_addr("ItemUseMedicine.gotStatName");
    let vitamin_no_effect = sym_addr("ItemUseMedicine.vitaminNoEffect");

    let bankswitch = sym_addr("Bankswitch");
    let bs_lo = (bankswitch & 0xFF) as u8;
    let bs_hi = (bankswitch >> 8) as u8;

    let mut found = false;
    for addr in got_stat..vitamin_no_effect {
        if rom(&mut h, addr) == LD_D_IMM && rom(&mut h, addr + 1) == PIKAHAPPY_USEDITEM {
            for offset in 2..8 {
                if rom(&mut h, addr + offset) == 0xCD
                    && rom(&mut h, addr + offset + 1) == bs_lo
                    && rom(&mut h, addr + offset + 2) == bs_hi
                {
                    found = true;
                    break;
                }
            }
        }
        if found {
            break;
        }
    }
    assert!(
        found,
        "PIKAHAPPY_USEDITEM farcall should be present in .gotStatName vitamin success path"
    );
}

// ─── No happiness in .vitaminNoEffect failure path ──────────────────

#[test]
fn no_happiness_in_vitamin_no_effect() {
    let mut h = banked_harness("ItemUseMedicine.vitaminNoEffect");
    let no_effect = sym_addr("ItemUseMedicine.vitaminNoEffect");

    let bankswitch = sym_addr("Bankswitch");
    let bs_lo = (bankswitch & 0xFF) as u8;
    let bs_hi = (bankswitch >> 8) as u8;

    // .vitaminNoEffect is only a few instructions (pop hl, ld hl, call, jp)
    // Scan 15 bytes — should NOT contain a happiness farcall
    for addr in no_effect..no_effect + 15 {
        if rom(&mut h, addr) == LD_D_IMM && rom(&mut h, addr + 1) == PIKAHAPPY_USEDITEM {
            for offset in 2..8 {
                if addr + offset + 2 < no_effect + 20
                    && rom(&mut h, addr + offset) == 0xCD
                    && rom(&mut h, addr + offset + 1) == bs_lo
                    && rom(&mut h, addr + offset + 2) == bs_hi
                {
                    panic!(
                        "Found PIKAHAPPY_USEDITEM farcall in .vitaminNoEffect at ${:04X} — \
                         happiness should NOT be applied on failure path",
                        addr
                    );
                }
            }
        }
    }
}

// ─── No happiness in .healingItemNoEffect failure path ──────────────

#[test]
fn no_happiness_in_healing_no_effect() {
    let mut h = banked_harness("ItemUseMedicine.healingItemNoEffect");
    let no_effect = sym_addr("ItemUseMedicine.healingItemNoEffect");

    let bankswitch = sym_addr("Bankswitch");
    let bs_lo = (bankswitch & 0xFF) as u8;
    let bs_hi = (bankswitch >> 8) as u8;

    for addr in no_effect..no_effect + 10 {
        if rom(&mut h, addr) == LD_D_IMM && rom(&mut h, addr + 1) == PIKAHAPPY_USEDITEM {
            for offset in 2..8 {
                if addr + offset + 2 < no_effect + 15
                    && rom(&mut h, addr + offset) == 0xCD
                    && rom(&mut h, addr + offset + 1) == bs_lo
                    && rom(&mut h, addr + offset + 2) == bs_hi
                {
                    panic!(
                        "Found PIKAHAPPY_USEDITEM farcall in .healingItemNoEffect — \
                         happiness should NOT be applied on failure path"
                    );
                }
            }
        }
    }
}
