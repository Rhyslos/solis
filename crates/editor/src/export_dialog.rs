//! File > Export for playtesting… ([decision 0093](../../../docs/decisions/0093-the-playtest-export.md)):
//! the window asking for the game's name, the area it starts in, the
//! folder and the window, and [`Playtest`], what an exported game reads to
//! know them (and what the editor remembers for next time).

use crate::workbench::{Workbench, WorkbenchAction};
use egui::Ui;
use std::path::{Path, PathBuf};

/// The first line of a playtest file.
const HEADER: &str = "playtest 1";
/// The longest game name kept.
pub const MAX_NAME: usize = 64;
/// The biggest playtest file read.
pub const MAX_PLAYTEST_BYTES: u64 = 16 << 10;

/// An exported game: its name, the area it starts in, and its window as
/// it first opens (players change it in the game's own menu, or with
/// Alt+Enter, and their choice is kept).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playtest {
    /// The game's name: the program's, its window's title and its saves
    /// folder's.
    pub name: String,
    /// The area it starts in, inside the assets, with `/` between folders.
    pub area: String,
    /// Whether it starts filling the screen.
    pub fullscreen: bool,
    /// Its window's size in pixels, when it isn't fullscreen.
    pub size: [u32; 2],
    /// Where the editor last exported it (remembered, not exported).
    pub folder: Option<PathBuf>,
}

impl Default for Playtest {
    fn default() -> Self {
        Self {
            name: "Game".into(),
            area: String::new(),
            fullscreen: false,
            size: [1280, 720],
            folder: None,
        }
    }
}

/// `name` made usable as a game's name and a file's: no characters
/// Windows refuses in file names, no spaces or dots at the ends, and at
/// most [`MAX_NAME`] characters; "Game" if nothing is left.
pub fn game_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !c.is_control() && !r#"<>:"/\|?*"#.contains(*c))
        .take(MAX_NAME)
        .collect();
    let kept = kept.trim_matches(|c: char| c.is_whitespace() || c == '.');
    // Names Windows keeps for devices.
    let reserved = ["CON", "PRN", "AUX", "NUL"];
    let upper = kept.to_ascii_uppercase();
    let device = reserved.contains(&upper.as_str())
        || ((upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.len() == 4
            && upper.as_bytes()[3].is_ascii_digit());
    if kept.is_empty() || device {
        "Game".into()
    } else {
        kept.to_owned()
    }
}

/// Whether `area` is an area's path inside the assets: relative, with `/`
/// between folders, never going up, ending `.solarea`.
pub fn usable_area(area: &str) -> bool {
    !area.is_empty()
        && area.len() < 1024
        && area.ends_with(".solarea")
        && !area.starts_with('/')
        && !area.contains('\\')
        && !area.contains(':')
        && area
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl Playtest {
    /// As written in a playtest file.
    pub fn text(&self) -> String {
        let mut text = format!(
            "{HEADER}\nname {}\narea {}\nwindow {} {} {}\n",
            game_name(&self.name),
            self.area,
            if self.fullscreen {
                "fullscreen"
            } else {
                "windowed"
            },
            self.size[0],
            self.size[1],
        );
        if let Some(folder) = &self.folder
            && let Some(folder) = folder.to_str()
            && !folder.contains('\n')
        {
            text.push_str(&format!("folder {folder}\n"));
        }
        text
    }

    /// Reads a playtest file. Anything odd is put right (a name made
    /// usable, a window size kept within what Solis opens); only a file
    /// that isn't one, or names no usable area, is refused.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some(HEADER) {
            return Err("it isn't a playtest file".into());
        }
        let mut found = Playtest::default();
        for line in lines {
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            match key {
                "name" => found.name = game_name(value),
                "area" => found.area = value.trim().to_owned(),
                "window" => {
                    let mut words = value.split_whitespace();
                    found.fullscreen = words.next() == Some("fullscreen");
                    let mut side = || words.next().and_then(|w| w.parse::<u32>().ok());
                    if let (Some(w), Some(h)) = (side(), side()) {
                        found.size = script::settings::PlayerSettings::default()
                            .with(script::settings::Setting::Resolution(w, h))
                            .resolution;
                    }
                }
                "folder" if !value.trim().is_empty() => found.folder = Some(value.into()),
                _ => {}
            }
        }
        if !usable_area(&found.area) {
            return Err("it doesn't name an area to start in".into());
        }
        Ok(found)
    }
}

