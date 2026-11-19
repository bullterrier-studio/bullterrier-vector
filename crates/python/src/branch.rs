//! The options of `Drawing.branch`.

use crate::expr::Num;
use bullterrier_vector as vector;

/// The threads a call of `branch` asks for; an option left out (`None`) keeps the core's default.
#[expect(clippy::too_many_arguments, reason = "the keyword arguments of one call")]
pub(crate) fn branch_of(
    spacing: f64,
    length: f64,
    fork_spacing: Option<f64>,
    forks: Option<i64>,
    spread: Option<Num>,
    angle: Option<Num>,
    wander: Option<Num>,
    wavelength: Option<Num>,
    seed: Option<Num>,
) -> vector::Branch {
    let mut branch = vector::Branch::new(spacing, length);
    if let Some(fork_spacing) = fork_spacing {
        branch = branch.fork_spacing(fork_spacing);
    }
    if let Some(forks) = forks {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count past 2^53 is refused by the work bound"
        )]
        let forks = forks as f64;
        branch = branch.forks(forks);
    }
    if let Some(Num(spread)) = spread {
        branch = branch.spread(spread);
    }
    if let Some(Num(angle)) = angle {
        branch = branch.angle(vector::Deg(angle));
    }
    if let Some(Num(wander)) = wander {
        branch = branch.wander(vector::Deg(wander));
    }
    if let Some(Num(wavelength)) = wavelength {
        branch = branch.wavelength(wavelength);
    }
    if let Some(Num(seed)) = seed {
        branch = branch.seed(seed);
    }
    branch
}
