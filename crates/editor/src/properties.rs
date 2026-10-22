//! The Properties panel: the fields of whatever is selected in the Scene
//! panel. Until the self-describing inspector arrives (Phase 2), each kind
//! of thing has a hand-written form.

use crate::lighting::{LampRequest, Selection};
use crate::models::{ModelId, ModelInfo};
use crate::panels::placeholder;
use crate::theme;
use crate::viewport::Viewport;
use egui::{RichText, Ui};

/// The Properties panel: the fields of whatever is selected.
pub(crate) fn properties(
    ui: &mut Ui,
    viewport: &mut Viewport,
    models: &[ModelInfo],
    selection: &mut Selection,
    removed: &mut Vec<ModelId>,
) {
    if *selection == Selection::Nothing {
        placeholder(ui, "Nothing selected. Pick something in the Scene panel.");
        return;
    }
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| match *selection {
            Selection::Nothing => {}
            Selection::Sun => {
                ui.label(RichText::new("Sun").strong());
                ui.add_space(4.0);
                crate::lighting::sun_properties(ui, &mut viewport.sun);
            }
            Selection::Environment => {
                crate::environment::environment_properties(ui, &mut viewport.environment);
            }
            Selection::Lamp(id) => {
                let Some(i) = viewport.lamps.iter().position(|l| l.id == id) else {
                    *selection = Selection::Nothing;
                    return;
                };
                let request = crate::lighting::lamp_properties(ui, &mut viewport.lamps[i]);
                match request {
                    LampRequest::None => {}
                    LampRequest::MoveHere => {
                        let camera = viewport.camera;
                        viewport.lamps[i].move_here(
                            camera.position,
                            camera.forward(),
                            camera.focus(),
                        );
                    }
                    LampRequest::Delete => {
                        viewport.lamps.remove(i);
                        *selection = Selection::Nothing;
                    }
                }
            }
            Selection::Model(id) => {
                let Some(model) = models.iter().find(|m| m.id == id) else {
                    *selection = Selection::Nothing;
                    return;
                };
                ui.label(RichText::new(&model.name).strong());
                ui.add_space(4.0);
                ui.label(format!(
                    "{} triangles",
                    crate::models::grouped(model.triangles)
                ));
                ui.label(model.size_text());
                ui.add_space(6.0);
                if ui
                    .button("Remove")
                    .on_hover_text("Take it out of the viewport, with any lights it brought")
                    .clicked()
                {
                    removed.push(model.id);
                    *selection = Selection::Nothing;
                }
            }
        });
}

/// A labelled row in Properties: the name on the left, the field after it.
pub(crate) fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_sized(
            egui::vec2(78.0, 18.0),
            egui::Label::new(RichText::new(label).color(crate::theme::TEXT_WEAK)),
        );
        add(ui)
    })
    .inner
}
