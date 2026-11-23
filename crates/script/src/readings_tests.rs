use super::*;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn names(list: &[&str]) -> Arc<[String]> {
    list.iter().map(|n| (*n).to_string()).collect()
}

#[test]
fn every_core_s_copies_are_gathered_into_one() {
    let kinds = vec!["Knight".to_string(), "Quest".to_string()];
    let made = Readings::gathered(
        &kinds,
        [
            (
                "Knight".into(),
                names(&["coins"]),
                vec![(id(1), vec![Some(Value::Int(3))])],
            ),
            (
                "Knight".into(),
                names(&["coins"]),
                vec![(id(2), vec![None])],
            ),
            // A kind nobody reads isn't kept.
            ("Gate".into(), names(&[]), vec![(id(3), vec![])]),
        ],
    );
    assert!(made.reads("Knight") && made.reads("Quest") && !made.reads("Gate"));
    assert_eq!(made.len(), 2);
    let (fields, row) = made.row("Knight", id(1)).unwrap();
    assert_eq!(
        (fields, row),
        (&["coins".to_string()][..], &[Some(Value::Int(3))][..])
    );
    assert_eq!(made.row("Knight", id(9)), None);
}

#[test]
fn the_is_the_one_entity_or_how_many_there_were() {
    let kinds = vec!["Quest".to_string(), "Knight".to_string()];
    let made = Readings::gathered(
        &kinds,
        [
            ("Quest".into(), names(&[]), vec![(id(4), vec![])]),
            (
                "Knight".into(),
                names(&[]),
                vec![(id(1), vec![]), (id(2), vec![])],
            ),
        ],
    );
    assert_eq!(made.the("Quest"), Ok(id(4)));
    assert_eq!(made.the("Knight"), Err(2));
    assert_eq!(made.the("Shop"), Err(0));
    assert!(not_one("Shop", 0).contains("there's none"));
    assert!(not_one("Knight", 2).contains("there are 2"));
}

#[test]
fn only_what_the_lust_reads_or_names_is_copied() {
    let readable = vec![
        ("Knight".to_string(), "knight".to_string()),
        ("Quest".to_string(), "quest".to_string()),
        ("Gate".to_string(), "gate".to_string()),
    ];
    let files = vec![
        (
            "main.lst".to_string(),
            "World:the(Component.Quest)".to_string(),
        ),
        // The engine's own files name every component; they don't count.
        (
            "solis/components.lst".to_string(),
            "Component.Gate".to_string(),
        ),
    ];
    let kinds = kinds_read(Some("local k = _E_knight(e)"), &files, &readable);
    assert_eq!(kinds, ["Knight", "Quest"]);
    assert!(kinds_read(None, &[], &readable).is_empty());
}
