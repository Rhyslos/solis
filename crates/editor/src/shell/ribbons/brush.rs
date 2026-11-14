//! HOME TOOLS · Brush's ribbon, while a brush's insides are edited
//! (Tools 1a, B3; the approved mock-up `docs/design/tools-1/
//! 03-brush-edit-faces.png`): Edit (Points 1, Edges 2, Faces 3, and Done
//! Tab), Shape (Extrude E, Inset, Split and Delete, B4: each greyed with
//! why when the mode or the pick doesn't allow it),
//! Brush (Material, which comes with B5, and Solid) and Snapping. The
//! Play group follows, as on Home.

use super::group;
use crate::brushing::edit::Mode;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Control, Group, Shape, switch, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};
use crate::workbench::Workbench;

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let now = w.viewport.brushing.edit.as_ref().map(|e| e.mode);
    let mode = |each: Mode| {
        let control = Control::Do {
            label: each.name(),
            command: C::BrushMode(each),
            lit: now == Some(each),
            keyed: true,
        };
        (Shape::Big(each.icon()), vec![Entry::Built(control)])
    };
    let done = Control::Do {
        label: "Done",
        command: C::EditBrushInsides,
        lit: false,
        keyed: true,
    };
    let mut edit: Vec<_> = Mode::ALL.into_iter().map(mode).collect();
    edit.push((Shape::Big(Icon::Check), vec![Entry::Built(done)]));
    vec![
        Group {
            label: Some("Edit"),
            slots: edit,
        },
        Group {
            label: Some("Shape"),
            slots: shape(w),
        },
        group!(
            Some("Brush"),
            [
                unbuilt_at(
                    Shape::Row(RowKind::List),
                    Entry::Unbuilt(UnbuiltId::BrushRibbonMaterial),
                ),
                switch("Solid", C::BrushSolid),
            ]
        ),
        snapping(w),
    ]
}

/// Extrude (its key small beside it, lit while extruding), Inset (lit
/// while insetting), Split and Delete.
fn shape(w: &Workbench) -> Vec<crate::shell::ribbon::Slot> {
    use crate::brushing::pull::PullKind;
    let pulling = w
        .viewport
        .brushing
        .edit
        .as_ref()
        .and_then(|e| e.pull.as_ref())
        .map(|p| p.kind);
    let button = |icon, label, command, lit, keyed| {
        let control = Control::Do {
            label,
            command,
            lit,
            keyed,
        };
        (Shape::Big(icon), vec![Entry::Built(control)])
    };
    let extruding = matches!(pulling, Some(PullKind::Extrude | PullKind::ExtrudeEdges));
    vec![
        button(Icon::ArrowUp, "Extrude", C::BrushExtrude, extruding, true),
        button(
            Icon::FourSquares,
            "Inset",
            C::BrushInset,
            pulling == Some(PullKind::Inset),
            false,
        ),
        button(Icon::Grid, "Split", C::BrushSplit, false, false),
        button(Icon::Trash, "Delete", C::BrushDelete, false, false),
    ]
}

/// Snapping, as the mock-up draws it: the grid and angle steps under
/// their label (scaling snaps as Home's Scale · 10% says).
fn snapping(w: &Workbench) -> Group {
    let mut slots = super::home::snapping(w, false).slots;
    slots.truncate(2);
    Group {
        label: Some("Snapping"),
        slots,
    }
}
