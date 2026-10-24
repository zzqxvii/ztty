use gpui::{Context, EntityId, Window};

use super::Workspace;
use crate::term::{State, TerminalEvent};

impl Workspace {
    /// What terminal `id` reported.
    pub(super) fn session_event(
        &mut self,
        id: EntityId,
        event: &TerminalEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((project, worktree)) = self.place_of(id) else {
            return;
        };
        let shown = self.shown.project == project
            && self.shown.worktree == worktree
            && self
                .shown_terminal()
                .is_some_and(|session| session.terminal.entity_id() == id);
        if let TerminalEvent::Exited(status) = event {
            let pid = self.projects[project].worktrees[worktree]
                .terminal(id)
                .map_or(0, |session| session.terminal.read(cx).pid());
            log::info!("app: terminal {pid}'s program ended, {status}");
            self.close_terminal(project, worktree, id, window, cx);
            return;
        }
        let Some(session) = self.projects[project].worktrees[worktree].terminal_mut(id) else {
            return;
        };
        match event {
            TerminalEvent::Title(title) => {
                log::debug!("app: a program names its terminal {title:?}");
                session.title.clone_from(title);
            }
            TerminalEvent::Folder(folder) => session.folder.clone_from(folder),
            TerminalEvent::Bell if !shown => {
                if !session.bell {
                    log::debug!("app: a bell in a hidden terminal");
                }
                session.bell = true;
            }
            TerminalEvent::Bell => {}
            // A hidden terminal that starts waiting marks its worktree until it is shown.
            TerminalEvent::Shows(Some(State::Blocked)) if !shown => {
                log::debug!("app: a hidden terminal waits");
                session.waiting = true;
            }
            TerminalEvent::Shows(_) => {}
            TerminalEvent::Exited(_) => unreachable!("handled before the borrow"),
        }
        if shown {
            self.follow(cx);
            self.retitle(window);
        }
        cx.notify();
    }

    /// A press on notification `tag`: its terminal hears of it and shows, unless it asked not to; false when no terminal here posted it.
    pub fn pressed(&mut self, tag: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let found = self
            .projects
            .iter()
            .enumerate()
            .find_map(|(project, view)| {
                view.worktrees
                    .iter()
                    .enumerate()
                    .find_map(|(worktree, tree)| {
                        tree.tabs.iter().enumerate().find_map(|(at, tab)| {
                            let session = tree.terminal(tab.terminal()?)?;
                            let focus = session.terminal.update(cx, |view, _| view.pressed(tag))?;
                            Some((project, worktree, at, focus))
                        })
                    })
            });
        let Some((project, worktree, at, focus)) = found else {
            return false;
        };
        if focus {
            cx.activate(true);
            window.activate_window();
            self.select_worktree(project, worktree, window, cx);
            self.select_tab(at, window, cx);
        }
        true
    }
}
