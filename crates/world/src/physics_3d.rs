//! 3D physics in the running game: bodies joining as entities do, game
//! code's pushes and moves, stepping it once a tick, and its state for
//! saves.

use crate::{Global, Id, PhysicsAsk, World};
use area::builtin::{CharacterBody, Collider, RigidBody, Transform};
use glam::Mat4;
use reflect::EntityId;

impl World {
    /// What physics knows of an entity: where it's placed and its physics
    /// components.
    pub(crate) fn physics_parts(
        &self,
        entity: hecs::Entity,
    ) -> (
        Option<Transform>,
        Option<RigidBody>,
        Option<Collider>,
        Option<CharacterBody>,
    ) {
        (
            self.ecs.get::<&Transform>(entity).ok().map(|c| *c),
            self.ecs.get::<&RigidBody>(entity).ok().map(|c| *c),
            self.ecs.get::<&Collider>(entity).ok().map(|c| *c),
            self.ecs.get::<&CharacterBody>(entity).ok().map(|c| *c),
        )
    }

    /// The game's physics (decision 0036), to read: game code reads its
    /// state on every core during a tick.
    pub fn physics(&self) -> &physics::Physics {
        &self.physics
    }

    /// Makes `id`'s part in physics match its components and where it is
    /// now: a body and collider if it has them, nothing if not.
    pub(crate) fn physics_join(&mut self, id: EntityId) {
        self.flat_join(id);
        let Some(&entity) = self.by_id.get(&id) else {
            self.physics.remove(id);
            return;
        };
        let global = self
            .ecs
            .get::<&Global>(entity)
            .map_or(Mat4::IDENTITY, |g| g.0);
        if let Ok(c) = self.ecs.get::<&CharacterBody>(entity).map(|c| *c) {
            let (_, rotation, position) = global.to_scale_rotation_translation();
            let spec = physics::CharacterSpec {
                height: c.height,
                radius: c.radius,
                step: c.step,
                slope: c.slope,
                snap: c.snap,
                mass: c.mass,
            };
            self.physics.add_character(id, (position, rotation), spec);
            self.touched = true;
            return;
        }
        let body = self.ecs.get::<&RigidBody>(entity).ok().map(|b| *b);
        let collider = self.ecs.get::<&Collider>(entity).ok().map(|c| *c);
        if body.is_none() && collider.is_none() {
            self.physics.remove(id);
            return;
        }
        let (scale, rotation, position) = global.to_scale_rotation_translation();
        let body = body.map(|b| physics::Body {
            kind: match b.kind {
                area::builtin::BodyKind::Dynamic => physics::BodyKind::Dynamic,
                area::builtin::BodyKind::Kinematic => physics::BodyKind::Kinematic,
                area::builtin::BodyKind::Fixed => physics::BodyKind::Fixed,
            },
            mass: b.mass,
            linear_damping: b.linear_damping,
            angular_damping: b.angular_damping,
            gravity: b.gravity,
            upright: b.upright,
            fast: b.fast,
        });
        let collider = collider.map(|c| physics::ColliderSpec {
            shape: match c.shape {
                area::builtin::ColliderShape::Box => physics::Shape::Box(c.size),
                area::builtin::ColliderShape::Sphere => physics::Shape::Sphere(c.radius),
                area::builtin::ColliderShape::Capsule => physics::Shape::Capsule {
                    radius: c.radius,
                    height: c.height,
                },
            },
            center: c.center,
            friction: c.friction,
            bounciness: c.bounciness,
        });
        self.physics
            .add(id, (position, rotation), scale, body, collider);
        self.touched = true;
    }

    /// Gives `id`'s body a push (kilograms times metres a second), at the
    /// next physics step.
    pub fn push(&mut self, id: EntityId, impulse: glam::Vec3) {
        self.physics_asked.push(PhysicsAsk::Push(id, impulse));
    }

    /// Sets how fast `id`'s body moves (metres a second), at the next
    /// physics step.
    pub fn set_velocity(&mut self, id: EntityId, velocity: glam::Vec3) {
        self.physics_asked.push(PhysicsAsk::Velocity(id, velocity));
    }

    /// Moves the character `id` (a `CharacterBody`) by `wanted` metres,
    /// as far as it can, at the next physics step (see
    /// `physics::Physics::move_character`).
    pub fn move_character(&mut self, id: EntityId, wanted: glam::Vec3) {
        // A 2D character (a `Collider2D` ticked Character) walks on the
        // ground: its move's X and Y.
        if self.flat.state().is_character(id) {
            self.physics_asked
                .push(PhysicsAsk::Flat(id, wanted.truncate()));
        } else {
            self.physics_asked.push(PhysicsAsk::Character(id, wanted));
        }
    }

    /// Bodies not made because there were already the most
    /// (`physics::MAX_BODIES`).
    pub fn physics_refused(&self) -> u64 {
        self.physics.refused()
    }

