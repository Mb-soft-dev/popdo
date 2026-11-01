mod store;
mod ui;

use std::io;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use store::Todo;

/// A tiny todo list made for tmux popups.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Without a command, popdo shows the todo list.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Add a todo: `popdo add buy milk`
    Add {
        #[arg(required = true)]
        words: Vec<String>,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("popdo: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> io::Result<()> {
    let path = store::path()?;
    let mut todos = store::load(&path)?;

    match cli.command {
        None => ui::show(&todos),
        Some(Command::Add { words }) => {
            let text = words.join(" ").trim().to_string();
            if text.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a todo can't be empty",
                ));
            }
            todos.push(Todo { text: text.clone() });
            store::save(&path, &todos)?;
            println!("Added: {text}");
            Ok(())
        }
    }
}
