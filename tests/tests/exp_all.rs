//! Emulator-based tests for the Exp. All experience distribution fix.
//!
//! The bug: `DivideExpDataByNumMonsGainingExp` (called inside `GainExperience`)
//! divides `wEnemyMonBaseStats` in place. The first `GainExperience` call (for
//! battle participants) leaves the stats divided by the participant count.
//! The second call (Exp. All distribution to all party members) then receives
//! `(base/2/numFighters)` instead of the correct `(base/2)`.
//!
//! The fix: Before the first `GainExperience` call, count battle participants.
//! After the call returns, multiply each of the 7 `wEnemyMonBaseStats` bytes
//! back by the participant count to approximately restore the halved values.
//! +51 bytes in bank $0F.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// EXP_ALL item ID ($4B).
const EXP_ALL: u8 = 0x4B;
/// NUM_STATS + 2 = 7 (5 base stats + base exp high + base exp low).
const NUM_BASE_STAT_BYTES: u8 = 7;

/// WRAM trap for halting execution.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_has_exp_all_branch_exists() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    let bank = sym_bank("FaintEnemyPokemon");
    h.select_rom_bank(bank);

    let playermonnotfaint = sym_addr("FaintEnemyPokemon.playermonnotfaint");

    // Before .hasExpAll, there should be `jr nz, .hasExpAll` somewhere
    // The code is: ld b, EXP_ALL / call IsItemInBag / jr nz, .hasExpAll
    // The `jr nz` at the address just before the no-Exp-All path
    // Let's verify the ld b, EXP_ALL instruction
    // Starting from PLAYERMONNOTFAINT, scan forward for ld b, $4B
    let mut found = false;
    for offset in 0u16..40 {
        let addr = playermonnotfaint + offset;
        let op = h.read_mem(addr);
        let imm = h.read_mem(addr + 1);
        if op == 0x06 && imm == EXP_ALL {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "Expected `ld b, EXP_ALL ($4B)` within 40 bytes of .playermonnotfaint"
    );
}

#[test]
fn rom_bytes_halve_loop_count_is_7() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("FaintEnemyPokemon"));

    let has_exp_all = sym_addr("FaintEnemyPokemon.hasExpAll");

    // .hasExpAll starts with: ld hl, wEnemyMonBaseStats / ld b, NUM_STATS+2
    // The ld b, 7 should be 2 bytes after hasExpAll (ld hl = 3 bytes)
    let ld_hl = h.read_mem(has_exp_all);
    assert_eq!(ld_hl, 0x21, "Expected ld hl,imm16 ($21) at .hasExpAll");

    let ld_b = h.read_mem(has_exp_all + 3);
    let count = h.read_mem(has_exp_all + 4);
    assert_eq!(ld_b, 0x06, "Expected ld b,imm8 ($06) for loop count");
    assert_eq!(
        count, NUM_BASE_STAT_BYTES,
        "Expected loop count = {NUM_BASE_STAT_BYTES}, got {count}"
    );
}

#[test]
fn rom_bytes_halve_loop_uses_srl_hl() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("FaintEnemyPokemon"));

    let halve_exp_data_loop = sym_addr("FaintEnemyPokemon.halveExpDataLoop");

    // .halveExpDataLoop should start with `srl [hl]` = CB 3E
    let prefix = h.read_mem(halve_exp_data_loop);
    let opcode = h.read_mem(halve_exp_data_loop + 1);
    assert_eq!(
        (prefix, opcode),
        (0xCB, 0x3E),
        "Expected srl [hl] (CB 3E) at .halveExpDataLoop, got ({prefix:02X} {opcode:02X})"
    );
}

#[test]
fn rom_bytes_count_loop_reads_party_gain_exp_flags() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("FaintEnemyPokemon"));

    let count_participants_loop = sym_addr("FaintEnemyPokemon.countParticipantsLoop");

    // Before .countParticipantsLoop: `ld a, [wPartyGainExpFlags]` = FA 57 D0
    // This should be a few bytes before COUNT_PARTICIPANTS_LOOP
    // The pattern is: ld a, [wPartyGainExpFlags] / ld b, a / xor a / ld c, 8 / ld d, 0
    // That's 3 + 1 + 1 + 2 + 2 = 9 bytes before the loop
    let start = count_participants_loop - 9;
    let op = h.read_mem(start);
    let lo = h.read_mem(start + 1);
    let hi = h.read_mem(start + 2);
    assert_eq!(op, 0xFA, "Expected ld a,[a16] ($FA)");
    let addr = (hi as u16) << 8 | lo as u16;
    let w_party_gain_exp_flags = sym_addr("wPartyGainExpFlags");
    assert_eq!(
        addr, w_party_gain_exp_flags,
        "Expected wPartyGainExpFlags (${w_party_gain_exp_flags:04X}), got ${addr:04X}"
    );
}

