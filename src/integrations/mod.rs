use std::{env, time::Duration};

use chrono::{DateTime, NaiveDate, Utc};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    core::{MediaType, QueueJobState},
    database::{Database, DatabaseError, JobEligibility, NewQueueJob, SeerrRequestInput},
    scheduler::{SearchOutcome, ServiceAvailability},
};

#[derive(Clone)]
struct Service {
    base_url: String,
    api_key: String,
}

#[derive(Clone)]
pub struct Integrations {
    http: Client,
    seerr: Option<Service>,
    radarr: Option<Service>,
    sonarr: Option<Service>,
    prowlarr: Option<Service>,
}

#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("service is not configured: {0}")]
    NotConfigured(&'static str),
    #[error("HTTP request to {service} failed: {source}")]
    Http {
        service: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{service} returned HTTP {status}")]
    Status {
        service: &'static str,
        status: StatusCode,
        retry_after: Option<String>,
    },
    #[error("invalid response from {0}: {1}")]
    InvalidResponse(&'static str, String),
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

impl Integrations {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(35))
            .build()?;
        Ok(Self {
            http,
            seerr: service_from_env("SEERR_URL", "SEERR_API_KEY"),
            radarr: service_from_env("RADARR_URL", "RADARR_API_KEY"),
            sonarr: service_from_env("SONARR_URL", "SONARR_API_KEY"),
            prowlarr: service_from_env("PROWLARR_URL", "PROWLARR_API_KEY"),
        })
    }

    pub fn is_seerr_configured(&self) -> bool {
        self.seerr.is_some()
    }

    pub async fn mark_seerr_unconfigured(&self, database: &Database) {
        if self.seerr.is_none() {
            if let Err(error) = self
                .set_service_health(
                    database,
                    "seerr",
                    false,
                    Some("SEERR_URL and SEERR_API_KEY are required"),
                )
                .await
            {
                tracing::warn!(%error, "Could not persist Seerr configuration status");
            }
        }
    }

    pub async fn sync_requests(&self, database: &Database) -> Result<(), IntegrationError> {
        let seerr = self
            .seerr
            .as_ref()
            .ok_or(IntegrationError::NotConfigured("Seerr"))?;
        let requests = match self.fetch_seerr_requests(seerr).await {
            Ok(requests) => requests,
            Err(error) => {
                self.set_service_health(database, "seerr", false, Some(&error.to_string()))
                    .await?;
                return Err(error);
            }
        };
        self.set_service_health(database, "seerr", true, None)
            .await?;
        let now = Utc::now().timestamp();

        let mut active_ids = Vec::new();
        let mut approved = Vec::new();
        for request in requests {
            let Some(external_id) = request.get("id").and_then(value_id) else {
                continue;
            };
            let external_id = external_id.to_string();
            let status = request
                .get("status")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            if status == 2 {
                active_ids.push(external_id.clone());
                approved.push((external_id, request));
            } else {
                database.deactivate_seerr_request(&external_id, now).await?;
            }
        }
        database
            .deactivate_missing_seerr_requests(&active_ids, now)
            .await?;

        let radarr_movies = if self.radarr.is_some() {
            self.radarr_movies().await.ok()
        } else {
            None
        };
        let radarr_queue = if self.radarr.is_some() {
            self.radarr_queue().await.ok()
        } else {
            None
        };
        let sonarr_series = if self.sonarr.is_some() {
            self.sonarr_series().await.ok()
        } else {
            None
        };
        let sonarr_queue = if self.sonarr.is_some() {
            self.sonarr_queue().await.ok()
        } else {
            None
        };

        for (external_id, request) in approved {
            if let Some((media_type, media_key)) = request_identity(&request) {
                let requester_id = request
                    .get("requestedBy")
                    .and_then(|v| v.get("id"))
                    .and_then(value_id)
                    .unwrap_or_default()
                    .to_string();
                let requester_name = request
                    .get("requestedBy")
                    .and_then(|v| v.get("displayName").or_else(|| v.get("username")))
                    .and_then(Value::as_str)
                    .unwrap_or("Seerr user")
                    .to_owned();
                let request_id = database
                    .record_seerr_request(&SeerrRequestInput {
                        external_id,
                        requester_id,
                        requester_name,
                        media_type: if media_type == "tv" {
                            "series"
                        } else {
                            media_type
                        },
                        media_key: media_key.clone(),
                        approved: true,
                        now,
                    })
                    .await?;

                match media_type {
                    "movie" => {
                        let Some(movies) = radarr_movies.as_ref() else {
                            continue;
                        };
                        let Some(movie_tmdb) = request.pointer("/media/tmdbId").and_then(value_id)
                        else {
                            continue;
                        };
                        let Some(movie) = movies.iter().find(|movie| {
                            movie.get("tmdbId").and_then(value_id) == Some(movie_tmdb)
                        }) else {
                            continue;
                        };
                        let Some(movie_id) = movie.get("id").and_then(value_id) else {
                            continue;
                        };
                        let downloaded =
                            movie.get("downloaded").and_then(Value::as_bool) == Some(true);
                        let in_queue = radarr_queue
                            .as_ref()
                            .is_some_and(|queue| movie_in_queue(queue, movie_id));
                        if downloaded || in_queue {
                            database
                                .reconcile_external_media(
                                    MediaType::Movie,
                                    &media_key,
                                    if downloaded {
                                        QueueJobState::Satisfied
                                    } else {
                                        QueueJobState::Completed
                                    },
                                    now,
                                )
                                .await?;
                            continue;
                        }
                        let eligibility = self.movie_eligibility(database, movie, now).await?;
                        let title = movie
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or("Untitled movie");
                        database
                            .enqueue_for_request(
                                &request_id,
                                &NewQueueJob {
                                    media_type: MediaType::Movie,
                                    media_key,
                                    title: title.to_owned(),
                                    series_key: None,
                                    episode_sort_key: None,
                                    external_manager_id: Some(movie_id),
                                    eligibility,
                                    first_requested_at: parse_timestamp(
                                        request.get("createdAt"),
                                        now,
                                    ),
                                },
                                now,
                            )
                            .await?;
                    }
                    "tv" => {
                        let Some(series_list) = sonarr_series.as_ref() else {
                            continue;
                        };
                        let Some(tvdb_id) = request.pointer("/media/tvdbId").and_then(value_id)
                        else {
                            continue;
                        };
                        let Some(series) = series_list.iter().find(|series| {
                            series.get("tvdbId").and_then(value_id) == Some(tvdb_id)
                        }) else {
                            continue;
                        };
                        let Some(series_id) = series.get("id").and_then(value_id) else {
                            continue;
                        };
                        let Some(requested_seasons) = requested_seasons(&request) else {
                            continue;
                        };
                        let episodes = self.sonarr_episodes(series_id).await?;
                        let series_key = format!("tvdb:{tvdb_id}");
                        for episode in episodes {
                            let season = episode
                                .get("seasonNumber")
                                .and_then(Value::as_i64)
                                .unwrap_or(-1);
                            if !requested_seasons.contains(&season)
                                || episode.get("monitored").and_then(Value::as_bool) != Some(true)
                            {
                                continue;
                            }
                            let Some(episode_id) = episode.get("id").and_then(value_id) else {
                                continue;
                            };
                            let season_number = season.max(0);
                            let episode_number = episode
                                .get("episodeNumber")
                                .and_then(Value::as_i64)
                                .unwrap_or(0);
                            let key =
                                format!("tvdb:{tvdb_id}:S{season_number:02}E{episode_number:02}");
                            if episode.get("hasFile").and_then(Value::as_bool) == Some(true) {
                                database
                                    .reconcile_external_media(
                                        MediaType::Episode,
                                        &key,
                                        QueueJobState::Satisfied,
                                        now,
                                    )
                                    .await?;
                                continue;
                            }
                            if sonarr_queue
                                .as_ref()
                                .is_some_and(|queue| episode_in_queue(queue, episode_id))
                            {
                                database
                                    .reconcile_external_media(
                                        MediaType::Episode,
                                        &key,
                                        QueueJobState::Completed,
                                        now,
                                    )
                                    .await?;
                                continue;
                            }
                            if !air_date_passed(&episode, now) {
                                continue;
                            }
                            let sort_key = format!("{season_number:04}-{episode_number:04}");
                            let title = format!(
                                "{} — {} S{:02}E{:02}",
                                series
                                    .get("title")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Series"),
                                episode
                                    .get("title")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Episode"),
                                season_number,
                                episode_number,
                            );
                            database
                                .enqueue_for_request(
                                    &request_id,
                                    &NewQueueJob {
                                        media_type: MediaType::Episode,
                                        media_key: key,
                                        title,
                                        series_key: Some(series_key.clone()),
                                        episode_sort_key: Some(sort_key),
                                        external_manager_id: Some(episode_id),
                                        eligibility: JobEligibility::Available,
                                        first_requested_at: parse_timestamp(
                                            request.get("createdAt"),
                                            now,
                                        ),
                                    },
                                    now,
                                )
                                .await?;
                        }
                    }
                    _ => continue,
                }
            }
        }
        Ok(())
    }

    async fn fetch_seerr_requests(&self, seerr: &Service) -> Result<Vec<Value>, IntegrationError> {
        let mut all = Vec::new();
        let mut skip = 0;
        loop {
            let skip_text = skip.to_string();
            let page: Value = self
                .get_json(
                    seerr,
                    "Seerr",
                    "api/v1/request",
                    &[
                        ("filter", "all"),
                        ("mediaType", "all"),
                        ("take", "100"),
                        ("skip", &skip_text),
                        ("sort", "added"),
                        ("sortDirection", "asc"),
                    ],
                )
                .await?;
            let Some(results) = page.get("results").and_then(Value::as_array) else {
                return Err(IntegrationError::InvalidResponse(
                    "Seerr",
                    "request response had no results array".into(),
                ));
            };
            let count = results.len();
            all.extend(results.iter().cloned());
            if count < 100 || all.len() >= 10_000 {
                break;
            }
            skip += count;
        }
        Ok(all)
    }

    async fn radarr_movies(&self) -> Result<Vec<Value>, IntegrationError> {
        let service = self
            .radarr
            .as_ref()
            .ok_or(IntegrationError::NotConfigured("Radarr"))?;
        self.get_json::<Value>(service, "Radarr", "api/v3/movie", &[])
            .await?
            .as_array()
            .cloned()
            .ok_or_else(|| {
                IntegrationError::InvalidResponse(
                    "Radarr",
                    "movie response was not an array".into(),
                )
            })
    }

    async fn radarr_queue(&self) -> Result<Vec<Value>, IntegrationError> {
        let service = self
            .radarr
            .as_ref()
            .ok_or(IntegrationError::NotConfigured("Radarr"))?;
        self.get_json::<Value>(
            service,
            "Radarr",
            "api/v3/queue",
            &[("page", "1"), ("pageSize", "1000")],
        )
        .await?
        .get("records")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| {
            IntegrationError::InvalidResponse(
                "Radarr",
                "queue response had no records array".into(),
            )
        })
    }

    async fn sonarr_series(&self) -> Result<Vec<Value>, IntegrationError> {
        let service = self
            .sonarr
            .as_ref()
            .ok_or(IntegrationError::NotConfigured("Sonarr"))?;
        self.get_json::<Value>(service, "Sonarr", "api/v3/series", &[])
            .await?
            .as_array()
            .cloned()
            .ok_or_else(|| {
                IntegrationError::InvalidResponse(
                    "Sonarr",
                    "series response was not an array".into(),
                )
            })
    }

    async fn sonarr_queue(&self) -> Result<Vec<Value>, IntegrationError> {
        let service = self
            .sonarr
            .as_ref()
            .ok_or(IntegrationError::NotConfigured("Sonarr"))?;
        self.get_json::<Value>(
            service,
            "Sonarr",
            "api/v3/queue",
            &[("page", "1"), ("pageSize", "1000")],
        )
        .await?
        .get("records")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| {
            IntegrationError::InvalidResponse(
                "Sonarr",
                "queue response had no records array".into(),
            )
        })
    }

    async fn sonarr_episodes(&self, series_id: i64) -> Result<Vec<Value>, IntegrationError> {
        let service = self
            .sonarr
            .as_ref()
            .ok_or(IntegrationError::NotConfigured("Sonarr"))?;
        let id = series_id.to_string();
        self.get_json::<Value>(service, "Sonarr", "api/v3/episode", &[("seriesId", &id)])
            .await?
            .as_array()
            .cloned()
            .ok_or_else(|| {
                IntegrationError::InvalidResponse(
                    "Sonarr",
                    "episode response was not an array".into(),
                )
            })
    }

    async fn movie_eligibility(
        &self,
        database: &Database,
        movie: &Value,
        now: i64,
    ) -> Result<JobEligibility, IntegrationError> {
        let delay_days = database
            .setting_value("movie_theatrical_delay_days")
            .await?
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0);
        if let Some(digital) = movie
            .get("digitalRelease")
            .and_then(Value::as_str)
            .and_then(parse_date)
        {
            let at = digital.timestamp();
            return Ok(if at <= now {
                JobEligibility::Available
            } else {
                JobEligibility::FutureRelease { eligible_at: at }
            });
        }
        if let Some(theatrical) = movie
            .get("inCinemas")
            .and_then(Value::as_str)
            .and_then(parse_date)
        {
            let at = theatrical
                .timestamp()
                .saturating_add(delay_days.max(0) * 86_400);
            return Ok(if at <= now {
                JobEligibility::Available
            } else {
                JobEligibility::FutureRelease { eligible_at: at }
            });
        }
        Ok(JobEligibility::UnknownReleaseMetadata)
    }

    pub async fn search_one(
        &self,
        database: &Database,
        run_id: &str,
        media_type: MediaType,
        manager_id: i64,
        timeout_seconds: u64,
    ) -> Result<SearchOutcome, IntegrationError> {
        let (service, route, name, body, service_name) = match media_type {
            MediaType::Movie => (
                self.radarr
                    .as_ref()
                    .ok_or(IntegrationError::NotConfigured("Radarr"))?,
                "api/v3/command",
                "MoviesSearch",
                json!({"name":"MoviesSearch", "movieIds":[manager_id]}),
                "Radarr",
            ),
            MediaType::Episode => (
                self.sonarr
                    .as_ref()
                    .ok_or(IntegrationError::NotConfigured("Sonarr"))?,
                "api/v3/command",
                "EpisodeSearch",
                json!({"name":"EpisodeSearch", "episodeIds":[manager_id]}),
                "Sonarr",
            ),
        };
        let response = with_api_key(self.http.post(join_url(&service.base_url, route)), service)
            .json(&body)
            .send()
            .await
            .map_err(|source| IntegrationError::Http {
                service: service_name,
                source,
            })?;
        let response = checked(response, service_name)?;
        let command: Value = response
            .json()
            .await
            .map_err(|source| IntegrationError::Http {
                service: service_name,
                source,
            })?;
        let command_id = command.get("id").and_then(value_id).ok_or_else(|| {
            IntegrationError::InvalidResponse(service_name, "command response had no id".into())
        })?;
        database
            .mark_search_accepted(run_id, &command_id.to_string())
            .await?;

        let started = Utc::now().timestamp();
        loop {
            if Utc::now().timestamp().saturating_sub(started) >= timeout_seconds as i64 {
                return Err(IntegrationError::InvalidResponse(
                    service_name,
                    format!("{name} timed out; command {command_id} must be reconciled"),
                ));
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
            let route = format!("api/v3/command/{command_id}");
            let state: Value = self.get_json(service, service_name, &route, &[]).await?;
            let status = state
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if matches!(status, "queued" | "started" | "inProgress" | "running") {
                continue;
            }
            if status != "completed" {
                return Err(IntegrationError::InvalidResponse(
                    service_name,
                    format!("{name} ended with status {status}"),
                ));
            }
            break;
        }
        // Command completion and queue visibility can race briefly: Radarr may
        // have dispatched a grab while its queue endpoint is still updating.
        for attempt in 0..=15 {
            if self.is_satisfied(media_type, manager_id).await? {
                return Ok(SearchOutcome::Found);
            }
            if attempt < 15 {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
        Ok(SearchOutcome::NoResults)
    }

    pub async fn is_satisfied(
        &self,
        media_type: MediaType,
        manager_id: i64,
    ) -> Result<bool, IntegrationError> {
        match media_type {
            MediaType::Movie => {
                if self
                    .radarr_queue()
                    .await?
                    .iter()
                    .any(|item| movie_in_queue(std::slice::from_ref(item), manager_id))
                {
                    return Ok(true);
                }
                Ok(self.radarr_movies().await?.iter().any(|item| {
                    item.get("id").and_then(value_id) == Some(manager_id)
                        && item.get("downloaded").and_then(Value::as_bool) == Some(true)
                }))
            }
            MediaType::Episode => {
                if self
                    .sonarr_queue()
                    .await?
                    .iter()
                    .any(|item| episode_in_queue(std::slice::from_ref(item), manager_id))
                {
                    return Ok(true);
                }
                Ok(self
                    .sonarr_episodes_for_search(manager_id)
                    .await?
                    .get("hasFile")
                    .and_then(Value::as_bool)
                    == Some(true))
            }
        }
    }

    async fn sonarr_episodes_for_search(&self, episode_id: i64) -> Result<Value, IntegrationError> {
        let service = self
            .sonarr
            .as_ref()
            .ok_or(IntegrationError::NotConfigured("Sonarr"))?;
        let route = format!("api/v3/episode/{episode_id}");
        self.get_json(service, "Sonarr", &route, &[]).await
    }

    pub async fn availability(&self, database: &Database) -> ServiceAvailability {
        let radarr = self
            .check_service(
                database,
                "radarr",
                self.radarr.as_ref(),
                "Radarr",
                "api/v3/system/status",
            )
            .await;
        let sonarr = self
            .check_service(
                database,
                "sonarr",
                self.sonarr.as_ref(),
                "Sonarr",
                "api/v3/system/status",
            )
            .await;
        let prowlarr = self
            .check_service(
                database,
                "prowlarr",
                self.prowlarr.as_ref(),
                "Prowlarr",
                "api/v1/system/status",
            )
            .await;
        ServiceAvailability {
            radarr,
            sonarr,
            prowlarr,
        }
    }

    pub async fn availability_from_database(&self, database: &Database) -> ServiceAvailability {
        let rows: Vec<(String, i64)> =
            match sqlx::query_as("SELECT service, available FROM service_health")
                .fetch_all(database.pool())
                .await
            {
                Ok(rows) => rows,
                Err(error) => {
                    tracing::warn!(%error, "Could not read cached service health");
                    return ServiceAvailability::default();
                }
            };
        let available = |key: &str| {
            rows.iter()
                .any(|(service, status)| service == key && *status == 1)
        };
        ServiceAvailability {
            radarr: available("radarr"),
            sonarr: available("sonarr"),
            prowlarr: available("prowlarr"),
        }
    }

    /// Fail closed if Radarr or Sonarr reports a search command already in
    /// progress. This prevents GXE from colliding with administrator searches.
    pub async fn has_external_search_active(
        &self,
        available: ServiceAvailability,
    ) -> Result<bool, IntegrationError> {
        for (service, name, enabled) in [
            (self.radarr.as_ref(), "Radarr", available.radarr),
            (self.sonarr.as_ref(), "Sonarr", available.sonarr),
        ] {
            if !enabled {
                continue;
            }
            let service = service.ok_or(IntegrationError::NotConfigured(name))?;
            let commands: Value = self
                .get_json(service, name, "api/v3/command", &[("status", "started")])
                .await?;
            let Some(commands) = commands.as_array() else {
                return Err(IntegrationError::InvalidResponse(
                    name,
                    "active command response was not an array".into(),
                ));
            };
            if commands.iter().any(|command| {
                let command_name = command
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                matches!(
                    command_name,
                    "MoviesSearch" | "EpisodeSearch" | "SeriesSearch"
                )
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn check_service(
        &self,
        db: &Database,
        key: &str,
        service: Option<&Service>,
        name: &'static str,
        path: &str,
    ) -> bool {
        let Some(service) = service else {
            let _ = self
                .set_service_health(db, key, false, Some("not configured"))
                .await;
            return false;
        };
        let result = self.get_json::<Value>(service, name, path, &[]).await;
        let available = result.is_ok();
        let detail = result.as_ref().err().map(ToString::to_string);
        if let Err(error) = self
            .set_service_health(db, key, available, detail.as_deref())
            .await
        {
            tracing::warn!(service = key, error = %error, "Could not persist service health");
        }
        available
    }

    pub async fn sync_loop(self, database: Database, interval: Duration) {
        loop {
            if let Err(error) = self.sync_requests(&database).await {
                tracing::warn!(error = %error, "Seerr synchronization did not complete");
            }
            tokio::time::sleep(interval).await;
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        service: &Service,
        service_name: &'static str,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, IntegrationError> {
        let response = with_api_key(self.http.get(join_url(&service.base_url, path)), service)
            .query(query)
            .send()
            .await
            .map_err(|source| IntegrationError::Http {
                service: service_name,
                source,
            })?;
        checked(response, service_name)?
            .json()
            .await
            .map_err(|source| IntegrationError::Http {
                service: service_name,
                source,
            })
    }

    async fn set_service_health(
        &self,
        db: &Database,
        service: &str,
        available: bool,
        detail: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let mut tx = db.pool().begin().await?;
        let previous: Option<(i64,)> = sqlx::query_as(
            "SELECT available FROM service_health WHERE service=?",
        )
        .bind(service)
        .fetch_optional(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO service_health (service, available, checked_at, detail) VALUES (?, ?, unixepoch(), ?) \
             ON CONFLICT(service) DO UPDATE SET available=excluded.available, checked_at=excluded.checked_at, detail=excluded.detail",
        )
        .bind(service)
        .bind(if available { 1_i64 } else { 0_i64 })
        .bind(detail)
        .execute(&mut *tx)
        .await?;

        let changed = previous.is_some_and(|(was_available,)| (was_available != 0) != available);
        let initial_failure = previous.is_none() && !available && detail != Some("not configured");
        if changed || initial_failure {
            let service_title = match service {
                "seerr" => "Seerr",
                "radarr" => "Radarr",
                "sonarr" => "Sonarr",
                "prowlarr" => "Prowlarr",
                _ => service,
            };
            if available {
                let outage_kind = format!("service_unavailable_{service}");
                sqlx::query(
                    "UPDATE dashboard_events SET acknowledged_at=unixepoch() \
                     WHERE kind=? AND acknowledged_at IS NULL",
                )
                .bind(outage_kind)
                .execute(&mut *tx)
                .await?;
            }
            let (kind, severity, message) = if available {
                (
                    format!("service_restored_{service}"),
                    "info",
                    format!("{service_title} is available again."),
                )
            } else {
                (
                    format!("service_unavailable_{service}"),
                    "warning",
                    format!("{service_title} is unavailable. Related queue work may be waiting."),
                )
            };
            sqlx::query(
                "INSERT INTO dashboard_events (id, kind, severity, message, created_at) \
                 SELECT ?, ?, ?, ?, unixepoch() \
                 WHERE NOT EXISTS (SELECT 1 FROM dashboard_events WHERE kind=? AND acknowledged_at IS NULL)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&kind)
            .bind(severity)
            .bind(message)
            .bind(&kind)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

fn service_from_env(url_key: &str, token_key: &str) -> Option<Service> {
    let base_url = env::var(url_key).ok()?.trim_end_matches('/').to_owned();
    let api_key = env::var(token_key).ok()?;
    if base_url.is_empty() || api_key.trim().is_empty() {
        return None;
    }
    Some(Service { base_url, api_key })
}

fn with_api_key(request: reqwest::RequestBuilder, service: &Service) -> reqwest::RequestBuilder {
    request.header("X-Api-Key", &service.api_key)
}

fn join_url(base: &str, route: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        route.trim_start_matches('/')
    )
}

fn checked(
    response: reqwest::Response,
    service: &'static str,
) -> Result<reqwest::Response, IntegrationError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    Err(IntegrationError::Status {
        service,
        status,
        retry_after,
    })
}

fn value_id(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str()?.parse().ok())
}

fn episode_in_queue(queue: &[Value], episode_id: i64) -> bool {
    queue.iter().any(|item| {
        item.get("episodeId").and_then(value_id) == Some(episode_id)
            || item.pointer("/episode/id").and_then(value_id) == Some(episode_id)
            || item
                .get("episodeIds")
                .and_then(Value::as_array)
                .is_some_and(|ids| ids.iter().any(|id| value_id(id) == Some(episode_id)))
    })
}

fn movie_in_queue(queue: &[Value], movie_id: i64) -> bool {
    queue.iter().any(|item| {
        item.get("movieId").and_then(value_id) == Some(movie_id)
            || item.pointer("/movie/id").and_then(value_id) == Some(movie_id)
    })
}

fn request_identity(request: &Value) -> Option<(&'static str, String)> {
    let media = request.get("media")?;
    let kind = request
        .get("type")
        .and_then(Value::as_str)
        .or_else(|| media.get("mediaType").and_then(Value::as_str))
        .unwrap_or_else(|| {
            if media.get("tvdbId").and_then(value_id).is_some() {
                "tv"
            } else {
                "movie"
            }
        });
    let external_id = match kind {
        "movie" => media.get("tmdbId").and_then(value_id)?,
        "tv" | "series" => media.get("tvdbId").and_then(value_id)?,
        _ => return None,
    };
    Some((
        if kind == "movie" { "movie" } else { "tv" },
        format!("{kind}:{external_id}"),
    ))
}

fn requested_seasons(request: &Value) -> Option<Vec<i64>> {
    request.get("seasons")?.as_array().map(|seasons| {
        seasons
            .iter()
            .filter_map(|season| {
                season
                    .as_i64()
                    .or_else(|| season.get("seasonNumber").and_then(Value::as_i64))
            })
            .collect()
    })
}

fn parse_date(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
        .or_else(|| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|date| date.and_utc())
        })
}

fn parse_timestamp(value: Option<&Value>, fallback: i64) -> i64 {
    value
        .and_then(Value::as_str)
        .and_then(parse_date)
        .map(|date| date.timestamp())
        .unwrap_or(fallback)
}

fn air_date_passed(episode: &Value, now: i64) -> bool {
    episode
        .get("airDateUtc")
        .and_then(Value::as_str)
        .and_then(parse_date)
        .is_some_and(|date| date.timestamp() <= now)
}
