use std::collections::BTreeSet;

use super::*;

/// Cells as fonts give them: even and odd sides, thin and thick lines.
const CELLS: [(u32, u32, u32); 4] = [(16, 32, 2), (9, 19, 1), (20, 41, 3), (7, 15, 1)];

fn arms(ch: char) -> [u8; 4] {
    LINES[(ch as u32 - 0x2500) as usize]
}

fn every_drawn() -> impl Iterator<Item = char> {
    ('\u{2500}'..='\u{259F}').chain('\u{E0B0}'..='\u{E0B7}')
}

fn corners(shape: &Shape) -> Option<Corners> {
    match *shape {
        Shape::Rect { x, y, w, h, .. } => Some([x, y, x + w, y + h]),
        Shape::Fill(_) => None,
    }
}

fn pixels(boxes: &[Corners]) -> BTreeSet<(u32, u32)> {
    boxes
        .iter()
        .flat_map(|&[x0, y0, x1, y1]| (y0..y1).flat_map(move |y| (x0..x1).map(move |x| (x, y))))
        .collect()
}

/// The merged spans across an edge of the rects touching it; sides go up, right, down, left.
fn edge(shapes: &[Shape], side: usize, cell_w: u32, cell_h: u32) -> Vec<(u32, u32)> {
    let mut spans: Vec<(u32, u32)> = shapes
        .iter()
        .filter_map(corners)
        .filter(|&[x0, y0, x1, y1]| [y0 == 0, x1 == cell_w, y1 == cell_h, x0 == 0][side])
        .map(|[x0, y0, x1, y1]| {
            if side.is_multiple_of(2) {
                (x0, x1)
            } else {
                (y0, y1)
            }
        })
        .collect();
    spans.sort();
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for span in spans {
        match merged.last_mut() {
            Some(last) if span.0 <= last.1 => last.1 = last.1.max(span.1),
            _ => merged.push(span),
        }
    }
    merged
}

/// Whether segments `a`-`b` and `c`-`d` cross inside both.
fn cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let turn = |p: Point, q: Point, r: Point| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    turn(c, d, a) * turn(c, d, b) < -1e-4 && turn(a, b, c) * turn(a, b, d) < -1e-4
}

#[test]
fn the_table_reads_as_unicode_names_say() {
    assert_eq!(arms('─'), [0, 1, 0, 1]);
    assert_eq!(arms('┃'), [2, 0, 2, 0]);
    assert_eq!(arms('┍'), [0, 2, 1, 0], "down light and right heavy");
    assert_eq!(arms('╒'), [0, 3, 1, 0], "down single and right double");
    assert_eq!(arms('╬'), [3, 3, 3, 3]);
    assert_eq!(arms('╽'), [1, 0, 2, 0], "light up and heavy down");
    assert_eq!(arms('╾'), [0, 1, 0, 2], "heavy left and light right");
}

