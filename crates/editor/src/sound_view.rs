//! Sound files in the editor (Phase 7, S1): their waveform on the
//! Library's tiles, and their details in Properties with Play and Stop.
//!
//! The app measures each sound file on a worker and hands the results in
//! ([`crate::Workbench::set_sounds`]); it plays previews and says each
//! frame how far one has got ([`crate::Workbench::set_preview`]). This
//! module only draws them, and asks for playing and stopping through
//! `LibraryRequest`s.

use crate::inspector;
use crate::library::LibraryRequest;
use crate::theme;
use egui::{Color32, RichText, Ui};

/// The colour of sounds: their Library stripe and waveform.
pub const SOUND_COLOR: Color32 = crate::theme::kind::SOUND;

/// What the app found in a sound file.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundInfo {
    /// How long it is, in seconds.
    pub seconds: f64,
    /// How many channels the file has (more than two are mixed to one).
    pub channels: u16,
    /// Samples a second.
    pub sample_rate: u32,
    /// "WAV" or "Ogg Vorbis".
    pub format: &'static str,
    /// Whether it's too long to hold in memory, so it's played from its
    /// file as it plays (in a game, once sounds play there).
    pub streamed: bool,
    /// Its loudness across its length: each column's lowest and highest
    /// sample, from -1 to 1.
    pub waveform: Vec<(f32, f32)>,
}

/// A sound file as far as the app has got with it.
#[derive(Debug, Clone, PartialEq)]
pub enum SoundState {
    /// Being measured.
    Measuring,
    /// Measured.
    Read(SoundInfo),
    /// It couldn't be read: why, in plain words.
    Failed(String),
}

/// The sound previewing, and how far it has got.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// Its file, inside the assets (as the Library names it).
    pub path: String,
    /// How far it has played, in seconds.
    pub seconds: f64,
    /// How long what's playing is, in seconds (for a sound event, the
    /// take it picked).
    pub length: f64,
    /// For a sound event: which take it picked and how it varied it.
    pub detail: Option<String>,
}

/// A sound event file as the app has read it.
#[derive(Debug, Clone, PartialEq)]
pub enum EventState {
    /// Read, with notes about what was put right.
    Read {
        /// Its settings.
        event: sound::SoundEvent,
        /// What was skipped or brought into range.
        notes: Vec<String>,
    },
    /// It couldn't be read: why, in plain words.
    Failed(String),
}

/// `seconds` as "4 ms", "0.42 s", "12.5 s" or "3:05".
pub fn length_text(seconds: f64) -> String {
    if seconds > 0.0 && seconds < 0.1 {
        format!("{} ms", (seconds * 1000.0).round().max(1.0))
    } else if seconds < 10.0 {
        format!("{seconds:.2} s")
    } else if seconds < 60.0 {
        format!("{seconds:.1} s")
    } else {
        let whole = seconds.round() as u64;
        format!("{}:{:02}", whole / 60, whole % 60)
    }
}

/// How the channels read: "Mono", "Stereo", or "6, mixed to mono".
pub fn channels_text(channels: u16) -> String {
    match channels {
        1 => "Mono".to_owned(),
        2 => "Stereo".to_owned(),
        n => format!("{n}, mixed to mono"),
    }
}

/// "44,100 Hz".
pub fn rate_text(rate: u32) -> String {
    let thousands = rate / 1000;
    if thousands == 0 {
        format!("{rate} Hz")
    } else {
        format!("{thousands},{:03} Hz", rate % 1000)
    }
}

/// How far through it the preview is (0 to 1), if it's this sound's.
pub(crate) fn played(path: &str, preview: Option<&Preview>) -> Option<f32> {
    let preview = preview.filter(|p| p.path == path)?;
    Some(if preview.length > 0.0 {
        (preview.seconds / preview.length).clamp(0.0, 1.0) as f32
    } else {
        0.0
    })
}

