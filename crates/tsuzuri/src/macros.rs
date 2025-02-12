/// Shorthand for creating a `Ok(vec![...])` in [`Handle`](crate::Handle)
/// implmentations.
///
/// Each expression in the macro is automcically converted to the aggregates
/// `Event` type using [std::convert::From].
#[macro_export]
macro_rules! events {
    () => {
        ::std::result::Result::Ok(::std::vec![])
    };
    ($($x:expr),+ $(,)?) => {
        ::std::result::Result::Ok(
            ::std::vec![
                $( <<Self as $crate::aggregate::Aggregate>::Event as ::std::convert::From<_>>::from($x) ),+
            ]
        )
    };
}
