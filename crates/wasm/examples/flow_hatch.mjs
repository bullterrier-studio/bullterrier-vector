// Flow hatch in Node: the drawing of examples/flow_hatch.py, loaded from flow_hatch.json (the document its
// `Drawing.to_json()` writes; tests/test_examples.py fails if the two differ). The font is a bundled one, so the
// document loads with no bytes.
//
//   node crates/wasm/examples/flow_hatch.mjs         prints the size of the SVG of the frame
//   node crates/wasm/examples/flow_hatch.mjs still   writes the SVG of the frame "still" to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 1080;
const HEIGHT = 560;

const drawing = Drawing.fromJson(readFileSync(new URL("flow_hatch.json", import.meta.url), "utf-8"));
const svg = () => drawing.toSvg(WIDTH, HEIGHT);

const name = process.argv[2];
if (name === undefined) {
  console.log(`still: svg ${Buffer.byteLength(svg())} bytes`);
} else if (name === "still") {
  process.stdout.write(svg());
} else {
  console.error(`expected still, got ${name}`);
  process.exitCode = 1;
}
