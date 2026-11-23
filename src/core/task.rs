use serde::{Deserialize, Serialize};

// Retained temporarily for the repository's original queue smoke fixture.
// Product queue records are QueueJob rows persisted in SQLite.
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Movie,
    Episode,
}

impl MediaType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Episode => "episode",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueueJobState {
    WaitingMetadata,
    WaitingRelease,
    Queued,
    RetryWait,
    Running,
    Suspended,
    Satisfied,
    Completed,
    Cancelled,
    Dismissed,
}

impl QueueJobState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WaitingMetadata => "waiting_metadata",
            Self::WaitingRelease => "waiting_release",
            Self::Queued => "queued",
            Self::RetryWait => "retry_wait",
            Self::Running => "running",
            Self::Suspended => "suspended",
            Self::Satisfied => "satisfied",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Dismissed => "dismissed",
        }
    }

    pub fn can_transition_to(&self, next: &Self) -> bool {
        use QueueJobState::*;
        matches!(
            (self, next),
            (
                WaitingMetadata,
                WaitingRelease | Queued | Suspended | Satisfied | Cancelled
            ) | (WaitingRelease, Queued | Suspended | Satisfied | Cancelled)
                | (
                    Queued,
                    Running | WaitingMetadata | WaitingRelease | Suspended | Satisfied | Cancelled
                )
                | (
                    RetryWait,
                    Queued | Running | Suspended | Satisfied | Cancelled
                )
                | (
                    Running,
                    Queued | RetryWait | Suspended | Satisfied | Completed
                )
                | (
                    Suspended,
                    Queued | WaitingMetadata | WaitingRelease | Satisfied | Cancelled
                )
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueJob {
    pub id: String,
    pub media_type: MediaType,
    pub media_key: String,
    pub title: String,
    pub series_key: Option<String>,
    pub episode_sort_key: Option<String>,
    pub external_manager_id: Option<i64>,
    pub state: QueueJobState,
    pub priority: i32,
    pub eligible_at: Option<i64>,
    pub next_attempt_at: Option<i64>,
    pub no_result_attempts: u32,
    pub first_requested_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRun {
    pub id: String,
    pub job_id: String,
    pub command_id: Option<String>,
    pub state: SearchRunState,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchRunState {
    Submitting,
    Active,
    Reconciling,
    Completed,
    NoResults,
    TechnicalError,
    Satisfied,
}

impl SearchRunState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Submitting | Self::Active | Self::Reconciling)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submitting => "submitting",
            Self::Active => "active",
            Self::Reconciling => "reconciling",
            Self::Completed => "completed",
            Self::NoResults => "no_results",
            Self::TechnicalError => "technical_error",
            Self::Satisfied => "satisfied",
        }
    }
}
