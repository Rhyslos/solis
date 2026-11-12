//! HOME TOOLS · Tile map's ribbon, while a tile map is selected and the
//! Map shelf tab is in front (the approved additions, round 3,
//! `01-tile-map-painting-a.png`; the project lead's answer 27): Paint
//! (the five tools), what is in hand (the terrain or the copy, the layer
//! painted, Exact), Show (the map's art or meaning, its values, chunks
//! and walls) and Copy (flip, flip, turn, put away). The Play group
//! follows, as on Home. There is no Undo group: Ctrl + Z works as today.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Control, Group, Shape, big, fact, item, row_list, switch};
use crate::tile_paint::{PaintTool, Turn};
use crate::unbuilt::Entry;
use crate::workbench::Workbench;

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let paint = &w.viewport.tile_overlay.paint;
    let tool = |each: PaintTool| {
        let control = Control::Do {
            label: each.name(),
            command: C::PaintWith(each),
            lit: paint.painting() && paint.tool == Some(each),
        };
        (Shape::Big(each.icon()), vec![Entry::Built(control)])
    };
    vec![
        Group {
            label: Some("Paint"),
            slots: PaintTool::ALL.into_iter().map(tool).collect(),
        },
        in_hand(w),
        show(w),
        group!(
            Some("Copy"),
            [
                big(Icon::FlipHorizontal, "Flip X", C::TurnCopy(Turn::FlipX)),
                big(Icon::FlipVertical, "Flip Y", C::TurnCopy(Turn::FlipY)),
                big(Icon::Turn, "Turn", C::TurnCopy(Turn::Rotate)),
                big(Icon::Close, "Put away", C::PutCopyAway),
            ]
        ),
    ]
}

/// What is in hand: the terrain painted (or the copy held), the layer
/// painted on, and Exact. Three rows leave no room for a label.
fn in_hand(w: &Workbench) -> Group {
    let paint = &w.viewport.tile_overlay.paint;
    let held = match &paint.stamp {
        Some(stamp) => format!("Copy {} × {}", stamp.size[0], stamp.size[1]),
        None => w
            .painted_terrain()
            .unwrap_or_else(|| "No terrain".to_owned()),
    };
    let layer = paint.layers.get(paint.layer).cloned().unwrap_or_default();
    // Top first, as the Map tab lists them.
    let layers = (0..paint.layers.len())
        .rev()
        .map(|i| item(paint.layers[i].clone(), C::PaintOnLayer(i)))
        .collect();
    Group {
        label: None,
        slots: vec![
            fact(held),
            row_list(format!("Layer: {layer}"), layers),
            switch("Exact", C::PaintExact),
        ],
    }
}

/// Show: the map's art or meaning, the values laid over it, its chunks
/// and its walls.
fn show(w: &Workbench) -> Group {
    let meaning = w.viewport.tile_overlay.paint.meaning;
    let names = w.map_value_names();
    let shown: Vec<&str> = names
        .iter()
        .filter(|name| w.open.map_edits.shown_values.contains(name))
        .map(String::as_str)
        .collect();
    let values_words = if shown.is_empty() {
        "Values: none".to_owned()
    } else {
        format!("Values: {}", shown.join(", "))
    };
    let values = names
        .iter()
        .enumerate()
        .map(|(i, name)| item(name.clone(), C::ShowValue(i)))
        .collect();
    Group {
        label: Some("Show"),
        slots: vec![
            row_list(
                format!("Map as: {}", if meaning { "Meaning" } else { "Art" }),
                vec![
                    item("Art", C::MapMeaning(false)),
                    item("Meaning", C::MapMeaning(true)),
                ],
            ),
            row_list(values_words, values),
            switch("Chunks", C::MapChunks),
            switch("Walls", C::MapWalls),
        ],
    }
}
