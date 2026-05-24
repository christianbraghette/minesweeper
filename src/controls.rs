#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Reveal,
    Flag,
    Menu,
    Quit,
}

impl Action {
    pub fn name(&self) -> &'static str {
        match self {
            Action::MoveUp => "Move Up",
            Action::MoveDown => "Move Down",
            Action::MoveLeft => "Move Left",
            Action::MoveRight => "Move Right",
            Action::Reveal => "Reveal Cell",
            Action::Flag => "Flag Cell",
            Action::Menu => "Back to Menu",
            Action::Quit => "Quit",
        }
    }
}
