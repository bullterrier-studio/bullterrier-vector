// The playground's slider logic (playground/params.mjs) against every preset of the real WebAssembly package: each Param
// gives a usable slider, each source value lands inside the Param's range, a cyclic value wraps, and the drawing renders
// something at the ends of every slider. Run through `uv run poe wasm-test`.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";
import { SOURCES, sliderSpec, valueFrom } from "../../playground/params.mjs";

const require = createRequire(import.meta.url);
const { Drawing } = require("../../pkg/bullterrier_vector_wasm.js");

const WIDTH = 240; // small: the test draws every preset at three values of every slider
// A drawing with less than this share of pixels unlike the corner pixel is a blank sheet; a real one has far more.
const BLANK_SHARE = 0.005;

function inkShare(rgba) {
  let different = 0;
  for (let i = 4; i < rgba.length; i += 4) {
    if (rgba[i] !== rgba[0] || rgba[i + 1] !== rgba[1] || rgba[i + 2] !== rgba[2] || rgba[i + 3] !== rgba[3]) different++;
  }
  return different / (rgba.length / 4);
}

const presets = Drawing.presetNames();
const SOURCE_FRACTIONS = [-0.5, 0, 0.25, 0.5, 1, 1.5, 3.25];

test("the package has presets to check", () => {
  assert.ok(presets.length >= 13, `${presets.length} presets`);
});

for (const name of presets) {
  const drawing = Drawing.preset(name);
  const [designWidth, designHeight] = drawing.size;
  const height = Math.round((WIDTH * designHeight) / designWidth);

  for (const param of drawing.parameters()) {
    const spec = sliderSpec(param);

    test(`${name}.${param.name}: the slider spec is usable`, () => {
      assert.ok(spec.min < spec.max, JSON.stringify(spec));
      assert.ok(spec.step > 0);
      assert.ok(spec.default >= spec.min && spec.default <= spec.max);
      assert.equal(spec.cyclic, param.unit === "cycle");
    });

    test(`${name}.${param.name}: every source value lands inside the range, a cyclic one wraps`, () => {
      for (const fraction of SOURCE_FRACTIONS) {
        const value = valueFrom(param, fraction);
        assert.ok(value >= spec.min && value <= spec.max, `${fraction} gave ${value}`);
      }
      assert.equal(valueFrom(param, 0), spec.min);
      if (spec.cyclic) {
        assert.equal(valueFrom(param, 1.25), valueFrom(param, 0.25));
        assert.equal(valueFrom(param, -0.75), valueFrom(param, 0.25));
      } else {
        assert.equal(valueFrom(param, 1), spec.max);
        assert.equal(valueFrom(param, 7), spec.max, "a bounded Param clamps");
        assert.equal(valueFrom(param, -7), spec.min, "a bounded Param clamps");
      }
    });

    test(`${name}.${param.name}: the drawing is not blank at the slider's ends and default`, () => {
      for (const value of [spec.min, spec.default, spec.max]) {
        const share = inkShare(drawing.toRgba(WIDTH, height, { [param.name]: value }));
        assert.ok(share >= BLANK_SHARE, `${name} ${param.name}=${value}: ${(share * 100).toFixed(3)}% of pixels drawn`);
      }
    });
  }
}

test("the sources are read from the page's state into 0..1", () => {
  const state = { now: 0, pointerX: 0.25, pointerY: 0.75, scroll: 0.5 };
  const read = (id, at = state) => SOURCES.find((source) => source.id === id).read(at);
  assert.deepEqual(SOURCES.map((source) => source.id), ["manual", "clock", "pointerX", "pointerY", "scroll"]);
  assert.equal(read("pointerX"), 0.25);
  assert.equal(read("pointerY"), 0.75);
  assert.equal(read("scroll"), 0.5);
  assert.equal(read("clock"), 0);
  const readings = Array.from({ length: 600 }, (_, tick) => read("clock", { ...state, now: tick * 100 }));
  assert.ok(readings.every((value) => value >= 0 && value < 1), "the clock stays in 0..1");
  assert.ok(Math.min(...readings) < 0.05 && Math.max(...readings) > 0.95, "the clock sweeps the whole range");
  assert.ok(readings.some((value, tick) => tick > 0 && value < readings[tick - 1]), "and then goes round again");
});

// Known-bad rows: a Param that gives no slider is refused with a message saying which and why.
test("a Param with min equal to max is refused naming the Param", () => {
  const flat = { name: "flat", default: 3, min: 3, max: 3, unit: "px", meaning: "nothing" };
  assert.throws(() => sliderSpec(flat), /flat.*min.*max/);
  assert.throws(() => valueFrom(flat, 0.5), /flat.*min.*max/);
});

test("a Param whose default lies outside its range is refused", () => {
  const stray = { name: "stray", default: 9, min: 0, max: 1, unit: "", meaning: "" };
  assert.throws(() => sliderSpec(stray), /stray.*default/);
});
