//! New project (board 02), a dialog over the launcher: the name, which
//! makes the project's folder inside Create in (remembered); the kind,
//! 2D, 3D or both, as three picture cards; and what to start from, the
//! starters Solis has, each marked with the kinds it suits. Create makes
//! the folder, writes the project and opens it in the editor.
//!
//! It never overwrites anything: the app looks at the folder the name
//! would make (on a worker, as the name is typed) and a folder that
//! isn't empty is refused, here and again when Create is pressed.

use super::names::{self, FolderState};
use super::{ProjectKind, pictures};
use crate::file_pages::{plain, sentence};
use crate::icons::{self, Icon};
use crate::tokens::{self, launcher as l, size, text};
use crate::widgets::{self, button::Button, field};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Align2, CornerRadius, Layout, Rect, Sense, Ui, pos2, vec2};
use std::path::PathBuf;

/// What a new project starts from: an empty project, or one of Learn's
/// examples, made under the new project's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Starter {
    /// Only what a project needs to open.
    #[default]
    Empty,
    /// The playground: a character in a 3D level.
    Playground,
    /// The turret sample: game code in three languages.
    TurretSample,
    /// The UI sample: a HUD and a pause menu.
    UiSample,
    /// The benchmark: fleets of ships.
    Benchmark,
}

impl Starter {
    /// Every starter, in the order shown.
    pub const ALL: [Self; 5] = [
        Self::Empty,
        Self::Playground,
        Self::TurretSample,
        Self::UiSample,
        Self::Benchmark,
    ];

    /// Its name under its tile.
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::Playground => "Playground",
            Self::TurretSample => "Turret sample",
            Self::UiSample => "UI sample",
            Self::Benchmark => "Benchmark",
        }
    }

    /// The kind of project it was made for, marked in its tile's corner
    /// (none for the empty project, which suits every kind). All of
    /// Learn's examples are 3D.
    pub fn made_for(self) -> Option<ProjectKind> {
        match self {
            Self::Empty => None,
            _ => Some(ProjectKind::ThreeD),
        }
    }

    /// Whether it suits a project of `kind`: a 3D example suits 3D and
    /// both, never 2D.
    pub fn suits(self, kind: ProjectKind) -> bool {
        match self.made_for() {
            None => true,
            Some(made) => kind == made || kind == ProjectKind::Both,
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Empty => Icon::Plus,
            Self::Playground => Icon::Gamepad,
            Self::TurretSample => Icon::Code,
            Self::UiSample => Icon::Screen,
            Self::Benchmark => Icon::Profiler,
        }
    }
}

/// What Create asks the app to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProjectPlan {
    /// The project's folder, made new (or empty).
    pub folder: PathBuf,
    /// Its name.
    pub name: String,
    /// 2D, 3D or both.
    pub kind: ProjectKind,
    /// What it starts from.
    pub starter: Starter,
    /// The folder it was made in, to remember for next time.
    pub create_in: PathBuf,
}

/// New project while it's open: what's typed and chosen, and what the app
/// found at the folder the name would make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewProject {
    /// The name typed.
    pub(crate) name: String,
    /// The folder the project's folder goes in.
    pub(crate) create_in: PathBuf,
    /// The kind chosen.
    pub(crate) kind: ProjectKind,
    /// What it starts from.
    pub(crate) starter: Starter,
    /// The name field is given the keys once.
    focused: bool,
    /// The folder last asked about, so each is asked about once.
    asked_about: Option<PathBuf>,
    /// What the app found there.
    looked: Option<(PathBuf, FolderState)>,
    /// Why the app couldn't make it, after Create.
    pub(crate) failed: Option<String>,
}

impl NewProject {
    /// A new dialog making projects in `create_in`, of both kinds, empty.
    pub(crate) fn new(create_in: PathBuf) -> Self {
        Self {
            name: String::new(),
            create_in,
            kind: ProjectKind::Both,
            starter: Starter::Empty,
            focused: false,
            asked_about: None,
            looked: None,
            failed: None,
        }
    }

