//! The timer store: exact ticks at any game speed, pausing, restarting
//! and stopping, repeating in rhythm, hours passed and jumped, order, and
//! a save's round trip and damaged ones.

use super::*;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn start(timers: &mut Timers, entity: u64, component: &str, name: &str, timing: Timing) {
    let change = TimerChange {
        entity: id(entity),
        component: component.into(),
        name: name.into(),
        plan: Some(timing.plan().unwrap()),
    };
    assert!(timers.apply(&change));
}

fn stop(timers: &mut Timers, entity: u64, component: &str, name: &str) {
    timers.apply(&TimerChange {
        entity: id(entity),
        component: component.into(),
        name: name.into(),
        plan: None,
    });
}

/// The ticks (from 1) on which something fired, over `ticks` ticks at
/// `speed`, the day standing at 8.
fn fired_on(timers: &mut Timers, ticks: u64, speed: f64) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    for tick in 1..=ticks {
        for fired in timers.advance(speed, 8.0) {
            out.push((tick, fired.name.to_string()));
        }
    }
    out
}

#[test]
fn seconds_become_whole_ticks_rounded_to_the_nearest() {
    assert_eq!(Timing::After(2.0).plan(), Ok(Plan::After(120)));
    assert_eq!(Timing::After(0.0).plan(), Ok(Plan::After(0)));
    assert_eq!(Timing::After(0.009).plan(), Ok(Plan::After(1)));
    assert_eq!(Timing::After(0.008).plan(), Ok(Plan::After(0)));
    assert_eq!(Timing::After(1.0 / 3.0).plan(), Ok(Plan::After(20)));
    // A repeating timer waits at least a tick.
    assert_eq!(Timing::Every(0.0).plan(), Ok(Plan::Every(1)));
    // An hour wraps into the day, as setTimeOfDay's does.
    assert_eq!(Timing::AtHour(30.0).plan(), Ok(Plan::AtHour(6.0)));
    assert_eq!(Timing::AtHour(-1.0).plan(), Ok(Plan::AtHour(23.0)));
}

#[test]
fn a_time_that_isn_t_one_is_a_mistake_in_plain_words() {
    for bad in [-1.0, f64::NAN, f64::INFINITY, MAX_SECONDS * 2.0] {
        let why = Timing::After(bad).plan().unwrap_err();
        assert!(why.contains("timer"), "{why}");
    }
    assert!(Timing::AtHour(f64::NAN).plan().is_err());
}

#[test]
fn two_seconds_fire_on_the_exact_tick_at_each_speed() {
    for (speed, tick) in [(1.0, 120), (0.5, 240), (2.0, 60), (0.25, 480)] {
        let mut timers = Timers::default();
        start(&mut timers, 1, "Door", "DoorOpens", Timing::After(2.0));
        let fired = fired_on(&mut timers, 1_000, speed);
        assert_eq!(
            fired,
            vec![(tick, "DoorOpens".to_string())],
            "speed {speed}"
        );
        assert!(timers.is_empty());
    }
}

#[test]
fn no_seconds_fire_on_the_next_tick() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Door", "Now", Timing::After(0.0));
    assert_eq!(fired_on(&mut timers, 3, 1.0), vec![(1, "Now".to_string())]);
}

#[test]
fn nothing_counts_while_the_game_is_paused() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Door", "DoorOpens", Timing::After(1.0));
    // Thirty ticks, then a long pause at speed 0, then thirty more.
    assert!(fired_on(&mut timers, 30, 1.0).is_empty());
    assert!(fired_on(&mut timers, 10_000, 0.0).is_empty());
    let left = timers.left((id(1), "Door", "DoorOpens"), 8.0, 1440.0);
    assert!((left - 0.5).abs() < 1e-9, "{left}");
    assert_eq!(
        fired_on(&mut timers, 30, 1.0),
        vec![(30, "DoorOpens".to_string())]
    );
}

#[test]
fn starting_a_running_timer_starts_it_again_from_the_new_time() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Door", "DoorOpens", Timing::After(1.0));
    assert!(fired_on(&mut timers, 50, 1.0).is_empty());
    start(&mut timers, 1, "Door", "DoorOpens", Timing::After(1.0));
    assert_eq!(timers.len(), 1);
    assert_eq!(
        fired_on(&mut timers, 100, 1.0),
        vec![(60, "DoorOpens".to_string())]
    );
}

