//! What game code can ask of the engine: the [`Host`] that answers the
//! native methods described in `solis/*.lst`, and lending it to Luau as
//! the `LUST_NATIVES` table for exactly as long as game code runs.

use crate::convert::QuatKeys;
use crate::luau::Luau;
use glam::{Quat, Vec3};
use mlua::{Table, Value as LuaValue};
use reflect::EntityId;
use std::cell::{Cell, RefCell};

/// Where a sound game code plays is heard from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoundAt {
    /// Not placed: the same everywhere (a 3D event plays at the listener).
    Nowhere,
    /// At a place in the world.
    Place(Vec3),
    /// On an entity, following it as it moves.
    On(EntityId),
}

/// Why a host with no running game keeps no timers.
const NO_TIMERS: &str = "timers run only while the game plays";

/// What game code can ask of the engine: the bodies of the native methods
/// described in `solis/*.lst` ([`crate::describe::NATIVES`]). Entities
/// that aren't in the game read as nothing, and changing them does
/// nothing.
pub trait Host {
    /// Writes words in the Console.
    fn say(&mut self, words: &str);
    /// How long the game has been running, in seconds of game time.
    fn time(&self) -> f64;
    /// The first entity with this name, in the level's tree order.
    fn find(&self, name: &str) -> Option<EntityId>;
    /// Where an entity is in the world.
    fn position(&self, which: EntityId) -> Option<Vec3>;
    /// Puts an entity at a place in the world.
    fn move_to(&mut self, which: EntityId, place: Vec3);
    /// Which way an entity faces in the world.
    fn rotation(&self, which: EntityId) -> Option<Quat>;
    /// Turns an entity to face this way in the world.
    fn turn_to(&mut self, which: EntityId, facing: Quat);
    /// An entity's name.
    fn name(&self, which: EntityId) -> Option<String>;
    /// Fires a projectile. A host without projectiles ignores it.
    fn fire(&mut self, shot: Shot) {
        let _ = shot;
    }
    /// Makes a copy of the module named `module` (its file in the
    /// project's assets) at `place`, and gives back its entity, which is
    /// in the game from the next tick; `None` when it can't be made.
    fn spawn(&mut self, module: &str, place: Vec3) -> Option<EntityId> {
        let _ = (module, place);
        None
    }
    /// [`Self::spawn`], its root's components given `values` before their
    /// first update (Fixes 2b, `World:spawnWith`). A host that can't give
    /// values spawns without them.
    fn spawn_with(
        &mut self,
        module: &str,
        place: Vec3,
        values: crate::game::SpawnValues,
    ) -> Option<EntityId> {
        let _ = values;
        self.spawn(module, place)
    }
    /// Takes an entity (and everything under it) out of the game, at the
    /// end of the tick.
    fn despawn(&mut self, which: EntityId) {
        let _ = which;
    }
    /// Whether the entity is in the game, as the tick started (Fixes 2): a
    /// spawned one from the tick after it was asked for, a despawned one
    /// until the tick it was despawned in ends.
    fn exists(&self, which: EntityId) -> bool {
        self.position(which).is_some()
    }
    /// The components game code reads, as the tick started (Fixes 2a): none
    /// while no game runs.
    fn readings(&self) -> Option<std::sync::Arc<crate::Readings>> {
        None
    }
    /// The one entity with the component `component`, as the tick started
    /// (`World:the`), or why there isn't one.
    fn the(&self, component: &str) -> Result<EntityId, String> {
        let readings = self.readings().unwrap_or_default();
        readings
            .the(component)
            .map_err(|found| crate::readings::not_one(component, found))
    }
    /// Turns the entity's collider (2D or 3D) on or off at the end of the
    /// tick (Fixes 2c): off, nothing bumps into it, rays pass through and
    /// trigger zones don't count it.
    fn set_solid(&mut self, which: EntityId, solid: bool) {
        let _ = (which, solid);
    }
    /// Shows or hides the entity and everything under it at the end of the
    /// tick (Fixes 2c): only its drawing; physics and game code go on.
    fn set_visible(&mut self, which: EntityId, visible: bool) {
        let _ = (which, visible);
    }
    /// How long the last frame took, in seconds of real time.
    fn frame_time(&self) -> f64 {
        1.0 / 60.0
    }
    /// A random number from 0 up to (not including) 1, the same for the
    /// same seed and the same place in the tick's order. A host without
    /// randomness gives a half.
    fn random(&mut self) -> f64 {
        0.5
    }
    /// Starts the game's randomness again from `seed` (at the end of the
    /// tick, like a move).
    fn set_seed(&mut self, seed: u64) {
        let _ = seed;
    }
    /// Gives an entity's body a push (kilograms times metres a second),
    /// made at the next physics step.
    fn push(&mut self, which: EntityId, impulse: Vec3) {
        let _ = (which, impulse);
    }
    /// How fast an entity's body moves, metres a second (none without a
    /// body).
    fn velocity(&self, which: EntityId) -> Option<Vec3> {
        let _ = which;
        None
    }
    /// Sets how fast an entity's body moves, at the next physics step.
    fn set_velocity(&mut self, which: EntityId, velocity: Vec3) {
        let _ = (which, velocity);
    }
    /// Walks a character (an entity with a `CharacterBody`, or a
    /// `Collider2D` ticked Character) by `wanted` metres, as far as it can,
    /// at the next physics step (`walk(wanted)` in Lust).
    fn walk(&mut self, which: EntityId, wanted: Vec3) {
        let _ = (which, wanted);
    }
    /// Whether a character stood on something after its last move
    /// (`grounded()` in Lust).
    fn grounded(&self, which: EntityId) -> bool {
        let _ = which;
        false
    }
    /// Who is inside the trigger zone `zone` as the tick started, in the
    /// order of their IDs (`inside()` in Lust); none for an entity that
    /// isn't a zone.
    fn inside(&self, zone: EntityId) -> Vec<EntityId> {
        let _ = zone;
        Vec::new()
    }
    /// Whether an entity carried the tag called `tag` (`Enemy`) as the
    /// tick started (`hasTag(Tag.Enemy)` in Lust).
    fn has_tag(&self, which: EntityId, tag: &str) -> bool {
        let _ = (which, tag);
        false
    }
    /// Gives an entity the tag called `tag` at the end of the tick
    /// (`addTag`).
    fn add_tag(&mut self, which: EntityId, tag: &str) {
        let _ = (which, tag);
    }
    /// Takes the tag called `tag` off an entity at the end of the tick
    /// (`removeTag`).
    fn remove_tag(&mut self, which: EntityId, tag: &str) {
        let _ = (which, tag);
    }
    /// Every entity carrying the tag called `tag` within `radius` metres
    /// of `place` as the tick started, nearest first, ties by ID
    /// (`World:within`).
    fn within(&self, tag: &str, place: Vec3, radius: f32) -> Vec<EntityId> {
        let _ = (tag, place, radius);
        Vec::new()
    }
    /// The entity carrying the tag called `tag` nearest to `place` as the
    /// tick started, ties by ID (`World:nearest`); none when none does.
    fn nearest(&self, tag: &str, place: Vec3) -> Option<EntityId> {
        let _ = (tag, place);
        None
    }
    /// Starts the running component's timer called `name` (`DoorOpens`)
    /// at the end of the tick (stage 1, timers; `startTimer` and its
    /// variants). `which` must be the component's own entity: a timer
    /// belongs to the component that starts it. The error says, in plain
    /// words, why it can't be started.
    fn start_timer(
        &mut self,
        which: EntityId,
        name: &str,
        timing: crate::timers::Timing,
    ) -> Result<(), String> {
        let _ = (which, name, timing);
        Err(NO_TIMERS.to_string())
    }
    /// Stops the running component's timer called `name` at the end of
    /// the tick (`stopTimer`); one not running changes nothing.
    fn stop_timer(&mut self, which: EntityId, name: &str) -> Result<(), String> {
        let _ = (which, name);
        Err(NO_TIMERS.to_string())
    }
    /// The seconds of game time left on the running component's timer
    /// called `name` as the tick started (`timeLeft`), or none when it
    /// isn't running (`timerRunning` is whether there is a time).
    fn timer_left(&self, which: EntityId, name: &str) -> Result<Option<f64>, String> {
        let _ = (which, name);
        Err(NO_TIMERS.to_string())
    }
    /// `keepStarted()` (stage 1, states): what the running component
    /// started in its own state so far (timers, tweens, looping sounds)
    /// no longer belongs to it, and carries on when it changes. `which`
    /// must be the component's own entity. A Rust component has no state
    /// of its own, so nothing of its belongs to one.
    fn keep_started(&mut self, which: EntityId) -> Result<(), String> {
        let _ = which;
        Ok(())
    }
    /// What the next timer or sound the running component starts belongs
    /// to: the option of its own state (stage 1, states), said by Solis's
    /// natives just before they start one; none for a component without a
    /// state of its own. Rust components never need it.
    fn set_started_in(&mut self, option: Option<std::sync::Arc<str>>) {
        let _ = option;
    }
    /// The component `component` on `which` changed its own state from
    /// `from` to `to`: what it started in the options it left ends, and
    /// it hears the change as the tick ends (stage 1, states). Said by
    /// Lust's runtime whenever a Lust component's state changes; Rust
    /// components have no state of their own.
    fn state_changed(&mut self, which: EntityId, component: &str, from: &str, to: &str) {
        let _ = (which, component, from, to);
    }
    /// Saves the game in `slot` (or, with `load`, loads the game saved
    /// there), at the end of the tick (decision 0038).
    fn save_game(&mut self, slot: &str, load: bool) {
        let _ = (slot, load);
    }
    /// An action's state this tick (decision 0037): at rest for a host
    /// without input.
    fn action(&self, name: &str) -> input::ActionState {
        let _ = name;
        input::ActionState::default()
    }
    /// The place in the world under the pointer, over a 2D game's picture
    /// (the action game, A1); the middle of the view before it's been
    /// over it. Nowhere in particular for a host without input.
    fn pointer(&self) -> Vec3 {
        Vec3::ZERO
    }
    /// Whether game code asked for the pointer shown and free over the
    /// game (`Input.pointerShown`), as the tick started.
    fn pointer_shown(&self) -> bool {
        false
    }
    /// Shows the pointer and frees it (true), or hides it and holds it
    /// while the game has the mouse, at the end of the tick.
    fn set_pointer_shown(&mut self, shown: bool) {
        let _ = shown;
    }
    /// The input that sets an action off (a key first, then the mouse,
    /// then the pad), for telling a player which to press.
    fn binding(&self, action: &str) -> Option<String> {
        let _ = action;
        None
    }
    /// Binds `input` to `action` from the next tick, in place of its
    /// inputs of the same kind.
    fn rebind(&mut self, action: &str, input: &str) {
        let _ = (action, input);
    }
    /// Plays the sound event named `event` (its file in the project's
    /// assets, without `.solsound`), at the end of the tick. Nothing comes
    /// back: sound never changes the game (decision 0056).
    fn play_sound(&mut self, event: &str, at: SoundAt) {
        let _ = (event, at);
    }
    /// Stops the sounds playing on an entity, with a short fade, at the
    /// end of the tick.
    fn stop_sounds_on(&mut self, which: EntityId) {
        let _ = which;
    }
    /// Turns the entity's `AudioSource` on or off, at the end of the tick.
    fn set_sound_playing(&mut self, which: EntityId, playing: bool) {
        let _ = (which, playing);
    }
    /// Shows (`shown`) or hides the screen named `screen` on the entity's
    /// Screen, at the end of the tick (decision 0087).
    fn show_screen(&mut self, which: EntityId, screen: &str, shown: bool) {
        let _ = (which, screen, shown);
    }
    /// Shows all the text the entity's Screen named `screen` is typing out,
    /// at the end of the tick (stage 1, text effects): `TypingFinished`
    /// follows.
    fn finish_typing(&mut self, which: EntityId, screen: &str) {
        let _ = (which, screen);
    }
    /// Whether the entity's Screen shows the screen named `screen`, as the
    /// tick started.
    fn screen_shown(&self, which: EntityId, screen: &str) -> bool {
        let _ = (which, screen);
        false
    }
    /// Plays a clip on the entity's Sprite at the end of the tick (Phase
    /// 9, D3): already playing it (or finished it), it carries on as it
    /// is; another clip starts from its beginning. With `again`
    /// (`animateFromStart`), it starts from its beginning whatever it was doing.
    fn play_clip(&mut self, which: EntityId, clip: &str, again: bool) {
        let _ = (which, clip, again);
    }
    /// Stops the entity's Sprite where it is in its clip, at the end of
    /// the tick.
    fn stop_clip(&mut self, which: EntityId) {
        let _ = which;
    }
    /// Mirrors the entity's Sprite at the end of the tick: `across`
    /// faces it the other way (left for right), `down` turns it upside
    /// down.
    fn set_flip(&mut self, which: EntityId, across: bool, down: bool) {
        let _ = (which, across, down);
    }
    /// Sets a bus's volume for the rest of the play, as a share of its
    /// Mixer fader (1 as set, 0 silent), at the end of the tick: an
    /// options menu's sliders.
    fn set_bus_volume(&mut self, bus: &str, share: f32) {
        let _ = (bus, share);
    }
    /// Sets a parameter of `which`'s animator, by its name in the graph,
    /// at the end of the tick (an animator's own Lust file calls it).
    fn animator_set(&mut self, which: EntityId, name: &str, value: AnimatorValue) {
        let _ = (which, name, value);
    }
    /// The state `which`'s animator is in, by its name in the graph, as
    /// the tick started.
    fn animator_state(&self, which: EntityId) -> Option<String> {
        let _ = which;
        None
    }
    /// Puts `which`'s animator straight into the state called `state`, at
    /// the end of the tick.
    fn animator_jump(&mut self, which: EntityId, state: &str) {
        let _ = (which, state);
    }
    /// How fast `which`'s animator plays its clips (1 as made), from the
    /// end of the tick.
    fn animator_speed(&mut self, which: EntityId, scale: f32) {
        let _ = (which, scale);
    }
    /// The tile map at play from the file `file` (its path without
    /// `.tilemap.lst`), as the tick started: the first entity showing it.
    fn map(&self, file: &str) -> Option<std::sync::Arc<tiles::TileMap>> {
        let _ = file;
        None
    }
    /// Paints a layer's cell of the map from `file` with a terrain's code
    /// (0 clears it), at the end of the tick.
    fn set_map_tile(&mut self, file: &str, layer: usize, cell: [u32; 2], code: u8) {
        let _ = (file, layer, cell, code);
    }
    /// Sets a value on a cell of the map from `file`, at the end of the
    /// tick.
    fn set_map_value(&mut self, file: &str, value: usize, cell: [u32; 2], to: tiles::CellValue) {
        let _ = (file, value, cell, to);
    }
    /// The first collider a ray meets (see `physics::State::cast_ray`).
    fn cast_ray(&self, from: Vec3, direction: Vec3, reach: f32) -> Option<physics::Hit> {
        let _ = (from, direction, reach);
        None
    }

