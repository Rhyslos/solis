//! File > Preferences: the window, its pages and its search; the
//! Appearance page, drawn with the design's own controls; and Hide
//! unfinished parts, with the one unfinished part today's editor shows.

use super::*;
use crate::panel::Panel;
use crate::test_support::*;
use egui::accesskit::Role;
use egui_kittest::kittest::{By, NodeT as _, Queryable};

/// Opens Preferences (the Settings window), as File, Preferences and
/// the search do.
fn opened() -> egui_kittest::Harness<'static, State> {
    let mut h = harness(Workbench::new(info()));
    h.run();
    run(&mut h, "Preferences");
    h
}

#[test]
fn edit_settings_opens_the_window_on_appearance() {
    let h = opened();
    assert!(h.state().0.settings_window.is_some());
    assert!(
        h.query_by_role_and_label(Role::Label, "Accent colour")
            .is_some()
    );
    assert!(h.query_by_label("Reset layout").is_some());
}

/// Opens the drop-down named `setting` and picks `choice` in its list.
fn choose(h: &mut egui_kittest::Harness<'static, State>, setting: &str, choice: &str) {
    h.get_by_role_and_label(Role::ComboBox, setting).click();
    h.run();
    h.get_by_role_and_label(Role::RadioButton, choice).click();
    h.run();
}

/// Clicks one of a row of choices (a swatch, a segment).
fn pick(h: &mut egui_kittest::Harness<'static, State>, choice: &str) {
    h.get_by_role_and_label(Role::RadioButton, choice).click();
    h.run();
}

#[test]
fn choosing_an_accent_changes_the_editor_s_colour() {
    let mut h = opened();
    // The accent restyles the editor, whose headings need its fonts.
    h.ctx.set_fonts(theme::fonts());
    h.run();
    pick(&mut h, "Purple");
    assert_eq!(h.state().0.accent(), Accent::Purple);
    assert_eq!(h.state().0.interface_look().accent, Accent::Purple);
    let style = h.ctx.global_style();
    assert_eq!(
        style.visuals.selection.bg_fill,
        crate::tokens::select_row(Accent::Purple)
    );
    // Reset this page puts the standard one back.
    h.get_by_label("Reset this page").click();
    h.run();
    assert_eq!(h.state().0.accent(), Accent::default());
}

#[test]
fn the_five_accents_are_offered_and_the_old_ones_are_gone() {
    let mut h = opened();
    h.ctx.set_fonts(theme::fonts());
    h.run();
    // Each is a swatch on the page, the one in use marked.
    for accent in Accent::ALL {
        let swatch = h.get_by_role_and_label(Role::RadioButton, accent.name());
        let marked = swatch.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True);
        assert_eq!(marked, accent == Accent::default(), "{}", accent.name());
    }
    for gone in ["Plum", "Marine green", "Petrol blue", "Amber", "Gold"] {
        assert!(h.query_by_label(gone).is_none(), "{gone}");
    }
}

#[test]
fn the_theme_and_rounded_panels_settings_are_gone() {
    let h = opened();
    for gone in ["Theme", "Rounded panels", "Use rounded panels", "Graphite"] {
        assert!(h.query_by_label(gone).is_none(), "{gone}");
    }
    assert!(!Setting::ALL.iter().any(|s| s.matches("graphite")));
    assert!(!Setting::ALL.iter().any(|s| s.matches("rounded")));
}

#[test]
fn interface_scale_draws_everything_larger_and_reset_puts_it_back() {
    let mut h = opened();
    h.ctx.set_fonts(theme::fonts());
    h.run();
    assert!(
        h.query_by_role_and_label(Role::Label, "Interface scale")
            .is_some()
    );
    let wide = h.ctx.content_rect().width();
    choose(&mut h, "Interface scale", "150%");
    h.run();
    assert_eq!(h.state().0.interface_look().scale.percent(), 150);
    assert_eq!(h.ctx.zoom_factor(), 1.5);
    // Each point is now a pixel and a half, so the same window holds
    // two thirds as many points across.
    assert_eq!(h.ctx.pixels_per_point(), 1.5);
    let narrow = h.ctx.content_rect().width();
    assert!((narrow * 1.5 - wide).abs() < 1.0, "{wide} then {narrow}");
    // The accent kept its choice.
    assert_eq!(h.state().0.accent(), Accent::default());
    // The Settings window never outgrows the editor's: with less room
    // its page scrolls, so the foot of the page is scrolled to.
    let screen = h.ctx.content_rect();
    let window = h.get_by_role_and_label(Role::Window, "Settings").rect();
    assert!(screen.contains_rect(window), "{window:?} in {screen:?}");
    h.get_by_label("Reset this page").scroll_to_me();
    h.run();
    h.get_by_label("Reset this page").click();
    h.run();
    h.run();
    assert_eq!(
        h.state().0.interface_look(),
        theme::InterfaceLook::default()
    );
    assert_eq!(h.ctx.zoom_factor(), 1.0);
}