#[test]
fn rom_bytes_multiply_cp_2_skip() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("FaintEnemyPokemon"));

    let multiply_base_stats_loop = sym_addr("FaintEnemyPokemon.multiplyBaseStatsLoop");
    let skip_multiply = sym_addr("FaintEnemyPokemon.skipMultiply");

    // After `pop af`, the code does: cp 2 / jr c, .skipMultiply
    // The `cp 2` should be a few bytes before MULTIPLY_BASE_STATS_LOOP
    // Pattern: pop af (F1) / cp 2 (FE 02) / jr c (38 xx) / ld b, a (47) / ld hl, ...
    // Let's search backwards from MULTIPLY_BASE_STATS_LOOP
    let mut found_cp2 = false;
    for offset in 2u16..12 {
        let addr = multiply_base_stats_loop - offset;
        let op = h.read_mem(addr);
        let imm = h.read_mem(addr + 1);
        if op == 0xFE && imm == 0x02 {
            // Verify jr c follows
            let jr_op = h.read_mem(addr + 2);
            assert_eq!(jr_op, 0x38, "Expected jr c ($38) after cp 2");
            // Verify jr target is .skipMultiply
            let jr_offset = h.read_mem(addr + 3) as i8;
            let target = (addr + 4).wrapping_add(jr_offset as u16);
            assert_eq!(
                target, skip_multiply,
                "jr c should target .skipMultiply (${skip_multiply:04X}), got ${target:04X}"
            );
            found_cp2 = true;
            break;
        }
    }
    assert!(
        found_cp2,
        "Expected `cp 2 / jr c, .skipMultiply` before .multiplyBaseStatsLoop"
    );
}

#[test]
fn rom_bytes_multiply_inner_loop_add_d() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("FaintEnemyPokemon"));

    let multiply_inner_loop = sym_addr("FaintEnemyPokemon.multiplyInnerLoop");

    // .multiplyInnerLoop: add d ($82) / dec e ($1D) / jr nz ($20)
    let add_d = h.read_mem(multiply_inner_loop);
    let dec_e = h.read_mem(multiply_inner_loop + 1);
    let jr_nz = h.read_mem(multiply_inner_loop + 2);
    assert_eq!(add_d, 0x82, "Expected add d ($82) at .multiplyInnerLoop");
    assert_eq!(dec_e, 0x1D, "Expected dec e ($1D)");
    assert_eq!(jr_nz, 0x20, "Expected jr nz ($20)");

    // jr nz should loop back to .multiplyInnerLoop itself
    let offset = h.read_mem(multiply_inner_loop + 3) as i8;
    let target = (multiply_inner_loop + 4).wrapping_add(offset as u16);
    assert_eq!(
        target, multiply_inner_loop,
        "jr nz should loop back to .multiplyInnerLoop"
    );
}

// ─── Behavioral: multiply restores halved values ──────────────────

fn setup_behavioral() -> TestHarness {
    let bank = sym_bank("FaintEnemyPokemon");
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    // Set up trap
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h
}

#[test]
fn multiply_restores_halved_values_2_participants() {
    let mut h = setup_behavioral();
    let w_enemy_mon_base_stats = sym_addr("wEnemyMonBaseStats");

    let multiply_base_stats_loop = sym_addr("FaintEnemyPokemon.multiplyBaseStatsLoop");
    let skip_multiply = sym_addr("FaintEnemyPokemon.skipMultiply");

    // Set up: 7 bytes already halved (simulating post-GainExperience state)
    // Original halved values: [50, 40, 30, 20, 10, 100, 80]
    // After DivideExpDataByNumMonsGainingExp with 2 participants: [25, 20, 15, 10, 5, 50, 40]
    let divided_values: [u8; 7] = [25, 20, 15, 10, 5, 50, 40];
    let expected_after_multiply: [u8; 7] = [50, 40, 30, 20, 10, 100, 80];
    for (i, &v) in divided_values.iter().enumerate() {
        h.write_mem(w_enemy_mon_base_stats + i as u16, v);
    }

    // Set up the multiply loop: B = participant count, HL = wEnemyMonBaseStats, C = 7
    // We'll run from MULTIPLY_BASE_STATS_LOOP to SKIP_MULTIPLY
    h.set_pc(multiply_base_stats_loop);
    h.set_sp(0xDFF0);
    // Set registers: B = 2 (participant count), C = 7 (NUM_STATS+2)
    h.gb.cpu().set_bc(0x0207); // B=2, C=7
    h.gb.cpu().set_hl(w_enemy_mon_base_stats);

    // Patch SKIP_MULTIPLY to STOP
    let saved_byte = h.read_mem(skip_multiply);
    h.write_mem(skip_multiply, STOP);

    h.step_to(skip_multiply);

    // Restore
    h.write_mem(skip_multiply, saved_byte);

    for (i, &expected) in expected_after_multiply.iter().enumerate() {
        let actual = h.read_mem(w_enemy_mon_base_stats + i as u16);
        assert_eq!(
            actual, expected,
            "wEnemyMonBaseStats[{i}]: expected {expected}, got {actual} (2 participants)"
        );
    }
}

