//! A field holding one of the project's names (decision 0127): a sound,
//! a sprite clip, a picture, a module, a screen, a level or a tag, as its
//! option's name (`Sfx.Alert`), so a renamed file follows. The kit's
//! drop-down with the file's thumbnail or its kind's icon before the
//! name; its list has a box to find a name, the project's folders as
//! headings, and ▶ on a sound to hear it. A file dragged from the Library
//! is taken when it stands for one of the field's names.

use crate::icons::Icon;
use crate::library::{LibraryDrag, LibraryFiles, names};
use crate::theme;
use crate::tokens::{self, size, text};
use crate::widgets::field::{self, Lead};
use egui::{Align2, CornerRadius, Rect, Response, Sense, Stroke, Ui, pos2, vec2};
use reflect::{NameKind, Value};

/// The most names a list shows at once: type to find the rest.
const MAX_LISTED: usize = 200;

/// What the user asked of a name field.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NameAsk {
    /// Hold this name.
    Set(Value),
    /// Play the sound event at this path in the project, to hear it.
    Play(String),
}

/// The kind's icon and colour, before a name with no picture.
fn kind_lead(kind: NameKind) -> Lead {
    let (icon, colour) = match kind {
        NameKind::Sound => (Icon::SoundEvent, theme::kind::SOUND_EVENT),
        NameKind::Clip => (Icon::Clips, theme::kind::CLIPS),
        NameKind::Picture => (Icon::Picture, theme::kind::PICTURE),
        NameKind::Module => (Icon::Module, theme::kind::MODULE),
        NameKind::Screen => (Icon::Screen, theme::kind::CODE),
        NameKind::Level => (Icon::LevelFile, theme::kind::LEVEL),
        NameKind::Tag => (Icon::Key, theme::kind::OTHER),
        NameKind::Font => (Icon::Text, theme::kind::TEXT),
    };
    Lead::Icon(icon, colour)
}

/// What goes before `option`: its file's thumbnail, or the kind's icon.
fn lead_of(library: &LibraryFiles, kind: NameKind, option: &str) -> Lead {
    library
        .named(kind, option)
        .and_then(|item| item.thumbnail)
        .map_or_else(|| kind_lead(kind), Lead::Picture)
}

/// Whether `options[at]` is one a field holds rather than a folder of
/// them: an option with none inside it, or a file with parts inside it
/// (`is_file`: a picture and its crops).
fn pickable(options: &[String], at: usize, is_file: impl Fn(&str) -> bool) -> bool {
    let option = &options[at];
    let has_inside = options.get(at + 1).is_some_and(|next| {
        next.len() > option.len()
            && next.starts_with(option.as_str())
            && next[option.len()..].starts_with('.')
    });
    !has_inside || is_file(option)
}

/// The field: `value` (for several entities, `mixed`) among `options`,
/// the names of `kind`. `library` finds each name's file.
pub(crate) fn name_picker(
    ui: &mut Ui,
    (label, value, mixed): (&str, &Value, bool),
    (options, kind): (&[String], NameKind),
    library: &LibraryFiles,
) -> Option<NameAsk> {
    let current = match value {
        Value::Text(t) => t.as_str(),
        _ => "",
    };
    let shown = if mixed { "—" } else { current };
    let lead = if mixed {
        kind_lead(kind)
    } else {
        lead_of(library, kind, current)
    };
    let sound = kind == NameKind::Sound;
    let play_room = if sound {
        size::field_h() + size::GAP
    } else {
        0.0
    };
    let width = (ui.available_width() - play_room).max(size::FIELD_MIN_W);
    let button = field::dropdown_button_led(ui, label, shown, width, lead);
    let mut asked = dropped(ui, &button, (options, kind), (current, library));
    egui::Popup::menu(&button)
        .width(width.max(size::FIELD_MIN_W * 3.0))
        // ▶ in the list plays a sound and leaves it open.
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if let Some(ask) = list(ui, (options, kind), (current, mixed), library) {
                asked = Some(ask);
            }
        });
    if sound {
        let file = library.named(kind, current).filter(|_| !mixed);
        if play_button(ui, label, file.is_some()).clicked()
            && let Some(file) = file
        {
            asked = Some(NameAsk::Play(file.path.clone()));
        }
    }
    asked.filter(|ask| mixed || *ask != NameAsk::Set(value.clone()))
}

/// A Library file dragged over the field: the field lights up when the
/// file stands for one of its names, and letting go there sets it.
fn dropped(
    ui: &Ui,
    button: &Response,
    (options, kind): (&[String], NameKind),
    (current, library): (&str, &LibraryFiles),
) -> Option<NameAsk> {
    let hovered = button.dnd_hover_payload::<LibraryDrag>()?;
    let option = names::dropped_option(library, &hovered.0, kind, options)?;
    ui.painter().rect_stroke(
        button.rect,
        CornerRadius::same(size::radius::CONTROL),
        Stroke::new(size::LINE * 1.5, crate::interface_look::accent_fill()),
        egui::StrokeKind::Inside,
    );
    let tip = format!("Drop: {option}");
    egui::Tooltip::always_open(
        ui.ctx().clone(),
        ui.layer_id(),
        button.id.with("drop"),
        egui::PopupAnchor::Pointer,
    )
    .show(|ui| ui.label(tip));
    button
        .dnd_release_payload::<LibraryDrag>()
        .filter(|_| *option != current)
        .map(|_| NameAsk::Set(Value::Text(option.clone())))
}

