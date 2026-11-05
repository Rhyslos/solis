# crates/ui/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `screen.rs` | What a screen is made of: elements, anchors, fills, the scale rule. | `Screen`, `Element`, `ElementKind`, `Anchor`, `Edge`, `Fill`, `Colour`, `ScaleRule` |
| `layout.rs` | A screen laid out on a real screen into shapes. Tests in `layout_tests.rs`. | `layout`, `layout_at`, `scale_of`, `Shape` |
| `presets.rs` | The screen sizes the UI editor previews. | `PRESETS`, `custom` |
| `sample.rs` | The mock-up's sample HUD. | `hud` |