#[test]
fn a_stopped_timer_never_fires_and_stopping_another_changes_nothing() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Door", "DoorOpens", Timing::After(1.0));
    stop(&mut timers, 1, "Door", "Respawn");
    assert!(timers.running(id(1), "Door", "DoorOpens"));
    stop(&mut timers, 1, "Door", "DoorOpens");
    assert!(!timers.running(id(1), "Door", "DoorOpens"));
    assert!(fired_on(&mut timers, 100, 1.0).is_empty());
    assert!(timers.is_empty());
    assert_eq!(timers.checksum(), None, "nothing left behind");
}

#[test]
fn time_left_counts_down_and_is_0_when_not_running() {
    let mut timers = Timers::default();
    assert_eq!(timers.left((id(1), "Door", "DoorOpens"), 8.0, 1440.0), 0.0);
    start(&mut timers, 1, "Door", "DoorOpens", Timing::After(2.0));
    let left = |t: &Timers| t.left((id(1), "Door", "DoorOpens"), 8.0, 1440.0);
    assert!((left(&timers) - 2.0).abs() < 1e-9);
    fired_on(&mut timers, 60, 1.0);
    assert!((left(&timers) - 1.0).abs() < 1e-9);
    // At half speed a tick is half a tick of game time.
    fired_on(&mut timers, 60, 0.5);
    assert!((left(&timers) - 0.5).abs() < 1e-9);
    // Another component's timer of the same name is its own.
    assert!(!timers.running(id(1), "Gate", "DoorOpens"));
    assert!(!timers.running(id(2), "Door", "DoorOpens"));
}

#[test]
fn a_repeating_timer_keeps_its_rhythm() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Spawner", "Wave", Timing::Every(0.5));
    let ticks: Vec<u64> = fired_on(&mut timers, 200, 1.0)
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    assert_eq!(ticks, vec![30, 60, 90, 120, 150, 180]);
    assert!(timers.running(id(1), "Spawner", "Wave"));
    // From its last turn, not from now: at speeds that don't divide its
    // interval in whole ticks it still keeps time on average.
    let mut timers = Timers::default();
    start(&mut timers, 1, "Spawner", "Wave", Timing::Every(0.5));
    let ticks: Vec<u64> = fired_on(&mut timers, 400, 0.3)
        .into_iter()
        .map(|(t, _)| t)
        .collect();
    assert_eq!(ticks, vec![100, 200, 300, 400]);
}

#[test]
fn a_repeating_timer_behind_fires_once_and_keeps_time() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Spawner", "Tick", Timing::Every(1.0 / 60.0));
    // At speed 3 three turns pass each tick: one fires.
    let fired = fired_on(&mut timers, 4, 3.0);
    assert_eq!(fired.len(), 4);
    let left = timers.left((id(1), "Spawner", "Tick"), 8.0, 1440.0);
    assert!((left - 1.0 / 60.0).abs() < 1e-9, "{left}");
}

/// The hours on which a timer at `at` fires as the day goes through
/// `hours`, one a tick.
fn fired_at_hours(at: f64, hours: &[f64]) -> Vec<f64> {
    let mut timers = Timers::default();
    timers.advance(1.0, hours[0]);
    start(&mut timers, 1, "Rooster", "Dawn", Timing::AtHour(at));
    let mut out = Vec::new();
    for &hour in &hours[1..] {
        if !timers.advance(1.0, hour).is_empty() {
            out.push(hour);
        }
    }
    out
}

#[test]
fn a_timer_at_an_hour_fires_as_the_clock_passes_it_each_day() {
    assert_eq!(
        fired_at_hours(6.0, &[5.0, 5.5, 5.99, 6.0, 6.5, 7.0]),
        vec![6.0]
    );
    // Two days: it fires each day, and keeps running.
    let day: Vec<f64> = (0..=96).map(|q| f64::from(q % 96) / 4.0 + 0.1).collect();
    let two_days: Vec<f64> = day.iter().chain(&day[1..]).copied().collect();
    assert_eq!(fired_at_hours(6.0, &two_days).len(), 2);
}

