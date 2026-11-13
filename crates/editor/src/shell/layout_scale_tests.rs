//! The scale check's case for the frame's layouts (decision 0109): an
//! editor frame on Home in each panel layout with 10,000 entities in the
//! Area list and 10,000 files in the Assets list, within the editor
//! frame's target.

use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::test_support::info;
use crate::viewport::NoRenderer;
use crate::workbench::Workbench;
use area::builtin::Transform;
use glam::Vec3;
use reflect::EntityId;

/// The target the 10,000-entity frame already has (BUG-28's case).
const TARGET_MS: f64 = 8.0;

/// The editor with `count` entities (every tenth heading a group of the
/// next nine, one selected) and `count` files in the Library's top
/// folder, a project open so the Assets list shows them.
pub(crate) fn full(count: usize, layout: crate::shell::PanelLayout) -> Workbench {
    let look = crate::interface_look::InterfaceLook {
        layout,
        ..Default::default()
    };
    let mut w = Workbench::new(info()).with_interface_look(look);
    w.set_project(Some(ProjectInfo {
        name: "Big".into(),
        root: "big".into(),
        assets: "big/assets".into(),
    }));
    let kinds = [LibraryKind::Model, LibraryKind::Texture, LibraryKind::Sound];
    w.set_library(
        (0..count)
            .map(|i| LibraryItem {
                id: None,
                kind: kinds[i % kinds.len()],
                name: format!("file {i}"),
                path: format!("file {i}.png"),
                thumbnail: None,
            })
            .collect(),
    );
    let mut edits = Vec::new();
    let mut parent = None;
    for i in 0..count {
        let id = EntityId::random();
        let mut data = area::EntityData::new(&format!("Thing {i}")).with(&Transform {
            position: Vec3::new((i % 50) as f32, (i / 50) as f32, 0.0),
            ..Transform::default()
        });
        if i % 10 == 0 {
            parent = Some(id);
        } else {
            data.parent = parent;
        }
        edits.push(area::Edit::Spawn {
            entities: vec![(id, data)],
        });
    }
    w.open.document.edit("Fill", edits, None, None).unwrap();
    let some = w.document().area().depth_first()[count / 2];
    w.select(some);
    w
}

/// How long one frame of `w` takes at 1440 × 900, in milliseconds.
pub(crate) fn frame_ms(w: Workbench) -> f64 {
    let mut renderer = NoRenderer {
        reason: "testing".into(),
    };
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            move |ui, workbench: &mut Workbench| {
                if crate::test_support::fonts_ready(ui.ctx()) {
                    let _ = workbench.show(ui, &mut renderer);
                }
            },
            w,
        );
    h.run();
    h.run();
    let frames = 20;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        h.step();
    }
    start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames)
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_editor_frame_with_10000_entities_and_10000_files() {
    let mut slowest = 0.0_f64;
    for layout in crate::shell::PanelLayout::ALL {
        let ms = frame_ms(full(10_000, layout));
        let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
        println!(
            "Scale check: editor frame on Home ({}), 10,000 entities and 10,000 files listed \
             {ms:.2} ms (target {TARGET_MS} ms) {verdict}",
            layout.name()
        );
        slowest = slowest.max(ms);
    }
    assert!(
        slowest <= TARGET_MS,
        "the editor takes {slowest:.2} ms per frame"
    );
}
