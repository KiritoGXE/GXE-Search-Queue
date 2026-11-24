import { useCallback, useEffect, useState } from 'react';
import { Activity, AlertTriangle, Bell, Boxes, ChevronRight, CircleUserRound, Command, LayoutDashboard, LogOut, Pause, Play, Settings2, ShieldAlert, TimerReset, X } from 'lucide-react';
import { deriveSystemState } from '../api/presentation';
import { Button, Drawer, IconButton, StatusBadge, Toast } from '../components/primitives';
import { useDashboard } from '../hooks/useDashboard';
import { IndexersPage } from '../features/indexers/IndexersPage';
import { OverviewPage } from '../features/overview/OverviewPage';
import { QueuePage } from '../features/queue/QueuePage';
import { SettingsPage } from '../features/settings/SettingsPage';
import type { DashboardEvent } from '../types/api';

type Section = 'overview' | 'queue' | 'indexers' | 'settings';
const navigation: { id: Section; label: string; subtitle: string; icon: typeof LayoutDashboard; code: string }[] = [
  { id: 'overview', label: 'Overview', subtitle: 'Stato sistema', icon: LayoutDashboard, code: '01' },
  { id: 'queue', label: 'Queue', subtitle: 'Lavori e retry', icon: Boxes, code: '02' },
  { id: 'indexers', label: 'Indexers', subtitle: 'Telemetria Prowlarr', icon: Activity, code: '03' },
  { id: 'settings', label: 'Settings', subtitle: 'Scheduler GXE', icon: Settings2, code: '04' },
];

function readToken() { try { return sessionStorage.getItem('gxe-token') || ''; } catch { return ''; } }
function readSection(): Section {
  const section = window.location.hash.replace('#', '');
  return navigation.some(item => item.id === section) ? section as Section : 'overview';
}

