//! The workbench's side of trigger zones (stage 1, triggers): which zones
//! the view outlines solid and which one shows handles, a handle's drag
//! and a freeform zone's points written as undo steps (one a drag), and
//! the keys while points are drawn or edited: `Enter` closes a drawing,
//! `Esc` gives it up (or finishes editing), `Tab` starts and finishes
//! editing, and `Delete` takes the picked point away. They come before
//! the brushes' Tab, Delete's other job and Esc's.

use crate::commands::{component_edits, merge_key};
use crate::play::PlayState;
use crate::workbench::Workbench;
use crate::zones::ZoneEvent;
use crate::zones::points::{PointsEdit, Stage};
use egui::{Key, Modifiers};
use level::builtin::{Transform, TriggerZone, ZoneForm};
use reflect::{Component, EntityId};

impl Workbench {
    /// Each frame: the zones picked (outlined solid), and the one picked
    /// alone (its handles shown). Points drawn or edited stop when their
    /// zone goes, or the game plays.
    pub(crate) fn prepare_zones(&mut self) {
        let playing = self.play_state() != PlayState::Editing;
        let level = self.open.document.level();
        let selection = &self.open.document.selection;
        let picked: Vec<EntityId> = selection
            .iter()
            .filter(|id| level.get(*id).is_some_and(|e| e.has(TriggerZone::NAME)))
            .collect();
        let alone = selection
            .primary()
            .filter(|_| selection.iter().count() == 1)
            .filter(|id| picked.contains(id));
        let zones = &mut self.viewport.zones;
        zones.picked = picked;
        zones.alone = alone;
        if zones.drag.as_ref().is_some_and(|d| Some(d.entity) != alone) {
            zones.drag = None;
        }
        let gone = zones
            .points
            .as_ref()
            .is_some_and(|edit| level.get(edit.entity).is_none());
        if gone || playing {
            zones.points = None;
        }
    }

    /// Carries out what the zone tools did this frame.
    pub(crate) fn zone_events(&mut self) {
        for event in std::mem::take(&mut self.viewport.zones.events) {
            match event {
                ZoneEvent::Sized {
                    entity,
                    zone,
                    place,
                } => self.size_zone(entity, &zone, place),
                ZoneEvent::Points {
                    entity,
                    points,
                    dragging,
                } => self.set_zone_points(entity, points, dragging),
                ZoneEvent::GaveUp => self.viewport.zones.tell("The shape was left as it was."),
                ZoneEvent::Refused(why) => self.viewport.zones.tell(why),
            }
        }
    }

    /// A handle's drag: the zone's new size, and its entity moved so the
    /// far side stays where it was. One undo step a drag.
    fn size_zone(&mut self, entity: EntityId, zone: &TriggerZone, place: glam::Vec3) {
        let level = self.open.document.level();
        let Some(data) = level.get(entity) else {
            return;
        };
        let mut edits = component_edits(entity, data.components.get(TriggerZone::NAME), zone);
        let transform = data.get::<Transform>().unwrap_or_default();
        // The parent's world: where the entity's own place is measured
        // from.
        let parent = self.open.document.world_matrix(entity) * transform.matrix().inverse();
        let local = parent.inverse().transform_point3(place);
        if local.is_finite() && local != transform.position {
            let moved = Transform {
                position: local,
                ..transform
            };
            edits.extend(component_edits(
                entity,
                data.components.get(Transform::NAME),
                &moved,
            ));
        }
        let key = merge_key(("zone size", entity), self.gesture);
        self.command("Size zone", edits, Some(key), None);
    }

    /// A freeform zone's points: the zone becomes freeform with them. A
    /// drag's changes are one undo step; anything else is a step of its
    /// own.
    fn set_zone_points(&mut self, entity: EntityId, points: Vec<glam::Vec2>, dragging: bool) {
        let level = self.open.document.level();
        let Some(data) = level.get(entity) else {
            return;
        };
        let Some(before) = data.get::<TriggerZone>() else {
            return;
        };
        let zone = TriggerZone {
            form: ZoneForm::Freeform,
            points,
            ..before
        };
        let edits = component_edits(entity, data.components.get(TriggerZone::NAME), &zone);
        let key = dragging.then(|| merge_key(("zone points", entity), self.gesture));
        self.command("Edit zone points", edits, key, None);
    }

    /// Starts drawing a freeform outline on `entity`'s zone in the view
    /// (Shape › Freeform): its shape stays as it was until the outline
    /// closes.
    pub(crate) fn start_zone_drawing(&mut self, entity: EntityId) {
        if self.play_state() != PlayState::Editing {
            return;
        }
        let world = self.open.document.world_matrix(entity);
        self.viewport.tools.cancel();
        self.viewport.zones.points = Some(PointsEdit::drawing(entity, world));
        self.show_panel(crate::panel::Panel::Viewport);
    }

