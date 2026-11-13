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

/// What game code can ask of the engine: the bodies of the native methods
/// described in `solis/*.lst` ([`crate::describe::NATIVES`]). Entities
/// that aren't in the game read as nothing, and changing them does
/// nothing.
pub trait Host {
    /// Writes words in the Console.
    fn say(&mut self, words: &str);
    /// How long the game has been running, in seconds of game time.
    fn time(&self) -> f64;
    /// The first entity with this name, in the area's tree order.
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
    /// Makes a copy of the assembly named `assembly` (its file in the
    /// project's assets) at `place`, and gives back its entity, which is
    /// in the game from the next tick; `None` when it can't be made.
    fn spawn(&mut self, assembly: &str, place: Vec3) -> Option<EntityId> {
        let _ = (assembly, place);
        None
    }
    /// Takes an entity (and everything under it) out of the game, at the
    /// end of the tick.
    fn despawn(&mut self, which: EntityId) {
        let _ = which;
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
