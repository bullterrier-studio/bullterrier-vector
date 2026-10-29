// A mesh upload from WebAssembly in a page: the drawing becomes triangles in WebAssembly memory, and the page wraps
// typed arrays over that memory and hands them to WebGL, without copying them through JavaScript objects.
//
// The pointer pattern: `toMesh` fills a Mesh whose buffers live in WebAssembly memory and reports where they are
// (a pointer, a byte offset, and a length, in elements) and a generation that changes exactly when their contents
// change. The page makes its views fresh after each call (a pointer can move, and the memory can grow) and uploads only
// when the generation moved. Each vertex is 8 floats: x, y (output pixels, y down), r, g, b, a (premultiplied), item,
// order. Each draw range is 3 integers: first index, index count, blend mode (0 normal, 1 multiply).

import * as btvec from "bullterrier-vector";

const canvas = document.querySelector("canvas");
const gl = canvas.getContext("webgl2", { premultipliedAlpha: true });

const spin = new btvec.Param("spin", { default: 0, min: 0, max: 360, unit: "deg", meaning: "how far the stars have turned" });
const centre = [540, 540];
const star = btvec.Path.star(centre, 420, 160, { points: 12 });
const drawing = new btvec.Drawing(
  [
    star.fill("#ff4f9a").rotate(spin, { about: centre }),
    star.fill("#2a5bff").blend("multiply").rotate(spin.neg(), { about: centre }),
  ],
  { size: [1080, 1080] },
);

const program = linkProgram(gl); // the page's own shaders: position and colour; item and order for its own effects
gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, gl.createBuffer());
describeVertexLayout(gl, program); // 8 floats a vertex, 32 bytes apart
gl.enable(gl.BLEND);

let mesh = null;
let uploaded = null;
let ranges = new Uint32Array(0);

function draw(now) {
  // the clock is the page's: one turn every 12 s; an angle wraps, so `spin` may grow without end
  mesh = drawing.toMesh(canvas.width, canvas.height, { spin: (now / 1000) * 30 }, mesh);
  if (mesh.generation !== uploaded) {
    const memory = btvec.memory.buffer;
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(memory, mesh.verticesPtr, mesh.verticesLength), gl.DYNAMIC_DRAW);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint32Array(memory, mesh.indicesPtr, mesh.indicesLength), gl.DYNAMIC_DRAW);
    ranges = new Uint32Array(memory, mesh.rangesPtr, mesh.rangesLength).slice(); // a copy: read every frame
    uploaded = mesh.generation;
  }
  gl.clear(gl.COLOR_BUFFER_BIT);
  for (let r = 0; r < ranges.length; r += 3) {
    const [first, count, blend] = ranges.subarray(r, r + 3);
    if (blend === 1) gl.blendFunc(gl.DST_COLOR, gl.ONE_MINUS_SRC_ALPHA); // multiply, premultiplied
    else gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA); // normal, premultiplied
    gl.drawElements(gl.TRIANGLES, count, gl.UNSIGNED_INT, first * 4);
  }
  requestAnimationFrame(draw);
}
requestAnimationFrame(draw);

// When the page is done with it, `mesh.free()` returns its memory to WebAssembly.

// The WebGL set-up is the page's own and has nothing to do with the library; it is left out here.
function linkProgram(gl) { /* compile and link a vertex and a fragment shader */ }
function describeVertexLayout(gl, program) { /* vertexAttribPointer for x, y; r, g, b, a; item; order */ }
