//! Renders the icon: `icons iconset SVG FOLDER` or `icons ico SVG SMALL FILE`.

use anyhow::{Context as _, bail};
use resvg::{tiny_skia, usvg};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let form = args.next().context("icons: name a form, iconset or ico")?;
    let svg = args.next().context("icons: name an SVG to render")?;
    let tree = read(&svg)?;
    match form.as_str() {
        "iconset" => {
            let out = args.next().context("icons: name where to write")?;
            iconset(&tree, &out)?;
            println!("icons: {svg} drawn into {out}");
        }
        "ico" => {
            let small = args
                .next()
                .context("icons: name the SVG the small sizes take")?;
            let out = args.next().context("icons: name where to write")?;
            ico(&tree, &read(&small)?, &out)?;
            println!("icons: {svg}, with {small} for the small sizes, drawn into {out}");
        }
        other => bail!("icons: no form {other}; iconset or ico"),
    }
    Ok(())
}

fn read(svg: &str) -> anyhow::Result<usvg::Tree> {
    let data = std::fs::read(svg).with_context(|| format!("icons: cannot read {svg}"))?;
    usvg::Tree::from_data(&data, &usvg::Options::default())
        .with_context(|| format!("icons: {svg} is not an SVG"))
}

/// A macOS iconset: each size at 1x and 2x.
fn iconset(tree: &usvg::Tree, out: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(out).with_context(|| format!("icons: cannot make {out}"))?;
    for points in [16, 32, 128, 256, 512] {
        for (scale, suffix) in [(1, ""), (2, "@2x")] {
            let png = std::path::Path::new(out).join(format!("icon_{points}x{points}{suffix}.png"));
            std::fs::write(&png, draw(tree, points * scale)?)
                .with_context(|| format!("icons: cannot write {}", png.display()))?;
        }
    }
    Ok(())
}

/// A Windows icon: one PNG per size, as Windows reads since Vista; `small` draws the sizes under 48.
fn ico(tree: &usvg::Tree, small: &usvg::Tree, out: &str) -> anyhow::Result<()> {
    let sides = [16, 24, 32, 48, 64, 128, 256];
    let images = sides
        .iter()
        .map(|&side| draw(if side < 48 { small } else { tree }, side))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut file = vec![0, 0, 1, 0];
    file.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (side, png) in sides.iter().zip(&images) {
        // The format writes 256 as 0.
        let edge = (side % 256) as u8;
        file.extend([edge, edge, 0, 0]);
        file.extend(1u16.to_le_bytes());
        file.extend(32u16.to_le_bytes());
        file.extend((png.len() as u32).to_le_bytes());
        file.extend((offset as u32).to_le_bytes());
        offset += png.len();
    }
    for png in &images {
        file.extend(png);
    }
    std::fs::write(out, file).with_context(|| format!("icons: cannot write {out}"))
}

/// The SVG drawn into a square PNG `side` pixels wide.
fn draw(tree: &usvg::Tree, side: u32) -> anyhow::Result<Vec<u8>> {
    let mut pixmap = tiny_skia::Pixmap::new(side, side).context("icons: no pixmap")?;
    let fit = side as f32 / tree.size().width();
    let transform = tiny_skia::Transform::from_scale(fit, fit);
    resvg::render(tree, transform, &mut pixmap.as_mut());
    pixmap.encode_png().context("icons: cannot encode a PNG")
}
