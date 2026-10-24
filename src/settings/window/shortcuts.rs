use ely_gpui_component::{
    forms::{Copy, Paste, SelectAll},
    settings::{KeyboardShortcutsList, SettingsSection, Shortcut},
};
use gpui::{Action, App, Context, Div, KeyContext, ParentElement, SharedString, Styled, div};

use super::SettingsWindow;
use crate::{
    i18n::text,
    menus::{Bigger, Minimize, Quit, ResetSize, Smaller},
    settings::window::OpenSettings,
    term::input::{self, Find, NextPrompt, PreviousPrompt},
    workspace::{
        self, CloseSession, LastSession, NewSession, NextSession, PreviousSession, SelectSession,
        TogglePanel, ToggleSessions,
    },
};

impl SettingsWindow {
    /// Every key the main window's terminal answers to.
    pub(super) fn shortcuts(&self, cx: &Context<Self>) -> Div {
        let keys = SettingsSection::new(text(cx, "settings.shortcuts", &[]))
            .row(KeyboardShortcutsList::new(shortcuts(cx), &self.search));
        div().flex().flex_col().gap_6().child(keys)
    }
}

/// The listed commands, in order: group, name and action.
fn listed(cx: &App) -> Vec<(SharedString, SharedString, Box<dyn Action>)> {
    let row = |group: &str, name: &str, action: Box<dyn Action>| {
        (text(cx, group, &[]), text(cx, name, &[]), action)
    };
    let sessions = "shortcuts.group.sessions";
    let mut listed = vec![
        row(sessions, "shortcuts.new_session", NewSession.boxed_clone()),
        row(
            sessions,
            "shortcuts.close_session",
            CloseSession.boxed_clone(),
        ),
        row(
            sessions,
            "shortcuts.next_session",
            NextSession.boxed_clone(),
        ),
        row(
            sessions,
            "shortcuts.previous_session",
            PreviousSession.boxed_clone(),
        ),
    ];
    listed.extend((0..8).map(|ix| {
        let n = (ix + 1).to_string();
        (
            text(cx, sessions, &[]),
            text(cx, "shortcuts.session", &[("n", &n)]),
            SelectSession(ix).boxed_clone(),
        )
    }));
    listed.extend([
        row(
            sessions,
            "shortcuts.last_session",
            LastSession.boxed_clone(),
        ),
        row("shortcuts.group.edit", "shortcuts.copy", Copy.boxed_clone()),
        row(
            "shortcuts.group.edit",
            "shortcuts.paste",
            Paste.boxed_clone(),
        ),
        row(
            "shortcuts.group.edit",
            "shortcuts.select_all",
            SelectAll.boxed_clone(),
        ),
        row("shortcuts.group.edit", "shortcuts.find", Find.boxed_clone()),
        row(
            "shortcuts.group.view",
            "shortcuts.previous_prompt",
            PreviousPrompt.boxed_clone(),
        ),
        row(
            "shortcuts.group.view",
            "shortcuts.next_prompt",
            NextPrompt.boxed_clone(),
        ),
        row(
            "shortcuts.group.view",
            "shortcuts.toggle_sessions",
            ToggleSessions.boxed_clone(),
        ),
        row(
            "shortcuts.group.view",
            "shortcuts.toggle_panel",
            TogglePanel.boxed_clone(),
        ),
        row(
            "shortcuts.group.view",
            "shortcuts.bigger",
            Bigger.boxed_clone(),
        ),
        row(
            "shortcuts.group.view",
            "shortcuts.smaller",
            Smaller.boxed_clone(),
        ),
        row(
            "shortcuts.group.view",
            "shortcuts.reset_size",
            ResetSize.boxed_clone(),
        ),
        row(
            "shortcuts.group.app",
            "shortcuts.settings",
            OpenSettings.boxed_clone(),
        ),
        row(
            "shortcuts.group.app",
            "shortcuts.minimize",
            Minimize.boxed_clone(),
        ),
        row("shortcuts.group.app", "shortcuts.quit", Quit.boxed_clone()),
    ]);
    listed
}

/// Each listed command's keys in the main window's terminal, a line per binding, from the keymap.
fn shortcuts(cx: &App) -> Vec<Shortcut> {
    let stack = [workspace::CONTEXT, input::CONTEXT]
        .map(|name| KeyContext::parse(name).expect("a context name parses"));
    let keymap = cx.key_bindings();
    let keymap = keymap.borrow();
    let mut shortcuts = Vec::new();
    for (group, name, action) in listed(cx) {
        for binding in keymap.bindings_for_action(action.as_ref()) {
            // gpui matches a binding at any depth of the context stack.
            let applies = binding.predicate().is_none_or(|predicate| {
                (1..=stack.len()).any(|depth| predicate.eval(&stack[..depth]))
            });
            if applies {
                let strokes: Vec<String> = binding
                    .keystrokes()
                    .iter()
                    .map(|key| key.unparse())
                    .collect();
                shortcuts.push(Shortcut {
                    group: group.clone(),
                    name: name.clone(),
                    keys: strokes.join(" ").into(),
                });
            }
        }
    }
    shortcuts
}
