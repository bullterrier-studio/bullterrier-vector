//! A drawing as the graph of merged calls that formats.md ("Shared drawings") defines: equal calls are one, and a call
//! knows how many other calls build on it or take it.

use super::{Drawing, Step};
use crate::node::Args;
use crate::{Error, Verb};
use std::collections::BTreeMap;
use std::sync::Arc;

/// One call of the merged graph.
pub(crate) struct CallNode {
    step: Arc<Step>,
    /// The call this one builds on.
    pub(crate) previous: Option<usize>,
    /// The calls of the drawings this one takes, in the order of its arguments.
    pub(crate) taken: Vec<usize>,
    /// How many calls build on this one or take it (a call that takes it twice counts twice).
    pub(crate) uses: usize,
}

impl CallNode {
    /// What the call is.
    pub(crate) fn verb(&self) -> &'static Verb {
        self.step.node.verb()
    }

    /// The values of the call.
    pub(crate) fn args(&self) -> &Args {
        &self.step.args
    }
}

/// The calls of a drawing, each once, in the order a walk first reaches them: the calls a call builds on or takes
/// come before it.
pub(crate) struct Graph {
    pub(crate) nodes: Vec<CallNode>,
    /// The last call of the drawing.
    pub(crate) root: usize,
}

/// The graph under construction.
struct Merge {
    nodes: Vec<CallNode>,
    /// The node of each distinct call, told apart by its key: the call's arguments and the nodes it builds on and takes.
    by_key: BTreeMap<Vec<u64>, usize>,
    /// The node of each step already visited, by where it is in memory (good while the drawing is held).
    by_step: BTreeMap<*const Step, usize>,
}

impl Merge {
    /// The node of `last`, and of every call it is made of. Walks a chain in a loop and the drawings a call takes by
    /// recursion, which their nesting bounds.
    fn visit(&mut self, last: &Arc<Step>) -> usize {
        let mut unknown = Vec::new();
        let mut cursor = Some(last);
        while let Some(step) = cursor
            && !self.by_step.contains_key(&Arc::as_ptr(step))
        {
            unknown.push(step);
            cursor = step.previous.as_ref();
        }
        let mut before = cursor.and_then(|step| self.by_step.get(&Arc::as_ptr(step)).copied());
        for step in unknown.into_iter().rev() {
            let taken: Vec<usize> = step
                .args
                .taken()
                .filter_map(|item| item.last.as_ref().ok())
                .map(|item| self.visit(item))
                .collect();
            let mut key = step.key.clone();
            key.push(before.map_or(0, |node| node as u64 + 1));
            key.extend(taken.iter().map(|node| *node as u64));
            let node = if let Some(node) = self.by_key.get(&key) {
                *node
            } else {
                self.nodes.push(CallNode {
                    step: Arc::clone(step),
                    previous: before,
                    taken,
                    uses: 0,
                });
                self.by_key.insert(key, self.nodes.len() - 1);
                self.nodes.len() - 1
            };
            self.by_step.insert(Arc::as_ptr(step), node);
            before = Some(node);
        }
        // `last` is known now: a loop above met it, or it was known already
        before.unwrap_or_default()
    }
}

impl Drawing {
    /// The merged graph of this drawing's calls.
    ///
    /// # Errors
    /// The first error of the chain that built the drawing.
    pub(crate) fn graph(&self) -> Result<Graph, Error> {
        let last = self.last.as_ref().map_err(|failure| failure.error.clone())?;
        let mut merge = Merge {
            nodes: Vec::new(),
            by_key: BTreeMap::new(),
            by_step: BTreeMap::new(),
        };
        let root = merge.visit(last);
        let mut nodes = merge.nodes;
        for at in 0..nodes.len() {
            let built_on = nodes[at].previous;
            let taken = nodes[at].taken.clone();
            for used in built_on.into_iter().chain(taken) {
                nodes[used].uses += 1;
            }
        }
        Ok(Graph { nodes, root })
    }
}
