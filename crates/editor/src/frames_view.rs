//! A clips file's shelf tabs drawn (Phase 9, D3; the editing itself is
//! [`crate::frames_editor`]; round 3, `04-sprite-clips-a.png`). Frames:
//! the file's clips in a column, and the chosen clip's frames along a
//! time ruler beside them, each
//! as wide as it lasts, the frames a ping-pong plays back faded after
//! them, a blue line where it's playing, and its events in a lane below.
//! Clip: the chosen one's speed, direction, repeats and whether it plays
//! first. New, Copy and Delete are on Animate's ribbon.

use crate::frames_editor::{FramesEditor, STANDARD_ZOOM, ZOOM_RANGE};
use crate::settled::{settled_drag, settled_number};
use crate::theme;
use animation::{ClipSet, Direction};
use assets::sprite_sheet::SpriteSheet;
use egui::{Color32, RichText, Ui};
use std::sync::Arc;

/// What the tabs draw from.
pub(crate) struct FramesUi<'a> {
    /// The file open.
    pub(crate) editor: &'a mut FramesEditor,
    /// Its sheet, if it's been read.
    pub(crate) sheet: Option<&'a Arc<SpriteSheet>>,
    /// The sheet's atlas as an egui picture.
    pub(crate) atlas: Option<&'a egui::TextureHandle>,
    /// The project's sheets, to choose the file's from.
    pub(crate) sheets: &'a crate::sprite_sheets::SpriteSheets,
    /// The project's sound events, by name, to choose a frame's event from.
    pub(crate) cue_sounds: &'a std::collections::HashMap<String, String>,
    /// What was put right reading the file, or doesn't fit its sheet.
    pub(crate) notes: Vec<String>,
}

/// The playing blue (the Play button's).
const PLAYHEAD: Color32 = Color32::from_rgb(0x2f, 0x8f, 0xd0);
/// An event's mark.
const EVENT: Color32 = Color32::from_rgb(0x7c, 0xc4, 0xe4);
/// A frame's tile, at most and at least this wide.
const TILE_WIDTH: (f32, f32) = (28.0, 600.0);
/// A frame's tile's height.
const TILE_HEIGHT: f32 = 118.0;
/// The ruler's marks are at least this far apart, in points.
const MARK_GAP: f32 = 8.0;
/// A time is written by a mark only this far from the last, in points.
const TIME_GAP: f32 = 24.0;

/// A frame being dragged to a new place in its clip.
#[derive(Debug, Clone, Copy)]
struct FrameDrag;

/// The file's name without its folder.
fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, n)| n)
}

/// "0.40 s".
fn seconds(s: f64) -> String {
    format!("{s:.2} s")
}

/// What a clip is, in a few words: "4 frames · 0.40 s" or "… · once".
fn about(clip: &animation::Clip) -> String {
    let frames = crate::library::frames_text(clip.frames.len());
    match clip.repeats {
        0 => format!("{frames} · {}", seconds(clip.pass_seconds())),
        1 => format!("{frames} · once"),
        n => format!("{frames} · ×{n}"),
    }
}

/// The Frames shelf tab's first column: the file's clips, searched, one
/// row each. New, Copy and Delete are on Animate's ribbon (round 3).
pub(crate) fn clips_list(ui: &mut Ui, f: &mut FramesUi<'_>) -> bool {
    let mut changed = false;
    let Some(sheet) = f.sheet else {
        ui.label(
            RichText::new("Choose the file's sheet on the Clip tab.").color(theme::text_weak()),
        );
        return false;
    };
    let clips = f.editor.clips(sheet);
    ui.add(
        egui::TextEdit::singleline(&mut f.editor.search)
            .hint_text("Search clips")
            .desired_width(f32::INFINITY),
    );
    let search = f.editor.search.to_lowercase();
    for (i, clip) in clips.clips.iter().enumerate() {
        if !search.is_empty() && !clip.name.to_lowercase().contains(&search) {
            continue;
        }
        changed |= clip_row(ui, f.editor, &clips, i, clip);
    }
    if clips.clips.is_empty() {
        ui.label(
            RichText::new(
                "No clips yet: tag runs of frames in Aseprite, or make one here with Clip › New.",
            )
            .color(theme::text_weak()),
        );
    }
    changed
}