#[test]
fn multiply_restores_halved_values_3_participants() {
    let mut h = setup_behavioral();
    let w_enemy_mon_base_stats = sym_addr("wEnemyMonBaseStats");

    let multiply_base_stats_loop = sym_addr("FaintEnemyPokemon.multiplyBaseStatsLoop");
    let skip_multiply = sym_addr("FaintEnemyPokemon.skipMultiply");

    // After DivideExpDataByNumMonsGainingExp with 3 participants:
    // halved = [90, 60, 30], divided by 3 = [30, 20, 10]
    // multiply by 3 should restore to [90, 60, 30]
    let divided_values: [u8; 7] = [30, 20, 10, 8, 4, 42, 17];
    let expected: [u8; 7] = [90, 60, 30, 24, 12, 126, 51];
    for (i, &v) in divided_values.iter().enumerate() {
        h.write_mem(w_enemy_mon_base_stats + i as u16, v);
    }

    h.set_pc(multiply_base_stats_loop);
    h.set_sp(0xDFF0);
    h.gb.cpu().set_bc(0x0307); // B=3, C=7
    h.gb.cpu().set_hl(w_enemy_mon_base_stats);

    let saved_byte = h.read_mem(skip_multiply);
    h.write_mem(skip_multiply, STOP);
    h.step_to(skip_multiply);
    h.write_mem(skip_multiply, saved_byte);

    for (i, &exp) in expected.iter().enumerate() {
        let actual = h.read_mem(w_enemy_mon_base_stats + i as u16);
        assert_eq!(
            actual, exp,
            "wEnemyMonBaseStats[{i}]: expected {exp}, got {actual} (3 participants)"
        );
    }
}

#[test]
fn count_participants_popcount() {
    let mut h = setup_behavioral();

    let count_participants_loop = sym_addr("FaintEnemyPokemon.countParticipantsLoop");

    // Test the participant counting loop.
    // wPartyGainExpFlags = 0b00010101 = 3 participants (bits 0, 2, 4 set)
    h.write_mem(sym_addr("wPartyGainExpFlags"), 0b00010101);

    // Run the counting loop. It starts by loading wPartyGainExpFlags into B,
    // then counts bits. We run from the ld a, [wPartyGainExpFlags] instruction.
    // That's 9 bytes before COUNT_PARTICIPANTS_LOOP.
    let count_start = count_participants_loop - 9;

    h.set_pc(count_start);
    h.set_sp(0xDFF0);

    // The counting ends with `push af` which saves the count.
    // After push af, execution continues with `xor a / ld [wBoostExpByExpAll], a / callfar GainExperience`
    // We want to stop at the push af + 1 instruction
    // push af is at COUNT_PARTICIPANTS_LOOP + loop_body_size... let me find it
    // The loop is: xor a / srl b / adc d / ld d,a / dec c / jr nz
    // That's 1 + 2 + 1 + 1 + 1 + 2 = 8 bytes
    // After the loop falls through, next is: push af
    let push_af_addr = count_participants_loop + 8;
    let push_af_op = h.read_mem(push_af_addr);
    assert_eq!(push_af_op, 0xF5, "Expected push af ($F5) after count loop");

    // Patch the byte after push af to STOP
    let after_push = push_af_addr + 1;
    let saved = h.read_mem(after_push);
    h.write_mem(after_push, STOP);

    h.step_to(after_push);
    h.write_mem(after_push, saved);

    // The count should be in A (and on top of stack via push af)
    // Register D holds the accumulated count
    let d = h.gb.cpu_i().de() >> 8;
    assert_eq!(
        d, 3,
        "Expected participant count = 3 for flags 0b00010101, got {d}"
    );
}

