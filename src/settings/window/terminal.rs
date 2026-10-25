use std::path::PathBuf;

use ely_gpui_component::{
    buttons::SegmentedControl,
    feedback::InlineMessage,
    forms::{NumberInput, PathInput},
    primitives::Severity,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, ControlSize},
};
use gpui::{App, Context, Div, Entity, ParentElement, SharedString, Styled, Window, div};

use super::{SettingsWindow, set};
use crate::{
    i18n::text,
    settings::{SCROLLBACK, SettingsStore, StartIn},
};

impl SettingsWindow {
    /// The program a session runs, where it starts and how far back it remembers.
    pub(super) fn terminal(&self, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let shell = PathInput::new("settings-shell", &self.shell).on_choose(chosen(&this, "shell"));
        let shell = path_field(shell, self.refused.clone(), cx);
        let folder = PathInput::new("settings-start-folder", &self.folder)
            .directories()
            .on_choose(chosen(&this, "start_folder"));
        let folder = path_field(folder, self.refused_folder.clone(), cx);
        let start = match settings.start_in {
            StartIn::Home => "home",
            StartIn::Session => "session",
            StartIn::Folder => "folder",
        };
        let start = SegmentedControl::new("settings-start-in", start)
            .segment("home", text(cx, "settings.start_in.home", &[]), None)
            .segment("session", text(cx, "settings.start_in.session", &[]), None)
            .segment("folder", text(cx, "settings.start_in.folder", &[]), None)
            .size(ControlSize::Sm)
            .on_change({
                let this = this.clone();
                move |value, _, cx| start_in(&this, value, cx)
            });
        let lines = NumberInput::new(("scrollback", self.drafts), f64::from(settings.scrollback))
            .range(f64::from(*SCROLLBACK.start()), f64::from(*SCROLLBACK.end()))
            .step(1000.)
            .size(ControlSize::Sm)
            .on_commit({
                let this = this.clone();
                move |lines, _, cx| {
                    let lines = lines as u32;
                    set(&this, format!("scrollback is {lines}"), cx, |settings| {
                        settings.scrollback = lines
                    });
                }
            });
        let section = SettingsSection::new(text(cx, "settings.terminal", &[]))
            .row(SettingsRow::new(text(cx, "settings.shell", &[])).control(shell))
            .row(SettingsRow::new(text(cx, "settings.start_in", &[])).control(start))
            .row(
                SettingsRow::new(text(cx, "settings.start_folder", &[]))
                    .description(text(cx, "settings.start_folder_hint", &[]))
                    .control(folder),
            )
            .row(SettingsRow::new(text(cx, "settings.scrollback", &[])).control(lines));
        div().flex().flex_col().gap_6().child(section)
    }
}

/// Hands Browse's pick for `field` to the window.
fn chosen(
    this: &Entity<SettingsWindow>,
    field: &'static str,
) -> impl Fn(PathBuf, &mut Window, &mut App) + 'static {
    let this = this.clone();
    move |path, _, cx| this.update(cx, |this, cx| this.chosen(field, path, cx))
}

/// A path field, with why its text is not saved.
fn path_field(path: PathInput, refused: Option<SharedString>, cx: &App) -> Div {
    let refused = refused.map(|why| InlineMessage::new(Severity::Danger, why));
    let field = div().w(cx.theme().prose_width()).flex().flex_col().gap_1();
    field.child(path).children(refused)
}

/// Where new sessions start, as the settings keep it.
fn start_in(this: &Entity<SettingsWindow>, value: &SharedString, cx: &mut App) {
    let start = match value.as_ref() {
        "home" => StartIn::Home,
        "session" => StartIn::Session,
        "folder" => StartIn::Folder,
        other => unreachable!("settings: no start {other}"),
    };
    set(
        this,
        format!("new sessions start in {start:?}"),
        cx,
        |settings| settings.start_in = start,
    );
}
