# Emulator-Based Test Harness Design

A Rust-based test harness for verifying the custom features in this Pokemon Yellow fork by running the compiled ROM inside a Game Boy emulator, controlling inputs, inspecting memory, and asserting expected behavior.

## Table of Contents

1. [Goals and Scope](#goals-and-scope)
2. [Architecture Overview](#architecture-overview)
3. [Emulator Library Selection](#emulator-library-selection)
4. [Boytacean Full API Reference](#boytacean-full-api-reference)
5. [Memory Map Reference](#memory-map-reference)
6. [RNG Control Strategy](#rng-control-strategy)
7. [Test Scenarios](#test-scenarios)
   - [Tier 1: Core Feature Tests](#tier-1-core-feature-tests) (Scenarios 1-10)
   - [Tier 2: Visual, State & Performance Tests](#tier-2-visual-state--performance-tests) (Scenarios 11-15)
   - [Tier 3: Audio & Link Regression](#tier-3-audio--link-regression) (Scenarios 16-18)
8. [Test Infrastructure](#test-infrastructure)
9. [CI Integration](#ci-integration)
10. [Implementation Plan](#implementation-plan)
11. [Open Questions and Risks](#open-questions-and-risks)

---

## Goals and Scope

### What We're Testing

This fork has 6 feature groups, each touching completely separate files:

| Feature | Files Modified | Testable Via Emulator? |
|---------|---------------|----------------------|
| Accuracy fix (1/256 miss bug + optimal rounding) | `engine/battle/core.asm` | Yes (primary target) |
| WARP text speed | `constants/ram_constants.asm`, `home/print_text.asm`, `engine/menus/options.asm` | Yes |
| Save delay removal | `engine/menus/save.asm` | Partially (timing) |
| Trade evolution → level evolution | `data/pokemon/evos_moves.asm` | Yes |
| Wild Pokemon additions | `data/wild/maps/*.asm` | Yes (encounter tables) |
| Pikachu learns Surf | `data/pokemon/base_stats/pikachu.asm` | Yes |
| "PRESENTS" intro text | `engine/movie/intro.asm` | Yes (tile inspection) |

### Scope Tiers

Testing capabilities are organized into tiers of increasing ambition. Each tier builds on the infrastructure of the previous one. All tiers are feasible with boytacean's API.

#### Tier 1 — Core Feature Tests

Unit-style tests for each of our fork's 6 feature groups. These are the primary deliverable.

- Accuracy rounding correctness (1/256 miss bug fix + optimal rounding)
- WARP text speed (immediate return from PrintLetterDelay)
- Save delay removal (skip prompt, reduced continue delay)
- Trade evolution → level 36 evolution (data verification)
- Wild Pokemon encounter tables (all 16 additions)
- Pikachu Surf learnset (HM03 in tmhm bitfield)
- "PRESENTS" intro text (tile data in VRAM)

#### Tier 2 — Visual, State & Performance Tests

Tests that leverage boytacean's rich rendering, save state, and cycle-counting APIs.

- **Golden-image screenshots** — capture framebuffer at key moments (PRESENTS subtitle, options menu with WARP, battle screen), compare against reference PNGs. Follows boytacean's own `compare_images()` / `save_image()` test pattern.
- **Save file integrity** — save → read SRAM via `ram_data_eager()` → reload ROM with `set_ram_data()` → verify data survives round-trip. Also test via `StateManager::save()`/`load()` for full state snapshots.
- **Cycle-count benchmarks** — measure exact CPU cycles for key routines. Verify WARP < FAST < MEDIUM < SLOW cycle counts. Measure save routine timing.
- **Scripted game navigation** — joypad automation (`key_press`/`key_lift` + `next_frame()`) to reach menus, trigger battles, navigate maps. Required infrastructure for Tier 3.

#### Tier 3 — Audio & Link Regression

Advanced tests using boytacean's APU and serial subsystems.

- **Audio smoke tests** — verify `audio_output() > 0` during title screen, check per-channel output levels during battle SFX. Uses boytacean's `audio_ch1_output()`..`audio_ch4_output()` and `audio_buffer()`.
- **Link cable regression** — bridge two `GameBoy` instances via custom `SerialDevice` implementation. Verify link battle handshake still works (important: `wLinkState` is checked 22+ times in `core.asm`). Confirm our accuracy fix doesn't break the link battle code path.
- **Extended scripted sequences** — navigate to specific wild encounter areas and verify the encounter, trigger evolution at level 36 in emulator.

#### Tier 4 — Future (Out of Scope for v1)

- Full end-to-end game playthroughs (massive scripting effort, diminishing returns for surgical fork changes)
- Pixel-perfect cross-emulator rendering comparison
- Audio waveform fidelity analysis (comparing APU output against hardware recordings)

### Design Principles

1. **Deterministic**: All tests must be reproducible (controlled RNG)
2. **Fast**: Tests should complete in seconds, not minutes
3. **Headless**: No GUI required (CI-compatible)
4. **Minimal ROM patching**: Use memory writes to set up state rather than modifying the ROM
5. **Focused**: Test our modifications, not the upstream engine

---

## Architecture Overview

```
┌────────────────────────────────────────────────────────────────┐
│                       Rust Test Binary                         │
│                                                                │
│  ┌───────────────────┐  ┌──────────────────────────────────┐  │
│  │  Test Cases        │  │   Harness Library                │  │
│  │                    │  │                                  │  │
│  │  Tier 1: Core      │  │  - ROM loader + symbol resolver  │  │
│  │   accuracy, warp,  │  │  - Memory read/write             │  │
│  │   evolutions, wild │  │  - CPU step/run-to-PC            │  │
│  │                    │  │  - Register injection             │  │
│  │  Tier 2: Visual    │  │  - Input scripting (joypad DSL)  │  │
│  │   golden images,   │  │  - Screenshot capture + compare  │  │
│  │   save integrity,  │  │  - Save state snapshot/restore   │  │
│  │   cycle benchmarks │  │  - Cycle counting                │  │
│  │                    │  │  - SRAM read/write                │  │
│  │  Tier 3: Advanced  │  │  - Audio channel inspection      │  │
│  │   audio, link,     │  │  - Link cable bridge             │  │
│  │   extended scripts │  │                                  │  │
│  └──────┬─────────────┘  └───────────┬──────────────────────┘  │
│         │                            │                         │
│         └────────────┬───────────────┘                         │
│                      │                                         │
│         ┌────────────▼────────────┐                            │
│         │  boytacean (Rust crate) │                            │
│         │                         │                            │
│         │  GameBoy, Ppu, Apu,     │                            │
│         │  Pad, Serial, State,    │                            │
│         │  Cpu, Mmu               │                            │
│         └─────────────────────────┘                            │
└────────────────────────────────────────────────────────────────┘
          │                    │
          ▼                    ▼
    pokeyellow.gbc       pokeyellow.sym
    (built ROM)          (symbol table)
```

### Key Components

1. **Emulator Core** (`boytacean` crate): Rust Game Boy emulator with full subsystem access
2. **Harness Library** (`src/harness.rs`): Wrapper providing high-level APIs
   - `load_rom(path)` → `TestHarness`
   - `run_until_pc(addr)` → step CPU until program counter reaches address
   - `read_mem(addr)` / `write_mem(addr, val)` → WRAM/HRAM access
   - `press(key, frames)` / `hold(key, frames)` → joypad input scripting
   - `run_frames(n)` → advance N frames
   - `capture_screenshot()` → RGB framebuffer snapshot
   - `compare_screenshot(reference_path)` → golden-image comparison
   - `cycle_count(from_pc, to_pc)` → measure CPU cycles between addresses
   - `save_sram()` / `load_sram(data)` → SRAM round-trip
   - `save_state()` / `load_state(data)` → full emulator state snapshot
3. **Input Script Builder** (`src/input.rs`): DSL for button press sequences
4. **Golden Image Module** (`src/golden.rs`): Screenshot capture, PNG encode/decode, pixel diff
5. **Link Bridge** (`src/link.rs`): `SerialDevice` implementation bridging two `GameBoy` instances
6. **Symbol Loader** (`src/symbols.rs`): Parses `pokeyellow.sym` to resolve labels to addresses
7. **Test Cases**: Standard `#[test]` functions organized by tier

---

## Emulator Library Selection

### Requirements

| Requirement | Priority | Notes |
|-------------|----------|-------|
| Headless execution | Must-have | No SDL/GPU dependencies |
| Memory read/write | Must-have | Arbitrary WRAM/HRAM access |
| CPU stepping | Must-have | Step-by-instruction or run-to-PC |
| Breakpoints | Nice-to-have | Can be simulated via stepping |
| Rust crate (library) | Must-have | Embeddable as `cargo` dependency |
| Game Boy Color support | Must-have | Our ROM is `.gbc` |
| Active maintenance | Nice-to-have | For bug fixes |
| Accuracy | Nice-to-have | Don't need cycle-accurate for our tests |

### Candidates

#### Rust-Native Crates

| Crate | Version | GBC? | Memory API | CPU Step | Breakpoints | License | Notes |
|-------|---------|------|-----------|----------|-------------|---------|-------|
| **boytacean** | v0.11.5 | Yes | `read_memory`/`write_memory` | `clock()`/`step_to(addr)` | No (use `step_to`) | Apache-2.0 | **Top pick.** MBC5 confirmed. Published on crates.io, active (Mar 2025). |
| **safeboy** | v0.2.1 | Yes | `read_memory`/`write_memory`/`get_direct_access` | `run()`/`run_frame()` | No | GPL-3.0 | SameBoy Rust bindings. Most accurate. Memory callbacks. Small project (25 commits). |
| **rgy** | v0.1.0 | No | Via `Mmu` in Debugger | `sched()` callback | Partial (`Debugger`) | MIT | `no_std` library design. No GBC = dealbreaker. |
| **gameroy** | N/A | No (DMG) | Internal | `step`/`stepback` | Yes (r/w/jump/exec) | Apache-2.0/MIT | Best debugger but DMG-only. Not on crates.io. |
| **mooneye-gb** | N/A | No | Internal | Internal | No | GPL-3.0+ | Research project, inactive. Not a library crate. |
| **rboy** | N/A | Yes | No API | No | No | MIT | Binary only, not a library. |

#### C/C++ via FFI

| Library | GBC? | Memory API | CPU Step | Breakpoints | Headless | Notes |
|---------|------|-----------|----------|-------------|----------|-------|
| **Gambatte** | Yes | `cpuread`/`cpuwrite` | `runfor` | No | Yes | Pokemon Yellow specifically verified by [gambatte-speedrun](https://github.com/pokemon-speedrunning/gambatte-speedrun). C++ FFI. |
| **mGBA** | Yes | `mCore` vtable | `core->step` | Yes (debugger) | Yes (libmgba) | Richest debugger. `bindgen`-friendly C API. ~50 function pointers. |
| **SameBoy** | Yes | C API | `GB_run` | Yes (debugger) | Yes (`make lib`) | Most accurate. `Tester/main.c` is a direct headless testing template. Pure C. |

#### Boytacean API (Recommended)

```rust
use boytacean::gb::{GameBoy, GameBoyMode};

// Construction (Pokemon Yellow is a DMG game with GBC color enhancements)
let mut gb = GameBoy::new(Some(GameBoyMode::Dmg));
gb.load(true).unwrap();
gb.load_rom(rom_bytes, None).unwrap();

// Execution
gb.clock();                           // One CPU instruction, returns cycles
gb.clock_many(count);                 // N instructions
gb.step_to(0x6797);                   // Run until PC == addr (breakpoint equivalent)
gb.next_frame();                      // One full frame

// Memory access
let val = gb.read_memory(0xD05E);     // Read wMoveMissed
gb.write_memory(0xD12A, 0x04);       // Write wLinkState

// State inspection
let regs = gb.registers();            // CPU registers (A, B, C, D, E, H, L, F, SP, PC)
let cpu = gb.cpu();                   // Direct CPU access
let mmu = gb.mmu();                   // Direct MMU access
```

#### Comparison Notes

- **boytacean** checks all boxes: published crate, GBC, MBC5, memory R/W, `step_to(addr)`, Apache-2.0. Not cycle-accurate but sufficient for game logic testing.
- **safeboy** has SameBoy's accuracy but GPL-3.0, small project, no single-instruction stepping.
- **Gambatte** is the Pokemon speedrunning community's trusted emulator (gambatte-speedrun has Pokemon Yellow TAS verification), but C++ FFI is painful.
- **SameBoy** has the best `Tester/main.c` reference for building headless harnesses, and `make lib` produces a linkable C library.
- **mGBA** has the richest debugger API (`mCore->step`, breakpoints, watchpoints) but C FFI effort is high.

### Recommendation

**Primary**: **boytacean** (`cargo add boytacean`) — only Rust crate that checks all boxes: GBC, MBC5, memory R/W, `step_to`, published, Apache-2.0, active.

**Accuracy fallback**: **safeboy** (SameBoy bindings) — if boytacean has accuracy issues. GPL-3.0 constraint.

**Production fallback**: **SameBoy C library** via `bindgen` — `make lib` produces `libsameboy.a`, use `Tester/main.c` as reference. Most accurate, pure C (easy FFI).

**Not recommended**: mGBA (high FFI effort), Gambatte (C++ FFI pain), mooneye-gb/rgy (no GBC), writing our own.

### Boytacean Integration

```toml
# tests/Cargo.toml
[dependencies]
boytacean = "0.11"
```

No C/C++ build dependencies, no vendor submodules, no `bindgen`. Just `cargo add boytacean` and start writing tests.

### SameBoy FFI Fallback (If Accuracy Matters)

```rust
// Using safeboy crate (wraps SameBoy)
use safeboy::{Gameboy, Model};

let mut gb = Gameboy::new(Model::CgbC); // or Model::DmgB for DMG mode
gb.load_rom_from_slice(&rom_bytes).unwrap();
gb.run_frame(); // advance one frame
let val = gb.read_memory(0xD05E); // read wMoveMissed
gb.write_memory(0xCFD5, 242); // write wPlayerMoveAccuracy
}
```

---

## Boytacean Full API Reference

All signatures verified from source at `/Users/me/aaa/github/boytacean/src/` (v0.11.5).

### Construction & ROM Loading

```rust
use boytacean::gb::{GameBoy, GameBoyMode};
use boytacean::pad::PadKey;

let mut gb = GameBoy::new(Some(GameBoyMode::Dmg));
gb.load(true).unwrap();                              // Initialize to post-boot state
gb.load_rom(&rom_bytes, None).unwrap();              // Load ROM data
gb.load_rom_file("game.gb", Some("game.sav")).unwrap(); // Load from files
gb.load_rom_empty().unwrap();                        // 32KB of NOPs (testing)
gb.reload().unwrap();                                // Reset + reload current ROM
GameBoy::verify_rom(&data);                          // Static validation
```

### CPU Stepping & Execution

```rust
gb.clock()                      -> u16   // One CPU instruction, returns cycles
gb.clock_many(count)            -> u16   // N instructions, clock peripherals once at end
gb.clock_step(addr)             -> u16   // Single step; returns early if PC matches addr
gb.clocks(count)                -> u64   // N instructions with peripheral sync, returns total cycles
gb.clocks_cycles(limit)         -> u64   // Run until cycle budget exhausted
gb.next_frame()                 -> u32   // Run until VBlank, returns cycles consumed
gb.step_to(addr)                -> u32   // Run until PC == addr (breakpoint equivalent)
gb.clocks_frame_buffer(limit)   -> ClockFrame // Run + capture framebuffer at VBlank
```

### Memory Access

```rust
gb.read_memory(addr: u16)       -> u8    // Read any address (WRAM, HRAM, IO, ROM)
gb.write_memory(addr: u16, val: u8)      // Write any address
gb.mmu().read_many(addr, len)   -> Vec<u8>  // Block read
gb.mmu().write_many(addr, &data)            // Block write
gb.vram_eager()                 -> Vec<u8>  // VRAM copy
gb.hram_eager()                 -> Vec<u8>  // HRAM copy
```

### CPU Register Access

```rust
gb.cpu_i().pc() -> u16    // Program counter (immutable)
gb.cpu_i().sp() -> u16    // Stack pointer
gb.cpu_i().a()  -> u8     // Accumulator
gb.cpu_i().af() -> u16    // AF pair (+ flags)
gb.cpu_i().bc() -> u16    // BC, DE, HL pairs also available
gb.cpu_i().ime() -> bool  // Interrupt master enable
gb.cpu_i().halted() -> bool

gb.cpu().set_pc(val: u16)  // Mutable register setters
gb.cpu().set_sp(val: u16)
gb.cpu().set_a(val: u8)    // All registers have set_ variants
gb.cpu().set_b(val: u8)
gb.cpu().set_af(val: u16)  // Also set_bc, set_de, set_hl

gb.registers() -> Registers  // Combined CPU+PPU snapshot
// Registers { pc, sp, a, b, c, d, e, h, l, scy, scx, wy, wx, ly, lyc }
```

### Joypad Input

```rust
use boytacean::pad::PadKey;
// PadKey variants: Up, Down, Left, Right, Start, Select, A, B

gb.key_press(PadKey::A);      // Press button (sets joypad interrupt flag)
gb.key_lift(PadKey::A);       // Release button
gb.pad().key_press(key);      // Direct Pad access (same effect)
gb.pad().key_lift(key);
gb.pad_i().int_pad() -> bool; // Check if interrupt pending
```

### Framebuffer & Screenshots

```rust
// RGB888 format: 160×144×3 = 69,120 bytes
gb.frame_buffer()              -> &[u8; 69120]   // RGB888 (lazy-rendered)
gb.frame_buffer_eager()        -> Vec<u8>         // Owned RGB888 copy
gb.frame_buffer_raw()          -> [u8; 69120]     // Color indices (0-3 per pixel)
gb.frame_buffer_raw_eager()    -> Vec<u8>         // Owned raw copy

// Other pixel formats
gb.frame_buffer_xrgb8888()     -> [u8; 92160]     // XRGB8888
gb.frame_buffer_xrgb8888_u32() -> [u32; 23040]    // XRGB8888 as u32
gb.frame_buffer_rgb1555()      -> [u8; 46080]     // RGB1555
gb.frame_buffer_rgb565()       -> [u8; 46080]     // RGB565

// Frame capture at VBlank timing
let cf: ClockFrame = gb.clocks_frame_buffer(70224);
cf.frame_buffer_eager() -> Option<Vec<u8>>

// Constants
boytacean::ppu::DISPLAY_WIDTH  = 160
boytacean::ppu::DISPLAY_HEIGHT = 144
boytacean::ppu::FRAME_BUFFER_SIZE = 69120
```

### PPU / Graphics Internals

```rust
// Public fields on Ppu (direct access)
gb.ppu().color_buffer -> Box<[u8; 23040]>   // Color IDs 0-3 per pixel (pub)
gb.ppu().shade_buffer -> Box<[u8; 23040]>   // Shade values 0-3 (pub, DMG)

// VRAM, OAM, HRAM access
gb.ppu_i().vram()     -> &[u8; 16384]       // Full VRAM (CGB-sized)
gb.ppu_i().vram_dmg() -> &[u8]              // First 8KB
gb.ppu_i().oam()      -> &[u8; 160]         // 40 sprites × 4 bytes
gb.ppu_i().hram()     -> &[u8; 128]         // High RAM

// Tile inspection
gb.ppu_i().tiles()    -> &[Tile; 768]       // All decoded tiles
gb.get_tile(index)    -> Tile               // Single tile (8×8 pixels)
gb.get_tile_buffer(i) -> Vec<u8>            // Tile as RGB with current palette
// Tile: .get(x,y)->u8, .buffer()->Vec<u8>, .palette_buffer(palette)->Vec<u8>

// Palettes
gb.ppu_i().palette_bg()    -> Palette       // Background palette
gb.ppu_i().palette_obj_0() -> Palette       // Sprite palette 0
gb.ppu_i().palette_obj_1() -> Palette       // Sprite palette 1
gb.ppu_i().palettes_color() -> &[[u8; 64]; 2] // CGB color palettes

// State
gb.ppu_ly()    -> u8       // Current scanline (LY register)
gb.ppu_mode()  -> PpuMode  // HBlank, VBlank, OamRead, VramRead
gb.ppu_frame() -> u16      // Frame counter
```

### Audio / APU

```rust
// Combined output
gb.audio_output()      -> u16           // Sum of all 4 channels

// Per-channel output levels (0-255 each)
gb.audio_ch1_output()  -> u8            // Channel 1 (square + sweep)
gb.audio_ch2_output()  -> u8            // Channel 2 (square)
gb.audio_ch3_output()  -> u8            // Channel 3 (wave)
gb.audio_ch4_output()  -> u8            // Channel 4 (noise)
gb.audio_all_output()  -> Vec<u16>      // [mixed, ch1, ch2, ch3, ch4]

// PCM sample buffer (i16, configurable sample rate)
gb.audio_buffer()      -> &VecDeque<i16> // Stereo PCM samples
gb.audio_buffer_eager(clear: bool) -> Vec<i16> // Owned copy, optionally drain

// Channel enable/disable (for isolation testing)
gb.set_audio_ch1_enabled(bool)
gb.set_audio_ch2_enabled(bool)
gb.set_audio_ch3_enabled(bool)
gb.set_audio_ch4_enabled(bool)

// Configuration
gb.audio_sampling_rate() -> u16         // e.g., 44100
gb.audio_channels()      -> u8          // 1 (mono) or 2 (stereo)

// Raw APU register access
gb.apu().read(addr)      -> u8          // NR1x-NR5x registers
gb.apu().write(addr, val)               // Write APU register
gb.apu().read_raw(addr)  -> u8          // Raw register (no side effects)
gb.apu().write_raw(addr, val)
```

### Serial / Link Cable

```rust
use boytacean::serial::SerialDevice;

// The trait to implement for custom link cable devices
pub trait SerialDevice {
    fn send(&mut self) -> u8;            // Called when GB sends a byte; return byte to receive
    fn receive(&mut self, byte: u8);     // Called when transfer completes
    fn allow_slave(&self) -> bool;       // External clock source support
    fn description(&self) -> String;
    fn state(&self) -> String;
}

// Built-in devices
gb.attach_null_serial();                 // NullDevice (no-op, returns 0xFF)
gb.attach_stdout_serial();               // StdoutDevice (prints received bytes)
gb.attach_printer_serial();              // PrinterDevice (GB Printer protocol)
gb.attach_serial(Box::new(device));      // Custom device

// Direct serial struct access
gb.serial().set_device(Box::new(device));
gb.serial_i().device() -> &dyn SerialDevice;
gb.serial_i().transferring() -> bool;

// BufferDevice (for capturing serial output in tests)
use boytacean::devices::buffer::BufferDevice;
let mut dev = BufferDevice::new();
dev.buffer() -> &Vec<u8>;               // Accumulated received bytes
```

### Save States (Full Emulator Snapshot)

```rust
use boytacean::state::{StateManager, SaveStateFormat, FromGbOptions, ToGbOptions};

// Save to bytes (BOSC = compressed, BOS = uncompressed, BESS = SameBoy-compatible)
let data: Vec<u8> = StateManager::save(&mut gb, Some(SaveStateFormat::Bosc), None)?;

// Load from bytes
StateManager::load(&data, &mut gb, None, None)?;

// Save/load to files
StateManager::save_file("state.bosc", &mut gb, Some(SaveStateFormat::Bosc), None)?;
StateManager::load_file("state.bosc", &mut gb, None, None)?;

// Options
FromGbOptions { thumbnail: bool, state_format, agent, agent_version }
ToGbOptions { reload: bool, devices: bool }
```

### SRAM (Cartridge Save Data)

```rust
gb.ram_data_eager()         -> Vec<u8>   // Read SRAM (battery-backed save)
gb.set_ram_data(data: Vec<u8>)           // Write SRAM

// Via Cartridge
gb.cartridge_i().ram_data() -> &Vec<u8>
gb.cartridge().set_ram_data(&data);
gb.cartridge().clear_ram_data();
gb.cartridge_i().has_battery() -> bool;
```

### Peripheral Enable/Disable (Headless Optimization)

```rust
gb.set_ppu_enabled(false);   // Skip PPU rendering (faster headless execution)
gb.set_apu_enabled(false);   // Skip audio processing
gb.set_dma_enabled(false);
gb.set_timer_enabled(false);
gb.set_serial_enabled(false);
gb.set_all_enabled(false);   // Disable all peripherals at once
```

### Existing Test Utilities

Boytacean ships with test helpers in `boytacean::test`:

```rust
use boytacean::test::{build_test, run_test, run_serial_test, run_image_test, run_step_test, TestOptions};

let opts = TestOptions { mode: None, ppu_enabled: Some(true), ..Default::default() };
let gb = run_test("rom.gb", Some(1_000_000), opts)?;                      // Run for N cycles
let gb = run_step_test("rom.gb", 0x0100, opts)?;                          // Run until PC
let (serial_str, gb) = run_serial_test("rom.gb", Some(300_000_000), opts)?; // Capture serial output
let (framebuf, gb) = run_image_test("rom.gb", Some(50_000_000), opts)?;    // Capture framebuffer
```

The SDL frontend also has golden-image comparison (`frontends/sdl/src/test.rs`):

```rust
// compare_images(source_pixels: &[u8], target_path: &str) -> bool
// save_image(pixels: &[u8], width: u32, height: u32, file_path: &str)
```

### Constants

```rust
GameBoy::CPU_FREQ    = 4_194_304  // 4.194304 MHz
GameBoy::VISUAL_FREQ = 59.7275    // ~60 Hz refresh rate
GameBoy::LCD_CYCLES  = 70_224     // CPU cycles per frame
```

---

## Memory Map Reference

All addresses from `pokeyellow.sym` (built with RGBDS v1.0.1).

### Battle System

| Symbol | Address | Size | Description |
|--------|---------|------|-------------|
| `wPlayerMoveAccuracy` | `$CFD5` | 1 | Player's move accuracy (after CalcHitChance) |
| `wEnemyMoveAccuracy` | `$CFCF` | 1 | Enemy's move accuracy |
| `wMoveMissed` | `$D05E` | 1 | 0 = hit, 1 = missed |
| `wDamage` | `$D0D6` | 2 | Damage dealt (big-endian) |
| `hWhoseTurn` | `$FFF3` | 1 | 0 = player's turn, 1 = enemy's turn |

### RNG

| Symbol | Address | Size | Description |
|--------|---------|------|-------------|
| `hRandomAdd` | `$FFD3` | 1 | Hardware RNG accumulator (add) |
| `hRandomSub` | `$FFD4` | 1 | Hardware RNG accumulator (sub) |
| `wLinkState` | `$D12A` | 1 | Link state ($04 = LINK_STATE_BATTLING) |
| `wLinkBattleRandomNumberList` | `$D147` | 10 | Pre-shared RNG list (SERIAL_RNS_LENGTH=10) |
| `wLinkBattleRandomNumberListIndex` | `$CCDE` | 1 | Current index into RNG list |

### Text Speed / Options

| Symbol | Address | Size | Description |
|--------|---------|------|-------------|
| `wOptions` | `$D354` | 1 | Options byte (bits 0-3: text delay, 4-5: sound, 6: battle style, 7: battle anim) |
| `wLetterPrintingDelayFlags` | `$D357` | 1 | Bit 0: fast delay, Bit 1: text delay active |
| `wStatusFlags5` | `$D72F` | 1 | Bit 6: BIT_NO_TEXT_DELAY |

#### Text Delay Constants

| Constant | Value | Binary |
|----------|-------|--------|
| `TEXT_DELAY_WARP` | 0 | `%000` |
| `TEXT_DELAY_FAST` | 1 | `%001` |
| `TEXT_DELAY_MEDIUM` | 3 | `%011` |
| `TEXT_DELAY_SLOW` | 5 | `%101` |
| `TEXT_DELAY_MASK` | `$0F` | `%001111` |

### Key ROM Addresses (Bank:Offset)

| Symbol | Bank:Addr | Description |
|--------|-----------|-------------|
| `MoveHitTest` | `$0F:$6700` | Entry point for accuracy check |
| `MoveHitTest.doAccuracyCheck` | `$0F:$6797` | Our modified accuracy logic |
| `MoveHitTest.accuracyHit` | `$0F:$67A7` | Hit path (ret) |
| `MoveHitTest.moveMissed` | `$0F:$67A8` | Miss path (sets wMoveMissed=1) |
| `BattleRandom` | `$0F:$7038` | RNG function (link vs hardware) |
| `PrintLetterDelay` | `$00:$38AE` | Text delay (ROM0, our WARP check) |
| `SaveMenu` | `$1C:$78A9` | Save flow (our delay removal) |
| `CalcHitChance` | (see .sym) | Accuracy/evasion modifier calculation |

### Serial Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `LINK_STATE_NONE` | `$00` | Normal single-player mode |
| `LINK_STATE_BATTLING` | `$04` | Link battle mode (uses shared RNG list) |
| `SERIAL_RNS_LENGTH` | `10` | Size of wLinkBattleRandomNumberList |

---

## RNG Control Strategy

### The Problem

In single-player mode, `BattleRandom` calls `Random` which uses hardware registers (`hRandomAdd`/`hRandomSub`). `Random` reads `rDIV` (`$FF04`), a free-running hardware timer, making it fundamentally non-deterministic. Additionally, `Random` is called every VBlank (~60Hz), continuously mutating the RNG state. There is no way to seed `hRandomAdd`/`hRandomSub` for deterministic behavior.

### Strategy Evaluation

Five strategies were analyzed. **Strategy C (breakpoint + register injection)** is recommended.

#### Strategy A: Link Battle RNG Hijack — NOT RECOMMENDED

Setting `wLinkState` (`$D12A`) to `$04` (LINK_STATE_BATTLING) forces BattleRandom to use the deterministic list, but this value is checked **22+ times in `engine/battle/core.asm` alone**, plus in `trainer_ai.asm`, `effects.asm`, `experience.asm`, `end_of_battle.asm`, and `battle_transitions.asm`. It would break:
- AI move selection
- Experience gain calculation
- Battle end handling
- Move selection menus
- Enemy mon data loading (`GetCurrentMove` at line 6220)

**Verdict**: Too many side effects for anything beyond isolated `doAccuracyCheck` testing.

#### Strategy B: Direct hRandomAdd/hRandomSub Write — NOT VIABLE

Cannot work because:
1. `Random_()` reads `rDIV` ($FF04), a hardware free-running counter
2. VBlank calls `Random` every frame (~60 Hz), constantly mutating the state
3. Even writing `hRandomAdd` immediately before the call, `rDIV` makes the result unpredictable

**Verdict**: Fundamentally non-deterministic.

#### Strategy C: Breakpoint + Register Injection — RECOMMENDED

Set a breakpoint at the instruction **immediately after** `call BattleRandom` in the `doAccuracyCheck` section, and force register A to the desired value.

**Breakpoint target**: `$0F:$679E` (the `cp b` instruction, opcode `$B8`)

```asm
; Byte sequence at the doAccuracyCheck block:
0F:6797: 78          ld a, b
0F:6798: FE FF       cp $FF
0F:679A: C8          ret z              ; N=255 always hits
0F:679B: CD 38 70    call BattleRandom  ; ← RNG call
0F:679E: B8          cp b               ; ← BREAKPOINT HERE, inject A
0F:679F: 38 06       jr c, .accuracyHit
0F:67A1: 20 05       jr nz, .moveMissed
0F:67A3: CB 78       bit 7, b
0F:67A5: 28 01       jr z, .moveMissed
0F:67A7:             .accuracyHit (ret)
0F:67A8:             .moveMissed
```

**Implementation with boytacean**:

```rust
fn test_accuracy_check(gb: &mut GameBoy, accuracy: u8, random_value: u8) -> bool {
    // Set up: write accuracy to register B and jump to doAccuracyCheck
    // Method 1: Use step_to to run until the cp b instruction
    gb.write_memory(0xCFD5, accuracy);  // wPlayerMoveAccuracy
    gb.write_memory(0xFFF3, 0x00);      // hWhoseTurn = player

    // Run from .calcHitChance to just after BattleRandom call
    // When PC reaches 0x679E (cp b), inject our random value into register A
    loop {
        gb.clock();
        let regs = gb.registers();
        if regs.pc == 0x679E {  // cp b instruction
            // Inject our desired random value into register A
            gb.cpu().set_a(random_value);
            break;
        }
    }

    // Continue execution until hit or miss
    gb.step_to(0x67A7);  // .accuracyHit
    // OR
    gb.step_to(0x67A8);  // .moveMissed

    // Check result
    gb.read_memory(0xD05E) == 0  // wMoveMissed: 0 = hit, 1 = miss
}
```

**To force a HIT**: Set A = 0. Since 0 < any accuracy (1-255), `cp b` sets carry → `jr c, .accuracyHit`.

**To force a MISS**: Set A = 255. Since 255 > any accuracy (1-254), `jr nz, .moveMissed`.

**Verdict**: Non-invasive, no code changes, no side effects. Requires emulator with register access (boytacean has it).

#### Strategy D: Test-Only ROM Patch (WRAM Flag) — GOOD FOR TEST BUILDS

Add a conditional `IF DEF(_TEST_RNG)` define that makes BattleRandom check a test flag in WRAM instead of `wLinkState`. This redirects to the deterministic list without triggering 22+ link-state checks elsewhere.

```asm
BattleRandom:
IF DEF(_TEST_RNG)
    ld a, [wTestDeterministicRNG]  ; new WRAM flag
    and a
    jr nz, .useDeterministicList
ENDC
    ld a, [wLinkState]
    cp LINK_STATE_BATTLING
    jp nz, Random
.useDeterministicList:
    ; ... rest of link battle RNG code
```

**Verdict**: Cleanest solution for test builds. Requires modifying the ROM source (conditional compile). Only affects the RNG path.

#### Strategy E: NOP Out BattleRandom Call — SIMPLEST FOR ONE-OFFS

Patch 3 bytes at ROM offset `$3E79B`:
- Replace `CD 38 70` (`call BattleRandom`) with `3E XX 00` (`ld a, XX; nop`)
- Where `XX` is the desired random value

**Verdict**: Quick and dirty. Good for manual testing, not suitable for an automated harness that needs different random values per test.

### Recommended Approach

**For the Rust test harness**: Use **Strategy C** (breakpoint + register injection). It requires no ROM modifications, has no side effects, and works with any random value. The implementation is straightforward with boytacean's `clock()` + `cpu()` API.

**For future expansion**: Consider **Strategy D** (test-only ROM flag) if we need to test full battle turns where multiple BattleRandom calls occur and we need to control all of them.

---

## Test Scenarios

### Tier 1: Core Feature Tests

#### 1. Accuracy Check — 1/256 Miss Bug Fix

**What we're testing**: N=255 always hits (never misses).

```rust
#[test]
fn accuracy_255_always_hits() {
    // For all 256 possible random values, a move with accuracy 255
    // should ALWAYS hit
    for random in 0..=255u8 {
        let mut emu = setup_battle_state();
        setup_deterministic_accuracy_check(&mut emu, 255, random);
        run_accuracy_check(&mut emu);
        assert_eq!(emu.read_mem(0xD05E), 0, // wMoveMissed
            "N=255 missed with random={random}");
    }
}
```

#### 2. Accuracy Check — Optimal Rounding (N >= 128)

**What we're testing**: For accuracy values >= 128, when `random == accuracy`, the move should HIT (using `<=` comparison gives closer approximation to N/255).

```rust
#[test]
fn accuracy_high_values_use_lte() {
    // For N >= 128: random == N should be a hit
    for accuracy in 128..=254u8 {
        let mut emu = setup_battle_state();
        setup_deterministic_accuracy_check(&mut emu, accuracy, accuracy);
        run_accuracy_check(&mut emu);
        assert_eq!(emu.read_mem(0xD05E), 0, // wMoveMissed
            "N={accuracy} (>=128) should hit when random==accuracy");
    }
}
```

#### 3. Accuracy Check — Optimal Rounding (N < 128)

**What we're testing**: For accuracy values < 128, when `random == accuracy`, the move should MISS (using `<` comparison gives closer approximation to N/255).

```rust
#[test]
fn accuracy_low_values_use_lt() {
    // For N < 128: random == N should be a miss
    for accuracy in 1..=127u8 {
        let mut emu = setup_battle_state();
        setup_deterministic_accuracy_check(&mut emu, accuracy, accuracy);
        run_accuracy_check(&mut emu);
        assert_eq!(emu.read_mem(0xD05E), 1, // wMoveMissed
            "N={accuracy} (<128) should miss when random==accuracy");
    }
}
```

#### 4. Accuracy Check — Basic Hit/Miss

```rust
#[test]
fn accuracy_random_below_always_hits() {
    // random < accuracy → always hit (regardless of N value)
    for accuracy in 1..=254u8 {
        let random = accuracy - 1; // guaranteed < accuracy
        let mut emu = setup_battle_state();
        setup_deterministic_accuracy_check(&mut emu, accuracy, random);
        run_accuracy_check(&mut emu);
        assert_eq!(emu.read_mem(0xD05E), 0, // wMoveMissed
            "random<accuracy should always hit");
    }
}

#[test]
fn accuracy_random_above_always_misses() {
    // random > accuracy → always miss (regardless of N value)
    for accuracy in 1..=254u8 {
        let random = accuracy + 1; // guaranteed > accuracy (when accuracy < 255)
        if random == 0 { continue; } // overflow guard
        let mut emu = setup_battle_state();
        setup_deterministic_accuracy_check(&mut emu, accuracy, random);
        run_accuracy_check(&mut emu);
        assert_eq!(emu.read_mem(0xD05E), 1, // wMoveMissed
            "random>accuracy should always miss");
    }
}
```

#### 5. Exhaustive Accuracy Check

```rust
#[test]
fn accuracy_exhaustive_all_255_values() {
    // For each possible accuracy value (1-255), verify hit rate
    // matches our expected behavior across all 256 random values
    for accuracy in 1..=255u8 {
        let mut hits = 0u32;
        for random in 0..=255u8 {
            let mut emu = setup_battle_state();
            setup_deterministic_accuracy_check(&mut emu, accuracy, random);
            run_accuracy_check(&mut emu);
            if emu.read_mem(0xD05E) == 0 { hits += 1; }
        }

        let expected_hits = if accuracy == 255 {
            256 // always hits
        } else if accuracy >= 128 {
            (accuracy as u32) + 1 // random <= accuracy
        } else {
            accuracy as u32 // random < accuracy
        };

        assert_eq!(hits, expected_hits,
            "N={accuracy}: got {hits} hits, expected {expected_hits}");
    }
}
```

#### 6. WARP Text Speed

**What we're testing**: When `wOptions & TEXT_DELAY_MASK == 0` (WARP mode), `PrintLetterDelay` returns immediately without any frame delays.

```rust
#[test]
fn warp_text_speed_skips_delay() {
    let mut emu = load_rom_and_init();

    // Set WARP text speed (TEXT_DELAY_WARP = 0)
    let options = emu.read_mem(0xD354); // wOptions
    emu.write_mem(0xD354, options & !0x0F); // clear text delay bits

    // Set flags for text printing
    let delay_flags = emu.read_mem(0xD357);
    emu.write_mem(0xD357, delay_flags | 0x03); // BIT_FAST_TEXT_DELAY | BIT_TEXT_DELAY

    // Clear BIT_NO_TEXT_DELAY in wStatusFlags5
    let flags5 = emu.read_mem(0xD72F);
    emu.write_mem(0xD72F, flags5 & !(1 << 6));

    // Set breakpoint at PrintLetterDelay entry ($00:$38AE)
    // and at .done ($00:$38EC)
    let start_frame = emu.frame_count();
    // Call PrintLetterDelay and measure frames elapsed
    emu.set_pc(0x38AE); // Jump to PrintLetterDelay
    emu.run_until_pc(0x38EC); // .done label
    let elapsed = emu.frame_count() - start_frame;

    assert_eq!(elapsed, 0, "WARP mode should not wait any frames");
}

#[test]
fn non_warp_text_speed_has_delay() {
    // Same setup but with TEXT_DELAY_FAST (1) — should have a delay
    let mut emu = load_rom_and_init();
    let options = emu.read_mem(0xD354);
    emu.write_mem(0xD354, (options & !0x0F) | 0x01); // TEXT_DELAY_FAST

    // ... similar setup and measurement
    // Assert elapsed > 0
}
```

#### 7. Trade Evolution → Level Evolution

**What we're testing**: Kadabra, Graveler, Machoke, and Haunter evolve at level 36 instead of requiring trade.

This can be tested by reading the evolution data directly from ROM rather than running through a full level-up sequence:

```rust
#[test]
fn trade_evolutions_changed_to_level_36() {
    let rom = load_rom_bytes("pokeyellow.gbc");
    let sym = parse_sym_file("pokeyellow.sym");

    let test_cases = [
        ("KadabraEvosMoves", "ALAKAZAM"),
        ("GravelerEvosMoves", "GOLEM"),
        ("MachokeEvosMoves", "MACHAMP"),
        ("HaunterEvosMoves", "GENGAR"),
    ];

    for (label, _expected_evo) in &test_cases {
        let addr = sym.resolve(label);
        // Evolution data format: db EVOLVE_LEVEL, <level>, <species>
        let evo_type = rom[addr];     // Should be EVOLVE_LEVEL (1)
        let evo_level = rom[addr + 1]; // Should be 36
        assert_eq!(evo_type, 1, "{label}: should be EVOLVE_LEVEL");
        assert_eq!(evo_level, 36, "{label}: should evolve at level 36");
    }
}
```

#### 8. Pikachu Learns Surf

**What we're testing**: Pikachu's TM/HM learnset includes HM03 (Surf).

```rust
#[test]
fn pikachu_can_learn_surf() {
    let rom = load_rom_bytes("pokeyellow.gbc");
    let sym = parse_sym_file("pokeyellow.sym");

    // The tmhm learnset is a bitfield after the base stats
    // HM03 (Surf) = TM/HM index 53 (TM50 + HM03)
    // Check if bit 52 (0-indexed) is set in the 7-byte learnset bitfield
    let pikachu_base = sym.resolve("PikachuBaseStats");
    let tmhm_offset = 20; // offset to tmhm learnset in base stats struct
    let learnset_addr = pikachu_base + tmhm_offset;

    // Surf = HM03 = move index 53 in the TM/HM list
    // Bit position: (53-1) = 52 → byte 52/8 = 6, bit 52%8 = 4
    let byte_idx = 6;
    let bit_idx = 4;
    let learnset_byte = rom[learnset_addr + byte_idx];
    assert!(learnset_byte & (1 << bit_idx) != 0,
        "Pikachu should be able to learn Surf (HM03)");
}
```

#### 9. Wild Pokemon Encounter Tables

**What we're testing**: Our added wild Pokemon appear in the correct locations.

```rust
#[test]
fn mew_appears_in_cerulean_cave_b1f() {
    let rom = load_rom_bytes("pokeyellow.gbc");
    let sym = parse_sym_file("pokeyellow.sym");

    let table_addr = sym.resolve("CeruleanCaveB1F_Pokemon");
    // Parse encounter table to verify MEW is present
    // Table format: encounter_rate, then pairs of (level, species)
    let encounter_rate = rom[table_addr];
    assert!(encounter_rate > 0, "CeruleanCaveB1F should have encounters");

    let mut found_mew = false;
    for i in 0..10 {
        let species = rom[table_addr + 1 + i * 2 + 1];
        if species == DEX_MEW {
            found_mew = true;
            break;
        }
    }
    assert!(found_mew, "Mew should be in CeruleanCaveB1F encounters");
}
```

#### 10. Save Delay Removal

**What we're testing**: The save flow skips the "Would you like to save?" prompt and the "Saving..." delay.

This is best tested by verifying the ROM code structure (no `WouldYouLikeToSaveText` call in SaveMenu) or by timing the save flow:

```rust
#[test]
fn save_menu_skips_save_prompt() {
    let rom = load_rom_bytes("pokeyellow.gbc");
    let sym = parse_sym_file("pokeyellow.sym");

    let save_menu = sym.resolve("SaveMenu");
    let save_prompt = sym.resolve("WouldYouLikeToSaveText");

    // Scan SaveMenu code for any reference to WouldYouLikeToSaveText
    // In the original, there's a `ld hl, WouldYouLikeToSaveText` followed by
    // `call SaveTheGame_YesOrNo`. In our version, this is skipped.
    // Verify by checking that the address is NOT loaded in the save flow.
    // (Implementation depends on how the call is encoded)
}
```

### Tier 2: Visual, State & Performance Tests

#### 11. Golden-Image — PRESENTS Subtitle

**What we're testing**: The "PRESENTS" text renders correctly under the Game Freak logo in the intro animation. Verifies the cosmetic change *visually*, not just tile data.

```rust
#[test]
fn presents_subtitle_golden_image() {
    let mut harness = TestHarness::new();

    // Boot the game and advance through intro until Game Freak logo
    // The PRESENTS text appears at hlcoord 7,11 (tiles $67-$6C)
    // Advance frames until we're past the logo animation
    harness.run_frames(300); // ~5 seconds to reach Game Freak logo

    // Wait for the specific frame where PRESENTS text is visible
    // Check VRAM or a known memory flag to detect the right moment
    harness.wait_for_memory(0xFF44, |ly| ly == 0); // Wait for VBlank
    let screenshot = harness.gb.frame_buffer_eager();

    // Compare against golden reference image
    let matches = compare_screenshot(&screenshot, "golden/presents_subtitle.png", 0.99);
    assert!(matches, "PRESENTS subtitle does not match golden image");
}
```

**First run**: Generate reference image with `save_screenshot()`, manually verify it's correct, commit to `tests/golden/`.

#### 12. Golden-Image — Options Menu with WARP

**What we're testing**: The options menu correctly displays "WARP" as the 4th text speed option.

```rust
#[test]
fn options_menu_warp_golden_image() {
    let mut harness = TestHarness::new();

    // Script: navigate to options menu
    // From title screen: press Start, then navigate to Options
    harness.run_frames(600); // Wait for title screen
    harness.press(PadKey::Start, 30);
    harness.run_frames(60);
    // Navigate to Options menu item and press A
    // (exact sequence depends on menu layout)

    // Set text speed to WARP via memory write (faster than menuing)
    let options = harness.read_mem(0xD354);
    harness.write_mem(0xD354, options & !0x0F); // TEXT_DELAY_WARP = 0

    // Enter options screen and capture
    // ... navigate to options screen ...
    let screenshot = harness.gb.frame_buffer_eager();
    let matches = compare_screenshot(&screenshot, "golden/options_warp.png", 0.98);
    assert!(matches, "Options menu WARP display does not match golden image");
}
```

#### 13. Save File Integrity

**What we're testing**: Our save delay removal doesn't corrupt save data. SRAM survives a save → reload round-trip.

```rust
#[test]
fn save_sram_round_trip() {
    let mut harness = TestHarness::new();

    // Set up a known game state in WRAM
    // Player name, badges, Pokedex, party, etc.
    harness.write_mem(0xD158, 0x80); // wPlayerName first byte (end marker)
    harness.write_mem(0xD356, 0xFF); // wObtainedBadges = all 8

    // Trigger save (set up minimal state for SaveGameData to succeed)
    // ... setup save prerequisites ...

    // Capture SRAM after save
    let sram_after_save = harness.gb.ram_data_eager();
    assert!(!sram_after_save.is_empty(), "SRAM should not be empty after save");

    // Reload the ROM with the saved SRAM
    let rom_bytes = std::fs::read("../pokeyellow.gbc").unwrap();
    harness.gb.load_rom(&rom_bytes, Some(&sram_after_save)).unwrap();
    harness.gb.load_boot_state();

    // Read back SRAM and verify it matches
    let sram_after_reload = harness.gb.ram_data_eager();
    assert_eq!(sram_after_save, sram_after_reload,
        "SRAM should survive save → reload round-trip");
}

#[test]
fn save_state_snapshot_restore() {
    use boytacean::state::{StateManager, SaveStateFormat};

    let mut harness = TestHarness::new();
    harness.run_frames(100); // Get to a stable state

    // Write a marker value to WRAM
    harness.write_mem(0xC000, 0x42);
    harness.write_mem(0xC001, 0xAB);

    // Snapshot full emulator state
    let state_data = StateManager::save(&mut harness.gb, Some(SaveStateFormat::Bosc), None)
        .expect("Failed to save state");

    // Mutate state
    harness.write_mem(0xC000, 0x00);
    harness.run_frames(60);

    // Restore snapshot
    StateManager::load(&state_data, &mut harness.gb, None, None)
        .expect("Failed to load state");

    // Verify marker values survived
    assert_eq!(harness.read_mem(0xC000), 0x42);
    assert_eq!(harness.read_mem(0xC001), 0xAB);
}
```

#### 14. WARP Speed Cycle Benchmark

**What we're testing**: WARP text speed is measurably faster than other speed settings. Cycle counts: WARP < FAST < MEDIUM < SLOW.

```rust
#[test]
fn text_speed_cycle_ordering() {
    let delays = [
        ("WARP",   0x00u8), // TEXT_DELAY_WARP
        ("FAST",   0x01u8), // TEXT_DELAY_FAST
        ("MEDIUM", 0x03u8), // TEXT_DELAY_MEDIUM
        ("SLOW",   0x05u8), // TEXT_DELAY_SLOW
    ];

    let mut cycle_counts = Vec::new();

    for (name, delay_value) in &delays {
        let mut harness = TestHarness::new();

        // Set text speed
        let options = harness.read_mem(0xD354);
        harness.write_mem(0xD354, (options & !0x0F) | delay_value);

        // Set up text printing flags so PrintLetterDelay does its work
        harness.write_mem(0xD357, 0x03); // BIT_FAST_TEXT_DELAY | BIT_TEXT_DELAY
        let flags5 = harness.read_mem(0xD72F);
        harness.write_mem(0xD72F, flags5 & !(1 << 6)); // clear BIT_NO_TEXT_DELAY

        // Set PC to PrintLetterDelay entry ($00:$38AE)
        harness.gb.cpu().set_pc(0x38AE);

        // Count cycles until .done
        let mut cycles: u64 = 0;
        loop {
            let c = harness.gb.clock();
            cycles += c as u64;
            if harness.gb.cpu_i().pc() == 0x38EC { break; } // .done
            if cycles > 1_000_000 { panic!("{name} exceeded cycle budget"); }
        }

        println!("{name}: {cycles} cycles");
        cycle_counts.push((*name, cycles));
    }

    // Assert strict ordering: WARP < FAST < MEDIUM < SLOW
    for i in 0..cycle_counts.len() - 1 {
        assert!(cycle_counts[i].1 < cycle_counts[i + 1].1,
            "{} ({} cycles) should be faster than {} ({} cycles)",
            cycle_counts[i].0, cycle_counts[i].1,
            cycle_counts[i + 1].0, cycle_counts[i + 1].1);
    }
}
```

#### 15. Save Routine Timing

**What we're testing**: The save flow completes faster than the upstream version (no prompt, reduced continue delay).

```rust
#[test]
fn save_menu_timing() {
    let mut harness = TestHarness::new();

    // Set up minimal state for SaveMenu to run
    // ... setup WRAM for save prerequisites ...

    // Measure cycles for the save menu flow
    // SaveMenu is at $1C:$78A9 — need bank $1C active
    let start_cycles = harness.total_cycles();

    // Run SaveMenu
    // ... trigger save menu via memory + PC setup ...

    let end_cycles = harness.total_cycles();
    let save_cycles = end_cycles - start_cycles;

    // The continue game delay was reduced from 30 to 10 frames
    // 10 frames × ~70224 cycles/frame ≈ 702,240 cycles max for the delay portion
    // The prompt was removed entirely, saving additional frames
    println!("Save menu completed in {save_cycles} cycles ({} frames)",
        save_cycles / 70224);

    // Assert it completes within our expected budget
    // (exact threshold determined empirically on first run)
    assert!(save_cycles < 5_000_000, "Save menu took too long: {save_cycles} cycles");
}
```

### Tier 3: Audio & Link Regression

#### 16. Title Screen Audio Smoke Test

**What we're testing**: Audio plays during the title screen (basic sanity check that our changes haven't broken the sound engine).

```rust
#[test]
fn title_screen_has_audio() {
    let mut harness = TestHarness::new();

    // Advance to title screen (~10 seconds of game time)
    harness.run_frames(600);

    // Clear audio buffer, then run a few more frames to collect fresh samples
    harness.gb.apu().clear_audio_buffer();
    harness.run_frames(60); // 1 second of audio

    // Check that audio buffer has samples
    let buffer = harness.gb.audio_buffer_eager(false);
    assert!(!buffer.is_empty(), "Audio buffer should have samples on title screen");

    // Check that at least some samples are non-zero (actual sound, not silence)
    let non_zero = buffer.iter().filter(|&&s| s != 0).count();
    let ratio = non_zero as f64 / buffer.len() as f64;
    assert!(ratio > 0.1, "Title screen should have audible sound (got {:.1}% non-zero samples)", ratio * 100.0);

    // Verify specific channels are active (Pokemon Yellow title uses ch1+ch2 for melody)
    let ch1 = harness.gb.audio_ch1_output();
    let ch2 = harness.gb.audio_ch2_output();
    // At least one of the melody channels should be producing output
    // (checked at a single point, so may be 0 between notes — run multiple checks)
    let mut any_ch1 = false;
    let mut any_ch2 = false;
    for _ in 0..600 {
        harness.gb.clock();
        if harness.gb.audio_ch1_output() > 0 { any_ch1 = true; }
        if harness.gb.audio_ch2_output() > 0 { any_ch2 = true; }
        if any_ch1 && any_ch2 { break; }
    }
    assert!(any_ch1 || any_ch2, "At least one melody channel should produce sound on title screen");
}
```

#### 17. Link Cable Regression — Accuracy Fix Doesn't Break Link Battles

**What we're testing**: Our accuracy fix (the `bit 7, b` optimal rounding) works correctly in link battle mode too. The fix is in `doAccuracyCheck` which runs for both single-player and link battles — we need to verify it doesn't interfere with the link battle RNG path.

```rust
/// Custom SerialDevice that bridges two GameBoy instances
struct LinkBridge {
    /// Data sent by the *other* side, waiting to be received by this side
    incoming: Option<u8>,
    /// Data sent by this side, to be delivered to the other side
    outgoing: Option<u8>,
}

impl SerialDevice for LinkBridge {
    fn send(&mut self) -> u8 {
        // Return whatever the other side sent us (or 0xFF if nothing)
        self.incoming.take().unwrap_or(0xFF)
    }

    fn receive(&mut self, byte: u8) {
        self.outgoing = Some(byte);
    }

    fn allow_slave(&self) -> bool { true }
    fn description(&self) -> String { "LinkBridge".into() }
    fn state(&self) -> String { String::new() }
}

#[test]
fn link_battle_accuracy_check_still_works() {
    // This test verifies the accuracy check in link battle mode
    // (wLinkState == LINK_STATE_BATTLING, using shared RNG list)
    let mut harness = TestHarness::new();

    // Set link battle state
    harness.write_mem(0xD12A, 0x04); // wLinkState = LINK_STATE_BATTLING

    // Pre-fill the RNG list with known values
    let rng_values: [u8; 10] = [0, 50, 100, 127, 128, 200, 242, 254, 255, 0];
    for (i, &val) in rng_values.iter().enumerate() {
        harness.write_mem(0xD147 + i as u16, val); // wLinkBattleRandomNumberList
    }
    harness.write_mem(0xCCDE, 0); // wLinkBattleRandomNumberListIndex = 0

    // Test accuracy check with known RNG values from the list
    // BattleRandom will read from the list instead of hardware RNG
    harness.write_mem(0xCFD5, 200); // wPlayerMoveAccuracy = 200 (N >= 128)
    harness.write_mem(0xFFF3, 0x00); // hWhoseTurn = player
    harness.write_mem(0xD05E, 0x00); // clear wMoveMissed

    // Run doAccuracyCheck — BattleRandom reads list[0] = 0
    // 0 < 200 → should hit
    // NOTE: don't inject register A here — let BattleRandom use the list naturally
    // ... setup PC to doAccuracyCheck and run ...

    // Verify the list index advanced
    let new_index = harness.read_mem(0xCCDE);
    assert_eq!(new_index, 1, "RNG list index should advance after BattleRandom call");
}
```

#### 18. Scripted Wild Encounter — Mew in Cerulean Cave

**What we're testing**: End-to-end verification that Mew can actually be encountered in Cerulean Cave B1F. Uses input scripting to walk through the cave, combined with memory manipulation to speed things up.

```rust
#[test]
fn mew_encounter_cerulean_cave_b1f() {
    let mut harness = TestHarness::new();

    // Fast-forward: set up game state with player inside Cerulean Cave B1F
    // Rather than playing through the entire game, write memory directly
    harness.write_mem(0xD35E, /* CeruleanCaveB1F map ID */);
    // ... set player position, badges, Pokedex flags ...

    // Override RNG to guarantee a specific encounter slot that contains Mew
    // The encounter table has Mew at a specific slot
    // Force the encounter by writing to the wild encounter trigger addresses

    // Walk in grass/cave floor to trigger encounter
    for _ in 0..100 {
        harness.press(PadKey::Up, 16);    // Walk up one tile (16 frames)
        harness.press(PadKey::Down, 16);  // Walk back

        // Check if wild battle started
        let battle_type = harness.read_mem(0xD057); // wIsInBattle
        if battle_type != 0 {
            // Check what Pokemon we encountered
            let species = harness.read_mem(0xCFE5); // wEnemyMonSpecies
            if species == /* DEX_MEW */ 0x15 {
                // Capture screenshot of the Mew battle for golden-image reference
                let screenshot = harness.gb.frame_buffer_eager();
                save_screenshot(&screenshot, "golden/mew_encounter.png");
                return; // Success
            }
        }
    }

    // If we didn't encounter Mew via walking, verify it's in the table at minimum
    // (The ROM byte test already covers this, but the emulator test adds confidence)
    panic!("Failed to encounter Mew in Cerulean Cave B1F within 100 steps");
}
```

---

## Test Infrastructure

### Project Structure

```
tests/
├── EMULATOR_TEST_HARNESS_DESIGN.md   # This document
├── Cargo.toml                         # Rust project for tests
├── src/
│   ├── lib.rs                         # Harness library (re-exports)
│   ├── harness.rs                     # TestHarness: emulator wrapper + high-level API
│   ├── symbols.rs                     # .sym file parser
│   ├── rom.rs                         # ROM byte reader + helpers
│   ├── input.rs                       # Input scripting DSL (Tier 2)
│   ├── golden.rs                      # Golden-image capture + comparison (Tier 2)
│   ├── benchmark.rs                   # Cycle-count benchmarking helpers (Tier 2)
│   └── link.rs                        # Link cable bridge (SerialDevice impl) (Tier 3)
├── golden/                             # Reference screenshots (committed to repo)
│   ├── presents_subtitle.png
│   ├── options_warp.png
│   └── README.md                      # Instructions for regenerating golden images
└── tests/
    ├── accuracy.rs                    # Accuracy check tests (Tier 1, emulator)
    ├── text_speed.rs                  # WARP text speed tests (Tier 1, emulator)
    ├── evolutions.rs                  # Evolution data tests (Tier 1, ROM bytes)
    ├── wild_pokemon.rs                # Wild encounter tests (Tier 1, ROM bytes)
    ├── pikachu_surf.rs                # Pikachu HM Surf test (Tier 1, ROM bytes)
    ├── save.rs                        # Save delay tests (Tier 1, ROM bytes)
    ├── golden_images.rs               # Screenshot comparison tests (Tier 2)
    ├── save_integrity.rs              # SRAM round-trip tests (Tier 2)
    ├── benchmarks.rs                  # Cycle-count benchmarks (Tier 2)
    ├── audio.rs                       # Audio smoke tests (Tier 3)
    └── link_regression.rs             # Link cable regression tests (Tier 3)
```

### Symbol File Parser

The `.sym` file format is simple: `BB:AAAA LabelName`

```rust
use std::collections::HashMap;
use std::fs;

pub struct SymbolTable {
    symbols: HashMap<String, (u8, u16)>, // label → (bank, address)
}

impl SymbolTable {
    pub fn load(path: &str) -> Self {
        let mut symbols = HashMap::new();
        for line in fs::read_to_string(path).unwrap().lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') { continue; }
            // Format: "BB:AAAA LabelName" or "00:AAAA LabelName"
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() != 2 { continue; }
            let addr_parts: Vec<&str> = parts[0].split(':').collect();
            if addr_parts.len() != 2 { continue; }
            let bank = u8::from_str_radix(addr_parts[0], 16).unwrap_or(0);
            let addr = u16::from_str_radix(addr_parts[1], 16).unwrap_or(0);
            symbols.insert(parts[1].to_string(), (bank, addr));
        }
        SymbolTable { symbols }
    }

    pub fn resolve(&self, label: &str) -> Option<(u8, u16)> {
        self.symbols.get(label).copied()
    }

    /// Convert bank:addr to flat ROM offset
    pub fn rom_offset(&self, label: &str) -> Option<usize> {
        let (bank, addr) = self.resolve(label)?;
        if bank == 0 {
            Some(addr as usize)
        } else {
            // Bank N maps to ROM offset: bank * 0x4000 + (addr - 0x4000)
            Some(bank as usize * 0x4000 + (addr as usize - 0x4000))
        }
    }
}
```

### Harness API (boytacean-based)

```rust
use boytacean::gb::{GameBoy, GameBoyMode};
use boytacean::pad::PadKey;
use boytacean::state::{StateManager, SaveStateFormat};
use std::path::Path;

pub struct TestHarness {
    pub gb: GameBoy,
    pub sym: SymbolTable,
    total_cycles: u64,
}

impl TestHarness {
    pub fn new() -> Self {
        let mut gb = GameBoy::new(Some(GameBoyMode::Dmg));
        gb.load(true).unwrap();

        let rom_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../pokeyellow.gbc");
        let rom_bytes = std::fs::read(&rom_path).expect("Failed to read pokeyellow.gbc");
        gb.load_rom(&rom_bytes, None).unwrap();

        let sym_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../pokeyellow.sym");
        let sym = SymbolTable::load(sym_path.to_str().unwrap());

        TestHarness { gb, sym, total_cycles: 0 }
    }

    // ── Memory Access ──────────────────────────────────────────────

    pub fn read_mem(&mut self, addr: u16) -> u8 {
        self.gb.read_memory(addr)
    }

    pub fn write_mem(&mut self, addr: u16, val: u8) {
        self.gb.write_memory(addr, val);
    }

    // ── Execution Control ──────────────────────────────────────────

    /// Run until PC reaches target address (with timeout)
    pub fn run_until_pc(&mut self, addr: u16, max_instructions: u64) -> bool {
        for _ in 0..max_instructions {
            let cycles = self.gb.clock();
            self.total_cycles += cycles as u64;
            if self.gb.registers().pc == addr {
                return true;
            }
        }
        false
    }

    /// Advance N frames
    pub fn run_frames(&mut self, n: u32) {
        for _ in 0..n {
            let cycles = self.gb.next_frame();
            self.total_cycles += cycles as u64;
        }
    }

    /// Total CPU cycles consumed so far
    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    // ── Input Scripting (Tier 2) ───────────────────────────────────

    /// Press a button for N frames, then release
    pub fn press(&mut self, key: PadKey, frames: u32) {
        self.gb.key_press(key);
        self.run_frames(frames);
        self.gb.key_lift(key);
    }

    /// Hold a button for N frames (does not release)
    pub fn hold(&mut self, key: PadKey, frames: u32) {
        self.gb.key_press(key);
        self.run_frames(frames);
    }

    /// Wait until a memory address satisfies a predicate (with timeout)
    pub fn wait_for_memory<F>(&mut self, addr: u16, pred: F, max_frames: u32) -> bool
    where F: Fn(u8) -> bool {
        for _ in 0..max_frames {
            if pred(self.read_mem(addr)) { return true; }
            self.run_frames(1);
        }
        false
    }

    // ── Screenshots (Tier 2) ───────────────────────────────────────

    /// Capture current framebuffer as RGB888 Vec
    pub fn capture_screenshot(&mut self) -> Vec<u8> {
        self.gb.frame_buffer_eager()
    }

    // ── Save States (Tier 2) ───────────────────────────────────────

    /// Snapshot full emulator state (compressed)
    pub fn save_state(&mut self) -> Vec<u8> {
        StateManager::save(&mut self.gb, Some(SaveStateFormat::Bosc), None)
            .expect("Failed to save state")
    }

    /// Restore from a previous snapshot
    pub fn load_state(&mut self, data: &[u8]) {
        StateManager::load(data, &mut self.gb, None, None)
            .expect("Failed to load state");
    }

    // ── Accuracy Testing (Strategy C) ──────────────────────────────

    /// Run accuracy check with injected random value
    /// Returns true if hit, false if miss
    pub fn test_accuracy(&mut self, accuracy: u8, random_value: u8) -> bool {
        self.write_mem(0xCFD5, accuracy);  // wPlayerMoveAccuracy
        self.write_mem(0xFFF3, 0x00);      // hWhoseTurn = player
        self.write_mem(0xD05E, 0x00);      // clear wMoveMissed

        // Step until cp b (0x679E) — right after BattleRandom returns
        let reached = self.run_until_pc(0x679E, 100_000);
        assert!(reached, "Failed to reach cp b instruction");

        // Inject random value into register A
        self.gb.cpu().set_a(random_value);

        // Continue until hit (0x67A7) or miss (0x67A8)
        loop {
            self.gb.clock();
            let pc = self.gb.registers().pc;
            if pc == 0x67A7 { return true; }   // .accuracyHit
            if pc == 0x67A8 { return false; }   // .moveMissed
        }
    }
}
```

### Input Scripting Module (`src/input.rs`) — Tier 2

A builder DSL for expressing button press sequences declaratively:

```rust
use boytacean::pad::PadKey;

pub struct InputScript {
    actions: Vec<InputAction>,
}

enum InputAction {
    Press(PadKey, u32),                     // Press for N frames, then release
    Hold(PadKey, u32),                      // Hold for N frames (don't release)
    Release(PadKey),                         // Release a held button
    Wait(u32),                               // Wait N frames with no input
    WaitForMemory(u16, u8, u32),            // Wait until addr==value (max frames)
}

impl InputScript {
    pub fn new() -> Self { Self { actions: vec![] } }

    pub fn press(mut self, key: PadKey, frames: u32) -> Self {
        self.actions.push(InputAction::Press(key, frames));
        self
    }

    pub fn wait(mut self, frames: u32) -> Self {
        self.actions.push(InputAction::Wait(frames));
        self
    }

    pub fn wait_for(mut self, addr: u16, value: u8, max_frames: u32) -> Self {
        self.actions.push(InputAction::WaitForMemory(addr, value, max_frames));
        self
    }

    /// Execute the script on a TestHarness
    pub fn run(self, harness: &mut TestHarness) { /* ... */ }
}

// Usage:
// InputScript::new()
//     .wait(600)                              // Wait for title screen
//     .press(PadKey::Start, 30)               // Press Start
//     .wait(60)                               // Wait for menu
//     .press(PadKey::Down, 10)                // Navigate to Options
//     .press(PadKey::A, 10)                   // Select Options
//     .wait_for(0xD354, 0x00, 300)            // Wait for WARP to be set
//     .run(&mut harness);
```

### Golden-Image Module (`src/golden.rs`) — Tier 2

Leverages boytacean's framebuffer output and follows their existing `compare_images` / `save_image` pattern:

```rust
use std::path::Path;

/// Save a framebuffer as a PNG file (for generating reference images)
pub fn save_screenshot(pixels: &[u8], path: &str) {
    // pixels is RGB888, 160×144
    // Use the `image` crate to encode as PNG
    use image::{ImageBuffer, Rgb};
    let img = ImageBuffer::<Rgb<u8>, _>::from_raw(160, 144, pixels.to_vec())
        .expect("Failed to create image buffer");
    img.save(path).expect("Failed to save screenshot");
}

/// Compare a framebuffer against a reference PNG
/// Returns true if similarity >= threshold (0.0 to 1.0)
pub fn compare_screenshot(pixels: &[u8], reference_path: &str, threshold: f64) -> bool {
    let reference = image::open(reference_path)
        .expect("Failed to open reference image")
        .to_rgb8();

    assert_eq!(reference.width(), 160);
    assert_eq!(reference.height(), 144);

    let ref_bytes = reference.as_raw();
    let total_pixels = 160 * 144;
    let mut matching = 0u32;

    for i in 0..total_pixels {
        let offset = i * 3;
        if pixels[offset] == ref_bytes[offset]
            && pixels[offset + 1] == ref_bytes[offset + 1]
            && pixels[offset + 2] == ref_bytes[offset + 2]
        {
            matching += 1;
        }
    }

    let similarity = matching as f64 / total_pixels as f64;
    similarity >= threshold
}
```

**Workflow for golden images**:
1. First run: `GENERATE_GOLDEN=1 cargo test` generates reference PNGs
2. Manually verify the PNGs are correct
3. Commit `tests/golden/*.png` to the repo
4. Subsequent runs compare against committed references

### Link Bridge Module (`src/link.rs`) — Tier 3

Custom `SerialDevice` implementation for bridging two `GameBoy` instances:

```rust
use boytacean::serial::SerialDevice;
use std::sync::{Arc, Mutex};

/// Shared state between two sides of a link cable bridge
struct LinkShared {
    a_to_b: Option<u8>,  // Data from side A waiting for side B
    b_to_a: Option<u8>,  // Data from side B waiting for side A
}

pub struct LinkBridge {
    shared: Arc<Mutex<LinkShared>>,
    is_side_a: bool,
}

impl LinkBridge {
    /// Create a pair of linked devices
    pub fn new_pair() -> (Box<LinkBridge>, Box<LinkBridge>) {
        let shared = Arc::new(Mutex::new(LinkShared {
            a_to_b: None,
            b_to_a: None,
        }));
        let side_a = Box::new(LinkBridge { shared: shared.clone(), is_side_a: true });
        let side_b = Box::new(LinkBridge { shared, is_side_a: false });
        (side_a, side_b)
    }
}

impl SerialDevice for LinkBridge {
    fn send(&mut self) -> u8 {
        let mut shared = self.shared.lock().unwrap();
        if self.is_side_a {
            shared.b_to_a.take().unwrap_or(0xFF)
        } else {
            shared.a_to_b.take().unwrap_or(0xFF)
        }
    }

    fn receive(&mut self, byte: u8) {
        let mut shared = self.shared.lock().unwrap();
        if self.is_side_a {
            shared.a_to_b = Some(byte);
        } else {
            shared.b_to_a = Some(byte);
        }
    }

    fn allow_slave(&self) -> bool { !self.is_side_a } // Side B is slave
    fn description(&self) -> String {
        format!("LinkBridge({})", if self.is_side_a { "A" } else { "B" })
    }
    fn state(&self) -> String { String::new() }
}

// Usage:
// let (bridge_a, bridge_b) = LinkBridge::new_pair();
// gb_a.attach_serial(bridge_a);
// gb_b.attach_serial(bridge_b);
// // Interleave execution:
// for _ in 0..1000 {
//     gb_a.clock();
//     gb_b.clock();
// }
```

### ROM Byte Tests vs Emulator Tests

Two categories of tests require different approaches:

1. **ROM byte tests** (static analysis): Read ROM bytes directly, no emulator needed
   - Evolution data, TM/HM learnsets, wild encounter tables, move data
   - Very fast, no emulator startup cost
   - Parse ROM bytes using sym file offsets

2. **Emulator tests** (dynamic analysis): Run code and observe behavior
   - Accuracy check (requires RNG control + code execution)
   - Text speed (requires timer/frame counting)
   - Save delay (requires timing measurement)
   - More complex, requires emulator initialization

### Two-Phase Test Approach

```rust
// Phase 1: ROM byte tests (no emulator needed)
#[cfg(test)]
mod rom_tests {
    #[test]
    fn kadabra_evolves_at_36() { /* read ROM bytes */ }
    #[test]
    fn pikachu_learns_surf() { /* read ROM bytes */ }
    #[test]
    fn mew_in_cerulean_cave() { /* read ROM bytes */ }
}

// Phase 2: Emulator tests (require GB emulator)
#[cfg(test)]
mod emu_tests {
    #[test]
    fn accuracy_255_always_hits() { /* run emulator */ }
    #[test]
    fn warp_text_no_delay() { /* run emulator */ }
}
```

---

## CI Integration

### GitHub Actions Workflow Addition

```yaml
# .github/workflows/test.yml
name: Tests

on:
  push:
    branches: [ dashed-patch, 'dashed/**' ]
  pull_request:

env:
  rgbds_version: v1.0.1

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Install RGBDS
        run: |
          git clone --depth 1 --branch ${{ env.rgbds_version }} https://github.com/gbdev/rgbds.git
          cd rgbds
          sudo apt-get update && sudo apt-get install -yq bison libpng-dev pkg-config
          sudo make -j$(nproc) install
          cd .. && rm -rf rgbds

      - name: Build ROM
        run: make -j$(nproc)

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache Cargo
        uses: actions/cache@v4
        with:
          path: |
            tests/target
            ~/.cargo/registry
          key: ${{ runner.os }}-cargo-${{ hashFiles('tests/Cargo.lock') }}

      - name: "Tier 1: ROM byte tests"
        working-directory: tests
        run: cargo test rom_tests -- --test-threads=1

      - name: "Tier 1: Emulator tests"
        working-directory: tests
        run: cargo test emu_tests -- --test-threads=1

      - name: "Tier 2: Golden image + state + benchmarks"
        working-directory: tests
        run: cargo test tier2 -- --test-threads=1

      - name: "Tier 3: Audio + link regression"
        working-directory: tests
        run: cargo test tier3 -- --test-threads=1
```

### Cargo.toml

```toml
[package]
name = "pokeyellow-tests"
version = "0.1.0"
edition = "2021"

[dependencies]
boytacean = "0.11"
image = { version = "0.25", default-features = false, features = ["png"] }  # Tier 2: golden images
```

No C/C++ build dependencies, no vendor submodules. Just `cargo build`.

---

## Implementation Plan

### Phase 1: Foundation + Tier 1 ROM Byte Tests

**Effort**: Small

1. Create `tests/Cargo.toml` with boytacean + image dependencies
2. Implement `symbols.rs` — sym file parser
3. Implement `rom.rs` — ROM byte reader with bank-aware offset resolution
4. Write Tier 1 static tests (no emulator needed):
   - Evolution data verification (4 Pokemon, level 36) — Scenario 7
   - Pikachu Surf learnset check — Scenario 8
   - Wild Pokemon encounter table verification (all 16 additions) — Scenario 9
   - Save delay code verification — Scenario 10
5. Add CI workflow for ROM byte tests

### Phase 2: Tier 1 Emulator Tests

**Effort**: Medium

1. Implement `harness.rs` — TestHarness with boytacean GameBoy wrapper
2. Validate emulator can boot the ROM and reach known PC addresses
3. Write Tier 1 emulator tests:
   - Accuracy N=255 always hits — Scenario 1
   - Optimal rounding N≥128 (hit) and N<128 (miss) — Scenarios 2-3
   - Basic hit/miss (random < accuracy, random > accuracy) — Scenario 4
   - Exhaustive accuracy (255×256 = 65,280 iterations) — Scenario 5
   - WARP text speed zero-frame delay — Scenario 6
4. Performance: benchmark exhaustive test, consider snapshot/restore for reuse

### Phase 3: Tier 2 — Visual, State & Performance

**Effort**: Medium

1. Implement `input.rs` — InputScript DSL
2. Implement `golden.rs` — screenshot capture + PNG comparison (using `image` crate)
3. Generate golden reference images for PRESENTS subtitle and options menu
4. Write Tier 2 tests:
   - PRESENTS subtitle golden image — Scenario 11
   - Options menu WARP golden image — Scenario 12
   - Save file SRAM round-trip — Scenario 13
   - Text speed cycle ordering benchmark — Scenario 14
   - Save routine timing — Scenario 15
5. Implement `benchmark.rs` — cycle-count measurement helpers
6. Add `tests/golden/` directory with reference PNGs to repo

### Phase 4: Tier 3 — Audio & Link Regression

**Effort**: Medium-Large

1. Implement `link.rs` — LinkBridge SerialDevice for dual-instance testing
2. Write Tier 3 tests:
   - Title screen audio smoke test — Scenario 16
   - Link cable accuracy regression — Scenario 17
   - Scripted Mew encounter — Scenario 18
3. Validate two-instance serial bridging works with boytacean

### Phase 5: CI & Polish

**Effort**: Small

1. Full CI workflow: build ROM → Tier 1 ROM byte tests → Tier 1 emulator tests → Tier 2 → Tier 3
2. Cargo caching for fast CI runs
3. Golden image regeneration workflow (`GENERATE_GOLDEN=1`)
4. Benchmark result tracking (optional: store in CI artifacts)
5. Documentation and test comments

### Dependency Graph

```
Phase 1 (ROM tests)  ──┐
                        ├──→  Phase 5 (CI)
Phase 2 (Emulator)   ──┤
                        │
Phase 3 (Visual/Perf) ─┤
                        │
Phase 4 (Audio/Link) ──┘

Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5
                ↘                   ↗
                  → Phase 3 (parallel)
```

Phases 3 and 4 can proceed in parallel once Phase 2 is complete. Phase 1 is independent and can be done first as a quick win.

---

## Open Questions and Risks

### Resolved Questions

1. ~~**Emulator library choice**~~: **Resolved** → boytacean v0.11.5. Only Rust crate with GBC, MBC5, memory R/W, `step_to`, Apache-2.0. Source verified locally at `/Users/me/aaa/github/boytacean`.

2. ~~**Side effects of LINK_STATE_BATTLING**~~: **Resolved** → Don't use it (Strategy A rejected). `wLinkState` is checked 22+ times in `core.asm` plus 15+ other files. Use Strategy C (breakpoint + register injection at `$0F:$679E`) instead — zero side effects.

3. ~~**Build time**~~: **Resolved** → boytacean is a pure Rust crate. No C/C++ FFI, no vendor submodules. `cargo build` only.

### Open Questions

1. **Battle state initialization**: Setting up a valid battle state in memory (two Pokemon, move data, status flags) requires careful initialization. Need to determine the minimum set of WRAM addresses for MoveHitTest to run without crashing. Mitigation: use save states — play to a battle, snapshot, use as test fixture.

2. **ROM bank switching**: The accuracy check code is in bank `$0F`. Need to verify boytacean handles MBC5 bank switching correctly when we `set_pc()` to a banked address. May need to write the bank number to the MBC5 bank register (`$2000-$2FFF`) before jumping.

3. **CalcHitChance bypass**: For accuracy tests, we set `wPlayerMoveAccuracy` directly and jump to `doAccuracyCheck` (`$0F:$6797`). Need to verify B register is loaded correctly from `wPlayerMoveAccuracy` at that entry point (first instruction is `ld a, b` — so B must already contain the accuracy value).

4. **Stack setup**: Jumping directly into ROM code requires a valid stack pointer. Need to ensure SP is set to a safe WRAM area and the stack has a valid return address.

5. **Golden image stability**: Framebuffer output may vary between boytacean versions (palette colors, rendering timing). Need to either pin the boytacean version or use fuzzy comparison thresholds.

6. **Audio determinism**: APU output depends on exact cycle timing. The `audio_ch*_output()` methods return instantaneous values that may vary between runs depending on when we sample. Need to accumulate over multiple frames for reliable assertions.

7. **Link bridge synchronization**: The `LinkBridge` implementation uses `Arc<Mutex<>>` for shared state, but serial transfers in the Game Boy are cycle-timed. Interleaving two `GameBoy` instances on a single thread may not produce byte-accurate timing. May need frame-level synchronization instead of instruction-level.

8. **Boytacean accuracy for our use case**: While boytacean passes many Blargg tests, we specifically need correct `cp`, `jr`, `bit`, and flag behavior for our accuracy code. Should write a small validation test that exercises these exact instructions before trusting the full test suite.

### Risks

1. **Emulator accuracy for flag behavior**: If boytacean doesn't correctly implement `cp` flag behavior or `bit 7, b` zero flag, our accuracy tests would be unreliable. Mitigation: write a dedicated flag validation test; fall back to safeboy (SameBoy bindings) if needed.

2. **State initialization complexity**: Battle state in Gen 1 has many interdependent variables. Mitigation: use save states as test fixtures (play to battle, `StateManager::save()`, commit state file).

3. **ROM byte tests may break on upstream rebase**: If upstream changes the ROM layout, symbol addresses shift. Mitigation: always use `.sym` file for addresses, never hardcode offsets.

4. **Emulator startup time for exhaustive tests**: 65,280 iterations (255×256) may be slow if each requires full emulator init. Mitigation: use `StateManager::save()/load()` to snapshot a ready-to-test battle state, restore for each iteration.

5. **Golden image fragility**: Reference screenshots may break if boytacean changes its default palette or rendering behavior. Mitigation: pin boytacean version in `Cargo.toml`, use `>=98%` similarity threshold rather than exact match.

6. **Two-instance link testing complexity**: Correctly synchronizing two GameBoy instances for serial communication is non-trivial. Mitigation: start with unit-level link tests (verify RNG list behavior) before attempting full link battle integration.

---

## Appendix: BattleRandom Full Analysis

### Code Flow

```
MoveHitTest ($0F:$6700)
  ├── Dream Eater check (target must be sleeping)
  ├── Swift check (always hits, return immediately)
  ├── Substitute check
  ├── Dig/Fly invulnerability check
  ├── Mist protection check
  ├── X Accuracy check (always hits, return immediately)
  ├── CalcHitChance ($0F:$6787)
  │   └── Scales accuracy by attacker accuracy stage / target evasion stage
  │       Result stored in wPlayerMoveAccuracy ($CFD5) or wEnemyMoveAccuracy ($CFCF)
  │       Clamped to [1, 255]
  └── .doAccuracyCheck ($0F:$6797)  ← OUR MODIFIED CODE
      ├── cp $FF → ret z (N=255 always hits)
      ├── call BattleRandom ($0F:$7038)
      │   ├── wLinkState == $04? → use wLinkBattleRandomNumberList
      │   └── else → call Random (hardware RNG)
      ├── cp b (compare random with accuracy)
      ├── jr c, .accuracyHit (random < accuracy → hit)
      ├── jr nz, .moveMissed (random > accuracy → miss)
      ├── bit 7, b (random == accuracy: check if N >= 128)
      ├── jr z, .moveMissed (N < 128 → miss)
      └── .accuracyHit (N >= 128 → hit)
```

### BattleRandom Internal Logic

```asm
BattleRandom:                          ; $0F:$7038
    ld a, [wLinkState]                 ; Check if in link battle
    cp LINK_STATE_BATTLING             ; $04
    jp nz, Random                      ; No → use hardware RNG

    ; Link battle mode: read from pre-shared list
    push hl
    push bc
    ld a, [wLinkBattleRandomNumberListIndex]
    ld c, a
    ld b, 0
    ld hl, wLinkBattleRandomNumberList
    add hl, bc                         ; hl = &list[index]
    inc a
    ld [wLinkBattleRandomNumberListIndex], a  ; index++
    cp SERIAL_RNS_LENGTH - 1           ; index == 9?
    ld a, [hl]                         ; a = list[index] (return value)
    pop bc
    pop hl
    ret c                              ; if index < 9, return

    ; index == 9: regenerate the list
    ; PRNG: next = prev * 5 + 1
    push hl, bc, af
    xor a
    ld [wLinkBattleRandomNumberListIndex], a  ; reset index to 0
    ld hl, wLinkBattleRandomNumberList
    ld b, SERIAL_RNS_LENGTH - 1        ; regenerate 9 entries
.loop
    ld a, [hl]
    ld c, a
    add a        ; a *= 2
    add a        ; a *= 4
    add c        ; a *= 5
    inc a        ; a = a * 5 + 1
    ld [hli], a
    dec b
    jr nz, .loop
    pop af, bc, hl                     ; restore original return value
    ret
```

### Flag Behavior in doAccuracyCheck

| Instruction | Z flag | C flag | Meaning |
|---|---|---|---|
| `cp $FF` (a=255) | Z=1 | C=0 | a == 255 |
| `cp $FF` (a<255) | Z=0 | C=1 or 0 | a != 255 |
| `cp b` (a < b) | Z=0 | C=1 | random < accuracy → HIT |
| `cp b` (a == b) | Z=1 | C=0 | random == accuracy → threshold check |
| `cp b` (a > b) | Z=0 | C=0 | random > accuracy → MISS |
| `bit 7, b` (b>=128) | Z=0 | - | accuracy >= 128 → HIT |
| `bit 7, b` (b<128) | Z=1 | - | accuracy < 128 → MISS |

## Appendix: Testing Approaches Survey

### Current State of GB ROM Testing

Automated runtime testing is essentially **nonexistent** in the GB/GBC ROM hacking community. The pret projects (pokered, pokeyellow, pokecrystal) only verify builds compile and match expected SHA1 hashes. The only notable automated test framework in Pokemon ROM hacking is pokeemerald-expansion's battle test system — but that's for GBA, not GB/GBC.

This codebase already uses:
- **Build verification**: CI compiles with RGBDS v1.0.1 and runs `checkdiff.sh`
- **RGBDS compile-time assertions**: `ASSERT`, `STATIC_ASSERT`, `table_width`/`assert_table_length`, `def_grass_wildmons`/`end_grass_wildmons` macros
- **No runtime testing** of any kind

### Available Testing Frameworks

| Approach | Effort | Runtime Testing | CI Friendly | Best For |
|----------|--------|----------------|-------------|----------|
| RGBDS assertions (extend existing) | Low | No | Yes | Data integrity |
| mGBA + Lua scripting | Medium | Yes | Needs xvfb | Gameplay testing |
| SameBoy headless tester | Medium | Screenshot only | Yes | Visual regression |
| Gambatte C++ test harness | High | Yes | Yes | Cycle-accurate tests |
| Rust emulator test harness | High | Yes | Yes (`cargo test`) | Deep integration |
| Mooneye-style test ROMs | Medium | Yes | With emulator | Hardware behavior |

### Why Rust Emulator Harness

The Rust approach is recommended because:
1. **`cargo test` integration** — standard Rust testing workflow, familiar CI setup
2. **No external processes** — emulator runs in-process, no display/xvfb needed
3. **Programmatic memory access** — can read/write any WRAM address without scripting
4. **Deterministic RNG** — can control BattleRandom via breakpoint + register injection (Strategy C)
5. **Two-phase testing** — ROM byte tests (fast, no emulator) + emulator tests (thorough)
6. **No community precedent** — we'd be pioneering this approach for GB ROM hacks

### Sources

- [mGBA Scripting API](https://mgba.io/docs/scripting.html)
- [SameBoy Tester](https://github.com/LIJI32/SameBoy/blob/master/Tester/main.c)
- [Mooneye Test Suite](https://github.com/Gekkio/mooneye-test-suite)
- [c-sp/game-boy-test-roms](https://github.com/c-sp/game-boy-test-roms)
- [Gekkio: GB Test ROM Do's and Don'ts](https://gekkio.fi/blog/2016/game-boy-test-rom-dos-and-donts/)
- [Gambatte testrunner](https://github.com/c-sp/game-boy-test-roms/blob/master/src/howto/gambatte.md)
- [mGBA-http](https://github.com/nikouu/mGBA-http)
- [boytacean](https://github.com/joamag/boytacean)
- [safeboy (SameBoy Rust bindings)](https://docs.rs/safeboy/latest/safeboy/)
- [rgy (Rust GB emulator with debugger)](https://docs.rs/rgy/latest/rgy/)

---

*Last updated: 2026-03-08*