#[test]
fn skip_multiply_when_1_participant() {
    let mut h = setup_behavioral();
    let w_enemy_mon_base_stats = sym_addr("wEnemyMonBaseStats");

    let multiply_base_stats_loop = sym_addr("FaintEnemyPokemon.multiplyBaseStatsLoop");
    let skip_multiply = sym_addr("FaintEnemyPokemon.skipMultiply");

    // With only 1 participant, cp 2 / jr c should skip the multiply loop.
    // Set A=1 (as if popped from stack) and run from `cp 2`
    // The cp 2 is 3 bytes before MULTIPLY_BASE_STATS_LOOP (after pop af which is 1 byte)
    // Actually let me find it precisely:
    // Pattern: pop af (F1) / cp 2 (FE 02) / jr c, .skipMultiply (38 xx) / ld b, a (47)
    // So cp 2 is 1 byte after pop af, and 4 bytes before ld b,a at MULTIPLY_BASE_STATS_LOOP - 5

    // Set known values in wEnemyMonBaseStats
    let test_values: [u8; 7] = [42, 37, 19, 88, 5, 200, 127];
    for (i, &v) in test_values.iter().enumerate() {
        h.write_mem(w_enemy_mon_base_stats + i as u16, v);
    }

    // Find cp 2 address
    let mut cp2_addr = 0u16;
    for offset in 2u16..12 {
        let addr = multiply_base_stats_loop - offset;
        if h.read_mem(addr) == 0xFE && h.read_mem(addr + 1) == 0x02 {
            cp2_addr = addr;
            break;
        }
    }
    assert!(cp2_addr != 0, "Could not find cp 2 instruction");

    // Set A = 1 (only 1 participant)
    h.set_a(1);
    h.set_pc(cp2_addr);
    h.set_sp(0xDFF0);

    // Patch SKIP_MULTIPLY to STOP
    let saved = h.read_mem(skip_multiply);
    h.write_mem(skip_multiply, STOP);
    h.step_to(skip_multiply);
    h.write_mem(skip_multiply, saved);

    // Values should be unchanged
    for (i, &expected) in test_values.iter().enumerate() {
        let actual = h.read_mem(w_enemy_mon_base_stats + i as u16);
        assert_eq!(
            actual, expected,
            "wEnemyMonBaseStats[{i}] should be unchanged with 1 participant"
        );
    }
}

#[test]
fn gain_exp_flags_loop_sets_all_party_bits() {
    let mut h = setup_behavioral();

    let gain_exp_flags_loop = sym_addr("FaintEnemyPokemon.gainExpFlagsLoop");

    // Test the .gainExpFlagsLoop which sets wPartyGainExpFlags for all party members.
    // wPartyCount = 4 → flags should become 0b00001111 = $0F
    h.write_mem(sym_addr("wPartyCount"), 4);
    h.write_mem(sym_addr("wPartyGainExpFlags"), 0x00);

    // .skipMultiply loads TRUE into wBoostExpByExpAll then does the flags loop.
    // Let's run from .skipMultiply up to the jpfar at the end.
    // But jpfar calls GainExperience which we can't run. Let me instead
    // run just the flags loop portion.
    h.set_pc(gain_exp_flags_loop);
    h.set_sp(0xDFF0);

    // ld a, [wPartyCount] already loaded into a by the code before the loop
    // At GAIN_EXP_FLAGS_LOOP, code expects: a = wPartyCount, b = 0
    h.set_a(4);
    h.gb.cpu().set_bc(0x0000); // b = 0

    // The loop: scf / rl b / dec a / jr nz
    // After 4 iterations: b = 0b00001111
    // Then: ld a, b / ld [wPartyGainExpFlags], a / jpfar GainExperience
    // jpfar is a macro that does multiple instructions; let's just run a few steps
    // and check the flags value after ld [wPartyGainExpFlags], a

    // Run enough steps for the loop (4 iterations × ~4 instructions each + 2 store instructions)
    for _ in 0..30 {
        let pc = h.gb.cpu_i().pc();
        // Stop if we reach a jpfar or something beyond the store
        if pc > gain_exp_flags_loop + 20 {
            break;
        }
        h.gb.clock();
    }

    let flags = h.read_mem(sym_addr("wPartyGainExpFlags"));
    assert_eq!(
        flags, 0x0F,
        "Expected wPartyGainExpFlags = $0F for 4 party members, got ${flags:02X}"
    );
}

#[test]
fn gain_exp_flags_loop_6_party_members() {
    let mut h = setup_behavioral();

    let gain_exp_flags_loop = sym_addr("FaintEnemyPokemon.gainExpFlagsLoop");

    h.write_mem(sym_addr("wPartyCount"), 6);
    h.write_mem(sym_addr("wPartyGainExpFlags"), 0x00);

    h.set_pc(gain_exp_flags_loop);
    h.set_sp(0xDFF0);
    h.set_a(6);
    h.gb.cpu().set_bc(0x0000);

    for _ in 0..50 {
        let pc = h.gb.cpu_i().pc();
        if pc > gain_exp_flags_loop + 20 {
            break;
        }
        h.gb.clock();
    }

    let flags = h.read_mem(sym_addr("wPartyGainExpFlags"));
    assert_eq!(
        flags, 0x3F,
        "Expected wPartyGainExpFlags = $3F for 6 party members, got ${flags:02X}"
    );
}
