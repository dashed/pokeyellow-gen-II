//! Emulator-based tests for two Ghost battle fixes in bank $0F.
//!
//! Bug 1 (Pokédex seen flag): In Pokémon Tower, wild Pokémon appear as "Ghost"
//! if the player doesn't have the Silph Scope. However, LoadEnemyMonData
//! unconditionally marks the real species as "seen" in the Pokédex, revealing
//! what's behind the Ghost even though the player hasn't identified it.
//! Fix: `call IsGhostBattle / jr z, .skipPokedexSeen` before FlagActionPredef.
//! +5 bytes in bank $0F.
//!
//! Bug 2 (sprite reveal on party menu return): After viewing the party menu or
//! bag during a ghost battle, the sprite reload path loads the real species
//! sprite from wEnemyMonSpecies, visually revealing the Pokémon behind the
//! Ghost. Fix: `call IsGhostBattle / jr nz, .notGhostReload / ld a, MON_GHOST`
//! before the existing GetMonHeader call. +9 bytes in bank $0F.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// MON_GHOST sprite constant ($B8).
const MON_GHOST: u8 = 0xB8;

/// Map constants.
const POKEMON_TOWER_1F: u8 = 0x8E;
const POKEMON_TOWER_3F: u8 = 0x90;
const POKEMON_TOWER_7F: u8 = 0x94;

/// SILPH_SCOPE item ID.
const SILPH_SCOPE: u8 = 0x48;

/// Trap address for ret.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_call_is_ghost_battle_before_pokedex_seen() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("IsGhostBattle"));

    let is_ghost_battle = sym_addr("IsGhostBattle");
    let skip_pokedex_seen = sym_addr("LoadEnemyMonData.skipPokedexSeen");
    let lo = (is_ghost_battle & 0xFF) as u8;
    let hi = (is_ghost_battle >> 8) as u8;

    // Scan backward from .skipPokedexSeen for `call IsGhostBattle` (CD lo hi).
    // Uses scanning instead of hardcoded offset so the test works both on the
    // ghost-battle branch alone and when merged with glitch-safety (which adds
    // extra bytes between the call and the label).
    let scan_start = skip_pokedex_seen.saturating_sub(0x30);
    for addr in scan_start..skip_pokedex_seen {
        if h.read_mem(addr) == 0xCD && h.read_mem(addr + 1) == lo && h.read_mem(addr + 2) == hi {
            return;
        }
    }
    panic!("call IsGhostBattle not found in 0x30 bytes before .skipPokedexSeen");
}

#[test]
fn rom_bytes_jr_z_targets_skip_pokedex_seen() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("IsGhostBattle"));

    let is_ghost_battle = sym_addr("IsGhostBattle");
    let skip_pokedex_seen = sym_addr("LoadEnemyMonData.skipPokedexSeen");
    let lo = (is_ghost_battle & 0xFF) as u8;
    let hi = (is_ghost_battle >> 8) as u8;

    // Find `call IsGhostBattle`, then verify `jr z, .skipPokedexSeen` follows.
    let scan_start = skip_pokedex_seen.saturating_sub(0x30);
    for addr in scan_start..skip_pokedex_seen {
        if h.read_mem(addr) == 0xCD && h.read_mem(addr + 1) == lo && h.read_mem(addr + 2) == hi {
            let jr_op = h.read_mem(addr + 3);
            assert_eq!(
                jr_op, 0x28,
                "Expected jr z ($28) after call IsGhostBattle, got ${jr_op:02X}"
            );
            let offset = h.read_mem(addr + 4) as i8;
            let target = (addr + 5).wrapping_add(offset as u16);
            assert_eq!(
                target, skip_pokedex_seen,
                "jr z should target .skipPokedexSeen (${skip_pokedex_seen:04X}), got ${target:04X}"
            );
            return;
        }
    }
    panic!("call IsGhostBattle not found in 0x30 bytes before .skipPokedexSeen");
}

// ─── Behavioral: IsGhostBattle returns ──────────────────────────────

/// Set up a harness for running IsGhostBattle directly.
fn setup_is_ghost_battle() -> TestHarness {
    let bank = sym_bank("IsGhostBattle");
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    // Set up trap for ret
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    h
}

/// Clear the bag (no items).
fn clear_bag(h: &mut TestHarness) {
    h.write_mem(sym_addr("wNumBagItems"), 0);
    h.write_mem(sym_addr("wBagItems"), 0xFF); // terminator
}

