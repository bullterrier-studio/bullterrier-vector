# The code API

Status: V1a, designed from the example programs in [examples/](examples/); waiting for the owner's review. Built so far,
in Rust and Python: `Path`, `Point`, `Rect`, `Path.map_points` and the errors (V2a); `Param`, `Expr`, `Shape`, `Style` and
`Drawing` with `translate`, `rotate`, `scale`, `fill`, `stroke`, `opacity`, `blend`, `only`, `without`, `with_values`,
`parameters()`, `shapes()` and `size`, and the style verbs on a `Path` (V2b). A `Shape`'s `style` has `fill`, `stroke`,
`stroke_width`, `join`, `cap`, `miter_limit`, `profile`, `halftone`, `opacity`, `blend`, `shadow`, `knockout`, `clip` and `mask` (V10, V10d); the rest of the members below arrive
with their steps. Also built: `text` (V5) with `width`, `fit` and `inside` (V6), and `along` (V6) on `Path` and `Drawing`, all in Rust
and Python, and `text` and `along` in a document; and the per-item numbers `by_index`, `by_order`, `random` and
`stagger`, and the steps `split`, `merge` and `slice` (V7), and the booleans `union`, `subtract`, `intersect`, `exclude`
and `resolve` (V8), and the style steps `shadow`, `knockout` and `halftone` (V10) and `clip` and `mask` (V10d), the offsets (V9) and the path modifiers `resample`, `dash`, `place_along`, `simplify` and `reverse` and tapered strokes (V11), in Rust, Python and a document. In Python `stroke`'s `width`, `join`, `cap` and `miter_limit` default to `None`, which the core reads as the
defaults written here. The rest is not built ([roadmap.md](roadmap.md)). Reasons:
[design-decisions.md](design-decisions.md), "The code API". The JSON form of a drawing: [formats.md](formats.md).

Signatures are Python's; "Languages" gives the Rust and JavaScript forms. The library is imported under the alias
`btvec` in every language.

```python
import bullterrier_vector as btvec
```

## Examples

| Program | Shows |
|---|---|
| [kinetic_type.py](../examples/kinetic_type.py), [.rs](../crates/core/examples/kinetic_type.rs), [.mjs](../crates/wasm/examples/kinetic_type.mjs) | reference piece 1 (M3), the same piece in all three: variable-font axes, a kick running through the letters, riso inks with a plane that knocks the letters out, a bend and a morph, a clip |
| [hatched_poster.py](examples/hatched_poster.py) | reference piece 3: flow-field hatching by tone, rings of a distance field, a plotter file |
| [star.py](../examples/star.py) | building and measuring a `Path` |
| [ring.py](../examples/presets/ring.py) | offsets and a boolean, eager on a `Path` and as steps on a `Drawing` |
| [cutout.py](../examples/cutout.py) | `resolve`, `subtract`, `intersect` and `exclude`: a headline cut out of a plate, the lens of two circles |
| [cut_paper_layers.py](../examples/cut_paper_layers.py) | `offset` layers of a headline nesting by a `step` Param, an `outline` cut line round them |
| [text_on_path.py](../examples/text_on_path.py) | text bent along a path, moving round it |
| [draw_on.py](../examples/draw_on.py) | draw-on by `progress`, staggered letter by letter |
| [marching_dashes.py](../examples/marching_dashes.py) | a dash pattern with a moving phase |
| [beads_on_a_path.py](../examples/beads_on_a_path.py) | petals stamped round a ring, moving round it, sized per copy |
| [brush_taper.py](../examples/brush_taper.py) | a brush stroke that thins at its ends, drawn on |
| [wobble_outline.py](../examples/wobble_outline.py) | a small noise wobble; a font axis as a described parameter |
| [bent_type.py](../examples/bent_type.py), [.rs](../crates/core/examples/bent_type.rs), [.mjs](../crates/wasm/examples/bent_type.mjs) | the deformer half of piece 1, in all three languages: a line that arches, ripples, twists and wobbles on `progress` and `phase` |
| [flow_hatch.py](../examples/flow_hatch.py), [.rs](../crates/core/examples/flow_hatch.rs), [.mjs](../crates/wasm/examples/flow_hatch.mjs) | a headline hatched along its own outline, closer at the edge, in all three languages |
| [trace_sequence.py](../examples/trace_sequence.py), [.rs](../crates/core/examples/trace_sequence.rs) | a drifting disc with dropouts, five frames traced alone and steadied with `Field.smooth_maps` (Python and Rust) |
| [trace.py](../examples/trace.py), [.rs](../crates/core/examples/trace.rs), [.mjs](../crates/wasm/examples/trace.mjs) | a relief traced into stacked cut-paper layers and a dithered strip traced exactly, in all three languages |
| [line_families.py](../examples/line_families.py), [.rs](../crates/core/examples/line_families.rs), [.mjs](../crates/wasm/examples/line_families.mjs) | three hills drawn only from lines that follow a tone (ridgelines, rings, dots), and the plotter SVG of them, in all three languages |
| [glyph_morph.py](../examples/glyph_morph.py), [.rs](../crates/core/examples/glyph_morph.rs), [.mjs](../crates/wasm/examples/glyph_morph.mjs) | a letter that turns into a circle and CAT into HORSE on one `progress`, in all three languages |
| [glyph_scatter.py](../examples/glyph_scatter.py) | letters split into parts, scattered by seeded random numbers |
| [halftone.py](../examples/halftone.py), [.rs](../crates/core/examples/halftone.rs), [.mjs](../crates/wasm/examples/halftone.mjs) | discs with a tone each, and four panels that are each a dot shape on a screen with a ramp `Field` for the tone, over paper, in all three languages |
| [cut_type.py](../examples/cut_type.py) | a headline faded by one mask, over stripes clipped to a circular window |
| [gradient_type.py](../examples/gradient_type.py) | a headline filled with one gradient across the frame, a glow and a ring with radial ones; the preset `gradient_type` (and `bent_type`, above) is a tuned variant of its example, which is the documented teaching program |
| [series_bars.py](../examples/series_bars.py) | a series of numbers as bars, with plain code |
| [frame_export.py](examples/frame_export.py) | a transparent PNG sequence and a clip from a function of time; a knockout |
| [mesh_upload.js](examples/mesh_upload.js) | a mesh in WebAssembly memory uploaded to WebGL, the pointer pattern |

## Four ideas

**`Path`** is a Bezier outline: one or more sub-paths, each open or closed. It is a plain value. Every verb on it
computes at once and returns a new `Path`; nothing changes the old one. This is the scripting layer: build, measure,
cut, combine and offset geometry the way Paper.js does, with callbacks where they help.

**`Shape`** is what a drawing is made of: one `Path` with a style (fill, stroke, opacity, blend mode, shadow, knockout)
and attributes (its index, and what its generator wrote: glyph, char, word, line, run, copy, label, level, part, order 0..1, the
origin it turns about; from `Drawing.from_svg`, the text `id`, `class` and `group`). A shape is read, never built by hand: `drawing.shapes()` evaluates a drawing into a list of
them. A list of shapes is a plain list; there is no collection type.

**`Drawing`** is an immutable tree of steps that evaluates to a list of shapes. Every call on a drawing returns a new
drawing with one more step; the old one is unchanged and can be used again, in several places. A drawing holds no
callbacks, so it draws again with new values, saves to JSON, and runs in a page or on a server unchanged.

**`Param`** is a named number with a unit, a range, a default and a meaning. Any number in a step may be a `Param`, an
expression on Params (`1 + 0.08 * pulse`), or a per-item number (`btvec.by_index(0, 1)`); its value is given when the
drawing is drawn (`drawing.to_png(1080, 1350, weight=820)`). `drawing.parameters()` lists them, so a tool can build a
slider or map a signal onto one without knowing the steps.

The geometry verbs (`translate`, `offset`, `subtract`, `slice`, `wobble`...) have the same names, arguments and meaning
on a `Path`, where they compute now, and on a `Drawing`, where they add a step that computes the same thing for every
shape at draw time. One implementation serves both. Style verbs exist only on `Drawing`; called on a `Path` they return
a `Drawing` of that path (`star.fill("#ffcc33")`). Three inputs feed steps: a `Font`, a `Field` (a number at every
point) and a `Flow` (a direction at every point).

## Path

Points are pairs: `Point`s, `(x, y)` tuples and any other pair of numbers (a list, a row of a numpy array: `Path.polyline(array)` takes an `(n, 2)` array) go in, `Point`s come out (`p.x`, `p.y`; `+` and `-` between points, `*` by
a number). `y` points down. Positions along a path are fractions 0..1 of its length (by arc length, not the curve
parameter). A path with several sub-paths is measured through them in order. A closed sub-path is measured from its
topmost point (the leftmost of a tie), in its own direction, so the start does not move when the outline changes. The
closed constructors below start at the top and run clockwise.

