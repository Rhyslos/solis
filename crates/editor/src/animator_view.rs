//! The animator tab drawn (Phase 9, D3b; the editing itself is
//! [`crate::animator_editor`], the canvas [`crate::graph_canvas`]), as
//! the project lead approved it: parameters and a preview on the left, the
//! graph of states and transitions on the right; and Properties for the
//! state or transition picked. The state the preview (or, while playing,
//! the selected entity) is in glows blue.

use crate::animator_editor::{AnimatorEditor, BOX_SIZE, Picked};
use crate::graph_canvas::{GraphBox, GraphEvent, GraphTarget, GraphWire, graph_canvas};
use crate::sprite_sheets::SpriteSheets;
use crate::theme;
use animation::{ClipSet, Condition, Facing, ParamValue, ParameterKind, Plays};
use egui::{Color32, RichText, Ui, pos2, vec2};
use reflect::{AssetId, Id};
use std::collections::HashMap;

/// What the tab and its Properties draw from.
pub(crate) struct AnimatorUi<'a> {
    /// The file open.
    pub(crate) editor: &'a mut AnimatorEditor,
    /// The project's sheets and clips files, and their clips.
    pub(crate) sheets: &'a SpriteSheets,
    /// Each Aseprite file's atlas as a picture, by path.
    pub(crate) pictures: &'a HashMap<String, egui::TextureHandle>,
    /// While playing, the selected entity's animator if it runs this file:
    /// its state and its parameters' values.
    pub(crate) live: Option<(String, Vec<(String, ParamValue)>)>,
    /// What was put right reading the file, and what in it can't work.
    pub(crate) notes: Vec<String>,
    /// What's picked is drawn in this.
    pub(crate) accent: Color32,
}

/// The ID a box has on the canvas: a state's own, or 0 for Any state.
fn box_id(state: Option<Id>) -> u64 {
    state.map_or(0, Id::raw)
}

fn state_of_box(id: u64) -> Option<Id> {
    Id::from_raw(id)
}

/// A direction's arrow and name, for lists.
pub(crate) fn facing_text(f: Facing) -> String {
    let arrow = match f {
        Facing::Up => "↑",
        Facing::UpRight => "↗",
        Facing::Right => "→",
        Facing::DownRight => "↘",
        Facing::Down => "↓",
        Facing::DownLeft => "↙",
        Facing::Left => "←",
        Facing::UpLeft => "↖",
    };
    format!("{arrow} {}", f.word())
}

impl AnimatorUi<'_> {
    /// The clips the file's sheet offers.
    fn clips(&self) -> Option<&ClipSet> {
        self.editor
            .file
            .sheet
            .and_then(|id| self.sheets.clips(id))
            .map(|c| &**c)
    }

    /// The state that glows: the game's, while playing, or the preview's.
    fn glowing(&self) -> String {
        match &self.live {
            Some((state, _)) => state.clone(),
            None => self
                .editor
                .running
                .state_name(self.editor.preview.state)
                .to_owned(),
        }
    }
}

/// What a state plays, in a few words, and its chip.
fn state_line(state: &animation::State, clips: Option<&ClipSet>) -> (String, Option<String>) {
    let (first, chip) = match &state.plays {
        Plays::Clip(c) if c.is_empty() => ("no clip".to_owned(), None),
        Plays::Clip(c) => (c.clone(), None),
        Plays::ByDirection { clips: by, .. } => {
            let first = by
                .iter()
                .find(|(d, _)| *d == Facing::Down)
                .or(by.first())
                .map_or_else(|| "no clips".to_owned(), |(_, c)| format!("{c} …"));
            let n = by.len();
            (
                first,
                Some(format!("{n} direction{}", if n == 1 { "" } else { "s" })),
            )
        }
    };
    let once = match &state.plays {
        Plays::Clip(c) => clips.and_then(|s| s.get(c)).is_some_and(|c| c.repeats == 1),
        Plays::ByDirection { clips: by, .. } => by
            .first()
            .and_then(|(_, c)| clips.and_then(|s| s.get(c)))
            .is_some_and(|c| c.repeats == 1),
    };
    let mut line = first;
    if !state.speed_by.is_empty() {
        line.push_str(&format!(" · ×{}", state.speed_by));
    } else if once {
        line.push_str(" · once");
    }
    (line, chip)
}

