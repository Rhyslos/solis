//! The Table tab (stage 1, data tables; the approved sketch is
//! `docs/design/data-tables/`): a bar of + Row, + Column ▾, the search,
//! what each row is called and CSV in and out; then the grid, its
//! headings held at the top as it scrolls, only the rows in view drawn
//! (a table of 10,000 rows draws as quickly as one of ten); and under it
//! how game code reads the table.
//!
//! Inside the grid the common spreadsheet keys work, and nowhere else:
//! the arrows move (Shift picks a block), Tab and Shift+Tab move across,
//! Enter or F2 types in the cell (a yes-or-no cell flips), a key typed
//! starts typing over it, Enter keeps what's typed and moves down, Tab
//! keeps it and moves across, Esc leaves it as it was, Delete empties the
//! block, and Ctrl+C and Ctrl+V copy and paste it as the tab-separated
//! text spreadsheets use.

use crate::table_editor::{Place, Sort, TableEditor, Typing};
use crate::tokens::{self, Accent, size, text};
use crate::widgets::{self, button::Button, field};
use egui::{
    Align2, Color32, CornerRadius, Key, Modifiers, Pos2, Rect, Response, Sense, Stroke, StrokeKind,
    Ui, pos2, vec2,
};
use level::table::text as sheet;
use level::table::{Cell, ColumnKind, Table};

/// A row's height in the grid.
pub(crate) const ROW_H: f32 = 30.0;
/// The headings' height: a column's name over what it holds.
pub(crate) const HEAD_H: f32 = 40.0;
/// The IDs' column's width.
const ID_W: f32 = 160.0;
/// The most options a choice or name cell lists while typing.
const MAX_LISTED: usize = 12;

/// What the Table tab draws from.
pub(crate) struct TableUi<'a> {
    /// The table open.
    pub(crate) editor: &'a mut TableEditor,
    /// The project's names by kind (`Picture` → its options), for name
    /// columns' lists.
    pub(crate) names: Option<&'a script::project_names::States>,
    /// Whether the keys are the tab's this frame.
    pub(crate) keys: bool,
    /// The accent.
    pub(crate) accent: Accent,
}

/// What the Table tab asks of the workbench after a frame.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct TableOut {
    /// The table changed: save it.
    pub(crate) changed: bool,
    /// Something to say in the Console (why a typed value wasn't kept).
    pub(crate) said: Option<String>,
    /// Import CSV was pressed.
    pub(crate) import: bool,
    /// Export CSV was pressed.
    pub(crate) export: bool,
}

/// A column's width, by what it holds.
pub(crate) fn width_of(kind: &ColumnKind) -> f32 {
    match kind {
        ColumnKind::Whole | ColumnKind::Number | ColumnKind::YesNo => 130.0,
        ColumnKind::Colour => 140.0,
        ColumnKind::Choice(_) => 150.0,
        ColumnKind::Name(_) => 190.0,
        ColumnKind::Text => 260.0,
    }
}

/// Each place's left edge and width in the grid (0 is the IDs).
fn places_of(table: &Table) -> Vec<(f32, f32)> {
    let mut x = 0.0;
    let mut out = Vec::with_capacity(table.columns.len() + 1);
    for w in std::iter::once(ID_W).chain(table.columns.iter().map(|c| width_of(&c.kind))) {
        out.push((x, w));
        x += w;
    }
    out
}

/// Draws the Table tab filling `ui`.
pub(crate) fn table_tab(ui: &mut Ui, t: TableUi<'_>) -> TableOut {
    let mut out = TableOut::default();
    let TableUi {
        editor,
        names,
        keys,
        accent,
    } = t;
    egui::Frame::new()
        .inner_margin(crate::theme::panel_margin())
        .show(ui, |ui| {
            bar(ui, editor, &mut out);
            ui.add_space(size::GAP);
            let note_h = 34.0;
            let grid_h = (ui.available_height() - note_h - size::GAP).max(ROW_H * 3.0);
            ui.allocate_ui(vec2(ui.available_width(), grid_h), |ui| {
                grid(ui, editor, (names, keys, accent), &mut out);
            });
            ui.add_space(size::GAP);
            how_code_reads(ui, &editor.path, &editor.file);
        });
    out
}

