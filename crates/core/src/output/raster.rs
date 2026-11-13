//! `Drawing::to_rgba`: the drawing as straight-alpha RGBA bytes. The only module that uses the rasteriser crates
//! (`vello_cpu` and `kurbo`); nothing of them reaches the API.

use super::cast;
use super::outline::{bez_path, dots, stroke_outline, within_device};
use super::ramp::Ramp;
use super::screen::{Screen, charged, screen};
use super::view::View;
use super::window::{Cells, Window};
use super::{Size, in_shape};
use crate::limits::Work;
use crate::path::{FLATNESS, Polyline};
use crate::shape::same_cut;
use crate::{
    Blend, Cap, Drawing, Error, GradientKind, Join, Paint, Path, Point, ShadowStyle, Shape, StrokeStyle, Style, Values,
};
use kurbo::{Affine, BezPath, Rect, Shape as _};
use std::f64::consts::SQRT_2;
use std::sync::Arc;
use vello_cpu::color::AlphaColor;
use vello_cpu::filter_effects::{EdgeMode, Filter, FilterPrimitive};
use vello_cpu::peniko::color::DynamicColor;
use vello_cpu::peniko::{BlendMode, ColorStop, Compose, Fill, Gradient, ImageAlphaType, InterpolationAlphaSpace, Mix};
use vello_cpu::{Level, Mask, Pixmap, RasterizerSettings, RenderContext, RenderMode, RenderSettings, Resources};

/// The widest half of the rasteriser's blur kernel, in pixels of its coarsest grid.
const KERNEL_RADIUS: f64 = 6.0; // constant: `MAX_KERNEL_SIZE` 13 of `vello_common`, half of it
/// How far the filter of one halving of the picture reads, in pixels of the grid it halves.
const HALVING_FILTER: f64 = 2.0; // constant: its [1, 3, 3, 1] taps reach two pixels

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
    /// A halftone is its dots in one path filled by the nonzero rule inside a clip to the shape's outline, under the
    /// stroke: a translucent colour is no darker where dots overlap.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 65,000,
    /// or `size` if it has more than 2^27 pixels; naming the shape if a coordinate or the stroke width of one lies
    /// beyond 10 million device pixels at this size; otherwise as [`Drawing::to_svg`].
    pub fn to_rgba(&self, size: Size, values: &Values) -> Result<Vec<u8>, Error> {
        let (width, height) = size.checked_pixels()?;
        self.to_rgba_window(size, Window::px(0.0, 0.0, f64::from(width), f64::from(height)), values)
    }

    /// The pixels of `window`, a rectangle of the output `size`, as [`Drawing::to_rgba`] returns them: the rows and
    /// columns of the whole picture that the window covers, so that a large picture can be drawn in bands without ever
    /// holding it. `size` is the whole output, from which the design frame is placed; the result is
    /// `window.width * window.height * 4` bytes.
    ///
    /// The sides of `size` may be up to 100,000, the limit of a design frame; the window holds the limits of
    /// `to_rgba` (65,000 a side, 2^27 pixels). A blurred shadow is drawn from a region a little larger than the
    /// window, which holds those limits too.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 100,000,
    /// or `window` if it is not whole pixels inside `size`, has a side past 65,000 or more than 2^27 pixels, or needs
    /// a region past those limits for the reach of its shadows; otherwise as [`Drawing::to_rgba`].
    pub fn to_rgba_window(&self, size: Size, window: Window, values: &Values) -> Result<Vec<u8>, Error> {
        let canvas = size.checked()?;
        let window = window.checked(canvas)?;
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), canvas);
        rgba(&shapes, &view, canvas, window)
    }
}

