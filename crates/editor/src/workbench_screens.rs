//! The workbench's side of the UI editor (decision 0089): opening a
//! screen's file on the canvas, saving each change, undo, the file
//! changed on disk, and the screen's view drawn from the game's code.

use crate::console::LogLevel;
use crate::layout;
use crate::panel::Panel;
use crate::screen_doc::ScreenDoc;
use crate::workbench::{Workbench, WorkbenchAction};
use std::sync::Arc;

/// The entity a screen's view is drawn for on the canvas: none of the
/// area's, so the screen reads only starting values.
pub(crate) fn preview_entity() -> reflect::EntityId {
    reflect::Id::from_raw(0x5c4e_e0fc_a4a5)
        .map_or_else(reflect::EntityId::random, reflect::EntityId)
}

/// Every component of the canvas's entity, with its starting values: so a
/// screen reading `Ui:entity():health()` draws, as it would at the start.
struct PreviewReader<'a> {
    schemas: Vec<reflect::ComponentSchema>,
    registry: &'a reflect::Registry,
    /// Numbers tried in place of the starting ones (Preview values), by
    /// component and field.
    preview: &'a std::collections::BTreeMap<(String, String), f64>,
}

impl script::ScreenReader for PreviewReader<'_> {
    fn values(&mut self, _entity: reflect::EntityId, name: &str) -> Option<reflect::ComponentData> {
        let schema = self
            .schemas
            .iter()
            .find(|s| s.name == name)
            .or_else(|| self.registry.get(name).map(|s| &**s))?;
        let mut data = reflect::ComponentData::default();
        for f in &schema.fields {
            let tried = self.preview.get(&(name.to_string(), f.name.clone()));
            let value = match (tried, &f.default) {
                (Some(n), reflect::Value::Int(_)) => reflect::Value::Int(n.round() as i64),
                (Some(n), reflect::Value::Float(_)) => reflect::Value::Float(*n),
                _ => f.default.clone(),
            };
            data.set(&f.name, value);
        }
        Some(data)
    }
}

impl Workbench {
    /// The screens in the code file at `path` (inside the assets), by
    /// name, when it has any.
    pub(crate) fn screens_in(&self, path: &str) -> Vec<String> {
        let Some(code) = self.program.as_ref().and_then(|p| p.screens.as_ref()) else {
            return Vec::new();
        };
        code.screens
            .iter()
            .filter(|(_, file)| file == path)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Opens the screen `screen` (its first, if `None`) of the file at
    /// `path` on the canvas, in the UI workspace. Gives whether it opened.
    pub(crate) fn open_screen(&mut self, path: &str, screen: Option<&str>) -> bool {
        let names = self.screens_in(path);
        let Some(name) = screen
            .map(str::to_string)
            .filter(|n| names.contains(n))
            .or_else(|| names.first().cloned())
        else {
            return false;
        };
        let text = self
            .program
            .as_ref()
            .and_then(|p| p.screens.as_ref())
            .and_then(|c| c.sources.iter().find(|(p, _)| p == path))
            .map(|(_, t)| t.clone());
        let Some(text) = text else {
            return false;
        };
        let same = self
            .screen_doc
            .as_ref()
            .is_some_and(|d| d.path == path && d.screen == name);
        if !same {
            let mut doc = ScreenDoc::new(path.to_string(), name, text);
            if let Err(why) = &doc.source {
                self.log(
                    LogLevel::Warning,
                    format!(
                        "The canvas can't follow {path} ({why}), so it shows the screen but \
                         can't change it: edit it as code."
                    ),
                );
            }
            doc.view = None;
            self.screen_doc = Some(doc);
            self.screen_from = None;
        }
        self.switch_workspace(crate::workspace::Workspace::Ui);
        layout::open_first_beside(&mut self.dock, Panel::Canvas, Panel::Game);
        true
    }

    /// Closes the screen open on the canvas.
    pub(crate) fn close_screen(&mut self) {
        self.screen_doc = None;
        self.screen_from = None;
    }

    /// Saves the screen open, after a change.
    pub(crate) fn save_screen(&mut self) {
        let Some(doc) = &mut self.screen_doc else {
            return;
        };
        doc.saved = doc.text.clone();
        self.outbox.push(WorkbenchAction::SaveScreen {
            path: doc.path.clone().into(),
            text: doc.text.clone(),
        });
    }

    /// Undoes (or redoes) the screen's last change.
    pub(crate) fn screen_undo(&mut self, redo: bool) {
        let Some(doc) = &mut self.screen_doc else {
            return;
        };
        if if redo { doc.redo() } else { doc.undo() } {
            self.save_screen();
        }
    }

    /// After the code is checked again: the open file changed on disk is
    /// taken in as one undo step, and the screen's view is drawn again
    /// from the game's code with the screen's starting values.
    pub(crate) fn refresh_screen(&mut self) {
        let program = self.program.clone();
        let from = program.as_ref().map(|p| Arc::as_ptr(p) as usize);
        let Some(doc) = &mut self.screen_doc else {
            return;
        };
        if self.screen_from == from && doc.view.is_some() && !doc.redraw {
            return;
        }
        doc.redraw = false;
        self.screen_from = from;
        let Some(program) = program.as_ref() else {
            doc.view_problem = Some(
                "The game's code has mistakes, so the screen can't be drawn: see Problems.".into(),
            );
            return;
        };
        let Some(code) = program.screens.as_ref() else {
            let path = doc.path.clone();
            self.close_screen();
            self.log(
                LogLevel::Warning,
                format!("{path} isn't a screen any more, so the canvas closed."),
            );
            return;
        };
        let on_disk = code
            .sources
            .iter()
            .find(|(p, _)| *p == doc.path)
            .map(|(_, t)| t.clone());
        let mut said = None;
        match on_disk {
            Some(text) if !doc.is_own(&text) => {
                doc.taken_in(text);
                said = Some(format!(
                    "{} changed on disk, so the canvas shows it as it is now (Ctrl+Z over the \
                     canvas undoes that).",
                    doc.path
                ));
            }
            Some(_) => {}
            None => {
                let path = doc.path.clone();
                self.close_screen();
                self.log(
                    LogLevel::Warning,
                    format!("{path} isn't a screen any more, so the canvas closed."),
                );
                return;
            }
        }
        // The view drawn by the game's own code, with no game running: for
        // an entity of its own, whose components all have their starting
        // values.
        let preview = doc.preview.clone();
        let mut reader = PreviewReader {
            schemas: program.schemas.clone(),
            registry: &self.document.registry,
            preview: &preview,
        };
        let made = script::screens::Screens::start(code)
            .map_err(|f| f.to_string())
            .and_then(|s| {
                s.view(&doc.screen, Some(preview_entity()), &mut reader)
                    .map_err(|p| p.to_string())
            });
        match made {
            Ok(view) => {
                doc.view = Some(view);
                doc.view_problem = None;
                doc.behind = false;
            }
            Err(why) => {
                doc.view = None;
                doc.view_problem = Some(format!("The screen's view stopped: {why}"));
            }
        }
        if let Some(said) = said {
            self.log(LogLevel::Info, said);
        }
    }
}

#[cfg(test)]
#[path = "workbench_screens_tests.rs"]
pub(crate) mod tests;