/// The animator tab. Returns whether the file changed (and must be saved).
pub(crate) fn animator_tab(ui: &mut Ui, a: &mut AnimatorUi<'_>) -> bool {
    let mut changed = false;
    let full = ui.available_rect_before_wrap();
    let side = 220.0_f32.min(full.width() * 0.4);
    let left = egui::Rect::from_min_size(full.min, vec2(side, full.height()));
    let right = egui::Rect::from_min_max(pos2(full.min.x + side, full.min.y), full.max);
    let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left.shrink2(vec2(8.0, 6.0))));
    egui::ScrollArea::vertical()
        .id_salt("animator side")
        .auto_shrink(false)
        .show(&mut left_ui, |ui| {
            changed |= parameters(ui, a);
            ui.add_space(10.0);
            preview(ui, a);
        });
    ui.painter().vline(
        right.left(),
        right.y_range(),
        egui::Stroke::new(1.0, theme::LINE),
    );
    let mut graph_ui = ui.new_child(egui::UiBuilder::new().max_rect(right));
    changed |= graph(&mut graph_ui, a);
    ui.advance_cursor_after_rect(full);
    changed
}

/// The parameters, each with its kind and the value the preview (or,
/// while playing, the game) has.
fn parameters(ui: &mut Ui, a: &mut AnimatorUi<'_>) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Parameters").color(theme::TEXT_WEAK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button("+", |ui| {
                for (label, kind) in [
                    ("Number", ParameterKind::Number(0.0)),
                    ("Switch", ParameterKind::Switch(false)),
                    ("Trigger", ParameterKind::Trigger),
                    ("Direction", ParameterKind::Direction(Facing::Down)),
                ] {
                    if ui.button(label).clicked() {
                        changed |= a.editor.add_parameter(kind);
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("Add a parameter");
        });
    });
    ui.add_space(4.0);
    let params = a.editor.file.parameters.clone();
    if params.is_empty() {
        ui.label(
            RichText::new("None yet. Add a switch (moving), a number (speed), a trigger (use) or a direction (facing) for transitions and states to follow.")
                .color(theme::TEXT_WEAK),
        );
    }
    let live = a.live.clone();
    for p in &params {
        let value = match &live {
            Some((_, values)) => values.iter().find(|(n, _)| *n == p.name).map(|(_, v)| *v),
            None => a
                .editor
                .running
                .parameter(&p.name)
                .and_then(|i| a.editor.preview.values.get(i).copied()),
        };
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(96.0);
                let id = ui.id().with(("param name", p.id));
                let mut name: String = ui
                    .data(|d| d.get_temp(id))
                    .unwrap_or_else(|| p.name.clone());
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .frame(egui::Frame::NONE)
                        .desired_width(92.0),
                );
                if edit.has_focus() {
                    ui.data_mut(|d| d.insert_temp(id, name.clone()));
                } else {
                    ui.data_mut(|d| d.remove::<String>(id));
                }
                if edit.lost_focus() && name != p.name {
                    changed |= a.editor.rename_parameter(&p.name, &name);
                }
                ui.label(RichText::new(p.kind.word()).small().color(theme::TEXT_WEAK));
                edit.context_menu(|ui| {
                    if ui.button("Start at this value").clicked() {
                        let start = match (p.kind, value) {
                            (ParameterKind::Number(_), Some(ParamValue::Number(n))) => {
                                Some(ParameterKind::Number(n))
                            }
                            (ParameterKind::Switch(_), Some(ParamValue::Switch(b))) => {
                                Some(ParameterKind::Switch(b))
                            }
                            (ParameterKind::Direction(_), Some(ParamValue::Direction(v))) => {
                                Some(ParameterKind::Direction(Facing::nearest(v)))
                            }
                            _ => None,
                        };
                        if let Some(k) = start {
                            changed |= a.editor.set_parameter_start(&p.name, k);
                        }
                        ui.close();
                    }
                    if ui.button("Delete").clicked() {
                        changed |= a.editor.delete_parameter(&p.name);
                        ui.close();
                    }
                });
            });
            ui.add_enabled_ui(live.is_none(), |ui| {
                parameter_value(ui, a.editor, &p.name, value);
            });
        });
        ui.add_space(2.0);
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new(if live.is_some() {
            "The selected entity's values in the running game."
        } else {
            "Set them here to try the animator in the preview; the game starts each at the value it's made with (right-click a name to change that)."
        })
        .small()
        .color(theme::TEXT_WEAK),
    );
    changed
}

