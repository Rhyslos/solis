use super::*;
use crate::shell::workspaces::SPLIT_LIMIT;

/// The standard layout, and one with the Library beside Level.
const STANDARD: PanelLayout = PanelLayout::Standard;
const LIBRARY_LEFT: PanelLayout = PanelLayout::LibraryLeft;

fn rearranged() -> Shell {
    let mut shell = Shell {
        tab: RibbonTab::Library,
        collapsed: true,
        ..Shell::default()
    };
    let home = shell.arrangement_mut(RibbonTab::Home);
    home.bottom_tab = BottomTab::Console;
    home.bottom_folded = true;
    home.hidden.set(Slot::Right, true);
    let sizes = &mut shell.sizes;
    sizes.set(RibbonTab::Home, STANDARD, Edge::Left, 0.25);
    sizes.set(RibbonTab::Home, STANDARD, Edge::Bottom, 0.4);
    sizes.set(RibbonTab::Home, LIBRARY_LEFT, Edge::LeftShare, 0.6);
    sizes.set(RibbonTab::Home, STANDARD, Edge::Placement, 300.0);
    sizes.set(RibbonTab::Debug, STANDARD, Edge::Split, 0.3);
    sizes.set(RibbonTab::Ui, PanelLayout::WideView, Edge::Right, 0.3);
    shell
}

#[test]
fn a_frame_reads_back_as_it_was_saved() {
    let shell = rearranged();
    let text = ron::to_string(&shell.saved()).unwrap();
    let back: ShellState = ron::from_str(&text).unwrap();
    let restored = Shell::restored(&back);
    assert_eq!(restored.tab, RibbonTab::Library);
    assert!(restored.collapsed);
    let home = restored.arrangement(RibbonTab::Home);
    assert_eq!(home.bottom_tab, BottomTab::Console);
    assert!(home.bottom_folded);
    assert!(home.hidden.has(Slot::Right) && !home.hidden.has(Slot::Left));
    // Every size, in every layout, as it was.
    assert_eq!(restored.sizes, shell.sizes);
    assert_eq!(restored.size(RibbonTab::Home, STANDARD, Edge::Left), 0.25);
    assert_eq!(
        restored.size(RibbonTab::Home, PanelLayout::WideView, Edge::Left),
        Edge::Left.standard(RibbonTab::Home, PanelLayout::WideView),
        "each layout its own"
    );
    // A tab never changed is its standard.
    assert_eq!(
        restored.arrangement(RibbonTab::Audio),
        Arrangement::standard(RibbonTab::Audio)
    );
    // Old builds' keys are kept for the bottom tab and its fold, and
    // the sizes in points of before are never written again.
    assert!(
        text.contains("shelf_tab") && text.contains("shelf_folded"),
        "{text}"
    );
    assert!(
        !text.contains("shelf_height") && !text.contains("split:0.5"),
        "{text}"
    );
}

#[test]
fn a_contextual_tab_s_frame_comes_back_too() {
    let mut shell = Shell {
        tab: RibbonTab::Tileset,
        ..Shell::default()
    };
    let tileset = shell.arrangement_mut(RibbonTab::Tileset);
    tileset.bottom_tab = BottomTab::Console;
    tileset.bottom_folded = true;
    shell.arrangement_mut(RibbonTab::Picture).bottom_tab = BottomTab::Problems;
    let text = ron::to_string(&shell.saved()).unwrap();
    let restored = Shell::restored(&ron::from_str(&text).unwrap());
    let tileset = restored.arrangement(RibbonTab::Tileset);
    assert_eq!(
        (tileset.bottom_tab, tileset.bottom_folded),
        (BottomTab::Console, true)
    );
    assert_eq!(
        restored.arrangement(RibbonTab::Picture).bottom_tab,
        BottomTab::Problems
    );
    assert_eq!(restored.tab, RibbonTab::Library, "its parent in front");
    let named: ShellState = ron::from_str("(tab: \"Tileset\")").unwrap();
    assert_eq!(Shell::restored(&named).tab, RibbonTab::Home);
}

#[test]
fn nothing_saved_is_the_standard() {
    let restored = Shell::restored(&ShellState::default());
    assert_eq!(restored.tab, RibbonTab::Home);
    assert!(!restored.collapsed);
    for tab in RibbonTab::ALL {
        assert_eq!(restored.arrangement(tab), Arrangement::standard(tab));
    }
    assert_eq!(restored.sizes.kept().count(), 0);
    // Missing fields are their standard too.
    let back: ShellState = ron::from_str("(tab: \"Debug\")").unwrap();
    assert_eq!(Shell::restored(&back).tab, RibbonTab::Debug);
    let back: WorkspaceState = ron::from_str("(hidden: [\"left\"])").unwrap();
    assert!(back.sizes.is_empty());
    let mut sizes = Sizes::default();
    let arrangement = back.restore(RibbonTab::Home, &mut sizes);
    assert!(arrangement.hidden.has(Slot::Left));
    assert_eq!(sizes.kept().count(), 0);
}

