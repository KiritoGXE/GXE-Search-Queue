import { useEffect, useRef, type ButtonHTMLAttributes, type ReactNode } from 'react';
import { AlertTriangle, Check, Info, LoaderCircle, X } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

export function Button({ variant = 'secondary', className = '', ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: 'primary' | 'secondary' | 'ghost' | 'danger' }) {
  return <button className={`button button-${variant} ${className}`} {...props}/>;
}

export function IconButton({ icon: Icon, label, className = '', children, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { icon: LucideIcon; label: string }) {
  return <button className={`icon-button ${className}`} aria-label={label} title={label} {...props}><Icon size={17} strokeWidth={1.8}/>{children}</button>;
}

export function SectionHeading({ index, eyebrow, title, action }: { index: string; eyebrow: string; title: string; action?: ReactNode }) {
  return <div className="section-heading"><span className="section-index">{index}</span><div className="section-heading-copy"><p className="eyebrow">{eyebrow}</p><h2>{title}</h2></div>{action && <div className="section-heading-action">{action}</div>}</div>;
}

export function StatusBadge({ tone, children, icon: Icon }: { tone: 'good' | 'info' | 'warning' | 'danger' | 'neutral'; children: ReactNode; icon?: LucideIcon }) {
  return <span className={`status-badge status-${tone}`}>{Icon && <Icon size={13} aria-hidden="true"/>}{children}</span>;
}

export function LoadingBlock({ rows = 3 }: { rows?: number }) {
  return <div className="loading-block" aria-label="Caricamento" aria-busy="true">{Array.from({ length: rows }, (_, index) => <span key={index} style={{ width: `${78 - index * 9}%` }}/>)}</div>;
}

export const Skeleton = LoadingBlock;

export function EmptyState({ icon: Icon = Info, title, children, action }: { icon?: LucideIcon; title: string; children?: ReactNode; action?: ReactNode }) {
  return <div className="empty-state"><span className="empty-icon"><Icon size={20}/></span><b>{title}</b>{children && <p>{children}</p>}{action}</div>;
}

export function InlineError({ message, retry }: { message: string; retry?: () => void }) {
  return <div className="inline-error" role="alert"><AlertTriangle size={16}/><div><b>Dati non aggiornati</b><p>{message}</p></div>{retry && <Button onClick={retry}>Riprova</Button>}</div>;
}

export function Toast({ kind, message, onDismiss }: { kind: 'success' | 'error'; message: string; onDismiss: () => void }) {
  const Icon = kind === 'success' ? Check : AlertTriangle;
  return <div className={`toast toast-${kind}`} role={kind === 'error' ? 'alert' : 'status'} aria-live={kind === 'error' ? 'assertive' : 'polite'}><Icon size={17}/><span>{message}</span><button aria-label="Chiudi notifica" onClick={onDismiss}><X size={16}/></button></div>;
}

export function Drawer({ open, title, eyebrow, onClose, children, className = '' }: { open: boolean; title: string; eyebrow?: string; onClose: () => void; children: ReactNode; className?: string }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) element.showModal();
    if (!open && element.open) element.close();
  }, [open]);
  return <dialog ref={dialog} className={`drawer-dialog ${className}`} onCancel={event => { event.preventDefault(); onClose(); }} onClose={onClose}>
    <div className="drawer-panel"><header className="drawer-header"><div>{eyebrow && <p className="eyebrow">{eyebrow}</p>}<h2>{title}</h2></div><IconButton icon={X} label="Chiudi pannello" onClick={onClose}/></header><div className="drawer-content">{children}</div></div>
  </dialog>;
}

export function ConfirmDialog({ open, title, description, confirmLabel, danger = false, busy = false, onCancel, onConfirm }: { open: boolean; title: string; description: string; confirmLabel: string; danger?: boolean; busy?: boolean; onCancel: () => void; onConfirm: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) element.showModal();
    if (!open && element.open) element.close();
  }, [open]);
  return <dialog ref={dialog} className="confirm-dialog" onCancel={event => { event.preventDefault(); onCancel(); }} onClose={onCancel}>
    <div className="confirm-inner"><div className={danger ? 'confirm-symbol danger' : 'confirm-symbol'}><AlertTriangle size={20}/></div><p className="eyebrow">CONFERMA OPERAZIONE</p><h2>{title}</h2><p>{description}</p><div className="confirm-actions"><Button onClick={onCancel} disabled={busy}>Annulla</Button><Button variant={danger ? 'danger' : 'primary'} onClick={onConfirm} disabled={busy}>{busy && <LoaderCircle className="spin" size={15}/>} {confirmLabel}</Button></div></div>
  </dialog>;
}

export function ErrorSummary({ title, detail }: { title: string; detail: string }) {
  return <div className="error-summary" role="alert"><AlertTriangle size={18}/><div><b>{title}</b><p>{detail}</p></div></div>;
}
