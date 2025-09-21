use std::{error::Error, marker::PhantomData};

use crate::{
    builder::{ErrorBuilderParent, StructBuilder},
    cons::Nil,
    error::AccumulatedError,
    path::{FieldName, PathSegment, SourcePath},
};

/// A builder to record the parsing results of elements of an array in the
/// input.
///
/// Arrays can be composed of single values or nested structs.
#[derive(Debug)]
pub struct ArrayBuilder<Parent, Value> {
    parent: Parent,
    base: SourcePath,
    errors: AccumulatedError,
    array_name: FieldName,
    values: Vec<Value>,
    _marker: PhantomData<Value>,
}

impl<Parent, Value> ArrayBuilder<Parent, Value>
where
    Parent: ErrorBuilderParent<Vec<Value>>,
{
    pub(crate) fn new(parent: Parent, base: SourcePath, field: FieldName) -> Self {
        Self {
            base,
            parent,
            errors: Default::default(),
            array_name: field,
            values: Default::default(),
            _marker: PhantomData,
        }
    }

    /// Record an [`Iterator`] of parsing results for single values.
    pub fn of_values<E>(self, values: impl IntoIterator<Item = Result<Value, E>>) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        values
            .into_iter()
            .enumerate()
            .fold(self, |rec, (index, result)| rec.value(index, result))
    }

    /// Consume an [`Iterator`] of nested structs from the input recording
    /// errors while parsing.
    ///
    /// The provided `Parser` is a closure that receives a [`StructBuilder`] for
    /// the element that's passed into the parser as well. Use the
    /// `StructBuilder` to record any parsing results while processing the
    /// element.
    pub fn of_structs<I, T, Parser>(self, elements: I, mut parse: Parser) -> Self
    where
        I: IntoIterator<Item = T>,
        Parser: FnMut(StructBuilder<Self, Value, Nil>, T) -> Self,
    {
        elements
            .into_iter()
            .enumerate()
            .fold(self, |rec, (index, element)| {
                parse(rec.strukt(index), element)
            })
    }

    /// Record a parsing results for a single value within the array at a
    /// certain index.
    ///
    /// This is a low-level operation, consider using
    /// [`of_values()`](Self::of_values) instead.
    pub fn value<E>(mut self, index: usize, result: Result<Value, E>) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        match result {
            Ok(value) => self.values.push(value),
            Err(error) => {
                self.errors.append(self.element_path(index), error);
            }
        }

        self
    }

    /// Start a [`StructBuilder`] to record the parsing results for a nested
    /// struct within the array at a certain index.
    pub fn strukt(self, index: usize) -> StructBuilder<Self, Value, Nil> {
        let path = self.element_path(index);
        StructBuilder::new(self, path)
    }

    /// Finish the `ArrayBuilder` and pass the final result to the parent
    /// builder.
    pub fn finish(self) -> Parent::AfterRecord {
        let result = if self.errors.is_empty() {
            Ok(self.values)
        } else {
            Err(self.errors)
        };

        self.parent.finish_child_builder(result)
    }

    fn element_path(&self, index: usize) -> SourcePath {
        self.base.join(PathSegment::Array {
            name: self.array_name.clone(),
            index,
        })
    }
}

impl<Parent, Value> ErrorBuilderParent<Value> for ArrayBuilder<Parent, Value> {
    type AfterRecord = Self;

    fn finish_child_builder(
        mut self,
        child_result: Result<Value, AccumulatedError>,
    ) -> Self::AfterRecord {
        match child_result {
            Ok(value) => self.values.push(value),
            Err(errors) => {
                self.errors.merge(errors);
            }
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorAccumulator, test_util::n};

    #[derive(Debug, PartialEq, Eq)]
    struct Test(u32);

    #[test]
    fn should_record_array_of_structs() {
        let (res,) = ErrorAccumulator::new()
            .array(n("foo"))
            .of_structs(vec!["42", "21", "33"], |rec, value| {
                rec.field(n("num"), value.parse()).on_ok(Test).finish()
            })
            .finish()
            .analyse()
            .unwrap();

        assert_eq!(vec![Test(42), Test(21), Test(33)], res);
    }

    #[test]
    fn should_record_array_of_values() {
        let (res,) = ErrorAccumulator::new()
            .array(n("foo"))
            .of_values(vec!["42".parse(), "21".parse(), "33".parse()])
            .finish()
            .analyse()
            .unwrap();

        assert_eq!(vec![42, 21, 33], res);
    }

    #[test]
    fn should_record_error_in_array() {
        let res = ErrorAccumulator::new()
            .array(n("foo"))
            .of_values(vec!["42".parse::<u32>(), "aa".parse(), "bb".parse()])
            .finish()
            .analyse()
            .unwrap_err();

        assert_eq!(
            res.get_by_path(&SourcePath::new().join(PathSegment::Array {
                name: n("foo"),
                index: 1
            }))
            .count(),
            1
        );
        assert_eq!(
            res.get_by_path(&SourcePath::new().join(PathSegment::Array {
                name: n("foo"),
                index: 2
            }))
            .count(),
            1
        );
    }
}
