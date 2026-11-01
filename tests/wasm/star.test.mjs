// Milestone M1, "a star everywhere": the star of examples/star.py drawn from Python, from the Rust example
// (crates/core/examples/star.rs) and from the Node example (crates/wasm/examples/star.mjs, run here as a person runs it).
// The SVG text is identical in all three; the pixels of the two native programs are identical bytes, and the Node
// example's pixels look the same as theirs. oracle.py runs the Python and Rust sides.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { assertLookTheSame } from "./compare.mjs";

const { star } = JSON.parse(readFileSync(new URL("../../pkg/oracle.json", import.meta.url), "utf-8"));
const read = (file) => new Uint8Array(readFileSync(new URL(`../../pkg/oracle-pixels/${file}`, import.meta.url)));
const example = new URL("../../crates/wasm/examples/star.mjs", import.meta.url).pathname;
const run = (output) => execFileSync(process.execPath, [example, output], { maxBuffer: 1 << 24 });

test("the SVG of the star is the same text from Python, Rust and Node", () => {
  assert.equal(star.rust_svg, star.python_svg, "Rust against Python");
  assert.equal(run("svg").toString("utf-8"), star.python_svg, "Node against Python");
});

test("the pixels of the star are the same bytes from Python and Rust", () => {
  assert.ok(Buffer.from(read(star.rust_rgba_file)).equals(read(star.python_rgba_file)));
});

test("the pixels of the star from Node look the same as the native ones", () => {
  const rgba = new Uint8Array(run("rgba"));
  assertLookTheSame(read(star.python_rgba_file), rgba, 400, "star Node against Python");
  assertLookTheSame(read(star.rust_rgba_file), rgba, 400, "star Node against Rust");
});

test("the PNG of the Node example has the signature and the size of the star", () => {
  const png = run("png");
  assert.deepEqual([...png.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
  assert.deepEqual([png.readUInt32BE(16), png.readUInt32BE(20)], [400, 400]);
});

test("the Node example without an argument prints the size of the SVG and the pixels", () => {
  const line = execFileSync(process.execPath, [example], { encoding: "utf-8" });
  assert.match(line, new RegExp(`^svg: ${Buffer.byteLength(star.python_svg)} bytes, rgba: 640000 bytes, checksum [0-9a-f]{8}\n$`));
});

test("the Node example refuses an unknown output with a message and exit 1", () => {
  assert.throws(
    () => execFileSync(process.execPath, [example, "gif"], { stdio: "pipe" }),
    (error) => error.status === 1 && error.stderr.toString().includes("expected svg, png or rgba, got gif"),
  );
});
