//! The path reader: each command of the small language, the ways numbers
//! may be written, what isn't a path, and damaged paths by the thousand.

use super::*;
use proptest::prelude::*;

fn points(path: &str) -> Vec<Vec<[f32; 2]>> {
    read(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .into_iter()
        .map(|outline| outline.points)
        .collect()
}

fn near(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3
}

#[test]
fn lines_are_read_as_places_and_as_steps() {
    assert_eq!(
        points("M3 2.5h8l2.5 2.5v8.5h-10.5z"),
        vec![vec![
            [3.0, 2.5],
            [11.0, 2.5],
            [13.5, 5.0],
            [13.5, 13.5],
            [3.0, 13.5]
        ]]
    );
    assert_eq!(
        points("M4 4L12 12H2V6"),
        vec![vec![[4.0, 4.0], [12.0, 12.0], [2.0, 12.0], [2.0, 6.0]]]
    );
    // Pairs after a move's first are lines; after a small move, steps.
    assert_eq!(
        points("M1 1 2 2 3 1"),
        vec![vec![[1.0, 1.0], [2.0, 2.0], [3.0, 1.0]]]
    );
    assert_eq!(
        points("m1 1 1 1 1 -1"),
        vec![vec![[1.0, 1.0], [2.0, 2.0], [3.0, 1.0]]]
    );
}

#[test]
fn a_closed_outline_says_so_and_the_pen_goes_back_to_its_start() {
    let read = read("M2 2h4v4zl-1 5").unwrap();
    assert!(read[0].closed);
    assert_eq!(read[0].points.len(), 3);
    assert!(!read[1].closed);
    assert_eq!(read[1].points, vec![[2.0, 2.0], [1.0, 7.0]]);
}

#[test]
fn each_move_starts_a_new_outline() {
    let read = points("M8 1v2M8 13v2M1 8h2");
    assert_eq!(read.len(), 3);
    assert_eq!(read[1], vec![[8.0, 13.0], [8.0, 15.0]]);
    // A move with nothing drawn after it leaves nothing.
    assert_eq!(points("M1 1M2 2h1").len(), 1);
    assert!(points("M1 1").is_empty());
    assert!(points("").is_empty());
    assert!(points("  ").is_empty());
}

#[test]
fn numbers_may_touch_each_other() {
    // The design's own: "v.5", "1.5.5", "-" starting the next number.
    assert_eq!(points("M8 11.5v.5"), vec![vec![[8.0, 11.5], [8.0, 12.0]]]);
    assert_eq!(points("M1.5.5l2-3"), vec![vec![[1.5, 0.5], [3.5, -2.5]]]);
    assert_eq!(points("M0,0 l 4 , 4"), vec![vec![[0.0, 0.0], [4.0, 4.0]]]);
}

#[test]
fn an_arc_s_flags_may_touch_what_follows() {
    // "0 010 7" is: not turned, the short way, clockwise, to (0, 7).
    let half = points("M10 6.5a3.5 3.5 0 010 7");
    let arc = &half[0];
    assert!(near(arc[0], [10.0, 6.5]));
    assert!(near(*arc.last().unwrap(), [10.0, 13.5]));
    // Clockwise from the top: it bulges to the right.
    assert!(arc.iter().any(|p| near(*p, [13.5, 10.0])), "{arc:?}");
    assert!(arc.iter().all(|p| p[0] >= 9.999));
}

#[test]
fn two_arcs_make_a_whole_circle() {
    let circle = &points("M2.5 7a4.5 4.5 0 1 0 9 0a4.5 4.5 0 1 0 -9 0z")[0];
    for p in circle {
        let far = ((p[0] - 7.0).powi(2) + (p[1] - 7.0).powi(2)).sqrt();
        assert!((far - 4.5).abs() < 1e-3, "{p:?} is {far} from the centre");
    }
    // All the way round: every side is reached.
    for side in [[2.5, 7.0], [11.5, 7.0], [7.0, 2.5], [7.0, 11.5]] {
        assert!(circle.iter().any(|p| near(*p, side)), "{side:?}");
    }
}

#[test]
fn an_arc_too_small_to_reach_is_grown_until_it_does() {
    let arc = &points("M0 0a1 1 0 0 1 8 0")[0];
    assert!(near(*arc.last().unwrap(), [8.0, 0.0]));
    assert!(arc.iter().any(|p| near(*p, [4.0, -4.0])), "{arc:?}");
}

#[test]
fn an_arc_with_no_radius_or_nowhere_to_go_is_a_line_or_nothing() {
    assert_eq!(
        points("M0 0a0 5 0 0 1 4 4"),
        vec![vec![[0.0, 0.0], [4.0, 4.0]]]
    );
    assert!(points("M3 3a2 2 0 1 1 0 0").is_empty());
}

#[test]
fn a_curve_starts_and_ends_where_it_says() {
    let curve = &points("M2 13C6 13 6 3 14 3")[0];
    assert!(near(curve[0], [2.0, 13.0]));
    assert!(near(*curve.last().unwrap(), [14.0, 3.0]));
    // It leaves level, along its first handle.
    assert!((curve[1][1] - 13.0).abs() < 0.3);
    assert!(curve.windows(2).all(|w| w[1][0] > w[0][0]));
}

#[test]
fn a_smooth_curve_mirrors_the_handle_before_it() {
    // The design's wave: the second curve leaves as the first arrived.
    let wave = &points("M2 12c2-6 4 2 6-4s4 2 6-4")[0];
    assert!(near(*wave.last().unwrap(), [14.0, 4.0]));
    let join = wave.iter().position(|p| near(*p, [8.0, 8.0])).unwrap();
    let before = [
        wave[join][0] - wave[join - 1][0],
        wave[join][1] - wave[join - 1][1],
    ];
    let after = [
        wave[join + 1][0] - wave[join][0],
        wave[join + 1][1] - wave[join][1],
    ];
    let cross = before[0] * after[1] - before[1] * after[0];
    assert!(cross.abs() < 0.02, "the join has a corner: {cross}");
    // With no curve before it, the first handle is the pen itself.
    let alone = &points("M0 0S4 4 8 0")[0];
    assert!(near(*alone.last().unwrap(), [8.0, 0.0]));
}

#[test]
fn what_is_not_a_path_is_an_error_with_its_place() {
    for (path, at, what) in [
        ("L3 3", 0, "a path starts with a move"),
        ("3 3", 0, "a command's letter was expected"),
        ("M3", 2, "a number was expected"),
        ("M3 x", 3, "a number was expected"),
        ("M1 1 Q2 2 3 3", 5, "a command the reader doesn't know"),
        ("M1 1a2 2 0 2 1 3 3", 11, "an arc's flag must be 0 or 1"),
        ("M1 1a2 2 30 0 1 3 3", 4, "a turned ellipse isn't read"),
        ("M1 1l-", 5, "a number was expected"),
        ("M1 1l.", 5, "a number was expected"),
        ("M1 1 é", 5, "a command's letter was expected"),
    ] {
        assert_eq!(read(path), Err(PathError { at, what }), "{path}");
    }
    let said = read("M3").unwrap_err().to_string();
    assert_eq!(said, "a number was expected (at letter 3)");
}

#[test]
fn a_path_that_goes_on_for_ever_is_refused() {
    let endless = format!("M0 0{}", "l1 1".repeat(MOST_POINTS + 1));
    assert_eq!(
        read(&endless).unwrap_err().what,
        "the path has too many corners"
    );
    // A number too large to be one isn't a place.
    assert!(read(&format!("M0 0h{}", "9".repeat(60))).is_err());
}

/// Real paths from the table, to damage.
const GOOD: [&str; 6] = [
    "M8 1.5l5.5 3v7L8 14.5 2.5 11.5v-7z",
    "M2.5 7a4.5 4.5 0 1 0 9 0a4.5 4.5 0 1 0 -9 0z",
    "M1.5 4.5a1 1 0 011-1h3.5l1.5 1.5h6a1 1 0 011 1v6.5a1 1 0 01-1 1h-11a1 1 0 01-1-1z",
    "M2 12c2-6 4 2 6-4s4 2 6-4",
    "M13 8a5 5 0 11-1.5-3.6M13 2.5v3h-3",
    "M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z",
];

/// What every read path must be, however it was damaged.
fn is_sound(outlines: &[Outline]) {
    let mut count = 0;
    for outline in outlines {
        assert!(outline.points.len() > 1);
        count += outline.points.len();
        for p in &outline.points {
            assert!(p[0].is_finite() && p[1].is_finite(), "{p:?}");
        }
    }
    assert!(count <= MOST_POINTS + CURVE_PIECES + 48);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn any_text_at_all_is_read_or_refused_never_a_crash(text in ".{0,80}") {
        if let Ok(outlines) = read(&text) {
            is_sound(&outlines);
        }
    }

    #[test]
    fn text_made_of_a_path_s_own_letters_is_read_or_refused(
        text in "[MmLlHhVvCcSsAaZz0-9 .,+-]{0,60}"
    ) {
        if let Ok(outlines) = read(&text) {
            is_sound(&outlines);
        }
    }

    #[test]
    fn a_damaged_path_is_read_or_refused(
        which in 0..GOOD.len(),
        at in 0usize..90,
        with in "[MLHVCSAZz0-9 .eE+-]{0,4}",
        cut in 0usize..4,
    ) {
        let good = GOOD[which];
        let at = at.min(good.len());
        let end = (at + cut).min(good.len());
        let damaged = format!("{}{}{}", &good[..at], with, &good[end..]);
        if let Ok(outlines) = read(&damaged) {
            is_sound(&outlines);
        }
    }
}
