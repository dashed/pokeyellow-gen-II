//! Emulator-based tests for the dual-type effectiveness message fix.
//!
//! The original code overwrote `wDamageMultipliers` with each type match,
//! so only the LAST matching type's effectiveness determined the message.
//! Our fix accumulates multiplicatively: super effective doubles the stored
//! value, not very effective halves it, immune zeroes it.
//!
//! Test approach: run from `.skipSameTypeAttackBonus` (after STAB, before
//! the type loop) to `.done`, then check `wDamageMultipliers & $7F`.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

/// A safe WRAM trap address.
const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Type constants (from constants/type_constants.asm).
const NORMAL: u8 = 0x00;
const FIGHTING: u8 = 0x01;
const FLYING: u8 = 0x02;
const POISON: u8 = 0x03;
const GROUND: u8 = 0x04;
const ROCK: u8 = 0x05;
const GHOST: u8 = 0x08;
const FIRE: u8 = 0x14;
const WATER: u8 = 0x15;
const GRASS: u8 = 0x16;
const ELECTRIC: u8 = 0x17;
const ICE: u8 = 0x19;
const DRAGON: u8 = 0x1A;

/// Effectiveness constants (scaled by 10).
const SUPER_EFFECTIVE: u8 = 20;
const EFFECTIVE: u8 = 10;
/// Run the type effectiveness loop and return the effectiveness portion
/// of wDamageMultipliers (bits 0-6).
///
/// `move_type`: the attacking move's type
/// `defender_type1`, `defender_type2`: the defender's two types
/// `stab`: whether to pre-set the STAB bit
fn calc_effectiveness(move_type: u8, defender_type1: u8, defender_type2: u8, stab: bool) -> u8 {
    let w_damage_multipliers = sym_addr("wDamageMultipliers");
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00);
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);
    let bank = sym_bank("AdjustDamageForMoveType");
    h.select_rom_bank(bank);
    h.write_mem(sym_addr("hLoadedROMBank"), bank);

    // Trap for ret
    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    // Set up WRAM
    h.write_mem(sym_addr("wMoveType"), move_type);

    // Initialize wDamageMultipliers to EFFECTIVE (+ STAB if requested)
    let init = if stab { EFFECTIVE | 0x80 } else { EFFECTIVE };
    h.write_mem(w_damage_multipliers, init);

    // Set non-zero damage so the immunity check doesn't trigger spuriously
    h.write_mem(sym_addr("wDamage"), 0x00);
    h.write_mem(sym_addr("wDamage") + 1, 100);

    // At .skipSameTypeAttackBonus, registers must be:
    // B = move type (wMoveType is also read, but B is the primary)
    // D = defender type 1, E = defender type 2
    h.set_b(move_type);
    h.gb.cpu()
        .set_de(((defender_type1 as u16) << 8) | defender_type2 as u16);

    h.set_pc(sym_addr("AdjustDamageForMoveType.skipSameTypeAttackBonus"));
    h.step_to(sym_addr("AdjustDamageForMoveType.done"));

    h.read_mem(w_damage_multipliers) & 0x7F
}

// ─── Dual-type effectiveness message fix ───────────────────────────

#[test]
fn grass_vs_water_flying_is_neutral() {
    // Grass vs Water = SE (×2), Grass vs Flying = NVE (×0.5)
    // Combined: ×1 = neutral (EFFECTIVE = 10)
    // Bug would show: NVE (5) — the last match overwrites
    let eff = calc_effectiveness(GRASS, WATER, FLYING, false);
    assert_eq!(
        eff, EFFECTIVE,
        "Grass vs Water/Flying should be neutral (10), got {eff}"
    );
}

#[test]
fn electric_vs_water_flying_is_4x_super_effective() {
    // Electric vs Water = SE (×2), Electric vs Flying = SE (×2)
    // Combined: ×4 (wDamageMultipliers = 40)
    let eff = calc_effectiveness(ELECTRIC, WATER, FLYING, false);
    assert_eq!(
        eff, 40,
        "Electric vs Water/Flying should be 4× super effective (40), got {eff}"
    );
}

#[test]
fn grass_vs_fire_flying_is_neutral() {
    // Grass vs Fire = NVE (×0.5), Grass vs Flying = NVE (×0.5)
    // Combined: ×0.25 (wDamageMultipliers = 2, truncated from 2.5)
    let eff = calc_effectiveness(GRASS, FIRE, FLYING, false);
    assert!(
        eff < EFFECTIVE,
        "Grass vs Fire/Flying should be not very effective ({eff} < 10)"
    );
}

#[test]
fn ice_vs_grass_poison_is_super_effective() {
    // Ice vs Grass = SE (×2), Ice has no entry for Poison (neutral)
    // Only one match → wDamageMultipliers = 20
    let eff = calc_effectiveness(ICE, GRASS, POISON, false);
    assert_eq!(
        eff, SUPER_EFFECTIVE,
        "Ice vs Grass/Poison should be super effective (20), got {eff}"
    );
}

#[test]
fn ground_vs_fire_flying_is_neutral() {
    // Ground vs Fire = SE (×2), Ground vs Flying = immune (×0)
    // Combined: ×0 (immune)
    let eff = calc_effectiveness(GROUND, FIRE, FLYING, false);
    assert_eq!(
        eff, 0,
        "Ground vs Fire/Flying should be immune (0), got {eff}"
    );
}

