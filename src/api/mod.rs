use std::{net::SocketAddr, path::PathBuf};

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use subtle::ConstantTimeEq;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    core::SearchRun,
    database::{Database, DatabaseError},
    integrations::{IndexerOverview, Integrations},
    scheduler::SearchOutcome,
};

#[derive(Clone)]
struct ApiState {
    database: Database,
    integrations: Integrations,
}

#[derive(Debug)]
struct ApiError(DatabaseError);

impl From<DatabaseError> for ApiError {
    fn from(value: DatabaseError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            DatabaseError::InvalidData(_)
            | DatabaseError::RequestNotApproved
            | DatabaseError::MediaNotAvailable => StatusCode::BAD_REQUEST,
            DatabaseError::QueueCapacityReached => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({"error": self.0.to_string()}))).into_response()
    }
}

pub fn router(
    database: Database,
    admin_token: String,
    web_dist: PathBuf,
    integrations: Integrations,
) -> Router {
    let state = ApiState {
        database,
        integrations,
    };
    let protected = Router::new()
        .route("/status", get(status))
        .route("/queue", get(queue))
        .route("/history", get(history))
        .route("/events", get(events))
        .route("/events/:id/ack", post(acknowledge_event))
        .route("/cooldowns", get(cooldowns))
        .route("/indexers", get(indexers))
        .route("/queue/:id/details", get(queue_job_details))
        .route("/settings", get(settings).put(update_settings))
        .route("/queue/pause", post(set_pause))
        .route("/queue/empty", post(empty_queue))
        .route("/queue/:id/priority", post(prioritize))
        .route("/queue/:id/retry", post(force_retry))
        .route("/queue/:id", delete(remove_job))
        .route("/runs/:id/reconcile", post(reconcile_run))
        .route_layer(middleware::from_fn_with_state(
            admin_token,
            require_admin_token,
        ))
        .with_state(state);

    let index = web_dist.join("index.html");
    Router::new()
        .route(
            "/api/health",
            get(|| async { Json(json!({"status": "ok"})) }),
        )
        .nest("/api/v1", protected)
        .fallback_service(ServeDir::new(web_dist).fallback(ServeFile::new(index)))
        .layer(TraceLayer::new_for_http())
}

pub fn parse_bind_address(
    address: &str,
    port: u16,
) -> Result<SocketAddr, std::net::AddrParseError> {
    format!("{address}:{port}").parse()
}

async fn require_admin_token(
    State(expected): State<String>,
    request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    let supplied = token.as_bytes();
    let expected_bytes = expected.as_bytes();
    let same_length = (supplied.len() == expected_bytes.len()) as u8;
    let same_value = if same_length == 1 {
        expected_bytes.ct_eq(supplied).unwrap_u8()
    } else {
        0
    };
    if same_length & same_value == 1 {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "unauthorized"})),
        )
            .into_response()
    }
}

