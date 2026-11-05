# Plan: what Solis makes is Lust (decision 0081)

The second step of "usable first" (decision 0070): every file Solis makes becomes a Lust data file, `<name>.<kind>.lst`, except `project.solis`. One pull request per step; the project lead chose the order on 5 November: the base and the farm game's files first, then the 2D camera and the UI, then the rest.

| Step | What moves | Status |
| --- | --- | --- |
| L1 | The base (`reflect::data_files`, `area::lust_data`, classes in `solis/data/`, moving older files when a project opens), tile maps (`.tilemap.lst`) and tilesets (`.tileset.lst`) | This pull request |
| — | The 2D camera, then game UI and the UI editor (its screens Lust from the start) | Next |
| L2 | Animators (`.animator.lst`) and sprites' clips (`.clips.lst`) | Later |
| L3 | Sound events (`.sound.lst`), the Mixer (`.mixer.lst`), 2D settings (`.pixels.lst`), input actions (`.input.lst`) | Later |
| L4 | Areas (`.area.lst`) and assemblies (`.assembly.lst`), with the scale check: an area of 10,000 entities must still open quickly | Later |
| L5 | Sidecars (`<file>.meta.lst`) | Later |
| L6 | Save games (`.save.lst`) and recordings (`.recording.lst`), with the scale check | Later |

Each step: a Lust shape and its classes, the reader (the older layout still read), the writer, an example file of the new version in `tests/fixtures/`, the format added to `reflect::data_files::KINDS` and to `app::to_lust`, its fuzz target reading the new layout, the manual and the changelog.
