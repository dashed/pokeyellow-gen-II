//! ROM byte tests for the healthy party deposit fix.
//!
//! Bug: `BillsPCDeposit` only checks `wPartyCount > 1` before allowing
//! deposit.  It never verifies that remaining party members have HP > 0.
//! Players can deposit all healthy Pokémon, leaving only fainted ones,
//! causing an immediate blackout on the next battle or after 1 step
//! (Yellow) / 4 steps (Red/Blue).
//!
//! Fix: After the user selects "Deposit" from the PC submenu, call
//! `CheckDepositAllowedByHP` which iterates party HP values (skipping
//! the selected mon at `wWhichPokemon`).  If all other mons have HP = 0,
//! the deposit is blocked and `CantDepositLastMonText` is shown.
//!
//! References:
//!   - <https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_(Generation_I)>
//!     (section: "Pokémon Storage System healthy party deposit")

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

fn rom(h: &mut TestHarness, addr: u16) -> u8 {
    h.read_mem(addr)
}

fn banked_harness(label: &str) -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.select_rom_bank(sym_bank(label));
    h
}

// ─── Structural: bank and label checks ──────────────────────────────

#[test]
fn check_deposit_in_same_bank_as_bills_pc() {
    let deposit_bank = sym_bank("BillsPCDeposit");
    let check_bank = sym_bank("CheckDepositAllowedByHP");
    assert_eq!(
        deposit_bank, check_bank,
        "CheckDepositAllowedByHP must be in same bank as BillsPCDeposit (bank ${:02X})",
        deposit_bank
    );
}

#[test]
fn check_deposit_in_bank_08() {
    assert_eq!(
        sym_bank("CheckDepositAllowedByHP"),
        0x08,
        "CheckDepositAllowedByHP should be in bank $08"
    );
}

// ─── THE FIX: call site in BillsPCDeposit ───────────────────────────

#[test]
fn bills_pc_deposit_calls_check_deposit() {
    let mut h = banked_harness("BillsPCDeposit");
    let deposit = sym_addr("BillsPCDeposit");
    let check_fn = sym_addr("CheckDepositAllowedByHP");
    let check_lo = (check_fn & 0xFF) as u8;
    let check_hi = (check_fn >> 8) as u8;

    // Search for `call CheckDepositAllowedByHP` ($CD lo hi) in BillsPCDeposit
    let end = deposit + 100; // search within first 100 bytes of the routine
    let mut found = false;
    for addr in deposit..end {
        if rom(&mut h, addr) == 0xCD
            && rom(&mut h, addr + 1) == check_lo
            && rom(&mut h, addr + 2) == check_hi
        {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "BillsPCDeposit should contain call CheckDepositAllowedByHP"
    );
}

// ─── THE SUBROUTINE: CheckDepositAllowedByHP structure ──────────────

#[test]
fn check_deposit_reads_party_count() {
    let mut h = banked_harness("CheckDepositAllowedByHP");
    let base = sym_addr("CheckDepositAllowedByHP");

    // First instructions: ld a, [wPartyCount] ($FA lo hi)
    assert_eq!(
        rom(&mut h, base),
        0xFA,
        "Expected ld a, [nn] ($FA) at CheckDepositAllowedByHP"
    );
}

#[test]
fn check_deposit_reads_which_pokemon_in_loop() {
    let mut h = banked_harness("CheckDepositAllowedByHP");
    let loop_start = sym_addr("CheckDepositAllowedByHP.checkLoop");
    let found_healthy = sym_addr("CheckDepositAllowedByHP.foundHealthy");

    // The loop should read wWhichPokemon ($FA lo hi) to skip the selected mon
    let mut found = false;
    for addr in loop_start..found_healthy {
        if rom(&mut h, addr) == 0xFA {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Loop should read wWhichPokemon (ld a, [nn] = $FA) to skip the deposited mon"
    );
}

#[test]
fn check_deposit_returns_scf_when_all_fainted() {
    let mut h = banked_harness("CheckDepositAllowedByHP");
    let skip_mon = sym_addr("CheckDepositAllowedByHP.skipMon");
    let found_healthy = sym_addr("CheckDepositAllowedByHP.foundHealthy");

    // Between .skipMon and .foundHealthy, there should be scf ($37) + ret ($C9)
    let mut found_scf_ret = false;
    for addr in skip_mon..found_healthy {
        if rom(&mut h, addr) == 0x37 && rom(&mut h, addr + 1) == 0xC9 {
            found_scf_ret = true;
            break;
        }
    }
    assert!(
        found_scf_ret,
        "Should have scf ($37) + ret ($C9) for 'all fainted' return path"
    );
}

#[test]
fn check_deposit_returns_clear_carry_when_healthy() {
    let mut h = banked_harness("CheckDepositAllowedByHP");
    let found_healthy = sym_addr("CheckDepositAllowedByHP.foundHealthy");

    // .foundHealthy should be: and a ($A7) + ret ($C9)
    assert_eq!(
        rom(&mut h, found_healthy),
        0xA7,
        "Expected and a ($A7) at .foundHealthy to clear carry"
    );
    assert_eq!(
        rom(&mut h, found_healthy + 1),
        0xC9,
        "Expected ret ($C9) after and a"
    );
}

// ─── Cross-reference: original party count check still exists ───────

#[test]
fn original_party_count_check_still_present() {
    let mut h = banked_harness("BillsPCDeposit");
    let base = sym_addr("BillsPCDeposit");

    // The original check: ld a, [wPartyCount] ($FA) / dec a ($3D) / jr nz ($20)
    assert_eq!(
        rom(&mut h, base),
        0xFA,
        "Expected ld a, [nn] at BillsPCDeposit start"
    );
    // After the 3-byte ld a, [nn]: dec a ($3D)
    assert_eq!(
        rom(&mut h, base + 3),
        0x3D,
        "Expected dec a ($3D) after ld a, [wPartyCount]"
    );
    // Then jr nz ($20)
    assert_eq!(
        rom(&mut h, base + 4),
        0x20,
        "Expected jr nz ($20) — original party count > 1 check"
    );
}