/// The Clip shelf tab (round 3, `04-sprite-clip-tab-a.png`): the file and
/// its sheet, and the chosen clip's playback: speed, direction, how many
/// times it plays, whether it plays first.
pub(crate) fn clip_tab(ui: &mut Ui, f: &mut FramesUi<'_>) -> bool {
    let mut changed = false;
    ui.label(RichText::new(file_name(&f.editor.path)).color(theme::text_weak()));
    let Some(sheet) = f.sheet else {
        changed |= choose_sheet(ui, f);
        return changed;
    };
    let clips = f.editor.clips(sheet);
    match clips.get(&f.editor.clip).cloned() {
        Some(clip) => changed |= clip_settings(ui, f.editor, &clip),
        None => {
            ui.label(RichText::new("Choose a clip on the Frames tab.").color(theme::text_weak()));
        }
    }
    for note in clips.notes.iter().chain(&f.notes) {
        ui.label(RichText::new(note).size(11.0).color(theme::WARNING));
    }
    changed
}

/// The sheet picker, for a file whose sheet isn't chosen (or is gone).
fn choose_sheet(ui: &mut Ui, f: &mut FramesUi<'_>) -> bool {
    ui.label(
        RichText::new("Choose the sprite sheet (an Aseprite file) whose frames these clips play:")
            .color(theme::text_weak()),
    );
    let mut chosen = None;
    egui::ComboBox::from_id_salt("frames sheet")
        .selected_text("Choose…")
        .show_ui(ui, |ui| {
            let sheets = crate::workbench_frames::aseprite_sheets(f.sheets);
            for (name, id) in &sheets {
                if ui.selectable_label(false, name).clicked() {
                    chosen = Some(*id);
                }
            }
            if sheets.is_empty() {
                ui.label("No sprite sheets yet: put an Aseprite file in the project.");
            }
        });
    chosen.is_some_and(|id| f.editor.change(|file| file.sheet = Some(id)))
}

/// One clip's row in the list: chosen by a click, renamed (clips made
/// here) by a double-click.
fn clip_row(
    ui: &mut Ui,
    editor: &mut FramesEditor,
    clips: &ClipSet,
    i: usize,
    clip: &animation::Clip,
) -> bool {
    let mut changed = false;
    if let Some((old, typed)) = &mut editor.renaming
        && *old == clip.name
    {
        let response = ui.add(egui::TextEdit::singleline(typed).desired_width(f32::INFINITY));
        response.request_focus();
        if response.lost_focus() {
            let (old, typed) = editor.renaming.take().unwrap_or_default();
            if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                changed |= editor.rename(clips, &old, &typed);
            }
        }
        return changed;
    }
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::click());
    let chosen = editor.clip == clip.name;
    let painter = ui.painter();
    if chosen {
        painter.rect_filled(rect, 3.0, theme::kind::SPRITE.gamma_multiply(0.22));
    } else if response.hovered() {
        painter.rect_filled(rect, 3.0, theme::field());
    }
    let first = clips.first == Some(i);
    let y = rect.center().y;
    if first {
        painter.text(
            egui::pos2(rect.left() + 6.0, y),
            egui::Align2::LEFT_CENTER,
            "*",
            egui::FontId::proportional(theme::TEXT_SIZE),
            theme::kind::SPRITE,
        );
    }
    painter.text(
        egui::pos2(rect.left() + 22.0, y),
        egui::Align2::LEFT_CENTER,
        &clip.name,
        egui::FontId::proportional(theme::TEXT_SIZE),
        theme::text(),
    );
    let small = egui::FontId::proportional(11.0);
    painter.text(
        egui::pos2(rect.left() + rect.width() * 0.4, y),
        egui::Align2::LEFT_CENTER,
        about(clip),
        small.clone(),
        theme::text_weak(),
    );
    painter.text(
        egui::pos2(rect.right() - 6.0, y),
        egui::Align2::RIGHT_CENTER,
        if clip.from_aseprite {
            "Aseprite"
        } else {
            "made here"
        },
        small,
        theme::text_weak(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, chosen, &clip.name)
    });
    if response.clicked() {
        editor.choose(&clip.name);
    }
    if response.double_clicked() && !clip.from_aseprite {
        editor.renaming = Some((clip.name.clone(), clip.name.clone()));
    }
    response.on_hover_text(if clip.from_aseprite {
        "An Aseprite tag: its frames, times and direction come from Aseprite"
    } else {
        "Made here: double-click to rename"
    });
    changed
}

