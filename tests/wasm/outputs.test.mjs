// Pixels, PNG and the mesh of the WebAssembly package against the native extension, on the corpus of oracle.py.
//
// What we compute ourselves is exact: the SVG text, the mesh buffers (hashed here with node:crypto, there with hashlib).
// The pixels are drawn by a rasteriser crate that is required only to look the same on every target, so they are
// compared within a tolerance fixed by measurement (compare.mjs). A PNG is decoded here with
// node:zlib, a decoder that shares nothing with the encoder, and must hold the pixels of toRgba.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { test } from "node:test";
import { inflateSync } from "node:zlib";
import { assertLookTheSame } from "./compare.mjs";

const require = createRequire(import.meta.url);
const { Drawing, Mesh, wasmMemory: memory } = require("../../pkg/bullterrier_vector_wasm.js");
const native = JSON.parse(readFileSync(new URL("../../pkg/oracle.json", import.meta.url), "utf-8"));
const nativePixels = (file) => new Uint8Array(readFileSync(new URL(`../../pkg/oracle-pixels/${file}`, import.meta.url)));

/** The fonts a document names, from the names to the keys of `native.fonts`, as the Buffers a loader gives `inputs`. */
const inputsOf = (names) =>
  Object.fromEntries(Object.entries(names).map(([name, key]) => [name, Buffer.from(native.fonts[key], "base64")]));

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

/** A PNG file read back: its chunk types, its header and its pixels as RGBA bytes (unfiltered by the rules of the PNG
 *  specification, which this does not share with the encoder). */
function decodePng(bytes, label) {
  assert.deepEqual([...bytes.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10], `${label}: png signature`);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const types = [];
  const data = [];
  let header = null;
  for (let at = 8; at < bytes.length; ) {
    const length = view.getUint32(at);
    const type = Buffer.from(bytes.subarray(at + 4, at + 8)).toString("latin1");
    types.push(type);
    const body = bytes.subarray(at + 8, at + 8 + length);
    if (type === "IHDR") {
      const fields = new DataView(body.buffer, body.byteOffset, body.byteLength);
      header = {
        width: fields.getUint32(0),
        height: fields.getUint32(4),
        depth: body[8],
        colour: body[9],
        interlace: body[12],
      };
    }
    if (type === "IDAT") data.push(body);
    at += 12 + length;
  }
  const { width, height } = header;
  const raw = inflateSync(Buffer.concat(data));
  const stride = width * 4;
  assert.equal(raw.length, height * (stride + 1), `${label}: png decompressed size`);
  const pixels = new Uint8Array(height * stride);
  for (let y = 0; y < height; y += 1) {
    const filter = raw[y * (stride + 1)];
    for (let x = 0; x < stride; x += 1) {
      const value = raw[y * (stride + 1) + 1 + x];
      const left = x >= 4 ? pixels[y * stride + x - 4] : 0;
      const up = y > 0 ? pixels[(y - 1) * stride + x] : 0;
      const upLeft = x >= 4 && y > 0 ? pixels[(y - 1) * stride + x - 4] : 0;
      const estimate = left + up - upLeft;
      const paeth = [Math.abs(estimate - left), Math.abs(estimate - up), Math.abs(estimate - upLeft)];
      const nearest = paeth[0] <= paeth[1] && paeth[0] <= paeth[2] ? left : paeth[1] <= paeth[2] ? up : upLeft;
      const prediction = [0, left, up, (left + up) >> 1, nearest][filter];
      assert.notEqual(prediction, undefined, `${label}: png filter type 0 to 4, got ${filter}`);
      pixels[y * stride + x] = (value + prediction) & 255;
    }
  }
  return { types, header, pixels };
}

/** A typed array over the bytes WebAssembly holds, made fresh: the memory can grow at any call. */
const view = (Type, pointer, length) => new Type(memory().buffer, pointer, length);
const bytesOf = (typed) => new Uint8Array(typed.buffer, typed.byteOffset, typed.byteLength);

/** The buffers of a mesh as hashes and lengths, the shape oracle.py writes. */
function describe(mesh) {
  return {
    vertices_sha256: sha256(bytesOf(view(Float32Array, mesh.verticesPtr, mesh.verticesLength))),
    indices_sha256: sha256(bytesOf(view(Uint32Array, mesh.indicesPtr, mesh.indicesLength))),
    ranges_sha256: sha256(bytesOf(view(Uint32Array, mesh.rangesPtr, mesh.rangesLength))),
    vertices_length: mesh.verticesLength,
    indices_length: mesh.indicesLength,
    ranges_length: mesh.rangesLength,
    generation: mesh.generation,
  };
}

