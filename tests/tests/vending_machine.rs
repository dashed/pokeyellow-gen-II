//! ROM byte tests for the vending machine glitch fix.
//!
//! Bug: The vending machine's money check at Celadon Dept Store hardcodes
//! ¥200 (Fresh Water price) for `HasEnoughMoney`, regardless of which drink
//! is selected. Soda Pop (¥300) and Lemonade (¥350) can be purchased with
//! only ¥200, reducing money to ¥0.
//!
//! Fix: Call `LoadVendingMachineItem` BEFORE the money check to load the
//! actual item price into `hVendingMachinePrice`, then copy it to `hMoney`
//! for `HasEnoughMoney`. The duplicate `LoadVendingMachineItem` call after
//! `.enoughMoney` is removed.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Vending_machine_glitch>

use pokeyellow_tests::{sym_addr, sym_bank};

fn rom() -> Vec<u8> {
    std::fs::read("../pokeyellow.gbc").expect("ROM not found")
}

fn rom_offset(bank: u32, addr: u16) -> usize {
    (bank * 0x4000 + (addr as u32 - 0x4000)) as usize
}

fn at(rom: &[u8], bank: u32, addr: u16) -> u8 {
    rom[rom_offset(bank, addr)]
}

// ─── Structural test ─────────────────────────────────────────────────

#[test]
fn vending_machine_menu_in_bank_1d() {
    assert_eq!(
        sym_bank("VendingMachineMenu"),
        0x1D,
        "VendingMachineMenu should be in bank $1D"
    );
}

// ─── THE FIX: LoadVendingMachineItem before HasEnoughMoney ───────────

#[test]
fn load_vending_machine_item_before_money_check() {
    let rom = rom();
    let bank = sym_bank("VendingMachineMenu") as u32;
    let enough_money = sym_addr("VendingMachineMenu.enoughMoney");

    // Search backwards from .enoughMoney for call LoadVendingMachineItem
    let load_item = sym_addr("LoadVendingMachineItem");
    let load_lo = (load_item & 0xFF) as u8;
    let load_hi = (load_item >> 8) as u8;

    let has_enough = sym_addr("HasEnoughMoney");
    let has_lo = (has_enough & 0xFF) as u8;
    let has_hi = (has_enough >> 8) as u8;

    let mut load_item_addr = None;
    let mut has_enough_addr = None;

    // Scan the area before .enoughMoney
    for i in (enough_money.saturating_sub(40)..enough_money).rev() {
        let b = at(&rom, bank, i);
        if b == 0xCD {
            let lo = at(&rom, bank, i + 1);
            let hi = at(&rom, bank, i + 2);
            if lo == load_lo && hi == load_hi && load_item_addr.is_none() {
                load_item_addr = Some(i);
            }
            if lo == has_lo && hi == has_hi && has_enough_addr.is_none() {
                has_enough_addr = Some(i);
            }
        }
    }

    let load_at =
        load_item_addr.expect("call LoadVendingMachineItem should be before .enoughMoney");
    let has_at = has_enough_addr.expect("call HasEnoughMoney should be before .enoughMoney");

    assert!(
        load_at < has_at,
        "LoadVendingMachineItem (${load_at:04X}) must be called BEFORE \
         HasEnoughMoney (${has_at:04X}) — this is THE FIX"
    );
}

#[test]
fn money_check_uses_vending_machine_price_not_hardcoded() {
    let rom = rom();
    let bank = sym_bank("VendingMachineMenu") as u32;
    let enough_money = sym_addr("VendingMachineMenu.enoughMoney");

    // The fix copies hVendingMachinePrice to hMoney using ldh pairs.
    // Look for ldh a, [hVendingMachinePrice] ($F0 $DC) before HasEnoughMoney
    let mut found_price_load = false;
    for i in (enough_money.saturating_sub(30)..enough_money).rev() {
        // ldh a, [hVendingMachinePrice] = $F0 $DC
        if at(&rom, bank, i) == 0xF0 && at(&rom, bank, i + 1) == 0xDC {
            found_price_load = true;
            break;
        }
    }
    assert!(
        found_price_load,
        "Should load hVendingMachinePrice ($F0 $DC) before HasEnoughMoney, \
         not hardcode ¥200"
    );
}

#[test]
fn no_hardcoded_200_price() {
    let rom = rom();
    let bank = sym_bank("VendingMachineMenu") as u32;
    let enough_money = sym_addr("VendingMachineMenu.enoughMoney");

    // The old buggy code had: xor a / ldh [hMoney], a / ldh [hMoney+2], a /
    // ld a, $2 / ldh [hMoney+1], a — hardcoding BCD $000200.
    // Verify ld a, $2 ($3E $02) is NOT present in the money check area.
    for i in enough_money.saturating_sub(25)..enough_money {
        if at(&rom, bank, i) == 0x3E && at(&rom, bank, i + 1) == 0x02 {
            // Check context: if next is ldh [hMoney+1], a ($E0 $A0), it's the old bug
            if at(&rom, bank, i + 2) == 0xE0 && at(&rom, bank, i + 3) == 0xA0 {
                panic!(
                    "Found hardcoded ld a, $2 / ldh [hMoney+1], a at ${i:04X} — \
                     old vending machine bug still present!"
                );
            }
        }
    }
    // Test passes if no hardcoded ¥200 pattern found
}

// ─── .enoughMoney path ───────────────────────────────────────────────

#[test]
fn enough_money_loads_item_from_hram() {
    let rom = rom();
    let bank = sym_bank("VendingMachineMenu.enoughMoney") as u32;
    let enough = sym_addr("VendingMachineMenu.enoughMoney");

    // .enoughMoney should start with ldh a, [hVendingMachineItem] ($F0 $DB)
    assert_eq!(
        at(&rom, bank, enough),
        0xF0,
        "Expected ldh a, [nn] ($F0) at .enoughMoney"
    );
    assert_eq!(
        at(&rom, bank, enough + 1),
        0xDB,
        "Expected hVendingMachineItem ($DB) — item already loaded by earlier call"
    );
}
