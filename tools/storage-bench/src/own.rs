//! A candidate storage of Solis's own, in safe Rust: one sparse set per
//! component type. Each component's values sit packed in a vector, with
//! the entity of each; a sparse index finds an entity's slot in O(1).
//! Components made while the program runs (Lua, Lust after a rebuild)
//! would be a column of plain values keyed by name.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// An entity: a slot and a generation, so a reused slot isn't mistaken
/// for the old entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}

/// One component type's values, packed.
pub struct Column<T> {
    values: Vec<T>,
    owners: Vec<u32>,
    /// Slot of each entity index in `values`, or `u32::MAX`.
    slots: Vec<u32>,
}

impl<T> Default for Column<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            owners: Vec::new(),
            slots: Vec::new(),
        }
    }
}

impl<T> Column<T> {
    fn insert(&mut self, index: u32, value: T) {
        let i = index as usize;
        if self.slots.len() <= i {
            self.slots.resize(i + 1, u32::MAX);
        }
        if self.slots[i] != u32::MAX {
            self.values[self.slots[i] as usize] = value;
            return;
        }
        self.slots[i] = self.values.len() as u32;
        self.values.push(value);
        self.owners.push(index);
    }

    fn remove(&mut self, index: u32) {
        let i = index as usize;
        let Some(&slot) = self.slots.get(i) else {
            return;
        };
        if slot == u32::MAX {
            return;
        }
        self.values.swap_remove(slot as usize);
        self.owners.swap_remove(slot as usize);
        if let Some(&moved) = self.owners.get(slot as usize) {
            self.slots[moved as usize] = slot;
        }
        self.slots[i] = u32::MAX;
    }

    pub fn get(&self, index: u32) -> Option<&T> {
        let slot = *self.slots.get(index as usize)?;
        self.values.get(slot as usize)
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        let slot = *self.slots.get(index as usize)?;
        self.values.get_mut(slot as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.owners.iter().copied().zip(&self.values)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (u32, &mut T)> {
        self.owners.iter().copied().zip(&mut self.values)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

trait AnyColumn: Any {
    fn remove(&mut self, index: u32);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> AnyColumn for Column<T> {
    fn remove(&mut self, index: u32) {
        Column::remove(self, index);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Every entity and its components.
#[derive(Default)]
pub struct World {
    generations: Vec<u32>,
    free: Vec<u32>,
    columns: HashMap<TypeId, Box<dyn AnyColumn>>,
}

impl World {
    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            return Entity {
                index,
                generation: self.generations[index as usize],
            };
        }
        self.generations.push(0);
        Entity {
            index: self.generations.len() as u32 - 1,
            generation: 0,
        }
    }

    pub fn insert<T: 'static>(&mut self, e: Entity, value: T) {
        self.columns
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Column::<T>::default()))
            .as_any_mut()
            .downcast_mut::<Column<T>>()
            .expect("column of its own type")
            .insert(e.index, value);
    }

    pub fn despawn(&mut self, e: Entity) {
        if self.generations.get(e.index as usize) != Some(&e.generation) {
            return;
        }
        for column in self.columns.values_mut() {
            column.remove(e.index);
        }
        self.generations[e.index as usize] += 1;
        self.free.push(e.index);
    }

    pub fn column<T: 'static>(&self) -> Option<&Column<T>> {
        self.columns
            .get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref::<Column<T>>()
    }

    pub fn column_mut<T: 'static>(&mut self) -> Option<&mut Column<T>> {
        self.columns
            .get_mut(&TypeId::of::<T>())?
            .as_any_mut()
            .downcast_mut::<Column<T>>()
    }

    /// Two columns at once, one changed: the first is borrowed for
    /// writing, so they must differ.
    pub fn two<A: 'static, B: 'static>(&mut self) -> Option<(&mut Column<A>, &Column<B>)> {
        let [a, b] = self
            .columns
            .get_disjoint_mut([&TypeId::of::<A>(), &TypeId::of::<B>()]);
        Some((
            a?.as_any_mut().downcast_mut::<Column<A>>()?,
            b?.as_any().downcast_ref::<Column<B>>()?,
        ))
    }

    pub fn get<T: 'static>(&self, e: Entity) -> Option<&T> {
        if self.generations.get(e.index as usize) != Some(&e.generation) {
            return None;
        }
        self.column::<T>()?.get(e.index)
    }
}
