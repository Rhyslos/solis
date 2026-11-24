//! What the running game sets on an entity that the level's data doesn't
//! hold: all of it, listed in this one place.
//!
//! An edit while playing makes entities again from the level: the ones
//! edited ([`World::update`]) or all of them ([`World::rebuild`]). What
//! the game made of an entity meanwhile has to outlive that, or the edit
//! undoes it: a screen game code showed is hidden, a sound it stopped
//! starts again. Both ways take it off first ([`World::carry`]) and put
//! it back after ([`World::put_back`]), so neither keeps something the
//! other forgets.
//!
//! | What | Where | Game code's calls |
//! | --- | --- | --- |
//! | The clip a sprite plays, and how far in | `Sprite`: clip, playing, time | `animate`, `animateFromStart`, `stopAnimation`, an animator, the clip moving on |
//! | How fast a sprite's clip plays | `Sprite`: speed | an animator, each tick |
//! | A sprite mirrored | `Sprite`: flip_x, flip_y | `setFlip` |
//! | Which way a sprite faces | `Sprite`: facing | a component's `facing` field, an animator facing by a parameter |
//! | The look a sprite wears | `Sprite`: look | a component's `look` field |
//! | A sound on or off | `AudioSource`: playing | `setSoundPlaying` |
//! | The state an animator is in, by name | `Animator`: state | an animator, each tick; `Animators:jumpTo` |
//! | How fast an animator plays | `Animator`: speed | `Animators:setPlaybackSpeed` |
//! | The screen an entity shows | `Screen`: screen, shown | `showScreen`, `hideScreen` |
//! | An animator's state and parameters | its run, beside its components | `Animators:setNumber`, `setSwitch`, `fire`, `setDirection`, `jumpTo` |
//! | The tags an entity carries | `Tags`: tags | `addTag`, `removeTag` |
//! | A tile map's cells and values | its map, beside its components | `Maps:setTerrain`, `setNumber`, `setSwitch`, `setChoice` |
//!
//! All but the last two are fields the level holds too ([`Part`]). For
//! each the rule is the same: the game's is kept while the level still
//! gives what it gave when the world took the entity in ([`Start`]). An
//! edit that changes the level's wins, and so does one that sets the field
//! by hand to what the level held all along ([`World::set_by_hand`]). An
//! entity given its first tag by game code is given a `Tags`, as
//! `showScreen` gives a Screen. The
//! last two are kept as long as the entity keeps the component they
//! belong to.
//!
//! What game code switches off (`setVisible`, `setSolid`) isn't in the
//! table: the world keeps it in lists of its own by entity ID
//! (`crate::switches`), which an edit never touches.
//!
//! A new call of game code's that sets something on an entity gets a row
//! here, and a [`Part`] if the level holds what it sets. The editor's
//! `game_set_tests.rs` lists every call and plays each that sets
//! something, so one added without being kept fails there.
//!
//! Two more things are the game's and are kept by rules of their own,
//! which both ways already share: where the game put an entity (`moveTo`,
//! `turnTo`, `face`, `walk` and physics: `World::set_transform` notes the
//! level's Transform, and the game's place stays while the level still has
//! that one) and how its body moves (`push`, `setVelocity`: physics isn't
//! made again with the entity).

use crate::animators::Running;
use crate::sprites::Playback;
use crate::tile_maps::MapState;
use crate::{Id, Values, World};
use level::builtin::{Animator, AudioSource, Screen, Sprite, Tags, TileMap};
use reflect::{Component, ComponentData, EntityId};

/// One thing game code sets on an entity's own component: some of the
/// component's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    /// A Sprite's clip, whether it plays and how far in.
    Playback,
    /// How fast a Sprite's clip plays (its animator sets it).
    SpriteSpeed,
    /// A Sprite mirrored across and down.
    Flip,
    /// Which way a Sprite faces (Fixes 3).
    Facing,
    /// The look a Sprite wears (Fixes 3).
    Look,
    /// An AudioSource on or off.
    Sounding,
    /// The state an Animator is in, by name.
    AnimatorState,
    /// How fast an Animator plays its clips.
    AnimatorSpeed,
    /// Which screen a Screen shows, and whether it's shown.
    Screen,
    /// The tags an entity carries (stage 1, tags).
    Tags,
}

