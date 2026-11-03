//! The fonts the tests draw with (`tests/fonts/`).
//!
//! `fraunces-subset.ttf` is Fraunces (SIL OFL 1.1, `tests/fonts/OFL.txt`), a variable font cut down to the glyphs the
//! tests need and to one axis. Made from the font file `Fraunces[SOFT,WONK,opsz,wght].ttf` of the Google Fonts
//! repository with fontTools 4.x:
//!
//! ```text
//! fonttools varLib.instancer Fraunces[SOFT,WONK,opsz,wght].ttf opsz=14 SOFT=0 WONK=0 wght=300:400:700 -o instance.ttf
//! pyftsubset instance.ttf --unicodes="U+0020,U+002D,U+002E,U+0041,U+0043,U+0046,U+0048,U+0049,U+0054,U+0056,\
//!   U+0065,U+0066,U+0069,U+006C,U+006F,U+00E9" --layout-features='kern,liga,rvrn' --name-IDs='*' --notdef-outline \
//!   --output-file=fraunces-subset.ttf
//! ```
//!
//! It has 2000 units to the em, `H` 1656 wide, a kerning pair (`A` `V`: 1334 of 1424), the `fi` ligature as one glyph
//! of 1333, and `é` as a composite of `e` and an accent.
//!
//! `fraunces-subset.woff2` and `fraunces-subset.woff` are that file in the two web containers, made by fontTools (with
//! `brotli`) and lossless (`recalcTimestamp=False` keeps the `head` table): `TTFont(path, recalcTimestamp=False)`, then
//! `font.flavor = "woff2"` (or `"woff"`) and `font.save(...)`.
//!
//! `synthetic.ttf` is made by this script (`uvx --from fonttools python script.py`): `A` is a chain of 64 composites
//! (the deepest the reader draws), each placing the one before with a scaled offset; `B` is a glyph of 3999 straight
//! segments; `C` is a square that the `kern` feature moves by (50, 20) without moving the pen (`HarfBuzz`: `x_advance` 500,
//! `x_offset` 50, `y_offset` 20).
//!
//! ```python
//! import math
//! from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
//! from fontTools.fontBuilder import FontBuilder
//! from fontTools.pens.ttGlyphPen import TTGlyphPen
//! from fontTools.ttLib.tables._g_l_y_f import GlyphComponent, Glyph, SCALED_COMPONENT_OFFSET
//! DEPTH = 64   # the deepest nesting of components read-fonts draws
//! POINTS = 4000  # the corners of the heavy glyph
//! fb = FontBuilder(1000, isTTF=True)
//! names = [".notdef"] + [f"g{i}" for i in range(DEPTH + 1)] + ["heavy", "C"]
//! fb.setupGlyphOrder(names)
//! fb.setupCharacterMap({0x41: f"g{DEPTH}", 0x42: "heavy", 0x43: "C"})
//! pen = TTGlyphPen(None)
//! pen.moveTo((0, 0)); pen.lineTo((0, 100)); pen.lineTo((100, 100)); pen.lineTo((100, 0)); pen.closePath()
//! glyphs = {".notdef": TTGlyphPen(None).glyph(), "g0": pen.glyph()}
//! for i in range(1, DEPTH + 1):
//!     comp = GlyphComponent()
//!     comp.glyphName = f"g{i-1}"
//!     comp.x, comp.y = 10, 0
//!     comp.flags = SCALED_COMPONENT_OFFSET
//!     comp.transform = [[0.98, 0.0], [0.0, 1.0]]
//!     g = Glyph()
//!     g.numberOfContours = -1
//!     g.components = [comp]
//!     glyphs[f"g{i}"] = g
//! pen = TTGlyphPen(None)
//! corners = [(round(500 + 400 * math.cos(2 * math.pi * k / POINTS)), round(400 + 400 * math.sin(2 * math.pi * k / POINTS))) for k in range(POINTS)]
//! pen.moveTo(corners[0])
//! for corner in corners[1:]:
//!     pen.lineTo(corner)
//! pen.closePath()
//! glyphs["heavy"] = pen.glyph()
//! glyphs["C"] = glyphs["g0"]  # a square, moved by the kern feature
//! fb.setupGlyf(glyphs)
//! fb.setupHorizontalMetrics({n: (500, 0) for n in names})
//! fb.setupHorizontalHeader(ascent=800, descent=-200)
//! fb.setupNameTable({"familyName": "Synthetic", "styleName": "Regular"})
//! fb.setupOS2()
//! fb.setupPost()
//! addOpenTypeFeaturesFromString(fb.font, "feature kern { pos C <50 20 0 0>; } kern;")
//! fb.save("synthetic.ttf")
//! ```
//!
//! `marks.ttf` (`uvx --from fonttools python script.py`, 1000 to the em): `B` is a 400 square with an advance of 500, `acute`
//! (U+0301) a 100 by 50 rectangle with no advance that a GPOS `mark` feature puts over `B` (`HarfBuzz`: `'B\u0301'` is
//! `[('B', 0, 500, 0, 0), ('acute', 0, 0, -350, 450)]`, both in cluster 0).
//!
//! ```python
//! from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
//! from fontTools.fontBuilder import FontBuilder
//! from fontTools.pens.ttGlyphPen import TTGlyphPen
//! fb = FontBuilder(1000, isTTF=True)
//! fb.setupGlyphOrder([".notdef", "B", "acute"])
//! fb.setupCharacterMap({0x42: "B", 0x301: "acute"})
//! def square(width, height):
//!     pen = TTGlyphPen(None)
//!     pen.moveTo((0, 0)); pen.lineTo((0, height)); pen.lineTo((width, height)); pen.lineTo((width, 0)); pen.closePath()
//!     return pen.glyph()
//! fb.setupGlyf({".notdef": TTGlyphPen(None).glyph(), "B": square(400, 400), "acute": square(100, 50)})
//! fb.setupHorizontalMetrics({".notdef": (500, 0), "B": (500, 0), "acute": (0, 0)})
//! fb.setupHorizontalHeader(ascent=800, descent=-200)
//! fb.setupNameTable({"familyName": "Marks", "styleName": "Regular"})
//! fb.setupOS2()
//! fb.setupPost()
//! addOpenTypeFeaturesFromString(fb.font, "languagesystem DFLT dflt;\n"
//!     "markClass [acute] <anchor 50 0> @TOP;\n"
//!     "feature mark { pos base B <anchor 200 450> mark @TOP; } mark;\n"
//!     "table GDEF { GlyphClassDef [B], , [acute], ; } GDEF;\n")
//! fb.save("marks.ttf")
//! ```
#![allow(dead_code, reason = "each test file uses a different part")]

use bullterrier_vector::{Error, Font};

pub(crate) const FRAUNCES: &[u8] = include_bytes!("../fonts/fraunces-subset.ttf");
pub(crate) const FRAUNCES_WOFF2: &[u8] = include_bytes!("../fonts/fraunces-subset.woff2");
pub(crate) const FRAUNCES_WOFF: &[u8] = include_bytes!("../fonts/fraunces-subset.woff");
pub(crate) const SYNTHETIC: &[u8] = include_bytes!("../fonts/synthetic.ttf");
pub(crate) const MARKS: &[u8] = include_bytes!("../fonts/marks.ttf");

pub(crate) fn fraunces() -> Result<Font, Error> {
    Font::new(FRAUNCES)
}

pub(crate) fn synthetic() -> Result<Font, Error> {
    Font::new(SYNTHETIC)
}

pub(crate) fn marks() -> Result<Font, Error> {
    Font::new(MARKS)
}
