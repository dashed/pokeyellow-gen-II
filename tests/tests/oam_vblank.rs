//! ROM byte tests for the OAM VBlank interruption fix.
//!
//! Bug: `UpdateSprites` calls the banked `_UpdateSprites` to build the OAM
//! buffer in WRAM. If VBlank fires mid-update, `hDMARoutine` copies the
//! half-built buffer to OAM hardware, causing sprite flickering/corruption.
//!
//! Fix: Add an `hOAMUpdateLocked` flag ($FFD9) that `UpdateSprites` sets
//! before calling `_UpdateSprites` and clears after. The VBlank handler
//! checks this flag and skips `hDMARoutine` when it is nonzero.
//! +5 bytes in `home/update_sprites.asm`, +5 bytes in `home/vblank.asm`.

use pokeyellow_tests::{sym_addr, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

const H_OAM_UPDATE_LOCKED_LO: u8 = 0xD9; // low byte of $FFD9

// ─── Structural: UpdateSprites (HOME) ────────────────────────────────

#[test]
fn update_sprites_in_home_bank() {
    // UpdateSprites must reside in HOME (bank 0) since it is called
    // from the overworld loop and other non-banked contexts.
    let addr = sym_addr("UpdateSprites");
    assert!(
        addr < 0x4000,
        "UpdateSprites ({:#06X}) should be in HOME (< $4000)",
        addr
    );
}

#[test]
fn lock_set_before_update_sprites_call() {
    // After `ld [wUpdateSpritesEnabled], a` (with A=$FF), there should be
    // `ldh [hOAMUpdateLocked], a` ($E0 $D9) before `call _UpdateSprites`.
    let mut h = TestHarness::new_headless();
    let base = sym_addr("UpdateSprites");
    let end = base + 30;

    // Find `ldh [hOAMUpdateLocked], a` → $E0 $D9
    let mut lock_pos = None;
    // Find `call _UpdateSprites`
    let update_addr = sym_addr("_UpdateSprites");
    let update_lo = (update_addr & 0xFF) as u8;
    let update_hi = (update_addr >> 8) as u8;
    let mut call_pos = None;

    for addr in base..end {
        if rom(&mut h, addr) == 0xE0 && rom(&mut h, addr + 1) == H_OAM_UPDATE_LOCKED_LO {
            if lock_pos.is_none() {
                lock_pos = Some(addr);
            }
        }
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == update_lo
            && rom(&mut h, addr + 2) == update_hi
        {
            call_pos = Some(addr);
        }
    }

    assert!(lock_pos.is_some(), "ldh [hOAMUpdateLocked], a not found");
    assert!(call_pos.is_some(), "call _UpdateSprites not found");
    assert!(
        lock_pos.unwrap() < call_pos.unwrap(),
        "lock ({:#06X}) must come before call _UpdateSprites ({:#06X})",
        lock_pos.unwrap(),
        call_pos.unwrap()
    );
}

#[test]
fn lock_uses_nonzero_value() {
    // The lock is set by reusing A=$FF from `ld a, $ff / ld [wUpdateSpritesEnabled], a`.
    // Verify the `ldh [hOAMUpdateLocked], a` is preceded by a store of $FF to
    // wUpdateSpritesEnabled, meaning A is $FF (nonzero) when the lock is written.
    let mut h = TestHarness::new_headless();
    let base = sym_addr("UpdateSprites");
    let end = base + 30;

    for addr in base..end {
        if rom(&mut h, addr) == 0xE0 && rom(&mut h, addr + 1) == H_OAM_UPDATE_LOCKED_LO {
            // The preceding instruction should be `ld [wUpdateSpritesEnabled], a` ($EA lo hi)
            // which is 3 bytes. Before that: `ld a, $ff` ($3E $FF) which is 2 bytes.
            // So at addr-5 we should see $3E $FF.
            assert_eq!(
                rom(&mut h, addr - 5),
                0x3E,
                "expected ld a, $FF ($3E) 5 bytes before ldh at {:#06X}",
                addr
            );
            assert_eq!(
                rom(&mut h, addr - 4),
                0xFF,
                "expected $FF operand 4 bytes before ldh at {:#06X}",
                addr
            );
            return;
        }
    }
    panic!("ldh [hOAMUpdateLocked], a not found in UpdateSprites");
}

#[test]
fn unlock_after_update_sprites_call() {
    // After `call _UpdateSprites`, there should be `xor a` ($AF) followed
    // by `ldh [hOAMUpdateLocked], a` ($E0 $D9) to clear the lock.
    let mut h = TestHarness::new_headless();
    let base = sym_addr("UpdateSprites");
    let end = base + 30;

    let update_addr = sym_addr("_UpdateSprites");
    let update_lo = (update_addr & 0xFF) as u8;
    let update_hi = (update_addr >> 8) as u8;

    for addr in base..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == update_lo
            && rom(&mut h, addr + 2) == update_hi
        {
            // After the 3-byte call: xor a ($AF) + ldh [hOAMUpdateLocked], a ($E0 $D9)
            let after = addr + 3;
            assert_eq!(
                rom(&mut h, after),
                0xAF,
                "expected xor a ($AF) after call _UpdateSprites at {:#06X}",
                after
            );
            assert_eq!(
                rom(&mut h, after + 1),
                0xE0,
                "expected ldh opcode ($E0) at {:#06X}",
                after + 1
            );
            assert_eq!(
                rom(&mut h, after + 2),
                H_OAM_UPDATE_LOCKED_LO,
                "expected hOAMUpdateLocked ($D9) at {:#06X}",
                after + 2
            );
            return;
        }
    }
    panic!("call _UpdateSprites not found in UpdateSprites");
}