    /// The folder the project would be made in, once there's a name.
    pub(crate) fn folder(&self) -> Option<PathBuf> {
        (!self.name.trim().is_empty()).then(|| names::folder_for(&self.create_in, &self.name))
    }

    /// What the app found at `folder`.
    pub(crate) fn found(&mut self, folder: PathBuf, state: FolderState) {
        self.looked = Some((folder, state));
    }

    /// What the app last found at the folder the name makes now, if it
    /// has said.
    fn state_now(&self) -> Option<&FolderState> {
        let folder = self.folder()?;
        self.looked
            .as_ref()
            .filter(|(at, _)| *at == folder)
            .map(|(_, state)| state)
    }

    /// Why Create can't be pressed now, in plain words, if it can't (the
    /// name, Create in, the folder); `None` too while the app is still
    /// looking at the folder, which [`Self::ready`] waits for.
    pub(crate) fn problem(&self) -> Option<String> {
        if let Some(why) = names::name_problem(&self.name) {
            return Some(why);
        }
        if self.create_in.as_os_str().is_empty() {
            return Some("Choose where to make it with Browse….".into());
        }
        let folder = self.folder()?;
        names::path_problem(&folder).or_else(|| self.state_now().and_then(FolderState::problem))
    }

    /// Whether Create can be pressed: nothing wrong, and the app has said
    /// the folder is free or empty.
    pub(crate) fn ready(&self) -> bool {
        self.problem().is_none()
            && self.starter.suits(self.kind)
            && matches!(
                self.state_now(),
                Some(FolderState::Free | FolderState::Empty)
            )
    }

    /// What Create asks for, if it can be pressed.
    pub(crate) fn plan(&self) -> Option<NewProjectPlan> {
        self.ready().then(|| NewProjectPlan {
            folder: names::folder_for(&self.create_in, &self.name),
            name: self.name.trim().to_string(),
            kind: self.kind,
            starter: self.starter,
            create_in: self.create_in.clone(),
        })
    }

    /// Chooses `kind`; a starter that doesn't suit it goes back to Empty.
    pub(crate) fn choose_kind(&mut self, kind: ProjectKind) {
        self.kind = kind;
        if !self.starter.suits(kind) {
            self.starter = Starter::Empty;
        }
    }

    /// The question about the folder the name makes now, if it hasn't
    /// been asked.
    fn question(&mut self) -> Option<WorkbenchAction> {
        let folder = self.folder()?;
        if names::name_problem(&self.name).is_some() || self.asked_about.as_ref() == Some(&folder) {
            return None;
        }
        self.asked_about = Some(folder.clone());
        Some(WorkbenchAction::LookAtFolder(folder))
    }
}

impl Workbench {
    /// Opens New project over the launcher, making projects in the folder
    /// last used.
    pub(crate) fn start_new_project(&mut self) {
        let create_in = self.launcher.create_in.clone().unwrap_or_default();
        self.launcher.new_project = Some(NewProject::new(create_in));
    }

    /// New project, while it's open.
    pub(super) fn new_project_dialog(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let Some(dialog) = &mut self.launcher.new_project else {
            return;
        };
        let frame = egui::Frame::new()
            .fill(tokens::panel())
            .stroke(egui::Stroke::new(1.0, tokens::pill_border()))
            .corner_radius(size::radius::CARD)
            .inner_margin(egui::Margin::same(l::DIALOG_PAD as i8));
        let mut done = None;
        egui::Modal::new(egui::Id::new("new project"))
            .frame(frame)
            .show(ui.ctx(), |ui| {
                ui.set_width(l::DIALOG_W - 2.0 * l::DIALOG_PAD);
                ui.spacing_mut().item_spacing.y = size::GAP;
                done = dialog_body(ui, dialog, asked);
            });
        if let Some(question) = dialog.question() {
            asked.push(question);
        }
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        match done {
            Some(true) => {
                if let Some(plan) = dialog.plan() {
                    dialog.failed = None;
                    asked.push(WorkbenchAction::CreateProject(plan));
                }
            }
            Some(false) => self.launcher.new_project = None,
            None if escape => self.launcher.new_project = None,
            None => {}
        }
    }
}

