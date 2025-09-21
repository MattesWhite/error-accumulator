pub trait Append<E> {
    type Output;

    fn append(self, elem: impl Into<Option<E>>) -> Self::Output;
}

pub trait ToTuple {
    type List;

    fn unwrap_tuple(self) -> Self::List;
}

pub trait AsRefTuple {
    type Ref<'t>
    where
        Self: 't;

    fn as_unwraped_tuple(&self) -> Self::Ref<'_>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nil;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cons<Head, Tail> {
    head: Option<Head>,
    tail: Tail,
}

impl<E> Append<E> for Nil {
    type Output = Cons<E, Nil>;

    fn append(self, elem: impl Into<Option<E>>) -> Self::Output {
        Cons {
            head: elem.into(),
            tail: Nil,
        }
    }
}

impl ToTuple for Nil {
    type List = ();

    fn unwrap_tuple(self) -> Self::List {}
}

impl AsRefTuple for Nil {
    type Ref<'t> = ();

    fn as_unwraped_tuple(&self) -> Self::Ref<'_> {}
}

impl<H, T, E> Append<E> for Cons<H, T>
where
    T: Append<E>,
{
    type Output = Cons<H, <T as Append<E>>::Output>;

    fn append(self, elem: impl Into<Option<E>>) -> Self::Output {
        Cons {
            head: self.head,
            tail: self.tail.append(elem),
        }
    }
}

impl<A> ToTuple for Cons<A, Nil> {
    type List = (A,);

    fn unwrap_tuple(self) -> Self::List {
        (self.head.unwrap(),)
    }
}

impl<A> AsRefTuple for Cons<A, Nil> {
    type Ref<'t>
        = (&'t A,)
    where
        A: 't;

    fn as_unwraped_tuple(&self) -> Self::Ref<'_> {
        (self.head.as_ref().unwrap(),)
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

macro_rules! tuple_expr {
    ($base:expr, $head:ident, $($rest:ident),+) => {
        tuple_expr!(@build ( $head, $( $rest ),+ ) [] $base)
    };
    (@build ($head:ident, $($rest:ident),+) [ $(, $level:expr)*] $last:expr) => {
        tuple_expr!(@build ($( $rest ),+) [$(, $level )*, $last] $last.tail)
    };
    // Recursion end
    (@build ($head:ident) [$(, $level:expr)*] $last:expr) => {
        ( $( $level.head.unwrap() ),*, $last.head.unwrap() )
    };
}

macro_rules! tuple_ref_expr {
    ($base:expr, $head:ident, $($rest:ident),+) => {
        tuple_ref_expr!(@build ( $head, $( $rest ),+ ) [] $base)
    };
    (@build ($head:ident, $($rest:ident),+) [ $(, $level:expr)*] $last:expr) => {
        tuple_ref_expr!(@build ($( $rest ),+) [$(, $level )*, $last] $last.tail)
    };
    // Recursion end
    (@build ($head:ident) [$(, $level:expr)*] $last:expr) => {
        ( $( $level.head.as_ref().unwrap() ),*, $last.head.as_ref().unwrap() )
    };
}

macro_rules! impl_to_tuple {
    ( $($elem:ident),+ ) => {
        impl< $( $elem ),+ > ToTuple for list_type!( $( $elem ),+ ) {
            type List = ( $( $elem ),+ );

            fn unwrap_tuple(self) -> Self::List {
                tuple_expr!(self, $( $elem ),+)
            }
        }

        impl< $( $elem ),+ > AsRefTuple for list_type!( $( $elem ),+ )

        {
            type Ref<'t> = ( $( &'t $elem ),+ )
            where
                $( $elem : 't ),+;

            fn as_unwraped_tuple(&self) -> Self::Ref<'_> {
                tuple_ref_expr!(self, $( $elem ),+)
            }
        }
    };
}

impl_to_tuple!(A, B);
impl_to_tuple!(A, B, C);
impl_to_tuple!(A, B, C, D);
impl_to_tuple!(A, B, C, D, E);
impl_to_tuple!(A, B, C, D, E, F);
impl_to_tuple!(A, B, C, D, E, F, G);
impl_to_tuple!(A, B, C, D, E, F, G, H);
impl_to_tuple!(A, B, C, D, E, F, G, H, I);
impl_to_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_to_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_to_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_4() {
        let list = Nil
            .append(true)
            .append(4)
            .append('c')
            .append("str")
            .unwrap_tuple();
        assert!(list.0);
        assert_eq!(list.1, 4);
        assert_eq!(list.2, 'c');
        assert_eq!(list.3, "str");
    }
}
