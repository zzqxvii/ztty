mod store;
pub mod window;

use std::{
    env,
    ffi::OsStr,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::{projects, themes};

pub use store::{SettingsStore, load, save};

/// Where a new session starts: home, the shown session's folder, or `start_folder`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartIn {
    #[default]
    Home,
    Session,
    Folder,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyle {
    #[default]
    Block,
    Bar,
    Underline,
    Hollow,
}

/// Which Option keys type Alt on macOS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionAsAlt {
    #[default]
    Off,
    Left,
    Right,
    Both,
}

/// What the right panel shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelTab {
    /// Git's changes: the branch, what changed, the commit field.
    Git,
    /// Git's commits: what the branch is built on.
    Commits,
    #[default]
    Files,
}

/// Everything the settings window edits. A field the file leaves out takes its default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// The language the app speaks, one of `i18n::LOCALES`.
    pub locale: String,
    /// A program path; empty runs the login shell.
    pub shell: String,
    pub start_in: StartIn,
    /// The folder Start in's Folder names; empty until one is chosen.
    pub start_folder: String,
    pub theme: String,
    /// The theme while the system is dark; none keeps `theme`.
    pub dark_theme: Option<String>,
    pub font_family: String,
    /// Families tried in order for a character Font family lacks.
    pub font_fallbacks: Vec<String>,
    pub font_size: f32,
    pub line_height: f32,
    /// The family and size the file tabs draw in, apart from the terminal's.
    pub editor_font_family: String,
    pub editor_font_size: f32,
    pub ligatures: bool,
    pub cursor_style: CursorStyle,
    pub cursor_blink: bool,
    pub background_opacity: f32,
    pub background_blur: bool,
    pub padding_x: f32,
    pub padding_y: f32,
    pub scrollback: u32,
    pub option_as_alt: OptionAsAlt,
    /// Option and Command edit text by word and line, as macOS's text does.
    pub natural_text_editing: bool,
    pub copy_on_select: bool,
    /// A check for a newer release at start and once a day.
    pub check_updates: bool,
    pub show_sessions: bool,
    pub show_panel: bool,
    /// The projects column on the right, Files and Git on the left.
    pub swap_panels: bool,
    pub panel: PanelTab,
    /// The folders the left column lists as projects, in their order.
    pub projects: Vec<PathBuf>,
    /// The sessions and right columns' widths, in pixels, as their borders were dragged.
    pub sessions_width: f32,
    pub panel_width: f32,
    /// The main window's size, unmaximized, in pixels.
    pub window_width: f32,
    pub window_height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // The system's language where the app speaks it, English otherwise.
            locale: crate::i18n::system().into(),
            shell: String::new(),
            start_in: StartIn::Home,
            start_folder: String::new(),
            theme: themes::default_id().into(),
            dark_theme: None,
            font_family: "JetBrains Mono".into(),
            font_fallbacks: Vec::new(),
            font_size: 13.0,
            line_height: 1.2,
            editor_font_family: "JetBrains Mono".into(),
            editor_font_size: 12.0,
            ligatures: true,
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            background_opacity: 0.88,
            background_blur: true,
            padding_x: 10.0,
            padding_y: 8.0,
            scrollback: 10_000,
            option_as_alt: OptionAsAlt::Off,
            natural_text_editing: false,
            copy_on_select: false,
            check_updates: true,
            show_sessions: true,
            show_panel: true,
            swap_panels: false,
            panel: PanelTab::Files,
            projects: Vec::new(),
            sessions_width: 240.0,
            panel_width: 240.0,
            window_width: 1200.0,
            window_height: 760.0,
        }
    }
}

pub const FONT_SIZE: RangeInclusive<f32> = 6.0..=72.0;
pub const LINE_HEIGHT: RangeInclusive<f32> = 1.0..=2.0;
pub const OPACITY: RangeInclusive<f32> = 0.3..=1.0;
pub const PADDING: RangeInclusive<f32> = 0.0..=64.0;
pub const SCROLLBACK: RangeInclusive<u32> = 0..=100_000;
pub const COLUMN: RangeInclusive<f32> = 160.0..=960.0;
pub const WINDOW: RangeInclusive<f32> = 100.0..=16384.0;

