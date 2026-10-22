//! The viewport panel: the 3D image, and the controls drawn over it in its
//! corners ([decision 0004](../../../docs/decisions/0004-editor-layout.md)):
//!
//! - top right: frame rate and entity count;
//! - bottom left: ≡ (what to show), Perspective, Lit and Show;
//! - bottom centre: the tools;
//! - bottom right: grid, angle and camera speed.
//!
//! The image itself comes from a [`ViewportRenderer`], so this crate never
//! touches the GPU and runs in tests without one.

use crate::camera::{Camera, FlyInput};
use crate::environment::EnvironmentSettings;
use crate::icons;
use crate::lighting::{Lamp, SunSettings};
use crate::look::{Look, LookKind};
use crate::models::ModelId;
use crate::theme::{self, Accent};
use egui::{Align, Color32, Layout, Rect, RichText, Sense, Ui, UiBuilder, Vec2, vec2};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

/// The tools ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
/// There is no Place tool: placing things from the library is part of
/// Select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Tool {
    /// Pick things in the viewport.
    #[default]
    Select,
    /// Move along an axis or freely.
    Move,
    /// Turn around an axis.
    Rotate,
    /// Make bigger or smaller.
    Scale,
    /// Move, rotate and scale at once.
    Transform,
}

impl Tool {
    /// Every tool, in the order the viewport shows them.
    pub const ALL: [Self; 5] = [
        Self::Select,
        Self::Move,
        Self::Rotate,
        Self::Scale,
        Self::Transform,
    ];

    /// The tool's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::Transform => "Transform",
        }
    }

    /// The key that picks the tool.
    pub fn key(self) -> egui::Key {
        match self {
            Self::Select => egui::Key::Q,
            Self::Move => egui::Key::W,
            Self::Rotate => egui::Key::R,
            Self::Scale => egui::Key::S,
            Self::Transform => egui::Key::T,
        }
    }

    /// What the tool's button says when pointed at, such as "Move (W)".
    pub fn hint(self) -> String {
        format!("{} ({})", self.name(), self.key().name())
    }
}

/// Which of the viewport's corner controls are shown (the ≡ menu). Saved
/// between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Overlays {
    /// The tools, bottom centre.
    pub tools: bool,
    /// Grid, angle and camera speed, bottom right.
    pub snapping: bool,
    /// Frame rate and entity count, top right.
    pub stats: bool,
}

impl Default for Overlays {
    fn default() -> Self {
        Self {
            tools: true,
            snapping: true,
            stats: true,
        }
    }
}

/// What one viewport frame needs from the renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportView {
    /// The image size in physical pixels.
    pub size_px: [u32; 2],
    /// World-to-screen transform.
    pub view_projection: Mat4,
    /// Where the camera is.
    pub camera_position: Vec3,
    /// The point on the ground the grid is centred on.
    pub focus: Vec3,
    /// The sky straight up, sRGB 0 to 1.
    pub sky_top: [f32; 3],
    /// The sky at the horizon, sRGB 0 to 1.
    pub sky_horizon: [f32; 3],
    /// The ground below the horizon, sRGB 0 to 1.
    pub ground: [f32; 3],
    /// The imported models to draw.
    pub models: Vec<ModelId>,
    /// How models are shaded.
    pub look: Look,
    /// The stylized look's outline width in pixels; 0 for none.
    pub outline_px: f32,
    /// The sun.
    pub sun: SunSettings,
    /// The lamps.
    pub lamps: Vec<Lamp>,
    /// The sky, fog and exposure.
    pub environment: EnvironmentSettings,
}

/// Draws viewport images. The app implements it on the window's GPU; tests
/// use one that draws nothing.
pub trait ViewportRenderer {
    /// Draws a frame and returns the egui texture showing it, or a
    /// plain-language reason why it could not.
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String>;
}

/// A renderer that never draws, with a reason to show instead.
#[derive(Debug, Clone)]
pub struct NoRenderer {
    /// Why there is no 3D view.
    pub reason: String,
}

impl ViewportRenderer for NoRenderer {
    fn render(&mut self, _view: &ViewportView) -> Result<egui::TextureId, String> {
        Err(self.reason.clone())
    }
}

/// sRGB 0 to 1 from a colour.
fn rgb(c: Color32) -> [f32; 3] {
    [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.0)
}

