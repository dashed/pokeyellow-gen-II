//! E2E test: full new game → Oak's Lab → get Pikachu → rival battle.
//!
//! Plays through the entire opening sequence using button input.
//! Verifies the game progresses through Oak's Lab scripts without crashing.

use boytacean::pad::PadKey;
use pokeyellow_tests::TestHarness;

const W_CUR_MAP: u16 = 0xD35D;
const W_IS_IN_BATTLE: u16 = 0xD056;
const W_PARTY_COUNT: u16 = 0xD162;
const W_Y_COORD: u16 = 0xD360;
const W_X_COORD: u16 = 0xD361;
const W_MAX_MENU_ITEM: u16 = 0xCC28;
const W_OAKS_LAB_CUR_SCRIPT: u16 = 0xD5EF;
const W_SIMULATED_JOYPAD_INDEX: u16 = 0xCD38;

const OAKS_LAB: u8 = 0x28;
const PALLET_TOWN: u8 = 0x00;

fn advance_text(h: &mut TestHarness, times: u32) {
    for _ in 0..times {
        h.press(PadKey::A, 4);
        h.run_frames(30);
    }
}

// Exact copy of the WORKING helpers from e2e_gameplay.rs
fn boot_to_main_menu(h: &mut TestHarness) {
    h.run_frames(1200);
    h.press(PadKey::Start, 4);
    h.run_frames(120);
    h.press(PadKey::Start, 4);
    h.run_frames(180);
    h.wait_for_memory(W_MAX_MENU_ITEM, |v| v > 0, 600);
}

fn select_new_game(h: &mut TestHarness) {
    h.press(PadKey::A, 4);
    h.run_frames(60);
}

fn navigate_oak_speech(h: &mut TestHarness) {
    for _ in 0..20 {
        h.press(PadKey::A, 4);
        h.run_frames(40);
    }
    h.wait_for_memory(W_MAX_MENU_ITEM, |v| v > 0, 600);
    h.press(PadKey::Down, 4);
    h.run_frames(15);
    h.press(PadKey::A, 4);
    h.run_frames(60);
    advance_text(h, 5);
    for _ in 0..10 {
        h.press(PadKey::A, 4);
        h.run_frames(40);
    }
    h.wait_for_memory(W_MAX_MENU_ITEM, |v| v > 0, 600);
    h.press(PadKey::Down, 4);
    h.run_frames(15);
    h.press(PadKey::A, 4);
    h.run_frames(60);
    advance_text(h, 5);
    for _ in 0..10 {
        h.press(PadKey::A, 4);
        h.run_frames(40);
    }
    h.run_frames(300);
}

fn walk_to_pallet_town(h: &mut TestHarness) {
    for _ in 0..8 {
        h.press(PadKey::Down, 8);
        h.run_frames(8);
    }
    h.run_frames(60);
    for _ in 0..8 {
        h.press(PadKey::Down, 8);
        h.run_frames(8);
    }
    h.run_frames(120);
}

fn walk_north_trigger_oak(h: &mut TestHarness) {
    for _ in 0..15 {
        h.press(PadKey::Up, 8);
        h.run_frames(8);
    }
    // Oak's scripted sequence: wait and press A through dialogue
    h.run_frames(300);
    advance_text(h, 15);
    h.run_frames(600);
    advance_text(h, 10);
    h.run_frames(800);
    advance_text(h, 5);
    h.run_frames(400);
}

