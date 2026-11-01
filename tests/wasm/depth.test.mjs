// The WebAssembly package must read, draw, save and refuse the deepest structures on the smallest stack it supports.
// Past its stack the engine throws a RangeError, which is a failure here: the deepest document `to_json` accepts is read
// and drawn by every output, and documents nested a hundred thousand levels are refused with a FormatError, in a worker
// whose stack and heap are limited (docs/design-decisions.md, "Bounded depth"). The worker's heap limit also keeps a
// runaway from taking the machine with it.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { test } from "node:test";
import { Worker, isMainThread, parentPort, workerData } from "node:worker_threads";

const SMALLEST_STACK_MB = 0.5; // below about 0.25 MB the engine throws a RangeError for even a modest document
const HEAP_LIMIT_MB = 512;

if (isMainThread) {
  const native = JSON.parse(readFileSync(new URL("../../pkg/oracle.json", import.meta.url), "utf-8"));

  test("the deepest document and hostile ones work on the smallest supported stack", async () => {
    const worker = new Worker(new URL(import.meta.url), {
      workerData: { deepest: native.deepest_document },
      resourceLimits: { stackSizeMb: SMALLEST_STACK_MB, maxOldGenerationSizeMb: HEAP_LIMIT_MB },
    });
    const steps = await new Promise((resolve, reject) => {
      worker.once("message", resolve);
      worker.once("error", reject);
    });
    assert.deepEqual(steps, ["read", "toSvg", "toRgba", "toMesh", "toJson", "parameters", "arrays", "objects", "mixed"]);
  });
} else {
  const require = createRequire(import.meta.url);
  const { Drawing } = require("../../pkg/bullterrier_vector_wasm.js");
  const steps = [];
  const drawing = Drawing.fromJson(workerData.deepest);
  steps.push("read");
  for (const [name, call] of [
    ["toSvg", () => drawing.toSvg(10, 10)],
    ["toRgba", () => drawing.toRgba(10, 10)],
    ["toMesh", () => drawing.toMesh(10, 10)],
    ["toJson", () => drawing.toJson()],
    ["parameters", () => drawing.parameters()],
  ]) {
    call();
    steps.push(name);
  }
  for (const [name, text] of [
    ["arrays", "[".repeat(100_000)],
    ["objects", '{"a":'.repeat(100_000)],
    ["mixed", '{"a":['.repeat(50_000)],
  ]) {
    assert.throws(() => Drawing.fromJson(text), { name: "FormatError" });
    steps.push(name);
  }
  parentPort.postMessage(steps);
}
