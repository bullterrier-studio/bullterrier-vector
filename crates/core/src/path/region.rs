//! The region a path fills, and its separate parts.

use super::curve::flatten;
use super::{Path, SubPath};
use crate::Error;
use crate::geometry::{Point, coordinate_point};

impl Path {
    /// Whether `point` is inside the region the outline fills, by the nonzero rule: the outline winds round it at
    /// least once, in either direction. Where outlines overlap or cross themselves, round twice is inside; a hole
    /// is an outline that runs the other way. An open sub-path fills as if closed, as in SVG. A point exactly on
    /// the outline may be either.
    ///
    /// # Errors
    /// [`Error::Parameter`] if `point` is NaN, infinite or beyond ±1e100.
    pub fn contains(&self, point: impl Into<Point>) -> Result<bool, Error> {
        let point = coordinate_point("point", point)?;
        let winding: i32 = self
            .subpaths
            .iter()
            .map(|subpath| winding(&subpath.outline(), point))
            .sum();
        Ok(winding != 0)
    }

    /// The separate parts of the path, in path order: each closed outline together with the holes inside it (a
    /// closed sub-path whose start lies inside the outline), and each open sub-path alone. An outline inside a hole
    /// is a part of its own, and holes inside it are its. A hole written before its outline stays with it, and a part
    /// keeps its sub-paths in the order they were written. An empty path has no parts.
    #[must_use]
    pub fn split(&self) -> Vec<Self> {
        let outlines: Vec<Vec<Point>> = self.subpaths.iter().map(SubPath::outline).collect();
        // how many other closed outlines each closed sub-path lies inside, judged by where it starts
        let containers: Vec<Vec<usize>> = self
            .subpaths
            .iter()
            .enumerate()
            .map(|(index, subpath)| {
                if !subpath.closed {
                    return Vec::new();
                }
                (0..outlines.len())
                    .filter(|&other| {
                        other != index && self.subpaths[other].closed && winding(&outlines[other], subpath.start) != 0
                    })
                    .collect()
            })
            .collect();
        // shallowest first, so that the outline a hole belongs to is placed before it
        let mut order: Vec<usize> = (0..self.subpaths.len()).collect();
        order.sort_by_key(|&index| containers[index].len());
        // the index of the sub-path that opens each one's part
        let mut part: Vec<Option<usize>> = vec![None; self.subpaths.len()];
        for index in order {
            let inside = &containers[index];
            let is_hole = inside.len() % 2 == 1;
            let outline = if is_hole {
                inside
                    .iter()
                    .copied()
                    .filter(|&other| part[other].is_some())
                    .max_by_key(|&other| (containers[other].len(), std::cmp::Reverse(other)))
            } else {
                None
            };
            part[index] = Some(outline.and_then(|outline| part[outline]).unwrap_or(index));
        }
        let mut slots: Vec<Option<usize>> = vec![None; self.subpaths.len()];
        let mut parts: Vec<Self> = Vec::new();
        for (index, subpath) in self.subpaths.iter().enumerate() {
            let Some(opener) = part[index] else { continue };
            let slot = *slots[opener].get_or_insert_with(|| {
                parts.push(Self::new());
                parts.len() - 1
            });
            parts[slot].subpaths.push(subpath.clone());
        }
        parts
    }
}

impl SubPath {
    /// The corners of the flattened outline, with the line from the last back to the first implied, as the region
    /// is filled.
    fn outline(&self) -> Vec<Point> {
        let edges = flatten(&self.curves());
        let mut corners: Vec<Point> = edges.iter().map(|edge| edge.from).collect();
        if let (Some(first), Some(last)) = (edges.first(), edges.last())
            && last.to != first.from
        {
            corners.push(last.to);
        }
        corners
    }
}

/// How many times the closed polygon through `corners` winds round `point`, counted by the edges that cross the
/// horizontal line through the point to its right: up counts one way, down the other.
fn winding(corners: &[Point], point: Point) -> i32 {
    let mut turns = 0;
    for (index, from) in corners.iter().enumerate() {
        let to = corners[(index + 1) % corners.len()];
        // positive when `point` is left of the edge from `from` to `to`
        let side = (to.x - from.x) * (point.y - from.y) - (point.x - from.x) * (to.y - from.y);
        if from.y <= point.y {
            if to.y > point.y && side > 0.0 {
                turns += 1;
            }
        } else if to.y <= point.y && side < 0.0 {
            turns -= 1;
        }
    }
    turns
}

/// The region a path fills, cut into spans by horizontal lines: the path flattened once, to be asked at many heights.
pub(crate) struct Region {
    /// Every straight edge of every outline, the closing edge of each included.
    edges: Vec<(Point, Point)>,
}

impl Region {
    /// The region `path` fills by the nonzero rule, as [`Path::contains`] decides it.
    pub(crate) fn of(path: &Path) -> Self {
        let mut edges = Vec::new();
        for subpath in &path.subpaths {
            let corners = subpath.outline();
            for (index, from) in corners.iter().enumerate() {
                edges.push((*from, corners[(index + 1) % corners.len()]));
            }
        }
        Self { edges }
    }