/// Draws `waveform` across `rect`: a column per pixel (or fewer), the part
/// already played brighter, and a line where the preview is.
pub(crate) fn draw_waveform(
    painter: &egui::Painter,
    rect: egui::Rect,
    waveform: &[(f32, f32)],
    played: Option<f32>,
) {
    let middle = rect.center().y;
    let half = rect.height() * 0.5;
    painter.hline(
        rect.x_range(),
        middle,
        egui::Stroke::new(1.0, SOUND_COLOR.gamma_multiply(0.35)),
    );
    if waveform.is_empty() {
        return;
    }
    let width = rect.width().max(1.0);
    let columns = (width as usize).min(waveform.len() * 4).max(1);
    let step = width / columns as f32;
    let head = played.map(|p| rect.left() + p * width);
    for c in 0..columns {
        let (lo, hi) = waveform[c * waveform.len() / columns];
        let x = rect.left() + (c as f32 + 0.5) * step;
        let top = middle - hi.clamp(-1.0, 1.0) * half;
        let bottom = (middle - lo.clamp(-1.0, 1.0) * half).max(top + 1.0);
        let colour = if head.is_some_and(|h| x <= h) {
            theme::TEXT_STRONG
        } else {
            SOUND_COLOR
        };
        painter.vline(
            x,
            top..=bottom,
            egui::Stroke::new(step.clamp(1.0, 2.0), colour),
        );
    }
    if let Some(x) = head {
        painter.vline(
            x,
            rect.y_range(),
            egui::Stroke::new(1.5, theme::TEXT_STRONG),
        );
    }
}

/// A sound file in Properties: its waveform, Play or Stop, and what it
/// holds. Returns what was asked for.
pub(crate) fn sound_properties(
    ui: &mut Ui,
    path: &str,
    state: Option<&SoundState>,
    preview: Option<&Preview>,
) -> Option<LibraryRequest> {
    let mut request = None;
    let name = path.rsplit('/').next().unwrap_or(path);
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(theme::panel_margin())
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        crate::icons::icon(
                            ui.painter(),
                            egui::Rect::from_min_size(
                                ui.cursor().min + egui::vec2(0.0, 2.0),
                                egui::vec2(14.0, 14.0),
                            ),
                            crate::icons::Icon::Sound,
                            SOUND_COLOR,
                        );
                        ui.add_space(18.0);
                        ui.label(RichText::new(name).strong());
                    });
                    ui.add_space(4.0);
                    match state {
                        None | Some(SoundState::Measuring) => {
                            ui.label(RichText::new("Reading it…").color(theme::TEXT_WEAK));
                        }
                        Some(SoundState::Failed(why)) => {
                            ui.label(
                                RichText::new(format!("Solis couldn't read it: {why}."))
                                    .color(theme::TEXT_WEAK),
                            );
                        }
                        Some(SoundState::Read(info)) => {
                            request = details(ui, path, info, preview);
                        }
                    }
                });
        });
    request
}

