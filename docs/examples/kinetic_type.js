// Kinetic type in JavaScript: the same program as kinetic_type.py, drawn live on a canvas in a page.
//
// Two lines of condensed type in a variable font start as hairlines, a blue plane rises with the letters cut out of
// it, the drop bursts them into a two-ink riso print misregistered more on each hit, the letters pulse one after
// another, and the print thins back to hairlines. The drawing is built once; each animation frame the page's clock
// becomes values and the drawing is drawn again. `pulse` is passed as one value per letter.

import * as btvec from "bullterrier-vector";

const WIDTH = 1080, HEIGHT = 1350;
const PAPER = "#f3eee4", INK = "#16151a", PINK = "#ff4f9a", BLUE = "#2a5bff";

const response = await fetch("fonts/Anybody[wdth,wght].ttf");
const font = new btvec.Font(new Uint8Array(await response.arrayBuffer()));

const weight = new btvec.Param("weight", { default: 600, min: 100, max: 780, meaning: "how heavy the letters are, before a pulse" });
const rise = new btvec.Param("rise", { default: 0, min: 0, max: 1, unit: "fraction", meaning: "how far the blue plane has risen" });
const ink = new btvec.Param("ink", { default: 0, min: 0, max: 1, unit: "fraction", meaning: "0: hairlines only; 1: the riso print" });
const burst = new btvec.Param("burst", { default: 0, min: 0, max: 1, unit: "fraction", meaning: "how hard the last hit was" });
const pulse = new btvec.Param("pulse", { default: 0, min: 0, max: 1, unit: "fraction", meaning: "a kick, per letter" });

// One shape per letter; each line fitted to the width by the width axis once, at the calm pose.
const letters = btvec
  .text("NO\nSLEEP", {
    font,
    size: 560,
    axes: { wght: weight.add(pulse.mul(120)) },
    leading: 520,
    anchor: [WIDTH / 2, 600],
    align: "center",
    width: 980,
    fit: "wdth",
  })
  .scale(pulse.mul(0.08).add(1))
  .rotate(btvec.random(-10, 10, { seed: 7 }).mul(burst))
  .translate(btvec.random(-30, 30, { seed: 8 }).mul(burst), btvec.random(-50, 50, { seed: 9 }).mul(burst));

const frame = btvec.Path.rect(0, 0, WIDTH, HEIGHT);
const misregistration = burst.mul(14).add(3); // a constant offset looks like a mistake; it grows on a hit

const paper = frame.fill(PAPER);
const plane = new btvec.Drawing([frame]).translate(0, rise.neg().add(1).mul(HEIGHT));
const blue = plane
  .subtract(letters.translate(misregistration, misregistration.mul(0.6)))
  .fill(BLUE)
  .blend("multiply");
const pink = letters.fill(PINK).blend("multiply").opacity(ink);
const hairlines = letters.stroke(INK, { width: 1.5 }).opacity(ink.neg().add(1));

const piece = new btvec.Drawing([paper, blue, pink, hairlines], { size: [WIDTH, HEIGHT] });
const count = letters.shapes().length; // 7: a space or a line break gives no shape

// A kick on every beat at 120 bpm: 1 on the beat, falling to 0 within 0.3 s.
const kick = (t) => 1 - btvec.ease.outCubic((t % 0.5) / 0.3);

// The values `t` seconds in: hairlines, the plane rises from 4 s, the drop at 6 s, thinning out from 10 s.
function valuesAt(t) {
  const printed = t >= 6 && t < 11;
  const heavy = btvec.ease.outBack((t - 6) / 0.3) - btvec.ease.inOutCubic((t - 10) / 1.5);
  const hit = (at) => (printed ? kick(at) : 0);
  return {
    rise: btvec.ease.inOutCubic((t - 4) / 2),
    ink: printed ? 1 : 0,
    weight: 100 + 500 * heavy,
    burst: hit(t),
    pulse: Array.from({ length: count }, (_, i) => hit(t - 0.045 * i)),
  };
}

const canvas = document.querySelector("canvas");
const context = canvas.getContext("2d");
const start = performance.now();

function draw(now) {
  const t = ((now - start) / 1000) % 12;
  const rgba = piece.toRgba(canvas.width, canvas.height, valuesAt(t));
  context.putImageData(new ImageData(new Uint8ClampedArray(rgba), canvas.width, canvas.height), 0, 0);
  requestAnimationFrame(draw);
}
requestAnimationFrame(draw);