impl Part {
    /// Every part.
    pub(crate) const ALL: [Self; 10] = [
        Self::Playback,
        Self::SpriteSpeed,
        Self::Flip,
        Self::Facing,
        Self::Look,
        Self::Sounding,
        Self::AnimatorState,
        Self::AnimatorSpeed,
        Self::Screen,
        Self::Tags,
    ];

    /// The component it's on and its fields there, as the level names
    /// them.
    pub(crate) fn fields(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Playback => (Sprite::NAME, &["clip", "playing", "time"]),
            Self::SpriteSpeed => (Sprite::NAME, &["speed"]),
            Self::Flip => (Sprite::NAME, &["flip_x", "flip_y"]),
            Self::Facing => (Sprite::NAME, &["facing"]),
            Self::Look => (Sprite::NAME, &["look"]),
            Self::Sounding => (AudioSource::NAME, &["playing"]),
            Self::AnimatorState => (Animator::NAME, &["state"]),
            Self::AnimatorSpeed => (Animator::NAME, &["speed"]),
            Self::Screen => (Screen::NAME, &["screen", "shown"]),
            Self::Tags => (Tags::NAME, &["tags"]),
        }
    }

    /// The part `component`'s `field` belongs to, if game code sets it.
    fn of(component: &str, field: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|part| {
            let (on, fields) = part.fields();
            on == component && fields.contains(&field)
        })
    }
}

/// An entity's [`Part`]s: each as the entity has it, or nothing where it
/// hasn't the component.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct GameSet {
    playback: Option<Playback>,
    sprite_speed: Option<f32>,
    flip: Option<[bool; 2]>,
    facing: Option<String>,
    look: Option<String>,
    sounding: Option<bool>,
    animator_state: Option<String>,
    animator_speed: Option<f32>,
    screen: Option<(String, bool)>,
    tags: Option<Tags>,
}

impl GameSet {
    /// Whether the entity has none of the components.
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Takes in a Sprite's parts.
    pub(crate) fn read_sprite(&mut self, sprite: &Sprite) {
        self.playback = Some(Playback::of(sprite));
        self.sprite_speed = Some(sprite.speed);
        self.flip = Some([sprite.flip_x, sprite.flip_y]);
        self.facing = Some(sprite.facing.clone());
        self.look = Some(sprite.look.clone());
    }

    /// Takes in an AudioSource's part.
    pub(crate) fn read_source(&mut self, source: &AudioSource) {
        self.sounding = Some(source.playing);
    }

    /// Takes in an Animator's parts.
    pub(crate) fn read_animator(&mut self, animator: &Animator) {
        self.animator_state = Some(animator.state.clone());
        self.animator_speed = Some(animator.speed);
    }

    /// Takes in an entity's tags.
    pub(crate) fn read_tags(&mut self, tags: &Tags) {
        self.tags = Some(tags.clone());
    }

    /// Takes in a Screen's part, from its values.
    pub(crate) fn read_screen(&mut self, data: &ComponentData) {
        let screen = Screen::from_data(data);
        self.screen = Some((screen.screen, screen.shown));
    }

    /// The parts `entity` has now.
    fn of(ecs: &hecs::World, entity: hecs::Entity) -> Self {
        let mut set = Self::default();
        if let Ok(sprite) = ecs.get::<&Sprite>(entity) {
            set.read_sprite(&sprite);
        }
        if let Ok(source) = ecs.get::<&AudioSource>(entity) {
            set.read_source(&source);
        }
        if let Ok(animator) = ecs.get::<&Animator>(entity) {
            set.read_animator(&animator);
        }
        if let Ok(values) = ecs.get::<&Values>(entity)
            && let Some(data) = values.0.get(Screen::NAME)
        {
            set.read_screen(data);
        }
        if let Ok(tags) = ecs.get::<&Tags>(entity) {
            set.read_tags(&tags);
        }
        set
    }

