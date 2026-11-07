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
  corners on a coarse grid, where edges coincide, 1 of 20,000 (56 before), also seen with plain bevel and miter joins. A smooth curve gains no triangles, and the scenes measured gained under 6%.
  Strokes with several sharp turns (two or more over about 100 degrees) were still covered twice by a small area in about 1 of 10,000 random round strokes (never missed): the rings are correct, and lyon emits two
  triangles on one base where the long offset edges of neighbouring segments run nearly alongside each other. No ring
  construction fixed it (a horizontal sweep only moves the failures; one convex piece per segment and corner cut them 3 to 4
  times at 7% more triangles). So the rings of every stroke go through `Path::resolved`, the boolean engine, before the fill:
  none of 20,000 random strokes (30 million points, all joins and ends) is then covered twice or missed. The engine's time
  grows with the pairs of edges whose boxes overlap, which no edge count bounds, so they are charged to
  `MAX_TESSELLATION_WORK` at 650 units a pair (measured 1.4 to 4 microseconds a pair in release) before it runs. *Cost,
  release, one thread:* `to_mesh` of the reference scene went from 17.6 to 61 ms (3.4 times), 200 stroked circles from 40
  to 157 ms (3.9 times), 200 stroked stars from 6.7 to 22.5 ms; a fill is unchanged. A cheaper route (resolving only
  strokes that fold back, found by a geometric test) is left open.

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
  the parameter and value; also a `ValueError`) and `GeometryError` (geometry that fails, such as measuring an
  empty path). *Reason:* a caller handles bad input and failed geometry differently: the first is
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
  *Consequence:* the same maths in every language; booleans took a crate nonetheless ("Booleans (V8)"), offsets (V9)
  face the same question.
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

## Fonts (V5a)

