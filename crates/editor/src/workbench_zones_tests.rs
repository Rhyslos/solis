//! Trigger zones in the editor: made from Home's ribbon where you're
//! looking, sized by a handle as one undo step, and a freeform outline
//! drawn, closed, given up, edited and undone.

use crate::test_support::*;
use crate::workbench::Workbench;
use crate::zones::ZoneEvent;
use crate::zones::points::{PointsEdit, Stage};
use glam::{Mat4, Vec2, Vec3};
use level::builtin::{TriggerZone, ZoneForm};
use reflect::EntityId;

fn zone_of(w: &Workbench, id: EntityId) -> TriggerZone {
    w.open
        .document
        .level()
        .get(id)
        .and_then(|e| e.get::<TriggerZone>())
        .unwrap()
}

/// A workbench with a new trigger zone, made as Home's ribbon makes one.
fn with_a_zone() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info());
    let id = w.add_trigger_zone();
    (w, id)
}

#[test]
fn a_new_zone_is_made_where_you_re_looking_and_picked() {
    let (w, id) = with_a_zone();
    let focus = w.viewport.camera.focus();
    let data = w.open.document.level().get(id).unwrap();
    assert_eq!(data.name, "Trigger zone");
    let place = data.get::<level::builtin::Transform>().unwrap().position;
    assert_eq!(place, focus);
    assert_eq!(w.open.document.selection.primary(), Some(id));
    assert_eq!(zone_of(&w, id), TriggerZone::default());
}

#[test]
fn a_handle_s_drag_is_one_undo_step() {
    let (mut w, id) = with_a_zone();
    let steps = w.open.document.history().state();
    w.gesture += 1;
    for width in [3.0, 4.0, 5.0] {
        w.viewport.zones.events.push(ZoneEvent::Sized {
            entity: id,
            zone: TriggerZone {
                size: Vec3::new(width, 2.0, 2.0),
                ..TriggerZone::default()
            },
            place: Vec3::new(width * 0.5, 0.0, 0.0),
        });
        w.zone_events();
    }
    assert_eq!(zone_of(&w, id).size.x, 5.0);
    w.undo();
    assert_eq!(zone_of(&w, id).size.x, 2.0, "the whole drag undone at once");
    assert_eq!(w.open.document.history().state(), steps);
}

/// Points drawn as a 2D view's clicks would place them.
fn draw(w: &mut Workbench, id: EntityId, points: &[(f32, f32)]) {
    w.start_zone_drawing(id);
    let edit = w.viewport.zones.points.as_mut().unwrap();
    for &(x, y) in points {
        edit.place(Vec2::new(x, y), &mut w.viewport.zones.events);
    }
}

const ELL: [(f32, f32); 6] = [
    (0.0, 0.0),
    (4.0, 0.0),
    (4.0, 2.0),
    (2.0, 2.0),
    (2.0, 4.0),
    (0.0, 4.0),
];

#[test]
fn drawing_and_closing_makes_a_freeform_zone_in_one_step() {
    let (mut w, id) = with_a_zone();
    let before = w.open.document.history().state();
    draw(&mut w, id, &ELL);
    assert_eq!(
        zone_of(&w, id).form,
        ZoneForm::Box,
        "nothing changes while drawing"
    );
    let tool = &mut w.viewport.zones;
    assert!(tool.points.as_mut().unwrap().close(&mut tool.events));
    tool.points = None;
    w.zone_events();
    let zone = zone_of(&w, id);
    assert_eq!(zone.form, ZoneForm::Freeform);
    assert_eq!(zone.points.len(), 6);
    w.undo();
    assert_eq!(zone_of(&w, id).form, ZoneForm::Box);
    assert_eq!(w.open.document.history().state(), before);
}