for (const { name, json, inputs, draws } of native.corpus) {
  const drawing = Drawing.fromJson(json, inputsOf(inputs));
  const each = (check) => () => draws.forEach((draw) => check(draw, `${name} ${draw.width}x${draw.height}`));

  test(`${name}: the svg text is the native one`, () => {
    assert.equal(drawing.toJson(), json, `${name}: the document reads back as it was written`);
    each((draw, label) => {
      const svg = drawing.toSvg(draw.width, draw.height, draw.values);
      assert.equal(sha256(Buffer.from(svg, "utf-8")), draw.svg_sha256, `${label}: svg`);
    })();
  });

  test(`${name}: the mesh buffers are the native ones`, () => {
    each((draw, label) => {
      if (draw.mesh.refusal) {
        // a mesh the core refuses: the same error, by name and message
        assert.throws(() => drawing.toMesh(draw.width, draw.height, draw.values), draw.mesh.refusal, `${label}: mesh refusal`);
        return;
      }
      const mesh = drawing.toMesh(draw.width, draw.height, draw.values);
      assert.deepEqual(describe(mesh), draw.mesh, `${label}: mesh`);
      mesh.free();
    })();
  });

  test(`${name}: the pixels look the native ones`, () => {
    each((draw, label) => {
      const rgba = drawing.toRgba(draw.width, draw.height, draw.values);
      assert.ok(rgba instanceof Uint8Array);
      assertLookTheSame(nativePixels(draw.rgba_file), rgba, draw.width, `${label}: pixels`);
    })();
  });

  test(`${name}: the png holds the pixels of toRgba`, () => {
    each((draw, label) => {
      const png = drawing.toPng(draw.width, draw.height, draw.values);
      const decoded = decodePng(png, label);
      assert.deepEqual(decoded.types, ["IHDR", "IDAT", "IEND"], `${label}: png chunks`);
      assert.deepEqual(
        decoded.header,
        { width: draw.width, height: draw.height, depth: 8, colour: 6, interlace: 0 },
        `${label}: png header`,
      );
      assert.ok(Buffer.from(decoded.pixels).equals(drawing.toRgba(draw.width, draw.height, draw.values)), `${label}: png pixels`);
    })();
  });
}

test("every draw of the corpus that draws something holds a mesh and pixels no other draw holds", () => {
  const seen = new Map();
  for (const { name, draws } of native.corpus) {
    for (const draw of draws) {
      if (draw.mesh.vertices_length === 0) continue;
      const label = `${name} ${draw.width}x${draw.height}`;
      const pixels = sha256(nativePixels(draw.rgba_file));
      const keys = draw.mesh.refusal ? [`pixels ${pixels}`] : [`mesh ${draw.mesh.vertices_sha256}`, `pixels ${pixels}`];
      for (const key of keys) {
        assert.equal(seen.get(key), undefined, `${label} and ${seen.get(key)} are the same draw (${key.split(" ")[0]}): a row that changes nothing tests nothing`);
        seen.set(key, label);
      }
    }
  }
});

const star = native.corpus.find((entry) => entry.name === "star");
const lifted = native.corpus.find((entry) => entry.name === "lifted_list");

test("a fresh Mesh is empty, generation 0, and a drawing with something to draw fills it", () => {
  const mesh = new Mesh();
  assert.equal(mesh.generation, 0);
  assert.equal(mesh.verticesLength + mesh.indicesLength + mesh.rangesLength, 0);
  Drawing.fromJson(star.json).toMeshInto(mesh, 400, 400);
  assert.equal(mesh.generation, 1);
  assert.deepEqual(describe(mesh), star.draws[0].mesh);
});

test("the mesh is in WebAssembly memory: its pointers are byte offsets into memory(), aligned to their element", () => {
  const mesh = Drawing.fromJson(star.json).toMesh(400, 400);
  assert.ok(memory() instanceof WebAssembly.Memory);
  for (const [pointer, length, size] of [
    [mesh.verticesPtr, mesh.verticesLength, 4],
    [mesh.indicesPtr, mesh.indicesLength, 4],
    [mesh.rangesPtr, mesh.rangesLength, 4],
  ]) {
    assert.equal(pointer % size, 0);
    assert.ok(pointer + length * size <= memory().buffer.byteLength);
  }
  assert.equal(mesh.verticesLength % 8, 0, "8 floats a vertex");
  assert.equal(mesh.indicesLength % 3, 0, "3 indices a triangle");
  assert.equal(mesh.rangesLength % 3, 0, "3 numbers a draw range");
});

