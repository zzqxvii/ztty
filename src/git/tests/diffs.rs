use std::ops::Range;

use ely_gpui_component::git::DiffLine;

use super::*;
use crate::git::diff::hunks;

fn line(kind: LineKind, at: (Option<usize>, Option<usize>), text: &str) -> DiffLine {
    lined(kind, at, text, Vec::new())
}

fn lined(
    kind: LineKind,
    (old, new): (Option<usize>, Option<usize>),
    text: &str,
    words: Vec<Range<usize>>,
) -> DiffLine {
    DiffLine {
        kind,
        old,
        new,
        text: text.to_string().into(),
        words,
    }
}

fn hunk(header: &str, lines: Vec<DiffLine>) -> Stretch {
    Stretch::Hunk {
        header: header.to_string().into(),
        lines,
    }
}

#[test]
fn diffs_read_as_ztty_stretches() {
    use LineKind::*;
    let root = repository("git-diff");
    write(&root, "a.txt", "one\nthe old line\nthree\n");
    write(&root, "old.txt", "kept\n");
    fs::write(root.join("bin.dat"), b"x\0y").expect("a binary file");
    prepare(&root, &["add", "."]);
    prepare(&root, &["commit", "-qm", "first"]);
    write(&root, "a.txt", "one\nthe new line\nthree\n");
    prepare(&root, &["add", "a.txt"]);
    write(&root, "a.txt", "one\nthe new line\nthree\nfour");
    fs::write(root.join("bin.dat"), b"x\0z").expect("a binary edit");
    prepare(&root, &["mv", "old.txt", "new.txt"]);
    write(&root, "fresh.txt", "fresh\nlines\n");
    write(&root, "notes/todo.txt", "todo\n");

    let a = change("a.txt", GitStatus::Modified);
    let staged = diff(&root, &a, ChangeSection::Staged).expect("a diff");
    let words = |kind, at, text| lined(kind, at, text, vec![Range { start: 4, end: 7 }]);
    let expected = hunk(
        "@@ -1,3 +1,3 @@",
        vec![
            line(Same, (Some(1), Some(1)), "one"),
            words(Removed, (Some(2), None), "the old line"),
            words(Added, (None, Some(2)), "the new line"),
            line(Same, (Some(3), Some(3)), "three"),
        ],
    );
    assert_eq!(staged, Some(vec![expected]));
    let unstaged = diff(&root, &a, ChangeSection::Unstaged).expect("a diff");
    let expected = hunk(
        "@@ -1,3 +1,4 @@",
        vec![
            line(Same, (Some(1), Some(1)), "one"),
            line(Same, (Some(2), Some(2)), "the new line"),
            line(Same, (Some(3), Some(3)), "three"),
            line(Added, (None, Some(4)), "four"),
        ],
    );
    assert_eq!(unstaged, Some(vec![expected]));
    let fresh = change("fresh.txt", GitStatus::Untracked);
    let untracked = diff(&root, &fresh, ChangeSection::Untracked).expect("a diff");
    let expected = hunk(
        "@@ -0,0 +1,2 @@",
        vec![
            line(Added, (None, Some(1)), "fresh"),
            line(Added, (None, Some(2)), "lines"),
        ],
    );
    assert_eq!(untracked, Some(vec![expected]));
    let binary = change("bin.dat", GitStatus::Modified);
    let binary = diff(&root, &binary, ChangeSection::Unstaged).expect("a diff");
    assert_eq!(binary, None, "a binary file has no lines");
    let moved = read(&root)
        .staged
        .into_iter()
        .find(|change| change.from.is_some());
    let moved = moved.expect("a staged rename");
    let renamed = diff(&root, &moved, ChangeSection::Staged).expect("a diff");
    assert_eq!(renamed, Some(Vec::new()), "a pure rename changes no line");
    let notes = change("notes/", GitStatus::Untracked);
    assert!(diff(&root, &notes, ChangeSection::Untracked).is_err());
}

/// A repository whose merge of `other`, which made c.txt `theirs`, stops on what `ours` did to it.
fn conflict(name: &str, ours: impl FnOnce(&Path)) -> PathBuf {
    let root = repository(name);
    write(&root, "c.txt", "base\n");
    prepare(&root, &["add", "."]);
    prepare(&root, &["commit", "-qm", "base"]);
    prepare(&root, &["checkout", "-qb", "other"]);
    write(&root, "c.txt", "theirs\n");
    prepare(&root, &["commit", "-qam", "theirs"]);
    prepare(&root, &["checkout", "-q", "main"]);
    ours(&root);
    prepare(&root, &["commit", "-qam", "ours"]);
    let merged = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["merge", "-q", "other"])
        .output()
        .expect("git runs");
    assert!(!merged.status.success(), "the merge stops");
    root
}

