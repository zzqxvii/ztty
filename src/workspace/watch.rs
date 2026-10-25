//! The shown worktree watched on disk, so Files and Git follow what changes there.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use futures::{
    FutureExt, StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
    future::{Either, select},
};
use gpui::{AsyncApp, Context, Task, WeakEntity};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::Workspace;

/// How long the disk must rest before the panels read again.
const QUIET: Duration = Duration::from_millis(300);

/// The shown worktree watched, so a change on disk reads Files and Git again.
pub(super) struct Watched {
    watcher: RecommendedWatcher,
    path: PathBuf,
    _pump: Task<()>,
}

impl Watched {
    /// Watches `folder`, reading the panels again once the disk rests after a change.
    pub(super) fn new(folder: &Path, cx: &mut Context<Workspace>) -> notify::Result<Self> {
        let (wake, events): (UnboundedSender<notify::Result<Event>>, _) = unbounded();
        let watcher = notify::recommended_watcher(move |event| {
            let _ = wake.unbounded_send(event);
        })?;
        let pump = cx.spawn(async move |this, cx| pump(events, this, cx).await);
        let mut watched = Self {
            watcher,
            path: folder.to_path_buf(),
            _pump: pump,
        };
        watched.watcher.watch(folder, RecursiveMode::Recursive)?;
        Ok(watched)
    }

    /// The folder watched.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Watches `folder` in place of the folder watched.
    pub(super) fn follow(&mut self, folder: &Path) -> notify::Result<()> {
        if folder == self.path {
            return Ok(());
        }
        self.watcher.unwatch(&self.path)?;
        self.watcher.watch(folder, RecursiveMode::Recursive)?;
        self.path = folder.to_path_buf();
        Ok(())
    }
}

/// Reads the panels after each change, once a burst of them has stopped.
async fn pump(
    mut events: UnboundedReceiver<notify::Result<Event>>,
    this: WeakEntity<Workspace>,
    cx: &mut AsyncApp,
) {
    while let Some(event) = events.next().await {
        if !event.is_ok_and(|event| matters(&event)) {
            continue;
        }
        // An editor saves by writing a temporary file and renaming it over the old one.
        let quiet = loop {
            let rest = cx.background_executor().timer(QUIET).fuse();
            let next = events.next().fuse();
            futures::pin_mut!(rest, next);
            match select(rest, next).await {
                Either::Left(((), _)) => break true,
                Either::Right((None, _)) => break false,
                Either::Right((Some(event), _)) => {
                    if !event.is_ok_and(|event| matters(&event)) {
                        continue;
                    }
                }
            }
        };
        if !quiet {
            return;
        }
        log::debug!("watch: the disk changed; the panels read again");
        if this.update(cx, |this, cx| this.refresh(cx)).is_err() {
            return;
        }
    }
}

/// Whether a change is one the panels read again for.
fn matters(event: &Event) -> bool {
    !matches!(event.kind, EventKind::Access(_))
        && !event.paths.is_empty()
        && !event.paths.iter().all(|path| ignored(path))
}

/// Whether a path is a repository's churn, which changes nothing the panels show.
fn ignored(path: &Path) -> bool {
    let parts: Vec<_> = path.components().map(|part| part.as_os_str()).collect();
    parts
        .windows(2)
        .any(|pair| pair[0] == ".git" && (pair[1] == "objects" || pair[1] == "logs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repositorys_objects_and_logs_are_ignored() {
        assert!(ignored(Path::new("/w/.git/objects/ab/cdef")));
        assert!(ignored(Path::new("/w/.git/logs/HEAD")));
        assert!(!ignored(Path::new("/w/.git/index")));
        assert!(!ignored(Path::new("/w/.git/HEAD")));
        assert!(!ignored(Path::new("/w/objects/a.rs")));
    }
}
