use anyhow::Context as _;
use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::{
    protocol::xproto::{Atom, AtomEnum, ConnectionExt as _, PropMode},
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

/// KWin blurs behind the rects a window names here, in its own pixels.
const BLUR_BEHIND: &str = "_KDE_NET_WM_BLUR_BEHIND_REGION";

/// The compositor's blur behind an X11 window, through `_KDE_NET_WM_BLUR_BEHIND_REGION`.
pub struct Effect {
    connection: RustConnection,
    window: u32,
    atom: Atom,
    /// The rect the property names, as last set.
    blurs: Option<[i32; 4]>,
}

impl Effect {
    /// The effect on `window`'s X11 window, blurring nothing yet; none off X11.
    pub fn on(window: &Window) -> Option<Self> {
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            log::error!("blur: gpui gave no handle to the window");
            return None;
        };
        let RawWindowHandle::Xcb(handle) = handle.as_raw() else {
            log::info!("blur: no X11 window, so no {BLUR_BEHIND}");
            return None;
        };
        // gpui opens $DISPLAY too; a property belongs to the window, whoever sets it.
        match Self::new(None, handle.window.get()) {
            Ok(effect) => Some(effect),
            Err(error) => {
                log::error!("blur: {BLUR_BEHIND} failed: {error:#}");
                None
            }
        }
    }

    /// An effect on `window` of the X server `display` names, or `$DISPLAY`'s.
    fn new(display: Option<&str>, window: u32) -> anyhow::Result<Self> {
        let (connection, _) = x11rb::connect(display).context("blur: no X server")?;
        let atom = (connection.intern_atom(false, BLUR_BEHIND.as_bytes()))
            .context("blur: the atom did not go out")?
            .reply()
            .context("blur: the atom")?
            .atom;
        log::info!(
            "blur: {BLUR_BEHIND} on X11 window {window}; KWin blurs behind it, other compositors pass it by"
        );
        Ok(Self {
            connection,
            window,
            atom,
            blurs: None,
        })
    }

    /// Names `rect`, in the window's pixels, or removes the property; the X server has it on return; sends only a change.
    pub fn blur(&mut self, rect: Option<[i32; 4]>) {
        if rect == self.blurs {
            return;
        }
        let turned = rect.is_some() != self.blurs.is_some();
        self.blurs = rect;
        let asked = match rect {
            Some(rect) => {
                let cardinals = rect.map(|side| side.max(0) as u32);
                (self.connection)
                    .change_property32(
                        PropMode::REPLACE,
                        self.window,
                        self.atom,
                        AtomEnum::CARDINAL,
                        &cardinals,
                    )
                    .map(|cookie| cookie.check())
            }
            None => (self.connection)
                .delete_property(self.window, self.atom)
                .map(|cookie| cookie.check()),
        };
        let what = rect.map_or("nothing".into(), |rect| format!("{rect:?}"));
        match asked {
            Ok(Ok(())) if turned => log::info!("blur: {BLUR_BEHIND} names {what}"),
            Ok(Ok(())) => log::debug!("blur: {BLUR_BEHIND} names {what}"),
            Ok(Err(error)) => log::error!("blur: the X server refused {BLUR_BEHIND}: {error}"),
            Err(error) => log::error!("blur: {BLUR_BEHIND} did not reach the X server: {error}"),
        }
    }
}

#[cfg(test)]
mod tests;