/// The frame as the rethink's R2 wrote it (the same as
/// `tests/fixtures/editor-settings/before-frame/side-layouts.ron`): the
/// shelf's tabs, heights in points, and decision 0109's side layouts.
const BEFORE_THE_FRAME: &str = "(tab:\"Debug\",ribbon_collapsed:false,workspaces:{\
    \"Animate\":(shelf_tab:\"Console\",shelf_height:360.0,shelf_folded:false,split:0.5,sizes:{}),\
    \"Audio\":(shelf_tab:\"Sound events\",shelf_height:300.0,shelf_folded:false,split:0.6,sizes:{}),\
    \"Debug\":(shelf_tab:\"Problems\",shelf_height:420.0,shelf_folded:true,split:0.5,sizes:{}),\
    \"Home\":(shelf_tab:\"Console\",shelf_height:300.0,shelf_folded:false,split:0.5,sizes:{\
    \"level and assets left\":{\"left\":330.0,\"left share\":0.6,\"shelf\":210.0},\
    \"shelf\":{\"column Entity 0\":260.0,\"column Place 1\":280.0}})})";

#[test]
fn a_frame_saved_before_it_reads_into_its_nearest() {
    let restored = Shell::restored(&ron::from_str(BEFORE_THE_FRAME).unwrap());
    assert_eq!(restored.tab, RibbonTab::Debug);
    let [w, h] = OLD_MIDDLE;
    // Home's side columns: their widths in points become shares of the
    // window they were made in, in the layout each became.
    let left = restored.size(RibbonTab::Home, LIBRARY_LEFT, Edge::Left);
    assert!((left - 330.0 / w).abs() < 1e-6, "{left}");
    assert_eq!(
        restored.size(RibbonTab::Home, LIBRARY_LEFT, Edge::LeftShare),
        0.6
    );
    // The Place tab's Placement column is the placing column now.
    assert_eq!(
        restored.size(RibbonTab::Home, STANDARD, Edge::Placement),
        280.0
    );
    // The bottom tab: the Console as it was; the shelf's Sound events,
    // a slot now, gives the standard.
    let home = restored.arrangement(RibbonTab::Home);
    assert_eq!(home.bottom_tab, BottomTab::Console);
    assert_eq!(
        restored.arrangement(RibbonTab::Audio).bottom_tab,
        Arrangement::standard(RibbonTab::Audio).bottom_tab
    );
    // Debug's shelf, dragged taller and folded, is its bottom panel.
    let debug = restored.arrangement(RibbonTab::Debug);
    assert_eq!(
        (debug.bottom_tab, debug.bottom_folded),
        (BottomTab::Problems, true)
    );
    let bottom = restored.size(RibbonTab::Debug, STANDARD, Edge::Bottom);
    assert!((bottom - 420.0 / h).abs() < 1e-6, "{bottom}");
    // What was the standard then isn't kept: Animate's taller shelf, and
    // every split that wasn't dragged.
    assert_eq!(
        restored.size(RibbonTab::Animate, STANDARD, Edge::Bottom),
        Edge::Bottom.standard(RibbonTab::Animate, STANDARD)
    );
    assert_eq!(
        restored.size(RibbonTab::Home, STANDARD, Edge::Split),
        size::SPLIT
    );
    // And what this build writes of it reads back the same.
    let again =
        Shell::restored(&ron::from_str(&ron::to_string(&restored.saved()).unwrap()).unwrap());
    assert_eq!(again.sizes, restored.sizes);
    assert_eq!(again.arrangements, restored.arrangements);
}

#[test]
fn a_size_saved_since_wins_over_one_from_before() {
    let text = "(workspaces: {\"Home\": (sizes: {\"level left assets bottom\": \
                {\"left\": 400.0, \"left width\": 0.3}, \"shelf\": {\"strip\": 200.0}})})";
    let restored = Shell::restored(&ron::from_str(text).unwrap());
    assert_eq!(restored.size(RibbonTab::Home, STANDARD, Edge::Left), 0.3);
}

