use std::{error::Error, marker::PhantomData};

use crate::{
    append_or_record,
    builder::{ArrayBuilder, BuilderFinisher, ErrorBuilderParent, FieldBuilder},
    cons::{Append, AsRefTuple, Nil, ToTuple},
    construct::{Constructor, ListValidator},
    error::AccumulatedError,
    path::{FieldName, PathSegment, SourcePath},
};

/// A builder to record parsing results for a nested struct in the input.
///
/// A `StructBuilder` can have other nested structs, arrays, and fields.
#[derive(Debug)]
pub struct StructBuilder<Parent, Value, List> {
    parent: Parent,
    errors: AccumulatedError,
    struct_path: SourcePath,
    values: List,
    _marker: PhantomData<Value>,
}

impl<Parent, Value> StructBuilder<Parent, Value, Nil>
where
    Parent: ErrorBuilderParent<Value>,
{
    pub(crate) fn new(parent: Parent, base: SourcePath) -> Self {
        Self {
            struct_path: base,
            parent,
            errors: Default::default(),
            values: Nil,
            _marker: PhantomData,
        }
    }
}

impl<Parent, Value, List> StructBuilder<Parent, Value, List>
where
    Parent: ErrorBuilderParent<Value>,
{
    /// Record a parsing result for a field in this struct.
    pub fn field<T, E>(
        self,
        field: FieldName,
        result: Result<T, E>,
    ) -> StructBuilder<Parent, Value, List::Output>
    where
        List: Append<T>,
        E: Error + Send + Sync + 'static,
        Self: ErrorBuilderParent<T, AfterRecord = StructBuilder<Parent, Value, List::Output>>,
    {
        let field_path = self.struct_path.join(PathSegment::Field(field));
        FieldBuilder::new(self, field_path).value(result).finish()
    }

    /// Start a [`FieldBuilder`] to record the parsing results for a field in
    /// this struct.
    pub fn field_builder<FieldValue>(self, field: FieldName) -> FieldBuilder<Self, FieldValue, Nil>
    where
        List: Append<FieldValue>,
    {
        let field_path = self.struct_path.join(PathSegment::Field(field));
        FieldBuilder::new(self, field_path)
    }

    /// Start a [`StructBuilder`] to record the parsing results of a nested
    /// struct within the current one.
    pub fn strukt<StructValue>(self, field: FieldName) -> StructBuilder<Self, StructValue, Nil>
    where
        List: Append<StructValue>,
    {
        let base = self.struct_path.join(PathSegment::Field(field));
        StructBuilder::new(self, base)
    }

    /// Start an [`ArrayBuilder`] to record the parsing results for a nested
    /// array within the current struct.
    pub fn array<ElementValue>(self, field: FieldName) -> ArrayBuilder<Self, ElementValue>
    where
        List: Append<Vec<ElementValue>>,
    {
        let base = self.struct_path.clone();
        ArrayBuilder::new(self, base, field)
    }

    /// Run another validation step on the previously recorded `Ok` values if
    /// there were no errors yet.
    ///
    /// In case an error was already recorded the `validator` is not executed.
    ///
    /// There are blanked implementations for [`ListValidator`] for closures
    /// that take the references to `Ok` values as arguments, e.g.
    ///
    /// ```
    /// # use std::{convert::Infallible, num::NonZeroU16};
    /// # use error_accumulator::{ErrorAccumulator, path::FieldName};
    /// # const FOO: FieldName = FieldName::new_unchecked("foo");
    /// # const BAR: FieldName = FieldName::new_unchecked("bar");
    /// # const BAZ: FieldName = FieldName::new_unchecked("baz");
    /// let res = ErrorAccumulator::new().strukt(FOO)
    ///     .field(BAR, NonZeroU16::try_from(16))
    ///     .field(BAZ, NonZeroU16::try_from(8))
    ///     // for some reason the type annotation with `&_` is required to make this compile
    ///     .with_previous(|bar: &_, baz: &_|
    ///         Ok::<_, Infallible>(format!("{bar}{baz}"))
    ///     )
    ///     .on_ok(|_, _, res| res)
    ///     .finish()
    ///     .analyse()
    ///     .unwrap()
    ///     .0;
    /// assert_eq!(res.as_str(), "168");
    /// ```
    pub fn with_previous<Valid, T, E>(
        self,
        validator: Valid,
    ) -> StructBuilder<Parent, Value, List::Output>
    where
        Valid: ListValidator<List, T, E>,
        List: AsRefTuple + Append<T>,
        E: Error + Send + Sync + 'static,
    {
        let Self {
            parent,
            mut errors,
            struct_path,
            values,
            _marker,
        } = self;

        let values = if errors.is_empty() {
            let result = validator.validate(&values);
            append_or_record(values, &struct_path, result, &mut errors)
        } else {
            values.append(None)
        };

        StructBuilder {
            parent,
            errors,
            struct_path,
            values,
            _marker,
        }
    }

    /// Provide a [`Constructor`] to build a struct values from all the recorded
    /// `Ok` values of the builder.
    pub fn on_ok<C>(self, constructor: C) -> BuilderFinisher<Parent, Value, List, C>
    where
        List: ToTuple,
        C: Constructor<List::List, Value>,
    {
        BuilderFinisher {
            parent: self.parent,
            accumulated_errors: self.errors,
            values: self.values,
            constructor,
            _marker: PhantomData,
        }
    }
}

