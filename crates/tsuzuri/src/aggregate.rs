use crate::error::Result;

/// Represents an aggregate root in an event-sourced system.
///
/// An aggregate root is the entry-point for the cluster of entities and that
/// are changed together in response to commands.
pub trait Aggregate: std::fmt::Debug + Send + Sync {
    type Command;
    type Event: Clone + std::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned;

    /// Initializes an aggregate with the given identifier.
    ///
    /// This method is called to create a new instance of an aggregate root
    /// with a default state.
    fn init(id: String) -> Self;
}

/// Handles a command, returning events.
///
/// Commands use the aggregates state to validate business rules, and returns
/// events which are later used to update the aggregate state.
pub trait Handle<C>: Aggregate {
    type Error: std::fmt::Debug;

    fn handle(&self, cmd: C) -> Result<Vec<Self::Event>, Self::Error>;
}

/// Applies an event, updating the aggregate state.
///
/// Events modify aggregate state, and are emitted as the result of commands.
pub trait Apply<E>: Aggregate {
    fn apply(&mut self, event: E);
}

// Aggregateそのものは不変であり、内部変更性を確保するために、Stateでラップする
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct State<T>(pub T);

impl<T> Aggregate for State<T>
where
    T: Aggregate,
{
    type Command = T::Command;
    type Event = T::Event;

    fn init(id: String) -> Self {
        State(T::init(id))
    }
}
