# editor/assets

Files built into the editor program itself.

| Folder | What it holds |
| --- | --- |
| `fonts/` | The editor's fonts, built in by `src/fonts.rs`, in every weight the design uses: Inter for interface text (`Inter-Regular.ttf`, `Inter-Medium.ttf`, `Inter-SemiBold.ttf`, `Inter-Bold.ttf`: weights 400, 500, 600 and 700) and JetBrains Mono for monospace text, the console, numbers and keys (`JetBrainsMonoNL-Regular.ttf`, and `JetBrainsMonoNL-Bold.ttf` for the axis letters in number fields). The Inter files are those of Inter's own release, version 4.001 (its 4.1 archive), unchanged. The JetBrains Mono files are the family's own, version 2.242, from its `jetbrains-mono` package, where they come packed for the web; they were unpacked to plain font files with nothing else changed. Their licences are in [`licenses/fonts`](../../../licenses/README.md). `fonts/` has no README of its own: it holds only the font files, shipped as they are. |

Anything added here needs its licence in `licenses/` (CONTRIBUTING.md, "Third-party material").
