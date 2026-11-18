# editor/src/zones

Trigger zones in the editor (stage 1, triggers; [the plan](../../../../docs/plans/game-stages.md#the-next-milestone-triggers); the approved mock-up [`docs/design/triggers/`](../../../../docs/design/triggers/README.md)). `mod.rs` holds `ViewZone` (a zone the `LevelView` outlines, where it is), `ZoneTool` (kept by the viewport as `Viewport::zones`: the zones picked, the one picked alone, a handle's drag, a freeform outline being drawn or edited, this frame's `ZoneEvent`s and a note for the status bar), a zone's footprint on the ground and its lines in 3D, the label beside it ("Door zone · 2 × 1 m"), and `zone_at`, which picks a zone by its outline (zones have no surface for the renderer to find). The view never changes the level: the workbench writes what the tools report (`../workbench_zones.rs`), one undo step a drag.

| File | What it does |
| --- | --- |
| `draw.rs` | Every zone over a view: a teal outline (`theme::TRIGGER_ZONE`), solid with a faint fill and handles when picked, dashed when not; the label above it; and a freeform outline's points while drawn or edited (each point, its edges, and while drawing a faint line to the pointer). |
| `sizing.rs` | The handles (eight on a box and four on a circle in 2D; one a face of a box and six on a sphere in 3D) and `SizeDrag`: a box's side or corner follows the pointer and its far side stays (its middle moves), a round zone's handle sets its radius; in a 2D view on the zone's ground, in 3D along the handle's own line. |
| `points.rs` | `PointsEdit`, a freeform outline drawn (each click a point; the first point or `Enter` closes it; a point that would make it cross itself is refused) or edited (drag a point, `Ctrl + click` an edge to add one, `Delete` takes the picked one away, never below three, never crossing). |
| `card.rs` | The Trigger zone card in Properties (Shape, Size or Radius or the points and Edit points, Notices, Fires, Only tags greyed until tags exist, Inside now) and the card saying what game code hears, with New code for this zone. |
| `tests.rs` | Footprints, 3D lines, labels, handles, and a handle's drag sizing a box (its far side kept) and a circle, in 2D and 3D. |
