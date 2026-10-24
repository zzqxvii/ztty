use std::{path::PathBuf, time::Instant};

use ely_gpui_component::overlays::ConfirmDialog;
use gpui::{
    App, AppContext, Context, Entity, EntityId, FocusHandle, Focusable, Subscription, Task, Window,
};

use crate::{
    blur::Blur,
    files::{Files, Opened},
    git::panel::{Acted, GitPanel, Opened as DiffOpened},
    i18n::text,
    projects,
    sessions::Session,
    settings::{Settings, SettingsStore},
    term::Launch,
    worktree::WorktreeView,
};

mod events;
mod keys;
mod left;
mod project;
mod session;
mod tabs;
mod view;
mod widths;

pub use keys::{
    CONTEXT, CloseSession, LastSession, NewSession, NextSession, NextTab, PreviousSession,
    PreviousTab, SelectSession, TITLE, TogglePanel, ToggleSessions, bind_keys,
};
use project::{ProjectView, Shown};
use widths::Dragged;

/// The main window's content: projects and their worktrees, the middle's tabs, and Git or Files.
pub struct Workspace {
    focus: FocusHandle,
    pub(super) projects: Vec<ProjectView>,
    pub(super) shown: Shown,
    files: Entity<Files>,
    git: Entity<GitPanel>,
    /// What the window shows of the desktop, and the blur behind it.
    background: Blur,
    /// Column widths a drag shows before the settings hold them.
    dragged: Dragged,
    /// Draws the window at each step of the status marks while one shown moves.
    ticking: Option<Task<()>>,
    /// A worktree a folder picker chose, shown at the next frame.
    pending: Option<Shown>,
    /// The project a press asked to remove.
    pub(super) removing: Option<usize>,
    _settings: Subscription,
    _acted: Subscription,
    _opened: Subscription,
    _diffed: Subscription,
    _activation: Subscription,
}

