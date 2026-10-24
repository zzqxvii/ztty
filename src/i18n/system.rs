//! The language the system asks for.

/// The tag of the system's language, one the app speaks.
pub fn tag() -> &'static str {
    pick(&asked())
}

/// The system's locale name, as it writes it.
#[cfg(windows)]
fn asked() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    // Windows' LOCALE_NAME_MAX_LENGTH.
    let mut name = [0u16; 85];
    // SAFETY: the buffer holds what the call writes.
    let len = unsafe { GetUserDefaultLocaleName(name.as_mut_ptr(), name.len() as i32) };
    if len > 0 {
        String::from_utf16_lossy(&name[..len as usize - 1])
    } else {
        log::warn!("i18n: Windows names no locale; the app speaks en-US");
        String::new()
    }
}

/// AppKit's locale, as `zh_CN` or `en_US`.
#[cfg(target_os = "macos")]
fn asked() -> String {
    use std::ffi::CStr;

    use objc::{class, msg_send, runtime::Object, sel, sel_impl};

    // SAFETY: NSLocale and NSString answer as Apple's runtime says.
    unsafe {
        let locale: *mut Object = msg_send![class!(NSLocale), currentLocale];
        let id: *mut Object = msg_send![locale, localeIdentifier];
        let text: *const i8 = msg_send![id, UTF8String];
        if text.is_null() {
            return String::new();
        }
        CStr::from_ptr(text).to_string_lossy().into_owned()
    }
}

/// `LC_ALL`, then `LC_MESSAGES`, then `LANG`, as the shell sets them.
#[cfg(all(unix, not(target_os = "macos")))]
fn asked() -> String {
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        match std::env::var(name) {
            Ok(value) if !value.is_empty() => return value,
            _ => {}
        }
    }
    String::new()
}

/// The language of `name`, `zh_CN.UTF-8` or `en-US`, among those the app speaks; English for a language it does not.
fn pick(name: &str) -> &'static str {
    let name = name.split('.').next().unwrap_or_default();
    let name = name.split('@').next().unwrap_or_default();
    let language = name
        .split(['_', '-'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    super::LOCALES
        .iter()
        .map(|(tag, _)| *tag)
        .find(|tag| tag.split('-').next() == Some(language.as_str()))
        .unwrap_or("en-US")
}

#[cfg(test)]
mod tests {
    use super::pick;

    #[test]
    fn a_locale_name_picks_its_language() {
        assert_eq!(pick("zh_CN.UTF-8"), "zh-CN");
        assert_eq!(pick("zh-Hans-CN"), "zh-CN");
        assert_eq!(pick("zh_CN@stroke"), "zh-CN");
        assert_eq!(pick("en_GB.UTF-8"), "en-US");
        assert_eq!(pick("de-DE"), "en-US", "a language the app does not speak");
        assert_eq!(pick(""), "en-US");
    }
}
