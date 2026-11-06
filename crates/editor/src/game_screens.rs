//! The game's screens while it plays ([decision 0087](../../../docs/decisions/0087-screens-through-the-engine.md)):
//! the screens' own Luau started with the program, their views run after
//! each tick (the game-wide objects copied in first), and each entity's
//! `Screen` that's shown drawn over the Game panel in its order. A screen
//! that fails keeps its last picture, and the Console says where, once
//! for each new message.

use crate::console::LogLevel;
use crate::workbench::Workbench;
use reflect::Component;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The screens' side of playing.
#[derive(Default)]
pub(crate) struct GameScreens {
    /// The screens running, and the program they were started from.
    running: Option<(script::Screens, Arc<script::Compiled>)>,
    /// What's drawn over the Game panel now, in order.
    pub(crate) pictures: Vec<Arc<ui::Screen>>,
    /// Which screen each picture is, and the entity it's shown for.
    shown: Vec<(reflect::EntityId, String)>,
    /// The button under the pointer: its picture and its number.
    pub(crate) hover: Option<(usize, usize)>,
    /// The button with the keys' focus (M3b): its picture and its number.
    pub(crate) focus: Option<(usize, usize)>,
    /// Whether the menu keys were used since the mouse last moved over a
    /// button: the focus ring shows only then.
    keys_used: bool,
    /// Which way MenuMove points, and for how many ticks, to move on
    /// once and then again while it's held.
    way: Option<([i8; 2], u32)>,
    /// Where the screens were laid out last, on the Game panel.
    pub(crate) area: ScreensArea,
    /// What buttons sent since the last tick, for the next one's input
    /// (decision 0088).
    pub(crate) sent: Vec<input::Sent>,
    /// Whether the pointer is free over the game while it has the keys,
    /// for a screen with buttons.
    pub(crate) freed: bool,
    /// The editor's own fonts, to measure text where buttons are (the
    /// renderer draws with the same).
    fonts: Option<ui::Fonts>,
    /// Each screen's last good picture, by name.
    last: BTreeMap<String, Arc<ui::Screen>>,
    /// Messages said already, so a failing screen isn't said every tick.
    told: BTreeSet<String>,
}

impl std::fmt::Debug for GameScreens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameScreens")
            .field("running", &self.running.is_some())
            .field("pictures", &self.pictures.len())
            .finish_non_exhaustive()
    }
}

impl GameScreens {
    /// Forgets everything (Stop, or no program); the fonts are kept.
    pub(crate) fn clear(&mut self) {
        let (fonts, freed) = (self.fonts.take(), self.freed);
        *self = Self::default();
        self.fonts = fonts;
        // Still free until the game gives the mouse and keys back.
        self.freed = freed;
    }

    /// Whether a screen shown has something to click: the pointer is then
    /// free over the game, even while it has the keys.
    pub(crate) fn wants_pointer(&self) -> bool {
        self.pictures
            .iter()
            .any(|p| p.view.as_ref().is_some_and(ui::view::has_actions))
    }

    /// Finds the button at `at` (pixels from the top left of `area`, the
    /// part of the Game panel's picture screens are laid out on, with its
    /// whole pixel step for a 2D game), topmost screen first, and shows it
    /// hovered. `None` (the pointer is elsewhere) hovers nothing.
    pub(crate) fn point_at(&mut self, at: Option<[f32; 2]>, area: ScreensArea) {
        self.area = area;
        let fonts = self.fonts.get_or_insert_with(editor_fonts);
        let found = at.and_then(|at| {
            let at = [at[0] - area.origin[0] as f32, at[1] - area.origin[1] as f32];
            self.pictures.iter().enumerate().rev().find_map(|(i, p)| {
                ui::layout::clickable_at(p, area.size, area.step, at, fonts).map(|a| (i, a))
            })
        });
        if at.is_none() && self.hover.is_none() {
            return;
        }
        if found != self.hover {
            self.hover = found;
            // The keys carry on from the button the mouse is on.
            if found.is_some() {
                self.focus = found;
                self.keys_used = false;
            }
            self.show_looks();
        }
    }

