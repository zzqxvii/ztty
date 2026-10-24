use std::{
    path::{MAIN_SEPARATOR, Path, PathBuf},
    time::Instant,
};

use ely_gpui_component::{
    i18n::I18n,
    primitives::Tooltip,
    theme::{ActiveTheme, IconSize},
};
use gpui::{AnyElement, App, Entity, SharedString, Subscription, div, prelude::*, rems};

use crate::{
    term::{Kind, Record, Records, State, TerminalView},
    workspace::TITLE,
};

pub mod orb;

use orb::Look;

/// A program on its own terminal.
pub struct Session {
    pub terminal: Entity<TerminalView>,
    /// The program's title; empty while it names none.
    pub title: String,
    /// The folder it reports, else the one it started in.
    pub folder: PathBuf,
    /// The program it runs, as its path; empty for the login shell.
    pub shell: String,
    /// A bell rang while it was hidden.
    pub bell: bool,
    /// It started waiting on the user while hidden.
    pub waiting: bool,
    /// The look its mark shows, and since when.
    pub look: Option<(Look, Instant)>,
    pub _events: Subscription,
}

impl Session {
    /// What it is called: its program's title, else the app's name.
    pub fn named(&self) -> &str {
        if self.title.is_empty() {
            TITLE
        } else {
            &self.title
        }
    }

    /// The look its mark takes from its terminal now.
    pub fn looks(&self, cx: &App) -> Option<Look> {
        let view = self.terminal.read(cx);
        let failed = view.ended().is_some_and(|code| code != 0);
        look(view.records().shown().map(|(_, record)| record), failed)
    }

    /// Notes the look its mark takes `now`; true while the mark moves.
    pub fn note(&mut self, now: Instant, cx: &App) -> bool {
        let shows = self.looks(cx);
        if self.look.map(|(shown, _)| shown) != shows {
            self.look = shows.map(|shows| (shows, now));
        }
        self.look
            .is_some_and(|(look, since)| orb::moves(look, now.saturating_duration_since(since)))
    }
}

/// The most records hover lists, and the most characters it shows of each part.
const LISTED: usize = 10;
const SHORT: usize = 120;

/// `folder` as a row shows it: the home folder as `~`, on one line.
pub fn shown(folder: &Path, home: Option<&Path>) -> String {
    let shown = match home.and_then(|home| folder.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~{MAIN_SEPARATOR}{}", rest.display()),
        None => folder.display().to_string(),
    };
    one_line(&shown)
}

/// `text` on one line, as gpui shapes labels: each control character as its picture, `␊` for a newline.
pub fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\0'..='\x1f' => char::from_u32(0x2400 + c as u32).expect("a control picture"),
            '\x7f' => '\u{2421}',
            c => c,
        })
        .collect()
}

/// The session shown once session `closed` of `left + 1` is gone, `active` having been shown.
pub fn after_close(active: usize, closed: usize, left: usize) -> usize {
    if closed < active || active == left {
        active - 1
    } else {
        active
    }
}

/// What a mark shows: the most pressing record's state, else a failed last command.
pub fn look(record: Option<&Record>, failed: bool) -> Option<Look> {
    Some(
        match (record.map(|record| (record.state, record.progress)), failed) {
            (Some((State::Blocked, _)), _) => Look::Waiting,
            (Some((State::Error, _)), _) => Look::Error,
            (Some((State::Working, progress)), _) => Look::Working(progress),
            (Some((State::Done, _)), _) => Look::Done,
            (_, true) => Look::Error,
            (Some((State::Idle, _)), false) => Look::Idle,
            (None, false) => return None,
        },
    )
}

/// The mark of `look` in a box `size` wide, its dots in its state's color; hover lists the terminal's records.
pub fn mark_of(
    look: Option<(Look, Instant)>,
    terminal: &Entity<TerminalView>,
    now: Instant,
    size: IconSize,
    cx: &App,
) -> Option<AnyElement> {
    let (look, since) = look?;
    let theme = cx.theme();
    let colors = &theme.colors;
    let color = match look {
        Look::Working(_) => colors.info,
        Look::Waiting => colors.warning,
        Look::Error => colors.danger,
        Look::Done => colors.success,
        Look::Idle => colors.fg_muted,
    };
    let pid = terminal.read(cx).pid() as usize;
    // Hover reads the records as they stand, so an open one follows them, and scrolls within the window.
    let watched = terminal.downgrade();
    let tip = Tooltip::rich(move |window, cx| {
        let lines = watched.upgrade().map_or_else(Vec::new, |terminal| {
            let view = terminal.read(cx);
            hover(
                view.records(),
                view.ended().filter(|&code| code != 0),
                cx.global::<I18n>(),
            )
        });
        let room = window.viewport_size().height - rems(3.).to_pixels(window.rem_size());
        div()
            .id("session-hover")
            .max_h(room)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .children(lines)
            .into_any_element()
    });
    // Every mark takes a box of its size, so the title stays put as the state changes.
    Some(
        div()
            .id(("session-status", pid))
            .size(theme.icon_size(size))
            .flex()
            .items_center()
            .justify_center()
            .hoverable_tooltip(tip)
            .child(orb::lattice(look, color, now, since))
            .into_any_element(),
    )
}

