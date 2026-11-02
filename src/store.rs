use std::collections::BTreeMap;
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

/// Everything popdo saves: one global list, and one list per git project.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Lists {
    #[serde(default)]
    pub global: Vec<Todo>,
    /// Keyed by the project's root folder. A `BTreeMap` keeps the keys
    /// sorted, so the file doesn't change order on every save.
    #[serde(default)]
    pub projects: BTreeMap<PathBuf, Vec<Todo>>,
}

impl Lists {
    /// The list of `project`, or the global list when `project` is `None`.
    pub fn get(&self, project: Option<&Path>) -> &[Todo] {
        match project {
            Some(project) => self.projects.get(project).map_or(&[], Vec::as_slice),
            None => &self.global,
        }
    }

    /// Like `get`, but creates the project list if it doesn't exist yet.
    pub fn get_mut(&mut self, project: Option<&Path>) -> &mut Vec<Todo> {
        match project {
            Some(project) => self.projects.entry(project.to_path_buf()).or_default(),
            None => &mut self.global,
        }
    }

    /// How many todos of this list are not done yet.
    pub fn open_count(&self, project: Option<&Path>) -> usize {
        self.get(project).iter().filter(|todo| !todo.done).count()
    }

    /// Removes project lists that became empty, to keep the file small.
    pub fn remove_empty(&mut self) {
        self.projects.retain(|_, todos| !todos.is_empty());
    }
}

/// The shapes the file can have on disk. popdo 0.1 saved only a list
/// (`[...]`), so it is still accepted and becomes the global list.
#[derive(Deserialize)]
#[serde(untagged)]
enum OnDisk {
    OldList(Vec<Todo>),
    Lists(Lists),
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

/// Reads all lists. A missing file just means there are no todos yet.
pub fn load(path: &Path) -> io::Result<Lists> {
    let json = match fs::read_to_string(path) {
        Ok(json) => json,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Lists::default()),
        Err(e) => return Err(e),
    };
    let on_disk = serde_json::from_str(&json).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not a valid todo file: {e}", path.display()),
        )
    })?;
    Ok(match on_disk {
        OnDisk::OldList(global) => Lists {
            global,
            ..Lists::default()
        },
        OnDisk::Lists(lists) => lists,
    })
}

pub fn save(path: &Path, lists: &Lists) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_string_pretty(lists)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_means_no_todos() {
        let dir = tempfile::tempdir().unwrap();
        let lists = load(&dir.path().join("todos.json")).unwrap();
        assert_eq!(lists, Lists::default());
    }

    #[test]
    fn saved_lists_load_back_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new-dir").join("todos.json");
        let mut done = Todo::new("Buy milk");
        done.done = true;
        let mut lists = Lists {
            global: vec![Todo::new("Préparer la réunion de l'équipe"), done],
            ..Lists::default()
        };
        lists
            .get_mut(Some(Path::new("/code/popdo")))
            .push(Todo::new("Write tests"));

        save(&path, &lists).unwrap();

        assert_eq!(load(&path).unwrap(), lists);
    }

    #[test]
    fn old_files_become_the_global_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(&path, r#"[{ "text": "old todo" }]"#).unwrap();

        let lists = load(&path).unwrap();

        assert_eq!(lists.global, vec![Todo::new("old todo")]);
        assert!(lists.projects.is_empty());
    }

    #[test]
    fn corrupted_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(&path, "not json").unwrap();

        let err = load(&path).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn open_count_skips_done_todos() {
        let mut lists = Lists {
            global: vec![Todo::new("a"), Todo::new("b")],
            ..Lists::default()
        };
        lists.global[0].done = true;

        assert_eq!(lists.open_count(None), 1);
        assert_eq!(lists.open_count(Some(Path::new("/nowhere"))), 0);
    }
}