/// A parameter's value widget for the preview.
fn parameter_value(
    ui: &mut Ui,
    editor: &mut AnimatorEditor,
    name: &str,
    value: Option<ParamValue>,
) {
    match value {
        Some(ParamValue::Switch(mut on)) => {
            let r = ui.checkbox(&mut on, "");
            r.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, name)
            });
            if r.changed() {
                editor.set_preview(name, ParamValue::Switch(on));
            }
        }
        Some(ParamValue::Number(mut n)) => {
            if ui
                .add(egui::DragValue::new(&mut n).speed(0.02).max_decimals(2))
                .changed()
            {
                editor.set_preview(name, ParamValue::Number(n));
            }
        }
        Some(ParamValue::Trigger(fired)) => {
            let r = ui.add_enabled(!fired, egui::Button::new("Fire"));
            r.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Fire {name}"))
            });
            if r.clicked() {
                editor.set_preview(name, ParamValue::Trigger(true));
            }
        }
        Some(ParamValue::Direction(v)) => {
            // A dial: click or drag round it to turn.
            let (rect, r) = ui.allocate_exact_size(vec2(40.0, 40.0), egui::Sense::click_and_drag());
            let c = rect.center();
            let p = ui.painter();
            p.circle_stroke(c, 18.0, egui::Stroke::new(1.0, theme::BUTTON_BORDER));
            p.line_segment(
                [c, c + vec2(v[0], -v[1]) * 16.0],
                egui::Stroke::new(2.0, crate::graph_canvas::LIVE),
            );
            if (r.clicked() || r.dragged())
                && let Some(at) = r.interact_pointer_pos()
            {
                let d = at - c;
                editor.set_preview(name, ParamValue::Direction([d.x, -d.y]));
            }
            ui.label(RichText::new(Facing::nearest(v).word()).color(theme::TEXT_WEAK));
        }
        None => {}
    }
}

/// The sheet's frame the preview shows, and what it's playing.
fn preview(ui: &mut Ui, a: &mut AnimatorUi<'_>) {
    egui::Frame::new()
        .fill(theme::FIELD)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let sheet_name = a
                .editor
                .file
                .sheet
                .and_then(|id| a.sheets.paths.get(&id))
                .map(|p| p.rsplit_once('/').map_or(p.as_str(), |(_, n)| n).to_owned());
            ui.label(
                RichText::new(match &sheet_name {
                    Some(n) => format!("Preview · {n}"),
                    None => "Preview".to_owned(),
                })
                .small()
                .color(theme::TEXT_WEAK),
            );
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 140.0), egui::Sense::hover());
            let sheet_id = a.editor.file.sheet;
            let sheet = sheet_id.and_then(|id| a.sheets.get(id));
            let atlas = sheet_id
                .and_then(|id| aseprite_path(a.sheets, id))
                .and_then(|path| a.pictures.get(&path));
            let clip = a
                .clips()
                .and_then(|c| c.chosen(&a.editor.preview_clip))
                .cloned();
            let frame = clip.as_ref().and_then(|c| c.at(a.editor.preview_time));
            match (sheet, atlas, &frame) {
                (Some(sheet), Some(atlas), Some(m)) => {
                    crate::sheet_view::draw_frame(
                        ui.painter(),
                        rect,
                        sheet,
                        atlas,
                        m.frame as usize,
                    );
                }
                _ => {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        if sheet_id.is_none() {
                            "Choose the sprite sheet in Properties"
                        } else {
                            "No clip to show"
                        },
                        egui::FontId::proportional(11.0),
                        theme::TEXT_WEAK,
                    );
                }
            }
            let state = a
                .editor
                .running
                .state_name(a.editor.preview.state)
                .to_owned();
            let mut caption = format!("● {state}");
            if !a.editor.preview_clip.is_empty() {
                caption.push_str(&format!(" · {}", a.editor.preview_clip));
            }
            if let Some(m) = frame {
                caption.push_str(&format!(" · frame {}", m.index + 1));
            }
            ui.label(
                RichText::new(caption)
                    .small()
                    .color(crate::graph_canvas::LIVE),
            );
            if ui.small_button("Start again").clicked() {
                a.editor.restart_preview();
            }
        });
}

