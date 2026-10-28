# Design

Status: draft for review, 2026-10-28. Batches that build it: [roadmap.md](roadmap.md). Decisions and their reasons:
[design-decisions.md](design-decisions.md).

## What this is

A typography-first generative vector library. You describe a picture in code: type, shapes and generative forms, with
deformers, repeaters and boolean operations applied to them. The library draws it as SVG or pixels. The same code works
in Rust, Python and JavaScript (through WebAssembly), and gives the same pixels in all three.

The aim is graphics worth posting: kinetic type, cut-paper forms, generative posters, pen-plotter art. Stars and
polygons are presets, not the point.

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
- Developers of web tools who want the same engine in the browser preview and on the server.

## Goals

- **Code first.** A drawing is written as ordinary code in each language: short, chainable, typed, discoverable from the
  editor. JSON is a layer on top: every drawing written in code saves to JSON and loads back to the same drawing, never
  the other way round (code shaped like JSON).
- **Type as geometry.** Real shaping (ligatures, kerning, complex scripts), variable-font axes as parameters, every glyph
  an editable path that knows its place in the text, word and line.
- **A generative layer.** Generators, deformers, repeaters, booleans, offsets, morph, tracing a grayscale image, drawing
  a series of numbers. Every node has described parameters (name, unit, range, default, meaning).
- **One engine everywhere.** The same drawing gives the same pixels in Rust, Python and WebAssembly, so a browser
  preview matches a server render.
- **State, not time.** Every parameter is a state (`progress` 0..1, `phase` in cycles); an animation sets values per
  frame and draws again. Drawing again with new values is cheap, fast enough for a live preview.
- **Good by default.** Defaults and presets look good with no tuning.

## Non-goals

- Timelines, keyframes, easing or an `onFrame` loop: values over time are the caller's.
- An editor application. The library gives an editor what it needs (geometry, hit testing, text layout); the editor's
  user interface is a separate project.
- Photo effects and compositing over footage.
- Writing a rasteriser, a shaper or a font parser: those come from existing crates.
- Stateful simulation (particles, springs, flocking) in v1. If it comes, it produces points that a node draws.

## Anti-goals (it fails even if it works when...)

- the output looks like a vector-library demo;
- the code API reads like configuration (dicts of dicts, stringly-typed options);
- preview and render differ;
- a user needs to know which crate draws the pixels;
- adding a node means touching the core.

## The code API (sketch)

Illustrative; V1 settles the names. One drawing, three languages:

```python
import bullterrier_vector as bv

poster = (
    bv.text("LOUD", font=bv.Font.from_file("Inter.ttf"), size=400, weight=800)
    .split("letters")
    .wobble(amount=6, phase=0.25)
    .draw_on(progress=0.7, stagger=0.5)
)
svg = poster.to_svg(width=1080, height=1350)
```

```rust
let poster = text("LOUD").font(&inter).size(400.0).weight(800.0)
    .split(Split::Letters)
    .wobble(Wobble { amount: 6.0, phase: 0.25, ..Wobble::default() })
    .draw_on(DrawOn { progress: 0.7, stagger: 0.5 });
let svg = poster.to_svg(Size::new(1080, 1350))?;
```

```js
const poster = text("LOUD", { font: inter, size: 400, weight: 800 })
  .split("letters").wobble({ amount: 6, phase: 0.25 }).drawOn({ progress: 0.7, stagger: 0.5 });
```

`poster.to_json()` and `Drawing.from_json(...)` save and load the same tree; `poster.parameters()` lists every
parameter with its description, so a tool can build sliders or map a signal onto one without knowing the node.

## Shape of the library

### One concept: the node

A drawing is a tree of nodes. A node takes zero or more inputs (sets of items) and gives one set of items. A
generator (text, star, trace) has no inputs, a modifier (wobble, offset, draw-on) has one, a combiner (group, boolean)
has several. One trait, one registry, one way to describe parameters, rather than separate hierarchies for shapes,
modifiers and repeaters. The chainable methods of the code API build this tree; JSON is the same tree written down.

### Items carry attributes

