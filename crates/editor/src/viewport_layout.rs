//! How the viewport is split into views, each with its own camera and
//! look while another is in use, and how the layout and views are saved.

use super::{VIEWS, Viewport};
use crate::camera::Camera;
use crate::look::Look;
use egui::{Rect, vec2};
use serde::{Deserialize, Serialize};

/// How the viewport is split into views. Saved per computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ViewLayout {
    /// One view.
    #[default]
    One,
    /// Two views side by side.
    SideBySide,
    /// Two views, one above the other.
    Stacked,
    /// Four views, two by two.
    Four,
}

impl ViewLayout {
    /// Every layout, in the menu's order.
    pub const ALL: [Self; 4] = [Self::One, Self::SideBySide, Self::Stacked, Self::Four];

    /// Its name in the menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::One => "One view",
            Self::SideBySide => "Two side by side",
            Self::Stacked => "Two, one above the other",
            Self::Four => "Four views",
        }
    }

    /// Each view shown in `rect`, by number, with where it goes. A thin
    /// gap separates them.
    pub fn rects(self, rect: Rect) -> Vec<(usize, Rect)> {
        let gap = 2.0;
        let (w, h) = (rect.width(), rect.height());
        let cell = |x0: f32, y0: f32, x1: f32, y1: f32| {
            Rect::from_min_max(rect.min + vec2(x0, y0), rect.min + vec2(x1, y1))
        };
        let (mx, my) = ((w - gap) / 2.0, (h - gap) / 2.0);
        match self {
            Self::One => vec![(0, rect)],
            Self::SideBySide => vec![(0, cell(0.0, 0.0, mx, h)), (1, cell(mx + gap, 0.0, w, h))],
            Self::Stacked => vec![(0, cell(0.0, 0.0, w, my)), (1, cell(0.0, my + gap, w, h))],
            Self::Four => vec![
                (0, cell(0.0, 0.0, mx, my)),
                (1, cell(mx + gap, 0.0, w, my)),
                (2, cell(0.0, my + gap, mx, h)),
                (3, cell(mx + gap, my + gap, w, h)),
            ],
        }
    }
}

/// One view's camera and look, as saved.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewSettings {
    /// Its camera (and whether it's a flat view).
    pub camera: Camera,
    /// How it shades models.
    pub look: Look,
}

/// Every view's settings and the layout, as saved per computer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Views {
    /// How the viewport is split.
    pub layout: ViewLayout,
    /// Each view's camera and look, first to last.
    pub views: Vec<ViewSettings>,
    /// The view shown alone, if one is.
    pub maximized: Option<usize>,
}

impl Viewport {
    /// Swaps view `i`'s own state with the fields in use.
    pub(super) fn exchange(&mut self, i: usize) {
        let pane = &mut self.panes[i];
        std::mem::swap(&mut self.camera, &mut pane.camera);
        std::mem::swap(&mut self.look, &mut pane.look);
        std::mem::swap(&mut self.tools, &mut pane.tools);
        std::mem::swap(&mut self.rect, &mut pane.rect);
        std::mem::swap(&mut self.covered, &mut pane.covered);
        std::mem::swap(&mut self.flying_from, &mut pane.flying_from);
        std::mem::swap(&mut self.wheel_carry, &mut pane.wheel_carry);
    }

    /// Makes view `i` the one in use: its camera, look and tools move
    /// into the fields the editor reads.
    pub(super) fn activate(&mut self, i: usize) {
        if i == self.active || i >= VIEWS {
            return;
        }
        self.exchange(self.active);
        self.exchange(i);
        self.active = i;
    }

    /// Which view is in use (the last one clicked).
    pub fn active(&self) -> usize {
        self.active
    }

    /// View `i`'s camera and look.
    pub fn view_settings(&self, i: usize) -> Option<ViewSettings> {
        if i == self.active {
            return Some(ViewSettings {
                camera: self.camera,
                look: self.look,
            });
        }
        self.panes.get(i).map(|p| ViewSettings {
            camera: p.camera,
            look: p.look,
        })
    }

    /// The layout and every view's settings, for saving.
    pub fn views(&self) -> Views {
        Views {
            layout: self.layout,
            views: (0..VIEWS).filter_map(|i| self.view_settings(i)).collect(),
            maximized: self.maximized,
        }
    }

    /// Uses `views` (as saved last time), with anything unusable put
    /// back. View 0 is in use afterwards.
    pub fn set_views(&mut self, views: &Views) {
        self.activate(0);
        self.layout = views.layout;
        self.maximized = views.maximized.filter(|&i| i < VIEWS);
        for (i, saved) in views.views.iter().take(VIEWS).enumerate() {
            let camera = saved.camera.sanitized();
            let look = saved.look.sanitized();
            if i == 0 {
                self.camera = camera;
                self.look = look;
            } else {
                self.panes[i].camera = camera;
                self.panes[i].look = look;
            }
        }
    }

    /// The views shown in `rect`, by number, with where each goes.
    pub fn shown(&self, rect: Rect) -> Vec<(usize, Rect)> {
        match self.maximized {
            Some(i) if self.layout != ViewLayout::One => vec![(i, rect)],
            _ => self.layout.rects(rect),
        }
    }
}
