use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, BufRead, Write, stdout};
use std::time::Duration;

// ─── Data types ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum CellState {
    Hidden,
    Revealed,
    Flagged,
}

#[derive(Clone)]
struct Cell {
    has_mine: bool,
    adjacent_mines: u8,
    state: CellState,
}

impl Cell {
    fn default() -> Self {
        Cell {
            has_mine: false,
            adjacent_mines: 0,
            state: CellState::Hidden,
        }
    }
}

// ─── Difficulty ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Config {
    width: usize,
    height: usize,
    num_mines: usize,
}

impl Config {
    fn beginner() -> Self {
        Config {
            width: 9,
            height: 9,
            num_mines: 10,
        }
    }
    fn intermediate() -> Self {
        Config {
            width: 16,
            height: 16,
            num_mines: 40,
        }
    }
    fn expert() -> Self {
        Config {
            width: 30,
            height: 16,
            num_mines: 99,
        }
    }
    fn tryhard() -> Self {
        Config {
            width: 30,
            height: 20,
            num_mines: 140,
        }
    }
    fn extreme() -> Self {
        Config {
            width: 30,
            height: 24,
            num_mines: 200,
        }
    }
}

// ─── Difficulty menu (runs before raw-mode) ───────────────────────────────────

/// Shows the menu in normal (cooked) terminal mode so `stdin` line-reading works
/// cleanly. Arrow-key navigation is NOT available here since we avoid raw mode
/// during input; we use plain numeric entry instead.
fn show_menu() -> io::Result<Config> {
    let stdin = io::stdin();

    loop {
        println!();
        println!("╔══════════════════════════════╗");
        println!("║      MINESWEEPER – RUST      ║");
        println!("╠══════════════════════════════╣");
        println!("║  Select difficulty:          ║");
        println!("║                              ║");
        println!("║  1. Beginner     ( 9× 9, 10) ║");
        println!("║  2. Intermediate (16×16, 40) ║");
        println!("║  3. Expert       (30×16, 99) ║");
        println!("║  4. Tryhard      (30×20,140) ║");
        println!("║  5. Extreme      (30×24,200) ║");
        println!("╚══════════════════════════════╝");
        print!("Choice [1-5]: ");
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;

        match line.trim() {
            "1" => return Ok(Config::beginner()),
            "2" => return Ok(Config::intermediate()),
            "3" => return Ok(Config::expert()),
            "4" => return Ok(Config::tryhard()),
            "5" => return Ok(Config::extreme()),
            _ => println!("Please enter a number between 1 and 5.\n"),
        }
    }
}

// ─── Game ─────────────────────────────────────────────────────────────────────

struct Game {
    grid: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
    num_mines: usize,
    cursor_x: usize,
    cursor_y: usize,
    game_over: bool,
    victory: bool,
}

impl Game {
    fn new(cfg: &Config) -> Self {
        let row = vec![Cell::default(); cfg.width];
        let grid = vec![row; cfg.height];
        let mut game = Game {
            grid,
            width: cfg.width,
            height: cfg.height,
            num_mines: cfg.num_mines,
            cursor_x: 0,
            cursor_y: 0,
            game_over: false,
            victory: false,
        };
        game.generate_map();
        game
    }

    // ── LCG PRNG ──────────────────────────────────────────────────────────────

    fn generate_map(&mut self) {
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let mut mines_placed = 0;
        while mines_placed < self.num_mines {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let x = (seed >> 33) as usize % self.width;
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let y = (seed >> 33) as usize % self.height;

            if !self.grid[y][x].has_mine {
                self.grid[y][x].has_mine = true;
                mines_placed += 1;
            }
        }

        for y in 0..self.height {
            for x in 0..self.width {
                if self.grid[y][x].has_mine {
                    continue;
                }
                let mut count: u8 = 0;
                for dy in -1_isize..=1 {
                    for dx in -1_isize..=1 {
                        let ny = y as isize + dy;
                        let nx = x as isize + dx;
                        if nx >= 0
                            && nx < self.width as isize
                            && ny >= 0
                            && ny < self.height as isize
                            && self.grid[ny as usize][nx as usize].has_mine
                        {
                            count += 1;
                        }
                    }
                }
                self.grid[y][x].adjacent_mines = count;
            }
        }
    }

    // ── Reveal ────────────────────────────────────────────────────────────────

    fn reveal(&mut self, x: usize, y: usize) {
        if self.grid[y][x].state != CellState::Hidden {
            return;
        }
        if self.grid[y][x].has_mine {
            self.game_over = true;
            // Reveal all mines immediately
            for ry in 0..self.height {
                for rx in 0..self.width {
                    if self.grid[ry][rx].has_mine {
                        self.grid[ry][rx].state = CellState::Revealed;
                    }
                }
            }
            return;
        }
        self.grid[y][x].state = CellState::Revealed;
        if self.grid[y][x].adjacent_mines == 0 {
            for dy in -1_isize..=1 {
                for dx in -1_isize..=1 {
                    if dy == 0 && dx == 0 {
                        continue;
                    }
                    let ny = y as isize + dy;
                    let nx = x as isize + dx;
                    if nx >= 0 && nx < self.width as isize && ny >= 0 && ny < self.height as isize {
                        self.reveal(nx as usize, ny as usize);
                    }
                }
            }
        }
    }

