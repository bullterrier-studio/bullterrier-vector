//! `Drawing::to_mesh`: the drawing as triangles. The only module that uses the tessellator crate (`lyon_tessellation`);
//! nothing of it reaches the API.
//!
//! Every shape is two layers at most, its fill and then its stroke. A layer is a set of closed outlines in design
//! pixels (the flattened path; or the outline of the stroke, which `kurbo` expands as it does for the pixels) filled by
//! the nonzero rule, so that a hole, an overlap and a stroke that crosses itself are each covered by one triangle at a
//! point. The tessellator computes in f32, so it works in design pixels, where the numbers are the drawing's own, and
//! the vertices are moved through the view afterwards: a drawing at twice the size has positions exactly twice as
//! large.

use super::outline::{bez_path, dots, stroke_outline, within_device};
use super::view::View;
use super::{Size, in_shape};
use crate::limits::Work;
use crate::path::{FLATNESS, Polyline};
use crate::{Blend, Drawing, Error, Point, Shape, Values};
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
    /// has no triangles. A shape with an empty path adds nothing but keeps its number as `item`. Curves are flattened,
    /// as for the pixels, to within 0.01 design pixels. Known exception to "once": a stroke with round joins or ends that
    /// doubles back at a sharp turn may have a patch covered three times, by `lyon_tessellation` (about 1 in 1,000
    /// random strokes).
    ///
    /// The triangles are made in design pixels in f32, so a coordinate beyond 10 million is refused.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `width` or `height` if a side of `size` is not a whole number from 1 to 100,000;
    /// naming the shape if a coordinate or the stroke width of one lies beyond 10 million pixels, in design pixels or
    /// at this size, or if the edges flattened for it, the edges of its stroke's outline or the vertices made for the
    /// draw pass 10 million, or if the tessellation of the draw would take more than about a second (the vertices it
    /// makes times the edges alive at a sweep line pass 250 million: an outline that crosses itself thousands of times,
    /// or has thousands of edges side by side); otherwise as [`Drawing::to_svg`]. [`Error::Geometry`] if the tessellator fails, which a
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
        for shape in &shapes {
            maker.shape(shape, &view)?;
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

impl Maker<'_> {
    /// Adds the layers of `shape` seen through `view`.
    fn shape(&mut self, shape: &Shape, view: &View) -> Result<(), Error> {
        let name = format!("shape {}", shape.index());
        let work = &self.work;
        let mut charge = |count| work.made_segments(count, &name);
        let polylines = shape.path().flattened(&mut charge)?;
        if polylines.is_empty() {
            return Ok(());
        }
        reachable(&polylines, view).map_err(|error| in_shape(shape, "", error))?;
        let style = shape.style();
        // a shape no generator ranked has order 0
        let order = shape.attributes().order().unwrap_or(0.0);
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            reason = "a list of at most a million shapes is exact in an f32, and an order from 0 to 1 loses nothing visible"
        )]
        let (item, order) = (shape.index() as f32, order as f32);
        let paint = |rgba: [u8; 4]| {
            let alpha = f64::from(rgba[3]) / 255.0 * style.opacity();
            Paint {
                rgba: premultiplied(rgba, alpha),
                blend: style.blend(),
                item,
                order,
            }
        };
        if let Some(rgba) = style.fill().rgba()
            && style.opacity() * f64::from(rgba[3]) > 0.0
        {
            let rings: Vec<Ring> = polylines.iter().map(|polyline| ring(&polyline.points)).collect();
            self.layer(&rings, &paint(rgba), view, &name)?;
        }
        let Some(stroke) = style.stroke() else { return Ok(()) };
        let Some(rgba) = stroke.color().rgba() else {
            return Ok(());
        };
        let device = view
            .length(stroke.width())
            .map_err(|error| in_shape(shape, "stroke width ", error))?;
        within_device("the stroke width", device).map_err(|error| in_shape(shape, "", error))?;
        if style.opacity() * f64::from(rgba[3]) > 0.0 {
            let outline = stroke_outline(&bez_path(&polylines), stroke, stroke.width(), FLATNESS, &name)?;
            let mut rings = rings_of(&outline);
            let mut charge = |count| self.work.made_segments(count, &name);
            for dot in dots(&polylines, stroke.cap(), stroke.width(), &mut charge)? {
                rings.extend(dot.iter().map(|polyline| ring(&polyline.points)));
            }
            self.layer(&rings, &paint(rgba), view, &name)?;
        }
        Ok(())
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
        let options = FillOptions::default().with_fill_rule(FillRule::NonZero);
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

/// Whether every coordinate of `polylines`, in design pixels and moved by `view`, is within the limit of the
/// tessellator's f32.
///
/// # Errors
/// [`Error::Parameter`] naming the coordinate.
fn reachable(polylines: &[Polyline], view: &View) -> Result<(), Error> {
    let farthest = |x: f64, y: f64| x.abs().max(y.abs());
    for point in polylines.iter().flat_map(|polyline| &polyline.points) {
        let moved = view.point(*point);
        within_device("a coordinate", farthest(point.x, point.y))?;
        within_device("a coordinate", farthest(moved.x, moved.y))?;
    }
    Ok(())
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
