# The code API

Status: V1a, designed from the example programs in [examples/](examples/); waiting for the owner's review. Built so far,
in Rust and Python: `Path`, `Point`, `Rect`, `Path.map_points` and the errors (V2a); `Param`, `Expr`, `Shape`, `Style` and
`Drawing` with `translate`, `rotate`, `scale`, `fill`, `stroke`, `opacity`, `blend`, `only`, `without`, `with_values`,
`parameters()`, `shapes()` and `size`, and the style verbs on a `Path` (V2b). A `Shape`'s `style` has `fill`, `stroke`,
`stroke_width`, `join`, `cap`, `miter_limit`, `opacity` and `blend`; the rest of the members below arrive with their
steps. Also built: `text` (V5) with `width`, `fit` and `inside` (V6), and `along` (V6) on `Path` and `Drawing`, all in Rust
and Python, and `text` and `along` in a document; and the per-item numbers `by_index`, `by_order`, `random` and
`stagger`, and the steps `split`, `merge` and `slice` (V7), and the booleans `union`, `subtract`, `intersect`, `exclude`
and `resolve` (V8), in Rust, Python and a document. In Python `stroke`'s `width`, `join`, `cap` and `miter_limit` default to `None`, which the core reads as the
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
| [kinetic_type.py](examples/kinetic_type.py), [.rs](examples/kinetic_type.rs), [.js](examples/kinetic_type.js) | reference piece 1: variable-font axes, per-letter stagger, a caller's signal per letter, riso inks, a clip |
| [cut_paper.py](examples/cut_paper.py) | reference piece 2: a traced mask, offset layers, shadows, a draw-on outline, a sequence of maps |
| [hatched_poster.py](examples/hatched_poster.py) | reference piece 3: flow-field hatching by tone, rings of a distance field, a plotter file |
| [kinetic_type.py](../examples/kinetic_type.py) | the headline of piece 1 as far as it is built: a variable-font weight a letter, two multiply inks |
| [star.py](../examples/star.py) | building and measuring a `Path` |
| [ring.py](../examples/ring.py) | offsets and a boolean, eager on a `Path` and as steps on a `Drawing` |
| [cutout.py](../examples/cutout.py) | `resolve`, `subtract`, `intersect` and `exclude`: a headline cut out of a plate, the lens of two circles |
| [cut_paper_layers.py](../examples/cut_paper_layers.py) | `offset` layers of a headline nesting by a `step` Param, an `outline` cut line round them |
| [text_on_path.py](../examples/text_on_path.py) | text bent along a path, moving round it |
| [draw_on.py](../examples/draw_on.py) | draw-on by `progress`, staggered letter by letter |
| [marching_dashes.py](examples/marching_dashes.py) | a dash pattern with a moving phase |
| [wobble_outline.py](examples/wobble_outline.py) | a small noise wobble; a font axis as a described parameter |
| [glyph_scatter.py](../examples/glyph_scatter.py) | letters split into parts, scattered by seeded random numbers |
| [radial_repeat.py](examples/radial_repeat.py) | a radial repeater; a number per copy |
| [halftone.py](examples/halftone.py) | halftone dots from a tone map |
| [duotone.py](examples/duotone.py) | a gradient map and posterize on colours |
| [series_bars.py](../examples/series_bars.py) | a series of numbers as bars, with plain code |
| [frame_export.py](examples/frame_export.py) | a transparent PNG sequence and a clip from a function of time; a knockout |
| [mesh_upload.js](examples/mesh_upload.js) | a mesh in WebAssembly memory uploaded to WebGL, the pointer pattern |

## Four ideas

**`Path`** is a Bezier outline: one or more sub-paths, each open or closed. It is a plain value. Every verb on it
computes at once and returns a new `Path`; nothing changes the old one. This is the scripting layer: build, measure,
cut, combine and offset geometry the way Paper.js does, with callbacks where they help.

**`Shape`** is what a drawing is made of: one `Path` with a style (fill, stroke, opacity, blend mode, shadow, knockout)
and attributes (its index, and what its generator wrote: glyph, char, word, line, copy, level, part, order 0..1, the
origin it turns about). A shape is read, never built by hand: `drawing.shapes()` evaluates a drawing into a list of
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

