mod app;
mod project;
mod store;
mod ui;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use app::App;
use store::Todo;

/// A tiny todo list made for tmux popups.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Use this folder to find the project, instead of the current one.
    #[arg(long, global = true, value_name = "DIR")]
    dir: Option<PathBuf>,

    /// Without a command, popdo shows the todo list.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Add a todo: `popdo add buy milk`
    Add {
        /// Add to the global list, even inside a project.
        #[arg(short, long)]
        global: bool,
        #[arg(required = true)]
        words: Vec<String>,
    },
    /// Print how many todos are not done yet (for the tmux status bar).
    Count,
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
    let mut lists = store::load(&path)?;
    let dir = match cli.dir {
        Some(dir) => dir,
        None => std::env::current_dir()?,
    };
    let project = project::find_root(&dir);

    match cli.command {
        None => ui::show(App::new(path, lists, project)),
        Some(Command::Count) => {
            println!("{}", lists.open_count(project.as_deref()));
            Ok(())
        }
        Some(Command::Add { global, words }) => {
            let text = words.join(" ").trim().to_string();
            if text.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a todo can't be empty",
                ));
            }
            let project = if global { None } else { project.as_deref() };
            lists.get_mut(project).push(Todo::new(&text));
            store::save(&path, &lists)?;
            println!("Added to {}: {text}", project::list_name(project));
            Ok(())
        }
    }
}
