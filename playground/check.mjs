// Opens the playground in headless Chrome (the DevTools protocol over Node's own WebSocket, no packages) and checks it:
// every preset drawn with every slider at its ends, the sources moving a Param and redrawing, a bad document refused in one
// line. Prints render times and sizes, writes a screenshot, exits 1 naming the first failure.
//
//   node playground/check.mjs [screenshot.png]     after `uv run poe playground`; CHROME=/path/to/chrome for another browser
import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { extname, join, normalize } from "node:path";

const HERE = new URL(".", import.meta.url).pathname;
const CHROME = process.env.CHROME ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const SCREENSHOT = process.argv[2] ?? join(HERE, "screenshot.png");
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".wasm": "application/wasm" };
// A drawing with less than this share of pixels unlike the corner pixel is a blank sheet (as tests/wasm/playground.test.mjs).
const BLANK_SHARE = 0.005;
const WAIT_MS = 10000; // the longest the page gets to draw again or become ready
const WINDOW = { width: 1200, height: 900 }; // wide enough for the two-column layout and tall enough to scroll
const SIZES = [360, 720, 1080]; // the page's size select, in CSS pixels
const SHOT_PRESET = "waves"; // the drawing the screenshot shows
const TIMED_MOVES = 5; // slider moves timed per size