#[test]
fn esc_while_drawing_keeps_the_shape_it_had() {
    let (mut w, id) = with_a_zone();
    draw(&mut w, id, &ELL[..4]);
    // As zone_keys does for Esc.
    w.viewport.zones.points = None;
    w.viewport.zones.events.push(ZoneEvent::GaveUp);
    w.zone_events();
    assert_eq!(zone_of(&w, id), TriggerZone::default());
    assert!(w.zone_status().unwrap().contains("left as it was"));
}

#[test]
fn a_point_that_would_cross_the_outline_is_refused() {
    let (mut w, id) = with_a_zone();
    draw(&mut w, id, &[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0)]);
    // From (4, 4) down to (2, -2) would cross the first edge.
    let tool = &mut w.viewport.zones;
    tool.points
        .as_mut()
        .unwrap()
        .place(Vec2::new(2.0, -2.0), &mut tool.events);
    assert_eq!(tool.points.as_ref().unwrap().points.len(), 3);
    w.zone_events();
    assert!(w.zone_status().unwrap().contains("cross itself"));
}

/// The zone made freeform with the L, and its points being edited.
fn editing_an_ell() -> (Workbench, EntityId) {
    let (mut w, id) = with_a_zone();
    draw(&mut w, id, &ELL);
    let tool = &mut w.viewport.zones;
    tool.points.as_mut().unwrap().close(&mut tool.events);
    tool.points = None;
    w.zone_events();
    w.prepare_zones();
    w.toggle_zone_points();
    assert_eq!(
        w.viewport.zones.points.as_ref().map(|e| e.stage),
        Some(Stage::Editing)
    );
    (w, id)
}

#[test]
fn editing_adds_and_deletes_points_each_as_one_step() {
    let (mut w, id) = editing_an_ell();
    let tool = &mut w.viewport.zones;
    let edit = tool.points.as_mut().unwrap();
    edit.add_on_edge(0, 0.5, &mut tool.events);
    w.zone_events();
    assert_eq!(zone_of(&w, id).points.len(), 7);
    assert_eq!(zone_of(&w, id).points[1], Vec2::new(2.0, 0.0));
    let tool = &mut w.viewport.zones;
    tool.points
        .as_mut()
        .unwrap()
        .delete_picked(&mut tool.events);
    w.zone_events();
    assert_eq!(zone_of(&w, id).points.len(), 6);
    w.undo();
    assert_eq!(zone_of(&w, id).points.len(), 7, "each its own step");
}

#[test]
fn never_fewer_than_three_points() {
    let (mut w, id) = with_a_zone();
    w.viewport.zones.points = Some(PointsEdit::editing(
        id,
        Mat4::IDENTITY,
        &TriggerZone {
            form: ZoneForm::Freeform,
            points: vec![Vec2::ZERO, Vec2::X, Vec2::Y],
            ..TriggerZone::default()
        },
    ));
    let tool = &mut w.viewport.zones;
    let edit = tool.points.as_mut().unwrap();
    edit.picked = Some(0);
    edit.delete_picked(&mut tool.events);
    assert_eq!(edit.points.len(), 3);
    w.zone_events();
    assert!(w.zone_status().unwrap().contains("at least 3"));
}

#[test]
fn tab_needs_one_freeform_zone_picked() {
    let (mut w, _) = with_a_zone();
    w.prepare_zones();
    assert!(
        w.why_not_edit_zone_points().is_some(),
        "a box has no points"
    );
    w.toggle_zone_points();
    assert!(w.viewport.zones.points.is_none());
}

#[test]
fn the_level_panel_shows_a_zone_with_its_dashed_icon() {
    let (w, id) = with_a_zone();
    let entity = w.open.document.level().get(id).unwrap();
    let (icon, _, note) = crate::level_panel::looks(entity, &Default::default());
    assert_eq!(icon, crate::icons::Icon::Volume);
    assert_eq!(note, "Trigger zone");
}