    /// Where the tile map from the file `file` lies in the world, as the
    /// tick started (decision 0082).
    fn map_place(&self, file: &str) -> Option<MapPlace> {
        let _ = file;
        None
    }
    /// The box a tile map on `which` covers in the world (its lowest and
    /// highest corners), as the tick started.
    fn map_bounds(&self, which: EntityId) -> Option<(Vec3, Vec3)> {
        let _ = which;
        None
    }
    /// How much of the world a 2D camera shows, in metres (x across, y
    /// up); nothing for a host without 2D settings.
    fn view_size(&self) -> Vec3 {
        Vec3::ZERO
    }
    /// Sets how fast the game runs, at the end of the tick: 0 pauses time,
    /// physics, projectiles and sprites; game code still runs.
    fn set_game_speed(&mut self, speed: f64) {
        let _ = speed;
    }
    /// How fast the game runs, as the tick started (1 as it starts).
    fn game_speed(&self) -> f64 {
        1.0
    }

    /// The game's hour as the tick started (decision 0092): 0 up to 24.
    fn time_of_day(&self) -> f64 {
        8.0
    }
    /// Sets the game's hour, at the end of the tick.
    fn set_time_of_day(&mut self, hour: f64) {
        let _ = hour;
    }

    /// The player's settings, as the tick started (decision 0091).
    fn settings(&self) -> crate::settings::PlayerSettings {
        crate::settings::PlayerSettings::default()
    }
    /// Changes one of the player's settings, at the end of the tick.
    fn set_setting(&mut self, change: crate::settings::Setting) {
        let _ = change;
    }