    /// Starts or finishes editing the picked freeform zone's points (Tab,
    /// Edit points).
    pub(crate) fn toggle_zone_points(&mut self) {
        if self.viewport.zones.points.take().is_some() {
            return;
        }
        if self.why_not_edit_zone_points().is_some() {
            return;
        }
        let Some(entity) = self.viewport.zones.alone else {
            return;
        };
        let Some(zone) = self
            .open
            .document
            .level()
            .get(entity)
            .and_then(|e| e.get::<TriggerZone>())
        else {
            return;
        };
        let world = self.open.document.world_matrix(entity);
        self.viewport.tools.cancel();
        self.viewport.zones.points = Some(PointsEdit::editing(entity, world, &zone));
    }

    /// Why the picked zone's points can't be edited now, or `None` when
    /// they can (or are being drawn or edited, when Tab ends it).
    pub(crate) fn why_not_edit_zone_points(&self) -> Option<String> {
        if self.viewport.zones.points.is_some() {
            return None;
        }
        if self.play_state() != PlayState::Editing {
            return Some("Stop the game first".to_owned());
        }
        let freeform = self.viewport.zones.alone.is_some_and(|id| {
            self.open
                .document
                .level()
                .get(id)
                .and_then(|e| e.get::<TriggerZone>())
                .is_some_and(|z| z.form == ZoneForm::Freeform)
        });
        (!freeform).then(|| "Pick one freeform trigger zone first".to_owned())
    }

    /// The keys while a freeform zone's points are drawn or edited, and
    /// Tab with one picked. Only where the keys are the level's.
    pub(crate) fn zone_keys(&mut self, ui: &egui::Ui) {
        if !self.keys.is_level() || self.viewport.is_flying() {
            return;
        }
        let active = self.viewport.zones.points.is_some();
        if !active && self.why_not_edit_zone_points().is_some() {
            return;
        }
        let (tab, enter, escape, delete) = ui.input_mut(|i| {
            let tab = i.consume_key(Modifiers::NONE, Key::Tab);
            if !active {
                return (tab, false, false, false);
            }
            (
                tab,
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
                i.consume_key(Modifiers::NONE, Key::Delete)
                    | i.consume_key(Modifiers::NONE, Key::Backspace),
            )
        });
        if tab {
            // Taken from egui too, which would move the keyboard's focus.
            ui.ctx()
                .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        }
        let tool = &mut self.viewport.zones;
        let Some(edit) = tool.points.as_mut() else {
            if tab {
                self.toggle_zone_points();
            }
            return;
        };
        match edit.stage {
            Stage::Drawing => {
                if enter && edit.close(&mut tool.events) {
                    tool.points = None;
                } else if escape {
                    tool.points = None;
                    tool.events.push(ZoneEvent::GaveUp);
                }
            }
            Stage::Editing => {
                if delete {
                    edit.delete_picked(&mut tool.events);
                }
                if tab || escape {
                    tool.points = None;
                }
            }
        }
    }

    /// What the status bar says about zones: a note for a moment (why a
    /// change was refused), or while drawing or editing points, how.
    pub(crate) fn zone_status(&self) -> Option<String> {
        let tool = &self.viewport.zones;
        if let Some(note) = tool.note_now() {
            return Some(note.to_owned());
        }
        let edit = tool.points.as_ref()?;
        let name = self
            .open
            .document
            .level()
            .get(edit.entity)
            .map_or("the zone", |e| e.name.as_str());
        Some(match edit.stage {
            Stage::Drawing => format!(
                "Drawing {name}: click to place points; click the first point or Enter closes it; \
                 Esc gives up"
            ),
            Stage::Editing => format!(
                "Editing {name}'s {} points: drag one to move it, Ctrl + click an edge to add \
                 one, Delete takes the picked one away; Tab or Esc finishes",
                edit.points.len()
            ),
        })
    }
}

#[cfg(test)]
#[path = "workbench_zones_tests.rs"]
mod tests;

/// A component's name made from a zone's: its words joined, each starting
/// with a capital ("Door zone" makes `DoorZone`); `Zone` when nothing
/// usable is left, and `Zone` before one that would start with a digit.
pub(crate) fn component_name(zone: &str) -> String {
    let mut name: String = zone
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut letters = w.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + letters.as_str()
            })
        })
        .collect();
    if name.is_empty() {
        name = "Zone".to_owned();
    } else if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert_str(0, "Zone");
    }
    name
}