/// The viewport's state between frames.
#[derive(Debug, Clone)]
pub struct Viewport {
    /// The camera.
    pub camera: Camera,
    /// The chosen tool.
    pub tool: Tool,
    /// Which corner controls are shown.
    pub overlays: Overlays,
    /// Whether the ≡ menu is open.
    menu_open: bool,
    /// The time per frame, smoothed, in seconds.
    frame_time: f32,
    /// The imported models drawn in the view.
    pub models: Vec<ModelId>,
    /// How models are shaded.
    pub look: Look,
    /// How many triangles they have together, for the corner.
    pub triangles: usize,
    /// The sun's settings.
    pub sun: SunSettings,
    /// The lamps.
    pub lamps: Vec<Lamp>,
    /// The sky, fog and exposure.
    pub environment: EnvironmentSettings,
    /// While looking around with the right mouse button: where the mouse
    /// was when it started, to put it back afterwards.
    flying_from: Option<egui::Pos2>,
    /// Wheel movement not yet a whole notch (trackpads scroll in small
    /// amounts), in notches.
    wheel_carry: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            tool: Tool::Select,
            overlays: Overlays::default(),
            menu_open: false,
            frame_time: 1.0 / 60.0,
            flying_from: None,
            wheel_carry: 0.0,
            models: Vec::new(),
            look: Look::default(),
            triangles: 0,
            sun: SunSettings::default(),
            lamps: Vec::new(),
            environment: EnvironmentSettings::default(),
        }
    }
}

/// How quickly the frame-rate counter follows changes (0 to 1 per frame).
const FRAME_TIME_SMOOTHING: f32 = 0.05;

impl Viewport {
    /// Whether the ≡ menu is open.
    pub fn menu_open(&self) -> bool {
        self.menu_open
    }

    /// Whether the right mouse button is held on the viewport, so W A S D,
    /// Q and E move the camera instead of picking tools.
    pub fn is_flying(&self) -> bool {
        self.flying_from.is_some()
    }

    /// The frame rate shown in the corner.
    pub fn frames_per_second(&self) -> f32 {
        1.0 / self.frame_time.max(1e-4)
    }

    /// The view for an image of `size_px` physical pixels.
    pub fn view(&self, size_px: [u32; 2], pixels_per_point: f32) -> ViewportView {
        let aspect = size_px[0].max(1) as f32 / size_px[1].max(1) as f32;
        ViewportView {
            size_px,
            view_projection: self.camera.view_projection(aspect),
            camera_position: self.camera.position,
            focus: self.camera.focus(),
            sky_top: rgb(theme::backdrop::SKY_TOP),
            sky_horizon: rgb(theme::backdrop::HORIZON),
            ground: rgb(theme::backdrop::GROUND),
            models: self.models.clone(),
            look: self.look,
            outline_px: self.look.outline_px(pixels_per_point),
            sun: self.sun,
            lamps: self.lamps.clone(),
            environment: self.environment,
        }
    }

