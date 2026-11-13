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
      if (difference > 0) {
        count += 1;
        largest = Math.max(largest, difference);
        first ??= { x: (at / 4) % width, y: Math.floor(at / 4 / width), channel, expected: value, actual: got[channel] };
      }
    });
  }
  return { count, largest, first };
}

// Windows (docs/design-decisions.md, "Render a window (V10e)"): over the 148 draws, each with one window, the WebAssembly window
// differs from the native one in at most 2 values, 1 level each: 0.005% of the values of the whole picture but 0.03% of the
// window's own, past MAX_FRACTION, so a window's allowed count is taken on the whole picture's values, the count
// the same picture is allowed. The native window against the whole's rows and columns: at most 1 level, 218 values in
// the worst draw.

/** Assert that two RGBA images look the same, naming the count and the first differing pixel if they do not. The count
 *  allowed is a fraction of `of` channel values: the images' own, or those of the whole picture that they are a window of. */
export function assertLookTheSame(expected, actual, width, label, of = expected.length) {
  assert.equal(actual.length, expected.length, `${label}: the length of the pixels`);
  const { count, largest, first } = differences(expected, actual, width);
  const allowed = Math.floor(of * MAX_FRACTION);
  assert.ok(
    count <= allowed && largest <= MAX_LEVEL,
    `${label}: ${count} of ${expected.length} channel values differ (${allowed} allowed), by up to ${largest}; ` +
      `first at ${JSON.stringify(first)}`,
  );
}

/** The rows and columns `[x, y, width, height]` of an RGBA image `across` pixels wide. */
export function windowOf(image, across, [x, y, width, height]) {
  const rows = [];
  for (let row = y; row < y + height; row += 1) {
    rows.push(image.subarray((row * across + x) * 4, (row * across + x + width) * 4));
  }
  return Buffer.concat(rows);
}
