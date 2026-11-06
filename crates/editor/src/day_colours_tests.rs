//! The day's colours in Properties (decision 0092).

use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

#[test]
fn hours_read_as_a_clock() {
    assert_eq!(clock(19.5), "19:30");
    assert_eq!(clock(0.0), "00:00");
    assert_eq!(clock(24.0), "00:00");
    assert_eq!(clock(-1.0), "23:00");
    assert_eq!(clock(23.999), "00:00");
}

fn harness() -> Harness<'static, (Value, Vec<Value>)> {
    Harness::builder()
        .with_size(egui::vec2(360.0, 400.0))
        .build_ui_state(
            |ui, (value, changes): &mut (Value, Vec<Value>)| {
                if let Some(v) = day_colours(ui, value, false) {
                    *value = v.clone();
                    changes.push(v);
                }
            },
            (Value::Text(DayColours::default().text()), Vec::new()),
        )
}

#[test]
fn a_colour_is_added_and_taken_away() {
    let mut h = harness();
    h.run();
    h.get_by_label("+ Add a colour").click();
    h.run();
    let Value::Text(t) = h.state().0.clone() else {
        panic!()
    };
    assert_eq!(DayColours::parse(&t).stops().len(), 6, "{t}");
    assert!(t.contains("13:30"), "the middle of 09:00 to 18:00: {t}");
    h.get_all_by_label("Remove").next().unwrap().click();
    h.run();
    let Value::Text(t) = h.state().0.clone() else {
        panic!()
    };
    assert_eq!(DayColours::parse(&t).stops().len(), 5, "{t}");
    assert!(!t.starts_with("4:30"), "the first taken away: {t}");
}

#[test]
fn the_preview_time_is_kept_for_the_viewport_and_not_saved() {
    let mut h = harness();
    h.run();
    h.get_by_label("Preview time").click();
    h.run();
    let kept: Option<Option<f64>> = h.ctx.data(|d| d.get_temp(preview_id()));
    assert_eq!(kept, Some(Some(12.0)));
    assert!(h.state().1.is_empty(), "nothing saved");
    h.get_by_label("Preview time").click();
    h.run();
    let kept: Option<Option<f64>> = h.ctx.data(|d| d.get_temp(preview_id()));
    assert_eq!(kept, Some(None));
}

/// A workbench whose area has day and night on (a day of 24 minutes,
/// starting at 8), a lamp that shines only at night, and a Sleeper that
/// sets the time to `hour` once, and says the time.
fn with_a_day(hour: f64) -> crate::workbench::Workbench {
    use crate::test_support::info;
    use area::EntityData;
    use area::builtin::{Environment, Light, Transform};
    let main = format!(
        "fetch \"solis/world\":World;\ncomponent Sleeper {{\n    has {{ bool slept = false; }}\n    method update(float dt) {{\n        is not slept? then {{\n            World:setTimeOfDay({hour:.1});\n            slept = true;\n        }} or {{\n            World:say(\"hour \" + World:timeOfDay().text());\n        }}\n    }}\n}}\npass Sleeper;\nend\n"
    );
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard(),
    );
    files.push(("main.lst".into(), main));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(c)) => std::sync::Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut w = crate::workbench::Workbench::new(info(), None);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }));
    w.set_program(Some(program));
    let mut sleeper = EntityData::new("Sleeper");
    sleeper
        .components
        .insert("Sleeper".into(), reflect::ComponentData::default());
    let entities = vec![
        EntityData::new("Environment").with(&Environment {
            day_night: true,
            ..Environment::default()
        }),
        EntityData::new("Street lamp")
            .with(&Transform::default())
            .with(&Light {
                only_at_night: true,
                ..Light::point()
            }),
        sleeper,
    ];
    let mut area = area::Area::new();
    for e in entities {
        area.spawn(e);
    }
    w.open_area(area, std::path::PathBuf::from("day.solarea"));
    w
}

#[test]
fn game_code_sets_the_hour_and_night_lamps_follow_it() {
    let mut w = with_a_day(22.0);
    // Editing: the area at its start, 08:00, the lamp off.
    w.viewport.area =
        crate::area_view::AreaView::of(w.document.area(), &std::collections::BTreeMap::new());
    let view = w.viewport.view([64, 64], 1.0);
    assert_eq!(view.hour, 8.0);
    assert_eq!(view.lamps.len(), 1);
    assert_eq!(view.lamps_shining().count(), 0, "off by day");
    // The preview time shows another hour.
    w.viewport.preview_hour = Some(23.0);
    let view = w.viewport.view([64, 64], 1.0);
    assert_eq!(view.lamps_shining().count(), 1, "on at night");
    w.viewport.preview_hour = None;
    // Playing: game code sends it to 22:00.
    w.play();
    for _ in 0..3 {
        w.run_game(1.0 / 60.0);
    }
    let hour = w.world().unwrap().time_of_day();
    assert!((22.0..22.1).contains(&hour), "{hour}");
    let said: Vec<String> = w
        .log_lines()
        .iter()
        .map(|l| l.text.clone())
        .filter(|t| t.starts_with("hour "))
        .collect();
    assert!(said.iter().any(|t| t.starts_with("hour 22")), "{said:?}");
    // A save keeps it.
    let text = w.save_of_game().unwrap().write();
    assert!(text.contains("\nday 16\n"), "{text}");
    w.stop();
}

#[test]
fn the_preview_time_is_left_out_while_playing_and_forgotten_with_its_project() {
    let mut w = with_a_day(22.0);
    let ctx = egui::Context::default();
    // Opening the project forgot any earlier preview.
    assert_eq!(w.preview_hour(&ctx), None);
    ctx.data_mut(|d| d.insert_temp(preview_id(), Some(23.0f64)));
    assert_eq!(w.preview_hour(&ctx), Some(23.0));
    w.play();
    assert_eq!(w.preview_hour(&ctx), None, "the game's hour while playing");
    w.stop();
    assert_eq!(w.preview_hour(&ctx), Some(23.0), "back after Stop");
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Another".into(),
        root: "/projects/another".into(),
        assets: "/projects/another/assets".into(),
    }));
    assert_eq!(w.preview_hour(&ctx), None, "forgotten with its project");
    let kept: Option<Option<f64>> = ctx.data(|d| d.get_temp(preview_id()));
    assert_eq!(kept, None);
}
