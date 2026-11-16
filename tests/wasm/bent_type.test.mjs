// The bent type of examples/bent_type.py drawn from Python, from the Rust example (crates/core/examples/bent_type.rs) and
// from the Node example (crates/wasm/examples/bent_type.mjs, run here as a person runs it): the SVG of each frame is
// the same text in all three. oracle.py runs the Python and Rust sides.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const { bent_type: bent } = JSON.parse(readFileSync(new URL("../../pkg/oracle.json", import.meta.url), "utf-8"));
const example = new URL("../../crates/wasm/examples/bent_type.mjs", import.meta.url).pathname;
const run = (frame) => execFileSync(process.execPath, [example, frame], { maxBuffer: 1 << 25 }).toString("utf-8");

for (const frame of Object.keys(bent.python)) {
  test(`the SVG of the frame ${frame} is the same text from Python, Rust and Node`, () => {
    assert.ok(bent.rust[frame] === bent.python[frame], "Rust against Python");
    assert.ok(run(frame) === bent.python[frame], "Node against Python");
  });
}

test("the Node example refuses an unknown frame with a message and exit 1", () => {
  assert.throws(
    () => execFileSync(process.execPath, [example, "gif"], { stdio: "pipe" }),
    (error) => error.status === 1 && error.stderr.toString().includes("expected calm, half or bent, got gif"),
  );
});
