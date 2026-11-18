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
- **`roxmltree` reads SVG; the import is trimmed** (2026-11-11). `Drawing::from_svg` parses with `roxmltree` (a library
  dependency of `crates/core` since this batch; before, a dev-dependency). *Reason:* XML has entities, namespaces,
  CDATA and character references that a hand-written reader would get wrong; `roxmltree` is 1 crate with one
  dependency (`memchr`, already in the lock file), no `unsafe`, no DTDs (a document with one is refused, so no entity bombs), builds for
  `wasm32-unknown-unknown`, and `cargo-deny` admits its licence (MIT or Apache-2.0). The JSON parser was written by hand
  (`json/parse.rs`) for the opposite reason: its grammar is small and the format ours. *Consequence:* the import reads
  `svg`, `g` and seven shape elements; everything else that changes the picture is an error naming it, so a logo that
  uses it fails loudly instead of drawing wrongly. `fill-rule="evenodd"` is refused because the renderer fills nonzero
  only. A group's `opacity` is refused because it fades the group as one picture, which `Style` cannot say; stroke
  widths scale with an even transform and a stretching one is refused, since a baked transform leaves `Style` with no
  stroke to stretch. `group` is one string of the enclosing ids (`logo/mark`) matched whole: a hierarchical match
  would widen `only`, left for a later batch. *DOCTYPE stays refused* (2026-11-11, measured): `roxmltree` with `allow_dtd`
  stops billion laughs and entity loops (its depth 10 and 255-references limits) but not a quadratic blow-up: a 60 KB
  entity, 255 of it in a second entity, that one used 300 times in 2 KB of text expands to 4.6 GB (4 GB memory cap hit; 20
  uses gave a 740 MB peak and an `Ok`). Nothing in its options caps the expanded size. Allowing Illustrator headers needs
  our own pre-scan (refuse `<!ENTITY` and allow a bare DOCTYPE), or a parser with an expansion cap; not built.
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
- **Limits:** 16 MiB of text, depth 40 counted through references, 100,000 calls; schema bounds on the literals that
  set work; counters checked while drawing (shapes, segments, field samples, copies, glyphs). *Reason:* a document may
  come from anyone; depth through references bounds the evaluator's recursion (small in WebAssembly), and 40 is what
  the loader reads on a 256 KB stack in an unoptimised build ("Bounded depth"); work cannot be estimated beforehand (a shared drawing
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
  corners on a coarse grid, where edges coincide, 1 of 20,000 (56 before), also seen with plain bevel and miter joins. Strokes with several sharp turns (two or more over about 100 degrees) are still covered twice by a small area in
  about 1 of 2,000 random round strokes (5 to 13 of 20,000, seed 2026-11-07), never missed: the rings are correct (their
  summed winding matches the distance to the centreline on a 0.1 pixel grid), and lyon emits two triangles on one base
  where the long offset edges of neighbouring segments run nearly alongside each other. No ring construction fixed it: a
  horizontal sweep only moves the failures, and one convex piece per segment and corner cut them 3 to 4 times at 7%
  more triangles and a triangle at every smooth corner. Resolving the rings with `Path::resolve` before the fill covers
  every point once, but costs 3.4 times the time of `to_mesh` on stroked scenes. Open: resolve only strokes that fold
  back, by a geometric test, with the engine's cost charged to `MAX_TESSELLATION_WORK` first.
  The limit stays: the known failing inputs are expected-failure tests in `mesh_stroke.rs` (they fail once the limit is gone),
  and the random property excludes strokes with two or more turns over 100 degrees, the boundary of the limit.
  A smooth curve gains no triangles, and the scenes measured gained under 6%.

- **Looking the same outranks byte equality** (2026-11-08, the owner). The promise is a drawing that looks the same on
  every target, not the same bytes or pixels. Byte equality of SVG text and mesh buffers stays the aim and the test
  (below), because it needs no reviewed baseline and catches real faults. *Reason:* a last-digit difference nobody can see
  is not worth code that fights the platform. *Consequence:* a difference found there that no one could see is fixed in the
  test (a measured tolerance, with the cause recorded here), not by contorting the code; a visible one is a defect.
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
  charge comes before the allocation. Drawings nested more than `MAX_DEPTH` (16, also the limit of
  an expression) deep hold an error, and a chain of steps costs no depth: it is walked in a loop and dropped in one. The
  other four counters arrive with the first step that produces what they count. *Reason:* the counters bound memory
  blow-up (a stack of itself 60 times), not the length of a chain, and the cache (V2b-2) serves repeats without
  charging. An expression is capped at `MAX_EXPR_NODES` (10,000, a shared part counted each time it is used, as
  evaluation reaches it) beside its `MAX_DEPTH`, checked at the call without evaluating: `e = e + e` 15 times is 2^16 - 1
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
  `MAX_DOCUMENT_DEPTH`, so no hostile document recurses past it, and the byte limit is checked before the first byte is read.
  *Reason:* `serde_json` keeps the last of two equal keys unless a visitor is written for it, parses floats to within
  one ulp unless a feature is on (the same text must give the same bits on every target), and brings `serde`,
  `itoa`, `ryu` and `memchr` into a core that must stay small and bit-exact. The parser is about 250 lines, and the
  rules the format needs (a pointer for every error, limits while reading) are part of it. *Consequence:* the number
  reader hands the digits to `f64::from_str`, which is correctly rounded on every target, and the writer lays out the
  shortest digits of `{:e}` by the rules of RFC 8785.
- **Where the loader's checks run** (2026-10-31). `from_json` refuses a text over `MAX_DOCUMENT_BYTES` before reading
  a byte, counts calls while it reads (`MAX_CALLS`), and the parser stops at `MAX_DOCUMENT_DEPTH` levels of text, so a deeper
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
  refuse a drawing that `from_json` read, with `ParameterError` naming `MAX_DOCUMENT_DEPTH`.
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
- **Two fonts bundled as WOFF2, compiled in** (2026-11-01; JetBrains Mono, a third, was added in 4b: "Character grids and overlay marks (4b)"). Bricolage Grotesque (axes `opsz`, `wght`, `wdth`) and Fraunces
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
  exhausting memory. The fixes keep one rule: a contour no thicker than 4 ulps of the largest coordinate is rounding and
  is dropped.
- **What the fuzz found later** (2026-11-08), two flakes that a random gate seed hit (2 x 20,000 cases of each property
  on the old code found neither: they are rare). (1) The rule above took the largest coordinate from the *result*, but
  the engine rounds to a frame as wide as all of its *input*: a subtract that left only a 0.008 piece beside a shape at
  713 kept a hair of area 3e-18 as a hole outside every outer ring. The scale is now the larger of the input's and the
  result's (`canonical`), and the one regression test is the shrunk case. (2) A shape beside a copy of it nudged by 1e-7
  and then moved by 7759.6: the move rounds an exact overlap (a corner on another edge, doubled back) into a sliver
  1e-12 thick, and `i_overlay` lost a quadrilateral of area 8e-16 (1e-8 thick, 7e-8 long) at that corner; a plain
  triangle for the shape does not. No fix in our code, and it is invisible; it is in api.md, "Booleans". The property
  states it: a scene whose generator nudges by `JITTER` adds ten nudges to its slack, so a probe keeps 1.4e-6 from
  every edge and the area bound grows to match, while every other scene keeps `1e-13` of its largest coordinate. The
  shrunk case is a plain test with that slack, red with none.
- **No boolean raises `GeometryError`** (2026-11-07). The engine resolves any outline, so the errors are
  `ParameterError`: a `tolerance` outside the range, a result beyond ±1e100, `MAX_SEGMENTS`. *Reason:* an error the
  engine cannot produce would be dead code. `GeometryError` stays for verbs that can fail geometrically.

## Offsets (V9)

- **The engine is `i_overlay`, in `path/offset.rs` and `path/boolean.rs` alone** (2026-11-08). `offset` resolves the path
  first (the nonzero region, as the booleans do) and offsets the polygons; `outline` strokes each flattened sub-path on
  its own and unites the strokes. Both results go through the canonical form of the booleans. *Reason:* the crate is
  already the boolean engine and has the offset, the stroke, the three joins and the three caps, so no second geometry
  engine enters the tree.
- **A miter beyond the limit is clipped flat, not bevelled** (2026-11-08). *Reason:* the engine gives a miter that is
  cut at a length (SVG 2 `miter-clip`) and cannot bevel; a chamfer pass of our own is complex and breaks on edges shorter
  than the cut. *Consequence:* `stroke(join="miter")` (SVG's `miter`) bevels past the limit and `outline` clips, so the
  two differ at a sharp corner. Measured on a corner of ratio 16 at `miter_limit=4`, width 10, vertex at y=40: the
  pixels and the mesh of the stroke reach y=39.5 (a bevel), the outline y=20.2 (cut 20 above the vertex, half the width
  times the limit); at limit 30 both reach the full tip. The SVG text of a stroke only says `stroke-linejoin="miter"`.
- **The round arc step comes from `tolerance`** (2026-11-08): `2 acos(1 - tolerance / r)` for radius `r`, the widest step
  whose chord strays by at most `tolerance`. *Reason:* the number that shapes the result is a parameter in its own terms
  (rule 1), and `tolerance` already is.
- **`Path.subpaths()`, not `contours`** (2026-11-18): `Field.contours()` already means the lines of a field, and
  `docs/formats.md` calls these sub-paths; one name keeps one meaning. Unlike `split`, holes are not kept with their outline.
- **Offsets and outlines drop the corners `tolerance` makes invisible** (2026-11-18). *Cause:* the engine joins each vertex
  of the flattened input with an arc of its own, so a smooth outline came back with two corners per corner it had (a circle
  of 383 gave 767, then 1,535, 3,071...; one letter 55, 1,929, 3,337, 5,560, 9,624, 17,640 over five calls of 4), and
  repeated offsets grew without bound. *Fix:* `thin` in `path/offset.rs`, Douglas and Peucker on each contour of the
  result before it is made canonical, at half of `tolerance`; arcs take the other half (`tolerance / 2`), so a round
  join still strays by at most `tolerance`. Booleans and the tapered outline are untouched. *Changed:* offset and
  outline bytes (fewer corners, within `tolerance` of the old ones); the radius where the engine's 1.8 degree arc floor
  bites moves from 80 to 40 times `tolerance`. A thinner than `tolerance` sliver can vanish.
- **Limits of the engine found when it was built in** (2026-11-08), kept, not worked around: a `miter_limit` above about 23
  acts as 23 (the engine takes the corner angle in integer maths); its arcs run from 1.8 to 45 degrees, so a round join of
  a radius above about 40 times `tolerance` strays by up to 0.00012 of the radius (and, since 2026-11-18, half of `tolerance` more); a turn under 5 degrees is bevelled; a
  `distance` beyond about 1e19 times the size of the shape silently gives an empty path, and so does a `width` far below
  the integer grid; a one-point sub-path paints nothing in `outline` though `stroke` draws a dot. *Reason:* each is far
  outside what a drawing asks of it, and a guard would be code for a case no one meets (rule 2).
- **`distance` and `width` are per-item numbers on a `Drawing`; `join`, `cap`, `miter_limit` and `tolerance` are
  literals** (2026-11-08). *Reason:* a list of shapes grown by different amounts is the use (a stagger, `by_index`); the
  rest choose a kind of corner or a precision, which one step does not vary per shape, as `tolerance` does not for the
  booleans. *Consequence:* a Param cannot switch a join; two steps can.
- **A `Drawing` step charges as the booleans do** (2026-11-08): the flattened edges, the pairs of edges whose boxes overlap
  and the vertices of the result count against `MAX_SEGMENTS`; a `Path` method charges nothing.
- **The differential holds exactly** (2026-11-08): 16 offset and outline drawings (every join and cap, two miter limits,
  an inset, a ring, a collapse, a coarse `tolerance`, a per-item list, a Param, text grown, shrunk and outlined) give
  the same SVG text and mesh buffers natively and in Node on macOS arm64, so no tolerance was loosened. *Consequence:*
  as for the booleans, Linux and Windows native bytes are checked only by CI's `wasm-test` against Node.

## Style effects (V10)

- **Shadow: a silhouette in the shape's group, blur as a CSS radius** (2026-11-08). The shadow is the shape's fill and
  stroke as one alpha, moved, blurred and tinted, painted behind the shape inside the group its opacity and blend apply
  to; `blur` is the radius of a CSS `box-shadow`, so the Gaussian's deviation is `blur / 2`, in design pixels scaled by
  the view like a stroke width. `blur = 0` is a hard copy (SVG: a tinted copy of the path, no filter). *Reason:* the
  numbers designers know from CSS and Figma; one group is what makes a shape at half opacity no darker where its shadow
  lies under it. *Consequence:* a shape that paints nothing casts none.
- **Shadow in SVG: primitives, an explicit region, sRGB** (2026-11-08). `feGaussianBlur` of `SourceAlpha`, `feOffset`,
  `feFlood`, `feComposite in`, `feMerge`; `filterUnits="userSpaceOnUse"` over the canvas grown by the reach;
  `color-interpolation-filters="sRGB"`. *Reason:* measured against `vello_cpu`, `resvg` and Chrome: `feDropShadow` is
  drawn with a wrong colour by `resvg` (35 levels off); the default region clips the blur to the shape's box plus 10%,
  where the rasteriser never clips; the default `linearRGB` changes coloured results, where the pixels blend as sRGB.
- **Shadow in pixels: a filter layer for a blur, a moved copy without** (2026-11-08). A blur is `vello_cpu`'s
  `DropShadowOnly` in a filter layer around the shape drawn once more (single-threaded, as the rasteriser is run); a blur
  of 0 is the shape moved by a transform and drawn in the tint inside a layer of the tint's alpha. *Reason:* the filter's
  offset is not exact for an offset between pixels (a 3.5 by 2.25 offset left 46 pixels up to 55 levels off the
  hand-built copy), the copy is byte-equal to one. The filter's 8-bit kernel is within 2 to 4 levels of the Gaussian from a
  deviation of 1 pixel and not a Gaussian below it (up to 22 levels at 0.3): documented, not worked around.
- **Shadow limits** (2026-11-08). A deviation past 1000 output pixels is refused naming `blur`, and the pixels of the
  layers a draw blurs are charged to `MAX_FILTER_PIXELS` (2^27, the largest picture). *Reason:* measured in release, a
  layer costs about 9 ns a pixel, a deviation of 1000 takes 0.3 s, of 3000 2.8 s and of 5000 does not return, and a layer
  is the shape's part on the canvas grown by 3 deviations and the offset on every side. SVG is not bounded by this: the
  limits are a rasteriser's.
- **`resvg` is the picture oracle of the output tests** (2026-11-08). A dev-dependency of `crates/core` only (14 crates,
  none in the bindings; `cargo-deny` does not check dev-only licences, `arrayref` being BSD-2), it renders the SVG to
  compare with `to_rgba`: pixels inside smooth areas within 4 levels, curved edges not closely (its coverage is off by up
  to a sixth of the range there; `vello_cpu`'s is within 5 levels of exact). The primary oracle of a blur stays the
  Gaussian's error function.

- **Knockout: cut by coverage, one mask for a run, groups that nest** (2026-11-08). A shape that knocks out removes the
  area its fill and stroke cover from everything before it, shadows included, and paints nothing; its colours, alphas,
  opacity, blend and shadow are ignored, because a cut has no tint: it is binary by coverage, and anti-aliasing is the
  only grey. SVG: the earlier shapes become a `<g>` masked by the canvas in white with the knockouts in black; shapes
  after it stay outside. *Reason:* a mask is the SVG form that cuts through a group, shadows and multiply included, and it
  renders alike in `resvg`. Knockouts in a row share one mask (ids by the first one's index, unique in the document); a
  shape between two is not in the first's group only, so the groups nest, which keeps a later multiply blending with what
  is left, as the pixels do (flat groups would blend it with its own group only). *Consequence:* nesting is the number of
  alternations, bounded at 128 (`MAX_KNOCKOUT_LAYERS`), under the 256 levels libxml2 and the tools built on it accept.
  The mask's region is the canvas grown by the shadows' reach, not the unbounded plane: the SVG's own viewport clips at
  the canvas, and a region of ever larger size costs every renderer a mask the size of it.
- **Knockout in pixels and in the mesh** (2026-11-08). Pixels: a `DestOut` layer painted with the knockout's fill and
  stroke made opaque, so what is below keeps `1 - coverage`, as the mask's luminance keeps it. Mesh: a boolean of regions,
  not a stencil, so the three flat buffers stay as they are. The region of a knockout is its fill and its stroke
  outline, each resolved (they may run opposite ways) and joined; `Path::subtract` takes it from each layer of each
  earlier shape, a shifted hard shadow included, after the layers are made and before they are tessellated, and a layer
  the region's box misses is left alone. *Consequence:* a stroke that is cut ends flat at the cut (a round cap is not
  re-made there), where the pixels end it flat too; the work is charged to `MAX_SEGMENTS` as a boolean step of a drawing
  is, so a cut of a scribble is refused before the engine runs.

- **Halftone: a lattice anchored to the frame, tone a number, area-true dots** (2026-11-08). The dots lie on a square
  lattice turned about the origin of the design frame, not about the shape's bounds; a dot's area is `(1 - tone)` of a
  cell, so a tone is the share of paper left bare, whatever the pitch. *Reason:* a lattice anchored to a shape's bounds
  moves every dot when the shape moves a pixel (a screen that swims in an animation, and two shapes side by side with two
  screens that do not meet); anchored to the frame, one screen runs under every shape, as a print screen does. A tone is a
  number per shape for now (`Field` tones arrive in V15 as one more accepted kind, which changes no call). Circles of
  full-cell area leave a cell's corners bare, so a tone of 0 is defined as the solid fill, and dots overlap below a tone of
  `1 - pi / 4`; the covered share there is less than the tone says, which is documented and not corrected (a correction
  needs the area of overlapping circles per tone and buys nothing visible).
- **Halftone is a clip and one path of dots, not a boolean per dot** (2026-11-08). SVG: `<clipPath>` of the outline and a
  clipped group of one `<path>` of all the dots; pixels: a clip of the outline and one path fill; both fill by the nonzero
  rule, so a translucent colour is no darker where dots overlap, as SVG's one path is not. The mesh cuts only the dots that
  cross the outline, all by one boolean. *Reason:* a boolean per dot cost about 500 ms for 3,000 dots in the prototypes; a
  clip costs the rasteriser a mask and the dots one path. Dots reach the rasteriser as polygons of the flatness, not as
  circles: `vello_cpu` flattens a curve it is given coarsely (up to 65 of 255 levels off the coverage of a circle in the
  spike). The fill stayed a colour with a halftone beside it (`Style::halftone`), not a `Paint` enum: a shape has one of
  them, and the setters keep it so (a `fill` clears the screen, a `halftone` clears the colour), which kept every user of
  `Style::fill` as it was.
- **Halftone limit** (2026-11-08). The lattice points near a shape, each times the edges of its outline and the corners of
  one dot, are charged to `MAX_SEGMENTS` before a dot is made, and the error names the `pitch`. *Reason:* each dot is
  tested against every edge (no spatial index: a few thousand dots on a few hundred edges is the use), and the dot's
  corners follow from its radius, so a huge pitch is refused as a tiny one is.

## Paint and compositing style (V10d)

- **Gradient paints** (2026-11-09). `Style::fill` and `StrokeStyle` hold a `Paint` (a `Color` or a `Gradient`), where the
  halftone stays a field of its own (a shape has one screen; a `fill` clears it). `Gradient::linear` and `radial`
  validate (two stops or more, offsets 0..1 that do not decrease, colours that paint, a line with a direction, a radius
  above 0), because `vello_cpu` silently draws a bad gradient as one colour: no output has to check again. The geometry
  is in the design frame as halftone's lattice is, and literals only, so a later accepted kind of number changes no call.
  Pad extend only, radial without a focal point: SVG and the rasteriser agree there, and nothing asked for more.
  `Gradient` keeps its parts behind one `Arc`, so that a `Style` stays small.
- **Unpremultiplied stops.** SVG interpolates colour and alpha apart; `vello_cpu` defaults to premultiplied, which differs
  for stops of different alpha (transparent red to opaque blue passes through purple in SVG and through blue in the
  default). `interpolation_alpha_space` is set to unpremultiplied explicitly, and a test of stops of different alpha
  fails on the other setting.
- **Where a gradient sits in an output.** `output/ramp.rs` places it through the same `View` as the paths and checks
  the range once; the SVG writer and the pixel writer draw from it. Ids come from one function (`gradient_id`): part 3
  gives its mask content a prefix of its own.
- **A shadow follows the paint's alpha.** A blurred shadow is the alpha of the group (SVG's `SourceAlpha`, and the pixels'
  filter layer draws the gradient body), so the silhouette fades where the gradient does. A hard shadow is a copy of the
  path under a gradient of its own with every stop in the tint and its own alpha, moved by the offset, which SVG and the
  pixels draw alike (checked against `resvg` with translucent stops).
- **A gradient stroke does not composite twice.** The stroke outline is one path, but zero-length segments under round or
  square caps are discs of their own, and a translucent gradient painted over both would be darker where they overlap.
  With such pieces the pixels paint them opaque into a layer and give the layer the gradient wherever it is covered
  (`Compose::SrcIn`, over the stroke's bounds); a stroke with no such piece is one fill. A plain translucent colour keeps its
  alpha layer. *Reason:* SVG paints a stroke once.
- **A knockout ignores a gradient** as it ignores colours and alpha: it counts as opaque coverage, in the pixels, the SVG
  mask and the mesh (where a knockout shape is not refused).
- **The mesh refuses a gradient** like a blurred shadow: `ParameterError` naming `fill` or `stroke` and the shape. A
  colour per vertex would interpolate in the triangles, not at the pixels, and a texture needs a range kind that three
  flat buffers lack.
- **The blend set is the ten separable W3C modes** (`screen`, `overlay`, `darken`, `lighten`, `color-dodge`, `color-burn`,
  `hard-light`, `soft-light`, `difference`, `exclusion`) under their CSS names; `hue`, `saturation`, `color` and
  `luminosity` mix channels and stay out. One list (`Blend::NAMES`, built from the variants) gives the word, the
  `mix-blend-mode` value, the verb's choice and the schema. SVG and pixels agree the way multiply does: the blend
  applies to the shape's group (shadow and shape together), and `vello_cpu` and `resvg` are checked against each other
  per mode (plain, at an opacity, with a shadow). Pixels against the W3C formulas worked in f64: an opaque source is
  within 1 level; a translucent one (colour alpha, opacity or both) within 2 levels, the 8-bit rounding of the group
  alpha and of the mix. The mesh keeps its 0 normal, 1 multiply range column and refuses every other mode naming
  `blend` and the shape: a range has no room for a formula, and a fixed blend function covers only multiply over an
  opaque ground. Only the f32 fallback of the non-u8 modes costs speed, nothing else.
- **A clip and a mask cut each shape on its own** (part 3), as `opacity` applies to a shape: not a group of the drawing.
  A clip commutes with painting over, so only a blend or a mask shows the difference (two opaque squares masked at
  half grey give 0.75 where they overlap, a group fade 0.5). A group form would be a different verb on a `Drawing`,
  not a style; it is not built. The region (the source's shapes united once, as a boolean's `other`) is an `Arc<Path>` on
  `Style`, the mask an `Arc<[Shape]>` of the source as painted; every shape one step cuts holds the same `Arc`.
- **Made once per step.** Outputs tell clips and masks apart by that pointer, or by equality: the SVG writes one `<clipPath>` or `<mask>`
  per distinct one and each element names it, the pixel writer builds one luminance `Mask` (a second `RenderContext` and
  `Pixmap` at the canvas size, because a mask of another size is ignored without a word), and the canvas pixels of each
  distinct mask are charged once to `MAX_FILTER_PIXELS`, which now says "the blur of its shadow" or "its mask". Per shape,
  a fifty-glyph headline at 1920 by 1080 would have charged 100 million pixels, built fifty masks and written fifty
  `<mask>`s. Equal masks built apart (two steps, or shapes put back into a drawing one by one, each of which makes a step) are one: the pointer is the quick test, equality the fallback.
- **Clip and mask share the shape's outer group** (SVG: the attributes go on the element or `<g>` that has the opacity and
  blend; pixels: one layer with the clip path, blend, opacity and mask), so a shadow is cut with the shape and a blend still
  mixes with the backdrop. A `<g clip-path>` around the element would isolate the blend; the one group does not. A knockout
  ignores both, like its colours, opacity and blend. A region that fills nothing draws nothing (not "no clip").
- **A mask holds no masks; a shape takes one.** The mask's shapes carry no mask, refused at the `mask` step naming `source`,
  so the depth of what a mask draws is bounded by construction, not by a limit. A mask's shapes may be clipped. A second
  `mask` on a masked shape is refused (the two would multiply, which nothing asked for); a second `clip` cuts to both
  (the regions are intersected once for each distinct earlier clip).
- **Ids of what a mask draws** are prefixed with the mask's id (`mask-3-shadow-0`, `mask-3-gradient-0-fill`), built in
  the one place that makes an id, because the shapes of a mask are numbered from 0 again and a shadow, halftone or
  gradient of shape 0 in the mask and in the drawing would otherwise share an id.
- **A tolerance for the luminance.** `vello_cpu` rounds the luminance of a mask pixel to nearest, `resvg` (tiny-skia) rounds it
  up: they differ by about one level, and the SVG tests of masks allow what the other rows of the file do (interior 4,
  edges 40 over 1%). A clip or mask in a layer composites an edge pixel a level off a direct draw (a white mask against
  none, 1 level), the rasteriser's own 8-bit rounding. Neither is worked around (CLAUDE.md, rule 2).
- **The mesh refuses a clip and a mask** like a gradient: `ParameterError` naming `clip` or `mask` and the shape, which
  uses the same wording, "use to_rgba or to_svg".
- **`gradient_map` and `posterize` stay unregistered** (docs only): they recolour fills and strokes, and there is nothing
  to recolour yet. When they are added they must map the stops of a gradient, not skip it.

## Render a window (V10e)

- **A window is a view shift, not a clip** (2026-11-13). `Drawing::to_rgba_window(size, Window::px(x, y, w, h), values)`
  places the design frame in the whole `size`, moves the view by the region's top left and draws a canvas of the region's
  size: no geometry is culled for the window except by the tests that already cull for the canvas. Halftone lattices,
  gradients and text stay anchored in design space because only the output offset of the `View` changes. `to_rgba` is the
  window that is the whole canvas, so there is one path. A Param named `window` is refused as `into` is, because in Python
  the option sits beside the Params.
- **Limits** apply to the window (65,000 a side, 2^27 pixels), not to the canvas, which keeps the design frame's 100,000. *Reason:*
  the property is rendering in bands what a whole render refuses. They apply to the region too (below), so a window of 64,700
  pixels or more across a canvas past the limit is refused.
- **One owner of the margin.** `output/window.rs` holds the window, its checks and the region; `raster.rs` tells it what the
  blurred shadows need (`blur_reach`: the offset, and twelve times the grid of `2^halvings`), and no effect sizes anything of its own. The region is the window grown by that margin, its top left on the
  decimation grid (the rasteriser halves the layer from its corner, so another corner gives another blur), cut to the canvas.
  Masks, clips and every layer are made of the region's size, so `MAX_FILTER_PIXELS` charges the region, not the canvas. A
  region past the pixel limits raises `ParameterError` naming `window`. The decimation plan is repeated from `vello_common`
  (not exported); a test of a blur whose grid is ignored fails.
- **A window is the bytes of the crop** (2026-11-15). The oracle is the whole render cut by hand, over 32 drawings (fills; strokes
  of each join and end; booleans; opacity groups; blurred shadows at and past the canvas edge, at far offsets and of shapes whose
  corners lie off every grid; hard shadows; masks; clips; gradients; blends; knockouts; halftones; text), on two canvases (331 by
  251 and 700 by 300 pixels), over 1-pixel windows, windows on each edge, offsets no multiple of a tile, a wide tile or a blur's
  grid, and 80 random windows (700 once, by hand, with none different); every one is the same bytes, and a 4 by 3 grid of
  windows stitches into the whole. It was not before (a level at some edge pixels of fills, strokes, masks, gradients,
  halftones, groups under opacity and blends; two on the 700 pixel canvas for blurred shadows). Four causes, each found by
  making it fail on its own:
  - *The `f32` of a shifted coordinate.* The rasteriser holds coordinates in `f32`, and the nearest `f32` of a coordinate
    moved by a whole number of pixels is not the move of the nearest. Every coordinate is now rounded once to a grid of 1/256
    pixel in the coordinates of the whole picture (`View::place`; a pixel of 2^16 and 8 places hold 24 bits), and then
    moved by the region's corner, which is exact. An edge moves by at most 1/512 pixel, from the whole render as before. A
    stroke's outline and the dots of its ends are made in the whole picture's coordinates, as every window makes them, and
    placed after; so is the offset of a hard shadow. The grid is the pixel output's: SVG and mesh coordinates are unchanged.
  - *The tiles.* The rasteriser composites a layer (opacity, blend, mask, clip) in wide tiles of 256 pixels across and 4 down,
    and a pixel within a tile of the edge of what it is given gets a rounding of its own. A region starts on a multiple of
    256 across and of 4 down (of the blur's grid if that is more) and holds 8 pixels more than the window on each side; the
    canvas's own edge is the whole picture's edge. Without either, windows differed by a level (the 1-pixel ones included).
  - *The reach of a blur.* Ten grids of the blur's coarsest halving fell short by a level on the 700 pixel canvas where it had
    held on the 331 pixel one; eleven held; twelve are taken (`Shadow::dependence`).
  - *The corner of a shadow's layer.* The rasteriser halves a blur's layer from the corner of the tight box of what is drawn
    into it, rounded to a tile, so a window that cut a shape's corner off halved from another corner than the whole picture
    did. The region reaches back to the corner of every blurred shape whose shadow it touches (`Blur`, `Cells::reaching`,
    repeated until no more shapes are touched); a shape too large for the limits then raises `ParameterError` naming
    `window`, where an unexact one would have been drawn. A shape the canvas's own edge cuts needs nothing.
  The cost is a region up to 263 pixels wider on the left and 16 on the other sides than the window, and a larger one around a
  blurred shadow's shape.
  Rule 2 of CLAUDE.md asks to loosen a test rather than add a special path for a difference nobody can see; here the caller
  (scene) bands only what is bit-exact, so the difference is visible to it, and the fix is one grid, one margin and one corner,
  no special path. The WebAssembly window is the native window's bytes up to the differential's tolerance as the whole render
  is (`compare.mjs`); the native window is exactly the whole's crop in the Node tests too. Python and WebAssembly share the
  core: one drawing is tested through each.
- **A shadow's offset is part of the margin.** The test of a shadow offset up and left (its source below and right of the
  region) fails without it, by up to 95 levels; one offset down and right passed without it. Cutting the region to the
  canvas changed no pixel in the tests (a shape running off the canvas included); it keeps a whole render, which is the
  window that is the canvas, within the limits of `to_rgba` whatever its blur asks beyond the canvas.

## Text runs (V10f)

- **One `text`, runs inside it** (2026-11-13). A text of runs is `text` whose `content` is a list of `Run`s; a plain text is
  the text of one run, so there is one layout path and no second system. The "runs per location" of `type/layout.rs`
  (stretches of one font at one axis location, shaped alone) were extended, not repeated: a location is now a run and its
  axis values, so a run boundary is a shaping boundary exactly as a change of axis value is, and every per-location fact
  (size, tracking, features, cap height) is read from the location's run. *Reason:* wrapping, fitting, alignment, `inside`
  and `along` already work on locations; they work on runs by the same code.
- **What a run takes from the text.** `size` and `tracking` are the text's unless the run gives its own; `font`, `axes` and
  `features` are the run's, because an axis tag or a feature means something only in a font. A text of runs that also gives
  `font`, `axes` or `features` is refused, rather than guessing which font a run without one would take. *Rejected:* a text
  `font` as the default of runs (two ways to write one font, and `Text::new(&font)` with runs then means two things).
- **A run is a record read like a call's arguments.** `Kind::Runs` is an array of objects, each read, written, validated,
  keyed and put into the schema by the code that reads a call's arguments, from the one table `RUN` (`node/text.rs`: content,
  font, size, axes, features, tracking). The table is a `Verb` that is never registered, so a run is no call and `verbs()`
  does not list it. *Reason:* a second set of readers and writers for the same six arguments was the alternative; this way a
  run's `axes` is the per-character argument `text` has, its `size` the read-once number, with the same domains and errors,
  and the generated schema and the draft agree on it (`tests/test_drawing_schema.py`). Several fonts need no new input rule:
  every font a run names is an input with its `sha256`, and equal bytes are one input.
- **Boundaries and words.** No kerning, ligature or join reaches across a run boundary (a run of `A` then a run of `V` are
  not kerned). Wrapping breaks at the whitespace of the joined text, so a word that spans runs is never broken inside and is
  one `word`. *Rejected:* shaping across the boundary when the fonts are the same, which would make the result depend on
  whether two runs happen to hold one font.
- **Sizes and lines.** A line's leading is 1.2 times its largest run's size (given `leading` wins), each line standing its own
  leading below the one before when they differ, as fitted lines already did. `fit="size"` is one factor for all runs on a
  line, fixed at the reference pose like every fit, and the line is then drawn at those sizes whatever a size `Param` is at.
  `fit` on an axis sets the same fraction of each font's range in every run whose font has the axis (a run whose font lacks it keeps
  its look); the search is over that fraction, not an axis value, because the fonts' ranges differ. `inside` plans with the
  largest size and the tallest capital of any run, since its baselines are placed before the lines exist: a run much smaller
  than the largest sits on baselines spaced for the largest (a known limit).
