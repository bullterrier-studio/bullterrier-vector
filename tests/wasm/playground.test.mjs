// The playground's slider logic (playground/params.mjs) against every preset of the real WebAssembly package: each Param
// gives a usable slider, each source value lands inside the Param's range, a cyclic value wraps, and the drawing renders
// something at the ends of every slider. Run through `uv run poe wasm-test`.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { test } from "node:test";
import { FRAME, SOURCES, sliderSpec, valueFrom } from "../../playground/params.mjs";

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
// A slider with fewer positions than this cannot move a drawing smoothly; with more, a step is below what a pointer drag or
// a finger can aim at and the number tells nothing. Both are the bounds of one thousand, a decade either way.
const MIN_POSITIONS = 100;
const MAX_POSITIONS = 100000;
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
      const positions = (spec.max - spec.min) / spec.step;
      assert.ok(positions >= MIN_POSITIONS && positions <= MAX_POSITIONS, `${positions} slider positions`);
      assert.equal(spec.min, param.min);
      assert.equal(spec.max, param.max);
      assert.equal(spec.default, param.default);
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
  // asymmetric on purpose: no reading may swap an axis or flip a value (1 - x) and still agree
  const state = { now: 0, pointerX: 0.2, pointerY: 0.7, scroll: 0.35 };
  const read = (id, at = state) => SOURCES.find((source) => source.id === id).read(at);
  assert.deepEqual(SOURCES.map((source) => source.id), ["manual", "clock", "pointerX", "pointerY", "scroll"]);
  assert.equal(read("pointerX"), 0.2);
  assert.equal(read("pointerY"), 0.7);
  assert.equal(read("scroll"), 0.35);
  assert.equal(read("clock"), 0);
  const readings = Array.from({ length: 600 }, (_, tick) => read("clock", { ...state, now: tick * 100 }));
  assert.ok(readings.every((value) => value >= 0 && value < 1), "the clock stays in 0..1");
  assert.ok(Math.min(...readings) < 0.05 && Math.max(...readings) > 0.95, "the clock sweeps the whole range");
  assert.ok(readings.some((value, tick) => tick > 0 && value < readings[tick - 1]), "and then goes round again");
});

test("each source says what wakes it", () => {
  const wakes = Object.fromEntries(SOURCES.map((source) => [source.id, source.wakes]));
  assert.deepEqual(wakes, { manual: undefined, clock: FRAME, pointerX: "pointermove", pointerY: "pointermove", scroll: "scroll" });
});

// Known-good rows, written out by hand: a Param in deg or cycle wraps, one in px does not.
const BY_HAND = [
  { unit: "deg", min: 0, max: 360, cyclic: true, at: [[0.25, 90], [1.25, 90], [-0.75, 90]] },
  { unit: "cycle", min: 0, max: 1, cyclic: true, at: [[0.25, 0.25], [1.25, 0.25], [-0.75, 0.25]] },
  { unit: "px", min: 10, max: 30, cyclic: false, at: [[0.2, 14], [0.7, 24], [1.25, 30], [-0.75, 10]] },
];
for (const { unit, min, max, cyclic, at } of BY_HAND) {
  test(`a Param in ${unit} (${min} to ${max}) is ${cyclic ? "" : "not "}cyclic`, () => {
    const param = { name: "p", default: min, min, max, unit, meaning: "" };
    assert.equal(sliderSpec(param).cyclic, cyclic);
    for (const [fraction, expected] of at) assert.ok(Math.abs(valueFrom(param, fraction) - expected) < 1e-9, `${fraction} gave ${valueFrom(param, fraction)}`);
  });
}

// Known-bad rows: a Param that gives no slider is refused with a message saying which and why.
test("a Param with min equal to max is refused naming the Param", () => {
  const flat = { name: "flat", default: 3, min: 3, max: 3, unit: "px", meaning: "nothing" };
  assert.throws(() => sliderSpec(flat), /flat.*min.*max/);
  assert.throws(() => valueFrom(flat, 0.5), /flat.*min.*max/);
});
