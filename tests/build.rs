use std::path::Path;

/// Nintendo logo bytes that must be present at ROM offset $0104–$0133.
/// The Game Boy boot ROM checks these; if they don't match, the ROM is invalid.
const NINTENDO_LOGO: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

/// Minimum valid ROM size (32 KiB = 2 banks).
const MIN_ROM_SIZE: u64 = 32 * 1024;

/// Source directories to scan for staleness checks.
const SOURCE_DIRS: &[&str] = &[
    "engine",
    "home",
    "data",
    "constants",
    "ram",
    "macros",
    "scripts",
    "text",
    "audio",
];

fn main() {
    // Declare the custom cfg so rustc doesn't warn about it.
    println!("cargo::rustc-check-cfg=cfg(rom_available)");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest_dir).join("..");
    let rom_path = root.join("pokeyellow.gbc");
    let sym_path = root.join("pokeyellow.sym");

    // Re-run this build script when the ROM or sym file changes.
    println!("cargo:rerun-if-changed={}", rom_path.display());
    println!("cargo:rerun-if-changed={}", sym_path.display());

    // --- ROM existence, size, and header check ---
    let rom_ok = match std::fs::metadata(&rom_path) {
        Ok(meta) if meta.len() >= MIN_ROM_SIZE => {
            match std::fs::read(&rom_path) {
                Ok(data) if data.len() >= 0x150 => {
                    let logo = &data[0x104..0x134];
                    if logo != NINTENDO_LOGO {
                        println!(
                            "cargo:warning=pokeyellow.gbc has invalid Nintendo logo header. \
                             The ROM may be corrupt. Rebuild with `make pokeyellow.gbc`."
                        );
                        false
                    } else {
                        // Header checksum at $014D
                        let expected = data[0x14D];
                        let mut x: u8 = 0;
                        for &byte in &data[0x134..=0x14C] {
                            x = x.wrapping_sub(byte).wrapping_sub(1);
                        }
                        if x != expected {
                            println!(
                                "cargo:warning=pokeyellow.gbc header checksum mismatch \
                                 (expected {:#04X}, computed {:#04X}). ROM may be corrupt.",
                                expected, x
                            );
                            false
                        } else {
                            true
                        }
                    }
                }
                _ => {
                    println!(
                        "cargo:warning=pokeyellow.gbc is too small to be a valid ROM. \
                         Rebuild with `make pokeyellow.gbc`."
                    );
                    false
                }
            }
        }
        Ok(meta) => {
            println!(
                "cargo:warning=pokeyellow.gbc is only {} bytes (minimum {}). \
                 Rebuild with `make pokeyellow.gbc`.",
                meta.len(),
                MIN_ROM_SIZE
            );
            false
        }
        Err(_) => {
            println!(
                "cargo:warning=pokeyellow.gbc not found. \
                 Build the ROM first: `make pokeyellow.gbc`"
            );
            false
        }
    };

    // --- Symbol file check ---
    let sym_ok = match std::fs::metadata(&sym_path) {
        Ok(meta) if meta.len() > 0 => true,
        _ => {
            println!(
                "cargo:warning=pokeyellow.sym not found or empty. \
                 Build the ROM first: `make pokeyellow.gbc`"
            );
            false
        }
    };

    // --- Staleness check ---
    if rom_ok {
        if let Ok(rom_mtime) = std::fs::metadata(&rom_path).and_then(|m| m.modified()) {
            let mut stale_file = None;
            for dir_name in SOURCE_DIRS {
                let dir = root.join(dir_name);
                if dir.exists() {
                    check_staleness(&dir, rom_mtime, &mut stale_file);
                }
            }
            if let Some(path) = stale_file {
                let relative = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                println!(
                    "cargo:warning=ROM may be stale: {relative} is newer than pokeyellow.gbc. \
                     Rebuild with `make pokeyellow.gbc`."
                );
            }
        }
    }

    // Set cfg flag so test code can conditionally compile.
    if rom_ok && sym_ok {
        println!("cargo:rustc-cfg=rom_available");
    }
}

/// Recursively check .asm files for any newer than `rom_mtime`.
fn check_staleness(
    dir: &Path,
    rom_mtime: std::time::SystemTime,
    stale_file: &mut Option<std::path::PathBuf>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            check_staleness(&path, rom_mtime, stale_file);
        } else if path.extension().and_then(|e| e.to_str()) == Some("asm") {
            if let Ok(meta) = std::fs::metadata(&path) {
                if let Ok(mtime) = meta.modified() {
                    if mtime > rom_mtime {
                        // Keep the first stale file found (for the warning message)
                        if stale_file.is_none() {
                            *stale_file = Some(path);
                        }
                    }
                }
            }
        }
    }
}
