# Design decisions

Each decision once, with its reason. The design: [design.md](design.md).

## Product

- **A general-purpose library first** (2026-10-28). Written for anyone making graphics in code, not for one ecosystem.
  *Reason:* a library tied to one family's architecture limits its users; the family gains from a library others use.
  *Consequence:* no Bullterrier format, name or concept in the API; the family is mentioned under "Works with" only.
- **Code API first, JSON second** (2026-10-28). A drawing is written as chainable, typed code in Rust, Python and
  JavaScript; JSON saves and loads the same tree. *Reason:* developers express ideas in code; an API designed from a
  file format reads like configuration. *Consequence:* each API is designed (with examples written first) before the
  JSON format; a drawing written in code round-trips through JSON to an identical render.
- **Rust core, Python and WebAssembly bindings** (2026-10-28), not Paper.js or another JavaScript engine. *Reason:* one
  engine in the browser, on a server, in Python and inside a Rust video tool, with preview identical to render; the hard
  parts (shaping, glyph outlines, offsets, tracing) exist as Rust crates and are missing or weak in Paper.js.
  *Consequence:* the core does no IO, runs single-threaded, and uses one portable maths implementation.
- **Licence: MIT** (2026-10-28), not the Rust ecosystem's usual `MIT OR Apache-2.0`. *Reason:* the owner's choice:
  one short, widely known permissive licence, usable in any project with only the notice kept. *Consequence:* one
  `LICENSE` file; `deny.toml` admits dependencies whose licences ask no more of the package than MIT does.
- **Frame export in, timeline out** (2026-10-28). The library ships a small helper that loops a caller's function of
  time over frames and writes a transparent PNG sequence or clip; it has no timeline, keyframes, mixing or `onFrame`
  loop. *Reason:* nearly every animated use ends in "render these frames with alpha", and the frame loop, the alpha and
  the encoder settings are easy to get wrong; sequencing values over time is a different job, which other tools do, and
  a drawing that knew time could not be driven by a page, a slider or another program. *Consequence:* time and frames
  exist only in the helper, which knows nothing of what the caller's function does; the helper writes files and runs
  ffmpeg, so it lives outside the core.
- **Effects earn their place with a gallery piece** (2026-10-28). An effect is added together with a gallery piece that
  uses it. *Reason:* the bar is "worth posting", and a plausible effect can still look bad: the prototypes dropped
  halftone inside letters, outline echoes and a large wobble once they were seen in a piece. A test proves an effect
  works, not that it looks good. *Consequence:* the effects track is open-ended, one effect per batch;
  `docs/effects-roadmap.md` lists candidates, not promises.
- **Maps in, pixels out; colour effects that stay vector are the library's own** (2026-10-28). The library takes
  grayscale maps (width, height, bytes) and gives RGBA pixels; it never links a raster library at run time. Duotone,
  gradient maps, posterize, halftone dots and glitch slices on a drawing are operations on fill and stroke colours and
  on geometry, not SVG filters, and are kept consistent with a raster library's pixel versions by differential tests
  that use it as a development-only reference. *Reason:* plain data keeps both libraries usable alone (general-purpose
  first); SVG filters render differently between renderers and plotters ignore them, and a filtered drawing is no longer
  editable geometry; a reference implementation catches drift in the colour maths without coupling two release cycles.
  *Consequence:* the reference is a development dependency only, never one of the package; which library it is, and the
  seams: design.md, "Works with".

## The code API

- **Four ideas, one vocabulary** (2026-10-28). `Path` (a Bezier value), `Shape` (paths with a style and attributes),
  `Drawing` (a re-drawable tree of steps) and `Param` (a described number); the same verbs, with the same names and
  arguments, work eagerly on a `Path` and as a step on a `Drawing`, over one implementation. *Reason:* both scripting
  (measure and cut a path now) and re-drawable drawings (save, animate, run in a page) are wanted; two vocabularies
  would double what a user learns and what is built and tested. *Consequence:* a batch that adds a verb adds it to both;
  V1 settles the names.
- **Expressions on parameters, no callbacks inside a drawing** (2026-10-28). Arithmetic on `Param`s builds an expression
  that saves with the drawing; per-item numbers (by index, seeded random) are values of the same kind. A drawing never
  holds a function of the caller's. *Reason:* a drawing must save to JSON, load in another language, and render in a
  browser or on a server that runs no user code; a callback can do none of these. *Consequence:* callbacks are fine
  when scripting with `Path`, which computes at once and is not saved.

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
- **Parameter descriptions: the library's own format** (2026-10-28), documented and versioned here, with the same fields
  and conventions as other tools use (name, unit, range, default, meaning; cyclic or bounded). *Reason:* any tool can read
  it without depending on another package.
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