    /// How many straight edges the region has.
    pub(crate) fn len(&self) -> usize {
        self.edges.len()
    }

    /// The lowest and the highest `y` of the outline, or `None` if the region has no edge.
    pub(crate) fn extent(&self) -> Option<(f64, f64)> {
        self.edges
            .iter()
            .flat_map(|(from, to)| [from.y, to.y])
            .fold(None, |range, y| match range {
                None => Some((y, y)),
                Some((low, high)) => Some((low.min(y), high.max(y))),
            })
    }

    /// The stretches of the horizontal line at `y` that lie inside the region, left to right, apart from each other
    /// and each of some length. An edge covers the `y` from its upper end up to but not including its lower end, so a
    /// line through a corner counts it once and the bottom edge of a rectangle is outside it. `charge` is told how many
    /// edges the scan visits, before it visits them.
    ///
    /// # Errors
    /// What `charge` returns.
    pub(crate) fn spans_at(
        &self,
        y: f64,
        charge: impl FnOnce(usize) -> Result<(), Error>,
    ) -> Result<Vec<(f64, f64)>, Error> {
        charge(self.edges.len())?;
        // where an edge crosses the line, and whether it goes down (+1) or up (-1)
        let mut crossings: Vec<(f64, i32)> = self
            .edges
            .iter()
            .filter(|(from, to)| (from.y <= y) != (to.y <= y))
            .map(|(from, to)| {
                let x = from.x + (y - from.y) / (to.y - from.y) * (to.x - from.x);
                (x, if to.y > from.y { 1 } else { -1 })
            })
            .collect();
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut spans: Vec<(f64, f64)> = Vec::new();
        let (mut winding, mut start) = (0, 0.0);
        for (x, turn) in crossings {
            let before = winding;
            winding += turn;
            if before == 0 {
                start = x;
            } else if winding == 0 && x > start {
                match spans.last_mut() {
                    // spans that meet are one
                    Some(last) if last.1 >= start => last.1 = x,
                    _ => spans.push((start, x)),
                }
            }
        }
        Ok(spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    type Outcome = Result<(), Box<dyn std::error::Error>>;

    fn spans(svg: &str, y: f64) -> Result<Vec<(f64, f64)>, Error> {
        Region::of(&Path::from_svg_path(svg)?).spans_at(y, |_| Ok(()))
    }

    /// `spans` equal `want` within `tolerance`, which allows for a curve flattened to straight edges.
    fn close(spans: &[(f64, f64)], want: &[(f64, f64)], tolerance: f64) -> bool {
        spans.len() == want.len()
            && spans
                .iter()
                .zip(want)
                .all(|(a, b)| (a.0 - b.0).abs() <= tolerance && (a.1 - b.1).abs() <= tolerance)
    }

    /// A rule, an outline, a height, and the spans there.
    type Row<'a> = (&'a str, &'a str, f64, &'a [(f64, f64)]);

    const RECT: &str = "M0 0H100V50H0Z";
    const HOLE: &str = "M0 0H100V100H0Z M25 25V75H75V25Z";
    const NESTED_SAME_WAY: &str = "M0 0H100V100H0Z M25 25H75V75H25Z";
    const OPEN_TRIANGLE: &str = "M0 0L100 0L50 100";
    const APART: &str = "M0 0H10V10H0Z M20 0H30V10H20Z";
    const TOUCHING: &str = "M0 0H10V10H0Z M10 0H20V10H10Z";
    const DIAMOND: &str = "M50 0L100 50L50 100L0 50Z";
    const APEX_UP: &str = "M50 0L100 100L0 100Z";

    #[test]
    fn spans_are_what_the_outline_fills_at_a_height() -> Outcome {
        // (rule, outline, y, spans), each by hand; an edge covers its upper end and not its lower
        let rows: [Row<'_>; 17] = [
            ("rect: the whole width", RECT, 25.0, &[(0.0, 100.0)]),
            ("rect: its top y is inside", RECT, 0.0, &[(0.0, 100.0)]),
            ("rect: its bottom y is outside", RECT, 50.0, &[]),
            ("rect: above it", RECT, -1.0, &[]),
            ("rect: below it", RECT, 51.0, &[]),
            (
                "hole: the ring has two spans",
                HOLE,
                50.0,
                &[(0.0, 25.0), (75.0, 100.0)],
            ),
            ("hole: above the hole it is whole", HOLE, 10.0, &[(0.0, 100.0)]),
            (
                "hole: at the hole's top y the hole starts",
                HOLE,
                25.0,
                &[(0.0, 25.0), (75.0, 100.0)],
            ),
            (
                "hole: at the hole's bottom y it is filled again",
                HOLE,
                75.0,
                &[(0.0, 100.0)],
            ),
            (
                "nonzero: winding twice is inside, not a hole",
                NESTED_SAME_WAY,
                50.0,
                &[(0.0, 100.0)],
            ),
            (
                "open outline is closed implicitly",
                OPEN_TRIANGLE,
                50.0,
                &[(25.0, 75.0)],
            ),
            ("two parts apart", APART, 5.0, &[(0.0, 10.0), (20.0, 30.0)]),
            ("two parts that touch are one span", TOUCHING, 5.0, &[(0.0, 20.0)]),
            ("through the side corners of a diamond", DIAMOND, 50.0, &[(0.0, 100.0)]),
            ("through the top corner there is no length", APEX_UP, 0.0, &[]),
            ("just under the top corner", APEX_UP, 10.0, &[(45.0, 55.0)]),
            ("an empty path has none", "", 5.0, &[]),
        ];
        for (rule, svg, y, want) in rows {
            assert_eq!(spans(svg, y)?, want, "{rule}");
        }
        Ok(())
    }

    #[test]
    fn a_circle_is_as_wide_as_its_chord() -> Outcome {
        let circle = Path::circle((10.0, 20.0), 50.0)?;
        let region = Region::of(&circle);
        // chord at distance d from the centre: 2 sqrt(r^2 - d^2); the flattening is within 0.01 of the curve
        for d in [0.0, 10.0, -30.0, 49.0] {
            let half = (50.0_f64 * 50.0 - d * d).sqrt();
            let got = region.spans_at(20.0 + d, |_| Ok(()))?;
            assert!(close(&got, &[(10.0 - half, 10.0 + half)], 0.05), "d {d}: {got:?}");
        }
        Ok(())
    }

    #[test]
    fn the_scan_is_charged_one_visit_to_every_edge_and_a_refusal_is_returned() -> Outcome {
        for (svg, edges) in [(RECT, 4), (OPEN_TRIANGLE, 3), (APART, 8)] {
            let region = Region::of(&Path::from_svg_path(svg)?);
            let mut charged = None;
            region.spans_at(5.0, |count| {
                charged = Some(count);
                Ok(())
            })?;
            assert_eq!(charged, Some(edges), "{svg}");
            assert_eq!(region.len(), edges, "{svg}");
        }
        let region = Region::of(&Path::from_svg_path(RECT)?);
        let refused = region.spans_at(5.0, |_| Err(Error::parameter("inside", "too much")));
        assert!(refused.is_err());
        Ok(())
    }

    #[test]
    fn the_extent_is_the_lowest_and_highest_y() -> Outcome {
        assert_eq!(Region::of(&Path::from_svg_path(HOLE)?).extent(), Some((0.0, 100.0)));
        assert_eq!(Region::of(&Path::new()).extent(), None);
        Ok(())
    }

    proptest! {
        /// Whatever the outline, the spans are apart and in order, the middle of each is inside the path by
        /// `Path::contains` (the point query, which shares no code with the scan), and the points just beside it are not.
        #[test]
        fn spans_are_the_inside_of_the_path(
            corners in proptest::collection::vec((0_u8..=60, 0_u8..=60), 3..9),
            closed in any::<bool>(),
            y in prop_oneof![(0_u8..=60).prop_map(f64::from), 0.0..60.0_f64],
        ) {
            let path = Path::polyline(corners.iter().map(|&(x, y)| (f64::from(x), f64::from(y))), closed).unwrap_or_default();
            let found = Region::of(&path).spans_at(y, |_| Ok(())).map_err(|e| TestCaseError::fail(e.to_string()))?;
            let side = 0.001;
            // and every point of the path is in a span, but those a hundredth from an end of one
            for step in 0..=160_u32 {
                let x = f64::from(step) * 0.37;
                if found.iter().all(|&(start, end)| (x - start).abs() > 0.01 && (x - end).abs() > 0.01) {
                    let in_a_span = found.iter().any(|&(start, end)| start < x && x < end);
                    let contained = path.contains((x, y)).map_err(|e| TestCaseError::fail(e.to_string()))?;
                    prop_assert_eq!(contained, in_a_span, "x {}", x);
                }
            }
            for (index, &(start, end)) in found.iter().enumerate() {
                prop_assert!(end > start);
                if let Some(&(_, before)) = index.checked_sub(1).and_then(|at| found.get(at)) {
                    prop_assert!(start > before);
                }
                let inside = |x: f64| path.contains((x, y)).map_err(|e| TestCaseError::fail(e.to_string()));
                // away from an edge a point is decided without doubt: spans and gaps of a hundredth or more
                if end - start > 0.01 {
                    prop_assert!(inside(f64::midpoint(start, end))?);
                }
                let gap_before = index.checked_sub(1).map_or(f64::INFINITY, |at| start - found[at].1);
                if gap_before > 0.01 {
                    prop_assert!(!inside(start - side)?);
                }
                let gap_after = found.get(index + 1).map_or(f64::INFINITY, |next| next.0 - end);
                if gap_after > 0.01 {
                    prop_assert!(!inside(end + side)?);
                }
            }
        }
    }
}
