//! Preferences and Project settings in a project of each kind (decision
//! 0118, R6): the settings of the kind the project hides aren't offered.

use super::settings::{Setting, found};
use super::*;
use crate::test_support::*;
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;

/// The settings of the kind the project hides aren't offered (decision
/// 0118, R6): not on their sub-tab, not in the search; Both shows all.
#[test]
fn a_project_s_kind_hides_the_other_kind_s_settings() {
    use crate::kinds::{self, PartKind};
    use crate::launcher::ProjectKind;
    for (kind, hidden) in [
        (ProjectKind::TwoD, PartKind::ThreeD),
        (ProjectKind::ThreeD, PartKind::TwoD),
    ] {
        for project in [false, true] {
            for setting in found(project, "", kind) {
                assert_ne!(kinds::setting(setting), hidden, "{setting:?} in {kind:?}");
            }
        }
    }
    let every =
        found(false, "", ProjectKind::Both).len() + found(true, "", ProjectKind::Both).len();
    assert_eq!(every, Setting::ALL.len(), "Both shows every setting");
    // Drawn: a 2D project's Preferences have no Look, and its Project
    // settings their 2D sub-tab; a 3D project's the other way round.
    for (kind, look, two_d) in [
        (ProjectKind::TwoD, false, true),
        (ProjectKind::ThreeD, true, false),
    ] {
        let mut w = Workbench::new(info());
        w.set_project(Some(crate::library::ProjectInfo {
            name: "Farm".into(),
            root: "/projects/farm".into(),
            assets: "/projects/farm".into(),
        }));
        w.apply_project_kind(kind);
        w.show_settings(Tab::Viewport);
        let mut h = harness(w);
        h.run();
        h.run();
        assert_eq!(h.query_by_label("Look").is_some(), look, "{kind:?}");
        h.state_mut().0.show_settings(Tab::Project);
        h.run();
        h.run();
        let tab = h.query_by_role_and_label(Role::Button, "2D").is_some();
        assert_eq!(tab, two_d, "{kind:?}");
    }
}
