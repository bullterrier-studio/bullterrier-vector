//! `Drawing::to_rgba`: the drawing as straight-alpha RGBA bytes. The only module that uses the rasteriser crates
//! (`vello_cpu` and `kurbo`); nothing of them reaches the API.

use super::view::View;
use super::{Size, in_shape};
use crate::path::Element;
use crate::{Blend, Cap, Drawing, Error, Join, Path, Point, Shape, Values};
use kurbo::BezPath;
use vello_cpu::color::AlphaColor;
use vello_cpu::peniko::{BlendMode, Compose, Fill, ImageAlphaType, Mix};
use vello_cpu::{Level, Pixmap, RasterizerSettings, RenderContext, RenderMode, RenderSettings, Resources};

impl Drawing {
    /// The drawing as straight-alpha RGBA bytes of `size`, with the Params at the `values` given and at their defaults
    /// where none is given: rows from the top, four bytes a pixel, so `width * height * 4` bytes; transparent black
    /// where nothing is drawn.
    ///
    /// Shapes are placed as in [`Drawing::to_svg`] and painted in order.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 65,000,
    /// or `size` if it has more than 2^27 pixels; otherwise as [`Drawing::to_svg`].
    pub fn to_rgba(&self, size: Size, values: &Values) -> Result<Vec<u8>, Error> {
        let (width, height) = size.checked_pixels()?;
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), (f64::from(width), f64::from(height)));
        rgba(&shapes, &view, width, height)
    }
}

/// The picture of `shapes` seen through `view`, `width` by `height` pixels.
fn rgba(shapes: &[Shape], view: &View, width: u16, height: u16) -> Result<Vec<u8>, Error> {
    let mut context = RenderContext::new_with(
        width,
        height,
        RenderSettings {
            level: Level::new(),
            num_threads: 0,
        },
    );
    context.set_fill_rule(Fill::NonZero);
    for shape in shapes {
        paint(&mut context, shape, view)?;
    }
    context.flush();
    let mut pixmap = Pixmap::new(width, height);
    context.render_with(
        &mut pixmap,
        &mut Resources::new(),
        RasterizerSettings {
            render_mode: RenderMode::OptimizeSpeed,
            ..RasterizerSettings::default()
        },
    );
    let mut bytes = pixmap.take_rgba8(ImageAlphaType::AlphaPremultiplied);
    unpremultiply(&mut bytes);
    Ok(bytes)
}

/// Paints `shape` seen through `view`: its fill, then its stroke, as one group to which the opacity and the blend apply
/// once (as SVG's `opacity` does), so that a stroke over a fill at half opacity is no darker where they overlap.
fn paint(context: &mut RenderContext, shape: &Shape, view: &View) -> Result<(), Error> {
    let path = view.path(shape.path()).map_err(|error| in_shape(shape, "", error))?;
    let elements = path.elements();
    let outline = bez_path(&elements);
    let style = shape.style();
    let stroke = match style.stroke() {
        Some(stroke) => {
            let width = view
                .length(stroke.width())
                .map_err(|error| in_shape(shape, "stroke width ", error))?;
            Some((stroke, width))
        }
        None => None,
    };
    let grouped = style.opacity() < 1.0 || style.blend() == Blend::Multiply;
    if grouped {
        let mix = match style.blend() {
            Blend::Normal => Mix::Normal,
            Blend::Multiply => Mix::Multiply,
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "an opacity from 0 to 1 loses nothing visible in an f32"
        )]
        let opacity = style.opacity() as f32;
        context.push_layer(
            None,
            Some(BlendMode::new(mix, Compose::SrcOver)),
            Some(opacity),
            None,
            None,
        );
    }
    if let Some([red, green, blue, alpha]) = style.fill().rgba() {
        context.set_paint(AlphaColor::from_rgba8(red, green, blue, alpha));
        context.fill_path(&outline);
    }
    if let Some((stroke, width)) = stroke
        && let Some([red, green, blue, alpha]) = stroke.color().rgba()
    {
        context.set_stroke(
            kurbo::Stroke::new(width)
                .with_join(join(stroke.join()))
                .with_caps(cap(stroke.cap()))
                .with_miter_limit(stroke.miter_limit()),
        );
        context.set_paint(AlphaColor::from_rgba8(red, green, blue, alpha));
        context.stroke_path(&outline);
        for dot in dots(&elements, stroke.cap(), width)? {
            context.fill_path(&bez_path(&dot.elements()));
        }
    }
    if grouped {
        context.pop_layer();
    }
    Ok(())
}

