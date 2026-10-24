use super::{Point, Shape};

/// Straight pieces a cubic curve is cut into.
const SEGMENTS: usize = 16;

/// A rounded corner from the light lines' centers to two edges.
pub(super) fn arc(code: u32, w: u32, h: u32, t: u32) -> Vec<Shape> {
    let (wf, hf, tf) = (w as f32, h as f32, t as f32);
    let cx = (w.saturating_sub(t) / 2) as f32 + tf / 2.;
    let cy = (h.saturating_sub(t) / 2) as f32 + tf / 2.;
    let r = wf.min(hf) / 2.;
    let s = 0.25;
    let [dy, dx]: [f32; 2] = [[1., 1.], [1., -1.], [-1., -1.], [-1., 1.]][(code - 0x256D) as usize];
    let (bend, end) = ((cx, cy + dy * r), (cx + dx * r, cy));
    let (lead_in, lead_out) = (
        (cx, if dy > 0. { hf } else { 0. }),
        (if dx > 0. { wf } else { 0. }, cy),
    );
    let mut path = Vec::new();
    if dy * (lead_in.1 - bend.1) > 0. {
        path.push(lead_in);
    }
    path.push(bend);
    path.extend(cubic(
        bend,
        [(cx, cy + dy * s * r), (cx + dx * s * r, cy), end],
    ));
    if dx * (lead_out.0 - end.0) > 0. {
        path.push(lead_out);
    }
    vec![Shape::Fill(stroke(&path, tf / 2.))]
}

/// Corner-to-corner bands, run half a pixel past so neighbors join once clipped.
pub(super) fn diagonals(code: u32, w: u32, h: u32, t: u32) -> Vec<Shape> {
    let (wf, hf, half) = (w as f32, h as f32, t as f32 / 2.);
    let (sx, sy) = ((wf / hf).min(1.) / 2., (hf / wf).min(1.) / 2.);
    let rising = [(wf + sx, -sy), (-sx, hf + sy)];
    let falling = [(-sx, -sy), (wf + sx, hf + sy)];
    match code {
        0x2571 => vec![Shape::Fill(stroke(&rising, half))],
        0x2572 => vec![Shape::Fill(stroke(&falling, half))],
        _ => {
            let band = stroke(&falling, half);
            let n = normal(rising[0], rising[1]);
            let at = n.0 * rising[0].0 + n.1 * rising[0].1;
            let before = clip(&band, (-n.0, -n.1), half - at);
            let after = clip(&band, n, at + half);
            [stroke(&rising, half), before, after]
                .into_iter()
                .filter(|polygon| polygon.len() >= 3)
                .map(Shape::Fill)
                .collect()
        }
    }
}

/// Powerline's arrows and half circles, filling the cell's height.
pub(super) fn powerline(code: u32, w: u32, h: u32, t: u32) -> Vec<Shape> {
    let (wf, hf, tf) = (w as f32, h as f32, t as f32);
    let arrow = vec![(0., 0.), (wf, hf / 2.), (0., hf)];
    let polygon = match code {
        0xE0B0 | 0xE0B2 => arrow,
        0xE0B1 | 0xE0B3 => stroke(&arrow, tf / 2.),
        0xE0B4 | 0xE0B6 => half_circle(wf, hf, 0.),
        _ => stroke(&half_circle(wf, hf, tf / 2.), tf / 2.),
    };
    let polygon = if matches!(code, 0xE0B2 | 0xE0B3 | 0xE0B6 | 0xE0B7) {
        polygon.into_iter().map(|(x, y)| (wf - x, y)).collect()
    } else {
        polygon
    };
    vec![Shape::Fill(polygon)]
}

/// The right half of a circle against the left edge, inset `by` so a stroke stays inside; its ends run straight a pixel, as Ghostty's do.
fn half_circle(w: f32, h: f32, by: f32) -> Vec<Point> {
    let c = (2f32.sqrt() - 1.) * 4. / 3.;
    let r = w.min(h / 2.) - by;
    let (top, bottom) = (by, h - by);
    let mut path = vec![(0., top), (1., top)];
    path.extend(cubic(
        (1., top),
        [(r * c, top), (r, top + r - r * c), (r, top + r)],
    ));
    path.push((r, bottom - r));
    path.extend(cubic(
        (r, bottom - r),
        [(r, bottom - r + r * c), (r * c, bottom), (1., bottom)],
    ));
    path.push((0., bottom));
    distinct(&path)
}

/// Points along a cubic after `from`, its end the last.
fn cubic(from: Point, [a, b, to]: [Point; 3]) -> impl Iterator<Item = Point> {
    (1..=SEGMENTS).map(move |step| {
        let t = step as f32 / SEGMENTS as f32;
        let u = 1. - t;
        let mix = |p: f32, q: f32, r: f32, s: f32| {
            u * u * u * p + 3. * u * u * t * q + 3. * u * t * t * r + t * t * t * s
        };
        (mix(from.0, a.0, b.0, to.0), mix(from.1, a.1, b.1, to.1))
    })
}

/// The path without points that repeat the one before.
fn distinct(path: &[Point]) -> Vec<Point> {
    let mut points: Vec<Point> = Vec::with_capacity(path.len());
    for &point in path {
        let repeat = points
            .last()
            .is_some_and(|last| (point.0 - last.0).hypot(point.1 - last.1) < 1e-3);
        if !repeat {
            points.push(point);
        }
    }
    points
}

/// The outline of `path` stroked `half` each side, with butt ends and mitered corners.
fn stroke(path: &[Point], half: f32) -> Vec<Point> {
    let points = distinct(path);
    assert!(points.len() >= 2, "a stroke needs two distinct points");
    let last = points.len() - 1;
    let offsets: Vec<Point> = (0..=last)
        .map(|index| {
            let before = normal(points[index.saturating_sub(1)], points[index.max(1)]);
            let after = normal(points[index.min(last - 1)], points[(index + 1).min(last)]);
            let mean = ((before.0 + after.0) / 2., (before.1 + after.1) / 2.);
            let scale = half / (mean.0 * mean.0 + mean.1 * mean.1);
            (mean.0 * scale, mean.1 * scale)
        })
        .collect();
    let ahead = points
        .iter()
        .zip(&offsets)
        .map(|(p, o)| (p.0 + o.0, p.1 + o.1));
    let back = points
        .iter()
        .zip(&offsets)
        .rev()
        .map(|(p, o)| (p.0 - o.0, p.1 - o.1));
    ahead.chain(back).collect()
}

/// The unit normal of the segment from `from` to `to`.
pub(super) fn normal(from: Point, to: Point) -> Point {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.hypot(dy);
    (-dy / length, dx / length)
}

/// The part of convex `polygon` where `n · p >= at`.
fn clip(polygon: &[Point], n: Point, at: f32) -> Vec<Point> {
    let side = |p: Point| n.0 * p.0 + n.1 * p.1 - at;
    let mut kept = Vec::new();
    for (index, &p) in polygon.iter().enumerate() {
        let q = polygon[(index + 1) % polygon.len()];
        let (sp, sq) = (side(p), side(q));
        if sp >= 0. {
            kept.push(p);
        }
        if (sp >= 0.) != (sq >= 0.) {
            let k = sp / (sp - sq);
            kept.push((p.0 + (q.0 - p.0) * k, p.1 + (q.1 - p.1) * k));
        }
    }
    kept
}