- **`read-fonts` and `harfrust`, pinned exactly, no `std`** (2026-11-01). A font is read, outlined and shaped by
  `read-fonts` (its font model draws unhinted outlines in font units and reads the variation location) and `harfrust` (the
  Rust port of HarfBuzz, which reads that same font). `skrifa` is not used: `read-fonts` 0.46 draws outlines itself.
  `rustybuzz` is archived, `ttf-parser` is in maintenance, `swash` pins an older `skrifa`, `parley` is a whole layout
  engine. Both crates are built with `default-features = false, features = ["libm"]`: with `std` the composite-glyph
  offsets (`hypot`) and rounding go to the platform's maths, which differs between targets. Cargo unifies features
  across the workspace, so `poe rust-fonts` reads `cargo tree` and fails when either has `std`. Outlines are drawn
  unhinted in font units and scaled by `size / units_per_em` in `f64` here, so the scale is ours. *Reason:* rule 2
  (the same bytes on every target) and rule 3 (no platform maths). *Consequence:* a new release may move a glyph or a
  shaping result, so the pins are exact and a bump is deliberate. Not yet proven: that native and WebAssembly give the
  same outline bytes for a composite glyph (V5c's differential test).
- **A font is its bytes** (2026-11-01). `Font::new` reads the bytes once and refuses what the library cannot shape (a
  collection, Type 1, bare CFF, over `MAX_FONT_BYTES` of 32 MiB, no outlines, units per em outside 16 to 16384), so
  a `Font` that exists is usable and nothing downstream checks it again. It is keyed in the cache by the SHA-256 of
  its bytes, not by the bytes (a key is compared, and a font is megabytes) and not by its name: the same bytes under
  two names are one step, and a document names the bytes it was drawn with. *Consequence:* `to_json` writes one `inputs`
  entry per byte string with `sha256`; two names for equal bytes, or one name for two byte strings, raise
  `ParameterError`.
- **Overlaps stay until `resolve`** (2026-11-01, changed 2026-11-07). `text` keeps each glyph's own contours, in the
  font's order and direction: glyphs of variable fonts overlap by design (a stem and a serif are separate contours) and
  mesh and pixels fill nonzero, so they draw right. `resolve` (V8) unites them into regions when asked
  ("Booleans (V8)"). *Consequence:* a boolean on raw text holds (it fills nonzero too), but `split` takes the contours
  of an `H` for parts until `resolve` has united them.
- **A line is shaped as one run** (2026-11-01). Each line of a text (split at `"\n"`) is shaped once, in the script
  and direction `harfrust` guesses from its first strong character: kerning, ligatures and the rules of one script
  hold, a line that mixes directions is not reordered, and a line break is a hard break. Attributes number the glyphs
  as they are drawn, left to right: `glyph` counts the shapes made (a space has none), `word` the runs of
  non-whitespace characters from 0 over the whole text, `line` the lines. *Consequence:* V5b shapes once per run of
  equal axis location (next entry), so kerning and ligatures do not cross a change of location.
- **`Reads::Once`** (2026-11-01). `text`'s `size`, `tracking`, `leading` and `anchor` are read once for the whole
  call: a Param can feed them a number, never a list (a list has an entry per shape, and a text has none until it is
  drawn). The check is `Expr::value`, which already refuses a list, so no second rule exists.
- **The origin of a glyph** (2026-11-01): the middle of its advance, half the font's cap height (`OS/2`) above its
  baseline, where api.md says a glyph turns and scales. A font that states no cap height (none, or `sCapHeight` 0) gets 0.7 em
  (`CAP_HEIGHT`, which only places `origin`).

## Web fonts and bundled fonts (V5d)

- **WOFF2 through `wuff`, unpacked in `Font::new`; WOFF refused** (2026-11-01). Neither `read-fonts` nor `harfrust` reads a
  web font. `wuff` 0.2.9 (MIT, pure Rust, `no_std`; hand-ported from Google's `woff2` and checked against every font of
  `google/fonts`) does, with `default-features = false, features = ["brotli"]`: `bytes` and `brotli-decompressor`
  (MIT or BSD-3-Clause) with `alloc-no-stdlib` (BSD-3-Clause, so `deny.toml` allows `BSD-3-Clause`). It builds for
  `wasm32` and has no `std`, so the unpacked bytes are the same everywhere. It is pinned exactly and used only in
  `type/unpack.rs`. The font is still its supplied bytes: the digest and a saved document name the WOFF2 file, and the
  unpacked sfnt is what is read. WOFF (zlib) would need its `z` feature, which brings `flate2`, for a format that every
  tool converts to WOFF2 or TrueType in one line, and its decoder sizes its output buffer from nothing (a `TODO` in
  1.x); it is refused with the way out. *Bomb:* the decoder stops at 128 MiB and a ratio of 100 to 1; `Font::new` then
  refuses an unpacked font over `MAX_FONT_BYTES` (32 MiB), the limit of a file, so one number bounds both. The unpacking
  allocates its output up front, so a hostile file costs up to 128 MiB for a moment, then an error naming `data`.
- **Two fonts bundled as WOFF2, compiled in** (2026-11-01). Bricolage Grotesque (axes `opsz`, `wght`, `wdth`) and Fraunces
  (`opsz`, `wght`, `SOFT`, `WONK`) are the upstream `google/fonts` files, both SIL OFL 1.1 with no Reserved Font Name,
  made into WOFF2 by fontTools and nothing else removed or changed (`recalcTimestamp=False`): the name table with the
  copyright and licence text is intact, so the files are not Modified Versions. The core does no IO, so `include_bytes!`
  puts them in `crates/core/fonts/`, which the crate, the sdist and the wheel carry with each `OFL.txt`. As WOFF2 they are
  400 KB for both (the TrueType files are 770 KB), and unpacking costs milliseconds once per `Font::bundled` call. Roboto
  Flex (1.8 MB) is not bundled. *Size:* the wheel grows from 1.71 to 2.25 MB, the release WebAssembly module from 2.85 to
  3.51 MB (898 to 1,406 KB gzipped: the fonts and the Brotli decoder); a feature that leaves the fonts out is the way to
  shrink a page that needs none, to add when someone asks. A font keeps its file's defaults (Fraunces opens at the black 9 point cut): changing
  them would make a Modified Version and make the bytes differ from upstream's.
- **A bundled font is an input the loader finds by digest** (2026-11-01). `to_json` writes every font with its `sha256`;
  when a document declares a font input with the digest of a bundled font and `inputs` does not give it, the loader reads
  the bundled one (`Font::bundled_bytes`, one place; a name is no key, since any font may be given any name). Supplied
  bytes win and are checked as before. *Reason:* a saved drawing in a bundled font should load anywhere, in Python,
  Rust and WebAssembly, with no extra call in each binding. *Consequence:* a hand-written document without `sha256` still
  supplies its bytes.
- **Licences travel with the fonts** (2026-11-01). The package licence is `MIT AND OFL-1.1` (workspace `Cargo.toml` and
  `pyproject.toml`), `license-files` lists each font's `OFL.txt`, and `cargo-deny` allows `OFL-1.1` for the three
  workspace crates only (`[[licenses.exceptions]]`). `poe build-check` requires the `OFL.txt` of every
  `crates/core/fonts/*/` it finds in the checkout, and the font files, in the sdist and the wheel, so removing one from
  `license-files` fails the gate rather than shipping an unlicensed font.

## Axes and features (V5b)

- **An item of `axes` is a source character, and a change of value cuts the shaping** (2026-11-01). A per-glyph value
  cannot be defined after shaping: a ligature merges characters and the shaper decides the glyphs from the axis values
  (advances, `kern`, `GSUB` all vary with the location). So the unit is the character of `content`, counted by `chars()`
  with spaces and line breaks (a list given for `"H\nH"` has three entries; the rule is one sentence a caller can
  predict, and a count that skipped breaks would make a list's length depend on the layout). The value of every
  character is read first (no shaping pass at the default location to learn clusters: the characters decide), a line
  is cut into runs of consecutive characters with equal location (equal as `f32`, which is what the font stores), and
  each run is shaped alone at its location (`instance_builder().variations`), with its own outlines. *Consequence:*
  kerning, ligatures, mark attachment and cursive joins do not cross a change of value, and a location per character
  costs a shaping per run; a text with equal values (a constant or a Param without a list) is one run per line, as
  before. *Rejected:* shaping once at the default location and moving glyphs by the difference of advances (kerning and
  ligatures would then be those of the wrong weight, and a ligature could not split); a value per shape (the shapes
  do not exist before the text is shaped).
- **`Font::axes` are `Param`s; ranges are the font's** (2026-11-01). The font's own range, default and name become a
  `Param` (the existing descriptor, no second type), so a slider, `parameters()` and `to_json` treat a font axis as any
  Param. An axis a `Param` cannot hold (minimum not below maximum, a tag that is not a name, a repeated tag) is left out of
  the list; it can still be set by tag, as the check is against the font's record, not the `Param`. A value outside the
  range is refused, never clamped, and a Param wider than the font's range is checked when drawn.
- **Features are literal `u32`s, tags are four printable ASCII characters** (2026-11-01). A feature value is a switch or the
  number of an alternate, not a quantity to animate, and a `u32` is what the shaper takes (0 off, 1 on, more for an
  alternate), so the argument type states the range and no value check exists. The draft schema said 2^53 − 1 and
  was tightened with an agreement row. A tag is exactly the schema's `tag`; `MAX_SETTINGS` (64 axes and 64 features a
  call) bounds the memory per character and the time per run that a hostile font with thousands of axes would cost.
- **`font-checks` is the font's rule, run twice by one function** (2026-11-01). `Font::check_axis` and
  `Font::check_feature` are called by the verb's own check (the call in code, constants only) and by the loader before the
  drawing is built (`ParameterError` naming the input: the caller gave the wrong font for this document, not a malformed
  document). *Reason:* one decision in one place, and the two entry points report different kinds on purpose.
- **Glyphs are charged as they are placed** (2026-11-01). With a location per character a text can ask for 100,000
  distinct outlines of one glyph; charging `MAX_SHAPES` and `MAX_OUTLINE_SEGMENTS` after all were drawn would draw them
  first. The charge happens per glyph as it is laid out, so memory stays within one glyph of the limit.

## Bindings of type (V5c)

- **An axis a `Param` cannot hold stays out of `font.axes`, and that is not a lie** (2026-11-01). The list is what a
  caller can put a slider on: a pinned axis (minimum equal to maximum) has nothing to vary, and a tag that is no `Param`
  name cannot be one. Both are still settable by tag, in range, and every refusal lists the font's axes with these among
  them (`tests/test_text.py` builds such fonts with fontTools and proves each). The alternatives cost more than the
  omission: a `Param` with an empty range changes what every Param is, and refusing the font at `Font::new` would refuse
  fonts that draw correctly.
- **Fonts in a document cross the bindings as bytes by name** (2026-11-01). Python takes a `dict` of `bytes` or `bytearray`
  and JavaScript an object of `Uint8Array`s (a view into a larger buffer included: its own bytes are copied), each checked
  by type naming `inputs[name]` or `inputs.name`, and the core gives every other refusal. There is no getter for the
  names a document declares: the document is plain JSON, and a missing input is a `ParameterError` that names it.
- **`anchor` in Python is plain numbers** (2026-11-01), as in Rust: it is read once, and `Text::anchor` takes a `Point`.
  A `Param` there is a `TypeError` naming `anchor`, not a silent default.
- **Text is exact between native and WebAssembly, with no rounding step** (2026-11-01). The ten text drawings of the
  differential (kerning, a ligature, a composite glyph, light, between and heavy weights, a `Param` axis and a weight a
  character, tracked lines, a stroke, 64 nested composites, a glyph of 3,999 segments) give the same SVG text and mesh
  buffers natively and in WebAssembly, so the fallback of rounding outline coordinates to a grid where they become a
  `Path` was not needed, and is not part of the design. A difference found later is a defect to diagnose, not to round away.
- **fontTools is the oracle of the font tests, a development dependency** (2026-11-01). It reads the same files with code
  that shares nothing with the library: outlines and advances (also along an axis), axes, feature tags. Test fonts that
  need a shape (two axes, a pinned axis) are built by it in the test, so no binary fixture hides what a font holds. A
  library outline may differ from fontTools' by up to a font unit along an axis (it rounds a delta where fontTools does not).

## Layout (V6)

- **The lines are planned at the reference pose; `width` alone is read as drawn** (2026-11-04). Where lines break, the size
  or axis value a fit finds and the lines of a text inside a shape are computed once with every Param at its default,
  whatever `with_values` fixed; the glyphs are then drawn at the live values. *Reason:* fitting at live values shrinks the
  word on every pulse (`prototype-findings.md`). `width` is the exception because the node contract draws every number at its Param's value and refuses one out of
  range: a width Param that did not move the breaks would mean nothing. *Consequence:* a live `size`, `leading` or axis value
  moves glyphs and can overflow a measure; it never re-wraps.
- **Lines break at whitespace only, and the whitespace at a break goes** (2026-11-04). Greedy, at `char::is_whitespace`
  except U+00A0, U+2007 and U+202F (no-break spaces, part of a word); a line drops the whitespace it ends in, and a part keeps the whitespace it
  begins with. *Reason:* one rule, exact and the same on every target. The Unicode line-break rules need data tables and,
  for Thai, a dictionary, which the shaper (`harfrust`) does not provide (inferred; not measured against a crate).
  *Consequence:* text without spaces (Chinese, Japanese, Thai) does not wrap. The wrap measures a part shaped whole at its
  nominal size, so a line shaped alone can differ from its measure by a kern at the break.
- **A word wider than `width` overflows; a word wider than every chord of a shape is refused** (2026-11-04). A measure has
  nowhere better to put a long word than a line of its own. A shape is a boundary: a glyph outside it is a wrong picture, so the text is refused naming the word and the line, and
  nothing is cut off. The same goes for text that outlasts the shape.
- **A fit by size is closed form; the leading follows each line's size** (2026-11-04). A line's width is linear in its size
  (tracking is in thousandths of an em), so the size is `width × size / measured width`, exact. Lines then differ in
  size, and a fixed leading would crowd the large ones: with no `leading`, each line stands 1.2 times its own size below
  the baseline before it. An explicit `leading`, and a text no line of which was resized, keep `n × leading`.
- **A fit by an axis brackets the ends, then halves 32 times** (2026-11-04). The line is shaped at the axis's minimum
  and maximum; a `width` outside the two widths is a `ParameterError` naming the line, the nearest width and the axis
  value there, never a clamp. Inside, the range is halved 32 times: that leaves 2⁻³² of the widest axis range
  (65535), under 2⁻¹⁶ of a unit, far below the unit to which the shaper rounds an advance. *Consequence:* the width is taken to change one way along
  the axis, so a width reachable only in the middle of the range is reported as not reachable, and the line comes out
  within about a unit of an advance per glyph of `width`. Empty and whitespace-only lines have nothing to fit and are skipped.
- **A text inside a shape: two heights, the widest chord, the top of the bounds** (2026-11-04). A line must hold its glyphs
  from the capital line to the baseline, so a line's chord is the widest span the shape has at both heights (the
  leftmost of equals); `align` places the line against it. The first capital line is the top of the shape's bounds and
  each baseline is `leading` (default 1.2 × `size`) below the one before; a baseline whose chord holds no next word is
  skipped, and an empty part takes a baseline. *Consequence, from the code and not tested:* only those two heights are
  looked at, so a shape that narrows between them, or a descender, can poke out. `leading` must be above 0 (baselines
  must advance), baselines tried are bounded by `MAX_GLYPHS`, `width`, `fit` and `anchor` are refused with `inside`
  (the shape sets the lines and where they stand: one decision, one place), and the region is the one `contains` fills,
  flattened once. Right-to-left text inside a shape is untested.
