//! `Drawing::to_svg`: the drawing as an SVG document, one `<path>` per shape in paint order.

use super::ramp::Ramp;
use super::screen::{Screen, screen};
use super::view::View;
use super::{Size, cast, fills, in_shape};
use crate::limits::{MAX_KNOCKOUT_LAYERS, Work};
use crate::path::number;
use crate::{Blend, Color, Drawing, Error, GradientKind, Paint, Shape, Values};
use std::fmt::Write as _;

/// The black a knockout is painted in inside a mask: what is black there is cut.
const BLACK: Color = Color::hex(0x00_00_00);

/// The largest alpha byte of a colour, which is fully opaque.
// constant: an alpha byte is 0..=255
const OPAQUE: f64 = 255.0;

impl Drawing {
    /// The drawing as an SVG document of `size`, with the Params at the `values` given and at their defaults where
    /// none is given.
    ///
    /// The design frame is scaled to fit `size` and centred (a drawing without a frame is drawn at one pixel to the
    /// design pixel), and content outside the frame is not clipped. The coordinates and stroke widths are written in
    /// output pixels, not as a transform, so that a drawing at twice the size has numbers exactly twice as large; every
    /// number is the shortest text that reads back as the same value. A shape is one `<path>` whose `opacity` applies
    /// to its fill and stroke together, as the pixels composite them; a shape whose path is empty writes no element.
    /// A shape that knocks out writes none either: everything written before it, back to the last shape that is not
    /// cut by the same knockouts, is wrapped in a `<g mask="url(#knockout-N)">`, `N` the number of the first knockout in
    /// a run, and the mask is the canvas (grown by the farthest a shadow in the group reaches) in white with the fill and
    /// stroke of each knockout of the run in black, whatever their colours, opacity and blend. A group nested in a
    /// group keeps multiply between the shapes of both, as the pixels do. At most `MAX_KNOCKOUT_LAYERS` (128) groups
    /// nest: knockouts in a row are one group, and every shape between two makes another.
    /// A halftone is a `<clipPath>` of the shape's outline and a `<g clip-path>` holding all the dots as one `<path>` of
    /// closed polygons (a translucent colour is no darker where dots overlap), then the stroke; a tone of 0 is a plain
    /// fill and a tone of 1 a fill of none. Its opacity and blend are on a `<g>` round both, and a hard shadow is a
    /// second `<g transform="translate(..)">` of the dots in the shadow's colour.
    /// The root has `isolation:isolate`, so that multiply blends with the picture and not with the page it is in.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 100,000;
    /// naming the shape if its coordinates or stroke width would be beyond ±1e100 at this size, or if a knockout would
    /// nest the document more than 128 groups deep, or `pitch` (a halftone's) if its dots would pass `MAX_SEGMENTS`; otherwise what [`Drawing::shapes`] raises.
    pub fn to_svg(&self, size: Size, values: &Values) -> Result<String, Error> {
        let canvas = size.checked()?;
        let (width, height) = canvas;
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), canvas);
        let (width, height) = (number(width), number(height));
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
             viewBox=\"0 0 {width} {height}\" style=\"isolation:isolate\">\n"
        );
        svg.push_str(&elements(&shapes, &view, canvas, &Work::new())?);
        svg.push_str("</svg>\n");
        Ok(svg)
    }
}

/// The elements of `shapes` in paint order, with a mask over everything below each run of knockouts. `work` counts the
/// edges that halftones flatten their shapes into and the dots they make.
fn elements(shapes: &[Shape], view: &View, canvas: (f64, f64), work: &Work) -> Result<String, Error> {
    let mut written = String::new();
    // the farthest a shadow in `written` reaches beyond the canvas
    let mut reach = 0.0_f64;
    let mut layers = 0;
    // the knockouts in a row so far: the number of the first, and the black of each
    let mut run: Option<(usize, String)> = None;
    for shape in shapes {
        if shape.style().knockout() {
            if let Some(black) = knocked(shape, view, work)? {
                run.get_or_insert_with(|| (shape.index(), String::new()))
                    .1
                    .push_str(&black);
            }
            continue;
        }
        let mut one = String::new();
        let farthest = element(&mut one, shape, view, (canvas, work))?;
        if one.is_empty() {
            continue;
        }
        written = cut(written, run.take(), (reach, canvas), &mut layers)?;
        reach = reach.max(farthest);
        written.push_str(&one);
    }
    cut(written, run, (reach, canvas), &mut layers)
}

