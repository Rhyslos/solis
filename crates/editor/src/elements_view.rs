//! The Elements panel (decision 0089): the open screen's pieces as a tree
//! in drawing order, like the Area panel. Click to pick (Shift or Ctrl
//! for more), drag a row onto another to move it there (into a container
//! when dropped on its middle), the eye hides a piece while editing and
//! the lock stops it being picked on the canvas. Code in the view is
//! shown greyed, to edit as code.

use crate::icons::{self, Icon};
use crate::screen_doc::ScreenDoc;
use crate::theme;
use egui::{Color32, RichText, Sense, Ui, Vec2};
use ui::source::{Block, Piece};

/// What the Elements panel asks of the workbench.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct ElementsOut {
    /// The screen changed: save it.
    pub(crate) changed: bool,
    /// The pointer is over the panel (Delete and Ctrl+Z are its).
    pub(crate) hovered: bool,
}

/// A row's label: its kind and its words, or a hint of what it is.
pub(crate) fn label(text: &str, piece: &ui::source::ElementSource) -> String {
    match &piece.arg {
        Some(a) => match &a.words {
            Some(w) => format!(
                "{} \"{}\"",
                piece.kind,
                w.chars().take(40).collect::<String>()
            ),
            None => format!(
                "{} {}",
                piece.kind,
                text[a.span.clone()].chars().take(30).collect::<String>()
            ),
        },
        None => piece.kind.clone(),
    }
}

/// A short summary of its look, on the right: `fill · centre`, `gap 14`.
fn summary(text: &str, piece: &ui::source::ElementSource) -> String {
    let mut bits = Vec::new();
    if let Some(w) = piece.style_text(text, "width") {
        bits.push(if w.is_empty() {
            "fill".into()
        } else {
            format!("w {w}")
        });
    }
    if piece.has("center") || piece.has("middle") {
        bits.push("centre".to_string());
    }
    if let Some(g) = piece.style_text(text, "gap") {
        bits.push(format!("gap {g}"));
    }
    bits.join(" · ")
}

/// The Elements panel.
pub(crate) fn elements_tab(ui: &mut Ui, doc: &mut ScreenDoc, accent: Color32) -> ElementsOut {
    let mut out = ElementsOut {
        hovered: ui.rect_contains_pointer(ui.max_rect()),
        ..Default::default()
    };
    let Some(block) = doc.block().cloned() else {
        let why = doc.source.as_ref().err().cloned().unwrap_or_default();
        crate::panels::placeholder(
            ui,
            &format!("The canvas can't follow this file ({why}): edit it as code."),
        );
        return out;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(&doc.screen).strong());
        ui.label(RichText::new("screen").color(theme::text_weak()));
    });
    let mut moves: Option<(Vec<usize>, Vec<usize>, usize)> = None;
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            rows(ui, doc, &block, &mut Vec::new(), 0, accent, &mut moves);
        });
    if let Some((from, into, index)) = moves {
        doc.picked = vec![from];
        out.changed |= doc.move_picked(&into, index);
    }
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Drag to reorder or move into another. The eye hides a piece while editing; the \
             lock stops it being picked on the canvas.",
        )
        .color(theme::text_weak())
        .small(),
    );
    if out.hovered && !ui.ctx().egui_wants_keyboard_input() {
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) && !doc.picked.is_empty() {
            out.changed |= doc.delete_picked();
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::D))
            && !doc.picked.is_empty()
        {
            out.changed |= doc.duplicate_picked();
        }
    }
    out
}

