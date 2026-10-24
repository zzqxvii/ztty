use ely_gpui_component::settings::{SettingsRow, SettingsSection};
use gpui::{Context, Div, ParentElement, Styled, div};

use super::{SettingsWindow, choice, switch};
use crate::{
    i18n::text,
    settings::{OptionAsAlt, SettingsStore},
};

/// Each way Option types Alt, its key and its name's message.
const OPTIONS: [(OptionAsAlt, &str, &str); 4] = [
    (OptionAsAlt::Off, "off", "option.off"),
    (OptionAsAlt::Left, "left", "option.left"),
    (OptionAsAlt::Right, "right", "option.right"),
    (OptionAsAlt::Both, "both", "option.both"),
];

impl SettingsWindow {
    /// What the keyboard sends: Option as Alt, natural text editing on macOS, copy on select everywhere.
    pub(super) fn input(&self, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let mut section = SettingsSection::new(text(cx, "settings.input", &[]));
        // Off macOS, Alt is always Alt.
        if cfg!(target_os = "macos") {
            let option = choice(
                &this,
                "option_as_alt",
                OPTIONS
                    .iter()
                    .map(|(each, key, name)| (*each, *key, text(cx, name, &[]))),
                settings.option_as_alt,
                |settings, option| settings.option_as_alt = option,
            );
            let natural = switch(
                &this,
                "natural_text_editing",
                settings.natural_text_editing,
                |settings, on| settings.natural_text_editing = on,
            );
            section = section
                .row(SettingsRow::new(text(cx, "settings.option_as_alt", &[])).control(option))
                .row(
                    SettingsRow::new(text(cx, "settings.natural_text_editing", &[]))
                        .description(text(cx, "settings.natural_text_editing.hint", &[]))
                        .control(natural),
                );
        }
        let copy = switch(
            &this,
            "copy_on_select",
            settings.copy_on_select,
            |settings, on| settings.copy_on_select = on,
        );
        let section =
            section.row(SettingsRow::new(text(cx, "settings.copy_on_select", &[])).control(copy));
        div().flex().flex_col().gap_6().child(section)
    }
}
