//! Helper traits to enable some of this crate's type magic.

use std::{convert::Infallible, error::Error};

use crate::cons::{AsRefTuple, Cons, Nil};

/// Marker trait for types that can turn a list of values into something else.
///
/// This is used for `on_ok()` methods so the provided closures can take each
/// recorded value as separate argument instead of deconstructing tuples.
///
/// There are default implementations for [`FnMut`] closures with up to 10
/// arguments.
pub trait Constructor<In, Out> {
    /// Take the input and convert it into output.
    fn construct(self, input: In) -> Out;
}

/// Marker trait for types that can validate a list of values into something
/// potentially fallible.
///
/// There are default implementations for [`FnMut`] closures with up to 10
/// reference arguments.
pub trait ListValidator<List, Out, Err> {
    /// Transform an input. May fails doing so.
    fn validate(self, values: &List) -> Result<Out, Err>;
}

impl<Out, F> Constructor<(), Out> for F
where
    F: FnMut() -> Out,
{
    fn construct(mut self, _: ()) -> Out {
        self()
    }
}

impl<A, Out, F> Constructor<(A,), Out> for F
where
    F: FnMut(A) -> Out,
{
    fn construct(mut self, (a,): (A,)) -> Out {
        self(a)
    }
}

macro_rules! impl_constructor {
    ($($elem:ident),+) => {
        impl< $( $elem ),+ , Out, Func> Constructor<($( $elem ),+), Out> for Func
        where
            Func: FnMut( $( $elem ),+ ) -> Out,
        {
            #[allow(non_snake_case)]
            fn construct(mut self, ( $( $elem ),+ ): ( $( $elem ),+ )) -> Out {
                self( $( $elem ),+ )
            }
        }
    };
}

impl_constructor!(A, B);
impl_constructor!(A, B, C);
impl_constructor!(A, B, C, D);
impl_constructor!(A, B, C, D, E);
impl_constructor!(A, B, C, D, E, F);
impl_constructor!(A, B, C, D, E, F, G);
impl_constructor!(A, B, C, D, E, F, G, H);
impl_constructor!(A, B, C, D, E, F, G, H, I);
impl_constructor!(A, B, C, D, E, F, G, H, I, J);
impl_constructor!(A, B, C, D, E, F, G, H, I, J, K);
impl_constructor!(A, B, C, D, E, F, G, H, I, J, K, L);

impl<Out, Func> ListValidator<Nil, Out, Infallible> for Func
where
    Func: FnMut() -> Result<Out, Infallible>,
{
    fn validate(mut self, _: &Nil) -> Result<Out, Infallible> {
        self()
    }
}

impl<A, Out, Err, Func> ListValidator<Cons<A, Nil>, Out, Err> for Func
where
    Func: for<'a> FnMut(&'a A) -> Result<Out, Err>,
    Err: Error + Send + Sync + 'static,
{
    fn validate(mut self, values: &Cons<A, Nil>) -> Result<Out, Err> {
        let (a,) = values.as_unwraped_tuple();
        self(a)
    }
}

macro_rules! list_type {
    ($head:ident, $($tail:ident),*) => {
        Cons< $head, list_type!( $( $tail ),* ) >
    };
    ($head:ident) => {
        Cons< $head, Nil >
    };
    () => {
        Nil
    };
}

macro_rules! impl_validator {
    ($($elem:ident),+) => {
        impl<$( $elem ),+ , Out, Err, Func> ListValidator<list_type!( $( $elem ),+ ), Out, Err> for Func
        where
            Func: for<'a> FnMut( $( &'a $elem ),+ ) -> Result<Out, Err>,
            Err: Error + Send + Sync + 'static,
        {
            #[allow(non_snake_case)]
            fn validate(mut self, values: & list_type!( $( $elem ),+ )) -> Result<Out, Err> {
                let ( $( $elem ),+ ) = $crate::cons::AsRefTuple::as_unwraped_tuple(values);
                self( $( $elem ),+ )
            }
        }
    };
}

impl_validator!(A, B);
impl_validator!(A, B, C);
impl_validator!(A, B, C, D);
impl_validator!(A, B, C, D, E);
impl_validator!(A, B, C, D, E, F);
impl_validator!(A, B, C, D, E, F, G);
impl_validator!(A, B, C, D, E, F, G, H);
impl_validator!(A, B, C, D, E, F, G, H, I);
impl_validator!(A, B, C, D, E, F, G, H, I, J);
impl_validator!(A, B, C, D, E, F, G, H, I, J, K);
impl_validator!(A, B, C, D, E, F, G, H, I, J, K, L);
