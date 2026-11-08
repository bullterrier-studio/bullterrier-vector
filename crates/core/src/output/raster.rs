//! `Drawing::to_rgba`: the drawing as straight-alpha RGBA bytes. The only module that uses the rasteriser crates
//! (`vello_cpu` and `kurbo`); nothing of them reaches the API.

use super::cast;
use super::outline::{bez_path, dots, stroke_outline, within_device};
use super::view::View;
use super::{Size, in_shape};
use crate::limits::Work;
use crate::path::{FLATNESS, Polyline};
use crate::{Blend, Cap, Drawing, Error, Join, Point, ShadowStyle, Shape, StrokeStyle, Style, Values};
use kurbo::{Affine, BezPath, Rect, Shape as _};
use std::f64::consts::SQRT_2;
use vello_cpu::color::AlphaColor;
use vello_cpu::filter_effects::{EdgeMode, Filter, FilterPrimitive};
use vello_cpu::peniko::{BlendMode, Compose, Fill, ImageAlphaType, Mix};
use vello_cpu::{Level, Pixmap, RasterizerSettings, RenderContext, RenderMode, RenderSettings, Resources};

/// The largest deviation of a shadow's blur, in output pixels.
// constant: measured on this machine in release builds, one layer at a deviation of 1000 takes 0.3 s and one of 3000
// 2.8 s, and one of 5000 does not return; 1000 is a blur radius of 2000 pixels, more than any picture asks for
const MAX_SIGMA: f64 = 1000.0;

impl Drawing {
    /// The drawing as straight-alpha RGBA bytes of `size`, with the Params at the `values` given and at their defaults
    /// where none is given: rows from the top, four bytes a pixel, so `width * height * 4` bytes; transparent black
    /// where nothing is drawn.
    ///
    /// Shapes are placed as in [`Drawing::to_svg`] and painted in order. A shape that knocks out paints nothing: the
    /// pixels its fill and stroke cover, as far as they cover them, are taken out of what the shapes before it painted
    /// (an edge pixel half covered loses half its alpha), whatever the colours, opacity, blend and shadow of the shape.
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
        paint(&mut context, shape, view, (width, height), &work)?;
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

/// What a shape is made of once placed: the fill and the stroke it paints.
struct Body {
    outline: BezPath,
    fill: Option<[u8; 4]>,
    stroke: Option<StrokeBody>,
}

/// A stroke ready to be painted: its outline as lines, the dots of its round or square ends, and its colour.
struct StrokeBody {
    lines: BezPath,
    dots: Vec<BezPath>,
    rgba: [u8; 4],
}

impl Body {
    /// Paints the fill, then the stroke; with a `tint`, in that colour and their own alphas.
    fn draw(&self, context: &mut RenderContext, tint: Option<[u8; 3]>) {
        let tinted = |[red, green, blue, alpha]: [u8; 4]| match tint {
            Some([red, green, blue]) => AlphaColor::from_rgba8(red, green, blue, alpha),
            None => AlphaColor::from_rgba8(red, green, blue, alpha),
        };
        if let Some(rgba) = self.fill {
            context.set_paint(tinted(rgba));
            context.fill_path(&self.outline);
        }
        let Some(StrokeBody { lines, dots, rgba }) = &self.stroke else {
            return;
        };
        let alpha = rgba[3];
        // the outline and the dots overlap, and a translucent colour painted twice where they do would be darker than
        // SVG's one stroke: they are painted opaque into a layer that has the colour's alpha
        let layered = alpha < 255 && !dots.is_empty();
        if layered {
            context.push_layer(None, None, Some(f32::from(alpha) / 255.0), None, None);
        }
        context.set_paint(tinted([rgba[0], rgba[1], rgba[2], if layered { 255 } else { alpha }]));
        context.fill_path(lines);
        for dot in dots {
            context.fill_path(dot);
        }
        if layered {
            context.pop_layer();
        }
    }
}

/// Paints `shape` seen through `view`, or if it knocks out, removes the area its fill and stroke cover from the pixels
/// painted so far. Otherwise: its shadow, then its fill, then its stroke, as one group to which the opacity
/// and the blend apply once (as SVG's `opacity` does), so that a stroke over a fill at half opacity is no darker where
/// they overlap, and neither is a shadow under a shape.
///
/// The rasteriser is given straight lines only: the path is flattened in design pixels, the same polygon for every
/// output, and a stroke is expanded into its outline here, to the same flatness in output pixels. The shadow is the
/// fill and stroke drawn once more into a filter layer, which keeps only their alpha, blurs, offsets and tints it.
fn paint(
    context: &mut RenderContext,
    shape: &Shape,
    view: &View,
    canvas: (u16, u16),
    work: &Work,
) -> Result<(), Error> {
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
    // a shape that knocks out paints nothing of its own, its shadow included
    let knockout = style.knockout();
    let shadow = match cast(style) {
        Some((style, rgba)) if !knockout => {
            Some(Shadow::new(style, rgba, view).map_err(|error| in_shape(shape, "shadow ", error))?)
        }
        _ => None,
    };
    // a group is composited over the pixels it can reach only, not the whole canvas: that is what keeps a drawing of
    // thousands of small translucent or multiplied shapes from costing a canvas each
    let reach = stroke.map_or(0.0, |(stroke, width)| stroke_reach(stroke, width));
    let shadow_reach = shadow.as_ref().map_or(0.0, Shadow::reach);
    let Some(bounds) = visible(&device, reach + shadow_reach, canvas) else {
        return Ok(());
    };
    if let Some(shadow) = &shadow
        && shadow.sigma > 0.0
    {
        // the layer a blur works in is the part of the shape that can reach the canvas, grown by what the blur reaches
        let near = within(&extent(&device, reach), &grown(&canvas_rect(canvas), shadow_reach));
        let layer = near.map_or(0.0, |near| area(&grown(&near, shadow_reach)));
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an area of at most 4 * 10^7 squared pixels, rounded up, is a whole number far below 2^63"
        )]
        work.filtered(layer.ceil() as u64, &name)?;
    }
    // what a knockout removes is the area covered, whatever the alpha of the colours that would paint it
    let covering = |rgba: [u8; 4]| {
        if knockout {
            [rgba[0], rgba[1], rgba[2], 255]
        } else {
            rgba
        }
    };
    let body = Body {
        outline,
        fill: style.fill().rgba().map(covering),
        stroke: match stroke {
            Some((stroke, width)) => match stroke.color().rgba() {
                Some(rgba) => {
                    // the outline has arcs for the round joins and ends; the rasteriser must be given lines only
                    let lines = stroke_outline(
                        &bez_path(&device),
                        stroke,
                        (stroke.join(), stroke.cap()),
                        width,
                        FLATNESS * view.scale(),
                        &name,
                    )?;
                    let dots = dots(&device, stroke.cap(), width, &mut charge)?
                        .iter()
                        .map(|dot| bez_path(dot))
                        .collect();
                    Some(StrokeBody {
                        lines,
                        dots,
                        rgba: covering(rgba),
                    })
                }
                None => None,
            },
            None => None,
        },
    };
    composite(context, style, shadow.as_ref(), &body, &bounds);
    Ok(())
}