/// The picture of `shapes` seen through `view` in an output of `canvas` pixels, cut to `window`.
fn rgba(shapes: &[Shape], view: &View, canvas: (f64, f64), window: Cells) -> Result<Vec<u8>, Error> {
    let (margin, align) = blur_reach(shapes, view);
    let region = window.region(margin, align, Cells::canvas(canvas))?;
    let view = view.shifted(f64::from(region.x), f64::from(region.y));
    // the region is within the limits of a pixel output, so its sides are at most 65,000
    #[expect(clippy::cast_possible_truncation, reason = "at most 65,000")]
    let pixmap = picture(shapes, &view, (region.width as u16, region.height as u16), &Work::new())?;
    let mut bytes = window.cut(region, &pixmap.take_rgba8(ImageAlphaType::AlphaPremultiplied));
    unpremultiply(&mut bytes);
    Ok(bytes)
}

/// How far a window's pixels depend on pixels around it, in output pixels, and the grid the blurs' decimation needs
/// the region's top left to lie on: what the blurred shadows of `shapes` (and of the shapes of their masks) ask for.
/// Every other effect is placed by its coordinates alone and asks for nothing.
fn blur_reach(shapes: &[Shape], view: &View) -> (u32, u32) {
    let (mut margin, mut align) = (0.0_f64, 1_u32);
    let masks = shapes
        .iter()
        .filter_map(|shape| shape.style().shared_mask())
        .flat_map(|mask| mask.iter());
    for shape in shapes.iter().chain(masks) {
        // a shadow that `paint` refuses is refused there, in the order of the draw
        let Some((style, rgba)) = cast(shape.style()) else {
            continue;
        };
        let Ok(shadow) = Shadow::new(style, rgba, view) else {
            continue;
        };
        if shadow.sigma > 0.0 {
            margin = margin.max(shadow.dependence());
            align = align.max(1 << decimations(shadow.sigma));
        }
    }
    // a margin past 2^32 pixels is past any canvas, and the region is cut to the canvas
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a whole number, saturating"
    )]
    (margin.ceil() as u32, align)
}

/// How many times the rasteriser halves the picture to blur with a deviation of `sigma` pixels: it removes a variance
/// of 1.5 a level and leaves one of at most 4 for a direct convolution (`vello_common`'s `plan_decimated_blur`).
fn decimations(sigma: f64) -> u32 {
    #[expect(clippy::cast_possible_truncation, reason = "the rasteriser computes in f32")]
    let mut variance = (sigma * sigma) as f32;
    let mut levels = 0;
    while variance > 4.0 {
        variance = (variance - 1.5) * 0.25;
        levels += 1;
    }
    levels
}