    /// `Screens:textSize()`.
    fn text_size(&self) -> f64 {
        self.settings().text_size
    }
    /// `Screens:setTextSize(size)`.
    fn set_text_size(&mut self, size: f64) {
        self.set_setting(crate::settings::Setting::TextSize(size));
    }
    /// `Screens:uiSize()`.
    fn ui_size(&self) -> f64 {
        self.settings().ui_size
    }
    /// `Screens:setUiSize(size)`.
    fn set_ui_size(&mut self, size: f64) {
        self.set_setting(crate::settings::Setting::UiSize(size));
    }
    /// `Window:fullscreen()`.
    fn fullscreen(&self) -> bool {
        self.settings().fullscreen
    }
    /// `Window:setFullscreen(on)`.
    fn set_fullscreen(&mut self, on: bool) {
        self.set_setting(crate::settings::Setting::Fullscreen(on));
    }
    /// `Window:resolution()`, in pixels.
    fn resolution(&self) -> [u32; 2] {
        self.settings().resolution
    }
    /// `Window:setResolution(width, height)`.
    fn set_resolution(&mut self, width: u32, height: u32) {
        self.set_setting(crate::settings::Setting::Resolution(width, height));
    }
    /// `Window:useNextResolution()`.
    fn use_next_resolution(&mut self) {
        let [w, h] = self.settings().next_resolution();
        self.set_resolution(w, h);
    }
    /// `Window:quit()`: closes the game at the end of the tick (decision
    /// 0093). While playing in the editor, play stops.
    fn quit(&mut self) {}

