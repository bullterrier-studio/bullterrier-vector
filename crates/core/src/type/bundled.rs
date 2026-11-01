//! The fonts that come with the library: WOFF2 files of two variable fonts under the SIL Open Font License 1.1 (each
//! folder of `crates/core/fonts/` holds the font and its `OFL.txt`, which every package ships). Both are the upstream
//! font file of the Google Fonts repository, made into WOFF2 by fontTools and nothing else (every table, the name
//! table with its copyright and licence text included, is as upstream; `recalcTimestamp=False` keeps `head`):
//!
//! | name | upstream file | commit of `google/fonts` |
//! |---|---|---|
//! | `bricolage_grotesque` | `ofl/bricolagegrotesque/BricolageGrotesque[opsz,wdth,wght].ttf` | `6ce172f74aa3` |
//! | `fraunces` | `ofl/fraunces/Fraunces[SOFT,WONK,opsz,wght].ttf` | `4024282d9b0c` |
//!
//! Neither licence names a Reserved Font Name. The fonts are compiled in as WOFF2, 400 KB for both where the TrueType
//! files are 770 KB, and unpacked by [`Font::new`](super::Font::new) like any WOFF2 file, so the bytes a document
//! names (`sha256`) are these files'.

/// The name of each bundled font and its WOFF2 file.
pub(super) const FONTS: [(&str, &[u8]); 2] = [
    (
        "bricolage_grotesque",
        include_bytes!("../../fonts/bricolage_grotesque/bricolage_grotesque.woff2"),
    ),
    ("fraunces", include_bytes!("../../fonts/fraunces/fraunces.woff2")),
];
