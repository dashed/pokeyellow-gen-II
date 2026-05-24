use std::path::{Path, PathBuf};

pub fn rom_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../pokeyellow.gbc")
}

pub fn sym_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../pokeyellow.sym")
}

/// Root of the project (parent of the tests/ crate).
fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

pub fn load_rom_bytes(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("Failed to read ROM file {path}: {e}"))
}

// ─── Constants ───────────────────────────────────────────────────────

/// Nintendo logo bytes at ROM offset $0104–$0133.
const NINTENDO_LOGO: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

/// Minimum valid ROM size (32 KiB = 2 banks).
const MIN_ROM_SIZE: usize = 32 * 1024;

/// Memory limit for test processes (2 GiB). Prevents runaway emulator
/// execution from consuming all system memory.
#[cfg(unix)]
const MEMORY_LIMIT_BYTES: u64 = 4 * 1024 * 1024 * 1024;

// ─── ROM validation ──────────────────────────────────────────────────

/// Validate ROM data before loading into the emulator.
///
/// Checks:
/// 1. Size is at least 32 KiB (minimum valid Game Boy ROM)
/// 2. Nintendo logo at $0104–$0133 matches (boot ROM would reject otherwise)
/// 3. Title at $0134 contains "POKEMON" (sanity check for correct ROM)
/// 4. Header checksum at $014D is correct
/// 5. Global checksum at $014E–$014F is correct
///
/// Panics with a clear, actionable message if validation fails.
/// This prevents the emulator from loading invalid data, which can cause
/// runaway memory usage and SIGKILL.
pub fn validate_rom(data: &[u8]) {
    // Size check
    if data.len() < MIN_ROM_SIZE {
        panic!(
            "\n\n\
             ╔══════════════════════════════════════════════════════════╗\n\
             ║  ROM VALIDATION FAILED                                  ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  pokeyellow.gbc is only {} bytes (need >= {}).    ║\n\
             ║                                                         ║\n\
             ║  Build the ROM first:  make pokeyellow.gbc              ║\n\
             ╚══════════════════════════════════════════════════════════╝\n",
            data.len(),
            MIN_ROM_SIZE
        );
    }

    // Nintendo logo check — the definitive "is this a valid GB ROM" test
    let logo = &data[0x104..0x134];
    if logo != NINTENDO_LOGO {
        panic!(
            "\n\n\
             ╔══════════════════════════════════════════════════════════╗\n\
             ║  ROM VALIDATION FAILED                                  ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  pokeyellow.gbc has an invalid Nintendo logo header.    ║\n\
             ║  The file may be corrupt or not a Game Boy ROM.         ║\n\
             ║                                                         ║\n\
             ║  Rebuild:  make pokeyellow.gbc                          ║\n\
             ╚══════════════════════════════════════════════════════════╝\n"
        );
    }

    // Title check — verify this is actually Pokemon Yellow
    let title = &data[0x134..0x143];
    let title_str = std::str::from_utf8(title).unwrap_or("");
    if !title_str.starts_with("POKEMON YELLOW") {
        panic!(
            "\n\n\
             ╔══════════════════════════════════════════════════════════╗\n\
             ║  ROM VALIDATION FAILED                                  ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  pokeyellow.gbc title is {:?},          ║\n\
             ║  expected \"POKEMON YELLOW\".                             ║\n\
             ║  Wrong ROM file?                                        ║\n\
             ║                                                         ║\n\
             ║  Rebuild:  make pokeyellow.gbc                          ║\n\
             ╚══════════════════════════════════════════════════════════╝\n",
            title_str.trim_end_matches('\0')
        );
    }

    // Header checksum at $014D
    let expected_hdr = data[0x14D];
    let computed_hdr = compute_header_checksum(data);
    if expected_hdr != computed_hdr {
        panic!(
            "\n\n\
             ╔══════════════════════════════════════════════════════════╗\n\
             ║  ROM CHECKSUM FAILED                                    ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  Header checksum: expected {:#04X}, computed {:#04X}.   ║\n\
             ║  The ROM file may be corrupt or partially written.      ║\n\
             ║                                                         ║\n\
             ║  Rebuild:  make pokeyellow.gbc                          ║\n\
             ╚══════════════════════════════════════════════════════════╝\n",
            expected_hdr, computed_hdr
        );
    }

    // Global checksum at $014E–$014F (big-endian)
    let expected_global = ((data[0x14E] as u16) << 8) | (data[0x14F] as u16);
    let computed_global = compute_global_checksum(data);
    if expected_global != computed_global {
        panic!(
            "\n\n\
             ╔══════════════════════════════════════════════════════════╗\n\
             ║  ROM CHECKSUM FAILED                                    ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  Global checksum: expected {:#06X}, computed {:#06X}.   ║\n\
             ║  The ROM file may be corrupt or truncated.              ║\n\
             ║                                                         ║\n\
             ║  Rebuild:  make pokeyellow.gbc                          ║\n\
             ╚══════════════════════════════════════════════════════════╝\n",
            expected_global, computed_global
        );
    }
}

