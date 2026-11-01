//! `Font`: the bytes of a TrueType or OpenType font, read once.

use crate::key::{Key, Keyed};
use crate::limits::MAX_FONT_BYTES;
use crate::param::is_name;
use crate::{Error, Param};
use read_fonts::TableProvider;
use read_fonts::model::{Font as Model, Format, Variation};
use read_fonts::tables::layout::FeatureRecord;
use read_fonts::types::{NameId, Tag};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::{self, Write as _};
use std::sync::Arc;

/// A TrueType or OpenType font, variable or not, with outlines in `glyf` or `CFF`. It is made from the bytes of the
/// font file by [`Font::new`], which reads them once: a `Font` that exists is one the library can shape and draw with.
/// Cloning is cheap, and a font can be used by any number of drawings and threads.
///
/// A font is its bytes: two fonts are the same font when their bytes are, whatever name they carry. A name is what a
/// saved drawing calls the font ([`Drawing::to_json`]); one without a name is given `font1`, `font2`... there.
///
/// [`Drawing::to_json`]: crate::Drawing::to_json
#[derive(Clone)]
pub struct Font {
    shared: Arc<Shared>,
}

struct Shared {
    model: Model,
    sha256: [u8; 32],
    name: Option<String>,
}

impl Font {
    /// The font in `data`, the bytes of a `.ttf` or `.otf` file.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `data` if it is longer than `MAX_FONT_BYTES` (32 MiB), is not a font the library
    /// reads (a font collection, a Type 1 font, a bare CFF font, a web font and anything that is no font), has no
    /// outlines (a bitmap or colour font), or has a units-per-em outside 16 to 16384.
    pub fn new(data: impl Into<Vec<u8>>) -> Result<Self, Error> {
        let data = data.into();
        if data.len() > MAX_FONT_BYTES {
            return Err(refuse(format!(
                "is {} bytes, but MAX_FONT_BYTES {MAX_FONT_BYTES} is the most accepted",
                data.len()
            )));
        }
        match Format::new(&data) {
            Some(Format::Sfnt(1)) => {}
            Some(Format::Sfnt(count)) => {
                return Err(refuse(format!(
                    "is a collection of {count} fonts: give the bytes of one font"
                )));
            }
            Some(_) => {
                return Err(refuse(
                    "is a font the library does not read: only TrueType and OpenType fonts".into(),
                ));
            }
            None => {
                return Err(refuse(
                    "is not a font: the bytes are not a TrueType or OpenType font".into(),
                ));
            }
        }
        let sha256 = Sha256::digest(&data).into();
        let model = Model::new(data, 0).ok_or_else(|| refuse("is not a font the library can read".into()))?;
        let metrics = model.metrics();
        if metrics.raw_units_per_em == 0 {
            return Err(refuse(
                "is not a font the library can read: it has no head table".into(),
            ));
        }
        if metrics.raw_units_per_em != metrics.units_per_em {
            return Err(refuse(format!(
                "has {} units per em, but 16 to 16384 are accepted",
                metrics.raw_units_per_em
            )));
        }
        if model.outline_glyphs().is_empty() {
            return Err(refuse("has no outlines: only fonts of outlines are read".into()));
        }
        Ok(Self {
            shared: Arc::new(Shared {
                model,
                sha256,
                name: None,
            }),
        })
    }

    /// This font called `name` in a saved drawing: letters, digits and `_`, starting with a letter.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `name` if it is none of those.
    pub fn named(self, name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        if !is_name(&name) {
            return Err(Error::parameter(
                "name",
                format!("must be letters, digits and _, starting with a letter, got {name:?}"),
            ));
        }
        Ok(Self {
            shared: Arc::new(Shared {
                model: self.shared.model.clone(),
                sha256: self.shared.sha256,
                name: Some(name),
            }),
        })
    }

