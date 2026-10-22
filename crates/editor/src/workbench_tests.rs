//! Tests of the whole editor: the workbench's state, and its panels drawn
//! with egui_kittest, without a GPU.

use super::*;
use crate::viewport::NoRenderer;
use egui_kittest::kittest::Queryable;

fn info() -> AppInfo {
    AppInfo {
        name: "Test Engine".into(),
        version: "9.9.9".into(),
    }
}

#[test]
fn the_title_names_program_version_and_scene() {
    let w = Workbench::new(info(), None);
    assert_eq!(w.title(), "Test Engine 9.9.9 · Untitled scene");
}

#[test]
fn a_new_workbench_uses_the_default_layout() {
    let w = Workbench::new(info(), None);
    assert!(layout::is_complete(w.layout()));
    assert_eq!(w.tool(), Tool::Select);
    assert_eq!(w.accent(), Accent::PetrolBlue);
}

#[test]
fn the_console_keeps_a_limited_number_of_messages() {
    let mut w = Workbench::new(info(), None);
    for i in 0..(CONSOLE_LIMIT + 10) {
        w.log(LogLevel::Info, format!("line {i}"));
    }
    let lines = w.log_lines();
    assert!(lines.len() <= CONSOLE_LIMIT && lines.len() > CONSOLE_LIMIT / 2);
    assert_eq!(
        lines.last().unwrap().text,
        format!("line {}", CONSOLE_LIMIT + 9)
    );
    // The oldest go first, and the counts follow.
    assert_eq!(
        lines[0].text,
        format!("line {}", CONSOLE_LIMIT + 10 - lines.len())
    );
    assert_eq!(w.console.count(LogLevel::Info), lines.len());
}

#[test]
fn the_console_filters_by_level_and_search() {
    let mut c = Console::default();
    c.push(LogLevel::Info, "Graphics: Vulkan".into());
    c.push(LogLevel::Warning, "pairs() order".into());
    c.push(LogLevel::Error, "Shader failed".into());
    assert_eq!(c.shown().count(), 3);
    c.hidden.insert(LogLevel::Info);
    assert_eq!(c.shown().count(), 2);
    c.search = "SHADER".into();
    assert_eq!(c.shown().count(), 1);
    assert_eq!(c.count(LogLevel::Warning), 1);
    c.clear();
    assert_eq!((c.shown().count(), c.count(LogLevel::Error)), (0, 0));
}

#[test]
fn presets_rearrange_the_panels() {
    let mut w = Workbench::new(info(), None);
    w.apply_preset(Preset::SmallScreen);
    let path = w.layout().find_tab(&Panel::Console).unwrap();
    let group = w.layout()[path.surface][path.node].tabs().unwrap().to_vec();
    assert_eq!(
        group,
        [Panel::Library, Panel::Code, Panel::Console, Panel::Problems]
    );
}

/// The whole editor in a test window, with no GPU behind the viewport.
fn harness() -> egui_kittest::Harness<'static, Workbench> {
    let mut renderer = NoRenderer {
        reason: "testing".into(),
    };
    let mut themed = false;
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            move |ui, workbench: &mut Workbench| {
                if !themed {
                    theme::apply(ui.ctx(), workbench.accent());
                    themed = true;
                }
                let _ = workbench.show(ui, &mut renderer);
            },
            Workbench::new(info(), None),
        )
}

#[test]
fn the_editor_draws_every_panel_without_a_gpu() {
    let mut h = harness();
    h.run();
    for label in ["Viewport", "Scene", "Properties", "Console", "Library"] {
        let _ = h.get_by_label(label);
    }
    let _ = h.get_by_label("Untitled scene");
    let _ = h.get_by_label("No 3D view: testing");
}

#[test]
fn tool_keys_pick_tools() {
    let mut h = harness();
    h.run();
    h.key_press(egui::Key::W);
    h.run();
    assert_eq!(h.state().tool(), Tool::Move);
    h.key_press(egui::Key::T);
    h.run();
    assert_eq!(h.state().tool(), Tool::Transform);
}

#[test]
fn clicking_a_tool_button_picks_it() {
    let mut h = harness();
    h.run();
    h.get_by_label("Rotate (R)").click();
    h.run();
    assert_eq!(h.state().tool(), Tool::Rotate);
}

#[test]
fn the_viewport_menu_hides_the_tool_bar() {
    let mut h = harness();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::Button, "≡")
        .click();
    h.run();
    h.get_by_label("Tools").click();
    h.run();
    assert!(!h.state().overlays().tools);
    assert!(h.query_by_label("Move (W)").is_none());
}

