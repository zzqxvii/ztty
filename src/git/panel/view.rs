use std::path::Path;

use ely_gpui_component::{
    feedback::Alert,
    git::{ChangeSection, Changed, ChangesList, CommitInput, CommitList},
    lists::GitStatus,
    overlays::ConfirmDialog,
    primitives::{Icon, IconName, Severity},
    theme::{ActiveTheme, TextSize},
    typography::{Caption, Ellipsis},
};
use gpui::{
    App, Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use super::{GitPanel, State};
use crate::git::{Change, Repo};

/// The most files a section lists: Ely's list draws every row, and each row costs every frame.
const LISTED: usize = 500;

/// `path` as the panel shows it, one string to a path, on one line: as it is, or quoted as git quotes a name with control characters, quotes, backslashes or bytes not in UTF-8.
pub(super) fn shown(path: &Path) -> SharedString {
    // Git ends a folder with a slash, and no file shares a folder's name.
    let bytes = path.as_os_str().as_encoded_bytes();
    let bytes = bytes.strip_suffix(b"/").unwrap_or(bytes);
    let plain = |c: char| !c.is_control() && c != '"' && c != '\\';
    if let Ok(text) = std::str::from_utf8(bytes)
        && text.chars().all(plain)
    {
        return text.to_string().into();
    }
    let mut out = String::from("\"");
    let octal = |out: &mut String, byte: u8| out.push_str(&format!("\\{byte:03o}"));
    for chunk in bytes.utf8_chunks() {
        for c in chunk.valid().chars() {
            match c {
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '"' | '\\' => out.extend(['\\', c]),
                c if c.is_control() => {
                    let mut bytes = [0; 4];
                    for byte in c.encode_utf8(&mut bytes).bytes() {
                        octal(&mut out, byte);
                    }
                }
                c => out.push(c),
            }
        }
        for &byte in chunk.invalid() {
            octal(&mut out, byte);
        }
    }
    out.push('"');
    out.into()
}

/// Changes as the rows of Ely's list.
fn rows(changes: &[Change]) -> Vec<Changed> {
    let row = |change: &Change| Changed {
        path: shown(&change.path),
        status: change.status,
        lines: None,
    };
    changes.iter().take(LISTED).map(row).collect()
}

/// How many changes the list holds against how many there are, if it leaves some out.
fn cut(repo: &Repo) -> Option<(usize, usize)> {
    let sections = [
        &repo.conflicted,
        &repo.staged,
        &repo.unstaged,
        &repo.untracked,
    ];
    let total: usize = sections.iter().map(|changes| changes.len()).sum();
    let listed: usize = sections
        .iter()
        .map(|changes| changes.len().min(LISTED))
        .sum();
    (listed < total).then_some((listed, total))
}

/// Whether a change names a folder, as git ends it with a slash.
fn is_folder(path: &Path) -> bool {
    path.as_os_str().as_encoded_bytes().ends_with(b"/")
}

/// The message key saying what discarding `change` costs.
fn loss(change: &Change, section: ChangeSection) -> &'static str {
    match (section, change.status) {
        (ChangeSection::Untracked, _) if is_folder(&change.path) => "git.loss.folder",
        (ChangeSection::Untracked, _) | (_, GitStatus::Added | GitStatus::Renamed) => {
            "git.loss.deleted"
        }
        _ => "git.loss.changes",
    }
}

/// HEAD's name: its branch, or its commit's short id while detached.
fn head(repo: &Repo) -> (IconName, String) {
    match (&repo.branch, &repo.commit) {
        (Some(branch), _) => (IconName::GitBranch, branch.clone()),
        (None, Some(commit)) => (
            IconName::GitCommitHorizontal,
            commit.chars().take(7).collect(),
        ),
        (None, None) => (IconName::GitCommitHorizontal, "HEAD".into()),
    }
}

