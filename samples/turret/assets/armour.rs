// Armour, the drone's second component, written in Rust: each hit dents
// a plate, and it tells the drone how many are left. (Until Solis builds
// a game's own Rust, in Phase 10, this file is compiled into Solis itself
// and offered only in the turret sample.)

use reflect::{ComponentData, ComponentSchema, FieldInfo, FieldKind, Value};
use script::{Context, Event, RustComponent, RustKind};

/// The armour's plates left.
pub struct Armour {
    plates: i64,
}

impl RustComponent for Armour {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        if event.name != "Hit" || self.plates == 0 {
            return false;
        }
        self.plates -= 1;
        let me = cx.me();
        cx.send(Some(me), "Dented", vec![Value::Int(self.plates)]);
        true
    }

    fn values(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("plates", Value::Int(self.plates));
        data
    }

    fn set(&mut self, data: &ComponentData) {
        if let Some(Value::Int(plates)) = data.get("plates") {
            self.plates = (*plates).max(0);
        }
    }
}

/// Armour as Solis registers it: its field, and how one is made.
pub fn armour() -> RustKind {
    RustKind {
        schema: ComponentSchema {
            name: "Armour".into(),
            help: "Plates that dent when hit (the turret sample's Rust component).".into(),
            fields: vec![FieldInfo::new(
                "plates",
                "Plates",
                FieldKind::Int,
                Value::Int(3),
            )],
        },
        make: |data| {
            let mut armour = Armour { plates: 3 };
            armour.set(data);
            Box::new(armour)
        },
    }
}