/// `written` inside a group masked by the `run` of knockouts (the number of the first and the black of each) that comes
/// after it, or as it is if there is none or nothing is written. The mask covers the canvas grown by `reach`.
///
/// # Errors
/// [`Error::Parameter`] naming the first knockout of the run if it would be the 129th group to nest.
fn cut(
    written: String,
    run: Option<(usize, String)>,
    (reach, canvas): (f64, (f64, f64)),
    layers: &mut usize,
) -> Result<String, Error> {
    let Some((first, black)) = run else { return Ok(written) };
    if written.is_empty() {
        return Ok(written);
    }
    *layers += 1;
    if *layers > MAX_KNOCKOUT_LAYERS {
        return Err(Error::parameter(
            &format!("shape {first}"),
            format!("MAX_KNOCKOUT_LAYERS {MAX_KNOCKOUT_LAYERS} passed"),
        ));
    }
    let (origin, width, height) = (
        number(0.0 - reach),
        number(canvas.0 + 2.0 * reach),
        number(canvas.1 + 2.0 * reach),
    );
    let id = format!("knockout-{first}");
    Ok(format!(
        "<mask id=\"{id}\" maskUnits=\"userSpaceOnUse\" x=\"{origin}\" y=\"{origin}\" width=\"{width}\" height=\"{height}\">\n\
         <rect x=\"{origin}\" y=\"{origin}\" width=\"{width}\" height=\"{height}\" fill=\"#ffffff\"/>\n{black}</mask>\n\
         <g mask=\"url(#{id})\">\n{written}</g>\n"
    ))
}

/// The markup of a shape that knocks out as a mask paints it: its fill and stroke in black. `None` if it paints neither or
/// its path is empty, which cut nothing.
fn knocked(shape: &Shape, view: &View, work: &Work) -> Result<Option<String>, Error> {
    let style = shape.style();
    let painted = fills(style) || style.stroke().is_some_and(|stroke| stroke.paint().paints());
    let data = view
        .path(shape.path())
        .map_err(|error| in_shape(shape, "", error))?
        .to_svg_path();
    if !painted || data.is_empty() {
        return Ok(None);
    }
    let fill = Fill::of(shape, view, work)?;
    let mut black = String::new();
    fill.define(&mut black, &data);
    drawn(&mut black, (shape, view), &data, &fill, Ink::Mask, None)?;
    Ok(Some(black))
}

