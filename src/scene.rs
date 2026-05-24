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
        "Options",
        "Quit",
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
                println!("  ║  ► {:<40}║", opt);
                execute!(stdout, ResetColor)?;
            } else {
                println!("  ║    {:<40}║", opt);
            }
        }
        println!("  ╚════════════════════════════════════════════╝");
        stdout.flush()?;

        // Gamepad input
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
                    // A (Xbox) / X (PlayStation)
                    return Ok(match_menu_selection(selected));
                }
                _ => {}
            }
        }

        // Keyboard / Mouse input
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
                    // Empirical row calculation relative to the menu layout
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

pub fn controls_menu_scene(settings: &mut Settings, gilrs: &mut Gilrs) -> io::Result<()> {
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
        println!("  ║              KEY BINDINGS                  ║");
        println!("  ╠════════════════════════════════════════════╣");
        println!("  ║  Select an action and press ENTER to       ║");
        println!("  ║  rebind it. Press ESC to go back.          ║");
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
                println!("  ║  ► {:<40}║", line);
                execute!(stdout, ResetColor)?;
            } else {
                println!("  ║    {:<40}║", line);
            }
        }
        println!("  ╚════════════════════════════════════════════╝");
        stdout.flush()?;

        // Gamepad input
        while let Some(GEvent { event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(Button::DPadUp, _) => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                gilrs::EventType::ButtonPressed(Button::DPadDown, _) => {
                    if selected < actions.len() - 1 {
                        selected += 1;
                    }
                }
                // B (Xbox) / Circle (PS) or Start — go back
                gilrs::EventType::ButtonPressed(Button::East, _)
                | gilrs::EventType::ButtonPressed(Button::Start, _) => {
                    return Ok(());
                }
                gilrs::EventType::ButtonPressed(Button::South, _) => {
                    // A (Xbox) / X (PlayStation) — rebind selected action
                    execute!(stdout, terminal::Clear(ClearType::All))?;
                    println!(
                        "\n  Press a new key for action: [{}]...",
                        actions[selected].name()
                    );
                    stdout.flush()?;
                    loop {
                        if event::poll(Duration::from_millis(100))? {
                            if let Event::Key(new_ke) = event::read()? {
                                if new_ke.kind == KeyEventKind::Press {
                                    // Remove existing bindings for this action
                                    settings
                                        .key_bindings
                                        .retain(|_, &mut a| a != actions[selected]);
                                    settings.key_bindings.insert(new_ke.code, actions[selected]);
                                    break;
                                }
                            }
                        }
                    }
                    execute!(stdout, terminal::Clear(ClearType::All))?;
                }
                _ => {}
            }
        }

        // Keyboard / Mouse input
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
                        // Flush screen and prompt for new key
                        execute!(stdout, terminal::Clear(ClearType::All))?;
                        println!(
                            "\n  Press a new key for action: [{}]...",
                            actions[selected].name()
                        );
                        stdout.flush()?;
                        loop {
                            if let Event::Key(new_ke) = event::read()? {
                                if new_ke.kind == KeyEventKind::Press {
                                    // Remove existing bindings for this action
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
                Event::Mouse(me) if me.kind == MouseEventKind::Down(MouseButton::Left) => {
                    // Action rows start at row 7 (1 blank + 5 header rows + 1 separator)
                    let row = me.row as isize;
                    let start_row = 7;
                    if row >= start_row && row < start_row + actions.len() as isize {
                        let clicked = (row - start_row) as usize;
                        if clicked == selected {
                            // Second click on already-selected item: trigger rebind
                            execute!(stdout, terminal::Clear(ClearType::All))?;
                            println!(
                                "\n  Press a new key for action: [{}]...",
                                actions[selected].name()
                            );
                            stdout.flush()?;
                            loop {
                                if let Event::Key(new_ke) = event::read()? {
                                    if new_ke.kind == KeyEventKind::Press {
                                        settings
                                            .key_bindings
                                            .retain(|_, &mut a| a != actions[selected]);
                                        settings
                                            .key_bindings
                                            .insert(new_ke.code, actions[selected]);
                                        break;
                                    }
                                }
                            }
                            execute!(stdout, terminal::Clear(ClearType::All))?;
                        } else {
                            selected = clicked;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub fn options_menu_scene(settings: &mut Settings, gilrs: &mut Gilrs) -> io::Result<()> {
    let mut stdout = io::stdout();
    let mut selected = 0_u8;
    const TOTAL_OPTIONS: u8 = 3; // 0: Volume, 1: Input Device, 2: Key Bindings

    execute!(stdout, terminal::Clear(ClearType::All))?;

    loop {
        execute!(stdout, cursor::MoveTo(0, 0))?;
        println!();
        println!("  ╔════════════════════════════════════════════╗");
        println!("  ║                  SETTINGS                  ║");
        println!("  ╠════════════════════════════════════════════╣");
        println!("  ║  ▲/▼: Navigate  |  ◄/►: Change Value       ║");
        println!("  ║  ENTER: Confirm/Open  |  ESC: Go Back      ║");
        println!("  ╠════════════════════════════════════════════╣");

        // --- Option 1: VOLUME ---
        let volume = (settings.volume as usize + 1) / 32;
        let filled = "█".repeat(volume);
        let empty = "░".repeat(8 - volume);
        let volume_line = format!("Volume: [{}{}] ({}/8)", filled, empty, volume);

        if selected == 0 {
            execute!(stdout, SetForegroundColor(Color::Cyan))?;
            println!("  ║  ► {:<40}║", volume_line);
            execute!(stdout, ResetColor)?;
        } else {
            println!("  ║    {:<40}║", volume_line);
        }

        // --- Option 2: INPUT DEVICE ---
        let input_mode = if settings.gamepad {
            "Gamepad"
        } else {
            "Keyboard"
        };
        let input_line = format!("Input Device: < {} >", input_mode);

        if selected == 1 {
            execute!(stdout, SetForegroundColor(Color::Cyan))?;
            println!("  ║  ► {:<40}║", input_line);
            execute!(stdout, ResetColor)?;
        } else {
            println!("  ║    {:<40}║", input_line);
        }

        let link_line = "Keyboard Key Bindings...";

        if selected == 2 {
            execute!(stdout, SetForegroundColor(Color::Cyan))?;
            println!("  ║  ► {:<40}║", link_line);
            execute!(stdout, ResetColor)?;
        } else {
            println!("  ║    {:<40}║", link_line);
        }

        println!("  ╚════════════════════════════════════════════╝");
        stdout.flush()?;

        while let Some(GEvent { event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(Button::DPadUp, _) => {
                    selected = selected.saturating_sub(1);
                }
                gilrs::EventType::ButtonPressed(Button::DPadDown, _) => {
                    if selected < TOTAL_OPTIONS - 1 {
                        selected = selected.saturating_add(1);
                    }
                }
                gilrs::EventType::ButtonPressed(Button::DPadLeft, _) => {
                    if selected == 0 {
                        settings.volume = settings.volume.saturating_sub(32);
                    } else if selected == 1 {
                        settings.gamepad = !settings.gamepad;
                    }
                }
                gilrs::EventType::ButtonPressed(Button::DPadRight, _) => {
                    if selected == 0 {
                        settings.volume = settings.volume.saturating_add(32); //.min(224);
                    } else if selected == 1 {
                        settings.gamepad = !settings.gamepad;
                    }
                }
                // B (Xbox) / Circle (PS) or Start — go back
                gilrs::EventType::ButtonPressed(Button::East, _)
                | gilrs::EventType::ButtonPressed(Button::Start, _) => {
                    return Ok(());
                }
                gilrs::EventType::ButtonPressed(Button::South, _) => {
                    // A (Xbox) / X (PlayStation) — confirm / open sub-menu
                    if selected == 2 {
                        execute!(stdout, terminal::Clear(ClearType::All))?;
                        controls_menu_scene(settings, gilrs)?;
                        execute!(stdout, terminal::Clear(ClearType::All))?;
                    }
                }
                _ => {}
            }
        }

        // Keyboard / Mouse input
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(ke) if ke.kind == KeyEventKind::Press => match ke.code {
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if selected < TOTAL_OPTIONS - 1 {
                            selected = selected.saturating_add(1);
                        }
                    }
                    KeyCode::Left => {
                        if selected == 0 {
                            settings.volume = settings.volume.saturating_sub(32);
                        } else if selected == 1 {
                            settings.gamepad = !settings.gamepad;
                        }
                    }
                    KeyCode::Right => {
                        if selected == 0 {
                            settings.volume = settings.volume.saturating_add(32); //.min(224);
                        } else if selected == 1 {
                            settings.gamepad = !settings.gamepad;
                        }
                    }
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if selected == 2 {
                            execute!(stdout, terminal::Clear(ClearType::All))?;
                            controls_menu_scene(settings, gilrs)?;
                            execute!(stdout, terminal::Clear(ClearType::All))?;
                        }
                    }
                    _ => {}
                },
                Event::Mouse(me) if me.kind == MouseEventKind::Down(MouseButton::Left) => {
                    // Options start at row 7; separator at row 9 splits Volume/Input from Key Bindings
                    let row = me.row as isize;
                    match row {
                        7 => selected = 0,
                        8 => selected = 1,
                        10 => selected = 2,
                        _ => {}
                    }
                }
                Event::Mouse(me) if me.kind == MouseEventKind::ScrollUp => {
                    selected = selected.saturating_sub(1);
                }
                Event::Mouse(me) if me.kind == MouseEventKind::ScrollDown => {
                    if selected < TOTAL_OPTIONS - 1 {
                        selected = selected.saturating_add(1);
                    }
                }
                _ => {}
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

fn end_scene(settings: &Settings, gilrs: &mut Gilrs) -> io::Result<PostGameAction> {
    loop {
        // Gamepad input
        while let Some(GEvent { event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(Button::South, _) => {
                    // A (Xbox) / X (PlayStation) — play again
                    return Ok(PostGameAction::PlayAgain);
                }
                gilrs::EventType::ButtonPressed(Button::Start, _) => {
                    return Ok(PostGameAction::Menu);
                }
                gilrs::EventType::ButtonPressed(Button::Select, _) => {
                    return Ok(PostGameAction::Quit);
                }
                _ => {}
            }
        }

        // Keyboard / Mouse input
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(ke) if ke.kind == KeyEventKind::Press => {
                    if let Some(action) = settings.key_bindings.get(&ke.code) {
                        match action {
                            Action::Reveal => return Ok(PostGameAction::PlayAgain),
                            Action::Menu => return Ok(PostGameAction::Menu),
                            Action::Quit => return Ok(PostGameAction::Quit),
                            _ => {}
                        }
                    }
                }
                Event::Mouse(me) if me.kind == MouseEventKind::Down(MouseButton::Left) => {
                    return Ok(PostGameAction::PlayAgain);
                }
                Event::Mouse(me) if me.kind == MouseEventKind::Down(MouseButton::Right) => {
                    return Ok(PostGameAction::Menu);
                }
                _ => {}
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
                return end_scene(settings, gilrs);
            }

            let mut changed = false;

            // ── Gamepad input ────────────────────────────────────────────────────
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
                return end_scene(settings, gilrs);
            }

            let mut changed = false;
            // ── Keyboard and Mouse input ─────────────────────────────────────────
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
                        let grid_start_col = 3; // Offset from border ("  ║")
                        let grid_start_row = 4; // Interface overhead rows

                        let mouse_col = me.column as isize;
                        let mouse_row = me.row as isize;

                        // Calculate which visible grid cell the mouse is over
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
