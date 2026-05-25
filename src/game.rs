use std::{
    io::{self, BufWriter, Write},
    time::{Duration, Instant},
};

use crossterm::{
    cursor, execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};

use crate::config::Config;
use crate::{audio::AudioEngine, settings::Settings};

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

pub struct Game {
    grid: Vec<Vec<Cell>>,
    pub width: usize,
    pub height: usize,
    pub num_mines: i32,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub camera_x: usize,
    pub camera_y: usize,
    pub game_over: bool,
    pub victory: bool,
    pub first_move: bool,
    pub start_time: Option<Instant>,
    pub elapsed: Duration,
    audio: AudioEngine,
}

impl Game {
    pub fn new(cfg: &Config, settings: &Settings) -> Self {
        let row = vec![Cell::default(); cfg.width];
        let grid = vec![row; cfg.height];
        let mut game = Game {
            grid,
            width: cfg.width,
            height: cfg.height,
            num_mines: cfg.num_mines,
            camera_x: 0,
            camera_y: 0,
            cursor_x: 0,
            cursor_y: 0,
            game_over: false,
            victory: false,
            first_move: true,
            start_time: None,
            elapsed: Duration::ZERO,
            audio: AudioEngine::new(),
        };
        game.audio.set_volume(settings.volume);
        game.generate_map();
        game
    }

    pub fn elapsed_secs(&self) -> u64 {
        match self.start_time {
            None => 0,
            Some(t) => {
                if self.game_over || self.victory {
                    self.elapsed.as_secs()
                } else {
                    t.elapsed().as_secs()
                }
            }
        }
    }

    // ── LCG PRNG ──────────────────────────────────────────────────────────────

    pub fn generate_map(&mut self) {
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

    fn set_starting_zone(&mut self) {
        // Trova la prima cella sicura con adjacent_mines == 0
        let safe = 'search: {
            for y in 0..self.height {
                for x in 0..self.width {
                    if !self.grid[y][x].has_mine && self.grid[y][x].adjacent_mines == 0 {
                        break 'search Some((x, y));
                    }
                }
            }
            // Fallback: prima cella non-mina qualsiasi
            'fallback: {
                for y in 0..self.height {
                    for x in 0..self.width {
                        if !self.grid[y][x].has_mine {
                            break 'fallback Some((x, y));
                        }
                    }
                }
                None
            }
        };

        let Some((safe_x, safe_y)) = safe else { return };

        // Shift ciclico delle righe (asse Y)
        // Vogliamo che safe_y finisca su cursor_y
        let dy = (self.cursor_y + self.height - safe_y) % self.height;
        self.grid.rotate_left(self.height - dy); // equivale a rotate_right(dy)

        // Shift ciclico delle colonne (asse X)
        // Vogliamo che safe_x finisca su cursor_x
        let dx = (self.cursor_x + self.width - safe_x) % self.width;
        for row in self.grid.iter_mut() {
            row.rotate_left(self.width - dx); // equivale a rotate_right(dx)
        }

