//! Tests of the whole editor: the workbench's state, and its panels drawn
//! with egui_kittest, without a GPU.

use super::*;
use crate::area_panel::AreaRequest;
use crate::commands::POINT_LIGHT_HEIGHT;
use crate::models::{FileLight, FileLightKind};
use crate::properties::PropertyRequest;
use crate::test_support::{lamp_post_file, with_a_lamp_post};
use crate::viewport::NoRenderer;
use area::Edit;
use area::builtin::Environment;
use area::builtin::{Light, LightKind, Transform};
use egui_kittest::kittest::Queryable;
use glam::Vec3;

fn info() -> AppInfo {
    AppInfo {
        name: "Test Engine".into(),
        version: "9.9.9".into(),
    }
}

#[test]
fn the_title_names_program_version_and_area() {
    let w = Workbench::new(info(), None);
    assert_eq!(w.title(), "Test Engine 9.9.9 · Untitled area");
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
        [
            Panel::Library,
            Panel::Code,
            Panel::Console,
            Panel::Problems,
            Panel::Profiler
        ]
    );
}

/// The whole editor in a test window, with no GPU behind the viewport.
fn harness() -> egui_kittest::Harness<'static, Workbench> {
    harness_with(Workbench::new(info(), None))
}

/// [`harness`], showing `workbench`.
fn harness_with(workbench: Workbench) -> egui_kittest::Harness<'static, Workbench> {
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
            workbench,
        )
}

