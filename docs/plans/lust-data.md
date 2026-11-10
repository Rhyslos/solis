# Plan: what Solis makes is Lust (decision 0081)

The second step of "usable first" (decision 0070): every file Solis makes becomes a Lust data file, `<name>.<kind>.lst`, except `project.solis`. One pull request per step; the project lead chose the order on 5 November: the base and the farm game's files first, then the 2D camera and the UI, then the rest.

| Step | What moves | Status |
| --- | --- | --- |
| L1 | The base (`reflect::data_files`, `area::lust_data`, classes in `solis/data/`, moving older files when a project opens), tile maps (`.tilemap.lst`) and tilesets (`.tileset.lst`) | Built (decision 0081) |
| — | The 2D camera, then game UI and the UI editor (its screens Lust from the start) | Built (the camera: decision 0082; game UI: decisions 0083, 0084 and 0087 to 0089); Atlas Crop's files (`.atlas.lst`) joined the data files with it (decision 0089) |
| L2 | Animators (`.animator.lst`) and sprites' clips (`.clips.lst`) | Not built |
| L3 | Sound events (`.sound.lst`), the Mixer (`.mixer.lst`), 2D settings (`.pixels.lst`), input actions (`.input.lst`) | Not built |
| L4 | Areas (`.area.lst`) and assemblies (`.assembly.lst`), with the scale check: an area of 10,000 entities must still open quickly | Not built |
| L5 | Sidecars (`<file>.meta.lst`) | Not built |
| L6 | Save games (`.save.lst`) and recordings (`.recording.lst`), with the scale check | Not built |

L2 to L6 are what's left of this step; ROADMAP.md's order builds them after Shared 1, the editor's UI remade.

Each step: a Lust shape and its classes, the reader (the older layout still read), the writer, an example file of the new version in `tests/fixtures/`, the format added to `reflect::data_files::KINDS` and to `app::to_lust`, its fuzz target reading the new layout, the manual and the changelog.
