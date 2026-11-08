//! `Drawing::to_mesh`: the drawing as triangles. The only module that uses the tessellator crate (`lyon_tessellation`);
//! nothing of it reaches the API.
//!
//! Every shape is two layers at most, its fill and then its stroke. A layer is a set of closed outlines in design
//! pixels (the flattened path; or the outline of the stroke, which `kurbo` expands as it does for the pixels) filled by
//! the nonzero rule, so that a hole, an overlap and a stroke that crosses itself are each covered by one triangle at a
//! point. The tessellator computes in f32, so it works in design pixels, where the numbers are the drawing's own, and
//! the vertices are moved through the view afterwards: a drawing at twice the size has positions exactly twice as
//! large.
//!
//! The fill of a halftone is a layer of its dots: those wholly inside the shape are plain polygons, those wholly outside
//! are left out, and the dots that cross the outline are cut to the shape by one boolean (`Path::intersect`).
//!
//! A shape that knocks out is not drawn: the region its fill and stroke cover is taken out of the layers of every shape
//! before it (`Path::subtract`, so the cut edge of a stroke is flat), their hard shadows included.

use super::cast;
use super::outline::{bez_path, dots, stroke_outline, within_device};
use super::screen::{Screen, screen};
use super::view::View;
use super::{Size, in_shape};
use crate::limits::Work;
use crate::math::{atan2, sin_cos};
use crate::path::{Builder, Combine, FLATNESS, Polyline};
use crate::{Blend, Cap, Drawing, Error, Join, Path, Point, Shape, Values};
use kurbo::{BezPath, PathEl};
use lyon_tessellation::math::point;
use lyon_tessellation::path::Path as Outlines;
use lyon_tessellation::{
    FillGeometryBuilder, FillOptions, FillRule, FillTessellator, FillVertex, GeometryBuilder, GeometryBuilderError,
    VertexId,
};

/// The triangles of a drawing, in three buffers that a display uploads as they are. [`Drawing::to_mesh`] fills one.
///
/// `vertices` holds 8 `f32` a vertex: `x` and `y` in output pixels (`y` down), the colour `r, g, b, a` premultiplied,
/// the `item` (the shape's index) and the `order` (the shape's `order` attribute, 0 where it has none). `indices`
/// holds three `u32` a triangle, in paint order. `ranges` holds three `u32` a draw range: the first index, the number
/// of indices, and the blend (0 normal, 1 multiply); the ranges follow one another and tile the indices.
///
/// A mesh is a display format: the pixels and the SVG are the reference.
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    current: Buffers,
    scratch: Buffers,
    generation: u64,
}

/// The tolerance the tessellator is given, in design pixels.
// constant: the outlines are already straight lines, flattened to FLATNESS, so the tessellator flattens nothing and the
// tolerance only decides when it takes two points to be one; its default 0.1 left 2 of 20,000 random strokes with
// corners on a grid covered more than once, and 0.01, 0.001 and 0.0001 each left 1, so it is the flatness itself
const FILL_TOLERANCE: f32 = 0.01;

/// The floats a vertex holds.
const VERTEX_FLOATS: usize = 8;

/// The three buffers of a mesh.
#[derive(Debug, Clone, Default, PartialEq)]
struct Buffers {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    ranges: Vec<u32>,
}

impl Mesh {
    /// An empty mesh, generation 0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The vertices: 8 floats each, `x, y, r, g, b, a, item, order`.
    #[must_use]
    pub fn vertices(&self) -> &[f32] {
        &self.current.vertices
    }