#[test]
fn what_cant_be_used_is_repaired() {
    let mut saved = rearranged().saved();
    saved.tab = "Shading".into();
    let home = saved.workspaces.get_mut("Home").unwrap();
    home.bottom_tab = "Object".into();
    home.hidden.push("middle".into());
    let standard = home.sizes.entry(STANDARD.key().into()).or_default();
    standard.insert("left width".into(), -4.0);
    standard.insert("bottom height".into(), f32::INFINITY);
    standard.insert("middle".into(), 0.3);
    home.sizes
        .entry("diagonal".into())
        .or_default()
        .insert("left width".into(), 0.3);
    saved
        .workspaces
        .insert("Sequencer".into(), WorkspaceState::default());
    let restored = Shell::restored(&saved);
    // An unbuilt screen isn't opened on.
    assert_eq!(restored.tab, RibbonTab::Home);
    let home = restored.arrangement(RibbonTab::Home);
    assert_eq!(
        home.bottom_tab,
        Arrangement::standard(RibbonTab::Home).bottom_tab
    );
    assert!(home.bottom_folded, "what could be read is kept");
    assert!(home.hidden.has(Slot::Right) && !home.hidden.has(Slot::Left));
    let size = |edge| restored.size(RibbonTab::Home, STANDARD, edge);
    assert_eq!(
        size(Edge::Left),
        Edge::Left.standard(RibbonTab::Home, STANDARD)
    );
    assert_eq!(
        size(Edge::Bottom),
        Edge::Bottom.standard(RibbonTab::Home, STANDARD)
    );
    assert_eq!(size(Edge::Placement), 300.0);
    // Old sizes that were never usable stay out too.
    for height in [0.0, -5.0, 1.0e9, f32::INFINITY, f32::NAN] {
        let state = WorkspaceState {
            shelf_height: height,
            split: height,
            ..WorkspaceState::default()
        };
        let mut sizes = Sizes::default();
        state.restore(RibbonTab::Debug, &mut sizes);
        assert_eq!(
            sizes.get(RibbonTab::Debug, STANDARD, Edge::Split),
            size::SPLIT,
            "{height}"
        );
        let bottom = sizes.get(RibbonTab::Debug, STANDARD, Edge::Bottom);
        assert!(Edge::Bottom.limits().contains(&bottom), "{height}");
    }
}

#[test]
fn the_file_page_saves_the_tab_before_it() {
    let mut shell = rearranged();
    shell.before_file = RibbonTab::Animate;
    shell.tab = RibbonTab::File;
    assert_eq!(shell.saved().tab, "Animate");
    shell.tab = RibbonTab::Vfx;
    assert_eq!(shell.saved().tab, "Home");
}

/// Random damage to a saved frame, this build's and one from before,
/// thousands of times: reading it never crashes, and what comes back is
/// always usable.
#[test]
fn random_damage_to_a_saved_frame_never_gives_an_unusable_one() {
    let ours = ron::to_string(&rearranged().saved()).unwrap();
    let words = [
        "Home",
        "Console",
        "Library",
        "Place",
        "Object",
        "Debug",
        "true",
        "left",
        "left width",
        "bottom height",
        "strip",
        "shelf",
        "library",
        "level left assets bottom",
        "column Place 1",
    ];
    let mut seed: u64 = 7;
    let mut next = |n: usize| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) as usize % n.max(1)
    };
    let mut read_back = 0;
    for round in 0..4_000 {
        let source = if round % 2 == 0 {
            ours.as_str()
        } else {
            BEFORE_THE_FRAME
        };
        let mut damaged = source.as_bytes().to_vec();
        for _ in 0..1 + next(3) {
            let at = next(damaged.len());
            let cut = next(6).min(damaged.len() - at);
            let with: Vec<u8> = match next(6) {
                0 => words[next(words.len())].as_bytes().to_vec(),
                1 => b"99999".to_vec(),
                2 => b"-1.5".to_vec(),
                3 => b"NaN".to_vec(),
                4 => b")".to_vec(),
                _ => Vec::new(),
            };
            damaged.splice(at..at + cut, with);
        }
        let text = String::from_utf8_lossy(&damaged);
        let parsed = ron::from_str::<ShellState>(&text).ok();
        read_back += usize::from(parsed.is_some());
        let shell = Shell::restored(&parsed.unwrap_or_default());
        assert!(
            shell.tab.is_built() && shell.tab != RibbonTab::File,
            "{text}"
        );
        for tab in RibbonTab::ALL {
            for layout in PanelLayout::ALL {
                for edge in Edge::ALL {
                    let value = shell.size(tab, layout, edge);
                    assert!(edge.limits().contains(&value), "{edge:?} {value} {text}");
                }
            }
            assert!(
                (SPLIT_LIMIT..=1.0 - SPLIT_LIMIT).contains(&shell.size(tab, STANDARD, Edge::Split)),
                "{text}"
            );
        }
    }
    assert!(read_back > 100, "{read_back}");
}

#[test]
fn an_old_arrangement_is_said_once_and_a_saved_frame_is_taken_in() {
    let mut w = Workbench::new(crate::test_support::info());
    let said = |w: &Workbench| {
        w.log_lines()
            .iter()
            .filter(|l| l.text == NOT_CARRIED_OVER)
            .count()
    };
    w.restore_shell(None, false);
    assert_eq!(said(&w), 0, "a first run has nothing to say");
    w.restore_shell(None, true);
    assert_eq!(said(&w), 1);
    w.restore_shell(Some(&rearranged().saved()), true);
    assert_eq!(said(&w), 1, "a frame of this build's says nothing");
    assert_eq!(w.ribbon_tab(), RibbonTab::Library);
}
