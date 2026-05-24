//! Debugging tools for Game Boy emulator tests.
//!
//! Provides structured snapshots of joypad state, CPU registers, and
//! memory-mapped game variables to diagnose test failures — especially
//! input-related issues where button presses are silently dropped.

use crate::TestHarness;
use boytacean::pad::PadKey;
use std::fmt;

// ── Pokemon Yellow joypad addresses ──────────────────────────────────

/// hJoyInput ($FFF5) — raw hardware joypad reading from ReadJoypad_ (VBlank).
const H_JOY_INPUT: u16 = 0xFFF5;
/// hJoyLast ($FFB1) — previous frame's hJoyInput.
const H_JOY_LAST: u16 = 0xFFB1;
/// hJoyReleased ($FFB2) — buttons released this frame.
const H_JOY_RELEASED: u16 = 0xFFB2;
/// hJoyPressed ($FFB3) — newly pressed buttons this frame.
const H_JOY_PRESSED: u16 = 0xFFB3;
/// hJoyHeld ($FFB4) — all currently held buttons.
const H_JOY_HELD: u16 = 0xFFB4;
/// hJoy5 ($FFB5) — game-visible joypad state (set by JoypadLowSensitivity).
const H_JOY5: u16 = 0xFFB5;
/// hJoy6 ($FFB6) — JoypadLowSensitivity mode flag.
const H_JOY6: u16 = 0xFFB6;
/// hJoy7 ($FFB7) — JoypadLowSensitivity mode flag.
const H_JOY7: u16 = 0xFFB7;
/// hDisableJoypadPolling ($FFF8) — blocks ReadJoypad_ from reading hardware.
const H_DISABLE_JOYPAD_POLLING: u16 = 0xFFF8;
/// hFrameCounter ($FFD5) — delay timer used by JoypadLowSensitivity.
const H_FRAME_COUNTER: u16 = 0xFFD5;

/// wJoyIgnore ($CD6B) — bitmask of buttons to filter from hJoyHeld/hJoyPressed.
const W_JOY_IGNORE: u16 = 0xCD6B;
/// wStatusFlags5 ($D72F) — bit 6 = BIT_DISABLE_JOYPAD.
const W_STATUS_FLAGS5: u16 = 0xD72F;
/// wMenuJoypadPollCount ($CC34) — HandleMenuInput poll timeout.
const W_MENU_JOYPAD_POLL_COUNT: u16 = 0xCC34;
/// wCurrentMenuItem ($CC26).
const W_CURRENT_MENU_ITEM: u16 = 0xCC26;
/// wMaxMenuItem ($CC28).
const W_MAX_MENU_ITEM: u16 = 0xCC28;
/// wMenuWatchedKeys ($CC29).
const W_MENU_WATCHED_KEYS: u16 = 0xCC29;

/// IE register ($FFFF) — interrupt enable.
const IE_REG: u16 = 0xFFFF;
/// IF register ($FF0F) — interrupt flags.
const IF_REG: u16 = 0xFF0F;

// ── Button constants (Pokemon Yellow format) ─────────────────────────

const PAD_A: u8 = 0x01;
const PAD_B: u8 = 0x02;
const PAD_SELECT: u8 = 0x04;
const PAD_START: u8 = 0x08;
const PAD_RIGHT: u8 = 0x10;
const PAD_LEFT: u8 = 0x20;
const PAD_UP: u8 = 0x40;
const PAD_DOWN: u8 = 0x80;

/// Format a joypad byte as human-readable button names.
fn format_buttons(val: u8) -> String {
    if val == 0 {
        return "none".to_string();
    }
    let mut parts = Vec::new();
    if val & PAD_A != 0 {
        parts.push("A");
    }
    if val & PAD_B != 0 {
        parts.push("B");
    }
    if val & PAD_SELECT != 0 {
        parts.push("Sel");
    }
    if val & PAD_START != 0 {
        parts.push("Start");
    }
    if val & PAD_RIGHT != 0 {
        parts.push("Right");
    }
    if val & PAD_LEFT != 0 {
        parts.push("Left");
    }
    if val & PAD_UP != 0 {
        parts.push("Up");
    }
    if val & PAD_DOWN != 0 {
        parts.push("Down");
    }
    parts.join("+")
}

/// Snapshot of the full joypad pipeline state.
#[derive(Clone)]
pub struct JoypadSnapshot {
    pub h_joy_input: u8,
    pub h_joy_last: u8,
    pub h_joy_released: u8,
    pub h_joy_pressed: u8,
    pub h_joy_held: u8,
    pub h_joy5: u8,
    pub h_joy6: u8,
    pub h_joy7: u8,
    pub h_disable_joypad_polling: u8,
    pub h_frame_counter: u8,
    pub w_joy_ignore: u8,
    pub w_status_flags5: u8,
    pub w_menu_joypad_poll_count: u8,
    pub w_current_menu_item: u8,
    pub w_max_menu_item: u8,
    pub w_menu_watched_keys: u8,
}

