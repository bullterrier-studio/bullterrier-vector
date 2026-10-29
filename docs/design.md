# Design

Status: planned; the repository has the scaffold, its gates and CI workflows, and one skeleton function; see
[docs/roadmap.md](roadmap.md).

Draft for review, 2026-10-28. Batches that build it: [roadmap.md](roadmap.md). Decisions and their reasons:
[design-decisions.md](design-decisions.md). What the prototypes taught: [prototype-findings.md](prototype-findings.md).

## What this is

A typography-first vector graphics scripting library. You write a picture as code: Bezier paths, type, booleans,
offsets, deformers, repeaters, generative forms and tracing. The library draws it as SVG or pixels. The same code works
in Rust, Python and JavaScript (through WebAssembly), and gives the same pixels in all three. A drawing is also data: it
saves to JSON and draws again with new values.

The aim is graphics worth posting: kinetic type, cut-paper forms, generative posters, pen-plotter art, and animations to
lay over footage or run live in a web page. Stars and polygons are presets, not the point.

### Scope

- **Paths.** Bezier paths are values: build, measure (length, point and tangent at a distance), split, slice, reverse,
  hit-test, resample, simplify, fit curves through points. Booleans, offsets and insets (round, miter, bevel joins),
  stroke outlines.
- **Type.** Fonts from bytes, shaping, variable axes as parameters, every glyph an editable path ("Fonts" below).
- **A generative layer.** Generators, deformers (wobble, wave, twist, bulge), repeaters, draw-on, morph, evenly spaced
  flow-field hatching.
- **Tracing.** A grayscale map to outlines at chosen levels; a map of colour labels to outlines per colour.
- **Output.** SVG that keeps every effect (shadows, halftone, knockouts); plotter-ready SVG (stroke only, ordered to
  reduce pen travel); RGBA pixels at any size; a mesh (triangles with colours and per-item attributes, in flat buffers
  for a GPU); a frame-export helper ("Animation").

### Fonts

- Any TrueType or OpenType font, variable or not, from bytes. WOFF and WOFF2: to verify in V5 (whether the font crates
  read them or they need unpacking first).
- Variable axes (weight, width, slant, custom) become described parameters; shaping with ligatures, kerning, complex
  scripts and OpenType features; every glyph an editable path that knows its place in the text, word and line.
- Not in v1: colour and bitmap fonts.
- Never: producing font files (a font editor or fontTools does that).

## The gap

What we found (October 2026, not exhaustive):

