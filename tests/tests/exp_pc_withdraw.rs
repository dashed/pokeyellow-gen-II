//! ROM byte tests for the Experience PC withdrawal freeze fix.
//!
//! Bug: When withdrawing a level 1 Pokémon with the "Medium Slow" growth
//! algorithm from the PC, CalcLevelFromExperience loops indefinitely because
//! the experience underflow (−54 → 16,777,162 unsigned) exceeds every level's
//! requirement.  The register `d` wraps past 255 and the loop never terminates,
//! softlocking the game.  This also affects glitch Pokémon with corrupted
//! experience values or invalid growth rates.
//!
//! Fix: Add a MAX_LEVEL (100) cap inside CalcLevelFromExperience.  When `d`
//! reaches MAX_LEVEL + 1, jump to the existing `dec d / ret` path, returning
//! MAX_LEVEL instead of looping forever.  Cost: +5 bytes in bank $16.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/Experience#Experience_underflow_glitch>

use pokeyellow_tests::{sym_addr, sym_bank};

// ─── Opcode constants ────────────────────────────────────────────────

const INC_D: u8 = 0x14; // inc d
const LD_A_D: u8 = 0x7A; // ld a, d
const CP_N: u8 = 0xFE; // cp n
const JR_Z: u8 = 0x28; // jr z, n
const DEC_D: u8 = 0x15; // dec d
const RET: u8 = 0xC9; // ret
const MAX_LEVEL: u8 = 100;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn calc_level_from_experience_in_bank_16() {
    assert_eq!(sym_bank("CalcLevelFromExperience"), 0x16);
}

#[test]
fn loop_has_max_level_cap() {
    // At .loop, the fix inserts after `inc d`:
    //   ld a, d       ($7A)
    //   cp MAX_LEVEL+1 ($FE $65)
    //   jr z, .done   ($28 nn)
    let rom = std::fs::read("../pokeyellow.gbc").expect("ROM not found");
    let bank: u32 = 0x16;
    let loop_addr = sym_addr("CalcLevelFromExperience.loop");

    let rom_offset = |addr: u16| -> usize { (bank * 0x4000 + (addr as u32 - 0x4000)) as usize };

    let at = |addr: u16| -> u8 { rom[rom_offset(addr)] };

    assert_eq!(at(loop_addr), INC_D, "Expected `inc d` at .loop");
    assert_eq!(at(loop_addr + 1), LD_A_D, "Expected `ld a, d` after inc d");
    assert_eq!(at(loop_addr + 2), CP_N, "Expected `cp n` opcode");
    assert_eq!(
        at(loop_addr + 3),
        MAX_LEVEL + 1,
        "Expected cp operand = MAX_LEVEL + 1 ({})",
        MAX_LEVEL + 1
    );
    assert_eq!(
        at(loop_addr + 4),
        JR_Z,
        "Expected `jr z` to cap at MAX_LEVEL"
    );
}

#[test]
fn jr_z_targets_done_label() {
    // The jr z at .loop+4 should target .done (dec d / ret)
    let rom = std::fs::read("../pokeyellow.gbc").expect("ROM not found");
    let bank: u32 = 0x16;
    let loop_addr = sym_addr("CalcLevelFromExperience.loop");
    let done_addr = sym_addr("CalcLevelFromExperience.done");

    let rom_offset = |addr: u16| -> usize { (bank * 0x4000 + (addr as u32 - 0x4000)) as usize };

    // jr z is at loop_addr + 4, operand at loop_addr + 5
    let jr_addr = loop_addr + 4;
    let jr_operand = rom[rom_offset(jr_addr + 1)] as i8;
    let target = (jr_addr as i32 + 2 + jr_operand as i32) as u16;

    assert_eq!(
        target, done_addr,
        "jr z should target .done ({:#06X}), got {:#06X}",
        done_addr, target
    );
}

#[test]
fn done_has_dec_d_ret() {
    // .done should be: dec d ($15) / ret ($C9)
    let rom = std::fs::read("../pokeyellow.gbc").expect("ROM not found");
    let bank: u32 = 0x16;
    let done_addr = sym_addr("CalcLevelFromExperience.done");

    let rom_offset = |addr: u16| -> usize { (bank * 0x4000 + (addr as u32 - 0x4000)) as usize };

    assert_eq!(
        rom[rom_offset(done_addr)],
        DEC_D,
        "Expected `dec d` at .done"
    );
    assert_eq!(
        rom[rom_offset(done_addr + 1)],
        RET,
        "Expected `ret` after dec d"
    );
}

#[test]
fn done_is_shared_with_normal_exit() {
    // The .done label should be reachable from both the MAX_LEVEL cap
    // AND the normal loop exit (jr nc falls through to .done).
    // Verify .done comes right after the jr nc instruction.
    let rom = std::fs::read("../pokeyellow.gbc").expect("ROM not found");
    let bank: u32 = 0x16;
    let loop_addr = sym_addr("CalcLevelFromExperience.loop");
    let done_addr = sym_addr("CalcLevelFromExperience.done");

    let rom_offset = |addr: u16| -> usize { (bank * 0x4000 + (addr as u32 - 0x4000)) as usize };

    // The jr nc .loop instruction should be 2 bytes before .done
    // (jr nc = 0x30, operand = signed offset)
    let jr_nc_addr = done_addr - 2;
    assert_eq!(
        rom[rom_offset(jr_nc_addr)],
        0x30, // jr nc
        "Expected `jr nc` two bytes before .done"
    );

    // Verify the jr nc targets .loop
    let jr_operand = rom[rom_offset(jr_nc_addr + 1)] as i8;
    let target = (jr_nc_addr as i32 + 2 + jr_operand as i32) as u16;
    assert_eq!(
        target, loop_addr,
        "jr nc should target .loop ({:#06X}), got {:#06X}",
        loop_addr, target
    );
}
