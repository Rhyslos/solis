use super::*;
use crate::shell::workspaces::{Arrangement, SPLIT_LIMIT};

fn rearranged() -> Shell {
    let mut shell = Shell {
        tab: RibbonTab::Library,
        collapsed: true,
        ..Shell::default()
    };
    let home = shell.arrangement_mut(RibbonTab::Home);
    home.shelf_tab = ShelfTab::Console;
    home.shelf_folded = true;
    let sizes = &mut shell.sizes;
    sizes.set(
        RibbonTab::Home,
        PanelLayout::Shelf,
        Edge::ShelfHeight,
        420.0,
    );
    sizes.set(
        RibbonTab::Home,
        PanelLayout::Shelf,
        Edge::Column(ShelfTab::Entity, 2),
        350.0,
    );
    sizes.set(RibbonTab::Home, SIDE, Edge::ShelfHeight, 210.0);
    sizes.set(RibbonTab::Home, SIDE, Edge::Left, 333.0);
    sizes.set(RibbonTab::Home, SIDE, Edge::Strip, 250.0);
    sizes.set(
        RibbonTab::Home,
        PanelLayout::AreaLeftAssetsRight,
        Edge::RightShare,
        0.4,
    );
    sizes.set(RibbonTab::Debug, PanelLayout::Shelf, Edge::Split, 0.3);
    sizes.set(
        RibbonTab::Ui,
        PanelLayout::Shelf,
        Edge::Column(ShelfTab::Screen, 0),
        300.0,
    );
    shell
}

/// The standard layout.
const SIDE: PanelLayout = PanelLayout::AreaLeftAssetsBottom;

#[test]
fn a_frame_reads_back_as_it_was_saved() {
    let shell = rearranged();
    let text = ron::to_string(&shell.saved()).unwrap();
    let back: ShellState = ron::from_str(&text).unwrap();
    let restored = Shell::restored(&back);
    assert_eq!(restored.tab, RibbonTab::Library);
    assert!(restored.collapsed);
    assert_eq!(
        restored.arrangement(RibbonTab::Home),
        Arrangement {
            shelf_tab: ShelfTab::Console,
            shelf_folded: true,
            front: [None, None],
            alone: None,
            merged: false,
        }
    );
    // Every size, in every layout, as it was.
    assert_eq!(restored.sizes, shell.sizes);
    assert_eq!(restored.size(RibbonTab::Debug, SIDE, Edge::Split), 0.3);
    assert_eq!(restored.size(RibbonTab::Home, SIDE, Edge::Left), 333.0);
    assert_eq!(
        restored.size(RibbonTab::Home, PanelLayout::Shelf, Edge::Left),
        Edge::Left.standard(RibbonTab::Home, PanelLayout::Shelf),
        "each layout its own"
    );
    // A tab never changed is its standard.
    assert_eq!(
        restored.arrangement(RibbonTab::Audio),
        Arrangement::standard(RibbonTab::Audio)
    );
}

#[test]
fn nothing_saved_is_the_standard() {
    let restored = Shell::restored(&ShellState::default());
    assert_eq!(restored.tab, RibbonTab::Home);
    assert!(!restored.collapsed);
    for tab in RibbonTab::ALL {
        assert_eq!(restored.arrangement(tab), Arrangement::standard(tab));
    }
    // Missing fields are their standard too.
    let back: ShellState = ron::from_str("(tab: \"Debug\")").unwrap();
    assert_eq!(Shell::restored(&back).tab, RibbonTab::Debug);
    let back: WorkspaceState = ron::from_str("(split: 0.7)").unwrap();
    assert_eq!(back.shelf_height, crate::tokens::size::SHELF_H);
    assert!(back.sizes.is_empty());
}

#[test]
fn a_frame_saved_before_the_layouts_reads_its_height_and_split() {
    // As a build before decision 0109 wrote it: no sizes.
    let text = "(tab: \"Audio\", ribbon_collapsed: false, workspaces: {\
                \"Home\": (shelf_tab: \"Entity\", shelf_height: 380.0, shelf_folded: false, split: 0.5),\
                \"Audio\": (shelf_tab: \"Console\", shelf_height: 300.0, shelf_folded: true, split: 0.7)})";
    let restored = Shell::restored(&ron::from_str(text).unwrap());
    let shelf = PanelLayout::Shelf;
    assert_eq!(
        restored.size(RibbonTab::Home, shelf, Edge::ShelfHeight),
        380.0
    );
    assert_eq!(restored.size(RibbonTab::Audio, SIDE, Edge::Split), 0.7);
    // The side layouts start at their own standard.
    assert_eq!(
        restored.size(RibbonTab::Home, SIDE, Edge::ShelfHeight),
        crate::tokens::size::SHELF_H_SIDE
    );
    // And what this build writes, an earlier one still reads.
    let saved = ron::to_string(&rearranged().saved()).unwrap();
    assert!(
        saved.contains("shelf_height:420.0") || saved.contains("shelf_height: 420.0"),
        "{saved}"
    );
}

