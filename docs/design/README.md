# docs/design

Designs the project lead made themselves, kept in the repository so the code, the tests and the plans all read the same source.

| Folder | What it holds |
| --- | --- |
| [`g-layout/`](g-layout/README.md) | The editor's interface: the specification and the thirteen screens of the "G layout". The source of truth for how the editor looks ([decision 0099](../decisions/0099-the-g-layout.md)). |
| [`tools-1/`](tools-1/README.md) | The approved mock-ups of Tools 1: the brush list, drawing out and editing brushes, extruding, sculpting and painting the ground, and its three looks ([decision 0111](../decisions/0111-tools-1-brushes-and-ground.md)). |
| [`field-hints/`](field-hints/README.md) | The mock-up of field hints and more kinds of settings: sliders, units and tips on a component's own settings, pickers for the project's names with search, ▶ and thumbnails, and dropping a Library file on a field ([decision 0127](../decisions/0127-field-hints-and-pickers.md)). |
| [`tags/`](tags/README.md) | The approved mock-up of tags: Project settings › Tags, an entity's tags as chips under its name with + Tag, and the Level panel showing them ([the game stages](../plans/game-stages.md)). |
| [`triggers/`](triggers/README.md) | The approved mock-up of trigger zones: making and sizing one in 2D and 3D, its card in Properties, and what game code hears ([the game stages](../plans/game-stages.md)). |
| [`ui-rethink/`](ui-rethink/README.md) | The approved mock-ups of the editor rethink: the launcher as its own window, new projects, Home made clear, the Library browser, the bottom panel, layouts with previews, prefabs, 2D projects and one frame for every tab ([decision 0118](../decisions/0118-the-editor-rethink.md)). Where they differ from the G layout, they win. |

A design here is the project lead's own work and is never edited to fit the code: when the two differ, the code is wrong or a decision says why. Mock-ups of parts a design lacks are drawn in its language, shown to the project lead and approved before they are built (CLAUDE.md, "UI is mocked up first"); an approved one is recorded in the plan that builds it.
