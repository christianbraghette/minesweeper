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
            Action::MoveUp => "Muoviti Su",
            Action::MoveDown => "Muoviti Giù",
            Action::MoveLeft => "Muoviti Sinistra",
            Action::MoveRight => "Muoviti Destra",
            Action::Reveal => "Rivela Cella",
            Action::Flag => "Piazza Bandiera",
            Action::Menu => "Torna al Menu",
            Action::Quit => "Esci",
        }
    }
}
