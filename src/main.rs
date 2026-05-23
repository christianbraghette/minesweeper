use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use gilrs::{Button, Event as GEvent, Gilrs};
use rodio::{OutputStream, Sink, Source, source::SineWave};
use std::collections::HashMap;
use std::io::{self, BufWriter, Write};
use std::time::{Duration, Instant};

// ─── Tipi di Azione Rimappabili ──────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Action {
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
    fn name(&self) -> &'static str {
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

// ─── Audio Engine Semplice (Senza file esterni) ───────────────────────────────

struct AudioEngine {
    _stream: Option<OutputStream>,
    sink: Option<Sink>,
}

impl AudioEngine {
    fn new() -> Self {
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                // Cediamo la proprietà dello stream per mantenerlo in vita nel thread
                return AudioEngine {
                    _stream: Some(stream),
                    sink: Some(sink),
                };
            }
        }
        AudioEngine {
            _stream: None,
            sink: None,
        }
    }

    fn play_reveal(&self) {
        if let Some(ref sink) = self.sink {
            sink.append(SineWave::new(440.0).take_duration(Duration::from_millis(50)));
        }
    }

    fn play_flag(&self) {
        if let Some(ref sink) = self.sink {
            sink.append(SineWave::new(600.0).take_duration(Duration::from_millis(70)));
        }
    }

    fn play_explosion(&self) {
        if let Some(ref sink) = self.sink {
            sink.append(SineWave::new(100.0).take_duration(Duration::from_millis(400)));
        }
    }

    fn play_victory(&self) {
        if let Some(ref sink) = self.sink {
            sink.append(SineWave::new(587.33).take_duration(Duration::from_millis(150)));
            sink.append(SineWave::new(880.0).take_duration(Duration::from_millis(300)));
        }
    }
}

// ─── Data types standard di gioco ─────────────────────────────────────────────

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

// ─── Struttura Globale Impostazioni ──────────────────────────────────────────

struct Settings {
    key_bindings: HashMap<KeyCode, Action>,
}

impl Settings {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(KeyCode::Up, Action::MoveUp);
        bindings.insert(KeyCode::Char('w'), Action::MoveUp);
        bindings.insert(KeyCode::Char('W'), Action::MoveUp);
        bindings.insert(KeyCode::Down, Action::MoveDown);
        bindings.insert(KeyCode::Char('s'), Action::MoveDown);
        bindings.insert(KeyCode::Char('S'), Action::MoveDown);
        bindings.insert(KeyCode::Left, Action::MoveLeft);
        bindings.insert(KeyCode::Char('a'), Action::MoveLeft);
        bindings.insert(KeyCode::Char('A'), Action::MoveLeft);
        bindings.insert(KeyCode::Right, Action::MoveRight);
        bindings.insert(KeyCode::Char('d'), Action::MoveRight);
        bindings.insert(KeyCode::Char('D'), Action::MoveRight);
        bindings.insert(KeyCode::Enter, Action::Reveal);
        bindings.insert(KeyCode::Char(' '), Action::Flag);
        bindings.insert(KeyCode::Char('m'), Action::Menu);
        bindings.insert(KeyCode::Char('M'), Action::Menu);
        bindings.insert(KeyCode::Char('q'), Action::Quit);
        bindings.insert(KeyCode::Char('Q'), Action::Quit);
        Settings {
            key_bindings: bindings,
        }
    }
}

// ─── Stato Navigazione Menu Principale ────────────────────────────────────────

enum MenuResult {
    Play(Config),
    Options,
    Quit,
}

// ─── Logica dei Menu (Tastiera, Mouse e Gamepad abilitati) ─────────────────────