Points are pairs: `(x, y)` tuples or `Point`s go in, `Point`s come out (`p.x`, `p.y`; `+` and `-` between points, `*` by
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
`ParameterError`: wrap the path in `btvec.Drawing(path)` first. `about` defaults to the shape's `origin` attribute (a
glyph's cap-height centre), else the centre of its bounds: on a stack of shapes, each turns or scales about its own
origin. To turn the whole stack as one, give `about=` a point, such as the frame's centre. `translate`, `rotate` and
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
| `outline` | `outline(width, *, join="round", cap="round", miter_limit=4, tolerance=0.01)` | the region a stroke of `width` paints, as a closed outline; caps `butt`, `round`, `square` | `ParameterError`: `width` not above 0 and within ±1e100, `miter_limit` below 1, `tolerance` and the result as `offset` |
| `drop_parts` | `drop_parts(*, smaller_than)` | without the parts whose area is below `smaller_than` square pixels | `smaller_than` below 0 |
| `fill_holes` | `fill_holes(*, smaller_than)` | with the holes whose area is below `smaller_than` filled | `smaller_than` below 0 |
| `slice` | `slice(start, end, *, near=None)` | the piece between two positions along it (draw-on: `slice(0, progress)`); on a `Drawing` each number is read per shape, so `text("WRITE").slice(0, stagger(progress, spread=0.6))` draws the letters on one after another (Rust: `drawing.slice(start, end)`, `drawing.slice_near(start, end, near)`); with `near`, a closed sub-path is measured from its point nearest `near`, as for every verb that takes it | a position outside 0..1; `start` above `end`; on a `Path`, `GeometryError`: an empty path (on a `Drawing` a shape with nothing in it stays) |
| `reverse` | `reverse()` | the same outline travelled the other way | |
| `dash` | `dash(pattern, *, phase=0, near=None)` | the pieces a dash pattern (`[on, off, ...]` in pixels) leaves; `phase` in cycles of the pattern, wrapping | an empty pattern; a length not above 0 |
| `resample` | `resample(spacing)` | straight segments with corners `spacing` pixels apart along it | `spacing` not above 0 |
| `simplify` | `simplify(*, tolerance=0.5)` | the fewest smooth curves (sharp corners kept) within `tolerance` pixels, so a dense polyline becomes a few smooth curves | `tolerance` not above 0 |
| `along` | `along(path, *, start=0, near=None)` | bent along `path`: a point `(x, y)` lands `start` lengths of `path`, plus `x` pixels, along it, and `y` pixels to the right of the way it runs there (`y` down: on a path running right, below it). `path` is walked as one length, its sub-paths in order, in the direction of the curve at that place (not of the straight edge it is flattened to). A closed `path` (every sub-path with a length closed) wraps, so a `cycle` Param turns shapes round it; any other goes on straight past its ends along the direction it ends or begins in, so no `start` or `x` is out of range. `near` as for `point_at`. The result is straight lines within 0.01 pixel of the true bend (each flattened edge is halved up to 8 times), counted against `MAX_SEGMENTS`; a sub-path keeps its closedness; the `origin` attribute is left where it was. An edge longer than half a closed `path` can be bent too coarsely: a known limit. Text on a path is `text(...).along(path)` | `GeometryError`: `path` empty or of no length; `ParameterError`: `start` NaN, infinite or sending a point beyond ±1e100, `near` not finite |
| `wobble` | `wobble(*, amplitude=4, wavelength=80, phase=0, seed=0)` | points displaced by smooth seeded noise of that amplitude and wavelength (pixels); `phase` in cycles moves it | `wavelength` not above 0 |
| `wave` | `wave(*, amplitude=8, wavelength=120, angle=0, phase=0)` | points displaced sideways by a sine travelling towards `angle` | `wavelength` not above 0 |
| `morph` | `morph(target, fraction)` | the outline `fraction` 0..1 of the way to `target`, points matched when their counts differ; on a `Drawing`, shapes pair by index | on a `Drawing`, different shape counts |
| `hatch` | `hatch(*, spacing=8, tone=None, flow=None, avoid=None)` | each region filled with evenly spaced open lines (below) | below |

A deformer's result is smooth at any size, however few points the outline had; V12 adds `twist` and `bulge` in the same
form.

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
- **Thin contours are dropped.** One no thicker than 4 ulps of the result's largest coordinate encloses less than a number
  holds (a spike of no width, a retraced curve's hair pieces, a sliver of 1e-13 beside a 600-wide shape). So are sub-paths
  of fewer than 3 corners, and the spike of an open polyline that doubles back.
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
`drawing.outline(2.0, Join::Round, Cap::Round, 4.0, 0.01)`.

Errors: `ParameterError` as the rows say; a `Drawing` names the step and the argument (`step 2 (offset).join`) and a draw
past `MAX_SEGMENTS` is refused as for the booleans. No offset raises `GeometryError`.

Known limits (the engine's, `docs/design-decisions.md`, "Offsets (V9)"): a `miter_limit` above about 23 acts as 23; a turn
under 5 degrees is bevelled; a round join or cap of a radius above about 80 times `tolerance` strays by up to 0.00012 of
the radius; a feature far below the extent of the shape or the offset is lost to rounding, so a `width` far below the
grid, or a `distance` beyond about 1e19 times the size of the shape, gives an empty path; a one-point sub-path paints
nothing in `outline` (`stroke` draws a dot).

### hatch

Evenly spaced lines following `flow` inside each region: no two closer than the spacing, each
line as long as the spacing allows, closed where a line meets its own start.

| Argument | Meaning |
|---|---|
| `spacing` | pixels between lines; or a pair `(at tone 0, at tone 1)`, read from `tone` |
| `tone` | a `Field` or a number; needed when `spacing` is a pair |
| `flow` | a `Flow`; default `Flow.angle(45)` (straight hatching) |
| `avoid` | a `Path` or `Drawing` the lines keep clear of |

The result is open lines, so set the stroke after it. Errors: `ParameterError` for a spacing not above 0, or a pair
without `tone`.

## Drawing

`Drawing(*items, size=None)`: the items (paths, shapes, drawings) stacked in order, the first at the bottom. `size` is
the design frame `(width, height)` in design pixels; outputs scale it to their size. Without one, a drawing takes the
size its items have; items whose sizes differ raise `ParameterError` naming both; with none at all, design pixels are
output pixels. A size is above 0 and at most 100,000 a side; others raise `ParameterError`. `Drawing(x)` of one item and no size draws and saves exactly as `x` does (a path becomes a one-shape
`Drawing`; a drawing is returned as it is).

### Items

| Step | Signature | Promise | Errors |
|---|---|---|---|
| `split` | `split()` | every shape split into its parts (as `Path.split`), each a shape of its own keeping the attributes and style, with `part` (from 0, within its shape) and its own bounds centre as `origin`, replacing one the shape carried | |
| `merge` | `merge(*, by=None)` | shapes with an equal attribute `by` (all shapes, if `None`) joined into one: their sub-paths in list order, not united (overlaps stay; `union` or `resolve` unites them), with the first's attributes and style, at the first's place; groups in the order their first members appear; shapes that lack `by` are a group | `by` that is no attribute name; `by` that no shape of the list carries (when drawn) |
| `only` | `only(**attributes)` | the shapes whose attributes equal these (a list means any of) | an attribute no shape carries |
| `without` | `without(**attributes)` | the other shapes | as `only` |
| `repeat` | `repeat(count, *, translate=(0, 0), rotate=0, scale=1, about=None)` | `count` copies, copy `i` transformed `i` times; writes `copy`, and `order` where a shape has none | `count` below 1 |

`index` is every shape's place in the current list. Generators write the rest: `text` writes `glyph`, `char`, `word`,
`line`, `order` and a typographic `origin`; `repeat` writes `copy` and, on shapes that carry none, `order` (0..1 over
the copies); `trace` writes `level`; `Drawing(...)` writes `order` on the shapes that carry none.
`order` runs 0..1 over the shapes the step that writes it writes it on, in list order (a single shape has 0; a stack
spreads it over its shapes that carry none, so a text keeps its per-letter order beside a rectangle) and survives
`split` and `only`; `index`
counts the current list. A shape whose path becomes empty stays in the list and draws nothing, so `index`, `order` and
per-item lists keep their meaning; only `only`, `without` and `merge` remove shapes. A shape with nothing in it that
`split` meets stays, with `part` 0. Rust: `drawing.split()`, `drawing.merge()` and `drawing.merge_by("word")`.

### Style

| Step | Signature | Promise | Errors |
|---|---|---|---|
| `fill` | `fill(color)` | fills each shape; `"none"` for no fill | an unreadable colour |
| `stroke` | `stroke(color, *, width=2, join="round", cap="round", miter_limit=4)` | strokes each outline | as `fill`; `width` not above 0; `miter_limit` below 1 |
| `halftone` | `halftone(color, *, tone, pitch=8, angle=45)` | fills each shape with a dot screen: dots `pitch` pixels apart on a grid at `angle`, each covering what `tone` (a number or a `Field`) asks (0 solid, 1 none) | as `fill`; `pitch` not above 0 |
| `opacity` | `opacity(alpha)` | multiplies opacity by `alpha` 0..1 | `alpha` outside 0..1 |
| `blend` | `blend(mode)` | paints with `"normal"` or `"multiply"` over what is below | another mode |
| `shadow` | `shadow(*, dx=0, dy=8, blur=16, color="#00000059")` | a drop shadow under each shape, offset and blurred in pixels | `blur` below 0 |
| `knockout` | `knockout()` | each shape removes what is below it in its drawing, instead of painting | |
| `gradient_map` | `gradient_map(colors)` | every fill and stroke colour replaced by its lightness read along `colors`, dark to light (two colours: a duotone) | fewer than 2 colours |
| `posterize` | `posterize(levels)` | every fill and stroke colour reduced to `levels` levels a channel | `levels` below 2 |

Colours are `"#rrggbb"` or `"#rrggbbaa"`. Until a fill or a stroke is set, a closed outline is filled black and an open
line stroked black at 2 pixels; once either is set, only what is set is painted. `fill("none")` counts as set.
`knockout` is the style form of a cut: it keeps both shapes and cuts through everything below, shadows included;
`subtract` makes new geometry (for a plotter, or for further verbs). Every style reaches SVG and pixels alike. Called on
a `Path`, a style step returns a `Drawing` of that path.

### Values and evaluation

| Method | Signature | Promise | Errors |
|---|---|---|---|
| `with_values` | `with_values(**values)` | the same drawing with these Params fixed; they leave `parameters()` | as outputs (below) |
| `parameters` | `parameters()` | every `Param` the drawing reads, in the order first used (the walk of [formats.md](formats.md), "Params") | two Params with one name that differ in a field (Params equal in all six fields are one) |
| `shapes` | `shapes(**values)` | the evaluated shapes, in design pixels | as outputs |
| `size` | property | the design frame, or `None` | |

## Shape

`drawing.shapes(**values)` returns `list[Shape]`: every shape the drawing evaluates to, in paint order (the bottom
first), after every step, in design pixels. A shape is a read-only value; `Drawing(*shapes)` puts shapes back into a
drawing with their style and attributes.

| Member | Type | Promise |
|---|---|---|
| `shape.path` | `Path` | its outline |
| `shape.style` | `Style` | the style as drawn: `fill` and `stroke` (`"#rrggbbaa"`, or `None` where nothing is painted; the default paint is resolved, so an unstyled closed outline has a black `fill`), `stroke_width`, `join`, `cap`, `miter_limit`, `halftone` (its colour, tone, pitch and angle, or `None`), `opacity`, `blend`, `shadow` (its `dx`, `dy`, `blur` and colour, or `None`), `knockout` (`bool`) |
| `shape.attributes` | `Mapping[str, int \| float \| str \| Point]` | `index` (`int`) always; then what generators wrote: `glyph` (`int`), `char` (`str`), `word` (`int`), `line` (`int`), `part` (`int`), `copy` (`int`), `level` (`float`), `order` (`float`), `origin` (`Point`) |

## Generators

Functions of the module that return a `Drawing`.

### text

`text(content, *, font, size=100, axes={}, features={}, tracking=0, leading=None, align="left", anchor=(0, 0),
width=None, fit=None, inside=None)`: one shape per drawn glyph (a space or a line break gives none), shaped with
kerning, ligatures and complex scripts, each outline the font's own contours (they may overlap and fill nonzero;
`resolve` removes the overlaps, at the cost of a step at every draw).

| Argument | Meaning |
|---|---|
| `content` | the text; `"\n"` breaks a line |
| `font` | a `Font` |
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
wrap), `line` the lines, and `origin` is the middle of the advance at half the cap height above the baseline.

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
`leading` not above 0 with `inside`, an `inside` shape with no outline, text that does not fit the shape, and more than
`MAX_GLYPHS` baselines tried.

### trace

`trace(field, *, levels=(0.5,), tolerance=1, before=(), after=(), sigma=1.2)`: one shape per level, outlining where
`field` is at or above it, with curves fitted within `tolerance` pixels. The edge of the field is padded, so a subject
cut by the frame gets no false edge. Writes `level` and `order`.

For a sequence of maps, pass the neighbouring maps in sequence order: `before` (up to this one) and `after` (from the
next). The edge is then averaged over them, nearer maps counting more and `sigma` (in maps) setting how far the
averaging reaches, and a hole or a part shows
only if a neighbour has it too. The library keeps no sequence.

Errors (`ParameterError`): levels not increasing, a neighbour of a different size or box, `tolerance` or `sigma` not
above 0.

The same verb draws contour lines: trace a `Field.distance` at several levels and stroke the result.

## Inputs

| Input | Signature | Promise | Errors |
|---|---|---|---|
| `Font` | `Font(data, *, name=None)` | a TrueType, OpenType or WOFF2 font, variable or not, from bytes (WOFF2 is unpacked; a saved drawing names the bytes given); `name` is what a saved drawing calls it | `data` not a font the library reads, a WOFF font, a damaged WOFF2 file, a font or an unpacked WOFF2 over 32 MiB |
| `Font.bundled` | `Font.bundled(name)` (Rust: `Font::bundled(name)?`) | a font that comes with the library, called `name` in a saved drawing: `"bricolage_grotesque"` or `"fraunces"`, SIL OFL 1.1. Its axes are the font file's, at its defaults (`fraunces` opens at `wght` 900, `opsz` 9, `WONK` 1; `bricolage_grotesque` at `wght` 800, `opsz` 96). A document that names it by `sha256` loads with no bytes supplied | `name` not bundled (the message lists them) |
| `Font.bundled_names` | `Font.bundled_names()` (Rust: `Font::bundled_names()`) | the names `Font.bundled` takes, in order | |
| `font.axes` | property | its variable axes by tag, each a `Param` named by the tag, with the font's range, default and name, in the font's order (Python: a new dict each time; Rust: `Font::axes()`, a list). Only an axis that can vary and is named by a `Param` name is listed: one pinned (minimum equal to maximum), with a tag that is not a name or a repeat of an earlier tag is left out, and is still set by its tag, and a refusal names it | |
| `font.features` | property | the OpenType feature tags its `GSUB` and `GPOS` tables list, sorted (Rust: `Font::features()`) | |
| `Field.map` | `Field.map(width, height, data, *, box=None, name=None)` | an 8-bit grayscale map, values 0 (black) to 1 (white), covering `box` `(x, y, width, height)` (default: one pixel a map pixel from the origin); beyond it, the nearest edge value | `len(data)` not `width × height` |
| `Field.ramp` | `Field.ramp(start, end)` | 0 at `start`, 1 at `end`, linear along the line between, held beyond | equal points |
| `Field.distance` | `Field.distance(drawing, *, normalized=False)` | from a `Drawing` or a `Path`: signed distance to the nearest outline in pixels, negative inside; normalised: inside each shape 0 on its outer edge to 1 on its holes' edges (its centre line when it has none), 0 outside | `GeometryError`: `drawing` empty when drawn |
| `Flow.angle` | `Flow.angle(angle)` | the same direction everywhere | |
| `Flow.noise` | `Flow.noise(*, wavelength=400, phase=0, seed=0)` | smooth seeded noise directions; `phase` in cycles moves it | `wavelength` not above 0 |
| `Flow.contour` | `Flow.contour(drawing)` | the direction of the nearest outline of `drawing` (a `Drawing` or a `Path`) | `GeometryError`: `drawing` empty when drawn |
| `flow.mix` | `mix(other, fraction)` | `fraction` 0..1 of the way to `other`; a direction and its opposite count as the same, so they never cancel | `fraction` outside 0..1 |

A field or a flow may read Params (a noise `phase`) and is part of the drawing that uses it.

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
(`ParameterError`): a bad name, a name an output option takes in Python (`into`, `pen_mm`), `min` not below `max`, a
default outside the range, a cyclic range other than one turn.

Arithmetic (`+`, `-`, `*`, `/`, unary `-`) on Params and numbers builds an `Expr`, which saves with the drawing. A
`Param` is an `Expr`, and so is every per-item number below, so they combine (`btvec.random(-10, 10, seed=7) * burst`).
Every number a step takes may be an `Expr`:

- a whole-number argument (`repeat`'s `count`, a `seed`, `posterize`'s `levels`) too, if its value is a whole number
  within ±(2⁵³ − 1) when drawn; any other value raises `ParameterError`, never rounded;
- a number inside a pair or a list (`translate=(dx, dy)`, `about`, `anchor`, `spacing=(a, b)`, a dash pattern, trace
  `levels`), component by component.

Except: these take literal numbers only: `Drawing`'s `size`, `Field.map`'s `width` and `height`, `text`'s `features`,
the values of `only`, `without` and `with_values`. A per-item number is refused (`ParameterError`) where an argument is
read once rather than per shape: `text`'s `size`, `tracking`, `leading`, `anchor` and `width` (its `axes` take one per
character), every number of `trace`, and every number of a `Field` or a `Flow`.

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
factor that fits (`min(width / frame width, height / frame height)`) and is centred; a drawing without a frame is drawn
at one output pixel to the design pixel from the top left. Content outside the frame is not clipped. `**values` give
Params their values by name; a Param not given takes its default.

