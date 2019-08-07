//! Easily splice a string multiple times, using offsets into the original string.
//! This way you don't have to track offsets after making a change, which can get hairy very
//! quickly!
//!
//! ## Usage
//!
//! ```rust
//! use multisplice::Multisplice;
//!
//! let source = "a b c d e";
//! let mut splicer = Multisplice::new(source);
//! splicer.splice(2, 3, "beep");
//! splicer.splice(6, 7, "boop");
//! assert_eq!(splicer.to_string(), "a beep c boop e".to_string());
//! assert_eq!(splicer.slice_range((3..7)), " c boop".to_string());
//! ```

#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(unused)]

use std::ops::{Bound, RangeBounds};

/// A single splice range.
struct Splice<'a> {
    /// Start index of this range.
    start: usize,
    /// End index of this range.
    end: usize,
    /// Replacement value.
    value: &'a str,
}

/// A multisplice operation.
pub struct Multisplice<'a> {
    /// The original string.
    source: &'a str,
    /// Splice operations.
    splices: Vec<Splice<'a>>,
}

impl<'a> Multisplice<'a> {
    /// Create a "multisplicer" for the given string.
    #[inline]
    pub fn new(source: &'a str) -> Self {
        Multisplice {
            source,
            splices: vec![],
        }
    }

    /// Replace the characters from index `start` up to (but not including) index `end` by the
    /// string `value`.
    pub fn splice(&mut self, start: usize, end: usize, value: &'a str) {
        // Sorted insert
        let mut insert_at = None;
        for (i, s) in self.splices.iter().enumerate() {
            assert!(
                !(s.start <= start && s.end > start),
                "Trying to splice an already spliced range"
            );
            if s.start > start {
                insert_at = Some(i);
                break;
            }
        }

        match insert_at {
            Some(i) => self.splices.insert(i, Splice { start, end, value }),
            None => self.splices.push(Splice { start, end, value }),
        };
    }

    /// Get a part of the spliced string, using indices `start` to `end` (exclusive) from the
    /// original string.
    /// If the `start` or `end` indices are in the middle of a spliced range, the full value of the
    /// splice is included in the return value. For example, when indices 1-10 were replaced with a
    /// value "Hello World", requesting a slice of indices 7-20 will return the entire "Hello
    /// World" string followed by indices 11-20.
    pub fn slice(&self, start: usize, end: usize) -> String {
        assert!(end <= self.source.len());

        let mut result = String::new();
        let mut last = start;
        for s in &self.splices {
            // ignore splices that are entirely contained in an earlier spliced range
            if s.end <= last {
                continue;
            }
            // ignore splices after the end of the source
            if s.start >= end {
                break;
            }
            if s.start >= last {
                result.push_str(&self.source[last..s.start]);
            }
            result.push_str(s.value);
            last = s.end;
        }
        // If our slice ends in the middle of a spliced range, we don't need to add any more of the
        // original string because it's been spliced away
        if end >= last {
            result.push_str(&self.source[last..end]);
        }

        result
    }

    /// Slice using range syntax.
    ///
    /// ```rust
    /// use multisplice::Multisplice;
    ///
    /// let source = "a b c d e";
    /// let mut splicer = Multisplice::new(source);
    /// splicer.splice(2, 3, "beep");
    /// splicer.splice(6, 7, "boop");
    /// assert_eq!(splicer.slice_range((..)), "a beep c boop e".to_string());
    /// assert_eq!(splicer.slice_range((2..)), "beep c boop e".to_string());
    /// assert_eq!(splicer.slice_range((3..7)), " c boop".to_string());
    /// assert_eq!(splicer.slice_range((4..=6)), "c boop".to_string());
    /// ```
    pub fn slice_range<R>(&self, range: R) -> String
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(n) => *n + 1,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => self.source.len(),
        };
        self.slice(start, end)
    }

    /// Execute the splices, returning the new string.
    #[inline]
    pub fn to_string(&self) -> String {
        self.slice(0, self.source.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice() {
        let mut splicer = Multisplice::new("a b c d e");
        splicer.splice(2, 3, "beep");
        splicer.splice(6, 7, "boop");
        assert_eq!(splicer.to_string(), "a beep c boop e".to_string());
    }

    #[test]
    fn splice_n_slice() {
        let mut splicer = Multisplice::new("a b c d e");
        splicer.splice(2, 3, "beep");
        splicer.splice(6, 7, "boop");
        assert_eq!(splicer.slice(2, 5), "beep c".to_string());
    }

    #[test]
    fn slice_overlap() {
        let mut splicer = Multisplice::new("a b c d e");
        splicer.splice(2, 7, "beep and boop");
        assert_eq!(splicer.to_string(), "a beep and boop e".to_string());
        assert_eq!(splicer.slice(0, 5), "a beep and boop".to_string());
        assert_eq!(splicer.slice(6, 9), "beep and boop e".to_string());
    }
}
