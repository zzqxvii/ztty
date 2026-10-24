use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
};

use ely_gpui_component::{
    lists::GitStatus,
    primitives::{Disclosure, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Palette, Radius, TextSize},
};
use gpui::{
    AnyElement, App, AppContext, Context, EventEmitter, Hsla, InteractiveElement, IntoElement,
    ParentElement, Render, Role, SharedString, StatefulInteractiveElement, Styled, Task, Window,
    div, prelude::FluentBuilder, px, relative, uniform_list,
};

use crate::{git, sessions};

/// How far each level of the tree steps in.
const INDENT: f32 = 12.;

/// A folder's entry: its name, and whether it is a folder, links followed.
#[derive(Debug, PartialEq)]
struct Entry {
    name: OsString,
    folder: bool,
}

/// A folder's entries, or why it could not be read.
type Listing = Result<Vec<Entry>, String>;

/// Reads `folder`: folders first, then files, each by name ignoring case.
fn list(folder: &Path) -> Listing {
    let read = || -> io::Result<Vec<Entry>> {
        fs::read_dir(folder)?
            .map(|entry| {
                let entry = entry?;
                let kind = entry.file_type()?;
                Ok(Entry {
                    folder: kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()),
                    name: entry.file_name(),
                })
            })
            .collect()
    };
    let mut entries = read().map_err(|error| error.to_string())?;
    entries.sort_by_cached_key(|entry| {
        let name = entry.name.to_string_lossy().to_lowercase();
        (!entry.folder, name, entry.name.clone())
    });
    Ok(entries)
}

/// `path` as marks are keyed: on macOS, whose file systems ignore Unicode forms, composed as Git writes it.
fn mark_key(path: &Path) -> Cow<'_, Path> {
    #[cfg(target_os = "macos")]
    if let Some(text) = path.to_str()
        && !unicode_normalization::is_nfc(text)
    {
        use unicode_normalization::UnicodeNormalization;
        return Cow::Owned(text.nfc().collect::<String>().into());
    }
    Cow::Borrowed(path)
}

/// Changes by their keys, and the folders from `root` down that hold one.
fn marks(
    root: &Path,
    changes: Vec<(PathBuf, GitStatus)>,
) -> (HashMap<PathBuf, GitStatus>, HashSet<PathBuf>) {
    let root = mark_key(root);
    let marks: HashMap<PathBuf, GitStatus> = changes
        .into_iter()
        .map(|(path, status)| (mark_key(&path).into_owned(), status))
        .collect();
    let dirty = marks
        .keys()
        .flat_map(|path| {
            let up = path.ancestors().skip(1);
            up.take_while(|folder| folder.starts_with(&root))
        })
        .map(Path::to_path_buf)
        .collect();
    (marks, dirty)
}

/// Git's letter for a mark, in its tint.
fn marked(mark: GitStatus, colors: &Palette) -> (&'static str, Hsla) {
    match mark {
        GitStatus::Modified => ("M", colors.warning),
        GitStatus::Added => ("A", colors.success),
        GitStatus::Deleted => ("D", colors.danger),
        GitStatus::Untracked => ("U", colors.success),
        GitStatus::Renamed => ("R", colors.info),
        GitStatus::Conflicted => ("!", colors.danger),
    }
}

/// What a row says past its name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Note {
    /// Git's mark on the path.
    Mark(GitStatus),
    /// A folder holding a change.
    Dirty,
    /// Why a folder shows nothing.
    Unreadable,
}

/// A row of the tree: a folder or a file, how deep it sits, and what marks it.
struct Row {
    path: PathBuf,
    depth: usize,
    label: SharedString,
    folder: bool,
    open: bool,
    /// Whether a chevron shows: a folder holding entries, or one the tree has yet to read.
    chevron: bool,
    note: Option<Note>,
}

