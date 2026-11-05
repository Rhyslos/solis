//! Where maps lie and the 2D follow camera (decision 0082): a place in
//! the world turned into a map's cell and back, a map's box, and the
//! kit's FollowCamera2D waiting in its dead zone, easing after the
//! character and staying inside a map.

use super::*;
use crate::code::Compiled;
use crate::host::MapPlace;
use glam::{Mat4, Vec3};
use reflect::Value;
use std::collections::HashMap;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A small 2D world: entities at places, one map (40 × 30 cells of a
/// metre, its top left corner at the map entity), a 30 × 17 m view.
struct Flat {
    places: HashMap<EntityId, Vec3>,
    map_entity: EntityId,
    place: MapPlace,
}

impl Flat {
    fn new() -> Self {
        let map_entity = id(9);
        Self {
            places: HashMap::from([(map_entity, Vec3::new(0.0, 30.0, 0.0))]),
            map_entity,
            place: MapPlace {
                transform: Mat4::from_translation(Vec3::new(0.0, 30.0, 0.0)),
                cell: [1.0, 1.0],
                size: [40, 30],
            },
        }
    }
}

impl Host for Flat {
    fn say(&mut self, _words: &str) {}
    fn time(&self) -> f64 {
        0.0
    }
    fn find(&self, _name: &str) -> Option<EntityId> {
        None
    }
    fn position(&self, which: EntityId) -> Option<Vec3> {
        self.places.get(&which).copied()
    }
    fn move_to(&mut self, which: EntityId, place: Vec3) {
        self.places.insert(which, place);
    }
    fn rotation(&self, _which: EntityId) -> Option<glam::Quat> {
        None
    }
    fn turn_to(&mut self, _which: EntityId, _facing: glam::Quat) {}
    fn name(&self, _which: EntityId) -> Option<String> {
        None
    }
    fn map_place(&self, file: &str) -> Option<MapPlace> {
        (file == "maps/farm").then_some(self.place)
    }
    fn map_bounds(&self, which: EntityId) -> Option<(Vec3, Vec3)> {
        (which == self.map_entity).then(|| self.place.bounds())
    }
    fn view_size(&self) -> Vec3 {
        Vec3::new(30.0, 17.0, 0.0)
    }
}

/// `main` with the engine's files, the kit and the farm map's own file.
fn compiled(main: &str) -> Compiled {
    let names = crate::map_code::MapNames {
        path: "maps/farm".into(),
        layers: vec!["Ground".into()],
        terrains: vec![(b'g', "grass".into())],
        values: Vec::new(),
    };
    let (path, text) = crate::map_code::map_file("Test 1.0", "solis", &names)
        .0
        .unwrap();
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push((path, text));
    files.push(("main.lst".into(), main.to_string()));
    let chunk = lust::to_lua_with(
        &lust::Engine {
            folders: crate::describe::engine_folders(&files),
        },
        "main.lst",
        &files,
    )
    .unwrap_or_else(|problems| panic!("{}", problems[0]));
    Compiled {
        lust: Some(chunk),
        lua: Vec::new(),
        schemas: Vec::new(),
        places: Default::default(),
        bytecode: Default::default(),
        screens: None,
    }
}

#[test]
fn a_place_becomes_a_cell_and_a_cell_a_place() {
    let code = compiled(
        "fetch \"solis/maps/maps/farm\":FarmMap;\n\
         fetch \"solis/tiles\":[MapCell, MapBounds];\n\
         fetch \"solis/world\":World;\n\
         component Farmer {\n    has {\n        FarmMap farm;\n        entity ground = nothing;\n\
         \x20       int x = -1;\n        int y = -1;\n        bool on = false;\n        bool offOn = true;\n\
         \x20       vec3 middle = [0, 0, 0];\n        vec3 high = [0, 0, 0];\n        vec3 shown = [0, 0, 0];\n    }\n\
         \x20   method update(float dt) {\n\
         \x20       MapCell cell = farm:cellAt([3.5, 25.2, 0]);\n\
         \x20       x = cell.x;\n        y = cell.y;\n        on = cell.inside;\n\
         \x20       offOn = farm:cellAt([-2, 5, 0]).inside;\n\
         \x20       middle = farm:placeOf(3, 4);\n\
         \x20       MapBounds box = ground:mapBounds();\n        is box.found? then {\n            high = box.high;\n        }\n\
         \x20       shown = World:viewSize();\n    }\n}\npass Farmer;\nend\n",
    );
    let mut host = Flat::new();
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, &mut host).unwrap();
    let mut data = ComponentData::default();
    data.set("ground", Value::Entity(Some(host.map_entity)));
    runtime.sync(id(1), "Farmer", &data, &mut host).unwrap();
    let failures = runtime.tick(1.0 / 60.0, &mut host).failures;
    assert!(failures.is_empty(), "{failures:?}");
    let farmer = runtime.values(id(1), "Farmer").unwrap();
    // 3.5 across, 4.8 down from the top left (30 - 25.2): cell 3, 4.
    assert_eq!(farmer.get("x"), Some(&Value::Int(3)));
    assert_eq!(farmer.get("y"), Some(&Value::Int(4)));
    assert_eq!(farmer.get("on"), Some(&Value::Bool(true)));
    assert_eq!(farmer.get("offOn"), Some(&Value::Bool(false)));
    assert_eq!(
        farmer.get("middle"),
        Some(&Value::floats(&[3.5, 25.5, 0.0]))
    );
    assert_eq!(farmer.get("high"), Some(&Value::floats(&[40.0, 30.0, 0.0])));
    assert_eq!(
        farmer.get("shown"),
        Some(&Value::floats(&[30.0, 17.0, 0.0]))
    );
}

