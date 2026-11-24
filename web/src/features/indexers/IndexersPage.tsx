import { useCallback, useEffect, useState } from 'react';
import { Activity, AlertTriangle, Clock3, Database, ExternalLink, RefreshCw, ShieldAlert, Signal, SignalZero } from 'lucide-react';
import { apiRequest } from '../../api/client';
import { formatDate } from '../../api/presentation';
import { Button, EmptyState, InlineError, SectionHeading, Skeleton, StatusBadge } from '../../components/primitives';
import type { Cooldown, IndexerOverview, ServiceHealth } from '../../types/api';

export function IndexersPage({ token, prowlarr, cooldowns, onRefresh }: { token: string; prowlarr?: ServiceHealth; cooldowns: Cooldown[] | null; onRefresh: () => void }) {
  const [overview, setOverview] = useState<IndexerOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState('');
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

  const refresh = useCallback(async (manual = false) => {
    if (manual) setRefreshing(true);
    try {
      const next = await apiRequest<IndexerOverview>(token, 'indexers');
      setOverview(next);
      setError('');
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Dati indexer non disponibili.');
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [token]);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), 60_000);
    return () => window.clearInterval(interval);
  }, [refresh]);
  useEffect(() => {
    const interval = window.setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => window.clearInterval(interval);
  }, []);

  const rows = overview?.indexers || [];
  const enabled = rows.filter(row => row.enabled).length;
  const prowlarrCooldowns = rows.filter(row => row.disabled_until && Date.parse(row.disabled_until) / 1000 > now).length;
  const failures = rows.reduce((sum, row) => sum + row.failed_queries_24h, 0);

  return <div className="page-stack page-indexers">
    <section className="page-intro"><div><p className="eyebrow">SIGNAL / INDEXER TELEMETRY</p><h1>Indexers<span>.</span></h1><p>Stato osservato da Prowlarr e dati aggregati sulle query recenti.</p></div><div className="intro-meta"><span>FINESTRA STATISTICHE</span><b>ULTIME 24 ORE</b></div></section>

    <div className="indexer-safety" role="note"><ShieldAlert size={18}/><p><b>Telemetria, non garanzia di rate limit.</b><span>Il conteggio degli errori Prowlarr non identifica necessariamente HTTP 429. La serializzazione delle ricerche GXE non dimostra una distanza minima per singolo indexer.</span></p><span className="safety-code">VALIDAZIONE / APERTA</span></div>

    {prowlarr && !prowlarr.available && <InlineError message={prowlarr.detail || 'Prowlarr non risulta raggiungibile dal worker GXE.'} retry={onRefresh}/>}
    {error && <InlineError message={error} retry={() => void refresh(true)}/>}

    <div className="indexer-summary-grid">
      <article className="indexer-summary"><div><span>INDEXER CONFIGURATI</span><Database size={17}/></div>{loading && !overview ? <Skeleton rows={1}/> : <strong>{rows.length.toString().padStart(2, '0')}</strong>}<small>Elenco in sola lettura da Prowlarr</small></article>
      <article className="indexer-summary"><div><span>ABILITATI IN PROWLARR</span><Signal size={17}/></div>{loading && !overview ? <Skeleton rows={1}/> : <strong>{enabled.toString().padStart(2, '0')}<small> / {rows.length}</small></strong>}<small>Configurazione abilitata, non test di raggiungibilità</small></article>
      <article className="indexer-summary"><div><span>COOLDOWN PROWLARR</span><Clock3 size={17}/></div>{loading && !overview ? <Skeleton rows={1}/> : <strong>{prowlarrCooldowns.toString().padStart(2, '0')}</strong>}<small>Disabled-until riportato da Prowlarr</small></article>
      <article className="indexer-summary"><div><span>QUERY FALLITE · 24H</span><AlertTriangle size={17}/></div>{loading && !overview ? <Skeleton rows={1}/> : <strong>{failures.toString().padStart(2, '0')}</strong>}<small>Categoria aggregata, non classificata come 429</small></article>
    </div>

    <section className="surface-panel indexer-list-panel">
      <SectionHeading index="01" eyebrow="PROWLARR / SOLA LETTURA" title="Indexer registrati" action={<Button onClick={() => { void refresh(true); onRefresh(); }} disabled={refreshing}><RefreshCw size={15} className={refreshing ? 'spin' : ''}/> Aggiorna</Button>}/>
      {loading && !overview ? <Skeleton rows={5}/> : rows.length ? <div className="indexer-list">{rows.map(indexer => {
        const until = indexer.disabled_until ? Math.floor(Date.parse(indexer.disabled_until) / 1000) : 0;
        const remaining = until - now;
        const cooldown = remaining > 0;
        return <article className="indexer-row" key={indexer.id}>
          <div className="indexer-identity"><span className={cooldown ? 'indexer-symbol symbol-warning' : indexer.enabled ? 'indexer-symbol symbol-enabled' : 'indexer-symbol symbol-disabled'}>{cooldown ? <Clock3 size={17}/> : indexer.enabled ? <Signal size={17}/> : <SignalZero size={17}/>}</span><div><b>{indexer.name}</b><small>{indexer.protocol || 'Protocollo non indicato'}{indexer.privacy ? ` · ${indexer.privacy}` : ''} · ID {indexer.id}</small></div></div>
          <div className="indexer-state-cell"><StatusBadge tone={cooldown ? 'warning' : indexer.enabled ? 'good' : 'neutral'}>{cooldown ? 'Cooldown Prowlarr' : indexer.enabled ? 'Abilitato' : 'Disabilitato'}</StatusBadge>{cooldown && <small>Fino a {formatDate(until)} · {Math.floor(remaining / 60)}m {String(remaining % 60).padStart(2, '0')}s</small>}</div>
          <div className="indexer-stat"><span>QUERY / 24H</span><b>{indexer.queries_24h}</b></div>
          <div className="indexer-stat"><span>ERRORI QUERY</span><b className={indexer.failed_queries_24h ? 'value-warning' : ''}>{indexer.failed_queries_24h}</b></div>
          <div className="indexer-stat"><span>GRAB</span><b>{indexer.grabs_24h}</b></div>
          <div className="indexer-stat"><span>MEDIA RISPOSTA · 24H</span><b>{indexer.average_response_time_ms_24h ? `${indexer.average_response_time_ms_24h} ms` : '—'}</b></div>
          <div className="indexer-last-failure"><span>ULTIMO ERRORE</span><b>{indexer.most_recent_failure ? formatDate(Date.parse(indexer.most_recent_failure) / 1000) : 'Nessuno segnalato'}</b></div>
        </article>;
      })}</div> : <EmptyState icon={Database} title="Nessun indexer ricevuto"><span>Controlla che Prowlarr sia collegato e che l'API sia accessibile.</span></EmptyState>}
      <div className="panel-footnote"><span><Activity size={14}/> Dati Prowlarr · {overview ? formatDate(overview.checked_at) : 'non ancora sincronizzati'}</span><span><ExternalLink size={13}/> Nessuna configurazione esterna viene modificata da questa vista.</span></div>
    </section>

    <section className="surface-panel tracked-cooldowns"><SectionHeading index="02" eyebrow="GXE / STATO INTERNO" title="Cooldown tracciati"/>
      {cooldowns === null ? <Skeleton rows={2}/> : cooldowns.length ? <div className="cooldown-list">{cooldowns.map(item => <article className="cooldown-row" key={item.id}><Clock3 size={16}/><div><b>{item.name}</b><small>{item.reason}</small></div><time>{formatDate(item.cooldown_until)}</time></article>)}</div> : <div className="cooldown-none"><CheckMark/> <span>Nessun cooldown GXE attivo registrato.</span><small>La raccolta automatica degli eventi HTTP 429 non è implementata in questa preview.</small></div>}
    </section>
  </div>;
}

function CheckMark() { return <span className="check-mark" aria-hidden="true">✓</span>; }
