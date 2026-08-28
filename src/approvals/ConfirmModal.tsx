import { useEffect, useCallback, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { GitCommit, Upload, Download, ChevronsUp, GitPullRequest, X, RefreshCw, FilePlus, FolderPlus, GitBranch, RotateCcw, Clock, FileText, Tag, Sparkles } from 'lucide-react';
import { useStore } from '../shared/store';
import { OP } from '../shared/ipc/ops';

const OP_LABELS: Record<string, string> = {
  [OP.GIT_COMMIT]:  'Git commit',
  [OP.GIT_PUSH]:    'Git push',
  [OP.GIT_PULL]:    'Git pull',
  [OP.GIT_REBASE]:  'Rebase on main',
  [OP.MR_CREATE]:   'Create MR',
  [OP.MR_UPDATE]:   'Update MR',
  [OP.MR_CLOSE]:    'Close MR',
  [OP.TASK_PROPERTY]: 'Update task property',
  [OP.TASK_HOURS]: 'Log hours',
  [OP.TASK_BODY]: 'Update task description',
  [OP.TASK_CREATE]: 'Create task',
  [OP.TASK_ADD_REPO]: 'Add repo to task',
  [OP.TASK_ADD_WORKTREE]: 'Add worktree to task',
  [OP.TASK_CREATE_FROM_EXPLORER]: 'Create task from explorer',
  [OP.SKILL_SAVE]: 'Write agent skill',
  [OP.GIT_DISCARD]: 'Discard changes',
  [OP.GIT_DISCARD_ALL]: 'Discard all changes',
};

/** Starting point for a hand-written MR description. Mirrors the headings the
 *  agent's create_mr contract requires (MR_DESCRIPTION in definitions.rs). */
const MR_SKELETON = '## What\n\n\n## Why\n\n';

const OP_ICONS: Record<string, React.ReactNode> = {
  [OP.GIT_COMMIT]:  <GitCommit    size={14} strokeWidth={1.75} />,
  [OP.GIT_PUSH]:    <Upload       size={14} strokeWidth={1.75} />,
  [OP.GIT_PULL]:    <Download     size={14} strokeWidth={1.75} />,
  [OP.GIT_REBASE]:  <ChevronsUp   size={14} strokeWidth={1.75} />,
  [OP.MR_CREATE]:   <GitPullRequest size={14} strokeWidth={1.75} />,
  [OP.MR_UPDATE]:   <RefreshCw    size={14} strokeWidth={1.75} />,
  [OP.MR_CLOSE]:    <X            size={14} strokeWidth={1.75} />,
  [OP.TASK_PROPERTY]: <Tag size={14} strokeWidth={1.75} />,
  [OP.TASK_HOURS]: <Clock size={14} strokeWidth={1.75} />,
  [OP.TASK_BODY]: <FileText size={14} strokeWidth={1.75} />,
  [OP.TASK_CREATE]: <FilePlus size={14} strokeWidth={1.75} />,
  [OP.TASK_ADD_REPO]: <FolderPlus size={14} strokeWidth={1.75} />,
  [OP.TASK_ADD_WORKTREE]: <GitBranch size={14} strokeWidth={1.75} />,
  [OP.TASK_CREATE_FROM_EXPLORER]: <FilePlus size={14} strokeWidth={1.75} />,
  [OP.SKILL_SAVE]: <Sparkles size={14} strokeWidth={1.75} />,
  [OP.GIT_DISCARD]: <RotateCcw   size={14} strokeWidth={1.75} />,
  [OP.GIT_DISCARD_ALL]: <RotateCcw size={14} strokeWidth={1.75} />,
};

// ── Payload renderer ──────────────────────────────────────────────────────────

function Field({ label, value, mono = false, block = false }: {
  label: string;
  value: string;
  mono?: boolean;
  block?: boolean;
}) {
  return (
    <div className={`cp-field ${block ? 'cp-field--block' : ''}`}>
      <span className="cp-field-label">{label}</span>
      <span className={`cp-field-value ${mono ? 'cp-field-value--mono' : ''}`}>{value}</span>
    </div>
  );
}

function EditableField({ label, value, onChange, multiline = false, placeholder }: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  multiline?: boolean;
  placeholder?: string;
}) {
  return (
    <div className="cp-field cp-field--block">
      <span className="cp-field-label">{label}</span>
      {multiline ? (
        <textarea
          className="cp-field-input cp-field-textarea"
          value={value}
          placeholder={placeholder}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : (
        <input
          className="cp-field-input"
          value={value}
          placeholder={placeholder}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
    </div>
  );
}

// Payloads carry the project name; the path's last segment is only a fallback
// for rows queued before they did (it is the branch leaf, not the repo, now that
// worktree dirs embed the branch's slashes).
function repoName(payload: Record<string, unknown>): string {
  const repo = payload.repo;
  if (typeof repo === 'string' && repo) return repo;
  const path = (payload.worktree_path as string | undefined) ?? '';
  return path.split('/').filter(Boolean).pop() ?? path;
}

function PayloadView({ op, payload, edits, setField }: {
  op: string;
  payload: Record<string, unknown>;
  edits: Record<string, string>;
  setField: (key: string, value: string) => void;
}) {
  const str = (k: string) => (payload[k] as string | undefined) ?? '';

  switch (op) {
    case OP.GIT_COMMIT:
      return (
        <>
          <Field label="Repo"    value={repoName(payload)} />
          <Field label="Branch"  value={str('branch')} mono />
          <EditableField
            label="Message"
            value={edits.message ?? ''}
            onChange={(v) => setField('message', v)}
            multiline
            placeholder="Commit message (first line is the title)"
          />
        </>
      );

    case OP.GIT_PUSH:
      return (
        <>
          <Field label="Repo"   value={repoName(payload)} />
          <Field label="Branch" value={str('branch')} mono />
          <div className="cp-hint">Pushes local commits to origin.</div>
        </>
      );

    case OP.GIT_PULL:
      return (
        <>
          <Field label="Repo"   value={repoName(payload)} />
          <Field label="Branch" value={str('branch')} mono />
          <div className="cp-hint">Pulls and rebases from origin.</div>
        </>
      );

    case OP.GIT_REBASE: {
      const onto = str('default_branch') || 'main';
      return (
        <>
          <Field label="Repo"   value={repoName(payload)} />
          <Field label="Branch" value={str('branch')} mono />
          <Field label="Onto"   value={`origin/${onto}`} mono />
        </>
      );
    }

    case OP.MR_CREATE:
    case OP.MR_UPDATE:
      return (
        <>
          {/* Everything except the text is already decided — show it, read-only,
              so it's clear what this MR will be opened from. */}
          {op === OP.MR_CREATE && (
            <>
              <Field label="Repo"   value={repoName(payload)} />
              <Field label="Branch" value={str('branch')} mono />
              <Field label="Into"   value={str('target_branch')} mono />
            </>
          )}
          <EditableField
            label="Title"
            value={edits.title ?? ''}
            onChange={(v) => setField('title', v)}
            placeholder="Merge request title"
          />
          <EditableField
            label="Description"
            value={edits.description ?? ''}
            onChange={(v) => setField('description', v)}
            multiline
            placeholder="Describe the change…"
          />
        </>
      );

    case OP.TASK_ADD_REPO:
      return (
        <>
          <Field label="Repo"   value={str('repo')} mono />
          <Field label="Task"   value={str('task_id')} mono />
          {str('branch') && <Field label="Branch" value={str('branch')} mono />}
          {str('target_branch') && <Field label="Based on" value={str('target_branch')} mono />}
          <div className="cp-hint">
            Attaches the repo and creates its worktree. Repos already on the task stay.
          </div>
        </>
      );

    case OP.TASK_ADD_WORKTREE:
      return (
        <>
          <Field label="Repo"   value={str('repo') || 'the task\u2019s only repo'} mono />
          <Field label="Task"   value={str('task_id')} mono />
          <Field label="Branch" value={str('branch')} mono />
          {str('target_branch') && <Field label="Based on" value={str('target_branch')} mono />}
          <div className="cp-hint">
            Checks out another branch of a repo the task already has, beside the
            worktrees it holds now. Nothing existing is touched.
          </div>
        </>
      );

    case OP.MR_CLOSE:
      return <div className="cp-hint">This will close the MR and cannot be undone.</div>;

    case OP.GIT_DISCARD:
      return (
        <>
          <Field label="File" value={str('file_path')} mono />
          <div className="cp-hint cp-hint--danger">Permanently discards this file's local changes — this cannot be undone.</div>
        </>
      );

    case OP.GIT_DISCARD_ALL:
      return (
        <>
          <Field label="Repo" value={repoName(payload)} />
          <div className="cp-hint cp-hint--danger">Permanently discards ALL local changes (reverts tracked files and removes untracked ones) — this cannot be undone.</div>
        </>
      );

    case OP.TASK_PROPERTY: {
      const v = payload.value;
      return (
        <>
          <Field label="Property" value={str('property')} />
          <Field label="Value" value={typeof v === 'string' ? v : JSON.stringify(v)} mono />
          <Field label="Task" value={str('task_id')} mono />
        </>
      );
    }

    case OP.TASK_HOURS:
      return (
        <>
          <Field label="Hours" value={String(payload.hours ?? '')} mono />
          <Field label="Task" value={str('task_id')} mono />
        </>
      );

    case OP.TASK_BODY:
      return (
        <>
          <Field label="Task" value={str('task_id')} mono />
          <EditableField
            label="New description"
            value={edits.markdown ?? ''}
            onChange={(v) => setField('markdown', v)}
            multiline
            placeholder="Page body (markdown)"
          />
          <div className="cp-hint cp-hint--danger">
            Replaces the whole task body. Anything markdown cannot represent is lost.
          </div>
        </>
      );

    // Editable, like a task body: this is a procedure the agent will follow on its
    // own later, and reading it is the whole point of the confirmation.
    case OP.SKILL_SAVE:
      return (
        <>
          <Field label="Skill" value={`user:${str('name')}`} mono />
          {str('previous') && <Field label="Replaces" value={`user:${str('previous')}`} mono />}
          <EditableField
            label="SKILL.md"
            value={edits.body ?? ''}
            onChange={(v) => setField('body', v)}
            multiline
            placeholder="Front matter, then the procedure"
          />
          <div className="cp-hint">
            Agents load skills at startup — this one works after a reload.
          </div>
        </>
      );

    case OP.TASK_CREATE:
      return (
        <>
          <EditableField
            label="Title"
            value={edits.title ?? ''}
            onChange={(v) => setField('title', v)}
            placeholder="Task title"
          />
          <EditableField
            label="Description"
            value={edits.body_markdown ?? ''}
            onChange={(v) => setField('body_markdown', v)}
            multiline
            placeholder="Task description (markdown)"
          />
        </>
      );

    case OP.TASK_CREATE_FROM_EXPLORER:
      return (
        <>
          <EditableField
            label="Title"
            value={edits.title ?? ''}
            onChange={(v) => setField('title', v)}
            placeholder="Task title"
          />
          <EditableField
            label="Description"
            value={edits.body_markdown ?? ''}
            onChange={(v) => setField('body_markdown', v)}
            multiline
            placeholder="Task description (markdown)"
          />
          {/* Name the source session explicitly: the agent targets whichever
              session is focused, so a mis-aimed conversion must be visible
              BEFORE approval — it moves that session's worktrees and repos. */}
          <Field label="Converting" value={str('explorer_id')} mono />
          <div className="cp-hint">
            Files a task, then moves this session's worktrees, repos, and annotations onto it.
          </div>
        </>
      );

    default:
      return (
        <pre className="cp-raw">{JSON.stringify(payload, null, 2)}</pre>
      );
  }
}

// ── Modal ─────────────────────────────────────────────────────────────────────

export function ConfirmModal() {
  const { pendingConfirmations, removeConfirmation, setLastError } = useStore();
  const setSkillsStale = useStore((s) => s.setSkillsStale);
  const confirmationsMinimized = useStore((s) => s.confirmationsMinimized);
  const setConfirmationsMinimized = useStore((s) => s.setConfirmationsMinimized);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Editable field values (commit message, MR title/description), seeded from the payload.
  const [edits, setEdits] = useState<Record<string, string>>({});
  const modalRef = useRef<HTMLDivElement>(null);
  const approveRef = useRef<HTMLButtonElement>(null);

  const current = pendingConfirmations[0];

  // Reset error + seed editable fields whenever the active confirmation changes.
  useEffect(() => {
    setError(null);
    if (!current) { setEdits({}); return; }
    const p = (current.payload ?? {}) as Record<string, unknown>;
    const seed: Record<string, string> = {};
    if (current.op_type === OP.GIT_COMMIT) seed.message = String(p.message ?? '');
    if (current.op_type === OP.MR_CREATE || current.op_type === OP.MR_UPDATE) {
      seed.title = String(p.title ?? '');
      // The git buttons open this dialog with no text at all, so hand-written MRs
      // start from the same two headings the agent's contract requires.
      seed.description = String(p.description ?? '') || MR_SKELETON;
    }
    if (current.op_type === OP.TASK_CREATE_FROM_EXPLORER || current.op_type === OP.TASK_CREATE) {
      seed.title = String(p.title ?? '');
      seed.body_markdown = String(p.body_markdown ?? '');
    }
    if (current.op_type === OP.TASK_BODY) seed.markdown = String(p.markdown ?? '');
    if (current.op_type === OP.SKILL_SAVE) seed.body = String(p.body ?? '');
    setEdits(seed);
  }, [current?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const setField = useCallback((key: string, value: string) => {
    setEdits((prev) => ({ ...prev, [key]: value }));
  }, []);

  // An MR needs a title from the user; everything else arrives complete.
  const missingTitle = current?.op_type === OP.MR_CREATE && !(edits.title ?? '').trim();

  const resolve = useCallback(
    async (approved: boolean) => {
      if (!current || running) return;
      // create_mr_impl falls back to a "WIP" title, so an empty field would
      // silently open a nameless MR — the text is the whole point of asking.
      if (approved && current.op_type === OP.MR_CREATE && !(edits.title ?? '').trim()) {
        setError('Give the merge request a title first.');
        return;
      }
      setRunning(true);
      setError(null);
      try {
        // Send only the fields the user actually changed, so untouched ops execute
        // with their original payload (and we never blank an MR field by accident).
        const p = (current.payload ?? {}) as Record<string, unknown>;
        const overrides: Record<string, string> = {};
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
        const msg = String(e);
        setError(msg);
        setLastError(msg);
      } finally {
        setRunning(false);
      }
    },
    [current, running, edits, removeConfirmation, setLastError, setSkillsStale]
  );

  // Autofocus the approve button when a confirmation opens (or is restored from
  // the statusbar), so keyboard actions (Enter / Tab) are scoped to the modal.
  useEffect(() => {
    if (current && !confirmationsMinimized) approveRef.current?.focus();
  }, [current?.id, confirmationsMinimized]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // While deferred the modal is unmounted — no shortcut may decide invisibly.
      if (!current || useStore.getState().confirmationsMinimized) return;
      const modal = modalRef.current;
      const insideModal = !!modal && modal.contains(document.activeElement);
      const active = document.activeElement as HTMLElement | null;
      // A field is a text input, textarea, OR a contenteditable region (e.g. CodeMirror).
      const inField =
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        active?.isContentEditable === true;

      // ⌘/Ctrl+Enter always approves.
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); resolve(true); return; }
      // Plain Enter approves ONLY when focus is inside the modal and not in a field
      // (so Enter while typing in CodeMirror / the message box never approves).
      if (e.key === 'Enter' && insideModal && !inField) { e.preventDefault(); resolve(true); return; }

      // Esc is scoped to the modal — never leaks to stacked overlays. First Esc in
      // a field blurs it; otherwise it DEFERS the queue (nothing is denied — the
      // statusbar badge keeps it pending). Denying is always an explicit click.
      if (e.key === 'Escape') {
        if (!insideModal) return;
        if (inField) { e.preventDefault(); active?.blur(); return; }
        e.preventDefault();
        setConfirmationsMinimized(true);
        return;
      }

      // Basic focus trap: Tab / Shift+Tab cycle within the dialog.
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

  // Deferred: the queue stays pending; the statusbar badge is the way back in.
  if (!current || confirmationsMinimized) return null;

  const label    = OP_LABELS[current.op_type] ?? current.op_type;
  const icon     = OP_ICONS[current.op_type]  ?? null;
  // At runtime the confirmation bridge tags agent-originated ops as 'mcp'.
  const isAgent  = current.origin === 'mcp';
  const payload  = current.payload as Record<string, unknown>;

  return (
    <div className="confirm-overlay">
      <div className="confirm-modal" role="dialog" aria-modal="true" ref={modalRef}>

        <div className="confirm-header">
          <span className="confirm-op-icon">{icon}</span>
          <span className="confirm-title">{label}</span>
          <span className={`confirm-origin-badge ${isAgent ? 'agent' : 'ui'}`}>
            {isAgent ? 'Agent' : 'UI'}
          </span>
          {pendingConfirmations.length > 1 && (
            <span className="confirm-queue">+{pendingConfirmations.length - 1} queued</span>
          )}
        </div>

        <div className="confirm-payload">
          <PayloadView op={current.op_type} payload={payload} edits={edits} setField={setField} />
        </div>

        {error && (
          <div className="confirm-error">{error}</div>
        )}

        <div className="confirm-actions">
          <button
            ref={approveRef}
            className="btn-primary"
            disabled={running || missingTitle}
            title={missingTitle ? 'A title is required' : undefined}
            onClick={() => resolve(true)}
          >
            {running ? 'Running…' : <><span>Approve</span> <kbd>⌘⏎</kbd></>}
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
            Later <kbd>Esc</kbd>
          </button>
        </div>

      </div>
    </div>
  );
}
