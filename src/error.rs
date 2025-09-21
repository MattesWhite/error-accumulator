//! Provide [`AccumulatedError`] to present a collection of errors.

use std::{error::Error, fmt};

use crate::path::SourcePath;

/// A list of recorded errors and their source's path in the input.
#[derive(Debug, Default)]
pub struct AccumulatedError {
    errors: Vec<(SourcePath, Box<dyn Error + Send + Sync + 'static>)>,
}

impl AccumulatedError {
    /// Get all accumulated errors of the given type.
    ///
    /// Errors are in accumulation order.
    pub fn get_by_type<E>(&self) -> impl Iterator<Item = (&SourcePath, &E)>
    where
        E: Error + Send + Sync + 'static,
    {
        self.errors
            .iter()
            .filter_map(|(path, stored)| stored.downcast_ref().map(|typed| (path, typed)))
    }

    /// Get all accumulated errors for a given path.
    ///
    /// Errors are in accumulation order.
    pub fn get_by_path(
        &self,
        path: &SourcePath,
    ) -> impl Iterator<Item = &Box<dyn Error + Send + Sync>> {
        self.errors
            .iter()
            .filter_map(move |(error_path, stored)| (error_path == path).then_some(stored))
    }

    /// Number of stored errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// True if no errors are stored.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(crate) fn append<E>(&mut self, path: SourcePath, error: E)
    where
        E: Error + Send + Sync + 'static,
    {
        self.errors.push((path, Box::new(error)));
    }

    pub(crate) fn merge(&mut self, other: AccumulatedError) {
        self.errors.extend(other.errors);
    }
}

impl fmt::Display for AccumulatedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Accumulated errors:")?;
        for (path, error) in &self.errors {
            writeln!(f, "- {path}: {error}")?;
        }
        Ok(())
    }
}

impl Error for AccumulatedError {}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use crate::{path::PathSegment, test_util::n};

    #[test]
    fn should_include_path_in_display() {
        let path1 = SourcePath::new().join(PathSegment::Field(n("foo")));
        let path2 = SourcePath::new().join(PathSegment::Array {
            name: n("bar"),
            index: 2,
        });
        let mut error = AccumulatedError::default();
        error.append(
            path1.clone(),
            io::Error::new(io::ErrorKind::Interrupted, "error1"),
        );
        error.append(
            path2.clone(),
            io::Error::new(io::ErrorKind::AlreadyExists, "error2"),
        );

        let display = error.to_string();
        dbg!(&display);

        assert!(display.contains(&path1.to_string()));
        assert!(display.contains(&path2.to_string()));
    }
}
