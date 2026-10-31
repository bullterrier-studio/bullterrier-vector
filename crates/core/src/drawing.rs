//! `Drawing`: a tree of steps that evaluates to a list of shapes.

mod eval;

use crate::Path;
use std::sync::Arc;

/// An immutable tree of steps that evaluates to a list of [`Shape`](crate::Shape)s: a source (so far a [`Path`]), and
/// the steps that build on it. A drawing holds no callbacks, so it draws again with new values. It is cheap to clone
/// and can be used again, in several places.
#[derive(Debug, Clone)]
pub struct Drawing {
    step: Arc<Step>,
}

/// The last step of a drawing; the steps before it are reached through it.
#[derive(Debug)]
struct Step {
    path: Path,
}

impl From<Path> for Drawing {
    /// A drawing of the one shape that is `path`, painted by default: a closed outline filled black, an open line
    /// stroked black.
    fn from(path: Path) -> Self {
        Self {
            step: Arc::new(Step { path }),
        }
    }
}