/// The Aseprite file's path behind a sheet or clips file.
fn aseprite_path(sheets: &SpriteSheets, id: AssetId) -> Option<String> {
    let path = sheets.paths.get(&id)?;
    if sheets.by_path.contains_key(path) {
        return Some(path.clone());
    }
    let file = sheets.frames_file(id)?;
    sheets.paths.get(&file.sheet?).cloned()
}

/// The graph: states as boxes, transitions as arrows.
fn graph(ui: &mut Ui, a: &mut AnimatorUi<'_>) -> bool {
    let mut changed = false;
    let glowing = a.glowing();
    let clips = a.clips().cloned();
    let file = &a.editor.file;
    let picked = &a.editor.picked;
    let is_picked = |id: Option<Id>| matches!(picked, Picked::States(s) if s.contains(&id));
    let mut boxes: Vec<GraphBox> = file
        .states
        .iter()
        .map(|s| {
            let (line, chip) = state_line(s, clips.as_ref());
            GraphBox {
                id: box_id(Some(s.id)),
                place: pos2(s.place[0], s.place[1]),
                size: vec2(BOX_SIZE[0], BOX_SIZE[1]),
                title: s.name.clone(),
                line,
                chip,
                picked: is_picked(Some(s.id)),
                live: s.name == glowing,
                start: file.entry_state().is_some_and(|e| e.id == s.id),
                movable: true,
                takes_arrows: true,
            }
        })
        .collect();
    boxes.push(GraphBox {
        id: box_id(None),
        place: pos2(file.any_place[0], file.any_place[1]),
        size: vec2(BOX_SIZE[0], BOX_SIZE[1]),
        title: "Any state".into(),
        line: "from every state".into(),
        chip: None,
        picked: is_picked(None),
        live: false,
        start: false,
        movable: true,
        takes_arrows: false,
    });
    let wires: Vec<GraphWire> = file
        .transitions
        .iter()
        .map(|t| GraphWire {
            id: t.id.raw(),
            from: box_id(t.from),
            to: box_id(Some(t.to)),
            label: if t.when.is_empty() {
                "always".to_owned()
            } else {
                t.when
                    .iter()
                    .map(Condition::text)
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            picked: matches!(picked, Picked::Transition(p) if *p == t.id),
        })
        .collect();
    let mut asked: Option<MenuAsk> = None;
    let editor = &mut *a.editor;
    let events = {
        let mut menu = |ui: &mut Ui, target: Option<GraphTarget>| match target {
            None => {
                if ui.button("Add state").clicked() {
                    asked = Some(MenuAsk::AddState);
                    ui.close();
                }
            }
            Some(GraphTarget::Box(b)) => {
                if ui.button("Make transition").clicked() {
                    asked = Some(MenuAsk::Aim(b));
                    ui.close();
                }
                if let Some(id) = state_of_box(b) {
                    if ui.button("Start here").clicked() {
                        asked = Some(MenuAsk::Entry(id));
                        ui.close();
                    }
                    if ui.button("Delete").clicked() {
                        asked = Some(MenuAsk::Delete(GraphTarget::Box(b)));
                        ui.close();
                    }
                }
            }
            Some(GraphTarget::Wire(w)) => {
                if ui.button("Delete").clicked() {
                    asked = Some(MenuAsk::Delete(GraphTarget::Wire(w)));
                    ui.close();
                }
            }
        };
        graph_canvas(ui, &mut editor.canvas, &boxes, &wires, a.accent, &mut menu)
    };
    for event in events {
        match event {
            GraphEvent::Pick(GraphTarget::Box(b), add) => editor.pick_state(state_of_box(b), add),
            GraphEvent::Pick(GraphTarget::Wire(w), _) => {
                if let Some(id) = Id::from_raw(w) {
                    editor.picked = Picked::Transition(id);
                }
            }
            GraphEvent::PickBoxes(ids, add) => {
                let mut set = match (&editor.picked, add) {
                    (Picked::States(s), true) => s.clone(),
                    _ => Default::default(),
                };
                set.extend(ids.into_iter().map(state_of_box));
                editor.picked = if set.is_empty() {
                    Picked::Nothing
                } else {
                    Picked::States(set)
                };
            }
            GraphEvent::Clear => editor.picked = Picked::Nothing,
            GraphEvent::Moved(ids, by) => {
                let ids: Vec<Option<Id>> = ids.into_iter().map(state_of_box).collect();
                changed |= editor.move_states(&ids, [by.x, by.y]);
            }
            GraphEvent::Join(from, to) => {
                if let Some(to) = state_of_box(to) {
                    changed |= editor.join(state_of_box(from), to);
                }
            }
            GraphEvent::Delete => changed |= editor.delete_picked(),
            GraphEvent::Edit(b) => editor.pick_state(state_of_box(b), false),
        }
    }
    match asked {
        Some(MenuAsk::AddState) => {
            let at = editor.canvas.pointer;
            changed |= editor.add_state([at.x, at.y]);
        }
        Some(MenuAsk::Aim(b)) => editor.canvas.aim_from(b),
        Some(MenuAsk::Entry(id)) => changed |= editor.set_entry(id),
        Some(MenuAsk::Delete(target)) => {
            editor.picked = match target {
                GraphTarget::Box(b) => Picked::States([state_of_box(b)].into()),
                GraphTarget::Wire(w) => Id::from_raw(w).map_or(Picked::Nothing, Picked::Transition),
            };
            changed |= editor.delete_picked();
        }
        None => {}
    }
    if file_is_empty(editor) {
        let r = ui.max_rect();
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            "Right-click to add a state",
            egui::FontId::proportional(13.0),
            theme::TEXT_WEAK,
        );
    }
    changed
}