#[test]
fn the_editor_draws_every_panel_without_a_gpu() {
    let mut h = harness();
    h.run();
    for label in ["Viewport", "Area", "Properties", "Console", "Library"] {
        let _ = h.get_by_label(label);
    }
    let _ = h.get_by_label("Untitled area");
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
fn play_says_what_it_does() {
    let mut h = harness();
    h.run();
    h.query_all_by_label("Play")
        .next()
        .expect("the Play button")
        .click();
    h.run();
    let last = h.state().log_lines().last().map(|l| l.text.clone());
    assert!(
        last.as_deref()
            .is_some_and(|l| l.starts_with("Playing. Changes made now are undone")),
        "{last:?}"
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
fn frames_are_wanted_only_while_something_moves() {
    let mut h = harness();
    h.hover_at(IN_VIEW);
    h.run();
    let ctx = h.ctx.clone();
    assert!(!h.state().wants_frames(&ctx), "nothing moves");
    let _ = h.get_by_label("Idle");
    // Flying: frames back to back, and the corner shows the frame rate.
    button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
    h.run();
    assert!(h.state().wants_frames(&ctx));
    h.step();
    let _ = h.get_by_label_contains("fps");
    button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
    h.run();
    assert!(!h.state().wants_frames(&ctx));
    // A running game wants frames; a paused one doesn't.
    h.state_mut().play();
    assert!(h.state().wants_frames(&ctx));
    h.state_mut().pause();
    assert!(!h.state().wants_frames(&ctx));
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
        "Play with two players",
        "Step one tick",
        "Stop",
        "Build ▾",
        "Build & Play",
    ] {
        let _ = h.get_by_label(button);
    }
    // The Play menu and the Play button.
    assert_eq!(h.query_all_by_label("Play").count(), 2);
}

#[test]
fn play_with_others_says_when_it_arrives() {
    let mut h = harness();
    h.run();
    h.get_by_label("Play with two players").click();
    h.run();
    let last = h.state().log_lines().last().map(|l| l.text.clone());
    assert!(last.is_some_and(|t| t.contains("Phase 16")));
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

fn crate_info() -> ModelInfo {
    ModelInfo {
        asset: AssetId(reflect::Id::from_raw(7).unwrap()),
        name: "Crate".into(),
        triangles: 12,
        lods: Vec::new(),
        min: Vec3::ZERO,
        max: Vec3::ONE,
        nodes: Vec::new(),
    }
}

fn with_a_model() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info(), None);
    let id = w.add_model(crate_info(), &[], 0.1, &[]);
    (w, id)
}

#[test]
fn an_imported_model_becomes_an_entity() {
    let (w, id) = with_a_model();
    let entity = w.document().area().get(id).unwrap();
    assert_eq!(entity.name, "Crate");
    assert!(entity.has("Transform") && entity.has("Model"));
    assert_eq!(w.models().len(), 1);
}

#[test]
fn the_cross_in_the_area_panel_deletes_a_model() {
    let (w, id) = with_a_model();
    let mut h = acting(w);
    h.run();
    h.get_by_label("Remove Crate").click();
    h.run();
    assert!(!h.state().0.document().area().contains(id));
    // The model stays loaded (an undo can bring it back).
    assert_eq!(h.state().0.models().len(), 1);
}

#[test]
fn delete_in_properties_deletes_the_selected_entity() {
    let (mut w, id) = with_a_model();
    w.select(id);
    let mut h = acting(w);
    h.run();
    let _ = h.get_by_label("Position");
    assert!(
        h.query_all_by_label("Model").count() >= 2,
        "component and field"
    );
    h.get_by_label("Delete").click();
    h.run();
    assert!(!h.state().0.document().area().contains(id));
    assert_eq!(h.state().0.selection(), None);
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
fn a_console_message_clicked_is_shown_in_full() {
    // The user test: long messages were cut off at the panel's edge, with
    // nothing saying the rest could be read.
    let long = format!("A long message {}the end", "and more ".repeat(60));
    let mut w = Workbench::new(info(), None);
    w.log(LogLevel::Warning, long.clone());
    let mut h = acting(w);
    h.run();
    h.get_by_label(&long).click();
    h.run();
    assert_eq!(
        h.state()
            .0
            .console
            .reading
            .as_ref()
            .map(|l| l.text.as_str()),
        Some(long.as_str())
    );
    assert_eq!(
        h.query_all_by_label(&long).count(),
        2,
        "listed, and in full"
    );
    h.get_by_label("Close").click();
    h.run();
    assert!(h.state().0.console.reading.is_none());
}

#[test]
fn help_offers_the_sample_area() {
    let mut h = acting(Workbench::new(info(), None));
    h.run();
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label("Add the sample area").click();
    h.run();
    assert!(h.state().1.contains(&WorkbenchAction::AddSampleArea));
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label("Make the sample game…").click();
    h.run();
    assert!(h.state().1.contains(&WorkbenchAction::MakeSampleGame));
}

#[test]
fn the_entity_menu_adds_lights_shown_in_area_and_properties() {
    let mut h = harness();
    h.run();
    // The top bar's menu (the Area panel's menu row also says Entity).
    h.get_all_by_label("Entity").next().unwrap().click();
    h.run();
    h.get_by_label("Add point light").click();
    h.run();
    let w = h.state();
    let id = w.selection().unwrap();
    let lamp = w.document().area().get(id).unwrap();
    assert_eq!(lamp.name, "Point light 1");
    assert_eq!(lamp.get::<Light>().unwrap().kind, LightKind::Point);
    assert_eq!(
        lamp.get::<Transform>().unwrap().position,
        w.camera().focus() + Vec3::Z * POINT_LIGHT_HEIGHT
    );
    // Listed in the Area panel, its fields in Properties.
    assert!(h.query_all_by_label("Point light 1").next().is_some());
    let _ = h.get_by_label("Strength");
    let _ = h.get_by_label("Move here");
    h.get_all_by_label("Entity").next().unwrap().click();
    h.run();
    h.get_by_label("Add spotlight").click();
    h.run();
    let spot = h.state().selection().unwrap();
    let data = h.state().document().area().get(spot).unwrap();
    assert_eq!(
        data.get::<Transform>().unwrap().position,
        h.state().camera().position
    );
    assert_eq!(data.get::<Light>().unwrap().kind, LightKind::Spot);
    // Delete removes it and clears the selection (scrolled to first:
    // the spotlight's settings fill the panel).
    h.get_by_label("Delete").scroll_to_me();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    assert!(!h.state().document().area().contains(spot));
    assert_eq!(h.state().selection(), None);
}

#[test]
fn add_entity_adds_an_empty_one_at_the_focus() {
    let mut w = Workbench::new(info(), None);
    let before = w.document().area().len();
    let id = w.add_entity();
    assert_eq!(w.document().area().len(), before + 1);
    let entity = w.document().area().get(id).unwrap();
    assert_eq!(entity.components.len(), 1);
    assert_eq!(
        entity.get::<Transform>().unwrap().position,
        w.camera().focus()
    );
    assert_eq!(w.selection(), Some(id));
}

#[test]
fn the_sun_is_selected_from_the_area_panel() {
    let mut h = harness();
    h.run();
    h.get_all_by_label("Sun").next().unwrap().click();
    h.run();
    let id = h.state().selection().unwrap();
    assert_eq!(h.state().document().area().get(id).unwrap().name, "Sun");
    let _ = h.get_by_label("Rotation");
    let _ = h.get_by_label("Shadows");
}

#[test]
fn lights_from_a_file_join_the_model_and_leave_with_it() {
    let mut w = Workbench::new(info(), None);
    let bulb = FileLight {
        name: "Bulb".into(),
        kind: FileLightKind::Point,
        position: Vec3::ONE,
        direction: -Vec3::Z,
        color: [1.0; 3],
        strength: 100.0,
        range: None,
    };
    let broken = FileLight {
        name: String::new(),
        kind: FileLightKind::Spot {
            inner: 0.2,
            outer: 0.5,
        },
        position: Vec3::NAN,
        strength: f32::INFINITY,
        range: Some(f32::NAN),
        ..bulb.clone()
    };
    let model = w.add_model(crate_info(), &[], 0.1, &[bulb, broken]);
    let hand_placed = w.add_point_light();
    let area = w.document().area();
    let children = area.children(Some(model));
    assert_eq!(children.len(), 2);
    assert_eq!(area.get(children[0]).unwrap().name, "Bulb");
    // A light with bad numbers is still usable.
    let second = area.get(children[1]).unwrap();
    assert_eq!(second.name, "Light 2");
    let t = second.get::<Transform>().unwrap();
    let l = second.get::<Light>().unwrap();
    assert!(t.position.is_finite());
    assert!(l.strength.is_finite() && l.range == 0.0);
    assert_eq!(l.kind, LightKind::Spot);
    w.select(children[0]);
    w.delete(model);
    assert!(
        w.document().area().contains(hand_placed),
        "the hand-placed light stays"
    );
    assert!(!w.document().area().contains(children[0]));
    assert_eq!(w.selection(), None);
    // Selecting something that isn't there selects nothing.
    w.select(children[0]);
    assert_eq!(w.selection(), None);
}

#[test]
fn a_sun_from_a_file_sets_the_area_sun() {
    let mut w = Workbench::new(info(), None);
    let suns = |w: &Workbench| {
        w.document()
            .area()
            .with::<Light>()
            .filter(|(_, l)| l.kind == LightKind::Sun)
            .collect::<Vec<_>>()
    };
    let sun = FileLight {
        name: "Sun".into(),
        kind: FileLightKind::Sun,
        position: Vec3::ZERO,
        direction: -Vec3::Z,
        color: [1.0, 0.0, 0.0],
        strength: 7.0,
        range: None,
    };
    w.add_model(crate_info(), &[], 0.1, &[sun]);
    let found = suns(&w);
    assert_eq!(
        found.len(),
        1,
        "the area's sun is changed, not a second added"
    );
    let (id, light) = &found[0];
    assert_eq!((light.color, light.strength), ([1.0, 0.0, 0.0], 7.0));
    let view = AreaView::of(w.document().area(), w.models());
    let direction = view.sun.unwrap().direction;
    assert!(direction.abs_diff_eq(-Vec3::Z, 1e-4), "{direction}");
    assert!(
        w.document()
            .area()
            .get(*id)
            .unwrap()
            .get::<Light>()
            .unwrap()
            .shadows
    );
}

#[test]
fn fields_change_through_property_requests() {
    let mut w = Workbench::new(info(), None);
    let env = w.document().area().with::<Environment>().next().unwrap().0;
    w.apply_property(
        PropertyRequest::SetField {
            entity: env,
            component: "Environment".into(),
            field: "fog".into(),
            value: reflect::Value::Bool(true),
        },
        0,
    );
    w.apply_property(
        PropertyRequest::Rename {
            entity: env,
            name: "World".into(),
        },
        0,
    );
    let data = w.document().area().get(env).unwrap();
    assert!(data.get::<Environment>().unwrap().fog);
    assert_eq!(data.name, "World");
    w.apply_property(
        PropertyRequest::RemoveComponent {
            entity: env,
            component: "Environment".into(),
        },
        0,
    );
    assert!(!w.document().area().get(env).unwrap().has("Environment"));
    // Requests about entities that are gone change nothing.
    w.delete(env);
    w.apply_property(
        PropertyRequest::Rename {
            entity: env,
            name: "Ghost".into(),
        },
        0,
    );
    w.apply_property(PropertyRequest::MoveHere(env), 0);
    assert!(!w.document().area().contains(env));
}

#[test]
fn the_environment_is_shown_in_properties() {
    let mut h = harness();
    h.run();
    h.get_all_by_label("Environment").next().unwrap().click();
    h.run();
    let _ = h.get_by_label("Exposure");
    let _ = h.get_by_label("Tone mapping");
}

#[test]
fn a_component_nobody_knows_is_kept_and_can_be_removed() {
    let mut w = Workbench::new(info(), None);
    let id = w.add_entity();
    w.document
        .edit(
            "Add Wobble",
            vec![Edit::AddComponent {
                entity: id,
                component: "Wobble".into(),
                data: reflect::ComponentData::default(),
            }],
            None,
            None,
        )
        .unwrap();
    let mut h = harness_with(w);
    h.run();
    let _ = h.get_by_label("Missing component: Wobble");
    h.get_by_label("Remove").click();
    h.run();
    assert!(!h.state().document().area().get(id).unwrap().has("Wobble"));
}

fn strength_of(w: &Workbench, id: EntityId) -> f32 {
    w.document()
        .area()
        .get(id)
        .unwrap()
        .get::<Light>()
        .unwrap()
        .strength
}

fn set_strength(id: EntityId, value: f64) -> PropertyRequest {
    PropertyRequest::SetField {
        entity: id,
        component: "Light".into(),
        field: "strength".into(),
        value: reflect::Value::Float(value),
    }
}

#[test]
fn a_drag_undoes_in_one_step_with_ctrl_z_and_redoes_from_the_edit_menu() {
    let mut w = Workbench::new(info(), None);
    let lamp = w.add_point_light();
    // One drag: the same gesture, many values.
    for v in [1100.0, 1200.0, 1300.0] {
        w.apply_property(set_strength(lamp, v), 7);
    }
    w.select(EntityId(reflect::Id::from_raw(1).unwrap()));
    let mut h = harness_with(w);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(strength_of(h.state(), lamp), 1000.0);
    assert_eq!(
        h.state().selection(),
        Some(lamp),
        "undo shows what it changed"
    );
    // The top bar's menu (panels' menu rows also say Edit).
    h.get_all_by_label("Edit").next().unwrap().click();
    h.run();
    h.get_by_label_contains("Redo Change Light.strength")
        .click();
    h.run();
    assert_eq!(strength_of(h.state(), lamp), 1300.0);
    // Undoing everything removes the light; Ctrl+Y brings it back.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(!h.state().document().area().contains(lamp));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run();
    assert!(h.state().document().area().contains(lamp));
    h.key_press_modifiers(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::Z,
    );
    h.run();
    assert_eq!(strength_of(h.state(), lamp), 1300.0);
}

#[test]
fn separate_drags_are_separate_steps() {
    let mut w = Workbench::new(info(), None);
    let lamp = w.add_point_light();
    w.apply_property(set_strength(lamp, 1100.0), 1);
    w.apply_property(set_strength(lamp, 1200.0), 2);
    w.undo();
    assert_eq!(strength_of(&w, lamp), 1100.0);
    // A drag that ends where it started leaves no step.
    let steps = w.document().history().undo_len();
    w.apply_property(set_strength(lamp, 1500.0), 3);
    w.apply_property(set_strength(lamp, 1100.0), 3);
    assert_eq!(w.document().history().undo_len(), steps);
}

#[test]
fn an_import_is_one_step_with_its_lights_and_sun() {
    let mut w = Workbench::new(info(), None);
    let start = w.document().area().clone();
    let lights = [
        FileLight {
            name: "Bulb".into(),
            kind: FileLightKind::Point,
            position: Vec3::ONE,
            direction: -Vec3::Z,
            color: [1.0; 3],
            strength: 100.0,
            range: None,
        },
        FileLight {
            name: "Sun".into(),
            kind: FileLightKind::Sun,
            position: Vec3::ZERO,
            direction: -Vec3::X,
            color: [1.0, 0.5, 0.0],
            strength: 9.0,
            range: None,
        },
    ];
    let model = w.add_model(crate_info(), &[], 0.1, &lights);
    assert_eq!(w.document().history().undo_label(), Some("Import Crate"));
    assert_eq!(w.document().history().undo_len(), 1);
    w.undo();
    assert_eq!(
        w.document().area(),
        &start,
        "model, lamp and sun change all undone"
    );
    w.redo();
    assert_eq!(w.document().area().children(Some(model)).len(), 1);
}

#[test]
fn delete_undoes_with_everything_under_it() {
    let mut w = Workbench::new(info(), None);
    let bulb = FileLight {
        name: "Bulb".into(),
        kind: FileLightKind::Point,
        position: Vec3::ONE,
        direction: -Vec3::Z,
        color: [1.0; 3],
        strength: 100.0,
        range: None,
    };
    let model = w.add_model(crate_info(), &[], 0.1, &[bulb]);
    let before = w.document().area().clone();
    w.select(model);
    w.delete(model);
    assert_eq!(w.selection(), None);
    w.undo();
    assert_eq!(w.document().area(), &before);
    assert_eq!(w.selection(), Some(model));
    // Move here is undoable too.
    w.move_here(model);
    w.undo();
    assert_eq!(w.document().area(), &before);
}

/// A workbench with three lights in a row at the top level.
fn three_lights() -> (Workbench, [EntityId; 3]) {
    let mut w = Workbench::new(info(), None);
    let a = w.add_point_light();
    let b = w.add_point_light();
    let c = w.add_spotlight();
    (w, [a, b, c])
}

#[test]
fn new_entities_go_to_the_end_of_the_list() {
    let (w, [a, b, c]) = three_lights();
    let order = w.document().area().depth_first();
    let at = |id| order.iter().position(|&e| e == id).unwrap();
    assert!(at(a) < at(b) && at(b) < at(c), "{order:?}");
}

#[test]
fn ctrl_and_shift_clicks_build_a_selection() {
    let (mut w, [a, b, c]) = three_lights();
    w.select(a);
    w.apply_area_request(AreaRequest::SelectRange(c));
    let selected: Vec<_> = w.document().selection.iter().collect();
    assert!(selected.contains(&a) && selected.contains(&b) && selected.contains(&c));
    assert_eq!(w.selection(), Some(a), "Properties keeps the first one");
    w.apply_area_request(AreaRequest::Toggle(b));
    assert!(!w.document().selection.contains(b));
    w.apply_area_request(AreaRequest::Toggle(b));
    assert_eq!(w.selection(), Some(b));
}

#[test]
fn dragging_onto_another_keeps_the_place_in_the_world_and_undoes() {
    let (mut w, [a, b, _]) = three_lights();
    // Move and turn the new parent, so its children's numbers must change.
    w.apply_property(
        PropertyRequest::SetField {
            entity: a,
            component: "Transform".into(),
            field: "rotation".into(),
            value: reflect::Value::quat(glam::Quat::from_rotation_z(1.0)),
        },
        1,
    );
    let before = w.document().area().world_matrix(b);
    w.apply_area_request(AreaRequest::Reparent {
        entity: b,
        onto: Some(a),
    });
    let area = w.document().area();
    assert_eq!(area.get(b).unwrap().parent, Some(a));
    let after = area.world_matrix(b);
    assert!(after.abs_diff_eq(before, 1e-4), "stays put in the world");
    assert_eq!(
        w.document().history().undo_label(),
        Some("Move under Point light 1")
    );
    // A parent can't go under its own child.
    w.apply_area_request(AreaRequest::Reparent {
        entity: a,
        onto: Some(b),
    });
    assert_eq!(w.document().area().get(a).unwrap().parent, None);
    // Dropping on empty space goes back to the top level.
    w.apply_area_request(AreaRequest::Reparent {
        entity: b,
        onto: None,
    });
    assert_eq!(w.document().area().get(b).unwrap().parent, None);
    assert!(
        w.document()
            .area()
            .world_matrix(b)
            .abs_diff_eq(before, 1e-4)
    );
    w.undo();
    w.undo();
    assert_eq!(w.document().area().get(b).unwrap().parent, None);
    assert!(
        w.document()
            .area()
            .world_matrix(b)
            .abs_diff_eq(before, 1e-4)
    );
}

#[test]
fn dragging_a_selected_entity_moves_the_whole_selection() {
    let (mut w, [a, b, c]) = three_lights();
    w.select(b);
    w.apply_area_request(AreaRequest::Toggle(c));
    w.apply_area_request(AreaRequest::Reparent {
        entity: c,
        onto: Some(a),
    });
    assert_eq!(w.document().area().children(Some(a)), [b, c]);
    assert_eq!(
        w.document().history().undo_len(),
        4,
        "three adds and one move"
    );
}

#[test]
fn duplicate_copies_the_tree_selects_the_copy_and_undoes_in_one_step() {
    let (mut w, [a, b, _]) = three_lights();
    w.apply_area_request(AreaRequest::Reparent {
        entity: b,
        onto: Some(a),
    });
    let count = w.document().area().len();
    w.select(a);
    w.duplicate_selection();
    let area = w.document().area();
    assert_eq!(area.len(), count + 2, "the light and the one under it");
    let copy = w.selection().unwrap();
    assert_ne!(copy, a);
    assert_eq!(area.get(copy).unwrap().name, "Point light 1");
    assert_eq!(area.children(Some(copy)).len(), 1, "with its own child");
    assert_eq!(area.children(Some(a)), [b], "the original keeps its child");
    w.undo();
    assert_eq!(w.document().area().len(), count);
    assert_eq!(w.selection(), Some(a));
}

#[test]
fn the_delete_key_deletes_the_whole_selection_in_one_step() {
    let (mut w, [a, b, c]) = three_lights();
    w.select(a);
    w.apply_area_request(AreaRequest::Toggle(c));
    let mut h = harness_with(w);
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    let area = h.state().document().area();
    assert!(!area.contains(a) && !area.contains(c) && area.contains(b));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(h.state().document().area().contains(a));
    assert!(h.state().document().area().contains(c));
}

#[test]
fn f2_renames_in_the_area_panel_and_ctrl_d_duplicates() {
    let (mut w, [a, ..]) = three_lights();
    w.select(a);
    let mut h = harness_with(w);
    h.run();
    h.key_press(egui::Key::F2);
    h.run();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("Lantern".into()));
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().document().area().get(a).unwrap().name, "Lantern");
    let count = h.state().document().area().len();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::D);
    h.run();
    assert_eq!(h.state().document().area().len(), count + 1);
}

