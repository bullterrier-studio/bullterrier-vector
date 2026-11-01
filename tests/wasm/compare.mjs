// Whether two RGBA images look the same: the pixel comparison of the native-against-WebAssembly tests. The rasteriser
// crate draws pixels that are required only to look the same on every target, so they are compared within a tolerance
// fixed by measurement (docs/design-decisions.md, "Output"); everything else the library computes is compared exactly.
import assert from "node:assert/strict";

// The pixel tolerance, measured on this corpus (docs/design-decisions.md, "Output", has the numbers). A pixel is compared
// as its premultiplied colour and its alpha, the values drawn: the straight-alpha colour of a nearly transparent pixel
// is the premultiplied one divided by a small alpha, which magnifies a one-level difference.
//
// Measured on this corpus (48 draws, 3,450,660 channel values) with Apple silicon NEON natively against WebAssembly
// simd128: 4 values differ, none by more than 1 level (2 as straight alpha), at most 0.005% of the values of one draw.
// The tolerance is twice the worst level and four times the worst fraction.
const MAX_LEVEL = 2; // a channel value (0..255) of a pixel may differ by at most this much...
const MAX_FRACTION = 0.0002; // ...and only this fraction of the channel values of a draw may differ at all

/** The channel values of two RGBA images that differ, as premultiplied colour and alpha (the values drawn): how many,
 *  the largest difference in levels, and the first pixel. */
function differences(expected, actual, width) {
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
      if (difference > 0) {
        count += 1;
        largest = Math.max(largest, difference);
        first ??= { x: (at / 4) % width, y: Math.floor(at / 4 / width), channel, expected: value, actual: got[channel] };
      }
    });
  }
  return { count, largest, first };
}

/** Assert that two RGBA images look the same, naming the count and the first differing pixel if they do not. */
export function assertLookTheSame(expected, actual, width, label) {
  assert.equal(actual.length, expected.length, `${label}: the length of the pixels`);
  const { count, largest, first } = differences(expected, actual, width);
  const allowed = Math.floor(expected.length * MAX_FRACTION);
  assert.ok(
    count <= allowed && largest <= MAX_LEVEL,
    `${label}: ${count} of ${expected.length} channel values differ (${allowed} allowed), by up to ${largest}; ` +
      `first at ${JSON.stringify(first)}`,
  );
}
