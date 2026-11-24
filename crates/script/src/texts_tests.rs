//! The game's text read by game code: `World:text` and a line's method
//! with values in Lust, Lua and Rust; a misspelt key failing before the
//! game runs (Lust) or with the nearest (Lua); the player's language
//! changed while the game runs; and a table's translated cells.

use crate::code::Compiled;
use crate::{Host, Runtime, Tables};
use level::languages::Languages;
use level::table::{Cell, Column, ColumnKind, Row, Table};
use reflect::{ComponentData, EntityId, Value};
use std::sync::Arc;
use translations::{Language, TableText, Texts, Translations};

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn lang(code: &str) -> Language {
    Language::new(code).unwrap()
}

/// A small table of items whose names are translated.
fn items() -> Arc<Table> {
    let mut table = Table::new("Item");
    let mut name = Column::new("name", ColumnKind::Text);
    name.translated = true;
    table.columns = vec![name, Column::new("price", ColumnKind::Whole)];
    table.rows = vec![Arc::new(Row {
        id: "Apple".into(),
        cells: vec![Cell::Text("Apple".into()), Cell::Whole(5)],
    })];
    table.kept.table = "Items".into();
    Arc::new(table)
}

/// The shop's words in English and Norwegian, and the items' names.
fn shop() -> Arc<Translations> {
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    let texts = items()
        .texts("Items")
        .into_iter()
        .map(|(key, text)| TableText {
            key,
            file: "data/items.table.lst".into(),
            text,
        })
        .collect();
    Arc::new(Translations::new(
        &languages,
        &[
            (
                "locale/en/shop.ftl".into(),
                "Shop-Welcome = Welcome!\nShop-Apples = { $count ->\n    [one] { $who } has one apple\n   *[other] { $who } has { $count } apples\n}\n".into(),
            ),
            (
                "locale/nb/shop.ftl".into(),
                "Shop-Welcome = Velkommen!\nShop-Apples = { $count ->\n    [one] { $who } har ett eple\n   *[other] { $who } har { $count } epler\n}\n".into(),
            ),
            ("locale/nb/items.ftl".into(), "Items-Apple-name = Eple\n".into()),
        ],
        texts,
    ))
}

/// A host that gives game code the text in a language, and the table.
struct Reader {
    texts: Texts,
    tables: Tables,
}

impl Reader {
    fn new(language: &str) -> Self {
        Self {
            texts: Texts::new(shop(), lang(language)),
            tables: Tables::new(&[("data/items".into(), items())], &["World"]),
        }
    }
}

impl Host for Reader {
    fn say(&mut self, _words: &str) {}
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
    fn texts(&self) -> Option<Texts> {
        Some(self.texts.clone())
    }
    fn tables(&self) -> Tables {
        self.tables.clone()
    }
}

/// The engine's files with the shop's text, its own file and `main`,
/// checked.
fn checked(main: &str) -> Result<lust::LuaChunk, Vec<lust::Problem>> {
    let (names, notes) = crate::text_code::TextNames::of(&shop(), &[]);
    assert!(notes.is_empty(), "{notes:?}");
    let project = crate::project_names::Names {
        texts: names.keys.clone(),
        languages: names.languages.clone(),
        ..crate::project_names::Names::standard()
    };
    let (mut files, notes) = crate::describe::descriptions_for("Test 1.0", "solis", &project);
    assert!(notes.is_empty(), "{notes:?}");
    files.push((
        format!("solis/{}", crate::text_code::FILE),
        crate::text_code::texts_file("Test 1.0", &names).unwrap(),
    ));
    let table = crate::table_code::TableNames {
        path: "data/items".into(),
        table: items(),
    };
    files.push(
        crate::table_code::table_file("Test 1.0", "solis", &table, &|_| "None".into())
            .0
            .unwrap(),
    );
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
        engine_reads: Vec::new(),
    }
}

const LUST_SIGN: &str = "fetch \"solis/project\":[Text, Language];\n\
fetch \"solis/world\":World;\n\
fetch \"solis/texts\":ShopText;\n\
fetch \"solis/tables/data/items\":[Items, Item];\n\
component Sign {\n    has {\n        string hello = \"\";\n        string apples = \"\";\n        string apple = \"\";\n    }\n\
\x20   method update(float dt) {\n\
\x20       hello = World:text(Text.Shop.Welcome);\n\
\x20       apples = ShopText:apples(3, \"Ada\");\n\
\x20       apple = Items:row(Item.Apple).name;\n\
\x20   }\n}\npass Sign;\nend\n";

/// Runs one tick of `code` for an entity with a `Sign`, reading in
/// `reader`, and gives back the sign's values.
fn sign(code: &Compiled, reader: &mut Reader) -> ComponentData {
    let mut runtime = Runtime::new().unwrap();
    runtime.load(code, reader).unwrap();
    runtime
        .sync(id(1), "Sign", &ComponentData::default(), reader)
        .unwrap();
    let failures = runtime.tick(0.1, reader).failures;
    assert!(failures.is_empty(), "{failures:?}");
    runtime.values(id(1), "Sign").unwrap()
}

fn text(data: &ComponentData, field: &str) -> String {
    match data.get(field) {
        Some(Value::Text(t)) => t.clone(),
        other => panic!("{field}: {other:?}"),
    }
}

