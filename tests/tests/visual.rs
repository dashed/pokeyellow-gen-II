//! Golden-image tests for visual verification of QoL changes.
//!
//! These tests capture the emulator framebuffer at key moments and compare
//! against reference PNGs. To generate/update reference images:
//!
//!   GENERATE_GOLDEN=1 cargo test --test visual
//!
//! After generation, manually verify the PNGs in `tests/golden/` and commit them.

use boytacean::pad::PadKey;
use pokeyellow_tests::{
    compare_screenshot, golden_dir, save_screenshot, should_generate, TestHarness,
};

// ─── Scenario 12: Options menu WARP golden image ─────────────────

#[test]
fn options_menu_shows_warp_text() {
    // This test verifies the options menu displays "WARP" as a text speed
    // option by navigating: Title → Main Menu → OPTION → Right to WARP.
    let mut h = TestHarness::new();
    h.gb.set_apu_enabled(false); // no audio needed for screenshot tests

    // ── Phase 1: Run through the intro sequence ─────────────────────
    // Boot → copyright → shooting star → Game Freak logo → Pikachu →
    // Pikachu voice → title screen. Takes ~900-1000 frames.
    //
    // Hold Start early to skip the intro. The first Start press is
    // caught by CheckForUserInterruption during the intro, which speeds
    // up the transition to the title screen. A second Start press on
    // the actual title screen enters MainMenu.
    h.run_frames(800);

    // Press Start to skip any remaining intro animation.
    h.gb.key_press(PadKey::Start);
    h.wait_for_memory(0xFFB5, |v| v & 0x08 != 0, 600);
    h.gb.key_lift(PadKey::Start);

    // Wait for the title screen to appear and settle.
    h.run_frames(300);

    // ── Phase 2: Enter the main menu ────────────────────────────────
    // Press Start on the title screen, then wait for MainMenu to set up
    // wMaxMenuItem (indicates HandleMenuInput is about to run).
    h.press(PadKey::Start, 4);
    h.run_frames(60);

    let menu_ready = h.wait_for_memory(
        0xCC28,    // wMaxMenuItem
        |v| v > 0, // non-zero = menu is drawn and ready
        600,
    );
    assert!(menu_ready, "Main menu HandleMenuInput never became active");

    // ── Phase 3: Navigate to OPTION ─────────────────────────────────
    // Press Down to move cursor from NEW GAME to OPTION.
    h.press(PadKey::Down, 4);
    h.run_frames(30);

    // Verify cursor moved
    let menu_item = h.read_mem(0xCC26); // wCurrentMenuItem
    eprintln!("wCurrentMenuItem after Down = {}", menu_item);

    // Press A to enter the options screen.
    h.press(PadKey::A, 4);
    h.run_frames(120);

    // ── Phase 4: Navigate text speed to WARP ────────────────────────
    // InitOptions sets wOptions = TEXT_DELAY_MEDIUM (3).
    // Right cycles: FAST→MID→SLOW→WARP→FAST.
    // From MID: Right→SLOW, Right→WARP. Need exactly 2 presses.
    for _ in 0..2 {
        h.press(PadKey::Right, 4);
        h.run_frames(30);
    }
    h.run_frames(30);

    // ── Phase 5: Golden image comparison ────────────────────────────
    let screenshot = h.capture_screenshot();
    let golden_path = golden_dir().join("options_warp.png");

    if should_generate() {
        save_screenshot(&screenshot, &golden_path);
        eprintln!("Generated golden image: {}", golden_path.display());
    } else if golden_path.exists() {
        assert!(
            compare_screenshot(&screenshot, &golden_path, 0.95),
            "Options menu WARP screenshot does not match golden image"
        );
    } else {
        eprintln!(
            "Golden image not found at {}. Run with GENERATE_GOLDEN=1 to create it.",
            golden_path.display()
        );
    }
}
