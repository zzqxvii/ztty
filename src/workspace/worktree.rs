//! The worktrees a project holds: making one from a branch name, and removing one.

use std::time::Duration;

use ely_gpui_component::{
    forms::TextInput,
    overlays::{ConfirmDialog, PromptDialog},
};
use gpui::{App, AppContext, Context, Entity, SharedString, Window};

use super::{Shown, Workspace};
use crate::{i18n::text, projects, worktree::WorktreeView};

/// A worktree a project's button asks for, while its prompt is open.
pub struct Making {
    pub(super) project: usize,
    pub(super) field: Entity<TextInput>,
}

/// Why a worktree was not made or not removed, in git's words, until dismissed.
pub struct Failed {
    /// The alert's title, an i18n key.
    pub(super) title: &'static str,
    pub(super) said: String,
}

impl Workspace {
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
                        this.failed = Some(Failed {
                            title: "workspace.worktree_failed",
                            said: format!("{error:#}"),
                        });
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

    /// Asks before `shown`'s worktree leaves the column and its folder leaves the disk.
    pub(super) fn ask_remove_worktree(&mut self, shown: Shown, cx: &mut Context<Self>) {
        let worktree = self
            .projects
            .get(shown.project)
            .and_then(|project| project.worktrees.get(shown.worktree));
        // The main checkout stays, and so does the folder git does not know.
        if worktree.is_none_or(|worktree| worktree.default) {
            return;
        }
        self.removing_worktree = Some(shown);
        cx.notify();
    }

    /// The question before a worktree leaves the column and its folder leaves the disk.
    pub(super) fn worktree_removal(&self, cx: &Context<Self>) -> Option<ConfirmDialog> {
        let shown = self.removing_worktree?;
        let worktree = self
            .projects
            .get(shown.project)?
            .worktrees
            .get(shown.worktree)?;
        let name = projects::name(&worktree.path);
        let close = cx.entity();
        let confirm = cx.entity();
        let dialog = ConfirmDialog::new(
            "remove-worktree",
            text(cx, "workspace.remove_worktree_title", &[("name", &name)]),
            text(cx, "workspace.remove_worktree_body", &[]).to_string(),
            move |_, cx| {
                close.update(cx, |this, cx| {
                    this.removing_worktree = None;
                    cx.notify();
                });
            },
        );
        Some(
            dialog
                .confirm(text(cx, "workspace.remove_worktree_confirm", &[]))
                .destructive()
                .on_confirm(move |window: &mut Window, cx: &mut App| {
                    confirm.update(cx, |this, cx| this.remove_worktree(shown, window, cx));
                }),
        )
    }

    /// Closes the worktree's terminals, removes its folder, and shows the column as git lists it after.
    fn remove_worktree(&mut self, shown: Shown, window: &mut Window, cx: &mut Context<Self>) {
        self.removing_worktree = None;
        let Some(project) = self.projects.get_mut(shown.project) else {
            return;
        };
        let Some(view) = project.worktrees.get(shown.worktree) else {
            return;
        };
        if view.default {
            log::info!("app: the main checkout at {} stays", view.path.display());
            return;
        }
        let folder = project.path.clone();
        let path = view.path.clone();
        let branch = view.branch.clone().unwrap_or_else(|| projects::name(&path));
        // The terminals close first, or Windows keeps the folder they stand in.
        let gone = project.worktrees.remove(shown.worktree);
        let pids: Vec<u32> = gone
            .terminals
            .iter()
            .map(|session| session.terminal.read(cx).pid())
            .collect();
        drop(gone);
        for pid in pids {
            log::info!("app: terminal {pid} closes with worktree {branch}");
        }
        log::info!("app: worktree {branch} at {} leaves", path.display());
        let was_shown = self.shown == shown;
        if self.shown.project == shown.project {
            let left = self.projects[shown.project].worktrees.len();
            if self.shown.worktree > shown.worktree {
                self.shown.worktree -= 1;
            } else if self.shown.worktree == shown.worktree {
                self.shown.worktree = shown.worktree.min(left.saturating_sub(1));
            }
        }
        if was_shown {
            self.select_worktree(self.shown.project, self.shown.worktree, window, cx);
        }
        cx.spawn(async move |this, cx| {
            let removed = cx
                .background_executor()
                .spawn({
                    let (folder, path) = (folder.clone(), path.clone());
                    async move {
                        // A process that just closed lets go of its folder a moment later.
                        std::thread::sleep(Duration::from_millis(250));
                        projects::remove(&folder, &path)
                    }
                })
                .await;
            match &removed {
                Ok(()) => log::info!("app: worktree {branch} removed from {}", path.display()),
                Err(error) => log::error!(
                    "app: worktree {branch} stays at {}: {error:#}",
                    path.display()
                ),
            }
            let read = cx
                .background_executor()
                .spawn(async move {
                    let mut found = projects::Project::new(folder);
                    found.discover();
                    found
                })
                .await;
            this.update(cx, |this, cx| {
                this.keep_worktrees(shown.project, read, cx);
                if let Err(error) = removed {
                    this.failed = Some(Failed {
                        title: "workspace.worktree_remove_failed",
                        said: format!("{error:#}"),
                    });
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
