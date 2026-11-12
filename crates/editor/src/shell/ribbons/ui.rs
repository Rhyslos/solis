//! The UI tab's ribbon (the approved additions, round 3,
//! `07-ui-editor-a.png`): Screen (the size the canvas shows, the
//! player's text size, the safe area and snapping) and Code (Open as
//! code). The drawing tools are the canvas's toolbox (the project lead's
//! answer 29); the preview values stay over the canvas. There is no align
//! or distribute: the Align grid is in Properties.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, big, item, row_list, switch};
use crate::workbench::Workbench;

/// The text sizes a screen can be tried at, in percent.
pub(crate) const TEXT_SIZES: [u16; 3] = [100, 150, 200];

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let presets = ui::presets::PRESETS;
    let doc = w.open.screen_doc.as_ref();
    let shown = doc
        .and_then(|d| presets.get(d.preset))
        .unwrap_or(&presets[0]);
    let sizes = presets
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let words = format!("{} \u{b7} {} \u{d7} {}", p.name, p.size[0], p.size[1]);
            item(words, C::ScreenSize(i as u8))
        })
        .collect();
    let text = doc.map_or(100, |d| d.text_percent);
    let texts = TEXT_SIZES
        .into_iter()
        .map(|p| item(format!("{p}%"), C::ScreenText(p)))
        .collect();
    vec![
        Group {
            label: Some("Screen"),
            slots: vec![
                row_list(
                    format!(
                        "{} \u{b7} {} \u{d7} {}",
                        shown.name, shown.size[0], shown.size[1]
                    ),
                    sizes,
                ),
                row_list(format!("Text {text}%"), texts),
                switch("Safe area", C::SafeArea),
                switch("Snap", C::ScreenSnap),
            ],
        },
        group!(
            Some("Code"),
            [big(Icon::Code, "Open as code", C::OpenScreenCode)]
        ),
    ]
}