impl Row {
    /// One row: its chevron, its icon, its name and Git's mark.
    fn draw(&self, files: &gpui::Entity<Files>, unreadable: &SharedString, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let note = match self.note {
            Some(Note::Mark(mark)) => {
                let (letter, ink) = marked(mark, colors);
                Some((SharedString::from(letter), ink))
            }
            Some(Note::Dirty) => Some((SharedString::from("•"), colors.warning)),
            Some(Note::Unreadable) => Some((unreadable.clone(), colors.danger)),
            None => None,
        };
        let (path, folder) = (self.path.clone(), self.folder);
        let files = files.clone();
        let chevron = SharedString::from(format!("chevron-{}", key(&self.path)));
        div()
            .id(key(&self.path))
            .role(Role::TreeItem)
            .aria_label(self.label.clone())
            .aria_level(self.depth + 1)
            .when(self.chevron, |row| row.aria_expanded(self.open))
            .w_full()
            .h(theme.control_height(ControlSize::Md))
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .rounded(theme.radius(Radius::Sm))
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.fg)
            .cursor_pointer()
            .hover(|row| row.bg(colors.hover))
            .on_click(move |_, _, cx| {
                files.update(cx, |files, cx| files.press(path.clone(), folder, cx));
            })
            .child(
                div()
                    .flex_none()
                    .flex()
                    .justify_center()
                    .w(px(INDENT))
                    .ml(px(INDENT * self.depth as f32))
                    .when(self.chevron, |slot| {
                        slot.child(Disclosure::new(chevron, self.open))
                    }),
            )
            .child(
                Icon::new(if self.folder {
                    IconName::Folder
                } else {
                    IconName::File
                })
                .size(IconSize::Sm)
                .color(colors.fg_muted),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(self.label.clone()),
            )
            .children(note.map(|(text, ink)| {
                div()
                    .flex_none()
                    .max_w(relative(0.5))
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(ink)
                    .truncate()
                    .child(text)
            }))
            .into_any_element()
    }
}

/// `path`'s tree key, one to a path: a path not in UTF-8 adds its bytes after a NUL, which no path holds.
fn key(path: &Path) -> SharedString {
    match path.to_str() {
        Some(text) => text.to_string().into(),
        None => {
            let bytes = path.as_os_str().as_encoded_bytes();
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            format!("{}\0{hex}", path.to_string_lossy()).into()
        }
    }
}

/// Files asks the window to show a file in a tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened(pub PathBuf);

/// The Files panel: the shown session's folder as a tree, Git's marks on names.
pub struct Files {
    root: PathBuf,
    /// Counts roots, so each root starts a fresh tree.
    generation: usize,
    listings: HashMap<PathBuf, Listing>,
    /// The root and the folders read so far.
    wanted: HashSet<PathBuf>,
    /// The folders the tree shows open.
    open: HashSet<PathBuf>,
    marks: HashMap<PathBuf, GitStatus>,
    /// Folders holding a mark.
    dirty: HashSet<PathBuf>,
    reading: Task<()>,
    checking: Task<()>,
}

impl Files {
    pub fn new(root: PathBuf) -> Self {
        Self {
            wanted: HashSet::from([root.clone()]),
            open: HashSet::new(),
            root,
            generation: 0,
            listings: HashMap::new(),
            marks: HashMap::new(),
            dirty: HashSet::new(),
            reading: Task::ready(()),
            checking: Task::ready(()),
        }
    }

    /// Shows `folder`, read afresh with its marks; a new folder starts a new tree.
    pub fn show(&mut self, folder: &Path, cx: &mut Context<Self>) {
        if folder != self.root {
            log::info!("files: the tree shows {}", folder.display());
            *self = Self {
                generation: self.generation + 1,
                ..Self::new(folder.to_path_buf())
            };
        }
        self.read(cx);
        let root = self.root.clone();
        let changes = cx.background_spawn(async move { git::changes(&root) });
        self.checking = cx.spawn(async move |files, cx| {
            let changes = changes.await;
            files.update(cx, |files, cx| files.mark(changes, cx)).ok();
        });
    }

    fn mark(
        &mut self,
        changes: anyhow::Result<Option<Vec<(PathBuf, GitStatus)>>>,
        cx: &mut Context<Self>,
    ) {
        let changes = match changes {
            Ok(changes) => changes.unwrap_or_default(),
            Err(error) => {
                log::error!("files: no Git marks in {}: {error:#}", self.root.display());
                Vec::new()
            }
        };
        let (marks, dirty) = marks(&self.root, changes);
        if marks != self.marks {
            log::debug!(
                "files: {} Git marks in {}",
                marks.len(),
                self.root.display()
            );
        }
        (self.marks, self.dirty) = (marks, dirty);
        cx.notify();
    }

    /// Reads the root and every folder the tree opened, off the main thread.
    fn read(&mut self, cx: &mut Context<Self>) {
        let folders: Vec<PathBuf> = self.wanted.iter().cloned().collect();
        let read = cx.background_spawn(async move {
            let read = folders.into_iter().map(|folder| (list(&folder), folder));
            read.collect::<Vec<_>>()
        });
        self.reading = cx.spawn(async move |files, cx| {
            let read = read.await;
            files.update(cx, |files, cx| files.store(read, cx)).ok();
        });
    }