fn run_main_menu(gilrs: &mut Gilrs) -> io::Result<MenuResult> {
    let mut selected = 0;
    let options = vec![
        "Beginner         ( 9× 9, 10)",
        "Intermediate     (16×16, 40)",
        "Expert           (30×16, 99)",
        "Tryhard          (30×20,140)",
        "Extreme          (30×24,200)",
        "Impostazioni Comandi",
        "Esci dal Gioco",
    ];

    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(ClearType::All))?;

    loop {
        execute!(stdout, cursor::MoveTo(0, 0))?;
        println!();
        println!("  ╔════════════════════════════════════════════╗");
        println!("  ║                MINESWEEPER                 ║");
        println!("  ╠════════════════════════════════════════════╣");
        println!("  ║  Select difficulty:                        ║");
        println!("  ║                                            ║");

        for (i, opt) in options.iter().enumerate() {
            if i == selected {
                execute!(stdout, SetForegroundColor(Color::Cyan))?;
                println!("  ║  ► {:<40} ║", opt);
                execute!(stdout, ResetColor)?;
            } else {
                println!("  ║    {:<40} ║", opt);
            }
        }
        println!("  ╚════════════════════════════════════════════╝");
        stdout.flush()?;

        // Gestione Input Gamepad
        while let Some(GEvent { event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(Button::DPadUp, _) => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                gilrs::EventType::ButtonPressed(Button::DPadDown, _) => {
                    if selected < options.len() - 1 {
                        selected += 1;
                    }
                }
                gilrs::EventType::ButtonPressed(Button::South, _) => {
                    // Tasto A (Xbox) / X (PlayStation)
                    return Ok(match_menu_selection(selected));
                }
                _ => {}
            }
        }

        // Gestione Input Tastiera / Mouse
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(ke) if ke.kind == KeyEventKind::Press => match ke.code {
                    KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                        if selected < options.len() - 1 {
                            selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        return Ok(match_menu_selection(selected));
                    }
                    _ => {}
                },
                Event::Mouse(me) if me.kind == MouseEventKind::Down(MouseButton::Left) => {
                    // Calcolo empirico della riga del terminale cliccata rispetto al menu grafico
                    let row = me.row as isize;
                    let start_row = 6;
                    if row >= start_row && row < start_row + options.len() as isize {
                        let clicked = (row - start_row) as usize;
                        if clicked == selected {
                            return Ok(match_menu_selection(selected));
                        } else {
                            selected = clicked;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn match_menu_selection(selected: usize) -> MenuResult {
    match selected {
        0 => MenuResult::Play(Config::beginner()),
        1 => MenuResult::Play(Config::intermediate()),
        2 => MenuResult::Play(Config::expert()),
        3 => MenuResult::Play(Config::tryhard()),
        4 => MenuResult::Play(Config::extreme()),
        5 => MenuResult::Options,
        _ => MenuResult::Quit,
    }
}

// ─── Menu Opzioni per la Configurazione dei Tasti ───────────────────────────

fn run_options_menu(settings: &mut Settings) -> io::Result<()> {
    let mut stdout = io::stdout();
    let actions = vec![
        Action::MoveUp,
        Action::MoveDown,
        Action::MoveLeft,
        Action::MoveRight,
        Action::Reveal,
        Action::Flag,
        Action::Menu,
        Action::Quit,
    ];
    let mut selected = 0;

    execute!(stdout, terminal::Clear(ClearType::All))?;

    loop {
        execute!(stdout, cursor::MoveTo(0, 0))?;
        println!();
        println!("  ╔════════════════════════════════════════════╗");
        println!("  ║             IMPOSTAZIONI TASTI             ║");
        println!("  ╠════════════════════════════════════════════╣");
        println!("  ║ Seleziona l'azione e premi INVIO per      ║");
        println!("  ║ riassegnarla. Premi ESC per uscire.        ║");
        println!("  ╠════════════════════════════════════════════╣");

        for (i, action) in actions.iter().enumerate() {
            let keys: Vec<String> = settings
                .key_bindings
                .iter()
                .filter(|(_, a)| **a == *action)
                .map(|(k, _)| format!("{:?}", k))
                .collect();
            let keys_str = keys.join(", ");

            let line = format!("{}: {}", action.name(), keys_str);
            if i == selected {
                execute!(stdout, SetForegroundColor(Color::Cyan))?;
                println!("  ║ ► {:<40} ║", line);
                execute!(stdout, ResetColor)?;
            } else {
                println!("  ║   {:<40} ║", line);
            }
        }
        println!("  ╚════════════════════════════════════════════╝");
        stdout.flush()?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(ke) if ke.kind == KeyEventKind::Press => match ke.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if selected < actions.len() - 1 {
                            selected += 1;
                        }
                    }
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        // Flusha lo schermo chiedendo il nuovo tasto
                        execute!(stdout, terminal::Clear(ClearType::All))?;
                        println!(
                            "\n  Premi il nuovo tasto per l'azione: [{}]...",
                            actions[selected].name()
                        );
                        stdout.flush()?;

                        loop {
                            if let Event::Key(new_ke) = event::read()? {
                                if new_ke.kind == KeyEventKind::Press {
                                    // Rimuove vecchie associazioni identiche per pulizia
                                    settings
                                        .key_bindings
                                        .retain(|_, &mut a| a != actions[selected]);
                                    settings.key_bindings.insert(new_ke.code, actions[selected]);
                                    break;
                                }
                            }
                        }
                        execute!(stdout, terminal::Clear(ClearType::All))?;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}

// ─── Game ─────────────────────────────────────────────────────────────────────

struct Game {
    grid: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
    num_mines: usize,
    cursor_x: usize,
    cursor_y: usize,
    camera_x: usize,
    camera_y: usize,
    game_over: bool,
    victory: bool,
    audio: AudioEngine,
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
            camera_x: 0,
            camera_y: 0,
            cursor_x: 0,
            cursor_y: 0,
            game_over: false,
            victory: false,
            audio: AudioEngine::new(),
        };
        game.generate_map();
        game.reveal_starting_zone();
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

    fn reveal_starting_zone(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.grid[y][x].has_mine && self.grid[y][x].adjacent_mines == 0 {
                    self.reveal(x, y);
                    self.cursor_x = x;
                    self.cursor_y = y;
                    return;
                }
            }
        }

        // --- 2. FALLBACK: Se non esistono celle da '0', cerchiamo la prima cella non-mina
        // che ha almeno un vicino identico (stesso numero di mine adiacenti).
        for y in 0..self.height {
            for x in 0..self.width {
                if self.grid[y][x].has_mine {
                    continue;
                }

                let current_mines = self.grid[y][x].adjacent_mines;

                // Controlliamo i vicini per vedere se ce n'è uno identico
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
                            let neighbor = &self.grid[ny as usize][nx as usize];
                            // Se il vicino non ha mine ed ha lo stesso numero di mine adiacenti
                            if !neighbor.has_mine && neighbor.adjacent_mines == current_mines {
                                self.reveal(x, y);
                                self.cursor_x = x;
                                self.cursor_y = y;
                                return;
                            }
                        }
                    }
                }
            }
        }

        // --- 3. ULTIMA SPIAGGIA: Se la mappa è un incubo matematico e non trova nemmeno coppie identiche,
        // riveliamo semplicemente la primissima cella sicura in alto a sinistra.
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.grid[y][x].has_mine {
                    self.reveal(x, y);
                    self.cursor_x = x;
                    self.cursor_y = y;
                    return;
                }
            }
        }
    }

    // ── Reveal ────────────────────────────────────────────────────────────────

    fn reveal(&mut self, x: usize, y: usize) {
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

    fn toggle_flag(&mut self) {
        let cell = &mut self.grid[self.cursor_y][self.cursor_x];
        match cell.state {
            CellState::Hidden => {
                cell.state = CellState::Flagged;
                self.num_mines = self.num_mines.saturating_sub(1);
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
        self.audio.play_victory();
    }

    // ── Render ────────────────────────────────────────────────────────────────

    // Each cell is rendered as 3 columns wide and 1 row tall.
    // Overhead consumed by the UI:
    //   - cols: 2 (left pad) + 2 (border ║ on each side)  = 4
    //   - rows: 1 (title) + 1 (controls) + 1 (counter) + 1 (blank)
    //           + 2 (top/bottom border) = ~6
    //
    // max_grid_width  = (term_cols - 4) / 3
    // max_grid_height = term_rows - 10   (clamped to ≥ 1)
    fn viewport_size(&self) -> (usize, usize) {
        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        let (cols, rows) = (cols as usize, rows as usize);
        let col_overhead = 4_usize; // left pad + two border chars
        let row_overhead = 6_usize; // title + hint + counter + borders
        let max_grid_w = ((cols.saturating_sub(col_overhead)) / 3).max(1);
        let max_grid_h = rows.saturating_sub(row_overhead).max(1);
        (max_grid_w, max_grid_h)
    }

    fn update_camera(&mut self) {
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

    fn draw(&self) -> io::Result<()> {
        let stdout = io::stdout();
        let mut out = BufWriter::new(stdout.lock());
        execute!(out, cursor::MoveTo(0, 0))?;

        execute!(out, SetForegroundColor(Color::Cyan))?;
        println!("╔═════ MINESWEEPER ═════╗");
        execute!(out, ResetColor)?;

        execute!(out, SetForegroundColor(Color::DarkGrey))?;
        println!("  [↑↓←→] Move  [ENTER] Reveal  [SPACE] Flag  [M] Menu  [Q] Quit");
        execute!(out, ResetColor)?;

        execute!(out, SetForegroundColor(Color::Yellow))?;
        println!("  Mines: {}", self.num_mines);
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
                                4 => Color::DarkBlue,
                                5 => Color::DarkRed,
                                6 => Color::Cyan,
                                7 => Color::Magenta,
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

// ─── Post Game Action ────────────────────────────────────────────────────────

enum PostGameAction {
    PlayAgain,
    Menu,
    Quit,
}

// ─── Session ─────────────────────────────────────────────────────────────────

fn play(cfg: &Config, settings: &Settings, gilrs: &mut Gilrs) -> io::Result<PostGameAction> {
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(ClearType::All))?;

    let mut game = Game::new(cfg);

    // Drain stale events (Enter from menu or previous session end).
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    game.update_camera();
    game.draw()?;

    let mut last_gamepad_move = Instant::now();

    loop {
        if game.game_over || game.victory {
            loop {
                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(ke) = event::read()? {
                        if ke.kind == KeyEventKind::Press {
                            if let Some(action) = settings.key_bindings.get(&ke.code) {
                                match action {
                                    Action::Reveal => return Ok(PostGameAction::PlayAgain),
                                    Action::Menu => return Ok(PostGameAction::Menu),
                                    Action::Quit => return Ok(PostGameAction::Quit),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Input Gamepad ────────────────────────────────────────────────────
        while let Some(GEvent { event, .. }) = gilrs.next_event() {
            if last_gamepad_move.elapsed() > Duration::from_millis(150) {
                match event {
                    gilrs::EventType::ButtonPressed(Button::DPadUp, _) => {
                        if game.cursor_y > 0 {
                            game.cursor_y -= 1;
                        }
                        last_gamepad_move = Instant::now();
                    }
                    gilrs::EventType::ButtonPressed(Button::DPadDown, _) => {
                        if game.cursor_y < game.height - 1 {
                            game.cursor_y += 1;
                        }
                        last_gamepad_move = Instant::now();
                    }
                    gilrs::EventType::ButtonPressed(Button::DPadLeft, _) => {
                        if game.cursor_x > 0 {
                            game.cursor_x -= 1;
                        }
                        last_gamepad_move = Instant::now();
                    }
                    gilrs::EventType::ButtonPressed(Button::DPadRight, _) => {
                        if game.cursor_x < game.width - 1 {
                            game.cursor_x += 1;
                        }
                        last_gamepad_move = Instant::now();
                    }
                    gilrs::EventType::ButtonPressed(Button::South, _) => {
                        game.reveal(game.cursor_x, game.cursor_y);
                        game.check_victory();
                    }
                    gilrs::EventType::ButtonPressed(Button::East, _) => {
                        game.toggle_flag();
                    }
                    _ => {}
                }
            }
        }

        // ── Input Tastiera e Mouse ───────────────────────────────────────────
        if event::poll(Duration::from_millis(20))? {
            match event::read()? {
                Event::Resize(_, _) => {
                    execute!(stdout, terminal::Clear(ClearType::All))?;
                }
                Event::Key(ke) if ke.kind == KeyEventKind::Press => {
                    if let Some(action) = settings.key_bindings.get(&ke.code) {
                        match action {
                            Action::MoveUp => {
                                if game.cursor_y > 0 {
                                    game.cursor_y -= 1;
                                }
                            }
                            Action::MoveDown => {
                                if game.cursor_y < game.height - 1 {
                                    game.cursor_y += 1;
                                }
                            }
                            Action::MoveLeft => {
                                if game.cursor_x > 0 {
                                    game.cursor_x -= 1;
                                }
                            }
                            Action::MoveRight => {
                                if game.cursor_x < game.width - 1 {
                                    game.cursor_x += 1;
                                }
                            }
                            Action::Reveal => {
                                game.reveal(game.cursor_x, game.cursor_y);
                                game.check_victory();
                            }
                            Action::Flag => game.toggle_flag(),
                            Action::Menu => return Ok(PostGameAction::Menu),
                            Action::Quit => return Ok(PostGameAction::Quit),
                        }
                    }
                }
                Event::Mouse(me) => {
                    let (view_w, view_h) = game.viewport_size();
                    let grid_start_col = 3; // Allineamento dato dai bordi ("  ║")
                    let grid_start_row = 4; // Righe di overhead dell'interfaccia

                    let mouse_col = me.column as isize;
                    let mouse_row = me.row as isize;

                    // Calcola su quale cella della griglia visibile si trova il mouse
                    let cell_click_x = (mouse_col - grid_start_col) / 3;
                    let cell_click_y = mouse_row - grid_start_row;

                    if cell_click_x >= 0
                        && cell_click_x < view_w as isize
                        && cell_click_y >= 0
                        && cell_click_y < view_h as isize
                    {
                        let map_x = game.camera_x + cell_click_x as usize;
                        let map_y = game.camera_y + cell_click_y as usize;

                        game.cursor_x = map_x;
                        game.cursor_y = map_y;

                        if me.kind == MouseEventKind::Down(MouseButton::Left) {
                            game.reveal(map_x, map_y);
                            game.check_victory();
                        } else if me.kind == MouseEventKind::Down(MouseButton::Right) {
                            game.toggle_flag();
                        }
                    }
                }
                _ => {}
            }
            game.update_camera();
            game.draw()?;
        }
    }
}

// ─── Main Entry Point ────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let mut settings = Settings::default();
    let mut gilrs = Gilrs::new().unwrap();

    'app: loop {
        // Abilitiamo la raw mode e il mouse anche per i menu in modo che la selezione tastiera/mouse sia fluida
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            event::EnableMouseCapture
        )?;

        let menu_res = run_main_menu(&mut gilrs)?;

        match menu_res {
            MenuResult::Options => {
                run_options_menu(&mut settings)?;
                continue 'app;
            }
            MenuResult::Quit => {
                execute!(
                    stdout,
                    event::DisableMouseCapture,
                    cursor::Show,
                    terminal::LeaveAlternateScreen
                )?;
                terminal::disable_raw_mode()?;
                break 'app;
            }
            MenuResult::Play(cfg) => {
                let action = loop {
                    match play(&cfg, &settings, &mut gilrs)? {
                        PostGameAction::PlayAgain => continue,
                        other => break other,
                    }
                };

                execute!(
                    stdout,
                    event::DisableMouseCapture,
                    cursor::Show,
                    terminal::LeaveAlternateScreen
                )?;
                terminal::disable_raw_mode()?;

                match action {
                    PostGameAction::Menu => continue 'app,
                    _ => break 'app,
                }
            }
        }
    }
    Ok(())
}