    /// Whether `part` is the same in both.
    fn same(&self, other: &Self, part: Part) -> bool {
        match part {
            Part::Playback => self.playback == other.playback,
            Part::SpriteSpeed => self.sprite_speed == other.sprite_speed,
            Part::Flip => self.flip == other.flip,
            Part::Facing => self.facing == other.facing,
            Part::Look => self.look == other.look,
            Part::Sounding => self.sounding == other.sounding,
            Part::AnimatorState => self.animator_state == other.animator_state,
            Part::AnimatorSpeed => self.animator_speed == other.animator_speed,
            Part::Screen => self.screen == other.screen,
            Part::Tags => self.tags == other.tags,
        }
    }

    /// Puts `part` on `entity` as it is here (nothing, where this has
    /// none of it). A Screen is given to an entity without one, as
    /// `showScreen` gives it.
    fn put(&self, part: Part, ecs: &mut hecs::World, entity: hecs::Entity) {
        match part {
            Part::Playback => {
                if let (Some(playback), Ok(mut sprite)) =
                    (&self.playback, ecs.get::<&mut Sprite>(entity))
                {
                    playback.put(&mut sprite);
                }
            }
            Part::SpriteSpeed => {
                if let (Some(speed), Ok(mut sprite)) =
                    (self.sprite_speed, ecs.get::<&mut Sprite>(entity))
                {
                    sprite.speed = speed;
                }
            }
            Part::Flip => {
                if let (Some([across, down]), Ok(mut sprite)) =
                    (self.flip, ecs.get::<&mut Sprite>(entity))
                {
                    sprite.flip_x = across;
                    sprite.flip_y = down;
                }
            }
            Part::Facing => {
                if let (Some(facing), Ok(mut sprite)) =
                    (&self.facing, ecs.get::<&mut Sprite>(entity))
                {
                    sprite.facing.clone_from(facing);
                }
            }
            Part::Look => {
                if let (Some(look), Ok(mut sprite)) = (&self.look, ecs.get::<&mut Sprite>(entity)) {
                    sprite.look.clone_from(look);
                }
            }
            Part::Sounding => {
                if let (Some(playing), Ok(mut source)) =
                    (self.sounding, ecs.get::<&mut AudioSource>(entity))
                {
                    source.playing = playing;
                }
            }
            Part::AnimatorState => {
                if let (Some(state), Ok(mut animator)) =
                    (&self.animator_state, ecs.get::<&mut Animator>(entity))
                {
                    animator.state.clone_from(state);
                }
            }
            Part::AnimatorSpeed => {
                if let (Some(speed), Ok(mut animator)) =
                    (self.animator_speed, ecs.get::<&mut Animator>(entity))
                {
                    animator.speed = speed;
                }
            }
            Part::Screen => {
                if let Some((screen, shown)) = &self.screen {
                    put_screen(ecs, entity, screen, *shown);
                }
            }
            // An entity the game gave its first tag is given a Tags, and
            // one the level gave none, taken back to the level's, none.
            Part::Tags => match &self.tags {
                Some(tags) => drop(ecs.insert_one(entity, tags.clone())),
                None => drop(ecs.remove_one::<Tags>(entity)),
            },
        }
    }
}

/// Makes `entity`'s Screen show `screen` (or not, by `shown`), keeping
/// the rest of the Screen it has; it's given one if it has none.
pub(crate) fn put_screen(ecs: &mut hecs::World, entity: hecs::Entity, screen: &str, shown: bool) {
    let write = |held: Option<&ComponentData>| {
        Screen {
            screen: screen.to_owned(),
            shown,
            ..held.map(Screen::from_data).unwrap_or_default()
        }
        .sanitized()
        .to_data()
    };
    if let Ok(mut values) = ecs.get::<&mut Values>(entity) {
        let data = write(values.0.get(Screen::NAME));
        values.0.insert(Screen::NAME.to_string(), data);
        return;
    }
    let mut values = Values::default();
    values.0.insert(Screen::NAME.to_string(), write(None));
    let _ = ecs.insert_one(entity, values);
}

/// An entity's [`GameSet`] as the level gave it when the world took the
/// entity in. Only entities with one of the components have it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Start(pub(crate) GameSet);

