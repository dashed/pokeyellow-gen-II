use image::{ImageBuffer, Rgb};
use std::path::{Path, PathBuf};

const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;

pub fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("golden")
}

pub fn should_generate() -> bool {
    std::env::var("GENERATE_GOLDEN").is_ok_and(|v| v == "1")
}

pub fn save_screenshot(pixels: &[u8], path: impl AsRef<Path>) {
    let img = ImageBuffer::<Rgb<u8>, _>::from_raw(SCREEN_WIDTH, SCREEN_HEIGHT, pixels.to_vec())
        .expect("Failed to create image buffer from framebuffer");
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent).ok();
    }
    img.save(path.as_ref()).expect("Failed to save screenshot");
}

pub fn compare_screenshot(pixels: &[u8], reference_path: impl AsRef<Path>, threshold: f64) -> bool {
    let reference = image::open(reference_path.as_ref())
        .unwrap_or_else(|e| {
            panic!(
                "Failed to open reference image {:?}: {}",
                reference_path.as_ref(),
                e
            )
        })
        .to_rgb8();

    assert_eq!(reference.width(), SCREEN_WIDTH);
    assert_eq!(reference.height(), SCREEN_HEIGHT);

    let ref_bytes = reference.as_raw();
    let total = (SCREEN_WIDTH * SCREEN_HEIGHT) as usize;
    let mut matching = 0usize;

    for i in 0..total {
        let off = i * 3;
        if pixels[off] == ref_bytes[off]
            && pixels[off + 1] == ref_bytes[off + 1]
            && pixels[off + 2] == ref_bytes[off + 2]
        {
            matching += 1;
        }
    }

    let similarity = matching as f64 / total as f64;
    similarity >= threshold
}
