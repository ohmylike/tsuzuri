use crate::store::sync::{
    reader::{DefaultQuery, ReadStore, Reader},
    writer::{WriteStore, Writer},
};

pub struct EventStore {
    pub read_store: ReadStore,
    pub write_store: WriteStore,
}

impl EventStore {
    pub fn new<S>(store: S) -> Self
    where
        S: Reader + Writer + Clone + 'static,
    {
        EventStore {
            read_store: ReadStore::new(store.clone(), DefaultQuery {}),
            write_store: WriteStore::new(store),
        }
    }
}
