//! The container of a font file: a WOFF2 file is unpacked to the sfnt (TrueType or OpenType) font inside it, a WOFF file
//! is refused, and anything else is the sfnt itself. The only user of `wuff`.

use crate::limits::MAX_FONT_BYTES;

/// The sfnt font in `data`, whatever container the file has.
///
/// # Errors
/// The reason (to follow "is" in a message naming `data`) if `data` is a WOFF font, or a WOFF2 font that cannot be
/// unpacked or unpacks to more than `MAX_FONT_BYTES`.
pub(super) fn sfnt(data: Vec<u8>) -> Result<Vec<u8>, String> {
    match data.get(..4) {
        Some(b"wOF2") => {
            let unpacked = wuff::decompress_woff2(&data)
                .map_err(|_| String::from("a WOFF2 font the library cannot unpack: the file is damaged"))?;
            if unpacked.len() > MAX_FONT_BYTES {
                return Err(format!(
                    "a WOFF2 font that unpacks to {} bytes, but MAX_FONT_BYTES {MAX_FONT_BYTES} is the most accepted",
                    unpacked.len()
                ));
            }
            Ok(unpacked)
        }
        Some(b"wOFF") => Err(String::from(
            "a WOFF font: WOFF is not read, only WOFF2 and plain TrueType or OpenType; convert it (fontTools: set \
             `font.flavor` to \"woff2\" or None, and save)",
        )),
        _ => Ok(data),
    }
}
