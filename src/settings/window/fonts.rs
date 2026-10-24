use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{FontPicker, TextInput},
    primitives::IconName,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, ControlSize},
};
use gpui::{
    App, AppContext, Context, Div, Entity, ParentElement, SharedString, Styled, Window, div,
};

use super::{SettingsWindow, set, switch};
use crate::{
    i18n::text,
    settings::{FONT_SIZE, LINE_HEIGHT, SettingsStore},
};

impl SettingsWindow {
    /// The terminal's font, then the one the file tabs draw in.
    pub(super) fn fonts(&self, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let theme = cx.theme();
        let this = cx.entity();
        let family = FontPicker::new("font_family", &self.font)
            .selected(settings.font_family.clone())
            .on_change({
                let this = this.clone();
                move |family, window, cx| {
                    let family = family.to_string();
                    let saved = set(
                        &this,
                        format!("font_family is {family:?}"),
                        cx,
                        |settings| settings.font_family = family,
                    );
                    // A refused pick shows the saved family again, the list closed.
                    if !saved {
                        let saved = SettingsStore::get(cx).font_family.clone();
                        let input = this.read(cx).font.clone();
                        input.update(cx, |input, cx| input.set_text(saved, cx));
                        window.blur(cx);
                    }
                }
            });
        let fallbacks = SettingsRow::new(text(cx, "settings.font.fallbacks", &[]))
            .description(text(cx, "settings.font.fallbacks.hint", &[]))
            .control(self.fallback_list(cx));
        let terminal = SettingsSection::new(text(cx, "settings.fonts.terminal", &[]))
            .row(
                SettingsRow::new(text(cx, "settings.font.family", &[]))
                    .control(div().w(theme.prose_width()).child(family)),
            )
            .row(fallbacks)
            .row(
                SettingsRow::new(text(cx, "settings.font.size", &[])).control(self.number(
                    "font_size",
                    settings.font_size,
                    &FONT_SIZE,
                    (1.0, 1),
                    |settings, size| settings.font_size = size,
                    cx,
                )),
            )
            .row(
                SettingsRow::new(text(cx, "settings.font.line_height", &[])).control(self.number(
                    "line_height",
                    settings.line_height,
                    &LINE_HEIGHT,
                    (0.05, 2),
                    |settings, height| settings.line_height = height,
                    cx,
                )),
            )
            .row(
                SettingsRow::new(text(cx, "settings.font.ligatures", &[])).control(switch(
                    &this,
                    "ligatures",
                    settings.ligatures,
                    |settings, on| settings.ligatures = on,
                )),
            );
        let file_family = FontPicker::new("editor_font_family", &self.editor_font)
            .selected(settings.editor_font_family.clone())
            .on_change({
                let this = this.clone();
                move |family, window, cx| {
                    let family = family.to_string();
                    let saved = set(
                        &this,
                        format!("editor_font_family is {family:?}"),
                        cx,
                        |settings| settings.editor_font_family = family,
                    );
                    if !saved {
                        let saved = SettingsStore::get(cx).editor_font_family.clone();
                        let input = this.read(cx).editor_font.clone();
                        input.update(cx, |input, cx| input.set_text(saved, cx));
                        window.blur(cx);
                    }
                }
            });
        let files = SettingsSection::new(text(cx, "settings.fonts.files", &[]))
            .row(
                SettingsRow::new(text(cx, "settings.font.family", &[]))
                    .control(div().w(theme.prose_width()).child(file_family)),
            )
            .row(
                SettingsRow::new(text(cx, "settings.font.size", &[])).control(self.number(
                    "editor_font_size",
                    settings.editor_font_size,
                    &FONT_SIZE,
                    (1.0, 1),
                    |settings, size| settings.editor_font_size = size,
                    cx,
                )),
            );
        div().flex().flex_col().gap_6().child(terminal).child(files)
    }

