use super::{Action, Key, code, printable};

/// xterm's encoding, as Ghostty sends it with no kitty flags set.
pub(super) fn legacy(key: &Key, app_cursor: bool) -> Option<Vec<u8>> {
    if key.action == Action::Release {
        return None;
    }
    if let Some(sequence) = function(key, app_cursor) {
        return Some(sequence);
    }
    printable_key(key, printable(key.name)?)
}

/// What Ctrl or Alt makes of a printable key; none for text and for Cmd.
pub(super) fn printable_key(key: &Key, ch: char) -> Option<Vec<u8>> {
    if key.platform {
        return None;
    }
    if key.ctrl {
        if let Some(byte) = control(ch, key.shift) {
            return Some(if key.alt {
                vec![0x1B, byte]
            } else {
                vec![byte]
            });
        }
        return Some(format!("\x1b[{};{}u", ch as u32, code(key)).into_bytes());
    }
    if !key.alt {
        return None;
    }
    let mut bytes = vec![0x1B];
    if key.text.is_empty() {
        let typed = if key.shift {
            ch.to_ascii_uppercase()
        } else {
            ch
        };
        bytes.extend(typed.to_string().as_bytes());
    } else {
        bytes.extend(key.text.as_bytes());
    }
    Some(bytes)
}

/// xterm's bytes for a function, cursor or editing key.
pub(super) fn function(key: &Key, app_cursor: bool) -> Option<Vec<u8>> {
    let mods = code(key);
    let modified = mods > 1;
    let cursor = |end: char| match (modified, app_cursor) {
        (true, _) => format!("\x1b[1;{mods}{end}"),
        (false, true) => format!("\x1bO{end}"),
        (false, false) => format!("\x1b[{end}"),
    };
    let tilde = |n: u32| {
        if modified {
            format!("\x1b[{n};{mods}~")
        } else {
            format!("\x1b[{n}~")
        }
    };
    let ss3 = |end: char| {
        if modified {
            format!("\x1b[1;{mods}{end}")
        } else {
            format!("\x1bO{end}")
        }
    };
    let alt_only = mods == 3;
    let sequence = match key.name {
        "up" => cursor('A'),
        "down" => cursor('B'),
        "right" => cursor('C'),
        "left" => cursor('D'),
        "home" => cursor('H'),
        "end" => cursor('F'),
        "insert" => tilde(2),
        "delete" => tilde(3),
        "pageup" => tilde(5),
        "pagedown" => tilde(6),
        "f1" => ss3('P'),
        "f2" => ss3('Q'),
        "f3" if modified => tilde(13),
        "f3" => ss3('R'),
        "f4" => ss3('S'),
        "enter" if !modified => "\r".into(),
        "enter" if alt_only => "\x1b\r".into(),
        "enter" => format!("\x1b[27;{mods};13~"),
        "tab" if !modified => "\t".into(),
        "tab" if mods == 2 => "\x1b[Z".into(),
        "tab" if alt_only => "\x1b\t".into(),
        "tab" => format!("\x1b[27;{mods};9~"),
        "escape" if !modified => "\x1b".into(),
        "escape" if alt_only => "\x1b\x1b".into(),
        "escape" => format!("\x1b[27;{mods};27~"),
        "backspace" => {
            let byte = if key.ctrl { 0x08 } else { 0x7F };
            return Some(if key.alt {
                vec![0x1B, byte]
            } else {
                vec![byte]
            });
        }
        name => tilde(FUNCTION_TILDES.iter().find(|(f, _)| *f == name)?.1),
    };
    Some(sequence.into_bytes())
}

/// F5 to F25 and the numbers xterm sends them with.
pub(super) const FUNCTION_TILDES: [(&str, u32); 21] = [
    ("f5", 15),
    ("f6", 17),
    ("f7", 18),
    ("f8", 19),
    ("f9", 20),
    ("f10", 21),
    ("f11", 23),
    ("f12", 24),
    ("f13", 25),
    ("f14", 26),
    ("f15", 28),
    ("f16", 29),
    ("f17", 31),
    ("f18", 32),
    ("f19", 33),
    ("f20", 34),
    ("f21", 42),
    ("f22", 43),
    ("f23", 44),
    ("f24", 45),
    ("f25", 46),
];

/// The C0 byte Ctrl turns a key into, as kitty and Ghostty do; `[`, `i` and `m` go as CSI u.
fn control(ch: char, shift: bool) -> Option<u8> {
    if shift && ch != '@' {
        return None;
    }
    Some(match ch {
        ' ' | '2' | '@' => 0,
        'a'..='h' | 'j' | 'k' | 'l' | 'n'..='z' => ch as u8 - b'a' + 1,
        '3' => 27,
        '4' | '\\' => 28,
        '5' | ']' => 29,
        '6' | '^' | '~' => 30,
        '7' | '/' | '_' => 31,
        '8' | '?' => 127,
        '0' | '1' | '9' => ch as u8,
        _ => return None,
    })
}
