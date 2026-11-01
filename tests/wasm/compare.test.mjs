// The pixel comparison of compare.mjs on known-good and known-bad images: it is what lets native and WebAssembly pixels
// differ by a rounding and nothing more, so it must accept exactly that.
import assert from "node:assert/strict";
import { test } from "node:test";
import { assertLookTheSame } from "./compare.mjs";

const WIDTH = 100;
const HEIGHT = 100;
/** A 100 by 100 image of one RGBA colour. */
const flat = (r, g, b, a) => Uint8Array.from({ length: WIDTH * HEIGHT * 4 }, (_, at) => [r, g, b, a][at % 4]);
/** The image with the channel `channel` of the pixels `pixels` raised by `by`. */
function changed(image, pixels, channel, by) {
  const copy = image.slice();
  pixels.forEach((pixel) => (copy[pixel * 4 + channel] += by));
  return copy;
}
const same = (expected, actual) => assertLookTheSame(expected, actual, WIDTH, "image");
const looksDifferent = (expected, actual, message) => assert.throws(() => same(expected, actual), message);

test("equal images look the same", () => {
  same(flat(10, 20, 30, 255), flat(10, 20, 30, 255));
});

test("a rounding (one level on a few values) looks the same", () => {
  const image = flat(10, 20, 30, 255);
  same(image, changed(image, [0, 7, 99], 1, 1));
});

test("two levels on a few values look the same", () => {
  const image = flat(10, 20, 30, 255);
  same(image, changed(image, [5], 0, 2));
});

test("three levels on one value is a difference, named with the first pixel", () => {
  const image = flat(10, 20, 30, 255);
  looksDifferent(image, changed(image, [205], 2, 3), /1 of 40000 channel values differ.*by up to 3.*"x":5,"y":2/);
});

test("many values off by one is a difference, with their count", () => {
  const image = flat(10, 20, 30, 255);
  const pixels = Array.from({ length: 100 }, (_, at) => at);
  looksDifferent(image, changed(image, pixels, 0, 1), /100 of 40000 channel values differ \(8 allowed\)/);
});

test("images of different lengths are a difference", () => {
  looksDifferent(flat(1, 2, 3, 4), flat(1, 2, 3, 4).subarray(4), /the length of the pixels/);
});

test("the colour of a transparent pixel does not matter, its alpha does", () => {
  same(flat(0, 0, 0, 0), flat(255, 17, 99, 0));
  looksDifferent(flat(0, 0, 0, 0), changed(flat(0, 0, 0, 0), [3], 3, 3), /differ/);
});

test("a straight colour that differs by a level where the alpha is small looks the same", () => {
  // the straight value is the premultiplied one divided by a small alpha: 4 levels of colour at alpha 8 is 0.1 drawn
  same(flat(100, 100, 100, 8), flat(104, 100, 100, 8));
});