#[test]
fn f_frames_the_selection() {
    let (mut w, _) = with_a_model();
    let far = Workbench::new(info(), None).camera().focus() + Vec3::new(50.0, 0.0, 0.0);
    w.viewport.camera.frame(far, far + Vec3::ONE);
    let model = w.document().area().depth_first()[0];
    let model = w
        .document()
        .area()
        .depth_first()
        .into_iter()
        .find(|&id| w.document().area().get(id).unwrap().has("Model"))
        .unwrap_or(model);
    w.select(model);
    w.frame_selection();
    // The camera now looks straight at the model's middle.
    let camera = w.camera();
    let to_middle = (Vec3::splat(0.5) - camera.position).normalize();
    assert!(to_middle.dot(camera.forward()) > 0.999, "{camera:?}");
}

/// A component a game might register at run time.
fn health_schema() -> reflect::ComponentSchema {
    reflect::ComponentSchema {
        name: "Health".into(),
        help: "How much damage it can take.".into(),
        fields: vec![
            reflect::FieldInfo::new(
                "max_hp",
                "Max HP",
                reflect::FieldKind::Float,
                reflect::Value::Float(100.0),
            ),
            reflect::FieldInfo::new(
                "team",
                "Team",
                reflect::FieldKind::Choice(vec!["Red".into(), "Blue".into()]),
                reflect::Value::Text("Red".into()),
            ),
        ],
    }
}

