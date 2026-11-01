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
- **Frame export details** (2026-11-01). `round(seconds × fps)` frames at `t = i / fps` from 0, so `t = seconds` is
  never drawn; a count below one frame is a `ParameterError`; the `#` run is a minimum width; the folder is created,
  files overwritten, frames of a longer earlier run left alone; a clip is encoded to a hidden sibling and renamed.
  *Reason:* a clip of `seconds` that loops should not repeat its first frame; a rename keeps an old clip whole when the
  scene fails midway; a function that deletes files in a folder it was only asked to write into is a hazard. ffmpeg is
  driven as an argv list (no shell) with raw RGBA on stdin, so no temporary frames are written for a clip. VP9 uses
  constant quality 15, the "visually lossless" end, as a clip is for editing over footage. *Consequence:* a clip's alpha
  is checked by decoding it, not byte for byte; ffprobe shows a `.webm` as `yuv420p` with an `alpha_mode` tag.
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
  WebAssembly the bindings give a pointer, a length and a generation counter, and Python typed copies (no `unsafe` in
  the bindings; zero-copy waits for the X2 spike to show a need).
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
- **A stack writes `order` only on shapes that carry none; `Drawing(x)` saves as `x`** (2026-10-29, the owner, from the V1b
  review). *Reason:* stacking a text must not undo its per-letter order, and a wrapper that changes nothing must not
  change the saved drawing; Rust converts a path where Python and JavaScript stack it, and all three must save alike. `repeat`
follows the same rule for `order`.
- **Transforms change geometry and `origin` only** (2026-10-31, from the V1 review). *Reason:* `shapes()` returns baked
  geometry, so `offset` and `scale` commute in meaning and a non-uniform `scale` needs no stroke rule; style values are
  design pixels. *Consequence:* to scale a stroke with its shape, give it the same Param (`width=24 * pop`).
- **`miter_limit` is a parameter of `stroke`, `offset` and `outline`, default 4** (2026-10-31). *Reason:* a miter without a
  limit is a hidden tunable (CLAUDE.md, rule 1); 4 is SVG's default, so the SVG output needs no translation.
- **A shape that becomes empty stays in the list** (2026-10-31). *Reason:* removing it would shift `index`, `order` and
  per-item lists in the middle of an animation.
- **On Rust `Path`, a verb that takes a plain number returns a `Result`** (2026-10-31). *Reason:* the finite-number rule is
  checked where the number enters; a checked newtype for every argument would cost more than a `?`.
- **`Style` holds the paint as drawn** (2026-10-29, the owner). `None` is not painted, `fill("none")` counts as set,
  `shapes()` resolves the default paint. *Reason:* a shape read back and stacked again must draw as it did; with
  `None` meaning both "unset" and "none", a `fill("none")` shape came back filled black.
- **A per-item number where an argument is read once is an error** (2026-10-29, the owner). *Reason:* a value per shape
  means nothing where there is no shape yet (`trace`'s `levels`, a field's points), and taking the first shape's value
  would hide the mistake.
- **Plain numbers are checked when the call is made, finite and whole within ±(2⁵³ − 1)** (2026-10-29). An expression
  without Params or per-item numbers is a plain number. *Reason:* a mistake surfaces at the line that made it; 2⁵³ is
  where a 64-bit float stops holding every whole number, and JSON readers in every language agree up to it.

## The JSON format

The format: [formats.md](formats.md). All 2026-10-29, V1b, pending the owner's review.

- **The envelope: `version`, `params`, `inputs`, `drawings`, `root`**, nothing else, empty tables left out. *Reason:*
  each holds what a reader needs before the tree (the version to refuse, the Params to describe, the bytes to ask
  for, the shared drawings to resolve). The design size is a `stack`'s `size`, where the code gives it; an envelope
  copy could disagree with it.
- **`params` is an array in first-use order** (the owner, from the V1b review). *Reason:* the order is what
  `parameters()` returns and what a slider panel shows; an object's keys are sorted in canonical form, which would lose
  it. A document in another order is refused rather than reordered, so the file says what the drawing returns.
- **One representation: a call is a one-key object, a drawing the array of its chain.** Every source, step, field, flow
  and expression is `{"<name>": <arguments>}`; a drawing is `[source, step, step...]`. *Reason:* the array reads in the
  code's order and nests only where the code passes a drawing as an argument; one rule covers every call and lets the
  registry add one by name; an unknown name is a single, precise error. Rejected: `{"node": "fill", "input": {...}}`,
  which reads inside out and nests one level per step.