- **`along` is generic; text on a path is `text(...).along(path)`** (2026-11-04). The roadmap listed `along` under V11
  (path modifiers) and text on a path under V6. One bend serves both, as a step like `translate` that carries any
  shapes, so text on a path needs no text-specific node and a `start` Param or a list turns or spreads it. *Consequence:*
  `along` left V11; placing unbent copies of a shape along a path stays there.
- **`along` is `x` along, `y` to the right, by the curve's tangent** (2026-11-04). A point goes `start` lengths of the
  guide plus `x` along it, `y` to its right (`y` down: below stays below on a guide running right, so letters laid out
  straight read the same way round it). The direction is the tangent of the curve at that place, not that of the
  flattened straight edge: the edge's direction jumps at each corner of the flattening, and letters would step. A closed
  guide (every sub-path with a length closed) wraps; any other goes on straight past its ends along the end tangent, so
  a live `start` never errors and the image is continuous across an end. A bent edge is halved up to 8 times until the
  bent middle is within 0.01 px (the flattening tolerance of the crate) of the middle of the bent ends; the result is
  lines (the bend of a Bezier is no Bezier), counted against `MAX_SEGMENTS` before they are made. *Consequence, a known
  limit:* an edge longer than half a closed guide can be bent coarsely, as its ends and middle can agree by chance. The
  `origin` attribute is left where it was.
