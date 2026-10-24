use anyhow::bail;
use gpui::Window;
use objc::{
    class, msg_send,
    runtime::{BOOL, Object, YES},
    sel, sel_impl,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// `NSVisualEffectMaterialUnderWindowBackground`.
const UNDER_WINDOW_BACKGROUND: isize = 21;

/// Gives gpui's effect view a blurring material; logs either way.
pub fn blur(window: &Window) {
    match effect_view(window) {
        Ok(view) => {
            unsafe {
                let _: () = msg_send![view, setMaterial: UNDER_WINDOW_BACKGROUND];
            }
            log::info!(
                "blur: the window's NSVisualEffectView takes underWindowBackground; gpui's Selection material draws no blur on macOS 27"
            );
        }
        Err(error) => log::error!("blur: the window does not blur: {error:#}"),
    }
}

/// The NSVisualEffectView gpui puts under the window's content for `Blurred`.
fn effect_view(window: &Window) -> anyhow::Result<*mut Object> {
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("no native handle: {error:?}"))?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        bail!("not an AppKit window");
    };
    unsafe {
        let view = appkit.ns_view.as_ptr() as *mut Object;
        let native: *mut Object = msg_send![view, window];
        let content: *mut Object = msg_send![native, contentView];
        let views: *mut Object = msg_send![content, subviews];
        let count: usize = msg_send![views, count];
        for ix in 0..count {
            let view: *mut Object = msg_send![views, objectAtIndex: ix];
            let effect: BOOL = msg_send![view, isKindOfClass: class!(NSVisualEffectView)];
            if effect == YES {
                return Ok(view);
            }
        }
    }
    bail!("gpui put no NSVisualEffectView under the content")
}