/// Paints `body` into `context` within `bounds` as `style` says: taken out of what is below it, if it knocks out; else
/// its shadow and then itself, in a group that the opacity and the blend apply to once, if there is an opacity or a blend.
fn composite(context: &mut RenderContext, style: &Style, shadow: Option<&Shadow>, body: &Body, bounds: &Rect) {
    if style.knockout() {
        // the body painted into a layer that is taken out of what is below it, within the part of the canvas it reaches
        context.push_layer(
            Some(&path_of(bounds)),
            Some(BlendMode::new(Mix::Normal, Compose::DestOut)),
            None,
            None,
            None,
        );
        body.draw(context, None);
        context.pop_layer();
        return;
    }
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
            Some(&path_of(bounds)),
            Some(BlendMode::new(mix, Compose::SrcOver)),
            Some(opacity),
            None,
            None,
        );
    }
    if let Some(shadow) = shadow {
        shadow.cast(context, body, &path_of(bounds));
    }
    body.draw(context, None);
    if grouped {
        context.pop_layer();
    }
}

/// The shadow of a shape in output pixels.
struct Shadow {
    dx: f64,
    dy: f64,
    /// The Gaussian's standard deviation.
    sigma: f64,
    rgba: [u8; 4],
}

impl Shadow {
    /// The shadow `style` casts seen through `view`.
    ///
    /// # Errors
    /// [`Error::Parameter`] if the offset is beyond [`MAX_DEVICE`] or the deviation beyond [`MAX_SIGMA`] output pixels.
    fn new(style: &ShadowStyle, rgba: [u8; 4], view: &View) -> Result<Self, Error> {
        let (dx, dy) = (view.length(style.dx())?, view.length(style.dy())?);
        within_device("offset", dx.abs().max(dy.abs()))?;
        let sigma = view.length(style.blur() / 2.0)?;
        if sigma > MAX_SIGMA {
            return Err(Error::parameter(
                "blur",
                format!(
                    "blur of {} design pixels is a deviation of {sigma} output pixels at this size, beyond {MAX_SIGMA}",
                    style.blur()
                ),
            ));
        }
        Ok(Self { dx, dy, sigma, rgba })
    }

