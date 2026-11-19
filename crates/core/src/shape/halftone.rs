//! The halftone fill of a shape as the style holds it: the dots' colour and tone, and the lattice they sit on.

use super::color::Color;
use super::style::refused;
use crate::geometry::Point;
use crate::key::{Key, Keyed};
use crate::lines::Family;
use crate::{Error, Field, Path};
use std::sync::Arc;

/// The tone of a halftone: how much of each cell the dot leaves bare, from 0 (solid) to 1 (no dot).
#[derive(Debug, Clone)]
pub enum Tone {
    /// One number everywhere.
    Level(f64),
    /// A field, read at the center of each dot.
    Field(Arc<Field>),
}

impl Tone {
    /// The number, for a tone that is one.
    #[must_use]
    pub fn level(&self) -> Option<f64> {
        match self {
            Self::Level(level) => Some(*level),
            Self::Field(_) => None,
        }
    }

    /// The field, for a tone that is one.
    #[must_use]
    pub fn field(&self) -> Option<&Field> {
        match self {
            Self::Level(_) => None,
            Self::Field(field) => Some(field),
        }
    }

    /// The tone at `point`, in design pixels, as drawn: a field's value is held to 0 to 1.
    #[must_use]
    pub fn at(&self, point: impl Into<Point>) -> f64 {
        match self {
            Self::Level(level) => *level,
            Self::Field(field) => field.at(point).clamp(0.0, 1.0),
        }
    }
}

impl PartialEq for Tone {
    /// Two numbers are equal when the numbers are; two fields when they are made alike, a distance field when the
    /// outline it measures to is the same.
    fn eq(&self, other: &Self) -> bool {
        crate::key::key_of(self) == crate::key::key_of(other)
    }
}

/// What a dot of a halftone is.
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    /// A round dot.
    Dot,
    /// A square whose sides run along the rows.
    Square,
    /// An ellipse, its long axis along the rows, `aspect` times as long as its short one.
    Ellipse,
    /// A square turned 45 degrees to the rows: a diamond, which touches its neighbours at tone 0.5 and past it fills
    /// the cell but for its corners.
    Diamond,
    /// The spot function of print screens: round in highlights, a diamond at tone 0.5, a round hole in shadows.
    Euclidean,
    /// A closed outline of the caller's, in its own units: its first sub-path, put on its centroid and scaled to the
    /// dot's area, its x axis along the rows.
    Path(Arc<Path>),
}

impl Mark {
    /// The words a document uses for the marks that are words, in the order of the variants; a caller's outline is no word.
    pub(crate) const NAMES: &'static [&'static str] = &["dot", "square", "ellipse", "diamond", "euclidean"];

    /// The mark of the caller's outline `path`.
    #[must_use]
    pub fn path(path: Path) -> Self {
        Self::Path(Arc::new(path))
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "dot" => Some(Self::Dot),
            "square" => Some(Self::Square),
            "ellipse" => Some(Self::Ellipse),
            "diamond" => Some(Self::Diamond),
            "euclidean" => Some(Self::Euclidean),
            _ => None,
        }
    }

    /// The mark a word names: `"dot"`, `"square"`, `"ellipse"`, `"diamond"` or `"euclidean"`. A caller's outline is
    /// [`Mark::path`].
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `shape` and listing the words.
    pub fn parse(name: &str) -> Result<Self, Error> {
        Self::from_name(name).ok_or_else(|| refused("shape", Self::NAMES, name))
    }

    /// The refusal of the optional argument `argument` (`aspect`) if this dot does not read it: only an ellipse does.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `argument`.
    pub(crate) fn refuse_unread(&self, argument: &str) -> Result<(), Error> {
        if argument == "aspect" && *self != Self::Ellipse {
            return Err(Error::parameter(
                argument,
                format!("is not read by the shape \"{}\"", self.name()),
            ));
        }
        Ok(())
    }

    /// The word a document uses.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Dot => "dot",
            Self::Square => "square",
            Self::Ellipse => "ellipse",
            Self::Diamond => "diamond",
            Self::Euclidean => "euclidean",
            Self::Path(_) => "path",
        }
    }
}

/// A halftone fill: a screen of dots in `color` over the shape, each as big as its `tone` says (0 solid, 1 none), on
/// a layout of rows `spacing` apart that `angle` turns.
#[derive(Debug, Clone, PartialEq)]
pub struct HalftoneStyle {
    pub(crate) color: Color,
    pub(crate) tone: Tone,
    pub(crate) spacing: f64,
    pub(crate) angle: f64,
    pub(crate) mark: Mark,
    pub(crate) aspect: f64,
    pub(crate) family: Family,
    pub(crate) center: Option<Point>,
    pub(crate) amplitude: f64,
    pub(crate) wavelength: f64,
}

impl HalftoneStyle {
    /// The colour of the dots, its alpha included.
    #[must_use]
    pub fn color(&self) -> Color {
        self.color
    }

    /// The tone from 0 to 1: the share of each cell the dot leaves bare, so 0 is solid and 1 has no dots. A number, or
    /// a field read at the center of each dot (see [`Tone`]).
    #[must_use]
    pub fn tone(&self) -> &Tone {
        &self.tone
    }

    /// The distance between neighbouring rows and dots in design pixels, above 0.
    #[must_use]
    pub fn spacing(&self) -> f64 {
        self.spacing
    }

    /// The turn of the layout in degrees, clockwise on the screen about the origin of the design frame. Rings have none.
    #[must_use]
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// What a dot is.
    #[must_use]
    pub fn shape(&self) -> &Mark {
        &self.mark
    }

    /// How much longer an [`Mark::Ellipse`] is than wide, at least 1; read by no other mark.
    #[must_use]
    pub fn aspect(&self) -> f64 {
        self.aspect
    }

    /// The layout the dots lie on.
    #[must_use]
    pub fn family(&self) -> Family {
        self.family
    }

    /// The center of rings, a spiral and rays, `None` for the middle of the shape's bounding box.
    #[must_use]
    pub fn center(&self) -> Option<Point> {
        self.center
    }

    /// How high a [`Family::Waves`] layout swings the rows, in spacings.
    #[must_use]
    pub fn amplitude(&self) -> f64 {
        self.amplitude
    }

    /// How long a wave of a [`Family::Waves`] layout is, in spacings.
    #[must_use]
    pub fn wavelength(&self) -> f64 {
        self.wavelength
    }
}

impl Keyed for Tone {
    fn write_key(&self, key: &mut Key) {
        match self {
            Self::Level(level) => {
                key.word(0);
                key.number(*level);
            }
            Self::Field(field) => {
                key.word(1);
                key.of(&**field);
            }
        }
    }
}

impl Keyed for HalftoneStyle {
    fn write_key(&self, key: &mut Key) {
        key.of(&self.color);
        key.of(&self.tone);
        key.number(self.spacing);
        key.number(self.angle);
        match &self.mark {
            Mark::Path(path) => {
                key.word(1);
                key.of(&**path);
            }
            other => {
                key.word(0);
                key.text(other.name());
            }
        }
        key.number(self.aspect);
        key.text(self.family.name());
        match self.center {
            Some(center) => {
                key.word(1);
                key.of(&center);
            }
            None => key.word(0),
        }
        key.number(self.amplitude);
        key.number(self.wavelength);
    }
}
