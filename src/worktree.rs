//! One worktree's runtime: the tabs that show its terminals and files.

use std::path::{Path, PathBuf};

use gpui::{Entity, EntityId};

use crate::{
    diff::DiffView,
    file::FileView,
    projects::Worktree,
    sessions::{self, Session, orb::Look},
};

/// What the middle column shows for a worktree.
#[derive(Clone)]
pub enum Tab {
    Terminal(EntityId),
    File {
        path: PathBuf,
        view: Entity<FileView>,
    },
    Diff {
        path: PathBuf,
        view: Entity<DiffView>,
    },
}

impl Tab {
    /// Its key, as the tab strip names it.
    pub fn key(&self) -> String {
        match self {
            Tab::Terminal(id) => id.to_string(),
            Tab::File { path, .. } => format!("file:{}", path.display()),
            Tab::Diff { path, .. } => format!("diff:{}", path.display()),
        }
    }

    /// The terminal it shows, while it shows one.
    pub fn terminal(&self) -> Option<EntityId> {
        match self {
            Tab::Terminal(id) => Some(*id),
            Tab::File { .. } | Tab::Diff { .. } => None,
        }
    }

    /// The path it shows, while it shows a file or a diff.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Tab::Terminal(_) => None,
            Tab::File { path, .. } | Tab::Diff { path, .. } => Some(path),
        }
    }
}

/// A worktree open in the window: its terminals, its tabs, and the tab shown.
pub struct WorktreeView {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub default: bool,
    pub terminals: Vec<Session>,
    pub tabs: Vec<Tab>,
    pub active: usize,
}

impl WorktreeView {
    pub fn new(worktree: &Worktree) -> Self {
        Self {
            path: worktree.path.clone(),
            branch: worktree.branch.clone(),
            default: worktree.default,
            terminals: Vec::new(),
            tabs: Vec::new(),
            active: 0,
        }
    }

    /// What the left column and the middle's header call it.
    pub fn name(&self) -> String {
        self.branch
            .clone()
            .unwrap_or_else(|| crate::projects::name(&self.path))
    }

    pub fn shown(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    pub fn terminal(&self, id: EntityId) -> Option<&Session> {
        self.terminals
            .iter()
            .find(|session| session.terminal.entity_id() == id)
    }

    pub fn terminal_mut(&mut self, id: EntityId) -> Option<&mut Session> {
        self.terminals
            .iter_mut()
            .find(|session| session.terminal.entity_id() == id)
    }

    pub fn shown_terminal(&self) -> Option<&Session> {
        self.terminal(self.shown()?.terminal()?)
    }

    pub fn shown_terminal_mut(&mut self) -> Option<&mut Session> {
        let id = self.shown()?.terminal()?;
        self.terminal_mut(id)
    }

    /// What a tab is called: its program's title, else its file's name.
    pub fn title_of(&self, tab: &Tab) -> String {
        match tab {
            Tab::Terminal(id) => self
                .terminal(*id)
                .map_or_else(String::new, |session| session.named().to_string()),
            Tab::File { path, .. } | Tab::Diff { path, .. } => crate::projects::name(path),
        }
    }

    /// The file tab that shows `path`, while one does.
    pub fn file(&self, path: &Path) -> Option<&Tab> {
        self.tabs
            .iter()
            .find(|tab| matches!(tab, Tab::File { path: shown, .. } if shown == path))
    }

    /// The diff tab that shows `path`, while one does.
    pub fn diff(&self, path: &Path) -> Option<&Tab> {
        self.tabs
            .iter()
            .find(|tab| matches!(tab, Tab::Diff { path: shown, .. } if shown == path))
    }

    /// Adds a terminal as the shown tab, and returns its id.
    pub fn add(&mut self, session: Session) -> EntityId {
        let id = session.terminal.entity_id();
        self.terminals.push(session);
        self.tabs.push(Tab::Terminal(id));
        self.active = self.tabs.len() - 1;
        id
    }

    /// Adds a file as the shown tab, or shows the tab that already holds it.
    pub fn open_file(&mut self, path: PathBuf, view: Entity<FileView>) {
        let at = self
            .tabs
            .iter()
            .position(|tab| matches!(tab, Tab::File { path: shown, .. } if *shown == path))
            .unwrap_or_else(|| {
                self.tabs.push(Tab::File { path, view });
                self.tabs.len() - 1
            });
        self.active = at;
    }

    /// Adds a bundle of changes as the shown tab, or shows the tab that already holds it.
    pub fn open_diff(&mut self, path: PathBuf, view: Entity<DiffView>) {
        let at = self
            .tabs
            .iter()
            .position(|tab| matches!(tab, Tab::Diff { path: shown, .. } if *shown == path))
            .unwrap_or_else(|| {
                self.tabs.push(Tab::Diff { path, view });
                self.tabs.len() - 1
            });
        self.active = at;
    }

    /// Removes tab `at`; a terminal goes with it, its program too.
    pub fn close_at(&mut self, at: usize) -> Option<Tab> {
        if at >= self.tabs.len() {
            return None;
        }
        let tab = self.tabs.remove(at);
        self.active = if self.tabs.is_empty() {
            0
        } else {
            sessions::after_close(self.active, at, self.tabs.len())
        };
        if let Some(id) = tab.terminal() {
            self.terminals
                .retain(|session| session.terminal.entity_id() != id);
        }
        Some(tab)
    }

    /// Shows tab `at`; false while there is no such tab.
    pub fn select(&mut self, at: usize) -> bool {
        if at >= self.tabs.len() {
            return false;
        }
        self.active = at;
        true
    }

    /// Moves tab `from` to place `to`, the shown one following.
    pub fn move_tab(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let moved = self.tabs.remove(from);
        self.tabs.insert(to, moved);
        self.active = after_move(self.active, from, to);
    }

    /// The terminal whose mark is the most pressing, the latest reported of equals.
    pub fn pressing(&self) -> Option<&Session> {
        self.terminals
            .iter()
            .filter(|session| session.look.is_some())
            .min_by(|one, other| {
                let (one, one_since) = one.look.expect("a look");
                let (other, other_since) = other.look.expect("a look");
                rank(one)
                    .cmp(&rank(other))
                    .then(other_since.cmp(&one_since))
            })
    }

    pub fn bell(&self) -> bool {
        self.terminals.iter().any(|session| session.bell)
    }

    pub fn waiting(&self) -> bool {
        self.terminals.iter().any(|session| session.waiting)
    }
}

/// How pressing a look is: waiting first, idle last.
fn rank(look: Look) -> u8 {
    match look {
        Look::Waiting => 0,
        Look::Error => 1,
        Look::Working(_) => 2,
        Look::Done => 3,
        Look::Idle => 4,
    }
}

/// The tab shown once tab `from` moves to place `to`.
pub fn after_move(active: usize, from: usize, to: usize) -> usize {
    if active == from {
        to
    } else if from < active && active <= to {
        active - 1
    } else if to <= active && active < from {
        active + 1
    } else {
        active
    }
}

#[cfg(test)]
mod tests {
    use super::after_move;

    #[test]
    fn the_shown_tab_follows_its_row() {
        assert_eq!(after_move(2, 2, 0), 0, "the shown tab moved");
        assert_eq!(after_move(2, 0, 3), 1, "a tab below it went up");
        assert_eq!(after_move(2, 3, 0), 3, "a tab above it went down");
        assert_eq!(after_move(3, 1, 0), 3, "a move below it leaves it");
    }
}
