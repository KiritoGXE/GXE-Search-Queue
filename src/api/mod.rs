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
use tower_http::{services::{ServeDir, ServeFile}, trace::TraceLayer};

use crate::{
    core::SearchRun,
    database::{Database, DatabaseError},
    scheduler::SearchOutcome,
};

#[derive(Clone)]
struct ApiState {
    database: Database,
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
            DatabaseError::InvalidData(_) | DatabaseError::RequestNotApproved | DatabaseError::MediaNotAvailable => StatusCode::BAD_REQUEST,
            DatabaseError::QueueCapacityReached => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({"error": self.0.to_string()}))).into_response()
    }
}

pub fn router(database: Database, admin_token: String, web_dist: PathBuf) -> Router {
    let state = ApiState { database };
    let protected = Router::new()
        .route("/status", get(status))
        .route("/queue", get(queue))
        .route("/history", get(history))
        .route("/cooldowns", get(cooldowns))
        .route("/settings", get(settings).put(update_settings))
        .route("/queue/pause", post(set_pause))
        .route("/queue/empty", post(empty_queue))
        .route("/queue/:id/priority", post(prioritize))
        .route("/queue/:id/retry", post(force_retry))
        .route("/queue/:id", delete(remove_job))
        .route("/runs/:id/reconcile", post(reconcile_run))
        .route_layer(middleware::from_fn_with_state(admin_token, require_admin_token))
        .with_state(state);

    let index = web_dist.join("index.html");
    Router::new()
        .route("/api/health", get(|| async { Json(json!({"status": "ok"})) }))
        .nest("/api/v1", protected)
        .fallback_service(ServeDir::new(web_dist).fallback(ServeFile::new(index)))
        .layer(TraceLayer::new_for_http())
}

pub fn parse_bind_address(address: &str, port: u16) -> Result<SocketAddr, std::net::AddrParseError> {
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
        (StatusCode::UNAUTHORIZED, Json(json!({"error": "unauthorized"}))).into_response()
    }
}

async fn status(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let counts = state.database.queue_counts().await?;
    let active = state.database.active_searches().await?;
    let paused = state.database.setting_value("global_paused").await?.as_deref() == Some("true");
    let health: Vec<(String, i64, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT service, available, checked_at, detail FROM service_health ORDER BY service",
    )
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    let cooldowns: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM indexer_cooldowns WHERE cooldown_until > unixepoch()")
        .fetch_one(state.database.pool())
        .await
        .map_err(DatabaseError::from)?;
    Ok(Json(json!({
        "paused": paused,
        "counts": counts,
        "active_search": active.first(),
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

async fn history(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(String, String, String, i64, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT id, job_id, state, started_at, completed_at, outcome_detail FROM search_runs ORDER BY started_at DESC LIMIT 500",
    )
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    Ok(Json(json!({"items": rows.into_iter().map(|(id, job_id, state, started_at, completed_at, detail)| json!({
        "id": id,
        "job_id": job_id,
        "state": state,
        "started_at": started_at,
        "completed_at": completed_at,
        "detail": detail,
    })).collect::<Vec<_>>()})))
}

async fn cooldowns(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(i64, String, i64, String)> = sqlx::query_as(
        "SELECT external_id, name, cooldown_until, reason FROM indexer_cooldowns WHERE cooldown_until > unixepoch() ORDER BY cooldown_until",
    )
    .fetch_all(state.database.pool())
    .await
    .map_err(DatabaseError::from)?;
    Ok(Json(json!({"items": rows.into_iter().map(|(id, name, cooldown_until, reason)| json!({
        "id": id,
        "name": name,
        "cooldown_until": cooldown_until,
        "reason": reason,
    })).collect::<Vec<_>>()})))
}

async fn settings(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings ORDER BY key")
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
    let now = unix_now();
    state.database.set_setting("minimum_interval_seconds", &input.minimum_interval_seconds.to_string(), now).await?;
    state.database.set_setting("queue_capacity", &input.queue_capacity.to_string(), now).await?;
    state.database.set_setting("search_timeout_seconds", &input.search_timeout_seconds.to_string(), now).await?;
    state.database.set_setting("movie_theatrical_delay_days", &input.movie_theatrical_delay_days.to_string(), now).await?;
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
    state.database.set_global_paused(input.paused, unix_now()).await?;
    Ok(Json(json!({"paused": input.paused})))
}

async fn empty_queue(State(state): State<ApiState>) -> Result<Json<Value>, ApiError> {
    let removed = state.database.empty_unstarted_queue(unix_now()).await?;
    Ok(Json(json!({"dismissed": removed})))
}

async fn prioritize(State(state): State<ApiState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    if state.database.prioritize_job(&id, unix_now()).await? {
        Ok(Json(json!({"updated": true})))
    } else {
        Err(ApiError(DatabaseError::InvalidData("job is missing or already started".into())))
    }
}

async fn remove_job(State(state): State<ApiState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    if state.database.remove_unstarted_job(&id, unix_now()).await? {
        Ok(Json(json!({"removed": true})))
    } else {
        Err(ApiError(DatabaseError::InvalidData("job is missing or already started".into())))
    }
}

async fn force_retry(State(state): State<ApiState>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    if state.database.force_retry(&id, unix_now()).await? {
        Ok(Json(json!({"queued": true})))
    } else {
        Err(ApiError(DatabaseError::InvalidData("job is missing or cannot be retried".into())))
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
        _ => return Err(ApiError(DatabaseError::InvalidData("unsupported reconciliation outcome".into()))),
    };
    if state.database.finish_search(&id, outcome, unix_now(), input.detail.as_deref()).await? {
        Ok(Json(json!({"reconciled": true})))
    } else {
        Err(ApiError(DatabaseError::InvalidData("no active run with that id".into())))
    }
}

pub fn search_run_is_active(run: &SearchRun) -> bool {
    run.state.is_active()
}

pub fn unix_now() -> i64 {
    chrono::Utc::now().timestamp()
}