- **Limits.** `MAX_RUNS` is 1,000: every run builds its font at its own locations, which is the cost a text of many runs
  pays; the characters of all runs share `MAX_GLYPHS`.
- **The `run` attribute** (the run's index, from 0; a plain text is run 0) is selectable like `line` (`only(run=2)`,
  `merge(by="run")`); `glyph`, `word` and `line` count through all runs.

## Path modifiers (V11)

- **`dash` (2026-11-16).** `[on, off, ...]`; an odd list repeats itself (SVG `stroke-dasharray`). Every length must be above
  0 (api.md: "a length not above 0" is an error), so a zero-length dash or gap is refused rather than meaning a dot or no
  gap. `phase` is in cycles of the whole even pattern and moves the dashes **forward** along the path (SVG's
  `stroke-dashoffset` moves them back): an animated phase 0 to 1 is marching ants in the direction of the path, and a
  whole cycle is the same picture. Pieces are open; a closed sub-path wholly inside one dash stays closed (as `slice`
  keeps a whole closed outline). A closed sub-path's last and first dash are not joined (a known limit, in api.md).
  The pattern is the first list-of-numbers argument (`Kind::Numbers`), each entry an expression as formats.md allows.
  The limit is a bound on the pieces, not the pieces left: (cycles crossed + 1) x dashes per cycle is charged to `MAX_SEGMENTS` before any is made,
  and a pattern has at most `MAX_DASH_PATTERN` (1,000) lengths. *Cost if wrong:* the direction or the closed-start
  rule changes pictures of a saved drawing, so decide before a release.

