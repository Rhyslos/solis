//! The splash screen and the update question.

use super::*;
use crate::test_support::info;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn splash(doing: &'static str, progress: Option<f32>) -> Harness<'static> {
    Harness::builder()
        .with_size([800.0, 600.0])
        .build_ui(move |ui| {
            show(
                ui,
                &SplashView {
                    info: &info(),
                    accent: Accent::default(),
                    logo: None,
                    doing,
                    progress,
                    time: 0.3,
                },
            );
        })
}

#[test]
fn the_splash_says_what_s_happening() {
    let mut h = splash("Checking for updates…", None);
    h.run();
    let name = format!("{} {}: Checking for updates…", info().name, info().version);
    assert!(h.query_by_label(&name).is_some());
    // A known amount too.
    let mut h = splash("Downloading build 12…", Some(0.5));
    h.run();
    assert!(h.query_by_label_contains("Downloading build 12…").is_some());
}

type Asked = (bool, Option<UpdateAnswer>);

fn question(offer: UpdateOffer) -> Harness<'static, Asked> {
    Harness::builder().with_size([800.0, 600.0]).build_ui_state(
        move |ui, state: &mut Asked| {
            let ctx = ui.ctx().clone();
            if let Some(answer) = update_question(&ctx, "Test Engine", &offer, &mut state.0, 0.0) {
                state.1 = Some(answer);
            }
        },
        (false, None),
    )
}

fn installed(latest: u64) -> UpdateOffer {
    UpdateOffer {
        latest,
        current: Some(10),
        installed: true,
    }
}

#[test]
fn the_question_names_both_builds_and_updates_when_asked() {
    let mut h = question(installed(12));
    h.run();
    assert!(
        h.query_by_label_contains("Build 12 of Test Engine is available (you have build 10)")
            .is_some()
    );
    h.get_by_label("Update now").click();
    h.run();
    assert_eq!(h.state().1, Some(UpdateAnswer::UpdateNow));
}

#[test]
fn opening_without_updating_can_skip_this_version() {
    let mut h = question(installed(12));
    h.run();
    h.get_by_label("Open without updating").click();
    h.run();
    assert_eq!(h.state().1, Some(UpdateAnswer::NotNow { skip: false }));
    let mut h = question(installed(12));
    h.run();
    h.get_by_label("Skip this version").click();
    h.run();
    assert!(h.state().0, "ticked");
    h.get_by_label("Open without updating").click();
    h.run();
    assert_eq!(h.state().1, Some(UpdateAnswer::NotNow { skip: true }));
}

#[test]
fn a_copy_that_wasn_t_installed_points_to_the_download_page() {
    let mut h = question(UpdateOffer {
        installed: false,
        ..installed(12)
    });
    h.run();
    assert!(h.query_by_label("Update now").is_none());
    assert!(
        h.query_by_label_contains("wasn't installed with the installer")
            .is_some()
    );
    h.get_by_label("Open the download page").click();
    h.run();
    assert_eq!(h.state().1, Some(UpdateAnswer::OpenDownloadPage));
}
