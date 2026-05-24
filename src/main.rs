mod game;
mod controls;
mod audio;
mod config;
mod scene;
mod settings;

use crate::{
    scene::{MenuResult, PostGameAction, game_scene, main_menu_scene, options_menu_scene}, settings::Settings
};
use crossterm::{
    cursor,
    event::{self},
    execute,
    terminal::{self},
};
use gilrs::{Gilrs};
use std::io;

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

        let menu_res = main_menu_scene(&mut gilrs)?;

        match menu_res {
            MenuResult::Options => {
                options_menu_scene(&mut settings, &mut gilrs)?;
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
                    match game_scene(&cfg, &settings, &mut gilrs)? {
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
