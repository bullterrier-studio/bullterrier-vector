# Design decisions

Each decision once, with its reason. The design: [design.md](design.md).

## Product

- **A general-purpose library first** (2026-10-28). Written for anyone making graphics in code, not for one ecosystem.
  *Reason:* a library tied to one family's architecture limits its users; the family gains from a library others use.
  *Consequence:* no Bullterrier format, name or concept in the API; the family is mentioned under "Works with" only.
- **Code API first, JSON second** (2026-10-28). A drawing is written as chainable, typed code in Rust, Python and
  JavaScript; JSON saves and loads the same tree. *Reason:* developers express ideas in code; an API designed from a
  file format reads like configuration. *Consequence:* each API is designed (with examples written first) before the
  JSON format; a drawing written in code round-trips through JSON to an identical render. The code API is V1a
  (`docs/api.md`), the format V1b.
- **Rust core, Python and WebAssembly bindings** (2026-10-28), not Paper.js or another JavaScript engine. *Reason:* one
  engine in the browser, on a server, in Python and inside a Rust video tool, with preview identical to render; the hard
  parts (shaping, glyph outlines, offsets, tracing) exist as Rust crates and are missing or weak in Paper.js.
  *Consequence:* the core does no IO, runs single-threaded, and uses one portable maths implementation.
- **Licence: MIT** (2026-10-28), not the Rust ecosystem's usual `MIT OR Apache-2.0`. The owner's choice. *Consequence:*
  one `LICENSE` file; `deny.toml` admits dependencies whose licences ask no more of the package than MIT does.
- **Frame export in, timeline out** (2026-10-28). The library ships a small helper that loops a caller's function of
  time over frames and writes a transparent PNG sequence or clip; it has no timeline, keyframes, mixing or `onFrame`
  loop. *Reason:* the owner wants to edit videos with these animations and drop them on footage with transparency, and
  the core stays timeless; sequencing values over time is the job of other tools. *Consequence:* time and frames
  exist only in the helper, which knows nothing of what the caller's function does; the helper writes files and runs
  ffmpeg, so it lives outside the core. The caller's function returns a drawing with its values set
  (`btvec.frames(scene, ...)`, 2026-10-29), so a frame can also change what values cannot, such as the map a figure is
  traced from.
- **Effects earn their place with a gallery piece** (2026-10-28). An effect is added together with a gallery piece that
  uses it. *Reason:* the bar is "worth posting", and a plausible effect can still look bad: the prototypes dropped
  halftone inside letters, outline echoes and a large wobble once they were seen in a piece. A test proves an effect
  works, not that it looks good. *Consequence:* the effects track is open-ended, one effect per batch;
  `docs/effects-roadmap.md` lists candidates, not promises.
- **Maps in, pixels out; colour effects that stay vector are the library's own** (2026-10-28). The library takes
  grayscale maps (width, height, bytes) and gives RGBA pixels; it never links a raster library at run time. Duotone,
  gradient maps, posterize, halftone dots, glitch slices and misregistration on a drawing are operations on fill and
  stroke colours and on geometry, not SVG filters, and are kept consistent with a raster library's pixel versions by
  differential tests that use it as a development-only reference; those tests skip cleanly when it is not installed, and
  say so. *Reason:* plain data keeps both libraries usable alone (general-purpose first); SVG filters render differently
  between renderers and plotters ignore them, and a filtered drawing is no longer editable geometry; a reference
  implementation catches drift in the colour maths without coupling two release cycles. *Consequence:* the reference is
  a development dependency only, never one of the package; which library it is, and the seams: design.md, "Works with".

## The code API

- **Four ideas, one vocabulary** (2026-10-28). `Path` (a Bezier value), `Shape` (paths with a style and attributes),
  `Drawing` (a re-drawable tree of steps) and `Param` (a described number); the same verbs, with the same names and
  arguments, work eagerly on a `Path` and as a step on a `Drawing`, over one implementation. *Reason:* both scripting
  (measure and cut a path now) and re-drawable drawings (save, animate, run in a page) are wanted; two vocabularies
  would double what a user learns and what is built and tested. *Consequence:* a batch that adds a verb adds it to both,
  and tests it once; the names are in `docs/api.md`.