/// The waveform, Play or Stop, and the rows of details.
fn details(
    ui: &mut Ui,
    path: &str,
    info: &SoundInfo,
    preview: Option<&Preview>,
) -> Option<LibraryRequest> {
    let mut request = None;
    let played = played(path, preview);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 64.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3.0, theme::FIELD);
    draw_waveform(
        ui.painter(),
        rect.shrink2(egui::vec2(4.0, 4.0)),
        &info.waveform,
        played,
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let playing = played.is_some();
        let (icon, label) = if playing {
            (crate::icons::Icon::Stop, "Stop")
        } else {
            (crate::icons::Icon::Play, "Play")
        };
        let button =
            ui.add(egui::Button::new(format!("     {label}")).min_size(egui::vec2(70.0, 0.0)));
        crate::icons::icon(
            ui.painter(),
            egui::Rect::from_center_size(
                egui::pos2(button.rect.left() + 13.0, button.rect.center().y),
                egui::vec2(11.0, 11.0),
            ),
            icon,
            theme::TEXT_STRONG,
        );
        button.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{label} the sound"))
        });
        let button = button.on_hover_text(if playing {
            "Stop listening"
        } else {
            "Listen to it (only you hear it: the game's volumes don't touch it)"
        });
        if button.clicked() {
            request = Some(if playing {
                LibraryRequest::StopSound
            } else {
                LibraryRequest::PlaySound(path.to_owned())
            });
        }
        let at = preview
            .filter(|p| p.path == path)
            .map_or(0.0, |p| p.seconds.min(info.seconds));
        ui.label(
            RichText::new(format!(
                "{} / {}",
                length_text(at),
                length_text(info.seconds)
            ))
            .color(theme::TEXT_WEAK),
        );
    });
    ui.add_space(6.0);
    inspector::row(ui, "Length", "How long it plays.", |ui| {
        ui.label(length_text(info.seconds));
    });
    inspector::row(
        ui,
        "Channels",
        "Mono is one sound; stereo is two, left and right. More than two are mixed to one.",
        |ui| {
            ui.label(channels_text(info.channels));
        },
    );
    inspector::row(
        ui,
        "Sample rate",
        "How many times a second the sound was measured. 44,100 and 48,000 are usual.",
        |ui| {
            ui.label(rate_text(info.sample_rate));
        },
    );
    inspector::row(ui, "Format", "The kind of file.", |ui| {
        ui.label(info.format);
    });
    inspector::row(
        ui,
        "Kept",
        "Sounds up to ten minutes long are held in memory, ready to play at once; longer ones \
         are played from their file as they play, so they don't fill the memory.",
        |ui| {
            ui.label(if info.streamed {
                "Streamed from its file"
            } else {
                "From memory"
            });
        },
    );
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_channels_and_rates_read_plainly() {
        assert_eq!(length_text(0.4213), "0.42 s");
        assert_eq!(length_text(0.0045), "5 ms");
        assert_eq!(length_text(0.0001), "1 ms");
        assert_eq!(length_text(0.0), "0.00 s");
        assert_eq!(length_text(12.54), "12.5 s");
        assert_eq!(length_text(185.2), "3:05");
        assert_eq!(length_text(3600.0), "60:00");
        assert_eq!(channels_text(1), "Mono");
        assert_eq!(channels_text(2), "Stereo");
        assert_eq!(channels_text(6), "6, mixed to mono");
        assert_eq!(rate_text(44_100), "44,100 Hz");
        assert_eq!(rate_text(8_000), "8,000 Hz");
        assert_eq!(rate_text(192_000), "192,000 Hz");
    }

    #[test]
    fn only_the_sound_previewing_shows_how_far_it_has_played() {
        let preview = |seconds, length| Preview {
            path: "sfx/step.wav".into(),
            seconds,
            length,
            detail: None,
        };
        assert_eq!(played("sfx/step.wav", Some(&preview(0.5, 2.0))), Some(0.25));
        assert_eq!(played("sfx/other.wav", Some(&preview(0.5, 2.0))), None);
        assert_eq!(played("sfx/step.wav", None), None);
        assert_eq!(played("sfx/step.wav", Some(&preview(0.5, 0.25))), Some(1.0));
        assert_eq!(played("sfx/step.wav", Some(&preview(0.5, 0.0))), Some(0.0));
    }

    #[test]
    fn a_waveform_draws_in_any_size_without_panicking() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.painter();
            for (w, h) in [(0.0, 0.0), (1.0, 1.0), (72.0, 40.0), (2000.0, 64.0)] {
                let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, h));
                draw_waveform(painter, rect, &[], None);
                draw_waveform(painter, rect, &[(-1.0, 1.0)], Some(0.5));
                draw_waveform(painter, rect, &[(f32::NAN, 2.0); 3000], Some(1.0));
            }
        });
        output.textures_delta.clear();
    }
}