    /// Each picture with the hovered button and the focused one (if
    /// they're on it) shown so; the focus ring only once the keys are used.
    fn show_looks(&mut self) {
        for (i, picture) in self.pictures.iter_mut().enumerate() {
            let on = |b: Option<(usize, usize)>| b.filter(|(p, _)| *p == i).map(|(_, a)| a);
            let looks = ui::view::Looks {
                hover: on(self.hover),
                focus: on(self.focus.filter(|_| self.keys_used)),
                text_percent: picture.looks.text_percent,
            };
            if picture.looks != looks {
                Arc::make_mut(picture).looks = looks;
            }
        }
    }

    /// The menu keys in this tick's input (decision 0088): MenuMove moves
    /// the focus to the nearest button that way (again every tenth of a
    /// second once held), MenuChoose gives back the focused button to press.
    /// Only while a screen with buttons is shown.
    fn menu_keys(&mut self, frame: &input::Frame) -> Option<(usize, usize)> {
        let state = |name: &str| frame.actions.get(name).copied().unwrap_or_default();
        let axis = state(input::MENU_MOVE).axis;
        // The way it points, on the screen (y down), one of four.
        let way = if axis.x.abs() >= axis.y.abs() && axis.x.abs() > 0.5 {
            Some([axis.x.signum() as i8, 0])
        } else if axis.y.abs() > 0.5 {
            Some([0, -axis.y.signum() as i8])
        } else {
            None
        };
        let held = match (way, self.way) {
            (Some(w), Some((was, n))) if w == was => n + 1,
            (Some(_), _) => 1,
            (None, _) => 0,
        };
        self.way = way.map(|w| (w, held));
        // Once, then after 0.4 s every 0.1 s (at 60 ticks a second).
        let step = held == 1 || held >= 24 && (held - 24) % 6 == 0;
        let choose = state(input::MENU_CHOOSE).pressed;
        if !(step || choose) {
            return None;
        }
        self.keys_used = true;
        // The focus is on the topmost screen with buttons.
        let top = (0..self.pictures.len()).rev().find(|&i| {
            self.pictures[i]
                .view
                .as_ref()
                .is_some_and(ui::view::has_actions)
        })?;
        let area = if self.area.size[0] == 0 || self.area.size[1] == 0 {
            ScreensArea {
                origin: [0, 0],
                size: [1920, 1080],
                step: None,
            }
        } else {
            self.area
        };
        let fonts = self.fonts.get_or_insert_with(editor_fonts);
        let (clickable, _) =
            ui::layout::clickables(&self.pictures[top], area.size, area.step, fonts);
        // With nothing focused yet, the first button is (so the first
        // press moves on from it, as a menu's first button is ready).
        let from = self
            .focus
            .filter(|(p, _)| *p == top)
            .map(|(_, a)| a)
            .or_else(|| clickable.first().map(|c| c.action));
        let towards = match way.filter(|_| step) {
            Some(w) => [f32::from(w[0]), f32::from(w[1])],
            None => [0.0, 0.0],
        };
        self.focus = ui::view::next_focus(&clickable, from, towards).map(|a| (top, a));
        self.show_looks();
        self.focus.filter(|_| choose)
    }
}

/// The editor's own Inter, to measure screens' text as the renderer draws it.
pub(crate) fn editor_fonts() -> ui::Fonts {
    ui::Fonts::new(vec![
        crate::theme::INTER_REGULAR.to_vec(),
        crate::theme::INTER_SEMIBOLD.to_vec(),
    ])
    .0
}

impl GameScreens {
    /// The menu keys in a tick's `frame`, while a screen with buttons is
    /// shown: the button they choose, if any, and the frame game code
    /// gets, holding only the menu actions (the game's own wait, so A
    /// choosing doesn't also jump).
    pub(crate) fn take_menu_keys(&mut self, frame: &mut input::Frame) -> Option<(usize, usize)> {
        if !self.wants_pointer() {
            self.way = None;
            return None;
        }
        let chosen = self.menu_keys(frame);
        frame
            .actions
            .retain(|name, _| input::MENU_ACTIONS.contains(&name.as_str()));
        chosen
    }
}

/// Where the game's screens are laid out on the Game panel's picture, in
/// its pixels: all of it, or a 2D game's picture inside its bars.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ScreensArea {
    /// Its top left corner.
    pub(crate) origin: [u32; 2],
    /// Its size.
    pub(crate) size: [u32; 2],
    /// A 2D game's whole pixel step.
    pub(crate) step: Option<u32>,
}