#[test]
fn a_click_on_a_zone_s_outline_picks_it_and_one_inside_does_not() {
    use crate::camera::{Camera, ViewKind};
    let zone = crate::zones::ViewZone {
        entity: EntityId::random(),
        name: "Door".into(),
        zone: TriggerZone::default(),
        transform: Mat4::IDENTITY,
    };
    let camera = Camera::default().as_kind(ViewKind::TwoD);
    let size = Vec2::new(800.0, 600.0);
    let edge = camera.project(Vec3::new(1.0, 0.0, 0.0), size).unwrap();
    let middle = camera.project(Vec3::ZERO, size).unwrap();
    let locked = Default::default();
    let at = |p| {
        crate::zones::zone_at(
            std::slice::from_ref(&zone),
            &locked,
            (&camera, size),
            p,
            10.0,
        )
    };
    assert_eq!(at(edge), Some(zone.entity));
    assert_eq!(at(middle), None, "inside is left for what's in the zone");
}

#[test]
fn a_zone_is_made_into_a_prefab_like_any_entity() {
    use level::text::FileKind;
    let (mut w, id) = with_a_zone();
    w.ask_make_prefab();
    let mut form = w.open.make_prefab.clone().expect("the dialog is open");
    form.name = "Spike pit".into();
    let plan = w.make_prefab_plan(&form).unwrap();
    assert_eq!(plan.file.kind, FileKind::Prefab { root: id });
    let kept = plan.file.level.get(id).unwrap();
    assert_eq!(kept.get::<TriggerZone>(), Some(TriggerZone::default()));
    let asset = reflect::AssetId::random();
    w.set_prefab(asset, std::sync::Arc::new(plan.file.clone()));
    w.swap_for_copy(asset, plan.swap.as_ref().unwrap());
    let copy = w.open.document.level().get(id).unwrap();
    assert!(copy.origin.is_some(), "the zone is a linked copy now");
    assert_eq!(zone_of(&w, id), TriggerZone::default());
}

