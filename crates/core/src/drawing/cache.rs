//! The cache of evaluated steps, shared by a drawing and every drawing derived from it.
//!
//! Steps are told apart by their content, not by where they sit in memory or in a hash: each step's key (its verb, its
//! arguments, the ids of the steps it builds on) is interned in a `BTreeMap` into a [`NodeId`], so equal steps built
//! apart are one. A result is kept under its node and the values the step's free Params take where it is drawn, which
//! is everything the output depends on. After each draw, a failed one included, the entries that draw did not use are
//! dropped, so the cache holds one draw, however many values a caller tries.

use crate::Shape;
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
    shapes: Arc<[Shape]>,
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
}

impl Cache {
    /// Starts a draw: its counters are its own.
    pub(crate) fn begin(&mut self) {
        self.draw += 1;
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
    pub(crate) fn lookup(&mut self, key: &EntryKey) -> Option<Arc<[Shape]>> {
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
    pub(crate) fn store(&mut self, key: EntryKey, shapes: Vec<Shape>, inputs: Vec<EntryKey>) -> Arc<[Shape]> {
        let shapes: Arc<[Shape]> = Arc::from(shapes);
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

    /// Ends a draw, whether it went through or failed: drops every result it did not use, and every step it did not
    /// reach.
    pub(crate) fn end(&mut self, reached: &BTreeSet<NodeId>) {
        let draw = self.draw;
        self.entries.retain(|_, entry| entry.used == draw);
        self.nodes.retain(|_, id| reached.contains(id));
    }

    pub(crate) fn stats(&self) -> Stats {
        Stats {
            computed: self.computed,
            reused: self.reused,
            entries: self.entries.len(),
        }
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
