use ely_gpui_component::forms::{Copy, Paste, SelectAll};
use gpui::{App, InteractiveElement, KeyBinding, Menu, MenuItem, OsAction, actions};

use crate::{
    i18n::text,
    settings::window::OpenSettings,
    term::{input::Find, zoom},
    workspace::{CloseSession, NewSession, TogglePanel, ToggleSessions},
};

actions!(ztty, [Quit, Bigger, Smaller, ResetSize, Minimize, Zoom]);

/// The PRD's menu bar, in the shown language.
pub fn menus(cx: &App) -> Vec<Menu> {
    let menu = |name: &str, items| Menu {
        name: text(cx, name, &[]),
        items,
        disabled: false,
    };
    let item = |key: &str| text(cx, key, &[]);
    vec![
        menu(
            "menu.app",
            vec![
                MenuItem::action(item("menu.settings"), OpenSettings),
                MenuItem::separator(),
                MenuItem::action(item("menu.quit"), Quit),
            ],
        ),
        menu(
            "menu.shell",
            vec![
                MenuItem::action(item("menu.new_tab"), NewSession),
                MenuItem::action(item("menu.close_tab"), CloseSession),
            ],
        ),
        menu(
            "menu.edit",
            vec![
                MenuItem::os_action(item("menu.copy"), Copy, OsAction::Copy),
                MenuItem::os_action(item("menu.paste"), Paste, OsAction::Paste),
                MenuItem::os_action(item("menu.select_all"), SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::action(item("menu.find"), Find),
            ],
        ),
        menu(
            "menu.view",
            vec![
                MenuItem::action(item("menu.toggle_sessions"), ToggleSessions),
                MenuItem::action(item("menu.toggle_panel"), TogglePanel),
                MenuItem::separator(),
                MenuItem::action(item("menu.bigger"), Bigger),
                MenuItem::action(item("menu.smaller"), Smaller),
                MenuItem::action(item("menu.reset_size"), ResetSize),
            ],
        ),
        // macOS lists the open windows here itself.
        menu(
            "menu.window",
            vec![
                MenuItem::action(item("menu.minimize"), Minimize),
                MenuItem::action(item("menu.zoom"), Zoom),
            ],
        ),
    ]
}

/// Binds app commands before building their menus.
pub fn init(cx: &mut App) {
    let (quit, modifier) = if cfg!(target_os = "macos") {
        ("cmd-q", "cmd")
    } else {
        ("ctrl-shift-q", "ctrl")
    };
    cx.bind_keys([
        KeyBinding::new(quit, Quit, None),
        KeyBinding::new(&format!("{modifier}-="), Bigger, None),
        KeyBinding::new(&format!("{modifier}-+"), Bigger, None),
        KeyBinding::new(&format!("{modifier}--"), Smaller, None),
        KeyBinding::new(&format!("{modifier}-0"), ResetSize, None),
    ]);
    if cfg!(target_os = "macos") {
        cx.bind_keys([KeyBinding::new("cmd-m", Minimize, None)]);
    }
    cx.set_global(zoom::Enlarged::default());
    cx.on_action(|_: &Quit, cx| {
        log::info!("app: quit asked");
        cx.quit();
    });
    cx.on_action(|_: &Bigger, cx| zoom::step(Some(1.0), cx));
    cx.on_action(|_: &Smaller, cx| zoom::step(Some(-1.0), cx));
    cx.on_action(|_: &ResetSize, cx| zoom::step(None, cx));
    install(cx);
}

/// Sets the menu bar in the shown language.
pub fn install(cx: &mut App) {
    let menus = menus(cx);
    cx.set_menus(menus);
}

/// Minimize and Zoom, which only a window answers.
pub fn window_actions<E: InteractiveElement>(root: E) -> E {
    root.on_action(|_: &Minimize, window, _| {
        log::info!("window: minimized");
        window.minimize_window();
    })
    .on_action(|_: &Zoom, window, _| {
        log::info!("window: zoomed");
        window.zoom_window();
    })
}
