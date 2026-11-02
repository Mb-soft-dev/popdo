use std::fs;
use std::path::{Path, PathBuf};

/// Finds the root of the git project that contains `dir`, if any.
///
/// A git worktree belongs to its main repository, so they share one list.
pub fn find_root(dir: &Path) -> Option<PathBuf> {
    // `ancestors` yields `dir`, then its parent, then the parent's parent...
    let root = dir.ancestors().find(|dir| dir.join(".git").exists())?;

    // In a worktree, `.git` is a file: "gitdir: /repo/.git/worktrees/<name>".
    let git_file = fs::read_to_string(root.join(".git")).ok();
    let main_repo = git_file.as_deref().and_then(main_repo_of_worktree);
    Some(main_repo.unwrap_or_else(|| root.to_path_buf()))
}

/// The name shown for a list: the project's folder name, or "global".
pub fn list_name(project: Option<&Path>) -> String {
    match project.and_then(Path::file_name) {
        Some(name) => name.to_string_lossy().into_owned(),
        None => "global".to_string(),
    }
}

fn main_repo_of_worktree(git_file: &str) -> Option<PathBuf> {
    let git_dir = git_file.trim().strip_prefix("gitdir: ")?;
    let (repo, _worktree) = git_dir.split_once("/.git/worktrees/")?;
    Some(PathBuf::from(repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_repo_from_a_sub_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let sub = dir.path().join("src").join("deep");
        fs::create_dir_all(&sub).unwrap();

        assert_eq!(find_root(&sub), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn no_git_means_no_project() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(find_root(dir.path()), None);
    }

    #[test]
    fn a_worktree_belongs_to_its_main_repo() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".git"),
            "gitdir: /code/may/.git/worktrees/may-2938\n",
        )
        .unwrap();

        assert_eq!(find_root(dir.path()), Some(PathBuf::from("/code/may")));
    }

    #[test]
    fn a_submodule_is_its_own_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".git"), "gitdir: ../.git/modules/lib\n").unwrap();

        assert_eq!(find_root(dir.path()), Some(dir.path().to_path_buf()));
    }
}
