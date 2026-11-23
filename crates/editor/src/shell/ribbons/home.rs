//! Home's ribbon, tidied as board 03 of decision 0118 draws it: Edit,
//! Create, the grid snaps and Lighting, each labelled under it but the
//! snaps; the Play group follows on every tab. Save and Build left it:
//! Save is `Ctrl + S`, the File page, the level's tab and the status
//! line; Build is the Build tab. While the Library is placing, the build
//! form (spec 240-249; round 2, `06-placing-a.png`): Edit, Place,
//! Snapping with Snap to surface, Paint, Transform (the tool bar's tools,
//! as there is no tool bar then).

use super::group;
use crate::icons::Icon;
use crate::launcher::ProjectKind;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{
    Control, Group, Shape, Steps, big, big_keyed, big_list, big_list_only, fact, item, row, switch,
    switch_with_steps, unbuilt_at,
};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};
use crate::viewport::Tool;
use crate::workbench::Workbench;

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    if w.placing() {
        return build_form(w);
    }
    // A 2D project's Home is laid out as board 08 draws it; what it
    // hides is the kind table's (`kinds.rs`), as on every ribbon.
    if w.open.kind == ProjectKind::TwoD {
        return vec![edit(), create_2d(), paint_tiles(), snapping_2d(w)];
    }
    vec![
        edit(),
        create(w, "Create"),
        snapping(w, false),
        group!(
            Some("Lighting"),
            [
                unbuilt_at(
                    Shape::Big(Icon::Sun),
                    Entry::Unbuilt(UnbuiltId::HomeLightingBake),
                ),
                unbuilt_at(
                    Shape::Row(RowKind::List),
                    Entry::Unbuilt(UnbuiltId::HomeLightingQuality),
                ),
                unbuilt_at(
                    Shape::Row(RowKind::Plain),
                    Entry::Unbuilt(UnbuiltId::HomeLightingLightmapView),
                ),
            ]
        ),
    ]
}

/// The build form, while placing.
fn build_form(w: &Workbench) -> Vec<Group> {
    let tool = |icon, label, tool| {
        let control = Control::Do {
            label,
            command: C::UseTool(tool),
            lit: w.viewport.tool == tool,
            keyed: false,
        };
        (Shape::Big(icon), vec![Entry::Built(control)])
    };
    vec![
        edit(),
        create(w, "Place"),
        snapping(w, true),
        group!(
            Some("Paint"),
            [
                unbuilt_at(
                    Shape::Big(Icon::Brush),
                    Entry::Unbuilt(UnbuiltId::PlacePaintBrush),
                ),
                unbuilt_at(
                    Shape::Big(Icon::Module),
                    Entry::Unbuilt(UnbuiltId::PlacePaintScatter),
                ),
            ]
        ),
        group!(
            Some("Transform"),
            [
                tool(Icon::Select, "Select", Tool::Select),
                tool(Icon::Move, "Move", Tool::Move),
                tool(Icon::Rotate, "Rotate", Tool::Rotate),
                tool(Icon::Scale, "Scale", Tool::Scale),
                tool(Icon::Transform, "All three", Tool::Transform),
            ]
        ),
    ]
}

/// Edit: Undo and Redo, as two small rows (board 03).
fn edit() -> Group {
    group!(Some("Edit"), [row("Undo", C::Undo), row("Redo", C::Redo)])
}