/// Everything inside the dialog. Returns Create (`true`) or Cancel.
fn dialog_body(
    ui: &mut Ui,
    dialog: &mut NewProject,
    asked: &mut Vec<WorkbenchAction>,
) -> Option<bool> {
    let mut done = None;
    ui.horizontal(|ui| {
        plain(
            ui,
            "New project",
            text::dialog_title(),
            tokens::text_emphasis(),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::button::icon(ui, Icon::Close, "Close New project").clicked() {
                done = Some(false);
            }
        });
    });
    sentence(ui, "Give it a name: its folder is made for you.");
    ui.add_space(l::DIALOG_SECTION_GAP - size::GAP);
    label(ui, "Name");
    let width = ui.available_width();
    let name = field::text_field(ui, "Name", &mut dialog.name, "My game", width);
    // Asked until the field has the keys: the click that opened the
    // dialog can take them away again as it ends.
    if !dialog.focused {
        name.request_focus();
        dialog.focused = name.has_focus();
    }
    let enter = name.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    folder_line(ui, dialog);
    ui.add_space(l::DIALOG_SECTION_GAP - size::GAP);
    label(ui, "Create in");
    ui.horizontal(|ui| {
        let browse_w = widgets::width_of(ui, "Browse…", text::ui()) + 4.0 * size::PAGE_BUTTON_PAD;
        let shown = dialog.create_in.display().to_string();
        let room = ui.available_width() - browse_w - size::GAP;
        let (rect, _) = ui.allocate_exact_size(vec2(room, size::field_h()), Sense::hover());
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(size::radius::CONTROL),
            tokens::field(),
        );
        widgets::border(ui, rect, size::radius::CONTROL, tokens::field_border());
        widgets::cut_off(
            ui,
            rect.shrink2(vec2(size::FIELD_PAD, 0.0)),
            &shown,
            text::mono_small(),
            tokens::text_primary(),
        );
        let browse = Button::plain("Browse…")
            .icon(Icon::FolderOpen)
            .show(ui)
            .on_hover_text("Choose the folder new projects are made in");
        if browse.clicked() {
            asked.push(WorkbenchAction::ChooseCreateIn);
        }
    });
    plain(
        ui,
        "Remembered for your next project.",
        text::row(),
        tokens::text_meta(),
    );
    ui.add_space(l::DIALOG_SECTION_GAP - size::GAP);
    label(ui, "Kind");
    kind_cards(ui, dialog);
    plain(
        ui,
        "Kept with the project: you can change it later in Project settings.",
        text::row(),
        tokens::text_meta(),
    );
    ui.add_space(l::DIALOG_SECTION_GAP - size::GAP);
    label(ui, "Start from");
    starters(ui, dialog);
    ui.add_space(l::DIALOG_SECTION_GAP);
    let across = ui.available_rect_before_wrap().x_range();
    tokens::hline(ui.painter(), across, ui.cursor().top(), tokens::line());
    ui.add_space(l::DIALOG_SECTION_GAP);
    ui.horizontal(|ui| {
        match &dialog.failed {
            Some(why) => plain(ui, why, text::row(), tokens::WARNING),
            None => plain(
                ui,
                "It opens in the editor as soon as it's made.",
                text::row(),
                tokens::text_meta(),
            ),
        };
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let ready = dialog.ready();
            let create = ui
                .add_enabled_ui(ready, |ui| Button::primary("Create").show(ui))
                .inner;
            if (create.clicked() || enter) && ready {
                done = Some(true);
            }
            if Button::plain("Cancel").show(ui).clicked() {
                done = Some(false);
            }
        });
    });
    done
}