#[test]
fn lust_reads_text_in_the_players_language() {
    let code = compiled(LUST_SIGN, &[]);
    let en = sign(&code, &mut Reader::new("en"));
    assert_eq!(text(&en, "hello"), "Welcome!");
    assert_eq!(text(&en, "apples"), "Ada has 3 apples");
    assert_eq!(text(&en, "apple"), "Apple");
    let nb = sign(&code, &mut Reader::new("nb"));
    assert_eq!(text(&nb, "hello"), "Velkommen!");
    assert_eq!(text(&nb, "apples"), "Ada har 3 epler");
    assert_eq!(text(&nb, "apple"), "Eple", "a table's translated cell");
}

#[test]
fn a_misspelt_key_or_a_value_left_out_fails_before_the_game_runs() {
    let misspelt = LUST_SIGN.replace("Text.Shop.Welcome", "Text.Shop.Welcom");
    let problems = checked(&misspelt).unwrap_err();
    assert!(problems[0].message.contains("Welcom"), "{}", problems[0]);
    let short = LUST_SIGN.replace("ShopText:apples(3, \"Ada\")", "ShopText:apples(3)");
    assert!(checked(&short).is_err(), "a value left out");
    let wrong = LUST_SIGN.replace(
        "ShopText:apples(3, \"Ada\")",
        "ShopText:apples(\"three\", \"Ada\")",
    );
    assert!(checked(&wrong).is_err(), "a number is a number");
}

#[test]
fn changing_language_while_playing_changes_the_text_at_once() {
    let code = compiled(LUST_SIGN, &[]);
    let mut reader = Reader::new("en");
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut reader).unwrap();
    runtime
        .sync(id(1), "Sign", &ComponentData::default(), &mut reader)
        .unwrap();
    assert!(runtime.tick(0.1, &mut reader).failures.is_empty());
    assert_eq!(
        text(&runtime.values(id(1), "Sign").unwrap(), "hello"),
        "Welcome!"
    );
    reader.texts = Texts::new(shop(), lang("nb"));
    assert!(runtime.tick(0.1, &mut reader).failures.is_empty());
    let after = runtime.values(id(1), "Sign").unwrap();
    assert_eq!(
        text(&after, "hello"),
        "Velkommen!",
        "the next tick reads Norwegian"
    );
    assert_eq!(
        text(&after, "apple"),
        "Eple",
        "and the table's rows are made again"
    );
}

const LUA_SIGN: &str = "local Sign = component(\"Sign\", {\n\
    { \"hello\", \"string\", \"\" },\n    { \"apples\", \"string\", \"\" },\n    { \"apple\", \"string\", \"\" },\n})\n\
function Sign:update(dt)\n\
    self.hello = solis.World:text(\"Shop.Welcome\")\n\
    self.apples = solis.ShopText:apples(1, \"Ada\")\n\
    self.apple = solis.Items:row(\"Apple\").name\n\
end\n";

#[test]
fn lua_reads_text_and_lines_with_values() {
    let code = compiled("", &[("sign.lua", LUA_SIGN)]);
    let nb = sign(&code, &mut Reader::new("nb"));
    assert_eq!(text(&nb, "hello"), "Velkommen!");
    assert_eq!(text(&nb, "apples"), "Ada har ett eple");
    assert_eq!(text(&nb, "apple"), "Eple");
}

#[test]
fn a_misspelt_key_in_lua_stops_with_the_nearest() {
    let code = compiled(
        "",
        &[(
            "sign.lua",
            &LUA_SIGN.replace("\"Shop.Welcome\"", "\"Shop.Welcom\""),
        )],
    );
    let mut reader = Reader::new("en");
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut reader).unwrap();
    runtime
        .sync(id(1), "Sign", &ComponentData::default(), &mut reader)
        .unwrap();
    let failures = format!("{:?}", runtime.tick(0.1, &mut reader).failures);
    assert!(
        failures.contains("did you mean Shop.Welcome?"),
        "{failures}"
    );
    assert!(
        failures.contains("sign.lua"),
        "at the game code's line: {failures}"
    );
}

#[test]
fn rust_reads_text_through_its_host() {
    let reader = Reader::new("nb");
    assert_eq!(reader.text("Shop.Welcome"), "Velkommen!");
    let values = [
        ("count", translations::TextValue::Number(2.0)),
        ("who", translations::TextValue::Text("Ada".into())),
    ];
    assert_eq!(reader.text_with("Shop.Apples", &values), "Ada har 2 epler");
    let tables = reader.tables();
    let apple = tables.get("Items").unwrap().row("Apple").unwrap();
    assert_eq!(
        apple.translated("name", reader.texts().as_ref()).as_deref(),
        Some("Eple")
    );
    assert_eq!(apple.translated("name", None).as_deref(), Some("Apple"));
    assert_eq!(apple.text("name"), Some("Apple"), "the cell as written");
}

#[test]
fn the_lines_with_values_file_names_each_value_in_order() {
    let (names, _) = crate::text_code::TextNames::of(&shop(), &[]);
    let file = crate::text_code::texts_file("Test 1.0", &names).unwrap();
    assert!(file.contains("object ShopText {"), "{file}");
    assert!(
        file.contains("method string apples(float count, string who) native;"),
        "{file}"
    );
    assert!(
        !file.contains("welcome"),
        "a line without values is World:text's"
    );
    assert_eq!(names.keys, ["Shop.Welcome", "Shop.Apples"]);
    assert_eq!(names.languages, ["En", "Nb"]);
}