/// The folder an export goes in by default: `<Game> exports/<Game>
/// playtest <date>` in `documents`.
pub fn default_folder(documents: &Path, name: &str, date: &str) -> PathBuf {
    let name = game_name(name);
    documents
        .join(format!("{name} exports"))
        .join(format!("{name} playtest {date}"))
}

/// What the export window asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRequest {
    /// The game as it's exported.
    pub playtest: Playtest,
    /// The folder it goes in (made; it must be empty if it's there).
    pub folder: PathBuf,
}

/// File > Export for playtesting…, while it's open.
#[derive(Debug, Clone, Default)]
pub(crate) struct ExportWindow {
    playtest: Playtest,
    folder: String,
    /// What happened to the last export: where it went, or why it didn't.
    done: Option<Result<PathBuf, String>>,
    /// Exporting now.
    busy: bool,
}

/// The window sizes offered, as the game's Resolution steps through them.
fn sizes() -> &'static [[u32; 2]] {
    &script::settings::RESOLUTIONS
}

impl Workbench {
    /// Remembers how the project was last exported (the app reads it with
    /// the project), for the window to start from.
    pub fn set_last_export(&mut self, last: Option<Playtest>) {
        self.last_export = last;
    }

    /// The Documents folder, for exports to go in by default.
    pub fn set_documents_folder(&mut self, folder: Option<PathBuf>) {
        self.documents = folder;
    }

    /// Opens File > Export for playtesting…: the game named after the
    /// project, starting in the open area, unless it was exported before.
    pub fn open_export(&mut self) {
        let Some(project) = &self.project else {
            self.log(
                crate::LogLevel::Warning,
                "Export for playtesting needs a project: open or make one first.",
            );
            return;
        };
        let assets = project.assets.clone();
        let open_area = self
            .document
            .file()
            .and_then(|f| f.strip_prefix(&assets).ok())
            .and_then(|f| f.to_str())
            .map(|f| f.replace('\\', "/"))
            .filter(|f| usable_area(f));
        let mut playtest = self.last_export.clone().unwrap_or_else(|| Playtest {
            name: game_name(&project.name),
            ..Playtest::default()
        });
        if !usable_area(&playtest.area) || !self.areas().contains(&playtest.area) {
            playtest.area = open_area
                .or_else(|| self.areas().into_iter().next())
                .unwrap_or_default();
        }
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let folder = match (&playtest.folder, &self.documents) {
            // A new dated folder beside the last, so old builds stay.
            (Some(last), _) if last.parent().is_some() => last
                .parent()
                .unwrap_or(last)
                .join(format!("{} playtest {date}", game_name(&playtest.name))),
            (_, Some(documents)) => default_folder(documents, &playtest.name, &date),
            _ => project
                .root
                .parent()
                .map(|p| default_folder(p, &playtest.name, &date))
                .unwrap_or_default(),
        };
        self.export_window = Some(ExportWindow {
            playtest,
            folder: folder.display().to_string(),
            done: None,
            busy: false,
        });
    }

    /// The project's areas, inside the assets.
    fn areas(&self) -> Vec<String> {
        let mut areas: Vec<String> = self
            .library
            .iter()
            .filter(|i| i.kind == crate::library::LibraryKind::Area && usable_area(&i.path))
            .map(|i| i.path.clone())
            .collect();
        areas.sort();
        areas
    }

    /// The folder chosen with the window's "…" button.
    pub fn set_export_folder(&mut self, folder: PathBuf) {
        if let Some(window) = &mut self.export_window {
            window.folder = folder.display().to_string();
        }
    }

    /// The app finished exporting: where it went, or why it didn't.
    pub fn export_finished(&mut self, outcome: Result<PathBuf, String>) {
        match &outcome {
            Ok(folder) => self.log(
                crate::LogLevel::Info,
                format!(
                    "Exported for playtesting to {}. Zip that folder and send it: the game runs \
                     from the program inside it.",
                    folder.display()
                ),
            ),
            Err(why) => self.log(
                crate::LogLevel::Error,
                format!("The export didn't finish: {why}."),
            ),
        }
        if let Some(window) = &mut self.export_window {
            window.busy = false;
            window.done = Some(outcome);
        }
    }