#[test]
fn a_conflict_diffs_against_head_our_side() {
    use LineKind::*;
    let root = conflict("git-conflict", |root| write(root, "c.txt", "ours\n"));
    let state = read(&root);
    assert_eq!(state.conflicted, [change("c.txt", GitStatus::Conflicted)]);
    let ours = diff(&root, &state.conflicted[0], ChangeSection::Conflicted).expect("a diff");
    let Some([Stretch::Hunk { lines, .. }]) = ours.as_deref() else {
        panic!("one hunk: {ours:?}");
    };
    let shown: Vec<_> = lines
        .iter()
        .map(|line| (line.kind, line.text.as_ref()))
        .collect();
    let expected = [
        (Added, "<<<<<<< HEAD"),
        (Same, "ours"),
        (Added, "======="),
        (Added, "theirs"),
        (Added, ">>>>>>> other"),
    ];
    assert_eq!(shown, expected);

    let root = conflict("git-conflict-deleted", |root| {
        prepare(root, &["rm", "-q", "c.txt"])
    });
    let state = read(&root);
    assert_eq!(state.conflicted, [change("c.txt", GitStatus::Conflicted)]);
    let theirs = diff(&root, &state.conflicted[0], ChangeSection::Conflicted).expect("a diff");
    let expected = hunk(
        "@@ -0,0 +1 @@",
        vec![line(Added, (None, Some(1)), "theirs")],
    );
    assert_eq!(
        theirs,
        Some(vec![expected]),
        "a file we deleted reads as new"
    );
}

#[test]
fn a_diff_runs_no_text_converter() {
    use LineKind::*;
    let root = repository("git-textconv");
    write(&root, "a.txt", "old\n");
    write(&root, ".gitattributes", "a.txt diff=review\n");
    prepare(&root, &["add", "."]);
    prepare(&root, &["commit", "-qm", "first"]);
    // Git dies when a converter fails, so this one would fail the diff it ran in.
    prepare(&root, &["config", "diff.review.textconv", "false"]);
    write(&root, "a.txt", "new\n");
    let a = change("a.txt", GitStatus::Modified);
    let expected = hunk(
        "@@ -1 +1 @@",
        vec![
            line(Removed, (Some(1), None), "old"),
            line(Added, (None, Some(1)), "new"),
        ],
    );
    let unstaged = diff(&root, &a, ChangeSection::Unstaged).expect("a diff");
    assert_eq!(unstaged, Some(vec![expected]));
}

#[test]
fn an_untracked_diff_outlasts_a_warning_not_a_missing_file() {
    use LineKind::*;
    let root = repository("git-crlf");
    prepare(&root, &["config", "core.autocrlf", "true"]);
    write(&root, "fresh.txt", "fresh\n");
    let fresh = change("fresh.txt", GitStatus::Untracked);
    let untracked = diff(&root, &fresh, ChangeSection::Untracked).expect("a diff git warned of");
    let expected = hunk("@@ -0,0 +1 @@", vec![line(Added, (None, Some(1)), "fresh")]);
    assert_eq!(untracked, Some(vec![expected]));
    let gone = change("gone.txt", GitStatus::Untracked);
    assert!(diff(&root, &gone, ChangeSection::Untracked).is_err());
}

#[test]
fn hunks_number_lines_skip_markers_and_stop_at_the_next_file() {
    use LineKind::*;
    let text = "* Unmerged path a\ndiff --git a/a b/a\n--- a/a\n+++ b/a\n\
        @@ -3 +3,2 @@ fn main\n-x\r\n+y\n+z\n\\ No newline at end of file\n\
        diff --git a/b b/b\nnew file mode 100644\n--- /dev/null\n+++ b/b\n@@ -0,0 +1 @@\n+b\n";
    let first = hunk(
        "@@ -3 +3,2 @@ fn main",
        vec![
            line(Removed, (Some(3), None), "x"),
            line(Added, (None, Some(3)), "y"),
            line(Added, (None, Some(4)), "z"),
        ],
    );
    let second = hunk("@@ -0,0 +1 @@", vec![line(Added, (None, Some(1)), "b")]);
    assert_eq!(hunks(text).expect("hunks"), Some(vec![first, second]));
    let blank = hunk(
        "@@ -1,3 +1,3 @@",
        vec![
            line(Same, (Some(1), Some(1)), "a"),
            line(Same, (Some(2), Some(2)), ""),
            line(Removed, (Some(3), None), "b"),
            line(Added, (None, Some(3)), "c"),
        ],
    );
    let suppressed = "@@ -1,3 +1,3 @@\n a\n\n-b\n+c\n";
    assert_eq!(hunks(suppressed).expect("hunks"), Some(vec![blank]));
    let binary = "diff --git a/x b/x\nBinary files a/x and b/x differ\n";
    assert_eq!(hunks(binary).expect("a diff"), None);
    assert!(hunks("@@ -a +b @@\n").is_err(), "a header without numbers");
    assert!(
        hunks("@@ -1 +1 @@\n?x\n").is_err(),
        "a line git never writes"
    );
}