#[test]
fn new_code_for_this_zone_makes_a_file_adds_its_component_and_opens_it() {
    use crate::workbench::WorkbenchAction;
    let (mut w, id) = with_a_zone();
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Doors".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.command(
        "Rename",
        vec![level::Edit::Rename {
            entity: id,
            name: "Door zone".into(),
        }],
        None,
        None,
    );
    let steps = w.open.document.history().undo_len();
    w.new_code_for_zone(id);
    let made: Vec<(String, String)> = w
        .open
        .outbox
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::NewTextFile { path, text } => {
                Some((path.to_string_lossy().into_owned(), text.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(made[0].0, "doorZone.lst");
    assert!(made[0].1.contains("component DoorZone {"), "{}", made[0].1);
    assert!(made[0].1.contains("on Entered(entity who) {"));
    assert!(made[0].1.contains("on Exited(entity who) {"));
    assert_eq!(made[1].0, "main.lst", "a project without one gets one");
    assert!(made[1].1.contains("fetch \"doorZone\":DoorZone;"));
    let zone = w.open.document.level().get(id).unwrap();
    assert!(zone.has("DoorZone"));
    assert_eq!(
        w.open.document.history().undo_len(),
        steps + 1,
        "one undo step"
    );
    // The file's code checks as it is.
    let mut files = script::describe::descriptions("Test 1.0", "solis");
    files.extend(made);
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let problems = script::check(&code);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_zone_s_component_is_named_from_its_name() {
    use crate::workbench_zones::component_name;
    assert_eq!(component_name("Door zone"), "DoorZone");
    assert_eq!(component_name("shop counter 2"), "ShopCounter2");
    assert_eq!(component_name("3 pits"), "Zone3Pits");
    assert_eq!(component_name("!!"), "Zone");
}

#[test]
fn the_card_asks_for_freeform_drawing_and_sets_its_fields() {
    use crate::properties::PropertyRequest;
    let (mut w, id) = with_a_zone();
    w.apply_property(
        PropertyRequest::SetField {
            entity: id,
            component: "TriggerZone".into(),
            field: "fires".into(),
            value: reflect::Value::Text("Once".into()),
        },
        1,
    );
    assert_eq!(zone_of(&w, id).fires, level::builtin::Fires::Once);
    w.apply_property(PropertyRequest::ZoneDraw(id), 2);
    assert_eq!(
        w.viewport
            .zones
            .points
            .as_ref()
            .map(|e| (e.entity, e.stage)),
        Some((id, Stage::Drawing))
    );
}

#[test]
fn inside_now_names_who_is_inside_while_playing() {
    let (mut w, id) = with_a_zone();
    assert_eq!(w.zone_facts().inside, None, "shown while playing");
    let at = w
        .open
        .document
        .level()
        .get(id)
        .and_then(|e| e.get::<level::builtin::Transform>())
        .unwrap();
    let mut walker = level::EntityData::new("Ann").with(&at);
    walker = walker.with(&level::builtin::Collider2D {
        character: true,
        ..level::builtin::Collider2D::default()
    });
    let ann = w.spawn_selected(walker);
    w.select(id);
    w.play();
    w.run_game(1.0 / 30.0);
    let _ = ann;
    assert_eq!(w.zone_facts().inside, Some(vec!["Ann".to_owned()]));
}

#[test]
fn a_zone_named_like_the_engine_s_own_component_gets_a_number() {
    let (mut w, id) = with_a_zone();
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Doors".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    // "Trigger zone" would be TriggerZone, which is Solis's own.
    w.new_code_for_zone(id);
    let zone = w.open.document.level().get(id).unwrap();
    assert!(zone.has("TriggerZone2"));
}

/// The keys pressed over the view, as a person presses them: while points
/// are drawn, `Enter` closes and `Esc` gives up (and the zone stays
/// picked); while they're edited, `Delete` takes the picked point away
/// (never the zone), and `Tab` and `Esc` finish.
#[test]
fn the_points_keys_are_the_zone_s_while_its_points_are_drawn_or_edited() {
    use crate::panel::Panel;
    use egui::Key;
    let (w, id) = with_a_zone();
    let mut h = rest_over(harness(w), Panel::Viewport);
    let start = |h: &mut egui_kittest::Harness<'static, State>| {
        h.state_mut().0.start_zone_drawing(id);
        let tool = &mut h.state_mut().0.viewport.zones;
        let edit = tool.points.as_mut().unwrap();
        for &(x, y) in &ELL {
            edit.place(Vec2::new(x, y), &mut tool.events);
        }
        h.run();
    };
    // Esc gives up: the zone is still a box, and still picked.
    start(&mut h);
    h.key_press(Key::Escape);
    h.run();
    assert!(h.state().0.viewport.zones.points.is_none());
    assert_eq!(zone_of(&h.state().0, id).form, ZoneForm::Box);
    assert_eq!(h.state().0.open.document.selection.primary(), Some(id));
    // Enter closes it.
    start(&mut h);
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().0.viewport.zones.points.is_none());
    assert_eq!(zone_of(&h.state().0, id).points.len(), 6);
    // Tab edits the points; Delete takes the picked one, not the zone.
    h.key_press(Key::Tab);
    h.run();
    let editing = h.state().0.viewport.zones.points.as_ref().map(|e| e.stage);
    assert_eq!(editing, Some(Stage::Editing));
    h.state_mut()
        .0
        .viewport
        .zones
        .points
        .as_mut()
        .unwrap()
        .picked = Some(3);
    h.key_press(Key::Delete);
    h.run();
    assert!(
        h.state().0.open.document.level().contains(id),
        "the zone stays"
    );
    assert_eq!(zone_of(&h.state().0, id).points.len(), 5);
    h.key_press(Key::Tab);
    h.run();
    assert!(h.state().0.viewport.zones.points.is_none(), "Tab finishes");
    // Not editing, Delete is Delete again.
    h.key_press(Key::Delete);
    h.run();
    assert!(!h.state().0.open.document.level().contains(id));
}
