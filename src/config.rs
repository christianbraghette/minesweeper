#[derive(Clone)]
pub struct Config {
    pub width: usize,
    pub height: usize,
    pub num_mines: i32,
}

impl Config {
    pub fn beginner() -> Self {
        Config {
            width: 9,
            height: 9,
            num_mines: 10,
        }
    }
    pub fn intermediate() -> Self {
        Config {
            width: 16,
            height: 16,
            num_mines: 40,
        }
    }
    pub fn expert() -> Self {
        Config {
            width: 30,
            height: 16,
            num_mines: 99,
        }
    }
    pub fn tryhard() -> Self {
        Config {
            width: 30,
            height: 20,
            num_mines: 140,
        }
    }
    pub fn extreme() -> Self {
        Config {
            width: 30,
            height: 24,
            num_mines: 200,
        }
    }
}