`to_svg` writes coordinates and stroke widths in output pixels, not as a transform, so a drawing at twice the size has
numbers exactly twice as large; every number is the shortest text that reads back as the same value. A shape is one
`<path>` in paint order; its `opacity` applies to fill and stroke together (a group opacity: the pixels composite the
same way); a colour's alpha is `fill-opacity` or `stroke-opacity`; multiply is `mix-blend-mode:multiply`; a shape with
an empty path writes nothing. The root has `isolation:isolate`, so multiply blends within the picture and not with the
page around it.

`to_rgba` paints the shapes in order, each as one group: fill, then stroke, then the shape's `opacity` and blend apply
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

To look at a drawing, `uv run poe look FILE [--size WxH] [name=value ...]` draws a `.json` document, or a `.py` program
that leaves a `drawing` (as `examples/star.py` does), to `dist/look/<name>.png` and `<name>.svg` and prints the two
paths. The size is the drawing's own frame, else 800x600; `name=value` sets a Param. Exit code 1 and one line on stderr
for a missing or unreadable file, a program without `drawing`, a bad size or value, or an error from the library. It
opens nothing.

| Method | Signature | Promise |
|---|---|---|
| `to_svg` | `to_svg(width, height, /, **values)` | an SVG document of that size; shadows, halftone, knockouts and multiply kept, so it shows what the pixels show |
| `to_png` | `to_png(width, height, /, **values)` | PNG bytes, 8-bit RGBA, transparent where nothing is drawn |
| `to_rgba` | `to_rgba(width, height, /, **values)` | `width × height × 4` bytes, rows top to bottom, straight alpha |
| `to_plotter_svg` | `to_plotter_svg(width_mm, height_mm, /, *, pen_mm=0.5, **values)` | lines only, one layer per stroke colour (one pen each), ordered to cut pen-up travel and simplified below the pen's resolution (below) |
| `to_mesh` | `to_mesh(width, height, /, *, into=None, **values)` | triangles for the picture (below) |
| `to_json` | `to_json()` | the drawing as JSON: the same tree written down |
| `Drawing.from_json` | `Drawing.from_json(text, *, inputs=None)` | the drawing back, given the bytes (`bytes` or `bytearray`) of its inputs by name, a dict; it draws to identical bytes. A static method. In Rust `Drawing::from_json(text, &Inputs::new().set("anybody", bytes))`, in JavaScript `Drawing.fromJson(text, { anybody: bytes })` with a `Uint8Array` each |
| `drawing_schema` | `btvec.drawing_schema()` | the JSON Schema of the format, as text, generated from the registered calls |

