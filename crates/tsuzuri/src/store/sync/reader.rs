use crate::store::{payload::Payload, sync::error::StoreError};
use async_trait::async_trait;
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

#[async_trait]
pub trait Reader: 'static + Send + Sync {
    async fn read(&self, id: &str, seq: usize) -> Result<Payload, StoreError>;
    async fn read_to(&self, id: &str, from: usize, to: usize) -> Result<BTreeSet<Payload>, StoreError>;
    async fn read_to_latest(&self, id: &str, from: usize) -> Result<BTreeSet<Payload>, StoreError> {
        self.read_to(id, from, usize::MAX).await
    }
}

// リードクエリ
#[async_trait]
pub trait Query: 'static + Send + Sync {}

pub struct DefaultQuery {}

#[async_trait]
impl Query for DefaultQuery {}

pub struct ReadStore {
    base: Arc<dyn Reader>,
    queries: Vec<Box<dyn Query>>,
}

impl Debug for ReadStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadStore").finish()
    }
}

impl Clone for ReadStore {
    fn clone(&self) -> Self {
        Self {
            base: Arc::clone(&self.base),
            queries: vec![],
        }
    }
}

impl ReadStore {
    pub fn new(reader: impl Reader, queries: impl Query) -> Self {
        Self {
            base: Arc::new(reader),
            queries: vec![Box::new(queries)],
        }
    }

    pub async fn read(&self, id: &str, seq: usize) -> Result<Payload, StoreError> {
        self.base.read(id, seq).await
    }

    pub async fn read_to(&self, id: &str, from: usize, to: usize) -> Result<BTreeSet<Payload>, StoreError> {
        self.base.read_to(id, from, to).await
    }

    pub async fn read_to_latest(&self, id: &str, from: usize) -> Result<BTreeSet<Payload>, StoreError> {
        self.base.read_to_latest(id, from).await
    }
}
