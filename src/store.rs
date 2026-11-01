use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Todo {
    pub text: String,
    /// `default` lets files saved before this field existed still load.
    #[serde(default)]
    pub done: bool,
}

impl Todo {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            done: false,
        }
    }
}

/// Where the todos live. Set `POPDO_FILE` to use another file.
pub fn path() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("POPDO_FILE") {
        return Ok(PathBuf::from(path));
    }
    let data_dir =
        dirs::data_local_dir().ok_or_else(|| io::Error::other("no data directory found"))?;
    Ok(data_dir.join("popdo").join("todos.json"))
}

/// Reads all todos. A missing file just means there are no todos yet.
pub fn load(path: &Path) -> io::Result<Vec<Todo>> {
    let json = match fs::read_to_string(path) {
        Ok(json) => json,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    serde_json::from_str(&json).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not a valid todo file: {e}", path.display()),
        )
    })
}

pub fn save(path: &Path, todos: &[Todo]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_string_pretty(todos)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_means_no_todos() {
        let dir = tempfile::tempdir().unwrap();
        let todos = load(&dir.path().join("todos.json")).unwrap();
        assert!(todos.is_empty());
    }

    #[test]
    fn saved_todos_load_back_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new-dir").join("todos.json");
        let mut done = Todo::new("Buy milk");
        done.done = true;
        let todos = vec![Todo::new("Préparer la réunion de l'équipe"), done];

        save(&path, &todos).unwrap();

        assert_eq!(load(&path).unwrap(), todos);
    }

    #[test]
    fn files_without_done_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(&path, r#"[{ "text": "old todo" }]"#).unwrap();

        assert_eq!(load(&path).unwrap(), vec![Todo::new("old todo")]);
    }

    #[test]
    fn corrupted_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(&path, "not json").unwrap();

        let err = load(&path).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
