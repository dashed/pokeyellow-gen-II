//! ROM byte tests for the ZZAZZ glitch fix.
//!
//! Bug: ReadTrainer.LastLoop calculates prize money by repeatedly adding
//! wTrainerBaseMoney to wAmountMoneyWon via AddBCD.  When the BCD addition
//! overflows $9999, AddBCD's overflow handler advances the DE pointer past
//! wAmountMoneyWon.  The original code used `inc de / inc de` to restore DE,
//! which only works when there is no overflow.  With overflow, DE drifts
//! forward by 3 bytes per iteration, spraying $99 across WRAM — corrupting
//! the player name (→ "ZZAZZ"), party data, and hundreds of other variables.
//!
//! Fix: Replace `inc de / inc de` with `ld de, wAmountMoneyWon + 2` to
//! unconditionally reload DE after each AddBCD call, preventing pointer drift.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/ZZAZZ_glitch>
//! Reference: <https://glitchcity.wiki/wiki/ZZAZZ_glitch>

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

const LD_DE_NN: u8 = 0x11; // ld de, nn
const DEC_B: u8 = 0x05; // dec b
const JR_NZ: u8 = 0x20; // jr nz, n
const RET: u8 = 0xC9; // ret
const INC_DE: u8 = 0x13; // inc de (the buggy instruction)

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn read_trainer_in_bank_0e() {
    assert_eq!(sym_bank("ReadTrainer"), 0x0E);
}

#[test]
fn last_loop_reloads_de_for_zzazz_fix() {
    // ReadTrainer.LastLoop should end with:
    //   ld de, wAmountMoneyWon + 2  ($11 lo hi)   ← ZZAZZ fix
    //   dec b                       ($05)
    //   jr nz, .LastLoop            ($20 nn)
    //   ret                         ($C9)
    //
    // SpecialTrainerMoves immediately follows, so we work backward.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0E);

    let end = sym_addr("SpecialTrainerMoves");
    // ret (1) + jr nz (2) + dec b (1) + ld de,nn (3) = 7 bytes before end
    let ld_de_addr = end - 7;

    assert_eq!(
        rom(&mut h, ld_de_addr),
        LD_DE_NN,
        "Expected `ld de, nn` (ZZAZZ fix reload)"
    );
}

#[test]
fn ld_de_target_is_amount_money_won_plus_2() {
    // The ld de, nn operand should be wAmountMoneyWon + 2
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0E);

    let end = sym_addr("SpecialTrainerMoves");
    let ld_de_addr = end - 7;
    let expected = sym_addr("wAmountMoneyWon") + 2;

    assert_eq!(
        rom16(&mut h, ld_de_addr + 1),
        expected,
        "ld de target should be wAmountMoneyWon + 2 ({:#06X})",
        expected
    );
}

#[test]
fn loop_control_intact() {
    // After ld de, nn: dec b / jr nz / ret
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0E);

    let end = sym_addr("SpecialTrainerMoves");

    assert_eq!(rom(&mut h, end - 4), DEC_B, "Expected `dec b`");
    assert_eq!(rom(&mut h, end - 3), JR_NZ, "Expected `jr nz`");
    assert_eq!(rom(&mut h, end - 1), RET, "Expected `ret`");
}

#[test]
fn no_consecutive_inc_de_in_last_loop() {
    // The old buggy pattern `inc de; inc de` ($13 $13) should NOT appear
    // anywhere between .LastLoop and SpecialTrainerMoves.
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0E);

    let start = sym_addr("ReadTrainer.LastLoop");
    let end = sym_addr("SpecialTrainerMoves");

    for addr in start..end.saturating_sub(1) {
        if rom(&mut h, addr) == INC_DE && rom(&mut h, addr + 1) == INC_DE {
            panic!(
                "Found consecutive `inc de; inc de` at {:#06X} — ZZAZZ bug still present",
                addr
            );
        }
    }
}
