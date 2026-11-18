//! The style steps of a drawing: `fill`, `stroke`, `opacity`, `blend`, `shadow`, `knockout`, `halftone`; and the same on a `Path`.

use super::Drawing;
use super::transform::Deg;
use crate::node::{Given, style};
use crate::{Blend, Cap, Color, Expr, Field, HalftoneStyle, Join, Mark, Paint, Path, Point, Screen, Tone};

/// A stroke: its paint, and the width, joins, caps and miter limit if not the defaults (2 pixels, round, round, 4). A
/// [`Color`] or a [`Gradient`](crate::Gradient) alone converts into one.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    paint: Paint,
    width: Option<Expr>,
    join: Option<Join>,
    cap: Option<Cap>,
    miter_limit: Option<Expr>,
    profile: Option<Vec<(Expr, Expr)>>,
}

impl Stroke {
    /// A stroke of `paint`, a colour or a gradient.
    #[must_use]
    pub fn new(paint: impl Into<Paint>) -> Self {
        Self {
            paint: paint.into(),
            width: None,
            join: None,
            cap: None,
            miter_limit: None,
            profile: None,
        }
    }

    /// This stroke `width` design pixels wide, above 0.
    #[must_use]
    pub fn width(mut self, width: impl Into<Expr>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// This stroke with its corners joined as `join`.
    #[must_use]
    pub fn join(mut self, join: Join) -> Self {
        self.join = Some(join);
        self
    }

    /// This stroke with the ends of an open outline finished as `cap`.
    #[must_use]
    pub fn cap(mut self, cap: Cap) -> Self {
        self.cap = Some(cap);
        self
    }

    /// This stroke with a mitred join cut to a bevel past `miter_limit` (SVG's ratio), at least 1.
    #[must_use]
    pub fn miter_limit(mut self, miter_limit: impl Into<Expr>) -> Self {
        self.miter_limit = Some(miter_limit.into());
        self
    }

    /// This stroke with its width `width` times the factor of `profile` at each place, as
    /// [`Path::tapered_outline`](crate::Path::tapered_outline) has it: `(position, factor)` stops, each number a number,
    /// a `Param` or an expression read for each shape. The stroke is then drawn as a filled region by every output,
    /// with round joins and ends (a miter or bevel join or a butt or square cap leaves the drawing with an
    /// [`error`](Drawing::error)). The outline is made from the path as it is when the drawing is drawn, so a later
    /// `slice` thins the tip where it now ends.
    #[must_use]
    pub fn profile<E: Into<Expr>>(mut self, profile: impl IntoIterator<Item = (E, E)>) -> Self {
        self.profile = Some(
            profile
                .into_iter()
                .map(|(at, factor)| (at.into(), factor.into()))
                .collect(),
        );
        self
    }
}

/// A drop shadow: what is left out is the default, no offset to the side, 8 pixels down, a blur of 16 and the colour
/// `#00000059`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shadow {
    dx: Option<Expr>,
    dy: Option<Expr>,
    blur: Option<Expr>,
    color: Option<Color>,
}

impl Shadow {
    /// A shadow with the default offset, blur and colour.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// This shadow `dx` design pixels to the right of its shape.
    #[must_use]
    pub fn dx(mut self, dx: impl Into<Expr>) -> Self {
        self.dx = Some(dx.into());
        self
    }

    /// This shadow `dy` design pixels below its shape.
    #[must_use]
    pub fn dy(mut self, dy: impl Into<Expr>) -> Self {
        self.dy = Some(dy.into());
        self
    }

    /// This shadow blurred by `blur` design pixels, at least 0: the radius of a CSS `box-shadow`, so the Gaussian's
    /// standard deviation is half of it. 0 is a hard copy.
    #[must_use]
    pub fn blur(mut self, blur: impl Into<Expr>) -> Self {
        self.blur = Some(blur.into());
        self
    }

