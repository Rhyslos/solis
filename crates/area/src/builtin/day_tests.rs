//! Day and night's colours (decision 0092).

use super::*;

#[test]
fn colours_read_back_as_written() {
    let d = DayColours::default();
    assert_eq!(
        d.text(),
        "4:30 #2a2f5a, 6:00 #e99a6a, 9:00 #fff6e0, 18:00 #f08a50, 21:00 #2a2f5a"
    );
    assert_eq!(DayColours::parse(&d.text()), d);
    let odd = DayColours::parse("21:30 #000000, 7 #ffffff, nonsense, 25:00 #ff0000, 3:00 #12");
    assert_eq!(
        odd.text(),
        "1:00 #ff0000, 7:00 #ffffff, 21:30 #000000",
        "sorted, 25 is 1, bad parts left out"
    );
    assert_eq!(
        DayColours::parse(""),
        DayColours::default(),
        "nothing: the default"
    );
    let many: Vec<String> = (0..20).map(|h| format!("{h}:00 #ffffff")).collect();
    assert_eq!(DayColours::parse(&many.join(", ")).stops().len(), MAX_STOPS);
}

#[test]
fn the_tint_runs_between_stops_and_round_midnight() {
    let d = DayColours::from_stops(&[(6.0, [0.0; 3]), (18.0, [1.0; 3])]);
    assert_eq!(d.at(6.0), [0.0; 3]);
    assert_eq!(d.at(18.0), [1.0; 3]);
    assert_eq!(d.at(12.0), [0.5; 3], "half way");
    assert_eq!(d.at(0.0), [0.5; 3], "round midnight, half way back");
    assert_eq!(d.at(30.0), d.at(6.0), "hours wrap");
    assert_eq!(d.at(f32::NAN), d.at(0.0));
    let one = DayColours::from_stops(&[(3.0, [0.2, 0.3, 0.4])]);
    assert_eq!(one.at(17.0), [0.2, 0.3, 0.4]);
}

#[test]
fn night_is_darker_than_half_the_brightest() {
    let d = DayColours::default();
    assert!(!d.is_night(12.0));
    assert!(d.is_night(23.0));
    assert!(d.is_night(3.0));
    assert!(!d.is_night(10.0));
    // Dusk: on somewhere between 18:00 and 21:00.
    assert!(!d.is_night(17.0) && d.is_night(21.0));
}

#[test]
fn a_day_all_of_black_is_all_night() {
    let d = DayColours::from_stops(&[(6.0, [0.0; 3]), (18.0, [0.0; 3])]);
    assert!(d.is_night(12.0) && d.is_night(0.0));
}

#[test]
fn the_environment_s_day_reads_back_and_older_ones_change_nothing() {
    use crate::builtin::Environment;
    use reflect::{Component, ComponentData, Value};
    let e = Environment {
        day_night: true,
        day_minutes: 10.0,
        day_start: 6.0,
        light_sprites: true,
        light_steps: 4,
        ..Default::default()
    };
    assert_eq!(Environment::from_data(&e.to_data()), e);
    // Ten-minute days from 06:00: five minutes in, it's 18:00; a shift
    // of 13 hours wraps round to 07:00.
    assert_eq!(e.hour_at(300.0, 0.0), 18.0);
    assert_eq!(e.hour_at(300.0, 13.0), 7.0);
    // One step would be all or nothing: it reads as two.
    let mut data = e.to_data();
    data.set("light_steps", Value::Int(1));
    assert_eq!(Environment::from_data(&data).light_steps, 2);
    data.set("light_steps", Value::Int(99));
    assert_eq!(Environment::from_data(&data).light_steps, 8);
    // An Environment from before day and night: off, and sprites unlit.
    let old = Environment::from_data(&ComponentData::default());
    assert!(!old.day_night && !old.light_sprites);
    assert_eq!(old.tint_at(23.0), [1.0; 3], "white with the day off");
}
