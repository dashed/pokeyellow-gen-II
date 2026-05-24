use boytacean::pad::PadKey;

use crate::TestHarness;

pub enum InputAction {
    Press(PadKey, u32),
    Hold(PadKey, u32),
    Release(PadKey),
    Wait(u32),
    WaitForMemory(u16, u8, u32), // addr, expected_value, max_frames
}

pub struct InputScript {
    actions: Vec<InputAction>,
}

impl Default for InputScript {
    fn default() -> Self {
        Self::new()
    }
}

impl InputScript {
    pub fn new() -> Self {
        Self { actions: vec![] }
    }

    pub fn press(mut self, key: PadKey, frames: u32) -> Self {
        self.actions.push(InputAction::Press(key, frames));
        self
    }

    pub fn hold(mut self, key: PadKey, frames: u32) -> Self {
        self.actions.push(InputAction::Hold(key, frames));
        self
    }

    pub fn release(mut self, key: PadKey) -> Self {
        self.actions.push(InputAction::Release(key));
        self
    }

    pub fn wait(mut self, frames: u32) -> Self {
        self.actions.push(InputAction::Wait(frames));
        self
    }

    pub fn wait_for(mut self, addr: u16, value: u8, max_frames: u32) -> Self {
        self.actions
            .push(InputAction::WaitForMemory(addr, value, max_frames));
        self
    }

    pub fn run(self, harness: &mut TestHarness) {
        for action in self.actions {
            match action {
                InputAction::Press(key, frames) => harness.press(key, frames),
                InputAction::Hold(key, frames) => harness.hold(key, frames),
                InputAction::Release(key) => harness.release(key),
                InputAction::Wait(frames) => {
                    harness.run_frames(frames);
                }
                InputAction::WaitForMemory(addr, value, max) => {
                    harness.wait_for_memory(addr, |v| v == value, max);
                }
            }
        }
    }
}