    /// The name a saved drawing calls the font, if it was given one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.shared.name.as_deref()
    }

    /// The variation axes of the font, each a [`Param`] called by its four-letter tag (`wght`), with the font's own
    /// minimum, default and maximum, its name for the axis as the meaning, and no unit. The axes come in the font's
    /// order and are what `axes` of [`Text`](crate::Text) takes: `Text::axes([("wght", &weight)])`. A static font has
    /// none. An axis that a `Param` cannot hold (a tag that is not letters and digits starting with a letter, a minimum
    /// not below the maximum, a default outside them, a tag the font lists twice) is left out; it can still be set by
    /// its tag.
    #[must_use]
    pub fn axes(&self) -> Vec<Param> {
        let mut seen = BTreeSet::new();
        self.variation_axes()
            .into_iter()
            .filter(|axis| seen.insert(axis.tag))
            .filter_map(|axis| {
                let param = Param::new(
                    &axis.tag.to_string(),
                    f64::from(axis.default),
                    f64::from(axis.min),
                    f64::from(axis.max),
                );
                param.ok().map(|param| param.meaning(&self.axis_name(axis.name)))
            })
            .collect()
    }

    /// The tags of the OpenType features the font's `GSUB` and `GPOS` tables list, sorted, once each: what `features` of
    /// [`Text`](crate::Text) takes. A table that cannot be read lists none, and a tag that is not four printable ASCII
    /// characters is left out.
    #[must_use]
    pub fn features(&self) -> Vec<String> {
        let tables = self.shared.model.tables();
        let mut found = BTreeSet::new();
        if let Ok(list) = tables.gsub().and_then(|gsub| gsub.feature_list()) {
            found.extend(list.feature_records().iter().map(FeatureRecord::feature_tag));
        }
        if let Ok(list) = tables.gpos().and_then(|gpos| gpos.feature_list()) {
            found.extend(list.feature_records().iter().map(FeatureRecord::feature_tag));
        }
        found
            .into_iter()
            .filter_map(|tag| tag_text(tag.to_be_bytes()))
            .collect()
    }

    /// Whether the font has the axis `tag`, and `value`, if given, lies in its range (both ends included).
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `axes` if `tag` is not four printable ASCII characters, the font has no such axis
    /// (the message lists the axes it has), or `value` is outside its range.
    pub(crate) fn check_axis(&self, tag: &str, value: Option<f64>) -> Result<(), Error> {
        let wanted = tag_of(tag, "axes")?;
        let own = self.variation_axes();
        let Some(axis) = own.iter().find(|axis| axis.tag == wanted) else {
            let listed = if own.is_empty() {
                String::from("the font has no axes")
            } else {
                let listed: Vec<String> = own.iter().map(Axis::describe).collect();
                format!("the font's axes are {}", listed.join(", "))
            };
            return Err(Error::parameter("axes", format!("has no axis {tag:?}: {listed}")));
        };
        match value {
            Some(value) if !(f64::from(axis.min)..=f64::from(axis.max)).contains(&value) => Err(Error::parameter(
                "axes",
                format!(
                    "{tag:?} must be {} to {} in this font, got {}",
                    axis.min,
                    axis.max,
                    shown(value)
                ),
            )),
            _ => Ok(()),
        }
    }

    /// Whether the font lists the feature `tag`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `features` if `tag` is not four printable ASCII characters or the font does not
    /// list it (the message lists the features it has).
    pub(crate) fn check_feature(&self, tag: &str) -> Result<(), Error> {
        tag_of(tag, "features")?;
        let features = self.features();
        if features.iter().any(|listed| listed == tag) {
            return Ok(());
        }
        let listed = if features.is_empty() {
            String::from("the font has no features")
        } else {
            format!("the font's features are {}", features.join(", "))
        };
        Err(Error::parameter(
            "features",
            format!("has no feature {tag:?}: {listed}"),
        ))
    }

    /// The font at `location`: an axis value for some of its axes (the others at their defaults), each inside the
    /// font's range.
    pub(super) fn at(&self, location: &[(Tag, f32)]) -> Model {
        if location.is_empty() {
            return self.shared.model.clone();
        }
        self.shared
            .model
            .instance_builder()
            .variations(location.iter().map(|&(tag, value)| Variation::new(tag, value)))
            .build()
    }

    /// The axes of the font, in its order.
    fn variation_axes(&self) -> Vec<Axis> {
        self.shared
            .model
            .axes()
            .iter()
            .map(|axis| Axis {
                tag: axis.tag(),
                min: axis.min_value(),
                default: axis.default_value(),
                max: axis.max_value(),
                name: axis.name(),
            })
            .collect()
    }

    /// The font's English name with the id `id`, or an empty one.
    fn axis_name(&self, id: NameId) -> String {
        self.shared
            .model
            .names()
            .get(id)
            .and_then(|name| name.english_or_first())
            .map(|name| name.name().chars().collect())
            .unwrap_or_default()
    }

    /// The SHA-256 of the font's bytes as 64 lower-case hexadecimal digits.
    pub(crate) fn sha256(&self) -> String {
        hex(&self.shared.sha256)
    }

    /// The parsed font.
    pub(super) fn model(&self) -> &Model {
        &self.shared.model
    }
}

