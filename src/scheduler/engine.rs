use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    core::{MediaType, QueueJob, SearchRun, SearchRunState},
    database::{queue::QueueJobRow, Database, DatabaseError},
};

use super::{no_result_delay_seconds, SchedulerHistory, SchedulerPolicy};

#[derive(Debug, Clone, Copy, Default)]
pub struct ServiceAvailability {
    pub radarr: bool,
    pub sonarr: bool,
    pub prowlarr: bool,
}

#[derive(Debug, Clone)]
pub struct ClaimedSearch {
    pub job: QueueJob,
    pub run_id: String,
}

#[derive(Debug, Clone, Copy)]
pub enum SearchOutcome {
    Found,
    NoResults,
    AlreadySatisfied,
    TechnicalError,
}

#[derive(Debug, FromRow)]
struct RunRow {
    id: String,
    job_id: String,
    command_id: Option<String>,
    state: String,
    started_at: i64,
    completed_at: Option<i64>,
}

impl TryFrom<RunRow> for SearchRun {
    type Error = DatabaseError;

    fn try_from(row: RunRow) -> Result<Self, Self::Error> {
        let state = match row.state.as_str() {
            "submitting" => SearchRunState::Submitting,
            "active" => SearchRunState::Active,
            "reconciling" => SearchRunState::Reconciling,
            "completed" => SearchRunState::Completed,
            "no_results" => SearchRunState::NoResults,
            "technical_error" => SearchRunState::TechnicalError,
            "satisfied" => SearchRunState::Satisfied,
            other => {
                return Err(DatabaseError::InvalidData(format!(
                    "unknown search state {other}"
                )))
            }
        };
        Ok(Self {
            id: row.id,
            job_id: row.job_id,
            command_id: row.command_id,
            state,
            started_at: row.started_at,
            completed_at: row.completed_at,
        })
    }
}

impl Database {
    /// Atomically reserve one eligible job. An active or ambiguous external
    /// operation prevents another GXE search from starting.
    pub async fn claim_next_search(
        &self,
        now: i64,
        availability: ServiceAvailability,
        policy: &SchedulerPolicy,
    ) -> Result<Option<ClaimedSearch>, DatabaseError> {
        let mut tx = self.pool().begin().await?;
        let active: (i64,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM search_runs WHERE state IN ('submitting', 'active', 'reconciling'))",
        ).fetch_one(&mut *tx).await?;
        if active.0 != 0 {
            tx.rollback().await?;
            return Ok(None);
        }

