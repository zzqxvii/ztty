//! A changed file's diff shown in a tab.

use std::{path::PathBuf, rc::Rc};

use ely_gpui_component::{
    git::{ChangeSection, Stretch},
    typography::Caption,
};
use gpui::{
    AppContext, Context, IntoElement, ParentElement, Render, SharedString, Styled, Task, Window,
    div,
};

use crate::{
    components::diff::{Layout, Viewer},
    git::Change,
    projects,
};

/// Why a diff shows no lines.
enum Note {
    NoLines,
    Binary,
    Failed(String),
}

/// A change's diff, read from git when the tab opens and again after each action.
pub struct DiffView {
    pub path: PathBuf,
    top: PathBuf,
    change: Change,
    section: ChangeSection,
    lines: Option<Result<Rc<Vec<Stretch>>, Note>>,
    layout: Layout,
    _reading: Option<Task<()>>,
}

impl DiffView {
    pub fn new(
        top: PathBuf,
        change: Change,
        section: ChangeSection,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            path: change.path.clone(),
            top,
            change,
            section,
            lines: None,
            layout: Layout::Unified,
            _reading: None,
        };
        view.reload(cx);
        view
    }

    /// What the tab calls it.
    pub fn name(&self) -> String {
        projects::name(&self.path)
    }

    /// Reads the diff again, off the main thread.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let (top, change, section) = (self.top.clone(), self.change.clone(), self.section);
        let read = cx.background_spawn(async move { crate::git::diff(&top, &change, section) });
        self._reading = Some(cx.spawn(async move |this, cx| {
            let read = read.await;
            let _ = this.update(cx, |this, cx| {
                this.lines = Some(match read {
                    Ok(Some(lines)) if lines.is_empty() => Err(Note::NoLines),
                    Ok(Some(lines)) => Ok(Rc::new(lines)),
                    Ok(None) => Err(Note::Binary),
                    Err(error) => {
                        log::error!("diff: {} is not read: {error:#}", this.path.display());
                        Err(Note::Failed(format!("{error:#}")))
                    }
                });
                cx.notify();
            });
        }));
    }
}

impl Render for DiffView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let note = |text: SharedString| div().p_3().child(Caption::new(text)).into_any_element();
        let Some(lines) = &self.lines else {
            return note(crate::i18n::text(cx, "file.reading", &[]));
        };
        let lines = match lines {
            Ok(lines) => lines.clone(),
            Err(Note::NoLines) => return note(crate::i18n::text(cx, "diff.no_lines", &[])),
            Err(Note::Binary) => return note(crate::i18n::text(cx, "diff.binary", &[])),
            Err(Note::Failed(reason)) => return note(reason.clone().into()),
        };
        let laid_out = cx.entity();
        let name = self.name();
        Viewer::new(format!("diff:{name}"), name.into(), lines, self.layout)
            .on_layout(move |layout, _, cx| {
                laid_out.update(cx, |this, cx| {
                    this.layout = layout;
                    cx.notify();
                });
            })
            .into_any_element()
    }
}