/// What hover shows: a line for each record, ten at most, then a failed command's status.
pub fn hover(records: &Records, failed: Option<i32>, i18n: &I18n) -> Vec<SharedString> {
    let mut lines: Vec<SharedString> = records
        .iter()
        .take(LISTED)
        .map(|(id, record)| line(records, id, record, i18n).into())
        .collect();
    let more = records.iter().count().saturating_sub(LISTED);
    if more > 0 {
        lines.push(i18n.text("sessions.more", &[("count", &more.to_string())]));
    }
    if let Some(code) = failed {
        let code = code.to_string();
        lines.push(i18n.text("sessions.last_exit", &[("code", &code)]));
    }
    lines
}

/// A record as hover and notifications say it: its state, then its app, title and message.
pub fn line(records: &Records, id: &str, record: &Record, i18n: &I18n) -> String {
    let label = label(record, i18n);
    match said(records, id, record) {
        Some(said) => i18n
            .text("sessions.record", &[("label", &label), ("said", &said)])
            .to_string(),
        None => label,
    }
}

/// What a record says of itself, as a row's line under the worktree's name: its message, else its title, else its app.
pub fn glance(records: &Records, id: &str, record: &Record) -> Option<String> {
    [
        record.msg.as_deref(),
        record.title.as_deref(),
        records.app(id),
    ]
    .into_iter()
    .flatten()
    .map(plain)
    .find(|text| !text.is_empty())
}