#[test]
fn what_cant_be_used_is_repaired() {
    let mut saved = rearranged().saved();
    saved.tab = "Shading".into();
    let home = saved.workspaces.get_mut("Home").unwrap();
    home.shelf_tab = "Object".into();
    home.shelf_height = f32::NAN;
    home.split = 0.99;
    let side = home.sizes.entry(SIDE.key().into()).or_default();
    side.insert("left".into(), -4.0);
    side.insert("strip".into(), f32::INFINITY);
    side.insert("middle".into(), 300.0);
    home.sizes
        .entry("diagonal".into())
        .or_default()
        .insert("left".into(), 300.0);
    saved
        .workspaces
        .insert("Sequencer".into(), WorkspaceState::default());
    let restored = Shell::restored(&saved);
    // An unbuilt screen isn't opened on.
    assert_eq!(restored.tab, RibbonTab::Home);
    let home = restored.arrangement(RibbonTab::Home);
    let standard = Arrangement::standard(RibbonTab::Home);
    let shelf = PanelLayout::Shelf;
    let size = |edge| restored.size(RibbonTab::Home, shelf, edge);
    assert_eq!(home.shelf_tab, standard.shelf_tab);
    assert_eq!(size(Edge::ShelfHeight), crate::tokens::size::SHELF_H);
    assert_eq!(size(Edge::Split), crate::tokens::size::SPLIT);
    assert!(home.shelf_folded, "what could be read is kept");
    // Damaged side sizes are their standard; what could be read is kept.
    let side = |edge| restored.size(RibbonTab::Home, SIDE, edge);
    assert_eq!(side(Edge::Left), Edge::Left.standard(RibbonTab::Home, SIDE));
    assert_eq!(side(Edge::Strip), crate::tokens::size::ASSETS_STRIP_H);
    assert_eq!(side(Edge::ShelfHeight), 210.0);
    for height in [0.0, -5.0, 1.0e9, f32::INFINITY] {
        let state = WorkspaceState {
            shelf_height: height,
            ..WorkspaceState::default()
        };
        let mut sizes = Sizes::default();
        state.restore(RibbonTab::Home, &mut sizes);
        assert_eq!(
            sizes.get(RibbonTab::Home, shelf, Edge::ShelfHeight),
            crate::tokens::size::SHELF_H
        );
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

/// Every kind of border.
const EDGES: [Edge; 9] = [
    Edge::ShelfHeight,
    Edge::Split,
    Edge::Left,
    Edge::Right,
    Edge::Strip,
    Edge::LeftShare,
    Edge::RightShare,
    Edge::Column(ShelfTab::Entity, 0),
    Edge::Column(ShelfTab::Screen, 2),
];

/// Random damage to a saved frame, thousands of times: reading it never
/// crashes, and what comes back is always usable.
#[test]
fn random_damage_to_a_saved_frame_never_gives_an_unusable_one() {
    let saved = ron::to_string(&rearranged().saved()).unwrap();
    let bytes = saved.as_bytes();
    let words = [
        "Home",
        "Console",
        "Entity",
        "Object",
        "Debug",
        "true",
        "left",
        "strip",
        "shelf",
        "area left assets bottom",
        "column Entity 0",
    ];
    let mut seed: u64 = 7;
    let mut next = |n: usize| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) as usize % n.max(1)
    };
    let mut read_back = 0;
    for _ in 0..3_000 {
        let mut damaged = bytes.to_vec();
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
                for edge in EDGES {
                    let value = shell.size(tab, layout, edge);
                    assert!(edge.limits().contains(&value), "{edge:?} {value} {text}");
                }
            }
            assert!(
                (SPLIT_LIMIT..=1.0 - SPLIT_LIMIT).contains(&shell.size(tab, SIDE, Edge::Split)),
                "{text}"
            );
        }
    }
    assert!(read_back > 0);
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
