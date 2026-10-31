//! The JSON text parser: strict, I-JSON (RFC 7493), and bounded.
//!
//! It refuses what the format refuses: a duplicate key, a lone surrogate, a number beyond a 64-bit float, a leading
//! zero, `NaN`, `Infinity`, a trailing comma, a control character in a string, bytes after the value. It never
//! recurses past `MAX_DEPTH` levels. An error carries the JSON Pointer of the value being read.

use super::value::{Json, pointer_token};
use crate::Error;
use crate::limits::MAX_DEPTH;
use std::collections::BTreeMap;

/// The JSON value `text` is.
///
/// # Errors
/// [`Error::Format`] at the pointer of the value being read, if `text` is not UTF-8 I-JSON, or nests deeper than
/// `MAX_DEPTH` levels.
pub(crate) fn parse(text: &str) -> Result<Json, Error> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
    };
    let outcome = parser.value(0).and_then(|value| {
        parser.space();
        if parser.at < parser.bytes.len() {
            Err(parser.refuse("is followed by more text after the value"))
        } else {
            Ok(value)
        }
    });
    outcome.map_err(Failure::into_error)
}

/// Why reading stopped: what is wrong, and the pointer to the value, built from the innermost level outwards.
struct Failure {
    message: String,
    /// The reference tokens from the value outwards.
    tokens: Vec<String>,
}

impl Failure {
    fn under(mut self, token: String) -> Self {
        self.tokens.push(token);
        self
    }

    fn into_error(self) -> Error {
        let mut pointer = String::new();
        for token in self.tokens.iter().rev() {
            pointer.push('/');
            pointer.push_str(token);
        }
        Error::Format {
            pointer,
            message: self.message,
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    /// A failure at the value being read.
    fn refuse(&self, what: &str) -> Failure {
        Failure {
            message: format!("not JSON: {what} (at byte {})", self.at),
            tokens: Vec::new(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// Reads `expected` or fails saying what was wanted.
    fn expect(&mut self, expected: u8, wanted: &str) -> Result<(), Failure> {
        if self.peek() == Some(expected) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.refuse(&format!("expected {wanted}")))
        }
    }

    /// The value that starts here, `depth` levels of arrays and objects deep.
    fn value(&mut self, depth: usize) -> Result<Json, Failure> {
        self.space();
        match self.peek() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(Json::String),
            Some(b't') => self.word("true", Json::Bool(true)),
            Some(b'f') => self.word("false", Json::Bool(false)),
            Some(b'n') => self.word("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.refuse("expected a value")),
            None => Err(self.refuse("the text ends where a value should be")),
        }
    }

    fn too_deep(depth: usize) -> Option<Failure> {
        (depth > MAX_DEPTH).then(|| Failure {
            message: format!("the nesting passes MAX_DEPTH {MAX_DEPTH}"),
            tokens: Vec::new(),
        })
    }

    fn word(&mut self, word: &str, value: Json) -> Result<Json, Failure> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(self.refuse(&format!("expected {word}")))
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, Failure> {
        if let Some(failure) = Self::too_deep(depth) {
            return Err(failure);
        }
        self.at += 1;
        let mut items = Vec::new();
        self.space();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let item = self
                .value(depth)
                .map_err(|failure| failure.under(items.len().to_string()))?;
            items.push(item);
            self.space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.refuse("expected , or ] in an array")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, Failure> {
        if let Some(failure) = Self::too_deep(depth) {
            return Err(failure);
        }
        self.at += 1;
        let mut members = BTreeMap::new();
        self.space();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Json::Object(members));
        }
        loop {
            self.space();
            if self.peek() != Some(b'"') {
                return Err(self.refuse("expected a key (a string) in an object"));
            }
            let key = self.string()?;
            self.space();
            self.expect(b':', ": after a key")
                .map_err(|failure| failure.under(pointer_token(&key)))?;
            let value = self
                .value(depth)
                .map_err(|failure| failure.under(pointer_token(&key)))?;
            if members.contains_key(&key) {
                return Err(Failure {
                    message: format!("not JSON: the key {key:?} appears twice (I-JSON allows each key once)"),
                    tokens: vec![pointer_token(&key)],
                });
            }
            members.insert(key, value);
            self.space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Json::Object(members));
                }
                _ => return Err(self.refuse("expected , or } in an object")),
            }
        }
    }

    /// The string that starts at the opening quote.
    fn string(&mut self) -> Result<String, Failure> {
        self.at += 1;
        let mut text = String::new();
        loop {
            // runs of ordinary characters are copied whole; the input is a `&str`, so they are valid UTF-8
            let start = self.at;
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.at += 1;
            }
            if let Ok(run) = std::str::from_utf8(&self.bytes[start..self.at]) {
                text.push_str(run);
            }
            match self.peek() {
                Some(b'"') => {
                    self.at += 1;
                    return Ok(text);
                }
                Some(b'\\') => {
                    self.at += 1;
                    text.push(self.escape()?);
                }
                Some(_) => return Err(self.refuse("a control character inside a string must be escaped")),
                None => return Err(self.refuse("the text ends inside a string")),
            }
        }
    }

    /// The character an escape stands for; the backslash is read.
    fn escape(&mut self) -> Result<char, Failure> {
        let Some(letter) = self.peek() else {
            return Err(self.refuse("the text ends inside an escape"));
        };
        self.at += 1;
        Ok(match letter {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => self.unicode()?,
            _ => return Err(self.refuse("an escape that JSON does not have")),
        })
    }

    /// The four hex digits after `\u`.
    fn hex4(&mut self) -> Result<u32, Failure> {
        let digits = self.bytes.get(self.at..self.at + 4);
        let value = digits
            .and_then(|digits| std::str::from_utf8(digits).ok())
            .filter(|digits| digits.bytes().all(|digit| digit.is_ascii_hexdigit()))
            .and_then(|digits| u32::from_str_radix(digits, 16).ok());
        match value {
            Some(value) => {
                self.at += 4;
                Ok(value)
            }
            None => Err(self.refuse("\\u needs four hex digits")),
        }
    }

    /// The character of `\uXXXX`, joining a surrogate pair; the `\u` is read.
    fn unicode(&mut self) -> Result<char, Failure> {
        let first = self.hex4()?;
        match first {
            0xD800..=0xDBFF => {
                if self.bytes[self.at..].starts_with(b"\\u") {
                    self.at += 2;
                    let second = self.hex4()?;
                    if (0xDC00..=0xDFFF).contains(&second) {
                        let joined = 0x1_0000 + ((first - 0xD800) << 10) + (second - 0xDC00);
                        if let Some(character) = char::from_u32(joined) {
                            return Ok(character);
                        }
                    }
                }
                Err(self.refuse("a lone surrogate (a high one needs a low one after it)"))
            }
            0xDC00..=0xDFFF => Err(self.refuse("a lone surrogate (a low one needs a high one before it)")),
            other => char::from_u32(other).ok_or_else(|| self.refuse("not a character")),
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
        self.at - start
    }

    /// A number by the grammar of JSON: `-`, an integer without a leading zero, a fraction, an exponent.
    fn number(&mut self) -> Result<Json, Failure> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.at += 1;
                if self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                    return Err(self.refuse("a number with a leading zero"));
                }
            }
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.refuse("a number needs digits")),
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if self.digits() == 0 {
                return Err(self.refuse("a number needs digits after the point"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return Err(self.refuse("a number needs digits in its exponent"));
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at]).map_err(|_| self.refuse("not a number"))?;
        match text.parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(Json::Number(number)),
            _ => Err(self.refuse("a number beyond a 64-bit float")),
        }
    }
}
