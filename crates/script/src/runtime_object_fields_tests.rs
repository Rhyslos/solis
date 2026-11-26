//! The engine's objects' fields (the uniformity pass, U5b): game code
//! reads `Screens.textSize` as the tick started and sets it with `=`,
//! from any component, which asks the engine as the call it replaces did.

use super::*;
use crate::code::Code;
use crate::settings::Setting;
use std::cell::RefCell;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A host with settings of its own, noting what game code says and sets.
#[derive(Default)]
struct Settings {
    said: RefCell<Vec<String>>,
    set: RefCell<Vec<Setting>>,
    speeds: RefCell<Vec<f64>>,
    hours: RefCell<Vec<f64>>,
}

impl Host for Settings {
    fn say(&mut self, words: &str) {
        self.said.borrow_mut().push(words.to_owned());
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
    fn settings(&self) -> crate::settings::PlayerSettings {
        crate::settings::PlayerSettings {
            text_size: 2.0,
            fullscreen: true,
            captions: true,
            ..Default::default()
        }
    }
    fn set_setting(&mut self, change: Setting) {
        self.set.borrow_mut().push(change);
    }
    fn game_speed(&self) -> f64 {
        0.5
    }
    fn set_game_speed(&mut self, speed: f64) {
        self.speeds.borrow_mut().push(speed);
    }
    fn time_of_day(&self) -> f64 {
        21.5
    }
    fn set_time_of_day(&mut self, hour: f64) {
        self.hours.borrow_mut().push(hour);
    }
}

/// A component that reads the fields, then sets each.
const MENU: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/screens\":Screens;\n\
    fetch \"solis/window\":Window;\n\
    component Menu {\n\
        method update(float dt) {\n\
            World:say(Screens.textSize.text() + \" \" + Window.fullscreen.text() + \" \" \
                + World.gameSpeed.text() + \" \" + World.timeOfDay.text() + \" \" \
                + Screens.captions.text());\n\
            Screens.textSize = 1.5;\n\
            Screens.uiSize = 1.25;\n\
            Screens.subtitles = true;\n\
            Screens.captionSize = 2;\n\
            Window.fullscreen = false;\n\
            World.gameSpeed = 0;\n\
            World.gameSpeed = 0;\n\
            World.timeOfDay = 6;\n\
        }\n\
    }\n\
    pass Menu;\n\
    end\n";

#[test]
fn game_code_reads_and_sets_the_engine_s_objects_fields() {
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push(("main.lst".into(), MENU.into()));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let compiled = match crate::compile(&code) {
        Ok(compiled) => compiled.expect("there's code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut runtime = Runtime::new().unwrap();
    let mut host = Settings::default();
    runtime.load(&compiled, &mut host).unwrap();
    runtime
        .sync(id(1), "Menu", &ComponentData::default(), &mut host)
        .unwrap();
    let ticked = runtime.tick(1.0 / 60.0, &mut host);
    assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    assert_eq!(
        host.said.into_inner(),
        ["2.0 true 0.5 21.5 true"],
        "each read is the engine's, not the field's starting value"
    );
    assert_eq!(
        host.set.into_inner(),
        [
            Setting::TextSize(1.5),
            Setting::UiSize(1.25),
            Setting::Subtitles(true),
            Setting::CaptionSize(2.0),
            Setting::Fullscreen(false),
        ]
    );
    assert_eq!(
        host.speeds.into_inner(),
        [0.0, 0.0],
        "setting the same value twice asks twice, as calling did"
    );
    assert_eq!(host.hours.into_inner(), [6.0]);
}

#[test]
fn the_old_calls_are_gone_from_lust_saying_the_field() {
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push((
        "main.lst".into(),
        "fetch \"solis/screens\":Screens;\n\
         component Menu {\n    method update(float dt) {\n        Screens:setTextSize(1.5);\n    }\n}\n\
         pass Menu;\nend\n"
            .into(),
    ));
    let problems = crate::compile(&Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    })
    .expect_err("setTextSize isn't Lust's any more");
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert_eq!(problems[0].line, 4);
    assert!(
        problems[0].message.contains("Screens.textSize = …;"),
        "{}",
        problems[0].message
    );
}
