//! [`ErrorAccumulator`] is a utility to write parsing functions that work
//! through as much data as possible collecting errors on the way instead of
//! doing early returns on the first error.
//!
//! # General approach
//!
//! The idea is that an input is walked through all its fields, structs, and
//! arrays. Along the way `ErrorAccumulator` is used to keep track of the
//! input's parsing results so in case all parsing was successful the validated
//! data can be returned and in case of at least one error all
//! [`AccumulatedError`]s and their source can be named.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

use std::{error::Error, marker::PhantomData};

use crate::{
    builder::{ArrayBuilder, ErrorBuilderParent, FieldBuilder, StructBuilder},
    cons::{Append, AsRefTuple, Nil, ToTuple},
    construct::{Constructor, ListValidator},
    error::AccumulatedError,
    path::{FieldName, PathSegment, SourcePath},
};

pub mod builder;
mod cons;
pub mod construct;
pub mod error;
pub mod path;

/// The entry-point to accumulate parsing results.
///
/// All parsing results are tracked, i.e. `Ok` values and errors alike.
///
/// Use the methods like [`field()`](Self::field), [`strukt()`](Self::strukt),
/// or [`array()`](Self::array) to tell the accumulator from where in the input
/// the next parsing results are derived.
///
/// The final [`analyse()`](Self::analyse) call returns all errors if at least
/// one error was recorded else a tuple of all recorded `Ok` values is returned.
/// There is also [`on_ok()`](Self::on_ok) to convert the recorded `Ok` values
/// before retruning the tuple.
#[derive(Debug)]
pub struct ErrorAccumulator<List> {
    errors: AccumulatedError,
    values: List,
    base: SourcePath,
}

/// Intermediate state when [`ErrorAccumulator::on_ok()`] was called.
///
/// See the method's documentation for more details.
#[derive(Debug)]
pub struct ErrorAccumulatorFinisher<List, Constructor, Out> {
    accumulated_errors: AccumulatedError,
    values: List,
    constructor: Constructor,
    _marker: PhantomData<Out>,
}

impl ErrorAccumulator<Nil> {
    /// Create a new, empty `ErrorAccumulator`.
    pub fn new() -> Self {
        Self {
            errors: Default::default(),
            values: Nil,
            base: Default::default(),
        }
    }
}

impl Default for ErrorAccumulator<Nil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ChildValue, List> ErrorBuilderParent<ChildValue> for ErrorAccumulator<List>
where
    List: Append<ChildValue>,
{
    type AfterRecord = ErrorAccumulator<List::Output>;

    fn finish_child_builder(
        self,
        child_result: Result<ChildValue, AccumulatedError>,
    ) -> Self::AfterRecord {
        let Self {
            errors: mut accumulated_errors,
            values,
            base,
        } = self;

        let values = match child_result {
            Ok(value) => values.append(value),
            Err(errors) => {
                accumulated_errors.merge(errors);
                values.append(None)
            }
        };

        ErrorAccumulator {
            errors: accumulated_errors,
            values,
            base,
        }
    }
}

impl<List> ErrorAccumulator<List> {
    /// Record a result of parsing a field of the input.
    pub fn field<FieldValue, E>(
        self,
        field: FieldName,
        result: Result<FieldValue, E>,
    ) -> ErrorAccumulator<List::Output>
    where
        List: Append<FieldValue>,
        E: Error + Send + Sync + 'static,
    {
        let path = self.base.join(PathSegment::Field(field));
        FieldBuilder::new(self, path).value(result).finish()
    }

    /// Start a [`FieldBuilder`] to record results for parsing of a single input
    /// field.
    ///
    /// This allows for more finegrained validation of a single input value,
    /// e.g. like testing for different properties.
    ///
    /// See [`FieldBuilder`] for more information.
    pub fn field_builder<FieldValue>(self, field: FieldName) -> FieldBuilder<Self, FieldValue, Nil>
    where
        List: Append<FieldValue>,
    {
        let path = self.base.join(PathSegment::Field(field));
        FieldBuilder::new(self, path)
    }

    /// Start a [`StructBuilder`] to analyse the parsing results of a nested
    /// struct of the input.
    ///
    /// This is mainly to record the correct source paths when walking the
    /// input's structure.
    ///
    /// See [`StructBuilder`] for more information.
    pub fn strukt<StructValue>(self, field: FieldName) -> StructBuilder<Self, StructValue, Nil>
    where
        List: Append<StructValue>,
    {
        let path = self.base.join(PathSegment::Field(field));
        StructBuilder::new(self, path)
    }

    /// Start an [`ArrayBuilder`] to analyse the elements of a nested array of
    /// the input.
    ///
    /// See [`ArrayBuilder`] for more information.
    pub fn array<ElementValue>(self, field: FieldName) -> ArrayBuilder<Self, ElementValue>
    where
        List: Append<Vec<ElementValue>>,
    {
        let base = self.base.clone();
        ArrayBuilder::new(self, base, field)
    }