const server = createServer(async (request, response) => {
  try {
    const requested = normalize(decodeURIComponent(request.url.split("?")[0]));
    const path = requested === "/" ? "/index.html" : requested;
    const body = await readFile(join(HERE, path));
    response.writeHead(200, { "content-type": TYPES[extname(path)] ?? "application/octet-stream" }).end(body);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const profile = await mkdtemp(join(tmpdir(), "bt-playground-"));
const chrome = spawn(CHROME, ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--no-first-run", `--window-size=${WINDOW.width},${WINDOW.height}`, "about:blank"], { stdio: ["ignore", "ignore", "pipe"] });
const chromeGone = new Promise((resolve) => chrome.once("exit", resolve));

class Failure extends Error {}
const check = (condition, what) => {
  if (!condition) throw new Failure(what);
};
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const median = (numbers) => [...numbers].sort((a, b) => a - b)[Math.floor(numbers.length / 2)];

let socket;
try {
  let endpoint = "";
  for await (const chunk of chrome.stderr) {
    endpoint += chunk;
    const found = /ws:\/\/\S+/.exec(endpoint);
    if (found) { endpoint = found[0]; break; }
  }
  socket = new WebSocket(endpoint);
  await new Promise((resolve) => (socket.onopen = resolve));
  let next = 0;
  const waiting = new Map();
  const thrown = []; // uncaught exceptions of the page
  socket.onmessage = ({ data }) => {
    const message = JSON.parse(data);
    if (message.method === "Runtime.exceptionThrown") thrown.push(message.params.exceptionDetails.exception?.description ?? message.params.exceptionDetails.text);
    if (message.method === "Log.entryAdded" && message.params.entry.level === "error") thrown.push(`${message.params.entry.text} ${message.params.entry.url ?? ""}`);
    waiting.get(message.id)?.(message);
  };
  const send = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const id = ++next;
    waiting.set(id, (message) => (message.error ? reject(new Error(JSON.stringify(message.error))) : resolve(message.result)));
    socket.send(JSON.stringify({ id, method, params, sessionId }));
  });
  const { targetId } = await send("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await send("Target.attachToTarget", { targetId, flatten: true });
  const page = (method, params) => send(method, params, sessionId);
  const evaluate = async (expression) => {
    const { result, exceptionDetails } = await page("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (exceptionDetails) throw new Error(exceptionDetails.exception?.description ?? exceptionDetails.text);
    return result.value;
  };
  const until = async (expression, what) => {
    for (let waited = 0; !(await evaluate(expression)); waited += 25) {
      check(waited < WAIT_MS, `timed out waiting for ${what}${thrown.length ? `; the page threw: ${thrown[0]}` : ""}`);
      await pause(25);
    }
  };
  const renders = () => evaluate("Number(document.body.dataset.renders ?? 0)");
  // run `action` in the page, then wait for the page to draw again
  const redrawn = async (what, action) => {
    const before = await renders();
    await evaluate(action);
    await until(`Number(document.body.dataset.renders) > ${before}`, `a redraw after ${what}`);
  };
  const inkShare = () => evaluate(`(() => {
    const { width, height } = view; const { data } = view.getContext("2d").getImageData(0, 0, width, height);
    let different = 0;
    for (let i = 4; i < data.length; i += 4) if (data[i] !== data[0] || data[i + 1] !== data[1] || data[i + 2] !== data[2] || data[i + 3] !== data[3]) different++;
    return different / (data.length / 4);
  })()`);
  const sound = async (what) => {
    const share = await inkShare();
    check(share >= BLANK_SHARE, `${what}: the canvas is blank (${(share * 100).toFixed(3)}% drawn)`);
    const error = await evaluate("document.getElementById('error').textContent");
    check(error === "", `${what}: the page shows an error: ${error}`);
    check(thrown.length === 0, `${what}: the page threw: ${thrown[0]}`);
  };
  const renderMs = async () => Number((await evaluate("document.querySelector('#time b').textContent")).replace(" ms", ""));
  const setSlider = (index, value) => `(() => { const s = document.querySelectorAll("#params input[type=range]")[${index}]; s.value = ${value}; s.dispatchEvent(new Event("input")); })()`;
  const setSelect = (id, value) => `(() => { const s = document.getElementById("${id}"); s.value = ${JSON.stringify(value)}; s.dispatchEvent(new Event("change")); })()`;
  const sourceOf = (index, source) => `(() => { const s = document.querySelectorAll("#params select")[${index}]; s.value = "${source}"; s.dispatchEvent(new Event("change")); })()`;
  const valueOf = (index) => evaluate(`Number(document.querySelectorAll("#params output")[${index}].dataset.value)`);
  const mouseTo = (x, y) => page("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });

  await page("Runtime.enable");
  await page("Log.enable");
  await page("Page.enable");
  await page("Page.bringToFront"); // a background tab gets no animation frames
  await page("Emulation.setDeviceMetricsOverride", { ...WINDOW, deviceScaleFactor: 1, mobile: false });
  await page("Page.navigate", { url: `http://127.0.0.1:${server.address().port}/` });
  await until("document.body.dataset.ready === 'true'", "the page to become ready");
  const wasmBytes = (await stat(join(HERE, "pkg", "bullterrier_vector_wasm_bg.wasm"))).size;
  console.log("browser:", await evaluate("navigator.userAgent"));
  console.log(`first render: ${await evaluate("document.body.dataset.firstRender")} ms from page start (module load included); .wasm ${(wasmBytes / 1e6).toFixed(2)} MB`);
  await sound("first render");

  // every preset: every slider at its ends
  const presets = await evaluate(`[...document.querySelectorAll("#preset option")].map((o) => o.value).filter(Boolean)`);
  check(presets.length > 0, "the picker lists no presets");
  console.log("preset: median render ms over the slider ends");
  for (const preset of presets) {
    await redrawn(`choosing ${preset}`, setSelect("preset", preset));
    await sound(`${preset} chosen`);
    const description = await evaluate("document.getElementById('description').textContent");
    check(description.length > 0, `${preset}: no description shown`);
    const sliders = await evaluate(`document.querySelectorAll("#params input[type=range]").length`);
    check(sliders > 0, `${preset}: no sliders`);
    const times = [];
    for (let index = 0; index < sliders; index++) {
      for (const end of ["min", "max"]) {
        const value = await evaluate(`document.querySelectorAll("#params input[type=range]")[${index}].${end}`);
        await redrawn(`${preset} slider ${index} to its ${end}`, setSlider(index, value));
        await sound(`${preset} slider ${index} at its ${end}`);
        times.push(await renderMs());
      }
    }
    console.log(`  ${preset}: ${median(times)}`);
  }

  // the sources: each moves the bound Param and draws again
  await redrawn("choosing the preset for the sources", setSelect("preset", SHOT_PRESET));
  const pointerMoves = { pointerX: [[100, 450], [1100, 450]], pointerY: [[600, 100], [600, 800]] };
  for (const [source, [from, to]] of Object.entries(pointerMoves)) {
    await redrawn(`binding ${source}`, sourceOf(0, source));
    const at = async ([x, y]) => {
      const before = await renders();
      await mouseTo(x, y);
      await until(`Number(document.body.dataset.renders) > ${before}`, `a redraw after a ${source} move`);
      return valueOf(0);
    };
    const first = await at(from);
    const second = await at(to);
    check(first !== second, `${source}: the Param did not change (${first} then ${second})`);
    await sound(`${source} moved`);
  }
  await redrawn("binding scroll", sourceOf(0, "scroll"));
  const atTop = await valueOf(0);
  const beforeScroll = await renders();
  await evaluate("window.scrollTo(0, document.documentElement.scrollHeight)");
  await until(`Number(document.body.dataset.renders) > ${beforeScroll}`, "a redraw after a scroll");
  check((await valueOf(0)) !== atTop, `scroll: the Param did not change (${atTop})`);
  await sound("scroll moved");
  await evaluate("window.scrollTo(0, 0)");
  await redrawn("binding clock", sourceOf(0, "clock"));
  const tick = await valueOf(0);
  await pause(300);
  check((await valueOf(0)) !== tick, "clock: the Param did not change in 300 ms");
  await sound("clock running");
  await evaluate(sourceOf(0, "manual"));

  // documents
  await evaluate(`document.querySelector("details").open = true`);
  const open = async (text) => {
    await evaluate(`(() => { document.getElementById("paste").value = ${JSON.stringify(text)}; document.getElementById("open").click(); })()`);
    return evaluate("document.getElementById('document-error').textContent");
  };
  const bad = await open("{ not json");
  check(bad.length > 0 && !bad.includes("\n"), `a bad document must give a one-line error, got ${JSON.stringify(bad)}`);
  const needsInput = await open(await readFile(join(HERE, "..", "crates", "wasm", "examples", "kinetic_type.json"), "utf-8"));
  check(/inputs\./.test(needsInput) && !needsInput.includes("\n"), `a document naming an input must say which, got ${JSON.stringify(needsInput)}`);
  await sound("after refused documents");
  const good = await open(await readFile(join(HERE, "..", "crates", "wasm", "examples", "halftone.json"), "utf-8"));
  check(good === "", `a good document was refused: ${good}`);
  await until("document.getElementById('preset').selectedOptions[0].textContent.startsWith('document')", "the document to show in the picker");
  await sound("a document opened");
  console.log("documents: bad JSON and a document needing an input are refused in one line; a good one opens");

  // render time by size (CSS pixels, device pixel ratio 1), on one drawing
  await redrawn("back to a preset", setSelect("preset", SHOT_PRESET));
  const [low, high] = await evaluate(`(() => { const s = document.querySelector("#params input[type=range]"); return [Number(s.min), Number(s.max)]; })()`);
  const bySize = [];
  for (const size of SIZES) {
    await redrawn(`size ${size}`, setSelect("size", String(size)));
    const times = [];
    for (let move = 0; move < TIMED_MOVES; move++) {
      await redrawn(`move ${move} at ${size}`, setSlider(0, low + ((high - low) * move) / TIMED_MOVES));
      times.push(await renderMs());
    }
    bySize.push(`${size} px: ${median(times)} ms`);
  }
  console.log(`${SHOT_PRESET} render ms per slider move (median) by size: ${bySize.join(", ")}`);

  await redrawn("size for the screenshot", setSelect("size", String(SIZES[1])));
  await pause(200);
  const { data } = await page("Page.captureScreenshot", { format: "png" });
  await writeFile(SCREENSHOT, Buffer.from(data, "base64"));
  console.log(`wrote ${SCREENSHOT}`);
  console.log("playground check: ok");
} catch (error) {
  console.error(`playground check FAILED: ${error instanceof Failure ? error.message : error.stack}`);
  process.exitCode = 1;
} finally {
  socket?.close();
  chrome.kill();
  server.close();
  await chromeGone;
  await rm(profile, { recursive: true, force: true });
}