    fn toggle_flag(&mut self) {
        let cell = &mut self.grid[self.cursor_y][self.cursor_x];
        match cell.state {
            CellState::Hidden => cell.state = CellState::Flagged,
            CellState::Flagged => cell.state = CellState::Hidden,
            CellState::Revealed => {}
        }
    }

    fn check_victory(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let c = &self.grid[y][x];
                if !c.has_mine && c.state != CellState::Revealed {
                    return;
                }
            }
        }
        self.victory = true;
    }

    // ── Render ────────────────────────────────────────────────────────────────

    fn draw(&self) -> io::Result<()> {
        let mut out = stdout();
        // Move to top-left and overwrite in place — no full clear avoids flicker.
        // Each line is cleared before writing so stale chars are erased.
        execute!(out, cursor::MoveTo(0, 0))?;

        // Title bar
        execute!(out, SetForegroundColor(Color::Cyan))?;
        execute!(out, terminal::Clear(ClearType::CurrentLine))?;
        println!("╔══ MINESWEEPER – RUST ══╗");
        execute!(out, ResetColor)?;

        // Controls hint
        execute!(out, SetForegroundColor(Color::DarkGrey))?;
        execute!(out, terminal::Clear(ClearType::CurrentLine))?;
        println!("  ↑↓←→ move  [ENTER] reveal  [SPACE] flag  [Q] quit");
        execute!(out, ResetColor)?;

        // Mine / flag counter
        let flags: usize = self
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.state == CellState::Flagged)
            .count();
        execute!(out, SetForegroundColor(Color::Yellow))?;
        execute!(out, terminal::Clear(ClearType::CurrentLine))?;
        println!("  Mines: {}  Flags: {}\n", self.num_mines, flags);
        execute!(out, ResetColor)?;

        // Grid — surrounded by a box border
        // Each cell is 3 chars wide, so inner width = width * 3.
        let inner_width = self.width * 3;
        let border_color = Color::DarkGrey;

        // Top border
        execute!(out, terminal::Clear(ClearType::CurrentLine))?;
        execute!(out, SetForegroundColor(border_color))?;
        print!("  ╔");
        print!("{}", "═".repeat(inner_width));
        println!("╗");

        for y in 0..self.height {
            execute!(out, terminal::Clear(ClearType::CurrentLine))?;
            execute!(out, SetForegroundColor(border_color))?;
            print!("  ║");

            for x in 0..self.width {
                let cell = &self.grid[y][x];
                let is_cursor = x == self.cursor_x && y == self.cursor_y;

                if is_cursor {
                    execute!(
                        out,
                        SetForegroundColor(Color::Black),
                        SetBackgroundColor(Color::White)
                    )?;
                }

                match cell.state {
                    CellState::Hidden => {
                        if !is_cursor {
                            execute!(out, SetForegroundColor(Color::DarkGrey))?;
                        }
                        print!(" . ");
                    }
                    CellState::Flagged => {
                        if !is_cursor {
                            execute!(out, SetForegroundColor(Color::Yellow))?;
                        }
                        print!(" P ");
                    }
                    CellState::Revealed => {
                        if cell.has_mine {
                            execute!(out, SetForegroundColor(Color::Red))?;
                            print!(" * ");
                        } else if cell.adjacent_mines == 0 {
                            if !is_cursor {
                                execute!(out, SetForegroundColor(Color::DarkGreen))?;
                            }
                            print!("   ");
                        } else {
                            let color = match cell.adjacent_mines {
                                1 => Color::Blue,
                                2 => Color::Green,
                                3 => Color::Red,
                                4 => Color::DarkBlue,
                                5 => Color::DarkRed,
                                6 => Color::Cyan,
                                7 => Color::Magenta,
                                _ => Color::White,
                            };
                            if !is_cursor {
                                execute!(out, SetForegroundColor(color))?;
                            }
                            print!(" {} ", cell.adjacent_mines);
                        }
                    }
                }

                if is_cursor {
                    execute!(out, ResetColor)?;
                }
            }

            // Right border
            execute!(out, SetForegroundColor(border_color))?;
            println!("║");
        }

        // Bottom border
        execute!(out, terminal::Clear(ClearType::CurrentLine))?;
        execute!(out, SetForegroundColor(border_color))?;
        print!("  ╚");
        print!("{}", "═".repeat(inner_width));
        println!("╝");

        execute!(out, ResetColor)?;

        // Clear any leftover lines below the grid (e.g. old banners after resize)
        execute!(out, terminal::Clear(ClearType::CurrentLine))?;

        // Status message
        if self.game_over {
            println!();
            execute!(out, SetForegroundColor(Color::Red))?;
            println!("  ██████╗  ██████╗  ██████╗ ███╗   ███╗");
            println!("  ██╔══██╗██╔═══██╗██╔═══██╗████╗ ████║");
            println!("  ██████╔╝██║   ██║██║   ██║██╔████╔██║");
            println!("  ██╔══██╗██║   ██║██║   ██║██║╚██╔╝██║");
            println!("  ██████╔╝╚██████╔╝╚██████╔╝██║ ╚═╝ ██║");
            println!("  ╚═════╝  ╚═════╝  ╚═════╝ ╚═╝     ╚═╝");
            println!();
            println!("  BOOM! You stepped on a mine.");
            execute!(out, SetForegroundColor(Color::DarkGrey))?;
            println!("  [ENTER] Play again  |  [M] Menu  |  [Q] Quit");
            execute!(out, ResetColor)?;
        } else if self.victory {
            println!();
            execute!(out, SetForegroundColor(Color::Green))?;
            println!("  ██╗   ██╗██╗ ██████╗████████╗ ██████╗ ██████╗██╗   ██╗██╗");
            println!("  ██║   ██║██║██╔════╝╚══██╔══╝██╔═══██╗██╔══██╗╚██╗ ██╔╝██║");
            println!("  ██║   ██║██║██║        ██║   ██║   ██║██████╔╝ ╚████╔╝ ██║");
            println!("  ╚██╗ ██╔╝██║██║        ██║   ██║   ██║██╔══██╗  ╚██╔╝  ╚═╝");
            println!("   ╚████╔╝ ██║╚██████╗   ██║   ╚██████╔╝██║  ██║   ██║   ██╗");
            println!("    ╚═══╝  ╚═╝ ╚═════╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚═╝");
            println!();
            println!("  Congratulations! You cleared the minefield!");
            execute!(out, SetForegroundColor(Color::DarkGrey))?;
            println!("  [ENTER] Play again  |  [M] Menu  |  [Q] Quit");
            execute!(out, ResetColor)?;
        }

        out.flush()?;
        Ok(())
    }
}

