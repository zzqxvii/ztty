//! The projects the left column lists, and how one joins the list or leaves it.

use std::path::{Path, PathBuf};

use ely_gpui_component::{forms::TextInput, overlays::PromptDialog};
use gpui::{AppContext, Context, Entity, PathPromptOptions, SharedString, Window};

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
    fn main_checkout(&self) -> &Path {
        self.worktrees
            .iter()
            .find(|worktree| worktree.default)
            .map_or(self.path.as_path(), |worktree| worktree.path.as_path())
    }
}

/// A worktree a project's button asks for, while its prompt is open.
pub struct Making {
    pub(super) project: usize,
    pub(super) field: Entity<TextInput>,
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

    /// Asks for a branch name, and makes a worktree of `index`'s repository beside its main checkout.
    pub(super) fn ask_worktree(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.projects.get(index) else {
            return;
        };
        if !project.repository {
            return;
        }
        let taken: Vec<&str> = project
            .worktrees
            .iter()
            .filter_map(|worktree| worktree.branch.as_deref())
            .collect();
        let branch = (1..)
            .map(|n| format!("wt-{n}"))
            .find(|name| !taken.contains(&name.as_str()))
            .unwrap_or_else(|| "wt".into());
        let field = cx.new(|cx| {
            let mut field = TextInput::new(window, cx);
            field.set_text(branch, cx);
            field
        });
        log::debug!("app: {} asks for a branch name", project.name);
        self.making = Some(Making {
            project: index,
            field,
        });
        cx.notify();
    }

    /// The prompt that asks for a branch name, while one is open.
    pub(super) fn asking(&self, cx: &Context<Self>) -> Option<PromptDialog> {
        let making = self.making.as_ref()?;
        let index = making.project;
        let name = self.projects.get(index)?.name.clone();
        let close = cx.entity();
        let submit = cx.entity();
        let reason: SharedString = text(cx, "workspace.worktree_name", &[]);
        let dialog = PromptDialog::new(
            "make-worktree",
            text(cx, "workspace.worktree_title", &[("name", &name)]),
            &making.field,
            move |_, cx| {
                close.update(cx, |this, cx| {
                    this.making = None;
                    cx.notify();
                });
            },
        )
        .label(text(cx, "workspace.worktree_label", &[]))
        .submit(text(cx, "workspace.worktree_submit", &[]))
        .check(move |branch| match projects::branch_ok(branch) {
            true => Ok(()),
            false => Err(reason.clone()),
        })
        .on_submit(move |branch, _, cx| {
            submit.update(cx, |this, cx| this.make_worktree(index, branch, cx))
        });
        Some(dialog)
    }

    /// Makes the worktree, off the main thread, and shows it once it stands.
    fn make_worktree(&mut self, index: usize, branch: &str, cx: &mut Context<Self>) {
        self.making = None;
        let Some(project) = self.projects.get(index) else {
            return;
        };
        let folder = project.path.clone();
        let path = projects::place(project.main_checkout(), branch);
        let branch = branch.trim().to_string();
        log::info!("app: worktree {branch} is made at {}", path.display());
        cx.spawn(async move |this, cx| {
            let made = cx
                .background_executor()
                .spawn({
                    let (folder, path, branch) = (folder.clone(), path.clone(), branch.clone());
                    async move { projects::make(&folder, &path, &branch) }
                })
                .await;
            match made {
                Ok(()) => log::info!("app: worktree {branch} made at {}", path.display()),
                Err(error) => {
                    log::error!("app: no worktree {branch} at {}: {error:#}", path.display());
                    this.update(cx, |this, cx| {
                        this.failed = Some(format!("{error:#}"));
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            }
            let read = cx
                .background_executor()
                .spawn(async move {
                    let mut found = projects::Project::new(folder);
                    found.discover();
                    found
                })
                .await;
            this.update(cx, |this, cx| this.keep_worktrees(index, read, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// The project's worktrees as git now lists them, each keeping its terminals, and the new one shown.
    fn keep_worktrees(&mut self, index: usize, found: projects::Project, cx: &mut Context<Self>) {
        let shown_here = self.shown.project == index;
        let shown_at = self.shown.worktree;
        let Some(project) = self.projects.get_mut(index) else {
            return;
        };
        let shown_path = shown_here
            .then(|| {
                project
                    .worktrees
                    .get(shown_at)
                    .map(|view| view.path.clone())
            })
            .flatten();
        let mut spare: Vec<Option<WorktreeView>> = std::mem::take(&mut project.worktrees)
            .into_iter()
            .map(Some)
            .collect();
        let mut kept = Vec::with_capacity(found.worktrees.len());
        let mut made = None;
        for worktree in &found.worktrees {
            let same = |view: &Option<WorktreeView>| {
                view.as_ref().is_some_and(|view| view.path == worktree.path)
            };
            let view = match spare.iter().position(same) {
                Some(at) => {
                    let mut view = spare[at].take().expect("one view to a worktree");
                    view.branch = worktree.branch.clone();
                    view.default = worktree.default;
                    view
                }
                None => {
                    made = Some(kept.len());
                    WorktreeView::new(worktree)
                }
            };
            kept.push(view);
        }
        log::info!("app: {} holds {} worktree(s)", project.name, kept.len());
        project.worktrees = kept;
        let shown_at = shown_path.and_then(|path| {
            project
                .worktrees
                .iter()
                .position(|worktree| worktree.path == path)
        });
        if let Some(at) = shown_at {
            self.shown.worktree = at;
        }
        if let Some(at) = made {
            self.pending = Some(Shown {
                project: index,
                worktree: at,
            });
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
}
