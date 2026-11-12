use super::*;
use crate::test_support::{harness, info, ribbon_tab, run};
use crate::workbench::WorkbenchAction;
use std::collections::BTreeSet;

fn fresh() -> Workbench {
    Workbench::new(info())
}

#[test]
fn every_command_has_a_name_of_its_own_and_a_place() {
    let w = fresh();
    let mut names = BTreeSet::new();
    for command in ShellCommand::ALL {
        let name = command.name(&w);
        assert!(!name.is_empty(), "{command:?} has no name");
        assert!(!command.place().is_empty(), "{command:?} has no place");
        assert!(names.insert(name.clone()), "two commands are called {name}");
        assert_eq!(ShellCommand::named(&name, &w), Some(command));
    }
    let listed: BTreeSet<String> = ShellCommand::ALL.iter().map(|c| format!("{c:?}")).collect();
    assert_eq!(
        listed.len(),
        ShellCommand::ALL.len(),
        "a command is listed twice"
    );
}

#[test]
fn a_switch_is_named_for_what_choosing_it_does_now() {
    let mut h = harness(fresh());
    h.run();
    run(&mut h, "Snap to the grid");
    assert!(h.state().0.viewport.snapping.grid);
    run(&mut h, "Stop snapping to the grid");
    assert!(!h.state().0.viewport.snapping.grid);
    run(&mut h, "Hide unfinished parts");
    assert!(h.state().0.interface_look().hide_unfinished);
    run(&mut h, "Show unfinished parts");
    assert!(!h.state().0.interface_look().hide_unfinished);
}

#[test]
fn a_command_that_cant_be_done_says_why_and_does_nothing() {
    let mut w = fresh();
    let ctx = egui::Context::default();
    let mut actions = Vec::new();
    assert_eq!(
        ShellCommand::Pause.why_not(&w).as_deref(),
        Some("Press Play first")
    );
    assert_eq!(
        ShellCommand::Delete.why_not(&w).as_deref(),
        Some("Select an entity first")
    );
    ShellCommand::Step.run(&mut w, &ctx, &mut actions);
    assert!(actions.is_empty());
    w.play();
    assert_eq!(
        ShellCommand::SaveArea.why_not(&w).as_deref(),
        Some("Stop the game first")
    );
    ShellCommand::SaveArea.run(&mut w, &ctx, &mut actions);
    assert!(actions.is_empty(), "nothing is saved while the game runs");
    w.stop();
    ShellCommand::SaveArea.run(&mut w, &ctx, &mut actions);
    assert_eq!(actions, [WorkbenchAction::SaveArea]);
}

#[test]
fn a_hover_text_gives_the_name_the_shortcut_and_the_reason() {
    let w = fresh();
    let save = ShellCommand::SaveArea.hover_text(&w);
    assert!(save.starts_with("Save area (Ctrl+S)\n"), "{save}");
    let undo = ShellCommand::Undo.hover_text(&w);
    assert_eq!(undo, "Undo (Ctrl+Z)\nNothing to undo");
}

#[test]
fn each_tab_shows_its_own_documents_and_shelf() {
    let mut h = harness(fresh());
    h.run();
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Home);
    for (tab, documents, shelf) in [
        // Nothing open: the sequencer's page, and the Console (round 3).
        (RibbonTab::Animate, vec![], ShelfTab::Console),
        (
            RibbonTab::Debug,
            vec![Panel::Game, Panel::Profiler],
            ShelfTab::Console,
        ),
        (RibbonTab::Ui, vec![Panel::Canvas], ShelfTab::Screen),
        (RibbonTab::Library, vec![Panel::Library], ShelfTab::Details),
        (RibbonTab::Audio, vec![Panel::Mixer], ShelfTab::Bus),
        (RibbonTab::Home, vec![Panel::Viewport], ShelfTab::Entity),
    ] {
        ribbon_tab(&mut h, tab);
        h.run();
        let w = &h.state().0;
        assert_eq!(w.ribbon_tab(), tab);
        assert_eq!(w.shelf_tab(), Some(shelf), "{tab:?}");
        for panel in documents {
            assert!(w.in_front.contains(&panel), "{tab:?} shows {panel:?}");
        }
        for panel in shelf.panels() {
            assert!(
                w.in_front.contains(&panel),
                "{tab:?}'s shelf shows {panel:?}"
            );
        }
    }
}