fn file_is_empty(editor: &AnimatorEditor) -> bool {
    editor.file.states.is_empty()
}

/// What a right-click menu asked for.
enum MenuAsk {
    AddState,
    Aim(u64),
    Entry(Id),
    Delete(GraphTarget),
}

// Properties.

/// Properties while the animator tab is in front: the state or transition
/// picked, or the file itself. Returns whether the file changed.
pub(crate) fn animator_properties(ui: &mut Ui, a: &mut AnimatorUi<'_>) -> bool {
    if let Some(state) = a.editor.picked_state().cloned() {
        return state_properties(ui, a, &state);
    }
    if let Some(t) = a.editor.picked_transition().cloned() {
        return transition_properties(ui, a, &t);
    }
    file_properties(ui, a)
}

fn heading(ui: &mut Ui, kind: &str, name: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(kind).color(theme::TEXT_WEAK));
        ui.label(RichText::new("·").color(theme::TEXT_WEAK));
        ui.label(RichText::new(name).strong());
    });
    ui.add_space(4.0);
}

fn hint(ui: &mut Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
}

/// A clip picker of the sheet's clips. Returns the clip chosen.
fn clip_choice(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    current: &str,
    clips: Option<&ClipSet>,
) -> Option<String> {
    let mut chosen = None;
    let shown = if current.is_empty() { "—" } else { current };
    egui::ComboBox::from_id_salt(salt)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            if ui.selectable_label(current.is_empty(), "—").clicked() {
                chosen = Some(String::new());
            }
            for c in clips.map_or(&[][..], |c| &c.clips) {
                if ui.selectable_label(c.name == current, &c.name).clicked() {
                    chosen = Some(c.name.clone());
                }
            }
            if clips.is_none() {
                ui.label(RichText::new("Choose the animator's sprite sheet first (click empty space in the graph).").color(theme::TEXT_WEAK));
            }
        });
    if !current.is_empty() && clips.is_some_and(|c| c.get(current).is_none()) {
        ui.label(RichText::new("missing").color(theme::WARNING))
            .on_hover_text(format!("The sprite sheet has no clip called \"{current}\""));
    }
    chosen
}

