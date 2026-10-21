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

use crate::icons;
use crate::theme::{self, Accent};
use egui::{Align, Color32, Layout, Rect, RichText, Sense, Ui, UiBuilder, Vec2, vec2};
use glam::{Mat4, Vec3, Vec4};
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

/// The viewport's camera. For now it holds still; flying around (the only
/// camera mode, decision 0004) arrives with the rest of navigation in
/// Phase 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Where the camera is, in metres.
    pub position: Vec3,
    /// The point it looks at.
    pub target: Vec3,
    /// Vertical field of view, in radians.
    pub fov_y: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(-16.0, -24.0, 10.0),
            target: Vec3::new(0.0, 0.0, 1.0),
            fov_y: 50f32.to_radians(),
        }
    }
}

impl Camera {
    /// Nearest distance drawn, in metres.
    pub const NEAR: f32 = 0.05;
    /// Furthest distance drawn, in metres.
    pub const FAR: f32 = 5_000.0;

    /// World-to-screen transform for an image `aspect` wide per unit high
    /// (depth 0 at near, 1 at far). Z is up.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let aspect = if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            1.0
        };
        // Right-handed perspective with depth 0 at near and 1 at far, as
        // wgpu expects.
        let y_scale = 1.0 / (self.fov_y * 0.5).tan();
        let x_scale = y_scale / aspect;
        let depth = Self::FAR / (Self::NEAR - Self::FAR);
        let projection = Mat4::from_cols(
            Vec4::new(x_scale, 0.0, 0.0, 0.0),
            Vec4::new(0.0, y_scale, 0.0, 0.0),
            Vec4::new(0.0, 0.0, depth, -1.0),
            Vec4::new(0.0, 0.0, depth * Self::NEAR, 0.0),
        );
        projection * self.view_matrix()
    }

    /// World-to-camera transform: the camera looks down its own -Z, with Y
    /// up on screen and the world's Z up.
    pub fn view_matrix(&self) -> Mat4 {
        let forward = (self.target - self.position).normalize_or(Vec3::Y);
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward);
        Mat4::from_cols(
            Vec4::new(right.x, up.x, -forward.x, 0.0),
            Vec4::new(right.y, up.y, -forward.y, 0.0),
            Vec4::new(right.z, up.z, -forward.z, 0.0),
            Vec4::new(
                -right.dot(self.position),
                -up.dot(self.position),
                forward.dot(self.position),
                1.0,
            ),
        )
    }

    /// The point on the ground the grid is centred on: below the target.
    pub fn focus(&self) -> Vec3 {
        Vec3::new(self.target.x, self.target.y, 0.0)
    }
}

/// What one viewport frame needs from the renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            tool: Tool::Select,
            overlays: Overlays::default(),
            menu_open: false,
            frame_time: 1.0 / 60.0,
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

    /// The frame rate shown in the corner.
    pub fn frames_per_second(&self) -> f32 {
        1.0 / self.frame_time.max(1e-4)
    }

    /// The view for an image of `size_px` physical pixels.
    pub fn view(&self, size_px: [u32; 2]) -> ViewportView {
        let aspect = size_px[0].max(1) as f32 / size_px[1].max(1) as f32;
        ViewportView {
            size_px,
            view_projection: self.camera.view_projection(aspect),
            camera_position: self.camera.position,
            focus: self.camera.focus(),
            sky_top: rgb(theme::backdrop::SKY_TOP),
            sky_horizon: rgb(theme::backdrop::HORIZON),
            ground: rgb(theme::backdrop::GROUND),
        }
    }

    /// Draws the viewport in `ui`, filling it.
    pub fn show(&mut self, ui: &mut Ui, renderer: &mut dyn ViewportRenderer, accent: Accent) {
        let dt = ui.input(|i| i.unstable_dt);
        if dt.is_finite() && dt > 0.0 {
            self.frame_time += (dt - self.frame_time) * FRAME_TIME_SMOOTHING;
        }
        let rect = ui.max_rect();
        let response = ui.allocate_rect(rect, Sense::click());
        if response.clicked() {
            self.menu_open = false;
        }
        let ppp = ui.ctx().pixels_per_point();
        let size_px = [
            (rect.width() * ppp).round().max(1.0) as u32,
            (rect.height() * ppp).round().max(1.0) as u32,
        ];
        let painter = ui.painter_at(rect);
        match renderer.render(&self.view(size_px)) {
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
            snapping(&mut bottom_right, accent);
        }
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
            ui.menu_button("Perspective ▴", |ui| {
                let _ = ui.add(egui::Button::new("Perspective").selected(true));
                for view in ["Top", "Front", "Right"] {
                    ui.add_enabled(false, egui::Button::new(view))
                        .on_disabled_hover_text("Flat views arrive in Phase 1");
                }
            });
            ui.menu_button("Lit ▴", |ui| {
                let _ = ui.add(egui::Button::new("Lit").selected(true));
                for look in ["Unlit", "Wireframe"] {
                    ui.add_enabled(false, egui::Button::new(look))
                        .on_disabled_hover_text("Other looks arrive with lighting in Phase 1");
                }
            });
            ui.menu_button("Show ▴", |ui| {
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
fn snapping(ui: &mut Ui, accent: Accent) {
    // Laid out from the right edge, so the boxes go in backwards to read
    // "Grid, Angle, Camera" from left to right.
    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for (label, value) in [("Camera", "4"), ("Angle", "15°"), ("Grid", "1 m")] {
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
    fn the_camera_looks_at_its_target() {
        let camera = Camera::default();
        let clip = camera.view_projection(16.0 / 9.0) * camera.target.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        assert!(ndc.x.abs() < 1e-4 && ndc.y.abs() < 1e-4, "{ndc}");
        assert!((0.0..1.0).contains(&ndc.z), "depth 0 to 1");
    }

    #[test]
    fn up_on_screen_is_up_in_the_world() {
        let camera = Camera::default();
        let vp = camera.view_projection(1.0);
        let project = |p: Vec3| {
            let c = vp * p.extend(1.0);
            c.truncate() / c.w
        };
        assert!(project(camera.target + Vec3::Z).y > project(camera.target).y);
    }

    #[test]
    fn a_bad_aspect_does_not_break_the_camera() {
        let camera = Camera::default();
        for aspect in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(camera.view_projection(aspect).is_finite());
        }
    }

    #[test]
    fn the_grid_is_centred_under_the_target() {
        let camera = Camera::default();
        assert_eq!(camera.focus(), Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn the_view_carries_the_size_and_backdrop() {
        let view = Viewport::default().view([800, 600]);
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
            r.render(&Viewport::default().view([1, 1])),
            Err("no GPU".to_string())
        );
    }
}
