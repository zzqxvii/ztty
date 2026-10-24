//! A file shown read-only in a tab.

use std::{fs, path::PathBuf};

use ely_gpui_component::{
    editor::CodeEditor,
    theme::{ActiveTheme, TextSize},
    typography::Caption,
};
use gpui::{
    AppContext, Context, IntoElement, ParentElement, Render, SharedString, Styled, Task, Window,
    div,
};

use crate::settings::SettingsStore;

/// The most bytes shown; past it the file is cut off with a line.
const LIMIT: usize = 2 * 1024 * 1024;

/// What a file holds.
#[derive(Clone, Debug, PartialEq)]
enum Content {
    Text { text: String, cut: bool },
    Binary,
    Failed(String),
}

/// Reads `path` off the main thread, as a tab shows it.
fn read(path: &std::path::Path) -> Content {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return Content::Failed(error.to_string()),
    };
    let cut = bytes.len() > LIMIT;
    let kept = &bytes[..bytes.len().min(LIMIT)];
    if kept.contains(&0) {
        return Content::Binary;
    }
    match std::str::from_utf8(kept) {
        Ok(text) => Content::Text {
            text: text.into(),
            cut,
        },
        // A cut may split a character; what decodes stays.
        Err(error) if cut => match std::str::from_utf8(&kept[..error.valid_up_to()]) {
            Ok(text) => Content::Text {
                text: text.into(),
                cut: true,
            },
            Err(_) => Content::Binary,
        },
        Err(_) => Content::Binary,
    }
}

/// One file open in a worktree's tab: its text read off the main thread, then shown read-only.
pub struct FileView {
    pub path: PathBuf,
    content: Option<Content>,
    editor: Option<gpui::Entity<CodeEditor>>,
    /// The text the editor holds.
    shown: Option<String>,
    /// The scale the editor draws at, against the theme's code size.
    scale: Option<f32>,
    reading: Option<Task<()>>,
}

impl FileView {
    pub fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            path,
            content: None,
            editor: None,
            shown: None,
            scale: None,
            reading: None,
        };
        view.reload(cx);
        view
    }

    /// What the tab calls it.
    pub fn name(&self) -> String {
        crate::projects::name(&self.path)
    }

    /// Reads the file again, off the main thread.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let reading = cx.background_spawn(async move { read(&path) });
        self.reading = Some(cx.spawn(async move |this, cx| {
            let content = reading.await;
            let _ = this.update(cx, |this, cx| {
                this.content = Some(content);
                cx.notify();
            });
        }));
    }

    /// The editor, once the text is read.
    pub fn editor(&self) -> Option<&gpui::Entity<CodeEditor>> {
        self.editor.as_ref()
    }
}

impl Render for FileView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let note = |text: SharedString| div().p_3().child(Caption::new(text)).into_any_element();
        let Some(content) = &self.content else {
            return note(crate::i18n::text(cx, "file.reading", &[]));
        };
        match content {
            Content::Failed(reason) => note(reason.clone().into()),
            Content::Binary => note(crate::i18n::text(cx, "file.binary", &[])),
            Content::Text { text, cut } => {
                // The settings' size, against the code size Ely draws at.
                let size = SettingsStore::get(cx).editor_font_size;
                let base = f32::from(
                    cx.theme()
                        .text_size(TextSize::Sm)
                        .to_pixels(window.rem_size()),
                );
                let scale = size / base;
                // A reload builds the editor again over the new text.
                let retext = self.shown.as_deref() != Some(text.as_str());
                if retext || self.scale != Some(scale) {
                    if retext {
                        let text = text.clone();
                        self.editor = Some(cx.new(|cx| {
                            let mut editor = CodeEditor::new(text.clone(), window, cx).read_only();
                            editor.set_scale(scale, cx);
                            editor
                        }));
                        self.shown = Some(text);
                    } else if let Some(editor) = self.editor.clone() {
                        editor.update(cx, |editor, cx| editor.set_scale(scale, cx));
                    }
                    self.scale = Some(scale);
                }
                let editor = self.editor.clone().expect("an editor");
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .children(cut.then(|| {
                        let mib = (LIMIT / (1024 * 1024)).to_string();
                        Caption::new(crate::i18n::text(cx, "file.cut", &[("mib", &mib)]))
                            .px_3()
                            .pt_2()
                    }))
                    .child(div().flex_1().min_h_0().child(editor))
                    .into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("ztty-file-{}-{name}", std::process::id()));
        fs::write(&path, bytes).expect("a test file");
        path
    }

    #[test]
    fn text_reads_as_text_and_nul_bytes_as_binary() {
        let path = scratch("text", "café\n".as_bytes());
        assert_eq!(
            read(&path),
            Content::Text {
                text: "café\n".into(),
                cut: false
            }
        );
        fs::remove_file(&path).expect("cleanup");
        let path = scratch("binary", b"a\0b");
        assert_eq!(read(&path), Content::Binary);
        fs::remove_file(&path).expect("cleanup");
    }

    #[test]
    fn a_missing_file_says_why() {
        let path = std::env::temp_dir().join("ztty-file-gone");
        let _ = fs::remove_file(&path);
        assert!(matches!(read(&path), Content::Failed(_)));
    }

    #[test]
    fn a_long_file_is_cut_at_its_limit() {
        let mut bytes = vec![b'a'; LIMIT + 10];
        bytes[LIMIT - 1] = 0xc3;
        bytes[LIMIT] = 0xa9;
        let path = scratch("long", &bytes);
        match read(&path) {
            Content::Text { text, cut } => {
                assert!(cut);
                assert_eq!(text.len(), LIMIT - 1, "the split character is dropped");
            }
            other => panic!("a text file, not {other:?}"),
        }
        fs::remove_file(&path).expect("cleanup");
    }
}
