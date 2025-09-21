use std::{error::Error, marker::PhantomData};

use crate::{
    append_or_record,
    builder::{BuilderFinisher, ErrorBuilderParent},
    cons::{Append, AsRefTuple, Cons, Nil, ToTuple},
    construct::{Constructor, ListValidator},
    error::AccumulatedError,
    path::SourcePath,
};

/// A builder to record parsing results for a field of the input.
#[derive(Debug)]
pub struct FieldBuilder<Parent, Value, List> {
    parent: Parent,
    errors: AccumulatedError,
    field: SourcePath,
    values: List,
    _marker: PhantomData<Value>,
}

impl<Parent, Value> FieldBuilder<Parent, Value, Nil>
where
    Parent: ErrorBuilderParent<Value>,
{
    pub(crate) fn new(parent: Parent, path: SourcePath) -> Self {
        Self {
            field: path,
            parent,
            errors: Default::default(),
            values: Nil,
            _marker: PhantomData,
        }
    }
}

impl<Parent, Value, List> FieldBuilder<Parent, Value, List>
where
    Parent: ErrorBuilderParent<Value>,
{
    /// Record a parsing result for the field.
    pub fn value<T, E>(self, result: Result<T, E>) -> FieldBuilder<Parent, Value, List::Output>
    where
        List: Append<T>,
        E: Error + Send + Sync + 'static,
    {
        let Self {
            parent,
            mut errors,
            field,
            values,
            _marker,
        } = self;

        let values = append_or_record(values, &field, result, &mut errors);

        FieldBuilder {
            parent,
            errors,
            field,
            values,
            _marker,
        }
    }

    /// Record a value for the field.
    ///
    /// This is infallible so it's easy to insert values that do not need
    /// parsing in the [`ErrorAccumulator`](crate::ErrorAccumulator)'s system.
    pub fn value_ok<T>(self, value: T) -> FieldBuilder<Parent, Value, List::Output>
    where
        List: Append<T>,
    {
        let Self {
            parent,
            errors,
            field,
            values,
            _marker,
        } = self;

        let values = values.append(value);

        FieldBuilder {
            parent,
            errors,
            field,
            values,
            _marker,
        }
    }

    /// Run another validation step on the previously recorded `Ok` values if
    /// there were no errors yet.
    ///
    /// In case an error was already recorded the `validator` is not executed.
    ///
    /// For an example, see the docs of
    /// [`StructBuilder::with_previous()`](crate::StructBuilder::with_previous).
    pub fn with_previous<Valid, T, E>(
        self,
        validator: Valid,
    ) -> FieldBuilder<Parent, Value, List::Output>
    where
        Valid: ListValidator<List, T, E>,
        List: AsRefTuple + Append<T>,
        E: Error + Send + Sync + 'static,
    {
        let Self {
            parent,
            mut errors,
            field,
            values,
            _marker,
        } = self;

        let values = if errors.is_empty() {
            let result = validator.validate(&values);
            append_or_record(values, &field, result, &mut errors)
        } else {
            values.append(None)
        };

        FieldBuilder {
            parent,
            errors,
            field,
            values,
            _marker,
        }
    }

    /// Provide a [`Constructor`] to convert the recorded `Ok` values into the
    /// target type.
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

impl<Parent, Value> FieldBuilder<Parent, Value, Cons<Value, Nil>>
where
    Parent: ErrorBuilderParent<Value>,
{
    /// Finish the builder and pass the builder's final result to the parent
    /// builder.
    pub fn finish(self) -> Parent::AfterRecord {
        let result = if self.errors.is_empty() {
            let (value,) = self.values.unwrap_tuple();
            Ok(value)
        } else {
            Err(self.errors)
        };

        self.parent.finish_child_builder(result)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroI16;

    use crate::{ErrorAccumulator, test_util::n};

    #[test]
    fn should_allow_multivalue_field_record() {
        let (num,) = ErrorAccumulator::new()
            .field_builder(n("foo"))
            .value("42".parse::<u32>())
            .value(NonZeroI16::try_from(-5))
            .on_ok(|v, _| v)
            .finish()
            .analyse()
            .unwrap();

        assert_eq!(num, 42);
    }

    #[test]
    fn should_return_error_on_multivalue_field_record() {
        let err = ErrorAccumulator::new()
            .field_builder(n("bar"))
            .value("42".parse::<u32>())
            .value(NonZeroI16::try_from(0))
            .on_ok(|v, _| v)
            .finish()
            .analyse()
            .unwrap_err();

        assert_eq!(err.len(), 1);
    }
}
