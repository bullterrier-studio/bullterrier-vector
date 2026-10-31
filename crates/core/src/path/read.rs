//! Path data in: SVG path data (the `d` attribute) read into a path.

use super::arc::arc_segments;
use super::{Path, Segment, SubPath};
use crate::Error;
use crate::geometry::Point;

/// Why a command could not be read, said without the command and the place, which the caller adds.
type Reason = &'static str;

impl Path {
    /// The path an SVG path data string (the `d` attribute of a `<path>`) describes: all of `MmLlHhVvCcSsQqTtAaZz`,
    /// with implicit repeats, numbers such as `1.5.5` or `1e-3`, and packed arc flags. The path is the one
    /// [`Path::to_svg_path`] would write: relative commands become absolute, `H` and `V` lines, `S` and `T` curves
    /// with the reflected control point, and `A` cubic curves (one per quarter turn at most; a zero radius is a line,
    /// radii too small to reach are scaled up, and an arc to its own start draws nothing). Data with nothing in it is
    /// an empty path.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `data`, if the data does not follow SVG's grammar or a coordinate it gives or
    /// works out is not finite. The message names the first bad command and its byte offset.
    pub fn from_svg_path(data: &str) -> Result<Self, Error> {
        Reader::new(data).read()
    }
}

/// How many numbers (or flags) each command takes.
fn arity(command: char) -> Option<usize> {
    match command.to_ascii_uppercase() {
        'M' | 'L' | 'T' => Some(2),
        'H' | 'V' => Some(1),
        'C' => Some(6),
        'S' | 'Q' => Some(4),
        'A' => Some(7),
        'Z' => Some(0),
        _ => None,
    }
}

