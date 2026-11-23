// The pixel comparison of compare.mjs on known-good and known-bad images: it is what lets native and WebAssembly pixels
// differ by a rounding and nothing more, so it must accept exactly that.
import assert from "node:assert/strict";
import { test } from "node:test";
import { assertLookTheSame, assertMeshLooksTheSame, assertSvgLooksTheSame } from "./compare.mjs";

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
  looksDifferent(image, changed(image, [205], 2, 3), /1 of 40000 channel values are more than 1 level apart.*the largest by 3.*"x":5,"y":2/);
});

test("many values off by one look the same", () => {
  const image = flat(10, 20, 30, 255);
  same(image, changed(image, Array.from({ length: 5000 }, (_, at) => at), 0, 1));
});

test("many values off by two are a difference, with their count", () => {
  const image = flat(10, 20, 30, 255);
  const pixels = Array.from({ length: 100 }, (_, at) => at);
  looksDifferent(image, changed(image, pixels, 0, 2), /100 of 40000 channel values are more than 1 level apart \(40 allowed\)/);
});

test("images of different lengths are a difference", () => {
  looksDifferent(flat(1, 2, 3, 4), flat(1, 2, 3, 4).subarray(4), /the length of the pixels/);
});

test("the colour of a transparent pixel does not matter, its alpha does", () => {
  same(flat(0, 0, 0, 0), flat(255, 17, 99, 0));
  looksDifferent(flat(0, 0, 0, 0), changed(flat(0, 0, 0, 0), [3], 3, 3), /apart/);
});

test("a straight colour that differs by a level where the alpha is small looks the same", () => {
  // the straight value is the premultiplied one divided by a small alpha: 4 levels of colour at alpha 8 is 0.1 drawn
  same(flat(100, 100, 100, 8), flat(104, 100, 100, 8));
});

test("svg numbers within 0.004 look the same; further, or other text around them, do not", () => {
  const svg = '<path d="M10.5 -2 L100.25 3" stroke-width="2"/>';
  assertSvgLooksTheSame(svg, '<path d="M10.503 -2 L100.25 3.003" stroke-width="2"/>', "svg");
  assert.throws(() => assertSvgLooksTheSame(svg, '<path d="M10.51 -2 L100.25 3" stroke-width="2"/>', "svg"), /number 0 is 10.51/);
  assert.throws(() => assertSvgLooksTheSame(svg, '<path d="M10.5 -2 L100.25 3 Z" stroke-width="2"/>', "svg"), /around the numbers/);
});

test("a mesh looks the same within 0.004 on a position, but not on its item", () => {
  const mesh = (vertices) => ({ vertices: Float32Array.from(vertices), indices: Uint32Array.of(0, 1, 2), ranges: Uint32Array.of(0, 3, 0) });
  const triangle = [0, 0, 1, 0, 0, 1, 0, 0, 5, 0, 0.5, 1, 0, 0, 1, 0, 0, 5, 0, 0.5, 1, 0, 0, 1, 0, 0, 5, 0];
  const base = mesh(triangle.slice(0, 24));
  const moved = mesh(triangle.slice(0, 24).map((value, at) => (at % 8 < 2 ? value + 0.003 : value)));
  assertMeshLooksTheSame(base, moved, "mesh");
  const far = mesh(triangle.slice(0, 24).map((value, at) => (at === 0 ? value + 0.01 : value)));
  assert.throws(() => assertMeshLooksTheSame(base, far, "mesh"), /1 vertex floats are too far apart, the first at index 0/);
  const other = mesh(triangle.slice(0, 24).map((value, at) => (at === 6 ? value + 1 : value)));
  assert.throws(() => assertMeshLooksTheSame(base, other, "mesh"), /too far apart/);
  // Indices and ranges are the mesh's structure, not samples: one wrong index is a wrong triangle, however long the mesh.
  const long = { ...base, indices: new Uint32Array(3000).map((_, at) => at % 3) };
  const rewired = { ...long, indices: long.indices.map((value, at) => (at === 1500 ? 2 : value)) };
  assert.throws(() => assertMeshLooksTheSame(long, rewired, "mesh"), /1 of 3000 indices differ, the first at 1500/);
});
