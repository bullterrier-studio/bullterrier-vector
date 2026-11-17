// Trace in Node: the drawing of examples/trace.py, loaded from trace.json (the document its `Drawing.to_json()` writes;
// tests/test_examples.py fails if the two differ). The document names two maps; the program builds their bytes from whole
// numbers, as the Python program does.
//
//   node crates/wasm/examples/trace.mjs         prints the size of the SVG
//   node crates/wasm/examples/trace.mjs still   writes the SVG to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 960;
const HEIGHT = 1180;
const SIDE = 96;
const COLUMNS = 80;
const ROWS = 12;
const BAYER = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

const relief = new Uint8Array(SIDE * SIDE).map((_, i) => {
  const [x, y] = [i % SIDE, Math.floor(i / SIDE)];
  const hill = (cx, cy, top, k) => top - Math.floor(((x - cx) ** 2 + (y - cy) ** 2) / k);
  return Math.max(0, hill(34, 40, 255, 6), hill(68, 56, 200, 5));
});
const dither = new Uint8Array(COLUMNS * 2 * ROWS * 2).map((_, i) => {
  const [x, y] = [Math.floor((i % (COLUMNS * 2)) / 2), Math.floor(i / (COLUMNS * 2) / 2)];
  return BAYER[y % 4][x % 4] * 16 + 8 < Math.floor((x * 255) / (COLUMNS - 1)) ? 1 : 0;
});

const drawing = Drawing.fromJson(readFileSync(new URL("trace.json", import.meta.url), "utf-8"), { relief, dither });
const svg = () => drawing.toSvg(WIDTH, HEIGHT);

const name = process.argv[2];
if (name === undefined) {
  console.log(`still: svg ${Buffer.byteLength(svg())} bytes`);
} else if (name === "still") {
  process.stdout.write(svg());
} else {
  console.error("expected still, got " + name);
  process.exitCode = 1;
}