fn state_properties(ui: &mut Ui, a: &mut AnimatorUi<'_>, state: &animation::State) -> bool {
    let mut changed = false;
    let clips = a.clips().cloned();
    let editor = &mut *a.editor;
    heading(ui, "State", &state.name);
    crate::inspector::row(
        ui,
        "Name",
        "What it's called, in the graph and in the game.",
        |ui| {
            let id = ui.id().with(("state name", state.id));
            let mut name: String = ui
                .data(|d| d.get_temp(id))
                .unwrap_or_else(|| state.name.clone());
            let r = ui.text_edit_singleline(&mut name);
            if r.has_focus() {
                ui.data_mut(|d| d.insert_temp(id, name.clone()));
            } else {
                ui.data_mut(|d| d.remove::<String>(id));
            }
            if r.lost_focus() && name != state.name {
                changed |= editor.rename_state(state.id, &name);
            }
        },
    );
    let by_direction = matches!(state.plays, Plays::ByDirection { .. });
    crate::inspector::row(
        ui,
        "Plays",
        "One clip, or a clip for each direction: the nearest to a direction parameter plays, keeping its place in the step when it turns.",
        |ui| {
            egui::ComboBox::from_id_salt(("plays", state.id))
                .selected_text(if by_direction {
                    "a clip by facing"
                } else {
                    "one clip"
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(!by_direction, "one clip").clicked() && by_direction {
                        changed |= editor.set_by_direction(state.id, false);
                    }
                    if ui
                        .selectable_label(by_direction, "a clip by facing")
                        .clicked()
                        && !by_direction
                    {
                        changed |= editor.set_by_direction(state.id, true);
                    }
                });
        },
    );
    match &state.plays {
        Plays::Clip(c) => {
            crate::inspector::row(
                ui,
                "Clip",
                "The clip it plays, from the animator's sprite sheet.",
                |ui| {
                    if let Some(new) = clip_choice(ui, ("clip", state.id), c, clips.as_ref()) {
                        changed |= editor.set_clip(state.id, &new);
                    }
                },
            );
        }
        Plays::ByDirection {
            by,
            clips: by_clips,
        } => {
            let directions: Vec<String> = editor
                .file
                .parameters
                .iter()
                .filter(|p| matches!(p.kind, ParameterKind::Direction(_)))
                .map(|p| p.name.clone())
                .collect();
            if directions.len() > 1 || !directions.contains(by) {
                crate::inspector::row(
                    ui,
                    "Facing by",
                    "The direction parameter that picks the clip.",
                    |ui| {
                        egui::ComboBox::from_id_salt(("by", state.id))
                            .selected_text(by.as_str())
                            .show_ui(ui, |ui| {
                                for d in &directions {
                                    if ui.selectable_label(d == by, d).clicked() {
                                        changed |= editor.set_direction_by(state.id, d);
                                    }
                                }
                            });
                    },
                );
            }
            ui.label(
                RichText::new("Clips by direction (the nearest to facing plays)")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
            for (facing, clip) in by_clips {
                crate::inspector::row(ui, &facing_text(*facing), "", |ui| {
                    if let Some(new) =
                        clip_choice(ui, ("dir", state.id, facing.word()), clip, clips.as_ref())
                    {
                        changed |= editor.set_direction_clip(state.id, *facing, &new);
                    }
                    let r = ui
                        .small_button("×")
                        .on_hover_text("Take this direction away");
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("Remove {}", facing.word()),
                        )
                    });
                    if r.clicked() {
                        changed |= editor.remove_direction(state.id, *facing);
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.add_space(100.0);
                if ui
                    .add_enabled(
                        by_clips.len() < Facing::ALL.len(),
                        egui::Button::new("+ Direction"),
                    )
                    .clicked()
                {
                    changed |= editor.add_direction(state.id);
                }
                ui.label(
                    RichText::new("(8 for diagonals)")
                        .small()
                        .color(theme::TEXT_WEAK),
                );
            });
        }
    }
    ui.add_space(4.0);
    crate::inspector::row(
        ui,
        "Speed",
        "How fast its clips play: 1 at their own speed.",
        |ui| {
            if let Some(v) = crate::frames_view::settled_drag(
                ui,
                "state speed",
                f64::from(state.speed),
                0.0..=16.0,
                0.01,
                " ×",
            ) {
                changed |= editor.set_state_speed(state.id, v as f32);
            }
        },
    );
    let numbers: Vec<String> = editor
        .file
        .parameters
        .iter()
        .filter(|p| matches!(p.kind, ParameterKind::Number(_)))
        .map(|p| p.name.clone())
        .collect();
    crate::inspector::row(
        ui,
        "Speed by",
        "A number parameter its speed is multiplied by (a walk that speeds up as the character does).",
        |ui| {
            let shown = if state.speed_by.is_empty() {
                "—"
            } else {
                state.speed_by.as_str()
            };
            egui::ComboBox::from_id_salt(("speed by", state.id))
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(state.speed_by.is_empty(), "—")
                        .clicked()
                    {
                        changed |= editor.set_speed_by(state.id, "");
                    }
                    for n in &numbers {
                        if ui.selectable_label(*n == state.speed_by, n).clicked() {
                            changed |= editor.set_speed_by(state.id, n);
                        }
                    }
                });
        },
    );
    let starts = editor.file.entry_state().is_some_and(|e| e.id == state.id);
    crate::inspector::row(
        ui,
        "Start",
        "Whether the animator starts in this state.",
        |ui| {
            if starts {
                ui.label(RichText::new("Starts here").color(theme::COLLIDER));
            } else if ui.button("Start here").clicked() {
                changed |= editor.set_entry(state.id);
            }
        },
    );
    hint(
        ui,
        "Click an arrow to set its conditions (all must hold) and the order they're tried in. Changes save at once.",
    );
    changed
}

