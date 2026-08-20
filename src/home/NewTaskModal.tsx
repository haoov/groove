import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Eye, Loader2, Pencil, Sparkles } from 'lucide-react';
import { useStore } from '../shared/store';
import { sendToAgent } from '../agent/agentSend';
import { ensureDeskSession } from '../sessions/desk';
import { Markdown } from '../shared/ui/Markdown';
import {
  AddField, MultiRow, Pill, draftRow, isHoursProperty, MULTI_KINDS, SINGLE_KINDS,
} from '../overview/propertyControls';
import type { Task, TaskSchema } from '../shared/ipc/ipc';

/**
 * File a task without opening it.
 *
 * The body starts from the Notion template so a hand-filed task looks like an
 * agent-filed one, and properties use the SAME controls as the overview
 * (propertyControls) — the task you file is shaped like the task you will edit.
 *
 * Properties are applied after the page exists: Notion's create call only takes a
 * fixed set, so the rest are patched (see apply_extra_properties in creation.rs).
 * One that won't take is reported, not fatal — the page is already filed.
 */
export function NewTaskModal({ onClose }: { onClose: () => void }) {
  const notify = useStore((s) => s.notify);
  const setLastError = useStore((s) => s.setLastError);
  const setTasks = useStore((s) => s.setTasks);
  const priorityProp = useStore((s) => s.config?.notion.properties.priority ?? null);

  const [title, setTitle] = useState('');
  const [body, setBody] = useState('');
  const [preview, setPreview] = useState(false);
  const [schema, setSchema] = useState<TaskSchema | null>(null);
  const [draft, setDraft] = useState<Record<string, unknown>>({});
  const [revealed, setRevealed] = useState<Set<string>>(new Set());
  const [saving, setSaving] = useState(false);

  // Start from the template, so a filed task looks like every other one.
  useEffect(() => {
    invoke<string>('get_task_template_markdown')
      .then((md) => setBody((b) => (b ? b : md)))
      .catch(() => {/* no template configured — a blank body is fine */});
  }, []);

  // Status and assignee are set by the backend from config; showing them here
  // would offer a choice that filing overrides anyway.
  useEffect(() => {
    invoke<TaskSchema>('get_task_schema').then(setSchema).catch((e) => setLastError(String(e)));
  }, [setLastError]);

  const rows = useMemo(() => {
    const editable = (schema?.properties ?? []).filter(
      (p) =>
        p.editable &&
        p.kind !== 'title' &&
        p.kind !== 'status' &&
        !isHoursProperty(p.name, p.kind),
    );
    const shown = editable.filter((p) => revealed.has(p.name));
    return {
      single: shown.filter((p) => SINGLE_KINDS.includes(p.kind)),
      multi: shown.filter((p) => MULTI_KINDS.includes(p.kind)),
      unset: editable.filter((p) => !revealed.has(p.name)),
    };
  }, [schema, revealed]);

  const set = (name: string, value: unknown) =>
    setDraft((d) => ({ ...d, [name]: value }));

  const file = async () => {
    if (!title.trim() || saving) return;
    setSaving(true);
    try {
      // Only properties the user actually set — an untouched pill must not clear
      // whatever default Notion applies.
      const properties = Object.fromEntries(
        Object.entries(draft).filter(([, v]) => v !== null && v !== undefined && !(Array.isArray(v) && v.length === 0)),
      );
      const task = await invoke<Task & { warnings?: string[] }>('create_task', {
        title: title.trim(),
        bodyMarkdown: body,
        properties,
      });
      notify({
        // The task IS filed either way; a property that didn't take needs the
        // user's attention rather than reading as a failure.
        kind: task.warnings?.length ? 'attention' : 'success',
        source: 'notion',
        taskId: task.short_id,
        title: `Filed ${task.short_id}`,
        detail: task.warnings?.length
          ? `Some properties did not apply — ${task.warnings.join('; ')}`
          : title.trim(),
      });
      // It belongs in Up next now — refresh rather than wait for the next sync.
      invoke<Task[]>('list_tasks').then(setTasks).catch(() => {});
      onClose();
    } catch (e) {
      setLastError(String(e));
    } finally {
      setSaving(false);
    }
  };

  /** Hand drafting to the desk agent — always available, session or not. */
  const askAgent = async () => {
    const ask =
      `File a new Notion task${title.trim() ? ` titled "${title.trim()}"` : ''}. ` +
      'Call get_task_template first, then create_task. Ask me what you need to know ' +
      'before filing — do not invent scope.\n';
    try {
      const deskId = await ensureDeskSession();
      await sendToAgent(deskId, ask);
      useStore.getState().requestConsoleFocus();
      onClose();
    } catch (e) {
      setLastError(String(e));
    }
  };

  return (
    <div className="wizard-overlay" onClick={onClose}>
      <div className="wizard-modal ntm" onClick={(e) => e.stopPropagation()}>
        <div className="wizard-header">
          <div className="wizard-title">New task</div>
          <div className="wizard-subtitle">Filed to the queue — not checked out.</div>
          <button className="wizard-close" onClick={onClose}>×</button>
        </div>

        <div className="wizard-body">
          <div className="ntm-fieldbar">
            <span className="ntm-bodylabel">Title</span>
          </div>
          <input
            className="ntm-title"
            autoFocus
            placeholder="What needs to happen?"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Escape') onClose(); }}
          />

          <div className="ntm-props">
            {rows.single.map((p) => (
              <Pill
                key={p.name}
                row={draftRow(p, draft[p.name])}
                busy={false}
                isPriority={p.name === priorityProp}
                onChange={(v) => set(p.name, v)}
              />
            ))}
            {rows.unset.length > 0 && (
              <AddField
                fields={rows.unset.map((p) => p.name)}
                onPick={(name) => setRevealed((s) => new Set(s).add(name))}
              />
            )}
          </div>

          {rows.multi.length > 0 && (
            <div className="props-sets">
              {rows.multi.map((p) => (
                <MultiRow
                  key={p.name}
                  row={draftRow(p, draft[p.name])}
                  busy={false}
                  // Nothing is being written yet, so there is no rate limit to respect.
                  debounce={0}
                  onChange={(v) => set(p.name, v)}
                  onError={setLastError}
                />
              ))}
            </div>
          )}

          <div className="ntm-bodybar">
            <span className="ntm-bodylabel">Description</span>
            <button
              className="home-link"
              onClick={() => setPreview((v) => !v)}
              title={preview ? 'Back to editing' : 'Render the markdown'}
            >
              {preview ? <Pencil size={11} strokeWidth={2} /> : <Eye size={11} strokeWidth={2} />}
              {preview ? 'edit' : 'preview'}
            </button>
          </div>

          {preview ? (
            <div className="ntm-preview">
              {body.trim()
                ? <Markdown text={body} />
                : <p className="wizard-empty">Nothing to preview yet.</p>}
            </div>
          ) : (
            <textarea
              className="composer-body ntm-body"
              spellCheck={false}
              placeholder="Markdown — the template's headings are pre-loaded."
              value={body}
              onChange={(e) => setBody(e.target.value)}
            />
          )}

          <div className="wizard-footer">
            <button className="home-link" onClick={askAgent} title="Let the desk agent draft and file it">
              <Sparkles size={11} strokeWidth={2} />
              ask the agent
            </button>
            <span className="composer-spacer" />
            <button className="btn-secondary" onClick={onClose}>Cancel</button>
            <button className="btn-primary" disabled={!title.trim() || saving} onClick={file}>
              {saving ? <Loader2 size={11} className="spin" /> : null}
              File task
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
