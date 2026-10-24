use alacritty_terminal::{
    event::VoidListener,
    term::{Config, test::TermSize},
    vte::ansi::Processor,
};

use super::*;

const PLAIN: FindOptions = FindOptions {
    case: false,
    word: false,
    regex: false,
};

fn term(bytes: &str, columns: usize, lines: usize, history: usize) -> Term<VoidListener> {
    let config = Config {
        scrolling_history: history,
        ..Config::default()
    };
    let mut term = Term::new(config, &TermSize::new(columns, lines), VoidListener);
    feed(&mut term, bytes);
    term
}

fn feed(term: &mut Term<VoidListener>, bytes: &str) {
    let mut parser: Processor = Processor::new();
    parser.advance(term, bytes.as_bytes());
}

fn found(term: &Term<VoidListener>, query: &str, options: FindOptions) -> Found {
    Found::search(term, query, options).expect("a pattern")
}

fn at(line: i32, column: usize) -> Point {
    Point::new(Line(line), Column(column))
}

#[test]
fn find_takes_case_words_and_patterns() {
    let term = term("one two One\r\nthree phone one", 20, 3, 10);
    let count =
        |query: &str, options| Found::search(&term, query, options).map(|found| found.hits.len());
    assert_eq!(count("one", PLAIN), Ok(4));
    assert_eq!(
        count(
            "One",
            FindOptions {
                case: true,
                ..PLAIN
            }
        ),
        Ok(1)
    );
    let word = FindOptions {
        word: true,
        ..PLAIN
    };
    assert_eq!(count("one", word), Ok(3), "phone holds no whole word");
    let regex = FindOptions {
        regex: true,
        ..PLAIN
    };
    assert_eq!(count("t[wh]", regex), Ok(2));
    assert_eq!(count("one$", regex), Ok(2), "a line ends after its text");
    assert_eq!(count("(", PLAIN), Ok(0), "plain text is escaped");
    assert!(count("(", regex).is_err());
}

#[test]
fn marks_wide_cells_and_unicode_words_are_found_where_they_show() {
    let shown = "cafe\u{301} caf\u{e9} \u{4e2d}\u{6587} \x1b[8msecret\x1b[0m";
    let line = term(shown, 30, 2, 10);
    let hits = |query: &str, options| found(&line, query, options).hits;
    assert_eq!(
        hits("cafe\u{301}", PLAIN),
        [at(0, 0)..=at(0, 3)],
        "a mark rides its cell"
    );
    let word = FindOptions {
        word: true,
        ..PLAIN
    };
    assert_eq!(hits("caf\u{e9}", word), [at(0, 5)..=at(0, 8)]);
    assert_eq!(hits("\u{4e2d}\u{6587}", word), [at(0, 10)..=at(0, 12)]);
    assert!(hits("secret", PLAIN).is_empty(), "hidden text stays hidden");
    let after = term("\u{4e2d}x", 10, 2, 10);
    let x = found(&after, "x", PLAIN).hits;
    assert_eq!(
        x,
        [at(0, 2)..=at(0, 2)],
        "a wide cell's spacer holds no text"
    );
}

#[test]
fn a_wrapped_line_is_one_line_mapped_back_to_its_cells() {
    let term = term("12345678needle", 10, 3, 10);
    assert_eq!(found(&term, "needle", PLAIN).hits, [at(0, 8)..=at(1, 3)]);
    let regex = FindOptions {
        regex: true,
        ..PLAIN
    };
    assert_eq!(found(&term, "needle$", regex).hits.len(), 1);
}

