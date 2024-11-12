use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub id: u64,
    pub payload: String,
}