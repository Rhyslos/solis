//! Runs the architecture checks on the real workspace.
//! If this test fails, its message says which crate broke which rule.

use std::path::Path;

fn workspace_root() -> &'static Path {
    // tools/layer-check -> workspace root
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn workspace_follows_architecture_rules() {
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    let violations = layer_check::check(&crates);
    let report: Vec<String> = violations.iter().map(ToString::to_string).collect();
    assert!(
        violations.is_empty(),
        "architecture rules broken:\n{}",
        report.join("\n")
    );
}

#[test]
fn every_planned_crate_exists() {
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    let names: Vec<&str> = crates.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["app", "area", "assets", "editor", "reflect", "render"]
    );
}

#[test]
fn every_crate_is_registered_in_the_workspace() {
    // Each crate in crates/ must be listed (with its path) in the root
    // Cargo.toml's [workspace.dependencies], except the app, which nothing
    // depends on.
    let root = std::fs::read_to_string(workspace_root().join("Cargo.toml")).unwrap();
    let internal = layer_check::internal_crate_names(&root).unwrap();
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    for c in crates.iter().filter(|c| c.name != "app") {
        assert!(
            internal.contains(&c.name),
            "{} is missing from [workspace.dependencies] in the root Cargo.toml",
            c.name
        );
    }
}

#[test]
fn only_the_app_is_in_the_top_layer() {
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    let top: Vec<&str> = crates
        .iter()
        .filter(|c| c.layer == Some(layer_check::MAX_LAYER))
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(top, ["app"]);
}

#[test]
fn the_engine_never_depends_on_the_editor() {
    // Games ship without the editor, so nothing in the engine layer may
    // reach up into it.
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    for c in crates.iter().filter(|c| c.layer < Some(2)) {
        assert!(
            !c.internal_deps.iter().any(|d| d == "editor"),
            "{} is part of the engine but depends on the editor",
            c.name
        );
    }
}