/// Put Silph Scope in the bag.
fn add_silph_scope(h: &mut TestHarness) {
    let w_bag_items = sym_addr("wBagItems");
    h.write_mem(sym_addr("wNumBagItems"), 1);
    h.write_mem(w_bag_items, SILPH_SCOPE); // item ID
    h.write_mem(w_bag_items + 1, 1); // quantity
    h.write_mem(w_bag_items + 2, 0xFF); // terminator
}

#[test]
fn is_ghost_battle_returns_z_in_tower_without_silph_scope() {
    let mut h = setup_is_ghost_battle();
    h.write_mem(sym_addr("wIsInBattle"), 1); // wild battle
    h.write_mem(sym_addr("wCurMap"), POKEMON_TOWER_3F);
    clear_bag(&mut h);

    h.set_pc(sym_addr("IsGhostBattle"));
    h.step_to(TRAP_ADDR);

    assert!(
        h.gb.cpu_i().zero(),
        "IsGhostBattle should return Z (ghost) in Pokémon Tower without Silph Scope"
    );
}

#[test]
fn is_ghost_battle_returns_nz_in_tower_with_silph_scope() {
    let mut h = setup_is_ghost_battle();
    h.write_mem(sym_addr("wIsInBattle"), 1); // wild battle
    h.write_mem(sym_addr("wCurMap"), POKEMON_TOWER_3F);
    add_silph_scope(&mut h);

    h.set_pc(sym_addr("IsGhostBattle"));
    h.step_to(TRAP_ADDR);

    assert!(
        !h.gb.cpu_i().zero(),
        "IsGhostBattle should return NZ (not ghost) in Pokémon Tower with Silph Scope"
    );
}

#[test]
fn is_ghost_battle_returns_nz_for_trainer_battle() {
    let mut h = setup_is_ghost_battle();
    h.write_mem(sym_addr("wIsInBattle"), 2); // trainer battle
    h.write_mem(sym_addr("wCurMap"), POKEMON_TOWER_3F);
    clear_bag(&mut h);

    h.set_pc(sym_addr("IsGhostBattle"));
    h.step_to(TRAP_ADDR);

    assert!(
        !h.gb.cpu_i().zero(),
        "IsGhostBattle should return NZ for trainer battles (even in Pokémon Tower)"
    );
}

#[test]
fn is_ghost_battle_returns_nz_outside_tower() {
    let mut h = setup_is_ghost_battle();
    h.write_mem(sym_addr("wIsInBattle"), 1); // wild battle
    h.write_mem(sym_addr("wCurMap"), 0x01); // Route 1 (well below tower range)
    clear_bag(&mut h);

    h.set_pc(sym_addr("IsGhostBattle"));
    h.step_to(TRAP_ADDR);

    assert!(
        !h.gb.cpu_i().zero(),
        "IsGhostBattle should return NZ outside Pokémon Tower"
    );
}

#[test]
fn is_ghost_battle_tower_1f_boundary() {
    // POKEMON_TOWER_1F ($8E) is the first tower floor — should be ghost without scope
    let mut h = setup_is_ghost_battle();
    h.write_mem(sym_addr("wIsInBattle"), 1);
    h.write_mem(sym_addr("wCurMap"), POKEMON_TOWER_1F);
    clear_bag(&mut h);

    h.set_pc(sym_addr("IsGhostBattle"));
    h.step_to(TRAP_ADDR);

    assert!(
        h.gb.cpu_i().zero(),
        "IsGhostBattle should return Z on Tower 1F without Silph Scope"
    );
}

#[test]
fn is_ghost_battle_tower_7f_boundary() {
    // POKEMON_TOWER_7F ($94) is the last tower floor — should be ghost without scope
    let mut h = setup_is_ghost_battle();
    h.write_mem(sym_addr("wIsInBattle"), 1);
    h.write_mem(sym_addr("wCurMap"), POKEMON_TOWER_7F);
    clear_bag(&mut h);

    h.set_pc(sym_addr("IsGhostBattle"));
    h.step_to(TRAP_ADDR);

    assert!(
        h.gb.cpu_i().zero(),
        "IsGhostBattle should return Z on Tower 7F without Silph Scope"
    );
}

#[test]
fn is_ghost_battle_map_below_tower_range() {
    // Map $8D is just below POKEMON_TOWER_1F ($8E)
    let mut h = setup_is_ghost_battle();
    h.write_mem(sym_addr("wIsInBattle"), 1);
    h.write_mem(sym_addr("wCurMap"), POKEMON_TOWER_1F - 1);
    clear_bag(&mut h);

    h.set_pc(sym_addr("IsGhostBattle"));
    h.step_to(TRAP_ADDR);

    assert!(
        !h.gb.cpu_i().zero(),
        "IsGhostBattle should return NZ for map just below Tower range"
    );
}

