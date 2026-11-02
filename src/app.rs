use std::io;
use std::mem;
use std::path::{Path, PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::store::{self, Lists, Todo};

/// What the keyboard is doing right now.
#[derive(Debug, Default, PartialEq)]
pub enum Mode {
    #[default]
    Normal,
    /// Typing a new todo.
    Add(String),
    /// Changing the text of the selected todo.
    Edit(String),
    /// Waiting for `y` before doing something that can't be undone.
    Confirm(Action),
}

/// Actions that ask "are you sure?" first.
#[derive(Debug, PartialEq)]
pub enum Action {
    DeleteSelected,
    ClearDone,
}

pub struct App {
    pub lists: Lists,
    /// The git project popdo was opened in, if any.
    pub project: Option<PathBuf>,
    /// Show the global list instead of the project list.
    pub show_global: bool,
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
    pub fn new(path: PathBuf, lists: Lists, project: Option<PathBuf>) -> Self {
        Self {
            lists,
            project,
            show_global: false,
            selected: 0,
            mode: Mode::Normal,
            error: None,
            should_quit: false,
            path,
        }
    }

    /// The project of the list on screen, or `None` for the global list.
    pub fn current_project(&self) -> Option<&Path> {
        if self.show_global {
            None
        } else {
            self.project.as_deref()
        }
    }

    /// The list on screen.
    pub fn todos(&self) -> &[Todo] {
        self.lists.get(self.current_project())
    }

    fn todos_mut(&mut self) -> &mut Vec<Todo> {
        // We can't call `self.current_project()` here: it would borrow all
        // of `self` while `self.lists` is borrowed mutably. Borrowing two
        // different fields (`project` and `lists`) at the same time is fine.
        let project = if self.show_global {
            None
        } else {
            self.project.as_deref()
        };
        self.lists.get_mut(project)
    }

    pub fn selected_todo(&self) -> Option<&Todo> {
        self.todos().get(self.selected)
    }

    pub fn done_count(&self) -> usize {
        self.todos().iter().filter(|todo| todo.done).count()
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
            Mode::Confirm(action) => {
                if key.code == KeyCode::Char('y') {
                    match action {
                        Action::DeleteSelected => self.delete_selected()?,
                        Action::ClearDone => self.clear_done()?,
                    }
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
            KeyCode::Char('J') => self.move_selected_down()?,
            KeyCode::Char('K') => self.move_selected_up()?,
            KeyCode::Char('a') => return Ok(Mode::Add(String::new())),
            KeyCode::Char('e') => {
                let text = self.selected_todo().map(|todo| todo.text.clone());
                return Ok(text.map_or(Mode::Normal, Mode::Edit));
            }
            KeyCode::Char(' ') => self.toggle_selected()?,
            KeyCode::Char('d') if self.selected_todo().is_some() => {
                return Ok(Mode::Confirm(Action::DeleteSelected));
            }
            KeyCode::Char('D') if self.done_count() > 0 => {
                return Ok(Mode::Confirm(Action::ClearDone));
            }
            KeyCode::Tab if self.project.is_some() => {
                self.show_global = !self.show_global;
                self.selected = 0;
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
        self.todos_mut().push(Todo::new(text));
        self.selected = self.todos().len() - 1;
        self.save()
    }

    fn rename_selected(&mut self, text: &str) -> io::Result<()> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        let selected = self.selected;
        match self.todos_mut().get_mut(selected) {
            Some(todo) => {
                todo.text = text.to_string();
                self.save()
            }
            None => Ok(()),
        }
    }

    fn toggle_selected(&mut self) -> io::Result<()> {
        let selected = self.selected;
        match self.todos_mut().get_mut(selected) {
            Some(todo) => {
                todo.done = !todo.done;
                self.save()
            }
            None => Ok(()),
        }
    }

    fn delete_selected(&mut self) -> io::Result<()> {
        let selected = self.selected;
        if selected >= self.todos().len() {
            return Ok(());
        }
        self.todos_mut().remove(selected);
        self.keep_selection_in_list();
        self.save()
    }

    fn clear_done(&mut self) -> io::Result<()> {
        self.todos_mut().retain(|todo| !todo.done);
        self.keep_selection_in_list();
        self.save()
    }

    /// Swaps the selected todo with the one below it.
    fn move_selected_down(&mut self) -> io::Result<()> {
        let selected = self.selected;
        if selected + 1 >= self.todos().len() {
            return Ok(());
        }
        self.todos_mut().swap(selected, selected + 1);
        self.selected += 1;
        self.save()
    }

    /// Swaps the selected todo with the one above it.
    fn move_selected_up(&mut self) -> io::Result<()> {
        let selected = self.selected;
        if selected == 0 || selected >= self.todos().len() {
            return Ok(());
        }
        self.todos_mut().swap(selected - 1, selected);
        self.selected -= 1;
        self.save()
    }

    fn select_next(&mut self) {
        if self.selected + 1 < self.todos().len() {
            self.selected += 1;
        }
    }

    fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// After a delete, moves the highlight up if it is past the end.
    fn keep_selection_in_list(&mut self) {
        self.selected = self.selected.min(self.todos().len().saturating_sub(1));
    }

    fn save(&mut self) -> io::Result<()> {
        self.lists.remove_empty();
        store::save(&self.path, &self.lists)
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

    const PROJECT: &str = "/code/popdo";

    /// An app showing a global list with these todos, saving into a
    /// temporary folder. Keep the `TempDir` alive: the folder is deleted
    /// when it is dropped.
    fn app_with(texts: &[&str]) -> (App, TempDir) {
        let lists = Lists {
            global: todos(texts),
            ..Lists::default()
        };
        new_app(lists, None)
    }

    /// An app opened inside the `PROJECT` git project.
    fn app_in_project(project_texts: &[&str], global_texts: &[&str]) -> (App, TempDir) {
        let mut lists = Lists {
            global: todos(global_texts),
            ..Lists::default()
        };
        *lists.get_mut(Some(Path::new(PROJECT))) = todos(project_texts);
        new_app(lists, Some(PathBuf::from(PROJECT)))
    }

    fn new_app(lists: Lists, project: Option<PathBuf>) -> (App, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (App::new(dir.path().join("todos.json"), lists, project), dir)
    }

    fn todos(texts: &[&str]) -> Vec<Todo> {
        texts.iter().map(|text| Todo::new(*text)).collect()
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    fn saved(app: &App) -> Lists {
        store::load(&app.path).unwrap()
    }

    #[test]
    fn a_then_enter_adds_and_saves_a_todo() {
        let (mut app, _dir) = app_with(&["one"]);

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "Acheter du pain");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.todos(), todos(&["one", "Acheter du pain"]));
        assert_eq!(app.selected, 1);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(saved(&app).global, app.todos());
    }

    #[test]
    fn esc_cancels_adding_and_q_is_just_text() {
        let (mut app, _dir) = app_with(&[]);

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "quit");
        press(&mut app, KeyCode::Esc);

        assert!(app.todos().is_empty());
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

        assert_eq!(app.todos()[1].text, "deux");
        assert_eq!(saved(&app).global, app.todos());
    }

    #[test]
    fn space_toggles_done() {
        let (mut app, _dir) = app_with(&["one"]);

        press(&mut app, KeyCode::Char(' '));
        assert!(app.todos()[0].done);
        assert!(saved(&app).global[0].done);

        press(&mut app, KeyCode::Char(' '));
        assert!(!app.todos()[0].done);
    }

    #[test]
    fn d_deletes_only_after_y() {
        let (mut app, _dir) = app_with(&["one", "two"]);

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('n'));
        assert_eq!(app.todos().len(), 2);

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));
        assert_eq!(app.todos(), todos(&["two"]));
        assert_eq!(saved(&app).global, app.todos());
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
    fn keys_do_nothing_on_an_empty_list() {
        let (mut app, _dir) = app_with(&[]);

        for key in ['e', ' ', 'd', 'D', 'J', 'K'] {
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

    #[test]
    fn shift_j_and_k_reorder_todos() {
        let (mut app, _dir) = app_with(&["one", "two", "three"]);

        press(&mut app, KeyCode::Char('J'));
        assert_eq!(app.todos(), todos(&["two", "one", "three"]));
        assert_eq!(app.selected, 1);
        assert_eq!(saved(&app).global, app.todos());

        press(&mut app, KeyCode::Char('K'));
        press(&mut app, KeyCode::Char('K'));
        assert_eq!(app.todos(), todos(&["one", "two", "three"]));
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn shift_d_clears_done_todos_after_y() {
        let (mut app, _dir) = app_with(&["a", "b", "c"]);
        press(&mut app, KeyCode::Char(' '));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char(' '));

        press(&mut app, KeyCode::Char('D'));
        assert_eq!(app.mode, Mode::Confirm(Action::ClearDone));
        press(&mut app, KeyCode::Char('y'));

        assert_eq!(app.todos(), todos(&["b"]));
        assert_eq!(app.selected, 0);
        assert_eq!(saved(&app).global, app.todos());
    }

    #[test]
    fn tab_switches_between_project_and_global() {
        let (mut app, _dir) = app_in_project(&["project todo"], &["global todo"]);
        assert_eq!(app.todos(), todos(&["project todo"]));

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.todos(), todos(&["global todo"]));

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.todos(), todos(&["project todo"]));
    }

    #[test]
    fn tab_does_nothing_outside_a_project() {
        let (mut app, _dir) = app_with(&["global todo"]);

        press(&mut app, KeyCode::Tab);

        assert!(!app.show_global);
        assert_eq!(app.todos(), todos(&["global todo"]));
    }

    #[test]
    fn todos_added_in_a_project_go_to_its_list() {
        let (mut app, _dir) = app_in_project(&[], &["global todo"]);

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "fix CI");
        press(&mut app, KeyCode::Enter);

        let saved = saved(&app);
        assert_eq!(saved.get(Some(Path::new(PROJECT))), todos(&["fix CI"]));
        assert_eq!(saved.global, todos(&["global todo"]));
    }

    #[test]
    fn empty_project_lists_are_not_saved() {
        let (mut app, _dir) = app_in_project(&["only one"], &[]);

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));

        assert!(saved(&app).projects.is_empty());
    }
}
