// The WebAssembly half of tools/bench.py: reads {document, size, warmups, repeats} as JSON on stdin and the package's
// path as the argument, times toRgba and toMesh with performance.now, and prints {to_rgba, to_mesh} (median
// milliseconds) as JSON on stdout.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)(process.argv[2]);
const { document, size, warmups, repeats } = JSON.parse(readFileSync(0, "utf-8"));
const drawing = Drawing.fromJson(document);

function median(call) {
  for (let i = 0; i < warmups; i += 1) call();
  const times = [];
  for (let i = 0; i < repeats; i += 1) {
    const start = performance.now();
    call();
    times.push(performance.now() - start);
  }
  times.sort((a, b) => a - b);
  const middle = times.length >> 1;
  return times.length % 2 ? times[middle] : (times[middle - 1] + times[middle]) / 2;
}

const [width, height] = size;
// the mesh is freed inside the call, as the native one is dropped inside its call
console.log(
  JSON.stringify({
    to_rgba: median(() => drawing.toRgba(width, height)),
    to_mesh: median(() => drawing.toMesh(width, height).free()),
  }),
);
