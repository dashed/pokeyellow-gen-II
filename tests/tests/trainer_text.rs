//! ROM byte tests for the trainer end battle text 2 fix.
//!
//! Bug: `ReadTrainerHeaderInfo` has a special case for offset `$a` (end battle
//! text 2 / lose text) that reads the pointer into DE instead of HL. However,
//! the `.done` epilogue immediately does `pop de`, destroying the value just
//! read. The caller in `TalkToTrainer.trainerNotYetFought` then uses
//! `push de` / `pop de` around a second call for offset `$8` (win text),
//! passing garbage to `SaveEndBattleTextPointers` as the lose text pointer.
//!
//! Fix (two parts):
//!
//! 1. **`ReadTrainerHeaderInfo`**: Remove the custom DE handler for `cp $a`.
//!    Instead, let it fall through to `.readPointer` (which reads into HL,
//!    same as offsets `$2`/`$4`/`$6`/`$8`). Saves 5 bytes in ROM0.
//!
//! 2. **`TalkToTrainer`**: Replace `push de` / `pop de` with `ld d, h` /
//!    `ld e, l` to copy the lose-text pointer from HL to DE before reading
//!    the win text. Now `SaveEndBattleTextPointers` receives the correct
//!    pointers: HL = win text, DE = lose text.
//!
//! Reference:
//!   - <https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── ReadTrainerHeaderInfo: cp $a falls through to .readPointer ─────

#[test]
fn read_trainer_header_is_in_home() {
    assert_eq!(
        sym_bank("ReadTrainerHeaderInfo"),
        0x00,
        "ReadTrainerHeaderInfo should be in bank $00 (HOME)"
    );
}

#[test]
fn cp_a_offset_routes_to_read_pointer() {
    // The cp $a / jr nz, .done sequence should sit immediately before
    // .readPointer, so that when a == $a the jr nz is NOT taken and
    // execution falls through to .readPointer.
    let mut h = TestHarness::new_headless();

    let read_pointer = sym_addr("ReadTrainerHeaderInfo.readPointer");

    // 4 bytes before .readPointer: cp $a (FE 0A)
    assert_eq!(rom(&mut h, read_pointer - 4), 0xFE, "expected `cp` opcode");
    assert_eq!(
        rom(&mut h, read_pointer - 3),
        0x0A,
        "expected $0A immediate"
    );

    // 2 bytes before .readPointer: jr nz, .done (20 xx)
    assert_eq!(
        rom(&mut h, read_pointer - 2),
        0x20,
        "expected `jr nz` opcode"
    );
}

#[test]
fn cp_a_jr_nz_targets_done() {
    // Verify that the jr nz after cp $a jumps to .done
    let mut h = TestHarness::new_headless();

    let read_pointer = sym_addr("ReadTrainerHeaderInfo.readPointer");
    let done = sym_addr("ReadTrainerHeaderInfo.done");

    // jr nz is at read_pointer - 2, displacement is at read_pointer - 1
    let displacement = rom(&mut h, read_pointer - 1) as i8;
    // jr target = pc_after_jr + displacement = read_pointer + displacement
    let target = (read_pointer as i32 + displacement as i32) as u16;

    assert_eq!(
        target, done,
        "jr nz should target .done (${done:04X}), got ${target:04X}"
    );
}

#[test]
fn no_de_handler_between_jr_nz_and_read_pointer() {
    // After the fix, the jr nz, .done is immediately followed by
    // .readPointer (ld a, [hli] = $2A). There should be no remnant
    // of the old DE handler (ld a,[hli] / ld d,[hl] / ld e,a / jr .done).
    let mut h = TestHarness::new_headless();

    let read_pointer = sym_addr("ReadTrainerHeaderInfo.readPointer");

    // .readPointer should be ld a, [hli] = $2A
    assert_eq!(
        rom(&mut h, read_pointer),
        0x2A,
        "expected `ld a, [hli]` ($2A) at .readPointer"
    );
}

#[test]
fn read_pointer_reads_into_hl() {
    // .readPointer: ld a, [hli] ($2A) / ld h, [hl] ($66) / ld l, a ($6F)
    let mut h = TestHarness::new_headless();

    let rp = sym_addr("ReadTrainerHeaderInfo.readPointer");
    assert_eq!(rom(&mut h, rp), 0x2A, "ld a, [hli]");
    assert_eq!(rom(&mut h, rp + 1), 0x66, "ld h, [hl]");
    assert_eq!(rom(&mut h, rp + 2), 0x6F, "ld l, a");
}