    /// The triangles: three indices into the vertices each, in paint order.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.current.indices
    }

    /// The draw ranges: three numbers each, the first index, the number of indices and the blend (0 normal, 1
    /// multiply).
    #[must_use]
    pub fn ranges(&self) -> &[u32] {
        &self.current.ranges
    }

    /// A number that changes exactly when the contents change: a mesh is generation 0 until a fill gives it contents
    /// that differ from what it held, and every such fill adds 1. A fill that makes the same buffers again leaves it,
    /// so that a display uploads only when the number moved.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl Drawing {
    /// Fills `into` with the triangles of the drawing at `size`, with the Params at the `values` given and at their
    /// defaults where none is given. `into` is left as it was if this returns an error.
    ///
    /// Shapes are placed as in [`Drawing::to_svg`] and painted in order. A shape is up to two layers: its fill, then
    /// its stroke, each a set of triangles that cover every point inside it once (by the nonzero rule, as the other
    /// outputs fill), so a translucent fill and stroke show darker where they overlap, which the pixels' one group
    /// per shape does not. A colour's alpha times the shape's `opacity` is the vertex alpha; a layer whose alpha is 0
    /// has no triangles. A shape with an empty path adds nothing but keeps its number as `item`. A shape that knocks out
    /// adds nothing either: the region its fill and stroke cover is taken out of the layers of the shapes before it
    /// (their blur-free shadows too) by a boolean, so what is left of a cut stroke ends flat at the cut, where the pixels
    /// end it the same way; the edges and the crossings of the cut are charged to `MAX_SEGMENTS` as a boolean step of a
    /// drawing is. The dots of a halftone are a fill layer: the dots wholly inside are polygons of their own, and the ones
    /// that cross the outline are cut to the shape by one boolean, charged the same way (tone 0 is a solid fill). Curves are flattened,
    /// as for the pixels, to within 0.01 design pixels.
    ///
    /// The triangles are made in design pixels in f32, so a coordinate beyond 10 million is refused.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 100,000;
    /// naming the shape if a coordinate or the stroke width of one lies beyond 10 million pixels, in design pixels or
    /// at this size, or if the edges flattened for it, the edges of its stroke's outline or the vertices made for the
    /// draw pass 10 million, or if the tessellation of the draw would take more than about a second (the vertices it
    /// makes times the edges alive at a sweep line pass 250 million: an outline that crosses itself thousands of times,
    /// or has thousands of edges side by side), or `pitch` (a halftone's) if its dots pass 10 million; otherwise as [`Drawing::to_svg`]. [`Error::Geometry`] if the tessellator fails, which a
    /// finite drawing within these limits does not make it do.
    pub fn to_mesh(&self, size: Size, values: &Values, into: &mut Mesh) -> Result<(), Error> {
        let (width, height) = size.checked()?;
        let shapes = self.shapes(values)?;
        let view = View::fit(self.size(), (width, height));
        let mut maker = Maker {
            buffers: &mut into.scratch,
            tessellator: FillTessellator::new(),
            work: Work::new(),
        };
        maker.buffers.vertices.clear();
        maker.buffers.indices.clear();
        maker.buffers.ranges.clear();
        let knockouts = maker.knockouts(&shapes, &view)?;
        // the box that holds the knockouts from each one on, so that a shape finds what is cut from it without a pass over them
        let mut reach = vec![Bounds::EMPTY; knockouts.len() + 1];
        for (at, (_, region)) in knockouts.iter().enumerate().rev() {
            reach[at] = reach[at + 1].union(&region.bounds);
        }
        for (position, shape) in shapes.iter().enumerate() {
            let first = knockouts.partition_point(|(at, _)| *at <= position);
            let knocked = (first < knockouts.len()).then(|| Knocked {
                regions: &knockouts[first..],
                bounds: reach[first],
            });
            maker.shape(shape, &view, knocked.as_ref())?;
        }
        if into.scratch != into.current {
            std::mem::swap(&mut into.current, &mut into.scratch);
            into.generation += 1;
        }
        Ok(())
    }
}

/// What the layers of a draw are made into.
struct Maker<'a> {
    buffers: &'a mut Buffers,
    tessellator: FillTessellator,
    /// The edges flattened and the vertices made so far in the draw, against `MAX_SEGMENTS`.
    work: Work,
}

/// Where a layer goes: its colour, its blend, and the item and order its vertices carry.
struct Paint {
    rgba: [f32; 4],
    blend: Blend,
    item: f32,
    order: f32,
}

/// A closed outline of a layer: its corners in design pixels.
type Ring = Vec<[f32; 2]>;

