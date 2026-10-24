pub mod app;
pub mod blur;
pub mod components;
pub mod diff;
pub mod file;
pub mod files;
pub mod git;
pub mod i18n;
pub mod menus;
pub mod projects;
pub mod sessions;
pub mod settings;
pub mod term;
#[cfg(all(test, unix))]
mod testlog;
#[cfg(all(test, target_os = "linux"))]
mod testx11;
pub mod themes;
pub mod updates;
pub mod workspace;
pub mod worktree;