/// The chosen clip's settings.
fn clip_settings(ui: &mut Ui, editor: &mut FramesEditor, clip: &animation::Clip) -> bool {
    let mut changed = false;
    egui::Grid::new("clip settings")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label("Speed");
            if let Some(v) = settled_drag(
                ui,
                "clip speed",
                f64::from(clip.speed),
                0.0..=16.0,
                0.01,
                " ×",
            ) {
                changed |= editor.set_speed(v as f32);
            }
            ui.end_row();
            ui.label("Direction");
            if clip.from_aseprite {
                ui.label(
                    RichText::new(format!("{} (Aseprite)", clip.direction.name()))
                        .color(theme::text_weak()),
                );
            } else {
                let mut picked = None;
                egui::ComboBox::from_id_salt("clip direction")
                    .selected_text(clip.direction.name())
                    .show_ui(ui, |ui| {
                        for d in [
                            Direction::Forward,
                            Direction::Reverse,
                            Direction::PingPong,
                            Direction::PingPongReverse,
                        ] {
                            if ui.selectable_label(d == clip.direction, d.name()).clicked() {
                                picked = Some(d);
                            }
                        }
                    });
                if let Some(d) = picked {
                    changed |= editor.set_direction(d);
                }
            }
            ui.end_row();
            ui.label("Plays");
            if clip.from_aseprite {
                let times = match clip.repeats {
                    0 => "over and over".to_owned(),
                    1 => "once".to_owned(),
                    n => format!("{n} times"),
                };
                ui.label(RichText::new(format!("{times} (Aseprite)")).color(theme::text_weak()));
            } else {
                let most = f64::from(animation::frames_file::MAX_REPEATS);
                let (repeats, _) =
                    settled_number(ui, "clip repeats", f64::from(clip.repeats), |d| {
                        d.range(0.0..=most).speed(0.05).custom_formatter(|n, _| {
                            match n.round() as u32 {
                                0 => "over and over".to_owned(),
                                1 => "once".to_owned(),
                                n => format!("{n} times"),
                            }
                        })
                    });
                if let Some(repeats) = repeats {
                    changed |= editor.set_repeats(repeats.round() as u32);
                }
            }
            ui.end_row();
            ui.label("Plays first");
            let mut first = editor.file.first == clip.name;
            if ui
                .checkbox(&mut first, "")
                .on_hover_text("The clip a sprite plays when its Clip is left empty")
                .changed()
            {
                changed |= editor.toggle_first();
            }
            ui.end_row();
        });
    ui.add_space(4.0);
    if clip.from_aseprite {
        ui.label(
            RichText::new(format!(
                "{} is Aseprite's tag: change its frames there, or Copy it to change it here. \
                 Its speed and the events added here are Solis's own.",
                clip.name
            ))
            .size(11.0)
            .color(theme::text_weak()),
        );
    }
    changed
}

/// The Frames tab. Returns whether the file changed. Space and Delete are
/// acted on while the keys are the tab's (`keys`: the pointer rests over
/// it and nothing is being typed).
pub(crate) fn frames_tab(ui: &mut Ui, f: &mut FramesUi<'_>, keys: bool) -> bool {
    let Some(sheet) = f.sheet else {
        ui.label(
            RichText::new("Choose its sprite sheet in the Clips tab to see its frames.")
                .color(theme::text_weak()),
        );
        return false;
    };
    let clips = f.editor.clips(sheet);
    let Some(clip) = clips.get(&f.editor.clip).cloned() else {
        ui.label(RichText::new("Choose a clip in the Clips tab.").color(theme::text_weak()));
        return false;
    };
    let mut changed = false;
    let own = !clip.from_aseprite;
    if keys {
        let (space, delete) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::Space),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
            )
        });
        if space {
            toggle_play(f.editor, &clip);
        }
        if delete && own {
            changed |= f.editor.delete_frames();
        }
    }
    changed |= toolbar(ui, f, sheet, &clip);
    ui.add_space(4.0);
    egui::ScrollArea::horizontal()
        .id_salt("frames ruler")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            changed |= ruler(ui, f, sheet, &clip);
        });
    changed
}

