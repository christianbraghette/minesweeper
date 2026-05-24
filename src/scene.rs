use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
    style::{Color, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use gilrs::{Button, Event as GEvent, Gilrs};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use crate::{config::Config, controls::Action, game::Game, settings::Settings};

pub enum MenuResult {
    Play(Config),
    Options,
    Quit,
}

pub fn main_menu_scene(gilrs: &mut Gilrs) -> io::Result<MenuResult> {
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

pub fn controls_menu_scene(settings: &mut Settings) -> io::Result<()> {
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

pub fn options_menu_scene(settings: &mut Settings) -> io::Result<()> {
    let mut stdout = io::stdout();
    let mut selected = 0_u8;
    const TOTAL_OPTIONS: u8 = 3; // 0: Volume, 1: Input, 2: Link ai Tasti

    execute!(stdout, terminal::Clear(ClearType::All))?;

    loop {
        execute!(stdout, cursor::MoveTo(0, 0))?;
        println!();
        println!("  ╔════════════════════════════════════════════╗");
        println!("  ║                 IMPOSTAZIONI               ║");
        println!("  ╠════════════════════════════════════════════╣");
        println!("  ║ ▲/▼: Naviga | ◄/►: Modifica Valori         ║");
        println!("  ║ INVIO: Conferma/Apri  | ESC: Torna Indietro║");
        println!("  ╠════════════════════════════════════════════╣");

        // --- 1. OPZIONE: VOLUME ---
        let volume = (settings.volume as usize + 1) / 32;
        let filled = "█".repeat(volume);
        let empty = "░".repeat(8 - volume);
        let volume_line = format!("Volume: [{}{}] ({}/8)", filled, empty, volume);

        if selected == 0 {
            execute!(stdout, SetForegroundColor(Color::Cyan))?;
            println!("  ║ ► {:<40} ║", volume_line);
            execute!(stdout, ResetColor)?;
        } else {
            println!("  ║   {:<40} ║", volume_line);
        }

        // --- 2. OPZIONE: INPUT TYPE ---
        let input_mode = if settings.gamepad {
            "Gamepad"
        } else {
            "Tastiera"
        };
        let input_line = format!("Dispositivo: < {} >", input_mode);

        if selected == 1 {
            execute!(stdout, SetForegroundColor(Color::Cyan))?;
            println!("  ║ ► {:<40} ║", input_line);
            execute!(stdout, ResetColor)?;
        } else {
            println!("  ║   {:<40} ║", input_line);
        }

        println!("  ╠════════════════════════════════════════════╣");

        // --- 3. OPZIONE: LINK AL SOTTO-MENU ---
        let link_line = "Configurazione Tasti Tastiera...".to_string();

        if selected == 2 {
            execute!(stdout, SetForegroundColor(Color::Cyan))?;
            println!("  ║ ► {:<40} ║", link_line);
            execute!(stdout, ResetColor)?;
        } else {
            println!("  ║   {:<40} ║", link_line);
        }

        println!("  ╚════════════════════════════════════════════╝");
        stdout.flush()?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(ke) = event::read()? {
                if ke.kind == KeyEventKind::Press {
                    match ke.code {
                        KeyCode::Up => {
                            if selected > 0 {
                                selected = selected.saturating_sub(1);
                            }
                        }
                        KeyCode::Down => {
                            if selected < TOTAL_OPTIONS - 1 {
                                selected = selected.saturating_add(1);
                            }
                        }
                        KeyCode::Left => {
                            if selected == 0 && settings.volume > 0 {
                                settings.volume -= 1;
                            }
                            if selected == 1 {
                                settings.gamepad = !settings.gamepad;
                            }
                        }
                        KeyCode::Right => {
                            if selected == 0 && settings.volume < 8 {
                                settings.volume += 1;
                            }
                            if selected == 1 {
                                settings.gamepad = !settings.gamepad;
                            }
                        }
                        KeyCode::Esc => break, // Esce dalle impostazioni globali
                        KeyCode::Enter => {
                            if selected == 2 {
                                execute!(stdout, terminal::Clear(ClearType::All))?;
                                controls_menu_scene(settings)?;
                                execute!(stdout, terminal::Clear(ClearType::All))?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

pub enum PostGameAction {
    PlayAgain,
    Menu,
    Quit,
}

fn end_scene(settings: &Settings) -> io::Result<PostGameAction> {
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

pub fn game_scene(
    cfg: &Config,
    settings: &Settings,
    gilrs: &mut Gilrs,
) -> io::Result<PostGameAction> {
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(ClearType::All))?;

    let mut game = Game::new(cfg, settings);

    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    game.update_camera();
    game.draw()?;

    let mut last_gamepad_move = Instant::now();

    if settings.gamepad {
        loop {
            if game.game_over || game.victory {
                return end_scene(settings);
            }

            let mut changed = false;

            // ── Input Gamepad ────────────────────────────────────────────────────
            while let Some(GEvent { event, .. }) = gilrs.next_event() {
                if last_gamepad_move.elapsed() > Duration::from_millis(100) {
                    match event {
                        gilrs::EventType::ButtonPressed(Button::DPadUp, _) => {
                            if game.cursor_y > 0 {
                                game.cursor_y -= 1;
                            }
                            last_gamepad_move = Instant::now();
                            changed = true;
                        }
                        gilrs::EventType::ButtonPressed(Button::DPadDown, _) => {
                            if game.cursor_y < game.height - 1 {
                                game.cursor_y += 1;
                            }
                            last_gamepad_move = Instant::now();
                            changed = true;
                        }
                        gilrs::EventType::ButtonPressed(Button::DPadLeft, _) => {
                            if game.cursor_x > 0 {
                                game.cursor_x -= 1;
                            }
                            last_gamepad_move = Instant::now();
                            changed = true;
                        }
                        gilrs::EventType::ButtonPressed(Button::DPadRight, _) => {
                            if game.cursor_x < game.width - 1 {
                                game.cursor_x += 1;
                            }
                            last_gamepad_move = Instant::now();
                            changed = true;
                        }
                        gilrs::EventType::ButtonPressed(Button::South, _) => {
                            game.reveal(game.cursor_x, game.cursor_y);
                            game.check_victory();
                            changed = true;
                        }
                        gilrs::EventType::ButtonPressed(Button::East, _) => {
                            game.toggle_flag();
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }
            if changed {
                game.update_camera();
                game.draw()?;
            }
        }
    } else {
        loop {
            if game.game_over || game.victory {
                return end_scene(settings);
            }

            let mut changed = false;
            // ── Input Tastiera e Mouse ───────────────────────────────────────────
            while event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Resize(_, _) => {
                        execute!(stdout, terminal::Clear(ClearType::All))?;
                        changed = true;
                    }
                    Event::Key(ke) if ke.kind == KeyEventKind::Press => {
                        if let Some(action) = settings.key_bindings.get(&ke.code) {
                            match action {
                                Action::MoveUp => {
                                    if game.cursor_y > 0 {
                                        game.cursor_y -= 1;
                                    }
                                    changed = true;
                                }
                                Action::MoveDown => {
                                    if game.cursor_y < game.height - 1 {
                                        game.cursor_y += 1;
                                    }
                                    changed = true;
                                }
                                Action::MoveLeft => {
                                    if game.cursor_x > 0 {
                                        game.cursor_x -= 1;
                                    }
                                    changed = true;
                                }
                                Action::MoveRight => {
                                    if game.cursor_x < game.width - 1 {
                                        game.cursor_x += 1;
                                    }
                                    changed = true;
                                }
                                Action::Reveal => {
                                    game.reveal(game.cursor_x, game.cursor_y);
                                    game.check_victory();
                                    changed = true;
                                }
                                Action::Flag => {
                                    game.toggle_flag();
                                    changed = true;
                                }
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
                            changed = true;

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
            }
            if changed {
                game.update_camera();
                game.draw()?;
            }
        }
    }
}
