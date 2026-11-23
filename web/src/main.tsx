import React, { useCallback, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import './style.css';

type Job = { id: string; media_type: string; title: string; state: string; priority: number; next_attempt_at?: number; no_result_attempts: number };
type Status = { paused: boolean; counts: Record<string, number>; active_search?: { id: string; state: string; job_id: string } | null; services: { service: string; available: boolean; detail?: string }[]; active_cooldowns: number };
const storedToken = () => sessionStorage.getItem('gxe-token') || '';

function App() {
  const [token, setToken] = useState(storedToken);
  const [status, setStatus] = useState<Status | null>(null);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [runs, setRuns] = useState<any[]>([]);
  const [settings, setSettings] = useState<any>({});
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [filter, setFilter] = useState('all');
  const api = useCallback(async (path: string, init: RequestInit = {}) => {
    const response = await fetch(`/api/v1/${path}`, { ...init, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json', ...init.headers } });
    if (response.status === 401) { sessionStorage.removeItem('gxe-token'); setToken(''); throw new Error('Session expired. Enter your admin token again.'); }
    const body = await response.json();
    if (!response.ok) throw new Error(body.error || `Request failed (${response.status})`);
    return body;
  }, [token]);
  const refresh = useCallback(async () => {
    if (!token) return;
    try {
      const [s, q, h, prefs] = await Promise.all([api('status'), api('queue'), api('history'), api('settings')]);
      setStatus(s); setJobs(q.items); setRuns(h.items); setSettings(prefs); setError('');
    } catch (e) { setError(e instanceof Error ? e.message : 'Unable to load dashboard'); }
  }, [api, token]);
  useEffect(() => { void refresh(); const timer = window.setInterval(refresh, 10000); return () => window.clearInterval(timer); }, [refresh]);
  const action = async (path: string, method = 'POST', body?: unknown) => {
    setBusy(true); try { await api(path, { method, ...(body === undefined ? {} : { body: JSON.stringify(body) }) }); await refresh(); }
    catch (e) { setError(e instanceof Error ? e.message : 'Action failed'); } finally { setBusy(false); }
  };
  if (!token) return <main className="login"><div className="brand-mark">G</div><p className="eyebrow">GXE REPUBLIC · SELF-HOSTED</p><h1>Search Queue</h1><p className="muted">Sign in with the administrator token configured for this instance.</p><form onSubmit={e => { e.preventDefault(); const value = new FormData(e.currentTarget).get('token')?.toString() || ''; sessionStorage.setItem('gxe-token', value); setToken(value); }}><input name="token" type="password" placeholder="Administrator token" autoComplete="current-password" required/><button className="primary">Open dashboard</button></form></main>;
  const visibleJobs = jobs.filter(job => filter === 'all' || job.media_type === filter);
  const activeRun = status?.active_search;
  return <div className="shell"><aside><div className="brand"><div className="brand-mark">G</div><div><b>GXE</b><small>SEARCH QUEUE</small></div></div><nav><a className="selected">◈ &nbsp; Overview</a><a href="#queue">▤ &nbsp; Queue</a><a href="#history">◷ &nbsp; History</a><a href="#settings">⚙ &nbsp; Settings</a></nav><div className="sidebar-bottom"><span className="dot green"/> Local instance <button className="link" onClick={() => { sessionStorage.removeItem('gxe-token'); setToken(''); }}>Sign out</button></div></aside>
    <main className="content"><header><div><p className="eyebrow">CONTROL CENTER</p><h1>Overview</h1></div><div className="head-actions"><span className="live"><i/> LIVE</span><button className={status?.paused ? 'primary' : 'secondary'} disabled={busy} onClick={() => action('queue/pause', 'POST', { paused: !status?.paused })}>{status?.paused ? 'Resume queue' : 'Pause queue'}</button></div></header>
      {error && <div className="alert">{error}</div>}
      {status?.paused && <div className="notice"><b>Queue paused</b><span>Queued work is preserved. Resume to continue scheduling.</span></div>}
      <div className="notice"><b>Preview safety</b><span>Disable Seerr automatic search before resuming. Prowlarr per-indexer 429 handling is not implemented or validated; GXE does not guarantee indexer protection.</span></div>
      <section className="stats"><article><span>ACTIVE WORK</span><strong>{activeRun ? '1' : '0'}</strong><small>{activeRun ? `Run ${activeRun.id.slice(0, 8)} · ${activeRun.state}` : 'No search in progress'}</small></article><article><span>QUEUE SIZE</span><strong>{(status?.counts?.movies || 0) + (status?.counts?.episodes || 0)}</strong><small>{status?.counts?.movies || 0} movies · {status?.counts?.episodes || 0} episodes</small></article><article><span>SUSPENDED</span><strong>{status?.counts?.suspended || 0}</strong><small>Needs attention or retry</small></article><article><span>INDEXER COOLDOWNS</span><strong>{status?.active_cooldowns || 0}</strong><small>Scheduling waits while active</small></article></section>
      <div className="grid"><section className="panel services"><div className="panel-head"><div><p className="eyebrow">INTEGRATIONS</p><h2>Connected services</h2></div><button className="icon-button" onClick={() => void refresh()} title="Refresh">↻</button></div><div className="service-list">{['seerr','radarr','sonarr','prowlarr'].map(key => { const item = status?.services?.find(x => x.service === key); return <div className="service" key={key} title={item?.detail || undefined}><span className={`dot ${item?.available ? 'green' : 'gray'}`}/><b>{key[0].toUpperCase()+key.slice(1)}</b><span className="service-state">{item?.available ? 'Connected' : item?.detail || 'Not connected'}</span></div>; })}</div><p className="footnote">Searches stay paused until the required manager and Prowlarr checks are healthy.</p></section>
      <section className="panel current"><p className="eyebrow">GLOBAL SCHEDULER</p><h2>{activeRun ? 'Search in progress' : 'Ready for next search'}</h2><p className="muted">One GXE-managed search at a time, with at least 10 minutes between starts.</p>{activeRun && <div className="run-box"><span className="spinner"/><div><b>{activeRun.job_id}</b><small>{activeRun.state} · inspect History to reconcile if needed</small></div></div>}</section></div>
      <section className="panel queue-panel" id="queue"><div className="panel-head"><div><p className="eyebrow">WORK ITEMS</p><h2>Search queue</h2></div><div className="head-actions"><select value={filter} onChange={e => setFilter(e.target.value)}><option value="all">All media</option><option value="movie">Movies</option><option value="episode">Episodes</option></select><button className="secondary" disabled={busy || jobs.length === 0} onClick={() => { if (window.confirm('Dismiss all not-yet-started queue items? Active searches will continue.')) void action('queue/empty'); }}>Empty queue</button></div></div><div className="table-wrap"><table><thead><tr><th>MEDIA</th><th>TYPE</th><th>STATE</th><th>RETRIES</th><th>ACTIONS</th></tr></thead><tbody>{visibleJobs.map(job => <tr key={job.id}><td><b>{job.title}</b><small>{job.id.slice(0, 8)}</small></td><td><span className="tag">{job.media_type}</span></td><td><span className={`state state-${job.state}`}>{job.state.replace('_',' ')}</span></td><td>{job.no_result_attempts}</td><td className="row-actions"><button onClick={() => action(`queue/${job.id}/priority`)} disabled={busy || ['running'].includes(job.state)} title="Prioritize">↑</button><button onClick={() => action(`queue/${job.id}/retry`)} disabled={busy} title="Retry now">↻</button><button onClick={() => action(`queue/${job.id}`, 'DELETE')} disabled={busy || job.state === 'running'} title="Remove">×</button></td></tr>)}{!visibleJobs.length && <tr><td colSpan={5} className="empty">Queue is clear. Approved Seerr requests will appear here after synchronization.</td></tr>}</tbody></table></div></section>
      <section className="panel history-panel" id="history"><div className="panel-head"><div><p className="eyebrow">RECENT ACTIVITY</p><h2>Search history</h2></div></div><div className="table-wrap"><table><thead><tr><th>RUN</th><th>STATE</th><th>STARTED</th><th>DETAIL</th><th>RECONCILE</th></tr></thead><tbody>{runs.slice(0, 12).map(run => <tr key={run.id}><td>{run.id.slice(0, 8)}</td><td><span className={`state state-${run.state}`}>{run.state.replace('_',' ')}</span></td><td>{new Date(run.started_at * 1000).toLocaleString()}</td><td>{run.detail || '—'}</td><td>{['submitting','active','reconciling'].includes(run.state) && <select defaultValue="" onChange={e => e.target.value && action(`runs/${run.id}/reconcile`, 'POST', { outcome: e.target.value, detail: 'Reconciled by administrator' })}><option value="" disabled>Choose outcome</option><option value="found">Found / grabbed</option><option value="no_results">No results</option><option value="satisfied">Already satisfied</option><option value="technical_error">Technical error</option></select>}</td></tr>)}{!runs.length && <tr><td colSpan={5} className="empty">No searches have run yet.</td></tr>}</tbody></table></div></section>
      <section className="panel settings-panel" id="settings"><div className="panel-head"><div><p className="eyebrow">SCHEDULING</p><h2>Settings</h2></div></div><form className="settings-form" onSubmit={e => { e.preventDefault(); const data = new FormData(e.currentTarget); void action('settings', 'PUT', Object.fromEntries(['minimum_interval_seconds','queue_capacity','search_timeout_seconds','movie_theatrical_delay_days'].map(k => [k, Number(data.get(k))]))); }}><label>Minimum interval (seconds)<input name="minimum_interval_seconds" type="number" min="600" defaultValue={settings.minimum_interval_seconds ?? 600}/></label><label>Queue capacity (0 = unlimited)<input name="queue_capacity" type="number" min="0" defaultValue={settings.queue_capacity ?? 0}/></label><label>Search timeout (seconds)<input name="search_timeout_seconds" type="number" min="1" defaultValue={settings.search_timeout_seconds ?? 900}/></label><label>Theatrical release delay (days)<input name="movie_theatrical_delay_days" type="number" min="0" defaultValue={settings.movie_theatrical_delay_days ?? 0}/></label><button className="primary" disabled={busy}>Save settings</button></form></section>
      <footer>GXE Search Queue <span>Experimental · v0.1.0-pre</span></footer>
    </main></div>;
}

createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
