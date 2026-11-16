// Bent type in Node: the drawing of examples/bent_type.py, loaded from bent_type.json (the document its
// `Drawing.to_json()` writes; tests/test_examples.py fails if the two differ). The font is a bundled one, so the
// document loads with no bytes.
//
//   node crates/wasm/examples/bent_type.mjs         prints the size of the SVG of each frame
//   node crates/wasm/examples/bent_type.mjs bent    writes the SVG of the frame "bent" to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 1080;
const HEIGHT = 560;
// the frames as { progress, phase }: the same three as the Python and Rust programs
const FRAMES = { calm: [0, 0], half: [0.5, 0.125], bent: [1, 0.25] };

const drawing = Drawing.fromJson(readFileSync(new URL("bent_type.json", import.meta.url), "utf-8"));
const svg = ([progress, phase]) => drawing.toSvg(WIDTH, HEIGHT, { progress, phase });

const name = process.argv[2];
if (name === undefined) {
  for (const [frame, values] of Object.entries(FRAMES)) console.log(`${frame}: svg ${Buffer.byteLength(svg(values))} bytes`);
} else if (name in FRAMES) {
  process.stdout.write(svg(FRAMES[name]));
} else {
  console.error(`expected calm, half or bent, got ${name}`);
  process.exitCode = 1;
}
