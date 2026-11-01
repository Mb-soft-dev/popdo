use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::{Block, List, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::store::Todo;

/// Shows the todos full screen (or in the tmux popup) until the user closes it.
pub fn show(todos: &[Todo]) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, todos);
    // Always give the terminal back, even if `run` failed.
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, todos: &[Todo]) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, todos))?;
        if let Event::Key(key) = event::read()? {
            if is_close_key(key) {
                return Ok(());
            }
        }
    }
}

/// q, Esc, or Ctrl-T again (so the same key opens and closes the popup).
fn is_close_key(key: KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => true,
        KeyCode::Char('t') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

fn draw(frame: &mut Frame, todos: &[Todo]) {
    let block = Block::bordered()
        .title(format!(" Todo ({}) ", todos.len()))
        .title_bottom(" q: close ");

    if todos.is_empty() {
        let hint = Paragraph::new("No todos yet. Add one with: popdo add buy milk").block(block);
        frame.render_widget(hint, frame.area());
        return;
    }

    let items = todos.iter().map(|todo| format!("• {}", todo.text));
    frame.render_widget(List::new(items).block(block), frame.area());
}
