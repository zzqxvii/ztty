//! The left column: the projects, each over its worktrees.

use std::{path::Path, time::Instant};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton},
    data_display::Badge,
    feedback::Alert,
    primitives::{Icon, IconName, Severity, Tooltip},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize, Theme},
};
use gpui::{
    AnyElement, App, Entity, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Role, StatefulInteractiveElement, Styled, div, prelude::*, relative,
    transparent_black,
};

use super::{ProjectView, Shown, Workspace};
use crate::{
    i18n::text,
    projects,
    sessions::{self, mark_of},
    settings::window::OpenSettings,
    worktree::WorktreeView,
};

/// The left column's body: project rows, each over its worktrees, then Add project.
pub(super) fn body(
    workspace: &Entity<Workspace>,
    this: &Workspace,
    home: Option<&Path>,
    now: Instant,
    cx: &App,
) -> AnyElement {
    let mut rows: Vec<AnyElement> = Vec::new();
    for (index, project) in this.projects.iter().enumerate() {
        let holds = this.shown.project == index;
        rows.push(project_row(workspace, index, project, holds, home, cx));
        if project.collapsed {
            continue;
        }
        for at in 0..project.worktrees.len() {
            rows.push(worktree_row(
                workspace,
                this,
                Shown {
                    project: index,
                    worktree: at,
                },
                project,
                home,
                now,
                cx,
            ));
        }
    }
    let list = div().flex().flex_col().gap_0p5().children(rows);
    let failed = this.failed.clone().map(|said| {
        let dismiss = workspace.clone();
        Alert::new(
            "worktree-failed",
            Severity::Danger,
            text(cx, "workspace.worktree_failed", &[]),
        )
        .body(said)
        .on_dismiss(move |_, cx| {
            dismiss.update(cx, |this, cx| {
                this.failed = None;
                cx.notify();
            })
        })
    });
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_1()
        .children(failed)
        .child(
            div()
                .id("project-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(list),
        )
        .child(foot(workspace, cx))
        .into_any_element()
}

/// The column's foot: the gear that opens Settings where no menu bar does, then Add project.
fn foot(workspace: &Entity<Workspace>, cx: &App) -> AnyElement {
    let gear = (!cfg!(target_os = "macos")).then(|| {
        IconButton::new("open-settings", IconName::Settings)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .tooltip(text(cx, "workspace.settings", &[]))
            .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenSettings), cx))
    });
    div()
        .flex()
        .items_center()
        .gap_1()
        .children(gear)
        .child(div().flex_1().min_w_0().child(add_button(workspace, cx)))
        .into_any_element()
}

/// A project's row: a folder that opens or closes its worktrees, its name, and the button that removes it.
fn project_row(
    workspace: &Entity<Workspace>,
    index: usize,
    project: &ProjectView,
    holds: bool,
    home: Option<&Path>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let folder = Icon::new(if project.collapsed {
        IconName::Folder
    } else {
        IconName::FolderOpen
    })
    .size(IconSize::Sm)
    .color(colors.fg_muted);
    let name = div()
        .text_size(theme.text_size(TextSize::Base))
        .line_height(relative(1.4))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(if project.saved {
            colors.fg
        } else {
            colors.fg_muted
        })
        .truncate()
        .child(project.name.clone());
    let error = project.error.clone().map(|error| {
        div()
            .text_size(theme.text_size(TextSize::Sm))
            .line_height(relative(1.4))
            .text_color(colors.danger)
            .truncate()
            .child(error)
    });
    let pressed = workspace.clone();
    let remove = workspace.clone();
    let make = workspace.clone();
    // Only a repository git named can take a worktree.
    let new_worktree = project.repository.then(|| {
        IconButton::new(("new-worktree", index), IconName::Plus)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .tooltip(text(cx, "workspace.new_worktree", &[]))
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                make.update(cx, |this, cx| this.ask_worktree(index, window, cx))
            })
    });
    let row = div()
        .id(("project-row", index))
        .role(Role::ListItem)
        .aria_label(project.name.clone())
        .flex()
        .items_center()
        .gap_1p5()
        .min_h(theme.control_height(ControlSize::Md))
        .px_1p5()
        .rounded(theme.radius(Radius::Sm))
        .cursor_pointer()
        .when(holds && project.collapsed, |row| row.bg(colors.active))
        .when(!(holds && project.collapsed), |row| {
            row.hover(|style| style.bg(colors.hover))
        })
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .on_click(move |_, _, cx| {
            pressed.update(cx, |this, cx| {
                if let Some(project) = this.projects.get_mut(index) {
                    project.collapsed = !project.collapsed;
                    let folded = if project.collapsed {
                        "folds"
                    } else {
                        "unfolds"
                    };
                    log::debug!("app: project {} {folded}", project.name);
                    cx.notify();
                }
            })
        })
        .child(folder)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(name)
                .children(error),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_0p5()
                .opacity(0.)
                .group_hover(format!("project-{index}"), |style| style.opacity(1.))
                .children(new_worktree)
                .child(
                    IconButton::new(("remove-project", index), IconName::X)
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .tooltip(text(cx, "workspace.remove_project", &[]))
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            remove.update(cx, |this, cx| {
                                this.removing = Some(index);
                                cx.notify();
                            })
                        }),
                ),
        );
    div()
        .id(("project", index))
        .group(format!("project-{index}"))
        .when(index > 0, |row| row.mt_3())
        .hoverable_tooltip(Tooltip::text(sessions::shown(&project.path, home)))
        .child(row)
        .into_any_element()
}

