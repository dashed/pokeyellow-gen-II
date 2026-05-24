//! ROM byte tests for the bicycle-hole music fix.
//!
//! Bug: When falling through a hole while riding the Bicycle, the bike
//! music keeps playing on the new map. `LeaveMapThroughHoleAnim` runs
//! the visual falling animation but never resets the music, so the bike
//! theme persists even though the player dismounts upon landing.
//!
//! Fix: At the start of `LeaveMapThroughHoleAnim`, check if bike music
//! is playing (`wLastMusicSoundID == MUSIC_BIKE_RIDING`) and call
//! `PlayDefaultMusic` to reset it before the animation plays.
//!
//! References:
//!   - <https://glitchcity.wiki/wiki/Bicycle_music_hole_glitch>
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)#Victory_Road_Bicycle_music_quirk>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// Helper to read a ROM byte at a banked address.
fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

/// Create a TestHarness with LeaveMapThroughHoleAnim bank selected.
fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("LeaveMapThroughHoleAnim"));
    h
}

// ─── Structural tests for the bike music check ─────────────────────

#[test]
fn hole_anim_starts_with_ld_a_last_music() {
    let mut h = rom_harness();
    let addr = sym_addr("LeaveMapThroughHoleAnim");
    // ld a, [wLastMusicSoundID] → $FA lo hi ($CFC9)
    assert_eq!(rom(&mut h, addr), 0xFA, "ld a, [nn] opcode");
    assert_eq!(rom(&mut h, addr + 1), 0xC9, "wLastMusicSoundID low byte");
    assert_eq!(rom(&mut h, addr + 2), 0xCF, "wLastMusicSoundID high byte");
}

#[test]
fn cp_music_bike_riding() {
    let mut h = rom_harness();
    let addr = sym_addr("LeaveMapThroughHoleAnim");
    // cp MUSIC_BIKE_RIDING → $FE $D2
    assert_eq!(rom(&mut h, addr + 3), 0xFE, "cp imm8 opcode");
    assert_eq!(
        rom(&mut h, addr + 4),
        0xD2,
        "MUSIC_BIKE_RIDING constant ($D2)"
    );
}

#[test]
fn call_z_play_default_music() {
    let mut h = rom_harness();
    let addr = sym_addr("LeaveMapThroughHoleAnim");
    let target = sym_addr("PlayDefaultMusic");
    // call z, PlayDefaultMusic → $CC lo hi
    assert_eq!(rom(&mut h, addr + 5), 0xCC, "call z opcode");
    let call_lo = rom(&mut h, addr + 6);
    let call_hi = rom(&mut h, addr + 7);
    let call_target = u16::from(call_hi) << 8 | u16::from(call_lo);
    assert_eq!(
        call_target, target,
        "call target should be PlayDefaultMusic"
    );
}

#[test]
fn original_code_follows_music_check() {
    let mut h = rom_harness();
    let addr = sym_addr("LeaveMapThroughHoleAnim");
    // offset 8: ld a, $ff → $3E $FF (original first instruction)
    assert_eq!(
        rom(&mut h, addr + 8),
        0x3E,
        "ld a, imm8 opcode (original code resumes)"
    );
    assert_eq!(rom(&mut h, addr + 9), 0xFF, "$FF = disable UpdateSprites");
}

#[test]
fn music_check_is_exactly_8_bytes() {
    let mut h = rom_harness();
    let addr = sym_addr("LeaveMapThroughHoleAnim");
    // Full sequence: FA C9 CF FE D2 CC xx xx (8 bytes)
    let expected_prefix = [0xFA, 0xC9, 0xCF, 0xFE, 0xD2, 0xCC];
    for (i, &exp) in expected_prefix.iter().enumerate() {
        assert_eq!(
            rom(&mut h, addr + i as u16),
            exp,
            "byte {} of music check",
            i
        );
    }
    // Bytes 6-7 are the PlayDefaultMusic address (verified in call_z test)
    // Byte 8 should be the original $3E (ld a, $FF)
    assert_eq!(
        rom(&mut h, addr + 8),
        0x3E,
        "original code starts at offset 8"
    );
}

#[test]
fn leave_map_through_hole_anim_is_in_bank_1c() {
    assert_eq!(
        sym_bank("LeaveMapThroughHoleAnim"),
        0x1C,
        "LeaveMapThroughHoleAnim should be in bank $1C"
    );
}

#[test]
fn play_default_music_is_in_home_bank() {
    assert_eq!(
        sym_bank("PlayDefaultMusic"),
        0x00,
        "PlayDefaultMusic should be in HOME (bank 0)"
    );
}