#[test]
fn play_says_when_it_arrives() {
    let mut h = harness();
    h.run();
    h.get_by_label("Play").click();
    h.run();
    let last = h.state().log_lines().last().map(|l| l.text.clone());
    assert_eq!(
        last.as_deref(),
        Some("Playing the scene arrives in Phase 3 (ROADMAP.md).")
    );
}

/// A point in the middle of the viewport, clear of its corner controls.
const IN_VIEW: egui::Pos2 = egui::pos2(500.0, 300.0);

fn button(
    h: &egui_kittest::Harness<'_, Workbench>,
    button: egui::PointerButton,
    pressed: bool,
    at: egui::Pos2,
) {
    h.event(egui::Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

#[test]
fn holding_right_mouse_and_w_flies_forward_without_changing_tool() {
    let mut h = harness();
    h.hover_at(IN_VIEW);
    h.run();
    let start = h.state().camera();
    button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
    h.run();
    assert!(h.state().viewport.is_flying());
    h.key_down(egui::Key::W);
    h.run_steps(5);
    h.key_up(egui::Key::W);
    button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
    h.run();
    assert!(!h.state().viewport.is_flying());
    let moved = h.state().camera().position - start.position;
    assert!(moved.dot(start.forward()) > 0.0, "moved {moved}");
    assert_eq!(
        h.state().tool(),
        Tool::Select,
        "W flew instead of picking Move"
    );
}

#[test]
fn right_dragging_looks_around() {
    let mut h = harness();
    h.hover_at(IN_VIEW);
    h.run();
    let start = h.state().camera();
    button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
    h.run();
    h.event(egui::Event::PointerMoved(IN_VIEW + egui::vec2(80.0, 0.0)));
    h.run();
    button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
    h.run();
    assert!(h.state().camera().heading > start.heading);
    assert_eq!(h.state().camera().position, start.position);
}

#[test]
fn the_wheel_changes_speed_while_flying_and_moves_otherwise() {
    let wheel = |h: &egui_kittest::Harness<'_, Workbench>| {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
    };
    let mut h = harness();
    h.hover_at(IN_VIEW);
    h.run();
    let start = h.state().camera();
    wheel(&h);
    h.run();
    let after_wheel = h.state().camera();
    assert!((after_wheel.position - start.position).dot(start.forward()) > 0.0);
    assert_eq!(after_wheel.speed, start.speed);

    button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
    h.run();
    wheel(&h);
    h.run();
    button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
    h.run();
    assert_eq!(h.state().camera().speed, start.speed + 1);
    assert_eq!(h.state().camera().position, after_wheel.position);
    let _ = h.get_by_label(&(start.speed + 1).to_string());
}

#[test]
fn middle_dragging_slides_the_camera() {
    let mut h = harness();
    h.hover_at(IN_VIEW);
    h.run();
    let start = h.state().camera();
    button(&h, egui::PointerButton::Middle, true, IN_VIEW);
    h.run();
    for step in 1..=5 {
        h.event(egui::Event::PointerMoved(
            IN_VIEW + egui::vec2(step as f32 * 20.0, 0.0),
        ));
        h.run();
    }
    button(
        &h,
        egui::PointerButton::Middle,
        false,
        IN_VIEW + egui::vec2(100.0, 0.0),
    );
    h.run();
    let moved = h.state().camera().position - start.position;
    assert!(moved.dot(start.right()) < 0.0, "moved {moved}");
    assert_eq!(h.state().camera().heading, start.heading);
}

#[test]
fn tool_keys_still_work_after_flying() {
    let mut h = harness();
    h.hover_at(IN_VIEW);
    h.run();
    button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
    h.run();
    button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
    h.run();
    h.key_press(egui::Key::R);
    h.run();
    assert_eq!(h.state().tool(), Tool::Rotate);
}

#[test]
fn console_messages_carry_the_time() {
    let mut w = Workbench::new(info(), None);
    w.log(LogLevel::Info, "hello");
    let time = &w.log_lines()[0].time;
    let parts: Vec<_> = time.split(':').collect();
    assert_eq!(parts.len(), 3, "{time}");
    assert!(
        parts
            .iter()
            .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_digit()))
    );
}