// ─── Post-game action ────────────────────────────────────────────────────────

enum PostGameAction {
    PlayAgain, // same config, new board
    Menu,      // go back to difficulty menu
    Quit,      // exit the program
}

// ─── Session ─────────────────────────────────────────────────────────────────

/// Runs one full game session (raw mode already active).
/// Returns what the player wants to do next.
fn play(cfg: &Config) -> io::Result<PostGameAction> {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All))?;

    let mut game = Game::new(cfg);

    // Drain stale events (Enter from menu or previous session end).
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    game.draw()?;
    loop {
        if game.game_over || game.victory {
            // Board frozen — wait for a decisive keypress.
            loop {
                if event::poll(Duration::from_millis(200))? {
                    if let Event::Key(ke) = event::read()? {
                        if ke.kind != KeyEventKind::Press {
                            continue;
                        }
                        match ke.code {
                            KeyCode::Enter => return Ok(PostGameAction::PlayAgain),
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                return Ok(PostGameAction::Menu);
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                return Ok(PostGameAction::Quit);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(ke) = event::read()? {
                if ke.kind != KeyEventKind::Press {
                    continue;
                }
                match ke.code {
                    KeyCode::Up => {
                        if game.cursor_y > 0 {
                            game.cursor_y -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if game.cursor_y < game.height - 1 {
                            game.cursor_y += 1;
                        }
                    }
                    KeyCode::Left => {
                        if game.cursor_x > 0 {
                            game.cursor_x -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if game.cursor_x < game.width - 1 {
                            game.cursor_x += 1;
                        }
                    }
                    KeyCode::Enter => {
                        game.reveal(game.cursor_x, game.cursor_y);
                        game.check_victory();
                    }
                    KeyCode::Char(' ') => {
                        game.toggle_flag();
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(PostGameAction::Quit),
                    _ => {}
                }
                game.draw()?;
            }
        }
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    'app: loop {
        // ── Menu (cooked mode) ────────────────────────────────────────────────
        let cfg = show_menu()?;

        // ── Enter raw / alternate-screen mode ─────────────────────────────────
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

        // ── Session loop: replay without returning to menu ─────────────────────
        let action = loop {
            match play(&cfg)? {
                PostGameAction::PlayAgain => continue, // same config, new board
                other => break other,
            }
        };

        // ── Restore terminal ──────────────────────────────────────────────────
        execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        match action {
            PostGameAction::Menu => continue 'app,
            _ => break 'app, // Quit
        }
    }
    Ok(())
}
