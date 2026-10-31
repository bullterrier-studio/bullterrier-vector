//! `Drawing::to_rgba`: the drawing as straight-alpha RGBA bytes. The only module that uses the rasteriser crates
//! (`vello_cpu` and `kurbo`); nothing of them reaches the API.

use super::view::View;
use super::{Size, in_shape};
use crate::limits::Work;
use crate::path::{FLATNESS, Polyline};
use crate::{Blend, Cap, Drawing, Error, Join, Path, Point, Shape, Values};
use kurbo::{BezPath, StrokeOpts};
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
    /// or `size` if it has more than 2^27 pixels; naming the shape if a coordinate or the stroke width of one lies
    /// beyond 10 million device pixels at this size; otherwise as [`Drawing::to_svg`].
    pub fn to_rgba(&self, size: Size, values: &Values) -> Result<Vec<u8>, Error> {
        let (width, height) = size.checked_pixels()?;
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), (f64::from(width), f64::from(height)));
        rgba(&shapes, &view, width, height)
    }
}

/// The largest coordinate, and the widest stroke, in device pixels that is handed to the rasteriser.
// constant: the rasteriser computes in f32, which holds whole numbers exactly up to 16,777,216; 10 million pixels is
// far beyond any canvas (at most 65,000 across) and keeps the arithmetic of a join or a curve inside that range
const MAX_DEVICE: f64 = 1.0e7;

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
    let work = Work::new();
    for shape in shapes {
        paint(&mut context, shape, view, &work)?;
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
///
/// The rasteriser is given straight lines only: the path is flattened in design pixels, the same polygon for every
/// output, and a stroke is expanded into its outline here, to the same flatness in output pixels.
fn paint(context: &mut RenderContext, shape: &Shape, view: &View, work: &Work) -> Result<(), Error> {
    let name = format!("shape {}", shape.index());
    let mut charge = |count| work.made_segments(count, &name);
    let polylines = shape.path().flattened(&mut charge)?;
    let device = moved(&polylines, view).map_err(|error| in_shape(shape, "", error))?;
    let outline = bez_path(&device);
    let style = shape.style();
    let stroke = match style.stroke() {
        Some(stroke) => {
            let width = view
                .length(stroke.width())
                .map_err(|error| in_shape(shape, "stroke width ", error))?;
            within_device("the stroke width", width).map_err(|error| in_shape(shape, "", error))?;
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
        let expanded = kurbo::Stroke::new(width)
            .with_join(join(stroke.join()))
            .with_caps(cap(stroke.cap()))
            .with_miter_limit(stroke.miter_limit());
        let flatness = FLATNESS * view.scale();
        context.set_paint(AlphaColor::from_rgba8(red, green, blue, alpha));
        // the outline has arcs for the round joins and ends; the rasteriser must be given lines only
        let curved = kurbo::stroke(
            outline.elements().iter().copied(),
            &expanded,
            &StrokeOpts::default(),
            flatness,
        );
        let mut lines = BezPath::new();
        kurbo::flatten(curved, flatness, |element| lines.push(element));
        context.fill_path(&lines);
        for dot in dots(&device, stroke.cap(), width, &mut charge)? {
            context.fill_path(&bez_path(&dot));
        }
    }
    if grouped {
        context.pop_layer();
    }
    Ok(())
}

/// `polylines` moved by `view` into output pixels.
///
/// # Errors
/// [`Error::Parameter`] if a point lies beyond [`MAX_DEVICE`].
fn moved(polylines: &[Polyline], view: &View) -> Result<Vec<Polyline>, Error> {
    let mut device = Vec::with_capacity(polylines.len());
    for polyline in polylines {
        let points: Vec<Point> = polyline.points.iter().map(|point| view.point(*point)).collect();
        let farthest = points
            .iter()
            .fold(0.0, |far: f64, point| far.max(point.x.abs()).max(point.y.abs()));
        within_device("a coordinate", farthest)?;
        device.push(Polyline {
            points,
            closed: polyline.closed,
            drawn: polyline.drawn,
        });
    }
    Ok(device)
}

/// `value` (a coordinate or a width, `what`), or the error unless it is within [`MAX_DEVICE`].
fn within_device(what: &str, value: f64) -> Result<(), Error> {
    if value <= MAX_DEVICE {
        Ok(())
    } else {
        Err(Error::parameter(
            "device",
            format!("{what} of {value} device pixels is beyond {MAX_DEVICE}"),
        ))
    }
}

/// The shapes that the sub-paths which never leave their start leave under a round or square end of a stroke `width`
/// wide: a disc or a square about the point, as SVG draws them. The stroker draws nothing for them, as it does for a
/// butt end; a lone move leaves nothing.
fn dots(
    polylines: &[Polyline],
    cap: Cap,
    width: f64,
    charge: &mut dyn FnMut(usize) -> Result<(), Error>,
) -> Result<Vec<Vec<Polyline>>, Error> {
    let half = width / 2.0;
    let mut dots = Vec::new();
    for polyline in polylines {
        let [at] = polyline.points[..] else { continue };
        if !polyline.drawn || cap == Cap::Butt {
            continue;
        }
        let shape = match cap {
            Cap::Square => Path::rect(at.x - half, at.y - half, width, width, 0.0)?,
            _ => Path::circle(at, half)?,
        };
        dots.push(shape.flattened(charge)?);
    }
    Ok(dots)
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

/// `polylines` as the rasteriser's path: straight lines only.
fn bez_path(polylines: &[Polyline]) -> BezPath {
    let at = |point: Point| kurbo::Point::new(point.x, point.y);
    let mut outline = BezPath::new();
    for polyline in polylines {
        let mut points = polyline.points.iter();
        if let Some(start) = points.next() {
            outline.move_to(at(*start));
        }
        for point in points {
            outline.line_to(at(*point));
        }
        if polyline.closed {
            outline.close_path();
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
