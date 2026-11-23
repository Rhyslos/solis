//! Data tables read by game code: a row by its ID and every row in order,
//! in Lust, Lua and Rust; a misspelt row or column failing before the game
//! runs; a new table taken in while it runs; and how quick a lookup is.

use super::*;
use crate::code::Compiled;
use crate::table_code::{TableNames, table_file};
use crate::{Host, Runtime};
use level::table::{Column, ColumnKind, Row};
use reflect::{ComponentData, EntityId, NameKind, Value};

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// Items with a price, a rarity, a picture, a colour and a name.
pub(crate) fn items(price_of_sword: i64) -> Arc<Table> {
    let mut table = Table::new("Item");
    table.columns = vec![
        Column::new("name", ColumnKind::Text),
        Column::new("price", ColumnKind::Whole),
        Column::new("weight", ColumnKind::Number),
        Column::new("stacks", ColumnKind::YesNo),
        Column::new(
            "rarity",
            ColumnKind::Choice(vec!["Common".into(), "Rare".into()]),
        ),
        Column::new("icon", ColumnKind::Name(NameKind::Picture)),
        Column::new("tint", ColumnKind::Colour),
    ];
    let row = |id: &str, name: &str, price, rare: bool| {
        Arc::new(Row {
            id: id.into(),
            cells: vec![
                Cell::Text(name.into()),
                Cell::Whole(price),
                Cell::Number(1.5),
                Cell::YesNo(!rare),
                Cell::Option(if rare { "Rare" } else { "Common" }.into()),
                Cell::Option("Apple".into()),
                Cell::Colour([1.0, 0.5, 0.0, 1.0]),
            ],
        })
    };
    table.rows = vec![
        row("Apple", "Apple", 5, false),
        row("IronSword", "Iron sword", price_of_sword, true),
        row("Potion", "Potion", 50, false),
    ];
    Arc::new(table)
}

/// The tables the game is given: the items, at `data/items`.
fn tables(price_of_sword: i64) -> Tables {
    Tables::new(&[("data/items".into(), items(price_of_sword))], &["World"])
}

/// The items' own Lust file.
fn items_file() -> (String, String) {
    let names = TableNames {
        path: "data/items".into(),
        table: items(120),
    };
    table_file("Test 1.0", "solis", &names, &|_| "Apple".into())
        .0
        .unwrap()
}

/// `main` checked with the engine's files and the items' own file.
fn checked(main: &str) -> Result<lust::LuaChunk, Vec<lust::Problem>> {
    let names = crate::project_names::Names {
        pictures: vec!["apple".into(), "pear".into()],
        ..Default::default()
    };
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names);
    files.push(items_file());
    files.push(("main.lst".into(), main.to_string()));
    lust::to_lua_with(
        &lust::Engine {
            folders: crate::describe::engine_folders(),
        },
        "main.lst",
        &files,
    )
}

fn compiled(main: &str, lua: &[(&str, &str)]) -> Compiled {
    let lust = (!main.is_empty())
        .then(|| checked(main).unwrap_or_else(|problems| panic!("{}", problems[0])));
    Compiled {
        timers: Vec::new(),
        state_events: Default::default(),
        components: Default::default(),
        lust,
        lua: lua
            .iter()
            .map(|(p, t)| ((*p).to_string(), (*t).to_string()))
            .collect(),
        schemas: Vec::new(),
        places: Default::default(),
        bytecode: Default::default(),
        hint_problems: Vec::new(),
        texts_used: Vec::new(),
        names: Default::default(),
        screens: None,
    }
}

/// A host that gives game code the tables and keeps what it says.
#[derive(Default)]
struct Shop {
    tables: Tables,
    said: Vec<String>,
}

impl Host for Shop {
    fn say(&mut self, words: &str) {
        self.said.push(words.to_string());
    }
    fn time(&self) -> f64 {
        0.0
    }
    fn find(&self, _name: &str) -> Option<EntityId> {
        None
    }
    fn position(&self, _which: EntityId) -> Option<glam::Vec3> {
        None
    }
    fn move_to(&mut self, _which: EntityId, _place: glam::Vec3) {}
    fn rotation(&self, _which: EntityId) -> Option<glam::Quat> {
        None
    }
    fn turn_to(&mut self, _which: EntityId, _facing: glam::Quat) {}
    fn name(&self, _which: EntityId) -> Option<String> {
        None
    }
    fn tables(&self) -> Tables {
        self.tables.clone()
    }
}

