// Kinetic type in Node: the drawing of examples/kinetic_type.py, loaded from kinetic_type.json (the document its
// `piece.to_json()` writes; tests/test_examples.py fails if the two differ). The font is an input of the document: its
// bytes are read from examples/fonts and checked against the document's sha256. The easing and the beat are the
// caller's, so they are here, in plain JavaScript: `valuesAt(t)`.
//
//   node crates/wasm/examples/kinetic_type.mjs        prints the size of the SVG of each frame
//   node crates/wasm/examples/kinetic_type.mjs bend   writes the SVG of the frame "bend" to standard output
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 1080;
const HEIGHT = 1350;
const BEAT = 0.5; // seconds: 120 bpm
const CHARACTERS = 8; // "NO\nSLEEP": `pulse` has an entry for each, the line break too
// the frames, in seconds: the same four as the Python and Rust programs
const FRAMES = { hairlines: 1, kick: 4.4, bend: 6.5, morph: 8.8 };

const read = (path) => readFileSync(new URL(path, import.meta.url));
const font = new Uint8Array(read("../../../examples/fonts/anybody-subset.ttf"));
const drawing = Drawing.fromJson(read("kinetic_type.json").toString("utf-8"), { anybody: font });

// 0 up to 1 over x from 0 to 1, easing in and out
const smooth = (x) => {
  x = Math.min(1, Math.max(0, x));
  return x * x * (3 - 2 * x);
};
// 1 on a beat, falling to 0 within 0.3 s, and again on every beat (the remainder is never negative, as Python's)
const kick = (t) => {
  const rest = t % BEAT;
  const falling = Math.max(0, 1 - (rest < 0 ? rest + BEAT : rest) / 0.3);
  return falling * falling * falling;
};
// the values t seconds in (0 to 12), as the timeline in the docstring of examples/kinetic_type.py
const valuesAt = (t) => {
  const printed = 4 <= t && t < 12;
  const hitting = printed && t < 8;
  const hit = (at) => (hitting ? kick(at) : 0);
  return {
    rise: smooth((t - 2) / 2),
    ink: printed ? 1 - smooth((t - 11.5) / 0.5) : 0,
    weight: printed ? 300 - 200 * smooth((t - 10) / 1.5) : 100,
    burst: hit(t - 4),
    pulse: Array.from({ length: CHARACTERS }, (_, i) => hit(t - 4 - 0.045 * i)),
    bend: smooth(t - 5.5) * (1 - smooth(t - 7.5)),
    phase: (t * 0.25) % 1,
    turn: smooth((t - 8) / 1.5),
  };
};
const svg = (t) => drawing.toSvg(WIDTH, HEIGHT, valuesAt(t));

const name = process.argv[2];
if (name === undefined) {
  for (const [frame, t] of Object.entries(FRAMES)) console.log(`${frame}: svg ${Buffer.byteLength(svg(t))} bytes`);
} else if (name in FRAMES) {
  process.stdout.write(svg(FRAMES[name]));
} else {
  console.error(`expected hairlines, kick, bend or morph, got ${name}`);
  process.exitCode = 1;
}