    /// The project's data tables, read-only (stage 1; decision 0128):
    /// `tables().get("Items")` and its `row("IronSword")`, whose
    /// `whole("price")` is a cell. None by default.
    fn tables(&self) -> crate::Tables {
        crate::Tables::default()
    }

    /// The game's text in the player's language (stage 1, translations):
    /// `texts().text("Shop.Welcome")`. None when the project has no text.
    fn texts(&self) -> Option<translations::Texts> {
        None
    }

    /// `World:text(key)`: a line of the game's text in the player's
    /// language (the main language's where it isn't translated), or the
    /// key itself when there's no such line.
    fn text(&self, key: &str) -> String {
        self.texts().map_or_else(|| key.to_owned(), |t| t.text(key))
    }

    /// A line of the game's text that takes values, with them
    /// (`("name", TextValue::Text("Ada".into()))`).
    fn text_with(&self, key: &str, values: &[(&str, translations::TextValue)]) -> String {
        self.texts()
            .map_or_else(|| key.to_owned(), |t| t.text_with(key, values))
    }

    /// The component about to run, and where what it does belongs in the
    /// tick's order ([decision 0033](../../../docs/decisions/0033-game-code-sees-the-start-of-the-tick.md)).
    /// A host that keeps changes to land later (game code on several
    /// cores) uses it; one that makes them at once needn't.
    fn at(&mut self, order: &crate::ticking::Order) {
        let _ = order;
    }
}

