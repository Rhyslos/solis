//! The font table: every family the type styles ask for exists, a
//! bundled weight draws with its own file, a missing one with its
//! stand-in, and every symbol the editor writes can be drawn.

use super::*;
use crate::tokens::text;
use egui::{Color32, Context, FontId};

/// Every family the editor draws text in.
fn families() -> Vec<FontFamily> {
    let mut all = vec![FontFamily::Proportional, FontFamily::Monospace];
    all.extend(
        [MEDIUM, SEMIBOLD, BOLD, MONO_BOLD]
            .into_iter()
            .map(|name| FontFamily::Name(name.into())),
    );
    all
}

fn context() -> Context {
    let ctx = Context::default();
    ctx.set_fonts(fonts());
    let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
    output.textures_delta.clear();
    ctx
}

/// Where in the font image a character's picture is, and how wide the
/// character is: two characters share a picture only if both fall back
/// to the replacement box.
fn glyph(ctx: &Context, c: char, family: &FontFamily) -> Option<([u16; 2], [u16; 2], f32)> {
    let font = FontId::new(13.0, family.clone());
    let galley = ctx.fonts_mut(|f| f.layout_no_wrap(c.to_string(), font, Color32::WHITE));
    let width = galley.size().x;
    galley
        .rows
        .first()
        .and_then(|row| row.row.glyphs.first())
        .map(|glyph| (glyph.uv_rect.min, glyph.uv_rect.max, width))
}

#[test]
fn inter_comes_first_and_every_weight_has_its_family() {
    let fonts = fonts();
    let first = |family: &FontFamily| fonts.families[family].first().cloned();
    assert_eq!(first(&FontFamily::Proportional).as_deref(), Some("inter"));
    assert_eq!(
        first(&FontFamily::Monospace).as_deref(),
        Some("jetbrains-mono")
    );
    for family in families() {
        let list = &fonts.families[&family];
        assert!(list.len() > 1, "{family:?} has no fallback for symbols");
        for font in list {
            assert!(fonts.font_data.contains_key(font), "{family:?}: no {font}");
        }
        let mut unique = list.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), list.len(), "{family:?} lists a font twice");
    }
    // Every type style asks for a family that exists.
    for (name, style) in text::all() {
        assert!(fonts.families.contains_key(&style.family), "{name}");
    }
}

/// The two Inter weights that aren't in `assets/fonts/` yet. When
/// `Inter-Medium.ttf` and `Inter-Bold.ttf` are added (and their two lines
/// in `fonts.rs` filled in), this list becomes empty.
#[test]
fn the_weights_still_to_be_bundled_stand_in_as_listed() {
    assert_eq!(
        standing_in(),
        [(MEDIUM, "inter"), (BOLD, "inter-semibold")],
        "the font files changed: update this list, assets/README.md and licenses/README.md"
    );
    let fonts = fonts();
    for (family, stand_in) in standing_in() {
        let first = fonts.families[&FontFamily::Name(family.into())][0].as_str();
        assert_eq!(first, stand_in, "{family} is drawn as {stand_in} meanwhile");
    }
}

#[test]
fn a_bundled_weight_draws_with_its_own_file() {
    let ctx = context();
    let proportional = FontFamily::Proportional;
    let name = |family: &str| FontFamily::Name(family.into());
    // Semibold letters are wider than regular ones; bold mono letters
    // are as wide (that is what mono means) but are another picture.
    let regular = glyph(&ctx, 'm', &proportional).unwrap();
    let semibold = glyph(&ctx, 'm', &name(SEMIBOLD)).unwrap();
    assert!(semibold.2 > regular.2, "semibold is no wider than regular");
    let mono = glyph(&ctx, 'm', &FontFamily::Monospace).unwrap();
    let mono_bold = glyph(&ctx, 'm', &name(MONO_BOLD)).unwrap();
    assert_eq!(mono.2, mono_bold.2, "both are one cell wide");
    assert_ne!((mono.0, mono.1), (mono_bold.0, mono_bold.1));
    // A weight standing in is, for now, the same picture as its stand-in.
    let medium = glyph(&ctx, 'm', &name(MEDIUM)).unwrap();
    assert_eq!(medium.2, regular.2);
}

/// What egui draws for a character no font has.
const REPLACEMENT: char = '\u{25FB}';

/// Every non-ASCII character in the editor's code (outside comments)
/// can be drawn in every family, so nothing shows as a box.
#[test]
fn symbols_have_glyphs() {
    let ctx = context();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    for entry in std::fs::read_dir(source).into_iter().flatten().flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
        for line in text.lines() {
            let code = line.split("//").next().unwrap_or_default();
            for c in code.chars().filter(|c| !c.is_ascii()) {
                for family in families() {
                    let (min, max, _) = glyph(&ctx, c, &family).unwrap();
                    let (no_min, no_max, _) = glyph(&ctx, REPLACEMENT, &family).unwrap();
                    assert_ne!(
                        (min, max),
                        (no_min, no_max),
                        "{c:?} in {:?} has no {family:?} glyph",
                        entry.path()
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(
        checked > 0,
        "found no symbols, so the test looked in the wrong place"
    );
}
