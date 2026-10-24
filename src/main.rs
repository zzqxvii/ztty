// A release build on Windows opens no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ely_gpui_component::Assets;
use gpui::App;
use ztty::app;

/// A Linux app with no display server fails at start instead of running headless.
#[cfg(target_os = "linux")]
fn application() -> gpui::Application {
    gpui_platform::linux(gpui::WindowingModes::WAYLAND | gpui::WindowingModes::X11)
}

#[cfg(not(target_os = "linux"))]
fn application() -> gpui::Application {
    gpui_platform::application()
}

fn run(cx: &mut App) -> anyhow::Result<()> {
    app::start(cx)?;
    app::open(cx)?;
    cx.activate(true);
    Ok(())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("ztty=info")).init();
    application().with_assets(Assets).run(|cx: &mut App| {
        ely_gpui_component::init(cx).expect("Ely failed to start");
        if let Err(error) = run(cx) {
            log::error!("{error:#}");
            std::process::exit(1);
        }
    });
}
