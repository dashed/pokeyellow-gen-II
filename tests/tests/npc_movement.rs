//! ROM byte tests for NPC movement fixes in `CanWalkOntoTile` and
//! `UpdateSpriteMovementDelay`.
//!
//! Bug 1 (movement restriction): `CanWalkOntoTile` only enforces lower
//! bounds (sub 1 / jr c at displacement 0) for upward and leftward NPC
//! movement. The intended upper bound checks for downward and rightward
//! movement (`cp $5`) exist but have no conditional jump — NPCs can
//! walk unlimited steps down or right until the counter overflows.
//!
//! Fix 1: Replace the dead `cp $5` with `cp $11 / jr nc, .impassable`
//! for both down and right paths. Displacement starts at $8, creating
//! symmetric 8-step bounds in all 4 directions ($0–$10). +4 bytes.
//!
//! Bug 2 (offscreen border): The screen boundary checks use `cp $80`
//! for Y and `cp $90` for X with `jr nc, .impassable`. Since `jr nc`
//! triggers when A >= operand, $80 (bottom row) and $90 (rightmost
//! column) are treated as offscreen when they are valid positions.
//!
//! Fix 2: Change `cp $80` to `cp $81` and `cp $90` to `cp $91`. Zero
//! ROM growth — only the immediate operands change.
//!
//! Bug 3 (movement delay wraparound): `UpdateSpriteMovementDelay`
//! decrements the delay counter with `dec [hl]` then checks `jr nz`.
//! When the random delay is 0, `dec` wraps to $FF and the NPC waits
//! 256 extra frames (~4.3 seconds) before moving.
//!
//! Fix 3: Add `ld a, [hl] / and a / jr z, .moving` before the `dec`
//! so delay 0 means "move immediately". +4 bytes.
//!
//! Reference:
//!   - <https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("CanWalkOntoTile"));
    h
}

// ─── Structural tests ──────────────────────────────────────────────

#[test]
fn can_walk_onto_tile_is_in_bank_01() {
    assert_eq!(
        sym_bank("CanWalkOntoTile"),
        0x01,
        "CanWalkOntoTile should be in bank $01"
    );
}

#[test]
fn downward_upper_bound_cp_11() {
    let mut h = rom_harness();
    // .upwards label marks the start of the upward path.
    // The downward path ends just before .upwards:
    //   cp $11 (2 bytes) / jr nc, .impassable (2 bytes) / jr .checkHorizontal (2 bytes) / .upwards:
    let upwards = sym_addr("CanWalkOntoTile.upwards");
    // cp $11 is at upwards - 6
    assert_eq!(
        rom(&mut h, upwards - 6),
        0xFE,
        "cp n opcode for Y upper bound"
    );
    assert_eq!(
        rom(&mut h, upwards - 5),
        0x11,
        "cp $11 immediate (8 steps from center)"
    );
}

#[test]
fn downward_jr_nc_impassable() {
    let mut h = rom_harness();
    let upwards = sym_addr("CanWalkOntoTile.upwards");
    // jr nc is at upwards - 4
    assert_eq!(
        rom(&mut h, upwards - 4),
        0x30,
        "jr nc opcode after cp $11 (Y)"
    );
    // Verify the jr nc target is .impassable
    let jr_offset = rom(&mut h, upwards - 3) as i8;
    let jr_pc = upwards - 2; // PC after reading the jr instruction
    let target = (jr_pc as i32 + jr_offset as i32) as u16;
    assert_eq!(
        target,
        sym_addr("CanWalkOntoTile.impassable"),
        "jr nc should target .impassable"
    );
}

#[test]
fn rightward_upper_bound_cp_11() {
    let mut h = rom_harness();
    // .left label marks the start of the leftward path.
    // The rightward path ends just before .left:
    //   cp $11 (2 bytes) / jr nc, .impassable (2 bytes) / jr .passable (2 bytes) / .left:
    let left = sym_addr("CanWalkOntoTile.left");
    assert_eq!(rom(&mut h, left - 6), 0xFE, "cp n opcode for X upper bound");
    assert_eq!(
        rom(&mut h, left - 5),
        0x11,
        "cp $11 immediate (8 steps from center)"
    );
}

#[test]
fn rightward_jr_nc_impassable() {
    let mut h = rom_harness();
    let left = sym_addr("CanWalkOntoTile.left");
    assert_eq!(rom(&mut h, left - 4), 0x30, "jr nc opcode after cp $11 (X)");
    let jr_offset = rom(&mut h, left - 3) as i8;
    let jr_pc = left - 2;
    let target = (jr_pc as i32 + jr_offset as i32) as u16;
    assert_eq!(
        target,
        sym_addr("CanWalkOntoTile.impassable"),
        "jr nc should target .impassable"
    );
}

#[test]
fn upward_lower_bound_preserved() {
    let mut h = rom_harness();
    let upwards = sym_addr("CanWalkOntoTile.upwards");
    // .upwards: sub $1 (2 bytes) / jr c, .impassable (2 bytes)
    assert_eq!(rom(&mut h, upwards), 0xD6, "sub n opcode (Y lower bound)");
    assert_eq!(rom(&mut h, upwards + 1), 0x01, "sub $1 immediate");
    assert_eq!(
        rom(&mut h, upwards + 2),
        0x38,
        "jr c opcode (Y lower bound)"
    );
}