export function App() {
  const [token, setToken] = useState(readToken);
  const [section, setSection] = useState<Section>(readSection);
  const [notificationsOpen, setNotificationsOpen] = useState(false);
  const [showLogout, setShowLogout] = useState(false);
  const unauthorized = useCallback(() => { sessionStorage.removeItem('gxe-token'); setToken(''); }, []);
  const dashboard = useDashboard(token, unauthorized);

  useEffect(() => {
    const onHash = () => setSection(readSection());
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);

  useEffect(() => {
    if (!token) return;
    window.history.replaceState(null, '', `#${section}`);
  }, [token]);

  const navigate = (target: Section) => {
    if (target === section) {
      document.querySelector<HTMLElement>('.page-content')?.scrollTo({ top: 0, behavior: 'smooth' });
      return;
    }
    window.history.pushState(null, '', `#${target}`);
    setSection(target);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  useEffect(() => {
    if (!token) return;
    const heading = document.querySelector<HTMLElement>('.page-content h1');
    if (heading) {
      heading.tabIndex = -1;
      heading.focus({ preventScroll: true });
    }
  }, [section, token]);

  const pendingEvents = (dashboard.events || []).filter(event => !event.acknowledged_at);
  const systemState = deriveSystemState(dashboard.status?.services || null, dashboard.refreshing, dashboard.errors.status || null);
  const currentSection = navigation.find(item => item.id === section)!;
  const execute = dashboard.execute;

  const logout = () => {
    sessionStorage.removeItem('gxe-token');
    setToken('');
    setShowLogout(false);
  };

  if (!token) return <LoginScreen onAuthenticated={value => { sessionStorage.setItem('gxe-token', value); setToken(value); }} />;

  return <div className="console-shell">
    <aside className="sidebar">
      <a className="brand" href="#overview" onClick={event => { event.preventDefault(); navigate('overview'); }} aria-label="GXE Search Queue — overview"><span className="brand-symbol"><i/><b>G</b><em>×</em></span><span className="brand-wordmark">GXE<small>SEARCH QUEUE</small></span></a>
      <div className="sidebar-label">OPERATIONS</div>
      <nav className="main-nav" aria-label="Navigazione principale">{navigation.map(item => { const Icon = item.icon; const active = section === item.id; return <a key={item.id} href={`#${item.id}`} className={active ? 'nav-link active' : 'nav-link'} aria-label={item.label} aria-current={active ? 'page' : undefined} onClick={event => { event.preventDefault(); navigate(item.id); }}><span className="nav-link-code">{item.code}</span><Icon size={17} strokeWidth={1.8}/><span className="nav-link-copy"><b>{item.label}</b><small>{item.subtitle}</small></span>{item.id === 'queue' && (dashboard.jobs?.length || 0) > 0 && <span className="nav-badge">{dashboard.jobs?.length}</span>}{active && <ChevronRight className="nav-chevron" size={14}/>}</a>; })}</nav>
      <div className="sidebar-spacer"/>
      <div className="sidebar-node"><span className={`node-mark node-${systemState}`}>{systemState === 'online' ? <Activity size={16}/> : systemState === 'offline' ? <AlertTriangle size={16}/> : <TimerReset size={16}/>}</span><div><b>{systemState === 'online' ? 'NODE ONLINE' : systemState === 'degraded' ? 'NODE DEGRADED' : systemState === 'offline' ? 'NODE OFFLINE' : 'NODE UNKNOWN'}</b><small>CONTROL / LOCAL</small></div><span className={`node-pulse pulse-${systemState}`}/></div>
      <div className="sidebar-profile"><span className="profile-icon"><CircleUserRound size={17}/></span><div><b>Amministratore</b><small>ACCESSO LOCALE</small></div><button aria-label="Esci dall'account" title="Esci" onClick={() => setShowLogout(true)}><LogOut size={16}/></button></div>
      <div className="sidebar-build"><span>GXE REPUBLIC</span><span>PREVIEW 0.1</span></div>
    </aside>

    <div className="console-main">
      <header className="console-header"><div className="console-breadcrumb"><span>GXE</span><ChevronRight size={13}/><b>{currentSection.label.toUpperCase()}</b></div><div className="header-actions"><div className="header-system"><span className={`header-system-dot dot-${systemState}`}/><span>{systemState.toUpperCase()}</span><small>{dashboard.refreshing ? 'SYNC…' : 'SYNC 10S'}</small></div><IconButton icon={Bell} label="Apri avvisi" className="notification-button" onClick={() => setNotificationsOpen(true)}><span className="notification-count">{pendingEvents.length}</span></IconButton><div className="header-divider"/><Button variant="primary" className="header-pause" disabled={dashboard.pendingAction === 'queue/pause' || !dashboard.status} onClick={() => void execute('queue/pause', 'queue/pause', 'POST', { paused: !dashboard.status?.paused })}>{dashboard.status?.paused ? <Play size={15}/> : <Pause size={15}/>}<span>{dashboard.status?.paused ? 'Riprendi coda' : 'Pausa coda'}</span></Button></div></header>

      <main className="page-content" key={section}>
        <div className="page-content-inner">
          {section === 'overview' && <OverviewPage status={dashboard.status} jobs={dashboard.jobs} runs={dashboard.runs} events={dashboard.events} statusError={dashboard.errors.status} queueError={dashboard.errors.queue} loading={dashboard.refreshing && !dashboard.status} lastUpdated={dashboard.lastUpdated} onQueue={execute} onAcknowledge={eventId => execute(`ack-${eventId}`, `events/${eventId}/ack`)} onGoQueue={() => navigate('queue')} onGoIndexers={() => navigate('indexers')} onRefresh={() => void dashboard.refresh()}/>}
          {section === 'queue' && <QueuePage token={token} jobs={dashboard.jobs} runs={dashboard.runs} loading={dashboard.refreshing} error={dashboard.errors.queue} onRefresh={() => void dashboard.refresh()} execute={execute}/>}
          {section === 'indexers' && <IndexersPage token={token} prowlarr={dashboard.status?.services.find(service => service.service === 'prowlarr')} cooldowns={dashboard.cooldowns} onRefresh={() => void dashboard.refresh()}/>}
          {section === 'settings' && <SettingsPage settings={dashboard.settings} loading={!dashboard.settings && dashboard.refreshing} error={dashboard.errors.settings} pending={dashboard.pendingAction === 'settings'} onRefresh={() => void dashboard.refresh()} execute={execute}/>}
        </div>
      </main>
    </div>

    {dashboard.notice && <Toast kind={dashboard.notice.kind} message={dashboard.notice.message} onDismiss={dashboard.clearNotice}/>}
    <Drawer open={notificationsOpen} title="Avvisi di sistema" eyebrow={`${pendingEvents.length} DA VERIFICARE`} onClose={() => setNotificationsOpen(false)} className="notifications-drawer"><NotificationList events={dashboard.events} onAcknowledge={eventId => execute(`ack-${eventId}`, `events/${eventId}/ack`)}/></Drawer>
    <ConfirmLogout open={showLogout} onCancel={() => setShowLogout(false)} onConfirm={logout}/>
  </div>;
}

function LoginScreen({ onAuthenticated }: { onAuthenticated: (token: string) => void }) {
  const [value, setValue] = useState('');
  const [visible, setVisible] = useState(false);
  const [error, setError] = useState('');
  const submit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError('');
    try {
      const response = await fetch('/api/v1/status', { headers: { Authorization: `Bearer ${value}` } });
      if (response.status === 401) { setError('Token non valido. Controlla il valore configurato per questa istanza.'); return; }
      if (!response.ok) { setError('GXE non è riuscito a verificare l’accesso. Riprova tra poco.'); return; }
      onAuthenticated(value);
    } catch { setError('GXE non è raggiungibile. Controlla la connessione al servizio.'); }
  };
  return <main className="login-screen"><div className="login-visual" aria-hidden="true"><div className="login-grid"/><div className="login-orbit orbit-outer"/><div className="login-orbit orbit-inner"/><div className="login-emblem"><span>G</span><i>×</i></div><div className="visual-coordinate">GXE / LOCAL OPERATIONS NODE</div><div className="visual-index">TC / 04—01</div></div><form className="login-form" onSubmit={submit}><a className="brand login-brand" href="#" aria-label="GXE Search Queue"><span className="brand-symbol"><i/><b>G</b><em>×</em></span><span className="brand-wordmark">GXE<small>SEARCH QUEUE</small></span></a><p className="eyebrow login-eyebrow">AUTHENTICATE / ADMINISTRATOR ACCESS</p><h1>Control<br/><span>the queue.</span></h1><p className="login-description">Accedi alla console operativa della tua istanza GXE.</p><label htmlFor="admin-token">Token amministratore</label><div className="token-input"><input id="admin-token" type={visible ? 'text' : 'password'} value={value} onChange={event => setValue(event.target.value)} autoComplete="current-password" autoFocus required/><button type="button" onClick={() => setVisible(!visible)}>{visible ? 'NASCONDI' : 'MOSTRA'}</button></div>{error && <p className="login-error" role="alert"><AlertTriangle size={15}/>{error}</p>}<Button variant="primary" type="submit" className="login-submit">Apri la console <ChevronRight size={17}/></Button><div className="login-footer"><span>ACCESSO PROTETTO</span><span>PREVIEW 0.1.0</span></div></form></main>;
}

function NotificationList({ events, onAcknowledge }: { events: DashboardEvent[] | null; onAcknowledge: (id: string) => Promise<unknown> }) {
  if (!events?.length) return <div className="drawer-empty"><ShieldAlert size={22}/><b>Nessun avviso registrato</b><span>Gli eventi significativi appariranno qui.</span></div>;
  return <div className="notification-list">{events.map(event => <article className={`notification-item severity-${event.severity} ${event.acknowledged_at ? 'is-acknowledged' : ''}`} key={event.id}><div className="notification-marker"><AlertTriangle size={15}/></div><div className="notification-copy"><span className="notification-kind">{event.kind.replaceAll('_', ' ')}</span><b>{event.message}</b><time>{new Date(event.created_at * 1000).toLocaleString('it-IT')}</time>{!event.acknowledged_at && <button onClick={() => void onAcknowledge(event.id)}>Segna come letto</button>}{!!event.acknowledged_at && <small>Letto</small>}</div></article>)}</div>;
}

function ConfirmLogout({ open, onCancel, onConfirm }: { open: boolean; onCancel: () => void; onConfirm: () => void }) {
  return <Drawer open={open} title="Terminare la sessione?" eyebrow="ACCESSO AMMINISTRATORE" onClose={onCancel} className="logout-drawer"><p>Il token verrà rimosso dalla sessione di questo browser. Potrai accedere di nuovo in qualsiasi momento.</p><div className="confirm-actions"><Button onClick={onCancel}>Resta nella console</Button><Button variant="danger" onClick={onConfirm}><LogOut size={15}/> Esci</Button></div></Drawer>;
}
