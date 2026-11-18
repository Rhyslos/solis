//! The browser's files and folders as tiles or a list (board 04): a tile
//! is its picture (what a level or picture holds, or its kind drawn), its
//! name, and its kind's dot and word. What a click, a double-click, a
//! drag and a right-click on one ask for is here too. Only the rows in
//! view are drawn, so a folder of thousands costs a screenful.

use super::{
    Entry, Files, LibraryDrag, LibraryItem, LibraryKind, LibraryRequest, LibraryState, Sounds,
    menus, moves_into, rename,
};
use crate::icons::{self, Icon};
use crate::sound_view::{self, EventState, SOUND_COLOR, SoundState};
use crate::theme;
use crate::tokens::{self, size, text};
use egui::{Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

/// What happened to one entry this frame.
pub(crate) struct Picked<'a> {
    pub(crate) entry: Entry<'a>,
    pub(crate) response: Response,
    /// Where its name is: a double-click there renames it, anywhere else
    /// opens it (board 04).
    pub(crate) name: Option<Rect>,
    /// Its + was clicked: place it.
    pub(crate) place: bool,
    /// Play or Stop, from a sound's ▶ or ■.
    pub(crate) listen: Option<LibraryRequest>,
    /// It's on the placing form, where a double-click places it.
    pub(crate) placing: bool,
}

/// What clicks, double-clicks, drags, drops and right-clicks on an entry
/// ask for. `order` is the entries in the order shown, for `Shift`.
pub(crate) fn item_actions(
    picked: Picked<'_>,
    state: &mut LibraryState,
    files: Option<Files<'_>>,
    order: &[&str],
    requests: &mut Vec<LibraryRequest>,
) {
    let Picked {
        entry,
        response,
        name,
        place,
        listen,
        placing,
    } = picked;
    let path = entry.path();
    requests.extend(listen);
    take_drops(entry, &response, requests);
    let moving = if state.is_picked(path) {
        let mut all = state.picked();
        all.retain(|p| p != path);
        all.insert(0, path.to_owned());
        all
    } else {
        vec![path.to_owned()]
    };
    let payload = match entry {
        Entry::File(item) => LibraryDrag(item.clone(), moving),
        Entry::Folder(_) => LibraryDrag(LibraryDrag::folder(path).0, moving),
    };
    response.dnd_set_drag_payload(payload);
    let modifiers = response.ctx.input(|i| i.modifiers);
    if response.clicked() {
        if modifiers.command {
            state.pick_also(path);
        } else if modifiers.shift {
            state.pick_range(path, order);
        } else {
            state.pick(path);
        }
    }
    let on_name = name.is_some_and(|rect| {
        response
            .interact_pointer_pos()
            .is_some_and(|at| rect.contains(at))
    });
    if place && let Some(id) = entry.file().and_then(|i| i.id) {
        requests.push(LibraryRequest::Place(id));
    } else if response.double_clicked() && on_name && !placing {
        if let Some(files) = files {
            rename::begin(state, files, path);
        }
    } else if response.double_clicked() {
        open(entry, state, placing, requests);
    }
    if response.secondary_clicked() && !state.is_picked(path) {
        // The menu acts on what's picked: what was pointed at becomes it.
        state.pick(path);
    }
    response.context_menu(|ui| menus::entry_menu(ui, entry, state, files, requests));
}

/// A double-click on an entry's picture: a folder opens; a level, or a
/// file with its own editor, opens; a file that would be placed is only
/// picked (the ribbon's **Place** places it, the project lead's answer
/// 25), or placed on the placing form.
fn open(
    entry: Entry<'_>,
    state: &mut LibraryState,
    placing: bool,
    requests: &mut Vec<LibraryRequest>,
) {
    let item = match entry {
        Entry::Folder(path) => {
            state.open_folder(path);
            return;
        }
        Entry::File(item) => item,
    };
    match (item.kind.placeable(), item.id) {
        (true, Some(id))
            if !item.kind.opens_on_double_click() && item.kind != LibraryKind::Texture =>
        {
            if placing {
                requests.push(LibraryRequest::Place(id));
            } else {
                state.pick(&item.path);
            }
        }
        _ => requests.push(LibraryRequest::Open(item.kind, item.path.clone())),
    }
}

