# docs/design/tags

The approved mock-up of tags and questions about the world (the game stages, stage 1's third milestone; [the plan](../../plans/game-stages.md#the-next-milestone-tags-and-questions-about-the-world)). The project lead approved it on 2026-11-18, answering also that:
- tags set on a prefab are on every copy, and a copy may add or take one away, shown with the usual changed dot and ↶ (a copy's own change wins);
- in a 2D project, distance is measured across the ground, whatever the layers.

| File | Shows |
| --- | --- |
| `01-project-settings-tags.png` | Project settings › Tags, a tab of its own: each tag with "Used by N entities and M prefabs" and its ⋯ menu, a new tag being typed ("New: press Enter to keep it"), **+ New tag**, a double-click on a name to rename it everywhere, ⋯ › Delete asking first if anything uses it, and a card saying how game code asks for tags |
| `02-entity-tags.png` | An entity's tags under its name in Properties, as chips with ×, and **+ Tag ▾** open: a box to search, the project's tags, and New tag…; the Level panel showing the tags faintly after the name, and its search finding them |
