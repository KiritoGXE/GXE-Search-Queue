use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub payload: String,
    pub status: TaskStatus,
    pub retry_count: u32,
}

impl Task {
    pub fn new(id: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            payload: payload.into(),
            status: TaskStatus::Pending,
            retry_count: 0,
        }
    }
}
