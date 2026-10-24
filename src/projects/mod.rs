//! The folders the left column lists as projects, and the worktrees found under them.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context as _;

mod git;

/// A folder the user added: its path, its name, and the worktrees found under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub path: PathBuf,
    pub name: String,
    pub worktrees: Vec<Worktree>,
    /// Why the folder cannot be read, while it cannot.
    pub error: Option<String>,
}

/// One working tree: a git worktree, or the folder itself when git names none.
#[derive(Clone, Debug, PartialEq)]
pub struct Worktree {
    pub path: PathBuf,
    /// The branch, as `main`; none when HEAD is detached.
    pub branch: Option<String>,
    /// The project's main checkout.
    pub default: bool,
}

impl Project {
    /// The project at `path`, with no worktrees yet; an unreadable folder carries why.
    pub fn new(path: PathBuf) -> Self {
        let name = name(&path);
        let error = fs::read_dir(&path).err().map(|error| error.to_string());
        Self {
            path,
            name,
            worktrees: Vec::new(),
            error,
        }
    }

    /// Finds the worktrees under the project; a folder git does not know stands as one worktree itself.
    pub fn discover(&mut self) {
        if self.error.is_some() {
            return;
        }
        match git::worktrees(&self.path) {
            Ok(found) if !found.is_empty() => {
                log::info!(
                    "projects: {} holds {} worktree(s)",
                    self.path.display(),
                    found.len()
                );
                self.worktrees = found
                    .into_iter()
                    .map(|worktree| Worktree {
                        path: git::native(worktree.path),
                        ..worktree
                    })
                    .collect();
            }
            Ok(_) => self.alone(),
            Err(error) => {
                log::info!(
                    "projects: {} stands as one worktree: {error:#}",
                    self.path.display()
                );
                self.alone();
            }
        }
    }

    /// The folder itself, as the one worktree.
    fn alone(&mut self) {
        self.worktrees = vec![Worktree {
            path: self.path.clone(),
            branch: None,
            default: true,
        }];
    }
}

/// The projects the settings list, in their order, each with its worktrees found; an unreadable folder carries why.
pub fn load(paths: &[PathBuf]) -> Vec<Project> {
    paths
        .iter()
        .map(|path| {
            let mut project = Project::new(path.clone());
            if let Some(error) = &project.error {
                log::warn!(
                    "projects: {} cannot be read: {error}",
                    project.path.display()
                );
            }
            project.discover();
            project
        })
        .collect()
}

/// A folder's own name, or its whole path while it names none.
pub fn name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// `path` made absolute; a relative path counts from the working folder.
pub fn absolute(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let folder = std::env::current_dir().context("projects: the working folder is unknown")?;
    Ok(folder.join(path))
}

/// `path` as a project: a readable folder; the error names the path.
pub fn checked(path: &Path) -> anyhow::Result<PathBuf> {
    let path = absolute(path)?;
    let folder = fs::metadata(&path)
        .with_context(|| format!("projects: {} cannot be read", path.display()))?;
    anyhow::ensure!(
        folder.is_dir(),
        "projects: {} is not a folder",
        path.display()
    );
    fs::read_dir(&path)
        .with_context(|| format!("projects: {} cannot be listed", path.display()))?;
    Ok(path)
}

/// Adds `path` to `projects` unless it is already there; returns the path kept.
pub fn add(projects: &mut Vec<PathBuf>, path: &Path) -> anyhow::Result<PathBuf> {
    let path = checked(path)?;
    anyhow::ensure!(
        !projects.iter().any(|held| same(held, &path)),
        "projects: {} is already listed",
        path.display()
    );
    projects.push(path.clone());
    Ok(path)
}

/// Whether two paths name the same folder, as the platform compares them.
pub fn same(one: &Path, other: &Path) -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        one.to_string_lossy().to_lowercase() == other.to_string_lossy().to_lowercase()
    } else {
        one == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("ztty-projects-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a test folder");
        folder
    }

    #[test]
    fn a_folder_is_named_by_its_last_part() {
        assert_eq!(name(Path::new("/tmp/notes")), "notes");
        assert_eq!(name(Path::new("/")), "/");
        let folder = scratch("named");
        assert_eq!(name(&folder), folder.file_name().unwrap().to_string_lossy());
        fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn only_a_readable_folder_is_a_project() {
        let folder = scratch("ok");
        assert_eq!(checked(&folder).expect("a folder"), folder);
        let file = folder.join("file.txt");
        fs::write(&file, "x").expect("a file");
        let error = checked(&file).expect_err("a file is no project");
        assert!(error.to_string().ends_with("is not a folder"), "{error}");
        let missing = folder.join("gone");
        let error = checked(&missing).expect_err("no folder");
        assert!(error.to_string().ends_with("cannot be read"), "{error}");
        fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn a_project_is_added_once() {
        let folder = scratch("once");
        let mut projects = Vec::new();
        add(&mut projects, &folder).expect("the first time");
        let error = add(&mut projects, &folder).expect_err("the second time");
        assert!(error.to_string().ends_with("is already listed"), "{error}");
        assert_eq!(projects, std::slice::from_ref(&folder));
        fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn a_relative_path_counts_from_the_working_folder() {
        let folder = std::env::current_dir().expect("a working folder");
        assert_eq!(
            absolute(Path::new(".")).expect("made absolute"),
            folder.join(".")
        );
        assert_eq!(absolute(&folder).expect("already absolute"), folder);
    }

    #[test]
    fn two_paths_meet_as_the_platform_compares_them() {
        let one = Path::new("/tmp/Notes");
        let other = Path::new("/tmp/notes");
        assert_eq!(same(one, other), cfg!(any(windows, target_os = "macos")));
        assert!(same(one, Path::new("/tmp/Notes")));
    }

    #[test]
    fn a_plain_folder_stands_as_one_worktree() {
        let folder = scratch("plain");
        let mut project = Project::new(folder.clone());
        project.discover();
        assert_eq!(project.worktrees.len(), 1);
        assert_eq!(project.worktrees[0].path, folder);
        assert!(project.worktrees[0].default);
        assert_eq!(project.worktrees[0].branch, None);
        assert!(project.error.is_none());
        fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn projects_load_in_the_settings_order() {
        let one = scratch("load-one");
        let two = scratch("load-two");
        let found = load(&[one.clone(), two.clone()]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].path, one);
        assert_eq!(found[1].path, two);
        assert!(found.iter().all(|project| project.worktrees.len() == 1));
        fs::remove_dir_all(&one).expect("cleanup");
        fs::remove_dir_all(&two).expect("cleanup");
    }

    #[test]
    fn a_missing_folder_carries_why_and_no_worktree() {
        let folder = scratch("missing").join("gone");
        let found = load(std::slice::from_ref(&folder));
        assert!(found[0].error.is_some(), "{found:?}");
        assert!(found[0].worktrees.is_empty());
    }
}
