//! A diff drawn by Ztty: its rows, its washes and its type, over the lines `git::diff` reads.

use std::{ops::Range, rc::Rc};

use ely_gpui_component::{
    buttons::SegmentedControl,
    git::{DiffLine, LineKind, Stretch},
    theme::{ActiveTheme, ControlSize, Palette, Radius},
};
use gpui::{
    App, ElementId, FontWeight, HighlightStyle, Hsla, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, StyledText, Window, div, prelude::*, px, uniform_list,
};

use crate::{i18n::text, settings::SettingsStore};

/// How a diff lays out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Unified,
    Split,
}

/// A row's height against its font size.
const LEADING: f32 = 1.6;

type OnLayout = Rc<dyn Fn(Layout, &mut Window, &mut App)>;

/// A row the viewer draws.
enum Row {
    Header(SharedString),
    Fold(usize),
    Line(DiffLine),
    Pair(Option<DiffLine>, Option<DiffLine>),
}

/// A changed file's diff, in Ztty's rows and Ztty's type.
#[derive(IntoElement)]
pub struct Viewer {
    id: ElementId,
    name: SharedString,
    stretches: Rc<Vec<Stretch>>,
    layout: Layout,
    on_layout: Option<OnLayout>,
}

impl Viewer {
    pub fn new(
        id: impl Into<ElementId>,
        name: SharedString,
        stretches: Rc<Vec<Stretch>>,
        layout: Layout,
    ) -> Self {
        Self {
            id: id.into(),
            name,
            stretches,
            layout,
            on_layout: None,
        }
    }

    pub fn on_layout(mut self, run: impl Fn(Layout, &mut Window, &mut App) + 'static) -> Self {
        self.on_layout = Some(Rc::new(run));
        self
    }
}

