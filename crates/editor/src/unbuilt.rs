//! Unfinished parts: the one way the editor shows something the design
//! has and Solis doesn't do yet
//! ([decision 0101](../../../docs/decisions/0101-unfinished-parts-as-placeholders.md)).
//!
//! - **One table** (`unbuilt/table.rs`) names every unfinished part:
//!   its words, the roadmap phase that builds it, and a plain sentence
//!   about what it will do. When a part is built its line is deleted,
//!   and the compiler points at every place that still draws it.
//! - **One way to draw** (`unbuilt/draw.rs`, and `unbuilt/page.rs` for a
//!   whole page): a function for each shape
//!   the frame has (a ribbon button, a shelf tab, a setting row, a whole
//!   page ...), each taking the part's id and nothing about looks. A part
//!   is drawn in place and in shape, in quieter greys, with a small
//!   hollow square mark; pointing at it says what it will do and when; a
//!   click only says, in the status bar, that it isn't built.
//! - **One switch hides them all** (`InterfaceLook::hide_unfinished`,
//!   "Hide unfinished parts"). Hiding is done on data, before anything is
//!   laid out: a list of things to draw is made of [`Entry::Built`] and
//!   [`Entry::Unbuilt`], and [`filter`] takes the unbuilt ones out, so
//!   what is left closes up with no gaps.
//!
//! Nothing outside this file and its folder may draw an unfinished part
//! or say that something isn't built: a test reads the source to check
//! (`source_rules_tests.rs`).

mod draw;
mod page;
#[cfg(test)]
pub(crate) mod sheet;
mod table;

pub use draw::{
    RowKind, card, chip, column, field_row, icon_button, link, menu_item, ribbon_big, ribbon_row,
    ribbon_row_width, setting_row, shelf_tab, status_item, sub_tab, tab, tool,
};
pub use page::{PageAnswer, page};
pub use table::{ALL, UnbuiltId};

use crate::interface_look;
use egui::{Context, Id};
use std::collections::BTreeSet;

/// The roadmap phase that builds an unfinished part: one of the phases
/// still to come, or none yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Game code: finding and making things.
    Shared2,
    /// Time and feel.
    Shared3,
    /// Data and the editor.
    Shared4,
    /// Screens, words and sound.
    Shared5,
    /// Worlds and testing.
    Shared6,
    /// Materials and shaders.
    Shared7,
    /// Effects.
    Shared8,
    /// Weather and the finished picture.
    Shared9,
    /// Cutscenes and behaviour.
    Shared10,
    /// Scripting, full export, Git and mods.
    Shared11,
    /// Netcode.
    Shared12,
    /// The first release.
    Shared13,
    /// The world and tiles.
    TwoD1,
    /// Movement, light and polish.
    TwoD2,
    /// 3D animation and navigation.
    ThreeD1,
    /// Real light and the sky.
    ThreeD2,
    /// Night, clouds and shadows.
    ThreeD3,
    /// Lamps, fog and reflections.
    ThreeD4,
    /// Bounced light.
    ThreeD5,
    /// No phase of the roadmap builds it yet.
    NotPlanned,
}

impl Phase {
    /// Every phase.
    pub const ALL: [Self; 20] = [
        Self::Shared2,
        Self::Shared3,
        Self::Shared4,
        Self::Shared5,
        Self::Shared6,
        Self::Shared7,
        Self::Shared8,
        Self::Shared9,
        Self::Shared10,
        Self::Shared11,
        Self::Shared12,
        Self::Shared13,
        Self::TwoD1,
        Self::TwoD2,
        Self::ThreeD1,
        Self::ThreeD2,
        Self::ThreeD3,
        Self::ThreeD4,
        Self::ThreeD5,
        Self::NotPlanned,
    ];

