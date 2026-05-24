//! ROM byte tests for the Item Finder coordinate 0 bug fix.
//!
//! Bug: `HiddenItemNear` uses `jr nc, .loop` after the lower-bound
//! coordinate checks (`Sub5ClampTo0` result vs item coordinate).
//! When the clamped lower bound equals the item coordinate (e.g.,
//! both are 0), `cp` clears carry and sets Z, so `jr nc` incorrectly
//! skips the item. Items at X=0 or Y=0 are never detected by the
//! Item Finder when the player is at coordinates 0–5.
//!
//! Fix: Add `jr z` before each `jr nc, .loop` so that equality
//! (item at the exact detection boundary) is treated as "in range".
//! +4 bytes in bank $1D.
//!
//! Note: No vanilla Yellow hidden items are placed at coordinate 0,
//! so this bug is latent but real. It matters for ROM hacks.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn rom_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank("HiddenItemNear"));
    h
}

// ─── Structural tests ──────────────────────────────────────────────

#[test]
fn hidden_item_near_is_in_bank_1d() {
    assert_eq!(
        sym_bank("HiddenItemNear"),
        0x1D,
        "HiddenItemNear should be in bank $1D"
    );
}

#[test]
fn y_lower_bound_has_jr_z_before_jr_nc() {
    let mut h = rom_harness();
    // .checkYUpper label is right after the jr nc instruction.
    // Working backwards: jr nc (2 bytes) + jr z (2 bytes) + cp d (1 byte)
    let check_y = sym_addr("HiddenItemNear.checkYUpper");
    // jr z is at checkYUpper - 4, jr nc is at checkYUpper - 2
    let jr_z_addr = check_y - 4;
    let jr_nc_addr = check_y - 2;
    // cp d = $BA at checkYUpper - 5
    assert_eq!(rom(&mut h, check_y - 5), 0xBA, "cp d opcode before jr z");
    assert_eq!(rom(&mut h, jr_z_addr), 0x28, "jr z opcode ($28)");
    assert_eq!(rom(&mut h, jr_nc_addr), 0x30, "jr nc opcode ($30)");
}

#[test]
fn y_lower_bound_jr_z_skips_jr_nc() {
    let mut h = rom_harness();
    let check_y = sym_addr("HiddenItemNear.checkYUpper");
    // jr z offset is at checkYUpper - 3; it should skip 2 bytes (the jr nc)
    let jr_z_offset = rom(&mut h, check_y - 3);
    assert_eq!(
        jr_z_offset, 0x02,
        "jr z should skip 2 bytes (the jr nc, .loop instruction)"
    );
}

#[test]
fn x_lower_bound_has_jr_z_before_jr_nc() {
    let mut h = rom_harness();
    let check_x = sym_addr("HiddenItemNear.checkXUpper");
    // cp e = $BB at checkXUpper - 5
    assert_eq!(rom(&mut h, check_x - 5), 0xBB, "cp e opcode before jr z");
    assert_eq!(rom(&mut h, check_x - 4), 0x28, "jr z opcode ($28)");
    assert_eq!(rom(&mut h, check_x - 2), 0x30, "jr nc opcode ($30)");
}

#[test]
fn x_lower_bound_jr_z_skips_jr_nc() {
    let mut h = rom_harness();
    let check_x = sym_addr("HiddenItemNear.checkXUpper");
    let jr_z_offset = rom(&mut h, check_x - 3);
    assert_eq!(
        jr_z_offset, 0x02,
        "jr z should skip 2 bytes (the jr nc, .loop instruction)"
    );
}

#[test]
fn sub5_clamp_to_0_structure() {
    let mut h = rom_harness();
    let addr = sym_addr("Sub5ClampTo0");
    // sub 5 → $D6 $05
    assert_eq!(rom(&mut h, addr), 0xD6, "sub n opcode");
    assert_eq!(rom(&mut h, addr + 1), 0x05, "sub 5 immediate");
    // cp $f0 → $FE $F0
    assert_eq!(rom(&mut h, addr + 2), 0xFE, "cp n opcode");
    assert_eq!(rom(&mut h, addr + 3), 0xF0, "cp $F0 threshold");
    // ret c → $D8
    assert_eq!(rom(&mut h, addr + 4), 0xD8, "ret c opcode");
    // xor a → $AF
    assert_eq!(rom(&mut h, addr + 5), 0xAF, "xor a opcode");
    // ret → $C9
    assert_eq!(rom(&mut h, addr + 6), 0xC9, "ret opcode");
}

#[test]
fn y_upper_bound_uses_add_4() {
    let mut h = rom_harness();
    let check_y = sym_addr("HiddenItemNear.checkYUpper");
    // At .checkYUpper: ld a, [wYCoord] (3 bytes) → add 4 (2 bytes) → cp d (1 byte) → jr c (2 bytes)
    // add 4 = $C6 $04 at offset +3
    assert_eq!(rom(&mut h, check_y + 3), 0xC6, "add n opcode");
    assert_eq!(rom(&mut h, check_y + 4), 0x04, "add 4 immediate");
}

#[test]
fn x_upper_bound_uses_add_5() {
    let mut h = rom_harness();
    let check_x = sym_addr("HiddenItemNear.checkXUpper");
    // At .checkXUpper: ld a, [wXCoord] (3 bytes) → add 5 (2 bytes) → cp e (1 byte) → jr c (2 bytes)
    assert_eq!(rom(&mut h, check_x + 3), 0xC6, "add n opcode");
    assert_eq!(rom(&mut h, check_x + 4), 0x05, "add 5 immediate");
}
