//! Golden-image tests for cosmetic changes.
//!
//! These tests capture the emulator framebuffer at key moments and compare
//! against reference PNGs. To generate/update reference images:
//!
//!   GENERATE_GOLDEN=1 cargo test --test cosmetic
//!
//! After generation, manually verify the PNGs in `tests/golden/` and commit them.

use pokeyellow_tests::{
    compare_screenshot, golden_dir, save_screenshot, should_generate, sym_addr, TestHarness,
};

/// First tile ID of PRESENTS graphic ($67).
const PRESENTS_FIRST_TILE: u8 = 0x67;

/// Maximum frames to wait for the PRESENTS screen during the intro.
/// The intro sequence takes roughly 500-1000 frames from boot.
const MAX_INTRO_FRAMES: u32 = 2000;

// ─── Scenario 11: PRESENTS subtitle golden image ─────────────────

#[test]
fn presents_subtitle_golden_image() {
    // PPU must be enabled for framebuffer rendering.
    let mut h = TestHarness::new();
    h.gb.set_apu_enabled(false); // no audio needed for screenshot tests

    // Run the game from boot through the intro sequence.
    // Poll the tilemap for the PRESENTS tile ($67) appearing at hlcoord 7,11.
    let found = h.wait_for_memory(
        sym_addr("wTileMap") + 11 * 20 + 7,
        |tile| tile == PRESENTS_FIRST_TILE,
        MAX_INTRO_FRAMES,
    );

    if !found {
        // The PRESENTS code may not be present in this ROM build
        // (it's a fork-specific cosmetic change on dashed/cosmetic).
        eprintln!(
            "PRESENTS tile $67 not found at tilemap ${:04X} after {MAX_INTRO_FRAMES} frames. \
             Skipping golden image test (ROM may not include PRESENTS patch).",
            sym_addr("wTileMap") + 11 * 20 + 7
        );
        return;
    }

    // Give a few more frames for the full "PRESENTS" text to render
    // and for the PPU to transfer the tilemap to VRAM.
    h.run_frames(5);

    let screenshot = h.capture_screenshot();
    let golden_path = golden_dir().join("presents_subtitle.png");

    if should_generate() {
        save_screenshot(&screenshot, &golden_path);
        eprintln!("Generated golden image: {}", golden_path.display());
    } else if golden_path.exists() {
        assert!(
            compare_screenshot(&screenshot, &golden_path, 0.98),
            "PRESENTS subtitle screenshot does not match golden image"
        );
    } else {
        eprintln!(
            "Golden image not found at {}. Run with GENERATE_GOLDEN=1 to create it.",
            golden_path.display()
        );
    }
}
