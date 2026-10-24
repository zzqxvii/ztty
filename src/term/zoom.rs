use gpui::{App, Global};

use crate::settings::{FONT_SIZE, SettingsStore};

/// Unsaved font-size offset.
#[derive(Default)]
pub struct Enlarged(f32);

impl Global for Enlarged {}

/// The font size the terminal draws.
pub fn size(cx: &App) -> f32 {
    shown(SettingsStore::get(cx).font_size, cx.global::<Enlarged>().0)
}

/// Moves the font `by` points; none resets it.
pub fn step(by: Option<f32>, cx: &mut App) {
    let setting = SettingsStore::get(cx).font_size;
    let enlarged = by.map_or(0.0, |by| moved(setting, cx.global::<Enlarged>().0, by));
    cx.set_global(Enlarged(enlarged));
    log::info!(
        "zoom: the terminal's font is {} points, {enlarged:+} on the setting",
        shown(setting, enlarged)
    );
}

fn shown(setting: f32, enlarged: f32) -> f32 {
    (setting + enlarged).clamp(*FONT_SIZE.start(), *FONT_SIZE.end())
}

/// The offset after moving `by` from the shown size.
fn moved(setting: f32, enlarged: f32, by: f32) -> f32 {
    shown(shown(setting, enlarged) + by, 0.0) - setting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_moves_the_font_inside_the_range() {
        assert_eq!(moved(13.0, 0.0, 1.0), 1.0);
        assert_eq!(moved(13.0, 2.0, -1.0), 1.0);
        assert_eq!(moved(71.0, 0.0, 1.0), 1.0);
        assert_eq!(moved(72.0, 0.0, 1.0), 0.0, "the top holds");
        assert_eq!(moved(6.0, 0.0, -1.0), 0.0, "the bottom holds");
        assert_eq!(
            shown(70.0, 5.0),
            72.0,
            "a larger setting under a zoom stays in range"
        );
        assert_eq!(moved(70.0, 5.0, -1.0), 1.0, "a step starts from what shows");
    }
}
