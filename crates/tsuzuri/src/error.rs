// use std::error;

#[derive(Debug)]
pub enum Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// /// The base error for the framework.
// #[derive(Debug, thiserror::Error)]
// pub enum AggregateError<T: error::Error> {
//     #[error("{0}")]
//     UserError(T),
// }