/// A value game code gives an animator's parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimatorValue {
    /// A number.
    Number(f32),
    /// On or off.
    Switch(bool),
    /// A trigger, fired.
    Fire,
    /// A direction on the ground (z is left out).
    Direction(Vec3),
}

/// The upright rotation that turns +Y (an entity's forward) to point
/// along `direction`: turned like a compass, then tipped up or down; no
/// turn for no direction.
pub fn facing_along(direction: Vec3) -> Quat {
    let flat = direction.truncate().length();
    if !direction.is_finite() || direction.length_squared() < 1e-12 {
        return Quat::IDENTITY;
    }
    let compass = (-direction.x).atan2(direction.y);
    let tip = direction.z.atan2(flat);
    Quat::from_euler(glam::EulerRot::ZXY, compass, tip, 0.0)
}

/// Where a tile map lies in the world (decision 0082): its entity's
/// transform, a cell's size in metres, and its size in cells. Its top
/// left corner is at the entity; columns go along x, rows down y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapPlace {
    /// The entity's transform.
    pub transform: glam::Mat4,
    /// A cell's size in metres, across and down.
    pub cell: [f32; 2],
    /// The map's size in cells.
    pub size: [u32; 2],
}

impl MapPlace {
    /// The cell at `place` (column and row, even off the map), and whether
    /// it's on the map.
    pub fn cell_at(&self, place: Vec3) -> ([i64; 2], bool) {
        let local = self.transform.inverse().transform_point3(place);
        let (x, y) = (
            (local.x / self.cell[0]).floor(),
            (-local.y / self.cell[1]).floor(),
        );
        let far = 1.0e9;
        if !(x.is_finite() && y.is_finite() && x.abs() < far && y.abs() < far) {
            return ([0, 0], false);
        }
        let (x, y) = (x as i64, y as i64);
        let [w, h] = self.size;
        (
            [x, y],
            (0..i64::from(w)).contains(&x) && (0..i64::from(h)).contains(&y),
        )
    }