    /// Paints the shadow of `body` into `context`, within `bounds`.
    ///
    /// With a blur it is the body drawn into a filter layer. Without, it is the body moved and drawn in the shadow's
    /// colour, as one layer with the shadow's alpha, as SVG's copy is: a filter would move it by whole pixels.
    fn cast(&self, context: &mut RenderContext, body: &Body, bounds: &BezPath) {
        let [red, green, blue, alpha] = self.rgba;
        if self.sigma > 0.0 {
            context.push_filter_layer(self.filter());
            body.draw(context, None);
            context.pop_layer();
            return;
        }
        let layered = alpha < 255;
        if layered {
            context.push_layer(Some(bounds), None, Some(f32::from(alpha) / 255.0), None, None);
        }
        context.set_transform(Affine::translate((self.dx, self.dy)));
        body.draw(context, Some([red, green, blue]));
        context.reset_transform();
        if layered {
            context.pop_layer();
        }
    }

    /// How far the shadow lies from its shape at most, in output pixels: the offset, and three deviations, beyond which
    /// the Gaussian is under 0.2%.
    fn reach(&self) -> f64 {
        self.dx.abs().max(self.dy.abs()) + 3.0 * self.sigma
    }

    /// The filter that keeps the alpha of what is drawn into its layer, moves and blurs it and gives it the colour.
    #[expect(clippy::cast_possible_truncation, reason = "the filter computes in f32")]
    fn filter(&self) -> Filter {
        let [red, green, blue, alpha] = self.rgba;
        Filter::from_primitive(FilterPrimitive::DropShadowOnly {
            dx: self.dx as f32,
            dy: self.dy as f32,
            std_deviation: self.sigma as f32,
            color: AlphaColor::from_rgba8(red, green, blue, alpha),
            edge_mode: EdgeMode::None,
        })
    }
}

/// How far a stroke `width` wide with this join and end reaches from the centreline of the path: a miter to its limit
/// of half widths, a square end to the corner of its square.
fn stroke_reach(stroke: &StrokeStyle, width: f64) -> f64 {
    let join = if stroke.join() == Join::Miter {
        stroke.miter_limit()
    } else {
        1.0
    };
    let end = if stroke.cap() == Cap::Square { SQRT_2 } else { 1.0 };
    width / 2.0 * join.max(end)
}

/// The box a path of `polylines` stroked out to `reach` can touch, a pixel more on each side for the edge of an
/// anti-aliased fill; empty (`left > right`) if it has no points.
fn extent(polylines: &[Polyline], reach: f64) -> Rect {
    let (mut left, mut top, mut right, mut bottom) =
        (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for point in polylines.iter().flat_map(|polyline| &polyline.points) {
        (left, right) = (left.min(point.x), right.max(point.x));
        (top, bottom) = (top.min(point.y), bottom.max(point.y));
    }
    grown(&Rect::new(left, top, right, bottom), reach + 1.0)
}

/// `rect` with `by` more on every side.
fn grown(rect: &Rect, by: f64) -> Rect {
    Rect::new(rect.x0 - by, rect.y0 - by, rect.x1 + by, rect.y1 + by)
}

/// The canvas as a box.
fn canvas_rect((width, height): (u16, u16)) -> Rect {
    Rect::new(0.0, 0.0, f64::from(width), f64::from(height))
}

/// The part of `rect` inside `limit`, if it has any area.
fn within(rect: &Rect, limit: &Rect) -> Option<Rect> {
    let both = Rect::new(
        rect.x0.max(limit.x0),
        rect.y0.max(limit.y0),
        rect.x1.min(limit.x1),
        rect.y1.min(limit.y1),
    );
    (both.x0 < both.x1 && both.y0 < both.y1).then_some(both)
}

/// The area of `rect`.
fn area(rect: &Rect) -> f64 {
    (rect.x1 - rect.x0) * (rect.y1 - rect.y0)
}

/// The path of `rect`.
fn path_of(rect: &Rect) -> BezPath {
    BezPath::from_vec(rect.path_elements(0.0).collect())
}

/// The whole pixels of the canvas that a path of `polylines` stroked out to `reach` can touch, as a box; nothing if it
/// touches none.
fn visible(polylines: &[Polyline], reach: f64, canvas: (u16, u16)) -> Option<Rect> {
    let touched = extent(polylines, reach);
    within(
        &Rect::new(
            touched.x0.floor(),
            touched.y0.floor(),
            touched.x1.ceil(),
            touched.y1.ceil(),
        ),
        &canvas_rect(canvas),
    )
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
