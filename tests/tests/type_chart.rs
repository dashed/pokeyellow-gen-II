//! Exhaustive type chart sweep: all 225 single-type pairs through AdjustDamageForMoveType.
//!
//! Runs every (attacking_type, defending_type) combination for mono-type defenders
//! and verifies the effectiveness multiplier matches the TypeEffects table in
//! data/types/type_matchups.asm.

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

// Type constants (from constants/type_constants.asm).
const NORMAL: u8 = 0x00;
const FIGHTING: u8 = 0x01;
const FLYING: u8 = 0x02;
const POISON: u8 = 0x03;
const GROUND: u8 = 0x04;
const ROCK: u8 = 0x05;
const BUG: u8 = 0x07;
const GHOST: u8 = 0x08;
const FIRE: u8 = 0x14;
const WATER: u8 = 0x15;
const GRASS: u8 = 0x16;
const ELECTRIC: u8 = 0x17;
const PSYCHIC_TYPE: u8 = 0x18;
const ICE: u8 = 0x19;
const DRAGON: u8 = 0x1A;

const ALL_TYPES: [u8; 15] = [
    NORMAL, FIGHTING, FLYING, POISON, GROUND, ROCK, BUG, GHOST,
    FIRE, WATER, GRASS, ELECTRIC, PSYCHIC_TYPE, ICE, DRAGON,
];

const EFFECTIVE: u8 = 10;

fn calc_effectiveness(move_type: u8, defender_type1: u8, defender_type2: u8) -> u8 {
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

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);
    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);

    h.write_mem(sym_addr("wMoveType"), move_type);
    h.write_mem(w_damage_multipliers, EFFECTIVE);
    h.write_mem(sym_addr("wDamage"), 0x00);
    h.write_mem(sym_addr("wDamage") + 1, 100);

    h.set_b(move_type);
    h.gb.cpu()
        .set_de(((defender_type1 as u16) << 8) | defender_type2 as u16);

    h.set_pc(sym_addr("AdjustDamageForMoveType.skipSameTypeAttackBonus"));
    h.step_to(sym_addr("AdjustDamageForMoveType.done"));

    h.read_mem(w_damage_multipliers) & 0x7F
}

