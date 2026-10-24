use std::path::{Path, PathBuf};

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    git::ChangeSection,
    primitives::{DragGhost, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};
use gpui::{
    AnyElement, App, AppContext, Context, Entity, EntityId, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, Role, ScrollHandle, StatefulInteractiveElement, Styled,
    Window, canvas, div, prelude::FluentBuilder,
};

use super::Workspace;
use crate::{git::Change, i18n::text, worktree::Tab};

impl Workspace {
    /// Shows the tab `key` names, as the tab strip's press does.
    pub fn select_tab_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.tab_at(key) else {
            return;
        };
        self.select_tab(at, window, cx);
    }

    /// Closes the tab `key` names, as its close button asks.
    pub fn close_tab_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.tab_at(key) else {
            return;
        };
        let Some(tab) = self.worktree().and_then(|view| view.tabs.get(at)) else {
            return;
        };
        let (terminal, path) = (tab.terminal(), tab.path().map(Path::to_path_buf));
        let (project, worktree) = (self.shown.project, self.shown.worktree);
        match (terminal, path) {
            (Some(id), _) => self.close_terminal(project, worktree, id, window, cx),
            (None, Some(path)) => self.close_file(project, worktree, at, &path, window, cx),
            (None, None) => {}
        }
    }

    /// Shows the diff of `change` in a tab of the shown worktree, in a tab of its own or the one it has.
    pub fn open_diff(
        &mut self,
        top: PathBuf,
        change: Change,
        section: ChangeSection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = change.path.clone();
        let known = self
            .worktree()
            .is_some_and(|view| view.diff(&path).is_some());
        log::info!("git: the diff of {} opens in a tab", path.display());
        if !known {
            let view = cx.new(|cx| crate::diff::DiffView::new(top, change, section, cx));
            if let Some(worktree) = self.worktree_mut() {
                worktree.open_diff(path, view);
            }
        } else if let Some(at) = self.worktree().and_then(|view| {
            view.tabs
                .iter()
                .position(|tab| tab.key() == format!("diff:{}", path.display()))
        }) {
            self.worktree_mut().expect("a worktree").select(at);
        }
        self.after_tab_change(window, cx);
    }

    /// Shows `path` in a tab of the shown worktree, in a tab of its own or the one it has.
    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self.worktree() else {
            return;
        };
        let known = view.file(&path).is_some();
        log::info!("files: {} opens in a tab", path.display());
        if !known {
            let file = cx.new(|cx| crate::file::FileView::new(path.clone(), cx));
            if let Some(view) = self.worktree_mut() {
                view.open_file(path, file);
            }
        } else if let Some(at) = self.worktree().and_then(|view| {
            view.tabs
                .iter()
                .position(|tab| tab.path() == Some(path.as_path()))
        }) {
            self.worktree_mut().expect("a worktree").select(at);
        }
        self.after_tab_change(window, cx);
    }

    /// Moves tab `from` to place `to`, as a drag asks.
    pub fn move_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        let Some(view) = self.worktree_mut() else {
            return;
        };
        view.move_tab(from, to);
        log::debug!("app: tab {} moves to {}", from + 1, to + 1);
        cx.notify();
    }

    /// Shows the next tab, or the one before, round the ends.
    pub fn step_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self.worktree() else {
            return;
        };
        let count = view.tabs.len();
        if count == 0 {
            return;
        }
        let by = if forward { 1 } else { count - 1 };
        let at = (view.active + by) % count;
        self.select_tab(at, window, cx);
    }

    fn tab_at(&self, key: &str) -> Option<usize> {
        self.worktree()?
            .tabs
            .iter()
            .position(|tab| tab.key() == key)
    }
}