/// An axis of the font, in its units.
struct Axis {
    tag: Tag,
    min: f32,
    default: f32,
    max: f32,
    name: NameId,
}

impl Axis {
    /// `wght (300 to 700, default 400)`.
    fn describe(&self) -> String {
        format!("{} ({} to {}, default {})", self.tag, self.min, self.max, self.default)
    }
}

/// A number for a message: a very large or small one in the short form, so that 1e300 is not 300 digits.
fn shown(value: f64) -> String {
    if value != 0.0 && !(1e-9..1e15).contains(&value.abs()) {
        format!("{value:e}")
    } else {
        value.to_string()
    }
}

/// Whether `text` is a tag: four printable ASCII characters.
pub(crate) fn is_tag(text: &str) -> bool {
    tag_bytes(text).is_some()
}

/// The bytes of `text` if it is a tag.
fn tag_bytes(text: &str) -> Option<[u8; 4]> {
    <[u8; 4]>::try_from(text.as_bytes())
        .ok()
        .filter(|bytes| tag_text(*bytes).is_some())
}

/// `text` as a tag.
///
/// # Errors
/// [`Error::Parameter`] naming `argument` if it is not four printable ASCII characters.
pub(super) fn tag_of(text: &str, argument: &str) -> Result<Tag, Error> {
    tag_bytes(text).map(|bytes| Tag::new(&bytes)).ok_or_else(|| {
        Error::parameter(
            argument,
            format!("has the tag {text:?}, but a tag is four printable ASCII characters"),
        )
    })
}

/// The tag with these bytes as text, if they are four printable ASCII characters.
fn tag_text(bytes: [u8; 4]) -> Option<String> {
    bytes
        .iter()
        .all(|byte| (0x20..=0x7e).contains(byte))
        .then(|| bytes.iter().map(|byte| char::from(*byte)).collect())
}

fn refuse(message: String) -> Error {
    Error::parameter("data", message)
}

impl Keyed for Font {
    /// The digest of the bytes: a font is its bytes, and the name is not part of what is drawn.
    fn write_key(&self, key: &mut Key) {
        for chunk in self.shared.sha256.as_chunks::<8>().0 {
            key.word(u64::from_be_bytes(*chunk));
        }
    }
}

impl fmt::Debug for Font {
    /// The name and the digest, never the bytes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Font")
            .field("name", &self.shared.name)
            .field("sha256", &self.sha256())
            .finish()
    }
}

/// `bytes` as lower-case hexadecimal.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        // writing to a String cannot fail
        let _ = write!(text, "{byte:02x}");
        text
    })
}
