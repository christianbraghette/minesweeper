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
enum StatoCella {
    Coperta,
    Rivelata,
    Bandierina,
}

#[derive(Clone)]
struct Cella {
    ha_mina: bool,
    mine_vicine: u8,
    stato: StatoCella,
}

impl Cella {
    fn default() -> Self {
        Cella {
            ha_mina: false,
            mine_vicine: 0,
            stato: StatoCella::Coperta,
        }
    }
}

// ─── Difficulty ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Config {
    larghezza: usize,
    altezza: usize,
    num_mine: usize,
}

impl Config {
    fn beginner()     -> Self { Config { larghezza:  9, altezza:  9, num_mine:  10 } }
    fn intermediate() -> Self { Config { larghezza: 16, altezza: 16, num_mine:  40 } }
    fn expert()       -> Self { Config { larghezza: 30, altezza: 16, num_mine:  99 } }
}

// ─── Difficulty menu (runs before raw-mode) ───────────────────────────────────

/// Shows the menu in normal (cooked) terminal mode so `stdin` line-reading works
/// cleanly for the custom option. Arrow-key navigation is NOT available here
/// since we avoid raw mode during input; we use plain numeric entry instead.
fn mostra_menu() -> io::Result<Config> {
    let stdin = io::stdin();

    loop {
        println!();
        println!("╔══════════════════════════════╗");
        println!("║      CAMPO MINATO – RUST     ║");
        println!("╠══════════════════════════════╣");
        println!("║  Seleziona difficoltà:       ║");
        println!("║                              ║");
        println!("║  1. Principiante  ( 9× 9,10) ║");
        println!("║  2. Intermedio    (16×16,40) ║");
        println!("║  3. Esperto       (30×16,99) ║");
        println!("║  4. Personalizzato           ║");
        println!("╚══════════════════════════════╝");
        print!("Scelta [1-4]: ");
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;

        match line.trim() {
            "1" => return Ok(Config::beginner()),
            "2" => return Ok(Config::intermediate()),
            "3" => return Ok(Config::expert()),
            "4" => return leggi_config_personalizzata(),
            _   => println!("Inserisci un numero tra 1 e 4.\n"),
        }
    }
}

fn leggi_u(prompt: &str, min: usize, max: usize) -> io::Result<usize> {
    let stdin = io::stdin();
    loop {
        print!("{} ({}-{}): ", prompt, min, max);
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;

        match line.trim().parse::<usize>() {
            Ok(v) if v >= min && v <= max => return Ok(v),
            Ok(_) => println!("  Valore fuori intervallo, riprova."),
            Err(_) => println!("  Inserisci un numero intero valido."),
        }
    }
}

fn leggi_config_personalizzata() -> io::Result<Config> {
    println!("\n--- Configurazione personalizzata ---");
    let larghezza = leggi_u("Larghezza", 5, 50)?;
    let altezza   = leggi_u("Altezza",   5, 50)?;
    let celle_totali = larghezza * altezza;
    let max_mine  = ((celle_totali as f64) * 0.8) as usize;
    let num_mine  = leggi_u("Mine", 1, max_mine)?;
    Ok(Config { larghezza, altezza, num_mine })
}

// ─── Game ─────────────────────────────────────────────────────────────────────

struct Gioco {
    griglia: Vec<Vec<Cella>>,
    larghezza: usize,
    altezza: usize,
    num_mine: usize,
    cursore_x: usize,
    cursore_y: usize,
    game_over: bool,
    vittoria: bool,
}

impl Gioco {
    fn nuovo(cfg: &Config) -> Self {
        let riga = vec![Cella::default(); cfg.larghezza];
        let griglia = vec![riga; cfg.altezza];
        let mut gioco = Gioco {
            griglia,
            larghezza: cfg.larghezza,
            altezza: cfg.altezza,
            num_mine: cfg.num_mine,
            cursore_x: 0,
            cursore_y: 0,
            game_over: false,
            vittoria: false,
        };
        gioco.genera_mappa();
        gioco
    }

    // ── LCG PRNG ──────────────────────────────────────────────────────────────

    fn genera_mappa(&mut self) {
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let mut mine_piazzate = 0;
        while mine_piazzate < self.num_mine {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let x = (seed >> 33) as usize % self.larghezza;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let y = (seed >> 33) as usize % self.altezza;

            if !self.griglia[y][x].ha_mina {
                self.griglia[y][x].ha_mina = true;
                mine_piazzate += 1;
            }
        }

        for y in 0..self.altezza {
            for x in 0..self.larghezza {
                if self.griglia[y][x].ha_mina {
                    continue;
                }
                let mut contatore: u8 = 0;
                for dy in -1_isize..=1 {
                    for dx in -1_isize..=1 {
                        let ny = y as isize + dy;
                        let nx = x as isize + dx;
                        if nx >= 0
                            && nx < self.larghezza as isize
                            && ny >= 0
                            && ny < self.altezza as isize
                            && self.griglia[ny as usize][nx as usize].ha_mina
                        {
                            contatore += 1;
                        }
                    }
                }
                self.griglia[y][x].mine_vicine = contatore;
            }
        }
    }