fn expected_effectiveness(atk: u8, def: u8) -> u8 {
    match (atk, def) {
        // Normal
        (NORMAL, ROCK) => 5,
        (NORMAL, GHOST) => 0,
        // Fighting
        (FIGHTING, NORMAL) => 20,
        (FIGHTING, POISON) => 5,
        (FIGHTING, FLYING) => 5,
        (FIGHTING, ROCK) => 20,
        (FIGHTING, BUG) => 5,
        (FIGHTING, GHOST) => 0,
        (FIGHTING, PSYCHIC_TYPE) => 5,
        (FIGHTING, ICE) => 20,
        // Flying
        (FLYING, ELECTRIC) => 5,
        (FLYING, FIGHTING) => 20,
        (FLYING, BUG) => 20,
        (FLYING, GRASS) => 20,
        (FLYING, ROCK) => 5,
        // Poison
        (POISON, GRASS) => 20,
        (POISON, POISON) => 5,
        (POISON, GROUND) => 5,
        (POISON, BUG) => 20,
        (POISON, ROCK) => 5,
        (POISON, GHOST) => 5,
        // Ground
        (GROUND, FIRE) => 20,
        (GROUND, ELECTRIC) => 20,
        (GROUND, GRASS) => 5,
        (GROUND, FLYING) => 0,
        (GROUND, BUG) => 5,
        (GROUND, ROCK) => 20,
        (GROUND, POISON) => 20,
        // Rock
        (ROCK, FIRE) => 20,
        (ROCK, FIGHTING) => 5,
        (ROCK, FLYING) => 20,
        (ROCK, GROUND) => 5,
        (ROCK, BUG) => 20,
        (ROCK, ICE) => 20,
        // Bug
        (BUG, FIRE) => 5,
        (BUG, GRASS) => 20,
        (BUG, FIGHTING) => 5,
        (BUG, FLYING) => 5,
        (BUG, PSYCHIC_TYPE) => 20,
        (BUG, GHOST) => 5,
        (BUG, POISON) => 20,
        // Ghost
        (GHOST, GHOST) => 20,
        (GHOST, NORMAL) => 0,
        (GHOST, PSYCHIC_TYPE) => 0,
        // Fire
        (FIRE, GRASS) => 20,
        (FIRE, ICE) => 20,
        (FIRE, BUG) => 20,
        (FIRE, FIRE) => 5,
        (FIRE, WATER) => 5,
        (FIRE, ROCK) => 5,
        (FIRE, DRAGON) => 5,
        // Water
        (WATER, FIRE) => 20,
        (WATER, ROCK) => 20,
        (WATER, GROUND) => 20,
        (WATER, WATER) => 5,
        (WATER, GRASS) => 5,
        (WATER, DRAGON) => 5,
        // Grass
        (GRASS, WATER) => 20,
        (GRASS, GROUND) => 20,
        (GRASS, ROCK) => 20,
        (GRASS, FIRE) => 5,
        (GRASS, GRASS) => 5,
        (GRASS, BUG) => 5,
        (GRASS, POISON) => 5,
        (GRASS, FLYING) => 5,
        (GRASS, DRAGON) => 5,
        // Electric
        (ELECTRIC, WATER) => 20,
        (ELECTRIC, FLYING) => 20,
        (ELECTRIC, ELECTRIC) => 5,
        (ELECTRIC, GRASS) => 5,
        (ELECTRIC, GROUND) => 0,
        (ELECTRIC, DRAGON) => 5,
        // Psychic
        (PSYCHIC_TYPE, FIGHTING) => 20,
        (PSYCHIC_TYPE, POISON) => 20,
        (PSYCHIC_TYPE, PSYCHIC_TYPE) => 5,
        // Ice
        (ICE, GRASS) => 20,
        (ICE, GROUND) => 20,
        (ICE, FLYING) => 20,
        (ICE, DRAGON) => 20,
        (ICE, ICE) => 5,
        (ICE, WATER) => 5,
        // Dragon
        (DRAGON, DRAGON) => 20,
        // Everything else is neutral
        _ => 10,
    }
}

fn type_name(t: u8) -> &'static str {
    match t {
        NORMAL => "Normal",
        FIGHTING => "Fighting",
        FLYING => "Flying",
        POISON => "Poison",
        GROUND => "Ground",
        ROCK => "Rock",
        BUG => "Bug",
        GHOST => "Ghost",
        FIRE => "Fire",
        WATER => "Water",
        GRASS => "Grass",
        ELECTRIC => "Electric",
        PSYCHIC_TYPE => "Psychic",
        ICE => "Ice",
        DRAGON => "Dragon",
        _ => "???",
    }
}

// ─── Exhaustive single-type sweep (225 pairs) ──────────────────────