/// One condition's widgets: the parameter (or "finished"), then how it's
/// tested. Returns the condition changed to.
fn condition_row(
    ui: &mut Ui,
    salt: (Id, usize),
    c: &Condition,
    params: &[animation::Parameter],
) -> Option<Condition> {
    let mut out = None;
    let current = c.parameter().unwrap_or(animation::animator_file::FINISHED);
    egui::ComboBox::from_id_salt(("cond param", salt.0, salt.1))
        .selected_text(current)
        .width(90.0)
        .show_ui(ui, |ui| {
            for p in params
                .iter()
                .filter(|p| !matches!(p.kind, ParameterKind::Direction(_)))
            {
                if ui.selectable_label(p.name == current, &p.name).clicked() && p.name != current {
                    out = Some(match p.kind {
                        ParameterKind::Number(_) => Condition::Above(p.name.clone(), 0.5),
                        _ => Condition::Is(p.name.clone()),
                    });
                }
            }
            if ui
                .selectable_label(matches!(c, Condition::Finished), "finished")
                .on_hover_text("The state's clip has played all its times")
                .clicked()
            {
                out = Some(Condition::Finished);
            }
        });
    let kind = params.iter().find(|p| p.name == current).map(|p| p.kind);
    match (c, kind) {
        (Condition::Is(n) | Condition::Not(n), Some(ParameterKind::Switch(_))) => {
            let on = matches!(c, Condition::Is(_));
            egui::ComboBox::from_id_salt(("cond on", salt.0, salt.1))
                .selected_text(if on { "is on" } else { "is off" })
                .width(60.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(on, "is on").clicked() && !on {
                        out = Some(Condition::Is(n.clone()));
                    }
                    if ui.selectable_label(!on, "is off").clicked() && on {
                        out = Some(Condition::Not(n.clone()));
                    }
                });
        }
        (Condition::Is(_), Some(ParameterKind::Trigger)) => {
            ui.label(RichText::new("fired").color(theme::TEXT_WEAK));
        }
        (Condition::Above(n, v) | Condition::Below(n, v), Some(ParameterKind::Number(_))) => {
            let above = matches!(c, Condition::Above(..));
            egui::ComboBox::from_id_salt(("cond cmp", salt.0, salt.1))
                .selected_text(if above { ">" } else { "<" })
                .width(36.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(above, ">").clicked() && !above {
                        out = Some(Condition::Above(n.clone(), *v));
                    }
                    if ui.selectable_label(!above, "<").clicked() && above {
                        out = Some(Condition::Below(n.clone(), *v));
                    }
                });
            if let Some(new) = crate::frames_view::settled_drag(
                ui,
                &format!("cond {} {}", salt.0, salt.1),
                f64::from(*v),
                -1.0e9..=1.0e9,
                0.01,
                "",
            ) {
                out = Some(if above {
                    Condition::Above(n.clone(), new as f32)
                } else {
                    Condition::Below(n.clone(), new as f32)
                });
            }
        }
        (Condition::Finished, _) => {}
        _ => {
            ui.label(RichText::new("never holds").color(theme::WARNING))
                .on_hover_text(
                    "It names a parameter that isn't there, or tests it in a way its kind can't be",
                );
        }
    }
    out
}