impl RenderOnce for Viewer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let family = theme.mono_family.clone();
        let size = px(SettingsStore::get(cx).editor_font_size);
        let height = size * LEADING;
        let rows = Rc::new(rows(&self.stretches, self.layout));
        let digits = digits(&self.stretches);
        let number_width = size * (digits as f32 * 0.62 + 0.6);
        let (added, removed) = stat(&self.stretches);
        let says = |key: &str| text(cx, key, &[]);

        let toggle = SegmentedControl::new(
            (self.id.clone(), "layout"),
            match self.layout {
                Layout::Unified => "unified",
                Layout::Split => "split",
            },
        )
        .size(ControlSize::Sm)
        .segment("unified", says("diff.unified"), None)
        .segment("split", says("diff.split"), None);
        let toggle = match self.on_layout.clone() {
            Some(run) => toggle.on_change(move |key, window, cx| {
                let layout = if key.as_ref() == "split" {
                    Layout::Split
                } else {
                    Layout::Unified
                };
                run(layout, window, cx);
            }),
            None => toggle,
        };

        let header = div()
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_1p5()
            .border_b_1()
            .border_color(colors.border)
            .text_size(theme.text_size(ely_gpui_component::theme::TextSize::Sm))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .child(self.name.clone()),
            )
            .child(div().text_color(colors.success).child(format!("+{added}")))
            .child(div().text_color(colors.danger).child(format!("−{removed}")))
            .child(toggle);

        let number = {
            let colors = colors.clone();
            move |value: Option<usize>| {
                div()
                    .flex_none()
                    .w(number_width)
                    .text_right()
                    .text_color(colors.fg_subtle)
                    .children(value.map(|value| value.to_string()))
            }
        };
        let list = uniform_list(
            (
                self.id.clone(),
                SharedString::from(match self.layout {
                    Layout::Unified => "unified",
                    Layout::Split => "split",
                }),
            ),
            rows.len(),
            move |range, _, cx| {
                let family = family.clone();
                let code = |line: &DiffLine| {
                    let (_, word) = washes(line.kind, &colors);
                    let styles: Vec<(Range<usize>, HighlightStyle)> = line
                        .words
                        .iter()
                        .map(|range| {
                            (
                                range.clone(),
                                HighlightStyle {
                                    background_color: Some(word),
                                    ..HighlightStyle::default()
                                },
                            )
                        })
                        .collect();
                    StyledText::new(line.text.clone()).with_highlights(styles)
                };
                range
                    .map(|ix| match &rows[ix] {
                        Row::Header(header) => div()
                            .w_full()
                            .h(height)
                            .flex()
                            .items_center()
                            .px_2()
                            .gap_2()
                            .rounded(px(2.))
                            .bg(colors.hover)
                            .text_color(colors.fg_muted)
                            .font_family(family.clone())
                            .text_size(size)
                            .whitespace_nowrap()
                            .child(header.clone())
                            .into_any_element(),
                        Row::Fold(count) => {
                            let key = if *count == 1 {
                                "diff.unchanged.one"
                            } else {
                                "diff.unchanged.other"
                            };
                            div()
                                .w_full()
                                .h(height)
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(colors.hover.opacity(0.5))
                                .text_color(colors.fg_subtle)
                                .font_family(family.clone())
                                .text_size(size)
                                .child(text(cx, key, &[("n", &count.to_string())]))
                                .into_any_element()
                        }
                        Row::Line(line) => {
                            let (wash, _) = washes(line.kind, &colors);
                            let sign = match line.kind {
                                LineKind::Same => " ",
                                LineKind::Added => "+",
                                LineKind::Removed => "−",
                            };
                            div()
                                .w_full()
                                .h(height)
                                .flex()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .when_some(wash, |row, wash| row.bg(wash))
                                .text_color(colors.fg)
                                .font_family(family.clone())
                                .text_size(size)
                                .whitespace_nowrap()
                                .child(number(line.old))
                                .child(number(line.new))
                                .child(
                                    div()
                                        .flex_none()
                                        .text_color(colors.fg_subtle)
                                        .child(sign),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .child(code(line)),
                                )
                                .into_any_element()
                        }
                        Row::Pair(left, right) => {
                            let side = |line: Option<&DiffLine>, shown: Option<usize>| {
                                let (wash, _) = match line {
                                    Some(line) => washes(line.kind, &colors),
                                    None => (Some(colors.hover.opacity(0.4)), colors.hover),
                                };
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_1()
                                    .overflow_hidden()
                                    .when_some(wash, |side, wash| side.bg(wash))
                                    .text_color(colors.fg)
                                    .font_family(family.clone())
                                    .text_size(size)
                                    .whitespace_nowrap()
                                    .child(number(shown))
                                    .children(
                                        line.map(|line| div().min_w_0().child(code(line))),
                                    )
                            };
                            div()
                                .w_full()
                                .h(height)
                                .flex()
                                .items_center()
                                .gap_1()
                                .px_2()
                                .child(side(left.as_ref(), left.as_ref().and_then(|line| line.old)))
                                .child(side(
                                    right.as_ref(),
                                    right.as_ref().and_then(|line| line.new),
                                ))
                                .into_any_element()
                        }
                    })
                    .collect::<Vec<_>>()
            },
        )
        .size_full();

        div()
            .size_full()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Sm))
            .child(header)
            .child(div().flex_1().min_h_0().child(list))
    }
}

/// A row's background wash, and the wash of its changed words.
fn washes(kind: LineKind, colors: &Palette) -> (Option<Hsla>, Hsla) {
    match kind {
        LineKind::Same => (None, colors.hover),
        LineKind::Added => (
            Some(colors.success.opacity(0.08)),
            colors.success.opacity(0.25),
        ),
        LineKind::Removed => (
            Some(colors.danger.opacity(0.08)),
            colors.danger.opacity(0.25),
        ),
    }
}

