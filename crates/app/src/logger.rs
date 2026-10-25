//! Messages the libraries write through the `log` facade, shown in the
//! Console: warnings and errors from every library (the GPU driver
//! layer, say), and plain information from Solis's own crates only, so the
//! Console isn't flooded.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};

/// The most messages kept waiting for the window to take them.
const WAITING_LIMIT: usize = 500;

/// Solis's own crates, whose information messages are shown too.
const OWN: &[&str] = &[
    "app", "editor", "render", "assets", "asset_db", "area", "reflect", "world",
];

/// Libraries whose warnings say nothing a game maker can act on (the
/// file window's library noting which way it opened, the window library
/// noting keyboard details): only their errors are shown.
const QUIET: &[&str] = &["rfd", "egui_winit"];

/// How serious a message is, as the Console shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Something happened.
    Info,
    /// Something works, but not quite as asked.
    Warning,
    /// Something failed.
    Error,
}

/// The messages written but not yet shown.
#[derive(Debug, Default)]
struct Waiting {
    messages: VecDeque<(Level, String)>,
    /// The last message written, so one repeated every frame is shown
    /// once.
    last: Option<String>,
    /// How many were dropped because nothing took them.
    dropped: usize,
}

/// Where the window takes the messages from.
#[derive(Debug, Clone, Default)]
pub struct Messages(Arc<Mutex<Waiting>>);

impl Messages {
    /// Every message written since last asked, oldest first.
    pub fn take(&self) -> Vec<(Level, String)> {
        let mut waiting = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let mut taken: Vec<_> = waiting.messages.drain(..).collect();
        let dropped = std::mem::take(&mut waiting.dropped);
        if dropped > 0 {
            taken.insert(
                0,
                (
                    Level::Warning,
                    format!("{dropped} older library messages were left out."),
                ),
            );
        }
        taken
    }

    fn push(&self, level: Level, text: String) {
        let mut waiting = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if waiting.last.as_deref() == Some(text.as_str()) {
            return;
        }
        waiting.last = Some(text.clone());
        waiting.messages.push_back((level, text));
        if waiting.messages.len() > WAITING_LIMIT {
            waiting.messages.pop_front();
            waiting.dropped += 1;
        }
    }
}

/// The logger itself.
struct Logger {
    messages: Messages,
}

/// Whether a message from `target` at `level` is shown.
fn wanted(target: &str, level: log::Level) -> bool {
    let krate = target.split("::").next().unwrap_or(target);
    match level {
        log::Level::Error => true,
        log::Level::Warn => !QUIET.contains(&krate),
        log::Level::Info => OWN.contains(&krate),
        log::Level::Debug | log::Level::Trace => false,
    }
}

/// A message as the Console shows it: other libraries' named first.
fn text_of(target: &str, message: &str) -> String {
    let krate = target.split("::").next().unwrap_or(target);
    if OWN.contains(&krate) {
        message.to_string()
    } else {
        format!("{krate}: {message}")
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        wanted(metadata.target(), metadata.level())
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let level = match record.level() {
            log::Level::Error => Level::Error,
            log::Level::Warn => Level::Warning,
            _ => Level::Info,
        };
        let text = text_of(record.target(), &record.args().to_string());
        self.messages.push(level, text);
    }

    fn flush(&self) {}
}

/// Installs the logger, once per run. Returns where to take its messages
/// from, or `None` if another logger was installed first.
pub fn install() -> Option<Messages> {
    let messages = Messages::default();
    let logger = Logger {
        messages: messages.clone(),
    };
    log::set_boxed_logger(Box::new(logger)).ok()?;
    log::set_max_level(log::LevelFilter::Info);
    Some(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Log;

    fn record<'a>(
        target: &'a str,
        level: log::Level,
        args: std::fmt::Arguments<'a>,
    ) -> log::Record<'a> {
        log::Record::builder()
            .target(target)
            .level(level)
            .args(args)
            .build()
    }

    #[test]
    fn warnings_from_anyone_and_information_from_solis_are_kept() {
        let messages = Messages::default();
        let logger = Logger {
            messages: messages.clone(),
        };
        logger.log(&record(
            "wgpu_core::device",
            log::Level::Warn,
            format_args!("slow"),
        ));
        logger.log(&record(
            "wgpu_core::device",
            log::Level::Info,
            format_args!("chatty"),
        ));
        logger.log(&record(
            "naga",
            log::Level::Error,
            format_args!("bad shader"),
        ));
        logger.log(&record(
            "render::pick",
            log::Level::Info,
            format_args!("picked"),
        ));
        logger.log(&record("render", log::Level::Debug, format_args!("detail")));
        // The user test: the file window's and window library's warnings
        // filled the Console with nothing to act on; their errors stay.
        logger.log(&record(
            "rfd::backend::xdg_desktop_portal",
            log::Level::Warn,
            format_args!("using zenity"),
        ));
        logger.log(&record("egui_winit", log::Level::Warn, format_args!("ime")));
        logger.log(&record("rfd", log::Level::Error, format_args!("no window")));
        assert_eq!(
            messages.take(),
            [
                (Level::Warning, "wgpu_core: slow".to_string()),
                (Level::Error, "naga: bad shader".to_string()),
                (Level::Info, "picked".to_string()),
                (Level::Error, "rfd: no window".to_string()),
            ]
        );
        assert!(messages.take().is_empty(), "taken once");
    }

    #[test]
    fn a_message_repeated_every_frame_is_shown_once_and_the_queue_stays_short() {
        let messages = Messages::default();
        for _ in 0..10 {
            messages.push(Level::Warning, "the same".into());
        }
        assert_eq!(messages.take().len(), 1);
        for i in 0..WAITING_LIMIT + 20 {
            messages.push(Level::Info, format!("message {i}"));
        }
        let taken = messages.take();
        assert_eq!(taken.len(), WAITING_LIMIT + 1);
        assert_eq!(taken[0].1, "20 older library messages were left out.");
        assert_eq!(taken[1].1, "message 20");
    }
}
