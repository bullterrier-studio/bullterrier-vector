// A mesh upload from WebAssembly in a page: the drawing becomes triangles in WebAssembly memory, and the page wraps a
// typed array over that memory and hands it to WebGL, without copying it through JavaScript objects.
//
// The pointer pattern: `toMesh` fills a Mesh whose buffers live in WebAssembly memory and reports where they are
// (a pointer and a length, in elements) and a generation that changes exactly when their contents change. The page
// makes its views fresh each frame (the pointer can move, and the memory can grow) and uploads only when the
// generation moved. Each vertex is 8 floats: x, y (output pixels, y down), r, g, b, a (premultiplied), item, position.

import * as btvec from "bullterrier-vector";

const canvas = document.querySelector("canvas");
const gl = canvas.getContext("webgl2", { premultipliedAlpha: true });

const spin = new btvec.Param("spin", { default: 0, min: 0, max: 360, unit: "deg", meaning: "how far the star has turned" });
const centre = [540, 540];
const star = btvec.Path.star(centre, 420, 160, { points: 12 });
const drawing = new btvec.Drawing([star.fill("#ff4f9a").rotate(spin, { about: centre })], { size: [1080, 1080] });

const program = linkProgram(gl); // the page's own shaders: position, colour; item and position for its own effects
const vertexBuffer = gl.createBuffer();
const indexBuffer = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
describeVertexLayout(gl, program); // 8 floats a vertex, 32 bytes apart

let mesh = null;
let uploaded = null;

function draw(now) {
  // the clock is the page's: one turn every 12 s; an angle wraps, so `spin` may grow without end
  mesh = drawing.toMesh(canvas.width, canvas.height, { spin: (now / 1000) * 30 }, mesh);
  if (mesh.generation !== uploaded) {
    const vertices = new Float32Array(btvec.memory.buffer, mesh.verticesPtr, mesh.verticesLength);
    const indices = new Uint32Array(btvec.memory.buffer, mesh.indicesPtr, mesh.indicesLength);
    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.DYNAMIC_DRAW);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.DYNAMIC_DRAW);
    uploaded = mesh.generation;
  }
  gl.clear(gl.COLOR_BUFFER_BIT);
  gl.drawElements(gl.TRIANGLES, mesh.indicesLength, gl.UNSIGNED_INT, 0);
  requestAnimationFrame(draw);
}
requestAnimationFrame(draw);

// The WebGL set-up is the page's own and has nothing to do with the library; it is left out here.
function linkProgram(gl) { /* compile and link a vertex and a fragment shader */ }
function describeVertexLayout(gl, program) { /* vertexAttribPointer for x, y; r, g, b, a; item; position */ }
