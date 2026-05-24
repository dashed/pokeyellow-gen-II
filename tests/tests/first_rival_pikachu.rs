//! ROM byte tests for the first rival battle Pikachu animation fix.
//!
//! Bug: In the first battle against Blue in Oak's Lab, Pikachu was just
//! received from a Poké Ball but enters using the walking Pokémon slide-in
//! animation.  `IsThisPartyMonStarterPikachu` only checks species/OT
//! identity, not whether Pikachu has escaped its Poké Ball yet.
//!
//! Fix: After `IsThisPartyMonStarterPikachu` returns carry, check
//! `wPikachuOverworldStateFlags` bit 3 (sprite drawing disabled = still
//! in Poké Ball).  If set, fall through to the normal Poké Ball send-out
//! animation.
//!
//! Reference: <https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Entering_the_first_battle_against_the_rival>

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

// ─── Opcode constants ────────────────────────────────────────────────

const JR_NC: u8 = 0x30; // jr nc, n
const JR_Z: u8 = 0x28; // jr z, n
const LD_A_NN: u8 = 0xFA; // ld a, [nn]
const BIT_3_A: u8 = 0x5F; // bit 3, a (CB prefix)
const CB_PREFIX: u8 = 0xCB;

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn send_out_mon_in_bank_0f() {
    assert_eq!(sym_bank("SendOutMon"), 0x0F);
}

#[test]
fn jr_nc_before_pokeball_check() {
    // After callfar IsThisPartyMonStarterPikachu, the fix changes
    // `jr c, .starterPikachu` to `jr nc, .notStarterPikachu`
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_starter = sym_addr("SendOutMon.notStarterPikachu");

    // The jr nc should be 7 bytes before .notStarterPikachu
    // (jr nc 2 + ld a,[nn] 3 + bit 3,a 2 + jr z 2 = 9 bytes between jr nc and label)
    // Actually: jr nc is AT some address, .notStarterPikachu is the target.
    // Let me just scan backward from .notStarterPikachu for the jr nc.
    // The jr nc is right after the callfar (6 bytes), which is right after
    // the `res USING_TRAPPING_MOVE, [hl]` instruction.

    // The jr nc should target .notStarterPikachu
    // Search for jr nc in the 10 bytes before .notStarterPikachu
    let mut found = false;
    for offset in 2..=10 {
        let addr = not_starter - offset;
        if rom(&mut h, addr) == JR_NC {
            let operand = rom(&mut h, addr + 1) as i8;
            let target = (addr as i32 + 2 + operand as i32) as u16;
            if target == not_starter {
                found = true;
                break;
            }
        }
    }
    assert!(
        found,
        "Expected `jr nc, .notStarterPikachu` before the overworld flags check"
    );
}

#[test]
fn overworld_flags_check_present() {
    // Between jr nc and .notStarterPikachu, the fix adds:
    //   ld a, [wPikachuOverworldStateFlags]  ($FA lo hi)
    //   bit 3, a                             ($CB $5F)
    //   jr z, .starterPikachu                ($28 nn)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_starter = sym_addr("SendOutMon.notStarterPikachu");
    // The ld a, [nn] should be 7 bytes before .notStarterPikachu
    // (ld a,[nn] 3 + CB prefix 1 + bit 3,a 1 + jr z 2 = 7)
    let ld_a_addr = not_starter - 7;

    assert_eq!(
        rom(&mut h, ld_a_addr),
        LD_A_NN,
        "Expected `ld a, [nn]` for wPikachuOverworldStateFlags"
    );

    // Check the operand is wPikachuOverworldStateFlags ($D42F)
    let lo = rom(&mut h, ld_a_addr + 1);
    let hi = rom(&mut h, ld_a_addr + 2);
    let addr_val = (hi as u16) << 8 | lo as u16;
    assert_eq!(
        addr_val, 0xD42F,
        "Expected wPikachuOverworldStateFlags ($D42F), got {:#06X}",
        addr_val
    );

    // Check bit 3, a
    assert_eq!(rom(&mut h, ld_a_addr + 3), CB_PREFIX, "Expected CB prefix");
    assert_eq!(rom(&mut h, ld_a_addr + 4), BIT_3_A, "Expected `bit 3, a`");

    // Check jr z
    assert_eq!(rom(&mut h, ld_a_addr + 5), JR_Z, "Expected `jr z`");
}

#[test]
fn jr_z_targets_starter_pikachu() {
    // The jr z should target .starterPikachu (the slide animation)
    let mut h = TestHarness::new();
    h.select_rom_bank(0x0F);

    let not_starter = sym_addr("SendOutMon.notStarterPikachu");
    let starter = sym_addr("SendOutMon.starterPikachu");

    let jr_z_addr = not_starter - 2; // jr z is 2 bytes before .notStarterPikachu
    assert_eq!(rom(&mut h, jr_z_addr), JR_Z, "Expected `jr z`");

    let operand = rom(&mut h, jr_z_addr + 1) as i8;
    let target = (jr_z_addr as i32 + 2 + operand as i32) as u16;
    assert_eq!(
        target, starter,
        "jr z should target .starterPikachu ({:#06X}), got {:#06X}",
        starter, target
    );
}

#[test]
fn label_ordering_correct() {
    // .notStarterPikachu should come before .starterPikachu
    let not_starter = sym_addr("SendOutMon.notStarterPikachu");
    let starter = sym_addr("SendOutMon.starterPikachu");
    assert!(
        not_starter < starter,
        ".notStarterPikachu ({:#06X}) should be before .starterPikachu ({:#06X})",
        not_starter,
        starter
    );
}
