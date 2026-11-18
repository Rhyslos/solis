//! The browser itself (board 04): the folder tree on the left, and beside
//! it back, up and the breadcrumb, how many items there are and how many
//! picked, then the files as tiles or a list. Its tools (the search, the
//! kind filter, grid or list) sit in its own header on the Library tab and
//! in the bar above it elsewhere ([`tools`]), so the two places work the
//! same (decision 0118: opening or closing other panels never changes
//! what it does).

use super::shown::{folder_name, shown};
use super::{
    Entry, Files, Filter, LibraryDrag, LibraryRequest, LibraryState, Place, join, menus,
    moves_into, rename, tiles, tree,
};
use crate::icons::{self, Icon};
use crate::tokens::{self, size, text};
use egui::{Align, Layout, Rect, Response, Sense, Ui, UiBuilder, vec2};

/// The browser filling `ui`.
pub(crate) fn browser(
    ui: &mut Ui,
    files: Files<'_>,
    state: &mut LibraryState,
    place: Place,
    requests: &mut Vec<LibraryRequest>,
) {
    let mut body = ui.available_rect_before_wrap();
    if place == Place::Tab {
        let bar = body.with_max_y(body.top() + size::COLUMN_HEADER_H);
        header(ui, bar, files, state);
        body = body.with_min_y(bar.bottom());
    }
    let files_rect = if state.folders_shown {
        let width = size::LIBRARY_TREE_W.min(body.width() * 0.4).round();
        let left = body.with_max_x(body.left() + width);
        let mut tree_ui = ui.new_child(UiBuilder::new().max_rect(left).id_salt("library tree"));
        tree::folder_tree(&mut tree_ui, files, state, requests);
        tokens::vline(ui.painter(), left.right(), left.y_range(), tokens::line());
        body.with_min_x(left.right() + size::LINE)
    } else {
        body
    };
    let least = match place {
        Place::Tab => size::LIBRARY_TILE_W,
        Place::Under => size::LIBRARY_TILE_W_UNDER,
    };
    files_area(ui, files_rect, (files, least), state, requests);
    ui.advance_cursor_after_rect(body);
}

/// The Library tab's own header (board 04): "Library" and the project's
/// name, and the tools at the right.
fn header(ui: &mut Ui, bar: Rect, files: Files<'_>, state: &mut LibraryState) {
    ui.painter().rect_filled(bar, 0.0, tokens::code());
    tokens::hline(ui.painter(), bar.x_range(), bar.bottom(), tokens::line());
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(bar.shrink2(vec2(size::ROW_PAD, 0.0)))
            .layout(Layout::left_to_right(Align::Center))
            .id_salt("library header"),
    );
    row.spacing_mut().item_spacing.x = size::GAP;
    row.label(
        egui::RichText::new("Library")
            .font(text::header())
            .color(tokens::text_primary()),
    );
    row.label(
        egui::RichText::new(&files.project.name)
            .font(text::ui())
            .color(tokens::text_meta()),
    );
    row.with_layout(Layout::right_to_left(Align::Center), |ui| {
        tools(ui, state, &files.project.name);
    });
}

/// The browser's tools, laid out from the right (board 04): grid or list,
/// the kind filter and the search. The same wherever the browser is.
pub(crate) fn tools(ui: &mut Ui, state: &mut LibraryState, project: &str) {
    ui.spacing_mut().item_spacing.x = size::GAP;
    view_switch(ui, state);
    let kinds: Vec<(Filter, &str)> = Filter::ALL
        .iter()
        .map(|f| {
            (
                *f,
                if *f == Filter::All {
                    "All kinds"
                } else {
                    f.name()
                },
            )
        })
        .collect();
    // In a narrow column the filter shrinks and the search gives way.
    let kinds_w = size::LIBRARY_KINDS_W.min(ui.available_width() * 0.5);
    crate::widgets::field::dropdown(ui, "Kind of file", &mut state.filter, &kinds, kinds_w);
    let width = size::LIBRARY_SEARCH_W.min(ui.available_width() - size::GAP);
    if width < size::LIBRARY_KINDS_W {
        return;
    }
    crate::widgets::field::search_field(
        ui,
        "Search the Library",
        &mut state.search,
        &format!("Search {project}"),
        width,
    );
}

