//! ROM byte tests for the lucky slot machine off-by-one fix.
//!
//! Bug: `GameCornerSelectLuckySlotMachine` generates a random byte and checks
//! `cp $7` / `jr nc, .not_max` / `ld a, $8`. Values 0–6 are caught and
//! replaced with 8 before the three right-shifts (`srl a` ×3) that produce
//! the slot index. However, value 7 slips through: 7 >> 3 = 0, an invalid
//! slot machine index (they are 1-indexed), producing the nonexistent
//! "slot machine −1" (255 when treated as unsigned).
//!
//! Fix: Change `cp $7` to `cp $8` so that value 7 is also caught.
//! One-byte change in bank $12.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("GameCornerSelectLuckySlotMachine"));
    h
}

// Z80/SM83 opcodes
const CP_IMM: u8 = 0xFE; // cp n
const JR_NC: u8 = 0x30; // jr nc, e
const LD_A_IMM: u8 = 0x3E; // ld a, n
const SRL_A: [u8; 2] = [0xCB, 0x3F]; // srl a (CB-prefixed)

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn function_in_bank_12() {
    assert_eq!(sym_bank("GameCornerSelectLuckySlotMachine"), 0x12);
}

#[test]
fn function_in_banked_range() {
    let addr = sym_addr("GameCornerSelectLuckySlotMachine");
    assert!(
        (0x4000..=0x7FFF).contains(&addr),
        "expected banked ROM range, got {:#06X}",
        addr
    );
}

// ─── The fix: cp $8 instead of cp $7 ────────────────────────────────

#[test]
fn cp_8_present() {
    // The comparison immediate should be $08 (the fix), not $07.
    let mut h = banked_harness();
    let start = sym_addr("GameCornerSelectLuckySlotMachine");
    let not_max = sym_addr("GameCornerSelectLuckySlotMachine.not_max");
    let mut found = false;
    for addr in start..not_max {
        if rom(&mut h, addr) == CP_IMM && rom(&mut h, addr + 1) == 0x08 {
            found = true;
            break;
        }
    }
    assert!(found, "cp $8 (FE 08) not found before .not_max");
}

#[test]
fn jr_nc_follows_cp_8() {
    // After `cp $8`, the next instruction should be `jr nc` ($30).
    let mut h = banked_harness();
    let start = sym_addr("GameCornerSelectLuckySlotMachine");
    let not_max = sym_addr("GameCornerSelectLuckySlotMachine.not_max");
    for addr in start..not_max {
        if rom(&mut h, addr) == CP_IMM && rom(&mut h, addr + 1) == 0x08 {
            assert_eq!(
                rom(&mut h, addr + 2),
                JR_NC,
                "jr nc ($30) expected after cp $8 at {:#06X}",
                addr
            );
            return;
        }
    }
    panic!("cp $8 not found");
}

#[test]
fn ld_a_8_follows_jr_nc() {
    // After `jr nc, .not_max`, the fallthrough should be `ld a, $8` ($3E $08).
    let mut h = banked_harness();
    let start = sym_addr("GameCornerSelectLuckySlotMachine");
    let not_max = sym_addr("GameCornerSelectLuckySlotMachine.not_max");
    for addr in start..not_max {
        if rom(&mut h, addr) == CP_IMM && rom(&mut h, addr + 1) == 0x08 {
            // cp $8 (2 bytes) + jr nc, e (2 bytes) = offset +4
            let ld_addr = addr + 4;
            assert_eq!(
                rom(&mut h, ld_addr),
                LD_A_IMM,
                "ld a, n ($3E) expected at {:#06X}",
                ld_addr
            );
            assert_eq!(
                rom(&mut h, ld_addr + 1),
                0x08,
                "ld a, $8 immediate expected at {:#06X}",
                ld_addr + 1
            );
            return;
        }
    }
    panic!("cp $8 not found");
}

// ─── Context: three srl a shifts ────────────────────────────────────

#[test]
fn three_srl_a_at_not_max() {
    // .not_max should have three consecutive `srl a` instructions (CB 3F).
    let mut h = banked_harness();
    let not_max = sym_addr("GameCornerSelectLuckySlotMachine.not_max");
    for i in 0..3u16 {
        let addr = not_max + i * 2;
        assert_eq!(
            rom(&mut h, addr),
            SRL_A[0],
            "srl a prefix ($CB) expected at .not_max + {}, got {:#04X}",
            i * 2,
            rom(&mut h, addr)
        );
        assert_eq!(
            rom(&mut h, addr + 1),
            SRL_A[1],
            "srl a opcode ($3F) expected at .not_max + {}, got {:#04X}",
            i * 2 + 1,
            rom(&mut h, addr + 1)
        );
    }
}

#[test]
fn result_stored_to_wram() {
    // After the three srl a, expect `ld [nn], a` ($EA lo hi) storing into
    // wLuckySlotHiddenEventIndex.
    let mut h = banked_harness();
    let not_max = sym_addr("GameCornerSelectLuckySlotMachine.not_max");
    // Three srl a = 6 bytes, so ld [nn], a starts at not_max + 6.
    let store_addr = not_max + 6;
    assert_eq!(
        rom(&mut h, store_addr),
        0xEA,
        "ld [nn], a ($EA) expected after three srl a"
    );
}

// ─── Regression: no old cp $7 ───────────────────────────────────────

#[test]
fn no_old_cp_7_in_function() {
    // The old buggy `cp $7` (FE 07) should not appear in the function.
    let mut h = banked_harness();
    let start = sym_addr("GameCornerSelectLuckySlotMachine");
    let end = sym_addr("GameCornerSetRocketHideoutDoorTile");
    for addr in start..end {
        if rom(&mut h, addr) == CP_IMM && rom(&mut h, addr + 1) == 0x07 {
            panic!(
                "found old buggy cp $7 (FE 07) at {:#06X} — should be cp $8",
                addr
            );
        }
    }
}
