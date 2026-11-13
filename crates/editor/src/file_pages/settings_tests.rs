//! File › Preferences: its sub-tabs and its search; the Appearance and
//! General tabs, drawn with the design's own controls; and Hide
//! unfinished parts.

use super::*;
use crate::panel::Panel;
use crate::test_support::*;
use egui::accesskit::Role;
use egui_kittest::kittest::{By, NodeT as _, Queryable};

/// Opens Preferences, as File, Preferences and the search do, on its
/// Appearance tab.
fn opened() -> egui_kittest::Harness<'static, State> {
    let mut h = harness(Workbench::new(info()));
    h.run();
    run(&mut h, "Preferences");
    assert_eq!(h.state().0.file_pages.page, Page::Preferences);
    h.get_by_role_and_label(Role::Button, "Appearance").click();
    h.run();
    h
}

#[test]
fn preferences_is_a_page_with_appearance_among_its_tabs() {
    let h = opened();
    assert_eq!(h.state().0.file_pages.prefs_tab, Tab::Appearance);
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
    // Reset to defaults puts the standard one back.
    h.get_by_label("Reset to defaults").click();
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
    h.get_by_label("Reset to defaults").scroll_to_me();
    h.run();
    h.get_by_label("Reset to defaults").click();
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
    h.get_by_label("Reset to defaults").click();
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
    w.show_settings(Tab::Appearance);
    let mut h = harness(w);
    h.ctx.set_fonts(theme::fonts());
    h.run();
    h.get_by_label("Reset to defaults").click();
    h.run();
    let look = h.state().0.interface_look();
    assert_eq!(look.accent, Accent::default());
    // Hide unfinished parts is on General, and the ribbon's labels on no
    // tab: both keep their choice.
    assert!(look.hide_unfinished);
    assert!(!look.ribbon_labels);
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn reset_layout_puts_the_tab_s_middle_back() {
    let mut h = opened();
    // The tab File was opened from.
    let tab = crate::shell::RibbonTab::Home;
    let standard = h.state().0.shell.arrangement(tab);
    let sizes = h.state().0.shell.sizes.clone();
    {
        use crate::shell::{PanelLayout, sizes::Edge};
        let w = &mut h.state_mut().0;
        w.show_panel(Panel::Console);
        w.fold_shelf(true);
        // A border in two layouts: both are put back.
        let shelf = PanelLayout::Shelf;
        w.shell.sizes.set(tab, shelf, Edge::ShelfHeight, 410.0);
        w.shell.sizes.set(tab, PanelLayout::AreaAndAssetsLeft, Edge::Left, 410.0);
    }
    h.get_all_by_label("Reset layout").next().unwrap().click();
    h.run();
    assert_eq!(h.state().0.shell.arrangement(tab), standard);
    assert_eq!(h.state().0.shell.sizes, sizes);
}

#[test]
fn the_search_box_narrows_the_settings() {
    let mut w = Workbench::new(info());
    w.show_settings(Tab::General);
    w.file_pages.settings_search = "layout".into();
    let mut h = harness(w);
    h.run();
    assert!(h.query_by_label("Reset layout").is_some(), "found on another tab");
    assert!(h.query_all_by_label("Accent colour").next().is_none());
    h.state_mut().0.file_pages.settings_search = "zebra".into();
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
    assert!(Setting::BaseResolution.matches("2d pixels"));
    assert!(Setting::GameName.matches("build saves"));
    assert!(Setting::Scale.matches("scale"));
    assert!(Setting::Scale.matches("sharp"));
    assert!(Setting::Density.matches("compact"));
    // Every word must be found.
    assert!(Setting::Accent.matches("accent colour"));
    assert!(!Setting::Accent.matches("accent zebra"));
    assert!(Setting::ALL.iter().all(|s| s.matches("")));
}

#[test]
fn every_setting_is_on_a_listed_tab_and_drawn_there() {
    use crate::unbuilt::Entry;
    let tabs: Vec<Tab> = Tab::preferences()
        .into_iter()
        .chain(Tab::project())
        .filter_map(|e| match e {
            Entry::Built(tab) => Some(tab),
            Entry::Unbuilt(_) => None,
        })
        .collect();
    for setting in Setting::ALL {
        let tab = setting.tab();
        assert!(tabs.contains(&tab), "{}", setting.name());
        let on_it = tab
            .sections()
            .into_iter()
            .flat_map(|(_, rows)| rows)
            .any(|row| row == Entry::Built(setting));
        assert!(on_it, "{} is in its tab's sections", setting.name());
    }
}


/// Preferences on `tab`, in the editor's own fonts and standard look.
fn on(tab: Tab, look: theme::InterfaceLook) -> egui_kittest::Harness<'static, State> {
    let mut w = Workbench::new(info()).with_interface_look(look);
    w.show_settings(tab);
    let mut h = harness(w);
    h.ctx.set_fonts(theme::fonts());
    theme::restyle(&h.ctx, look);
    h.run();
    h.run();
    h
}

