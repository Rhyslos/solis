use super::*;
use crate::{TableText, Translations};
use level::languages::Languages;

fn project(nb: &str) -> Translations {
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    Translations::new(
        &languages,
        &[
            ("locale/en/shop.ftl".into(), "Shop-Welcome = Say \"hi\", { $name }\nShop-Lines =\n    First line\n    second line\n".into()),
            ("locale/nb/shop.ftl".into(), nb.into()),
        ],
        vec![TableText {
            key: "Items.Apple.name".into(),
            file: "items.table.lst".into(),
            text: "Apple, red".into(),
        }],
    )
}

#[test]
fn export_quotes_every_field_doubles_quotes_and_keeps_line_breaks() {
    let csv = export(&project("Shop-Welcome = Si \"hei\", { $name }\n"));
    assert_eq!(
        csv,
        "\"key\",\"en\",\"nb\"\r\n\
         \"Shop.Welcome\",\"Say \"\"hi\"\", { $name }\",\"Si \"\"hei\"\", { $name }\"\r\n\
         \"Shop.Lines\",\"First line\nsecond line\",\"\"\r\n\
         \"Items.Apple.name\",\"Apple, red\",\"\"\r\n"
    );
}

#[test]
fn a_spreadsheet_round_trips_into_the_files() {
    let before = project("");
    let mut csv = export(&before);
    csv = csv.replace(
        "\"Shop.Lines\",\"First line\nsecond line\",\"\"",
        "\"Shop.Lines\",\"First line\nsecond line\",\"Første linje\nandre linje\"",
    );
    csv = csv.replace(
        "\"Items.Apple.name\",\"Apple, red\",\"\"",
        "\"Items.Apple.name\",\"Changed\",\"Eple, rødt\"",
    );
    let imported = import(&csv, &before).unwrap();
    assert!(imported.notes.is_empty(), "{:?}", imported.notes);
    let nb = crate::Language::new("nb").unwrap();
    assert_eq!(
        imported.changes,
        [
            Change {
                language: nb,
                key: "Shop.Lines".into(),
                text: "Første linje\nandre linje".into()
            },
            Change {
                language: nb,
                key: "Items.Apple.name".into(),
                text: "Eple, rødt".into()
            },
        ],
        "the main text of a table's cell is the table's"
    );
    let (files, notes) = crate::edit::apply(&imported.changes, &before, |_| String::new());
    assert!(notes.is_empty());
    assert_eq!(files.len(), 2, "{files:?}");
    let after = Translations::new(
        &{
            let mut l = Languages::default();
            l.add("nb").unwrap();
            l
        },
        &[
            ("locale/en/shop.ftl".into(), "Shop-Welcome = Say \"hi\", { $name }\nShop-Lines =\n    First line\n    second line\n".into()),
            files[0].clone(),
            files[1].clone(),
        ],
        Vec::new(),
    );
    assert_eq!(
        after.text(nb, "Shop.Lines", &[]),
        "Første linje\nandre linje"
    );
    assert_eq!(after.text(nb, "Items.Apple.name", &[]), "Eple, rødt");
    assert!(
        import(&export(&project("")), &project(""))
            .unwrap()
            .changes
            .is_empty(),
        "nothing changed"
    );
}

#[test]
fn unknown_columns_and_keys_are_passed_over_with_notes() {
    let csv =
        "keys,en,de\r\nShop.Welcome,\"Say \"\"hi\"\", { $name }\",Hallo\r\nShop.New,New,Neu\r\n";
    let imported = import(csv, &project("")).unwrap();
    assert!(imported.changes.is_empty(), "{:?}", imported.changes);
    assert_eq!(imported.notes.len(), 2, "{:?}", imported.notes);
    assert!(imported.notes[0].contains("de isn't one"));
    assert!(imported.notes[1].starts_with("1 line has keys"));
}

#[test]
fn a_spreadsheet_without_keys_is_refused() {
    assert!(import("", &project("")).is_err());
    assert!(
        import("en,nb\r\na,b\r\n", &project(""))
            .unwrap_err()
            .contains("no key column")
    );
}