async fn status(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let counts = state.database.queue_counts().await?;
    let active: Option<(String, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT search_runs.id, search_runs.job_id, search_runs.state, search_runs.started_at, \
                queue_jobs.title, queue_jobs.media_type \
         FROM search_runs JOIN queue_jobs ON queue_jobs.id=search_runs.job_id \
         WHERE search_runs.state IN ('submitting','active','reconciling') \
         ORDER BY search_runs.started_at LIMIT 1",
    )
    .fetch_optional(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    let paused = state
        .database
        .setting_value("global_paused")
        .await?
        .as_deref()
        == Some("true");
    let health: Vec<(String, i64, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT service, available, checked_at, detail FROM service_health ORDER BY service",
    )
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    let cooldowns: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM indexer_cooldowns WHERE cooldown_until > unixepoch()")
            .fetch_one(state.database.pool())
            .await
            .map_err(DatabaseError::from)?;
    let period_start = unix_now().saturating_sub(90 * 24 * 60 * 60);
    let outcomes: (i64, i64, i64) = sqlx::query_as(
        "SELECT \
            COALESCE(SUM(CASE WHEN state='completed' THEN 1 ELSE 0 END), 0), \
            COALESCE(SUM(CASE WHEN state='no_results' THEN 1 ELSE 0 END), 0), \
            COALESCE(SUM(CASE WHEN state='technical_error' THEN 1 ELSE 0 END), 0) \
         FROM search_runs WHERE completed_at >= ?",
    )
    .bind(period_start)
    .fetch_one(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    Ok(Json(json!({
        "paused": paused,
        "counts": counts,
        "outcomes_90d": {
            "completed": outcomes.0,
            "no_results": outcomes.1,
            "technical_errors": outcomes.2,
        },
        "active_search": active.map(|(id, job_id, state, started_at, title, media_type)| json!({
            "id": id,
            "job_id": job_id,
            "state": state,
            "started_at": started_at,
            "title": title,
            "media_type": media_type,
        })),
        "services": health.into_iter().map(|(service, available, checked_at, detail)| json!({
            "service": service,
            "available": available != 0,
            "checked_at": checked_at,
            "detail": detail,
        })).collect::<Vec<_>>(),
        "active_cooldowns": cooldowns.0,
    })))
}

async fn queue(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let jobs = state.database.list_queue_jobs().await?;
    Ok(Json(json!({"items": jobs})))
}

async fn indexers(
    State(state): State<ApiState>,
) -> Result<Json<IndexerOverview>, (StatusCode, Json<Value>)> {
    match state.integrations.indexer_overview().await {
        Ok(overview) => Ok(Json(overview)),
        Err(error) => {
            tracing::warn!(%error, "Could not read Prowlarr indexer overview");
            Err((
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": "Prowlarr indexer data is temporarily unavailable"})),
            ))
        }
    }
}

async fn queue_job_details(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Some(job) = state.database.queue_job(&id).await? else {
        return Err(ApiError(DatabaseError::InvalidData(
            "queue item was not found".into(),
        )));
    };
    let requests: Vec<(String, String, String, i64)> = sqlx::query_as(
        "SELECT requests.external_id, requests.requester_name, requests.media_type, requests.created_at \
         FROM request_jobs JOIN requests ON requests.id=request_jobs.request_id \
         WHERE request_jobs.job_id=? AND requests.active=1 ORDER BY requests.created_at",
    )
    .bind(&id)
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    let runs: Vec<JobRunRecord> = sqlx::query_as(
        "SELECT id, state, started_at, completed_at, outcome_detail \
         FROM search_runs WHERE job_id=? ORDER BY started_at DESC LIMIT 50",
    )
    .bind(&id)
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    Ok(Json(json!({
        "job": job,
        "requests": requests.into_iter().map(|(external_id, requester_name, media_type, created_at)| json!({
            "external_id": external_id,
            "requester_name": requester_name,
            "media_type": media_type,
            "created_at": created_at,
        })).collect::<Vec<_>>(),
        "runs": runs.into_iter().map(|run| json!({
            "id": run.id,
            "state": run.state,
            "started_at": run.started_at,
            "completed_at": run.completed_at,
            "detail": run.outcome_detail,
        })).collect::<Vec<_>>(),
    })))
}

#[derive(sqlx::FromRow)]
struct JobRunRecord {
    id: String,
    state: String,
    started_at: i64,
    completed_at: Option<i64>,
    outcome_detail: Option<String>,
}

#[derive(sqlx::FromRow)]
struct HistoryRecord {
    id: String,
    job_id: String,
    state: String,
    started_at: i64,
    completed_at: Option<i64>,
    outcome_detail: Option<String>,
    title: Option<String>,
    media_type: Option<String>,
}

async fn history(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let rows: Vec<HistoryRecord> = sqlx::query_as(
        "SELECT search_runs.id, search_runs.job_id, search_runs.state, search_runs.started_at, \
                search_runs.completed_at, search_runs.outcome_detail, queue_jobs.title, queue_jobs.media_type \
         FROM search_runs LEFT JOIN queue_jobs ON queue_jobs.id=search_runs.job_id \
         ORDER BY search_runs.started_at DESC LIMIT 500",
    )
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    Ok(Json(json!({"items": rows.into_iter().map(|run| json!({
        "id": run.id,
        "job_id": run.job_id,
        "state": run.state,
        "started_at": run.started_at,
        "completed_at": run.completed_at,
        "detail": run.outcome_detail,
        "title": run.title,
        "media_type": run.media_type,
    })).collect::<Vec<_>>()})))
}

async fn events(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(String, String, String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, kind, severity, message, created_at, acknowledged_at \
         FROM dashboard_events ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    Ok(Json(
        json!({"items": rows.into_iter().map(|(id, kind, severity, message, created_at, acknowledged_at)| json!({
        "id": id,
        "kind": kind,
        "severity": severity,
        "message": message,
        "created_at": created_at,
        "acknowledged_at": acknowledged_at,
    })).collect::<Vec<_>>()}),
    ))
}

async fn acknowledge_event(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE dashboard_events SET acknowledged_at=? WHERE id=? AND acknowledged_at IS NULL",
    )
    .bind(unix_now())
    .bind(id)
    .execute(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    Ok(Json(json!({"acknowledged": result.rows_affected() == 1})))
}

async fn cooldowns(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(i64, String, i64, String)> = sqlx::query_as(
        "SELECT external_id, name, cooldown_until, reason FROM indexer_cooldowns WHERE cooldown_until > unixepoch() ORDER BY cooldown_until",
    )
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    Ok(Json(
        json!({"items": rows.into_iter().map(|(id, name, cooldown_until, reason)| json!({
        "id": id,
        "name": name,
        "cooldown_until": cooldown_until,
        "reason": reason,
    })).collect::<Vec<_>>()}),
    ))
}

async fn settings(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM settings ORDER BY key")
            .fetch_all(state.database.pool())
            .await
            .map_err(DatabaseError::from)?;
    let mut values = json!({
        "minimum_interval_seconds": 600,
        "queue_capacity": 0,
        "search_timeout_seconds": 900,
        "movie_theatrical_delay_days": 0,
    });
    for (key, value) in rows {
        if let Some(object) = values.as_object_mut() {
            object.insert(key, Value::String(value));
        }
    }
    Ok(Json(values))
}

#[derive(Deserialize)]
struct SettingsInput {
    minimum_interval_seconds: i64,
    queue_capacity: i64,
    search_timeout_seconds: i64,
    movie_theatrical_delay_days: i64,
}

async fn update_settings(
    State(state): State<ApiState>,
    Json(input): Json<SettingsInput>,
) -> Result<Json<Value>, ApiError> {
    if input.minimum_interval_seconds < 600
        || input.queue_capacity < 0
        || input.search_timeout_seconds <= 0
        || input.movie_theatrical_delay_days < 0
    {
        return Err(ApiError(DatabaseError::InvalidData(
            "settings are outside the supported range".into(),
        )));
    }
    let now = unix_now();
    let mut transaction = state
        .database
        .pool()
        .begin()
        .await
        .map_err(DatabaseError::from)?;
    for (key, value) in [
        ("minimum_interval_seconds", input.minimum_interval_seconds),
        ("queue_capacity", input.queue_capacity),
        ("search_timeout_seconds", input.search_timeout_seconds),
        (
            "movie_theatrical_delay_days",
            input.movie_theatrical_delay_days,
        ),
    ] {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
        )
        .bind(key)
        .bind(value.to_string())
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(DatabaseError::from)?;
    }
    transaction.commit().await.map_err(DatabaseError::from)?;
    Ok(Json(json!({"saved": true})))
}

#[derive(Deserialize)]
struct PauseInput {
    paused: bool,
}

async fn set_pause(
    State(state): State<ApiState>,
    Json(input): Json<PauseInput>,
) -> Result<Json<Value>, ApiError> {
    state
        .database
        .set_global_paused(input.paused, unix_now())
        .await?;
    Ok(Json(json!({"paused": input.paused})))
}

async fn empty_queue(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let removed = state.database.empty_unstarted_queue(unix_now()).await?;
    Ok(Json(json!({"dismissed": removed})))
}

async fn prioritize(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    if state.database.prioritize_job(&id, unix_now()).await? {
        Ok(Json(json!({"updated": true})))
    } else {
        Err(ApiError(DatabaseError::InvalidData(
            "job is missing or already started".into(),
        )))
    }
}

async fn remove_job(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    if state.database.remove_unstarted_job(&id, unix_now()).await? {
        Ok(Json(json!({"removed": true})))
    } else {
        Err(ApiError(DatabaseError::InvalidData(
            "job is missing or already started".into(),
        )))
    }
}

async fn force_retry(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    if state.database.force_retry(&id, unix_now()).await? {
        Ok(Json(json!({"queued": true})))
    } else {
        Err(ApiError(DatabaseError::InvalidData(
            "job is missing or cannot be retried".into(),
        )))
    }
}

#[derive(Deserialize)]
struct ReconcileInput {
    outcome: String,
    detail: Option<String>,
}

async fn reconcile_run(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(input): Json<ReconcileInput>,
) -> Result<Json<Value>, ApiError> {
    let outcome = match input.outcome.as_str() {
        "found" => SearchOutcome::Found,
        "no_results" => SearchOutcome::NoResults,
        "satisfied" => SearchOutcome::AlreadySatisfied,
        "technical_error" => SearchOutcome::TechnicalError,
        _ => {
            return Err(ApiError(DatabaseError::InvalidData(
                "unsupported reconciliation outcome".into(),
            )))
        }
    };
    if state
        .database
        .finish_search(&id, outcome, unix_now(), input.detail.as_deref())
        .await?
    {
        Ok(Json(json!({"reconciled": true})))
    } else {
        Err(ApiError(DatabaseError::InvalidData(
            "no active run with that id".into(),
        )))
    }
}

pub fn search_run_is_active(run: &SearchRun) -> bool {
    run.state.is_active()
}

pub fn unix_now() -> i64 {
    chrono::Utc::now().timestamp()
}