#[test]
fn a_component_registered_at_run_time_is_added_shown_edited_and_undone() {
    let mut w = Workbench::new(info(), None);
    w.register_component(health_schema()).unwrap();
    assert!(
        w.register_component(health_schema()).is_err(),
        "a name is taken once"
    );
    let id = w.add_entity();
    let mut h = harness_with(w);
    h.run();
    h.get_by_label("Add component").click();
    h.run();
    // Grouped: Solis's own, then the game's (the user test found one long
    // list hard to read).
    let _ = h.get_by_label("Built in");
    let _ = h.get_by_label("Game code");
    h.get_by_label("Health").click();
    h.run();
    assert!(h.state().document().area().get(id).unwrap().has("Health"));
    // Its fields are drawn from its schema, with no code for it.
    let _ = h.get_by_label("Max HP");
    let _ = h.get_by_label("Team");
    h.state_mut().apply_property(
        PropertyRequest::SetField {
            entity: id,
            component: "Health".into(),
            field: "max_hp".into(),
            value: reflect::Value::Float(5000.0),
        },
        9,
    );
    let hp = |w: &Workbench| {
        w.document().area().get(id).unwrap().components["Health"]
            .get("max_hp")
            .cloned()
    };
    assert_eq!(hp(h.state()), Some(reflect::Value::Float(5000.0)));
    assert_eq!(
        h.state().document().history().undo_label(),
        Some("Change Health.max_hp")
    );
    h.state_mut().undo();
    assert_eq!(hp(h.state()), Some(reflect::Value::Float(100.0)));
    // × takes it off; undo puts it back with its values.
    h.run();
    h.get_by_label("Remove Health").click();
    h.run();
    assert!(!h.state().document().area().get(id).unwrap().has("Health"));
    h.state_mut().undo();
    assert_eq!(hp(h.state()), Some(reflect::Value::Float(100.0)));
}

