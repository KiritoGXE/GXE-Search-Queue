import { useCallback, useEffect, useRef, useState } from 'react';
import { ApiError, apiRequest, normalizeSettings } from '../api/client';
import type { Cooldown, DashboardEvent, QueueJob, SearchRun, ServiceHealth, Settings, StatusResponse } from '../types/api';

type SectionKey = 'status' | 'queue' | 'history' | 'events' | 'settings' | 'cooldowns';
export type SectionErrors = Partial<Record<SectionKey, string>>;

export function useDashboard(token: string, onUnauthorized: () => void) {
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [jobs, setJobs] = useState<QueueJob[] | null>(null);
  const [runs, setRuns] = useState<SearchRun[] | null>(null);
  const [events, setEvents] = useState<DashboardEvent[] | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [cooldowns, setCooldowns] = useState<Cooldown[] | null>(null);
  const [errors, setErrors] = useState<SectionErrors>({});
  const [refreshing, setRefreshing] = useState(false);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const [notice, setNotice] = useState<{ kind: 'success' | 'error'; message: string } | null>(null);
  const unauthorizedRef = useRef(onUnauthorized);
  unauthorizedRef.current = onUnauthorized;
  const inFlight = useRef(false);

  const refresh = useCallback(async () => {
    if (!token || inFlight.current) return;
    inFlight.current = true;
    setRefreshing(true);
    const paths: { key: SectionKey; path: string }[] = [
      { key: 'status', path: 'status' }, { key: 'queue', path: 'queue' },
      { key: 'history', path: 'history' }, { key: 'events', path: 'events' },
      { key: 'settings', path: 'settings' }, { key: 'cooldowns', path: 'cooldowns' },
    ];
    const results = await Promise.allSettled(paths.map(item => apiRequest<Record<string, unknown>>(token, item.path)));
    const nextErrors: SectionErrors = {};
    results.forEach((result, index) => {
      const key = paths[index].key;
      if (result.status === 'fulfilled') {
        const body = result.value;
        switch (key) {
          case 'status': setStatus(body as unknown as StatusResponse); break;
          case 'queue': setJobs((body.items || []) as QueueJob[]); break;
          case 'history': setRuns((body.items || []) as SearchRun[]); break;
          case 'events': setEvents((body.items || []) as DashboardEvent[]); break;
          case 'settings': setSettings(normalizeSettings(body)); break;
          case 'cooldowns': setCooldowns((body.items || []) as Cooldown[]); break;
        }
      } else {
        const error = result.reason;
        if (error instanceof ApiError && error.status === 401) unauthorizedRef.current();
        nextErrors[key] = error instanceof Error ? error.message : 'Aggiornamento non riuscito.';
      }
    });
    setErrors(nextErrors);
    if (results.some(result => result.status === 'fulfilled')) setLastUpdated(Date.now());
    setRefreshing(false);
    inFlight.current = false;
  }, [token]);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 10_000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const execute = useCallback(async (key: string, path: string, method = 'POST', body?: unknown) => {
    setPendingAction(key);
    setNotice(null);
    try {
      const result = await apiRequest<{ error?: string }>(token, path, {
        method,
        ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      });
      const successMessages: Record<string, string> = {
        'queue/pause': 'Stato della coda aggiornato.',
        'queue/empty': 'Lavori non avviati rimossi dalla coda.',
        'settings': 'Impostazioni salvate.',
      };
      setNotice({ kind: 'success', message: successMessages[key] || 'Operazione completata.' });
      await refresh();
      return result;
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) unauthorizedRef.current();
      setNotice({ kind: 'error', message: error instanceof Error ? error.message : 'Operazione non riuscita.' });
      throw error;
    } finally {
      setPendingAction(null);
    }
  }, [refresh, token]);

  const clearNotice = useCallback(() => setNotice(null), []);

  return { status, jobs, runs, events, settings, cooldowns, errors, refreshing, pendingAction, lastUpdated, notice, refresh, execute, clearNotice };
}