/// The bar over the grid.
fn bar(ui: &mut Ui, editor: &mut TableEditor, out: &mut TableOut) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
        if Button::primary("+ Row")
            .named("Add a row")
            .show(ui)
            .clicked()
        {
            match editor.add_row() {
                Ok(()) => out.changed = true,
                Err(why) => out.said = Some(why),
            }
        }
        let column = Button::plain("+ Column ▾").named("Add a column").show(ui);
        egui::Popup::menu(&column).show(|ui| {
            widgets::menu::caps(ui, "What it holds");
            for kind in ColumnKind::all() {
                if widgets::menu::item(ui, &kind.label(), "").clicked() {
                    match editor.add_column(kind) {
                        Ok(()) => out.changed = true,
                        Err(why) => out.said = Some(why),
                    }
                    ui.close();
                }
            }
        });
        field::search_field(ui, "Search the table", &mut editor.search, "Search", 220.0);
        ui.label(
            egui::RichText::new("Each row is a")
                .font(text::row())
                .color(tokens::text_meta()),
        );
        let mut name = editor.file.row_name.clone();
        let named = field::text_field(ui, "What each row is called", &mut name, "Item", 120.0);
        if named.lost_focus() && name != editor.file.row_name {
            match editor.set_row_name(&name) {
                Ok(()) => out.changed = true,
                Err(why) => out.said = Some(why),
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if Button::plain("Export CSV")
                .named("Export the table as CSV")
                .show(ui)
                .clicked()
            {
                out.export = true;
            }
            if Button::plain("Import CSV")
                .named("Import CSV into the table")
                .show(ui)
                .clicked()
            {
                out.import = true;
            }
            let count = editor.shown().len();
            let all = editor.file.rows.len();
            let words = if count == all {
                format!("{all} rows")
            } else {
                format!("{count} of {all} rows")
            };
            ui.label(
                egui::RichText::new(words)
                    .font(text::row())
                    .color(tokens::text_meta()),
            );
        });
    });
}

/// The note under the grid: how game code reads the table, with its own
/// names.
fn how_code_reads(ui: &mut Ui, path: &str, table: &Table) {
    let stem = path.rsplit('/').next().unwrap_or(path);
    let stem = reflect::data_files::stem(std::path::Path::new(stem)).unwrap_or_default();
    let object = level::table::object_name_for(&stem).unwrap_or_else(|| "Table".into());
    let row = table.rows.first().map_or("Apple", |r| r.id.as_str());
    let column = table.columns.first().map_or("id", |c| c.name.as_str());
    let example = format!("{object}:row({}.{row}).{column}", table.row_name);
    let rect = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(rect.min, vec2(rect.width(), 30.0));
    let warm = tokens::warning_row();
    ui.painter().rect_filled(rect, CornerRadius::same(3), warm);
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
        CornerRadius::ZERO,
        crate::theme::WARNING,
    );
    let galley = ui.painter().layout_job({
        let mut job = egui::text::LayoutJob::default();
        let plain = egui::TextFormat::simple(text::row(), tokens::text_body());
        let code = egui::TextFormat::simple(text::mono(), tokens::text_primary());
        job.append(
            "Game code reads it with checked names: ",
            0.0,
            plain.clone(),
        );
        job.append(&example, 0.0, code);
        job.append(
            ". A misspelt row or column fails before the game runs.",
            0.0,
            plain,
        );
        job
    });
    ui.painter().galley(
        pos2(rect.min.x + 14.0, rect.center().y - galley.size().y / 2.0),
        galley,
        tokens::text_body(),
    );
    ui.allocate_rect(rect, Sense::hover());
}

