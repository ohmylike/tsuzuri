use crate::store::{payload::Payload, sync::error::StoreError};
use async_trait::async_trait;
use std::{fmt::Debug, sync::Arc};

#[async_trait]
pub trait Writer: 'static + Sync + Send {
    async fn write(&self, id: &str, payload: Payload) -> Result<(), StoreError>;
}

pub struct WriteStore {
    base: Arc<dyn Writer>,
}

impl Debug for WriteStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteStore").finish()
    }
}

impl Clone for WriteStore {
    fn clone(&self) -> Self {
        Self {
            base: Arc::clone(&self.base),
        }
    }
}

impl WriteStore {
    pub fn new(writer: impl Writer) -> Self {
        Self { base: Arc::new(writer) }
    }

    pub async fn write(&self, id: &str, payload: Payload) -> Result<(), StoreError> {
        self.base.write(id, payload).await
    }
}
