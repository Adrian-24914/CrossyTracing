#![allow(dead_code)]

use crate::input::InputState;
use std::cell::Cell;

pub struct NativeWindow {
    client_size: Cell<(usize, usize)>,
}

impl NativeWindow {
    pub fn new(_title: &str, width: usize, height: usize) -> Self {
        Self {
            client_size: Cell::new((width, height)),
        }
    }

    pub fn pump_messages(&self) -> Option<InputState> {
        Some(InputState::new(Vec::new(), |_| false))
    }

    pub fn set_title(&self, _title: &str) {}

    pub fn set_client_size(&self, width: usize, height: usize) {
        self.client_size.set((width, height));
    }

    pub fn client_size(&self) -> (usize, usize) {
        self.client_size.get()
    }

    pub fn present_scaled(&self, pixels: &[u32], source_width: usize, source_height: usize) {
        assert_eq!(pixels.len(), source_width * source_height);
    }
}