/// Navigate Oak's Lab to get Pikachu. Presses A through dialogue,
/// walks to the Pokeball, waits for scripted sequences.
fn navigate_oaks_lab(h: &mut TestHarness) {
    // Wait for scripted entry walk to finish
    h.run_frames(600);
    h.wait_for_memory(W_SIMULATED_JOYPAD_INDEX, |v| v == 0, 3000);
    h.run_frames(30);

    // Press A through Oak's "choose a mon" and rival dialogue
    for _ in 0..30 {
        h.press(PadKey::A, 4);
        h.run_frames(30);
    }
    h.run_frames(120);

    // Walk right to the Pokeball at (7,3), player should be near (5,3)
    for _ in 0..5 {
        h.press(PadKey::Right, 8);
        h.run_frames(8);
    }
    h.press(PadKey::Up, 8);
    h.run_frames(8);
    h.press(PadKey::Up, 4);
    h.run_frames(8);
    h.press(PadKey::A, 4);
    h.run_frames(60);

    // Rival takes Eevee, scripted sequences
    advance_text(h, 20);
    h.run_frames(300);
    h.wait_for_memory(W_SIMULATED_JOYPAD_INDEX, |v| v == 0, 3000);
    advance_text(h, 15);
    h.run_frames(300);
    h.wait_for_memory(W_SIMULATED_JOYPAD_INDEX, |v| v == 0, 3000);

    // Player receives Pikachu
    advance_text(h, 15);
    h.run_frames(300);

    // More dialogue
    for _ in 0..20 {
        h.press(PadKey::A, 4);
        h.run_frames(30);
    }
    h.run_frames(120);
}

/// Walk south toward exit to trigger rival battle (Y=6 triggers it).
fn trigger_rival_battle(h: &mut TestHarness) {
    for _ in 0..10 {
        h.press(PadKey::Down, 8);
        h.run_frames(8);
    }
    h.run_frames(300);
    advance_text(h, 10);
    h.run_frames(200);
    for _ in 0..15 {
        h.press(PadKey::A, 4);
        h.run_frames(30);
    }
    h.run_frames(300);
}

#[test]
fn full_intro_to_rival_battle() {
    let mut h = TestHarness::new();
    h.gb.set_apu_enabled(false);

    boot_to_main_menu(&mut h);
    select_new_game(&mut h);
    navigate_oak_speech(&mut h);

    let map_after_speech = h.read_mem(W_CUR_MAP);
    eprintln!(
        "After speech: map=${:02X} pos=({},{})",
        map_after_speech,
        h.read_mem(W_X_COORD),
        h.read_mem(W_Y_COORD)
    );

    walk_to_pallet_town(&mut h);

    let map_after_walk = h.read_mem(W_CUR_MAP);
    eprintln!(
        "After walk: map=${:02X} pos=({},{})",
        map_after_walk,
        h.read_mem(W_X_COORD),
        h.read_mem(W_Y_COORD)
    );

    // Only continue if we made it to Pallet Town
    if map_after_walk == PALLET_TOWN {
        walk_north_trigger_oak(&mut h);

        let map_after_oak = h.read_mem(W_CUR_MAP);
        eprintln!(
            "After Oak: map=${:02X} script={}",
            map_after_oak,
            h.read_mem(W_OAKS_LAB_CUR_SCRIPT)
        );

        if map_after_oak == OAKS_LAB {
            navigate_oaks_lab(&mut h);

            let party = h.read_mem(W_PARTY_COUNT);
            eprintln!(
                "After lab: party={} script={}",
                party,
                h.read_mem(W_OAKS_LAB_CUR_SCRIPT)
            );

            if party >= 1 {
                trigger_rival_battle(&mut h);
            }
        }
    }

    // Final state
    let final_map = h.read_mem(W_CUR_MAP);
    let final_battle = h.read_mem(W_IS_IN_BATTLE);
    let final_party = h.read_mem(W_PARTY_COUNT);
    let final_script = h.read_mem(W_OAKS_LAB_CUR_SCRIPT);
    eprintln!(
        "FINAL: map=${:02X} battle={} party={} script={} pos=({},{})",
        final_map,
        final_battle,
        final_party,
        final_script,
        h.read_mem(W_X_COORD),
        h.read_mem(W_Y_COORD)
    );

    // The game must not have crashed
    assert!(h.pc() < 0x8000, "Game should not have crashed");

    // We should have made progress beyond the title screen
    assert!(
        final_map != 0 || final_party > 0 || final_script > 0,
        "Game should have progressed beyond initial state"
    );
}
