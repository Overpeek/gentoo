use std::collections::HashMap;

use winit::event::{VirtualKeyCode, KeyboardInput, ElementState, WindowEvent};

pub struct Input {
    keymap: HashMap<VirtualKeyCode, bool>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keymap: HashMap::new(),
        }
    }

    pub fn key_held(&self, key: VirtualKeyCode) -> bool {
        if let Some(value) = self.keymap.get(&key) {
            *value
        } else {
            false
        }
    }

    pub fn update_key(&mut self, input: &KeyboardInput) {
        input.virtual_keycode.map(|keycode| {
            self.keymap.insert(
                keycode,
                match input.state {
                    ElementState::Pressed => true,
                    _ => false,
                }
            );
        });
    }

    pub fn update(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.update_key(input),
            _ => (),
        }
    }
}