/// The Lust a zone's own code file starts with: its component, with
/// empty `Entered` and `Exited` to fill in.
pub(crate) fn zone_code(name: &str, path: &str, zone: &str) -> String {
    let fetch = path.strip_suffix(".lst").unwrap_or(path);
    format!(
        "-- {path}: what the trigger zone \"{zone}\" does when something walks in or out.\n\
         -- It's fetched from main.lst:  fetch \"{fetch}\":{name};\n\
         \n\
         fetch \"solis/zones\":[Entered, Exited];\n\
         \n\
         component {name} {{\n\
         \x20   -- Something walked into the zone: who it was.\n\
         \x20   on Entered(entity who) {{\n\
         \x20   }}\n\
         \n\
         \x20   -- Something walked out of it.\n\
         \x20   on Exited(entity who) {{\n\
         \x20   }}\n\
         }}\n\
         \n\
         pass {name};\n\
         end\n"
    )
}

impl Workbench {
    /// New code for this zone…: a code file named after the zone at the
    /// top of the project, its component added to the zone (one undo
    /// step), and the file opened. A project with no `main.lst` gets one
    /// that fetches it; one that has one is told the line to add.
    pub(crate) fn new_code_for_zone(&mut self, entity: EntityId) {
        let Some(zone_name) = self
            .open
            .document
            .level()
            .get(entity)
            .map(|e| e.name.clone())
        else {
            return;
        };
        if self.open.project.is_none() {
            self.log(
                crate::console::LogLevel::Warning,
                "Open or make a project first: a zone's code is a file in it.",
            );
            return;
        }
        let taken = |p: &str| {
            self.open
                .library
                .iter()
                .any(|i| i.path.eq_ignore_ascii_case(p))
        };
        let registered = |n: &str| self.open.document.registry.get(n).is_some();
        let base = component_name(&zone_name);
        let (path, name) = (1..)
            .map(|n| {
                let name = if n == 1 {
                    base.clone()
                } else {
                    format!("{base}{n}")
                };
                let mut file = name.clone();
                if let Some(first) = file.get(..1) {
                    file.replace_range(..1, &first.to_ascii_lowercase());
                }
                (format!("{file}.lst"), name)
            })
            .find(|(p, n)| !taken(p) && !registered(n))
            .unwrap_or_default();
        let fetch = format!("fetch \"{}\":{name};", path.trim_end_matches(".lst"));
        let mut actions = vec![crate::workbench::WorkbenchAction::NewTextFile {
            path: path.clone().into(),
            text: zone_code(&name, &path, &zone_name),
        }];
        if taken(crate::workbench_library::MAIN) {
            self.log(
                crate::console::LogLevel::Info,
                format!(
                    "Made {path} for {zone_name}. Add this line to main.lst so the game runs it: \
                     {fetch}"
                ),
            );
        } else {
            actions.push(crate::workbench::WorkbenchAction::NewTextFile {
                path: crate::workbench_library::MAIN.into(),
                text: format!(
                    "-- main.lst: where your game's code starts. Other code files are fetched \
                     from here.\n{fetch}\nend\n"
                ),
            });
        }
        let add = level::Edit::AddComponent {
            entity,
            component: name.clone(),
            data: reflect::ComponentData::default(),
        };
        self.command(&format!("Add {name}"), vec![add], None, None);
        self.library_request(
            crate::library::LibraryRequest::Open(crate::library::LibraryKind::Code, path),
            &mut actions,
        );
        self.open.outbox.extend(actions);
    }

    /// What the Trigger zone card shows of the zone Properties shows:
    /// which shapes the project offers, and who is inside while playing.
    pub(crate) fn zone_facts(&self) -> crate::zones::card::ZoneFacts {
        use crate::launcher::ProjectKind;
        let flat = match self.open.kind {
            ProjectKind::TwoD => true,
            ProjectKind::ThreeD => false,
            ProjectKind::Both => self.viewport.camera.kind == crate::camera::ViewKind::TwoD,
        };
        let inside = self.open.playing.as_ref().and_then(|playing| {
            let zone = self.open.document.selection.primary()?;
            Some(
                playing
                    .world
                    .inside(zone)
                    .iter()
                    .map(|who| {
                        playing
                            .world
                            .get::<world::Name>(*who)
                            .map_or_else(|| who.to_string(), |n| n.0)
                    })
                    .collect(),
            )
        });
        crate::zones::card::ZoneFacts {
            flat,
            inside,
            tags: self.open.tags.clone(),
        }
    }
}