#[test]
fn density_makes_buttons_and_fields_taller_or_shorter() {
    let mut h = opened();
    h.ctx.set_fonts(theme::fonts());
    // The standard look first, as the app applies it when it starts.
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
    h.run();
    assert!(h.query_by_role_and_label(Role::Label, "Density").is_some());
    // A field (here the scale's drop-down) is as tall as the density says.
    let height = |h: &egui_kittest::Harness<'static, State>| {
        h.get_by_role_and_label(Role::ComboBox, "Interface scale")
            .rect()
            .height()
    };
    assert_eq!(height(&h), 24.0);
    pick(&mut h, "Compact");
    h.run();
    assert_eq!(
        h.state().0.interface_look().density,
        theme::Density::Compact
    );
    assert_eq!(height(&h), 22.0);
    pick(&mut h, "Comfortable");
    h.run();
    assert_eq!(height(&h), 28.0);
    h.get_by_label("Reset this page").click();
    h.run();
    h.run();
    assert_eq!(height(&h), 24.0);
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn reset_this_page_leaves_the_switches_of_other_pages_alone() {
    let mut w = Workbench::new(info()).with_interface_look(theme::InterfaceLook {
        accent: Accent::Teal,
        hide_unfinished: true,
        ribbon_labels: false,
        ..theme::InterfaceLook::default()
    });
    w.open_settings();
    let mut h = harness(w);
    h.ctx.set_fonts(theme::fonts());
    h.run();
    h.get_by_label("Reset this page").click();
    h.run();
    let look = h.state().0.interface_look();
    assert_eq!(look.accent, Accent::default());
    // Hide unfinished parts is on this page, so it goes back too; the
    // ribbon's labels aren't, and keep their choice.
    assert!(!look.hide_unfinished);
    assert!(!look.ribbon_labels);
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn reset_layout_puts_the_tab_s_middle_back() {
    let mut h = opened();
    let tab = h.state().0.ribbon_tab();
    let standard = h.state().0.shell.arrangement(tab);
    {
        let w = &mut h.state_mut().0;
        w.show_panel(Panel::Console);
        w.fold_shelf(true);
        w.shell.arrangement_mut(tab).shelf_height = 410.0;
    }
    h.get_all_by_label("Reset layout").next().unwrap().click();
    h.run();
    assert_eq!(h.state().0.shell.arrangement(tab), standard);
}

#[test]
fn the_search_box_narrows_the_settings() {
    let mut w = Workbench::new(info());
    w.open_settings();
    w.settings_window.as_mut().unwrap().search = "layout".into();
    let mut h = harness(w);
    h.run();
    assert!(h.query_by_label("Reset layout").is_some());
    assert!(h.query_all_by_label("Accent colour").next().is_none());
    h.state_mut().0.settings_window.as_mut().unwrap().search = "zebra".into();
    h.run();
    assert!(h.query_by_label("No setting matches that.").is_some());
}

#[test]
fn reset_layout_is_a_command_and_there_are_no_presets() {
    let mut h = harness(Workbench::new(info()));
    h.run();
    let names: Vec<String> = h
        .state()
        .0
        .search_items()
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert!(names.contains(&"Reset layout".to_string()));
    for gone in ["Layouts", "Wide screen", "Small screen"] {
        assert!(!names.iter().any(|n| n == gone), "{gone}");
        assert!(h.query_by_label(gone).is_none(), "{gone}");
    }
}

#[test]
fn search_finds_settings_by_name_hint_or_page() {
    assert!(Setting::Accent.matches("accent"));
    assert!(Setting::Accent.matches("colour"));
    assert!(Setting::Layout.matches("panel"));
    assert!(Setting::Layout.matches("appearance"));
    assert!(Setting::Scale.matches("scale"));
    assert!(Setting::Scale.matches("larger"));
    assert!(Setting::Density.matches("compact"));
    // Every word must be found.
    assert!(Setting::Accent.matches("accent colour"));
    assert!(!Setting::Accent.matches("accent zebra"));
    assert!(Setting::ALL.iter().all(|s| s.matches("")));
}

#[test]
fn every_setting_is_on_a_listed_page() {
    for setting in Setting::ALL {
        assert!(Page::ALL.contains(&setting.page()), "{}", setting.name());
    }
}

/// The Settings window on Appearance, in the editor's own fonts and
/// standard look, as the app shows it.
fn appearance() -> egui_kittest::Harness<'static, State> {
    let mut h = opened();
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
    h.run();
    h
}