    /// Moves physics on by one tick: bodies follow what game code moved,
    /// its pushes and velocities are made, physics steps, and the
    /// entities physics moved follow their bodies (those under them too).
    /// The editor calls it once per tick, after game code's changes land.
    pub fn step_physics(&mut self) {
        // What game code moved, in the order moved, each once.
        let mut moved = std::mem::take(&mut self.physics_moved);
        let mut seen = reflect::IdSet::default();
        moved.retain(|id| seen.insert(*id));
        let asked = std::mem::take(&mut self.physics_asked);
        // 2D physics first (decision 0071): its own world beside 3D's.
        let walks: Vec<(EntityId, glam::Vec2)> = asked
            .iter()
            .filter_map(|ask| match ask {
                PhysicsAsk::Flat(id, wanted) => Some((*id, *wanted)),
                _ => None,
            })
            .collect();
        self.step_flat(&moved, &walks);
        let stepped = self.step_bodies(&moved, asked);
        // The one way out, whichever physics moved something and however
        // far the 3D step got: those under what moved follow it now, so
        // nothing is drawn or summed a tick behind what carries it. A
        // dynamic body under something stays physics' to move only when
        // 3D physics stepped.
        self.after_physics = stepped;
        self.settle();
        self.after_physics = false;
    }

    /// 3D physics' part of [`Self::step_physics`]: bodies go where game
    /// code `moved` their entities, what it `asked` for is made, physics
    /// steps, and the entities it moved follow their bodies (those under
    /// them wait for the caller to settle). Returns whether physics
    /// stepped: not with nothing in it, nor while the game is paused.
    fn step_bodies(&mut self, moved: &[EntityId], asked: Vec<PhysicsAsk>) -> bool {
        if self.physics.state().is_empty() {
            return false;
        }
        self.touched = true;
        for &id in moved {
            if let Some(global) = self.global(id) {
                let (_, rotation, position) = global.to_scale_rotation_translation();
                self.physics.set_pose(id, position, rotation);
            }
        }
        // Pushes and velocities in the order asked, then characters' moves
        // in the order asked (worked out on every core at once).
        let mut characters = Vec::new();
        for ask in asked {
            match ask {
                PhysicsAsk::Push(id, impulse) => self.physics.push(id, impulse),
                PhysicsAsk::Velocity(id, velocity) => self.physics.set_velocity(id, velocity),
                PhysicsAsk::Character(id, wanted) => characters.push((id, wanted)),
                PhysicsAsk::Flat(..) => {}
            }
        }
        // At the game's speed; paused, bodies stay where they are (game
        // code's own moves above still land).
        let dt = self.tick_time() as f32;
        if dt <= 0.0 {
            return false;
        }
        if !characters.is_empty() {
            self.physics.move_characters(&characters, dt);
        }
        self.physics.step(dt);
        // The entities follow their bodies, without sending it back.
        self.following = true;
        for (id, position, rotation) in self.physics.moved() {
            self.change_transform(id, |parent, mut t| {
                if parent == Mat4::IDENTITY {
                    t.position = position;
                    t.rotation = rotation;
                } else {
                    t.position = parent.inverse().transform_point3(position);
                    let turn = parent.to_scale_rotation_translation().1;
                    t.rotation = (turn.inverse() * rotation).normalize();
                }
                (t.position.is_finite() && t.rotation.is_finite()).then_some(t)
            });
        }
        self.following = false;
        true
    }

    /// Every body's place and motion (`physics::BodyState`), in entity
    /// order: what a save game keeps of physics (decision 0038).
    pub fn physics_states(&self) -> Vec<(EntityId, physics::BodyState)> {
        self.physics.state().body_states()
    }

    /// Builds physics again from scratch, every entity joining in ID
    /// order, then gives the bodies in `states` their motion, and their
    /// places too `with_place`. Nothing of Rapier's own (contacts,
    /// islands) is kept, so a save game, which builds the running game's
    /// physics again this way as it saves and the loaded game's as it
    /// loads, carries on exactly alike both ways, and nothing in a file
    /// can reach Rapier's insides.
    pub fn rebuild_physics(&mut self, states: &[(EntityId, physics::BodyState)], with_place: bool) {
        self.join_all();
        for (id, state) in states {
            self.physics.set_body_state(*id, *state, with_place);
        }
    }

    /// Empties physics and joins every entity again, in ID order.
    pub(crate) fn join_all(&mut self) {
        self.physics.clear();
        self.flat.clear();
        self.walls_built.clear();
        let mut ids: Vec<EntityId> = self
            .order
            .iter()
            .filter_map(|(e, _)| self.ecs.get::<&Id>(*e).ok().map(|i| i.0))
            .collect();
        ids.sort_unstable();
        for id in ids {
            self.physics_join(id);
        }
        self.physics_moved.clear();
        self.touched = true;
    }

    /// Whether physics broke (Rapier panicked on a damaged save) and was
    /// emptied since last asked.
    pub fn take_physics_broke(&mut self) -> bool {
        self.physics.take_broke()
    }
}
