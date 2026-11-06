# crates/ui/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `screen.rs` | What a screen is made of: elements, anchors, fills, the scale rule. | `Screen`, `Element`, `ElementKind`, `Anchor`, `Edge`, `Fill`, `Colour`, `ScaleRule` |
| `layout.rs` | A screen laid out on a real screen into shapes. Tests in `layout_tests.rs`. | `layout`, `layout_with`, `layout_at`, `boxes`, `scale_of`, `Shape` |
| `text.rs` | Fonts, text laid out letter by letter, and the letters' distance images in one atlas (decision 0084). Tests in `text_tests.rs`. | `Fonts`, `Atlas`, `Placed`, `SDF_SIZE`, `SPREAD` |
| `view.rs` | A screen written in Lust (decision 0087): the tree Lust's view gives, laid out with Lust's own rules (column, row, stack, space, text, button, image, progress, canvas with box, circle and line) into elements in design pixels. Tests in `view_tests.rs`. | `Node`, `Style`, `Measure`, `WithPictures`, `lay_out`, `boxes`, `design_of`, `colour` |
| `pictures.rs` | The pictures a game's screens show, packed into one atlas, each with its edge copied round it, colours premultiplied (decision 0089); cutting a crop out of a picture. Tests in the file. | `Pictures`, `Placed`, `split`, `cut`, `SIDE` |
| `source.rs` | A screen's Lust file as the UI editor reads and changes it (decision 0089): each view's pieces with where they are in the text, and edits that change the fewest letters (a `[ ]` item set or taken away, words, a piece taken out, put in or moved), keeping comments and code as written. Tests and a fuzz test in `source_tests.rs`. | `parse`, `ScreenSource`, `Block`, `Piece`, `ElementSource`, `set_style`, `remove_styles`, `set_words`, `remove_piece`, `insert_piece`, `move_piece` |
| `presets.rs` | The screen sizes the UI editor previews. | `PRESETS`, `custom` |
| `sample.rs` | The mock-up's sample HUD. | `hud` |
