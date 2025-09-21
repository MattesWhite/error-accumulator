//! Builder to parse input and accumulate errors.

use std::marker::PhantomData;

use crate::{AccumulatedError, Constructor, cons::ToTuple};

mod array;
mod field;
mod strukt;

pub use self::{array::ArrayBuilder, field::FieldBuilder, strukt::StructBuilder};

/// Parent builders can have child builders to simulate nested structures.
pub trait ErrorBuilderParent<T> {
    /// The parent builder after the child builder finished.
    ///
    /// Due to the strongly typed nature of most builders it is common that
    /// after the final result of the child builder is recorded the parent is
    /// turned into another type.
    type AfterRecord;

    /// Record the final result of the child builder.
    fn finish_child_builder(self, child_result: Result<T, AccumulatedError>) -> Self::AfterRecord;
}

/// Intermediate state when either [`FieldBuilder::on_ok()`] or
/// [`StructBuilder::on_ok()`] were called.
///
/// See those method's documentations for more details.
#[derive(Debug)]
pub struct BuilderFinisher<Parent, Out, List, Constructor> {
    parent: Parent,
    accumulated_errors: AccumulatedError,
    values: List,
    constructor: Constructor,
    _marker: PhantomData<Out>,
}

impl<Parent, Value, List, C> BuilderFinisher<Parent, Value, List, C>
where
    Parent: ErrorBuilderParent<Value>,
    List: ToTuple,
    C: Constructor<List::List, Value>,
{
    /// Finish the wrapped builder and pass the final result to the parent
    /// builder.
    pub fn finish(self) -> Parent::AfterRecord {
        let result = if self.accumulated_errors.is_empty() {
            Ok(self.constructor.construct(self.values.unwrap_tuple()))
        } else {
            Err(self.accumulated_errors)
        };

        self.parent.finish_child_builder(result)
    }
}
