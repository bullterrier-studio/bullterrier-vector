//! `Key`: a value written as numbers, so that two values are equal exactly when their keys are. The cache tells steps
//! apart by it, and a key is compared, never hashed.

/// Numbers that identify a value. Every writer begins a variable-length part with its length or a tag, so that one
/// key is never the start of another.
#[derive(Debug, Default)]
pub(crate) struct Key(Vec<u64>);

impl Key {
    /// An empty key.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A tag or a count.
    pub(crate) fn word(&mut self, word: u64) {
        self.0.push(word);
    }

    /// A count of items that follow.
    pub(crate) fn count(&mut self, count: usize) {
        self.0.push(count as u64);
    }

    /// A number by its bits. A negative zero is written as zero, so that numbers that compare equal have equal keys.
    pub(crate) fn number(&mut self, number: f64) {
        self.0.push((number + 0.0).to_bits());
    }

    /// A word: its length, then its bytes.
    pub(crate) fn text(&mut self, text: &str) {
        self.count(text.len());
        self.0.extend(text.bytes().map(u64::from));
    }

    /// The value written by `keyed`.
    pub(crate) fn of(&mut self, keyed: &impl Keyed) {
        keyed.write_key(self);
    }

    /// The numbers written so far.
    pub(crate) fn finish(self) -> Vec<u64> {
        self.0
    }
}

/// A value that writes itself as a [`Key`]: everything that decides what it is, and nothing that does not.
pub(crate) trait Keyed {
    /// Writes this value to `key`.
    fn write_key(&self, key: &mut Key);
}

impl<T: Keyed> Keyed for &T {
    fn write_key(&self, key: &mut Key) {
        (*self).write_key(key);
    }
}

impl<T: Keyed> Keyed for Option<T> {
    fn write_key(&self, key: &mut Key) {
        match self {
            None => key.word(0),
            Some(inner) => {
                key.word(1);
                inner.write_key(key);
            }
        }
    }
}

impl<T: Keyed + ?Sized> Keyed for std::sync::Arc<T> {
    fn write_key(&self, key: &mut Key) {
        (**self).write_key(key);
    }
}

impl<T: Keyed> Keyed for [T] {
    fn write_key(&self, key: &mut Key) {
        key.count(self.len());
        for item in self {
            key.of(item);
        }
    }
}

impl Keyed for u64 {
    fn write_key(&self, key: &mut Key) {
        key.word(*self);
    }
}

impl Keyed for f64 {
    fn write_key(&self, key: &mut Key) {
        key.number(*self);
    }
}
