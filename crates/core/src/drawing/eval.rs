//! Drawing a `Drawing`: its steps evaluated, in order, to a list of shapes, each one served from the cache when the
//! values it depends on are the ones it was last computed at.

use super::cache::{Cache, EntryKey, NodeId, Stats};
use super::{Drawing, Step};
use crate::key::Key;
use crate::limits::Work;
use crate::node::{Context, Inputs};
use crate::{Error, Rect, Shape, Values};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

impl Drawing {
    /// The shapes the drawing evaluates to, in paint order (the bottom first), in design pixels, with the Params at
    /// the `values` given and at their defaults where none is given. The default paint is resolved: an unpainted
    /// closed outline has a black fill, an unpainted open line a black stroke.
    ///
    /// A step whose result the cache holds for the values it depends on is not computed again: drawing again with a
    /// changed value computes only the steps that depend on it, and a drawing used twice is computed once.
    ///
    /// # Errors
    /// The first error of the chain that built the drawing; [`Error::Parameter`] naming the Param if `values` gives
    /// one the drawing does not read, or one outside its range (a number that is not finite included), whether or
    /// not a step reads it with shapes, or a list of the wrong length; naming the step and the argument
    /// (`step 3 (rotate).angle`) if a number is outside what the argument accepts, with the index of the shape it was
    /// read for.
    pub fn shapes(&self, values: &Values) -> Result<Vec<Shape>, Error> {
        let step = self.last.as_ref().map_err(|failure| failure.error.clone())?;
        let free = step.free();
        values.names_within(&free)?;
        let mut cache = self.cache.lock();
        let found = match values.accepted_by(&free) {
            Ok(accepted) => Run::draw(&mut cache, step, &accepted)?,
            // a value a step reads is refused where it is read, naming the step; one no step reads with shapes is
            // refused here, so that a value out of range never goes unnoticed
            Err(refused) => {
                Run::draw(&mut cache, step, values)?;
                return Err(refused);
            }
        };
        Ok(found.shapes.iter().cloned().map(Shape::painted).collect())
    }

    /// The smallest rectangle that holds every shape the drawing evaluates to with `values`: the union of their
    /// [`Path::bounds`](crate::Path::bounds), in design pixels. Strokes and effects add nothing.
    ///
    /// # Errors
    /// Those of [`Drawing::shapes`]; [`Error::Geometry`] if the drawing draws no shape or only empty paths.
    pub fn bounds(&self, values: &Values) -> Result<Rect, Error> {
        let mut found: Option<(f64, f64, f64, f64)> = None;
        for shape in self.shapes(values)? {
            let Rect { x, y, width, height } = shape.path().bounds()?;
            let (left, top, right, bottom) = (x, y, x + width, y + height);
            found = Some(found.map_or((left, top, right, bottom), |(l, t, r, b)| {
                (l.min(left), t.min(top), r.max(right), b.max(bottom))
            }));
        }
        let (left, top, right, bottom) =
            found.ok_or_else(|| Error::geometry("bounds", "the drawing draws no shape"))?;
        Ok(Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }

    /// What the most recent draw did: how many steps it computed and how many it was served, and how many results
    /// the cache holds. The cache is shared by this drawing, every drawing derived from it by a step, and a stack that
    /// has it first, so the numbers are those of the last draw of any of them. Drawing the same thing again computes
    /// nothing; changing a value computes the steps that depend on it.
    #[must_use]
    pub fn stats(&self) -> Stats {
        self.cache.lock().stats()
    }
}

/// A result, and the nearest entries it was made of that the cache holds: its own if it is held, else those of the
/// steps it was made from; none for the shapes before a source, of which there are none.
struct Found {
    shapes: Arc<Vec<Shape>>,
    keys: Vec<EntryKey>,
}

/// One draw: the cache, the ids of the steps met so far, and the counters.
struct Run<'a> {
    cache: &'a mut Cache,
    /// The id of each step met, by where it is in memory, which is good for as long as the draw holds the drawing.
    ids: BTreeMap<*const Step, NodeId>,
    /// How many times each step is built on or taken by a step met, to tell a result used twice from one used once.
    uses: BTreeMap<NodeId, usize>,
    work: Work,
}

