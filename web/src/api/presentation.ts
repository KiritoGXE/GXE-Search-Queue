import type { QueueJob, QueueState, ServiceHealth } from '../types/api';

export type SystemState = 'online' | 'degraded' | 'offline' | 'unknown';

export function deriveSystemState(services: ServiceHealth[] | null, loading: boolean, error: string | null): SystemState {
  if (!services?.length) return loading ? 'unknown' : error ? 'unknown' : 'unknown';
  const byName = new Map(services.map(service => [service.service, service]));
  const required = ['radarr', 'sonarr', 'prowlarr'].map(name => byName.get(name));
  if (required.every(service => service && !service.available)) return 'offline';
  if (required.some(service => !service?.available) || !byName.get('seerr')?.available) return 'degraded';
  return 'online';
}

export function stateLabel(state: QueueState | string): string {
  const labels: Record<string, string> = {
    waiting_metadata: 'Metadati in attesa', waiting_release: 'In attesa di uscita', queued: 'In coda',
    retry_wait: 'Retry programmato', running: 'In elaborazione', suspended: 'Sospeso',
    satisfied: 'Già soddisfatto', completed: 'Completato', cancelled: 'Annullato', dismissed: 'Rimosso',
    submitting: 'Invio comando', active: 'Attivo', reconciling: 'Da riconciliare', no_results: 'Nessun risultato',
    technical_error: 'Errore tecnico',
  };
  return labels[state] || state.replaceAll('_', ' ');
}

export function mediaLabel(type: string): string {
  return type === 'movie' ? 'Film' : type === 'episode' ? 'Episodio' : type;
}

export function formatDate(epochSeconds: number | null | undefined): string {
  if (!epochSeconds) return '—';
  return new Intl.DateTimeFormat('it-IT', { dateStyle: 'medium', timeStyle: 'short' }).format(epochSeconds * 1000);
}

export function formatDuration(seconds: number): string {
  const safe = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const remainder = safe % 60;
  return hours ? `${hours}h ${String(minutes).padStart(2, '0')}m` : `${minutes}m ${String(remainder).padStart(2, '0')}s`;
}

export function nextAttemptLabel(job: QueueJob, now: number): string {
  const target = job.next_attempt_at ?? job.eligible_at;
  if (!target) return job.state === 'queued' ? 'Pronto per lo scheduler' : '—';
  const left = target - now;
  if (left <= 0) return 'Pronto per lo scheduler';
  return `Tra ${formatDuration(left)}`;
}

export function describeState(state: string): string {
  switch (state) {
    case 'queued': return 'In attesa del prossimo turno dello scheduler.';
    case 'retry_wait': return 'Il prossimo tentativo è pianificato secondo la policy di retry.';
    case 'waiting_release': return 'La data di uscita non è ancora eleggibile.';
    case 'waiting_metadata': return 'GXE attende metadati sufficienti prima di avviare una ricerca.';
    case 'suspended': return 'Il lavoro è sospeso e richiede una verifica o il ripristino di un servizio.';
    case 'running': return 'Una ricerca è in corso; il download, se avviato, procede indipendentemente.';
    default: return 'Stato sincronizzato dal backend GXE.';
  }
}