    // ── Reveal ────────────────────────────────────────────────────────────────

    fn rivela(&mut self, x: usize, y: usize) {
        if self.griglia[y][x].stato != StatoCella::Coperta {
            return;
        }
        if self.griglia[y][x].ha_mina {
            self.game_over = true;
            // Reveal all mines immediately
            for ry in 0..self.altezza {
                for rx in 0..self.larghezza {
                    if self.griglia[ry][rx].ha_mina {
                        self.griglia[ry][rx].stato = StatoCella::Rivelata;
                    }
                }
            }
            return;
        }
        self.griglia[y][x].stato = StatoCella::Rivelata;
        if self.griglia[y][x].mine_vicine == 0 {
            for dy in -1_isize..=1 {
                for dx in -1_isize..=1 {
                    if dy == 0 && dx == 0 {
                        continue;
                    }
                    let ny = y as isize + dy;
                    let nx = x as isize + dx;
                    if nx >= 0
                        && nx < self.larghezza as isize
                        && ny >= 0
                        && ny < self.altezza as isize
                    {
                        self.rivela(nx as usize, ny as usize);
                    }
                }
            }
        }
    }

    fn alterna_bandierina(&mut self) {
        let cella = &mut self.griglia[self.cursore_y][self.cursore_x];
        match cella.stato {
            StatoCella::Coperta    => cella.stato = StatoCella::Bandierina,
            StatoCella::Bandierina => cella.stato = StatoCella::Coperta,
            StatoCella::Rivelata   => {}
        }
    }

    fn controlla_vittoria(&mut self) {
        for y in 0..self.altezza {
            for x in 0..self.larghezza {
                let c = &self.griglia[y][x];
                if !c.ha_mina && c.stato != StatoCella::Rivelata {
                    return;
                }
            }
        }
        self.vittoria = true;
    }

    // ── Render ────────────────────────────────────────────────────────────────

    fn disegna(&self) -> io::Result<()> {
        let mut out = stdout();
        // Move to top-left and overwrite in place — no full clear avoids flicker.
        // Each line is padded to the terminal width so stale chars are overwritten.
        execute!(out, cursor::MoveTo(0, 0))?;

        // Title bar
        execute!(out, SetForegroundColor(Color::Cyan))?;
        execute!(out, terminal::Clear(ClearType::CurrentLine))?;
        println!("╔══ CAMPO MINATO IN RUST ══╗");
        execute!(out, ResetColor)?;

        // Controls hint
        execute!(out, SetForegroundColor(Color::DarkGrey))?;
        execute!(out, terminal::Clear(ClearType::CurrentLine))?;
        println!("  ↑↓←→ muovi  [INVIO] rivela  [SPAZIO] bandierina  [Q] esci");
        execute!(out, ResetColor)?;

        // Mine counter
        let bandiere: usize = self
            .griglia
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.stato == StatoCella::Bandierina)
            .count();
        execute!(out, SetForegroundColor(Color::Yellow))?;
        println!("  Mine: {}  Bandiere: {}\n", self.num_mine, bandiere);
        execute!(out, ResetColor)?;

