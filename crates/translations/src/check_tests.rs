use super::*;
use crate::{Language, TableText, Translations};
use level::languages::Languages;

fn project() -> Translations {
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    Translations::new(
        &languages,
        &[
            (
                "locale/en/shop.ftl".into(),
                "Shop-Welcome = Welcome, { $name }!\nShop-Bye = Bye\nShop-Old = Old\n".into(),
            ),
            (
                "locale/nb/shop.ftl".into(),
                "Shop-Welcome = Velkommen, { $navn }!\nShop-Bye = Ha det\n".into(),
            ),
        ],
        vec![TableText {
            key: "Items.Apple.name".into(),
            file: "items.table.lst".into(),
            text: "Apple".into(),
        }],
    )
}

#[test]
fn missing_lines_are_counted_for_each_language() {
    let mut used = HashSet::new();
    note_uses(
        "World:text(Text.ShopBye); ShopText:welcome(\"Ada\"); Text.ShopOld",
        &project().code_keys(),
        &mut used,
    );
    let problems = check(&project(), &used);
    let missing: Vec<&FileProblem> = problems
        .iter()
        .filter(|p| p.message.contains("missing"))
        .collect();
    assert_eq!(missing.len(), 1, "{problems:?}");
    assert!(
        missing[0]
            .message
            .starts_with("Norsk is missing 2 of 4 lines: Shop.Old, Items.Apple.name."),
        "{}",
        missing[0].message
    );
    assert_eq!(missing[0].file, "locale/nb");
}

#[test]
fn values_that_dont_match_are_said_at_the_translations_line() {
    let problems = check(&project(), &HashSet::new());
    let p = problems
        .iter()
        .find(|p| p.message.contains("values should match"))
        .unwrap();
    assert_eq!((p.file.as_str(), p.line), ("locale/nb/shop.ftl", 1));
    assert!(p.message.contains("uses $navn"), "{}", p.message);
    assert!(p.message.contains("leaves out $name"), "{}", p.message);
}

#[test]
fn keys_nothing_uses_are_said_and_tables_cells_are_always_used() {
    let keys = project().code_keys();
    let mut used = HashSet::new();
    note_uses(
        "fetch \"solis/texts\":ShopText;\nsay(ShopText:welcome(\"Ada\"));",
        &keys,
        &mut used,
    );
    note_uses("solis.World:text(\"Shop.Bye\")", &keys, &mut used);
    assert_eq!(used.len(), 2, "{used:?}");
    let problems = check(&project(), &used);
    let p = problems
        .iter()
        .find(|p| p.message.contains("nothing uses"))
        .unwrap();
    assert!(
        p.message
            .starts_with("1 line of the main language's text nothing uses: Shop.Old."),
        "{}",
        p.message
    );
    assert_eq!((p.file.as_str(), p.line), ("locale/en/shop.ftl", 3));
}

#[test]
fn a_longer_name_isnt_a_use() {
    let mut used = HashSet::new();
    note_uses(
        "Text.ShopByebye; Text.ShopWelcome2",
        &["Shop.Bye".into(), "Shop.Welcome".into()],
        &mut used,
    );
    assert!(used.is_empty(), "{used:?}");
    note_uses(
        "solis.ShopText:welcome(\"Ada\")",
        &["Shop.Welcome".into()],
        &mut used,
    );
    assert!(used.contains("shop.welcome"), "Lua's call");
}

#[test]
fn a_project_with_only_its_main_language_has_nothing_missing() {
    let t = Translations::new(
        &Languages::default(),
        &[("locale/en/a.ftl".into(), "Menu-Start = Start\n".into())],
        Vec::new(),
    );
    let mut used = HashSet::new();
    used.insert("menu.start".to_owned());
    assert!(check(&t, &used).is_empty());
    let _ = Language::english();
}