/// The ▶ after a sound's field: plays its sound, when it has a file.
fn play_button(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    let side = size::field_h();
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    let response = response.on_hover_text("Play it, to hear it");
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, format!("Play {label}"))
    });
    if ui.is_rect_visible(rect) {
        let fill = if response.hovered() && enabled {
            tokens::hover()
        } else {
            tokens::card()
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(size::radius::CONTROL),
            fill,
            Stroke::new(size::LINE, tokens::field_border()),
            egui::StrokeKind::Inside,
        );
        let ink = if enabled {
            tokens::text_primary()
        } else {
            tokens::text_faint()
        };
        let icon = Rect::from_center_size(
            rect.center(),
            vec2(size::ICON_SMALL, size::ICON_SMALL) * 0.8,
        );
        crate::icons::icon(ui.painter(), icon, Icon::Play, ink);
    }
    response
}

/// The open list: a box to find a name, then the names under their
/// folders' headings, each with its picture or icon, and ▶ on a sound.
fn list(
    ui: &mut Ui,
    (options, kind): (&[String], NameKind),
    (current, mixed): (&str, bool),
    library: &LibraryFiles,
) -> Option<NameAsk> {
    let find_id = ui.id().with("find");
    let mut find: String = ui.data(|d| d.get_temp(find_id)).unwrap_or_default();
    let width = ui.available_width();
    let typed = field::text_field(ui, "Find a name", &mut find, "Find a name", width);
    if !typed.has_focus() && find.is_empty() {
        typed.request_focus();
    }
    let wanted = find.to_lowercase();
    let mut asked = None;
    let mut heading = None;
    let mut listed = 0usize;
    let mut found = 0usize;
    let is_file = |option: &str| kind == NameKind::Picture && library.has(kind, option);
    let pickable: Vec<usize> = (0..options.len())
        .filter(|&at| pickable(options, at, is_file))
        .collect();
    egui::ScrollArea::vertical()
        .max_height(field::LIST_MAX_H)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for &at in &pickable {
                let option = &options[at];
                if !wanted.is_empty() && !option.to_lowercase().contains(&wanted) {
                    continue;
                }
                found += 1;
                if listed == MAX_LISTED {
                    continue;
                }
                listed += 1;
                let (folder, name) = option.rsplit_once('.').unwrap_or(("", option));
                if heading != Some(folder) && !folder.is_empty() {
                    field::list_note(ui, &folder.replace('.', " › "));
                }
                heading = Some(folder);
                let row = Row {
                    words: name,
                    lead: lead_of(library, kind, option),
                    chosen: option == current && !mixed,
                    play: (kind == NameKind::Sound)
                        .then(|| library.named(kind, option))
                        .flatten()
                        .map(|f| f.path.as_str()),
                };
                match row.show(ui) {
                    Some(RowAsk::Pick) => {
                        asked = Some(NameAsk::Set(Value::Text(option.clone())));
                        ui.close();
                    }
                    Some(RowAsk::Play(path)) => asked = Some(NameAsk::Play(path.to_owned())),
                    None => {}
                }
            }
            let what = if found == pickable.len() {
                format!("{} names · drag one from the Library too", pickable.len())
            } else {
                format!("{found} of {} names", pickable.len())
            };
            let what = if found > MAX_LISTED {
                format!("Only the first {MAX_LISTED}: type to find others.")
            } else {
                what
            };
            field::list_note(ui, &what);
        });
    ui.data_mut(|d| d.insert_temp(find_id, find));
    asked
}

/// What a row of the list asked for.
enum RowAsk<'a> {
    Pick,
    Play(&'a str),
}

/// One row of the list.
struct Row<'a> {
    words: &'a str,
    lead: Lead,
    chosen: bool,
    /// The sound's file, for its ▶.
    play: Option<&'a str>,
}

impl<'a> Row<'a> {
    fn show(&self, ui: &mut Ui) -> Option<RowAsk<'a>> {
        let width = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, size::MENU_ROW_H), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::RadioButton,
                ui.is_enabled(),
                self.chosen,
                self.words,
            )
        });
        let play_rect = Rect::from_min_size(
            pos2(rect.right() - size::MENU_ROW_H, rect.top()),
            vec2(size::MENU_ROW_H, size::MENU_ROW_H),
        );
        let play = self.play.map(|path| {
            let id = response.id.with("play");
            let r = ui.interact(play_rect, id, Sense::click());
            r.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Play {}", self.words),
                )
            });
            (r, path)
        });
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if self.chosen {
                painter.rect_filled(
                    rect,
                    CornerRadius::same(size::radius::SMALL),
                    crate::interface_look::selected(),
                );
            } else if response.hovered() {
                painter.rect_filled(
                    rect,
                    CornerRadius::same(size::radius::SMALL),
                    tokens::hover(),
                );
            }
            let room = rect.shrink2(vec2(size::FIELD_PAD, 0.0));
            let lead = Rect::from_center_size(
                pos2(room.left() + size::LEAD / 2.0, room.center().y),
                vec2(size::LEAD, size::LEAD),
            );
            self.lead.paint(painter, lead);
            let ink = if self.chosen {
                tokens::text_emphasis()
            } else {
                tokens::text_body()
            };
            painter.text(
                pos2(lead.right() + size::GAP, room.center().y),
                Align2::LEFT_CENTER,
                self.words,
                text::row(),
                ink,
            );
            if let Some((r, _)) = &play {
                let ink = if r.hovered() {
                    tokens::text_primary()
                } else {
                    tokens::text_meta()
                };
                let icon = Rect::from_center_size(
                    play_rect.center(),
                    vec2(size::ICON_SMALL, size::ICON_SMALL) * 0.7,
                );
                crate::icons::icon(painter, icon, Icon::Play, ink);
            }
        }
        if let Some((r, path)) = play
            && r.clicked()
        {
            return Some(RowAsk::Play(path));
        }
        response.clicked().then_some(RowAsk::Pick)
    }
}

#[cfg(test)]
#[path = "name_picker_tests.rs"]
mod tests;
