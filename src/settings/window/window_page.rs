use ely_gpui_component::{
    forms::Slider,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, TextSize},
};
use gpui::{Context, Div, ParentElement, Styled, div};

use super::{SettingsWindow, set, switch};
use crate::{
    i18n::text,
    settings::{OPACITY, PADDING, SettingsStore},
};

impl SettingsWindow {
    /// What the window shows of the desktop, its padding and the side columns' sides.
    pub(super) fn window_page(&self, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let theme = cx.theme();
        let this = cx.entity();
        let opacity = percent(settings.background_opacity);
        let slider = Slider::new("background_opacity", opacity)
            .range(percent(*OPACITY.start()), percent(*OPACITY.end()))
            .on_change({
                let this = this.clone();
                move |percent, _, cx| {
                    let value = fraction(percent);
                    set(
                        &this,
                        format!("background_opacity is {value}"),
                        cx,
                        |settings| settings.background_opacity = value,
                    );
                }
            });
        let opacity = div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().w(theme.label_width()).child(slider))
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(format!("{opacity}%")),
            );
        let section = SettingsSection::new(text(cx, "settings.window", &[]))
            .row(SettingsRow::new(text(cx, "settings.opacity", &[])).control(opacity))
            .row(
                SettingsRow::new(text(cx, "settings.blur", &[])).control(switch(
                    &this,
                    "background_blur",
                    settings.background_blur,
                    |settings, on| settings.background_blur = on,
                )),
            )
            .row(
                SettingsRow::new(text(cx, "settings.padding_x", &[])).control(self.number(
                    "padding_x",
                    settings.padding_x,
                    &PADDING,
                    (1.0, 0),
                    |settings, padding| settings.padding_x = padding,
                    cx,
                )),
            )
            .row(
                SettingsRow::new(text(cx, "settings.padding_y", &[])).control(self.number(
                    "padding_y",
                    settings.padding_y,
                    &PADDING,
                    (1.0, 0),
                    |settings, padding| settings.padding_y = padding,
                    cx,
                )),
            )
            .row(
                SettingsRow::new(text(cx, "settings.swap_panels", &[]))
                    .description(text(cx, "settings.swap_panels.hint", &[]))
                    .control(switch(
                        &this,
                        "swap_panels",
                        settings.swap_panels,
                        |settings, on| settings.swap_panels = on,
                    )),
            );
        div().flex().flex_col().gap_6().child(section)
    }
}

/// `fraction` in whole percents; the slider moves by them, so a saved fraction carries no float noise.
fn percent(fraction: f32) -> f64 {
    (f64::from(fraction) * 100.).round()
}

fn fraction(percent: f64) -> f32 {
    (percent / 100.) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_percent_saves_as_its_own_short_fraction() {
        let (low, high) = (percent(*OPACITY.start()), percent(*OPACITY.end()));
        assert_eq!((low, high), (30., 100.));
        for whole in 30..=100 {
            let saved = fraction(f64::from(whole));
            assert_eq!(percent(saved), f64::from(whole));
            let text = serde_json::to_string(&saved).expect("a number");
            assert!(text.len() <= 4, "{whole}% saves as {text}");
        }
    }
}