#[test]
fn the_current_match_follows_its_line() {
    let mut erased = term("needle OLD\r\nneedle KEEP\r\nneedle LAST", 20, 4, 10);
    let then = found(&erased, "needle", PLAIN);
    feed(&mut erased, "\x1b[1;1H\x1b[2K");
    let now = found(&erased, "needle", PLAIN);
    assert_eq!(now.follow(&then, 1), Some(0), "KEEP, though OLD is gone");

    let mut ring = term("n 1\r\nn 2\r\nn 3\r\nn 4\r\nn 5", 10, 3, 2);
    let then = found(&ring, "n", PLAIN);
    feed(&mut ring, "\r\nn 6");
    let now = found(&ring, "n", PLAIN);
    assert_eq!(
        now.follow(&then, 2),
        Some(1),
        "n 3, a line up, though n 1 left"
    );

    let mut same = term("n\r\nn\r\nn", 10, 3, 10);
    let then = found(&same, "n", PLAIN);
    feed(&mut same, "\r\nx\r\nx");
    let now = found(&same, "n", PLAIN);
    assert_eq!(
        now.follow(&then, 1),
        Some(1),
        "of like lines, the one that scrolled"
    );

    let mut cleared = term("n\r\nn\r\nn\r\nn\r\nn", 10, 3, 10);
    let then = found(&cleared, "n", PLAIN);
    feed(&mut cleared, "\x1b[3J");
    let now = found(&cleared, "n", PLAIN);
    assert_eq!(
        now.follow(&then, 3),
        Some(1),
        "a cleared history moves no line"
    );

    let mut gone = term("needle A\r\nneedle B\r\n\r\nneedle C", 20, 5, 10);
    let then = found(&gone, "needle", PLAIN);
    feed(&mut gone, "\x1b[2;1H\x1b[2K");
    let now = found(&gone, "needle", PLAIN);
    assert_eq!(now.follow(&then, 1), Some(0), "B is gone: A is nearest");

    let mut wide = term(&format!("{}needle", "x".repeat(15)), 12, 4, 10);
    let then = found(&wide, "needle", PLAIN);
    wide.resize(TermSize::new(30, 4));
    let now = found(&wide, "needle", PLAIN);
    assert_eq!(now.follow(&then, 0), Some(0));
    assert_eq!(now.hits[0].start().column, Column(15), "the reflowed cells");
    assert_ne!(now.hits[0].start(), then.hits[0].start());
}

#[test]
fn a_line_that_grows_or_changes_keeps_the_match_at_its_place() {
    let mut grown = term("needle A needle B", 30, 3, 10);
    let then = found(&grown, "needle", PLAIN);
    feed(&mut grown, "!");
    let now = found(&grown, "needle", PLAIN);
    assert_eq!(
        now.follow(&then, 1),
        Some(1),
        "the same text runs through it"
    );

    let mut ring = term(
        "needle 1\r\nneedle 2\r\nneedle 3\r\nneedle 4\r\nneedle 5",
        10,
        3,
        2,
    );
    let then = found(&ring, "needle", PLAIN);
    feed(&mut ring, "!\r\nneedle 6");
    let now = found(&ring, "needle", PLAIN);
    assert_eq!(now.follow(&then, 4), Some(3), "needle 5!, a line up");

    let mut both = term(
        "needle 1\r\nneedle 2\r\nneedle 5\r\nneedle 6\r\nneedle 60",
        10,
        3,
        2,
    );
    let then = found(&both, "needle", PLAIN);
    feed(&mut both, "\r\nx");
    let now = found(&both, "needle", PLAIN);
    assert_eq!(
        now.follow(&then, 3),
        Some(2),
        "needle 6 itself before needle 60 in its place"
    );

    let mut twins = term("needle\r\nneedle", 20, 3, 10);
    let then = found(&twins, "needle", PLAIN);
    feed(&mut twins, "!");
    let now = found(&twins, "needle", PLAIN);
    assert_eq!(
        now.follow(&then, 1),
        Some(1),
        "the grown line, not its twin above"
    );

    let mut rewritten = term("needle\r\nA\r\nneedle", 20, 4, 10);
    let then = found(&rewritten, "needle", PLAIN);
    feed(&mut rewritten, "\x1b[2;1HB\x1b[3;7H!");
    let now = found(&rewritten, "needle", PLAIN);
    assert_eq!(
        now.follow(&then, 1),
        Some(1),
        "grown in its place, though the line above changed"
    );

    let mut changed = term("needle A needle B", 30, 3, 10);
    let then = found(&changed, "needle", PLAIN);
    feed(&mut changed, "\rneedle X needle Y");
    let now = found(&changed, "needle", PLAIN);
    assert_eq!(now.follow(&then, 1), Some(1), "else the nearest column");
}

#[test]
fn following_a_long_line_takes_one_pass() {
    let mut long = term(&"a".repeat(1_000_000), 147, 48, 10_000);
    let then = found(&long, "a", PLAIN);
    feed(&mut long, "!");
    let now = found(&long, "a", PLAIN);
    let last = then.hits.len() - 1;
    let clock = std::time::Instant::now();
    assert_eq!(now.follow(&then, last), Some(last));
    let took = clock.elapsed();
    assert!(took.as_secs() < 5, "a quadratic follow took {took:?}");
}

#[test]
fn steps_go_round_the_ends() {
    assert_eq!(stepped(Some(0), 3, false), Some(2));
    assert_eq!(stepped(Some(2), 3, true), Some(0));
    assert_eq!(stepped(Some(1), 3, false), Some(0));
    assert_eq!(stepped(None, 0, true), None);
}
