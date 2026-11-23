//! The one tolerance of "looks the same", and the helpers that apply it. Two outputs of the same drawing by different
//! routes (a window against the whole render, native against WebAssembly, one platform against another) are not
//! promised to be equal bytes; they are promised to look the same, which is this file's numbers and nothing else.
//! `tests/wasm/compare.mjs` repeats these numbers for the Node tests: this file is their source.

/// Pixels: at most this many levels (of 255) apart on all but a thousandth of the samples...
pub(crate) const NEAR_LEVEL: u8 = 1;
/// ...of which at most this many in a thousand may be further apart (but never past [`MAX_LEVEL`]).
pub(crate) const FAR_PER_THOUSAND: usize = 1;
/// Pixels: no sample is further apart than this many levels of 255.
pub(crate) const MAX_LEVEL: u8 = 2;
/// Geometry (SVG numbers, mesh positions), in output units (pixels): one 8-bit level of edge coverage is 1/255 of a
/// pixel, so a coordinate that moves by less than 0.004 cannot change a pixel by a level.
pub(crate) const GEOMETRY_TOLERANCE: f64 = 0.004;

/// How two same-sized RGBA images differ, as the product of each colour with its alpha (a faint pixel's straight colour
/// moves by several levels for one level of what is drawn): the samples compared, those past [`NEAR_LEVEL`], the
/// largest difference and the index of the first sample past [`NEAR_LEVEL`].
pub(crate) struct Levels {
    pub(crate) samples: usize,
    pub(crate) far: usize,
    pub(crate) largest: u8,
    pub(crate) first_far: Option<usize>,
}

fn premultiplied(pixel: [u8; 4]) -> [u8; 4] {
    let alpha = u16::from(pixel[3]);
    #[expect(clippy::cast_possible_truncation, reason = "at most 255")]
    let times = |channel: u8| ((u16::from(channel) * alpha + 127) / 255) as u8;
    [times(pixel[0]), times(pixel[1]), times(pixel[2]), pixel[3]]
}

/// The differences of two images of equal length, in levels of the premultiplied samples.
pub(crate) fn levels(expected: &[u8], got: &[u8]) -> Levels {
    let mut found = Levels {
        samples: expected.len(),
        far: 0,
        largest: 0,
        first_far: None,
    };
    for (at, (a, b)) in expected
        .as_chunks::<4>()
        .0
        .iter()
        .zip(got.as_chunks::<4>().0)
        .enumerate()
    {
        for (channel, (x, y)) in premultiplied(*a).into_iter().zip(premultiplied(*b)).enumerate() {
            let apart = x.abs_diff(y);
            found.largest = found.largest.max(apart);
            if apart > NEAR_LEVEL {
                found.far += 1;
                found.first_far.get_or_insert(at * 4 + channel);
            }
        }
    }
    found
}

/// `Ok` when two RGBA images look the same, else a message with the counts (never the buffers).
pub(crate) fn pixels_look_the_same(expected: &[u8], got: &[u8]) -> Result<(), String> {
    if expected.len() != got.len() {
        return Err(format!("{} bytes, expected {}", got.len(), expected.len()));
    }
    let found = levels(expected, got);
    if found.largest > MAX_LEVEL || found.far > found.samples * FAR_PER_THOUSAND / 1000 {
        return Err(format!(
            "{} of {} samples are more than {NEAR_LEVEL} level apart, the largest by {}, the first at sample {:?}",
            found.far, found.samples, found.largest, found.first_far
        ));
    }
    Ok(())
}

/// The numbers of a text and what is left of it with each number replaced by `#`.
fn numbers_and_skeleton(text: &str) -> (Vec<f64>, String) {
    let bytes = text.as_bytes();
    let (mut numbers, mut skeleton, mut at) = (Vec::new(), String::new(), 0);
    while at < bytes.len() {
        let starts = bytes[at].is_ascii_digit()
            || (matches!(bytes[at], b'-' | b'.')
                && bytes
                    .get(at + 1)
                    .is_some_and(|next| next.is_ascii_digit() || *next == b'.'));
        let word = at > 0 && bytes[at - 1].is_ascii_alphabetic() && bytes[at] != b'-';
        if starts && !word {
            let mut end = at + 1;
            while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
                end += 1;
            }
            if let Ok(number) = text[at..end].parse() {
                numbers.push(number);
                skeleton.push('#');
                at = end;
                continue;
            }
        }
        skeleton.push(char::from(bytes[at]));
        at += 1;
    }
    (numbers, skeleton)
}

/// `Ok` when two SVG (or path) texts are the same but for numbers within [`GEOMETRY_TOLERANCE`].
pub(crate) fn svg_looks_the_same(expected: &str, got: &str) -> Result<(), String> {
    let ((a, shape_a), (b, shape_b)) = (numbers_and_skeleton(expected), numbers_and_skeleton(got));
    if shape_a != shape_b {
        return Err("the text around the numbers differs".to_owned());
    }
    match a.iter().zip(&b).position(|(x, y)| (x - y).abs() > GEOMETRY_TOLERANCE) {
        Some(index) => Err(format!("number {index} is {}, expected {}", b[index], a[index])),
        None => Ok(()),
    }
}
