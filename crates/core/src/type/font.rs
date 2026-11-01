//! `Font`: the bytes of a TrueType or OpenType font, read once.

use crate::Error;
use crate::key::{Key, Keyed};
use crate::limits::MAX_FONT_BYTES;
use crate::param::is_name;
use read_fonts::model::{Font as Model, Format};
use sha2::{Digest, Sha256};
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

    /// The SHA-256 of the font's bytes as 64 lower-case hexadecimal digits.
    pub(crate) fn sha256(&self) -> String {
        hex(&self.shared.sha256)
    }

    /// The parsed font.
    pub(super) fn model(&self) -> &Model {
        &self.shared.model
    }
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