impl Settings {
    /// Every value inside its range, every theme built in; the error names the field.
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            crate::i18n::speaks(&self.locale),
            "locale is {:?}, no language the app speaks",
            self.locale
        );
        within("font_size", self.font_size, &FONT_SIZE)?;
        within("editor_font_size", self.editor_font_size, &FONT_SIZE)?;
        within("line_height", self.line_height, &LINE_HEIGHT)?;
        within("background_opacity", self.background_opacity, &OPACITY)?;
        within("padding_x", self.padding_x, &PADDING)?;
        within("padding_y", self.padding_y, &PADDING)?;
        within("scrollback", self.scrollback, &SCROLLBACK)?;
        within("sessions_width", self.sessions_width, &COLUMN)?;
        within("panel_width", self.panel_width, &COLUMN)?;
        within("window_width", self.window_width, &WINDOW)?;
        within("window_height", self.window_height, &WINDOW)?;
        for (field, id) in [
            ("theme", Some(&self.theme)),
            ("dark_theme", self.dark_theme.as_ref()),
        ] {
            if let Some(id) = id {
                anyhow::ensure!(themes::known(id), "{field} is {id:?}, no built-in theme");
            }
        }
        anyhow::ensure!(!self.font_family.is_empty(), "font_family is empty");
        anyhow::ensure!(
            !self.editor_font_family.is_empty(),
            "editor_font_family is empty"
        );
        anyhow::ensure!(
            !self.font_fallbacks.iter().any(String::is_empty),
            "font_fallbacks holds an empty family"
        );
        for path in &self.projects {
            anyhow::ensure!(
                path.is_absolute(),
                "projects holds {}, which is not a full path",
                path.display()
            );
        }
        for (index, path) in self.projects.iter().enumerate() {
            anyhow::ensure!(
                !self.projects[..index]
                    .iter()
                    .any(|held| projects::same(held, path)),
                "projects lists {} twice",
                path.display()
            );
        }
        if !self.shell.is_empty() {
            program_path(&self.shell)?;
        }
        if !self.start_folder.is_empty() {
            start_folder(&self.start_folder)?;
        }
        anyhow::ensure!(
            self.start_in != StartIn::Folder || !self.start_folder.is_empty(),
            "start_in is folder, but start_folder names none"
        );
        Ok(())
    }
}

/// The folder `folder` names for sessions to start in: a full path to one that exists.
pub fn start_folder(folder: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(folder);
    anyhow::ensure!(
        path.is_absolute(),
        "start_folder is {folder:?}, not a full path"
    );
    anyhow::ensure!(
        path.is_dir(),
        "start_folder is {folder:?}, which names no folder"
    );
    Ok(path.to_path_buf())
}

/// The program `shell` names, as a session starts it: a file at a full path, or the first in `PATH`'s full folders.
pub fn program_path(shell: &str) -> anyhow::Result<String> {
    program_path_in(shell, &env::var_os("PATH").unwrap_or_default())
}

/// The program `shell` names, in `folders` as `PATH` lists them.
pub fn program_path_in(shell: &str, folders: &impl AsRef<OsStr>) -> anyhow::Result<String> {
    find(shell, folders.as_ref(), cfg!(windows))
}

/// `shell`'s program, or the first one named so in `folders`, listed as `PATH` lists them; with `exe`, as on Windows, a name without an extension means name.exe.
fn find(shell: &str, folders: &OsStr, exe: bool) -> anyhow::Result<String> {
    let path = Path::new(shell);
    // A relative path, or a relative folder, would hang on each session's own folder.
    let found = if path.is_absolute() {
        program(path).then(|| path.to_path_buf())
    } else if path.components().count() == 1 {
        let name = if exe && !shell.contains('.') {
            format!("{shell}.exe")
        } else {
            shell.to_string()
        };
        env::split_paths(folders)
            .filter(|folder| folder.is_absolute())
            .map(|folder| folder.join(&name))
            .find(|file| program(file))
    } else {
        None
    };
    let found = found.with_context(|| format!("shell is {shell:?}, which names no program"))?;
    let text = found.to_str().with_context(|| {
        format!(
            "shell is {shell:?}, found at {}, which is not UTF-8",
            found.display()
        )
    })?;
    Ok(text.to_string())
}

/// Whether `path` is a file the running user may run, as the system answers, ACLs too.
#[cfg(unix)]
fn program(path: &Path) -> bool {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let Ok(name) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    path.is_file() && unsafe { libc::access(name.as_ptr(), libc::X_OK) } == 0
}

#[cfg(not(unix))]
fn program(path: &Path) -> bool {
    path.is_file()
}

fn within<T: PartialOrd + std::fmt::Display>(
    field: &str,
    value: T,
    range: &RangeInclusive<T>,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        range.contains(&value),
        "{field} is {value}, outside {} to {}",
        range.start(),
        range.end()
    );
    Ok(())
}

#[cfg(test)]
mod tests;