const LUST_SHOP: &str = "fetch \"solis/tables/data/items\":[Items, Item, ItemRow, ItemRarity];\n\
fetch \"solis/project\":Picture;\n\
fetch \"solis/world\":World;\n\
component Shop {\n    has {\n        int price = 0;\n        int total = 0;\n        string names = \"\";\n\
\x20       bool rare = false;\n        bool apple = false;\n        float weight = 0;\n        float red = 0;\n    }\n\
\x20   method update(float dt) {\n\
\x20       ItemRow sword = Items:row(Item.IronSword);\n\
\x20       price = sword.price;\n\
\x20       rare = sword.rarity == ItemRarity.Rare && sword.id == Item.IronSword && not sword.stacks;\n\
\x20       apple = sword.icon == Picture.Apple;\n\
\x20       weight = sword.weight;\n\
\x20       red = sword.tint.r;\n\
\x20       total = 0;\n        names = \"\";\n\
\x20       loop (ItemRow row in Items:rows()) {\n            total += row.price;\n            names = names + row.name + \";\";\n        }\n\
\x20   }\n}\npass Shop;\nend\n";

#[test]
fn lust_reads_a_row_by_its_id_and_every_row_in_order() {
    let code = compiled(LUST_SHOP, &[]);
    let mut host = Shop {
        tables: tables(120),
        ..Shop::default()
    };
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut host).unwrap();
    runtime
        .sync(id(1), "Shop", &ComponentData::default(), &mut host)
        .unwrap();
    let failures = runtime.tick(1.0 / 60.0, &mut host).failures;
    assert!(failures.is_empty(), "{failures:?}");
    let shop = runtime.values(id(1), "Shop").unwrap();
    assert_eq!(shop.get("price"), Some(&Value::Int(120)));
    assert_eq!(shop.get("total"), Some(&Value::Int(175)));
    assert_eq!(
        shop.get("names"),
        Some(&Value::Text("Apple;Iron sword;Potion;".into()))
    );
    assert_eq!(shop.get("rare"), Some(&Value::Bool(true)));
    assert_eq!(shop.get("apple"), Some(&Value::Bool(true)));
    assert_eq!(shop.get("weight"), Some(&Value::Float(1.5)));
    assert_eq!(shop.get("red"), Some(&Value::Float(1.0)));
}

#[test]
fn a_table_saved_while_the_game_runs_is_read_at_the_next_tick() {
    let code = compiled(LUST_SHOP, &[]);
    let mut host = Shop {
        tables: tables(120),
        ..Shop::default()
    };
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut host).unwrap();
    runtime
        .sync(id(1), "Shop", &ComponentData::default(), &mut host)
        .unwrap();
    assert!(runtime.tick(0.1, &mut host).failures.is_empty());
    host.tables = tables(99);
    assert!(runtime.tick(0.1, &mut host).failures.is_empty());
    let shop = runtime.values(id(1), "Shop").unwrap();
    assert_eq!(shop.get("price"), Some(&Value::Int(99)));
    assert_eq!(shop.get("total"), Some(&Value::Int(154)));
}

#[test]
fn a_misspelt_row_or_column_fails_before_the_game_runs() {
    let row = LUST_SHOP.replace("Item.IronSword);", "Item.IronSwrod);");
    let problems = checked(&row).unwrap_err();
    assert!(problems[0].message.contains("IronSwrod"), "{}", problems[0]);
    let column = LUST_SHOP.replace("sword.price;", "sword.prise;");
    let problems = checked(&column).unwrap_err();
    assert!(problems[0].message.contains("prise"), "{}", problems[0]);
    let changed = LUST_SHOP.replace("price = sword.price;", "sword.price = 3;");
    assert!(
        checked(&changed).is_err(),
        "a row's values can't be changed"
    );
}

const LUA_SHOP: &str = "local Shop = component(\"Shop\", {\n\
    { \"price\", \"int\", 0 },\n    { \"count\", \"int\", 0 },\n    { \"rarity\", \"string\", \"\" },\n})\n\
function Shop:update(dt)\n\
    local sword = solis.Items:row(\"IronSword\")\n\
    self.price = sword.price\n\
    self.rarity = sword.rarity\n\
    self.count = #solis.Items:rows()\n\
end\n";

#[test]
fn lua_reads_a_row_by_its_id_and_every_row() {
    let code = compiled("", &[("shop.lua", LUA_SHOP)]);
    let mut host = Shop {
        tables: tables(120),
        ..Shop::default()
    };
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut host).unwrap();
    runtime
        .sync(id(1), "Shop", &ComponentData::default(), &mut host)
        .unwrap();
    let failures = runtime.tick(0.1, &mut host).failures;
    assert!(failures.is_empty(), "{failures:?}");
    let shop = runtime.values(id(1), "Shop").unwrap();
    assert_eq!(shop.get("price"), Some(&Value::Int(120)));
    assert_eq!(shop.get("count"), Some(&Value::Int(3)));
    assert_eq!(shop.get("rarity"), Some(&Value::Text("Rare".into())));
}