    /// Draws the viewport in `ui`, filling it.
    pub fn show(&mut self, ui: &mut Ui, renderer: &mut dyn ViewportRenderer, accent: Accent) {
        let dt = ui.input(|i| i.unstable_dt);
        if dt.is_finite() && dt > 0.0 {
            self.frame_time += (dt - self.frame_time) * FRAME_TIME_SMOOTHING;
        }
        let rect = ui.max_rect();
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        if response.clicked() {
            self.menu_open = false;
        }
        self.navigate(ui, &response, dt);
        let ppp = ui.ctx().pixels_per_point();
        let size_px = [
            (rect.width() * ppp).round().max(1.0) as u32,
            (rect.height() * ppp).round().max(1.0) as u32,
        ];
        let painter = ui.painter_at(rect);
        match renderer.render(&self.view(size_px, ppp)) {
            Ok(texture) => {
                painter.image(
                    texture,
                    rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Err(reason) => {
                let message = format!("No 3D view: {reason}");
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &message)
                });
                painter.rect_filled(rect, 0.0, theme::backdrop::HORIZON);
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    message,
                    egui::FontId::proportional(13.0),
                    theme::TEXT_WEAK,
                );
            }
        }
        let inner = rect.shrink(8.0);
        let corner = |ui: &mut Ui, salt: &str, layout: Layout| {
            ui.new_child(
                UiBuilder::new()
                    .id_salt(("viewport corner", salt))
                    .max_rect(inner)
                    .layout(layout),
            )
        };
        if self.overlays.stats {
            let mut top_right = corner(ui, "stats", Layout::top_down(Align::Max));
            self.stats(&mut top_right);
        }
        let mut bottom_left = corner(ui, "view", Layout::bottom_up(Align::Min));
        self.view_controls(&mut bottom_left);
        if self.overlays.tools {
            // A centred layout would stretch the bar, so it gets a rect of
            // exactly its own width, centred along the bottom.
            let bar = Rect::from_center_size(
                egui::pos2(inner.center().x, inner.bottom() - TOOL_BAR_SIZE.y / 2.0),
                TOOL_BAR_SIZE,
            );
            let mut bottom_centre = ui.new_child(
                UiBuilder::new()
                    .id_salt(("viewport corner", "tools"))
                    .max_rect(bar)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            self.tools(&mut bottom_centre, accent);
        }
        if self.overlays.snapping {
            let mut bottom_right = corner(ui, "snapping", Layout::bottom_up(Align::Max));
            snapping(&mut bottom_right, accent, self.camera.speed);
        }
    }

    /// Flies the camera from this frame's mouse and keys
    /// ([decision 0009](../../../docs/decisions/0009-fly-camera-controls.md)).
    fn navigate(&mut self, ui: &Ui, response: &egui::Response, dt: f32) {
        use egui::{Key, PointerButton, ViewportCommand};
        let ctx = ui.ctx();
        let (pressed_here, held, pointer) = ui.input(|i| {
            (
                i.pointer.button_pressed(PointerButton::Secondary),
                i.pointer.button_down(PointerButton::Secondary),
                i.pointer.interact_pos(),
            )
        });
        let starting = self.flying_from.is_none()
            && pressed_here
            && response.hovered()
            && response.is_pointer_button_down_on();
        if starting {
            self.flying_from = pointer.or(Some(response.rect.center()));
            self.menu_open = false;
            // Keys now steer the camera, not a text field.
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            ctx.send_viewport_cmd(ViewportCommand::CursorVisible(false));
            ctx.send_viewport_cmd(ViewportCommand::CursorGrab(grab_mode()));
        }
        if let Some(from) = self.flying_from
            && !held
        {
            self.flying_from = None;
            ctx.send_viewport_cmd(ViewportCommand::CursorGrab(egui::CursorGrab::None));
            ctx.send_viewport_cmd(ViewportCommand::CursorVisible(true));
            ctx.send_viewport_cmd(ViewportCommand::CursorPosition(from));
        }

        let flying = self.flying_from.is_some();
        let notches = if flying || response.hovered() {
            let (whole, carry) = wheel_notches(ui, self.wheel_carry);
            self.wheel_carry = carry;
            whole
        } else {
            self.wheel_carry = 0.0;
            0.0
        };
        let mut input = FlyInput {
            dt,
            view_height: response.rect.height(),
            ..FlyInput::default()
        };
        if flying {
            ui.input(|i| {
                // Raw mouse movement keeps going when the hidden pointer
                // reaches the edge of the screen.
                let look = i.pointer.motion().unwrap_or(i.pointer.delta());
                input.look = glam::Vec2::new(look.x, look.y);
                let axis = |plus: Key, minus: Key| {
                    f32::from(u8::from(i.key_down(plus))) - f32::from(u8::from(i.key_down(minus)))
                };
                input.movement = glam::Vec3::new(
                    axis(Key::D, Key::A),
                    axis(Key::W, Key::S),
                    axis(Key::E, Key::Q),
                );
                input.boost = i.modifiers.shift;
            });
            input.speed_notches = notches as i32;
        } else {
            input.dolly_notches = notches;
            if response.dragged_by(PointerButton::Middle) {
                let delta = response.drag_delta();
                input.pan = glam::Vec2::new(delta.x, delta.y);
            }
        }
        self.camera.fly(&input);
    }

    fn stats(&self, ui: &mut Ui) {
        theme::overlay_frame()
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{:.0} fps · {:.1} ms",
                            self.frames_per_second(),
                            self.frame_time * 1000.0
                        ))
                        .monospace()
                        .size(11.0),
                    );
                    ui.label(RichText::new("0 entities").monospace().size(11.0));
                    if self.triangles > 0 {
                        ui.label(
                            RichText::new(format!(
                                "{} triangles",
                                crate::models::grouped(self.triangles)
                            ))
                            .monospace()
                            .size(11.0),
                        );
                    }
                });
            });
    }

    fn view_controls(&mut self, ui: &mut Ui) {
        // Bottom-up: the row of buttons first, the ≡ menu above it.
        ui.horizontal(|ui| {
            let menu = ui
                .add(egui::Button::new("≡").selected(self.menu_open))
                .on_hover_text("What the viewport shows");
            if menu.clicked() {
                self.menu_open = !self.menu_open;
            }
            up_menu(ui, "Perspective ▴", |ui| {
                let _ = ui.add(egui::Button::new("Perspective").selected(true));
                for view in ["Top", "Front", "Right"] {
                    ui.add_enabled(false, egui::Button::new(view))
                        .on_disabled_hover_text("Flat views arrive in Phase 1");
                }
            });
            let look = &mut self.look;
            up_menu(ui, format!("{} ▴", look.kind.name()), |ui| {
                look_menu(ui, look)
            });
            up_menu(ui, "Show ▴", |ui| {
                for thing in ["Grid", "Axes", "Lights", "Cameras", "Colliders"] {
                    ui.add_enabled(false, egui::Checkbox::new(&mut true, thing))
                        .on_disabled_hover_text(
                            "Choosing what to show arrives as there is more to show",
                        );
                }
            });
        });
        if self.menu_open {
            theme::overlay_frame()
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_min_width(220.0);
                    ui.label(RichText::new("Show in the viewport").color(theme::TEXT_WEAK));
                    ui.checkbox(&mut self.overlays.tools, "Tools");
                    ui.checkbox(&mut self.overlays.snapping, "Grid, angle and camera speed");
                    ui.checkbox(&mut self.overlays.stats, "Frame rate and entity count");
                });
        }
    }

    fn tools(&mut self, ui: &mut Ui, accent: Accent) {
        theme::overlay_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = TOOL_GAP;
                for tool in Tool::ALL {
                    if tool_button(ui, tool, self.tool == tool, accent).clicked() {
                        self.tool = tool;
                    }
                }
            });
        });
    }
}

