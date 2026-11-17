// Flow hatch in Node: the drawing of examples/flow_hatch.py, loaded from flow_hatch.json (the document its
// `Drawing.to_json()` writes; tests/test_examples.py fails if the two differ). The font is a bundled one, so the
// document loads with no bytes.
//
//   node crates/wasm/examples/flow_hatch.mjs         prints the size of the SVG of each frame
//   node crates/wasm/examples/flow_hatch.mjs still   writes the SVG of the frame "still" to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 1080;
const HEIGHT = 560;

const drawing = Drawing.fromJson(readFileSync(new URL("flow_hatch.json", import.meta.url), "utf-8"));
// the frames: the headline at its weight, and at 300
const FRAMES = { still: {}, light: { weight: 300 } };
const svg = (name) => drawing.toSvg(WIDTH, HEIGHT, FRAMES[name]);

const name = process.argv[2];
if (name === undefined) {
  for (const frame of Object.keys(FRAMES)) console.log(`${frame}: svg ${Buffer.byteLength(svg(frame))} bytes`);
} else if (name in FRAMES) {
  process.stdout.write(svg(name));
} else {
  console.error("expected still or light, got " + name);
  process.exitCode = 1;
}
