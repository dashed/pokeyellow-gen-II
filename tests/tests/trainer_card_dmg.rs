//! ROM byte tests for the Trainer Card DMG transition garbage fix.
//!
//! Bug: On DMG with modern IPS LCD screen mods, the Trainer Card screen
//! can show brief garbage when loading into and out of it. The faster
//! IPS LCD response time makes partially-loaded data visible during
//! transitions that were hidden on the original slow LCD.
//!
//! Fix: Add `Delay3` calls gated on `wOnSGB == 0` at two points in
//! `StartMenu_TrainerInfo`: after `RunPaletteCommand` (before the card
//! becomes visible) and after `DrawStartMenu` (before restoring the map
//! palette). +14 bytes in bank $04.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("StartMenu_TrainerInfo"));
    h
}

/// Pattern: ld a, [wOnSGB] ($FA $1A $CF) / and a ($A7) / call z, Delay3 ($CC lo hi)
fn find_sgb_delay_pattern(h: &mut TestHarness, start: u16, end: u16) -> Option<u16> {
    let delay3 = sym_addr("Delay3");
    let d3_lo = (delay3 & 0xFF) as u8;
    let d3_hi = (delay3 >> 8) as u8;
    let on_sgb = sym_addr("wOnSGB");
    let sgb_lo = (on_sgb & 0xFF) as u8;
    let sgb_hi = (on_sgb >> 8) as u8;

    for addr in start..end {
        if rom(h, addr) == 0xFA
            && rom(h, addr + 1) == sgb_lo
            && rom(h, addr + 2) == sgb_hi
            && rom(h, addr + 3) == 0xA7
            && rom(h, addr + 4) == 0xCC
            && rom(h, addr + 5) == d3_lo
            && rom(h, addr + 6) == d3_hi
        {
            return Some(addr);
        }
    }
    None
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn trainer_info_in_bank_04() {
    assert_eq!(sym_bank("StartMenu_TrainerInfo"), 0x04);
}

#[test]
fn entry_delay_after_run_palette_command() {
    // The first Delay3 pattern should come after `call RunPaletteCommand`
    let mut h = banked_harness();
    let base = sym_addr("StartMenu_TrainerInfo");
    let end = base + 80;

    let run_pal = sym_addr("RunPaletteCommand");
    let rp_lo = (run_pal & 0xFF) as u8;
    let rp_hi = (run_pal >> 8) as u8;

    // Find call RunPaletteCommand
    let mut call_pos = None;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == rp_lo
            && rom(&mut h, addr + 2) == rp_hi
        {
            call_pos = Some(addr);
            break;
        }
    }
    assert!(call_pos.is_some(), "call RunPaletteCommand not found");

    // The SGB+Delay3 pattern should start after the call
    let pattern = find_sgb_delay_pattern(&mut h, call_pos.unwrap() + 3, end);
    assert!(
        pattern.is_some(),
        "wOnSGB/Delay3 pattern not found after RunPaletteCommand"
    );
}

#[test]
fn entry_delay_before_gb_pal_normal() {
    // The first Delay3 pattern should come before `call GBPalNormal`
    let mut h = banked_harness();
    let base = sym_addr("StartMenu_TrainerInfo");
    let end = base + 80;

    let gb_pal = sym_addr("GBPalNormal");
    let gp_lo = (gb_pal & 0xFF) as u8;
    let gp_hi = (gb_pal >> 8) as u8;

    // Find call GBPalNormal
    let mut pal_pos = None;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == gp_lo
            && rom(&mut h, addr + 2) == gp_hi
        {
            pal_pos = Some(addr);
            break;
        }
    }
    assert!(pal_pos.is_some(), "call GBPalNormal not found");

    let pattern = find_sgb_delay_pattern(&mut h, base, pal_pos.unwrap());
    assert!(
        pattern.is_some(),
        "wOnSGB/Delay3 pattern not found before GBPalNormal"
    );
}

