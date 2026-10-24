use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context as _;
use gpui::App;

use super::{OMARCHY, Omarchy, TerminalTheme, apply, parse};
use crate::settings::SettingsStore;

/// How often Omarchy's theme file is read for a switch.
const CHECK: Duration = Duration::from_millis(500);

/// A Ghostty theme file's keys; Omarchy's other lines pass by.
const COLORS: [&str; 7] = [
    "palette",
    "background",
    "foreground",
    "cursor-color",
    "cursor-text",
    "selection-background",
    "selection-foreground",
];

/// Omarchy 4's current theme for Ghostty, under $HOME whatever XDG_STATE_HOME says.
pub fn path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".local/state/omarchy/current/theme/ghostty.conf"))
}

/// Follows Omarchy's theme; an unreadable file, or a bad one worn, stops the start.
pub fn start(cx: &mut App) -> anyhow::Result<()> {
    let settings = SettingsStore::get(cx);
    let worn = settings.theme == OMARCHY || settings.dark_theme.as_deref() == Some(OMARCHY);
    let path = path().context("omarchy: no home folder")?;
    match read(&path) {
        Ok((text, theme)) => {
            log::info!("omarchy: the theme follows {}", path.display());
            cx.set_global(Omarchy(theme));
            watch(path, text, cx);
            Ok(())
        }
        Err(error) if worn => Err(error.context("settings wear the Omarchy theme")),
        Err(error) if missing(&error) => {
            log::info!("omarchy: none at {}", path.display());
            Ok(())
        }
        Err(error) if error.is::<io::Error>() => Err(error),
        Err(error) => {
            log::error!("{error:#}; no Omarchy theme to follow");
            Ok(())
        }
    }
}

/// Omarchy signals only Alacritty, kitty and Ghostty, so this reads the file.
fn watch(path: PathBuf, mut shown: String, cx: &mut App) {
    cx.spawn(async move |cx| {
        let mut failed = String::new();
        loop {
            cx.background_executor().timer(CHECK).await;
            match check(&path, &shown) {
                Ok(None) => failed.clear(),
                Ok(Some((text, theme))) => {
                    shown = text;
                    failed.clear();
                    cx.update(|cx| {
                        log::info!("omarchy: the theme changed");
                        cx.set_global(Omarchy(theme));
                        apply(cx);
                    });
                }
                Err(error) => {
                    let error = format!("{error:#}");
                    if error != failed {
                        log::error!("{error}; the last Omarchy theme stays");
                        failed = error;
                    }
                }
            }
        }
    })
    .detach();
}

/// The file's theme when its text is not `shown`'s; none while unchanged.
pub(super) fn check(path: &Path, shown: &str) -> anyhow::Result<Option<(String, TerminalTheme)>> {
    match read(path) {
        Ok((text, _)) if text == shown => Ok(None),
        // Gone mid-switch: the next check finds the new folder.
        Err(error) if missing(&error) => Ok(None),
        next => next.map(Some),
    }
}

fn read(path: &Path) -> anyhow::Result<(String, TerminalTheme)> {
    let text =
        fs::read_to_string(path).with_context(|| format!("omarchy: reading {}", path.display()))?;
    let theme = colors(&text).with_context(|| format!("omarchy: {}", path.display()))?;
    Ok((text, theme))
}

fn missing(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == ErrorKind::NotFound)
}

/// Omarchy writes no cursor-text; the background stands in, as Ghostty's default.
pub(super) fn colors(text: &str) -> anyhow::Result<TerminalTheme> {
    let key = |line: &str| {
        line.split_once('=')
            .map_or("", |(key, _)| key.trim())
            .to_owned()
    };
    let mut lines: Vec<&str> = text
        .lines()
        .filter(|line| COLORS.contains(&key(line).as_str()))
        .collect();
    let background = lines
        .iter()
        .find(|line| key(line) == "background")
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| format!("cursor-text = {}", value.trim()));
    if !lines.iter().any(|line| key(line) == "cursor-text") {
        lines.extend(background.as_deref());
    }
    parse(OMARCHY, "Omarchy", &lines.join("\n"))
}