    /// The place in the world at the middle of the cell `[x, y]`.
    pub fn place_of(&self, x: i64, y: i64) -> Vec3 {
        self.transform.transform_point3(Vec3::new(
            (x as f32 + 0.5) * self.cell[0],
            -(y as f32 + 0.5) * self.cell[1],
            0.0,
        ))
    }

    /// The box the map covers: its lowest and highest corners.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let [w, h] = self.size;
        let a = self.transform.transform_point3(Vec3::ZERO);
        let b = self.transform.transform_point3(Vec3::new(
            w as f32 * self.cell[0],
            -(h as f32) * self.cell[1],
            0.0,
        ));
        (a.min(b), a.max(b))
    }
}

/// A projectile game code fires (`fire`): where from, which way, how fast
/// (metres a second), how long it lives (seconds), and its colour (linear
/// RGB; above 1 glows).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    /// Where it starts.
    pub from: Vec3,
    /// Which way it flies.
    pub direction: Vec3,
    /// How fast.
    pub speed: f32,
    /// How long it lives.
    pub life: f32,
    /// Its colour.
    pub colour: [f32; 3],
}

/// A host for when nothing is running (loading code to read its
/// components): it knows no entities, and words said go nowhere.
#[derive(Debug, Default)]
pub struct NoHost;

impl Host for NoHost {
    fn say(&mut self, _words: &str) {}
    fn time(&self) -> f64 {
        0.0
    }
    fn find(&self, _name: &str) -> Option<EntityId> {
        None
    }
    fn position(&self, _which: EntityId) -> Option<Vec3> {
        None
    }
    fn move_to(&mut self, _which: EntityId, _place: Vec3) {}
    fn rotation(&self, _which: EntityId) -> Option<Quat> {
        None
    }
    fn turn_to(&mut self, _which: EntityId, _facing: Quat) {}
    fn name(&self, _which: EntityId) -> Option<String> {
        None
    }
}

/// A whole number from `low` to `high` (both included, either way round)
/// for a random `x` from 0 to 1; `low` when they aren't sensible numbers.
pub(crate) fn random_int(low: f64, high: f64, x: f64) -> f64 {
    if !(low.is_finite() && high.is_finite()) {
        return 0.0;
    }
    let (low, high) = (low.min(high).ceil(), low.max(high).floor());
    if high < low {
        return low;
    }
    (low + ((high - low + 1.0) * x).floor()).min(high)
}

