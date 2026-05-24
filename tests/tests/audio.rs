//! Audio smoke tests — verify sound engine isn't broken by our changes.
//!
//! These tests check that the APU produces non-silent output during
//! gameplay sequences where music should be playing.

use pokeyellow_tests::TestHarness;

// ─── Scenario 16: Title screen audio smoke test ─────────────────

#[test]
fn title_screen_has_audio() {
    let mut h = TestHarness::new();
    // APU must be enabled (it is by default in new(), not new_headless())

    // Advance past intro to title screen (~1100 frames ≈ 18 seconds)
    h.run_frames(1100);

    // Clear audio buffer, then collect 1 second of fresh samples
    let _ = h.gb.audio_buffer_eager(true); // discard + clear
    h.run_frames(60);

    let buffer = h.gb.audio_buffer_eager(false);
    assert!(
        !buffer.is_empty(),
        "Audio buffer should have samples on title screen"
    );

    // Check that a meaningful fraction of samples are non-zero (actual sound)
    let non_zero = buffer.iter().filter(|&&s| s != 0).count();
    let ratio = non_zero as f64 / buffer.len() as f64;
    assert!(
        ratio > 0.1,
        "Title screen should have audible sound ({:.1}% non-zero, expected >10%)",
        ratio * 100.0
    );
}

#[test]
fn title_screen_melody_channels_active() {
    let mut h = TestHarness::new();

    // Advance past intro to title screen. The intro sequence (copyright →
    // shooting star → Game Freak → Pikachu → title) takes ~1000 frames.
    h.run_frames(1100);

    // Sample ch1 and ch2 (melody channels) across multiple frames.
    // Per-channel output is instantaneous and may be 0 between notes,
    // so we sample at many points across several frames of music.
    let mut any_ch1 = false;
    let mut any_ch2 = false;
    for _ in 0..120 {
        // Sample multiple times within each frame
        for _ in 0..100 {
            h.gb.clock();
            if h.gb.audio_ch1_output() > 0 {
                any_ch1 = true;
            }
            if h.gb.audio_ch2_output() > 0 {
                any_ch2 = true;
            }
        }
        if any_ch1 && any_ch2 {
            break;
        }
        // Advance to next frame
        h.run_frames(1);
    }

    assert!(
        any_ch1 || any_ch2,
        "At least one melody channel (ch1/ch2) should produce sound on title screen"
    );
}