/// An area of `count` entities: models, lights and empties, some under
/// others, as a scale check's worth of a level.
fn crowded(count: usize) -> Workbench {
    let mut w = Workbench::new(info(), None);
    let info = crate_info();
    let asset = info.asset;
    w.models.insert(asset, info);
    let mut entities = Vec::new();
    let mut parent = None;
    for i in 0..count {
        let id = EntityId::random();
        let mut data = area::EntityData::new(&format!("Thing {i}")).with(&Transform {
            position: Vec3::new((i % 50) as f32, (i / 50) as f32, 0.0),
            ..Transform::default()
        });
        match i % 3 {
            0 => {
                data.set(&area::builtin::Model {
                    asset: Some(reflect::AssetRef { asset, part: None }),
                });
            }
            1 => data.set(&Light::point()),
            _ => {}
        }
        // Every tenth starts a group the next nine sit under.
        if i % 10 == 0 {
            parent = Some(id);
        } else {
            data.parent = parent;
        }
        data.order = area::order_after(Some(&format!("a{}", i % 10)));
        entities.push(Edit::Spawn {
            entities: vec![(id, data)],
        });
    }
    // Spawned parents first: each group's head comes before its members.
    w.document.edit("Fill", entities, None, None).unwrap();
    w
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_editor_with_2000_entities() {
    const TARGET_MS: f64 = 4.0;
    let mut w = crowded(2_000);
    let some = w.document().area().depth_first()[1_000];
    w.select(some);
    let mut h = harness_with(w);
    h.run();
    let frames = 100;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: editor frame with 2,000 entities listed and one inspected {ms:.2} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "the editor takes {ms:.2} ms per frame");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_editor_with_four_views() {
    const TARGET_MS: f64 = 4.0;
    let mut w = crowded(2_000);
    w.viewport.layout = crate::viewport::ViewLayout::Four;
    let some = w.document().area().depth_first()[1_000];
    w.select(some);
    w.viewport.tool = Tool::Move;
    let mut h = harness_with(w);
    h.run();
    let frames = 100;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: editor frame with four views, 2,000 entities and the gizmo {ms:.2} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "the editor takes {ms:.2} ms per frame");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_playing_while_dragging() {
    // While playing, the game is built again whenever the area changes:
    // every frame of a drag. Timed with a change each frame.
    const TARGET_MS: f64 = 4.0;
    let mut w = crowded(2_000);
    let some = w.document().area().depth_first()[1_000];
    w.select(some);
    w.play();
    let mut h = harness_with(w);
    h.run();
    let frames = 100;
    let start = std::time::Instant::now();
    for i in 0..frames {
        h.state_mut().apply_property(
            PropertyRequest::SetField {
                entity: some,
                component: "Transform".into(),
                field: "position".into(),
                value: reflect::Value::vec3(Vec3::new(f32::from(i as u8), 0.0, 0.0)),
            },
            77,
        );
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: editor frame while playing and dragging, 2,000 entities {ms:.2} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "the editor takes {ms:.2} ms per frame");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_dragging_everything() {
    // Ctrl+A and a Move drag: every entity changes every frame, and the
    // drag stays one undo step however long it lasts.
    const TARGET_MS: f64 = 8.0;
    let mut w = crowded(2_000);
    w.select_all();
    w.viewport.tool = Tool::Move;
    let mut h = harness_with(w);
    h.run();
    h.state_mut()
        .viewport
        .tools
        .events
        .push(crate::viewport_tools::ViewportEvent::GizmoStart);
    let frames = 100;
    let start = std::time::Instant::now();
    for i in 0..frames {
        h.state_mut().viewport.tools.events.push(
            crate::viewport_tools::ViewportEvent::GizmoChange {
                change: crate::gizmo::Change::Translate(Vec3::X * i as f32 * 0.1),
                flip: false,
            },
        );
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    assert_eq!(
        h.state().document().history().undo_len(),
        2,
        "fill and drag"
    );
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: dragging all 2,000 entities {ms:.2} ms a frame (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "the editor takes {ms:.2} ms per frame");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_unpacked_model_with_1000_parts() {
    // A big model unpacked: a thousand parts, each its own entity drawn
    // from one model, with all of them selected.
    const TARGET_MS: f64 = 4.0;
    let (mut w, model) = with_a_model();
    let mut info = crate_info();
    info.nodes = (0..1_000)
        .map(|i| crate::models::NodeInfo {
            part: i + 1,
            name: format!("Part {i}"),
            parent: (i > 0).then(|| (i as usize - 1) / 4),
            local: glam::Mat4::from_translation(Vec3::X),
            has_meshes: true,
        })
        .collect();
    w.reload_model(info, &[]);
    let start = std::time::Instant::now();
    w.unpack(model);
    let unpack_ms = start.elapsed().as_secs_f64() * 1000.0;
    w.select_all();
    let mut h = harness_with(w);
    h.run();
    let frames = 100;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: a 1,000-part model unpacked ({unpack_ms:.0} ms), all selected {ms:.2} ms a frame (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "the editor takes {ms:.2} ms per frame");
}

#[test]
fn a_crowded_area_still_lists_and_inspects() {
    let mut w = crowded(200);
    let some = w.document().area().depth_first()[100];
    w.select(some);
    let mut h = harness_with(w);
    h.run();
    let _ = h.get_by_label("Position");
    assert_eq!(h.state().document().area().len(), 202);
}

#[test]
fn the_library_lists_the_project_and_a_double_click_places() {
    let mut w = Workbench::new(info(), None);
    let mut h = acting(w);
    h.run();
    let _ = h.get_by_label_contains("No project is open");
    w = Workbench::new(info(), None);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    assert_eq!(w.title(), "Test Engine 9.9.9 · Harbour · Untitled area");
    let id = AssetId(reflect::Id::from_raw(42).unwrap());
    w.set_library(vec![crate::library::LibraryItem {
        id: Some(id),
        kind: crate::library::LibraryKind::Model,
        name: "Lighthouse".into(),
        path: "lighthouse.glb".into(),
        thumbnail: None,
    }]);
    let mut h = acting(w);
    h.run();
    h.get_by_label("Place Lighthouse").click();
    h.run();
    assert!(h.state().1.contains(&WorkbenchAction::PlaceAsset(id)));
}

#[test]
fn the_file_menu_asks_for_projects_and_lists_recent_ones() {
    let mut w = Workbench::new(info(), None);
    w.set_recent(vec!["/projects/harbour".into()]);
    let mut h = acting(w);
    h.run();
    h.get_all_by_label("File").next().unwrap().click();
    h.run();
    h.get_by_label("New project…").click();
    h.run();
    assert!(h.state().1.contains(&WorkbenchAction::NewProject));
    h.get_all_by_label("File").next().unwrap().click();
    h.run();
    h.get_by_label_contains("Recent projects").click();
    h.run();
    h.get_by_label("harbour").click();
    h.run();
    assert!(
        h.state()
            .1
            .contains(&WorkbenchAction::OpenRecent("/projects/harbour".into()))
    );
}

#[test]
fn opening_another_project_starts_a_new_area_and_reloads_swap_models() {
    let (mut w, _) = with_a_model();
    assert!(w.has_model(crate_info().asset));
    let mut bigger = crate_info();
    bigger.triangles = 99;
    w.reload_model(bigger, &["a texture was too big".into()]);
    assert_eq!(w.models()[&crate_info().asset].triangles, 99);
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("Reloaded Crate"))
    );
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Other".into(),
        root: "/other".into(),
        assets: std::path::PathBuf::from("/other").join("assets"),
    }));
    assert!(w.models().is_empty());
    assert_eq!(w.document().area().len(), 2, "a fresh area");
}

#[test]
fn the_tab_and_title_show_unsaved_changes_and_saving_clears_them() {
    let mut w = Workbench::new(info(), None);
    assert!(!w.document().is_dirty());
    w.add_point_light();
    assert!(w.document().is_dirty());
    assert_eq!(w.title(), "Test Engine 9.9.9 · Untitled area •");
    w.mark_saved("/levels/harbour.solarea".into());
    assert_eq!(w.title(), "Test Engine 9.9.9 · harbour");
    let mut h = harness_with(w);
    h.run();
    let _ = h.get_by_label("harbour");
    // Undo after saving is a change; redo is back at the save.
    h.state_mut().undo();
    assert!(h.state().document().is_dirty());
    h.state_mut().redo();
    assert!(!h.state().document().is_dirty());
}

