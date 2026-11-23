//! A module in Details (board 07, panel 2): its picture, name, kind and
//! folder, how many times it's placed and in how many levels, its parts,
//! and **Open** and **Place**. The app counts the copies in the project's
//! levels on a worker ([`crate::WorkbenchAction::CountPlaced`]); the open level's
//! are counted from the level as it is now, saved or not.

use crate::Workbench;
use crate::icons::{self, Icon};
use crate::library::{LibraryItem, LibraryRequest};
use crate::properties::PropertyRequest;
use crate::tokens::{self, size, text};
use egui::{RichText, Ui};
use reflect::AssetId;

/// The copies of a module in the project's levels, as the app counted
/// them on disk: each level's path inside the project, and its count.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Placed {
    /// Each level that places it, and how many times.
    pub levels: Vec<(String, usize)>,
}

/// What Details shows of a module, worked out before it's drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModuleDetails {
    /// The module's file.
    pub(crate) item: LibraryItem,
    /// Its parts: the entities right under its root, each with its icon
    /// and colour.
    pub(crate) parts: Vec<(Icon, egui::Color32, String)>,
    /// How many copies, in how many levels; none until counted.
    pub(crate) placed: Option<(usize, usize)>,
}

impl Workbench {
    /// The app's count of `id`'s copies in the project's levels.
    pub fn set_placed(&mut self, id: AssetId, placed: Placed) {
        self.open.placed.insert(id, placed);
    }

    /// Details' module, while a module is what it shows.
    pub(crate) fn module_details(&self) -> Option<ModuleDetails> {
        let (path, _) = self.open.sound_shown.as_ref()?;
        let item = self
            .open
            .library
            .iter()
            .find(|i| &i.path == path && i.kind == crate::library::LibraryKind::Module)?
            .clone();
        let id = item.id?;
        let parts = self
            .open
            .modules
            .get(&id)
            .map(|file| parts_of(file, &self.open.models))
            .unwrap_or_default();
        Some(ModuleDetails {
            placed: self.placed_count(id),
            parts,
            item,
        })
    }

    /// How many copies of `id` the project's levels place, and in how many
    /// levels: the open level's as it is now, the others' as saved.
    fn placed_count(&self, id: AssetId) -> Option<(usize, usize)> {
        let counted = self.open.placed.get(&id)?;
        let open_file = self.open.document.file().and_then(|file| {
            let project = self.open.project.as_ref()?;
            let inside = file.strip_prefix(&project.assets).ok()?;
            Some(inside.to_string_lossy().replace('\\', "/"))
        });
        let here = self
            .open
            .document
            .level()
            .iter()
            .filter(|(entity, e)| {
                e.origin
                    .as_ref()
                    .is_some_and(|o| o.module == id && o.path.is_empty() && o.instance == *entity)
            })
            .count();
        let mut copies = here;
        let mut levels = usize::from(here > 0);
        for (level, n) in &counted.levels {
            if open_file.as_deref() == Some(level.as_str()) || *n == 0 {
                continue;
            }
            copies += n;
            levels += 1;
        }
        Some((copies, levels))
    }
}

/// The entities right under a module's root, in their order.
fn parts_of(
    file: &level::text::LevelFile,
    models: &std::collections::BTreeMap<AssetId, crate::models::ModelInfo>,
) -> Vec<(Icon, egui::Color32, String)> {
    let level::text::FileKind::Module { root } = file.kind else {
        return Vec::new();
    };
    let mut under: Vec<(&str, &level::EntityData)> = file
        .level
        .iter()
        .filter(|(_, e)| e.parent == Some(root))
        .map(|(_, e)| (e.order.as_str(), e))
        .collect();
    under.sort_by(|a, b| a.0.cmp(b.0));
    under
        .into_iter()
        .map(|(_, e)| {
            let (icon, colour, _) = crate::level_panel::looks(e, models);
            (icon, colour, e.name.clone())
        })
        .collect()
}

/// The side of Details' picture of a module (board 07).
const PICTURE: f32 = 64.0;

/// "Placed 6 times, in 2 levels" (or "Not placed yet").
pub(crate) fn placed_words(placed: Option<(usize, usize)>) -> String {
    match placed {
        None => "Counting where it's placed\u{2026}".into(),
        Some((0, _)) => "Not placed yet".into(),
        Some((copies, levels)) => format!(
            "Placed {copies} time{}, in {levels} level{}",
            if copies == 1 { "" } else { "s" },
            if levels == 1 { "" } else { "s" },
        ),
    }
}

/// Draws Details for a module. Returns what was asked for: Open (its page),
/// or Place.
pub(crate) fn details(
    ui: &mut Ui,
    details: &ModuleDetails,
    requests: &mut Vec<PropertyRequest>,
    library: &mut Vec<LibraryRequest>,
) {
    let item = &details.item;
    egui::Frame::new()
        .inner_margin(crate::theme::panel_margin())
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let side = PICTURE;
                let (picture, _) =
                    ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
                ui.painter().rect_filled(
                    picture,
                    egui::CornerRadius::same(size::radius::TAB),
                    tokens::hover(),
                );
                match item.thumbnail {
                    Some(texture) => {
                        ui.painter().image(
                            texture,
                            picture,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                    None => icons::icon(
                        ui.painter(),
                        picture.shrink(side * 0.3),
                        Icon::Module,
                        crate::theme::kind::MODULE,
                    ),
                }
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&item.name)
                            .font(text::dialog_title())
                            .color(tokens::text_emphasis()),
                    );
                    let folder = item.folder();
                    let kind = if folder.is_empty() {
                        "Module".to_string()
                    } else {
                        format!("Module \u{b7} {folder}")
                    };
                    ui.label(RichText::new(kind).color(tokens::text_meta()));
                    ui.label(
                        RichText::new(placed_words(details.placed)).color(tokens::text_meta()),
                    );
                });
            });
            ui.add_space(size::CARD_INNER_GAP);
            ui.label(
                RichText::new("ITS PARTS")
                    .font(text::tile_kind())
                    .color(tokens::text_meta()),
            );
            if details.parts.is_empty() {
                ui.label(RichText::new("Just its top entity").color(tokens::text_meta()));
            }
            for (icon, colour, name) in &details.parts {
                ui.horizontal(|ui| {
                    let (spot, _) = ui.allocate_exact_size(
                        egui::vec2(size::ICON_SMALL, size::ICON_SMALL),
                        egui::Sense::hover(),
                    );
                    icons::icon(ui.painter(), spot, *icon, *colour);
                    ui.label(RichText::new(name).color(tokens::text_body()));
                });
            }
            ui.add_space(size::CARD_INNER_GAP);
            ui.horizontal(|ui| {
                if crate::widgets::button::Button::plain("Open")
                    .show(ui)
                    .on_hover_text("Opens the module on its own page")
                    .clicked()
                    && let Some(id) = item.id
                {
                    requests.push(PropertyRequest::OpenModule(id));
                }
                if crate::widgets::button::Button::plain("Place")
                    .show(ui)
                    .on_hover_text("Places a copy where the camera looks")
                    .clicked()
                    && let Some(id) = item.id
                {
                    library.push(LibraryRequest::Place(id));
                }
                ui.label(RichText::new("Drag it into the view").color(tokens::text_meta()));
            });
        });
}

#[cfg(test)]
#[path = "module_details_tests.rs"]
mod tests;