/// Plays or pauses (from the start again once a clip played a number of
/// times has finished).
fn toggle_play(editor: &mut FramesEditor, clip: &animation::Clip) {
    if !editor.playing
        && clip
            .length_seconds()
            .is_some_and(|length| editor.time >= length)
    {
        editor.time = 0.0;
    }
    editor.playing = !editor.playing;
}

/// The Frames tab's tool bar: playing, adding and taking out frames, the
/// picked frames' time and event, and zoom.
fn toolbar(ui: &mut Ui, f: &mut FramesUi<'_>, sheet: &SpriteSheet, clip: &animation::Clip) -> bool {
    let mut changed = false;
    let own = !clip.from_aseprite;
    ui.horizontal_wrapped(|ui| {
        if ui
            .button("Start")
            .on_hover_text("Back to the start")
            .clicked()
        {
            f.editor.time = 0.0;
        }
        if ui
            .button("■ Stop")
            .on_hover_text("Stop, back at the start")
            .clicked()
        {
            f.editor.playing = false;
            f.editor.time = 0.0;
        }
        let play = egui::Button::new(if f.editor.playing {
            RichText::new("Pause")
        } else {
            RichText::new("▶ Play").color(theme::on_accent())
        })
        .fill(if f.editor.playing {
            theme::field()
        } else {
            theme::GO
        });
        if ui
            .add(play)
            .on_hover_text("Play or pause (Space)")
            .clicked()
        {
            toggle_play(f.editor, clip);
        }
        if ui
            .button("End")
            .on_hover_text("To the last frame")
            .clicked()
        {
            f.editor.playing = false;
            f.editor.time = clip
                .length_seconds()
                .unwrap_or_else(|| (clip.pass_seconds() - 0.001).max(0.0));
        }
        let now = clip.at(f.editor.time);
        ui.label(
            RichText::new(format!(
                "{} / {} · frame {}",
                seconds(f.editor.time),
                seconds(clip.length_seconds().unwrap_or(clip.pass_seconds())),
                now.map_or(0, |m| m.index + 1)
            ))
            .color(theme::text_weak()),
        );
        ui.separator();
        ui.add_enabled_ui(own, |ui| {
            let add = ui.menu_button("+ Add frames ▾", |ui| {
                ui.label(
                    RichText::new("Click the sheet's frames to add them after the picked one:")
                        .color(theme::text_weak()),
                );
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(360.0);
                            for n in 0..sheet.frames.len() {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(40.0, 48.0),
                                    egui::Sense::click(),
                                );
                                ui.painter().rect_filled(rect, 2.0, theme::field());
                                if let Some(atlas) = f.atlas {
                                    crate::sheet_view::draw_frame(
                                        ui.painter(),
                                        rect.shrink(3.0),
                                        sheet,
                                        atlas,
                                        n,
                                    );
                                }
                                ui.painter().text(
                                    rect.left_top() + egui::vec2(3.0, 1.0),
                                    egui::Align2::LEFT_TOP,
                                    (n + 1).to_string(),
                                    egui::FontId::proportional(10.0),
                                    theme::text_weak(),
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        true,
                                        format!("Add frame {}", n + 1),
                                    )
                                });
                                if response.clicked() {
                                    changed |= f.editor.add_frames(sheet, &[n as u32]);
                                }
                            }
                        });
                    });
                if ui.button("Add them all").clicked() {
                    let all: Vec<u32> = (0..sheet.frames.len() as u32).collect();
                    changed |= f.editor.add_frames(sheet, &all);
                    ui.close();
                }
            });
            add.response
                .on_disabled_hover_text("An Aseprite tag's frames are changed in Aseprite");
            if ui
                .add_enabled(!f.editor.selected.is_empty(), egui::Button::new("Delete"))
                .on_hover_text("Take the picked frames out of the clip (Delete)")
                .clicked()
            {
                changed |= f.editor.delete_frames();
            }
        });
        if let Some(&first) = f.editor.selected.first()
            && let Some(frame) = clip.frames.get(first)
        {
            ui.separator();
            let picked = f.editor.selected.len();
            ui.label(if picked == 1 {
                format!("Frame {}:", first + 1)
            } else {
                format!("{picked} frames:")
            });
            ui.label(RichText::new("time").color(theme::text_weak()));
            if own {
                if let Some(ms) = settled_drag(
                    ui,
                    "frame ms",
                    f64::from(frame.ms),
                    1.0..=f64::from(animation::frames_file::MAX_MS),
                    1.0,
                    " ms",
                ) {
                    changed |= f.editor.set_ms(ms as u32);
                }
            } else {
                ui.label(
                    RichText::new(format!("{} ms (Aseprite)", frame.ms)).color(theme::text_weak()),
                );
            }
            ui.label(RichText::new("event").color(theme::text_weak()));
            changed |= event_field(ui, f, &clips_now(f, sheet), frame);
        }
        ui.separator();
        ui.label(RichText::new("Zoom").color(theme::text_weak()));
        if ui
            .add_enabled(f.editor.zoom > ZOOM_RANGE.0, egui::Button::new("-"))
            .on_hover_text("Zoom out")
            .on_disabled_hover_text("Zoomed out as far as it goes")
            .clicked()
        {
            f.editor.zoom = (f.editor.zoom / 1.5).max(ZOOM_RANGE.0);
        }
        if ui
            .add_enabled(f.editor.zoom < ZOOM_RANGE.1, egui::Button::new("+"))
            .on_hover_text("Zoom in")
            .on_disabled_hover_text("Zoomed in as far as it goes")
            .clicked()
        {
            f.editor.zoom = (f.editor.zoom * 1.5).min(ZOOM_RANGE.1);
        }
        if (f.editor.zoom - STANDARD_ZOOM).abs() > 1.0 && ui.small_button("100%").clicked() {
            f.editor.zoom = STANDARD_ZOOM;
        }
    });
    changed
}