/// Appends the elements of `shape` seen through `view` on a canvas of `canvas` pixels, or nothing if its path is empty,
/// and returns how far its shadow reaches beyond the shape in output pixels (0 where it casts none). `work` is what the
/// draw has made so far.
///
/// With a shadow the opacity and the blend belong to a `<g>` around the shadow and the shape, so that they apply once
/// to both. A blurred shadow is that group's `filter`: the blur, offset and tint of the shape's alpha, merged under the
/// shape. A shadow with no blur is a plain copy of the path before the shape, tinted. A halftone is a group of its own
/// (the dots, clipped, and the stroke), which takes the opacity and the blend when there is no shadow's group to.
fn element(svg: &mut String, shape: &Shape, view: &View, (canvas, work): ((f64, f64), &Work)) -> Result<f64, Error> {
    let in_shape = |what: &'static str| move |error| in_shape(shape, what, error);
    let device = view.path(shape.path()).map_err(in_shape(""))?;
    let data = device.to_svg_path();
    if data.is_empty() {
        return Ok(0.0);
    }
    let style = shape.style();
    let fill = Fill::of(shape, view, work)?;
    let group = |svg: &mut String| {
        if style.opacity() < 1.0 {
            let _ = write!(svg, " opacity=\"{}\"", number(style.opacity()));
        }
        if style.blend() == Blend::Multiply {
            svg.push_str(" style=\"mix-blend-mode:multiply\"");
        }
    };
    let grouped = style.opacity() < 1.0 || style.blend() == Blend::Multiply;
    let Some((shadow, tint)) = cast(style) else {
        fill.define(svg, &data);
        define_gradients(svg, shape, view, (Ink::Own, (0.0, 0.0)))?;
        if let Fill::Plain(_) = fill {
            // one path takes the group's attributes itself
            svg.push_str("<path");
            body(svg, &data, (shape, view), Ink::Own, fill.plain())?;
            group(svg);
            svg.push_str("/>\n");
        } else {
            if grouped {
                svg.push_str("<g");
                group(svg);
                svg.push_str(">\n");
            }
            drawn(svg, (shape, view), &data, &fill, Ink::Own, None)?;
            if grouped {
                svg.push_str("</g>\n");
            }
        }
        return Ok(0.0);
    };
    let (dx, dy) = (
        view.length(shadow.dx()).map_err(in_shape("shadow dx "))?,
        view.length(shadow.dy()).map_err(in_shape("shadow dy "))?,
    );
    fill.define(svg, &data);
    define_gradients(svg, shape, view, (Ink::Own, (0.0, 0.0)))?;
    if shadow.blur() > 0.0 {
        let sigma = view.length(shadow.blur() / 2.0).map_err(in_shape("shadow blur "))?;
        // the region the filter works in: the canvas, and the farthest the shadow reaches (3 deviations and the offset)
        let reach = 3.0 * sigma + dx.abs().max(dy.abs());
        let (x, y, width, height) = (-reach, -reach, canvas.0 + 2.0 * reach, canvas.1 + 2.0 * reach);
        let id = format!("shadow-{}", shape.index());
        let _ = writeln!(
            svg,
            "<filter id=\"{id}\" filterUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
             color-interpolation-filters=\"sRGB\">",
            number(x),
            number(y),
            number(width),
            number(height),
        );
        let _ = write!(
            svg,
            "<feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{}\"/>\
             <feOffset dx=\"{}\" dy=\"{}\" result=\"b\"/>",
            number(sigma),
            number(dx),
            number(dy),
        );
        let [red, green, blue, alpha] = tint;
        let _ = write!(svg, "<feFlood flood-color=\"#{red:02x}{green:02x}{blue:02x}\"");
        if alpha < u8::MAX {
            let _ = write!(svg, " flood-opacity=\"{}\"", number(f64::from(alpha) / OPAQUE));
        }
        svg.push_str("/>");
        svg.push_str(
            "<feComposite in2=\"b\" operator=\"in\" result=\"s\"/>\
             <feMerge><feMergeNode in=\"s\"/><feMergeNode in=\"SourceGraphic\"/></feMerge>\n</filter>\n",
        );
        let _ = write!(svg, "<g filter=\"url(#{id})\"");
        group(svg);
        svg.push_str(">\n");
        drawn(svg, (shape, view), &data, &fill, Ink::Own, None)?;
        svg.push_str("</g>\n");
        return Ok(reach);
    }
    define_gradients(svg, shape, view, (Ink::Tint(tint), (dx, dy)))?;
    if grouped {
        svg.push_str("<g");
        group(svg);
        svg.push_str(">\n");
    }
    if let Fill::Plain(_) = fill {
        let copy = device.translate(dx, dy).map_err(in_shape("shadow "))?.to_svg_path();
        drawn(svg, (shape, view), &copy, &fill, Ink::Tint(tint), None)?;
    } else {
        drawn(svg, (shape, view), &data, &fill, Ink::Tint(tint), Some((dx, dy)))?;
    }
    drawn(svg, (shape, view), &data, &fill, Ink::Own, None)?;
    if grouped {
        svg.push_str("</g>\n");
    }
    Ok(dx.abs().max(dy.abs()))
}

/// The paint of a path that has no fill.
static NO_PAINT: Paint = Paint::NONE;

/// Appends the gradients that paint `shape` as `ink` says, moved by `shift` output pixels: its own, or for a shadow's
/// copy the same in the colour of the tint with their own alphas, so that the copy follows the paint's alpha.
fn define_gradients(
    svg: &mut String,
    shape: &Shape,
    view: &View,
    (ink, shift): (Ink, (f64, f64)),
) -> Result<(), Error> {
    let style = shape.style();
    let paints = [
        ("fill", Some(style.fill())),
        ("stroke", style.stroke().map(crate::StrokeStyle::paint)),
    ];
    for (slot, paint) in paints {
        let Some(Paint::Gradient(gradient)) = paint else {
            continue;
        };
        let mut ramp = Ramp::new(gradient, view).map_err(|error| in_shape(shape, "gradient ", error))?;
        if let Ink::Tint([red, green, blue, _]) = ink {
            ramp = ramp.tinted([red, green, blue]).shifted(shift.0, shift.1);
        }
        let id = gradient_id(shape.index(), slot, ink);
        match ramp.kind {
            GradientKind::Linear { start, end } => {
                let _ = writeln!(
                    svg,
                    "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">",
                    number(start.x),
                    number(start.y),
                    number(end.x),
                    number(end.y)
                );
            }
            GradientKind::Radial { center, radius } => {
                let _ = writeln!(
                    svg,
                    "<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"{}\" cy=\"{}\" r=\"{}\">",
                    number(center.x),
                    number(center.y),
                    number(radius)
                );
            }
        }
        for (offset, [red, green, blue, alpha]) in &ramp.stops {
            let _ = write!(
                svg,
                "<stop offset=\"{}\" stop-color=\"#{red:02x}{green:02x}{blue:02x}\"",
                number(*offset)
            );
            if *alpha < u8::MAX {
                let _ = write!(svg, " stop-opacity=\"{}\"", number(f64::from(*alpha) / OPAQUE));
            }
            svg.push_str("/>\n");
        }
        svg.push_str(match ramp.kind {
            GradientKind::Linear { .. } => "</linearGradient>\n",
            GradientKind::Radial { .. } => "</radialGradient>\n",
        });
    }
    Ok(())
}

