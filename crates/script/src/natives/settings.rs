//! The player's settings (decision 0091): `Screens` and `Window`.

use super::Scope;
use crate::convert::vec3_value;
use crate::host::Lent;
use crate::native_values::native;
use glam::Vec3;
use mlua::{Table, Value as LuaValue};

/// Lends this file's natives into `natives`, each answered by `h`.
pub(super) fn lend<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    lend_reading(scope, natives, h)?;
    lend_screens(scope, natives, h)?;
    lend_window(scope, natives, h)?;
    lend_words(scope, natives, h)?;
    Ok(())
}

/// The player's language, subtitles and captions (stage 1, translations).
fn lend_words<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Screens_language",
        scope.create_function(move |_, _w: LuaValue| {
            let host = h.borrow();
            Ok(shown_language(host.texts().as_ref(), host.settings()).option())
        })?,
    )?;
    natives.raw_set(
        "Screens_setLanguage",
        native(
            scope,
            "Screens_setLanguage",
            move |_, (_w, which): (LuaValue, String)| {
                let found = language_named(h.borrow().texts().as_ref(), &which).map_err(|why| {
                    mlua::Error::runtime(format!("solis.Screens:setLanguage(which): {why}"))
                })?;
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::Language(found));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Screens_languageName",
        native(
            scope,
            "Screens_languageName",
            move |_, (_w, which): (LuaValue, String)| {
                Ok(language_named(h.borrow().texts().as_ref(), &which)
                    .map_err(|why| {
                        mlua::Error::runtime(format!("solis.Screens:languageName(which): {why}"))
                    })?
                    .name())
            },
        )?,
    )?;
    natives.raw_set(
        "Screens_subtitles",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().subtitles))?,
    )?;
    natives.raw_set(
        "Screens_captions",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().captions))?,
    )?;
    natives.raw_set(
        "Screens_captionSize",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().caption_size))?,
    )?;
    natives.raw_set(
        "Screens_captionBackground",
        scope
            .create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().caption_background))?,
    )?;
    for key in ["Screens_setSubtitles", "Screens_setCaptions"] {
        natives.raw_set(
            key,
            native(scope, key, move |_, (_w, on): (LuaValue, bool)| {
                let change = if key == "Screens_setSubtitles" {
                    crate::settings::Setting::Subtitles(on)
                } else {
                    crate::settings::Setting::Captions(on)
                };
                h.borrow_mut().set_setting(change);
                Ok(())
            })?,
        )?;
    }
    for key in ["Screens_setCaptionSize", "Screens_setCaptionBackground"] {
        natives.raw_set(
            key,
            native(scope, key, move |_, (_w, v): (LuaValue, f64)| {
                let change = if key == "Screens_setCaptionSize" {
                    crate::settings::Setting::CaptionSize(v)
                } else {
                    crate::settings::Setting::CaptionBackground(v)
                };
                h.borrow_mut().set_setting(change);
                Ok(())
            })?,
        )?;
    }
    Ok(())
}

/// The language the player reads in: theirs, the computer's, or the
/// project's main one (English with no text).
pub(crate) fn shown_language(
    texts: Option<&translations::Texts>,
    settings: crate::settings::PlayerSettings,
) -> translations::Language {
    match texts {
        Some(texts) => texts.language(),
        None => settings.language.unwrap_or_default(),
    }
}

/// The project's language whose option game code named (`Nb`), or why
/// there's none.
pub(crate) fn language_named(
    texts: Option<&translations::Texts>,
    which: &str,
) -> Result<translations::Language, String> {
    let languages = texts.map_or_else(
        || vec![translations::Language::english()],
        |t| t.all().languages(),
    );
    translations::language::of_option(which, &languages).ok_or_else(|| {
        let had: Vec<String> = languages
            .iter()
            .map(translations::Language::option)
            .collect();
        format!(
            "{which} isn't one of the project's languages ({}): add it in Project settings › \
             Languages",
            had.join(", ")
        )
    })
}

/// Reading the settings.
fn lend_reading<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    // The player's settings (decision 0091).
    natives.raw_set(
        "Screens_textSize",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().text_size))?,
    )?;
    natives.raw_set(
        "Screens_uiSize",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().ui_size))?,
    )?;
    natives.raw_set(
        "Window_fullscreen",
        scope.create_function(move |_, _w: LuaValue| Ok(h.borrow().settings().fullscreen))?,
    )?;
    natives.raw_set(
        "Window_resolution",
        scope.create_function(move |_, _w: LuaValue| {
            let [w, h2] = h.borrow().settings().resolution;
            Ok(vec3_value(Vec3::new(w as f32, h2 as f32, 0.0)))
        })?,
    )?;
    Ok(())
}

/// Changing the text and UI sizes.
fn lend_screens<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Screens_setTextSize",
        native(
            scope,
            "Screens_setTextSize",
            move |_, (_w, size): (LuaValue, f64)| {
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::TextSize(size));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Screens_setUiSize",
        native(
            scope,
            "Screens_setUiSize",
            move |_, (_w, size): (LuaValue, f64)| {
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::UiSize(size));
                Ok(())
            },
        )?,
    )?;
    Ok(())
}

/// Changing the window, and quitting.
fn lend_window<'s>(
    scope: &'s Scope<'s, '_>,
    natives: &Table,
    h: &'s Lent<'_, '_>,
) -> mlua::Result<()> {
    natives.raw_set(
        "Window_setFullscreen",
        native(
            scope,
            "Window_setFullscreen",
            move |_, (_w, on): (LuaValue, bool)| {
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::Fullscreen(on));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Window_setResolution",
        native(
            scope,
            "Window_setResolution",
            move |_, (_w, width, height): (LuaValue, i64, i64)| {
                let side = |v: i64| v.clamp(0, i64::from(u32::MAX)) as u32;
                h.borrow_mut()
                    .set_setting(crate::settings::Setting::Resolution(
                        side(width),
                        side(height),
                    ));
                Ok(())
            },
        )?,
    )?;
    natives.raw_set(
        "Window_quit",
        scope.create_function(move |_, _w: LuaValue| {
            h.borrow_mut().quit();
            Ok(())
        })?,
    )?;
    natives.raw_set(
        "Window_useNextResolution",
        scope.create_function(move |_, _w: LuaValue| {
            let [width, height] = h.borrow().settings().next_resolution();
            h.borrow_mut()
                .set_setting(crate::settings::Setting::Resolution(width, height));
            Ok(())
        })?,
    )?;
    Ok(())
}
