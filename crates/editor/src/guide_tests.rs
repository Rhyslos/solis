use super::*;
use crate::shell::RibbonTab;
use crate::shell::documentation::DOCUMENTATION;
use crate::test_support::{State, harness, info, ribbon_tab};
use crate::workbench::WorkbenchAction;
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;

fn found() -> Vec<FoundBrowser> {
    vec![
        FoundBrowser {
            name: "Example Browser".into(),
            program: PathBuf::from("C:\\Example\\browser.exe"),
        },
        FoundBrowser {
            name: "Another".into(),
            program: PathBuf::from("C:\\Another\\another.exe"),
        },
    ]
}

fn words(settings: &GuideSettings) -> Vec<String> {
    settings.choices().0.into_iter().map(|(_, w)| w).collect()
}

#[test]
fn the_list_is_the_default_the_browsers_found_and_a_program_to_pick() {
    let mut settings = GuideSettings {
        found: found(),
        ..Default::default()
    };
    assert_eq!(
        words(&settings),
        ["Default browser", "Example Browser", "Another", PICK_WORDS]
    );
    assert_eq!(settings.choices().1, GuideChoice::Default);
    assert!(!settings.choose(GuideChoice::Listed(1)));
    assert_eq!(
        settings.browser,
        GuideBrowser::Installed {
            name: "Another".into(),
            program: PathBuf::from("C:\\Another\\another.exe"),
        }
    );
    assert_eq!(settings.choices().1, GuideChoice::Listed(1));
    // Picking a program is the app's to ask; the choice stays meanwhile.
    assert!(settings.choose(GuideChoice::Pick));
    assert!(matches!(settings.browser, GuideBrowser::Installed { .. }));
}

#[test]
fn a_program_picked_before_is_listed_by_its_name() {
    let settings = GuideSettings {
        browser: GuideBrowser::Chosen(PathBuf::from("C:\\Tools\\reader.exe")),
        found: Vec::new(),
    };
    assert_eq!(words(&settings), ["Default browser", "reader", PICK_WORDS]);
    assert_eq!(settings.choices().1, GuideChoice::Chosen);
}

#[test]
fn a_saved_browser_not_found_this_time_is_still_listed() {
    let settings = GuideSettings {
        browser: GuideBrowser::Installed {
            name: "Old Browser".into(),
            program: PathBuf::from("C:\\Old\\old.exe"),
        },
        found: found(),
    };
    assert_eq!(
        words(&settings),
        [
            "Default browser",
            "Example Browser",
            "Another",
            "Old Browser",
            PICK_WORDS
        ]
    );
    assert_eq!(settings.choices().1, GuideChoice::Listed(2));
}

fn asked_for_the_guide(h: &egui_kittest::Harness<'static, State>) -> bool {
    h.state()
        .1
        .actions
        .contains(&WorkbenchAction::OpenGuide(None))
}

#[test]
fn documentation_is_in_the_tab_bar_with_no_project_and_opens_the_guide() {
    let mut w = Workbench::new(info());
    w.start_on_recent_projects();
    let mut h = harness(w);
    h.run();
    assert!(h.state().0.open.project.is_none());
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::File);
    h.get_by_label(DOCUMENTATION).click();
    h.run();
    assert!(asked_for_the_guide(&h));
}

#[test]
fn documentation_is_on_every_tab_right_of_the_project_and_left_of_the_chevron() {
    let mut h = harness(crate::test_support::in_layout(
        crate::shell::PanelLayout::default(),
    ));
    h.run();
    for tab in [
        RibbonTab::Home,
        RibbonTab::Library,
        RibbonTab::Animate,
        RibbonTab::Ui,
        RibbonTab::Audio,
        RibbonTab::Debug,
        RibbonTab::Build,
        RibbonTab::File,
    ] {
        ribbon_tab(&mut h, tab);
        let button = h.get_by_label(DOCUMENTATION).rect();
        let chevron = h.get_by_label("Collapse the ribbon").rect();
        let file_tab = h.get_by_label("File tab").rect();
        assert!(
            file_tab.y_range().contains(button.center().y),
            "{tab:?}: in the tab bar, level with its tabs"
        );
        assert!(
            button.right() <= chevron.left(),
            "{tab:?}: left of the chevron"
        );
        assert!(button.left() > 1000.0, "{tab:?}: at the bar's right end");
    }
}

#[test]
fn the_search_finds_documentation_too() {
    let w = Workbench::new(info());
    let command = crate::shell::ShellCommand::named("Documentation ↗", &w);
    assert_eq!(command, Some(crate::shell::ShellCommand::OpenGuide));
    assert!(w.search_items().iter().any(|i| i.name == "Documentation ↗"));
}

#[test]
fn a_program_you_choose_asks_the_app_for_one() {
    let mut w = Workbench::new(info());
    w.set_found_browsers(found());
    let _ = crate::test_support::do_command(&mut w, &egui::Context::default(), "Preferences");
    let mut h = harness(w);
    h.run();
    h.get_by_role_and_label(Role::ComboBox, "Open the guide in")
        .click();
    h.run();
    h.get_by_role_and_label(Role::RadioButton, PICK_WORDS)
        .click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::PickGuideBrowser)
    );
    h.get_by_role_and_label(Role::ComboBox, "Open the guide in")
        .click();
    h.run();
    h.get_by_role_and_label(Role::RadioButton, "Example Browser")
        .click();
    h.run();
    assert!(matches!(
        h.state().0.guide_browser(),
        GuideBrowser::Installed { name, .. } if name == "Example Browser"
    ));
}
