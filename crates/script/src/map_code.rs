//! Each tile map's own Lust file ([decision 0079](../../../docs/decisions/0079-tile-maps-reached-through-their-own-files.md)):
//! like the animators' (decision 0078), the game API guidelines' rule 4.
//! `maps/farm.tilemap.lst` gives `solis/maps/maps/farm.lst`: states of its
//! layers (`FarmLayer`), its tileset's terrains (`FarmTerrain`) and each
//! choice value's options (`FarmCrop`), and a class with a method per thing
//! code can ask or change (`FarmMap`: `terrain(layer, x, y)`,
//! `setTerrain(…)`, `growth(x, y)`, `setGrowth(x, y, to)`, `width()`…).
//! A component holds one as a field the engine fills in (`FarmMap farm;`).
//!
//! The methods' bodies call the engine's `Maps` object with the map's path
//! and places by number; game writers never see them.

use crate::project_names::{NO_OPTION, option_name_of};

/// What a map's value holds, for its methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueNames {
    /// A whole number: `int growth(x, y)`, `setGrowth(x, y, to)`.
    Number,
    /// On or off: `bool watered(x, y)`, `setWatered(x, y, on)`.
    Switch,
    /// One of these options, or none: a state of its own.
    Choice(Vec<String>),
}

/// One tile map file, as the editor read it: its path without
/// `.tilemap.lst`, its layers, its tileset's terrains (code and name) and
/// its values.
#[derive(Debug, Clone, PartialEq)]
pub struct MapNames {
    /// Its path in the assets, without the extension (`maps/farm`).
    pub path: String,
    /// Its layers' names, in order.
    pub layers: Vec<String>,
    /// Its tileset's terrains: the code each cell holds, and the name.
    pub terrains: Vec<(u8, String)>,
    /// Its values: name and what it holds, in order.
    pub values: Vec<(String, ValueNames)>,
}

/// The methods every map's class has, which no value may take.
const OWN: &[&str] = &[
    "width",
    "height",
    "terrain",
    "setTerrain",
    "cellAt",
    "placeOf",
];

/// Words with each one's first letter a capital (`use tool` is `UseTool`),
/// as a Lust name, or none.
fn joined(name: &str) -> Option<String> {
    let compact: String = name
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| {
                f.to_ascii_uppercase().to_string() + c.as_str()
            })
        })
        .collect();
    option_name_of(&compact)
}

/// Options for `names` (each made a Lust name), each with its place, and
/// notes on those left out. `first` goes first, standing for none.
fn options(
    names: impl Iterator<Item = (usize, String)>,
    first: Option<&str>,
    what: &str,
    notes: &mut Vec<String>,
) -> Vec<(String, Option<usize>)> {
    let mut out: Vec<(String, Option<usize>)> =
        first.map(|f| (f.to_string(), None)).into_iter().collect();
    for (at, name) in names {
        match joined(&name) {
            Some(option) if !out.iter().any(|(o, _)| o.eq_ignore_ascii_case(&option)) => {
                out.push((option, Some(at)));
            }
            _ => notes.push(format!(
                "The {what} \"{name}\" can't be named in game code (or is the same as another but \
                 for capitals and spaces); rename it to reach it from code."
            )),
        }
    }
    out
}

/// A `check` turning a state into a number: `int at = -1;` then one
/// `when` per option.
fn to_number(
    state_value: &str,
    out: &str,
    options: &[(String, Option<usize>)],
    numbers: &dyn Fn(Option<usize>) -> i64,
) -> String {
    let mut text = format!("        int {out} = -1;\n        check {state_value} {{\n");
    for (option, at) in options {
        text.push_str(&format!(
            "            when {option} {{\n                {out} = {};\n            }}\n",
            numbers(*at)
        ));
    }
    text.push_str("        }\n");
    text
}

