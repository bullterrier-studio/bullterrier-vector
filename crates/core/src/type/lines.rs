//! The lines a text is set in: which part of the content each shows.

use std::ops::Range;

/// A line of the content: the bytes it shows, and which characters of the content they are.
pub(super) struct Line {
    /// The bytes of the content the line shows.
    pub(super) bytes: Range<usize>,
    /// The characters of the content the line shows, counted from 0 (a line break is one).
    pub(super) characters: Range<usize>,
}

/// The lines of `content`: one for every `"\n"`-separated part.
pub(super) fn plan(content: &str) -> Vec<Line> {
    let mut start = 0;
    let mut before = 0;
    content
        .split('\n')
        .map(|text| {
            let count = text.chars().count();
            let line = Line {
                bytes: start..start + text.len(),
                characters: before..before + count,
            };
            start += text.len() + 1;
            before += count + 1;
            line
        })
        .collect()
}
