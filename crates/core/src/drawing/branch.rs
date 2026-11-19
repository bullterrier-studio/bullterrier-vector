//! The typed step `branch`.

use super::Drawing;
use super::transform::Deg;
use crate::Expr;
use crate::node::{Given, branch};

/// What [`Drawing::branch`] grows: `spacing` and `length` with the options it leaves at their defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    given: Vec<(&'static str, Expr)>,
}

impl Branch {
    /// A root every `spacing` design pixels along each outline, and paths `length` long.
    #[must_use]
    pub fn new(spacing: f64, length: f64) -> Self {
        Self {
            given: vec![("spacing", Expr::from(spacing)), ("length", Expr::from(length))],
        }
    }

    fn with(mut self, name: &'static str, value: Expr) -> Self {
        self.given.retain(|(known, _)| *known != name);
        self.given.push((name, value));
        self
    }

    /// Distance from the root per fork, in design pixels (default 80): a thread forks once in each stretch this long.
    #[must_use]
    pub fn fork_spacing(self, fork_spacing: f64) -> Self {
        self.with("fork_spacing", Expr::from(fork_spacing))
    }

    /// The most forks one path takes, a whole number from 0 (default 2; 0 grows no forks).
    #[must_use]
    pub fn forks(self, forks: usize) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count past 2^53 is refused by the work bound"
        )]
        let forks = forks as f64;
        self.with("forks", Expr::from(forks))
    }

    /// How much shorter than `length` a root's tree may be, 0 to 1 (default 0): each root's factor is a seeded number
    /// from `1 - shorter` to 1.
    #[must_use]
    pub fn shorter(self, shorter: impl Into<Expr>) -> Self {
        self.with("shorter", shorter.into())
    }

    /// The turn of a fork from its parent, clockwise to a seeded side (default 45 degrees).
    #[must_use]
    pub fn angle<T: Into<Expr>>(self, angle: Deg<T>) -> Self {
        self.with("angle", angle.0.into())
    }

    /// How far the heading drifts (default 30 degrees): it turns by `wander` times the change of smooth seeded noise
    /// from -1 to 1, so at most twice `wander` from where a thread set out.
    #[must_use]
    pub fn wander<T: Into<Expr>>(self, wander: Deg<T>) -> Self {
        self.with("wander", wander.0.into())
    }

    /// Design pixels along a thread over which the heading drifts (default 80).
    #[must_use]
    pub fn wavelength(self, wavelength: impl Into<Expr>) -> Self {
        self.with("wavelength", wavelength.into())
    }

    /// The seed, a whole number (default 0).
    #[must_use]
    pub fn seed(self, seed: impl Into<Expr>) -> Self {
        self.with("seed", seed.into())
    }
}

impl Drawing {
    /// This drawing with every shape replaced by forking threads grown out of the outline of the region it fills: one
    /// open shape per root-to-tip path, keeping the shape's style and attributes, with `root` (from 0 on the shape),
    /// `forks` (the forks the path takes; 0 is the trunk), `order` (the path's share 0 to 1 of its shape's paths, replacing the
    /// shape's own) and its root as `origin`. Every path of a root starts
    /// on the outline and has the same length, so `slice(0, growth)` grows a tree from its root and a fork appears
    /// where its parent passes; paths of a tree share the points of their common part, so stroke them opaque or unite
    /// them first. Equal shapes at different places in the list grow different threads.
    ///
    /// A refused number leaves the drawing with an [`error`](Drawing::error) naming the step; a request past
    /// `MAX_SEGMENTS` points or `MAX_SHAPES` paths is refused when drawn, before any thread is grown, naming
    /// `wavelength` (one path), `fork_spacing` (one root) or `spacing` (all the roots).
    #[must_use]
    pub fn branch(&self, branch: &Branch) -> Self {
        let given = branch
            .given
            .iter()
            .map(|(name, value)| (*name, Given::Number(value.clone())))
            .collect();
        self.then(&branch::Branch, given)
    }
}