In JavaScript (the package built from `crates/wasm`) a document loads with `Drawing.fromJson(text, inputs?)` (`inputs` an object of names to `Uint8Array`s, none for a document without fonts); a drawing has
`toJson()`, `parameters()` (plain objects `{ name, default, min, max, unit, meaning }`; `unit` is `""` for none), `size`
(`[w, h]` or `undefined`) and the outputs, each `(width, height, values?)` with `values` an object of Param names to a
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
index count, blend mode: 0 normal, 1 multiply), in paint order, so a page draws each range with its blending.
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
(a boolean while meshing), halftone dots are triangles. A shadow with a blur above 0 has no triangle form: `to_mesh`
raises `ParameterError` naming the step, until V10 decides how a display draws it (it needs an offscreen layer and blur
passes, which spike X1 did not build).

### Plotter

A pen draws lines, so `to_plotter_svg` says what becomes of everything else: a fill becomes hatching at `pen_mm`
spacing; halftone dots become circles; a knockout is applied as a boolean; a shape at opacity 0 is not drawn. Shadows,
any other opacity and blend modes have no pen form and are left out.

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
is computed. Depth is bounded: drawings nested more than 128 deep are refused when built, an expression more than 128 levels
deep is refused when used (printing and comparing it are safe, however long a loop made it), and `from_json` refuses a
document nested more than 128 levels. All of it needs a thread stack of at least 256 KB (a Python thread has 512 KB on
macOS). In Python and JavaScript the error points at the call that caused it; in Rust, eager calls return a
`Result` and a drawing's chain reports its first error at the first draw ("Languages").

