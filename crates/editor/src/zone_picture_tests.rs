//! Trigger zones in approved pictures (stage 1, triggers; the approved
//! mock-up `docs/design/triggers/`): a 2D view with a zone sized wide and
//! picked (solid, with handles and its label) beside a circle that isn't
//! (dashed); a 3D view with a box zone picked; and the zone's cards in
//! Properties. Drawn for real at 1440 × 900 and cropped to what they show.

use crate::interface_look::InterfaceLook;
use crate::launcher::ProjectKind;
use crate::picture_support::{Picture, drawn, editor_window, matches_approved};
use crate::workbench::Workbench;
use glam::Vec3;
use level::builtin::{Transform, TriggerZone, ZoneForm};
use reflect::EntityId;

const SIZE: [f32; 2] = [1440.0, 900.0];

/// Gives `id` a zone and a place.
fn set(w: &mut Workbench, id: EntityId, name: &str, zone: TriggerZone, at: Vec3) {
    let level = w.open.document.level();
    let data = level.get(id).unwrap();
    let mut edits = crate::commands::component_edits(
        id,
        data.components
            .get(<TriggerZone as reflect::Component>::NAME),
        &zone,
    );
    edits.extend(crate::commands::component_edits(
        id,
        data.components.get(<Transform as reflect::Component>::NAME),
        &Transform {
            position: at,
            ..Transform::default()
        },
    ));
    edits.push(level::Edit::Rename {
        entity: id,
        name: name.into(),
    });
    w.command("Set", edits, None, None);
}

/// A project of `kind` with a door zone 4 × 2 m picked and a pit, a
/// circle, beside it.
fn doors(kind: ProjectKind) -> Workbench {
    let mut w = Workbench::new(crate::test_support::info());
    w.set_project_kind(kind);
    let pit = w.add_trigger_zone();
    let pit_zone = TriggerZone {
        form: ZoneForm::Round,
        radius: 1.5,
        ..TriggerZone::default()
    };
    set(&mut w, pit, "Pit", pit_zone, Vec3::new(-3.0, -3.0, 0.0));
    let door = w.add_trigger_zone();
    let door_zone = TriggerZone {
        size: Vec3::new(4.0, 2.0, 2.0),
        ..TriggerZone::default()
    };
    set(
        &mut w,
        door,
        "Door zone",
        door_zone,
        Vec3::new(3.0, 1.0, 1.0),
    );
    w.select(door);
    w
}

/// Draws `w` and crops a part of the window.
fn part_of(w: Workbench, min: [u32; 2], size: [u32; 2]) -> Option<Picture> {
    let mut h = editor_window(w, InterfaceLook::default(), SIZE)?;
    for _ in 0..4 {
        h.run();
    }
    Some(drawn(&mut h).cropped(min, size))
}

/// The view, in the Standard layout.
const VIEW: ([u32; 2], [u32; 2]) = ([252, 130], [866, 506]);

#[test]
fn a_zone_in_a_2d_view() {
    let mut w = doors(ProjectKind::TwoD);
    w.viewport.camera.zoom = 2.0;
    let Some(picture) = part_of(w, VIEW.0, VIEW.1) else {
        return;
    };
    matches_approved("zone-2d", &picture);
}

#[test]
fn a_zone_in_a_3d_view() {
    let mut w = doors(ProjectKind::ThreeD);
    w.viewport.camera.look_at(Vec3::new(1.0, 0.0, 0.0));
    let Some(picture) = part_of(w, VIEW.0, VIEW.1) else {
        return;
    };
    matches_approved("zone-3d", &picture);
}

#[test]
fn the_zone_s_cards_in_properties() {
    let w = doors(ProjectKind::TwoD);
    let Some(picture) = part_of(w, [1120, 100], [320, 700]) else {
        return;
    };
    matches_approved("zone-card", &picture);
}
