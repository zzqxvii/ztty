use ely_gpui_component::{
    buttons::Button,
    forms::{Choice, Select, Switch},
    settings::{SettingsRow, SettingsSection},
    theme::ControlSize,
};
use gpui::{App, Context, Div, Entity, ParentElement, SharedString, Styled, div};

use super::{SettingsWindow, set};
use crate::{
    i18n::{LOCALES, text},
    settings::SettingsStore,
    updates::{self, Updates, VERSION},
};

impl SettingsWindow {
    /// The language, the check for a newer release and what the last check found.
    pub(super) fn general(&self, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let language = Select::new(
            "locale",
            LOCALES.iter().map(|(tag, name)| Choice::new(*tag, *name)),
        )
        .selected(settings.locale.clone())
        .size(ControlSize::Sm)
        .on_change({
            let this = this.clone();
            move |tag, _, cx| {
                let tag = tag.to_string();
                set(&this, format!("locale is {tag}"), cx, |settings| {
                    settings.locale = tag
                });
            }
        });
        let section = SettingsSection::new(text(cx, "settings.general", &[]))
            .row(SettingsRow::new(text(cx, "settings.language", &[])).control(language))
            .row(
                SettingsRow::new(text(cx, "settings.check_updates", &[]))
                    .description(text(cx, "settings.check_updates.hint", &[]))
                    .control(auto_check(&this, settings.check_updates)),
            )
            .row(updates_row(cx));
        div().flex().flex_col().gap_6().child(section)
    }
}

/// The switch for automatic checks; turning it on checks at once.
fn auto_check(this: &Entity<SettingsWindow>, on: bool) -> Switch {
    let this = this.clone();
    Switch::new("check_updates", on).on_change(move |on, _, cx| {
        let change = format!("check_updates is {on}");
        if set(&this, change, cx, |settings| settings.check_updates = on) && on {
            updates::check(false, cx);
        }
    })
}

/// What the last check found, a button that checks now, and one that opens a newer release's page.
fn updates_row(cx: &App) -> SettingsRow {
    let found = cx.global::<Updates>();
    let status: SharedString = match found {
        Updates::Unchecked => text(cx, "settings.updates.unchecked", &[("version", VERSION)]),
        Updates::Checking => text(cx, "settings.updates.checking", &[]),
        Updates::Latest => text(cx, "settings.updates.latest", &[("version", VERSION)]),
        Updates::Newer { version, .. } => text(
            cx,
            "settings.updates.newer",
            &[("version", version), ("current", VERSION)],
        ),
        Updates::Failed(why) => text(cx, "settings.updates.failed", &[("why", why)]),
    };
    let page = match found {
        Updates::Newer { page, .. } => Some(page.clone()),
        _ => None,
    };
    let check = Button::new("check-updates", text(cx, "settings.updates.check_now", &[]))
        .size(ControlSize::Sm)
        .loading(*found == Updates::Checking)
        .on_click(|_, _, cx| updates::check(false, cx));
    let open = page.map(|page| {
        Button::new("open-update", text(cx, "settings.updates.download", &[]))
            .primary()
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                log::info!("updates: Settings opens {page}");
                cx.open_url(&page);
            })
    });
    let control = div()
        .flex()
        .items_center()
        .gap_2()
        .children(open)
        .child(check);
    SettingsRow::new(text(cx, "settings.updates", &[]))
        .description(status)
        .control(control)
}
