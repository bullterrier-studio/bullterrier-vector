//! Reading the buffers `Drawing::to_mesh` fills, and counting how many triangles cover a point: the oracle the mesh
//! tests measure against closed-form and brute-force geometry written in the tests.

use bullterrier_vector::{Drawing, Error, Mesh, Size, Values};

/// The floats a vertex has: `x, y`, premultiplied `r, g, b, a`, `item`, `order`.
pub(crate) const FLOATS: usize = 8;

/// A triangle's corners.
pub(crate) type Triangle = [(f64, f64); 3];

/// One vertex read back from the buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Vertex {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) rgba: [f32; 4],
    pub(crate) item: u32,
    pub(crate) order: f32,
}

impl Vertex {
    /// What a vertex says about how it is painted, bit for bit: its colour and its item.
    pub(crate) fn paint(&self) -> ([u32; 4], u32) {
        (self.rgba.map(f32::to_bits), self.item)
    }
}

/// The mesh of `drawing` at `width` by `height` pixels with no Param values, in a new `Mesh`.
pub(crate) fn mesh_of(drawing: &Drawing, width: f64, height: f64) -> Result<Mesh, Error> {
    let mut mesh = Mesh::new();
    drawing.to_mesh(Size::px(width, height), &Values::new(), &mut mesh)?;
    Ok(mesh)
}

/// Vertex number `index`.
pub(crate) fn vertex(mesh: &Mesh, index: u32) -> Vertex {
    let at = index as usize * FLOATS;
    let v = &mesh.vertices()[at..at + FLOATS];
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "an item is a whole index"
    )]
    let item = v[6] as u32;
    Vertex {
        x: f64::from(v[0]),
        y: f64::from(v[1]),
        rgba: [v[2], v[3], v[4], v[5]],
        item,
        order: v[7],
    }
}

/// The vertices of triangle number `number`.
pub(crate) fn corners(mesh: &Mesh, number: usize) -> [Vertex; 3] {
    let at = number * 3;
    let i = &mesh.indices()[at..at + 3];
    [vertex(mesh, i[0]), vertex(mesh, i[1]), vertex(mesh, i[2])]
}

/// The triangles of the mesh as positions, in index order.
pub(crate) fn triangles(mesh: &Mesh) -> Vec<Triangle> {
    (0..mesh.indices().len() / 3)
        .map(|number| corners(mesh, number).map(|v| (v.x, v.y)))
        .collect()
}

/// The area of a triangle.
pub(crate) fn area(triangle: &Triangle) -> f64 {
    let [(ax, ay), (bx, by), (cx, cy)] = *triangle;
    ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)).abs() / 2.0
}

/// How far from an edge of a triangle a point must be to count as clearly inside or outside it.
const EDGE_BAND: f64 = 1.0e-3;

/// How many of `triangles` cover `point`, or `None` when it lies within a thousandth of a pixel of an edge of some
/// triangle, where f32 positions and the f64 test may disagree about which side it is on.
pub(crate) fn covering(triangles: &[Triangle], point: (f64, f64)) -> Option<usize> {
    let mut count = 0;
    for triangle in triangles {
        let [a, b, c] = *triangle;
        let turn = (b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1);
        if turn == 0.0 {
            continue;
        }
        // signed distances to the three edge lines, positive on the triangle's side
        let sides = [(a, b), (b, c), (c, a)].map(|(from, to)| {
            let (dx, dy) = (to.0 - from.0, to.1 - from.1);
            turn.signum() * (dx * (point.1 - from.1) - dy * (point.0 - from.0)) / libm::hypot(dx, dy)
        });
        let nearest = sides.into_iter().fold(f64::INFINITY, f64::min);
        if nearest > EDGE_BAND {
            count += 1;
        } else if nearest > -EDGE_BAND {
            return None;
        }
    }
    Some(count)
}

/// Checks the layout every mesh has: whole vertices and triangles, indices that point at vertices, and draw ranges
/// that tile the indices, none empty and none the same blend as the one before it.
pub(crate) fn check_layout(mesh: &Mesh) -> Result<(), String> {
    let (vertices, indices, ranges) = (mesh.vertices(), mesh.indices(), mesh.ranges());
    if vertices.len() % FLOATS != 0 || indices.len() % 3 != 0 || ranges.len() % 3 != 0 {
        return Err(format!(
            "lengths {} / {} / {} are not whole vertices, triangles and ranges",
            vertices.len(),
            indices.len(),
            ranges.len()
        ));
    }
    let count = vertices.len() / FLOATS;
    if let Some(bad) = indices.iter().find(|&&index| index as usize >= count) {
        return Err(format!("index {bad} with {count} vertices"));
    }
    if let Some(bad) = vertices.iter().find(|v| !v.is_finite()) {
        return Err(format!("a vertex float is {bad}"));
    }
    let mut next = 0;
    let mut before = None;
    for range in ranges.chunks(3) {
        let (first, length, blend) = (range[0] as usize, range[1] as usize, range[2]);
        if first != next || length == 0 || length % 3 != 0 || blend > 1 || before == Some(blend) {
            return Err(format!("range {range:?} after {next} indices, blend before {before:?}"));
        }
        next += length;
        before = Some(blend);
    }
    if next != indices.len() {
        return Err(format!("ranges cover {next} of {} indices", indices.len()));
    }
    Ok(())
}

/// An error unless the two meshes hold the same triangles: equal indices and ranges, and vertices that agree in
/// position to a thousandth of a pixel (the shadow moves f32 positions, the hand-built copy f64 ones) and exactly in
/// colour. The item and the order are left out: a shadow has those of its shape, a copy those a stack gives it.
pub(crate) fn same_triangles(left: &Mesh, right: &Mesh) -> Result<(), Box<dyn std::error::Error>> {
    let (left_floats, right_floats) = (left.vertices(), right.vertices());
    let vertices_agree = left_floats.len() == right_floats.len()
        && left_floats
            .chunks(FLOATS)
            .zip(right_floats.chunks(FLOATS))
            .all(|(a, b)| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3 && a[2..6] == b[2..6]);
    if vertices_agree && left.indices() == right.indices() && left.ranges() == right.ranges() {
        Ok(())
    } else {
        Err(format!(
            "{} against {} vertices, {} against {} indices, ranges {:?} against {:?}",
            left_floats.len() / FLOATS,
            right_floats.len() / FLOATS,
            left.indices().len(),
            right.indices().len(),
            left.ranges(),
            right.ranges()
        )
        .into())
    }
}