        // Ricalcola adjacent_mines dopo lo shift
        for y in 0..self.height {
            for x in 0..self.width {
                if self.grid[y][x].has_mine {
                    self.grid[y][x].adjacent_mines = 0;
                    continue;
                }
                let mut count: u8 = 0;
                for dy in -1_isize..=1 {
                    for dx in -1_isize..=1 {
                        let ny = y as isize + dy;
                        let nx = x as isize + dx;
                        if ny >= 0
                            && ny < self.height as isize
                            && nx >= 0
                            && nx < self.width as isize
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

    pub fn reveal(&mut self, x: usize, y: usize) {
        if (x > self.width) || (y > self.height) {
            return;
        }
        if self.first_move {
            self.cursor_x = x;
            self.cursor_y = y;
            self.set_starting_zone(); // shifta la mappa sul cursore
            self.start_time = Some(Instant::now()); // avvia il timer
            self.first_move = false;
        }
        if self.grid[y][x].state == CellState::Revealed {
            let mine_count = self.grid[y][x].adjacent_mines;
            if mine_count == 0 {
                return;
            }

            let mut flag_count = 0;
            for dy in -1_isize..=1 {
                for dx in -1_isize..=1 {
                    if dy == 0 && dx == 0 {
                        continue;
                    }
                    let ny = y as isize + dy;
                    let nx = x as isize + dx;
                    if nx >= 0 && nx < self.width as isize && ny >= 0 && ny < self.height as isize {
                        if self.grid[ny as usize][nx as usize].state == CellState::Flagged {
                            flag_count += 1;
                        }
                    }
                }
            }

            if flag_count >= mine_count {
                for dy in -1_isize..=1 {
                    for dx in -1_isize..=1 {
                        if dy == 0 && dx == 0 {
                            continue;
                        }
                        let ny = y as isize + dy;
                        let nx = x as isize + dx;
                        if nx >= 0
                            && nx < self.width as isize
                            && ny >= 0
                            && ny < self.height as isize
                        {
                            if self.grid[ny as usize][nx as usize].state == CellState::Hidden {
                                self.reveal(nx as usize, ny as usize);
                            }
                        }
                    }
                }
            }
            return;
        }

        if self.grid[y][x].state == CellState::Flagged {
            return;
        }

        if self.grid[y][x].has_mine {
            self.game_over = true;
            self.elapsed = self.start_time.map_or(Duration::ZERO, |t| t.elapsed());
            self.audio.play_explosion();
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
        self.audio.play_reveal();

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

    pub fn toggle_flag(&mut self) {
        let cell = &mut self.grid[self.cursor_y][self.cursor_x];
        match cell.state {
            CellState::Hidden => {
                cell.state = CellState::Flagged;
                self.num_mines -= 1;
                self.audio.play_flag();
            }
            CellState::Flagged => {
                cell.state = CellState::Hidden;
                self.num_mines += 1;
                self.audio.play_flag();
            }
            CellState::Revealed => {}
        }
    }

    pub fn check_victory(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let c = &self.grid[y][x];
                if !c.has_mine && c.state != CellState::Revealed {
                    return;
                }
            }
        }
        self.victory = true;
        self.elapsed = self.start_time.map_or(Duration::ZERO, |t| t.elapsed());
        self.audio.play_victory();
    }

    // ── Render ────────────────────────────────────────────────────────────────

    // Each cell is rendered as 3 columns wide and 1 row tall.
    // Overhead consumed by the UI:
    //   - cols: 2 (left pad) + 2 (border ║ on each side)  = 4
    //   - rows: 1 (title) + 2 (controls) + 1 (counter) + 1 (blank)
    //           + 2 (top/bottom border) = ~6
    //
    // max_grid_width  = (term_cols - 4) / 3
    // max_grid_height = term_rows - 10   (clamped to ≥ 1)
    pub fn viewport_size(&self) -> (usize, usize) {
        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        let (cols, rows) = (cols as usize, rows as usize);
        let col_overhead = 4_usize; // left pad + two border chars
        let row_overhead = 6_usize; // title + hint + counter + borders
        let max_grid_w = ((cols.saturating_sub(col_overhead)) / 3).max(1);
        let max_grid_h = rows.saturating_sub(row_overhead).max(1);
        (max_grid_w, max_grid_h)
    }

    pub fn update_camera(&mut self) {
        let (view_w, view_h) = self.viewport_size();
        if self.cursor_x >= self.camera_x + view_w {
            self.camera_x = self.cursor_x - view_w + 1;
        } else if self.cursor_x < self.camera_x {
            self.camera_x = self.cursor_x;
        }
        if self.cursor_y >= self.camera_y + view_h {
            self.camera_y = self.cursor_y - view_h + 1;
        } else if self.cursor_y < self.camera_y {
            self.camera_y = self.cursor_y;
        }
        if self.camera_x + view_w > self.width {
            self.camera_x = self.width.saturating_sub(view_w);
        }
        if self.camera_y + view_h > self.height {
            self.camera_y = self.height.saturating_sub(view_h);
        }
    }

    pub fn draw(&self) -> io::Result<()> {
        let stdout = io::stdout();
        let mut out = BufWriter::new(stdout.lock());
        execute!(out, cursor::MoveTo(0, 0))?;

        execute!(out, SetForegroundColor(Color::Cyan))?;
        println!("╔═════ MINESWEEPER ═════╗");
        execute!(out, ResetColor)?;

        execute!(out, SetForegroundColor(Color::DarkGrey))?;
        println!("  [↑↓←→] Move  [ENTER] Reveal [SPACE] Flag  [M] Menu  [Q] Quit");
        execute!(out, ResetColor)?;

        execute!(
            out,
            terminal::Clear(ClearType::CurrentLine),
            SetForegroundColor(Color::Yellow)
        )?;
        // Congela il tempo mostrato quando la partita è finita
        let display_time = if self.game_over || self.victory {
            self.elapsed.as_secs()
        } else {
            self.start_time.map_or(0, |t| t.elapsed().as_secs())
        };
        println!(
            "  Mines: {}    Time: {}:{}",
            self.num_mines,
            display_time / 60,
            display_time % 60
        );
        execute!(out, ResetColor)?;

        let (max_w, max_h) = self.viewport_size();
        let view_w = max_w.min(self.width);
        let view_h = max_h.min(self.height);
        let border_color = Color::Cyan;

        if self.camera_y == 0 {
            execute!(
                out,
                terminal::Clear(ClearType::CurrentLine),
                SetForegroundColor(border_color)
            )?;
            print!("  ");
            if self.camera_x == 0 {
                print!("╔");
            } else {
                print!("═");
            }
            print!("{}", "═".repeat(view_w * 3));
            if self.camera_x + view_w >= self.width {
                println!("╗");
            } else {
                println!("═");
            }
        }

        for y_offset in 0..view_h {
            let y = self.camera_y + y_offset;

            print!("  ");

            if self.camera_x == 0 {
                execute!(out, SetForegroundColor(border_color))?;
                print!("║");
            } else {
                print!(" ");
            }

            for x_offset in 0..view_w {
                let x = self.camera_x + x_offset;
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
                        print!(" F ");
                    }
                    CellState::Revealed => {
                        if cell.has_mine {
                            execute!(out, SetForegroundColor(Color::Red))?;
                            print!(" * ");
                        } else if cell.adjacent_mines == 0 {
                            print!("   ");
                        } else {
                            let color = match cell.adjacent_mines {
                                1 => Color::Blue,
                                2 => Color::Green,
                                3 => Color::Red,
                                4 => Color::Magenta,
                                5 => Color::DarkBlue,
                                6 => Color::DarkGreen,
                                7 => Color::DarkRed,
                                8 => Color::DarkMagenta,
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

            if self.camera_x + view_w >= self.width {
                execute!(out, SetForegroundColor(border_color))?;
                println!("║");
            } else {
                println!();
            }
        }

        if self.camera_y + view_h >= self.height {
            execute!(
                out,
                terminal::Clear(ClearType::CurrentLine),
                SetForegroundColor(border_color)
            )?;
            print!("  ");
            if self.camera_x == 0 {
                print!("╚");
            } else {
                print!("═");
            }
            print!("{}", "═".repeat(view_w * 3));
            if self.camera_x + view_w >= self.width {
                println!("╝");
            } else {
                println!("═");
            }
        }

        execute!(out, ResetColor)?;

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
            println!("  ██╗   ██╗██╗ ██████╗████████╗ ██████╗ ██████╗ ██╗   ██╗██╗");
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