    /// Run another validation step on the previously recorded `Ok` values if
    /// there were no errors yet.
    ///
    /// In case an error was already recorded the `validator` is not executed.
    ///
    /// For an example, see the docs of [`StructBuilder::with_previous()`].
    pub fn with_previous<Valid, T, E>(self, validator: Valid) -> ErrorAccumulator<List::Output>
    where
        Valid: ListValidator<List, T, E>,
        List: AsRefTuple + Append<T>,
        E: Error + Send + Sync + 'static,
    {
        let Self {
            mut errors,
            values,
            base,
        } = self;

        let values = if errors.is_empty() {
            let result = validator.validate(&values);
            append_or_record(values, &base, result, &mut errors)
        } else {
            values.append(None)
        };

        ErrorAccumulator {
            errors,
            values,
            base,
        }
    }

    /// Provide a [`Constructor`] function that is called on
    /// [`analyse()`](ErrorAccumulatorFinisher::analyse) in case all recorded
    /// results (including nested results) where [`Ok`].
    ///
    /// - The input to the constructor are the recorded `Ok` values in order of
    ///   recording.
    /// - After providing the constructor no more results can be recorded.
    pub fn on_ok<C, Out>(self, constructor: C) -> ErrorAccumulatorFinisher<List, C, Out>
    where
        List: ToTuple,
        C: Constructor<List::List, Out>,
    {
        ErrorAccumulatorFinisher {
            accumulated_errors: self.errors,
            values: self.values,
            constructor,
            _marker: PhantomData,
        }
    }

    /// Analyse all recorded results.
    ///
    /// If at least one error was recorded the [`AccumulatedError`]s are
    /// returned else a tuple of all recorded `Ok` values in recording order is
    /// returned.
    pub fn analyse(self) -> Result<List::List, AccumulatedError>
    where
        List: ToTuple,
    {
        if self.errors.is_empty() {
            // Would only panic if there were any errors.
            Ok(self.values.unwrap_tuple())
        } else {
            Err(self.errors)
        }
    }
}

impl<List, Constr, Out> ErrorAccumulatorFinisher<List, Constr, Out>
where
    List: ToTuple,
    Constr: Constructor<List::List, Out>,
{
    /// Like [`ErrorAccumulator::analyse()`] but the recorded `Ok` values are
    /// processed by the provided [`Constructor`].
    pub fn analyse(self) -> Result<Out, AccumulatedError> {
        if self.accumulated_errors.is_empty() {
            // Would only panic if there were any errors.
            Ok(self.constructor.construct(self.values.unwrap_tuple()))
        } else {
            Err(self.accumulated_errors)
        }
    }
}

fn append_or_record<L, T, E>(
    list: L,
    path: &SourcePath,
    result: Result<T, E>,
    errors: &mut AccumulatedError,
) -> L::Output
where
    L: Append<T>,
    E: Error + Send + Sync + 'static,
{
    match result {
        Ok(value) => list.append(value),
        Err(error) => {
            errors.append(path.clone(), error);
            list.append(None)
        }
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;

    pub fn n(name: &str) -> FieldName {
        name.parse().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroI16, ParseIntError, TryFromIntError};

    use super::*;
    use crate::test_util::n;

    #[derive(Debug, PartialEq, Eq)]
    struct TestThing {
        num: u32,
        non_zero: NonZeroI16,
    }

    impl TestThing {
        fn new(num: u32, non_zero: NonZeroI16) -> Self {
            Self { num, non_zero }
        }
    }

    #[test]
    fn should_return_ok_values() {
        let (num, non_zero) = ErrorAccumulator::new()
            .field_builder(n("foo"))
            .value("42".parse::<u32>())
            .finish()
            .field_builder(n("bar"))
            .value(NonZeroI16::try_from(-5))
            .finish()
            .analyse()
            .unwrap();

        assert_eq!(num, 42);
        assert_eq!(non_zero.get(), -5);
    }

    #[test]
    fn should_return_on_one_error() {
        let err = ErrorAccumulator::new()
            .field("bar".parse().unwrap(), "42".parse::<u32>())
            .field("foo".parse().unwrap(), NonZeroI16::try_from(0))
            .analyse()
            .unwrap_err();

        assert_eq!(err.len(), 1);
    }

    #[test]
    fn should_return_multiple_errors() {
        let err = ErrorAccumulator::new()
            .field(n("foo"), "foo".parse::<u32>())
            .field(n("bar"), NonZeroI16::try_from(0))
            .analyse()
            .unwrap_err();

        assert_eq!(err.get_by_type::<ParseIntError>().count(), 1);
        assert_eq!(err.get_by_type::<TryFromIntError>().count(), 1);
    }

    #[test]
    fn should_allow_construction_on_success() {
        let thing = ErrorAccumulator::new()
            .field(n("bar"), "42".parse::<u32>())
            .field(n("foo"), NonZeroI16::try_from(-5))
            .on_ok(TestThing::new)
            .analyse()
            .unwrap();

        assert_eq!(thing, TestThing::new(42, NonZeroI16::new(-5).unwrap()))
    }
}