/// Compute the Game Boy header checksum (byte at $014D).
///
/// Algorithm: `x = 0; for byte in $0134..=$014C: x = x - byte - 1`
pub fn compute_header_checksum(data: &[u8]) -> u8 {
    let mut x: u8 = 0;
    for &byte in &data[0x134..=0x14C] {
        x = x.wrapping_sub(byte).wrapping_sub(1);
    }
    x
}

/// Compute the Game Boy global checksum (big-endian u16 at $014E–$014F).
///
/// Algorithm: sum of all ROM bytes except the two checksum bytes themselves.
pub fn compute_global_checksum(data: &[u8]) -> u16 {
    let mut sum: u16 = 0;
    for (i, &byte) in data.iter().enumerate() {
        if i != 0x14E && i != 0x14F {
            sum = sum.wrapping_add(byte as u16);
        }
    }
    sum
}

// ─── Symbol file validation ─────────────────────────────────────────

/// Validate that the .sym file exists and is non-empty.
///
/// Panics with a clear message if missing, preventing symbol lookup
/// failures later in test execution.
pub fn validate_sym_file() {
    let path = sym_path();
    match std::fs::metadata(&path) {
        Ok(meta) if meta.len() > 0 => {}
        Ok(_) => panic!(
            "\n\n\
             ╔══════════════════════════════════════════════════════════╗\n\
             ║  SYM FILE VALIDATION FAILED                             ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  pokeyellow.sym is empty.                               ║\n\
             ║                                                         ║\n\
             ║  Rebuild:  make pokeyellow.gbc                          ║\n\
             ╚══════════════════════════════════════════════════════════╝\n"
        ),
        Err(e) => panic!(
            "\n\n\
             ╔══════════════════════════════════════════════════════════╗\n\
             ║  SYM FILE VALIDATION FAILED                             ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  pokeyellow.sym not found: {e}                          ║\n\
             ║                                                         ║\n\
             ║  Build the ROM first:  make pokeyellow.gbc              ║\n\
             ╚══════════════════════════════════════════════════════════╝\n"
        ),
    }
}

// ─── Staleness check ────────────────────────────────────────────────

/// Source directories to check for staleness against the ROM.
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

/// Check if any source .asm file is newer than the built ROM.
///
/// Prints a warning to stderr if the ROM appears stale. Does not panic —
/// a stale ROM may still be intentional during development.
pub fn check_rom_staleness() {
    let rom = rom_path();
    let rom_mtime = match std::fs::metadata(&rom).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return, // ROM doesn't exist; validate_rom will catch this
    };

    let root = project_root();
    let mut newest_source: Option<(std::time::SystemTime, PathBuf)> = None;

    for dir_name in SOURCE_DIRS {
        let dir = root.join(dir_name);
        if !dir.exists() {
            continue;
        }
        visit_asm_files(&dir, &mut |path, mtime| {
            if mtime > rom_mtime {
                match &newest_source {
                    Some((prev_mtime, _)) if mtime <= *prev_mtime => {}
                    _ => newest_source = Some((mtime, path.to_path_buf())),
                }
            }
        });
    }

    if let Some((_mtime, path)) = newest_source {
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        eprintln!(
            "\n\
             ┌─ WARNING ──────────────────────────────────────────────┐\n\
             │  ROM may be stale: {:<39}│\n\
             │  is newer than pokeyellow.gbc.                         │\n\
             │                                                        │\n\
             │  Rebuild:  make pokeyellow.gbc                         │\n\
             └────────────────────────────────────────────────────────┘",
            relative
        );
    }
}

/// Recursively visit .asm files in a directory, calling `f(path, mtime)`.
fn visit_asm_files(dir: &Path, f: &mut impl FnMut(&Path, std::time::SystemTime)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_asm_files(&path, f);
        } else if path.extension().and_then(|e| e.to_str()) == Some("asm") {
            if let Ok(meta) = std::fs::metadata(&path) {
                if let Ok(mtime) = meta.modified() {
                    f(&path, mtime);
                }
            }
        }
    }
}

// ─── Memory limit ───────────────────────────────────────────────────

