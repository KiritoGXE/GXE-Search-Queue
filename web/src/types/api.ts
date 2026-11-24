export type MediaType = 'movie' | 'episode';
export type QueueState =
  | 'waiting_metadata'
  | 'waiting_release'
  | 'queued'
  | 'retry_wait'
  | 'running'
  | 'suspended'
  | 'satisfied'
  | 'completed'
  | 'cancelled'
  | 'dismissed';
export type RunState = 'submitting' | 'active' | 'reconciling' | 'completed' | 'no_results' | 'technical_error' | 'satisfied';

export interface QueueJob {
  id: string;
  media_type: MediaType;
  media_key: string;
  title: string;
  series_key: string | null;
  episode_sort_key: string | null;
  external_manager_id: number | null;
  state: QueueState;
  priority: number;
  eligible_at: number | null;
  next_attempt_at: number | null;
  no_result_attempts: number;
  first_requested_at: number;
}

export interface ServiceHealth {
  service: 'seerr' | 'radarr' | 'sonarr' | 'prowlarr' | string;
  available: boolean;
  checked_at: number | null;
  detail: string | null;
}

export interface SearchRun {
  id: string;
  job_id: string;
  state: RunState;
  started_at: number;
  completed_at: number | null;
  detail: string | null;
  title?: string | null;
  media_type?: MediaType | null;
}

export interface DashboardEvent {
  id: string;
  kind: string;
  severity: 'info' | 'warning' | 'error' | 'critical';
  message: string;
  created_at: number;
  acknowledged_at: number | null;
}

export interface StatusResponse {
  paused: boolean;
  counts: Record<string, number>;
  outcomes_90d: { completed: number; no_results: number; technical_errors: number };
  active_search: { id: string; job_id: string; state: RunState; started_at: number; title: string; media_type: MediaType } | null;
  services: ServiceHealth[];
  active_cooldowns: number;
}

export interface RequestSummary {
  external_id: string;
  requester_name: string;
  media_type: string;
  created_at: number;
}

export interface QueueJobDetails {
  job: QueueJob;
  requests: RequestSummary[];
  runs: SearchRun[];
}

export interface IndexerSummary {
  id: number;
  name: string;
  enabled: boolean;
  protocol: string | null;
  privacy: string | null;
  disabled_until: string | null;
  most_recent_failure: string | null;
  queries_24h: number;
  grabs_24h: number;
  failed_queries_24h: number;
  average_response_time_ms_24h: number;
}

export interface IndexerOverview {
  checked_at: number;
  period_started_at: number;
  indexers: IndexerSummary[];
}

export interface Cooldown {
  id: number;
  name: string;
  cooldown_until: number;
  reason: string;
}

export interface Settings {
  minimum_interval_seconds: number;
  queue_capacity: number;
  search_timeout_seconds: number;
  movie_theatrical_delay_days: number;
}
