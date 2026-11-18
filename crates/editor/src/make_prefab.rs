//! Make prefab… (board 07, panel 1): the picked entities saved as a prefab,
//! named and put in a folder in a dialog, and, if asked, swapped for a
//! copy linked to it, as one undo step.
//!
//! One picked entity is the prefab's root, with everything under it.
//! Several are put under a new root of the prefab's name, each keeping
//! its place in the world, and that root is what the copy stands in for.
//! The file is the app's to write ([`WorkbenchAction::MakePrefab`]); once it
//! has, the swap is [`Workbench::swap_for_copy`].

use crate::Workbench;
use crate::console::LogLevel;
use crate::widgets;
use crate::workbench::WorkbenchAction;
use level::builtin::Transform;
use level::prefab::Expander;
use level::text::{FileKind, LevelFile};
use level::{Edit, EntityData};
use reflect::{AssetId, EntityId};
use std::collections::HashSet;

/// The width of the dialog's labels (Name, Save in), so its fields line up.
const LABEL_W: f32 = 60.0;

/// The dialog while it's open.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MakePrefabForm {
    /// The prefab's name, which names its file.
    pub(crate) name: String,
    /// The folder it's saved in, inside the project's files (`/`
    /// between folders; empty for the top).
    pub(crate) folder: String,
    /// Whether the picked entities become a copy linked to it.
    pub(crate) swap: bool,
    /// The entities picked: the tops of what goes in.
    pub(crate) picks: Vec<EntityId>,
    /// How many entities go in, the picked and everything under them.
    pub(crate) count: usize,
}

/// What the app does to make a prefab: write `file` as a new prefab named
/// `name` in `folder`, then, with `swap`, hand it back to
/// [`Workbench::swap_for_copy`].
#[derive(Debug, Clone, PartialEq)]
pub struct MakePrefabPlan {
    /// Its name, for its file (`chicken.prefab.lst`).
    pub name: String,
    /// The folder, inside the project's files.
    pub folder: std::path::PathBuf,
    /// Its contents.
    pub file: LevelFile,
    /// What becomes a copy of it, if anything.
    pub swap: Option<SwapPlan>,
}

/// The entities that become one linked copy, and the copy's root.
#[derive(Debug, Clone, PartialEq)]
pub struct SwapPlan {
    /// The picked entities (each with everything under it) that go.
    pub picks: Vec<EntityId>,
    /// The copy's root: the one picked entity's own ID, or a new one.
    pub root: EntityId,
    /// The copy's root as the level places it: its name, parent and order.
    pub placed: EntityData,
}

impl Workbench {
    /// Opens Make prefab… for what's picked in the level: named after the
    /// first picked entity, saved in the Library's folder, swapped for a
    /// copy. Nothing picked, or the game running: why, in the Console.
    pub(crate) fn ask_make_prefab(&mut self) {
        let picks = self.selected_roots();
        let Some(&first) = picks.first() else {
            self.log(
                LogLevel::Warning,
                "Pick the entities to make a prefab of first.",
            );
            return;
        };
        if self.open.playing.is_some() {
            self.log(
                LogLevel::Warning,
                "Stop playing first: a prefab is made from the level.",
            );
            return;
        }
        let level = self.open.document.level();
        let count = picks.iter().map(|&p| level.tree(p).len()).sum();
        let name = level
            .get(first)
            .map_or_else(String::new, |e| e.name.clone());
        self.open.make_prefab = Some(MakePrefabForm {
            name,
            folder: self.open.library_state.folder.clone(),
            swap: true,
            picks,
            count,
        });
    }

    /// A prefab was just made at `path` (inside the project): its tile
    /// carries a "New" badge, and it's picked in the Library.
    pub fn prefab_made(&mut self, path: &str) {
        self.open.library_state.new_prefabs.insert(path.to_owned());
        self.pick_in_library(path);
    }