#[test]
fn ctrl_s_saves_and_ctrl_shift_s_saves_as() {
    let mut h = acting(Workbench::new(info(), None));
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    h.key_press_modifiers(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::S,
    );
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::O);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    assert_eq!(
        h.state().1,
        [
            WorkbenchAction::SaveArea,
            WorkbenchAction::SaveAreaAs,
            WorkbenchAction::OpenArea,
            WorkbenchAction::NewArea
        ]
    );
}

#[test]
fn leaving_unsaved_changes_asks_first() {
    let mut w = Workbench::new(info(), None);
    w.add_point_light();
    let mut h = acting(w);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    assert!(h.state().1.is_empty(), "nothing happens before the answer");
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().1.is_empty());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert_eq!(
        h.state().1,
        [WorkbenchAction::SaveArea, WorkbenchAction::NewArea]
    );
    // Closing the window asks too; Don't save lets it close.
    assert!(!h.state_mut().0.close_requested());
    h.run();
    h.get_by_label("Don't save").click();
    h.run();
    assert_eq!(h.state().1.last(), Some(&WorkbenchAction::Quit));
    assert!(h.state_mut().0.close_requested());
}

#[test]
fn an_area_changed_on_disk_reloads_as_one_undoable_step() {
    let mut w = Workbench::new(info(), None);
    let start = w.document().area().clone();
    w.open_area(start.clone(), "/levels/harbour.solarea".into());
    let mut changed = start.clone();
    changed.spawn(area::EntityData::new("From Git"));
    w.offer_reload(changed.clone());
    assert_eq!(w.document().area(), &changed);
    assert!(!w.document().is_dirty(), "it matches the file");
    assert_eq!(
        w.document().history().undo_label(),
        Some("Reloaded harbour.solarea from disk")
    );
    w.undo();
    assert_eq!(w.document().area(), &start);
}

#[test]
fn with_unsaved_changes_a_reload_asks_keep_mine_or_load_the_file() {
    let mut w = Workbench::new(info(), None);
    w.open_area(
        area::builtin::starter_area(),
        "/levels/harbour.solarea".into(),
    );
    let mine = w.add_point_light();
    let mut theirs = w.document().area().clone();
    theirs.remove(mine);
    theirs.spawn(area::EntityData::new("From Git"));
    w.offer_reload(theirs.clone());
    assert!(
        w.document().area().contains(mine),
        "nothing changes before the answer"
    );
    let mut h = acting(w);
    h.run();
    h.get_by_label("Keep mine").click();
    h.run();
    assert!(h.state().0.document().area().contains(mine));
    h.state_mut().0.offer_reload(theirs.clone());
    h.run();
    h.get_by_label("Load the file").click();
    h.run();
    assert_eq!(h.state().0.document().area(), &theirs);
    // Ctrl+Z brings back what was there.
    h.state_mut().0.undo();
    assert!(h.state().0.document().area().contains(mine));
}

#[test]
fn placing_an_assembly_makes_a_copy_that_saves_as_one_line() {
    let (w, _, root, lamp) = with_a_lamp_post();
    let area = w.document().area();
    assert_eq!(area.get(lamp).unwrap().parent, Some(root));
    assert_eq!(
        area.get(root).unwrap().get::<Transform>().unwrap().position,
        w.camera().focus(),
        "placed where the camera looks"
    );
    let file = w.area_file().unwrap();
    assert!(!file.area.contains(lamp), "the copy's parts aren't written");
    // Only the root's new position is a change.
    assert_eq!(file.overrides.len(), 1);
    assert_eq!(w.expand_file(&file).unwrap(), *w.document().area());
}

#[test]
fn changed_fields_show_in_bold_and_revert_or_apply() {
    let (mut w, asset, _, lamp) = with_a_lamp_post();
    w.apply_property(
        PropertyRequest::SetField {
            entity: lamp,
            component: "Light".into(),
            field: "strength".into(),
            value: reflect::Value::Float(42.0),
        },
        3,
    );
    w.select(lamp);
    let part = w.assembly_part().unwrap();
    assert!(!part.root);
    assert_eq!(
        part.expected.unwrap().get::<Light>().unwrap().strength,
        1000.0,
        "the assembly's value"
    );
    let mut h = acting(w);
    h.run();
    let _ = h.get_by_label_contains("Part of");
    h.get_by_label("Apply Strength").click();
    h.run();
    assert!(h.state().1.iter().any(|a| matches!(
        a,
        WorkbenchAction::ApplyToAssembly { assembly, component, .. }
            if *assembly == asset && component == "Light"
    )));
    h.get_by_label("Revert Strength").click();
    h.run();
    let light = h
        .state()
        .0
        .document()
        .area()
        .get(lamp)
        .unwrap()
        .get::<Light>()
        .unwrap();
    assert_eq!(light.strength, 1000.0);
}

#[test]
fn an_assembly_that_now_places_an_unknown_one_keeps_its_old_version() {
    // A Git pull: the lamp post now holds a copy of a sign, whose file
    // isn't read yet.
    let (mut w, asset, root, lamp) = with_a_lamp_post();
    let before = w.document().area().clone();
    let sign = AssetId(reflect::Id::from_raw(901).unwrap());
    let (_, file) = lamp_post_file();
    let mut file = (*file).clone();
    let one = EntityId(reflect::Id::from_raw(1).unwrap());
    let holder = EntityId(reflect::Id::from_raw(3).unwrap());
    let mut copy = area::EntityData::new("Sign");
    copy.parent = Some(one);
    copy.origin = Some(area::Origin {
        assembly: sign,
        instance: holder,
        path: Vec::new(),
    });
    file.area.insert(holder, copy);
    let file = std::sync::Arc::new(file);
    w.set_assembly(asset, file.clone());
    assert_eq!(*w.document().area(), before, "nothing changed");
    assert!(
        !std::sync::Arc::ptr_eq(&w.assemblies()[&asset], &file),
        "the version the copies were made from stays in use"
    );
    assert_eq!(w.log_lines().last().unwrap().level, LogLevel::Error);
    // Once the sign is read (first, as the app now does), the update
    // goes through.
    let mut sign_area = area::Area::new();
    sign_area.insert(
        EntityId(reflect::Id::from_raw(5).unwrap()),
        area::EntityData::new("Sign board").with(&Transform::default()),
    );
    w.set_assembly(
        sign,
        std::sync::Arc::new(area::text::AreaFile {
            kind: area::text::FileKind::Assembly {
                root: EntityId(reflect::Id::from_raw(5).unwrap()),
            },
            area: sign_area,
            overrides: Default::default(),
        }),
    );
    w.set_assembly(asset, file.clone());
    assert!(std::sync::Arc::ptr_eq(&w.assemblies()[&asset], &file));
    assert!(
        w.document().area().len() > before.len(),
        "the sign arrived in the copy"
    );
    assert!(w.document().area().contains(root) && w.document().area().contains(lamp));
}

