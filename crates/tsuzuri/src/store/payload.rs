use std::cmp::Ordering;
// use std::error::Error;
use time::OffsetDateTime;

#[derive(Debug)]
// pub struct SerializeError(Box<dyn Error + Sync + Send>);
pub struct SerializeError;

/// Basic format of the data to be saved.
#[derive(Debug, Clone)]
// #[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Payload {
    /// Aggregate entity identifier
    pub id: String,
    /// The sequence number for an aggregate instance.
    pub sequence: usize,
    /// Data body in binary format
    pub bytes: Vec<u8>,
    pub metadata: Option<Vec<u8>>,
    /// Time the Event was generated
    pub created_at: OffsetDateTime,
}

impl Payload {
    pub fn new(id: &str, seq: usize, event: Vec<u8>, metadata: Option<Vec<u8>>) -> Result<Self, SerializeError> {
        Ok(Self {
            id: id.to_string(),
            sequence: seq,
            bytes: event,
            metadata,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

impl Eq for Payload {}

impl PartialEq<Self> for Payload {
    fn eq(&self, other: &Self) -> bool {
        self.sequence.eq(&other.sequence) && self.id.eq(&other.id) && self.created_at.eq(&other.created_at)
    }
}

impl PartialOrd<Self> for Payload {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Payload {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sequence
            .cmp(&other.sequence)
            .then_with(|| self.created_at.cmp(&other.created_at))
            .then_with(|| self.id.cmp(&other.id))
    }
}