#[test]
fn a_level_filter_hides_and_shows_its_messages() {
    let mut h = harness();
    h.state_mut().log(LogLevel::Warning, "Shader slow to build");
    h.run();
    assert!(h.query_by_label("Shader slow to build").is_some());
    h.get_by_label("Warnings").click();
    h.run();
    assert!(h.query_by_label("Shader slow to build").is_none());
    h.get_by_label("Warnings").click();
    h.run();
    assert!(h.query_by_label("Shader slow to build").is_some());
}

#[test]
fn every_panel_has_a_named_tab_and_the_bar_has_its_buttons() {
    let mut h = harness();
    h.run();
    for panel in Panel::ALL {
        let _ = h.get_by_label(panel.title());
    }
    for button in [
        "Play",
        "Play with two players",
        "Stop",
        "Build ▾",
        "Build & Play",
    ] {
        let _ = h.get_by_label(button);
    }
}

#[test]
fn play_with_others_says_when_it_arrives() {
    let mut h = harness();
    h.run();
    h.get_by_label("Play with two players").click();
    h.run();
    let last = h.state().log_lines().last().map(|l| l.text.clone());
    assert!(last.is_some_and(|t| t.contains("Phase 8")));
}

/// Part of the scale check (`tools/scale/scale-check.sh`, decision
/// 0011): how long the editor takes to lay out and draw one frame of
/// its interface, with a full console. Not run with the normal tests.
#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_editor_frame() {
    const TARGET_MS: f64 = 4.0;
    let mut h = harness();
    for i in 0..CONSOLE_LIMIT {
        h.state_mut().log(LogLevel::Info, format!("Message {i}"));
    }
    h.run();
    let frames = 200;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!("Scale check: editor frame {ms:.2} ms (target {TARGET_MS} ms) {verdict}");
    assert!(ms <= TARGET_MS, "the editor takes {ms:.2} ms per frame");
}

#[test]
fn the_look_menu_switches_the_look() {
    let mut h = harness();
    h.run();
    h.get_by_label("Standard ▴").click();
    h.run();
    h.get_by_label("Stylized").click();
    h.run();
    assert_eq!(h.state().look().kind, crate::look::LookKind::Stylized);
    // The menu stays open while its settings change, and the button now
    // names the look.
    let _ = h.get_by_label("Stylized ▴");
    h.get_by_label("2").click();
    h.run();
    assert_eq!(h.state().look().bands, 2);
    h.get_by_label("Outlines").click();
    h.run();
    assert!(!h.state().look().outlines);
}

/// The whole editor in a test window, keeping every action it asks for.
fn acting(
    workbench: Workbench,
) -> egui_kittest::Harness<'static, (Workbench, Vec<WorkbenchAction>)> {
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            |ui, state: &mut (Workbench, Vec<WorkbenchAction>)| {
                let mut renderer = NoRenderer {
                    reason: "testing".into(),
                };
                let actions = state.0.show(ui, &mut renderer);
                state.1.extend(actions);
            },
            (workbench, Vec::new()),
        )
}

fn with_a_model() -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.add_model(
        ModelInfo {
            id: ModelId(7),
            name: "Crate".into(),
            triangles: 12,
            min: glam::Vec3::ZERO,
            max: glam::Vec3::ONE,
        },
        &[],
        0.1,
    );
    w
}

#[test]
fn the_cross_in_the_scene_panel_removes_a_model() {
    let mut h = acting(with_a_model());
    h.run();
    h.get_by_label("Remove Crate").click();
    h.run();
    assert!(
        h.state()
            .1
            .contains(&WorkbenchAction::RemoveModel(ModelId(7)))
    );
    assert!(h.state().0.models().is_empty());
}

#[test]
fn remove_in_properties_removes_the_selected_model() {
    let mut w = with_a_model();
    w.select(Selection::Model(ModelId(7)));
    let mut h = acting(w);
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    assert!(
        h.state()
            .1
            .contains(&WorkbenchAction::RemoveModel(ModelId(7)))
    );
    assert_eq!(h.state().0.selection(), Selection::Nothing);
}

#[test]
fn the_console_clears_and_searches_from_its_panel() {
    let mut w = Workbench::new(info(), None);
    w.log(LogLevel::Warning, "Texture too big");
    w.log(LogLevel::Info, "Imported crate");
    let mut h = acting(w);
    h.run();
    assert!(h.query_by_label("Texture too big").is_some());
    h.get_by_label("Clear").click();
    h.run();
    assert!(h.query_by_label("Texture too big").is_none());
    assert!(h.state().0.log_lines().is_empty());
}