/// A small heading, as the changes list draws its own.
fn heading(text: SharedString, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .px_2()
        .py_1()
        .text_size(theme.text_size(TextSize::Xs))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.colors.fg_muted)
        .child(text)
}

impl GitPanel {
    /// The question before a discard, which cannot be taken back.
    fn discard_dialog(&self, cx: &Context<Self>) -> Option<ConfirmDialog> {
        let (change, section) = self.discarding.clone()?;
        let (close, confirm) = (cx.entity(), cx.entity());
        let path = shown(&change.path);
        let title = crate::i18n::text(cx, "git.discard_title", &[("path", path.as_ref())]);
        let said = crate::i18n::text(cx, loss(&change, section), &[]).to_string();
        let dialog = ConfirmDialog::new("git-discard", title, said, {
            move |_, cx| {
                close.update(cx, |panel, cx| {
                    panel.discarding = None;
                    cx.notify();
                })
            }
        });
        let confirmed = move |_: &mut Window, cx: &mut App| {
            let change = change.clone();
            confirm.update(cx, |panel, cx| panel.discard(change, section, cx))
        };
        Some(
            dialog
                .confirm(crate::i18n::text(cx, "git.discard_confirm", &[]))
                .destructive()
                .on_confirm(confirmed),
        )
    }
}