- **Python** has no cross-platform library for typography-driven generative graphics. [DrawBot](https://www.drawbot.com)
  is the reference for scripted posters and type, and runs on macOS only; [drawbot-skia](https://pypi.org/project/drawbot-skia/)
  ports part of its API, leaving shaping and line wrapping to be built and with no browser build;
  [Shoebot](https://shoebot.readthedocs.io/) draws through Cairo. Lower-level bindings (skia-python, pycairo) draw paths
  but leave type layout and generative tools to the user.
- **Rust** has strong building blocks from [Linebender](https://linebender.org): kurbo (curves), Parley (text layout,
  shaping through HarfRust), peniko (styles), vello (rendering). We did not find a high-level creative layer on top of
  them: type as editable geometry, generative nodes, a drawing you can describe and redraw.
- **JavaScript** has [Paper.js](http://paperjs.org) (a vector scene graph, with little typography and no glyph outlines)
  and p5.js (immediate-mode drawing on a canvas, no vector model to edit or export).

What this library adds: type as geometry (every glyph an editable path, variable-font axes as parameters), a generative
layer, and one engine whose output is identical in Python, Rust and the browser.

## Who it is for

- Designers and creative coders making posters, type specimens, social graphics and generative series from scripts or
  notebooks.
- Motion designers and tool builders who animate drawings by setting values per frame (from a script, a video tool, an
  audio analyser or a browser loop).
- Pen-plotter artists who need clean, ordered line output.
- Developers of web tools who want the same engine in the browser preview, in a live page and on the server.

## Goals

- **Code first.** A drawing is written as ordinary code in each language: short, chainable, typed, discoverable from the
  editor. JSON is a layer on top: every drawing written in code saves to JSON and loads back to the same drawing, never
  the other way round (code shaped like JSON).
- **Scripting and drawings in one vocabulary.** The same verbs compute at once on a path and build a re-drawable
  drawing ("The code API").
- **Type as geometry.** Real shaping, variable-font axes as parameters, every glyph an editable path.
- **A generative layer.** Generators, deformers, repeaters, booleans, offsets, morph, hatching, tracing a grayscale map.
  Every node has described parameters (name, unit, range, default, meaning).
- **One engine everywhere.** The same drawing gives the same pixels in Rust, Python and WebAssembly, so a browser
  preview matches a server render.
- **State, not time.** Every parameter is a state; drawing again with new values is cheap, fast enough for a live
  preview ("Animation").
- **Output that loses nothing.** The SVG shows what the pixels show; the plotter file is lines only, in a sensible
  order.
- **Good by default.** Defaults and presets look good with no tuning; every effect ships with a gallery piece.

## Non-goals

- Timelines, keyframes, easing over time or an `onFrame` loop: values over time are the caller's. The frame-export
  helper only calls the caller's function.
- An editor application. The library gives an editor what it needs (geometry, hit testing, text layout); the editor's
  user interface is a separate project.
- Photo effects and compositing over footage.
- Writing a rasteriser, a shaper or a font parser: those come from existing crates.
- Stateful simulation (particles, springs, flocking) in v1. If it comes, it produces points that a node draws.
- Effects that need several frames (a shape's trail): they belong to a video tool.

## Anti-goals (it fails even if it works when...)

- the output looks like a vector-library demo;
- the code API reads like configuration (dicts of dicts, stringly-typed options);
- preview and render differ;
- a user needs to know which crate draws the pixels;
- adding a node means touching the core.

## The code API

Decided: code first, JSON second, and four ideas; everything else is a method on one of them. The names, signatures and
examples: [api.md](api.md).

| Idea | What it is |
|---|---|
| `Path` | a Bezier outline (one or more sub-paths): a plain value; every verb on it computes at once |
| `Shape` | one path with a style and attributes (index; glyph, char, word, line; part; copy; level; order 0..1; origin) |
| `Drawing` | an immutable tree of steps over shapes: draws again with new values, saves to JSON, the same in every language |
| `Param` | a named number with unit, range, default and meaning; any number in a step may be one |

The geometry verbs (offset, subtract, slice, wobble...) have the same names and arguments on a `Path`, where they
compute now, and on a `Drawing`, where they add a step computed at draw time. Every call returns a new value. Arithmetic
on `Param`s builds an expression that saves with the drawing; per-item numbers (by index, by order, seeded random)
work wherever a number does, and a caller's signal per item is a list given at draw time. A drawing holds no callbacks.

```python
import bullterrier_vector as btvec

star = btvec.Path.star((200, 200), outer=100, inner=40, points=5)  # a Path: computed now
band = star.offset(8).subtract(star.offset(-8))
tip = star.point_at(0)  # positions along a path are fractions 0..1 of its length

progress = btvec.Param("progress", default=0, min=0, max=1, unit="fraction", meaning="how much has drawn on")
poster = (
    btvec.text("LOUD", font=btvec.Font(font_bytes), size=400, axes={"wght": 800})
    .wobble(amplitude=4, phase=0.25)
    .slice(0, btvec.stagger(progress, spread=0.5))  # a Drawing: steps, computed at draw time
    .stroke("#15161a", width=4)
)
svg = poster.to_svg(1080, 1350, progress=0.7)
```

```rust
let poster = btvec::text("LOUD", Text::new(&font).size(400.0).axis("wght", 800.0))
    .wobble(Wobble::new().amplitude(4.0).phase(0.25))
    .slice(0.0, Stagger::new(&progress).spread(0.5))
    .stroke(Stroke::new(Color::hex(0x15161a)).width(4.0));
let svg = poster.to_svg(Size::px(1080.0, 1350.0), &Values::new().set("progress", 0.7))?;
```

```js
const poster = btvec.text("LOUD", { font, size: 400, axes: { wght: 800 } })
  .wobble({ amplitude: 4, phase: 0.25 })
  .slice(0, btvec.stagger(progress, { spread: 0.5 }))
  .stroke("#15161a", { width: 4 });
const svg = poster.toSvg(1080, 1350, { progress: 0.7 });
```

`poster.to_json()` and `Drawing.from_json(...)` save and load the same tree (the format is V1b), and
`poster.parameters()` lists every parameter with its description, so a tool can build sliders or map a signal onto one
without knowing the steps.

## Animation: state, not time

Every parameter is a state (`progress` 0..1, `phase` in cycles, `seed`), never a time. Whoever animates sets values
and draws again:

- **Frame export** (planned, V4). A small helper loops a caller's function (time in, a drawing with its values out) over
  frames and writes a transparent PNG sequence, or ProRes 4444 or WebM with alpha through ffmpeg. It is the only part
  that knows frames; it has no timeline, keyframes or mixing, and it lives outside the core.
- **Live in a page** (planned: the package in V3, an example in V20). The WebAssembly package redraws on a canvas from
  values the page supplies: the clock, scroll, the pointer, a Web Audio level; or hands the page a mesh to draw with the
  GPU (V3; a GPU display and live remeshing are spikes to measure first). A piece too heavy to redraw live is exported
  with frame export instead.
- **Any other driver** (a script, a slider, another program) sets values per frame and draws again.

Placement that stays stable as values change, for animated hatching, is open (D7).

## Effects

An effect is a finished look built from the library's primitives: named, with described parameters and good defaults,
and added only with a gallery piece that uses it. They come in an open-ended effects track (planned, from V15);
candidates: [effects-roadmap.md](effects-roadmap.md).

Effects usually applied to pixels that must stay vector (duotone, gradient maps, posterize, halftone dots, glitch
slices, misregistration) are the library's own: operations on fill and stroke colours and on geometry, not SVG filters.
Halftone is a fill with its own fast path.

## Shape of the library

### One concept: the node

A `Drawing` is a tree of steps; each step is a node. A node takes zero or more inputs (lists of shapes) and gives one
list of shapes. A generator (text, trace, a literal path) has no inputs, a modifier (wobble, offset, draw-on) has one, a
combiner (group, boolean) has several. One trait, one registry, one way to describe parameters, rather than separate
hierarchies for shapes, modifiers and repeaters. The chainable methods of the code API build this tree; JSON is the same
tree written down.

### Shapes carry attributes

What flows between nodes is a list of shapes: paths, a style, and attributes. Text writes per-glyph attributes (index
in text, word, line; order 0..1); repeaters write a copy index; trace writes a level. Modifiers read attributes
without knowing who wrote them. Per-item variation (staggering a draw-on letter by letter, scattering by index) is one
mechanism in the core, so every modifier gets it and none implements it.

### Modules, each hiding one decision

| Module | Hides | Never exposed |
|---|---|---|
| geometry | path representation, path maths (measure, split, fit), booleans, offsets, stroke outline, halftone, robustness fixes | the curve and boolean crates |
| type | font loading, shaping, layout, variable axes, glyph outlines | the font, shaping and layout crates |
| scene | shapes, attributes, the node trait, evaluating a tree, per-item variation, caching between renders | evaluation order, caches |
| parameters | declaring, describing, validating values; expressions; design pixels scaled by output size; cyclic angles wrap | scaling arithmetic (callers never scale) |
| nodes | the built-ins, the registry | internals of any node |
| output | SVG writing (every effect kept), plotter SVG, rasterising, SVG import | the raster and SVG crates |
| json | saving and loading drawings, versioning | the serialisation library |

Parameters are declared once, in Rust; the Python and JavaScript APIs, their type stubs and the JSON Schema are built
from those declarations.

### Rules that keep it portable and deterministic

- The core does no input or output of its own: fonts, SVG files and maps arrive as bytes. The language layers read
  files; frame export writes them.
- It runs single-threaded; parallel rendering is an optional feature.
- Transcendental maths (sin, cos, exp, pow...) goes through one portable implementation on every target, so output
  matches bit for bit. Iteration order never depends on a hash seed.
- Randomness comes only from the seed.
- Inputs are plain data: a map to trace is a width, a height and bytes; a series is a list of numbers. The library
  reads no other tool's file format.

## Deliverables

| # | Deliverable | For |
|---|---|---|
| 1 | Rust crate `bullterrier-vector` | Rust users, render servers, video tools |
| 2 | Python package `bullterrier-vector` (wheels per platform) | scripts, notebooks, batch posters |
| 3 | npm package (WebAssembly) | browser tools, live pages, JS users |
| 4 | Command line `bt-vector`: `render`, `describe`, `list`, `validate` | rendering JSON drawings without code |
| 5 | Frame export: a transparent PNG sequence or clip from a function of time | animations to lay over footage |
| 6 | The JSON drawing format: spec, JSON Schema, versioning | saving, sharing, presets, other tools |
| 7 | Built-in nodes and effects, each with described parameters; every effect with a gallery piece | everything above |
| 8 | Bundled open-licence fonts | type working out of the box |
| 9 | Presets and a gallery | proof it looks good; starting points |
| 10 | A browser playground (a drawing plus sliders built from its parameter descriptions) and a live page example | proof of the WebAssembly build; a showcase |
| 11 | Three reference pieces | the acceptance test for "worth posting" |
| 12 | Docs: README, API guides per language, the JSON format, worked examples run as tests | users and contributors |
| 13 | Tests: contract tests over every node, property tests, the same output on every target, geometry fuzzing, frame-time benchmarks | keeping 1–12 true |

### The three reference pieces

1. **Kinetic type:** letters in a variable font breathe in weight and width with a value (a bass level, a slider),
   staggered letter by letter, split and scatter, reassemble.
2. **Cut paper:** a figure traced from a grayscale mask (say, a segmentation of a dancer) into stacked paper layers in
   flat colours (a blobby mount, shifted echoes, drop shadows), its outline drawing on.
3. **Generative poster:** evenly spaced flow-field hatching inside giant letterforms, denser where the tone is darker,
   as a printable SVG and a pen-plotter file.

All three were built once as throwaway prototypes ([prototype-findings.md](prototype-findings.md)). A feature earns a
place in the library when one of these needs it, or when a user asks for it with a real piece in hand.

## Works with

The library is independent: it depends on no other Bullterrier package and none is needed to use it. It meets other
tools only through plain data, with no runtime link in either direction. In the
[Bullterrier](https://github.com/bullterrier-studio) family it is the drawing layer:

- **Maps in:** bullterrier-image's subject or depth map of a picture (width, height, bytes) drives tracing, hatching or
  halftone; the result stays vector.
- **Pixels out:** a drawing rendered as RGBA at any size takes any of bullterrier-image's pixel effects (the result is
  pixels), or masks one.
- **Colour effects:** the vector-native duotone, gradient map and posterize are kept consistent with bullterrier-image's
  pixel versions by differential tests that use it as a development-only reference, never a dependency of the package;
  those tests skip cleanly when it is not installed, and say so. A flat-colour pixel effect on a high-resolution render
  can come back as vector through the multi-colour trace (curves reconstructed, not exact).
- **Values over time:** bullterrier-control produces them; bullterrier-video sets a drawing's values per frame and
  composites it over footage.

## Open decisions

| # | Question | Needed by |
|---|---|---|
| D4 | Command line in Rust (installable with cargo) or in the Python package? | V18 |
| D5 | Who owns stateful simulation, if ever? | later |
| D6 | Custom nodes written in Python or JavaScript (local use only), and when? | later |
| D7 | Placement that stays stable as values change (evenly spaced lines that do not jump between frames): a stateless rule continuous in its parameters, or explicit state passed in and out? The answer bears on D5. | animated hatching, after V15 |