/// What the game made of one entity, taken off it before it's made again
/// from the level.
#[derive(Debug)]
pub(crate) struct Carried {
    id: EntityId,
    /// Its parts as the level gave them, and as the game has them.
    start: GameSet,
    now: GameSet,
    /// Its animator's run and its tile map, which the level doesn't hold.
    run: Option<Running>,
    map: Option<MapState>,
}

impl World {
    /// The parts the level gave `entity` (none for one without any of the
    /// components).
    fn start_of(&self, entity: hecs::Entity) -> GameSet {
        self.ecs
            .get::<&Start>(entity)
            .map(|start| start.0.clone())
            .unwrap_or_default()
    }

    /// Takes off `entity` everything the game made of it that making it
    /// again from the level would lose; nothing, if there's nothing.
    pub(crate) fn carry(&mut self, entity: hecs::Entity) -> Option<Carried> {
        let id = self.ecs.get::<&Id>(entity).ok()?.0;
        let start = self.start_of(entity);
        let now = GameSet::of(&self.ecs, entity);
        let run = self.ecs.remove_one::<Running>(entity).ok();
        let map = self.ecs.remove_one::<MapState>(entity).ok();
        (now != start || run.is_some() || map.is_some()).then_some(Carried {
            id,
            start,
            now,
            run,
            map,
        })
    }

    /// [`Self::carry`] for every entity with anything to carry, before the
    /// whole world is made again. Only entities with one of the
    /// components, a run, a map or a Screen are looked at, so a big world
    /// without them costs nothing here.
    pub(crate) fn carry_all(&mut self) -> Vec<Carried> {
        type Has<'a> = (
            hecs::Entity,
            Option<&'a Start>,
            Option<&'a Running>,
            Option<&'a MapState>,
            Option<&'a Values>,
            Option<&'a Tags>,
        );
        let entities: Vec<hecs::Entity> = self
            .ecs
            .query::<Has<'_>>()
            .iter()
            .filter(|(_, start, run, map, values, tags)| {
                let screen = values.is_some_and(|v| v.0.contains_key(Screen::NAME));
                start.is_some() || run.is_some() || map.is_some() || screen || tags.is_some()
            })
            .map(|(entity, ..)| entity)
            .collect();
        entities
            .into_iter()
            .filter_map(|entity| self.carry(entity))
            .collect()
    }

    /// Puts back what was carried, on the entity made again from the
    /// level: each part the game had changed, while the level still gives
    /// what it gave (an edit that changed it wins); its run and its map,
    /// while it still has an Animator and a TileMap.
    pub(crate) fn put_back(&mut self, carried: Carried) {
        let Some(&entity) = self.by_id.get(&carried.id) else {
            return;
        };
        let fresh = self.start_of(entity);
        for part in Part::ALL {
            if !carried.now.same(&carried.start, part) && fresh.same(&carried.start, part) {
                carried.now.put(part, &mut self.ecs, entity);
            }
        }
        if let Some(run) = carried.run
            && self.ecs.satisfies::<&Animator>(entity)
        {
            let _ = self.ecs.insert_one(entity, run);
        }
        if let Some(map) = carried.map
            && self.ecs.satisfies::<&TileMap>(entity)
        {
            let _ = self.ecs.insert_one(entity, map);
        }
        // A Screen is among the values the structure's sum holds.
        self.structure = std::sync::OnceLock::new();
        self.touched = true;
    }

    /// A field of `id`'s component was set by hand in the editor while
    /// playing: if game code sets that field too, the level's value takes
    /// the place of the game's, even when it's the value the level held
    /// all along (so [`Self::update`] and [`Self::rebuild`], which keep
    /// the game's while the level is unchanged, kept it). Call it after
    /// them. Any other field is the level's already.
    pub fn set_by_hand(&mut self, id: EntityId, component: &str, field: &str) {
        let (Some(part), Some(&entity)) = (Part::of(component, field), self.by_id.get(&id)) else {
            return;
        };
        self.start_of(entity).put(part, &mut self.ecs, entity);
        self.structure = std::sync::OnceLock::new();
        self.touched = true;
    }
}

#[cfg(test)]
#[path = "game_set_tests.rs"]
mod tests;