/// The text being read and the pen: where the path stands.
struct Reader<'a> {
    text: &'a str,
    /// Byte offset of the next character to read.
    at: usize,
    subpaths: Vec<SubPath>,
    current: Point,
    /// Where the open sub-path began: what `Z` returns to.
    start: Point,
    /// The second control point of the cubic just drawn, and the control point of the quadratic just drawn: what
    /// `S` and `T` reflect. `None` after any other command.
    last_cubic_control: Option<Point>,
    last_quad_control: Option<Point>,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Self {
        let origin = Point::new(0.0, 0.0);
        Self {
            text,
            at: 0,
            subpaths: Vec::new(),
            current: origin,
            start: origin,
            last_cubic_control: None,
            last_quad_control: None,
        }
    }

    fn read(mut self) -> Result<Path, Error> {
        self.skip_whitespace();
        while let Some(letter) = self.peek() {
            let offset = self.at;
            let fail = |reason: Reason| failure(letter, offset, reason);
            let Some(count) = arity(letter) else {
                return Err(fail("not a path command"));
            };
            if self.subpaths.is_empty() && !matches!(letter, 'M' | 'm') {
                return Err(fail("path data must start with M"));
            }
            self.at += letter.len_utf8();
            self.skip_whitespace();
            if count == 0 {
                self.close();
                self.skip_whitespace();
                reject_numbers_after_close(&self, letter, offset)?;
                continue;
            }
            let mut command = letter;
            let mut group_offset = offset;
            loop {
                let arguments = self
                    .arguments(command, count)
                    .map_err(|reason| failure(letter, group_offset, reason))?;
                self.run(command, &arguments)
                    .map_err(|reason| failure(letter, group_offset, reason))?;
                // numbers that follow a complete group repeat the command; after a move they are lines
                self.skip_whitespace();
                group_offset = self.at;
                if !matches!(self.peek(), Some(',' | '+' | '-' | '.' | '0'..='9')) {
                    break;
                }
                self.skip_comma();
                command = match command {
                    'M' => 'L',
                    'm' => 'l',
                    other => other,
                };
            }
        }
        Ok(Path {
            subpaths: self.subpaths,
        })
    }

    fn peek(&self) -> Option<char> {
        self.text.get(self.at..)?.chars().next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(' ' | '\t' | '\n' | '\r' | '\u{c}') = self.peek() {
            self.at += 1;
        }
    }

    /// Skips the comma that may stand between two numbers, and the whitespace round it.
    fn skip_comma(&mut self) {
        self.skip_whitespace();
        if self.peek() == Some(',') {
            self.at += 1;
            self.skip_whitespace();
        }
    }

    /// The `count` arguments of one group of `command`: numbers, or for an arc's two flags, 0 or 1 (which may be
    /// packed against what follows).
    fn arguments(&mut self, command: char, count: usize) -> Result<Vec<f64>, Reason> {
        let mut values = Vec::with_capacity(count);
        for index in 0..count {
            if index > 0 {
                self.skip_comma();
            }
            let is_flag = command.eq_ignore_ascii_case(&'A') && (index == 3 || index == 4);
            values.push(if is_flag { self.flag()? } else { self.number()? });
        }
        Ok(values)
    }

    fn flag(&mut self) -> Result<f64, Reason> {
        let value = match self.peek() {
            Some('0') => 0.0,
            Some('1') => 1.0,
            _ => return Err("expected an arc flag, 0 or 1"),
        };
        self.at += 1;
        Ok(value)
    }

    /// A number as SVG writes it: an optional sign, digits with an optional point (or a point and digits), and an
    /// optional exponent.
    fn number(&mut self) -> Result<f64, Reason> {
        let begin = self.at;
        if matches!(self.peek(), Some('+' | '-')) {
            self.at += 1;
        }
        let whole = self.digits();
        let mut fraction = 0;
        if self.peek() == Some('.') {
            self.at += 1;
            fraction = self.digits();
        }
        if whole + fraction == 0 {
            return Err("expected a number");
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.at += 1;
            if matches!(self.peek(), Some('+' | '-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return Err("expected digits in the exponent");
            }
        }
        let value: f64 = self
            .text
            .get(begin..self.at)
            .and_then(|written| written.parse().ok())
            .ok_or("expected a number")?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err("a number is out of range")
        }
    }

    /// Skips ASCII digits, returning how many.
    fn digits(&mut self) -> usize {
        let mut count = 0;
        while let Some('0'..='9') = self.peek() {
            self.at += 1;
            count += 1;
        }
        count
    }

    /// Does one group of `command` with `values`, moving the pen.
    fn run(&mut self, command: char, values: &[f64]) -> Result<(), Reason> {
        let relative = command.is_ascii_lowercase();
        let origin = if relative { self.current } else { Point::new(0.0, 0.0) };
        // the point a pair of numbers names, relative to the pen if the command is lower case
        let pair = |index: usize| Point::new(origin.x + values[index], origin.y + values[index + 1]);
        let previous_cubic = self.last_cubic_control.take();
        let previous_quad = self.last_quad_control.take();
        match command.to_ascii_uppercase() {
            'M' => {
                let target = checked(pair(0))?;
                self.subpaths.push(SubPath::open(target));
                self.current = target;
                self.start = target;
            }
            'L' => self.draw(Segment::Line(checked(pair(0))?)),
            'H' => self.draw(Segment::Line(checked(Point::new(
                origin.x + values[0],
                self.current.y,
            ))?)),
            'V' => self.draw(Segment::Line(checked(Point::new(
                self.current.x,
                origin.y + values[0],
            ))?)),
            'C' => {
                let (c1, c2, end) = (checked(pair(0))?, checked(pair(2))?, checked(pair(4))?);
                self.last_cubic_control = Some(c2);
                self.draw(Segment::Cubic(c1, c2, end));
            }
            'S' => {
                let c1 = checked(reflection(self.current, previous_cubic))?;
                let (c2, end) = (checked(pair(0))?, checked(pair(2))?);
                self.last_cubic_control = Some(c2);
                self.draw(Segment::Cubic(c1, c2, end));
            }
            'Q' => {
                let (control, end) = (checked(pair(0))?, checked(pair(2))?);
                self.last_quad_control = Some(control);
                self.draw(Segment::Quad(control, end));
            }
            'T' => {
                let control = checked(reflection(self.current, previous_quad))?;
                let end = checked(pair(0))?;
                self.last_quad_control = Some(control);
                self.draw(Segment::Quad(control, end));
            }
            _ => {
                let end = checked(pair(5))?;
                let radii = (values[0], values[1]);
                let flags = (values[3] != 0.0, values[4] != 0.0);
                let segments = arc_segments(self.current, radii, values[2], flags, end);
                for segment in &segments {
                    checked_segment(*segment)?;
                }
                for segment in segments {
                    self.draw(segment);
                }
            }
        }
        Ok(())
    }

    /// Adds `segment` to the open sub-path, starting a new one at the last start if the last was closed, and moves
    /// the pen to its end.
    fn draw(&mut self, segment: Segment) {
        if self.subpaths.last().is_none_or(|last| last.closed) {
            self.subpaths.push(SubPath::open(self.start));
        }
        if let Some(open) = self.subpaths.last_mut() {
            open.segments.push(segment);
        }
        self.current = segment.end();
    }

    fn close(&mut self) {
        self.last_cubic_control = None;
        self.last_quad_control = None;
        if let Some(last) = self.subpaths.pop() {
            self.subpaths.push(last.closed());
        }
        self.current = self.start;
    }
}

/// `previous` mirrored through `pen`; the pen itself when there was no `previous`.
fn reflection(pen: Point, previous: Option<Point>) -> Point {
    previous.map_or(pen, |control| {
        Point::new(2.0 * pen.x - control.x, 2.0 * pen.y - control.y)
    })
}

fn checked(point: Point) -> Result<Point, Reason> {
    if point.x.is_finite() && point.y.is_finite() {
        Ok(point)
    } else {
        Err("a coordinate is out of range")
    }
}

fn checked_segment(segment: Segment) -> Result<(), Reason> {
    match segment {
        Segment::Line(end) => checked(end).map(drop),
        Segment::Quad(control, end) => checked(control).and(checked(end)).map(drop),
        Segment::Cubic(c1, c2, end) => checked(c1).and(checked(c2)).and(checked(end)).map(drop),
    }
}

/// The error for `letter`'s group that began at byte `offset`.
fn failure(letter: char, offset: usize, reason: Reason) -> Error {
    Error::parameter("data", format!("command `{letter}` at byte {offset}: {reason}"))
}

/// `Z` takes no numbers: a number, or a comma, straight after it is an error naming it.
fn reject_numbers_after_close(reader: &Reader<'_>, letter: char, offset: usize) -> Result<(), Error> {
    if matches!(reader.peek(), Some(',' | '+' | '-' | '.' | '0'..='9')) {
        Err(failure(letter, offset, "takes no numbers"))
    } else {
        Ok(())
    }
}
