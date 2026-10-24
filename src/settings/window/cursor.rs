use ely_gpui_component::{
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, Radius},
};
use gpui::{
    App, Context, Div, ParentElement, Styled, Window, div, prelude::FluentBuilder, px, relative,
};

use super::{SettingsWindow, choice, switch};
use crate::{
    i18n::text,
    settings::{CursorStyle, Settings, SettingsStore},
    themes::{self, TerminalTheme},
};

/// Each cursor style, its key and its name's message.
const CURSORS: [(CursorStyle, &str, &str); 4] = [
    (CursorStyle::Block, "block", "cursor.block"),
    (CursorStyle::Bar, "bar", "cursor.bar"),
    (CursorStyle::Underline, "underline", "cursor.underline"),
    (CursorStyle::Hollow, "hollow", "cursor.hollow"),
];

impl SettingsWindow {
    /// The cursor's shape, its blink and a prompt drawn as the terminal draws it.
    pub(super) fn cursor(&self, window: &Window, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let style = choice(
            &this,
            "cursor_style",
            CURSORS
                .iter()
                .map(|(each, key, name)| (*each, *key, text(cx, name, &[]))),
            settings.cursor_style,
            |settings, style| settings.cursor_style = style,
        );
        let active = themes::active(settings, window.appearance(), cx);
        let lit = self.lit || !settings.cursor_blink;
        let cursor = SettingsSection::new(text(cx, "settings.cursor", &[]))
            .row(SettingsRow::new(text(cx, "settings.cursor.style", &[])).control(style))
            .row(
                SettingsRow::new(text(cx, "settings.cursor.blink", &[])).control(switch(
                    &this,
                    "cursor_blink",
                    settings.cursor_blink,
                    |settings, on| settings.cursor_blink = on,
                )),
            )
            .row(
                SettingsRow::new(text(cx, "settings.cursor.preview", &[]))
                    .control(prompt(settings, &active, lit, cx)),
            );
        div().flex().flex_col().gap_6().child(cursor)
    }
}

/// A prompt in the terminal's theme and font, its cursor drawn as the terminal draws it.
fn prompt(settings: &Settings, theme: &TerminalTheme, lit: bool, cx: &App) -> Div {
    let rule = (px(settings.font_size) / 14.).max(px(1.));
    let mark = div().absolute().bg(theme.cursor);
    let cursor = match settings.cursor_style {
        CursorStyle::Block => mark.inset_0(),
        CursorStyle::Bar => mark.top_0().bottom_0().left_0().w(rule * 2.),
        CursorStyle::Underline => mark.left_0().right_0().bottom_0().h(rule * 2.),
        CursorStyle::Hollow => div()
            .absolute()
            .inset_0()
            .border_1()
            .border_color(theme.cursor),
    };
    let chrome = cx.theme();
    div()
        .flex()
        .p_2()
        .rounded(chrome.radius(Radius::Sm))
        .border_1()
        .border_color(chrome.colors.border)
        .bg(theme.background)
        .text_color(theme.foreground)
        .font_family(settings.font_family.clone())
        .text_size(px(settings.font_size))
        .line_height(relative(settings.line_height))
        .child(div().text_color(theme.ansi[2]).child("$"))
        .child(" ls")
        .child(
            div()
                .relative()
                .child(" ")
                .when(lit, |cell| cell.child(cursor)),
        )
}
