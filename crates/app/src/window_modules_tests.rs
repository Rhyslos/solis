use super::*;

#[test]
fn copies_are_counted_in_each_level_by_either_word() {
    let root = reflect::test_support::TempFolder::new("count-copies").unwrap();
    let id: AssetId = "5KD8V0Q2MX7TA".parse().unwrap();
    let other = AssetId::random();
    let block = |word: &str, module: AssetId| {
        format!(
            "entity {} {{\n    {word} = \"{module}\";\n}}\n",
            reflect::EntityId::random()
        )
    };
    let meadow = format!(
        "level version 5;\n{}{}{}",
        block("module", id),
        block("module", id),
        block("module", other)
    );
    let old = format!("level version 4;\n{}", block("assembly", id));
    std::fs::write(root.join("meadow.sollevel"), meadow).unwrap();
    std::fs::write(root.join("old.sollevel"), old).unwrap();
    std::fs::write(root.join("none.sollevel"), "level version 5;\n").unwrap();
    let levels: Vec<PathBuf> = [
        "meadow.sollevel",
        "old.sollevel",
        "none.sollevel",
        "gone.sollevel",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    let placed = count_in(&root, &levels, id);
    assert_eq!(
        placed.levels,
        [
            ("meadow.sollevel".to_string(), 2),
            ("old.sollevel".to_string(), 1)
        ]
    );
}