/// The stretches as rows, in the layout asked for.
fn rows(stretches: &[Stretch], layout: Layout) -> Vec<Row> {
    let mut rows = Vec::new();
    for stretch in stretches {
        match stretch {
            Stretch::Hunk { header, lines } => {
                rows.push(Row::Header(header.clone()));
                match layout {
                    Layout::Unified => rows.extend(lines.iter().cloned().map(Row::Line)),
                    Layout::Split => rows.extend(
                        paired(lines)
                            .into_iter()
                            .map(|(left, right)| Row::Pair(left, right)),
                    ),
                }
            }
            Stretch::Folded(lines) => rows.push(Row::Fold(lines.len())),
        }
    }
    rows
}

/// A hunk's lines in pairs: a run of removals against the additions after it.
fn paired(lines: &[DiffLine]) -> Vec<(Option<DiffLine>, Option<DiffLine>)> {
    let mut pairs = Vec::new();
    let mut ix = 0;
    while ix < lines.len() {
        if lines[ix].kind == LineKind::Same {
            let line = lines[ix].clone();
            pairs.push((Some(line.clone()), Some(line)));
            ix += 1;
            continue;
        }
        let removed_at = ix;
        while ix < lines.len() && lines[ix].kind == LineKind::Removed {
            ix += 1;
        }
        let removed = ix - removed_at;
        let added_at = ix;
        while ix < lines.len() && lines[ix].kind == LineKind::Added {
            ix += 1;
        }
        let added = ix - added_at;
        for at in 0..removed.max(added) {
            pairs.push((
                (at < removed).then(|| lines[removed_at + at].clone()),
                (at < added).then(|| lines[added_at + at].clone()),
            ));
        }
    }
    pairs
}

/// The widest line number, in digits.
fn digits(stretches: &[Stretch]) -> usize {
    stretches
        .iter()
        .flat_map(|stretch| match stretch {
            Stretch::Hunk { lines, .. } | Stretch::Folded(lines) => lines.iter(),
        })
        .flat_map(|line| [line.old, line.new])
        .flatten()
        .max()
        .unwrap_or(1)
        .to_string()
        .len()
        .max(2)
}

/// How many lines a diff adds and removes.
fn stat(stretches: &[Stretch]) -> (usize, usize) {
    stretches
        .iter()
        .filter_map(|stretch| match stretch {
            Stretch::Hunk { lines, .. } => Some(lines),
            Stretch::Folded(_) => None,
        })
        .flatten()
        .fold((0, 0), |(added, removed), line| match line.kind {
            LineKind::Added => (added + 1, removed),
            LineKind::Removed => (added, removed + 1),
            LineKind::Same => (added, removed),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::SharedString;

    fn line(kind: LineKind, old: Option<usize>, new: Option<usize>, text: &str) -> DiffLine {
        DiffLine {
            kind,
            old,
            new,
            text: SharedString::from(text),
            words: Vec::new(),
        }
    }

    #[test]
    fn a_run_of_removals_pairs_with_the_additions_after_it() {
        let lines = vec![
            line(LineKind::Same, Some(1), Some(1), "a"),
            line(LineKind::Removed, Some(2), None, "b"),
            line(LineKind::Removed, Some(3), None, "c"),
            line(LineKind::Added, None, Some(2), "d"),
            line(LineKind::Same, Some(4), Some(3), "e"),
        ];
        let pairs = paired(&lines);
        assert_eq!(pairs.len(), 4);
        assert_eq!(pairs[1].0.as_ref().map(|line| line.text.clone()), Some("b".into()));
        assert_eq!(pairs[1].1.as_ref().map(|line| line.text.clone()), Some("d".into()));
        assert!(pairs[2].0.is_some() && pairs[2].1.is_none());
        assert_eq!(pairs[3].0.as_ref().map(|line| line.text.clone()), Some("e".into()));
    }

    #[test]
    fn a_folded_stretch_is_one_row() {
        let stretches = vec![Stretch::Folded(vec![
            line(LineKind::Same, Some(1), Some(1), "a"),
            line(LineKind::Same, Some(2), Some(2), "b"),
        ])];
        assert_eq!(rows(&stretches, Layout::Unified).len(), 1);
        assert_eq!(stat(&stretches), (0, 0));
    }
}