- **A no-break space is part of a word; a transform moves `origin`, and `about` defaults to it** (2026-11-07, from the V6
  carry-over). U+00A0, U+2007 and U+202F are part of a word for the wrap and for the `word` attribute alike, decided by one
  predicate. `translate` adds to `origin`, `rotate` and `scale` transform it, and their default `about` is `origin`, else the
  centre of the bounds. `along` leaves it. *Reason:* `merge(by=word)` must group what a wrap keeps together; and per-item
  transforms of glyphs (V7) pivot on the typographic centre, not on a bounding box that moves with each letterform.

## Per-item variation (V7)

- **Per-item numbers are one `Expr` node and never fold to a constant** (2026-11-07). `by_index`, `by_order`, `random` and
  `stagger` are one variant in `expr/item.rs`, which alone knows what each reads and refuses. An expression is plain
  (`is_plain`: no Param, no per-item number) and folds to its value, or per-item (`is_per_item`) and is worked out for
  each shape when drawn. *Reason:* a number whose value is the shape's place folded to one constant would draw every shape
  alike; and one node means the loader, writer, schema and bindings learn one thing. *Consequence:* an argument that is not read per
  shape (`Reads::Once`, a literal) refuses one.
- **`random` is SplitMix64 on `seed` and `index`** (2026-11-07). The number of a shape is the `index + 1`-th output of the
  generator started at `seed`, its top 53 bits scaled by 2^-53. *Reason:* integer arithmetic and one exact conversion,
  so the same bytes on every target with no `libm` call, no clock and no state; a hash of the index, not a stream, so a
  shape's number does not depend on how many shapes were drawn before it. *Consequence:* the numbers move when an `only`
  or `split` renumbers `index`.