- **`place_along` (2026-11-16).** Copies of the whole drawing at stops along a path, carried and turned, never bent
  (`Drawing` only: it needs attributes and a list that changes length, as `merge` does). Rulings:
  *Ruling: the stamp is the whole list, placed by one anchor (the origin all shapes share, else the middle of their
  overall bounds, as `rotate`'s `about`) — per-shape origins would collapse a stamp onto the path point — an arrow of
  two shapes would lose its layout if wrong.*
  *Ruling: `count` and `spacing` are plain numbers, never Params — a per-shape Param list is one number for the shape
  with `index` i at the step that reads it, so a shape count that moves with a value changes what every later list and
  `by_index` mean; `start` stays animatable (one value, not a list) — animating the count would need a new argument
  kind and a rule for the lists; cost if wrong: a count cannot be animated, scale copies to 0 instead.*
  *Ruling: exactly one of `count` and `spacing` — Rust takes `Spacing::Count(n)` or `Spacing::Every(px)` so the bad
  state cannot be written; the registry and schema have two optional keys, and the loader (the verb's `check`) refuses
  both or neither; the schema cannot say "exactly one" (the same limit as `Kind::Runs`) and `count` is a `number` with
  a minimum of 1, so a fraction is refused when drawn, not by the schema — cost: an editor does not flag it.*
  *Ruling: open path, `count` includes both ends (one copy at the start); `spacing` stops at 0, s, 2s, ... up to the
  end with a 1e-12 relative slack so a position a rounding error short of the end counts as on it; closed path
  (every sub-path with a length closed, `along`'s rule), `count` n is i·L/n, `spacing` is floor(L/s) stops, at least
  one — no two closer than `spacing` (ceil would leave a last gap shorter than it) — a closed path with `spacing`
  not dividing L has a last gap longer than `spacing`.*
  *Ruling: positions and direction come from `along`'s walk (`Guide::frame`), the curve's own direction at the place,
  the leaving direction at a corner; the turn uses the unit direction itself (no angle, no trig), so a horizontal or
  vertical path turns exactly.*
  *Ruling: `origin` moves with its shape (it is on the path point when it was the anchor), unlike `along`, which leaves
  it, because a placed shape is a rigid copy.*
  *Ruling: `copy` is written as `repeat` will write it; `order` is rewritten to the share of the copy only for a stamp
  of one shape (a lone shape's `order` carries nothing, and `Drawing(path)` writes 0 on it), else the shapes keep their
  own — so `by_order` varies per copy for beads and per letter for a stamped word; per-item numbers index shapes, so
  for a many-shape stamp `merge(by="copy")` or `only(copy=..)` selects per copy; `repeat` (not built) must follow.*
  *Ruling: `turn` is a boolean in Rust and Python and the word `"along"` or `"none"` in a document — a boolean
  argument kind would touch the reader, writer, schema and keys for one flag — a document says `"turn": "none"` where
  Python says `turn=False`.*
  Copies times shapes is charged to `MAX_SHAPES` and `MAX_SEGMENTS` before any copy is made.

- **`simplify` (2026-11-16).** Schneider's fit (Graphics Gems, 1990) per run between corners, in `path/simplify.rs`. The fit is greedy (a run is split where the fit fails), so the result has
  few curves, not the fewest.
  *Ruling: a corner is a turn of more than 45 degrees between neighbouring edges, an internal `// constant:`, not an
  argument (api.md fixes `simplify(*, tolerance=0.5)` and no example needs it) — a jittery trace that turns sharply at
  every point keeps its jitter as corners and is returned nearly as it was; cost if wrong: add `corner_angle` as a
  literal argument later (a new optional key, nothing saved breaks).*
  *Ruling: a cubic is accepted only when it is proven within `tolerance` of the run along its whole length: exact at
  every point, and between them by a speed bound (a cubic moves at most 3 times its longest control leg per unit of
  parameter) — an interval of an edge's parameter span is proven when the distance of its middle from the edge plus
  the reach of that bound is within the tolerance, else it is halved; a run whose points all lie within `tolerance`
  of its chord is one line (exact for collinear points, so a dense square is four `L`) — Schneider measures at the
  points only, and a smooth cubic through the 12 corners of a circle of radius 100 bulges 3.4 pixels off the edges;
  sampling fixed shares between points (the first version: quarter, half, three quarters) still let a cubic overshoot
  by 2 to 9%, caught by the property test; cost: a fit that needs more than 256 interval checks per point (just
  inside the tolerance, or long edges at a small tolerance) is split instead, down to the input's own segments, and a
  short flat run next to a curve is a line with a tangent kink up to the tolerance in size.*
  *Ruling: a split falls in the middle half of its run (at the worst point where that is inside it) — splitting at the
  worst point alone can peel one point at a time and is quadratic in the points; the middle half
  bounds the work to O(n log n) and the stack of an explicit loop to about log n jobs, so no recursion and no stack
  test; cost: sometimes a segment more than the unrestricted fit.*
  *Ruling: a sub-path never gets more segments than it had, it is kept as it was when the fit would have more (four
  cubics of a circle stay four) — "simplify" that adds segments would surprise; cost: a fitted sub-path is compared
  with the stored segment count, so a curve already within tolerance is never re-fitted smoother.*
  *Ruling: a closed sub-path with no corner is fitted as one loop from its written start with a centred tangent there,
  and a corner-ful one splits at its written start too (smooth unless the start is itself a corner), so `start`
  survives — a rotated start would change what `slice` and `dash` measure from.* The flattened edges are charged to
  `MAX_SEGMENTS` as `along` charges them, and the output never has more segments than that.

- **Tapered outline (2026-11-16).** `Path::tapered_outline(width, &profile, tolerance)` in `path/taper.rs`, the kernel
  `stroke(profile=)` calls; `outline(profile=)` is its Path and Drawing form. Built from the centre line the measured
  walk gives, cut at the profile's stops so the radius is linear on each straight piece, and the union (one `i_overlay`
  run, via `offset.rs`) of the convex hulls of consecutive end discs, whose corners are on the circles: exactly the sweep
  of a disc of linearly changing radius, so the oracle is a brute-force distance to sampled discs.
  *Ruling: positions are arc length per sub-path, 0 at its start and 1 at its end — settled from three examples: a
  brush stroke on one open wave tapers both ends either way; `dash(...).outline(profile=)` must taper each dash (the
  pencil-stroke look), which a position along the whole path cannot (only the first and last dash would thin); `text`
  letters and the two contours of an "O" each taper on their own, a global position would taper only the first and last
  contour — a position along the whole path (draw the profile once over a many-piece path) is not possible; join the
  pieces first, then outline.*
  *Ruling: a closed sub-path is walked from its written start (as `resample`), and the profile runs once round it; where
  the end meets the start the width steps from the last factor to the first (both discs are in the union, so the larger
  shows) — a taper to 0 on a ring pinches it at its start (a brush stroke closing on itself), equal end factors give a
  seamless ring; cost if wrong: a ring that should have no seam shows a step unless the profile's ends are equal.*
  *Ruling: joins and caps are round, so `join` and `cap` other than `"round"` with a profile are a `ParameterError`
  naming the argument (the verb check, the core's `require_round_stroke` for Python), and `miter_limit` is unused — the
  width changes the sweep of a disc; a mitre or square end of a changing width has no single definition; cost if wrong:
  a mitred taper needs a different kernel.*
  *Ruling: the centre line is the walk of `length` (0.01 flattening), `tolerance` bounds the discs' polygons — one
  arc-length system, so a position agrees with `dash` and `slice`; cost: a `tolerance` below 0.01 does not refine the
  curve itself.*
  *Ruling: a sub-path of no length (a lone move, a line to its own start) paints nothing, as plain `outline` does —
  one rule for the verb; cost: a dot of a taper needs the plain stroke.*
  *Ruling: a profile is its own argument kind (`Kind::Pairs`, `[[position, factor], ...]`, each number a per-item
  expression) and its own limit `MAX_PROFILE_STOPS` (1,000), not `MAX_DASH_PATTERN`; every number is read per shape, the
  kernel validates the whole profile when drawn (positions and factors, naming `profile`), the loader checks the count
  and the round join and cap.*
  The corners of the hulls (pieces times the corners of a disc of the largest radius, which grows as sqrt(radius /
  tolerance)) are charged to `MAX_SEGMENTS` before any is built, so a huge `width` at a fine `tolerance` is a
  `ParameterError` naming `width`.

- **Tapered stroke (2026-11-16).** `stroke(..., profile=)` stores the stops on the `Stroke` style; the path stays the centre
  line. Rulings:
  *Ruling: the outline is made at draw time from the shape's current path, once per shape and output, in one function
  (`tapered_region` in `output/outline.rs`) that SVG, pixels and mesh all call, to a fixed 0.01 design pixels — not at the
  step (selection, bounds, `slice` and `dash` after the stroke keep seeing the centre line, and `slice(0, progress)` thins
  the tip where the path now ends) and not scaled by the output (the SVG at twice the size has numbers exactly twice as
  large, and the three outputs draw one polygon); cost if wrong: a very large output shows the facets of 0.01 design pixel
  discs, and an outline is rebuilt on every draw (a cache of it needs the key of path and profile).*
  *Ruling: a profiled stroke is a filled region in every output, with the stroke's paint; in SVG a shape with a fill and a
  profiled stroke is two `<path>`s, fill first, and written inside a `<g>` carrying opacity, blend, clip and mask when it has
  any, so they apply once to both, and a shadow's tinted copy is dimmed by one group; the mesh fills it as it fills a shape, so
  nothing it refused before is newly refused; cost if wrong: none seen, the SVG has two elements where a plain stroke has one.*
  *Ruling: `join` and `cap` other than `"round"` with a profile, a profile of no or over `MAX_PROFILE_STOPS` stops and a
  position or factor the kernel refuses are refused in the step (the verb's check, and per shape where the numbers are read),
  naming it; an outline over `MAX_SEGMENTS` is refused when drawn, naming the shape; cost if wrong: a Param-driven profile
  is refused when its value is read, not when the drawing is written.*
  *Ruling: a shadow follows the stroke's region as a plain stroke's shadow follows its outline, and a stroke whose
  reach is measured (the pixel margin of a blur) is half the width times the largest factor.*
  *Measured: against `resvg` a half-transparent tapered stroke differs by up to 10 levels in pixels that are not edges (the
  tip is thinner than a pixel and covered by under 24 levels), so the oracle rows of `style_taper.rs` allow 10 where the
  other effects allow 4; a missing or doubled translucent stroke is 128.*

## Deformers (V12)

- **One warp kernel; `wave` (2026-11-16).** Every deformer is a map of the plane and a step length; one kernel
  (`path/warp.rs`) flattens the path, cuts each straight edge into `ceil(length / step)` equal pieces keeping every original
  vertex, maps every point, keeps each sub-path closed or open, keeps and maps a lone point, and refuses a result beyond
  ±1e100. The pieces of all the sub-paths are counted and charged to `MAX_SEGMENTS` before any is allocated, so a step
  small next to the path is a `ParameterError`, never an OOM or a hang. *Reason:* resampling first (V11 `resample`) does not keep corners (it chamfers a
  sharp apex), and a deformer must not move a corner off the outline it deforms. `along` stays as it was: it halves an edge
  where its own bend is not straight, not into equal pieces, and its bytes are fixed by its tests.
  *Ruling: `wave` displaces `amplitude * sin(2 pi (d / wavelength - phase))` along the perpendicular of the direction of
  travel, `d` the distance along it, `y` down like `rotate` (angle 0 displaces down the screen); `phase` is in cycles and
  reduced mod 1 before any maths, so a whole cycle more is the same bits for a phase that is exact (0.25 and 1.25); a phase
  such as 0.3 differs from 1.3 by the rounding of the addition; cost if wrong: a saved drawing's wave runs the other way.*
  *Ruling: step = wavelength / 24, not the 16 first planned (the relative arm of the error; the absolute budget of "One absolute error budget" below caps it). A chord of a sine over a step `h` strays by at most
  `amplitude (2 pi h / wavelength)^2 / 8`: 1.93% of the amplitude for 16 pieces, 0.86% for 24. The tests hold the exact image
  of every sampled point within 1% of the amplitude (plus 0.01 pixel of flattening) of the output, which 16 fails (checked: the
  closed-form test fails with 16); a step is also never above its edge, so a short edge is one piece. Cost if wrong: 1.5 times
  the points of a 16-piece wave, a hair of speed.*
  *Ruling: field anchoring. The wave is a function of the point's own design-frame coordinates, not of the shape's origin or
  bounds, so a wave runs continuously across the letters of a word. Cost: letters that translate swim through the field (a
  `translate` before the wave changes which part of the wave they are in), and `translate` after the wave moves the wave with the
  shape; a wave that follows each shape needs the shape's own frame (move it to the origin, wave, move it back).*
  *Ruling: a coordinate beyond ±1e100 after the displacement names `amplitude` (the only argument that can send a point
  there from a path within range); a count past `MAX_SEGMENTS` names `wavelength`, the argument that sets the step.*
  *Ruling: noise (`noise.rs`). Value noise on a three-axis lattice: `x` and `y` in wavelengths (one cell a wavelength) and the
  phase, whose cell index wraps mod 4, so a cycle is four cells and `phase + 1` is `phase` to the bit (the phase is reduced
  mod 1 before any maths; 0.25 and 0.5 are tested for equality). A lattice value is the top 53 bits of the library's one
  integer hash (`SplitMix64`, the generator of `random`, now `expr::splitmix`) chained over seed-and-stream, `ix`, `iy` and
  the phase cell, mapped to [-1, 1); the blend is the quintic fade, trilinear, so a value is a convex combination of
  lattice values and never leaves [-1, 1], and the field has continuous slope and curvature. `amplitude` means: each of `dx` and
  `dy` is at most `amplitude` (a displacement at most `amplitude * sqrt(2)`); the tests check the bound over a dense sample.
  Constants (derived in `tests/it/path_wobble.rs`): slope at most 3.75 per cell per axis, 15 per cycle of phase; second
  derivative along a line at most 25.6 per field per cell squared (twice the fade's own maximum 10 sqrt(3) / 3 = 5.77, at
  u = 0.2113, plus the mixed term 4 (15/8)^2), at most 36.2 for a displacement of two fields. Cost if wrong: a saved drawing's noise changes (the hash or the
  cell layout is part of the file's meaning), so a later flow field that shares the module shares this field.*
  *Ruling: `wobble` displaces by two fields, streams 0 (`dx`) and 1 (`dy`) of the one seed (`splitmix(seed, stream)`), so one
  seed is one wobble; step = wavelength / 24: a chord over a step `h` strays from the curve by at most `sqrt(2) * 25.6 / 8 *
  (h / wavelength)^2` of the amplitude, 0.79% at 24 (the bound is loose; the tests hold the cut line within 1% of the
  amplitude of a 400th-of-a-wavelength resampling of the same line, and 3 pieces fail). A fractional seed is refused at the
  draw, not when the call is made, as it is a plain number (a `random` seed is also checked early only as a constant).*

- **`twist` and `bulge`: radial deformers (2026-11-16, V12 task 3).** Designed from three usage sketches before any
  signature: `text("O", ...).twist(90)` (the middle of the letter turns a quarter, the rim holds still, so the strokes
  curl); `circle.bulge(0.8, radius=60)` and `circle.bulge(-0.5)` (a fish-eye, a pinch); `text(...).twist(progress * 180)`
  with a Param. They settled the form `twist(angle=90, *, radius=None, about=None)` and `bulge(amount=0.5, *, radius=None,
  about=None)`: the one thing a person gives is the strength; the radius and the pivot have a reasonable default (the shape
  itself) and are named when wanted. `about` and the default pivot are those of `rotate` and `scale` (the shape's `origin`,
  else the middle of its bounds), unlike `wave` and `wobble`, whose field is anchored in the design frame: a twist is a
  property of one shape's own centre.
  *Ruling: `twist` turns a point at distance `r` from `about` by `angle * falloff(r)` degrees with `falloff(r) = 1 - s^2 (3
  - 2s)`, `s = min(r / radius, 1)`, clockwise on the screen like `rotate` (the same rotation matrix, `y` down); a point at
  or beyond `radius` is returned as it was, to the bit. The smoothstep is 1 at the centre, 0 at `radius` and has slope 0 at
  both, so the twist has no kink at the centre or at the rim; a linear falloff would show a crease at `radius`. The turn
  of a point depends on its distance only, so a twist keeps every distance to `about` (tested on rays from the pivot, to
  1e-9 relative). Cost if wrong: a saved drawing's twist is another shape (the falloff is part of the file's meaning).*
  *Ruling: `bulge` moves a point to `r * (1 + amount * falloff(r))` along its ray, the same falloff. The slope of `r -> r'` is
  `1 + amount h(s)`, `h(s) = 1 - 9 s^2 + 8 s^3`; `h` is 1 at `s = 0`, 0 at `s = 1`, and has its minimum -11/16 at `s = 3/4`
  (`h' = 6 s (4 s - 3)`), so the map is strictly increasing, and nothing folds over, exactly for `-1 < amount < 16/11`.
  `amount` outside that is a `ParameterError` naming `amount`; a literal is refused at the call through a new domain,
  `Domain::Between(low, high)` (above both bounds, below the second: written to the generated schema as
  `exclusiveMinimum`/`exclusiveMaximum`, and by hand in the committed one). Cost if wrong: the smoothstep falloff cannot
  push a middle further than 1.45 times (a fish-eye of more is a later falloff, not a larger amount); the refusal of a
  valid fold-over that a person wants (self-overlap as an effect) is deliberate, as `along` refuses nothing and folds are
  not reachable by accident here.*
  *Ruling: `radius` defaults to the distance from `about` to the farthest corner of the shape's bounds (the whole shape
  twists, its far corner stays); 1 if that is 0 (a point at `about`), so the default never refuses. A `radius` of 0, negative,
  NaN, infinite or past 1e100 names `radius`. The default is derived per shape, so on a `Drawing` the letters of a word
  twist each about its own middle and far corner, not as one. Cost if wrong: a word twisted as one body needs `about` and
  `radius` given (a number for each shape).*
  *Ruling: the step is `radius / n` (the relative arm; the absolute budget of "One absolute error budget" below caps it), not a design number: the kernel needs a length and the map's own scale is the radius. A
  chord over a step `h` strays from the image of its edge by at most `h^2 / 8` times the largest second derivative of the
  image along the line. Twist, `a` the angle in radians, `phi = a f(r)`: `|f'| <= 1.5 / radius`, `|f''| <= 6 / radius^2`, so
  `|image''| <= |phi''| |d| + phi'^2 |d| + 2 |phi'| <= (10.5 a + 2.25 a^2) / radius` and the chord error is within `0.005 *
  radius` for `n = 7.5 sqrt(a^2 + 14 a / 3)` (22 at 90 degrees, 60 at a turn): the pieces grow with the angle, as a map
  that turns more bends more. Bulge: `|image''| <= 9 |amount| / radius`, `n = 15 sqrt(|amount|)`. The error budget is half of
  the hundredth of the radius the tests hold the output to (`0.01 * radius + 0.01` for the flattening of curves, against
  the exact images of a dense sampling); checked: `0.04` instead of `0.005` fails the twist closed-form tests, so the bound
  is exercised (it is loose for bulge, whose real error is a fraction of the bound). `// constant: CHORD_ERROR`. Cost if
  wrong: pieces go as `1 / sqrt(error)`, so a bound 8 times looser makes 2.8 times fewer points; a hair of speed.*
  *Ruling: a count past `MAX_SEGMENTS` names `radius` (the argument that sets the step), a turn so large that not even one
  radius can be cut into `MAX_SEGMENTS` pieces names `angle` (it alone sets the step then), a point beyond ±1e100 names
  `amount` for a bulge and `angle` for a twist. A long edge far outside the radius is cut into pieces of the same length
  although nothing moves there (the kernel cuts every edge alike); a huge shape with a small `radius` is refused rather
  than cut short, and the message is the same `MAX_SEGMENTS` one. Cost if wrong: a refusal where a smarter cut would
  have succeeded; the fix is in the kernel (cut only within the radius), not in the verbs.*
  *Ruling: neither verb moves the `origin` attribute (about the origin it is the fixed point of the map; about another
  `about` the attribute stays where it was all the same, as for `wave`);
  `rotate` and `scale`, which are rigid or affine maps, move it with the shape. Cost if wrong: a later turn about the
  origin of a bulged shape is about the old origin.*
  *Housekeeping: the loader's test of an unknown call named `wobble`, which the wobble batch registered (a test that
  failed at that commit, found by this batch's run); it names `morph` (V13) now.*

- **`envelope`, `arch` and `Path.arc` (2026-11-16, V12 task 4).** Designed from a look at a word on a contact sheet (a
  design spike, discarded) before any signature. Three forms were tried for bending a word: `along` an arc (letters fanned
  round the curve), a frame mapped between a curve and a flat bottom, and between a curve and the same curve lower. The
  fan (`along`) turns every letter to the arc's direction and can wrap past half a circle; it is for text on a path. The
  frame map keeps the columns of the word straight (a letter is sheared and stretched, never turned) and its height under
  control: it is the banner, the flag, the arch of a logo. Use `along` for a word that follows a curve, `envelope` or
  `arch` for a word that is shaped by one. The forms settled as `envelope(top, bottom=None, *, frame=None, amount=1)`,
  `arch(bend, *, frame=None)` and the constructor `Path.arc(center, radius, start, end)` that the curve needs (`from_svg_path`
  was the only way to an arc, and a circle is closed).
  *Ruling: the map is `lerp(T(u), B(u), v)`, `u` and `v` the shares of the frame, `T` and `B` the points of `top` and
  `bottom` at that share of their lengths, walked by `along`'s guide (straight past an open end, wrapping a closed curve
  with its seam at its start). A column stays straight; `bottom` left out is the frame's straight bottom edge, the flat
  baseline of the classic arch. Cost if wrong: a saved drawing's bend is another shape.*
  *Ruling: `amount` blends each edge between the frame's straight edge and the curve, `lerp(straight(u), T(u), amount)`, any
  finite number. `top` and `bottom` are literal paths in a document (a `Kind::Path`, as for `along`), so without `amount` a
  bend could not animate; with it `amount * progress` does, and a negative amount bends the other way. Cost if wrong: one
  more number on a verb that could have done without; 1 is the default and costs nothing to ignore.*
  *Ruling: the default frame is the bounds of the whole drawing (all shapes, read when the step is drawn), shared by every
  shape; on a `Path` its own bounds. A frame per shape collapses a line of a word into a block (each letter fills the
  frame's height: the sheet showed it), while one frame keeps the lines, the x-height and the spaces where they were.
  `frame=(x, y, width, height)` overrides it, one for every shape; a bend per shape is made by drawing the shapes apart. A
  frame with no width or height (a single horizontal line has no height of its own) is a `ParameterError` naming `frame`,
  not a silent block. The frame is four numbers, read per item, as `Kind::Rect` (a `Kind::Numbers` of exactly four: the
  generated and the hand-written schema both say `minItems` and `maxItems` 4, the loader and the step check the count too).*
  *Ruling: live bounds. `fit` (V6) reads its layout at the reference pose (every Param at its default) so that an animated
  weight does not make the line breathe; the frame here is read from the shapes the step receives, at the live values, so
  a word whose weight is animated changes its width and the arch's frame changes with it. The evaluator hands a node its
  input shapes only; reading the reference pose of the input needs the previous steps evaluated a second time at the
  defaults, which is a change to the evaluation machinery (the cache keys by what a step reads), not to this verb. Cost if
  wrong: an arch over a breathing word breathes by the same amount; a fix is a frame given (`frame=`), or later a
  `frame="reference"` that evaluates the input at the calm pose.*
  *Ruling: `arch` is a parabola; `bend` is the lift of the middle of both edges as a fraction of the frame's width,
  positive up the screen. A second verb exists because the most asked-for shape (a word on an arch) needs no geometry from the
  caller and animates on one number, and because its closed form lets `u` follow `x`, so the spacing of the letters is
  kept, which an `envelope` with an arc cannot do (it spaces by arc length: the letters near a steep part of the arc crowd
  together). The two share one map (`lattice` in `path/envelope.rs`: the logic of `u`, `v` and the blend is in one place)
  and differ in the edge functions only. Cost if wrong: a circular arch (a constant curvature) is `envelope` with
  `Path.arc`.*
  *Ruling: the orientation of an envelope is the order of its curves: v interpolates from `top` (0) to `bottom` (1), so
  `bottom` lying above `top` turns the word upside down (swapping the two mirrors the result: each point goes to the
  original's with v replaced by 1 - v), and curves that cross fold it. Documented, not refused: a flipped banner is a
  look a caller may want, and "above" has no meaning for curves that cross or for closed ones. Cost: a flip is silent,
  a caller who gives the curves in the wrong order sees an upside-down word and no error.*
  *Ruling: `arch` bends the whole band: the top and the bottom edge follow the same parabola, so the frame's height is kept
  at every column and `-bend` is the exact mirror of `+bend` (the displacement of every point reverses); `arch(0)` is the
  identity. The first definition lifted the top only and kept the bottom flat: a bend of -0.3 on a 365 by 150 word left a
  column 40.5 tall in the middle (the top sank 109.5 toward the bottom) where +0.3 made it 259.5 (measured on the demo's
  frame; the contact sheet showed a word squashed to a sliver). Implemented by changing the bottom edge function of the
  shared `lattice` map only. Cost: the flat-baseline look is no longer one call (it is `envelope(top=<curve>)`, with an arc
  or a parabola path); a saved drawing that used `arch` changes shape; the vertical stretch of the middle of the old look
  is gone, so an arched word is no longer taller in the middle.*
  *Ruling: the step is a 256th of the frame's width, not a length of the map's own. The error of an edge comes from the
  curvature of the curves (a chord over `h` of a curve of length `L` strays by at most `curvature (L / 256)^2 / 8`), and for
  a slanted edge of a wide frame from the product of the change of the slope of the edge between the curves and the share it
  crosses (`slope W^2 / (4 * 256^2 H)`: 0.04 pixel for a frame 1000 by 100); the curves themselves are walked by the guide
  (flattened within 0.01 pixel). 128 pieces put a curve's chord 0.01 pixel off its curve, which the test of the top edge
  refuses (checked), so 256 is exercised. A tall vertical edge is cut as a long one although the map is linear along it (the
  kernel cuts every edge alike). Cost if wrong: pieces go as the root of the error; a frame much taller than wide is cut finer
  than it needs.*
  *Ruling: a count past `MAX_SEGMENTS` names `frame` (the argument that sets the step); `amount` and `bend` name a point beyond
  ±1e100; `top` or `bottom` empty or of no length is a `GeometryError` of `envelope`, as `along` has it, naming neither (the
  message is the guide's). The curves of a `Drawing` step are walked once for all shapes.*
  *Ruling: `Path.arc` takes degrees clockwise on the screen from +x (like `rotate`, `y` down) and a sweep `end - start` from
  -360 to 360 other than 0: 360 gives a full circle that is open and ends where it starts (no close; a stroke shows two caps), so
  that the sweep is exactly what is asked and no angle means "closed"; `Path.circle` is the closed one. It reuses the cubic
  cutter of `from_svg_path`'s arcs (one cubic a quarter turn at most, last end exact). Cost if wrong: a 0 sweep that someone
  wants as a point is refused naming `end`.*

- **One absolute error budget for every deformer (2026-11-16, V12 task 6).** The demo found that "smooth at any size" was
  false: each deformer cut edges so a chord's error was a *share* of its own scale (0.005 of the radius for `twist` and
  `bulge`, about 0.9% and 0.8% of the amplitude for `wave` and `wobble`, `PIECES` of the frame for `envelope` and
  `arch`), which on a big shape is 1 to 7 design pixels: `rect(120, 90, 360, 120).twist(60)` has a radius near 190 and a
  chord error near 0.95 px, and a 4x zoom shows the facets. The rule is now one: the chord error of any deformer is at
  most `DEFORM_ERROR_PX = 0.05` design pixels (0.4 device pixels at 8x zoom, below what shows). It lives in one place,
  the warp kernel: a verb passes its step and the chord error that step leaves (`WAVE_ERROR * |amplitude|`, `0.005 *
  radius`, ...), and the kernel shortens the step by `sqrt(0.05 / error)` where that error is above the budget (error goes
  as the square of the step). The per-verb relative bounds stay as the other arm of the minimum, so a small shape costs
  what it did. *Cost: more points on big shapes* (the demo's `rect(120, 90, 360, 120).twist(60)`: 94 to 406 points,
  `bulge(0.8)`: 70 to 296; a 90-glyph word at 1080p changes by 0% (wobble), 1% (wave), 7% (twist), 6% (bulge) because glyph edges are short; a
  4000 px square twisted with radius 2000: 148 to 2076), and *a wave of amplitude 8 now has 29 pieces to the wavelength's
  24* (0.0086 x 8 is above 0.05). A huge amplitude (1e50) is now refused by the limit like a tiny wavelength, naming
  `wavelength`; a displacement beyond 1e100 is still named (`amplitude`, `angle`, `bend`) because the kernel maps the
  original vertices and refuses an out-of-range one before it counts pieces, after it has refused a step that is too
  small by the verb's own cut (a tiny wavelength names `wavelength` as before). `envelope` has no closed form for its
  curve's error, so its relative bound is taken as 1e-4 of the frame's width (the half-circle figure worked at `PIECES`):
  frames wider than 500 px are cut finer; `arch` uses its parabola's exact `bend * width / PIECES^2`. A frame much wider than
  the 4000 px of the demo costs `sqrt(width)` more pieces. *Ruling: 0.05, not 0.1; the demo cases stay in the tens of
  thousands of points and 12 to 17 ms, and the limit of 10 million is reached only by amplitudes or shapes far past any
  design; cost if wrong: a looser constant is a one-line change and halves the extra points.*
  *Ruling: the refusal text of a number outside a `Domain::Between` is one string owned by the rule (`STRONGEST_BULGE_RULE`:
  "above -1 and below 16/11 (1.4545), so that the shape does not fold over"), carried by the domain and read by `Path::bulge`
  too, so `Path.bulge(1.5)` and `Drawing.bulge(1.5)` refuse in the same words (a Drawing adds only its step prefix). Other
  domains still read differently on the two routes (`radius` "must be a number above 0" on a `Path`, "must be above 0 and
  at most 1e100..." on a `Drawing`); those are not Between and are left as they were. Cost if wrong: none.*

## Morph (V13)

- **Spike (2026-11-16).** A throwaway script drew letters and words morphed by several designs and the PNGs were looked at
  (the script and its images are not kept). Compared: *pooling* a word's contours into one shape and pairing contours by
  position, against *pairing shapes by index* (glyph `i` to glyph `i`): pooling smears letters (`cat` to `horse`, `NO` to
  `YES`), per-glyph pairing reads letter by letter. A contour with no partner: an outer shrinking to a point is fine; for a
  hole, four designs: collapse to its own centre, collapse to the mapped point of the other shape's box, *wound* (lerped like
  a matched contour) and *subtracted* (lerped, then cut from the rest). Start alignment of a closed pair with and without a
  search over the reversed contour. What the PNGs showed: a wound unmatched hole is a proven defect, its reversed winding
  paints black where it leaves the moving body (`O` to `L` at 0.9 and 0.95, `B` to `L`, `O` to `C`: dark specks outside the
  letter, or in the counter); cutting the result to the body cannot paint outside, so no frame shows a speck (plain subtraction erased an outer inside the hole, found by review; the ruling below is the final form). Start alignment is by shift alone: a
  search that may reverse one contour would turn a hole into a fill. An unresolved glyph (`A` with a crossbar of its own) morphs as it is:
  the crossbar is an unmatched outer that shrinks, and `A` to `O` grows the counter from a point.
- *Ruling: unmatched holes are wound in the other way and the result is cut to the body (the intersection of the body
  with the holes wound in, and the body), so a hole cuts what the nonzero rule says and an outer inside it stays (an `@`'s inner letter;
  cutting the hole out of the body erased it, found by review) and nothing is painted outside the body; cost if wrong: a
  boolean per frame whenever a hole has no partner (the result is then canonical, the ends keep the fill but not the vertex
  list).*
- *Ruling: shapes pair by index and contours inside a shape by role (the sign of the area, after one reversal of the whole
  path if its largest contour runs the other way) and then by nearest centre in each shape's own box scaled to 0..1, ties to
  the lower indices (so `morph(a, b, t)` fills what `morph(b, a, 1 - t)` fills); cost if wrong: a word of another length pops
  letters in and out at their own place (a spare glyph grows from its own centre), and position-aware word pairing is a later
  item.*
- *Ruling: unresolved input stays unresolved, no hidden boolean; cost if wrong: overlapping contours of one glyph (an `E`
  made of four bars) morph as separate outers, so `resolve` the text first (`examples/glyph_morph.py` does).*
- *Ruling: `fraction` is any finite number, so an elastic or back ease that overshoots extrapolates the lerp; cost if wrong:
  the result can leave the hull of the two shapes, and a point past ±1e100 is a `ParameterError` naming `fraction`.*
- *Ruling: a contour with no partner stops at its point (past 1 for one that shrinks, before 0 for one that grows) and
  never comes back through it; found by the demo (a counter of `R` reappearing at `fraction` 1.15 on the way to a circle);
  cost if wrong: an unmatched contour does not follow the straight line a matched one does beyond the ends.*
- *Ruling: no search over the orientation of a contour; cost if wrong: shapes wound in opposite directions are normalised
  only as a whole, so a glyph that comes reversed from a font is fixed, a single hand-reversed contour is not.*
- *Ruling: points are the union of both outlines' vertex fractions (arc length normalised per contour), so 0 is the source and
  1 the target with no resolution to tune; an open pair is joined end to end or end to start, with no cyclic shift; cost if
  wrong: a pair of very different vertex counts yields as many points as both together.*
- *Ruling: `ALIGN_SAMPLES` = 128, the equal steps of length that decide where a closed pair starts, a `// constant` in
  `path/morph.rs`. Measured once over every ordered pair of the closed contours of the glyphs of `ABRSgq&@` in the bundled
  Bricolage Grotesque and of a circle (380 pairs): the cost of the start it picks (summed squared distance of 1024 samples) is
  at most 1.0005 times the best of the 1024 starts; a search costs the square of the samples. Cost if wrong: a start off by
  under a 128th of a perimeter.*
- *Ruling: limits: the points of the flattening, the pairs ranked and the points of each result contour are charged to
  `MAX_SEGMENTS` before any is allocated (a subtraction charges as the booleans do); on a `Drawing`, a target shape with no
  partner is a shape the step makes and counts against `MAX_SHAPES`.*
- *Ruling: on a `Drawing` the per-item `order` is the `order` attribute a shape carries: a pair keeps the source's, a shape
  that grows keeps the target's own (so `text("CAT")` and `text("HORSE")` give the glyphs 0, 0.5, 1 and 0.75, 1 under one
  `stagger`); cost if wrong: letters of the two words start at uneven times; `by_index` reads the index over the result.*
- *Later: the start of a contour jumps when an animated target changes (D7); style (colour) morph; position-aware word
  pairing.*

## Kinetic type piece (M3)

The piece is `examples/kinetic_type.{py,rs,mjs}` ("NO SLEEP" into "GO DREAM"); the library needed no change. A spike drew
nine frames before the timeline was fixed; a frame costs 12 to 20 ms at 540x675 and about 25 ms at 1080x1350, so 30 fps
holds.

- *Ruling: done is the piece running from the shipped examples in Python, Rust and Node with four frames equal as SVG text,
  and the Python clip; a live canvas page is not built yet. Cost if wrong: no browser demo yet.*
- *Ruling: easing (`smooth`, `kick`) is local to each example, no `btvec.ease`: easing and time are the caller's (CLAUDE.md,
  "State, not time"). Cost: each piece repeats a few lines.*
- *Ruling: the font is the Anybody subset (`wdth` 50 to 150, `wght` 100 to 900, capitals and digits): `fit="wdth"` fills the
  measure on both lines, which the bundled Bricolage (`wdth` 75 to 100) cannot. Cost: Rust and Node read a file (the Rust
  example compiles it in).*
- *Ruling: the arch's frame is the bounds of the calm pose, passed as `frame=`, so a pulse or a morph does not make the arch
  breathe. Both headlines are `resolve`d before `morph` and the target "GO DREAM" has the same shape of line (2 + 5
  letters, O into O, E into E). Cost: the mid-turn frames of `A` and `M` are rough (filed in the roadmap).*
- *Ruling: `pulse` is one number per character and drives the weight only. A per-character list counts the line break (8)
  and a per-item number counts shapes (7), so the same list cannot also scale each letter. Cost: no per-letter scale.*
- Rust needs `seed(7.0)`: `Expr` has no `From` for integers (the sketches that said `seed(7)` were wrong; they were deleted
  with the rest of `docs/examples/kinetic_type.*`, whose `btvec.ease` never existed; `docs/examples/json` stays).

## Fields and streamlines (V15a)

- **`Flow` and `Field` are values, `hatch` the verb.** A field is built once from literal numbers and one path and read with
  `at`; the call that uses it is `hatch`, named as `api.md` drafted it. *Reason:* the drafted JSON (`{"Field.ramp": ...}`)
  and the drafted example already read this way. *Cost:* a Param (a noise `phase`) in a field waits for V15b.
- **One path, not a drawing, in `Field.distance` and `Flow.contour`** (V15a; V15b took the drawing, below). A drawing would be read at its default values when the
  field is built, so a headline whose weight is a Param would be hatched along its default outline while the letters move.
  A path has no Params, so nothing is frozen by accident. V15b evaluates a drawing with the step's values.
- **`hatch` refuses what would be slow or stall, before it makes a line.** A lattice point looks at the cells within the
  widest spacing, at most the whole grid: cells times `min(ratio squared, cells)` above a billion visits (`MAX_VISITS`) is
  refused (1 to 1e6 over 400 by 400 took 20 s; a wide ratio over a small shape is cheap and allowed). A step the
  coordinates cannot hold (`reach + step == reach`, near 1e17) is refused, and lattice points are `left + i * lattice`, so
  no sum can stop moving. A path of two points closes into two opposite edges: it encloses nothing.
- **JSON names the path `drawing`.** `Field.distance` and `Flow.contour` take a `Path` in Python and Rust; their document
  argument stays `drawing` (one path in a one-element list) because V15b widens it to a drawing and the poster example
  already writes that form. The angle is `angle` in Python, Rust documents and JSON.
- **A mix is a tree, blended linearly in doubled-angle space.** `(cos 2a, sin 2a)` of a direction and of its opposite are the
  same vector, so `mix(contour(p), contour(p.reverse()), 0.5)` is `contour(p)` where a plain blend of the vectors cancels to
  nothing. The weighted sum is linear, so the tree evaluates in any grouping; the tree is kept (not flattened to weights) so that
  a document written again is the same text. Depth 16 and 64 leaves are enforced where `mix` builds it (rule 7); the stack
  test reads, writes and hatches the deepest. Exactly cancelling parts keep the first flow's direction.
- **A contour's direction is smooth along the polygon.** The nearest edge's tangent, turned half way into the neighbour's at
  each vertex: a piecewise-constant tangent made a line round a ring wander 4 pixels (measured: a loop of radius 100 spread
  95 to 99); with the blend it stays within 1.5.
- **Normalised distance has two definitions.** A part with holes: `d / (d + h)`, exactly 0 on the outer edge and 1 on a hole's
  edge. A part with none: `d` over its largest inscribed radius (probes on a 48 by 48 grid and three rounds of refinement,
  about 1 percent), so a stroke's middle line is 1 only if the stroke is as thick as the letter's thickest part. The drafted
  "centre line" would need a medial axis. *Cost:* the thin stem of a letter with a fat bowl peaks below 1.
- **Streamlines.** Jobard and Lefer: lines stop at half the spacing from another line's samples (the floor; a line is never
  promised a full spacing from its neighbour), seeds stand one spacing from a sample, and a lattice of the narrowest spacing
  finds a start for every part of the region. Steps are a quarter of the local spacing, midpoint rule. Under a tone, a seed is
  tested against the spacing of the line it comes from, so the gap between lines follows the tone line by line. All six
  constants are `// constant:` in `field/hatch.rs`. A line shorter than a spacing is dropped. *Cost:* near a boundary or where
  lines gather, short stubs and gaps up to two spacings remain.
- **Work is bounded before it starts.** The lattice and the sample grid together may have 4 million cells, and every sample
  is charged to `MAX_SEGMENTS`. The grid is a linked list through a `Vec` (no hashed collection, rule 4); lengths use `sqrt`,
  never `hypot` (rule 3). Same bytes on every target: the SVG of `flow_hatch` is compared in Python, Rust and Node.
- **Straight hatching is `hatch` with `Flow.angle`.** No second routine. Crosshatch and tone levels are stacked hatch steps (V15b).

## Fields and lines (V15b)

- **A field or flow keeps what it was told and what it made.** Its numbers stay expressions and its drawings stay
  drawings (`FieldKind`, `FlowKind`, what a document writes); beside them is the made value (the outline in a grid, the
  numbers worked out) at the Params' defaults, so `at` works anywhere. A `hatch` step makes it again with the values it
  draws with (`bind`). *Reason:* a value with a Param has no one number; the document and the cache need the unevaluated
  form, `at` needs the evaluated one.
- **A drawing in a field is taken by the step, like `avoid`.** `Given::drawings` lists a field's and a flow's drawings, so
  the graph, the cache, `reads` and `MAX_DEPTH` see them and the step is handed their shapes. *Rejected:* drawing it
  inside the step with `Drawing::shapes`: the drawing may share its cache with the drawing being drawn, and the cache's
  lock is not re-entered (a mutant that did so hung the test; killed). *Cost:* a field over a plain path is also taken
  and made again at every draw of the step that is not in the cache (measured: no visible time on the headline of
  `flow_hatch`; the outline is built once per distinct value of the Params it reads).
- **A key holds no drawing.** A field's or flow's key is its calls and numbers; its drawings are told apart by the node
  ids of the calls that take them. *Reason:* a drawing's own key holds the keys of the drawings its steps take, once for
  each way to reach them; nested through two fields a level, 15 levels needed gigabytes (the stack test did).
  `==` on a field or flow is the key plus `Drawing::is_same_as`: the same object, or each a lone path of equal data.
- **`Field.map` is bytes, an input, and sampled at pixel centres.** 8-bit, because that is what a photo is and what a
  document can name by `sha256`; vector links no raster library. Value `b/255` at the centre of its pixel over the box,
  bilinear between centres, clamped beyond (the nearest edge value: a photo has no edge to fall off). `to_map` samples the
  same centres, so `Field.map(w, h, f.to_map(w, h, box=b), box=b)` is `f` at the centres within half a level. A raw
  distance clamps almost everywhere to 0 or 1, so `to_map` takes the range to write (`low`, `high`) instead of choosing
  one. A map is capped at 2^24 pixels (16 MiB, 4096 by 4096); a larger photo is scaled down by the caller.
- **Contour lines are a value, not a step.** `Field.contours(levels, box, cell)` returns a `Path` (like `Path.arc`), by marching
  squares followed edge to edge rather than as loose segments (no hash map, a bit per edge, deterministic). A saddle
  joins its high corners when the mean of the four is high; a reading equal to the level counts as above it. Open where the
  line reaches the box, closed otherwise. Not a document verb: V14's `trace` is the verb for filled regions and will take a
  field; a saved drawing holds the path data, and a level that changes with a Param waits for it. *Cost:* the grid is
  sampled whole (4 million nodes at most, `MAX_SEGMENTS` for the lines).
- **Tone levels are `hatch(below=)` and passes are steps.** A level shrinks the region lines may enter to where the tone is
  at most it, so each pass places its own lines from its own seeds and passes cross; a stack of hatch steps is a
  crosshatch, and nothing else is new (no angle list, no pass type). The cut is where the tone passes the level within one
  step (a quarter spacing): a line is not clipped at the exact crossing. *Rejected:* one verb with a list of (level, angle)
  passes: it needs a list-of-pairs argument with its own limits and gives nothing a stack of steps does not.
- **The contour direction blends the edges about as near as the nearest.** Weights `(1 - (d - d0) / d0)^2` for `d` under
  `2 d0` (`BAND`, a constant with its reason), in doubled-angle space; along an edge `d0` is small and one edge counts
  alone, at the middle of a stroke both count and the direction turns instead of jumping (measured on a stroke whose edges
  differ by 21.8 degrees: a 23.7 degree jump between samples a quarter pixel apart, under 1 after). Where two edges meet
  at a right angle the doubled vectors are opposite and cancel on the bisector: a line field must have a defect at such a
  corner (its index is a half), so the lines of the two edges still end there. *Limit:* the blend does not remove it; a
  hatch drops a line that ends against another line and is shorter than three spacings (`SHORTEST` was one; straight hatching of an L, a star, a plus, a comb, a triangle and six letters at five spacings and four angles: 4 of 240 runs differ, all in letters, each by about one line of under three spacings), which
  removes the ticks by construction; a chord (a line that leaves the region at both ends) keeps the old one-spacing floor,
  so a coarse hatch of a small shape still draws. Measured on the headline of `flow_hatch`: at spacing 3 to 9 a fragment 8.5
  long ended 17 pixels inside a letter before, and every line under three spacings now is a chord within a step of the
  outline.
- **`to_map` and `contours` charge their reads before the first one** (`MAX_READ_WORK`, 2e8 units). A read of a ramp, a
  constant or a map is one unit, of a distance field `1 + sqrt(edges)` (three times that normalised): a nearest-edge search looks
  at about that many edges. *Measured:* 2 microseconds a read on a headline (about 21 units), 26 on a path of 200,000
  edges (448), so 1 million reads of that took 26 s and 4 million nodes 100 s; the limit is some ten to twenty seconds.
  `Field.contours` has no default `box`: only a map has an extent, and a field made by a distance or ramp reads everywhere.
  A grid's cell count is `extent / cell` rounded to the nearest whole number when within 1e-9 of it, so the documented recipe
  for a map's pixel centres stays on them.
- **A stack test for the new nesting.** Drawings nested 15 levels through the field and the flow of a hatch step are
  drawn, saved, loaded and dropped on the 256 KB stack (passes at 112 KB).
- **Limit: halftone's tone was still a number.** Lifted by 4a (below): `halftone(tone=)` takes a field.

## Line families, plotter output and tidy contours (V15c)

- **Three verbs, not one `lines` with a family.** `rows` (displaced rows), `bands` (width carries tone) and `stipple` (dots)
  have different outputs (open lines, closed outlines, closed dots) and different arguments; one verb would have been a
  union of unrelated options. The carriers of `bands` are one verb because they differ only in where the centre lines run.
  *Rejected:* a `family` for rows and dots, and a rotation for rows (turn the region instead).
- **A tone is read where the centre line is; a width is `(at tone 0, at tone 1)`.** As `hatch`'s spacing pair, so the call
  reads the same in every family; the width is the documented mapping and the oracle reads it off the polygon's vertices.
- **Hidden lines by a horizon, conservative between points.** Rows are made from the nearest (lowest) up; a horizon holds the
  lowest height seen at each point of one shared lattice, and a point is seen when it is above it. Between lattice points the
  horizon is the interpolation of its two values, which lies at or below the true envelope of the nearer rows, so a drawn
  point is never behind a nearer row (the test rebuilds every row and checks); the cost is that a row can end a fraction of a
  step early where two rows cross between points, and at a region's edge that falls between points. *Reason for the lattice:*
  one pass, no segment intersection, and spans from `Region::spans_at` give the region's edge exactly.
- **Rows are a window on the region, and read the tone over their footprint** (2026-11-17, from the owner's look at the
  dog). A lifted row is cut to the region, and only its part inside raises the horizon, so a peak lifted out of the outline
  neither crosses it nor hides the rows behind (clipping the finished rows instead left empty patches between the ears).
  The tone is read as a 3 by 3 tent over half a step along and half a spacing across: a photo's fur and grass varied faster
  than rows a spacing apart can draw and chopped them into short pieces; a blur upstream of half a spacing was measured to
  clean them and keep the face, and the tent does the same with no setting. *Rejected:* a `smooth` argument (a decision the
  rows can make from their own spacing), and rows that follow the row above (softer detail, a different look).
- **Stipple is a threshold, not blue noise.** An R2 sequence over the lattice picks the cells that get a dot, so the count in
  any slab follows the density to a cell or two with no relaxation and no state; the jitter inside a cell breaks the grid.
  *Rejected:* dart throwing with a minimum distance (state, order-dependent, cost), and Poisson disc.
- **`within` cuts exactly, from the outline's own edges.** Sampling the vertices missed a two-point line across a circle;
  every edge is cut where it meets an edge of the region and each part kept by its middle, never by its start (a point on the
  outline is inside or out by chance). It is the one way to keep contours inside a subject, so a `contours` of a matte gives
  the silhouette and `within` the lines of the picture. Not a verb yet: a document would need a region argument.
- **Contour lines run with the high side on the left.** Marching squares followed an edge in whichever direction it met it
  first, so the loops of a matte did not fill as the subject (a hole filled in). Orienting each line by the corner that is high
  at its first edge makes a `contours(0.5)` of a matte a region the nonzero rule reads; a closed loop inside a hole winds the
  other way. *Reason for `smooth` on the grid, not the lines:* a line from a smooth field is smooth at every level and its tiny
  loops disappear before they are traced; smoothing the lines after would keep the loops and the bunching. `smooth` is a
  Gaussian in design pixels (refused past 2 billion steps), `min_length` drops short lines. The weights per level, the levels
  and the palette are the poster's, not the library's.
- **Every allocation is bounded by its own count, before it is made.** A wide thin region made `rows` build its lattice of
  columns with no row to put on it (4.6 GB at 1e9 wide), and a far `centre` made `bands` take a difference of squares
  that cancelled and a ring count that saturated (`capacity overflow`). Each count is now worked out in f64 per kind of
  line (rings by their radii, an arm by its arc length) and refused first, naming `centre` when a centre away from the
  region is what stretches the carrier. *Reason:* a refusal that depends on a sum made after the allocation is no bound.
- **A region that fills nothing gives an empty path in every verb** (a point, a lone straight line: `Path::may_fill`).
  `rows` had returned a sliver for a line, `stipple` an error for a point and `spiral` and `rays` the error of a
  one-point path. *Reason:* the verbs share a region, so they share what an empty one does.
- **`within` asks whichever index is sparser, and charges what it weighs.** The edges are in y bands and x strips; the cut
  of a segment and the middle test of each part use the one with fewer edges, so a comb that is one band tall costs what a
  round outline costs. What is left (long lines across very many edges, output-sized work) is charged before each
  question against half a billion edge visits (measured 1.4e8 a second: a 100,000-edge comb under a line of 100,000 points
  took 73 s, now under a second) and refused naming `region`, or `spacing` inside `bands`. A region that cannot be
  flattened inside the segment budget is refused, never taken for an empty one.
- **A family refuses what it does not read.** `bands` is one verb over five carriers, so a `centre` on lines or an `angle`
  on rings would otherwise be accepted and ignored, which hides a misspelt family. `Bands::refuse_unread` is the one place
  the rule lives (the builder, the document and the bindings call it); `angle` is therefore unset, not 0, by default.
  *Rejected:* a verb per family (the summary says what they share); `width` stays one pair from 0 (a line may fade to
  nothing: the step, the schema and the path agree).
- **Plotter output is a writer, not a step.** `to_plot_svg` is `to_svg` for a pen: strokes only, a layer per pen (a stroke or
  fill colour), written in Inkscape's layer convention. The pen width is the plot's (`Plot`), a fill is `hatch` at that width, and
  what has no pen form (gradient, halftone, shadow, clip, mask, knockout, blend) is refused naming the shape, as the mesh
  refuses what it cannot draw. Order is nearest neighbour with flips and, for a closed loop, the nearest vertex; a grid of the
  ends finds it (ties go to the earlier stroke, so the order is deterministic). Measured on twelve seeds of seven strokes, at
  most 1.41 times the best order; 50,783 lines of the poster order in 2.4 s. A crowd the grid cannot spread (100,000
  strokes in a 10 by 10 corner of a big page: 9.8 s, 300,000: 38.7 s) is quadratic, so the entries scanned are counted and
  past 2e8 the strokes left keep their drawing order. *Rejected:* an optimal tour (NP-hard; two-opt improves a little and
  costs time); a document verb (the plot is an output); a k-d tree for crowds (a later batch if one matters).
  *The sides carry a unit:* `Plot::unit` (`px`, `mm`, `in`) writes `width="210mm"` and a `viewBox` of the same numbers, so
  plotter software reads the physical size; outside pixels a side need not be whole. The pen stays in design pixels (it
  scales with the frame), and a shape the picture does not show (opacity 0, a fully transparent fill or stroke) is not
  plotted. In Python the options are keywords (so no Param may be named `pen_width`, `tolerance`, `unit`, `into` or
  `window`; `pen_mm` is free again), in JavaScript an object after the values, as `toRgba` takes its window.
- **Douglas and Peucker without recursion, with a budget.** The worst case is quadratic (a spiral), so the tests are counted
  and a call over a billion is refused naming `tolerance`; the stack stays flat (rule 7).
- **A name tells fonts apart, as it does maps.** Two fonts with equal bytes and different names shared a key, so a drawing
  using both saved with one name for both; the name is now in the key and `to_json` refuses the pair, as for maps.
- **A window of a pixel render once differed in one pixel by one level** for `bands` at spacing 7 and 20 degrees over the
  differential's map (one pixel, one channel; rasteriser rounding where outlines of nearby bands meet). Not a visible
  difference (rule 2): the corpus uses spacing 8, which is exact; recorded here, nothing added.
- **Left for later.** A document verb for contours and `within` (V14's `trace` takes a field), rays whose width follows the
  radius, a field in `halftone`'s `tone`, two-opt for the plotter order, hatching that avoids the strokes of other pens.

## Trace (V14a)

A map of plain numbers (a `Field.map`, V15b) becomes filled shapes: `trace_levels`, `trace_labels`, `trace_cells`, each a
source verb with the map as its `map` argument, so a document holds the call and names the map as an input and never embeds
the traced paths.

- **Sources, not methods and not eager.** A trace makes many shapes with an attribute each (`level`), so it is a `Drawing`
  source (`Drawing::trace_*`), not a `Path` method. It is a verb, not a function that returns shapes, because a saved
  drawing would otherwise hold every traced point. *Rejected:* one `trace(mode=...)` verb (the options differ: a cell trace
  has no fitting, a level trace no palette), a new `Kind` for a map (`Field.map` already carries bytes, a name, a digest and
  a box through documents and the schema).
- **Levels are filled regions, stacked.** Level L is where the map is at or above L, so the shapes nest and the lowest goes
  first; a level the map never reaches gives no shape (an empty shape would only be an element to skip). The level is the
  `level` attribute (already in the attribute list); a label or cell shape carries its colour byte as the new whole-number
  attribute `label` (a byte is an index, not a tone: one name for both would read as a tone). No palette argument: the shapes
  come unpainted and `only(label=..).fill(..)` paints them, because a list of colours would be a new argument kind in the
  reader, writer and schema for what a loop already says. *Cost:* a palette is a few lines in the caller.
- **One lattice, one tracer.** Levels and labels run the contour tracer of `Field::contours` over the map's pixel centres
  (a label is the indicator of its byte at 0.5), so saddles, orientation and the edge walk are written once.
- **Edge padding.** The lattice gets rings of the edge value and an outermost ring far below every level, so a shape that
  meets the border goes on past it and closes outside the frame. No edge lies along the border, which a stroke would draw
  as a false closed edge and a morphological opening would eat into. The padding is two pixels plus `max(open, close) +
  tolerance` (in map pixels, rounded up), so shrinking cannot reach the frame. *Cost:* shapes reach past the frame; a view
  crops them, and for a plotter or a cutter one `Drawing.intersect` with the frame rectangle cuts them (labels still tile).
  A padded lattice is at most 2^25 nodes (8 bytes in the grid and 8 in the readings: half a gigabyte), which holds the
  largest map a `Field.map` can be (4096 by 4096) with hundreds of pixels of padding. Measured: three levels of 3000 by
  3000 with smooth 2, open 2 and close 2 in 3.0 s and 646 MB, of 4096 by 4096 in 6.6 s and 1.4 GB; a dither of 4096 by
  4096 cells of two pixels (4.9 million points) in 2.1 s and 1.0 GB; the padding that fills the cap (28.9 million
  nodes, open 15 on a 100-pixel frame) in 6.4 s and 855 MB. The old bound was a node count borrowed from `contours`
  (4 million) and refused a 2000 by 2000 map. When the padding is what is too wide the refusal names the one of `open`,
  `close` and `tolerance` that is largest.
- **Tidy order.** Blur (on the map), trace, close, open, fill small holes, drop small parts, fit curves. Open and close
  are offsets with round joins (a ball rolled along the edge) at 0.01 pixel; the area limits subtract or unite the
  small rings from the canonical result, so a dropped part takes its holes with it. Curve fitting is `Path::simplify` (corners
  past 45 degrees stay), so every fitted point is within `tolerance` of the traced polyline. `smooth` is the standard
  deviation of the blur, in design pixels (the map's pixels are scaled by its box).
- **Exact cells.** `trace_cells` follows the cracks between cells of different colour, clockwise round each colour, turning
  towards the colour first (two cells that meet at a corner are two regions) and keeping only the corners where the
  direction changes. The result is rectilinear, its area per colour is the cells' count times the cell area, the colours
  tile the map with no gap or overlap, and holes are sub-paths running the other way (nonzero fill). The cells must be uniform
  (every pixel of a cell one byte) and divide the map: a map that is not what the caller said is refused naming `cell`.
  *Rejected:* fitting curves (it rounds the cells) and one polygon per cell (a dither of a photo is millions).
- **Labels tile.** Each label is traced and tidied on its own, and fitted apart neighbours leave gaps and overlaps (measured
  on 64 by 64 maps: smooth 2 left 40 square pixels of gap and 23 of overlap, even untidied three labels met with gaps of
  a pixel or less); for riso and cut paper an overlap is two inks on one spot. So the shapes are made to tile, by
  construction: the order is the **commonest label first** (the lowest byte of a tie, so it is the paper), then the others
  in order of the byte; the commonest is not traced, it is the padded rectangle without all the others, and each other
  label is its own traced shape without the labels after it. Any list of sets cut this way, the first being the whole
  frame, is a disjoint cover. Each traced shape is resolved first (fitted apart, a hole can poke out of its outline by the
  tolerance and would count against its neighbours), the cuts are `Path` booleans at 0.01 pixels, so a tidied label is a
  polyline within 0.01 pixels of its curve; a label the others swallow is left out. Heavy `smooth` loses what the blur
  makes minor to the commonest label. *Rejected:* an argmax of blurred indicators (it tiles too, but a label would
  then not be what `open` and `close` shaped) and a hairline workaround in the caller.
- **Limits, charged before allocation.** Passes over the lattice (levels, labels but the commonest) times its nodes at
  most `MAX_READ_WORK` (2e8), at most 256 levels, `max_colors` at most 256 (default 32), the cracks of a cell trace, every
  traced point and every edge, edge-box pair and vertex of the label cuts against `MAX_SEGMENTS`, the blur against its
  work bound. Each refusal names the argument: `levels`, `max_colors` (also when the cuts pass the budget), `smooth`,
  `cell`, and for a padded lattice past 2^25 nodes the largest of `open`, `close` and `tolerance`. `max_colors`, `cell`
  are whole numbers in the schema too (`Domain::WholeBetween`); `levels` takes numbers, Params or expressions in Rust,
  Python and documents (read once, as `dash`'s `pattern`); `max_colors` is a plain argument of the two verbs, not a field of
  `Trace`, which holds only the tidying.
- **Known limits.** Tracing a binary indicator cuts the corner of every one-pixel step by an eighth of a pixel
  (curve fitting hides it; the labels' cuts keep a corner's gap to the commonest label); the raw trace of a map winds the other way round than the canonical form (fill is nonzero, so
  it does not matter); sequences of maps are V14b, below.

## Trace sequences (V14b)

A video traced frame by frame flickers: shapes move with the grain and holes pop in and out. One helper on `Field` makes a
caller's list of maps steady before each is traced.

- **Maps in, maps out, on the whole list.** `Field.smooth_maps(maps, *, sigma)` takes the list and gives a list of
  `Field.map`s. *Reason:* the result is what every V14a verb and every document already reads, so there is no new verb,
  `Kind`, schema entry or WebAssembly call, and the library keeps no sequence. A per-frame call (`at=i`) was rejected: the
  caller would build the whole list anyway. *Cost:* Rust and Python only; JavaScript loads documents and does not build
  fields.
- **Smoothing is a truncated Gaussian over the maps** (`ceil(3 sigma)` either way, weights renormalised where the sequence
  ends, bytes rounded): the oracle is a hand-computed weighted mean, and a static sequence is exactly itself. A `sigma` so
  small that its taps reach no neighbour (`exp(-1/(2 sigma^2))` is 0) gives the maps back, never a NaN weight.
- **Limits, charged before work:** the work is counted per output frame (pixels times the frames its window reaches, at most
  2e8), so the cost of a clip grows with its frame count and any clip whose maps the caller already holds runs in one call;
  the memory bound is the output the caller asked for, at most 2^28 pixels over the maps. Refusals name `maps` or `sigma`.
- **Rejected: hole persistence** (`Field.persist_holes(maps, levels, reach, at_least)`: a hole survives only if
  neighbouring maps have it, else it is raised to the tone round it). It was built, then measured on the demo's own nine
  frames: smoothing alone left 148,999 changed pixels between consecutive traced frames, smoothing then persistence 156,845
  (5% worse). It also made notches that touch the map's edge (which are not holes) flicker more, and it made the caller pass
  the trace's levels twice. Smoothing in time already removes most of what it was for.
- **Measured (the cut-paper demo, nine frames of the owner's dog drifting and zooming, grain of 36 grey levels):** see the demos
  folder's `30-vector-m4-cut-paper` README; smoothing in time also smooths the motion, so the flicker of steadied frames can fall
  below that of the clean frames traced alone. *Known limits:* the ends average fewer frames; a hole or speck that lasts
  across frames stays (`min_area` and `min_hole` drop small ones); a fast motion lags.

## Cut-paper piece (M4)

M4 is the demos folder's poster of the owner's dog as cut paper (like M5, it uses his photo, so it is not in `examples/`;
`examples/trace.py` and `examples/trace_sequence.py` are the public parts of the recipe). Each tone level is a sheet cut to the
dog's own traced silhouette; the lowest sheet is the whole silhouette, so a tone darker than the first level is never a hole;
the darkest tones (the map inverted, traced at a high level, finer tidying) are an accent sheet glued on top so eyes and nose read,
with small catchlights; every sheet has a `shadow` (V10), the sun, hills and headline are paper too. Reasons: a level trace of a
photo has a dark fur that fills the lowest sheets, so the face read only once the darkest sheet was separate. Limit: the
eyes' catchlights are placed by hand (a hint of where the eyes are), the glue between image and vector is the demo's own
`dogmaps.py`.

## Presets (V17)

- **A preset is a whole drawing, as a JSON document.** It mirrors `Font::bundled`: a sorted table of names and texts in
  `preset.rs`, texts included with `include_str!`, loaded by `from_json` with no inputs, so a preset is not a second
  format or loader. The files are `to_json` output, generated from the example programs, never edited by hand. A preset
  has no description key (the format has none).
- **The word differs from the image library's.** There a preset is a set of parameters for one effect; here it is a whole
  drawing with its own Params, because a drawing's look is its shapes, not one setting.

## Preset descriptions (V19)

- **A description lives in the `PRESETS` table, not in the document.** The table already owns the names, so a name and its
  description cannot drift apart, and the format stays a drawing with no field that only presets use (a user's document
  has no use for one). One helper owns the lookup and the refusal, so `preset` and `preset_description` cannot differ.
- **Presets come from programs.** `examples/presets/<name>.py` leaves a `drawing`; `tools/write_presets.py` writes the JSON;
  a test compares each file with its program's `to_json()` (the Python builder, an independent route to the same text).
  `ring` is one file, preset and example both (`examples/presets/ring.py` also teaches the eager `Path` against the
  `Drawing` steps). `gradient_type` and `bent_type` are tuned variants of their examples (other centring and defaults,
  for a headline that stands alone): the preset is the shipped look, the example in `examples/` the documented teaching
  program, and docs and tests refer to the example by name.

## Browser playground (V20)

- **A static page, no framework.** `playground/` is HTML, one module for the page and one for the slider maths: nothing to
  install, nothing to build but the WebAssembly glue, readable end to end. A page that loads a `.wasm` needs http, so
  `playground-serve` is `python -m http.server`, bound to 127.0.0.1. *Alternative:* a framework and a bundler: more to keep working than the page does.
- **The slider maths is its own module.** `params.mjs` (a Param to a slider, a source reading to a value, the sources) has no
  DOM, so Node tests it on every preset of the real package inside `wasm-test`; the page only wires it to elements.
  Each source in its table names what wakes it (a window event or the animation loop): a new source is one entry.
- **The Chrome check is a named task, not part of `check`.** It needs a browser and a built page, and a gate that needs one fails
  on machines and CI images without it. `playground-check` is run by hand before a change to the page.
- **`textContent` only.** Preset names, descriptions, Param meanings and a document's error messages come from the library or
  from a pasted file: the page never builds markup from them.
- **Left out:** a render worker (a move renders on the page's thread), a GPU mesh display, a hosted site, inputs for a document
  from the page (a document that needs a font or a map is refused, naming it), Safari, Firefox and phones (the check runs Chrome
  alone), and `wasm-opt` (the glue is as `wasm-bindgen` writes it).

## Command line (V18)

- **`bt-vector` is Python, in the package (D4).** The CLI is argument parsing, file IO and one call into the core; `rules.md`
  already assumes a Python CLI, and control and audio do the same. A Rust crate (clap, a binding, a test crate) is not
  worth it for that. A Python-free `cargo install` command waits on someone who needs the CLI without Python.
- **Skipped for now:** plotter output (`to_plot_svg` needs a pen width), `--json` for `describe`, a `schema` command
  (`drawing_schema()` exists), render windows. Each is a flag or command later.
- **One duplicate:** `tools/look.py` keeps its own `WxH` and `NAME=VALUE` parsers; the dev tool cannot import the CLI's
  private helpers.

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
- **Optimised Rust tests, the stack tests not** (2026-11-08). `cargo test` builds with `opt-level = 1` (`[profile.test]`),
  workspace code included. Measured warm at load 7 to 12: `tests/it` 437 s of CPU unoptimised, 59 s optimised;
  `drawing_limits` 70 s of CPU and 64 s wall, 13 s; `poe rust-test` about 120 s wall, 30 s. *Reason:* the time is in this
  workspace's own code (the million-shape tests, the proptest oracles, the glyph corpus); optimising dependencies alone
  (`[profile.dev.package."*"]` at 3) took `it` from 437 s to 367 s of CPU and `drawing_limits` not at all. Debug assertions
  and overflow checks stay on. *Consequence:* optimised frames are smaller, so the stack tests (`crates/core/tests/stack.rs`:
  the 256 KB bound of "Bounded depth") would prove less: the glyph test needs 146 KB unoptimised and 18 KB at `opt-level = 1`
  of the 256 KB. They are a binary of their own that `poe rust-test` runs a second time with `--profile stack` (`inherits =
  "dev"`: unoptimised, the worst case), and a 120 KB local added under the glyph reader fails there and passes optimised. The
  recursion tests (a million-level expression, a hundred-thousand-step chain) fail on a recursive walk at either level.
- **Bounded depth, not a bigger stack** (2026-11-01, limits measured 2026-11-16). Whatever a caller can build has its
  depth limited where it is built, so no walk recurses over unbounded depth: drawings nested past `MAX_DEPTH` (16) are
  refused when built, an expression past 16 levels becomes a node without operands (operators never fail, so a loop can
  build one; the call that uses it is refused), and the parser refuses a document nested past `MAX_DOCUMENT_DEPTH` (40)
  before the loader recurses. The smallest supported thread stack is **256 KB** (a Python thread gets 512 KB on macOS),
  and the promise holds for every build: the stack tests run unoptimised (`[profile.stack]`), whose frames are the
  largest. In WebAssembly the engine throws a catchable `RangeError` below about 0.25 MB. *Reason:* an overflow is a
  segfault in the caller's process. A growing stack (`stacker`) was rejected: it cannot grow the WebAssembly engine's
  stack, loses CPython's overflow guard on 3.14 and later, adds `unsafe` and a C compiler to the sdist build, and trades a
  bounded stack for unbounded memory.
  *Measured* (macOS arm64, `stack.rs` on a thread of N KB, N in steps of 16 up to 256; the smallest N that passes):
  at the first limit of 128 the nested rows needed 1 MB (768 KB failed), 128 nested drawings over a 128-level
  expression 2 MB, and a document of 128 JSON levels (30 nested stacks) 512 KB (448 KB failed), so the promise held in
  release builds only (52 KB and 96 to 128 KB there). At `MAX_DEPTH` 16 and `MAX_DOCUMENT_DEPTH` 40: drawings nested
  through stacks, booleans or clips 128 KB, through masks 112 KB, over the deepest expression 160 KB, over the deepest
  glyph (the glyph alone needs 128 KB) 176 KB, the deepest document 144 KB and over the deepest expression 160 KB; the
  worst case is 176 KB of 256, a margin of 30 per cent for another platform's frames. A depth of 20 left 224 KB (13 per
  cent), so 16. The frames cost about 6 KB a level of nesting in drawing evaluation (`drawing/eval.rs`), 4 KB a level of
  a document (`json/parse.rs`, `read.rs`, `checks.rs`) and more in an expression's value.
  *Rejected:* walks over nested drawings made iterative (an explicit stack in `eval.rs` and `graph.rs`) while keeping 128:
  the loader's three modules and the expression walk overflow too, well past one module of work. *Consequence:*
  `tests/test_stack_depth.py` and `crates/core/tests/stack.rs` run the deepest structures on the minimum stack; a new
  node or reader adds its structure to them, and a limit raised or a frame grown is measured again in the same way (a
  `KB` sweep of those rows). A document holds about ten nested drawings, an example such as `hatched_poster` fits.

The template's decisions (the gate tiers, the pass record, the memory cap, the test layout, the platforms) are in
[template-decisions.md](template-decisions.md).

## Open

Undecided questions and what they wait for are in [design.md, "Open decisions"](design.md#open-decisions), among them
stable placement over changing values (D7), which the prototypes raised. A question moves here, with its reason, once
it is decided.

## Wow gaps (2026-11-13)

A review of the docs against the code found two missing capabilities. The code has no deformer, morph, trace, dash modifier or
streamline (only `along`, the text-on-a-path bend); `docs/examples/` holds the reference pieces for them.

- **Transforming type (V12 and V13) is the headline.** Letters that bend, ripple, twist and turn into other shapes or words are what
  makes kinetic type look finished, and the first public demo needs them. *Reason:* the live page and M3 have nothing
  striking to show without them. So the order is V11 (resample), V12, V13, M3, then V15 before V14.
- **Depth and light stay out of vector.** Lit, chrome and glass type exists in image (`relight`, `liquid_metal`, planned `glass`)
  and is joined by scene (C5, an effect on a drawing layer). *Reason:* vector never links a raster library. Vector's part is the
  distance field of a shape as a grayscale map (V15). Settled with the scene agent: the first version needs no map (`relight` with
  `field="light"` on a light fill); the distance field is a later refinement.

## Halftone screens (4a)

- **One owner of the rows.** `bands` and the halftone screens both lay things along straight rows, waves, rings, a spiral or
  rays, so the maths that says where a row is, which rows cover a box, how many cells a ring has and how a turn is read
  lives once, in `lines/layout.rs` (`Layout`, `Family`, `Distances`, `RoundRows`, `ring_cells`, `ray_wedges`). The move was
  done first, as a structure-only step, against a pinned SVG hash of every family at a spread of settings
  (`tests/it/lines_bands_pinned.rs`): not one byte of `bands` changed. A screen is therefore the same layout a `bands` of the
  same family draws its lines along, by construction, not by a test that compares two copies of the formulas.
  *Alternative:* copy the five formulas into the screen; refused, because the two would drift.
- **Rows are walked by index, and their work is bounded without subtraction.** A screen first subtracted two squares to bound
  the cells of its rings; for a centre 1e20 away the squares cancelled, the bound passed the work charge, and the loop
  `k += 1.0` never advanced. `Layout::round_rows` is now the one function that says which rows reach a box: it refuses a
  centre so far that a step would not move there (as `bands` always did), bounds the work as rows times the cells of the
  outermost row (a product of non-negative numbers, no difference), and hands back the row indices, which are walked as
  integers. A bound that cannot cancel and a loop that cannot stall leave no input that hangs, so no timeout or clamp.
- **A cell has an area, and a dot covers `1 - tone` of it.** The invariant in the brief is stated per cell, so the engine is
  built from it: a screen yields cells (centre, direction of the row, length along it, width across), the mark gives the
  polygon of area `1 - tone` of a unit cell, and the dot is that polygon stretched by the cell's two sides, turned to the
  row and put on the centre. An affine map multiplies every area by the cell's, so the invariant holds for every mark and
  every screen with no per-case code. The cell's area is exact: a wedge of a ring is the mean radius times its angle times
  the pitch (an annular sector), the cell of a wave is the grid's cell moved, so the maths of the areas is two lines each.
- **Exact area by solving the corners, not by the circle.** A round dot used to be the polygon inscribed in the circle of the
  right area, a percent short at the flatness of 0.01 pixels. The corners are now solved for the polygon's own shoelace area
  (`r^2 = 2 A / (n sin(2 pi / n))`), so a dot's area is `(1 - tone)` of its cell to rounding, and the oracle in the tests is the
  shoelace formula itself. The euclidean mark has no closed form (the level sets of `(cos 2 pi x + cos 2 pi y) / 2` are not
  elementary), so a table of 129 outlines at equal steps of cover is built once for a call, two neighbours blended for a dot,
  and the blend scaled to the exact area: the table only has to be close, the scale makes it right.
- **Which marks stay in their cell.** The square, the diamond and the euclidean mark are confined to the cell (past half the
  cover the diamond and the euclidean mark become the cell less its corners, as a print screen does, and tone 0 is the whole
  cell), so they tile; the dot, the ellipse and a caller's path are not (they overlap at low tone, as the round dot always
  did, documented). *Alternative:* clip the round ones to the cell too; refused, because the overlap is what the earlier
  halftone did and changing it would change every existing picture.
- **A caller's outline is one closed sub-path, centred on its centroid.** The centre of its bounds would put a heart or a
  triangle's mass off the lattice point; the centroid keeps the mass on it. Several sub-paths would need a hole's winding to
  survive the nonzero fill of separate dots, and the mesh draws each dot as a polygon of its own, so one outline is the rule
  and a second is refused naming `shape`. Its winding is made clockwise whichever way it was drawn.
- **`tone` is a number or a field, as one argument.** A number stays a number per shape (a Param, `by_index`): `Tone::Level`;
  a field is read at each dot's centre and held to 0 to 1 (`Tone::Field`). In a document the argument is the `Field`
  kind that reads one number per shape: a number or an expression is the per-shape number, an object is a field. The style
  keeps what it was given, so `Style.halftone.tone` is a float or a `Field`; Rust's `Halftone::new` takes either, as Python's
  `tone=` does. Two fields compare by their keys; a distance field's key holds the digest of the outline it was bound to,
  worked out once when it is bound, so the cache cannot take two different distance fields for one and a style that is
  put back into a drawing (which binds the field again) equals itself.
- **`shape` is a word or a path, in a document too.** One argument in every language: Python and Rust take the `Path` where
  the word goes, a document writes `"shape": "ellipse"` or `"shape": {"path": "M0 0 ..."}` (a word or an object, told apart
  by JSON type, so the schema is an `anyOf`). An earlier document had `"shape": "path"` plus `"mark"`, an argument Python does
  not have, and its errors named `mark`; they name `shape` now.
- **One table of what a layout reads.** `Family::reads` (in `lines/layout.rs`) says which of `angle`, `centre`, `wavelength`
  and `amplitude` a layout reads; `bands` asks it by family, a halftone by `Screen::family`, and both refuse an argument given
  and unread through `Family::refuse_unread`. A mark's one rule, that only an ellipse reads `aspect` (a caller's path does not:
  it has its own shape), is `Mark::refuse_unread`. Left out, an argument takes its default and is not refused, so the optional
  arguments of `halftone` have no declared default (the summary says it), as `bands`' have none. A drawn style put back into
  a drawing (`From<&HalftoneStyle>`) gives only what is read, by the same table.
- **Rays are image's: a ring's worth of cells half way to the nearest side.** The count is `ring_cells` of the radius half
  way to the nearest side of the shape's box (`2 * round(pi r)` for `r` in pitches), not the farthest corner's; the rays merge
  in pairs, a power of two at a time, until none is narrower than half a pitch (image's `NARROWEST_RAY`), so a row has
  `rays / step` cells, fractional when `rays` is not a multiple of `step`: the last is narrower and fills the rest of the
  turn, its dot covering `1 - tone` of its own smaller cell. The test counts the dots of each row against a hand
  computation (4, 16, 31, 31, 31, 62 ... for a square of 100 at a pitch of 5). `bands` keeps its own count of rays, one
  every spacing at the farthest corner, since its lines are drawn out to it.
- **A dot is not tilted on a wave.** A dot sits on the displaced row but keeps the direction of the layout, as an ellipse
  or a diamond is easy to read against the grid; tilting it by the row's slope would turn a round-looking ellipse of aspect 1
  into a mark that changes along the wave. *Consequence:* on a steep wave a dot's cell is a sheared one and the dot does not
  fill it.
- **`line` and `matrix` are left out.** `bands` draws lines along these layouts, with the width carrying the tone; a caller's
  `Path` is the matrix. Image has both because it has no path.
- **Work.** The cells near the shape are counted before any is made: an upper bound per layout (a rectangle of the lattice,
  the rows times the cells of the outermost, `Layout::round_rows`), each times the outline's edges plus a dot's corners, charged to
  `MAX_SEGMENTS`, so a tiny pitch, a far centre or an `aspect` of 1e100 is refused naming `pitch` before an allocation. A
  dot's reach is worked out from the mark, never from a polygon made for the purpose: the first version made one to measure,
  and a huge `aspect` allocated 4 GB before the charge ran (found by the adversarial test, which now stays).
- **Plotter output.** `to_plot_svg` draws a halftone as its dots, hatched at the pen width like any solid: the region is the
  dots wholly inside and the crossing ones cut by one boolean, as the mesh makes it, from the one screen answer. It refused a
  halftone before.
- **The work charge sizes a cell per screen.** A grid and a wave have cells of one pitch; only the wedge screens are charged for
  cells up to two pitches.

## Character grids and overlay marks (4b)

- **A character grid is outlined glyphs, not SVG `<text>`** (owner's decision, 2026-11-18). `characters` makes one shape
  for each character with the layout `text` uses, so SVG, pixels, the mesh and the plotter show the same shapes. `<text>`
  would be a second renderer (the viewer's font engine, its fallback and shaping) that the pixels, the mesh and the plotter
  could not share. The text stays as data: every shape carries `char`, `row` and `column`, so `only(char="#")`, a colour
  for each cell and a search work as they would on text. *Cost:* a grid is as heavy as its glyph outlines (a 36 by 18 grid
  is about 0.5 MB of SVG); `text` is the same.
- **One text path.** A character is set by `typeset`, once for each distinct character of the grid, alone: its `origin`
  attribute is the middle of its advance and of a capital, so a cell's shape is that shape moved by the cell's middle minus
  its `origin`. No second shaping or outline code, and no ligature crosses a cell (JetBrains Mono ligates `->`, `==` and
  `!=`, and a grid of ASCII art is full of them): a test sets `->==!=<=` and counts eight cells. A character the font
  lacks is its notdef glyph, as in `text`; a space makes no shape (no outline). A control character is refused naming its
  row and column (a tab has no width in a grid).
- **Advance and capital, not ink, are centred** (deviation from "bounds centred in the cell"). Centring each glyph's ink
  would put a full stop, a hyphen and an underscore in the middle of their cell and read as a different picture; centring
  the pen box keeps the row's baseline, which is what a grid of characters is. Symmetric capitals (`H`, `O`, `#`) are
  centred by ink as well, which the tests check against the font's own numbers.
- **The default size fits the line box.** The largest size at which the widest advance of the grid is within the cell across
  and the font's line box (the `hhea` ascender above the baseline and descender below it, about the middle of a capital) is
  within the cell down: descenders never reach the next row. For JetBrains Mono (advance 0.6 em, line box 1.32 em) a cell of 6
  by 12 gets 9 pixels, the height deciding; the caller's `size` overrides it. A font with no line box takes 0.8 and 0.2 em
  (`LINE_ABOVE`, `LINE_BELOW`).
- **`colors` are rows of colours, as image's Python and JSON data have them.** Image's `characters(...).colors` in Python is a
  list of colour texts for each line (spaces included), so it plugs in as it is in Python and JSON (`Kind::Colors`, an array of
  arrays). Image's Rust `Characters::colors()` is one flat `&[[f32; 3]]`, row after row: in Rust the caller cuts it with
  `chunks(columns)` and converts each triple to a `Color` (vector does not link image; the doc of `Characters::colors` shows it).
  A mismatch is refused naming the row and the counts. *Alternative:* a flat list, which only Python could have fed from image without a
  step, and which could not name a row.
- **One meaning per name across vector and image** (2026-11-18). `characters(content, ...)` names its text as `text` does
  (image's `Characters.rows` is a count of lines and its text is `.text`). A mark's text is `text` (record field, `role`
  value and `text_size`), because `label` is already an integer attribute that `trace_labels` and `trace_cells` write
  (`only(label=3)`). The spacing of cells is `cell` in `characters`, `trace_cells` and image's `ascii`. *Deliberate
  exception:* `halftone` keeps `pitch` (its own history, and image's halftone calls it `pitch` too); it is the distance of
  dots on a lattice that can be turned, not the size of a cell of text.
- **The font is optional everywhere** (Rust `.font(&font)`, Python `font=None`, JSON no `font`): a left-out font is the
  bundled JetBrains Mono, so marks with no text need none, and a document that uses the default lists no font input.
- **Records for marks, one mechanism.** `boxes` and `points` are lists of records read like the arguments of a call from a
  table that is never registered (`BOX`, `POINT`), as the `runs` of a `text` are (`RUN`). The one list-of-records machinery
  was generalised first, as a structure-only step (`Given::Records`, `Value::Records` with its table, `Kind::records()`);
  a new kind of record is a new table and a new `Kind`, nothing else. Every number of a record is read once, so a Param
  moves a box.
- **A mark's attributes are `role` and `mark`, not `part` and `index`** (deviation from the brief). `part` already means the
  index of a part of a `split` shape (a whole number) and `index` is the shape's own place in the list, renumbered by every
  step, so neither can name which mark a shape belongs to. `mark` counts the marks (boxes in order, then points) and `role` is
  text (`bracket`, `cross`, `leader`, `text`), so `only(role="text")` styles the glyphs and `without(role="text")` the
  lines. `class` was not used: it is the SVG import's attribute, and `class` is a keyword in Python.
- **A label's place is a rule, not a solver.** The leader runs `cross + text_size` up and to the right from the box's top
  right corner or from the point; the label's pen is a quarter em past its end and the middle of a capital is on its height.
  A label is therefore right of its box and beyond its own crosshair's reach by construction (a test), whatever the sizes.
  Two close marks can overlap labels; placing them apart is a solver the library does not have (a caller moves the boxes).
  The quarter em (`LABEL_GAP`) is a documented constant that only keeps the leader's end off the first glyph.
- **A refusal names the argument the caller gave, once.** Every coordinate a grid or a mark makes is a sum (a cell times
  its columns, a corner plus a width, a point plus a cross, a leader plus a text size) checked in one owner against the
  argument that added to it (`cell`, `size`, `width`, `height`, `cross`, `text_size`), so the geometry below never fails
  on its own `dx` or `point`. A limit names the list that took the running total over it (`boxes` or `points`). The
  shared glyph entry (`string`) renames the layout's `size` to the caller's own (`text_size` in `marks`). A step is placed
  once: errors of the call's own arguments are placed at one closure, applied to nothing else, and whatever takes the
  `Context` (layout, work charges) returns errors placed already. Typing that difference (a placed and an unplaced
  `Error`) would change the public `Error`, so it is a convention with a test over the refusals instead.
- **The font: JetBrains Mono.** The `[wght]` variable font of `google/fonts` (`6e4b84c976ca`), SIL OFL 1.1 with no Reserved
  Font Name, made into WOFF2 by fontTools as the others (nothing removed or changed): 72 KB as WOFF2 (187 KB as TrueType),
  one axis (`wght` 100 to 800), every glyph 0.6 em wide, 976 glyphs, drawn for code, so `#@.:-=+*%` are distinct at small
  sizes. *Alternatives:* Noto Sans Mono and IBM Plex Mono are larger or have more axes or weights to carry for the same job.
  The three bundled fonts are 472 KB as WOFF2 (the wasm module grows by what the font file weighs: it is compiled in).
- **Limits.** A grid is at most `MAX_GLYPHS` characters (refused before anything is drawn) and charges its shapes and the
  segments of every cell's outline to `MAX_SHAPES` and `MAX_OUTLINE_SEGMENTS` before making any. `MAX_MARKS` 10,000 boxes
  and points, each at most a few dozen shapes, with `MAX_GLYPHS` characters of labels between them.