/// The grid: headings held at the top, rows in view drawn, the keys.
fn grid(
    ui: &mut Ui,
    editor: &mut TableEditor,
    (names, keys_here, accent): (Option<&script::project_names::States>, bool, Accent),
    out: &mut TableOut,
) {
    let places = places_of(&editor.file);
    let width = places.last().map_or(ID_W, |(x, w)| x + w);
    let shown = editor.shown().to_vec();
    let height = HEAD_H + shown.len() as f32 * ROW_H;
    // The box typed in has the keyboard, so the keys are the grid's
    // while typing wherever the pointer is.
    if editor.typing.is_some() {
        typing_keys(ui, editor, out);
    } else if keys_here {
        keys(ui, editor, &shown, out);
    }
    let reveal = std::mem::take(&mut editor.reveal);
    egui::ScrollArea::both()
        .id_salt("table grid")
        .auto_shrink(false)
        .show_viewport(ui, |ui, view| {
            let (full, _) =
                ui.allocate_exact_size(vec2(width.max(view.width()), height), Sense::hover());
            let response = ui.interact(full, grid_id(), Sense::click_and_drag());
            // The grid keeps the keys once clicked: Tab and the arrows
            // move its cursor rather than to another field.
            if response.clicked() || response.drag_started() || response.secondary_clicked() {
                response.request_focus();
            }
            if response.has_focus() {
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        grid_id(),
                        egui::EventFilter {
                            tab: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                        },
                    );
                });
            }
            let origin = full.min;
            editor.drawn_at = Some((origin.x, origin.y + view.min.y));
            let cell_rect = |at: usize, place: usize| {
                let (x, w) = places[place];
                Rect::from_min_size(origin + vec2(x, HEAD_H + at as f32 * ROW_H), vec2(w, ROW_H))
            };
            if reveal
                && let Some((row, place)) = editor.cursor
                && let Some(at) = shown.iter().position(|r| *r == row)
            {
                let mut r = cell_rect(at, place);
                r.min.y -= HEAD_H;
                ui.scroll_to_rect(r, None);
            }
            let first = ((view.min.y - HEAD_H) / ROW_H).floor().max(0.0) as usize;
            let last =
                (((view.max.y - HEAD_H) / ROW_H).ceil().max(0.0) as usize + 1).min(shown.len());
            let painter = ui.painter().clone();
            for (at, &row) in shown.iter().enumerate().take(last).skip(first) {
                draw_row(&painter, editor, row, at, &places, origin, accent);
            }
            heads(ui, editor, &places, origin, view, &response, out);
            click(ui, editor, &shown, &places, origin, &response, out);
            typing_box(ui, editor, &shown, &cell_rect, names, out);
        });
}

/// The grid's own name: holding the keyboard, the keys are the
/// grid's (`key_owner.rs`).
pub(crate) fn grid_id() -> egui::Id {
    egui::Id::new("table grid cells")
}

/// One row's cells at its place among those shown (`at`).
fn draw_row(
    painter: &egui::Painter,
    editor: &TableEditor,
    row: usize,
    at: usize,
    places: &[(f32, f32)],
    origin: Pos2,
    accent: Accent,
) {
    let Some(r) = editor.file.rows.get(row) else {
        return;
    };
    let y = origin.y + HEAD_H + at as f32 * ROW_H;
    let line = tokens::line();
    let cursor_row = editor.cursor.is_some_and(|(c, _)| c == row);
    for (place, &(x, w)) in places.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(origin.x + x, y), vec2(w, ROW_H));
        let fill = if editor.picked((row, place)) && editor.anchor.is_some() {
            tokens::select_item(accent)
        } else if place == 0 {
            tokens::panel()
        } else {
            tokens::card()
        };
        painter.rect_filled(rect, CornerRadius::ZERO, fill);
        painter.rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, line),
            StrokeKind::Inside,
        );
        let inner = rect.shrink2(vec2(12.0, 0.0));
        let clip = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        if place == 0 {
            clip.text(
                inner.left_center(),
                Align2::LEFT_CENTER,
                &r.id,
                text::ui(),
                tokens::text_primary(),
            );
            continue;
        }
        let Some(cell) = r.cells.get(place - 1) else {
            continue;
        };
        draw_cell(&clip, inner, cell, &editor.file.columns[place - 1].kind);
    }
    if cursor_row {
        let full = Rect::from_min_size(
            pos2(origin.x, y),
            vec2(places.last().map_or(0.0, |(x, w)| x + w), ROW_H),
        );
        painter.rect_stroke(
            full,
            CornerRadius::same(2),
            Stroke::new(1.5, accent.colors().fill),
            StrokeKind::Inside,
        );
        if let Some((_, place)) = editor.cursor
            && let Some(&(x, w)) = places.get(place)
        {
            let cell = Rect::from_min_size(pos2(origin.x + x, y), vec2(w, ROW_H));
            painter.rect_stroke(
                cell,
                CornerRadius::same(2),
                Stroke::new(2.0, accent.colors().fill),
                StrokeKind::Inside,
            );
        }
    }
}