- **`stagger` runs the shape at order `p` from `p × spread` for a length of `1 - spread`, clamped to 0..1** (2026-11-07).
  The first shape starts at progress 0 and the last ends at 1, so `progress` 0 draws nothing and 1 draws everything for any
  `spread`. *Reason:* a window of fixed length after a staggered start is the one formula in which progress 0 and 1 are
  the ends for every spread. `spread` 1 would be a window of length 0 (a division by zero), so it is refused with
  a value outside 0..1.
- **A shape that carries no `order` reads 0** (2026-11-07). A lone path or shape given as it is gets `first` from
  `by_order` and starts first in `stagger`; a stack writes `order` on shapes that carry none. *Reason:* one rule instead
  of an error for a drawing that has one shape.
- **`merge` joins sub-paths; it does not unite them** (2026-11-07). The merged shape holds the sub-paths of its group in
  list order, with the first's attributes and style, at the first's place. *Reason:* a word as a unit to move or slice
  needs no region algebra, and the boolean batch is the riskiest (roadmap, V8). *Consequence:* overlapping contours stay
  overlapping; `union` or `resolve` (V8) unites them.
- **`split` gives a part the centre of its own bounds as `origin`** (2026-11-07). The origin of the letter is replaced.
  *Reason:* a part that turns about its letter's origin (the dot of an `i`) would swing round it; each part scatters as a
  piece.
