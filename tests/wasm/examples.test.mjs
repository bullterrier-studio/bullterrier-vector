// The frames of the examples drawn from Python, from the Rust example (crates/core/examples/<name>.rs) and from the Node
// example (crates/wasm/examples/<name>.mjs, run here as a person runs it): the SVG of each frame looks the same in
// all three (compare.mjs). oracle.py runs the Python and Rust sides.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { assertSvgLooksTheSame } from "./compare.mjs";

const native = JSON.parse(readFileSync(new URL("../../pkg/oracle.json", import.meta.url), "utf-8"));
// each example with the frames it names, in the words its Node program prints when it refuses another
const EXAMPLES = {
  bent_type: "calm, half or bent",
  eaten_letters: "still or fine",
  flow_hatch: "still or light",
  glyph_morph: "start, half or end",
  halftone: "still",
  kinetic_type: "hairlines, kick, bend or morph",
  line_families: "still or plot",
  overlay: "still",
  threads: "grown or half",
  trace: "still",
};

for (const [name, frames] of Object.entries(EXAMPLES)) {
  const found = native[name];
  const example = new URL(`../../crates/wasm/examples/${name}.mjs`, import.meta.url).pathname;
  const run = (frame) => execFileSync(process.execPath, [example, frame], { maxBuffer: 1 << 25 }).toString("utf-8");

  for (const frame of Object.keys(found.python)) {
    test(`${name}: the SVG of the frame ${frame} looks the same from Python, Rust and Node`, () => {
      assertSvgLooksTheSame(found.python[frame], found.rust[frame], `${name} ${frame}: Rust against Python`);
      assertSvgLooksTheSame(found.python[frame], run(frame), `${name} ${frame}: Node against Python`);
    });
  }

  test(`${name}: the Node example refuses an unknown frame with a message and exit 1`, () => {
    assert.throws(
      () => execFileSync(process.execPath, [example, "gif"], { stdio: "pipe" }),
      (error) => error.status === 1 && error.stderr.toString().includes(`expected ${frames}, got gif`),
    );
  });
}