/// Files dropped on an entry: onto a folder they move into it; a sound
/// file onto a sound event becomes one of its takes.
fn take_drops(entry: Entry<'_>, response: &Response, requests: &mut Vec<LibraryRequest>) {
    match entry {
        Entry::Folder(folder) => {
            if let Some(dragged) = response.dnd_release_payload::<LibraryDrag>() {
                requests.extend(moves_into(folder, &dragged.1));
            } else if response.dnd_hover_payload::<LibraryDrag>().is_some() {
                drop_mark(response, theme::FOLDER);
            }
        }
        Entry::File(item) if item.kind == LibraryKind::SoundEvent => {
            if let Some(dragged) = response.dnd_release_payload::<LibraryDrag>() {
                requests.extend(crate::sound_event_view::add_take(&item.path, &dragged.0));
            }
        }
        Entry::File(_) => {}
    }
}

/// The outline on what something dragged would be dropped on.
pub(crate) fn drop_mark(response: &Response, colour: Color32) {
    response.ctx.layer_painter(response.layer_id).rect_stroke(
        response.rect,
        size::radius::SMALL,
        Stroke::new(size::TILE_OUTLINE, colour),
        StrokeKind::Inside,
    );
}

/// The entries as tiles that fill the width, each at least `least` wide
/// and no more than a quarter wider (boards 03 and 04): only the rows in
/// view are drawn.
pub(crate) fn grid(
    ui: &mut Ui,
    entries: &[Entry<'_>],
    (files, state, least): (Files<'_>, &mut LibraryState, f32),
    requests: &mut Vec<LibraryRequest>,
) {
    let gap = size::TILE_GAP * 2.0;
    let per_row = ((ui.available_width() + gap) / (least + gap))
        .floor()
        .max(1.0) as usize;
    let width = ((ui.available_width() - (per_row - 1) as f32 * gap) / per_row as f32)
        .floor()
        .min(least * 1.25);
    let rows = entries.len().div_ceil(per_row);
    let order: Vec<&str> = entries.iter().map(|e| e.path()).collect();
    egui::ScrollArea::vertical()
        .id_salt("library tiles")
        .auto_shrink(false)
        .show_rows(ui, tile_height(width) + gap, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for entry in entries.iter().skip(row * per_row).take(per_row) {
                        let picked = tile(ui, *entry, (files, &mut *state), width, requests);
                        item_actions(picked, state, Some(files), &order, requests);
                    }
                });
                ui.add_space(gap);
            }
        });
}

/// How tall a tile `width` wide is: its square, its name and its kind.
fn tile_height(width: f32) -> f32 {
    width + size::GAP + 2.0 * size::ASSET_NAME_H
}

/// One tile (spec 306-309, board 04): the square picture (an accent
/// outline round it when picked), the name, cut off with "…" (or the box
/// to rename it), and the kind's dot and word; a + to place it.
fn tile<'a>(
    ui: &mut Ui,
    entry: Entry<'a>,
    (files, state): (Files<'_>, &mut LibraryState),
    width: f32,
    requests: &mut Vec<LibraryRequest>,
) -> Picked<'a> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, tile_height(width)), Sense::click_and_drag());
    let path = entry.path();
    let picked = state.is_picked(path);
    let accent = files.accent;
    let round = CornerRadius::same(size::radius::TAB);
    if picked {
        ui.painter().rect_filled(
            rect.expand(size::ROW_GAP),
            round,
            tokens::select_item(accent),
        );
    }
    let picture = Rect::from_min_size(rect.min, vec2(width, width));
    let fill = if response.hovered() {
        tokens::card_border()
    } else {
        tokens::hover()
    };
    ui.painter().rect(
        picture,
        round,
        fill,
        Stroke::new(size::LINE, tokens::field_border()),
        StrokeKind::Inside,
    );
    match entry {
        Entry::Folder(_) => folder_picture(ui.painter(), picture),
        Entry::File(item) => {
            paint_thumbnail(
                &ui.painter().with_clip_rect(picture),
                picture,
                item,
                files.sounds,
            );
            if item.kind == LibraryKind::SoundEvent {
                event_badge(ui, item, picture, &response, files.sounds);
            }
        }
    }
    if picked {
        ui.painter().rect_stroke(
            picture,
            round,
            Stroke::new(size::TILE_OUTLINE, accent.colors().fill),
            StrokeKind::Inside,
        );
    }
    let name = Rect::from_min_size(
        pos2(rect.left(), picture.bottom() + size::GAP),
        vec2(width, size::ASSET_NAME_H),
    );
    let boxed = Rect::from_min_size(
        name.min - vec2(0.0, (size::NAME_BOX_H - size::ASSET_NAME_H) / 2.0),
        vec2(width, size::NAME_BOX_H),
    );
    if !rename::name_box(ui, boxed, path, (files, state), requests) {
        crate::widgets::cut_off(
            ui,
            name,
            entry.name(),
            text::tile_name(),
            tokens::text_tile(),
        );
    }
    kind_line(ui, name.translate(vec2(0.0, size::ASSET_NAME_H)), entry);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, picked, entry.name())
    });
    let response = response.on_hover_text(hint(entry));
    let item = entry.file();
    let place = item.is_some_and(|i| i.kind.placeable() && place_button(ui, i, picture));
    let listen = item.and_then(|i| listen_button(ui, i, picture, &response, files.sounds));
    Picked {
        entry,
        response,
        name: Some(name),
        place,
        listen,
        placing: false,
    }
}

