//! Renaming in place (board 04, decision 0118): the name under a tile, on
//! a list's row or in a folder tree turns into a box with the name picked
//! out. `Enter` keeps what's typed, `Esc` puts the old name back, and
//! clicking away keeps it too. A name that won't do (empty, with a
//! character Windows forbids, or one already in the folder) is said under
//! the box while it's typed, and if it's kept anyway the old name stays
//! and the Console says why. The file moves with its ID file, so
//! everything that uses it still finds it.

use super::{Files, LibraryRequest, LibraryState, join, last_part, parent_of};
use crate::tokens::{self, size, text};
use egui::{Rect, Ui, pos2};

/// The most characters a name has (`asset_db::MAX_NAME_CHARS`).
const MAX_NAME_CHARS: usize = 100;

/// Characters Windows forbids in a name.
const FORBIDDEN: &str = "<>:\"/\\|?*";

/// `typed`, trimmed, if it can name a file or folder in a folder where
/// `taken` says which names are used already (in any letter case);
/// otherwise why not, in plain words.
pub(crate) fn checked_name(typed: &str, taken: impl Fn(&str) -> bool) -> Result<String, String> {
    let name = typed.trim();
    if name.is_empty() {
        return Err("a name can't be empty".into());
    }
    if name.starts_with('.') {
        return Err("a name can't start with a dot (the file would be hidden)".into());
    }
    if name.chars().any(|c| FORBIDDEN.contains(c)) {
        return Err("a name can't have < > : \" / \\ | ? or * in it (Windows forbids them)".into());
    }
    if name.chars().any(|c| {
        c.is_control() || matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
    }) {
        return Err("a name can't have a control or text-direction character in it".into());
    }
    if name.ends_with('.') {
        return Err("a name can't end with a dot on Windows".into());
    }
    if assets::reserved_on_windows(name) {
        return Err(format!(
            "\u{201c}{name}\u{201d} is a name Windows keeps for itself"
        ));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!("a name can be {MAX_NAME_CHARS} characters at most"));
    }
    if taken(&name.to_lowercase()) {
        return Err(format!(
            "there's already something called \u{201c}{name}\u{201d} in this folder"
        ));
    }
    Ok(name.to_owned())
}

/// The new path of the file or folder at `path`, shown as `shown`, when
/// `typed` is its new name: a file keeps its ending (`farm.png` renamed
/// `meadow` is `meadow.png`), so its kind never changes by renaming. Or
/// why the name won't do.
pub(crate) fn rename_to(files: Files<'_>, path: &str, typed: &str) -> Result<String, String> {
    let folder = parent_of(path);
    let file_name = last_part(path);
    let ending = files
        .items
        .at(path)
        .and_then(|item| file_name.strip_prefix(item.name.as_str()))
        .unwrap_or("");
    let own = file_name.to_lowercase();
    let taken = |lowered: &str| {
        let wanted = format!("{lowered}{}", ending.to_lowercase());
        wanted != own
            && (files
                .items
                .iter()
                .any(|i| i.folder() == folder && i.file_name().to_lowercase() == wanted)
                || files
                    .folders
                    .iter()
                    .any(|f| parent_of(f) == folder && last_part(f).to_lowercase() == wanted))
    };
    let name = checked_name(typed, taken)?;
    Ok(join(folder, &format!("{name}{ending}")))
}

/// Draws the box to rename the entry at `path` in `rect`, if it's the one
/// being renamed, and does what `Enter`, `Esc` or clicking away asks.
/// Returns whether it drew it.
pub(crate) fn name_box(
    ui: &mut Ui,
    rect: Rect,
    path: &str,
    (files, state): (Files<'_>, &mut LibraryState),
    requests: &mut Vec<LibraryRequest>,
) -> bool {
    let Some((renamed, mut typed)) = state.renaming.clone().filter(|(p, _)| p == path) else {
        return false;
    };
    let frame = ui.ctx().cumulative_frame_nr();
    if state.rename_frame == frame {
        return false;
    }
    state.rename_frame = frame;
    let id = egui::Id::new(("library rename", path));
    let first = !state.rename_focused;
    state.rename_focused = true;
    let (_, answer) =
        crate::widgets::rows::rename_in(ui, rect, id, &mut typed, text::tile_name(), first);
    let checked = rename_to(files, &renamed, &typed);
    if let Err(why) = &checked {
        why_not(ui, rect, why);
    }
    match answer {
        crate::widgets::rows::Renamed::Leave => state.renaming = None,
        crate::widgets::rows::Renamed::Keep => {
            state.renaming = None;
            finish(state, &renamed, &typed, checked, requests);
        }
        crate::widgets::rows::Renamed::Typing => state.renaming = Some((renamed, typed)),
    }
    true
}

/// Renames `path` to what `checked` found, or says why it can't be.
fn finish(
    state: &mut LibraryState,
    path: &str,
    typed: &str,
    checked: Result<String, String>,
    requests: &mut Vec<LibraryRequest>,
) {
    match checked {
        Ok(to) if to == path => {}
        Ok(to) => {
            requests.push(LibraryRequest::Move(path.to_owned(), to.clone()));
            // What's picked, and the folder open, follow the new name.
            if state.selected.as_deref() == Some(path) {
                state.selected = Some(to.clone());
            }
            if state.also.remove(path) {
                state.also.insert(to.clone());
            }
            if state.folder == path || state.folder.starts_with(&format!("{path}/")) {
                state.folder = format!("{to}{}", &state.folder[path.len()..]);
            }
        }
        // Clicked away without changing it: nothing to say.
        Err(_) if typed.trim() == last_part(path) => {}
        Err(why) => requests.push(LibraryRequest::Say(format!(
            "Kept the name {}: {why}.",
            last_part(path)
        ))),
    }
}

/// Why the name typed won't do, in a small box under the box being typed
/// in.
fn why_not(ui: &Ui, rect: Rect, why: &str) {
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        ui.id().with("why not"),
    ));
    let galley = painter.layout(
        why.to_owned(),
        text::tile_kind(),
        tokens::text_primary(),
        rect.width().max(size::LIBRARY_TILE_W * 2.0),
    );
    let at = pos2(rect.left(), rect.bottom() + size::ROW_GAP);
    let back = Rect::from_min_size(at, galley.size()).expand(size::ROW_GAP);
    painter.rect_filled(back, size::radius::SMALL, tokens::error_row());
    painter.galley(at, galley, tokens::text_primary());
    // Said to screen readers (and tests) too.
    let said = ui.interact(back, ui.id().with("why"), egui::Sense::hover());
    said.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, why));
}

/// Starts renaming `path` (`F2`, a double-click on a name, the menu's
/// Rename, or a new file arriving) with its name as the Library shows it.
pub(crate) fn begin(state: &mut LibraryState, files: Files<'_>, path: &str) {
    let name = files
        .items
        .at(path)
        .map_or_else(|| last_part(path).to_owned(), |item| item.name.clone());
    state.start_renaming(path, &name);
}
