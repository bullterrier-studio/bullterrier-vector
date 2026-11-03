//! `type`: fonts, shaping, layout and glyph outlines, behind one small surface. A [`Font`] is bytes checked once, when
//! it is made; [`text`] is the drawing of a string in a font. The font, shaping and outline crates (`read-fonts`,
//! `harfrust`) are used only inside this module, and none of their types is part of the API.

mod bundled;
mod font;
mod layout;
mod lines;
mod outline;
mod shaping;
mod text;
mod unpack;

pub use font::Font;
pub(crate) use font::is_tag;
pub(crate) use layout::{Settings, typeset};
pub use text::{Align, Text, text};