/// The kind's dot and word under a tile's name: "Folder" for a folder.
fn kind_line(ui: &Ui, line: Rect, entry: Entry<'_>) {
    let (colour, word) = match entry {
        Entry::Folder(_) => (theme::FOLDER, "Folder"),
        Entry::File(item) => (item.colour(), item.kind.name()),
    };
    let dot = pos2(line.left() + size::KIND_DOT / 2.0, line.center().y);
    ui.painter()
        .circle_filled(dot, size::KIND_DOT / 2.0, colour);
    let words = Rect::from_min_max(
        pos2(dot.x + size::KIND_DOT / 2.0 + size::ROW_GAP, line.top()),
        line.max,
    );
    crate::widgets::cut_off(ui, words, word, text::tile_kind(), tokens::text_meta());
}

/// A folder's picture: the folder drawn in its colour.
fn folder_picture(painter: &egui::Painter, picture: Rect) {
    let side = (picture.width() * 0.36).max(size::ICON_SMALL);
    icons::icon(
        painter,
        Rect::from_center_size(picture.center(), vec2(side, side)),
        Icon::Library,
        theme::FOLDER,
    );
}

/// The hint shown when pointing at an entry.
fn hint(entry: Entry<'_>) -> String {
    let item = match entry {
        Entry::Folder(path) => {
            return format!(
                "Folder · {path}\nDouble-click to open it; double-click its name to rename it. \
                 Drag files onto it to move them in."
            );
        }
        Entry::File(item) => item,
    };
    let what = match item.kind {
        LibraryKind::Model | LibraryKind::Assembly => {
            "Drag it into the viewport or onto an entity, or pick it and press Place, to place it."
        }
        LibraryKind::Level => "Double-click its picture to open it.",
        LibraryKind::Sound => "▶ to listen; click it to see it in Details.",
        LibraryKind::SpriteSheet => {
            "Drag it into the viewport, or pick it and press Place, to place it as a sprite."
        }
        LibraryKind::SpriteClips => {
            "Double-click its picture to open its clips; drag it into the viewport to place a \
             sprite that plays them."
        }
        LibraryKind::Tileset => "Double-click its picture to open it in the Tileset tab.",
        LibraryKind::TileMap => {
            "Drag it into the viewport, or pick it and press Place, to place it."
        }
        LibraryKind::Animator => "Double-click its picture to open it in the Animator tab.",
        LibraryKind::SoundEvent => {
            "▶ to listen (a different take each time); drop sound files on it to add takes."
        }
        LibraryKind::Texture => {
            "Drag it into the viewport to place it as a sprite; double-click its picture to \
             open it in its own program."
        }
        _ => "Double-click its picture to open it in its own program.",
    };
    format!(
        "{} · {}\n{what} Double-click its name to rename it.",
        item.kind.name(),
        item.path
    )
}

/// "1 clip", "6 clips".
pub(crate) fn clips_text(n: usize) -> String {
    if n == 1 {
        "1 clip".into()
    } else {
        format!("{n} clips")
    }
}

/// "1 frame", "24 frames".
pub(crate) fn frames_text(n: usize) -> String {
    if n == 1 {
        "1 frame".into()
    } else {
        format!("{n} frames")
    }
}

