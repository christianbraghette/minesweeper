use crate::controls::Action;
use crossterm::event::KeyCode;
use std::collections::HashMap;

pub struct Settings {
    pub key_bindings: HashMap<KeyCode, Action>,
    pub gamepad: bool,
    pub volume: u8,
}

impl Settings {
    pub fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(KeyCode::Up, Action::MoveUp);
        bindings.insert(KeyCode::Down, Action::MoveDown);
        bindings.insert(KeyCode::Left, Action::MoveLeft);
        bindings.insert(KeyCode::Right, Action::MoveRight);
        bindings.insert(KeyCode::Enter, Action::Reveal);
        bindings.insert(KeyCode::Char(' '), Action::Flag);
        bindings.insert(KeyCode::Char('m'), Action::Menu);
        bindings.insert(KeyCode::Char('q'), Action::Quit);
        Settings {
            key_bindings: bindings,
            gamepad: false,
            volume: 255,
        }
    }
}