/// A cell's value, drawn in `inner` as its kind shows it.
fn draw_cell(painter: &egui::Painter, inner: Rect, cell: &Cell, kind: &ColumnKind) {
    let words = |s: &str, colour: Color32, x: f32| {
        painter.text(
            pos2(x, inner.center().y),
            Align2::LEFT_CENTER,
            s,
            text::ui(),
            colour,
        );
    };
    match (cell, kind) {
        (Cell::YesNo(on), _) => {
            let square =
                Rect::from_center_size(pos2(inner.min.x + 7.0, inner.center().y), vec2(14.0, 14.0));
            draw_check(painter, square, *on);
        }
        (Cell::Colour(c), _) => {
            let swatch =
                Rect::from_min_size(pos2(inner.min.x, inner.center().y - 7.0), vec2(22.0, 14.0));
            let [r, g, b, a] = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            painter.rect_filled(
                swatch,
                CornerRadius::same(2),
                Color32::from_rgba_unmultiplied(r, g, b, a),
            );
            painter.rect_stroke(
                swatch,
                CornerRadius::same(2),
                Stroke::new(1.0, tokens::field_border()),
                StrokeKind::Inside,
            );
            words(
                &sheet::colour_hex(*c),
                tokens::text_body(),
                swatch.max.x + 8.0,
            );
        }
        (Cell::Option(o), ColumnKind::Name(k)) => {
            let spot =
                Rect::from_center_size(pos2(inner.min.x + 7.0, inner.center().y), vec2(14.0, 14.0));
            crate::icons::icon(painter, spot, name_icon(*k), tokens::text_meta());
            if o.is_empty() {
                words("none", tokens::text_faint(), spot.max.x + 8.0);
            } else {
                let last = o.rsplit('.').next().unwrap_or(o);
                words(last, tokens::text_primary(), spot.max.x + 8.0);
            }
        }
        (Cell::Whole(_) | Cell::Number(_), _) => {
            words(&sheet::cell_text(cell), tokens::text_primary(), inner.min.x);
        }
        _ => {
            let shown = sheet::cell_text(cell);
            let one_line = shown.lines().next().unwrap_or("");
            words(one_line, tokens::text_primary(), inner.min.x);
        }
    }
}

/// A yes-or-no box: ticked or empty.
fn draw_check(painter: &egui::Painter, square: Rect, on: bool) {
    let fill = if on {
        tokens::text_primary()
    } else {
        tokens::field()
    };
    painter.rect_filled(square, CornerRadius::same(3), fill);
    painter.rect_stroke(
        square,
        CornerRadius::same(3),
        Stroke::new(1.0, tokens::field_border()),
        StrokeKind::Inside,
    );
    if on {
        let c = square.center();
        let tick = [
            pos2(c.x - 3.5, c.y),
            pos2(c.x - 1.0, c.y + 3.0),
            pos2(c.x + 4.0, c.y - 3.0),
        ];
        painter.line(tick.to_vec(), Stroke::new(1.8, tokens::app()));
    }
}

/// The icon a name column's cells show.
fn name_icon(kind: reflect::NameKind) -> crate::icons::Icon {
    use crate::icons::Icon;
    match kind {
        reflect::NameKind::Sound => Icon::SoundEvent,
        reflect::NameKind::Clip => Icon::Clips,
        reflect::NameKind::Picture => Icon::Picture,
        reflect::NameKind::Prefab => Icon::Prefab,
        reflect::NameKind::Screen => Icon::Screen,
        reflect::NameKind::Level => Icon::LevelFile,
        reflect::NameKind::Tag => Icon::Key,
    }
}

