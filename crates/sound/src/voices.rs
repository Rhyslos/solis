//! Choosing what's heard (Phase 7, S5; decision 0056, question 8): when
//! more sounds want to play than the project's voices allow (64 unless
//! the Mixer says otherwise), the most important are heard. Plain data
//! and sums: the app gathers the candidates each frame, and the device
//! plays what's chosen.
//!
//! - **Ranked** by the event's priority, then by how loud each would be
//!   at the listener, then by a key, so the same game always chooses the
//!   same.
//! - **Loops** (`AudioSource`s) that aren't chosen rest ("virtual"): not
//!   mixed, but their place in the sound is kept by the clock, so one
//!   coming back carries on where it would have been.
//! - **One event asked for many times in a tick** plays a few copies, a
//!   little louder, instead of hundreds ([`copies`]).

use crate::event::{Fade, Priority};

/// The fewest and most voices a project may set.
pub const VOICES_RANGE: (u32, u32) = (16, 256);
/// Voices unless the project says otherwise.
pub const STANDARD_VOICES: u32 = 64;
/// The most copies of one event started in one tick.
pub const MAX_COPIES: usize = 3;
/// The most a pile of copies is made louder, in decibels.
pub const MOST_PILE_DB: f32 = 6.0;

/// What a candidate is: a loop on an entity, or one of this frame's
/// requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// An entity's loop, by its ID's number.
    Loop(u64),
    /// This frame's request, by its place in the frame's list.
    Shot(usize),
}

/// A sound that wants to be heard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    /// Which one.
    pub key: Key,
    /// Its event's priority.
    pub priority: Priority,
    /// How loud it would be at the listener, in decibels (minus infinity
    /// out of earshot: never chosen).
    pub loudness_db: f32,
}

/// Which candidates are heard: at most `room`, the most important first,
/// never one out of earshot. Gives back their keys, most important first.
pub fn choose(candidates: &[Candidate], room: usize) -> Vec<Key> {
    let mut heard: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.loudness_db.is_finite())
        .collect();
    let rank = |c: &&Candidate| (std::cmp::Reverse(c.priority), c.key);
    if heard.len() > room {
        // Only the best `room` need sorting.
        heard.select_nth_unstable_by(room, |a, b| order(a, b, rank));
        heard.truncate(room);
    }
    heard.sort_unstable_by(|a, b| order(a, b, rank));
    heard.into_iter().map(|c| c.key).collect()
}

/// Priority first, then louder, then the key.
fn order<K: Ord>(
    a: &&Candidate,
    b: &&Candidate,
    rank: impl Fn(&&Candidate) -> (std::cmp::Reverse<Priority>, K),
) -> std::cmp::Ordering {
    let (pa, ka) = rank(a);
    let (pb, kb) = rank(b);
    pa.cmp(&pb)
        .then(b.loudness_db.total_cmp(&a.loudness_db))
        .then(ka.cmp(&kb))
}

/// How loud a sound of `volume_db` at `place` is at `listener`, in
/// decibels, as its event fades between `near_m` and `far_m` (`None`
/// placement: heard the same everywhere). Minus infinity past `far_m`.
/// An estimate for ranking, close to what Kira plays.
pub fn loudness_at(
    volume_db: f32,
    placed: Option<([f32; 3], f32, f32, Fade)>,
    listener: [f32; 3],
) -> f32 {
    let volume = if volume_db.is_finite() {
        volume_db
    } else {
        0.0
    };
    let Some((place, near, far, fade)) = placed else {
        return volume;
    };
    let d: f32 = place
        .iter()
        .zip(listener)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt();
    if !d.is_finite() || d > far {
        return f32::NEG_INFINITY;
    }
    if d <= near || far <= near {
        return volume;
    }
    let share = (d - near) / (far - near);
    let drop = match fade {
        // Even in decibels, down to Kira's silence at far.
        Fade::Realistic => 60.0 * share,
        // Even in loudness itself.
        Fade::Even => -20.0 * (1.0 - share).max(1e-3).log10(),
        Fade::None => 0.0,
    };
    volume - drop
}

/// When one event is asked for `asked` times in one tick: how many copies
/// play, and how much louder each is, in decibels, so a volley of a
/// hundred shots sounds big without a hundred voices.
pub fn copies(asked: usize) -> (usize, f32) {
    if asked <= MAX_COPIES {
        return (asked, 0.0);
    }
    let louder = 10.0 * (asked as f32 / MAX_COPIES as f32).log10();
    (MAX_COPIES, louder.min(MOST_PILE_DB))
}