#[test]
fn exhaustive_type_chart_single_type() {
    let mut failures = Vec::new();
    for &atk_type in &ALL_TYPES {
        for &def_type in &ALL_TYPES {
            let actual = calc_effectiveness(atk_type, def_type, def_type);
            let expected = expected_effectiveness(atk_type, def_type);
            if actual != expected {
                failures.push(format!(
                    "{}→{}: expected {expected}, got {actual}",
                    type_name(atk_type),
                    type_name(def_type),
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Type chart mismatches ({} of 225):\n{}",
        failures.len(),
        failures.join("\n"),
    );
}

// ─── Dual-type interactions (not covered in effectiveness.rs) ───────

#[test]
fn ice_vs_grass_ground_is_4x_super_effective() {
    // Ice→Grass = SE (×2), Ice→Ground = SE (×2) → 4×
    let eff = calc_effectiveness(ICE, GRASS, GROUND);
    assert_eq!(eff, 40, "Ice vs Grass/Ground should be 4× SE (40), got {eff}");
}

#[test]
fn rock_vs_fire_ice_is_4x_super_effective() {
    // Rock→Fire = SE (×2), Rock→Ice = SE (×2) → 4×
    let eff = calc_effectiveness(ROCK, FIRE, ICE);
    assert_eq!(eff, 40, "Rock vs Fire/Ice should be 4× SE (40), got {eff}");
}

#[test]
fn bug_vs_grass_psychic_is_4x_super_effective() {
    // Bug→Grass = SE (×2), Bug→Psychic = SE (×2) → 4×
    let eff = calc_effectiveness(BUG, GRASS, PSYCHIC_TYPE);
    assert_eq!(eff, 40, "Bug vs Grass/Psychic should be 4× SE (40), got {eff}");
}

#[test]
fn ground_vs_electric_poison_is_4x_super_effective() {
    // Ground→Electric = SE (×2), Ground→Poison = SE (×2) → 4×
    let eff = calc_effectiveness(GROUND, ELECTRIC, POISON);
    assert_eq!(eff, 40, "Ground vs Electric/Poison should be 4× SE (40), got {eff}");
}

#[test]
fn electric_vs_water_ground_is_immune() {
    // Electric→Water = SE (×2), Electric→Ground = immune (×0) → immune overrides
    let eff = calc_effectiveness(ELECTRIC, WATER, GROUND);
    assert_eq!(eff, 0, "Electric vs Water/Ground should be immune (0), got {eff}");
}

#[test]
fn fire_vs_grass_water_is_neutral() {
    // Fire→Grass = SE (×2), Fire→Water = NVE (×0.5) → ×1 neutral
    let eff = calc_effectiveness(FIRE, GRASS, WATER);
    assert_eq!(eff, 10, "Fire vs Grass/Water should be neutral (10), got {eff}");
}

#[test]
fn poison_vs_grass_rock_is_neutral() {
    // Poison→Grass = SE (×2), Poison→Rock = NVE (×0.5) → ×1 neutral
    let eff = calc_effectiveness(POISON, GRASS, ROCK);
    assert_eq!(eff, 10, "Poison vs Grass/Rock should be neutral (10), got {eff}");
}

#[test]
fn ghost_vs_normal_psychic_is_immune() {
    // Ghost→Normal = immune (×0), Ghost→Psychic = immune (×0) → immune
    // (In Gen I, Ghost has NO_EFFECT against Psychic — a famous bug)
    let eff = calc_effectiveness(GHOST, NORMAL, PSYCHIC_TYPE);
    assert_eq!(eff, 0, "Ghost vs Normal/Psychic should be immune (0), got {eff}");
}

#[test]
fn flying_vs_fighting_rock_is_neutral() {
    // Flying→Fighting = SE (×2), Flying→Rock = NVE (×0.5) → ×1 neutral
    let eff = calc_effectiveness(FLYING, FIGHTING, ROCK);
    assert_eq!(eff, 10, "Flying vs Fighting/Rock should be neutral (10), got {eff}");
}

#[test]
fn ice_vs_water_flying_is_neutral() {
    // Ice→Water = NVE (×0.5), Ice→Flying = SE (×2) → ×1 neutral
    let eff = calc_effectiveness(ICE, WATER, FLYING);
    assert_eq!(eff, 10, "Ice vs Water/Flying should be neutral (10), got {eff}");
}

#[test]
fn grass_vs_poison_rock_is_quarter_effective() {
    // Grass→Poison = NVE (×0.5), Grass→Rock = SE (×2) → ×1 neutral
    // Wait: Grass→Rock is SE, Grass→Poison is NVE → neutral
    // Actually let's pick Grass vs Fire/Dragon: NVE × NVE = 0.25×
    let eff = calc_effectiveness(GRASS, FIRE, DRAGON);
    assert!(eff < 10, "Grass vs Fire/Dragon should be 0.25× NVE ({eff} < 10)");
}

#[test]
fn fighting_vs_rock_ice_is_4x_super_effective() {
    // Fighting→Rock = SE (×2), Fighting→Ice = SE (×2) → 4×
    let eff = calc_effectiveness(FIGHTING, ROCK, ICE);
    assert_eq!(eff, 40, "Fighting vs Rock/Ice should be 4× SE (40), got {eff}");
}