/// A worktree's card, stepped in under its project: its mark and name over its folder, and what it is doing.
fn worktree_row(
    workspace: &Entity<Workspace>,
    this: &Workspace,
    shown: Shown,
    owner: &ProjectView,
    home: Option<&Path>,
    now: Instant,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let (project, at) = (shown.project, shown.worktree);
    let is_shown = this.shown == shown;
    let worktree = &owner.worktrees[at];
    let pressing = worktree.pressing();
    let mark = pressing
        .and_then(|session| mark_of(session.look, &session.terminal, now, IconSize::Md, cx));
    let resting = div()
        .size(theme.icon_size(IconSize::Md))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(theme.status_dot())
                .rounded_full()
                .bg(colors.fg_subtle),
        )
        .into_any_element();
    let alert = worktree.bell() || worktree.waiting();
    // Only a project of several says which checkout is the main one.
    let main = worktree.default && owner.worktrees.len() > 1;
    let title = div()
        .flex_1()
        .min_w_0()
        .text_size(theme.text_size(TextSize::Base))
        .line_height(relative(1.4))
        .font_weight(if alert {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::MEDIUM
        })
        .text_color(colors.fg)
        .truncate()
        .child(worktree.name());
    let pressed = workspace.clone();
    let card = div()
        .id(format!("worktree-{project}-{at}"))
        .role(Role::ListItem)
        .aria_label(worktree.name())
        .aria_selected(is_shown)
        .flex()
        .flex_col()
        .px_2()
        .py_1()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(if is_shown {
            colors.border
        } else {
            transparent_black()
        })
        .when(is_shown, |card| card.bg(colors.active))
        .when(!is_shown, |card| card.hover(|style| style.bg(colors.hover)))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .on_click(move |_, window, cx| {
            pressed.update(cx, |this, cx| this.select_worktree(project, at, window, cx))
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(mark.unwrap_or(resting))
                .child(title)
                .children(main.then(|| Badge::new(text(cx, "workspace.default_badge", &[])))),
        )
        .child(line(subtitle(worktree, home), colors.fg_muted, theme));
    match pressing.and_then(|session| activity(session, cx)) {
        Some(said) => card
            .child(line(said, colors.fg_subtle, theme))
            .into_any_element(),
        None => card.into_any_element(),
    }
}

/// A line under a card's title, stepped in under it.
fn line(text: String, color: Hsla, theme: &Theme) -> AnyElement {
    div()
        .pl_6()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_size(theme.text_size(TextSize::Sm))
        .line_height(relative(1.4))
        .text_color(color)
        .child(text)
        .into_any_element()
}

/// The line under a worktree's name: its folder, or the path that names it while it has no branch.
fn subtitle(worktree: &WorktreeView, home: Option<&Path>) -> String {
    if worktree.branch.is_some() {
        projects::name(&worktree.path)
    } else {
        sessions::shown(&worktree.path, home)
    }
}

/// What the worktree's most pressing record says it is doing, as a card's last line.
fn activity(session: &sessions::Session, cx: &App) -> Option<String> {
    let terminal = session.terminal.read(cx);
    let records = terminal.records();
    let (id, record) = records.shown()?;
    sessions::glance(records, id, record)
}

/// The button that adds a project through the platform's folder picker.
pub(super) fn add_button(workspace: &Entity<Workspace>, cx: &App) -> AnyElement {
    let workspace = workspace.clone();
    Button::new("add-project", text(cx, "workspace.add_project", &[]))
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
        .icon(IconName::FolderPlus)
        .full_width()
        .on_click(move |_, _, cx| workspace.update(cx, |this, cx| this.add_project(cx)))
        .into_any_element()
}