/// How the fill of a shape is painted.
enum Fill {
    /// One paint all over the shape, [`Paint::NONE`] for none.
    Plain(Paint),
    /// A screen of dots: the colour, the id of the clip path that is the shape's outline, and the dots as path data.
    Dots { color: Color, id: String, dots: String },
}

impl Fill {
    /// The fill of `shape` seen through `view`: a halftone of the tone 0 is a plain fill of its colour, and one with no
    /// dots a fill of none. `work` is charged the edges the shape is flattened into for the dots, and the dots.
    fn of(shape: &Shape, view: &View, work: &Work) -> Result<Self, Error> {
        let style = shape.style();
        let Some(halftone) = style.halftone() else {
            return Ok(Self::Plain(style.fill().clone()));
        };
        let name = format!("shape {}", shape.index());
        let outline = shape.path().flattened(&mut |count| work.made_segments(count, &name))?;
        let dots = match screen(halftone, &outline, &mut |count| {
            work.made_segments(count, &format!("{name} pitch"))
        })? {
            Screen::Solid => return Ok(Self::Plain(Paint::Color(halftone.color()))),
            Screen::Dots(dots) if dots.is_empty() => return Ok(Self::Plain(Paint::NONE)),
            Screen::Dots(dots) => dots,
        };
        let polygons: Vec<String> = dots
            .all()
            .map(|dot| {
                let mut corners: Vec<String> = dot
                    .iter()
                    .enumerate()
                    .map(|(at, corner)| {
                        let placed = view.point(*corner);
                        format!(
                            "{}{} {}",
                            if at == 0 { "M" } else { "L" },
                            number(placed.x),
                            number(placed.y)
                        )
                    })
                    .collect();
                corners.push("Z".to_owned());
                corners.join(" ")
            })
            .collect();
        Ok(Self::Dots {
            color: halftone.color(),
            id: format!("halftone-{}", shape.index()),
            dots: polygons.join(" "),
        })
    }

    /// The paint of a plain fill, none for a screen of dots, whose path has no fill.
    fn plain(&self) -> &Paint {
        match self {
            Self::Plain(paint) => paint,
            Self::Dots { .. } => &NO_PAINT,
        }
    }

    /// Appends the clip path of a screen of dots: the outline `data` of the shape. A plain fill has none.
    fn define(&self, svg: &mut String, data: &str) {
        if let Self::Dots { id, .. } = self {
            let _ = writeln!(svg, "<clipPath id=\"{id}\"><path d=\"{data}\"/></clipPath>");
        }
    }
}

/// How the fill and stroke of a path are coloured.
#[derive(Clone, Copy)]
enum Ink {
    /// As the style has them.
    Own,
    /// The silhouette in the colour of the tint and the alphas of the style, under an `opacity` of the tint's alpha.
    Tint([u8; 4]),
    /// Opaque black wherever the style paints, as a mask cuts.
    Mask,
}

/// What an attribute is painted with.
enum Coat {
    /// A colour, or none.
    Flat(Color),
    /// The gradient with this id.
    Gradient(String),
}

/// The id of the gradient that paints `slot` (`fill` or `stroke`) of the shape numbered `index`: one id for every
/// gradient the document has, and the only place one is made. A shadow's copy has a gradient of its own, in the tint.
fn gradient_id(index: usize, slot: &str, ink: Ink) -> String {
    match ink {
        Ink::Tint(_) => format!("gradient-{index}-shadow-{slot}"),
        Ink::Own | Ink::Mask => format!("gradient-{index}-{slot}"),
    }
}

impl Ink {
    /// `paint`, which fills or strokes (`slot`) the shape numbered `index`, as this ink paints it: a colour as
    /// [`Ink::on`] has it, a gradient by the id `gradient_id` gives, or black where the ink is a mask's.
    fn coat(self, paint: &Paint, slot: &str, index: usize) -> Coat {
        match (paint, self) {
            (Paint::Color(color), _) => Coat::Flat(self.on(*color)),
            (Paint::Gradient(_), Self::Mask) => Coat::Flat(BLACK),
            (Paint::Gradient(_), _) => Coat::Gradient(gradient_id(index, slot, self)),
        }
    }

