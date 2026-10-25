//! A project's actions and their bindings, and their file.

use serde::{Deserialize, Serialize};

/// The most actions a project has.
pub const MAX_ACTIONS: usize = 256;
/// The most inputs one action (or one direction of an axis) has.
pub const MAX_INPUTS: usize = 16;
/// The biggest bindings file read.
pub(crate) const MAX_FILE_BYTES: usize = 1 << 20;

/// What an action gives game code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionKind {
    /// Down or up: `held`, `pressed`, `released`.
    #[default]
    Button,
    /// Two numbers, across (x) and up (y), each from -1 to 1 for keys and
    /// sticks (which way to go); with `Mouse Motion` or `Mouse Wheel`, how
    /// far to go this tick (see [`Action::mouse_speed`]).
    Axis,
}

/// One action and what sets it off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    /// Its name, as game code asks for it (`"Jump"`).
    pub name: String,
    /// A button or an axis.
    #[serde(default)]
    pub kind: ActionKind,
    /// A button's inputs (keys, mouse buttons, pad buttons); an axis's
    /// whole-axis inputs (`Pad LeftStick`, `Mouse Motion`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// An axis's inputs pushing it up (y +1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub up: Vec<String>,
    /// … down (y -1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub down: Vec<String>,
    /// … left (x -1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<String>,
    /// … right (x +1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub right: Vec<String>,
    /// For an axis with `Mouse Motion` or `Mouse Wheel`: how far a pixel
    /// of mouse movement (or a notch of the wheel) goes, in the game's own
    /// units (Look's are degrees, Zoom's steps).
    #[serde(default = "default_mouse_speed")]
    pub mouse_speed: f32,
    /// For an axis with `Mouse Motion` or `Mouse Wheel`: how far a stick
    /// or key held all the way goes in a second, in the same units.
    #[serde(default = "default_pad_speed")]
    pub pad_speed: f32,
}

fn default_mouse_speed() -> f32 {
    0.15
}

fn default_pad_speed() -> f32 {
    180.0
}

impl Action {
    /// A button action set off by `inputs`.
    pub fn button(name: &str, inputs: &[&str]) -> Self {
        Self {
            name: name.into(),
            kind: ActionKind::Button,
            inputs: inputs.iter().map(|s| (*s).to_string()).collect(),
            up: Vec::new(),
            down: Vec::new(),
            left: Vec::new(),
            right: Vec::new(),
            mouse_speed: default_mouse_speed(),
            pad_speed: default_pad_speed(),
        }
    }

    /// An axis action, from whole-axis `inputs` and four directions'
    /// keys (up, down, left, right).
    pub fn axis(name: &str, inputs: &[&str], [up, down, left, right]: [&[&str]; 4]) -> Self {
        let list = |l: &[&str]| l.iter().map(|s| (*s).to_string()).collect();
        Self {
            kind: ActionKind::Axis,
            up: list(up),
            down: list(down),
            left: list(left),
            right: list(right),
            ..Self::button(name, inputs)
        }
    }

    /// Whether it goes with the mouse's movement or wheel: its values are
    /// then how far to go this tick, not which way.
    pub fn is_motion(&self) -> bool {
        self.kind == ActionKind::Axis
            && self
                .inputs
                .iter()
                .any(|i| i == crate::MOUSE_MOTION || i == crate::MOUSE_WHEEL)
    }

    /// Every input it has, in any role.
    pub fn all_inputs(&self) -> impl Iterator<Item = &String> {
        self.inputs
            .iter()
            .chain(&self.up)
            .chain(&self.down)
            .chain(&self.left)
            .chain(&self.right)
    }

    /// The first input of the kind `prefix` names (`""` a key, `"Mouse "`,
    /// `"Pad "`), for showing a player which to press.
    pub fn first_input(&self, prefix: &str) -> Option<&str> {
        let kind_of = |i: &&String| {
            if i.starts_with("Pad ") {
                "Pad "
            } else if i.starts_with("Mouse ") {
                "Mouse "
            } else {
                ""
            }
        };
        self.all_inputs()
            .find(|i| kind_of(i) == prefix)
            .map(String::as_str)
    }
}

/// A project's actions, in the order they were made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bindings {
    /// The actions.
    #[serde(rename = "action", default)]
    pub actions: Vec<Action>,
}

/// Why a bindings file couldn't be read.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingsError {
    /// Bigger than 1 MB.
    TooBig,
    /// Not TOML, or not the shape of one.
    Damaged(String),
    /// Something in it isn't allowed: a name, an input, too many.
    Wrong(String),
}

impl std::fmt::Display for BindingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooBig => write!(f, "it's bigger than 1 MB"),
            Self::Damaged(why) => write!(f, "it isn't readable ({why})"),
            Self::Wrong(why) => write!(f, "{why}"),
        }
    }
}

impl Default for Bindings {
    /// The ready-made actions: Move, Look, Jump, Sprint and Interact
    /// (decision 0037), and the cameras' Pan, Turn, Zoom and Drag
    /// (decision 0041).
    fn default() -> Self {
        let mut look = Action::axis(
            "Look",
            &[crate::MOUSE_MOTION, "Pad RightStick"],
            [&[], &[], &[], &[]],
        );
        look.mouse_speed = default_mouse_speed();
        look.pad_speed = default_pad_speed();
        let mut zoom = Action::axis(
            "Zoom",
            &[crate::MOUSE_WHEEL],
            [&["PageUp", "Pad RT"], &["PageDown", "Pad LT"], &[], &[]],
        );
        zoom.mouse_speed = 1.0;
        zoom.pad_speed = 8.0;
        Self {
            actions: vec![
                Action::axis("Move", &["Pad LeftStick"], [&["W"], &["S"], &["A"], &["D"]]),
                look,
                Action::button("Jump", &["Space", "Pad A"]),
                Action::button("Sprint", &["Shift", "Pad LeftStick"]),
                Action::button("Interact", &["F", "Pad X"]),
                Action::axis(
                    "Pan",
                    &["Pad LeftStick"],
                    [&["Up"], &["Down"], &["Left"], &["Right"]],
                ),
                Action::axis(
                    "Turn",
                    &["Pad RightStick"],
                    [&[], &[], &["Alt+Left"], &["Alt+Right"]],
                ),
                zoom,
                Action::button("Drag", &["Mouse Middle"]),
            ],
        }
    }
}