- **Draw-on is `Drawing.slice`, built in V7** (2026-11-07). The roadmap listed it under V11. `slice` wraps `Path::slice`
  as a step whose positions are read per shape, so `slice(0, stagger(progress))` draws a text on letter by letter.
  *Reason:* the done-when of V7 is a word that draws on letter by letter with no text-specific modifier, and the one step
  is smaller than a dedicated draw-on node. Dash with a moving phase, placing shapes unbent, simplify and resample
  stay in V11.

## Booleans (V8)

- **The engine is `i_overlay`, in `path/boolean.rs` alone** (2026-11-07). One engine run per verb: `resolve` is a run
  with nothing to combine, the others combine two paths. *Reason:* a spike on 9.1.0, pinned exactly (a new release may snap
  or order contours differently), found: MIT OR Apache-2.0, five new crates (`i_float`, `i_key_sort`, `i_shape`,
  `i_tree`, all `no_std`, one author; `libm` was already a dependency) and `cargo deny` and `cargo shear` pass; it builds
  for `wasm32-unknown-unknown` (also with `simd128`) and the module has no imports; the float maths that reaches
  the output is `libm`, the sweep is integer arithmetic, no hash-ordered collection, clock or IO in the five crates;
  200 seeded pairs by 4 operations hash alike natively (release and debug) and in Node. The differential (16 boolean
  drawings, exact SVG text and mesh buffers) holds on macOS arm64. It panics on non-finite or over-range input, so the core
  validates first and wraps nothing. *Consequence:* the spike did not check Linux or Windows native bytes (the Linux
  `wasm-test` compares native with Node in CI). `linesweeper` was not evaluated. The crate does not appear in the API.
