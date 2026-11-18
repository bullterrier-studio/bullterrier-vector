// What the playground's sliders do, apart from the page: how a Param description (an element of `Drawing.parameters()`)
// becomes a slider, how a number read from a source (0 to 1) becomes a value of the Param, and what the sources are.

// A slider has this many positions from min to max, whatever the Param's range: fine enough to look continuous.
const SLIDER_STEPS = 1000;
// The clock source goes round once in this many milliseconds: slow enough to watch a drawing move.
const CLOCK_PERIOD_MS = 8000;

/** What wakes a source that keeps changing by itself: the page's animation loop, not an event. */
export const FRAME = "frame";

/** The sources a Param can be bound to. `read(state)` gives 0 to 1 from `{ now, pointerX, pointerY, scroll }` (now in
 * milliseconds, the others as fractions of the page). `wakes` names what makes the source worth reading again: a window
 * event (`"pointermove"`, `"scroll"`) or `FRAME`; `manual` has none, the slider itself is the value. */
export const SOURCES = [
  { id: "manual", label: "manual", read: () => 0 },
  { id: "clock", label: "clock", wakes: FRAME, read: ({ now }) => (now % CLOCK_PERIOD_MS) / CLOCK_PERIOD_MS },
  { id: "pointerX", label: "pointer x", wakes: "pointermove", read: ({ pointerX }) => pointerX },
  { id: "pointerY", label: "pointer y", wakes: "pointermove", read: ({ pointerY }) => pointerY },
  { id: "scroll", label: "scroll", wakes: "scroll", read: ({ scroll }) => scroll },
];

// The library's cyclic units: a Param in one wraps round (docs/formats.md "Params").
const CYCLIC_UNITS = ["deg", "cycle"];

function checked(param) {
  const { name, min, max } = param;
  if (!(min < max)) throw new Error(`parameter "${name}" cannot be a slider: its min (${min}) is not below its max (${max})`);
  return param;
}

/** The slider for `param`: `{ min, max, step, default, cyclic }`; a cyclic Param (unit "deg" or "cycle") wraps round.
 * Throws an Error naming the Param if it has no usable range. */
export function sliderSpec(param) {
  const { min, max, default: start, unit } = checked(param);
  return { min, max, step: (max - min) / SLIDER_STEPS, default: start, cyclic: CYCLIC_UNITS.includes(unit) };
}

/** The value of `param` for a source reading `fraction`, 0 at its min to 1 at its max: a cyclic Param wraps (1.25 is 0.25),
 * any other holds at its ends. Always inside the range. Throws as `sliderSpec`. */
export function valueFrom(param, fraction) {
  const { min, max, cyclic } = sliderSpec(param);
  const place = cyclic ? fraction - Math.floor(fraction) : Math.min(1, Math.max(0, fraction));
  return min + place * (max - min);
}
