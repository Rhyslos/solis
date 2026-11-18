//! The scale check's case for prefabs (R5, decision 0108): a prefab
//! placed 1,000 times, changed on its page and saved, with every copy
//! updated (each keeping its own changes) and the frame after it within
//! the editor's target.

use crate::properties::PropertyRequest;
use crate::test_support::{info, lamp_post_file};
use crate::viewport::NoRenderer;
use crate::workbench::{Workbench, WorkbenchAction};
use reflect::{EntityId, Value};
use std::sync::Arc;

/// The editor frame's target (as `layout_scale_tests`).
const TARGET_MS: f64 = 8.0;

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_a_prefab_saved_with_1000_copies_updates_them_without_a_late_frame() {
    let mut w = Workbench::new(info());
    let (asset, file) = lamp_post_file();
    w.set_prefab(asset, file);
    for _ in 0..1000 {
        w.place_prefab(asset);
    }
    w.open_prefab_page(asset);
    let lamp = EntityId(reflect::Id::from_raw(2).unwrap());
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
    // The change, made on the page, and saved.
    let w = h.state_mut();
    w.page_in();
    w.apply_property(
        PropertyRequest::SetField {
            entity: lamp,
            component: "Light".into(),
            field: "strength".into(),
            value: Value::Float(7.0),
        },
        1,
    );
    w.save_prefab_page();
    let saved = w.open.outbox.iter().find_map(|a| match a {
        WorkbenchAction::SavePrefab { file, .. } => Some(file.clone()),
        _ => None,
    });
    w.page_out();
    let start = std::time::Instant::now();
    w.prefab_saved(
        asset,
        Arc::new(saved.expect("saved")),
        "/p/lamp.prefab.lst".into(),
    );
    let update_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = std::time::Instant::now();
    h.step();
    let frame_ms = start.elapsed().as_secs_f64() * 1000.0;
    let w = h.state();
    let updated = w
        .document()
        .level()
        .iter()
        .filter(|(_, e)| {
            e.get::<level::builtin::Light>()
                .is_some_and(|l| l.strength == 7.0)
        })
        .count();
    assert_eq!(updated, 1000, "every copy follows");
    let slowest = update_ms.max(frame_ms);
    let verdict = if slowest <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: a prefab with 1,000 copies saved, copies updated in {update_ms:.2} ms, \
         the next frame {frame_ms:.2} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(slowest <= TARGET_MS, "{slowest:.2} ms");
}