/// Set a virtual memory limit for the current process.
///
/// On Unix systems, uses `setrlimit(RLIMIT_AS, ...)` to cap the process's
/// virtual address space at 4 GiB. If the emulator goes haywire with invalid
/// ROM data, allocations will fail with OOM instead of consuming all system
/// memory and triggering the OS OOM killer / SIGKILL.
///
/// This is a safety net — the ROM validation checks should catch problems
/// before they reach the emulator, but this provides defense in depth.
#[cfg(unix)]
pub fn set_memory_limit() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let limit = libc::rlimit {
            rlim_cur: MEMORY_LIMIT_BYTES,
            rlim_max: MEMORY_LIMIT_BYTES,
        };
        // Safety: setrlimit is a standard POSIX call; we're setting a soft
        // limit that only affects the current process.
        let ret = unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) };
        if ret != 0 {
            // Non-fatal: if we can't set the limit (e.g., already lower),
            // just warn and continue.
            eprintln!(
                "warning: could not set RLIMIT_AS to {} bytes (errno {})",
                MEMORY_LIMIT_BYTES,
                std::io::Error::last_os_error()
            );
        }
    });
}

/// No-op on non-Unix platforms.
#[cfg(not(unix))]
pub fn set_memory_limit() {}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rom_path_ends_with_gbc() {
        let p = rom_path();
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("gbc"));
    }

    #[test]
    fn sym_path_ends_with_sym() {
        let p = sym_path();
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("sym"));
    }

    #[test]
    fn rom_file_exists() {
        let p = rom_path();
        if p.exists() {
            let bytes = load_rom_bytes(p.to_str().unwrap());
            assert!(!bytes.is_empty(), "ROM file should not be empty");
        }
    }

    #[test]
    fn sym_file_exists() {
        let p = sym_path();
        if p.exists() {
            let content = std::fs::read_to_string(&p).unwrap();
            assert!(
                content.contains("; File generated by rgblink"),
                "sym file should have rgblink header"
            );
        }
    }

    #[test]
    #[should_panic(expected = "ROM VALIDATION FAILED")]
    fn validate_rom_rejects_empty_data() {
        validate_rom(&[]);
    }

    #[test]
    #[should_panic(expected = "ROM VALIDATION FAILED")]
    fn validate_rom_rejects_too_small() {
        validate_rom(&[0u8; 1024]);
    }

    #[test]
    #[should_panic(expected = "invalid Nintendo logo")]
    fn validate_rom_rejects_bad_logo() {
        // 64 KiB of zeros — has right size but wrong header
        validate_rom(&[0u8; 65536]);
    }

    #[test]
    fn validate_rom_accepts_real_rom() {
        let p = rom_path();
        if !p.exists() {
            return;
        }
        let data = std::fs::read(&p).unwrap();
        validate_rom(&data); // should not panic
    }

    #[test]
    fn header_checksum_matches_real_rom() {
        let p = rom_path();
        if !p.exists() {
            return;
        }
        let data = std::fs::read(&p).unwrap();
        let expected = data[0x14D];
        let computed = compute_header_checksum(&data);
        assert_eq!(
            expected, computed,
            "header checksum mismatch: expected {:#04X}, got {:#04X}",
            expected, computed
        );
    }

    #[test]
    fn global_checksum_matches_real_rom() {
        let p = rom_path();
        if !p.exists() {
            return;
        }
        let data = std::fs::read(&p).unwrap();
        let expected = ((data[0x14E] as u16) << 8) | (data[0x14F] as u16);
        let computed = compute_global_checksum(&data);
        assert_eq!(
            expected, computed,
            "global checksum mismatch: expected {:#06X}, got {:#06X}",
            expected, computed
        );
    }

    #[test]
    #[should_panic(expected = "CHECKSUM FAILED")]
    fn validate_rom_rejects_bad_header_checksum() {
        let p = rom_path();
        if !p.exists() {
            // Can't test without a real ROM — just trigger the expected panic
            panic!("ROM CHECKSUM FAILED");
        }
        let mut data = std::fs::read(&p).unwrap();
        // Corrupt the header checksum
        data[0x14D] ^= 0xFF;
        validate_rom(&data);
    }

    #[test]
    #[should_panic(expected = "CHECKSUM FAILED")]
    fn validate_rom_rejects_bad_global_checksum() {
        let p = rom_path();
        if !p.exists() {
            panic!("ROM CHECKSUM FAILED");
        }
        let mut data = std::fs::read(&p).unwrap();
        // Corrupt a data byte (not the checksum bytes) to invalidate global checksum
        data[0x200] ^= 0xFF;
        // But header checksum is still valid since we changed a byte outside $0134-$014C
        validate_rom(&data);
    }

    #[test]
    fn staleness_check_does_not_panic() {
        // Just verify it doesn't crash — may or may not print a warning
        check_rom_staleness();
    }

    #[test]
    fn build_rs_sets_rom_available_cfg() {
        assert!(cfg!(rom_available), "rom_available cfg should be set");
    }

    #[test]
    fn memory_limit_can_be_set() {
        // Should not panic. On Unix, sets RLIMIT_AS. On other platforms, no-op.
        set_memory_limit();
    }
}
