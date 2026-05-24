//! ROM byte tests for the tile pair collision loop misalignment fix.
//!
//! Bug: `CheckForTilePairCollisions` iterates over tile pair arrays
//! (`TilePairCollisionsLand`/`Water`) in 3-byte entries: [tileset, tile1, tile2].
//! In `.currentTileMatchesFirstInPair`, when tile1 matches the player's tile
//! but tile2 does not match the tile in front, the code jumps back to
//! `.tilePairCollisionLoop` with HL still pointing at tile2. The loop then
//! reads tile2 as the next entry's tileset byte, misaligning ALL subsequent
//! reads. This causes incorrect collision detection and poor performance
//! (scanning garbage data) if more tile pairs are added to the arrays.
//!
//! Fix: Change `jr .tilePairCollisionLoop` to `jr .retry` so HL advances
//! past tile2 via the `inc hl` at `.retry` before restarting the loop.
//! Zero ROM growth — only the relative jump offset changes.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    TestHarness::new_headless()
    // HOME bank — no ROM bank selection needed
}

// ─── Structural tests ────────────────────────────────────────────────

#[test]
fn check_for_tile_pair_collisions_in_home() {
    assert_eq!(sym_bank("CheckForTilePairCollisions"), 0x00);
}

#[test]
fn loop_start_loads_tileset() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("CheckForTilePairCollisions.tilePairCollisionLoop");
    // ld a, [wCurMapTileset] → $FA lo hi (3 bytes)
    assert_eq!(rom(&mut h, loop_addr), 0xFA, "ld a, [nn] opcode");
}

#[test]
fn entry_read_uses_hli() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("CheckForTilePairCollisions.tilePairCollisionLoop");
    // After ld a, [wCurMapTileset] (3 bytes) + ld b, a (1 byte) = offset +4
    // ld a, [hli] → $2A
    assert_eq!(
        rom(&mut h, loop_addr + 4),
        0x2A,
        "ld a, [hli] reads tileset from array"
    );
}

#[test]
fn end_marker_check_ff() {
    let mut h = rom_harness();
    let loop_addr = sym_addr("CheckForTilePairCollisions.tilePairCollisionLoop");
    // After ld a, [hli] at +4 = offset +5
    // cp $ff → $FE $FF
    assert_eq!(rom(&mut h, loop_addr + 5), 0xFE, "cp n opcode");
    assert_eq!(rom(&mut h, loop_addr + 6), 0xFF, "cp $FF end marker");
}

// ─── THE FIX: first pair non-match targets .retry ────────────────────

#[test]
fn first_pair_nonmatch_targets_retry() {
    let mut h = rom_harness();
    let first_pair = sym_addr("CheckForTilePairCollisions.currentTileMatchesFirstInPair");
    // .currentTileMatchesFirstInPair:
    //   inc hl      (+0, 1 byte)
    //   ld a, [hl]  (+1, 1 byte)
    //   cp c        (+2, 1 byte)
    //   jr z, .foundMatch (+3, 2 bytes)
    //   jr .retry   (+5, 2 bytes) ← THE FIX
    let jr_addr = first_pair + 5;
    assert_eq!(rom(&mut h, jr_addr), 0x18, "jr opcode");
    let offset = rom(&mut h, jr_addr + 1) as i8;
    let target = (jr_addr + 2).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("CheckForTilePairCollisions.retry"),
        "jr should target .retry (not .tilePairCollisionLoop)"
    );
}

#[test]
fn retry_has_inc_hl_before_loop() {
    let mut h = rom_harness();
    let retry = sym_addr("CheckForTilePairCollisions.retry");
    // .retry: inc hl → $23
    assert_eq!(rom(&mut h, retry), 0x23, "inc hl at .retry");
    // Followed by jr .tilePairCollisionLoop → $18 xx
    assert_eq!(
        rom(&mut h, retry + 1),
        0x18,
        "jr opcode after inc hl at .retry"
    );
    let offset = rom(&mut h, retry + 2) as i8;
    let target = (retry + 3).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("CheckForTilePairCollisions.tilePairCollisionLoop"),
        ".retry jr should target .tilePairCollisionLoop"
    );
}

// ─── Second pair path is correct ─────────────────────────────────────

#[test]
fn second_pair_nonmatch_advances_past_tile2() {
    let mut h = rom_harness();
    let second_pair = sym_addr("CheckForTilePairCollisions.currentTileMatchesSecondInPair");
    // .currentTileMatchesSecondInPair:
    //   dec hl      (+0, 1 byte)
    //   ld a, [hli] (+1, 1 byte)
    //   cp c        (+2, 1 byte)
    //   inc hl      (+3, 1 byte) ← advances past tile2
    //   jr nz, .tilePairCollisionLoop (+4, 2 bytes)
    assert_eq!(
        rom(&mut h, second_pair + 3),
        0x23,
        "inc hl advances past tile2 before loop restart"
    );
    assert_eq!(rom(&mut h, second_pair + 4), 0x20, "jr nz opcode");
    let offset = rom(&mut h, second_pair + 5) as i8;
    let target = (second_pair + 6).wrapping_add(offset as u16);
    assert_eq!(
        target,
        sym_addr("CheckForTilePairCollisions.tilePairCollisionLoop"),
        "jr nz should target .tilePairCollisionLoop"
    );
}

// ─── Negative test ───────────────────────────────────────────────────

#[test]
fn first_pair_nonmatch_does_not_target_loop_directly() {
    let mut h = rom_harness();
    let first_pair = sym_addr("CheckForTilePairCollisions.currentTileMatchesFirstInPair");
    let jr_addr = first_pair + 5;
    let offset = rom(&mut h, jr_addr + 1) as i8;
    let target = (jr_addr + 2).wrapping_add(offset as u16);
    assert_ne!(
        target,
        sym_addr("CheckForTilePairCollisions.tilePairCollisionLoop"),
        "first pair non-match must NOT jump directly to .tilePairCollisionLoop (would skip tile2)"
    );
}