/// The clips as they are now (after a change earlier in the frame).
fn clips_now(f: &FramesUi<'_>, sheet: &SpriteSheet) -> ClipSet {
    f.editor.clips(sheet)
}

/// The picked frames' event: typed, or picked from the project's sound
/// events (a frame whose event names one plays it).
fn event_field(
    ui: &mut Ui,
    f: &mut FramesUi<'_>,
    clips: &ClipSet,
    frame: &animation::ClipFrame,
) -> bool {
    let mut changed = false;
    // Aseprite's own events on the frame stay; the editable one is Solis's.
    let own = clips.get(&f.editor.clip).is_some_and(|c| !c.from_aseprite);
    let solis_event = if own {
        frame.events.first().cloned().unwrap_or_default()
    } else {
        f.editor
            .file
            .tags
            .iter()
            .find(|t| t.name == f.editor.clip)
            .and_then(|t| t.events.iter().find(|(at, _)| *at == frame.frame))
            .map(|(_, e)| e.clone())
            .unwrap_or_default()
    };
    let id = ui.id().with(("event typed", &f.editor.clip, frame.frame));
    let mut typed: String = ui.data(|d| d.get_temp(id)).unwrap_or(solis_event.clone());
    let response = ui.add(
        egui::TextEdit::singleline(&mut typed)
            .hint_text("none")
            .desired_width(110.0),
    );
    if response.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, typed.clone()));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    if response.lost_focus() && typed.trim() != solis_event {
        changed |= f.editor.set_event(clips, &typed);
    }
    ui.menu_button("Sound events ▾", |ui| {
        ui.label(RichText::new("Sound events").color(theme::text_weak()));
        let sounds = crate::workbench_frames::sound_names(f.cue_sounds);
        for name in &sounds {
            if ui.button(name).clicked() {
                changed |= f.editor.set_event(clips, name);
                ui.close();
            }
        }
        if sounds.is_empty() {
            ui.label("None yet: an event named like a sound event plays it.");
        }
        ui.separator();
        if ui.button("No event").clicked() {
            changed |= f.editor.set_event(clips, "");
            ui.close();
        }
    })
    .response
    .on_hover_text("Pick a sound event: the frame plays it, and game code hears CueReached");
    let fixed: Vec<&String> = if own {
        Vec::new()
    } else {
        frame.events.iter().filter(|e| **e != solis_event).collect()
    };
    if !fixed.is_empty() {
        ui.label(
            RichText::new(format!(
                "and {} (Aseprite)",
                fixed
                    .iter()
                    .map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .color(theme::text_weak()),
        );
    }
    changed
}

/// The time ruler, the frames along it and their events.
fn ruler(ui: &mut Ui, f: &mut FramesUi<'_>, sheet: &SpriteSheet, clip: &animation::Clip) -> bool {
    let mut changed = false;
    let own = !clip.from_aseprite;
    let zoom = f.editor.zoom;
    let width_of = |ms: u32| (ms as f32 / 1000.0 * zoom).clamp(TILE_WIDTH.0, TILE_WIDTH.1);
    // One pass, as played: the frames, then those a ping-pong plays back.
    let order: Vec<usize> = clip.pass_order().collect();
    let total: f32 = order
        .iter()
        .map(|&i| width_of(clip.frames[i].ms) + 4.0)
        .sum();
    let height = 18.0 + TILE_HEIGHT + 8.0 + 24.0;
    let (area, _) = ui.allocate_exact_size(
        egui::vec2(total.max(ui.available_width()), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(area);
    let left = area.left() + 2.0;
    let ruler_y = area.top();
    // The ruler: a mark every 50 ms, a time every 100 ms.
    painter.hline(
        area.x_range(),
        ruler_y + 17.0,
        egui::Stroke::new(1.0, theme::button_border()),
    );
    let mut x = left;
    let mut marks = Vec::new();
    for &i in &order {
        marks.push(x);
        x += width_of(clip.frames[i].ms) + 4.0;
    }
    for mark in ruler_marks(&order, clip, &width_of, left, painter.clip_rect().x_range()) {
        painter.vline(
            mark.x,
            (ruler_y + 11.0)..=(ruler_y + 17.0),
            egui::Stroke::new(1.0, theme::button_border()),
        );
        if mark.time {
            painter.text(
                egui::pos2(mark.x + 2.0, ruler_y),
                egui::Align2::LEFT_TOP,
                seconds(f64::from(mark.ms) / 1000.0),
                egui::FontId::proportional(10.0),
                theme::text_weak(),
            );
        }
    }
    // The tiles.
    let tiles_y = ruler_y + 22.0;
    let mut drop_at = None;
    let dragging = egui::DragAndDrop::has_payload_of_type::<FrameDrag>(ui.ctx());
    for (step, (&i, &tx)) in order.iter().zip(&marks).enumerate() {
        let frame = &clip.frames[i];
        let w = width_of(frame.ms);
        let rect = egui::Rect::from_min_size(egui::pos2(tx, tiles_y), egui::vec2(w, TILE_HEIGHT));
        // Steps past the clip's own frames are played back (ping-pong).
        let ghost = step >= clip.frames.len();
        let id = ui.id().with(("frame tile", step));
        let sense = if ghost {
            egui::Sense::hover()
        } else if own {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::click()
        };
        let response = ui.interact(rect, id, sense);
        let picked = !ghost && f.editor.selected.contains(&i);
        let fade = if ghost { 0.45 } else { 1.0 };
        painter.rect_filled(rect, 4.0, theme::field().gamma_multiply(fade));
        let stroke = if picked {
            egui::Stroke::new(2.0, theme::kind::SPRITE)
        } else {
            egui::Stroke::new(1.0, theme::button_border().gamma_multiply(fade))
        };
        painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
        if let Some(atlas) = f.atlas {
            let inner = rect.shrink2(egui::vec2(4.0, 18.0));
            let tinted = ui.painter_at(rect).with_layer_id(ui.layer_id());
            crate::sheet_view::draw_frame(&tinted, inner, sheet, atlas, frame.frame as usize);
            if ghost {
                tinted.rect_filled(rect, 4.0, theme::panel().gamma_multiply(0.55));
            }
        }
        let text_color = if ghost {
            theme::text_weak().gamma_multiply(0.7)
        } else {
            theme::text()
        };
        painter.text(
            rect.left_top() + egui::vec2(5.0, 3.0),
            egui::Align2::LEFT_TOP,
            (i + 1).to_string(),
            egui::FontId::proportional(11.0),
            text_color,
        );
        painter.text(
            rect.right_bottom() - egui::vec2(5.0, 3.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("{} ms", frame.ms),
            egui::FontId::proportional(10.5),
            theme::text_weak().gamma_multiply(fade),
        );
        if ghost {
            continue;
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                picked,
                format!("Frame {} of {}", i + 1, clip.name),
            )
        });
        if response.clicked() {
            let m = ui.input(|inp| inp.modifiers);
            f.editor.pick(i, m.command, m.shift);
            f.editor.playing = false;
            f.editor.time = f64::from(ms_before(&order, clip, step)) / 1000.0;
        }
        if own {
            if response.drag_started() {
                if !f.editor.selected.contains(&i) {
                    f.editor.pick(i, false, false);
                }
                egui::DragAndDrop::set_payload(ui.ctx(), FrameDrag);
            }
            if dragging && response.contains_pointer() {
                let pointer = ui.input(|inp| inp.pointer.hover_pos()).unwrap_or_default();
                let before = pointer.x < rect.center().x;
                let line_x = if before {
                    rect.left() - 2.0
                } else {
                    rect.right() + 2.0
                };
                painter.vline(
                    line_x,
                    rect.y_range(),
                    egui::Stroke::new(2.0, theme::kind::SPRITE),
                );
                drop_at = Some(if before { i } else { i + 1 });
            }
        }
    }
    if let Some(to) = drop_at
        && ui.input(|i| i.pointer.any_released())
        && egui::DragAndDrop::take_payload::<FrameDrag>(ui.ctx()).is_some()
    {
        changed |= f.editor.move_frames(to);
    }
    // The events lane.
    let lane = egui::Rect::from_min_size(
        egui::pos2(area.left(), tiles_y + TILE_HEIGHT + 8.0),
        egui::vec2(area.width(), 22.0),
    );
    painter.rect_filled(lane, 3.0, theme::field().gamma_multiply(0.7));
    painter.text(
        lane.left_center() + egui::vec2(4.0, 0.0),
        egui::Align2::LEFT_CENTER,
        if order.iter().any(|&i| !clip.frames[i].events.is_empty()) {
            ""
        } else {
            "Events: none (pick a frame and give it one above)"
        },
        egui::FontId::proportional(10.5),
        theme::text_weak(),
    );
    for (step, (&i, &tx)) in order.iter().zip(&marks).enumerate() {
        let events = &clip.frames[i].events;
        if events.is_empty() {
            continue;
        }
        let fade = if step >= clip.frames.len() { 0.5 } else { 1.0 };
        painter.text(
            egui::pos2(tx + 2.0, lane.center().y),
            egui::Align2::LEFT_CENTER,
            format!("■ {}", events.join(", ")),
            egui::FontId::proportional(11.0),
            EVENT.gamma_multiply(fade),
        );
    }
    // Where it's playing.
    let pass_s = clip.pass_seconds();
    let into = if pass_s > 0.0 {
        if clip.length_seconds().is_some() {
            (f.editor.time % pass_s.max(1e-9)).min(pass_s)
        } else {
            f.editor.time % pass_s
        }
    } else {
        0.0
    };
    let ms = (into * 1000.0) as u32;
    let head = x_of_ms(&order, clip, &width_of, left, ms);
    painter.vline(
        head,
        ruler_y..=(lane.bottom()),
        egui::Stroke::new(2.0, PLAYHEAD),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(head - 5.0, ruler_y),
            egui::pos2(head + 5.0, ruler_y),
            egui::pos2(head, ruler_y + 8.0),
        ],
        PLAYHEAD,
        egui::Stroke::NONE,
    ));
    changed
}