/// `shapes` painted in order into a new pixmap of `canvas` pixels, premultiplied.
fn picture(shapes: &[Shape], view: &View, canvas: (u16, u16), work: &Work) -> Result<Pixmap, Error> {
    let (width, height) = canvas;
    let mut context = RenderContext::new_with(
        width,
        height,
        RenderSettings {
            level: Level::new(),
            num_threads: 0,
        },
    );
    context.set_fill_rule(Fill::NonZero);
    let mut cuts = Cuts::default();
    for shape in shapes {
        paint(&mut context, shape, view, canvas, (work, &mut cuts))?;
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
    Ok(pixmap)
}

/// The clips and masks a draw has made, told apart by `same_cut` (the `Arc` the steps share between their shapes, or an equal one), so that a
/// headline of fifty glyphs faded by one mask builds that mask once.
#[derive(Default)]
struct Cuts {
    clips: Vec<(Arc<Path>, Arc<BezPath>)>,
    masks: Vec<(Arc<[Shape]>, Mask)>,
}

/// How a shape is cut: to a region, and faded by a mask.
struct Cut {
    clip: Option<Arc<BezPath>>,
    mask: Option<Mask>,
}

impl Cuts {
    /// The clip of `style` in output pixels, made if it is the first of its step.
    ///
    /// # Errors
    /// [`Error::Parameter`] if the region would pass `MAX_SEGMENTS` or lies beyond [`MAX_DEVICE`] device pixels.
    fn clip(&mut self, style: &Style, view: &View, work: &Work, name: &str) -> Result<Option<Arc<BezPath>>, Error> {
        let Some(region) = style.shared_clip() else {
            return Ok(None);
        };
        if let Some((_, made)) = self.clips.iter().find(|(known, _)| same_cut(known, region)) {
            return Ok(Some(Arc::clone(made)));
        }
        let polylines = region.flattened(&mut |count| work.made_segments(count, &format!("{name} clip")))?;
        let made = Arc::new(bez_path(&moved(&polylines, view)?));
        self.clips.push((Arc::clone(region), Arc::clone(&made)));
        Ok(Some(made))
    }

    /// The mask of `style` as pixels of the canvas, made if it is the first of its step: the shapes of the mask drawn
    /// apart, their luminance by the alpha they cover. Its pixels are charged once.
    ///
    /// # Errors
    /// [`Error::Parameter`] if the canvas pixels of the masks of the draw pass `MAX_FILTER_PIXELS`, or what the shapes
    /// of the mask are refused for.
    fn mask(
        &mut self,
        style: &Style,
        view: &View,
        canvas: (u16, u16),
        (work, name): (&Work, &str),
    ) -> Result<Option<Mask>, Error> {
        let Some(shapes) = style.shared_mask() else {
            return Ok(None);
        };
        if let Some((_, made)) = self.masks.iter().find(|(known, _)| same_cut(known, shapes)) {
            return Ok(Some(made.clone()));
        }
        work.filtered(u64::from(canvas.0) * u64::from(canvas.1), name, "its mask")?;
        // a mask must be the size of the canvas, or the rasteriser leaves it out without a word
        let pixmap = picture(shapes, view, canvas, work).map_err(|error| error.inside(&format!("{name} mask")))?;
        let made = Mask::new_luminance(&pixmap);
        self.masks.push((Arc::clone(shapes), made.clone()));
        Ok(Some(made))
    }
}

/// What a shape is made of once placed: the fill and the stroke it paints.
struct Body {
    outline: BezPath,
    fill: Option<FillBody>,
    stroke: Option<StrokeBody>,
}

/// A fill ready to be painted.
enum FillBody {
    /// The whole outline in a colour.
    Plain([u8; 4]),
    /// The whole outline in a gradient.
    Ramp(Ramp),
    /// A screen of dots in a colour, as lines, to be clipped to the outline.
    Dots { rgba: [u8; 4], dots: BezPath },
}

/// A stroke ready to be painted: its outline as lines, the dots of its round or square ends, the box that holds them
/// all, and its paint.
struct StrokeBody {
    lines: BezPath,
    dots: Vec<BezPath>,
    bounds: BezPath,
    paint: StrokePaint,
}

/// The paint of a stroke.
enum StrokePaint {
    Plain([u8; 4]),
    Ramp(Ramp),
}

/// `ramp` as the rasteriser has it: unpremultiplied stops in sRGB, held at the ends, as SVG interpolates them. The
/// rasteriser's own default is premultiplied, which differs wherever the stops differ in alpha.
fn gradient(ramp: &Ramp) -> Gradient {
    // SVG holds the first stop before offset 0 and the last after offset 1. The rasteriser holds the stop it sorts last
    // at a shared end offset instead, so the earlier stop at 1 moves one f32 step in, and a line whose first two
    // stops share offset 0 is made longer at its start by `LEAD` of its length, with the first colour in front.
    let mut kind = ramp.kind;
    let mut stops = ramp.stops.clone();
    if let GradientKind::Linear { start, end } = &mut kind
        && stops.get(1).is_some_and(|(offset, _)| *offset <= 0.0)
    {
        *start = Point::new(start.x - (end.x - start.x) * LEAD, start.y - (end.y - start.y) * LEAD);
        for (offset, _) in &mut stops {
            *offset = (*offset + LEAD) / (1.0 + LEAD);
        }
        stops.insert(0, (0.0, stops[0].1));
    }
    let last = stops.len() - 1;
    let stops: Vec<ColorStop> = stops
        .iter()
        .enumerate()
        .map(|(index, (offset, [red, green, blue, alpha]))| ColorStop {
            #[expect(clippy::cast_possible_truncation, reason = "the rasteriser computes in f32")]
            offset: if *offset >= 1.0 && index < last {
                1.0 - f32::EPSILON
            } else {
                *offset as f32
            },
            color: DynamicColor::from_alpha_color(AlphaColor::from_rgba8(*red, *green, *blue, *alpha)),
        })
        .collect();
    let gradient = match kind {
        GradientKind::Linear { start, end } => Gradient::new_linear((start.x, start.y), (end.x, end.y)),
        #[expect(clippy::cast_possible_truncation, reason = "the rasteriser computes in f32")]
        GradientKind::Radial { center, radius } => Gradient::new_radial((center.x, center.y), radius as f32),
    };
    gradient
        .with_interpolation_alpha_space(InterpolationAlphaSpace::Unpremultiplied)
        .with_stops(stops.as_slice())
}

/// How much of a line's length is added in front of it to hold the first colour there (see `gradient`).
const LEAD: f64 = 1.0 / 64.0; // constant: any share well above the rasteriser's colour table step (1/1024)

impl Body {
    /// Paints the fill, then the stroke; with a `tint`, in that colour and their own alphas.
    fn draw(&self, context: &mut RenderContext, tint: Option<[u8; 3]>) {
        let tinted = |[red, green, blue, alpha]: [u8; 4]| match tint {
            Some([red, green, blue]) => AlphaColor::from_rgba8(red, green, blue, alpha),
            None => AlphaColor::from_rgba8(red, green, blue, alpha),
        };
        // a shadow's gradient is the same in the tint, with the stops' own alphas
        let ramped = |ramp: &Ramp| gradient(&tint.map_or_else(|| ramp.clone(), |rgb| ramp.tinted(rgb)));
        match &self.fill {
            Some(FillBody::Plain(rgba)) => {
                context.set_paint(tinted(*rgba));
                context.fill_path(&self.outline);
            }
            Some(FillBody::Ramp(ramp)) => {
                context.set_paint(ramped(ramp));
                context.fill_path(&self.outline);
            }
            // the dots are one path, filled once by the nonzero rule, so that a translucent colour is no darker where
            // two overlap, as SVG's one path is not
            Some(FillBody::Dots { rgba, dots }) => {
                context.push_clip_path(&self.outline);
                context.set_paint(tinted(*rgba));
                context.fill_path(dots);
                context.pop_clip();
            }
            None => {}
        }
        let Some(StrokeBody {
            lines,
            dots,
            bounds,
            paint,
        }) = &self.stroke
        else {
            return;
        };
        match paint {
            StrokePaint::Plain(rgba) => {
                let alpha = rgba[3];
                // the outline and the dots overlap, and a translucent colour painted twice where they do would be
                // darker than SVG's one stroke: they are painted opaque into a layer that has the colour's alpha
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
            StrokePaint::Ramp(ramp) if dots.is_empty() => {
                context.set_paint(ramped(ramp));
                context.fill_path(lines);
            }
            // the pieces overlap and the gradient has an alpha of its own: they are painted opaque into a layer, which
            // is then given the gradient wherever it is covered (`SrcIn`), so that no pixel is painted twice
            StrokePaint::Ramp(ramp) => {
                context.push_layer(None, None, None, None, None);
                context.set_paint(AlphaColor::from_rgba8(0, 0, 0, 255));
                context.fill_path(lines);
                for dot in dots {
                    context.fill_path(dot);
                }
                context.set_blend_mode(BlendMode::new(Mix::Normal, Compose::SrcIn));
                context.set_paint(ramped(ramp));
                context.fill_path(bounds);
                context.set_blend_mode(BlendMode::new(Mix::Normal, Compose::SrcOver));
                context.pop_layer();
            }
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
    (work, cuts): (&Work, &mut Cuts),
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
    // a knockout takes the area it covers whatever it is clipped to or masked by
    let clip = if knockout {
        None
    } else {
        cuts.clip(style, view, work, &name)?
    };
    let Some(bounds) = visible(&device, reach + shadow_reach, canvas) else {
        // a shape off the canvas is drawn by the other outputs, so a pitch they refuse is refused here too
        if let Some(halftone) = style.halftone() {
            charged(halftone, &polylines, &mut |count| {
                work.made_segments(count, &format!("shape {} pitch", shape.index()))
            })?;
        }
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
        work.filtered(layer.ceil() as u64, &name, "the blur of its shadow")?;
    }
    let fill = fill_body(shape, &polylines, view, work, knockout)?;
    let body = Body {
        outline,
        fill,
        stroke: match stroke {
            Some((stroke, width)) => match stroke_paint(stroke.paint(), view, knockout)
                .map_err(|error| in_shape(shape, "gradient ", error))?
            {
                Some(paint) => {
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
                        bounds: path_of(&extent(&device, reach)),
                        paint,
                    })
                }
                None => None,
            },
            None => None,
        },
    };
    let mask = if knockout {
        None
    } else {
        cuts.mask(style, view, canvas, (work, &name))?
    };
    composite(context, style, shadow.as_ref(), &body, (&bounds, &Cut { clip, mask }));
    Ok(())
}

/// The paint of a stroke seen through `view`; `None` if it paints nothing. A knockout takes the area covered, whatever the
/// paint, so a gradient is opaque for it.
///
/// # Errors
/// [`Error::Parameter`] if a gradient lies beyond [`MAX_DEVICE`] device pixels.
fn stroke_paint(paint: &Paint, view: &View, knockout: bool) -> Result<Option<StrokePaint>, Error> {
    Ok(match paint {
        Paint::Color(color) => color.rgba().map(|rgba| StrokePaint::Plain(covering(knockout, rgba))),
        Paint::Gradient(_) if knockout => Some(StrokePaint::Plain(OPAQUE_BLACK)),
        Paint::Gradient(gradient) => Some(StrokePaint::Ramp(Ramp::new(gradient, view)?)),
    })
}

/// What a knockout paints a gradient as: the area covered, in any colour.
const OPAQUE_BLACK: [u8; 4] = [0, 0, 0, 255];

/// `rgba` as it paints: what a knockout removes is the area covered, whatever the alpha of the colour that would paint it.
fn covering(knockout: bool, rgba: [u8; 4]) -> [u8; 4] {
    if knockout {
        [rgba[0], rgba[1], rgba[2], 255]
    } else {
        rgba
    }
}

/// The fill of `shape`, whose path flattened is `polylines`, seen through `view`: a colour, or the dots of a halftone as
/// lines; `None` if it paints nothing. A halftone of the tone 0 is the colour, and `work` is charged for the dots of the
/// others.
///
/// # Errors
/// [`Error::Parameter`] if the dots would be past `MAX_SEGMENTS` or lie beyond [`MAX_DEVICE`] device pixels.
fn fill_body(
    shape: &Shape,
    polylines: &[Polyline],
    view: &View,
    work: &Work,
    knockout: bool,
) -> Result<Option<FillBody>, Error> {
    let style = shape.style();
    let Some(halftone) = style.halftone() else {
        return Ok(match style.fill() {
            Paint::Color(color) => color.rgba().map(|rgba| FillBody::Plain(covering(knockout, rgba))),
            Paint::Gradient(_) if knockout => Some(FillBody::Plain(OPAQUE_BLACK)),
            Paint::Gradient(gradient) => Some(FillBody::Ramp(
                Ramp::new(gradient, view).map_err(|error| in_shape(shape, "gradient ", error))?,
            )),
        });
    };
    let screened = screen(halftone, polylines, &mut |count| {
        work.made_segments(count, &format!("shape {} pitch", shape.index()))
    })?;
    let Some(rgba) = halftone.color().rgba().map(|rgba| covering(knockout, rgba)) else {
        return Ok(None);
    };
    let found = match screened {
        Screen::Solid => return Ok(Some(FillBody::Plain(rgba))),
        Screen::Dots(found) if found.is_empty() => return Ok(None),
        Screen::Dots(found) => found,
    };
    let mut dots = BezPath::new();
    for dot in found.all() {
        let corners: Vec<Point> = dot.iter().map(|corner| view.point(*corner)).collect();
        let farthest = corners
            .iter()
            .fold(0.0, |far: f64, corner| far.max(corner.x.abs()).max(corner.y.abs()));
        within_device("a dot", farthest).map_err(|error| in_shape(shape, "", error))?;
        dots.extend(bez_path(&[Polyline {
            points: corners,
            closed: true,
            drawn: true,
        }]));
    }
    Ok(Some(FillBody::Dots { rgba, dots }))
}

/// Paints `body` into `context` within `bounds` as `style` says: taken out of what is below it, if it knocks out; else
/// its shadow and then itself, in a group that the opacity and the blend apply to once, if there is an opacity or a blend.
fn composite(
    context: &mut RenderContext,
    style: &Style,
    shadow: Option<&Shadow>,
    body: &Body,
    (bounds, cut): (&Rect, &Cut),
) {
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
    let grouped = style.opacity() < 1.0 || style.blend() != Blend::Normal || cut.clip.is_some() || cut.mask.is_some();
    if grouped {
        let mix = mix_of(style.blend());
        #[expect(
            clippy::cast_possible_truncation,
            reason = "an opacity from 0 to 1 loses nothing visible in an f32"
        )]
        let opacity = style.opacity() as f32;
        // the clip and the mask belong to the group the opacity and the blend do (as SVG's `<g>` has them), so that the
        // shadow is cut and a blend mixes with what is below; the clip is inside the bounds the group can reach
        let reach = path_of(bounds);
        context.push_layer(
            Some(cut.clip.as_deref().unwrap_or(&reach)),
            Some(BlendMode::new(mix, Compose::SrcOver)),
            Some(opacity),
            cut.mask.clone(),
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

/// The compositing mix that draws `blend`.
fn mix_of(blend: Blend) -> Mix {
    match blend {
        Blend::Normal => Mix::Normal,
        Blend::Multiply => Mix::Multiply,
        Blend::Screen => Mix::Screen,
        Blend::Overlay => Mix::Overlay,
        Blend::Darken => Mix::Darken,
        Blend::Lighten => Mix::Lighten,
        Blend::ColorDodge => Mix::ColorDodge,
        Blend::ColorBurn => Mix::ColorBurn,
        Blend::HardLight => Mix::HardLight,
        Blend::SoftLight => Mix::SoftLight,
        Blend::Difference => Mix::Difference,
        Blend::Exclusion => Mix::Exclusion,
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

    /// How far the pixels of a window must reach around it for this blurred shadow to come out as in the whole
    /// picture: the offset, and what the blur reads: a kernel of at most 6 pixels and a filter of 2 at each of its
    /// halvings, either way twice, on the grid of the coarsest of them.
    fn dependence(&self) -> f64 {
        let grid = f64::from(1_u32 << decimations(self.sigma));
        self.dx.abs().max(self.dy.abs()) + (KERNEL_RADIUS + 2.0 * HALVING_FILTER) * grid
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

/// `STRAIGHT[alpha][colour]` is `colour * 255 / alpha` rounded to the nearest whole number (halves up), 0 for alpha 0.
static STRAIGHT: [[u8; 256]; 256] = {
    let mut table = [[0; 256]; 256];
    let mut alpha = 1;
    while alpha < 256 {
        let mut colour = 0;
        while colour <= alpha {
            // a premultiplied colour is at most its alpha, so the quotient is at most 255
            #[expect(clippy::cast_possible_truncation, reason = "at most 255")]
            let straight = ((colour * 255 + alpha / 2) / alpha) as u8;
            table[alpha][colour] = straight;
            colour += 1;
        }
        alpha += 1;
    }
    table
};

/// Turns premultiplied RGBA bytes into straight alpha, in place: each colour byte becomes `colour * 255 / alpha`
/// rounded to the nearest whole number (halves up), and a pixel with alpha 0 becomes all zeros.
fn unpremultiply(bytes: &mut [u8]) {
    let (pixels, _) = bytes.as_chunks_mut::<4>();
    for [red, green, blue, alpha] in pixels {
        let row = &STRAIGHT[usize::from(*alpha)];
        for channel in [red, green, blue] {
            *channel = row[usize::from(*channel)];
        }
    }
}