        // Grid
        for y in 0..self.altezza {
            execute!(out, terminal::Clear(ClearType::CurrentLine))?;
            print!("  ");
            for x in 0..self.larghezza {
                let cella = &self.griglia[y][x];
                let is_cursore = x == self.cursore_x && y == self.cursore_y;

                if is_cursore {
                    execute!(
                        out,
                        SetForegroundColor(Color::Black),
                        SetBackgroundColor(Color::White)
                    )?;
                }

                match cella.stato {
                    StatoCella::Coperta => {
                        if !is_cursore {
                            execute!(out, SetForegroundColor(Color::DarkGrey))?;
                        }
                        print!(" . ");
                    }
                    StatoCella::Bandierina => {
                        if !is_cursore {
                            execute!(out, SetForegroundColor(Color::Yellow))?;
                        }
                        print!(" P ");
                    }
                    StatoCella::Rivelata => {
                        if cella.ha_mina {
                            execute!(out, SetForegroundColor(Color::Red))?;
                            print!(" * ");
                        } else if cella.mine_vicine == 0 {
                            if !is_cursore {
                                execute!(out, SetForegroundColor(Color::DarkGreen))?;
                            }
                            print!("   ");
                        } else {
                            let colore = match cella.mine_vicine {
                                1 => Color::Blue,
                                2 => Color::Green,
                                3 => Color::Red,
                                4 => Color::DarkBlue,
                                5 => Color::DarkRed,
                                6 => Color::Cyan,
                                7 => Color::Magenta,
                                _ => Color::White,
                            };
                            if !is_cursore {
                                execute!(out, SetForegroundColor(colore))?;
                            }
                            print!(" {} ", cella.mine_vicine);
                        }
                    }
                }

                if is_cursore {
                    execute!(out, ResetColor)?;
                }
            }
            println!();
        }

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
            println!("  BOOM! Hai calpestato una mina.");
            execute!(out, SetForegroundColor(Color::DarkGrey))?;
            println!("  [INVIO] Rivinci  |  [M] Menu  |  [Q] Esci");
            execute!(out, ResetColor)?;
        } else if self.vittoria {
            println!();
            execute!(out, SetForegroundColor(Color::Green))?;
            println!("  ██╗   ██╗██╗████████╗████████╗ ██████╗ ██████╗ ██╗ █████╗ ██╗");
            println!("  ██║   ██║██║╚══██╔══╝╚══██╔══╝██╔═══██╗██╔══██╗██║██╔══██╗██║");
            println!("  ██║   ██║██║   ██║      ██║   ██║   ██║██████╔╝██║███████║██║");
            println!("  ╚██╗ ██╔╝██║   ██║      ██║   ██║   ██║██╔══██╗██║██╔══██║╚═╝");
            println!("   ╚████╔╝ ██║   ██║      ██║   ╚██████╔╝██║  ██║██║██║  ██║██╗");
            println!("    ╚═══╝  ╚═╝   ╚═╝      ╚═╝    ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝╚═╝");
            println!();
            println!("  Complimenti! Hai ripulito il campo minato!");
            execute!(out, SetForegroundColor(Color::DarkGrey))?;
            println!("  [INVIO] Rivinci  |  [M] Menu  |  [Q] Esci");
            execute!(out, ResetColor)?;
        }

        out.flush()?;
        Ok(())
    }
}

// ─── Post-game action ────────────────────────────────────────────────────────

enum AzioneFinale {
    Rivinci,   // same config, new board
    Menu,      // go back to difficulty menu
    Esci,      // quit
}

// ─── Main ─────────────────────────────────────────────────────────────────────

/// Runs one full game session (raw mode already active).
/// Returns what the player wants to do next.
fn gioca(cfg: &Config) -> io::Result<AzioneFinale> {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All))?;

    let mut gioco = Gioco::nuovo(cfg);

    // Drain stale events (Enter from menu or previous session end).
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    gioco.disegna()?;
    loop {
        if gioco.game_over || gioco.vittoria {
            // Board frozen — wait for a decisive keypress.
            loop {
                if event::poll(Duration::from_millis(200))? {
                    if let Event::Key(ke) = event::read()? {
                        if ke.kind != KeyEventKind::Press { continue; }
                        match ke.code {
                            KeyCode::Enter                              => return Ok(AzioneFinale::Rivinci),
                            KeyCode::Char('m') | KeyCode::Char('M')   => return Ok(AzioneFinale::Menu),
                            KeyCode::Char('q') | KeyCode::Char('Q')   => return Ok(AzioneFinale::Esci),
                            _ => {}
                        }
                    }
                }
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(ke) = event::read()? {
                if ke.kind != KeyEventKind::Press { continue; }
                match ke.code {
                    KeyCode::Up    => { if gioco.cursore_y > 0                  { gioco.cursore_y -= 1; } }
                    KeyCode::Down  => { if gioco.cursore_y < gioco.altezza - 1  { gioco.cursore_y += 1; } }
                    KeyCode::Left  => { if gioco.cursore_x > 0                  { gioco.cursore_x -= 1; } }
                    KeyCode::Right => { if gioco.cursore_x < gioco.larghezza - 1{ gioco.cursore_x += 1; } }
                    KeyCode::Enter => {
                        gioco.rivela(gioco.cursore_x, gioco.cursore_y);
                        gioco.controlla_vittoria();
                    }
                    KeyCode::Char(' ') => { gioco.alterna_bandierina(); }
                    KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(AzioneFinale::Esci),
                    _ => {}
                }
                gioco.disegna()?;
            }
        }
    }
}

fn main() -> io::Result<()> {
    'app: loop {
        // ── Menu (cooked mode) ────────────────────────────────────────────────
        let cfg = mostra_menu()?;

        // ── Enter raw / alternate-screen mode ─────────────────────────────────
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

        // ── Session loop: replay without returning to menu ─────────────────────
        let azione = loop {
            match gioca(&cfg)? {
                AzioneFinale::Rivinci => continue,          // same config, new board
                other                 => break other,       // Menu or Esci
            }
        };

        // ── Restore terminal ──────────────────────────────────────────────────
        execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        match azione {
            AzioneFinale::Menu => continue 'app,
            _                  => break 'app,              // Esci
        }
    }
    Ok(())
}