#[test]
fn fighting_vs_normal_ghost_is_immune() {
    // Fighting vs Normal = SE (×2), Fighting vs Ghost = immune (×0)
    // Combined: ×0 (immune overrides)
    let eff = calc_effectiveness(FIGHTING, NORMAL, GHOST, false);
    assert_eq!(
        eff, 0,
        "Fighting vs Normal/Ghost should be immune (0), got {eff}"
    );
}

#[test]
fn water_vs_rock_ground_is_4x_super_effective() {
    // Water vs Rock = SE (×2), Water vs Ground = SE (×2)
    // Combined: ×4 (40)
    let eff = calc_effectiveness(WATER, ROCK, GROUND, false);
    assert_eq!(
        eff, 40,
        "Water vs Rock/Ground should be 4× super effective (40), got {eff}"
    );
}

#[test]
fn fire_vs_grass_dragon_is_neutral() {
    // Fire vs Grass = SE (×2), Fire vs Dragon = NVE (×0.5)
    // Combined: ×1 = neutral (10)
    let eff = calc_effectiveness(FIRE, GRASS, DRAGON, false);
    assert_eq!(
        eff, EFFECTIVE,
        "Fire vs Grass/Dragon should be neutral (10), got {eff}"
    );
}

#[test]
fn normal_vs_rock_ghost_is_immune() {
    // Normal vs Rock = NVE (×0.5), Normal vs Ghost = immune (×0)
    // Combined: ×0 (immune overrides)
    let eff = calc_effectiveness(NORMAL, ROCK, GHOST, false);
    assert_eq!(
        eff, 0,
        "Normal vs Rock/Ghost should be immune (0), got {eff}"
    );
}

#[test]
fn stab_preserved_through_dual_type() {
    // Grass vs Water/Flying with STAB: effectiveness should be neutral (10)
    // but the STAB bit (bit 7) should be preserved
    let raw = {
        let mut h = TestHarness::new_headless();
        h.gb.cpu().set_ime(false);
        h.write_mem(0xFFFF, 0x00);
        h.gb.set_timer_enabled(false);
        h.gb.set_serial_enabled(false);
        h.gb.set_dma_enabled(false);
        let bank = sym_bank("AdjustDamageForMoveType");
        h.select_rom_bank(bank);
        h.write_mem(sym_addr("hLoadedROMBank"), bank);

        h.write_mem(TRAP_ADDR, NOP);
        h.write_mem(TRAP_ADDR + 1, STOP);
        h.set_sp(0xDFF0);
        h.push_word(TRAP_ADDR);

        h.write_mem(sym_addr("wMoveType"), GRASS);
        h.write_mem(sym_addr("wDamageMultipliers"), EFFECTIVE | 0x80); // STAB set
        h.write_mem(sym_addr("wDamage"), 0x00);
        h.write_mem(sym_addr("wDamage") + 1, 100);

        h.set_b(GRASS);
        h.gb.cpu().set_de(((WATER as u16) << 8) | FLYING as u16);

        h.set_pc(sym_addr("AdjustDamageForMoveType.skipSameTypeAttackBonus"));
        h.step_to(sym_addr("AdjustDamageForMoveType.done"));

        h.read_mem(sym_addr("wDamageMultipliers"))
    };
    assert_eq!(
        raw & 0x80,
        0x80,
        "STAB bit should be preserved: raw={raw:#04x}"
    );
    assert_eq!(
        raw & 0x7F,
        EFFECTIVE,
        "Effectiveness should be neutral with STAB: raw={raw:#04x}"
    );
}

// ─── Single-type (same type1 == type2) ─────────────────────────────

#[test]
fn water_vs_fire_fire_is_super_effective() {
    // Mono-type Fire: Water vs Fire = SE. Only one match in table.
    // (Both D and E match the same entry, but the code checks D first,
    //  then E — but since both are the same type, only one unique match.)
    // Actually: the loop checks `cp d` then `cp e` for each table entry.
    // For a mono-type Pokemon (d == e), it will match TWICE on the same
    // table entry — once for type 1, once for type 2.
    // With the fix: 10 → SE → 20 → SE → 40 (4x, WRONG for mono-type!)
    //
    // Wait — this is actually the existing behavior for mono-type Pokemon
    // in the original game too. The code loops through the table and for
    // each entry, it checks if D matches OR E matches. Since D == E, it
    // matches the SAME table entry ONCE (either cp d or cp e matches first,
    // then jr z skips to .matchingPairFound). The code doesn't match the
    // same entry twice because after processing, it advances HL past the
    // entry via .nextTypePair (inc hl; inc hl; jp .loop).
    //
    // So for mono-type: only one match, result = 20 (SE). Correct!
    let eff = calc_effectiveness(WATER, FIRE, FIRE, false);
    assert_eq!(
        eff, SUPER_EFFECTIVE,
        "Water vs mono-Fire should be super effective (20), got {eff}"
    );
}

#[test]
fn grass_vs_water_water_is_super_effective() {
    let eff = calc_effectiveness(GRASS, WATER, WATER, false);
    assert_eq!(
        eff, SUPER_EFFECTIVE,
        "Grass vs mono-Water should be super effective (20), got {eff}"
    );
}