#[test]
fn done_pops_de_and_returns() {
    // .done: pop de ($D1) / ret ($C9)
    let mut h = TestHarness::new_headless();

    let done = sym_addr("ReadTrainerHeaderInfo.done");
    assert_eq!(rom(&mut h, done), 0xD1, "pop de");
    assert_eq!(rom(&mut h, done + 1), 0xC9, "ret");
}

// ─── TalkToTrainer: ld d,h / ld e,l instead of push/pop de ─────────

#[test]
fn talk_to_trainer_is_in_home() {
    assert_eq!(
        sym_bank("TalkToTrainer"),
        0x00,
        "TalkToTrainer should be in bank $00 (HOME)"
    );
}

#[test]
fn talk_to_trainer_copies_hl_to_de_after_lose_text_read() {
    // After `call ReadTrainerHeaderInfo` for offset $a (lose text),
    // the code should do `ld d, h` ($54) / `ld e, l` ($5D) to save
    // the lose text pointer in DE before the win text read overwrites HL.
    let mut h = TestHarness::new_headless();

    let not_fought = sym_addr("TalkToTrainer.trainerNotYetFought");

    // Layout at .trainerNotYetFought:
    //   ld a, $4          (3E 04)       +0
    //   call ReadTrainer  (CD xx xx)    +2
    //   call PrintText    (CD xx xx)    +5
    //   ld a, $a          (3E 0A)       +8
    //   call ReadTrainer  (CD xx xx)    +10
    //   ld d, h           (54)          +13
    //   ld e, l           (5D)          +14
    //   ld a, $8          (3E 08)       +15
    //   call ReadTrainer  (CD xx xx)    +17
    //   call SaveEnd...   (CD xx xx)    +20

    // Verify ld a, $a
    assert_eq!(rom(&mut h, not_fought + 8), 0x3E, "ld a at offset +8");
    assert_eq!(rom(&mut h, not_fought + 9), 0x0A, "immediate $0A");

    // Verify call ReadTrainerHeaderInfo
    assert_eq!(rom(&mut h, not_fought + 10), 0xCD, "call opcode at +10");

    // Verify ld d, h / ld e, l (the fix)
    assert_eq!(
        rom(&mut h, not_fought + 13),
        0x54,
        "expected `ld d, h` ($54) after lose text read"
    );
    assert_eq!(
        rom(&mut h, not_fought + 14),
        0x5D,
        "expected `ld e, l` ($5D) after lose text read"
    );
}

#[test]
fn talk_to_trainer_reads_win_text_after_de_copy() {
    // After ld d,h / ld e,l, should read offset $8 (win text)
    let mut h = TestHarness::new_headless();

    let not_fought = sym_addr("TalkToTrainer.trainerNotYetFought");

    // ld a, $8 at +15
    assert_eq!(rom(&mut h, not_fought + 15), 0x3E, "ld a at +15");
    assert_eq!(
        rom(&mut h, not_fought + 16),
        0x08,
        "immediate $08 (win text)"
    );

    // call ReadTrainerHeaderInfo at +17
    assert_eq!(rom(&mut h, not_fought + 17), 0xCD, "call opcode at +17");
    let target = rom(&mut h, not_fought + 18) as u16 | ((rom(&mut h, not_fought + 19) as u16) << 8);
    assert_eq!(
        target,
        sym_addr("ReadTrainerHeaderInfo"),
        "call target should be ReadTrainerHeaderInfo"
    );
}

#[test]
fn talk_to_trainer_calls_save_end_battle_text_pointers() {
    // After the win text read, should call SaveEndBattleTextPointers
    let mut h = TestHarness::new_headless();

    let not_fought = sym_addr("TalkToTrainer.trainerNotYetFought");

    // call SaveEndBattleTextPointers at +20
    assert_eq!(rom(&mut h, not_fought + 20), 0xCD, "call opcode at +20");
    let target = rom(&mut h, not_fought + 21) as u16 | ((rom(&mut h, not_fought + 22) as u16) << 8);
    assert_eq!(
        target,
        sym_addr("SaveEndBattleTextPointers"),
        "call target should be SaveEndBattleTextPointers"
    );
}