/// The unfinished parts in the Settings window: not the frame's own
/// (the tab bar's and the ribbon's above it, the shelf bar's and the
/// status bar's under it).
fn unbuilt_in_the_window<'h>(
    h: &'h egui_kittest::Harness<'static, State>,
) -> Vec<egui_kittest::Node<'h>> {
    use crate::tokens::size;
    use crate::unbuilt::{UnbuiltId, get};
    let frame: Vec<&str> = [
        UnbuiltId::ShellStatusBranch,
        UnbuiltId::HomeShelfAnimation,
        UnbuiltId::HomeShelfTransport,
        UnbuiltId::HomeShelfFrameCounter,
        UnbuiltId::HomeToolPose,
        UnbuiltId::HomeToolMotionPath,
        UnbuiltId::HomeToolAddKey,
        UnbuiltId::HomeToolMeasure,
    ]
    .into_iter()
    .map(|id| get(id).name)
    .collect();
    h.query_all(
        By::new()
            .predicate(|node| node.description().as_deref() == Some(crate::unbuilt::DESCRIPTION)),
    )
    .filter(|node| node.rect().top() >= size::TAB_BAR_H + size::RIBBON_H)
    .filter(|node| {
        let label = node.accesskit_node().label().unwrap_or_default();
        !frame.contains(&label.as_str())
    })
    .collect()
}