/// A menu that opens upwards from its button (the ▴ says so), since the
/// buttons sit at the bottom of the viewport. It stays open while its
/// settings are changed, and closes on a click elsewhere.
fn up_menu(ui: &mut Ui, title: impl Into<egui::WidgetText>, content: impl FnOnce(&mut Ui)) {
    let response = ui.add(egui::Button::new(title));
    egui::Popup::menu(&response)
        .align(egui::RectAlign::TOP_START)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(content);
}

/// The look menu: the three looks, and the stylized look's settings.
fn look_menu(ui: &mut Ui, look: &mut Look) {
    for kind in LookKind::ALL {
        if ui
            .add(egui::Button::new(kind.name()).selected(look.kind == kind))
            .on_hover_text(kind.hint())
            .clicked()
        {
            look.kind = kind;
        }
    }
    ui.add_enabled(false, egui::Button::new("Wireframe"))
        .on_disabled_hover_text("Wireframe arrives with the editor's tools in Phase 3");
    ui.separator();
    let stylized = look.kind == LookKind::Stylized;
    ui.add_enabled_ui(stylized, |ui| {
        ui.label(RichText::new("Stylized look").color(theme::TEXT_WEAK));
        ui.horizontal(|ui| {
            ui.label("Bands of light");
            for bands in Look::BANDS {
                if ui
                    .add(egui::Button::new(bands.to_string()).selected(look.bands == bands))
                    .clicked()
                {
                    look.bands = bands;
                }
            }
        });
        ui.checkbox(&mut look.outlines, "Outlines");
    })
    .response
    .on_disabled_hover_text("Choose Stylized to change these");
}

/// How the hidden pointer is held while looking around: macOS can only
/// lock it in place, other systems can only keep it inside the window.
fn grab_mode() -> egui::CursorGrab {
    if cfg!(target_os = "macos") {
        egui::CursorGrab::Locked
    } else {
        egui::CursorGrab::Confined
    }
}