/// The headings, held at the top of the view: a column's name over what
/// it holds; a click sorts by it (lowest first, then highest, then the
/// file's order), a right-click offers its menu.
fn heads(
    ui: &mut Ui,
    editor: &mut TableEditor,
    places: &[(f32, f32)],
    origin: Pos2,
    view: Rect,
    response: &Response,
    out: &mut TableOut,
) {
    let painter = ui.painter();
    let top = origin.y + view.min.y;
    for (place, &(x, w)) in places.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(origin.x + x, top), vec2(w, HEAD_H));
        painter.rect_filled(rect, CornerRadius::ZERO, tokens::panel());
        painter.rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, tokens::strong_line()),
            StrokeKind::Inside,
        );
        let (name, what) = match place {
            0 => ("ID".to_string(), String::new()),
            p => {
                let c = &editor.file.columns[p - 1];
                let mut what = match &c.kind {
                    ColumnKind::Choice(_) => {
                        script::table_code::choice_state(&editor.file.row_name, &c.name)
                    }
                    k => k.label(),
                };
                if c.translated {
                    what.push_str(" · translated");
                }
                (c.label(), what)
            }
        };
        let arrow = match editor.sort {
            Some(Sort { place: p, rising }) if p == place => {
                if rising {
                    " ▲"
                } else {
                    " ▼"
                }
            }
            _ => "",
        };
        let clip = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        let at = pos2(rect.min.x + 12.0, rect.center().y);
        let head = clip.text(
            at,
            Align2::LEFT_CENTER,
            format!("{name}{arrow}"),
            text::header(),
            tokens::text_label(),
        );
        clip.text(
            pos2(head.max.x + 6.0, at.y + 1.0),
            Align2::LEFT_CENTER,
            what,
            text::mono_small(),
            tokens::text_meta(),
        );
    }
    let Some(pointer) = response.interact_pointer_pos().or(response.hover_pos()) else {
        return;
    };
    if pointer.y > top + HEAD_H {
        return;
    }
    let local = pointer.x - origin.x;
    let Some(place) = places
        .iter()
        .position(|(x, w)| local >= *x && local < x + w)
    else {
        return;
    };
    if response.clicked() {
        editor.sort = match editor.sort {
            Some(Sort {
                place: p,
                rising: true,
            }) if p == place => Some(Sort {
                place,
                rising: false,
            }),
            Some(Sort {
                place: p,
                rising: false,
            }) if p == place => None,
            _ => Some(Sort {
                place,
                rising: true,
            }),
        };
    }
    if response.secondary_clicked() && place > 0 {
        ui.memory_mut(|m| {
            m.data
                .insert_temp(egui::Id::new("table column menu"), place - 1)
        });
    }
    let column: Option<usize> = ui.memory(|m| m.data.get_temp(egui::Id::new("table column menu")));
    if let Some(column) = column {
        column_menu(response, editor, column, out);
    }
}

/// A column's right-click menu: rename, what it holds, its options,
/// translated, move and delete.
fn column_menu(response: &Response, editor: &mut TableEditor, column: usize, out: &mut TableOut) {
    egui::Popup::context_menu(response).show(|ui| {
        let Some(c) = editor.file.columns.get(column).cloned() else {
            return;
        };
        widgets::menu::caps(ui, &c.label());
        let key = egui::Id::new(("table column name", column));
        let mut name: String = ui
            .memory(|m| m.data.get_temp(key))
            .unwrap_or_else(|| c.name.clone());
        let typed = field::text_field(ui, "The column's name", &mut name, "price", 180.0);
        ui.memory_mut(|m| m.data.insert_temp(key, name.clone()));
        if typed.lost_focus() && name != c.name {
            match editor.rename_column(column, &name) {
                Ok(()) => out.changed = true,
                Err(why) => out.said = Some(why),
            }
            ui.memory_mut(|m| m.data.remove::<String>(key));
        }
        if let ColumnKind::Choice(options) = &c.kind {
            let key = egui::Id::new(("table options", column));
            let mut list: String = ui
                .memory(|m| m.data.get_temp(key))
                .unwrap_or_else(|| options.join(", "));
            let typed =
                field::text_field(ui, "The column's options", &mut list, "Common, Rare", 180.0);
            ui.memory_mut(|m| m.data.insert_temp(key, list.clone()));
            if typed.lost_focus() {
                let new: Vec<String> = list
                    .split(',')
                    .map(|o| o.trim().to_string())
                    .filter(|o| !o.is_empty())
                    .collect();
                if &new != options {
                    match editor.set_options(column, new) {
                        Ok(()) => out.changed = true,
                        Err(why) => out.said = Some(why),
                    }
                }
                ui.memory_mut(|m| m.data.remove::<String>(key));
            }
        }
        if c.kind == ColumnKind::Text {
            let mut on = c.translated;
            if widgets::menu::check(ui, &mut on, "Translated").clicked() {
                out.changed |= editor.set_translated(column, on);
            }
        }
        ui.menu_button("Holds", |ui| {
            for kind in ColumnKind::all() {
                let chosen = kind.word() == c.kind.word();
                if widgets::menu::choice(ui, &kind.label(), chosen).clicked() && !chosen {
                    out.changed |= editor.set_kind(column, kind);
                    ui.close();
                }
            }
        });
        widgets::menu::line(ui);
        if column > 0 && widgets::menu::item(ui, "Move left", "").clicked() {
            out.changed |= editor.move_column(column, column - 1);
            ui.close();
        }
        if column + 1 < editor.file.columns.len()
            && widgets::menu::item(ui, "Move right", "").clicked()
        {
            out.changed |= editor.move_column(column, column + 1);
            ui.close();
        }
        if widgets::menu::item(ui, "Delete the column", "").clicked() {
            out.changed |= editor.delete_column(column);
            ui.close();
        }
    });
}