#[test]
fn entry_delay_uses_call_z() {
    // Verify the conditional call is `call z` ($CC), not unconditional `call` ($CD)
    let mut h = banked_harness();
    let base = sym_addr("StartMenu_TrainerInfo");
    let end = base + 80;

    let pattern_addr = find_sgb_delay_pattern(&mut h, base, end);
    assert!(pattern_addr.is_some(), "first SGB/Delay3 pattern not found");
    // The call z opcode is at pattern_addr + 4
    assert_eq!(
        rom(&mut h, pattern_addr.unwrap() + 4),
        0xCC,
        "expected call z ($CC), not unconditional call"
    );
}

#[test]
fn exit_delay_before_load_gb_pal() {
    // The second Delay3 pattern should come before `call LoadGBPal`
    let mut h = banked_harness();
    let base = sym_addr("StartMenu_TrainerInfo");
    let end = base + 100;

    let load_pal = sym_addr("LoadGBPal");
    let lp_lo = (load_pal & 0xFF) as u8;
    let lp_hi = (load_pal >> 8) as u8;

    // Find call LoadGBPal
    let mut load_pos = None;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == lp_lo
            && rom(&mut h, addr + 2) == lp_hi
        {
            load_pos = Some(addr);
            break;
        }
    }
    assert!(load_pos.is_some(), "call LoadGBPal not found");

    // Search backwards from LoadGBPal for the pattern
    // The pattern is 7 bytes, so it should start at load_pos - 7
    let pattern = find_sgb_delay_pattern(&mut h, load_pos.unwrap() - 10, load_pos.unwrap());
    assert!(
        pattern.is_some(),
        "wOnSGB/Delay3 pattern not found before LoadGBPal"
    );
}

#[test]
fn exit_delay_after_reload_map_data() {
    // The second Delay3 pattern should come after `call ReloadMapData`
    let mut h = banked_harness();
    let base = sym_addr("StartMenu_TrainerInfo");
    let end = base + 100;

    let reload = sym_addr("ReloadMapData");
    let rm_lo = (reload & 0xFF) as u8;
    let rm_hi = (reload >> 8) as u8;

    // Find call ReloadMapData
    let mut reload_pos = None;
    for addr in base..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == rm_lo
            && rom(&mut h, addr + 2) == rm_hi
        {
            reload_pos = Some(addr);
            break;
        }
    }
    assert!(reload_pos.is_some(), "call ReloadMapData not found");

    let pattern = find_sgb_delay_pattern(&mut h, reload_pos.unwrap() + 3, end);
    assert!(
        pattern.is_some(),
        "wOnSGB/Delay3 pattern not found after ReloadMapData"
    );
}

#[test]
fn exit_delay_uses_call_z() {
    // Verify the second conditional call is also `call z` ($CC)
    let mut h = banked_harness();
    let base = sym_addr("StartMenu_TrainerInfo");
    let end = base + 100;

    // Find the second SGB/Delay3 pattern (skip the first)
    let first = find_sgb_delay_pattern(&mut h, base, end);
    assert!(first.is_some(), "first SGB/Delay3 pattern not found");

    let second = find_sgb_delay_pattern(&mut h, first.unwrap() + 7, end);
    assert!(second.is_some(), "second SGB/Delay3 pattern not found");
    assert_eq!(
        rom(&mut h, second.unwrap() + 4),
        0xCC,
        "expected call z ($CC) for exit delay"
    );
}

#[test]
fn exactly_two_delay_patterns() {
    // There should be exactly 2 wOnSGB/Delay3 patterns in the function
    let mut h = banked_harness();
    let base = sym_addr("StartMenu_TrainerInfo");
    // Function ends at jp RedisplayStartMenu_DoNotDrawStartMenu — scan ~100 bytes
    let end = base + 100;

    let mut count = 0;
    let mut search_start = base;
    loop {
        match find_sgb_delay_pattern(&mut h, search_start, end) {
            Some(addr) => {
                count += 1;
                search_start = addr + 7;
            }
            None => break,
        }
    }
    assert_eq!(
        count, 2,
        "expected exactly 2 wOnSGB/Delay3 patterns, found {}",
        count
    );
}