    /// Draws File > Export for playtesting…, if it's open.
    pub(crate) fn export_window(&mut self, ctx: &egui::Context, actions: &mut Vec<WorkbenchAction>) {
        let Some(mut window) = self.export_window.take() else {
            return;
        };
        let areas = self.areas();
        let program = self.info.name.clone();
        let mut open = true;
        let mut close = false;
        egui::Window::new("Export for playtesting")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_width(560.0)
            .show(ctx, |ui| {
                close = contents(ui, &mut window, &areas, &program, actions);
            });
        if open && !close {
            self.export_window = Some(window);
        }
    }
}

/// The window's insides; true when it should close.
fn contents(
    ui: &mut Ui,
    window: &mut ExportWindow,
    areas: &[String],
    program: &str,
    actions: &mut Vec<WorkbenchAction>,
) -> bool {
    let name = game_name(&window.playtest.name);
    ui.label(format!(
        "A folder to zip and send. Your friend runs {name}.exe inside it: no {program} needed."
    ));
    ui.add_space(8.0);
    let mut close = false;
    egui::Grid::new("export grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Game's name");
            ui.add(egui::TextEdit::singleline(&mut window.playtest.name).desired_width(280.0))
                .on_hover_text("The program's name, the window's title and the saves folder's");
            ui.end_row();
            ui.label("Starts in");
            egui::ComboBox::from_id_salt("export area")
                .selected_text(if window.playtest.area.is_empty() {
                    "No area: save one first"
                } else {
                    window.playtest.area.trim_end_matches(".solarea")
                })
                .width(280.0)
                .show_ui(ui, |ui| {
                    for area in areas {
                        ui.selectable_value(
                            &mut window.playtest.area,
                            area.clone(),
                            area.trim_end_matches(".solarea"),
                        );
                    }
                });
            ui.end_row();
            ui.label("Into the folder");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut window.folder).desired_width(280.0))
                    .on_hover_text("Made for the export; a folder that has files in already isn't used");
                if ui
                    .button("…")
                    .on_hover_text("Choose the folder")
                    .clicked()
                {
                    actions.push(WorkbenchAction::ChooseExportFolder);
                }
            });
            ui.end_row();
            ui.label("Window");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("export window")
                    .selected_text(if window.playtest.fullscreen {
                        "Fullscreen"
                    } else {
                        "Windowed"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut window.playtest.fullscreen, false, "Windowed");
                        ui.selectable_value(&mut window.playtest.fullscreen, true, "Fullscreen");
                    });
                let [w, h] = window.playtest.size;
                egui::ComboBox::from_id_salt("export size")
                    .selected_text(format!("{w} × {h}"))
                    .show_ui(ui, |ui| {
                        for size in sizes() {
                            ui.selectable_value(
                                &mut window.playtest.size,
                                *size,
                                format!("{} × {}", size[0], size[1]),
                            );
                        }
                    });
            });
            ui.end_row();
            ui.label("");
            ui.label(
                egui::RichText::new(
                    "How it starts. Players change these in your game's menu, or Alt+Enter for \
                     fullscreen, and their choice is kept.",
                )
                .weak(),
            );
            ui.end_row();
        });
    ui.add_space(8.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(format!(
            "Packs: the program, your code (Lust and Lua), areas, sprites, sounds, screens. Saves, \
             the player's settings and a crash log go in the player's Saved Games/{name} folder."
        ));
    });
    match &window.done {
        Some(Ok(folder)) => {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(format!("Exported to {}.", folder.display()));
                if ui.button("Open folder").clicked() {
                    actions.push(WorkbenchAction::OpenExternally(folder.clone()));
                }
            });
        }
        Some(Err(why)) => {
            ui.add_space(6.0);
            ui.colored_label(crate::theme::ERROR, format!("Not exported: {why}."));
        }
        None => {}
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ready = usable_area(&window.playtest.area)
                && !window.folder.trim().is_empty()
                && !window.busy;
            let export = ui.add_enabled(
                ready,
                egui::Button::new(if window.busy { "Exporting…" } else { "Export" }),
            );
            if export.clicked() {
                window.playtest.name = game_name(&window.playtest.name);
                let folder = PathBuf::from(window.folder.trim());
                window.playtest.folder = Some(folder.clone());
                window.busy = true;
                window.done = None;
                actions.push(WorkbenchAction::ExportPlaytest(ExportRequest {
                    playtest: window.playtest.clone(),
                    folder,
                }));
            }
            let done = window.done.as_ref().is_some_and(Result::is_ok);
            if ui.button(if done { "Close" } else { "Cancel" }).clicked() {
                close = true;
            }
        });
    });
    close
}

#[cfg(test)]
#[path = "export_dialog_tests.rs"]
mod tests;