/// The follow camera on `camera`, following `farmer` inside the map.
fn camera(host: &mut Flat, smoothing: f64, bounded: bool) -> Runtime {
    let code = compiled("fetch \"solis/kit/cameras\":FollowCamera2D;\nend\n");
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&code, host).unwrap();
    let mut data = ComponentData::default();
    data.set("target", Value::Entity(Some(id(1))));
    if bounded {
        data.set("bounds", Value::Entity(Some(host.map_entity)));
    }
    data.set("smoothing", Value::Float(smoothing));
    runtime.sync(id(2), "FollowCamera2D", &data, host).unwrap();
    runtime
}

#[test]
fn the_camera_waits_in_its_dead_zone_then_catches_up_smoothly() {
    let mut host = Flat::new();
    host.places.insert(id(1), Vec3::new(20.0, 15.0, 0.0));
    host.places.insert(id(2), Vec3::new(20.0, 15.0, 10.0));
    let mut runtime = camera(&mut host, 0.2, false);
    // Inside the dead zone (a metre across): the camera stays.
    host.places.insert(id(1), Vec3::new(20.8, 15.3, 0.0));
    assert!(runtime.tick(1.0 / 60.0, &mut host).failures.is_empty());
    assert_eq!(host.places[&id(2)], Vec3::new(20.0, 15.0, 10.0));
    // Out of it: the camera eases after, never past, and keeps its height.
    host.places.insert(id(1), Vec3::new(26.0, 15.0, 0.0));
    let mut last = 20.0;
    for _ in 0..120 {
        assert!(runtime.tick(1.0 / 60.0, &mut host).failures.is_empty());
        let x = host.places[&id(2)].x;
        assert!(x >= last - 1e-4 && x <= 25.0 + 1e-3, "{x}");
        last = x;
    }
    assert!((last - 25.0).abs() < 0.01, "it caught up: {last}");
    assert_eq!(host.places[&id(2)].z, 10.0);
}

#[test]
fn the_camera_stays_inside_its_map() {
    let mut host = Flat::new();
    // The farmer in the map's top left corner: the view (30 × 17) stops
    // at the map's edges, 15 and 8.5 in.
    host.places.insert(id(1), Vec3::new(1.0, 29.0, 0.0));
    host.places.insert(id(2), Vec3::new(1.0, 29.0, 10.0));
    let mut runtime = camera(&mut host, 0.0, true);
    assert!(runtime.tick(1.0 / 60.0, &mut host).failures.is_empty());
    assert_eq!(host.places[&id(2)], Vec3::new(15.0, 21.5, 10.0));
    // A map narrower than the view: held in its middle.
    host.place.size = [20, 30];
    assert!(runtime.tick(1.0 / 60.0, &mut host).failures.is_empty());
    assert_eq!(host.places[&id(2)].x, 10.0);
}

#[test]
fn a_cell_far_off_or_not_a_number_is_off_the_map() {
    let place = Flat::new().place;
    assert_eq!(
        place.cell_at(Vec3::new(f32::NAN, 0.0, 0.0)),
        ([0, 0], false)
    );
    assert_eq!(place.cell_at(Vec3::new(1.0e12, 0.0, 0.0)), ([0, 0], false));
    assert_eq!(place.cell_at(Vec3::new(39.9, 0.1, 0.0)), ([39, 29], true));
    assert!(!place.cell_at(Vec3::new(40.0, 0.1, 0.0)).1);
    assert_eq!(place.place_of(0, 0), Vec3::new(0.5, 29.5, 0.0));
}

#[test]
fn the_manual_s_hoe_is_sound_code() {
    // docs/manual/tile-maps.md, "Which cell is the player on?".
    let page = include_str!("../../../docs/manual/tile-maps.md").replace("\r\n", "\n");
    let start = page
        .find("fetch \"solis/maps/maps/farm\":[FarmMap, FarmLayer, FarmTerrain];\nfetch \"solis/tiles\":MapCell;")
        .unwrap();
    let end = start + page[start..].find("end\n").unwrap() + 4;
    let names = crate::map_code::MapNames {
        path: "maps/farm".into(),
        layers: vec!["Ground".into()],
        terrains: vec![(b'.', "grass".into()), (b't', "tilled".into())],
        values: Vec::new(),
    };
    let (path, text) = crate::map_code::map_file("Test 1.0", "solis", &names)
        .0
        .unwrap();
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push((path, text));
    files.push(("main.lst".into(), page[start..end].to_string()));
    let engine = lust::Engine {
        folders: crate::describe::engine_folders(&files),
    };
    if let Err(problems) = lust::check_with(&engine, "main.lst", &files) {
        panic!("{}", problems[0]);
    }
}
