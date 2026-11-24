import { useEffect, useMemo, useState } from 'react';
import { ArrowUp, ChevronDown, ChevronRight, Film, ListFilter, RotateCcw, Search, Trash2, Tv2 } from 'lucide-react';
import { apiRequest } from '../../api/client';
import { describeState, formatDate, mediaLabel, nextAttemptLabel, stateLabel } from '../../api/presentation';
import { Button, ConfirmDialog, Drawer, EmptyState, IconButton, InlineError, SectionHeading, Skeleton, StatusBadge } from '../../components/primitives';
import type { QueueJob, QueueJobDetails, QueueState, SearchRun } from '../../types/api';

export type ExecuteAction = (key: string, path: string, method?: string, body?: unknown) => Promise<unknown>;
type Filter = 'all' | 'waiting' | 'retry_wait' | 'running' | 'suspended';
type Sort = 'queue' | 'priority' | 'title' | 'oldest' | 'next';
type Entry = { kind: 'movie' | 'series'; key: string; label: string; jobs: QueueJob[] };

const filters: { id: Filter; label: string }[] = [
  { id: 'all', label: 'Tutti' }, { id: 'waiting', label: 'In attesa' }, { id: 'retry_wait', label: 'Retry' },
  { id: 'running', label: 'Attivi' }, { id: 'suspended', label: 'Sospesi' },
];