- **An immutable chain** (2026-10-29). Every call returns a new description and nothing mutates; a number that
  changes is a named `Param` whose value is given at draw time (`drawing.to_png(1080, 1350, weight=820)`); eager `Path`
  scripting keeps the Paper.js feel for geometry. *Reason:* a drawing is drawn again with new values, saved, shared
  between places in one picture and cached between frames; a mutable scene would change everywhere it is used, and its
  saved form and cache would depend on the order of calls. *Consequence:* `with_values` fixes values; a drawing built
  once serves every frame.
- **Expressions on parameters, no callbacks inside a drawing** (2026-10-28). Arithmetic on `Param`s builds an expression
  that saves with the drawing; per-item numbers (by index, seeded random) are values of the same kind. A drawing never
  holds a function of the caller's. *Reason:* a drawing must save to JSON, load in another language, and render in a
  browser or on a server that runs no user code; a callback can do none of these. *Consequence:* callbacks are fine
  when scripting with `Path`, which computes at once and is not saved.
- **Per-item numbers are functions; a caller's per-item signal is a list** (2026-10-29). A number that differs per
  shape is a function of the shape's attributes (`btvec.by_index(lo, hi)`, `btvec.random(lo, hi, seed=...)`,
  `btvec.by_order(...)`); a per-item value that depends on the caller's signal is a list given at draw time
  (`pulse=[kick(t - 0.045 * i) for i in range(8)]`). *Reason:* a function of attributes is data, so it saves and runs
  anywhere; a signal lives outside the drawing, and a list carries it in as plain data where a callback could not go.
  *Consequence:* any Param accepts a number or a list; a list of the wrong length is a `ParameterError`.
- **`ease`: pure functions of a number, in the library** (2026-10-29; was D8). A small module of easing curves, each a
  function of a number with no time in it. *Reason:* every piece shaped its values with easing curves, and a function of
  a number carries no clock, so the core stays timeless. *Consequence:* the functions serve the caller's code (a
  function of time for frame export, a list per item); a drawing holds no easing.
