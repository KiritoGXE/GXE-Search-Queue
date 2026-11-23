use sqlx::FromRow;
use uuid::Uuid;

use crate::core::{MediaType, QueueJob, QueueJobState};

use super::{Database, DatabaseError};

#[derive(Debug, Clone)]
pub struct SeerrRequestInput {
    pub external_id: String,
    pub requester_id: String,
    pub requester_name: String,
    pub media_type: &'static str,
    pub media_key: String,
    pub approved: bool,
    pub now: i64,
}

#[derive(Debug, Clone)]
pub struct NewQueueJob {
    pub media_type: MediaType,
    pub media_key: String,
    pub title: String,
    pub series_key: Option<String>,
    pub episode_sort_key: Option<String>,
    pub external_manager_id: Option<i64>,
    pub eligibility: JobEligibility,
    pub first_requested_at: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum JobEligibility {
    Available,
    FutureRelease { eligible_at: i64 },
    UnknownReleaseMetadata,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct QueueJobRow {
    pub(crate) id: String,
    pub(crate) media_type: String,
    pub(crate) media_key: String,
    pub(crate) title: String,
    pub(crate) series_key: Option<String>,
    pub(crate) episode_sort_key: Option<String>,
    pub(crate) external_manager_id: Option<i64>,
    pub(crate) state: String,
    pub(crate) priority: i32,
    pub(crate) eligible_at: Option<i64>,
    pub(crate) next_attempt_at: Option<i64>,
    pub(crate) no_result_attempts: i64,
    pub(crate) first_requested_at: i64,
}

impl TryFrom<QueueJobRow> for QueueJob {
    type Error = DatabaseError;

    fn try_from(row: QueueJobRow) -> Result<Self, Self::Error> {
        let media_type = match row.media_type.as_str() {
            "movie" => MediaType::Movie,
            "episode" => MediaType::Episode,
            other => {
                return Err(DatabaseError::InvalidData(format!(
                    "unknown media type {other}"
                )))
            }
        };
        let state = match row.state.as_str() {
            "waiting_metadata" => QueueJobState::WaitingMetadata,
            "waiting_release" => QueueJobState::WaitingRelease,
            "queued" => QueueJobState::Queued,
            "retry_wait" => QueueJobState::RetryWait,
            "running" => QueueJobState::Running,
            "suspended" => QueueJobState::Suspended,
            "satisfied" => QueueJobState::Satisfied,
            "completed" => QueueJobState::Completed,
            "cancelled" => QueueJobState::Cancelled,
            "dismissed" => QueueJobState::Dismissed,
            other => {
                return Err(DatabaseError::InvalidData(format!(
                    "unknown queue state {other}"
                )))
            }
        };
        Ok(Self {
            id: row.id,
            media_type,
            media_key: row.media_key,
            title: row.title,
            series_key: row.series_key,
            episode_sort_key: row.episode_sort_key,
            external_manager_id: row.external_manager_id,
            state,
            priority: row.priority,
            eligible_at: row.eligible_at,
            next_attempt_at: row.next_attempt_at,
            no_result_attempts: row.no_result_attempts.max(0) as u32,
            first_requested_at: row.first_requested_at,
        })
    }
}

impl Database {
    /// Persist a Seerr request idempotently. Request synchronization must call
    /// this only for approved requests; direct Radarr/Sonarr additions are not
    /// queue sources.
    pub async fn record_seerr_request(
        &self,
        request: &SeerrRequestInput,
    ) -> Result<String, DatabaseError> {
        if !request.approved {
            return Err(DatabaseError::RequestNotApproved);
        }
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO requests (id, source, external_id, requester_id, requester_name, media_type, media_key, active, created_at, updated_at) \
             VALUES (?, 'seerr', ?, ?, ?, ?, ?, 1, ?, ?) \
             ON CONFLICT(source, external_id) DO UPDATE SET requester_id=excluded.requester_id, requester_name=excluded.requester_name, media_type=excluded.media_type, media_key=excluded.media_key, active=1, updated_at=excluded.updated_at",
        )
        .bind(&id)
        .bind(&request.external_id)
        .bind(&request.requester_id)
        .bind(&request.requester_name)
        .bind(request.media_type)
        .bind(&request.media_key)
        .bind(request.now)
        .bind(request.now)
        .execute(self.pool())
        .await?;

        let row: (String,) =
            sqlx::query_as("SELECT id FROM requests WHERE source='seerr' AND external_id=?")
                .bind(&request.external_id)
                .fetch_one(self.pool())
                .await?;
        Ok(row.0)
    }

    /// Insert one deduplicated movie/episode work item and link it to a request.
    /// Existing jobs retain their original queue age and priority.
    pub async fn enqueue_for_request(
        &self,
        request_id: &str,
        job: &NewQueueJob,
        now: i64,
    ) -> Result<String, DatabaseError> {
        let mut tx = self.pool().begin().await?;
        let capacity: Option<(String,)> =
            sqlx::query_as("SELECT value FROM settings WHERE key='queue_capacity'")
                .fetch_optional(&mut *tx)
                .await?;
        let capacity = capacity
            .and_then(|(value,)| value.parse::<i64>().ok())
            .unwrap_or(0);
        if capacity > 0 {
            let (active_jobs,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM queue_jobs WHERE state IN ('waiting_metadata','waiting_release','queued','retry_wait','running','suspended')",
            ).fetch_one(&mut *tx).await?;
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM queue_jobs WHERE media_type=? AND media_key=?")
                    .bind(job.media_type.as_str())
                    .bind(&job.media_key)
                    .fetch_optional(&mut *tx)
                    .await?;
            if active_jobs >= capacity && existing.is_none() {
                tx.rollback().await?;
                return Err(DatabaseError::QueueCapacityReached);
            }
        }
        let id = Uuid::new_v4().to_string();
        let request_type: Option<(String,)> =
            sqlx::query_as("SELECT media_type FROM requests WHERE id=?")
                .bind(request_id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((request_type,)) = request_type else {
            tx.rollback().await?;
            return Err(DatabaseError::InvalidData(
                "request must exist before work is linked".into(),
            ));
        };
        let expected_request_type = match job.media_type {
            MediaType::Movie => "movie",
            MediaType::Episode => "series",
        };
        if request_type != expected_request_type {
            tx.rollback().await?;
            return Err(DatabaseError::InvalidData(
                "request and queue job media types do not match".into(),
            ));
        }

        let (initial_state, eligible_at) = match (job.media_type, job.eligibility) {
            (MediaType::Movie, JobEligibility::Available) => (QueueJobState::Queued, None),
            (MediaType::Movie, JobEligibility::FutureRelease { eligible_at })
                if eligible_at > now =>
            {
                (QueueJobState::WaitingRelease, Some(eligible_at))
            }
            (MediaType::Movie, JobEligibility::FutureRelease { .. }) => {
                (QueueJobState::Queued, None)
            }
            (MediaType::Movie, JobEligibility::UnknownReleaseMetadata) => {
                (QueueJobState::WaitingMetadata, None)
            }
            (MediaType::Episode, JobEligibility::Available) => (QueueJobState::Queued, None),
            (MediaType::Episode, _) => {
                tx.rollback().await?;
                return Err(DatabaseError::MediaNotAvailable);
            }
        };
        sqlx::query(
            "INSERT INTO queue_jobs (id, media_type, media_key, title, series_key, episode_sort_key, external_manager_id, state, eligible_at, first_requested_at, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(media_type, media_key) DO UPDATE SET \
             title=CASE WHEN queue_jobs.state='cancelled' THEN excluded.title ELSE queue_jobs.title END, \
             series_key=CASE WHEN queue_jobs.state='cancelled' THEN excluded.series_key ELSE queue_jobs.series_key END, \
             episode_sort_key=CASE WHEN queue_jobs.state='cancelled' THEN excluded.episode_sort_key ELSE queue_jobs.episode_sort_key END, \
             external_manager_id=CASE WHEN queue_jobs.state='cancelled' THEN excluded.external_manager_id ELSE queue_jobs.external_manager_id END, \
             state=CASE WHEN queue_jobs.state IN ('cancelled','waiting_metadata','waiting_release') THEN excluded.state ELSE queue_jobs.state END, \
             eligible_at=CASE WHEN queue_jobs.state IN ('cancelled','waiting_metadata','waiting_release') THEN excluded.eligible_at ELSE queue_jobs.eligible_at END, \
             next_attempt_at=CASE WHEN queue_jobs.state='cancelled' THEN NULL ELSE queue_jobs.next_attempt_at END, \
             no_result_attempts=CASE WHEN queue_jobs.state='cancelled' THEN 0 ELSE queue_jobs.no_result_attempts END, \
             priority=CASE WHEN queue_jobs.state='cancelled' THEN 0 ELSE queue_jobs.priority END, \
             first_requested_at=CASE WHEN queue_jobs.state='cancelled' THEN excluded.first_requested_at ELSE queue_jobs.first_requested_at END, \
             updated_at=CASE WHEN queue_jobs.state='cancelled' THEN excluded.updated_at ELSE queue_jobs.updated_at END",
        )
        .bind(&id)
        .bind(job.media_type.as_str())
        .bind(&job.media_key)
        .bind(&job.title)
        .bind(&job.series_key)
        .bind(&job.episode_sort_key)
        .bind(job.external_manager_id)
        .bind(initial_state.as_str())
        .bind(eligible_at)
        .bind(job.first_requested_at)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let (job_id,): (String,) =
            sqlx::query_as("SELECT id FROM queue_jobs WHERE media_type=? AND media_key=?")
                .bind(job.media_type.as_str())
                .bind(&job.media_key)
                .fetch_one(&mut *tx)
                .await?;
        let link = sqlx::query("INSERT INTO request_jobs (request_id, job_id, created_at) VALUES (?, ?, ?) ON CONFLICT DO NOTHING")
            .bind(request_id)
            .bind(&job_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        if link.rows_affected() == 1 {
            sqlx::query("UPDATE queue_jobs SET state=?, eligible_at=?, next_attempt_at=NULL, no_result_attempts=0, updated_at=? WHERE id=? AND state IN ('cancelled','dismissed')")
                .bind(initial_state.as_str()).bind(eligible_at).bind(now).bind(&job_id).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(job_id)
    }

    /// Cancellation only removes shared work when no active Seerr request
    /// still references it. A running external operation is never interrupted.
    pub async fn set_request_active(
        &self,
        request_id: &str,
        active: bool,
        now: i64,
    ) -> Result<(), DatabaseError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("UPDATE requests SET active=?, updated_at=? WHERE id=?")
            .bind(if active { 1_i64 } else { 0_i64 })
            .bind(now)
            .bind(request_id)
            .execute(&mut *tx)
            .await?;
        if !active {
            sqlx::query(
                "UPDATE queue_jobs SET state='cancelled', updated_at=? \
                 WHERE id IN (SELECT request_jobs.job_id FROM request_jobs WHERE request_jobs.request_id=?) \
                 AND state IN ('waiting_metadata','waiting_release','queued','retry_wait','suspended') \
                 AND NOT EXISTS (SELECT 1 FROM request_jobs rj JOIN requests r ON r.id=rj.request_id WHERE rj.job_id=queue_jobs.id AND r.active=1)",
            ).bind(now).bind(request_id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM request_jobs WHERE request_id=?")
                .bind(request_id).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn ready_jobs(&self, now: i64) -> Result<Vec<QueueJob>, DatabaseError> {
        let rows = sqlx::query_as::<_, QueueJobRow>(
            "SELECT id, media_type, media_key, title, series_key, episode_sort_key, external_manager_id, state, priority, eligible_at, next_attempt_at, no_result_attempts, first_requested_at \
             FROM queue_jobs WHERE state IN ('queued', 'retry_wait') \
             AND (eligible_at IS NULL OR eligible_at <= ?) \
             AND (next_attempt_at IS NULL OR next_attempt_at <= ?) \
             ORDER BY priority DESC, first_requested_at ASC",
        )
        .bind(now)
        .bind(now)
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(QueueJob::try_from).collect()
    }

    /// Reconcile queued work that Radarr/Sonarr has already grabbed, imported,
    /// or otherwise satisfied. Running operations remain owned by the worker.
    pub async fn reconcile_external_media(
        &self,
        media_type: MediaType,
        media_key: &str,
        satisfied_state: QueueJobState,
        now: i64,
    ) -> Result<bool, DatabaseError> {
        if !matches!(satisfied_state, QueueJobState::Completed | QueueJobState::Satisfied) {
            return Err(DatabaseError::InvalidData("external reconciliation requires a satisfied terminal state".into()));
        }
        let result = sqlx::query(
            "UPDATE queue_jobs SET state=?, next_attempt_at=NULL, updated_at=? \
             WHERE media_type=? AND media_key=? \
             AND state IN ('waiting_metadata','waiting_release','queued','retry_wait','suspended')",
        )
        .bind(satisfied_state.as_str())
        .bind(now)
        .bind(media_type.as_str())
        .bind(media_key)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn list_queue_jobs(&self) -> Result<Vec<QueueJob>, DatabaseError> {
        let rows = sqlx::query_as::<_, QueueJobRow>(
            "SELECT id, media_type, media_key, title, series_key, episode_sort_key, external_manager_id, state, priority, eligible_at, next_attempt_at, no_result_attempts, first_requested_at \
             FROM queue_jobs WHERE state NOT IN ('completed', 'satisfied', 'cancelled', 'dismissed') \
             ORDER BY priority DESC, first_requested_at ASC",
        )
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(QueueJob::try_from).collect()
    }

    pub async fn deactivate_seerr_request(&self, external_id: &str, now: i64) -> Result<(), DatabaseError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM requests WHERE source='seerr' AND external_id=? AND active=1",
        )
        .bind(external_id)
        .fetch_optional(self.pool())
        .await?;
        if let Some((id,)) = row {
            self.set_request_active(&id, false, now).await?;
        }
        Ok(())
    }

    pub async fn deactivate_missing_seerr_requests(
        &self,
        active_external_ids: &[String],
        now: i64,
    ) -> Result<(), DatabaseError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, external_id FROM requests WHERE source='seerr' AND active=1",
        )
        .fetch_all(self.pool())
        .await?;
        for (id, external_id) in rows {
            if !active_external_ids.iter().any(|active| active == &external_id) {
                self.set_request_active(&id, false, now).await?;
            }
        }
        Ok(())
    }

    pub async fn setting_value(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key=?")
            .bind(key)
            .fetch_optional(self.pool())
            .await?;
        Ok(row.map(|(value,)| value))
    }

    pub async fn queue_counts(&self) -> Result<serde_json::Value, DatabaseError> {
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT media_type, state, COUNT(*) FROM queue_jobs WHERE state NOT IN ('completed','satisfied','cancelled','dismissed') GROUP BY media_type, state",
        )
        .fetch_all(self.pool())
        .await?;
        let mut counts = serde_json::Map::new();
        counts.insert("movies".into(), serde_json::Value::from(0));
        counts.insert("episodes".into(), serde_json::Value::from(0));
        counts.insert("suspended".into(), serde_json::Value::from(0));
        for (media_type, state, count) in rows {
            let bucket = if media_type == "movie" { "movies" } else { "episodes" };
            let old = counts.get(bucket).and_then(serde_json::Value::as_i64).unwrap_or(0);
            counts.insert(bucket.into(), serde_json::Value::from(old + count));
            if state == "suspended" {
                let old = counts.get("suspended").and_then(serde_json::Value::as_i64).unwrap_or(0);
                counts.insert("suspended".into(), serde_json::Value::from(old + count));
            }
        }
        Ok(serde_json::Value::Object(counts))
    }

    pub async fn transition_job(
        &self,
        job_id: &str,
        expected: &QueueJobState,
        next: &QueueJobState,
        now: i64,
    ) -> Result<bool, DatabaseError> {
        if !expected.can_transition_to(next) {
            return Err(DatabaseError::InvalidTransition {
                from: expected.as_str().to_owned(),
                to: next.as_str().to_owned(),
            });
        }
        let result =
            sqlx::query("UPDATE queue_jobs SET state=?, updated_at=? WHERE id=? AND state=?")
                .bind(next.as_str())
                .bind(now)
                .bind(job_id)
                .bind(expected.as_str())
                .execute(self.pool())
                .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn active_search_exists(&self) -> Result<bool, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM search_runs WHERE state IN ('submitting', 'active', 'reconciling'))",
        )
        .fetch_one(self.pool())
        .await?;
        Ok(row.0 != 0)
    }

