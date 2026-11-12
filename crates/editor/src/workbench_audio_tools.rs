//! What Audio's ribbon does (Screen 10; round 3, `08-audio-mixer.png`):
//! a new bus, a ducking rule, removing the bus shown, clearing solo,
//! muting the game and how many sounds are heard at once. Each is a
//! command (`shell::ShellCommand`) that makes the same `MixerRequest`s as
//! the strips and the Bus group, so it saves as they do.

use crate::mixer_view::MixerRequest;
use crate::shell::ShellCommand;
use crate::workbench::{Workbench, WorkbenchAction};

/// How many sounds can be heard at once: the choices Monitor's list
/// offers (within `sound::voices::VOICES_RANGE`).
pub(crate) const HEARD_AT_ONCE: [u16; 5] = [16, 32, 64, 128, 256];

impl Workbench {
    /// Why one of Audio's commands can't be done now.
    pub(crate) fn why_not_audio(&self, command: ShellCommand) -> Option<String> {
        use ShellCommand as C;
        let state = &self.open.mixer;
        let say = |why: &str| Some(why.to_owned());
        match command {
            C::AddBus if state.mixer.buses.len() >= sound::mixer::MAX_BUSES => {
                say("A project has 64 buses at most")
            }
            C::AddDuckingRule => {
                let (_, bus) = crate::mixer_view::with_bus_shown(state);
                if state.mixer.buses.len() < 2 {
                    say("Add another bus first: a bus is ducked while another plays")
                } else if bus.ducking.len() >= sound::mixer::MAX_RULES {
                    say("A bus has 8 ducking rules at most")
                } else {
                    None
                }
            }
            C::RemoveBus if state.bus_shown() == 0 => {
                say("Master is the top bus and stays: pick another bus in the mixer")
            }
            C::ClearSolo if state.listening.soloed.is_empty() => say("No bus is soloed"),
            _ => None,
        }
    }

    /// Does one of Audio's commands.
    pub(crate) fn run_audio(&mut self, command: ShellCommand, actions: &mut Vec<WorkbenchAction>) {
        use ShellCommand as C;
        let state = &self.open.mixer;
        let requests = match command {
            C::AddBus => crate::mixer_view::add_bus(state),
            C::AddDuckingRule => crate::bus_view::add_rule(state),
            C::RemoveBus => crate::bus_view::remove_bus(state),
            C::ClearSolo => vec![MixerRequest::ClearSolo],
            C::MuteGame => vec![MixerRequest::MuteAll],
            C::HeardAtOnce(n) => {
                let mut mixer = state.mixer.clone();
                mixer.voices = u32::from(n);
                vec![MixerRequest::Changed { mixer, save: true }]
            }
            _ => Vec::new(),
        };
        for request in requests {
            self.mixer_request(request, actions);
        }
    }
}
