use super::*;
use level::languages::Languages;

fn languages(codes: &[&str]) -> Languages {
    let mut l = Languages::default();
    for code in codes {
        if !l.has(code) {
            l.add(code).unwrap();
        }
    }
    l.set_main(codes[0]);
    l
}

fn file(path: &str, text: &str) -> (String, String) {
    (path.to_owned(), text.to_owned())
}

fn lang(code: &str) -> Language {
    Language::new(code).unwrap()
}

fn apples() -> Translations {
    Translations::new(
        &languages(&["en", "fr", "pl", "nb"]),
        &[
            file(
                "locale/en/shop.ftl",
                "Shop-Apples = { $count ->\n    [one] { $count } apple\n   *[other] { $count } apples\n}\nShop-Welcome = Welcome, { $name }!\nShop-Bye = Bye\n",
            ),
            file(
                "locale/fr/shop.ftl",
                "Shop-Apples = { $count ->\n    [one] { $count } pomme\n   *[other] { $count } pommes\n}\n",
            ),
            file(
                "locale/pl/shop.ftl",
                "Shop-Apples = { $count ->\n    [one] { $count } jabłko\n    [few] { $count } jabłka\n    [many] { $count } jabłek\n   *[other] { $count } jabłka\n}\n",
            ),
            file(
                "locale/nb/shop.ftl",
                "Shop-Welcome = Velkommen, { $name }!\n",
            ),
        ],
        vec![TableText {
            key: "Items.IronSword.description".into(),
            file: "data/items.table.lst".into(),
            text: "A plain blade.".into(),
        }],
    )
}

fn count(t: &Translations, code: &str, n: f64) -> String {
    t.text(
        lang(code),
        "Shop.Apples",
        &[("count", TextValue::Number(n))],
    )
}

#[test]
fn plurals_follow_each_languages_own_rules() {
    let t = apples();
    assert_eq!(count(&t, "en", 1.0), "1 apple");
    assert_eq!(count(&t, "en", 0.0), "0 apples");
    assert_eq!(count(&t, "en", 2.0), "2 apples");
    // French: 0 and 1 are both "one".
    assert_eq!(count(&t, "fr", 0.0), "0 pomme");
    assert_eq!(count(&t, "fr", 1.0), "1 pomme");
    assert_eq!(count(&t, "fr", 2.0), "2 pommes");
    // Polish: one, few (2–4, 22–24…), many (5–21, 25…).
    assert_eq!(count(&t, "pl", 1.0), "1 jabłko");
    assert_eq!(count(&t, "pl", 3.0), "3 jabłka");
    assert_eq!(count(&t, "pl", 5.0), "5 jabłek");
    assert_eq!(count(&t, "pl", 22.0), "22 jabłka");
    assert_eq!(count(&t, "pl", 25.0), "25 jabłek");
    assert_eq!(count(&t, "pl", 1.5), "1.5 jabłka");
}

#[test]
fn values_are_put_in_as_text() {
    let t = apples();
    let name = [("name", TextValue::Text("Ada".into()))];
    assert_eq!(t.text(lang("en"), "Shop.Welcome", &name), "Welcome, Ada!");
    assert_eq!(
        t.text(lang("nb"), "shop.welcome", &name),
        "Velkommen, Ada!",
        "any capitals"
    );
    assert_eq!(
        t.text(lang("en"), "Shop.Welcome", &[]),
        "Welcome, {$name}!",
        "a value not given shows as written"
    );
}

#[test]
fn the_main_language_is_the_fallback_then_the_key() {
    let t = apples();
    assert_eq!(
        t.text(lang("nb"), "Shop.Bye", &[]),
        "Bye",
        "Norsk has no Bye"
    );
    assert_eq!(
        t.text(lang("fr"), "Items.IronSword.description", &[]),
        "A plain blade."
    );
    assert_eq!(
        t.text(lang("de"), "Shop.Bye", &[]),
        "Bye",
        "not a language of the project"
    );
    assert_eq!(t.text(lang("en"), "Shop.Nothing", &[]), "Shop.Nothing");
}

#[test]
fn a_game_works_with_no_translation_files_at_all() {
    let t = Translations::default();
    assert_eq!(t.main(), Language::english());
    assert!(t.keys().is_empty());
    assert_eq!(t.text(lang("en"), "Menu.Start", &[]), "Menu.Start");
}

#[test]
fn missing_lines_are_listed_per_language_and_counted() {
    let t = apples();
    assert_eq!(
        t.keys(),
        [
            "Shop.Apples",
            "Shop.Welcome",
            "Shop.Bye",
            "Items.IronSword.description"
        ]
    );
    assert_eq!(
        t.missing(lang("nb")),
        ["Shop.Apples", "Shop.Bye", "Items.IronSword.description"]
    );
    assert_eq!(t.count(lang("nb")), 1);
    assert_eq!(t.count(lang("en")), 4);
    assert!(t.missing(lang("en")).is_empty());
    assert_eq!(
        t.code_keys(),
        ["Shop.Apples", "Shop.Welcome", "Shop.Bye"],
        "tables' cells aren't Text options"
    );
}

#[test]
fn files_of_other_languages_and_twice_written_keys_are_said() {
    let t = Translations::new(
        &languages(&["en"]),
        &[
            file("locale/en/a.ftl", "Shop-A = one\n"),
            file("locale/en/b.ftl", "Shop-A = two\n"),
            file("locale/de/a.ftl", "Shop-A = eins\n"),
            file("notes/readme.txt", "x"),
        ],
        Vec::new(),
    );
    assert_eq!(t.text(lang("en"), "Shop.A", &[]), "one");
    let said: Vec<&str> = t.problems().iter().map(|p| p.message.as_str()).collect();
    assert!(
        said.iter()
            .any(|m| m.contains("written in locale/en/a.ftl too")),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|m| m.contains("de isn't one of the project's languages")),
        "{said:?}"
    );
}

#[test]
fn texts_follow_the_players_language() {
    let all = Arc::new(apples());
    let name = [("name", TextValue::Text("Ada".into()))];
    let en = Texts::new(Arc::clone(&all), lang("en"));
    let nb = Texts::new(Arc::clone(&all), lang("nb-NO"));
    assert_eq!(nb.language(), lang("nb"), "nb-NO finds nb");
    assert_eq!(en.text_with("Shop.Welcome", &name), "Welcome, Ada!");
    assert_eq!(nb.text_with("Shop.Welcome", &name), "Velkommen, Ada!");
    assert_eq!(
        Texts::new(all, lang("ja")).language(),
        lang("en"),
        "not the project's: the main one"
    );
    assert!(!en.same(&nb));
}

#[test]
fn a_new_lines_file_is_named_after_its_group() {
    let t = apples();
    assert_eq!(t.file_for(lang("fr"), "Shop.Apples"), "locale/fr/shop.ftl");
    assert_eq!(t.file_for(lang("de"), "Menu.Start"), "locale/de/menu.ftl");
}