#[test]
fn jumping_the_clock_past_the_hour_fires_once() {
    // Sleeping from 22:00 to the next morning at 7.
    assert_eq!(fired_at_hours(6.0, &[22.0, 7.0, 7.1]), vec![7.0]);
    // Setting the clock back from 7 to 5 is the day going on to 5, which
    // comes before the next six: that fires as the clock passes it.
    assert!(fired_at_hours(6.0, &[7.0, 5.0]).is_empty());
    assert_eq!(fired_at_hours(6.0, &[7.0, 5.0, 6.5]), vec![6.5]);
    // A jump that lands on the hour fires; one short of it doesn't.
    assert_eq!(fired_at_hours(6.0, &[22.0, 6.0]), vec![6.0]);
    assert!(fired_at_hours(6.0, &[22.0, 5.5]).is_empty());
}

#[test]
fn passing_midnight_works_the_same() {
    assert_eq!(fired_at_hours(0.0, &[23.5, 23.9, 0.1, 0.5]), vec![0.1]);
    assert_eq!(fired_at_hours(23.75, &[23.5, 0.25]), vec![0.25]);
    assert_eq!(fired_at_hours(0.5, &[23.5, 1.0]), vec![1.0]);
}

#[test]
fn time_left_at_an_hour_is_the_game_time_until_it() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Rooster", "Dawn", Timing::AtHour(6.0));
    // A day of 24 minutes: an hour is a minute.
    let left = timers.left((id(1), "Rooster", "Dawn"), 4.0, 24.0 * 60.0);
    assert!((left - 120.0).abs() < 1e-9, "{left}");
    let left = timers.left((id(1), "Rooster", "Dawn"), 6.0, 24.0 * 60.0);
    assert!(
        (left - 24.0 * 60.0).abs() < 1e-9,
        "a whole day at the hour: {left}"
    );
}

#[test]
fn what_fires_in_a_tick_comes_in_due_entity_component_and_start_order() {
    let mut timers = Timers::default();
    start(&mut timers, 2, "Door", "B", Timing::After(0.5));
    start(&mut timers, 1, "Zeta", "C", Timing::After(0.5));
    start(&mut timers, 1, "Alpha", "D", Timing::After(0.5));
    start(&mut timers, 1, "Alpha", "A", Timing::After(0.5));
    start(&mut timers, 9, "Door", "Early", Timing::After(0.25));
    let mut order = Vec::new();
    for _ in 0..30 {
        order.extend(timers.advance(1.0, 8.0));
    }
    let names: Vec<(u64, &str, &str)> = order
        .iter()
        .map(|f| (f.entity.0.raw(), &*f.component, &*f.name))
        .collect();
    assert_eq!(
        names,
        vec![
            (9, "Door", "Early"),
            (1, "Alpha", "D"),
            (1, "Alpha", "A"),
            (1, "Zeta", "C"),
            (2, "Door", "B"),
        ]
    );
}

#[test]
fn ending_by_owner_takes_only_their_timers() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Door", "A", Timing::After(1.0));
    start(&mut timers, 1, "Gate", "B", Timing::Every(1.0));
    start(&mut timers, 2, "Door", "C", Timing::AtHour(6.0));
    assert_eq!(timers.retain(|entity, _| entity != id(2)), 1);
    assert_eq!(timers.retain(|_, component| component != "Gate"), 1);
    assert_eq!(timers.len(), 1);
    assert!(timers.running(id(1), "Door", "A"));
}

#[test]
fn with_nothing_waiting_the_clock_starts_again() {
    // So a game, and its save loaded while nothing waited, count alike.
    let mut timers = Timers::default();
    start(&mut timers, 1, "Door", "A", Timing::After(0.1));
    fired_on(&mut timers, 100, 1.0);
    let mut fresh = Timers::default();
    fresh.advance(1.0, 8.0);
    assert_eq!(timers, fresh);
    assert_eq!(timers.checksum(), None);
}

