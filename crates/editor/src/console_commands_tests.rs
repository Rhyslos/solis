//! The console's commands: reading command lines, running each command,
//! Tab, the history, and the `'` key in the whole window.

use super::*;
use crate::console::LogLevel;
use crate::test_support::*;
use area::builtin::{Light, Transform};
use egui_kittest::kittest::Queryable;
use glam::Vec3;

fn run(w: &mut Workbench, line: &str) -> Vec<WorkbenchAction> {
    let mut actions = Vec::new();
    w.run_command(line, &egui::Context::default(), &mut actions);
    actions
}

/// The newest console message.
fn said(w: &Workbench) -> (LogLevel, String) {
    let line = w.log_lines().last().expect("a message");
    (line.level, line.text.clone())
}

/// Everything the last command answered, one line each.
fn answer(w: &Workbench) -> String {
    let lines = w.log_lines();
    let echo = lines
        .iter()
        .rposition(|l| l.text.starts_with("› "))
        .expect("a command");
    lines[echo + 1..]
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn position(w: &Workbench, id: EntityId) -> Vec3 {
    w.document
        .area()
        .get(id)
        .unwrap()
        .get::<Transform>()
        .unwrap()
        .position
}

#[test]
fn words_are_split_on_spaces_and_quotes_keep_them_together() {
    assert_eq!(words("select  Crate   Lamp"), ["select", "Crate", "Lamp"]);
    assert_eq!(
        words(r#"select "Point light 1" Crate"#),
        ["select", "Point light 1", "Crate"]
    );
    assert_eq!(words(r#"new """#), ["new", ""], "empty quotes are a word");
    assert_eq!(words(r#"select "never closed"#), ["select", "never closed"]);
    assert!(words("   ").is_empty());
    assert_eq!(words(&"a ".repeat(500)).len(), MAX_WORDS);
}

#[test]
fn numbers_colours_choices_and_switches_are_read_for_their_field() {
    let registry = area::builtin::registry().unwrap();
    let field = |c: &str, f: &str| {
        registry
            .get(c)
            .unwrap()
            .fields
            .iter()
            .find(|i| i.name == f)
            .unwrap()
            .clone()
    };
    let w = |s: &str| words(s);
    let position = field("Transform", "position");
    assert_eq!(
        parse_value(&position, &w("1 2.5 -3")).unwrap(),
        Value::floats(&[1.0, 2.5, -3.0])
    );
    assert_eq!(
        parse_value(&position, &w("1,2,3m")).unwrap(),
        Value::floats(&[1.0, 2.0, 3.0])
    );
    assert!(parse_value(&position, &w("1 2")).is_err());
    assert!(parse_value(&position, &w("1 two 3")).is_err());
    assert!(parse_value(&position, &w("1 NaN 3")).is_err(), "no NaN");
    assert!(parse_value(&position, &w("1 inf 3")).is_err());

    let rotation = field("Transform", "rotation");
    let turned = parse_value(&rotation, &w("0 0 90")).unwrap();
    let degrees = degrees_of(&turned);
    assert!((degrees[2] - 90.0).abs() < 1e-3, "{degrees:?}");

    let color = field("Light", "color");
    let orange = parse_value(&color, &w("#ff8000")).unwrap();
    let [r, g, b] = orange.as_floats::<3>().unwrap();
    assert!((r - 1.0).abs() < 1e-6 && (g - 128.0 / 255.0).abs() < 1e-6 && b == 0.0);
    assert!(parse_value(&color, &w("1 0.5 0")).is_ok());
    assert!(parse_value(&color, &w("2 0 0")).is_err(), "out of range");
    assert!(parse_value(&color, &w("#ff80")).is_err());

    let kind = field("Light", "kind");
    assert_eq!(
        parse_value(&kind, &w("spot")).unwrap(),
        Value::Text("Spot".into()),
        "any capitals"
    );
    assert!(
        parse_value(&kind, &w("laser"))
            .unwrap_err()
            .contains("Sun, Point, Spot")
    );

    let shadows = field("Light", "shadows");
    assert_eq!(parse_value(&shadows, &w("on")).unwrap(), Value::Bool(true));
    assert_eq!(parse_value(&shadows, &w("no")).unwrap(), Value::Bool(false));
    assert!(parse_value(&shadows, &w("maybe")).is_err());

    let strength = field("Light", "strength");
    assert_eq!(
        parse_value(&strength, &w("250")).unwrap(),
        Value::Float(250.0)
    );
    assert!(parse_value(&strength, &w("lots")).is_err());

    let model = field("Model", "asset");
    assert!(
        parse_value(&model, &w("abc"))
            .unwrap_err()
            .contains("Properties")
    );
}

#[test]
fn select_finds_entities_by_name_or_id_and_says_when_nothing_matches() {
    let (mut w, crate_id) = with_a_crate();
    let lamp = w.add_point_light();
    w.document.selection.clear();
    run(&mut w, "select crate");
    assert_eq!(
        w.document.selection.primary(),
        Some(crate_id),
        "any capitals"
    );
    run(&mut w, r#"select "point light 1" Crate"#);
    assert_eq!(w.document.selection.iter().count(), 2);
    assert_eq!(w.document.selection.primary(), Some(lamp));
    run(&mut w, &format!("select {crate_id}"));
    assert_eq!(w.document.selection.iter().collect::<Vec<_>>(), [crate_id]);
    run(&mut w, "select none");
    assert_eq!(w.document.selection.primary(), None);
    run(&mut w, "select all");
    assert_eq!(w.document.selection.iter().count(), w.document.area().len());
    run(&mut w, "select Nothing");
    assert_eq!(said(&w).0, LogLevel::Error);
    assert!(said(&w).1.contains("nothing is called"));
    run(&mut w, "select");
    assert_eq!(said(&w).0, LogLevel::Error);
}

#[test]
fn set_changes_every_selected_entity_in_one_undo_step() {
    let (mut w, crate_id) = with_a_crate();
    let lamp = w.add_point_light();
    w.document.selection.select_all([crate_id, lamp], crate_id);
    run(&mut w, "set transform.position 1 2 3");
    assert_eq!(position(&w, crate_id), Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(position(&w, lamp), Vec3::new(1.0, 2.0, 3.0));
    w.undo();
    assert_eq!(position(&w, crate_id), Vec3::ZERO, "one step undoes both");
    assert_ne!(position(&w, lamp), Vec3::new(1.0, 2.0, 3.0));
    w.redo();

    // Only the lamp has a Light; the crate is left alone.
    run(&mut w, "set Light.strength 250");
    let light = w.document.area().get(lamp).unwrap().get::<Light>().unwrap();
    assert_eq!(light.strength, 250.0);

    // Two typed changes are two undo steps, never merged.
    run(&mut w, "set Light.strength 10");
    w.undo();
    let light = w.document.area().get(lamp).unwrap().get::<Light>().unwrap();
    assert_eq!(light.strength, 250.0);
}

#[test]
fn set_and_get_say_what_went_wrong() {
    let (mut w, crate_id) = with_a_crate();
    for (line, wanted) in [
        ("set Transform.position 1 2 3", "nothing is selected"),
        ("set", "set what"),
        ("set Wobble.x 1", "no component called \"Wobble\""),
        ("set Transform.size 1", "it has position, rotation, scale"),
        ("set Transform 1 2 3", "which setting"),
    ] {
        run(&mut w, line);
        assert_eq!(said(&w).0, LogLevel::Error, "{line}");
        assert!(said(&w).1.contains(wanted), "{line}: {}", said(&w).1);
    }
    w.document.selection.select_only(crate_id);
    run(&mut w, "set Transform.position");
    assert!(said(&w).1.contains("to what"), "{}", said(&w).1);
    run(&mut w, "set Light.strength 5");
    assert!(said(&w).1.contains("nothing selected has a Light"));
    run(&mut w, "get Light");
    assert!(said(&w).1.contains("has no Light"));
    let before = w.document.history().undo_len();
    run(&mut w, "set Transform.position 1 two 3");
    assert_eq!(w.document.history().undo_len(), before, "nothing changed");
}

#[test]
fn get_shows_a_component_or_one_setting() {
    let (mut w, crate_id) = with_a_crate();
    w.document.selection.select_only(crate_id);
    run(&mut w, "get Transform.position");
    assert_eq!(
        said(&w),
        (
            LogLevel::Info,
            "Transform.position = 0.000 0.000 0.000 m".into()
        )
    );
    run(&mut w, "get transform");
    let text = answer(&w);
    assert!(
        text.contains("Transform.rotation = 0.000 0.000 0.000°"),
        "{text}"
    );
    assert!(
        text.contains("Transform.scale = 1.000 1.000 1.000"),
        "{text}"
    );
    let lamp = w.add_point_light();
    w.document.selection.select_only(lamp);
    run(&mut w, "get Light.color");
    assert_eq!(said(&w).1, "Light.color = #ffffff");
    run(&mut w, "get Light.kind");
    assert_eq!(said(&w).1, "Light.kind = Point");
}

#[test]
fn the_other_commands_do_what_their_menus_do() {
    let (mut w, crate_id) = with_a_crate();
    let entities = w.document.area().len();
    run(&mut w, "new Harbour wall");
    assert_eq!(w.document.area().len(), entities + 1);
    let new = w.document.selection.primary().unwrap();
    assert_eq!(w.document.area().get(new).unwrap().name, "Harbour wall");
    run(&mut w, "duplicate");
    assert_eq!(w.document.area().len(), entities + 2);
    run(&mut w, "delete");
    assert_eq!(w.document.area().len(), entities + 1);
    run(&mut w, "undo");
    assert_eq!(w.document.area().len(), entities + 2);
    run(&mut w, "redo");
    assert_eq!(w.document.area().len(), entities + 1);
    w.document.selection.clear();
    run(&mut w, "delete");
    assert_eq!(said(&w), (LogLevel::Error, "nothing is selected".into()));

    run(&mut w, "find har");
    assert!(said(&w).1.starts_with("Harbour wall ("), "{}", said(&w).1);
    for i in 0..25 {
        run(&mut w, &format!("new Post {i}"));
    }
    run(&mut w, "find post");
    assert_eq!(answer(&w).lines().count(), MAX_FOUND + 1);
    assert!(answer(&w).ends_with("…and 5 more"));
    run(&mut w, "find zebra");
    assert!(said(&w).1.contains("Nothing's name"));

    assert_eq!(run(&mut w, "save"), [WorkbenchAction::SaveArea]);
    run(&mut w, "tool rotate");
    assert_eq!(w.tool(), Tool::Rotate);
    run(&mut w, "look unlit");
    assert_eq!(w.look().kind, LookKind::Unlit);
    run(&mut w, "look shiny");
    assert_eq!(said(&w).0, LogLevel::Error);

    w.document.selection.select_only(crate_id);
    run(&mut w, "frame");
    run(&mut w, "stats");
    assert!(
        answer(&w).contains("1 with a model, 1 light)"),
        "{}",
        answer(&w)
    );

    run(&mut w, "HELP");
    assert!(
        answer(&w).contains("\n  select <name or ID>"),
        "{}",
        answer(&w)
    );
    assert_eq!(answer(&w).lines().count(), COMMANDS.len() + 2);
    run(&mut w, "help set");
    assert!(said(&w).1.starts_with("set <Component.field> <value>"));
    run(&mut w, "help wobble");
    assert_eq!(said(&w).0, LogLevel::Error);
    run(&mut w, "wobble");
    assert!(said(&w).1.contains("no command \"wobble\""));

    run(&mut w, "clear");
    assert!(w.log_lines().is_empty());
    run(&mut w, "   ");
    assert!(w.log_lines().is_empty(), "an empty line does nothing");
}

#[test]
fn a_command_is_echoed_before_its_answer() {
    let (mut w, _) = with_a_crate();
    run(&mut w, "find crate");
    let lines = w.log_lines();
    assert_eq!(lines[lines.len() - 2].text, "› find crate");
}

#[test]
fn tab_completes_commands_names_and_settings() {
    let (mut w, _) = with_a_crate();
    w.add_point_light();
    let done = w.complete("sel");
    assert_eq!(done.line, "select ");
    assert!(done.choices.is_empty());
    let done = w.complete("s");
    assert_eq!(
        done.line, "s",
        "select, set, save, step, stop, stats share only s"
    );
    assert_eq!(
        done.choices,
        ["select", "set", "save", "step", "stop", "stats"]
    );
    assert_eq!(w.complete("st").line, "st");
    assert_eq!(w.complete("sta").line, "stats ");
    assert_eq!(w.complete("select po").line, r#"select "Point light 1" "#);
    assert_eq!(w.complete("select cr").line, "select Crate ");
    let done = w.complete("set Light.s");
    assert_eq!(done.line, "set Light.s");
    assert_eq!(
        done.choices,
        ["Light.strength", "Light.soft_edge", "Light.shadows"]
    );
    assert_eq!(w.complete("set light.str").line, "set Light.strength ");
    assert_eq!(w.complete("tool ro").line, "tool rotate ");
    assert_eq!(w.complete("look ").choices.len(), 3);
    assert_eq!(w.complete("wobble x").line, "wobble x", "nothing to offer");
    assert_eq!(w.complete("").choices.len(), COMMANDS.len());
}

#[test]
fn the_history_walks_back_and_forth_and_keeps_what_was_typed() {
    let mut c = crate::console::Console::default();
    c.older();
    assert_eq!(c.input, "", "nothing to walk");
    for line in ["help", "select Crate", "select Crate", "frame"] {
        c.remember(line);
    }
    assert_eq!(c.history(), ["help", "select Crate", "frame"], "no repeats");
    c.input = "half typ".into();
    c.older();
    assert_eq!(c.input, "frame");
    c.older();
    c.older();
    c.older();
    assert_eq!(c.input, "help", "stops at the oldest");
    c.newer();
    assert_eq!(c.input, "select Crate");
    c.newer();
    c.newer();
    assert_eq!(c.input, "half typ", "back to what was typed");
    c.newer();
    assert_eq!(c.input, "half typ");

    // Saved history is read back, the newest kept, junk left out.
    let mut saved: Vec<String> = (0..150).map(|i| format!("find {i}")).collect();
    saved.push("two\nlines".into());
    saved.push(" ".into());
    saved.push("x".repeat(5_000));
    c.set_history(saved);
    assert_eq!(c.history().len(), crate::console::HISTORY_LIMIT);
    assert_eq!(c.history().last().unwrap(), "find 149");
}

#[test]
fn any_command_line_is_answered_without_a_panic() {
    // A small fuzz: random lines from command names, field names, numbers,
    // quotes and junk. Each gets an answer, and the area stays sound.
    let (mut w, crate_id) = with_a_crate();
    w.add_point_light();
    let pieces = [
        "select",
        "set",
        "get",
        "new",
        "delete",
        "duplicate",
        "undo",
        "redo",
        "find",
        "help",
        "tool",
        "look",
        "stats",
        "frame",
        "Transform.position",
        "Light.color",
        "Light.kind",
        "Transform",
        "Light.",
        ".",
        "1",
        "-2.5",
        "1e309",
        "#ff00ff",
        "\"",
        "all",
        "none",
        "Crate",
        "\"Point light 1\"",
        "spot",
        "on",
        "NaN",
        "é",
        "😀",
        "\t",
        ",",
        "#",
        "°",
    ];
    let mut seed = 0x5EED_CAFEu64;
    for _ in 0..3_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let count = 1 + seed % 6;
        let line: Vec<&str> = (0..count)
            .map(|i| pieces[((seed >> (i * 7)) as usize) % pieces.len()])
            .collect();
        let line = line.join(if seed.is_multiple_of(5) { "" } else { " " });
        w.console.clear();
        run(&mut w, &line);
        if !line.trim().is_empty() {
            assert!(!w.log_lines().is_empty(), "{line:?} got an answer");
        }
        let _ = w.complete(&line);
        if w.document.area().len() > 200 {
            w.new_area();
        }
    }
    let _ = crate_id;
}

#[test]
fn the_quote_key_opens_the_command_line_and_enter_runs_it() {
    let (w, crate_id) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.key_press(egui::Key::Quote);
    h.event(egui::Event::Text("'".into()));
    h.run();
    h.run();
    assert!(
        h.state().0.console.input.is_empty(),
        "the ' isn't typed into it: {:?}",
        h.state().0.console.input
    );
    assert!(
        h.ctx
            .memory(|m| m.has_focus(crate::console::command_line_id()))
    );
    h.event(egui::Event::Text("select cr".into()));
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    h.run();
    assert_eq!(h.state().0.console.input, "select Crate ");
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
    assert_eq!(h.state().0.document.selection.primary(), Some(crate_id));
    assert!(h.state().0.console.input.is_empty());
    assert!(
        h.ctx
            .memory(|m| m.has_focus(crate::console::command_line_id())),
        "it stays in the command line for the next one"
    );
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(h.state().0.console.input, "select Crate");
    assert_eq!(h.state().0.console_history(), ["select Crate"]);
}

#[test]
fn the_quote_key_is_left_alone_while_typing_elsewhere() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    // The first text box (a search box), not the command line.
    h.query_all_by_role(egui::accesskit::Role::TextInput)
        .next()
        .unwrap()
        .click();
    h.run();
    assert!(h.ctx.memory(|m| m.focused().is_some()));
    h.key_press(egui::Key::Quote);
    h.event(egui::Event::Text("'".into()));
    h.run();
    assert!(
        !h.ctx
            .memory(|m| m.has_focus(crate::console::command_line_id()))
    );
}

#[test]
fn seed_and_checksum_say_what_they_do() {
    let mut w = Workbench::new(info(), None);
    run(&mut w, "seed");
    assert_eq!(answer(&w), "Each Play has a new seed.");
    run(&mut w, "seed 1234");
    assert_eq!(w.fixed_seed, Some(1234));
    run(&mut w, "seed twelve");
    assert_eq!(said(&w).0, LogLevel::Error);
    run(&mut w, "checksum");
    assert!(said(&w).1.contains("press Play first"));
    w.play();
    run(&mut w, "seed");
    assert_eq!(answer(&w), "The game's seed is 1234, chosen for every Play.");
    run(&mut w, "checksum");
    assert!(answer(&w).starts_with("At tick 0: world "), "{}", answer(&w));
    w.stop();
    run(&mut w, "seed new");
    assert_eq!(w.fixed_seed, None);
}