/// The outlines of a layer and the colour it is painted in.
type Layer = (Vec<Ring>, [u8; 4]);

impl Maker<'_> {
    /// The regions that the shapes which knock out take away, each with its position in `shapes`.
    ///
    /// # Errors
    /// As [`Maker::shape`] for the shape that knocks out, and what resolving a region raises.
    fn knockouts(&self, shapes: &[Shape], view: &View) -> Result<Vec<(usize, Region)>, Error> {
        let mut found = Vec::new();
        for (position, shape) in shapes.iter().enumerate().filter(|(_, shape)| shape.style().knockout()) {
            let name = format!("shape {}", shape.index());
            let mut charge = |count| self.work.made_segments(count, &name);
            let polylines = shape.path().flattened(&mut charge)?;
            if polylines.is_empty() {
                continue;
            }
            reachable(polylines.iter().flat_map(|polyline| &polyline.points), view)
                .map_err(|error| in_shape(shape, "", error))?;
            let layers = self.layers(shape, &polylines, view, &name, true)?;
            // each layer on its own is resolved (the fill and the stroke may run opposite ways), and then they are joined
            let mut parts = Vec::new();
            let mut bounds = Bounds::EMPTY;
            for (rings, _) in &layers {
                bounds = bounds.with(rings);
                parts.push(path_of(rings)?.resolved(FLATNESS, &mut charge)?);
            }
            if !parts.is_empty() {
                found.push((
                    position,
                    Region {
                        path: Path::joined(&parts),
                        bounds,
                    },
                ));
            }
        }
        Ok(found)
    }

    /// The layers of `shape`, its fill and then its stroke, as closed outlines in design pixels with the colour each is
    /// painted in. `polylines` is its path flattened. A layer that is not painted is left out, and so is one with no
    /// alpha (a colour with none, or an opacity of 0) unless `covering` asks for the area the shape covers.
    fn layers(
        &self,
        shape: &Shape,
        polylines: &[Polyline],
        view: &View,
        name: &str,
        covering: bool,
    ) -> Result<Vec<Layer>, Error> {
        let style = shape.style();
        let visible = |rgba: [u8; 4]| covering || style.opacity() * f64::from(rgba[3]) > 0.0;
        let mut layers: Vec<Layer> = Vec::new();
        layers.extend(self.fill_layer(shape, polylines, view, name, visible)?);
        if let Some(stroke) = style.stroke()
            && let Some(rgba) = stroke.color().rgba()
        {
            let device = view
                .length(stroke.width())
                .map_err(|error| in_shape(shape, "stroke width ", error))?;
            within_device("the stroke width", device).map_err(|error| in_shape(shape, "", error))?;
            if visible(rgba) {
                // kurbo's round joins and ends are arcs on a self-overlapping outline, which the tessellator sometimes covers
                // three times: the outline is made with bevel joins and butt ends, and discs added where the arcs were
                let (join, cap) = match (stroke.join(), stroke.cap()) {
                    (Join::Round, cap) => (Join::Bevel, cap),
                    other => other,
                };
                let cap = if cap == Cap::Round { Cap::Butt } else { cap };
                let outline = stroke_outline(
                    &bez_path(polylines),
                    stroke,
                    (join, cap),
                    stroke.width(),
                    FLATNESS,
                    name,
                )?;
                let mut rings = rings_of(&outline);
                let mut charge = |count| self.work.made_segments(count, name);
                let round = (stroke.join() == Join::Round, stroke.cap() == Cap::Round);
                for wedge in round_wedges(polylines, round, stroke.width() / 2.0) {
                    charge(wedge.len())?;
                    rings.push(wedge);
                }
                for dot in dots(polylines, stroke.cap(), stroke.width(), &mut charge)? {
                    rings.extend(dot.iter().map(|polyline| ring(&polyline.points)));
                }
                layers.push((rings, rgba));
            }
        }
        Ok(layers)
    }

    /// The layer of the fill of `shape`, whose path flattened is `polylines`: its outline in the fill colour, or for a
    /// halftone the dots. The dots wholly inside are plain polygons, those outside are left out, and only those that cross
    /// the outline are cut to the shape, all together by one boolean (charged to `MAX_SEGMENTS` as a boolean step of a
    /// drawing is). `None` if the fill paints nothing or `visible` says its colour does not.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming the shape's `pitch` if its dots pass `MAX_SEGMENTS`, or if a dot lies beyond the
    /// tessellator's range.
    fn fill_layer(
        &self,
        shape: &Shape,
        polylines: &[Polyline],
        view: &View,
        name: &str,
        visible: impl Fn([u8; 4]) -> bool,
    ) -> Result<Option<Layer>, Error> {
        let style = shape.style();
        let whole = || polylines.iter().map(|polyline| ring(&polyline.points)).collect();
        let Some(halftone) = style.halftone() else {
            return Ok(style
                .fill()
                .rgba()
                .filter(|rgba| visible(*rgba))
                .map(|rgba| (whole(), rgba)));
        };
        let Some(rgba) = halftone.color().rgba().filter(|rgba| visible(*rgba)) else {
            return Ok(None);
        };
        let found = match screen(halftone, polylines, &mut |count| {
            self.work.made_segments(count, &format!("{name} pitch"))
        })? {
            Screen::Solid => return Ok(Some((whole(), rgba))),
            Screen::Dots(found) if found.is_empty() => return Ok(None),
            Screen::Dots(found) => found,
        };
        reachable(found.all().flatten(), view).map_err(|error| in_shape(shape, "", error))?;
        let mut rings: Vec<Ring> = found.inside.iter().map(|dot| ring(dot)).collect();
        if !found.crossing.is_empty() {
            let crossing = path_of(&found.crossing.iter().map(|dot| ring(dot)).collect::<Vec<_>>())?;
            let mut charge = |count| self.work.made_segments(count, name);
            let cut = shape
                .path()
                .combined(&crossing, Combine::Intersect, FLATNESS, &mut charge)?;
            rings.extend(
                cut.flattened(&mut charge)?
                    .iter()
                    .map(|polyline| ring(&polyline.points)),
            );
        }
        Ok(Some((rings, rgba)))
    }

    /// `rings` with `knocked` taken out of them, charged to `MAX_SEGMENTS` as a boolean step of a drawing is. The
    /// regions are joined only here, where the boolean they go into is charged for their edges.
    fn cut(&self, rings: Vec<Ring>, knocked: &Knocked, name: &str) -> Result<Vec<Ring>, Error> {
        if !Bounds::EMPTY.with(&rings).meets(&knocked.bounds) {
            return Ok(rings);
        }
        let mut charge = |count| self.work.made_segments(count, name);
        let region = Path::joined(knocked.regions.iter().map(|(_, region)| &region.path));
        let left = path_of(&rings)?.combined(&region, Combine::Subtract, FLATNESS, &mut charge)?;
        Ok(left
            .flattened(&mut charge)?
            .iter()
            .map(|polyline| ring(&polyline.points))
            .collect())
    }

    /// Adds the layers of `shape` seen through `view`, with the `knocked` region taken out of every one, if there is
    /// one. A shape that knocks out has none.
    fn shape(&mut self, shape: &Shape, view: &View, knocked: Option<&Knocked>) -> Result<(), Error> {
        let style = shape.style();
        if style.knockout() {
            return Ok(());
        }
        let name = format!("shape {}", shape.index());
        let work = &self.work;
        let mut charge = |count| work.made_segments(count, &name);
        let polylines = shape.path().flattened(&mut charge)?;
        if polylines.is_empty() {
            return Ok(());
        }
        reachable(polylines.iter().flat_map(|polyline| &polyline.points), view)
            .map_err(|error| in_shape(shape, "", error))?;
        // a shape no generator ranked has order 0
        let order = shape.attributes().order().unwrap_or(0.0);
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            reason = "a list of at most a million shapes is exact in an f32, and an order from 0 to 1 loses nothing visible"
        )]
        let (item, order) = (shape.index() as f32, order as f32);
        let paint = |rgba: [u8; 4], tint: Option<[u8; 4]>| {
            // a shadow is the colour of its tint, as strong as the tint and the layer it is the silhouette of are
            let (rgba, strength) = match tint {
                Some(tint) => (tint, f64::from(tint[3]) / 255.0 * f64::from(rgba[3]) / 255.0),
                None => (rgba, f64::from(rgba[3]) / 255.0),
            };
            Paint {
                rgba: premultiplied(rgba, strength * style.opacity()),
                blend: style.blend(),
                item,
                order,
            }
        };
        let layers = self.layers(shape, &polylines, view, &name, false)?;
        if let Some((shadow, tint)) = cast(style) {
            if shadow.blur() > 0.0 {
                return Err(Error::parameter(
                    "shadow",
                    format!(
                        "shape {}: blur {} has no triangle form, only a blur of 0 does; use to_rgba or to_svg",
                        shape.index(),
                        shadow.blur()
                    ),
                ));
            }
            let (dx, dy) = (shadow.dx(), shadow.dy());
            for (axis, offset) in [("dx", dx), ("dy", dy)] {
                within_device(&format!("the shadow {axis}"), offset.abs())
                    .and_then(|()| within_device(&format!("the shadow {axis}"), view.length(offset)?.abs()))
                    .map_err(|error| in_shape(shape, "", error))?;
            }
            #[expect(clippy::cast_possible_truncation, reason = "the tessellator computes in f32")]
            let (dx, dy) = (dx as f32, dy as f32);
            for (rings, rgba) in layers.iter().filter(|_| tint[3] > 0) {
                let shifted: Vec<Ring> = rings
                    .iter()
                    .map(|ring| ring.iter().map(|[x, y]| [x + dx, y + dy]).collect())
                    .collect();
                let shifted = self.knocked(shifted, knocked, &name)?;
                self.layer(&shifted, &paint(*rgba, Some(tint)), view, &name)?;
            }
        }
        for (rings, rgba) in layers {
            let rings = self.knocked(rings, knocked, &name)?;
            self.layer(&rings, &paint(rgba, None), view, &name)?;
        }
        Ok(())
    }

    /// `rings` with `knocked` taken out of them, as they are if there is none.
    fn knocked(&self, rings: Vec<Ring>, knocked: Option<&Knocked>, name: &str) -> Result<Vec<Ring>, Error> {
        match knocked {
            Some(knocked) => self.cut(rings, knocked, name),
            None => Ok(rings),
        }
    }

    /// Tessellates `rings` by the nonzero rule into triangles of `paint`, seen through `view`, and adds them to the
    /// buffers and to the draw ranges.
    fn layer(&mut self, rings: &[Ring], paint: &Paint, view: &View, name: &str) -> Result<(), Error> {
        let mut outlines = Outlines::builder();
        for ring in rings.iter().filter(|ring| ring.len() >= 3) {
            let mut corners = ring.iter().map(|[x, y]| point(*x, *y));
            if let Some(first) = corners.next() {
                outlines.begin(first);
            }
            for corner in corners {
                outlines.line_to(corner);
            }
            outlines.end(true);
        }
        let first_index = self.buffers.indices.len();
        let mut sink = Sink {
            sweep: crossing_at_most(rings),
            buffers: self.buffers,
            view,
            paint,
            first_vertex: 0,
            work: &self.work,
            name,
            refused: None,
        };
        let options = FillOptions::default()
            .with_fill_rule(FillRule::NonZero)
            .with_tolerance(FILL_TOLERANCE);
        let outcome = self.tessellator.tessellate_path(&outlines.build(), &options, &mut sink);
        let refused = sink.refused.take();
        if let Err(failure) = outcome {
            return Err(refused.unwrap_or_else(|| Error::geometry("to_mesh", format!("{name}: {failure:?}"))));
        }
        let indices = self.buffers.indices.len();
        if indices > first_index {
            let blend = u32::from(paint.blend == Blend::Multiply);
            let (first, count) = (as_index(first_index), as_index(indices - first_index));
            match self.buffers.ranges.last_chunk_mut::<3>() {
                Some([_, length, last]) if *last == blend => *length += count,
                _ => self.buffers.ranges.extend([first, count, blend]),
            }
        }
        Ok(())
    }
}