#[test]
fn every_drawn_char_draws_inside_its_cell() {
    for (w, h, t) in CELLS {
        for ch in every_drawn() {
            let shapes = shapes(ch, w, h, t);
            assert!(!shapes.is_empty(), "{ch} draws nothing");
            for shape in &shapes {
                match shape {
                    Shape::Rect {
                        x, y, w: rw, h: rh, ..
                    } => {
                        assert!(*rw > 0 && *rh > 0, "{ch} has an empty rect at {w}x{h}");
                        assert!(x + rw <= w && y + rh <= h, "{ch} leaves {w}x{h}");
                    }
                    Shape::Fill(polygon) => {
                        assert!(polygon.len() >= 3, "{ch} has a degenerate polygon");
                        let reach = 2. * t as f32;
                        for &(px, py) in polygon {
                            let inside = (-reach..=w as f32 + reach).contains(&px)
                                && (-reach..=h as f32 + reach).contains(&py);
                            assert!(inside, "{ch} reaches ({px}, {py}) in {w}x{h}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn no_pixel_is_painted_twice() {
    for (w, h, t) in CELLS {
        for ch in every_drawn() {
            let shapes = shapes(ch, w, h, t);
            let rects: Vec<Corners> = shapes.iter().filter_map(corners).collect();
            for (index, a) in rects.iter().enumerate() {
                for b in &rects[index + 1..] {
                    let apart = a[2] <= b[0] || b[2] <= a[0] || a[3] <= b[1] || b[3] <= a[1];
                    assert!(apart, "{ch} overlaps {a:?} and {b:?} at {w}x{h}");
                }
            }
            for shape in &shapes {
                let Shape::Fill(polygon) = shape else {
                    continue;
                };
                let n = polygon.len();
                for i in 0..n {
                    for j in i + 2..n {
                        if i == 0 && j == n - 1 {
                            continue;
                        }
                        let (a, b) = (polygon[i], polygon[i + 1]);
                        let (c, d) = (polygon[j], polygon[(j + 1) % n]);
                        assert!(
                            !cross(a, b, c, d),
                            "{ch}'s outline crosses itself at {w}x{h}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cutting_overlaps_keeps_every_pixel() {
    for (w, h, t) in CELLS {
        for code in 0x2500..=0x259F_u32 {
            let boxes = match code {
                0x2504..=0x250B | 0x254C..=0x254F => dashes(code, w, h, t),
                0x256D..=0x2573 | 0x2591..=0x2593 => continue,
                0x2500..=0x257F => lines(LINES[(code - 0x2500) as usize], w, h, t),
                _ => blocks(code, w, h),
            };
            let kept: Vec<Corners> = rects(boxes.clone(), 1.)
                .iter()
                .filter_map(corners)
                .collect();
            assert_eq!(pixels(&kept), pixels(&boxes), "U+{code:04X} at {w}x{h}");
        }
    }
}

#[test]
fn the_cross_cuts_its_second_band_around_the_first() {
    for (w, h, t) in CELLS {
        let shapes = shapes('╳', w, h, t);
        let [Shape::Fill(first), pieces @ ..] = shapes.as_slice() else {
            panic!("╳ is polygons");
        };
        assert_eq!(
            pieces.len(),
            2,
            "two pieces around the first band at {w}x{h}"
        );
        let n = outline::normal(first[0], first[1]);
        let line = |p: Point| n.0 * p.0 + n.1 * p.1;
        let centre = (line(first[0]) + line(first[3])) / 2.;
        for piece in pieces {
            let Shape::Fill(piece) = piece else {
                panic!("pieces are polygons");
            };
            for &p in piece {
                let off = (line(p) - centre).abs();
                assert!(
                    off >= t as f32 / 2. - 1e-3,
                    "{p:?} is inside the first band at {w}x{h}"
                );
            }
        }
    }
}

#[test]
fn straight_lines_sit_centered_at_their_weight() {
    let (w, h, t) = (16, 32, 2);
    let across = |ch: char, side: usize| edge(&shapes(ch, w, h, t), side, w, h);
    assert_eq!(across('─', 1), [(15, 17)]);
    assert_eq!(across('━', 1), [(14, 18)]);
    assert_eq!(across('═', 1), [(13, 15), (17, 19)]);
    assert_eq!(across('│', 2), [(7, 9)]);
    assert_eq!(across('┃', 2), [(6, 10)]);
    assert_eq!(across('║', 2), [(5, 7), (9, 11)]);
}

#[test]
fn every_arm_meets_the_straight_line_of_its_weight() {
    for (w, h, t) in CELLS {
        let across = [['│', '┃', '║'], ['─', '━', '═']];
        for code in 0x2500..=0x257F_u32 {
            let ch = char::from_u32(code).expect("a char");
            let weights = arms(ch);
            if weights == [0; 4] {
                continue;
            }
            let shapes = shapes(ch, w, h, t);
            for (side, weight) in weights.into_iter().enumerate() {
                let found = edge(&shapes, side, w, h);
                if weight == 0 {
                    assert!(found.is_empty(), "{ch} touches side {side} with no arm");
                    continue;
                }
                let straight = across[side % 2][usize::from(weight) - 1];
                let want = edge(&super::shapes(straight, w, h, t), side, w, h);
                assert_eq!(
                    found, want,
                    "{ch}'s side {side} misses {straight} in {w}x{h}"
                );
            }
        }
    }
}

#[test]
fn dashes_tile_with_one_gap_between_cells() {
    for (w, h, t) in CELLS {
        for (dashes, count) in [("┄┅┆┇", 3), ("┈┉┊┋", 4), ("╌╍╎╏", 2)] {
            for ch in dashes.chars() {
                let vertical = "┆┇┊┋╎╏".contains(ch);
                let side = if vertical { h } else { w };
                let drawn = shapes(ch, w, h, t);
                if side < 2 * count {
                    let solid = if vertical { '│' } else { '─' };
                    assert_eq!(drawn, shapes(solid, w, h, t), "{ch} at {w}x{h} is solid");
                    continue;
                }
                let mut runs: Vec<(u32, u32)> = drawn
                    .iter()
                    .filter_map(corners)
                    .map(|[x0, y0, x1, y1]| if vertical { (y0, y1) } else { (x0, x1) })
                    .collect();
                runs.sort();
                assert_eq!(runs.len() as u32, count, "{ch} at {w}x{h}");
                let gap = runs[1].0 - runs[0].1;
                assert!(gap > 0, "{ch} has no gap in {w}x{h}");
                for pair in runs.windows(2) {
                    assert_eq!(pair[1].0 - pair[0].1, gap, "{ch} gaps unevenly");
                }
                let lead = if vertical { 0 } else { gap / 2 };
                assert_eq!(runs[0].0, lead, "{ch} starts off center at {w}x{h}");
                let margins = runs[0].0 + side - runs[runs.len() - 1].1;
                assert_eq!(
                    margins, gap,
                    "{ch} breaks the rhythm across cells at {w}x{h}"
                );
            }
        }
    }
}

#[test]
fn halves_cover_the_cell_and_shades_wash_it() {
    for (w, h, t) in CELLS {
        let span = |ch: char| corners(&shapes(ch, w, h, t)[0]).expect("a rect");
        let (upper, lower) = (span('▀'), span('▄'));
        assert_eq!((upper[1], lower[3]), (0, h));
        assert!(upper[3] >= lower[1], "a gap between ▀ and ▄ at {w}x{h}");
        let (left, right) = (span('▌'), span('▐'));
        assert_eq!((left[0], right[2]), (0, w));
        assert!(left[2] >= right[0], "a gap between ▌ and ▐ at {w}x{h}");
        for (ch, alpha) in [('░', 0.25), ('▒', 0.5), ('▓', 0.75)] {
            let want = Shape::Rect {
                x: 0,
                y: 0,
                w,
                h,
                alpha,
            };
            assert_eq!(shapes(ch, w, h, t), [want]);
        }
    }
}

#[test]
fn powerline_fills_the_cell_and_mirrors() {
    let fill = |ch: char| match shapes(ch, 16, 32, 2).as_slice() {
        [Shape::Fill(polygon)] => polygon.clone(),
        other => panic!("{ch} is one polygon, not {other:?}"),
    };
    assert_eq!(fill('\u{E0B0}'), [(0., 0.), (16., 16.), (0., 32.)]);
    assert_eq!(fill('\u{E0B2}'), [(16., 0.), (0., 16.), (16., 32.)]);
    assert_eq!(fill('\u{E0B1}').len(), 6, "a mitered chevron");
    assert_eq!(fill('\u{E0B6}')[0], (16., 0.));
    let outline = fill('\u{E0B5}');
    let low = outline.iter().map(|p| p.1).fold(f32::MAX, f32::min);
    let high = outline.iter().map(|p| p.1).fold(f32::MIN, f32::max);
    let right = outline.iter().map(|p| p.0).fold(f32::MIN, f32::max);
    assert!(
        low.abs() < 1e-3 && (high - 32.).abs() < 1e-3,
        "{low}..{high}"
    );
    assert!(
        right <= 16.01,
        "the outline stays in the half circle: {right}"
    );
}

#[test]
fn arcs_end_on_the_light_lines() {
    let (w, h, t) = (16, 32, 2);
    let drawn = shapes('╭', w, h, t);
    let [Shape::Fill(outline)] = drawn.as_slice() else {
        panic!("╭ is one polygon");
    };
    let near = |x: f32, y: f32| {
        outline
            .iter()
            .any(|p| (p.0 - x).abs() < 0.05 && (p.1 - y).abs() < 0.05)
    };
    assert!(near(7., 32.) && near(9., 32.), "the bottom end spans │");
    assert!(near(16., 15.) && near(16., 17.), "the right end spans ─");
}