/// The most bytes of one line game code says; the rest is cut.
pub const MAX_SAID_BYTES: usize = 2_000;
/// The most lines game code says while the natives are lent once (a tick,
/// a load); the rest are counted, not shown.
pub const MAX_SAID_LINES: usize = 200;

/// Runs `run` with the natives lent from `host` and the time limit
/// watching it.
pub(crate) fn call_with<R>(
    luau: &Luau,
    lust: Option<&Table>,
    host: &mut dyn Host,
    send: Option<&mlua::Function>,
    run: impl FnOnce() -> mlua::Result<R>,
) -> mlua::Result<R> {
    with_natives(luau, lust, host, send, |_| luau.clock.limited(run))
}

/// Lends each renamed native under its old name too: the same function.
fn lend_old_names(natives: &Table, renamed: &[crate::describe::Renamed]) -> mlua::Result<()> {
    for r in renamed {
        let same: LuaValue = natives.raw_get(r.new)?;
        natives.raw_set(r.old, same)?;
    }
    Ok(())
}

/// The host while game code runs: shared by Lust's natives and, between
/// Lua calls, Rust components.
pub(crate) type Lent<'a, 'h> = RefCell<&'a mut (dyn Host + 'h)>;

/// Runs `run` with the natives filled from `host` (and `send` as their
/// `send_event`) for its length: as `LUST_NATIVES` in the Lust's globals
/// `lust`, and for Lua's `solis`. `run` is given the host too, for Rust
/// components to borrow between Lua calls. Past [`MAX_SAID_LINES`], what
/// game code says is counted, and the count said at the end. A native
/// that takes several values is made with
/// [`native`](crate::native_values::native), under its own key,
/// so a value of the wrong kind (from Lua: Lust is checked first) is a
/// mistake that names the call.
pub(crate) fn with_natives<'h, R>(
    luau: &Luau,
    lust: Option<&Table>,
    host: &mut (dyn Host + 'h),
    send: Option<&mlua::Function>,
    run: impl FnOnce(&Lent<'_, 'h>) -> mlua::Result<R>,
) -> mlua::Result<R> {
    let lua = &luau.lua;
    let host: Lent<'_, 'h> = RefCell::new(host);
    let lines = Cell::new(0usize);
    // Made once for all the rotations crossing while the natives are lent.
    let keys = &QuatKeys::new(lua)?;
    let result = lua.scope(|scope| {
        let natives = lua.create_table()?;
        let h = &host;
        let said = &lines;
        crate::natives::lend(scope, &natives, h, keys, said)?;
        // What game code makes with new (the uniformity pass, U6).
        crate::made::lend(scope, lua, &natives, h)?;
        // The data tables' natives: made once for each set of tables.
        let (tables, texts) = {
            let host = h.borrow();
            (host.tables(), host.texts())
        };
        luau.lend_tables(tables, texts, &natives)?;
        // Other entities' components as the tick started (Fixes 2).
        let readings = h.borrow().readings();
        luau.lend_readings(readings, &natives)?;
        // A native renamed on purpose keeps working under its old name for
        // a while (Phase 8, C3): the same function, lent twice.
        lend_old_names(&natives, crate::describe::RENAMED)?;
        if let Some(send) = send {
            natives.raw_set("send_event", send.clone())?;
        }
        if let Some(lust) = lust {
            lust.raw_set("LUST_NATIVES", &natives)?;
        }
        luau.lend(LuaValue::Table(natives))?;
        let result = run(&host);
        if let Some(lust) = lust {
            lust.raw_set("LUST_NATIVES", LuaValue::Nil)?;
        }
        luau.lend(LuaValue::Nil)?;
        result
    });
    let more = lines.get().saturating_sub(MAX_SAID_LINES);
    if more > 0 {
        host.borrow_mut().say(&format!(
            "(Game code said {more} more lines just now; only the first {MAX_SAID_LINES} are shown.)"
        ));
    }
    result
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