/// What a knockout takes away: the region it covers, and a box that holds it.
struct Region {
    path: Path,
    bounds: Bounds,
}

/// The knockouts after a shape, which take their regions out of it together, and a box that holds them all.
struct Knocked<'a> {
    regions: &'a [(usize, Region)],
    bounds: Bounds,
}

/// A box in design pixels: left, top, right and bottom.
#[derive(Clone, Copy)]
struct Bounds([f32; 4]);

impl Bounds {
    /// The box that holds nothing: every box is wider than it.
    const EMPTY: Self = Self([f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY]);

    /// This box grown to hold every corner of `rings`.
    fn with(self, rings: &[Ring]) -> Self {
        let mut found = self.0;
        for [x, y] in rings.iter().flatten() {
            found = [found[0].min(*x), found[1].min(*y), found[2].max(*x), found[3].max(*y)];
        }
        Self(found)
    }

    /// The box that holds this one and `other`.
    fn union(self, other: &Self) -> Self {
        let (a, b) = (self.0, other.0);
        Self([a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
    }

    /// Whether the two boxes share a point.
    fn meets(&self, other: &Self) -> bool {
        let (a, b) = (self.0, other.0);
        a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
    }
}

/// The path that has `rings` for sub-paths, each closed.
fn path_of(rings: &[Ring]) -> Result<Path, Error> {
    let mut builder = Builder::new();
    for ring in rings.iter().filter(|ring| ring.len() >= 3) {
        let mut corners = ring.iter().map(|[x, y]| Point::new(f64::from(*x), f64::from(*y)));
        if let Some(first) = corners.next() {
            builder.move_to(first)?;
        }
        for corner in corners {
            builder.line_to(corner)?;
        }
        builder.close();
    }
    Ok(builder.finish())
}

/// An index or a count as the `u32` of the buffers.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the vertices of a draw are at most MAX_SEGMENTS, 10 million, and a triangle's three indices are a few times that: far below 2^32"
)]
fn as_index(count: usize) -> u32 {
    count as u32
}