- **`unsafe` in the dependencies is accepted** (2026-11-07). Counts of the word in `src`: `i_overlay` 52 (48 blocks),
  `i_key_sort` 19 (17 blocks), `i_tree` 19 (13 blocks and 6 `unsafe fn`), `i_float` and `i_shape` 0, mostly
  `get_unchecked`. *Reason:* `unsafe_code = "deny"` covers this workspace's code only; accepted in place of writing a
  sweep ourselves. *Consequence:* the fuzz (below) is the guard: no panic or hang in 6 x 10,000 cases, a glyph corpus
  and the extremes.
- **The `i64` engine on an automatic grid** (2026-11-07). The crate rounds both operands onto an integer grid it picks
  per call from their joint bounding box, 2^-60 of the half-extent with `i64` (2^-29 with `i32`). *Reason:* `i32` drops a
  1e-6 hole beside shapes 4000 wide and a unit square beside a shape 1e12 away; `i64` is finer than `f64` at that
  size, and costs 30 to 45 per cent more on large inputs and nothing at glyph size. A fixed absolute grid cannot give
  0.01 pixel and ±1e100 at once, and choosing the bits by hand would add a constant to justify (rule 1). *Consequence:* precision follows the extent of the operands, and `a.union(b)` and `a.resolve()` can differ in
  a corner (api.md, "Booleans").
- **The fill rule is nonzero** (2026-11-07). *Reason:* it is the rule of `contains`, of every fill and of the fonts, so a
  boolean agrees with what is drawn. *Consequence:* a contour inside another that runs the same way is absorbed, one that
  runs the other way is a hole; the several shapes of an `other` are each resolved before they are joined, or a shape that
  ran the other way would cut the overlap out of the rest.
- **Flatten, then combine; `tolerance` is a keyword, default `FLATNESS`** (2026-11-07). Curves become straight edges
  that stray by at most `tolerance` design pixels, and the result has straight edges only. *Reason:* the engine takes
  polygons; the number that shapes the result is a parameter in its own terms (rule 1) and its default is the
  `FLATNESS` of the flattening for output. On a `Drawing` step it is a literal number, not a `Param`. *Consequence:* a
  curve is not restored by a later boolean.
- **The canonical form is made by re-sorting the engine's result** (2026-11-07). The engine gives outers positive
  shoelace (clockwise with `y` down), holes the other way, ordered and started by their leftmost point; the order does
  not depend on the order or start of the input. The library orders by topmost point and starts each contour
  there. *Reason:* a draw-on starts at the same place on every shape ("Path"). *Consequence:* one stable sort and
  a rotation of each contour; where two tops tie in exact arithmetic, the start can differ between calls.
- **`resolve` is a step of its own, not part of `text`** (2026-11-07). *Reason:* text draws right unresolved (mesh and
  pixels fill nonzero), and resolving costs on every draw: a line of 35 capitals takes 0.033 ms to `shapes` and 1.8 ms
  after `resolve` (`docs/benchmarks.md`, "Booleans (V8)"), about 50 microseconds a glyph, which a caller animating a
  weight pays every frame. *Consequence:* a caller who needs the true parts of a glyph (`split`) writes `resolve`.