#[test]
fn leftward_lower_bound_preserved() {
    let mut h = rom_harness();
    let left = sym_addr("CanWalkOntoTile.left");
    // .left: sub $1 (2 bytes) / jr c, .impassable (2 bytes)
    assert_eq!(rom(&mut h, left), 0xD6, "sub n opcode (X lower bound)");
    assert_eq!(rom(&mut h, left + 1), 0x01, "sub $1 immediate");
    assert_eq!(rom(&mut h, left + 2), 0x38, "jr c opcode (X lower bound)");
}

#[test]
fn displacement_initialized_at_8() {
    let mut h = rom_harness();
    let init = sym_addr("InitializeSpriteStatus");
    // InitializeSpriteStatus sets both displacements to $8.
    // The sequence includes: ld a, $8 / ld [hli], a / ld [hl], a
    // Search for $3E $08 (ld a, $8) near the init function.
    // From code: offset +8 from InitializeSpriteStatus is `ld a, $8`
    // (after: ld [hl], $1 / inc l / ld [hl], $ff / inc h / ldh a / add $2 / ld l, a / ld a, $8)
    // Let's scan for $3E $08 within the first 20 bytes
    let mut found = false;
    for i in 0..20 {
        if rom(&mut h, init + i) == 0x3E && rom(&mut h, init + i + 1) == 0x08 {
            // Verify ld [hli], a follows
            assert_eq!(
                rom(&mut h, init + i + 2),
                0x22,
                "ld [hli], a should follow ld a, $8"
            );
            // Verify ld [hl], a follows that
            assert_eq!(
                rom(&mut h, init + i + 3),
                0x77,
                "ld [hl], a should set X displacement"
            );
            found = true;
            break;
        }
    }
    assert!(found, "ld a, $8 not found in InitializeSpriteStatus");
}

// ─── Screen boundary tests (offscreen border fix) ───────────────────

#[test]
fn y_screen_boundary_cp_81() {
    let mut h = rom_harness();
    let not_scripted = sym_addr("CanWalkOntoTile.notScripted");
    // Scan from .notScripted for the pattern: add d ($82) / cp $81 ($FE $81)
    // add d = opcode $82. We search for $82 followed by $FE $81.
    let mut found = false;
    for i in 0..40 {
        if rom(&mut h, not_scripted + i) == 0x82
            && rom(&mut h, not_scripted + i + 1) == 0xFE
            && rom(&mut h, not_scripted + i + 2) == 0x81
        {
            // Verify jr nc follows
            assert_eq!(
                rom(&mut h, not_scripted + i + 3),
                0x30,
                "jr nc should follow cp $81 (Y boundary)"
            );
            found = true;
            break;
        }
    }
    assert!(
        found,
        "add d / cp $81 / jr nc pattern not found for Y screen boundary"
    );
}

#[test]
fn x_screen_boundary_cp_91() {
    let mut h = rom_harness();
    let not_scripted = sym_addr("CanWalkOntoTile.notScripted");
    // Scan for: add e ($83) / cp $91 ($FE $91)
    let mut found = false;
    for i in 0..50 {
        if rom(&mut h, not_scripted + i) == 0x83
            && rom(&mut h, not_scripted + i + 1) == 0xFE
            && rom(&mut h, not_scripted + i + 2) == 0x91
        {
            assert_eq!(
                rom(&mut h, not_scripted + i + 3),
                0x30,
                "jr nc should follow cp $91 (X boundary)"
            );
            found = true;
            break;
        }
    }
    assert!(
        found,
        "add e / cp $91 / jr nc pattern not found for X screen boundary"
    );
}

// ─── Movement delay wraparound fix tests ─────────────────────────────

#[test]
fn delay_zero_check_before_dec() {
    let mut h = rom_harness();
    let tick = sym_addr("UpdateSpriteMovementDelay.tickMoveCounter");
    // .tickMoveCounter should start with: ld a, [hl] ($7E) / and a ($A7) / jr z ($28)
    assert_eq!(rom(&mut h, tick), 0x7E, "ld a, [hl] at .tickMoveCounter");
    assert_eq!(rom(&mut h, tick + 1), 0xA7, "and a after ld a, [hl]");
    assert_eq!(
        rom(&mut h, tick + 2),
        0x28,
        "jr z opcode (skip to .moving when delay is 0)"
    );
}

#[test]
fn delay_jr_z_targets_moving() {
    let mut h = rom_harness();
    let tick = sym_addr("UpdateSpriteMovementDelay.tickMoveCounter");
    let moving = sym_addr("UpdateSpriteMovementDelay.moving");
    // jr z is at tick+2 (opcode) / tick+3 (offset), PC after = tick+4
    let jr_offset = rom(&mut h, tick + 3) as i8;
    let jr_pc = tick + 4; // PC after reading the jr instruction
    let target = (jr_pc as i32 + jr_offset as i32) as u16;
    assert_eq!(target, moving, "jr z should target .moving");
}

#[test]
fn delay_dec_and_jr_nz_follow_fix() {
    let mut h = rom_harness();
    let tick = sym_addr("UpdateSpriteMovementDelay.tickMoveCounter");
    // After the fix (3 bytes: $7E $A7 $28 offset), the original dec [hl] / jr nz should follow
    // dec [hl] at tick+4, jr nz at tick+5
    assert_eq!(rom(&mut h, tick + 4), 0x35, "dec [hl] after zero check");
    assert_eq!(rom(&mut h, tick + 5), 0x20, "jr nz after dec [hl]");
}
