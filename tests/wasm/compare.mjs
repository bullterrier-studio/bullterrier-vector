// Whether two outputs look the same: the comparison of the native-against-WebAssembly tests, for pixels, SVG text and
// meshes. No output is promised to be equal bytes across routes; each is compared within the one tolerance below.
import assert from "node:assert/strict";

// The one tolerance of "looks the same" for the Node tests. The numbers are those of
// crates/core/tests/it/common/looks_the_same.rs (NEAR_LEVEL, FAR_PER_THOUSAND, MAX_LEVEL, GEOMETRY_TOLERANCE), which is
// their source: change them there first. A pixel is compared as its premultiplied colour and its alpha, the values
// drawn: the straight colour of a nearly transparent pixel is the premultiplied one divided by a small alpha, which
// magnifies a one-level difference.
const NEAR_LEVEL = 1; // all but a thousandth of the channel values (0..255) are at most this far apart...
const FAR_PER_THOUSAND = 1;
const MAX_LEVEL = 2; // ...and none is further than this
const GEOMETRY_TOLERANCE = 0.004; // output units (pixels): 1/255 of a pixel, the least edge shift that changes a level

/** The channel values of two RGBA images that are more than NEAR_LEVEL apart, as premultiplied colour and alpha: how
 *  many, the largest difference in levels, and the first such pixel. */
export function differences(expected, actual, width) {
  const premultiplied = (image, at) => [
    ...[0, 1, 2].map((c) => Math.round((image[at + c] * image[at + 3]) / 255)),
    image[at + 3],
  ];
  let count = 0;
  let largest = 0;
  let first = null;
  for (let at = 0; at < expected.length; at += 4) {
    const wanted = premultiplied(expected, at);
    const got = premultiplied(actual, at);
    wanted.forEach((value, channel) => {
      const difference = Math.abs(value - got[channel]);
      largest = Math.max(largest, difference);
      if (difference > NEAR_LEVEL) {
        count += 1;
        first ??= { x: (at / 4) % width, y: Math.floor(at / 4 / width), channel, expected: value, actual: got[channel] };
      }
    });
  }
  return { count, largest, first };
}

/** Assert that two RGBA images look the same, naming the count and the first differing pixel if they do not. The count
 *  allowed is a thousandth part of `of` channel values: the images' own, or those of the whole picture that they are a
 *  window of. */
export function assertLookTheSame(expected, actual, width, label, of = expected.length) {
  assert.equal(actual.length, expected.length, `${label}: the length of the pixels`);
  const { count, largest, first } = differences(expected, actual, width);
  const allowed = Math.floor((of * FAR_PER_THOUSAND) / 1000);
  assert.ok(
    count <= allowed && largest <= MAX_LEVEL,
    `${label}: ${count} of ${expected.length} channel values are more than ${NEAR_LEVEL} level apart (${allowed} allowed), ` +
      `the largest by ${largest}; first at ${JSON.stringify(first)}`,
  );
}

/** The numbers of a text, and the text with each number replaced by `#`. */
function numbersAndSkeleton(text) {
  const numbers = [];
  const skeleton = text.replace(/-?(?:\d+\.?\d*|\.\d+)/g, (found) => {
    numbers.push(Number(found));
    return "#";
  });
  return { numbers, skeleton };
}

/** Assert that two SVG texts are the same but for numbers within GEOMETRY_TOLERANCE. */
export function assertSvgLooksTheSame(expected, actual, label) {
  const want = numbersAndSkeleton(expected);
  const got = numbersAndSkeleton(actual);
  assert.ok(want.skeleton === got.skeleton, `${label}: the text around the numbers differs (${expected.length} against ${actual.length} characters)`);
  const index = want.numbers.findIndex((number, at) => Math.abs(number - got.numbers[at]) > GEOMETRY_TOLERANCE);
  assert.equal(index, -1, `${label}: number ${index} is ${got.numbers[index]}, expected ${want.numbers[index]}`);
}

/** Assert that two meshes look the same: equal lengths, vertex positions (floats 0 and 1 of 8) within GEOMETRY_TOLERANCE,
 *  colours (floats 2 to 5) within MAX_LEVEL of 255, the item and order (floats 6 and 7) and the integer buffers equal on
 *  all but a thousandth of their values. The buffers are typed arrays: only counts and the first index are reported. */
export function assertMeshLooksTheSame(expected, actual, label) {
  for (const key of ["vertices", "indices", "ranges"]) {
    assert.equal(actual[key].length, expected[key].length, `${label}: length of ${key}`);
  }
  const far = (floats) => {
    let count = 0;
    let first = -1;
    for (let at = 0; at < expected.vertices.length; at += 8) {
      for (const [from, to, allowed] of floats) {
        for (let k = from; k < to; k += 1) {
          if (!(Math.abs(expected.vertices[at + k] - actual.vertices[at + k]) <= allowed)) {
            count += 1;
            if (first < 0) first = at + k;
          }
        }
      }
    }
    return { count, first };
  };
  const wrong = far([[0, 2, GEOMETRY_TOLERANCE], [2, 6, MAX_LEVEL / 255], [6, 8, 0]]);
  assert.equal(wrong.count, 0, `${label}: ${wrong.count} vertex floats are too far apart, the first at index ${wrong.first}`);
  for (const key of ["indices", "ranges"]) {
    let count = 0;
    let first = -1;
    expected[key].forEach((value, at) => {
      if (value !== actual[key][at]) {
        count += 1;
        if (first < 0) first = at;
      }
    });
    // Indices and ranges are the mesh's structure, not samples: no share of them may differ.
    assert.ok(count === 0, `${label}: ${count} of ${expected[key].length} ${key} differ, the first at ${first}`);
  }
}

/** The rows and columns `[x, y, width, height]` of an RGBA image `across` pixels wide. */
export function windowOf(image, across, [x, y, width, height]) {
  const rows = [];
  for (let row = y; row < y + height; row += 1) {
    rows.push(image.subarray((row * across + x) * 4, (row * across + x + width) * 4));
  }
  return Buffer.concat(rows);
}