/// Points of scrolling that count as one wheel notch, for trackpads and
/// mice that report points.
const POINTS_PER_NOTCH: f32 = 50.0;

/// Whole wheel notches this frame (up is positive), and the part of a notch
/// left over, added to `carry` from earlier frames.
fn wheel_notches(ui: &Ui, carry: f32) -> (f32, f32) {
    let scrolled = ui.input(|i| {
        i.events
            .iter()
            .map(|event| match event {
                egui::Event::MouseWheel { unit, delta, .. } => match unit {
                    egui::MouseWheelUnit::Line => delta.y,
                    egui::MouseWheelUnit::Point => delta.y / POINTS_PER_NOTCH,
                    egui::MouseWheelUnit::Page => delta.y * 3.0,
                },
                _ => 0.0,
            })
            .sum::<f32>()
    });
    let total = carry + scrolled;
    if !total.is_finite() {
        return (0.0, 0.0);
    }
    let whole = total.trunc();
    (whole, total - whole)
}

/// One tool button's size.
const TOOL_BUTTON: Vec2 = vec2(30.0, 26.0);
/// Space between tool buttons.
const TOOL_GAP: f32 = 2.0;
/// The tool bar's frame: margin plus border, each side.
const TOOL_BAR_EDGE: f32 = 4.0;
/// The whole tool bar's size.
const TOOL_BAR_SIZE: Vec2 = vec2(
    Tool::ALL.len() as f32 * TOOL_BUTTON.x
        + (Tool::ALL.len() - 1) as f32 * TOOL_GAP
        + 2.0 * TOOL_BAR_EDGE,
    TOOL_BUTTON.y + 2.0 * TOOL_BAR_EDGE,
);

/// One tool's icon button, filled with the accent when chosen.
fn tool_button(ui: &mut Ui, tool: Tool, chosen: bool, accent: Accent) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(TOOL_BUTTON, Sense::click());
    let response = response.on_hover_text(tool.hint());
    let fill = if chosen {
        accent.colors().fill
    } else if response.hovered() {
        theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    let color = if chosen {
        Color32::WHITE
    } else {
        theme::TEXT_STRONG
    };
    icons::tool_icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(16.0, 16.0)),
        tool,
        color,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, tool.hint())
    });
    response
}

/// Grid, angle and camera speed, bottom right.
fn snapping(ui: &mut Ui, accent: Accent, camera_speed: u8) {
    let speed = camera_speed.to_string();
    // Laid out from the right edge, so the boxes go in backwards to read
    // "Grid, Angle, Camera" from left to right.
    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for (label, value) in [
            ("Camera", speed.as_str()),
            ("Angle", "15°"),
            ("Grid", "1 m"),
        ] {
            theme::overlay_frame()
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    // Still right to left inside: the value goes in first.
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(value).color(accent.colors().light).strong());
                        ui.label(label);
                    });
                });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_its_own_key() {
        let mut keys: Vec<_> = Tool::ALL.iter().map(|t| t.key()).collect();
        keys.sort_by_key(|k| k.name());
        keys.dedup();
        assert_eq!(keys.len(), Tool::ALL.len());
        assert_eq!(Tool::Move.hint(), "Move (W)");
        assert_eq!(Tool::Transform.key(), egui::Key::T);
    }

    #[test]
    fn the_view_carries_the_size_and_backdrop() {
        let view = Viewport::default().view([800, 600], 1.0);
        assert_eq!(view.size_px, [800, 600]);
        assert_eq!(view.sky_top, rgb(theme::backdrop::SKY_TOP));
        assert!(view.view_projection.is_finite());
    }

    #[test]
    fn every_overlay_shows_by_default() {
        let o = Overlays::default();
        assert!(o.tools && o.snapping && o.stats);
    }

    #[test]
    fn old_saved_overlays_fill_in_new_switches() {
        let o: Overlays = ron::from_str("(stats: false)").unwrap();
        assert!(!o.stats);
        assert!(o.tools && o.snapping);
    }

    #[test]
    fn no_renderer_explains_itself() {
        let mut r = NoRenderer {
            reason: "no GPU".into(),
        };
        assert_eq!(
            r.render(&Viewport::default().view([1, 1], 1.0)),
            Err("no GPU".to_string())
        );
    }
}