fn transition_properties(ui: &mut Ui, a: &mut AnimatorUi<'_>, t: &animation::Transition) -> bool {
    let mut changed = false;
    let editor = &mut *a.editor;
    let name = |id: Option<Id>| match id {
        None => "Any state".to_owned(),
        Some(id) => editor
            .file
            .state(id)
            .map_or_else(|| "?".to_owned(), |s| s.name.clone()),
    };
    heading(
        ui,
        "Transition",
        &format!("{} → {}", name(t.from), name(Some(t.to))),
    );
    ui.label(RichText::new("Taken when all of these hold:").color(theme::TEXT_WEAK));
    let params = editor.file.parameters.clone();
    for (i, c) in t.when.iter().enumerate() {
        ui.horizontal(|ui| {
            if let Some(new) = condition_row(ui, (t.id, i), c, &params) {
                changed |= editor.set_condition(t.id, i, new);
            }
            let r = ui
                .small_button("×")
                .on_hover_text("Take this condition away");
            r.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Remove condition {}", i + 1),
                )
            });
            if r.clicked() {
                changed |= editor.remove_condition(t.id, i);
            }
        });
    }
    if t.when.is_empty() {
        ui.label(
            RichText::new("None: it's taken at once.")
                .small()
                .color(theme::WARNING),
        );
    }
    if ui
        .add_enabled(
            t.when.len() < animation::animator_file::MAX_CONDITIONS,
            egui::Button::new("+ Condition"),
        )
        .clicked()
    {
        changed |= editor.add_condition(t.id);
    }
    ui.add_space(6.0);
    let same_from: Vec<Id> = editor
        .file
        .transitions
        .iter()
        .filter(|o| o.from == t.from)
        .map(|o| o.id)
        .collect();
    if same_from.len() > 1 {
        let at = same_from.iter().position(|id| *id == t.id).unwrap_or(0);
        crate::inspector::row(
            ui,
            "Tried",
            "When several transitions leave the same state, earlier ones are tried first.",
            |ui| {
                ui.label(format!("{} of {}", at + 1, same_from.len()));
                if ui
                    .add_enabled(at > 0, egui::Button::new("Earlier"))
                    .clicked()
                {
                    changed |= editor.move_transition(t.id, true);
                }
                if ui
                    .add_enabled(at + 1 < same_from.len(), egui::Button::new("Later"))
                    .clicked()
                {
                    changed |= editor.move_transition(t.id, false);
                }
            },
        );
    }
    hint(
        ui,
        "A transition from Any state is tried before the state's own, from every state but the one it goes to. At most one transition is taken each tick.",
    );
    changed
}

fn file_properties(ui: &mut Ui, a: &mut AnimatorUi<'_>) -> bool {
    let mut changed = false;
    let name = crate::sprite_sheets::name_of(&a.editor.path);
    heading(ui, "Animator", &name);
    let listed = a.sheets.listed();
    let current = a.editor.file.sheet;
    crate::inspector::row(
        ui,
        "Clips from",
        "The sprite sheet (or clips file) whose clips its states play: its clips are listed here and play in the preview. A sprite plays the clips of its own sheet.",
        |ui| {
            let shown = match current {
                None => "Choose…".to_owned(),
                Some(id) => listed
                    .iter()
                    .find(|(_, i)| *i == id)
                    .map_or_else(|| format!("Missing sheet {id}"), |(n, _)| n.clone()),
            };
            egui::ComboBox::from_id_salt("animator sheet")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (n, id) in &listed {
                        if ui.selectable_label(current == Some(*id), n).clicked() {
                            changed |= a.editor.set_sheet(Some(*id));
                        }
                    }
                    if listed.is_empty() {
                        ui.label("No sprite sheets yet: put an Aseprite file in the project.");
                    }
                });
        },
    );
    let f = &a.editor.file;
    ui.label(
        RichText::new(format!(
            "{} states · {} transitions · {} parameters",
            f.states.len(),
            f.transitions.len(),
            f.parameters.len()
        ))
        .color(theme::TEXT_WEAK),
    );
    for note in &a.notes {
        ui.label(RichText::new(note).small().color(theme::WARNING));
    }
    hint(
        ui,
        "Put it on an entity with an Animator component beside its Sprite. Click a state or an arrow to change it.",
    );
    changed
}

#[cfg(test)]
#[path = "animator_view_tests.rs"]
mod tests;