    /// `color` as this ink paints it.
    fn on(self, color: Color) -> Color {
        match (self, color.rgba()) {
            (Self::Tint([red, green, blue, _]), Some([_, _, _, alpha])) => Color::from_bytes([red, green, blue, alpha]),
            (Self::Mask, Some(_)) => BLACK,
            _ => color,
        }
    }

    /// The alpha of the tint when it is below 1: what the whole silhouette is painted under.
    fn dimmed(self) -> Option<u8> {
        match self {
            Self::Tint([_, _, _, alpha]) if alpha < u8::MAX => Some(alpha),
            _ => None,
        }
    }
}

/// Appends what paints `shape` as `ink` says with `fill`: a path, or for a screen of dots the dots in a group clipped to
/// the clip path `Fill::define` wrote and then the stroke, if any. `data` is the outline for the stroke, and for a
/// plain fill the path. A screen is moved by `moved` (the hard shadow's offset), by a transform on its group.
fn drawn(
    svg: &mut String,
    (shape, view): (&Shape, &View),
    data: &str,
    fill: &Fill,
    ink: Ink,
    moved: Option<(f64, f64)>,
) -> Result<(), Error> {
    let dimmed = |svg: &mut String| {
        if let Some(alpha) = ink.dimmed() {
            let _ = write!(svg, " opacity=\"{}\"", number(f64::from(alpha) / OPAQUE));
        }
    };
    let Fill::Dots { color, id, dots } = fill else {
        svg.push_str("<path");
        body(svg, data, (shape, view), ink, fill.plain())?;
        dimmed(svg);
        svg.push_str("/>\n");
        return Ok(());
    };
    let wrapped = moved.is_some() || ink.dimmed().is_some();
    if wrapped {
        svg.push_str("<g");
        if let Some((dx, dy)) = moved {
            let _ = write!(svg, " transform=\"translate({} {})\"", number(dx), number(dy));
        }
        dimmed(svg);
        svg.push_str(">\n");
    }
    let _ = write!(svg, "<g clip-path=\"url(#{id})\"><path d=\"{dots}\"");
    attribute(svg, "fill", &Coat::Flat(ink.on(*color)));
    svg.push_str("/></g>\n");
    if shape.style().stroke().is_some() {
        svg.push_str("<path");
        body(svg, data, (shape, view), ink, &NO_PAINT)?;
        svg.push_str("/>\n");
    }
    if wrapped {
        svg.push_str("</g>\n");
    }
    Ok(())
}

/// Appends the attributes of the path `data` that paint it: the `fill` given and the stroke of the style, coloured as
/// `ink` says.
fn body(svg: &mut String, data: &str, (shape, view): (&Shape, &View), ink: Ink, fill: &Paint) -> Result<(), Error> {
    let style = shape.style();
    let _ = write!(svg, " d=\"{data}\"");
    attribute(svg, "fill", &ink.coat(fill, "fill", shape.index()));
    if let Some(stroke) = style.stroke() {
        attribute(svg, "stroke", &ink.coat(stroke.paint(), "stroke", shape.index()));
        let _ = write!(
            svg,
            " stroke-width=\"{}\" stroke-linejoin=\"{}\" stroke-linecap=\"{}\" stroke-miterlimit=\"{}\"",
            number(
                view.length(stroke.width())
                    .map_err(|error| in_shape(shape, "stroke width ", error))?
            ),
            stroke.join().name(),
            stroke.cap().name(),
            number(stroke.miter_limit()),
        );
    }
    Ok(())
}

/// Appends the attribute `name` (`fill` or `stroke`) for `coat`, and `<name>-opacity` where a colour's alpha is below 255.
fn attribute(svg: &mut String, name: &str, coat: &Coat) {
    let color = match coat {
        Coat::Flat(color) => color,
        Coat::Gradient(id) => {
            let _ = write!(svg, " {name}=\"url(#{id})\"");
            return;
        }
    };
    let Some([red, green, blue, alpha]) = color.rgba() else {
        let _ = write!(svg, " {name}=\"none\"");
        return;
    };
    let _ = write!(svg, " {name}=\"#{red:02x}{green:02x}{blue:02x}\"");
    if alpha < u8::MAX {
        let _ = write!(svg, " {name}-opacity=\"{}\"", number(f64::from(alpha) / OPAQUE));
    }
}