- **Buffers, not objects: a mesh output** (2026-10-29, the owner's idea). A drawing also renders to triangles
  (positions, colours, per-item attributes; draw ranges with their blend mode) held in three flat buffers; across
  WebAssembly the bindings give a pointer, a length and a generation counter, and Python a view without a copy.
  *Reason:* a live page or a GPU display needs the picture as triangles every frame; one object per shape across the
  WebAssembly boundary costs more than the drawing, while a typed array over WebAssembly memory uploads at once, and the
  generation lets a page skip an unchanged upload. *Consequence:* the mesh is deterministic and covered by the
  same-bytes test; pixels and SVG stay the reference; a GPU display and live remeshing are spikes to measure
  (`docs/roadmap.md`), not promises. Multiply reaches the page as draw ranges, since fixed-function blending needs no
  shader, and a knockout as booleans while meshing; only a blurred shadow has no triangle form, and it raises, so
  nothing is left out quietly.
- **Positions along a path are fractions 0..1, from a canonical start** (2026-10-29, approved by the owner). Positions
  are fractions of the length, by arc length; a closed sub-path is measured from its topmost point; every outline the
  library makes runs in one canonical orientation (outer contours clockwise, holes counter-clockwise, in order of their
  topmost points, whatever the font's format). *Reason:* a position is then a state (draw-on is `slice(0, progress)`),
  a draw-on runs the same way in every font, and the start does not move as a glyph changes; a distance is still
  `fraction × length`. *Consequence:* where the top jumps between two peaks from one map to the next, `near=` fixes
  the start; `reverse` and a negative `scale` are the stated exceptions to the canonical form.
- **A negative `scale` mirrors; zero raises** (2026-10-29, approved by the owner; the review had asked for both to
  raise). *Reason:* zero collapses the outline, while no other verb mirrors a shape. *Consequence:* a mirror flips the
  winding, so it leaves the canonical orientation, as `reverse` does.

## The JSON format

The format: [formats.md](formats.md). All 2026-10-29, V1b, pending the owner's review.

- **The envelope: `version`, `params`, `inputs`, `drawings`, `root`**, nothing else, empty tables left out. *Reason:*
  each holds what a reader needs before the tree (the version to refuse, the Params to describe, the bytes to ask
  for, the shared drawings to resolve). The design size is a `stack`'s `size`, where the code gives it; an envelope
  copy could disagree with it.
- **One representation: a call is a one-key object, a drawing the array of its chain.** Every source, step, field, flow
  and expression is `{"<name>": <arguments>}`; a drawing is `[source, step, step...]`. *Reason:* the array reads in the
  code's order and nests only where the code passes a drawing as an argument; one rule covers every call and lets the
  registry add one by name; an unknown name is a single, precise error. Rejected: `{"node": "fill", "input": {...}}`,
  which reads inside out and nests one level per step.
- **Shared drawings by name.** A drawing used in several places is written once in `drawings` and referred to by name.
  *Reason:* the reference pieces use one drawing in up to six places; written out, a file repeats it and an edit must
  be made in each, and a drawing built from copies of copies grows exponentially.
- **A closed expression grammar:** `param`, `add`, `sub`, `mul`, `div`, `neg` and the four per-item numbers, nothing
  else. *Reason:* exactly what the code API can build, so a document stays data that any reader evaluates the same way
  and the schema can check; a function api.md lacks would be a format feature no code can write. Constant expressions
  are folded and the operands of `add` and `mul` ordered, because Python, Rust and JavaScript spell the same
  expression differently and must save the same JSON.
- **Inputs by name, bytes beside the document.** A font or map is declared by name and kind, with an optional SHA-256;
  the caller supplies the bytes. *Reason:* a document stays small, stays text, and fetches nothing; the digest makes the
  canonical text a complete cache key and catches the wrong mask. No embedded form: base64 grows a font by a third and
  is a second way to supply an input.
- **A canonical form: normalised tree, RFC 8785 text.** *Reason:* "the same drawing gives the same bytes" must hold
  for the saved file too, so it can address a cache; RFC 8785 is a published scheme with implementations in every
  language, and the normalisation (defaults left out, one path spelling, colours in lower case, sharing by equality)
  removes the differences between languages and between hand-written and saved files.
- **Defaults left out of saved documents** (question 4 in formats.md). *Reason:* short files, and the code API's callers
  already rely on defaults. *Consequence:* after the first release a default changes only with a new version.
- **Limits:** 16 MiB of text, depth 128 counted through references, 100,000 calls. *Reason:* a document may come from
  anyone; depth through references bounds the evaluator's recursion (small in WebAssembly), and 128 is the default of
  common JSON parsers (serde_json's among them).
- **Versioning by an integer major.** A version only grows (new calls, new optional arguments, new values); anything
  else is a new version; a reader refuses versions and names it does not document, naming them. *Reason:* one number
  says whether a reader can read a document; growth within a version keeps old documents valid, and an unknown name
  with the library's version in the message tells the user a newer library wrote it. Version 1 is a draft until the
  first release (no compatibility shims before 1.0).
- **`FormatError`** for every problem in a document's text, with a JSON Pointer; `ParameterError` stays for the
  caller's values (inputs, draw-time values). *Reason:* a bad document is fixed in the document, so a caller of
  `from_json` catches one kind, which says where.
- **A hand-written draft schema until V2.** *Reason:* the examples and known-bad documents need a checker before the
  Rust declarations exist; V2 generates the schema from them (Rust is the source of truth) and a test keeps the two in
  agreement until the draft goes.

## Output

- **SVG keeps every effect** (2026-10-28). A drawing written as SVG shows what it shows in pixels: shadows, halftone,
  knockouts. *Reason:* SVG is a deliverable (print, a vector editor, a web page), and a writer that leaves out
  what it cannot express changes the picture with no error: we found that skia-python 144.1's SVG canvas left out
  every shape drawn with a shadow filter in our test; we did not check other versions. *Consequence:* an effect
  without an SVG form is not added.
- **A fast halftone fill** (2026-10-28). Halftone is a fill (a screen: pitch, angle, dot size from a tone) clipped to a
  shape, not thousands of dot shapes through general booleans. *Reason:* built from shapes it cost about 500 ms a frame
  in the prototypes, too slow for a live preview; a regular screen can be generated and clipped directly.
  *Consequence:* halftone is a fill of its own, timed by the benchmark.

## Names and formats

- **Name `bullterrier-vector`** (2026-10-28): crate and PyPI distribution `bullterrier-vector`, import and Rust library
  `bullterrier_vector`, command `bt-vector`. *Reason:* the family's naming rule; people find a library through its
  description and keywords, not its name.
- **Alias `btvec`** (2026-10-29): the documentation imports the library as `btvec` in every language (`import
  bullterrier_vector as btvec`; `use bullterrier_vector as btvec;`; `import * as btvec from "bullterrier-vector"`).
  *Reason:* short to type, says what it is, and unlikely to clash with another library's usual alias.
- **Errors by kind** (2026-10-29): `BullterrierVectorError` > `ParameterError` (a value outside what is accepted, naming
  the parameter and value; also a `ValueError`) and `GeometryError` (geometry that fails, such as a boolean on an
  outline that cannot be resolved). *Reason:* a caller handles bad input and failed geometry differently: the first is
  fixed in the call, the second in the shapes.
- **Parameter descriptions: the library's own format** (2026-10-28), documented and versioned in `docs/formats.md`,
  with the same fields and conventions as other tools use (name, unit, range, default, meaning; cyclic or bounded).
  *Reason:* any tool can read it without depending on another package.
- **JSON for drawing files** (2026-10-28), with a JSON Schema. *Reason:* native in the browser and npm, read by every
  language; deep node trees are awkward in TOML.

## Engineering

- **Rust is the source of truth** (2026-10-28). Parameters are declared once in Rust; Python and JavaScript are thin
  layers, their stubs and the JSON Schema are generated. The version is the Cargo workspace's; the Python package reads
  it from there.
- **Determinism is a gate, not a hope** (2026-10-28). Clippy bans the platform's transcendental functions (`f64::sin`,
  `exp`, `powf`...; the portable `libm` crate replaces them), hash-ordered collections and the clock, and the core is
  built for `wasm32-unknown-unknown` in every gate run. *Reason:* bit-identical output across targets fails silently
  otherwise. *Consequence:* `clippy.toml` exists (clippy reads its `disallowed-*` lists from no other file).
- **Same output on every target is a differential test, not golden files** (2026-10-28). The same drawings are rendered
  natively and in WebAssembly and compared byte for byte. *Reason:* committed golden images go stale and get updated
  without review; a differential test needs no reviewed baseline.
- **Platforms** (2026-10-28). One abi3 wheel (every Python from 3.12) for Linux x86_64 and aarch64 (manylinux_2_28), macOS
  arm64 and x86_64, and Windows x86_64; the sdist, which builds the extension with rustup, covers every other platform.
  No musl wheel. *Reason:* no dependency sets a floor, so this is the conventional set; revisit when asked.
  *Consequence:* development and the full gate target macOS and Linux first. CI builds and installs a wheel on all five
  platforms; its gate proofs run on Linux, and its macOS and Windows legs run the Rust tests and the rest of the suite.
  Windows stays a reduced set until the first real CI run says otherwise.

## Open

Undecided questions and what they wait for are in [design.md, "Open decisions"](design.md#open-decisions), among them
stable placement over changing values (D7), which the prototypes raised. A question moves here, with its reason, once
it is decided.