    /// The phase as the roadmap heads it ("Shared 7: Materials and
    /// shaders"), or `None` when no phase builds the part yet. A test
    /// reads `ROADMAP.md` and fails if a title isn't a heading there, so
    /// these words can't fall behind a renamed phase.
    pub fn title(self) -> Option<&'static str> {
        Some(match self {
            Self::Shared2 => "Shared 2: Game code: finding and making things",
            Self::Shared3 => "Shared 3: Time and feel",
            Self::Shared4 => "Shared 4: Data and the editor",
            Self::Shared5 => "Shared 5: Screens, words and sound",
            Self::Shared6 => "Shared 6: Worlds and testing",
            Self::Shared7 => "Shared 7: Materials and shaders",
            Self::Shared8 => "Shared 8: Effects",
            Self::Shared9 => "Shared 9: Weather and the finished picture",
            Self::Shared10 => "Shared 10: Cutscenes and behaviour",
            Self::Shared11 => "Shared 11: Scripting, full export, Git and mods",
            Self::Shared12 => "Shared 12: Netcode",
            Self::Shared13 => FIRST_RELEASE,
            Self::TwoD1 => "2D 1: The world and tiles",
            Self::TwoD2 => "2D 2: Movement, light and polish",
            Self::ThreeD1 => "3D 1: 3D animation and navigation",
            Self::ThreeD2 => "3D 2: Real light and the sky",
            Self::ThreeD3 => "3D 3: Night, clouds and shadows",
            Self::ThreeD4 => "3D 4: Lamps, fog and reflections",
            Self::ThreeD5 => "3D 5: Bounced light",
            Self::NotPlanned => return None,
        })
    }
}

/// The last phase's number, without the name the roadmap gives the
/// release (the program's name is never written in code: rule 16). The
/// test that reads the roadmap matches this one by how it starts.
pub(crate) const FIRST_RELEASE: &str = "Shared 13";

/// One unfinished part, as the table has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unbuilt {
    /// Its name in code.
    pub id: UnbuiltId,
    /// Its dotted id in the plan's inventory ("home.lighting.bake").
    pub key: &'static str,
    /// The words shown on it.
    pub name: &'static str,
    /// The phase that builds it.
    pub phase: Phase,
    /// What it will do, in one plain sentence (no full stop).
    pub what: &'static str,
}

/// The table's line for `id`.
pub fn get(id: UnbuiltId) -> &'static Unbuilt {
    // `ALL` is in the enum's own order (a test checks).
    &ALL[id as usize]
}

/// What an unfinished part is described as, for screen readers and for
/// the tests that look for every one on screen.
pub const DESCRIPTION: &str = "Not built yet";

impl Unbuilt {
    /// What pointing at the part says: "Not built yet: Bake. Works out
    /// bounced light for the area. Planned for 3D 5: Bounced light."
    pub fn hover_text(&self) -> String {
        let when = match self.phase.title() {
            Some(title) => format!("Planned for {title}."),
            None => "Not planned yet.".to_owned(),
        };
        format!("{DESCRIPTION}: {}. {}. {when}", self.name, self.what)
    }

    /// What a click on the part says in the status bar: "Bake isn't built
    /// yet (3D 5: Bounced light)".
    pub fn click_text(&self) -> String {
        let when = self.phase.title().unwrap_or("not planned yet");
        format!("{} isn't built yet ({when})", self.name)
    }

    /// The line on a whole unbuilt page: "Not built yet · planned for
    /// Shared 7: Materials and shaders".
    pub fn page_line(&self) -> String {
        match self.phase.title() {
            Some(title) => format!("{DESCRIPTION} \u{b7} planned for {title}"),
            None => format!("{DESCRIPTION} \u{b7} not planned yet"),
        }
    }
}

/// One thing in a list of things to draw: a part that works, or one
/// that isn't built yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry<T> {
    /// A part that works, as the list's owner describes it.
    Built(T),
    /// An unfinished part.
    Unbuilt(UnbuiltId),
}

/// Whether unfinished parts are hidden (Preferences, "Hide unfinished
/// parts").
pub fn hidden() -> bool {
    interface_look::in_use().hide_unfinished
}