impl<Parent, OwnValue, ChildValue, List> ErrorBuilderParent<ChildValue>
    for StructBuilder<Parent, OwnValue, List>
where
    List: Append<ChildValue>,
{
    type AfterRecord = StructBuilder<Parent, OwnValue, List::Output>;

    fn finish_child_builder(
        self,
        child_result: Result<ChildValue, AccumulatedError>,
    ) -> Self::AfterRecord {
        let Self {
            parent,
            mut errors,
            struct_path,
            values,
            _marker,
        } = self;

        let values = match child_result {
            Ok(value) => values.append(value),
            Err(child_errors) => {
                errors.merge(child_errors);
                values.append(None)
            }
        };

        StructBuilder {
            parent,
            errors,
            struct_path,
            values,
            _marker,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io, num::NonZeroI16};

    use super::*;
    use crate::{ErrorAccumulator, test_util::n};

    #[test]
    fn should_record_nested_structs() {
        let foo_struct = ErrorAccumulator::new().strukt(n("foo"));
        let foo_struct = foo_struct
            .field_builder(n("bar"))
            .value("42".parse::<u32>())
            .value(NonZeroI16::try_from(-5))
            .on_ok(|v, _| v)
            .finish();
        let foo_baz_struct = foo_struct
            .strukt(n("baz"))
            .field(n("quux"), Ok::<_, io::Error>("god"))
            .on_ok(|s: &str| s.chars().rev().collect::<String>())
            .finish();

        let (res,) = foo_baz_struct
            .on_ok(|num, s| format!("{num}|{s}"))
            .finish()
            .analyse()
            .unwrap();

        assert_eq!(res.as_str(), "42|dog")
    }

    #[test]
    fn should_record_nested_error() {
        let foo_struct = ErrorAccumulator::new().strukt(n("foo"));
        let foo_struct = foo_struct
            .field_builder(n("bar"))
            .value("42".parse::<u32>())
            .value(NonZeroI16::try_from(-5))
            .on_ok(|v, _| v)
            .finish();
        let foo_baz_struct = foo_struct
            .strukt(n("baz"))
            .field(
                n("quux"),
                Err::<&str, _>(io::Error::new(io::ErrorKind::AddrInUse, "bad")),
            )
            .on_ok(|s: &str| s.chars().rev().collect::<String>())
            .finish();

        let res = foo_baz_struct
            .on_ok(|num, s| format!("{num}|{s}"))
            .finish()
            .analyse()
            .unwrap_err();

        assert_eq!(
            res.get_by_path(
                &SourcePath::new()
                    .join(PathSegment::Field(n("foo")))
                    .join(PathSegment::Field(n("baz")))
                    .join(PathSegment::Field(n("quux")))
            )
            .count(),
            1
        );
    }
}