/// A section's name.
fn label(ui: &mut Ui, words: &str) {
    plain(ui, words, text::setting_name(), tokens::text_primary());
}

/// The line under Name: the folder it makes, or why it can't.
fn folder_line(ui: &mut Ui, dialog: &NewProject) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::GAP;
        if let Some(why) = dialog.problem().filter(|_| !dialog.name.is_empty()) {
            icon(ui, Icon::Warning, tokens::WARNING);
            plain(ui, &why, text::row(), tokens::WARNING);
            return;
        }
        icon(ui, Icon::FolderOpen, tokens::KIND_FOLDER);
        plain(ui, "Makes the folder", text::row(), tokens::text_meta());
        let folder = dialog.folder().map_or_else(
            || {
                format!(
                    "{}{}…",
                    dialog.create_in.display(),
                    std::path::MAIN_SEPARATOR
                )
            },
            |f| format!("{}{}", f.display(), std::path::MAIN_SEPARATOR),
        );
        let room = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            vec2(room, widgets::line_height(ui, &text::mono_small())),
            Sense::hover(),
        );
        widgets::cut_off(
            ui,
            rect,
            &folder,
            text::mono_small(),
            tokens::text_primary(),
        );
    });
}

/// A small icon in the line of words.
fn icon(ui: &mut Ui, which: Icon, ink: egui::Color32) {
    let (spot, _) =
        ui.allocate_exact_size(vec2(size::ICON_SMALL, size::ICON_SMALL), Sense::hover());
    icons::icon(ui.painter(), spot, which, ink);
}

/// The three kinds as picture cards, the chosen one lit with a tick.
fn kind_cards(ui: &mut Ui, dialog: &mut NewProject) {
    let gap = size::CARD_GAP;
    let width = (ui.available_width() - 2.0 * gap) / 3.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for kind in ProjectKind::ALL {
            let chosen = dialog.kind == kind;
            let (rect, response) = ui.allocate_exact_size(vec2(width, l::KIND_H), Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, chosen, kind.name())
            });
            if response.clicked() {
                dialog.choose_kind(kind);
            }
            if ui.is_rect_visible(rect) {
                draw_kind_card(ui, rect, kind, chosen, response.hovered());
                widgets::focus_ring(ui, &response, rect, size::radius::CARD);
            }
        }
    });
}

fn draw_kind_card(ui: &Ui, rect: Rect, kind: ProjectKind, chosen: bool, hovered: bool) {
    let accent = crate::interface_look::in_use().accent;
    let fill = if chosen {
        tokens::select_item(accent)
    } else if hovered {
        tokens::card()
    } else {
        tokens::panel()
    };
    let radius = size::radius::CARD;
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius), fill);
    let edge = if chosen {
        crate::interface_look::accent_fill()
    } else {
        tokens::card_border()
    };
    widgets::border(ui, rect, radius, edge);
    let inner = rect.shrink(l::KIND_PAD);
    let picture = Rect::from_min_size(inner.min, vec2(inner.width(), l::KIND_PICTURE_H));
    pictures::kind_picture(
        ui.painter(),
        picture,
        kind,
        CornerRadius::same(size::radius::SMALL),
    );
    let name_at = Rect::from_min_size(
        pos2(inner.left(), picture.bottom() + size::GAP),
        vec2(inner.width(), widgets::line_height(ui, &text::card_name())),
    );
    widgets::words(
        ui,
        name_at,
        Align2::LEFT_CENTER,
        kind.name(),
        text::card_name(),
        tokens::text_emphasis(),
    );
    if chosen {
        let tick = Rect::from_center_size(
            pos2(name_at.right() - l::TICK / 2.0, name_at.center().y),
            vec2(l::TICK, l::TICK),
        );
        ui.painter().circle_filled(
            tick.center(),
            l::TICK / 2.0,
            crate::interface_look::accent_fill(),
        );
        icons::icon(
            ui.painter(),
            tick.shrink(3.0),
            Icon::Check,
            tokens::ON_ACCENT,
        );
    }
    let about = Rect::from_min_max(pos2(inner.left(), name_at.bottom() + size::GAP), inner.max);
    let galley = ui.painter().layout(
        kind.about().to_string(),
        text::row(),
        tokens::text_meta(),
        about.width(),
    );
    ui.painter().galley(about.min, galley, tokens::text_meta());
}