What flows between nodes is a list of items: a path, its style, and attributes. Text writes per-glyph attributes (index
in text, word, line; position 0..1); repeaters write a copy index; trace writes a level. Modifiers read attributes
without knowing who wrote them. Per-item variation (staggering a draw-on letter by letter, scattering by index) is one
mechanism in the core, so every modifier gets it and none implements it.

### Modules, each hiding one decision

| Module | Hides | Never exposed |
|---|---|---|
| geometry | path representation, path maths, booleans, offsets, stroke outline, robustness fixes | the curve and boolean crates |
| type | font loading, shaping, layout, variable axes, glyph outlines | the font, shaping and layout crates |
| scene | items, attributes, the node trait, evaluating a tree, per-item variation, caching between renders | evaluation order, caches |
| parameters | declaring, describing, validating values; design pixels scaled by output size; cyclic angles wrap | scaling arithmetic (callers never scale) |
| nodes | the built-ins, the registry | internals of any node |
| output | SVG writing, rasterising, SVG import | the raster and SVG crates |
| json | saving and loading drawings, versioning | the serialisation library |

Parameters are declared once, in Rust; the Python and JavaScript APIs, their type stubs and the JSON Schema are built
from those declarations.

### Rules that keep it portable and deterministic

- The core does no input or output of its own: fonts, SVG files and images arrive as bytes. The language layers read
  files.
- It runs single-threaded; parallel rendering is an optional feature.
- Transcendental maths (sin, cos, exp, pow...) goes through one portable implementation on every target, so output
  matches bit for bit. Iteration order never depends on a hash seed.
- Randomness comes only from the seed.
- Inputs are plain data: an image to trace is a width, a height and bytes; a series is a list of numbers. The library
  reads no other tool's file format.

## Deliverables

| # | Deliverable | For |
|---|---|---|
| 1 | Rust crate `bullterrier-vector` | Rust users, render servers, video tools |
| 2 | Python package `bullterrier-vector` (wheels per platform) | scripts, notebooks, batch posters |
| 3 | npm package (WebAssembly) | browser tools, JS users |
| 4 | Command line `bt-vector`: `render`, `describe`, `list`, `validate` | rendering JSON drawings without code |
| 5 | The JSON drawing format: spec, JSON Schema, versioning | saving, sharing, presets, other tools |
| 6 | Built-in nodes, each with described parameters | everything above |
| 7 | Bundled open-licence fonts | type working out of the box |
| 8 | Presets and a gallery | proof it looks good; starting points |
| 9 | A browser playground: a drawing plus sliders built from its parameter descriptions | proof of the WebAssembly build; a showcase |
| 10 | Three reference pieces | the acceptance test for "worth posting" |
| 11 | Docs: README, API guides per language, the JSON format, worked examples run as tests | users and contributors |
| 12 | Tests: contract tests over every node, property tests, the same output on every target, geometry fuzzing, frame-time benchmarks | keeping 1–11 true |

### The three reference pieces

1. **Kinetic type:** letters in a variable font breathe in weight with a value (a bass level, a slider), split and
   scatter, reassemble.
2. **Cut paper:** a person cut out of a photo (any grayscale mask) traced into four offset layers in flat colours, its
   outline drawing on.
3. **Generative poster:** flow-field hatching inside giant letterforms, as a printable SVG and a pen-plotter file.

A feature earns a place in the library when one of these needs it, or when a user asks for it with a real piece in hand.

## Works with

The library is independent: it depends on no other Bullterrier package and none is needed to use it. In the
[Bullterrier](https://github.com/bullterrier-studio) family it is the drawing layer: bullterrier-control produces values
over time, bullterrier-image textures pictures (a drawing's outlines as a region), bullterrier-video sets a drawing's
values per frame and composites it. They meet only through plain data.

## Open decisions

| # | Question | Needed by |
|---|---|---|
| D4 | Command line in Rust (installable with cargo) or in the Python package? | V17 |
| D5 | Who owns stateful simulation, if ever? | later |
| D6 | Custom nodes written in Python or JavaScript (local use only), and when? | later |