/// Tiles or a list: two small buttons, the one in force lit.
fn view_switch(ui: &mut Ui, state: &mut LibraryState) {
    for (list, icon, name) in [
        (true, Icon::Lines, "Show as a list"),
        (false, Icon::Grid, "Show as tiles"),
    ] {
        let (rect, response) =
            ui.allocate_exact_size(vec2(size::ICON_BUTTON, size::ICON_BUTTON), Sense::click());
        let on = state.list == list;
        response
            .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, name));
        if on || response.hovered() {
            ui.painter()
                .rect_filled(rect, size::radius::CONTROL, tokens::card_border());
        }
        let ink = if on {
            tokens::text_primary()
        } else {
            tokens::text_label()
        };
        let spot = Rect::from_center_size(rect.center(), vec2(size::ICON_SMALL, size::ICON_SMALL));
        icons::icon(ui.painter(), spot, icon, ink);
        if response.on_hover_text(name).clicked() {
            state.list = list;
        }
    }
}

/// Back, up and the breadcrumb with the counts, then the files.
fn files_area(
    ui: &mut Ui,
    area: Rect,
    (files, least): (Files<'_>, f32),
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let entries = shown(files.items, files.folders, state);
    follow_renaming(files, state, &entries);
    let nav = area.with_max_y(area.top() + size::LIBRARY_NAV_H);
    let content = area
        .with_min_y(nav.bottom())
        .shrink2(vec2(size::ROW_PAD, size::ROW_GAP));
    // Right-clicking where there's no file: what can be made here; a click
    // there lets go of what's picked. Files drawn over it have their own.
    let space = ui.interact(content, ui.id().with("empty space"), Sense::click());
    if space.clicked() {
        state.selected = None;
        state.also.clear();
    }
    let folder = state.folder.clone();
    space.context_menu(|ui| menus::space_menu(ui, &folder, &files.project.name, requests));
    nav_row(ui, nav, (files, entries.len()), state, requests);
    let mut inner = ui.new_child(UiBuilder::new().max_rect(content).id_salt("library files"));
    if entries.is_empty() {
        crate::panels::empty_note(&mut inner, &empty_words(files, state));
    } else if state.list {
        tiles::list(&mut inner, &entries, (files, state), requests);
    } else {
        tiles::grid(&mut inner, &entries, (files, state, least), requests);
    }
}

/// What an empty view says.
fn empty_words(files: Files<'_>, state: &LibraryState) -> String {
    if !state.search.trim().is_empty() {
        "Nothing has that name.".to_owned()
    } else if state.filter != Filter::All {
        format!("No {} here.", state.filter.name().to_lowercase())
    } else if files.items.is_empty() && state.folder.is_empty() {
        format!(
            "{} has no files yet. Right-click here to make a folder, a level or a code file, \
             or drop files onto the window.",
            files.project.name
        )
    } else {
        "This folder is empty. Right-click to make something in it.".to_owned()
    }
}

/// A file just made starts being renamed once it's listed (board 04's
/// "Crops"); a name being typed for something that has gone is dropped.
fn follow_renaming(files: Files<'_>, state: &mut LibraryState, entries: &[Entry<'_>]) {
    if let Some(new) = state.rename_new.clone()
        && entries.iter().any(|e| e.path() == new)
    {
        state.rename_new = None;
        state.pick(&new);
        rename::begin(state, files, &new);
    }
    let gone = state.renaming.as_ref().is_some_and(|(path, _)| {
        files.items.at(path).is_none() && !files.folders.iter().any(|f| f == path)
    });
    if gone {
        state.renaming = None;
    }
}