- **Shared drawings by name, defined on the merged graph.** Equal drawings are merged by a structural hash, and a
  drawing is written once in `drawings` exactly when two or more calls build on it or take it. *Reason:* the reference
  pieces use one drawing in up to six places; written out, a file repeats it and an edit must be made in each, and a
  drawing built from copies of copies grows exponentially. Counting on the graph, never on the expanded tree, gives one
  answer for a shared prefix and keeps every whole-tree computation linear in the document.
- **A closed expression grammar:** `param`, `add`, `sub`, `mul`, `div`, `neg` and the four per-item numbers, nothing
  else. *Reason:* exactly what the code API can build, so a document stays data that any reader evaluates the same way
  and the schema can check; a function api.md lacks would be a format feature no code can write. Constant expressions
  are folded and a literal operand of `add` and `mul` written first, because Python, Rust and JavaScript spell the same
  expression differently and must save the same JSON.
- **Inputs by name, bytes beside the document; an input is its bytes.** A font or map is declared by name and kind, with
  an optional SHA-256; the caller supplies the bytes; equal bytes are one input. *Reason:* a document stays small, stays
  text, and fetches nothing; the digest makes the canonical text a complete cache key and catches the wrong mask. No
  embedded form: base64 grows a font by a third and is a second way to supply an input.
- **A canonical form: normalised tree, RFC 8785 text.** *Reason:* "the same drawing gives the same bytes" must hold
  for the saved file too, so it can address a cache; RFC 8785 is a published scheme with implementations in every
  language, and the normalisation (defaults left out, one path spelling, colours in lower case, sharing by equality)
  removes the differences between languages and between hand-written and saved files.
- **Defaults left out of saved documents** (question 4 in formats.md). *Reason:* short files, and the code API's callers
  already rely on defaults. *Consequence:* after the first release a default changes only with a new version.
- **Limits:** 16 MiB of text, depth 128 counted through references, 100,000 calls; schema bounds on the literals that
  set work; counters checked while drawing (shapes, segments, field samples, copies, glyphs). *Reason:* a document may
  come from anyone; depth through references bounds the evaluator's recursion (small in WebAssembly), and 128 is the
  default of common JSON parsers (serde_json's among them); work cannot be estimated beforehand (a shared drawing
  stacked twice, sixty times over), so it is counted as it is done.
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
- **SVG coordinates are baked, with a viewBox and isolation** (2026-10-31). `to_svg` moves each path into output pixels
  and multiplies stroke widths by the scale; the root has `viewBox="0 0 w h"` and `style="isolation:isolate"`, and there
  is no `transform`. *Reason:* the check that a drawing at 2x has exactly twice the numbers of 1x needs the numbers in
  the file; the isolation makes `mix-blend-mode:multiply` blend with the picture, as the pixels do, and not with the
  page around it. *Consequence:* a viewer cannot change the scale without re-drawing.
- **SVG numbers are the shortest round-trip text** (2026-10-31). Every number in the SVG goes through the formatter
  of path data, never rounded to a number of decimals. *Reason:* one formatter for every number (CLAUDE.md rule 4);
  doubling a power-of-two scale changes no rounding, so 2x is exactly twice. *Consequence:* a document is longer than
  one rounded to 3 decimals; the plotter output (V15) may round, as a pen has a resolution.
- **One view for every output** (2026-10-31). The design frame is scaled by `min(width / frame width, height / frame
  height)` and centred; a drawing with no frame is at scale 1 from the top left; content outside the frame is not
  clipped. *Reason:* SVG, pixels and the mesh must place a drawing the same way, so the rule lives in one module and
  none of them decides it. A viewer's `preserveAspectRatio` does the same for an SVG with a viewBox, and an
  unclipped frame is what an SVG viewport does.
- **Rasteriser: `vello_cpu`** (2026-10-31), with `kurbo`, both pinned exactly; `vello_cpu` is used only in
  `output/raster.rs`, `kurbo` also in `output/outline.rs` and `output/mesh.rs`. *Reason:*
  measured against the alternative, tiny-skia: an edge pixel's coverage was within 0.5/255 of the closed-form area
  (tiny-skia: 25/255), and 200 mixed shapes at 1080p took 27.5 ms native and about 39 ms in WebAssembly, one thread.
  *Consequence:* pixels are required to look the same on every target, not to match byte for byte (native against
  WebAssembly differed by one level on 0.002% of values); what we compute ourselves (placement, widths, SVG, the
  straight-alpha conversion) stays exact. The crate panics when a row is padded past 65,535, which a width above 65,408
  is, so a pixel side stops at 65,000.
