//! A tileset's and a map file's Properties, for the file picked in the
//! Library: a tileset's terrains (colour, name and code), and a map
//! file's tileset and size. Moved out of the Map tab (`map_tab.rs`),
//! which shows the selected map.

use crate::map_tab::name_field;
use crate::theme;
use egui::{RichText, Ui};
use tiles::{TileMap, Tileset};

/// A change to a tileset's terrains, asked for in its Properties. The
/// workbench makes it through the tileset's editor, as one undo step.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TerrainEdit {
    /// Add a terrain.
    Add,
    /// Take this terrain away, with its tiles and rules.
    Remove(reflect::Id),
    /// Change this terrain's name, code or colour (`None` keeps it).
    Set {
        id: reflect::Id,
        name: Option<String>,
        code: Option<char>,
        colour: Option<[f32; 3]>,
    },
}

/// A tileset's Properties (clicked in the Library): its terrains, each
/// with its colour, name and code. Gives back the changes asked for.
pub(crate) fn tileset_properties(
    ui: &mut Ui,
    path: &str,
    set: &Tileset,
    notes: &[String],
) -> Vec<TerrainEdit> {
    let mut asked = Vec::new();
    let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
    ui.horizontal(|ui| {
        ui.label(RichText::new("▦").color(theme::kind::SPRITE));
        ui.label(RichText::new(name).strong());
    });
    ui.label(RichText::new("Terrains (what a cell can be)").color(theme::text_weak()));
    for t in &set.terrains {
        let set_it = |name, code, colour| TerrainEdit::Set {
            id: t.id,
            name,
            code,
            colour,
        };
        ui.horizontal(|ui| {
            // Once the mouse is let go: a drag in the picker is one undo
            // step and one save, not one a frame.
            let colour = crate::settled::settled_colour(ui, ("terrain", t.id), t.colour);
            if colour.is_some() {
                asked.push(set_it(None, None, colour));
            }
            let name = name_field(ui, ("terrain", t.id), &t.name, 110.0);
            if name.is_some() {
                asked.push(set_it(name, None, None));
            }
            let id = ui.id().with(("code", t.id));
            let mut code: String = ui
                .data(|d| d.get_temp(id))
                .unwrap_or_else(|| t.code.to_string());
            let r = ui
                .add(
                    egui::TextEdit::singleline(&mut code)
                        .desired_width(18.0)
                        .char_limit(1),
                )
                .on_hover_text("The one character the map file writes for it");
            if r.has_focus() {
                ui.data_mut(|d| d.insert_temp(id, code.clone()));
            } else {
                ui.data_mut(|d| d.remove::<String>(id));
                if let Some(c) = code.chars().next()
                    && r.lost_focus()
                    && c != t.code
                {
                    asked.push(set_it(None, Some(c), None));
                }
            }
            let x = ui.small_button("×").on_hover_text(
                "Take this terrain away, with its tiles and rules (maps keep its code, drawn \
                     pink). Ctrl+Z here brings it back.",
            );
            x.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Remove {}", t.name),
                )
            });
            if x.clicked() {
                asked.push(TerrainEdit::Remove(t.id));
            }
        });
    }
    let can_add = set.terrains.len() < tiles::tileset::MAX_TERRAINS;
    if ui
        .add_enabled(can_add, egui::Button::new("+ Terrain"))
        .on_disabled_hover_text(format!(
            "A tileset has at most {} terrains",
            tiles::tileset::MAX_TERRAINS
        ))
        .clicked()
    {
        asked.push(TerrainEdit::Add);
    }
    ui.label(
        RichText::new("Its name, the character the map file writes for it, and its colour. The first fills new maps. Double-click the tileset in the Library to give its terrains art in the Tileset tab.")
            .small()
            .color(theme::text_weak()),
    );
    for note in notes {
        ui.label(RichText::new(note).small().color(theme::WARNING));
    }
    asked
}

/// A map file's Properties (clicked in the Library): its tileset and
/// size. Gives back the tileset and size chosen, if changed.
pub(crate) fn map_file_properties(
    ui: &mut Ui,
    path: &str,
    map: &TileMap,
    tilesets: &[(String, reflect::AssetId)],
    notes: &[String],
) -> Option<(Option<reflect::AssetId>, [u32; 2])> {
    let mut out = None;
    let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
    ui.horizontal(|ui| {
        ui.label(RichText::new("▦").color(theme::kind::SPRITE));
        ui.label(RichText::new(name).strong());
    });
    crate::inspector::row(ui, "Tileset", "The terrains its cells can be.", |ui| {
        let shown = match map.tileset {
            None => "None".to_owned(),
            Some(id) => tilesets
                .iter()
                .find(|(_, i)| *i == id)
                .map_or_else(|| format!("Missing tileset {id}"), |(n, _)| n.clone()),
        };
        egui::ComboBox::from_id_salt(("map tileset", path))
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (n, id) in tilesets {
                    if ui.selectable_label(map.tileset == Some(*id), n).clicked()
                        && map.tileset != Some(*id)
                    {
                        out = Some((Some(*id), map.size()));
                    }
                }
                if tilesets.is_empty() {
                    ui.label("No tilesets yet: right-click in the Library > New > Tileset.");
                }
            });
    });
    crate::inspector::row(
        ui,
        "Size",
        "Its width and height in cells; cells past a smaller size are taken away.",
        |ui| {
            let [w, h] = map.size();
            let max = f64::from(tiles::map::MAX_SIDE);
            let a = crate::settled::settled_whole_drag(
                ui,
                ("map w", path),
                f64::from(w),
                1.0..=max,
                0.2,
                "",
            );
            ui.label(RichText::new("×").color(theme::text_weak()));
            let b = crate::settled::settled_whole_drag(
                ui,
                ("map h", path),
                f64::from(h),
                1.0..=max,
                0.2,
                "",
            );
            ui.label(RichText::new("cells").color(theme::text_weak()));
            if a.is_some() || b.is_some() {
                out = Some((
                    map.tileset,
                    [a.map_or(w, |v| v as u32), b.map_or(h, |v| v as u32)],
                ));
            }
        },
    );
    ui.label(
        RichText::new(format!(
            "{} layers · {} values. Select an entity placing it to change its layers and values in the Map tab.",
            map.layers().len(),
            map.values().len()
        ))
        .small()
        .color(theme::text_weak()),
    );
    for note in notes {
        ui.label(RichText::new(note).small().color(theme::WARNING));
    }
    out
}
