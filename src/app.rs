use std::io;
use std::mem;
use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::store::{self, Todo};

/// What the keyboard is doing right now.
#[derive(Debug, Default, PartialEq)]
pub enum Mode {
    #[default]
    Normal,
    /// Typing a new todo.
    Add(String),
    /// Changing the text of the selected todo.
    Edit(String),
    /// Waiting for `y` before deleting the selected todo.
    ConfirmDelete,
}

pub struct App {
    pub todos: Vec<Todo>,
    /// Index of the highlighted todo. Ignored when the list is empty.
    pub selected: usize,
    pub mode: Mode,
    /// Last save error, shown in the footer until the next key.
    pub error: Option<String>,
    pub should_quit: bool,
    path: PathBuf,
}

/// The result of one key press in a text box.
enum Input {
    Typing(String),
    Submit(String),
    Cancel,
}

impl App {
    pub fn new(path: PathBuf, todos: Vec<Todo>) -> Self {
        Self {
            todos,
            selected: 0,
            mode: Mode::Normal,
            error: None,
            should_quit: false,
            path,
        }
    }

    pub fn selected_todo(&self) -> Option<&Todo> {
        self.todos.get(self.selected)
    }

    pub fn done_count(&self) -> usize {
        self.todos.iter().filter(|todo| todo.done).count()
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        // Ctrl-T closes the popup from any mode, the same key that opened it.
        if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        self.error = None;
        // Take the mode out of `self`, so each branch owns its text.
        let mode = mem::take(&mut self.mode);
        self.mode = match self.next_mode(mode, key) {
            Ok(mode) => mode,
            Err(e) => {
                self.error = Some(format!("Could not save: {e}"));
                Mode::Normal
            }
        };
    }

    fn next_mode(&mut self, mode: Mode, key: KeyEvent) -> io::Result<Mode> {
        match mode {
            Mode::Normal => self.on_normal_key(key.code),
            Mode::Add(text) => match read_input(key, text) {
                Input::Typing(text) => Ok(Mode::Add(text)),
                Input::Submit(text) => self.add(&text).map(|()| Mode::Normal),
                Input::Cancel => Ok(Mode::Normal),
            },
            Mode::Edit(text) => match read_input(key, text) {
                Input::Typing(text) => Ok(Mode::Edit(text)),
                Input::Submit(text) => self.rename_selected(&text).map(|()| Mode::Normal),
                Input::Cancel => Ok(Mode::Normal),
            },
            Mode::ConfirmDelete => {
                if key.code == KeyCode::Char('y') {
                    self.delete_selected()?;
                }
                Ok(Mode::Normal)
            }
        }
    }

    fn on_normal_key(&mut self, code: KeyCode) -> io::Result<Mode> {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('a') => return Ok(Mode::Add(String::new())),
            KeyCode::Char('e') => {
                let text = self.selected_todo().map(|todo| todo.text.clone());
                return Ok(text.map_or(Mode::Normal, Mode::Edit));
            }
            KeyCode::Char(' ') => self.toggle_selected()?,
            KeyCode::Char('d') if self.selected_todo().is_some() => {
                return Ok(Mode::ConfirmDelete);
            }
            _ => {}
        }
        Ok(Mode::Normal)
    }

    fn add(&mut self, text: &str) -> io::Result<()> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        self.todos.push(Todo::new(text));
        self.selected = self.todos.len() - 1;
        self.save()
    }

    fn rename_selected(&mut self, text: &str) -> io::Result<()> {
        let text = text.trim();
        match self.todos.get_mut(self.selected) {
            Some(todo) if !text.is_empty() => {
                todo.text = text.to_string();
                self.save()
            }
            _ => Ok(()),
        }
    }

    fn toggle_selected(&mut self) -> io::Result<()> {
        match self.todos.get_mut(self.selected) {
            Some(todo) => {
                todo.done = !todo.done;
                self.save()
            }
            None => Ok(()),
        }
    }

    fn delete_selected(&mut self) -> io::Result<()> {
        if self.selected >= self.todos.len() {
            return Ok(());
        }
        self.todos.remove(self.selected);
        // Keep the highlight on a todo that still exists.
        self.selected = self.selected.min(self.todos.len().saturating_sub(1));
        self.save()
    }

    fn select_next(&mut self) {
        if self.selected + 1 < self.todos.len() {
            self.selected += 1;
        }
    }

    fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn save(&self) -> io::Result<()> {
        store::save(&self.path, &self.todos)
    }
}

