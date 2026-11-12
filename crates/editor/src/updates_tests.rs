//! Checking for updates, and the Restart to update button.

use super::*;
use crate::test_support::{do_command, info};
use crate::viewport::NoRenderer;
use egui_kittest::kittest::Queryable;

type State = (Workbench, Vec<WorkbenchAction>);

/// The whole editor in a test window, keeping every action it asks for.
fn window(workbench: Workbench) -> egui_kittest::Harness<'static, State> {
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            |ui, state: &mut State| {
                if !crate::test_support::fonts_ready(ui.ctx()) {
                    return;
                }
                let mut renderer = NoRenderer {
                    reason: "testing".into(),
                };
                let actions = state.0.show(ui, &mut renderer);
                state.1.extend(actions);
            },
            (workbench, Vec::new()),
        )
}

/// Does the command called `name`, keeping what it asks the app for.
fn command(h: &mut egui_kittest::Harness<'static, State>, name: &str) {
    h.run();
    let ctx = h.ctx.clone();
    let (workbench, asked) = h.state_mut();
    asked.extend(do_command(workbench, &ctx, name));
    h.run();
}

#[test]
fn check_for_updates_asks_the_app_and_the_updates_page_says_how_it_went() {
    let mut w = Workbench::new(info());
    w.set_update(UpdateNotice::UpToDate(41));
    let mut h = window(w);
    h.run();
    command(&mut h, "Check for updates");
    assert!(h.state().1.contains(&WorkbenchAction::CheckForUpdates));
    // File, Updates: the page that says where updating stands.
    command(&mut h, "Updates");
    assert_eq!(h.state().0.ribbon_tab(), crate::shell::RibbonTab::File);
    assert_eq!(
        h.state().0.file_pages.page,
        crate::file_pages::Page::Updates
    );
    assert!(h.query_by_label_contains("Check now").is_some());
}

#[test]
fn checking_can_t_be_asked_twice_at_once() {
    let mut w = Workbench::new(info());
    w.set_update(UpdateNotice::Downloading(42));
    let ctx = egui::Context::default();
    let check = crate::shell::ShellCommand::CheckForUpdates;
    assert_eq!(check.why_not(&w).as_deref(), Some("Already checking"));
    let mut asked = Vec::new();
    check.run(&mut w, &ctx, &mut asked);
    assert!(asked.is_empty());
    assert!(
        !w.search_items()
            .iter()
            .any(|i| i.name == "Check for updates"),
        "not offered while it can't be done"
    );
}

#[test]
fn checking_when_starting_can_be_turned_off() {
    let mut h = window(Workbench::new(info()));
    h.run();
    assert!(h.state().0.auto_update(), "on unless turned off");
    command(&mut h, "Don't check for updates when starting");
    assert!(!h.state().0.auto_update());
    assert!(h.state().1.contains(&WorkbenchAction::SetAutoUpdate(false)));
    command(&mut h, "Check for updates when starting");
    assert!(h.state().0.auto_update());
}

#[test]
fn restart_to_update_shows_only_when_a_build_is_ready() {
    let mut h = window(Workbench::new(info()));
    h.run();
    assert!(h.query_by_label("Restart to update").is_none());
    h.state_mut().0.set_update(UpdateNotice::Ready(43));
    h.run();
    h.get_by_label("Restart to update").click();
    h.run();
    assert_eq!(h.state().1.last(), Some(&WorkbenchAction::Quit));
    assert!(h.state().0.restarting());
}

#[test]
fn a_restart_asks_about_unsaved_work_and_can_be_called_off() {
    let mut w = Workbench::new(info());
    w.add_point_light();
    w.set_update(UpdateNotice::Ready(43));
    let mut h = window(w);
    h.run();
    h.get_by_label("Restart to update").click();
    h.run();
    assert!(h.state().1.is_empty(), "asks first");
    h.get_by_label("Cancel").click();
    h.run();
    assert!(!h.state().0.restarting(), "called off");
    // Asked again, Don't save goes ahead.
    h.get_by_label("Restart to update").click();
    h.run();
    h.get_by_label("Don't save").click();
    h.run();
    assert_eq!(h.state().1.last(), Some(&WorkbenchAction::Quit));
    assert!(h.state().0.restarting());
}

#[test]
fn a_restart_without_a_ready_build_is_only_a_close() {
    let mut w = Workbench::new(info());
    w.restarting = true;
    assert!(!w.restarting());
    w.set_update(UpdateNotice::Ready(1));
    assert!(w.restarting());
}

#[test]
fn every_notice_reads_as_a_line() {
    for notice in [
        UpdateNotice::NotChecked,
        UpdateNotice::Checking,
        UpdateNotice::UpToDate(1),
        UpdateNotice::Downloading(2),
        UpdateNotice::Ready(3),
        UpdateNotice::NotInstalled(4),
        UpdateNotice::FromSource,
        UpdateNotice::Failed("offline".into()),
    ] {
        assert!(!notice.line().is_empty());
    }
    assert!(
        UpdateNotice::Failed("offline".into())
            .line()
            .contains("offline")
    );
}

#[test]
fn help_s_check_asks_whether_to_update_and_passes_the_answer_on() {
    use crate::splash::UpdateOffer;
    let mut w = Workbench::new(info());
    let offer = UpdateOffer {
        latest: 12,
        current: Some(10),
        installed: true,
    };
    w.offer_update(offer.clone());
    let mut h = window(w);
    h.run();
    h.get_by_label("Update now").click();
    h.run();
    assert!(h.state().1.contains(&WorkbenchAction::DownloadUpdate));
    // Asked again, and skipped: the build to skip goes with the answer.
    h.state_mut().0.offer_update(offer);
    h.run();
    h.get_by_label("Skip this version").click();
    h.run();
    h.get_by_label("Open without updating").click();
    h.run();
    assert_eq!(
        h.state().1.last(),
        Some(&WorkbenchAction::SkipUpdate(Some(12)))
    );
    assert!(
        h.query_by_label("Update available").is_none(),
        "answered: gone"
    );
}