        let global_paused: Option<(String,)> =
            sqlx::query_as("SELECT value FROM settings WHERE key='global_paused'")
                .fetch_optional(&mut *tx)
                .await?;
        if global_paused.is_some_and(|(value,)| value.eq_ignore_ascii_case("true")) {
            tx.rollback().await?;
            return Ok(None);
        }
        // Until indexer-specific blocking has been validated through supported
        // Prowlarr behavior, a known cooldown conservatively pauses all searches.
        let active_cooldown: (i64,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM indexer_cooldowns WHERE cooldown_until > ?)",
        )
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;
        if active_cooldown.0 != 0 {
            tx.rollback().await?;
            return Ok(None);
        }

        let interval: Option<(String,)> =
            sqlx::query_as("SELECT value FROM settings WHERE key='minimum_interval_seconds'")
                .fetch_optional(&mut *tx)
                .await?;
        let interval_seconds = interval
            .and_then(|(value,)| value.parse::<i64>().ok())
            .unwrap_or(policy.minimum_interval_seconds);
        let last_start: Option<(i64,)> =
            sqlx::query_as("SELECT started_at FROM search_runs ORDER BY started_at DESC LIMIT 1")
                .fetch_optional(&mut *tx)
                .await?;
        if last_start.is_some_and(|(started,)| now < started.saturating_add(interval_seconds)) {
            tx.rollback().await?;
            return Ok(None);
        }

        let rows = sqlx::query_as::<_, QueueJobRow>(
            "SELECT id, media_type, media_key, title, series_key, episode_sort_key, external_manager_id, state, priority, eligible_at, next_attempt_at, no_result_attempts, first_requested_at \
             FROM queue_jobs WHERE state IN ('queued', 'retry_wait') \
             AND (eligible_at IS NULL OR eligible_at <= ?) \
             AND (next_attempt_at IS NULL OR next_attempt_at <= ?) \
             ORDER BY priority DESC, first_requested_at ASC",
        ).bind(now).bind(now).fetch_all(&mut *tx).await?;
        let jobs: Vec<QueueJob> = rows
            .into_iter()
            .map(QueueJob::try_from)
            .collect::<Result<_, _>>()?;
        let mut jobs: Vec<QueueJob> = jobs
            .into_iter()
            .filter(|job| match job.media_type {
                MediaType::Movie => availability.radarr,
                MediaType::Episode => availability.sonarr,
            })
            .collect();
        if !availability.prowlarr {
            jobs.clear();
        }

        let prior_rows = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT queue_jobs.media_type, queue_jobs.series_key \
             FROM search_runs JOIN queue_jobs ON queue_jobs.id=search_runs.job_id \
             ORDER BY search_runs.started_at DESC LIMIT 100",
        )
        .fetch_all(&mut *tx)
        .await?;
        let previous_type = prior_rows
            .first()
            .and_then(|(kind, _)| match kind.as_str() {
                "movie" => Some(MediaType::Movie),
                "episode" => Some(MediaType::Episode),
                _ => None,
            });
        let consecutive_same_type = prior_rows
            .iter()
            .take_while(|(kind, _)| previous_type.is_some_and(|t| kind == t.as_str()))
            .count() as u32;
        let history = SchedulerHistory {
            previous_start_at: last_start.map(|(started,)| started),
            previous_media_type: previous_type,
            consecutive_same_type,
            previous_series_key: prior_rows.first().and_then(|(_, key)| key.clone()),
            active_search: false,
        };
        let Some(job) = policy.choose(&jobs, &history).cloned() else {
            tx.rollback().await?;
            return Ok(None);
        };

        let run_id = Uuid::new_v4().to_string();
        let update = sqlx::query("UPDATE queue_jobs SET state='running', updated_at=? WHERE id=? AND state IN ('queued', 'retry_wait')")
            .bind(now).bind(&job.id).execute(&mut *tx).await?;
        if update.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(None);
        }
        sqlx::query("INSERT INTO search_runs (id, job_id, state, started_at) VALUES (?, ?, 'submitting', ?)")
            .bind(&run_id).bind(&job.id).bind(now).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some(ClaimedSearch { job, run_id }))
    }

    /// Store the command identifier accepted by Radarr or Sonarr.
    pub async fn mark_search_accepted(
        &self,
        run_id: &str,
        command_id: &str,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            "UPDATE search_runs SET state='active', command_id=? WHERE id=? AND state='submitting'",
        )
        .bind(command_id)
        .bind(run_id)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected() == 1)
    }

    /// Keep ambiguous operations active until reconciliation proves their state.
    pub async fn mark_search_ambiguous(&self, run_id: &str) -> Result<bool, DatabaseError> {
        let result = sqlx::query("UPDATE search_runs SET state='reconciling' WHERE id=? AND state IN ('submitting', 'active')")
            .bind(run_id).execute(self.pool()).await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn active_searches(&self) -> Result<Vec<SearchRun>, DatabaseError> {
        let rows = sqlx::query_as::<_, RunRow>(
            "SELECT id, job_id, command_id, state, started_at, completed_at FROM search_runs WHERE state IN ('submitting', 'active', 'reconciling')",
        ).fetch_all(self.pool()).await?;
        rows.into_iter().map(SearchRun::try_from).collect()
    }

    pub async fn finish_search(
        &self,
        run_id: &str,
        outcome: SearchOutcome,
        now: i64,
        detail: Option<&str>,
    ) -> Result<bool, DatabaseError> {
        let mut tx = self.pool().begin().await?;
        let row: Option<(String, String, i64)> = sqlx::query_as(
            "SELECT search_runs.job_id, queue_jobs.state, queue_jobs.no_result_attempts \
             FROM search_runs JOIN queue_jobs ON queue_jobs.id=search_runs.job_id \
             WHERE search_runs.id=? AND search_runs.state IN ('submitting', 'active', 'reconciling')",
        ).bind(run_id).fetch_optional(&mut *tx).await?;
        let Some((job_id, job_state, attempts)) = row else {
            tx.rollback().await?;
            return Ok(false);
        };
        if job_state != "running" {
            return Err(DatabaseError::InvalidData(format!(
                "active run {run_id} has job state {job_state}"
            )));
        }

        let (run_state, job_state, next_attempt, increment_attempts) = match outcome {
            SearchOutcome::Found => ("completed", "completed", None, false),
            SearchOutcome::NoResults => {
                let attempts = attempts.saturating_add(1) as u32;
                (
                    "no_results",
                    "retry_wait",
                    Some(now + no_result_delay_seconds(attempts)),
                    true,
                )
            }
            SearchOutcome::AlreadySatisfied => ("satisfied", "satisfied", None, false),
            // Technical retry timings are not approved; suspend for review and
            // leave the content no-result counter unchanged.
            SearchOutcome::TechnicalError => ("technical_error", "suspended", None, false),
        };
        sqlx::query("UPDATE search_runs SET state=?, completed_at=?, outcome_detail=? WHERE id=?")
            .bind(run_state)
            .bind(now)
            .bind(detail)
            .bind(run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE queue_jobs SET state=?, next_attempt_at=?, no_result_attempts=no_result_attempts + ?, updated_at=? WHERE id=?")
            .bind(job_state).bind(next_attempt).bind(if increment_attempts { 1_i64 } else { 0_i64 }).bind(now).bind(job_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(true)
    }
}
