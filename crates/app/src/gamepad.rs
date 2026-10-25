//! Gamepads ([decision 0037](../../../docs/decisions/0037-input-as-named-actions.md)):
//! the first connected one, read each frame with `gilrs` on Windows (the
//! only system Solis is made for, decision 0012), and handed to the
//! editor as `input::Pad`. Elsewhere (the Linux machines checks run on)
//! there's no gamepad.

/// Reads the gamepad, if the system can.
#[derive(Default)]
pub struct Gamepads {
    #[cfg(windows)]
    gilrs: Option<gilrs::Gilrs>,
    #[cfg(windows)]
    tried: bool,
}

impl Gamepads {
    /// Ready to read (gilrs starts at the first read).
    pub fn new() -> Self {
        Self::default()
    }

    /// The first connected gamepad as it is now, if any.
    #[cfg(windows)]
    pub fn read(&mut self) -> Option<input::Pad> {
        use gilrs::{Axis, Button};
        if !self.tried {
            self.tried = true;
            self.gilrs = gilrs::Gilrs::new().ok();
        }
        let gilrs = self.gilrs.as_mut()?;
        // Events keep gilrs's own state up to date.
        while gilrs.next_event().is_some() {}
        let (_, pad) = gilrs.gamepads().find(|(_, p)| p.is_connected())?;
        let mut buttons = std::collections::BTreeSet::new();
        for (button, name) in [
            (Button::South, "A"),
            (Button::East, "B"),
            (Button::West, "X"),
            (Button::North, "Y"),
            (Button::LeftTrigger, "LB"),
            (Button::RightTrigger, "RB"),
            (Button::Select, "Back"),
            (Button::Start, "Start"),
            (Button::LeftThumb, "LeftStick"),
            (Button::RightThumb, "RightStick"),
            (Button::DPadUp, "Up"),
            (Button::DPadDown, "Down"),
            (Button::DPadLeft, "Left"),
            (Button::DPadRight, "Right"),
        ] {
            if pad.is_pressed(button) {
                buttons.insert(name.to_string());
            }
        }
        // Triggers count as pressed past half way.
        for (button, name) in [(Button::LeftTrigger2, "LT"), (Button::RightTrigger2, "RT")] {
            if pad.button_data(button).is_some_and(|d| d.value() > 0.5) {
                buttons.insert(name.to_string());
            }
        }
        let stick = |x: Axis, y: Axis| glam::Vec2::new(pad.value(x), pad.value(y));
        Some(input::Pad {
            buttons,
            left: stick(Axis::LeftStickX, Axis::LeftStickY),
            right: stick(Axis::RightStickX, Axis::RightStickY),
        })
    }

    /// No gamepads here.
    #[cfg(not(windows))]
    pub fn read(&mut self) -> Option<input::Pad> {
        None
    }
}