/// ‹ (back), ˄ (up) and "Farm Days › farm", each part a place to drop
/// files, and "14 items · 1 selected" at the right.
fn nav_row(
    ui: &mut Ui,
    nav: Rect,
    (files, count): (Files<'_>, usize),
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    // The counts first, at the right, so a long breadcrumb never runs
    // over them.
    let room = nav.shrink2(vec2(size::ROW_GAP, 0.0));
    let picked = state.picked().len();
    let items = if count == 1 { "item" } else { "items" };
    let counts = format!("{count} {items} \u{b7} {picked} selected");
    let counts_w = crate::widgets::width_of(ui, &counts, text::ui()) + size::GAP;
    // In a narrow column they give way to the breadcrumb.
    let fits = counts_w <= room.width() / 2.0;
    let counts_rect = room.with_min_x(room.right() - if fits { counts_w } else { 0.0 });
    if fits {
        let mut right = ui.new_child(
            UiBuilder::new()
                .max_rect(counts_rect)
                .layout(Layout::right_to_left(Align::Center))
                .id_salt("library counts"),
        );
        quiet(&mut right, &counts);
    }
    let left = room.with_max_x(counts_rect.left());
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(left)
            .layout(Layout::left_to_right(Align::Center))
            .id_salt("library nav"),
    );
    row.set_clip_rect(left.intersect(ui.clip_rect()));
    row.spacing_mut().item_spacing.x = size::ROW_GAP;
    let back = row
        .add_enabled_ui(!state.back.is_empty(), |ui| {
            crate::widgets::button::icon(ui, Icon::ChevronLeft, "Back")
        })
        .inner;
    if back.clicked() {
        state.go_back();
    }
    let up = row
        .add_enabled_ui(!state.folder.is_empty() && state.search.is_empty(), |ui| {
            crate::widgets::button::icon(ui, Icon::ChevronUp, "Up one folder")
        })
        .inner;
    if up.clicked() {
        state.go_up();
    }
    if state.search.trim().is_empty() {
        breadcrumb(&mut row, files, state, requests);
    } else {
        quiet(&mut row, "Found in the whole project");
    }
}

/// "Farm Days › farm › animals", each part opening its folder and taking
/// files dropped on it.
fn breadcrumb(
    ui: &mut Ui,
    files: Files<'_>,
    state: &mut LibraryState,
    requests: &mut Vec<LibraryRequest>,
) {
    let mut parts = vec![String::new()];
    let mut so_far = String::new();
    for part in state.folder.split('/').filter(|p| !p.is_empty()) {
        so_far = join(&so_far, part);
        parts.push(so_far.clone());
    }
    let last = parts.len() - 1;
    let mut go = None;
    for (i, folder) in parts.iter().enumerate() {
        if i > 0 {
            quiet(ui, "\u{203a}");
        }
        let words = folder_name(folder, &files.project.name);
        let ink = if i == last {
            tokens::text_emphasis()
        } else {
            tokens::text_label()
        };
        let response = ui.add(
            egui::Label::new(egui::RichText::new(words).font(text::ui()).color(ink))
                .selectable(false)
                .sense(Sense::click()),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Folder {words}"))
        });
        if response.clicked() {
            go = Some(folder.clone());
        }
        take_moves(&response, folder, requests);
    }
    if let Some(folder) = go {
        state.open_folder(&folder);
    }
}

/// Files dropped on `response` move into `folder`; while they're over it,
/// it's outlined.
pub(crate) fn take_moves(response: &Response, folder: &str, requests: &mut Vec<LibraryRequest>) {
    if let Some(dragged) = response.dnd_release_payload::<LibraryDrag>() {
        requests.extend(moves_into(folder, &dragged.1));
    } else if response.dnd_hover_payload::<LibraryDrag>().is_some() {
        tiles::drop_mark(response, crate::theme::FOLDER);
    }
}

/// Quiet words that can't be picked.
fn quiet(ui: &mut Ui, words: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(words)
                .font(text::ui())
                .color(tokens::text_meta()),
        )
        .selectable(false),
    );
}