    /// This shadow tinted `color`, which paints (not [`Color::NONE`]); its alpha is the shadow's strength.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A halftone fill: the dots' colour and tone, and what is left out takes the defaults: a `pitch` of 8 design pixels, an
/// `angle` of 45 degrees, round dots on a grid.
#[derive(Debug, Clone)]
pub struct Halftone {
    color: Color,
    tone: ToneGiven,
    pitch: Option<Expr>,
    angle: Option<Expr>,
    shape: Option<Mark>,
    aspect: Option<Expr>,
    screen: Option<Screen>,
    centre: Option<Point>,
    amplitude: Option<Expr>,
    wavelength: Option<Expr>,
}

/// The tone as the caller gave it.
#[derive(Debug, Clone)]
enum ToneGiven {
    Number(Expr),
    Field(Field),
}

impl Halftone {
    /// A screen of dots of `color`, `tone` from 0 (solid) to 1 (no dots): the share of each cell left bare. A number or
    /// an expression, which a per-item number may be.
    #[must_use]
    pub fn new(color: Color, tone: impl Into<Expr>) -> Self {
        Self::of(color, ToneGiven::Number(tone.into()))
    }

    /// A screen of dots of `color` whose tone is `tone`, a [`Field`] read at the centre of each dot (held to 0 to 1).
    #[must_use]
    pub fn field(color: Color, tone: Field) -> Self {
        Self::of(color, ToneGiven::Field(tone))
    }

    fn of(color: Color, tone: ToneGiven) -> Self {
        Self {
            color,
            tone,
            pitch: None,
            angle: None,
            shape: None,
            aspect: None,
            screen: None,
            centre: None,
            amplitude: None,
            wavelength: None,
        }
    }

    /// This screen with its dots `pitch` design pixels apart, above 0.
    #[must_use]
    pub fn pitch(mut self, pitch: impl Into<Expr>) -> Self {
        self.pitch = Some(pitch.into());
        self
    }

    /// This screen with its layout turned `angle` degrees clockwise on the screen about the origin of the design
    /// frame (rings have no direction and ignore it; a spiral and rays turn about their centre).
    #[must_use]
    pub fn angle<T: Into<Expr>>(mut self, angle: Deg<T>) -> Self {
        self.angle = Some(angle.0.into());
        self
    }

    /// This screen with dots of `shape` (default [`Mark::Dot`]).
    #[must_use]
    pub fn shape(mut self, shape: Mark) -> Self {
        self.shape = Some(shape);
        self
    }

    /// This screen with its ellipses `aspect` times as long as wide, at least 1; read by [`Mark::Ellipse`] only.
    #[must_use]
    pub fn aspect(mut self, aspect: impl Into<Expr>) -> Self {
        self.aspect = Some(aspect.into());
        self
    }

    /// This screen laid out as `screen` (default [`Screen::Grid`]).
    #[must_use]
    pub fn screen(mut self, screen: Screen) -> Self {
        self.screen = Some(screen);
        self
    }

    /// This screen about `centre`, for rings, a spiral and rays (default: the middle of the shape's bounding box); a
    /// grid or a wave has none, and a drawing given one is refused.
    #[must_use]
    pub fn centre(mut self, centre: impl Into<Point>) -> Self {
        self.centre = Some(centre.into());
        self
    }

    /// This screen's wave swinging its rows `amplitude` pitches (default 1); read by [`Screen::Wave`] only.
    #[must_use]
    pub fn amplitude(mut self, amplitude: impl Into<Expr>) -> Self {
        self.amplitude = Some(amplitude.into());
        self
    }