#[test]
fn a_tab_that_isnt_built_cant_be_shown_while_unfinished_parts_are_hidden() {
    let mut h = harness(fresh());
    ribbon_tab(&mut h, RibbonTab::Shading);
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Shading);
    run(&mut h, "Hide unfinished parts");
    h.run();
    assert_eq!(
        h.state().0.ribbon_tab(),
        RibbonTab::Home,
        "the tab in front went with the unfinished parts"
    );
    ribbon_tab(&mut h, RibbonTab::Vfx);
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Home);
}

mod frame_layout {
    use super::*;
    use crate::test_support::{State, fonts_ready, harness};
    use crate::tokens::size;
    use egui_kittest::kittest::{NodeT as _, Queryable};

    /// The editor in a window `width` wide.
    fn window(width: f32, w: Workbench) -> egui_kittest::Harness<'static, State> {
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(width, 700.0))
            .build_ui_state(
                |ui, state: &mut State| {
                    if !fonts_ready(ui.ctx()) {
                        return;
                    }
                    let (workbench, picker) = state;
                    let actions = workbench.show(ui, picker);
                    picker.actions.extend(actions);
                },
                (w, Default::default()),
            );
        h.run();
        h.run();
        h
    }

    #[test]
    fn the_ribbon_is_66_tall_with_groups_side_by_side_and_the_design_s_sizes() {
        let mut h = harness(fresh());
        h.run();
        let groups = ribbon::groups_drawn(&h.ctx);
        let labels: Vec<_> = groups.iter().map(|(label, _)| *label).collect();
        assert_eq!(
            labels,
            [
                Some("Edit"),
                Some("Create"),
                None,
                Some("Lighting"),
                Some("Project"),
                Some("Play")
            ]
        );
        // The test window keeps a margin round the editor: places are
        // measured from the tab bar's own corner.
        let tab = h.get_by_label("Home tab").rect();
        let top = tab.bottom() - size::TAB_BAR_H;
        let left = groups[0].1.left();
        let wide = h.get_by_label("Collapse the ribbon").rect().right() + size::TAB_BAR_PAD - left;
        for (label, rect) in &groups {
            assert_eq!(rect.top(), top + size::TAB_BAR_H, "{label:?}");
            assert_eq!(rect.height(), size::RIBBON_H, "{label:?}");
        }
        // Each group starts where the one before it ends; Play is pushed
        // to the window's right edge.
        for pair in groups[..5].windows(2) {
            assert_eq!(pair[1].1.left(), pair[0].1.right());
        }
        assert_eq!(groups[5].1.right(), left + wide);
        assert!(groups[5].1.left() > groups[4].1.right() + 100.0);
        // A large button is 50 by 44; a row is 20 tall, or 19 where three
        // are stacked.
        assert_eq!(
            h.get_by_label("Add entity").rect().size(),
            egui::vec2(size::RIBBON_BIG[0], size::RIBBON_BIG[1])
        );
        assert_eq!(h.get_by_label("Play").rect().size(), egui::vec2(50.0, 44.0));
        assert_eq!(h.get_by_label("New tile map").rect().height(), 20.0);
        let rows = ["Grid · 1", "Angle · 15°", "Scale · 10%"].map(|l| h.get_by_label(l).rect());
        for row in rows {
            assert_eq!(row.height(), size::RIBBON_ROW_H_TIGHT);
            assert_eq!(row.left(), rows[0].left());
        }
        assert_eq!(rows[1].top() - rows[0].bottom(), size::RIBBON_ROW_GAP);
        // The tab bar above it is 34, its tabs 30 and standing on its foot.
        assert_eq!(tab.height(), size::RIBBON_TAB_H);
        assert_eq!(tab.bottom(), groups[0].1.top());
        // The first tab starts after the logo's tile and its room.
        let first = h.get_by_label("File tab").rect();
        assert_eq!(
            first.left() - left,
            size::TAB_BAR_PAD + size::LOGO_TILE + size::LOGO_GAP + size::TAB_BAR_GAP
        );
        let search = h.get_by_label("Search or run a command").rect();
        assert_eq!(search.size(), egui::vec2(size::SEARCH_W, size::SEARCH_H));
    }

    #[test]
    fn hidden_parts_leave_no_gap_and_no_empty_group() {
        let mut h = harness(fresh());
        run(&mut h, "Hide unfinished parts");
        h.run();
        let groups = ribbon::groups_drawn(&h.ctx);
        let labels: Vec<_> = groups.iter().map(|(label, _)| *label).collect();
        assert_eq!(
            labels,
            [
                Some("Edit"),
                Some("Create"),
                None,
                Some("Project"),
                Some("Play")
            ],
            "Lighting had nothing built, so the whole group went"
        );
        for pair in groups[..4].windows(2) {
            assert_eq!(pair[1].1.left(), pair[0].1.right());
        }
        // The tabs close up too.
        let animate = h.get_by_label("Animate tab").rect();
        assert_eq!(animate.left(), h.get_by_label("Library tab").rect().right());
        assert!(h.query_by_label("Shading").is_none());
        assert!(crate::unbuilt::drawn(&h.ctx).is_empty());
        run(&mut h, "Show unfinished parts");
    }

    #[test]
    fn a_narrow_window_scrolls_the_ribbon_sideways_and_the_wheel_moves_it() {
        let mut h = window(900.0, fresh());
        let before = ribbon::groups_drawn(&h.ctx);
        // Nothing wraps: every group is still in the one band.
        let band_top = before[0].1.top();
        assert!(before.iter().all(|(_, r)| r.top() == band_top));
        let play = before.last().unwrap().1;
        assert!(play.right() > 900.0, "the Play group is off to the right");
        let right_edge = h.get_by_label("Collapse the ribbon").rect().right() + size::TAB_BAR_PAD;
        // The wheel over the ribbon moves it sideways.
        // (Stepped frame by frame: a hover text over a button keeps
        // asking to be drawn again.)
        h.hover_at(egui::pos2(400.0, band_top + 30.0));
        h.step();
        for _ in 0..12 {
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -120.0),
                modifiers: egui::Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            });
            h.step();
        }
        for _ in 0..20 {
            h.step();
        }
        let after = ribbon::groups_drawn(&h.ctx);
        assert!(
            after[0].1.left() < before[0].1.left(),
            "the groups moved left"
        );
        let end = after.last().unwrap().1.right();
        assert!((end - right_edge).abs() < 1.0, "to the very end: {end}");
        // Elsewhere the wheel is nobody's sideways scroll.
        let kept = after[0].1.left();
        h.hover_at(egui::pos2(400.0, 400.0));
        h.step();
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 120.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        for _ in 0..10 {
            h.step();
        }
        assert_eq!(ribbon::groups_drawn(&h.ctx)[0].1.left(), kept);
    }

    #[test]
    fn the_chevron_and_a_double_click_collapse_the_ribbon_and_a_click_drops_it() {
        let mut h = harness(fresh());
        h.run();
        let viewport_top = |h: &egui_kittest::Harness<'static, State>| {
            h.state().0.shell.drawn_at(Panel::Viewport).unwrap().top()
        };
        let open = viewport_top(&h);
        h.get_by_label("Collapse the ribbon").click();
        h.run();
        assert!(h.state().0.ribbon_collapsed());
        assert_eq!(
            viewport_top(&h),
            open - size::RIBBON_H,
            "66 more for the center"
        );
        assert!(h.query_by_label("Add entity").is_none());
        // A click on a tab drops the ribbon over the center, which stays
        // where it is.
        h.get_by_label("Home tab").click();
        h.run();
        assert!(h.state().0.shell.dropped);
        assert_eq!(viewport_top(&h), open - size::RIBBON_H);
        // Choosing something on it puts it away again.
        let entities = h.state().0.document().area().len();
        h.get_by_label("Add entity").click();
        h.run();
        h.run();
        assert_eq!(h.state().0.document().area().len(), entities + 1);
        assert!(!h.state().0.shell.dropped);
        assert!(h.query_by_label("Add camera").is_none());
        // So does the pointer leaving it.
        h.get_by_label("Home tab").click();
        h.run();
        assert!(h.state().0.shell.dropped);
        h.hover_at(egui::pos2(700.0, 500.0));
        h.run();
        h.run();
        assert!(!h.state().0.shell.dropped);
        // A double-click on a tab brings the ribbon back.
        let tab = h.get_by_label("Home tab").rect().center();
        h.hover_at(tab);
        h.run();
        // (Both clicks in one frame: the test window's frames are too
        // far apart in time for two of them to be a double-click.)
        for pressed in [true, false, true, false] {
            h.input_mut().events.push(egui::Event::PointerButton {
                pos: tab,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
        assert!(!h.state().0.ribbon_collapsed());
        assert_eq!(viewport_top(&h), open);
        let _ = h.get_by_label("Collapse the ribbon");
    }

    #[test]
    fn a_create_button_makes_the_usual_kind_and_its_arrow_or_a_right_click_lists_the_rest() {
        let mut h = harness(fresh());
        h.run();
        assert!(h.query_by_label("Spotlight").is_none(), "the list is shut");
        // A right-click opens the list; nothing is made.
        let before = h.state().0.document().area().len();
        h.get_by_label("Add point light").click_secondary();
        h.run();
        assert_eq!(h.state().0.document().area().len(), before);
        h.get_by_label("Spotlight").click();
        h.run();
        let w = &h.state().0;
        let made = w.document().area().get(w.selection().unwrap()).unwrap();
        assert_eq!(made.name, "Spotlight 1");
        // The small arrow does the same, and an unfinished kind is in
        // the list as one.
        h.get_by_label("Model: more kinds").click();
        h.run();
        // (The button itself is called by the usual kind it makes.)
        assert!(h.query_all_by_label("Import model…").count() >= 2);
        let _ = h.get_by_label("Ready-made shapes");
        h.key_press(egui::Key::Escape);
        h.run();
        // More is a list and nothing else.
        h.get_by_label("More").click();
        h.run();
        h.get_by_label("Reverb zone").click();
        h.run();
        let w = &h.state().0;
        assert!(
            w.document()
                .area()
                .get(w.selection().unwrap())
                .unwrap()
                .has("ReverbZone")
        );
    }

    #[test]
    fn the_play_group_follows_the_game() {
        let mut h = harness(fresh());
        h.run();
        h.get_by_label("Play").click();
        h.run();
        assert_eq!(h.state().0.play_state(), crate::PlayState::Playing);
        assert!(h.query_by_label("Play").is_none(), "Play became Stop");
        // Step can't be pressed until the game is paused.
        let step = h.get_by_label("Step one tick");
        assert!(step.accesskit_node().is_disabled());
        h.get_by_label("Pause").click();
        h.run();
        assert_eq!(h.state().0.play_state(), crate::PlayState::Paused);
        assert!(
            !h.get_by_label("Step one tick")
                .accesskit_node()
                .is_disabled()
        );
        h.get_by_label("Step one tick").click();
        h.run();
        h.get_by_label("Stop").click();
        h.run();
        assert_eq!(h.state().0.play_state(), crate::PlayState::Editing);
        let _ = h.get_by_label("Play");
        // The group is on every tab that has a ribbon.
        for tab in [RibbonTab::Library, RibbonTab::Animate, RibbonTab::Ui] {
            ribbon_tab(&mut h, tab);
            let _ = h.get_by_label("Play");
        }
    }

    #[test]
    fn the_file_page_stands_in_place_of_the_ribbon_and_the_editor() {
        let mut h = harness(fresh());
        h.run();
        h.get_by_label("File tab").click();
        h.run();
        h.run();
        assert_eq!(h.state().0.ribbon_tab(), RibbonTab::File);
        assert!(h.query_by_label("Add entity").is_none(), "no ribbon");
        assert!(h.state().0.panels_shown().is_empty(), "no panels");
        // The column is 220 wide, its items 36 tall, in the design's order.
        // With no project open, Back to editor says what it does then.
        let back = if h.query_all_by_label("Back to editor").next().is_some() {
            "Back to editor"
        } else {
            "Carry on without a project"
        };
        let items = [
            back,
            "Recent projects",
            "New project",
            "Open…",
            "Save all",
            "Project settings",
            "Preferences",
            "Plugins",
            "Learn",
            "Updates",
            "About",
            "Exit",
        ];
        let leftmost = |label: &str| {
            h.get_all_by_label(label)
                .map(|n| n.rect())
                .min_by(|a, b| a.left().total_cmp(&b.left()))
                .unwrap_or_else(|| panic!("{label}"))
        };
        let rects = items.map(leftmost);
        for (label, rect) in items.iter().zip(rects) {
            assert_eq!(rect.height(), size::FILE_MENU_ITEM_H, "{label}");
            assert_eq!(rect.left(), rects[0].left(), "{label}");
            assert_eq!(
                rect.width(),
                size::FILE_MENU_W - 2.0 * size::FILE_MENU_PAD[1],
                "{label}"
            );
        }
        assert!(
            rects
                .windows(2)
                .all(|pair| pair[0].bottom() <= pair[1].top())
        );
        // The area's keys rest while its page isn't on screen.
        let entities = h.state().0.document().area().len();
        h.state_mut().0.open.document.selection.clear();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.key_press(egui::Key::Delete);
        h.run();
        assert_eq!(h.state().0.document().area().len(), entities);
        // Learn and Updates are pages; Esc goes back to the tab before.
        h.get_by_label("Learn").click();
        h.run();
        assert_eq!(h.state().0.file_pages.page, crate::file_pages::Page::Learn);
        h.key_press(egui::Key::Escape);
        h.run();
        assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Home);
        assert!(h.state().0.panels_shown().contains(&Panel::Viewport));
    }
}
