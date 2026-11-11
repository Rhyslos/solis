# editor/assets

Files built into the editor program itself.

| Folder | What it holds |
| --- | --- |
| `fonts/` | The editor's fonts, built in by `src/fonts.rs`: `Inter-Regular.ttf` and `Inter-SemiBold.ttf` (interface text), and `JetBrainsMonoNL-Regular.ttf` and `JetBrainsMonoNL-Bold.ttf` (monospace text: the console, numbers, keys; bold for the axis letters in number fields). The design also uses Inter at two more weights, **`Inter-Medium.ttf`** and **`Inter-Bold.ttf`**, which are not here yet: until they are, text in those weights is drawn in Regular and SemiBold (`src/fonts.rs` has one line for each to fill in, and a test that lists what stands in). They must be the files of Inter's own release, version 4.001, unchanged, like the two that are here. The JetBrains Mono files are the family's own, version 2.242, from its `jetbrains-mono` package, where they come packed for the web; they were unpacked to plain font files with nothing else changed. Their licences are in [`licenses/fonts`](../../../licenses/README.md). `fonts/` has no README of its own: it holds only the font files, shipped as they are. |

Anything added here needs its licence in `licenses/` (CONTRIBUTING.md, "Third-party material").
