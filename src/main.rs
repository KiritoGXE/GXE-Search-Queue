use std::{env, path::Path, time::Duration};

use gxe_search_queue::{
    api::{self, unix_now},
    database::Database,
    integrations::Integrations,
    scheduler::{SchedulerPolicy, SearchOutcome},
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let database_path =
        env::var("DATABASE_PATH").unwrap_or_else(|_| "./data/gxe-search-queue.sqlite3".into());
    if database_path != ":memory:" {
        if let Some(parent) = Path::new(&database_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let database = Database::connect(&database_path).await?;
    database.migrate().await?;
    for (key, value) in [
        ("minimum_interval_seconds", "600"),
        ("queue_capacity", "0"),
        ("search_timeout_seconds", "900"),
        ("movie_theatrical_delay_days", "0"),
    ] {
        if database.setting_value(key).await?.is_none() {
            database.set_setting(key, value, unix_now()).await?;
        }
    }
    if database.setting_value("global_paused").await?.is_none() {
        database.set_global_paused(true, unix_now()).await?;
    }
    let active = database.active_searches().await?;
    if !active.is_empty() {
        tracing::warn!(
            count = active.len(),
            "Persisted searches need external reconciliation before scheduling can resume"
        );
    }
    let admin_token = env::var("GXE_ADMIN_TOKEN").map_err(|_| {
        "GXE_ADMIN_TOKEN is required; set a random secret of at least 32 characters"
    })?;
    if admin_token.len() < 32 || admin_token.starts_with("replace-with-") {
        return Err(
            "Set GXE_ADMIN_TOKEN to a unique random secret of at least 32 characters".into(),
        );
    }
    let bind = env::var("API_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".into());
    let port = env::var("API_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);
    let address = api::parse_bind_address(&bind, port)?;
    let integrations = Integrations::from_env()?;
    let web_dist = env::var("WEB_DIST").unwrap_or_else(|_| "/app/web".into());
    let app = api::router(
        database.clone(),
        admin_token,
        web_dist.into(),
        integrations.clone(),
    );
    let worker_db = database.clone();
    let worker = tokio::spawn(async move {
        run_worker(worker_db, integrations).await;
    });
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, path = %database_path, "GXE Search Queue API and dashboard listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    worker.abort();
    database.close().await;
    Ok(())
}

async fn run_worker(database: Database, integrations: Integrations) {
    let policy = SchedulerPolicy::default();
    let mut health_checked = 0_i64;
    let mut sync_at = 0_i64;
    let mut external_checked = 0_i64;
    let mut external_search_active = true;
    let mut tick = tokio::time::interval(Duration::from_secs(3));
    loop {
        tick.tick().await;
        let now = unix_now();
        if now - sync_at >= 60 {
            if integrations.is_seerr_configured() {
                if let Err(error) = integrations.sync_requests(&database).await {
                    tracing::warn!(%error, "Seerr synchronization failed; it will retry");
                }
            } else {
                integrations.mark_seerr_unconfigured(&database).await;
            }
            sync_at = now;
            if let Err(error) = database.prune_history(now).await {
                tracing::warn!(%error, "Could not prune old history");
            }
        }
        if now - health_checked >= 30 {
            integrations.availability(&database).await;
            health_checked = now;
        }
        let availability = integrations.availability_from_database(&database).await;
        if now - external_checked >= 10 {
            if let Err(error) = integrations.reconcile_active_searches(&database).await {
                tracing::warn!(%error, "Could not reconcile a persisted search; scheduler is waiting");
                external_search_active = true;
                external_checked = now;
                continue;
            }
            match integrations.has_external_search_active(availability).await {
                Ok(active) => external_search_active = active,
                Err(error) => {
                    tracing::warn!(%error, "Could not verify external search activity; scheduler is waiting");
                    external_search_active = true;
                }
            }
            external_checked = now;
        }
        if external_search_active {
            continue;
        }
        match database.claim_next_search(now, availability, &policy).await {
            Ok(Some(claimed)) => {
                let Some(manager_id) = claimed.job.external_manager_id else {
                    let _ = database
                        .finish_search(
                            &claimed.run_id,
                            SearchOutcome::TechnicalError,
                            unix_now(),
                            Some("missing Radarr/Sonarr identifier"),
                        )
                        .await;
                    continue;
                };
                match integrations
                    .is_satisfied(claimed.job.media_type, manager_id)
                    .await
                {
                    Ok(true) => {
                        let _ = database
                            .finish_search(
                                &claimed.run_id,
                                SearchOutcome::AlreadySatisfied,
                                unix_now(),
                                Some("already present in library"),
                            )
                            .await;
                        continue;
                    }
                    Err(error) => {
                        let _ = database
                            .finish_search(
                                &claimed.run_id,
                                SearchOutcome::TechnicalError,
                                unix_now(),
                                Some(&error.to_string()),
                            )
                            .await;
                        continue;
                    }
                    Ok(false) => {}
                }
                let timeout = database
                    .setting_value("search_timeout_seconds")
                    .await
                    .ok()
                    .flatten()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(900);
                match integrations
                    .search_one(
                        &database,
                        &claimed.run_id,
                        claimed.job.media_type,
                        manager_id,
                        timeout,
                    )
                    .await
                {
                    Ok(outcome) => {
                        let detail = match outcome {
                            SearchOutcome::Found => "A matching manager queue item or library file was detected",
                            SearchOutcome::NoResults => "No matching queue item or library file appeared during the result reconciliation window",
                            SearchOutcome::AlreadySatisfied => "The media was already satisfied before search",
                            SearchOutcome::TechnicalError => "The external service reported a technical error",
                        };
                        if let Err(error) = database
                            .finish_search(&claimed.run_id, outcome, unix_now(), Some(detail))
                            .await
                        {
                            tracing::error!(%error, run_id = %claimed.run_id, "Could not finish search run");
                        }
                    }
                    Err(error) => {
                        tracing::error!(%error, run_id = %claimed.run_id, "Search outcome is uncertain; preserving run for reconciliation");
                        let _ = database.mark_search_ambiguous(&claimed.run_id).await;
                    }
                }
            }
            Ok(None) => {}
            Err(error) => tracing::error!(%error, "Scheduler could not claim work"),
        }
    }
}
