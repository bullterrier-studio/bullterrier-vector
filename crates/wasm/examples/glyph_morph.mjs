// Glyph morph in Node: the drawing of examples/glyph_morph.py, loaded from glyph_morph.json (the document its
// `Drawing.to_json()` writes; tests/test_examples.py fails if the two differ). The font is a bundled one, so the
// document loads with no bytes.
//
//   node crates/wasm/examples/glyph_morph.mjs         prints the size of the SVG of each frame
//   node crates/wasm/examples/glyph_morph.mjs half    writes the SVG of the frame "half" to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 1080;
const HEIGHT = 560;
// the frames as progress: the same three as the Python and Rust programs
const FRAMES = { start: 0, half: 0.5, end: 1 };

const drawing = Drawing.fromJson(readFileSync(new URL("glyph_morph.json", import.meta.url), "utf-8"));
const svg = (progress) => drawing.toSvg(WIDTH, HEIGHT, { progress });

const name = process.argv[2];
if (name === undefined) {
  for (const [frame, progress] of Object.entries(FRAMES)) console.log(`${frame}: svg ${Buffer.byteLength(svg(progress))} bytes`);
} else if (name in FRAMES) {
  process.stdout.write(svg(FRAMES[name]));
} else {
  console.error(`expected start, half or end, got ${name}`);
  process.exitCode = 1;
}
