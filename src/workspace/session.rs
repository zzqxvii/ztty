use std::path::PathBuf;

use gpui::{AppContext, Context, EntityId, Window};

use super::Workspace;
use crate::{
    sessions::Session,
    settings::{SettingsStore, StartIn},
    term::{
        Events, Launch, Pty, TerminalView, config,
        shell::{ShellIntegration, launch},
        spawn,
    },
};

impl Workspace {
    /// A terminal tab in the shown worktree: the shell the settings name, in home or the shown terminal's folder.
    pub(super) fn open_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(home) = dirs::home_dir() else {
            log::error!("app: no home folder, so no new terminal");
            return;
        };
        let Some(root) = self.worktree().map(|view| view.path.clone()) else {
            log::error!("app: no worktree is shown, so no new terminal");
            return;
        };
        let folder = match SettingsStore::get(cx).start_in {
            StartIn::Home => home,
            StartIn::Session => self
                .shown_terminal()
                .map_or(root, |session| session.folder.clone()),
        };
        self.open_at(folder, window, cx);
    }

    /// A terminal in the shown worktree's own folder.
    pub(super) fn open_at_worktree(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.worktree().map(|view| view.path.clone()) else {
            return;
        };
        self.open_at(root, window, cx);
    }

    /// Runs `launch` as a tab of the shown worktree, as `shot` asks.
    pub fn open_launch(&mut self, launch: Launch, window: &mut Window, cx: &mut Context<Self>) {
        let settings = SettingsStore::get(cx);
        match spawn(&launch, (80, 24), (8, 16), config(settings)) {
            Ok((pty, events)) => self.add(pty, events, launch, window, cx),
            Err(error) => log::error!("app: the run does not start: {error:#}"),
        }
    }

    fn open_at(&mut self, folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let settings = SettingsStore::get(cx);
        let started = launch(&settings.shell, folder, &cx.global::<ShellIntegration>().0)
            .and_then(|launch| Ok((spawn(&launch, (80, 24), (8, 16), config(settings))?, launch)));
        match started {
            Ok(((pty, events), launch)) => self.add(pty, events, launch, window, cx),
            Err(error) => log::error!("app: no new terminal: {error:#}"),
        }
    }

    fn add(
        &mut self,
        pty: Pty,
        events: Events,
        launch: Launch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let terminal = cx.new(|cx| TerminalView::new(pty, events, window, cx));
        let id = terminal.entity_id();
        let events = cx.subscribe_in(&terminal, window, move |this, _, event, window, cx| {
            this.session_event(id, event, window, cx)
        });
        let shell = launch
            .program
            .as_ref()
            .map_or_else(String::new, |(program, _)| program.clone());
        let pid = terminal.read(cx).pid();
        log::info!("app: terminal {pid} opens in {}", launch.cwd.display());
        let session = Session {
            terminal,
            title: String::new(),
            folder: launch.cwd,
            shell,
            bell: false,
            waiting: false,
            look: None,
            _events: events,
        };
        if let Some(view) = self.worktree_mut() {
            view.add(session);
        }
        self.after_tab_change(window, cx);
    }

    /// Closes terminal `id` of `project`'s worktree `worktree`; the app's last one closes the window.
    pub(super) fn close_terminal(
        &mut self,
        project: usize,
        worktree: usize,
        id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shown_there = self.shown.project == project && self.shown.worktree == worktree;
        let pid = self
            .projects
            .get(project)
            .and_then(|view| view.worktrees.get(worktree))
            .and_then(|tree| tree.terminal(id))
            .map(|session| session.terminal.read(cx).pid());
        let at = self
            .projects
            .get(project)
            .and_then(|view| view.worktrees.get(worktree))
            .and_then(|tree| tree.tabs.iter().position(|tab| tab.terminal() == Some(id)));
        let closed = self
            .projects
            .get_mut(project)
            .and_then(|view| view.worktrees.get_mut(worktree))
            .and_then(|tree| at.and_then(|at| tree.close_at(at)));
        if closed.is_none() {
            return;
        }
        if let Some(pid) = pid {
            log::info!("app: terminal {pid} closes");
        }
        if self.terminals().is_empty() {
            log::info!("app: no terminal left; the window closes");
            window.remove_window();
            return;
        }
        if shown_there {
            if self.worktree().is_some_and(|view| view.tabs.is_empty()) {
                self.retitle(window);
            } else {
                self.after_tab_change(window, cx);
            }
        }
        cx.notify();
    }

    /// Closes the file or diff tab at `at` of `project`'s worktree `worktree`.
    pub(super) fn close_file(
        &mut self,
        project: usize,
        worktree: usize,
        at: usize,
        path: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shown_there = self.shown.project == project && self.shown.worktree == worktree;
        let closed = self
            .projects
            .get_mut(project)
            .and_then(|view| view.worktrees.get_mut(worktree))
            .and_then(|tree| tree.close_at(at));
        if closed.is_none() {
            return;
        }
        log::info!("app: the tab of {} closes", path.display());
        if shown_there {
            if self.worktree().is_some_and(|view| view.tabs.is_empty()) {
                self.retitle(window);
            } else {
                self.after_tab_change(window, cx);
            }
        }
        cx.notify();
    }

    /// Closes the shown tab.
    pub(super) fn close_shown_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.worktree().map(|view| view.active) else {
            return;
        };
        let Some(tab) = self.worktree().and_then(|view| view.tabs.get(at)) else {
            return;
        };
        let (terminal, path) = (tab.terminal(), tab.path().map(std::path::Path::to_path_buf));
        let (project, worktree) = (self.shown.project, self.shown.worktree);
        match (terminal, path) {
            (Some(id), _) => self.close_terminal(project, worktree, id, window, cx),
            (None, Some(path)) => self.close_file(project, worktree, at, &path, window, cx),
            (None, None) => {}
        }
    }
}
