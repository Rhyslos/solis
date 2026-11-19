use super::*;
use level::table::{Cell, Row};

fn items() -> Arc<Table> {
    let mut table = Table::new("Item");
    table.columns = vec![
        Column::new("price", ColumnKind::Whole),
        Column::new(
            "rarity",
            ColumnKind::Choice(vec!["Common".into(), "Rare".into()]),
        ),
        Column::new("icon", ColumnKind::Name(NameKind::Picture)),
        Column::new("pickUp", ColumnKind::Name(NameKind::Sound)),
        Column::new("tint", ColumnKind::Colour),
    ];
    table.rows = ["Apple", "IronSword"]
        .into_iter()
        .map(|id| {
            Arc::new(Row {
                id: id.into(),
                cells: table.columns.iter().map(|c| c.kind.empty()).collect(),
            })
        })
        .collect();
    let _ = Cell::Whole(0);
    Arc::new(table)
}

fn first(kind: NameKind) -> String {
    match kind {
        NameKind::Picture => "Apple".into(),
        _ => NO_OPTION.into(),
    }
}

#[test]
fn a_table_gets_a_state_of_its_ids_a_row_class_and_an_object() {
    let names = TableNames {
        path: "data/items".into(),
        table: items(),
    };
    let (file, notes) = table_file("Test 1.0", "solis", &names, &first);
    assert!(notes.is_empty());
    let (path, text) = file.unwrap();
    assert_eq!(path, "solis/tables/data/items.lst");
    for line in [
        "fetch \"solis/project\":[Sound, Picture];",
        "state Item = [Apple, IronSword];",
        "state ItemRarity = [Common, Rare];",
        "class ItemRow {",
        "        lock Item id = Item.Apple;",
        "        lock int price = 0;",
        "        lock ItemRarity rarity = ItemRarity.Common;",
        "        lock Picture icon = Picture.Apple;",
        "        lock Sound pickUp = Sound.None;",
        "        lock colour tint = [1, 1, 1];",
        "object Items {",
        "    method ItemRow row(Item which) native;",
        "    method ItemRow array rows() native;",
        "pass Item, ItemRarity, ItemRow, Items;",
    ] {
        assert!(text.contains(line), "{line}\n{text}");
    }
}

#[test]
fn an_empty_table_still_checks() {
    let names = TableNames {
        path: "crops".into(),
        table: Arc::new(Table::new("Crop")),
    };
    let (path, text) = table_file("Test 1.0", "solis", &names, &first).0.unwrap();
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push((path, text.clone()));
    files.push((
        "main.lst".into(),
        "fetch \"solis/tables/crops\":[Crops, Crop, CropRow];\nend\n".into(),
    ));
    assert!(text.contains("state Crop = [None];"));
    lust::check_with(
        &lust::Engine {
            folders: crate::describe::engine_folders(),
        },
        "main.lst",
        &files,
    )
    .unwrap_or_else(|p| panic!("{}", p[0]));
}

#[test]
fn tables_whose_names_clash_are_left_out_with_a_note() {
    let list = [
        TableNames {
            path: "items".into(),
            table: items(),
        },
        TableNames {
            path: "old/items".into(),
            table: items(),
        },
        TableNames {
            path: "world".into(),
            table: Arc::new(Table::new("Place")),
        },
        TableNames {
            path: "3d".into(),
            table: Arc::new(Table::new("Thing")),
        },
    ];
    let (files, notes) = table_files("Test 1.0", "solis", &list, &["World"], &first);
    assert_eq!(files.len(), 1);
    assert_eq!(notes.len(), 3, "{notes:?}");
    assert!(notes[0].contains("old/items"));
    assert!(notes[1].contains("the engine's World"));
    assert!(notes[2].contains("3d"));
}