/// The names of everything in the window described as not built yet.
fn described_as_unbuilt(h: &egui_kittest::Harness<'static, State>) -> Vec<String> {
    unbuilt_in_the_window(h)
        .iter()
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

/// The one unfinished part on the page.
fn unbuilt_row<'h>(h: &'h egui_kittest::Harness<'static, State>) -> egui_kittest::Node<'h> {
    let mut parts = unbuilt_in_the_window(h);
    assert_eq!(parts.len(), 1, "one unfinished part on the page");
    parts.remove(0)
}

#[test]
fn the_appearance_page_is_laid_out_as_the_design_s_settings_are() {
    use crate::tokens::size;
    let h = appearance();
    // Small capitals over each section.
    for caps in ["THEME", "LAYOUT"] {
        assert!(h.query_by_label(caps).is_some(), "{caps}");
    }
    // Each control is the design's own, at the design's size.
    let rect_of = |role: Role, name: &str| h.get_by_role_and_label(role, name).rect();
    let toggle = rect_of(Role::CheckBox, "Show hints");
    assert_eq!([toggle.width(), toggle.height()], size::TOGGLE);
    let hide = rect_of(Role::CheckBox, "Hide unfinished parts");
    assert_eq!([hide.width(), hide.height()], size::TOGGLE);
    let swatch = rect_of(Role::RadioButton, "Blue");
    assert_eq!([swatch.width(), swatch.height()], [size::SWATCH; 2]);
    assert_eq!(
        rect_of(Role::RadioButton, "Default").height(),
        size::SEGMENT_H
    );
    let scale = rect_of(Role::ComboBox, "Interface scale");
    assert_eq!(
        [scale.width(), scale.height()],
        [size::SETTING_FIELD_W, 24.0]
    );
    let reset = rect_of(Role::Button, "Reset layout");
    assert_eq!(reset.height(), size::PAGE_BUTTON_H);
    // Every row's control ends at the same right edge, the names start
    // at the same left edge, and the rows come in the design's order.
    let purple = rect_of(Role::RadioButton, "Purple");
    for control in [toggle, hide, scale, reset] {
        assert_eq!(control.right(), purple.right());
    }
    let names: Vec<egui::Rect> = [
        "Accent colour",
        "Interface scale",
        "Density",
        "Panel layout",
        "Hints",
        "Hide unfinished parts",
    ]
    .iter()
    .map(|name| rect_of(Role::Label, name))
    .collect();
    for pair in names.windows(2) {
        assert_eq!(pair[0].left(), pair[1].left());
        assert!(pair[0].top() < pair[1].top());
    }
    // A name is to the left of its control, with room between.
    assert!(names[1].right() + size::SETTING_COLUMN_GAP <= scale.left());
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn the_page_is_used_from_the_keyboard() {
    use egui::Key;
    let mut h = appearance();
    h.get_by_role_and_label(Role::CheckBox, "Show hints")
        .focus();
    h.run();
    assert!(h.state().0.hints_on());
    h.key_press(Key::Space);
    h.run();
    assert!(!h.state().0.hints_on());
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().0.hints_on());
    // The arrow keys move along the accents and the densities.
    h.get_by_role_and_label(Role::RadioButton, "Blue").focus();
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().0.accent(), Accent::Grey);
    h.get_by_role_and_label(Role::RadioButton, "Default")
        .focus();
    h.run();
    h.key_press(Key::ArrowLeft);
    h.run();
    assert_eq!(
        h.state().0.interface_look().density,
        theme::Density::Compact
    );
    // And step the scale without opening its list.
    h.get_by_role_and_label(Role::ComboBox, "Interface scale")
        .focus();
    h.run();
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(h.state().0.interface_look().scale.percent(), 125);
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn code_editor_split_is_shown_as_a_part_that_is_not_built_yet() {
    use crate::unbuilt::{self, UnbuiltId};
    let mut h = appearance();
    assert_eq!(described_as_unbuilt(&h), ["Code editor split"]);
    assert!(unbuilt::drawn(&h.ctx).contains(&UnbuiltId::PrefsCodeSplit));
    // It stands last in the Layout section, a row like the others.
    let row = unbuilt_row(&h).rect();
    let hide = h
        .get_by_role_and_label(Role::CheckBox, "Hide unfinished parts")
        .rect();
    assert!(row.top() > hide.bottom());
    assert_eq!(row.right(), hide.right());
    // A click changes nothing and says so at the right of the status bar.
    let before = h.state().0.interface_look();
    unbuilt_row(&h).click();
    h.step();
    h.step();
    let words = "Code editor split isn't built yet (not planned yet)";
    assert!(h.query_by_label(words).is_some());
    assert_eq!(h.state().0.interface_look(), before);
    for _ in 0..14 {
        h.step();
    }
    assert!(
        h.query_by_label(words).is_none(),
        "the words go by themselves"
    );
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn hide_unfinished_parts_takes_it_away_and_shows_it_again() {
    let mut h = appearance();
    let tops = |h: &egui_kittest::Harness<'static, State>| -> Vec<f32> {
        ["Hints", "Hide unfinished parts"]
            .iter()
            .map(|name| h.get_by_role_and_label(Role::Label, name).rect().top())
            .collect()
    };
    let before = tops(&h);
    h.get_by_role_and_label(Role::CheckBox, "Hide unfinished parts")
        .click();
    h.run();
    h.run();
    assert!(h.state().0.interface_look().hide_unfinished);
    assert!(crate::unbuilt::hidden());
    assert!(described_as_unbuilt(&h).is_empty());
    assert!(crate::unbuilt::drawn(&h.ctx).is_empty());
    assert!(h.query_by_label("Code editor split").is_none());
    // Everything that works is where it was.
    assert_eq!(tops(&h), before);
    // The switch shows it is on, and turns it off again.
    let switch = h.get_by_role_and_label(Role::CheckBox, "Hide unfinished parts");
    assert_eq!(
        switch.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    switch.click();
    h.run();
    h.run();
    assert!(!h.state().0.interface_look().hide_unfinished);
    assert_eq!(described_as_unbuilt(&h), ["Code editor split"]);
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn an_editor_started_with_them_hidden_never_draws_one() {
    let mut w = Workbench::new(info()).with_interface_look(theme::InterfaceLook {
        hide_unfinished: true,
        ..theme::InterfaceLook::default()
    });
    w.open_settings();
    let mut h = harness(w);
    h.ctx.set_fonts(theme::fonts());
    h.run();
    h.run();
    assert!(described_as_unbuilt(&h).is_empty());
    assert!(crate::unbuilt::drawn(&h.ctx).is_empty());
    assert!(h.query_by_role_and_label(Role::Label, "Hints").is_some());
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn searching_the_settings_finds_the_switch_and_never_an_unfinished_part() {
    let mut w = Workbench::new(info());
    w.open_settings();
    w.settings_window.as_mut().unwrap().search = "unfinished".into();
    let mut h = harness(w);
    h.ctx.set_fonts(theme::fonts());
    h.run();
    h.run();
    assert!(
        h.query_by_role_and_label(Role::CheckBox, "Hide unfinished parts")
            .is_some()
    );
    h.state_mut().0.settings_window.as_mut().unwrap().search = "code editor".into();
    h.run();
    assert!(h.query_by_label("No setting matches that.").is_some());
    assert!(described_as_unbuilt(&h).is_empty());
    assert!(Setting::HideUnfinished.matches("hide"));
    assert!(Setting::HideUnfinished.matches("grey mark"));
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}