/// The most edges of `rings` that a horizontal line crosses at once, which is how many edges the tessellator's sweep
/// keeps alive: every vertex it makes costs time in proportion.
fn crossing_at_most(rings: &[Ring]) -> usize {
    // (y, whether an edge starts there); at one y the ends come first, so that edges meeting there count once, and an
    // edge along a sweep line is alive at no other
    let mut events: Vec<(f32, bool)> = Vec::new();
    for ring in rings.iter().filter(|ring| ring.len() >= 3) {
        for (from, to) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            let (top, bottom) = (from[1].min(to[1]), from[1].max(to[1]));
            if top < bottom {
                events.push((top, true));
                events.push((bottom, false));
            }
        }
    }
    events.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
    let (mut alive, mut most) = (0_usize, 0_usize);
    for (_, starts) in events {
        if starts {
            alive += 1;
            most = most.max(alive);
        } else {
            alive -= 1;
        }
    }
    most
}

/// The vertex of the tessellator's output as the buffers hold it.
struct Sink<'a> {
    buffers: &'a mut Buffers,
    view: &'a View,
    paint: &'a Paint,
    /// The number of the layer's first vertex in the buffers: the tessellator numbers its own from 0.
    first_vertex: u32,
    work: &'a Work,
    name: &'a str,
    /// The most edges of the layer that a sweep line crosses at once: what each vertex costs the tessellator.
    sweep: usize,
    /// The refusal that stopped the tessellation, kept since the tessellator can only report a failure of its own.
    refused: Option<Error>,
}

