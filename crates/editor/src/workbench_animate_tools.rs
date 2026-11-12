//! What Animate's ribbon does (the approved additions, round 3,
//! `06-animate-ribbons.png`): a clips file's clips, frames and playback,
//! and an animator's states and preview. Each is a command
//! (`shell::ShellCommand`); each change is one undo step of the file's and
//! saved at once, as from its own tabs.

use crate::animator_editor::Picked;
use crate::shell::ShellCommand;
use crate::workbench::Workbench;

impl Workbench {
    /// The clip chosen in the open clips file, as its sheet makes it.
    pub(crate) fn chosen_clip(&self) -> Option<animation::Clip> {
        let editor = self.open.frames_editor.as_ref()?;
        let sheet = self.viewport.sheets.get(editor.file.sheet?)?;
        editor.clips(sheet).get(&editor.clip).cloned()
    }

    /// Why one of Animate's commands can't be done now.
    pub(crate) fn why_not_animate(&self, command: ShellCommand) -> Option<String> {
        use ShellCommand as C;
        let say = |why: &str| Some(why.to_owned());
        match command {
            C::ClipNew | C::ClipCopy | C::ClipDelete | C::FramesTakeOut | C::ClipDirection(_) => {
                let Some(editor) = &self.open.frames_editor else {
                    return say("Open a clips file from the Library first");
                };
                let Some(clip) = self.chosen_clip() else {
                    return (command != C::ClipNew)
                        .then(|| "Choose its sheet, then a clip, first".to_owned());
                };
                match command {
                    C::FramesTakeOut if clip.from_aseprite => {
                        say("An Aseprite tag's frames are Aseprite's")
                    }
                    C::FramesTakeOut if editor.selected.is_empty() => {
                        say("Pick frames on the Frames tab first")
                    }
                    C::ClipDirection(_) if clip.from_aseprite => {
                        say("An Aseprite tag's direction is Aseprite's")
                    }
                    _ => None,
                }
            }
            C::StateAdd | C::StateStartHere | C::StateDelete | C::PreviewAgain => {
                let Some(editor) = &self.open.animator_editor else {
                    return say("Open an animator from the Library first");
                };
                match command {
                    C::StateStartHere if picked_state(&editor.picked).is_none() => {
                        say("Pick one state in the graph first")
                    }
                    C::StateDelete if editor.picked == Picked::Nothing => {
                        say("Pick a state or a transition in the graph first")
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Does one of Animate's commands.
    pub(crate) fn run_animate(&mut self, command: ShellCommand) {
        use ShellCommand as C;
        match command {
            C::ClipNew | C::ClipCopy | C::ClipDelete | C::FramesTakeOut | C::ClipDirection(_) => {
                self.run_clips_command(command);
            }
            _ => self.run_animator_command(command),
        }
    }

    /// A clips file's command, saved when it changed the file.
    fn run_clips_command(&mut self, command: ShellCommand) {
        use ShellCommand as C;
        let sheets = std::sync::Arc::clone(&self.viewport.sheets);
        let Some(editor) = &mut self.open.frames_editor else {
            return;
        };
        let Some(sheet) = editor.file.sheet.and_then(|id| sheets.get(id)) else {
            return;
        };
        let clips = editor.clips(sheet);
        let changed = match command {
            C::ClipNew => editor.new_clip(&clips, Vec::new()),
            C::ClipCopy => editor.copy_clip(&clips),
            C::ClipDelete => editor.delete_clip(),
            C::FramesTakeOut => editor.delete_frames(),
            C::ClipDirection(i) => crate::shell::animate_direction(i)
                .is_some_and(|direction| editor.set_direction(direction)),
            _ => false,
        };
        if changed {
            self.save_frames();
        }
    }

    /// An animator's command, saved when it changed the file.
    fn run_animator_command(&mut self, command: ShellCommand) {
        use ShellCommand as C;
        let Some(editor) = &mut self.open.animator_editor else {
            return;
        };
        let changed = match command {
            C::StateAdd => {
                // Below the states there are, so a new one is never on
                // top of another.
                let lowest = editor
                    .file
                    .states
                    .iter()
                    .map(|s| s.place[1])
                    .fold(0.0_f32, f32::max);
                editor.add_state([0.0, lowest + NEW_STATE_GAP])
            }
            C::StateStartHere => {
                picked_state(&editor.picked).is_some_and(|id| editor.set_entry(id))
            }
            C::StateDelete => editor.delete_picked(),
            C::PreviewAgain => {
                editor.restart_preview();
                false
            }
            _ => false,
        };
        if changed {
            self.save_animator();
        }
    }
}

/// How far below the lowest state a new one goes.
const NEW_STATE_GAP: f32 = 80.0;

/// The one state picked, if exactly one (not Any state) is.
fn picked_state(picked: &Picked) -> Option<reflect::Id> {
    match picked {
        Picked::States(states) if states.len() == 1 => states.iter().next().copied().flatten(),
        _ => None,
    }
}
