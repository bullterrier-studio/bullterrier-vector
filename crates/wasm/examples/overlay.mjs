// Overlay in Node: the drawing of examples/overlay.py, loaded from overlay.json (the document its `Drawing.to_json()`
// writes; tests/test_examples.py fails if the two differ). The font is a bundled one, which the document names by its
// digest, so no bytes are supplied.
//
//   node crates/wasm/examples/overlay.mjs         prints the size of the SVG
//   node crates/wasm/examples/overlay.mjs still   writes the SVG to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const drawing = Drawing.fromJson(readFileSync(new URL("overlay.json", import.meta.url), "utf-8"));
const svg = () => drawing.toSvg(432, 360);

const name = process.argv[2];
if (name === undefined) {
  console.log(`still: svg ${Buffer.byteLength(svg())} bytes`);
} else if (name === "still") {
  process.stdout.write(svg());
} else {
  console.error("expected still, got " + name);
  process.exitCode = 1;
}
