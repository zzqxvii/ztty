mod cards;

use cards::grid;
use ely_gpui_component::{
    forms::Switch,
    settings::{SettingsRow, SettingsSection},
};
use gpui::{Context, Div, ParentElement, Styled, Window, div};

use super::{SettingsWindow, set};
use crate::{i18n::text, settings::SettingsStore};

impl SettingsWindow {
    /// The light theme, and when the system is dark the dark one.
    pub(super) fn appearance(&self, window: &Window, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let separate = settings.dark_theme.is_some();
        let dark = Switch::new("dark_theme", separate).on_change({
            let this = this.clone();
            move |on, _, cx| {
                let theme = SettingsStore::get(cx).theme.clone();
                set(
                    &this,
                    format!("separate dark theme is {on}"),
                    cx,
                    |settings| settings.dark_theme = on.then_some(theme),
                );
            }
        });
        let light = separate.then(|| text(cx, "settings.theme.light", &[]));
        let mut themes = SettingsSection::new(text(cx, "settings.theme", &[]))
            .row(grid(&this, light, false, settings, window, cx))
            .row(SettingsRow::new(text(cx, "settings.dark_theme", &[])).control(dark));
        if separate {
            let dark = Some(text(cx, "settings.theme.dark", &[]));
            themes = themes.row(grid(&this, dark, true, settings, window, cx));
        }
        div().flex().flex_col().gap_6().child(themes)
    }
}