impl Render for GitPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().size_full().flex().flex_col().gap_2();
        let (repo, commits) = match &self.state {
            State::Reading => return root,
            State::Outside => {
                let said = crate::i18n::text(cx, "git.outside", &[]);
                return root.child(Caption::new(said).px_2());
            }
            State::Failed(error) => {
                let failed = Alert::new(
                    "git-failed",
                    Severity::Danger,
                    crate::i18n::text(cx, "git.status_failed", &[]),
                );
                return root.child(failed.body(error.clone()));
            }
            State::Read(repo, commits) => (repo, commits.clone()),
        };
        let theme = cx.theme();
        let (icon, name) = head(repo);
        let branch = div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .text_size(theme.text_size(TextSize::Sm))
            .font_weight(FontWeight::MEDIUM)
            .child(Icon::new(icon).color(theme.colors.fg_muted))
            .child(div().min_w_0().child(Ellipsis::new(name.clone())));
        let error = self.error.as_ref().map(|(what, said)| {
            let dismiss = cx.entity();
            Alert::new(
                "git-error",
                Severity::Danger,
                crate::i18n::text(cx, what, &[]),
            )
            .body(said.clone())
            .on_dismiss(move |_, cx| {
                dismiss.update(cx, |panel, cx| {
                    panel.error = None;
                    cx.notify();
                })
            })
        });
        let (act, all, flip, commit) = (cx.entity(), cx.entity(), cx.entity(), cx.entity());
        let changes = ChangesList::new("git-changes", rows(&repo.staged), rows(&repo.unstaged))
            .conflicted(rows(&repo.conflicted))
            .untracked(rows(&repo.untracked))
            .on_action(move |row, section, action, _, cx| {
                act.update(cx, |panel, cx| panel.row_action(row, section, action, cx))
            })
            .on_all(move |stage, _, cx| all.update(cx, |panel, cx| panel.all(stage, cx)));
        let staged = repo.staged.len();
        let message = CommitInput::new("git-commit", &self.subject, &self.body, name, staged)
            .amend(self.amend)
            .on_amend(move |on, _, cx| {
                flip.update(cx, |panel, cx| {
                    log::debug!("git panel: amend {on}");
                    panel.amend = on;
                    cx.notify();
                })
            })
            .on_commit(move |_, cx| commit.update(cx, |panel, cx| panel.commit(cx)));
        let recent = (!commits.is_empty()).then(|| {
            let list = CommitList::new("git-commits", commits);
            div().flex_1().min_h_0().child(list)
        });
        root.child(branch)
            .children(error)
            .child(
                div()
                    .id("git-changes-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(cut(repo).map(|(listed, total)| {
                        let (listed, total) = (listed.to_string(), total.to_string());
                        let said = crate::i18n::text(
                            cx,
                            "git.listed",
                            &[("listed", &listed), ("total", &total)],
                        );
                        Caption::new(said).px_2()
                    }))
                    .child(changes),
            )
            .child(message)
            .children(
                recent
                    .is_some()
                    .then(|| heading(crate::i18n::text(cx, "git.commits", &[]), cx)),
            )
            .children(recent)
            .children(self.discard_dialog(cx))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn a_shown_path_is_one_line_and_one_to_a_path() {
        let show = |path: &str| shown(Path::new(path)).to_string();
        assert_eq!(show("src/main.rs"), "src/main.rs");
        assert_eq!(
            show("notes/"),
            "notes",
            "the list names a folder by its name"
        );
        assert_eq!(show("café/ü.txt"), "café/ü.txt");
        assert_eq!(show("line\nbreak"), "\"line\\nbreak\"");
        assert_eq!(show("a\"b\\c\td"), "\"a\\\"b\\\\c\\td\"");
        assert_eq!(show("bell\u{7}"), "\"bell\\007\"");
        assert_ne!(
            show("\"line\\nbreak\""),
            show("line\nbreak"),
            "a name that spells a quoting reads apart from the name quoted"
        );
        #[cfg(unix)]
        {
            use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
            let raw = Path::new(OsStr::from_bytes(b"bad\xff.txt"));
            assert_eq!(shown(raw).as_ref(), "\"bad\\377.txt\"");
            assert_ne!(shown(raw), shown(Path::new("bad\u{FFFD}.txt")));
        }
    }

    #[test]
    fn a_discard_says_what_it_costs() {
        let change = |path: &str, status| Change {
            path: path.into(),
            from: None,
            status,
        };
        let untracked = ChangeSection::Untracked;
        let folder = change("notes/", GitStatus::Untracked);
        assert_eq!(loss(&folder, untracked), "git.loss.folder");
        assert_eq!(
            loss(&change("a.txt", GitStatus::Untracked), untracked),
            "git.loss.deleted"
        );
        let intended = change("a.txt", GitStatus::Added);
        assert_eq!(loss(&intended, ChangeSection::Unstaged), "git.loss.deleted");
        let edited = change("a.txt", GitStatus::Modified);
        assert_eq!(loss(&edited, ChangeSection::Unstaged), "git.loss.changes");
    }

    #[test]
    fn a_long_section_lists_its_first_files_and_says_so() {
        let changes = |count: usize| -> Vec<Change> {
            let change = |n| Change {
                path: format!("f{n}.txt").into(),
                from: None,
                status: GitStatus::Modified,
            };
            (0..count).map(change).collect()
        };
        let mut repo = Repo {
            top: PathBuf::from("/r"),
            branch: None,
            commit: None,
            conflicted: Vec::new(),
            staged: changes(2),
            unstaged: changes(LISTED),
            untracked: Vec::new(),
        };
        assert_eq!(cut(&repo), None);
        repo.unstaged = changes(LISTED + 100);
        assert_eq!(rows(&repo.unstaged).len(), LISTED);
        assert_eq!(cut(&repo), Some((LISTED + 2, LISTED + 102)));
    }

    #[test]
    fn head_reads_as_its_branch_or_its_commit() {
        let repo = |branch: Option<&str>| Repo {
            top: PathBuf::from("/r"),
            branch: branch.map(Into::into),
            commit: Some("1a2b3c4d5e6f".into()),
            conflicted: Vec::new(),
            staged: Vec::new(),
            unstaged: Vec::new(),
            untracked: Vec::new(),
        };
        assert_eq!(
            head(&repo(Some("main"))),
            (IconName::GitBranch, "main".into())
        );
        let detached = (IconName::GitCommitHorizontal, "1a2b3c4".into());
        assert_eq!(head(&repo(None)), detached);
    }
}
