use alacritty_terminal::{
    term::color::Colors,
    vte::ansi::{Color, NamedColor, Rgb},
};
use gpui::{Hsla, Rgba};

use crate::themes::TerminalTheme;

/// The colors a terminal paints with: its theme, under any a program set by OSC 4, 10, 11 or 12.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ink {
    pub ansi: [Hsla; 16],
    pub fg: Hsla,
    pub bg: Hsla,
    pub cursor: Hsla,
    pub cursor_text: Hsla,
    pub selection: Hsla,
    /// None keeps each cell's own color.
    pub selection_text: Option<Hsla>,
}

impl Ink {
    pub fn new(theme: &TerminalTheme, set: &Colors) -> Self {
        let named = |name: NamedColor| set[name].map(hsla);
        let mut ansi = theme.ansi;
        for (index, color) in ansi.iter_mut().enumerate() {
            if let Some(spec) = set[index] {
                *color = hsla(spec);
            }
        }
        Self {
            ansi,
            fg: named(NamedColor::Foreground).unwrap_or(theme.foreground),
            bg: named(NamedColor::Background).unwrap_or(theme.background),
            cursor: named(NamedColor::Cursor).unwrap_or(theme.cursor),
            cursor_text: theme.cursor_text,
            selection: theme.selection_background,
            selection_text: theme.selection_foreground,
        }
    }

    pub fn resolve(&self, color: Color, set: &Colors) -> Hsla {
        match color {
            Color::Spec(spec) => hsla(spec),
            Color::Indexed(index) => self.indexed(index.into(), set),
            Color::Named(name) => self.named(name),
        }
    }

    fn named(&self, name: NamedColor) -> Hsla {
        match name {
            NamedColor::Foreground | NamedColor::BrightForeground => self.fg,
            NamedColor::Background => self.bg,
            NamedColor::Cursor => self.cursor,
            NamedColor::DimForeground => self.fg.opacity(DIM),
            dim if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize)
                .contains(&(dim as usize)) =>
            {
                self.named(dim.to_bright()).opacity(DIM)
            }
            ansi => self.ansi[ansi as usize],
        }
    }

    fn indexed(&self, index: usize, set: &Colors) -> Hsla {
        if let Some(spec) = set[index] {
            return hsla(spec);
        }
        match index {
            0..16 => self.ansi[index],
            16..232 => {
                let step = |level: usize| if level == 0 { 0 } else { 55 + 40 * level as u8 };
                let cube = index - 16;
                hsla(Rgb {
                    r: step(cube / 36),
                    g: step(cube / 6 % 6),
                    b: step(cube % 6),
                })
            }
            _ => {
                let gray = 8 + 10 * (index - 232) as u8;
                hsla(Rgb {
                    r: gray,
                    g: gray,
                    b: gray,
                })
            }
        }
    }

    /// Color `index` as an answer to a program's query: 0–255, then 256 text, 257 ground, 258 cursor.
    pub fn query(&self, index: usize, set: &Colors) -> Rgb {
        let color = match index {
            256 => self.fg,
            257 => self.bg,
            258 => self.cursor,
            index => self.indexed(index.min(255), set),
        };
        let rgba = color.to_rgb();
        let byte = |channel: f32| (channel * 255.0).round() as u8;
        Rgb {
            r: byte(rgba.r),
            g: byte(rgba.g),
            b: byte(rgba.b),
        }
    }
}

/// How much of the text color dim text keeps.
pub const DIM: f32 = 0.66;

fn hsla(spec: Rgb) -> Hsla {
    let channel = |value: u8| f32::from(value) / 255.0;
    Rgba {
        r: channel(spec.r),
        g: channel(spec.g),
        b: channel(spec.b),
        a: 1.0,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes;

    fn ink(set: &Colors) -> Ink {
        Ink::new(themes::find("catppuccin-mocha").expect("mocha"), set)
    }

    #[test]
    fn indexed_colors_follow_the_xterm_cube_and_ramp() {
        let set = Colors::default();
        let ink = ink(&set);
        assert_eq!(ink.query(196, &set), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(ink.query(16, &set), Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(
            ink.query(244, &set),
            Rgb {
                r: 128,
                g: 128,
                b: 128
            }
        );
        assert_eq!(
            ink.query(1, &set),
            Rgb {
                r: 0xf3,
                g: 0x8b,
                b: 0xa8
            },
            "the theme's red"
        );
        assert_eq!(
            ink.query(257, &set),
            Rgb {
                r: 0x1e,
                g: 0x1e,
                b: 0x2e
            }
        );
    }

    #[test]
    fn a_color_the_program_set_wins() {
        let mut set = Colors::default();
        set[1] = Some(Rgb { r: 1, g: 2, b: 3 });
        set[NamedColor::Background] = Some(Rgb { r: 9, g: 9, b: 9 });
        let ink = ink(&set);
        assert_eq!(
            ink.resolve(Color::Named(NamedColor::Red), &set),
            hsla(Rgb { r: 1, g: 2, b: 3 })
        );
        assert_eq!(ink.bg, hsla(Rgb { r: 9, g: 9, b: 9 }));
        assert_eq!(
            ink.resolve(Color::Named(NamedColor::DimRed), &set),
            ink.ansi[1].opacity(DIM)
        );
    }
}