#[test]
fn help_offers_the_sample_scene() {
    let mut h = acting(Workbench::new(info(), None));
    h.run();
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label("Add the sample scene").click();
    h.run();
    assert!(h.state().1.contains(&WorkbenchAction::AddSampleScene));
}

#[test]
fn the_entity_menu_adds_lamps_shown_in_scene_and_properties() {
    let mut h = harness();
    h.run();
    // The top bar's menu (the Scene panel's menu row also says Entity).
    h.get_all_by_label("Entity").next().unwrap().click();
    h.run();
    h.get_by_label("Add point light").click();
    h.run();
    let w = h.state();
    assert_eq!(w.lamps().len(), 1);
    let lamp = &w.lamps()[0];
    assert_eq!(lamp.name, "Point light 1");
    assert_eq!(w.selection(), Selection::Lamp(lamp.id));
    assert_eq!(
        lamp.position,
        w.camera().focus() + glam::Vec3::Z * Lamp::POINT_HEIGHT
    );
    // Listed in the Scene panel, its fields in Properties.
    assert!(h.query_all_by_label("Point light 1").next().is_some());
    let _ = h.get_by_label("Move here");
    // The top bar's menu (the Scene panel's menu row also says Entity).
    h.get_all_by_label("Entity").next().unwrap().click();
    h.run();
    h.get_by_label("Add spotlight").click();
    h.run();
    let spot = h.state().lamps()[1].clone();
    assert_eq!(spot.position, h.state().camera().position);
    assert!(matches!(spot.kind, crate::lighting::LampKind::Spot { .. }));
    // Delete removes it and clears the selection.
    h.get_by_label("Delete").click();
    h.run();
    assert_eq!(h.state().lamps().len(), 1);
    assert_eq!(h.state().selection(), Selection::Nothing);
}

#[test]
fn the_sun_is_selected_from_the_scene_panel() {
    let mut h = harness();
    h.run();
    h.get_all_by_label("Sun").next().unwrap().click();
    h.run();
    assert_eq!(h.state().selection(), Selection::Sun);
    let _ = h.get_by_label("Heading");
    let _ = h.get_by_label("Shadows");
}

#[test]
fn lamps_from_a_model_leave_with_it() {
    let mut w = Workbench::new(info(), None);
    let model = ModelInfo {
        id: ModelId(4),
        name: "Room".into(),
        triangles: 2,
        min: glam::Vec3::ZERO,
        max: glam::Vec3::ONE,
    };
    w.add_model(model, &[], 0.1);
    let from_file = LampFromFile {
        name: "Bulb".into(),
        cone: None,
        position: glam::Vec3::ONE,
        direction: -glam::Vec3::Z,
        color: [1.0; 3],
        strength: 100.0,
        range: None,
    };
    w.add_model_lamps(ModelId(4), &[from_file.clone(), from_file]);
    w.add_point_light();
    assert_eq!(w.lamps().len(), 3);
    assert_eq!(w.lamps()[0].name, "Bulb");
    w.select(Selection::Lamp(w.lamps()[0].id));
    w.remove_model(ModelId(4));
    assert_eq!(w.lamps().len(), 1, "the hand-placed lamp stays");
    assert_eq!(w.selection(), Selection::Nothing);
    // Selecting something that isn't there selects nothing.
    w.select(Selection::Model(ModelId(99)));
    assert_eq!(w.selection(), Selection::Nothing);
}

#[test]
fn a_sun_from_a_file_sets_the_scene_sun() {
    let mut w = Workbench::new(info(), None).with_sun(SunSettings {
        shadows: false,
        ..SunSettings::default()
    });
    w.use_sun_from_file("Sun", -glam::Vec3::Z, [1.0, 0.0, 0.0], 7.0);
    let sun = w.sun();
    assert!((sun.height - 90.0).abs() < 1e-3);
    assert_eq!(
        (sun.color, sun.strength, sun.shadows),
        ([255, 0, 0], 7.0, false)
    );
}

#[test]
fn the_environment_is_edited_in_properties() {
    let mut h = harness();
    h.run();
    h.get_all_by_label("Environment").next().unwrap().click();
    h.run();
    assert_eq!(h.state().selection(), Selection::Environment);
    let _ = h.get_by_label("Exposure");
    h.get_by_label("On").click();
    h.run();
    assert!(h.state().environment().fog);
    let _ = h.get_by_label("Half hidden at");
    let saved = h.state().environment();
    let restored = Workbench::new(info(), None).with_environment(saved);
    assert_eq!(restored.environment(), saved);
}
