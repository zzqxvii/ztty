//! The main window's actions and the keys they take.

use gpui::{Action, App, KeyBinding, actions};

actions!(
    ztty,
    [
        ToggleSessions,
        TogglePanel,
        NewSession,
        CloseSession,
        NextSession,
        PreviousSession,
        LastSession,
        NextTab,
        PreviousTab
    ]
);

/// Shows the worktree at this place of the left column, from 0; past the last one, the key goes on to the program.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = ztty, no_json)]
pub struct SelectSession(pub usize);

/// The key context of the main window's content.
pub const CONTEXT: &str = "Workspace";

/// The window's title while no terminal names it.
pub const TITLE: &str = "Ztty";

/// Ghostty's keys: Cmd on macOS; off it, Ctrl-Shift for keys a terminal takes, Alt for digits.
pub fn bind_keys(cx: &mut App) {
    let (sessions, panel, new, close, digit) = if cfg!(target_os = "macos") {
        ("cmd-b", "cmd-alt-b", "cmd-t", "cmd-w", "cmd")
    } else {
        (
            "ctrl-shift-b",
            "ctrl-shift-alt-b",
            "ctrl-shift-t",
            "ctrl-shift-w",
            "alt",
        )
    };
    let (next_tab, previous_tab) = if cfg!(target_os = "macos") {
        ("cmd-shift-]", "cmd-shift-[")
    } else {
        ("ctrl-shift-]", "ctrl-shift-[")
    };
    let context = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new(sessions, ToggleSessions, context),
        KeyBinding::new(panel, TogglePanel, context),
        KeyBinding::new(new, NewSession, context),
        KeyBinding::new(close, CloseSession, context),
        KeyBinding::new("ctrl-tab", NextSession, context),
        KeyBinding::new("ctrl-shift-tab", PreviousSession, context),
        KeyBinding::new(&format!("{digit}-9"), LastSession, context),
        KeyBinding::new(next_tab, NextTab, context),
        KeyBinding::new(previous_tab, PreviousTab, context),
    ]);
    cx.bind_keys(
        (1..=8).map(|n| KeyBinding::new(&format!("{digit}-{n}"), SelectSession(n - 1), context)),
    );
}