## Conventions

- **Names in the field's own terms.** `amplitude`, `wavelength`, `pitch`, `spacing`, `tracking`, `leading`, `phase`,
  `seed`, `tolerance`; never `strength`, `intensity`, `factor`, `amount`, `val` or `k`. One name per concept: a
  position along a path is `at` (or `start`, `end`), a pivot is `about`, an angle is `angle` (`repeat`'s keywords
  mirror the verbs: `rotate=`), "this fraction of the way to the other" is `fraction`, a colour argument is `color`.
- **Units.** Design pixels for lengths, scaled with the output, so no caller ever scales. Degrees, clockwise, for
  angles, wrapping. Cycles for phases, wrapping. Fractions 0..1 for positions along a path and for progress. Thousandths
  of an em for tracking; font axes in the font's own units; areas in square pixels; millimetres only in the plotter
  output. A number is a state, never a duration.
- **Sampling in design pixels.** Everything the library samples internally (tracing an analytic field, resampling
  before a deformer, flattening curves for pixels and the mesh, seeding hatch lines) uses steps in design pixels, each a
  named constant, never the output size; a map is sampled at its own pixels. A drawing has the same geometry at any
  output size: only rasterising depends on it.
- **Arguments.** What a call cannot do without is positional (`offset(8)`, `rotate(30)`), unless its keyword makes
  the call read as a sentence (`drop_parts(smaller_than=1500)`, `text("NO", font=font)`, `halftone(INK, tone=photo)`);
  every option is keyword-only (`wobble(amplitude=3)`).
