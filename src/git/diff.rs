use std::{ffi::OsStr, path::Path};

use anyhow::{Context as _, bail, ensure};
use ely_gpui_component::git::{ChangeSection, DiffLine, LineKind, Stretch, mark_words};

use super::{Change, git, line, said, warnings};

/// `change`'s diff as `section` holds it, in Ely's stretches; none for a binary file.
pub fn diff(
    top: &Path,
    change: &Change,
    section: ChangeSection,
) -> anyhow::Result<Option<Vec<Stretch>>> {
    let (path, shown) = (change.path.as_os_str(), change.path.display());
    let flags: &[&str] = match section {
        ChangeSection::Staged => &["--cached", "-M", "--"],
        ChangeSection::Unstaged => &["--"],
        // HEAD is our side: a file we deleted reads as new.
        ChangeSection::Conflicted => &["HEAD", "--"],
        ChangeSection::Untracked if path.as_encoded_bytes().ends_with(b"/") => {
            bail!("git: {shown} is a folder")
        }
        ChangeSection::Untracked => &["--no-index", "--", "/dev/null"],
        ChangeSection::Ignored => bail!("git: {shown} is ignored"),
    };
    let mut args: Vec<&OsStr> = ["diff", "--no-color", "--no-ext-diff", "--no-textconv"]
        .into_iter()
        .chain(flags.iter().copied())
        .map(OsStr::new)
        .collect();
    args.push(path);
    args.extend(change.from.as_deref().map(Path::as_os_str));
    let output = git(top, &args, &[])?;
    // Without an index git exits 1 for a difference and a failure alike; only a difference prints.
    let differ = section == ChangeSection::Untracked
        && output.status.code() == Some(1)
        && !output.stdout.is_empty();
    ensure!(
        output.status.success() || differ,
        "git: `git {}` failed in {}: {}",
        line(&args),
        top.display(),
        said(&output)
    );
    warnings(top, &args, &output);
    hunks(&String::from_utf8_lossy(&output.stdout))
}

/// A hunk header's first old and new line numbers.
fn starts(header: &str) -> Option<(usize, usize)> {
    let mut ranges = header.strip_prefix("@@ -")?.split(' ');
    let first = |range: &str| range.split(',').next()?.parse().ok();
    let old = first(ranges.next()?)?;
    let new = first(ranges.next()?.strip_prefix('+')?)?;
    Some((old, new))
}

/// Git's unified diff as Ely's stretches, changed words marked; none for a binary file.
pub(super) fn hunks(text: &str) -> anyhow::Result<Option<Vec<Stretch>>> {
    let (mut stretches, mut at, mut inside) = (Vec::new(), (0, 0), false);
    for line in text.lines() {
        if line.starts_with("diff ") {
            inside = false;
        } else if line.starts_with("@@") {
            at = starts(line).with_context(|| format!("git: hunk header {line:?}"))?;
            let header = line.to_string().into();
            stretches.push(Stretch::Hunk {
                header,
                lines: Vec::new(),
            });
            inside = true;
        } else if !inside {
            if line.starts_with("Binary files ") {
                return Ok(None);
            }
        } else if let Some(Stretch::Hunk { lines, .. }) = stretches.last_mut() {
            let (kind, text) = match line.as_bytes().first() {
                // `diff.suppressBlankEmpty` drops a blank line's space.
                None => (LineKind::Same, ""),
                Some(b' ') => (LineKind::Same, &line[1..]),
                Some(b'-') => (LineKind::Removed, &line[1..]),
                Some(b'+') => (LineKind::Added, &line[1..]),
                Some(b'\\') => continue,
                Some(_) => bail!("git: diff line {line:?} is malformed"),
            };
            let old = (kind != LineKind::Added).then_some(at.0);
            let new = (kind != LineKind::Removed).then_some(at.1);
            at = (
                at.0 + usize::from(old.is_some()),
                at.1 + usize::from(new.is_some()),
            );
            lines.push(DiffLine {
                kind,
                old,
                new,
                text: text.to_string().into(),
                words: Vec::new(),
            });
        }
    }
    for stretch in &mut stretches {
        if let Stretch::Hunk { lines, .. } = stretch {
            mark_words(lines);
        }
    }
    Ok(Some(stretches))
}
