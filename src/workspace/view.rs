//! How the main window draws its columns.

use std::time::Instant;

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    primitives::{FocusScope, Icon, IconName},
    shell::{ColumnShell, ResizeBorder, ShellColumn},
    theme::{ActiveTheme, ControlSize, IconSize, Platform, Radius},
};
use gpui::{
    App, Context, Div, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels,
    Render, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder, px,
};

use super::widths::Side;
use super::{
    CONTEXT, CloseSession, LastSession, NewSession, NextSession, NextTab, PreviousSession,
    PreviousTab, SelectSession, TogglePanel, ToggleSessions, Workspace, left, tabs,
};
use crate::{
    i18n::text,
    menus,
    settings::{PanelTab, Settings, SettingsStore},
};

/// A header's padding, and the gap between its parts.
const HEADER_PAD: f32 = 12.;
const GAP: f32 = 8.;

/// The width a header leaves its tabs, once its edges, gaps, toggles and the window's own buttons are counted.
fn tab_room(
    window: &Window,
    sides: f32,
    show_sessions: bool,
    show_panel: bool,
    swapped: bool,
    cx: &App,
) -> Pixels {
    let theme = cx.theme();
    let rem = window.rem_size();
    let button = f32::from(theme.control_height(ControlSize::Sm).to_pixels(rem));
    let mac = theme.platform == Platform::Mac;
    // The middle column leads or ends the window while the side column on that side hides.
    let middle_first = if swapped { !show_panel } else { !show_sessions };
    let middle_last = if swapped { !show_sessions } else { !show_panel };
    // The window's own buttons end the last column, off macOS.
    let trail = match (middle_last, theme.platform) {
        (false, _) | (true, Platform::Mac) => 0.,
        (true, Platform::Windows) => 3. * f32::from(theme.caption_button_width().to_pixels(rem)),
        (true, Platform::Linux) => 3. * button + 2. * GAP,
    };
    // A first column on macOS leaves the system's traffic lights their room.
    let lead = if middle_first && mac && !window.is_fullscreen() {
        f32::from(theme.traffic_light_inset().to_pixels(rem))
    } else {
        HEADER_PAD
    };
    let flush = trail > 0. && theme.platform == Platform::Windows;
    let mut room = f32::from(window.viewport_size().width) - sides - lead - trail;
    if !flush {
        room -= HEADER_PAD;
    }
    if trail > 0. {
        room -= GAP;
    }
    // The items row's two gaps, then the toggles.
    room -= 2. * GAP + button;
    if !show_sessions {
        room -= button + GAP;
    }
    // The strip's own new-tab button.
    room -= button + GAP;
    px(room.max(0.))
}

/// Shows Git or Files in the right column, as the settings keep it.
fn show_pane(panel: PanelTab, cx: &mut App) {
    if SettingsStore::get(cx).panel == panel {
        return;
    }
    match SettingsStore::update(cx, |settings| settings.panel = panel) {
        Ok(()) => log::info!("app: the right column shows {panel:?}"),
        Err(error) => log::error!("app: the right column stays: {error:#}"),
    }
}

/// The right column's two views as quiet icons at its foot: the shown one in the text's color, the other's muted.
fn pane_tabs(shown: PanelTab, cx: &App) -> Div {
    let theme = cx.theme();
    let colors = &theme.colors;
    let side = theme.control_height(ControlSize::Sm);
    let tab = |panel: PanelTab, id: &'static str, icon: IconName, key: &'static str| {
        let on = panel == shown;
        let fg = if on { colors.fg } else { colors.fg_muted };
        div()
            .id(id)
            .role(Role::RadioButton)
            .aria_toggled(on.into())
            .aria_label(text(cx, key, &[]))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .w(side)
            .h(side)
            .rounded(theme.radius(Radius::Sm))
            .cursor_pointer()
            .hover(|style| style.bg(colors.hover))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(move |_, _, cx| show_pane(panel, cx))
            .child(Icon::new(icon).size(IconSize::Sm).color(fg))
    };
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(tab(
            PanelTab::Files,
            "pane-files",
            IconName::FolderOpen,
            "panel.files",
        ))
        .child(tab(
            PanelTab::Git,
            "pane-git",
            IconName::GitBranch,
            "panel.git",
        ))
}