- **Defaults look good** with no tuning: a default comes from a gallery piece, not from a round number. The defaults
  written here (leading 1.2 × size, pitch 8, shadow 8 and 16, wavelengths 80, 120 and 400...) are placeholders until the
  batch that builds the step tunes them with its gallery piece.
- **Reading as a sentence.** `plane.subtract(letters)`, `figure.drop_parts(smaller_than=1500)`,
  `letters.only(char="O")`, `petal.repeat(12, rotate=30)`. Related verbs sit together in the tables above.
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

## Languages

The same names in every language, in each language's case: `snake_case` in Python and Rust, `camelCase` in JavaScript
(`toPng`, `byIndex`, `outCubic`).

| | Python | Rust | JavaScript |
|---|---|---|---|
| Constructors | `btvec.Drawing(a, b, size=(w, h))`, `btvec.Font(data)`, `btvec.Param("rise", default=0, min=0, max=1)` | `Drawing::new([a, b]).with_size(w, h)`, `Drawing::from(path)`, `Font::new(data)?`, `Param::new("rise", 0.0, 0.0, 1.0)?.unit(Unit::Fraction)?` | `new btvec.Drawing([a, b], { size: [w, h] })`, `new btvec.Font(data)`, `new btvec.Param("rise", { default: 0, min: 0, max: 1 })` |
| Options | keyword arguments | a value named after the step or function, built with `new` and setters (`Text::new(&font).size(560.0)`, `Random::new(-10.0, 10.0).seed(7)`); the required argument alone converts into it (`.rotate(Deg(30.0))`, `.stroke(color)`, `.scale(2.0)`; `Deg` takes anything that converts into an expression: `Deg(Random::new(-10.0, 10.0).seed(7) * &burst)`); built so far: `Rotate`, `ScaleBy`, `Stroke`, and `Select` for `only` and `without` (`Select::new().attribute("char", 'O')`) | an options object last (`{ width: 1.5 }`) |
| Expressions | operators: `1 + 0.08 * pulse` | operators on `Param`, `Expr` and their references, so nothing is cloned: `1.0 + 0.08 * &pulse`, `0.6 * &misregistration` | methods (`add`, `sub`, `mul`, `div`, `neg`), read in source order: `pulse.mul(0.08).add(1)`; a number that comes first is lifted once with `btvec.expr(n)`: `btvec.expr(1).sub(rise).mul(HEIGHT)` |
| Draw-time values | keyword arguments: `to_png(1080, 1350, weight=820)` | `&Values::new().set("weight", 820.0)` | an object: `toPng(1080, 1350, { weight: 820 })` |
| Output options | keywords beside the values: `to_mesh(w, h, into=mesh, **values)`, `to_plotter_svg(w, h, pen_mm=0.5, **values)` (so no Param may take those names) | a last argument after the values: `to_mesh(size, &values, &mut mesh)`, `to_plotter_svg(Size::mm(300.0, 375.0), &values, Plotter::new().pen_mm(0.5))` | an options object after the values: `toPlotterSvg(wMm, hMm, values, { penMm: 0.5 })`; a `Mesh` is lent as its own argument: `toMeshInto(mesh, w, h, values)` |
| Units as types | | `Deg(..)` for every angle of a `Drawing` step (`rotate`, `repeat`'s `rotate`, `Flow::angle`, `halftone`'s and `wave`'s `angle`); a `Path` method takes plain degrees (`rotate(30.0, None)?`); `Size::px(w, h)` (whole output pixels, else an error at the draw) and `Size::mm(w, h)` for output sizes | |
| Colours | `"#ff4f9a"` | `Color::hex(0xff4f9a)`, `Color::NONE` | `"#ff4f9a"` |
| Errors | exceptions | `Result<_, btvec::Error>`. On a `Path`, a constructor or verb that can fail returns a `Result` (`Path::rect(..)?`, `path.offset(8.0)?`) and one that cannot returns the value (`reverse`, `split`, `length`); a verb that takes a plain number returns a `Result`, as a NaN or an infinity is a `ParameterError` (`path.translate(1.0, 2.0)?`); a path is built from `Path::start(point)`, so no segment comes before a start. `Error` is `#[non_exhaustive]`: `Error::Parameter { parameter, message }` and `Error::Geometry { verb, message }`, owned strings so a drawing can prefix the step. A `Drawing` chain keeps its first error and returns it from the first draw, `to_json` or `parameters()`, naming the step by its place in the chain and its verb (`step 3 (rotate): ...`); `parameters()` is the early check, so no `?` per step | thrown `Error` objects whose `name` is `"ParameterError"`, `"GeometryError"` or `"FormatError"` and whose message is the core's (the same text as Python's). No classes to `instanceof`: wasm-bindgen cannot define subclasses; the `name` is the contract. A value of a type a call cannot read is a plain `TypeError` |
| Optional `Path` arguments | keywords: `point_at(0.5, near=p)`, `rotate(30, about=p)`, `Path.rect(0, 0, w, h, radius=8)`, `scale((2, 3))` | plain arguments, `None` for the default: `point_at(0.5, Some(p))?`, `rotate(30.0, None)?`, `Path::rect(0.0, 0.0, w, h, 8.0)?`; `scale` takes a number or a pair through `Scale`: `scale(2.0, None)?`, `scale((2.0, 3.0), None)?`; `Path::polyline(points, closed)` and `Path::star(center, outer, inner, points)` take their options in place; `map_points` takes `FnMut(Point) -> Result<Point, Error>` | an options object last: `pointAt(0.5, { near: p })`, `scale([2, 3])` |
| Bytes in | `bytes` | `Vec<u8>` | `Uint8Array` |
| Frame export | `btvec.frames` | the caller's loop over `to_png` | the caller's loop over `toRgba` or `toMesh` |

