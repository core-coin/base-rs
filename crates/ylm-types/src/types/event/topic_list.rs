use crate::{abi::token::WordToken, Error, Result, YlmType};
use alloc::borrow::Cow;

mod sealed {
    pub trait Sealed {}
}
use sealed::Sealed;

/// A list of Ylem event topics.
///
/// This trait is implemented only on tuples of arity up to 4. The tuples must
/// contain only [`YlmType`]s where the token is a [`WordToken`], and as such
/// it is sealed to prevent prevent incorrect downstream implementations.
///
/// See the [Ylem event ABI specification][YlmEvent] for more details on how
/// events' topics are encoded.
///
/// [YlmEvent]: https://docs.soliditylang.org/en/latest/abi-spec.html#events
///
/// # Implementer's Guide
///
/// It should not be necessary to implement this trait manually. Instead, use
/// the [`ylm!`](crate::ylm!) procedural macro to parse Ylem syntax into
/// types that implement this trait.
pub trait TopicList: YlmType + Sealed {
    /// The number of topics.
    const COUNT: usize;

    /// Detokenize the topics into a tuple of rust types.
    ///
    /// This function accepts an iterator of `WordToken`.
    fn detokenize<I, D>(topics: I) -> Result<Self::RustType>
    where
        I: IntoIterator<Item = D>,
        D: Into<WordToken>;
}

macro_rules! impl_topic_list_tuples {
    ($($c:literal => $($lt:lifetime $t:ident),*;)+) => {$(
        impl<$($t,)*> Sealed for ($($t,)*) {}
        impl<$($lt,)* $($t: YlmType<Token<$lt> = WordToken>,)*> TopicList for ($($t,)*) {
            const COUNT: usize = $c;

            fn detokenize<I, D>(topics: I) -> Result<Self::RustType>
            where
                I: IntoIterator<Item = D>,
                D: Into<WordToken>
            {
                let err = || Error::Other(Cow::Borrowed("topic list length mismatch"));
                let mut iter = topics.into_iter();
                Ok(($(
                    <$t>::detokenize(iter.next().ok_or_else(err)?.into()),
                )*))
            }
        }
    )+};
}

impl Sealed for () {}
impl TopicList for () {
    const COUNT: usize = 0;

    #[inline]
    fn detokenize<I, D>(_: I) -> Result<Self::RustType>
    where
        I: IntoIterator<Item = D>,
        D: Into<WordToken>,
    {
        Ok(())
    }
}

impl_topic_list_tuples! {
    1 => 'a T;
    2 => 'a T, 'b U;
    3 => 'a T, 'b U, 'c V;
    4 => 'a T, 'b U, 'c V, 'd W;
}