    pub async fn set_global_paused(&self, paused: bool, now: i64) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES ('global_paused', ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
        )
        .bind(if paused { "true" } else { "false" })
        .bind(now)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Assign a high queue priority to an unstarted item. The scheduler still
    /// enforces the interval, cooldown, and category fairness rules.
    pub async fn prioritize_job(&self, job_id: &str, now: i64) -> Result<bool, DatabaseError> {
        let mut tx = self.pool().begin().await?;
        let (top,): (i64,) =
            sqlx::query_as("SELECT COALESCE(MAX(priority), 0) + 1 FROM queue_jobs")
                .fetch_one(&mut *tx)
                .await?;
        let result = sqlx::query(
            "UPDATE queue_jobs SET priority=?, updated_at=? WHERE id=? \
             AND state IN ('waiting_metadata','waiting_release','queued','retry_wait','suspended')",
        )
        .bind(top)
        .bind(now)
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn remove_unstarted_job(
        &self,
        job_id: &str,
        now: i64,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            "UPDATE queue_jobs SET state='dismissed', updated_at=? WHERE id=? \
             AND state IN ('waiting_metadata','waiting_release','queued','retry_wait','suspended')",
        )
        .bind(now)
        .bind(job_id)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn empty_unstarted_queue(&self, now: i64) -> Result<u64, DatabaseError> {
        let result = sqlx::query("UPDATE queue_jobs SET state='dismissed', updated_at=? WHERE state IN ('waiting_metadata','waiting_release','queued','retry_wait','suspended')")
            .bind(now).execute(self.pool()).await?;
        Ok(result.rows_affected())
    }

    /// A manual retry clears the delay but never changes the no-result counter.
    /// The search worker must still check whether Radarr/Sonarr needs the media.
    pub async fn force_retry(&self, job_id: &str, now: i64) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            "UPDATE queue_jobs SET state='queued', next_attempt_at=NULL, updated_at=? WHERE id=? \
             AND state IN ('queued','retry_wait','suspended')",
        )
        .bind(now)
        .bind(job_id)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn set_setting(&self, key: &str, value: &str, now: i64) -> Result<(), DatabaseError> {
        let valid = match key {
            "minimum_interval_seconds" => value.parse::<i64>().is_ok_and(|n| n >= 600),
            "queue_capacity" => value.parse::<i64>().is_ok_and(|n| n >= 0),
            "search_timeout_seconds" => value.parse::<i64>().is_ok_and(|n| n > 0),
            "movie_theatrical_delay_days" => value.parse::<i64>().is_ok_and(|n| n >= 0),
            _ => false,
        };
        if !valid {
            return Err(DatabaseError::InvalidData(format!(
                "invalid or unsupported setting {key}"
            )));
        }
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Prune event/search history after 90 days. Queue jobs, requests, and active
    /// external operation state are deliberately retained.
    pub async fn prune_history(&self, now: i64) -> Result<(), DatabaseError> {
        let cutoff = now.saturating_sub(90 * 24 * 60 * 60);
        let mut tx = self.pool().begin().await?;
        sqlx::query("DELETE FROM dashboard_events WHERE created_at < ?")
            .bind(cutoff)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "DELETE FROM search_runs WHERE completed_at IS NOT NULL AND completed_at < ? \
             AND state NOT IN ('submitting','active','reconciling')",
        )
        .bind(cutoff)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
