CREATE TABLE requests (
    id TEXT PRIMARY KEY NOT NULL,
    source TEXT NOT NULL CHECK (source = 'seerr'),
    external_id TEXT NOT NULL,
    requester_id TEXT NOT NULL,
    requester_name TEXT NOT NULL,
    media_type TEXT NOT NULL CHECK (media_type IN ('movie', 'series')),
    media_key TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (source, external_id)
);

CREATE TABLE queue_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    media_type TEXT NOT NULL CHECK (media_type IN ('movie', 'episode')),
    media_key TEXT NOT NULL,
    title TEXT NOT NULL,
    series_key TEXT,
    episode_sort_key TEXT,
    external_manager_id INTEGER,
    state TEXT NOT NULL CHECK (state IN (
        'waiting_metadata', 'waiting_release', 'queued', 'retry_wait',
        'running', 'suspended', 'satisfied', 'completed', 'cancelled', 'dismissed'
    )),
    priority INTEGER NOT NULL DEFAULT 0,
    eligible_at INTEGER,
    next_attempt_at INTEGER,
    no_result_attempts INTEGER NOT NULL DEFAULT 0 CHECK (no_result_attempts >= 0),
    first_requested_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (media_type, media_key),
    CHECK ((media_type = 'movie' AND series_key IS NULL) OR (media_type = 'episode' AND series_key IS NOT NULL))
);

CREATE INDEX queue_jobs_ready_idx ON queue_jobs(state, next_attempt_at, eligible_at, priority DESC, first_requested_at);
CREATE INDEX queue_jobs_series_idx ON queue_jobs(series_key, episode_sort_key);

CREATE TABLE request_jobs (
    request_id TEXT NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
    job_id TEXT NOT NULL REFERENCES queue_jobs(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (request_id, job_id)
);

CREATE TABLE search_runs (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES queue_jobs(id),
    command_id TEXT,
    state TEXT NOT NULL CHECK (state IN ('submitting', 'active', 'reconciling', 'completed', 'no_results', 'technical_error', 'satisfied')),
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    outcome_detail TEXT
);

CREATE UNIQUE INDEX one_active_gxe_search ON search_runs((1))
    WHERE state IN ('submitting', 'active', 'reconciling');
CREATE INDEX search_runs_started_idx ON search_runs(started_at DESC);

CREATE TABLE service_health (
    service TEXT PRIMARY KEY NOT NULL CHECK (service IN ('seerr', 'radarr', 'sonarr', 'prowlarr')),
    available INTEGER NOT NULL DEFAULT 0 CHECK (available IN (0, 1)),
    checked_at INTEGER,
    detail TEXT
);

CREATE TABLE indexer_cooldowns (
    external_id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    cooldown_until INTEGER NOT NULL,
    reason TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE dashboard_events (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error', 'critical')),
    message TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    acknowledged_at INTEGER
);

CREATE TABLE scheduler_state (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