impl GameScreens {
    /// Whether the screens need starting (again) for `program`: its
    /// screens aren't the ones running, or there are none and pictures
    /// are left.
    pub(crate) fn out_of_date(&self, program: Option<&Arc<script::Compiled>>) -> bool {
        match (program.filter(|p| p.screens.is_some()), &self.running) {
            (Some(program), Some((_, from))) => !Arc::ptr_eq(program, from),
            (Some(_), None) => self.told.is_empty(),
            (None, _) => self.running.is_some() || !self.pictures.is_empty(),
        }
    }
}

/// The screens shown in `world`, in drawing order: each entity's Screen
/// that's shown and names one, by its order, then the area's order.
/// With each Screen component's look (decision 0089, M6).
pub(crate) fn shown_with_looks(
    world: &world::World,
) -> Vec<(reflect::EntityId, String, ui::ScreenLook)> {
    let mut shown: Vec<(i64, usize, reflect::EntityId, area::builtin::Screen)> = world
        .values_named(area::builtin::Screen::NAME)
        .into_iter()
        .enumerate()
        .map(|(i, (id, data))| (i, id, area::builtin::Screen::from_data(&data)))
        .filter(|(_, _, s)| s.shown && !s.screen.is_empty())
        .map(|(i, id, s)| (s.order, i, id, s))
        .collect();
    shown.sort_by_key(|(order, i, _, _)| (*order, *i));
    shown
        .into_iter()
        .map(|(_, _, id, s)| {
            let [r, g, b] = s.outline_color.map(|c| (c * 255.0).round() as u8);
            let look = ui::ScreenLook {
                glow: s.glow,
                outline: s.outline,
                outline_colour: ui::Colour::rgb(r, g, b),
                pixel: s.pixel_size as f32,
            };
            (id, s.screen, look)
        })
        .collect()
}

/// What screens read: the running game's components (Lust, Lua and
/// Rust), and Solis's own as the world has them.
struct GameReader<'a>(
    Option<&'a mut script::Game>,
    &'a world::World,
    script::settings::PlayerSettings,
);

impl script::ScreenReader for GameReader<'_> {
    fn values(&mut self, entity: reflect::EntityId, name: &str) -> Option<reflect::ComponentData> {
        if let Some(found) = self.0.as_deref_mut().and_then(|g| g.values(entity, name)) {
            return Some(found);
        }
        self.1.component_data(entity, name)
    }

    fn settings(&self) -> script::settings::PlayerSettings {
        self.2
    }
}

impl Workbench {
    /// Runs the shown screens' views again (after ticks, or when the code
    /// changed) and keeps their pictures for the Game panel.
    pub(crate) fn refresh_game_screens(&mut self) {
        let Some(playing) = &mut self.playing else {
            self.game_screens.clear();
            return;
        };
        let program = self.program.clone().filter(|p| p.screens.is_some());
        let Some(program) = program else {
            self.game_screens.clear();
            return;
        };
        let screens = &mut self.game_screens;
        let seen = &mut self.screen_pictures.seen;
        let shown_pictures = self.screen_pictures.pictures.clone();
        // Started with the program, and again when it changes.
        let fresh = !matches!(&screens.running, Some((_, from)) if Arc::ptr_eq(from, &program));
        let mut said = Vec::new();
        let mut warned = Vec::new();
        if fresh {
            screens.last.clear();
            screens.running = None;
            if let Some(code) = &program.screens {
                warned.extend(code.notes.iter().cloned());
                match script::Screens::start(code) {
                    Ok(started) => screens.running = Some((started, Arc::clone(&program))),
                    Err(failure) => {
                        said.push(format!("The game's screens couldn't start: {}", failure.at))
                    }
                }
            }
        }
        if let Some((running, _)) = &screens.running {
            if let Some(game) = &mut playing.game {
                running.set_objects(&game.object_values());
            }
            let mut pictures = Vec::new();
            let mut shown = Vec::new();
            let settings = playing.view.settings;
            let mut reader = GameReader(playing.game.as_mut(), &playing.world, settings);
            for (entity, name, look) in shown_with_looks(&playing.world) {
                match running.view(&name, Some(entity), &mut reader) {
                    Ok(view) => {
                        crate::screen_pictures::names_in(&view, seen);
                        let picture = Arc::new(ui::Screen {
                            name: name.clone(),
                            design: ui::view::design_of(&view),
                            scale: ui::ScaleRule::Grow,
                            elements: Vec::new(),
                            view: Some(view),
                            looks: ui::view::Looks {
                                text_percent: settings.text_percent(),
                                ..Default::default()
                            },
                            pictures: shown_pictures.clone(),
                            ui_size: settings.ui_size as f32,
                            look,
                        });
                        screens.last.insert(name.clone(), Arc::clone(&picture));
                        pictures.push(picture);
                        shown.push((entity, name));
                    }
                    Err(problem) => {
                        said.push(format!("The screen {name} stopped: {}", problem));
                        if let Some(last) = screens.last.get(&name) {
                            pictures.push(Arc::clone(last));
                            shown.push((entity, name));
                        }
                    }
                }
            }
            // The same button stays hovered and focused while its screen
            // stays.
            let by_screen = |b: Option<(usize, usize)>, shown: &[(reflect::EntityId, String)]| {
                b.and_then(|(i, a)| Some((shown.get(i)?.clone(), a)))
            };
            let hovered = by_screen(screens.hover, &screens.shown);
            let focused = by_screen(screens.focus, &screens.shown);
            screens.pictures = pictures;
            screens.shown = shown;
            let again = |b: Option<((reflect::EntityId, String), usize)>,
                         shown: &[(reflect::EntityId, String)]| {
                b.and_then(|(was, a)| Some((shown.iter().position(|s| *s == was)?, a)))
            };
            screens.hover = again(hovered, &screens.shown);
            screens.focus = again(focused, &screens.shown);
            screens.show_looks();
        }
        for line in warned {
            if self.game_screens.told.insert(line.clone()) {
                self.log(LogLevel::Warning, line);
            }
        }
        for line in said {
            if self.game_screens.told.insert(line.clone()) {
                self.log(LogLevel::Error, line);
            }
        }
    }