// ─── Structural: VBlank handler (HOME) ──────────────────────────────

#[test]
fn vblank_in_home_bank() {
    let addr = sym_addr("VBlank");
    assert!(
        addr < 0x4000,
        "VBlank ({:#06X}) should be in HOME (< $4000)",
        addr
    );
}

#[test]
fn vblank_checks_lock_before_dma() {
    // Before `call hDMARoutine`, VBlank should have:
    //   ldh a, [hOAMUpdateLocked]  ($F0 $D9)
    //   and a                       ($A7)
    //   jr nz, .skipOAM            ($20 xx)
    //   call hDMARoutine           ($CD $80 $FF)
    let mut h = TestHarness::new_headless();
    let base = sym_addr("VBlank");
    let end = base + 80;

    for addr in base..end {
        if rom(&mut h, addr) == 0xF0 && rom(&mut h, addr + 1) == H_OAM_UPDATE_LOCKED_LO {
            assert_eq!(
                rom(&mut h, addr + 2),
                0xA7,
                "expected and a ($A7) after ldh a, [hOAMUpdateLocked] at {:#06X}",
                addr + 2
            );
            assert_eq!(
                rom(&mut h, addr + 3),
                0x20,
                "expected jr nz ($20) at {:#06X}",
                addr + 3
            );
            // The jr nz target should skip exactly the 3-byte `call hDMARoutine`
            let offset = rom(&mut h, addr + 4) as i8;
            assert_eq!(
                offset, 3,
                "jr nz should skip 3 bytes (call hDMARoutine), got offset {}",
                offset
            );
            return;
        }
    }
    panic!("ldh a, [hOAMUpdateLocked] not found in VBlank handler");
}

#[test]
fn vblank_skips_only_dma_not_prepare_oam() {
    // The .skipOAM label should land right at `ld a, BANK(PrepareOAMData)`,
    // meaning PrepareOAMData is NOT skipped — only hDMARoutine is.
    let mut h = TestHarness::new_headless();
    let base = sym_addr("VBlank");
    let end = base + 80;

    for addr in base..end {
        // Find `call hDMARoutine` ($CD $80 $FF)
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == 0x80
            && rom(&mut h, addr + 2) == 0xFF
        {
            // The byte after `call hDMARoutine` should be `ld a, N` ($3E)
            // which is the start of `ld a, BANK(PrepareOAMData)`
            assert_eq!(
                rom(&mut h, addr + 3),
                0x3E,
                "expected ld a, BANK(PrepareOAMData) ($3E) after call hDMARoutine at {:#06X}",
                addr + 3
            );
            return;
        }
    }
    panic!("call hDMARoutine ($CD $80 $FF) not found in VBlank handler");
}

#[test]
fn no_dma_call_without_lock_check() {
    // Negative test: verify there is no second `call hDMARoutine` in VBlank
    // that bypasses the lock check (i.e. only one DMA call site exists).
    let mut h = TestHarness::new_headless();
    let base = sym_addr("VBlank");
    let end = sym_addr("DelayFrame"); // VBlank ends before DelayFrame

    let mut dma_count = 0;
    let mut addr = base;
    while addr < end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == 0x80
            && rom(&mut h, addr + 2) == 0xFF
        {
            dma_count += 1;
            addr += 3;
        } else {
            addr += 1;
        }
    }
    assert_eq!(
        dma_count, 1,
        "expected exactly 1 call to hDMARoutine in VBlank, found {}",
        dma_count
    );
}
