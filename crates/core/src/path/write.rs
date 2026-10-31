//! Path data out: the canonical written form of a path.

use super::{Path, Segment};
use crate::geometry::Point;

impl Path {
    /// The path as SVG path data in canonical form: each sub-path is `M x y`, its segments as absolute `L`, `Q` and
    /// `C`, then `Z` if it is closed; commands are separated by one space. Numbers are the shortest text that reads
    /// back as the same value, never with an exponent, and `-0` is written `0`, so
    /// [`Path::from_svg_path`] of the result is this path exactly. An empty path is written as an empty string.
    #[must_use]
    pub fn to_svg_path(&self) -> String {
        let mut commands: Vec<String> = Vec::new();
        for subpath in &self.subpaths {
            commands.push(format!("M{}", pair(subpath.start)));
            for segment in &subpath.segments {
                commands.push(match *segment {
                    Segment::Line(end) => format!("L{}", pair(end)),
                    Segment::Quad(control, end) => format!("Q{} {}", pair(control), pair(end)),
                    Segment::Cubic(control1, control2, end) => {
                        format!("C{} {} {}", pair(control1), pair(control2), pair(end))
                    }
                });
            }
            if subpath.closed {
                commands.push("Z".to_owned());
            }
        }
        commands.join(" ")
    }
}

fn pair(point: Point) -> String {
    format!("{} {}", number(point.x), number(point.y))
}

/// `value` as the shortest decimal text that parses back to it; `-0` is `0`.
fn number(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}