    fn store(&mut self, read: Vec<(Listing, PathBuf)>, cx: &mut Context<Self>) {
        let mut changed = false;
        for (listing, folder) in read {
            if self.listings.get(&folder) == Some(&listing) {
                continue;
            }
            match &listing {
                Ok(entries) => {
                    log::debug!("files: {} holds {}", folder.display(), entries.len())
                }
                Err(error) => log::warn!("files: {} is unreadable: {error}", folder.display()),
            }
            self.listings.insert(folder, listing);
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    /// Reads `folder` the tree opened, unread so far.
    fn load(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        if self.wanted.insert(folder) {
            self.read(cx);
        }
    }

    /// A row was pressed: a folder folds or unfolds, a file asks for its tab.
    fn press(&mut self, path: PathBuf, folder: bool, cx: &mut Context<Self>) {
        if !folder {
            log::info!("files: {} opens", path.display());
            cx.emit(Opened(path));
            return;
        }
        if self.open.contains(&path) {
            self.open.remove(&path);
            log::debug!("files: {} folds", path.display());
        } else {
            self.open.insert(path.clone());
            log::debug!("files: {} unfolds", path.display());
            if !self.listings.contains_key(&path) {
                self.load(path, cx);
            }
        }
        cx.notify();
    }

    /// The rows the tree shows: the root's entries, then each open folder's; a root it cannot read says why.
    fn rows(&self, root: SharedString) -> Vec<Row> {
        let mut rows = Vec::new();
        match self.listings.get(&self.root) {
            Some(Err(_)) => rows.push(Row {
                path: self.root.clone(),
                depth: 0,
                label: root,
                folder: true,
                open: false,
                chevron: false,
                note: Some(Note::Unreadable),
            }),
            _ => self.entries(&mut rows, &self.root, 0, false),
        }
        rows
    }

    /// Adds `folder`'s entries at `depth`, and what each open one holds under them; `inside` marks what an untracked folder holds.
    fn entries(&self, rows: &mut Vec<Row>, folder: &Path, depth: usize, inside: bool) {
        let Some(Ok(entries)) = self.listings.get(folder) else {
            return;
        };
        for entry in entries {
            let child = folder.join(&entry.name);
            let label: SharedString = sessions::one_line(&entry.name.to_string_lossy()).into();
            if entry.folder {
                self.walk(rows, &child, label, depth, inside);
                continue;
            }
            let own = self.marks.get(mark_key(&child).as_ref()).copied();
            let mark = own.or(inside.then_some(GitStatus::Untracked));
            rows.push(Row {
                path: child,
                depth,
                label,
                folder: false,
                open: false,
                chevron: false,
                note: mark.map(Note::Mark),
            });
        }
    }

    /// Adds `path`'s row and, while it shows open, the rows under it; `inside` marks what an untracked folder holds.
    fn walk(
        &self,
        rows: &mut Vec<Row>,
        path: &Path,
        label: SharedString,
        depth: usize,
        inside: bool,
    ) {
        let own = self.marks.get(mark_key(path).as_ref()).copied();
        let mark = own.or(inside.then_some(GitStatus::Untracked));
        let held = mark == Some(GitStatus::Untracked);
        let listing = self.listings.get(path);
        let open = self.open.contains(path);
        let note = match listing {
            Some(Err(_)) => Some(Note::Unreadable),
            _ => match mark {
                Some(mark) => Some(Note::Mark(mark)),
                None if self.dirty.contains(mark_key(path).as_ref()) => Some(Note::Dirty),
                None => None,
            },
        };
        let holds = match listing {
            None => true,
            Some(Ok(entries)) => !entries.is_empty(),
            Some(Err(_)) => false,
        };
        rows.push(Row {
            path: path.to_path_buf(),
            depth,
            label,
            folder: true,
            open,
            chevron: holds,
            note,
        });
        if open {
            self.entries(rows, path, depth + 1, held);
        }
    }
}

impl EventEmitter<Opened> for Files {}

impl Render for Files {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let home = dirs::home_dir();
        let label = match self.root.file_name() {
            Some(name) if home.as_deref() != Some(self.root.as_path()) => {
                sessions::one_line(&name.to_string_lossy())
            }
            _ => sessions::shown(&self.root, home.as_deref()),
        };
        let rows = Rc::new(self.rows(label.into()));
        let unreadable = crate::i18n::text(cx, "files.unreadable", &[]);
        let files = cx.entity();
        let count = rows.len();
        uniform_list(("files", self.generation), count, move |range, _, cx| {
            range
                .map(|ix| rows[ix].draw(&files, &unreadable, cx))
                .collect::<Vec<_>>()
        })
        .size_full()
    }
}

#[cfg(test)]
mod tests;