test("toMeshInto refills the mesh in place: an equal refill keeps the generation and the buffers", () => {
  const drawing = Drawing.fromJson(lifted.json);
  const mesh = new Mesh();
  drawing.toMeshInto(mesh, 100, 80, { lift: [0, 40] });
  const before = { ...describe(mesh), pointer: mesh.verticesPtr };
  drawing.toMeshInto(mesh, 100, 80, { lift: [0, 40] });
  assert.deepEqual({ ...describe(mesh), pointer: mesh.verticesPtr }, before);
});

test("toMeshInto with other values changes the buffers and adds 1 to the generation, as a new mesh would hold", () => {
  const drawing = Drawing.fromJson(lifted.json);
  const mesh = new Mesh();
  drawing.toMeshInto(mesh, 100, 80, { lift: 5 });
  drawing.toMeshInto(mesh, 100, 80, { lift: [0, 40] });
  assert.equal(mesh.generation, 2);
  const fresh = drawing.toMesh(100, 80, { lift: [0, 40] });
  assert.deepEqual({ ...describe(mesh), generation: 0 }, { ...describe(fresh), generation: 0 });
  assert.deepEqual(describe(fresh), lifted.draws[0].mesh);
});

test("toMeshInto that throws leaves the mesh as it was", () => {
  const drawing = Drawing.fromJson(lifted.json);
  const mesh = new Mesh();
  drawing.toMeshInto(mesh, 100, 80, { lift: 5 });
  const before = describe(mesh);
  assert.throws(() => drawing.toMeshInto(mesh, 100, 80, { lift: 500 }), (error) => error.name === "ParameterError");
  assert.throws(() => drawing.toMeshInto(mesh, 0, 80), (error) => error.name === "ParameterError");
  assert.deepEqual(describe(mesh), before);
});

test("toMeshInto given something that is not a Mesh throws an Error naming Mesh", () => {
  const drawing = Drawing.fromJson(lifted.json);
  for (const wrong of [{}, null, undefined, 5]) {
    assert.throws(
      () => drawing.toMeshInto(wrong, 100, 80),
      (error) => error instanceof Error && error.message.includes("Mesh"),
    );
  }
});

test("a mesh at twice the size has exactly twice the positions, and the rest the same", () => {
  const drawing = Drawing.fromJson(star.json);
  const floats = (mesh) => view(Float32Array, mesh.verticesPtr, mesh.verticesLength).slice();
  const one = floats(drawing.toMesh(400, 400));
  const two = floats(drawing.toMesh(800, 800));
  assert.equal(two.length, one.length);
  for (let at = 0; at < one.length; at += 8) {
    assert.equal(two[at], 2 * one[at], `x of vertex ${at / 8}`);
    assert.equal(two[at + 1], 2 * one[at + 1], `y of vertex ${at / 8}`);
    assert.deepEqual([...two.subarray(at + 2, at + 8)], [...one.subarray(at + 2, at + 8)], `colour of vertex ${at / 8}`);
  }
});

test("a view made before the memory grew is detached; a fresh one reads the same mesh", () => {
  const drawing = Drawing.fromJson(star.json);
  const mesh = drawing.toMesh(400, 400);
  const stale = view(Float32Array, mesh.verticesPtr, mesh.verticesLength);
  const before = describe(mesh);
  // growing is what any draw that outgrows the memory does; it is done here by hand, since the draws before this test
  // leave free memory that a large draw would take without growing
  memory().grow(1);
  assert.equal(stale.length, 0, "the old view is detached once the memory grew");
  assert.deepEqual(describe(mesh), before);
});

test("free returns a mesh's memory at once; using it afterwards throws", () => {
  const mesh = new Mesh();
  mesh.free();
  assert.throws(() => mesh.generation);
});

for (const call of ["toRgba", "toPng", "toMesh"]) {
  for (const [label, values] of [
    ["a text", { lift: "5" }],
    ["a list holding a text", { lift: [1, "2"] }],
    ["null", { lift: null }],
    ["a number for the object", 5],
  ]) {
    test(`${call} given a value that is ${label} is a TypeError naming it`, () => {
      const drawing = Drawing.fromJson(lifted.json);
      assert.throws(
        () => drawing[call](100, 80, values),
        (error) => error instanceof TypeError && /lift|object/.test(error.message),
      );
    });
  }

  test(`${call} takes the values as optional`, () => {
    assert.ok(Drawing.fromJson(star.json)[call](400, 400));
  });
}
