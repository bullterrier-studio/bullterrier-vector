// The WebAssembly package against the native extension: the same documents, sizes and values must give the same SVG
// text (exactly), the same JSON, the same Param descriptions and the same errors. The expected results come from
// oracle.py, which asks the native Python extension; run both through `uv run poe wasm-test`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { test } from "node:test";

const require = createRequire(import.meta.url);
const { Drawing } = require("../../pkg/bullterrier_vector_wasm.js");
const native = JSON.parse(readFileSync(new URL("../../pkg/oracle.json", import.meta.url), "utf-8"));

for (const [name, expected] of Object.entries(native.documents)) {
  test(`${name}: toSvg writes the text the native extension writes`, () => {
    const drawing = Drawing.fromJson(expected.json);
    for (const { width, height, values, svg } of expected.draws) {
      assert.equal(drawing.toSvg(width, height, values), svg, `${width}x${height} ${JSON.stringify(values)}`);
    }
  });

  test(`${name}: toJson, size and parameters are the native ones`, () => {
    const drawing = Drawing.fromJson(expected.json);
    assert.equal(drawing.toJson(), expected.canonical);
    assert.deepEqual(drawing.size ?? null, expected.size);
    assert.deepEqual(drawing.parameters(), expected.parameters);
  });
}

test("values are optional", () => {
  const { json, draws } = native.documents.star;
  assert.equal(Drawing.fromJson(json).toSvg(400, 400), draws[0].svg);
});

for (const { call, json, width, height, values, name, message } of native.refusals) {
  test(`${call} refuses with the native ${name}: ${message.slice(0, 40)}`, () => {
    const refusal = (error) => {
      assert.ok(error instanceof Error);
      assert.equal(error.name, name);
      assert.equal(error.message, message);
      return true;
    };
    if (call === "fromJson") {
      assert.throws(() => Drawing.fromJson(json), refusal);
    } else {
      assert.throws(() => Drawing.fromJson(json).toSvg(width, height, values), refusal);
    }
  });
}

for (const [label, values, named] of [
  ["a text", { lift: "5" }, "lift"],
  ["a list holding a text", { lift: [1, "2"] }, "lift"],
  ["null", { lift: null }, "lift"],
  ["a number for the object", 5, "object"],
]) {
  test(`a value that is ${label} is a TypeError naming it`, () => {
    const drawing = Drawing.fromJson(native.documents.lifted.json);
    assert.throws(
      () => drawing.toSvg(100, 100, values),
      (error) => error instanceof TypeError && error.message.includes(named),
    );
  });
}
