use super::*;

fn v(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// An L: a 4 × 4 square with its top-right 2 × 2 quarter cut away.
fn ell() -> Vec<Vec2> {
    vec![
        v(0.0, 0.0),
        v(4.0, 0.0),
        v(4.0, 2.0),
        v(2.0, 2.0),
        v(2.0, 4.0),
        v(0.0, 4.0),
    ]
}

/// Whether `p` is in any piece (each convex, counter-clockwise).
fn in_pieces(pieces: &[Vec<Vec2>], p: Vec2) -> bool {
    pieces.iter().any(|piece| {
        let n = piece.len();
        (0..n).all(|i| side(piece[i], piece[(i + 1) % n], p) >= 0.0)
    })
}

#[test]
fn a_square_is_one_piece() {
    let square = [v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)];
    assert_eq!(check(&square), Ok(()));
    let pieces = convex_pieces(&square);
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].len(), 4);
}

#[test]
fn a_concave_shape_is_split_into_convex_pieces_covering_it_exactly() {
    let shape = ell();
    assert_eq!(check(&shape), Ok(()));
    let pieces = convex_pieces(&shape);
    assert!(pieces.len() >= 2, "an L needs two convex pieces at least");
    let area: f32 = pieces.iter().map(|p| twice_area(p)).sum();
    assert!(
        (area - twice_area(&shape)).abs() < 1e-4,
        "the pieces cover it once"
    );
    // Inside and outside, checked on a fine grid, including edges.
    for i in -4..=36 {
        for j in -4..=36 {
            let p = v(i as f32 * 0.125, j as f32 * 0.125);
            assert_eq!(in_pieces(&pieces, p), contains(&shape, p), "at {p}");
        }
    }
    // The notch is outside; both arms are inside.
    assert!(!contains(&shape, v(3.0, 3.0)));
    assert!(contains(&shape, v(3.0, 1.0)));
    assert!(contains(&shape, v(1.0, 3.0)));
}

#[test]
fn clockwise_outlines_work_the_same() {
    let mut shape = ell();
    shape.reverse();
    let pieces = convex_pieces(&shape);
    assert!(!pieces.is_empty());
    assert!(pieces.iter().all(|p| twice_area(p) > 0.0));
    assert!(!in_pieces(&pieces, v(3.0, 3.0)));
    assert!(in_pieces(&pieces, v(1.0, 1.0)));
}

#[test]
fn a_shape_crossing_itself_is_refused_with_a_reason() {
    let bow_tie = [v(0.0, 0.0), v(2.0, 2.0), v(2.0, 0.0), v(0.0, 2.0)];
    let refused = check(&bow_tie);
    assert!(matches!(refused, Err(OutlineError::Crosses(..))));
    let why = refused.unwrap_err().to_string();
    assert!(why.contains("cross itself"), "{why}");
    assert!(convex_pieces(&bow_tie).is_empty());
}

#[test]
fn edges_that_only_touch_are_refused_too() {
    // The fifth point sits on the first edge.
    let touching = [
        v(0.0, 0.0),
        v(4.0, 0.0),
        v(4.0, 4.0),
        v(2.0, 4.0),
        v(2.0, 0.0),
        v(0.0, 4.0),
    ];
    assert!(matches!(check(&touching), Err(OutlineError::Crosses(..))));
}

#[test]
fn too_few_too_many_and_odd_points_are_refused() {
    assert_eq!(
        check(&[v(0.0, 0.0), v(1.0, 0.0)]),
        Err(OutlineError::TooFew)
    );
    let many: Vec<Vec2> = (0..=MAX_OUTLINE_POINTS)
        .map(|i| Vec2::from_angle(i as f32 * 0.01))
        .collect();
    assert_eq!(check(&many), Err(OutlineError::TooMany));
    assert_eq!(
        check(&[v(0.0, 0.0), v(f32::NAN, 0.0), v(0.0, 1.0)]),
        Err(OutlineError::OutOfRange)
    );
    assert_eq!(
        check(&[v(0.0, 0.0), v(1e9, 0.0), v(0.0, 1.0)]),
        Err(OutlineError::OutOfRange)
    );
    assert_eq!(
        check(&[v(0.0, 0.0), v(0.0, 0.0), v(1.0, 1.0)]),
        Err(OutlineError::SamePlace(0))
    );
    assert!(check(&[v(0.0, 0.0), v(1.0, 0.0), v(2.0, 0.0)]).is_err());
}

#[test]
fn points_in_a_straight_line_along_an_edge_are_fine() {
    let square = [
        v(0.0, 0.0),
        v(1.0, 0.0),
        v(2.0, 0.0),
        v(2.0, 2.0),
        v(0.0, 2.0),
    ];
    assert_eq!(check(&square), Ok(()));
    let pieces = convex_pieces(&square);
    let area: f32 = pieces.iter().map(|p| twice_area(p)).sum();
    assert!((area - 8.0).abs() < 1e-5);
}

#[test]
fn a_big_wavy_shape_makes_few_pieces_quickly() {
    // A star of 256 points: deeply concave.
    let star: Vec<Vec2> = (0..MAX_OUTLINE_POINTS)
        .map(|i| {
            let r = if i % 2 == 0 { 10.0 } else { 6.0 };
            Vec2::from_angle(i as f32 / MAX_OUTLINE_POINTS as f32 * std::f32::consts::TAU) * r
        })
        .collect();
    assert_eq!(check(&star), Ok(()));
    let started = std::time::Instant::now();
    let pieces = convex_pieces(&star);
    let took = started.elapsed();
    assert!(pieces.len() < MAX_OUTLINE_POINTS, "{} pieces", pieces.len());
    let area: f32 = pieces.iter().map(|p| twice_area(p)).sum();
    assert!((area - twice_area(&star)).abs() / twice_area(&star) < 1e-4);
    // Generous, for a slow debug build on a busy machine.
    assert!(took.as_millis() < 2_000, "{took:?}");
}

#[test]
fn the_same_outline_always_gives_the_same_pieces() {
    assert_eq!(convex_pieces(&ell()), convex_pieces(&ell()));
}

#[test]
fn random_outlines_never_crash_and_usable_ones_cover_their_inside() {
    // A small fixed generator: the same every run.
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 2001) as f32 / 100.0 - 10.0
    };
    for _ in 0..500 {
        let n = 3 + (next().abs() as usize % 9);
        let points: Vec<Vec2> = (0..n).map(|_| v(next(), next())).collect();
        let pieces = convex_pieces(&points);
        if check(&points).is_ok() {
            let area: f32 = pieces.iter().map(|p| twice_area(p)).sum();
            let whole = twice_area(&points).abs();
            assert!((area - whole).abs() <= whole * 1e-3 + 1e-3, "{points:?}");
        } else {
            assert!(pieces.is_empty());
        }
    }
}