/// Clicks and drags on the cells: a click picks a cell (Shift: a block
/// from the cursor), a drag picks a block, a double-click types in it (a
/// yes-or-no cell flips on a click), a right-click offers the row's menu.
fn click(
    ui: &mut Ui,
    editor: &mut TableEditor,
    shown: &[usize],
    places: &[(f32, f32)],
    origin: Pos2,
    response: &Response,
    out: &mut TableOut,
) {
    let at = |pointer: Pos2| -> Option<Place> {
        let index = ((pointer.y - origin.y - HEAD_H) / ROW_H).floor();
        if index < 0.0 {
            return None;
        }
        let row = *shown.get(index as usize)?;
        let x = pointer.x - origin.x;
        let place = places.iter().position(|(l, w)| x >= *l && x < l + w)?;
        Some((row, place))
    };
    let top = response.rect.min.y + ui.clip_rect().min.y - response.rect.min.y;
    let pointer = response.interact_pointer_pos();
    let in_cells = pointer.is_some_and(|p| p.y > top + HEAD_H);
    if !in_cells {
        return;
    }
    let Some(spot) = pointer.and_then(at) else {
        return;
    };
    let shift = ui.input(|i| i.modifiers.shift);
    if response.drag_started() || (response.clicked() && !shift) {
        if editor.typing.as_ref().is_some_and(|t| t.at != spot) {
            if let Err(why) = editor.confirm() {
                out.said = Some(why);
            }
            out.changed = true;
        }
        editor.cursor = Some(spot);
        editor.anchor = None;
    } else if response.dragged() || (response.clicked() && shift) {
        editor.anchor = editor.anchor.or(editor.cursor);
        editor.cursor = Some(spot);
    }
    let yes_no = spot.1 > 0
        && editor
            .file
            .columns
            .get(spot.1 - 1)
            .is_some_and(|c| c.kind == ColumnKind::YesNo);
    if response.clicked() && yes_no && !shift {
        out.changed |= flip(editor, spot);
    } else if response.double_clicked() && !yes_no {
        editor.start_typing(None);
    }
    if response.secondary_clicked() {
        editor.cursor = Some(spot);
        ui.memory_mut(|m| m.data.remove::<usize>(egui::Id::new("table column menu")));
    }
    if ui.memory(|m| {
        m.data
            .get_temp::<usize>(egui::Id::new("table column menu"))
            .is_none()
    }) {
        row_menu(response, editor, out);
    }
}

/// Flips a yes-or-no cell.
fn flip(editor: &mut TableEditor, at: Place) -> bool {
    match editor
        .file
        .rows
        .get(at.0)
        .and_then(|r| r.cells.get(at.1 - 1))
    {
        Some(Cell::YesNo(on)) => {
            let on = !*on;
            editor.set_cell(at, Cell::YesNo(on))
        }
        _ => false,
    }
}

