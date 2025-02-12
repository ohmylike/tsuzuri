use crate::store::{
    payload::Payload,
    sync::{error::StoreError, reader::Reader, writer::Writer},
};
use async_trait::async_trait;
use std::io;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};
use tokio::sync::RwLock;

/// A simple in-memory store that keeps payloads organized by `id` and sequence number.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    // The inner store maps an `id` (String) to a BTreeMap where the key is the sequence number.
    store: Arc<RwLock<HashMap<String, BTreeMap<usize, Payload>>>>,
}

impl MemoryStore {
    /// Create a new empty MemoryStore.
    pub fn new() -> Self {
        MemoryStore {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Append a new payload to the store.
    ///
    /// Returns an error if a payload with the same sequence number already exists for the given id.
    pub async fn append(&self, id: &str, payload: Payload) -> Result<(), StoreError> {
        let mut store = self.store.write().await;
        let entry = store.entry(id.to_string()).or_insert_with(BTreeMap::new);
        if entry.contains_key(&payload.sequence) {
            return Err(StoreError::Write(Box::new(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "Payload with sequence {} already exists for id {}",
                    payload.sequence, id
                ),
            ))));
        }
        entry.insert(payload.sequence, payload);
        Ok(())
    }
}

#[async_trait]
impl Reader for MemoryStore {
    async fn read(&self, id: &str, seq: usize) -> Result<Payload, StoreError> {
        let store = self.store.read().await;
        if let Some(map) = store.get(id) {
            if let Some(payload) = map.get(&seq) {
                return Ok(payload.clone());
            }
        }
        Err(StoreError::Read(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Payload not found for id: {} and sequence: {}", id, seq),
        ))))
    }

    async fn read_to(&self, id: &str, from: usize, to: usize) -> Result<BTreeSet<Payload>, StoreError> {
        let store = self.store.read().await;
        // 存在しない場合は空のBTreeSetを返す
        let set: BTreeSet<_> = if let Some(map) = store.get(id) {
            map.range(from..to).map(|(_seq, payload)| payload.clone()).collect()
        } else {
            BTreeSet::new()
        };
        Ok(set)
    }
}

#[async_trait]
impl Writer for MemoryStore {
    async fn write(&self, id: &str, payload: Payload) -> Result<(), StoreError> {
        self.append(id, payload).await
    }
}