/// The shown worktree's tabs, in the middle column's header at the window's top, no wider than `room`, then the button that opens another.
pub(super) fn strip(
    workspace: &Entity<Workspace>,
    this: &Workspace,
    room: Pixels,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    let worktree = this.worktree()?;
    let shown = worktree.shown().map(Tab::key);
    let chosen = worktree
        .tabs
        .iter()
        .position(|tab| Some(tab.key()) == shown);
    // A strip too long for its room scrolls, and brings the shown tab into view.
    let scroll = window
        .use_keyed_state(("tabs-scroll", 0usize), cx, |_, _| ScrollHandle::new())
        .read(cx)
        .clone();
    let followed =
        window.use_keyed_state(("tabs-followed", 0usize), cx, |_, _| None::<Option<String>>);
    let follow =
        (followed.read(cx).as_ref() != Some(&shown)).then(|| (followed.clone(), shown.clone()));
    let tabs: Vec<AnyElement> = worktree
        .tabs
        .iter()
        .enumerate()
        .map(|(at, tab)| {
            let icon = match tab {
                Tab::Terminal(_) => IconName::FileTerminal,
                Tab::File { .. } => IconName::FileText,
                Tab::Diff { .. } => IconName::GitBranch,
            };
            let key = tab.key();
            let on = shown.as_deref() == Some(key.as_str());
            tab_cell(workspace, cx, at, key, worktree.title_of(tab), icon, on)
        })
        .collect();
    let opened = workspace.clone();
    Some(
        div()
            .id("tab-strip")
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .h_full()
            .child(
                div()
                    .id("tabs")
                    .flex()
                    .flex_none()
                    .items_center()
                    .h_full()
                    .max_w(room)
                    .overflow_x_scroll()
                    .track_scroll(&scroll)
                    .children(tabs)
                    .children(follow.map(|(followed, selected)| {
                        let scroll = scroll.clone();
                        canvas(
                            move |_, window, cx| {
                                if scroll.bounds().size.width <= Pixels::ZERO {
                                    return;
                                }
                                followed
                                    .update(cx, |followed, _| *followed = Some(selected.clone()));
                                if let Some(at) = chosen {
                                    scroll.scroll_to_item(at);
                                    window.request_animation_frame();
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                    })),
            )
            .child(
                IconButton::new("new-tab", IconName::Plus)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(text(cx, "workspace.new_tab", &[]))
                    .on_click(move |_, window, cx| {
                        opened.update(cx, |this, cx| this.open_terminal(window, cx))
                    }),
            )
            .into_any_element(),
    )
}

/// What a drag carries: the window's tabs, and the place one set off from.
struct TabDrag {
    owner: EntityId,
    from: usize,
}

/// One tab: its icon and title, a line under the shown one, its close, and a drag to move it.
fn tab_cell(
    workspace: &Entity<Workspace>,
    cx: &App,
    at: usize,
    key: String,
    title: String,
    icon: IconName,
    on: bool,
) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let owner = workspace.entity_id();
    let fg = if on { colors.fg } else { colors.fg_muted };
    let hover = colors.hover;
    let picked = workspace.clone();
    let key_click = key.clone();
    let middle = workspace.clone();
    let key_middle = key.clone();
    let closed = workspace.clone();
    let key_close = key.clone();
    let moved = workspace.clone();
    let dragged = title.clone();
    div()
        .id(("tab", at))
        .role(Role::Tab)
        .aria_selected(on)
        .aria_label(title.clone())
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .gap_1p5()
        .h_full()
        .pl_3()
        .pr_1()
        .max_w(theme.tab_width().1)
        .text_size(theme.text_size(TextSize::Sm))
        .text_color(fg)
        .cursor_pointer()
        .when(on, |tab| {
            tab.child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .h(theme.tab_indicator())
                    .bg(colors.accent),
            )
        })
        .when(!on, |tab| tab.hover(|style| style.text_color(colors.fg)))
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .on_mouse_down(MouseButton::Middle, move |_, window, cx| {
            middle.update(cx, |this, cx| this.close_tab_key(&key_middle, window, cx))
        })
        .on_click(move |_, window, cx| {
            picked.update(cx, |this, cx| this.select_tab_key(&key_click, window, cx))
        })
        .on_drag(TabDrag { owner, from: at }, move |_, _, _, cx| {
            DragGhost::new(dragged.clone().into(), Some(icon), cx)
        })
        .drag_over::<TabDrag>(move |style, drag, _, _| {
            if drag.owner == owner {
                style.bg(hover)
            } else {
                style
            }
        })
        .on_drop(move |drag: &TabDrag, _, cx| {
            if drag.owner == owner && drag.from != at {
                moved.update(cx, |this, cx| this.move_tab(drag.from, at, cx));
            }
        })
        .child(Icon::new(icon).size(IconSize::Sm).color(fg))
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(title),
        )
        .child(
            IconButton::new(("close-tab", at), IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "workspace.close_tab", &[]))
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    closed.update(cx, |this, cx| this.close_tab_key(&key_close, window, cx))
                }),
        )
        .into_any_element()
}

/// The middle column's body: the tab it shows.
pub(super) fn body(this: &Workspace) -> AnyElement {
    let Some(worktree) = this.worktree() else {
        return div().size_full().into_any_element();
    };
    let content: Option<AnyElement> = worktree.shown().and_then(|tab| match tab {
        Tab::Terminal(id) => worktree
            .terminal(*id)
            .map(|session| session.terminal.clone().into_any_element()),
        Tab::File { view, .. } => Some(view.clone().into_any_element()),
        Tab::Diff { view, .. } => Some(view.clone().into_any_element()),
    });
    div()
        .size_full()
        .flex()
        .flex_col()
        .children(content)
        .into_any_element()
}