export function QueuePage({ token, jobs, runs, loading, error, onRefresh, execute }: { token: string; jobs: QueueJob[] | null; runs: SearchRun[] | null; loading: boolean; error?: string; onRefresh: () => void; execute: ExecuteAction }) {
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [sort, setSort] = useState<Sort>('queue');
  const [showHistory, setShowHistory] = useState(false);
  const [expandedSeries, setExpandedSeries] = useState<Set<string>>(() => new Set());
  const [expandedSeasons, setExpandedSeasons] = useState<Set<string>>(() => new Set());
  const [selected, setSelected] = useState<QueueJob | null>(null);
  const [confirm, setConfirm] = useState<{ kind: 'empty' | 'remove'; job?: QueueJob } | null>(null);
  const [confirmBusy, setConfirmBusy] = useState(false);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const items = jobs || [];

  useEffect(() => {
    const interval = window.setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => window.clearInterval(interval);
  }, []);

  const filtered = useMemo(() => {
    let result = items.filter(job => {
      const matchesQuery = job.title.toLocaleLowerCase().includes(query.toLocaleLowerCase());
      const matchesFilter = filter === 'all' || (filter === 'waiting' ? ['queued', 'waiting_metadata', 'waiting_release'].includes(job.state) : job.state === filter);
      return matchesQuery && matchesFilter;
    });
    if (sort === 'priority') result = [...result].sort((a, b) => b.priority - a.priority || a.first_requested_at - b.first_requested_at);
    if (sort === 'title') result = [...result].sort((a, b) => a.title.localeCompare(b.title, 'it'));
    if (sort === 'oldest') result = [...result].sort((a, b) => a.first_requested_at - b.first_requested_at);
    if (sort === 'next') result = [...result].sort((a, b) => (a.next_attempt_at ?? a.eligible_at ?? Infinity) - (b.next_attempt_at ?? b.eligible_at ?? Infinity));
    return result;
  }, [items, query, filter, sort]);

  const entries = useMemo(() => makeEntries(filtered), [filtered]);
  const stateCounts = useMemo(() => items.reduce<Record<string, number>>((result, job) => { result[job.state] = (result[job.state] || 0) + 1; return result; }, {}), [items]);
  const toggle = (set: Set<string>, setter: (value: Set<string>) => void, key: string) => {
    const next = new Set(set);
    if (next.has(key)) next.delete(key); else next.add(key);
    setter(next);
  };

  const confirmAction = async () => {
    if (!confirm || confirmBusy) return;
    setConfirmBusy(true);
    try {
      if (confirm.kind === 'empty') await execute('queue/empty', 'queue/empty');
      else if (confirm.job) await execute(`remove-${confirm.job.id}`, `queue/${confirm.job.id}`, 'DELETE');
      setConfirm(null);
    } catch { /* The global action notice contains the error; keep the dialog open for context. */ }
    finally { setConfirmBusy(false); }
  };

  return <div className="page-stack queue-page" id="queue">
    <section className="page-intro"><div><p className="eyebrow">WORK CONTROL / GLOBAL SCHEDULER</p><h1>Queue<span>.</span></h1><p>Ogni riga è un'unità di ricerca. Gli episodi restano collegati alla propria serie.</p></div><Button variant="secondary" onClick={onRefresh}><RotateCcw size={15}/> Aggiorna dati</Button></section>

    <div className="queue-state-strip" aria-label="Conteggio elementi per stato">{[
      ['In coda', stateCounts.queued || 0, 'neutral'], ['Retry', stateCounts.retry_wait || 0, 'warning'], ['Attivi', stateCounts.running || 0, 'info'], ['Sospesi', stateCounts.suspended || 0, 'danger'],
    ].map(([label, value, tone]) => <div className="queue-state-metric" key={label}><span>{label}</span><StatusBadge tone={tone as 'neutral' | 'warning' | 'info' | 'danger'}>{String(value).padStart(2, '0')}</StatusBadge></div>)}</div>

    <div className="queue-view-tabs" role="group" aria-label="Vista della coda"><button aria-pressed={!showHistory} className={!showHistory ? 'active' : ''} onClick={() => setShowHistory(false)}>Manifest <small>{items.length}</small></button><button aria-pressed={showHistory} className={showHistory ? 'active' : ''} onClick={() => setShowHistory(true)}>Search history <small>{runs?.length || 0}</small></button></div>

    {showHistory ? <section className="surface-panel history-workspace"><SectionHeading index="02" eyebrow="ULTIME 500 OPERAZIONI / RETENTION 90 GIORNI" title="Search history" action={<Button onClick={onRefresh}><RotateCcw size={15}/> Aggiorna</Button>}/>{error && <InlineError message={error} retry={onRefresh}/>} {!runs?.length ? <EmptyState icon={RotateCcw} title="Nessuna ricerca nello storico"><span>Le operazioni concluse o riconciliate appariranno qui.</span></EmptyState> : <div className="history-list">{runs.map(run => <article className="history-row" key={run.id}><span className={`run-dot run-${run.state}`}/><div className="history-title"><b>{run.title || `Job ${run.job_id.slice(0, 8)}`}</b><small>{run.media_type ? mediaLabel(run.media_type) : 'Contenuto'} · {run.job_id.slice(0, 8).toUpperCase()}</small></div><StatusBadge tone={run.state === 'completed' || run.state === 'satisfied' ? 'good' : run.state === 'technical_error' ? 'danger' : run.state === 'no_results' ? 'warning' : 'info'}>{stateLabel(run.state)}</StatusBadge><time>{formatDate(run.started_at)}</time><span className="history-detail">{run.detail || '—'}</span></article>)}</div>}</section> : <section className="surface-panel queue-workspace"><SectionHeading index="01" eyebrow="ELEMENTI NON TERMINALI" title="Manifest" action={<span className="result-count">{filtered.length} DI {items.length}</span>}/>

      <div className="queue-controls"><label className="queue-search"><Search size={16}/><input value={query} onChange={event => setQuery(event.target.value)} placeholder="Cerca un titolo o una serie" aria-label="Cerca nella coda"/><kbd>/</kbd></label><label className="sort-control"><ListFilter size={15}/><span className="sr-only">Ordina la coda</span><select value={sort} onChange={event => setSort(event.target.value as Sort)}><option value="queue">Ordine coda</option><option value="priority">Priorità</option><option value="oldest">Richieste più vecchie</option><option value="next">Prossimo tentativo</option><option value="title">Titolo A–Z</option></select></label><Button variant="danger" disabled={!items.length} onClick={() => setConfirm({ kind: 'empty' })}><Trash2 size={15}/> Svuota non avviati</Button></div>
      <div className="queue-filter-bar" role="group" aria-label="Filtra coda per stato">{filters.map(item => <button key={item.id} className={filter === item.id ? 'queue-filter active' : 'queue-filter'} aria-pressed={filter === item.id} onClick={() => setFilter(item.id)}>{item.label}<small>{item.id === 'all' ? items.length : item.id === 'waiting' ? (stateCounts.queued || 0) + (stateCounts.waiting_metadata || 0) + (stateCounts.waiting_release || 0) : stateCounts[item.id] || 0}</small></button>)}</div>
      {error && <InlineError message={error} retry={onRefresh}/>}
      {loading && !jobs ? <Skeleton rows={6}/> : !filtered.length ? <EmptyState icon={query ? Search : Tv2} title={query ? 'Nessun risultato' : 'La coda è vuota'}><span>{query ? 'Modifica la ricerca o rimuovi i filtri.' : 'Le richieste Seerr approvate compariranno qui quando GXE le sincronizza.'}</span></EmptyState> : <div className="queue-table-wrap"><table className="queue-table"><thead><tr><th className="col-order">ORDINE*</th><th>RICHIESTA</th><th>STATO</th><th>PROSSIMA AZIONE</th><th>TENTATIVI</th><th><span className="sr-only">Azioni</span></th></tr></thead><tbody>
        {entries.map(entry => entry.kind === 'movie' ? <MovieRow key={entry.key} job={entry.jobs[0]} index={items.indexOf(entry.jobs[0]) + 1} now={now} onDetails={setSelected} onPriority={job => void execute(`priority-${job.id}`, `queue/${job.id}/priority`)} onRetry={job => void execute(`retry-${job.id}`, `queue/${job.id}/retry`)} onRemove={job => setConfirm({ kind: 'remove', job })}/> : <SeriesRows key={entry.key} entry={entry} now={now} expanded={expandedSeries} expandedSeasons={expandedSeasons} onToggleSeries={() => toggle(expandedSeries, setExpandedSeries, entry.key)} onToggleSeason={key => toggle(expandedSeasons, setExpandedSeasons, key)} onDetails={setSelected} onPriority={job => void execute(`priority-${job.id}`, `queue/${job.id}/priority`)} onRetry={job => void execute(`retry-${job.id}`, `queue/${job.id}/retry`)} onRemove={job => setConfirm({ kind: 'remove', job })}/>)}
      </tbody></table><p className="queue-order-note">* L'ordine mostrato segue priorità e prima richiesta; lo scheduler può scegliere un altro elemento eleggibile per rispettare retry, release e fairness.</p></div>}
    </section>}

    <QueueDetails token={token} selected={selected} onClose={() => setSelected(null)} execute={execute}/>
    <ConfirmDialog open={!!confirm} title={confirm?.kind === 'empty' ? 'Rimuovere i lavori non avviati?' : 'Rimuovere questo elemento?'} description={confirm?.kind === 'empty' ? 'Saranno rimossi tutti gli elementi non ancora avviati. Una ricerca già attiva continuerà.' : `“${confirm?.job?.title || ''}” verrà rimosso dalla coda se non è già iniziato.`} confirmLabel={confirm?.kind === 'empty' ? 'Svuota coda' : 'Rimuovi elemento'} danger busy={confirmBusy} onCancel={() => setConfirm(null)} onConfirm={() => void confirmAction()}/>
  </div>;
}

function makeEntries(jobs: QueueJob[]): Entry[] {
  const entries = new Map<string, Entry>();
  for (const job of jobs) {
    if (job.media_type === 'movie' || !job.series_key) {
      entries.set(`movie:${job.id}`, { kind: 'movie', key: job.id, label: job.title, jobs: [job] });
    } else {
      const key = `series:${job.series_key}`;
      const existing = entries.get(key);
      if (existing) existing.jobs.push(job);
      else entries.set(key, { kind: 'series', key, label: seriesName(job), jobs: [job] });
    }
  }
  return [...entries.values()];
}

function seriesName(job: QueueJob): string {
  const separator = job.title.indexOf(' — ');
  return separator > 0 ? job.title.slice(0, separator) : job.title.replace(/\s+S\d{2}E\d{2}$/, '');
}

function episodeParts(job: QueueJob) {
  const key = job.episode_sort_key?.match(/^(\d+)-(\d+)$/);
  const suffix = job.title.match(/S(\d{2})E(\d{2})$/);
  const season = key ? Number(key[1]) : suffix ? Number(suffix[1]) : 0;
  const episode = key ? Number(key[2]) : suffix ? Number(suffix[2]) : 0;
  const separator = job.title.indexOf(' — ');
  let title = separator > 0 ? job.title.slice(separator + 3) : job.title;
  title = title.replace(/\s+S\d{2}E\d{2}$/, '');
  return { season, episode, title, code: `S${String(season).padStart(2, '0')}E${String(episode).padStart(2, '0')}` };
}

function MovieRow({ job, index, now, onDetails, onPriority, onRetry, onRemove }: { job: QueueJob; index: number; now: number; onDetails: (job: QueueJob) => void; onPriority: (job: QueueJob) => void; onRetry: (job: QueueJob) => void; onRemove: (job: QueueJob) => void }) {
  return <tr className="queue-item-row movie-row"><td className="queue-order">{index > 0 ? String(index).padStart(2, '0') : '—'}</td><td className="queue-title-cell"><Film size={15}/><div><button className="title-link" onClick={() => onDetails(job)}>{job.title}</button><small>FILM · {job.id.slice(0, 8).toUpperCase()}</small></div></td><td><StatePill state={job.state}/></td><td className="next-action"><span>{nextAttemptLabel(job, now)}</span>{job.state === 'retry_wait' && job.next_attempt_at && <small>{formatDate(job.next_attempt_at)}</small>}</td><td className="attempt-cell">{job.no_result_attempts}</td><td><RowActions job={job} onDetails={onDetails} onPriority={onPriority} onRetry={onRetry} onRemove={onRemove}/></td></tr>;
}

function SeriesRows({ entry, now, expanded, expandedSeasons, onToggleSeries, onToggleSeason, onDetails, onPriority, onRetry, onRemove }: { entry: Entry; now: number; expanded: Set<string>; expandedSeasons: Set<string>; onToggleSeries: () => void; onToggleSeason: (key: string) => void; onDetails: (job: QueueJob) => void; onPriority: (job: QueueJob) => void; onRetry: (job: QueueJob) => void; onRemove: (job: QueueJob) => void }) {
  const seasons = new Map<number, QueueJob[]>();
  [...entry.jobs].sort((a, b) => (a.episode_sort_key || '').localeCompare(b.episode_sort_key || '')).forEach(job => {
    const season = episodeParts(job).season;
    seasons.set(season, [...(seasons.get(season) || []), job]);
  });
  const isOpen = expanded.has(entry.key);
  const running = entry.jobs.filter(job => job.state === 'running').length;
  return <>
    <tr className="series-heading-row"><td colSpan={6}><button className="series-toggle" aria-expanded={isOpen} onClick={onToggleSeries}>{isOpen ? <ChevronDown size={16}/> : <ChevronRight size={16}/>}<Tv2 size={17}/><span><b>{entry.label}</b><small>{entry.jobs.length} EPISODI · {running ? `${running} IN ELABORAZIONE` : `${entry.jobs.filter(job => job.state === 'retry_wait').length} RETRY`}</small></span><span className="series-counter">{entry.jobs.length.toString().padStart(2, '0')}</span></button></td></tr>
    {isOpen && [...seasons.entries()].sort((a, b) => a[0] - b[0]).flatMap(([season, seasonJobs]) => {
      const seasonKey = `${entry.key}:season:${season}`;
      const seasonOpen = expandedSeasons.has(seasonKey);
      return <SeasonGroup key={seasonKey} season={season} seasonKey={seasonKey} jobs={seasonJobs} open={seasonOpen} now={now} onToggle={() => onToggleSeason(seasonKey)} onDetails={onDetails} onPriority={onPriority} onRetry={onRetry} onRemove={onRemove}/>;
    })}
  </>;
}

function SeasonGroup({ season, seasonKey, jobs, open, now, onToggle, onDetails, onPriority, onRetry, onRemove }: { season: number; seasonKey: string; jobs: QueueJob[]; open: boolean; now: number; onToggle: () => void; onDetails: (job: QueueJob) => void; onPriority: (job: QueueJob) => void; onRetry: (job: QueueJob) => void; onRemove: (job: QueueJob) => void }) {
  return <><tr className="season-heading-row"><td colSpan={6}><button className="season-toggle" aria-expanded={open} aria-controls={`episodes-${seasonKey}`} onClick={onToggle}>{open ? <ChevronDown size={14}/> : <ChevronRight size={14}/>}Stagione {season.toString().padStart(2, '0')}<small>{jobs.length} episodi richiesti</small></button></td></tr>{open && jobs.map(job => <EpisodeRow key={job.id} job={job} season={season} now={now} onDetails={onDetails} onPriority={onPriority} onRetry={onRetry} onRemove={onRemove}/>)}</>;
}

function EpisodeRow({ job, season, now, onDetails, onPriority, onRetry, onRemove }: { job: QueueJob; season: number; now: number; onDetails: (job: QueueJob) => void; onPriority: (job: QueueJob) => void; onRetry: (job: QueueJob) => void; onRemove: (job: QueueJob) => void }) {
  const episode = episodeParts(job);
  return <tr className="queue-item-row episode-row"><td className="queue-order">{episode.code}</td><td className="queue-title-cell"><span className="episode-branch" aria-hidden="true">└</span><div><button className="title-link" onClick={() => onDetails(job)}>{episode.title || `Episodio ${episode.episode}`}</button><small>STAGIONE {season.toString().padStart(2, '0')} · EPISODIO {episode.episode.toString().padStart(2, '0')}</small></div></td><td><StatePill state={job.state}/></td><td className="next-action"><span>{nextAttemptLabel(job, now)}</span>{job.state === 'retry_wait' && job.next_attempt_at && <small>{formatDate(job.next_attempt_at)}</small>}</td><td className="attempt-cell">{job.no_result_attempts}</td><td><RowActions job={job} onDetails={onDetails} onPriority={onPriority} onRetry={onRetry} onRemove={onRemove}/></td></tr>;
}

function StatePill({ state }: { state: QueueState }) {
  const tone = state === 'running' ? 'info' : state === 'retry_wait' || state === 'waiting_release' ? 'warning' : state === 'suspended' ? 'danger' : state === 'completed' || state === 'satisfied' ? 'good' : 'neutral';
  return <StatusBadge tone={tone}>{stateLabel(state)}</StatusBadge>;
}

function RowActions({ job, onDetails, onPriority, onRetry, onRemove }: { job: QueueJob; onDetails: (job: QueueJob) => void; onPriority: (job: QueueJob) => void; onRetry: (job: QueueJob) => void; onRemove: (job: QueueJob) => void }) {
  const started = job.state === 'running';
  return <div className="queue-row-actions"><IconButton icon={ArrowUp} label={`Prioritizza ${job.title}`} onClick={() => onPriority(job)} disabled={started} title="Porta in cima alla priorità"/><IconButton icon={RotateCcw} label={`Riprova ${job.title} ora`} onClick={() => onRetry(job)} disabled={started || !['queued', 'retry_wait', 'suspended'].includes(job.state)}/><IconButton icon={Trash2} label={`Rimuovi ${job.title}`} onClick={() => onRemove(job)} disabled={started}/><button className="details-link" onClick={() => onDetails(job)}>Dettagli</button></div>;
}

function QueueDetails({ token, selected, onClose, execute }: { token: string; selected: QueueJob | null; onClose: () => void; execute: ExecuteAction }) {
  const [details, setDetails] = useState<QueueJobDetails | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    if (!selected) { setDetails(null); setError(''); return; }
    let mounted = true;
    setLoading(true);
    setError('');
    apiRequest<QueueJobDetails>(token, `queue/${selected.id}/details`).then(value => { if (mounted) setDetails(value); }).catch(reason => { if (mounted) setError(reason instanceof Error ? reason.message : 'Dettagli non disponibili.'); }).finally(() => { if (mounted) setLoading(false); });
    return () => { mounted = false; };
  }, [selected, token]);
  const job = details?.job || selected;
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => { const id = window.setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000); return () => window.clearInterval(id); }, []);
  return <Drawer open={!!selected} title={job?.title || 'Dettagli elemento'} eyebrow={job ? `${mediaLabel(job.media_type)} / ${job.id.slice(0, 8).toUpperCase()}` : 'DETTAGLI CODA'} onClose={onClose} className="job-drawer">
    {loading && !details ? <Skeleton rows={6}/> : error ? <InlineError message={error} retry={() => selected && void apiRequest<QueueJobDetails>(token, `queue/${selected.id}/details`).then(setDetails).catch(() => setError('Nuovo tentativo non riuscito.'))}/> : job && <>
      <div className="drawer-status"><StatePill state={job.state}/><span>Prossima azione: {nextAttemptLabel(job, now)}</span></div><p className="job-description">{describeState(job.state)}</p>
      <dl className="detail-grid"><div><dt>Priorità</dt><dd>{job.priority > 0 ? `Priorità ${job.priority}` : 'Standard'}</dd></div><div><dt>Richiesta iniziale</dt><dd>{formatDate(job.first_requested_at)}</dd></div><div><dt>Retry senza risultati</dt><dd>{job.no_result_attempts}</dd></div><div><dt>Eleggibile dal</dt><dd>{formatDate(job.eligible_at)}</dd></div><div><dt>Prossimo tentativo</dt><dd>{formatDate(job.next_attempt_at)}</dd></div><div><dt>Riferimento media</dt><dd>{job.media_key}</dd></div></dl>
      {details?.requests?.length ? <section className="drawer-section"><h3>Richiedenti</h3>{details.requests.map(request => <div className="requester-row" key={request.external_id}><b>{request.requester_name}</b><small>{request.media_type.toUpperCase()} · {formatDate(request.created_at)}</small></div>)}</section> : <section className="drawer-section"><h3>Richiedenti</h3><p className="quiet-copy">Nessun richiedente attivo collegato.</p></section>}
      <section className="drawer-section"><h3>Storico tentativi</h3>{details?.runs.length ? <div className="attempt-list">{details.runs.map(run => <article className="attempt-row" key={run.id}><span className={`attempt-marker marker-${run.state}`}/><div><b>{stateLabel(run.state)}</b><small>{formatDate(run.started_at)}{run.completed_at ? ` · terminato ${formatDate(run.completed_at)}` : ' · in corso'}</small>{run.detail && <p>{run.detail}</p>}</div></article>)}</div> : <p className="quiet-copy">Non risultano ricerche avviate per questo elemento.</p>}</section>
      {['submitting', 'active', 'reconciling'].includes(details?.runs[0]?.state || '') && <ReconcileControl jobId={details!.runs[0].id} execute={execute}/>}
    </>}
  </Drawer>;
}

function ReconcileControl({ jobId, execute }: { jobId: string; execute: ExecuteAction }) {
  const [outcome, setOutcome] = useState('');
  const [detail, setDetail] = useState('');
  return <form className="reconcile-form" onSubmit={event => { event.preventDefault(); if (!outcome) return; void execute(`reconcile-${jobId}`, `runs/${jobId}/reconcile`, 'POST', { outcome, detail: detail || undefined }); }}><h3>Riconcilia operazione</h3><p>Usa lo stato effettivo in Radarr/Sonarr. GXE non ripeterà automaticamente un comando ambiguo.</p><label>Esito<select required value={outcome} onChange={event => setOutcome(event.target.value)}><option value="">Seleziona l'esito</option><option value="found">Trovato / aggiunto al download</option><option value="no_results">Nessun risultato</option><option value="satisfied">Già soddisfatto</option><option value="technical_error">Errore tecnico</option></select></label><label>Nota (facoltativa)<input value={detail} onChange={event => setDetail(event.target.value)} placeholder="Contesto per lo storico"/></label><Button variant="primary" type="submit" disabled={!outcome}>Registra esito</Button></form>;
}
