//! The language the app speaks: the catalogs it holds, and the system's when the settings name none.

use ely_gpui_component::i18n::I18n;
use gpui::{App, SharedString};

mod en;
mod system;
mod zh;

/// Each language the app speaks, named in itself.
pub const LOCALES: [(&str, &str); 2] = [("en-US", "English"), ("zh-CN", "中文")];

/// Whether the app speaks `tag`.
pub fn speaks(tag: &str) -> bool {
    LOCALES.iter().any(|(each, _)| *each == tag)
}

/// The system's language, among those the app speaks.
pub fn system() -> &'static str {
    system::tag()
}

/// `tag`'s catalog.
fn messages(tag: &str) -> &'static [(&'static str, &'static str)] {
    match tag {
        "en-US" => en::MESSAGES,
        "zh-CN" => zh::MESSAGES,
        other => unreachable!("i18n: no language {other}"),
    }
}

/// Installs every catalog with `tag` shown.
pub fn install(cx: &mut App, tag: &str) {
    let mut i18n = I18n::new(tag);
    for (each, _) in LOCALES {
        i18n = i18n.catalog(each, messages(each));
    }
    cx.set_global(i18n);
    log::info!("i18n: {} installed, {} messages", tag, messages(tag).len());
}

/// Shows `tag` in every window and tells the menu bar.
pub fn set_locale(tag: &str, cx: &mut App) {
    I18n::set_locale(tag, cx);
    crate::menus::install(cx);
}

/// `key`'s message in the shown language, each `{name}` filled from `args`; a key no catalog holds stops the app.
pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
    cx.global::<I18n>().text(key, args)
}

/// The English catalog alone, for tests that read messages without an app.
#[cfg(test)]
pub(crate) fn english() -> I18n {
    I18n::new("en-US").catalog("en-US", en::MESSAGES)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Each `{name}` a message fills.
    fn placeholders(message: &str) -> BTreeSet<&str> {
        message
            .split('{')
            .skip(1)
            .filter_map(|rest| rest.split('}').next())
            .collect()
    }

    #[test]
    fn every_language_holds_the_same_keys_in_the_same_order() {
        let source: Vec<&str> = en::MESSAGES.iter().map(|(key, _)| *key).collect();
        for (tag, _) in LOCALES {
            let keys: Vec<&str> = messages(tag).iter().map(|(key, _)| *key).collect();
            assert_eq!(keys, source, "{tag} holds another set of keys");
            for (key, message) in messages(tag) {
                assert!(!message.is_empty(), "{tag}'s {key} is empty");
            }
        }
    }

    #[test]
    fn every_language_fills_the_same_arguments() {
        for (key, english) in en::MESSAGES {
            let (_, chinese) = zh::MESSAGES
                .iter()
                .find(|(each, _)| each == key)
                .expect("every key is translated");
            assert_eq!(placeholders(english), placeholders(chinese), "{key}");
        }
    }

    #[test]
    fn the_catalogs_build_and_hold_no_duplicate_key() {
        let mut i18n = I18n::new("en-US");
        for (tag, _) in LOCALES {
            i18n = i18n.catalog(tag, messages(tag));
        }
        assert_eq!(i18n.locale().tag, "en-US");
    }

    #[test]
    fn only_the_languages_named_are_spoken() {
        assert!(speaks("en-US"));
        assert!(speaks("zh-CN"));
        assert!(!speaks("de-DE"));
        assert!(!speaks(""));
    }
}