/// A header button that hides or shows a column, saved.
fn column_toggle(
    workspace: &Entity<Workspace>,
    (id, icon): (&'static str, IconName),
    tip: SharedString,
    flip: fn(&mut Settings),
) -> IconButton {
    let workspace = workspace.clone();
    IconButton::new(id, icon)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
        .tooltip(tip)
        .on_click(move |_, _, cx| workspace.update(cx, |this, cx| this.toggle(flip, cx)))
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.background.fit(window);
        if let Some(shown) = self.pending.take() {
            self.select_worktree(shown.project, shown.worktree, window, cx);
        }
        let now = Instant::now();
        self.tick(now, cx);
        let settings = SettingsStore::get(cx);
        let sides = self.sides(settings);
        let (show_sessions, show_panel, tab) =
            (settings.show_sessions, settings.show_panel, settings.panel);
        let workspace = cx.entity();
        // Each toggle names what a press does, in the language the settings hold.
        let projects_tip = if show_sessions {
            text(cx, "workspace.hide_projects", &[])
        } else {
            text(cx, "workspace.show_projects", &[])
        };
        let panel_tip = if show_panel {
            text(cx, "workspace.hide_panel", &[])
        } else {
            text(cx, "workspace.show_panel", &[])
        };
        // Each side column keeps its toggle at the window's edge it holds.
        let swapped = settings.swap_panels;
        let (projects_icon, panel_icon) = if swapped {
            (IconName::PanelRight, IconName::PanelLeft)
        } else {
            (IconName::PanelLeft, IconName::PanelRight)
        };
        let projects_toggle = || {
            column_toggle(
                &workspace,
                ("toggle-sessions", projects_icon),
                projects_tip.clone(),
                |settings| settings.show_sessions = !settings.show_sessions,
            )
        };
        let panel_toggle = || {
            column_toggle(
                &workspace,
                ("toggle-panel", panel_icon),
                panel_tip.clone(),
                |settings| settings.show_panel = !settings.show_panel,
            )
        };
        let room = tab_room(window, sides, show_sessions, show_panel, swapped, cx);
        // A hidden side column leaves its toggle at its end of the middle's header.
        let mut middle = ShellColumn::new("terminal");
        if swapped {
            if !show_panel {
                middle = middle.leading(panel_toggle());
            }
            if !show_sessions {
                middle = middle.action(projects_toggle());
            }
        } else {
            if !show_sessions {
                middle = middle.leading(projects_toggle());
            }
            if !show_panel {
                middle = middle.action(panel_toggle());
            }
        }
        // The tabs sit in the window's top row, as Orca's do, after the toggles.
        if let Some(strip) = tabs::strip(&workspace, self, room, window, cx) {
            middle = middle.leading(strip);
        }
        let middle_body = tabs::body(self);
        let projects = show_sessions.then(|| {
            let home = dirs::home_dir();
            let body = left::body(&workspace, self, home.as_deref(), now, cx);
            let column = self.side_column(Side::Sessions, cx);
            let column = if swapped {
                column.action(projects_toggle())
            } else {
                column.leading(projects_toggle())
            };
            column.child(
                div()
                    .id("projects")
                    .size_full()
                    .overflow_y_scroll()
                    .p_2()
                    .child(body),
            )
        });
        let panel = show_panel.then(|| {
            // Hidden, not dropped, so each keeps its state.
            let pane = |on: bool| {
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .when(!on, |body| body.hidden())
            };
            // Git's list draws every row, so terminal output must not redraw it.
            let fill = || StyleRefinement::default().size_full();
            let column = self.side_column(Side::Panel, cx);
            let column = if swapped {
                column.leading(panel_toggle())
            } else {
                column.action(panel_toggle())
            };
            column
                .child(pane(tab == PanelTab::Git).child(self.git.clone().cached(fill())))
                .child(pane(tab == PanelTab::Files).child(self.files.clone()))
                // The column's foot, past the panes.
                .child(div().flex_none().px_2().pb_2().child(pane_tabs(tab, cx)))
        });
        // The side columns keep to their edges; the middle holds the rest.
        let (first, last) = if swapped {
            (panel, projects)
        } else {
            (projects, panel)
        };
        let mut shell = ColumnShell::new("workspace").glass();
        if let Some(first) = first {
            shell = shell.column(first);
        }
        shell = shell.column(middle.children(Some(middle_body)));
        if let Some(last) = last {
            shell = shell.column(last);
        }
        // The context sits above the focus root, so its keys work wherever focus is.
        div()
            .size_full()
            .key_context(CONTEXT)
            .map(menus::window_actions)
            .on_action(cx.listener(|this, _: &ToggleSessions, _, cx| {
                this.toggle(
                    |settings| settings.show_sessions = !settings.show_sessions,
                    cx,
                )
            }))
            .on_action(cx.listener(|this, _: &TogglePanel, _, cx| {
                this.toggle(|settings| settings.show_panel = !settings.show_panel, cx)
            }))
            .on_action(
                cx.listener(|this, _: &NewSession, window, cx| this.open_terminal(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &CloseSession, window, cx| this.close_shown_tab(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &NextSession, window, cx| {
                    this.step_worktree(true, window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &PreviousSession, window, cx| {
                this.step_worktree(false, window, cx)
            }))
            .on_action(cx.listener(|this, _: &LastSession, window, cx| {
                let rows = this.rows();
                if let Some(&(project, worktree)) = rows.last() {
                    this.select_worktree(project, worktree, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &NextTab, window, cx| this.step_tab(true, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousTab, window, cx| this.step_tab(false, window, cx)),
            )
            .on_action(cx.listener(|this, select: &SelectSession, window, cx| {
                let rows = this.rows();
                match rows.get(select.0) {
                    Some(&(project, worktree)) => {
                        this.select_worktree(project, worktree, window, cx)
                    }
                    None => cx.propagate(),
                }
            }))
            .child(
                ResizeBorder::new()
                    .when(self.background.see_through(), ResizeBorder::glass)
                    .child(FocusScope::new(&self.focus).root().size_full().child(shell)),
            )
            .children(self.removal(cx))
    }
}