- **The rasteriser is given straight lines only** (2026-10-31). Curves are flattened by us, once, in design pixels, with
  the walk that measuring uses (0.01 design pixels), moved by the view, and a stroke is expanded by `kurbo` to the same
  flatness in output pixels (0.01 times the scale) and flattened again, before `vello_cpu` fills the outline. It also
  draws the disc or square of a zero-length sub-path under a round or square end, which `kurbo` leaves out. *Reason:*
  `vello_cpu` flattens at a fixed quarter of a pixel, which took 2.5% off the area of a circle of radius 10; with our
  flattening it is within 0.02%, and measure, pixels and the mesh agree on one polygon. Flattening is charged to
  `MAX_SEGMENTS` after each curve, and each shape's stroke outline has a count of its own, since a round join of a wide stroke
  makes thousands of edges from one vertex (a drawing of 20,000 stroked circles makes 10 million in all). *Consequence:* a drawing shown 25 times larger than its frame is flattened more
  coarsely than the crate would have, since the flatness is in design pixels; the stroker can lose a sliver of the inside
  of a sharp turn after a segment shorter than half the stroke width (the mesh's strokes come from the same outline);
  200 mixed shapes at 1080p took 13.3 ms instead of 10.8 with opacity (3.7 instead of 7.6 without), the stroke expansion
  being the difference.
- **Group opacity, sRGB blending, straight alpha** (2026-10-31). A shape's fill and stroke are drawn into one layer and
  the shape's opacity and blend apply to it; colours blend as sRGB values; `to_rgba` returns straight alpha. *Reason:*
  that is what SVG's `opacity` on a path and the browsers do, so the SVG and the pixels agree, and a stroke over a fill
  at half opacity is not darker where they overlap. The conversion from the rasteriser's premultiplied bytes is ours and
  rounds to nearest, so it does not depend on the crate.
- **PNG: the `png` crate, fixed settings** (2026-10-31), pinned exactly and used only in `output/png.rs`; the encoder
  settings (fast DEFLATE, the crate's adaptive row filter) are two named constants there. *Reason:* a PNG encoder is
  reuse, not a differentiator, and the crate is pure Rust and builds for WebAssembly; the settings are fixed so that one
  version of the crate gives one file for one picture, and the fast mode costs a few percent of file size for a large
  saving in time (the crate's own description). *Consequence:* the bytes of a PNG may change between versions of the crate (the pixels inside
  do not: `to_png` is tested by decoding it, with a decoder written from the specification, to exactly `to_rgba`), so no
  golden file holds one; the crate brings `flate2`, `fdeflate`, `crc32fast` and two versions of `miniz_oxide` (a
  `cargo deny` duplicate warning, no extra licence).
- **Pixel limits** (2026-10-31). A pixel side is 1 to 65,000 and at most 2^27 pixels (512 MiB of RGBA); a device
  coordinate or stroke width beyond 10 million raises `ParameterError` naming the shape. *Reason:* the canvas of the
  crate is 16-bit, its arithmetic is f32, and a 65,000-pixel square would take 16 GiB; no clamping, so a caller learns
  that the picture is not what they asked for. SVG keeps the 100,000 of the design frame, and so does the mesh, which has no canvas.
- **Mesh: `lyon_tessellation` for fills, `kurbo` outlines for strokes** (2026-10-31). The nonzero fill of the
  flattened path, and of the stroke's outline (the same `kurbo` expansion the pixels use), is tessellated by
  `lyon_tessellation` (pinned exactly, `std` off) in design pixels, and the vertices are moved through the view
  afterwards. *Reason:* measured on a path with a hole and a self-overlapping pentagram, lyon's nonzero fill made 66
  triangles and was wrong at none of 39,686 sampled points, no point covered twice, while lyon's own stroker
  covered 338 sampled points twice where a stroke overlaps itself, which a translucent colour shows; `kurbo`'s
  outline filled the same way was right everywhere (111 triangles for a zig-zag). Lyon computes in f32, so working
  in design pixels keeps its numbers the drawing's own, and a power-of-two scale then gives exactly twice the
  positions at twice the size. *Consequence:* a shape's fill and stroke are two layers, so at opacity below 1 their
  overlap shows darker than in the pixels' one group; a vertex is charged to `MAX_SEGMENTS` like an edge, and to
  `MAX_TESSELLATION_WORK` for the time it takes: lyon's sweep keeps every edge it crosses alive, so a star polygon of
  1,000 corners (237,000 vertices) took 1.6 s and one of 3,200 took 24 s, and a comb of 20,000 teeth, which crosses
  nothing, 3.6 s, though all are far below 10 million vertices. Measured in release builds, vertices times the
  most edges a horizontal line crosses (known before tessellating) cost 1 to 4.3 ns a unit in a star, a stroked star, a
  comb and a zig-zag alike, so a budget of 250 million units bounds a draw to about a second whatever the outline; the
  pixels (`vello_cpu`) draw the same inputs in under 0.03 s, so the limit belongs to the mesh. A limit on edges alone
  would refuse a circle of radius 2,000 (1,024 edges, nothing crossing) before a star polygon that takes a second (900). *Round strokes:* kurbo's outline with arcs for round joins and ends, filled
  by lyon, covered a patch of 25 of 20,000 random strokes three times, which a translucent colour or a multiply shows;
  a disc added over a bevel outline left holes (9 of 20,000) because lyon fails where a polygon overlaps one tangent to it.
  The stroke is now the bevel and butt-end outline plus, beside it and running the same way round, a circular segment
  where a corner turns enough for the bevel to be over 0.01 short of the arc and a half disc at each open end: none of
  20,000 random strokes is covered twice or missed, with real corners, whole-number corners or miter and bevel joins; with
  corners on a coarse grid, where edges coincide, 1 of 20,000 (56 before), also seen with plain bevel and miter joins.
  A smooth curve gains no triangles, and the scenes measured gained under 6%.

- **Native against WebAssembly: exact for what we compute, a measured tolerance for pixels** (2026-11-01). The
  differential test (`poe wasm-test`) draws 44 drawings natively through the Python extension and in WebAssembly through
  Node, from the same JSON documents: the SVG text and the three mesh buffers (hashed with `hashlib` on one side and
  `node:crypto` on the other), the generation and the lengths are compared exactly; the pixels are compared as the
  premultiplied colour and the alpha (the values drawn), and may differ by 2 levels on 0.02% of the values of a draw. A
  PNG is decoded with `node:zlib` and must hold exactly the pixels of `toRgba`. *Measurement:* on 48 draws (3,450,660
  channel values), Apple silicon NEON natively against WebAssembly simd128, 4 values differed, none by more than 1
  level (2 as straight alpha, where a division by a small alpha magnifies a rounding), at most 0.005% of one draw; the
  tolerance is twice the level and four times the fraction. A larger difference fails and prints the count and the first
  pixel. *Reason:* the rasteriser crate chooses its arithmetic by target, so equal bytes are not promised, and the
  buffers and text we write are our own arithmetic, so a difference there is a defect. The mesh buffers matched exactly
  on the machine measured. *Consequence:* x86 and a real browser were not measured; the tolerance is to be revisited by
  measurement if CI on another architecture fails it, not loosened.
- **A `Mesh` is lent to a call as an argument, not in an options object** (2026-11-01). JavaScript refills a mesh with
  `drawing.toMeshInto(mesh, w, h, values)` and gets a new one from `toMesh`; `wasmMemory()` returns the memory the
  buffers are in. *Reason:* wasm-bindgen lends an exported object to a call only as an argument of its own; read out of
  an options object it would be taken over and the caller's object invalidated, which would lose it when the call
  throws. The memory is a function because the module's own export of it is called `memory`.

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

## Geometry (V2a)

- **No curve crate** (2026-10-31). Curves, measuring, hit-testing and SVG path data are our own code on `libm`. *Reason:*
  `clippy.toml` bans the platform's transcendental functions only in this workspace's code; a curve crate's maths
  would reach the output unchecked and leave the native-against-WebAssembly differential test (V3) as the only guard
  for bit-identical bytes. The surface needed is small (flattening, arc length, nearest point, nonzero winding).
  *Consequence:* the same maths in every language; booleans and offsets (V8, V9) face the same question.
- **Arcs are cubics, one per quarter turn at most** (2026-10-31). An SVG arc is converted from endpoint to centre form
  (SVG 1.1, F.6.5), then cut into `ceil(sweep / 90°)` equal pieces, each a cubic with handles `4/3 tan(piece / 4)`.
  A circle is four cubics (kappa `0.5522847498307936`), straying from the circle by at most 2.7e-4 of the radius.
  *Reason:* one rule, so reading path data and `Path.circle` agree, and a path holds exact coordinates.
- **One flattening serves every measure** (2026-10-31). Length, `point_at`, `tangent_at`, `locate` and `slice` read one
  walk over the same flattened edges, so they agree by construction. A curve is flat when its control points lie
  within `FLATNESS` (0.01 design pixels, independent of the output size) of the evenly spaced points of its chord, not
  merely near the chord line. *Reason:* that bounds the pace too: the curve at parameter `t` is within `FLATNESS` of
  the edge at fraction `t`, so a position along an edge is a position along the curve. A distance-only test would let
  a position drift along the curve.
- **`tangent_at` leaves, `locate` takes the first, `slice` has a direction** (2026-10-31). At a corner the tangent is
  the direction leaving it, since a draw-on or a dash starts there; a position on the boundary between sub-paths
  belongs to the later one; `locate` of an equally near point gives the smallest position, and a closed start is 0,
  never 1. `slice(start, end)` with `start` above `end` is a `ParameterError`, not a hidden reversal (`reverse`
  exists); `slice(a, a)` is a lone start, so the result is still a path at that point; a closed sub-path wholly
  inside the piece keeps its close flag, so `slice(0, 1)` of an outline is the outline. *Reason:* each rule is the one
  a draw-on or a dash march needs to give the same picture at 0, at 1 and at a corner, with no special case in the
  caller.
- **The `Error` shape** (2026-10-31). `Error::Parameter { parameter, message }` and `Error::Geometry { verb, message }`
  hold owned strings, and the enum is `#[non_exhaustive]`. *Reason:* a drawing prefixes the place of a failure
  (`step 3 (rotate).angle`), which `&'static str` fields cannot hold, and `Error::Format` arrives with the JSON
  loader (V2c) without breaking a caller's `match`. The core owns every range check; the bindings only choose the
  exception class.
- **Shortest round-trip numbers in path data** (2026-10-31). A number is written as the shortest text that reads back
  as the same value, never with an exponent, and `-0` as `0`, so `Path.from_svg_path(p.to_svg_path()) == p` exactly,
  at any size. *Reason:* a fixed number of decimals (the skeleton's six) loses precision on small paths and bloats
  large ones.
- **A coordinate is within ±1e100** (2026-10-31, from the V2a review). Every coordinate a path holds, given or worked
  out (a transform, an arc, a constructor, `map_points`), lies within ±1e100 design pixels, and a query point too;
  beyond that is a `ParameterError` naming the value, where the number enters. *Reason:* measuring squares
  differences, so from about 1.4e154 the length overflowed to infinity, `slice` panicked and `point_at(0)` was not
  the start; 1e100 leaves room for squares, sums of squares and sums of lengths over any number of segments, and no
  drawing comes near it. *Consequence:* a result never holds a NaN or an infinity, with no check after the fact.

## Drawings (V2b-1)

- **The node contract** (2026-10-31). A node is a crate-private unit struct whose declaration (name, role, arguments) is
  static data; it evaluates the whole list, and a shape-by-shape node loops through one helper that numbers the shape
  and places its errors. It reads an argument only through an accessor that evaluates the expression for that shape,
  applies the argument's declared range and names the argument in the error. Its context is the place, the Param values
  as that place sees them and the counters of the draw: no clock, no caller state. *Reason:* each range is declared
  once and checked in one place; a node cannot leave a default method unimplemented. Kinds with no consumer yet (whole
  numbers, per-item numbers, read once) are declared by the first node that needs them, so no variant is dead code.
  The contract is written in the module docs of `node.rs`.
- **Registry vocabulary and what is generated** (2026-10-31). `verbs()` lists `Verb`s of `Arg`s with a kind, a default,
  whether the value is read per shape or is a literal, and a range (`Domain`); a per-call check covers a rule across
  arguments. From the declaration come the call-time checks, the draw-time checks, the evaluation and the list. The
  typed builders (`Drawing::translate`...), the binding methods and stubs are hand-written one-liners, and V2c generates
  the schema entry. *Reason:* thirteen verbs, three of them geometry, do not pay for a macro or a build script; the
  contract table fails a registered verb with no row and a `Number` argument with no range rows, so a missing surface
  shows. *Consequence:* revisit at V3 or when a verb needs more than a one-liner; V2c's schema agreement test is the
  second guard.
- **Counters charge creation** (2026-10-31). A source charges the shapes it makes and a stack its whole list (a drawing
  used twice is charged twice), against `MAX_SHAPES` (1,000,000); steps that change or drop shapes charge nothing. The
  charge comes before the allocation. Drawings nested more than `MAX_DEPTH` (128, the document limit, also the limit of
  an expression) deep hold an error, and a chain of steps costs no depth: it is walked in a loop and dropped in one. The
  other four counters arrive with the first step that produces what they count. *Reason:* the counters bound memory
  blow-up (a stack of itself 60 times), not the length of a chain, and the cache (V2b-2) serves repeats without
  charging. An expression is capped at `MAX_EXPR_NODES` (10,000, a shared part counted each time it is used, as
  evaluation reaches it) beside its `MAX_DEPTH`, checked at the call without evaluating: `e = e + e` 28 times is 2^29
  nodes. formats.md names no such limit yet.
- **Order of a stack** (2026-10-31). A stack writes `order` on the shapes that carry none, spread 0..1 over those shapes
  in list order (one shape: 0); a path or shape source writes none, so `Drawing(path)` equals the path. *Reason:*
  api.md says both "the shapes that carry none" and "over the shapes a generator made"; this reading lets a text keep
  its per-letter order beside a rectangle.
- **Style `painted`** (2026-10-31). A private flag says a fill or a stroke was set, `fill("none")` included. `shapes()`
  resolves the default paint of an unpainted shape and sets the flag, so a shape put back into a stack draws as it did.
  *Reason:* `None` could not mean both unset and none (see "`Style` holds the paint as drawn").
- **The place of a step** (2026-10-31). The source of a chain is `source (stack)`, the steps after it are numbered from 1
  (`step 1 (translate)`), the drawings a step takes are `item 1`, `item 2`, ..., joined with ` / `, and an argument
  follows a dot: `source (stack) / item 2 / step 1 (translate).dx`; the message of a per-shape error starts with
  `shape 7: ` (from 0). *Reason:* a stack is always the source of its chain, so its place has no number, and the
  reader can follow the path from the outside in.
- **`with_values` fixes for the steps before it** (2026-10-31). The steps before it and the drawings they take see its
  values, wrapped for a cyclic Param and checked against the Param's range at the call; a step after it reads the
  Param as before, which brings it back into `parameters()` at its place in the order first used. A fixed Param is not
  the caller's to give and still counts for the one-name rule. *Reason:* api.md and formats.md say "for the drawing
  before it"; a drawing used in two places is fixed at one of them only. A value given when drawing
  is checked against its Param's range (NaN and infinities refused, a cyclic one wrapped) before anything is drawn,
  whether or not a step reads it with shapes, so a value out of range never goes unnoticed; where a step does read it
  the error names the step and the shape. A value for a Param the drawing does not read is refused as unknown. A list's
  length is checked where a step reads it.
- **`only` and `without` refuse an attribute only when there is something to ask** (2026-10-31). A name that is not an
  attribute, or a value its attribute cannot hold, is refused at the call; a known name that no shape of a non-empty list
  carries is refused when drawn, naming it. An empty list passes. *Reason:* a live text of no letters must not
  raise from the next `only(char="O")`.

## Drawings (V2b-2)

- **Merging by equality, cache per family, keep what a redraw starts from** (2026-10-31). A result is kept under the
  step's structure (its call and the results it builds on, as numbers) and the values of the free Params it reads, so
  two drawings built apart that are equal share entries. A drawing and every drawing derived from it by a step share one
  cache, and a stack shares its first item's; `Drawing::stats()` reports the last draw of that family. The cache keeps
  only the results a redraw can start from: what the draw returned, a result whose next step (or the stack that takes it
  as an item) reads other Params free than it does, and a result the draw used twice. A result that only feeds a step
  reading the same Params is made again with that step and is dropped, so a chain of steps that read no Param keeps one
  result, not one per step. After each draw, a failed one included, the entries it did not use are dropped.
  *Reason:* changing one value then computes only the steps that read it, and a drawing used twice is computed once;
  keeping every step's result held a chain's length times its shape count (2,000 shapes through 1,000 steps took 927 MB
  against 18 MB without a cache). *Consequence:* the memory kept is at most one result per place a Param starts being
  read, so a chain whose every step reads a Param of its own still keeps every result; a draw with alternating values
  recomputes what the other left out.
- **Words are read by the core; a step is raised at its call in Python** (2026-10-31). The core reads a colour, join,
  cap, blend and unit from its word (`parse`) and writes it back (`name`), so a wrong word is the same
  `ParameterError` in every language. The Python bindings check the chain's error after each step and raise it there,
  while Rust reports it at the first draw. A `Path` verb refuses a `Param` or an expression with `ParameterError`
  telling the caller to wrap the path in a `Drawing`. *Reason:* Python points at the line that is wrong; the bindings keep
  no range check of their own.

## JSON (V2c)

- **A small strict parser in the crate, no JSON dependency** (2026-10-31). `json/parse.rs` reads the text into a tree
  of its own and refuses what I-JSON refuses: a duplicate key, a lone surrogate, a number beyond a 64-bit float,
  a leading zero, `NaN`, `Infinity`, a trailing comma, bytes after the value. It counts nesting as it reads and stops at
  `MAX_DEPTH`, so no hostile document recurses past it, and the byte limit is checked before the first byte is read.
  *Reason:* `serde_json` keeps the last of two equal keys unless a visitor is written for it, parses floats to within
  one ulp unless a feature is on (the same text must give the same bits on every target), and brings `serde`,
  `itoa`, `ryu` and `memchr` into a core that must stay small and bit-exact. The parser is about 250 lines, and the
  rules the format needs (a pointer for every error, limits while reading) are part of it. *Consequence:* the number
  reader hands the digits to `f64::from_str`, which is correctly rounded on every target, and the writer lays out the
  shortest digits of `{:e}` by the rules of RFC 8785.
- **Where the loader's checks run** (2026-10-31). `from_json` refuses a text over `MAX_DOCUMENT_BYTES` before reading
  a byte, counts calls while it reads (`MAX_CALLS`), and the parser stops at `MAX_DEPTH` levels of text, so a deeper
  text is reported as the nesting limit even when a reference check would have failed first. What the schema states
  value by value (version, names, types, arguments, the range of a literal, the rules of an expression) is found
  while reading, before `drawing-declared`; the listed checks then run in their order. `with-values` needs the calls
  made, so the drawing is built twice over the checked document: first with no call checked (a `with_values` that
  names a Param the drawing before it does not read is the error), then each call's own checks (`call-checks`), so the
  first failure of the first kind anywhere is reported before the first of the second kind. *Reason:* the
  document lists the order, and a check that needs the drawing must not run earlier than one that does not.
- **A value a call refuses** (2026-10-31) is a `FormatError` at the pointer of the value with the call's own message
  (`must be above 0, got 0`), without the place of the code API (`step 3 (stroke).width`). *Reason:* the fix is in the
  document, which has no steps.
- **Depth through references** (2026-10-31). A `{"drawing": name}` call counts as the array it names, so each level of
  reference adds that array's level; each shared drawing is measured once. A document written canonically (shared
  drawings only where two calls use them) can nest more or less than a hand-written spelling of it, so `to_json` can
  refuse a drawing that `from_json` read, with `ParameterError` naming `MAX_DEPTH`.
- **Shared names follow the walk** (2026-10-31). `d1`, `d2`... are given in the order the walk completes a shared
  drawing: a call is reached after the drawings it builds on and takes. In formats.md's example (`a` and `b = a.rotate`)
  this is `a`, then `b`.
- **A selection of nothing** (2026-10-31). `only` and `without` accept an empty list of values (it matches no shape,
  as in code), and `to_json` writes `[]`; the draft schema's `minItems: 1` there was loosened with the generated schema.
- **A literal divisor of 0 has no spelling** (2026-10-31): the schema rejects it, so `to_json` refuses a drawing that
  has one (`ParameterError`) instead of writing a document `from_json` would refuse.
- **No input is read yet** (2026-10-31). No registered kind of argument reads an input, so a document that declares one
  fails `input-used`, and `from_json` takes no `inputs`; both arrive with the first call that reads one.

- **`MAX_PARAMS` is 1,000** (2026-10-31). Building a step costs time in proportion to the Params the drawing already
  reads, so 8,000 distinct Params stall `from_json` for seconds (4.5 s in a debug build) inside `MAX_CALLS`. A
  document's `params` past 1,000 entries is a `FormatError` at the first one past it (`limits`), and a drawing built in
  code that would read a 1,001st name raises `ParameterError` at that call. *Reason:* a limit counted in what the work
  grows with, enforced where the Params are declared and where they are read, beats a timer, which the core may not
  have. No drawing a person wrote has a tenth of this.
- **The schema is generated by a public `drawing_schema()`** (2026-10-31). Rust `drawing_schema() -> String` and Python
  `bullterrier_vector.drawing_schema() -> str` return the schema as compact JSON text. It is built from `verbs()`: each
  call's name, role, summary, arguments with kind, default and domain, set into a fixed envelope (the document's keys,
  Params, inputs, expressions, the words every call spells the same: `json/schema_fixed.json`). *Reason:* editors,
  validators and the CLI (V18) need the schema, and a crate-private generator would leave them the draft, which
  stops describing the library as calls are added; the function is one name and returns text. The generator has no list
  of calls, and a new `Kind` or `Domain` does not compile until it has a schema. *Consequence:* the agreement with the
  draft is a Python test over the same documents (`tests/test_drawing_schema.py`), which also fails on a tampered
  output; the draft stays the specification of the calls not yet registered.
- **Where the code and the draft differed, the code was right** (2026-10-31): an empty list for `only` and
  `without` matches no shape (the draft's `minItems: 1` is gone, and the loader now reads `[]` for `origin` too);
  `origin` is an attribute to select by; whole attributes are at most `u32::MAX` (the type that holds them) and a
  `char` is one character; a coordinate is within 1e100 (`Domain::Coordinate`); a stroke colour is never `"none"`
  (declared by the new `Domain::Painting`, so the schema states what the call check refuses, and the check is the
  domain's, so the schema and the check cannot drift). The draft was changed to match.

## Engineering

- **Rust is the source of truth** (2026-10-28). Parameters are declared once in Rust; Python and JavaScript are thin
  layers, their stubs and the JSON Schema are generated. The version is the Cargo workspace's; the Python package reads
  it from there.
- **Determinism is a gate, not a hope** (2026-10-28). Clippy bans the platform's transcendental functions (`f64::sin`,
  `exp`, `powf`...; the portable `libm` crate replaces them), hash-ordered collections and the clock, and the core is
  built for `wasm32-unknown-unknown` in every gate run. *Reason:* bit-identical output across targets fails silently
  otherwise. *Consequence:* `clippy.toml` exists (clippy reads its `disallowed-*` lists from no other file).
- **The browser package is wasm-bindgen with a pinned command and a SIMD build** (2026-10-31). `crates/wasm` is the only
  crate that knows `wasm-bindgen` (`deny.toml`, as for PyO3); the `wasm-bindgen` command is pinned in
  `[workspace.metadata.tools]` and `rust-tools` fails when its version is not that of the crate in `Cargo.lock`. The
  package is built with `RUSTFLAGS=-C target-feature=+simd128`, set by the `wasm-test` task for that build alone, never in
  a config file or for a native build. A JavaScript error is an `Error` whose `name` is the kind, as wasm-bindgen cannot
  define subclasses. *Reason:* glue made by another version than the crate fails only when it runs; measured, SIMD makes
  the rasteriser 10-15x faster than scalar WebAssembly, and current browsers have it. *Consequence:* the gate
  needs Node and the command (`poe setup` installs it); a browser without SIMD cannot load the package.
- **Same output on every target is a differential test, not golden files** (2026-10-28). The same drawings are rendered
  natively and in WebAssembly and compared byte for byte. *Reason:* committed golden images go stale and get updated
  without review; a differential test needs no reviewed baseline.
- **Bounded depth, not a bigger stack** (2026-11-01). Whatever a caller can build has its depth limited where it is
  built, so no walk recurses over unbounded depth: drawings nested past `MAX_DEPTH` (128) are refused when built, the
  parser refuses a document nested past it before the loader recurses, and an expression past 128 levels becomes a node
  without operands (operators never fail, so a loop can build one; the call that uses it is refused). The smallest
  supported thread stack is **256 KB** (a Python thread gets 512 KB on macOS); measured need in a release build: 128
  nested drawings about 52 KB, the deepest document `from_json` reads 96 to 128 KB. In WebAssembly the engine throws a
  catchable `RangeError` below about 0.25 MB. *Reason:* an overflow is a segfault in the caller's process. A growing
  stack (`stacker`) was rejected: it cannot grow the WebAssembly engine's stack, loses CPython's overflow guard on 3.14
  and later, adds `unsafe` and a C compiler to the sdist build, and trades a bounded stack for unbounded memory.
  *Consequence:* `tests/test_stack_depth.py` and `crates/core/tests/expr.rs` run the deepest structures on the minimum
  stack; a new node or reader adds its structure to them.
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