#[test]
fn a_save_round_trips_and_carries_on_alike() {
    let mut timers = Timers::default();
    timers.advance(1.0, 5.5);
    start(&mut timers, 1, "Door", "DoorOpens", Timing::After(2.0));
    start(&mut timers, 3, "Spawner", "Wave", Timing::Every(0.75));
    start(&mut timers, 2, "Rooster", "Dawn", Timing::AtHour(6.0));
    fired_on(&mut timers, 50, 1.0);
    let text = timers.write();
    let mut read = Timers::read(&text).unwrap();
    assert_eq!(read, timers);
    assert_eq!(read.checksum(), timers.checksum());
    assert_eq!(read.write(), text);
    for hour in [5.9, 6.1, 7.0] {
        assert_eq!(read.advance(1.0, hour), timers.advance(1.0, hour));
    }
    assert_eq!(Timers::read(""), Ok(Timers::default()));
    assert_eq!(Timers::default().write(), "");
}

#[test]
fn damaged_saves_are_refused_whole() {
    let mut timers = Timers::default();
    start(&mut timers, 1, "Door", "DoorOpens", Timing::Every(1.0));
    let good = timers.write();
    let bad = [
        "at\n",
        "at 0 - 0\n",
        "at 0 - 1\nonce 0000000000001 Door DoorOpens 1 60\n",
        "at 0 - 1\nonce 0000000000001 Door doorOpens 0 60\n",
        "at 0 - 1\nonce 0000000000001 Do-or DoorOpens 0 60\n",
        "at 0 - 1\nevery 0000000000001 Door DoorOpens 0 60 0\n",
        "at 0 - 1\nevery 0000000000001 Door DoorOpens 0 60 7\n",
        "at 0 - 1\nhour 0000000000001 Door Dawn 0 7ff8000000000000\n",
        "at 0 - 1\nhour 0000000000001 Door Dawn 0 4038000000000000\n",
        "at 0 7ff0000000000000 1\nonce 0000000000001 Door A 0 1\n",
        "at 0 - 2\nonce 0000000000001 Door A 0 1\nonce 0000000000001 Door A 1 1\n",
        "at 0 - 1\nsoon 0000000000001 Door A 0 1\n",
        "at 0 - 1\nonce 0000000000001 Door A 0 1 more\n",
        "at 0 - 1 2\n",
    ];
    for text in bad {
        assert!(Timers::read(text).is_err(), "{text:?}");
    }
    assert!(Timers::read(&good).is_ok());
}

#[test]
fn fuzzed_saves_never_crash_the_reader() {
    let mut timers = Timers::default();
    timers.advance(1.0, 3.0);
    start(&mut timers, 1, "Door", "DoorOpens", Timing::After(2.0));
    start(&mut timers, 7, "Spawner", "Wave", Timing::Every(0.5));
    start(&mut timers, 2, "Rooster", "Dawn", Timing::AtHour(6.25));
    let good = timers.write().into_bytes();
    let mut seed: u64 = 0x5eed;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..5_000 {
        let mut bytes = good.clone();
        for _ in 0..(next() % 6 + 1) {
            let at = (next() as usize) % bytes.len();
            match next() % 3 {
                0 => bytes[at] = next() as u8,
                1 => {
                    bytes.remove(at);
                }
                _ => bytes.insert(at, b"0 -\n9a"[(next() % 6) as usize]),
            }
            if bytes.is_empty() {
                break;
            }
        }
        let text = String::from_utf8_lossy(&bytes);
        if let Ok(read) = Timers::read(&text) {
            // Whatever reads, writes and reads back as itself.
            assert_eq!(Timers::read(&read.write()), Ok(read));
        }
    }
}

#[test]
fn names_have_the_shape_of_a_state_s_option() {
    for good in ["DoorOpens", "A", "Wave_2", "X9"] {
        assert!(valid_name(good), "{good}");
    }
    let long = "A".repeat(MAX_NAME + 1);
    for bad in [
        "",
        "doorOpens",
        "Door Opens",
        "Door.Opens",
        "Ü",
        long.as_str(),
    ] {
        assert!(!valid_name(bad), "{bad}");
    }
}

