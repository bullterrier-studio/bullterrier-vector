// A star in Node: the drawing of examples/star.py, loaded from the JSON document `Drawing.to_json()` writes for it
// (the JavaScript package loads and draws documents; building one is done in Python or Rust).
//
//   node crates/wasm/examples/star.mjs           prints the size of the SVG and a checksum of the pixels
//   node crates/wasm/examples/star.mjs svg       writes the SVG document to standard output
//   node crates/wasm/examples/star.mjs png       writes the PNG file
//   node crates/wasm/examples/star.mjs rgba      writes the 400 by 400 straight-alpha pixels
//
// Needs the package in pkg/, which `uv run poe wasm-test` builds. The pixels are drawn by a rasteriser that is only
// required to look the same as the native ones, so the checksum may differ from the Rust example's.
import { createRequire } from "node:module";

const { Drawing } = createRequire(import.meta.url)("../../../pkg/bullterrier_vector_wasm.js");

const STAR = `{"root":[{"stack":{"items":[[{"path":"M200 50 L235.2671151375484 151.45898033750314 L342.658477444273 153.6474508437579 L257.0633909777092 218.54101966249684 L288.167787843871 321.3525491562421 L200 260 L111.83221215612905 321.3525491562421 L142.93660902229078 218.54101966249686 L57.341522555726954 153.64745084375792 L164.7328848624516 151.45898033750316 Z"},{"fill":{"color":"#ffcc33"}},{"stroke":{"color":"#1a1a1a","width":4}}],[{"path":"M200 42 C204.41827799864635 42 208 45.581722001353654 208 50 C208 54.418277998646346 204.41827799864635 58 200 58 C195.58172200135365 58 192 54.418277998646346 192 50 C192 45.581722001353654 195.58172200135365 42 200 42 Z"},{"fill":{"color":"#e2412e"}}],[{"path":"M200 260 L167.1670106718773 282.8472057761859"},{"stroke":{"color":"#e2412e"}}]],"size":[400,400]}}],"version":1}`;
const SIDE = 400;

// the 32-bit FNV-1a hash, as the Rust example computes it
const checksum = (bytes) => bytes.reduce((hash, byte) => Math.imul(hash ^ byte, 0x01000193) >>> 0, 0x811c9dc5);

const star = Drawing.fromJson(STAR);
switch (process.argv[2]) {
  case "svg":
    process.stdout.write(star.toSvg(SIDE, SIDE));
    break;
  case "png":
    process.stdout.write(star.toPng(SIDE, SIDE));
    break;
  case "rgba":
    process.stdout.write(star.toRgba(SIDE, SIDE));
    break;
  case undefined: {
    const rgba = star.toRgba(SIDE, SIDE);
    console.log(
      `svg: ${Buffer.byteLength(star.toSvg(SIDE, SIDE))} bytes, rgba: ${rgba.length} bytes, checksum ${checksum(rgba).toString(16).padStart(8, "0")}`,
    );
    break;
  }
  default:
    console.error(`expected svg, png or rgba, got ${process.argv[2]}`);
    process.exitCode = 1;
}
