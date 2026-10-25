//! The projects the left column lists, and how one joins the list or leaves it.

use std::path::{Path, PathBuf};

use ely_gpui_component::overlays::ConfirmDialog;
use gpui::{App, Context, PathPromptOptions, Window};

use super::Workspace;
use crate::{
    i18n::text,
    projects::{self, Worktree},
    settings::SettingsStore,
    worktree::WorktreeView,
};

/// A project the left column lists, with the runtime of each of its worktrees.
pub struct ProjectView {
    pub path: PathBuf,
    pub name: String,
    pub error: Option<String>,
    /// A project the settings hold; a scratch one is not saved.
    pub saved: bool,
    /// Whether git named its worktrees, so a new one can be made of it.
    pub repository: bool,
    pub collapsed: bool,
    pub worktrees: Vec<WorktreeView>,
}

impl ProjectView {
    pub(super) fn new(project: projects::Project) -> Self {
        Self {
            worktrees: project.worktrees.iter().map(WorktreeView::new).collect(),
            path: project.path,
            name: project.name,
            error: project.error,
            saved: true,
            repository: project.repository,
            collapsed: false,
        }
    }

    /// The start folder as the one project while the settings list none.
    pub(super) fn scratch(folder: PathBuf, name: String) -> Self {
        let worktree = Worktree {
            path: folder.clone(),
            branch: None,
            default: true,
        };
        Self {
            path: folder,
            name,
            error: None,
            saved: false,
            repository: false,
            collapsed: false,
            worktrees: vec![WorktreeView::new(&worktree)],
        }
    }

    /// The main checkout, which a new worktree stands beside.
    pub(super) fn main_checkout(&self) -> &Path {
        self.worktrees
            .iter()
            .find(|worktree| worktree.default)
            .map_or(self.path.as_path(), |worktree| worktree.path.as_path())
    }
}

/// The worktree the middle column shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    pub project: usize,
    pub worktree: usize,
}

impl Workspace {
    /// Asks the platform for a folder, and adds it as a project.
    pub(super) fn add_project(&mut self, cx: &mut Context<Self>) {
        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(crate::i18n::text(cx, "workspace.add_project", &[])),
        };
        let receiver = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = receiver.await else {
                return;
            };
            let Some(path) = paths.pop() else {
                return;
            };
            let found = cx
                .background_executor()
                .spawn(async move {
                    let mut project = projects::Project::new(path);
                    project.discover();
                    project
                })
                .await;
            if this
                .update(cx, |this, cx| this.keep_project(found, cx))
                .is_err()
            {
                log::debug!("app: the window closed before the project was added");
            }
        })
        .detach();
    }

    /// Lists `found` as a project, and shows its first worktree.
    fn keep_project(&mut self, found: projects::Project, cx: &mut Context<Self>) {
        let path = match projects::checked(&found.path) {
            Ok(path) => path,
            Err(error) => {
                log::error!("app: the project stays off the list: {error:#}");
                return;
            }
        };
        if self
            .projects
            .iter()
            .any(|held| held.saved && projects::same(&held.path, &path))
        {
            log::info!("app: {path:?} is already listed");
            return;
        }
        let kept = path.clone();
        let saved = SettingsStore::update(cx, |settings| {
            if !settings
                .projects
                .iter()
                .any(|held| projects::same(held, &kept))
            {
                settings.projects.push(kept.clone());
            }
        });
        if let Err(error) = saved {
            log::error!("app: the project is not kept: {error:#}");
            return;
        }
        let first = !self.projects.iter().any(|project| project.saved);
        if first {
            self.projects.clear();
        }
        log::info!("app: project {} added", path.display());
        self.projects.push(ProjectView::new(found));
        if first {
            let at = self.projects.len() - 1;
            if !self.projects[at].worktrees.is_empty() {
                self.pending = Some(Shown {
                    project: at,
                    worktree: 0,
                });
            }
        }
        cx.notify();
    }

    /// Removes the project `index` from the left column and the settings; its terminals close.
    pub(super) fn remove_project(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.removing = None;
        let Some(project) = self.projects.get(index) else {
            return;
        };
        let name = project.name.clone();
        let saved = project.saved;
        let path = project.path.clone();
        let removed = self.projects.remove(index);
        let pids: Vec<u32> = removed
            .worktrees
            .iter()
            .flat_map(|worktree| worktree.terminals.iter())
            .map(|session| session.terminal.read(cx).pid())
            .collect();
        for pid in pids {
            log::info!("app: terminal {pid} closes with its project");
        }
        log::info!("app: project {name} removed");
        if saved
            && let Err(error) = SettingsStore::update(cx, |settings| {
                settings
                    .projects
                    .retain(|held| !projects::same(held, &path));
            })
        {
            log::error!("app: the removal is not kept: {error:#}");
        }
        if self.projects.is_empty() {
            let home = dirs::home_dir().unwrap_or_default();
            let name = crate::i18n::text(cx, "workspace.current_folder", &[]).to_string();
            self.projects.push(ProjectView::scratch(home, name));
        }
        let project = self.shown.project.min(self.projects.len() - 1);
        let worktree = self
            .shown
            .worktree
            .min(self.projects[project].worktrees.len().saturating_sub(1));
        self.shown = Shown { project, worktree };
        if self.terminals().is_empty() {
            log::info!("app: no terminal left; the window closes");
            window.remove_window();
            return;
        }
        if self.worktree().is_some_and(|view| view.tabs.is_empty()) {
            self.open_at_worktree(window, cx);
            return;
        }
        self.select_worktree(project, worktree, window, cx);
    }

    /// Shows the next worktree, or the one before, round the ends.
    pub(super) fn step_worktree(
        &mut self,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let now = rows
            .iter()
            .position(|&row| row == (self.shown.project, self.shown.worktree));
        let by = if forward { 1 } else { rows.len() - 1 };
        let at = now.map_or(0, |at| (at + by) % rows.len());
        self.select_worktree(rows[at].0, rows[at].1, window, cx);
    }

    /// The question before a project leaves the left column.
    pub(super) fn removal(&self, cx: &Context<Self>) -> Option<ConfirmDialog> {
        let index = self.removing?;
        let name = self.projects.get(index)?.name.clone();
        let close = cx.entity();
        let confirm = cx.entity();
        let dialog = ConfirmDialog::new(
            "remove-project",
            text(cx, "workspace.remove_title", &[("name", &name)]),
            text(cx, "workspace.remove_body", &[]).to_string(),
            move |_, cx| {
                close.update(cx, |this, cx| {
                    this.removing = None;
                    cx.notify();
                });
            },
        );
        Some(
            dialog
                .confirm(text(cx, "workspace.remove_confirm", &[]))
                .destructive()
                .on_confirm(move |window: &mut Window, cx: &mut App| {
                    confirm.update(cx, |this, cx| this.remove_project(index, window, cx));
                }),
        )
    }
}