The kinetic type piece, the part that builds the drawing, in all three (whole programs:
[Python](examples/kinetic_type.py), [Rust](examples/kinetic_type.rs), [JavaScript](examples/kinetic_type.js)):

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
    .scale(1 + 0.08 * pulse)
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
png = piece.to_png(1080, 1350, rise=1, ink=1, weight=620, burst=0.4, pulse=[0.9, 0.6, 0.3, 0.1, 0, 0, 0])
```

```rust
let letters = btvec::text(
    "NO\nSLEEP",
    Text::new(&font).size(560.0).axis("wght", &weight + 120.0 * &pulse).leading(520.0)
        .anchor((WIDTH / 2.0, 600.0)).align(Align::Center).width(980.0).fit("wdth"),
)
.scale(1.0 + 0.08 * &pulse)
.rotate(Deg(Random::new(-10.0, 10.0).seed(7) * &burst))
.translate(Random::new(-30.0, 30.0).seed(8) * &burst, Random::new(-50.0, 50.0).seed(9) * &burst);
let frame = Path::rect(0.0, 0.0, WIDTH, HEIGHT)?;
let misregistration = 3.0 + 14.0 * &burst;
let paper = frame.fill(PAPER);
let blue = Drawing::from(frame).translate(0.0, HEIGHT * (1.0 - &rise))
    .subtract(&letters.translate(&misregistration, 0.6 * &misregistration))
    .fill(BLUE).blend(Blend::Multiply);
let pink = letters.fill(PINK).blend(Blend::Multiply).opacity(&ink);
let hairlines = letters.stroke(Stroke::new(INK).width(1.5)).opacity(1.0 - &ink);
let piece = Drawing::new([paper, blue, pink, hairlines]).with_size(WIDTH, HEIGHT);
let values = Values::new().set("rise", 1.0).set("ink", 1.0).set("weight", 620.0).set("burst", 0.4)
    .set("pulse", vec![0.9, 0.6, 0.3, 0.1, 0.0, 0.0, 0.0]);
let png = piece.to_png(Size::px(WIDTH, HEIGHT), &values)?;
```

```js
const letters = btvec
  .text("NO\nSLEEP", { font, size: 560, axes: { wght: weight.add(pulse.mul(120)) }, leading: 520,
                       anchor: [WIDTH / 2, 600], align: "center", width: 980, fit: "wdth" })
  .scale(pulse.mul(0.08).add(1))
  .rotate(btvec.random(-10, 10, { seed: 7 }).mul(burst))
  .translate(btvec.random(-30, 30, { seed: 8 }).mul(burst), btvec.random(-50, 50, { seed: 9 }).mul(burst));
