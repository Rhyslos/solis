//! The Hints panel (the editor overhaul's O7, decision 0085): a few plain
//! sentences about what's selected and what you can do with it, so a
//! newcomer always has a next step. It can be turned off in Settings >
//! Appearance once you know your way around.

use crate::play::PlayState;
use crate::theme;
use crate::workbench::Workbench;
use egui::{RichText, Ui};

/// What one of the engine's components is for, in a sentence, and what to
/// try with it.
pub(crate) fn about(component: &str) -> Option<&'static str> {
    Some(match component {
        "Transform" => {
            "Transform is where it is, how it's turned and how big it is: drag the gizmo in the viewport, or type the numbers."
        }
        "Model" => {
            "Model shows a 3D model from the Library. Drag another model onto it from the Library to swap it."
        }
        "Light" => {
            "Light makes it shine. Change its colour and strength here; Kind turns a bulb into a spotlight or the sun."
        }
        "Environment" => "Environment is the sky, the fog and how bright the whole area looks.",
        "Camera" => {
            "Camera is what the game looks through: the first camera in the area is the one used when you press Play."
        }
        "RigidBody" => "Rigid body makes it fall, bump and roll with physics when the game runs.",
        "Collider" => "Collider gives it a solid shape, so things bump into it.",
        "CharacterBody" => {
            "Character body lets game code walk it around without falling through floors."
        }
        "Collider2D" => {
            "2D collider gives it a solid shape in a 2D game, so characters stop at it."
        }
        "AudioSource" => "Sound source plays a sound event from where it stands.",
        "Listener" => "Listener is the ears: sounds are heard from here.",
        "ReverbZone" => "Reverb zone makes sounds inside it echo, as in a hall or a cave.",
        "Sprite" => "Sprite shows a picture from a sprite sheet. Clip picks which animation plays.",
        "Animator" => {
            "Animator plays clips by itself, switching between them as game code asks: open its graph to change how."
        }
        "TileMap" => {
            "Tile map is a grid of tiles: pick a tool at the bottom of the viewport and paint."
        }
        "Screen" => {
            "Screen shows one of the game's screens (a HUD, a menu) over the game while it plays: pick it, then press Play."
        }
        _ => return None,
    })
}

impl Workbench {
    /// The hints for now: what's selected and what to do with it, or how
    /// to start when nothing is. The first is the main one.
    pub fn hints(&self) -> Vec<String> {
        let mut hints = Vec::new();
        let selected: Vec<_> = self.document.selection.iter().collect();
        match selected.as_slice() {
            [] => hints.push(
                "Select something in the viewport or the Area panel to see what it is and what you \
                 can do with it. Press Space to search for any action, file or entity."
                    .to_string(),
            ),
            [one] => {
                if let Some(entity) = self.document.area().get(*one) {
                    let mut names: Vec<&String> = entity.components.keys().collect();
                    names.sort_by_key(|n| *n != "Transform");
                    let places = self.program.as_ref().map(|p| &p.places);
                    for name in names {
                        if let Some((file, _)) = places.and_then(|p| p.get(name)) {
                            hints.push(format!(
                                "{}'s {name} component comes from {file}. Change its code in \
                                 Ledger and save: the game picks it up at once, even while playing.",
                                entity.name
                            ));
                        } else if let Some(about) = about(name) {
                            hints.push(about.to_string());
                        }
                    }
                    if hints.is_empty() {
                        hints.push(format!(
                            "{} is empty. Add a component in Properties to give it a job: a \
                             Light, a Model, or one from your game code.",
                            entity.name
                        ));
                    }
                }
            }
            many => hints.push(format!(
                "{} entities are selected. A change in Properties applies to each of them.",
                many.len()
            )),
        }
        match self.play_state() {
            PlayState::Playing => hints.push(
                "You're playing. Changes you make now are listed when you press Stop, to keep or \
                 undo."
                    .to_string(),
            ),
            PlayState::Paused => hints.push(
                "The game is paused. Step runs one tick at a time; Play carries on.".to_string(),
            ),
            PlayState::Editing => {}
        }
        hints
    }

    /// Hints on or off (Settings > Appearance), for saving.
    pub fn hints_on(&self) -> bool {
        self.hints
    }

    /// Turns hints on or off (as saved last time).
    pub fn set_hints_on(&mut self, on: bool) {
        self.hints = on;
    }
}

/// Draws the Hints panel: `hints`, the first strong, then where they come
/// from; or, turned off, how to turn them on.
pub(crate) fn hints_panel(ui: &mut Ui, hints: Option<&[String]>) {
    egui::Frame::new()
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let Some(hints) = hints else {
                ui.label(
                    RichText::new("Hints are off. Turn them on in Edit > Settings… > Appearance.")
                        .color(theme::TEXT_WEAK),
                );
                return;
            };
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (n, hint) in hints.iter().enumerate() {
                    let colour = if n == 0 {
                        theme::TEXT_STRONG
                    } else {
                        theme::TEXT
                    };
                    ui.label(RichText::new(hint).color(colour));
                    ui.add_space(6.0);
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Hints show here for what's selected. Turn them off in Edit > Settings… \
                         > Appearance.",
                    )
                    .color(theme::TEXT_WEAK)
                    .size(11.5),
                );
            });
        });
}

#[cfg(test)]
#[path = "hints_tests.rs"]
mod tests;