/// A sub-path read for the dot it may leave.
struct Sub {
    start: Point,
    /// It has a segment or a close.
    drawn: bool,
    /// Some segment reaches a point other than its start.
    moved: bool,
}

/// The shapes that the sub-paths of `elements` which never leave their start leave under a round or square end of a
/// stroke `width` wide: a disc or a square about the point, as SVG draws them. The stroker draws nothing for them, as
/// it does for a butt end; a lone move leaves nothing.
fn dots(elements: &[Element], cap: Cap, width: f64) -> Result<Vec<Path>, Error> {
    let mut subs: Vec<Sub> = Vec::new();
    for element in elements {
        let points: &[Point] = match element {
            Element::Move(start) => {
                subs.push(Sub {
                    start: *start,
                    drawn: false,
                    moved: false,
                });
                continue;
            }
            Element::Line(to) => &[*to],
            Element::Quad(control, to) => &[*control, *to],
            Element::Cubic(first, second, to) => &[*first, *second, *to],
            Element::Close => &[],
        };
        if let Some(sub) = subs.last_mut() {
            sub.drawn = true;
            sub.moved |= points.iter().any(|point| *point != sub.start);
        }
    }
    let half = width / 2.0;
    subs.into_iter()
        .filter(|sub| sub.drawn && !sub.moved && cap != Cap::Butt)
        .map(|sub| match cap {
            Cap::Square => Path::rect(sub.start.x - half, sub.start.y - half, width, width, 0.0),
            _ => Path::circle(sub.start, half),
        })
        .collect()
}

fn join(join: Join) -> kurbo::Join {
    match join {
        Join::Round => kurbo::Join::Round,
        Join::Miter => kurbo::Join::Miter,
        Join::Bevel => kurbo::Join::Bevel,
    }
}

fn cap(cap: Cap) -> kurbo::Cap {
    match cap {
        Cap::Butt => kurbo::Cap::Butt,
        Cap::Round => kurbo::Cap::Round,
        Cap::Square => kurbo::Cap::Square,
    }
}

/// `elements` as the rasteriser's path.
fn bez_path(elements: &[Element]) -> BezPath {
    let at = |point: crate::Point| kurbo::Point::new(point.x, point.y);
    let mut outline = BezPath::new();
    for element in elements {
        match *element {
            Element::Move(to) => outline.move_to(at(to)),
            Element::Line(to) => outline.line_to(at(to)),
            Element::Quad(control, to) => outline.quad_to(at(control), at(to)),
            Element::Cubic(first, second, to) => outline.curve_to(at(first), at(second), at(to)),
            Element::Close => outline.close_path(),
        }
    }
    outline
}

/// Turns premultiplied RGBA bytes into straight alpha, in place: each colour byte becomes `colour * 255 / alpha`
/// rounded to the nearest whole number (halves up), and a pixel with alpha 0 becomes all zeros.
fn unpremultiply(bytes: &mut [u8]) {
    let (pixels, _) = bytes.as_chunks_mut::<4>();
    for [red, green, blue, alpha] in pixels {
        let alpha = u32::from(*alpha);
        for channel in [red, green, blue] {
            // a premultiplied colour is at most its alpha, so the quotient is at most 255
            let straight = (u32::from(*channel) * 255 + alpha / 2).checked_div(alpha).unwrap_or(0);
            #[expect(clippy::cast_possible_truncation, reason = "at most 255")]
            let byte = straight as u8;
            *channel = byte;
        }
    }
}
