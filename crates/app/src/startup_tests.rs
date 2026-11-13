//! The splash shows for its minimum time, and no longer.

use super::*;

const MS: Duration = Duration::from_millis(1);

#[test]
fn the_splash_shows_for_at_least_its_time() {
    assert!(!done(Duration::ZERO));
    assert!(!done(100 * MS), "too soon");
    assert!(!done(SPLASH_AT_LEAST - MS), "just too soon");
    assert!(done(SPLASH_AT_LEAST));
    assert!(
        done(10 * SPLASH_AT_LEAST),
        "a slow first frame opens at once"
    );
}