    /// This screen's wave `wavelength` pitches long (default 6), above 0; read by [`Screen::Wave`] only.
    #[must_use]
    pub fn wavelength(mut self, wavelength: impl Into<Expr>) -> Self {
        self.wavelength = Some(wavelength.into());
        self
    }
}

impl From<&HalftoneStyle> for Halftone {
    /// The screen a drawn style holds, to put into a drawing again.
    fn from(style: &HalftoneStyle) -> Self {
        let tone = match style.tone() {
            Tone::Level(level) => ToneGiven::Number(Expr::from(*level)),
            Tone::Field(field) => ToneGiven::Field((**field).clone()),
        };
        let mut halftone = Self::of(style.color(), tone);
        halftone.pitch = Some(Expr::from(style.pitch()));
        halftone.angle = Some(Expr::from(style.angle()));
        halftone.shape = Some(style.mark().clone());
        halftone.aspect = Some(Expr::from(style.aspect()));
        halftone.screen = Some(style.screen());
        halftone.centre = style.centre();
        halftone.amplitude = Some(Expr::from(style.amplitude()));
        halftone.wavelength = Some(Expr::from(style.wavelength()));
        halftone
    }
}

impl<P: Into<Paint>> From<P> for Stroke {
    fn from(paint: P) -> Self {
        Self::new(paint)
    }
}

impl Drawing {
    /// This drawing with every shape filled with `paint`, a [`Color`] ([`Color::NONE`] for no fill) or a
    /// [`Gradient`](crate::Gradient), whose geometry is in the design frame, not in the shape. Once a fill or a stroke
    /// is set, only what is set is painted.
    #[must_use]
    pub fn fill(&self, paint: impl Into<Paint>) -> Self {
        self.then(&style::Fill, vec![("color", Given::Paint(paint.into()))])
    }

    /// This drawing with every outline stroked: a [`Color`] or a [`Gradient`](crate::Gradient) for the default stroke,
    /// or a [`Stroke`].
    ///
    /// A colour of none, a width not above 0 and a miter limit below 1 leave the drawing with an
    /// [`error`](Drawing::error), or, when they read a Param, are refused when the drawing is drawn.
    #[must_use]
    pub fn stroke(&self, stroke: impl Into<Stroke>) -> Self {
        let stroke = stroke.into();
        let mut given = vec![("color", Given::Paint(stroke.paint))];
        if let Some(width) = stroke.width {
            given.push(("width", Given::Number(width)));
        }
        if let Some(join) = stroke.join {
            given.push(("join", Given::Text(join.name().to_owned())));
        }
        if let Some(cap) = stroke.cap {
            given.push(("cap", Given::Text(cap.name().to_owned())));
        }
        if let Some(limit) = stroke.miter_limit {
            given.push(("miter_limit", Given::Number(limit)));
        }
        if let Some(profile) = stroke.profile {
            given.push(("profile", Given::Pairs(profile)));
        }
        self.then(&style::Stroke, given)
    }

    /// This drawing with the opacity of every shape multiplied by `alpha`, from 0 to 1.
    #[must_use]
    pub fn opacity(&self, alpha: impl Into<Expr>) -> Self {
        self.then(&style::Opacity, vec![("alpha", Given::Number(alpha.into()))])
    }

    /// This drawing with every shape painted over what is below it as `mode`.
    #[must_use]
    pub fn blend(&self, mode: Blend) -> Self {
        self.then(&style::Blend, vec![("mode", Given::Text(mode.name().to_owned()))])
    }

    /// This drawing with a drop shadow under every shape, replacing any it has: the silhouette of the shape's fill and
    /// stroke, moved, blurred and tinted, painted behind the shape inside the group that its opacity and blend apply to.
    /// A shape with an empty path or nothing painted casts none.
    ///
    /// A `blur` below 0 or a colour of none leave the drawing with an [`error`](Drawing::error), or, when they read a
    /// Param, are refused when the drawing is drawn.
    #[must_use]
    pub fn shadow(&self, shadow: Shadow) -> Self {
        let mut given = Vec::new();
        for (name, number) in [("dx", shadow.dx), ("dy", shadow.dy), ("blur", shadow.blur)] {
            if let Some(number) = number {
                given.push((name, Given::Number(number)));
            }
        }
        if let Some(color) = shadow.color {
            given.push(("color", Given::Color(color)));
        }
        self.then(&style::Shadow, given)
    }