#[test]
fn a_changed_assembly_updates_its_copies_and_keeps_their_changes() {
    let (mut w, asset, root, lamp) = with_a_lamp_post();
    w.apply_property(
        PropertyRequest::SetField {
            entity: lamp,
            component: "Light".into(),
            field: "strength".into(),
            value: reflect::Value::Float(42.0),
        },
        3,
    );
    // The assembly's lamp turns orange (another person, a Git pull).
    let (_, file) = lamp_post_file();
    let mut file = (*file).clone();
    let two = EntityId(reflect::Id::from_raw(2).unwrap());
    assert!(area::assembly::set_in_file(
        &mut file,
        &[two],
        "Light",
        "color",
        reflect::Value::floats(&[1.0, 0.5, 0.0])
    ));
    w.set_assembly(asset, std::sync::Arc::new(file));
    let light = w
        .document()
        .area()
        .get(lamp)
        .unwrap()
        .get::<Light>()
        .unwrap();
    assert_eq!(
        light.color,
        [1.0, 0.5, 0.0],
        "the assembly's change arrives"
    );
    assert_eq!(light.strength, 42.0, "the copy's own change stays");
    assert!(w.document().area().contains(root));
}

#[test]
fn unpack_turns_a_copy_into_plain_entities() {
    let (mut w, _, root, lamp) = with_a_lamp_post();
    w.unpack(root);
    let area = w.document().area();
    assert!(area.get(root).unwrap().origin.is_none());
    assert!(area.get(lamp).unwrap().origin.is_none());
    let file = w.area_file().unwrap();
    assert!(
        file.overrides.is_empty() && file.area.contains(lamp),
        "written whole now"
    );
    w.undo();
    assert!(w.document().area().get(lamp).unwrap().origin.is_some());
}

#[test]
fn a_duplicated_copy_is_still_a_copy() {
    let (mut w, _, root, _) = with_a_lamp_post();
    w.select(root);
    w.duplicate_selection();
    let copy = w.selection().unwrap();
    assert_ne!(copy, root);
    let file = w.area_file().unwrap();
    let placed = file.area.iter().filter(|(_, e)| e.origin.is_some()).count();
    assert_eq!(placed, 2, "two copies of the assembly");
    assert_eq!(file.area.len(), 4, "with the starter sun and environment");
}

#[test]
fn a_selection_becomes_an_assembly_and_a_copy_of_it() {
    let mut w = Workbench::new(info(), None);
    let post = w.add_entity();
    let lamp = w.add_point_light();
    w.reparent(&[lamp], Some(post));
    let file = w.assembly_file(post).unwrap();
    assert_eq!(file.kind, area::text::FileKind::Assembly { root: post });
    assert_eq!(file.area.len(), 2);
    let asset = AssetId::random();
    w.set_assembly(asset, std::sync::Arc::new(file));
    w.make_copy(post, asset);
    let area = w.document().area();
    assert!(
        area.get(post).unwrap().origin.is_some(),
        "the root is now a copy"
    );
    assert_eq!(area.tree(post).len(), 2);
    let saved = w.area_file().unwrap();
    assert_eq!(
        saved.area.len(),
        3,
        "the starter sun, environment and the copy"
    );
    assert!(
        saved.overrides.is_empty(),
        "a fresh copy matches its assembly exactly"
    );
}

#[test]
fn a_model_unpacks_into_its_parts() {
    let (mut w, model) = with_a_model();
    let asset = crate_info().asset;
    let node = |part, name: &str, parent| crate::models::NodeInfo {
        part,
        name: name.into(),
        parent,
        local: glam::Mat4::from_translation(Vec3::X),
        has_meshes: part != 1,
    };
    let mut info = crate_info();
    info.nodes = vec![
        node(1, "Ship", None),
        node(2, "Mast", Some(0)),
        node(3, "Sail", Some(1)),
    ];
    w.reload_model(info, &[]);
    w.unpack(model);
    let area = w.document().area();
    assert!(
        !area.get(model).unwrap().has("Model"),
        "the whole isn't drawn twice"
    );
    let tree = area.tree(model);
    assert_eq!(tree.len(), 4);
    let sail = tree
        .iter()
        .find(|&&id| area.get(id).unwrap().name == "Sail")
        .copied()
        .unwrap();
    let part = area
        .get(sail)
        .unwrap()
        .get::<area::builtin::Model>()
        .unwrap();
    assert_eq!(part.asset.unwrap().part, Some(3));
    // Each part sits where it did inside the model: three steps of 1 m.
    let at = area.world_matrix(sail).transform_point3(Vec3::ZERO);
    assert!(at.abs_diff_eq(Vec3::new(3.0, 0.0, 0.0), 1e-5), "{at}");
    // The viewport draws the parts, not the whole.
    let view = AreaView::of(area, w.models());
    let mut drawn: Vec<_> = view.models.iter().map(|m| (m.asset, m.node)).collect();
    drawn.sort();
    assert_eq!(drawn, [(asset, Some(1)), (asset, Some(2))]);
    // F on the sail centres on the sail, not on the whole model's box
    // moved there.
    w.select(sail);
    w.frame_selection();
    let focus = w.camera().focus();
    let to_sail = (Vec3::new(3.0, 0.0, 0.0) - w.camera().position).normalize();
    assert!(
        to_sail.dot(w.camera().forward()) > 0.999,
        "looks at the sail: {focus}"
    );
    w.undo();
    assert!(w.document().area().get(model).unwrap().has("Model"));
}

#[test]
fn an_entity_field_shows_a_name_and_picks_from_the_area() {
    // The user test: an entity field showed a long ID and couldn't be set
    // in Properties at all.
    let mut w = Workbench::new(info(), None);
    w.register_component(reflect::ComponentSchema {
        name: "Follow".into(),
        help: "Follows something.".into(),
        fields: vec![reflect::FieldInfo::new(
            "target",
            "Target",
            reflect::FieldKind::Entity,
            reflect::Value::Entity(None),
        )],
    })
    .unwrap();
    let ship = w.add_entity();
    w.apply_property(
        PropertyRequest::Rename {
            entity: ship,
            name: "Flagship".into(),
        },
        1,
    );
    let follower = w.add_entity();
    w.apply_property(
        PropertyRequest::AddComponent {
            entity: follower,
            component: "Follow".into(),
        },
        2,
    );
    w.select(follower);
    let mut h = harness_with(w);
    h.run();
    // None, as a word; the list offers the area's entities by name.
    h.get_by_value("None").click();
    h.run();
    // The Area panel lists it too: the list's is the last.
    h.query_all_by_label("Flagship").last().unwrap().click();
    h.run();
    let target = |w: &Workbench| {
        w.document().area().get(follower).unwrap().components["Follow"]
            .get("target")
            .cloned()
    };
    assert_eq!(target(h.state()), Some(reflect::Value::Entity(Some(ship))));
    // Shown by its name now, not its ID.
    h.run();
    assert!(h.query_all_by_value("Flagship").next().is_some());
    assert!(h.query_by_value(&ship.to_string()).is_none());
    assert!(h.query_by_label(&ship.to_string()).is_none());
}