Every outline the library makes (`text`, `trace`, booleans, `offset`, `outline`) is in one canonical form, whatever the
source (TrueType outlines run clockwise, CFF ones counter-clockwise): outer contours run clockwise and holes
counter-clockwise; outer contours come in order of their topmost point (top to bottom, then left to right), each
followed by its holes in the same order. A draw-on therefore runs the same way on every glyph of every font (a `text`
outline is the font's own contours, in its order and direction, until `resolve`). Where two
points of an outline are nearly as high (a raised hand beside the head), the topmost can change from one map to the next
and the start jumps between them, moving everything measured from it: `point_at`, `tangent_at`, `locate`, `slice`,
`dash` and `along(start=)`. Each of these takes `near=point`, which measures every closed sub-path from its point
nearest `point` instead (the head, say). Two verbs leave the canonical form on purpose: `reverse`, and `scale` by a
negative value, which mirrors and so flips the winding. A path's region (`contains`, booleans, `offset`, `outline`, fills) is where its winding number is
not zero.

Building returns a new path each call:

| Method | Signature | Promise | Errors |
|---|---|---|---|
| `Path` | `Path()` | an empty path | |
| `move_to` | `move_to(point)` | starts a new sub-path at `point` | |
| `line_to` | `line_to(point)` | a straight segment to `point`; after `close()` it starts a new sub-path at the closed one's start | `ParameterError` before any `move_to` |
| `curve_to` | `curve_to(control1, control2, point)` | a cubic Bezier segment to `point` | as `line_to` |
| `quad_to` | `quad_to(control, point)` | a quadratic Bezier segment to `point` | as `line_to` |
| `close` | `close()` | closes the open sub-path: a straight line back to its start is implied, not a segment of its own | as `line_to` |
| `Path.line` | `Path.line(start, end)` | one open segment | |
| `Path.polyline` | `Path.polyline(points, *, closed=False)` | straight segments through `points`; a closed one whose last point is its first does not store it twice | fewer than 2 points |
| `Path.rect` | `Path.rect(x, y, width, height, *, radius=0)` | a closed rectangle from the left end of its top edge, corners rounded by `radius` | a negative size; `radius` negative or over half the shorter side |
| `Path.circle` | `Path.circle(center, radius)` | a closed circle | `radius` not above 0 |
| `Path.arc` | `Path.arc(center, radius, start, end)` | an open circular arc from the angle `start` to `end`, in degrees clockwise on the screen from the +x axis (`y` down, as for `rotate`: 0 is the right of the center, 90 below it); `end` below `start` runs counter-clockwise. The sweep `end - start` is at most 360 and not 0; a sweep of 360 is a full circle that is still open (it ends where it starts, with no close: a stroke has two caps, `Path.circle` makes the closed one). Cubics, one per quarter turn at most, within 3e-4 of the radius of the circle. Rust: `Path::arc(center, radius, start, end)` | `radius` not above 0 or past 1e100; `start` or `end` NaN or infinite; `end` equal to `start` or more than 360 from it; `center` past 1e100 |
| `Path.polygon` | `Path.polygon(center, radius, sides)` | a regular polygon, a corner at the top | `sides` outside 3 to 100000; `radius` not above 0 |
| `Path.star` | `Path.star(center, outer, inner, *, points=5)` | a star, a tip at the top; `inner` may exceed `outer` | `points` outside 2 to 100000; a radius not above 0 |
| `Path.from_svg_path` | `Path.from_svg_path(data)` | the path an SVG `d` attribute describes; arcs become cubics, one per quarter turn at most; empty data is an empty path | `data` malformed, or a coordinate not finite or beyond ±1e100 (names the first bad command and its byte offset) |

Measuring and hit-testing (on `Path` only):

| Method | Signature | Promise | Errors |
|---|---|---|---|
| `length` | property | the length in design pixels | |
| `bounds` | property | the smallest `Rect(x, y, width, height)` around the outline | `GeometryError` on an empty path |
| `point_at` | `point_at(at, *, near=None)` | the point a fraction `at` of the way along | `ParameterError`: `at` outside 0..1 or NaN; `GeometryError`: an empty path |
| `tangent_at` | `tangent_at(at, *, near=None)` | the unit direction of travel there | as `point_at`; `GeometryError` also for a path with no length |
| `locate` | `locate(point, *, near=None)` | the position 0..1 of the point on the path nearest `point` | `GeometryError`: an empty path |
| `contains` | `contains(point)` | whether `point` is inside the filled region (nonzero rule; an open sub-path fills as if closed; a point exactly on the outline may be either) | `ParameterError`: `point` not finite |
| `subpaths` | `subpaths()` | a `list[Path]` of its sub-paths, in path order, each alone exactly as written (closed or open): an outline and its counters come apart, so an O gives two, winding opposite ways; joined again in order they draw the path. An empty path gives `[]`. Rust `subpaths()`; not in JavaScript (`Path` is not exposed there) | |
| `split` | `split()` | a `list[Path]` of its parts, in path order: each closed outline with the holes inside it, and each open line; a closed sub-path inside an odd number of others is a hole of the innermost, inside an even number it is a part of its own | |
| `map_points` | `map_points(fn)` | the path with every point, on and off the curve, passed through `fn(point) -> point` | what `fn` raises |
| `to_svg_path` | `to_svg_path()` | SVG path data for the path | |

`map_points` is the one verb that takes a callback, so it has no step on `Drawing`. What `fn` raises comes out unchanged;
a point it returns that is not finite is a `ParameterError` naming `fn`.

Settled edge cases:
- **Plain numbers** are finite or a `ParameterError` naming the argument: a NaN or an infinity never reaches a result.
- **Coordinates** are within ±1e100: a point, size or radius beyond it, or one a transform, arc or constructor would
  produce, is a `ParameterError` naming the argument and the value.
- **Corners and boundaries.** `tangent_at` at a corner is the direction *leaving* it (the next segment's); at `at=1` it is
  the last segment's, and on a closed sub-path the same as at 0; segments of no length are skipped. A position on the
  boundary between two sub-paths belongs to the later one. `locate` of a point equally near several places gives the
  smallest position, and the start of a closed sub-path is 0, never 1. Of equally near places `near` takes the first
  as written.
- **No length.** A path that is only a start point (a lone `M`) has length 0; `point_at` gives that point, `locate` 0,
  and `tangent_at` raises `GeometryError`: it has no direction.
- **Empty path.** Length 0; `bounds`, `point_at`, `tangent_at`, `locate` and `slice` raise `GeometryError`; `split` gives
  `[]`; `translate`, `rotate`, `scale` and `reverse` give the empty path.
- **`slice`.** `start` above `end` is a `ParameterError` (`reverse()` runs a path backwards); `slice(a, a)` is a lone
  `M` at that point (length 0, not empty). A closed sub-path wholly inside the piece stays closed, so `slice(0, 1)` of a
  closed outline is closed again; any other piece is open. Curves stay curves, cut where the positions fall.
- **`reverse`** reverses the order of the sub-paths and each of them, so `point_at(t)` equals `reverse().point_at(1 - t)`
  except exactly on a boundary between sub-paths, which belongs to the later one in either direction. Reversing twice
  gives the path back, except that a closed sub-path loses the straight segments of no length that begin it.

## Verbs on Path and Drawing

Each verb below exists on both. On a `Path` its numbers are plain numbers and it returns a `Path`; on a `Drawing` its
numbers may be Params or per-item numbers and it applies to every shape. A `Path` verb given a `Param` raises
`TypeError` ("Conventions"): wrap the path in `btvec.Drawing(path)` first. `about` defaults to the shape's `origin` attribute (a
glyph's cap-height center), else the center of its bounds: on a stack of shapes, each turns or scales about its own
origin. To turn the whole stack as one, give `about=` a point, such as the frame's center. `translate`, `rotate` and
`scale` change geometry and `origin` only (`translate` adds to it, `rotate` and `scale` turn and scale it with the shape,
so it stays put under the default `about`): stroke `width`, `shadow` and `halftone` `pitch` are design pixels and stay as
set; to scale one with the shape, give it the same Param (`stroke(color, width=24 * pop)`).

| Verb | Signature | Promise | Errors |
|---|---|---|---|
| `translate` | `translate(dx, dy)` | moves by `dx`, `dy` | |
| `rotate` | `rotate(angle, *, about=None)` | turns clockwise on the screen (`y` down) about `about`; `angle` in degrees, any size, wrapping; multiples of 90 are exact | |
| `scale` | `scale(by, *, about=None)` | scales by a number `by`, or by an `(x, y)` pair separately, about `about`; a negative value mirrors | `by` (or a component) equal to 0 |
| `union` | `union(other, *, tolerance=0.01)` | the region inside either | `ParameterError` (below) |
| `subtract` | `subtract(other, *, tolerance=0.01)` | the region inside this and outside `other` | as `union` |
| `intersect` | `intersect(other, *, tolerance=0.01)` | the region inside both | as `union` |
| `exclude` | `exclude(other, *, tolerance=0.01)` | the region inside exactly one (the symmetric difference) | as `union` |
| `resolve` | `resolve(*, tolerance=0.01)` | its own overlaps removed: the region it fills, as closed polygons | as `union` |
| `offset` | `offset(distance, *, join="round", miter_limit=4, tolerance=0.01)` | the region it fills moved outward by `distance` (inward when negative; past nothing, an empty path); joins `round`, `miter`, `bevel`; a `miter` sharper than `miter_limit` (SVG's ratio) is clipped flat at that length (SVG's `miter-clip`), not bevelled: the engine cannot bevel | `ParameterError`: `distance` not a number within ±1e100, `miter_limit` below 1, `tolerance` (as `union`), a result beyond ±1e100 |
| `outline` | `outline(width, *, profile=None, join="round", cap="round", miter_limit=4, tolerance=0.01)` | the region a stroke of `width` paints, as a closed outline; caps `butt`, `round`, `square`. With a `profile` the width varies along the stroke ("Tapered strokes" below); `join` and `cap` must then be `"round"` and `miter_limit` is unused | `ParameterError`: `width` not above 0 and within ±1e100, `miter_limit` below 1, `tolerance` and the result as `offset`; with a `profile`: it empty, over `MAX_PROFILE_STOPS` stops, a position outside 0..1 or going back, a factor not from 0 to 1e100, `join` or `cap` not `"round"`, or the discs' corners over `MAX_SEGMENTS` (a huge `width` at a fine `tolerance`; refused before any is made) |
| `sweep` | `sweep(dx, dy, *, tolerance=0.01)` | the region it fills (nonzero rule) covers as it moves `dx` to the right and `dy` down in a straight line (the Minkowski sum with the segment): the region, its moved copy and the parallelogram every edge passes over, united; a hole it crosses far enough fills in; `(0, 0)` gives the region it fills, as `offset(0)`. On a `Drawing` `dx` and `dy` are numbers for each shape, as `translate`'s; the `origin` is not moved | `ParameterError`: `dx` or `dy` not within ±1e100 or moving a point past it, `tolerance` (as `union`), past `MAX_SEGMENTS` (the parallelograms, five points an edge, charged before any is made, then the pairs of edges that may cross, as the booleans: a long diagonal sweep of a shape of many edges is refused, naming `dx` on a `Path`, the step on a `Drawing`); a shape far below the vector (1e90 against 10) is lost to the engine's rounding, as for the booleans |
| `slice` | `slice(start, end, *, near=None)` | the piece between two positions along it (draw-on: `slice(0, progress)`); on a `Drawing` each number is read per shape, so `text("WRITE").slice(0, stagger(progress, spread=0.6))` draws the letters on one after another (Rust: `drawing.slice(start, end, Slice::new())`, `drawing.slice(start, end, Slice::new().near(near))`); with `near`, a closed sub-path is measured from its point nearest `near`, as for every verb that takes it | a position outside 0..1; `start` above `end`; on a `Path`, `GeometryError`: an empty path (on a `Drawing` a shape with nothing in it stays) |
| `reverse` | `reverse()` | the same outline travelled the other way. On a `Drawing` a step with no arguments: every shape is reversed, style, attributes and `origin` are kept, so `text(...).reverse().slice(0, progress)` draws on from the other end | |
| `dash` | `dash(pattern, *, phase=0, near=None)` | the pieces a dash pattern (`[on, off, ...]` in pixels) leaves. An odd-length pattern repeats itself to make an even one, as SVG's `stroke-dasharray` does (`[a]` is `[a, a]`, `[a, b, c]` is `[a, b, c, a, b, c]`). Each sub-path starts the pattern at its own start (a closed one at its top, or the point nearest `near`, as `slice`). `phase` is in cycles of the whole even pattern and wraps; it moves the dashes forward along the path (marching ants are `phase` a Param going 0 to 1; `phase` 1 is `phase` 0, negative phases run back). The pieces are open, curves stay curves cut where the pattern falls; a closed sub-path wholly inside one dash stays closed; a sub-path with no length stays a lone point, a path every dash misses is empty, an empty path stays empty. Shortcut: a closed sub-path's last dash, when it runs to the end, and its first, when it starts at the start, are two pieces, not one. On a `Drawing`, every number of `pattern` and `phase` is a per-item number (a Param allowed), style and attributes are kept | `ParameterError`: an empty pattern, more than `MAX_DASH_PATTERN` lengths, a length not a number above 0 and within 1e100, `phase` NaN or infinite, a bound on the pieces (cycles crossed + 1, times dashes per cycle) above `MAX_SEGMENTS`, charged before any is made |
| `resample` | `resample(spacing)` | straight segments with corners `spacing` pixels apart along it: per sub-path, at 0, `spacing`, 2 `spacing`, ... (measured as `length` measures; a closed sub-path from its top, as `point_at`; no `near`), every corner on the path; an open sub-path keeps its end, so its last gap is at most `spacing`; a closed one stays closed and its last gap wraps (a `spacing` past its perimeter leaves its start alone); a sub-path of no length stays a point, an empty path stays empty. On a `Drawing` `spacing` is a per-item number, `origin` is untouched; curves are measured on their flattening (within 0.01 pixel), so on a curve a gap is `spacing` to about that much | `ParameterError`: `spacing` not a number above 0 and within ±1e100; more than `MAX_SEGMENTS` corners (refused before any are made) |
| `simplify` | `simplify(*, tolerance=0.5)` | few smooth curves (sharp corners kept) within `tolerance` pixels, so a dense polyline becomes a few smooth curves. Each sub-path is flattened (within 0.01 pixel), cut at every vertex where the direction turns by more than 45 degrees (a corner, kept exactly) and each run between corners fitted by cubic Beziers (Schneider's fit) or, where all its points are within `tolerance` of its chord, one line: every input point is within `tolerance` of the result, and every point of the result within it of the input (a cubic is accepted only when a bound on its speed proves that, else the run is split, down to the input's own segments). An open sub-path keeps its ends; a closed one stays closed, keeps its written start and, with no corner, is fitted as a smooth loop; a sub-path never gets more segments than it had (it is kept as it was otherwise); a sub-path with no length stays a point, an empty path stays empty. On a `Drawing` `tolerance` is a per-item number, style, attributes and `origin` are kept. Known limit: a turn is measured between neighbouring edges, so a noisy trace that turns sharply at every point keeps its jitter as corners: jitter bigger than the point spacing keeps its kinks, and `resample` does not smooth it | `ParameterError`: `tolerance` not a number above 0 and within 1e100; more than `MAX_SEGMENTS` flattened edges (refused as each curve is flattened) |
| `along` | `along(path, *, start=0, near=None)` | bent along `path`: a point `(x, y)` lands `start` lengths of `path`, plus `x` pixels, along it, and `y` pixels to the right of the way it runs there (`y` down: on a path running right, below it). `path` is walked as one length, its sub-paths in order, in the direction of the curve at that place (not of the straight edge it is flattened to). A closed `path` (every sub-path with a length closed) wraps, so a `cycle` Param turns shapes round it; any other goes on straight past its ends along the direction it ends or begins in, so no `start` or `x` is out of range. `near` as for `point_at`. The result is straight lines within 0.01 pixel of the true bend (each flattened edge is halved up to 8 times), counted against `MAX_SEGMENTS`; a sub-path keeps its closedness; the `origin` attribute is left where it was. An edge longer than half a closed `path` can be bent too coarsely: a known limit. Text on a path is `text(...).along(path)` | `GeometryError`: `path` empty or of no length; `ParameterError`: `start` NaN, infinite or sending a point beyond ±1e100, `near` not finite |
| `place_along` | `place_along(path, *, count=None, spacing=None, start=0, turn=True, near=None)` | `Drawing` only: a copy of all the shapes at each stop along `path`, carried and not bent (`along` bends). Exactly one of `count` (whole, at least 1; an open path has a stop on both ends, one copy sits at the start; a closed path, every sub-path with a length closed, has them evenly round, none repeated at the end) and `spacing` (stops at 0, s, 2s, ... as far as the path goes, the end only where it falls on one; on a closed path as many as fit with no two closer than `spacing`, at least one). Both are plain numbers, not Params, so the number of shapes does not depend on a value. `start` is a fraction of the path's length (a number, a Param or an expression, one value for the whole step; a closed path wraps, an open one goes on straight past its ends as `along` does). Positions are walked as `along` walks `path` (its sub-paths in order as one length; `near` as for `point_at`). A copy is carried by one anchor, the origin all the shapes share, else the middle of the bounds of all of them, so a stamp of several shapes keeps its layout; with `turn` it is turned so its x axis runs along the direction of the path there (the leaving direction at a corner), `turn=False` only moves it. Each shape keeps style and attributes, its `origin` carried with it (on the path point if it was the anchor), and gets `copy` (the stop, from 0); a drawing of one shape also gets as `order` the share 0..1 of its copy (0 for one copy), so `by_order` and `by_index` vary per copy. A stamp of several shapes keeps its shapes' own `order`, and per-item numbers index its shapes, not its copies: `merge(by="copy")` makes a shape of each copy. The shapes are listed copy by copy. Rust: `drawing.place_along(&path, Spacing::Count(12), PlaceAlong::new().start(&t).turn(false))`, `Spacing::Every(40.0)` | `ParameterError`: `count` 0 or not whole, `spacing` not above 0 and within 1e100, neither or both of them given, `start` NaN or infinite, `near` past ±1e100, more than `MAX_SHAPES` shapes or `MAX_SEGMENTS` segments (copies times shapes, refused before any is made); `GeometryError`: `path` empty or of no length |
| `branch` | `branch(spacing, *, length, fork_spacing=80, forks=2, shorter=0, angle=45, wander=30, wavelength=80, seed=0)` | `Drawing` only: every shape replaced by forking threads grown out of the outline of the region it fills ("Threads" below). One open shape per root-to-tip path, keeping the shape's style and attributes, with `root` (from 0 on the shape), `forks` (the forks the path takes; 0 is the trunk), `order` and the root as `origin`. `order` is the path's share 0..1 of the paths its input shape grew (as `place_along` writes its copies'), replacing the shape's own, so `stagger` after `branch` runs the paths one after another, not the letters | `ParameterError`: `spacing`, `length` or `fork_spacing` not above 0 and within 1e100; `forks` not a whole number from 0; `shorter` outside 0..1; `angle` or `wander` not finite; `wavelength` not from 1e-200 to 1e100; `seed` not whole; more than `MAX_SEGMENTS` points or `MAX_SHAPES` paths, counted before any thread grows (the message gives the paths and the points, and the limit passed), naming `wavelength` (one path), `fork_spacing` (one root's paths) or `spacing` (all the roots) |
| `wobble` | `wobble(*, amplitude=4, wavelength=80, phase=0, seed=0)` | points displaced by smooth seeded noise: a point moves by `(dx, dy)`, two independent fields, each at most `amplitude` pixels in size (so a displacement is at most `amplitude * sqrt(2)`), changing by at most `3.75 * amplitude` per `wavelength` along either axis (value noise on a lattice one wavelength wide, quintic blend). `phase` is in cycles and wraps first (`phase` 1 is `phase` 0; `phase + 1` is `phase` up to the rounding of the sum, to the bit for a phase such as 0.25), four lattice cells to the cycle, and moves the noise smoothly; `seed` is a whole number within ±(2^53 - 1) as for `random`, the same on every target; `amplitude` may be 0 (the outline, flattened) or negative. Anchored in the design frame as `wave` is: one field runs across all the letters of a word. The path is flattened and each edge cut into equal pieces no longer than a 24th of the wavelength, every vertex kept (corners stay corners); a chord strays from the noise by under 0.8% of `amplitude`, and the result is straight lines, a sub-path keeping its closedness. On a `Drawing` every argument is a per-item number (a Param allowed), style, attributes and `origin` are kept | `ParameterError`: `wavelength` not from 1e-200 to 1e100 (shorter, a coordinate over it would overflow); `amplitude` or `phase` NaN or infinite; `seed` not a whole number within ±(2^53 - 1) (on a `Drawing`, when drawn); a displaced point beyond ±1e100 (naming `amplitude`); more than `MAX_SEGMENTS` pieces, counted before any is made (naming `wavelength`) |
| `wave` | `wave(*, amplitude=8, wavelength=120, angle=0, phase=0)` | points displaced sideways by a sine travelling towards `angle`: a point `(x, y)` at distance `d = x cos(angle) + y sin(angle)` along the direction of travel moves `amplitude * sin(2 pi (d / wavelength - phase))` along the perpendicular `(-sin(angle), cos(angle))`. `angle` is in degrees, clockwise on the screen as for `rotate` (`y` down: angle 0 travels towards +x and displaces down the screen where the sine is positive; on a `Drawing` step a `Deg` in Rust, a plain number in Python); `phase` is in cycles and wraps first (`phase` 1 is `phase` 0); `amplitude` may be 0 (the outline, flattened) or negative. The wave is anchored in the design frame, the point's own coordinates, not the shape's: one wave runs across all the letters of a word, a shape translated before the wave slides through it, one translated after carries it along. The path is flattened and each edge cut into equal pieces no longer than a 24th of the wavelength, every vertex of the flattening kept (corners stay corners); every point of the result is within 0.86% of `amplitude` of the sine it stands for, and the result is straight lines, a sub-path keeping its closedness. On a `Drawing` every argument is a per-item number (a Param allowed), style, attributes and `origin` are kept (`origin` is not moved) | `ParameterError`: `wavelength` not above 0 or past 1e100; `amplitude`, `angle` or `phase` NaN or infinite; a displaced point beyond ±1e100 (naming `amplitude`); more than `MAX_SEGMENTS` pieces, counted before any is made (naming `wavelength`) |
| `twist` | `twist(angle=90, *, radius=None, about=None)` | points turned about `about` by `angle * falloff(r)` degrees, `r` the distance of the point from `about` and `falloff(r) = 1 - s^2 (3 - 2s)`, `s = min(r / radius, 1)`: the center turns by the whole `angle`, the turn dies away smoothly (slope 0 at both ends) and a point at `radius` or beyond does not move, so the rim holds still while the middle turns. The turn is clockwise on the screen as for `rotate` (`y` down; a point near the center moves as `rotate(angle)` moves it), and a twist only turns: every point keeps its distance to `about`. `about` is a point (any plain number or a pair of them; on a `Drawing` per item), defaulting as for `rotate` and `scale` to the shape's `origin` attribute, else the middle of its bounds (on a `Path`: the middle of its bounds), and `radius` (a number above 0, at most 1e100) to the distance from `about` to the farthest corner of the shape's bounds, so the whole shape twists and its far corner stays (a shape with no extent from `about` has radius 1). `angle` is any finite number of degrees and does not wrap: 360 turns the middle once round against the rim; on a `Drawing` step a `Deg` in Rust (`drawing.twist(Deg(90.0))`, `drawing.twist(Twist::new(Deg(90.0)).radius(80.0).about((100.0, 100.0)))`), a plain number in Python. The path is flattened and each edge cut into equal pieces no longer than `radius / n`, `n = 7.5 sqrt(a^2 + 14 a / 3)` for `a` the angle in radians (the pieces grow with the turn), every vertex of the flattening kept (corners stay corners); every point of the result is within a hundredth of `radius` of the true twist, and the result is straight lines, a sub-path keeping its closedness. On a `Drawing` every argument is a per-item number (a Param allowed: `twist(progress * 180)` turns a letter as `progress` goes 0 to 1), style, attributes and `origin` are kept (`origin` is not moved: it is the pivot by default) | `ParameterError`: `radius` not above 0 or past 1e100 (or more than `MAX_SEGMENTS` pieces, counted before any is made); `angle` NaN or infinite, or so large that not even one radius can be cut into `MAX_SEGMENTS` pieces; `about` NaN, infinite or past 1e100 |
| `bulge` | `bulge(stretch=0.5, *, radius=None, about=None)` | points moved along their ray from `about` from distance `r` to `r * (1 + stretch * falloff(r))`, the same `falloff` and the same `about` and `radius` rules as `twist`: a positive `stretch` bulges the middle outward (a fish-eye), a negative one pinches it inward, 0 leaves the outline as it is (flattened), and a point at `radius` or beyond does not move (to the bit). The center stays and every point keeps its direction from `about`. `stretch` must be above -1 and below 16/11 (1.4545...): `r -> r'` has the slope `1 + stretch (1 - 9 s^2 + 8 s^3)`, whose lowest value is `1 - 11/16 stretch` (at `s = 3/4`) and highest 1, so it is strictly increasing exactly there and no part of the shape folds over itself. Pieces no longer than `radius / (15 sqrt(abs(stretch)))`, every vertex kept, within a hundredth of `radius` of the true bulge; the result is straight lines, a sub-path keeping its closedness. On a `Drawing` every argument is a per-item number (a Param allowed), style, attributes and `origin` are kept | `ParameterError`: `stretch` NaN, infinite, not above -1 or not below 16/11 (a literal is refused at the call, naming `step N (bulge).stretch`, a Param or expression when drawn), or sending a point beyond ±1e100; `radius` as for `twist`; `about` as for `twist` |
| `envelope` | `envelope(top, bottom=None, *, frame=None, fraction=1)` | the `frame` `(x, y, width, height)` (a `Rect` or four numbers; default: the bounds of this `Path`, on a `Drawing` the bounds of all its shapes read when the step is drawn, so one frame is shared and a line of text keeps its height among its lines) mapped onto the region between the curve `top` and the curve `bottom` (default: the frame's own straight bottom edge `y = y0 + height`): a point `(x, y)` goes to `lerp(T(u), B(u), v)`, `u = (x - x0) / width` and `v = (y - y0) / height`, `T(u)` and `B(u)` the points `u` of the way along the arc length of `top` and `bottom`. A column of the frame stays a straight line from `T(u)` to `B(u)`, so the columns of a word stay straight and shear, which `along` (letters fanned round the curve) does not; to keep every column's height give `bottom` as `top` moved down by the frame's height. `top` and `bottom` set the orientation: v = 0 lies on `top`, so a `bottom` curve lying above `top` flips the word upside down (swapping the two mirrors the result: each point goes to the original's with v replaced by 1 - v), and curves that cross fold it. The curves are walked as `along` walks a guide (sub-paths in order as one length): `u` outside 0 to 1 goes on straight past an open curve's ends, a closed curve wraps and its seam is at its start (`u` = 0 and `u` = 1 are the same point). `fraction` (a number; on a `Drawing` per item, a Param allowed) blends each edge between the frame's straight edge (0) and the curve (1), `T_eff(u) = lerp(straight(u), T(u), fraction)`, any finite number: 0 is the identity, above 1 exaggerates, below 0 bends the other way; it is how a bend animates, `top` and `bottom` being literal paths. The path is flattened and each edge cut into equal pieces of at most a 256th of the frame's width, every vertex kept (corners stay corners): every point of the result is within 0.05 pixel plus 0.2% of its displacement of the exact image on the shapes the tests hold it to; the result is straight lines, each sub-path keeping its closedness, a lone point is moved, an empty path stays empty. On a `Drawing` shapes keep style, attributes and `origin` (not moved). Rust: `drawing.envelope(&top, Envelope::new().bottom(&b).frame(rect).fraction(&p))`, `path.envelope(&top, Some(&b), Some(rect), 1.0)` | `ParameterError`: `frame` NaN, infinite, past 1e100, with a width or height not above 0 (a line has no frame of its own to default to: give `frame`), or more than `MAX_SEGMENTS` pieces (counted before any is made); `fraction` NaN, infinite or sending a point past 1e100; `GeometryError`: `top` or `bottom` empty or of no length; `FormatError` naming `frame`: a document `frame` without four numbers |
| `arch` | `arch(bend, *, frame=None)` | the classic arch on the same map as `envelope` (`frame` as there): the whole band of the frame (top and bottom edge alike) is lifted by a parabola through the frame's corners whose middle is `bend * width` above them (`bend` negative: below, a sag; `y` is down, so positive lifts up the screen), so the frame's height is kept at every column, `-bend` is the exact mirror of `bend`, and a point `(x, y)` goes to `(x, y - 4 bend width u (1 - u))`. `bend` 0 is the identity. The lift is a share of the frame's WIDTH, not its height: a 365 px word with `bend` 0.3 rises 110 px, a long word wants a small `bend`. A flat baseline under a curved top is `envelope(top=<curve>)`. The parabola is closed form: `u` follows `x`, so the horizontal spacing of the letters of a word is kept (`envelope` with an arc spaces them by arc length). `bend` is a number; on a `Drawing` per item, a Param allowed, shapes keep style, attributes and `origin`. Cut and bounded as `envelope`. Rust: `drawing.arch(0.2)`, `drawing.arch(Arch::new(&b).frame(rect))`, `path.arch(0.2, None)` | `ParameterError`: `frame` as for `envelope`; `bend` NaN, infinite or sending a point past 1e100 |
| `morph` | `morph(target, fraction)` | the outline `fraction` of the way from this path to `target`: 0 is this path, 1 is `target`, and any finite number is the straight lerp of the two (outside 0 to 1 it extrapolates, so an overshooting ease leaves the hull of the two shapes). Both are flattened as for drawing. A closed contour is an outer or a hole by the sign of its area (no area: an outer), after the whole path is reversed once if its largest contour runs the other way; closed contours pair with closed ones of the same role, open with open, nearest first by the distance between the contours' centers, each taken in the box of its own path scaled to 0..1 (ties to the lower indices, so `a.morph(b, t)` fills what `b.morph(a, 1 - t)` fills). A closed pair starts where the two outlines lie nearest (128 equal steps of length, summed squared distance; no reversal search); an open pair is joined end to end or end to start, whichever is nearer. The points of a pair are the vertices of both outlines, each at its share of the length, so 0 and 1 are the flattenings of the two paths. A contour with no partner shrinks to (or grows from) one point: the place in the other path's box that matches its center's place in its own box, or its own center if the other path is empty; past 1 (before 0 for a growing one) it stays that point, where a matched contour goes on along its line. A hole with no partner is wound in the other way and the result cut to the body, so it takes away what the nonzero rule says (an outer inside it stays) and never paints outside the body (the closed part is then canonical, as for `Path.subtract`); an overlapping pair of outers is not united (`resolve` first). The result is straight lines, each sub-path keeping its closedness, grouped by role in pairing order. On a `Drawing` the shape at each index pairs with the shape of `target` (a `Drawing`, `Path` or `Shape`, its shapes taken one for one, never united) at that index; `fraction` is a per-item number (a Param, `stagger` or `by_index` allowed), read with the shape's `index` and its `order` attribute (the source's for a pair, the target shape's own for one that grows; a word's glyphs carry theirs); a paired shape keeps the source's style, attributes and `origin`; a source shape with no partner shrinks to its own center keeping its style; a target shape with no partner grows from its own center in place, with its own style and attributes, so the result has as many shapes as the longer drawing. Rust: `path.morph(&target, 0.5)`, `drawing.morph(target, Stagger::new(&progress))` | `ParameterError`: `fraction` NaN or infinite, or sending a point past 1e100; more than `MAX_SEGMENTS` points (flattening, pairs ranked and result, counted before any is made; naming `fraction` on a `Path` and the step on a `Drawing`); on a `Drawing`, a `target` that holds an error, and more than `MAX_SHAPES` shapes |
| `hatch` | `hatch(spacing, *, tone=None, flow=None, avoid=None, below=None)` | each region filled with evenly spaced open lines (below) | below |

A deformer's result is smooth at any size, however few points the outline had: `wave`, `wobble`, `twist`, `bulge`,
`envelope` and `arch` all cut each edge into pieces fine enough for the effect, and charge them to `MAX_SEGMENTS` before
making any. The bounds in the rows above (a share of the amplitude, radius or frame width) hold for a small shape; for
every deformer the result is also within 0.05 design pixels of the exact map, however big the shape (0.4 device pixels
at an 8x zoom), so a big shape gets more points and a very big amplitude or a very small radius against a big shape hits
`MAX_SEGMENTS` (a `ParameterError`).

A large `wobble`, `twist` or `bulge` stretch moves neighbouring parts of an outline across each other, so outlines can overlap or fuse (a `bulge` within its limits never folds one outline over itself, but two letters can meet); nothing checks for it, so judge it by eye.

Closing and opening are two offsets: `offset(r).offset(-r)` fills gaps and dents narrower than `2r` (a blobby mount);
`offset(-r).offset(r)` removes spurs thinner than `2r`.

### Booleans

`union`, `subtract`, `intersect`, `exclude` and `resolve` work on regions by the nonzero rule; an open sub-path fills as
if closed. `a.subtract(b)` is `a` minus `b`; `a.exclude(b)` is what only one of them fills. Curves are first cut to
straight edges that stray from them by at most `tolerance` design pixels (a number, not a `Param`). The result is closed
polygons only, in the canonical form of "Path" (each contour starts at its topmost corner, the leftmost of a tie), an empty
path where nothing is left. Overlaps and self-crossings are merged: a bow-tie becomes its two triangles, and
`text(...).resolve()` makes each glyph one outline (`text` itself keeps the font's contours).

On a `Drawing`, `other` is a `Drawing`, a `Path` or a `Shape`. Its shapes count as one region (each resolved on its own,
united once per step) and every shape of the drawing is combined with it, keeping its style and attributes. With an empty
`other`, `intersect` leaves each shape an empty path and the others keep the shape's region. `other` may hold Params.
Rust: `path.union(&other, 0.01)?`, `drawing.union(other, 0.01)`; JavaScript: in a document ([formats.md](formats.md)).

Errors: `ParameterError` for a `tolerance` not above 0 or beyond ±1e100, a result coordinate beyond ±1e100 and, on a
`Drawing`, a draw past `MAX_SEGMENTS` (the message names the step). No boolean raises `GeometryError`: the engine
resolves any outline. It is for verbs that can fail geometrically (an empty path measured, a guide of no length).

Known limits:
- **Precision follows the extent.** Both operands are rounded together, onto a grid of about 2^-60 of the half-extent of
  their joint bounding box (finer than a number holds at that size). A corner can move by an ulp of that extent when the
  other operand changes; a feature much smaller than the extent is perturbed (a 0.004 triangle beside a 600-wide bar moved
  by 1e-14) and one smaller than the grid vanishes (1e-300 beside 1e100). Near zero the engine's scale leaves the range
  of a number: exact to 1e-12 relative down to coordinates of 1e-290, then off by 3e-14 relative at 1e-295 and 2.5e-9 at
  1e-300.
- **Thin contours are dropped.** One no thicker than 4 ulps of the largest coordinate of the input (or the result, if
  more) encloses less than a number holds (a spike of no width, a retraced curve's hair pieces, a sliver of 1e-13 beside
  a 600-wide shape). So are sub-paths of fewer than 3 corners, and the spike of an open polyline that doubles back.
- **Details of the size of a nudge can vanish beside a rounding sliver.** Two edges that nearly coincide (a copy of a
  shape nudged by 1e-7) beside an overlap that a number's rounding turned into a sliver (a corner on another edge, doubled
  back, then moved by 7759.6: 1e-12 thick) lost a quadrilateral of area 8e-16 at the corner, 1e-8 thick; the area and
  every point farther than ten nudges from an edge are right. Not visible at any size a drawing is shown.
- **Touching shapes merge.** Shapes that share an edge become one contour, the shared edge dissolved; shapes that meet at
  a corner stay two. A boolean cannot tell two touching shapes from one. Collinear and duplicate vertices are not kept, so
  the vertex count differs from the input's.
- **Not stable to the last digit.** `resolve` of a result keeps its region, not its corners. `a.union(b)` and `a.resolve()`
  can differ in a corner by the rounding above, and so in the start of a contour and in the order of two outers whose
  tops are equal in exact arithmetic.
- **Work grows with the crossings of the outlines,** which no input size bounds. On a `Drawing` a step charges
  `MAX_SEGMENTS` (10,000,000, summed over the draw) for the flattened edges, for the pairs of edges whose boxes overlap
  (at least the crossings) and for the result's vertices, and is refused past it; `Path` methods called directly charge
  nothing and are unbounded, as `along` is. Random scribbles, release build: 2,000 edges (896,000 pairs) 0.68 s and 161 MB,
  4,000 (3.6 million) 3.1 s and 504 MB, 8,000 (14.3 million) 15.4 s and 2.0 GB; about 140 bytes a pair (350 for a star
  polygon), so the limit is a rough guard of 1.4 to 3.5 GB, not a budget. A smooth outline has a few pairs an edge.
- **Flattening stops at 65,536 edges a curve,** whatever the `tolerance`, which is then not met.
- **Per frame,** `resolve` on text costs about 50 microseconds a glyph and a `subtract` of a line from a plate about 130
  ([benchmarks.md](benchmarks.md), "Booleans (V8)"), measured with a new weight on every call, so nothing is cached.

### Offsets

`offset` and `outline` give the region a path fills moved, and the region a stroke paints, as closed straight-edged
polygons in the canonical form of "Path" (rows above; a `miter` clipped flat, not bevelled). On a `Drawing` they are
steps: each shape keeps its style and attributes and gets the `Path` result; a shape that shrinks past nothing keeps an
empty path. `distance` and `width` are numbers for each shape (a number, a `Param`, `by_index`, `Random`...), so a list
of shapes grows by different amounts; `join`, `cap`, `miter_limit` and `tolerance` are plain values, not Params.
Rust: `path.offset(2.0, Join::Round, 4.0, 0.01)?`, `drawing.offset(2.0, Join::Round, 4.0, 0.01)`,
`drawing.outline(2.0, Join::Round, Cap::Round, 4.0, 0.01)`, `path.sweep(dx, dy, 0.01)?`, `drawing.sweep(dx, dy, 0.01)`
(the flat-nib look: `text(...).sweep(28, -16)` in one call, where a union of shifted copies combs at the ends).

Corners: curves are flattened to `tolerance`, then corners are dropped before the offset, each only where it lies within
half of `tolerance` of the edge kept in its place, so offsetting a result again does not double its corners (five
offsets of a letter by 4 stay near the first result's size). Round joins and caps are cut to within the other half, so
a round-joined result lies between the offsets of the flattened path by `distance` (or half the `width`) less and more
`tolerance`; with `miter` and `bevel` a dropped corner throws no point of its own and the joins beside it follow the
edge that replaced it. An open line keeps its first two and last
two corners (they set its caps), and a shape thinner than `tolerance` still paints.

A miter at a hidden corner is lost with it: a corner that a line between its kept neighbours hides within half of
`tolerance` throws no miter point, so the result can differ from the exact offset of the unflattened polygon by up to
`miter_limit` times `distance` there (a square of 10 with a spike 0.0049 high on an edge, its apex just wider than the
miter cut-off, at `tolerance` 0.01: the lowest y of the result is -10.00, where the exact offset reaches -39.34 for
limit 4 and distance 10, -209.05 for limit 23 and distance 10, -38.34 for limit 10 and distance 4). It hits polygons
and polylines too: their tiny real corners are dropped like any. Bevel joins are not affected. An input whose every
tiny corner must keep its miter is expressed with a smaller `tolerance`.

Errors: `ParameterError` as the rows say; a `Drawing` names the step and the argument (`step 2 (offset).join`) and a draw
past `MAX_SEGMENTS` is refused as for the booleans. No offset raises `GeometryError`.

Known limits (the engine's, `docs/design-decisions.md`, "Offsets (V9)"): a `miter_limit` above about 23 acts as 23; a turn
under 5 degrees is bevelled; a round join or cap of a radius above about 4,000 times `tolerance` strays from its arc by
more than half of `tolerance`, by up to 0.000123 of the radius (the engine's narrowest step of 1.8 degrees strays by
`1 - cos(0.9 degrees)` of the radius); a feature far below the extent of the shape or the offset is lost to rounding,
so a `width` far below the grid, or a `distance` beyond about 1e19 times the size of the shape, gives an empty path; a
one-point sub-path paints nothing in `outline` (`stroke` draws a dot).

### Tapered strokes

`outline(width, profile=[(position, factor), ...])` paints a stroke whose width is `width * factor` at each place:
`[(0, 0), (0.15, 1), (0.85, 1), (1, 0)]` is a brush stroke that thins to a point at both ends. `position` is 0 to 1 by
arc length along each sub-path on its own (a closed one from its top, as `slice` and `dash`), so every open sub-path
tapers at its own free ends: `dash([30, 10]).outline(8, profile=taper)` tapers each dash, and `text(...)` tapers each
contour. Factors are linear between stops and hold the end values outside them; two stops at one position make a step.
A closed sub-path has no free ends, the profile runs once round it from its start, and where its end meets its start the
width steps from the last factor to the first (equal end factors make a seamless ring; ends at 0 pinch it at its top).
The stroke is the sweep of a round disc, so joins and caps are round, a factor of 0 at a free end closes the outline to a
point, and a sub-path of no length paints nothing. The disc's diameter follows the profile, so where the profile changes
faster than about half the width per unit of length the outline is wider across than its factor says: at 5 px along a
100 px line with profile `[(0, 0), (0.15, 1), (0.85, 1), (1, 0)]` and width 26 it is 16.4 wide where the factor says 8.7. On a `Drawing`, `profile` is `[[position, factor], ...]` in a
document and each number is a per-item number (a Param allowed). Rust: `path.outline(width, Join::Round, Cap::Round, 4.0, 0.01, Some(profile.as_slice()))?`,
`drawing.outline(width, Outline::new().profile(profile))`. `stroke(color, width=24, profile=taper)` is the same region as a stroke: the stroke call takes the same pair list, each output
fills what `outline` with that profile returns for the shape's path as it is when drawn (so `slice(0, progress)` after or before it
thins the tip where the path now ends, and `dash`, selection, bounds and `slice` still see the center line), fill first and
the stroke over it, with the stroke's paint (colour, translucency, gradient); the shadow, blend, opacity, clip, mask and knockout of
the shape apply once to fill and stroke together. A tapered stroke is drawn to `0.01` design pixels whatever the size of the
output, so the SVG at twice the size has numbers exactly twice as large. Known limits: the center line is measured as `length` measures it
(flattened to 0.01 pixels, whatever `tolerance` is; `tolerance` bounds the round parts); a `profile` stroke has no miter or
square ends (refused)

### Threads

`branch` grows forking threads out of each shape's outline: the thin, wandering threads that grow out of the edges of
heavy letters.

```python
growth = btvec.Param("growth", default=1, min=0, max=1)
letter = btvec.text("R", font=font, size=560, axes={"wght": 900}, anchor=(400, 600)).resolve()
threads = letter.branch(70, length=220, fork_spacing=60, forks=2, shorter=0.5, angle=40, wander=35, seed=7)
taper = [(0, 1), (1, 0.15)]
ink = btvec.Drawing(
    letter,
    threads.without(forks=0).slice(0, growth).outline(3, profile=taper),  # the forked paths, thin
    threads.only(forks=0).slice(0, growth).outline(6, profile=taper),  # the trunks, heavier
).fill("#141414")
poster = btvec.Drawing(btvec.Path.rect(0, 0, 800, 800).fill("#f2c230"), ink, size=(800, 800))
```

Rust: `letter.branch(&Branch::new(70.0, 220.0).fork_spacing(60.0).forks(2).shorter(0.5).angle(Deg(40.0)).wander(Deg(35.0)).seed(7.0))`
([examples/threads.py](../examples/threads.py), [Rust](../crates/core/examples/threads.rs),
[JavaScript](../crates/wasm/examples/threads.mjs)).

- **Roots.** Each shape's path is resolved (as `resolve`, to 0.01 pixels); each closed contour of length `P` is cut into
  `max(1, floor(P / spacing))` equal stretches from its top, with one root in each at a seeded place. A thread sets out
  along the outward normal, the left of the way the contour runs: out of an outer contour, into a hole (a counter grows
  threads too).
- **Drift.** A thread's heading is its start heading plus `wander` times the change of the one value noise (`wobble`'s)
  read along the thread every `wavelength` pixels, so it drifts smoothly and at most `2 × wander` from where it set out.
  A path takes `ceil(L / (wavelength / 24))` equal steps, `L` its length (`max(1, ...)`), so it ends exactly at `L`, a step is at
  most a 24th of `wavelength`, and a path has one point more than it has steps.
- **Forks.** The distance from the root is cut into stretches of `fork_spacing`; a thread that crosses a whole stretch,
  on a path of fewer than `forks` forks, forks once in it, at a seeded place (on its nearest point before it), turning
  `angle` degrees to a seeded side. A root has `sum C(m, k)` paths for `k` from 0 to `forks`, `m = floor(length /
  fork_spacing)` (4 stretches, 2 forks: 11 paths).
- **Lengths.** Every path of a root ends at the same distance from it: `length` times the root's factor, a seeded number
  from `1 - shorter` to 1; its stretches scale with it, so every root has the same number of paths whatever its factor.
  `slice(0, growth)` therefore grows a tree from its root at one speed, a fork appearing where its parent passes, and
  `outline(profile=)` gives the paths of a tree equal widths where they share points.
- **Seeds.** Everything random comes from `seed` and the shape's index in the list (as `random`'s), so equal letters at
  different places grow different threads, and adding a shape after one leaves its threads alone.
- **Costs of the form.** The paths of a tree share their common part, stored once per path: a plotter retraces a shared
  trunk once per tip, and a translucent ink darkens it. Draw them opaque, or `outline(width)` them first (an open
  thread has no area, so uniting raw threads gives slivers), then `merge()` and `resolve()` them into one region.
  `merge(by="root")` joins root 0 of every input shape, as `root` restarts on each: grow and unite each shape's
  threads on its own.
- `spacing`, `length`, `fork_spacing` and `forks` are plain numbers (they set how many shapes are made, as
  `place_along`'s `count`); `shorter`, `angle`, `wander`, `wavelength` and `seed` are per-item numbers.

### hatch

Evenly spaced streamlines (Jobard and Lefer) of `flow` inside the region each shape fills (nonzero rule). A line
is traced both ways from a seed and stops where it leaves the region, enters `avoid`, or comes within half a spacing of a
point of another line; the next seeds stand one spacing to either side of a line, so lines settle one spacing apart where the
flow is straight and split or end where it spreads or gathers. A line that comes round to its own start is closed. The
floor is half the spacing between any two lines, never the spacing itself. A line that ends against another line and is shorter
than three spacings (at its start) is dropped: the ticks left where the lines of two edges meet; a closed line, and a chord that
leaves the region at both ends (one longer than a spacing), stay.

| Argument | Meaning |
|---|---|
| `spacing` | pixels between lines; or a pair `(at tone 0, at tone 1)`, linear between and held beyond, read from `tone` |
| `tone` | a `Field` or a number; needed when `spacing` is a pair |
| `flow` | a `Flow`; default `Flow.angle(45)` (straight hatching) |
| `avoid` | a `Path`, `Shape` or `Drawing` (on a `Path`, a `Path`) the lines keep clear of |
| `below` | a number or a `Param` (needs a `tone`): lines only where the tone is at most this level; a line stops near where the tone passes the level and none starts above it |

The result is open lines (a shape keeps its style, so set `fill("none")` and the stroke after it). The step makes
a path of the lines for each shape. Errors: `ParameterError` for a spacing not above 0 or beyond 1e100, a pair without
`tone`, a shape so large next to the spacing that its grid would pass 4 million cells, a pair of spacings so far apart
for the shape that placing the lines would take over a billion cell visits (1 to 1000 over 400 by 400), a spacing too small
for the coordinates (near 1e17 a step must be at least a few units), or more than 10 million points; a path of two
points encloses nothing, so it gives no lines; a shape with no outline gives no lines. Rust: `path.hatch(&Hatch::new(8.0).flow(flow), Some(&avoid))`,
`Hatch::between(near, far, tone)`, `.below(level)`, `drawing.hatch(&hatch, Some(&avoid_drawing))`.

**Crosshatch and tone levels** are hatch steps stacked, each at its own angle and level (0 is dark, so the darker the
tone, the more passes reach it); no second routine:

```python
photo = btvec.Field.map(w, h, grey_bytes, box=(0, 0, 1080, 1350))  # tone of a photo, 0 black
region = btvec.Path.rect(0, 0, 1080, 1350)
passes = [(0.85, 45), (0.6, 135), (0.4, 0), (0.2, 90)]  # (level, angle)
crosshatch = btvec.Drawing(
    *[
        btvec.Drawing(region)
        .hatch(7, tone=photo, flow=btvec.Flow.angle(angle), below=level)
        .stroke("#16151a", width=0.8)
        for level, angle in passes
    ]
)
```

The same in Rust (`tone` as a spacing pair `Hatch::between(7.0, 7.0, tone)`):

```rust
let photo = Field::map(w, h, &grey_bytes, Some(Rect { x: 0.0, y: 0.0, width: 1080.0, height: 1350.0 }), None)?;
let region = Path::rect(0.0, 0.0, 1080.0, 1350.0, 0.0)?;
let passes = [(0.85, 45.0), (0.6, 135.0), (0.4, 0.0), (0.2, 90.0)]; // (level, angle)
let mut stack = Vec::new();
for (level, angle) in passes {
    let hatch = Hatch::between(7.0, 7.0, photo.clone()).flow(Flow::angle(angle)?).below(level);
    stack.push(Drawing::from(&region).hatch(&hatch, None).stroke(Stroke::new(Color::hex(0x16_15_1a)).width(0.8)));
}
let crosshatch = Drawing::new(stack);
```

And as a document (two passes; the `photo` input is supplied by whoever loads it, as formats.md "Inputs" says):

```json
{
  "version": 1,
  "inputs": { "photo": { "kind": "map" } },
  "drawings": { "region": [{ "path": "M0 0 L1080 0 L1080 1350 L0 1350 Z" }] },
  "root": [
    {
      "stack": {
        "items": [
          [
            { "drawing": "region" },
            {
              "hatch": {
                "spacing": 7,
                "tone": { "Field.map": { "width": 540, "height": 675, "data": "photo", "box": [0, 0, 1080, 1350] } },
                "flow": { "Flow.angle": { "angle": 45 } },
                "below": 0.85
              }
            },
            { "stroke": { "color": "#16151a", "width": 0.8 } }
          ],
          [
            { "drawing": "region" },
            {
              "hatch": {
                "spacing": 7,
                "tone": { "Field.map": { "width": 540, "height": 675, "data": "photo", "box": [0, 0, 1080, 1350] } },
                "flow": { "Flow.angle": { "angle": 135 } },
                "below": 0.6
              }
            },
            { "stroke": { "color": "#16151a", "width": 0.8 } }
          ]
        ]
      }
    }
  ]
}
```

Each pass places its own lines, so passes cross; a lighter tone gets fewer of them. `hatch` reads the `tone` of a
`Field.map` at the pixel centers, bilinear between them.

### Line families (V15c)

A tone (a `Field`, or a number) makes the look of lines: `rows`, `bands` and `stipple` work inside the region each shape
fills (nonzero rule), on a `Path` and as steps on a `Drawing` (a shape keeps its style, attributes and `origin`), with the
Params of a number read once with the values of the draw (a per-item number is refused). Every call is bounded before it
starts: a spacing too small for the region is a `ParameterError` naming `spacing`.

| Call | Result | Arguments |
|---|---|---|
| `rows(spacing, *, tone, amplitude)` | open lines: ridgelines | rows `spacing` apart, the first half a spacing below the region's top, each lifted towards the top by `amplitude` times the tone at the point of the row (a negative amplitude pushes down). The tone is read over the point's share of the rows (half a step along and half a spacing across, weighing 1, 2, 1 each way), so detail finer than the rows can draw (fur, grass) averages out instead of chopping the rows; a slope reads as it is. A row hides what lies behind it (higher up the picture, below its curve): it is cut where a nearer row passes over it. A row starts where its base is inside the region and is drawn only where it is still inside once lifted; a part lifted out is cut away and hides nothing. Points are half a spacing apart. |
| `bands(spacing, *, tone, width, family="lines", angle=None, center=None, wavelength=None, amplitude=None)` | closed outlines, for filling | lines whose width follows a tone: a carrier laid over the region and cut to it, each line as wide at a point as `width = (at tone 0, at tone 1)` says (each from 0, so a line can fade to nothing; linear, held beyond; the tone is read on the center line; 0 is dark, so `(thick, thin)` draws dark thick). `family`: `"lines"` along `angle` degrees (0 when left out), `"waves"` (needs `wavelength` and `amplitude`, both in rows, that is in multiples of `spacing`, as a halftone's are in pitches; a sine in phase at the origin), `"rings"` round `center` (default the middle of the region's box), `"spiral"` (arms `spacing` apart along any ray from the center, turned by `angle`) and `"rays"` (as many as are `spacing` apart at the box's farthest corner, so they crowd near the center: use a narrow width). A ring wholly inside is two loops, the inner the other way. An argument the family does not read (`wavelength` or `amplitude` but for waves, `center` for lines and waves, `angle` for rings) is a `ParameterError` naming it and the family, in a call, in Rust and in a document. |
| `stipple(spacing, *, tone, radius, density=None, seed=None)` | closed round dots, for filling | one dot at most in each cell of a square lattice `spacing` apart; `radius` is one number, or `(at tone 0, at tone 1)` as `density` is (linear, held beyond, read at the dot's center: `(2, 0.5)` grows the dark dots); `density` (default `(1, 0)`) is the share of cells that get a dot where the tone is 0 and where it is 1. Which cells is a low-discrepancy threshold (R2) moved by `seed` (default 0), so the dots of a tone are evenly spread and their count follows the tone to within a cell or two. |
| `Path.within(region)` | open lines | the parts of a path's lines inside the region `region` fills, cut exactly where they cross its outline; a closed loop wholly inside stays closed. A method of a `Path` only, with no document step yet: a drawing author calls it on the `Path` before the drawing is built (`Drawing(lines.within(region))`). |

```python
photo = btvec.Field.map(w, h, grey_bytes, box=(0, 0, 1080, 1440))
dog = matte_field.contours(0.5, box=centers, cell=2, smooth=3, min_length=200)  # the dog's outline, from image's matte
engraving = dog.bands(6, tone=photo, width=(5, 0.4), family="rings", center=nose)
btvec.Drawing(engraving).fill("#15161a")
```

Rust: `Rows::new(spacing, tone, amplitude)?`, `Bands::lines(spacing, tone, (thick, thin))?` (the width at tone 0, which is dark, then at tone 1) (also `waves`, `rings`,
`spiral`, `rays`; `.angle(..)?`, `.center(..)?`, which refuse a family that does not read them), `Stipple::new(spacing, tone, (r0, r1))?` (the radius at tone 0 and at tone 1, `(r, r)` for one size; `.density(a, b)`, `.seed(n)`), then
`path.rows(&rows)?` or `drawing.rows(&rows)`. Errors: `ParameterError` for a spacing not above 0, a `width` below 0, a
`wavelength` not above 0, a `radius` not above 0 (either of a pair), a `density` outside 0 to 1, a `seed` that is not whole, a non-finite number, a carrier of over 2
million points (`bands`: a `center` far from the region stretches rings and arms, and is named), 5 million (`rows`, its
columns alone too) or 2 million cells (`stipple`), more than 10 million points, or `within` weighing over 500 million
edges (naming `region`, or `spacing` inside `bands`). A region that fills nothing (a point, a lone line) gives an empty
path in all three. A drawing in the
tone's `Field` is taken by the step like `hatch`'s. [line_families.py](../examples/line_families.py) draws a hill of each.

## Trace

A map (`Field.map`: 8-bit bytes, the same input form in documents) traced into filled shapes; `trace_levels` traces any field. Three sources, all on `Drawing`;
the shapes carry no paint (paint each with `only(level=..).fill(..)`, or `only(label=..).fill(..)` for labels and cells): a level shape has its level as the `level` attribute, a label or cell shape its colour byte as the `label` attribute (a whole number).

| Call | Result | Arguments |
|---|---|---|
| `Drawing.trace_levels(map, levels, *, box=None, cell=None, smooth=0, tolerance=0.5, open=0, close=0, min_area=0, min_hole=0)` | one shape for each of `levels` (a number or a list; a map's are 0 black to 1 white) the field reaches, in the order given: the region where it is at or above it. `map` is any `Field`, read at the centers of `box` `(x, y, width, height)` cut into equal cells no wider than `cell` pixels; `None` is a `Field.map`'s own box and one map pixel (its bytes, traced as they are), and any other field needs both. A field reads its Params and drawings with the values of the draw | `smooth` is the standard deviation, in design pixels, of a Gaussian that blurs the map first; `close`, then `open` (design pixels) fill and remove what is narrower than twice the radius; `min_hole` and `min_area` (square pixels) fill small holes and drop small parts; `tolerance` fits curves within that many design pixels of the traced polyline (0 keeps it). `levels` and each number is a number, a `Param` or an expression, read once |
| `Drawing.trace_labels(map, *, max_colors=32, ...)` | one shape for each byte the map holds, attribute `label` the byte; the shapes tile the frame and its padding: no overlap, no gap, whatever the tidying | the tidying of `trace_levels`. The commonest label (the lowest byte of a tie) comes first and is whatever the others leave; the others follow in order of the byte, each cut by the labels after it, so a tidied shape is a polyline within 0.01 pixels of its curve. Cutting too many outlines apart is refused naming `max_colors` |
| `Drawing.trace_cells(map, cell, *, max_colors=32)` | one shape (one outline with all its regions) for each colour, in order of the byte, attribute `label`, exactly | the map is cells of `cell` map pixels, each one byte (a dithered picture scaled by `cell`); cells of one colour that touch are merged, edges lie on cell sides, no fitting; areas are the cells' and the shapes tile the map |

A shape that meets the map's border goes on past it and closes outside the frame, so no edge lies along the border. The
overhang is the larger of `open` and `close` plus `tolerance` (in map pixels, rounded up), plus two map pixels. For a plotter
or a cutter crop it to the frame in one call, `drawing.intersect(btvec.Path.rect(0, 0, width, height))` (the labels still
tile); a view crops it for the screen. The map's `box` places the map. Rust: `Drawing::trace_levels(&map, [0.3, 0.6], &Trace::new().smooth(1.5).open(2.0))`
(a field that is not a map: `&Trace::new().frame(rect).cell(2.0)`; `trace_labels` refuses either)
(`levels` is any list of numbers, Params or expressions), `Drawing::trace_labels(&map, 8, &Trace::new())`,
`Drawing::trace_cells(&map, 2, 8)`; in a document
`{"trace_levels": {"map": {"Field.map": {...}}, "levels": [0.3, 0.6], "open": 2}}`.

```python
photo = btvec.Field.map(w, h, grey_bytes, box=(0, 0, 1080, 1440))
layers = [
    btvec.Drawing.trace_levels(photo, l, smooth=4, close=6, min_area=300).fill(c)
    for l, c in zip([0.2, 0.45, 0.7], ["#e9dcc3", "#c9a678", "#8a5a3b"])
]
print(btvec.Drawing(*layers, size=(1080, 1440)).to_svg(1080, 1440))
```

Errors: `ParameterError` naming `levels` (none, over 256, not finite, or passes times lattice nodes over 2e8; for labels
`max_colors` names the passes), `smooth`, `tolerance`, `open`, `close`, `min_area`, `min_hole` (below 0, not finite or
over 1e100; the largest of `open`, `close` and `tolerance` also when the padded lattice passes 2^25 nodes (a 4096 by 4096
map holds hundreds of pixels of padding), `smooth` also when the blur passes 2e9 steps), `max_colors` (not a whole number from
1 to 256, fewer than the map's colours, or labels whose outlines are too many to cut apart within the draw's segment
budget), `cell` (`trace_cells`: not a whole number from 1, not dividing the map, or a cell of more than one byte;
`trace_levels`: not above 0, left out for a field that is not a map, a grid of over 2^24 cells or reads past the work
limit), `box` (left out for a field that is not a map, or as for `Field.map`) and `map` (`trace_labels`, `trace_cells`: not a
`Field.map`). From Python a count that is not whole (2.5) or below 0 raises `ParameterError` naming it.
[trace.py](../examples/trace.py) traces a relief and a dithered strip; [eaten_letters.py](../examples/eaten_letters.py) a
word eaten at the rim by a noisy distance field (noise of 14 pixels on the edge bites the letters):

```python
edge = btvec.Field.distance(word)  # negative inside
eaten = edge + btvec.Field.noise(wavelength=45, seed=3, octaves=3) * 40 * (abs(edge).min(60) * (1 / 60)).max(0.35)
btvec.Drawing.trace_levels(-eaten, [-40, -18, 0], box=(0, 0, 1200, 600), cell=2, smooth=1.5)
```

### A sequence of maps (V14b)

The library keeps no sequence: the caller holds a list of `Field.map`s (frames of a video, one size and box) and makes it
steady before tracing each. `Field.smooth_maps` takes the whole list and gives a list of unnamed `Field.map`s, so every verb above
reads the result, and a document names it as a map input like any other (there is no document verb and no JavaScript
call: the browser package loads documents, it does not build fields).

| Call | Does | Refuses |
|---|---|---|
| `Field.smooth_maps(maps, *, sigma)` (Rust: `Field::smooth_maps(&maps, sigma)?`) | each map becomes the Gaussian weighted mean of itself and its neighbours, `sigma` in maps: frame `j` weighs `exp(-(i-j)^2 / 2 sigma^2)` for `|i-j|` up to `ceil(3 sigma)`; the weights of the frames that exist add to 1 (the ends are not padded); bytes rounded; `sigma` 0, a `sigma` too small for any neighbour to weigh above 0, and a static sequence give the maps back | `maps` (not all `Field.map`s of one size and box, or over 2^28 pixels together); `sigma` (below 0, not finite, or the work for one output frame, the maps it reaches times pixels, over 2e8: a clip costs its frame count times that, so 30 frames of 1920 by 1080 at `sigma` 1.5 run in one call) |

```python
maps = [btvec.Field.map(w, h, frame, box=(0, 0, 1080, 1440)) for frame in frames]
steady = btvec.Field.smooth_maps(maps, sigma=1.5)
shapes = [btvec.Drawing.trace_levels(m, [0.3, 0.6], smooth=4, close=6) for m in steady]
```

Limits, measured and by design: the ends of a sequence average fewer frames, so they are the least steady; a hole or
speck that lasts across frames stays (`min_area` and `min_hole` drop small ones); smoothing in time also lags a fast
motion. [trace_sequence.py](../examples/trace_sequence.py) and
[.rs](../crates/core/examples/trace_sequence.rs) trace five noisy frames alone and steadied.

## Drawing

`Drawing(*items, size=None)`: the items (paths, shapes, drawings) stacked in order, the first at the bottom. `size` is
the design frame `(width, height)` in design pixels; outputs scale it to their size. Without one, a drawing takes the
size its items have; items whose sizes differ raise `ParameterError` naming both; with none at all, design pixels are
output pixels. A size is above 0 and at most 100,000 a side; others raise `ParameterError`. `Drawing(x)` of one item and no size draws and saves exactly as `x` does (a path becomes a one-shape
`Drawing`; a drawing is returned as it is).

### SVG import

```python
logo = btvec.Drawing.from_svg(svg_text)  # a shape per path, rect, circle, ellipse, line, polyline, polygon
ring = logo.only(id="ring")  # the part the SVG calls "ring"
lines = logo.only(**{"class": "outline"})  # `class` is a Python keyword: pass it in a dict
```

`Drawing.from_svg(text)` reads one SVG document. Each shape has `index`, `order` (as any stack writes it) and, where the
element has them, `id`, `class` (as written) and `group`: the ids of the enclosing `g` elements, outermost first, joined
by `/` (`"logo/mark"`; a `g` without an id adds nothing). `only(group="logo")` matches the whole text, so a shape in
`logo/mark` is not in it; list the paths wanted (`group=["logo", "logo/mark"]`). `class` is the attribute's whole
text too: `class="st0 st1"` is selected by `"st0 st1"`, not by `"st0"` (selecting by one token is a follow-up). `class` is a Python
keyword, so write `drawing.only(**{"class": "st0"})`; `class_=` is not an attribute and raises. The `viewBox` is the drawing's size
(without one, `width` and `height` in pixels) and its corner becomes the origin. `transform` (`matrix`, `translate`,
`scale`, `rotate`, `skewX`, `skewY`) is worked into the points. `fill` (black unless set), `fill-opacity`, `stroke` (none
unless set), `stroke-opacity`, `stroke-width` (1 unless set, scaled by the transform), `stroke-linejoin`,
`stroke-linecap` and `stroke-miterlimit` come from attributes or `style` and pass down groups; `opacity` on shapes.
Colours: `#rgb`, `#rrggbb`, `#rrggbbaa`, `none`. Lengths: numbers, pixels.

Every imported shape has a fill, so `.stroke(c)` alone leaves it showing. To outline everything, clear the fill first:
`logo.fill("none").stroke(c)`. To keep some fills, select those out with `only` and join the rest: `btvec.Drawing(logo.only(id="ring"),
logo.without(id="ring").fill("none").stroke(c))`.

Skipped: `title`, `desc`, `metadata`, comments, an empty `defs`, elements of other namespaces (an editor's own), and
attributes the importer has no use for (`data-*`, `aria-*`, `isolation`, `shape-rendering`, `width` and `height` next to a
`viewBox`). Every attribute and `style` property that changes how a shape is drawn and is not read is refused (below).

Anything else that changes the picture is a `ParameterError` that names the element or attribute and what to do, never
dropped. The first version's limits:

| Not read | Why and what to do |
|---|---|
| `fill-rule="evenodd"` | the renderer fills nonzero only: split the shape into separate paths that need no evenodd |
| `use`, `symbol`, a `defs` that holds anything | write the shape out where it is used |
| gradients and patterns (`fill="url(#g)"`) | paint with one colour |
| `clip-path`, `mask`, `filter`, markers, dashes, `paint-order`, `vector-effect`, `transform-origin` (write `rotate(angle x y)`), `display="none"`, blend modes, a `transform` inside `style` (write the attribute) | delete them or draw the effect with the library's own steps |
| `text`, `image`, `style` elements and `<?xml-stylesheet?>` (CSS stylesheets) | convert text to outlines; set fill and stroke on the elements |
| a `<!DOCTYPE>` line (an Illustrator header has one) | remove the line: entity definitions can expand to gigabytes, and the parser cannot cap that |
| nested `svg`, `opacity` on a `g` | a group's opacity fades it as one picture; set `opacity` on the shapes |
| colour names, `rgb()`, `currentColor`; lengths in percent or units other than pixels | write `#rrggbb` and pixels |
| a stroke under a transform that stretches one way or skews | a stroke cannot follow it; draw it without the transform |
| `rx` unequal to `ry` | only round corners |

Past `MAX_SVG_ELEMENTS` (20,000 elements, the root included), `MAX_SVG_DEPTH` (128 levels, the root being 1) or
`MAX_DOCUMENT_BYTES` (16 MiB) it raises as well. Errors: `ParameterError` naming `svg`.

### Items

| Step | Signature | Promise | Errors |
|---|---|---|---|
| `split` | `split()` | every shape split into its parts (as `Path.split`), each a shape of its own keeping the attributes and style, with `part` (from 0, within its shape) and its own bounds center as `origin`, replacing one the shape carried | |
| `merge` | `merge(*, by=None)` | shapes with an equal attribute `by` (all shapes, if `None`) joined into one: their sub-paths in list order, not united (overlaps stay; `union` or `resolve` unites them), with the first's attributes and style, at the first's place; groups in the order their first members appear; shapes that lack `by` are a group | `by` that is no attribute name; `by` that no shape of the list carries (when drawn) |
| `only` | `only(**attributes)` | the shapes whose attributes equal these (a list means any of; `class` goes in a dict, `only(**{"class": "c"})`) | an attribute no shape carries |
| `without` | `without(**attributes)` | the other shapes | as `only` |

`index` is every shape's place in the current list. Generators write the rest: `text` writes `glyph`, `char`, `word`,
`line`, `order` and a typographic `origin`; `trace` writes `level`; `Drawing.from_svg` writes the text `id`, `class` and `group` ("SVG import"); `Drawing(...)` writes `order` on the shapes that carry none.
`order` runs 0..1 over the shapes the step that writes it writes it on, in list order (a single shape has 0; a stack
spreads it over its shapes that carry none, so a text keeps its per-letter order beside a rectangle) and survives
`split` and `only`; `index`
counts the current list. A shape whose path becomes empty stays in the list and draws nothing, so `index`, `order` and
per-item lists keep their meaning; only `only`, `without` and `merge` remove shapes. A shape with nothing in it that
`split` meets stays, with `part` 0. Rust: `drawing.split()`, `drawing.merge(Merge::new())` and `drawing.merge(Merge::new().by("word"))`.

### Style

| Step | Signature | Promise | Errors |
|---|---|---|---|
| `fill` | `fill(color)` | fills each shape with a colour or a gradient; `"none"` for no fill | an unreadable colour |
| `stroke` | `stroke(color, *, width=2, join="round", cap="round", miter_limit=4, profile=None)` | strokes each outline with a colour or a gradient; with a `profile` (a list of `[position, factor]` pairs as `outline` takes, "Tapered strokes" above) the width follows it along each sub-path and the stroke is drawn as a filled region by SVG, pixels and mesh, `width` and each number of the profile per-item numbers (Rust: `Stroke::new(c).width(24).profile([(0, 0), (0.15, 1), (0.85, 1), (1, 0)])`; `Style.profile` gives it back) | as `fill`; a paint of `"none"`; `width` not above 0; `miter_limit` below 1; with a `profile`: as `outline`'s (at the step: `join` or `cap` not `"round"`, no stops or over `MAX_PROFILE_STOPS`, a position outside 0..1 or going back, a factor outside 0..1e100), and when drawn a region over `MAX_SEGMENTS` |
| `linear_gradient` | `linear_gradient(stops, *, start, end)` | a paint for `fill` and `stroke`: `stops` are `(offset, color)`, the colour along the line from `start` to `end`, held at the end colours beyond it | fewer than 2 stops; an offset outside 0..1 or below the one before; a stop colour of `"none"`; `start` equal to `end`; a coordinate beyond ±1e100 |
| `radial_gradient` | `radial_gradient(stops, *, center, radius)` | a paint: offset 0 at `center`, offset 1 at `radius`, held beyond | as `linear_gradient`; `radius` not above 0 |
| `halftone` | `halftone(color, *, tone, pitch=8, angle=None, shape="dot", aspect=None, screen="grid", center=None, amplitude=None, wavelength=None)` | fills each shape with a screen of dots, one in each cell of a layout `pitch` pixels apart; a dot covers `1 - tone` of its own cell (0 solid, 1 none): `tone` is a number (a Param, a number for each shape) or a `Field`, read at the dot's center; `shape` is `"dot"`, `"square"`, `"ellipse"` (`aspect` times as long as wide), `"diamond"`, `"euclidean"` or a `Path`; `screen` is `"grid"` (turned by `angle`), `"wave"` (rows swung `amplitude` pitches over `wavelength` pitches), `"rings"`, `"spiral"` or `"rays"` (about `center`); an argument left out takes its default (`angle` 45, `aspect` 1, `amplitude` 1, `wavelength` 6, `center` the middle of the shape's box), and one given that nothing reads is refused as `bands` refuses it: `aspect` for any shape but an ellipse, `angle` for rings, `center` for a grid or a wave, `amplitude` and `wavelength` for any screen but a wave | `color` of `"none"`; `tone` outside 0..1; `pitch` not above 0; `angle` not finite; an unknown `shape` or `screen` (the valid ones are listed); `aspect` below 1; an argument nothing reads (above); a `wavelength` not above 0; a `center` too far from the shape for a pitch to move there; a `Path` that is not one closed outline with an area (`shape`); the limit below |
| `opacity` | `opacity(alpha)` | multiplies opacity by `alpha` 0..1 | `alpha` outside 0..1 |
| `blend` | `blend(mode)` | paints over what is below with a CSS blend `mode`: `"normal"`, `"multiply"`, `"screen"`, `"overlay"`, `"darken"`, `"lighten"`, `"color-dodge"`, `"color-burn"`, `"hard-light"`, `"soft-light"`, `"difference"` or `"exclusion"`; it applies to a shape with its shadow once | another word, the non-separable `"hue"`, `"saturation"`, `"color"` and `"luminosity"` included; a mesh: any mode but normal and multiply |
| `shadow` | `shadow(*, dx=0, dy=8, blur=16, color="#00000059")` | a drop shadow under each shape: the silhouette of its fill and stroke, moved by `dx` and `dy`, blurred and tinted `color`; the last call wins | `blur` below 0; `dx`, `dy` or `blur` not finite; `color` of `"none"`; pixel outputs: a blur past the limits below |
| `knockout` | `knockout()` | each shape removes what is below it in its drawing, instead of painting: the area its fill and stroke cover is cut out of the shapes before it, shadows included | `to_svg`: more than 128 knockout layers nested (below) |
| `clip` | `clip(region)` | cuts each shape, and its shadow, to the region the drawing `region` (a `Drawing`, `Path` or `Shape`) fills: all its shapes united once, whichever way round their outlines run; each shape is cut on its own, as `opacity` applies to it, not as a group of the drawing; a region that fills nothing leaves nothing drawn; a second clip cuts to both; a knockout is not cut | a `region` that holds an error; the work of uniting it past `MAX_SEGMENTS`; a mesh |
| `mask` | `mask(source)` | fades each shape, and its shadow, by the luminance of `source` as painted (gradients, blends and opacity included; 0.2126 R + 0.7152 G + 0.0722 B of the sRGB values, times the alpha): white keeps, black and what `source` leaves bare hide; each shape on its own; a knockout is not masked | a shape that is masked already, or a `source` whose shapes are masked (naming `source`); the canvas pixels of the masks of a draw past `MAX_FILTER_PIXELS`; a mesh |

Colours are `"#rgb"`, `"#rrggbb"` or `"#rrggbbaa"`. A gradient is a value (Rust `Gradient::linear(start, end, stops)?` and
`Gradient::radial(center, radius, stops)?`, stops `(f64, Color)`; Python `Gradient`, `Style.fill` and `Style.stroke`
give back the text or the `Gradient`; Rust `Style::fill()` and `StrokeStyle::paint()` give a `Paint`, a `Color` or a
`Gradient`). Its numbers are literals (no Param) in design pixels of the frame, not of the shape: a shape moved under a
gradient shows another part of it, so set the paint after the transforms. Stops are interpolated in sRGB with the alpha
not premultiplied (as SVG does), colour and alpha apart; there is no repeat, reflect or focal point. A `fill` of a
colour after a gradient replaces it, and the other way round. `to_svg` writes a `<linearGradient>` or `<radialGradient>`
in user space with the id `gradient-N-fill` or `gradient-N-stroke` for shape `N`, before the path that uses it. A shadow
follows the paint's alpha, as it follows a translucent colour's: blurred it is the group's alpha through the filter;
blur 0 is a copy under a gradient of its own in the tint with the stops' alphas (`gradient-N-shadow-fill`). A shape that
knocks out ignores its paint, a gradient too. `to_mesh` raises `ParameterError` naming `fill` or `stroke` for a gradient
(no triangle form: a colour per vertex would not be the colour at a pixel); use `to_rgba` or `to_svg`. Until a fill or a stroke is set, a closed outline is filled black and an open
line stroked black at 2 pixels; once either is set, only what is set is painted. `fill("none")` counts as set.
`shadow`'s `dx`, `dy` and `blur` are design pixels (per-item numbers) and scale with the output like a stroke width.
`blur` is the radius of a CSS `box-shadow`: the Gaussian's standard deviation is half of it, so the shadow of a straight edge is at half
strength where the edge was. The shadow is painted behind its shape, inside the group the shape's
`opacity` and blend apply to once (shadow and shape are no darker where they overlap), and a translucent fill makes a
translucent shadow (the silhouette's alpha times the colour's). A shape with an empty path, or that paints nothing, casts
none. `blur=0` is a hard copy: `to_svg` writes a tinted copy of the path, no filter. Blurred, `to_svg` writes a `<filter>`
(a Gaussian blur of the alpha, offset, flooded with the colour, merged under the shape) on a `<g>`, with a region of the
canvas grown by the shadow's reach (3 deviations and the offset), `color-interpolation-filters="sRGB"` and the id
`shadow-N` for shape `N`; it renders alike in `resvg`, which the tests compare with the pixels. Limits, for pixel outputs
(`to_svg` is bounded only by what a coordinate holds, 1e100): the deviation, `blur / 2` times the scale of the output, is
at most 1000 output pixels, else `ParameterError` naming `blur`; the layers that blurs work in (each the shape's part on
the canvas, grown by the reach) total at most 2^27 pixels in a draw, else `ParameterError` naming the shape and
`MAX_FILTER_PIXELS`. The pixels' blur is an 8-bit filter: within 2 to 4 levels of the Gaussian from a deviation of 1
output pixel up, and not a Gaussian below it (up to 22 levels off at 0.3 pixel); that is not worked around.
`halftone` is a fill of its own: it replaces a `fill`, and a `fill` after it replaces the screen (a shape has one fill, and
`Style.fill` is `None` while it is a screen, `Style.halftone` has everything the step was given: `color`, `tone`, `pitch`,
`angle`, `shape`, `aspect`, `screen`, `center`, `amplitude`, `wavelength`; in Rust `HalftoneStyle`, whose `tone()` is a
`Tone`, `Level(f64)` or `Field`). `Style.halftone.tone` is the number, or the `Field` the step was given (the same field
everywhere, as drawn: a Param in it is already read); `shape` is the word, or the `Path` the caller gave.
A screen is a layout of cells with a dot in each, and a dot covers `1 - tone` of the area of *its own cell*, for every shape
and every screen. The cells are the rows `bands` lays lines along (one layout module serves both):
- `"grid"`: rows `pitch` apart, a cell a square of the pitch, turned `angle` degrees clockwise on the screen about the origin
  of the design frame, a dot on each lattice point: anchored to the frame and not to the shape, so the dots stay put when a
  shape moves and neighbouring shapes share one screen. `pitch` is in design pixels and is not scaled by `scale` (like a
  stroke width).
- `"wave"`: the grid, each row swung across by `amplitude` pitches as a sine of `wavelength` pitches along it (in phase at
  the origin); the cell is the grid's cell moved with its row, so its area is the pitch squared and the dot is not tilted.
- `"rings"`: rings about `center` (default the middle of the shape's bounding box), ring `k` at radius `(k + 1/2)` pitches,
  a cell the wedge of a ring, `2 round(pi (k + 1/2))` to the ring; `angle` does nothing.
- `"spiral"`: one row, an Archimedean spiral out of `center` a pitch further out each turn, the first arm at `angle`; the
  cell is a wedge of an arm, `2 round(pi (k + 1))` to the turn of arm `k`.
- `"rays"`: rows straight out of `center` starting at `angle`, a cell a pitch long: as many rays as a ring half way to the
  nearest side of the shape's box has cells (`2 round(pi r)` for `r` pitches, as bullterrier-image counts them), merged in pairs
  (a power of two at a time) towards the center so that no cell is narrower than half a pitch at its middle, the last of a
  row narrower when the rays do not divide evenly; a cell is a wedge, wider outward.
`shape` is the dot: `"dot"` (a regular polygon of as many corners as keep it within 0.01 design pixels of the circle),
`"square"` (its sides along the rows), `"ellipse"` (its long axis along the rows, `aspect` times its short one, `aspect` at
least 1), `"diamond"` (a square turned 45 degrees to the rows: its corners on the axes of the row, touching its neighbours at
tone 0.5, past it the cell with its corners cut), `"euclidean"` (the cosine spot function of print screens: round at low cover,
a diamond at tone 0.5, past it the cell less four round holes at its corners; 128 corners) or a `Path`: the first and only
closed sub-path of the caller's outline, centered on its centroid (the center of its area), scaled to the dot's area, its x
axis along the row and its winding made clockwise on the screen whichever way it was drawn. Image's `line` and `matrix` shapes
are not here: `bands` draws lines along these same layouts, and a caller's `Path` is the matrix. The area of each dot is exact:
the corners of a round shape are solved for the polygon's own area, not taken from the circle. A dot is cut to the shape's
outline. A round mark (dot, ellipse, a caller's outline) of a tone below `1 - pi / 4` (about 0.21) makes neighbours overlap,
where the union covers less than `(1 - tone)` of a cell; the square, the diamond and the euclidean mark stay in their cell.
Their colour's alpha is that of every dot, once where dots overlap. A tone of the number 0 is the solid fill of the colour
(circles cannot cover a cell's corners), a tone of 1 draws nothing; a field is read at each dot's center and held to 0 to 1, a
dot of tone 1 is left out and one of tone 0 fills its cell as its shape does. A stroke is
drawn over the dots; opacity, blend and shadow apply to dots and stroke together, and a shadow is the dots moved and tinted.
`to_plot_svg` draws a halftone as its dots hatched at the pen width, like any solid (the dots wholly inside as they are, those
that cross the outline cut to it by one boolean), from the same screen the other outputs draw.
`to_svg` writes a `<clipPath id="halftone-N">` of the shape's outline and a `<g clip-path>` holding all the dots as one
`<path>`, then the stroke; pixels clip one path of the dots to the outline; neither makes a boolean per dot (the
prototypes' did, about 500 ms for 3,000 dots; [benchmarks.md](benchmarks.md)). `to_mesh` makes the dots wholly inside the
shape plain polygons, leaves out those wholly outside and cuts only the dots that cross the outline, all together by one
boolean. Limit: before any dot is made, the cells near the shape (an upper bound worked out from the layout: for rings,
a spiral and rays, the annulus the shape's box and the farthest dot reach), each times the corners of a dot, are charged to
`MAX_SEGMENTS`, and each dot is placed against the edges of the outline near it (a grid of them), charged as they are
weighed, so a shape of many edges (a frame with a headline cut out) takes the pitches its plain box takes; a pitch too small for its shape (about 50,000 dots for a disc) or an
`aspect` that would make a dot a thousand pitches long is refused with `ParameterError` naming the shape and its `pitch`, on
every screen and in every output; a field tone is also charged to the field's read limit. A center far from the shape
stretches its rings and rays out: it costs cells like a small pitch does.
The layout is laid over the whole shape, not the part on the canvas: a shape far larger than the canvas (a 20,000 by
20,000 square at pitch 8) is refused though few of its dots would show; use a larger pitch or a smaller shape.
`knockout` is the style form of a cut: it keeps both shapes and cuts through everything below, shadows included;
`subtract` makes new geometry (for a plotter, or for further verbs). Every style reaches SVG and pixels alike. Called on
a `Path`, a style step returns a `Drawing` of that path.

A shape that knocks out paints nothing itself. What it removes is the area its fill and stroke cover (a fill or stroke
that is not `"none"`), and only from the shapes before it in paint order: those after it paint over the hole. Its
colours and their alphas, its opacity, its blend and its shadow are not drawn and do not matter: a knockout is by
coverage, so an anti-aliased edge removes the share of a pixel it covers, as any edge does. A shape that paints nothing,
or has an empty path, removes nothing. `to_svg` wraps everything written before a run of knockouts in a
`<g mask="url(#knockout-N)">`, `N` the number of the first knockout of the run; the `<mask>` is the canvas, grown by the
farthest a shadow in the group reaches, in white, with each knockout's fill and stroke in black
(`maskUnits="userSpaceOnUse"`), and renders alike in `resvg`, which the tests compare with the pixels. Knockouts in a row
share one mask; a shape between two puts the first inside the second's group, so a multiply after a knockout still
blends with what is left below, as in the pixels. Limits: at most 128 groups nest (`ParameterError` naming the
knockout's shape and `MAX_KNOCKOUT_LAYERS`), because common XML readers refuse a document nested more than 256 levels;
the part of an earlier shape outside the mask's region (the canvas and its shadows' reach) is cut off, which shows only
where the SVG is displayed with `overflow:visible`. Pixels paint the knockout into a layer that is taken out of what is
below it, within the part of the canvas it reaches, with the same anti-aliasing as the SVG.

A clip and a mask belong to the group a shape's opacity and blend do, outermost: a shadow is cut or faded with its shape, and
a blend mixes with what is below, as the pixels do. A knockout ignores both, as it ignores its colours. The region and the
mask lie in the design frame, as a gradient does (set them after the transforms). `clip` and `mask` make the region
and the mask once for the step and every shape it cuts shares them, so a headline of fifty letters faded by one mask has one
`<mask id="mask-N">` (the canvas, with the mask's shapes drawn in it) that each letter's element names, one luminance mask in
the pixels, and its canvas pixels charged once to `MAX_FILTER_PIXELS`, the budget of the blur's layers (two masks of a
8,200 by 8,200 canvas pass it). A `<clipPath id="clip-N">` is written once the same way. What a mask draws has ids with
the mask's own prefix (`mask-3-shadow-0`), so shapes numbered alike never share one. A mesh refuses a clip and a mask,
naming `clip` or `mask` and the shape: three flat buffers have no clip or mask range. The pixels' luminance is rounded by
`vello_cpu` and `resvg`'s is not: they differ by about a level, within the tolerance of the SVG tests
([design-decisions.md](design-decisions.md), "Paint and compositing style (V10d)").

### Values and evaluation

| Method | Signature | Promise | Errors |
|---|---|---|---|
| `with_values` | `with_values(**values)` | the same drawing with these Params fixed; they leave `parameters()` | as outputs (below) |
| `parameters` | `parameters()` | every `Param` the drawing reads, in the order first used (the walk of [formats.md](formats.md), "Params") | two Params with one name that differ in a field (Params equal in all six fields are one) |
| `shapes` | `shapes(**values)` | the evaluated shapes, in design pixels | as outputs |
| `bounds` | `bounds(**values)` | the bounds of the geometry: the smallest `Rect` round every shape drawn (the union of their path bounds); not the painted extent, since strokes, shadows and effects reach past it. A shape with an empty path (an SVG `<path d=""/>`) has no bounds | as outputs; `GeometryError`: no shape drawn |
| `size` | property | the design frame, or `None` | |

## Shape

`drawing.shapes(**values)` returns `list[Shape]`: every shape the drawing evaluates to, in paint order (the bottom
first), after every step, in design pixels. A shape is a read-only value; `Drawing(*shapes)` puts shapes back into a
drawing with their style and attributes.

| Member | Type | Promise |
|---|---|---|
| `shape.path` | `Path` | its outline |
| `shape.style` | `Style` | the style as drawn: `fill` and `stroke` (`"#rrggbbaa"`, a `Gradient`, or `None` where nothing is painted; the default paint is resolved, so an unstyled closed outline has a black `fill`), `stroke_width`, `join`, `cap`, `miter_limit`, `halftone` (its colour, tone, pitch, angle, shape, aspect, screen, center, amplitude and wavelength, or `None`), `opacity`, `blend`, `shadow` (its `dx`, `dy`, `blur` and colour, or `None`), `knockout` (`bool`), `clip` (a `Path` or `None`) and `mask` (the mask's shapes, a list of `Shape`, or `None`) |
| `shape.attributes` | `Mapping[str, int \| float \| str \| Point]` | `index` (`int`) always; then what generators wrote: `glyph` (`int`), `char` (`str`), `word` (`int`), `line` (`int`), `run` (`int`), `part` (`int`), `copy` (`int`), `root` and `forks` (`int`, of `branch` threads), `label` (`int`), `row` and `column` (`int`, of a character grid), `mark` (`int`) and `role` (`str`) of overlay marks, `level` (`float`), `order` (`float`), `origin` (`Point`); from `Drawing.from_svg`: `id`, `class`, `group` (`str`) |

## Generators

Functions of the module that return a `Drawing`.

### text

`text(content, *, font=None, size=100, axes={}, features={}, tracking=0, leading=None, align="left", anchor=(0, 0),
width=None, fit=None, inside=None)`: one shape per drawn glyph (a space or a line break gives none), shaped with
kerning, ligatures and complex scripts, each outline the font's own contours (they may overlap and fill nonzero;
`resolve` removes the overlaps, at the cost of a step at every draw).

| Argument | Meaning |
|---|---|
| `content` | the text; `"\n"` breaks a line; or a list of `run`s (below) |
| `font` | a `Font`; none with runs, which bring their own |
| `size` | pixels to the em |
| `axes` | variable axis values by tag (`{"wght": weight}`): a number, a `Param` or an expression in the font's own units, each in the font's range; a per-item number gives each character its own, and advances follow (below) |
| `features` | OpenType features by tag (`{"ss01": 1, "liga": 0}`): literal whole numbers from 0 to 4,294,967,295, not expressions; at most 64 axes and 64 features (`MAX_SETTINGS`) |
| `tracking` | extra space between glyphs, in thousandths of an em; none between a base and its marks. It breaks the joins of cursive scripts (Arabic) until a script-aware rule exists: a known limit |
| `leading` | pixels between baselines; default 1.2 × `size` |
| `align` | `"left"`, `"center"` or `"right"` |
| `anchor` | where the first baseline meets the alignment: its left end, middle or right end |
| `width` | pixels, above 0: each `"\n"`-separated part wraps to this width, breaking at whitespace. Read as drawn (a `Param` moves the breaks); without `fit` the lines keep the widths the wrap gives them |
| `fit` | an axis tag or `"size"`, needs `width`: each line is made `width` wide, once, at the reference pose (every Param at its default, whatever `with_values` fixed), so the fit never changes from frame to frame and live values may overflow |
| `inside` | a `Path` (a literal): the text is set in the region it fills instead (nonzero rule, as `contains`); takes its lines and place from the shape, so no `width`, `fit` or `anchor` |

Built so far: every argument above, in Python as above (`anchor` takes plain numbers, a `Point` or a tuple, a `features`
value is a Python `int`, `inside` is a `Path`) and in Rust as
`text(content, Text::new(&font).size(..).axes([("wght", &weight)]).features([("liga", 0)]).width(980.0).fit("wdth"))` and
`Text::inside(&shape)` (V5a to V6); JavaScript has no builders, a text arrives in a document (`Drawing.fromJson`, below).
Each line is shaped as one run in the script and direction of its first strong character (no
reordering inside a line), or as several where the axis values change between characters; `glyph` counts the shapes
made, `word` the runs of characters between breaks over the whole text (a no-break space is part of a word, as in a
wrap), `line` the lines, `run` the run (0 for a plain text), and `origin` is the middle of the advance at half the cap
height above the baseline.

**Runs.** `run(content, *, font, size=None, axes={}, features={}, tracking=None)` (Rust `Run::new(content,
&font).size(..).axes(..).features(..).tracking(..)`) is a stretch of a text with a font of its own; `text([run(..),
run(..)], width=900)` (Rust `text([Run::new(..), ..], Text::default().width(900.0))`) sets them as one text. A run takes
the `size` and `tracking` of the text unless it gives its own; `font`, `axes` and `features` are the run's, so the text
gives none of them. A plain `text("x", font=f)` is the text of one run. `examples/ransom_note.py` sets a headline letter
by letter in three typefaces. What follows is the same for runs as for a plain text, except where it says so:
- Each run is shaped apart: kerning, ligatures and joins do not reach across the boundary between two runs (as across a
  change of axis value). A word that spans runs is one word: wrapping breaks at the whitespace of the joined text and
  never inside it, and `word` counts it once.
- A line takes the leading of its largest run, 1.2 times that size, unless `leading` is given; with runs of one size the
  lines stand `n × leading` below the first, else each stands its own leading below the one before. `inside` plans its
  lines with the largest size and the tallest capital of any run.
- `fit="size"` is one factor for every run on a line (the proportions of the runs stay). `fit` on an axis sets each run
  whose font has the axis, at the same fraction of that font's range, and leaves the others as they are; `ParameterError`
  if no run's font has it, or if a run gives the axis in its own `axes`. The error for a width the axes cannot reach
  names the axis value of the first run that has it.
- `glyph`, `word` and `line` count through all the runs; `run` is the index of the glyph's run. A `Param` list for a
  run's `axes` has one entry per character of that run, counted from its first; an error from a run names it (`run 1:`).
- At least one and at most `MAX_RUNS` (1,000) runs; the characters of all runs together at most `MAX_GLYPHS`.

**Wrap.** With `width`, a part breaks greedily at whitespace (`char::is_whitespace`, except U+00A0, U+2007 and U+202F, which are part of a word):
a line takes words while it stays within `width`. The whitespace run at a break is dropped, and so is the whitespace a line ends
in; whitespace that begins a part is kept. A word wider than `width` stands alone and overflows. There is no Unicode
line-break rule: text without spaces (Chinese, Japanese, Thai) does not wrap. The wrap measures a part shaped whole at its
nominal size, so a kern across a break is counted though each line is then shaped alone (a line can differ from its
measure by that kern). `align` places each line against `anchor`, not against `width`; `line` numbers the lines made.
`width` is read as drawn; `size`, `tracking`, `axes`, `features` and `leading`, as far as they decide the lines, are read at
the reference pose, so the breaks move with `width` alone (the glyphs are drawn at the live values).

**Fit.** `fit="size"` sets each line to the size at which it is `width` wide (a line's width is in proportion to its size,
tracking included); `size` still decides where lines break and the spacing of empty lines. Lines then differ in size, so
unless `leading` is given each stands 1.2 times its own size below the baseline before it; uniform text and an explicit
`leading` stand `n × leading` below the first. `fit` on an axis (`"wdth"`) sets that axis for each line: the line is shaped
at the axis's minimum and maximum, `width` must lie between the two widths, and the value is then found by 32 halvings
of the range (the line comes out within about a unit of an advance per glyph of `width`). The width is taken to
change one way along the axis; a width reachable only in the middle of the range is reported as not reachable. An empty
or whitespace-only line is not fitted.

**Inside.** The chord of a baseline is the widest span that lies in the shape both at the baseline and at a capital's
height above it (the leftmost of equals); `align` places a line against its chord. The first capital line is at the top of
the shape's bounds, so the first baseline is a cap height below it; each baseline is then `leading` (default 1.2 ×
`size`) below the one before. A line takes words while they fit its chord; a word that does not fit a baseline's chord
moves to the next baseline, leaving that one empty, and an empty part (`"\n\n"`) takes a baseline of its own. A word wider
than every chord, or text that outlasts the shape, is refused (unlike `width`, which overflows), naming the word and the
line; nothing is cut off. The layout is made once, at the reference pose: a `Param` of `size` or `leading` moves the
glyphs, not the lines. At most `MAX_GLYPHS` baselines are tried. Not tested: right-to-left text inside a shape.

**A value per character.** The items of `axes` are the characters of `content`, every one counted (a space and a
line break too, so `"H\nH"` has three): a `Param` given a list when drawn gives character `i` entry `i`, and the list must
have one entry per character. A change of value between neighbouring characters cuts the line into runs, each shaped
alone at its own location: kerning, ligatures, mark positioning and cursive joins do not cross the change (a ligature
whose letters differ in weight is drawn as separate letters), and a mark whose value differs from its base's leaves it.
`char` is the first character of a glyph's cluster; a ligature of equal-valued characters takes that value. The
`Param`s of `font.axes` are the font's own ranges; an expression may leave them, and the draw is then refused naming
`axes`, the value and the font's range.

Errors (`ParameterError`): an axis or feature the font lacks (the message lists the font's), an axis value outside the
font's range, a `width` not above 0, `fit` without `width`, `fit` that is neither `"size"` nor an axis of the font, `fit`
on an axis also given in `axes`, a line `fit` cannot make `width` wide (the message names the line and the nearest width
the axis reaches, and the axis value there; for `"size"`, a line of no width), `inside` with `width`, `fit` or `anchor`,
`leading` not above 0 with `inside`, an `inside` shape with no outline, text that does not fit the shape, more than
`MAX_GLYPHS` baselines tried; and, for runs, none or more than `MAX_RUNS`, a `font`, `axes` or `features` on the text as
well, or a plain text without a `font`. In Python `content` that is neither a `str` nor a list of `Run` is a `TypeError`.

### characters

`characters(content, *, cell, font=None, size=None, colors=None)`: a grid of characters, one outlined glyph in each cell. Rust:
`characters(content, Characters::new(cell).size(10.0).colors(colors))` (`.font(&font)` for another font).

| Argument | Meaning |
|---|---|
| `content` | the text (named as in `text`): a `str` with `"\n"` between its rows, or a list of `str`, one for each row (Python). A row may be shorter than another |
| `cell` | the width and height of a cell in design pixels: one number for a square cell, or `(width, height)`; above 0 and finite, a literal |
| `font` | a `Font`; default the bundled `jetbrains_mono` in Rust, Python and documents |
| `size` | pixels to the em, a number, `Param` or expression read once; default the largest at which the widest glyph of the grid fits the cell across and the font's line box (ascender to descender) fits it down |
| `colors` | rows of colour texts, one for each character of each row, spaces included: what bullterrier-image's `characters(...).colors` returns, so its output goes in as it is (`btvec.characters(art.text, cell=(6, 12), colors=art.colors)`). Each shape is filled with its colour; without `colors` the shapes have no paint |

One shape for each character that has an outline (a space has none). Every character is set alone by the layout of `text`,
so no kerning and no ligature joins two cells (`->` is two cells although the font has a ligature for it); the glyph is
the outline `text` draws, a character the font lacks is its notdef glyph, as in `text`. A glyph stands with the middle of
its advance on the middle of its cell across and the middle of a capital on the middle of the cell down: capitals center,
and a full stop, a comma or an underscore keep their place on the baseline of their row, as in a printed grid. Attributes:
`char`, `row` and `column` (from 0), `order` (the shape's place among the shapes, 0 to 1) and `origin` (the middle of its
cell, so a rotation or a scale turns each glyph about its own cell); `only(char="#")` picks a character, `only(row=3)` a row.

Errors (`ParameterError`): a `cell` not above 0 or not finite, a `size` not above 0, a control character in `content` (the
message names its row and column), more than `MAX_GLYPHS` characters (refused before anything is drawn), `colors` that do
not match the rows (the message names the row and the counts), a colour that is not `#rgb`, `#rrggbb`, `#rrggbbaa` or
`none`; the outlines are charged to `MAX_SHAPES` and `MAX_OUTLINE_SEGMENTS` before any is made. In Python `content` that is
neither a `str` nor a list of `str` and a `cell` that is not a number or a pair are a `TypeError`.

### marks

`marks(boxes=None, points=None, *, bracket=None, cross=None, text_size=None, font=None)` (`None` is the default: no marks, 0.25, 12, 12, the bundled font): overlay marks in the look of a
heads-up display, in design pixels. Rust: `marks(Marks::new().boxes([BoxMark::new(40.0, 60.0, 120.0, 80.0).text("EAR L")])
.points([PointMark::new(200.0, 90.0).text("NOSE")]).bracket(0.3))` (`.font(&font)` for another font).

| Argument | Meaning |
|---|---|
| `boxes` | boxes to bracket: `(x, y, width, height)` or `(x, y, width, height, text)` (the top left corner; width and height above 0); numbers, `Param`s or expressions, read once |
| `points` | points to mark with a crosshair: `(x, y)` or `(x, y, text)` |
| `bracket` | the length of a bracket's arm, in times the smaller side of its box: above 0, at most 0.5; default 0.25 |
| `cross` | how far each line of a crosshair reaches either side of its point, in pixels: above 0; default 12 |
| `text_size` | pixels to the em of the texts: above 0; default 12 |
| `font` | the texts' font; default the bundled `jetbrains_mono` in Rust, Python and documents |

A box has an open bracket (arm, corner, arm: three points) in each of its four corners. A point has two open lines through
it, each `2 * cross` long. A mark with a `text` (a non-empty one line of printable characters) has a leader, an open line
from the box's top right corner or from the point, up and to the right by `cross + text_size` in each direction, and the
label: the glyphs of `text` at `text_size`, the pen a quarter em right of the leader's end and the middle of a capital on
its height. That fixed rule keeps a label clear of its own box (it lies right of the corner) and of its own crosshair (it
lies beyond its reach); no layout is solved, so labels of marks that lie close can overlap each other. A label is shaped
as a string (a ligature of the font joins its characters).

Every shape has `role` (`"bracket"`, `"cross"`, `"leader"` or `"text"`) and `mark` (which mark it belongs to, from 0: the
boxes in order, then the points), none has a paint: select to style (`marks.only(role="text").fill(c)`,
`marks.without(role="text").stroke(c, width=1.5)`). Brackets, crosshairs and leaders are open lines of two or three points,
so `to_plot_svg` plots them as they are. The label's shapes also carry what `text` writes (`char`, `glyph`, `word`...).
No box and no point is an empty drawing.

Errors (`ParameterError`): a `bracket` not above 0 or past 0.5, a `cross`, `text_size`, `width` or `height` not above 0, a
number that is not finite or lies so far out that a line passes ±1e100, a label with a control character (a line break
included), more than `MAX_MARKS` marks or `MAX_GLYPHS` characters of labels. In Python a box or a point that is not a
tuple of the right length of numbers (and a label) is a `TypeError`.

### trace

Tracing a map is `Drawing.trace_levels`, `Drawing.trace_labels` and `Drawing.trace_cells` (see "Trace" above for their
arguments, errors and the shape attributes they write). For a sequence of maps, make each map steady with
`Field.smooth_maps(maps, *, sigma)` first and trace the result (see "A sequence of maps (V14b)"). The library keeps no
sequence.

To draw contour lines of a field (a distance, a ramp, a map) at levels, use `Field.contours` and stroke the result; `trace_levels` is for the filled regions a field's levels enclose, with curves fitted.

## Inputs

| Input | Signature | Promise | Errors |
|---|---|---|---|
| `Font` | `Font(data, *, name=None)` | a TrueType, OpenType or WOFF2 font, variable or not, from bytes (WOFF2 is unpacked; a saved drawing names the bytes given); `name` is what a saved drawing calls it | `data` not a font the library reads, a WOFF font, a damaged WOFF2 file, a font or an unpacked WOFF2 over 32 MiB |
| `Font.bundled` | `Font.bundled(name)` (Rust: `Font::bundled(name)?`) | a font that comes with the library, called `name` in a saved drawing: `"bricolage_grotesque"`, `"fraunces"` or `"jetbrains_mono"` (a monospaced face, every glyph 0.6 em wide: the default of `characters` and `marks`), SIL OFL 1.1. Its axes are the font file's, at its defaults (`fraunces` opens at `wght` 900, `opsz` 9, `WONK` 1; `bricolage_grotesque` at `wght` 800, `opsz` 96; `jetbrains_mono` at `wght` 400 of 100 to 800). A document that names it by `sha256` loads with no bytes supplied | `name` not bundled (the message lists them) |
| `Font.bundled_names` | `Font.bundled_names()` (Rust: `Font::bundled_names()`) | the names `Font.bundled` takes, in order | |
| `font.axes` | property | its variable axes by tag, each a `Param` named by the tag, with the font's range, default and name, in the font's order (Python: a new dict each time; Rust: `Font::axes()`, a list). Only an axis that can vary and is named by a `Param` name is listed: one pinned (minimum equal to maximum), with a tag that is not a name or a repeat of an earlier tag is left out, and is still set by its tag, and a refusal names it | |
| `font.features` | property | the OpenType feature tags its `GSUB` and `GPOS` tables list, sorted (Rust: `Font::features()`) | |
| `Field.map` | `Field.map(width, height, data, *, box=None, name=None)` | an 8-bit grayscale map, values 0 (black) to 1 (white): `data` is `height` rows of `width` bytes, top row first (Python: `bytes`, a `bytearray`, a `memoryview`, a `numpy` array of `uint8` or any contiguous 8-bit buffer, or a list of whole numbers from 0 to 255; Rust: `Field::map(w, h, &bytes, Some(rect), Some("name"))`). It lies over `box` `(x, y, width, height)` in design units (default: one pixel a map pixel, from the origin); a pixel's value stands at its center, the field is bilinear between centers and holds the nearest edge value beyond the box. A saved drawing names it (an input of kind `map`, with its SHA-256) and the loader supplies the bytes. At most 2^24 pixels | `len(data)` not `width × height`; a size of 0 or past 2^24 pixels; a `box` not finite, past ±1e100 or with no width or height; a `name` that is not letters, digits and `_` |
| `field.to_map` | `to_map(width, height, *, box=None, low=0, high=1)` (Rust: `to_map(w, h, Some(rect), low, high)`) | the field written out as an 8-bit grayscale map, as `bytes` (Rust: `Vec<u8>`), `height` rows of `width` bytes, top row first: each pixel is the field at its center over `box` (default as for `Field.map`), `low` written as 0 and `high` as 255, linear between, held beyond, rounded to the nearest level. It is the input form of `Field.map` and of image's `relight` (`Field.distance(word, normalized=True).to_map(w, h, box=frame)` is a bevel: 0 on the edge, rising inwards); a raw distance in pixels needs a `low` and `high` (`-bevel` to `0`) to keep its range | `width` or `height` of 0 or past 2^24 pixels in all, or a field too costly to read that many times (a distance field of a very detailed drawing; the message says so, before any pixel is made); `box` as for `Field.map`; `high` not above `low` or either not finite |
| `field.contours` | `contours(levels, *, box, cell, smooth=0, min_length=0)` (Rust: `contours(&[levels], rect, cell)`, and `contours_with(.., &Tidy::default().smooth(s).min_length(l))`) | the lines where the field equals each of `levels` (a number or a list), as one `Path`: marching squares over `box` cut into equal cells no wider than `cell`, the field read at their corners and a line crossing a cell edge at the linear interpolation of the two readings (a reading equal to the level counts as above it; a saddle joins its high corners when the mean of its four is high). A line that reaches the box ends there, open; any other is closed. Every line runs with the high side (a reading at or above the level) on its left as drawn (y down), so the loops of a map that is high inside a region wind one way and its holes the other: `contours(0.5, ..)` of a matte is that region's outline, ready to fill, to hatch inside or to give `rows`. `smooth` blurs the readings with a Gaussian of that many pixels before tracing (grass and grain make hundreds of tiny loops and a jagged line; a few pixels removes them; the blur is refused past 2 billion steps), `min_length` drops every line shorter than that many pixels. Levels are made in the order given. To follow a map's pixels, give `box` the box of the pixel centers (the map's box inset half a pixel) and a `cell` of one pixel. Like `Path.arc` it is a value, not a step: a saved drawing holds its path data | `levels` more than 256 or not finite; `cell` not above 0, a grid of over 4 million nodes, or a field too costly to read at that many nodes (a distance field of a very detailed drawing; the message says so); `box` as for `Field.map` (it has no default: a field, unlike a map, has no extent of its own); more than 10 million points |
| `Field.ramp` | `Field.ramp(start, end)` | 0 at `start`, 1 at `end`, linear along the line between, held beyond | equal points |
| `Field.distance` | `Field.distance(drawing, *, normalized=False)` | from a `Path`, a `Shape` or a `Drawing` (all its shapes together; a `hatch` step reads the drawing again with the values it draws with, so a headline whose weight is a `Param` is hatched along its outline at that weight): signed distance to the nearest outline in pixels, negative inside (nonzero rule); normalised: inside each part (an outline with its holes) 0 on its outer edge rising to 1 on its holes' edges (`d / (d + distance to the holes)`), or, for a part with no hole, to 1 at its deepest point (`d` over its largest inscribed radius, estimated to about a percent), 0 outside and inside a hole. Where parts overlap the distance is to the nearest edge of any: `resolve` first | `GeometryError`: the drawing has no edge; the drawing's own error |
| `Field.noise` | `Field.noise(*, wavelength=400, phase=0, seed=0, octaves=1)` (Rust: `Field::noise(wavelength, phase, seed, octaves)?`) | smooth seeded value noise from -1 to 1, the kernel `wobble` displaces by, one lattice cell `wavelength` pixels wide; `phase` in cycles moves it, one cycle on is the same field up to the rounding of the sum (to the bit for a phase such as 0.25); each of `octaves` (1 to 8) adds the noise at half the wavelength and half the amplitude of the one before, the sum divided by the sum of the amplitudes; equal seeds make equal fields; `wavelength` and `phase` may be `Param`s | `wavelength` not from 1e-200 to 1e100; `seed` not a whole number within ±(2^53 - 1); `octaves` not from 1 to 8; a per-item number |
| field arithmetic | `a + b`, `a - b`, `a * b`, `-a`, `abs(a)`, `a.min(b)`, `a.max(b)` (Rust: `a.add(&b)?`, `a.mul(&b)?`, `a.min(&b)?`, `a.max(&b)?`, `a.abs()?`; `a - b` is `a.add(&b.mul(&Field::constant(-1.0)?)?)?`) | the operation point by point; a number (a `Param`, an expression) on either side is a constant field; two numbers fold into one and a number is written first (`f * 14` prints `14 * f`). Clamping is `f.min(high).max(low)`. A tree nests at most 16 operations deep and reads at most 10,000 fields and operations | more depth or reads, naming `other` (`field` for `abs`); a per-item number, naming `value`; an operand that is neither a field nor a number is a `TypeError` |
| `field.at` | `at(x, y)` | the field's number at a point | |
| `Flow.angle` | `Flow.angle(angle)` | the same direction everywhere, clockwise from the right on screen; 30 and 210 are the same; a number or a `Param` | a per-item number |
| `Flow.noise` | `Flow.noise(*, wavelength=400, phase=0, seed=0)` | smooth seeded noise directions (value noise turning the direction half a turn each way); `phase` in cycles moves it, one cycle on is the same flow up to the rounding of the sum; `wavelength` and `phase` may be `Param`s | `wavelength` not from 1e-200 to 1e100; a per-item number |
| `Flow.contour` | `Flow.contour(drawing)` | the direction of the nearest outline of a `Path`, `Shape` or `Drawing` (as for `Field.distance`), turned smoothly between neighbouring edges of one outline, and blended over every edge about as near as the nearest (weights fall to 0 at twice its distance), so the direction turns across the middle of a stroke instead of jumping from one edge's to the other's. Where two edges meet at a right angle the blend cancels on the bisector: a line field has a defect there | `GeometryError`: the drawing has no edge; the drawing's own error |
| `flow.mix` | `mix(other, fraction)` | `fraction` (a number or a `Param`) 0..1 of the way to `other`, blended in doubled-angle space, so a direction and its opposite count as the same and never cancel; where the two cancel exactly the first flow's direction stands. A mix holds at most 64 flows; mixes and turns nest at most 16 deep together | `fraction` outside 0..1; more flows or depth |
| `flow.rotate` | `rotate(angle)` (Rust: `flow.rotate(90.0)?`) | the flow turned `angle` degrees clockwise on screen at every point (a number or a `Param`); 0 and 180 change nothing, two turns are one turn by their sum, and `Flow.contour(d).rotate(90)` runs across the outlines of `d` (a flow has no sign, so across has no inside or outside). It turns the direction at each point and moves no point, so it has no `about`: a flow turned as a picture about `p` is `Flow.contour(d.rotate(a, about=p)).rotate(a)`. Mixes and turns nest at most 16 deep together; a read costs a unit more than the flow's | `angle` NaN or infinite, or a per-item number; more depth, naming `flow` |
| `flow.at` | `at(x, y)` | the direction at a point in degrees, from 0 up to 180 | |

A `Field` or a `Flow` is a value, built once (a grid of an outline's edges makes it quick to read) and part of the
drawing that uses it. Its numbers (a number given as a `Drawing` step's `tone`, `Flow.angle`'s angle, `Flow.noise`'s and `Field.noise`'s `wavelength` and `phase`, a number operand,
`flow.mix`'s fraction, `flow.rotate`'s angle) may be `Param`s or expressions, never per-item numbers, and the drawings it reads are drawings:
`field.at` and `flow.at` read them with every Param at its default, and a `hatch` step reads them again with the values
it draws with (the step takes the drawings like `avoid`, so the cache, `parameters()` and `with_values` see them). A
`Path` goes where a drawing is asked for. A `Field` or `Flow` prints as its call (`Flow.angle(45)`); two are equal when
they are the same call over drawings that are the same object (or are each one path of the same data). In a document
the drawing is the argument `drawing`, an array of calls (one path is `[{"path": "..."}]`), and a map is an input.

## Params, expressions and per-item numbers

`Param(name, *, default, min, max, unit="", meaning="")`.

| Field | Meaning |
|---|---|
| `name` | letters, digits and `_`, starting with a letter; unique among every Param the drawing reads, those a `with_values` fixed included |
| `default` | the value when none is given; all defaults together are the reference pose |
| `min`, `max` | the range a given value must lie in |
| `unit` | `"px"`, `"deg"`, `"cycle"`, `"fraction"` or `""` (a plain number: a font axis, a count, a seed) |
| `meaning` | one line a person reads beside a slider |

A Param in `deg` or `cycle` is cyclic: any value is accepted and wraps into its range, which spans one turn; a
default at the end of the range is read as its start (`default=360` is 0), like the same value given. Errors
(`ParameterError`): a bad name, a name an output option takes in Python (`into`, `pen_width`, `tolerance`, `unit`, `window`), `min` not below `max`, a
default outside the range, a cyclic range other than one turn.

Arithmetic (`+`, `-`, `*`, `/`, unary `-`) on Params and numbers builds an `Expr`, which saves with the drawing. A
`Param` is an `Expr`, and so is every per-item number below, so they combine (`btvec.random(-10, 10, seed=7) * burst`).
Every number a step takes may be an `Expr`:

- a whole-number argument (a `seed`) too, if its value is a whole number
  within ±(2⁵³ − 1) when drawn; any other value raises `ParameterError`, never rounded;
- a number inside a pair or a list (`translate=(dx, dy)`, `about`, `anchor`, `spacing=(a, b)`, a dash pattern, trace
  `levels`), component by component.

Except: these take literal numbers only: `Drawing`'s `size`, `Field.map`'s `width` and `height`, `text`'s `features`,
the values of `only`, `without` and `with_values`. A per-item number is refused (`ParameterError`) where an argument is
read once rather than per shape: `text`'s `size`, `tracking`, `leading`, `anchor` and `width` (its `axes` take one per
character), every number of `trace`, and every number of a `Field`, a `Flow` or a `hatch` spacing.

An expression with no Param and no per-item number is a plain number: it is computed and checked when the call is made,
like a literal. Every number is finite: NaN or an infinity raises `ParameterError`, when the call is made or when
drawn.

An expression whose value leaves the range its argument accepts raises `ParameterError` at draw time, naming the step,
the argument and the value; a division by zero raises `ParameterError` naming the expression.

A Param's value at draw time is a number, or a list with one number per shape (`pulse=[...]`): the way to give each
item a value from the caller's own signal. Entry `i` goes to the shape whose `index` is `i` at the step that reads the
Param; `text`'s `axes` is read for the characters of `content` instead, as there are no shapes yet: entry `i` goes to
the `i`-th character. Every value, and every entry of a list, is checked against its Param's range before anything is
drawn (a cyclic Param wraps it; NaN and infinities are refused), whether or not a step reads it with shapes; a value for a Param the
drawing does not read is refused as unknown. A list's length is checked where a step reads it: one that differs from
the number of items there (shapes, or the characters of a `text`'s `axes`) raises `ParameterError` naming the Param, the step and both counts; the length of a list
that no step reads with shapes is not checked.

Per-item numbers take a value per shape, from its attributes:

| Function | Signature | Promise | Errors |
|---|---|---|---|
| `by_index` | `by_index(first, last)` | `first` on the first shape of the current list, `last` on the last, linear between | |
| `by_order` | `by_order(first, last)` | the same, read from each shape's `order` | |
| `random` | `random(low, high, *, seed=0)` | a number in `low..high` per shape, from `seed` and its index | `low` above `high`; `seed` not a whole number |
| `stagger` | `stagger(progress, *, spread=0.5)` | each shape's own progress 0..1: the shape at order `p` runs while `progress` goes from `p × spread` to `p × spread + 1 - spread` (0: all together; towards 1: one after another) | `spread` outside 0..1 or equal to 1 |

On a single shape `by_index` and `by_order` give `first`; a shape that carries no `order` (a lone path or shape given
as it is) reads 0. Operands that are plain numbers are checked when the call is made, the others when drawn. Rust: the
functions `by_index` and `by_order`, and `Random::new(low, high).seed(s)` and `Stagger::new(progress).spread(s)`, which
convert into an `Expr`.

`btvec.ease` holds pure functions of a number, for the caller's own code (a function of time for `frames`, a list
of values per item): `in_cubic`, `out_cubic`, `in_out_cubic` and `out_back` (overshooting to about 1.1). Each maps 0..1
to 0..1 and holds its end values outside 0..1. They take numbers only: a drawing holds no easing.

## Outputs

`width` and `height` are output pixels: whole numbers from 1 to 100,000 for SVG and the mesh; for pixel outputs
(`to_rgba`, `to_png`) from 1 to 65,000, and at most 2^27 pixels (512 MiB of RGBA) in all. The design frame scales by the largest
factor that fits (`min(width / frame width, height / frame height)`) and is centered; a drawing without a frame is drawn
at one output pixel to the design pixel from the top left. Content outside the frame is not clipped. `**values` give
Params their values by name; a Param not given takes its default.

`to_svg` writes coordinates, stroke widths and a shadow's offset, deviation and region in output pixels, not as a transform, so a drawing at twice the size has
numbers exactly twice as large; every number is the shortest text that reads back as the same value. A shape is one
`<path>` in paint order; its `opacity` applies to fill and stroke together (a group opacity: the pixels composite the
same way); a colour's alpha is `fill-opacity` or `stroke-opacity`; a blend is `mix-blend-mode:<mode>`; a shape with
an empty path writes nothing. A shape with a shadow is a `<g>` that holds the shadow and the path and carries the
opacity and blend (`style` above). The root has `isolation:isolate`, so a blend works within the picture and not with the
page around it.

`to_rgba` paints the shapes in order, each as one group: its shadow, then fill, then stroke, then the shape's `opacity` and blend apply
to the group once (a stroke over a fill at opacity 0.5 is no darker where they overlap). Colours blend as sRGB values,
with no gamma conversion, as SVG does. The bytes are straight alpha: a colour byte is the premultiplied colour times 255
over the alpha, rounded to the nearest whole number (halves up), and a pixel of alpha 0 is `0, 0, 0, 0`. Placement,
widths and the corner and end rules are exact, but the pixels are drawn by a rasteriser crate that is only required to
look the same on every target: an edge pixel can differ by a level. Curves are flattened once, in design pixels, to
within 0.01 of the curve (the flattening that measuring uses), and a stroke is expanded to the same flatness in output
pixels, so the rasteriser is given straight lines only; a circle of radius 10 covers its area to within 0.05%. A
sub-path that never leaves its start (`line_to` its own start) leaves a disc under a round end and a square under a
square end, as SVG draws it, and nothing under a butt end. A segment much shorter than the stroke is wide, at a sharp
turn, can lose a sliver of the stroke on the inside of the turn.

`to_png` is the pixels of `to_rgba` (the same limits and errors) as a PNG file: 8 bits a channel, colour type 6, straight
alpha, no interlacing, and no chunks but the header, the data and the end (no colour space: read it as sRGB). The same
call gives the same bytes; the bytes may change when the encoder crate is updated, what they decode to does not.

A window renders one rectangle of a large output: `to_rgba(width, height, /, *, window=(x, y, w, h), **values)` and `to_png`
likewise (Rust `to_rgba_window(Size::px(width, height), Window::px(x, y, w, h), &values)` and `to_png_window`; JavaScript
`toRgba(width, height, values, { x, y, width, height })`, `toPng` the same). `width` and `height` are the whole output, the
one the design frame is placed in, up to 100,000 a side; the window is whole pixels inside it, and obeys the limits of a pixel
output (65,000 a side, 2^27 pixels), so a canvas the whole render refuses can be drawn in bands. The result is the window's
pixels only (`w × h × 4` bytes), **the same bytes** as the same rows and columns of the whole render, for every drawing
(`tests/it/raster_window.rs`; [design-decisions.md](design-decisions.md), "Render a window (V10e)"). The rasteriser draws a
region larger than the window: 8 pixels on every side, its top left on a multiple of 256 across (its wide tile) and of 4
down, and, for a blurred shadow, the offset and the reach of its blur, and back to the corner of the shadow's shape. That
region obeys the limits of a pixel output too: a window within 300 pixels of 65,000 a side, one near them with a wide blur,
or one that cuts a blurred shape too large to hold raises `ParameterError` naming `window`, as does a window that is not
whole pixels inside the output; a window never differs from the whole, it is drawn or refused. A Param may not be
named `window` (as `into`, it is an output option).

To look at a drawing, `uv run poe look FILE [--size WxH] [name=value ...]` draws a `.json` document, or a `.py` program
that leaves a `drawing` (as `examples/star.py` does), to `dist/look/<name>.png` and `<name>.svg` and prints the two
paths. The size is the drawing's own frame, else 800x600; `name=value` sets a Param. Exit code 1 and one line on stderr
for a missing or unreadable file, a program without `drawing`, a bad size or value, or an error from the library. It
opens nothing.

| Method | Signature | Promise |
|---|---|---|
| `to_svg` | `to_svg(width, height, /, **values)` | an SVG document of that size; shadows, halftone, knockouts, clips, masks and blends kept, so it shows what the pixels show |
| `to_png` | `to_png(width, height, /, *, window=None, **values)` | PNG bytes, 8-bit RGBA, transparent where nothing is drawn; of the window, if given |
| `to_rgba` | `to_rgba(width, height, /, *, window=None, **values)` | `width × height × 4` bytes (a window's `w × h × 4`), rows top to bottom, straight alpha |
| `to_plot_svg` | `to_plot_svg(width, height, /, *, pen_width, tolerance=None, unit="px", **values)` | lines only, one layer per stroke colour (one pen each), ordered to cut pen-up travel and simplified below the pen's resolution (below) |
| `to_mesh` | `to_mesh(width, height, /, *, into=None, **values)` | triangles for the picture (below) |
| `to_json` | `to_json()` | the drawing as JSON: the same tree written down |
| `Drawing.from_json` | `Drawing.from_json(text, *, inputs=None)` | the drawing back, given the bytes (`bytes` or `bytearray`) of its inputs by name, a dict; it draws to identical bytes. A static method. In Rust `Drawing::from_json(text, &Inputs::new().set("anybody", bytes))`, in JavaScript `Drawing.fromJson(text, { anybody: bytes })` with a `Uint8Array` each |
| `Drawing.preset` | `Drawing.preset(name)` (Rust: `Drawing::preset(name)?`, JavaScript: `Drawing.preset(name)`) | a drawing that comes with the library (`Drawing.preset_names()` lists them; `bt-vector list` shows each with its description). It is an ordinary drawing: `parameters()` describes its sliders and it needs no bytes. A static method; a `name` that is not a preset is refused (the message lists them) |
| `Drawing.preset_description` | `Drawing.preset_description(name)` (Rust: `Drawing::preset_description(name)?`, JavaScript: `Drawing.presetDescription(name)`) | the one-sentence description of the preset `name`: what its look is (one sentence, ends with a full stop, no line break). A static method; an unknown `name` is refused exactly as `Drawing.preset` refuses it |
| `Drawing.preset_names` | `Drawing.preset_names()` (Rust: `Drawing::preset_names()`, JavaScript: `Drawing.presetNames()`) | the names `Drawing.preset` takes, sorted |
| `drawing_schema` | `btvec.drawing_schema()` | the JSON Schema of the format, as text, generated from the registered calls |

The browser playground (`playground/`) is a static page over the package built with `uv run poe playground`
(`wasm-bindgen --target web` into the ignored `playground/pkg/`; the page needs http, `uv run poe playground-serve`): it
lists `Drawing.presetNames()` with their descriptions, draws with `toRgba` at the canvas's size, makes a slider of each
`parameters()` entry and can bind it to the clock, the pointer or the scroll; a JSON document opens by paste or file (one
that names inputs is refused, naming the input). `uv run poe playground-check` drives it in headless Chrome.

In JavaScript (the package built from `crates/wasm`) a document loads with `Drawing.fromJson(text, inputs?)` (`inputs` an object of names to `Uint8Array`s, none for a document without fonts); a drawing has
`toJson()`, `parameters()` (plain objects `{ name, default, min, max, unit, meaning }`; `unit` is `""` for none), `size`
(`[w, h]` or `undefined`) and the outputs, each `(width, height, values?)` (`toRgba` and `toPng` also `window?`) with `values` an object of Param names to a
number or an array of numbers: `toSvg` (a string), `toRgba` and `toPng` (a `Uint8Array`, a copy), `toMesh` (a new
`Mesh`) and `toMeshInto(mesh, width, height, values?)` (refills `mesh`, below). Errors are as in Python, thrown as
`Error`s named by kind. The same drawing gives the same SVG text and mesh buffers in WebAssembly as natively, exactly;
the pixels look the same: at most 2 levels apart on at most 0.02% of the values of a picture (the rasteriser crate is only
required to look the same on every target).

The same drawing and values look the same on every target (the same bytes wherever that holds, which the differential test checks). `to_json` raises `ParameterError` for a drawing beyond a
limit of [formats.md](formats.md) ("Limits and security") and for a literal division by 0; `from_json` raises
`FormatError`. Errors (`ParameterError`): a width or height that is not a whole number from 1 to 100,000 (to 65,000 for
pixel outputs, which also refuse more than 2^27 pixels, naming `size`), a coordinate or stroke width beyond 10 million
device pixels in a pixel output or a mesh (a mesh also checks design pixels), more than 10,000,000 edges flattened in
one draw (in a mesh the vertices made count too), or in the outline of one shape's stroke (all naming the shape), a value for no Param (the message lists the Params), a value
outside its Param's range, a list of the wrong length; and whatever a step raises. How fonts and maps are referred to in JSON: [formats.md](formats.md), "Inputs".

### Mesh

A `Mesh` holds three buffers, not objects. `vertices`: 8 `float32` a vertex, `x, y` (output pixels, y down), `r, g, b,
a` (premultiplied), `item` (the shape's index) and `order` (its `order` attribute), for a shader's own per-item
effects. `indices`: `uint32`, three a triangle, in paint order. `ranges`: `uint32`, three a draw range (first index,
index count, blend mode: 0 normal, 1 multiply; `to_mesh` refuses every other blend), in paint order, so a page draws each range with its blending.
`generation`: a number that changes exactly when the contents change, so a redraw with the same values uploads
nothing. A new mesh is generation 0 and empty; a fill whose buffers differ from what the mesh held adds 1 (the first
fill with something to draw gives 1), and one that makes the same buffers leaves it.

A shape is up to two layers, its fill and then its stroke, each covering every point once, so a translucent fill and
stroke show darker where they overlap, unlike the pixels' one group per shape. A vertex alpha is the colour's alpha
times the shape's opacity; a layer of alpha 0 has no triangles. A shape with an empty path adds nothing but keeps its
number as `item`; `order` is 0 where a shape has no `order` attribute. Curves are flattened as for the pixels, to
within 0.01 design pixels. A shape whose tessellation would take more than about a second (an outline that
crosses itself thousands of times, or has thousands of edges side by side) raises `ParameterError` naming the shape and
`MAX_TESSELLATION_WORK`; pixels and SVG draw it.

`into=mesh` refills that mesh in place of a new one and returns it; a draw that raises leaves it as it was. In
Python `vertices`, `indices` and `ranges` return `array.array` copies (typecodes `f` and `I`), a new one at each
read, which `numpy.asarray` reads through the buffer protocol; the copy keeps the bindings free of `unsafe`. In
JavaScript, `new Mesh()` is empty (generation 0), `drawing.toMesh(w, h, values)` returns a new one and
`drawing.toMeshInto(mesh, w, h, values)` refills one in place (the buffers are reused, and it is left as it was if the
call throws; there is no `{ into }` options object because wasm-bindgen lends an object to a call only as an argument of
its own). `mesh.verticesPtr`, `mesh.indicesPtr` and `mesh.rangesPtr` (byte offsets) and the matching `...Length`
(in elements) locate the buffers, which are the core's own and are not copied, in `btvec.wasmMemory()` (a
`WebAssembly.Memory`); `mesh.generation` is a number. Make the typed-array views fresh after every call, since a pointer
can move and the memory can grow, which detaches the arrays made before; `mesh.free()` returns the buffers to
WebAssembly at once instead of when the mesh is collected
([mesh_upload.js](examples/mesh_upload.js)).

The mesh is a display format; pixels and SVG stay the reference. Drawn with WebGL it matches `to_rgba` inside plain
shapes and differs on edges (anti-aliasing: a few levels of 255 on average at 4x MSAA, more at hairlines) and where a
shape's fill and stroke overlap under opacity or multiply (two layers, not one group). A multiply range is exact over any
ground only drawn in two passes, as [mesh_upload.js](examples/mesh_upload.js) does; the single
`blendFunc(DST_COLOR, ONE_MINUS_SRC_ALPHA)` is right only over an opaque ground (spike X1, `roadmap.md`). What each
step becomes: opacity goes into the vertex alpha, multiply into the draw ranges, a knockout cuts the triangles below it
(a boolean while meshing: the region its fill and its stroke outline cover is subtracted from the layers below, hard
shadows included, and the edges and crossings are charged to `MAX_SEGMENTS` as a boolean step is, so a stroke that is
cut ends flat at the cut, where the pixels end it the same way), halftone dots are triangles (those that cross the shape's outline cut to it). A shadow with a blur of 0 is the
shape's triangles moved and tinted, under it (one layer for the fill, one for the stroke, each in the tint times its own
alpha, so where they overlap the shadow is darker than the pixels' one silhouette); with a blur above 0 there is no
triangle form and `to_mesh` raises `ParameterError` naming `shadow` and the blur, because a blur needs an offscreen layer
and blur passes, which three flat buffers and draw ranges cannot hold (a later range kind for one would break no caller).

### Plotter

`Drawing.to_plot_svg(width, height, /, *, pen_width, tolerance=None, unit="px", **values)` (Rust `to_plot_svg(size, &values, &Plot::new(pen_width)?.tolerance(t).unit(PlotUnit::Mm))`,
JavaScript `toPlotSvg(width, height, values, { penWidth, tolerance, unit })`) writes strokes only. A pen draws lines, so:

- **The unit** is `"px"` (default: whole sides, bare numbers), `"mm"` or `"in"`: the sides are then in that unit, any number above 0
  up to 100,000, the document says `width="210mm" height="297mm"` and its `viewBox` has the same numbers, so Inkscape and plotter
  software read the physical size. The design frame is fitted into it as for pixels, and `pen_width` (design pixels) scales with it.

- **One layer per pen**, a pen being a stroke or fill colour (alpha ignored), in the order the colours first appear:
  `<g inkscape:groupmode="layer" inkscape:label="pen 1 #d0341a" fill="none" stroke="#d0341a" stroke-width=..>` holding one `<path>` per line.
- **A stroke is its center line**, the shape's own width ignored (`pen_width` is the pen's). **A fill is hatched** at 45 degrees
  `pen_width` apart inside its outline (a solid too small for a line is its outline), so a `bands` drawing plots as the
  hatched bands.
- **Simplified** (Douglas and Peucker) to within `tolerance` design pixels of the line the drawing has, default a quarter of
  `pen_width`; 0 keeps every point. A closed outline is a line that comes back to its start. The simplification is refused
  past a billion distance tests.
- **Ordered** by nearest neighbour to cut pen-up travel: the pen starts at the top left corner of each layer, draws the line
  with the nearest end (reversed if that is its last; a closed loop entered at its nearest vertex), and goes on from where it
  ended. A grid finds the nearest end, so a plot of 50,000 lines orders in about two seconds; a crowd the grid cannot spread
  (100,000 strokes packed into a corner of a big page) is counted, and past 2e8 entries scanned the strokes left keep their
  drawing order. Measured on small cases the
  travel is at most 1.41 times the best order over twelve seeds of seven strokes.
- **Refused, naming the shape** (`shape 3`): a gradient, shadow, clip, mask, knockout or blend mode, which a pen
  cannot draw. A plot draws only what the picture shows: a shape with opacity 0, or a fill or stroke that is fully transparent, is not
  plotted; a translucent colour is its opaque pen. A shape drawn later does not hide one drawn earlier. A fill the pen is too wide
  to hatch, or a `pen_width` or `tolerance` past 1e100, is a `ParameterError` naming it.

Leave the paper out of the drawing sent to the plotter: a background rectangle filled with the paper's colour becomes
a solid block of hatching. Keep the inks in a drawing of their own and stack the paper under it only for pixels and SVG
([hatched_poster.py](examples/hatched_poster.py)).

### Frame export

`btvec.frames(scene, *, width, height, fps=30, seconds, out)` (Python only; Rust and JavaScript callers loop over `to_png`,
`toRgba` or `toMesh` themselves). It calls `scene(t)`, a function of seconds returning a `Drawing` (usually
`drawing.with_values(...)`), for `round(seconds × fps)` frames (Python's `round`: a half goes to the even number) at
`t = i / fps`, `i` from 0, so the first frame is at 0 and `t = seconds` is never drawn (a loop of `seconds` repeats
without a doubled frame), and writes each frame exactly as `to_png` draws it, at `width` × `height`. `fps` and
`seconds` are finite numbers above 0, and `seconds` covers at least half a frame. `out` (a `str` or path; the suffix in any case) says what is written:

- `.png` with one run of `#` in the file name (`"frames/badge-####.png"`): a transparent PNG sequence. The run is the
  minimum number of digits of the zero-based index, zero padded; an index that needs more widens it (`##` gives `00` to
  `99`, then `100`). A `#` in a folder name is literal.
- `.mov`: ProRes 4444 with alpha. `.webm`: VP9 with alpha. A clip needs ffmpeg on the `PATH` (with `prores_ks` or
  `libvpx-vp9`); the frames go to it as raw RGBA. A clip is lossy: it decodes to pixels close to the PNG ones, not equal.

The folder is created, with its parents, once the first frame is drawn. Existing files of the same names are overwritten;
frames of a longer earlier run stay. If `scene` or a write fails, the frames already written stay; a clip is written under
a hidden name beside `out` and moved into place when ffmpeg is done, so a failure leaves no half-written clip and an
older `out` untouched. It is the only part of the library that knows time. A `scene` that builds a new drawing per frame
(a new map to trace, say) makes a clip of many drawings, not one drawing and its values, so the clip cannot be saved as
one JSON.

Errors: `ParameterError` for `fps` or `seconds` not finite and above 0, `seconds` shorter than half a frame, an `out` of
no known kind or a `.png` without exactly one run of `#` in its file name (nothing is drawn or written), and a size the
outputs refuse (as `to_png`, on the first draw); what `scene` raises; `OSError` when a file or folder cannot be written
or ffmpeg is missing or fails (its message is in the error).

## Errors

| Error | When | Carries |
|---|---|---|
| `BullterrierVectorError` | the base of every error the library raises for a caller's input | |
| `ParameterError` | a value outside what is accepted; also a `ValueError` | the parameter's name and the value |
| `GeometryError` | geometry that fails: an empty path measured, a guide of no length, an `offset` on an outline that cannot be resolved; no boolean raises it | the verb and the shape's index |
| `FormatError` | a document `from_json` cannot read ([formats.md](formats.md), "Errors"); also a `ValueError` | the JSON Pointer of the value and what is wrong |

A drawing that reads more than 1,000 Params of different names (`MAX_PARAMS`) raises `ParameterError` at the call that
passes it.

Plain numbers are checked when a call is made; Params, expressions and lists when the drawing is drawn; geometry when it
is computed. Depth is bounded: drawings nested more than 16 deep are refused when built, an expression more than 16 levels
deep is refused when used (printing and comparing it are safe, however long a loop made it), and `from_json` refuses a
document nested more than 40 levels. All of it needs a thread stack of at least 256 KB (a Python thread has 512 KB on
macOS). In Python and JavaScript the error points at the call that caused it; in Rust, eager calls return a
`Result` and a drawing's chain reports its first error at the first draw ("Languages").

## Conventions

- **Names in the field's own terms.** `amplitude`, `wavelength`, `pitch`, `spacing`, `tracking`, `leading`, `phase`,
  `seed`, `tolerance`; never `strength`, `intensity`, `factor`, `amount`, `val` or `k`. One name per concept: a
  position along a path is `at` (or `start`, `end`), a pivot is `about`, an angle is `angle`, "this fraction of the way to the other" is `fraction`, a colour argument is `color`.
- **Units.** Design pixels for lengths, scaled with the output, so no caller ever scales. Degrees, clockwise, for
  angles, wrapping. Cycles for phases, wrapping. Fractions 0..1 for positions along a path and for progress. Thousandths
  of an em for tracking; font axes in the font's own units; areas in square pixels; millimetres only in the plotter
  output. A number is a state, never a duration.
- **Sampling in design pixels.** Everything the library samples internally (tracing an analytic field, resampling
  before a deformer, flattening curves for pixels and the mesh, seeding hatch lines) uses steps in design pixels, each a
  named constant, never the output size; a map is sampled at its own pixels. A drawing has the same geometry at any
  output size: only rasterising depends on it.
- **Arguments.** What a call cannot do without is positional (`offset(8)`, `rotate(30)`), unless its keyword makes
  the call read as a sentence (`text("NO", font=font)`, `halftone(INK, tone=photo)`);
  every option is keyword-only (`wobble(amplitude=3)`).
- **A `Path` verb takes plain numbers; a Param is for a `Drawing` step.** A `Path` verb computes at once, so it has no
  values to read a Param with: a `Param` or an expression given to one (`path.rows(spacing=gap)`, a `tone` of
  `path.stipple`) is a `TypeError`, never read at its default. A `Field` is still a `tone` there (its own Params are
  read at their defaults, as `field.at` reads them). Wrap the path in `btvec.Drawing(path)` to vary it.
- **Defaults look good** with no tuning: a default comes from a gallery piece, not from a round number. The defaults
  written here (leading 1.2 × size, pitch 8, shadow 8 and 16, wavelengths 80, 120 and 400...) are placeholders until the
  batch that builds the step tunes them with its gallery piece.
- **Reading as a sentence.** `plane.subtract(letters)`, `figure.offset(8)`,
  `letters.only(char="O")`, `petal.rotate(30)`. Related verbs sit together in the tables above.
- **How a method is documented.** One line that states the promise in the field's terms; each argument with its unit,
  range and default; each error it raises and when. Never how it works.

## What this API deliberately does not have

- **No callbacks inside a drawing.** A drawing must save to JSON and draw where no caller's code runs. A value that
  depends on the caller's signal is a list given at draw time. (`Path.map_points` takes one, and computes at once.)
- **No mutation.** Every call returns a new value; a drawing used in two places is the same drawing in both.
- **No timeline, clock or easing over time.** A Param is a state; whoever animates it sets it and draws again. Only
  `frames` loops over time, calling the caller's function.
- **No global state.** No current style, canvas, font or random generator; randomness comes from a `seed`.
- **No implicit IO.** Fonts and maps arrive as bytes; only `frames` writes files, to the path it is given.
- **No configuration objects.** Options are keyword arguments of the step they shape.
- **No collection type.** A list of shapes is a list; a drawing is the only container.

## Command line

`bt-vector` (installed with the Python package) renders and checks drawings without code. SOURCE is a preset name or,
when it ends in `.json`, a document file.

| Command | What it does |
|---|---|
| `bt-vector list` | one line per preset: `name  WxH  description` |
| `bt-vector describe SOURCE [--input NAME=FILE ...]` | a preset's description, then the design size and each `Param`: name, default, min, max, unit (`-` for none), meaning |
| `bt-vector render SOURCE -o OUT [--size WxH] [--input NAME=FILE ...] [NAME=VALUE ...]` | writes `OUT` (`.png` or `.svg`, by its suffix) and prints its path |
| `bt-vector validate FILE.json [FILE.json ...] [--input NAME=FILE ...]` | prints `ok FILE` for each document that loads, `FILE: <message>` on stderr for each that does not |

`render` draws at the drawing's design size, 800x600 when it has none, unless `--size` is given. `NAME=VALUE` sets a
Param (a name the drawing has no Param of is refused, listing its Params). `--input` (on `describe`, `render` and
`validate`) gives `from_json` the bytes of an input the document names (a font or a map that is not bundled).
Exit codes: 0 done; 1 refused, one line `bt-vector: <what is wrong>` on stderr (never a traceback); 2 wrong usage.
Results go to stdout, errors to stderr. The same in Python: `bullterrier_vector.cli.run(argv)` returns the exit code.

## Languages

The same names in every language, in each language's case: `snake_case` in Python and Rust, `camelCase` in JavaScript
(`toPng`, `presetNames`). Python and Rust build drawings; JavaScript (the package built from `crates/wasm`) loads a
document and draws it, so the table has no JavaScript column and the paragraph below it says what JavaScript has.

|  | Python | Rust |
|---|---|---|
| Constructors | `btvec.Drawing(a, b, size=(w, h))`, `btvec.Font(data)`, `btvec.Param("rise", default=0, min=0, max=1)` | `Drawing::new([a, b]).with_size(w, h)`, `Drawing::from(path)`, `Font::new(data)?`, `Param::new("rise", 0.0, 0.0, 1.0)?.unit(Unit::Fraction)?` |
| Options | keyword arguments | a value named after the step or function, built with `new` and setters (`Text::new(&font).size(560.0)`, `Random::new(-10.0, 10.0).seed(7.0)`); the required argument alone converts into it (`.rotate(Deg(30.0))`, `.stroke(color)`, `.scale(2.0)`; `Deg` takes anything that converts into an expression: `Deg(Random::new(-10.0, 10.0).seed(7.0) * &burst)`); built so far: `Rotate`, `ScaleBy`, `Stroke`, and `Select` for `only` and `without` (`Select::new().attribute("char", 'O')`). A step with required arguments takes them in place and its options last, as one such value whose `new()` alone is every default: `along(&path, Along::new().near(p))`, `slice(0.0, 0.5, Slice::new())`, `dash([8.0, 4.0], Dash::new().phase(&march))`, `merge(Merge::new().by("word"))`, `outline(4.0, Outline::new().profile(taper))`; one method per step, never a second name for an option |
| Expressions | operators: `1 + 0.08 * pulse` | operators on `Param`, `Expr` and their references, so nothing is cloned: `1.0 + 0.08 * &pulse`, `0.6 * &misregistration` |
| Draw-time values | keyword arguments: `to_png(1080, 1350, weight=820)` | `&Values::new().set("weight", 820.0)` |
| Output options | keywords beside the values: `to_mesh(w, h, into=mesh, **values)`, `to_plot_svg(w, h, pen_width=0.5, tolerance=None, unit="mm", **values)` (so no Param may take `into`, `pen_width`, `tolerance`, `unit` or `window`) | a last argument after the values: `to_mesh(size, &values, &mut mesh)`, `to_plot_svg(Size::px(300.0, 375.0), &values, &Plot::new(0.5)?.unit(PlotUnit::Mm))` |
| Units as types | | `Deg(..)` for every angle of a `Drawing` step (`rotate`, `Flow::angle`, `halftone`'s and `wave`'s `angle`, `branch`'s `angle` and `wander`); a `Path` method takes plain degrees (`rotate(30.0, None)?`); `Size::px(w, h)` (whole output pixels, else an error at the draw; for a plot in `PlotUnit::Mm` or `In` the sides of the same `Size` are in that unit and need not be whole) |
| Colours | `"#ff4f9a"` | `Color::hex(0xff4f9a)`, `Color::NONE` |
| Errors | exceptions | `Result<_, btvec::Error>`. On a `Path`, a constructor or verb that can fail returns a `Result` (`Path::rect(..)?`, `path.offset(8.0)?`) and one that cannot returns the value (`reverse`, `split`, `length`); a verb that takes a plain number returns a `Result`, as a NaN or an infinity is a `ParameterError` (`path.translate(1.0, 2.0)?`); a path is built from `Path::start(point)`, so no segment comes before a start. `Error` is `#[non_exhaustive]`: `Error::Parameter { parameter, message }` and `Error::Geometry { verb, message }`, owned strings so a drawing can prefix the step. A `Drawing` chain keeps its first error and returns it from the first draw, `to_json` or `parameters()`, naming the step by its place in the chain and its verb (`step 3 (rotate): ...`); `parameters()` is the early check, so no `?` per step |
| Optional `Path` arguments | keywords: `point_at(0.5, near=p)`, `rotate(30, about=p)`, `Path.rect(0, 0, w, h, radius=8)`, `scale((2, 3))` | plain arguments, `None` for the default: `point_at(0.5, Some(p))?`, `rotate(30.0, None)?`, `Path::rect(0.0, 0.0, w, h, 8.0)?`; `scale` takes a number or a pair through `Scale`: `scale(2.0, None)?`, `scale((2.0, 3.0), None)?`; `Path::polyline(points, closed)` and `Path::star(center, outer, inner, points)` take their options in place; `map_points` takes `FnMut(Point) -> Result<Point, Error>` |
| Bytes in | `bytes` | `Vec<u8>` |
| Frame export | `btvec.frames` | the caller's loop over `to_png` |

The kinetic type piece, an excerpt of the part that builds the drawing, in Python and Rust (whole programs, with the bend and the morph:
[Python](../examples/kinetic_type.py), [Rust](../crates/core/examples/kinetic_type.rs); [the JavaScript program](../crates/wasm/examples/kinetic_type.mjs) loads the document of the Python drawing):

```python
letters = (
    btvec.text(
        "NO\nSLEEP",
        font=font,
        size=560,
        axes={"wght": weight + 120 * pulse},
        leading=520,
        anchor=(WIDTH / 2, 600),
        align="center",
        width=980,
        fit="wdth",
    )
    .rotate(btvec.random(-10, 10, seed=7) * burst)
    .translate(btvec.random(-30, 30, seed=8) * burst, btvec.random(-50, 50, seed=9) * burst)
)
frame = btvec.Path.rect(0, 0, WIDTH, HEIGHT)
misregistration = 3 + 14 * burst
blue = (
    btvec.Drawing(frame)
    .translate(0, HEIGHT * (1 - rise))
    .subtract(letters.translate(misregistration, 0.6 * misregistration))
    .fill(BLUE)
    .blend("multiply")
)
pink = letters.fill(PINK).blend("multiply").opacity(ink)
hairlines = letters.stroke(INK, width=1.5).opacity(1 - ink)
piece = btvec.Drawing(frame.fill(PAPER), blue, pink, hairlines, size=(WIDTH, HEIGHT))
png = piece.to_png(1080, 1350, rise=1, ink=1, weight=620, burst=0.4, pulse=[0.9, 0.6, 0.3, 0.1, 0, 0, 0, 0])
```

```rust
let letters = btvec::text(
    "NO\nSLEEP",
    Text::new(&font).size(560.0).axes([("wght", &weight + 120.0 * &pulse)]).leading(520.0)
        .anchor((WIDTH / 2.0, 600.0)).align(Align::Center).width(980.0).fit("wdth"),
)
.rotate(Deg(Random::new(-10.0, 10.0).seed(7.0) * &burst))
.translate(Random::new(-30.0, 30.0).seed(8.0) * &burst, Random::new(-50.0, 50.0).seed(9.0) * &burst);
let frame = Path::rect(0.0, 0.0, WIDTH, HEIGHT, 0.0)?;
let misregistration = 3.0 + 14.0 * &burst;
let paper = frame.fill(PAPER);
let blue = Drawing::from(frame).translate(0.0, HEIGHT * (1.0 - &rise))
    .subtract(letters.translate(&misregistration, 0.6 * &misregistration), 0.01)
    .fill(BLUE).blend(Blend::Multiply);
let pink = letters.fill(PINK).blend(Blend::Multiply).opacity(&ink);
let hairlines = letters.stroke(Stroke::new(INK).width(1.5)).opacity(1.0 - &ink);
let piece = Drawing::new([paper, blue, pink, hairlines]).with_size(WIDTH, HEIGHT);
let values = Values::new().set("rise", 1.0).set("ink", 1.0).set("weight", 620.0).set("burst", 0.4)
    .set("pulse", vec![0.9, 0.6, 0.3, 0.1, 0.0, 0.0, 0.0, 0.0]);
let png = piece.to_png(Size::px(WIDTH, HEIGHT), &values)?;
```

**JavaScript** has no builders: it loads a document (`Drawing.fromJson(text, inputs?)`, `Drawing.preset(name)`,
`Drawing.presetNames()`, `Drawing.presetDescription(name)`), writes it back (`toJson`), describes it (`parameters`, `size`)
and draws it (`toSvg`, `toRgba`, `toPng`, `toMesh`, `toMeshInto`, `toPlotSvg`), with `Mesh` and `wasmMemory()`. Draw-time
values are an object (`toPng(1080, 1350, { weight: 820 })`), output options an object after the values (`toPlotSvg(w, h,
values, { penWidth: 0.5, unit: "mm" })`; a `Mesh` is lent as its own argument: `toMeshInto(mesh, w, h, values)`), bytes in
are `Uint8Array`s and colours `"#ff4f9a"` strings. Errors are thrown `Error` objects whose `name` is `"ParameterError"`,
`"GeometryError"` or `"FormatError"` and whose message is the core's (the same text as Python's); there are no classes to
`instanceof` (wasm-bindgen cannot define subclasses), so the `name` is the contract. Frame export is the caller's loop over
`toRgba` or `toMesh`.


## Coverage of the prototypes' needs

Every ranked need in [prototype-findings.md](prototype-findings.md), "What the library needs, ranked":

| # | Need | Where the API expresses it |
|---|---|---|
| 1 | Per-glyph variable-font layout that re-flows; tracking, stacked lines, fitting at a reference pose, outlines cached by axis location | `text(axes=..., tracking=, leading=, width=, fit=)`, per-item numbers in `axes`; caching is internal |
| 2 | Evenly spaced streamlines with spacing from a tone, region masks, obstacles, closed loops; line fields and distance fields | `hatch(spacing=(a, b), tone=, flow=, avoid=)` inside each shape; `Flow.noise`, `Flow.contour`, `flow.mix`; `Field.distance` ([flow_hatch.py](../examples/flow_hatch.py)) |
| 3 | Steady tracing over a sequence of maps: smoothing in time | `Field.smooth_maps(maps, sigma=)`, then `trace_levels` / `trace_labels` |
| 4 | Booleans that hold on overlapping, rotated glyphs; ink layers from them | `union`, `subtract`, `intersect`, `exclude`; glyph overlaps removed by `resolve`; inks as drawings with `blend("multiply")` ([kinetic_type.py](../examples/kinetic_type.py)) |
| 5 | Shrink and grow per layer; dropping small parts and filling small holes | `offset(r).offset(-r)`, `offset(-r).offset(r)`; `trace_levels(min_area=, min_hole=)` for traced shapes |
| 6 | Per-item transforms about a typographic origin; seeded randomness and stagger by index as one mechanism | `translate`, `rotate`, `scale` about each shape's origin; `by_index`, `by_order`, `random`, `stagger`; lists at draw time |
| 7 | Curve fitting through dense polylines | `simplify`; `trace_levels(tolerance=)` |
| 8 | One picture to both pixels and SVG, every effect kept | `to_svg`, `to_png`, `to_rgba` |
| 9 | A fast halftone fill | `halftone` |
| 10 | Plotter export: stroke only, ordered, simplified, solids hatched, a layer per pen | `to_plot_svg` |
| 11 | A variable-width stroke, tapered at free ends | `outline(width, profile=)` as a filled outline, and `stroke(profile=)` drawn the same way by every output |
| 12 | Partial path by length from a stable start | `slice(0, progress)`: every outline the library makes in one orientation and order, closed ones measured from the topmost point, or from the point nearest `near=` where the top jumps |
| 13 | Fitting text to a box by a variable axis, with a clear signal when the axis runs out | `text(width=, fit=)`; `ParameterError` naming the line and the width reached |

Also from the findings: draw-on (`slice`), the riso misregistration that grows on a hit (an expression,
`3 + 14 * burst`), the accent letter (`only`, `without`), type under the paper (stacking order), paper that slides
(`translate` by a Param). Gaps beyond the ranked list: a multi-colour trace (a map of colour labels; V14 names it) and a
re-drawable series node (a series is plain code now, [series_bars.py](../examples/series_bars.py); the effects track adds a
node if a piece needs one).

Verbs no example uses yet, and what needs them: `merge` (words and lines as units), `reverse` (V11: a draw-on the other way round), `locate`
(hit-testing for an editor), `Flow.contour` and
`flow.mix` (V15: the findings' line fields, noise mixed with a glyph's contour), `resample` (V11; dots spaced evenly along a path
on the effects track), `bulge`, `envelope` and `Path.arc` (V12), `map_points` (a caller's own deformation of a glyph, computed at once: every glyph an editable
path).

## Decided after review

- **Primitive shapes only as eager `Path` constructors**; a Param on `scale` or `rotate` animates one.
- **`frames` in Python only**; Rust and JavaScript callers loop over `to_png`.
- **Any Param accepts a per-item list without declaring it**, read by `index` at the step that reads it. Recorded
  alternative: by the item the generator made, which would survive `split` and `only`.
- **Default paint**: an unstyled closed outline is filled black and an open line stroked; once a fill or stroke is set,
  only what is set is painted.
- **The design size is optional and inherited**; items whose sizes disagree raise `ParameterError`.
- **The reference pose** is every Param at its default, whatever `with_values` fixed.
- **The mesh** carries multiply as draw ranges and knockouts as booleans; a blurred shadow, a gradient, a blend beyond multiply, a clip and a mask raise.
- **Whole-number arguments** take expressions whose value is whole when drawn.
- **Frame export may rebuild the drawing per frame**; such a clip is many drawings, not one JSON.
- **Positions along a path are fractions 0..1**, closed outlines measured from their topmost point, every outline the
  library makes in one canonical orientation; `near=` moves the start where the top jumps ("Path").
- **A negative `scale` mirrors; zero raises.**
- **A stack writes `order` only on shapes that carry none**; `Drawing(x)` of one item and no size saves as `x`.
- **`Style` holds the paint as drawn**: `None` is not painted, `fill("none")` counts as set, and `shapes()` resolves the
  default paint.
- **A per-item number where an argument is read once is an error**, when loaded and when drawn.
- **Inputs carry an optional name** (`Font`, `Field.map`), and `from_json` takes their bytes by name; a drawing has no
  name of its own; defaults are part of the saved format ([formats.md](formats.md)).

## Open questions for the owner

None remain.
