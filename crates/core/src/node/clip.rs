//! The steps that cut a shape: `clip` keeps what lies inside a region, `mask` sets the opacity by the luminance of
//! other shapes. Each shape is cut on its own (as `opacity` is), not as a group of the drawing. The region and the
//! mask are made once for the step and shared by every shape it cuts.

use super::boolean::region;
use super::{Args, Context, Inputs, Node, each_shape};
use crate::path::{Combine, FLATNESS};
use crate::{Arg, Domain, Error, Kind, Path, Reads, Role, Shape, Verb};
use std::sync::Arc;

/// The drawing whose shapes make the region or the mask.
const fn source(name: &'static str) -> Arg {
    Arg {
        name,
        kind: Kind::Drawing,
        default: None,
        reads: Reads::Literal,
        domain: Domain::Any,
    }
}

/// Every shape cut to a region.
#[derive(Debug)]
pub(crate) struct Clip;

pub(crate) static CLIP: Verb = Verb {
    name: "clip",
    role: Role::Step,
    summary: "cuts every shape, and its shadow, to the region the drawing `region` fills (all its shapes united once); each shape is cut on its own; a region that fills nothing leaves nothing drawn; a knockout is not cut",
    args: &[source("region")],
    check: None,
};

/// Every shape faded by the luminance of other shapes.
#[derive(Debug)]
pub(crate) struct Mask;

pub(crate) static MASK: Verb = Verb {
    name: "mask",
    role: Role::Step,
    summary: "fades every shape, and its shadow, by the luminance of the drawing `source` as painted (luminance times alpha, white keeps and black hides); each shape is masked on its own; a shape takes one mask, and the shapes of `source` carry none; a knockout is not masked",
    args: &[source("source")],
    check: None,
};

impl Node for Clip {
    fn verb(&self) -> &'static Verb {
        &CLIP
    }

    fn evaluate(&self, _: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let place = cx.place.to_string();
        let mut charge = |count| cx.work.made_segments(count, &place);
        let region = region(&inputs.taken[0], FLATNESS, &mut charge)
            .map_err(|error| error.at_argument(&cx.place, None))?
            .into_owned();
        let region = Arc::new(region);
        // a shape that is clipped already is cut to both; the shapes one earlier clip made share it, so it is met once
        let mut met: Vec<(Arc<Path>, Arc<Path>)> = Vec::new();
        each_shape(inputs.previous, cx, |shape, _| {
            let mut cut = shape.clone();
            let clip = match shape.style().shared_clip() {
                None => Arc::clone(&region),
                Some(before) => {
                    if let Some((_, both)) = met.iter().find(|(old, _)| Arc::ptr_eq(old, before)) {
                        Arc::clone(both)
                    } else {
                        let both = Arc::new(before.combined(&region, Combine::Intersect, FLATNESS, &mut charge)?);
                        met.push((Arc::clone(before), Arc::clone(&both)));
                        both
                    }
                }
            };
            cut.style_mut().set_clip(clip);
            Ok(cut)
        })
    }
}

impl Node for Mask {
    fn verb(&self) -> &'static Verb {
        &MASK
    }

    fn evaluate(&self, _: &Args, inputs: Inputs<'_>, cx: &Context<'_>) -> Result<Vec<Shape>, Error> {
        let source = &inputs.taken[0];
        if let Some(masked) = source.iter().find(|shape| shape.style().mask().is_some()) {
            return Err(Error::parameter(
                "source",
                format!(
                    "shape {} of the mask is masked itself: a mask holds no masks",
                    masked.index()
                ),
            )
            .at_argument(&cx.place, None));
        }
        let mask: Arc<[Shape]> = source.iter().cloned().map(Shape::painted).collect();
        each_shape(inputs.previous, cx, |shape, _| {
            if shape.style().mask().is_some() {
                return Err(Error::parameter("source", "is masked already: a shape takes one mask"));
            }
            let mut cut = shape.clone();
            cut.style_mut().set_mask(Arc::clone(&mask));
            Ok(cut)
        })
    }
}