#[test]
fn is_ghost_battle_map_above_tower_range() {
    // Map $95 is just above POKEMON_TOWER_7F ($94)
    let mut h = setup_is_ghost_battle();
    h.write_mem(sym_addr("wIsInBattle"), 1);
    h.write_mem(sym_addr("wCurMap"), POKEMON_TOWER_7F + 1);
    clear_bag(&mut h);

    h.set_pc(sym_addr("IsGhostBattle"));
    h.step_to(TRAP_ADDR);

    assert!(
        !h.gb.cpu_i().zero(),
        "IsGhostBattle should return NZ for map just above Tower range"
    );
}

// ─── ROM byte verification: sprite reload fix ────────────────────────

#[test]
fn rom_bytes_sprite_reload_calls_is_ghost_battle() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("IsGhostBattle"));

    let is_ghost_battle = sym_addr("IsGhostBattle");
    // `call IsGhostBattle` is 9 bytes before .notGhostReload
    let call_addr = sym_addr("PartyMenuOrRockOrRun.notGhostReload") - 9;

    // Verify `call IsGhostBattle` (CD xx xx) at call site
    let op = h.read_mem(call_addr);
    let lo = h.read_mem(call_addr + 1);
    let hi = h.read_mem(call_addr + 2);
    assert_eq!(op, 0xCD, "Expected call opcode ($CD), got ${op:02X}");
    assert_eq!(
        lo,
        (is_ghost_battle & 0xFF) as u8,
        "Expected IsGhostBattle low byte"
    );
    assert_eq!(
        hi,
        (is_ghost_battle >> 8) as u8,
        "Expected IsGhostBattle high byte"
    );
}

#[test]
fn rom_bytes_sprite_reload_jr_nz_skips_ghost_path() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("IsGhostBattle"));

    let not_ghost_reload = sym_addr("PartyMenuOrRockOrRun.notGhostReload");
    // `call IsGhostBattle` is 9 bytes before .notGhostReload
    let call_addr = not_ghost_reload - 9;

    // After the call, expect `jr nz, .notGhostReload` (20 xx)
    let jr_op = h.read_mem(call_addr + 3);
    assert_eq!(jr_op, 0x20, "Expected jr nz ($20), got ${jr_op:02X}");

    // Verify the offset targets .notGhostReload
    let offset = h.read_mem(call_addr + 4) as i8;
    let target = (call_addr + 5).wrapping_add(offset as u16);
    assert_eq!(
        target, not_ghost_reload,
        "jr nz should target .notGhostReload (${not_ghost_reload:04X}), got ${target:04X}"
    );
}

#[test]
fn rom_bytes_sprite_reload_ld_a_mon_ghost() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("IsGhostBattle"));

    // `ld a, MON_GHOST` is 4 bytes before .notGhostReload
    let ld_a_mon_ghost = sym_addr("PartyMenuOrRockOrRun.notGhostReload") - 4;

    // Ghost path: `ld a, MON_GHOST` (3E B8)
    let ld_op = h.read_mem(ld_a_mon_ghost);
    let imm = h.read_mem(ld_a_mon_ghost + 1);
    assert_eq!(ld_op, 0x3E, "Expected ld a,imm8 ($3E), got ${ld_op:02X}");
    assert_eq!(imm, MON_GHOST, "Expected MON_GHOST ($B8), got ${imm:02X}");
}

#[test]
fn rom_bytes_sprite_reload_not_ghost_loads_enemy_species() {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.select_rom_bank(sym_bank("IsGhostBattle"));

    let not_ghost_reload = sym_addr("PartyMenuOrRockOrRun.notGhostReload");

    // .notGhostReload: `ld a, [wEnemyMonSpecies]` (FA E4 CF = $CFE4 = wEnemyMonSpecies)
    let op = h.read_mem(not_ghost_reload);
    let lo = h.read_mem(not_ghost_reload + 1);
    let hi = h.read_mem(not_ghost_reload + 2);
    assert_eq!(op, 0xFA, "Expected ld a,[a16] ($FA), got ${op:02X}");

    let addr = (hi as u16) << 8 | lo as u16;
    // wEnemyMonSpecies = $CFE4
    assert_eq!(
        addr, 0xCFE4,
        "Expected wEnemyMonSpecies ($CFE4), got ${addr:04X}"
    );
}
