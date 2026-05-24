//! Emulator-based tests for the wavy screen animation fix.
//!
//! The bug: `AnimationWavyScreen` uses an HBlank polling loop to set `rSCX`
//! per-scanline, creating a wave distortion for Psychic/Psywave/Night Shade.
//! But the loop only catches scanlines mid-frame — after VBlank ends, several
//! scanlines render before the first HBlank write, leaving the top ~3 lines
//! without the wave effect.
//!
//! The fix: write the current wave offset to `hSCX` at the top of `.loop`,
//! so the VBlank handler copies it to `rSCX` before scanline 0 renders.
//! After the animation, clear `hSCX` so the screen isn't shifted.

use pokeyellow_tests::{sym_addr, TestHarness};

// ─── Test: .loop writes hSCX before entering inner loop ──────────

#[test]
fn loop_sets_hscx_to_first_offset() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    let bank = h.bank_of("AnimationWavyScreen.loop");
    let loop_addr = h.addr_of("AnimationWavyScreen.loop");
    let inner_loop_addr = h.addr_of("AnimationWavyScreen.innerLoop");
    let offsets_addr = h.addr_of("WavyScreenLineOffsets");

    h.select_rom_bank(bank);

    // Clear hSCX to a known value so we can detect the write
    h.write_mem(sym_addr("hSCX"), 0xFF);

    // Set HL to point to a known position in WavyScreenLineOffsets.
    // The table starts with: 0, 0, 0, 0, 0, 1, 1, 1, 2, 2, ...
    // Point to offset +5 (value = 1) for a non-zero test value.
    let hl_val = offsets_addr + 5;
    h.gb.cpu().set_hl(hl_val);

    // Set D = $80 (terminator marker, used by the loop)
    // Set E = SCREEN_HEIGHT_PX - 1 = 143
    h.gb.cpu().set_de(0x808F);

    // Set C = 1 (so the outer loop runs once then exits)
    h.gb.cpu().c = 1;

    h.set_pc(loop_addr);

    // Step exactly 2 instructions: `ld a, [hl]` + `ldh [hSCX], a`
    // ld a, [hl] = 2 M-cycles, ldh [hSCX], a = 3 M-cycles
    h.gb.clock(); // ld a, [hl]
    h.gb.clock(); // ldh [hSCX], a

    // Verify hSCX is set to 1 (the value at offsets_addr + 5)
    let hscx = h.read_mem(sym_addr("hSCX"));
    assert_eq!(
        hscx, 1,
        "hSCX should be set to the wave offset (1), got {hscx:#04x}"
    );

    // PC should now be at `push hl` (one instruction before .innerLoop)
    assert_eq!(
        h.pc(),
        inner_loop_addr - 1,
        "PC should be at push hl, just before .innerLoop"
    );
}

#[test]
fn loop_sets_hscx_zero_for_first_offset() {
    // When HL points to the start of the table (value = 0), hSCX should be 0
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    let bank = h.bank_of("AnimationWavyScreen.loop");
    let loop_addr = h.addr_of("AnimationWavyScreen.loop");
    let offsets_addr = h.addr_of("WavyScreenLineOffsets");

    h.select_rom_bank(bank);

    // Set hSCX to a non-zero value to confirm it gets overwritten
    h.write_mem(sym_addr("hSCX"), 0x42);

    // HL points to start of table (value = 0)
    h.gb.cpu().set_hl(offsets_addr);

    h.set_pc(loop_addr);

    // Step 2 instructions
    h.gb.clock(); // ld a, [hl]
    h.gb.clock(); // ldh [hSCX], a

    let hscx = h.read_mem(sym_addr("hSCX"));
    assert_eq!(
        hscx, 0,
        "hSCX should be 0 when wave offset is 0, got {hscx:#04x}"
    );
}

#[test]
fn loop_sets_hscx_negative_offset() {
    // Test with a negative offset value (-1 = $FF, -2 = $FE)
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    let bank = h.bank_of("AnimationWavyScreen.loop");
    let loop_addr = h.addr_of("AnimationWavyScreen.loop");
    let offsets_addr = h.addr_of("WavyScreenLineOffsets");

    h.select_rom_bank(bank);

    h.write_mem(sym_addr("hSCX"), 0x00);

    // Table offset +24 has value -2 ($FE): the table is
    // 0,0,0,0,0,1,1,1,2,2,2,2,2,1,1,1,0,0,0,0,0,-1,-1,-1,-2,-2,-2,-2,-2,-1,-1,-1
    // Index 24 = -2 = 0xFE
    h.gb.cpu().set_hl(offsets_addr + 24);

    h.set_pc(loop_addr);

    h.gb.clock(); // ld a, [hl]
    h.gb.clock(); // ldh [hSCX], a

    let hscx = h.read_mem(sym_addr("hSCX"));
    assert_eq!(
        hscx, 0xFE,
        "hSCX should be $FE (-2) for negative wave offset, got {hscx:#04x}"
    );
}

// ─── Test: cleanup clears hSCX after animation ──────────────────

#[test]
fn cleanup_clears_hscx() {
    // Verify that after the loop exits, the code clears hSCX to 0.
    // We enter just after the loop (at `xor a` / `ldh [hSCX], a`)
    // by setting C=0 at .next so `dec c / jr nz, .loop` falls through.
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    let bank = h.bank_of("AnimationWavyScreen.next");
    let next_addr = h.addr_of("AnimationWavyScreen.next");

    h.select_rom_bank(bank);

    // Set hSCX to a non-zero value
    h.write_mem(sym_addr("hSCX"), 0x42);

    // Set C=1 so `dec c` makes it 0 and `jr nz, .loop` falls through
    h.gb.cpu().c = 1;

    h.set_pc(next_addr);

    // Step: dec c (1 cycle) / jr nz, .loop (2 cycles, not taken) / xor a (1) / ldh [hSCX], a (3)
    // = 4 instructions until hSCX is written
    for _ in 0..4 {
        h.gb.clock();
    }

    let hscx = h.read_mem(sym_addr("hSCX"));
    assert_eq!(
        hscx, 0,
        "hSCX should be cleared to 0 after animation loop, got {hscx:#04x}"
    );
}

// ─── Test: verify fix bytes in ROM ──────────────────────────────

#[test]
fn rom_bytes_verify_fix_present() {
    // Verify the fix instructions exist at .loop:
    // $7E = ld a, [hl]
    // $E0 $AE = ldh [hSCX], a  (hSCX = $FFAE)
    // $E5 = push hl
    let rom = pokeyellow_tests::load_rom_bytes(&pokeyellow_tests::rom_path().to_string_lossy());
    let sym = pokeyellow_tests::SymbolTable::load(&pokeyellow_tests::sym_path().to_string_lossy());

    let loop_offset = sym
        .rom_offset("AnimationWavyScreen.loop")
        .expect("AnimationWavyScreen.loop not found in sym file");

    assert_eq!(rom[loop_offset], 0x7E, "Expected `ld a, [hl]` at .loop");
    assert_eq!(
        rom[loop_offset + 1],
        0xE0,
        "Expected `ldh [n], a` opcode at .loop+1"
    );
    assert_eq!(
        rom[loop_offset + 2],
        0xAE,
        "Expected hSCX offset ($AE) at .loop+2"
    );
    assert_eq!(rom[loop_offset + 3], 0xE5, "Expected `push hl` at .loop+3");
}
