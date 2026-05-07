mod app;
mod config;
mod events;
mod git;
mod model;
mod terminal;
mod ui;

#[cfg(test)]
mod tests;

use std::{env, io};

use crate::{
    git::run_cmd,
    model::App,
    terminal::{restore_terminal, run_app, setup_terminal},
};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--version") {
        println!("GitSimple-Terminal Rust 0.3.0");
        return Ok(());
    }
    if args.get(1).is_some_and(|arg| arg == "--check") {
        println!("GitSimple-Terminal Rust 0.3.0");
        let git = run_cmd("git", ["--version"]);
        println!(
            "Git: {}",
            if git.ok {
                git.text.trim()
            } else {
                "no disponible"
            }
        );
        println!("Terminal: fondo predeterminado respetado");
        return if git.ok {
            Ok(())
        } else {
            Err(io::Error::other(git.text))
        };
    }

    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}