    /// The hovered button was clicked: runs its `{ }` (decision 0088),
    /// and what it sends waits for the next tick's input. Says in the
    /// Console if the screen stopped.
    pub(crate) fn press_hovered(&mut self) {
        if let Some(button) = self.game_screens.hover {
            self.press(button);
        }
    }

    /// Runs button `action` of picture `picture` (decision 0088); what it
    /// sends waits for the next tick's input.
    pub(crate) fn press(&mut self, (picture, action): (usize, usize)) {
        let Some((entity, name)) = self.game_screens.shown.get(picture).cloned() else {
            return;
        };
        let Some(playing) = &mut self.playing else {
            return;
        };
        let Some((running, _)) = &self.game_screens.running else {
            return;
        };
        let mut reader = GameReader(playing.game.as_mut(), &playing.world, playing.view.settings);
        match running.press(&name, Some(entity), action, &mut reader) {
            Ok(events) => {
                let mut dropped = 0;
                for event in events {
                    let sent = input::Sent {
                        to: event.to,
                        name: event.name,
                        values: event.values,
                    };
                    if sent.recordable() && self.game_screens.sent.len() < input::MAX_SENT {
                        self.game_screens.sent.push(sent);
                    } else {
                        dropped += 1;
                    }
                }
                if dropped > 0 {
                    self.log(
                        LogLevel::Warning,
                        format!(
                            "The screen {name} sent {dropped} event(s) the game couldn't take: at \
                             most {} wait for each tick, and each carries plain values (no \
                             assets, texts of at most {} letters).",
                            input::MAX_SENT,
                            input::MAX_SENT_TEXT
                        ),
                    );
                }
            }
            Err(problem) => {
                self.log(
                    LogLevel::Error,
                    format!("The screen {name} stopped: {problem}"),
                );
            }
        }
    }

    /// The player's settings (decision 0091), as game code last set them.
    pub fn player_settings(&self) -> script::settings::PlayerSettings {
        self.player_settings
    }

    /// Starts from these player settings (an exported game's saved ones).
    pub fn set_player_settings(&mut self, settings: script::settings::PlayerSettings) {
        self.player_settings = settings.sanitized();
        if let Some(playing) = &mut self.playing {
            playing.view.settings = self.player_settings;
        }
    }

    /// The game's screens drawn over the Game panel now, in order.
    pub fn game_screens(&self) -> &[Arc<ui::Screen>] {
        &self.game_screens.pictures
    }

    /// The names of the game's screens, for a Screen's choices.
    pub(crate) fn screen_names(&self) -> Vec<String> {
        self.program
            .as_ref()
            .and_then(|p| p.screens.as_ref())
            .map(|s| s.screens.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "game_screens_tests.rs"]
mod tests;