/// The starters' tiles, a starter that doesn't suit the kind greyed.
fn starters(ui: &mut Ui, dialog: &mut NewProject) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(l::STARTER_GAP, l::STARTER_GAP);
        for starter in Starter::ALL {
            let fits = starter.suits(dialog.kind);
            let chosen = dialog.starter == starter;
            let height =
                l::STARTER_H + size::GAP + widgets::line_height(ui, &text::template_name());
            let sense = if fits { Sense::click() } else { Sense::hover() };
            let (rect, response) = ui.allocate_exact_size(vec2(l::STARTER_W, height), sense);
            let said = format!("Start from {}", starter.name());
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::RadioButton, fits, chosen, &said)
            });
            if fits && response.clicked() {
                dialog.starter = starter;
            }
            if ui.is_rect_visible(rect) {
                draw_starter(ui, rect, starter, chosen, fits, response.hovered());
            }
            let hover = match (fits, starter.made_for()) {
                (false, Some(kind)) => format!(
                    "Made for {} projects: choose {} or Both to start from it",
                    kind.name(),
                    kind.name()
                ),
                (_, None) => "Only what a project needs to open".to_string(),
                (true, Some(_)) => {
                    "Made as your project, under its name, to take apart".to_string()
                }
            };
            response.on_hover_text(hover);
        }
    });
}

fn draw_starter(ui: &Ui, rect: Rect, starter: Starter, chosen: bool, fits: bool, hovered: bool) {
    let tile = Rect::from_min_size(rect.min, vec2(l::STARTER_W, l::STARTER_H));
    let radius = size::radius::CARD;
    let fill = if chosen {
        tokens::select_item(crate::interface_look::in_use().accent)
    } else if hovered && fits {
        tokens::hover()
    } else {
        tokens::viewport()
    };
    ui.painter()
        .rect_filled(tile, CornerRadius::same(radius), fill);
    let edge = if chosen {
        crate::interface_look::accent_fill()
    } else {
        tokens::card_border()
    };
    widgets::border(ui, tile, radius, edge);
    let ink = if fits {
        tokens::text_label()
    } else {
        tokens::text_faint()
    };
    let spot = Rect::from_center_size(
        tile.center(),
        vec2(size::RIBBON_BIG_ICON, size::RIBBON_BIG_ICON),
    );
    icons::icon(ui.painter(), spot, starter.icon(), ink);
    if let Some(kind) = starter.made_for() {
        let font = text::mono_small();
        let width = widgets::width_of(ui, kind.badge(), font.clone()) + 2.0 * size::GAP;
        let mark = Rect::from_min_size(
            pos2(tile.right() - width - size::GAP, tile.top() + size::GAP),
            vec2(width, l::BADGE_H - 4.0),
        );
        ui.painter()
            .rect_filled(mark, CornerRadius::same(size::radius::SMALL), tokens::app());
        widgets::words(ui, mark, Align2::CENTER_CENTER, kind.badge(), font, ink);
    }
    let words = Rect::from_min_max(pos2(rect.left(), tile.bottom() + size::GAP), rect.max);
    let name_ink = if fits {
        tokens::text_primary()
    } else {
        tokens::text_faint()
    };
    widgets::words(
        ui,
        words,
        Align2::LEFT_TOP,
        starter.name(),
        text::template_name(),
        name_ink,
    );
}
