import { useEffect, useState, type FormEvent, type ReactNode } from 'react';
import { Check, Clock3, Gauge, Info, RotateCcw, Save, Settings2 } from 'lucide-react';
import { Button, InlineError, SectionHeading, StatusBadge } from '../../components/primitives';
import type { Settings } from '../../types/api';
import type { ExecuteAction } from '../queue/QueuePage';

type SettingKey = keyof Settings;
type Draft = Record<SettingKey, string>;
const toDraft = (settings: Settings): Draft => Object.fromEntries(Object.entries(settings).map(([key, value]) => [key, String(value)])) as Draft;

export function SettingsPage({ settings, loading, error, pending, onRefresh, execute }: { settings: Settings | null; loading: boolean; error?: string; pending: boolean; onRefresh: () => void; execute: ExecuteAction }) {
  const [draft, setDraft] = useState<Draft>({ minimum_interval_seconds: '600', queue_capacity: '0', search_timeout_seconds: '900', movie_theatrical_delay_days: '0' });
  const [dirty, setDirty] = useState(false);
  const [fieldError, setFieldError] = useState('');
  const [saved, setSaved] = useState(false);
  useEffect(() => { if (settings && !dirty) setDraft(toDraft(settings)); }, [settings, dirty]);

  const update = (key: SettingKey, value: string) => { setDraft(current => ({ ...current, [key]: value })); setDirty(true); setSaved(false); setFieldError(''); };
  const save = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const numeric = Object.fromEntries(Object.entries(draft).map(([key, value]) => [key, Number(value)])) as unknown as Settings;
    if (Object.values(numeric).some(value => !Number.isInteger(value))) { setFieldError('Inserisci valori numerici interi in tutti i campi.'); return; }
    if (numeric.minimum_interval_seconds < 600) { setFieldError('L’intervallo non può essere inferiore a 600 secondi.'); return; }
    if (numeric.queue_capacity < 0 || numeric.search_timeout_seconds <= 0 || numeric.movie_theatrical_delay_days < 0) { setFieldError('Uno o più valori sono fuori dai limiti consentiti.'); return; }
    try {
      await execute('settings', 'settings', 'PUT', numeric);
      setDraft(toDraft(numeric)); setDirty(false); setSaved(true); setFieldError('');
    } catch { /* Error feedback is shown by the application shell. */ }
  };

  return <div className="page-stack settings-page" id="settings">
    <section className="page-intro"><div><p className="eyebrow">SYSTEM CONFIGURATION / GXE</p><h1>Settings<span>.</span></h1><p>Parametri effettivamente gestiti dallo scheduler GXE.</p></div><Button variant="secondary" onClick={onRefresh}><RotateCcw size={15}/> Ricarica</Button></section>
    {error && <InlineError message={error} retry={onRefresh}/>}

    <section className="settings-context"><span className="settings-context-icon"><Settings2 size={21}/></span><div><b>Configurazione locale</b><p>Queste impostazioni sono salvate nel database SQLite persistente di GXE. Radarr, Sonarr e Prowlarr mantengono le proprie configurazioni.</p></div><StatusBadge tone={settings ? 'good' : 'neutral'}>{settings ? 'Sincronizzate' : 'Caricamento'}</StatusBadge></section>

    <form className="settings-form" onSubmit={save}>
      <section className="surface-panel settings-group"><SectionHeading index="01" eyebrow="QUEUE / RATE CONTROL" title="Ritmo dello scheduler"/><p className="group-description">Il limite si applica all’avvio delle ricerche GXE. Non è un rate limiter per singolo indexer.</p>
        <SettingField label="Intervallo minimo" description="Tempo minimo tra l’inizio di due ricerche GXE." unit="secondi" name="minimum_interval_seconds" value={draft.minimum_interval_seconds} min={600} disabled={loading} icon={Clock3} onChange={value => update('minimum_interval_seconds', value)}><p>Vincolo: almeno 10 minuti (600 secondi). Se una ricerca dura più a lungo, lo scheduler attende che termini.</p></SettingField>
        <SettingField label="Capacità della coda" description="Numero massimo di lavori attivi non terminali." unit="elementi" name="queue_capacity" value={draft.queue_capacity} min={0} disabled={loading} icon={Gauge} onChange={value => update('queue_capacity', value)}><p>Imposta 0 per non applicare un limite. Svuotare la coda non elimina download o file.</p></SettingField>
      </section>

      <section className="surface-panel settings-group"><SectionHeading index="02" eyebrow="SEARCH / RECOVERY" title="Tempo e disponibilità"/><p className="group-description">Modifiche di pianificazione e timeout applicate dal worker.</p>
        <SettingField label="Timeout ricerca" description="Tempo massimo di attesa per l’operazione esterna." unit="secondi" name="search_timeout_seconds" value={draft.search_timeout_seconds} min={1} disabled={loading} icon={Clock3} onChange={value => update('search_timeout_seconds', value)}><p>Un timeout non prova che Radarr o Sonarr non abbiano eseguito il comando. Gli esiti ambigui richiedono riconciliazione.</p></SettingField>
        <SettingField label="Ritardo uscita cinematografica" description="Giorni aggiunti alla data theatrical se manca quella digitale." unit="giorni" name="movie_theatrical_delay_days" value={draft.movie_theatrical_delay_days} min={0} disabled={loading} icon={Clock3} onChange={value => update('movie_theatrical_delay_days', value)}><p>Usato solo quando la data di uscita digitale non è disponibile. Il valore iniziale di 30 giorni non è preimpostato.</p></SettingField>
      </section>

      {fieldError && <div className="field-error" role="alert"><Info size={16}/>{fieldError}</div>}
      {saved && !dirty && <div className="saved-indicator" role="status"><Check size={16}/> Parametri salvati.</div>}
      <div className="settings-submit-bar"><div><span className={dirty ? 'dirty-indicator' : 'clean-indicator'}/><span>{dirty ? 'Modifiche non salvate' : 'Tutte le modifiche sono salvate'}</span></div><Button variant="primary" type="submit" disabled={!dirty || pending || loading}>{pending ? <><span className="button-spinner"/> Salvataggio…</> : <><Save size={16}/> Salva impostazioni</>}</Button></div>
    </form>

    <section className="settings-note"><Info size={17}/><p><b>Configurazioni non presenti in questa schermata</b><span>Policy di retry tecnici, sincronizzazione, gestione indexer e notifiche non sono esposte dal backend attuale; non vengono simulate.</span></p></section>
  </div>;
}

function SettingField({ label, description, unit, name, value, min, disabled, icon: Icon, onChange, children }: { label: string; description: string; unit: string; name: SettingKey; value: string; min: number; disabled: boolean; icon: typeof Clock3; onChange: (value: string) => void; children: ReactNode }) {
  return <div className="setting-row"><span className="setting-icon"><Icon size={17}/></span><div className="setting-copy"><label htmlFor={name}>{label}<small>{description}</small></label>{children}</div><div className="setting-input-wrap"><input id={name} name={name} type="number" min={min} step="1" inputMode="numeric" value={value} disabled={disabled} onChange={event => onChange(event.target.value)}/><span>{unit}</span></div></div>;
}