/// A mark on the time ruler: where it is, how far into the pass, and
/// whether the time is written by it.
#[derive(Debug, Clone, PartialEq)]
struct Mark {
    x: f32,
    ms: u32,
    time: bool,
}

/// The steps the ruler can mark at, in milliseconds: the first that
/// keeps marks [`MARK_GAP`] apart in a tile, with times written at the
/// first of them (from 100 ms) that is a multiple of it and keeps times
/// [`TIME_GAP`] apart.
const MARK_STEPS: [u32; 10] = [
    50, 100, 200, 500, 1_000, 2_000, 5_000, 10_000, 20_000, 60_000,
];

/// The ruler's marks over one pass's tiles (`order`, the first starting
/// at `left`) that fall within `seen`, the part on screen: a mark every
/// 50 ms and a time every 100 ms where a tile is wide enough, and further
/// apart where a tile is too narrow for how long it lasts. Placed as the
/// playhead is ([`x_of_ms`]), walking the tiles once (the review, BUG-21:
/// every mark of the pass was drawn, each placed by walking every tile
/// before it, so a long clip's ruler alone could stop the editor).
fn ruler_marks(
    order: &[usize],
    clip: &animation::Clip,
    width_of: &dyn Fn(u32) -> f32,
    left: f32,
    seen: egui::Rangef,
) -> Vec<Mark> {
    let mut marks = Vec::new();
    let mut x = left;
    let mut start = 0;
    let mut steps = (Some(MARK_STEPS[0]), Some(MARK_STEPS[1]));
    for &i in order {
        let len = clip.frames[i].ms;
        let w = width_of(len);
        let per_ms = w / len.max(1) as f32;
        let wide = |ms: u32, gap: f32| ms as f32 * per_ms >= gap;
        let step = MARK_STEPS.into_iter().find(|&s| wide(s, MARK_GAP));
        let time = step.and_then(|step| {
            MARK_STEPS
                .into_iter()
                .find(|&t| t >= 100 && t % step == 0 && wide(t, TIME_GAP))
        });
        steps = (step, time);
        // The part of the tile on screen, in milliseconds from its start.
        let (from, to) = ((seen.min - x) / per_ms, (seen.max - x) / per_ms);
        if let Some(step) = step
            && to >= 0.0
            && from < len as f32
        {
            let mut ms = (start + from.max(0.0) as u32).div_ceil(step) * step;
            while ms < start + len && (ms - start) as f32 <= to {
                marks.push(Mark {
                    x: x + w * (ms - start) as f32 / len.max(1) as f32,
                    ms,
                    time: time.is_some_and(|t| ms % t == 0),
                });
                ms += step;
            }
        }
        start += len;
        x += w + 4.0;
    }
    // The pass's end, at the last tile's right edge.
    let (step, time) = steps;
    if step.is_some_and(|s| start % s == 0) && seen.contains(x - 4.0) {
        marks.push(Mark {
            x: x - 4.0,
            ms: start,
            time: time.is_some_and(|t| start % t == 0),
        });
    }
    marks
}

/// Milliseconds into the pass before step `step` starts.
fn ms_before(order: &[usize], clip: &animation::Clip, step: usize) -> u32 {
    order[..step].iter().map(|&i| clip.frames[i].ms).sum()
}

/// Where `ms` into the pass falls along the ruler, tiles being as wide as
/// they last (but never narrower or wider than a tile may be).
fn x_of_ms(
    order: &[usize],
    clip: &animation::Clip,
    width_of: &dyn Fn(u32) -> f32,
    left: f32,
    ms: u32,
) -> f32 {
    let mut x = left;
    let mut start = 0;
    for &i in order {
        let len = clip.frames[i].ms;
        let w = width_of(len);
        if ms < start + len {
            return x + w * (ms - start) as f32 / len.max(1) as f32;
        }
        start += len;
        x += w + 4.0;
    }
    x - 4.0
}

#[cfg(test)]
#[path = "frames_view_tests.rs"]
mod tests;