fn read_input(key: KeyEvent, mut text: String) -> Input {
    match key.code {
        KeyCode::Enter => return Input::Submit(text),
        KeyCode::Esc => return Input::Cancel,
        // `pop` removes one whole character, so accents are safe.
        KeyCode::Backspace => {
            text.pop();
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => text.push(c),
        _ => {}
    }
    Input::Typing(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// An app with these todos, saving into a temporary folder.
    /// Keep the `TempDir` alive: the folder is deleted when it is dropped.
    fn app_with(texts: &[&str]) -> (App, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let todos = texts.iter().map(|text| Todo::new(*text)).collect();
        (App::new(dir.path().join("todos.json"), todos), dir)
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    fn saved(app: &App) -> Vec<Todo> {
        store::load(&app.path).unwrap()
    }

    #[test]
    fn a_then_enter_adds_and_saves_a_todo() {
        let (mut app, _dir) = app_with(&["one"]);

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "Acheter du pain");
        press(&mut app, KeyCode::Enter);

        assert_eq!(
            app.todos,
            vec![Todo::new("one"), Todo::new("Acheter du pain")]
        );
        assert_eq!(app.selected, 1);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(saved(&app), app.todos);
    }

    #[test]
    fn esc_cancels_adding_and_q_is_just_text() {
        let (mut app, _dir) = app_with(&[]);

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "quit");
        press(&mut app, KeyCode::Esc);

        assert!(app.todos.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn e_edits_the_selected_todo() {
        let (mut app, _dir) = app_with(&["one", "two"]);

        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('e'));
        assert_eq!(app.mode, Mode::Edit("two".to_string()));
        for _ in 0..3 {
            press(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, "deux");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.todos[1].text, "deux");
        assert_eq!(saved(&app), app.todos);
    }

    #[test]
    fn space_toggles_done() {
        let (mut app, _dir) = app_with(&["one"]);

        press(&mut app, KeyCode::Char(' '));
        assert!(app.todos[0].done);
        assert!(saved(&app)[0].done);

        press(&mut app, KeyCode::Char(' '));
        assert!(!app.todos[0].done);
    }

    #[test]
    fn d_deletes_only_after_y() {
        let (mut app, _dir) = app_with(&["one", "two"]);

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('n'));
        assert_eq!(app.todos.len(), 2);

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));
        assert_eq!(app.todos, vec![Todo::new("two")]);
        assert_eq!(saved(&app), app.todos);
    }

    #[test]
    fn deleting_the_last_todo_moves_the_highlight_up() {
        let (mut app, _dir) = app_with(&["one", "two"]);

        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));

        assert_eq!(app.selected, 0);
    }

    #[test]
    fn highlight_stays_inside_the_list() {
        let (mut app, _dir) = app_with(&["one", "two"]);

        for _ in 0..5 {
            press(&mut app, KeyCode::Down);
        }
        assert_eq!(app.selected, 1);

        for _ in 0..5 {
            press(&mut app, KeyCode::Up);
        }
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn edit_toggle_and_delete_do_nothing_on_an_empty_list() {
        let (mut app, _dir) = app_with(&[]);

        for key in ['e', ' ', 'd'] {
            press(&mut app, KeyCode::Char(key));
            assert_eq!(app.mode, Mode::Normal);
        }
        assert!(app.error.is_none());
    }

    #[test]
    fn ctrl_t_closes_even_while_typing() {
        let (mut app, _dir) = app_with(&[]);

        press(&mut app, KeyCode::Char('a'));
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));

        assert!(app.should_quit);
    }
}