    /// An input for each fallback picker and the one that adds a family.
    pub(super) fn grow_fallbacks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wanted = SettingsStore::get(cx).font_fallbacks.len() + 1;
        while self.fallbacks.len() < wanted {
            let input = cx.new(|cx| TextInput::new(window, cx));
            self.fallbacks.push(input);
        }
    }

    /// Each fallback family, a picker that replaces it and a button that removes it, then Add or the empty picker it opens.
    fn fallback_list(&self, cx: &Context<Self>) -> Div {
        let this = cx.entity();
        let families = SettingsStore::get(cx).font_fallbacks.clone();
        let removed_tip = text(cx, "settings.font.remove", &[]);
        let cancel_tip = text(cx, "settings.font.cancel", &[]);
        let width = cx.theme().prose_width();
        let row = |index: usize, picker: FontPicker, remove: IconButton| {
            let tip = if index < families.len() {
                removed_tip.clone()
            } else {
                cancel_tip.clone()
            };
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(div().w(width).child(picker))
                .child(
                    remove
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .tooltip(tip),
                )
        };
        let rows = families.iter().enumerate().map(|(index, family)| {
            let picker = FontPicker::new(keyed(&self.fallbacks[index]), &self.fallbacks[index])
                .selected(family.clone())
                .on_change(pick(&this, index));
            let remove = IconButton::new(("remove_fallback", index), IconName::X).on_click({
                let (this, family) = (this.clone(), family.clone());
                move |_, window, cx| {
                    let dropped = format!("font_fallbacks drops {family:?}");
                    let removed = set(&this, dropped, cx, |settings| {
                        settings.font_fallbacks.remove(index);
                    });
                    // Each input left keeps its own family, and none the dropped one.
                    if removed {
                        this.update(cx, |this, _| this.fallbacks.remove(index));
                        window.blur(cx);
                    }
                }
            });
            row(index, picker, remove)
        });
        let next = families.len();
        let add = if self.adding {
            let picker = FontPicker::new(keyed(&self.fallbacks[next]), &self.fallbacks[next])
                .on_change(pick(&this, next));
            let cancel = IconButton::new("cancel_fallback", IconName::X).on_click({
                let this = this.clone();
                move |_, window, cx| {
                    this.update(cx, |this, cx| this.leave_adding(cx));
                    window.blur(cx);
                }
            });
            row(next, picker, cancel)
        } else {
            let add = Button::new("add_fallback", text(cx, "settings.font.add_fallback", &[]))
                .icon(IconName::Plus)
                .size(ControlSize::Sm)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |this, cx| {
                            this.adding = true;
                            cx.notify();
                        })
                    }
                });
            div().child(add)
        };
        div().flex().flex_col().gap_2().children(rows).child(add)
    }

    /// Closes the empty picker, its text gone.
    fn leave_adding(&mut self, cx: &mut Context<Self>) {
        let next = SettingsStore::get(cx).font_fallbacks.len();
        if let Some(input) = self.fallbacks.get(next) {
            input.update(cx, |input, cx| input.set_text("", cx));
        }
        self.adding = false;
        cx.notify();
    }
}

/// A fallback picker's id, its input's: Ely's combobox keeps state by id and listens to the input it met first.
fn keyed(input: &Entity<TextInput>) -> (&'static str, u64) {
    ("font_fallback", input.entity_id().as_u64())
}

/// Saves the family picked at fallback `index`, past the last adds one; a refusal shows the saved family again, the list closed.
fn pick(
    this: &Entity<SettingsWindow>,
    index: usize,
) -> impl Fn(&SharedString, &mut Window, &mut App) + 'static {
    let this = this.clone();
    move |family, window, cx| {
        let family = family.to_string();
        let adds = index == SettingsStore::get(cx).font_fallbacks.len();
        let saved = set(
            &this,
            format!("font_fallbacks[{index}] is {family:?}"),
            cx,
            |settings| {
                if adds {
                    settings.font_fallbacks.push(family)
                } else {
                    settings.font_fallbacks[index] = family
                }
            },
        );
        if adds && saved {
            this.update(cx, |this, cx| {
                this.adding = false;
                cx.notify();
            });
        } else if !saved {
            let shown = SettingsStore::get(cx)
                .font_fallbacks
                .get(index)
                .cloned()
                .unwrap_or_default();
            let input = this.read(cx).fallbacks[index].clone();
            input.update(cx, |input, cx| input.set_text(shown, cx));
            window.blur(cx);
        }
    }
}