/// What a file looks like in a tile's square `picture`: a sound's
/// waveform, a sheet's (or its clips') first frame with how many frames
/// or clips, a model's or picture's thumbnail, a text file's extension,
/// or its kind drawn in its colour.
pub(crate) fn paint_thumbnail(
    painter: &egui::Painter,
    picture: Rect,
    item: &LibraryItem,
    sounds: Sounds<'_>,
) {
    let waveform = match sounds.states.get(&item.path) {
        Some(SoundState::Read(info)) if item.kind == LibraryKind::Sound => Some(info),
        _ => None,
    };
    // A clips file shows its sheet's first frame, and how many clips.
    let sheet_path = match item.kind {
        LibraryKind::SpriteSheet => Some(item.path.as_str()),
        LibraryKind::SpriteClips => item
            .id
            .and_then(|id| sounds.sheets.frames_file(id))
            .and_then(|f| sounds.sheets.paths.get(&f.sheet?))
            .map(String::as_str),
        _ => None,
    };
    let badge = match item.kind {
        LibraryKind::SpriteClips => item
            .id
            .and_then(|id| sounds.sheets.clips(id))
            .map(|set| clips_text(set.clips.len())),
        _ => None,
    };
    let sheet = sheet_path
        .and_then(|path| Some((sounds.sheets.at(path)?, sounds.sheet_pictures.get(path)?)));
    if let Some(info) = waveform {
        sound_view::draw_waveform(
            painter,
            picture.shrink2(vec2(size::TILE_PAD, picture.height() * 0.3)),
            &info.waveform,
            sounds.played(item),
        );
    } else if let Some((sheet, atlas)) = sheet {
        crate::sheet_view::draw_frame(painter, picture.shrink(size::ROW_GAP), sheet, atlas, 0);
        painter.text(
            picture.right_top() + vec2(-size::ROW_GAP, size::ROW_GAP),
            egui::Align2::RIGHT_TOP,
            badge.unwrap_or_else(|| frames_text(sheet.frames.len())),
            text::tile_kind(),
            theme::text(),
        );
    } else if let Some(texture) = item.thumbnail {
        painter.image(
            texture,
            picture.shrink(size::TILE_OUTLINE),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else if let Some(extension) = item.text_extension() {
        // A text file's icon is its extension, in its colour.
        painter.text(
            picture.center(),
            egui::Align2::CENTER_CENTER,
            extension,
            egui::FontId::monospace((picture.width() * 0.2).round()),
            item.colour(),
        );
    } else {
        let side = (picture.width() * 0.34).max(size::ICON_SMALL);
        icons::icon(
            painter,
            Rect::from_center_size(picture.center(), vec2(side, side)),
            item.kind.icon(),
            item.kind.color(),
        );
    }
}

/// The small + on a placeable item; returns whether it was clicked.
fn place_button(ui: &mut Ui, item: &LibraryItem, area: Rect) -> bool {
    let plus = Rect::from_min_size(
        pos2(area.right() - size::LOCK_BUTTON - 1.0, area.top() + 1.0),
        vec2(size::LOCK_BUTTON, size::LOCK_BUTTON),
    );
    let button = ui
        .put(
            plus,
            egui::Button::new(egui::RichText::new("+").color(theme::text_strong())).frame(false),
        )
        .on_hover_text("Place it where the camera looks");
    button.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Place {}", item.name),
        )
    });
    button.clicked()
}

/// On a sound event's tile: how many takes it has, and a frame while a
/// sound file is dragged over it (dropping adds it as a take).
fn event_badge(ui: &Ui, item: &LibraryItem, picture: Rect, response: &Response, sounds: Sounds) {
    let painter = ui.painter();
    if let Some(EventState::Read { event, .. }) = sounds.events.get(&item.path) {
        painter.text(
            picture.right_bottom() - vec2(size::TILE_PAD, size::TILE_PAD),
            egui::Align2::RIGHT_BOTTOM,
            format!("×{}", event.takes.len()),
            text::tile_kind(),
            SOUND_COLOR,
        );
    }
    let sound_over = response
        .dnd_hover_payload::<LibraryDrag>()
        .is_some_and(|d| d.0.kind == LibraryKind::Sound);
    if sound_over {
        drop_mark(response, SOUND_COLOR);
    }
}

/// A sound's ▶ (or ■ while it plays), shown while it's pointed at or
/// playing; returns what a click asks for.
fn listen_button(
    ui: &mut Ui,
    item: &LibraryItem,
    area: Rect,
    response: &Response,
    sounds: Sounds<'_>,
) -> Option<LibraryRequest> {
    if !matches!(item.kind, LibraryKind::Sound | LibraryKind::SoundEvent) {
        return None;
    }
    let playing = sounds.played(item).is_some();
    let spot = Rect::from_min_size(
        pos2(area.right() - size::LOCK_BUTTON - 1.0, area.top() + 1.0),
        vec2(size::LOCK_BUTTON, size::LOCK_BUTTON),
    );
    let pointed = response.hovered() || ui.rect_contains_pointer(spot);
    if !playing && !pointed {
        return None;
    }
    let button = ui
        .put(spot, egui::Button::new("").frame(false))
        .on_hover_text(if playing { "Stop" } else { "Listen" });
    let shape = if playing { Icon::Stop } else { Icon::Play };
    let ink = if button.hovered() {
        theme::text_strong()
    } else {
        SOUND_COLOR
    };
    icons::icon(ui.painter(), spot.shrink(size::ROW_GAP), shape, ink);
    button.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("{} {}", if playing { "Stop" } else { "Play" }, item.name),
        )
    });
    button.clicked().then(|| {
        if playing {
            LibraryRequest::StopSound
        } else {
            LibraryRequest::PlaySound(item.path.clone())
        }
    })
}