impl Workspace {
    /// The window's content, on the projects the settings list; `first` runs as the opening terminal, as `shot` asks.
    pub fn new(first: Option<Launch>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = SettingsStore::get(cx);
        let home = dirs::home_dir().unwrap_or_default();
        let mut projects: Vec<ProjectView> = projects::load(&settings.projects)
            .into_iter()
            .map(ProjectView::new)
            .collect();
        if projects.is_empty() {
            projects.push(ProjectView::scratch(home.clone()));
        }
        let folder = projects
            .first()
            .and_then(|project| project.worktrees.first())
            .map_or_else(|| home.clone(), |worktree| worktree.path.clone());
        let git = cx.new(|cx| GitPanel::new(folder.clone(), window, cx));
        let files = cx.new(|_| Files::new(folder));
        // A change the Git panel made shows in Files' marks too, and in every open diff.
        let acted = cx.subscribe(&git, |this, _, _: &Acted, cx| {
            if let Some(folder) = this.follow_folder() {
                this.files.update(cx, |files, cx| files.show(&folder, cx));
            }
            this.reload_diffs(cx);
        });
        // A file the tree opens shows in the middle's tabs.
        let opened = cx.subscribe_in(&files, window, |this, _, Opened(path), window, cx| {
            this.open_file(path.clone(), window, cx)
        });
        // A change the Git panel opens shows there too.
        let diffed = cx.subscribe_in(&git, window, |this, _, opened: &DiffOpened, window, cx| {
            this.open_diff(
                opened.top.clone(),
                opened.change.clone(),
                opened.section,
                window,
                cx,
            )
        });
        // A file may have changed while the window was away.
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.reload_files(cx);
            }
        });
        let mut workspace = Self {
            focus: cx.focus_handle(),
            projects,
            shown: Shown::default(),
            files,
            git,
            background: Blur::new(window, SettingsStore::get(cx)),
            dragged: Dragged::default(),
            ticking: None,
            pending: None,
            removing: None,
            _settings: cx.observe_global_in::<SettingsStore>(window, |this, window, cx| {
                this.background.follow(window, SettingsStore::get(cx));
                cx.notify();
            }),
            _acted: acted,
            _opened: opened,
            _diffed: diffed,
            _activation: activation,
        };
        match first {
            Some(launch) => workspace.open_launch(launch, window, cx),
            None => workspace.select_worktree(0, 0, window, cx),
        }
        workspace
    }

    /// The project and worktree shown.
    pub fn worktree(&self) -> Option<&WorktreeView> {
        self.projects
            .get(self.shown.project)?
            .worktrees
            .get(self.shown.worktree)
    }

    pub fn worktree_mut(&mut self) -> Option<&mut WorktreeView> {
        self.projects
            .get_mut(self.shown.project)?
            .worktrees
            .get_mut(self.shown.worktree)
    }

    /// Every terminal in the window.
    pub fn terminals(&self) -> Vec<&Session> {
        self.projects
            .iter()
            .flat_map(|project| project.worktrees.iter())
            .flat_map(|worktree| worktree.terminals.iter())
            .collect()
    }

    /// The terminal the shown tab shows.
    pub fn shown_terminal(&self) -> Option<&Session> {
        self.worktree()?.shown_terminal()
    }

    /// The terminal `id` names, with the project and worktree that hold it.
    pub fn find_terminal(&self, id: EntityId) -> Option<(usize, usize, &Session)> {
        self.projects
            .iter()
            .enumerate()
            .find_map(|(project, view)| {
                view.worktrees
                    .iter()
                    .enumerate()
                    .find_map(|(worktree, tree)| {
                        tree.terminal(id)
                            .map(|session| (project, worktree, session))
                    })
            })
    }

    /// The project and worktree rows the left column shows.
    pub(super) fn rows(&self) -> Vec<(usize, usize)> {
        self.projects
            .iter()
            .enumerate()
            .filter(|(_, project)| !project.collapsed)
            .flat_map(|(project, view)| (0..view.worktrees.len()).map(move |at| (project, at)))
            .collect()
    }

    /// Shows the worktree `worktree` of `project`, with a terminal if it has none.
    pub fn select_worktree(
        &mut self,
        project: usize,
        worktree: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .projects
            .get(project)
            .and_then(|view| view.worktrees.get(worktree))
            .is_none()
        {
            return;
        }
        if let Some(old) = self.shown_terminal().map(|s| s.terminal.entity_id()) {
            self.let_go(old, cx);
        }
        self.shown = Shown { project, worktree };
        if self.worktree().is_some_and(|view| view.tabs.is_empty()) {
            self.open_at_worktree(window, cx);
            return;
        }
        self.clear_alerts();
        self.focus_shown(window, cx);
        self.follow(cx);
        log::debug!(
            "app: worktree {} shown",
            self.worktree().map_or_else(String::new, WorktreeView::name)
        );
        self.retitle(window);
        cx.notify();
    }

    /// Shows tab `at` of the shown worktree.
    pub fn select_tab(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.worktree_mut().is_some_and(|view| view.select(at)) {
            return;
        }
        self.after_tab_change(window, cx);
    }

    /// After the shown tab changed: the new terminal takes the keys, and the panels follow it.
    fn after_tab_change(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(session) = self
            .worktree_mut()
            .and_then(WorktreeView::shown_terminal_mut)
        {
            session.bell = false;
            session.waiting = false;
        }
        self.focus_shown(window, cx);
        self.follow(cx);
        self.retitle(window);
        cx.notify();
    }

    fn focus_shown(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(session) = self.worktree().and_then(WorktreeView::shown_terminal) {
            session.terminal.update(cx, |view, _| view.set_front(true));
            window.focus(&session.terminal.focus_handle(cx), cx);
            return;
        }
        let editor = self.worktree().and_then(|view| match view.shown()? {
            crate::worktree::Tab::File { view, .. } => view.read(cx).editor().cloned(),
            crate::worktree::Tab::Terminal(_) | crate::worktree::Tab::Diff { .. } => None,
        });
        if let Some(editor) = editor {
            window.focus(&editor.focus_handle(cx), cx);
        }
    }

    /// A terminal hidden from view lets its gesture go, as a window that hides it does.
    fn let_go(&self, id: EntityId, cx: &mut Context<Self>) {
        if let Some((_, _, session)) = self.find_terminal(id) {
            session.terminal.update(cx, |view, cx| {
                view.let_go(cx);
                view.set_front(false);
            });
        }
    }

    /// The folder Files and Git show: the shown worktree's root, whatever its tabs do.
    pub(super) fn follow_folder(&self) -> Option<PathBuf> {
        self.worktree().map(|view| view.path.clone())
    }

    pub(super) fn follow(&mut self, cx: &mut Context<Self>) {
        let Some(folder) = self.follow_folder() else {
            return;
        };
        self.files.update(cx, |files, cx| files.show(&folder, cx));
        self.git.update(cx, |git, cx| git.show(&folder, cx));
    }

    /// Reads every file tab again, as a program may have changed it meanwhile.
    fn reload_files(&mut self, cx: &mut Context<Self>) {
        for project in &mut self.projects {
            for worktree in &mut project.worktrees {
                for tab in &worktree.tabs {
                    if let crate::worktree::Tab::File { view, .. } = tab {
                        view.update(cx, |view, cx| view.reload(cx));
                    }
                }
            }
        }
    }

    /// Reads every diff in the window again, as git changed the files under it.
    fn reload_diffs(&mut self, cx: &mut Context<Self>) {
        for project in &mut self.projects {
            for worktree in &mut project.worktrees {
                for tab in &worktree.tabs {
                    if let crate::worktree::Tab::Diff { view, .. } = tab {
                        view.update(cx, |view, cx| view.reload(cx));
                    }
                }
            }
        }
    }

    /// Clears the bell and the waiting mark of each of the shown worktree's terminals.
    fn clear_alerts(&mut self) {
        if let Some(view) = self.worktree_mut() {
            for session in &mut view.terminals {
                session.bell = false;
                session.waiting = false;
            }
        }
    }

    /// Notes each terminal's mark `now`, and draws the window at each step while one shown moves.
    fn tick(&mut self, now: Instant, cx: &mut Context<Self>) {
        let mut moving = false;
        for project in &mut self.projects {
            for worktree in &mut project.worktrees {
                for session in &mut worktree.terminals {
                    moving |= session.note(now, cx);
                }
            }
        }
        let moving = moving && SettingsStore::get(cx).show_sessions;
        if moving == self.ticking.is_some() {
            return;
        }
        log::debug!("app: status marks {}", if moving { "move" } else { "rest" });
        self.ticking = moving.then(|| {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(crate::sessions::orb::wait(Instant::now()))
                        .await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
        });
    }

    /// The project and worktree that hold terminal `id`.
    pub(super) fn place_of(&self, id: EntityId) -> Option<(usize, usize)> {
        self.projects
            .iter()
            .enumerate()
            .find_map(|(project, view)| {
                view.worktrees
                    .iter()
                    .enumerate()
                    .find(|(_, tree)| tree.terminal(id).is_some())
                    .map(|(worktree, _)| (project, worktree))
            })
    }
    /// What the window is called: the shown tab's title, else the worktree's name.
    fn title(&self) -> String {
        self.worktree()
            .map(|view| match view.shown() {
                Some(tab) => view.title_of(tab),
                None => view.name(),
            })
            .unwrap_or_else(|| TITLE.to_string())
    }

    fn retitle(&self, window: &mut Window) {
        let title = self.title();
        if window.window_title() != title {
            log::debug!("app: the title is {title:?}");
            window.set_window_title(&title);
        }
    }

    /// Shows or hides a side column, as the settings keep it.
    fn toggle(&mut self, flip: fn(&mut Settings), cx: &mut Context<Self>) {
        match SettingsStore::update(cx, flip) {
            Ok(()) => {
                let settings = SettingsStore::get(cx);
                let shown = |on: bool| if on { "shown" } else { "hidden" };
                log::info!(
                    "app: the projects {}, right column {}",
                    shown(settings.show_sessions),
                    shown(settings.show_panel)
                );
            }
            Err(error) => log::error!("app: the columns stay: {error:#}"),
        }
    }

    /// Shows the next worktree, or the one before, round the ends.
    fn step_worktree(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
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
    fn removal(&self, cx: &Context<Self>) -> Option<ConfirmDialog> {
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