    /// This drawing with every shape filled with a screen of dots, replacing any fill it has (as [`Drawing::fill`]
    /// replaces a halftone). The dots lie on a lattice anchored to the origin of the design frame, not to the shape, so
    /// they stay put when the shape moves; a shape's stroke is drawn over them. A tone of 0 fills the shape solid and a
    /// tone of 1 draws nothing.
    ///
    /// A colour of none, a tone outside 0 to 1 and a pitch not above 0 leave the drawing with an
    /// [`error`](Drawing::error), or, when they read a Param, are refused when the drawing is drawn.
    #[must_use]
    pub fn halftone(&self, halftone: Halftone) -> Self {
        let mut given = vec![
            ("color", Given::Color(halftone.color)),
            (
                "tone",
                match halftone.tone {
                    ToneGiven::Number(tone) => Given::Number(tone),
                    ToneGiven::Field(field) => Given::Field(field),
                },
            ),
        ];
        for (name, number) in [
            ("pitch", halftone.pitch),
            ("angle", halftone.angle),
            ("aspect", halftone.aspect),
            ("amplitude", halftone.amplitude),
            ("wavelength", halftone.wavelength),
        ] {
            if let Some(number) = number {
                given.push((name, Given::Number(number)));
            }
        }
        match halftone.shape {
            Some(Mark::Path(path)) => {
                given.push(("shape", Given::Text("path".to_owned())));
                given.push(("mark", Given::Path((*path).clone())));
            }
            Some(other) => given.push(("shape", Given::Text(other.name().to_owned()))),
            None => {}
        }
        if let Some(screen) = halftone.screen {
            given.push(("screen", Given::Text(screen.name().to_owned())));
        }
        if let Some(centre) = halftone.centre {
            given.push(("centre", Given::Pair(Expr::from(centre.x), Expr::from(centre.y))));
        }
        self.then(&style::Halftone, given)
    }

    /// This drawing with every shape removing what is below it in the drawing instead of painting: the area its fill
    /// and stroke cover is cut out of the shapes before it, their shadows included; the shapes after it paint over the
    /// hole. A shape that paints nothing, or has an empty path, removes nothing.
    #[must_use]
    pub fn knockout(&self) -> Self {
        self.then(&style::Knockout, Vec::new())
    }
}

impl Path {
    /// A [`Drawing`] of this path filled with `paint`, a colour or a gradient.
    #[must_use]
    pub fn fill(&self, paint: impl Into<Paint>) -> Drawing {
        Drawing::from(self.clone()).fill(paint)
    }

    /// A [`Drawing`] of this path stroked.
    #[must_use]
    pub fn stroke(&self, stroke: impl Into<Stroke>) -> Drawing {
        Drawing::from(self.clone()).stroke(stroke)
    }

    /// A [`Drawing`] of this path with its opacity multiplied by `alpha`, from 0 to 1.
    #[must_use]
    pub fn opacity(&self, alpha: impl Into<Expr>) -> Drawing {
        Drawing::from(self.clone()).opacity(alpha)
    }

    /// A [`Drawing`] of this path painted over what is below it as `mode`.
    #[must_use]
    pub fn blend(&self, mode: Blend) -> Drawing {
        Drawing::from(self.clone()).blend(mode)
    }

    /// A [`Drawing`] of this path with a drop shadow.
    #[must_use]
    pub fn shadow(&self, shadow: Shadow) -> Drawing {
        Drawing::from(self.clone()).shadow(shadow)
    }

    /// A [`Drawing`] of this path filled with a screen of dots.
    #[must_use]
    pub fn halftone(&self, halftone: Halftone) -> Drawing {
        Drawing::from(self.clone()).halftone(halftone)
    }

    /// A [`Drawing`] of this path as a knockout.
    #[must_use]
    pub fn knockout(&self) -> Drawing {
        Drawing::from(self.clone()).knockout()
    }
}
