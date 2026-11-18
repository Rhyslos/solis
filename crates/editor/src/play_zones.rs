//! Trigger zones while playing (stage 1, triggers): the world's overlap
//! pass runs once a tick, as game code's turn begins, on where physics'
//! last step left everything, and each crossing becomes two events for
//! game code: the zone hears `Entered(who)` or `Exited(who)`, and whoever
//! walked hears `EnteredZone(zone)` or `LeftZone(zone)`. They come after
//! the sprites' cues, in the pass's order (by zone, then by who), and
//! only to entities with game code.

use reflect::Value;

/// The overlap pass's crossings this tick, as events for game code.
pub(super) fn zone_events(world: &mut world::World) -> Vec<script::Event> {
    let crossings = world.overlap_zones();
    let mut events = Vec::new();
    for crossing in crossings {
        let (to_zone, to_who) = if crossing.entered {
            ("Entered", "EnteredZone")
        } else {
            ("Exited", "LeftZone")
        };
        for (to, name, about) in [
            (crossing.zone, to_zone, crossing.who),
            (crossing.who, to_who, crossing.zone),
        ] {
            if world.has_game_components(to) {
                events.push(script::Event {
                    to,
                    name: name.to_owned(),
                    values: vec![Value::Entity(Some(about))],
                    only: None,
                });
            }
        }
    }
    events
}