#[test]
fn a_misspelt_row_in_lua_stops_with_the_nearest_id() {
    let code = compiled(
        "",
        &[(
            "shop.lua",
            &LUA_SHOP.replace("\"IronSword\"", "\"IronSwrod\""),
        )],
    );
    let mut host = Shop {
        tables: tables(120),
        ..Shop::default()
    };
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut host).unwrap();
    runtime
        .sync(id(1), "Shop", &ComponentData::default(), &mut host)
        .unwrap();
    let failures = runtime.tick(0.1, &mut host).failures;
    let said = format!("{failures:?}");
    assert!(
        said.contains("Items has no row IronSwrod: did you mean IronSword?"),
        "{said}"
    );
    assert!(said.contains("shop.lua"), "{said}");
}

#[test]
fn a_game_without_tables_has_no_table_in_lua() {
    let code = compiled("", &[("shop.lua", LUA_SHOP)]);
    let mut host = Shop::default();
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut host).unwrap();
    runtime
        .sync(id(1), "Shop", &ComponentData::default(), &mut host)
        .unwrap();
    let failures = runtime.tick(0.1, &mut host).failures;
    assert!(
        format!("{failures:?}").contains("solis.Items"),
        "{failures:?}"
    );
}

#[test]
fn rust_reads_rows_by_id_and_cells_by_column() {
    let tables = tables(120);
    let items = tables.get("Items").unwrap();
    let sword = items.row("IronSword").unwrap();
    assert_eq!(sword.whole("price"), Some(120));
    assert_eq!(sword.number("price"), Some(120.0));
    assert_eq!(sword.text("rarity"), Some("Rare"));
    assert_eq!(sword.yes("stacks"), Some(false));
    assert_eq!(sword.colour("tint"), Some([1.0, 0.5, 0.0, 1.0]));
    assert_eq!(sword.whole("prise"), None);
    assert!(items.row("IronSwrod").is_none());
    let ids: Vec<&str> = items.rows().map(|r| r.id()).collect();
    assert_eq!(ids, ["Apple", "IronSword", "Potion"]);
    assert_eq!(items.nearest("ironswrd"), Some("IronSword"));
}

#[test]
fn the_hash_follows_the_tables() {
    assert_eq!(Tables::default().hash(), 0);
    assert_eq!(tables(120).hash(), tables(120).hash());
    assert_ne!(tables(120).hash(), tables(121).hash());
    assert_ne!(tables(120).hash(), 0);
    // A table named like an engine object is left out.
    let clash = Tables::new(&[("world".into(), items(1))], &["World"]);
    assert!(clash.is_empty());
}

/// How long a lookup by ID takes in Lust, after its first: printed for
/// the scale case (`cargo test -p script --release -- --ignored
/// lookups_are_quick --nocapture`).
#[test]
#[ignore = "a measurement, run by hand in a release build"]
fn lookups_are_quick() {
    let mut table = Table::new("Thing");
    table.columns = (0..20)
        .map(|n| Column::new(format!("c{n}"), ColumnKind::Whole))
        .collect();
    table.rows = (0..10_000)
        .map(|n| {
            Arc::new(Row {
                id: format!("T{n}"),
                cells: (0..20).map(Cell::Whole).collect(),
            })
        })
        .collect();
    let table = Arc::new(table);
    let names = TableNames {
        path: "things".into(),
        table: Arc::clone(&table),
    };
    let (path, text) = table_file("Test 1.0", "solis", &names, &|_| "None".into())
        .0
        .unwrap();
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push((path, text));
    files.push((
        "main.lst".into(),
        "fetch \"solis/tables/things\":[Things, Thing, ThingRow];\n\
         component Reader {\n    has {\n        int sum = 0;\n    }\n\
         \x20   method update(float dt) {\n\
         \x20       loop 100000 {\n            sum += Things:row(Thing.T5000).c7;\n        }\n\
         \x20   }\n}\npass Reader;\nend\n"
            .into(),
    ));
    let chunk = lust::to_lua_with(
        &lust::Engine {
            folders: crate::describe::engine_folders(),
        },
        "main.lst",
        &files,
    )
    .unwrap_or_else(|p| panic!("{}", p[0]));
    let code = Compiled {
        lust: Some(chunk),
        ..compiled("", &[])
    };
    let mut host = Shop {
        tables: Tables::new(&[("things".into(), table)], &[]),
        ..Shop::default()
    };
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut host).unwrap();
    runtime
        .sync(id(1), "Reader", &ComponentData::default(), &mut host)
        .unwrap();
    assert!(runtime.tick(0.0, &mut host).failures.is_empty());
    let start = std::time::Instant::now();
    assert!(runtime.tick(0.0, &mut host).failures.is_empty());
    let each = start.elapsed().as_nanos() / 100_000;
    println!("a lookup by ID and a column read: {each} ns (with the loop's own cost)");
    assert!(each < 2_000, "{each} ns");
}