/// A record's app, title and message, on one line, each as plain text.
fn said(records: &Records, id: &str, record: &Record) -> Option<String> {
    let parts: Vec<String> = [
        records.app(id),
        record.title.as_deref(),
        record.msg.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(plain)
    .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// A record's state, as hover names it.
fn label(record: &Record, i18n: &I18n) -> String {
    let key = match (record.state, record.kind) {
        (State::Idle, ..) => "sessions.idle",
        (State::Working, _) if record.progress.is_some() => {
            let percent = record.progress.unwrap_or_default().to_string();
            return i18n
                .text("sessions.working_percent", &[("percent", &percent)])
                .to_string();
        }
        (State::Working, _) => "sessions.working",
        (State::Blocked, Some(Kind::Permission)) => "sessions.waiting_permission",
        (State::Blocked, Some(Kind::Question)) => "sessions.waiting_answer",
        (State::Blocked, Some(Kind::Auth)) => "sessions.waiting_login",
        (State::Blocked, None) => "sessions.waiting",
        (State::Done, _) => "sessions.done",
        (State::Error, _) => "sessions.error",
    };
    i18n.text(key, &[]).to_string()
}

/// A program's text as hover shows it: on one line, 120 characters at most, without what could disguise it.
pub fn plain(text: &str) -> String {
    let kept: String = text.chars().filter(|&c| !invisible(c)).collect();
    let line = one_line(&kept);
    match line.char_indices().nth(SHORT) {
        Some((at, _)) => format!("{}…", &line[..at]),
        None => line,
    }
}

/// C1 controls, direction controls and invisible formatting; joiners stay, as scripts and emoji need them.
fn invisible(c: char) -> bool {
    matches!(
        c,
        '\u{80}'..='\u{9f}'
            | '\u{ad}'
            | '\u{61c}'
            | '\u{180e}'
            | '\u{200b}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{e0000}'..='\u{e007f}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_under_home_read_from_a_tilde() {
        let home = PathBuf::from("/Users/ztty");
        let shown = |folder: &str| shown(Path::new(folder), Some(&home));
        assert_eq!(shown("/Users/ztty"), "~");
        assert_eq!(
            shown("/Users/ztty/src/app"),
            format!("~{MAIN_SEPARATOR}src/app")
        );
        assert_eq!(
            shown("/Users/zttyse"),
            "/Users/zttyse",
            "a sibling is no child"
        );
        assert_eq!(shown("/tmp"), "/tmp");
        assert_eq!(
            shown("/Users/ztty/line\nbreak"),
            format!("~{MAIN_SEPARATOR}line␊break")
        );
    }

    #[test]
    fn control_characters_show_as_their_pictures() {
        assert_eq!(one_line("a\nb\tc\rd\0e\u{7f}"), "a␊b␉c␍d␀e␡");
        assert_eq!(one_line("café ✓"), "café ✓", "printable text stays");
    }

    #[test]
    fn closing_shows_the_next_session_or_keeps_the_shown_one() {
        assert_eq!(after_close(2, 0, 3), 1, "one before it went");
        assert_eq!(after_close(1, 1, 3), 1, "it went: the next one");
        assert_eq!(after_close(3, 3, 3), 2, "the last went: the one before");
        assert_eq!(after_close(1, 2, 3), 1, "one after it went");
    }

    #[test]
    fn the_mark_shows_the_most_pressing_look() {
        let record = |state, progress| Record {
            state,
            kind: None,
            progress,
            app: None,
            title: None,
            msg: None,
        };
        let waiting = look(Some(&record(State::Blocked, None)), true);
        assert_eq!(
            waiting,
            Some(Look::Waiting),
            "waiting outranks a failed command"
        );
        let working = look(Some(&record(State::Working, Some(40))), false);
        assert_eq!(working, Some(Look::Working(Some(40))));
        let idle = record(State::Idle, None);
        assert_eq!(
            look(Some(&idle), true),
            Some(Look::Error),
            "a failed command outranks idle"
        );
        assert_eq!(look(Some(&idle), false), Some(Look::Idle));
        assert_eq!(
            look(Some(&record(State::Done, None)), true),
            Some(Look::Done)
        );
        assert_eq!(
            look(Some(&record(State::Error, None)), false),
            Some(Look::Error)
        );
        assert_eq!(look(None, false), None);
    }

    #[test]
    fn hover_shows_a_program_s_text_as_text() {
        assert_eq!(plain("evil\u{202e}txt.exe"), "eviltxt.exe");
        assert_eq!(
            plain("a\u{200b}b\u{feff}c\u{e0041}d\u{85}e\u{2066}f"),
            "abcdef"
        );
        assert_eq!(
            plain("👨\u{200d}💻 two\nlines"),
            "👨\u{200d}💻 two␊lines",
            "joiners stay"
        );
        assert_eq!(plain(&"x".repeat(SHORT)), "x".repeat(SHORT));
        assert_eq!(
            plain(&"é".repeat(SHORT + 1)),
            format!("{}…", "é".repeat(SHORT))
        );
    }

    #[test]
    fn hover_lists_each_record_with_its_app_then_a_failed_command() {
        let record =
            |state: State, kind: Option<Kind>, progress: Option<u8>, texts: [Option<&str>; 3]| {
                let [app, title, msg] = texts.map(|text| text.map(String::from));
                Record {
                    state,
                    kind,
                    progress,
                    app,
                    title,
                    msg,
                }
            };
        let mut records = Records::default();
        records.report(
            String::new(),
            record(
                State::Working,
                None,
                Some(40),
                [Some("deploy"), None, Some("Deploying")],
            ),
            None,
        );
        records.report(
            "eu".into(),
            record(
                State::Blocked,
                Some(Kind::Permission),
                None,
                [None, Some("EU West"), Some("Approve?")],
            ),
            None,
        );
        records.report(
            "eu/db".into(),
            record(State::Done, None, None, [Some("db"), None, None]),
            None,
        );
        let i18n = crate::i18n::english();
        let lines: Vec<String> = hover(&records, Some(2), &i18n)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            lines,
            [
                "Working, 40%: deploy · Deploying",
                "Waiting for permission: deploy · EU West · Approve?",
                "Done: db",
                "Last command exited with 2"
            ]
        );
        for at in 0..LISTED {
            records.report(
                format!("r{at}"),
                record(State::Idle, None, None, [None; 3]),
                None,
            );
        }
        let lines = hover(&records, None, &i18n);
        assert_eq!(lines.len(), LISTED + 1);
        assert_eq!(lines[LISTED].to_string(), "3 more");
    }

    #[test]
    fn a_row_shows_a_records_message_then_its_title_then_its_app() {
        let record = |texts: [Option<&str>; 3]| {
            let [app, title, msg] = texts.map(|text| text.map(String::from));
            Record {
                state: State::Working,
                kind: None,
                progress: None,
                app,
                title,
                msg,
            }
        };
        let mut records = Records::default();
        records.report(
            String::new(),
            record([Some("omp"), Some("Build"), Some("Linking")]),
            None,
        );
        records.report("app".into(), record([Some("omp"), None, None]), None);
        let said = |id: &str| {
            let (id, record) = records.iter().find(|(at, _)| *at == id).expect("a record");
            glance(&records, id, record)
        };
        assert_eq!(said("").as_deref(), Some("Linking"), "the message speaks");
        assert_eq!(said("app").as_deref(), Some("omp"), "then the app");
    }
}