/// A row's right-click menu: a row above or below, move, delete.
fn row_menu(response: &Response, editor: &mut TableEditor, out: &mut TableOut) {
    egui::Popup::context_menu(response).show(|ui| {
        let Some((row, _)) = editor.cursor else {
            return;
        };
        let rows = editor.block().map_or(row..=row, |(r, _)| r);
        if widgets::menu::item(ui, "Add a row below", "").clicked() {
            match editor.add_row() {
                Ok(()) => out.changed = true,
                Err(why) => out.said = Some(why),
            }
            ui.close();
        }
        if row > 0 && widgets::menu::item(ui, "Move up", "").clicked() {
            out.changed |= editor.move_row(row, row - 1);
            ui.close();
        }
        if row + 1 < editor.file.rows.len() && widgets::menu::item(ui, "Move down", "").clicked() {
            out.changed |= editor.move_row(row, row + 1);
            ui.close();
        }
        widgets::menu::line(ui);
        if widgets::menu::item(ui, "Copy", "Ctrl+C").clicked() {
            if let Some(text) = editor.copy() {
                ui.ctx().copy_text(text);
            }
            ui.close();
        }
        if widgets::menu::item(ui, "Clear", "Delete").clicked() {
            out.changed |= editor.clear();
            ui.close();
        }
        let label = if rows.start() == rows.end() {
            "Delete the row".to_string()
        } else {
            format!("Delete {} rows", rows.end() - rows.start() + 1)
        };
        if widgets::menu::item(ui, &label, "").clicked() {
            out.changed |= editor.delete_rows(rows);
            ui.close();
        }
    });
}

/// The box typed in over the cell being typed in; for a choice or one of
/// the project's names, a list under it of what fits what's typed.
fn typing_box(
    ui: &mut Ui,
    editor: &mut TableEditor,
    shown: &[usize],
    cell_rect: &dyn Fn(usize, usize) -> Rect,
    names: Option<&script::project_names::States>,
    out: &mut TableOut,
) {
    let Some(Typing { at, .. }) = editor.typing.clone() else {
        return;
    };
    let Some(index) = shown.iter().position(|r| *r == at.0) else {
        editor.typing = None;
        return;
    };
    let rect = cell_rect(index, at.1);
    let options: Option<Vec<String>> =
        match at.1.checked_sub(1).and_then(|c| editor.file.columns.get(c)) {
            Some(c) => match &c.kind {
                ColumnKind::Choice(options) => Some(options.clone()),
                ColumnKind::Name(kind) => Some(
                    names
                        .and_then(|n| n.get(kind.state()))
                        .cloned()
                        .unwrap_or_default(),
                ),
                _ => None,
            },
            None => None,
        };
    let Some(mut typing) = editor.typing.take() else {
        return;
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(2), tokens::field());
    let edit = egui::TextEdit::singleline(&mut typing.text)
        .frame(egui::Frame::NONE)
        .font(text::ui())
        .text_color(tokens::text_primary())
        .margin(egui::Margin::ZERO)
        .desired_width(rect.width() - 24.0)
        .lock_focus(true);
    let line = Rect::from_center_size(
        pos2(rect.center().x, rect.center().y),
        vec2(rect.width() - 24.0, 16.0),
    );
    let response = ui.put(line, edit);
    if !typing.focused {
        response.request_focus();
        typing.focused = true;
    }
    let Some(options) = options else {
        editor.typing = Some(typing);
        return;
    };
    let words = typing.text.trim().to_lowercase();
    let fits: Vec<&String> = options
        .iter()
        .filter(|o| words.is_empty() || o.to_lowercase().contains(&words))
        .take(MAX_LISTED)
        .collect();
    let list = Rect::from_min_size(
        rect.left_bottom(),
        vec2(rect.width().max(200.0), fits.len() as f32 * 24.0 + 8.0),
    );
    let mut picked = None;
    egui::Area::new(egui::Id::new("table options list"))
        .fixed_pos(list.min)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(list.width() - 12.0);
                for option in &fits {
                    if field::list_row(ui, option, false).clicked() {
                        picked = Some((*option).clone());
                    }
                }
                if fits.is_empty() {
                    field::list_note(ui, "Nothing fits what's typed.");
                }
            });
        });
    let done = picked.is_some();
    if let Some(option) = picked {
        typing.text = option;
    }
    editor.typing = Some(typing);
    if done {
        match editor.confirm() {
            Ok(()) => out.changed = true,
            Err(why) => out.said = Some(why),
        }
    }
}