impl Run<'_> {
    /// The shapes after `last` with `values`, ending the draw, whether it went through or failed, by dropping what it
    /// did not use.
    fn draw(cache: &mut Cache, last: &Arc<Step>, values: &Values) -> Result<Found, Error> {
        cache.begin();
        let mut run = Run {
            cache,
            ids: BTreeMap::new(),
            uses: BTreeMap::new(),
            work: Work::new(),
        };
        // a draw that fails ends too: what it computed stays, what it did not use goes
        let found = run.evaluate(last, values, None);
        let reached: BTreeSet<NodeId> = run.ids.values().copied().collect();
        run.cache.end(&reached);
        found
    }

    /// The id of the step `last`, and of every step it is made of. Walks a chain in a loop and the drawings a step
    /// takes by recursion, which nesting bounds.
    fn node(&mut self, last: &Arc<Step>) -> NodeId {
        let mut unknown = Vec::new();
        let mut cursor = Some(last);
        while let Some(step) = cursor
            && !self.ids.contains_key(&Arc::as_ptr(step))
        {
            unknown.push(step);
            cursor = step.previous.as_ref();
        }
        let mut id = cursor.and_then(|step| self.ids.get(&Arc::as_ptr(step)).copied());
        for step in unknown.into_iter().rev() {
            let mut key = step.key.clone();
            key.push(id.map_or(0, |before| before + 1));
            if let Some(before) = id {
                *self.uses.entry(before).or_default() += 1;
            }
            for item in step.args.taken() {
                if let Ok(item) = &item.last {
                    let taken = self.node(item);
                    *self.uses.entry(taken).or_default() += 1;
                    key.push(taken);
                }
            }
            let own = self.cache.node(key);
            self.ids.insert(Arc::as_ptr(step), own);
            id = Some(own);
        }
        // `last` is known now: a loop above met it, or it was known already
        id.unwrap_or_default()
    }

    /// Where the result of `step` is held when it sees `env`: the values of the Params it reads and nothing fixed
    /// reads for it.
    fn entry(&mut self, step: &Arc<Step>, env: &Values) -> EntryKey {
        let node = self.node(step);
        let mut values = Key::new();
        for param in step.reads.iter().filter(|param| !step.fixed.contains(param.name())) {
            values.of(&env.get(param.name()));
        }
        EntryKey::new(node, values.finish())
    }

    /// Whether the cache keeps the result of `step`, which `after` builds on in its chain, or, if it ends the chain,
    /// `consumer` takes (or nothing does: it is the drawing drawn). A later draw starts from a result only where the
    /// step after it reads other Params free than it does, or the result is used twice in this draw; a result that
    /// only feeds a step with the same free Params is made again whenever that step is, so keeping it would hold
    /// every step of a long chain at once.
    fn kept(&self, step: &Arc<Step>, after: Option<&Arc<Step>>, consumer: Option<&Arc<Step>>) -> bool {
        let shared = self
            .ids
            .get(&Arc::as_ptr(step))
            .and_then(|id| self.uses.get(id))
            .is_some_and(|uses| *uses > 1);
        shared
            || match (after, consumer) {
                (Some(after), _) => !after.same_free_as_before,
                (None, Some(consumer)) => consumer.free_len != step.free_len,
                (None, None) => true,
            }
    }

    /// The shapes after `last` and every step before it, in order, with each shape's `index` its place in the list. A
    /// step sees `values` with the values of every `with_values` after it laid over them. The latest step whose result
    /// the cache holds is where the work starts. `consumer` is the step that takes this chain as one of its items, if it
    /// is one's; else the chain is the drawing drawn.
    fn evaluate(&mut self, last: &Arc<Step>, values: &Values, consumer: Option<&Arc<Step>>) -> Result<Found, Error> {
        let mut chain = Vec::new();
        let mut cursor = Some(last);
        while let Some(step) = cursor {
            chain.push(step);
            cursor = step.previous.as_ref();
        }
        // from the last step back, each with the values it sees: a `with_values` changes them for the steps before it
        let mut envs: Vec<Cow<'_, Values>> = vec![Cow::Borrowed(values)];
        let mut seen_by = Vec::with_capacity(chain.len());
        for step in &chain {
            seen_by.push(envs.len() - 1);
            if let Some(fixed) = step.args.fixes() {
                let next = envs[envs.len() - 1].over(fixed);
                envs.push(Cow::Owned(next));
            }
        }
        // back from the last step to the first one the cache holds
        let mut found = Found {
            shapes: Arc::new(Vec::new()),
            keys: Vec::new(),
        };
        let mut missing = Vec::new();
        let mut after = None;
        for (step, env) in chain.into_iter().zip(seen_by) {
            let key = self.entry(step, &envs[env]);
            if let Some(shapes) = self.cache.lookup(&key) {
                found = Found {
                    shapes,
                    keys: vec![key],
                };
                break;
            }
            let kept = self.kept(step, after, consumer);
            missing.push((step, env, key, kept));
            after = Some(step);
        }
        // and forward from there
        for (step, env, key, kept) in missing.into_iter().rev() {
            let env: &Values = &envs[env];
            let place = step.place();
            let mut inputs = std::mem::take(&mut found.keys);
            let mut taken = Vec::new();
            for (at, item) in step.args.taken().enumerate() {
                let Ok(item) = &item.last else { continue };
                let made = self
                    .evaluate(item, env, Some(step))
                    .map_err(|error| error.inside(&format!("{place} / item {}", at + 1)))?;
                inputs.extend(made.keys);
                taken.push(made.shapes);
            }
            let cx = Context {
                place,
                env,
                work: &self.work,
            };
            let made = step.node.evaluate(
                &step.args,
                Inputs {
                    previous: &found.shapes,
                    taken: &taken,
                },
                &cx,
            )?;
            let numbered = made
                .into_iter()
                .enumerate()
                .map(|(index, shape)| shape.at(index))
                .collect();
            found = if kept {
                Found {
                    shapes: self.cache.store(key.clone(), numbered, inputs),
                    keys: vec![key],
                }
            } else {
                self.cache.computed_unkept();
                Found {
                    shapes: Arc::new(numbered),
                    keys: inputs,
                }
            };
        }
        Ok(found)
    }
}
