//! Flying the viewport's camera from the mouse and keys
//! ([decision 0009](../../../docs/decisions/0009-fly-camera-controls.md)),
//! and the wheel's notches.

use super::Viewport;
use crate::camera::FlyInput;
use crate::viewport_tools;
use egui::Ui;

impl Viewport {
    /// Flies the camera from this frame's mouse and keys
    /// ([decision 0009](../../../docs/decisions/0009-fly-camera-controls.md)).
    pub(super) fn navigate(&mut self, ui: &Ui, response: &egui::Response, dt: f32, painting: bool) {
        use egui::{Key, PointerButton, ViewportCommand};
        let ctx = ui.ctx();
        let (pressed_here, held, pointer) = ui.input(|i| {
            (
                i.pointer.button_pressed(PointerButton::Secondary),
                i.pointer.button_down(PointerButton::Secondary),
                i.pointer.interact_pos(),
            )
        });
        // Flat views don't fly: the right button slides them instead.
        let flat = self.camera.is_flat();
        // While the left button is doing something for the tools (a click
        // on its way, a box, a gizmo drag), the right button is theirs: it
        // cancels a box or a drag, and never starts flying over one.
        let tools_idle = self.tools.pressing == viewport_tools::Pressing::Nothing;
        let starting = !flat
            && !painting
            && self.flying_from.is_none()
            && tools_idle
            && pressed_here
            && response.hovered()
            && response.is_pointer_button_down_on();
        if starting {
            self.flying_from = pointer.or(Some(response.rect.center()));
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
            view_width: response.rect.width(),
            mouse: pointer
                .map(|p| glam::Vec2::new(p.x - response.rect.min.x, p.y - response.rect.min.y)),
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
            let slides = response.dragged_by(PointerButton::Middle)
                || (flat && !painting && response.dragged_by(PointerButton::Secondary));
            if slides {
                let delta = response.drag_delta();
                input.pan = glam::Vec2::new(delta.x, delta.y);
            }
        }
        self.camera.fly(&input);
    }
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
pub(crate) const POINTS_PER_NOTCH: f32 = 50.0;

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
