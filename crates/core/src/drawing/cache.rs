//! The cache of evaluated steps, shared by a drawing and every drawing derived from it.
//!
//! Steps are told apart by their content, not by where they sit in memory or in a hash: each step's key (its verb, its
//! arguments, the ids of the steps it builds on) is interned in a `BTreeMap` into a [`NodeId`], so equal steps built
//! apart are one. A result is kept under its node and the values the step's free Params take where it is drawn, which
//! is everything the output depends on.
//!
//! What is kept is what a later draw can start from, not every step: a result is kept if it is what the draw
//! returned, if the step after it (or the step that takes it as an item) reads other Params free than it does, or if
//! the draw used it twice. A result that only feeds a step reading the same Params is made again whenever that step
//! is, so a chain of steps that read no Param keeps one result, not one per step. After each draw, a failed one
//! included, the entries that draw did not use are dropped, so the cache holds what one draw kept, however many
//! values a caller tries.

use crate::Shape;
use crate::field::Grids;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// The number an interned step goes by; never reused, so a stale one names nothing.
pub(crate) type NodeId = u64;

/// What the most recent draw of a family of drawings did, to see that a redraw computes only what a changed value
/// reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    /// The steps computed, each counted once: work done.
    pub computed: usize,
    /// The steps served from the cache without computing them, and so without the steps before them.
    pub reused: usize,
    /// The results held now.
    pub entries: usize,
}

/// A result's place: the step, and the values its free Params took.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EntryKey {
    node: NodeId,
    values: Vec<u64>,
}

impl EntryKey {
    pub(crate) fn new(node: NodeId, values: Vec<u64>) -> Self {
        Self { node, values }
    }
}

/// The shapes a step made, and where the results they were made from are.
#[derive(Debug)]
struct Entry {
    shapes: Arc<Vec<Shape>>,
    /// The entries of the step before and of the drawings the step took.
    inputs: Vec<EntryKey>,
    /// The draw that last used it.
    used: u64,
}

/// Everything remembered between draws.
#[derive(Debug, Default)]
pub(crate) struct Cache {
    nodes: BTreeMap<Vec<u64>, NodeId>,
    next: NodeId,
    entries: BTreeMap<EntryKey, Entry>,
    draw: u64,
    computed: usize,
    reused: usize,
    pub(crate) grids: GridStore,
}

impl Cache {
    /// Starts a draw: its counters are its own.
    pub(crate) fn begin(&mut self) {
        self.draw += 1;
        self.grids.draw = self.draw;
        self.computed = 0;
        self.reused = 0;
    }

    /// The id of the step with this `key`, the same for the same key.
    pub(crate) fn node(&mut self, key: Vec<u64>) -> NodeId {
        let next = self.next;
        let id = *self.nodes.entry(key).or_insert(next);
        if id == next {
            self.next += 1;
        }
        id
    }

    /// The result kept under `key`, if there is one. It counts as used, and so does every result it was made of.
    pub(crate) fn lookup(&mut self, key: &EntryKey) -> Option<Arc<Vec<Shape>>> {
        let shapes = Arc::clone(&self.entries.get(key)?.shapes);
        self.reused += 1;
        let mut pending = vec![key.clone()];
        while let Some(next) = pending.pop() {
            if let Some(entry) = self.entries.get_mut(&next)
                && entry.used != self.draw
            {
                entry.used = self.draw;
                pending.extend(entry.inputs.iter().cloned());
            }
        }
        Some(shapes)
    }

    /// Keeps `shapes` made from the results `inputs` as the result for `key`: one step computed.
    pub(crate) fn store(&mut self, key: EntryKey, shapes: Vec<Shape>, inputs: Vec<EntryKey>) -> Arc<Vec<Shape>> {
        let shapes = Arc::new(shapes);
        self.computed += 1;
        let used = self.draw;
        self.entries.insert(
            key,
            Entry {
                shapes: Arc::clone(&shapes),
                inputs,
                used,
            },
        );
        shapes
    }

    /// Counts a step computed whose result is not kept.
    pub(crate) fn computed_unkept(&mut self) {
        self.computed += 1;
    }

    /// Ends a draw, whether it went through or failed: drops every result it did not use, and every step it did not
    /// reach.
    pub(crate) fn end(&mut self, reached: &BTreeSet<NodeId>) {
        let draw = self.draw;
        self.entries.retain(|_, entry| entry.used == draw);
        self.nodes.retain(|_, id| reached.contains(id));
        self.grids.end();
    }

    pub(crate) fn stats(&self) -> Stats {
        Stats {
            computed: self.computed,
            reused: self.reused,
            entries: self.entries.len(),
        }
    }
}

/// Readings of distance leaves over grids, kept from one draw to the next: what [`Grids`] promises, by content, so a
/// reading is found again whenever the same outline is read over the same grid, whatever else the draw changed.
///
/// What is kept is what a draw used: a reading is dropped at the end of the first draw (a failed one included) that did
/// not read it. The store therefore holds one draw's grids, each at most `MAX_MAP_PIXELS` readings of 8 bytes, and how
/// many a draw reads is limited by the read-work budget it is charged.
#[derive(Debug, Default)]
pub(crate) struct GridStore {
    held: RefCell<BTreeMap<Vec<u64>, Held>>,
    draw: u64,
}

