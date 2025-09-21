//! [`SourcePath`] to identify the path to the source of an accumulated error.

use std::{borrow::Cow, fmt, num::ParseIntError, str::FromStr};

const INVALID_FIELD_NAME_CHARS: [char; 3] = ['.', '[', ']'];

/// Errors parsing a [`SourcePath`] or its components.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid [`FieldName`].
    #[error(
        "failed to parse '{0}' as it contains at least one invalid character: {INVALID_FIELD_NAME_CHARS:?}"
    )]
    InvalidCharInName(String),
    /// Incomplete array path.
    #[error("array segment '{0}' does not contain proper brackets")]
    IncompleteArraySegment(String),
    /// Invalid index in array path.
    #[error("invalid index")]
    InvalidIdx(#[from] ParseIntError),
}

/// The full path to source of error from the input.
///
/// Composed of [`PathSegment`]s.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourcePath {
    segments: Vec<PathSegment>,
}

/// A segment of a full [`SourcePath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// The segment references a field.
    Field(FieldName),
    /// The segement references an element of an array.
    Array {
        /// The array's name.
        name: FieldName,
        /// The element's position within the array.
        index: usize,
    },
}

/// A valid name of an input's field.
///
/// At the moment most characters are allowed excluding `.`, `[`, and `]`. This
/// might change in the future.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldName(Cow<'static, str>);

impl SourcePath {
    /// Construct a new, empty path.
    pub fn new() -> Self {
        Default::default()
    }

    /// Append a new segment to the path.
    pub fn join(&self, segment: PathSegment) -> Self {
        let mut new = self.clone();
        new.segments.push(segment);
        new
    }

    /// Check if the other path has the same base as the path at hand.
    ///
    /// For example: `foo.bar` is the base of `foo.bar.baz`.
    pub fn is_matching_base(&self, base: &Self) -> bool {
        base.segments
            .iter()
            .zip(&self.segments)
            .all(|(base, to_match)| base == to_match)
    }
}

impl fmt::Display for SourcePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            f.write_str("root")
        } else {
            let mut segments = self.segments.iter();
            let start = segments.next().expect("segments is not empty");
            write!(f, "{start}")?;
            for segment in segments {
                write!(f, ".{segment}")?;
            }
            Ok(())
        }
    }
}

impl FromStr for SourcePath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let segments = s
            .split('.')
            .map(|segment| segment.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { segments })
    }
}

impl PathSegment {
    /// Construct a field segment.
    pub fn field(name: FieldName) -> Self {
        Self::Field(name)
    }

    /// Construct an array segment.
    pub fn array(name: FieldName, index: usize) -> Self {
        Self::Array { name, index }
    }
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSegment::Field(field) => write!(f, "{field}"),
            PathSegment::Array { name, index } => write!(f, "{name}[{index}]"),
        }
    }
}

impl FromStr for PathSegment {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.ends_with(']') {
            if let Some(idx) = s.find('[') {
                let idx_str = &s[idx + 1..s.len() - 1];
                let field_idx = idx_str.parse()?;
                return Ok(Self::Array {
                    name: (&s[..idx]).parse()?,
                    index: field_idx,
                });
            } else {
                return Err(Error::IncompleteArraySegment(s.to_string()));
            }
        }

        Ok(Self::Field(s.parse()?))
    }
}

impl fmt::Display for FieldName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FieldName {
    // Used in `field!` macro.
    #[doc(hidden)]
    pub const fn new_unchecked(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// Access the inner name as a string slice.
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl AsRef<str> for FieldName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for FieldName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_str_as_field_name(s)?;

        Ok(Self(Cow::Owned(s.to_string())))
    }
}

impl TryFrom<String> for FieldName {
    type Error = Error;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        validate_str_as_field_name(&name)?;
        Ok(Self(Cow::Owned(name)))
    }
}

impl<'a> TryFrom<&'a str> for FieldName {
    type Error = <Self as FromStr>::Err;

    fn try_from(name: &'a str) -> Result<Self, Self::Error> {
        name.parse()
    }
}

fn validate_str_as_field_name(name: &str) -> Result<(), Error> {
    if name.contains(&INVALID_FIELD_NAME_CHARS) {
        Err(Error::InvalidCharInName(name.to_string()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::n;

    #[test]
    fn should_serialize_root() {
        let path = SourcePath::new();

        let string = path.to_string();

        assert_eq!(string.as_str(), "root");
    }

    #[test]
    fn should_display_multi_segment_path() {
        let path = SourcePath::new()
            .join(PathSegment::field(n("foo")))
            .join(PathSegment::array(n("bar"), 42))
            .join(PathSegment::field(n("baz")));

        let string = path.to_string();

        assert_eq!(string.as_str(), "foo.bar[42].baz");
    }

    #[test]
    fn should_parse_path() {
        let expect = SourcePath::new()
            .join(PathSegment::array(n("foo"), 21))
            .join(PathSegment::field(n("bar")))
            .join(PathSegment::field(n("xyz")));
        let path = "foo[21].bar.xyz";

        let parsed = path.parse::<SourcePath>().unwrap();

        assert_eq!(parsed, expect);
    }
}