/// The rows for `block`'s pieces, indented by `depth`.
fn rows(
    ui: &mut Ui,
    doc: &mut ScreenDoc,
    block: &Block,
    path: &mut Vec<usize>,
    depth: usize,
    accent: Color32,
    moves: &mut Option<(Vec<usize>, Vec<usize>, usize)>,
) {
    for (i, piece) in block.pieces.iter().enumerate() {
        path.push(i);
        match piece {
            Piece::Code(c) => {
                let first = doc.text[c.span.clone()]
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 14.0 + 18.0);
                    ui.label(
                        RichText::new(format!(
                            "‹code› {}",
                            first.chars().take(36).collect::<String>()
                        ))
                        .color(theme::text_weak())
                        .monospace(),
                    )
                    .on_hover_text(format!("Line {}: edited as code", c.line));
                });
            }
            Piece::Element(e) => {
                let picked = doc.picked.contains(path);
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 22.0),
                    Sense::click_and_drag(),
                );
                let named = label(&doc.text, e);
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        picked,
                        &named,
                    )
                });
                let painter = ui.painter_at(rect);
                if picked {
                    painter.rect_filled(rect, 2.0, accent);
                } else if response.hovered() {
                    painter.rect_filled(rect, 2.0, theme::hover());
                }
                let x = rect.left() + depth as f32 * 14.0 + 6.0;
                let ink = if picked {
                    Color32::WHITE
                } else {
                    theme::text()
                };
                let icon = if e.children.is_some() {
                    Icon::Layers
                } else {
                    Icon::Properties
                };
                icons::icon(
                    &painter,
                    egui::Rect::from_center_size(
                        egui::pos2(x + 6.0, rect.center().y),
                        Vec2::splat(12.0),
                    ),
                    icon,
                    ink,
                );
                painter.text(
                    egui::pos2(x + 18.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    label(&doc.text, e),
                    egui::FontId::proportional(13.0),
                    ink,
                );
                let sum = summary(&doc.text, e);
                // Eye and lock, then the summary.
                let eye = egui::Rect::from_center_size(
                    egui::pos2(rect.right() - 34.0, rect.center().y),
                    Vec2::splat(16.0),
                );
                let lock = egui::Rect::from_center_size(
                    egui::pos2(rect.right() - 14.0, rect.center().y),
                    Vec2::splat(16.0),
                );
                let hidden = doc.hidden.contains(path);
                let locked = doc.locked.contains(path);
                if hidden || response.hovered() || picked {
                    let c = if hidden { ink } else { ink.gamma_multiply(0.5) };
                    icons::icon(&painter, eye.shrink(2.0), Icon::Hidden, c);
                }
                if locked || response.hovered() || picked {
                    let c = if locked { ink } else { ink.gamma_multiply(0.5) };
                    icons::icon(&painter, lock.shrink(2.0), Icon::Lock, c);
                }
                if !sum.is_empty() {
                    painter.text(
                        egui::pos2(rect.right() - 50.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        sum,
                        egui::FontId::proportional(11.0),
                        if picked {
                            Color32::WHITE
                        } else {
                            theme::text_weak()
                        },
                    );
                }
                if response.clicked() {
                    let at = response.interact_pointer_pos().unwrap_or_default();
                    if eye.contains(at) {
                        if !doc.hidden.remove(path) {
                            doc.hidden.insert(path.clone());
                        }
                    } else if lock.contains(at) {
                        if !doc.locked.remove(path) {
                            doc.locked.insert(path.clone());
                        }
                    } else if ui.input(|i| i.modifiers.shift || i.modifiers.command) {
                        if let Some(k) = doc.picked.iter().position(|p| p == path) {
                            doc.picked.remove(k);
                        } else {
                            doc.picked.push(path.clone());
                        }
                    } else {
                        doc.picked = vec![path.clone()];
                    }
                }
                let response = response.on_hover_text(format!("Line {}", e.line));
                // Dragging a row onto another.
                if response.dragged() {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("elements drag"), path.clone()));
                }
                let dragging: Option<Vec<usize>> =
                    ui.data(|d| d.get_temp(egui::Id::new("elements drag")));
                if let Some(from) = &dragging
                    && from != path
                    && let Some(p) = ui.input(|i| i.pointer.interact_pos())
                    && rect.contains(p)
                {
                    let into_middle = e.children.is_some()
                        && (p.y - rect.top()) > rect.height() * 0.3
                        && (p.y - rect.top()) < rect.height() * 0.7;
                    let line_y = if into_middle {
                        None
                    } else if p.y < rect.center().y {
                        Some(rect.top())
                    } else {
                        Some(rect.bottom())
                    };
                    match line_y {
                        Some(y) => {
                            painter.hline(rect.x_range(), y, egui::Stroke::new(2.0, accent));
                        }
                        None => {
                            painter.rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0, accent),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                    if ui.input(|i| i.pointer.any_released()) {
                        let (parent, own) = path.split_at(path.len() - 1);
                        let target = if into_middle {
                            let len = e.children.as_ref().map_or(0, |c| c.pieces.len());
                            (path.clone(), len)
                        } else if p.y < rect.center().y {
                            (parent.to_vec(), own[0])
                        } else {
                            (parent.to_vec(), own[0] + 1)
                        };
                        *moves = Some((from.clone(), target.0, target.1));
                        ui.data_mut(|d| d.remove::<Vec<usize>>(egui::Id::new("elements drag")));
                    }
                }
                if let Some(children) = &e.children {
                    rows(ui, doc, children, path, depth + 1, accent, moves);
                }
            }
        }
        path.pop();
    }
    if depth == 0 && ui.input(|i| i.pointer.any_released()) {
        ui.data_mut(|d| d.remove::<Vec<usize>>(egui::Id::new("elements drag")));
    }
}

#[cfg(test)]
#[path = "elements_view_tests.rs"]
mod tests;