impl GeometryBuilder for Sink<'_> {
    fn begin_geometry(&mut self) {
        self.first_vertex = as_index(self.buffers.vertices.len() / VERTEX_FLOATS);
    }

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        let first = self.first_vertex;
        self.buffers
            .indices
            .extend([first + a.offset(), first + b.offset(), first + c.offset()]);
    }
}

impl FillGeometryBuilder for Sink<'_> {
    fn add_fill_vertex(&mut self, vertex: FillVertex) -> Result<VertexId, GeometryBuilderError> {
        // each vertex is charged as an edge is, and for the sweep it costs, before it is kept: a drawing whose outline
        // crosses itself many times makes a vertex at every crossing, and the tessellation stops at the first refusal
        if let Err(error) = self
            .work
            .made_segments(1, self.name)
            .and_then(|()| self.work.made_tessellation(self.sweep, self.name))
        {
            self.refused = Some(error);
            return Err(GeometryBuilderError::TooManyVertices);
        }
        let at = vertex.position();
        let moved = self.view.point(Point::new(f64::from(at.x), f64::from(at.y)));
        let own = as_index(self.buffers.vertices.len() / VERTEX_FLOATS) - self.first_vertex;
        let [red, green, blue, alpha] = self.paint.rgba;
        #[expect(clippy::cast_possible_truncation, reason = "positions are f32 in the buffers")]
        self.buffers.vertices.extend([
            moved.x as f32,
            moved.y as f32,
            red,
            green,
            blue,
            alpha,
            self.paint.item,
            self.paint.order,
        ]);
        Ok(VertexId(own))
    }
}

