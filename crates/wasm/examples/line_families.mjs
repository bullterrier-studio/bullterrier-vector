// Line families in Node: the drawing of examples/line_families.py, loaded from line_families.json (the document its
// `Drawing.to_json()` writes; tests/test_examples.py fails if the two differ).
//
//   node crates/wasm/examples/line_families.mjs         prints the size of the SVG of each frame
//   node crates/wasm/examples/line_families.mjs plot    writes the plotter SVG to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 1080;
const HEIGHT = 420;

const drawing = Drawing.fromJson(readFileSync(new URL("line_families.json", import.meta.url), "utf-8"));
// the frames: the picture, and the SVG a pen plotter reads, with a pen 0.8 pixels wide
const FRAMES = {
  still: () => drawing.toSvg(WIDTH, HEIGHT),
  plot: () => drawing.toPlotSvg(WIDTH, HEIGHT, {}, { penWidth: 0.8 }),
};

const name = process.argv[2];
if (name === undefined) {
  for (const frame of Object.keys(FRAMES)) console.log(`${frame}: svg ${Buffer.byteLength(FRAMES[frame]())} bytes`);
} else if (name in FRAMES) {
  process.stdout.write(FRAMES[name]());
} else {
  console.error("expected still or plot, got " + name);
  process.exitCode = 1;
}
