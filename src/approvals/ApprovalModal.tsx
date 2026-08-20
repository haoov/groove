import { useEffect, useMemo, useState } from 'react';
import { useStore } from '../shared/store';
import { opMeta } from './ops';

/** The global approval surface: a modal that demands a decision on the oldest
 *  pending confirmation. It is deliberately NOT inline in the agent console —
 *  an approval interrupts whatever the user is doing until they act. */
export function ApprovalModal() {
  const confirmations = useStore((s) => s.confirmations);
  const resolve = useStore((s) => s.resolveConfirmation);
  const current = confirmations[0];

  if (!current) return null;
  return (
    <ApprovalCard
      key={current.id}
      count={confirmations.length}
      onResolve={resolve}
      id={current.id}
      opType={current.op_type}
      payload={current.payload}
    />
  );
}

function ApprovalCard({
  id, opType, payload, count, onResolve,
}: {
  id: string;
  opType: string;
  payload: Record<string, unknown>;
  count: number;
  onResolve: (id: string, approved: boolean, overrides?: Record<string, unknown>) => void;
}) {
  const meta = useMemo(() => opMeta(opType), [opType]);

  // Seed editable fields from the payload; edits become payload_overrides.
  const [edits, setEdits] = useState<Record<string, string>>(() => {
    const seed: Record<string, string> = {};
    for (const f of meta.edit ?? []) {
      const v = payload[f.key];
      seed[f.key] = typeof v === 'string' ? v : '';
    }
    return seed;
  });

  const approve = () => {
    const overrides = meta.edit?.length
      ? Object.fromEntries(meta.edit.map((f) => [f.key, edits[f.key] ?? '']))
      : undefined;
    onResolve(id, true, overrides);
  };
  const reject = () => onResolve(id, false);

  const canApprove = (meta.edit ?? []).every((f) => (edits[f.key] ?? '').trim().length > 0);

  // Enter approves (unless a multiline field has focus); Escape rejects.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { e.preventDefault(); reject(); }
      else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && canApprove) { e.preventDefault(); approve(); }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  });

  return (
    <div className="ov-scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) reject(); }}>
      <div className={`approval${meta.danger ? ' danger' : ''}`} role="dialog" aria-modal="true">
        <div className="approval-h">
          <span className="approval-title">{meta.title}</span>
          {count > 1 && <span className="approval-more">1 of {count}</span>}
        </div>

        <dl className="approval-summary">
          {meta.summary(payload).map(([k, v]) => (
            <div key={k}><dt>{k}</dt><dd>{v || '—'}</dd></div>
          ))}
        </dl>

        {(meta.edit ?? []).map((f) => (
          <label key={f.key} className="approval-field">
            <span>{f.label}</span>
            {f.multiline ? (
              <textarea rows={4} value={edits[f.key] ?? ''} autoFocus
                onChange={(e) => setEdits((s) => ({ ...s, [f.key]: e.target.value }))} />
            ) : (
              <input value={edits[f.key] ?? ''} autoFocus
                onChange={(e) => setEdits((s) => ({ ...s, [f.key]: e.target.value }))} />
            )}
          </label>
        ))}

        <div className="approval-actions">
          <button className="approval-reject" onClick={reject}>Reject</button>
          <button className={`approval-approve${meta.danger ? ' danger' : ''}`} disabled={!canApprove} onClick={approve}>
            {meta.verb}
          </button>
        </div>
      </div>
    </div>
  );
}
