export class ApiError extends Error {
  constructor(message: string, readonly status: number) {
    super(message);
    this.name = 'ApiError';
  }
}

export async function apiRequest<T>(token: string, path: string, init: RequestInit = {}): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`/api/v1/${path.replace(/^\//, '')}`, {
      ...init,
      headers: {
        ...(init.body ? { 'Content-Type': 'application/json' } : {}),
        Authorization: `Bearer ${token}`,
        ...init.headers,
      },
    });
  } catch {
    throw new ApiError('GXE non è raggiungibile. Verifica la connessione e riprova.', 0);
  }

  const body = await response.json().catch(() => ({}));
  if (!response.ok) {
    if (response.status === 401) throw new ApiError('Sessione scaduta. Inserisci di nuovo il token amministratore.', 401);
    const message = typeof body.error === 'string' ? body.error : `Richiesta non riuscita (${response.status})`;
    throw new ApiError(message, response.status);
  }
  return body as T;
}

export function normalizeSettings(raw: Record<string, unknown>): import('../types/api').Settings {
  const numeric = (key: string, fallback: number) => {
    const value = Number(raw[key]);
    return Number.isFinite(value) ? value : fallback;
  };
  return {
    minimum_interval_seconds: numeric('minimum_interval_seconds', 600),
    queue_capacity: numeric('queue_capacity', 0),
    search_timeout_seconds: numeric('search_timeout_seconds', 900),
    movie_theatrical_delay_days: numeric('movie_theatrical_delay_days', 0),
  };
}