/// The things to make, under `label` (Create, or Place while placing):
/// Entity, Model, Light, Camera and Shape, and the rest under More (board
/// 03). Shape opens the brush list, and is lit while it is open or a
/// brush is being drawn out (Tools 1a, B2).
fn create(w: &Workbench, label: &'static str) -> Group {
    group!(
        Some(label),
        [
            big(Icon::Empty, "Entity", C::AddEntity),
            big_list(
                Icon::Model,
                "Model",
                C::ImportModel,
                vec![
                    item("Import model…", C::ImportModel),
                    // Ready-made shapes are shape brushes (Tools 1a).
                    item("Shape brush…", C::ChooseShapeBrush),
                ],
            ),
            big_list(
                Icon::Light,
                "Light",
                C::AddPointLight,
                vec![
                    item("Point light", C::AddPointLight),
                    item("Spotlight", C::AddSpotlight),
                ],
            ),
            big(Icon::Camera, "Camera", C::AddCamera),
            big_keyed(
                Icon::Shape,
                "Shape",
                C::ChooseShapeBrush,
                w.viewport.brushing.is_active(),
            ),
            big_list_only(
                Icon::More,
                "More",
                vec![
                    item("Sprite", C::AddSprite),
                    item("Tile map", C::NewTileMap),
                    item("Reverb zone", C::AddReverbZone),
                    item("Trigger zone", C::AddTriggerZone),
                    item("Make module\u{2026}", C::MakeModule),
                ],
            ),
        ]
    )
}

/// Create in a 2D project (board 08): Entity, Sprite, Tile map, Camera,
/// Trigger zone (the board's Zone; stage 1), and More with the rest, a
/// lamp (a point light, which lights sprites) first. Models and shapes
/// are 3D only.
fn create_2d() -> Group {
    group!(
        Some("Create"),
        [
            big(Icon::Empty, "Entity", C::AddEntity),
            big(Icon::Sprite, "Sprite", C::AddSprite),
            big(Icon::TileMap, "Tile map", C::NewTileMap),
            big(Icon::Camera, "Camera", C::AddCamera),
            big(Icon::Volume, "Trigger zone", C::AddTriggerZone),
            big_list_only(
                Icon::More,
                "More",
                vec![
                    item("Point light", C::AddPointLight),
                    item("Reverb zone", C::AddReverbZone),
                    item("Make module\u{2026}", C::MakeModule),
                ],
            ),
        ]
    )
}

/// The tile painter's tools on a 2D project's Home (board 08's Paint
/// tiles): Brush (its right button empties cells, the board's Erase),
/// Fill and the Eyedropper (the board's Pick). They work on the tile map
/// selected; Rectangle and Line are on its own ribbon and the tool bar.
fn paint_tiles() -> Group {
    use crate::tile_paint::PaintTool;
    let tool = |tool: PaintTool| big(tool.icon(), tool.name(), C::PaintWith(tool));
    group!(
        Some("Paint tiles"),
        [
            tool(PaintTool::Brush),
            tool(PaintTool::Fill),
            tool(PaintTool::Eyedropper),
        ]
    )
}

/// A 2D project's snapping (board 08): the tile grid's size, and what
/// moves in the 2D view land on, whole tiles or whole pixels.
fn snapping_2d(w: &Workbench) -> Group {
    let [x, y] = w.open.pixels.tile;
    let grid = if x == y {
        format!("Grid \u{b7} {x} px")
    } else {
        format!("Grid \u{b7} {x} \u{d7} {y} px")
    };
    Group {
        label: None,
        slots: vec![
            fact(grid),
            switch("Snap to tiles", C::SnapToTiles),
            switch("Pixel perfect", C::PixelPerfect),
        ],
    }
}

/// Snapping's three rows, and while placing Snap to surface beside them.
pub(super) fn snapping(w: &Workbench, placing: bool) -> Group {
    let snapping = w.viewport.snapping;
    let mut slots = vec![
        switch_with_steps(
            format!("Grid · {}", snapping.grid_step),
            C::SnapToGrid,
            Steps::Grid,
        ),
        switch_with_steps(
            format!("Angle · {}°", snapping.angle_step),
            C::SnapAngles,
            Steps::Angle,
        ),
        switch("Scale · 10%", C::SnapScale),
    ];
    if placing {
        slots.push(unbuilt_at(
            Shape::Row(RowKind::Check),
            Entry::Unbuilt(UnbuiltId::PlaceSnapSurface),
        ));
    }
    // Three rows leave no room for a label.
    Group { label: None, slots }
}
