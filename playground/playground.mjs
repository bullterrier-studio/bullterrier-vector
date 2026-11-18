// The page: a drawing (a preset or a document) drawn on a canvas by the WebAssembly package, one slider per Param, each
// slider driven by hand or by a source (clock, pointer, scroll). The slider maths is in params.mjs. Everything the library
// or a document says (names, descriptions, meanings, errors) goes into the page as text, never as markup.
import init, { Drawing } from "./pkg/bullterrier_vector_wasm.js";
import { SOURCES, sliderSpec, valueFrom } from "./params.mjs";

// Pixels above twice the CSS size cost four times the work for a difference the eye does not see on today's screens.
const MAX_DEVICE_PIXEL_RATIO = 2;
// A document without a design size is drawn at this shape (the preset shape).
const FALLBACK_SIZE = [1080, 560];

const $ = (id) => document.getElementById(id);
const make = (tag, text, className) => {
  const element = document.createElement(tag);
  if (text !== undefined) element.textContent = text;
  if (className) element.className = className;
  return element;
};
const view = $("view");
const context = view.getContext("2d");
const started = performance.now();

let drawing; // the Drawing on show
let rows = []; // one per Param: { param, spec, slider, source, output }
const state = { now: 0, pointerX: 0.5, pointerY: 0.5, scroll: 0 };

function oneLine(error) {
  return `${error.name}: ${String(error.message).split("\n")[0]}`;
}

function buildRows() {
  const holder = $("params");
  holder.replaceChildren();
  rows = drawing.parameters().map((param) => {
    const spec = sliderSpec(param);
    const box = make("div", undefined, "param");
    const slider = Object.assign(document.createElement("input"), {
      type: "range", min: spec.min, max: spec.max, step: spec.step, value: spec.default, id: `slider-${param.name}`,
    });
    const head = make("div", undefined, "head");
    const label = make("label", param.name);
    label.htmlFor = slider.id;
    label.append(" ", make("small", param.unit));
    const output = make("output");
    output.htmlFor = slider.id;
    head.append(label, output);
    const meaning = make("p", param.meaning, "meaning");
    const source = Object.assign(document.createElement("select"), { ariaLabel: `Source of ${param.name}` });
    for (const { id, label: text } of SOURCES) source.append(Object.assign(make("option", text), { value: id }));
    const row = make("div", undefined, "row");
    row.append(slider, source);
    box.title = param.meaning;
    box.append(head, meaning, row);
    holder.append(box);
    const entry = { param, spec, slider, source, output };
    slider.addEventListener("input", ask);
    source.addEventListener("change", ask);
    return entry;
  });
}

// The value of each Param as this frame has it: the slider's own, or what its source reads.
function currentValues() {
  const values = {};
  for (const row of rows) {
    const source = SOURCES.find((candidate) => candidate.id === row.source.value);
    row.slider.disabled = source.id !== "manual";
    if (source.id !== "manual") row.slider.value = valueFrom(row.param, source.read(state));
    values[row.param.name] = Number(row.slider.value);
    row.output.value = values[row.param.name].toFixed(3).replace(/\.?0+$/, "");
    row.output.dataset.value = String(values[row.param.name]);
  }
  return values;
}

function render() {
  const [designWidth, designHeight] = drawing.size ?? FALLBACK_SIZE;
  const cssWidth = Number($("size").value);
  view.style.width = `${cssWidth}px`;
  view.style.aspectRatio = `${designWidth} / ${designHeight}`;
  const scale = Math.min(window.devicePixelRatio || 1, MAX_DEVICE_PIXEL_RATIO);
  const width = Math.round(view.clientWidth * scale);
  const height = Math.round((width * designHeight) / designWidth);
  const values = currentValues();
  const before = performance.now();
  try {
    const rgba = drawing.toRgba(width, height, values);
    const rendered = performance.now();
    view.width = width;
    view.height = height;
    context.putImageData(new ImageData(new Uint8ClampedArray(rgba.buffer, rgba.byteOffset, rgba.length), width, height), 0, 0);
    const drawn = performance.now();
    $("time").replaceChildren(make("b", `${(rendered - before).toFixed(0)} ms`), ` render, ${(drawn - rendered).toFixed(0)} ms draw`,
      document.createElement("br"), `${width} x ${height} px`);
    $("error").textContent = "";
  } catch (error) {
    $("error").textContent = oneLine(error);
  }
  document.body.dataset.renders = String(Number(document.body.dataset.renders ?? 0) + 1);
}

// Moves between two frames are dropped: only the state of the frame renders; a clock keeps the frames coming.
let scheduled = false;
function ask() {
  if (scheduled) return;
  scheduled = true;
  requestAnimationFrame((now) => {
    scheduled = false;
    state.now = now;
    render();
    if (rows.some((row) => row.source.value === "clock")) ask();
  });
}

function show(next, description) {
  drawing = next;
  $("description").textContent = description;
  buildRows();
  ask();
}

function openDocument(text, name) {
  try {
    show(Drawing.fromJson(text), "");
    $("document-error").textContent = "";
    const option = $("document-option");
    option.textContent = `document: ${name}`;
    option.hidden = false;
    $("preset").value = "";
  } catch (error) {
    $("document-error").textContent = oneLine(error);
  }
}

await init();
for (const name of Drawing.presetNames()) $("preset").append(Object.assign(make("option", name), { value: name }));
$("preset").addEventListener("change", () => {
  if ($("preset").value) show(Drawing.preset($("preset").value), Drawing.presetDescription($("preset").value));
});
$("size").addEventListener("change", ask);
$("open").addEventListener("click", () => openDocument($("paste").value, "pasted"));
$("file").addEventListener("change", async () => {
  const file = $("file").files[0];
  if (file) openDocument(await file.text(), file.name);
});
addEventListener("pointermove", (event) => {
  state.pointerX = event.clientX / innerWidth;
  state.pointerY = event.clientY / innerHeight;
  if (rows.some((row) => row.source.value.startsWith("pointer"))) ask();
});
addEventListener("scroll", () => {
  const room = document.documentElement.scrollHeight - innerHeight;
  state.scroll = room > 0 ? scrollY / room : 0;
  if (rows.some((row) => row.source.value === "scroll")) ask();
});
addEventListener("resize", ask);

const first = Drawing.presetNames()[0];
$("preset").value = first;
show(Drawing.preset(first), Drawing.presetDescription(first));
await new Promise(requestAnimationFrame);
await new Promise(requestAnimationFrame);
document.body.dataset.firstRender = (performance.now() - started).toFixed(0);
document.body.dataset.ready = "true";