impl fmt::Display for JoypadSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Joypad Pipeline:")?;
        writeln!(
            f,
            "    hJoyInput($FFF5)={:02X} [{}]",
            self.h_joy_input,
            format_buttons(self.h_joy_input)
        )?;
        writeln!(
            f,
            "    hJoyLast($FFB1)={:02X}  hJoyReleased($FFB2)={:02X}",
            self.h_joy_last, self.h_joy_released
        )?;
        writeln!(
            f,
            "    hJoyPressed($FFB3)={:02X} [{}]",
            self.h_joy_pressed,
            format_buttons(self.h_joy_pressed)
        )?;
        writeln!(
            f,
            "    hJoyHeld($FFB4)={:02X} [{}]",
            self.h_joy_held,
            format_buttons(self.h_joy_held)
        )?;
        writeln!(
            f,
            "    hJoy5($FFB5)={:02X} [{}]  ← game reads this",
            self.h_joy5,
            format_buttons(self.h_joy5)
        )?;
        writeln!(
            f,
            "    hJoy6={:02X}  hJoy7={:02X}  hFrameCounter={:02X}",
            self.h_joy6, self.h_joy7, self.h_frame_counter
        )?;
        writeln!(f, "  Blocking flags:")?;
        writeln!(
            f,
            "    hDisableJoypadPolling($FFF8)={:02X}{}",
            self.h_disable_joypad_polling,
            if self.h_disable_joypad_polling != 0 {
                " *** BLOCKING ReadJoypad_ ***"
            } else {
                ""
            }
        )?;
        writeln!(
            f,
            "    wStatusFlags5($D72F)={:02X} BIT_DISABLE_JOYPAD={}{}",
            self.w_status_flags5,
            if self.w_status_flags5 & (1 << 6) != 0 {
                "SET"
            } else {
                "clear"
            },
            if self.w_status_flags5 & (1 << 6) != 0 {
                " *** DISCARDING ALL PRESSES ***"
            } else {
                ""
            }
        )?;
        writeln!(
            f,
            "    wJoyIgnore($CD6B)={:02X} [{}]{}",
            self.w_joy_ignore,
            format_buttons(self.w_joy_ignore),
            if self.w_joy_ignore != 0 {
                " *** MASKING BUTTONS ***"
            } else {
                ""
            }
        )?;
        writeln!(f, "  Menu state:")?;
        writeln!(
            f,
            "    wCurrentMenuItem={} wMaxMenuItem={} wMenuJoypadPollCount={:02X}",
            self.w_current_menu_item, self.w_max_menu_item, self.w_menu_joypad_poll_count
        )?;
        write!(
            f,
            "    wMenuWatchedKeys={:02X} [{}]",
            self.w_menu_watched_keys,
            format_buttons(self.w_menu_watched_keys)
        )
    }
}

impl JoypadSnapshot {
    /// True if any blocking flag prevents joypad input from reaching the game.
    pub fn is_blocked(&self) -> bool {
        self.h_disable_joypad_polling != 0
            || self.w_status_flags5 & (1 << 6) != 0
            || self.w_joy_ignore != 0
    }
}

/// CPU and interrupt state snapshot.
pub struct CpuSnapshot {
    pub pc: u16,
    pub sp: u16,
    pub a: u8,
    pub ie: u8,
    pub if_: u8,
    pub ime: bool,
}

impl fmt::Display for CpuSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  CPU: PC=${:04X} SP=${:04X} A=${:02X} IE=${:02X} IF=${:02X} IME={}",
            self.pc,
            self.sp,
            self.a,
            self.ie,
            self.if_,
            if self.ime { "on" } else { "off" }
        )
    }
}

// ── TestHarness debug extensions ─────────────────────────────────────

impl TestHarness {
    /// Capture a snapshot of all joypad-related memory addresses.
    pub fn joypad_snapshot(&mut self) -> JoypadSnapshot {
        JoypadSnapshot {
            h_joy_input: self.read_mem(H_JOY_INPUT),
            h_joy_last: self.read_mem(H_JOY_LAST),
            h_joy_released: self.read_mem(H_JOY_RELEASED),
            h_joy_pressed: self.read_mem(H_JOY_PRESSED),
            h_joy_held: self.read_mem(H_JOY_HELD),
            h_joy5: self.read_mem(H_JOY5),
            h_joy6: self.read_mem(H_JOY6),
            h_joy7: self.read_mem(H_JOY7),
            h_disable_joypad_polling: self.read_mem(H_DISABLE_JOYPAD_POLLING),
            h_frame_counter: self.read_mem(H_FRAME_COUNTER),
            w_joy_ignore: self.read_mem(W_JOY_IGNORE),
            w_status_flags5: self.read_mem(W_STATUS_FLAGS5),
            w_menu_joypad_poll_count: self.read_mem(W_MENU_JOYPAD_POLL_COUNT),
            w_current_menu_item: self.read_mem(W_CURRENT_MENU_ITEM),
            w_max_menu_item: self.read_mem(W_MAX_MENU_ITEM),
            w_menu_watched_keys: self.read_mem(W_MENU_WATCHED_KEYS),
        }
    }

