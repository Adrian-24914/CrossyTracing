#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Pause,
    Reset,
    Orthographic,
    DisplayPerformance,
    DisplayHigh,
    DisplayUltra,
    Start,
}

impl Key {
    const ALL: [Self; 11] = [
        Self::Left,
        Self::Right,
        Self::Up,
        Self::Down,
        Self::Pause,
        Self::Reset,
        Self::Orthographic,
        Self::DisplayPerformance,
        Self::DisplayHigh,
        Self::DisplayUltra,
        Self::Start,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Up => 2,
            Self::Down => 3,
            Self::Pause => 4,
            Self::Reset => 5,
            Self::Orthographic => 6,
            Self::DisplayPerformance => 7,
            Self::DisplayHigh => 8,
            Self::DisplayUltra => 9,
            Self::Start => 10,
        }
    }
}

#[derive(Default)]
pub struct InputState {
    pressed: Vec<Key>,
    held: [bool; Key::ALL.len()],
}

impl InputState {
    pub(crate) fn new(pressed: Vec<Key>, is_held: impl Fn(Key) -> bool) -> Self {
        let mut held = [false; Key::ALL.len()];
        for key in Key::ALL {
            held[key.index()] = is_held(key);
        }
        Self { pressed, held }
    }

    pub fn pressed(&self) -> &[Key] {
        &self.pressed
    }

    pub fn was_pressed(&self, key: Key) -> bool {
        self.pressed.contains(&key)
    }

    pub fn is_held(&self, key: Key) -> bool {
        self.held[key.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressed_and_held_keys_keep_separate_semantics() {
        let input = InputState::new(vec![Key::Pause], |key| key == Key::Left);

        assert!(input.was_pressed(Key::Pause));
        assert!(!input.is_held(Key::Pause));
        assert!(!input.was_pressed(Key::Left));
        assert!(input.is_held(Key::Left));
    }

    #[test]
    fn default_input_is_empty() {
        let input = InputState::default();

        assert!(input.pressed().is_empty());
        assert!(Key::ALL.into_iter().all(|key| !input.is_held(key)));
    }
}
