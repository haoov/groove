import { useEffect, useCallback, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { OP } from '../shared/ipc/ops';
import { chordLabel } from '../shared/lib/keys';
import { errorText } from '../shared/lib/appError';
import { opSpec } from './ops';
import type { Edits } from './ops/spec';

const APPROVE_HINT = chordLabel({ key: 'enter', ctrl: true });
const DEFER_HINT = chordLabel({ key: 'escape' });

export function ConfirmModal() {
  const pendingConfirmations = useStore((s) => s.pendingConfirmations);
  const removeConfirmation = useStore((s) => s.removeConfirmation);
  const setLastError = useStore((s) => s.setLastError);
  const setSkillsStale = useStore((s) => s.setSkillsStale);
  const confirmationsMinimized = useStore((s) => s.confirmationsMinimized);
  const setConfirmationsMinimized = useStore((s) => s.setConfirmationsMinimized);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [edits, setEdits] = useState<Edits>({});
  const modalRef = useRef<HTMLDivElement>(null);
  const approveRef = useRef<HTMLButtonElement>(null);

  const current = pendingConfirmations[0];
  const spec = current ? opSpec(current.op_type) : undefined;
  const payload = (current?.payload ?? {}) as Record<string, unknown>;
  const invalid = spec?.validate?.(edits) ?? null;

  useEffect(() => {
    setError(null);
    setEdits(current && spec?.seed ? spec.seed((current.payload ?? {}) as Record<string, unknown>) : {});
  }, [current?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const setField = useCallback((key: string, value: string) => {
    setEdits((prev) => ({ ...prev, [key]: value }));
  }, []);

  const resolve = useCallback(
    async (approved: boolean) => {
      if (!current || running) return;
      if (approved && invalid) {
        setError(invalid);
        return;
      }
      setRunning(true);
      setError(null);
      try {
        // Send only the changed fields.
        const p = (current.payload ?? {}) as Record<string, unknown>;
        const overrides: Edits = {};
        for (const [k, v] of Object.entries(edits)) {
          if (v !== String(p[k] ?? '')) overrides[k] = v;
        }
        const hasOverrides = approved && Object.keys(overrides).length > 0;
        await invoke('resolve_confirmation', {
          id: current.id,
          approved,
          payloadOverrides: hasOverrides ? overrides : null,
        });
        if (approved && current.op_type === OP.SKILL_SAVE) setSkillsStale(true);
        removeConfirmation(current.id);
      } catch (e) {
        const msg = errorText(e);
        setError(msg);
        setLastError(msg);
      } finally {
        setRunning(false);
      }
    },
    [current, running, edits, invalid, removeConfirmation, setLastError, setSkillsStale]
  );

  useEffect(() => {
    if (current && !confirmationsMinimized) approveRef.current?.focus();
  }, [current?.id, confirmationsMinimized]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // No shortcut resolves while the modal is deferred.
      if (!current || useStore.getState().confirmationsMinimized) return;
      const modal = modalRef.current;
      const insideModal = !!modal && modal.contains(document.activeElement);
      const active = document.activeElement as HTMLElement | null;
      const inField =
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        active?.isContentEditable === true;

      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); resolve(true); return; }
      // Plain Enter never approves from inside a field.
      if (e.key === 'Enter' && insideModal && !inField) { e.preventDefault(); resolve(true); return; }

      // Esc blurs a field or defers the queue; it never denies.
      if (e.key === 'Escape') {
        if (!insideModal) return;
        if (inField) { e.preventDefault(); active?.blur(); return; }
        e.preventDefault();
        setConfirmationsMinimized(true);
        return;
      }

      // Focus trap: Tab cycles within the dialog.
      if (e.key === 'Tab' && insideModal && modal) {
        const focusables = Array.from(
          modal.querySelectorAll<HTMLElement>('button, textarea, input, [tabindex]:not([tabindex="-1"])'),
        ).filter((el) => !el.hasAttribute('disabled'));
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
        else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [current, resolve, setConfirmationsMinimized]);

  if (!current || confirmationsMinimized) return null;

  const Icon = spec?.icon;
  const View = spec?.View;
  // The bridge tags agent-originated ops as 'mcp'.
  const isAgent = current.origin === 'mcp';

  return (
    <div className="confirm-overlay">
      <div className="confirm-modal" role="dialog" aria-modal="true" ref={modalRef}>

        <div className="confirm-header">
          <span className="confirm-op-icon">{Icon && <Icon size={14} strokeWidth={1.75} />}</span>
          <span className="confirm-title">{spec?.label ?? current.op_type}</span>
          <span className={`confirm-origin-badge ${isAgent ? 'agent' : 'ui'}`}>
            {isAgent ? 'Agent' : 'UI'}
          </span>
          {pendingConfirmations.length > 1 && (
            <span className="confirm-queue">+{pendingConfirmations.length - 1} queued</span>
          )}
        </div>

        <div className="confirm-payload">
          {View
            ? <View payload={payload} edits={edits} setField={setField} />
            : <pre className="cp-raw">{JSON.stringify(payload, null, 2)}</pre>}
        </div>

        {error && (
          <div className="confirm-error">{error}</div>
        )}

        <div className="confirm-actions">
          <button
            ref={approveRef}
            className="btn-primary"
            disabled={running || !!invalid}
            title={invalid ?? undefined}
            onClick={() => resolve(true)}
          >
            {running ? 'Running…' : <><span>Approve</span> <kbd>{APPROVE_HINT}</kbd></>}
          </button>
          <button className="btn-secondary" disabled={running} onClick={() => resolve(false)}>
            Deny
          </button>
          <button
            className="btn-secondary confirm-later"
            disabled={running}
            onClick={() => setConfirmationsMinimized(true)}
            title="Keep it pending — review later from the statusbar"
          >
            Later <kbd>{DEFER_HINT}</kbd>
          </button>
        </div>

      </div>
    </div>
  );
}