/// The colour `rgba` (straight, bytes) at `alpha` (0 to 1) as premultiplied floats.
fn premultiplied(rgba: [u8; 4], alpha: f64) -> [f32; 4] {
    let channel = |byte: u8| f64::from(byte) / 255.0 * alpha;
    #[expect(clippy::cast_possible_truncation, reason = "colours are f32 in the buffers")]
    [
        channel(rgba[0]) as f32,
        channel(rgba[1]) as f32,
        channel(rgba[2]) as f32,
        alpha as f32,
    ]
}

/// Whether every coordinate of `points`, in design pixels and moved by `view`, is within the limit of the
/// tessellator's f32.
///
/// # Errors
/// [`Error::Parameter`] naming the coordinate.
fn reachable<'a>(points: impl Iterator<Item = &'a Point>, view: &View) -> Result<(), Error> {
    let farthest = |x: f64, y: f64| x.abs().max(y.abs());
    for point in points {
        let moved = view.point(*point);
        within_device("a coordinate", farthest(point.x, point.y))?;
        within_device("a coordinate", farthest(moved.x, moved.y))?;
    }
    Ok(())
}

/// The pieces a stroke of round joins (`round.0`) and round ends (`round.1`), half a width `half`, needs once its
/// outline is made with bevel joins and butt ends: a half disc beyond each end of an open outline, and at each corner
/// that turns enough for the bevel to miss the arc by more than [`FLATNESS`], the circular segment between the bevel
/// and the arc. The pieces lie beside the outline and not over it, because the tessellator leaves holes where a disc
/// overlaps a stroke that is tangent to it. A bevel of the turn `a` is `half * (1 - cos(a / 2))` short of the arc, so a
/// smooth curve, whose corners turn by a fraction of a degree, gets none.
fn round_wedges(polylines: &[Polyline], round: (bool, bool), half: f64) -> Vec<Ring> {
    // the wedge is deeper than FLATNESS when cos(a) < 2c^2 - 1, c = 1 - FLATNESS / half; none is when c <= 0
    let c = 1.0 - FLATNESS / half;
    if c <= 0.0 {
        return Vec::new();
    }
    let cos_limit = 2.0 * c * c - 1.0;
    // an arc of step s lies within half * s^2 / 8 of its chord
    let step = (8.0 * FLATNESS / half).sqrt();
    let mut wedges = Vec::new();
    for polyline in polylines {
        // flattening makes no edge of length 0 and drops a closing repeat of the start, so every edge has a direction
        let corners = &polyline.points;
        let count = corners.len();
        if count < 2 {
            continue;
        }
        let direction = |from: Point, to: Point| {
            let (x, y) = (to.x - from.x, to.y - from.y);
            let length = (x * x + y * y).sqrt();
            (x / length, y / length)
        };
        for (index, at) in corners.iter().enumerate() {
            let before = (index > 0 || polyline.closed).then(|| corners[(index + count - 1) % count]);
            let after = (index + 1 < count || polyline.closed).then(|| corners[(index + 1) % count]);
            let (incoming, outgoing) = match (before, after) {
                (Some(before), Some(after)) if round.0 => (direction(before, *at), direction(*at, after)),
                (None, Some(after)) if round.1 => {
                    let outward = direction(after, *at);
                    (outward, (-outward.0, -outward.1))
                }
                (Some(before), None) if round.1 => {
                    let outward = direction(before, *at);
                    (outward, (-outward.0, -outward.1))
                }
                _ => continue,
            };
            let (cross, dot) = (
                incoming.0 * outgoing.1 - incoming.1 * outgoing.0,
                incoming.0 * outgoing.0 + incoming.1 * outgoing.1,
            );
            if before.is_some() && after.is_some() && dot >= cos_limit {
                continue;
            }
            wedges.push(wedge(*at, incoming, (cross, dot), half, step));
        }
    }
    wedges
}