const frame = btvec.Path.rect(0, 0, WIDTH, HEIGHT);
const misregistration = burst.mul(14).add(3);
const blue = new btvec.Drawing([frame]).translate(0, btvec.expr(1).sub(rise).mul(HEIGHT))
  .subtract(letters.translate(misregistration, misregistration.mul(0.6))).fill(BLUE).blend("multiply");
const pink = letters.fill(PINK).blend("multiply").opacity(ink);
const hairlines = letters.stroke(INK, { width: 1.5 }).opacity(btvec.expr(1).sub(ink));
const piece = new btvec.Drawing([frame.fill(PAPER), blue, pink, hairlines], { size: [WIDTH, HEIGHT] });
const values = { rise: 1, ink: 1, weight: 620, burst: 0.4, pulse: [0.9, 0.6, 0.3, 0.1, 0, 0, 0] };
const rgba = piece.toRgba(1080, 1350, values);
```

## Coverage of the prototypes' needs

Every ranked need in [prototype-findings.md](prototype-findings.md), "What the library needs, ranked":

| # | Need | Where the API expresses it |
|---|---|---|
| 1 | Per-glyph variable-font layout that re-flows; tracking, stacked lines, fitting at a reference pose, outlines cached by axis location | `text(axes=..., tracking=, leading=, width=, fit=)`, per-item numbers in `axes`; caching is internal |
| 2 | Evenly spaced streamlines with spacing from a tone, region masks, obstacles, closed loops; line fields and distance fields | `hatch(spacing=(a, b), tone=, flow=, avoid=)` inside each shape; `Flow.noise`, `Flow.contour`, `flow.mix`; `Field.distance` |
| 3 | Steady tracing over a sequence of maps: smoothing and hole persistence | `trace(before=, after=, sigma=)` |
| 4 | Booleans that hold on overlapping, rotated glyphs; ink layers from them | `union`, `subtract`, `intersect`, `exclude`; glyph overlaps removed by `resolve`; inks as drawings with `blend("multiply")` ([kinetic_type.py](examples/kinetic_type.py)) |
| 5 | Shrink and grow per layer; dropping small parts and filling small holes | `offset(r).offset(-r)`, `offset(-r).offset(r)`; `drop_parts`, `fill_holes` |
| 6 | Per-item transforms about a typographic origin; seeded randomness and stagger by index as one mechanism | `translate`, `rotate`, `scale` about each shape's origin; `by_index`, `by_order`, `random`, `stagger`; lists at draw time |
| 7 | Curve fitting through dense polylines | `simplify`; `trace(tolerance=)` |
| 8 | One picture to both pixels and SVG, every effect kept | `to_svg`, `to_png`, `to_rgba` |
| 9 | A fast halftone fill | `halftone` |
| 10 | Plotter export: stroke only, ordered, simplified, solids hatched, a layer per pen | `to_plotter_svg` |
| 11 | A variable-width stroke, tapered at free ends | gap: an effects-track verb, named with its gallery piece |
| 12 | Partial path by length from a stable start | `slice(0, progress)`: every outline the library makes in one orientation and order, closed ones measured from the topmost point, or from the point nearest `near=` where the top jumps |
| 13 | Fitting text to a box by a variable axis, with a clear signal when the axis runs out | `text(width=, fit=)`; `ParameterError` naming the line and the width reached |

Also from the findings: draw-on (`slice`), the riso misregistration that grows on a hit (an expression,
`3 + 14 * burst`), the accent letter (`only`, `without`), type under the paper (stacking order), paper that slides
(`translate` by a Param). Gaps beyond the ranked list: a multi-colour trace (a map of colour labels; V14 names it) and a
re-drawable series node (a series is plain code now, [series_bars.py](../examples/series_bars.py); the effects track adds a
node if a piece needs one).

Verbs no example uses yet, and what needs them: `merge` (words and lines as units), `reverse` (V11: a draw-on the other way round), `locate`
(hit-testing for an editor), `Flow.contour` and
`flow.mix` (V15: the findings' line fields, noise mixed with a glyph's contour), `wave` (V12; the effects track's
deformers), `morph` (V13; glyph-to-shape morph on the effects track), `resample` (V11; dots spaced evenly along a path
on the effects track), `map_points` (a caller's own deformation of a glyph, computed at once: every glyph an editable
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
- **The mesh** carries multiply as draw ranges and knockouts as booleans; only a blurred shadow raises, until V10.
- **Whole-number arguments** take expressions whose value is whole when drawn.
- **JavaScript expressions** lift a leading number with `btvec.expr(n)`.
- **Frame export may rebuild the drawing per frame**; such a clip is many drawings, not one JSON.
- **Positions along a path are fractions 0..1**, closed outlines measured from their topmost point, every outline the
  library makes in one canonical orientation; `near=` moves the start where the top jumps ("Path").
- **A negative `scale` mirrors; zero raises.**
- **A stack and `repeat` write `order` only on shapes that carry none**; `Drawing(x)` of one item and no size saves as `x`.
- **`Style` holds the paint as drawn**: `None` is not painted, `fill("none")` counts as set, and `shapes()` resolves the
  default paint.
- **A per-item number where an argument is read once is an error**, when loaded and when drawn.
- **Inputs carry an optional name** (`Font`, `Field.map`), and `from_json` takes their bytes by name; a drawing has no
  name of its own; defaults are part of the saved format ([formats.md](formats.md)).

## Open questions for the owner

None remain.