#[test]
fn restarting_every_tick_and_ending_many_keeps_the_store_small_and_right() {
    // A component that starts its timer every tick (a common slip) never
    // sees it finish, and the store doesn't grow with each start.
    let mut timers = Timers::default();
    for _ in 0..20_000 {
        start(&mut timers, 1, "Door", "Late", Timing::After(10.0));
        assert!(timers.advance(1.0, 8.0).is_empty());
    }
    assert_eq!(timers.len(), 1);
    // The last start came a tick before.
    assert_eq!(
        fired_on(&mut timers, 600, 1.0),
        vec![(599, "Late".to_string())]
    );
    // Many started and most ended: the rest still finish, in order.
    for n in 1..=10_000 {
        start(&mut timers, n, "Door", "A", Timing::After(1.0));
    }
    assert_eq!(
        timers.retain(|entity, _| entity.0.raw() % 1_000 == 0),
        9_990
    );
    let fired = fired_on(&mut timers, 60, 1.0);
    assert_eq!(fired.len(), 10);
    assert!(fired.iter().all(|(tick, _)| *tick == 60));
    // Started again after: each from its own new time.
    for n in [3, 7] {
        start(&mut timers, n, "Door", "A", Timing::After(0.5));
    }
    assert_eq!(fired_on(&mut timers, 30, 1.0).len(), 2);
    assert!(timers.is_empty());
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_100000_timers_with_10000_finishing_a_tick() {
    // Stage 1's scale case for timers: 100,000 waiting, 10,000 of them
    // finishing every tick (each fired and started again, as a handler
    // restarting it would), the rest far off. The store's own share of a
    // 60th of a second, apart from the handlers' code and from making the
    // events (said apart: the engine's events are made the same way).
    const TARGET_MS: f64 = 2.0;
    let mut timers = Timers::default();
    let components: [Arc<str>; 4] = [
        "Door".into(),
        "Gate".into(),
        "Hive".into(),
        "Spawner".into(),
    ];
    let names: [Arc<str>; 2] = ["Soon".into(), "Later".into()];
    let change = |n: u64, soon: bool, plan: Plan| TimerChange {
        entity: id(n / 4 + 1),
        component: Arc::clone(&components[(n % 4) as usize]),
        name: Arc::clone(&names[usize::from(!soon)]),
        plan: Some(plan),
    };
    for n in 0..100_000 {
        let soon = n % 10 == 0;
        let plan = if soon {
            Plan::After(1)
        } else {
            Plan::After(600_000)
        };
        assert!(timers.apply(&change(n, soon, plan)));
    }
    assert_eq!(timers.len(), 100_000);
    let ticks = 60;
    let (mut store, mut making) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    let mut finished = 0usize;
    // A few ticks first, so the buckets have their room, as in a game
    // that has run a while.
    for tick in 0..ticks + 5 {
        let began = std::time::Instant::now();
        let fired = timers.advance(1.0, 8.0);
        let advanced = began.elapsed();
        let events: Vec<crate::Event> = fired
            .into_iter()
            .map(|f| crate::Event {
                to: f.entity,
                name: "TimerFinished".to_string(),
                values: vec![reflect::Value::Text(f.name.to_string())],
                only: Some(f.component),
            })
            .collect();
        let made = began.elapsed() - advanced;
        // Each started again for the next tick, landing in order.
        let landing = std::time::Instant::now();
        for event in &events {
            let restart = TimerChange {
                entity: event.to,
                component: Arc::clone(event.only.as_ref().unwrap()),
                name: Arc::clone(&names[0]),
                plan: Some(Plan::After(1)),
            };
            timers.apply(&restart);
        }
        if tick >= 5 {
            store += advanced + landing.elapsed();
            making += made;
            finished += events.len();
        }
    }
    assert_eq!(finished, 10_000 * ticks);
    assert_eq!(timers.len(), 100_000);
    let per_tick = |d: std::time::Duration| d.as_secs_f64() * 1000.0 / f64::from(ticks as u32);
    let (ms, events_ms) = (per_tick(store), per_tick(making));
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: 100,000 timers waiting, 10,000 finishing a tick and started again: the \
         store's own share {ms:.3} ms a tick (target {TARGET_MS} ms) {verdict}; making their \
         TimerFinished events {events_ms:.3} ms more"
    );
    assert!(ms <= TARGET_MS, "the timers take {ms:.3} ms a tick");
}
