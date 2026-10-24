use ely_gpui_component::{
    buttons::SegmentedControl,
    feedback::InlineMessage,
    forms::{NumberInput, PathInput},
    primitives::Severity,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, ControlSize},
};
use gpui::{App, Context, Div, Entity, ParentElement, SharedString, Styled, div};

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
        let shell = div()
            .w(cx.theme().prose_width())
            .flex()
            .flex_col()
            .gap_1()
            .child(PathInput::new("settings-shell", &self.shell))
            .children((self.refused.clone()).map(|why| InlineMessage::new(Severity::Danger, why)));
        let start = match settings.start_in {
            StartIn::Home => "home",
            StartIn::Session => "session",
        };
        let start = SegmentedControl::new("settings-start-in", start)
            .segment("home", text(cx, "settings.start_in.home", &[]), None)
            .segment("session", text(cx, "settings.start_in.session", &[]), None)
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
            .row(SettingsRow::new(text(cx, "settings.scrollback", &[])).control(lines));
        div().flex().flex_col().gap_6().child(section)
    }
}

/// Where new sessions start, as the settings keep it.
fn start_in(this: &Entity<SettingsWindow>, value: &SharedString, cx: &mut App) {
    let start = match value.as_ref() {
        "home" => StartIn::Home,
        "session" => StartIn::Session,
        other => unreachable!("settings: no start {other}"),
    };
    set(
        this,
        format!("new sessions start in {start:?}"),
        cx,
        |settings| settings.start_in = start,
    );
}