    /// The folder chosen with Browse…, inside the project's files.
    pub fn set_prefab_folder(&mut self, folder: &str) {
        if let Some(form) = &mut self.open.make_prefab {
            form.folder = folder.trim_matches('/').to_string();
        }
    }

    /// Draws Make prefab… while it's open, and carries out its answer.
    pub(crate) fn make_prefab_dialog(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        let Some(mut form) = self.open.make_prefab.take() else {
            return;
        };
        use widgets::dialog::{Answer, dialog_with_note};
        let mut browse = false;
        let note = format!(
            "{} {} become{} one.",
            form.count,
            if form.count == 1 {
                "entity"
            } else {
                "entities"
            },
            if form.count == 1 { "s" } else { "" }
        );
        let answers = [Answer::right("Cancel"), Answer::main("Make prefab")];
        let picked = dialog_with_note(
            ctx,
            "make prefab",
            "Make prefab",
            |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [LABEL_W, 20.0],
                        egui::Label::new(
                            egui::RichText::new("Name").color(crate::tokens::text_label()),
                        ),
                    );
                    widgets::field::text_field(
                        ui,
                        "Name",
                        &mut form.name,
                        "The prefab's name",
                        236.0,
                    );
                });
                ui.horizontal(|ui| {
                    let shown = if form.folder.is_empty() {
                        "(the project's folder)".to_string()
                    } else {
                        form.folder.clone()
                    };
                    ui.add_sized(
                        [LABEL_W, 20.0],
                        egui::Label::new(
                            egui::RichText::new("Save in").color(crate::tokens::text_label()),
                        ),
                    );
                    egui::Frame::new()
                        .fill(crate::tokens::field())
                        .corner_radius(crate::tokens::size::radius::CONTROL)
                        .inner_margin(egui::Margin::symmetric(6, 3))
                        .show(ui, |ui| {
                            ui.set_width(150.0);
                            ui.label(
                                egui::RichText::new(shown).font(crate::tokens::text::mono_small()),
                            );
                        })
                        .response
                        .on_hover_text(
                            "The project's folder the prefab is saved in: Browse… picks another",
                        );
                    browse = widgets::button::Button::plain("Browse\u{2026}")
                        .show(ui)
                        .clicked();
                });
                widgets::checkbox::checkbox(ui, &mut form.swap, "Swap these for a linked copy")
                    .on_hover_text(
                        "The picked entities become one copy of the prefab, which follows it",
                    );
            },
            &note,
            &answers,
        );
        if browse {
            actions.push(WorkbenchAction::ChoosePrefabFolder(form.folder.clone()));
        }
        match picked {
            Some(0) => {}
            Some(_) => match self.make_prefab_plan(&form) {
                Ok(plan) => actions.push(WorkbenchAction::MakePrefab(plan)),
                Err(why) => {
                    self.log(LogLevel::Warning, why);
                    self.open.make_prefab = Some(form);
                }
            },
            None => self.open.make_prefab = Some(form),
        }
    }

    /// The plan for the dialog's answer: the prefab's contents and the
    /// swap. A name that's empty is refused, with why.
    pub(crate) fn make_prefab_plan(&self, form: &MakePrefabForm) -> Result<MakePrefabPlan, String> {
        let name = form.name.trim();
        if name.is_empty() {
            return Err("A prefab needs a name.".into());
        }
        let (file, root, placed) = match form.picks.as_slice() {
            [] => return Err("Pick the entities to make a prefab of first.".into()),
            [one] => {
                let mut file = self
                    .prefab_file(*one)
                    .ok_or("The picked entities couldn't be made a prefab.")?;
                if let Some(top) = file.level.get_mut(*one) {
                    top.name = name.to_string();
                }
                let level = self.open.document.level();
                let original = level.get(*one).ok_or("The picked entity is gone.")?;
                let mut placed = EntityData::new(&original.name);
                placed.parent = original.parent;
                placed.order.clone_from(&original.order);
                (file, *one, placed)
            }
            many => self.grouped_prefab(many, name)?,
        };
        Ok(MakePrefabPlan {
            name: name.to_string(),
            folder: std::path::PathBuf::from(&form.folder),
            file,
            swap: form.swap.then(|| SwapPlan {
                picks: form.picks.clone(),
                root,
                placed,
            }),
        })
    }

    /// Several picked entities under one new root named `name`: the
    /// prefab's contents, the root's ID, and where the copy goes (beside
    /// the first picked, under their parent if they share one).
    fn grouped_prefab(
        &self,
        picks: &[EntityId],
        name: &str,
    ) -> Result<(LevelFile, EntityId, EntityData), String> {
        let document = &self.open.document;
        let level = document.level();
        let parents: HashSet<Option<EntityId>> = picks
            .iter()
            .filter_map(|&p| level.get(p).map(|e| e.parent))
            .collect();
        let parent = match parents.len() {
            1 => parents.into_iter().next().flatten(),
            _ => None,
        };
        let root = EntityId::random();
        let mut placed = EntityData::new(name);
        placed.parent = parent;
        placed.order = level
            .get(picks[0])
            .map_or_else(|| level.order_at_end(parent), |e| e.order.clone());
        let mut contents = level::Level::new();
        contents.insert(root, EntityData::new(name).with(&Transform::default()));
        let parent_world = parent.map_or(glam::Mat4::IDENTITY, |p| document.world_matrix(p));
        let inside: HashSet<EntityId> = picks.iter().flat_map(|&p| level.tree(p)).collect();
        let mut order = None;
        for &pick in picks {
            for id in level.tree(pick) {
                let mut data = level.get(id).ok_or("A picked entity is gone.")?.clone();
                if id == pick {
                    data.parent = Some(root);
                    let next = level::order_after(order.as_deref());
                    data.order.clone_from(&next);
                    order = Some(next);
                    // Where it is in the world stays where it is.
                    let local = parent_world.inverse() * document.world_matrix(id);
                    let (scale, rotation, position) = local.to_scale_rotation_translation();
                    data.set(&Transform {
                        position,
                        rotation: rotation.normalize(),
                        scale,
                    });
                }
                // Copies placed inside stay copies; a part whose copy is
                // outside becomes the prefab's own.
                if data
                    .origin
                    .as_ref()
                    .is_some_and(|o| !inside.contains(&o.instance))
                {
                    data.origin = None;
                }
                contents.insert(id, data);
            }
        }
        let lookup = |asset| self.open.prefabs.get(&asset).cloned();
        let file = Expander::new(&lookup)
            .collapse(&contents, FileKind::Prefab { root })
            .map_err(|why| format!("The picked entities couldn't be made a prefab: {why}."))?;
        Ok((file, root, placed))
    }

    /// The prefab `id` was made from a plan with `swap`: its picked
    /// entities become one copy linked to it, in one undo step.
    pub fn swap_for_copy(&mut self, id: AssetId, swap: &SwapPlan) {
        let entities = {
            let lookup = |asset| self.open.prefabs.get(&asset).cloned();
            Expander::new(&lookup).place(id, swap.root, &swap.placed)
        };
        let entities = match entities {
            Ok(entities) => entities,
            Err(why) => {
                self.log(LogLevel::Error, format!("Couldn't swap in a copy: {why}."));
                return;
            }
        };
        let level = self.open.document.level();
        let picks: Vec<EntityId> = swap
            .picks
            .iter()
            .copied()
            .filter(|&p| level.contains(p))
            .collect();
        let mut edits = level::prefab::despawn(level, &picks);
        edits.push(Edit::Spawn { entities });
        let mut selected = crate::document::Selection::default();
        selected.select_only(swap.root);
        let label = format!("Make prefab {}", self.asset_name(id));
        self.command(&label, edits, None, Some(selected));
    }
}

#[cfg(test)]
#[path = "make_prefab_tests.rs"]
mod tests;