/// The entries as a list, one row each; only the rows in view are drawn.
/// With `code`, the Code panel: each code file with what uses it.
pub(crate) fn list(
    ui: &mut Ui,
    entries: &[Entry<'_>],
    (files, state): (Files<'_>, &mut LibraryState),
    requests: &mut Vec<LibraryRequest>,
) {
    let order: Vec<&str> = entries.iter().map(|e| e.path()).collect();
    egui::ScrollArea::vertical()
        .id_salt("library list")
        .auto_shrink(false)
        .show_rows(ui, size::LIBRARY_ROW_H, entries.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for entry in &entries[range] {
                let picked = row(ui, *entry, (files, &mut *state), requests);
                item_actions(picked, state, Some(files), &order, requests);
            }
        });
}

/// One row of the list: the kind's colour, its icon, its name (or the box
/// to rename it), the folder while searching, and its kind's word.
fn row<'a>(
    ui: &mut Ui,
    entry: Entry<'a>,
    (files, state): (Files<'_>, &mut LibraryState),
    requests: &mut Vec<LibraryRequest>,
) -> Picked<'a> {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), size::LIBRARY_ROW_H),
        Sense::click_and_drag(),
    );
    let path = entry.path();
    let picked = state.is_picked(path);
    if picked {
        ui.painter()
            .rect_filled(rect, size::radius::SMALL, tokens::select_row(files.accent));
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, size::radius::SMALL, tokens::hover());
    }
    let (icon, colour) = match entry {
        Entry::Folder(_) => (Icon::Library, theme::FOLDER),
        Entry::File(item) => (item.kind.icon(), item.colour()),
    };
    let spot = Rect::from_center_size(
        pos2(rect.left() + size::ROW_PAD, rect.center().y),
        vec2(size::ROW_ICON, size::ROW_ICON),
    );
    icons::icon(ui.painter(), spot, icon, colour);
    let words_left = spot.right() + size::GAP;
    let kind_w = size::KEY_COLUMN_W;
    let name = Rect::from_min_max(
        pos2(words_left, rect.top()),
        pos2(rect.right() - kind_w - size::GAP, rect.bottom()),
    );
    let shown = if state.search.is_empty() {
        entry.name().to_owned()
    } else {
        format!("{}  ·  {}", entry.name(), super::parent_of(path))
    };
    let boxed = Rect::from_center_size(name.center(), vec2(name.width(), size::NAME_BOX_H));
    if !rename::name_box(ui, boxed, path, (files, state), requests) {
        crate::widgets::cut_off(ui, name, &shown, text::row(), tokens::text_primary());
    }
    let after = entry
        .file()
        .and_then(|item| files.sounds.code_uses.get(&item.path).cloned())
        .unwrap_or_else(|| match entry {
            Entry::Folder(_) => "Folder".to_owned(),
            Entry::File(item) => item.kind.name().to_owned(),
        });
    let kind = Rect::from_min_max(pos2(rect.right() - kind_w, rect.top()), rect.max);
    crate::widgets::cut_off(ui, kind, &after, text::row(), tokens::text_meta());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, picked, entry.name())
    });
    let response = response.on_hover_text(hint(entry));
    let name_width = crate::widgets::width_of(ui, &shown, text::row());
    let item = entry.file();
    let buttons = Rect::from_min_max(pos2(kind.left() - size::LOCK_BUTTON, rect.top()), rect.max);
    let place = item.is_some_and(|i| i.kind.placeable() && place_button(ui, i, buttons));
    let listen = item.and_then(|i| listen_button(ui, i, buttons, &response, files.sounds));
    Picked {
        entry,
        response,
        name: Some(name.with_max_x(name.left() + name_width.min(name.width()))),
        place,
        listen,
        placing: false,
    }
}

/// The Code panel: the project's code files as a list, with what uses
/// each.
pub(crate) fn code_list(
    ui: &mut Ui,
    files: Files<'_>,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let entries: Vec<Entry<'_>> = files
        .items
        .iter()
        .filter(|i| i.kind == LibraryKind::Code)
        .map(Entry::File)
        .collect();
    if entries.is_empty() {
        crate::panels::empty_note(
            ui,
            &format!(
                "{} has no code yet. Lust, Lua and Rust files in it show here: right-click in \
                 the Library and choose New code file to start one.",
                files.project.name
            ),
        );
        return;
    }
    list(ui, &entries, (files, state), requests);
}
