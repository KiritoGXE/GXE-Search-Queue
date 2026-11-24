import { useEffect, useState } from 'react';
import { Activity, AlertTriangle, ArrowRight, CheckCircle2, CircleHelp, Clock3, Film, Pause, Play, Radio, ServerCog, ShieldCheck, Tv2, WifiOff } from 'lucide-react';
import { deriveSystemState, formatDate, formatDuration, mediaLabel, stateLabel } from '../../api/presentation';
import { Button, EmptyState, InlineError, SectionHeading, Skeleton, StatusBadge } from '../../components/primitives';
import type { DashboardEvent, QueueJob, SearchRun, ServiceHealth, StatusResponse } from '../../types/api';
import type { ExecuteAction } from '../queue/QueuePage';

export function OverviewPage({ status, jobs, runs, events, statusError, queueError, loading, lastUpdated, onQueue, onAcknowledge, onGoQueue, onGoIndexers, onRefresh }: { status: StatusResponse | null; jobs: QueueJob[] | null; runs: SearchRun[] | null; events: DashboardEvent[] | null; statusError?: string; queueError?: string; loading: boolean; lastUpdated: number | null; onQueue: ExecuteAction; onAcknowledge: (eventId: string) => Promise<unknown>; onGoQueue: () => void; onGoIndexers: () => void; onRefresh: () => void }) {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    if (!status?.active_search) return;
    const id = window.setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => window.clearInterval(id);
  }, [!!status?.active_search]);

  const services = status?.services || [];
  const systemState = deriveSystemState(status ? services : null, loading, statusError || null);
  const counts = status?.counts || {};
  const queueItems = jobs || [];
  const retries = queueItems.filter(job => job.state === 'retry_wait').length;
  const pendingAlerts = (events || []).filter(event => !event.acknowledged_at);
  const active = status?.active_search;
  const systemCopy: Record<string, string> = {
    online: 'Servizi richiesti raggiungibili', degraded: 'Alcuni servizi richiedono attenzione', offline: 'Servizi di ricerca non raggiungibili', unknown: loading ? 'Sincronizzazione in corso' : 'Stato non disponibile',
  };

  return <div className="page-stack overview-page" id="overview">
    <section className="page-intro overview-intro"><div><p className="eyebrow">GXE / CONTROL ROOM</p><h1>Overview<span>.</span></h1><p>Stato operativo della coda e delle integrazioni.</p></div><div className="updated-meta"><span>ULTIMO AGGIORNAMENTO</span><b>{lastUpdated ? new Date(lastUpdated).toLocaleTimeString('it-IT', { hour: '2-digit', minute: '2-digit', second: '2-digit' }) : '—'}</b><button onClick={onRefresh} disabled={loading}>Aggiorna ora</button></div></section>
    {statusError && <InlineError message={statusError} retry={onRefresh}/>}
    {queueError && <InlineError message={queueError} retry={onRefresh}/>}

    <section className={`system-banner system-${systemState}`} aria-live="polite"><div className="system-state-icon">{systemState === 'online' ? <ShieldCheck size={23}/> : systemState === 'offline' ? <WifiOff size={23}/> : systemState === 'unknown' ? <CircleHelp size={23}/> : <AlertTriangle size={23}/>}</div><div><span className="eyebrow">SYSTEM STATUS</span><h2>{systemState === 'online' ? 'Online' : systemState === 'degraded' ? 'Degraded' : systemState === 'offline' ? 'Offline' : 'Unknown'}</h2><p>{systemCopy[systemState]}</p></div><div className="system-state-basis"><span>VALUTATO DA</span><b>Radarr · Sonarr · Prowlarr · Seerr</b><small>Disponibilità API più recente salvata dal worker</small></div></section>

    <section className="overview-metrics" aria-label="Metriche coda"><Metric icon={Activity} label="Elementi attivi" value={String((counts.movies || 0) + (counts.episodes || 0))} detail={`${counts.movies || 0} film · ${counts.episodes || 0} episodi`} tone="orange"/><Metric icon={Clock3} label="In retry" value={String(retries)} detail="Elementi in attesa del prossimo tentativo" tone="cyan"/><Metric icon={CheckCircle2} label="Ricerche completate" value={String(status?.outcomes_90d.completed || 0)} detail="Ultimi 90 giorni" tone="green"/><Metric icon={AlertTriangle} label="Senza risultati" value={String(status?.outcomes_90d.no_results || 0)} detail={`${status?.outcomes_90d.technical_errors || 0} errori tecnici · 90 giorni`} tone="warning"/></section>

    <section className="overview-primary-grid">
      <article className={`active-operation ${active ? 'operation-active' : ''}`}><header><div><span className="eyebrow">CURRENT OPERATION / 01</span><h2>{active ? 'Ricerca attiva' : 'Scheduler'}</h2></div><StatusBadge tone={active ? 'info' : status?.paused ? 'warning' : 'neutral'} icon={active ? Activity : status?.paused ? Pause : Clock3}>{active ? stateLabel(active.state) : status?.paused ? 'In pausa' : 'In attesa'}</StatusBadge></header>
        {loading && !status ? <Skeleton rows={3}/> : active ? <div className="active-operation-content"><div className="operation-orbit" aria-hidden="true"><span/><i/></div><div className="operation-data"><p className="eyebrow">{mediaLabel(active.media_type)} · JOB {active.job_id.slice(0, 8).toUpperCase()}</p><h3>{active.title || `Elemento ${active.job_id.slice(0, 8)}`}</h3><p>La ricerca esterna è in corso. Il download non occupa lo slot dello scheduler dopo il completamento della ricerca.</p><div className="operation-meta"><span><Clock3 size={14}/> Avviata {formatDate(active.started_at)}</span><b>{formatDuration(now - active.started_at)}</b></div></div></div> : <div className="scheduler-idle"><div className="idle-symbol"><Radio size={22}/></div><div><b>{status?.paused ? 'La coda è sospesa' : queueItems.length ? 'Nessuna ricerca GXE attiva' : 'In attesa di richieste'}</b><p>{status?.paused ? 'Il lavoro resta salvato. Riprendi la coda per autorizzare nuove ricerche.' : queueItems.length ? 'Lo scheduler attende intervallo minimo, eleggibilità e disponibilità dei servizi.' : 'Le richieste approvate in Seerr appariranno dopo la sincronizzazione.'}</p></div></div>}
        <footer><span><span className={active ? 'signal-led active' : 'signal-led'}/>{active ? 'SLOT OCCUPATO' : 'SCHEDULER GLOBALE'}</span><span>INTERVALLO MINIMO · {Math.round((Number(status?.counts?.minimum_interval_seconds) || 600) / 60)} MIN</span></footer>
      </article>
      <article className="service-panel"><SectionHeading index="02" eyebrow="HEALTH CHECK / 30S" title="Servizi" action={<button className="panel-link" onClick={onGoIndexers}>Indexers <ArrowRight size={14}/></button>}/>{services.length ? <div className="service-list">{['seerr', 'radarr', 'sonarr', 'prowlarr'].map(name => { const service = services.find(value => value.service === name); return <ServiceRow key={name} service={service}/>; })}</div> : loading ? <Skeleton rows={4}/> : <EmptyState icon={ServerCog} title="Stato non ricevuto"><span>Il worker non ha ancora completato il controllo dei servizi.</span></EmptyState>}<div className="service-footnote"><span>{status?.active_cooldowns || 0} cooldown GXE registrati</span><small>Non equivale al conteggio dei cooldown Prowlarr.</small></div></article>
    </section>

    <section className="overview-secondary-grid">
      <article className="surface-panel alerts-panel"><SectionHeading index="03" eyebrow="ACTION REQUIRED" title="Avvisi" action={<span className="alert-total">{pendingAlerts.length.toString().padStart(2, '0')} APERTI</span>}/>{pendingAlerts.length ? <div className="overview-alert-list">{pendingAlerts.slice(0, 4).map(event => <article className={`overview-alert severity-${event.severity}`} key={event.id}><span className="alert-symbol"><AlertTriangle size={15}/></span><div><b>{event.message}</b><small>{formatDate(event.created_at)}</small></div><button onClick={() => void onAcknowledge(event.id)}>Segna come letto</button></article>)}</div> : <div className="all-clear"><CheckCircle2 size={18}/><div><b>Nessun intervento richiesto</b><small>Gli avvisi del sistema appariranno qui.</small></div></div>}</article>
      <article className="surface-panel recent-panel"><SectionHeading index="04" eyebrow="SEARCH RUNS / 90-DAY RETENTION" title="Attività recente" action={<button className="panel-link" onClick={onGoQueue}>Apri coda <ArrowRight size={14}/></button>}/>{runs?.length ? <div className="recent-list">{runs.slice(0, 5).map(run => { const job = queueItems.find(item => item.id === run.job_id); return <div className="recent-run" key={run.id}><span className={`run-dot run-${run.state}`}/><div><b>{run.title || job?.title || `Job ${run.job_id.slice(0, 8)}`}</b><small>{stateLabel(run.state)} · {formatDate(run.started_at)}</small></div><span className="run-kind">{run.media_type ? mediaLabel(run.media_type) : job ? mediaLabel(job.media_type) : run.id.slice(0, 7)}</span></div>; })}</div> : loading ? <Skeleton rows={3}/> : <EmptyState icon={Activity} title="Nessuna ricerca registrata"><span>Le ricerche avviate compariranno nello storico.</span></EmptyState>}</article>
    </section>
  </div>;
}

function Metric({ icon: Icon, label, value, detail, tone }: { icon: typeof Activity; label: string; value: string; detail: string; tone: string }) {
  return <article className={`overview-metric metric-${tone}`}><div className="metric-top"><span>{label}</span><Icon size={17}/></div><strong>{value}</strong><small>{detail}</small></article>;
}

function ServiceRow({ service }: { service?: ServiceHealth }) {
  const name = service?.service || 'Servizio';
  const tone = !service ? 'neutral' : service.available ? 'good' : 'danger';
  return <div className="service-row"><span className={`service-led led-${tone}`}/><div><b>{name[0].toUpperCase() + name.slice(1)}</b><small>{service?.checked_at ? `Controllato ${formatDate(service.checked_at)}` : 'Mai controllato'}</small></div><StatusBadge tone={tone}>{!service ? 'Sconosciuto' : service.available ? 'Disponibile' : 'Non raggiungibile'}</StatusBadge>{!service?.available && service?.detail && <span className="sr-only">{service.detail}</span>}</div>;
}