/// The circular segment of radius `half` about `at` on the outer side of a turn from the direction `incoming` by the
/// angle whose cross product and dot product are `turn`, its arc in steps of at most `step`, running the way round
/// that the outline of a stroke does (positive area); a turn right back on itself is the half disc ahead of `incoming`.
fn wedge(at: Point, incoming: (f64, f64), turn: (f64, f64), half: f64, step: f64) -> Ring {
    let angle = atan2(turn.0, turn.1);
    // the outer side is the right of a left turn and the left of a right turn
    let side = if angle >= 0.0 { 1.0 } else { -1.0 };
    let (start_x, start_y) = (side * incoming.1 * half, -side * incoming.0 * half);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "an angle of at most pi over a step of at least 1e-3 is a few thousand steps"
    )]
    let steps = ((angle.abs() / step).ceil() as usize).max(1);
    #[expect(clippy::cast_precision_loss, reason = "a few thousand steps are exact")]
    let each = angle / steps as f64;
    #[expect(clippy::cast_possible_truncation, reason = "the tessellator computes in f32")]
    let mut ring: Ring = (0..=steps)
        .map(|index| {
            #[expect(clippy::cast_precision_loss, reason = "a few thousand steps are exact")]
            let (sin, cos) = sin_cos(each * index as f64);
            [
                (at.x + start_x * cos - start_y * sin) as f32,
                (at.y + start_x * sin + start_y * cos) as f32,
            ]
        })
        .collect();
    // the outline of a stroke runs the same way round whichever way the path turns, and where the two overlap the
    // nonzero rule must see the same sign on both
    if angle < 0.0 {
        ring.reverse();
    }
    ring
}

/// The corners of a polyline as a ring of the tessellator's f32.
fn ring(points: &[Point]) -> Ring {
    #[expect(clippy::cast_possible_truncation, reason = "the tessellator computes in f32")]
    points.iter().map(|at| [at.x as f32, at.y as f32]).collect()
}

/// The closed outlines of `lines`: one for each move. `lines` has straight edges only (`kurbo::flatten`), and an open
/// outline fills as if closed.
fn rings_of(lines: &BezPath) -> Vec<Ring> {
    let mut rings: Vec<Ring> = Vec::new();
    #[expect(clippy::cast_possible_truncation, reason = "the tessellator computes in f32")]
    let corner = |at: kurbo::Point| [at.x as f32, at.y as f32];
    for element in lines.elements() {
        match *element {
            PathEl::MoveTo(at) => rings.push(vec![corner(at)]),
            PathEl::LineTo(at) => {
                if let Some(current) = rings.last_mut() {
                    current.push(corner(at));
                }
            }
            // a closing edge back to the start is the fill's own, and no curve is left after flattening
            PathEl::ClosePath | PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
        }
    }
    rings
}