/// The grid's keys, while the tab has them.
fn keys(ui: &mut Ui, editor: &mut TableEditor, shown: &[usize], out: &mut TableOut) {
    if shown.is_empty() && editor.typing.is_none() {
        if ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)))) {
            paste(ui, editor, out);
        }
        return;
    }
    let pressed = |ui: &mut Ui, key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    let times = |ui: &mut Ui, m: Modifiers, key: Key| {
        ui.input_mut(|i| i.count_and_consume_key(m, key)) as isize
    };
    for (key, rows, places) in [
        (Key::ArrowUp, -1, 0),
        (Key::ArrowDown, 1, 0),
        (Key::ArrowLeft, 0, -1),
        (Key::ArrowRight, 0, 1),
        (Key::Tab, 0, 1),
    ] {
        let n = times(ui, Modifiers::NONE, key);
        if n > 0 {
            editor.step(rows * n, places * n, false);
        }
        let n = times(ui, Modifiers::SHIFT, key);
        if n > 0 && key == Key::Tab {
            editor.step(0, -n, false);
        } else if n > 0 {
            editor.step(rows * n, places * n, true);
        }
    }
    if pressed(ui, Key::Enter) || pressed(ui, Key::F2) {
        let at = editor.cursor;
        let yes_no = at.is_some_and(|(_, p)| {
            p > 0
                && editor
                    .file
                    .columns
                    .get(p - 1)
                    .is_some_and(|c| c.kind == ColumnKind::YesNo)
        });
        match at {
            Some(at) if yes_no => out.changed |= flip(editor, at),
            _ => editor.start_typing(None),
        }
    }
    if pressed(ui, Key::Delete) {
        out.changed |= editor.clear();
    }
    if pressed(ui, Key::Escape) {
        editor.anchor = None;
    }
    let (copy, paste_now, typed) = ui.input(|i| {
        let copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy));
        let paste = i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)));
        let typed = i.events.iter().find_map(|e| match e {
            egui::Event::Text(t) if !t.trim().is_empty() && !i.modifiers.command => Some(t.clone()),
            _ => None,
        });
        (copy, paste, typed)
    });
    if copy && let Some(text) = editor.copy() {
        ui.ctx().copy_text(text);
    }
    if paste_now {
        paste(ui, editor, out);
    }
    if let Some(first) = typed {
        editor.start_typing(Some(first));
    }
}

/// Pastes what the computer's clipboard gave this frame.
fn paste(ui: &mut Ui, editor: &mut TableEditor, out: &mut TableOut) {
    let text = ui.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Paste(t) => Some(t.clone()),
            _ => None,
        })
    });
    if let Some(text) = text {
        out.changed |= editor.paste(&text);
    }
}

/// The keys while a cell is typed in: Enter keeps it and moves down, Tab
/// keeps it and moves across, Esc leaves the cell as it was.
fn typing_keys(ui: &mut Ui, editor: &mut TableEditor, out: &mut TableOut) {
    let take = |ui: &mut Ui, m: Modifiers, key: Key| ui.input_mut(|i| i.consume_key(m, key));
    if take(ui, Modifiers::NONE, Key::Escape) {
        editor.typing = None;
        ui.memory_mut(|m| m.request_focus(grid_id()));
        return;
    }
    let moves = if take(ui, Modifiers::NONE, Key::Enter) {
        Some((1, 0))
    } else if take(ui, Modifiers::NONE, Key::Tab) {
        Some((0, 1))
    } else if take(ui, Modifiers::SHIFT, Key::Tab) {
        Some((0, -1))
    } else {
        None
    };
    let Some((rows, places)) = moves else {
        return;
    };
    match editor.confirm() {
        Ok(()) => {
            out.changed = true;
            editor.step(rows, places, false);
            ui.memory_mut(|m| m.request_focus(grid_id()));
        }
        Err(why) => out.said = Some(why),
    }
}
