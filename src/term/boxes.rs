mod outline;

/// A point in device pixels from the cell's top-left.
pub type Point = (f32, f32);

/// A piece of a drawn character in device pixels from the cell's top-left; pieces never overlap.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A rectangle filled at `alpha` of the text color.
    Rect {
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        alpha: f32,
    },
    /// A simple polygon, filled.
    Fill(Vec<Point>),
}

/// Whether the terminal draws `ch` as shapes instead of through the font.
pub fn drawn(ch: char) -> bool {
    matches!(ch, '\u{2500}'..='\u{259F}' | '\u{E0B0}'..='\u{E0B7}')
}

const NONE: u8 = 0;
const LIGHT: u8 = 1;
const HEAVY: u8 = 2;
const DOUBLE: u8 = 3;

/// Each line character's arms, up, right, down, left, from U+2500.
#[rustfmt::skip]
const LINES: [[u8; 4]; 128] = [
    [0, 1, 0, 1], [0, 2, 0, 2], [1, 0, 1, 0], [2, 0, 2, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
    [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 1, 1, 0], [0, 2, 1, 0], [0, 1, 2, 0], [0, 2, 2, 0],
    [0, 0, 1, 1], [0, 0, 1, 2], [0, 0, 2, 1], [0, 0, 2, 2], [1, 1, 0, 0], [1, 2, 0, 0], [2, 1, 0, 0], [2, 2, 0, 0],
    [1, 0, 0, 1], [1, 0, 0, 2], [2, 0, 0, 1], [2, 0, 0, 2], [1, 1, 1, 0], [1, 2, 1, 0], [2, 1, 1, 0], [1, 1, 2, 0],
    [2, 1, 2, 0], [2, 2, 1, 0], [1, 2, 2, 0], [2, 2, 2, 0], [1, 0, 1, 1], [1, 0, 1, 2], [2, 0, 1, 1], [1, 0, 2, 1],
    [2, 0, 2, 1], [2, 0, 1, 2], [1, 0, 2, 2], [2, 0, 2, 2], [0, 1, 1, 1], [0, 1, 1, 2], [0, 2, 1, 1], [0, 2, 1, 2],
    [0, 1, 2, 1], [0, 1, 2, 2], [0, 2, 2, 1], [0, 2, 2, 2], [1, 1, 0, 1], [1, 1, 0, 2], [1, 2, 0, 1], [1, 2, 0, 2],
    [2, 1, 0, 1], [2, 1, 0, 2], [2, 2, 0, 1], [2, 2, 0, 2], [1, 1, 1, 1], [1, 1, 1, 2], [1, 2, 1, 1], [1, 2, 1, 2],
    [2, 1, 1, 1], [1, 1, 2, 1], [2, 1, 2, 1], [2, 1, 1, 2], [2, 2, 1, 1], [1, 1, 2, 2], [1, 2, 2, 1], [2, 2, 1, 2],
    [1, 2, 2, 2], [2, 1, 2, 2], [2, 2, 2, 1], [2, 2, 2, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
    [0, 3, 0, 3], [3, 0, 3, 0], [0, 3, 1, 0], [0, 1, 3, 0], [0, 3, 3, 0], [0, 0, 1, 3], [0, 0, 3, 1], [0, 0, 3, 3],
    [1, 3, 0, 0], [3, 1, 0, 0], [3, 3, 0, 0], [1, 0, 0, 3], [3, 0, 0, 1], [3, 0, 0, 3], [1, 3, 1, 0], [3, 1, 3, 0],
    [3, 3, 3, 0], [1, 0, 1, 3], [3, 0, 3, 1], [3, 0, 3, 3], [0, 3, 1, 3], [0, 1, 3, 1], [0, 3, 3, 3], [1, 3, 0, 3],
    [3, 1, 0, 1], [3, 3, 0, 3], [1, 3, 1, 3], [3, 1, 3, 1], [3, 3, 3, 3], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
    [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 1], [1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0],
    [0, 0, 0, 2], [2, 0, 0, 0], [0, 2, 0, 0], [0, 0, 2, 0], [0, 2, 0, 1], [1, 0, 2, 0], [0, 1, 0, 2], [2, 0, 1, 0],
];

/// Quadrant blocks from U+2596: top-left, top-right, bottom-left, bottom-right bits.
const QUADRANTS: [u8; 10] = [
    0b0010, 0b0001, 0b1000, 0b1011, 0b1001, 0b1110, 0b1101, 0b0100, 0b0110, 0b0111,
];

/// A box by its corners: left, top, right, bottom.
type Corners = [u32; 4];

/// The shapes of a `drawn` char in a `w` by `h` cell whose light lines are `t` thick.
pub fn shapes(ch: char, w: u32, h: u32, t: u32) -> Vec<Shape> {
    let code = ch as u32;
    match code {
        0x2504..=0x250B | 0x254C..=0x254F => rects(dashes(code, w, h, t), 1.),
        0x256D..=0x2570 => outline::arc(code, w, h, t),
        0x2571..=0x2573 => outline::diagonals(code, w, h, t),
        0x2500..=0x257F => rects(lines(LINES[(code - 0x2500) as usize], w, h, t), 1.),
        0x2591..=0x2593 => rects(vec![[0, 0, w, h]], (code - 0x2590) as f32 / 4.),
        0x2580..=0x259F => rects(blocks(code, w, h), 1.),
        0xE0B0..=0xE0B7 => outline::powerline(code, w, h, t),
        _ => panic!("U+{code:04X} is not a drawn character"),
    }
}

/// Rects from boxes with their overlaps cut away, so each pixel takes the color once.
fn rects(boxes: Vec<Corners>, alpha: f32) -> Vec<Shape> {
    let mut kept: Vec<Corners> = Vec::new();
    for corners in boxes {
        let mut pieces = vec![corners];
        for seen in &kept {
            pieces = pieces
                .into_iter()
                .flat_map(|piece| outside(piece, *seen))
                .collect();
        }
        kept.extend(pieces);
    }
    kept.into_iter()
        .filter(|[x0, y0, x1, y1]| x1 > x0 && y1 > y0)
        .map(|[x0, y0, x1, y1]| Shape::Rect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
            alpha,
        })
        .collect()
}

/// The parts of `piece` outside `cut`.
fn outside(piece: Corners, cut: Corners) -> Vec<Corners> {
    let ([x0, y0, x1, y1], [cx0, cy0, cx1, cy1]) = (piece, cut);
    if cx0 >= x1 || cx1 <= x0 || cy0 >= y1 || cy1 <= y0 {
        return vec![piece];
    }
    let (top, bottom) = (cy0.max(y0), cy1.min(y1));
    [
        [x0, y0, x1, top],
        [x0, bottom, x1, y1],
        [x0, top, cx0.max(x0), bottom],
        [cx1.min(x1), top, x1, bottom],
    ]
    .into_iter()
    .filter(|[a, b, c, d]| c > a && d > b)
    .collect()
}

/// Where an arm ends at the center, by the arms crossing it: `[heavy, double, light, short]`.
fn stop((a, b): (u8, u8), straight: bool, [heavy, double, light, short]: [u32; 4]) -> u32 {
    if a == HEAVY || b == HEAVY {
        heavy
    } else if a != b || straight {
        if a == DOUBLE || b == DOUBLE {
            double
        } else {
            light
        }
    } else if a == NONE && b == NONE {
        light
    } else {
        short
    }
}

/// Arms from the center to the edges, joined as Ghostty joins them.
fn lines([up, right, down, left]: [u8; 4], w: u32, h: u32, t: u32) -> Vec<Corners> {
    let heavy = 2 * t;
    let h_light_top = h.saturating_sub(t) / 2;
    let h_light_bottom = h_light_top + t;
    let h_heavy_top = h.saturating_sub(heavy) / 2;
    let h_heavy_bottom = h_heavy_top + heavy;
    let h_double_top = h_light_top.saturating_sub(t);
    let h_double_bottom = h_light_bottom + t;
    let v_light_left = w.saturating_sub(t) / 2;
    let v_light_right = v_light_left + t;
    let v_heavy_left = w.saturating_sub(heavy) / 2;
    let v_heavy_right = v_heavy_left + heavy;
    let v_double_left = v_light_left.saturating_sub(t);
    let v_double_right = v_light_right + t;
    let up_bottom = stop(
        (left, right),
        down == up,
        [h_heavy_bottom, h_double_bottom, h_light_bottom, h_light_top],
    );
    let down_top = stop(
        (left, right),
        up == down,
        [h_heavy_top, h_double_top, h_light_top, h_light_bottom],
    );
    let left_right = stop(
        (up, down),
        left == right,
        [v_heavy_right, v_double_right, v_light_right, v_light_left],
    );
    let right_left = stop(
        (up, down),
        right == left,
        [v_heavy_left, v_double_left, v_light_left, v_light_right],
    );
    let (top_double, bottom_double) = (up == DOUBLE, down == DOUBLE);
    let mut out = Vec::new();
    match up {
        LIGHT => out.push([v_light_left, 0, v_light_right, up_bottom]),
        HEAVY => out.push([v_heavy_left, 0, v_heavy_right, up_bottom]),
        DOUBLE => {
            let end = |side| {
                if side == DOUBLE {
                    h_light_top
                } else {
                    up_bottom
                }
            };
            out.push([v_double_left, 0, v_light_left, end(left)]);
            out.push([v_light_right, 0, v_double_right, end(right)]);
        }
        _ => {}
    }
    match right {
        LIGHT => out.push([right_left, h_light_top, w, h_light_bottom]),
        HEAVY => out.push([right_left, h_heavy_top, w, h_heavy_bottom]),
        DOUBLE => {
            let start = |double| if double { v_light_right } else { right_left };
            out.push([start(top_double), h_double_top, w, h_light_top]);
            out.push([start(bottom_double), h_light_bottom, w, h_double_bottom]);
        }
        _ => {}
    }
    match down {
        LIGHT => out.push([v_light_left, down_top, v_light_right, h]),
        HEAVY => out.push([v_heavy_left, down_top, v_heavy_right, h]),
        DOUBLE => {
            let start = |side| {
                if side == DOUBLE {
                    h_light_bottom
                } else {
                    down_top
                }
            };
            out.push([v_double_left, start(left), v_light_left, h]);
            out.push([v_light_right, start(right), v_double_right, h]);
        }
        _ => {}
    }
    match left {
        LIGHT => out.push([0, h_light_top, left_right, h_light_bottom]),
        HEAVY => out.push([0, h_heavy_top, left_right, h_heavy_bottom]),
        DOUBLE => {
            let end = |double| if double { v_light_left } else { left_right };
            out.push([0, h_double_top, end(top_double), h_light_top]);
            out.push([0, h_light_bottom, end(bottom_double), h_double_bottom]);
        }
        _ => {}
    }
    out
}

/// Dashes that tile across cells; a cell too small for them takes a solid light line.
fn dashes(code: u32, w: u32, h: u32, t: u32) -> Vec<Corners> {
    let wide = t.max(4);
    let (count, thick, gap, vertical) = match code {
        0x2504 => (3, t, wide, false),
        0x2505 => (3, 2 * t, wide, false),
        0x2506 => (3, t, wide, true),
        0x2507 => (3, 2 * t, wide, true),
        0x2508 => (4, t, wide, false),
        0x2509 => (4, 2 * t, wide, false),
        0x250A => (4, t, wide, true),
        0x250B => (4, 2 * t, wide, true),
        0x254C => (2, t, t, false),
        0x254D => (2, 2 * t, 2 * t, false),
        0x254E => (2, t, 2 * t, true),
        _ => (2, 2 * t, 2 * t, true),
    };
    let (along, across) = if vertical { (h, w) } else { (w, h) };
    if along < 2 * count {
        let solid = if vertical {
            [LIGHT, NONE, LIGHT, NONE]
        } else {
            [NONE, LIGHT, NONE, LIGHT]
        };
        return lines(solid, w, h, t);
    }
    let gap = gap.min(along / (2 * count));
    let total = along - count * gap;
    let (dash, mut extra) = (total / count, total % count);
    let side = across.saturating_sub(thick) / 2;
    let mut at = if vertical { 0 } else { gap / 2 };
    (0..count)
        .map(|_| {
            let end = at + dash + u32::from(extra > 0);
            extra = extra.saturating_sub(1);
            let piece = if vertical {
                [side, at, side + thick, end]
            } else {
                [at, side, end, side + thick]
            };
            at = end + gap;
            piece
        })
        .collect()
}

/// Block elements but the shades: eighths, halves and quadrants.
fn blocks(code: u32, w: u32, h: u32) -> Vec<Corners> {
    let part = |share: f32, side: u32| (side as f32 * share).round() as u32;
    let eighths = |n: u32| n as f32 / 8.;
    match code {
        0x2580 => vec![[0, 0, w, part(0.5, h)]],
        0x2581..=0x2587 => vec![[0, h - part(eighths(code - 0x2580), h), w, h]],
        0x2588 => vec![[0, 0, w, h]],
        0x2589..=0x258F => vec![[0, 0, part(eighths(0x2590 - code), w), h]],
        0x2590 => vec![[w - part(0.5, w), 0, w, h]],
        0x2594 => vec![[0, 0, w, part(0.125, h)]],
        0x2595 => vec![[w - part(0.125, w), 0, w, h]],
        _ => {
            let bits = QUADRANTS[(code - 0x2596) as usize];
            let (mx, my) = (part(0.5, w), part(0.5, h));
            let (nx, ny) = (w - mx, h - my);
            [
                [0, 0, mx, my],
                [nx, 0, w, my],
                [0, ny, mx, h],
                [nx, ny, w, h],
            ]
            .into_iter()
            .enumerate()
            .filter(|(index, _)| bits & (0b1000 >> index) != 0)
            .map(|(_, corners)| corners)
            .collect()
        }
    }
}

#[cfg(test)]
mod tests;