/// `entries` as they are to be drawn: all of them, or, with unfinished
/// parts hidden, only the ones that work. Called on a list before it is
/// laid out, so what is left closes up.
pub fn filter<T>(entries: Vec<Entry<T>>) -> Vec<Entry<T>> {
    filter_when(entries, hidden())
}

/// As [`filter`], with the setting handed in.
pub fn filter_when<T>(entries: Vec<Entry<T>>, hide: bool) -> Vec<Entry<T>> {
    if hide {
        entries
            .into_iter()
            .filter(|entry| matches!(entry, Entry::Built(_)))
            .collect()
    } else {
        entries
    }
}

/// Groups of entries (a ribbon's groups, a page's sections) as they are
/// to be drawn: each group's entries filtered, and then, with unfinished
/// parts hidden, any group left with nothing taken out too.
pub fn filter_groups<G, T>(groups: Vec<(G, Vec<Entry<T>>)>) -> Vec<(G, Vec<Entry<T>>)> {
    filter_groups_when(groups, hidden())
}

/// As [`filter_groups`], with the setting handed in.
pub fn filter_groups_when<G, T>(
    groups: Vec<(G, Vec<Entry<T>>)>,
    hide: bool,
) -> Vec<(G, Vec<Entry<T>>)> {
    groups
        .into_iter()
        .map(|(group, entries)| (group, filter_when(entries, hide)))
        .filter(|(_, entries)| !(hide && entries.is_empty()))
        .collect()
}

/// The unfinished parts drawn in one frame, kept with the window.
#[derive(Debug, Clone, Default)]
struct Drawn {
    /// The frame they were drawn in.
    pass: u64,
    ids: BTreeSet<UnbuiltId>,
}

fn drawn_id() -> Id {
    Id::new("unfinished parts drawn")
}

/// Notes that `id` is being drawn in this frame.
pub(crate) fn note(ctx: &Context, id: UnbuiltId) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let drawn = data.get_temp_mut_or_default::<Drawn>(drawn_id());
        if drawn.pass != pass {
            drawn.pass = pass;
            drawn.ids.clear();
        }
        drawn.ids.insert(id);
    });
}

/// The unfinished parts the frame just finished (or the one being drawn)
/// drew. The tests walk the editor with this: every part in the table is
/// drawn somewhere, and none while they are hidden.
pub fn drawn(ctx: &Context) -> BTreeSet<UnbuiltId> {
    let now = ctx.cumulative_pass_nr();
    ctx.data(|data| data.get_temp::<Drawn>(drawn_id()))
        .filter(|drawn| drawn.pass + 1 >= now)
        .map(|drawn| drawn.ids)
        .unwrap_or_default()
}

/// How long the status bar keeps saying that a clicked part isn't built.
pub const SAID_FOR: f64 = 3.0;

/// A click on an unfinished part: which, and when.
#[derive(Debug, Clone, Copy)]
struct Said {
    id: UnbuiltId,
    at: f64,
}

fn said_id() -> Id {
    Id::new("unfinished part clicked")
}

/// Notes a click on `id`: the status bar says so for a moment. Nothing
/// else happens: no window, no sound, no line in the Console.
pub(crate) fn say(ctx: &Context, id: UnbuiltId) {
    let at = ctx.input(|input| input.time);
    ctx.data_mut(|data| data.insert_temp(said_id(), Said { id, at }));
    ctx.request_repaint();
}

/// What the status bar says about the last unfinished part clicked, for
/// three seconds after the click: "Bake isn't built yet (3D 5: Bounced
/// light)".
pub fn said(ctx: &Context) -> Option<String> {
    let said = ctx.data(|data| data.get_temp::<Said>(said_id()))?;
    let left = SAID_FOR - (ctx.input(|input| input.time) - said.at);
    if left <= 0.0 || left > SAID_FOR {
        return None;
    }
    // Drawn again when the time is up, so the words go without anyone
    // moving the mouse.
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
    Some(get(said.id).click_text())
}

#[cfg(test)]
#[path = "unbuilt_tests.rs"]
mod tests;
