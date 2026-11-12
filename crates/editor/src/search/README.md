# editor/src/search

The parts of the one search for everything ([decision 0085](../../../../docs/decisions/0085-the-editor-overhaul.md)). The module's own file is [`../search.rs`](../search.rs): what a result is, the search's state, opening it (the box in the tab bar, or Space) and doing a result. Its tests are in [`../search_tests.rs`](../search_tests.rs).

| File | What it does |
| --- | --- |
| `list.rs` | What can be found. The editor's commands come from the one table, `shell/commands.rs` (one that can't be done now is left out); then the panels, the area's entities and the project's files. The entities and files are read once and kept until the area (`Document::revision`) or the project's files change, and scored once for what's typed (`search_results`), so an open search doesn't walk the whole area every frame. |
| `score.rs` | How well a name matches what is typed (a word's start, inside a word, letters in order), and which results are shown, best first. |
| `draw.rs` | The search while it is open: the field, the list under it, and the keys that choose and do a result. |
