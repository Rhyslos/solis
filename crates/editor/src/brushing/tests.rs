//! The brush list's and drawing out's words and the shapes' drawn
//! settings.

use super::*;
use draw::{Draw, Nothing, Step};
use list::List;

#[test]
fn drawn_out_shapes_start_with_the_lists_counts() {
    let size = Vec3::new(2.0, 3.0, 1.0);
    assert_eq!(settings_for(Shape::Cylinder, size).sides, 16);
    assert_eq!(settings_for(Shape::Sphere, size).sides, 16);
    assert_eq!(settings_for(Shape::Arch, size).sides, 12);
    // A step for each 25 cm of height, at least one, within the limits.
    assert_eq!(settings_for(Shape::Stairs, size).steps, 4);
    let low = Vec3::new(2.0, 3.0, 0.05);
    assert_eq!(settings_for(Shape::Stairs, low).steps, 1);
    let tall = Vec3::new(2.0, 3.0, 400.0);
    assert_eq!(
        settings_for(Shape::Stairs, tall).steps,
        mesh::shapes::MAX_STEPS
    );
    let odd = Vec3::new(2.0, 3.0, f32::NAN);
    assert_eq!(settings_for(Shape::Stairs, odd).steps, 1);
    assert_eq!(settings_for(Shape::Box, size).size, size);
}

#[test]
fn every_shape_has_its_own_icon() {
    let mut icons: Vec<_> = Shape::ALL.into_iter().map(icon).collect();
    icons.dedup();
    assert_eq!(icons.len(), Shape::ALL.len());
}

#[test]
fn the_status_bar_and_hints_say_what_is_going_on() {
    let mut brushing = Brushing::default();
    assert!(!brushing.is_active());
    assert_eq!(brushing.status_words(0.5), None);
    assert_eq!(brushing.hint(0.5), None);
    let mut list = List::beside(egui::Pos2::ZERO);
    list.next();
    brushing.list = Some(list);
    assert!(brushing.is_active());
    assert_eq!(
        brushing.status_words(0.5).as_deref(),
        Some("Choosing a shape brush: Ramp")
    );
    assert!(
        brushing
            .hint(0.5)
            .unwrap()
            .contains("`B` again to move down it")
    );
    brushing.list = None;
    brushing.draw = Some(Draw::new(Shape::Box));
    assert!(
        brushing
            .status_words(0.5)
            .unwrap()
            .starts_with("Drawing a Box: drag on the ground")
    );
    let hint = brushing.hint(0.5).unwrap();
    assert!(
        hint.contains("Grid · 0.5") && hint.contains("`Ctrl`"),
        "{hint}"
    );
    if let Some(draw) = &mut brushing.draw {
        draw.step = Step::Waiting {
            nothing: Some(Nothing::NoFootprint),
        };
    }
    assert!(
        brushing
            .status_words(0.5)
            .unwrap()
            .starts_with("Nothing made")
    );
    // Esc closes the list first, then gives the drawing up.
    brushing.list = Some(List::beside(egui::Pos2::ZERO));
    assert!(brushing.escape());
    assert!(brushing.list.is_none() && brushing.draw.is_some());
    assert!(brushing.escape());
    assert!(!brushing.is_active());
    assert!(!brushing.escape(), "nothing left to close");
}