#[derive(Debug)]
struct Held {
    readings: Arc<Vec<f64>>,
    /// The draw that last read it.
    used: u64,
}

impl GridStore {
    /// Ends a draw: drops the readings it did not use.
    fn end(&mut self) {
        let draw = self.draw;
        self.held.get_mut().retain(|_, held| held.used == draw);
    }
}

impl Grids for GridStore {
    fn grid(&self, key: Vec<u64>, fill: &mut dyn FnMut() -> Vec<f64>) -> Arc<Vec<f64>> {
        let draw = self.draw;
        if let Some(held) = self.held.borrow_mut().get_mut(&key) {
            held.used = draw;
            return Arc::clone(&held.readings);
        }
        // not held while filling: a fill reads outlines, never the store
        let readings = Arc::new(fill());
        self.held.borrow_mut().insert(
            key,
            Held {
                readings: Arc::clone(&readings),
                used: draw,
            },
        );
        readings
    }
}

/// The cache of a family of drawings: one is shared by a drawing and everything derived from it, and a stack uses its
/// first item's. It holds only what is content-addressed, so sharing it cannot change a result.
#[derive(Debug, Clone, Default)]
pub(crate) struct Shared(Arc<Mutex<Cache>>);

impl Shared {
    /// The cache, for one draw or one look. A draw that panicked leaves entries that are each complete, so a poisoned
    /// lock is taken as it is.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Cache> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    //! Beside the store because what it holds (which readings, how many, whose) is no public name.
    use super::*;
    use crate::{Drawing, Error, Field, Param, Path, Rect, Trace, Values};

    fn held_of(drawing: &Drawing) -> Vec<Arc<Vec<f64>>> {
        let cache = drawing.cache.lock();
        let held = cache.grids.held.borrow();
        held.values().map(|held| Arc::clone(&held.readings)).collect()
    }

    /// The circle moved by `shift`; the noise added to its distance reads `growth`, which the distance ignores.
    fn drawing() -> Result<Drawing, Error> {
        let shift = Param::new("shift", 0.0, -100.0, 100.0)?;
        let growth = Param::new("growth", 40.0, 10.0, 80.0)?;
        let circle = Drawing::from(Path::circle((100.0, 100.0), 40.0)?).translate(&shift, 0.0);
        let noise = Field::noise(&growth, 0.0, 1.0, 2)?;
        let field = Field::distance(circle, false)?.add(&noise)?;
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };
        Ok(Drawing::trace_levels(
            &field,
            [10.0],
            &Trace::new().frame(frame).cell(5.0),
        ))
    }

    #[test]
    fn a_reading_held_is_not_made_again_for_its_key_only() {
        let (mut store, mut made) = (GridStore::default(), 0);
        store.draw = 1;
        let mut fill = || {
            made += 1;
            vec![1.0, 2.0]
        };
        let first = store.grid(vec![1, 2], &mut fill);
        let again = store.grid(vec![1, 2], &mut fill);
        let other = store.grid(vec![1, 3], &mut fill);
        assert_eq!(made, 2, "a second key is made, a repeated one is not");
        assert!(Arc::ptr_eq(&first, &again) && !Arc::ptr_eq(&first, &other));
    }

    #[test]
    fn a_draw_ends_by_dropping_the_readings_it_did_not_read() {
        let mut store = GridStore {
            draw: 1,
            ..GridStore::default()
        };
        store.grid(vec![1], &mut || vec![0.0]);
        store.grid(vec![2], &mut || vec![0.0]);
        store.end();
        assert_eq!(store.held.borrow().len(), 2, "both were read by draw 1");
        store.draw = 2;
        let mut made = 0;
        store.grid(vec![2], &mut || {
            made += 1;
            vec![0.0]
        });
        store.end();
        let keys: Vec<_> = store.held.borrow().keys().cloned().collect();
        assert_eq!(
            (keys, made),
            (vec![vec![2]], 0),
            "only what draw 2 read stays, held as it was"
        );
    }

    #[test]
    fn a_param_the_distance_ignores_leaves_its_grid_held_and_a_moved_outline_replaces_it() -> Result<(), Error> {
        let drawing = drawing()?;
        let draw = |shift: f64, growth: f64| -> Result<Vec<Arc<Vec<f64>>>, Error> {
            drawing.shapes(&Values::new().set("shift", shift).set("growth", growth))?;
            Ok(held_of(&drawing))
        };
        let (first, grown, moved) = (draw(0.0, 40.0)?, draw(0.0, 25.0)?, draw(15.0, 25.0)?);
        assert_eq!(
            (first.len(), grown.len(), moved.len()),
            (1, 1, 1),
            "one draw's grid at a time"
        );
        assert!(
            Arc::ptr_eq(&first[0], &grown[0]),
            "growth moved, the outline did not: a hit"
        );
        assert!(!Arc::ptr_eq(&first[0], &moved[0]), "the outline moved: a miss");
        Ok(())
    }

    #[test]
    fn a_sweep_of_outlines_never_grows_the_store_past_one_draws_grids() -> Result<(), Error> {
        let drawing = drawing()?;
        for shift in 0..12 {
            drawing.shapes(&Values::new().set("shift", f64::from(shift)))?;
            assert_eq!(held_of(&drawing).len(), 1, "after {shift}");
        }
        Ok(())
    }
}
