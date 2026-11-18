// A halftone in Node: the drawing of examples/halftone.py, loaded from halftone.json (the document its
// `Drawing.to_json()` writes; tests/test_examples.py fails if the two differ).
//
//   node crates/wasm/examples/halftone.mjs         prints the size of the SVG of the frame
//   node crates/wasm/examples/halftone.mjs still   writes the SVG to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 640;
const HEIGHT = 520;

const drawing = Drawing.fromJson(readFileSync(new URL("halftone.json", import.meta.url), "utf-8"));
const FRAMES = {
  still: () => drawing.toSvg(WIDTH, HEIGHT),
};

const name = process.argv[2];
if (name === undefined) {
  for (const frame of Object.keys(FRAMES)) console.log(`${frame}: svg ${Buffer.byteLength(FRAMES[frame]())} bytes`);
} else if (name in FRAMES) {
  process.stdout.write(FRAMES[name]());
} else {
  console.error("expected still, got " + name);
  process.exitCode = 1;
}