- **`other` is one drawing, `Kind::Drawing`** (2026-11-07). A boolean's operand is evaluated with the caller's Params
  (a cutter moved by a `Param` is a drawing), so it is an argument kind of its own, and in a document an array of
  calls. Its shapes are united once per step, not once per shape of the subject; a lone shape is used as it is.
  *Consequence:* style stays the subject's, and an empty `other` makes `intersect` empty and the others keep the region.
- **A `Drawing` step charges the pairs of edges, a `Path` method nothing** (2026-11-07). Work and memory grow with the
  crossings, which no count of edges bounds: `Drawing.resolve` of a 16,000-edge scribble was killed at 4189 MB under a
  4000 MB cap with the existing limits. The step now charges `MAX_SEGMENTS` for the flattened edges, for the pairs of edges
  whose boxes overlap (counted in `O(n log n)` before the engine runs; at least the crossings) and for the result's
  vertices. *Reason:* the pair count is at least the crossings the work grows with, and is counted in `O(n log n)`; 10 million
  pairs cost 1.4 to 3.5 GB. A direct `Path` call stays unbounded, as `along` is: a `Path` method has no draw to count
  in. *Consequence:* a rough guard, not a budget (api.md, "Booleans").
- **What the fuzz found** (2026-11-07), one fix and one regression test each: a result contour whose corners are in line
  once rounded to numbers; a hair-thin contour that rounding turned over (a hole before any outer); a thinness test that
  read a contour smaller than 1e-160 as flat; a thinness test judged on the contour instead of the whole result's width;
  a lone point or line widening the engine's frame and so moving the other operand's corners; edges that cross heavily
  exhausting memory. The fixes keep one rule: a contour no thicker than 4 ulps of the result's largest coordinate is rounding and
  is dropped.
- **No boolean raises `GeometryError`** (2026-11-07). The engine resolves any outline, so the errors are
  `ParameterError`: a `tolerance` outside the range, a result beyond ±1e100, `MAX_SEGMENTS`. *Reason:* an error the
  engine cannot produce would be dead code. `GeometryError` stays for verbs that can fail geometrically.

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
- **A gate that passed is not repeated on the same tree** (2026-11-01). `poe check` notes the tree of `HEAD` when it
  starts and, when every step passed, records it in the git folder if the working tree was clean at both ends and `HEAD`
  was unchanged; the pre-push hook skips the gate for a tip with that tree. *Reason:* the hook repeated a 6-minute gate
  on contents just checked, and a slow loop gets worked around. *Consequence:* nothing is skipped that was not checked;
  an edit, a commit during the run or any uncommitted file means no record, and CI checks everything again.
- **Bounded depth, not a bigger stack** (2026-11-01). Whatever a caller can build has its depth limited where it is
  built, so no walk recurses over unbounded depth: drawings nested past `MAX_DEPTH` (128) are refused when built, the
  parser refuses a document nested past it before the loader recurses, and an expression past 128 levels becomes a node
  without operands (operators never fail, so a loop can build one; the call that uses it is refused). The smallest
  supported thread stack is **256 KB** (a Python thread gets 512 KB on macOS); measured need in a release build: 128
  nested drawings about 52 KB, the deepest document `from_json` reads 96 to 128 KB. In WebAssembly the engine throws a
  catchable `RangeError` below about 0.25 MB. *Reason:* an overflow is a segfault in the caller's process. A growing
  stack (`stacker`) was rejected: it cannot grow the WebAssembly engine's stack, loses CPython's overflow guard on 3.14
  and later, adds `unsafe` and a C compiler to the sdist build, and trades a bounded stack for unbounded memory.
  *Consequence:* `tests/test_stack_depth.py` and `crates/core/tests/it/expr.rs` run the deepest structures on the minimum
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