#[test]
fn the_appearance_tab_is_laid_out_as_the_design_s_settings_are() {
    use crate::tokens::size;
    let h = on(Tab::Appearance, theme::InterfaceLook::default());
    for caps in ["THEME", "LAYOUT"] {
        assert!(h.query_by_label(caps).is_some(), "{caps}");
    }
    let rect_of = |role: Role, name: &str| h.get_by_role_and_label(role, name).rect();
    let swatch = rect_of(Role::RadioButton, "Blue");
    assert_eq!([swatch.width(), swatch.height()], [size::SWATCH; 2]);
    assert_eq!(rect_of(Role::RadioButton, "Default").height(), size::SEGMENT_H);
    let scale = rect_of(Role::ComboBox, "Interface scale");
    assert_eq!([scale.width(), scale.height()], [size::SETTING_FIELD_W, 24.0]);
    let reset = rect_of(Role::Button, "Reset layout");
    assert_eq!(reset.height(), size::PAGE_BUTTON_H);
    // Every row's control ends at the same right edge, the names start
    // at the same left edge, and the rows come in the design's order.
    let purple = rect_of(Role::RadioButton, "Purple");
    for control in [scale, reset, rect_of(Role::ComboBox, "Shelf opens on")] {
        assert_eq!(control.right(), purple.right());
    }
    let names: Vec<egui::Rect> = ["Accent colour", "Interface scale", "Density", "Shelf opens on", "Panel layout"]
        .iter()
        .map(|name| rect_of(Role::Label, name))
        .collect();
    for pair in names.windows(2) {
        assert_eq!(pair[0].left(), pair[1].left());
        assert!(pair[0].top() < pair[1].top());
    }
    assert!(names[1].right() + size::SETTING_COLUMN_GAP <= scale.left());
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn the_tabs_are_used_from_the_keyboard() {
    use egui::Key;
    let mut h = on(Tab::General, theme::InterfaceLook::default());
    h.get_by_role_and_label(Role::CheckBox, "Show hints").focus();
    h.run();
    assert!(h.state().0.hints_on());
    h.key_press(Key::Space);
    h.run();
    assert!(!h.state().0.hints_on());
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().0.hints_on());
    h.state_mut().0.show_settings(Tab::Appearance);
    h.run();
    h.get_by_role_and_label(Role::RadioButton, "Blue").focus();
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().0.accent(), Accent::Grey);
    h.get_by_role_and_label(Role::RadioButton, "Default").focus();
    h.run();
    h.key_press(Key::ArrowLeft);
    h.run();
    assert_eq!(h.state().0.interface_look().density, theme::Density::Compact);
    h.get_by_role_and_label(Role::ComboBox, "Interface scale").focus();
    h.run();
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(h.state().0.interface_look().scale.percent(), 125);
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn code_editor_split_is_shown_as_a_part_that_is_not_built_yet() {
    use crate::unbuilt::{self, UnbuiltId, get};
    let mut h = on(Tab::Appearance, theme::InterfaceLook::default());
    assert!(unbuilt::drawn(&h.ctx).contains(&UnbuiltId::PrefsCodeSplit));
    let row = h.get_by_label(get(UnbuiltId::PrefsCodeSplit).name).rect();
    let layout = h.get_by_role_and_label(Role::Button, "Reset layout").rect();
    assert!(row.top() > layout.bottom(), "last in Layout");
    let before = h.state().0.interface_look();
    h.get_by_label(get(UnbuiltId::PrefsCodeSplit).name).click();
    h.step();
    h.step();
    let words = "Code editor split isn't built yet (not planned yet)";
    assert!(h.query_by_label(words).is_some());
    assert_eq!(h.state().0.interface_look(), before);
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn hide_unfinished_parts_takes_them_away_and_shows_them_again() {
    use crate::unbuilt::{self, UnbuiltId};
    let mut h = on(Tab::General, theme::InterfaceLook::default());
    assert!(unbuilt::drawn(&h.ctx).contains(&UnbuiltId::PrefsTabInput), "a sub-tab");
    let hints = h.get_by_role_and_label(Role::Label, "Show hints").rect().top();
    h.get_by_role_and_label(Role::CheckBox, "Hide unfinished parts").click();
    h.run();
    h.run();
    assert!(h.state().0.interface_look().hide_unfinished);
    assert!(unbuilt::drawn(&h.ctx).is_empty());
    assert!(h.query_by_label("How an unfinished part looks").is_none());
    assert_eq!(h.get_by_role_and_label(Role::Label, "Show hints").rect().top(), hints);
    h.get_by_role_and_label(Role::CheckBox, "Hide unfinished parts").click();
    h.run();
    h.run();
    assert!(!h.state().0.interface_look().hide_unfinished);
    assert!(unbuilt::drawn(&h.ctx).contains(&UnbuiltId::PrefsTabInput));
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn searching_the_settings_finds_the_switch_and_never_an_unfinished_part() {
    let mut h = on(Tab::Appearance, theme::InterfaceLook::default());
    h.state_mut().0.file_pages.settings_search = "unfinished".into();
    h.run();
    h.run();
    assert!(h.query_by_role_and_label(Role::CheckBox, "Hide unfinished parts").is_some());
    h.state_mut().0.file_pages.settings_search = "code editor".into();
    h.run();
    assert!(h.query_by_label("No setting matches that.").is_some());
    assert!(Setting::HideUnfinished.matches("hide"));
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn the_viewport_tab_changes_what_the_viewport_shows() {
    let mut h = on(Tab::Viewport, theme::InterfaceLook::default());
    assert!(h.state().0.viewport.overlays.stats);
    h.get_by_role_and_label(Role::CheckBox, "Frame time and frame rate").click();
    h.run();
    assert!(!h.state().0.viewport.overlays.stats);
    h.get_all_by_role_and_label(Role::RadioButton, "Tiles").next().unwrap().click();
    h.run();
    assert_eq!(
        h.state().0.viewport.snapping.pixel_snap,
        crate::transforming::PixelSnap::Tiles
    );
    h.get_by_role_and_label(Role::RadioButton, "Stylized").click();
    h.run();
    assert_eq!(h.state().0.viewport.look.kind, crate::look::LookKind::Stylized);
    h.get_by_label("Reset to defaults").click();
    h.run();
    assert!(h.state().0.viewport.overlays.stats);
    assert_eq!(h.state().0.viewport.look.kind, crate::look::LookKind::Standard);
    theme::restyle(&h.ctx, theme::InterfaceLook::default());
}

#[test]
fn shelf_opens_on_entity_brings_it_to_the_front_when_something_is_selected() {
    use crate::interface_look::ShelfOpensOn;
    use crate::shell::{RibbonTab, workspaces::ShelfTab};
    let (w, crate_id) = with_a_crate();
    let mut h = harness(w.with_interface_look(theme::InterfaceLook {
        shelf_opens_on: ShelfOpensOn::Entity,
        ..theme::InterfaceLook::default()
    }));
    h.run();
    h.state_mut().0.show_panel(Panel::Console);
    h.state_mut().0.open.document.selection.clear();
    h.run();
    h.state_mut().0.open.document.selection.select(crate_id);
    h.run();
    assert_eq!(h.state().0.shell.arrangement(RibbonTab::Home).shelf_tab, ShelfTab::Entity);
    // As it is (the standard): the Console stays.
    let mut look = h.state().0.interface_look();
    look.shelf_opens_on = ShelfOpensOn::AsItIs;
    h.state_mut().0.interface = look;
    h.state_mut().0.show_panel(Panel::Console);
    h.state_mut().0.open.document.selection.clear();
    h.run();
    h.state_mut().0.open.document.selection.select(crate_id);
    h.run();
    assert_eq!(h.state().0.shell.arrangement(RibbonTab::Home).shelf_tab, ShelfTab::Console);
}