    /// Capture CPU and interrupt state.
    pub fn cpu_snapshot(&mut self) -> CpuSnapshot {
        CpuSnapshot {
            pc: self.pc(),
            sp: self.gb.cpu_i().sp(),
            a: self.a(),
            ie: self.read_mem(IE_REG),
            if_: self.read_mem(IF_REG),
            ime: self.gb.cpu_i().ime(),
        }
    }

    /// Print a full diagnostic dump to stderr.
    pub fn dump_state(&mut self, label: &str) {
        let joy = self.joypad_snapshot();
        let cpu = self.cpu_snapshot();
        eprintln!("=== {} ===", label);
        eprintln!("{}", cpu);
        eprintln!("{}", joy);
    }

    /// Press a key and trace joypad state each frame. Returns the frame
    /// number (0-based) where hJoy5 first showed the expected button, or
    /// None if it never appeared within `max_frames`.
    pub fn press_traced(&mut self, key: PadKey, hold_frames: u32, max_frames: u32) -> Option<u32> {
        let expected_bit = pad_key_to_game_bit(&key);
        let key_u8 = crate::harness::pad_key_to_u8(&key);
        let mut detected_frame = None;

        self.gb.key_press(key);
        eprintln!(
            "--- press_traced: pressing {} (game bit ${:02X}) for {} frames ---",
            format_buttons(expected_bit),
            expected_bit,
            hold_frames
        );

        for frame in 0..max_frames {
            self.run_frames(1);

            let joy = self.joypad_snapshot();
            let cpu = self.cpu_snapshot();

            // Release after hold_frames
            if frame == hold_frames {
                self.gb.key_lift(PadKey::from_u8(key_u8));
                eprintln!("  frame {}: RELEASED key", frame);
            }

            let hit = joy.h_joy5 & expected_bit != 0;
            if hit && detected_frame.is_none() {
                detected_frame = Some(frame);
            }

            // Print state for every frame during hold, plus a few after
            if frame < hold_frames + 5 || hit {
                eprintln!(
                    "  frame {:3}: PC=${:04X} hJoyInput={:02X}[{}] hJoyPressed={:02X}[{}] hJoyHeld={:02X}[{}] hJoy5={:02X}[{}] hFrameCtr={:02X} blocked={}{}",
                    frame,
                    cpu.pc,
                    joy.h_joy_input, format_buttons(joy.h_joy_input),
                    joy.h_joy_pressed, format_buttons(joy.h_joy_pressed),
                    joy.h_joy_held, format_buttons(joy.h_joy_held),
                    joy.h_joy5, format_buttons(joy.h_joy5),
                    joy.h_frame_counter,
                    joy.is_blocked(),
                    if hit { " *** DETECTED ***" } else { "" }
                );
            }

            if detected_frame.is_some() && frame > hold_frames {
                break;
            }
        }

        if detected_frame.is_none() {
            eprintln!(
                "  press_traced: button NOT detected in hJoy5 after {} frames!",
                max_frames
            );
            self.dump_state("final state");
        }

        detected_frame
    }

    /// Run frames while tracing joypad state (no key press — observe passively).
    pub fn trace_frames(&mut self, n: u32, label: &str) {
        eprintln!("--- trace_frames: {} ({} frames) ---", label, n);
        for frame in 0..n {
            self.run_frames(1);
            let joy = self.joypad_snapshot();
            // Only print if something interesting is happening
            if joy.h_joy5 != 0 || joy.h_joy_pressed != 0 || joy.is_blocked() {
                eprintln!(
                    "  frame {:3}: hJoyInput={:02X} hJoyPressed={:02X} hJoy5={:02X}[{}] blocked={}",
                    frame,
                    joy.h_joy_input,
                    joy.h_joy_pressed,
                    joy.h_joy5,
                    format_buttons(joy.h_joy5),
                    joy.is_blocked()
                );
            }
        }
    }

    /// Force-clear all joypad blocking flags.
    /// Useful for debugging to isolate whether a block is the root cause.
    pub fn clear_joypad_blocks(&mut self) {
        self.write_mem(H_DISABLE_JOYPAD_POLLING, 0x00);
        let flags5 = self.read_mem(W_STATUS_FLAGS5);
        self.write_mem(W_STATUS_FLAGS5, flags5 & !(1 << 6)); // clear BIT_DISABLE_JOYPAD
        self.write_mem(W_JOY_IGNORE, 0x00);
    }
}

/// Map a PadKey to the Pokemon Yellow button bit format.
fn pad_key_to_game_bit(key: &PadKey) -> u8 {
    match *key {
        PadKey::A => PAD_A,
        PadKey::B => PAD_B,
        PadKey::Select => PAD_SELECT,
        PadKey::Start => PAD_START,
        PadKey::Right => PAD_RIGHT,
        PadKey::Left => PAD_LEFT,
        PadKey::Up => PAD_UP,
        PadKey::Down => PAD_DOWN,
    }
}