/// Questions turning a number back into a state's option.
fn from_number(
    value: &str,
    state: &str,
    options: &[(String, Option<usize>)],
    numbers: &dyn Fn(Option<usize>) -> i64,
) -> String {
    let mut text = String::new();
    for (option, at) in options.iter().skip(1) {
        text.push_str(&format!(
            "        is {value} == {}? then {{\n            return {state}.{option};\n        }}\n",
            numbers(*at)
        ));
    }
    text.push_str(&format!(
        "        return {state}.{};\n",
        options.first().map_or(NO_OPTION, |(o, _)| o.as_str())
    ));
    text
}

/// The Lust file for one map (its path and text; none when the map can't
/// be named at all), and notes on anything game code can't reach.
pub fn map_file(
    made_by: &str,
    folder: &str,
    map: &MapNames,
) -> (Option<(String, String)>, Vec<String>) {
    let path = &map.path;
    let mut notes = Vec::new();
    let Some(base) = path.rsplit('/').next().and_then(option_name_of) else {
        notes.push(format!(
            "The tile map {path} can't be reached from game code: its file name must start with a \
             letter and hold only letters, digits and _. Rename it to use it from code."
        ));
        return (None, notes);
    };
    let (class, layer_state, terrain_state) = (
        format!("{base}Map"),
        format!("{base}Layer"),
        format!("{base}Terrain"),
    );
    let file = format!("\"{}\"", path.replace('\\', "/").replace('"', ""));
    let mut layers = options(
        map.layers.iter().cloned().enumerate(),
        None,
        "layer",
        &mut notes,
    );
    if layers.is_empty() {
        layers.push((NO_OPTION.to_string(), None));
    }
    let terrains = options(
        map.terrains
            .iter()
            .map(|(code, name)| (usize::from(*code), name.clone())),
        Some("Empty"),
        "terrain",
        &mut notes,
    );
    let list = |o: &[(String, Option<usize>)]| {
        if o.is_empty() {
            NO_OPTION.to_string()
        } else {
            o.iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    let mut states = format!(
        "-- The map's layers.\nstate {layer_state} = [{}];\n\n-- Its tileset's terrains (Empty: nothing painted).\nstate {terrain_state} = [{}];\n\n",
        list(&layers),
        list(&terrains)
    );
    let index = |at: Option<usize>| at.map_or(-1, |a| a as i64);
    let code = |at: Option<usize>| at.map_or(0, |a| a as i64);
    let mut methods = format!(
        "    -- How many cells across and up.\n    method int width() {{\n        return Maps:width({file});\n    }}\n\
         \x20   method int height() {{\n        return Maps:height({file});\n    }}\n\
         \x20   -- The cell at a place in the world (whether it's on the map, and its x and y).\n\
         \x20   method MapCell cellAt(vec3 place) {{\n        return Maps:cellAt({file}, place);\n    }}\n\
         \x20   -- The place in the world at a cell's middle.\n\
         \x20   method vec3 placeOf(int x, int y) {{\n        return Maps:placeOf({file}, x, y);\n    }}\n\
         \x20   -- The terrain painted on a layer's cell.\n    method {terrain_state} terrain({layer_state} layer, int x, int y) {{\n"
    );
    methods.push_str(&to_number("layer", "at", &layers, &index));
    methods.push_str(&format!(
        "        int code = Maps:terrain({file}, at, x, y);\n"
    ));
    methods.push_str(&from_number("code", &terrain_state, &terrains, &code));
    methods.push_str(&format!(
        "    }}\n    -- Paints a layer's cell with a terrain (Empty clears it).\n    method setTerrain({layer_state} layer, int x, int y, {terrain_state} to) {{\n"
    ));
    methods.push_str(&to_number("layer", "at", &layers, &index));
    methods.push_str(&to_number("to", "code", &terrains, &code));
    methods.push_str(&format!(
        "        Maps:setTerrain({file}, at, x, y, code);\n    }}\n"
    ));
    let mut taken: Vec<String> = OWN.iter().map(|m| (*m).to_string()).collect();
    let mut passed = vec![layer_state.clone(), terrain_state.clone()];
    for (at, (name, kind)) in map.values.iter().enumerate() {
        let Some(upper) = joined(name) else {
            notes.push(format!(
                "The value \"{name}\" in {path} can't be named in game code; rename it to reach it from code."
            ));
            continue;
        };
        let lower = upper[..1].to_ascii_lowercase() + &upper[1..];
        let setter = format!("set{upper}");
        if taken
            .iter()
            .any(|t| t.eq_ignore_ascii_case(&lower) || t.eq_ignore_ascii_case(&setter))
        {
            notes.push(format!(
                "The value \"{name}\" in {path} would be the method {lower}, which the map's class has \
                 already; rename it to reach it from code."
            ));
            continue;
        }
        taken.push(lower.clone());
        taken.push(setter.clone());
        match kind {
            ValueNames::Number => methods.push_str(&format!(
                "    -- The value {name} on a cell.\n    method int {lower}(int x, int y) {{\n        return Maps:number({file}, {at}, x, y);\n    }}\n\
                 \x20   method {setter}(int x, int y, int to) {{\n        Maps:setNumber({file}, {at}, x, y, to);\n    }}\n"
            )),
            ValueNames::Switch => methods.push_str(&format!(
                "    -- The value {name} on a cell.\n    method bool {lower}(int x, int y) {{\n        return Maps:isOn({file}, {at}, x, y);\n    }}\n\
                 \x20   method {setter}(int x, int y, bool on) {{\n        Maps:setSwitch({file}, {at}, x, y, on);\n    }}\n"
            )),
            ValueNames::Choice(names) => {
                let state = format!("{base}{upper}");
                let choices = options(names.iter().cloned().enumerate(), Some(NO_OPTION), "option", &mut notes);
                states.push_str(&format!(
                    "-- The options of the value {name} (None: not set).\nstate {state} = [{}];\n\n",
                    list(&choices)
                ));
                methods.push_str(&format!(
                    "    -- The value {name} on a cell.\n    method {state} {lower}(int x, int y) {{\n        int chosen = Maps:choice({file}, {at}, x, y);\n"
                ));
                methods.push_str(&from_number("chosen", &state, &choices, &index));
                methods.push_str(&format!("    }}\n    method {setter}(int x, int y, {state} to) {{\n"));
                methods.push_str(&to_number("to", "chosen", &choices, &index));
                methods.push_str(&format!("        Maps:setChoice({file}, {at}, x, y, chosen);\n    }}\n"));
                passed.push(state);
            }
        }
    }
    passed.push(class.clone());
    let text = format!(
        "-- Made by {made_by}: the tile map {path}'s names, for game code. It's written again\n\
         -- whenever the map or its tileset changes, so changes made here don't last.\n\
         -- Hold one in a component as a field the engine fills in: {class} map;\n\n\
         fetch \"{folder}/tiles\":[Maps, MapCell];\n\n\
         {states}\
         class {class} {{\n    has {{\n        entity owner = nothing;\n    }}\n{methods}}}\n\n\
         pass {};\nend\n",
        passed.join(", ")
    );
    (Some((format!("{folder}/maps/{path}.lst"), text)), notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn farm() -> MapNames {
        MapNames {
            path: "maps/farm".into(),
            layers: vec!["Ground".into(), "Crops".into()],
            terrains: vec![
                (b'g', "grass".into()),
                (b's', "tilled soil".into()),
                (b'w', "Water".into()),
            ],
            values: vec![
                ("growth".into(), ValueNames::Number),
                ("watered".into(), ValueNames::Switch),
                (
                    "crop".into(),
                    ValueNames::Choice(vec!["parsnip".into(), "corn".into()]),
                ),
                ("width".into(), ValueNames::Number),
                ("bad-name".into(), ValueNames::Switch),
            ],
        }
    }

    #[test]
    fn a_map_becomes_states_and_a_class_with_a_method_each() {
        let (file, notes) = map_file("Test 1.0", "solis", &farm());
        let (path, text) = file.unwrap();
        assert_eq!(path, "solis/maps/maps/farm.lst");
        assert!(
            text.contains("state FarmLayer = [Ground, Crops];"),
            "{text}"
        );
        assert!(
            text.contains("state FarmTerrain = [Empty, Grass, TilledSoil, Water];"),
            "{text}"
        );
        assert!(
            text.contains("state FarmCrop = [None, Parsnip, Corn];"),
            "{text}"
        );
        for method in [
            "method int width()",
            "method FarmTerrain terrain(FarmLayer layer, int x, int y)",
            "method setTerrain(FarmLayer layer, int x, int y, FarmTerrain to)",
            "method int growth(int x, int y)",
            "method setGrowth(int x, int y, int to)",
            "method bool watered(int x, int y)",
            "method FarmCrop crop(int x, int y)",
            "method setCrop(int x, int y, FarmCrop to)",
        ] {
            assert!(text.contains(method), "{method}: {text}");
        }
        assert_eq!(
            notes.len(),
            2,
            "width clashes, bad-name can't be a name: {notes:?}"
        );
    }

    #[test]
    fn the_file_checks_in_lust_and_a_component_can_hold_one() {
        let (path, text) = map_file("Test 1.0", "solis", &farm()).0.unwrap();
        let mut files = crate::describe::descriptions("Test 1.0", "solis");
        files.push((path, text));
        files.push((
            "main.lst".into(),
            "fetch \"solis/maps/maps/farm\":[FarmMap, FarmLayer, FarmTerrain, FarmCrop];\n\
             component Farmer {\n    has {\n        FarmMap farm;\n    }\n\
             \x20   method update(float dt) {\n\
             \x20       is farm:terrain(FarmLayer.Ground, 3, 4) == FarmTerrain.TilledSoil && not farm:watered(3, 4)? then {\n\
             \x20           farm:setWatered(3, 4, true);\n            farm:setGrowth(3, 4, farm:growth(3, 4) + 1);\n        }\n\
             \x20       is farm:crop(3, 4) == FarmCrop.None? then {\n            farm:setCrop(3, 4, FarmCrop.Parsnip);\n        }\n\
             \x20       farm:setTerrain(FarmLayer.Crops, 0, farm:height() - 1, FarmTerrain.Empty);\n    }\n}\npass Farmer;\nend\n"
                .into(),
        ));
        let engine = lust::Engine {
            folders: crate::describe::engine_folders(&files),
        };
        if let Err(problems) = lust::check_with(&engine, "main.lst", &files) {
            let text: Vec<String> = problems.iter().map(ToString::to_string).collect();
            panic!("{}", text.join("\n"));
        }
    }

    #[test]
    fn a_map_with_nothing_or_a_bad_name_is_still_safe() {
        let empty = MapNames {
            path: "cave".into(),
            layers: Vec::new(),
            terrains: Vec::new(),
            values: Vec::new(),
        };
        let (path, text) = map_file("Test 1.0", "solis", &empty).0.unwrap();
        let mut files = crate::describe::descriptions("Test 1.0", "solis");
        files.push((path, text));
        files.push((
            "main.lst".into(),
            "fetch \"solis/maps/cave\":CaveMap;\nend\n".into(),
        ));
        let engine = lust::Engine {
            folders: crate::describe::engine_folders(&files),
        };
        lust::check_with(&engine, "main.lst", &files).unwrap();
        let bad = MapNames {
            path: "2cave".into(),
            ..empty
        };
        let (file, notes) = map_file("Test 1.0", "solis", &bad);
        assert!(file.is_none() && notes[0].contains("2cave"));
    }
}
