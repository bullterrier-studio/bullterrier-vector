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

test("the module is built for WebAssembly SIMD", () => {
  const bytes = readFileSync(new URL("../../pkg/bullterrier_vector_wasm_bg.wasm", import.meta.url));
  const [features] = WebAssembly.Module.customSections(new WebAssembly.Module(bytes), "target_features");
  assert.match(Buffer.from(features).toString("latin1"), /\+\x07simd128/);
});

test("values are optional", () => {
  const { json, draws } = native.documents.star;
  assert.equal(Drawing.fromJson(json).toSvg(400, 400), draws[0].svg);
});

/** The fonts a refusal supplies, by the name the document gives them, as the Buffers a loader gives `inputs`. */
const fontsOf = (names) =>
  Object.fromEntries(Object.entries(names).map(([name, key]) => [name, Buffer.from(native.fonts[key], "base64")]));

for (const { call, json, fonts, width, height, values, name, message } of native.refusals) {
  test(`${call} refuses with the native ${name}: ${message.slice(0, 40)}`, () => {
    const refusal = (error) => {
      assert.ok(error instanceof Error);
      assert.equal(error.name, name);
      assert.equal(error.message, message);
      return true;
    };
    if (call === "fromJson") {
      assert.throws(() => Drawing.fromJson(json, fontsOf(fonts)), refusal);
    } else {
      assert.throws(() => Drawing.fromJson(json, fontsOf(fonts))[call](width, height, values), refusal);
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

// wasm-bindgen reads a string argument without checking its type: a parsed document, the commonest slip, would trap
for (const [label, value] of [
  ["a parsed document", JSON.parse(native.documents.star.json)],
  ["a number", 5],
  ["null", null],
  ["undefined", undefined],
  ["the bytes of the text", new TextEncoder().encode(native.documents.star.json)],
]) {
  test(`fromJson given ${label} is a TypeError naming text`, () => {
    assert.throws(
      () => Drawing.fromJson(value),
      (error) => error instanceof TypeError && error.message.includes("text"),
    );
  });
}

// a document with a font in it: the loader gives the bytes by the name the document calls them
const text = native.corpus.find((entry) => entry.name === "text_param_axis");
const fraunces = Buffer.from(native.fonts.fraunces, "base64");

test("a text document loads given its font and writes itself back as it was written", () => {
  const drawing = Drawing.fromJson(text.json, { fraunces });
  assert.equal(drawing.toJson(), text.json);
  assert.deepEqual(drawing.parameters().map((parameter) => parameter.name), ["wght"]);
});

test("the bytes of the font may be any Uint8Array, a view into a larger buffer too", () => {
  const larger = new Uint8Array(fraunces.length + 100);
  larger.set(fraunces, 40);
  const view = larger.subarray(40, 40 + fraunces.length);
  const expected = Drawing.fromJson(text.json, { fraunces }).toSvg(400, 160);
  assert.equal(Drawing.fromJson(text.json, { fraunces: new Uint8Array(fraunces) }).toSvg(400, 160), expected);
  assert.equal(Drawing.fromJson(text.json, { fraunces: view }).toSvg(400, 160), expected);
});

for (const [label, inputs, named] of [
  ["a string", "fraunces", "inputs"],
  ["a font as a string", { fraunces: "bytes" }, "inputs.fraunces"],
  ["a font as an array of numbers", { fraunces: [...fraunces.subarray(0, 8)] }, "inputs.fraunces"],
  ["a font as an ArrayBuffer", { fraunces: fraunces.buffer }, "inputs.fraunces"],
  ["a font as undefined", { fraunces: undefined }, "inputs.fraunces"],
]) {
  test(`fromJson given inputs that are ${label} is a TypeError naming ${named}`, () => {
    assert.throws(
      () => Drawing.fromJson(text.json, inputs),
      (error) => error instanceof TypeError && error.message.includes(named),
    );
  });
}

test("a document without inputs loads with an empty inputs object or none", () => {
  const { json } = native.documents.star;
  assert.equal(Drawing.fromJson(json, {}).toJson(), Drawing.fromJson(json).toJson());
});