impl Bindings {
    /// Adds the ready-made actions (see [`Bindings::default`]) it's
    /// missing, at the end; returns their names. A project from before
    /// the cameras gets Pan, Turn, Zoom and Drag this way.
    pub fn add_missing(&mut self) -> Vec<String> {
        let mut added = Vec::new();
        for action in Self::default().actions {
            if self.get(&action.name).is_none() && self.actions.len() < MAX_ACTIONS {
                added.push(action.name.clone());
                self.actions.push(action);
            }
        }
        added
    }

    /// The action called `name`.
    pub fn get(&self, name: &str) -> Option<&Action> {
        self.actions.iter().find(|a| a.name == name)
    }

    /// The action called `name`, to change.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Action> {
        self.actions.iter_mut().find(|a| a.name == name)
    }

    /// Reads a bindings file, refusing anything past the limits or not
    /// allowed (never panics on damage).
    pub fn read(text: &str) -> Result<Self, BindingsError> {
        if text.len() > MAX_FILE_BYTES {
            return Err(BindingsError::TooBig);
        }
        let mut bindings: Self =
            toml::from_str(text).map_err(|e| BindingsError::Damaged(e.message().to_string()))?;
        // The arrow keys are `Up`, `Down`, `Left` and `Right` (egui's
        // names, which pressed keys come as); files from before this was
        // found wrote `ArrowUp` and so on.
        for action in &mut bindings.actions {
            for list in [
                &mut action.inputs,
                &mut action.up,
                &mut action.down,
                &mut action.left,
                &mut action.right,
            ] {
                for input in list.iter_mut() {
                    for way in ["Up", "Down", "Left", "Right"] {
                        let old = format!("Arrow{way}");
                        if input.ends_with(&old) && !input.starts_with("Pad ") {
                            *input = format!("{}{way}", &input[..input.len() - old.len()]);
                        }
                    }
                }
            }
        }
        bindings.check()?;
        Ok(bindings)
    }

    /// Whether everything is allowed: sensible, distinct names; known
    /// inputs; within the limits; sensible speeds.
    pub fn check(&self) -> Result<(), BindingsError> {
        if self.actions.len() > MAX_ACTIONS {
            return Err(BindingsError::Wrong(format!(
                "it has more than {MAX_ACTIONS} actions"
            )));
        }
        let mut names = std::collections::HashSet::new();
        for action in &self.actions {
            if !crate::valid_action_name(&action.name) {
                return Err(BindingsError::Wrong(format!(
                    "\"{}\" isn't an action name: use letters, digits and _, starting with a letter",
                    action.name.chars().take(64).collect::<String>()
                )));
            }
            if !names.insert(action.name.as_str()) {
                return Err(BindingsError::Wrong(format!(
                    "two actions are called {}",
                    action.name
                )));
            }
            for list in [
                &action.inputs,
                &action.up,
                &action.down,
                &action.left,
                &action.right,
            ] {
                if list.len() > MAX_INPUTS {
                    return Err(BindingsError::Wrong(format!(
                        "{} has more than {MAX_INPUTS} inputs in one list",
                        action.name
                    )));
                }
                if let Some(bad) = list.iter().find(|i| !crate::valid_input_name(i)) {
                    return Err(BindingsError::Wrong(format!(
                        "{} has \"{}\", which isn't a key, mouse or gamepad input",
                        action.name,
                        bad.chars().take(32).collect::<String>()
                    )));
                }
            }
            for speed in [action.mouse_speed, action.pad_speed] {
                if !(speed.is_finite() && (0.0..=100_000.0).contains(&speed)) {
                    return Err(BindingsError::Wrong(format!(
                        "{} has a speed that isn't a sensible number",
                        action.name
                    )));
                }
            }
        }
        Ok(())
    }

    /// The file's text: TOML, one line per setting, so changes merge in
    /// Git.
    pub fn write(&self) -> String {
        let body = toml::to_string(self).unwrap_or_default();
        format!(
            "# The game's input actions (decision 0037). Edit > Input actions… changes them;\n\
             # game code asks for an action by its name: held(\"Jump\").\n\n{body}"
        )
    }

    /// Binds `input` to the action `name` in place of the inputs of the
    /// same kind (a key replaces its keys, a pad button its pad buttons,
    /// a mouse button its mouse buttons): what a player's rebinding does.
    /// Returns whether it could.
    pub fn rebind(&mut self, name: &str, input: &str) -> bool {
        if !crate::valid_input_name(input) {
            return false;
        }
        let Some(action) = self.get_mut(name) else {
            return false;
        };
        let kind = |i: &str| {
            if i.starts_with("Pad ") {
                1
            } else if i.starts_with("Mouse ") {
                2
            } else {
                0
            }
        };
        let k = kind(input);
        // An axis's direction keys stay (rebinding a direction needs the
        // Input actions window); its whole-axis inputs are what changes.
        action.inputs.retain(|i| kind(i) != k);
        if action.inputs.len() >= MAX_INPUTS {
            // Full of other kinds: never past the limit a file is read
            // within, so a save holding these bindings still loads.
            return false;
        }
        action.inputs.push(input.to_string());
        true
    }
}