#[test]
fn a_sound_picked_in_the_library_shows_in_properties_and_plays() {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    let id = w.add_model(crate_info(), &[], 0.1, &[]);
    w.select(id);
    w.set_library(vec![crate::library::LibraryItem {
        id: None,
        kind: crate::library::LibraryKind::Sound,
        name: "step".into(),
        path: "step.wav".into(),
        thumbnail: None,
    }]);
    w.set_sounds(std::collections::HashMap::from([(
        "step.wav".to_owned(),
        crate::SoundState::Read(crate::SoundInfo {
            seconds: 2.0,
            channels: 2,
            sample_rate: 44_100,
            format: "WAV",
            streamed: false,
            waveform: vec![(-0.5, 0.5); 64],
        }),
    )]));
    let mut h = acting(w);
    h.run();
    assert!(
        h.query_by_label("Sample rate").is_none(),
        "the model shows first"
    );
    h.get_by_label("step").click();
    h.run();
    let _ = h.get_by_label("Sample rate");
    let _ = h.get_by_label("44,100 Hz");
    let _ = h.get_by_label("Stereo");
    let _ = h.get_by_label("From memory");
    h.get_by_label("Play the sound").click();
    h.run();
    assert!(
        h.state()
            .1
            .contains(&WorkbenchAction::PlaySound("step.wav".into()))
    );
    // The app says it's playing, half a second in.
    h.state_mut().0.set_preview(Some(crate::Preview {
        path: "step.wav".into(),
        seconds: 0.5,
        length: 2.0,
        detail: None,
    }));
    h.run();
    let _ = h.get_by_label("0.50 s / 2.00 s");
    h.get_by_label("Stop the sound").click();
    h.run();
    assert!(h.state().1.contains(&WorkbenchAction::StopSound));
    // Picking something else in the area (here, nothing) shows that.
    h.state_mut().0.document.selection.clear();
    h.run();
    assert!(h.query_by_label("Sample rate").is_none());
}

#[test]
fn a_sound_that_couldnt_be_read_says_why_in_properties() {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    w.set_library(vec![crate::library::LibraryItem {
        id: None,
        kind: crate::library::LibraryKind::Sound,
        name: "broken".into(),
        path: "broken.ogg".into(),
        thumbnail: None,
    }]);
    w.set_sounds(std::collections::HashMap::from([(
        "broken.ogg".to_owned(),
        crate::SoundState::Failed("it's damaged (its setup is cut off)".into()),
    )]));
    let mut h = acting(w);
    h.run();
    h.get_by_label("broken").click();
    h.run();
    let _ = h.get_by_label_contains("couldn't read it: it's damaged");
}

fn with_a_sound_event() -> (Workbench, AssetId) {
    let mut w = Workbench::new(info(), None);
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    let grass = AssetId(reflect::Id::from_raw(77).unwrap());
    w.set_library(vec![
        crate::library::LibraryItem {
            id: None,
            kind: crate::library::LibraryKind::SoundEvent,
            name: "footstep".into(),
            path: "footstep.solsound".into(),
            thumbnail: None,
        },
        crate::library::LibraryItem {
            id: Some(grass),
            kind: crate::library::LibraryKind::Sound,
            name: "step_grass".into(),
            path: "step_grass.wav".into(),
            thumbnail: None,
        },
    ]);
    w.set_sound_events(std::collections::HashMap::from([(
        "footstep.solsound".to_owned(),
        crate::EventState::Read {
            event: sound::SoundEvent {
                takes: vec![grass],
                ..sound::SoundEvent::default()
            },
            notes: vec!["line 9: \"reverb\" isn't a sound event setting, so it's skipped".into()],
        },
    )]));
    (w, grass)
}

#[test]
fn a_sound_event_shows_its_takes_and_settings_and_plays() {
    let (w, _) = with_a_sound_event();
    let mut h = acting(w);
    h.run();
    h.get_by_label("footstep").click();
    h.run();
    let _ = h.get_by_label("Takes (1 of up to 32) · drop sound files here");
    assert_eq!(
        h.get_all_by_label("step_grass").count(),
        2,
        "in the Library and in the takes"
    );
    let _ = h.get_by_label_contains("\"reverb\" isn't a sound event setting");
    let _ = h.get_by_label("Bus");
    h.get_by_label("Play the sound event").click();
    h.run();
    assert!(
        h.state()
            .1
            .contains(&WorkbenchAction::PlaySound("footstep.solsound".into()))
    );
    h.state_mut().0.set_preview(Some(crate::Preview {
        path: "footstep.solsound".into(),
        seconds: 0.1,
        length: 0.2,
        detail: Some("step_grass, +0.4 st, -1.2 dB".into()),
    }));
    h.run();
    let _ = h.get_by_label("Playing step_grass, +0.4 st, -1.2 dB");
    let _ = h.get_by_label("Stop the sound event");
}

#[test]
fn changing_a_sound_event_saves_it() {
    let (w, grass) = with_a_sound_event();
    let mut h = acting(w);
    h.run();
    h.get_by_label("footstep").click();
    h.run();
    h.get_by_label("2D").click();
    h.run();
    let saved = h.state().1.iter().find_map(|a| match a {
        WorkbenchAction::SaveSoundEvent { path, event } => Some((path.clone(), event.clone())),
        _ => None,
    });
    let (path, event) = saved.expect("a save");
    assert_eq!(path, std::path::PathBuf::from("footstep.solsound"));
    assert_eq!(event.space, sound::event::Space::Flat);
    assert_eq!(event.takes, vec![grass], "the rest kept");
    h.state_mut().1.clear();
    h.get_by_label("Remove step_grass").click();
    h.run();
    assert!(h.state().1.iter().any(|a| matches!(
        a,
        WorkbenchAction::SaveSoundEvent { event, .. } if event.takes.is_empty()
    )));
}

#[test]
fn a_take_is_added_once_and_only_to_a_readable_event() {
    let (mut w, grass) = with_a_sound_event();
    let other = AssetId(reflect::Id::from_raw(78).unwrap());
    let mut actions = Vec::new();
    w.library_request(
        crate::library::LibraryRequest::AddTake("footstep.solsound".into(), other),
        &mut actions,
    );
    match actions.as_slice() {
        [WorkbenchAction::SaveSoundEvent { event, .. }] => {
            assert_eq!(event.takes, vec![grass, other])
        }
        other => panic!("{other:?}"),
    }
    actions.clear();
    w.library_request(
        crate::library::LibraryRequest::AddTake("footstep.solsound".into(), grass),
        &mut actions,
    );
    assert!(actions.is_empty(), "already a take");
    w.library_request(
        crate::library::LibraryRequest::AddTake("missing.solsound".into(), grass),
        &mut actions,
    );
    assert!(actions.is_empty());
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("isn't a sound event Solis could read"))
    );
    w.library_request(
        crate::library::LibraryRequest::SaveSoundEvent(
            "../outside.solsound".into(),
            sound::SoundEvent::default(),
        ),
        &mut actions,
    );
    assert!(actions.is_empty(), "never outside the assets");
}

#[test]
fn new_sound_event_is_in_the_import_menu() {
    let (w, _) = with_a_sound_event();
    let mut h = acting(w);
    h.run();
    h.get_by_label("Import").click();
    h.run();
    h.get_by_label("New sound event").click();
    h.run();
    assert!(
        h.state()
            .1
            .contains(&WorkbenchAction::NewSoundEvent("".into()))
    );
}
