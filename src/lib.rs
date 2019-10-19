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
//! // static string
//! splicer.splice(2, 3, "beep");
//! // owned string
//! splicer.splice(6, 7, "boop".to_string());
//! assert_eq!(splicer.to_string(), "a beep c boop e");
//! assert_eq!(splicer.slice_range((3..7)), " c boop");
//! ```

#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(unused)]

use std::{
    borrow::Cow,
    ops::{Bound, Range, RangeBounds},
};

fn get_start_bound(bound: Bound<&usize>) -> usize {
    match bound {
        Bound::Included(n) => *n,
        Bound::Excluded(n) => *n + 1,
        Bound::Unbounded => 0,
    }
}

fn get_end_bound(bound: Bound<&usize>, unbounded: usize) -> usize {
    match bound {
        Bound::Included(n) => *n + 1,
        Bound::Excluded(n) => *n,
        Bound::Unbounded => unbounded,
    }
}

/// A single splice range.
#[derive(Debug)]
struct Splice<'a> {
    /// The range to replace.
    range: Range<usize>,
    /// Replacement value.
    value: Cow<'a, str>,
}

/// A multisplice operation.
#[derive(Debug)]
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
    ///
    /// If the replacement lifetime outlives the input string, you can pass in cheap &str references.
    /// Else, pass in an owned String using `replacement.to_string()`.
    ///
    /// # Example
    /// ```rust
    /// use multisplice::Multisplice;
    ///
    /// let mut splicer = Multisplice::new("a b c d e");
    /// splicer.splice(2, 3, "beep");
    /// {
    ///     let replacement = "boop".to_string();
    ///     splicer.splice(6, 7, replacement);
    /// }
    /// assert_eq!(splicer.to_string(), "a beep c boop e");
    /// ```
    #[inline]
    pub fn splice(&mut self, start: usize, end: usize, value: impl Into<Cow<'a, str>>) {
        self.splice_cow(start, end, value.into())
    }

    /// Replace the characters in the range `range` by the string `value`.
    ///
    /// If the replacement lifetime outlives the input string, you can pass in cheap &str references.
    /// Else, pass in an owned String using `replacement.to_string()`.
    ///
    /// # Example
    /// ```rust
    /// use multisplice::Multisplice;
    ///
    /// let mut splicer = Multisplice::new("a b c d e");
    /// splicer.splice_range(2..3, "beep");
    /// {
    ///     let replacement = "boop".to_string();
    ///     splicer.splice_range(6.., replacement);
    /// }
    /// assert_eq!(splicer.to_string(), "a beep c boop");
    /// ```
    #[inline]
    pub fn splice_range(&mut self, range: impl RangeBounds<usize>, value: impl Into<Cow<'a, str>>) {
        let start = get_start_bound(range.start_bound());
        let end = get_end_bound(range.end_bound(), self.source.len());
        self.splice_cow(start, end, value.into())
    }

    fn splice_cow(&mut self, start: usize, end: usize, value: Cow<'a, str>) {
        // Sorted insert
        let mut insert_at = None;
        for (i, s) in self.splices.iter().enumerate() {
            let range = &s.range;
            assert!(
                !(range.start <= start && range.end > start),
                "Trying to splice an already spliced range"
            );
            if range.start > start {
                insert_at = Some(i);
                break;
            }
        }

        let splice = Splice {
            range: Range { start, end },
            value,
        };
        match insert_at {
            Some(i) => self.splices.insert(i, splice),
            None => self.splices.push(splice),
        };
    }

    /// Get a part of the spliced string, using indices `start` to `end` (exclusive) from the
    /// original string.
    /// If the `start` or `end` indices are in the middle of a spliced range, the full value of the
    /// splice is included in the return value. For example, when indices 1-10 were replaced with a
    /// value "Hello World", requesting a slice of indices 7-20 will return the entire "Hello
    /// World" string followed by indices 11-20.
    ///
    /// # Example
    ///
    /// ```rust
    /// use multisplice::Multisplice;
    /// use std::borrow::Cow;
    ///
    /// let mut splicer = Multisplice::new("a b c d e");
    /// splicer.splice(2, 3, "beep");
    /// splicer.splice(6, 7, "boop");
    /// assert_eq!(splicer.slice(2, 5), "beep c");
    /// // Does not allocate a new String if there were no changes
    /// assert_eq!(splicer.slice(3, 6), Cow::Borrowed(" c "));
    /// ```
    ///
    /// ```rust
    /// use multisplice::Multisplice;
    ///
    /// let mut splicer = Multisplice::new("a b c d e");
    /// splicer.splice(2, 7, "beep and boop");
    /// assert_eq!(splicer.to_string(), "a beep and boop e");
    /// // Slicing in the middle of a spliced range:
    /// assert_eq!(splicer.slice(0, 5), "a beep and boop");
    /// assert_eq!(splicer.slice(6, 9), "beep and boop e");
    /// ```
    pub fn slice(&self, start: usize, end: usize) -> Cow<'a, str> {
        assert!(end <= self.source.len());

        let mut result = String::new();
        let mut last = start;
        for s in &self.splices {
            let range = &s.range;
            // ignore splices that are entirely contained in an earlier spliced range
            if range.end <= last {
                continue;
            }
            // ignore splices after the end of the source
            if range.start >= end {
                break;
            }
            if range.start >= last {
                result.push_str(&self.source[last..range.start]);
            }
            result.push_str(&s.value);
            last = range.end;
        }
        // If our slice ends in the middle of a spliced range, we don't need to add any more of the
        // original string because it's been spliced away
        if end >= last {
            if result.is_empty() {
                return Cow::Borrowed(&self.source[last..end]);
            }
            result.push_str(&self.source[last..end]);
        }

        result.into()
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
    /// assert_eq!(splicer.slice_range((..)), "a beep c boop e");
    /// assert_eq!(splicer.slice_range((2..)), "beep c boop e");
    /// assert_eq!(splicer.slice_range((3..7)), " c boop");
    /// assert_eq!(splicer.slice_range((4..=6)), "c boop");
    /// ```
    #[inline]
    pub fn slice_range(&self, range: impl RangeBounds<usize>) -> Cow<'a, str> {
        let start = get_start_bound(range.start_bound());
        let end = get_end_bound(range.end_bound(), self.source.len());
        self.slice(start, end)
    }
}

impl ToString for Multisplice<'_> {
    /// Execute the splices, returning the new string.
    #[inline]
    fn to_string(&self) -> String {
        self.slice_range(..).into()
    }
}