/// Where in a loop of `length_s` seconds to carry on, `playing_s` seconds
/// after it started (a loop coming back from resting).
pub fn loop_position(playing_s: f64, length_s: f64) -> f64 {
    if !playing_s.is_finite() || !length_s.is_finite() || length_s <= 0.0 || playing_s <= 0.0 {
        return 0.0;
    }
    playing_s % length_s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(key: Key, priority: Priority, loudness_db: f32) -> Candidate {
        Candidate {
            key,
            priority,
            loudness_db,
        }
    }

    #[test]
    fn priority_first_then_loudness_then_key() {
        let all = [
            c(Key::Shot(0), Priority::Low, 0.0),
            c(Key::Shot(1), Priority::Normal, -20.0),
            c(Key::Loop(5), Priority::High, -40.0),
            c(Key::Shot(2), Priority::Normal, -5.0),
            c(Key::Shot(3), Priority::Normal, -5.0),
        ];
        assert_eq!(
            choose(&all, 4),
            [Key::Loop(5), Key::Shot(2), Key::Shot(3), Key::Shot(1)]
        );
        assert_eq!(choose(&all, 0), []);
        assert_eq!(choose(&all, 99).len(), 5);
    }

    #[test]
    fn out_of_earshot_is_never_chosen() {
        let all = [
            c(Key::Loop(1), Priority::High, f32::NEG_INFINITY),
            c(Key::Loop(2), Priority::Low, f32::NAN),
            c(Key::Loop(3), Priority::Low, -50.0),
        ];
        assert_eq!(choose(&all, 64), [Key::Loop(3)]);
    }

    #[test]
    fn the_right_64_of_thousands_are_chosen_the_same_each_time() {
        let all: Vec<Candidate> = (0..3000_u64)
            .map(|i| {
                c(
                    Key::Loop(i),
                    if i % 100 == 0 {
                        Priority::High
                    } else {
                        Priority::Normal
                    },
                    -((i % 997) as f32) / 10.0,
                )
            })
            .collect();
        let chosen = choose(&all, 64);
        assert_eq!(chosen.len(), 64);
        // All 30 high-priority ones, then the loudest normal ones.
        for i in (0..3000).step_by(100) {
            assert!(chosen.contains(&Key::Loop(i)), "{i}");
        }
        let mut sorted = all.clone();
        sorted.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(b.loudness_db.total_cmp(&a.loudness_db))
                .then(a.key.cmp(&b.key))
        });
        let expected: Vec<Key> = sorted.iter().take(64).map(|c| c.key).collect();
        assert_eq!(chosen, expected);
        assert_eq!(choose(&all, 64), chosen, "the same every time");
    }

    #[test]
    fn loudness_fades_with_distance_as_the_event_says() {
        let at = |d: f32, fade| loudness_at(-3.0, Some(([d, 0.0, 0.0], 2.0, 30.0, fade)), [0.0; 3]);
        assert_eq!(at(1.0, Fade::Realistic), -3.0, "full until near");
        assert!((at(16.0, Fade::Realistic) - (-33.0)).abs() < 0.01);
        assert_eq!(
            at(31.0, Fade::Realistic),
            f32::NEG_INFINITY,
            "silent past far"
        );
        assert!(
            (at(16.0, Fade::Even) - (-3.0 - 6.02)).abs() < 0.01,
            "half as loud"
        );
        assert_eq!(at(29.0, Fade::None), -3.0);
        assert_eq!(loudness_at(-3.0, None, [9e9; 3]), -3.0, "flat");
        assert_eq!(
            loudness_at(
                0.0,
                Some(([f32::NAN, 0.0, 0.0], 2.0, 30.0, Fade::Even)),
                [0.0; 3]
            ),
            f32::NEG_INFINITY
        );
    }

    #[test]
    fn a_pile_of_one_event_plays_a_few_louder_copies() {
        assert_eq!(copies(1), (1, 0.0));
        assert_eq!(copies(3), (3, 0.0));
        let (n, louder) = copies(30);
        assert_eq!(n, 3);
        assert!((louder - 6.0).abs() < 1e-4, "{louder}");
        assert_eq!(copies(1000).1, MOST_PILE_DB);
        let (_, six) = copies(6);
        assert!((six - 3.01).abs() < 0.01);
    }

    #[test]
    fn a_loop_carries_on_where_it_would_have_been() {
        assert!((loop_position(12.5, 4.0) - 0.5).abs() < 1e-9);
        assert_eq!(loop_position(3.0, 4.0), 3.0);
        assert_eq!(loop_position(-1.0, 4.0), 0.0);
        assert_eq!(loop_position(5.0, 0.0), 0.0);
        assert_eq!(loop_position(f64::NAN, 4.0), 0.0);
    }
}
